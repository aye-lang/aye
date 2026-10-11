//! `aye lsp`: a Language Server Protocol server on stdin and stdout.
//!
//! Messages are JSON-RPC 2.0 with `Content-Length` framing. The server
//! keeps the text of every open document (full sync) and offers:
//!
//! - `textDocument/publishDiagnostics` after `didOpen`, `didChange` and
//!   `didSave`: the errors of `driver::front_end` on the document's root,
//!   which is what `aye check <file>` loads, a module of that one file,
//!   with the open text in place of the file on disk. The range of a
//!   diagnostic is the one the compiler gives (`Diag::end`), else the token
//!   at the error; the severity is the diagnostic's. Errors in files the
//!   document imports are not published as such: an `import` line whose
//!   module has errors gets one diagnostic of its own, with the errors as
//!   `relatedInformation`. `didClose` clears them.
//! - `textDocument/codeAction`: a `quickfix` for each diagnostic in the
//!   range that has a fix-it (the edits `aye fix` applies).
//! - `textDocument/formatting`: one edit that replaces the whole text with
//!   `fmt::format`, or no edit if the text is formatted or does not lex.
//! - `textDocument/hover` and `textDocument/definition` (`nav`): the
//!   declaration that the name at the position stands for, with its `///`
//!   comment, and where it is declared; `null` if there is none.
//!
//! There is no JSON library in the project, so this file has a small JSON
//! reader for the messages a client sends; output is built with
//! `diag::json_str`.

use crate::diag::{Diag, Severity, Span, json_str};
use crate::driver::{self, Root};
use crate::fmt;
use crate::module::SourceFile;
use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::panic::{self, AssertUnwindSafe};

mod nav;

