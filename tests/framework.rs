//! Tests of the web framework (`framework/`, written in Aye).
//!
//! - `framework/examples/hello` is built and started with `--port 0`; it
//!   prints `listening on 127.0.0.1:<port>`.
//! - `tests/framework/hello/X.http` is the response the server must send to
//!   the method and path in `HELLO_CASES` (with `Connection: close`), with
//!   line endings read as `\n`.
//! - `framework/examples/pages` is an app of `app/` directories: `web gen`
//!   (`framework/gen`, run with `aye run`) writes its `.web/`, which is
//!   checked, built and started the same way. `tests/framework/pages/X.http`
//!   are the responses to the requests in `PAGES_CASES` (pages in the root
//!   layout and a nested one, a `[...path]` catch-all with percent-decoding,
//!   a `(marketing)` group whose layout wraps `/pricing`, the `not-found.aye` page for a path and for a
//!   page that returns `Error.NotFound`, an API `route.aye` with JSON, a
//!   `{"error":"NotFound"}` and a 405, the guestbook (a form `POST` that
//!   answers 303 with `Location` and `Set-Cookie`, the page it points to,
//!   a `{"error":"BadRequest"}` and a `{"error":"PayloadTooLarge"}`), and the files of `public/` with
//!   their conditional requests). The `ETag` and `Last-Modified` values of a
//!   file depend on its modification time in the checkout, so the
//!   transcripts hold `<etag>` and `<date>` there; `PAGES_ETAG_CASES` send
//!   the `ETag` of a first `GET` back as `If-None-Match`. `web gen`
//!   reads the exports of the files with `aye doc --signatures`, so it runs
//!   with `AYE` set to the `aye` under test. It also builds the counter
//!   island into `.web/client/counter.wasm` (only where the wasm32 toolchain
//!   is; else it prints a warning and the island cases are skipped): the
//!   server answers `/_web/islands.js` (`PAGES_CASES`); `/counter` has the
//!   island `div` that `web.island` writes and, once, the script tag that the
//!   server adds (`/about` has none); and
//!   `/_web/islands/<name>.wasm` with `application/wasm`, a 404 for a
//!   missing name and a 400 for `..` (`PAGES_ISLAND_CASES`,
//!   `PAGES_BINARY_CASES`; the body of the `.wasm` must start with `\0asm`).
//! - `framework/dev` (`web dev`) is run with `aye run` on a copy of the pages
//!   example, in its own process group: it generates, checks and builds the
//!   app, starts the server and relays its `listening on` line. Killing the
//!   group must leave no server behind. When the test edits
//!   `app/about/page.aye` of its copy, the CLI prints `restarting`, starts
//!   the server again on the same port and relays the new `listening on`
//!   line; a change in `public/` restarts nothing.
//! - `tests/framework/dev_reload.mjs` (Node 22; the test is skipped when
//!   `node` is missing or older) plays the browser against `web dev`: it
//!   runs the page's `/_web/dev.js` in a fake `window` with an SSE client.
//!   When the test breaks `app/about/page.aye`, the script reloads once (to
//!   the error page); when the test fixes it, it reloads a second time.
//! - `framework/examples/pages/islands/counter` (an island: `view/` for both
//!   builds, `client.aye` for wasm32-freestanding) is built for
//!   wasm32-freestanding, and `tests/framework/island_exports.mjs` lists the
//!   imports and exports of the module: the three `dom_*` and `env`'s
//!   `aye_write`, `aye_abort` and `strtod` (from `std.json`), no others;
//!   `mount`, `event`, `memory` and `malloc` among the exports. Skipped
//!   without clang's wasm32 backend or `node`.
//! - `tests/framework/islands_glue.mjs` (Node 22) runs the text of
//!   `islandsScript()` in a `vm` sandbox with a fake DOM: the counter island
//!   (built as above) starts at `Count: 3`, two clicks give `Count: 5`, and
//!   an island whose `.wasm` is missing logs one `console.error`. Skipped
//!   like `island_build`.
//! - `tests/framework/island_e2e.mjs` (Node 22) plays the browser against a
//!   running app: it fetches `/counter`, builds a fake DOM from the island
//!   `div`, runs the served `/_web/islands.js` with a real `fetch` of the
//!   `.wasm`, clicks twice and prints `text=Count: 5` (`island_e2e`, on a
//!   copy of the pages example). `dev_island` runs it against `web dev`
//!   (the dev tag comes before the islands tag), changes the label in
//!   `islands/counter/view/view.aye` from `Count: ` to `Total: `, waits for
//!   the restart and expects `text=Total: 5`: `web gen` rebuilt the island.
//!   Skipped like `island_build`.
//! - `web gen --prod` (`gen_prod`, on a copy of the pages example with a
//!   2 MiB `public/big.bin`, a dotfile and a symlink) also writes
//!   `.web/assets.aye`: one `@embedFile` per regular file of `public/` and
//!   per `.wasm` of `.web/client/`, the public ones with a hashed URL from
//!   the first 8 hex digits of the file's SHA-256 (`shasum`); the dotfile and
//!   the link are left out. `aye check` and `aye build` of `.web` pass, and a
//!   gen without `--prod` writes no `assets.aye`.
//! - `prod_serve` generates the pages example with `--prod`, builds `.web`,
//!   renames the app tree away and starts the executable from another
//!   directory with `--dev`: `/style.css` (`max-age=0`) and its hashed URL
//!   (`immutable`, which `/about` links) are 200 from memory with the
//!   SHA-256 ETag and no `Last-Modified`, `If-None-Match` gives 304, `HEAD`
//!   has no body, `/img/logo.svg` is served, `/nope.css`, a traversal path,
//!   `/_web/dev.js` and `/_web/events` are 404 and no page gets the dev
//!   script; `/_web/islands/counter.wasm` is served when the island was built.
//! - The `test` blocks of `framework/web` run with `aye test`.

use aye::driver;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// A method, a path and the file of `tests/framework/hello/` with its
/// response.
const HELLO_CASES: &[(&str, &str, &str)] = &[
    ("GET", "/", "root.http"),
    ("HEAD", "/", "head.http"),
    ("POST", "/", "not_allowed.http"),
    ("GET", "/nope", "missing.http"),
];

/// A method, a path and the file of `tests/framework/pages/` with its
/// response.
const PAGES_CASES: &[(&str, &str, &str)] = &[
    ("GET", "/", "root.http"),
    ("GET", "/about", "about.http"),
    ("GET", "/counter", "counter.http"),
    ("GET", "/blog/hello", "blog_hello.http"),
    ("GET", "/blog/layouts", "blog_layouts.http"),
    ("GET", "/api/items/1", "api_item.http"),
    ("GET", "/api/items/9", "api_item_missing.http"),
    ("POST", "/api/items/1", "api_item_post.http"),
    ("GET", "/blog/missing", "blog_missing.http"),
    ("GET", "/guestbook", "guestbook_empty.http"),
    ("GET", "/docs/a/b%20c", "docs_rest.http"),
    ("GET", "/docs/a/b/c", "docs_slash.http"),
    ("GET", "/docs", "docs_missing.http"),
    ("GET", "/pricing", "pricing.http"),
    ("GET", "/%70ricing", "pricing.http"),
    ("GET", "/nope", "missing.http"),
    ("GET", "/style.css", "style_css.http"),
    ("HEAD", "/style.css", "style_css_head.http"),
    ("GET", "/img/logo.svg", "logo_svg.http"),
    ("GET", "/missing.css", "missing_css.http"),
    ("GET", "/../secret", "traversal.http"),
    ("GET", "/_web/islands.js", "islands_js.http"),
];

