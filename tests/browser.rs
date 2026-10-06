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
    for module in ["browser", "browser/url", "browser/dom", "browser/encoding", "browser/font", "browser/js", "browser/image"] {
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
    let usage = "error: usage: anyknown dump --tree|--text|--style|--layout [--js] [--width PX] [--font fixed|system] [--id ID | --tag NAME] FILE|URL\n";
    expect(&[], 2, "", "error: usage: anyknown [open] FILE|URL | dump --tree|--text|--style|--layout|--a11y|--md ... FILE|URL | screenshot FILE|URL -o OUT.png | js FILE | agent | mcp\n");
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
    expect(&["dump", "--text", &format!("{base}/agent")], 0, "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) anyknown/0.1\n", "");
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

// ---- B4: paint and window

/// A scratch directory for the files the browser writes.
fn scratch() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("anyknown-out-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// `browser/tests/paint/X.html`: `anyknown screenshot --font fixed` in a
/// 400x300 viewport must write exactly `X.png`; `full.html` is also taken
/// with `--full` into `full.full.png`.
#[test]
fn screenshot_fixtures() {
    let dir = root().join("browser").join("tests").join("paint");
    let mut htmls: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "html"))
        .collect();
    htmls.sort();
    assert!(htmls.len() >= 10, "expected at least 10 paint fixtures, found {}", htmls.len());
    let mut runs: Vec<(String, Vec<&str>, PathBuf)> =
        htmls.iter().map(|h| (h.strip_prefix(root()).unwrap().to_string_lossy().into_owned(), vec![], h.with_extension("png"))).collect();
    runs.push(("browser/tests/paint/full.html".into(), vec!["--full"], dir.join("full.full.png")));
    let bless = std::env::var_os("ANYKNOWN_BLESS").is_some();
    let out_dir = scratch();
    let mut failures = Vec::new();
    for (i, (page, extra, golden)) in runs.iter().enumerate() {
        let out = out_dir.join(format!("shot{i}.png"));
        let mut args = vec!["screenshot", page.as_str(), "-o", out.to_str().unwrap(), "--font", "fixed", "--width", "400", "--height", "300"];
        args.extend(extra.iter());
        let res = run(&args);
        if !res.status.success() || !res.stderr.is_empty() {
            failures.push(format!("{page} {extra:?}: {}\n{}", res.status, text(&res.stderr)));
            continue;
        }
        let got = std::fs::read(&out).unwrap();
        if bless {
            std::fs::write(golden, &got).unwrap();
            continue;
        }
        let want = std::fs::read(golden).unwrap_or_default();
        if got != want {
            failures.push(format!("{page} {extra:?}: the image differs from {} (new one: {})", golden.display(), out.display()));
        }
    }
    assert!(failures.is_empty(), "{} of {} screenshots failed:\n\n{}", failures.len(), runs.len(), failures.join("\n"));
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { 0xEDB8_8320 ^ (crc >> 1) } else { crc >> 1 };
        }
    }
    !crc
}

/// Checks a PNG: signature, chunk CRCs, IHDR (8-bit RGBA), and, when
/// python3 is there, that zlib inflates the IDAT data to the image's
/// size. Returns (width, height).
fn check_png(path: &Path) -> (u32, u32) {
    let d = std::fs::read(path).unwrap();
    assert_eq!(&d[..8], b"\x89PNG\r\n\x1a\n", "{}", path.display());
    let be = |b: &[u8]| u32::from_be_bytes([b[0], b[1], b[2], b[3]]);
    let (mut i, mut size, mut kinds) = (8, (0, 0), Vec::new());
    while i < d.len() {
        let n = be(&d[i..]) as usize;
        let kind = &d[i + 4..i + 8];
        assert_eq!(crc32(&d[i + 4..i + 8 + n]), be(&d[i + 8 + n..]), "CRC of {} in {}", String::from_utf8_lossy(kind), path.display());
        if kind == b"IHDR" {
            size = (be(&d[i + 8..]), be(&d[i + 12..]));
            assert_eq!(&d[i + 16..i + 21], &[8, 6, 0, 0, 0], "bit depth, color type and methods");
        }
        kinds.push(String::from_utf8_lossy(kind).into_owned());
        i += 12 + n;
    }
    assert_eq!(kinds, ["IHDR", "IDAT", "IEND"], "{}", path.display());
    let script = "import sys, zlib, struct\nd = open(sys.argv[1], 'rb').read()\nn = struct.unpack('>I', d[33:37])[0]\nprint(len(zlib.decompress(d[41:41 + n])))";
    if let Ok(out) = Command::new("python3").args(["-I", "-c", script]).arg(path).output() {
        if out.status.success() {
            let len: u64 = text(&out.stdout).trim().parse().unwrap();
            assert_eq!(len, size.1 as u64 * (size.0 as u64 * 4 + 1), "inflated size of {}", path.display());
        }
    }
    size
}

