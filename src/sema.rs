//! Semantic analysis: name resolution and type checking.
//! Produces a typed AST (`Program`) in which every expression carries its
//! concrete type; untyped literals have already been given their type.
//!
//! Error recovery: a statement with an error is reported and dropped, and
//! checking goes on with the next one. A name whose type could not be
//! determined gets `Type::Error`; any use of it fails without a message, so
//! one mistake is reported once.

use crate::ast::{self, ArmBody, AssignOp, BinOp, ExprKind, StmtKind, TypeExprKind, UnOp};
use crate::diag::{Error, Result, Span};
use std::collections::HashMap;
use std::fmt;

// ------------------------------------------------------------------ types

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IntTy {
    I8,
    I16,
    I32,
    I64,
    Isize,
    U8,
    U16,
    U32,
    U64,
    Usize,
}

impl IntTy {
    pub fn signed(self) -> bool {
        matches!(self, IntTy::I8 | IntTy::I16 | IntTy::I32 | IntTy::I64 | IntTy::Isize)
    }

    /// Bit width. `isize`/`usize` are 64 bits (LP64 targets only for now).
    pub fn bits(self) -> u32 {
        match self {
            IntTy::I8 | IntTy::U8 => 8,
            IntTy::I16 | IntTy::U16 => 16,
            IntTy::I32 | IntTy::U32 => 32,
            _ => 64,
        }
    }

    pub fn min(self) -> i128 {
        if self.signed() { -(1i128 << (self.bits() - 1)) } else { 0 }
    }

    pub fn max(self) -> i128 {
        if self.signed() { (1i128 << (self.bits() - 1)) - 1 } else { (1i128 << self.bits()) - 1 }
    }

