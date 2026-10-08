//! Source spans and diagnostics shared by every compiler stage.

use crate::codes::Code;
use std::fmt;

/// A 1-based line/column position in a source file. `file` indexes the
/// files of the program (0 is the first file of the root module).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Span {
    pub line: u32,
    pub col: u32,
    pub file: u16,
}

impl Span {
    pub fn new(line: u32, col: u32) -> Span {
        Span { line, col, file: 0 }
    }

    /// The position `n` columns to the right, in the same file.
    pub fn right(self, n: u32) -> Span {
        Span { col: self.col + n, ..self }
    }
}

/// One edit of a fix-it: replace the text from `start` up to (not
/// including) `end` with `replacement`. An insertion has `start == end`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub start: Span,
    pub end: Span,
    pub replacement: String,
}

/// A machine-applicable fix: its edits, applied together, and a short
/// description for people (`help: ...`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fix {
    pub help: String,
    pub edits: Vec<Edit>,
}

/// A compile error: `file:line:col: error[E0001]: message`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diag {
    pub file: String,
    pub span: Span,
    pub code: Code,
    pub msg: String,
    pub fix: Option<Fix>,
}

impl fmt::Display for Diag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}: error[{}]: {}", self.file, self.span.line, self.span.col, self.code, self.msg)
    }
}

impl Diag {
    /// One entry of the `diagnostics` array in `aye check --json`. `file`
    /// is the file of the error; a fix-it edits only that file.
    pub fn to_json(&self) -> String {
        let edits: Vec<String> = self
            .fix
            .iter()
            .flat_map(|f| &f.edits)
            .map(|e| {
                format!(
                    r#"{{"line":{},"col":{},"end_line":{},"end_col":{},"replacement":{}}}"#,
                    e.start.line,
                    e.start.col,
                    e.end.line,
                    e.end.col,
                    json_str(&e.replacement)
                )
            })
            .collect();
        format!(
            r#"{{"severity":"error","code":"{}","file":{},"line":{},"col":{},"message":{},"fixes":[{}]}}"#,
            self.code,
            json_str(&self.file),
            self.span.line,
            self.span.col,
            json_str(&self.msg),
            edits.join(",")
        )
    }

    /// The diagnostic as the command line prints it: the error line and, for
    /// a fix-it, `file:line:col: help: ...` at its first edit.
    pub fn render(&self) -> String {
        match &self.fix {
            Some(f) => {
                let at = f.edits[0].start;
                format!("{self}\n{}:{}:{}: help: {}", self.file, at.line, at.col, f.help)
            }
            None => self.to_string(),
        }
    }
}

/// `s` as a quoted JSON string.
pub fn json_str(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Stage-internal error before the file name is attached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub span: Span,
    pub code: Code,
    pub msg: String,
    pub fix: Option<Fix>,
}

impl Error {
    pub fn new(code: Code, span: Span, msg: impl Into<String>) -> Error {
        Error { span, code, msg: msg.into(), fix: None }
    }

    /// Attach a fix-it of one edit.
    pub fn fix(mut self, help: impl Into<String>, start: Span, end: Span, replacement: impl Into<String>) -> Error {
        self.fix = Some(Fix { help: help.into(), edits: vec![Edit { start, end, replacement: replacement.into() }] });
        self
    }

    /// The diagnostic, with the name of its file from `files` (indexed by
    /// `Span::file`).
    pub fn with_file(self, files: &[String]) -> Diag {
        let file = files.get(self.span.file as usize).cloned().unwrap_or_default();
        Diag { file, span: self.span, code: self.code, msg: self.msg, fix: self.fix }
    }
}

/// Byte offset of the 1-based line/column `at` in `src`, where a column
/// counts characters. Past the end of a line or file it is clamped there.
pub fn offset(src: &str, at: Span) -> usize {
    let mut line = 1;
    let mut col = 1;
    for (i, c) in src.char_indices() {
        if line == at.line && (col == at.col || c == '\n') {
            return i;
        }
        if c == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    src.len()
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_str_escapes() {
        assert_eq!(json_str("a\"b\\c\nd\te\rf\u{1}g"), r#""a\"b\\c\nd\te\rf\u0001g""#);
    }
}
