//! Lexer with Go-style automatic terminator insertion.
//!
//! A newline (or end of file) becomes a `Tok::Semi` terminator when the
//! previous token is an identifier, a literal, `)`, `]`, `}`, one of the
//! keywords `return`, `break`, `continue`, `unreachable`, or `.*`, `.?`;
//! except that after a line of a multi-line string (`\\ text`), a newline
//! before another such line, or before a line that starts with `,`, `)`,
//! `]` or `}`, is not one. Aye source never contains an explicit `;`.
//!
//! After a lexical error the rest of the line is skipped and lexing goes on,
//! so every bad line is reported.

use crate::codes::*;
use crate::diag::{Error, Result, Span};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kw {
    Fn,
    Let,
    Var,
    Pub,
    Import,
    Struct,
    Enum,
    Error,
    Interface,
    Dyn,
    Match,
    If,
    Else,
    For,
    While,
    Break,
    Continue,
    Return,
    Defer,
    Errdefer,
    Try,
    Catch,
    Comptime,
    Where,
    As,
    Test,
    Mut,
    Extern,
    Orelse,
    Unreachable,
    Is,
}

const KEYWORDS: &[(&str, Kw)] = &[
    ("fn", Kw::Fn),
    ("let", Kw::Let),
    ("var", Kw::Var),
    ("pub", Kw::Pub),
    ("import", Kw::Import),
    ("struct", Kw::Struct),
    ("enum", Kw::Enum),
    ("error", Kw::Error),
    ("interface", Kw::Interface),
    ("dyn", Kw::Dyn),
    ("match", Kw::Match),
    ("if", Kw::If),
    ("else", Kw::Else),
    ("for", Kw::For),
    ("while", Kw::While),
    ("break", Kw::Break),
    ("continue", Kw::Continue),
    ("return", Kw::Return),
    ("defer", Kw::Defer),
    ("errdefer", Kw::Errdefer),
    ("try", Kw::Try),
    ("catch", Kw::Catch),
    ("comptime", Kw::Comptime),
    ("where", Kw::Where),
    ("as", Kw::As),
    ("test", Kw::Test),
    ("mut", Kw::Mut),
    ("extern", Kw::Extern),
    ("orelse", Kw::Orelse),
    ("unreachable", Kw::Unreachable),
    ("is", Kw::Is),
];

/// Is `word` an Aye keyword (`@cimport` renames C names that are)?
pub fn is_keyword(word: &str) -> bool {
    KEYWORDS.iter().any(|(k, _)| *k == word)
}

impl Kw {
    pub fn as_str(self) -> &'static str {
        KEYWORDS.iter().find(|(_, k)| *k == self).map(|(s, _)| *s).unwrap()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    Ident(String),
    /// `@name` builtin reference.
    Builtin(String),
    Int(u64),
    Float(f64),
    Str(Vec<u8>),
    /// One line of a multi-line string, `\\ text`: the text after `\\`
    /// up to the end of the line. The parser joins consecutive lines.
    LineStr(Vec<u8>),
    Char(u32),
    Kw(Kw),
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Colon,
    Dot,
    DotDot,
    /// `..=`, an inclusive range in a `match` pattern.
    DotDotEq,
    /// `...`, the variadic part of a C function's parameters.
    Ellipsis,
    /// `.*`, pointer dereference.
    DotStar,
    /// `.?`, optional unwrap.
    DotQuestion,
    /// `=>` in a `match` arm.
    FatArrow,
    /// `&`, address-of or bitwise and.
    Amp,
    /// `|`, bitwise or, or around an `if x |v|` capture.
    Pipe,
    Caret,
    Tilde,
    Shl,
    Shr,
    Question,
    Bang,
    Assign,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercentEq,
    PlusPctEq,
    MinusPctEq,
    StarPctEq,
    AmpEq,
    PipeEq,
    CaretEq,
    ShlEq,
    ShrEq,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    PlusPct,
    MinusPct,
    StarPct,
    EqEq,
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,
    AndAnd,
    OrOr,
    /// Statement terminator inserted at a newline or end of file.
    Semi,
    Eof,
}

