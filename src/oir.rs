//! OIR: Ore's typed intermediate representation, between sema and the C
//! backend.
//!
//! Lowering from the typed AST:
//! - flattens expressions so every operand is a local or a constant, which
//!   fixes left-to-right evaluation order (C leaves argument order unspecified);
//! - lowers `&&`/`||` to `if`, `while`/`for` to `Loop`, and `x op= v` to `x = x op v`;
//! - expands `defer`: deferred statements are copied to every scope exit
//!   (end of block, `return`, `break`, `continue`) in LIFO order; `errdefer`
//!   statements only to exits that return an error;
//! - lowers `try` and `catch` to a test of the error union's `err` field, and
//!   records error return trace entries (`TracePush`, `TraceRestore`);
//! - makes runtime checks explicit: checked arithmetic carries its source
//!   span, and indexing carries the length to check against;
//! - lowers `match` to an `if` chain on the enum tag, the error value or
//!   the integer;
//! - reads globals in place: a global is a local slot that names it;
//! - lowers `alloc.create(v)` and `alloc.alloc(v, n)` to an allocation, a
//!   null test and the error union;
//! - indexes a C pointer `[*]T` without a check.

use crate::ast::{BinOp, UnOp};
use crate::diag::Span;
use crate::sema::{self, FloatTy, Intrinsic, IntTy, TArmBody, TExpr, TExprKind, TStmt, TStmtKind, Type};
use std::collections::HashMap;

pub type LocalId = usize;

#[derive(Debug, Clone)]
pub struct Program {
    pub structs: Vec<sema::StructDef>,
    pub enums: Vec<sema::EnumDef>,
    pub externs: Vec<sema::ExternFn>,
    pub funcs: Vec<Func>,
    /// Error names; an error's value is its index plus one.
    pub errors: Vec<String>,
    pub ifaces: Vec<sema::IfaceDef>,
    /// (interface, struct) pairs that need a vtable.
    pub vtables: Vec<(String, String)>,
    /// The structs used as `Allocator` (thunks and a vtable each).
    pub alloc_impls: Vec<String>,
    /// The tests; test `i` is the function `test#i` of its module.
    pub tests: Vec<sema::TestDef>,
    /// Globals and their values.
    pub globals: Vec<sema::GlobalDef>,
}

#[derive(Debug, Clone)]
pub struct Func {
    pub name: String,
    pub params: Vec<LocalId>,
    pub ret: Type,
    pub locals: Vec<Local>,
    pub body: Block,
    pub is_main: bool,
    /// Exported to C under its own name (`pub extern fn`).
    pub export: bool,
}

