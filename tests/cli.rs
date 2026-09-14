use std::io::Write;
use std::process::{Command, Stdio};

fn run(args: &[&str], stdin: &str) -> (String, String, i32) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_babeltele"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn babeltele");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

const PAPER: &str = "Q3 revenue is projected to rise ~30% YoY; if it lands, the Berlin team ships the mobile app in October.\n";

#[test]
fn paper_example_from_stdin() {
    // chars mode: the maximal symbolic rendering from the paper's micro-example
    let (out, err, code) = run(&["-s", "-O", "chars"], PAPER);
    assert_eq!(code, 0, "{err}");
    assert!(out.starts_with("Q3rev"), "{out}");
    assert!(out.contains("~30%YoY"), "{out}");
    assert!(out.contains("⇒"), "{out}");
    assert!(out.contains("@Oct"), "{out}");
    assert!(out.ends_with('\n'));
    assert!(
        err.contains("kind=prose") && err.contains("lang=en") && err.contains("optimize=chars"),
        "{err}"
    );
}

#[test]
fn default_is_token_mode() {
    // token mode (default): same structure, but no CJK/emoji that would cost more tokens
    let (out, err, code) = run(&["-s"], PAPER);
    assert_eq!(code, 0, "{err}");
    assert!(err.contains("optimize=tokens"), "{err}");
    assert!(out.contains("~30%"), "{out}");
    assert!(out.contains("⇒"), "{out}");
    assert!(out.contains("Oct"), "{out}");
    assert!(out.contains("team"), "1-token word kept: {out}");
    assert!(!out.contains("👥") && !out.contains("📱"), "{out}");
    let (chars, _, _) = run(&["-O", "chars"], PAPER);
    assert!(
        chars.chars().count() < out.chars().count(),
        "chars mode is shorter in characters: {chars} vs {out}"
    );
    let (_, err, code) = run(&["-O", "bytes"], PAPER);
    assert_eq!(code, 2, "{err}");
}

#[test]
fn autodetects_kinds() {
    let cases: &[(&str, &str)] = &[
        (r#"{"a": 1, "ok": true, "n": null}"#, "kind=json"),
        (
            "fn main() {\n    let x = 1;\n    println!(\"{}\", x);\n}\n",
            "kind=code",
        ),
        (
            "# Title\n\nSome text here.\n\n- one\n- two\n",
            "kind=markdown",
        ),
        (
            "2024-01-02 10:00:00 INFO started\n2024-01-02 10:00:01 ERROR failed: x\n",
            "kind=log",
        ),
        ("name,age,city\nbob,3,nyc\nann,5,la\n", "kind=csv"),
        (
            "User: hi there, how are you?\nAssistant: hello, how can I help you today?\n",
            "kind=dialogue",
        ),
        ("host = localhost\nport = 5432\nuser = admin\n", "kind=kv"),
        (
            "<html><body><p>Hello <b>world</b></p></body></html>",
            "kind=html",
        ),
    ];
    for (input, expected) in cases {
        let (_, err, code) = run(&["-s"], input);
        assert_eq!(code, 0);
        assert!(err.contains(expected), "input {input:?}: {err}");
    }
}

#[test]
fn json_is_dense_and_structured() {
    let input = r#"{"name":"Ann","active":true,"tags":["a","b"],"rows":[{"id":1,"ok":false},{"id":2,"ok":true}]}"#;
    let (out, _, _) = run(&["-O", "chars"], input);
    assert_eq!(
        out.trim(),
        "{name:Ann,active:✅,tags:[a,b],rows:[id,ok]:(1,❌)(2,✅)}"
    );
    let (out, _, _) = run(&[], input);
    assert_eq!(
        out.trim(),
        "{name:Ann,active:true,tags:[a,b],rows:[id,ok]:(1,false)(2,true)}"
    );
}

#[test]
fn levels_differ() {
    let text =
        "The quick brown fox jumps over the lazy dog because it is very important for the team.";
    let (l1, _, _) = run(&["-l", "1"], text);
    let (l2, _, _) = run(&["-l", "2"], text);
    let (l3, _, _) = run(&["-l", "3"], text);
    assert!(l1.chars().count() >= l2.chars().count(), "{l1} vs {l2}");
    assert!(l2.chars().count() >= l3.chars().count(), "{l2} vs {l3}");
    assert!(l1.contains("The "), "level 1 keeps articles: {l1}");
    assert!(!l2.contains("the "), "level 2 drops articles: {l2}");
}

#[test]
fn forced_language_and_kind() {
    let (out, err, code) = run(
        &["-L", "pl", "-k", "prose", "-s", "-O", "chars"],
        "Zespół wyda aplikację w październiku, ponieważ klienci czekają.",
    );
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("Oct") && out.contains("∵"), "{out}");
    assert!(err.contains("lang=pl (1.00)"), "{err}");
    let (out, _, code) = run(
        &["-L", "pl", "-k", "prose"],
        "Zespół wyda aplikację w październiku, ponieważ klienci czekają.",
    );
    assert_eq!(code, 0);
    assert!(
        out.contains("Oct") && out.contains("ponieważ"),
        "1-token Polish word kept in token mode: {out}"
    );
    let (_, err, code) = run(&["-L", "xx"], "hello");
    assert_eq!(code, 2);
    assert!(err.contains("unsupported language"));
    let (_, err, code) = run(&["-k", "nope"], "hello");
    assert_eq!(code, 2, "{err}");
}

#[test]
fn legend_and_help() {
    let (out, _, code) = run(&["--legend"], "");
    assert_eq!(code, 0);
    assert!(out.contains("⇒") && out.contains("∵"));
    let (out, _, code) = run(&["--help"], "");
    assert_eq!(code, 0);
    assert!(out.contains("--level") && out.contains("--llm"));
}

#[test]
fn empty_input_is_fine() {
    let (out, _, code) = run(&[], "");
    assert_eq!(code, 0);
    assert_eq!(out, "");
    let (out, _, code) = run(&[], "   \n\n  ");
    assert_eq!(code, 0);
    assert!(out.trim().is_empty(), "{out:?}");
}

#[test]
fn reads_files() {
    let dir = std::env::temp_dir().join(format!("babeltele-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("in.txt");
    std::fs::write(
        &f,
        "There are no errors in the log because the team fixed the bug yesterday.",
    )
    .unwrap();
    let (out, err, code) = run(&["-O", "chars", f.to_str().unwrap()], "");
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("∵"), "{out}");
    let (_, err, code) = run(&["/nonexistent/babeltele/file.txt"], "");
    assert_eq!(code, 1);
    assert!(err.contains("babeltele:"));
}

#[test]
fn protected_tokens_survive() {
    let (out, _, _) = run(&[], "Run `cargo build` then open https://example.com/x?y=1 and mail bob@example.com about ./deploy.sh and db-host:5432.");
    for t in [
        "`cargo build`",
        "https://example.com/x?y=1",
        "bob@example.com",
        "./deploy.sh",
        "db-host:5432",
    ] {
        assert!(out.contains(t), "missing {t} in {out}");
    }
}
