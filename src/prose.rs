//! Natural-language prose → BabelTele encoder.
//!
//! Pipeline: normalise → tokenise → per-paragraph encode (phrase & word
//! lexicon lookups, function-word rules, number/unit fusion, conditionals) →
//! render with spacing rules that glue operators and emoji to their operands.

use crate::lexicon::{self, Kind, Lexicon, Optimize};
use crate::numwords;

#[derive(Clone, Copy)]
pub struct Options {
    /// 1 = symbols/abbreviations only, 2 = also drop fillers (default), 3 = aggressive.
    pub level: u8,
    pub lang: &'static str,
    /// Minimise BPE tokens (default) or characters.
    pub optimize: Optimize,
}

impl Options {
    #[cfg(test)]
    pub fn new(level: u8, lang: &'static str) -> Options {
        Options {
            level,
            lang,
            optimize: Optimize::Tokens,
        }
    }

    pub fn tokens(&self) -> bool {
        self.optimize == Optimize::Tokens
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Word(String),
    Num(String),
    Sym(char),
    Protected(String),
    Space,
    Nl(usize),
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Normalise typography so downstream rules see one canonical form.
fn normalize(text: &str) -> String {
    let mut s = text.replace("\r\n", "\n").replace('\r', "\n");
    for (a, b) in [
        ("\u{2019}", "'"),
        ("\u{2018}", "'"),
        ("\u{201C}", "\""),
        ("\u{201D}", "\""),
        ("\u{201E}", "\""),
        ("\u{00AB}", "\""),
        ("\u{00BB}", "\""),
        ("...", "…"),
        ("->", "→"),
        ("=>", "⇒"),
        ("<=", "≤"),
        (">=", "≥"),
        ("!=", "≠"),
        (" -- ", " — "),
        ("\u{00A0}", " "),
    ] {
        s = s.replace(a, b);
    }
    s
}

fn tokenize(text: &str) -> Vec<Tok> {
    let chars: Vec<char> = text.chars().collect();
    let mut toks = Vec::with_capacity(chars.len() / 4);
    let mut i = 0;
    let n = chars.len();
    while i < n {
        let c = chars[i];
        if c == '\n' {
            let mut k = 0;
            while i < n && (chars[i] == '\n' || chars[i] == ' ' || chars[i] == '\t') {
                if chars[i] == '\n' {
                    k += 1;
                }
                i += 1;
            }
            toks.push(Tok::Nl(k));
            continue;
        }
        if c == ' ' || c == '\t' {
            while i < n && (chars[i] == ' ' || chars[i] == '\t') {
                i += 1;
            }
            toks.push(Tok::Space);
            continue;
        }
        // Protected spans: `code`, URLs, e-mails, paths, @handles, #tags, $VARS, --flags
        let run_end = {
            let mut j = i;
            while j < n && !chars[j].is_whitespace() {
                j += 1;
            }
            j
        };
        let run: String = chars[i..run_end].iter().collect();
        let protected = c == '`'
            || run.contains("://")
            || run.starts_with("www.")
            || (run.contains('@') && run.contains('.') && !run.starts_with('@'))
            || (c == '@' && run.len() > 1 && chars[i + 1].is_alphanumeric())
            || (c == '#' && run.len() > 1 && chars[i + 1].is_alphanumeric())
            || (c == '$' && run.len() > 1 && (chars[i + 1].is_alphabetic() || chars[i + 1] == '{'))
            || (c == '-' && run.starts_with("--") && run.len() > 2 && chars[i + 2].is_alphabetic())
            || (c == '-' && run.len() == 2 && chars[i + 1].is_alphabetic())
            // host:port, key=value, snake_case identifiers, dotted names (db.prod.host)
            || (c.is_alphanumeric() && is_identifier_like(&run))
            || (c == '/' && run.len() > 2 && chars[i + 1].is_alphabetic() && run[1..].contains('/'))
            || ((run.starts_with("./") || run.starts_with("../") || run.starts_with("~/")) && run.len() > 2)
            || (run.contains('/') && run.contains('.') && !run.starts_with('.') && run.chars().filter(|c| *c == '/').count() >= 1 && run.chars().any(|c| c.is_alphabetic()) && !run.chars().next().unwrap().is_uppercase() && run.chars().filter(|c| c.is_alphanumeric()).count() > 3 && !run.contains('%'));
        if protected {
            if c == '`' {
                // backtick span may contain spaces
                let mut j = i + 1;
                while j < n && chars[j] != '`' && chars[j] != '\n' {
                    j += 1;
                }
                if j < n && chars[j] == '`' {
                    toks.push(Tok::Protected(chars[i..=j].iter().collect()));
                    i = j + 1;
                    continue;
                }
            } else {
                // strip trailing sentence punctuation from the protected run
                let mut end = run_end;
                while end > i + 1
                    && matches!(
                        chars[end - 1],
                        '.' | ',' | ';' | ':' | ')' | ']' | '"' | '\'' | '!' | '?' | '…'
                    )
                {
                    end -= 1;
                }
                toks.push(Tok::Protected(chars[i..end].iter().collect()));
                i = end;
                continue;
            }
        }
        if c.is_ascii_digit()
            || ((c == '-'
                || c == '+'
                || c == '$'
                || c == '€'
                || c == '£'
                || c == '¥'
                || c == '~'
                || c == '≈')
                && i + 1 < n
                && chars[i + 1].is_ascii_digit()
                && (i == 0 || !is_word_char(chars[i - 1])))
        {
            let start = i;
            i += 1;
            while i < n {
                let d = chars[i];
                if d.is_ascii_digit() {
                    i += 1;
                } else if matches!(d, '.' | ',' | ':' | '/' | '-' | '–' | '_')
                    && i + 1 < n
                    && chars[i + 1].is_ascii_digit()
                {
                    i += 1;
                } else if matches!(d, '%' | '°' | '‰' | '+') {
                    i += 1;
                    break;
                } else if d.is_alphabetic()
                    && !(d == 'x' && i + 1 < n && chars[i + 1].is_ascii_digit())
                {
                    // unit / ordinal suffix glued to the digits (5km, 3rd, 10GB, 2024a)
                    let s = i;
                    while i < n
                        && (chars[i].is_alphanumeric()
                            || chars[i] == '/' && i + 1 < n && chars[i + 1].is_alphabetic())
                    {
                        i += 1;
                    }
                    let suffix: String = chars[s..i].iter().collect();
                    if suffix.chars().count() > 5 {
                        // "2024was" -> split back
                        i = s;
                    }
                    break;
                } else {
                    break;
                }
            }
            toks.push(Tok::Num(chars[start..i].iter().collect()));
            continue;
        }
        if c.is_alphabetic() || c == '_' {
            let start = i;
            i += 1;
            while i < n {
                let d = chars[i];
                if is_word_char(d) {
                    i += 1;
                } else if (d == '\'' || d == '-') && i + 1 < n && chars[i + 1].is_alphanumeric() {
                    i += 1;
                } else if d == '.'
                    && i + 2 < n
                    && chars[i + 1].is_alphabetic()
                    && chars[i + 2] == '.'
                    && (i - start) <= 2
                {
                    // abbreviation like e.g. / i.e. / a.m. / U.S.
                    i += 3;
                    break;
                } else {
                    break;
                }
            }
            toks.push(Tok::Word(chars[start..i].iter().collect()));
            continue;
        }
        toks.push(Tok::Sym(c));
        i += 1;
    }
    toks
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Cls {
    Word,
    Num,
    Prefix,
    Infix,
    Joiner,
    Glue,
    Comma,
    Close,
    Open,
    Newline,
}

#[derive(Clone, Debug)]
struct Item {
    text: String,
    cls: Cls,
    from_lex: bool,
}

fn cls_of(kind: Kind) -> Cls {
    match kind {
        Kind::Prefix => Cls::Prefix,
        Kind::Infix => Cls::Infix,
        Kind::Joiner => Cls::Joiner,
        Kind::Glue => Cls::Glue,
        Kind::Word | Kind::Drop | Kind::Copula => Cls::Word,
    }
}

fn is_capitalized(w: &str) -> bool {
    let mut it = w.chars();
    match it.next() {
        Some(c) if c.is_uppercase() => {
            it.next().is_none_or(|d| !d.is_uppercase()) || w.chars().count() <= 1
        }
        _ => false,
    }
}

fn is_allcaps(w: &str) -> bool {
    let letters: Vec<char> = w.chars().filter(|c| c.is_alphabetic()).collect();
    letters.len() >= 2 && letters.iter().all(|c| c.is_uppercase())
}

/// Technical tokens that must survive verbatim: host:8080, key=value, my_var, a.b.c, v1.2.3
fn is_identifier_like(run: &str) -> bool {
    let core = run.trim_end_matches(|c: char| {
        matches!(
            c,
            '.' | ',' | ';' | ':' | ')' | ']' | '"' | '\'' | '!' | '?'
        )
    });
    if core.len() < 3
        || core
            .chars()
            .any(|c| !(c.is_alphanumeric() || matches!(c, ':' | '=' | '_' | '.' | '-' | '/')))
    {
        return false;
    }
    let has_alpha = core.chars().any(|c| c.is_alphabetic());
    let port = core.split_once(':').is_some_and(|(h, p)| {
        !h.is_empty() && !p.is_empty() && p.chars().all(|c| c.is_ascii_digit())
    });
    let assignment = core.split_once('=').is_some_and(|(k, v)| {
        !k.is_empty()
            && !v.is_empty()
            && k.chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '.')
    });
    let snake = core.contains('_') && has_alpha;
    let dotted =
        core.matches('.').count() >= 2 && has_alpha && core.split('.').all(|seg| !seg.is_empty());
    let file = core.rsplit_once('.').is_some_and(|(stem, ext)| {
        !stem.is_empty()
            && stem
                .chars()
                .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '/'))
            && matches!(
                ext,
                "sh" | "py"
                    | "rs"
                    | "go"
                    | "js"
                    | "ts"
                    | "jsx"
                    | "tsx"
                    | "md"
                    | "txt"
                    | "json"
                    | "yaml"
                    | "yml"
                    | "toml"
                    | "csv"
                    | "tsv"
                    | "html"
                    | "htm"
                    | "css"
                    | "xml"
                    | "sql"
                    | "c"
                    | "h"
                    | "cpp"
                    | "hpp"
                    | "java"
                    | "kt"
                    | "rb"
                    | "php"
                    | "env"
                    | "cfg"
                    | "conf"
                    | "ini"
                    | "log"
                    | "pdf"
                    | "png"
                    | "jpg"
                    | "jpeg"
                    | "gif"
                    | "svg"
                    | "zip"
                    | "tar"
                    | "gz"
                    | "exe"
                    | "dll"
                    | "so"
                    | "lock"
                    | "bak"
                    | "tmp"
                    | "docx"
                    | "xlsx"
                    | "pptx"
                    | "mp3"
                    | "mp4"
                    | "wav"
                    | "iso"
                    | "img"
                    | "bin"
                    | "dat"
                    | "db"
                    | "sqlite"
                    | "com"
                    | "org"
                    | "net"
                    | "io"
                    | "dev"
                    | "pl"
                    | "de"
                    | "uk"
                    | "eu"
            )
    });
    port || assignment || snake || dotted || file
}

