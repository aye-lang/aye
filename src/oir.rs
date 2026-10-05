//! OIR: Ore's typed intermediate representation, between sema and the C
//! backend.
//!
//! Lowering from the typed AST:
//! - flattens expressions so every operand is a local or a constant, which
//!   fixes left-to-right evaluation order (C leaves argument order unspecified);
//! - lowers `&&`/`||` to `if`, `while`/`for` to `Loop`, and `x op= v` to `x = x op v`;
//! - expands `defer`: deferred statements are copied to every scope exit
//!   (end of block, `return`, `break`, `continue`) in LIFO order;
//! - makes runtime checks explicit: checked arithmetic carries its source
//!   span, and indexing carries the length to check against;
//! - lowers `match` to an `if` chain on the enum tag.

use crate::ast::{BinOp, UnOp};
use crate::diag::Span;
use crate::sema::{self, FloatTy, IntTy, TArmBody, TExpr, TExprKind, TStmt, TStmtKind, Type};
use std::collections::HashMap;

pub type LocalId = usize;

#[derive(Debug, Clone)]
pub struct Program {
    pub structs: Vec<sema::StructDef>,
    pub enums: Vec<sema::EnumDef>,
    pub externs: Vec<sema::ExternFn>,
    pub funcs: Vec<Func>,
}

#[derive(Debug, Clone)]
pub struct Func {
    pub name: String,
    pub params: Vec<LocalId>,
    pub ret: Type,
    pub locals: Vec<Local>,
    pub body: Block,
    pub is_main: bool,
}

/// A function-wide local slot: a user variable (`name` set) or a temporary.
#[derive(Debug, Clone)]
pub struct Local {
    pub name: Option<String>,
    pub ty: Type,
}

pub type Block = Vec<Stmt>;

#[derive(Debug, Clone)]
pub struct Stmt {
    pub kind: StmtKind,
    /// Source position of the Ore statement this was lowered from.
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum StmtKind {
    Assign(Place, Rvalue),
    /// Evaluate for side effects (calls, prints).
    Eval(Rvalue),
    If { cond: Operand, then: Block, els: Block },
    /// Infinite loop. `cont` runs after `body` and on `continue`.
    Loop { body: Block, cont: Block },
    Break,
    Continue,
    Return(Option<Operand>),
}

#[derive(Debug, Clone)]
pub enum Operand {
    Local(LocalId),
    Int(i128, IntTy),
    Float(f64, FloatTy),
    Bool(bool),
    /// String literal, type `[]u8`.
    Str(Vec<u8>),
}

/// A memory location: a local followed by field/index projections.
#[derive(Debug, Clone)]
pub struct Place {
    pub local: LocalId,
    pub proj: Vec<Proj>,
}

#[derive(Debug, Clone)]
pub enum Proj {
    /// Struct field, or `len`/`ptr` of a slice.
    Field(String),
    /// Bounds-checked element access.
    Index { index: Operand, len: Len, span: Span },
    /// Dereference of a `*T`.
    Deref,
}

/// Is `t` a `?*T`? It is represented as a C pointer that may be null;
/// every other `?T` is a `{ has, val }` struct.
pub fn is_ptr_optional(t: &Type) -> bool {
    matches!(t, Type::Optional(inner) if matches!(**inner, Type::Ptr(..)))
}

/// Length of the indexed/sliced sequence.
#[derive(Debug, Clone, Copy)]
pub enum Len {
    /// Fixed array `[N]T`.
    Array(u64),
    /// Slice `[]T`, length read at runtime.
    Slice,
}

#[derive(Debug, Clone)]
pub enum Rvalue {
    Use(Operand),
    Load(Place),
    /// Arithmetic or comparison; `ty` is the operand type. Never `&&`/`||`.
    Binary { op: BinOp, lhs: Operand, rhs: Operand, ty: Type, span: Span },
    Neg { val: Operand, ty: Type, span: Span },
    Not(Operand),
    Cast { val: Operand, from: Type, to: Type, span: Span },
    Call { name: String, is_extern: bool, args: Vec<Operand> },
    Print { arg: Option<(Operand, Type)>, newline: bool },
    /// Fields in declaration order.
    StructLit { name: String, fields: Vec<Operand> },
    /// Enum value; payload fields in declaration order.
    Variant { enum_name: String, tag: usize, variant: String, fields: Vec<Operand> },
    AddrOf(Place),
    /// Wrap a value into the optional type `ty`.
    Some { val: Operand, ty: Type },
    Null(Type),
    /// Is the optional at `place` (of type `ty`) non-null?
    IsSome { place: Place, ty: Type },
    ArrayLit { ty: Type, elems: Vec<Operand> },
    /// `base[lo..hi]`, producing a slice. `hi = None` means the full length.
    Slice { base: Place, len: Len, elem: Type, lo: Operand, hi: Option<Operand>, span: Span },
}

pub fn lower(p: &sema::Program) -> Program {
    let structs: HashMap<&str, &sema::StructDef> = p.structs.iter().map(|s| (s.name.as_str(), s)).collect();
    let enums: HashMap<&str, &sema::EnumDef> = p.enums.iter().map(|e| (e.name.as_str(), e)).collect();
    let funcs = p.fns.iter().map(|f| Lower::func(f, &structs, &enums)).collect();
    Program { structs: p.structs.clone(), enums: p.enums.clone(), externs: p.externs.clone(), funcs }
}

struct Scope<'a> {
    defers: Vec<&'a [TStmt]>,
    /// The body scope of a loop: `break`/`continue` exit up to here.
    is_loop: bool,
}