    pub fn name(self) -> &'static str {
        match self {
            IntTy::I8 => "i8",
            IntTy::I16 => "i16",
            IntTy::I32 => "i32",
            IntTy::I64 => "i64",
            IntTy::Isize => "isize",
            IntTy::U8 => "u8",
            IntTy::U16 => "u16",
            IntTy::U32 => "u32",
            IntTy::U64 => "u64",
            IntTy::Usize => "usize",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FloatTy {
    F32,
    F64,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Type {
    Int(IntTy),
    Float(FloatTy),
    Bool,
    /// Result type of functions without a return type. Not nameable.
    Void,
    Array(Box<Type>, u64),
    Slice(Box<Type>),
    /// `[*]T`: unchecked C pointer, for FFI.
    CPtr(Box<Type>),
    /// `*T` (mutable = false) or `*mut T` (mutable = true). Never null.
    Ptr(Box<Type>, bool),
    /// `?T`
    Optional(Box<Type>),
    Struct(String),
    Enum(String),
    /// The type of something that already has a reported error.
    Error,
}

/// How `Type::Error` prints. Messages that mention it are follow-up errors
/// and are not reported.
const ERROR_TY: &str = "{error}";

pub const USIZE: Type = Type::Int(IntTy::Usize);
const I64: Type = Type::Int(IntTy::I64);

impl Type {
    pub fn is_int(&self) -> bool {
        matches!(self, Type::Int(_))
    }

    pub fn is_numeric(&self) -> bool {
        matches!(self, Type::Int(_) | Type::Float(_))
    }

    fn from_name(name: &str) -> Option<Type> {
        Some(match name {
            "i8" => Type::Int(IntTy::I8),
            "i16" => Type::Int(IntTy::I16),
            "i32" => Type::Int(IntTy::I32),
            "i64" => Type::Int(IntTy::I64),
            "isize" => Type::Int(IntTy::Isize),
            "u8" => Type::Int(IntTy::U8),
            "u16" => Type::Int(IntTy::U16),
            "u32" => Type::Int(IntTy::U32),
            "u64" => Type::Int(IntTy::U64),
            "usize" => Type::Int(IntTy::Usize),
            "f32" => Type::Float(FloatTy::F32),
            "f64" => Type::Float(FloatTy::F64),
            "bool" => Type::Bool,
            _ => return None,
        })
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Int(i) => f.write_str(i.name()),
            Type::Float(FloatTy::F32) => f.write_str("f32"),
            Type::Float(FloatTy::F64) => f.write_str("f64"),
            Type::Bool => f.write_str("bool"),
            Type::Void => f.write_str("void"),
            Type::Array(t, n) => write!(f, "[{n}]{t}"),
            Type::Slice(t) => write!(f, "[]{t}"),
            Type::CPtr(t) => write!(f, "[*]{t}"),
            Type::Ptr(t, false) => write!(f, "*{t}"),
            Type::Ptr(t, true) => write!(f, "*mut {t}"),
            Type::Optional(t) => write!(f, "?{t}"),
            Type::Struct(n) | Type::Enum(n) => f.write_str(n),
            Type::Error => f.write_str(ERROR_TY),
        }
    }
}

// -------------------------------------------------------------- typed AST

pub type VarId = usize;

#[derive(Debug, Clone)]
pub struct Program {
    pub structs: Vec<StructDef>,
    pub enums: Vec<EnumDef>,
    pub externs: Vec<ExternFn>,
    pub fns: Vec<FnDef>,
}

#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<(String, Type)>,
}

#[derive(Debug, Clone)]
pub struct EnumDef {
    pub name: String,
    /// The tag of a variant is its index.
    pub variants: Vec<VariantDef>,
}

#[derive(Debug, Clone)]
pub struct VariantDef {
    pub name: String,
    pub fields: Vec<(String, Type)>,
}

#[derive(Debug, Clone)]
pub struct ExternFn {
    pub name: String,
    pub params: Vec<Type>,
    pub ret: Type,
}

#[derive(Debug, Clone)]
pub struct FnDef {
    pub name: String,
    pub params: Vec<VarId>,
    pub ret: Type,
    pub vars: Vec<VarInfo>,
    pub body: Vec<TStmt>,
    pub is_main: bool,
}

#[derive(Debug, Clone)]
pub struct VarInfo {
    pub name: String,
    pub ty: Type,
    pub mutable: bool,
}

#[derive(Debug, Clone)]
pub struct TStmt {
    pub kind: TStmtKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum TStmtKind {
    Let(VarId, TExpr),
    /// `op` is `None` for plain `=`, otherwise the compound operator.
    Assign { target: TExpr, op: Option<BinOp>, value: TExpr },
    /// With `capture`, `cond` is a `?T` and the variable gets its value.
    If { cond: TExpr, capture: Option<VarId>, then: Vec<TStmt>, els: Vec<TStmt> },
    While { cond: TExpr, body: Vec<TStmt> },
    For { var: VarId, lo: TExpr, hi: TExpr, body: Vec<TStmt> },
    Break,
    Continue,
    Return(Option<TExpr>),
    Defer(Vec<TStmt>),
    Block(Vec<TStmt>),
    Expr(TExpr),
}

#[derive(Debug, Clone)]
pub struct TExpr {
    pub kind: TExprKind,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum TExprKind {
    Int(i128),
    Float(f64),
    Bool(bool),
    Str(Vec<u8>),
    /// `null` of the optional type `ty`.
    Null,
    /// A `T` wrapped into `?T`.
    Some(Box<TExpr>),
    /// Is the optional non-null?
    IsSome(Box<TExpr>),
    /// `&x` / `&mut x`; the operand is a place.
    AddrOf(Box<TExpr>),
    /// `p.*`, also inserted for field access through a pointer.
    Deref(Box<TExpr>),
    Var(VarId),
    Call { name: String, is_extern: bool, args: Vec<TExpr> },
    /// `@print(x)` / `@println(x)` / `@println()`.
    Print { arg: Option<Box<TExpr>>, newline: bool },
    Binary { op: BinOp, lhs: Box<TExpr>, rhs: Box<TExpr> },
    Unary { op: UnOp, expr: Box<TExpr> },
    Cast(Box<TExpr>),
    /// Struct field by index.
    Field { base: Box<TExpr>, index: usize },
    SliceLen(Box<TExpr>),
    SlicePtr(Box<TExpr>),
    ArrayLen(Box<TExpr>),
    Index { base: Box<TExpr>, index: Box<TExpr> },
    Slice { base: Box<TExpr>, lo: Option<Box<TExpr>>, hi: Option<Box<TExpr>> },
    /// Field initializers in source order: (field index, value).
    StructLit(Vec<(usize, TExpr)>),
    /// Enum value: variant index and payload initializers in source order.
    Variant { index: usize, fields: Vec<(usize, TExpr)> },
    /// Arms in source order. The type is `void` for a `match` statement.
    Match { scrutinee: Box<TExpr>, arms: Vec<TArm> },
    ArrayLit(Vec<TExpr>),
}

#[derive(Debug, Clone)]
pub struct TArm {
    /// Variant index; `None` for `_`.
    pub variant: Option<usize>,
    /// Pattern variables and the payload field each one is bound to.
    pub bindings: Vec<(VarId, usize)>,
    pub body: TArmBody,
}

#[derive(Debug, Clone)]
pub enum TArmBody {
    Expr(TExpr),
    Block(Vec<TStmt>),
}

// ---------------------------------------------------------------- checker

#[derive(Clone)]
struct FnSig {
    params: Vec<Type>,
    ret: Type,
    is_extern: bool,
}

struct Checker {
    structs: HashMap<String, StructDef>,
    enums: HashMap<String, EnumDef>,
    fns: HashMap<String, FnSig>,
    /// Top-level names (structs, enums, fns, extern fns) and where they are declared.
    top_spans: HashMap<String, Span>,
    errors: Vec<Error>,
    /// Number of failed checks, including silent follow-up ones.
    failures: usize,
    // Per-function state.
    vars: Vec<VarInfo>,
    scopes: Vec<HashMap<String, (VarId, Span)>>,
    ret: Type,
    loop_depth: u32,
    in_defer: bool,
}

pub fn check(file: &ast::File) -> std::result::Result<Program, Vec<Error>> {
    let mut c = Checker {
        structs: HashMap::new(),
        enums: HashMap::new(),
        fns: HashMap::new(),
        top_spans: HashMap::new(),
        errors: Vec::new(),
        failures: 0,
        vars: Vec::new(),
        scopes: Vec::new(),
        ret: Type::Void,
        loop_depth: 0,
        in_defer: false,
    };
    let p = c.program(file);
    if c.errors.is_empty() { Ok(p) } else { Err(c.errors) }
}

fn err<T>(span: Span, msg: impl Into<String>) -> Result<T> {
    Err(Error::new(span, msg))
}

/// A failure caused by an error that was already reported.
fn silent() -> Error {
    Error::new(Span::default(), "")
}

fn mismatch(span: Span, expected: &Type, found: &Type) -> Error {
    Error::new(span, format!("mismatched types: expected `{expected}`, found `{found}`"))
}

/// Is this expression an untyped numeric literal (whose type comes from context)?
fn is_untyped(e: &ast::Expr) -> bool {
    match &e.kind {
        ExprKind::Int(_) | ExprKind::Char(_) | ExprKind::Float(_) => true,
        ExprKind::Paren(inner) => is_untyped(inner),
        ExprKind::Unary { op: UnOp::Neg, expr } => is_untyped(expr),
        ExprKind::Binary { op, lhs, rhs } if op.is_arith() => is_untyped(lhs) && is_untyped(rhs),
        _ => false,
    }
}

fn is_null(e: &ast::Expr) -> bool {
    match &e.kind {
        ExprKind::Null => true,
        ExprKind::Paren(inner) => is_null(inner),
        _ => false,
    }
}

fn strip_parens(e: &ast::Expr) -> &ast::Expr {
    match &e.kind {
        ExprKind::Paren(inner) => strip_parens(inner),
        _ => e,
    }
}

fn is_unsigned(t: &Type) -> bool {
    matches!(t, Type::Int(i) if !i.signed())
}

fn block_returns(stmts: &[TStmt]) -> bool {
    stmts.iter().any(|s| match &s.kind {
        TStmtKind::Return(_) => true,
        TStmtKind::Block(b) => block_returns(b),
        TStmtKind::If { then, els, .. } => block_returns(then) && block_returns(els),
        TStmtKind::Expr(TExpr { kind: TExprKind::Match { arms, .. }, .. }) => {
            arms.iter().all(|a| matches!(&a.body, TArmBody::Block(b) if block_returns(b)))
        }
        _ => false,
    })
}

impl Checker {
    /// Record an error and go on. Follow-up errors are dropped: the silent
    /// error, and any message that mentions `Type::Error`.
    fn report(&mut self, e: Error) {
        self.failures += 1;
        if !e.msg.is_empty() && !e.msg.contains(ERROR_TY) {
            self.errors.push(e);
        }
    }