fn is_abbrev(w: &str) -> bool {
    let n = w.chars().count();
    n <= 4 && w.chars().any(|c| c.is_uppercase())
        || n <= 3 && w.chars().all(|c| c.is_ascii_lowercase())
}

/// Language-specific small function words.
struct FnWords {
    to: &'static [&'static str],
    from: &'static [&'static str],
    in_: &'static [&'static str],
    on_at: &'static [&'static str],
    of: &'static [&'static str],
    for_: &'static [&'static str],
    by: &'static [&'static str],
    and: &'static [&'static str],
    not: &'static [&'static str],
    no: &'static [&'static str],
    it: &'static [&'static str],
    if_: &'static [&'static str],
    unless: &'static [&'static str],
    then: &'static [&'static str],
    between: &'static [&'static str],
    aux: &'static [&'static str],
    up_to: &'static [&'static str],
}

const EMPTY: &[&str] = &[];

/// Words that mark a clause boundary by themselves, making a preceding comma
/// redundant (used at level 3).
fn l3_connector(lang: &str, w: &str) -> bool {
    let list: &[&str] = match lang {
        "en" => &[
            "which", "that", "who", "whom", "whose", "and", "but", "or", "so", "because", "if",
            "when", "while", "then", "whereas", "although", "though", "since", "unless", "until",
            "where", "as", "yet", "nor",
        ],
        "pl" => &[
            "który",
            "która",
            "które",
            "którzy",
            "których",
            "którego",
            "której",
            "którym",
            "i",
            "oraz",
            "ale",
            "lecz",
            "więc",
            "ponieważ",
            "bo",
            "gdyż",
            "jeśli",
            "jeżeli",
            "gdy",
            "kiedy",
            "lub",
            "albo",
            "a",
            "że",
            "żeby",
            "aby",
            "choć",
            "chociaż",
            "dopóki",
            "natomiast",
            "zaś",
        ],
        "de" => &[
            "dass", "daß", "weil", "wenn", "und", "aber", "oder", "sondern", "obwohl", "während",
            "bis", "damit", "denn", "sowie", "falls", "sobald", "bevor", "nachdem", "doch",
            "jedoch",
        ],
        "fr" => &[
            "qui", "que", "et", "mais", "ou", "donc", "car", "si", "quand", "lorsque", "alors",
            "puis", "tandis", "bien", "parce", "dont", "où", "ni",
        ],
        "es" => &[
            "que", "y", "e", "pero", "o", "u", "porque", "si", "cuando", "mientras", "aunque",
            "sino", "pues", "donde", "ni", "luego", "entonces",
        ],
        "it" => &[
            "che", "e", "ed", "ma", "o", "oppure", "perché", "se", "quando", "mentre", "anche",
            "quindi", "dove", "né", "poi", "però", "cui",
        ],
        "pt" => &[
            "que", "e", "mas", "ou", "porque", "se", "quando", "enquanto", "embora", "pois",
            "onde", "nem", "então", "porém", "contudo",
        ],
        "nl" => &[
            "die", "dat", "en", "maar", "of", "omdat", "als", "wanneer", "terwijl", "hoewel",
            "dus", "want", "zodat", "toen", "waar",
        ],
        "ru" => &[
            "который",
            "которая",
            "которые",
            "которых",
            "которого",
            "которой",
            "что",
            "и",
            "но",
            "а",
            "или",
            "потому",
            "если",
            "когда",
            "пока",
            "хотя",
            "чтобы",
            "поэтому",
            "где",
            "либо",
            "зато",
            "однако",
        ],
        "uk" => &[
            "який",
            "яка",
            "які",
            "яких",
            "якого",
            "якої",
            "що",
            "і",
            "й",
            "та",
            "але",
            "а",
            "або",
            "чи",
            "бо",
            "тому",
            "якщо",
            "коли",
            "поки",
            "хоча",
            "щоб",
            "де",
            "проте",
            "однак",
        ],
        "cs" => &[
            "který", "která", "které", "kteří", "kterých", "kterého", "že", "a", "ale", "nebo",
            "protože", "pokud", "když", "zatímco", "ačkoli", "aby", "tak", "takže", "kde", "však",
            "avšak",
        ],
        _ => &[
            "which", "that", "who", "and", "but", "or", "because", "if", "when", "while",
        ],
    };
    list.contains(&w)
}

