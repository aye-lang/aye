//! `aye lsp` (O6.1): a client that speaks JSON-RPC with `Content-Length`
//! framing to the server on its stdin and stdout.

use aye::lsp::Json;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

struct Client {
    child: Child,
    stdin: ChildStdin,
    messages: Receiver<Json>,
}

impl Client {
    fn start() -> Client {
        let mut child = Command::new(env!("CARGO_BIN_EXE_aye"))
            .arg("lsp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let mut out = BufReader::new(child.stdout.take().unwrap());
        let (tx, messages) = channel();
        std::thread::spawn(move || {
            loop {
                let mut len = 0;
                loop {
                    let mut line = String::new();
                    if out.read_line(&mut line).unwrap() == 0 {
                        return;
                    }
                    let line = line.trim_end();
                    if line.is_empty() {
                        break;
                    }
                    if let Some(n) = line.strip_prefix("Content-Length: ") {
                        len = n.parse().unwrap();
                    }
                }
                let mut body = vec![0u8; len];
                out.read_exact(&mut body).unwrap();
                let text = String::from_utf8(body).unwrap();
                if tx.send(Json::parse(&text).unwrap()).is_err() {
                    return;
                }
            }
        });
        Client {
            child,
            stdin,
            messages,
        }
    }

    fn send(&mut self, body: &str) {
        write!(self.stdin, "Content-Length: {}\r\n\r\n{body}", body.len()).unwrap();
        self.stdin.flush().unwrap();
    }

    fn recv(&mut self) -> Json {
        self.messages
            .recv_timeout(Duration::from_secs(60))
            .expect("no message from `aye lsp`")
    }

    fn request(&mut self, id: i64, method: &str, params: &str) -> Json {
        self.send(&format!(
            r#"{{"jsonrpc":"2.0","id":{id},"method":"{method}","params":{params}}}"#
        ));
        let reply = self.recv();
        assert_eq!(
            reply.get("id").and_then(Json::as_i64),
            Some(id),
            "{reply:?}"
        );
        reply
    }

    fn notify(&mut self, method: &str, params: &str) {
        self.send(&format!(
            r#"{{"jsonrpc":"2.0","method":"{method}","params":{params}}}"#
        ));
    }

    /// The next message, which must be `publishDiagnostics`: its params.
    fn diagnostics(&mut self) -> Json {
        let msg = self.recv();
        assert_eq!(
            msg.get("method").and_then(Json::as_str),
            Some("textDocument/publishDiagnostics"),
            "{msg:?}"
        );
        msg.get("params").unwrap().clone()
    }

    fn initialize(&mut self) {
        let reply = self.request(
            1,
            "initialize",
            r#"{"processId":null,"rootUri":null,"capabilities":{}}"#,
        );
        let caps = reply.get("result").unwrap().get("capabilities").unwrap();
        assert_eq!(
            caps.get("documentFormattingProvider"),
            Some(&Json::Bool(true))
        );
        let sync = caps.get("textDocumentSync").unwrap();
        assert_eq!(sync.get("change").and_then(Json::as_i64), Some(1));
        self.notify("initialized", "{}");
    }

    fn finish(mut self) {
        let reply = self.request(99, "shutdown", "null");
        assert_eq!(reply.get("result"), Some(&Json::Null));
        self.notify("exit", "null");
        let status = self.child.wait().unwrap();
        assert!(status.success(), "{status:?}");
    }
}

/// A scratch directory with `main.aye` in it; its `file:` URI.
fn scratch(name: &str, src: &str) -> (PathBuf, String) {
    let dir = std::env::temp_dir().join(format!("aye-lsp-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("main.aye");
    std::fs::write(&file, src).unwrap();
    let uri = format!("file://{}", file.display());
    (dir, uri)
}

fn open(uri: &str, src: &str) -> String {
    format!(
        r#"{{"textDocument":{{"uri":{},"languageId":"aye","version":1,"text":{}}}}}"#,
        aye::diag::json_str(uri),
        aye::diag::json_str(src)
    )
}

/// An unformatted program that uses an undefined variable (E0018 at 3:14).
const BAD: &str = "pub fn main() {\n    let x   =   1\n    @println(y)\n}\n";
const GOOD: &str = "pub fn main() {\n    let x = 1\n    @println(x)\n}\n";

#[test]
fn diagnostics_and_formatting() {
    let (dir, uri) = scratch("diag", BAD);
    let mut c = Client::start();
    c.initialize();
    c.notify("textDocument/didOpen", &open(&uri, BAD));
    let p = c.diagnostics();
    assert_eq!(p.get("uri").and_then(Json::as_str), Some(uri.as_str()));
    let list = p.get("diagnostics").unwrap().as_array().unwrap();
    assert_eq!(list.len(), 1, "{list:?}");
    let d = &list[0];
    assert_eq!(d.get("code").and_then(Json::as_str), Some("E0018"));
    assert_eq!(d.get("severity").and_then(Json::as_i64), Some(1));
    assert_eq!(d.get("source").and_then(Json::as_str), Some("aye"));
    assert_eq!(
        d.get("range").unwrap().write(),
        r#"{"start":{"line":2,"character":13},"end":{"line":2,"character":14}}"#
    );
    assert!(
        d.get("message")
            .and_then(Json::as_str)
            .unwrap()
            .contains("`y`")
    );

    let params = format!(
        r#"{{"textDocument":{{"uri":{}}},"options":{{"tabSize":4,"insertSpaces":true}}}}"#,
        aye::diag::json_str(&uri)
    );
    let reply = c.request(2, "textDocument/formatting", &params);
    let edits = reply.get("result").unwrap().as_array().unwrap();
    assert_eq!(edits.len(), 1, "{edits:?}");
    assert_eq!(
        edits[0].get("range").unwrap().write(),
        r#"{"start":{"line":0,"character":0},"end":{"line":4,"character":0}}"#
    );
    let new_text = edits[0].get("newText").and_then(Json::as_str).unwrap();
    assert_eq!(new_text, aye::fmt::format(BAD).unwrap());
    assert_eq!(
        new_text,
        "pub fn main() {\n    let x = 1\n    @println(y)\n}\n"
    );

    c.finish();
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn change_fixes_the_error() {
    let (dir, uri) = scratch("change", BAD);
    let mut c = Client::start();
    c.initialize();
    c.notify("textDocument/didOpen", &open(&uri, BAD));
    assert_eq!(
        c.diagnostics()
            .get("diagnostics")
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let change = format!(
        r#"{{"textDocument":{{"uri":{},"version":2}},"contentChanges":[{{"text":{}}}]}}"#,
        aye::diag::json_str(&uri),
        aye::diag::json_str(GOOD)
    );
    c.notify("textDocument/didChange", &change);
    let p = c.diagnostics();
    assert_eq!(p.get("version").and_then(Json::as_i64), Some(2));
    assert_eq!(p.get("diagnostics"), Some(&Json::Arr(Vec::new())));

    // Formatted text: no edit.
    let params = format!(
        r#"{{"textDocument":{{"uri":{}}}}}"#,
        aye::diag::json_str(&uri)
    );
    let reply = c.request(2, "textDocument/formatting", &params);
    assert_eq!(reply.get("result"), Some(&Json::Arr(Vec::new())));

    // Text that does not lex: no edit, and the diagnostics stay.
    let broken = "pub fn main() {\n    let s = \"abc\n}\n";
    let change = format!(
        r#"{{"textDocument":{{"uri":{},"version":3}},"contentChanges":[{{"text":{}}}]}}"#,
        aye::diag::json_str(&uri),
        aye::diag::json_str(broken)
    );
    c.notify("textDocument/didChange", &change);
    assert!(
        !c.diagnostics()
            .get("diagnostics")
            .unwrap()
            .as_array()
            .unwrap()
            .is_empty()
    );
    let reply = c.request(3, "textDocument/formatting", &params);
    assert_eq!(reply.get("result"), Some(&Json::Arr(Vec::new())));

    // Closing clears the diagnostics.
    c.notify("textDocument/didClose", &params);
    assert_eq!(
        c.diagnostics().get("diagnostics"),
        Some(&Json::Arr(Vec::new()))
    );

    c.finish();
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn exit_without_shutdown_fails() {
    let mut c = Client::start();
    c.initialize();
    c.notify("exit", "null");
    assert_eq!(c.child.wait().unwrap().code(), Some(1));
}
