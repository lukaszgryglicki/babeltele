//! Multilingual concept lexicon: maps words/phrases of many human languages to
//! BabelTele tokens (symbols, emoji, CJK ideograms, abbreviations).
//!
//! All data lives in `lexdata*.rs`; this module builds fast lookup tables once
//! (`OnceLock`) and exposes word / phrase / proper-noun lookups.

use std::collections::HashMap;
use std::sync::OnceLock;

/// What the encoder minimises.
///
/// `Tokens` (default) only applies a substitution when it costs no more BPE
/// tokens than the source text under both `o200k_base` and `cl100k_base`
/// (see `tokcost.rs`), so the output is cheaper for an LLM to read.
/// `Chars` applies every substitution that is shorter in characters — the
/// maximal symbolic "BabelTele look", but CJK/emoji often cost 2–4 tokens.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Optimize {
    #[default]
    Tokens,
    Chars,
}

impl std::str::FromStr for Optimize {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "tokens" | "token" | "tok" | "t" => Ok(Optimize::Tokens),
            "chars" | "char" | "characters" | "c" => Ok(Optimize::Chars),
            _ => Err(format!(
                "unknown optimisation target '{s}' (expected: tokens, chars)"
            )),
        }
    }
}

/// How a replacement token behaves when rendered next to its neighbours.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// Delete the source word entirely.
    Drop,
    /// Operator that attaches to the following token (`¬x`, `∀x`, `~30%`).
    Prefix,
    /// Operator glued on both sides (`a→b`, `x@Oct`, `a&b`).
    Infix,
    /// Connective with spaces on both sides (` ∧ `, ` 但 `).
    Joiner,
    /// Latin abbreviation, spaced like an ordinary word (`rev`, `vs`, `e.g.`).
    Word,
    /// Emoji / CJK / numeric token: never needs spaces around it.
    Glue,
    /// Copula (is/are/jest/ist…): dropped, except before a number where it becomes `=`.
    Copula,
}

pub use Kind::{
    Copula as C, Drop as D, Glue as G, Infix as I, Joiner as J, Prefix as P, Word as W,
};

#[derive(Clone, Copy, Debug)]
pub struct Entry {
    pub out: &'static str,
    pub kind: Kind,
    /// Minimum compression level (1..=3) at which this substitution is applied.
    pub level: u8,
}

/// One concept row: (min level, kind, output, "lang:w1,w2 w3;lang2:w4").
pub type Row = (u8, Kind, &'static str, &'static str);

type WordMap = HashMap<String, Entry>;
/// first word -> [(remaining words, entry)] sorted longest-first.
type PhraseMap = HashMap<String, Vec<(Vec<String>, Entry)>>;

pub struct Lexicon {
    words: HashMap<&'static str, WordMap>,
    phrases: HashMap<&'static str, PhraseMap>,
    proper: WordMap,
    proper_phrases: PhraseMap,
}

pub const MAX_PHRASE_WORDS: usize = 6;

/// All raw concept tables (universal + generated per-language ones).
pub fn tables() -> [&'static [Row]; 4] {
    [
        crate::lexdata::CONCEPTS,
        crate::lexdata_en::CONTENT,
        crate::lexdata_pl::CONTENT,
        crate::lexdata_xx::CONTENT,
    ]
}

/// Token cost (o200k_base, cl100k_base) of a string in the form it is rendered
/// (`" word"` for spaced items, bare for glued ones), from the generated table.
pub fn token_cost(key: &str) -> Option<(u8, u8)> {
    crate::tokcost::COSTS
        .binary_search_by(|(k, _, _)| k.cmp(&key))
        .ok()
        .map(|i| {
            let (_, o, c) = crate::tokcost::COSTS[i];
            (o, c)
        })
}

/// The rendered form of an output token: spaced like a word, or glued to its neighbours.
pub fn out_key(out: &str, kind: Kind) -> String {
    let spaced = match kind {
        Kind::Word | Kind::Prefix => true,
        Kind::Joiner => out.chars().any(|c| c.is_ascii_alphabetic()),
        _ => false,
    };
    if spaced {
        format!(" {out}")
    } else {
        out.to_string()
    }
}