/// A method, a path and the file of `tests/framework/pages/` with its
/// response, for the requests that need the islands built (`web gen` builds
/// them only where the wasm32 toolchain is).
const PAGES_ISLAND_CASES: &[(&str, &str, &str)] = &[
    ("GET", "/_web/islands/nope.wasm", "islands_nope_wasm.http"),
    ("GET", "/_web/islands/../x", "islands_traversal.http"),
];

/// A method, a path, the bytes the body must start with and the file of
/// `tests/framework/pages/` with the head of the response (`split_binary`):
/// the islands' `.wasm` files, which are not text.
const PAGES_BINARY_CASES: &[(&str, &str, &[u8], &str)] = &[(
    "GET",
    "/_web/islands/counter.wasm",
    b"\0asm",
    "islands_counter_wasm.http",
)];

/// A method, a path, extra header lines (each ending in `\r\n`), a body and
/// the file of `tests/framework/pages/` with the response: requests that
/// carry cookies or a form.
const PAGES_BODY_CASES: &[(&str, &str, &str, &str, &str)] = &[
    (
        "GET",
        "/api/echo?q=a%20b+c%zz&q=2",
        "Cookie: id=1; name=anchor%20x \r\n",
        "",
        "api_echo.http",
    ),
    ("GET", "/api/echo", "", "", "api_echo_none.http"),
    (
        "POST",
        "/api/echo",
        "Content-Type: application/x-www-form-urlencoded\r\n",
        "x=1&msg=hi+there%21",
        "api_echo_post.http",
    ),
    (
        "POST",
        "/api/echo",
        "Content-Type: text/plain\r\n",
        "msg=hi",
        "api_echo_post_text.http",
    ),
    (
        "POST",
        "/guestbook",
        "Content-Type: application/x-www-form-urlencoded\r\n",
        "name=Ann+B",
        "guestbook_post.http",
    ),
    (
        "GET",
        "/guestbook?added=1",
        "Cookie: last=Ann%20B\r\n",
        "",
        "guestbook_added.http",
    ),
    (
        "POST",
        "/guestbook",
        "Content-Type: application/x-www-form-urlencoded\r\n",
        "name=%3Ci%3E",
        "guestbook_post_escape.http",
    ),
    (
        "GET",
        "/guestbook",
        "Cookie: last=%3Ci%3E\r\n",
        "",
        "guestbook_escape.http",
    ),
    (
        "POST",
        "/guestbook",
        "Content-Type: application/x-www-form-urlencoded\r\n",
        "name=",
        "guestbook_post_empty.http",
    ),
    (
        "POST",
        "/guestbook",
        "Content-Type: application/x-www-form-urlencoded\r\n",
        "name=aaaaaaaaaabbbbbbbbbbccccccccccddddddddddE",
        "guestbook_post_long.http",
    ),
    (
        "GET",
        "/style.css",
        "If-Modified-Since: Fri, 01 Jan 2100 00:00:00 GMT\r\n",
        "",
        "style_css_ims.http",
    ),
    (
        "GET",
        "/style.css",
        "If-Modified-Since: Sat, 01 Jan 2000 00:00:00 GMT\r\n",
        "",
        "style_css_ims_old.http",
    ),
];

/// A method, a path and the file of `tests/framework/pages/` with the
/// response to the request that has the `ETag` of a first `GET` of the path
/// as its `If-None-Match`.
const PAGES_ETAG_CASES: &[(&str, &str, &str)] = &[("GET", "/style.css", "style_css_etag.http")];

/// The built app, killed when dropped.
struct App {
    child: Child,
    port: u16,
}

impl Drop for App {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The CLI of `web dev`, run in its own process group; the group is killed
/// when it is dropped, as the server is a grandchild.
struct Dev {
    child: Child,
    port: u16,
    /// The lines the CLI prints after the first `listening on` line.
    lines: Receiver<String>,
}

impl Dev {
    /// Sends SIGTERM to the whole process group (`kill` complains when the
    /// group is already gone, which `Drop` does not mind).
    fn kill_group(&self) {
        let _ = Command::new("kill")
            .args(["-s", "TERM", "--", &format!("-{}", self.child.id())])
            .stderr(Stdio::null())
            .status();
    }
}

impl Drop for Dev {
    fn drop(&mut self) {
        self.kill_group();
        let _ = self.child.wait();
    }
}

/// Copies the directory `from` to `to` (which must not exist), leaving out
/// `.web`.
fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        if name == ".web" {
            continue;
        }
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &to.join(name));
        } else {
            std::fs::copy(entry.path(), to.join(name)).unwrap();
        }
    }
}

/// A port that nothing listens on.
fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// Runs `aye run framework/dev -- <app> --port <port>` and reads its output
/// until the `listening on` line.
fn start_dev(app: &Path, port: u16) -> Dev {
    start_dev_until(app, port, &format!("listening on 127.0.0.1:{port}"))
}

/// Like `start_dev`, and reads until the line `want`.
fn start_dev_until(app: &Path, port: u16, want: &str) -> Dev {
    let child = Command::new(env!("CARGO_BIN_EXE_aye"))
        .current_dir(root())
        .env("AYE", env!("CARGO_BIN_EXE_aye"))
        .args(["run", "framework/dev", "--"])
        .arg(app)
        .args(["--port", &port.to_string()])
        .stdout(Stdio::piped())
        .process_group(0)
        .spawn()
        .unwrap();
    let mut dev = Dev {
        child,
        port,
        lines: mpsc::channel().1,
    };
    let mut lines = BufReader::new(dev.child.stdout.take().unwrap());
    loop {
        let mut line = String::new();
        let n = lines.read_line(&mut line).unwrap();
        assert!(n > 0, "web dev ended before it printed {want:?}");
        if line.trim().ends_with(want) {
            break;
        }
    }
    // Keep reading so that the CLI never blocks on a full pipe.
    let (tx, rx) = mpsc::channel();
    dev.lines = rx;
    std::thread::spawn(move || {
        let mut line = String::new();
        while lines.read_line(&mut line).is_ok_and(|n| n > 0) {
            if tx.send(line.trim().to_string()).is_err() {
                return;
            }
            line.clear();
        }
    });
    dev
}

/// Waits at most `limit` for a line of the CLI that contains `want`; the
/// lines before it are dropped.
fn wait_line(dev: &Dev, want: &str, limit: Duration) -> bool {
    wait_recv(&dev.lines, want, limit)
}

