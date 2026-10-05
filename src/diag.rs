//! Source spans and diagnostics shared by every compiler stage.

use std::fmt;

/// A 1-based line/column position in the source file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Span {
    pub line: u32,
    pub col: u32,
}

impl Span {
    pub fn new(line: u32, col: u32) -> Span {
        Span { line, col }
    }
}

/// A compile error: `file:line:col: error: message`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diag {
    pub file: String,
    pub span: Span,
    pub msg: String,
}

impl fmt::Display for Diag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}: error: {}", self.file, self.span.line, self.span.col, self.msg)
    }
}

impl Diag {
    /// One entry of the `diagnostics` array in `ore check --json`.
    pub fn to_json(&self) -> String {
        format!(
            r#"{{"severity":"error","line":{},"col":{},"message":{}}}"#,
            self.span.line,
            self.span.col,
            json_str(&self.msg)
        )
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
    pub msg: String,
}

impl Error {
    pub fn new(span: Span, msg: impl Into<String>) -> Error {
        Error { span, msg: msg.into() }
    }

    pub fn with_file(self, file: &str) -> Diag {
        Diag { file: file.to_string(), span: self.span, msg: self.msg }
    }
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
