//! Autodetection of what the input *is* (prose, JSON, code, log, …) and of the
//! natural language it is written in.

use std::fmt;

use whatlang::{Detector, Lang};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Prose,
    Json,
    Code,
    Markdown,
    Csv,
    Log,
    KeyValue,
    Dialogue,
    Html,
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Kind::Prose => "prose",
            Kind::Json => "json",
            Kind::Code => "code",
            Kind::Markdown => "markdown",
            Kind::Csv => "csv",
            Kind::Log => "log",
            Kind::KeyValue => "kv",
            Kind::Dialogue => "dialogue",
            Kind::Html => "html",
        };
        f.write_str(s)
    }
}

impl std::str::FromStr for Kind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        Ok(match s.to_ascii_lowercase().as_str() {
            "prose" | "text" | "txt" => Kind::Prose,
            "json" => Kind::Json,
            "code" | "source" | "src" => Kind::Code,
            "markdown" | "md" => Kind::Markdown,
            "csv" | "tsv" | "table" => Kind::Csv,
            "log" | "logs" => Kind::Log,
            "kv" | "keyvalue" | "key-value" | "ini" | "env" | "yaml" | "yml" => Kind::KeyValue,
            "dialogue" | "dialog" | "chat" | "transcript" => Kind::Dialogue,
            "html" | "xml" => Kind::Html,
            other => return Err(format!("unknown input kind: {other}")),
        })
    }
}

/// Language codes the lexicon knows about (ISO 639-1).
pub const LANGS: &[&str] = &[
    "en", "pl", "de", "fr", "es", "it", "pt", "nl", "ru", "uk", "cs", "sk", "sv", "da", "no", "fi",
    "tr", "hu", "ro",
];

fn lang_code(lang: Lang) -> Option<&'static str> {
    Some(match lang {
        Lang::Eng => "en",
        Lang::Pol => "pl",
        Lang::Deu => "de",
        Lang::Fra => "fr",
        Lang::Spa => "es",
        Lang::Ita => "it",
        Lang::Por => "pt",
        Lang::Nld => "nl",
        Lang::Rus => "ru",
        Lang::Ukr => "uk",
        Lang::Ces => "cs",
        Lang::Slk => "sk",
        Lang::Swe => "sv",
        Lang::Dan => "da",
        Lang::Nob => "no",
        Lang::Fin => "fi",
        Lang::Tur => "tr",
        Lang::Hun => "hu",
        Lang::Ron => "ro",
        _ => return None,
    })
}