/// Waits at most `limit` for a line from `lines` that contains `want`; the
/// lines before it are dropped.
fn wait_recv(lines: &Receiver<String>, want: &str, limit: Duration) -> bool {
    let end = Instant::now() + limit;
    while let Some(left) = end.checked_duration_since(Instant::now()) {
        match lines.recv_timeout(left) {
            Ok(line) if line.contains(want) => return true,
            Ok(_) => {}
            Err(RecvTimeoutError::Timeout) => return false,
            Err(RecvTimeoutError::Disconnected) => return false,
        }
    }
    false
}

/// Does something listen on `port`?
fn listening(port: u16) -> bool {
    TcpStream::connect(("127.0.0.1", port)).is_ok()
}

/// Builds `framework/examples/<name>` into the target directory.
fn build(name: &str) -> PathBuf {
    build_dir(name, &root().join("framework").join("examples").join(name))
}

/// Builds the module in `dir` into the target directory as `<name>`.
fn build_dir(name: &str, dir: &Path) -> PathBuf {
    let exe = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("framework-{name}"));
    let out = Command::new(env!("CARGO_BIN_EXE_aye"))
        .current_dir(root())
        .arg("build")
        .arg(dir)
        .arg("-o")
        .arg(&exe)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "building {name} failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    exe
}

/// Starts `exe --port 0` and reads the port from its first line.
fn start(exe: &Path) -> App {
    start_with(exe, &[])
}

/// Like `start`, with `extra` arguments after `--port 0`.
fn start_with(exe: &Path, extra: &[&str]) -> App {
    start_in(exe, None, extra)
}

/// Like `start_with`, run in the directory `cwd` (this process's own when
/// `None`).
fn start_in(exe: &Path, cwd: Option<&Path>, extra: &[&str]) -> App {
    let mut command = Command::new(exe);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    let mut child = command
        .args(["--port", "0"])
        .args(extra)
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let port = line
        .trim()
        .strip_prefix("listening on 127.0.0.1:")
        .unwrap_or_else(|| panic!("unexpected first line: {line:?}"))
        .parse()
        .unwrap();
    App { child, port }
}

/// The response to `method path`, read until the server closes, with `\r\n`
/// as `\n`.
fn request(app: &App, method: &str, path: &str) -> String {
    request_with(app, method, path, "", "")
}

/// Like `request`, with `extra` header lines (each ending in `\r\n`) and a
/// body.
fn request_with(app: &App, method: &str, path: &str, extra: &str, body: &str) -> String {
    String::from_utf8(request_raw(app, method, path, extra, body))
        .unwrap()
        .replace("\r\n", "\n")
}

/// Like `request_with`, with the response as it came (a binary body is not
/// text).
fn request_raw(app: &App, method: &str, path: &str, extra: &str, body: &str) -> Vec<u8> {
    let mut s = TcpStream::connect(("127.0.0.1", app.port)).unwrap();
    s.set_read_timeout(Some(std::time::Duration::from_secs(10)))
        .unwrap();
    let length = if body.is_empty() {
        String::new()
    } else {
        format!("Content-Length: {}\r\n", body.len())
    };
    write!(
        s,
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n{extra}{length}\r\n{body}"
    )
    .unwrap();
    let mut buf = Vec::new();
    s.read_to_end(&mut buf).unwrap();
    buf
}

/// `response` with the values of its `ETag` and `Last-Modified` lines
/// replaced by `<etag>` and `<date>`.
fn scrub(response: &str) -> String {
    response
        .split_inclusive('\n')
        .map(|line| {
            if line.starts_with("ETag: ") {
                "ETag: <etag>\n"
            } else if line.starts_with("Last-Modified: ") {
                "Last-Modified: <date>\n"
            } else {
                line
            }
        })
        .collect()
}

/// The head of a binary `response` (up to and including the blank line,
/// `\r\n` as `\n`, scrubbed, `Content-Length` as `<length>`) and its body;
/// the length is checked against the body.
fn split_binary(response: &[u8]) -> (String, &[u8]) {
    let at = response
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("no end of headers")
        + 4;
    let head = String::from_utf8(response[..at].to_vec())
        .unwrap()
        .replace("\r\n", "\n");
    let (body, length) = (&response[at..], header_value(&head, "Content-Length"));
    assert_eq!(length, body.len().to_string(), "Content-Length");
    let head = scrub(&head)
        .split_inclusive('\n')
        .map(|line| {
            if line.starts_with("Content-Length: ") {
                "Content-Length: <length>\n"
            } else {
                line
            }
        })
        .collect();
    (head, body)
}

/// The value of the header called `name` in `response`.
fn header_value<'a>(response: &'a str, name: &str) -> &'a str {
    response
        .lines()
        .find_map(|line| line.strip_prefix(name)?.strip_prefix(": "))
        .unwrap_or_else(|| panic!("no {name} header in {response:?}"))
}

#[test]
fn hello() {
    let app = start(&build("hello"));
    for (method, path, file) in HELLO_CASES {
        let want =
            std::fs::read_to_string(root().join("tests/framework/hello").join(file)).unwrap();
        assert_eq!(request(&app, method, path), want, "{method} {path}");
    }
}

