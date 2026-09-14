//! babeltele — translate anything on stdin into BabelTele, the LLM-native
//! compressed notation described in arXiv:2606.19857.
//!
//! Works fully offline (compiled multilingual lexicon + symbolic grammar);
//! `--llm` optionally delegates to a real model.

mod detect;
mod formats;
mod json;
mod lexdata;
mod lexdata_en;
mod lexdata_pl;
mod lexdata_xx;
mod lexicon;
mod llm;
mod numwords;
mod prose;
mod tokcost;

use std::io::{self, BufWriter, IsTerminal, Read, Write};

use clap::Parser;

use detect::Kind;
use lexicon::Optimize;
use prose::Options;

#[derive(Parser, Debug)]
#[command(
    name = "babeltele",
    version,
    about = "Translate human text (any format, any language) into BabelTele — LLM-native compressed notation.",
    long_about = "Reads stdin (or files), autodetects the input kind (prose, JSON, code, Markdown, CSV, logs, key=value, dialogue, HTML) and its natural language, and writes a BabelTele rendering to stdout: cross-lingual fragments, logic/math symbols, emoji, collapsed syntax. Fully offline by default.\n\nBased on: Zhu et al., \"Large Language Models Do Not Always Need Readable Language\", arXiv:2606.19857."
)]
struct Cli {
    /// Input files (default: stdin)
    files: Vec<String>,

    /// Compression level: 1 = light (keeps articles/punctuation), 2 = BabelTele (default),
    /// 3 = aggressive (also drops clause commas and omissible relative pronouns; with -O chars disemvowels long words)
    #[arg(short = 'l', long, default_value_t = 2, value_parser = clap::value_parser!(u8).range(1..=3))]
    level: u8,

    /// Input kind (default: autodetect)
    #[arg(short = 'k', long, default_value = "auto")]
    kind: String,

    /// Natural language of the input, ISO 639-1 (default: autodetect)
    #[arg(short = 'L', long, default_value = "auto")]
    lang: String,

    /// What to minimise: `tokens` (default; a substitution is applied only if it costs no more
    /// BPE tokens under o200k_base/cl100k_base than the words it replaces) or `chars`
    /// (maximal symbolic rendering: CJK/emoji everywhere — shortest text, but often more tokens)
    #[arg(short = 'O', long, default_value = "tokens", value_name = "TARGET")]
    optimize: Optimize,

    /// Print detection results and size statistics to stderr
    #[arg(short = 's', long)]
    stats: bool,

    /// Print the symbol legend (for humans) and exit
    #[arg(long)]
    legend: bool,

    /// Delegate compression to an LLM: `copilot` (local GitHub Copilot CLI) or `api` (OpenAI-compatible endpoint; see --help-llm)
    #[arg(long, value_name = "BACKEND")]
    llm: Option<llm::Backend>,

    /// Model name for --llm (copilot: e.g. claude-haiku-4.5; api: e.g. gpt-4o-mini)
    #[arg(long, value_name = "MODEL")]
    model: Option<String>,

    /// Describe the LLM back-ends and their environment variables, then exit
    #[arg(long)]
    help_llm: bool,

    /// Print every lexicon string in rendered form (input for scripts/gen_tokcost.py) and exit
    #[arg(long, hide = true)]
    dump_lexicon: bool,
}

const LEGEND: &str = "\
BabelTele symbol legend (offline encoder)
  ⇒ then / implies / leads to      ⇐ if (mid-sentence)      ? if / whether (sentence start)
  ∵ because                        ∴ so / therefore         ¬ not / don't      ∅ none / null / nothing
  ∧ and (logic)                    ∨ or                     ≠ not equal        = is / equals (before quantities)
  ≥ at least  ≤ at most            > more than  < less than ~ approximately    ± plus or minus
  ↑ increase / up  ↓ decrease / down  📈 grow / rise  📉 fall / drop  → to / into / becomes  ← from
  @ at / in (time & place)         ∈ in / member of         ∀ every / all      ∃ there is / exists
  ∞ forever / unlimited            Δ change / difference    Σ total / sum      ⏳ wait / pending
  ✅ yes / true / done / OK        ❌ no / false / failed   ⚠ warning / risk   💀 fatal / crash
  ℹ info  🐛 debug / bug  👣 trace  🚀 launch / ship / deploy  🔧 fix / repair  🔒 secure / locked
  👤 person / user  👥 team / people  💰 money / cost  💵 dollars  ⏰ time / deadline  📅 date / schedule
  📱 mobile app  💻 computer / laptop  🌐 web / internet  📧 email  📞 call / phone  📄 document / file
  ✎ write  👁 see / look  🧠 think / know  ❓ question  ❗ important  ⭐ good / great  🗑 delete
  k=thousand  M=million  B=billion  YoY=year over year  Q1..Q4=quarters  h/min/s/d/w/mo/y=units
  Flags = countries (🇺🇸 🇩🇪 🇵🇱 …)   Months: Jan…Dec   Days: Mon…Sun   ×N = repeated N times
Structured inputs: JSON → {k:v} with ✅ ❌ ∅ and [k1,k2]:(v1,v2)(v1,v2) tables; code keeps identifiers and
string literals, symbolises operators (→ ⇒ ≠ ≤ ≥ ∧ ∨) and compresses comments; logs keep timestamps
once per date and use 💀 ❌ ⚠ ℹ 🐛 👣 for levels; dialogue speakers become U/A/S or initials.
";

