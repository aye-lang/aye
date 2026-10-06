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

/// `browser/tests/style/X.html`: `anyknown dump --style` must print
/// `X.style`; `media.html` is also dumped 400px wide into
/// `media.w400.style`.
#[test]
fn style_fixtures() {
    let dir = root().join("browser").join("tests").join("style");
    let mut htmls: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "html"))
        .collect();
    htmls.sort();
    assert!(htmls.len() >= 20, "expected at least 20 style fixtures, found {}", htmls.len());
    let mut runs: Vec<(Vec<String>, PathBuf)> = htmls
        .iter()
        .map(|h| (vec![h.strip_prefix(root()).unwrap().to_string_lossy().into_owned()], h.with_extension("style")))
        .collect();
    runs.push((vec!["--width".into(), "400".into(), "browser/tests/style/media.html".into()], dir.join("media.w400.style")));
    let bless = std::env::var_os("ANYKNOWN_BLESS").is_some();
    let mut failures = Vec::new();
    for (args, golden) in &runs {
        let mut all = vec!["dump", "--style"];
        all.extend(args.iter().map(|s| s.as_str()));
        let out = run(&all);
        let got = text(&out.stdout);
        if !out.status.success() || !out.stderr.is_empty() {
            failures.push(format!("{args:?}: {}\n{}", out.status, text(&out.stderr)));
            continue;
        }
        if bless {
            std::fs::write(golden, &got).unwrap();
            continue;
        }
        let want = std::fs::read_to_string(golden).unwrap_or_default();
        if got != want {
            failures.push(format!("{args:?}: output differs from {}\n--- expected\n{want}--- got\n{got}", golden.display()));
        }
    }
    assert!(failures.is_empty(), "{} of {} style runs failed:\n\n{}", failures.len(), runs.len(), failures.join("\n"));
}

/// `browser/tests/layout/X.html`: `anyknown dump --layout` (fixed-metric
/// font, 800x600 viewport) must print `X.layout`; `percentages.html` is
/// also laid out 400px wide into `percentages.w400.layout`.
#[test]
fn layout_fixtures() {
    let dir = root().join("browser").join("tests").join("layout");
    let mut htmls: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "html"))
        .collect();
    htmls.sort();
    assert!(htmls.len() >= 25, "expected at least 25 layout fixtures, found {}", htmls.len());
    let mut runs: Vec<(Vec<String>, PathBuf)> = htmls
        .iter()
        .map(|h| (vec![h.strip_prefix(root()).unwrap().to_string_lossy().into_owned()], h.with_extension("layout")))
        .collect();
    runs.push((
        vec!["--width".into(), "400".into(), "browser/tests/layout/percentages.html".into()],
        dir.join("percentages.w400.layout"),
    ));
    let bless = std::env::var_os("ANYKNOWN_BLESS").is_some();
    let mut failures = Vec::new();
    for (args, golden) in &runs {
        let mut all = vec!["dump", "--layout"];
        all.extend(args.iter().map(|s| s.as_str()));
        let out = run(&all);
        let got = text(&out.stdout);
        if !out.status.success() || !out.stderr.is_empty() {
            failures.push(format!("{args:?}: {}\n{}", out.status, text(&out.stderr)));
            continue;
        }
        if bless {
            std::fs::write(golden, &got).unwrap();
            continue;
        }
        let want = std::fs::read_to_string(golden).unwrap_or_default();
        if got != want {
            failures.push(format!("{args:?}: output differs from {}\n--- expected\n{want}--- got\n{got}", golden.display()));
        }
    }
    assert!(failures.is_empty(), "{} of {} layout runs failed:\n\n{}", failures.len(), runs.len(), failures.join("\n"));
}

