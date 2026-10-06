//! Tests of the anyknown browser (`browser/`, written in Ore).
//!
//! - `browser/tests/X.html`: `anyknown dump --tree` must print `X.tree` and
//!   `anyknown dump --text` must print `X.txt`, with an empty stderr (the
//!   browser is built in debug mode, so a leak would show there).
//! - `ore test` on the browser's root module and on its standalone modules
//!   (`url`, `dom`, `encoding`) must pass.
//! - A few command-line cases: `data:` URLs, `--id`/`--tag`, errors.
//! - `browser/tests/js/X.js`: `anyknown js` must print `X.out` (Node's
//!   output) and exit 0 with an empty stderr, or, when `X.err` exists,
//!   print it on stderr and exit 1 (an uncaught exception).
//!
//! To accept new output after a deliberate change, run
//! `ANYKNOWN_BLESS=1 cargo test --test browser` and review the diff.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn ore() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_ore"));
    c.current_dir(root());
    c
}

/// The browser, built once for all tests.
fn anyknown() -> &'static Path {
    static EXE: OnceLock<PathBuf> = OnceLock::new();
    EXE.get_or_init(|| {
        let dir = std::env::temp_dir().join(format!("anyknown-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("anyknown");
        let out = ore().arg("build").arg("browser").arg("-o").arg(&exe).output().unwrap();
        assert!(out.status.success(), "building the browser failed:\n{}", String::from_utf8_lossy(&out.stderr));
        exe
    })
}

fn run(args: &[&str]) -> Output {
    Command::new(anyknown()).current_dir(root()).args(args).output().unwrap()
}

fn text(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

#[test]
fn fixtures() {
    let dir = root().join("browser").join("tests");
    let mut htmls: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "html"))
        .collect();
    htmls.sort();
    assert!(htmls.len() >= 20, "expected at least 20 fixtures, found {}", htmls.len());
    let bless = std::env::var_os("ANYKNOWN_BLESS").is_some();
    let mut failures = Vec::new();
    for html in &htmls {
        let rel = html.strip_prefix(root()).unwrap().to_string_lossy().into_owned();
        for (flag, ext) in [("--tree", "tree"), ("--text", "txt")] {
            let out = run(&["dump", flag, &rel]);
            let got = text(&out.stdout);
            let golden = html.with_extension(ext);
            if !out.status.success() || !out.stderr.is_empty() {
                failures.push(format!("{rel} {flag}: {}\n{}", out.status, text(&out.stderr)));
                continue;
            }
            if bless {
                std::fs::write(&golden, &got).unwrap();
                continue;
            }
            let want = std::fs::read_to_string(&golden).unwrap_or_default();
            if got != want {
                failures.push(format!("{rel} {flag}: output differs from {}\n--- expected\n{want}--- got\n{got}", golden.display()));
            }
        }
    }
    assert!(failures.is_empty(), "{} of {} fixture runs failed:\n\n{}", failures.len(), htmls.len() * 2, failures.join("\n"));
}

#[test]
fn ore_unit_tests() {
    for module in ["browser", "browser/url", "browser/dom", "browser/encoding", "browser/js"] {
        let out = ore().arg("test").arg(module).output().unwrap();
        let stdout = text(&out.stdout);
        assert!(
            out.status.success() && stdout.contains(" passed; 0 failed"),
            "`ore test {module}` failed:\n{stdout}{}",
            text(&out.stderr)
        );
    }
}

/// `anyknown args...` must exit with `status` and print exactly `stdout`
/// (and `stderr`).
fn expect(args: &[&str], status: i32, stdout: &str, stderr: &str) {
    let out = run(args);
    assert_eq!(text(&out.stdout), stdout, "stdout of anyknown {args:?}");
    assert_eq!(text(&out.stderr), stderr, "stderr of anyknown {args:?}");
    assert_eq!(out.status.code(), Some(status), "status of anyknown {args:?}");
}