#[test]
fn pages() {
    let app = root().join("framework").join("examples").join("pages");
    let generated = Command::new(env!("CARGO_BIN_EXE_aye"))
        .current_dir(root())
        .env("AYE", env!("CARGO_BIN_EXE_aye"))
        .arg("run")
        .arg(root().join("framework").join("gen"))
        .arg("--")
        .arg(&app)
        .arg("--web")
        .arg(root().join("framework").join("web"))
        .output()
        .unwrap();
    assert!(
        generated.status.success(),
        "web gen failed:\n{}{}",
        String::from_utf8_lossy(&generated.stdout),
        String::from_utf8_lossy(&generated.stderr)
    );
    let web = app.join(".web");
    let check = Command::new(env!("CARGO_BIN_EXE_aye"))
        .current_dir(root())
        .arg("check")
        .arg(&web)
        .output()
        .unwrap();
    assert!(
        check.status.success(),
        "aye check .web failed:\n{}{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
    let app = start(&build_dir("pages", &web));
    for (method, path, file) in PAGES_CASES {
        let want =
            std::fs::read_to_string(root().join("tests/framework/pages").join(file)).unwrap();
        assert_eq!(scrub(&request(&app, method, path)), want, "{method} {path}");
    }
    let islands = driver::find_target("wasm32-freestanding")
        .unwrap()
        .toolchain_available();
    assert_eq!(
        web.join("client/counter.wasm").is_file(),
        islands,
        "web gen builds the island exactly when the wasm32 toolchain is there"
    );
    if islands {
        for (method, path, file) in PAGES_ISLAND_CASES {
            let want =
                std::fs::read_to_string(root().join("tests/framework/pages").join(file)).unwrap();
            assert_eq!(scrub(&request(&app, method, path)), want, "{method} {path}");
        }
        for (method, path, prefix, file) in PAGES_BINARY_CASES {
            let want =
                std::fs::read_to_string(root().join("tests/framework/pages").join(file)).unwrap();
            let response = request_raw(&app, method, path, "", "");
            let (head, body) = split_binary(&response);
            assert_eq!(head, want, "{method} {path}");
            assert!(body.starts_with(prefix), "{method} {path}: {body:?}");
        }
    } else {
        println!("skipped: no wasm32 clang; the islands were not built");
    }
    for (method, path, file) in PAGES_ETAG_CASES {
        let want =
            std::fs::read_to_string(root().join("tests/framework/pages").join(file)).unwrap();
        let first = request(&app, method, path);
        let etag = header_value(&first, "ETag");
        let extra = format!("If-None-Match: {etag}\r\n");
        assert_eq!(
            scrub(&request_with(&app, method, path, &extra, "")),
            want,
            "{method} {path} with its ETag"
        );
    }
    for (method, path, extra, body, file) in PAGES_BODY_CASES {
        let want =
            std::fs::read_to_string(root().join("tests/framework/pages").join(file)).unwrap();
        assert_eq!(
            scrub(&request_with(&app, method, path, extra, body)),
            want,
            "{method} {path}"
        );
    }
}

/// The build id in the first `build` frame of `GET /_web/events` on `app`:
/// the status line and headers are checked on the way. Reads for up to 5 s.
fn first_build_id(app: &App) -> String {
    let mut s = TcpStream::connect(("127.0.0.1", app.port)).unwrap();
    s.set_read_timeout(Some(Duration::from_millis(200)))
        .unwrap();
    write!(
        s,
        "GET /_web/events HTTP/1.1\r\nHost: 127.0.0.1\r\nAccept: text/event-stream\r\n\r\n"
    )
    .unwrap();
    let limit = Instant::now() + Duration::from_secs(5);
    let mut got = String::new();
    while Instant::now() < limit {
        let mut buf = [0u8; 512];
        match s.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => got.push_str(&String::from_utf8_lossy(&buf[..n]).replace("\r\n", "\n")),
            Err(_) => {}
        }
        if let Some(at) = got.find("event: build\ndata: ") {
            let rest = &got[at + "event: build\ndata: ".len()..];
            if let Some(end) = rest.find("\n\n") {
                assert!(got.starts_with("HTTP/1.1 200 OK\n"), "{got:?}");
                assert_eq!(header_value(&got, "Content-Type"), "text/event-stream");
                return rest[..end].to_string();
            }
        }
    }
    panic!("no build frame: {got:?}");
}

#[test]
fn dev_script() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("framework-dev-script");
    let _ = std::fs::remove_dir_all(&dir);
    copy_dir(&root().join("framework/examples/pages"), &dir);
    let generated = Command::new(env!("CARGO_BIN_EXE_aye"))
        .current_dir(root())
        .env("AYE", env!("CARGO_BIN_EXE_aye"))
        .arg("run")
        .arg(root().join("framework").join("gen"))
        .arg("--")
        .arg(&dir)
        .arg("--web")
        .arg(root().join("framework").join("web"))
        .output()
        .unwrap();
    assert!(
        generated.status.success(),
        "web gen failed:\n{}{}",
        String::from_utf8_lossy(&generated.stdout),
        String::from_utf8_lossy(&generated.stderr)
    );
    let exe = build_dir("dev-script-server", &dir.join(".web"));
    let tag = "<script src=\"/_web/dev.js\"></script>";

    let app = start_with(&exe, &["--dev"]);
    let about = request(&app, "GET", "/about");
    assert_eq!(about.matches(tag).count(), 1, "{about}");
    assert!(about.contains(&format!("{tag}</body>")), "{about}");
    assert!(request(&app, "GET", "/").contains(tag));
    let js = request(&app, "GET", "/_web/dev.js");
    assert!(js.starts_with("HTTP/1.1 200 OK\n"), "{js}");
    assert!(
        header_value(&js, "Content-Type").starts_with("text/javascript"),
        "{js}"
    );
    assert!(js.contains("new EventSource(\"/_web/events\")"), "{js}");
    assert!(js.contains("location.reload()"), "{js}");
    let api = request(&app, "GET", "/api/items/1");
    assert!(!api.contains(tag), "{api}");
    let id = first_build_id(&app);
    assert!(id.parse::<u64>().is_ok(), "{id:?}");
    assert_eq!(first_build_id(&app), id, "same id on a second connection");

    std::thread::sleep(Duration::from_millis(20));
    let again = start_with(&exe, &["--dev"]);
    assert_ne!(first_build_id(&again), id, "another id after a restart");

    let plain = start(&exe);
    assert!(!request(&plain, "GET", "/about").contains(tag));
    assert!(request(&plain, "GET", "/_web/dev.js").starts_with("HTTP/1.1 404 "));
    assert!(request(&plain, "GET", "/_web/events").starts_with("HTTP/1.1 404 "));
}

