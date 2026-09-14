//! Optional LLM back-ends (never used unless `--llm` is passed).
//!
//! * `copilot` — shells out to the local GitHub Copilot CLI (`copilot -p`).
//! * `api`     — OpenAI-compatible chat completions endpoint via `curl`.
//!   The API key never appears on the command line: it is passed to curl through
//!   a `-K -` config read from stdin, and the request body goes through a
//!   0600 temporary file.

use std::io::Write;
use std::process::{Command, Stdio};

/// System prompt from the BabelTele paper, Appendix C.1 (arXiv:2606.19857).
pub const PROMPT: &str = "your task: compress verbose human text into minimal Token sequence. Audience ≠ human, but another equally intelligent LLM.\n\
Core Directive\n\
Omnilingual: ignore single-language grammar; traverse all human languages (Chinese, English, German compounds, Japanese Kanji, Latin roots, etc.), pick highest info-density words.\n\
Symbolic Collapse: optionally replace conjunctions, emotions, long sentences with Emoji, math/logical symbols (=>, ∈, ≠), punctuation.\n\
Universality: any LLM should fully understand compressed output without a codebook.\n\
Lossless: retain all information & details.\n\
Output ONLY the compressed text, nothing else.\n\
Compress the content below:\n";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    Copilot,
    Api,
}

impl std::str::FromStr for Backend {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "copilot" | "cli" => Ok(Backend::Copilot),
            "api" | "openai" | "http" => Ok(Backend::Api),
            other => Err(format!("unknown LLM backend '{other}' (use: copilot, api)")),
        }
    }
}

pub fn compress(backend: Backend, model: Option<&str>, text: &str) -> Result<String, String> {
    match backend {
        Backend::Copilot => copilot(model, text),
        Backend::Api => api(model, text),
    }
}

fn copilot(model: Option<&str>, text: &str) -> Result<String, String> {
    let mut cmd = Command::new("copilot");
    cmd.args(["-s", "--no-color", "--available-tools=", "--no-ask-user"]);
    if let Some(m) = model {
        cmd.args(["--model", m]);
    }
    cmd.arg("-p").arg(format!("{PROMPT}\n{text}"));
    cmd.stdin(Stdio::null());
    let out = cmd
        .output()
        .map_err(|e| format!("cannot run `copilot`: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "copilot failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(clean(&String::from_utf8_lossy(&out.stdout)))
}

fn api(model: Option<&str>, text: &str) -> Result<String, String> {
    let url = std::env::var("BABELTELE_API_URL")
        .unwrap_or_else(|_| "https://api.openai.com/v1/chat/completions".into());
    let key = std::env::var("BABELTELE_API_KEY")
        .or_else(|_| std::env::var("OPENAI_API_KEY"))
        .map_err(|_| "set BABELTELE_API_KEY (or OPENAI_API_KEY) for --llm api".to_string())?;
    let model = model
        .map(str::to_string)
        .or_else(|| std::env::var("BABELTELE_MODEL").ok())
        .unwrap_or_else(|| "gpt-4o-mini".into());
    let body = serde_json::json!({
        "model": model,
        "temperature": 0,
        "messages": [
            {"role": "system", "content": PROMPT},
            {"role": "user", "content": text}
        ]
    });
    let mut tmp = std::env::temp_dir();
    tmp.push(format!("babeltele-{}.json", std::process::id()));
    {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp)
            .map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
        }
        f.write_all(body.to_string().as_bytes())
            .map_err(|e| e.to_string())?;
    }
    let mut child = Command::new("curl")
        .args([
            "-sS",
            "-K",
            "-",
            "-H",
            "Content-Type: application/json",
            "--data-binary",
        ])
        .arg(format!("@{}", tmp.display()))
        .arg(&url)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot run `curl`: {e}"))?;
    {
        let mut stdin = child.stdin.take().unwrap();
        let _ = writeln!(
            stdin,
            "header = \"Authorization: Bearer {}\"",
            key.replace('"', "")
        );
    }
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&tmp);
    if !out.status.success() {
        return Err(format!(
            "curl failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).map_err(|e| format!("bad API response: {e}"))?;
    if let Some(err) = v.get("error") {
        return Err(format!("API error: {err}"));
    }
    let content = v["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| format!("unexpected API response: {v}"))?;
    Ok(clean(content))
}

fn clean(s: &str) -> String {
    let t = s.trim();
    // strip a wrapping code fence if the model added one
    if t.starts_with("```") {
        let inner = t.trim_start_matches('`');
        let inner = inner.split_once('\n').map_or("", |x| x.1);
        return inner.trim_end_matches('`').trim().to_string();
    }
    t.to_string()
}