#[test]
fn data_urls() {
    expect(&["dump", "--text", "data:text/html,<h1>Hi</h1><p>there%21"], 0, "Hi\n\nthere!\n", "");
    expect(
        &["dump", "--tree", "data:text/html;base64,PHA+Q2Fm6TwvcD4="],
        0,
        "#document\n  <html>\n    <head>\n    <body>\n      <p>\n        \"Café\"\n",
        "",
    );
    // Plain text is shown as it is.
    expect(&["dump", "--text", "data:,a  <b>%0A  c"], 0, "a  <b>\n  c\n", "");
}

#[test]
fn queries() {
    let page = "browser/tests/basic.html";
    expect(&["dump", "--tree", "--id", "main", page], 0, "<div id=\"main\" class=\"content\">\n  \"\\n    \"\n  <p>\n    \"Second paragraph.\"\n  \"\\n  \"\n", "");
    expect(&["dump", "--text", "--tag", "P", page], 0, "This is a small page with a link.\nSecond paragraph.\n", "");
    expect(&["dump", "--text", "--id", "none", page], 0, "", "");
}

#[test]
fn errors() {
    let usage = "error: usage: anyknown dump --tree|--text [--id ID | --tag NAME] FILE|URL\n";
    expect(&[], 2, "", usage);
    expect(&["dump", "--tree"], 2, "", usage);
    expect(&["dump", "--tree", "browser/tests/missing.html"], 1, "", &format!(
        "error: cannot fetch file://{}/browser/tests/missing.html: not found\n",
        root().display()
    ));
    expect(&["dump", "--text", "https://example.com/"], 1, "", "error: cannot fetch https://example.com/: this kind of URL is not supported yet\n");
    expect(&["dump", "--text", "data:text/html;base64,%%%"], 1, "", "error: cannot fetch data:text/html;base64,%%%: invalid data: URL\n");
}

/// One JS conformance script: None when it passes, else what went wrong.
fn js_case(js: &Path, bless: bool) -> Option<String> {
    let rel = js.strip_prefix(root()).unwrap().to_string_lossy().into_owned();
    let out = run(&["js", &rel]);
    let got = text(&out.stdout);
    let golden = js.with_extension("out");
    if bless {
        std::fs::write(&golden, &got).unwrap();
        return None;
    }
    let want = std::fs::read_to_string(&golden).unwrap_or_default();
    let mut problems = Vec::new();
    if got != want {
        problems.push(format!("stdout differs from {}\n--- expected\n{want}--- got\n{got}", golden.display()));
    }
    let stderr = text(&out.stderr);
    match std::fs::read_to_string(js.with_extension("err")) {
        Ok(want_err) => {
            if stderr != want_err || out.status.code() != Some(1) {
                problems.push(format!("expected status 1 and stderr {want_err:?}, got {} and {stderr:?}", out.status));
            }
        }
        Err(_) => {
            if !out.status.success() || !stderr.is_empty() {
                problems.push(format!("expected status 0 and no stderr, got {}:\n{stderr}", out.status));
            }
        }
    }
    if problems.is_empty() { None } else { Some(format!("{rel}: {}", problems.join("\n"))) }
}

#[test]
fn js_conformance() {
    let dir = root().join("browser").join("tests").join("js");
    let mut scripts: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "js"))
        .collect();
    scripts.sort();
    assert!(scripts.len() >= 150, "expected at least 150 JS scripts, found {}", scripts.len());
    anyknown();
    let bless = std::env::var_os("ANYKNOWN_BLESS").is_some();
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    let failures: Vec<String> = std::thread::scope(|s| {
        let handles: Vec<_> = scripts
            .chunks(scripts.len().div_ceil(threads))
            .map(|chunk| s.spawn(move || chunk.iter().filter_map(|js| js_case(js, bless)).collect::<Vec<_>>()))
            .collect();
        handles.into_iter().flat_map(|h| h.join().unwrap()).collect()
    });
    assert!(failures.is_empty(), "{} of {} JS scripts failed:\n\n{}", failures.len(), scripts.len(), failures.join("\n\n"));
}

#[test]
fn js_command_line() {
    expect(&["js", "browser/tests/js/missing.js"], 1, "", "error: cannot read browser/tests/js/missing.js\n");
}
