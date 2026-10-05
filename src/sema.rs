//! Semantic analysis: name resolution and type checking.
//! Produces a typed AST (`Program`) in which every expression carries its
//! concrete type; untyped literals have already been given their type.

use crate::ast::{self, AssignOp, BinOp, ExprKind, StmtKind, TypeExprKind, UnOp};
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
    Struct(String),
}

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
            Type::Struct(n) => f.write_str(n),
        }
    }
}

// -------------------------------------------------------------- typed AST

pub type VarId = usize;

#[derive(Debug, Clone)]
pub struct Program {
    pub structs: Vec<StructDef>,
    pub externs: Vec<ExternFn>,
    pub fns: Vec<FnDef>,
}

#[derive(Debug, Clone)]
pub struct StructDef {
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
    If { cond: TExpr, then: Vec<TStmt>, els: Vec<TStmt> },
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
    ArrayLit(Vec<TExpr>),
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
    fns: HashMap<String, FnSig>,
    // Per-function state.
    vars: Vec<VarInfo>,
    scopes: Vec<HashMap<String, VarId>>,
    ret: Type,
    loop_depth: u32,
    in_defer: bool,
}

pub fn check(file: &ast::File) -> Result<Program> {
    let mut c = Checker {
        structs: HashMap::new(),
        fns: HashMap::new(),
        vars: Vec::new(),
        scopes: Vec::new(),
        ret: Type::Void,
        loop_depth: 0,
        in_defer: false,
    };
    c.program(file)
}