#[test]
fn png_validity() {
    let out = scratch().join("valid.png");
    let res = run(&["screenshot", "browser/tests/article.html", "-o", out.to_str().unwrap(), "--font", "fixed", "--width", "320", "--height", "200"]);
    assert!(res.status.success() && res.stderr.is_empty(), "{}", text(&res.stderr));
    assert_eq!(check_png(&out), (320, 200));
    let full = scratch().join("valid_full.png");
    let res = run(&["screenshot", "browser/tests/paint/full.html", "-o", full.to_str().unwrap(), "--font", "fixed", "--width", "300", "--height", "100", "--full"]);
    assert!(res.status.success(), "{}", text(&res.stderr));
    assert_eq!(check_png(&full), (300, 416));
    for golden in std::fs::read_dir(root().join("browser/tests/paint")).unwrap() {
        let p = golden.unwrap().path();
        if p.extension().is_some_and(|e| e == "png") {
            check_png(&p);
        }
    }
}

/// `--font system` draws FreeType glyphs; skipped without a system font.
#[test]
fn screenshot_system_font() {
    let out = scratch().join("system.png");
    let res = run(&["screenshot", "browser/tests/article.html", "-o", out.to_str().unwrap(), "--font", "system"]);
    if text(&res.stderr).contains("no system font found") {
        println!("skipped: no system font");
        return;
    }
    assert!(res.status.success() && res.stderr.is_empty(), "{}\n{}", res.status, text(&res.stderr));
    assert_eq!(check_png(&out), (800, 600));
    let fixed = scratch().join("system_fixed.png");
    run(&["screenshot", "browser/tests/article.html", "-o", fixed.to_str().unwrap(), "--font", "fixed"]);
    assert_ne!(std::fs::read(&out).unwrap(), std::fs::read(&fixed).unwrap());
}

#[test]
fn screenshot_errors() {
    let usage = "error: usage: anyknown screenshot FILE|URL -o OUT.png [--width W] [--height H] [--font fixed|system] [--full]\n";
    expect(&["screenshot", "browser/tests/basic.html"], 2, "", usage);
    expect(&["screenshot", "-o", "x.png"], 2, "", usage);
    expect(&["screenshot", "browser/tests/basic.html", "-o", "x.png", "--width", "0"], 2, "", usage);
    expect(&["screenshot", "browser/tests/missing.html", "-o", "x.png", "--font", "fixed"], 1, "", &format!(
        "error: cannot fetch file://{}/browser/tests/missing.html: not found\n",
        root().display()
    ));
}

fn has_sdl() -> bool {
    Command::new("pkg-config").args(["--exists", "sdl2"]).status().is_ok_and(|s| s.success())
}

/// `anyknown args...` with SDL's dummy video driver (no display).
fn run_window(args: &[&str]) -> Output {
    Command::new(anyknown()).current_dir(root()).env("SDL_VIDEODRIVER", "dummy").args(args).output().unwrap()
}

/// The window, headless: a frame or two, then exit 0 with nothing on
/// stderr (no leaks). Skipped when SDL2 is not installed.
#[test]
fn window_frames() {
    if !has_sdl() {
        println!("skipped: no SDL2");
        return;
    }
    for args in [vec!["--frames", "1", "browser/tests/article.html"], vec!["open", "browser/tests/links.html", "--frames", "2", "--font", "fixed"]] {
        let out = run_window(&args);
        assert!(out.status.success(), "anyknown {args:?}: {}\n{}{}", out.status, text(&out.stdout), text(&out.stderr));
        assert!(out.stderr.is_empty(), "anyknown {args:?}: {}", text(&out.stderr));
    }
    // The last frame, saved: the window's size, chrome included.
    let png = scratch().join("window.png");
    let out = run_window(&["browser/tests/paint/text.html", "--font", "fixed", "--width", "500", "--height", "300", "--frames", "1", "--save", png.to_str().unwrap()]);
    assert!(out.status.success() && out.stderr.is_empty(), "{}", text(&out.stderr));
    assert_eq!(check_png(&png), (500, 300));
    expect(&["--frames", "1", "--bogus", "x.html"], 2, "", "error: usage: anyknown [open] FILE|URL [--width W] [--height H] [--font fixed|system] [--no-js] [--frames N [--events SCRIPT] [--state] [--text] [--save OUT.png]]\n");
}