/// A function-wide local slot: a user variable (`name` set) or a temporary.
/// With `global`, the slot is the global `name` itself: it is not declared
/// in the function.
#[derive(Debug, Clone)]
pub struct Local {
    pub name: Option<String>,
    pub ty: Type,
    pub global: bool,
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
    /// Add the position of a `return` or `try` that passes an error on to
    /// the error return trace.
    TracePush(Span),
    /// Drop the trace entries added after the trace length in the local was
    /// saved (`Rvalue::TraceLen`): the error was handled by `catch`.
    TraceRestore(LocalId),
    /// Panic at `span` with `msg` unless `cond` holds; without `cond`,
    /// always (`unreachable`). In `fast` mode the check is left out, and
    /// reaching an unconditional one is undefined behavior.
    Check { cond: Option<Operand>, msg: &'static str, span: Span },
    /// A block of `@setSafety(on)`: its runtime checks are those of the
    /// build mode (`on`) or none.
    Safety { on: bool, body: Block },
}

#[derive(Debug, Clone)]
pub enum Operand {
    Local(LocalId),
    Int(i128, IntTy),
    Float(f64, FloatTy),
    Bool(bool),
    /// String literal, type `[]u8`.
    Str(Vec<u8>),
    /// A top-level function as a value; `is_extern` for a C name.
    Fn { name: String, is_extern: bool },
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

/// Is `t` an optional pointer, `?*T`, `?[*]T` or `?fn(...)`? It is a C
/// pointer that may be null; every other `?T` is a `{ has, val }` struct.
pub fn is_ptr_optional(t: &Type) -> bool {
    matches!(t, Type::Optional(inner) if matches!(**inner, Type::Ptr(..) | Type::CPtr(_) | Type::Fn(..)))
}

/// Length of the indexed/sliced sequence.
#[derive(Debug, Clone, Copy)]
pub enum Len {
    /// Fixed array `[N]T`.
    Array(u64),
    /// Slice `[]T`, length read at runtime.
    Slice,
    /// C pointer `[*]T`: no length to check against (slicing needs both
    /// bounds).
    Unchecked,
}

#[derive(Debug, Clone)]
pub enum Rvalue {
    Use(Operand),
    Load(Place),
    /// Arithmetic or comparison; `ty` is the operand type. Never `&&`/`||`.
    Binary { op: BinOp, lhs: Operand, rhs: Operand, ty: Type, span: Span },
    Neg { val: Operand, ty: Type, span: Span },
    Not(Operand),
    /// `~val`
    BitNot { val: Operand, ty: Type },
    /// Is the integer `val` in one of the inclusive ranges?
    InRanges { val: Operand, ranges: Vec<(i128, i128)>, ty: IntTy },
    Cast { val: Operand, from: Type, to: Type, span: Span },
    Call { name: String, is_extern: bool, args: Vec<Operand> },
    /// Call the function value `callee`.
    CallValue { callee: Operand, args: Vec<Operand> },
    /// The size of `ty` in bytes, a `usize`.
    SizeOf(Type),
    /// The all-zero value of `ty`.
    Zeroed(Type),
    /// The pointer `val` as the pointer type `ty`.
    PtrCast { val: Operand, ty: Type },
    /// Call method `method` through the vtable of the `dyn` value `recv`.
    DynCall { recv: Operand, method: String, args: Vec<Operand> },
    /// The pointer `ptr` to struct `strukt` as a `dyn iface`.
    ToDyn { ptr: Operand, iface: String, strukt: String },
    /// The pointer `ptr` to struct `strukt` as an `Allocator`.
    ToAllocator { ptr: Operand, strukt: String },
    /// Allocate a `ty` with the allocator `alloc`: a `?*mut ty`, null when
    /// out of memory.
    Alloc { alloc: Operand, ty: Type, span: Span },
    /// Allocate `len` elements of type `elem`, each set to `value`: a
    /// `?[]mut elem`, null when out of memory. No elements allocate nothing.
    /// With `old` (a slice), its elements are copied first and it is freed
    /// on success (`resize`).
    AllocSlice { alloc: Operand, value: Operand, elem: Type, len: Operand, old: Option<Operand>, span: Span },
    /// The empty slice of type `ty`.
    EmptySlice(Type),
    /// An allocator method or test builtin, with the operand types.
    Intrinsic { op: Intrinsic, args: Vec<(Operand, Type)>, span: Span },
    Print { arg: Option<(Operand, Type)>, newline: bool },
    /// `std.log.err`/`std.log.info`: the format's pieces around the
    /// arguments, written to stderr.
    Log { error: bool, pieces: Vec<Vec<u8>>, args: Vec<(Operand, Type)> },
    /// Fields in declaration order.
    StructLit { name: String, fields: Vec<Operand> },
    /// A value of the union `name` with its field `field` set.
    UnionLit { name: String, field: String, val: Operand },
    /// Enum value; payload fields in declaration order.
    Variant { enum_name: String, tag: usize, variant: String, fields: Vec<Operand> },
    AddrOf(Place),
    /// Wrap a value into the optional type `ty`.
    Some { val: Operand, ty: Type },
    Null(Type),
    /// Is the optional at `place` (of type `ty`) non-null?
    IsSome { place: Place, ty: Type },
    ArrayLit { ty: Type, elems: Vec<Operand> },
    /// The array `ty` with `val` in every element.
    ArrayRepeat { ty: Type, val: Operand },
    /// `base[lo..hi]`, producing a slice. `hi = None` means the full length.
    Slice { base: Place, len: Len, elem: Type, lo: Operand, hi: Option<Operand>, span: Span },
    /// Does the error union at `place` hold an error?
    IsErr(Place),
    /// The error union `ty` holding the value `val` (`None` for `!void`).
    Ok { val: Option<Operand>, ty: Type },
    /// The error union `ty` holding the error value `err`.
    Err { err: Operand, ty: Type },
    /// The current length of the error return trace (a `u32`).
    TraceLen,
}

/// Lower the program. `linked` are the libraries it is linked with: an
/// `if @hasLink(name)` keeps only the branch that runs. Only the functions
/// and vtables that the program can reach are kept (from `main`, the
/// tests, exported functions and globals), so a function that calls into
/// a library that is not linked is left out when nothing calls it.
pub fn lower(p: &sema::Program, linked: &[String]) -> Program {
    let structs: HashMap<&str, &sema::StructDef> = p.structs.iter().map(|s| (s.name.as_str(), s)).collect();
    let enums: HashMap<&str, &sema::EnumDef> = p.enums.iter().map(|e| (e.name.as_str(), e)).collect();
    let (funcs, refs): (Vec<Func>, Vec<Vec<Ref>>) = p.fns.iter().map(|f| Lower::func(f, &structs, &enums, linked)).unzip();
    let (funcs, vtables) = reachable(p, funcs, refs);
    Program {
        structs: p.structs.clone(),
        enums: p.enums.clone(),
        externs: p.externs.clone(),
        funcs,
        errors: p.errors.clone(),
        ifaces: p.ifaces.clone(),
        vtables,
        alloc_impls: p.alloc_impls.clone(),
        tests: p.tests.clone(),
        globals: p.globals.clone(),
    }
}

/// What a function's code refers to: a function (called or as a value),
/// or the vtable of a struct for an interface (a `dyn` made).
#[derive(Clone, PartialEq, Eq, Hash)]
enum Ref {
    Fn(String),
    Vtable(String, String),
}

/// The functions in `ConstVal::Fn` values of a global.
fn const_fns(v: &sema::ConstVal, out: &mut Vec<Ref>) {
    match v {
        sema::ConstVal::Fn(name, false) => out.push(Ref::Fn(name.clone())),
        sema::ConstVal::Some(x) | sema::ConstVal::Union(_, x) => const_fns(x, out),
        sema::ConstVal::Agg(xs) | sema::ConstVal::Variant(_, xs) => xs.iter().for_each(|x| const_fns(x, out)),
        _ => {}
    }
}

/// The functions and vtables reachable from `main`, the tests, the
/// exported functions and the globals; `refs[i]` are those of `funcs[i]`.
fn reachable(p: &sema::Program, funcs: Vec<Func>, refs: Vec<Vec<Ref>>) -> (Vec<Func>, Vec<(String, String)>) {
    let index: HashMap<&str, usize> = funcs.iter().enumerate().map(|(i, f)| (f.name.as_str(), i)).collect();
    let mut todo: Vec<Ref> = Vec::new();
    for f in &funcs {
        if f.is_main || f.export || sema::is_test(&f.name) {
            todo.push(Ref::Fn(f.name.clone()));
        }
    }
    for g in &p.globals {
        const_fns(&g.value, &mut todo);
    }
    let mut seen: std::collections::HashSet<Ref> = std::collections::HashSet::new();
    while let Some(r) = todo.pop() {
        if !seen.insert(r.clone()) {
            continue;
        }
        match &r {
            Ref::Fn(name) => {
                if let Some(&i) = index.get(name.as_str()) {
                    todo.extend(refs[i].iter().cloned());
                }
            }
            Ref::Vtable(iface, strukt) => {
                let methods = p.ifaces.iter().filter(|i| i.name == *iface).flat_map(|i| &i.methods);
                todo.extend(methods.map(|m| Ref::Fn(format!("{strukt}.{}", m.name))));
            }
        }
    }
    let vtables = p.vtables.iter().filter(|(i, s)| seen.contains(&Ref::Vtable(i.clone(), s.clone()))).cloned().collect();
    let funcs = funcs.into_iter().filter(|f| seen.contains(&Ref::Fn(f.name.clone()))).collect();
    (funcs, vtables)
}

/// The value of an `if` condition when it is known before the program runs:
/// `@hasLink(name)`, `true`, `false`, and `!`, `&&`, `||` of those.
fn static_bool(e: &TExpr, linked: &[String]) -> Option<bool> {
    match &e.kind {
        TExprKind::HasLink(name) => Some(linked.contains(name)),
        TExprKind::Bool(b) => Some(*b),
        TExprKind::Unary { op: UnOp::Not, expr } => static_bool(expr, linked).map(|b| !b),
        TExprKind::Binary { op: BinOp::And, lhs, rhs } => Some(static_bool(lhs, linked)? && static_bool(rhs, linked)?),
        TExprKind::Binary { op: BinOp::Or, lhs, rhs } => Some(static_bool(lhs, linked)? || static_bool(rhs, linked)?),
        _ => None,
    }
}

struct Scope<'a> {
    /// Deferred statements; `true` for `errdefer`.
    defers: Vec<(&'a [TStmt], bool)>,
    /// The body scope of a loop: `break`/`continue` exit up to here.
    is_loop: bool,
    /// The body of a `catch` block: the saved trace length to restore when
    /// the block is left without an error.
    restore: Option<LocalId>,
}

struct Lower<'a> {
    structs: &'a HashMap<&'a str, &'a sema::StructDef>,
    enums: &'a HashMap<&'a str, &'a sema::EnumDef>,
    locals: Vec<Local>,
    scopes: Vec<Scope<'a>>,
    span: Span,
    /// The function's return type.
    ret: Type,
    /// The local slot of each global the function uses.
    globals: HashMap<String, LocalId>,
    /// The libraries the program is linked with (`@hasLink`).
    linked: &'a [String],
    /// The functions and vtables the code refers to.
    refs: Vec<Ref>,
}

fn local(id: LocalId) -> Place {
    Place { local: id, proj: vec![] }
}

fn field(mut p: Place, name: &str) -> Place {
    p.proj.push(Proj::Field(name.to_string()));
    p
}

fn is_terminator(s: Option<&Stmt>) -> bool {
    match s.map(|s| &s.kind) {
        Some(StmtKind::Safety { body, .. }) => is_terminator(body.last()),
        k => matches!(k, Some(StmtKind::Break | StmtKind::Continue | StmtKind::Return(_))),
    }
}

fn has_call(e: &TExpr) -> bool {
    match &e.kind {
        TExprKind::Call { .. }
        | TExprKind::CallValue { .. }
        | TExprKind::Print { .. }
        | TExprKind::Log { .. }
        | TExprKind::Match { .. }
        | TExprKind::Try(_)
        | TExprKind::Catch { .. }
        | TExprKind::OrElse { .. }
        | TExprKind::Unwrap(_)
        | TExprKind::Unreachable
        | TExprKind::DynCall { .. }
        | TExprKind::Create { .. }
        | TExprKind::AllocSlice { .. }
        | TExprKind::Intrinsic { .. } => true,
        TExprKind::Int(_)
        | TExprKind::Float(_)
        | TExprKind::Bool(_)
        | TExprKind::Str(_)
        | TExprKind::Var(_)
        | TExprKind::Global(_)
        | TExprKind::Null
        | TExprKind::EmptySlice
        | TExprKind::ErrorValue(_)
        | TExprKind::FnRef { .. }
        | TExprKind::HasLink(_)
        | TExprKind::SizeOf(_)
        | TExprKind::Zeroed => false,
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
        | TExprKind::Deref(expr)
        | TExprKind::OkWrap(expr)
        | TExprKind::ErrWrap(expr)
        | TExprKind::PtrCast(expr)
        | TExprKind::IsVariant { expr, .. }
        | TExprKind::ToDyn { ptr: expr, .. }
        | TExprKind::ToAllocator { ptr: expr, .. } => has_call(expr),
        TExprKind::Slice { base, lo, hi } => {
            has_call(base) || lo.as_deref().is_some_and(has_call) || hi.as_deref().is_some_and(has_call)
        }
        TExprKind::StructLit(fs) | TExprKind::Variant { fields: fs, .. } => fs.iter().any(|(_, e)| has_call(e)),
        TExprKind::ArrayLit(es) => es.iter().any(has_call),
        TExprKind::ArrayRepeat(x) => has_call(x),
    }
}

impl<'a> Lower<'a> {
    fn func(
        f: &'a sema::FnDef,
        structs: &'a HashMap<&'a str, &'a sema::StructDef>,
        enums: &'a HashMap<&'a str, &'a sema::EnumDef>,
        linked: &'a [String],
    ) -> (Func, Vec<Ref>) {
        // Sema variables keep their ids as OIR locals; temporaries follow.
        let locals = f.vars.iter().map(|v| Local { name: Some(v.name.clone()), ty: v.ty.clone(), global: false }).collect();
        let mut l = Lower {
            structs,
            enums,
            locals,
            scopes: Vec::new(),
            span: Span::default(),
            ret: f.ret.clone(),
            globals: HashMap::new(),
            linked,
            refs: Vec::new(),
        };
        let mut body = Vec::new();
        l.block(&f.body, false, &mut body);
        // Falling off the end of a `!void` function returns "no error".
        if matches!(&f.ret, Type::ErrorUnion(_, p) if **p == Type::Void) && !is_terminator(body.last()) {
            let ok = l.ok_void(&mut body);
            l.push(&mut body, StmtKind::Return(Some(ok)));
        }
        let func = Func {
            name: f.name.clone(),
            params: f.params.clone(),
            ret: f.ret.clone(),
            locals: l.locals,
            body,
            is_main: f.is_main,
            export: f.export,
        };
        (func, l.refs)
    }