/// `--font system` measures text with the system's fonts through
/// FreeType; skipped when none of the known font files is there.
#[test]
fn layout_system_font() {
    let page = "browser/tests/layout/nested.html";
    let out = run(&["dump", "--layout", "--font", "system", page]);
    if text(&out.stderr).contains("no system font found") {
        println!("skipped: no system font");
        return;
    }
    assert!(out.status.success() && out.stderr.is_empty(), "{}\n{}", out.status, text(&out.stderr));
    let got = text(&out.stdout);
    assert!(got.starts_with("viewport 800x600\n  block <html> (0, 0) 800x"), "{got}");
    assert!(got.contains("\"The Ore Times\""), "{got}");
    // Proportional glyphs: widths differ from the fixed-metric font's.
    let fixed = text(&run(&["dump", "--layout", page]).stdout);
    assert_ne!(got, fixed);
}

#[test]
fn ore_unit_tests() {
    for module in ["browser", "browser/url", "browser/dom", "browser/encoding", "browser/font"] {
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
    let usage = "error: usage: anyknown dump --tree|--text|--style|--layout [--width PX] [--font fixed|system] [--id ID | --tag NAME] FILE|URL\n";
    expect(&[], 2, "", usage);
    expect(&["dump", "--tree"], 2, "", usage);
    expect(&["dump", "--layout", "--font", "serif", "browser/tests/basic.html"], 2, "", usage);
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
    // Style sheets linked from a page over HTTP are fetched relative to
    // the URL after redirects.
    let link_style = std::fs::read_to_string(root().join("browser/tests/style/link.style")).unwrap();
    expect(&["dump", "--style", &format!("{base}/redirect-style")], 0, &link_style, "");
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

// ---- the agent interface (browser/agent, docs/agent-protocol.md)

/// `s` with the repository's path written as `$ROOT`, so that goldens
/// with absolute `file:` URLs do not depend on the checkout.
fn rooted(s: &str) -> String {
    s.replace(&root().display().to_string(), "$ROOT")
}

/// Compares `got` with the golden file (or writes it when blessing).
fn check_golden(golden: &Path, got: &str, what: &str, failures: &mut Vec<String>) {
    if std::env::var_os("ANYKNOWN_BLESS").is_some() {
        std::fs::write(golden, got).unwrap();
        return;
    }
    let want = std::fs::read_to_string(golden).unwrap_or_default();
    if got != want {
        failures.push(format!("{what}: output differs from {}\n--- expected\n{want}--- got\n{got}", golden.display()));
    }
}

/// `browser/tests/agent/X.html`: `anyknown dump --a11y` must print
/// `X.a11y` and `anyknown dump --md` must print `X.md`; a few fixtures
/// also have `--boxes`, `--json` and `--refs` variants.
#[test]
fn agent_fixtures() {
    let dir = root().join("browser").join("tests").join("agent");
    let mut htmls: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "html"))
        .collect();
    htmls.sort();
    assert!(htmls.len() >= 15, "expected at least 15 agent fixtures, found {}", htmls.len());
    let mut runs: Vec<(Vec<String>, PathBuf)> = Vec::new();
    for h in &htmls {
        let rel = h.strip_prefix(root()).unwrap().to_string_lossy().into_owned();
        runs.push((vec!["--a11y".into(), rel.clone()], h.with_extension("a11y")));
        runs.push((vec!["--md".into(), rel], h.with_extension("md")));
    }
    let page = |name: &str| format!("browser/tests/agent/{name}.html");
    runs.push((vec!["--a11y".into(), "--boxes".into(), page("long_page")], dir.join("long_page.boxes.a11y")));
    runs.push((vec!["--a11y".into(), "--json".into(), page("form_login")], dir.join("form_login.json")));
    runs.push((vec!["--a11y".into(), "--json".into(), "--boxes".into(), page("tables")], dir.join("tables.boxes.json")));
    runs.push((vec!["--md".into(), "--refs".into(), page("article")], dir.join("article.refs.md")));
    let mut failures = Vec::new();
    for (args, golden) in &runs {
        let mut all = vec!["dump"];
        all.extend(args.iter().map(|s| s.as_str()));
        let out = run(&all);
        if !out.status.success() || !out.stderr.is_empty() {
            failures.push(format!("{args:?}: {}\n{}", out.status, text(&out.stderr)));
            continue;
        }
        check_golden(golden, &rooted(&text(&out.stdout)), &format!("{args:?}"), &mut failures);
    }
    assert!(failures.is_empty(), "{} of {} agent dumps failed:\n\n{}", failures.len(), runs.len(), failures.join("\n"));
}