/// Scripted input (`--events`, ui/script.ore) through the window's event
/// handler: scrolling, links, fragments, history, the address bar, find.
#[test]
fn window_events() {
    if !has_sdl() {
        println!("skipped: no SDL2");
        return;
    }
    let ui = format!("file://{}/browser/tests/ui/", root().display());
    // A 400x340 window: the page is 1260 px tall in a 300 px viewport.
    let cases: &[(&str, &str)] = &[
        ("key:down wheel:-1", "url long.html\nscroll 80\nfocus page\naddress long.html\nhistory 1 of 1\nmatches 0\nstatus \n"),
        ("key:end key:pageup key:space key:up", "url long.html\nscroll 920\nfocus page\naddress long.html\nhistory 1 of 1\nmatches 0\nstatus \n"),
        ("key:end key:home wheel:3", "url long.html\nscroll 0\nfocus page\naddress long.html\nhistory 1 of 1\nmatches 0\nstatus \n"),
        // A link to a fragment scrolls to its element (y 958); the link
        // keeps the focus.
        ("click:30,58", "url long.html#end\nscroll 958\nfocus page\naddress long.html#end\nhistory 2 of 2\nmatches 0\nstatus \nelement a\n"),
        // Back and forward restore scroll offsets.
        ("key:pagedown key:home click:30,58 key:cmd+[ key:cmd+]", "url long.html#end\nscroll 958\nfocus page\naddress long.html#end\nhistory 2 of 2\nmatches 0\nstatus \n"),
        ("click:130,58 key:cmd+[", "url long.html\nscroll 0\nfocus page\naddress long.html\nhistory 1 of 2\nmatches 0\nstatus \n"),
        ("move:130,58", "url long.html\nscroll 0\nfocus page\naddress long.html\nhistory 1 of 1\nmatches 0\nstatus other.html\n"),
        // The back button, the forward button, reload.
        ("click:130,58 click:20,20 click:50,20 click:85,20", "url other.html\nscroll 0\nfocus page\naddress other.html\nhistory 2 of 2\nmatches 0\nstatus \n"),
        // Typing in the address bar (Cmd+L selects everything first).
        ("key:cmd+l text:other.htm key:backspace text:ml key:escape", "url long.html\nscroll 0\nfocus page\naddress long.html\nhistory 1 of 1\nmatches 0\nstatus \n"),
        ("key:cmd+l text:browser/tests/ui/other.html", "url long.html\nscroll 0\nfocus address\naddress browser/tests/ui/other.html\nhistory 1 of 1\nmatches 0\nstatus \n"),
        ("key:cmd+l text:browser/tests/ui/other.html key:return", "url other.html\nscroll 0\nfocus page\naddress other.html\nhistory 2 of 2\nmatches 0\nstatus \n"),
        // Find in page: matches, Enter scrolls to the next one.
        // ("To the end", "Other", "The end"; the third is at y 962, shown a
        // third down the 268 px left above the find bar.)
        ("key:cmd+f text:the", "url long.html\nscroll 0\nfocus find\naddress long.html\nhistory 1 of 1\nmatches 3\nstatus \n"),
        ("key:cmd+f text:the key:return key:return", "url long.html\nscroll 873\nfocus find\naddress long.html\nhistory 1 of 1\nmatches 3\nstatus \n"),
        ("key:cmd+f text:the key:return key:return key:shift+return", "url long.html\nscroll 0\nfocus find\naddress long.html\nhistory 1 of 1\nmatches 3\nstatus \n"),
        ("key:cmd+f text:the key:escape", "url long.html\nscroll 0\nfocus page\naddress long.html\nhistory 1 of 1\nmatches 0\nstatus \n"),
    ];
    let mut failures = Vec::new();
    for (events, want) in cases {
        let out = run_window(&["browser/tests/ui/long.html", "--font", "fixed", "--width", "400", "--height", "340", "--frames", "1", "--state", "--events", events]);
        let got = text(&out.stdout).replace(&ui, "");
        if !out.status.success() || !out.stderr.is_empty() || got != *want {
            failures.push(format!("--events {events:?}: {}\n{}--- expected\n{want}--- got\n{got}", out.status, text(&out.stderr)));
        }
    }
    assert!(failures.is_empty(), "{} of {} scripts failed:\n\n{}", failures.len(), cases.len(), failures.join("\n"));
}

