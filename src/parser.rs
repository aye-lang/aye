//! Recursive-descent parser producing the AST in `ast.rs`.
//! The grammar it accepts is documented in `docs/grammar.ebnf`.
//!
//! Error recovery: a statement that fails to parse is reported and skipped up
//! to the end of its line (or the `}` closing its block); a declaration that
//! fails is skipped up to the next line starting with a declaration keyword.

use crate::ast::*;
use crate::codes::*;
use crate::diag::{Error, Result, Span};
use crate::lexer::{Kw, Tok, Token};

pub fn parse(tokens: Vec<Token>) -> std::result::Result<File, Vec<Error>> {
    let mut p = Parser { toks: tokens, pos: 0, no_struct_lit: false, errors: Vec::new(), tests: 0 };
    let f = p.file();
    if p.errors.is_empty() { Ok(f) } else { Err(p.errors) }
}

struct Parser {
    toks: Vec<Token>,
    pos: usize,
    /// Set while parsing `if`/`while`/`for`/`match` headers, where `Name {`
    /// starts the body rather than a struct literal (same rule as Go).
    no_struct_lit: bool,
    errors: Vec<Error>,
    /// Number of `test` blocks so far.
    tests: usize,
}

fn not_supported(kw: Kw, span: Span) -> Error {
    let what = match kw {
        Kw::Comptime => "`comptime` is only supported on a parameter (`comptime T: type`)",
        Kw::Where => "`where` is only supported after a function's return type (`fn f(comptime T: type) T where T: I`)",
        _ => return Error::new(E0006, span, format!("unexpected `{}`", kw.as_str())),
    };
    Error::new(E0007, span, what)
}

/// Binding strength of `catch`: looser than `+`, tighter than comparisons.
const CATCH_PREC: u8 = 4;

fn is_unsupported_kw(kw: Kw) -> bool {
    matches!(kw, Kw::Comptime | Kw::Where)
}