impl fmt::Display for Tok {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Tok::Ident(n) => return write!(f, "identifier `{n}`"),
            Tok::Builtin(n) => return write!(f, "`@{n}`"),
            Tok::Int(_) => "integer literal",
            Tok::Float(_) => "float literal",
            Tok::Str(_) | Tok::LineStr(_) => "string literal",
            Tok::Char(_) => "character literal",
            Tok::Kw(k) => return write!(f, "`{}`", k.as_str()),
            Tok::Semi => "newline",
            Tok::Eof => "end of file",
            Tok::LParen => "`(`",
            Tok::RParen => "`)`",
            Tok::LBrace => "`{`",
            Tok::RBrace => "`}`",
            Tok::LBracket => "`[`",
            Tok::RBracket => "`]`",
            Tok::Comma => "`,`",
            Tok::Colon => "`:`",
            Tok::Dot => "`.`",
            Tok::DotDot => "`..`",
            Tok::DotDotEq => "`..=`",
            Tok::Ellipsis => "`...`",
            Tok::DotStar => "`.*`",
            Tok::DotQuestion => "`.?`",
            Tok::FatArrow => "`=>`",
            Tok::Amp => "`&`",
            Tok::Pipe => "`|`",
            Tok::Caret => "`^`",
            Tok::Tilde => "`~`",
            Tok::Shl => "`<<`",
            Tok::Shr => "`>>`",
            Tok::Question => "`?`",
            Tok::Bang => "`!`",
            Tok::Assign => "`=`",
            Tok::PlusEq => "`+=`",
            Tok::MinusEq => "`-=`",
            Tok::StarEq => "`*=`",
            Tok::SlashEq => "`/=`",
            Tok::PercentEq => "`%=`",
            Tok::PlusPctEq => "`+%=`",
            Tok::MinusPctEq => "`-%=`",
            Tok::StarPctEq => "`*%=`",
            Tok::AmpEq => "`&=`",
            Tok::PipeEq => "`|=`",
            Tok::CaretEq => "`^=`",
            Tok::ShlEq => "`<<=`",
            Tok::ShrEq => "`>>=`",
            Tok::Plus => "`+`",
            Tok::Minus => "`-`",
            Tok::Star => "`*`",
            Tok::Slash => "`/`",
            Tok::Percent => "`%`",
            Tok::PlusPct => "`+%`",
            Tok::MinusPct => "`-%`",
            Tok::StarPct => "`*%`",
            Tok::EqEq => "`==`",
            Tok::NotEq => "`!=`",
            Tok::Lt => "`<`",
            Tok::Le => "`<=`",
            Tok::Gt => "`>`",
            Tok::Ge => "`>=`",
            Tok::AndAnd => "`&&`",
            Tok::OrOr => "`||`",
        };
        f.write_str(s)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub tok: Tok,
    pub span: Span,
    /// The position just after the token.
    pub end: Span,
}

/// Does a newline after this token terminate the statement?
fn ends_statement(t: &Tok) -> bool {
    matches!(
        t,
        Tok::Ident(_)
            | Tok::Int(_)
            | Tok::Float(_)
            | Tok::Str(_)
            | Tok::LineStr(_)
            | Tok::Char(_)
            | Tok::RParen
            | Tok::RBracket
            | Tok::RBrace
            | Tok::DotStar
            | Tok::DotQuestion
            | Tok::Kw(Kw::Unreachable)
            | Tok::Kw(Kw::Return)
            | Tok::Kw(Kw::Break)
            | Tok::Kw(Kw::Continue)
    )
}

struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: u32,
    col: u32,
    toks: Vec<Token>,
    errors: Vec<Error>,
}

pub fn lex(src: &str) -> std::result::Result<Vec<Token>, Vec<Error>> {
    let mut lx =
        Lexer { chars: src.chars().collect(), pos: 0, line: 1, col: 1, toks: Vec::new(), errors: Vec::new() };
    lx.run();
    if lx.errors.is_empty() { Ok(lx.toks) } else { Err(lx.errors) }
}

impl Lexer {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_at(&self, n: usize) -> Option<char> {
        self.chars.get(self.pos + n).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.chars.get(self.pos).copied()?;
        self.pos += 1;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(c)
    }

    fn here(&self) -> Span {
        Span::new(self.line, self.col)
    }