/// A JSON value.
#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    /// Parse one JSON document; anything but white space after it is an
    /// error.
    pub fn parse(s: &str) -> Result<Json, String> {
        let mut p = Parser {
            s: s.as_bytes(),
            i: 0,
        };
        let v = p.value(0)?;
        p.ws();
        if p.i != p.s.len() {
            return Err(p.err("trailing characters"));
        }
        Ok(v)
    }

    /// The member `key` of an object.
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(m) => m.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Json]> {
        match self {
            Json::Arr(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Json::Num(n) if n.fract() == 0.0 => Some(*n as i64),
            _ => None,
        }
    }

    /// The value as JSON text.
    pub fn write(&self) -> String {
        match self {
            Json::Null => "null".to_string(),
            Json::Bool(b) => b.to_string(),
            Json::Num(n) if n.fract() == 0.0 && n.abs() < 1e15 => format!("{}", *n as i64),
            Json::Num(n) => format!("{n}"),
            Json::Str(s) => json_str(s),
            Json::Arr(a) => format!(
                "[{}]",
                a.iter().map(Json::write).collect::<Vec<_>>().join(",")
            ),
            Json::Obj(m) => format!(
                "{{{}}}",
                m.iter()
                    .map(|(k, v)| format!("{}:{}", json_str(k), v.write()))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        }
    }
}

/// How deeply values may nest; a deeper message is an error, not a stack
/// overflow.
const MAX_DEPTH: usize = 128;

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn err(&self, what: &str) -> String {
        format!("invalid JSON at byte {}: {what}", self.i)
    }

    fn ws(&mut self) {
        while matches!(self.s.get(self.i), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.i += 1;
        }
    }

    fn eat(&mut self, lit: &str) -> bool {
        if self.s[self.i..].starts_with(lit.as_bytes()) {
            self.i += lit.len();
            true
        } else {
            false
        }
    }

    fn value(&mut self, depth: usize) -> Result<Json, String> {
        if depth > MAX_DEPTH {
            return Err(self.err("nested too deeply"));
        }
        self.ws();
        match self.s.get(self.i) {
            None => Err(self.err("unexpected end")),
            Some(b'{') => {
                self.i += 1;
                let mut members = Vec::new();
                self.ws();
                if self.eat("}") {
                    return Ok(Json::Obj(members));
                }
                loop {
                    self.ws();
                    let key = self.string()?;
                    self.ws();
                    if !self.eat(":") {
                        return Err(self.err("expected `:`"));
                    }
                    members.push((key, self.value(depth + 1)?));
                    self.ws();
                    if self.eat(",") {
                        continue;
                    }
                    if self.eat("}") {
                        return Ok(Json::Obj(members));
                    }
                    return Err(self.err("expected `,` or `}`"));
                }
            }
            Some(b'[') => {
                self.i += 1;
                let mut items = Vec::new();
                self.ws();
                if self.eat("]") {
                    return Ok(Json::Arr(items));
                }
                loop {
                    items.push(self.value(depth + 1)?);
                    self.ws();
                    if self.eat(",") {
                        continue;
                    }
                    if self.eat("]") {
                        return Ok(Json::Arr(items));
                    }
                    return Err(self.err("expected `,` or `]`"));
                }
            }
            Some(b'"') => Ok(Json::Str(self.string()?)),
            Some(_) if self.eat("true") => Ok(Json::Bool(true)),
            Some(_) if self.eat("false") => Ok(Json::Bool(false)),
            Some(_) if self.eat("null") => Ok(Json::Null),
            Some(b'-' | b'0'..=b'9') => {
                let start = self.i;
                while matches!(
                    self.s.get(self.i),
                    Some(b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9')
                ) {
                    self.i += 1;
                }
                let text = std::str::from_utf8(&self.s[start..self.i]).unwrap_or("");
                text.parse::<f64>()
                    .map(Json::Num)
                    .map_err(|_| self.err("bad number"))
            }
            Some(_) => Err(self.err("unexpected character")),
        }
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let digits = self.s.get(self.i..self.i + 4).ok_or(self.err("bad \\u"))?;
        let text = std::str::from_utf8(digits).map_err(|_| self.err("bad \\u"))?;
        let n = u32::from_str_radix(text, 16).map_err(|_| self.err("bad \\u"))?;
        self.i += 4;
        Ok(n)
    }

    fn string(&mut self) -> Result<String, String> {
        if !self.eat("\"") {
            return Err(self.err("expected a string"));
        }
        let mut out = Vec::new();
        loop {
            let Some(&b) = self.s.get(self.i) else {
                return Err(self.err("unterminated string"));
            };
            self.i += 1;
            match b {
                b'"' => break,
                b'\\' => {
                    let Some(&e) = self.s.get(self.i) else {
                        return Err(self.err("unterminated string"));
                    };
                    self.i += 1;
                    let c = match e {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'u' => {
                            let mut n = self.hex4()?;
                            if (0xd800..0xdc00).contains(&n) && self.eat("\\u") {
                                let low = self.hex4()?;
                                if (0xdc00..0xe000).contains(&low) {
                                    n = 0x10000 + ((n - 0xd800) << 10) + (low - 0xdc00);
                                } else {
                                    return Err(self.err("bad surrogate pair"));
                                }
                            }
                            char::from_u32(n).unwrap_or('\u{fffd}')
                        }
                        _ => return Err(self.err("bad escape")),
                    };
                    let mut buf = [0u8; 4];
                    out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
                }
                b => out.push(b),
            }
        }
        String::from_utf8(out).map_err(|_| self.err("string is not UTF-8"))
    }
}

/// Read one message: the headers, then `Content-Length` bytes. `Ok(None)`
/// is the end of the input.
fn read_message(input: &mut impl BufRead) -> std::io::Result<Option<String>> {
    let mut len = None;
    loop {
        let mut line = String::new();
        if input.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            if len.is_some() {
                break;
            }
            continue;
        }
        if let Some((name, value)) = line.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            len = value.trim().parse::<usize>().ok();
        }
    }
    let mut body = vec![0u8; len.unwrap_or(0)];
    input.read_exact(&mut body)?;
    Ok(Some(String::from_utf8_lossy(&body).into_owned()))
}

fn send(out: &mut impl Write, body: &str) -> std::io::Result<()> {
    write!(out, "Content-Length: {}\r\n\r\n{body}", body.len())?;
    out.flush()
}

fn response(id: &Json, result: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":{},"result":{result}}}"#,
        id.write()
    )
}

fn error_response(id: &Json, code: i32, message: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":{},"error":{{"code":{code},"message":{}}}}}"#,
        id.write(),
        json_str(message)
    )
}