    fn temp(&mut self, ty: Type) -> LocalId {
        self.locals.push(Local { name: None, ty, global: false });
        self.locals.len() - 1
    }

    /// The local slot that names the global `name`.
    fn global(&mut self, name: &str, ty: &Type) -> LocalId {
        if let Some(&id) = self.globals.get(name) {
            return id;
        }
        self.locals.push(Local { name: Some(name.to_string()), ty: ty.clone(), global: true });
        let id = self.locals.len() - 1;
        self.globals.insert(name.to_string(), id);
        id
    }

    fn push(&self, out: &mut Block, kind: StmtKind) {
        out.push(Stmt { kind, span: self.span });
    }

    fn block(&mut self, stmts: &'a [TStmt], is_loop: bool, out: &mut Block) {
        self.scoped(stmts, Scope { defers: Vec::new(), is_loop, restore: None }, out);
    }

    fn scoped(&mut self, stmts: &'a [TStmt], scope: Scope<'a>, out: &mut Block) {
        if let Some(TStmt { kind: TStmtKind::SetSafety(on), span }) = stmts.first() {
            let mut body = Vec::new();
            self.scoped(&stmts[1..], scope, &mut body);
            out.push(Stmt { kind: StmtKind::Safety { on: *on, body }, span: *span });
            return;
        }
        self.scopes.push(scope);
        for s in stmts {
            self.stmt(s, out);
        }
        let scope = self.scopes.pop().unwrap();
        if !is_terminator(out.last()) {
            for &(d, on_err) in scope.defers.iter().rev() {
                if !on_err {
                    self.block(d, false, out);
                }
            }
            if let Some(t) = scope.restore {
                self.push(out, StmtKind::TraceRestore(t));
            }
        }
    }