/// Very common function words per language, used as a tie-breaker / fallback
/// for short inputs where trigram statistics are unreliable.
const STOPWORDS: &[(&str, &[&str])] = &[
    (
        "en",
        &[
            "the", "and", "is", "are", "of", "to", "in", "that", "it", "for", "with", "was",
            "this", "not", "be",
        ],
    ),
    (
        "pl",
        &[
            "i", "w", "się", "nie", "na", "jest", "z", "do", "to", "że", "są", "oraz", "który",
            "przez", "być",
        ],
    ),
    (
        "de",
        &[
            "und", "der", "die", "das", "ist", "nicht", "mit", "ein", "eine", "sind", "auf", "für",
            "von", "zu", "den",
        ],
    ),
    (
        "fr",
        &[
            "le", "la", "les", "et", "est", "des", "une", "un", "pas", "que", "pour", "dans",
            "sont", "avec", "sur",
        ],
    ),
    (
        "es",
        &[
            "el", "la", "los", "las", "y", "es", "una", "un", "que", "para", "con", "por", "son",
            "del", "en",
        ],
    ),
    (
        "it",
        &[
            "il", "la", "e", "è", "che", "di", "un", "una", "per", "con", "sono", "non", "del",
            "della", "gli",
        ],
    ),
    (
        "pt",
        &[
            "o", "a", "os", "as", "e", "é", "que", "um", "uma", "para", "com", "não", "são", "do",
            "da",
        ],
    ),
    (
        "nl",
        &[
            "de", "het", "een", "en", "is", "van", "niet", "zijn", "met", "voor", "dat", "op",
            "te", "wordt", "ook",
        ],
    ),
    (
        "ru",
        &[
            "и", "в", "не", "на", "что", "это", "с", "для", "как", "по", "из", "или", "но", "он",
            "она",
        ],
    ),
    (
        "uk",
        &[
            "і", "в", "не", "на", "що", "це", "з", "для", "як", "по", "та", "або", "але", "він",
            "вона",
        ],
    ),
    (
        "cs",
        &[
            "a", "je", "se", "na", "v", "že", "s", "pro", "jsou", "není", "to", "ale", "jako",
            "nebo", "být",
        ],
    ),
    (
        "sk",
        &[
            "a", "je", "sa", "na", "v", "že", "s", "pre", "sú", "nie", "to", "ale", "ako", "alebo",
            "byť",
        ],
    ),
    (
        "sv",
        &[
            "och", "är", "att", "det", "som", "en", "ett", "på", "för", "med", "inte", "av", "den",
            "till", "har",
        ],
    ),
    (
        "da",
        &[
            "og", "er", "at", "det", "som", "en", "et", "på", "for", "med", "ikke", "af", "den",
            "til", "har",
        ],
    ),
    (
        "no",
        &[
            "og", "er", "at", "det", "som", "en", "et", "på", "for", "med", "ikke", "av", "den",
            "til", "har",
        ],
    ),
    (
        "fi",
        &[
            "ja", "on", "ei", "että", "se", "oli", "kun", "hän", "mutta", "tai", "ovat", "jos",
            "niin", "myös", "voi",
        ],
    ),
    (
        "tr",
        &[
            "ve", "bir", "bu", "için", "ile", "de", "da", "olan", "olarak", "çok", "daha", "gibi",
            "ama", "en", "var",
        ],
    ),
    (
        "hu",
        &[
            "a", "az", "és", "hogy", "nem", "is", "egy", "van", "meg", "de", "ez", "azt", "csak",
            "már", "mint",
        ],
    ),
    (
        "ro",
        &[
            "și", "de", "la", "în", "este", "cu", "pe", "care", "un", "o", "nu", "din", "sunt",
            "pentru", "sau",
        ],
    ),
];

/// Words that look like natural language (used to feed the language detector).
fn natural_words(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        let t = line.trim();
        // Skip lines that are mostly code / markup / data.
        if t.starts_with("http") || t.starts_with('{') || t.starts_with('[') || t.starts_with('<') {
            continue;
        }
        for w in t.split(|c: char| !(c.is_alphabetic() || c == '\'' || c == '’')) {
            if w.chars().count() >= 2
                && w.chars()
                    .all(|c| c.is_alphabetic() || c == '\'' || c == '’')
            {
                out.push_str(w);
                out.push(' ');
            }
        }
    }
    out
}

