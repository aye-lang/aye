//! `aye fmt`: the canonical layout of a source file.
//!
//! The formatter works on the token stream, not the AST, and keeps every
//! comment. It formats a file that lexes and whose brackets match, even
//! if it has other errors; otherwise it reports the first problem. It never joins or splits lines: the line breaks of the source
//! decide where statements end (Go's newline rule), so they stay. Per line
//! it sets the indentation (4 spaces per level, from the open brackets and
//! continued expressions) and the spaces between tokens; it collapses runs
//! of blank lines into one and drops blank lines at the start and end of the
//! file. Trailing comments get one space before `//`, or line up when
//! several consecutive lines have one. Tokens are copied as written
//! (`0xff`, `1_000` and string escapes stay).
//!
//! Since only spaces and blank lines change, the formatted file has the
//! same tokens (checked before returning) and thus the same meaning.

use crate::codes::{E0001, E0006};
use crate::diag::{Error, Span};
use crate::lexer::{self, Kw, Tok};

/// The kind of a bracket.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Br {
    /// `(a + b)`, `if (x)`.
    Group,
    /// `f(x)`, `@println(x)`, `Pair(i64)`.
    Call,
    /// The parameters in `fn name(...)`.
    FnParams,
    /// The parameters of a function type `fn(A) R`.
    FnType,
    /// `x[i]`, `x[a..b]`.
    Index,
    /// The `[N]`, `[]` or `[*]` of an array, slice or C pointer type.
    ArrayType,
    /// A block, struct literal, declaration body or pattern.
    Brace,
    /// The `{...}` of an array or slice literal: `[3]u8{1, 2, 3}`.
    ArrayLit,
}

/// What a token is, for spacing and indentation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Role {
    /// An identifier, literal or builtin.
    Operand,
    Keyword,
    /// A prefix operator: `-x`, `!x`, `~x`, `&x`, `*T`, `?T`, `!T`.
    Prefix,
    /// An operator with a space on both sides: `a + b`, `x = 1`, `=>`.
    Binary,
    /// An operator without spaces: `a.b`, `0..n`, `'a'..='z'`, `E!T`, `...`.
    Tight,
    /// `.*`, `.?`
    Postfix,
    Comma,
    Colon,
    Open(Br),
    Close(Br),
    /// The `|` that opens or closes a capture `|v|`.
    CapOpen,
    CapClose,
    /// A statement terminator (a newline); not printed.
    Semi,
}

/// A token with its source text and role.
pub(crate) struct Item {
    pub tok: Tok,
    pub text: String,
    pub line: u32,
    pub col: u32,
    pub role: Role,
}

/// A lexed file: its tokens (with the terminators) and the comment of each
/// line (`comments[line - 1]`, the text from `//` with trailing blanks
/// removed).
pub(crate) struct Scan {
    pub items: Vec<Item>,
    pub comments: Vec<Option<String>>,
}

/// The formatted text of `src`, or its lexical errors or a bracket that
/// does not match.
pub fn format(src: &str) -> Result<String, Vec<Error>> {
    format_mapped(src).map(|(text, _)| text)
}

/// The formatted text of `src` and, for each line of `src`, its line
/// number in the result (a dropped blank line maps to the next line kept).
pub fn format_mapped(src: &str) -> Result<(String, Vec<u32>), Vec<Error>> {
    let scan = scan(src)?;
    balanced(&scan.items)?;
    let (out, map) = layout(&scan);
    // The formatter only changes spaces: the tokens must be the same.
    let before: Vec<Tok> = scan.items.into_iter().map(|i| i.tok).collect();
    let after: Vec<Tok> = scan_tokens(&out)?;
    if before != after {
        return Err(vec![Error::new(E0001, Span::new(1, 1), "internal error: formatting would change the tokens")]);
    }
    Ok((out, map))
}

/// Every `(`, `[` and `{` is closed by the matching bracket.
fn balanced(items: &[Item]) -> Result<(), Vec<Error>> {
    let mut open: Vec<&Item> = Vec::new();
    for it in items {
        let want = match it.tok {
            Tok::LParen | Tok::LBracket | Tok::LBrace => {
                open.push(it);
                continue;
            }
            Tok::RParen => Tok::LParen,
            Tok::RBracket => Tok::LBracket,
            Tok::RBrace => Tok::LBrace,
            _ => continue,
        };
        if open.pop().map(|o| &o.tok) != Some(&want) {
            let msg = format!("`{}` does not match an open bracket (`aye fmt` needs matching brackets)", it.text);
            return Err(vec![Error::new(E0006, Span::new(it.line, it.col), msg)]);
        }
    }
    match open.pop() {
        Some(o) => {
            let msg = format!("`{}` is never closed (`aye fmt` needs matching brackets)", o.text);
            Err(vec![Error::new(E0006, Span::new(o.line, o.col), msg)])
        }
        None => Ok(()),
    }
}

