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