/// Forms in the window (`browser/tests/ui/form.html`, 400x340): clicks
/// and Tab focus controls, typing edits a field, Enter submits (by GET
/// to `result.html`), Space toggles a checkbox, a select opens a
/// dropdown and takes arrow keys, labels and reset buttons work, the
/// clipboard copies and pastes. `--state` shows the focused element and
/// its value.
#[test]
fn window_forms() {
    if !has_sdl() {
        println!("skipped: no SDL2");
        return;
    }
    let ui = format!("file://{}/browser/tests/ui/", root().display());
    let state = |url: &str, rest: &str| format!("url {url}\nscroll 0\nfocus page\naddress {url}\nhistory {}\nmatches 0\nstatus \n{rest}", if url == "form.html" { "1 of 1" } else { "2 of 2" });
    // The query field is at (60, 60), the Go button at (170, 60), the
    // label "All" at (45, 95), the select at (40, 125), the textarea at
    // (50, 165), the reset button at (30, 205), the link at (80, 205).
    let cases: Vec<(&str, String)> = vec![
        ("click:60,60 key:end text:_there key:return", state("result.html?q=hi+there&go=1&lang=en&note=&tags=b", "")),
        ("click:60,60 text:!", state("form.html", "element input name=q\nvalue hi!\n")),
        ("click:60,60 key:backspace key:left text:_ key:delete key:home text:o", state("form.html", "element input name=q\nvalue o \n")),
        ("click:60,60 key:shift+home text:yo", state("form.html", "element input name=q\nvalue yo\n")),
        (
            "key:tab key:tab key:tab key:space key:tab key:down key:down key:tab text:a key:return text:b",
            state("form.html", "element textarea name=note\nvalue a\nb\n"),
        ),
        (
            "key:tab key:tab key:tab key:space key:tab key:down key:down key:tab text:a key:return text:b click:170,60",
            state("result.html?q=hi&go=1&all=yes&lang=ja&note=a%0D%0Ab&tags=b", ""),
        ),
        ("click:45,95 click:170,60", state("result.html?q=hi&go=1&all=yes&lang=en&note=&tags=b", "")),
        ("click:40,125 click:40,170", state("form.html", "element select name=lang\nvalue fr\n")),
        // The disabled option is skipped by Enter (it stays on English)...
        ("click:40,125 key:down key:down key:return", state("form.html", "element select name=lang\nvalue en\n")),
        ("click:40,125 key:down key:down key:down key:return", state("form.html", "element select name=lang\nvalue ja\n")),
        // ...and by the arrow keys on the closed select.
        ("click:40,125 key:escape key:down key:down", state("form.html", "element select name=lang\nvalue ja\n")),
        ("click:40,125 key:escape click:170,60", state("result.html?q=hi&go=1&lang=en&note=&tags=b", "")),
        ("click:60,60 text:xyz click:30,205 click:170,60", state("result.html?q=hi&go=1&lang=en&note=&tags=b", "")),
        ("click:60,60 key:cmd+a key:cmd+c key:end key:cmd+v", state("form.html", "element input name=q\nvalue hihi\n")),
        ("click:60,60 key:left key:shift+left key:cmd+x key:end key:cmd+v", state("form.html", "element input name=q\nvalue ih\n")),
        ("click:60,60 key:escape", state("form.html", "")),
        ("click:60,60 click:300,300", state("form.html", "")),
        ("key:shift+tab key:shift+tab key:return", state("other.html", "")),
        // The list box (rows a, b, c at y 240, 257, 273): a click toggles
        // an option of a `multiple` select.
        ("click:20,273 click:20,240 click:170,60", state("result.html?q=hi&go=1&lang=en&note=&tags=a&tags=b&tags=c", "")),
        ("click:20,257 click:20,273", state("form.html", "element select name=tags\nvalue c\n")),
    ];
    let mut failures = Vec::new();
    for (events, want) in &cases {
        let out = run_window(&["browser/tests/ui/form.html", "--font", "fixed", "--width", "400", "--height", "340", "--frames", "1", "--state", "--events", events]);
        let got = text(&out.stdout).replace(&ui, "");
        if !out.status.success() || !out.stderr.is_empty() || got != *want {
            failures.push(format!("--events {events:?}: {}\n{}--- expected\n{want}--- got\n{got}", out.status, text(&out.stderr)));
        }
    }
    assert!(failures.is_empty(), "{} of {} scripts failed:\n\n{}", failures.len(), cases.len(), failures.join("\n"));
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
    let usage = "error: usage: anyknown dump --a11y [--boxes] [--json] | --md [--refs] [--js] [--width PX] FILE|URL\n";
    expect(&["dump", "--a11y"], 2, "", usage);
    expect(&["dump", "--md", "--width", "x", "browser/tests/agent/article.html"], 2, "", usage);
    expect(&["dump", "--md", "browser/tests/agent/missing.html"], 1, "", &format!(
        "error: cannot fetch file://{}/browser/tests/agent/missing.html: not found\n",
        root().display()
    ));
    expect(&["agent", "--color"], 2, "", "error: usage: anyknown agent|mcp [--width PX] [--height PX] [--no-js]\n");
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

/// Form submission over HTTP: `browser/tests/agent/http/X.jsonl` (with
/// `$BASE` for the test server's address) fed to `anyknown agent` must
/// print `X.out`. The forms in `browser/tests/forms/` submit to the
/// server's `/echo`, whose page shows the request (method, path,
/// Content-Type, fields, raw body).
#[test]
fn agent_forms_http() {
    use std::io::Write;
    let Some(srv) = server() else {
        println!("skipped: cannot start python3 for the test server");
        return;
    };
    let dir = root().join("browser").join("tests").join("agent").join("http");
    let mut inputs: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "jsonl"))
        .collect();
    inputs.sort();
    assert!(inputs.len() >= 2, "expected at least 2 HTTP sessions, found {}", inputs.len());
    let mut failures = Vec::new();
    for input in &inputs {
        let name = input.file_name().unwrap().to_string_lossy().into_owned();
        let mut child = Command::new(anyknown())
            .current_dir(root())
            .arg("agent")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let requests = std::fs::read_to_string(input).unwrap().replace("$BASE", &srv.base);
        child.stdin.take().unwrap().write_all(requests.as_bytes()).unwrap();
        let out = child.wait_with_output().unwrap();
        if !out.status.success() || !out.stderr.is_empty() {
            failures.push(format!("{name}: {}\n{}", out.status, text(&out.stderr)));
            continue;
        }
        let got = rooted(&text(&out.stdout)).replace(&srv.base, "$BASE");
        check_golden(&input.with_extension("out"), &got, &name, &mut failures);
    }
    assert!(failures.is_empty(), "{} of {} HTTP sessions failed:\n\n{}", failures.len(), inputs.len(), failures.join("\n"));
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
    expect(&["js"], 2, "", "error: usage: anyknown js FILE\n");
}