struct Lower<'a> {
    structs: &'a HashMap<&'a str, &'a sema::StructDef>,
    enums: &'a HashMap<&'a str, &'a sema::EnumDef>,
    locals: Vec<Local>,
    scopes: Vec<Scope<'a>>,
    span: Span,
}

fn is_terminator(s: Option<&Stmt>) -> bool {
    matches!(s.map(|s| &s.kind), Some(StmtKind::Break | StmtKind::Continue | StmtKind::Return(_)))
}

fn has_call(e: &TExpr) -> bool {
    match &e.kind {
        TExprKind::Call { .. } | TExprKind::Print { .. } | TExprKind::Match { .. } => true,
        TExprKind::Int(_)
        | TExprKind::Float(_)
        | TExprKind::Bool(_)
        | TExprKind::Str(_)
        | TExprKind::Var(_)
        | TExprKind::Null => false,
        TExprKind::Binary { lhs, rhs, .. } | TExprKind::Index { base: lhs, index: rhs } => has_call(lhs) || has_call(rhs),
        TExprKind::Unary { expr, .. }
        | TExprKind::Cast(expr)
        | TExprKind::Field { base: expr, .. }
        | TExprKind::SliceLen(expr)
        | TExprKind::SlicePtr(expr)
        | TExprKind::ArrayLen(expr)
        | TExprKind::Some(expr)
        | TExprKind::IsSome(expr)
        | TExprKind::AddrOf(expr)
        | TExprKind::Deref(expr) => has_call(expr),
        TExprKind::Slice { base, lo, hi } => {
            has_call(base) || lo.as_deref().is_some_and(has_call) || hi.as_deref().is_some_and(has_call)
        }
        TExprKind::StructLit(fs) | TExprKind::Variant { fields: fs, .. } => fs.iter().any(|(_, e)| has_call(e)),
        TExprKind::ArrayLit(es) => es.iter().any(has_call),
    }
}

impl<'a> Lower<'a> {
    fn func(
        f: &'a sema::FnDef,
        structs: &'a HashMap<&'a str, &'a sema::StructDef>,
        enums: &'a HashMap<&'a str, &'a sema::EnumDef>,
    ) -> Func {
        // Sema variables keep their ids as OIR locals; temporaries follow.
        let locals = f.vars.iter().map(|v| Local { name: Some(v.name.clone()), ty: v.ty.clone() }).collect();
        let mut l = Lower { structs, enums, locals, scopes: Vec::new(), span: Span::default() };
        let mut body = Vec::new();
        l.block(&f.body, false, &mut body);
        Func {
            name: f.name.clone(),
            params: f.params.clone(),
            ret: f.ret.clone(),
            locals: l.locals,
            body,
            is_main: f.is_main,
        }
    }

    fn temp(&mut self, ty: Type) -> LocalId {
        self.locals.push(Local { name: None, ty });
        self.locals.len() - 1
    }

    fn push(&self, out: &mut Block, kind: StmtKind) {
        out.push(Stmt { kind, span: self.span });
    }