fn err<T>(span: Span, msg: impl Into<String>) -> Result<T> {
    Err(Error::new(span, msg))
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

fn is_unsigned(t: &Type) -> bool {
    matches!(t, Type::Int(i) if !i.signed())
}

fn block_returns(stmts: &[TStmt]) -> bool {
    stmts.iter().any(|s| match &s.kind {
        TStmtKind::Return(_) => true,
        TStmtKind::Block(b) => block_returns(b),
        TStmtKind::If { then, els, .. } => block_returns(then) && block_returns(els),
        _ => false,
    })
}

impl Checker {
    fn program(&mut self, file: &ast::File) -> Result<Program> {
        // 1. Struct names.
        let mut struct_order = Vec::new();
        let mut top_spans: HashMap<String, Span> = HashMap::new();
        for item in &file.items {
            let (name, span) = match item {
                ast::Item::Struct(s) => (&s.name, s.span),
                ast::Item::Fn(f) => (&f.name, f.span),
            };
            if Type::from_name(name).is_some() || name == "true" || name == "false" {
                return err(span, format!("`{name}` is a builtin name and cannot be redefined"));
            }
            if top_spans.insert(name.clone(), span).is_some() {
                return err(span, format!("`{name}` is defined more than once"));
            }
            if let ast::Item::Struct(s) = item {
                self.structs.insert(s.name.clone(), StructDef { name: s.name.clone(), fields: vec![] });
                struct_order.push(s);
            }
        }
        // 2. Struct fields.
        for s in &struct_order {
            let mut fields: Vec<(String, Type)> = Vec::new();
            for f in &s.fields {
                if fields.iter().any(|(n, _)| *n == f.name) {
                    return err(f.span, format!("duplicate field `{}` in struct `{}`", f.name, s.name));
                }
                fields.push((f.name.clone(), self.resolve_type(&f.ty)?));
            }
            self.structs.get_mut(&s.name).unwrap().fields = fields;
        }
        for s in &struct_order {
            self.check_recursive(&s.name, &s.name, s.span, &mut Vec::new())?;
        }
        // 3. Function signatures.
        let mut main_found = false;
        let mut externs = Vec::new();
        for item in &file.items {
            let ast::Item::Fn(f) = item else { continue };
            let mut params = Vec::new();
            for p in &f.params {
                let ty = self.resolve_type(&p.ty)?;
                if f.is_extern && !matches!(ty, Type::Int(_) | Type::Float(_) | Type::Bool | Type::CPtr(_)) {
                    return err(p.ty.span, format!("type `{ty}` cannot be used in an `extern fn` signature"));
                }
                params.push(ty);
            }
            let ret = match &f.ret {
                Some(t) => self.resolve_type(t)?,
                None => Type::Void,
            };
            if f.is_extern {
                if matches!(ret, Type::Array(..) | Type::Slice(_) | Type::Struct(_)) {
                    return err(f.ret.as_ref().unwrap().span, format!("type `{ret}` cannot be used in an `extern fn` signature"));
                }
                externs.push(ExternFn { name: f.name.clone(), params: params.clone(), ret: ret.clone() });
            }
            if f.name == "main" && !f.is_extern {
                if !f.is_pub {
                    return err(f.span, "`main` must be declared `pub fn main()`");
                }
                if !params.is_empty() || ret != Type::Void {
                    return err(f.span, "`main` must take no parameters and have no return type");
                }
                main_found = true;
            }
            self.fns.insert(f.name.clone(), FnSig { params, ret, is_extern: f.is_extern });
        }
        if !main_found {
            return err(Span::new(1, 1), "no `pub fn main()` found");
        }
        // 4. Bodies.
        let mut fns = Vec::new();
        for item in &file.items {
            if let ast::Item::Fn(f) = item
                && !f.is_extern
            {
                fns.push(self.fn_body(f)?);
            }
        }
        let structs = struct_order.iter().map(|s| self.structs[&s.name].clone()).collect();
        Ok(Program { structs, externs, fns })
    }

    fn check_recursive(&self, root: &str, name: &str, span: Span, path: &mut Vec<String>) -> Result<()> {
        if path.iter().any(|p| p == name) {
            return Ok(());
        }
        path.push(name.to_string());
        for (_, ty) in &self.structs[name].fields {
            let mut t = ty;
            while let Type::Array(inner, _) = t {
                t = inner;
            }
            if let Type::Struct(n) = t {
                if n == root {
                    return err(span, format!("struct `{root}` contains itself by value (infinite size)"));
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
                None => return err(t.span, format!("unknown type `{n}`")),
            },
            TypeExprKind::Array(n, elem) => Type::Array(Box::new(self.resolve_type(elem)?), *n),
            TypeExprKind::Slice(elem) => Type::Slice(Box::new(self.resolve_type(elem)?)),
            TypeExprKind::CPtr(elem) => Type::CPtr(Box::new(self.resolve_type(elem)?)),
        })
    }

    // ------------------------------------------------------------ functions

    fn declare(&mut self, name: &str, ty: Type, mutable: bool, span: Span) -> Result<VarId> {
        let scope = self.scopes.last_mut().unwrap();
        if scope.contains_key(name) {
            return err(span, format!("`{name}` is already declared in this scope"));
        }
        let id = self.vars.len();
        scope.insert(name.to_string(), id);
        self.vars.push(VarInfo { name: name.to_string(), ty, mutable });
        Ok(id)
    }

    fn lookup(&self, name: &str) -> Option<VarId> {
        self.scopes.iter().rev().find_map(|s| s.get(name).copied())
    }

    fn fn_body(&mut self, f: &ast::FnDecl) -> Result<FnDef> {
        let sig = self.fns[&f.name].clone();
        self.vars.clear();
        self.scopes = vec![HashMap::new()];
        self.ret = sig.ret.clone();
        self.loop_depth = 0;
        self.in_defer = false;
        let mut params = Vec::new();
        for (p, ty) in f.params.iter().zip(&sig.params) {
            params.push(self.declare(&p.name, ty.clone(), false, p.span)?);
        }
        let body = self.block(f.body.as_ref().unwrap())?;
        if sig.ret != Type::Void && !block_returns(&body) {
            return err(f.span, format!("function `{}` must return a value of type `{}` on every path", f.name, sig.ret));
        }
        Ok(FnDef {
            name: f.name.clone(),
            params,
            ret: sig.ret,
            vars: std::mem::take(&mut self.vars),
            body,
            is_main: f.name == "main",
        })
    }

    fn block(&mut self, b: &ast::Block) -> Result<Vec<TStmt>> {
        self.scopes.push(HashMap::new());
        let r = b.stmts.iter().map(|s| self.stmt(s)).collect();
        self.scopes.pop();
        r
    }

    fn stmt(&mut self, s: &ast::Stmt) -> Result<TStmt> {
        let kind = match &s.kind {
            StmtKind::Let { name, mutable, ty, init } => {
                let init = match ty {
                    Some(t) => {
                        let ty = self.resolve_type(t)?;
                        self.expect(init, &ty)?
                    }
                    None => self.value(init, None)?,
                };
                let id = self.declare(name, init.ty.clone(), *mutable, s.span)?;
                TStmtKind::Let(id, init)
            }
            StmtKind::Assign { target, op, value } => {
                let target = self.value(target, None)?;
                self.check_assignable(&target)?;
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
            StmtKind::If { cond, then, els } => {
                let cond = self.condition(cond, "if")?;
                let then = self.block(then)?;
                let els = match els {
                    Some(e) => match &e.kind {
                        StmtKind::Block(b) => self.block(b)?,
                        _ => vec![self.stmt(e)?],
                    },
                    None => vec![],
                };
                TStmtKind::If { cond, then, els }
            }
            StmtKind::While { cond, body } => {
                let cond = self.condition(cond, "while")?;
                self.loop_depth += 1;
                let body = self.block(body);
                self.loop_depth -= 1;
                TStmtKind::While { cond, body: body? }
            }
            StmtKind::For { var, var_span, lo, hi, body } => {
                let (lo, hi) = self.operand_pair(lo, hi, None)?;
                if lo.ty != hi.ty {
                    return err(hi.span, format!("range bounds have different types: `{}` and `{}`", lo.ty, hi.ty));
                }
                if !lo.ty.is_int() {
                    return err(lo.span, format!("range bounds must be integers, found `{}`", lo.ty));
                }
                self.scopes.push(HashMap::new());
                let var = self.declare(var, lo.ty.clone(), false, *var_span)?;
                self.loop_depth += 1;
                let body = self.block(body);
                self.loop_depth -= 1;
                self.scopes.pop();
                TStmtKind::For { var, lo, hi, body: body? }
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
                    StmtKind::Block(b) => self.block(b),
                    _ => self.stmt(inner).map(|s| vec![s]),
                };
                (self.loop_depth, self.in_defer) = saved;
                TStmtKind::Defer(body?)
            }
            StmtKind::Block(b) => TStmtKind::Block(self.block(b)?),
            StmtKind::Expr(e) => {
                if !matches!(e.kind, ExprKind::Call { .. } | ExprKind::Builtin { .. }) {
                    return err(e.span, "expression value is not used (only calls can be statements)");
                }
                TStmtKind::Expr(self.expr(e, None)?)
            }
        };
        Ok(TStmt { kind, span: s.span })
    }

    fn condition(&mut self, e: &ast::Expr, what: &str) -> Result<TExpr> {
        let c = self.value(e, Some(&Type::Bool))?;
        if c.ty != Type::Bool {
            return err(c.span, format!("`{what}` condition must be `bool`, found `{}`", c.ty));
        }
        Ok(c)
    }

    fn check_assignable(&self, t: &TExpr) -> Result<()> {
        match &t.kind {
            TExprKind::Var(id) => {
                let v = &self.vars[*id];
                if !v.mutable {
                    return err(t.span, format!("cannot assign to `{}`: it is immutable (declare it with `var`)", v.name));
                }
                Ok(())
            }
            TExprKind::Field { base, .. } => self.check_assignable(base),
            TExprKind::Index { base, .. } => {
                if matches!(base.ty, Type::Slice(_)) {
                    return err(t.span, "cannot assign through a slice: slice elements are read-only");
                }
                self.check_assignable(base)
            }
            TExprKind::SliceLen(_) | TExprKind::ArrayLen(_) | TExprKind::SlicePtr(_) => {
                err(t.span, "cannot assign to a slice or array's `.len`/`.ptr`")
            }
            _ => err(t.span, "invalid assignment target"),
        }
    }

    /// Can we take the address of this expression (needed to slice an array)?
    fn is_lvalue(t: &TExpr) -> bool {
        match &t.kind {
            TExprKind::Var(_) => true,
            TExprKind::Field { base, .. } => Self::is_lvalue(base),
            TExprKind::Index { base, .. } => matches!(base.ty, Type::Slice(_)) || Self::is_lvalue(base),
            _ => false,
        }
    }

    // ---------------------------------------------------------- expressions

    /// Check `e` and require its type to be exactly `ty`.
    fn expect(&mut self, e: &ast::Expr, ty: &Type) -> Result<TExpr> {
        let t = self.value(e, Some(ty))?;
        if t.ty != *ty {
            return Err(mismatch(t.span, ty, &t.ty));
        }
        Ok(t)
    }

    /// Check an expression that must produce a value (not a void call).
    fn value(&mut self, e: &ast::Expr, expected: Option<&Type>) -> Result<TExpr> {
        let t = self.expr(e, expected)?;
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
            ExprKind::Cast { expr, ty } => {
                let to = self.resolve_type(ty)?;
                let t = self.value(expr, None)?;
                if !t.ty.is_numeric() || !to.is_numeric() {
                    return err(span, format!("cannot cast `{}` to `{to}` (only numeric casts are supported)", t.ty));
                }
                (TExprKind::Cast(Box::new(t)), to)
            }
            ExprKind::Field { base, name } => {
                let b = self.value(base, None)?;
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
                let mut inits: Vec<(usize, TExpr)> = Vec::new();
                for f in fields {
                    let Some(index) = def.fields.iter().position(|(n, _)| *n == f.name) else {
                        return err(f.span, format!("struct `{name}` has no field `{}`", f.name));
                    };
                    if inits.iter().any(|(i, _)| *i == index) {
                        return err(f.span, format!("field `{}` is initialized twice", f.name));
                    }
                    let v = self.expect(&f.value, &def.fields[index].1)?;
                    inits.push((index, v));
                }
                if let Some((missing, _)) =
                    def.fields.iter().enumerate().find(|(i, _)| !inits.iter().any(|(j, _)| j == i)).map(|(_, f)| f)
                {
                    return err(span, format!("missing field `{missing}` in `{name}` literal"));
                }
                (TExprKind::StructLit(inits), Type::Struct(name.clone()))
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
}
