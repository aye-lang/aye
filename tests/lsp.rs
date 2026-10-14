//! `aye lsp` (O6.1): a client that speaks JSON-RPC with `Content-Length`
//! framing to the server on its stdin and stdout.

use aye::lsp::Json;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

struct Client {
    child: Child,
    stdin: ChildStdin,
    messages: Receiver<Json>,
}

impl Client {
    fn start() -> Client {
        Client::start_with(None)
    }

    /// With `cache` as `$AYE_CACHE_DIR`.
    fn start_with(cache: Option<&std::path::Path>) -> Client {
        match cache {
            Some(dir) => Client::start_env(&[("AYE_CACHE_DIR", dir)]),
            None => Client::start_env(&[]),
        }
    }

    /// With the environment variables `vars`.
    fn start_env(vars: &[(&str, &std::path::Path)]) -> Client {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_aye"));
        for (k, v) in vars {
            cmd.env(k, v);
        }
        let mut child = cmd
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
        self.params_of_diagnostics(msg)
    }

    /// `msg`, which must be `publishDiagnostics`: its params.
    fn params_of_diagnostics(&self, msg: Json) -> Json {
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
        assert!(
            caps.get("codeActionProvider")
                .is_some_and(|p| *p != Json::Null)
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

/// `f()` (4:18 to 4:21) has the wrong type: the compiler knows where the call
/// ends, which the token at the error (`f`) does not tell.
const MISMATCH: &str =
    "fn f() i32 {\n    return 1\n}\n\npub fn main() {\n    let n: i64 = f()\n    @println(n)\n}\n";

#[test]
fn range_ends_where_the_compiler_says() {
    let (dir, uri) = scratch("range", MISMATCH);
    let mut c = Client::start();
    c.initialize();
    c.notify("textDocument/didOpen", &open(&uri, MISMATCH));
    let p = c.diagnostics();
    let list = p.get("diagnostics").unwrap().as_array().unwrap();
    assert_eq!(list.len(), 1, "{list:?}");
    assert_eq!(list[0].get("code").and_then(Json::as_str), Some("E0023"));
    assert_eq!(list[0].get("severity").and_then(Json::as_i64), Some(1));
    assert_eq!(
        list[0].get("range").unwrap().write(),
        r#"{"start":{"line":5,"character":17},"end":{"line":5,"character":20}}"#
    );
    c.finish();
    let _ = std::fs::remove_dir_all(dir);
}

/// `sum` is assigned but declared with `let` (E0032 at 3:5, fix: `var` at 2:5).
const LET: &str = "pub fn main() {\n    let sum = 0\n    sum += 1\n    @println(sum)\n}\n";

#[test]
fn code_action_applies_the_fix() {
    let (dir, uri) = scratch("action", LET);
    let mut c = Client::start();
    c.initialize();
    c.notify("textDocument/didOpen", &open(&uri, LET));
    let p = c.diagnostics();
    let list = p.get("diagnostics").unwrap().as_array().unwrap();
    assert_eq!(list.len(), 1, "{list:?}");
    assert_eq!(list[0].get("code").and_then(Json::as_str), Some("E0032"));
    let range = list[0].get("range").unwrap().write();
    assert_eq!(
        range,
        r#"{"start":{"line":2,"character":4},"end":{"line":2,"character":7}}"#
    );

    let ask = |range: &str| {
        format!(
            r#"{{"textDocument":{{"uri":{}}},"range":{range},"context":{{"diagnostics":[]}}}}"#,
            aye::diag::json_str(&uri)
        )
    };
    let reply = c.request(2, "textDocument/codeAction", &ask(&range));
    let actions = reply.get("result").unwrap().as_array().unwrap();
    assert_eq!(actions.len(), 1, "{actions:?}");
    let a = &actions[0];
    assert_eq!(a.get("kind").and_then(Json::as_str), Some("quickfix"));
    assert_eq!(
        a.get("title").and_then(Json::as_str),
        Some("declare `sum` with `var`")
    );
    let attached = a.get("diagnostics").unwrap().as_array().unwrap();
    assert_eq!(
        attached[0].get("code").and_then(Json::as_str),
        Some("E0032")
    );
    assert_eq!(attached[0].get("severity").and_then(Json::as_i64), Some(1));
    let edits = a
        .get("edit")
        .and_then(|e| e.get("changes"))
        .and_then(|c| c.get(&uri))
        .and_then(Json::as_array)
        .unwrap();
    assert_eq!(edits.len(), 1, "{edits:?}");
    assert_eq!(
        edits[0].get("range").unwrap().write(),
        r#"{"start":{"line":1,"character":4},"end":{"line":1,"character":7}}"#
    );
    assert_eq!(edits[0].get("newText").and_then(Json::as_str), Some("var"));

    // Where the diagnostic is not: no action.
    let away = r#"{"start":{"line":0,"character":0},"end":{"line":0,"character":3}}"#;
    let reply = c.request(3, "textDocument/codeAction", &ask(away));
    assert_eq!(reply.get("result"), Some(&Json::Arr(Vec::new())));

    // The fixed text is clean.
    let fixed = LET.replacen("let sum", "var sum", 1);
    let change = format!(
        r#"{{"textDocument":{{"uri":{},"version":2}},"contentChanges":[{{"text":{}}}]}}"#,
        aye::diag::json_str(&uri),
        aye::diag::json_str(&fixed)
    );
    c.notify("textDocument/didChange", &change);
    assert_eq!(
        c.diagnostics().get("diagnostics"),
        Some(&Json::Arr(Vec::new()))
    );

    c.finish();
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn import_errors_are_related_information() {
    let main = "import lib\n\npub fn main() {\n    @println(lib.f())\n}\n";
    let (dir, uri) = scratch("import", main);
    std::fs::create_dir_all(dir.join("lib")).unwrap();
    std::fs::write(
        dir.join("lib/f.aye"),
        "pub fn f() i32 {\n    return missing\n}\n",
    )
    .unwrap();
    let mut c = Client::start();
    c.initialize();
    c.notify("textDocument/didOpen", &open(&uri, main));
    let p = c.diagnostics();
    let list = p.get("diagnostics").unwrap().as_array().unwrap();
    assert_eq!(list.len(), 1, "{list:?}");
    let d = &list[0];
    assert_eq!(d.get("severity").and_then(Json::as_i64), Some(1));
    assert_eq!(
        d.get("range").unwrap().write(),
        r#"{"start":{"line":0,"character":0},"end":{"line":0,"character":10}}"#
    );
    assert_eq!(
        d.get("message").and_then(Json::as_str),
        Some("module `lib` has 1 error")
    );
    let related = d.get("relatedInformation").unwrap().as_array().unwrap();
    assert_eq!(related.len(), 1, "{related:?}");
    let loc = related[0].get("location").unwrap();
    assert!(
        loc.get("uri")
            .and_then(Json::as_str)
            .unwrap()
            .ends_with("/lib/f.aye"),
        "{loc:?}"
    );
    assert_eq!(
        loc.get("range").unwrap().write(),
        r#"{"start":{"line":1,"character":11},"end":{"line":1,"character":18}}"#
    );
    assert!(
        related[0]
            .get("message")
            .and_then(Json::as_str)
            .unwrap()
            .starts_with("E0018: ")
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

const TYPED: &str = "/// A point.
struct Point {
    /// Across.
    x: i64
    y: i64

    /// The sum of the coordinates.
    pub fn len(self: *Point) i64 {
        return self.x + self.y
    }
}

struct Line {
    from: Point
    to: Point
}

pub fn main() {
    let p = Point { x: 1, y: 2 }
    let l = Line { from: p, to: p }
    @println(p.len())
    @println(l.to.x)
    if p.x > 0 {
        let inner = 1
        @println(inner)
    }
    @println(inner)
}
";

#[test]
fn hover_and_definition_through_types() {
    let (dir, uri) = scratch("typed", TYPED);
    let mut c = Client::start();
    c.initialize();
    c.notify("textDocument/didOpen", &open(&uri, TYPED));
    // `inner` after its block is an error; sema still resolves the rest.
    c.diagnostics();

    // A method through a local: `p.len()`.
    let h = at(&mut c, 2, "hover", &uri, 20, 16);
    assert_eq!(
        hover_text(&h),
        "```aye\npub fn len(self: *Point) i64\n```\n\nThe sum of the coordinates."
    );
    assert_eq!(
        h.get("range").unwrap().write(),
        r#"{"start":{"line":20,"character":15},"end":{"line":20,"character":18}}"#
    );
    let d = at(&mut c, 3, "definition", &uri, 20, 16);
    assert_eq!(d.get("uri").and_then(Json::as_str), Some(uri.as_str()));
    assert_eq!(
        d.get("range").unwrap().write(),
        r#"{"start":{"line":7,"character":11},"end":{"line":7,"character":14}}"#
    );
    // Just past the end of `len` is on it.
    let h = at(&mut c, 4, "hover", &uri, 20, 18);
    assert!(hover_text(&h).starts_with("```aye\npub fn len("), "{h:?}");

    // A field through a local: `p.x`.
    let h = at(&mut c, 5, "hover", &uri, 22, 9);
    assert_eq!(hover_text(&h), "```aye\nx: i64\n```\n\nAcross.");
    let d = at(&mut c, 6, "definition", &uri, 22, 9);
    assert_eq!(
        d.get("range").unwrap().write(),
        r#"{"start":{"line":3,"character":4},"end":{"line":3,"character":5}}"#
    );

    // A chain: `l.to.x`.
    let h = at(&mut c, 7, "hover", &uri, 21, 15);
    assert_eq!(hover_text(&h), "```aye\nto: Point\n```");
    let d = at(&mut c, 8, "definition", &uri, 21, 15);
    assert_eq!(
        d.get("range").unwrap().write(),
        r#"{"start":{"line":14,"character":4},"end":{"line":14,"character":6}}"#
    );
    let h = at(&mut c, 9, "hover", &uri, 21, 18);
    assert_eq!(hover_text(&h), "```aye\nx: i64\n```\n\nAcross.");

    // A `let` in a block: seen in the block, not after it.
    let h = at(&mut c, 10, "hover", &uri, 24, 17);
    assert_eq!(hover_text(&h), "```aye\nlet inner = 1\n```");
    assert_eq!(at(&mut c, 11, "hover", &uri, 26, 13), Json::Null);
    assert_eq!(at(&mut c, 12, "definition", &uri, 26, 13), Json::Null);

    c.finish();
    let _ = std::fs::remove_dir_all(dir);
}

const TYPE_FN: &str = "fn Box(comptime T: type) type {
    return struct {
        v: T

        /// The value.
        pub fn get(self: *Self) T {
            return self.v
        }
    }
}

pub fn main() {
    let b = Box(i64) { v: 1 }
    @println(b.get())
}
";

#[test]
fn hover_on_a_member_of_a_type_function() {
    let (dir, uri) = scratch("typefn", TYPE_FN);
    let mut c = Client::start();
    c.initialize();
    c.notify("textDocument/didOpen", &open(&uri, TYPE_FN));
    c.diagnostics();
    // The tokens show no member there: sema's types render it.
    let h = at(&mut c, 2, "hover", &uri, 13, 15);
    assert_eq!(
        hover_text(&h),
        "```aye\nfn get(self: *Box(i64)) i64\n```\n\nThe value."
    );
    let d = at(&mut c, 3, "definition", &uri, 13, 15);
    assert_eq!(
        d.get("range").unwrap().write(),
        r#"{"start":{"line":5,"character":15},"end":{"line":5,"character":18}}"#
    );
    c.finish();
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn definition_in_a_path_dependency() {
    let src = "import lib\n\npub fn main() {\n    @println(lib.triple(1))\n}\n";
    let (dir, _) = scratch("dep", "");
    let app = dir.join("app");
    let lib = dir.join("lib");
    std::fs::create_dir_all(&app).unwrap();
    std::fs::create_dir_all(&lib).unwrap();
    std::fs::write(app.join("main.aye"), src).unwrap();
    std::fs::write(
        app.join("aye.toml"),
        "name = \"app\"\n\n[deps]\nlib = { path = \"../lib\" }\n",
    )
    .unwrap();
    std::fs::write(lib.join("aye.toml"), "name = \"lib\"\n").unwrap();
    std::fs::write(
        lib.join("lib.aye"),
        "/// Three times `n`.\npub fn triple(n: i64) i64 {\n    return n * 3\n}\n",
    )
    .unwrap();
    let uri = format!("file://{}", app.join("main.aye").display());
    let mut c = Client::start();
    c.initialize();
    c.notify("textDocument/didOpen", &open(&uri, src));
    c.diagnostics();

    let h = at(&mut c, 2, "hover", &uri, 3, 18);
    assert_eq!(
        hover_text(&h),
        "```aye\npub fn triple(n: i64) i64\n```\n\nThree times `n`."
    );
    let d = at(&mut c, 3, "definition", &uri, 3, 18);
    let file = std::fs::canonicalize(lib.join("lib.aye")).unwrap();
    assert_eq!(
        d.get("uri").and_then(Json::as_str),
        Some(format!("file://{}", file.display()).as_str())
    );
    assert_eq!(
        d.get("range").unwrap().write(),
        r#"{"start":{"line":1,"character":7},"end":{"line":1,"character":13}}"#
    );

    c.finish();
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn hover_and_definition_in_a_url_dependency() {
    // tests/deps/url: the app and `tinyjson` as a `file://` tarball, fetched
    // and locked by `aye check` into its own cache.
    let url = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/deps/url");
    let (dir, _) = scratch("urldep", "");
    let app = dir.join("app");
    let cache = dir.join("cache");
    std::fs::create_dir_all(&app).unwrap();
    let tarball = dir.join("tinyjson-1.2.tar.gz");
    let status = Command::new("tar")
        .env("COPYFILE_DISABLE", "1")
        .arg("-czf")
        .arg(&tarball)
        .arg("-C")
        .arg(&url)
        .arg("tinyjson-1.2")
        .status()
        .unwrap();
    assert!(status.success(), "tar -czf failed");
    let src = std::fs::read_to_string(url.join("app/main.aye")).unwrap();
    std::fs::write(app.join("main.aye"), &src).unwrap();
    let toml = std::fs::read_to_string(url.join("app/aye.toml.in")).unwrap();
    let toml = toml.replace("@TARBALL@", tarball.to_str().unwrap());
    std::fs::write(app.join("aye.toml"), toml).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_aye"))
        .current_dir(&app)
        .env("AYE_CACHE", &cache)
        .env_remove("XDG_CACHE_HOME")
        .arg("check")
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    // The server must not download: without the tarball, only the cache has
    // the dependency.
    std::fs::remove_file(&tarball).unwrap();
    let hex = std::fs::read_dir(cache.join("sha256"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();

    let uri = format!("file://{}", app.join("main.aye").display());
    let mut c = Client::start_env(&[("AYE_CACHE", &cache)]);
    c.initialize();
    c.notify("textDocument/didOpen", &open(&uri, &src));
    c.diagnostics();

    // `tinyjson.count`: the top module of the dependency.
    let h = at(&mut c, 2, "hover", &uri, 6, 23);
    assert_eq!(hover_text(&h), "```aye\npub fn count(text: []u8) i64\n```");
    let d = at(&mut c, 3, "definition", &uri, 6, 23);
    assert_eq!(
        d.get("uri").and_then(Json::as_str),
        Some(format!("file://{}", hex.join("tinyjson.aye").display()).as_str())
    );
    assert_eq!(
        d.get("range").unwrap().write(),
        r#"{"start":{"line":4,"character":7},"end":{"line":4,"character":12}}"#
    );
    // `parse.digits`: a submodule, imported as `tinyjson.parse`.
    let d = at(&mut c, 4, "definition", &uri, 7, 20);
    assert_eq!(
        d.get("uri").and_then(Json::as_str),
        Some(format!("file://{}", hex.join("parse/parse.aye").display()).as_str())
    );
    // The module name itself.
    let d = at(&mut c, 5, "definition", &uri, 2, 9);
    assert_eq!(
        d.get("uri").and_then(Json::as_str),
        Some(format!("file://{}", hex.join("tinyjson.aye").display()).as_str())
    );
    c.finish();

    // Not in the cache (and not downloadable): nothing.
    std::fs::remove_dir_all(&cache).unwrap();
    let mut c = Client::start_env(&[("AYE_CACHE", &cache)]);
    c.initialize();
    c.notify("textDocument/didOpen", &open(&uri, &src));
    c.diagnostics();
    let d = at(&mut c, 2, "definition", &uri, 6, 23);
    assert_eq!(d.write(), "null");
    c.finish();
    assert!(!cache.join("sha256").exists());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn hover_and_definition_of_a_cimport_member() {
    if !aye::cimport::libclang_available() {
        println!("skipped: no libclang");
        return;
    }
    let src = "let c = @cimport(\"tiny.h\")\n\npub fn main() {\n    let p = c.tiny_add(1, 2)\n    @println(p)\n}\n";
    let (dir, uri) = scratch("cimport", src);
    std::fs::write(
        dir.join("tiny.h"),
        "/* A function. */\nint tiny_add(int a, int b);\nstruct tiny_pt { int x; int y; };\n",
    )
    .unwrap();
    let cache = dir.join("cache");
    let mut c = Client::start_with(Some(&cache));
    c.initialize();
    c.notify("textDocument/didOpen", &open(&uri, src));
    c.diagnostics();

    let h = at(&mut c, 2, "hover", &uri, 3, 15);
    let text = hover_text(&h);
    assert!(
        text.starts_with("```aye\npub extern fn tiny_add(a: c_int, b: c_int) c_int\n```"),
        "{h:?}"
    );
    let d = at(&mut c, 3, "definition", &uri, 3, 15);
    let target = d.get("uri").and_then(Json::as_str).unwrap().to_string();
    assert!(target.contains("/cache/cimport/tiny-"), "{target}");
    let file = target.strip_prefix("file://").unwrap();
    let gen_src = std::fs::read_to_string(file).unwrap();
    let line = d
        .get("range")
        .and_then(|r| r.get("start"))
        .and_then(|r| r.get("line"))
        .and_then(Json::as_i64)
        .unwrap() as usize;
    assert!(gen_src.lines().nth(line).unwrap().contains("tiny_add"));
    c.finish();
    let _ = std::fs::remove_dir_all(dir);
}

/// One error per source; the range of the diagnostic must end after the name,
/// the offending expression or the type, not at the token fallback.
fn range_of(name: &str, src: &str, code: &str) -> String {
    let (dir, uri) = scratch(name, src);
    let mut c = Client::start();
    c.initialize();
    c.notify("textDocument/didOpen", &open(&uri, src));
    let p = c.diagnostics();
    let list = p.get("diagnostics").unwrap().as_array().unwrap();
    assert_eq!(list.len(), 1, "{list:?}");
    assert_eq!(list[0].get("code").and_then(Json::as_str), Some(code));
    let r = list[0].get("range").unwrap().write();
    c.finish();
    let _ = std::fs::remove_dir_all(dir);
    r
}

#[test]
fn more_errors_have_real_ends() {
    // Unknown field in a struct literal: the field name.
    let src = "struct P {\n    x: i32,\n}\n\npub fn main() {\n    let p = P { widthy: 1 }\n    @println(p.x)\n}\n";
    assert_eq!(
        range_of("field", src, "E0038"),
        r#"{"start":{"line":5,"character":16},"end":{"line":5,"character":22}}"#
    );
    // Unknown struct name.
    let src = "pub fn main() {\n    let p = Missing { x: 1 }\n    @println(p.x)\n}\n";
    assert_eq!(
        range_of("struct", src, "E0017"),
        r#"{"start":{"line":1,"character":12},"end":{"line":1,"character":19}}"#
    );
    // Assignment to an immutable name: the whole target.
    let src = "pub fn main() {\n    let sum = 0\n    sum += 1\n    @println(sum)\n}\n";
    let r = range_of("assign", src, "E0032");
    assert_eq!(
        r,
        r#"{"start":{"line":2,"character":4},"end":{"line":2,"character":7}}"#
    );
}

#[test]
fn unknown_variant_ends_after_its_name() {
    let src = "enum Color {\n    Red,\n    Blue,\n}\n\npub fn main() {\n    let c = Color.Red\n    if c is .Gren {\n        @println(1)\n    }\n}\n";
    let r = range_of("variant", src, "E0041");
    assert!(r.contains(r#""line":7"#), "{r}");
    let (s, e) = (
        r.split(r#""character":"#).nth(1).unwrap(),
        r.split(r#""character":"#).nth(2).unwrap(),
    );
    let num = |t: &str| t.split(['}', ',']).next().unwrap().parse::<u32>().unwrap();
    assert!(num(e) - num(s) >= 4, "{r}");
}

#[test]
fn two_quick_changes_publish_once() {
    let (dir, uri) = scratch("debounce", BAD);
    let mut c = Client::start();
    c.initialize();
    c.notify("textDocument/didOpen", &open(&uri, BAD));
    c.diagnostics();
    let change = |v: i64, text: &str| {
        format!(
            r#"{{"textDocument":{{"uri":{},"version":{v}}},"contentChanges":[{{"text":{}}}]}}"#,
            aye::diag::json_str(&uri),
            aye::diag::json_str(text)
        )
    };
    let t = Instant::now();
    c.notify("textDocument/didChange", &change(2, BAD));
    std::thread::sleep(Duration::from_millis(20));
    c.notify("textDocument/didChange", &change(3, GOOD));
    let p = c.diagnostics();
    assert!(
        t.elapsed() >= Duration::from_millis(150),
        "{:?}",
        t.elapsed()
    );
    assert_eq!(p.get("version").and_then(Json::as_i64), Some(3));
    assert_eq!(p.get("diagnostics"), Some(&Json::Arr(Vec::new())));
    // No second publish: the next message is the reply.
    c.finish();
    let _ = std::fs::remove_dir_all(dir);
}

const SIBLINGS: &str = "pub fn main() {
    if true {
        let x = 1
        @println(x)
    } else {
        let x = 2
        @println(x)
    }
    for i in 0..2 {
        @println(i)
    }
    for i in 0..3 {
        @println(i + 1)
    }
}
";

#[test]
fn a_name_in_a_block_is_its_own_declaration() {
    let (dir, uri) = scratch("blocks", SIBLINGS);
    let mut c = Client::start();
    c.initialize();
    c.notify("textDocument/didOpen", &open(&uri, SIBLINGS));
    c.diagnostics();
    let def_line = |d: &Json| {
        d.get("range")
            .and_then(|r| r.get("start"))
            .and_then(|s| s.get("line"))
            .and_then(Json::as_i64)
    };
    // Each block's `x` and each loop's `i`, from inside it and from just
    // past its end.
    for (id, line, character, decl, sig) in [
        (2, 3, 17, 2, "let x = 1"),
        (4, 3, 18, 2, "let x = 1"),
        (6, 6, 17, 5, "let x = 2"),
        (8, 6, 18, 5, "let x = 2"),
        (10, 9, 17, 8, "for i in 0..2"),
        (12, 12, 17, 11, "for i in 0..3"),
        (14, 12, 18, 11, "for i in 0..3"),
    ] {
        let h = at(&mut c, id, "hover", &uri, line, character);
        assert_eq!(
            hover_text(&h),
            format!("```aye\n{sig}\n```"),
            "{line}:{character}"
        );
        let d = at(&mut c, id + 1, "definition", &uri, line, character);
        assert_eq!(def_line(&d), Some(decl), "{line}:{character}: {d:?}");
    }
    c.finish();
    let _ = std::fs::remove_dir_all(dir);
}

const DOCUMENTED: &str = "/// A box of `T`.
pub fn Box(comptime T: type) type {
    return struct {
        v: T

        /// The value.
        pub fn get(self: *Self) T {
            return self.v
        }
    }
}

/// Two shapes.
pub enum Shape {
    circle,
    square,

    pub fn round(self: Shape) bool {
        return self == Shape.circle
    }
}

/// Two.
pub let TWO = 1 +
    1

pub fn main() {
    let b = Box(i64) { v: TWO }
    @println(b.get())
    @println(Shape.round(Shape.square))
}
";

/// Hover shows a declaration as `aye doc --signatures` does: one renderer.
#[test]
fn hover_renders_declarations_like_aye_doc() {
    let (dir, uri) = scratch("documented", DOCUMENTED);
    let file = dir.join("main.aye");
    let out = Command::new(env!("CARGO_BIN_EXE_aye"))
        .args(["doc", "--signatures"])
        .arg(&file)
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let signatures = String::from_utf8(out.stdout).unwrap();
    // Line and character of the first `name` in the line containing `line`.
    let pos = |line: &str, name: &str| {
        let (l, text) = DOCUMENTED
            .lines()
            .enumerate()
            .find(|(_, t)| t.contains(line))
            .unwrap();
        (l as u32, text.find(name).unwrap() as u32)
    };
    let mut c = Client::start();
    c.initialize();
    c.notify("textDocument/didOpen", &open(&uri, DOCUMENTED));
    c.diagnostics();
    for (id, line, name, doc) in [
        (2, "let b = Box", "Box", "A box of `T`."),
        (3, "Shape.round(", "Shape", "Two shapes."),
    ] {
        let (l, ch) = pos(line, name);
        let h = at(&mut c, id, "hover", &uri, l, ch);
        let text = hover_text(&h);
        let sig = text
            .strip_prefix("```aye\n")
            .and_then(|t| t.strip_suffix(&format!("\n```\n\n{doc}")))
            .unwrap_or_else(|| panic!("{text}"));
        assert!(sig.contains('\n'), "{sig}");
        assert!(
            signatures.contains(&format!("/// {doc}\n{sig}\n")),
            "{sig}\n---\n{signatures}"
        );
    }
    // A `let` over several lines: its first line (`aye doc` prints all).
    let (l, ch) = pos("v: TWO", "TWO");
    let h = at(&mut c, 4, "hover", &uri, l, ch);
    assert_eq!(hover_text(&h), "```aye\npub let TWO = 1 + ...\n```\n\nTwo.");
    assert!(signatures.contains("pub let TWO = 1 + 1\n"), "{signatures}");
    c.finish();
    let _ = std::fs::remove_dir_all(dir);
}

/// Declaration-level errors end after the declared name or the type.
#[test]
fn declaration_errors_end_after_the_name() {
    let main = "\npub fn main() {\n    @println(1)\n}\n";
    let cases: [(&str, String, &str); 5] = [
        // Missing return: the function name.
        (
            "E0050",
            format!("fn first() i32 {{\n}}\n{main}"),
            r#"{"start":{"line":0,"character":3},"end":{"line":0,"character":8}}"#,
        ),
        // Not C-compatible in an `extern fn`: the whole type.
        (
            "E0052",
            format!("extern fn puts(s: []u8) i32\n{main}"),
            r#"{"start":{"line":0,"character":18},"end":{"line":0,"character":22}}"#,
        ),
        // Default value in a union: the field name.
        (
            "E0104",
            format!("extern union U {{\n    a: i32 = 1,\n    b: i64,\n}}\n{main}"),
            r#"{"start":{"line":1,"character":4},"end":{"line":1,"character":5}}"#,
        ),
        // Duplicate field: the second name.
        (
            "E0015",
            format!("struct P {{\n    width: i32,\n    width: i32,\n}}\n{main}"),
            r#"{"start":{"line":2,"character":4},"end":{"line":2,"character":9}}"#,
        ),
        // `comptime` parameter of an `extern fn`: the function name.
        (
            "E0076",
            format!("extern fn pick(comptime T: type) void\n{main}"),
            r#"{"start":{"line":0,"character":10},"end":{"line":0,"character":14}}"#,
        ),
    ];
    for (code, src, want) in cases {
        let (dir, uri) = scratch(&format!("decl_{code}"), &src);
        let mut c = Client::start();
        c.initialize();
        c.notify("textDocument/didOpen", &open(&uri, &src));
        let p = c.diagnostics();
        let list = p.get("diagnostics").unwrap().as_array().unwrap();
        let d = list
            .iter()
            .find(|d| d.get("code").and_then(Json::as_str) == Some(code))
            .unwrap_or_else(|| panic!("{code} missing: {list:?}"));
        assert_eq!(d.get("range").unwrap().write(), want, "{code}");
        c.finish();
        let _ = std::fs::remove_dir_all(dir);
    }
}

/// A hover during typing does not postpone the check of the last change.
#[test]
fn hover_does_not_restart_the_debounce() {
    let (dir, uri) = scratch("debounce_hover", BAD);
    let mut c = Client::start();
    c.initialize();
    c.notify("textDocument/didOpen", &open(&uri, BAD));
    c.diagnostics();
    let change = |v: i64, text: &str| {
        format!(
            r#"{{"textDocument":{{"uri":{},"version":{v}}},"contentChanges":[{{"text":{}}}]}}"#,
            aye::diag::json_str(&uri),
            aye::diag::json_str(text)
        )
    };
    c.notify("textDocument/didChange", &change(2, BAD));
    c.notify("textDocument/didChange", &change(3, GOOD));
    let t = Instant::now();
    std::thread::sleep(Duration::from_millis(100));
    let h = Instant::now();
    c.send(&format!(
        r#"{{"jsonrpc":"2.0","id":7,"method":"textDocument/hover","params":{{"textDocument":{{"uri":{}}},"position":{{"line":1,"character":8}}}}}}"#,
        aye::diag::json_str(&uri)
    ));
    // On a busy machine the check can fire before the hover is read, so the
    // reply and the diagnostics come in either order.
    let (mut reply, mut published) = (None, None);
    while reply.is_none() || published.is_none() {
        let msg = c.recv();
        if msg.get("id").and_then(Json::as_i64) == Some(7) {
            reply = Some(msg);
        } else {
            published = Some((c.params_of_diagnostics(msg), t.elapsed(), h.elapsed()));
        }
    }
    assert!(reply.unwrap().get("result").is_some());
    let (p, e, since_hover) = published.unwrap();
    assert_eq!(p.get("version").and_then(Json::as_i64), Some(3));
    // 150 ms from the last change, about 50 ms after the hover; a restart by
    // the hover would put it 150 ms or more after the hover.
    assert!(e >= Duration::from_millis(150), "{e:?}");
    assert!(since_hover < Duration::from_millis(140), "{since_hover:?}");
    c.finish();
    let _ = std::fs::remove_dir_all(dir);
}