    fn program(&mut self, file: &ast::File) -> Program {
        // 1. Top-level names. An item whose name is rejected is skipped.
        let mut items = Vec::new();
        for item in &file.items {
            let (name, span) = match item {
                ast::Item::Struct(s) => (&s.name, s.span),
                ast::Item::Enum(e) => (&e.name, e.span),
                ast::Item::Fn(f) => (&f.name, f.span),
            };
            if Type::from_name(name).is_some() || matches!(name.as_str(), "true" | "false" | "null") {
                self.report(Error::new(span, format!("`{name}` is a builtin name and cannot be redefined")));
                continue;
            }
            if self.top_spans.contains_key(name) {
                self.report(Error::new(span, format!("`{name}` is defined more than once")));
                continue;
            }
            self.top_spans.insert(name.clone(), span);
            match item {
                ast::Item::Struct(s) => {
                    self.structs.insert(s.name.clone(), StructDef { name: s.name.clone(), fields: vec![] });
                }
                ast::Item::Enum(e) => {
                    self.enums.insert(e.name.clone(), EnumDef { name: e.name.clone(), variants: vec![] });
                }
                ast::Item::Fn(_) => {}
            }
            items.push(item);
        }
        // 2. Struct fields and enum variants.
        for item in &items {
            match item {
                ast::Item::Struct(s) => {
                    let fields = self.fields(&s.fields, &format!("struct `{}`", s.name));
                    self.structs.get_mut(&s.name).unwrap().fields = fields;
                }
                ast::Item::Enum(e) => {
                    if e.variants.is_empty() {
                        self.report(Error::new(e.span, format!("enum `{}` has no variants", e.name)));
                    }
                    let mut variants: Vec<VariantDef> = Vec::new();
                    for v in &e.variants {
                        if variants.iter().any(|x| x.name == v.name) {
                            self.report(Error::new(v.span, format!("duplicate variant `{}` in enum `{}`", v.name, e.name)));
                            continue;
                        }
                        let fields = self.fields(&v.fields, &format!("variant `{}.{}`", e.name, v.name));
                        variants.push(VariantDef { name: v.name.clone(), fields });
                    }
                    self.enums.get_mut(&e.name).unwrap().variants = variants;
                }
                ast::Item::Fn(_) => {}
            }
        }
        for item in &items {
            let (name, span) = match item {
                ast::Item::Struct(s) => (&s.name, s.span),
                ast::Item::Enum(e) => (&e.name, e.span),
                ast::Item::Fn(_) => continue,
            };
            if let Err(e) = self.check_recursive(name, name, span, &mut Vec::new()) {
                self.report(e);
            }
        }
        // 3. Function signatures.
        let mut main_found = false;
        let mut externs = Vec::new();
        for item in &items {
            let ast::Item::Fn(f) = item else { continue };
            let ffi_ok = |t: &Type| matches!(t, Type::Int(_) | Type::Float(_) | Type::Bool | Type::CPtr(_) | Type::Error);
            let mut params = Vec::new();
            for p in &f.params {
                let ty = self.resolve_or_report(&p.ty);
                if f.is_extern && !ffi_ok(&ty) {
                    self.report(Error::new(p.ty.span, format!("type `{ty}` cannot be used in an `extern fn` signature")));
                }
                params.push(ty);
            }
            let ret = match &f.ret {
                Some(t) => self.resolve_or_report(t),
                None => Type::Void,
            };
            if f.is_extern {
                if ret != Type::Void && !ffi_ok(&ret) {
                    let span = f.ret.as_ref().unwrap().span;
                    self.report(Error::new(span, format!("type `{ret}` cannot be used in an `extern fn` signature")));
                }
                externs.push(ExternFn { name: f.name.clone(), params: params.clone(), ret: ret.clone() });
            }
            if f.name == "main" && !f.is_extern {
                main_found = true;
                if !f.is_pub {
                    self.report(Error::new(f.span, "`main` must be declared `pub fn main()`"));
                } else if !params.is_empty() || ret != Type::Void {
                    self.report(Error::new(f.span, "`main` must take no parameters and have no return type"));
                }
            }
            self.fns.insert(f.name.clone(), FnSig { params, ret, is_extern: f.is_extern });
        }
        if !main_found {
            self.report(Error::new(Span::new(1, 1), "no `pub fn main()` found"));
        }
        // 4. Bodies.
        let mut fns = Vec::new();
        for item in &items {
            if let ast::Item::Fn(f) = item
                && !f.is_extern
            {
                fns.push(self.fn_body(f));
            }
        }
        let mut structs = Vec::new();
        let mut enums = Vec::new();
        for item in &items {
            match item {
                ast::Item::Struct(s) => structs.push(self.structs[&s.name].clone()),
                ast::Item::Enum(e) => enums.push(self.enums[&e.name].clone()),
                ast::Item::Fn(_) => {}
            }
        }
        Program { structs, enums, externs, fns }
    }

    /// Fields of a struct or enum variant; `owner` names it in messages.
    fn fields(&mut self, decls: &[ast::FieldDecl], owner: &str) -> Vec<(String, Type)> {
        let mut fields: Vec<(String, Type)> = Vec::new();
        for f in decls {
            if fields.iter().any(|(n, _)| *n == f.name) {
                self.report(Error::new(f.span, format!("duplicate field `{}` in {owner}", f.name)));
                continue;
            }
            let ty = self.resolve_or_report(&f.ty);
            fields.push((f.name.clone(), ty));
        }
        fields
    }

    /// Fields that a struct or enum holds by value.
    fn by_value_fields(&self, name: &str) -> Vec<&Type> {
        match self.structs.get(name) {
            Some(s) => s.fields.iter().map(|(_, t)| t).collect(),
            None => self.enums[name].variants.iter().flat_map(|v| v.fields.iter().map(|(_, t)| t)).collect(),
        }
    }

    fn check_recursive(&self, root: &str, name: &str, span: Span, path: &mut Vec<String>) -> Result<()> {
        if path.iter().any(|p| p == name) {
            return Ok(());
        }
        path.push(name.to_string());
        for ty in self.by_value_fields(name) {
            let mut t = ty;
            while let Type::Array(inner, _) | Type::Optional(inner) = t {
                t = inner;
            }
            if let Type::Struct(n) | Type::Enum(n) = t {
                if n == root {
                    let kind = if self.structs.contains_key(root) { "struct" } else { "enum" };
                    return err(span, format!("{kind} `{root}` contains itself by value (infinite size)"));
                }
                self.check_recursive(root, n, span, path)?;
            }
        }
        Ok(())
    }

    fn resolve_type(&self, t: &ast::TypeExpr) -> Result<Type> {
        Ok(match &t.kind {
            TypeExprKind::Named(n) => match Type::from_name(n) {
                Some(ty) => ty,
                None if self.structs.contains_key(n) => Type::Struct(n.clone()),
                None if self.enums.contains_key(n) => Type::Enum(n.clone()),
                None => return err(t.span, format!("unknown type `{n}`")),
            },
            TypeExprKind::Array(n, elem) => Type::Array(Box::new(self.resolve_type(elem)?), *n),
            TypeExprKind::Slice(elem) => Type::Slice(Box::new(self.resolve_type(elem)?)),
            TypeExprKind::CPtr(elem) => Type::CPtr(Box::new(self.resolve_type(elem)?)),
            TypeExprKind::Ptr(elem, mutable) => Type::Ptr(Box::new(self.resolve_type(elem)?), *mutable),
            TypeExprKind::Optional(elem) => Type::Optional(Box::new(self.resolve_type(elem)?)),
        })
    }

    fn resolve_or_report(&mut self, t: &ast::TypeExpr) -> Type {
        self.resolve_type(t).unwrap_or_else(|e| {
            self.report(e);
            Type::Error
        })
    }

    // ------------------------------------------------------------ functions