    /// Emit the exit code of the innermost scopes, innermost first, stopping
    /// after the loop body scope when `to_loop` is set: deferred statements
    /// (`errdefer` too when `on_err`), and, unless an error leaves, the trace
    /// restore of `catch` blocks.
    fn run_defers(&mut self, to_loop: bool, on_err: bool, out: &mut Block) {
        let mut pending: Vec<Result<&'a [TStmt], LocalId>> = Vec::new();
        for scope in self.scopes.iter().rev() {
            pending.extend(scope.defers.iter().rev().filter(|(_, e)| on_err || !e).map(|(d, _)| Ok(*d)));
            if let (Some(t), false) = (scope.restore, on_err) {
                pending.push(Err(t));
            }
            if to_loop && scope.is_loop {
                break;
            }
        }
        let span = self.span;
        for p in pending {
            match p {
                Ok(d) => self.block(d, false, out),
                Err(t) => out.push(Stmt { kind: StmtKind::TraceRestore(t), span }),
            }
        }
        self.span = span;
    }

    /// A local set to "no error" of the function's `E!void` return type.
    fn ok_void(&mut self, out: &mut Block) -> Operand {
        let t = self.temp(self.ret.clone());
        let rv = Rvalue::Ok { val: None, ty: self.ret.clone() };
        self.push(out, StmtKind::Assign(local(t), rv));
        Operand::Local(t)
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
            // A condition known now: only the branch that runs.
            TStmtKind::If { cond, capture: None, then, els } if let Some(b) = static_bool(cond, self.linked) => {
                let mut t = Vec::new();
                self.block(if b { then } else { els }, false, &mut t);
                self.span = s.span;
                self.push(out, StmtKind::If { cond: Operand::Bool(true), then: t, els: Vec::new() });
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
            // On an error, `err` gets it and the error branch runs; the
            // trace entries of the error are dropped when that branch ends
            // normally, as for `catch`.
            TStmtKind::IfErr { cond, value, then, err, els } => {
                let saved = self.temp(Type::Int(IntTy::U32));
                self.push(out, StmtKind::Assign(local(saved), Rvalue::TraceLen));
                let u = self.place(cond, out);
                let c = self.temp(Type::Bool);
                self.push(out, StmtKind::Assign(local(c), Rvalue::IsErr(u.clone())));
                let mut ok = Vec::new();
                if let Some(v) = value {
                    self.push(&mut ok, StmtKind::Assign(local(*v), Rvalue::Load(field(u.clone(), "val"))));
                }
                self.block(then, false, &mut ok);
                self.span = s.span;
                let mut bad = Vec::new();
                self.push(&mut bad, StmtKind::Assign(local(*err), Rvalue::Load(field(u, "err"))));
                self.scoped(els, Scope { defers: Vec::new(), is_loop: false, restore: Some(saved) }, &mut bad);
                self.span = s.span;
                self.push(out, StmtKind::If { cond: Operand::Local(c), then: bad, els: ok });
            }
            TStmtKind::While { cond, capture, body } => {
                let mut b = Vec::new();
                let (c, val) = match capture {
                    None => (self.operand(cond, &mut b), None),
                    // `while x |v|`: test `x` and unwrap it into `v` on each iteration.
                    Some(v) => {
                        let p = self.place(cond, &mut b);
                        let has = self.temp(Type::Bool);
                        let rv = Rvalue::IsSome { place: p.clone(), ty: cond.ty.clone() };
                        self.push(&mut b, StmtKind::Assign(local(has), rv));
                        (Operand::Local(has), Some((*v, Self::unwrap_place(p, &cond.ty))))
                    }
                };
                self.push(&mut b, StmtKind::If { cond: c, then: vec![], els: vec![Stmt { kind: StmtKind::Break, span: s.span }] });
                if let Some((v, p)) = val {
                    self.push(&mut b, StmtKind::Assign(local(v), Rvalue::Load(p)));
                }
                self.block(body, true, &mut b);
                self.span = s.span;
                self.push(out, StmtKind::Loop { body: b, cont: vec![] });
            }
            TStmtKind::ForEach { index, elem, seq, body } => {
                let usize_ty = Type::Int(IntTy::Usize);
                // The sequence is evaluated once: a slice is copied, and an
                // array is read in place.
                let (base, len) = match &seq.ty {
                    Type::Array(_, n) => (self.place(seq, out), Len::Array(*n)),
                    _ => {
                        let rv = self.rvalue(seq, out);
                        let t = self.temp(seq.ty.clone());
                        self.push(out, StmtKind::Assign(local(t), rv));
                        (local(t), Len::Slice)
                    }
                };
                let end = self.temp(usize_ty.clone());
                let n = match len {
                    Len::Array(n) => Rvalue::Use(Operand::Int(n as i128, IntTy::Usize)),
                    _ => Rvalue::Load(field(base.clone(), "len")),
                };
                self.push(out, StmtKind::Assign(local(end), n));
                let i = index.unwrap_or_else(|| self.temp(usize_ty.clone()));
                self.push(out, StmtKind::Assign(local(i), Rvalue::Use(Operand::Int(0, IntTy::Usize))));
                let mut b = Vec::new();
                let c = self.temp(Type::Bool);
                let cmp = Rvalue::Binary {
                    op: BinOp::Lt,
                    lhs: Operand::Local(i),
                    rhs: Operand::Local(end),
                    ty: usize_ty.clone(),
                    span: s.span,
                };
                self.push(&mut b, StmtKind::Assign(local(c), cmp));
                self.push(&mut b, StmtKind::If { cond: Operand::Local(c), then: vec![], els: vec![Stmt { kind: StmtKind::Break, span: s.span }] });
                let mut p = base;
                p.proj.push(Proj::Index { index: Operand::Local(i), len, span: s.span });
                self.push(&mut b, StmtKind::Assign(local(*elem), Rvalue::Load(p)));
                self.block(body, true, &mut b);
                self.span = s.span;
                // `i < end` held, so `i + 1` cannot overflow.
                let step = Rvalue::Binary {
                    op: BinOp::AddWrap,
                    lhs: Operand::Local(i),
                    rhs: Operand::Int(1, IntTy::Usize),
                    ty: usize_ty,
                    span: s.span,
                };
                let cont = vec![Stmt { kind: StmtKind::Assign(local(i), step), span: s.span }];
                self.push(out, StmtKind::Loop { body: b, cont });
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
                self.run_defers(true, false, out);
                self.push(out, StmtKind::Break);
            }
            TStmtKind::Continue => {
                self.run_defers(true, false, out);
                self.push(out, StmtKind::Continue);
            }
            TStmtKind::Return(value) => {
                let mut v = match value {
                    Some(e) => Some(self.operand(e, out)),
                    None if matches!(self.ret, Type::ErrorUnion(..)) => Some(self.ok_void(out)),
                    None => None,
                };
                // The return value is computed before deferred code runs.
                if self.has_defers()
                    && let (Some(Operand::Local(id)), Some(e)) = (&v, value)
                {
                    let t = self.temp(e.ty.clone());
                    self.push(out, StmtKind::Assign(local(t), Rvalue::Use(Operand::Local(*id))));
                    v = Some(Operand::Local(t));
                }
                match value {
                    // An error: the trace gets this `return`, and `errdefer`s run.
                    Some(TExpr { kind: TExprKind::ErrWrap(_), .. }) => {
                        self.push(out, StmtKind::TracePush(s.span));
                        self.run_defers(false, true, out);
                    }
                    // An error union from a call: test it.
                    Some(TExpr {
                        kind:
                            TExprKind::Call { .. }
                            | TExprKind::DynCall { .. }
                            | TExprKind::Create { .. }
                            | TExprKind::AllocSlice { .. }
                            | TExprKind::Match { .. },
                        ty: Type::ErrorUnion(..),
                        ..
                    }) => {
                        let Some(Operand::Local(u)) = v else { unreachable!() };
                        let c = self.temp(Type::Bool);
                        self.push(out, StmtKind::Assign(local(c), Rvalue::IsErr(local(u))));
                        let mut then = Vec::new();
                        self.push(&mut then, StmtKind::TracePush(s.span));
                        self.run_defers(false, true, &mut then);
                        self.push(&mut then, StmtKind::Return(v.clone()));
                        self.push(out, StmtKind::If { cond: Operand::Local(c), then, els: vec![] });
                        self.run_defers(false, false, out);
                    }
                    _ => self.run_defers(false, false, out),
                }
                self.push(out, StmtKind::Return(v));
            }
            TStmtKind::Defer(body) => {
                self.scopes.last_mut().unwrap().defers.push((body, false));
            }
            TStmtKind::ErrDefer(body) => {
                self.scopes.last_mut().unwrap().defers.push((body, true));
            }
            TStmtKind::Block(b) => self.block(b, false, out),
            TStmtKind::Expr(e) => self.eval(e, out),
            // Handled by `scoped`, as the first statement of a block.
            TStmtKind::SetSafety(_) => {}
        }
    }