fn fn_words(lang: &str) -> FnWords {
    match lang {
        "en" => FnWords {
            to: &["to", "towards", "toward", "into", "onto", "until", "till"],
            from: &["from"],
            in_: &["in", "within", "inside"],
            on_at: &["on", "at", "upon"],
            of: &["of"],
            for_: &["for"],
            by: &["by"],
            and: &["and"],
            not: &["not"],
            no: &["no"],
            it: &["it"],
            if_: &["if", "when", "whenever", "once"],
            unless: &["unless"],
            then: &["then"],
            between: &["between"],
            aux: &[
                "do", "does", "did", "is", "are", "was", "were", "has", "have", "had", "will",
                "would",
            ],
            up_to: &["up to", "at most", "maximum"],
        },
        "pl" => FnWords {
            to: &["do", "ku"],
            from: &["od", "spośród"],
            in_: &["w", "we", "wewnątrz"],
            on_at: &["na", "przy", "u"],
            of: EMPTY,
            for_: &["dla"],
            by: &["o", "przez"],
            and: &["i", "oraz"],
            not: &["nie"],
            no: &["brak"],
            it: &["to"],
            if_: &["jeśli", "jeżeli", "gdy", "kiedy", "jak"],
            unless: &["chyba"],
            then: &["to", "wtedy", "wówczas"],
            between: &["między", "pomiędzy"],
            aux: &[
                "jest", "są", "był", "była", "było", "były", "będzie", "będą",
            ],
            up_to: &["najwyżej", "maksymalnie"],
        },
        "de" => FnWords {
            to: &["zu", "nach", "bis"],
            from: &["von", "aus"],
            in_: &["in", "im", "innerhalb"],
            on_at: &["an", "am", "auf", "bei", "beim"],
            of: EMPTY,
            for_: &["für"],
            by: &["um", "durch"],
            and: &["und"],
            not: &["nicht", "kein", "keine", "keinen", "keinem", "keiner"],
            no: EMPTY,
            it: &["es"],
            if_: &["wenn", "falls", "sobald", "sofern"],
            unless: EMPTY,
            then: &["dann", "so"],
            between: &["zwischen"],
            aux: &[
                "ist", "sind", "war", "waren", "wird", "werden", "hat", "haben",
            ],
            up_to: &["höchstens", "maximal"],
        },
        "fr" => FnWords {
            to: &["à", "vers", "jusqu'à", "jusqu'en"],
            from: &["de", "depuis"],
            in_: &["dans", "en"],
            on_at: &["sur", "chez"],
            of: &["du", "des"],
            for_: &["pour"],
            by: &["par"],
            and: &["et"],
            not: &["pas", "ne", "n'"],
            no: &["aucun", "aucune"],
            it: &["il", "ce", "c'"],
            if_: &["si", "lorsque", "quand"],
            unless: EMPTY,
            then: &["alors"],
            between: &["entre"],
            aux: &[
                "est", "sont", "était", "étaient", "sera", "seront", "a", "ont",
            ],
            up_to: &["maximum"],
        },
        "es" => FnWords {
            to: &["a", "hacia", "hasta"],
            from: &["desde"],
            in_: &["en", "dentro"],
            on_at: &["sobre"],
            of: &["de", "del"],
            for_: &["para"],
            by: &["por"],
            and: &["y", "e"],
            not: &["no"],
            no: &["ningún", "ninguna", "ninguno"],
            it: &["ello", "eso", "esto"],
            if_: &["si", "cuando"],
            unless: EMPTY,
            then: &["entonces"],
            between: &["entre"],
            aux: &[
                "es", "son", "era", "eran", "está", "están", "será", "serán", "ha", "han",
            ],
            up_to: &["máximo"],
        },
        "it" => FnWords {
            to: &["a", "verso", "fino"],
            from: &["da", "dal", "dalla", "dai"],
            in_: &["in", "nel", "nella", "nei", "nelle", "nello"],
            on_at: &["su", "sul", "sulla", "presso"],
            of: &["di", "del", "della", "dei", "delle", "dello"],
            for_: &["per"],
            by: EMPTY,
            and: &["e", "ed"],
            not: &["non"],
            no: &["nessun", "nessuna", "nessuno"],
            it: &["esso", "ciò"],
            if_: &["se", "quando", "qualora"],
            unless: EMPTY,
            then: &["allora"],
            between: &["tra", "fra"],
            aux: &[
                "è", "sono", "era", "erano", "sarà", "saranno", "ha", "hanno",
            ],
            up_to: &["massimo"],
        },
        "pt" => FnWords {
            to: &["a", "para", "até"],
            from: &["desde"],
            in_: &["em", "no", "na", "nos", "nas", "dentro"],
            on_at: &["sobre"],
            of: &["de", "do", "da", "dos", "das"],
            for_: EMPTY,
            by: &["por", "pelo", "pela"],
            and: &["e"],
            not: &["não"],
            no: &["nenhum", "nenhuma"],
            it: &["isso", "isto"],
            if_: &["se", "quando", "caso"],
            unless: EMPTY,
            then: &["então"],
            between: &["entre"],
            aux: &[
                "é", "são", "era", "eram", "está", "estão", "será", "serão", "tem", "têm",
            ],
            up_to: &["máximo"],
        },
        "nl" => FnWords {
            to: &["naar", "tot", "toe"],
            from: &["van", "vanaf", "uit"],
            in_: &["in", "binnen"],
            on_at: &["op", "aan", "bij"],
            of: EMPTY,
            for_: &["voor"],
            by: &["door", "met"],
            and: &["en"],
            not: &["niet", "geen"],
            no: EMPTY,
            it: &["het"],
            if_: &["als", "indien", "wanneer", "zodra"],
            unless: &["tenzij"],
            then: &["dan"],
            between: &["tussen"],
            aux: &[
                "is", "zijn", "was", "waren", "wordt", "worden", "heeft", "hebben", "zal", "zullen",
            ],
            up_to: &["maximaal", "hoogstens"],
        },
        "ru" => FnWords {
            to: &["к", "ко", "до"],
            from: &["от", "из", "с", "со"],
            in_: &["в", "во", "внутри"],
            on_at: &["на", "у", "при"],
            of: EMPTY,
            for_: &["для"],
            by: &["на"],
            and: &["и"],
            not: &["не", "нет"],
            no: &["нет"],
            it: &["это", "оно"],
            if_: &["если", "когда"],
            unless: EMPTY,
            then: &["то", "тогда"],
            between: &["между"],
            aux: &[
                "есть",
                "был",
                "была",
                "было",
                "были",
                "будет",
                "будут",
                "является",
                "являются",
            ],
            up_to: &["максимум"],
        },
        "uk" => FnWords {
            to: &["до", "к"],
            from: &["від", "з", "із", "зі"],
            in_: &["в", "у", "всередині"],
            on_at: &["на", "при"],
            of: EMPTY,
            for_: &["для"],
            by: &["на"],
            and: &["і", "й", "та"],
            not: &["не", "немає"],
            no: &["немає"],
            it: &["це", "воно"],
            if_: &["якщо", "коли"],
            unless: EMPTY,
            then: &["то", "тоді"],
            between: &["між"],
            aux: &["є", "був", "була", "було", "були", "буде", "будуть"],
            up_to: &["максимум"],
        },
        "cs" | "sk" => FnWords {
            to: &["do", "k", "ke", "ku"],
            from: &["od", "z", "ze", "zo"],
            in_: &["v", "ve", "vo", "uvnitř"],
            on_at: &["na", "u", "při", "pri"],
            of: EMPTY,
            for_: &["pro", "pre"],
            by: &["o"],
            and: &["a"],
            not: &["ne", "nie"],
            no: &["žádný", "žádná", "žádné", "žiadny", "žiadna", "žiadne"],
            it: &["to"],
            if_: &[
                "pokud",
                "jestli",
                "jestliže",
                "když",
                "jakmile",
                "ak",
                "keď",
            ],
            unless: EMPTY,
            then: &["pak", "tak", "potom"],
            between: &["mezi", "medzi"],
            aux: &[
                "je", "jsou", "byl", "byla", "bylo", "byly", "bude", "budou", "sú", "bol", "bola",
                "bolo", "boli", "budú",
            ],
            up_to: &["maximálně", "nejvýše", "maximálne", "najviac"],
        },
        _ => FnWords {
            to: EMPTY,
            from: EMPTY,
            in_: EMPTY,
            on_at: EMPTY,
            of: EMPTY,
            for_: EMPTY,
            by: EMPTY,
            and: EMPTY,
            not: EMPTY,
            no: EMPTY,
            it: EMPTY,
            if_: EMPTY,
            unless: EMPTY,
            then: EMPTY,
            between: EMPTY,
            aux: EMPTY,
            up_to: EMPTY,
        },
    }
}

/// (lang, word) pairs whose PROPER-table match is ambiguous and requires capitalisation.
const AMBIGUOUS_PROPER: &[(&str, &str)] = &[
    ("it", "mai"),
    ("ro", "mai"),
    ("es", "marca"),
    ("it", "marca"),
    ("pt", "marca"),
    ("pl", "kina"),
    ("ro", "luni"),
    ("pt", "segunda"),
    ("es", "mayo"),
    ("fr", "mars"),
    ("cs", "září"),
    ("pl", "maja"),
    ("cs", "kanada"),
    ("de", "mal"),
    ("tr", "ocak"),
    ("tr", "ekim"),
    ("sk", "marca"),
    ("en", "sept"),
    ("pl", "czech"),
    ("cs", "kina"),
];

struct Encoder<'a> {
    lex: &'static Lexicon,
    opts: Options,
    fw: FnWords,
    toks: &'a [Tok],
    out: Vec<Item>,
    cond_open: bool,
    sentence_start: bool,
    quote_open: bool,
    paren_depth: i32,
}

impl<'a> Encoder<'a> {
    fn lvl(&self, e: &lexicon::Entry) -> bool {
        e.level <= self.opts.level
    }

    fn push(&mut self, text: impl Into<String>, cls: Cls, from_lex: bool) {
        let text = text.into();
        if text.is_empty() {
            return;
        }
        // never emit the same operator twice in a row ("um etwa" → ~~)
        if matches!(cls, Cls::Prefix | Cls::Infix | Cls::Glue)
            && self
                .out
                .last()
                .is_some_and(|l| l.text == text && l.cls == cls)
        {
            return;
        }
        self.out.push(Item {
            text,
            cls,
            from_lex,
        });
        self.sentence_start = false;
    }

    /// Number of content items since the last sentence boundary (Close/Newline).
    fn clause_items(&self) -> usize {
        self.out
            .iter()
            .rev()
            .take_while(|it| !matches!(it.cls, Cls::Close | Cls::Newline))
            .filter(|it| matches!(it.cls, Cls::Word | Cls::Num | Cls::Glue))
            .count()
    }

    fn push_entry(&mut self, e: lexicon::Entry) {
        match e.kind {
            Kind::Drop => {}
            Kind::Copula => {}
            k => self.push(e.out, cls_of(k), true),
        }
    }

    /// Index of next non-space token at or after `i`.
    fn next_sig(&self, mut i: usize) -> Option<usize> {
        while i < self.toks.len() {
            match &self.toks[i] {
                Tok::Space => i += 1,
                Tok::Nl(1) => i += 1,
                _ => return Some(i),
            }
        }
        None
    }

    /// Lowercase words ahead starting at token `i` (up to `max`), skipping spaces.
    fn words_ahead(&self, i: usize, max: usize) -> (Vec<String>, Vec<usize>) {
        let mut words = Vec::new();
        let mut ends = Vec::new();
        let mut j = i;
        while words.len() < max && j < self.toks.len() {
            match &self.toks[j] {
                Tok::Word(w) => {
                    words.push(w.to_lowercase());
                    ends.push(j + 1);
                    j += 1;
                }
                Tok::Space => j += 1,
                _ => break,
            }
        }
        (words, ends)
    }