    /// No shadowing: the name must not be visible here at all, whether as a
    /// local in any enclosing scope, a parameter or a top-level name.
    fn declare(&mut self, name: &str, ty: Type, mutable: bool, span: Span) -> Result<VarId> {
        let prev = self.scopes.iter().find_map(|s| s.get(name).map(|&(_, sp)| sp));
        if let Some(prev) = prev.or_else(|| self.top_spans.get(name).copied()) {
            return err(span, format!(
                "`{name}` is already declared at {}:{} (shadowing is not allowed)",
                prev.line, prev.col
            ));
        }
        let id = self.vars.len();
        self.scopes.last_mut().unwrap().insert(name.to_string(), (id, span));
        self.vars.push(VarInfo { name: name.to_string(), ty, mutable });
        Ok(id)
    }

    /// `declare`, reporting a failure; `None` if it failed. After a failure
    /// the name still refers to an error-typed variable here, so its uses
    /// are not reported again.
    fn declare_or_report(&mut self, name: &str, ty: Type, mutable: bool, span: Span) -> Option<VarId> {
        match self.declare(name, ty, mutable, span) {
            Ok(id) => Some(id),
            Err(e) => {
                self.report(e);
                let id = self.vars.len();
                self.scopes.last_mut().unwrap().insert(name.to_string(), (id, span));
                self.vars.push(VarInfo { name: name.to_string(), ty: Type::Error, mutable });
                None
            }
        }
    }

    fn lookup(&self, name: &str) -> Option<VarId> {
        self.scopes.iter().rev().find_map(|s| s.get(name).map(|&(id, _)| id))
    }

    fn fn_body(&mut self, f: &ast::FnDecl) -> FnDef {
        let sig = self.fns[&f.name].clone();
        self.vars.clear();
        self.scopes = vec![HashMap::new()];
        self.ret = sig.ret.clone();
        self.loop_depth = 0;
        self.in_defer = false;
        let failures = self.failures;
        let mut params = Vec::new();
        for (p, ty) in f.params.iter().zip(&sig.params) {
            params.extend(self.declare_or_report(&p.name, ty.clone(), false, p.span));
        }
        let body = self.block(f.body.as_ref().unwrap());
        // A statement with an error may have been the `return`.
        if self.failures == failures && sig.ret != Type::Void && !block_returns(&body) {
            self.report(Error::new(
                f.span,
                format!("function `{}` must return a value of type `{}` on every path", f.name, sig.ret),
            ));
        }
        FnDef {
            name: f.name.clone(),
            params,
            ret: sig.ret,
            vars: std::mem::take(&mut self.vars),
            body,
            is_main: f.name == "main",
        }
    }

    /// Statements with errors are reported and left out.
    fn block(&mut self, b: &ast::Block) -> Vec<TStmt> {
        self.scopes.push(HashMap::new());
        let mut out = Vec::new();
        for s in &b.stmts {
            match self.stmt(s) {
                Ok(t) => out.push(t),
                Err(e) => self.report(e),
            }
        }
        self.scopes.pop();
        out
    }

    fn stmt(&mut self, s: &ast::Stmt) -> Result<TStmt> {
        let kind = match &s.kind {
            StmtKind::Let { name, mutable, ty, init } => {
                let (ty, init) = match ty {
                    Some(t) => match self.resolve_type(t) {
                        Ok(ty) => {
                            let init = self.expect(init, &ty);
                            (ty, init)
                        }
                        Err(e) => (Type::Error, Err(e)),
                    },
                    None => {
                        let init = self.value(init, None);
                        (init.as_ref().map_or(Type::Error, |i| i.ty.clone()), init)
                    }
                };
                // Declared even when the initializer has an error, so that
                // later uses are not reported as undefined.
                let id = self.declare_or_report(name, ty, *mutable, s.span);
                let init = init?;
                TStmtKind::Let(id.ok_or_else(silent)?, init)
            }
            StmtKind::Assign { target, op, value } => {
                let target = self.value(target, None)?;
                self.check_mutable(&target, false)?;
                let value = self.expect(value, &target.ty)?;
                let op = match op {
                    AssignOp::Set => None,
                    AssignOp::Op(o) => {
                        self.arith_operand_ok(*o, &target.ty, s.span)?;
                        Some(*o)
                    }
                };
                TStmtKind::Assign { target, op, value }
            }
            StmtKind::If { cond, capture, then, els } => {
                let (cond, capture, then) = match capture {
                    None => (self.condition(cond, "if"), None, self.block(then)),
                    Some((name, name_span)) => {
                        let c = self.value(cond, None).and_then(|c| match &c.ty {
                            Type::Optional(_) => Ok(c),
                            t => err(c.span, format!("`if` with `|{name}|` needs an optional (`?T`) value, found `{t}`")),
                        });
                        let inner = match &c {
                            Ok(TExpr { ty: Type::Optional(t), .. }) => (**t).clone(),
                            _ => Type::Error,
                        };
                        self.scopes.push(HashMap::new());
                        let v = self.declare_or_report(name, inner, false, *name_span);
                        let then = self.block(then);
                        self.scopes.pop();
                        (c, Some(v), then)
                    }
                };
                let els = match els {
                    Some(e) => match &e.kind {
                        StmtKind::Block(b) => self.block(b),
                        _ => self.stmt(e).map_or_else(|e| { self.report(e); vec![] }, |s| vec![s]),
                    },
                    None => vec![],
                };
                let cond = cond?;
                let capture = match capture {
                    Some(v) => Some(v.ok_or_else(silent)?),
                    None => None,
                };
                TStmtKind::If { cond, capture, then, els }
            }
            StmtKind::While { cond, body } => {
                let cond = self.condition(cond, "while");
                self.loop_depth += 1;
                let body = self.block(body);
                self.loop_depth -= 1;
                TStmtKind::While { cond: cond?, body }
            }
            StmtKind::For { var, var_span, lo, hi, body } => {
                let range = self.operand_pair(lo, hi, None).and_then(|(lo, hi)| {
                    if lo.ty != hi.ty {
                        return err(hi.span, format!("range bounds have different types: `{}` and `{}`", lo.ty, hi.ty));
                    }
                    if !lo.ty.is_int() {
                        return err(lo.span, format!("range bounds must be integers, found `{}`", lo.ty));
                    }
                    Ok((lo, hi))
                });
                let ty = range.as_ref().map_or(Type::Error, |(lo, _)| lo.ty.clone());
                self.scopes.push(HashMap::new());
                let var = self.declare_or_report(var, ty, false, *var_span);
                self.loop_depth += 1;
                let body = self.block(body);
                self.loop_depth -= 1;
                self.scopes.pop();
                let (lo, hi) = range?;
                TStmtKind::For { var: var.ok_or_else(silent)?, lo, hi, body }
            }
            StmtKind::Break | StmtKind::Continue => {
                let word = if matches!(s.kind, StmtKind::Break) { "break" } else { "continue" };
                if self.loop_depth == 0 {
                    if self.in_defer {
                        return err(s.span, format!("`{word}` cannot leave a `defer` block"));
                    }
                    return err(s.span, format!("`{word}` outside of a loop"));
                }
                if word == "break" { TStmtKind::Break } else { TStmtKind::Continue }
            }
            StmtKind::Return(value) => {
                if self.in_defer {
                    return err(s.span, "`return` is not allowed inside `defer`");
                }
                let ret = self.ret.clone();
                match (value, &ret) {
                    (None, Type::Void) => TStmtKind::Return(None),
                    (None, t) => return err(s.span, format!("missing return value of type `{t}`")),
                    (Some(e), Type::Void) => {
                        return err(e.span, "function has no return type but a value was returned");
                    }
                    (Some(e), t) => TStmtKind::Return(Some(self.expect(e, t)?)),
                }
            }
            StmtKind::Defer(inner) => {
                let saved = (self.loop_depth, self.in_defer);
                self.loop_depth = 0;
                self.in_defer = true;
                let body = match &inner.kind {
                    StmtKind::Block(b) => Ok(self.block(b)),
                    _ => self.stmt(inner).map(|s| vec![s]),
                };
                (self.loop_depth, self.in_defer) = saved;
                TStmtKind::Defer(body?)
            }
            StmtKind::Block(b) => TStmtKind::Block(self.block(b)),
            StmtKind::Expr(e) => TStmtKind::Expr(self.expr_stmt(e)?),
        };
        Ok(TStmt { kind, span: s.span })
    }