    fn push(&mut self, tok: Tok, span: Span) {
        let end = self.here();
        self.toks.push(Token { tok, span, end });
    }

    fn terminate(&mut self, span: Span) {
        if self.toks.last().is_some_and(|t| ends_statement(&t.tok)) {
            self.push(Tok::Semi, span);
        }
    }

    fn run(&mut self) {
        while self.peek().is_some() {
            if let Err(e) = self.token() {
                self.errors.push(e);
                while self.peek().is_some_and(|c| c != '\n') {
                    self.bump();
                }
            }
        }
        let span = self.here();
        self.terminate(span);
        self.push(Tok::Eof, span);
    }

    /// Lex one token (or skip whitespace, a newline or a comment).
    fn token(&mut self) -> Result<()> {
        if let Some(c) = self.peek() {
            let span = self.here();
            match c {
                '\n' => {
                    if !self.continues_line_str() {
                        self.terminate(span);
                    }
                    self.bump();
                }
                ' ' | '\t' | '\r' => {
                    self.bump();
                }
                '/' if self.peek_at(1) == Some('/') => {
                    while self.peek().is_some_and(|c| c != '\n') {
                        self.bump();
                    }
                }
                c if c.is_ascii_alphabetic() || c == '_' => {
                    let word = self.ident();
                    let tok = match KEYWORDS.iter().find(|(s, _)| *s == word) {
                        Some((_, k)) => Tok::Kw(*k),
                        None => Tok::Ident(word),
                    };
                    self.push(tok, span);
                }
                '@' => {
                    self.bump();
                    if !self.peek().is_some_and(|c| c.is_ascii_alphabetic() || c == '_') {
                        return Err(Error::new(E0001, span, "expected builtin name after `@`"));
                    }
                    let word = self.ident();
                    self.push(Tok::Builtin(word), span);
                }
                c if c.is_ascii_digit() => {
                    let tok = self.number(span)?;
                    self.push(tok, span);
                }
                '"' => {
                    let s = self.string(span)?;
                    self.push(Tok::Str(s), span);
                }
                '\\' if self.peek_at(1) == Some('\\') => {
                    self.bump();
                    self.bump();
                    let mut text = String::new();
                    while let Some(c) = self.peek().filter(|&c| c != '\n' && !(c == '\r' && self.peek_at(1) == Some('\n'))) {
                        text.push(c);
                        self.bump();
                    }
                    self.push(Tok::LineStr(text.into_bytes()), span);
                }
                '\'' => {
                    let c = self.char_lit(span)?;
                    self.push(Tok::Char(c), span);
                }
                _ => {
                    let tok = self.punct(span)?;
                    self.push(tok, span);
                }
            }
        }
        Ok(())
    }

    /// At a newline after a line of a multi-line string: the string goes on
    /// (the next line starts with `\\`), or the next line starts with `,`,
    /// `)`, `]` or `}`, which a line string cannot be followed by on its line.
    fn continues_line_str(&self) -> bool {
        if !matches!(self.toks.last(), Some(Token { tok: Tok::LineStr(_), .. })) {
            return false;
        }
        let mut i = self.pos + 1;
        while matches!(self.chars.get(i), Some(' ' | '\t' | '\r')) {
            i += 1;
        }
        match self.chars.get(i) {
            Some('\\') => self.chars.get(i + 1) == Some(&'\\'),
            Some(',' | ')' | ']' | '}') => true,
            _ => false,
        }
    }