    fn next_word_lower(&self, i: usize) -> Option<(String, usize)> {
        let j = self.next_sig(i)?;
        match &self.toks[j] {
            Tok::Word(w) => Some((w.to_lowercase(), j)),
            _ => None,
        }
    }

    /// Does the next significant token look like a quantity (number, ~, >, <, currency, ∅…)?
    fn next_is_quantity(&self, i: usize) -> bool {
        let Some(j) = self.next_sig(i) else {
            return false;
        };
        match &self.toks[j] {
            Tok::Num(_) => true,
            Tok::Sym(c) => matches!(
                c,
                '$' | '€'
                    | '£'
                    | '¥'
                    | '~'
                    | '≈'
                    | '>'
                    | '<'
                    | '≥'
                    | '≤'
                    | '#'
                    | '∅'
                    | '-'
                    | '+'
            ),
            Tok::Word(w) => {
                let lw = w.to_lowercase();
                if numwords::number_word(self.opts.lang, &lw).is_some() {
                    return true;
                }
                let (words, _) = self.words_ahead(j, lexicon::MAX_PHRASE_WORDS);
                let refs: Vec<&str> = words.iter().map(String::as_str).collect();
                let e = self
                    .lex
                    .phrase(self.opts.lang, &refs)
                    .map(|(e, _)| e)
                    .or_else(|| self.lex.word(self.opts.lang, &lw));
                match e {
                    Some(e) => {
                        matches!(
                            e.out,
                            "~" | ">" | "<" | "≥" | "≤" | "∅" | "=" | "≠" | "½" | "¼" | "¾"
                        ) && e.kind != Kind::Copula
                    }
                    None => false,
                }
            }
            _ => false,
        }
    }

    fn next_is_capitalized_or_time(&self, i: usize) -> bool {
        let Some(j) = self.next_sig(i) else {
            return false;
        };
        match &self.toks[j] {
            Tok::Num(_) => true,
            Tok::Word(w) => {
                let lw = w.to_lowercase();
                is_capitalized(w)
                    || is_allcaps(w)
                    || self.lex.proper(&lw).is_some()
                    || matches!(
                        lw.as_str(),
                        "morning"
                            | "evening"
                            | "night"
                            | "noon"
                            | "midnight"
                            | "monday"
                            | "weekend"
                            | "rano"
                            | "wieczorem"
                            | "nocy"
                            | "weekendzie"
                    )
            }
            Tok::Protected(_) => true,
            _ => false,
        }
    }

    fn prev_cls(&self) -> Option<Cls> {
        self.out.last().map(|i| i.cls)
    }

    fn prev_text(&self) -> &str {
        self.out.last().map(|i| i.text.as_str()).unwrap_or("")
    }

    fn pop_last(&mut self) {
        self.out.pop();
    }

    /// Parse a run of number words starting at `i`; returns (value, end index).
    fn parse_number_words(&self, i: usize) -> Option<(f64, usize)> {
        let lang = self.opts.lang;
        let mut total = 0.0;
        let mut current = 0.0;
        let mut j = i;
        let mut count = 0;
        let mut last_end = i;
        let mut only_one = true;
        loop {
            let Some(k) = self.next_sig(j) else { break };
            let Tok::Word(w) = &self.toks[k] else { break };
            let lw = w.to_lowercase();
            // "thirty-five"
            let parts: Vec<&str> = lw.split('-').collect();
            let mut ok = true;
            let mut vals = Vec::new();
            for p in &parts {
                match numwords::number_word(lang, p) {
                    Some(v) => vals.push(v),
                    None => {
                        ok = false;
                        break;
                    }
                }
            }
            if !ok || (count > 0 && matches!(lw.as_str(), "one" | "jeden" | "eins")) {
                break;
            }
            for (v, mul) in vals {
                if mul {
                    if v >= 1000.0 {
                        total += if current == 0.0 { 1.0 } else { current } * v;
                        current = 0.0;
                    } else {
                        current = if current == 0.0 { 1.0 } else { current } * v;
                    }
                } else {
                    current += v;
                }
                if v != 1.0 {
                    only_one = false;
                }
            }
            count += 1;
            last_end = k + 1;
            j = k + 1;
            if count >= 6 {
                break;
            }
        }
        if count == 0 || only_one {
            return None;
        }
        Some((total + current, last_end))
    }

    /// Absorb units / percent / currency words after a number; returns the new end index.
    fn absorb_units(&self, mut num: String, i: usize) -> (String, usize) {
        let mut end = i;
        let mut j = i;
        let mut steps = 0;
        while steps < 2 {
            steps += 1;
            let Some(k) = self.next_sig(j) else { break };
            match &self.toks[k] {
                Tok::Sym('%') => {
                    num.push('%');
                    end = k + 1;
                    break;
                }
                Tok::Sym(c @ ('°' | '‰')) => {
                    num.push(*c);
                    end = k + 1;
                    break;
                }
                Tok::Word(w) => {
                    let lw = w.to_lowercase();
                    if let Some((v, true)) = numwords::number_word(self.opts.lang, &lw) {
                        // "3 million" -> 3M
                        if let Ok(base) = num.replace(',', "").parse::<f64>() {
                            if v >= 1000.0 {
                                num = numwords::format_value(base * v);
                                end = k + 1;
                                j = k + 1;
                                continue;
                            }
                        }
                        break;
                    }
                    if numwords::is_percent_word(&lw) {
                        num.push('%');
                        end = k + 1;
                        break;
                    }
                    if let Some((w2, k2)) = self.next_word_lower(k + 1) {
                        if numwords::is_percent_phrase(&lw, &w2) {
                            num.push('%');
                            end = k2 + 1;
                            break;
                        }
                    }
                    if let Some(u) = numwords::unit_suffix(&lw) {
                        // emoji units (👤) cost 2 tokens where "users" costs 1: token mode keeps the word
                        if self.opts.tokens()
                            && !u.chars().all(|c| c.is_ascii() || matches!(c, '×' | '°'))
                        {
                            break;
                        }
                        if !num
                            .chars()
                            .last()
                            .is_some_and(|c| c.is_alphabetic() || c == '%')
                        {
                            num.push_str(u);
                            end = k + 1;
                        }
                        break;
                    }
                    if let Some((sym, prefix)) = numwords::currency(&lw) {
                        if prefix {
                            num = format!("{sym}{num}");
                        } else {
                            num.push_str(sym);
                        }
                        end = k + 1;
                        break;
                    }
                    break;
                }
                _ => break,
            }
        }
        (num, end)
    }

    fn encode_paragraph(&mut self) {
        let n = self.toks.len();
        let mut i = 0;
        self.sentence_start = true;
        self.cond_open = false;
        while i < n {
            match &self.toks[i] {
                Tok::Space => {
                    i += 1;
                }
                Tok::Nl(_) => {
                    // single newline inside a paragraph: keep if it starts a list item / short line
                    let keep = match self.next_sig(i) {
                        Some(j) => match &self.toks[j] {
                            Tok::Sym(c) => matches!(c, '-' | '*' | '•' | '>' | '#' | '|'),
                            Tok::Num(x) => {
                                x.ends_with('.')
                                    || x.ends_with(')')
                                    || self
                                        .toks
                                        .get(j + 1)
                                        .is_some_and(|t| matches!(t, Tok::Sym('.') | Tok::Sym(')')))
                            }
                            _ => false,
                        },
                        None => false,
                    } || self
                        .prev_cls()
                        .is_some_and(|c| c == Cls::Close && self.prev_text() == ":");
                    if keep {
                        self.out.push(Item {
                            text: "\n".into(),
                            cls: Cls::Newline,
                            from_lex: false,
                        });
                        self.sentence_start = true;
                        self.cond_open = false;
                    }
                    i += 1;
                }
                Tok::Protected(p) => {
                    let p = p.clone();
                    self.push(p, Cls::Word, false);
                    i += 1;
                }
                Tok::Num(x) => {
                    let (num, end) = self.absorb_units(x.clone(), i + 1);
                    // "$ 30" -> "$30"
                    if let Some(prev) = self.out.last() {
                        if prev.cls == Cls::Prefix
                            && matches!(prev.text.as_str(), "$" | "€" | "£" | "¥")
                        {
                            let sym = prev.text.clone();
                            self.pop_last();
                            self.push(format!("{sym}{num}"), Cls::Num, false);
                            i = end;
                            continue;
                        }
                    }
                    self.push(num, Cls::Num, false);
                    i = end;
                }
                Tok::Sym(c) => {
                    let c = *c;
                    i += 1;
                    self.handle_sym(c, i);
                }
                Tok::Word(w) => {
                    let w = w.clone();
                    i = self.handle_word(&w, i);
                }
            }
        }
    }