fn scan_tokens(src: &str) -> Result<Vec<Tok>, Vec<Error>> {
    Ok(scan(src)?.items.into_iter().map(|i| i.tok).collect())
}

/// Lex `src` and give each token its text and role.
pub(crate) fn scan(src: &str) -> Result<Scan, Vec<Error>> {
    let toks = lexer::lex(src)?;
    let lines: Vec<Vec<char>> = src.split('\n').map(|l| l.chars().collect()).collect();
    let mut comments: Vec<Option<String>> = vec![None; lines.len()];
    // The column after the last token of each line.
    let mut last_end = vec![1u32; lines.len()];
    let mut items = Vec::new();
    for t in &toks {
        if t.tok == Tok::Eof {
            break;
        }
        let text = if t.tok == Tok::Semi {
            String::new()
        } else {
            let line = &lines[t.span.line as usize - 1];
            last_end[t.span.line as usize - 1] = t.end.col;
            line[t.span.col as usize - 1..t.end.col as usize - 1].iter().collect()
        };
        items.push(Item { tok: t.tok.clone(), text, line: t.span.line, col: t.span.col, role: Role::Semi });
    }
    for (i, line) in lines.iter().enumerate() {
        let rest: String = line[(last_end[i] as usize - 1).min(line.len())..].iter().collect();
        let rest = rest.trim();
        if rest.starts_with("//") {
            comments[i] = Some(rest.to_string());
        }
    }
    classify(&mut items);
    Ok(Scan { items, comments })
}

/// Does a token of this role end an operand, so that a following `-`, `*`
/// or `&` is binary and a `(` or `[` is a call or an index?
fn ends_operand(r: Option<Role>) -> bool {
    match r {
        Some(Role::Operand | Role::Postfix) => true,
        Some(Role::Close(b)) => !matches!(b, Br::FnParams | Br::FnType | Br::ArrayType),
        _ => false,
    }
}

fn classify(items: &mut [Item]) {
    // Open brackets: their kind, and `array_type` when they opened.
    let mut stack: Vec<(Br, bool)> = Vec::new();
    // After the `]` of `[N]T` or `[]T` (a type is being written), a `{`
    // starts an array literal.
    let mut array_type = false;
    // Between the `)` of `fn name(...)` and the body: a `{` is the body.
    let mut signature = false;
    let mut capture_close = None;
    // In a `for` header before `in`, which is a keyword there.
    let mut for_header = false;
    for i in 0..items.len() {
        let prev = i.checked_sub(1).map(|j| items[j].role);
        let prev_tok = i.checked_sub(1).map(|j| &items[j].tok);
        let prev2_tok = i.checked_sub(2).map(|j| &items[j].tok);
        let operand = ends_operand(prev);
        let tok_at = |n: usize| items.get(i + n).map(|it| &it.tok);
        let role = match &items[i].tok {
            Tok::Semi => {
                signature = false;
                Role::Semi
            }
            Tok::Ident(n) if for_header && n == "in" => {
                for_header = false;
                Role::Keyword
            }
            Tok::Kw(Kw::For) => {
                for_header = true;
                Role::Keyword
            }
            // `simd[N]T`: the `[N]` is a type's, as in `[N]T`.
            Tok::Ident(n) if n == "simd" && tok_at(1) == Some(&Tok::LBracket) => Role::Prefix,
            Tok::Ident(_) | Tok::Builtin(_) | Tok::Int(_) | Tok::Float(_) | Tok::Str(_) | Tok::LineStr(_) | Tok::Char(_) => {
                Role::Operand
            }
            Tok::Kw(_) => Role::Keyword,
            Tok::Comma => Role::Comma,
            Tok::Colon => Role::Colon,
            // `x is .V`: the `.` starts the pattern.
            Tok::Dot if prev_tok == Some(&Tok::Kw(Kw::Is)) => Role::Prefix,
            Tok::Dot | Tok::DotDot | Tok::DotDotEq | Tok::Ellipsis => Role::Tight,
            Tok::DotStar | Tok::DotQuestion => Role::Postfix,
            Tok::Minus | Tok::Amp | Tok::Star if operand => Role::Binary,
            Tok::Minus | Tok::Amp | Tok::Star | Tok::Tilde | Tok::Question => Role::Prefix,
            Tok::Bang if operand => Role::Tight,
            Tok::Bang => Role::Prefix,
            Tok::Pipe if capture_close == Some(i) => {
                capture_close = None;
                Role::CapClose
            }
            Tok::Pipe
                if prev_tok == Some(&Tok::Kw(Kw::Catch))
                    || (matches!(tok_at(1), Some(Tok::Ident(_)))
                        && tok_at(2) == Some(&Tok::Pipe)
                        && tok_at(3) == Some(&Tok::LBrace)) =>
            {
                capture_close = Some(i + 2);
                Role::CapOpen
            }
            Tok::LParen => {
                let kind = if prev_tok == Some(&Tok::Kw(Kw::Fn)) {
                    Br::FnType
                } else if operand && prev2_tok == Some(&Tok::Kw(Kw::Fn)) {
                    Br::FnParams
                } else if operand {
                    Br::Call
                } else {
                    Br::Group
                };
                stack.push((kind, array_type));
                Role::Open(kind)
            }
            Tok::LBracket => {
                let kind = if operand { Br::Index } else { Br::ArrayType };
                stack.push((kind, array_type));
                Role::Open(kind)
            }
            Tok::LBrace => {
                let kind = if array_type && !signature { Br::ArrayLit } else { Br::Brace };
                signature = false;
                stack.push((kind, false));
                Role::Open(kind)
            }
            Tok::RParen | Tok::RBracket | Tok::RBrace => {
                let (kind, saved) = stack.pop().unwrap_or((Br::Group, false));
                if kind == Br::FnParams {
                    signature = true;
                }
                if matches!(items[i].tok, Tok::RParen) {
                    array_type = saved;
                }
                Role::Close(kind)
            }
            _ => Role::Binary,
        };
        items[i].role = role;
        // Does this token keep a type going (so a later `{` is an array
        // literal's)?
        array_type = match (&items[i].tok, role) {
            (_, Role::Close(Br::ArrayType)) => true,
            (Tok::RParen, _) => array_type,
            (Tok::Ident(_) | Tok::Dot | Tok::Kw(Kw::Mut | Kw::Fn | Kw::Dyn), _) => array_type,
            (_, Role::Prefix | Role::Open(Br::ArrayType)) => array_type,
            (Tok::Bang, _) => array_type,
            _ => false,
        };
    }
}