    fn block(&mut self, stmts: &'a [TStmt], is_loop: bool, out: &mut Block) {
        self.scopes.push(Scope { defers: Vec::new(), is_loop });
        for s in stmts {
            self.stmt(s, out);
        }
        let scope = self.scopes.pop().unwrap();
        if !is_terminator(out.last()) {
            for d in scope.defers.iter().rev() {
                self.block(d, false, out);
            }
        }
    }

    /// Emit the deferred statements of the innermost scopes, innermost first,
    /// stopping after the loop body scope when `to_loop` is set.
    fn run_defers(&mut self, to_loop: bool, out: &mut Block) {
        let mut pending: Vec<&'a [TStmt]> = Vec::new();
        for scope in self.scopes.iter().rev() {
            pending.extend(scope.defers.iter().rev());
            if to_loop && scope.is_loop {
                break;
            }
        }
        for d in pending {
            self.block(d, false, out);
        }
    }

    fn has_defers(&self) -> bool {
        self.scopes.iter().any(|s| !s.defers.is_empty())
    }

    fn stmt(&mut self, s: &'a TStmt, out: &mut Block) {
        self.span = s.span;
        match &s.kind {
            TStmtKind::Let(id, init) => {
                let rv = self.rvalue(init, out);
                self.push(out, StmtKind::Assign(Place { local: *id, proj: vec![] }, rv));
            }
            TStmtKind::Assign { target, op: None, value } => {
                let place = self.place(target, out);
                let rv = self.rvalue(value, out);
                self.push(out, StmtKind::Assign(place, rv));
            }
            TStmtKind::Assign { target, op: Some(op), value } => {
                let place = self.place(target, out);
                let cur = if place.proj.is_empty() {
                    Operand::Local(place.local)
                } else {
                    let t = self.temp(target.ty.clone());
                    self.push(out, StmtKind::Assign(Place { local: t, proj: vec![] }, Rvalue::Load(place.clone())));
                    Operand::Local(t)
                };
                let v = self.operand(value, out);
                let rv = Rvalue::Binary { op: *op, lhs: cur, rhs: v, ty: target.ty.clone(), span: s.span };
                self.push(out, StmtKind::Assign(place, rv));
            }
            TStmtKind::If { cond, capture, then, els } => {
                let mut t = Vec::new();
                let c = match capture {
                    None => self.operand(cond, out),
                    Some(v) => {
                        let p = self.place(cond, out);
                        let has = self.temp(Type::Bool);
                        let rv = Rvalue::IsSome { place: p.clone(), ty: cond.ty.clone() };
                        self.push(out, StmtKind::Assign(Place { local: has, proj: vec![] }, rv));
                        let val = Rvalue::Load(Self::unwrap_place(p, &cond.ty));
                        self.push(&mut t, StmtKind::Assign(Place { local: *v, proj: vec![] }, val));
                        Operand::Local(has)
                    }
                };
                self.block(then, false, &mut t);
                let mut e = Vec::new();
                self.block(els, false, &mut e);
                self.span = s.span;
                self.push(out, StmtKind::If { cond: c, then: t, els: e });
            }
            TStmtKind::While { cond, body } => {
                let mut b = Vec::new();
                let c = self.operand(cond, &mut b);
                self.push(&mut b, StmtKind::If { cond: c, then: vec![], els: vec![Stmt { kind: StmtKind::Break, span: s.span }] });
                self.block(body, true, &mut b);
                self.span = s.span;
                self.push(out, StmtKind::Loop { body: b, cont: vec![] });
            }
            TStmtKind::For { var, lo, hi, body } => {
                let Type::Int(it) = lo.ty else { unreachable!() };
                let lo_op = self.operand(lo, out);
                // The upper bound is evaluated once, before the loop.
                let hi_rv = self.rvalue(hi, out);
                let end = self.temp(hi.ty.clone());
                self.push(out, StmtKind::Assign(Place { local: end, proj: vec![] }, hi_rv));
                let i = Place { local: *var, proj: vec![] };
                self.push(out, StmtKind::Assign(i.clone(), Rvalue::Use(lo_op)));
                let mut b = Vec::new();
                let c = self.temp(Type::Bool);
                let cmp = Rvalue::Binary {
                    op: BinOp::Lt,
                    lhs: Operand::Local(*var),
                    rhs: Operand::Local(end),
                    ty: lo.ty.clone(),
                    span: s.span,
                };
                self.push(&mut b, StmtKind::Assign(Place { local: c, proj: vec![] }, cmp));
                self.push(&mut b, StmtKind::If { cond: Operand::Local(c), then: vec![], els: vec![Stmt { kind: StmtKind::Break, span: s.span }] });
                self.block(body, true, &mut b);
                self.span = s.span;
                // `i < end` held, so `i + 1` cannot overflow.
                let step = Rvalue::Binary {
                    op: BinOp::AddWrap,
                    lhs: Operand::Local(*var),
                    rhs: Operand::Int(1, it),
                    ty: lo.ty.clone(),
                    span: s.span,
                };
                let cont = vec![Stmt { kind: StmtKind::Assign(i, step), span: s.span }];
                self.push(out, StmtKind::Loop { body: b, cont });
            }
            TStmtKind::Break => {
                self.run_defers(true, out);
                self.span = s.span;
                self.push(out, StmtKind::Break);
            }
            TStmtKind::Continue => {
                self.run_defers(true, out);
                self.span = s.span;
                self.push(out, StmtKind::Continue);
            }
            TStmtKind::Return(value) => {
                let mut v = value.as_ref().map(|e| self.operand(e, out));
                if self.has_defers() {
                    // The return value is computed before deferred code runs.
                    if let (Some(Operand::Local(id)), Some(e)) = (&v, value) {
                        let t = self.temp(e.ty.clone());
                        self.push(out, StmtKind::Assign(Place { local: t, proj: vec![] }, Rvalue::Use(Operand::Local(*id))));
                        v = Some(Operand::Local(t));
                    }
                    self.run_defers(false, out);
                }
                self.span = s.span;
                self.push(out, StmtKind::Return(v));
            }
            TStmtKind::Defer(body) => {
                self.scopes.last_mut().unwrap().defers.push(body);
            }
            TStmtKind::Block(b) => self.block(b, false, out),
            TStmtKind::Expr(e) => self.eval(e, out),
        }
    }