/// Detect the natural language; returns (code, confidence in 0..=1).
pub fn detect_lang(text: &str) -> (&'static str, f64) {
    let sample = natural_words(text);
    let words: Vec<String> = sample
        .split_whitespace()
        .map(|w| w.to_lowercase())
        .collect();
    if words.is_empty() {
        return ("en", 0.0);
    }

    // Stopword scoring (works on tiny inputs, resolves close Slavic/Romance pairs).
    let mut best_sw: Option<(&'static str, usize)> = None;
    let mut scores: Vec<(&'static str, usize)> = Vec::with_capacity(STOPWORDS.len());
    for (code, list) in STOPWORDS {
        let hits = words.iter().filter(|w| list.contains(&w.as_str())).count();
        scores.push((code, hits));
        if hits > 0 && best_sw.is_none_or(|(_, b)| hits > b) {
            best_sw = Some((code, hits));
        }
    }

    let allow: Vec<Lang> = [
        Lang::Eng,
        Lang::Pol,
        Lang::Deu,
        Lang::Fra,
        Lang::Spa,
        Lang::Ita,
        Lang::Por,
        Lang::Nld,
        Lang::Rus,
        Lang::Ukr,
        Lang::Ces,
        Lang::Slk,
        Lang::Swe,
        Lang::Dan,
        Lang::Nob,
        Lang::Fin,
        Lang::Tur,
        Lang::Hun,
        Lang::Ron,
    ]
    .to_vec();
    let detector = Detector::with_allowlist(allow);
    let trig = detector
        .detect(&sample)
        .and_then(|i| lang_code(i.lang()).map(|c| (c, i.confidence())));

    match (trig, best_sw) {
        (Some((code, conf)), Some((sw, hits))) => {
            if code == sw {
                return (code, conf.max(0.5));
            }
            // Disagreement: trust stopwords when they are clear, or when the
            // trigram model is unsure (short text).
            let sw_ratio = hits as f64 / words.len() as f64;
            let second = scores
                .iter()
                .filter(|(c, _)| *c != sw)
                .map(|(_, h)| *h)
                .max()
                .unwrap_or(0);
            if (hits >= 3 && hits > second + 1) || conf < 0.3 || sw_ratio > 0.25 {
                (sw, (sw_ratio * 3.0).min(0.95))
            } else {
                (code, conf)
            }
        }
        (Some((code, conf)), None) => {
            // No stopword evidence at all and an unsure trigram model (typical
            // for 2–4 word inputs): English is by far the most likely input.
            if conf < 0.2 && words.len() < 8 && code != "en" {
                ("en", 0.2)
            } else {
                (code, conf)
            }
        }
        (None, Some((sw, hits))) => (sw, (hits as f64 / words.len() as f64 * 3.0).min(0.9)),
        (None, None) => ("en", 0.0),
    }
}

fn looks_like_json(t: &str) -> bool {
    let t = t.trim_start_matches('\u{feff}').trim();
    if !((t.starts_with('{') && t.ends_with('}')) || (t.starts_with('[') && t.ends_with(']'))) {
        return false;
    }
    serde_json::from_str::<serde_json::Value>(t).is_ok()
}

fn looks_like_jsonl(t: &str) -> bool {
    let lines: Vec<&str> = t.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    lines.len() >= 2
        && lines.iter().all(|l| {
            l.starts_with('{')
                && l.ends_with('}')
                && serde_json::from_str::<serde_json::Value>(l).is_ok()
        })
}

fn looks_like_html(t: &str) -> bool {
    let lower = t.trim_start().to_ascii_lowercase();
    lower.starts_with("<!doctype html")
        || lower.starts_with("<html")
        || lower.starts_with("<?xml")
        || (lower.matches('<').count() >= 3
            && [
                "<p", "<div", "<span", "<br", "<a ", "<li", "<h1", "<h2", "<h3", "<td", "<body",
                "<ul", "<table",
            ]
            .iter()
            .filter(|tag| lower.contains(*tag))
            .count()
                >= 2)
}

/// Heuristic: does a line look like a line of source code?
fn code_like_line(l: &str) -> bool {
    let t = l.trim();
    if t.is_empty() {
        return false;
    }
    let ends = t.ends_with(';')
        || t.ends_with('{')
        || t.ends_with('}')
        || t.ends_with(':') && t.contains('(')
        || t.ends_with(')') && t.contains('(');
    let starts = [
        "fn ",
        "pub ",
        "let ",
        "const ",
        "var ",
        "def ",
        "class ",
        "import ",
        "from ",
        "#include",
        "package ",
        "func ",
        "return",
        "if ",
        "if(",
        "else",
        "for ",
        "for(",
        "while ",
        "while(",
        "use ",
        "struct ",
        "enum ",
        "impl ",
        "static ",
        "void ",
        "int ",
        "public ",
        "private ",
        "protected ",
        "async ",
        "await ",
        "try",
        "catch",
        "except",
        "elif ",
        "match ",
        "switch",
        "case ",
        "break",
        "continue",
        "throw ",
        "raise ",
        "@",
        "#!",
        "//",
        "/*",
        "*/",
        " * ",
        "#define",
        "#if",
        "#endif",
        "type ",
        "interface ",
        "export ",
        "module ",
        "require(",
        "print(",
        "console.",
        "std::",
        "self.",
        "this.",
        "}",
        "{",
        "namespace ",
        "using ",
        "typedef ",
        "template",
        "mod ",
        "extern ",
        "unsafe ",
        "loop",
        "yield ",
        "lambda",
        "with ",
        "assert",
        "select ",
        "insert ",
        "update ",
        "delete ",
        "create ",
        "drop ",
        "alter ",
        "where ",
        "join ",
        "group by",
        "order by",
    ];
    let lower = t.to_ascii_lowercase();
    let kw = starts.iter().any(|s| lower.starts_with(s));
    let ops = [
        " = ", "==", "!=", "=>", "->", "&&", "||", "+=", "-=", "::", "();", "()", "[]", "{}", ");",
    ];
    let opcount = ops.iter().filter(|o| t.contains(*o)).count();
    let hash_comment = t.starts_with('#') && !t.starts_with("# ") && !t.starts_with("#!");
    ends || kw
        || opcount >= 2
        || (opcount >= 1 && !t.contains(' '))
        || hash_comment
        || t.contains("($")
        || t.contains("=\"")
}

fn markdown_score(t: &str) -> usize {
    let mut score = 0;
    let mut fence = false;
    for l in t.lines() {
        let s = l.trim_start();
        if s.starts_with("```") || s.starts_with("~~~") {
            fence = !fence;
            score += 3;
            continue;
        }
        if fence {
            continue;
        }
        if s.starts_with("# ")
            || s.starts_with("## ")
            || s.starts_with("### ")
            || s.starts_with("#### ")
        {
            score += 3;
        } else if s.starts_with("- ")
            || s.starts_with("* ")
            || s.starts_with("+ ")
            || s.starts_with("> ")
        {
            score += 1;
        } else if s.starts_with('|') && s.ends_with('|') {
            score += 2;
        } else if s.len() > 2
            && s.as_bytes()[0].is_ascii_digit()
            && (s.starts_with(|c: char| c.is_ascii_digit()) && s.contains(". "))
            && s.find(". ").is_some_and(|i| i <= 3)
        {
            score += 1;
        }
        if s.contains("](") || s.contains("**") || s.contains("`") {
            score += 1;
        }
    }
    score
}

fn log_like_line(l: &str) -> bool {
    let t = l.trim_start();
    let b = t.as_bytes();
    // ISO / syslog / apache style timestamps at line start
    let iso = b.len() >= 10
        && b[..4].iter().all(u8::is_ascii_digit)
        && (b[4] == b'-' || b[4] == b'/')
        && b[7] == b[4]
        && b[5..7].iter().all(u8::is_ascii_digit)
        && b[8..10].iter().all(u8::is_ascii_digit);
    let clock = b.len() >= 8
        && b[..2].iter().all(u8::is_ascii_digit)
        && b[2] == b':'
        && b[3..5].iter().all(u8::is_ascii_digit)
        && b[5] == b':';
    let bracket_ts = t.starts_with('[') && b.len() > 5 && b[1].is_ascii_digit();
    let months = [
        "Jan ", "Feb ", "Mar ", "Apr ", "May ", "Jun ", "Jul ", "Aug ", "Sep ", "Oct ", "Nov ",
        "Dec ",
    ];
    let syslog = months.iter().any(|m| t.starts_with(m))
        && b.len() > 15
        && b[7..9].iter().all(|c| c.is_ascii_digit() || *c == b' ');
    let upper = t.to_ascii_uppercase();
    let level = [
        "ERROR", "WARN", "INFO", "DEBUG", "TRACE", "FATAL", "CRITICAL", "NOTICE", "PANIC", "SEVERE",
    ]
    .iter()
    .any(|lv| {
        upper.contains(&format!("[{lv}"))
            || upper.contains(&format!(" {lv} "))
            || upper.contains(&format!(" {lv}:"))
            || upper.starts_with(&format!("{lv} "))
            || upper.starts_with(&format!("{lv}:"))
            || upper.contains(&format!("\t{lv}\t"))
            || upper.contains(&format!(" {lv}]"))
            || upper.contains(&format!("LEVEL={lv}"))
    });
    (iso || clock || bracket_ts || syslog)
        && (level || t.contains(" - ") || t.contains(": ") || t.contains(" ] ") || t.contains("] "))
        || (level && (t.contains(" - ") || t.contains(": ")) && b.len() > 12)
}

fn csv_like(t: &str) -> bool {
    let lines: Vec<&str> = t.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.len() < 2 {
        return false;
    }
    for sep in [',', '\t', ';', '|'] {
        let counts: Vec<usize> = lines
            .iter()
            .take(50)
            .map(|l| l.matches(sep).count())
            .collect();
        let first = counts[0];
        if first == 0 {
            continue;
        }
        let same = counts.iter().filter(|c| **c == first).count();
        // Prose sentences also contain commas, so demand column consistency and no sentence punctuation.
        let prose_like = sep == ','
            && lines
                .iter()
                .take(50)
                .filter(|l| l.trim_end().ends_with('.') || l.contains(". "))
                .count()
                * 3
                > lines.len().min(50);
        if same * 10 >= counts.len() * 9 && !prose_like {
            if sep == ','
                && lines
                    .iter()
                    .take(50)
                    .any(|l| l.split(',').any(|c| c.split_whitespace().count() > 14))
            {
                continue;
            }
            // a single comma per line with a space after it is a sentence, not a 2-column table
            if sep == ',' && first == 1 && lines.iter().take(50).any(|l| l.contains(", ")) {
                continue;
            }
            return true;
        }
    }
    false
}

fn kv_like(t: &str) -> bool {
    let lines: Vec<&str> = t
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();
    if lines.len() < 2 {
        return false;
    }
    let kv = lines
        .iter()
        .filter(|l| {
            let Some(i) = l.find(|c| c == '=' || c == ':') else {
                return false;
            };
            let key = l[..i].trim();
            !key.is_empty()
                && key.split_whitespace().count() <= 3
                && key.chars().all(|c| {
                    c.is_alphanumeric()
                        || c == '_'
                        || c == '-'
                        || c == '.'
                        || c == ' '
                        || c == '['
                        || c == ']'
                })
                && !l[i + 1..].trim_start().starts_with(':')
        })
        .count();
    kv * 10 >= lines.len() * 8
}

fn dialogue_like(t: &str) -> bool {
    let lines: Vec<&str> = t.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    if lines.len() < 2 {
        return false;
    }
    let speaker = lines
        .iter()
        .filter(|l| {
            let Some(i) = l.find(':') else { return false };
            let label = &l[..i];
            let n = label.split_whitespace().count();
            (1..=3).contains(&n)
                && label
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_uppercase() || c == '[' || c == '<' || c == '*')
                && label.chars().all(|c| {
                    c.is_alphanumeric()
                        || c == ' '
                        || c == '_'
                        || c == '['
                        || c == ']'
                        || c == '<'
                        || c == '>'
                        || c == '*'
                        || c == '-'
                        || c == '.'
                })
                && l[i + 1..].trim().split_whitespace().count() >= 1
        })
        .count();
    if speaker * 10 < lines.len() * 6 {
        return false;
    }
    // speakers repeat in a transcript and say whole sentences; keys in a config block do neither
    let mut labels: Vec<String> = lines
        .iter()
        .filter_map(|l| l.split_once(':').map(|(a, _)| a.trim().to_lowercase()))
        .collect();
    labels.sort();
    let repeats = labels.windows(2).any(|w| w[0] == w[1]);
    let sentences = lines
        .iter()
        .filter(|l| {
            l.split_once(':')
                .is_some_and(|(_, v)| v.split_whitespace().count() >= 3)
        })
        .count();
    repeats || sentences * 10 >= lines.len() * 6
}

pub fn detect_kind(text: &str) -> Kind {
    let t = text.trim();
    if t.is_empty() {
        return Kind::Prose;
    }
    if looks_like_json(t) || looks_like_jsonl(t) {
        return Kind::Json;
    }
    if looks_like_html(t) {
        return Kind::Html;
    }
    let lines: Vec<&str> = t.lines().filter(|l| !l.trim().is_empty()).collect();
    let n = lines.len().max(1);
    let code = lines.iter().filter(|l| code_like_line(l)).count();
    let logs = lines.iter().filter(|l| log_like_line(l)).count();
    if logs * 2 >= n && n >= 2 || (n == 1 && logs == 1 && code == 0) {
        return Kind::Log;
    }
    let md = markdown_score(t);
    // key=value blocks (env files, ini, simple yaml) — a leading "# comment" is not a heading
    if kv_like(t) && md <= 3 && !dialogue_like(t) {
        return Kind::KeyValue;
    }
    if t.starts_with("```") || t.starts_with("# ") && lines.len() > 1 {
        return Kind::Markdown;
    }
    if code * 5 >= n * 2 && (n > 1 || t.contains('(') || t.contains(';')) && md * 2 < n {
        return Kind::Code;
    }
    if md >= 3 && md * 4 >= n {
        return Kind::Markdown;
    }
    if dialogue_like(t) {
        return Kind::Dialogue;
    }
    if csv_like(t) && n >= 2 {
        return Kind::Csv;
    }
    if kv_like(t) {
        return Kind::KeyValue;
    }
    Kind::Prose
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds() {
        assert_eq!(detect_kind(r#"{"a": 1, "b": [1,2]}"#), Kind::Json);
        assert_eq!(
            detect_kind("fn main() {\n    println!(\"hi\");\n}\n"),
            Kind::Code
        );
        assert_eq!(
            detect_kind("# Title\n\nSome text\n\n- a\n- b\n"),
            Kind::Markdown
        );
        assert_eq!(
            detect_kind("2024-01-02 10:00:00 INFO started\n2024-01-02 10:00:01 ERROR failed: x\n"),
            Kind::Log
        );
        assert_eq!(
            detect_kind("name,age,city\nbob,3,nyc\nann,5,la\n"),
            Kind::Csv
        );
        assert_eq!(
            detect_kind("User: hi there\nAssistant: hello, how can I help?\nUser: nothing\n"),
            Kind::Dialogue
        );
        assert_eq!(
            detect_kind("host = localhost\nport = 5432\nuser = admin\n"),
            Kind::KeyValue
        );
        assert_eq!(
            detect_kind("<html><body><p>Hi</p><div>x</div></body></html>"),
            Kind::Html
        );
        assert_eq!(detect_kind("The quick brown fox jumps over the lazy dog. It was a sunny day, and everyone was happy."), Kind::Prose);
    }

    #[test]
    fn langs() {
        assert_eq!(
            detect_lang(
                "The revenue is projected to rise by thirty percent and the team is happy."
            )
            .0,
            "en"
        );
        assert_eq!(detect_lang("Przychody w trzecim kwartale wzrosną o około trzydzieści procent, jeśli wszystko pójdzie zgodnie z planem.").0, "pl");
        assert_eq!(
            detect_lang(
                "Der Umsatz wird im dritten Quartal voraussichtlich um dreißig Prozent steigen."
            )
            .0,
            "de"
        );
        assert_eq!(
            detect_lang("Выручка в третьем квартале вырастет примерно на тридцать процентов.").0,
            "ru"
        );
    }
}