/// The path of a `file:` URI, percent-decoded; any other URI is used as it
/// is, as the name of an unsaved document.
fn uri_path(uri: &str) -> String {
    let Some(rest) = uri.strip_prefix("file://") else {
        return uri.to_string();
    };
    // `file://host/path`: the path starts at the next `/`.
    let path = &rest[rest.find('/').unwrap_or(rest.len())..];
    let b = path.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let hex = |c: u8| (c as char).to_digit(16);
        match (
            b[i],
            b.get(i + 1).and_then(|&c| hex(c)),
            b.get(i + 2).and_then(|&c| hex(c)),
        ) {
            (b'%', Some(h), Some(l)) => {
                out.push((h * 16 + l) as u8);
                i += 3;
            }
            (c, _, _) => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The LSP character offset (UTF-16 code units) of the first `chars`
/// characters of `line`.
fn utf16_col(line: &str, chars: usize) -> u32 {
    line.chars().take(chars).map(|c| c.len_utf16() as u32).sum()
}

/// How many characters of `line` from character `start` (0-based) make the
/// token there: an identifier or number, a string literal, else one
/// character; none at the end of the line.
fn token_len(line: &str, start: usize) -> usize {
    let rest: Vec<char> = line.chars().skip(start).collect();
    match rest.first() {
        None => 0,
        Some('"') => {
            let mut i = 1;
            while i < rest.len() {
                match rest[i] {
                    '\\' => i += 1,
                    '"' => return i + 1,
                    _ => {}
                }
                i += 1;
            }
            rest.len()
        }
        Some(c) if c.is_alphanumeric() || *c == '_' || *c == '@' => {
            1 + rest[1..]
                .iter()
                .take_while(|c| c.is_alphanumeric() || **c == '_')
                .count()
        }
        Some(_) => 1,
    }
}

/// The LSP position (0-based line, UTF-16 character) of the 1-based line and
/// character column `at` in `src`.
fn position(src: &str, at: Span) -> (u32, u32) {
    let line_no = at.line.saturating_sub(1);
    let line = src
        .split('\n')
        .nth(line_no as usize)
        .unwrap_or("")
        .trim_end_matches('\r');
    (
        line_no,
        utf16_col(line, (at.col as usize).saturating_sub(1)),
    )
}

fn range_json(start: (u32, u32), end: (u32, u32)) -> String {
    format!(
        r#"{{"start":{{"line":{},"character":{}}},"end":{{"line":{},"character":{}}}}}"#,
        start.0, start.1, end.0, end.1
    )
}

/// The LSP range of `d` in the document `src`: up to `Diag::end` if the
/// compiler gave one after the start, else the token at the error.
fn range(src: &str, d: &Diag) -> ((u32, u32), (u32, u32)) {
    let start = position(src, d.span);
    let end = match d.end {
        Some(e) if (e.line, e.col) > (d.span.line, d.span.col) => position(src, e),
        _ => {
            let line = src
                .split('\n')
                .nth(start.0 as usize)
                .unwrap_or("")
                .trim_end_matches('\r');
            let from = (d.span.col as usize).saturating_sub(1);
            (start.0, utf16_col(line, from + token_len(line, from)))
        }
    };
    (start, end)
}

/// The LSP diagnostic of `d` in the document `src`. Aye lines and columns
/// are 1-based, and a column counts characters; LSP's are 0-based, and a
/// character offset counts UTF-16 units. A fix-it's description goes on the
/// message after a newline as `help: ...`.
fn diagnostic(src: &str, d: &Diag) -> String {
    let mut message = d.msg.clone();
    if let Some(f) = &d.fix {
        message.push_str("\nhelp: ");
        message.push_str(&f.help);
    }
    let r = range(src, d);
    let severity = match d.severity {
        Severity::Error => 1,
        Severity::Warning => 2,
    };
    format!(
        r#"{{"range":{},"severity":{severity},"code":"{}","source":"aye","message":{}}}"#,
        range_json(r.0, r.1),
        d.code,
        json_str(&message)
    )
}

/// The `file:` URI of the file `path`, percent-encoded.
fn path_uri(path: &str) -> String {
    let mut out = String::from("file://");
    for b in path.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                out.push(b as char)
            }
            b => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// The `import a.b.c` lines of `src`: the 0-based line, the path, and the
/// length of the line in characters.
fn import_lines(src: &str) -> Vec<(u32, Vec<String>, usize)> {
    src.split('\n')
        .enumerate()
        .filter_map(|(i, line)| {
            let line = line.trim_end_matches('\r');
            let rest = line.strip_prefix("import ")?;
            let word = rest
                .trim()
                .split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '.'))
                .next()?;
            let path: Vec<String> = word.split('.').map(str::to_string).collect();
            let len = line.trim_end().chars().count();
            (!path.iter().any(String::is_empty)).then_some((i as u32, path, len))
        })
        .collect()
}