    /// An expression evaluated for its side effects.
    fn eval(&mut self, e: &'a TExpr, out: &mut Block) {
        if let TExprKind::Match { .. } = e.kind {
            self.lower_match(e, None, out);
        } else {
            let rv = self.rvalue(e, out);
            self.push(out, StmtKind::Eval(rv));
        }
    }

    /// The value inside the optional at `p`.
    fn unwrap_place(mut p: Place, opt: &Type) -> Place {
        if !is_ptr_optional(opt) {
            p.proj.push(Proj::Field("val".to_string()));
        }
        p
    }

    /// `match`: bind the pattern variables and run the arm whose variant
    /// equals the tag. The last arm needs no test (the match is exhaustive).
    /// The value of an expression arm goes to `result`.
    fn lower_match(&mut self, e: &'a TExpr, result: Option<LocalId>, out: &mut Block) {
        let TExprKind::Match { scrutinee, arms } = &e.kind else { unreachable!() };
        let Type::Enum(ename) = &scrutinee.ty else { unreachable!() };
        let def = self.enums[ename.as_str()];
        let span = self.span;
        let place = self.place(scrutinee, out);
        let tag = self.temp(Type::Int(IntTy::U32));
        let mut tag_place = place.clone();
        tag_place.proj.push(Proj::Field("tag".to_string()));
        self.push(out, StmtKind::Assign(Place { local: tag, proj: vec![] }, Rvalue::Load(tag_place)));
        let mut bodies = Vec::new();
        for arm in arms {
            let mut b = Vec::new();
            if let Some(vi) = arm.variant {
                let v = &def.variants[vi];
                for &(var, fi) in &arm.bindings {
                    let mut p = place.clone();
                    for name in ["u", &v.name, &v.fields[fi].0] {
                        p.proj.push(Proj::Field(name.to_string()));
                    }
                    self.push(&mut b, StmtKind::Assign(Place { local: var, proj: vec![] }, Rvalue::Load(p)));
                }
            }
            match (&arm.body, result) {
                (TArmBody::Expr(x), Some(r)) => {
                    let rv = self.rvalue(x, &mut b);
                    self.push(&mut b, StmtKind::Assign(Place { local: r, proj: vec![] }, rv));
                }
                (TArmBody::Expr(x), None) => self.eval(x, &mut b),
                (TArmBody::Block(stmts), _) => self.block(stmts, false, &mut b),
            }
            self.span = span;
            bodies.push(b);
        }
        let mut chain: Option<Block> = None;
        for (arm, body) in arms.iter().zip(bodies).rev() {
            chain = Some(match (chain, arm.variant) {
                (Some(rest), Some(vi)) => {
                    let c = self.temp(Type::Bool);
                    let cmp = Rvalue::Binary {
                        op: BinOp::Eq,
                        lhs: Operand::Local(tag),
                        rhs: Operand::Int(vi as i128, IntTy::U32),
                        ty: Type::Int(IntTy::U32),
                        span,
                    };
                    vec![
                        Stmt { kind: StmtKind::Assign(Place { local: c, proj: vec![] }, cmp), span },
                        Stmt { kind: StmtKind::If { cond: Operand::Local(c), then: body, els: rest }, span },
                    ]
                }
                _ => body,
            });
        }
        out.extend(chain.unwrap_or_default());
    }

