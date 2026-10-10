//! Tests of the web framework (`framework/`, written in Aye).
//!
//! - `framework/examples/hello` is built and started with `--port 0`; it
//!   prints `listening on 127.0.0.1:<port>`.
//! - `tests/framework/hello/X.http` is the response the server must send to
//!   a `GET` of the path in `HELLO_CASES` (with `Connection: close`), with
//!   line endings read as `\n`.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// A path and the file of `tests/framework/hello/` with its response.
const HELLO_CASES: &[(&str, &str)] = &[("/", "root.http"), ("/nope", "missing.http")];

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
    let exe = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("framework-{name}"));
    let out = Command::new(env!("CARGO_BIN_EXE_aye"))
        .current_dir(root())
        .arg("build")
        .arg(root().join("framework").join("examples").join(name))
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

/// The response to `GET path`, read until the server closes, with `\r\n`
/// as `\n`.
fn get(app: &App, path: &str) -> String {
    let mut s = TcpStream::connect(("127.0.0.1", app.port)).unwrap();
    s.set_read_timeout(Some(std::time::Duration::from_secs(10)))
        .unwrap();
    write!(
        s,
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut buf = Vec::new();
    s.read_to_end(&mut buf).unwrap();
    String::from_utf8(buf).unwrap().replace("\r\n", "\n")
}

#[test]
fn hello() {
    let app = start(&build("hello"));
    for (path, file) in HELLO_CASES {
        let want =
            std::fs::read_to_string(root().join("tests/framework/hello").join(file)).unwrap();
        assert_eq!(get(&app, path), want, "GET {path}");
    }
}
