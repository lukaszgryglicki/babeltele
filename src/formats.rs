//! Encoders for structured / semi-structured inputs: source code, Markdown,
//! CSV/TSV, logs, key-value blocks, dialogue transcripts and HTML.

use std::collections::HashMap;

use crate::prose::{self, Options};

/// Compress the text portion of a line while leaving leading indentation alone.
fn prose_inline(s: &str, opts: Options) -> String {
    let out = prose::encode(s, opts);
    if out.is_empty() {
        s.trim().to_string()
    } else {
        out
    }
}

// ---------------------------------------------------------------- code ----

fn split_comment(line: &str) -> Option<(usize, &str)> {
    // returns (index where comment starts, comment marker)
    let mut in_str: Option<char> = None;
    let b: Vec<char> = line.chars().collect();
    let mut i = 0;
    let mut byte = 0;
    while i < b.len() {
        let c = b[i];
        if let Some(q) = in_str {
            if c == '\\' {
                i += 1;
                byte += c.len_utf8() + b.get(i).map_or(0, |x| x.len_utf8());
                i += 1;
                continue;
            }
            if c == q {
                in_str = None;
            }
        } else {
            if c == '"' || c == '\'' || c == '`' {
                in_str = Some(c);
            } else if c == '/' && b.get(i + 1) == Some(&'/') {
                return Some((byte, "//"));
            } else if c == '#'
                && !(i + 1 < b.len()
                    && (b[i + 1] == '!'
                        || b[i + 1] == '['
                        || b[i + 1] == 'i'
                        || b[i + 1] == 'd'
                        || b[i + 1] == 'e'
                        || b[i + 1] == 'p'))
            {
                return Some((byte, "#"));
            } else if c == '-' && b.get(i + 1) == Some(&'-') && b.get(i + 2) == Some(&' ') {
                return Some((byte, "--"));
            }
        }
        byte += c.len_utf8();
        i += 1;
    }
    None
}