    fn operand(&mut self, e: &'a TExpr, out: &mut Block) -> Operand {
        match self.rvalue(e, out) {
            Rvalue::Use(op) => op,
            rv => {
                let t = self.temp(e.ty.clone());
                self.push(out, StmtKind::Assign(Place { local: t, proj: vec![] }, rv));
                Operand::Local(t)
            }
        }
    }

    fn place(&mut self, e: &'a TExpr, out: &mut Block) -> Place {
        match &e.kind {
            TExprKind::Var(id) => Place { local: *id, proj: vec![] },
            TExprKind::Deref(ptr) => {
                let mut p = self.place(ptr, out);
                p.proj.push(Proj::Deref);
                p
            }
            TExprKind::Field { base, index } => {
                let Type::Struct(sname) = &base.ty else { unreachable!() };
                let name = self.structs[sname.as_str()].fields[*index].0.clone();
                let mut p = self.place(base, out);
                p.proj.push(Proj::Field(name));
                p
            }
            TExprKind::SliceLen(base) | TExprKind::SlicePtr(base) => {
                let name = if matches!(e.kind, TExprKind::SliceLen(_)) { "len" } else { "ptr" };
                let mut p = self.place(base, out);
                p.proj.push(Proj::Field(name.to_string()));
                p
            }
            TExprKind::Index { base, index } => {
                let len = match &base.ty {
                    Type::Array(_, n) => Len::Array(*n),
                    _ => Len::Slice,
                };
                let mut p = self.place(base, out);
                let index = self.operand(index, out);
                p.proj.push(Proj::Index { index, len, span: e.span });
                p
            }
            _ => {
                let local = match self.operand(e, out) {
                    Operand::Local(t) => t,
                    // A constant in place position (e.g. `"abc"[0]`): spill it.
                    op => {
                        let t = self.temp(e.ty.clone());
                        self.push(out, StmtKind::Assign(Place { local: t, proj: vec![] }, Rvalue::Use(op)));
                        t
                    }
                };
                Place { local, proj: vec![] }
            }
        }
    }