    fn handle_sym(&mut self, c: char, i: usize) {
        match c {
            '.' | '!' | '?' | '…' => {
                let more = self.next_sig(i).is_some();
                self.cond_open = false;
                if c == '.' {
                    // collapse "x . ." / trailing periods
                    if self.prev_cls() == Some(Cls::Close)
                        && matches!(self.prev_text(), ";" | "." | "?" | "!")
                        || self.out.is_empty()
                    {
                        return;
                    }
                    if self.opts.level >= 2 {
                        if more {
                            self.push(";", Cls::Close, false);
                        }
                    } else {
                        self.push(".", Cls::Close, false);
                    }
                } else if c == '…' {
                    self.push("…", Cls::Close, false);
                } else {
                    // "?!" runs
                    if self.prev_cls() == Some(Cls::Close) && matches!(self.prev_text(), "?" | "!")
                    {
                        return;
                    }
                    self.push(c.to_string(), Cls::Close, false);
                }
                self.sentence_start = true;
            }
            ',' => {
                if self.cond_open && self.paren_depth == 0 {
                    self.cond_open = false;
                    self.push("⇒", Cls::Infix, false);
                    return;
                }
                if self.prev_cls().is_none_or(|k| {
                    matches!(
                        k,
                        Cls::Comma | Cls::Close | Cls::Infix | Cls::Open | Cls::Newline
                    )
                }) {
                    return;
                }
                // level 3: a comma is redundant when the next word already marks the
                // clause boundary ("records, which we" → "records which we"): 1 token each
                if self.opts.level >= 3 && self.next_sig(i).is_some_and(|j| matches!(&self.toks[j], Tok::Word(w) if l3_connector(self.opts.lang, &w.to_lowercase()))) {
                    return;
                }
                self.push(",", Cls::Comma, false);
            }
            ';' | ':' => {
                if self
                    .prev_cls()
                    .is_none_or(|k| matches!(k, Cls::Close | Cls::Comma | Cls::Newline))
                {
                    return;
                }
                self.push(c.to_string(), Cls::Close, false);
                self.sentence_start = c == ';';
                self.cond_open = false;
            }
            '(' | '[' | '{' => {
                self.paren_depth += 1;
                self.push(c.to_string(), Cls::Open, false);
            }
            ')' | ']' | '}' => {
                self.paren_depth -= 1;
                // drop empty parens produced by dropped content
                if self.prev_cls() == Some(Cls::Open) {
                    self.pop_last();
                    return;
                }
                self.push(c.to_string(), Cls::Close, false);
            }
            '"' => {
                self.quote_open = !self.quote_open;
                if self.quote_open {
                    self.push("\"", Cls::Open, false);
                } else {
                    self.push("\"", Cls::Close, false);
                }
            }
            '\'' => {
                if self
                    .prev_cls()
                    .is_none_or(|k| matches!(k, Cls::Open | Cls::Newline))
                {
                    self.push("'", Cls::Open, false);
                } else {
                    self.push("'", Cls::Close, false);
                }
            }
            '-' | '–' | '—' => {
                // standalone dash between clauses → em-dash spaced as joiner
                if self
                    .prev_cls()
                    .is_none_or(|k| matches!(k, Cls::Newline | Cls::Close))
                {
                    self.push("•", Cls::Word, false);
                } else {
                    self.push("—", Cls::Joiner, false);
                }
            }
            '&' | '+' | '=' | '→' | '⇒' | '≤' | '≥' | '≠' | '<' | '>' | '/' | '×' | '÷' | '±'
            | '≈' | '∈' | '∧' | '∨' | '¬' | '@' => {
                let cls = if matches!(c, '¬' | '≈' | '@')
                    && self
                        .prev_cls()
                        .is_none_or(|k| k != Cls::Word && k != Cls::Num)
                {
                    Cls::Prefix
                } else {
                    Cls::Infix
                };
                self.push(c.to_string(), cls, false);
            }
            '~' => self.push("~", Cls::Prefix, false),
            '$' | '€' | '£' | '¥' => self.push(c.to_string(), Cls::Prefix, false),
            '*' | '•' | '·' | '#' | '|' | '§' | '%' | '^' | '\\' => {
                self.push(c.to_string(), Cls::Word, false)
            }
            _ => {
                if !c.is_whitespace() && !c.is_control() {
                    self.push(c.to_string(), Cls::Glue, false);
                }
            }
        }
    }

    /// Returns the next token index to process.
    fn handle_word(&mut self, w: &str, i: usize) -> usize {
        let lang = self.opts.lang;
        // French/Italian elision: l'équipe → équipe, d'environ → environ (the article carries no information)
        if matches!(lang, "fr" | "it") && self.opts.level >= 2 {
            let lower = w.to_lowercase();
            for pre in [
                "qu'", "l'", "d'", "n'", "s'", "j'", "c'", "m'", "t'", "qu’", "l’", "d’", "n’",
                "s’", "j’", "c’", "m’", "t’",
            ] {
                if lower.starts_with(pre) && lower.chars().count() > pre.chars().count() + 1 {
                    let rest: String = w.chars().skip(pre.chars().count()).collect();
                    return self.handle_word(&rest, i);
                }
            }
        }
        let lw = w.to_lowercase();
        let level = self.opts.level;
        let at_start = self.sentence_start;

        // level 3, English: relative pronoun / complementiser before a personal pronoun is
        // omissible ("the bug which we fixed" → "the bug we fixed", "believes that it" → "believes it")
        if level >= 3 && lang == "en" && !at_start && matches!(lw.as_str(), "which" | "that" | "whom" | "who") && self.next_sig(i + 1).is_some_and(|j| matches!(&self.toks[j], Tok::Word(n) if matches!(n.to_lowercase().as_str(), "i" | "we" | "you" | "they" | "he" | "she" | "it"))) {
            return i + 1;
        }

        // ALLCAPS acronyms are kept verbatim (except sentence-long shouting handled by lexicon anyway)
        if is_allcaps(w) && w.chars().count() <= 6 && !self.lex.proper(&lw).is_some_and(|_| true) {
            self.push(w, Cls::Word, false);
            return i + 1;
        }

        let (words, ends) = self.words_ahead(i, lexicon::MAX_PHRASE_WORDS);
        let refs: Vec<&str> = words.iter().map(String::as_str).collect();

        // 1. multi-word proper names
        if let Some((e, n)) = self.lex.proper_phrase(&refs) {
            self.push_entry(e);
            return ends[n - 1];
        }

        // 2. "from X to Y" / "between X and Y" ranges → X–Y
        if (self.fw.from.contains(&lw.as_str()) || self.fw.between.contains(&lw.as_str()))
            && level >= 1
        {
            if let Some((v1, e1)) = self.range_value(i + 1) {
                let is_between = self.fw.between.contains(&lw.as_str());
                if let Some((mid, k)) = self.next_word_lower(e1) {
                    if (is_between && self.fw.and.contains(&mid.as_str()))
                        || (!is_between && self.fw.to.contains(&mid.as_str()))
                    {
                        if let Some((v2, e2)) = self.range_value(k + 1) {
                            let (v2u, e2) = self.absorb_units(v2, e2);
                            self.push(format!("{v1}–{v2u}"), Cls::Num, false);
                            return e2;
                        }
                    }
                }
            }
        }

        // 3. lexicon phrases (longest first)
        if let Some((e, n)) = self.lex.phrase(lang, &refs) {
            if self.lvl(&e) {
                if e.kind == Kind::Copula {
                    self.apply_copula(ends[n - 1]);
                } else if e.out == "~" && e.kind == Kind::Prefix {
                    if self.next_is_quantity(ends[n - 1]) {
                        self.push("~", Cls::Prefix, true);
                    } else {
                        self.push("re", Cls::Word, true);
                    }
                } else {
                    self.push_entry(e);
                }
                return ends[n - 1];
            }
        }

        // 4. contractions (English)
        if lang == "en" {
            if let Some(r) = numwords::contraction(&lw) {
                match r {
                    "" => {}
                    "I" | "you" | "we" | "they" | "he" | "she" | "what" | "who" | "where"
                    | "how" | "let's" => self.push(r, Cls::Word, false),
                    // "¬可" costs 2 tokens, "cannot" 1; "don't" is 2 tokens, "not" 1
                    "¬" | "¬将" | "¬可" | "¬应" | "¬必" if self.opts.tokens() => {
                        let plain = match r {
                            "¬" => "not",
                            "¬将" => "will not",
                            "¬可" => "cannot",
                            "¬应" => "should not",
                            _ => "must not",
                        };
                        self.push(plain, Cls::Word, false);
                    }
                    "h" => {
                        if let Some(last) = self.out.last_mut() {
                            if last.cls == Cls::Num {
                                last.text.push('h');
                            }
                        }
                    }
                    _ => self.push(r, Cls::Prefix, true),
                }
                return i + 1;
            }
        }

        // 5. number words → digits (+ units). Digits after a space cost an extra
        //    token (" 6" = 2 tokens, " six" = 1), so in token mode English numerals
        //    are only converted when they span several words ("two thousand five hundred").
        if let Some((v, end)) = self.parse_number_words(i) {
            let words_used = self.toks[i..end]
                .iter()
                .filter(|t| matches!(t, Tok::Word(_)))
                .count();
            if self.opts.tokens() && lang == "en" && words_used < 3 {
                // keep "twenty percent" / "six hours" verbatim (never "twenty%")
                let mut text = String::new();
                for t in &self.toks[i..end] {
                    match t {
                        Tok::Word(w) => text.push_str(w),
                        Tok::Sym(c) => text.push(*c),
                        Tok::Space => text.push(' '),
                        _ => {}
                    }
                }
                self.push(text.trim().to_string(), Cls::Word, false);
                if let Some((nw, k)) = self.next_word_lower(end) {
                    if numwords::is_percent_word(&nw)
                        || numwords::unit_suffix(&nw).is_some()
                        || numwords::currency(&nw).is_some()
                    {
                        if let Tok::Word(orig) = &self.toks[k] {
                            let orig = orig.clone();
                            self.push(orig, Cls::Word, false);
                        }
                        return k + 1;
                    }
                }
                return end;
            }
            let (num, end) = self.absorb_units(numwords::format_value(v), end);
            self.push(num, Cls::Num, false);
            return end;
        }
        // lone multiplier: "a million users" → 1M👤
        if let Some((v, true)) = numwords::number_word(lang, &lw) {
            if v >= 1e6 && !(self.opts.tokens() && lang == "en") {
                let (num, end) = self.absorb_units(numwords::format_value(v), i + 1);
                self.push(num, Cls::Num, false);
                return end;
            }
        }
        if let Some(o) = numwords::ordinal(lang, &lw) {
            if self.prev_cls() != Some(Cls::Num) {
                self.push(o, Cls::Num, false);
                return i + 1;
            }
        }

        // 6. function-word rules (level ≥ 2 unless they produce a symbol)
        if level >= 2 || self.fw.not.contains(&lw.as_str()) || self.fw.no.contains(&lw.as_str()) {
            if let Some(next) = self.function_word(&lw, i, at_start) {
                return next;
            }
        }

        // 7. proper nouns
        let ambiguous = AMBIGUOUS_PROPER.contains(&(lang, lw.as_str()));
        let allow_proper = if matches!(lang, "en" | "de") || ambiguous {
            is_capitalized(w) || is_allcaps(w)
        } else {
            true
        };
        if allow_proper {
            if let Some(e) = self.lex.proper(&lw) {
                if !(ambiguous
                    && !self.next_is_quantity(i + 1)
                    && !self.prev_cls().is_some_and(|c| c == Cls::Num))
                    || is_capitalized(w)
                {
                    self.push_entry(e);
                    return i + 1;
                }
            }
        }

        // 8. single-word lexicon
        if let Some(e) = self.lex.word(lang, &lw) {
            if self.lvl(&e) {
                match e.kind {
                    Kind::Copula => self.apply_copula(i + 1),
                    Kind::Prefix if e.out == "~" => {
                        if self.next_is_quantity(i + 1) {
                            self.push("~", Cls::Prefix, true);
                        } else if lang == "en" && lw == "about" {
                            self.push("re", Cls::Word, true);
                        } else if lang == "en" {
                            self.push(w, Cls::Word, false);
                        } else {
                            self.push("~", Cls::Prefix, true);
                        }
                    }
                    Kind::Prefix
                        if matches!(e.out, ">" | "<" | "≥" | "≤")
                            && !self.next_is_quantity(i + 1) =>
                    {
                        if matches!(
                            lw.as_str(),
                            "over"
                                | "under"
                                | "above"
                                | "below"
                                | "beyond"
                                | "past"
                                | "ponad"
                                | "poniżej"
                                | "powyżej"
                                | "über"
                                | "unter"
                                | "sous"
                                | "bajo"
                                | "sotto"
                                | "sob"
                                | "onder"
                                | "над"
                                | "под"
                                | "nad"
                                | "pod"
                        ) {
                            // spatial/temporal preposition ("over the weekend"): not a comparison
                            self.push(w, Cls::Word, false);
                        } else {
                            // "higher latency" → >latency
                            self.push(e.out, Cls::Infix, true);
                        }
                    }
                    _ => self.push_entry(e),
                }
                return i + 1;
            }
        }

        // 9. unknown word: level 3 disemvowels long lowercase words; possessives dropped
        let mut word = w.to_string();
        if level >= 3 {
            if lang == "en" && (word.ends_with("'s") || word.ends_with("s'")) {
                word.truncate(word.len() - 2);
            }
            // "cmprssn" costs more tokens than "compression": chars mode only
            if !self.opts.tokens()
                && word.chars().count() >= 8
                && word.chars().all(|c| c.is_lowercase())
            {
                word = disemvowel(&word);
            }
        }
        self.push(word, Cls::Word, false);
        i + 1
    }