const HELP_LLM: &str = "\
LLM back-ends (opt-in; the default encoder never touches the network)

  --llm copilot        Runs the local GitHub Copilot CLI:
                         copilot -s --no-color --available-tools= --no-ask-user [--model M] -p <prompt>
                       Requires `copilot` on PATH and a logged-in session.

  --llm api            POSTs to an OpenAI-compatible chat-completions endpoint via `curl`.
                         BABELTELE_API_URL   default https://api.openai.com/v1/chat/completions
                         BABELTELE_API_KEY   or OPENAI_API_KEY (sent as Authorization: Bearer …)
                         BABELTELE_MODEL     or --model, default gpt-4o-mini
                       The key is passed to curl via `-K -` (stdin), never on the command line.

Both back-ends use the compression prompt from the paper (Appendix C.1, arXiv:2606.19857).
";

fn read_input(files: &[String]) -> io::Result<String> {
    let mut buf = Vec::new();
    if files.is_empty() {
        io::stdin().lock().read_to_end(&mut buf)?;
    } else {
        for f in files {
            let data =
                std::fs::read(f).map_err(|e| io::Error::new(e.kind(), format!("{f}: {e}")))?;
            buf.extend_from_slice(&data);
            if !buf.ends_with(b"\n") {
                buf.push(b'\n');
            }
        }
    }
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

fn main() {
    let cli = Cli::parse();
    if cli.legend {
        print!("{LEGEND}");
        return;
    }
    if cli.help_llm {
        print!("{HELP_LLM}");
        return;
    }
    if cli.dump_lexicon {
        let mut w = BufWriter::new(io::stdout().lock());
        for k in lexicon::cost_keys() {
            let _ = writeln!(w, "{k}");
        }
        let _ = w.flush();
        return;
    }
    if cli.files.is_empty() && io::stdin().is_terminal() {
        eprintln!("babeltele: reading from terminal — type or paste text, end with Ctrl-D (use --help for options)");
    }
    let input = match read_input(&cli.files) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("babeltele: {e}");
            std::process::exit(1);
        }
    };
    let kind: Kind = if cli.kind.eq_ignore_ascii_case("auto") {
        detect::detect_kind(&input)
    } else {
        match cli.kind.parse() {
            Ok(k) => k,
            Err(e) => {
                eprintln!("babeltele: {e}");
                std::process::exit(2);
            }
        }
    };
    let (lang, conf): (&'static str, f64) = if cli.lang.eq_ignore_ascii_case("auto") {
        detect::detect_lang(&input)
    } else {
        match detect::LANGS
            .iter()
            .find(|l| l.eq_ignore_ascii_case(&cli.lang))
        {
            Some(l) => (l, 1.0),
            None => {
                eprintln!(
                    "babeltele: unsupported language '{}' (supported: {})",
                    cli.lang,
                    detect::LANGS.join(", ")
                );
                std::process::exit(2);
            }
        }
    };
    let opts = Options {
        level: cli.level,
        lang,
        optimize: cli.optimize,
    };

    let output = if let Some(backend) = cli.llm {
        match llm::compress(backend, cli.model.as_deref(), &input) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("babeltele: {e}");
                std::process::exit(3);
            }
        }
    } else {
        match kind {
            Kind::Json => json::encode(&input, opts),
            Kind::Code => formats::encode_code(&input, opts),
            Kind::Markdown => formats::encode_markdown(&input, opts),
            Kind::Csv => formats::encode_csv(&input, opts),
            Kind::Log => formats::encode_log(&input, opts),
            Kind::KeyValue => formats::encode_kv(&input, opts),
            Kind::Dialogue => formats::encode_dialogue(&input, opts),
            Kind::Html => formats::encode_html(&input, opts),
            Kind::Prose => prose::encode(&input, opts),
        }
    };

    if cli.stats {
        let cin = input.chars().count();
        let cout = output.chars().count();
        let pct = |a: usize, b: usize| {
            if a == 0 {
                0.0
            } else {
                100.0 * b as f64 / a as f64
            }
        };
        // real BPE token counts (o200k_base = GPT-4o/o1/GPT-5 family; a good proxy for the rest)
        let bpe = tiktoken_rs::o200k_base_singleton();
        let tin = bpe.encode_ordinary(&input).len();
        let tout = bpe.encode_ordinary(&output).len();
        eprintln!(
            "babeltele: kind={kind} lang={lang} ({conf:.2}) level={} optimize={} mode={} chars {cin} → {cout} ({:.1}%) tokens {tin} → {tout} ({:.1}%)",
            cli.level,
            match cli.optimize {
                Optimize::Tokens => "tokens",
                Optimize::Chars => "chars",
            },
            match cli.llm {
                Some(llm::Backend::Copilot) => "llm:copilot",
                Some(llm::Backend::Api) => "llm:api",
                None => "offline",
            },
            pct(cin, cout),
            pct(tin, tout),
        );
    }

    let stdout = io::stdout();
    let mut w = BufWriter::new(stdout.lock());
    let res = w
        .write_all(output.as_bytes())
        .and_then(|_| {
            if output.ends_with('\n') || output.is_empty() {
                Ok(())
            } else {
                w.write_all(b"\n")
            }
        })
        .and_then(|_| w.flush());
    if let Err(e) = res {
        if e.kind() != io::ErrorKind::BrokenPipe {
            eprintln!("babeltele: {e}");
            std::process::exit(1);
        }
    }
}