/// Is there a space between two tokens of these roles on one line?
pub(crate) fn space(a: Role, b: Role) -> bool {
    use Role::*;
    match (a, b) {
        (_, Comma | Colon | Semi) => false,
        (Open(Br::Brace), Close(Br::Brace)) => false,
        (_, Close(Br::Brace)) => true,
        (_, Close(_)) => false,
        (Open(Br::Brace), _) => true,
        (Open(_), _) => false,
        (Comma | Colon, _) => true,
        (_, Tight | Postfix) => false,
        (Tight | Prefix | Close(Br::ArrayType), _) => false,
        (_, Open(Br::Call | Br::FnParams | Br::FnType | Br::Index | Br::ArrayLit)) => false,
        (CapOpen, _) | (_, CapClose) => false,
        _ => true,
    }
}

/// The tokens joined on one line with canonical spaces.
pub(crate) fn join<'a>(items: impl IntoIterator<Item = &'a Item>) -> String {
    let mut out = String::new();
    let mut prev: Option<Role> = None;
    for it in items {
        if it.role == Role::Semi {
            continue;
        }
        if prev.is_some_and(|p| space(p, it.role)) {
            out.push(' ');
        }
        out.push_str(&it.text);
        prev = Some(it.role);
    }
    out
}

/// One output line: indentation, code and trailing comment.
struct Line {
    indent: usize,
    code: String,
    comment: Option<String>,
}