    /// A number (digits or words) usable as a range endpoint at token `i`.
    fn range_value(&self, i: usize) -> Option<(String, usize)> {
        let j = self.next_sig(i)?;
        match &self.toks[j] {
            Tok::Num(x) => Some((x.clone(), j + 1)),
            Tok::Word(_) => self
                .parse_number_words(j)
                .map(|(v, e)| (numwords::format_value(v), e)),
            _ => None,
        }
    }

    fn apply_copula(&mut self, after: usize) {
        if self.sentence_start && self.opts.lang != "en" {
            // "Jest jeden problem" / "Есть одна проблема" → existential;
            // "∃" costs 2 tokens, so in token mode it is simply dropped
            if !self.opts.tokens() {
                self.push("∃", Cls::Prefix, true);
            }
        } else if self.next_is_quantity(after) {
            self.push("=", Cls::Infix, true);
        }
    }

    fn function_word(&mut self, lw: &str, i: usize, at_start: bool) -> Option<usize> {
        let fw = &self.fw;
        let lang = self.opts.lang;
        let next = i + 1;
        let s = lw;

        // " ¬" and " ∅" cost 2 tokens where "not"/"no" cost 1: token mode keeps the word
        let tokens = self.opts.tokens();
        if fw.not.contains(&s) {
            // "do not" → ¬ ; "not only" etc. fall through to lexicon phrases already handled
            if let Some(last) = self.out.last() {
                if last.cls == Cls::Word
                    && self.fw.aux.contains(&last.text.to_lowercase().as_str())
                    && (self.opts.level >= 2 || !tokens)
                {
                    self.pop_last();
                }
            }
            if tokens {
                self.push(s, Cls::Word, false);
            } else {
                self.push("¬", Cls::Prefix, true);
            }
            return Some(next);
        }
        if fw.no.contains(&s) {
            if tokens {
                self.push(s, Cls::Word, false);
                return Some(next);
            }
            match self.next_sig(next).map(|j| &self.toks[j]) {
                Some(Tok::Word(_)) | Some(Tok::Num(_)) if !at_start || lang != "en" => {
                    self.push("∅", Cls::Prefix, true);
                }
                Some(Tok::Word(_)) | Some(Tok::Num(_)) if at_start => {
                    // "No errors found" → ∅errors ; "No, ..." handled below
                    self.push("∅", Cls::Prefix, true);
                }
                _ => self.push("❌", Cls::Glue, true),
            }
            return Some(next);
        }
        if self.opts.level < 2 {
            return None;
        }
        if lang == "en" && matches!(s, "will" | "shall") {
            // Chinese future marker: one char, universally read by LLMs (but 2 cl100k tokens)
            if tokens {
                self.push(s, Cls::Word, false);
            } else {
                self.push("将", Cls::Glue, true);
            }
            return Some(next);
        }
        if lang == "en" && matches!(s, "have" | "has" | "had") {
            if let Some((nw, _)) = self.next_word_lower(next) {
                let participle = nw.ends_with("ed")
                    || nw.ends_with("en")
                    || matches!(
                        nw.as_str(),
                        "been"
                            | "done"
                            | "made"
                            | "gone"
                            | "built"
                            | "found"
                            | "sent"
                            | "put"
                            | "set"
                            | "left"
                            | "kept"
                            | "shown"
                            | "known"
                            | "become"
                            | "begun"
                            | "brought"
                            | "bought"
                            | "come"
                            | "cut"
                            | "dealt"
                            | "drawn"
                            | "felt"
                            | "fought"
                            | "got"
                            | "gotten"
                            | "grown"
                            | "heard"
                            | "held"
                            | "hit"
                            | "laid"
                            | "led"
                            | "lost"
                            | "meant"
                            | "met"
                            | "paid"
                            | "read"
                            | "run"
                            | "said"
                            | "sold"
                            | "sat"
                            | "shut"
                            | "slept"
                            | "spent"
                            | "spoken"
                            | "stood"
                            | "stuck"
                            | "taught"
                            | "thought"
                            | "thrown"
                            | "understood"
                            | "won"
                            | "written"
                            | "already"
                            | "just"
                            | "not"
                            | "never"
                            | "also"
                    );
                if participle {
                    return Some(next);
                }
            }
            return None;
        }
        if fw.aux.contains(&s) && lang == "en" && matches!(s, "do" | "does" | "did") {
            // auxiliary "do" before "not" → dropped when ¬ appears; before verb → drop
            if let Some((nw, _)) = self.next_word_lower(next) {
                if nw == "not" || !at_start {
                    return Some(next);
                }
            }
            return None;
        }
        if fw.to.contains(&s) {
            if self.next_is_quantity(next) || self.next_is_capitalized_or_time(next) {
                self.push("→", Cls::Infix, true);
            } else if matches!(
                s,
                "until"
                    | "till"
                    | "dopóki"
                    | "aż do"
                    | "bis"
                    | "jusqu\x27à"
                    | "hasta"
                    | "finché"
                    | "até"
                    | "totdat"
                    | "пока"
                    | "доки"
                    | "dokud"
            ) {
                // temporal "until <clause>" carries meaning: keep it readable
                self.push(s, Cls::Word, false);
            } else if lang != "en"
                && matches!(
                    self.next_sig(next).map(|j| &self.toks[j]),
                    Some(Tok::Word(_))
                )
                && self
                    .prev_cls()
                    .is_some_and(|c| c == Cls::Word || c == Cls::Glue)
                && lw.chars().count() > 1
            {
                // "wzrost do poziomu" → keep direction as → only for clear cases; otherwise drop
            }
            return Some(next);
        }
        if fw.from.contains(&s) {
            if self.next_is_capitalized_or_time(next) && !self.next_is_quantity(next) {
                self.push("←", Cls::Infix, true);
            }
            return Some(next);
        }
        if fw.in_.contains(&s) {
            if self.next_is_capitalized_or_time(next) {
                self.push("@", Cls::Infix, true);
            } else if matches!(
                self.next_sig(next).map(|j| &self.toks[j]),
                Some(Tok::Word(_))
            ) && !self
                .prev_cls()
                .is_none_or(|c| matches!(c, Cls::Close | Cls::Newline | Cls::Infix | Cls::Prefix))
            {
                // " ∈" is 2 tokens, " in" is 1
                if tokens {
                    self.push(s, Cls::Word, false);
                } else {
                    self.push("∈", Cls::Infix, true);
                }
            }
            return Some(next);
        }
        if fw.on_at.contains(&s) {
            if self
                .prev_cls()
                .is_some_and(|c| !matches!(c, Cls::Close | Cls::Newline | Cls::Infix | Cls::Prefix))
                && matches!(
                    self.next_sig(next).map(|j| &self.toks[j]),
                    Some(Tok::Word(_)) | Some(Tok::Num(_)) | Some(Tok::Protected(_))
                )
            {
                self.push("@", Cls::Infix, true);
            }
            return Some(next);
        }
        if fw.of.contains(&s) {
            return Some(next);
        }
        if fw.for_.contains(&s) {
            if lang == "en"
                && !self.next_is_quantity(next)
                && !at_start
                && self
                    .prev_cls()
                    .is_some_and(|c| matches!(c, Cls::Word | Cls::Glue | Cls::Num))
            {
                // a spaced digit costs 2 tokens; "for" costs 1
                if tokens {
                    self.push(s, Cls::Word, false);
                } else {
                    self.push("4", Cls::Infix, true);
                }
            }
            return Some(next);
        }
        if fw.by.contains(&s) {
            if self.next_is_quantity(next) {
                return Some(next);
            }
            if lang == "en" {
                if let Some((nw, _)) = self.next_word_lower(next) {
                    if self.lex.proper(&nw).is_some()
                        || matches!(
                            nw.as_str(),
                            "tomorrow"
                                | "tonight"
                                | "noon"
                                | "midnight"
                                | "eod"
                                | "friday"
                                | "monday"
                                | "next"
                                | "end"
                                | "then"
                        )
                    {
                        self.push("≤", Cls::Infix, true);
                        return Some(next);
                    }
                }
                self.push("by", Cls::Word, false);
                return Some(next);
            }
            if matches!(lang, "pl" | "cs" | "sk") && s == "o" {
                // "o 30%" handled above; "o systemie" → re
                self.push("re", Cls::Word, true);
                return Some(next);
            }
            return Some(next);
        }
        if fw.and.contains(&s) {
            if self.prev_cls() == Some(Cls::Comma) {
                return Some(next);
            }
            if self
                .prev_cls()
                .is_none_or(|c| matches!(c, Cls::Close | Cls::Newline | Cls::Open))
            {
                self.push("&", Cls::Word, true);
            } else {
                self.push("&", Cls::Infix, true);
            }
            return Some(next);
        }
        if fw.it.contains(&s) {
            let drop = if let Some((nw, _)) = self.next_word_lower(next) {
                self.fw.aux.contains(&nw.as_str())
                    || self
                        .lex
                        .word(lang, &nw)
                        .is_some_and(|e| e.kind == Kind::Copula)
                    || (self.cond_open
                        && self.prev_cls() == Some(Cls::Prefix)
                        && self.prev_text() == "?")
            } else {
                false
            };
            if !drop {
                if lang == "en" || tokens {
                    self.push(if lang == "en" { "it" } else { s }, Cls::Word, false);
                } else if at_start || lang == "ru" || lang == "uk" {
                    // pl/cs "to" as demonstrative → 此 ; ru "это" → 此
                    self.push("此", Cls::Glue, true);
                }
                // mid-sentence pl/cs "to" is usually the copula ("X to Y") → dropped
            }
            return Some(next);
        }
        if fw.if_.contains(&s) {
            if self.prev_cls() == Some(Cls::Comma) && self.clause_items() >= 2 {
                // "let me know, if you have questions" → trailing condition ("⇐" is 2 tokens: keep "if")
                self.pop_last();
                if tokens {
                    self.push(s, Cls::Word, false);
                } else {
                    self.push("⇐", Cls::Infix, true);
                }
            } else if at_start
                || self
                    .prev_cls()
                    .is_none_or(|c| matches!(c, Cls::Close | Cls::Newline | Cls::Open | Cls::Comma))
            {
                if self.prev_cls() == Some(Cls::Comma) {
                    self.pop_last();
                    self.push(";", Cls::Close, false);
                }
                self.push("?", Cls::Prefix, true);
                self.cond_open = true;
            } else if tokens {
                self.push(s, Cls::Word, false);
            } else {
                self.push("⇐", Cls::Infix, true);
            }
            return Some(next);
        }
        if fw.unless.contains(&s) {
            if tokens {
                // "?¬" / "⇐¬" cost 3 tokens; "unless" costs 1
                if self.prev_cls() == Some(Cls::Comma) {
                    self.pop_last();
                }
                self.push(s, Cls::Word, false);
                return Some(next);
            }
            if at_start
                || self
                    .prev_cls()
                    .is_none_or(|c| matches!(c, Cls::Close | Cls::Newline | Cls::Open | Cls::Comma))
            {
                if self.prev_cls() == Some(Cls::Comma) {
                    self.pop_last();
                    self.push(";", Cls::Close, false);
                }
                self.push("?¬", Cls::Prefix, true);
                self.cond_open = true;
            } else {
                self.push("⇐¬", Cls::Infix, true);
            }
            return Some(next);
        }
        if fw.then.contains(&s) {
            if self.cond_open {
                self.cond_open = false;
                if self.prev_cls() == Some(Cls::Comma) {
                    self.pop_last();
                }
                self.push("⇒", Cls::Infix, true);
                return Some(next);
            }
            // the comma of "if X, then Y" already produced ⇒
            if self.prev_cls() == Some(Cls::Infix) && self.prev_text() == "⇒" {
                return Some(next);
            }
            if lang == "en" {
                self.push("→", Cls::Infix, true);
                return Some(next);
            }
            return None;
        }
        if fw.up_to.contains(&s) && self.next_is_quantity(next) {
            self.push("≤", Cls::Prefix, true);
            return Some(next);
        }
        None
    }
}