/// Is replacing `src` (a lowercase word or phrase in running text) by `out` a
/// token saving? Requires no more tokens under `cl100k_base` and, under
/// `o200k_base`, strictly fewer — a tie is only accepted when the output is
/// ASCII or a non-letter symbol that is a single token in both vocabularies
/// (→ ≤ ~ …), because the cost of CJK/emoji varies between other tokenizers.
/// Unknown strings are judged conservatively (ASCII output that is not longer).
pub fn substitution_ok(src: &str, out: &str, kind: Kind) -> bool {
    if kind == Kind::Drop || out.is_empty() {
        return true;
    }
    match (
        token_cost(&format!(" {src}")),
        token_cost(&out_key(out, kind)),
    ) {
        (Some((so, sc)), Some((oo, oc))) => {
            let tie_ok =
                out.is_ascii() || (oo == 1 && oc == 1 && !out.chars().any(char::is_alphabetic));
            oc <= sc && (oo < so || (oo == so && tie_ok))
        }
        _ => out.is_ascii() && out.len() <= src.len(),
    }
}

impl Lexicon {
    fn build(opt: Optimize) -> Lexicon {
        let mut lex = Lexicon {
            words: HashMap::new(),
            phrases: HashMap::new(),
            proper: HashMap::new(),
            proper_phrases: HashMap::new(),
        };
        for table in tables() {
            for &(level, kind, out, langs) in table {
                lex.add_row(opt, level, kind, out, langs);
            }
        }
        for &(out, names) in crate::lexdata::PROPER {
            let entry = Entry {
                out,
                kind: infer_kind(out),
                level: 1,
            };
            for name in names.split(',') {
                let name = name.trim().to_lowercase();
                if opt == Optimize::Tokens && !substitution_ok(&name, out, entry.kind) {
                    continue;
                }
                let parts: Vec<String> = name.split_whitespace().map(String::from).collect();
                match parts.len() {
                    0 => {}
                    1 => {
                        lex.proper.entry(name).or_insert(entry);
                    }
                    _ => lex
                        .proper_phrases
                        .entry(parts[0].clone())
                        .or_default()
                        .push((parts[1..].to_vec(), entry)),
                }
            }
        }
        for list in lex
            .phrases
            .values_mut()
            .flat_map(|m| m.values_mut())
            .chain(lex.proper_phrases.values_mut())
        {
            list.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        }
        lex
    }