    fn rvalue(&mut self, e: &'a TExpr, out: &mut Block) -> Rvalue {
        match &e.kind {
            TExprKind::Int(v) => match &e.ty {
                Type::Int(it) => Rvalue::Use(Operand::Int(*v, *it)),
                _ => unreachable!(),
            },
            TExprKind::Float(v) => match &e.ty {
                Type::Float(ft) => Rvalue::Use(Operand::Float(*v, *ft)),
                _ => unreachable!(),
            },
            TExprKind::Bool(b) => Rvalue::Use(Operand::Bool(*b)),
            TExprKind::Str(s) => Rvalue::Use(Operand::Str(s.clone())),
            TExprKind::Null => Rvalue::Null(e.ty.clone()),
            TExprKind::Some(x) => Rvalue::Some { val: self.operand(x, out), ty: e.ty.clone() },
            TExprKind::IsSome(x) => Rvalue::IsSome { place: self.place(x, out), ty: x.ty.clone() },
            TExprKind::AddrOf(x) => Rvalue::AddrOf(self.place(x, out)),
            TExprKind::Match { .. } => {
                let t = self.temp(e.ty.clone());
                self.lower_match(e, Some(t), out);
                Rvalue::Use(Operand::Local(t))
            }
            TExprKind::Var(id) => Rvalue::Use(Operand::Local(*id)),
            TExprKind::Call { name, is_extern, args } => {
                let args = args.iter().map(|a| self.operand(a, out)).collect();
                Rvalue::Call { name: name.clone(), is_extern: *is_extern, args }
            }
            TExprKind::Print { arg, newline } => {
                let arg = arg.as_ref().map(|a| (self.operand(a, out), a.ty.clone()));
                Rvalue::Print { arg, newline: *newline }
            }
            TExprKind::Binary { op: op @ (BinOp::And | BinOp::Or), lhs, rhs } => {
                let t = self.temp(Type::Bool);
                let tp = Place { local: t, proj: vec![] };
                let l = self.operand(lhs, out);
                self.push(out, StmtKind::Assign(tp.clone(), Rvalue::Use(l)));
                let mut rb = Vec::new();
                let r = self.operand(rhs, &mut rb);
                self.push(&mut rb, StmtKind::Assign(tp, Rvalue::Use(r)));
                let (then, els) = if *op == BinOp::And { (rb, vec![]) } else { (vec![], rb) };
                self.push(out, StmtKind::If { cond: Operand::Local(t), then, els });
                Rvalue::Use(Operand::Local(t))
            }
            TExprKind::Binary { op, lhs, rhs } => {
                let l = self.operand(lhs, out);
                let r = self.operand(rhs, out);
                Rvalue::Binary { op: *op, lhs: l, rhs: r, ty: lhs.ty.clone(), span: e.span }
            }
            TExprKind::Unary { op: UnOp::Neg, expr } => {
                let val = self.operand(expr, out);
                Rvalue::Neg { val, ty: e.ty.clone(), span: e.span }
            }
            TExprKind::Unary { op: UnOp::Not, expr } => Rvalue::Not(self.operand(expr, out)),
            TExprKind::Cast(inner) => {
                let val = self.operand(inner, out);
                Rvalue::Cast { val, from: inner.ty.clone(), to: e.ty.clone(), span: e.span }
            }
            TExprKind::Field { .. }
            | TExprKind::SliceLen(_)
            | TExprKind::SlicePtr(_)
            | TExprKind::Index { .. }
            | TExprKind::Deref(_) => {
                Rvalue::Load(self.place(e, out))
            }
            TExprKind::ArrayLen(base) => {
                if has_call(base) {
                    self.operand(base, out);
                }
                let Type::Array(_, n) = base.ty else { unreachable!() };
                Rvalue::Use(Operand::Int(n as i128, IntTy::Usize))
            }
            TExprKind::Slice { base, lo, hi } => {
                let (len, elem) = match &base.ty {
                    Type::Array(el, n) => (Len::Array(*n), (**el).clone()),
                    Type::Slice(el) => (Len::Slice, (**el).clone()),
                    _ => unreachable!(),
                };
                let base = self.place(base, out);
                let lo = match lo {
                    Some(x) => self.operand(x, out),
                    None => Operand::Int(0, IntTy::Usize),
                };
                let hi = hi.as_ref().map(|x| self.operand(x, out));
                Rvalue::Slice { base, len, elem, lo, hi, span: e.span }
            }
            TExprKind::StructLit(inits) => {
                let Type::Struct(name) = &e.ty else { unreachable!() };
                // Evaluate in source order, then arrange in declaration order.
                let mut ops: Vec<(usize, Operand)> = inits.iter().map(|(i, x)| (*i, self.operand(x, out))).collect();
                ops.sort_by_key(|(i, _)| *i);
                Rvalue::StructLit { name: name.clone(), fields: ops.into_iter().map(|(_, o)| o).collect() }
            }
            TExprKind::Variant { index, fields } => {
                let Type::Enum(name) = &e.ty else { unreachable!() };
                let mut ops: Vec<(usize, Operand)> = fields.iter().map(|(i, x)| (*i, self.operand(x, out))).collect();
                ops.sort_by_key(|(i, _)| *i);
                Rvalue::Variant {
                    enum_name: name.clone(),
                    tag: *index,
                    variant: self.enums[name.as_str()].variants[*index].name.clone(),
                    fields: ops.into_iter().map(|(_, o)| o).collect(),
                }
            }
            TExprKind::ArrayLit(elems) => {
                let elems = elems.iter().map(|x| self.operand(x, out)).collect();
                Rvalue::ArrayLit { ty: e.ty.clone(), elems }
            }
        }
    }
}