fn layout(scan: &Scan) -> (String, Vec<u32>) {
    let items = &scan.items;
    let nlines = scan.comments.len();
    // The items of each line (terminators excluded).
    let mut by_line: Vec<Vec<usize>> = vec![Vec::new(); nlines];
    for (i, it) in items.iter().enumerate() {
        if it.role != Role::Semi {
            by_line[it.line as usize - 1].push(i);
        }
    }
    // The base indentation of each open bracket: the closing line's.
    let mut bases: Vec<usize> = Vec::new();
    // The previous code line continues its statement on the next line.
    let mut cont = false;
    // The indentation of the previous code line when it ends with a line of
    // a multi-line string: the string's next line, or the `,` after it, is
    // indented like it.
    let mut line_str: Option<usize> = None;
    let mut lines: Vec<Option<Line>> = Vec::new();
    for (l, idx) in by_line.iter().enumerate() {
        let comment = scan.comments[l].clone();
        let inner = bases.last().map_or(0, |b| b + 1);
        let Some(&first) = idx.first() else {
            lines.push(comment.map(|c| Line { indent: inner + cont as usize, code: String::new(), comment: Some(c) }));
            continue;
        };
        let continued = cont && !matches!(items[first].role, Role::Close(_));
        let indent = match (items[first].role, line_str) {
            (Role::Close(_), _) => bases.last().copied().unwrap_or(0),
            (Role::Operand | Role::Comma, Some(n)) if matches!(items[first].tok, Tok::LineStr(_) | Tok::Comma) => n,
            _ => inner + continued as usize,
        };
        for &i in idx {
            match items[i].role {
                // A block opened on a continued line (`if a &&\n b {`) is
                // indented from the statement, as in Go.
                Role::Open(Br::Brace) => bases.push(indent - continued as usize),
                Role::Open(_) => bases.push(indent),
                Role::Close(_) => {
                    bases.pop();
                }
                _ => {}
            }
        }
        let last = *idx.last().unwrap();
        let next_ends = items.get(last + 1).is_none_or(|n| n.role == Role::Semi);
        cont = !next_ends && !matches!(items[last].role, Role::Open(_) | Role::Comma);
        line_str = matches!(items[last].tok, Tok::LineStr(_)).then_some(indent);
        let code = join(idx.iter().map(|&i| &items[i]));
        lines.push(Some(Line { indent, code, comment }));
    }
    align_comments(&mut lines);
    // Blank lines: none at the start or end, at most one in a row. A
    // blank line maps to the next line kept (or the last line).
    let mut out = String::new();
    let mut map: Vec<Option<u32>> = Vec::with_capacity(nlines);
    let mut n: u32 = 0;
    let mut blank = false;
    for line in &lines {
        let Some(line) = line else {
            blank = n > 0;
            map.push(None);
            continue;
        };
        if blank {
            out.push('\n');
            n += 1;
            blank = false;
        }
        n += 1;
        map.push(Some(n));
        out.push_str(&"    ".repeat(line.indent));
        out.push_str(&line.code);
        if let Some(c) = &line.comment {
            if !line.code.is_empty() {
                out.push(' ');
            }
            out.push_str(c);
        }
        out.push('\n');
    }
    let mut next = n.max(1);
    let mut map: Vec<u32> = map
        .into_iter()
        .rev()
        .map(|m| {
            next = m.unwrap_or(next);
            next
        })
        .collect();
    map.reverse();
    (out, map)
}

/// Consecutive lines with code and a trailing comment put their comments
/// in one column, one space after the longest line's code. Padding goes
/// into `code`, so the single space added later completes it.
fn align_comments(lines: &mut [Option<Line>]) {
    let has = |l: &Option<Line>| l.as_ref().is_some_and(|l| !l.code.is_empty() && l.comment.is_some());
    let width = |l: &Line| 4 * l.indent + l.code.chars().count();
    let mut i = 0;
    while i < lines.len() {
        let mut j = i;
        while j < lines.len() && has(&lines[j]) {
            j += 1;
        }
        if j - i >= 2 {
            let w = lines[i..j].iter().flatten().map(width).max().unwrap();
            for l in lines[i..j].iter_mut().flatten() {
                let pad = w - width(l);
                l.code.push_str(&" ".repeat(pad));
            }
        }
        i = j.max(i + 1);
    }
}

#[cfg(test)]
mod tests {
    use super::format;

    fn f(src: &str) -> String {
        format(src).unwrap()
    }

    #[test]
    fn spaces_and_indentation() {
        assert_eq!(f("fn  f( a:i64 ,b : []u8 )?*mut T{\nreturn -a+b[1..]\n}\n"), "fn f(a: i64, b: []u8) ?*mut T {\n    return -a + b[1..]\n}\n");
        assert_eq!(f("let x = [3]u8{ 1,2,3 }\nlet p = Point{x:1}\n"), "let x = [3]u8{1, 2, 3}\nlet p = Point { x: 1 }\n");
        assert_eq!(f("fn f() !void {\nif x |v| {\n}\nlet y = g() catch |e| 3 | 4\n}\n"), "fn f() !void {\n    if x |v| {\n    }\n    let y = g() catch |e| 3 | 4\n}\n");
    }

    #[test]
    fn blank_lines_and_line_map() {
        let (out, map) = super::format_mapped("\n\nlet a = 1\n\n\n\nlet b = 2\n\n").unwrap();
        assert_eq!(out, "let a = 1\n\nlet b = 2\n");
        assert_eq!(map, vec![1, 1, 1, 3, 3, 3, 3, 3, 3]);
    }

    #[test]
    fn idempotent_on_odd_input() {
        let src = "fn f(a: i64,\nb: i64) i64 {\nreturn a +\nb *\n(a -\nb)\n}\n";
        let once = f(src);
        assert_eq!(f(&once), once);
    }
}