    fn ident(&mut self) -> String {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == '_' {
                s.push(c);
                self.bump();
            } else {
                break;
            }
        }
        s
    }

    fn number(&mut self, span: Span) -> Result<Tok> {
        let radix = match (self.peek(), self.peek_at(1)) {
            (Some('0'), Some('x')) => 16,
            (Some('0'), Some('o')) => 8,
            (Some('0'), Some('b')) => 2,
            _ => 10,
        };
        if radix != 10 {
            self.bump();
            self.bump();
        }
        let mut digits = String::new();
        while let Some(c) = self.peek() {
            if c == '_' {
                self.bump();
            } else if c.is_digit(radix) {
                digits.push(c);
                self.bump();
            } else {
                break;
            }
        }
        let mut is_float = false;
        if radix == 10 {
            if self.peek() == Some('.') && self.peek_at(1).is_some_and(|c| c.is_ascii_digit()) {
                is_float = true;
                digits.push('.');
                self.bump();
                while let Some(c) = self.peek() {
                    if c == '_' {
                        self.bump();
                    } else if c.is_ascii_digit() {
                        digits.push(c);
                        self.bump();
                    } else {
                        break;
                    }
                }
            }
            if matches!(self.peek(), Some('e') | Some('E')) {
                let sign = matches!(self.peek_at(1), Some('+') | Some('-'));
                let d = if sign { self.peek_at(2) } else { self.peek_at(1) };
                if d.is_some_and(|c| c.is_ascii_digit()) {
                    is_float = true;
                    digits.push('e');
                    self.bump();
                    if sign {
                        digits.push(self.bump().unwrap());
                    }
                    while let Some(c) = self.peek().filter(|c| c.is_ascii_digit()) {
                        digits.push(c);
                        self.bump();
                    }
                }
            }
        }
        if self.peek().is_some_and(|c| c.is_ascii_alphanumeric()) {
            return Err(Error::new(E0003, self.here(), "invalid character in number literal"));
        }
        if digits.is_empty() {
            return Err(Error::new(E0003, span, "number literal has no digits"));
        }
        if is_float {
            let v: f64 = digits.parse().map_err(|_| Error::new(E0003, span, "invalid float literal"))?;
            if !v.is_finite() {
                return Err(Error::new(E0003, span, "float literal is out of range for f64"));
            }
            Ok(Tok::Float(v))
        } else {
            let v = u64::from_str_radix(&digits, radix)
                .map_err(|_| Error::new(E0003, span, "integer literal is too large (max 2^64-1)"))?;
            Ok(Tok::Int(v))
        }
    }

    /// Reads one (possibly escaped) character inside a string or char literal.
    fn escaped(&mut self, start: Span) -> Result<u32> {
        let esc_span = self.here();
        match self.bump() {
            None | Some('\n') => Err(Error::new(E0004, start, "unterminated literal")),
            Some('\\') => {
                let c = self.bump().ok_or_else(|| Error::new(E0004, start, "unterminated literal"))?;
                Ok(match c {
                    'n' => 10,
                    't' => 9,
                    'r' => 13,
                    '0' => 0,
                    '\\' => '\\' as u32,
                    '"' => '"' as u32,
                    '\'' => '\'' as u32,
                    'x' => {
                        let mut v = 0;
                        for _ in 0..2 {
                            let d = self
                                .peek()
                                .and_then(|c| c.to_digit(16))
                                .ok_or_else(|| Error::new(E0005, esc_span, "`\\x` escape needs two hex digits"))?;
                            self.bump();
                            v = v * 16 + d;
                        }
                        v
                    }
                    _ => return Err(Error::new(E0005, esc_span, format!("unknown escape sequence `\\{c}`"))),
                })
            }
            Some(c) => Ok(c as u32),
        }
    }

    fn string(&mut self, span: Span) -> Result<Vec<u8>> {
        self.bump(); // opening quote
        let mut out = Vec::new();
        loop {
            match self.peek() {
                Some('"') => {
                    self.bump();
                    return Ok(out);
                }
                None | Some('\n') => {
                    let msg = "unterminated string literal (a string ends on its line; write a multi-line string as lines that start with `\\\\`)";
                    return Err(Error::new(E0004, span, msg));
                }
                Some('\\') if self.peek_at(1) == Some('x') => {
                    // `\xHH` is a raw byte, not a code point.
                    let b = self.escaped(span)?;
                    out.push(b as u8);
                }
                _ => {
                    let c = self.escaped(span)?;
                    let ch = char::from_u32(c).unwrap();
                    let mut buf = [0u8; 4];
                    out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                }
            }
        }
    }

    fn char_lit(&mut self, span: Span) -> Result<u32> {
        self.bump(); // opening quote
        if self.peek() == Some('\'') {
            return Err(Error::new(E0004, span, "empty character literal"));
        }
        let c = self.escaped(span)?;
        if self.peek() != Some('\'') {
            return Err(Error::new(E0004, span, "character literal must contain exactly one character"));
        }
        self.bump();
        Ok(c)
    }

    fn punct(&mut self, span: Span) -> Result<Tok> {
        let c = self.bump().unwrap();
        let next = self.peek();
        let take = |lx: &mut Lexer, t: Tok| {
            lx.bump();
            t
        };
        let tok = match (c, next) {
            ('(', _) => Tok::LParen,
            (')', _) => Tok::RParen,
            ('{', _) => Tok::LBrace,
            ('}', _) => Tok::RBrace,
            ('[', _) => Tok::LBracket,
            (']', _) => Tok::RBracket,
            (',', _) => Tok::Comma,
            (':', _) => Tok::Colon,
            ('?', _) => Tok::Question,
            ('.', Some('.')) => {
                self.bump();
                match self.peek() {
                    Some('=') => take(self, Tok::DotDotEq),
                    Some('.') => take(self, Tok::Ellipsis),
                    _ => Tok::DotDot,
                }
            }
            ('.', Some('*')) => take(self, Tok::DotStar),
            ('.', Some('?')) => take(self, Tok::DotQuestion),
            ('.', _) => Tok::Dot,
            ('=', Some('=')) => take(self, Tok::EqEq),
            ('=', Some('>')) => take(self, Tok::FatArrow),
            ('=', _) => Tok::Assign,
            ('!', Some('=')) => take(self, Tok::NotEq),
            ('!', _) => Tok::Bang,
            ('<' | '>', Some(n)) if n == c => {
                self.bump();
                let eq = self.peek() == Some('=');
                match (c, eq) {
                    ('<', false) => Tok::Shl,
                    ('>', false) => Tok::Shr,
                    ('<', true) => take(self, Tok::ShlEq),
                    _ => take(self, Tok::ShrEq),
                }
            }
            ('<', Some('=')) => take(self, Tok::Le),
            ('<', _) => Tok::Lt,
            ('>', Some('=')) => take(self, Tok::Ge),
            ('>', _) => Tok::Gt,
            ('&', Some('&')) => take(self, Tok::AndAnd),
            ('&', Some('=')) => take(self, Tok::AmpEq),
            ('&', _) => Tok::Amp,
            ('|', Some('|')) => take(self, Tok::OrOr),
            ('|', Some('=')) => take(self, Tok::PipeEq),
            ('|', _) => Tok::Pipe,
            ('^', Some('=')) => take(self, Tok::CaretEq),
            ('^', _) => Tok::Caret,
            ('~', _) => Tok::Tilde,
            ('+' | '-' | '*', Some('%')) => {
                self.bump();
                let eq = self.peek() == Some('=');
                if eq {
                    self.bump();
                }
                match (c, eq) {
                    ('+', false) => Tok::PlusPct,
                    ('-', false) => Tok::MinusPct,
                    ('*', false) => Tok::StarPct,
                    ('+', true) => Tok::PlusPctEq,
                    ('-', true) => Tok::MinusPctEq,
                    _ => Tok::StarPctEq,
                }
            }
            ('+', Some('=')) => take(self, Tok::PlusEq),
            ('-', Some('=')) => take(self, Tok::MinusEq),
            ('*', Some('=')) => take(self, Tok::StarEq),
            ('/', Some('=')) => take(self, Tok::SlashEq),
            ('%', Some('=')) => take(self, Tok::PercentEq),
            ('+', _) => Tok::Plus,
            ('-', _) => Tok::Minus,
            ('*', _) => Tok::Star,
            ('/', _) => Tok::Slash,
            ('%', _) => Tok::Percent,
            (';', _) => {
                return Err(Error::new(E0002, span, "unexpected `;` (Aye ends statements at newlines)"));
            }
            ('\\', _) => {
                return Err(Error::new(E0001, span, "unexpected character `\\` (a multi-line string's lines start with `\\\\`)"));
            }
            _ => return Err(Error::new(E0001, span, format!("unexpected character `{c}`"))),
        };
        Ok(tok)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(src: &str) -> Vec<Tok> {
        lex(src).unwrap().into_iter().map(|t| t.tok).collect()
    }

    #[test]
    fn inserts_terminators_like_go() {
        assert_eq!(
            toks("let x = 1\nx += 2 // hi\nreturn\n"),
            vec![
                Tok::Kw(Kw::Let),
                Tok::Ident("x".into()),
                Tok::Assign,
                Tok::Int(1),
                Tok::Semi,
                Tok::Ident("x".into()),
                Tok::PlusEq,
                Tok::Int(2),
                Tok::Semi,
                Tok::Kw(Kw::Return),
                Tok::Semi,
                Tok::Eof,
            ]
        );
    }

    #[test]
    fn terminator_after_unwrap_and_unreachable() {
        let id = |n: &str| Tok::Ident(n.into());
        assert_eq!(
            toks("x.?\nunreachable\ny orelse z"),
            vec![
                id("x"),
                Tok::DotQuestion,
                Tok::Semi,
                Tok::Kw(Kw::Unreachable),
                Tok::Semi,
                id("y"),
                Tok::Kw(Kw::Orelse),
                id("z"),
                Tok::Semi,
                Tok::Eof,
            ]
        );
    }

    #[test]
    fn no_terminator_after_operator_or_brace() {
        assert_eq!(
            toks("a +\nb {\n}"),
            vec![
                Tok::Ident("a".into()),
                Tok::Plus,
                Tok::Ident("b".into()),
                Tok::LBrace,
                Tok::RBrace,
                Tok::Semi,
                Tok::Eof,
            ]
        );
    }

    #[test]
    fn numbers_and_ranges() {
        assert_eq!(
            toks("0..10 1.5 2e3 0xff 1_000 'a' '\\n'"),
            vec![
                Tok::Int(0),
                Tok::DotDot,
                Tok::Int(10),
                Tok::Float(1.5),
                Tok::Float(2000.0),
                Tok::Int(255),
                Tok::Int(1000),
                Tok::Char(97),
                Tok::Char(10),
                Tok::Semi,
                Tok::Eof,
            ]
        );
    }

    #[test]
    fn wrapping_ops_and_strings() {
        assert_eq!(
            toks("a +% b *%= \"hi\\n\""),
            vec![
                Tok::Ident("a".into()),
                Tok::PlusPct,
                Tok::Ident("b".into()),
                Tok::StarPctEq,
                Tok::Str(b"hi\n".to_vec()),
                Tok::Semi,
                Tok::Eof,
            ]
        );
    }

    #[test]
    fn bitwise_and_shift_ops() {
        assert_eq!(
            toks("a << b >>= c &= ~d ^ e |= f ..= g ..."),
            vec![
                Tok::Ident("a".into()),
                Tok::Shl,
                Tok::Ident("b".into()),
                Tok::ShrEq,
                Tok::Ident("c".into()),
                Tok::AmpEq,
                Tok::Tilde,
                Tok::Ident("d".into()),
                Tok::Caret,
                Tok::Ident("e".into()),
                Tok::PipeEq,
                Tok::Ident("f".into()),
                Tok::DotDotEq,
                Tok::Ident("g".into()),
                Tok::Ellipsis,
                Tok::Eof,
            ]
        );
    }

    #[test]
    fn line_strings() {
        let ls = |s: &str| Tok::LineStr(s.as_bytes().to_vec());
        assert_eq!(
            toks("let a =\n  \\\\x \"y\" // z\n  \\\\\r\nf(\\\\w\n)\n"),
            vec![
                Tok::Kw(Kw::Let),
                Tok::Ident("a".into()),
                Tok::Assign,
                ls("x \"y\" // z"),
                ls(""),
                Tok::Semi,
                Tok::Ident("f".into()),
                Tok::LParen,
                ls("w"),
                Tok::RParen,
                Tok::Semi,
                Tok::Eof,
            ]
        );
    }

    #[test]
    fn spans_are_line_col() {
        let t = lex("fn\n  main").unwrap();
        assert_eq!(t[1].span, Span::new(2, 3));
    }

    #[test]
    fn rejects_semicolon() {
        let e = lex("let x = 1;").unwrap_err();
        assert_eq!(e[0].span, Span::new(1, 10));
    }

    #[test]
    fn reports_one_error_per_bad_line() {
        let e = lex("let a = \"\\q\\q\"\nlet b = 1;\nlet c = 2\n").unwrap_err();
        let spans: Vec<Span> = e.iter().map(|e| e.span).collect();
        assert_eq!(spans, vec![Span::new(1, 10), Span::new(2, 10)]);
    }
}