    /// An expression evaluated for its side effects.
    fn eval(&mut self, e: &'a TExpr, out: &mut Block) {
        match e.kind {
            TExprKind::Match { .. } => self.lower_match(e, None, out),
            TExprKind::Catch { .. } => self.lower_catch(e, None, out),
            TExprKind::OrElse { .. } => self.lower_orelse(e, None, out),
            TExprKind::Unreachable => self.push(out, StmtKind::Check { cond: None, msg: "reached unreachable code", span: e.span }),
            TExprKind::Try(_) => {
                self.lower_try(e, out);
            }
            _ => {
                let rv = self.rvalue(e, out);
                self.push(out, StmtKind::Eval(rv));
            }
        }
    }

    /// `try x`: if the error union `x` holds an error, add this `try` to the
    /// trace, run the exit code with `errdefer`s and return the error.
    /// Returns the place of the payload (`None` for `void`).
    fn lower_try(&mut self, e: &'a TExpr, out: &mut Block) -> Option<Place> {
        let TExprKind::Try(inner) = &e.kind else { unreachable!() };
        let span = self.span;
        let u = self.place(inner, out);
        let c = self.temp(Type::Bool);
        self.push(out, StmtKind::Assign(local(c), Rvalue::IsErr(u.clone())));
        let mut then = Vec::new();
        let err = self.temp(Type::Int(IntTy::U16));
        self.push(&mut then, StmtKind::Assign(local(err), Rvalue::Load(field(u.clone(), "err"))));
        self.push(&mut then, StmtKind::TracePush(e.span));
        self.run_defers(false, true, &mut then);
        let r = self.temp(self.ret.clone());
        let rv = Rvalue::Err { err: Operand::Local(err), ty: self.ret.clone() };
        self.push(&mut then, StmtKind::Assign(local(r), rv));
        self.push(&mut then, StmtKind::Return(Some(Operand::Local(r))));
        self.span = span;
        self.push(out, StmtKind::If { cond: Operand::Local(c), then, els: vec![] });
        (e.ty != Type::Void).then(|| field(u, "val"))
    }

    /// `x catch |e| body`: on an error, bind `e` and run the body (its value
    /// goes to `result`); otherwise `result` gets the payload. The trace
    /// length is saved first and restored when the body ends normally.
    fn lower_catch(&mut self, e: &'a TExpr, result: Option<LocalId>, out: &mut Block) {
        let TExprKind::Catch { expr, capture, body } = &e.kind else { unreachable!() };
        let span = self.span;
        let saved = self.temp(Type::Int(IntTy::U32));
        self.push(out, StmtKind::Assign(local(saved), Rvalue::TraceLen));
        let u = self.place(expr, out);
        let c = self.temp(Type::Bool);
        self.push(out, StmtKind::Assign(local(c), Rvalue::IsErr(u.clone())));
        let mut then = Vec::new();
        if let Some(v) = capture {
            self.push(&mut then, StmtKind::Assign(local(*v), Rvalue::Load(field(u.clone(), "err"))));
        }
        match (&**body, result) {
            (TArmBody::Expr(x), Some(r)) => {
                let rv = self.rvalue(x, &mut then);
                self.push(&mut then, StmtKind::Assign(local(r), rv));
                self.push(&mut then, StmtKind::TraceRestore(saved));
            }
            (TArmBody::Expr(x), None) => {
                self.eval(x, &mut then);
                self.push(&mut then, StmtKind::TraceRestore(saved));
            }
            (TArmBody::Block(stmts), _) => {
                self.scoped(stmts, Scope { defers: Vec::new(), is_loop: false, restore: Some(saved) }, &mut then);
            }
        }
        self.span = span;
        let els = match result {
            Some(r) => vec![Stmt { kind: StmtKind::Assign(local(r), Rvalue::Load(field(u, "val"))), span }],
            None => vec![],
        };
        self.push(out, StmtKind::If { cond: Operand::Local(c), then, els });
    }