fn disemvowel(w: &str) -> String {
    let mut out = String::with_capacity(w.len());
    for (i, c) in w.chars().enumerate() {
        if i == 0
            || !matches!(
                c,
                'a' | 'e'
                    | 'i'
                    | 'o'
                    | 'u'
                    | 'y'
                    | 'ą'
                    | 'ę'
                    | 'ó'
                    | 'á'
                    | 'é'
                    | 'í'
                    | 'ú'
                    | 'ä'
                    | 'ö'
                    | 'ü'
                    | 'è'
                    | 'ê'
                    | 'à'
                    | 'â'
                    | 'ô'
                    | 'û'
                    | 'î'
                    | 'ë'
                    | 'ï'
                    | 'ù'
                    | 'ß'
            )
        {
            out.push(c);
        }
    }
    if out.chars().count() < 3 {
        w.to_string()
    } else {
        out
    }
}

/// Spacing decision between two rendered items.
fn needs_space(a: &Item, b: &Item) -> bool {
    use Cls::*;
    match (a.cls, b.cls) {
        (Newline, _) | (_, Newline) => false,
        (_, Comma) | (Comma, _) => false,
        (_, Close) => false,
        (Open, _) => false,
        (Prefix, _) => false,
        (_, Infix) | (Infix, _) => false,
        (_, Glue) | (Glue, _) => false,
        // alphabetic joiners ("but", "oder") need spaces; symbolic/CJK ones (∧ ∨ 但) do not
        (Joiner, _) => a.text.chars().any(|c| c.is_ascii_alphabetic()),
        (_, Joiner) => b.text.chars().any(|c| c.is_ascii_alphabetic()),
        (Close, _) => true,
        (_, Open) => true,
        (Num, Word) => {
            // 3M👤, 30%YoY, 5GB glue; "6h vs 2" does not (number already carries a unit)
            let a_has_unit = a.text.chars().last().is_some_and(|c| c.is_alphabetic());
            !((b.from_lex && !b.text.chars().all(|c| c.is_ascii_alphabetic()))
                || (is_abbrev(&b.text) && !a_has_unit))
        }
        (Word, Word) => {
            !(b.from_lex
                && a.text.chars().last().is_some_and(|c| c.is_ascii_digit())
                && a.text.chars().count() <= 4)
        }
        (Word, Num) | (Num, Num) | (Word, Prefix) | (Num, Prefix) => true,
    }
}

