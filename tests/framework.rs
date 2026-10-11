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
//!   are the responses to the requests in `PAGES_CASES`.
//! - The `test` blocks of `framework/web` run with `aye test`.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
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
    ("GET", "/nope", "missing.http"),
];

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
    let mut s = TcpStream::connect(("127.0.0.1", app.port)).unwrap();
    s.set_read_timeout(Some(std::time::Duration::from_secs(10)))
        .unwrap();
    write!(
        s,
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut buf = Vec::new();
    s.read_to_end(&mut buf).unwrap();
    String::from_utf8(buf).unwrap().replace("\r\n", "\n")
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
        assert_eq!(request(&app, method, path), want, "{method} {path}");
    }
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