    fn add_row(
        &mut self,
        opt: Optimize,
        level: u8,
        kind: Kind,
        out: &'static str,
        langs: &'static str,
    ) {
        for group in langs.split(';') {
            let group = group.trim();
            if group.is_empty() {
                continue;
            }
            let (lang, list) = match group.split_once(':') {
                Some(x) => x,
                None => ("*", group),
            };
            let lang: &'static str = match lang.trim() {
                "en" => "en",
                "pl" => "pl",
                "de" => "de",
                "fr" => "fr",
                "es" => "es",
                "it" => "it",
                "pt" => "pt",
                "nl" => "nl",
                "ru" => "ru",
                "uk" => "uk",
                "cs" => "cs",
                "sk" => "sk",
                "sv" => "sv",
                "da" => "da",
                "no" => "no",
                "fi" => "fi",
                "tr" => "tr",
                "hu" => "hu",
                "ro" => "ro",
                _ => "*",
            };
            let entry = Entry { out, kind, level };
            for word in list.split(',') {
                let word = word.trim().to_lowercase();
                if word.is_empty() {
                    continue;
                }
                if opt == Optimize::Tokens && !substitution_ok(&word, out, kind) {
                    continue;
                }
                let parts: Vec<String> = word.split_whitespace().map(String::from).collect();
                if parts.len() == 1 {
                    // A replacement must never be longer than the word it replaces
                    // (drops are always fine; the output is empty).
                    if kind != Kind::Drop && out.chars().count() > word.chars().count() {
                        continue;
                    }
                    // Never let a later table override a lower-level, more specific mapping.
                    self.words
                        .entry(lang)
                        .or_default()
                        .entry(word)
                        .and_modify(|e| {
                            if level < e.level {
                                *e = entry;
                            }
                        })
                        .or_insert(entry);
                } else {
                    let first = parts[0].clone();
                    let rest: Vec<String> = parts[1..].to_vec();
                    self.phrases
                        .entry(lang)
                        .or_default()
                        .entry(first)
                        .or_default()
                        .push((rest, entry));
                }
            }
        }
    }

    /// Look up a single lowercase word in `lang` (falling back to the universal table).
    pub fn word(&self, lang: &str, lower: &str) -> Option<Entry> {
        self.words
            .get(lang)
            .and_then(|m| m.get(lower))
            .or_else(|| self.words.get("*").and_then(|m| m.get(lower)))
            .copied()
    }

    /// Try to match a phrase starting at `words[0]` (all lowercase). Returns
    /// the entry and the number of words consumed (>= 2).
    pub fn phrase(&self, lang: &str, words: &[&str]) -> Option<(Entry, usize)> {
        if words.len() < 2 {
            return None;
        }
        for l in [lang, "*"] {
            let Some(list) = self.phrases.get(l).and_then(|m| m.get(words[0])) else {
                continue;
            };
            'cand: for (rest, entry) in list {
                if rest.len() + 1 > words.len() {
                    continue;
                }
                for (i, w) in rest.iter().enumerate() {
                    if words[i + 1] != w {
                        continue 'cand;
                    }
                }
                return Some((*entry, rest.len() + 1));
            }
        }
        None
    }

    /// Proper-noun table (months, weekdays, countries, well-known acronyms).
    pub fn proper(&self, lower: &str) -> Option<Entry> {
        self.proper.get(lower).copied()
    }

    /// Multi-word proper names ("united states", "unia europejska"); returns the
    /// entry and the number of words consumed (>= 2).
    pub fn proper_phrase(&self, words: &[&str]) -> Option<(Entry, usize)> {
        if words.len() < 2 {
            return None;
        }
        let list = self.proper_phrases.get(words[0])?;
        'cand: for (rest, entry) in list {
            if rest.len() + 1 > words.len() {
                continue;
            }
            for (i, w) in rest.iter().enumerate() {
                if words[i + 1] != w {
                    continue 'cand;
                }
            }
            return Some((*entry, rest.len() + 1));
        }
        None
    }
}

/// Guess the rendering kind of a proper-table output.
fn infer_kind(out: &str) -> Kind {
    if out.is_empty() {
        return Kind::Drop;
    }
    if out
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '/' || c == '\'')
    {
        Kind::Word
    } else {
        Kind::Glue
    }
}

pub fn lexicon(opt: Optimize) -> &'static Lexicon {
    static TOK: OnceLock<Lexicon> = OnceLock::new();
    static CHR: OnceLock<Lexicon> = OnceLock::new();
    match opt {
        Optimize::Tokens => TOK.get_or_init(|| Lexicon::build(Optimize::Tokens)),
        Optimize::Chars => CHR.get_or_init(|| Lexicon::build(Optimize::Chars)),
    }
}