/// Does the directory of the file `name` end with `/a/b` for the path
/// `a.b`: is the file part of the module?
fn in_module(name: &str, path: &[String]) -> bool {
    let dir = name.rsplit_once('/').map_or("", |(d, _)| d);
    format!("/{dir}").ends_with(&format!("/{}", path.join("/")))
}

/// The errors of the root `path` (the text `src` replaces the file on
/// disk's): the name of the document's file, all the files, and the errors
/// of every file. The root is the one `aye check <path>` loads; a file that
/// is not on disk is a root of its own.
fn check(path: &str, src: &str) -> (String, Vec<SourceFile>, Vec<Diag>) {
    let mut root = Root::load(path).unwrap_or_else(|_| Root::single(path, src));
    root.files[0].text = src.to_string();
    let name = root.files[0].name.clone();
    let checked = panic::catch_unwind(AssertUnwindSafe(|| driver::front_end(root, None)));
    match checked {
        Ok(c) => (name, c.files, c.result.err().unwrap_or_default()),
        Err(_) => (name, Vec::new(), Vec::new()),
    }
}

/// How many errors of an imported module go in `relatedInformation`.
const MAX_RELATED: usize = 10;

/// The errors in the file `path` (its text `src` replaces the disk's) as
/// the JSON array of LSP diagnostics, and for each `import` line whose
/// module has errors one more, with those errors as related information.
fn diagnostics(path: &str, src: &str) -> String {
    let (name, files, ds) = check(path, src);
    let mut list: Vec<String> = ds
        .iter()
        .filter(|d| d.file == name)
        .map(|d| diagnostic(src, d))
        .collect();
    for (line, import, len) in import_lines(src) {
        // The longest prefix of the import that is a module with errors.
        let theirs = |k: usize| -> Vec<&Diag> {
            ds.iter()
                .filter(|d| d.file != name && in_module(&d.file, &import[..k]))
                .collect()
        };
        let Some(k) = (1..=import.len()).rev().find(|&k| !theirs(k).is_empty()) else {
            continue;
        };
        let theirs = theirs(k);
        let related: Vec<String> = theirs
            .iter()
            .take(MAX_RELATED)
            .map(|d| {
                let text = files
                    .iter()
                    .find(|f| f.name == d.file)
                    .map_or("", |f| f.text.as_str());
                let (start, end) = range(text, d);
                format!(
                    r#"{{"location":{{"uri":{},"range":{}}},"message":{}}}"#,
                    json_str(&path_uri(&d.file)),
                    range_json(start, end),
                    json_str(&format!("{}: {}", d.code, d.msg))
                )
            })
            .collect();
        let message = format!(
            "module `{}` has {} error{}",
            import[..k].join("."),
            theirs.len(),
            if theirs.len() == 1 { "" } else { "s" }
        );
        let text = src.split('\n').nth(line as usize).unwrap_or("");
        list.push(format!(
            r#"{{"range":{},"severity":1,"source":"aye","message":{},"relatedInformation":[{}]}}"#,
            range_json((line, 0), (line, utf16_col(text, len))),
            json_str(&message),
            related.join(",")
        ));
    }
    format!("[{}]", list.join(","))
}

/// The code actions for the diagnostics of the document `uri` (`path`, text
/// `src`) that touch the LSP range from `from` to `to` and have a fix-it: a
/// `quickfix` whose edit applies the fix-it to the document.
fn code_actions(uri: &str, path: &str, src: &str, from: (u32, u32), to: (u32, u32)) -> String {
    let (name, _, ds) = check(path, src);
    let list: Vec<String> = ds
        .iter()
        .filter(|d| d.file == name)
        .filter_map(|d| {
            let fix = d.fix.as_ref()?;
            let (start, end) = range(src, d);
            // The ranges touch, ends included.
            if end < from || to < start {
                return None;
            }
            let edits: Vec<String> = fix
                .edits
                .iter()
                .map(|e| {
                    format!(
                        r#"{{"range":{},"newText":{}}}"#,
                        range_json(position(src, e.start), position(src, e.end)),
                        json_str(&e.replacement)
                    )
                })
                .collect();
            Some(format!(
                r#"{{"title":{},"kind":"quickfix","diagnostics":[{}],"isPreferred":true,"edit":{{"changes":{{{}:[{}]}}}}}}"#,
                json_str(&fix.help),
                diagnostic(src, d),
                json_str(uri),
                edits.join(",")
            ))
        })
        .collect();
    format!("[{}]", list.join(","))
}

