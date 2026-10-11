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

const NAV: &str = "import std.text\nimport util\n\n/// Add one.\n/// Second line.\nfn inc(n: i32) i32 {\n    return n + 1\n}\n\npub fn main() {\n    let a = inc(1)\n    let b = util.twice(a)\n    @println(text.eql(\"a\", \"a\"))\n}\n";
const UTIL: &str = "/// Double `n`.\npub fn twice(n: i32) i32 {\n    return n * 2\n}\n";

/// `method` (hover or definition) at the 0-based `line` and `character`:
/// the result.
fn at(c: &mut Client, id: i64, method: &str, uri: &str, line: u32, character: u32) -> Json {
    let params = format!(
        r#"{{"textDocument":{{"uri":{}}},"position":{{"line":{line},"character":{character}}}}}"#,
        aye::diag::json_str(uri)
    );
    let reply = c.request(id, &format!("textDocument/{method}"), &params);
    reply.get("result").unwrap().clone()
}

fn hover_text(result: &Json) -> &str {
    result
        .get("contents")
        .and_then(|c| c.get("value"))
        .and_then(Json::as_str)
        .unwrap_or_else(|| panic!("no hover: {result:?}"))
}

#[test]
fn hover_and_definition() {
    let (dir, uri) = scratch("nav", NAV);
    std::fs::create_dir_all(dir.join("util")).unwrap();
    std::fs::write(dir.join("util/util.aye"), UTIL).unwrap();
    let mut c = Client::start();
    let reply = c.request(
        1,
        "initialize",
        r#"{"processId":null,"rootUri":null,"capabilities":{}}"#,
    );
    let caps = reply.get("result").unwrap().get("capabilities").unwrap();
    assert_eq!(caps.get("hoverProvider"), Some(&Json::Bool(true)));
    assert_eq!(caps.get("definitionProvider"), Some(&Json::Bool(true)));
    c.notify("initialized", "{}");
    c.notify("textDocument/didOpen", &open(&uri, NAV));
    c.diagnostics();

    // A local function: its signature and `///` text.
    let h = at(&mut c, 2, "hover", &uri, 10, 13);
    assert_eq!(
        hover_text(&h),
        "```aye\nfn inc(n: i32) i32\n```\n\nAdd one.\nSecond line."
    );
    assert_eq!(
        h.get("range").unwrap().write(),
        r#"{"start":{"line":10,"character":12},"end":{"line":10,"character":15}}"#
    );
    let d = at(&mut c, 3, "definition", &uri, 10, 13);
    assert_eq!(d.get("uri").and_then(Json::as_str), Some(uri.as_str()));
    assert_eq!(
        d.get("range").unwrap().write(),
        r#"{"start":{"line":5,"character":3},"end":{"line":5,"character":6}}"#
    );

    // Locals: `let a` and the parameter `n`.
    let h = at(&mut c, 4, "hover", &uri, 11, 23);
    assert_eq!(hover_text(&h), "```aye\nlet a = inc(1)\n```");
    let h = at(&mut c, 5, "hover", &uri, 6, 11);
    assert_eq!(hover_text(&h), "```aye\nn: i32\n```");
    let d = at(&mut c, 6, "definition", &uri, 11, 23);
    assert_eq!(
        d.get("range").unwrap().write(),
        r#"{"start":{"line":10,"character":8},"end":{"line":10,"character":9}}"#
    );

    // A function of an imported module of the program.
    let h = at(&mut c, 7, "hover", &uri, 11, 18);
    assert_eq!(
        hover_text(&h),
        "```aye\npub fn twice(n: i32) i32\n```\n\nDouble `n`."
    );
    let d = at(&mut c, 8, "definition", &uri, 11, 18);
    let util_uri = format!("file://{}", dir.join("util/util.aye").display());
    assert_eq!(d.get("uri").and_then(Json::as_str), Some(util_uri.as_str()));
    assert_eq!(
        d.get("range").unwrap().write(),
        r#"{"start":{"line":1,"character":7},"end":{"line":1,"character":12}}"#
    );

    // A function of std.
    let h = at(&mut c, 9, "hover", &uri, 12, 18);
    let text = hover_text(&h);
    assert!(
        text.starts_with("```aye\npub fn eql(a: []u8, b: []u8) bool\n```\n\nDo `a` and `b`"),
        "{text}"
    );
    let d = at(&mut c, 10, "definition", &uri, 12, 18);
    let std_file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("std/text/text.aye");
    assert_eq!(
        d.get("uri").and_then(Json::as_str),
        Some(format!("file://{}", std_file.display()).as_str())
    );
    let src = std::fs::read_to_string(&std_file).unwrap();
    let line = src
        .lines()
        .position(|l| l.starts_with("pub fn eql("))
        .unwrap();
    assert_eq!(
        d.get("range").unwrap().write(),
        format!(
            r#"{{"start":{{"line":{line},"character":7}},"end":{{"line":{line},"character":10}}}}"#
        )
    );

    // A module: the hover names it, the definition is its first file.
    let h = at(&mut c, 11, "hover", &uri, 12, 14);
    assert_eq!(hover_text(&h), "```aye\nimport std.text\n```");
    let d = at(&mut c, 12, "definition", &uri, 11, 13);
    assert_eq!(d.get("uri").and_then(Json::as_str), Some(util_uri.as_str()));

    // White space, a keyword and a name that is not declared: null.
    for (id, line, character) in [(13, 8, 0), (14, 10, 1), (15, 9, 1), (16, 12, 5)] {
        for method in ["hover", "definition"] {
            let id = id * 2 + (method == "hover") as i64;
            let r = at(&mut c, id, method, &uri, line, character);
            assert_eq!(r, Json::Null, "{method} at {line}:{character}");
        }
    }
    // Past the end of the document.
    assert_eq!(at(&mut c, 100, "hover", &uri, 50, 0), Json::Null);

    c.finish();
    let _ = std::fs::remove_dir_all(dir);
}