    /// `x orelse body`: the value of the optional `x` goes to `result`, or
    /// when it is `null` the body runs (its value goes to `result`).
    fn lower_orelse(&mut self, e: &'a TExpr, result: Option<LocalId>, out: &mut Block) {
        let TExprKind::OrElse { expr, body } = &e.kind else { unreachable!() };
        let span = self.span;
        let p = self.place(expr, out);
        let has = self.temp(Type::Bool);
        self.push(out, StmtKind::Assign(local(has), Rvalue::IsSome { place: p.clone(), ty: expr.ty.clone() }));
        let mut then = Vec::new();
        // A `?T` result (`a orelse b` where `b` is a `?T`) takes `x` whole.
        if let Some(r) = result {
            let p = if e.ty == expr.ty { p } else { Self::unwrap_place(p, &expr.ty) };
            self.push(&mut then, StmtKind::Assign(local(r), Rvalue::Load(p)));
        }
        let mut els = Vec::new();
        match (&**body, result) {
            (TArmBody::Expr(x), Some(r)) => {
                let rv = self.rvalue(x, &mut els);
                self.push(&mut els, StmtKind::Assign(local(r), rv));
            }
            (TArmBody::Expr(x), None) => self.eval(x, &mut els),
            (TArmBody::Block(stmts), _) => self.block(stmts, false, &mut els),
        }
        self.span = span;
        self.push(out, StmtKind::If { cond: Operand::Local(has), then, els });
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
    /// A `match` on errors tests the error value instead of the tag, and a
    /// `match` on an integer tests the arms' ranges.
    fn lower_match(&mut self, e: &'a TExpr, result: Option<LocalId>, out: &mut Block) {
        let TExprKind::Match { scrutinee, arms } = &e.kind else { unreachable!() };
        let def = match &scrutinee.ty {
            Type::Enum(ename) => Some(self.enums[ename.as_str()]),
            _ => None,
        };
        let span = self.span;
        let place = self.place(scrutinee, out);
        let tag_ty = match &scrutinee.ty {
            Type::Enum(_) => IntTy::U32,
            Type::Int(it) => *it,
            _ => IntTy::U16,
        };
        let tag = self.temp(Type::Int(tag_ty));
        let tag_place = if def.is_some() { field(place.clone(), "tag") } else { place.clone() };
        self.push(out, StmtKind::Assign(local(tag), Rvalue::Load(tag_place)));
        let mut bodies = Vec::new();
        for arm in arms {
            let mut b = Vec::new();
            if let ([vi], Some(def)) = (&arm.variants[..], def) {
                let vi = *vi;
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
            chain = Some(match chain {
                Some(rest) if !arm.variants.is_empty() || !arm.ranges.is_empty() => {
                    let c = self.temp(Type::Bool);
                    let cmp = match &arm.variants[..] {
                        [vi] => Rvalue::Binary {
                            op: BinOp::Eq,
                            lhs: Operand::Local(tag),
                            rhs: Operand::Int(*vi as i128, tag_ty),
                            ty: Type::Int(tag_ty),
                            span,
                        },
                        [] => Rvalue::InRanges { val: Operand::Local(tag), ranges: arm.ranges.clone(), ty: tag_ty },
                        vs => {
                            let ranges = vs.iter().map(|&v| (v as i128, v as i128)).collect();
                            Rvalue::InRanges { val: Operand::Local(tag), ranges, ty: tag_ty }
                        }
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

    /// The place of the operand `op` of type `ty`: a constant goes to a
    /// temporary.
    fn spill(&mut self, op: Operand, ty: &Type, out: &mut Block) -> Place {
        match op {
            Operand::Local(id) => local(id),
            op => {
                let t = self.temp(ty.clone());
                self.push(out, StmtKind::Assign(local(t), Rvalue::Use(op)));
                local(t)
            }
        }
    }

    /// The value at `p` as an operand.
    fn load(&mut self, p: Place, ty: &Type, out: &mut Block) -> Operand {
        if p.proj.is_empty() {
            return Operand::Local(p.local);
        }
        let t = self.temp(ty.clone());
        self.push(out, StmtKind::Assign(local(t), Rvalue::Load(p)));
        Operand::Local(t)
    }

    /// `a == b` for the values of type `ty` at `a` and `b`, a `bool`: a
    /// struct field by field (stopping at the first difference), an
    /// optional by `has` and then its value, the rest with C's `==`.
    fn equal(&mut self, a: Place, b: Place, ty: &Type, out: &mut Block) -> Operand {
        let span = self.span;
        let r = self.temp(Type::Bool);
        match ty {
            Type::Struct(name) => {
                let fields = self.structs[name.as_str()].fields.clone();
                self.push(out, StmtKind::Assign(local(r), Rvalue::Use(Operand::Bool(true))));
                // Each field's test runs only when the fields before it are equal.
                let mut tail: Block = Vec::new();
                for (f, ft) in fields.iter().rev() {
                    let mut b_ = Vec::new();
                    let c = self.equal(field(a.clone(), f), field(b.clone(), f), ft, &mut b_);
                    self.push(&mut b_, StmtKind::Assign(local(r), Rvalue::Use(c)));
                    if !tail.is_empty() {
                        let rest = std::mem::take(&mut tail);
                        b_.push(Stmt { kind: StmtKind::If { cond: Operand::Local(r), then: rest, els: vec![] }, span });
                    }
                    tail = b_;
                }
                out.extend(tail);
            }
            Type::Optional(inner) if !is_ptr_optional(ty) => {
                let ha = self.temp(Type::Bool);
                self.push(out, StmtKind::Assign(local(ha), Rvalue::IsSome { place: a.clone(), ty: ty.clone() }));
                let hb = self.temp(Type::Bool);
                self.push(out, StmtKind::Assign(local(hb), Rvalue::IsSome { place: b.clone(), ty: ty.clone() }));
                let same = Rvalue::Binary { op: BinOp::Eq, lhs: Operand::Local(ha), rhs: Operand::Local(hb), ty: Type::Bool, span };
                self.push(out, StmtKind::Assign(local(r), same));
                let mut vals = Vec::new();
                let c = self.equal(field(a, "val"), field(b, "val"), inner, &mut vals);
                self.push(&mut vals, StmtKind::Assign(local(r), Rvalue::Use(c)));
                let both = vec![Stmt { kind: StmtKind::If { cond: Operand::Local(ha), then: vals, els: vec![] }, span }];
                self.push(out, StmtKind::If { cond: Operand::Local(r), then: both, els: vec![] });
            }
            _ => {
                let (x, y) = (self.load(a, ty, out), self.load(b, ty, out));
                let rv = Rvalue::Binary { op: BinOp::Eq, lhs: x, rhs: y, ty: ty.clone(), span };
                self.push(out, StmtKind::Assign(local(r), rv));
            }
        }
        Operand::Local(r)
    }

    /// Mix the value of type `ty` at `p` into the `u64` hash in the local
    /// `h`, part by part as `equal` compares it: a struct field by field,
    /// an optional by `has` and then its value.
    fn hash(&mut self, p: Place, ty: &Type, h: LocalId, out: &mut Block) {
        let u64t = Type::Int(IntTy::U64);
        let mix = |this: &mut Self, val: Operand, vty: Type, out: &mut Block| {
            let args = vec![(Operand::Local(h), u64t.clone()), (val, vty)];
            let rv = Rvalue::Intrinsic { op: Intrinsic::HashMix, args, span: this.span };
            this.push(out, StmtKind::Assign(local(h), rv));
        };
        match ty {
            Type::Struct(name) => {
                for (f, ft) in self.structs[name.as_str()].fields.clone() {
                    self.hash(field(p.clone(), &f), &ft, h, out);
                }
            }
            Type::Optional(inner) if !is_ptr_optional(ty) => {
                let has = self.temp(Type::Bool);
                self.push(out, StmtKind::Assign(local(has), Rvalue::IsSome { place: p.clone(), ty: ty.clone() }));
                mix(self, Operand::Local(has), Type::Bool, out);
                let mut then = Vec::new();
                self.hash(field(p, "val"), inner, h, &mut then);
                self.push(out, StmtKind::If { cond: Operand::Local(has), then, els: vec![] });
            }
            _ => {
                let v = self.load(p, ty, out);
                mix(self, v, ty.clone(), out);
            }
        }
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
            TExprKind::Global(name) => local(self.global(name, &e.ty)),
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
            // `x.?`: panic unless `x` holds a value.
            TExprKind::Unwrap(x) => {
                let p = self.place(x, out);
                let has = self.temp(Type::Bool);
                self.push(out, StmtKind::Assign(local(has), Rvalue::IsSome { place: p.clone(), ty: x.ty.clone() }));
                self.push(out, StmtKind::Check { cond: Some(Operand::Local(has)), msg: "unwrap of null", span: e.span });
                Self::unwrap_place(p, &x.ty)
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
                    Type::CPtr(_) => Len::Unchecked,
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
            // A copy: a call later in the expression may change a `var` global.
            TExprKind::Global(_) => Rvalue::Load(self.place(e, out)),
            TExprKind::ErrorValue(v) => Rvalue::Use(Operand::Int(*v as i128, IntTy::U16)),
            TExprKind::OkWrap(x) => Rvalue::Ok { val: Some(self.operand(x, out)), ty: e.ty.clone() },
            TExprKind::ErrWrap(x) => Rvalue::Err { err: self.operand(x, out), ty: e.ty.clone() },
            TExprKind::Try(_) => Rvalue::Load(self.lower_try(e, out).unwrap()),
            TExprKind::Catch { .. } => {
                let t = self.temp(e.ty.clone());
                self.lower_catch(e, Some(t), out);
                Rvalue::Use(Operand::Local(t))
            }
            TExprKind::OrElse { .. } => {
                let t = self.temp(e.ty.clone());
                self.lower_orelse(e, Some(t), out);
                Rvalue::Use(Operand::Local(t))
            }
            // The panic does not return: the value is never read.
            TExprKind::Unreachable => {
                self.push(out, StmtKind::Check { cond: None, msg: "reached unreachable code", span: e.span });
                Rvalue::Use(Operand::Local(self.temp(e.ty.clone())))
            }
            TExprKind::Call { name, is_extern, args } => {
                let args = args.iter().map(|a| self.operand(a, out)).collect();
                self.refs.push(Ref::Fn(name.clone()));
                Rvalue::Call { name: name.clone(), is_extern: *is_extern, args }
            }
            TExprKind::FnRef { name, is_extern } => {
                self.refs.push(Ref::Fn(name.clone()));
                Rvalue::Use(Operand::Fn { name: name.clone(), is_extern: *is_extern })
            }
            TExprKind::HasLink(name) => Rvalue::Use(Operand::Bool(self.linked.contains(name))),
            TExprKind::CallValue { callee, args } => {
                let callee = self.operand(callee, out);
                let args = args.iter().map(|a| self.operand(a, out)).collect();
                Rvalue::CallValue { callee, args }
            }
            TExprKind::SizeOf(t) => Rvalue::SizeOf(t.clone()),
            TExprKind::Zeroed => Rvalue::Zeroed(e.ty.clone()),
            TExprKind::PtrCast(x) => Rvalue::PtrCast { val: self.operand(x, out), ty: e.ty.clone() },
            TExprKind::DynCall { recv, method, args } => {
                let recv = self.operand(recv, out);
                let args = args.iter().map(|a| self.operand(a, out)).collect();
                Rvalue::DynCall { recv, method: method.clone(), args }
            }
            TExprKind::ToDyn { ptr, strukt } => {
                let Type::Dyn(iface) = &e.ty else { unreachable!() };
                self.refs.push(Ref::Vtable(iface.clone(), strukt.clone()));
                Rvalue::ToDyn { ptr: self.operand(ptr, out), iface: iface.clone(), strukt: strukt.clone() }
            }
            TExprKind::ToAllocator { ptr, strukt } => {
                self.refs.push(Ref::Fn(format!("{strukt}.alloc")));
                self.refs.push(Ref::Fn(format!("{strukt}.free")));
                Rvalue::ToAllocator { ptr: self.operand(ptr, out), strukt: strukt.clone() }
            }
            TExprKind::Intrinsic { op: Intrinsic::Hash, args } => {
                let v = self.operand(&args[0], out);
                let p = self.spill(v, &args[0].ty, out);
                let h = self.temp(Type::Int(IntTy::U64));
                self.push(out, StmtKind::Assign(local(h), Rvalue::Use(Operand::Int(0x9e37_79b9_7f4a_7c15, IntTy::U64))));
                self.hash(p, &args[0].ty, h, out);
                Rvalue::Use(Operand::Local(h))
            }
            TExprKind::Intrinsic { op, args } => {
                let args = args.iter().map(|a| (self.operand(a, out), a.ty.clone())).collect();
                Rvalue::Intrinsic { op: *op, args, span: e.span }
            }
            // `p = alloc(T)`; if `p` is not null, `p.* = value` and the
            // result is `p`, else the error `OutOfMemory`.
            TExprKind::Create { alloc, value } => {
                let a = self.operand(alloc, out);
                let v = self.operand(value, out);
                let opt = Type::Optional(Box::new(Type::Ptr(Box::new(value.ty.clone()), true)));
                let p = self.temp(opt.clone());
                self.push(out, StmtKind::Assign(local(p), Rvalue::Alloc { alloc: a, ty: value.ty.clone(), span: e.span }));
                let has = self.temp(Type::Bool);
                self.push(out, StmtKind::Assign(local(has), Rvalue::IsSome { place: local(p), ty: opt }));
                let u = self.temp(e.ty.clone());
                let mut then = Vec::new();
                self.push(&mut then, StmtKind::Assign(Place { local: p, proj: vec![Proj::Deref] }, Rvalue::Use(v)));
                self.push(&mut then, StmtKind::Assign(local(u), Rvalue::Ok { val: Some(Operand::Local(p)), ty: e.ty.clone() }));
                let oom = Operand::Int(sema::OUT_OF_MEMORY as i128, IntTy::U16);
                let els = vec![Stmt { kind: StmtKind::Assign(local(u), Rvalue::Err { err: oom, ty: e.ty.clone() }), span: self.span }];
                self.push(out, StmtKind::If { cond: Operand::Local(has), then, els });
                Rvalue::Use(Operand::Local(u))
            }
            // `r = alloc(n elements)`: if `r` is not null, the result is
            // its slice, else the error `OutOfMemory`.
            // `resize` evaluates its arguments (old, len, value) in order.
            TExprKind::AllocSlice { alloc, value, len, old } => {
                let a = self.operand(alloc, out);
                let o = old.as_ref().map(|o| self.operand(o, out));
                let (v, n) = if o.is_some() {
                    let n = self.operand(len, out);
                    (self.operand(value, out), n)
                } else {
                    let v = self.operand(value, out);
                    (v, self.operand(len, out))
                };
                let Type::ErrorUnion(_, slice) = &e.ty else { unreachable!() };
                let opt = Type::Optional(slice.clone());
                let r = self.temp(opt.clone());
                let rv = Rvalue::AllocSlice { alloc: a, value: v, elem: value.ty.clone(), len: n, old: o, span: e.span };
                self.push(out, StmtKind::Assign(local(r), rv));
                let has = self.temp(Type::Bool);
                self.push(out, StmtKind::Assign(local(has), Rvalue::IsSome { place: local(r), ty: opt }));
                let u = self.temp(e.ty.clone());
                let s = self.temp((**slice).clone());
                let mut then = Vec::new();
                self.push(&mut then, StmtKind::Assign(local(s), Rvalue::Load(field(local(r), "val"))));
                self.push(&mut then, StmtKind::Assign(local(u), Rvalue::Ok { val: Some(Operand::Local(s)), ty: e.ty.clone() }));
                let oom = Operand::Int(sema::OUT_OF_MEMORY as i128, IntTy::U16);
                let els = vec![Stmt { kind: StmtKind::Assign(local(u), Rvalue::Err { err: oom, ty: e.ty.clone() }), span: self.span }];
                self.push(out, StmtKind::If { cond: Operand::Local(has), then, els });
                Rvalue::Use(Operand::Local(u))
            }
            TExprKind::EmptySlice => Rvalue::EmptySlice(e.ty.clone()),
            TExprKind::Print { arg, newline } => {
                let arg = arg.as_ref().map(|a| (self.operand(a, out), a.ty.clone()));
                Rvalue::Print { arg, newline: *newline }
            }
            TExprKind::Log { error, pieces, args } => {
                let args = args.iter().map(|a| (self.operand(a, out), a.ty.clone())).collect();
                Rvalue::Log { error: *error, pieces: pieces.clone(), args }
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
            // Structs and optionals (other than pointers) compare part by part.
            TExprKind::Binary { op: op @ (BinOp::Eq | BinOp::Ne), lhs, rhs }
                if matches!(lhs.ty, Type::Struct(_)) || (matches!(lhs.ty, Type::Optional(_)) && !is_ptr_optional(&lhs.ty)) =>
            {
                let l = self.operand(lhs, out);
                let r = self.operand(rhs, out);
                let (a, b) = (self.spill(l, &lhs.ty, out), self.spill(r, &lhs.ty, out));
                let eq = self.equal(a, b, &lhs.ty, out);
                if *op == BinOp::Eq { Rvalue::Use(eq) } else { Rvalue::Not(eq) }
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
            TExprKind::Unary { op: UnOp::BitNot, expr } => Rvalue::BitNot { val: self.operand(expr, out), ty: e.ty.clone() },
            TExprKind::Cast(inner) => {
                let val = self.operand(inner, out);
                Rvalue::Cast { val, from: inner.ty.clone(), to: e.ty.clone(), span: e.span }
            }
            TExprKind::Field { .. }
            | TExprKind::SliceLen(_)
            | TExprKind::SlicePtr(_)
            | TExprKind::Index { .. }
            | TExprKind::Unwrap(_)
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
                    Type::Slice(el, _) => (Len::Slice, (**el).clone()),
                    Type::CPtr(el) => (Len::Unchecked, (**el).clone()),
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
            TExprKind::StructLit(inits) if matches!(&e.ty, Type::Struct(n) if self.structs[n.as_str()].is_union) => {
                let Type::Struct(name) = &e.ty else { unreachable!() };
                let (i, x) = &inits[0];
                let field = self.structs[name.as_str()].fields[*i].0.clone();
                Rvalue::UnionLit { name: name.clone(), field, val: self.operand(x, out) }
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
            TExprKind::ArrayRepeat(x) => Rvalue::ArrayRepeat { ty: e.ty.clone(), val: self.operand(x, out) },
            TExprKind::IsVariant { expr, index } => {
                let p = self.place(expr, out);
                let tag = self.temp(Type::Int(IntTy::U32));
                self.push(out, StmtKind::Assign(local(tag), Rvalue::Load(field(p, "tag"))));
                let rhs = Operand::Int(*index as i128, IntTy::U32);
                Rvalue::Binary { op: BinOp::Eq, lhs: Operand::Local(tag), rhs, ty: Type::Int(IntTy::U32), span: e.span }
            }
            TExprKind::ArrayLit(elems) => {
                let elems = elems.iter().map(|x| self.operand(x, out)).collect();
                Rvalue::ArrayLit { ty: e.ty.clone(), elems }
            }
        }
    }
}