/// The edits that format `src`: none when it is formatted or does not lex,
/// else one edit that replaces all of it.
fn formatting(src: &str) -> String {
    match fmt::format(src) {
        Ok(new) if new != src => {
            let lines: Vec<&str> = src.split('\n').collect();
            let last = lines.last().copied().unwrap_or("");
            format!(
                r#"[{{"range":{{"start":{{"line":0,"character":0}},"end":{{"line":{},"character":{}}}}},"newText":{}}}]"#,
                lines.len() - 1,
                utf16_col(last, usize::MAX),
                json_str(&new)
            )
        }
        _ => "[]".to_string(),
    }
}

struct Doc {
    text: String,
    version: Option<i64>,
}

struct Server<W: Write> {
    out: W,
    docs: HashMap<String, Doc>,
    shutdown: bool,
}

impl<W: Write> Server<W> {
    fn publish(&mut self, uri: &str) -> std::io::Result<()> {
        let Some(doc) = self.docs.get(uri) else {
            return Ok(());
        };
        let list = diagnostics(&uri_path(uri), &doc.text);
        let version = doc.version.map(|v| format!(r#","version":{v}"#));
        self.notify_diagnostics(uri, &list, version.as_deref().unwrap_or(""))
    }

    fn notify_diagnostics(&mut self, uri: &str, list: &str, version: &str) -> std::io::Result<()> {
        let body = format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{{"uri":{}{version},"diagnostics":{list}}}}}"#,
            json_str(uri)
        );
        send(&mut self.out, &body)
    }

    /// Handle one message. `Some(code)` is the exit status after `exit`.
    fn handle(&mut self, msg: &Json) -> std::io::Result<Option<i32>> {
        let method = msg.get("method").and_then(Json::as_str).unwrap_or("");
        let params = msg.get("params");
        let doc_of = |key: &str| params.and_then(|p| p.get(key)).and_then(|d| d.get("uri"));
        let uri = doc_of("textDocument")
            .and_then(Json::as_str)
            .map(str::to_string);
        match (method, msg.get("id")) {
            ("exit", _) => return Ok(Some(if self.shutdown { 0 } else { 1 })),
            // A response from the client; the server sends no requests.
            ("", _) => {}
            (_, Some(id)) if self.shutdown => {
                send(
                    &mut self.out,
                    &error_response(id, -32600, "the server is shut down"),
                )?;
            }
            ("initialize", Some(id)) => {
                let result = r#"{"capabilities":{"textDocumentSync":{"openClose":true,"change":1,"save":{"includeText":true}},"documentFormattingProvider":true,"codeActionProvider":{"codeActionKinds":["quickfix"]},"hoverProvider":true,"definitionProvider":true},"serverInfo":{"name":"aye","version":"0.1.0"}}"#;
                send(&mut self.out, &response(id, result))?;
            }
            ("shutdown", Some(id)) => {
                self.shutdown = true;
                send(&mut self.out, &response(id, "null"))?;
            }
            ("textDocument/formatting", Some(id)) => {
                let result = match uri.as_ref().and_then(|u| self.docs.get(u)) {
                    Some(doc) => formatting(&doc.text),
                    None => "null".to_string(),
                };
                send(&mut self.out, &response(id, &result))?;
            }
            ("textDocument/codeAction", Some(id)) => {
                let range = params.and_then(|p| p.get("range"));
                let at = |end: &str| {
                    let p = range.and_then(|r| r.get(end))?;
                    let n = |k: &str| u32::try_from(p.get(k).and_then(Json::as_i64)?).ok();
                    Some((n("line")?, n("character")?))
                };
                let result = match (
                    uri.as_ref().and_then(|u| self.docs.get(u).map(|d| (u, d))),
                    at("start"),
                    at("end"),
                ) {
                    (Some((u, doc)), Some(from), Some(to)) => {
                        let path = uri_path(u);
                        panic::catch_unwind(AssertUnwindSafe(|| {
                            code_actions(u, &path, &doc.text, from, to)
                        }))
                        .unwrap_or_else(|_| "null".to_string())
                    }
                    _ => "null".to_string(),
                };
                send(&mut self.out, &response(id, &result))?;
            }
            ("textDocument/hover" | "textDocument/definition", Some(id)) => {
                let pos = params.and_then(|p| p.get("position"));
                let at = |k: &str| {
                    let n = pos.and_then(|p| p.get(k)).and_then(Json::as_i64)?;
                    u32::try_from(n).ok()
                };
                let result = match (
                    uri.as_ref().and_then(|u| self.docs.get(u).map(|d| (u, d))),
                    at("line"),
                    at("character"),
                ) {
                    (Some((u, doc)), Some(line), Some(character)) => {
                        let find = if method == "textDocument/hover" {
                            nav::hover
                        } else {
                            nav::definition
                        };
                        let path = uri_path(u);
                        panic::catch_unwind(AssertUnwindSafe(|| {
                            find(u, &path, &doc.text, line, character)
                        }))
                        .unwrap_or_else(|_| "null".to_string())
                    }
                    _ => "null".to_string(),
                };
                send(&mut self.out, &response(id, &result))?;
            }
            (_, Some(id)) => {
                let text = format!("method not found: {method}");
                send(&mut self.out, &error_response(id, -32601, &text))?;
            }
            ("textDocument/didOpen", None) => {
                let item = params.and_then(|p| p.get("textDocument"));
                let field = |k: &str| item.and_then(|i| i.get(k));
                if let (Some(uri), Some(text)) = (
                    field("uri").and_then(Json::as_str),
                    field("text").and_then(Json::as_str),
                ) {
                    let version = field("version").and_then(Json::as_i64);
                    let doc = Doc {
                        text: text.to_string(),
                        version,
                    };
                    self.docs.insert(uri.to_string(), doc);
                    self.publish(uri)?;
                }
            }
            ("textDocument/didChange", None) => {
                // Full sync: the last change holds the whole text.
                let text = params
                    .and_then(|p| p.get("contentChanges"))
                    .and_then(Json::as_array)
                    .and_then(|c| c.last())
                    .and_then(|c| c.get("text"))
                    .and_then(Json::as_str);
                let version = params
                    .and_then(|p| p.get("textDocument"))
                    .and_then(|d| d.get("version"))
                    .and_then(Json::as_i64);
                if let (Some(uri), Some(text)) = (uri, text) {
                    let doc = Doc {
                        text: text.to_string(),
                        version,
                    };
                    self.docs.insert(uri.clone(), doc);
                    self.publish(&uri)?;
                }
            }
            ("textDocument/didSave", None) => {
                if let Some(uri) = uri {
                    let text = params.and_then(|p| p.get("text")).and_then(Json::as_str);
                    if let (Some(doc), Some(text)) = (self.docs.get_mut(&uri), text) {
                        doc.text = text.to_string();
                    }
                    self.publish(&uri)?;
                }
            }
            ("textDocument/didClose", None) => {
                if let Some(uri) = uri {
                    self.docs.remove(&uri);
                    self.notify_diagnostics(&uri, "[]", "")?;
                }
            }
            _ => {}
        }
        Ok(None)
    }
}