    /// An expression used as a statement: a call or a `match`.
    fn expr_stmt(&mut self, e: &ast::Expr) -> Result<TExpr> {
        match &e.kind {
            ExprKind::Match { .. } => self.match_expr(e, None, true),
            ExprKind::Call { .. } | ExprKind::Builtin { .. } => self.expr(e, None),
            _ => err(e.span, "expression value is not used (only calls can be statements)"),
        }
    }

    fn condition(&mut self, e: &ast::Expr, what: &str) -> Result<TExpr> {
        let c = self.value(e, Some(&Type::Bool))?;
        if c.ty != Type::Bool {
            let hint = if matches!(c.ty, Type::Optional(_)) {
                " (unwrap it with `if x |v| { }` or compare it with `null`)"
            } else {
                ""
            };
            return err(c.span, format!("`{what}` condition must be `bool`, found `{}`{hint}", c.ty));
        }
        Ok(c)
    }

    /// Can `t` be assigned to (`addr_of` false) or borrowed with `&mut`
    /// (`addr_of` true)?
    fn check_mutable(&self, t: &TExpr, addr_of: bool) -> Result<()> {
        let what = if addr_of { "take `&mut` of" } else { "assign to" };
        match &t.kind {
            TExprKind::Var(id) => {
                let v = &self.vars[*id];
                if !v.mutable {
                    return err(t.span, format!("cannot {what} `{}`: it is immutable (declare it with `var`)", v.name));
                }
                Ok(())
            }
            TExprKind::Field { base, .. } => self.check_mutable(base, addr_of),
            TExprKind::Index { base, .. } => {
                if matches!(base.ty, Type::Slice(_)) {
                    return err(t.span, if addr_of {
                        "cannot take `&mut` of a slice element: slice elements are read-only"
                    } else {
                        "cannot assign through a slice: slice elements are read-only"
                    });
                }
                self.check_mutable(base, addr_of)
            }
            TExprKind::Deref(p) => match &p.ty {
                Type::Ptr(inner, false) => err(t.span, format!(
                    "cannot {what} data behind a read-only pointer `{}` (use `*mut {inner}`)",
                    p.ty
                )),
                _ => Ok(()),
            },
            TExprKind::SliceLen(_) | TExprKind::ArrayLen(_) | TExprKind::SlicePtr(_) => {
                err(t.span, "cannot assign to a slice or array's `.len`/`.ptr`")
            }
            _ => err(t.span, "invalid assignment target"),
        }
    }

    /// Can we take the address of this expression (`&x`, or slicing an array)?
    fn is_lvalue(t: &TExpr) -> bool {
        match &t.kind {
            TExprKind::Var(_) | TExprKind::Deref(_) => true,
            TExprKind::Field { base, .. } => Self::is_lvalue(base),
            TExprKind::Index { base, .. } => matches!(base.ty, Type::Slice(_)) || Self::is_lvalue(base),
            _ => false,
        }
    }

    // ---------------------------------------------------------- expressions

    /// Check `e` and require its type to be `ty`. Two implicit coercions
    /// apply: a `T` is wrapped where `?T` is expected, and a `*mut T` is
    /// accepted where `*T` is expected.
    fn expect(&mut self, e: &ast::Expr, ty: &Type) -> Result<TExpr> {
        if *ty == Type::Error {
            return self.value(e, None);
        }
        // `null` and `match` arms need the optional type itself.
        let inner = match ty {
            Type::Optional(inner) if !matches!(strip_parens(e).kind, ExprKind::Null | ExprKind::Match { .. }) => {
                Some(&**inner)
            }
            _ => None,
        };
        let t = self.value(e, Some(inner.unwrap_or(ty)))?;
        if t.ty == *ty {
            return Ok(t);
        }
        if inner == Some(&t.ty) {
            let span = t.span;
            return Ok(TExpr { kind: TExprKind::Some(Box::new(t)), ty: ty.clone(), span });
        }
        if let (Type::Ptr(want, false), Type::Ptr(got, true)) = (ty, &t.ty)
            && want == got
        {
            return Ok(TExpr { ty: ty.clone(), ..t });
        }
        Err(mismatch(t.span, ty, &t.ty))
    }

    /// Check an expression that must produce a value (not a void call).
    fn value(&mut self, e: &ast::Expr, expected: Option<&Type>) -> Result<TExpr> {
        let t = self.expr(e, expected)?;
        if t.ty == Type::Error {
            return Err(silent());
        }
        if t.ty == Type::Void {
            return err(t.span, "this expression does not produce a value");
        }
        Ok(t)
    }

    /// Check two operands that must share a type; an untyped literal on either
    /// side takes the type of the other side.
    fn operand_pair(&mut self, l: &ast::Expr, r: &ast::Expr, hint: Option<&Type>) -> Result<(TExpr, TExpr)> {
        if is_untyped(l) && !is_untyped(r) {
            let rt = self.value(r, hint)?;
            let lt = self.value(l, Some(&rt.ty))?;
            Ok((lt, rt))
        } else {
            let lt = self.value(l, hint)?;
            let rt = self.value(r, Some(&lt.ty))?;
            Ok((lt, rt))
        }
    }

    fn int_lit(&self, v: i128, span: Span, expected: Option<&Type>) -> Result<TExpr> {
        let ty = match expected {
            Some(Type::Float(_)) => {
                return Ok(TExpr { kind: TExprKind::Float(v as f64), ty: expected.unwrap().clone(), span });
            }
            Some(t @ Type::Int(_)) => t.clone(),
            Some(t) => return err(span, format!("mismatched types: expected `{t}`, found integer literal")),
            None => I64,
        };
        let Type::Int(it) = ty else { unreachable!() };
        if v < it.min() || v > it.max() {
            return err(span, format!("integer literal `{v}` does not fit in `{ty}`"));
        }
        Ok(TExpr { kind: TExprKind::Int(v), ty, span })
    }

