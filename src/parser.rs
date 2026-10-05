//! Recursive-descent parser producing the AST in `ast.rs`.
//! The grammar it accepts is documented in `docs/grammar.ebnf`.

use crate::ast::*;
use crate::diag::{Error, Result, Span};
use crate::lexer::{Kw, Tok, Token};

pub fn parse(tokens: Vec<Token>) -> Result<File> {
    let mut p = Parser { toks: tokens, pos: 0, no_struct_lit: false };
    p.file()
}

struct Parser {
    toks: Vec<Token>,
    pos: usize,
    /// Set while parsing `if`/`while`/`for` headers, where `Name {` starts the
    /// body rather than a struct literal (same rule as Go).
    no_struct_lit: bool,
}

fn not_supported(kw: Kw, span: Span) -> Error {
    let what = match kw {
        Kw::Import => "`import` (modules)",
        Kw::Enum => "`enum`",
        Kw::Error => "`error` sets",
        Kw::Interface => "`interface`",
        Kw::Dyn => "`dyn`",
        Kw::Match => "`match`",
        Kw::Errdefer => "`errdefer`",
        Kw::Try => "`try`",
        Kw::Catch => "`catch`",
        Kw::Comptime => "`comptime`",
        Kw::Where => "`where`",
        Kw::Test => "`test` blocks",
        Kw::Mut => "`mut`",
        _ => return Error::new(span, format!("unexpected `{}`", kw.as_str())),
    };
    Error::new(span, format!("{what} is not yet supported"))
}

fn is_unsupported_kw(kw: Kw) -> bool {
    matches!(
        kw,
        Kw::Import
            | Kw::Enum
            | Kw::Error
            | Kw::Interface
            | Kw::Dyn
            | Kw::Match
            | Kw::Errdefer
            | Kw::Try
            | Kw::Catch
            | Kw::Comptime
            | Kw::Where
            | Kw::Test
            | Kw::Mut
    )
}

impl Parser {
    fn peek(&self) -> &Tok {
        &self.toks[self.pos].tok
    }

    fn span(&self) -> Span {
        self.toks[self.pos].span
    }

    fn bump(&mut self) -> Token {
        let t = self.toks[self.pos].clone();
        if self.pos < self.toks.len() - 1 {
            self.pos += 1;
        }
        t
    }

    fn at(&self, t: &Tok) -> bool {
        self.peek() == t
    }