/// Serve the client on `input` and `output` until `exit` or the end of the
/// input. Returns the exit status: 0 after `shutdown` and `exit`, else 1.
pub fn run(mut input: impl BufRead, output: impl Write) -> i32 {
    let mut server = Server {
        out: output,
        docs: HashMap::new(),
        shutdown: false,
    };
    loop {
        let body = match read_message(&mut input) {
            Ok(Some(b)) => b,
            _ => return 1,
        };
        let step = match Json::parse(&body) {
            Ok(msg) => server.handle(&msg),
            Err(e) => send(&mut server.out, &error_response(&Json::Null, -32700, &e)).map(|_| None),
        };
        match step {
            Ok(Some(code)) => return code,
            Ok(None) => {}
            Err(_) => return 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_round_trip() {
        let text = r#"{"a":[1,2.5,true,null,"x\né😀"],"b":{}}"#;
        let v = Json::parse(text).unwrap();
        assert_eq!(
            v.get("a").unwrap().as_array().unwrap()[4].as_str(),
            Some("x\n\u{e9}\u{1f600}")
        );
        assert_eq!(Json::parse(&v.write()).unwrap(), v);
        assert!(Json::parse("{\"a\":1} x").is_err());
        assert!(Json::parse(&"[".repeat(500)).is_err());
    }

    #[test]
    fn file_uri_is_decoded() {
        assert_eq!(uri_path("file:///a/b%20c/d.aye"), "/a/b c/d.aye");
        assert_eq!(uri_path("untitled:Untitled-1"), "untitled:Untitled-1");
    }

    #[test]
    fn columns_count_utf16_units() {
        assert_eq!(utf16_col("a\u{1f600}b", 2), 3);
        assert_eq!(token_len("let x = \"hi\"", 8), 4);
        assert_eq!(token_len("let xs = 1", 4), 2);
        assert_eq!(token_len("ab", 2), 0);
    }
}