fn code_ops(s: &str, level: u8) -> String {
    // operator symbolisation outside string literals
    let mut out = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    let mut in_str: Option<char> = None;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if let Some(q) = in_str {
            out.push(c);
            if c == '\\' && i + 1 < chars.len() {
                out.push(chars[i + 1]);
                i += 2;
                continue;
            }
            if c == q {
                in_str = None;
            }
            i += 1;
            continue;
        }
        if c == '"' || c == '\'' || c == '`' {
            in_str = Some(c);
            out.push(c);
            i += 1;
            continue;
        }
        let rest: String = chars[i..chars.len().min(i + 3)].iter().collect();
        let mut matched = None;
        for (pat, rep) in [
            ("===", "≡"),
            ("!==", "≢"),
            ("->", "→"),
            ("=>", "⇒"),
            ("!=", "≠"),
            ("<=", "≤"),
            (">=", "≥"),
            ("&&", "∧"),
            ("||", "∨"),
            ("<-", "←"),
        ] {
            if rest.starts_with(pat) {
                matched = Some((pat.chars().count(), rep));
                break;
            }
        }
        if let Some((n, rep)) = matched {
            out.push_str(rep);
            i += n;
            continue;
        }
        if level >= 2 && c == ' ' {
            // drop spaces around punctuation/operators
            let prev = out.chars().last();
            let next = chars.get(i + 1).copied();
            let tight = |x: Option<char>| {
                x.is_some_and(|x| {
                    matches!(
                        x,
                        '=' | ','
                            | '('
                            | ')'
                            | '{'
                            | '}'
                            | '['
                            | ']'
                            | ';'
                            | ':'
                            | '+'
                            | '-'
                            | '*'
                            | '/'
                            | '<'
                            | '>'
                            | '≠'
                            | '≤'
                            | '≥'
                            | '→'
                            | '⇒'
                            | '∧'
                            | '∨'
                            | '≡'
                            | '≢'
                            | '|'
                            | '&'
                            | '.'
                            | '!'
                            | '?'
                            | '%'
                    )
                })
            };
            if tight(prev) || tight(next) {
                i += 1;
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

fn code_words(s: &str, level: u8, tokens: bool) -> String {
    if level < 2 {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut in_str: Option<char> = None;
    let mut word = String::new();
    let flush = |word: &mut String, out: &mut String| {
        if word.is_empty() {
            return;
        }
        // ∅ ✗ ↩ cost 2–3 tokens where the keyword costs 1: chars mode only
        let rep = match word.as_str() {
            "null" | "None" | "nil" | "NULL" | "undefined" if !tokens => Some("∅"),
            "true" | "True" | "TRUE" => Some("✓"),
            "false" | "False" | "FALSE" if !tokens => Some("✗"),
            "return" if level >= 3 && !tokens => Some("↩"),
            "function" if level >= 3 => Some("fn"),
            "public" if level >= 3 => Some("pub"),
            "private" if level >= 3 => Some("priv"),
            "protected" if level >= 3 => Some("prot"),
            "import" if level >= 3 => Some("imp"),
            "export" if level >= 3 => Some("exp"),
            "const" if level >= 3 => Some("cst"),
            "string" if level >= 3 => Some("str"),
            "boolean" if level >= 3 => Some("bool"),
            "integer" if level >= 3 => Some("int"),
            _ => None,
        };
        match rep {
            Some(r) => out.push_str(r),
            None => out.push_str(word),
        }
        word.clear();
    };
    for c in s.chars() {
        if let Some(q) = in_str {
            out.push(c);
            if c == q {
                in_str = None;
            }
            continue;
        }
        if c == '"' || c == '\'' || c == '`' {
            flush(&mut word, &mut out);
            in_str = Some(c);
            out.push(c);
            continue;
        }
        if c.is_alphanumeric() || c == '_' {
            word.push(c);
        } else {
            flush(&mut word, &mut out);
            out.push(c);
        }
    }
    flush(&mut word, &mut out);
    out
}

pub fn encode_code(text: &str, opts: Options) -> String {
    let mut out = Vec::new();
    let mut in_block_comment = false;
    for raw in text.lines() {
        let line = raw.trim_end();
        if line.trim().is_empty() {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        let ind = if opts.level >= 2 {
            " ".repeat((indent / 4).max(if indent > 0 { 1 } else { 0 }))
        } else {
            line[..indent].to_string()
        };
        let body = line.trim_start();
        if in_block_comment {
            let end = body.find("*/");
            let txt = end
                .map_or(body, |e| &body[..e])
                .trim_start_matches('*')
                .trim();
            if end.is_some() {
                in_block_comment = false;
            }
            if !txt.is_empty() {
                out.push(format!("{ind}// {}", prose_inline(txt, opts)));
            }
            continue;
        }
        if body.starts_with("/*") || body.starts_with("/**") {
            let inner = body.trim_start_matches('/').trim_start_matches('*');
            let end = inner.find("*/");
            let txt = end.map_or(inner, |e| &inner[..e]).trim();
            if end.is_none() {
                in_block_comment = true;
            }
            if !txt.is_empty() {
                out.push(format!("{ind}// {}", prose_inline(txt, opts)));
            }
            continue;
        }
        if body.starts_with("\"\"\"") || body.starts_with("'''") {
            out.push(format!("{ind}{body}"));
            continue;
        }
        let (code, comment) = match split_comment(body) {
            Some((idx, marker)) => (
                &body[..idx],
                Some((marker, body[idx + marker.len()..].trim())),
            ),
            None => (body, None),
        };
        let mut rendered = code_words(
            &code_ops(code.trim_end(), opts.level),
            opts.level,
            opts.tokens(),
        );
        if let Some((marker, txt)) = comment {
            if !txt.is_empty() {
                let c = prose_inline(txt, opts);
                if !rendered.is_empty() {
                    rendered.push(' ');
                }
                rendered.push_str(marker);
                rendered.push_str(&c);
            }
        }
        if rendered.trim().is_empty() {
            continue;
        }
        out.push(format!("{ind}{rendered}"));
    }
    out.join("\n")
}

// ------------------------------------------------------------ markdown ----

/// A space costs nothing before a word in BPE tokenizers, so token mode keeps
/// markers such as `#`, `•`, `INFO` readable; chars mode glues them.
fn sp(opts: Options) -> &'static str {
    if opts.tokens() {
        " "
    } else {
        ""
    }
}

fn md_inline(s: &str, opts: Options) -> String {
    // links / images first, then emphasis, then prose
    let mut t = s.to_string();
    let re_img = regex::Regex::new(r"!\[([^\]]*)\]\(([^)]+)\)").unwrap();
    t = re_img.replace_all(&t, "🖼$1→$2").into_owned();
    let re_link = regex::Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").unwrap();
    t = re_link.replace_all(&t, "$1→$2").into_owned();
    for m in ["**", "__", "~~"] {
        t = t.replace(m, "");
    }
    // single * / _ emphasis around words
    let re_em = regex::Regex::new(r"(^|\s)[*_]([^*_\s][^*_]*?)[*_](\s|$|[.,;:!?])").unwrap();
    t = re_em.replace_all(&t, "$1$2$3").into_owned();
    prose_inline(&t, opts)
}

pub fn encode_markdown(text: &str, opts: Options) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut fence: Option<String> = None;
    let mut code_buf: Vec<String> = Vec::new();
    let mut para: Vec<String> = Vec::new();
    let flush_para = |para: &mut Vec<String>, out: &mut Vec<String>| {
        if !para.is_empty() {
            let joined = para.join(" ");
            out.push(md_inline(&joined, opts));
            para.clear();
        }
    };
    for raw in text.lines() {
        let line = raw.trim_end();
        let t = line.trim_start();
        if let Some(f) = &fence {
            if t.starts_with(f.as_str()) {
                let code = encode_code(&code_buf.join("\n"), opts);
                out.push(format!("```\n{code}\n```"));
                code_buf.clear();
                fence = None;
            } else {
                code_buf.push(line.to_string());
            }
            continue;
        }
        if t.starts_with("```") || t.starts_with("~~~") {
            flush_para(&mut para, &mut out);
            fence = Some(t[..3].to_string());
            continue;
        }
        if t.is_empty() {
            flush_para(&mut para, &mut out);
            continue;
        }
        if t.starts_with('#') {
            flush_para(&mut para, &mut out);
            let level = t.chars().take_while(|c| *c == '#').count();
            let title = t[level..].trim();
            out.push(format!(
                "{}{}{}",
                "#".repeat(level),
                sp(opts),
                md_inline(title, opts)
            ));
            continue;
        }
        if t.starts_with("- ") || t.starts_with("* ") || t.starts_with("+ ") || t.starts_with("• ")
        {
            flush_para(&mut para, &mut out);
            let indent = (line.len() - t.len()) / 2;
            let item = &t[2..];
            let (marker, rest) = if item.starts_with("[ ] ") {
                ("☐", &item[4..])
            } else if item.starts_with("[x] ") || item.starts_with("[X] ") {
                ("☑", &item[4..])
            } else {
                ("•", item)
            };
            out.push(format!(
                "{}{marker}{}{}",
                " ".repeat(indent),
                sp(opts),
                md_inline(rest, opts)
            ));
            continue;
        }
        if let Some(pos) = t
            .find(". ")
            .filter(|p| *p <= 3 && t[..*p].chars().all(|c| c.is_ascii_digit()))
        {
            flush_para(&mut para, &mut out);
            out.push(format!("{}){}", &t[..pos], md_inline(&t[pos + 2..], opts)));
            continue;
        }
        if t.starts_with("> ") || t == ">" {
            flush_para(&mut para, &mut out);
            // "❝" is 2 tokens where "> " is 1
            let marker = if opts.tokens() { "> " } else { "❝" };
            out.push(format!(
                "{marker}{}",
                md_inline(t.trim_start_matches('>').trim(), opts)
            ));
            continue;
        }
        if t.starts_with('|') {
            flush_para(&mut para, &mut out);
            let inner = t.trim_matches('|');
            if inner.chars().all(|c| matches!(c, '-' | ':' | '|' | ' ')) {
                continue;
            }
            let cells: Vec<String> = inner.split('|').map(|c| csv_cell(c.trim(), opts)).collect();
            out.push(cells.join("|"));
            continue;
        }
        if t.chars().all(|c| matches!(c, '-' | '*' | '_' | ' ')) && t.len() >= 3 {
            flush_para(&mut para, &mut out);
            out.push("—".into());
            continue;
        }
        para.push(t.to_string());
    }
    flush_para(&mut para, &mut out);
    if fence.is_some() && !code_buf.is_empty() {
        out.push(format!(
            "```\n{}\n```",
            encode_code(&code_buf.join("\n"), opts)
        ));
    }
    out.join("\n")
}

// ----------------------------------------------------------------- csv ----

fn csv_cell(c: &str, opts: Options) -> String {
    let c = c.trim().trim_matches('"').trim();
    // ✅ ❌ ∅ cost 1–2 tokens; the literals cost 1 (or 0 for an empty cell): chars mode only
    if !opts.tokens() {
        match c.to_ascii_lowercase().as_str() {
            "true" | "yes" | "y" | "tak" | "ja" | "oui" | "sí" | "si" | "sim" | "да" | "так"
            | "ano" => return "✅".into(),
            "false" | "no" | "n" | "nie" | "nein" | "non" | "não" | "нет" | "ні" | "ne" => {
                return "❌".into()
            }
            "null" | "none" | "nil" | "n/a" | "na" | "-" | "" => return "∅".into(),
            _ => {}
        }
    }
    if c.split_whitespace().count() >= 3 {
        prose_inline(c, opts)
    } else {
        // short cells: only proper-name substitutions (Poland → 🇵🇱, October → Oct)
        let lex = crate::lexicon::lexicon(opts.optimize);
        let lower = c.to_lowercase();
        let words: Vec<&str> = lower.split_whitespace().collect();
        if let Some((e, n)) = lex.proper_phrase(&words) {
            if n == words.len() {
                return e.out.to_string();
            }
        }
        if words.len() == 1 && c.chars().next().is_some_and(|ch| ch.is_uppercase()) {
            if let Some(e) = lex.proper(&lower) {
                return e.out.to_string();
            }
        }
        c.to_string()
    }
}

fn detect_sep(lines: &[&str]) -> char {
    let mut best = (',', 0usize);
    for sep in [',', '\t', ';', '|'] {
        let counts: Vec<usize> = lines
            .iter()
            .take(20)
            .map(|l| l.matches(sep).count())
            .collect();
        if counts.is_empty() || counts[0] == 0 {
            continue;
        }
        let consistent = counts.iter().filter(|c| **c == counts[0]).count();
        let score = consistent * 1000 + counts[0];
        if score > best.1 {
            best = (sep, score);
        }
    }
    best.0
}

fn split_csv(line: &str, sep: char) -> Vec<String> {
    let mut cells = Vec::new();
    let mut cur = String::new();
    let mut inq = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '"' {
            if inq && chars.peek() == Some(&'"') {
                cur.push('"');
                chars.next();
            } else {
                inq = !inq;
            }
        } else if c == sep && !inq {
            cells.push(std::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
    }
    cells.push(cur);
    cells
}

pub fn encode_csv(text: &str, opts: Options) -> String {
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.is_empty() {
        return String::new();
    }
    let sep = detect_sep(&lines);
    let header = split_csv(lines[0].trim_matches('|'), sep);
    let mut out = Vec::with_capacity(lines.len());
    out.push(format!(
        "[{}]",
        header
            .iter()
            .map(|h| h.trim())
            .collect::<Vec<_>>()
            .join(",")
    ));
    for l in &lines[1..] {
        let l = l.trim_matches('|');
        if l.chars().all(|c| matches!(c, '-' | ':' | '|' | ' ' | ',')) {
            continue;
        }
        let cells: Vec<String> = split_csv(l, sep)
            .iter()
            .map(|c| csv_cell(c, opts))
            .collect();
        out.push(format!("({})", cells.join(",")));
    }
    out.join("\n")
}

// ----------------------------------------------------------------- log ----

fn level_symbol(w: &str, tokens: bool) -> Option<&'static str> {
    // emoji levels cost 2–3 tokens; the canonical level word costs 1
    Some(match (w.to_ascii_uppercase().as_str(), tokens) {
        ("FATAL" | "CRITICAL" | "CRIT" | "PANIC" | "EMERG" | "SEVERE", false) => "💀",
        ("FATAL" | "CRITICAL" | "CRIT" | "PANIC" | "EMERG" | "SEVERE", true) => "FATAL",
        ("ERROR" | "ERR" | "E", false) => "❌",
        ("ERROR" | "ERR" | "E", true) => "ERROR",
        ("WARN" | "WARNING" | "W", false) => "⚠",
        ("WARN" | "WARNING" | "W", true) => "WARN",
        ("INFO" | "INFORMATION" | "NOTICE" | "I", false) => "ℹ",
        ("INFO" | "INFORMATION" | "NOTICE" | "I", true) => "INFO",
        ("DEBUG" | "DBG" | "D", false) => "🐛",
        ("DEBUG" | "DBG" | "D", true) => "DEBUG",
        ("TRACE" | "VERBOSE" | "V" | "T", false) => "👣",
        ("TRACE" | "VERBOSE" | "V" | "T", true) => "TRACE",
        _ => return None,
    })
}

pub fn encode_log(text: &str, opts: Options) -> String {
    let re_ts = regex::Regex::new(r"^\s*\[?(?P<date>\d{4}[-/]\d{2}[-/]\d{2})?[T ]?(?P<time>\d{2}:\d{2}:\d{2})?(?:[.,]\d{1,9})?(?:Z|[+-]\d{2}:?\d{2})?\]?\s*").unwrap();
    let re_level = regex::Regex::new(r"^\s*[\[(]?(?P<lvl>FATAL|CRITICAL|CRIT|PANIC|SEVERE|ERROR|ERR|WARN|WARNING|INFO|NOTICE|DEBUG|DBG|TRACE|VERBOSE)[\])]?:?\s*[-:|]?\s*").unwrap();
    let re_bracket_level =
        regex::Regex::new(r"\[(FATAL|CRITICAL|ERROR|WARN|WARNING|INFO|DEBUG|TRACE)\]").unwrap();
    let mut out: Vec<String> = Vec::new();
    let mut last_date: Option<String> = None;
    let mut last_msg: Option<String> = None;
    let mut repeat = 0usize;
    let flush_repeat = |out: &mut Vec<String>, repeat: &mut usize| {
        if *repeat > 0 {
            if let Some(last) = out.last_mut() {
                last.push_str(&format!(" ×{}", *repeat + 1));
            }
            *repeat = 0;
        }
    };
    for raw in text.lines() {
        let line = raw.trim_end();
        if line.trim().is_empty() {
            continue;
        }
        let mut rest = line;
        let mut prefix = String::new();
        if let Some(c) = re_ts.captures(rest) {
            if c.name("date").is_some() || c.name("time").is_some() {
                let date = c.name("date").map(|m| m.as_str().to_string());
                let time = c.name("time").map(|m| m.as_str().to_string());
                if let Some(d) = &date {
                    if last_date.as_deref() != Some(d.as_str()) {
                        prefix.push_str(&d[5..]); // drop the year, keep MM-DD
                        last_date = Some(d.clone());
                        if time.is_some() {
                            prefix.push(' ');
                        }
                    }
                }
                if let Some(t) = &time {
                    prefix.push_str(t);
                }
                rest = &rest[c.get(0).unwrap().end()..];
            }
        }
        let mut level = None;
        if let Some(c) = re_level.captures(rest) {
            level = level_symbol(c.name("lvl").unwrap().as_str(), opts.tokens());
            rest = &rest[c.get(0).unwrap().end()..];
        } else if let Some(c) = re_bracket_level.captures(rest) {
            level = level_symbol(c.get(1).unwrap().as_str(), opts.tokens());
            let r = c.get(0).unwrap().range();
            let s = format!("{}{}", &rest[..r.start], &rest[r.end..]);
            let msg = compress_log_msg(s.trim(), opts);
            let lvl = level
                .map(|l| {
                    if prefix.is_empty() {
                        l.to_string()
                    } else {
                        format!("{}{l}", sp(opts))
                    }
                })
                .unwrap_or_default();
            let entry = format!(
                "{prefix}{lvl}{}",
                if prefix.is_empty() && level.is_none() {
                    msg.clone()
                } else {
                    format!(" {msg}")
                }
            );
            push_log(
                &mut out,
                entry,
                &msg,
                &mut last_msg,
                &mut repeat,
                &flush_repeat,
            );
            continue;
        }
        let mut msg = compress_log_msg(rest.trim(), opts);
        let mut entry = prefix.clone();
        if let Some(l) = level {
            if !entry.is_empty() {
                entry.push_str(sp(opts));
            }
            entry.push_str(l);
            if let Some(stripped) = msg.strip_prefix(l) {
                msg = stripped.trim_start().to_string();
            }
        }
        if !msg.is_empty() {
            if !entry.is_empty() {
                entry.push(' ');
            }
            entry.push_str(&msg);
        }
        push_log(
            &mut out,
            entry,
            &msg,
            &mut last_msg,
            &mut repeat,
            &flush_repeat,
        );
    }
    flush_repeat(&mut out, &mut repeat);
    out.join("\n")
}

fn push_log(
    out: &mut Vec<String>,
    entry: String,
    msg: &str,
    last_msg: &mut Option<String>,
    repeat: &mut usize,
    flush: &dyn Fn(&mut Vec<String>, &mut usize),
) {
    if last_msg.as_deref() == Some(msg) && !msg.is_empty() {
        *repeat += 1;
        return;
    }
    flush(out, repeat);
    *last_msg = Some(msg.to_string());
    out.push(entry);
}

fn compress_log_msg(msg: &str, opts: Options) -> String {
    // keep key=value pairs verbatim, compress the free-text remainder
    let mut words: Vec<String> = Vec::new();
    let mut text_run: Vec<&str> = Vec::new();
    let flush = |run: &mut Vec<&str>, words: &mut Vec<String>| {
        if !run.is_empty() {
            let joined = run.join(" ");
            let enc = prose::encode(&joined, opts);
            words.push(if enc.is_empty() { joined } else { enc });
            run.clear();
        }
    };
    for w in msg.split_whitespace() {
        let structured = w.contains('=')
            || w.contains("://")
            || w.starts_with('/')
            || w.contains('.') && w.chars().filter(|c| c.is_ascii_digit()).count() >= 2
            || w.starts_with('[')
            || w.starts_with('{')
            || w.chars().all(|c| c.is_ascii_hexdigit() || c == '-') && w.len() >= 8
            || w.starts_with('0') && w.len() > 1;
        if structured {
            flush(&mut text_run, &mut words);
            words.push(w.to_string());
        } else {
            text_run.push(w);
        }
    }
    flush(&mut text_run, &mut words);
    words.join(" ")
}

// ------------------------------------------------------------------ kv ----

pub fn encode_kv(text: &str, opts: Options) -> String {
    let mut out = Vec::new();
    for raw in text.lines() {
        let line = raw.trim_end();
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') || t.starts_with("//") || t.starts_with(';') {
            let c = t.trim_start_matches(['#', '/', ';']).trim();
            if !c.is_empty() {
                out.push(format!("#{}", prose_inline(c, opts)));
            }
            continue;
        }
        let indent = (line.len() - t.len()) / 2;
        if let Some(idx) = t.find(['=', ':']) {
            let (k, v) = (t[..idx].trim(), t[idx + 1..].trim());
            let v = v.trim_matches(['"', '\'']);
            let vv = match v.to_ascii_lowercase().as_str() {
                "true" | "yes" | "on" if !opts.tokens() => "✅".to_string(),
                "false" | "no" | "off" if !opts.tokens() => "❌".to_string(),
                "null" | "none" | "~" | "" if !opts.tokens() => "∅".to_string(),
                _ if v.split_whitespace().count() >= 4 => format!("\"{}\"", prose_inline(v, opts)),
                _ => v.to_string(),
            };
            out.push(format!(
                "{}{}:{}",
                " ".repeat(indent),
                k.replace(' ', "_"),
                vv
            ));
        } else {
            out.push(format!("{}{}", " ".repeat(indent), prose_inline(t, opts)));
        }
    }
    out.join("\n")
}

// ------------------------------------------------------------ dialogue ----

pub fn encode_dialogue(text: &str, opts: Options) -> String {
    let mut labels: HashMap<String, String> = HashMap::new();
    let mut used: Vec<String> = Vec::new();
    let mut out = Vec::new();
    for raw in text.lines() {
        let t = raw.trim();
        if t.is_empty() {
            continue;
        }
        let Some(idx) = t.find(':') else {
            out.push(prose_inline(t, opts));
            continue;
        };
        let label = t[..idx]
            .trim()
            .trim_matches(['[', ']', '<', '>', '*'])
            .trim();
        let msg = t[idx + 1..].trim();
        if label.is_empty()
            || label.split_whitespace().count() > 3
            || !label.chars().next().is_some_and(|c| c.is_alphabetic())
        {
            out.push(prose_inline(t, opts));
            continue;
        }
        let short = labels
            .entry(label.to_lowercase())
            .or_insert_with(|| {
                let canon = match label.to_lowercase().as_str() {
                    "user" | "human" | "customer" | "client" | "użytkownik" | "klient" => {
                        "U".to_string()
                    }
                    "assistant" | "ai" | "bot" | "chatgpt" | "copilot" | "claude" | "gpt"
                    | "model" | "asystent" => "A".to_string(),
                    "system" | "sys" => "S".to_string(),
                    "agent" | "support" | "operator" => "Ag".to_string(),
                    _ => {
                        let initials: String = label
                            .split_whitespace()
                            .filter_map(|w| w.chars().next())
                            .map(|c| c.to_uppercase().next().unwrap())
                            .collect();
                        initials
                    }
                };
                let mut cand = canon.clone();
                let mut n = 2;
                while used.contains(&cand) {
                    cand = format!("{canon}{n}");
                    n += 1;
                }
                used.push(cand.clone());
                cand
            })
            .clone();
        out.push(format!("{short}:{}", prose_inline(msg, opts)));
    }
    out.join("\n")
}

// ---------------------------------------------------------------- html ----

pub fn encode_html(text: &str, opts: Options) -> String {
    let mut t = text.to_string();
    for tag in ["script", "style", "head", "noscript", "svg"] {
        let re_drop = regex::Regex::new(&format!(r"(?is)<{tag}\b.*?</{tag}\s*>")).unwrap();
        t = re_drop.replace_all(&t, " ").into_owned();
    }
    let re_block = regex::Regex::new(r"(?i)</?(p|div|br|li|ul|ol|h[1-6]|tr|table|section|article|header|footer|blockquote|pre|hr|title|dd|dt)\b[^>]*>").unwrap();
    t = re_block.replace_all(&t, "\n").into_owned();
    let re_li = regex::Regex::new(r"(?i)<li\b[^>]*>").unwrap();
    t = re_li.replace_all(&t, "\n• ").into_owned();
    let re_td = regex::Regex::new(r"(?i)</t[dh]\s*>").unwrap();
    t = re_td.replace_all(&t, " | ").into_owned();
    let re_tag = regex::Regex::new(r"(?s)<[^>]+>").unwrap();
    t = re_tag.replace_all(&t, "").into_owned();
    for (ent, rep) in [
        ("&nbsp;", " "),
        ("&amp;", "&"),
        ("&lt;", "<"),
        ("&gt;", ">"),
        ("&quot;", "\""),
        ("&#39;", "'"),
        ("&apos;", "'"),
        ("&hellip;", "…"),
        ("&mdash;", "—"),
        ("&ndash;", "–"),
        ("&copy;", "©"),
        ("&laquo;", "\""),
        ("&raquo;", "\""),
    ] {
        t = t.replace(ent, rep);
    }
    let re_num = regex::Regex::new(r"&#(\d+);").unwrap();
    t = re_num
        .replace_all(&t, |c: &regex::Captures| {
            c[1].parse::<u32>()
                .ok()
                .and_then(char::from_u32)
                .map(|ch| ch.to_string())
                .unwrap_or_default()
        })
        .into_owned();
    let lines: Vec<String> = t
        .lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|l| !l.is_empty())
        .collect();
    let joined = lines.join("\n\n");
    prose::encode(&joined, opts)
}

#[cfg(test)]
mod tests {
    use super::*;

    const O: Options = Options {
        level: 2,
        lang: "en",
        optimize: crate::lexicon::Optimize::Chars,
    };

    #[test]
    fn code_keeps_strings_symbolises_ops() {
        let out = encode_code(
            "if (a != b && c >= 2) {\n    return \"x != y\"; // the value is not equal\n}\n",
            O,
        );
        assert!(out.contains("a≠b∧c≥2"), "{out}");
        assert!(out.contains("\"x != y\""), "{out}");
        assert!(out.contains("//"), "{out}");
    }

    #[test]
    fn csv_and_log() {
        let out = encode_csv(
            "name,ok,note\nAnn,true,\"The build is very slow today\"\nBob,false,\n",
            O,
        );
        assert!(out.starts_with("[name,ok,note]"), "{out}");
        assert!(out.contains("(Ann,✅,"), "{out}");
        assert!(out.contains("(Bob,❌,∅)"), "{out}");
        let out = encode_log("2024-01-02 10:00:00 INFO server started on port 8080\n2024-01-02 10:00:01 ERROR connection failed: timeout\n2024-01-02 10:00:01 ERROR connection failed: timeout\n", O);
        assert!(out.contains("ℹ"), "{out}");
        assert!(out.contains("❌"), "{out}");
        assert!(out.contains("×2"), "{out}");
        assert!(!out.contains("2024-01-02"), "{out}");
    }

    #[test]
    fn markdown_and_dialogue() {
        let out = encode_markdown("# The Title\n\nSome **bold** text with a [link](http://x.y).\n\n- item one\n- item two\n", O);
        assert!(out.starts_with('#'), "{out}");
        assert!(out.contains("link→http://x.y"), "{out}");
        assert!(out.contains("•item"), "{out}");
        let out = encode_dialogue(
            "User: Is the build working?\nAssistant: Yes, it is working now.\n",
            O,
        );
        assert!(out.starts_with("U:"), "{out}");
        assert!(out.contains("\nA:"), "{out}");
    }
}