#[test]
fn dev_start() {
    let app = Path::new(env!("CARGO_TARGET_TMPDIR")).join("framework-dev-start");
    let _ = std::fs::remove_dir_all(&app);
    copy_dir(&root().join("framework/examples/pages"), &app);
    let dev = start_dev(&app, free_port());
    let want = std::fs::read_to_string(root().join("tests/framework/pages/about.http")).unwrap();
    let server = App {
        child: Command::new("true").spawn().unwrap(),
        port: dev.port,
    };
    // The CLI starts the server with `--dev`: the page has the script tag
    // and the length of it, and is the transcript otherwise.
    let got = request(&server, "GET", "/about");
    let tag = "<script src=\"/_web/dev.js\"></script>";
    assert!(got.contains(&format!("{tag}</body>")), "{got}");
    let body = |s: &str| s.split_once("\n\n").unwrap().1.replace(tag, "");
    assert_eq!(body(&got), body(&want));
    dev.kill_group();
    let mut gone = false;
    for _ in 0..50 {
        if !listening(dev.port) {
            gone = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    assert!(gone, "the server still listens after the group was killed");
}

#[test]
fn dev_reload() {
    let app = Path::new(env!("CARGO_TARGET_TMPDIR")).join("framework-dev-reload");
    let _ = std::fs::remove_dir_all(&app);
    copy_dir(&root().join("framework/examples/pages"), &app);
    let dev = start_dev(&app, free_port());
    let server = App {
        child: Command::new("true").spawn().unwrap(),
        port: dev.port,
    };
    assert!(request(&server, "GET", "/about").contains("<h1>About</h1>"));

    let page = app.join("app/about/page.aye");
    let source = std::fs::read_to_string(&page).unwrap();
    std::fs::write(&page, source.replace("<h1>About</h1>", "<h1>About us</h1>")).unwrap();
    let limit = Duration::from_secs(120);
    assert!(
        wait_line(&dev, "restarting (app/about/page.aye changed)", limit),
        "no restart line"
    );
    let want = format!("listening on 127.0.0.1:{}", dev.port);
    assert!(
        wait_line(&dev, &want, limit),
        "no {want:?} after the restart"
    );
    assert!(request(&server, "GET", "/about").contains("<h1>About us</h1>"));

    let css = app.join("public/style.css");
    let style = std::fs::read_to_string(&css).unwrap();
    std::fs::write(&css, format!("{style}\n/* edited */\n")).unwrap();
    assert!(
        !wait_line(&dev, "restarting", Duration::from_secs(1)),
        "a change in public/ restarted the server"
    );
    assert!(request(&server, "GET", "/style.css").contains("/* edited */"));
}

/// `GET path` on `port`, read until the server closes, with `\r\n` as `\n`;
/// null while nothing answers (or the answer is cut off).
fn try_get(port: u16, path: &str) -> Option<String> {
    let mut s = TcpStream::connect(("127.0.0.1", port)).ok()?;
    s.set_read_timeout(Some(Duration::from_secs(10))).ok()?;
    write!(
        s,
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n"
    )
    .ok()?;
    let mut buf = Vec::new();
    s.read_to_end(&mut buf).ok()?;
    Some(String::from_utf8(buf).ok()?.replace("\r\n", "\n"))
}

/// Polls `GET path` every 200 ms, for at most `limit`, until the response
/// starts with `status_line`; null on a timeout.
fn wait_status(port: u16, path: &str, status_line: &str, limit: Duration) -> Option<String> {
    let end = Instant::now() + limit;
    while Instant::now() < end {
        if let Some(response) = try_get(port, path) {
            if response.starts_with(status_line) {
                return Some(response);
            }
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    None
}

#[test]
fn dev_error() {
    let app = Path::new(env!("CARGO_TARGET_TMPDIR")).join("framework-dev-error");
    let _ = std::fs::remove_dir_all(&app);
    copy_dir(&root().join("framework/examples/pages"), &app);
    let dev = start_dev(&app, free_port());
    let limit = Duration::from_secs(120);
    assert!(
        wait_status(dev.port, "/about", "HTTP/1.1 200", limit).is_some(),
        "no page before the break"
    );

    // A type error: `raw` takes text.
    let page = app.join("app/about/page.aye");
    let source = std::fs::read_to_string(&page).unwrap();
    std::fs::write(&page, source.replace("raw(\"<h1>About</h1>\")", "raw(5)")).unwrap();
    assert!(
        wait_line(&dev, "restarting (app/about/page.aye changed)", limit),
        "no restart line"
    );
    let response =
        wait_status(dev.port, "/", "HTTP/1.1 500", limit).expect("no error page after the break");
    assert_eq!(
        header_value(&response, "Content-Type"),
        "text/html; charset=utf-8"
    );
    assert!(
        response.contains("app/about/page.aye:"),
        "no mapped path in {response}"
    );
    assert!(
        response.contains("mismatched types"),
        "no message in {response}"
    );
    assert!(!response.contains(".web/"), "a .web path in {response}");
    // Every path gets the page.
    let other = try_get(dev.port, "/about").unwrap();
    assert!(other.starts_with("HTTP/1.1 500"), "{other}");

    std::fs::write(&page, source).unwrap();
    let want = format!("listening on 127.0.0.1:{}", dev.port);
    assert!(wait_line(&dev, &want, limit), "no {want:?} after the fix");
    let response = wait_status(dev.port, "/about", "HTTP/1.1 200", Duration::from_secs(10))
        .expect("no page after the fix");
    assert!(response.contains("<h1>About</h1>"), "{response}");
}

#[test]
fn dev_broken() {
    let app = Path::new(env!("CARGO_TARGET_TMPDIR")).join("framework-dev-broken");
    let _ = std::fs::remove_dir_all(&app);
    copy_dir(&root().join("framework/examples/pages"), &app);
    let page = app.join("app/about/page.aye");
    let source = std::fs::read_to_string(&page).unwrap();
    let broken = source.replace("raw(\"<h1>About</h1>\")", "raw(5)");
    std::fs::write(&page, &broken).unwrap();

    // The first build fails: the CLI serves the error page from the start.
    let port = free_port();
    let dev = start_dev_until(
        &app,
        port,
        &format!("serving the error page: 127.0.0.1:{port}"),
    );
    let limit = Duration::from_secs(120);
    let response = try_get(dev.port, "/about").expect("no answer");
    assert!(response.starts_with("HTTP/1.1 500"), "{response}");
    assert!(response.contains("app/about/page.aye:"), "{response}");
    assert!(response.contains("mismatched types"), "{response}");

    std::fs::write(&page, &source).unwrap();
    let want = format!("listening on 127.0.0.1:{}", dev.port);
    assert!(wait_line(&dev, &want, limit), "no {want:?} after the fix");
    let response = wait_status(dev.port, "/about", "HTTP/1.1 200", Duration::from_secs(10))
        .expect("no page after the fix");
    assert!(response.contains("<h1>About</h1>"), "{response}");

    // Broken a second time while the server runs.
    std::fs::write(&page, &broken).unwrap();
    wait_status(dev.port, "/about", "HTTP/1.1 500", limit).expect("no error page the second time");
}

/// The build id of the first `build` frame of `GET /_web/events` on `port`.
fn events_id(port: u16) -> String {
    first_build_id(&App {
        child: Command::new("true").spawn().unwrap(),
        port,
    })
}

#[test]
fn dev_reload_cycle() {
    let app = Path::new(env!("CARGO_TARGET_TMPDIR")).join("framework-dev-cycle");
    let _ = std::fs::remove_dir_all(&app);
    copy_dir(&root().join("framework/examples/pages"), &app);
    let dev = start_dev(&app, free_port());
    let limit = Duration::from_secs(120);
    let tag = "<script src=\"/_web/dev.js\"></script>";
    // The app server's id.
    let a = events_id(dev.port);
    assert!(a.parse::<u64>().is_ok(), "{a:?}");

    let page = app.join("app/about/page.aye");
    let source = std::fs::read_to_string(&page).unwrap();
    std::fs::write(&page, source.replace("raw(\"<h1>About</h1>\")", "raw(5)")).unwrap();
    assert!(
        wait_line(&dev, "serving the error page", limit),
        "no error page line"
    );
    let response = try_get(dev.port, "/").expect("no answer");
    assert!(response.starts_with("HTTP/1.1 500"), "{response}");
    assert_eq!(response.matches(tag).count(), 1, "{response}");
    let js = try_get(dev.port, "/_web/dev.js").expect("no script");
    assert!(js.starts_with("HTTP/1.1 200 OK\n"), "{js}");
    assert!(
        header_value(&js, "Content-Type").starts_with("text/javascript"),
        "{js}"
    );
    assert!(js.contains("location.reload()"), "{js}");
    // The CLI's id for this broken build.
    let b = events_id(dev.port);
    assert!(b.parse::<u64>().is_ok(), "{b:?}");
    assert_ne!(b, a, "the error server has its own id");
    assert_eq!(
        events_id(dev.port),
        b,
        "the same id while the build is broken"
    );

    std::fs::write(&page, source).unwrap();
    let want = format!("listening on 127.0.0.1:{}", dev.port);
    assert!(wait_line(&dev, &want, limit), "no {want:?} after the fix");
    let c = events_id(dev.port);
    assert_ne!(c, b, "the app server has another id after the fix");
}

/// The Node 22 script that plays the browser, killed when dropped; `lines`
/// are what it prints.
struct Browser {
    child: Child,
    lines: Receiver<String>,
}

impl Drop for Browser {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Is `node` there, and version 22 or newer?
fn node_22() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .ok()
        .and_then(|out| {
            let version = String::from_utf8_lossy(&out.stdout).into_owned();
            version
                .trim()
                .strip_prefix('v')?
                .split('.')
                .next()?
                .parse::<u32>()
                .ok()
        })
        .is_some_and(|major| major >= 22)
}

#[test]
fn dev_reload_node() {
    if !node_22() {
        println!("skipped: node not found");
        return;
    }
    let app = Path::new(env!("CARGO_TARGET_TMPDIR")).join("framework-dev-node");
    let _ = std::fs::remove_dir_all(&app);
    copy_dir(&root().join("framework/examples/pages"), &app);
    let dev = start_dev(&app, free_port());
    let limit = Duration::from_secs(120);
    let mut child = Command::new("node")
        .arg(root().join("tests/framework/dev_reload.mjs"))
        .arg(format!("http://127.0.0.1:{}/", dev.port))
        .args(["2", "120"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let stdout = child.stdout.take().unwrap();
    let (tx, lines) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                return;
            }
        }
    });
    let mut browser = Browser { child, lines };
    assert!(
        wait_recv(&browser.lines, "connected", limit),
        "the script did not connect to the app server"
    );

    let page = app.join("app/about/page.aye");
    let source = std::fs::read_to_string(&page).unwrap();
    std::fs::write(&page, source.replace("raw(\"<h1>About</h1>\")", "raw(5)")).unwrap();
    assert!(
        wait_line(&dev, "serving the error page", limit),
        "no error page line"
    );
    assert!(
        wait_recv(&browser.lines, "reloads=1", limit),
        "no reload to the error page"
    );
    assert!(
        wait_recv(&browser.lines, "connected", limit),
        "the script did not connect to the error server"
    );

    std::fs::write(&page, source).unwrap();
    let want = format!("listening on 127.0.0.1:{}", dev.port);
    assert!(wait_line(&dev, &want, limit), "no {want:?} after the fix");
    assert!(
        wait_recv(&browser.lines, "reloads=2", limit),
        "no reload to the fixed page"
    );
    let status = browser.child.wait().unwrap();
    assert!(status.success(), "the script ended with {status}");
}

#[test]
fn web_units() {
    let out = Command::new(env!("CARGO_BIN_EXE_aye"))
        .current_dir(root())
        .arg("test")
        .arg(root().join("framework").join("examples").join("hello"))
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success() && stdout.contains("; 0 failed;"),
        "aye test failed:\n{stdout}{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn dev_units() {
    let out = Command::new(env!("CARGO_BIN_EXE_aye"))
        .current_dir(root())
        .arg("test")
        .arg(root().join("framework").join("dev"))
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success() && stdout.contains("; 0 failed;"),
        "aye test failed:\n{stdout}{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn island_build() {
    let target = driver::find_target("wasm32-freestanding").unwrap();
    if !target.toolchain_available() || !driver::node_available() {
        println!("skipped: no wasm32 clang or no node");
        return;
    }
    let wasm = Path::new(env!("CARGO_TARGET_TMPDIR")).join("framework-counter.wasm");
    let out = Command::new(env!("CARGO_BIN_EXE_aye"))
        .current_dir(root())
        .args(["build", "--target", "wasm32-freestanding"])
        .arg(root().join("framework/examples/pages/islands/counter"))
        .arg("-o")
        .arg(&wasm)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "building the counter island failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = Command::new("node")
        .arg(root().join("tests/framework/island_exports.mjs"))
        .arg(&wasm)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "node failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let lines = String::from_utf8_lossy(&out.stdout).into_owned();
    let imports: Vec<&str> = lines.lines().filter(|l| l.starts_with("import ")).collect();
    assert_eq!(
        imports,
        [
            "import env.aye_abort",
            "import env.aye_write",
            "import env.dom_on",
            "import env.dom_query",
            "import env.dom_set_text",
            "import env.strtod",
        ]
    );
    for export in ["mount", "event", "memory", "malloc"] {
        assert!(
            lines.lines().any(|l| l == format!("export {export}")),
            "no export {export}:\n{lines}"
        );
    }
}

#[test]
fn islands_glue() {
    let target = driver::find_target("wasm32-freestanding").unwrap();
    if !target.toolchain_available() || !driver::node_available() || !node_22() {
        println!("skipped: no wasm32 clang or no node 22");
        return;
    }
    let wasm = Path::new(env!("CARGO_TARGET_TMPDIR")).join("framework-glue-counter.wasm");
    let out = Command::new(env!("CARGO_BIN_EXE_aye"))
        .current_dir(root())
        .args(["build", "--target", "wasm32-freestanding"])
        .arg(root().join("framework/examples/pages/islands/counter"))
        .arg("-o")
        .arg(&wasm)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "building the counter island failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = Command::new("node")
        .arg(root().join("tests/framework/islands_glue.mjs"))
        .arg(root().join("framework/web/islands.aye"))
        .arg(&wasm)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "the glue test failed:\n{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "Count: 5");
}

/// Can an island be built and played: the wasm32 toolchain, `node` and
/// Node 22?
fn island_tools() -> bool {
    driver::find_target("wasm32-freestanding")
        .unwrap()
        .toolchain_available()
        && driver::node_available()
        && node_22()
}

/// Runs `island_e2e.mjs` against the server on `port`: the last line it
/// prints, `text=<count>`.
fn island_text(port: u16) -> String {
    let out = Command::new("node")
        .arg(root().join("tests/framework/island_e2e.mjs"))
        .arg(format!("http://127.0.0.1:{port}/"))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "the island script failed:\n{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .last()
        .unwrap_or_default()
        .to_string()
}

#[test]
fn island_e2e() {
    if !island_tools() {
        println!("skipped: no wasm32 clang or no node 22");
        return;
    }
    let app = Path::new(env!("CARGO_TARGET_TMPDIR")).join("framework-island-e2e-app");
    let _ = std::fs::remove_dir_all(&app);
    copy_dir(&root().join("framework/examples/pages"), &app);
    run_gen(&app, &[]);
    let web = app.join(".web");
    assert!(web.join("client/counter.wasm").is_file());
    let server = start(&build_dir("island-e2e", &web));
    assert_eq!(island_text(server.port), "text=Count: 5");
}

#[test]
fn dev_island() {
    if !island_tools() {
        println!("skipped: no wasm32 clang or no node 22");
        return;
    }
    let app = Path::new(env!("CARGO_TARGET_TMPDIR")).join("framework-dev-island");
    let _ = std::fs::remove_dir_all(&app);
    copy_dir(&root().join("framework/examples/pages"), &app);
    let dev = start_dev(&app, free_port());
    let limit = Duration::from_secs(120);
    let page = try_get(dev.port, "/counter").unwrap();
    let dev_tag = page.find("/_web/dev.js").expect("no dev tag in /counter");
    let islands_tag = page
        .find("/_web/islands.js")
        .expect("no islands tag in /counter");
    assert!(dev_tag < islands_tag, "the dev tag comes first:\n{page}");
    assert_eq!(island_text(dev.port), "text=Count: 5");

    let view = app.join("islands/counter/view/view.aye");
    let source = std::fs::read_to_string(&view).unwrap();
    assert!(source.contains("try b.append(\"Count: \")"));
    std::fs::write(
        &view,
        source.replace("try b.append(\"Count: \")", "try b.append(\"Total: \")"),
    )
    .unwrap();
    let want = format!("listening on 127.0.0.1:{}", dev.port);
    assert!(wait_line(&dev, &want, limit), "no {want:?} after the edit");
    assert_eq!(island_text(dev.port), "text=Total: 5");
}

/// The hex SHA-256 of the file at `path`, from `shasum` (macOS) or
/// `sha256sum`.
fn sha256_hex(path: &Path) -> String {
    let out = Command::new("shasum")
        .args(["-a", "256"])
        .arg(path)
        .output()
        .or_else(|_| Command::new("sha256sum").arg(path).output())
        .unwrap();
    assert!(out.status.success(), "no shasum or sha256sum");
    String::from_utf8_lossy(&out.stdout)
        .split_whitespace()
        .next()
        .unwrap()
        .to_string()
}

/// Runs `web gen` on `app` with the `extra` arguments.
fn run_gen(app: &Path, extra: &[&str]) {
    let out = Command::new(env!("CARGO_BIN_EXE_aye"))
        .current_dir(root())
        .env("AYE", env!("CARGO_BIN_EXE_aye"))
        .arg("run")
        .arg(root().join("framework").join("gen"))
        .arg("--")
        .arg(app)
        .arg("--web")
        .arg(root().join("framework").join("web"))
        .args(extra)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "web gen failed:\n{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn gen_prod() {
    let app = Path::new(env!("CARGO_TARGET_TMPDIR")).join("framework-gen-prod-app");
    let _ = std::fs::remove_dir_all(&app);
    copy_dir(&root().join("framework/examples/pages"), &app);
    std::fs::write(app.join("public/big.bin"), vec![7u8; 2 << 20]).unwrap();
    std::fs::write(app.join("public/.hidden"), "x").unwrap();
    std::os::unix::fs::symlink("style.css", app.join("public/link.css")).unwrap();
    let generate = |extra: &[&str]| run_gen(&app, extra);
    let web = app.join(".web");
    // Without `--prod` nothing is embedded.
    generate(&[]);
    assert!(!web.join("assets.aye").exists());
    generate(&["--prod"]);
    let source = std::fs::read_to_string(web.join("assets.aye")).unwrap();
    let islands = web.join("client/counter.wasm").is_file();
    let count = 3 + usize::from(islands);
    assert_eq!(source.matches("@embedFile").count(), count, "{source}");
    assert!(source.contains(&format!("pub let table = [{count}]Asset{{")));
    for (stem, file) in [
        ("/style", "style.css"),
        ("/img/logo", "img/logo.svg"),
        ("/big", "big.bin"),
    ] {
        let ext = file.rsplit_once('.').unwrap().1;
        let sha = sha256_hex(&app.join("public").join(file));
        let want = format!("path: \"/{file}\", hashed: \"{stem}.{}.{ext}\"", &sha[..8]);
        assert!(source.contains(&want), "no {want} in:\n{source}");
        assert!(
            source.contains(&format!("etag: \"\\\"{sha}\\\"\"")),
            "no etag of {file}"
        );
        assert!(
            source.contains(&format!("@embedFile(\"../public/{file}\")")),
            "no embed of {file}"
        );
    }
    assert!(source.contains("contentType: \"text/css; charset=utf-8\""));
    assert!(source.contains("contentType: \"image/svg+xml\""));
    assert!(!source.contains("link.css") && !source.contains(".hidden"));
    if islands {
        let sha = sha256_hex(&web.join("client/counter.wasm"));
        let want = format!(
            "path: \"/_web/islands/counter.wasm\", hashed: \"/_web/islands/counter.wasm\", contentType: \"application/wasm\", etag: \"\\\"{sha}\\\"\", bytes: @embedFile(\"client/counter.wasm\")"
        );
        assert!(source.contains(&want), "no {want} in:\n{source}");
    }
    assert!(source.contains("import web.Asset"));
    let check = Command::new(env!("CARGO_BIN_EXE_aye"))
        .current_dir(root())
        .arg("check")
        .arg(&web)
        .output()
        .unwrap();
    assert!(
        check.status.success(),
        "aye check .web failed:\n{}{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
    build_dir("gen-prod", &web);
}

#[test]
fn prod_serve() {
    let app = Path::new(env!("CARGO_TARGET_TMPDIR")).join("framework-prod-serve-app");
    let gone = Path::new(env!("CARGO_TARGET_TMPDIR")).join("framework-prod-serve-gone");
    let from = Path::new(env!("CARGO_TARGET_TMPDIR")).join("framework-prod-serve-cwd");
    for dir in [&app, &gone, &from] {
        let _ = std::fs::remove_dir_all(dir);
    }
    std::fs::create_dir_all(&from).unwrap();
    copy_dir(&root().join("framework/examples/pages"), &app);
    run_gen(&app, &["--prod"]);
    let web = app.join(".web");
    let main = std::fs::read_to_string(web.join("main.aye")).unwrap();
    assert!(main.contains("web.runProd("), "{main}");
    let exe = build_dir("prod-serve", &web);
    let css = std::fs::read_to_string(app.join("public/style.css")).unwrap();
    let svg = std::fs::read_to_string(app.join("public/img/logo.svg")).unwrap();
    let sha = sha256_hex(&app.join("public/style.css"));
    let etag = format!("\"{sha}\"");
    let islands = web.join("client/counter.wasm").is_file();
    // Nothing of the app is on disk any more, and the server runs somewhere else.
    std::fs::rename(&app, &gone).unwrap();
    let server = start_in(&exe, Some(&from), &["--dev"]);

    let split = |response: &str| {
        let (head, body) = response.split_once("\n\n").unwrap();
        (head.to_string(), body.to_string())
    };
    let plain = request(&server, "GET", "/style.css");
    let (head, body) = split(&plain);
    assert!(head.starts_with("HTTP/1.1 200 OK\n"), "{head}");
    assert_eq!(
        header_value(&plain, "Content-Type"),
        "text/css; charset=utf-8"
    );
    assert_eq!(header_value(&plain, "Cache-Control"), "public, max-age=0");
    assert_eq!(header_value(&plain, "ETag"), etag);
    assert!(!head.contains("Last-Modified"), "{head}");
    assert_eq!(body, css);

    // The page links the hashed URL, which is immutable.
    let about = request(&server, "GET", "/about");
    assert!(about.starts_with("HTTP/1.1 200 OK\n"), "{about}");
    let hashed = format!("/style.{}.css", &sha[..8]);
    assert!(
        about.contains(&format!("<link rel=\"stylesheet\" href=\"{hashed}\">")),
        "{about}"
    );
    assert!(!about.contains("/_web/dev.js"), "--dev is ignored: {about}");
    let long = request(&server, "GET", &hashed);
    let (head, body) = split(&long);
    assert!(head.starts_with("HTTP/1.1 200 OK\n"), "{head}");
    assert_eq!(
        header_value(&long, "Cache-Control"),
        "public, max-age=31536000, immutable"
    );
    assert_eq!(header_value(&long, "ETag"), etag);
    assert_eq!(body, css);

    // A matching `If-None-Match` gives 304 and no body, for both URLs.
    for path in ["/style.css", hashed.as_str()] {
        let extra = format!("If-None-Match: {etag}\r\n");
        let cached = request_with(&server, "GET", path, &extra, "");
        let (head, body) = split(&cached);
        assert!(head.starts_with("HTTP/1.1 304 "), "{path}: {head}");
        assert_eq!(header_value(&cached, "ETag"), etag, "{path}");
        assert!(body.is_empty(), "{path}: {body:?}");
        let other = request_with(&server, "GET", path, "If-None-Match: \"x\"\r\n", "");
        assert!(other.starts_with("HTTP/1.1 200 OK\n"), "{path}: {other}");
    }
    let headed = request(&server, "HEAD", &hashed);
    let (head, body) = split(&headed);
    assert!(head.starts_with("HTTP/1.1 200 OK\n"), "{head}");
    assert_eq!(
        header_value(&headed, "Content-Length"),
        css.len().to_string()
    );
    assert!(body.is_empty(), "{body:?}");

    let logo = request(&server, "GET", "/img/logo.svg");
    let (head, body) = split(&logo);
    assert!(head.starts_with("HTTP/1.1 200 OK\n"), "{head}");
    assert_eq!(header_value(&logo, "Content-Type"), "image/svg+xml");
    assert_eq!(body, svg);

    for path in [
        "/nope.css",
        "/../Cargo.toml",
        "/_web/dev.js",
        "/_web/events",
        "/_web/islands/nope.wasm",
    ] {
        let response = request(&server, "GET", path);
        assert!(
            response.starts_with("HTTP/1.1 404 Not Found\n"),
            "{path}: {response}"
        );
    }
    let script = request(&server, "GET", "/_web/islands.js");
    assert!(script.starts_with("HTTP/1.1 200 OK\n"), "{script}");
    if islands {
        let wasm = request_raw(&server, "GET", "/_web/islands/counter.wasm", "", "");
        let (head, body) = split_binary(&wasm);
        assert!(head.starts_with("HTTP/1.1 200 OK\n"), "{head}");
        assert!(head.contains("Content-Type: application/wasm\n"), "{head}");
        assert!(
            head.contains("Cache-Control: public, max-age=0\n"),
            "{head}"
        );
        assert!(body.starts_with(b"\0asm"), "{body:?}");
    } else {
        println!("skipped: no wasm32 clang; the island was not embedded");
    }
}

/// A method, a path and the file of `tests/framework/posts/` with its
/// response, for the posts app.
const POSTS_CASES: &[(&str, &str, &str)] = &[
    ("GET", "/", "root.http"),
    ("GET", "/posts/hello", "post_hello.http"),
    ("GET", "/posts/less-is-more", "post_less_is_more.http"),
    ("GET", "/posts/on-files", "post_on_files.http"),
    ("GET", "/posts/nope", "post_missing.http"),
    ("GET", "/nope", "missing.http"),
];

/// Builds the posts app in its own directory `name` under the test
/// directory and starts it on the file `data/posts.json`.
fn posts_server(name: &str) -> App {
    let app = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&app);
    copy_dir(&root().join("framework/examples/posts"), &app);
    run_gen(&app, &[]);
    let web = app.join(".web");
    let check = Command::new(env!("CARGO_BIN_EXE_aye"))
        .current_dir(root())
        .arg("check")
        .arg(&web)
        .output()
        .unwrap();
    assert!(
        check.status.success(),
        "aye check .web failed:\n{}{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
    let exe = build_dir("posts", &web);
    let data = app.join("data/posts.json");
    start_with(&exe, &["--posts", data.to_str().unwrap()])
}

#[test]
fn posts_pages() {
    let server = posts_server("framework-posts-app");
    for (method, path, file) in POSTS_CASES {
        let want =
            std::fs::read_to_string(root().join("tests/framework/posts").join(file)).unwrap();
        assert_eq!(
            scrub(&request(&server, method, path)),
            want,
            "{method} {path}"
        );
    }
}

const FORM: &str = "Content-Type: application/x-www-form-urlencoded\r\n";
const JSON: &str = "Content-Type: application/json\r\n";

/// Comments on the posts app: the form, the post page with the cookie and the
/// JSON API run in order against one server, so each case sees the comments
/// of the earlier ones. Each is a method, a path, extra header lines, a body
/// and the file of `tests/framework/posts/` with the response.
#[test]
fn posts_comments() {
    let server = posts_server("framework-posts-comments-app");
    let long_body = format!("name=Ann&body={}", "a".repeat(2001));
    let cases: &[(&str, &str, &str, &str, &str)] = &[
        (
            "POST",
            "/posts/hello",
            FORM,
            "name=Ann+B&body=%3Cb%3Ehi%3C%2Fb%3E%21",
            "comment_post.http",
        ),
        (
            "GET",
            "/posts/hello",
            "Cookie: name=Ann%20B\r\n",
            "",
            "comment_page.http",
        ),
        (
            "POST",
            "/posts/hello",
            FORM,
            "name=Ann&body=",
            "comment_empty.http",
        ),
        (
            "POST",
            "/posts/hello",
            FORM,
            "name=Ann",
            "comment_missing.http",
        ),
        (
            "POST",
            "/posts/hello",
            FORM,
            &long_body,
            "comment_long.http",
        ),
        (
            "POST",
            "/posts/nope",
            FORM,
            "name=Ann&body=hi",
            "comment_nope.http",
        ),
        ("GET", "/api/posts", "", "", "api_posts.http"),
        (
            "POST",
            "/api/posts/hello/comments",
            JSON,
            "{\"name\":\"Bo\",\"body\":\"yo\"}",
            "api_comment_post.http",
        ),
        (
            "GET",
            "/api/posts/hello/comments",
            "",
            "",
            "api_comments.http",
        ),
        (
            "POST",
            "/api/posts/nope/comments",
            JSON,
            "{\"name\":\"Bo\",\"body\":\"yo\"}",
            "api_comment_nope.http",
        ),
        (
            "POST",
            "/api/posts/hello/comments",
            JSON,
            "{\"name\":",
            "api_comment_bad.http",
        ),
        (
            "POST",
            "/api/posts/hello/comments",
            JSON,
            "{\"name\":\"Bo\"}",
            "api_comment_field.http",
        ),
    ];
    for (method, path, extra, body, file) in cases {
        let want =
            std::fs::read_to_string(root().join("tests/framework/posts").join(file)).unwrap();
        assert_eq!(
            scrub(&request_with(&server, method, path, extra, body)),
            want,
            "{method} {path}"
        );
    }
}