    fn float_lit(&self, v: f64, span: Span, expected: Option<&Type>) -> Result<TExpr> {
        let ty = match expected {
            Some(t @ Type::Float(_)) => t.clone(),
            Some(t) => return err(span, format!("mismatched types: expected `{t}`, found float literal")),
            None => Type::Float(FloatTy::F64),
        };
        Ok(TExpr { kind: TExprKind::Float(v), ty, span })
    }

    fn arith_operand_ok(&self, op: BinOp, ty: &Type, span: Span) -> Result<()> {
        let ok = match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => ty.is_numeric(),
            BinOp::Rem | BinOp::AddWrap | BinOp::SubWrap | BinOp::MulWrap => ty.is_int(),
            _ => unreachable!(),
        };
        if !ok {
            return err(span, format!("operator `{}` cannot be applied to `{ty}`", op.as_str()));
        }
        Ok(())
    }

    fn expr(&mut self, e: &ast::Expr, expected: Option<&Type>) -> Result<TExpr> {
        let span = e.span;
        let (kind, ty) = match &e.kind {
            ExprKind::Int(v) => return self.int_lit(*v as i128, span, expected),
            ExprKind::Char(c) => return self.int_lit(*c as i128, span, expected),
            ExprKind::Float(v) => return self.float_lit(*v, span, expected),
            ExprKind::Bool(b) => (TExprKind::Bool(*b), Type::Bool),
            ExprKind::Null => match expected {
                Some(t @ Type::Optional(_)) => (TExprKind::Null, t.clone()),
                Some(t) => {
                    return err(span, format!(
                        "mismatched types: expected `{t}`, found `null` (only optional types `?T` can be `null`)"
                    ));
                }
                None => {
                    return err(span, "cannot infer the type of `null` (add a type annotation, e.g. `let x: ?i32 = null`)");
                }
            },
            ExprKind::Match { .. } => return self.match_expr(e, expected, false),
            ExprKind::Str(s) => (TExprKind::Str(s.clone()), Type::Slice(Box::new(Type::Int(IntTy::U8)))),
            ExprKind::Paren(inner) => return self.expr(inner, expected),
            ExprKind::Ident(name) => match self.lookup(name) {
                Some(id) => (TExprKind::Var(id), self.vars[id].ty.clone()),
                None if self.fns.contains_key(name) => {
                    return err(span, format!("function `{name}` is not a value (call it with `{name}(...)`)"));
                }
                None => return err(span, format!("undefined variable `{name}`")),
            },
            ExprKind::Call { name, args } => {
                let Some(sig) = self.fns.get(name).cloned() else {
                    return err(span, format!("undefined function `{name}`"));
                };
                if args.len() != sig.params.len() {
                    return err(span, format!(
                        "function `{name}` takes {} argument(s) but {} were given",
                        sig.params.len(),
                        args.len()
                    ));
                }
                let args = args.iter().zip(&sig.params).map(|(a, p)| self.expect(a, p)).collect::<Result<_>>()?;
                (TExprKind::Call { name: name.clone(), is_extern: sig.is_extern, args }, sig.ret)
            }
            ExprKind::Builtin { name, args } => {
                let newline = match name.as_str() {
                    "print" => false,
                    "println" => true,
                    _ => return err(span, format!("unknown builtin `@{name}`")),
                };
                if args.len() > 1 || (!newline && args.is_empty()) {
                    return err(span, format!("`@{name}` takes {} argument", if newline { "at most one" } else { "one" }));
                }
                let arg = match args.first() {
                    Some(a) => {
                        let t = self.value(a, None)?;
                        let printable = match &t.ty {
                            Type::Int(_) | Type::Float(_) | Type::Bool => true,
                            Type::Slice(el) => **el == Type::Int(IntTy::U8),
                            _ => false,
                        };
                        if !printable {
                            return err(t.span, format!("`@{name}` cannot print a value of type `{}`", t.ty));
                        }
                        Some(Box::new(t))
                    }
                    None => None,
                };
                (TExprKind::Print { arg, newline }, Type::Void)
            }
            ExprKind::Binary { op: op @ (BinOp::Eq | BinOp::Ne), lhs, rhs } if is_null(lhs) || is_null(rhs) => {
                let other = if is_null(lhs) { rhs } else { lhs };
                let t = self.value(other, None)?;
                if !matches!(t.ty, Type::Optional(_)) {
                    return err(span, format!("only optional values (`?T`) can be compared with `null`, found `{}`", t.ty));
                }
                let is_some = TExpr { kind: TExprKind::IsSome(Box::new(t)), ty: Type::Bool, span };
                match op {
                    BinOp::Ne => (is_some.kind, Type::Bool),
                    _ => (TExprKind::Unary { op: UnOp::Not, expr: Box::new(is_some) }, Type::Bool),
                }
            }
            ExprKind::Binary { op, lhs, rhs } => {
                let op = *op;
                if matches!(op, BinOp::And | BinOp::Or) {
                    let l = self.expect(lhs, &Type::Bool)?;
                    let r = self.expect(rhs, &Type::Bool)?;
                    (TExprKind::Binary { op, lhs: Box::new(l), rhs: Box::new(r) }, Type::Bool)
                } else {
                    let hint = if op.is_arith() { expected.filter(|t| t.is_numeric()) } else { None };
                    let (l, r) = self.operand_pair(lhs, rhs, hint)?;
                    if l.ty != r.ty {
                        return err(span, format!(
                            "mismatched types in `{}`: `{}` and `{}` (use `as` to convert)",
                            op.as_str(),
                            l.ty,
                            r.ty
                        ));
                    }
                    let ty = if op.is_arith() {
                        self.arith_operand_ok(op, &l.ty, span)?;
                        l.ty.clone()
                    } else {
                        let ok = match op {
                            BinOp::Eq | BinOp::Ne => {
                                matches!(l.ty, Type::Int(_) | Type::Float(_) | Type::Bool)
                            }
                            _ => l.ty.is_numeric(),
                        };
                        if !ok {
                            return err(span, format!("operator `{}` cannot be applied to `{}`", op.as_str(), l.ty));
                        }
                        Type::Bool
                    };
                    (TExprKind::Binary { op, lhs: Box::new(l), rhs: Box::new(r) }, ty)
                }
            }
            ExprKind::Unary { op: UnOp::Neg, expr } => match &expr.kind {
                ExprKind::Int(v) => return self.int_lit(-(*v as i128), span, expected),
                ExprKind::Float(v) => return self.float_lit(-*v, span, expected),
                _ => {
                    let t = self.value(expr, expected.filter(|t| t.is_numeric()))?;
                    let ok = match &t.ty {
                        Type::Int(i) => i.signed(),
                        Type::Float(_) => true,
                        _ => false,
                    };
                    if !ok {
                        return err(span, format!("cannot negate a value of type `{}`", t.ty));
                    }
                    let ty = t.ty.clone();
                    (TExprKind::Unary { op: UnOp::Neg, expr: Box::new(t) }, ty)
                }
            },
            ExprKind::Unary { op: UnOp::Not, expr } => {
                let t = self.value(expr, Some(&Type::Bool))?;
                if t.ty != Type::Bool {
                    return err(span, format!("operator `!` requires `bool`, found `{}`", t.ty));
                }
                (TExprKind::Unary { op: UnOp::Not, expr: Box::new(t) }, Type::Bool)
            }
            ExprKind::AddrOf { mutable, expr } => {
                let t = self.value(expr, None)?;
                if !Self::is_lvalue(&t) {
                    return err(span, "cannot take the address of a temporary value (store it in a variable first)");
                }
                if *mutable {
                    self.check_mutable(&t, true)?;
                }
                let ty = Type::Ptr(Box::new(t.ty.clone()), *mutable);
                (TExprKind::AddrOf(Box::new(t)), ty)
            }
            ExprKind::Deref(inner) => {
                let t = self.value(inner, None)?;
                let Type::Ptr(pointee, _) = &t.ty else {
                    return err(span, format!("cannot dereference a value of type `{}` (`.*` needs a pointer)", t.ty));
                };
                let ty = (**pointee).clone();
                (TExprKind::Deref(Box::new(t)), ty)
            }
            ExprKind::Cast { expr, ty } => {
                let to = self.resolve_type(ty)?;
                let t = self.value(expr, None)?;
                if !t.ty.is_numeric() || !to.is_numeric() {
                    return err(span, format!("cannot cast `{}` to `{to}` (only numeric casts are supported)", t.ty));
                }
                (TExprKind::Cast(Box::new(t)), to)
            }
            // `Enum.Variant` without payload.
            ExprKind::Field { base, name } if matches!(&base.kind, ExprKind::Ident(n) if self.enums.contains_key(n)) => {
                let ExprKind::Ident(ename) = &base.kind else { unreachable!() };
                let def = &self.enums[ename];
                let Some(index) = def.variants.iter().position(|v| v.name == *name) else {
                    return err(span, format!("enum `{ename}` has no variant `{name}`"));
                };
                if !def.variants[index].fields.is_empty() {
                    return err(span, format!("variant `{ename}.{name}` has fields; write `{ename}.{name} {{ ... }}`"));
                }
                (TExprKind::Variant { index, fields: vec![] }, Type::Enum(ename.clone()))
            }
            ExprKind::Field { base, name } => {
                let mut b = self.value(base, None)?;
                // Field access dereferences one pointer level.
                if let Type::Ptr(pointee, _) = &b.ty {
                    let (ty, span) = ((**pointee).clone(), b.span);
                    b = TExpr { kind: TExprKind::Deref(Box::new(b)), ty, span };
                }
                match &b.ty {
                    Type::Struct(sname) => {
                        let def = &self.structs[sname];
                        let Some(index) = def.fields.iter().position(|(n, _)| n == name) else {
                            return err(span, format!("struct `{sname}` has no field `{name}`"));
                        };
                        let ty = def.fields[index].1.clone();
                        (TExprKind::Field { base: Box::new(b), index }, ty)
                    }
                    Type::Slice(el) if name == "len" => {
                        let _ = el;
                        (TExprKind::SliceLen(Box::new(b)), USIZE)
                    }
                    Type::Slice(el) if name == "ptr" => {
                        let ty = Type::CPtr(el.clone());
                        (TExprKind::SlicePtr(Box::new(b)), ty)
                    }
                    Type::Array(..) if name == "len" => (TExprKind::ArrayLen(Box::new(b)), USIZE),
                    t => return err(span, format!("type `{t}` has no field `{name}`")),
                }
            }
            ExprKind::Index { base, index } => {
                let b = self.value(base, None)?;
                let elem = match &b.ty {
                    Type::Array(el, _) | Type::Slice(el) => (**el).clone(),
                    t => return err(span, format!("cannot index into a value of type `{t}`")),
                };
                let i = self.value(index, Some(&USIZE))?;
                if !is_unsigned(&i.ty) {
                    return err(i.span, format!("index must be an unsigned integer, found `{}`", i.ty));
                }
                (TExprKind::Index { base: Box::new(b), index: Box::new(i) }, elem)
            }
            ExprKind::Slice { base, lo, hi } => {
                let b = self.value(base, None)?;
                let elem = match &b.ty {
                    Type::Array(el, _) => {
                        if !Self::is_lvalue(&b) {
                            return err(span, "can only slice an array stored in a variable");
                        }
                        (**el).clone()
                    }
                    Type::Slice(el) => (**el).clone(),
                    t => return err(span, format!("cannot slice a value of type `{t}`")),
                };
                let mut bound = |x: &Option<Box<ast::Expr>>| -> Result<Option<Box<TExpr>>> {
                    match x {
                        None => Ok(None),
                        Some(x) => {
                            let t = self.value(x, Some(&USIZE))?;
                            if !is_unsigned(&t.ty) {
                                return err(t.span, format!("slice bound must be an unsigned integer, found `{}`", t.ty));
                            }
                            Ok(Some(Box::new(t)))
                        }
                    }
                };
                let lo = bound(lo)?;
                let hi = bound(hi)?;
                (TExprKind::Slice { base: Box::new(b), lo, hi }, Type::Slice(Box::new(elem)))
            }
            ExprKind::StructLit { name, fields } => {
                let Some(def) = self.structs.get(name).cloned() else {
                    return err(span, format!("unknown struct `{name}`"));
                };
                let inits = self.field_inits(&def.fields, fields, &format!("struct `{name}`"), name, span)?;
                (TExprKind::StructLit(inits), Type::Struct(name.clone()))
            }
            ExprKind::VariantLit { enum_name, variant, fields } => {
                let Some(def) = self.enums.get(enum_name).cloned() else {
                    return err(span, format!("unknown enum `{enum_name}`"));
                };
                let Some(index) = def.variants.iter().position(|v| v.name == *variant) else {
                    return err(span, format!("enum `{enum_name}` has no variant `{variant}`"));
                };
                let v = &def.variants[index];
                let lit = format!("{enum_name}.{variant}");
                if v.fields.is_empty() {
                    return err(span, format!("variant `{lit}` has no fields; write `{lit}`"));
                }
                let inits = self.field_inits(&v.fields, fields, &format!("variant `{lit}`"), &lit, span)?;
                (TExprKind::Variant { index, fields: inits }, Type::Enum(enum_name.clone()))
            }
            ExprKind::ArrayLit { ty, elems } => {
                let aty = self.resolve_type(ty)?;
                let Type::Array(el, n) = &aty else { unreachable!() };
                if elems.len() as u64 != *n {
                    return err(span, format!("array literal has {} element(s) but type `{aty}` needs {n}", elems.len()));
                }
                let elems = elems.iter().map(|x| self.expect(x, el)).collect::<Result<_>>()?;
                (TExprKind::ArrayLit(elems), aty)
            }
        };
        Ok(TExpr { kind, ty, span })
    }