// ---- page scripts and the DOM bindings (browser/engine/script.ore)

/// The window, headless: a click and a key reach the page's listeners,
/// which change the text (`--text` prints the document's text at the
/// end); with `--no-js` nothing changes.
#[test]
fn window_scripts() {
    if !has_sdl() {
        println!("skipped: no SDL2");
        return;
    }
    let cases: &[(&[&str], &str)] = &[
        (&["--events", "click:50,65"], "\nclicked\n"),
        (&["--events", "key:x"], "\nkey x\n"),
        (&["--events", "click:50,200"], "\nbefore\n"),
        (&["--events", "click:50,65", "--no-js"], "\nbefore\n"),
    ];
    for (extra, want) in cases {
        let mut args = vec!["browser/tests/ui/script.html", "--font", "fixed", "--width", "400", "--height", "300", "--frames", "1", "--text"];
        args.extend_from_slice(extra);
        let out = run_window(&args);
        let got = text(&out.stdout);
        assert!(out.status.success() && out.stderr.is_empty(), "{args:?}: {}\n{}", out.status, text(&out.stderr));
        assert!(got.contains(want), "{args:?}: expected {want:?} in the text:\n{got}");
    }
}

/// One run of a DOM fixture: `args` then the page; stdout must be
/// `golden`, and stderr (console messages, uncaught exceptions) must be
/// `err_golden`, or empty when there is no such file.
fn jsdom_case(args: &[&str], page: &Path, golden: &Path, err_golden: Option<&Path>, bless: bool) -> Option<String> {
    let rel = page.strip_prefix(root()).unwrap().to_string_lossy().into_owned();
    let mut all: Vec<&str> = args.to_vec();
    all.push(&rel);
    let out = run(&all);
    let got = rooted(&text(&out.stdout));
    let stderr = rooted(&text(&out.stderr));
    if bless {
        std::fs::write(golden, &got).unwrap();
        if let Some(e) = err_golden {
            if stderr.is_empty() {
                let _ = std::fs::remove_file(e);
            } else {
                std::fs::write(e, &stderr).unwrap();
            }
        }
        return None;
    }
    let mut problems = Vec::new();
    if !out.status.success() {
        problems.push(format!("status {}", out.status));
    }
    let want = std::fs::read_to_string(golden).unwrap_or_default();
    if got != want {
        problems.push(format!("stdout differs from {}\n--- expected\n{want}--- got\n{got}", golden.display()));
    }
    let want_err = err_golden.and_then(|e| std::fs::read_to_string(e).ok()).unwrap_or_default();
    if stderr != want_err {
        problems.push(format!("stderr differs\n--- expected\n{want_err}--- got\n{stderr}"));
    }
    if problems.is_empty() { None } else { Some(format!("{rel} {args:?}: {}", problems.join("\n"))) }
}