/// `a.b.c` written as identifiers and field accesses: the path, joined
/// with `.`.
fn ident_path(e: &Expr) -> Option<String> {
    match &e.kind {
        ExprKind::Ident(n) => Some(n.clone()),
        ExprKind::Field { base, name } => Some(format!("{}.{name}", ident_path(base)?)),
        _ => None,
    }
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
        Error::new(E0006, self.span(), format!("expected {expected}, found {}", self.peek()))
    }

    /// The error after an element of a list closed by `closer` when neither
    /// `,` nor `closer` follows. When the list goes on with `closer` on the
    /// next line, the fix-it adds the missing trailing `,`.
    fn missing_comma(&self, closer: &Tok, expected: &str) -> Error {
        let e = self.unexpected(expected);
        let next = self.toks.get(self.pos + 1).map(|t| &t.tok);
        if self.at(&Tok::Semi) && next == Some(closer) && self.pos > 0 {
            let at = self.toks[self.pos - 1].end;
            return e.fix("add a trailing `,`", at, at, ",");
        }
        e
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

    // ------------------------------------------------------------ recovery

    /// Skip the rest of a failed statement that started at token `start`.
    /// Stops after a newline that is outside the braces the statement
    /// opened, or that is followed by a line starting left of the
    /// statement's column, or at its column with anything but `}` (so an
    /// unclosed `{` on the bad line does not swallow the rest of the block).
    /// Stops before a `}` that closes the block.
    fn sync_stmt(&mut self, start: usize) {
        let col = self.toks[start].span.col;
        let mut depth: i32 = 0;
        for t in &self.toks[start..self.pos] {
            match t.tok {
                Tok::LBrace => depth += 1,
                Tok::RBrace => depth -= 1,
                _ => {}
            }
        }
        let mut depth = depth.max(0);
        loop {
            match self.peek() {
                Tok::Eof => return,
                Tok::LBrace => depth += 1,
                Tok::RBrace if depth == 0 || self.span().col < col => return,
                Tok::RBrace => depth -= 1,
                Tok::Semi => {
                    self.bump();
                    let next = self.span().col;
                    if depth == 0 || next < col || (next == col && !self.at(&Tok::RBrace)) {
                        return;
                    }
                    continue;
                }
                _ => {}
            }
            self.bump();
        }
    }

    /// Skip a failed declaration: up to the next line that starts with a
    /// declaration keyword, or end of file.
    fn sync_item(&mut self) {
        loop {
            match self.peek() {
                Tok::Eof => return,
                Tok::Semi => {
                    self.bump();
                    if matches!(
                        self.peek(),
                        Tok::Kw(
                            Kw::Fn
                                | Kw::Pub
                                | Kw::Extern
                                | Kw::Struct
                                | Kw::Enum
                                | Kw::Error
                                | Kw::Interface
                                | Kw::Test
                                | Kw::Import
                        )
                    ) {
                        return;
                    }
                }
                _ => {
                    self.bump();
                }
            }
        }
    }

    // ---------------------------------------------------------------- items

    fn file(&mut self) -> File {
        let (mut imports, mut items) = (Vec::new(), Vec::new());
        loop {
            self.skip_semis();
            if self.at(&Tok::Eof) {
                return File { imports, items };
            }
            let r = if self.at(&Tok::Kw(Kw::Import)) {
                self.import().map(|i| imports.push(i))
            } else {
                self.item().map(|item| items.push(item))
            };
            let r = r.and_then(|_| self.end_of_stmt());
            if let Err(e) = r {
                self.errors.push(e);
                self.no_struct_lit = false;
                self.sync_item();
            }
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
                        Err(Error::new(E0007, self.span(), "`extern struct` is not yet supported"))
                    }
                    _ => Err(self.unexpected("`fn` after `extern`")),
                }
            }
            Tok::Kw(Kw::Struct) => Ok(Item::Struct(self.struct_decl(is_pub)?)),
            Tok::Kw(Kw::Enum) => Ok(Item::Enum(self.enum_decl(is_pub)?)),
            Tok::Kw(Kw::Error) => Ok(Item::ErrorSet(self.error_decl(is_pub)?)),
            Tok::Kw(Kw::Interface) => Ok(Item::Interface(self.interface_decl(is_pub)?)),
            Tok::Kw(Kw::Test) if !is_pub => Ok(Item::Test(self.test_decl()?)),
            Tok::Kw(Kw::Let) | Tok::Kw(Kw::Var) => {
                Err(Error::new(E0007, self.span(), "global variables are not yet supported"))
            }
            Tok::Kw(k) if is_unsupported_kw(k) => Err(not_supported(k, self.span())),
            _ => Err(self.unexpected("declaration (`fn`, `extern fn`, `struct`, `enum`, `error`, `interface` or `test`)")),
        }
    }

    /// `import a.b.c`
    fn import(&mut self) -> Result<Import> {
        let span = self.expect(&Tok::Kw(Kw::Import))?;
        let mut path = vec![self.ident()?];
        while self.eat(&Tok::Dot) {
            path.push(self.ident()?);
        }
        Ok(Import { path, span })
    }

    /// A name that can be a path: `File`, `fs.File`, `std.fs.File`.
    fn path(&mut self) -> Result<(String, Span)> {
        let (mut name, span) = self.ident()?;
        while self.at(&Tok::Dot) && matches!(self.toks.get(self.pos + 1).map(|t| &t.tok), Some(Tok::Ident(_))) {
            self.bump();
            name = format!("{name}.{}", self.ident()?.0);
        }
        Ok((name, span))
    }

    fn fn_decl(&mut self, is_pub: bool, is_extern: bool) -> Result<FnDecl> {
        let mut f = self.fn_sig(is_pub, is_extern)?;
        if is_extern {
            if self.at(&Tok::LBrace) {
                return Err(Error::new(E0011, self.span(), "`extern fn` cannot have a body"));
            }
        } else {
            if !self.at(&Tok::LBrace) {
                return Err(self.unexpected("`{` to start function body"));
            }
            f.body = Some(self.block()?);
        }
        Ok(f)
    }

    /// `fn name(params) ret`, without the body.
    fn fn_sig(&mut self, is_pub: bool, is_extern: bool) -> Result<FnDecl> {
        self.expect(&Tok::Kw(Kw::Fn))?;
        let (name, span) = self.ident()?;
        self.expect(&Tok::LParen)?;
        let mut params = Vec::new();
        while !self.at(&Tok::RParen) {
            let comptime = self.eat(&Tok::Kw(Kw::Comptime));
            let (pname, pspan) = self.ident()?;
            self.expect(&Tok::Colon)?;
            let ty = self.type_expr()?;
            params.push(Param { name: pname, span: pspan, ty, comptime });
            if !self.eat(&Tok::Comma) {
                break;
            }
        }
        self.expect(&Tok::RParen)?;
        let ret = match self.peek() {
            Tok::LBrace | Tok::Semi | Tok::RBrace | Tok::Eof | Tok::Kw(Kw::Where) => None,
            _ => Some(self.type_expr()?),
        };
        let mut bounds = Vec::new();
        if self.eat(&Tok::Kw(Kw::Where)) {
            loop {
                let (param, span) = self.ident()?;
                self.expect(&Tok::Colon)?;
                let (iface, iface_span) = self.path()?;
                bounds.push(Bound { param, span, iface, iface_span });
                if !self.eat(&Tok::Comma) {
                    break;
                }
            }
        }
        Ok(FnDecl { name, span, is_pub, is_extern, params, ret, bounds, body: None })
    }

    /// `struct Name { fields and methods }`: fields are separated by `,` or
    /// newlines; a method (`fn` or `pub fn`) ends with its block.
    fn struct_decl(&mut self, is_pub: bool) -> Result<StructDecl> {
        self.expect(&Tok::Kw(Kw::Struct))?;
        let (name, span) = self.ident()?;
        self.struct_body(name, span, is_pub)
    }

    /// `{ fields and methods }` of a struct declaration or of an anonymous
    /// `struct { ... }`.
    fn struct_body(&mut self, name: String, span: Span, is_pub: bool) -> Result<StructDecl> {
        self.expect(&Tok::LBrace)?;
        let (mut fields, mut methods) = (Vec::new(), Vec::new());
        loop {
            self.skip_semis();
            if self.eat(&Tok::RBrace) {
                break;
            }
            if self.at(&Tok::Kw(Kw::Fn)) || self.at(&Tok::Kw(Kw::Pub)) {
                let is_pub = self.eat(&Tok::Kw(Kw::Pub));
                methods.push(self.fn_decl(is_pub, false)?);
                if !self.at(&Tok::RBrace) {
                    self.expect(&Tok::Semi)?;
                }
                continue;
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
        Ok(StructDecl { name, span, is_pub, fields, methods })
    }

    /// `interface Name { fn m(self: *Self, ...) T ... }`, one method
    /// signature per line.
    fn interface_decl(&mut self, is_pub: bool) -> Result<InterfaceDecl> {
        self.expect(&Tok::Kw(Kw::Interface))?;
        let (name, span) = self.ident()?;
        self.expect(&Tok::LBrace)?;
        let mut methods = Vec::new();
        loop {
            self.skip_semis();
            if self.eat(&Tok::RBrace) {
                break;
            }
            if !self.at(&Tok::Kw(Kw::Fn)) {
                return Err(self.unexpected("`fn` (an interface has only method signatures)"));
            }
            methods.push(self.fn_sig(false, false)?);
            match self.peek() {
                Tok::Semi => {
                    self.bump();
                }
                Tok::RBrace => {}
                Tok::LBrace => return Err(Error::new(E0006, self.span(), "an interface method has no body")),
                _ => return Err(self.unexpected("newline or `}` after interface method")),
            }
        }
        Ok(InterfaceDecl { name, span, is_pub, methods })
    }

    /// `test "name" { ... }`: a function `test#N() !void`.
    fn test_decl(&mut self) -> Result<TestDecl> {
        let span = self.expect(&Tok::Kw(Kw::Test))?;
        let Tok::Str(bytes) = self.peek().clone() else {
            return Err(self.unexpected("test name (a string literal)"));
        };
        self.bump();
        if !self.at(&Tok::LBrace) {
            return Err(self.unexpected("`{` to start test body"));
        }
        let body = Some(self.block()?);
        let void = TypeExpr { kind: TypeExprKind::Named("void".to_string()), span };
        let ret = Some(TypeExpr { kind: TypeExprKind::ErrorUnion(None, Box::new(void)), span });
        let name = format!("test#{}", self.tests);
        self.tests += 1;
        let func = FnDecl { name, span, is_pub: false, is_extern: false, params: vec![], ret, bounds: vec![], body };
        Ok(TestDecl { name: String::from_utf8_lossy(&bytes).into_owned(), func })
    }

    fn enum_decl(&mut self, is_pub: bool) -> Result<EnumDecl> {
        self.expect(&Tok::Kw(Kw::Enum))?;
        let (name, span) = self.ident()?;
        self.expect(&Tok::LBrace)?;
        let mut variants = Vec::new();
        loop {
            self.skip_semis();
            if self.eat(&Tok::RBrace) {
                break;
            }
            let (vname, vspan) = self.ident()?;
            let fields = if self.at(&Tok::LBrace) {
                let fields = self.field_list()?;
                if fields.is_empty() {
                    return Err(Error::new(E0012, vspan, format!("a variant without fields is written `{vname}`, without `{{}}`")));
                }
                fields
            } else {
                Vec::new()
            };
            variants.push(VariantDecl { name: vname, span: vspan, fields });
            match self.peek() {
                Tok::Comma | Tok::Semi => {
                    self.bump();
                }
                Tok::RBrace => {}
                _ => return Err(self.unexpected("`,`, newline or `}` after enum variant")),
            }
        }
        Ok(EnumDecl { name, span, is_pub, variants })
    }

    /// `error Name { A, B }` (names separated by `,` or newlines) or
    /// `error Name = A || B`.
    fn error_decl(&mut self, is_pub: bool) -> Result<ErrorSetDecl> {
        self.expect(&Tok::Kw(Kw::Error))?;
        let (name, span) = self.ident()?;
        if self.eat(&Tok::Assign) {
            let mut sets = vec![self.path()?];
            while self.eat(&Tok::OrOr) {
                sets.push(self.path()?);
            }
            return Ok(ErrorSetDecl { name, span, is_pub, kind: ErrorSetKind::Merge(sets) });
        }
        self.expect(&Tok::LBrace)?;
        let mut errors = Vec::new();
        loop {
            self.skip_semis();
            if self.eat(&Tok::RBrace) {
                break;
            }
            errors.push(self.ident()?);
            match self.peek() {
                Tok::Comma | Tok::Semi => {
                    self.bump();
                }
                Tok::RBrace => {}
                _ => return Err(self.unexpected("`,`, newline or `}` after error name")),
            }
        }
        Ok(ErrorSetDecl { name, span, is_pub, kind: ErrorSetKind::List(errors) })
    }

    /// `{ name: T, ... }` with fields separated by `,` or newlines.
    fn field_list(&mut self) -> Result<Vec<FieldDecl>> {
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
        Ok(fields)
    }

    // ---------------------------------------------------------------- types

    fn type_expr(&mut self) -> Result<TypeExpr> {
        let span = self.span();
        let kind = match self.peek().clone() {
            Tok::Ident(_) => {
                let (name, _) = self.path()?;
                if self.eat(&Tok::Bang) {
                    TypeExprKind::ErrorUnion(Some((name, span)), Box::new(self.type_expr()?))
                } else if self.eat(&Tok::LParen) {
                    let mut args = Vec::new();
                    while !self.at(&Tok::RParen) {
                        args.push(self.type_expr()?);
                        if !self.eat(&Tok::Comma) {
                            break;
                        }
                    }
                    self.expect(&Tok::RParen)?;
                    TypeExprKind::Generic(name, args)
                } else {
                    TypeExprKind::Named(name)
                }
            }
            Tok::LBracket => {
                self.bump();
                match self.peek().clone() {
                    Tok::RBracket => {
                        self.bump();
                        let mutable = self.eat(&Tok::Kw(Kw::Mut));
                        TypeExprKind::Slice(Box::new(self.type_expr()?), mutable)
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
            Tok::Star => {
                self.bump();
                let mutable = self.eat(&Tok::Kw(Kw::Mut));
                TypeExprKind::Ptr(Box::new(self.type_expr()?), mutable)
            }
            Tok::Question => {
                self.bump();
                TypeExprKind::Optional(Box::new(self.type_expr()?))
            }
            Tok::Bang => {
                self.bump();
                TypeExprKind::ErrorUnion(None, Box::new(self.type_expr()?))
            }
            Tok::Kw(Kw::Dyn) => {
                self.bump();
                TypeExprKind::Dyn(self.path()?.0)
            }
            Tok::Kw(k) if is_unsupported_kw(k) => return Err(not_supported(k, span)),
            Tok::Kw(Kw::Struct) => {
                return Err(Error::new(E0007, span, "an anonymous `struct { ... }` can only be returned from a type function"));
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
            let start = self.pos;
            let r = match self.peek() {
                Tok::Kw(Kw::Else) => Err(self.else_on_new_line(stmts.last())),
                _ => self.stmt().and_then(|s| {
                    stmts.push(s);
                    self.end_of_stmt()
                }),
            };
            if let Err(e) = r {
                self.errors.push(e);
                self.no_struct_lit = false;
                self.sync_stmt(start);
            }
        }
        self.no_struct_lit = saved;
        Ok(Block { stmts, span })
    }

    /// `else` at the start of a statement: the `}` before it ended the line.
    /// When `prev` is an `if` without `else` that ends right before the
    /// line break, the fix-it joins the two lines.
    fn else_on_new_line(&self, prev: Option<&Stmt>) -> Error {
        let e = Error::new(E0008, self.span(), "`else` must be on the same line as the `}` that closes the `if` block");
        let mut last = prev;
        while let Some(Stmt { kind: StmtKind::If { els, .. }, .. }) = last {
            match els {
                Some(s) => last = Some(s),
                None => break,
            }
        }
        let open_if = matches!(last, Some(Stmt { kind: StmtKind::If { els: None, .. }, .. }));
        if open_if && self.pos >= 2 {
            let (brace, semi) = (&self.toks[self.pos - 2], &self.toks[self.pos - 1]);
            if brace.tok == Tok::RBrace && semi.span == brace.end && self.span().line == brace.span.line + 1 {
                return e.fix("put `else` on the line of the `}`", brace.end, self.span(), " ");
            }
        }
        e
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
                        E0013,
                        self.span(),
                        format!("`{} {name}` needs an initializer (`= value`)", kw.as_str()),
                    ));
                }
                self.bump();
                let init = self.expr()?;
                StmtKind::Let { name, mutable: kw == Kw::Var, ty, init }
            }
            Tok::Kw(Kw::If) => return self.if_stmt(),
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
            Tok::Kw(kw @ (Kw::Defer | Kw::Errdefer)) => {
                self.bump();
                let inner_span = self.span();
                let inner = if self.at(&Tok::LBrace) {
                    Stmt { kind: StmtKind::Block(self.block()?), span: inner_span }
                } else {
                    self.simple_stmt()?
                };
                if kw == Kw::Defer { StmtKind::Defer(Box::new(inner)) } else { StmtKind::ErrDefer(Box::new(inner)) }
            }
            Tok::LBrace => StmtKind::Block(self.block()?),
            Tok::Kw(k) if is_unsupported_kw(k) => return Err(not_supported(k, span)),
            Tok::Kw(Kw::Fn | Kw::Struct | Kw::Enum | Kw::Error | Kw::Pub | Kw::Extern | Kw::Interface | Kw::Test) => {
                return Err(Error::new(E0010, span, "declarations are only allowed at the top level"));
            }
            Tok::Kw(Kw::Import) => {
                return Err(Error::new(E0010, span, "`import` is only allowed at the top level of a file"));
            }
            _ => return self.simple_stmt(),
        };
        Ok(Stmt { kind, span })
    }

    fn if_stmt(&mut self) -> Result<Stmt> {
        let span = self.expect(&Tok::Kw(Kw::If))?;
        let cond = self.header_expr()?;
        let capture = if self.eat(&Tok::Pipe) {
            let c = self.ident()?;
            self.expect(&Tok::Pipe)?;
            Some(c)
        } else {
            None
        };
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
        Ok(Stmt { kind: StmtKind::If { cond, capture, then, els }, span })
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
            Tok::Plus => (BinOp::Add, 5),
            Tok::Minus => (BinOp::Sub, 5),
            Tok::PlusPct => (BinOp::AddWrap, 5),
            Tok::MinusPct => (BinOp::SubWrap, 5),
            Tok::Star => (BinOp::Mul, 6),
            Tok::Slash => (BinOp::Div, 6),
            Tok::Percent => (BinOp::Rem, 6),
            Tok::StarPct => (BinOp::MulWrap, 6),
            _ => return None,
        })
    }

    /// Precedence climbing; all binary operators are left-associative except
    /// comparisons, which do not chain. `catch` sits between the comparisons
    /// and `+`, as in Zig.
    fn binary(&mut self, min_prec: u8) -> Result<Expr> {
        let mut lhs = self.cast()?;
        loop {
            if self.at(&Tok::Kw(Kw::Catch)) && CATCH_PREC >= min_prec {
                lhs = self.catch(lhs)?;
                continue;
            }
            let Some((op, prec)) = self.binop() else { break };
            if prec < min_prec {
                break;
            }
            let span = self.bump().span;
            let rhs = self.binary(prec + 1)?;
            lhs = Expr { kind: ExprKind::Binary { op, lhs: Box::new(lhs), rhs: Box::new(rhs) }, span };
            if op.is_comparison() && self.binop().is_some_and(|(o, _)| o.is_comparison()) {
                return Err(Error::new(E0009, self.span(), "comparison operators cannot be chained"));
            }
        }
        Ok(lhs)
    }

    /// `lhs catch [|e|] body`, where the body is a block or an expression
    /// that binds tighter than `catch`.
    fn catch(&mut self, lhs: Expr) -> Result<Expr> {
        let span = self.bump().span;
        let capture = if self.eat(&Tok::Pipe) {
            let c = self.ident()?;
            self.expect(&Tok::Pipe)?;
            Some(c)
        } else {
            None
        };
        let body = if self.at(&Tok::LBrace) {
            ArmBody::Block(self.block()?)
        } else {
            ArmBody::Expr(self.binary(CATCH_PREC + 1)?)
        };
        Ok(Expr { kind: ExprKind::Catch { expr: Box::new(lhs), capture, body: Box::new(body) }, span })
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
        if self.at(&Tok::Kw(Kw::Try)) {
            let span = self.bump().span;
            let expr = Box::new(self.unary()?);
            return Ok(Expr { kind: ExprKind::Try(expr), span });
        }
        if self.at(&Tok::Amp) {
            let span = self.bump().span;
            let mutable = self.eat(&Tok::Kw(Kw::Mut));
            let expr = Box::new(self.unary()?);
            return Ok(Expr { kind: ExprKind::AddrOf { mutable, expr }, span });
        }
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
                    if self.at(&Tok::LParen) {
                        let kind = ExprKind::MethodCall { base: Box::new(e), name, args: self.args()? };
                        e = Expr { kind, span };
                    } else if let Some(enum_name) = ident_path(&e)
                        && self.at(&Tok::LBrace)
                        && !self.no_struct_lit
                    {
                        let kind = ExprKind::VariantLit { enum_name, variant: name, fields: self.field_inits()? };
                        e = Expr { kind, span: e.span };
                    } else {
                        e = Expr { kind: ExprKind::Field { base: Box::new(e), name }, span };
                    }
                }
                Tok::DotStar => {
                    let span = self.bump().span;
                    e = Expr { kind: ExprKind::Deref(Box::new(e)), span };
                }
                // `F(T) { ... }`: a literal of the struct that `F(T)` returns
                // (also `mod.F(T) { ... }`).
                Tok::LBrace
                    if !self.no_struct_lit
                        && (matches!(e.kind, ExprKind::Call { .. })
                            || matches!(&e.kind, ExprKind::MethodCall { base, .. } if ident_path(base).is_some())) =>
                {
                    let ty = expr_to_type(&e)?;
                    e = Expr { kind: ExprKind::StructLit { ty, fields: self.field_inits()? }, span: e.span };
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
            args.push(self.arg()?);
            if !self.eat(&Tok::Comma) {
                break;
            }
        }
        if !self.at(&Tok::RParen) {
            return Err(self.missing_comma(&Tok::RParen, "`,` or `)`"));
        }
        self.bump();
        self.no_struct_lit = saved;
        Ok(args)
    }

    /// A call argument: an expression, or a type that does not read as an
    /// expression (`*T`, `?T`, `dyn I`), for a `comptime` parameter.
    fn arg(&mut self) -> Result<Expr> {
        if matches!(self.peek(), Tok::Star | Tok::Question | Tok::Kw(Kw::Dyn)) {
            let span = self.span();
            return Ok(Expr { kind: ExprKind::Type(self.type_expr()?), span });
        }
        self.expr()
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
                    _ if name == "null" => ExprKind::Null,
                    Tok::LParen => ExprKind::Call { name, args: self.args()? },
                    Tok::LBrace if !self.no_struct_lit => {
                        let ty = TypeExpr { kind: TypeExprKind::Named(name), span };
                        ExprKind::StructLit { ty, fields: self.field_inits()? }
                    }
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
                if !self.at(&Tok::LBrace) {
                    return Ok(Expr { kind: ExprKind::Type(ty), span });
                }
                if !matches!(ty.kind, TypeExprKind::Array(..) | TypeExprKind::Slice(..)) {
                    return Err(Error::new(E0007, span, "only array literals (`[N]T{...}`) and the empty slice (`[]T{}`) are supported"));
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
                    return Err(self.missing_comma(&Tok::RBrace, "`,` or `}` (multi-line literals need a trailing comma)"));
                }
                self.bump();
                self.no_struct_lit = saved;
                ExprKind::ArrayLit { ty, elems }
            }
            Tok::Kw(Kw::Match) => {
                self.bump();
                let scrutinee = Box::new(self.header_expr()?);
                self.expect(&Tok::LBrace)?;
                let saved = std::mem::replace(&mut self.no_struct_lit, false);
                let mut arms = Vec::new();
                loop {
                    self.skip_semis();
                    if self.eat(&Tok::RBrace) {
                        break;
                    }
                    arms.push(self.match_arm()?);
                    match self.peek() {
                        Tok::Comma | Tok::Semi => {
                            self.bump();
                        }
                        Tok::RBrace => {}
                        _ => return Err(self.unexpected("`,`, newline or `}` after match arm")),
                    }
                }
                self.no_struct_lit = saved;
                ExprKind::Match { scrutinee, arms }
            }
            Tok::Kw(Kw::Struct) => {
                self.bump();
                ExprKind::StructType(self.struct_body(String::new(), span, false)?)
            }
            Tok::Kw(k) if is_unsupported_kw(k) => return Err(not_supported(k, span)),
            _ => return Err(self.unexpected("expression")),
        };
        Ok(Expr { kind, span })
    }

    /// `pattern => body`, where the pattern is `_`, `.V` or `.V { a, b: c, .. }`
    /// and the body is a block or an expression.
    fn match_arm(&mut self) -> Result<MatchArm> {
        let span = self.span();
        let (mut variant, mut fields, mut rest) = (None, None, false);
        match self.peek() {
            Tok::Ident(n) if n == "_" => {
                self.bump();
            }
            Tok::Dot => {
                self.bump();
                variant = Some(self.ident()?.0);
                if self.eat(&Tok::LBrace) {
                    let mut pats = Vec::new();
                    while !self.at(&Tok::RBrace) {
                        if self.eat(&Tok::DotDot) {
                            rest = true;
                            break;
                        }
                        let (field, fspan) = self.ident()?;
                        let (binding, binding_span) =
                            if self.eat(&Tok::Colon) { self.ident()? } else { (field.clone(), fspan) };
                        pats.push(FieldPat { field, span: fspan, binding, binding_span });
                        if !self.eat(&Tok::Comma) {
                            break;
                        }
                    }
                    self.expect(&Tok::RBrace)?;
                    fields = Some(pats);
                }
            }
            _ => return Err(self.unexpected("pattern (`.Variant` or `_`)")),
        }
        self.expect(&Tok::FatArrow)?;
        let body = if self.at(&Tok::LBrace) { ArmBody::Block(self.block()?) } else { ArmBody::Expr(self.expr()?) };
        Ok(MatchArm { variant, fields, rest, body, span })
    }

    /// `{ name: value, ... }` of a struct or variant literal.
    fn field_inits(&mut self) -> Result<Vec<FieldInit>> {
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
            return Err(self.missing_comma(&Tok::RBrace, "`,` or `}` (multi-line literals need a trailing comma)"));
        }
        self.bump();
        Ok(fields)
    }
}