    /// Field initializers of a struct or variant literal, in source order.
    /// `owner` names the struct or variant in messages, `lit` the literal.
    fn field_inits(
        &mut self,
        def: &[(String, Type)],
        fields: &[ast::FieldInit],
        owner: &str,
        lit: &str,
        span: Span,
    ) -> Result<Vec<(usize, TExpr)>> {
        let mut inits: Vec<(usize, TExpr)> = Vec::new();
        for f in fields {
            let Some(index) = def.iter().position(|(n, _)| *n == f.name) else {
                return err(f.span, format!("{owner} has no field `{}`", f.name));
            };
            if inits.iter().any(|(i, _)| *i == index) {
                return err(f.span, format!("field `{}` is initialized twice", f.name));
            }
            let v = self.expect(&f.value, &def[index].1)?;
            inits.push((index, v));
        }
        if let Some((missing, _)) =
            def.iter().enumerate().find(|(i, _)| !inits.iter().any(|(j, _)| j == i)).map(|(_, f)| f)
        {
            return err(span, format!("missing field `{missing}` in `{lit}` literal"));
        }
        Ok(inits)
    }

    /// `match` on an enum. As a statement (`is_stmt`) the arms produce no
    /// value; otherwise all arms must have the same type.
    fn match_expr(&mut self, e: &ast::Expr, expected: Option<&Type>, is_stmt: bool) -> Result<TExpr> {
        let ExprKind::Match { scrutinee, arms } = &e.kind else { unreachable!() };
        let s = self.value(scrutinee, None)?;
        let Type::Enum(ename) = &s.ty else {
            return err(s.span, format!("`match` needs an enum value, found `{}`", s.ty));
        };
        let def = self.enums[ename].clone();
        // Patterns, in source order.
        let mut variants = Vec::new();
        let mut covered = vec![false; def.variants.len()];
        let mut wildcard = false;
        for arm in arms {
            if wildcard {
                return err(arm.span, "unreachable arm: the `_` arm above already matches everything");
            }
            match &arm.variant {
                None => {
                    wildcard = true;
                    variants.push(None);
                }
                Some(name) => {
                    let Some(i) = def.variants.iter().position(|v| v.name == *name) else {
                        return err(arm.span, format!("enum `{ename}` has no variant `{name}`"));
                    };
                    if covered[i] {
                        return err(arm.span, format!("variant `{ename}.{name}` is already matched by an earlier arm"));
                    }
                    covered[i] = true;
                    variants.push(Some(i));
                }
            }
        }
        if !wildcard {
            let missing: Vec<String> = def
                .variants
                .iter()
                .zip(&covered)
                .filter(|(_, c)| !**c)
                .map(|(v, _)| format!("`{ename}.{}`", v.name))
                .collect();
            if !missing.is_empty() {
                let s = if missing.len() == 1 { "" } else { "s" };
                return err(e.span, format!("`match` is not exhaustive: missing variant{s} {}", missing.join(", ")));
            }
        }
        // Without an expected type, the first arm that is not an untyped
        // literal or `null` decides the type of the `match`.
        let mut want = if is_stmt { None } else { expected.cloned() };
        let mut order: Vec<usize> = (0..arms.len()).collect();
        if !is_stmt
            && want.is_none()
            && let Some(i) = arms.iter().position(|a| matches!(&a.body, ArmBody::Expr(x) if !is_untyped(x) && !is_null(x)))
        {
            order.remove(i);
            order.insert(0, i);
        }
        let mut checked: Vec<Option<TArm>> = vec![None; arms.len()];
        for i in order {
            self.scopes.push(HashMap::new());
            let arm = self.match_arm(&def, &arms[i], variants[i], want.as_ref(), is_stmt);
            self.scopes.pop();
            let arm = arm?;
            if let (false, None, TArmBody::Expr(x)) = (is_stmt, &want, &arm.body) {
                want = Some(x.ty.clone());
            }
            checked[i] = Some(arm);
        }
        let ty = if is_stmt { Type::Void } else { want.unwrap_or(Type::Error) };
        let arms = checked.into_iter().map(Option::unwrap).collect();
        Ok(TExpr { kind: TExprKind::Match { scrutinee: Box::new(s), arms }, ty, span: e.span })
    }