/// `browser/tests/jsdom/X.html`: `anyknown dump --tree --js` must print
/// `X.tree` (the document after its scripts and timers ran), with the
/// console on stderr equal to `X.err` (or empty). `X.style` and `X.a11y`,
/// when they exist, are `dump --style --js` and `dump --a11y --js`.
#[test]
fn jsdom_fixtures() {
    let dir = root().join("browser").join("tests").join("jsdom");
    let mut pages: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "html"))
        .collect();
    pages.sort();
    assert!(pages.len() >= 25, "expected at least 25 DOM fixtures, found {}", pages.len());
    let bless = std::env::var_os("ANYKNOWN_BLESS").is_some();
    let mut runs: Vec<(Vec<&str>, PathBuf, PathBuf, Option<PathBuf>)> = Vec::new();
    for p in &pages {
        runs.push((vec!["dump", "--tree", "--js"], p.clone(), p.with_extension("tree"), Some(p.with_extension("err"))));
        for (flag, ext) in [("--style", "style"), ("--a11y", "a11y")] {
            let golden = p.with_extension(ext);
            if golden.exists() {
                runs.push((vec!["dump", flag, "--js"], p.clone(), golden, None));
            }
        }
    }
    anyknown();
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    let failures: Vec<String> = std::thread::scope(|s| {
        let handles: Vec<_> = runs
            .chunks(runs.len().div_ceil(threads))
            .map(|chunk| {
                s.spawn(move || {
                    chunk
                        .iter()
                        .filter_map(|(args, page, golden, err)| jsdom_case(args, page, golden, err.as_deref(), bless))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles.into_iter().flat_map(|h| h.join().unwrap()).collect()
    });
    assert!(failures.is_empty(), "{} of {} DOM fixture runs failed:\n\n{}", failures.len(), runs.len(), failures.join("\n\n"));
}

/// The DOM fixtures again with a collection at every allocation
/// (`ANYKNOWN_JS_GC=stress`): the bindings keep what they hold reachable.
/// `gc_wrappers` allocates too much for that.
#[test]
fn jsdom_gc_stress() {
    let dir = root().join("browser").join("tests").join("jsdom");
    let mut pages: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "html") && !p.ends_with("gc_wrappers.html"))
        .collect();
    pages.sort();
    anyknown();
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    let failures: Vec<String> = std::thread::scope(|s| {
        let handles: Vec<_> = pages
            .chunks(pages.len().div_ceil(threads))
            .map(|chunk| {
                s.spawn(move || {
                    let mut bad = Vec::new();
                    for page in chunk {
                        let rel = page.strip_prefix(root()).unwrap().to_string_lossy().into_owned();
                        let out = Command::new(anyknown())
                            .current_dir(root())
                            .env("ANYKNOWN_JS_GC", "stress")
                            .args(["dump", "--tree", "--js", &rel])
                            .output()
                            .unwrap();
                        let want = std::fs::read_to_string(page.with_extension("tree")).unwrap_or_default();
                        if !out.status.success() || rooted(&text(&out.stdout)) != want {
                            bad.push(format!("{rel}: {}\n{}", out.status, text(&out.stderr)));
                        }
                    }
                    bad
                })
            })
            .collect();
        handles.into_iter().flat_map(|h| h.join().unwrap()).collect()
    });
    assert!(failures.is_empty(), "{} DOM fixtures failed under GC stress:\n\n{}", failures.len(), failures.join("\n\n"));
}

// ---- B8: net

/// Request headers, cookies kept in a file, size limits, TLS errors and
/// `<meta charset>` over HTTP. (Cookies, the Referer and the cache within
/// one session: `browser/tests/agent/http/net_cookies.jsonl`.)
#[test]
fn net_http() {
    let Some(srv) = server() else {
        println!("skipped: cannot start python3 for the test server");
        return;
    };
    let base = &srv.base;
    // A URL the user typed: no Referer, no cookies yet.
    let headers = "Accept: text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8\nAccept-Language: en-US,en;q=0.9\nReferer: \nCookie: \n";
    expect(&["dump", "--text", &format!("{base}/headers")], 0, headers, "");
    // ANYKNOWN_COOKIE_FILE keeps the persistent cookies (p, Max-Age)
    // from one run to the next; session cookies (a, d) end with the run.
    let file = scratch().join("cookies.txt");
    let _ = std::fs::remove_file(&file);
    let with_file = |url: String| {
        Command::new(anyknown()).current_dir(root()).env("ANYKNOWN_COOKIE_FILE", &file).args(["dump", "--text", &url]).output().unwrap()
    };
    let first = with_file(format!("{base}/net/start"));
    assert!(first.status.success() && first.stderr.is_empty(), "{}", text(&first.stderr));
    let saved = std::fs::read_to_string(&file).unwrap();
    assert!(saved.starts_with("127.0.0.1\t1\t/\t0\t0\t") && saved.ends_with("\tp\t5\n") && saved.lines().count() == 1, "{saved}");
    let second = with_file(format!("{base}/headers"));
    assert!(text(&second.stdout).contains("\nCookie: p=5\n"), "{}", text(&second.stdout));
    // Too large: 33 MiB, with and without a Content-Length.
    for big in ["big", "big?stream"] {
        expect(&["dump", "--text", &format!("{base}/{big}")], 1, "", &format!("error: cannot fetch {base}/{big}: the response is too large\n"));
    }
    // TLS to a server that speaks plain HTTP.
    let https = base.replace("http://", "https://");
    expect(&["dump", "--text", &format!("{https}/")], 1, "", &format!("error: cannot fetch {https}/: the secure connection failed (TLS)\n"));
    // No charset in the Content-Type: the <meta charset> decides.
    expect(&["dump", "--text", &format!("{base}/meta-charset")], 0, "\u{201c}quoted\u{201d} caf\u{e9}\n", "");
}

// ---- B8: HTML and CSS

/// `noscript` follows scripting: the dumps without `--js` show it (the
/// agent fixture `noscript`), with `--js` they leave it out.
#[test]
fn noscript_with_scripts() {
    let page = "browser/tests/agent/noscript.html";
    expect(&["dump", "--text", "--js", page], 0, "Scripts\n\nAlways here.\n\nAn icon and a formula .\n", "");
    expect(&["dump", "--md", "--js", page], 0, "# Scripts\n\nAlways here.\n\nAn icon and a formula .\n", "");
    expect(
        &["dump", "--a11y", "--js", page],
        0,
        "- heading \"Scripts\" [level=1] [e1]\n- paragraph: Always here.\n- paragraph: An icon and a formula .\n",
        "",
    );
}