/// The type that a call-shaped expression names: `F(A, B)` in a struct
/// literal `F(A, B) { ... }`.
fn expr_to_type(e: &Expr) -> Result<TypeExpr> {
    let kind = match &e.kind {
        ExprKind::Ident(_) | ExprKind::Field { .. } if let Some(path) = ident_path(e) => TypeExprKind::Named(path),
        ExprKind::Call { name, args } => {
            TypeExprKind::Generic(name.clone(), args.iter().map(expr_to_type).collect::<Result<_>>()?)
        }
        ExprKind::MethodCall { base, name, args } if let Some(path) = ident_path(base) => {
            TypeExprKind::Generic(format!("{path}.{name}"), args.iter().map(expr_to_type).collect::<Result<_>>()?)
        }
        ExprKind::Type(t) => return Ok(t.clone()),
        _ => return Err(Error::new(E0006, e.span, "expected a type")),
    };
    Ok(TypeExpr { kind, span: e.span })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;

    fn parse_src(src: &str) -> std::result::Result<File, Error> {
        parse(lex(src).unwrap()).map_err(|mut e| e.remove(0))
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
    fn imports() {
        let f = parse_src("import std\nimport std.mem.Allocator\n").unwrap();
        assert_eq!(f.imports.len(), 2);
        assert_eq!(f.imports[1].path.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(), ["std", "mem", "Allocator"]);
        let e = parse_src("fn f() {\n import std\n}").unwrap_err();
        assert_eq!(e.msg, "`import` is only allowed at the top level of a file");
    }

    #[test]
    fn unsupported_keywords() {
        let e = parse_src("fn f() {\n comptime x\n}").unwrap_err();
        assert_eq!(e.msg, "`comptime` is only supported on a parameter (`comptime T: type`)");
    }

    /// `catch` binds looser than `+` and tighter than `==`.
    #[test]
    fn catch_precedence() {
        let f = parse_src("fn f() {\n let b = g() catch 0 + 1 == 2\n}").unwrap();
        let Item::Fn(f) = &f.items[0] else { panic!() };
        let StmtKind::Let { init, .. } = &f.body.as_ref().unwrap().stmts[0].kind else { panic!() };
        let ExprKind::Binary { op: BinOp::Eq, lhs, .. } = &init.kind else { panic!("{init:?}") };
        let ExprKind::Catch { body, .. } = &lhs.kind else { panic!("{lhs:?}") };
        assert!(matches!(&**body, ArmBody::Expr(Expr { kind: ExprKind::Binary { op: BinOp::Add, .. }, .. })));
    }

    #[test]
    fn chained_comparison_rejected() {
        let e = parse_src("fn f() {\n let b = 1 < 2 < 3\n}").unwrap_err();
        assert_eq!(e.msg, "comparison operators cannot be chained");
    }
}
