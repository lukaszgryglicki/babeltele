//! JSON → dense BabelTele-style notation.
//!
//! `{"name":"Ann","active":true,"tags":["a","b"],"note":"The build is very slow today"}`
//! becomes `{name:Ann,active:✅,tags:[a,b],note:"build slow today"}`.
//! Arrays of flat objects sharing the same keys collapse into a header + tuples:
//! `[id,name]:(1,Ann)(2,Bob)`.

use serde_json::Value;

use crate::prose::{self, Options};

pub fn encode(text: &str, opts: Options) -> String {
    let t = text.trim_start_matches('\u{feff}').trim();
    // JSON Lines: one object per line
    if !(t.starts_with('{') && t.ends_with('}') && serde_json::from_str::<Value>(t).is_ok())
        && !(t.starts_with('[') && serde_json::from_str::<Value>(t).is_ok())
    {
        let mut out = Vec::new();
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            match serde_json::from_str::<Value>(l) {
                Ok(v) => out.push(value(&v, opts, 0)),
                Err(_) => out.push(prose::encode(l, opts)),
            }
        }
        return out.join("\n");
    }
    match serde_json::from_str::<Value>(t) {
        Ok(v) => value(&v, opts, 0),
        Err(_) => prose::encode(text, opts),
    }
}

fn needs_quotes(s: &str) -> bool {
    s.is_empty()
        || s.chars().any(|c| {
            matches!(
                c,
                ',' | ':' | '{' | '}' | '[' | ']' | '(' | ')' | '"' | '\n' | '\t'
            )
        })
        || s.chars().next().is_some_and(|c| c.is_whitespace())
        || s.chars().last().is_some_and(|c| c.is_whitespace())
        || matches!(s, "true" | "false" | "null" | "∅" | "✅" | "❌")
        || s.parse::<f64>().is_ok()
}

fn string(s: &str, opts: Options) -> String {
    let words = s.split_whitespace().count();
    if words >= 4 {
        let enc = prose::encode(s, opts);
        let enc = if enc.is_empty() { s.to_string() } else { enc };
        format!("\"{}\"", enc.replace('"', "'"))
    } else if needs_quotes(s) {
        format!("\"{}\"", s.replace('"', "'"))
    } else {
        s.to_string()
    }
}

fn key(k: &str) -> String {
    if k.chars()
        .any(|c| matches!(c, ',' | ':' | '{' | '}' | '[' | ']' | '(' | ')' | '"'))
        || k.contains(' ')
        || k.is_empty()
    {
        format!("\"{}\"", k.replace('"', "'"))
    } else {
        k.to_string()
    }
}

fn flat_object(v: &Value) -> Option<Vec<&str>> {
    let obj = v.as_object()?;
    if obj.values().any(|x| x.is_object() || x.is_array()) {
        return None;
    }
    Some(obj.keys().map(String::as_str).collect())
}

pub fn value(v: &Value, opts: Options, depth: usize) -> String {
    match v {
        // "∅"/"❌" cost 2 tokens, "null"/"false" cost 1; "✅" ties "true" only under o200k
        Value::Null => if opts.tokens() { "null" } else { "∅" }.into(),
        Value::Bool(true) => if opts.tokens() { "true" } else { "✅" }.into(),
        Value::Bool(false) => if opts.tokens() { "false" } else { "❌" }.into(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => string(s, opts),
        Value::Array(items) => {
            if items.is_empty() {
                return "[]".into();
            }
            // table form for arrays of flat objects with identical keys
            if items.len() >= 2 {
                if let Some(keys) = flat_object(&items[0]) {
                    if items
                        .iter()
                        .all(|it| flat_object(it).is_some_and(|k| k == keys))
                    {
                        let header: Vec<String> = keys.iter().map(|k| key(k)).collect();
                        let rows: Vec<String> = items
                            .iter()
                            .map(|it| {
                                let o = it.as_object().unwrap();
                                let cells: Vec<String> = keys
                                    .iter()
                                    .map(|k| value(&o[*k], opts, depth + 1))
                                    .collect();
                                format!("({})", cells.join(","))
                            })
                            .collect();
                        return format!("[{}]:{}", header.join(","), rows.join(""));
                    }
                }
            }
            let parts: Vec<String> = items.iter().map(|x| value(x, opts, depth + 1)).collect();
            format!("[{}]", parts.join(","))
        }
        Value::Object(map) => {
            if map.is_empty() {
                return "{}".into();
            }
            let parts: Vec<String> = map
                .iter()
                .map(|(k, x)| format!("{}:{}", key(k), value(x, opts, depth + 1)))
                .collect();
            if depth == 0 && parts.iter().map(|p| p.chars().count() + 1).sum::<usize>() > 100 {
                // top-level: one key per line for readability of big documents
                return parts.join("\n");
            }
            format!("{{{}}}", parts.join(","))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dense_json() {
        let o = Options {
            level: 2,
            lang: "en",
            optimize: crate::lexicon::Optimize::Chars,
        };
        let out = encode(
            r#"{"name":"Ann","active":true,"n":null,"tags":["a","b"],"note":"The build is very slow today"}"#,
            o,
        );
        assert_eq!(
            out,
            "{name:Ann,active:✅,n:∅,tags:[a,b],note:\"構築遅今日\"}"
        );
        let out = encode(r#"[{"id":1,"name":"Ann"},{"id":2,"name":"Bob"}]"#, o);
        assert_eq!(out, "[id,name]:(1,Ann)(2,Bob)");
    }
}
