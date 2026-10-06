//! Tests of the anyknown browser (`browser/`, written in Ore).
//!
//! - `browser/tests/X.html`: `anyknown dump --tree` must print `X.tree` and
//!   `anyknown dump --text` must print `X.txt`, with an empty stderr (the
//!   browser is built in debug mode, so a leak would show there).
//! - `ore test` on the browser's root module and on its standalone modules
//!   (`url`, `dom`, `encoding`) must pass.
//! - A few command-line cases: `data:` URLs, `--id`/`--tag`, errors.
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
    for module in ["browser", "browser/url", "browser/dom", "browser/encoding"] {
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
    expect(&["dump", "--text", "ftp://example.com/"], 1, "", "error: cannot fetch ftp://example.com/: this kind of URL is not supported yet\n");
    expect(&["dump", "--text", "data:text/html;base64,%%%"], 1, "", "error: cannot fetch data:text/html;base64,%%%: invalid data: URL\n");
}

/// The local test server (`browser/tests/server.py`) serving
/// `browser/tests`; killed when dropped. `None` without python3.
struct Server {
    child: std::process::Child,
    base: String,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn server() -> Option<Server> {
    use std::io::BufRead;
    let tests = root().join("browser").join("tests");
    let mut child = Command::new("python3")
        .arg("-I")
        .arg(tests.join("server.py"))
        .arg(&tests)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;
    let mut line = String::new();
    std::io::BufReader::new(child.stdout.take().unwrap()).read_line(&mut line).ok()?;
    let port: u16 = line.trim().parse().ok()?;
    Some(Server { child, base: format!("http://127.0.0.1:{port}") })
}

#[test]
fn http() {
    let Some(srv) = server() else {
        println!("skipped: cannot start python3 for the test server");
        return;
    };
    let base = &srv.base;
    let basic = std::fs::read_to_string(root().join("browser/tests/basic.txt")).unwrap();
    // A page over HTTP reads as from a file, also after a redirect.
    expect(&["dump", "--text", &format!("{base}/basic.html")], 0, &basic, "");
    expect(&["dump", "--text", &format!("{base}/redirect")], 0, &basic, "");
    expect(&["dump", "--text", &format!("{base}/agent")], 0, "anyknown/0.1\n", "");
    expect(&["dump", "--text", &format!("{base}/gzip")], 0, &format!("{}\n", ["compressed"; 50].join(" ")), "");
    // The charset of the Content-Type header decodes the body.
    expect(&["dump", "--text", &format!("{base}/latin1")], 0, "Café à la crème\n", "");
    // An error status still has a page.
    let out = run(&["dump", "--text", &format!("{base}/missing.html")]);
    assert_eq!(out.status.code(), Some(0));
    assert!(text(&out.stdout).contains("404"), "{}", text(&out.stdout));
    expect(
        &["dump", "--text", &format!("{base}/redirect-loop")],
        1,
        "",
        &format!("error: cannot fetch {base}/redirect-loop: too many redirects\n"),
    );
    // Nothing listens on port 1.
    expect(&["dump", "--text", "http://127.0.0.1:1/"], 1, "", "error: cannot fetch http://127.0.0.1:1/: cannot connect\n");
}
