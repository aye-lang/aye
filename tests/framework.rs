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
//!   with `AYE` set to the `aye` under test.
//! - `framework/dev` (`web dev`) is run with `aye run` on a copy of the pages
//!   example, in its own process group: it generates, checks and builds the
//!   app, starts the server and relays its `listening on` line. Killing the
//!   group must leave no server behind.
//! - The `test` blocks of `framework/web` run with `aye test`.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

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
];

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
    let want = format!("listening on 127.0.0.1:{port}");
    let mut dev = Dev { child, port };
    let mut lines = BufReader::new(dev.child.stdout.take().unwrap());
    loop {
        let mut line = String::new();
        let n = lines.read_line(&mut line).unwrap();
        assert!(n > 0, "web dev ended before it printed {want:?}");
        if line.trim() == want {
            break;
        }
    }
    // Keep reading so that the CLI never blocks on a full pipe.
    std::thread::spawn(move || {
        let _ = std::io::copy(&mut lines, &mut std::io::sink());
    });
    dev
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
    let mut child = Command::new(exe)
        .args(["--port", "0"])
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
    String::from_utf8(buf).unwrap().replace("\r\n", "\n")
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
    assert_eq!(request(&server, "GET", "/about"), want);
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