    fn eat(&mut self, t: &Tok) -> bool {
        if self.at(t) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn unexpected(&self, expected: &str) -> Error {
        Error::new(self.span(), format!("expected {expected}, found {}", self.peek()))
    }

    fn expect(&mut self, t: &Tok) -> Result<Span> {
        if self.at(t) {
            Ok(self.bump().span)
        } else {
            Err(self.unexpected(&t.to_string()))
        }
    }

    fn ident(&mut self) -> Result<(String, Span)> {
        match self.peek().clone() {
            Tok::Ident(name) => {
                let span = self.bump().span;
                Ok((name, span))
            }
            _ => Err(self.unexpected("identifier")),
        }
    }

    fn skip_semis(&mut self) {
        while self.eat(&Tok::Semi) {}
    }

    /// A declaration or statement must be followed by a newline (or `}`/EOF).
    fn end_of_stmt(&mut self) -> Result<()> {
        match self.peek() {
            Tok::Semi => {
                self.bump();
                Ok(())
            }
            Tok::RBrace | Tok::Eof => Ok(()),
            _ => Err(self.unexpected("newline after statement")),
        }
    }

    // ---------------------------------------------------------------- items

    fn file(&mut self) -> Result<File> {
        let mut items = Vec::new();
        loop {
            self.skip_semis();
            if self.at(&Tok::Eof) {
                return Ok(File { items });
            }
            items.push(self.item()?);
            self.end_of_stmt()?;
        }
    }

    fn item(&mut self) -> Result<Item> {
        let is_pub = self.eat(&Tok::Kw(Kw::Pub));
        match self.peek().clone() {
            Tok::Kw(Kw::Fn) => Ok(Item::Fn(self.fn_decl(is_pub, false)?)),
            Tok::Kw(Kw::Extern) => {
                self.bump();
                match self.peek() {
                    Tok::Kw(Kw::Fn) => Ok(Item::Fn(self.fn_decl(is_pub, true)?)),
                    Tok::Kw(Kw::Struct) => {
                        Err(Error::new(self.span(), "`extern struct` is not yet supported"))
                    }
                    _ => Err(self.unexpected("`fn` after `extern`")),
                }
            }
            Tok::Kw(Kw::Struct) => Ok(Item::Struct(self.struct_decl(is_pub)?)),
            Tok::Kw(Kw::Let) | Tok::Kw(Kw::Var) => {
                Err(Error::new(self.span(), "global variables are not yet supported"))
            }
            Tok::Kw(k) if is_unsupported_kw(k) => Err(not_supported(k, self.span())),
            _ => Err(self.unexpected("declaration (`fn`, `extern fn` or `struct`)")),
        }
    }

    fn fn_decl(&mut self, is_pub: bool, is_extern: bool) -> Result<FnDecl> {
        self.expect(&Tok::Kw(Kw::Fn))?;
        let (name, span) = self.ident()?;
        self.expect(&Tok::LParen)?;
        let mut params = Vec::new();
        while !self.at(&Tok::RParen) {
            let (pname, pspan) = self.ident()?;
            self.expect(&Tok::Colon)?;
            let ty = self.type_expr()?;
            params.push(Param { name: pname, span: pspan, ty });
            if !self.eat(&Tok::Comma) {
                break;
            }
        }
        self.expect(&Tok::RParen)?;
        let ret = match self.peek() {
            Tok::LBrace | Tok::Semi | Tok::Eof => None,
            _ => Some(self.type_expr()?),
        };
        let body = if is_extern {
            if self.at(&Tok::LBrace) {
                return Err(Error::new(self.span(), "`extern fn` cannot have a body"));
            }
            None
        } else {
            if !self.at(&Tok::LBrace) {
                return Err(self.unexpected("`{` to start function body"));
            }
            Some(self.block()?)
        };
        Ok(FnDecl { name, span, is_pub, is_extern, params, ret, body })
    }

    fn struct_decl(&mut self, is_pub: bool) -> Result<StructDecl> {
        self.expect(&Tok::Kw(Kw::Struct))?;
        let (name, span) = self.ident()?;
        self.expect(&Tok::LBrace)?;
        let mut fields = Vec::new();
        loop {
            self.skip_semis();
            if self.eat(&Tok::RBrace) {
                break;
            }
            let (fname, fspan) = self.ident()?;
            self.expect(&Tok::Colon)?;
            let ty = self.type_expr()?;
            fields.push(FieldDecl { name: fname, span: fspan, ty });
            match self.peek() {
                Tok::Comma | Tok::Semi => {
                    self.bump();
                }
                Tok::RBrace => {}
                _ => return Err(self.unexpected("`,`, newline or `}` after struct field")),
            }
        }
        Ok(StructDecl { name, span, is_pub, fields })
    }

    // ---------------------------------------------------------------- types

    fn type_expr(&mut self) -> Result<TypeExpr> {
        let span = self.span();
        let kind = match self.peek().clone() {
            Tok::Ident(name) => {
                self.bump();
                TypeExprKind::Named(name)
            }
            Tok::LBracket => {
                self.bump();
                match self.peek().clone() {
                    Tok::RBracket => {
                        self.bump();
                        TypeExprKind::Slice(Box::new(self.type_expr()?))
                    }
                    Tok::Star => {
                        self.bump();
                        self.expect(&Tok::RBracket)?;
                        TypeExprKind::CPtr(Box::new(self.type_expr()?))
                    }
                    Tok::Int(n) => {
                        self.bump();
                        self.expect(&Tok::RBracket)?;
                        TypeExprKind::Array(n, Box::new(self.type_expr()?))
                    }
                    _ => return Err(self.unexpected("array length (integer literal), `]` or `*`")),
                }
            }
            Tok::Star => return Err(Error::new(span, "pointer types are not yet supported")),
            Tok::Question => return Err(Error::new(span, "optional types are not yet supported")),
            Tok::Bang => return Err(Error::new(span, "error union types are not yet supported")),
            Tok::Kw(k) if is_unsupported_kw(k) => return Err(not_supported(k, span)),
            Tok::Kw(Kw::Struct) => {
                return Err(Error::new(span, "anonymous struct types are not yet supported"));
            }
            _ => return Err(self.unexpected("type")),
        };
        Ok(TypeExpr { kind, span })
    }

    // ----------------------------------------------------------- statements

    fn block(&mut self) -> Result<Block> {
        let span = self.expect(&Tok::LBrace)?;
        let saved = std::mem::replace(&mut self.no_struct_lit, false);
        let mut stmts = Vec::new();
        loop {
            self.skip_semis();
            if self.eat(&Tok::RBrace) {
                break;
            }
            if self.at(&Tok::Eof) {
                return Err(self.unexpected("`}`"));
            }
            stmts.push(self.stmt()?);
            self.end_of_stmt()?;
        }
        self.no_struct_lit = saved;
        Ok(Block { stmts, span })
    }

    fn stmt(&mut self) -> Result<Stmt> {
        let span = self.span();
        let kind = match self.peek().clone() {
            Tok::Kw(kw @ (Kw::Let | Kw::Var)) => {
                self.bump();
                let (name, _) = self.ident()?;
                let ty = if self.eat(&Tok::Colon) { Some(self.type_expr()?) } else { None };
                if !self.at(&Tok::Assign) {
                    return Err(Error::new(
                        self.span(),
                        format!("`{} {name}` needs an initializer (`= value`)", kw.as_str()),
                    ));
                }
                self.bump();
                let init = self.expr()?;
                StmtKind::Let { name, mutable: kw == Kw::Var, ty, init }
            }
            Tok::Kw(Kw::If) => return self.if_stmt(),
            Tok::Kw(Kw::Else) => {
                return Err(Error::new(
                    span,
                    "`else` must be on the same line as the `}` that closes the `if` block",
                ));
            }
            Tok::Kw(Kw::While) => {
                self.bump();
                let cond = self.header_expr()?;
                let body = self.block()?;
                StmtKind::While { cond, body }
            }
            Tok::Kw(Kw::For) => {
                self.bump();
                let (var, var_span) = self.ident()?;
                match self.peek() {
                    Tok::Ident(w) if w == "in" => {
                        self.bump();
                    }
                    _ => return Err(self.unexpected("`in`")),
                }
                let lo = self.header_expr()?;
                self.expect(&Tok::DotDot)?;
                let hi = self.header_expr()?;
                let body = self.block()?;
                StmtKind::For { var, var_span, lo, hi, body }
            }
            Tok::Kw(Kw::Break) => {
                self.bump();
                StmtKind::Break
            }
            Tok::Kw(Kw::Continue) => {
                self.bump();
                StmtKind::Continue
            }
            Tok::Kw(Kw::Return) => {
                self.bump();
                if matches!(self.peek(), Tok::Semi | Tok::RBrace) {
                    StmtKind::Return(None)
                } else {
                    StmtKind::Return(Some(self.expr()?))
                }
            }
            Tok::Kw(Kw::Defer) => {
                self.bump();
                let inner_span = self.span();
                let inner = if self.at(&Tok::LBrace) {
                    Stmt { kind: StmtKind::Block(self.block()?), span: inner_span }
                } else {
                    self.simple_stmt()?
                };
                StmtKind::Defer(Box::new(inner))
            }
            Tok::LBrace => StmtKind::Block(self.block()?),
            Tok::Kw(k) if is_unsupported_kw(k) => return Err(not_supported(k, span)),
            Tok::Kw(Kw::Fn) | Tok::Kw(Kw::Struct) | Tok::Kw(Kw::Pub) | Tok::Kw(Kw::Extern) => {
                return Err(Error::new(span, "declarations are only allowed at the top level"));
            }
            _ => return self.simple_stmt(),
        };
        Ok(Stmt { kind, span })
    }

    fn if_stmt(&mut self) -> Result<Stmt> {
        let span = self.expect(&Tok::Kw(Kw::If))?;
        let cond = self.header_expr()?;
        let then = self.block()?;
        let els = if self.eat(&Tok::Kw(Kw::Else)) {
            let else_span = self.span();
            match self.peek() {
                Tok::Kw(Kw::If) => Some(Box::new(self.if_stmt()?)),
                Tok::LBrace => {
                    Some(Box::new(Stmt { kind: StmtKind::Block(self.block()?), span: else_span }))
                }
                _ => return Err(self.unexpected("`if` or `{` after `else`")),
            }
        } else {
            None
        };
        Ok(Stmt { kind: StmtKind::If { cond, then, els }, span })
    }

    /// Expression statement or assignment.
    fn simple_stmt(&mut self) -> Result<Stmt> {
        let span = self.span();
        let target = self.expr()?;
        let op = match self.peek() {
            Tok::Assign => AssignOp::Set,
            Tok::PlusEq => AssignOp::Op(BinOp::Add),
            Tok::MinusEq => AssignOp::Op(BinOp::Sub),
            Tok::StarEq => AssignOp::Op(BinOp::Mul),
            Tok::SlashEq => AssignOp::Op(BinOp::Div),
            Tok::PercentEq => AssignOp::Op(BinOp::Rem),
            Tok::PlusPctEq => AssignOp::Op(BinOp::AddWrap),
            Tok::MinusPctEq => AssignOp::Op(BinOp::SubWrap),
            Tok::StarPctEq => AssignOp::Op(BinOp::MulWrap),
            _ => return Ok(Stmt { kind: StmtKind::Expr(target), span }),
        };
        let op_span = self.bump().span;
        let value = self.expr()?;
        Ok(Stmt { kind: StmtKind::Assign { target, op, value }, span: op_span })
    }

    // ---------------------------------------------------------- expressions

    /// Expression in an `if`/`while`/`for` header: no bare struct literals.
    fn header_expr(&mut self) -> Result<Expr> {
        let saved = std::mem::replace(&mut self.no_struct_lit, true);
        let e = self.expr();
        self.no_struct_lit = saved;
        e
    }

    fn expr(&mut self) -> Result<Expr> {
        self.binary(1)
    }

    fn binop(&self) -> Option<(BinOp, u8)> {
        Some(match self.peek() {
            Tok::OrOr => (BinOp::Or, 1),
            Tok::AndAnd => (BinOp::And, 2),
            Tok::EqEq => (BinOp::Eq, 3),
            Tok::NotEq => (BinOp::Ne, 3),
            Tok::Lt => (BinOp::Lt, 3),
            Tok::Le => (BinOp::Le, 3),
            Tok::Gt => (BinOp::Gt, 3),
            Tok::Ge => (BinOp::Ge, 3),
            Tok::Plus => (BinOp::Add, 4),
            Tok::Minus => (BinOp::Sub, 4),
            Tok::PlusPct => (BinOp::AddWrap, 4),
            Tok::MinusPct => (BinOp::SubWrap, 4),
            Tok::Star => (BinOp::Mul, 5),
            Tok::Slash => (BinOp::Div, 5),
            Tok::Percent => (BinOp::Rem, 5),
            Tok::StarPct => (BinOp::MulWrap, 5),
            _ => return None,
        })
    }

    /// Precedence climbing; all binary operators are left-associative except
    /// comparisons, which do not chain.
    fn binary(&mut self, min_prec: u8) -> Result<Expr> {
        let mut lhs = self.cast()?;
        while let Some((op, prec)) = self.binop() {
            if prec < min_prec {
                break;
            }
            let span = self.bump().span;
            let rhs = self.binary(prec + 1)?;
            lhs = Expr { kind: ExprKind::Binary { op, lhs: Box::new(lhs), rhs: Box::new(rhs) }, span };
            if op.is_comparison() && self.binop().is_some_and(|(o, _)| o.is_comparison()) {
                return Err(Error::new(self.span(), "comparison operators cannot be chained"));
            }
        }
        Ok(lhs)
    }

    fn cast(&mut self) -> Result<Expr> {
        let mut e = self.unary()?;
        while self.at(&Tok::Kw(Kw::As)) {
            let span = self.bump().span;
            let ty = self.type_expr()?;
            e = Expr { kind: ExprKind::Cast { expr: Box::new(e), ty }, span };
        }
        Ok(e)
    }

    fn unary(&mut self) -> Result<Expr> {
        let op = match self.peek() {
            Tok::Minus => UnOp::Neg,
            Tok::Bang => UnOp::Not,
            _ => return self.postfix(),
        };
        let span = self.bump().span;
        let expr = self.unary()?;
        Ok(Expr { kind: ExprKind::Unary { op, expr: Box::new(expr) }, span })
    }

    fn postfix(&mut self) -> Result<Expr> {
        let mut e = self.primary()?;
        loop {
            match self.peek() {
                Tok::Dot => {
                    self.bump();
                    let (name, span) = self.ident()?;
                    e = Expr { kind: ExprKind::Field { base: Box::new(e), name }, span };
                }
                Tok::LBracket => {
                    let span = self.bump().span;
                    let saved = std::mem::replace(&mut self.no_struct_lit, false);
                    let lo = if self.at(&Tok::DotDot) { None } else { Some(Box::new(self.expr()?)) };
                    if self.eat(&Tok::DotDot) {
                        let hi =
                            if self.at(&Tok::RBracket) { None } else { Some(Box::new(self.expr()?)) };
                        self.expect(&Tok::RBracket)?;
                        e = Expr { kind: ExprKind::Slice { base: Box::new(e), lo, hi }, span };
                    } else {
                        self.expect(&Tok::RBracket)?;
                        let index = lo.unwrap();
                        e = Expr { kind: ExprKind::Index { base: Box::new(e), index }, span };
                    }
                    self.no_struct_lit = saved;
                }
                _ => return Ok(e),
            }
        }
    }

    fn args(&mut self) -> Result<Vec<Expr>> {
        self.expect(&Tok::LParen)?;
        let saved = std::mem::replace(&mut self.no_struct_lit, false);
        let mut args = Vec::new();
        while !self.at(&Tok::RParen) {
            args.push(self.expr()?);
            if !self.eat(&Tok::Comma) {
                break;
            }
        }
        if !self.at(&Tok::RParen) {
            return Err(self.unexpected("`,` or `)`"));
        }
        self.bump();
        self.no_struct_lit = saved;
        Ok(args)
    }

    fn primary(&mut self) -> Result<Expr> {
        let span = self.span();
        let kind = match self.peek().clone() {
            Tok::Int(v) => {
                self.bump();
                ExprKind::Int(v)
            }
            Tok::Float(v) => {
                self.bump();
                ExprKind::Float(v)
            }
            Tok::Str(s) => {
                self.bump();
                ExprKind::Str(s)
            }
            Tok::Char(c) => {
                self.bump();
                ExprKind::Char(c)
            }
            Tok::Ident(name) => {
                self.bump();
                match self.peek() {
                    _ if name == "true" => ExprKind::Bool(true),
                    _ if name == "false" => ExprKind::Bool(false),
                    Tok::LParen => ExprKind::Call { name, args: self.args()? },
                    Tok::LBrace if !self.no_struct_lit => self.struct_lit(name)?,
                    _ => ExprKind::Ident(name),
                }
            }
            Tok::Builtin(name) => {
                self.bump();
                ExprKind::Builtin { name, args: self.args()? }
            }
            Tok::LParen => {
                self.bump();
                let saved = std::mem::replace(&mut self.no_struct_lit, false);
                let inner = self.expr()?;
                self.no_struct_lit = saved;
                self.expect(&Tok::RParen)?;
                ExprKind::Paren(Box::new(inner))
            }
            Tok::LBracket => {
                let ty = self.type_expr()?;
                if !matches!(ty.kind, TypeExprKind::Array(..)) {
                    return Err(Error::new(span, "only fixed-size array literals (`[N]T{...}`) are supported"));
                }
                self.expect(&Tok::LBrace)?;
                let saved = std::mem::replace(&mut self.no_struct_lit, false);
                let mut elems = Vec::new();
                while !self.at(&Tok::RBrace) {
                    elems.push(self.expr()?);
                    if !self.eat(&Tok::Comma) {
                        break;
                    }
                }
                if !self.at(&Tok::RBrace) {
                    return Err(self.unexpected("`,` or `}` (multi-line literals need a trailing comma)"));
                }
                self.bump();
                self.no_struct_lit = saved;
                ExprKind::ArrayLit { ty, elems }
            }
            Tok::Kw(k) if is_unsupported_kw(k) => return Err(not_supported(k, span)),
            _ => return Err(self.unexpected("expression")),
        };
        Ok(Expr { kind, span })
    }

    fn struct_lit(&mut self, name: String) -> Result<ExprKind> {
        self.expect(&Tok::LBrace)?;
        let mut fields = Vec::new();
        while !self.at(&Tok::RBrace) {
            let (fname, fspan) = self.ident()?;
            self.expect(&Tok::Colon)?;
            let value = self.expr()?;
            fields.push(FieldInit { name: fname, span: fspan, value });
            if !self.eat(&Tok::Comma) {
                break;
            }
        }
        if !self.at(&Tok::RBrace) {
            return Err(self.unexpected("`,` or `}` (multi-line literals need a trailing comma)"));
        }
        self.bump();
        Ok(ExprKind::StructLit { name, fields })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;

    fn parse_src(src: &str) -> Result<File> {
        parse(lex(src).unwrap())
    }

    #[test]
    fn parses_functions_and_structs() {
        let f = parse_src(
            "struct P { x: i32, y: i32 }\nstruct Q {\n  a: [3]u8\n  b: []i32\n}\n\
             extern fn puts(s: [*]u8) i32\npub fn main() {\n  let p = P { x: 1, y: 2 }\n}\n",
        )
        .unwrap();
        assert_eq!(f.items.len(), 4);
    }

    #[test]
    fn struct_literal_not_allowed_in_if_header() {
        let f = parse_src("fn f(x: i32) {\n if x == y { return }\n}").unwrap();
        assert_eq!(f.items.len(), 1);
    }

    #[test]
    fn missing_newline_between_statements() {
        let e = parse_src("fn f() {\n  let x = 1 let y = 2\n}").unwrap_err();
        assert_eq!(e.msg, "expected newline after statement, found `let`");
        assert_eq!(e.span, Span::new(2, 13));
    }

    #[test]
    fn else_on_next_line_is_an_error() {
        let e = parse_src("fn f() {\n if true {\n }\n else {\n }\n}").unwrap_err();
        assert!(e.msg.contains("same line"), "{}", e.msg);
    }

    #[test]
    fn unsupported_keywords() {
        let e = parse_src("enum E { A }").unwrap_err();
        assert_eq!(e.msg, "`enum` is not yet supported");
        let e = parse_src("fn f() {\n match x {}\n}").unwrap_err();
        assert_eq!(e.msg, "`match` is not yet supported");
    }

    #[test]
    fn chained_comparison_rejected() {
        let e = parse_src("fn f() {\n let b = 1 < 2 < 3\n}").unwrap_err();
        assert_eq!(e.msg, "comparison operators cannot be chained");
    }
}