/// Letters that BPE tokenizers merge with a preceding space (" word" is one
/// token; "@word" splits the word): everything alphabetic except CJK ideographs.
fn starts_with_spaceable_letter(s: &str) -> bool {
    s.chars()
        .next()
        .is_some_and(|c| c.is_alphabetic() && !is_cjk(c))
}

fn is_cjk(c: char) -> bool {
    matches!(c as u32, 0x2E80..=0x9FFF | 0xAC00..=0xD7AF | 0xF900..=0xFAFF | 0xFF00..=0xFFEF | 0x20000..=0x3134F)
}

fn render(items: &[Item], tokens: bool) -> String {
    let mut s = String::new();
    let mut prev: Option<&Item> = None;
    for it in items {
        if let Some(p) = prev {
            // In token mode a space before a word is free (it is absorbed into the
            // word's token) while gluing a symbol to a word splits the word: always space.
            let glue_hurts = tokens
                && starts_with_spaceable_letter(&it.text)
                && !matches!(p.cls, Cls::Open | Cls::Newline)
                && !(p.cls == Cls::Prefix && p.text == "?¬");
            // " →" costs the same as "→" (and " ≤"/" ⇒"/" 🔧" even less in cl100k): space
            // operators and emoji off words; CJK glue stays glued (cheaper that way)
            let spaceable_glue = it.cls == Cls::Glue
                && !it.text.chars().next().is_some_and(is_cjk)
                && p.cls != Cls::Glue;
            let free_space = tokens
                && (matches!(it.cls, Cls::Infix | Cls::Prefix) || spaceable_glue)
                && matches!(p.cls, Cls::Word | Cls::Num | Cls::Glue)
                && !it.text.starts_with('¬');
            if needs_space(p, it) || glue_hurts || free_space {
                s.push(' ');
            }
        }
        s.push_str(&it.text);
        prev = Some(it);
    }
    // tidy: dangling operators at the end
    let trimmed = s.trim_end_matches(|c: char| {
        c.is_whitespace() || matches!(c, '&' | '→' | '⇒' | '⇐' | '∈' | '@' | '=' | '—')
    });
    trimmed.trim().to_string()
}

/// Encode one block of prose (may contain several paragraphs).
pub fn encode(text: &str, opts: Options) -> String {
    let lex = lexicon::lexicon(opts.optimize);
    let norm = normalize(text);
    let toks = tokenize(&norm);
    // split into paragraphs on blank lines
    let mut paragraphs: Vec<&[Tok]> = Vec::new();
    let mut start = 0;
    for (i, t) in toks.iter().enumerate() {
        if let Tok::Nl(k) = t {
            if *k >= 2 {
                if i > start {
                    paragraphs.push(&toks[start..i]);
                }
                start = i + 1;
            }
        }
    }
    if start < toks.len() {
        paragraphs.push(&toks[start..]);
    }
    let mut out_paragraphs = Vec::with_capacity(paragraphs.len());
    for p in paragraphs {
        let mut enc = Encoder {
            lex,
            opts,
            fw: fn_words(opts.lang),
            toks: p,
            out: Vec::new(),
            cond_open: false,
            sentence_start: true,
            quote_open: false,
            paren_depth: 0,
        };
        enc.encode_paragraph();
        let r = render(&enc.out, opts.tokens());
        if !r.is_empty() {
            out_paragraphs.push(r);
        }
    }
    out_paragraphs.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(level: u8, lang: &'static str) -> Options {
        Options {
            level,
            lang,
            optimize: Optimize::Chars,
        }
    }

    /// Chars mode: the maximal symbolic rendering.
    fn en(s: &str) -> String {
        encode(s, chars(2, "en"))
    }

    /// Token mode (the default).
    fn ent(s: &str) -> String {
        encode(s, Options::new(2, "en"))
    }

    #[test]
    fn paper_micro_example() {
        let out = en("Q3 revenue is projected to rise approximately 30% YoY; if it lands, the Berlin team ships the mobile app in October.");
        assert!(out.contains("Q3rev"), "{out}");
        assert!(out.contains("~30%YoY"), "{out}");
        assert!(out.contains("?lands⇒"), "{out}");
        assert!(out.contains("@Oct"), "{out}");
        assert!(out.chars().count() < 60, "{out}");
    }

    #[test]
    fn numbers_and_units() {
        assert_eq!(en("thirty percent"), "30%");
        assert_eq!(en("3 million users"), "3M👤");
        assert_eq!(en("five hours"), "5h");
        assert_eq!(en("from 5 to 10 minutes"), "5–10min");
        assert_eq!(en("It costs 20 dollars."), "it=$20");
    }

    #[test]
    fn negation_and_conditionals() {
        // "build" → 構築 (cross-lingual), "does not work" → ❌
        assert_eq!(en("The build does not work."), "構築❌");
        let out = en("If the tests fail, then we stop.");
        assert!(out.starts_with('?'), "{out}");
        assert!(out.contains('⇒'), "{out}");
        assert!(!out.contains("⇒→"), "then must not fire twice: {out}");
        let out = en("The build fails unless the cache is warm.");
        assert!(out.contains("⇐¬"), "{out}");
        let out = en("There are no errors in the log.");
        assert!(out.starts_with('∄'), "{out}");
    }

    #[test]
    fn joiner_spacing() {
        let out = en("The new version is better than the old one, but it is not perfect yet.");
        assert!(!out.contains("但 "), "no space after CJK joiner: {out}");
        assert!(out.contains("¬perfect"), "{out}");
    }

    #[test]
    fn polish() {
        let out = encode("Przychody w trzecim kwartale wzrosną o około trzydzieści procent, ponieważ zespół w Berlinie wydał aplikację w październiku.", chars(2, "pl"));
        assert!(out.contains("~30%"), "{out}");
        assert!(out.contains("∵"), "{out}");
        assert!(out.contains("Oct"), "{out}");
    }

    #[test]
    fn protected_spans_survive() {
        let out = en(
            "See https://example.com/a/b and email me at bob@example.com or run `cargo build` now.",
        );
        assert!(out.contains("https://example.com/a/b"), "{out}");
        assert!(out.contains("bob@example.com"), "{out}");
        assert!(out.contains("`cargo build`"), "{out}");
    }

    #[test]
    fn level_one_keeps_articles() {
        let l1 = encode("The team is very happy.", chars(1, "en"));
        assert!(l1.contains("The") || l1.contains("the"), "{l1}");
        let l2 = en("The team is very happy.");
        assert!(!l2.to_lowercase().contains("the "), "{l2}");
    }

    #[test]
    fn token_mode_keeps_cheap_words() {
        // 1-token English words are not traded for 2–4-token symbols/ideograms
        let out = ent("The team cannot deploy the build in Germany because the tests fail.");
        assert!(out.contains("team"), "{out}");
        assert!(out.contains("Germany"), "{out}");
        assert!(out.contains("because"), "{out}");
        assert!(!out.contains('🇩'), "{out}");
        // cheap symbols still apply, fillers are still dropped
        let out = ent("Q3 revenue is projected to rise approximately 30% YoY; if it lands, the Berlin team ships the mobile app in October.");
        assert!(out.contains("~30%"), "{out}");
        assert!(out.contains("⇒"), "{out}");
        assert!(out.contains("Oct"), "{out}");
        assert!(!out.to_lowercase().contains("the "), "{out}");
        // negation keeps the 1-token word ("does not work" → ❌ is a 3→2 token phrase and stays)
        assert_eq!(ent("The build does not compile."), "build not compile");
        assert_eq!(ent("The build does not work."), "build ❌");
        assert!(
            ent("There are no errors.").starts_with("∄ err"),
            "{}",
            ent("There are no errors.")
        );
        // small English numerals stay words (" 6" costs 2 tokens), long ones are converted
        assert_eq!(ent("six hours"), "six hours");
        assert_eq!(ent("two thousand five hundred users"), "2500 users");
        // no disemvoweling at level 3
        let l3 = encode("the compression subsystem", Options::new(3, "en"));
        assert!(l3.contains("compression"), "{l3}");
    }

    #[test]
    fn token_mode_spaces_symbols_from_words() {
        // "@nightly" splits the word into 3 tokens; "@ nightly" is 2
        let out = ent("The job runs at midnight in Berlin.");
        assert!(
            out.contains("@ midnight") || out.contains("@ Berlin"),
            "{out}"
        );
        assert!(!out.contains("@m") && !out.contains("@B"), "{out}");
        // digits stay glued to their operator (a space before digits costs a token)
        assert!(ent("about 30% of users").contains("~30%"));
    }
}