    /// One arm: declare the pattern variables, then check the body.
    fn match_arm(
        &mut self,
        def: &EnumDef,
        arm: &ast::MatchArm,
        variant: Option<usize>,
        want: Option<&Type>,
        is_stmt: bool,
    ) -> Result<TArm> {
        let mut bindings: Vec<(VarId, usize)> = Vec::new();
        if let Some(vi) = variant {
            let v = &def.variants[vi];
            let what = format!("{}.{}", def.name, v.name);
            match (&arm.fields, v.fields.is_empty()) {
                (None, true) => {}
                (None, false) => {
                    return err(arm.span, format!("variant `{what}` has fields; match them with `.{} {{ .. }}`", v.name));
                }
                (Some(_), true) => return err(arm.span, format!("variant `{what}` has no fields; write `.{}`", v.name)),
                (Some(pats), false) => {
                    for p in pats {
                        let Some(fi) = v.fields.iter().position(|(n, _)| *n == p.field) else {
                            return err(p.span, format!("variant `{what}` has no field `{}`", p.field));
                        };
                        if bindings.iter().any(|&(_, f)| f == fi) {
                            return err(p.span, format!("field `{}` is matched twice", p.field));
                        }
                        let id = self.declare(&p.binding, v.fields[fi].1.clone(), false, p.binding_span)?;
                        bindings.push((id, fi));
                    }
                    if !arm.rest
                        && let Some((name, _)) =
                            v.fields.iter().enumerate().find(|(i, _)| !bindings.iter().any(|&(_, f)| f == *i)).map(|(_, f)| f)
                    {
                        return err(arm.span, format!(
                            "pattern for `{what}` does not mention field `{name}` (add `..` to ignore the rest)"
                        ));
                    }
                }
            }
        }
        let body = match &arm.body {
            ArmBody::Block(b) if is_stmt => TArmBody::Block(self.block(b)),
            ArmBody::Block(b) => {
                return err(b.span, "a `match` that produces a value needs an expression after `=>`, not a block");
            }
            ArmBody::Expr(x) if is_stmt => TArmBody::Expr(self.expr_stmt(x)?),
            ArmBody::Expr(x) => TArmBody::Expr(match want {
                Some(t) => self.expect(x, t)?,
                None => self.value(x, None)?,
            }),
        };
        Ok(TArm { variant, bindings, body })
    }
}