/// Every string whose token cost the generated table must know, in rendered
/// form: `" source"` for each lexicon word/phrase and the `out_key` of each
/// output. Used by `--dump-lexicon` (feeds `scripts/gen_tokcost.py`) and tests.
pub fn cost_keys() -> Vec<String> {
    let mut keys = Vec::new();
    for table in tables() {
        for &(_, kind, out, langs) in table {
            if kind != Kind::Drop && !out.is_empty() {
                keys.push(out_key(out, kind));
            }
            for group in langs.split(';') {
                let list = group.split_once(':').map_or(group, |x| x.1);
                for word in list.split(',') {
                    let word = word.trim().to_lowercase();
                    if !word.is_empty() {
                        keys.push(format!(" {word}"));
                    }
                }
            }
        }
    }
    for &(out, names) in crate::lexdata::PROPER {
        keys.push(out_key(out, infer_kind(out)));
        for name in names.split(',') {
            let name = name.trim().to_lowercase();
            if !name.is_empty() {
                keys.push(format!(" {name}"));
            }
        }
    }
    keys.sort();
    keys.dedup();
    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_lookups() {
        let lex = lexicon(Optimize::Chars);
        let e = lex.word("en", "and").unwrap();
        assert_eq!(e.out, "&");
        assert_eq!(e.kind, Kind::Infix);
        let e = lex.word("pl", "ponieważ").unwrap();
        assert_eq!(e.out, "∵");
        assert!(lex.word("en", "zzzzqqq").is_none());
    }

    #[test]
    fn phrase_longest_first() {
        let lex = lexicon(Optimize::Chars);
        let (e, n) = lex
            .phrase("en", &["as", "soon", "as", "possible", "ok"])
            .unwrap();
        assert_eq!(e.out, "ASAP");
        assert_eq!(n, 4);
        let (e, n) = lex.phrase("en", &["as", "well", "as", "x"]).unwrap();
        assert_eq!(e.out, "∧");
        assert_eq!(n, 3);
    }

    #[test]
    fn proper_table() {
        let lex = lexicon(Optimize::Chars);
        assert_eq!(lex.proper("october").unwrap().out, "Oct");
        assert_eq!(lex.proper("październik").unwrap().out, "Oct");
        assert_eq!(lex.proper("germany").unwrap().out, "🇩🇪");
        let (e, n) = lex
            .proper_phrase(&["united", "states", "of", "america", "x"])
            .unwrap();
        assert_eq!((e.out, n), ("🇺🇸", 4));
        assert!(lex.proper("may").is_none());
    }

    #[test]
    fn no_replacement_longer_than_source() {
        // Every single-word mapping must not be longer (in chars) than the word.
        for opt in [Optimize::Chars, Optimize::Tokens] {
            let lex = lexicon(opt);
            let mut bad = Vec::new();
            for (lang, map) in &lex.words {
                for (w, e) in map {
                    if e.out.chars().count() > w.chars().count() {
                        bad.push(format!("{lang}:{w}->{}", e.out));
                    }
                }
            }
            bad.sort();
            assert!(
                bad.is_empty(),
                "{} longer than source: {}",
                bad.len(),
                bad.join(" ")
            );
        }
    }

    #[test]
    fn token_cost_table_is_complete_and_sorted() {
        // Forgetting to regenerate tokcost.rs after editing the lexicon fails here.
        let costs = crate::tokcost::COSTS;
        assert!(
            costs.windows(2).all(|w| w[0].0 < w[1].0),
            "COSTS must be strictly sorted"
        );
        let missing: Vec<String> = cost_keys()
            .into_iter()
            .filter(|k| token_cost(k).is_none())
            .collect();
        assert!(
            missing.is_empty(),
            "{} keys missing from tokcost.rs (run scripts/gen_tokcost.py): {:?}",
            missing.len(),
            &missing[..missing.len().min(20)]
        );
    }

    #[test]
    fn token_mode_never_costs_more_tokens() {
        let lex = lexicon(Optimize::Tokens);
        // single words: symbol substitutions that are more expensive than the English word are gone
        assert!(
            lex.word("en", "because").is_none(),
            "because→∵ costs 2 tokens vs 1"
        );
        assert!(
            lex.word("en", "team").is_none(),
            "team→👥 costs 2 tokens vs 1"
        );
        assert!(
            lex.proper("germany").is_none(),
            "Germany→🇩🇪 costs 4 tokens vs 1"
        );
        // cheap ones stay
        assert_eq!(lex.proper("october").unwrap().out, "Oct");
        assert_eq!(lex.word("en", "and").unwrap().out, "&");
        assert_eq!(
            lex.phrase("en", &["as", "soon", "as", "possible"])
                .unwrap()
                .0
                .out,
            "ASAP"
        );
        // multi-word phrases → 2-token symbol are still a win or a tie
        assert_eq!(
            lex.phrase("en", &["as", "a", "result", "of"])
                .unwrap()
                .0
                .out,
            "∵"
        );
        // o200k knows Polish well (" ponieważ" is 1 token) so that word stays too,
        // while multi-token phrases are still symbolised
        assert!(lex.word("pl", "ponieważ").is_none());
        assert!(
            lex.word("pl", "gdyż").is_none(),
            "gdyż→∵ is a 2→2 tie with a 2-token symbol"
        );
        assert_eq!(lex.phrase("pl", &["z", "powodu"]).unwrap().0.out, "∵");
        // and every surviving mapping really is not more expensive
        let mut bad = Vec::new();
        for (lang, map) in &lex.words {
            for (w, e) in map {
                if !substitution_ok(w, e.out, e.kind) {
                    bad.push(format!("{lang}:{w}->{}", e.out));
                }
            }
        }
        assert!(bad.is_empty(), "{bad:?}");
    }
}