/// `browser/tests/agent/sessions/X.jsonl`: requests fed to `anyknown
/// agent` (or `anyknown mcp` for `X.mcp.jsonl`) on stdin; the responses
/// must be `X.out` (`X.mcp.out`).
#[test]
fn agent_sessions() {
    use std::io::Write;
    let dir = root().join("browser").join("tests").join("agent").join("sessions");
    let mut inputs: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "jsonl"))
        .collect();
    inputs.sort();
    assert!(inputs.len() >= 4, "expected at least 4 agent sessions, found {}", inputs.len());
    let mut failures = Vec::new();
    for input in &inputs {
        let name = input.file_name().unwrap().to_string_lossy().into_owned();
        let mode = if name.ends_with(".mcp.jsonl") { "mcp" } else { "agent" };
        let mut child = Command::new(anyknown())
            .current_dir(root())
            .arg(mode)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(&std::fs::read(input).unwrap()).unwrap();
        let out = child.wait_with_output().unwrap();
        if !out.status.success() || !out.stderr.is_empty() {
            failures.push(format!("{name}: {}\n{}", out.status, text(&out.stderr)));
            continue;
        }
        check_golden(&input.with_extension("out"), &rooted(&text(&out.stdout)), &name, &mut failures);
    }
    assert!(failures.is_empty(), "{} of {} agent sessions failed:\n\n{}", failures.len(), inputs.len(), failures.join("\n"));
}

#[test]
fn agent_errors() {
    let usage = "error: usage: anyknown dump --a11y [--boxes] [--json] | --md [--refs] [--width PX] FILE|URL\n";
    expect(&["dump", "--a11y"], 2, "", usage);
    expect(&["dump", "--md", "--width", "x", "browser/tests/agent/article.html"], 2, "", usage);
    expect(&["dump", "--md", "browser/tests/agent/missing.html"], 1, "", &format!(
        "error: cannot fetch file://{}/browser/tests/agent/missing.html: not found\n",
        root().display()
    ));
    expect(&["agent", "--color"], 2, "", "error: usage: anyknown agent|mcp [--width PX] [--height PX]\n");
    expect(&["dump", "--a11y", "data:text/html,<button>OK</button>"], 0, "- button \"OK\" [e1]\n", "");
    expect(&["dump", "--md", "data:text/html,<h2>Hi</h2><p>a <b>b</b>"], 0, "## Hi\n\na **b**\n", "");
}

/// The agent over HTTP: a link followed from a page served by the test
/// server.
#[test]
fn agent_http() {
    use std::io::Write;
    let Some(srv) = server() else {
        println!("skipped: cannot start python3 for the test server");
        return;
    };
    let base = &srv.base;
    let mut child = Command::new(anyknown())
        .current_dir(root())
        .arg("agent")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let input = format!(
        "{{\"id\":1,\"cmd\":\"navigate\",\"url\":\"{base}/agent/site/index.html\"}}\n{{\"id\":2,\"cmd\":\"click\",\"ref\":\"e1\"}}\n"
    );
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success() && out.stderr.is_empty(), "{}\n{}", out.status, text(&out.stderr));
    let got = text(&out.stdout);
    let lines: Vec<&str> = got.lines().collect();
    assert_eq!(lines.len(), 2, "{got}");
    assert!(lines[0].contains("\"title\":\"Site home\""), "{got}");
    assert!(lines[1].contains(&format!("\"url\":\"{base}/agent/site/docs.html\"")), "{got}");
    assert!(lines[1].contains("heading \\\"Documentation\\\" [level=1] [e10]"), "{got}");
}
