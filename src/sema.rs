//! Semantic analysis: name resolution and type checking.
//! Produces a typed AST (`Program`) in which every expression carries its
//! concrete type; untyped literals have already been given their type.
//!
//! Error recovery: a statement with an error is reported and dropped, and
//! checking goes on with the next one. A name whose type could not be
//! determined gets `Type::Error`; any use of it fails without a message, so
//! one mistake is reported once.
//!
//! Generics are checked once where they are defined, with each `comptime`
//! type an abstract `Type::Param` that has only the methods of its `where`
//! constraints. Each instance (`Pool(Node)`, `max(i64)`) is memoized, has
//! its constraints checked at the instantiation, and is checked again with
//! concrete types to produce the monomorphized code.

use crate::ast::{self, ArmBody, AssignOp, BinOp, ExprKind, StmtKind, TypeExprKind, UnOp};
use crate::codes::*;
use crate::diag::{Error, Result, Span};
use std::collections::{HashMap, HashSet};
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
        matches!(
            self,
            IntTy::I8 | IntTy::I16 | IntTy::I32 | IntTy::I64 | IntTy::Isize
        )
    }

    /// Bit width. `isize`/`usize` have the target's pointer width.
    pub fn bits(self, l: Layout) -> u32 {
        match self {
            IntTy::I8 | IntTy::U8 => 8,
            IntTy::I16 | IntTy::U16 => 16,
            IntTy::I32 | IntTy::U32 => 32,
            IntTy::I64 | IntTy::U64 => 64,
            IntTy::Isize | IntTy::Usize => l.ptr_bits as u32,
        }
    }

    pub fn min(self, l: Layout) -> i128 {
        if self.signed() {
            -(1i128 << (self.bits(l) - 1))
        } else {
            0
        }
    }

    pub fn max(self, l: Layout) -> i128 {
        if self.signed() {
            (1i128 << (self.bits(l) - 1)) - 1
        } else {
            (1i128 << self.bits(l)) - 1
        }
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

/// The integer widths of a target that the language leaves to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout {
    /// The width of a pointer, `usize` and `isize`.
    pub ptr_bits: u8,
    /// The width of C's `long` (`c_long`, `c_ulong`).
    pub c_long_bits: u8,
}

impl Layout {
    /// macOS and Linux on 64-bit machines.
    pub const LP64: Layout = Layout {
        ptr_bits: 64,
        c_long_bits: 64,
    };
    /// wasm32: 32-bit pointers and `long`.
    pub const WASM32: Layout = Layout {
        ptr_bits: 32,
        c_long_bits: 32,
    };
}

/// The allocator types of std (`std.mem.Allocator`, `std.heap.General`,
/// `std.heap.Arena`). Their methods are compiler intrinsics.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BuiltinTy {
    /// A data pointer and a vtable of byte-level `alloc` and `free`
    /// functions, like `dyn`. Only `General` and `Arena` make one.
    Allocator,
    /// libc `malloc`/`free`; in debug mode it records each allocation's
    /// site and reports leaks and double frees.
    General,
    /// Allocates from a backing allocator; `deinit` frees everything.
    Arena,
}

impl BuiltinTy {
    /// The name in messages.
    pub fn name(self) -> &'static str {
        match self {
            BuiltinTy::Allocator => "std.mem.Allocator",
            BuiltinTy::General => "std.heap.General",
            BuiltinTy::Arena => "std.heap.Arena",
        }
    }

    /// The name in generated C.
    pub fn c_name(self) -> &'static str {
        match self {
            BuiltinTy::Allocator => "Allocator",
            BuiltinTy::General => "General",
            BuiltinTy::Arena => "Arena",
        }
    }
}

/// The error set of `Allocator.create`, `std.mem.AllocError`. Its only
/// error, `OutOfMemory`, has the error value 1.
pub const ALLOC_ERROR: &str = "std::mem::AllocError";

/// `std.log.err`, `std.log.info` and `std.log.line`.
const LOG_ERR: &str = "std::log::err";
const LOG_INFO: &str = "std::log::info";
const LOG_LINE: &str = "std::log::line";
/// `std.testing.allocator()`, the allocator of `@testAllocator()`.
const TESTING_ALLOCATOR: &str = "std::testing::allocator";
pub const OUT_OF_MEMORY: u16 = 1;

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
    /// `simd[N]T`: `N` lanes (a power of two, 2 to 64) of a number or
    /// `bool` (a mask, from a comparison), a C vector.
    Simd(Box<Type>, u64),
    /// `[]T` (mutable = false) or `[]mut T` (mutable = true): a pointer and
    /// a length. Only `[]mut T` elements can be assigned.
    Slice(Box<Type>, bool),
    /// `[*]T`: unchecked C pointer, for FFI.
    CPtr(Box<Type>),
    /// `*T` (mutable = false) or `*mut T` (mutable = true). Never null.
    Ptr(Box<Type>, bool),
    /// `?T`
    Optional(Box<Type>),
    Struct(String),
    Enum(String),
    /// The type of an error value. The name is the error set's name, or
    /// `INFERRED` plus a function name for the inferred set of a `!T`
    /// function.
    ErrorSet(String),
    /// `E!T`: an error of set `E` (the name, as in `ErrorSet`) or a `T`.
    /// Only a function's return type.
    ErrorUnion(String, Box<Type>),
    /// `dyn I`: a pointer to a value of a type that implements interface `I`,
    /// and that type's vtable for `I`.
    Dyn(String),
    /// A predeclared allocator type.
    Builtin(BuiltinTy),
    /// `fn(A, B) R`: a top-level function as a value (a C function
    /// pointer). The result is `Void` for none.
    Fn(Vec<Type>, Box<Type>),
    /// A `comptime` type parameter while its generic is checked: the
    /// generic's name and the parameter's name. Never reaches codegen:
    /// generated code is monomorphized.
    Param(String, String),
    /// The type of something that already has a reported error.
    Error,
}

/// The deepest nesting of a type alias chain.
const MAX_ALIAS_DEPTH: usize = 64;

/// The deepest nesting of generic instances in a type argument: a generic
/// that instantiates itself with a larger type (`Nest(?T)` in `Nest(T)`)
/// would never end. There is no limit on the number of instances.
const MAX_INSTANCE_DEPTH: usize = 32;

/// The built-in `where` constraint that only integer types satisfy.
const INTEGER: &str = "Integer";

/// The built-in `where` constraint that the types `==` compares satisfy:
/// a `T` with it has `==`, `!=` and `@hash`.
const HASH: &str = "Hash";

/// The number of lanes of a `simd` type.
fn v_lanes(t: &Type) -> u64 {
    match t {
        Type::Simd(_, n) => *n,
        _ => 0,
    }
}

/// The hint for a number written where a `simd` is expected.
fn splat_hint(t: &Type) -> String {
    match t {
        Type::Simd(..) => format!(" (make a vector of it with `@splat({t}, x)`)"),
        _ => String::new(),
    }
}

/// Name prefix of the inferred error set of a `!T` function.
const INFERRED: &str = "error set of ";

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

    fn from_name(name: &str, l: Layout) -> Option<Type> {
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
            // C's integer types; `long` has the target's width.
            "c_short" => Type::Int(IntTy::I16),
            "c_ushort" => Type::Int(IntTy::U16),
            "c_int" => Type::Int(IntTy::I32),
            "c_uint" => Type::Int(IntTy::U32),
            "c_long" if l.c_long_bits == 32 => Type::Int(IntTy::I32),
            "c_ulong" if l.c_long_bits == 32 => Type::Int(IntTy::U32),
            "c_long" | "c_longlong" => Type::Int(IntTy::I64),
            "c_ulong" | "c_ulonglong" => Type::Int(IntTy::U64),
            "std::mem::Allocator" => Type::Builtin(BuiltinTy::Allocator),
            "std::heap::General" => Type::Builtin(BuiltinTy::General),
            "std::heap::Arena" => Type::Builtin(BuiltinTy::Arena),
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
            Type::Simd(t, n) => write!(f, "simd[{n}]{t}"),
            Type::Slice(t, false) => write!(f, "[]{t}"),
            Type::Slice(t, true) => write!(f, "[]mut {t}"),
            Type::CPtr(t) => write!(f, "[*]{t}"),
            Type::Ptr(t, false) => write!(f, "*{t}"),
            Type::Ptr(t, true) => write!(f, "*mut {t}"),
            Type::Optional(t) => write!(f, "?{t}"),
            Type::Struct(n) | Type::Enum(n) | Type::ErrorSet(n) => f.write_str(n),
            Type::ErrorUnion(set, t) if set.starts_with(INFERRED) => write!(f, "!{t}"),
            Type::ErrorUnion(set, t) => write!(f, "{set}!{t}"),
            Type::Dyn(i) => write!(f, "dyn {i}"),
            Type::Builtin(b) => f.write_str(b.name()),
            Type::Fn(params, ret) => {
                let ps: Vec<String> = params.iter().map(|p| p.to_string()).collect();
                write!(f, "fn({})", ps.join(", "))?;
                if **ret != Type::Void {
                    write!(f, " {ret}")?;
                }
                Ok(())
            }
            Type::Param(_, n) => f.write_str(n),
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
    /// Every error name; an error's value is its index plus one (0 means
    /// "no error"). The same name in two sets is the same error.
    pub errors: Vec<String>,
    pub ifaces: Vec<IfaceDef>,
    /// The (interface, struct) pairs that are converted to `dyn`; each one
    /// needs a vtable.
    pub vtables: Vec<(String, String)>,
    /// The structs converted to `Allocator` (each needs a vtable of
    /// thunks to its `alloc` and `free`).
    pub alloc_impls: Vec<String>,
    /// The tests: test `i` is the function `test#i` of its module.
    pub tests: Vec<TestDef>,
    /// Top-level `let` constants and `var` globals, with their values.
    pub globals: Vec<GlobalDef>,
    /// The libraries of every `@link`, in order of first appearance.
    pub links: Vec<String>,
    /// The libraries of `@linkOptional` that no `@link` names: linked only
    /// where they are installed.
    pub optional_links: Vec<String>,
    /// The target's integer widths.
    pub layout: Layout,
}

#[derive(Debug, Clone)]
pub struct GlobalDef {
    pub name: String,
    pub ty: Type,
    pub value: ConstVal,
}

/// The value of a global, computed when the program is compiled.
#[derive(Debug, Clone, PartialEq)]
pub enum ConstVal {
    Int(i128),
    Float(f64),
    Bool(bool),
    Str(Vec<u8>),
    /// `null` of an optional type.
    Null,
    /// A value wrapped into an optional type.
    Some(Box<ConstVal>),
    /// An array's elements, or a struct's fields in declaration order.
    Agg(Vec<ConstVal>),
    /// An enum value: the variant's index and its fields in declaration order.
    Variant(usize, Vec<ConstVal>),
    /// An error value (see `Program::errors`).
    Error(u16),
    /// `[]T{}`
    EmptySlice,
    /// A top-level function; `true` for a C function (`extern fn`).
    Fn(String, bool),
    /// `@zeroed(T)` of an array or struct: every byte zero.
    Zeroed,
    /// A union's value: the field that is set and its value.
    Union(usize, Box<ConstVal>),
}

#[derive(Debug, Clone)]
pub struct IfaceDef {
    pub name: String,
    pub methods: Vec<IfaceMethod>,
}

#[derive(Debug, Clone)]
pub struct IfaceMethod {
    pub name: String,
    /// `self: *mut Self` (true) or `self: *Self`.
    pub mutable: bool,
    /// The parameters after `self`.
    pub params: Vec<Type>,
    pub ret: Type,
}

/// A `test` block.
#[derive(Debug, Clone)]
pub struct TestDef {
    /// The module's path; empty for the root module.
    pub module: String,
    pub name: String,
}

impl TestDef {
    /// The name `aye test` shows: `name`, or `module: name` outside the
    /// root module.
    pub fn shown(&self) -> String {
        if self.module.is_empty() {
            self.name.clone()
        } else {
            format!("{}: {}", self.module, self.name)
        }
    }
}

/// The function `name` is a test (`test#N`, `html::test#N`).
pub fn is_test(name: &str) -> bool {
    name.rsplit("::")
        .next()
        .is_some_and(|n| n.starts_with("test#"))
}

#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<(String, Type)>,
    /// An `extern union`: the fields share their memory.
    pub is_union: bool,
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
    /// Ends with `...`.
    pub variadic: bool,
}

/// A function's name is `f`, `Struct.method` or `test#N`.
#[derive(Debug, Clone)]
pub struct FnDef {
    pub name: String,
    pub params: Vec<VarId>,
    pub ret: Type,
    pub vars: Vec<VarInfo>,
    pub body: Vec<TStmt>,
    pub is_main: bool,
    /// `pub extern fn` with a body: exported to C under its own name.
    pub export: bool,
}

#[derive(Debug, Clone)]
pub struct VarInfo {
    pub name: String,
    pub ty: Type,
    pub mutable: bool,
    /// The `let` keyword that declared it, for the fix-it to `var`.
    pub let_span: Option<Span>,
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
    Assign {
        target: TExpr,
        op: Option<BinOp>,
        value: TExpr,
    },
    /// With `capture`, `cond` is a `?T` and the variable gets its value.
    If {
        cond: TExpr,
        capture: Option<VarId>,
        then: Vec<TStmt>,
        els: Vec<TStmt>,
    },
    /// `if cond |value| { then } else |err| { els }` on the error union
    /// `cond`: `value` gets the payload (none for `void`), `err` the error.
    IfErr {
        cond: TExpr,
        value: Option<VarId>,
        then: Vec<TStmt>,
        err: VarId,
        els: Vec<TStmt>,
    },
    /// With `capture`, `cond` is a `?T`: the loop runs while it has a value.
    While {
        cond: TExpr,
        capture: Option<VarId>,
        body: Vec<TStmt>,
    },
    For {
        var: VarId,
        lo: TExpr,
        hi: TExpr,
        body: Vec<TStmt>,
    },
    /// `for index, elem in seq`; `seq` is an array or a slice.
    ForEach {
        index: Option<VarId>,
        elem: VarId,
        seq: TExpr,
        body: Vec<TStmt>,
    },
    Break,
    Continue,
    Return(Option<TExpr>),
    Defer(Vec<TStmt>),
    ErrDefer(Vec<TStmt>),
    Block(Vec<TStmt>),
    Expr(TExpr),
    /// The last statement of a `catch` or `orelse` block that gives the
    /// block's value: the value, computed before the block's `defer`s run.
    Value(TExpr),
    /// `@setSafety(on)`, the first statement of a block: the runtime checks
    /// of the rest of the block are on (as the build mode has them) or off.
    SetSafety(bool),
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
    /// A top-level `let` constant or `var` global.
    Global(String),
    /// `is_extern`: a C function, called by its C name (also an exported
    /// Aye function).
    Call {
        name: String,
        is_extern: bool,
        args: Vec<TExpr>,
    },
    /// A top-level function as a value, of type `fn(...)`.
    FnRef {
        name: String,
        is_extern: bool,
    },
    /// A call of a function value.
    CallValue {
        callee: Box<TExpr>,
        args: Vec<TExpr>,
    },
    /// `@hasLink("name")`: is the library linked? Known when the program
    /// is built, not when it is checked.
    HasLink(String),
    /// `@sizeOf(T)`: the size of `T` in bytes, as C lays it out.
    SizeOf(Type),
    /// `@zeroed(T)`: the all-zero value of the type `ty`.
    Zeroed,
    /// `@ptrCast(T, p)`: the pointer `p` as the pointer type `ty`.
    PtrCast(Box<TExpr>),
    /// `@print(x)` / `@println(x)` / `@println()`.
    Print {
        arg: Option<Box<TExpr>>,
        newline: bool,
    },
    Binary {
        op: BinOp,
        lhs: Box<TExpr>,
        rhs: Box<TExpr>,
    },
    Unary {
        op: UnOp,
        expr: Box<TExpr>,
    },
    Cast(Box<TExpr>),
    /// Struct field by index.
    Field {
        base: Box<TExpr>,
        index: usize,
    },
    SliceLen(Box<TExpr>),
    SlicePtr(Box<TExpr>),
    ArrayLen(Box<TExpr>),
    Index {
        base: Box<TExpr>,
        index: Box<TExpr>,
    },
    Slice {
        base: Box<TExpr>,
        lo: Option<Box<TExpr>>,
        hi: Option<Box<TExpr>>,
    },
    /// Field initializers in source order: (field index, value).
    StructLit(Vec<(usize, TExpr)>),
    /// Enum value: variant index and payload initializers in source order.
    Variant {
        index: usize,
        fields: Vec<(usize, TExpr)>,
    },
    /// Arms in source order. The type is `void` for a `match` statement.
    Match {
        scrutinee: Box<TExpr>,
        arms: Vec<TArm>,
    },
    ArrayLit(Vec<TExpr>),
    /// `[N]T{value, ..}`: `value`, evaluated once, in every element.
    ArrayRepeat(Box<TExpr>),
    /// An error value (see `Program::errors`).
    ErrorValue(u16),
    /// `try x`; the type is the payload of `x`'s error union.
    Try(Box<TExpr>),
    /// `x catch |e| body`; the type is the payload of `x`'s error union.
    Catch {
        expr: Box<TExpr>,
        capture: Option<VarId>,
        body: Box<TArmBody>,
    },
    /// `x orelse body`; the type is the payload of the optional `x`.
    OrElse {
        expr: Box<TExpr>,
        body: Box<TArmBody>,
    },
    /// `x.?`: the value of the optional `x` (a panic if it is `null`).
    Unwrap(Box<TExpr>),
    /// `unreachable`: a panic (in `fast` mode, undefined behavior).
    Unreachable,
    /// `x is .V`: does the enum value `x` hold variant `index`?
    IsVariant {
        expr: Box<TExpr>,
        index: usize,
    },
    /// A call through the vtable of a `dyn` value.
    DynCall {
        recv: Box<TExpr>,
        method: String,
        args: Vec<TExpr>,
    },
    /// A `*T` or `*mut T` (T is the struct `strukt`) converted to `dyn I`.
    ToDyn {
        ptr: Box<TExpr>,
        strukt: String,
    },
    /// `@downcast(*S, d)`: the data pointer of the `dyn iface` value `d` when
    /// it was made from a `strukt`, else `null`.
    Downcast {
        value: Box<TExpr>,
        iface: String,
        strukt: String,
    },
    /// A `*mut S` converted to `Allocator`: calls go to `S.alloc` and
    /// `S.free`.
    ToAllocator {
        ptr: Box<TExpr>,
        strukt: String,
    },
    /// `alloc.create(value)`: type `AllocError!*mut T`.
    Create {
        alloc: Box<TExpr>,
        value: Box<TExpr>,
    },
    /// `alloc.alloc(value, len)`: type `AllocError![]mut T`, every element
    /// set to `value`. With `old`, `alloc.resize(old, len, value)`: the
    /// elements of `old` are copied first, and `old` is freed on success.
    AllocSlice {
        alloc: Box<TExpr>,
        value: Box<TExpr>,
        len: Box<TExpr>,
        old: Option<Box<TExpr>>,
    },
    /// `[]T{}`: the empty slice.
    EmptySlice,
    /// `std.log.err(format, args...)`, `std.log.info` or `std.log.line`
    /// (`prefix` says which): the literal pieces of the format around its
    /// `{}`s (one more than the arguments) and the arguments.
    Log {
        prefix: LogPrefix,
        pieces: Vec<Vec<u8>>,
        args: Vec<TExpr>,
    },
    Intrinsic {
        op: Intrinsic,
        args: Vec<TExpr>,
    },
    /// A `T` returned from an `E!T` function.
    OkWrap(Box<TExpr>),
    /// An error value returned from an `E!T` function.
    ErrWrap(Box<TExpr>),
}

/// What `std.log.err`, `std.log.info` and `std.log.line` write before the
/// message: `error: `, `info: ` or nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogPrefix {
    Err,
    Info,
    None,
}

/// Allocator methods and test builtins. Receivers are passed as pointers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intrinsic {
    GeneralInit,
    GeneralAllocator,
    GeneralDeinit,
    ArenaInit,
    ArenaAllocator,
    ArenaDeinit,
    /// `arena.reset()`: keeps the first block, frees the rest.
    ArenaReset,
    /// `alloc.destroy(p)`: args are the allocator and the pointer.
    Destroy,
    /// `alloc.free(s)`: args are the allocator and the slice.
    Free,
    TestAllocator,
    Expect,
    /// `@expectEqual(expected, actual)`
    ExpectEqual,
    /// `@atomicLoad(p)`, `@atomicStore(p, v)`, `@atomicAdd(p, v)` and
    /// `@atomicSub(p, v)` (the value before), `@atomicCas(p, expected, new)`
    /// (whether it was `expected` and is `new` now); sequentially consistent.
    AtomicLoad,
    AtomicStore,
    AtomicAdd,
    AtomicSub,
    AtomicCas,
    /// `@hash(v)`: a `u64` from the value, the same for values that `==`
    /// finds equal. Lowered to `HashMix` steps.
    Hash,
    /// The hash `args[0]` (a `u64`) mixed with the scalar `args[1]`.
    HashMix,
    /// `@tagName(x)`: the name of an error or enum tag, a static `[]u8`.
    TagName,
    /// `@reduceAdd(v)`, `@reduceMin(v)`, `@reduceMax(v)`: the sum (wrapping
    /// for integers), least or greatest lane of a `simd`.
    ReduceAdd,
    ReduceMin,
    ReduceMax,
    /// `@all(m)`, `@any(m)` of a mask.
    All,
    Any,
    /// `@select(m, a, b)`.
    Select,
}

#[derive(Debug, Clone)]
pub struct TArm {
    /// Variant indexes, or error values in a `match` on errors; empty for
    /// `_` and integer patterns.
    pub variants: Vec<usize>,
    /// In a `match` on an integer: the inclusive ranges the arm matches;
    /// empty for `_`.
    pub ranges: Vec<(i128, i128)>,
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
    /// The C symbol of an `extern("name") fn`.
    link: Option<String>,
    /// A C function whose parameters end with `...`.
    variadic: bool,
}

impl FnSig {
    /// The name codegen sees for a call: the key, with an `extern("name")`
    /// link name in place of the last path segment.
    fn c_key(&self, key: &str) -> String {
        match &self.link {
            Some(l) => format!("{}{l}", &key[..key.rfind("::").map_or(0, |i| i + 2)]),
            None => key.to_string(),
        }
    }
}

/// The pattern of a `match` arm: the variant indexes or error values
/// (empty for `_` and integer patterns) and the integer ranges.
type ArmPat = (Vec<usize>, Vec<(i128, i128)>);

/// A checked global.
struct GlobalInfo {
    ty: Type,
    mutable: bool,
    /// The `let` keyword of a constant, for the fix-it to `var`.
    kw_span: Span,
    value: Option<ConstVal>,
}

/// The default values of a struct's fields (aligned with its fields), with
/// the `comptime` parameters and the module they are checked in.
struct DefaultDecls<'a> {
    exprs: Vec<Option<&'a ast::Expr>>,
    env: Vec<(String, Type, Span)>,
    module: String,
}

/// The default value expressions of the fields `decls`, aligned with the
/// fields `fields` keeps (a duplicate is left out).
fn default_exprs(decls: &[ast::FieldDecl]) -> Vec<Option<&ast::Expr>> {
    let mut seen: Vec<&str> = Vec::new();
    let mut out = Vec::new();
    for f in decls {
        if !seen.contains(&f.name.as_str()) {
            seen.push(&f.name);
            out.push(f.default.as_ref());
        }
    }
    out
}

/// How a method takes its receiver, from the type of its first parameter.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Recv {
    /// No `Self`, `*Self` or `*mut Self` first parameter: only `Type.f()`.
    Static,
    /// `self: Self`
    Value,
    /// `self: *Self` (false) or `self: *mut Self` (true).
    Ptr(bool),
}

/// A function to check: its declaration, its struct (for a method), and
/// the `comptime` parameters in scope with the types they stand for.
#[derive(Clone)]
struct FnCtx<'a> {
    decl: &'a ast::FnDecl,
    owner: Option<String>,
    env: Vec<(String, Type, Span)>,
    /// The generic that this function is an instance of, or a method of an
    /// instance of.
    generic: Option<String>,
}

impl FnCtx<'_> {
    /// The module of the function `key` with this context.
    fn module(&self, key: &str) -> String {
        module_of(
            self.generic
                .as_deref()
                .or(self.owner.as_deref())
                .unwrap_or(key),
        )
        .to_string()
    }
}

struct Checker<'a> {
    structs: HashMap<String, StructDef>,
    enums: HashMap<String, EnumDef>,
    /// Error sets (named and inferred) and the names of their errors.
    sets: HashMap<String, Vec<String>>,
    /// `Program::errors`.
    error_names: Vec<String>,
    /// Functions by name: `f`, `Struct.method` and `test#N`.
    fns: HashMap<String, FnSig>,
    /// Each function's declaration and context.
    fn_decls: HashMap<String, FnCtx<'a>>,
    /// The receiver of each method (`Struct.method`).
    recvs: HashMap<String, Recv>,
    ifaces: HashMap<String, IfaceDef>,
    vtables: Vec<(String, String)>,
    alloc_impls: Vec<String>,
    /// Checked bodies; `None` while a body is being checked.
    bodies: HashMap<String, Option<FnDef>>,
    /// Top-level names (structs, enums, fns, extern fns) and where they are declared.
    top_spans: HashMap<String, Span>,
    errors: Vec<Error>,
    /// Number of failed checks, including silent follow-up ones.
    failures: usize,
    /// Errors are counted but not reported (in the instances of a generic
    /// whose definition has errors).
    quiet: bool,
    /// Generic functions (with `comptime` parameters) and type functions
    /// (returning `type`), by name.
    generics: HashMap<String, &'a ast::FnDecl>,
    /// Instances, memoized: (generic, type arguments) to the struct name of
    /// a type function's instance, or the key of a generic function's.
    instances: HashMap<(String, Vec<Type>), String>,
    /// Instance structs in order of creation.
    instance_structs: Vec<String>,
    /// Where each instance for concrete types was first made: errors in
    /// its body (a literal that does not fit its integer type) name it.
    instance_sites: HashMap<String, Span>,
    /// The instance whose body is being checked, and its site.
    in_instance: Option<(String, Span)>,
    /// The names of every instance (`instances`' values).
    instance_names: HashSet<String>,
    /// The nesting depth of each instance struct (see `type_depth`).
    instance_depth: HashMap<String, usize>,
    /// Instance structs whose type arguments mention a type parameter.
    abstract_structs: HashSet<String>,
    /// The `where` constraints of each type parameter: interface names.
    bounds: HashMap<(String, String), Vec<String>>,
    /// Generics whose definition is checked or being checked, and those of
    /// them that have errors.
    defined: HashSet<String>,
    failed: HashSet<String>,
    /// Constraint checks and recursive-type checks of instances made before
    /// every method signature is known; done after that.
    sigs_done: bool,
    pending_bounds: Vec<(String, Vec<Type>, Span)>,
    /// Instances made before their constraint check failed: their methods
    /// are checked quietly (the error is at the instantiation).
    unsatisfied: Vec<String>,
    pending_recursive: Vec<(String, Span)>,
    /// Function bodies to check, in order. Instances add their functions.
    queue: Vec<String>,
    /// Top-level `let` constants and `var` globals, in declaration order;
    /// each one's type, mutability and value once its initializer is
    /// checked (`None`: it has an error), and those being checked.
    global_decls: Vec<&'a ast::GlobalDecl>,
    globals: HashMap<String, GlobalInfo>,
    globals_active: Vec<String>,
    /// Type aliases (`let Name = Type`), and the types they name once
    /// resolved, and those being resolved.
    alias_decls: HashMap<String, &'a ast::GlobalDecl>,
    aliases: HashMap<String, Type>,
    aliases_active: Vec<String>,
    /// The default values of struct fields (`x: T = value`), by struct:
    /// each field's expression, and the context to check them in.
    default_decls: HashMap<String, DefaultDecls<'a>>,
    /// Checked defaults (`None`: it has an error), and those being checked.
    defaults: HashMap<(String, usize), Option<TExpr>>,
    defaults_active: Vec<(String, usize)>,
    // Per-function state.
    vars: Vec<VarInfo>,
    scopes: Vec<HashMap<String, (VarId, Span)>>,
    ret: Type,
    loop_depth: u32,
    in_defer: bool,
    /// The struct whose method is checked (what `Self` means).
    self_ty: Option<String>,
    in_test: bool,
    /// The `comptime` parameters in scope: name, the type it stands for (a
    /// `Type::Param` while the generic itself is checked), declaration.
    env: Vec<(String, Type, Span)>,
    /// The module of the function being checked (`""`: the root module).
    module: String,
    /// The `extern struct`s: C layout, C-compatible fields.
    extern_structs: HashSet<String>,
    /// `Program::links`.
    links: Vec<String>,
    /// The libraries of `@linkOptional`.
    optional_links: Vec<String>,
    /// The program's files, indexed by `Span::file`.
    files: Vec<String>,
    /// The target's integer widths.
    layout: Layout,
}

/// Check a file. Without `require_main` (`aye test`, or `aye check` of a
/// file with tests) the file does not need `pub fn main()`.
/// `files` are the names of the program's files, indexed by `Span::file`;
/// `layout` gives the target's integer widths.
pub fn check(
    file: &ast::File,
    require_main: bool,
    files: &[String],
    layout: Layout,
) -> std::result::Result<Program, Vec<Error>> {
    let mut c = Checker {
        files: files.to_vec(),
        structs: HashMap::new(),
        enums: HashMap::new(),
        sets: HashMap::from([(ALLOC_ERROR.to_string(), vec!["OutOfMemory".to_string()])]),
        error_names: vec!["OutOfMemory".to_string()],
        fns: HashMap::new(),
        fn_decls: HashMap::new(),
        recvs: HashMap::new(),
        ifaces: HashMap::new(),
        vtables: Vec::new(),
        alloc_impls: Vec::new(),
        bodies: HashMap::new(),
        top_spans: HashMap::new(),
        errors: Vec::new(),
        failures: 0,
        quiet: false,
        generics: HashMap::new(),
        instances: HashMap::new(),
        instance_structs: Vec::new(),
        instance_depth: HashMap::new(),
        instance_names: HashSet::new(),
        instance_sites: HashMap::new(),
        in_instance: None,
        abstract_structs: HashSet::new(),
        bounds: HashMap::new(),
        defined: HashSet::new(),
        failed: HashSet::new(),
        sigs_done: false,
        pending_bounds: Vec::new(),
        unsatisfied: Vec::new(),
        pending_recursive: Vec::new(),
        queue: Vec::new(),
        global_decls: Vec::new(),
        globals: HashMap::new(),
        globals_active: Vec::new(),
        alias_decls: HashMap::new(),
        aliases: HashMap::new(),
        aliases_active: Vec::new(),
        default_decls: HashMap::new(),
        defaults: HashMap::new(),
        defaults_active: Vec::new(),
        vars: Vec::new(),
        scopes: Vec::new(),
        ret: Type::Void,
        loop_depth: 0,
        in_defer: false,
        self_ty: None,
        in_test: false,
        env: Vec::new(),
        module: String::new(),
        extern_structs: HashSet::new(),
        links: Vec::new(),
        optional_links: Vec::new(),
        layout,
    };
    let p = c.program(file, require_main);
    if c.errors.is_empty() {
        Ok(p)
    } else {
        Err(c.errors)
    }
}

fn err<T>(code: Code, span: Span, msg: impl Into<String>) -> Result<T> {
    Err(Error::new(code, span, msg))
}

/// A failure caused by an error that was already reported.
fn silent() -> Error {
    Error::new(Code(0), Span::default(), "")
}

fn mismatch(span: Span, expected: &Type, found: &Type) -> Error {
    let hint = match (expected, found) {
        (Type::Bool, Type::Simd(..)) => {
            " (a comparison of `simd` values is a mask: use `@all(m)` or `@any(m)`)"
        }
        _ => "",
    };
    Error::new(
        E0023,
        span,
        format!("mismatched types: expected `{expected}`, found `{found}`{hint}"),
    )
}

/// Is this expression an untyped numeric literal (whose type comes from context)?
fn is_untyped(e: &ast::Expr) -> bool {
    match &e.kind {
        ExprKind::Int(_) | ExprKind::Char(_) | ExprKind::Float(_) => true,
        ExprKind::Paren(inner) => is_untyped(inner),
        ExprKind::Unary {
            op: UnOp::Neg | UnOp::BitNot,
            expr,
        } => is_untyped(expr),
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

/// Does `f` return `type` (a type function)?
fn is_type_fn(f: &ast::FnDecl) -> bool {
    matches!(&f.ret, Some(ast::TypeExpr { kind: TypeExprKind::Named(n), .. }) if n == "type")
}

/// Is `f` generic: a type function, or a function with `comptime` parameters?
fn is_generic(f: &ast::FnDecl) -> bool {
    is_type_fn(f) || f.params.iter().any(|p| p.comptime)
}

/// The struct that a type function returns: its body is `return struct { ... }`.
fn type_fn_struct(f: &ast::FnDecl) -> Option<&ast::StructDecl> {
    match &f.body.as_ref()?.stmts[..] {
        [
            ast::Stmt {
                kind:
                    StmtKind::Return(Some(ast::Expr {
                        kind: ExprKind::StructType(d),
                        ..
                    })),
                ..
            },
        ] => Some(d),
        _ => None,
    }
}

/// The type that an expression names, if it has the shape of one: `T`,
/// `F(A, B)` or a type that does not read as an expression (`*T`, ...).
fn expr_type(e: &ast::Expr) -> Option<ast::TypeExpr> {
    let kind = match &e.kind {
        ExprKind::Ident(n) => TypeExprKind::Named(n.clone()),
        ExprKind::Call { name, args } => TypeExprKind::Generic(
            name.clone(),
            args.iter().map(expr_type).collect::<Option<_>>()?,
        ),
        ExprKind::Type(t) => return Some(t.clone()),
        _ => return None,
    };
    Some(ast::TypeExpr { kind, span: e.span })
}

/// Primitive and predeclared type names, for spelling suggestions.
const TYPE_NAMES: &[&str] = &[
    "i8",
    "i16",
    "i32",
    "i64",
    "isize",
    "u8",
    "u16",
    "u32",
    "u64",
    "usize",
    "f32",
    "f64",
    "bool",
    "c_short",
    "c_ushort",
    "c_int",
    "c_uint",
    "c_long",
    "c_ulong",
    "c_longlong",
    "c_ulonglong",
];

/// Is `name` a primitive or predeclared type name?
pub fn is_type_name(name: &str) -> bool {
    TYPE_NAMES.contains(&name)
}

/// The module of the canonical name `name` (`util` for `util::f`; `""`
/// for the root module).
fn module_of(name: &str) -> &str {
    name.rsplit_once("::").map_or("", |(m, _)| m)
}

/// `v` as a value of `it`, or an overflow error in a constant at `span`.
fn fits(v: i128, it: IntTy, l: Layout, span: Span) -> Result<i128> {
    if v < it.min(l) || v > it.max(l) {
        return err(
            E0089,
            span,
            format!(
                "integer overflow in a constant: `{v}` does not fit in `{}`",
                it.name()
            ),
        );
    }
    Ok(v)
}

/// The value of `@zeroed(t)` in a constant.
fn zero_of(t: &Type) -> ConstVal {
    match t {
        Type::Int(_) => ConstVal::Int(0),
        Type::Float(_) => ConstVal::Float(0.0),
        Type::Bool => ConstVal::Bool(false),
        Type::Optional(_) => ConstVal::Null,
        _ => ConstVal::Zeroed,
    }
}

/// `v` wrapped around to a value of `it`.
fn wrap(v: i128, it: IntTy, l: Layout) -> i128 {
    let bits = it.bits(l);
    let m = v & ((1i128 << bits) - 1);
    if it.signed() && m >= 1i128 << (bits - 1) {
        m - (1i128 << bits)
    } else {
        m
    }
}

/// A float value of type `t`: rounded to `f32` precision for `f32`.
fn round_float(v: f64, t: &Type) -> f64 {
    if *t == Type::Float(FloatTy::F32) {
        v as f32 as f64
    } else {
        v
    }
}

/// `a op b` on integers of type `it` in a constant, with the checks that
/// the program makes at run time.
fn fold_int(op: BinOp, a: i128, b: i128, it: IntTy, l: Layout, span: Span) -> Result<i128> {
    let bits = it.bits(l) as i128;
    match op {
        BinOp::Add => fits(a + b, it, l, span),
        BinOp::Sub => fits(a - b, it, l, span),
        BinOp::Mul => fits(a.checked_mul(b).unwrap_or(i128::MAX), it, l, span),
        BinOp::Div | BinOp::Rem if b == 0 => err(E0089, span, "division by zero in a constant"),
        BinOp::Div => fits(a / b, it, l, span),
        // `MIN % -1` overflows like `MIN / -1`.
        BinOp::Rem => fits(a / b, it, l, span).map(|_| a % b),
        BinOp::AddWrap => Ok(wrap(a + b, it, l)),
        BinOp::SubWrap => Ok(wrap(a - b, it, l)),
        BinOp::MulWrap => Ok(wrap(a.wrapping_mul(b), it, l)),
        BinOp::BitAnd => Ok(a & b),
        BinOp::BitOr => Ok(a | b),
        BinOp::BitXor => Ok(a ^ b),
        BinOp::Shl | BinOp::Shr if b < 0 || b >= bits => err(
            E0089,
            span,
            format!(
                "shift amount `{b}` is out of range for `{}` (0 to {})",
                it.name(),
                bits - 1
            ),
        ),
        BinOp::Shl => Ok(wrap(a << b, it, l)),
        _ => Ok(a >> b),
    }
}

/// Are `a` and `b` one edit apart: one character inserted, deleted or
/// replaced, or two adjacent characters swapped?
fn one_edit(a: &str, b: &str) -> bool {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let (long, short) = if a.len() >= b.len() {
        (&a, &b)
    } else {
        (&b, &a)
    };
    match long.len() - short.len() {
        0 => {
            let diff: Vec<usize> = (0..a.len()).filter(|&i| a[i] != b[i]).collect();
            match diff[..] {
                [_] => true,
                [i, j] => j == i + 1 && a[i] == b[j] && a[j] == b[i],
                _ => false,
            }
        }
        1 => {
            let i = (0..short.len())
                .find(|&i| long[i] != short[i])
                .unwrap_or(short.len());
            long[i + 1..] == short[i..]
        }
        _ => false,
    }
}

/// Add a fix-it that renames `name` (at `span`) to the only candidate one
/// edit away, if there is exactly one and `name` has at least 3 characters.
fn suggest<'n>(
    e: Error,
    name: &str,
    span: Span,
    candidates: impl IntoIterator<Item = &'n str>,
) -> Error {
    if name.chars().count() < 3 {
        return e;
    }
    let mut close: Vec<&str> = candidates
        .into_iter()
        .filter(|c| one_edit(name, c))
        .collect();
    close.sort();
    close.dedup();
    match close[..] {
        [c] => {
            let end = span.right(name.chars().count() as u32);
            e.fix(format!("change `{name}` to `{c}`"), span, end, c)
        }
        _ => e,
    }
}

/// Is `b` just `return [value]`, `break` or `continue`, the form that a
/// `match` arm may take in a `match` that produces a value?
fn is_jump_block(b: &ast::Block) -> bool {
    matches!(
        &b.stmts[..],
        [ast::Stmt {
            kind: StmtKind::Return(_) | StmtKind::Break | StmtKind::Continue,
            ..
        }]
    )
}

/// Is a `match` at the end of a value block a statement? It is when an arm
/// is a block that does more than leave, or when no arm gives a value.
fn match_is_stmt(arms: &[ast::MatchArm]) -> bool {
    let block = |a: &ast::MatchArm| matches!(a.body, ArmBody::Block(_));
    let value = |a: &ast::MatchArm| matches!(&a.body, ArmBody::Expr(x) if !matches!(strip_parens(x).kind, ExprKind::Unreachable));
    arms.iter()
        .any(|a| matches!(&a.body, ArmBody::Block(b) if !is_jump_block(b)))
        || (arms.iter().any(block) && !arms.iter().any(value))
}

/// Does every path through `stmts` end in `return` (or, with `or_jumps`,
/// in `break` or `continue`)?
fn block_returns(stmts: &[TStmt], or_jumps: bool) -> bool {
    stmts.iter().any(|s| match &s.kind {
        TStmtKind::Return(_)
        | TStmtKind::Expr(TExpr {
            kind: TExprKind::Unreachable,
            ..
        }) => true,
        TStmtKind::Break | TStmtKind::Continue => or_jumps,
        TStmtKind::Block(b) => block_returns(b, or_jumps),
        TStmtKind::If { then, els, .. } | TStmtKind::IfErr { then, els, .. } => {
            block_returns(then, or_jumps) && block_returns(els, or_jumps)
        }
        TStmtKind::Expr(TExpr {
            kind: TExprKind::Match { arms, .. },
            ..
        }) => arms.iter().all(|a| match &a.body {
            TArmBody::Block(b) => block_returns(b, or_jumps),
            TArmBody::Expr(x) => matches!(x.kind, TExprKind::Unreachable),
        }),
        _ => false,
    })
}

/// Does a `catch` or `orelse` block end with its value?
fn gives_value(stmts: &[TStmt]) -> bool {
    matches!(
        stmts.last(),
        Some(TStmt {
            kind: TStmtKind::Value(_),
            ..
        })
    )
}

/// Is `got` the type `want` with `mut` dropped: `*mut T` for `*T`, `[]mut T`
/// for `[]T`, or either inside `?`? The two have one representation, so the
/// value converts as it is.
fn drops_mut(got: &Type, want: &Type) -> bool {
    match (want, got) {
        (Type::Ptr(w, false), Type::Ptr(g, true))
        | (Type::Slice(w, false), Type::Slice(g, true)) => w == g,
        (Type::Optional(w), Type::Optional(g)) => drops_mut(g, w),
        _ => false,
    }
}

/// The position `at` in a message about `here`: `line:col`, or
/// `file:line:col` when `at` is in another file than `here` (`files` are
/// the file names, indexed by `Span::file`).
pub fn place(files: &[String], at: Span, here: Span) -> String {
    match files.get(at.file as usize) {
        Some(f) if at.file != here.file => format!("{f}:{}:{}", at.line, at.col),
        _ => format!("{}:{}", at.line, at.col),
    }
}

/// Does a function returning `t` need a `return` with a value on every path?
fn needs_value(t: &Type) -> bool {
    match t {
        Type::Void => false,
        Type::ErrorUnion(_, p) => **p != Type::Void,
        _ => true,
    }
}

/// The C type that `t` becomes, written as a type: a pointer is a C
/// pointer, whether it is `mut`, `[*]` or optional. Two declarations of one
/// C function must have the same shape.
fn c_shape(t: &Type) -> Type {
    match t {
        Type::Ptr(e, _) | Type::CPtr(e) => Type::CPtr(Box::new(c_shape(e))),
        Type::Optional(e) if matches!(**e, Type::Ptr(..) | Type::CPtr(_) | Type::Fn(..)) => {
            c_shape(e)
        }
        Type::Fn(params, ret) => {
            Type::Fn(params.iter().map(c_shape).collect(), Box::new(c_shape(ret)))
        }
        Type::Array(e, n) => Type::Array(Box::new(c_shape(e)), *n),
        t => t.clone(),
    }
}

/// An error of set `set` in a message: `` `Set.Name` ``, or `` `Name` `` for
/// an inferred set.
fn qualify(set: &str, name: &str) -> String {
    if set.starts_with(INFERRED) {
        format!("`{name}`")
    } else {
        format!("`{set}.{name}`")
    }
}

/// An error set in a message.
fn describe_set(set: &str) -> String {
    match set.strip_prefix(INFERRED) {
        Some(f) => format!("the inferred error set of `{f}`"),
        None => format!("error set `{set}`"),
    }
}

impl<'a> Checker<'a> {
    /// The position `at` in a message about `here`: `line:col`, with the
    /// file's name first when it is another file.
    fn place(&self, at: Span, here: Span) -> String {
        place(&self.files, at, here)
    }

    /// Record an error and go on. Follow-up errors are dropped: the silent
    /// error, and any message that mentions `Type::Error`.
    fn report(&mut self, mut e: Error) {
        self.failures += 1;
        if !self.quiet && !e.msg.is_empty() && !e.msg.contains(ERROR_TY) {
            // Instance names are shown without the suffix that tells apart
            // instances that print alike (`instance_name`).
            while let Some(i) = e.msg.find(")#") {
                let digits = e.msg[i + 2..]
                    .chars()
                    .take_while(char::is_ascii_digit)
                    .count();
                e.msg.replace_range(i + 1..i + 2 + digits, "");
            }
            if let Some((name, site)) = self.in_instance.as_ref().filter(|_| e.code != E0080) {
                e.msg = format!(
                    "{} (in the instance `{name}` made at {})",
                    e.msg,
                    self.place(*site, e.span)
                );
            }
            // Canonical names of other modules are shown as paths.
            e.msg = e.msg.replace("::", ".");
            self.errors.push(e);
        }
    }

    fn program(&mut self, file: &'a ast::File, require_main: bool) -> Program {
        // 1. Top-level names. An item whose name is rejected is skipped.
        let mut items = Vec::new();
        let mut tests: Vec<&'a ast::TestDecl> = Vec::new();
        for item in &file.items {
            let (name, span) = match item {
                ast::Item::Struct(s) => (&s.name, s.span),
                ast::Item::Enum(e) => (&e.name, e.span),
                ast::Item::ErrorSet(e) => (&e.name, e.span),
                ast::Item::Fn(f) => (&f.name, f.span),
                ast::Item::Interface(i) => (&i.name, i.span),
                ast::Item::Global(g) => (&g.name, g.span),
                // The module loader replaces each with a module.
                ast::Item::CImport(_) => continue,
                ast::Item::Link(l) => {
                    let list = if l.optional {
                        &mut self.optional_links
                    } else {
                        &mut self.links
                    };
                    if !list.contains(&l.name) {
                        list.push(l.name.clone());
                    }
                    continue;
                }
                ast::Item::Test(t) => {
                    if tests
                        .iter()
                        .any(|x| x.name == t.name && x.module == t.module)
                    {
                        self.report(Error::new(
                            E0015,
                            t.func.span,
                            format!("test \"{}\" is defined more than once", t.name),
                        ));
                        continue;
                    }
                    tests.push(t);
                    items.push(item);
                    continue;
                }
            };
            let bare = name.rsplit("::").next().unwrap();
            if Type::from_name(bare, self.layout).is_some()
                || matches!(
                    bare,
                    "true" | "false" | "null" | "Self" | "type" | "simd" | INTEGER | HASH
                )
            {
                self.report(Error::new(
                    E0014,
                    span,
                    format!("`{name}` is a builtin name and cannot be redefined"),
                ));
                continue;
            }
            if self.top_spans.contains_key(name) {
                self.report(Error::new(
                    E0015,
                    span,
                    format!("`{name}` is defined more than once"),
                ));
                continue;
            }
            self.top_spans.insert(name.clone(), span);
            match item {
                ast::Item::Struct(s) => {
                    self.structs.insert(
                        s.name.clone(),
                        StructDef {
                            name: s.name.clone(),
                            fields: vec![],
                            is_union: s.is_union,
                        },
                    );
                    if s.is_extern {
                        self.extern_structs.insert(s.name.clone());
                    }
                }
                ast::Item::Enum(e) => {
                    self.enums.insert(
                        e.name.clone(),
                        EnumDef {
                            name: e.name.clone(),
                            variants: vec![],
                        },
                    );
                }
                ast::Item::ErrorSet(e) => {
                    self.sets.insert(e.name.clone(), vec![]);
                }
                ast::Item::Interface(i) => {
                    self.ifaces.insert(
                        i.name.clone(),
                        IfaceDef {
                            name: i.name.clone(),
                            methods: vec![],
                        },
                    );
                }
                ast::Item::Fn(f) if is_generic(f) => {
                    self.generics.insert(f.name.clone(), f);
                }
                ast::Item::Global(g) => self.global_decls.push(g),
                ast::Item::Fn(_)
                | ast::Item::Test(_)
                | ast::Item::Link(_)
                | ast::Item::CImport(_) => {}
            }
            items.push(item);
        }
        // A top-level `let` whose value names a type is a type alias.
        let (aliases, globals): (Vec<_>, Vec<_>) = self
            .global_decls
            .clone()
            .into_iter()
            .partition(|g| self.is_alias(g, 0));
        self.global_decls = globals;
        let alias_names: Vec<(String, Span)> =
            aliases.iter().map(|g| (g.name.clone(), g.span)).collect();
        self.alias_decls = aliases.into_iter().map(|g| (g.name.clone(), g)).collect();
        self.error_sets(&items);
        // Generics: each one's declaration, and its instance for its own
        // parameters, through which its definition is checked with the
        // parameters abstract. Their bodies are checked first (phase 4).
        let generics: Vec<&'a ast::FnDecl> = items
            .iter()
            .filter_map(|i| match i {
                ast::Item::Fn(f) if is_generic(f) => Some(f),
                _ => None,
            })
            .collect();
        for f in generics {
            self.define_generic(&f.name);
        }
        for (name, span) in &alias_names {
            let _ = self.alias(name, *span);
        }
        // 2. Struct fields, enum variants and interface methods.
        for item in &items {
            match item {
                ast::Item::Struct(s) => {
                    let fields = self.fields(&s.fields, &format!("struct `{}`", s.name));
                    if s.is_extern {
                        for f in &s.fields {
                            let Some((_, ty)) = fields.iter().find(|(n, _)| *n == f.name) else {
                                continue;
                            };
                            if !self.c_compat(ty, true) {
                                self.report(Error::new(E0052, f.ty.span, format!(
                                    "field `{}` of `extern struct {}` has type `{ty}`, which is not C-compatible{}",
                                    f.name,
                                    s.name,
                                    self.c_hint(ty, true)
                                )));
                            }
                        }
                    }
                    self.structs.get_mut(&s.name).unwrap().fields = fields;
                    let exprs = default_exprs(&s.fields);
                    if let (true, Some(f)) =
                        (s.is_union, s.fields.iter().find(|f| f.default.is_some()))
                    {
                        self.report(Error::new(E0104, f.span, format!(
                            "a field of the union `{}` cannot have a default value: a literal sets exactly one field",
                            s.name
                        )));
                    } else if exprs.iter().any(Option::is_some) {
                        let module = module_of(&s.name).to_string();
                        self.default_decls.insert(
                            s.name.clone(),
                            DefaultDecls {
                                exprs,
                                env: Vec::new(),
                                module,
                            },
                        );
                    }
                }
                ast::Item::Enum(e) => {
                    if e.variants.is_empty() {
                        self.report(Error::new(
                            E0054,
                            e.span,
                            format!("enum `{}` has no variants", e.name),
                        ));
                    }
                    let mut variants: Vec<VariantDef> = Vec::new();
                    for v in &e.variants {
                        if variants.iter().any(|x| x.name == v.name) {
                            self.report(Error::new(
                                E0015,
                                v.span,
                                format!("duplicate variant `{}` in enum `{}`", v.name, e.name),
                            ));
                            continue;
                        }
                        let fields =
                            self.fields(&v.fields, &format!("variant `{}.{}`", e.name, v.name));
                        variants.push(VariantDef {
                            name: v.name.clone(),
                            fields,
                        });
                    }
                    self.enums.get_mut(&e.name).unwrap().variants = variants;
                }
                ast::Item::Interface(i) => {
                    let methods = self.iface_methods(i);
                    self.ifaces.get_mut(&i.name).unwrap().methods = methods;
                }
                ast::Item::ErrorSet(_)
                | ast::Item::Fn(_)
                | ast::Item::Test(_)
                | ast::Item::Global(_)
                | ast::Item::Link(_)
                | ast::Item::CImport(_) => {}
            }
        }
        for item in &items {
            let (name, span) = match item {
                ast::Item::Struct(s) => (&s.name, s.span),
                ast::Item::Enum(e) => (&e.name, e.span),
                _ => continue,
            };
            if let Err(e) = self.check_recursive(name, name, span, &mut Vec::new()) {
                self.report(e);
            }
        }
        // 3. Function, method and test signatures.
        let mut main_found = false;
        let mut externs = Vec::new();
        // C functions (declared and exported) by C name: the function type,
        // its C shape, whether it is variadic, and whether it is defined.
        let mut symbols: HashMap<String, (Type, Type, bool, bool)> = HashMap::new();
        let mut decls: Vec<(String, &'a ast::FnDecl, Option<String>)> = Vec::new();
        for item in &items {
            match item {
                ast::Item::Fn(f) if !is_generic(f) => decls.push((f.name.clone(), f, None)),
                ast::Item::Struct(s) => {
                    for m in &s.methods {
                        if s.fields.iter().any(|f| f.name == m.name) {
                            self.report(Error::new(
                                E0015,
                                m.span,
                                format!(
                                    "`{}` is both a field and a method of struct `{}`",
                                    m.name, s.name
                                ),
                            ));
                            continue;
                        }
                        let key = format!("{}.{}", s.name, m.name);
                        if decls.iter().any(|(k, ..)| *k == key) {
                            self.report(Error::new(
                                E0015,
                                m.span,
                                format!("duplicate method `{}` in struct `{}`", m.name, s.name),
                            ));
                            continue;
                        }
                        decls.push((key, m, Some(s.name.clone())));
                    }
                }
                ast::Item::Enum(e) => {
                    for m in &e.methods {
                        if e.variants.iter().any(|v| v.name == m.name) {
                            self.report(Error::new(
                                E0015,
                                m.span,
                                format!(
                                    "`{}` is both a variant and a method of enum `{}`",
                                    m.name, e.name
                                ),
                            ));
                            continue;
                        }
                        let key = format!("{}.{}", e.name, m.name);
                        if decls.iter().any(|(k, ..)| *k == key) {
                            self.report(Error::new(
                                E0015,
                                m.span,
                                format!("duplicate method `{}` in enum `{}`", m.name, e.name),
                            ));
                            continue;
                        }
                        decls.push((key, m, Some(e.name.clone())));
                    }
                }
                ast::Item::Test(t) => decls.push((t.func.name.clone(), &t.func, None)),
                _ => {}
            }
        }
        for (key, f, owner) in &decls {
            if let Some(p) = f.params.iter().find(|p| p.comptime) {
                self.report(Error::new(
                    E0076,
                    p.span,
                    "only a top-level function can have `comptime` parameters",
                ));
            }
            let sig = self.register(key, f, owner.clone(), None);
            if f.is_extern {
                let not_c = |ty: &Type, c: &Self| {
                    format!(
                        "type `{ty}` cannot be used in an `extern fn` signature: it is not C-compatible{}",
                        c.c_hint(ty, false)
                    )
                };
                for (p, ty) in f.params.iter().zip(&sig.params) {
                    if !self.c_compat(ty, false) {
                        self.report(Error::new(E0052, p.ty.span, not_c(ty, self)));
                    }
                }
                if sig.ret != Type::Void && !self.c_compat(&sig.ret, false) {
                    let span = f.ret.as_ref().unwrap().span;
                    self.report(Error::new(E0052, span, not_c(&sig.ret, self)));
                }
                if f.body.is_none() {
                    let (params, ret, variadic) =
                        (sig.params.clone(), sig.ret.clone(), sig.variadic);
                    externs.push(ExternFn {
                        name: f.link.clone().unwrap_or_else(|| f.name.clone()),
                        params,
                        ret,
                        variadic,
                    });
                }
                // One C symbol, one C signature and at most one definition.
                let sym = f
                    .link
                    .clone()
                    .unwrap_or_else(|| key.rsplit("::").next().unwrap().to_string());
                let ty = Type::Fn(sig.params.clone(), Box::new(sig.ret.clone()));
                let (shape, defined) = (c_shape(&ty), f.body.is_some());
                match symbols.get_mut(&sym) {
                    Some((first, s, v, _)) if *s != shape || *v != sig.variadic => {
                        let msg = format!(
                            "C function `{sym}` is declared twice with different signatures: `{first}` and `{ty}`"
                        );
                        self.report(Error::new(E0096, f.span, msg));
                    }
                    Some((.., true)) if defined => {
                        let msg = format!(
                            "C function `{sym}` is defined twice: each `pub extern fn` needs its own name"
                        );
                        self.report(Error::new(E0096, f.span, msg));
                    }
                    Some((.., d)) => *d |= defined,
                    None => {
                        symbols.insert(sym, (ty, shape, sig.variadic, defined));
                    }
                }
            }
            if key == "main" && !f.is_extern {
                main_found = true;
                let ret_ok = matches!(&sig.ret, Type::Void | Type::Error)
                    || matches!(&sig.ret, Type::ErrorUnion(_, p) if **p == Type::Void);
                if !f.is_pub {
                    self.report(Error::new(
                        E0051,
                        f.span,
                        "`main` must be declared `pub fn main()`",
                    ));
                } else if !sig.params.is_empty() || !ret_ok {
                    self.report(Error::new(
                        E0051,
                        f.span,
                        "`main` must take no parameters and return nothing or `!void`",
                    ));
                }
            }
        }
        if require_main && !main_found {
            self.report(Error::new(
                E0051,
                Span::new(1, 1),
                "no `pub fn main()` found",
            ));
        }
        // Every method signature is known now: check the instances made so far.
        self.sigs_done = true;
        for (g, args, span) in std::mem::take(&mut self.pending_bounds) {
            if let Err(e) = self.check_bounds(&g, &args, span) {
                self.report(e);
                if let Some(n) = self.instances.get(&(g, args)) {
                    self.unsatisfied.push(n.clone());
                }
            }
        }
        for (name, span) in std::mem::take(&mut self.pending_recursive) {
            if let Err(e) = self.check_recursive(&name, &name, span, &mut Vec::new()) {
                self.report(e);
            }
        }
        // Globals, before the bodies that use them, and field defaults.
        for g in self.global_decls.clone() {
            let _ = self.global(&g.name, g.span);
        }
        let mut with_defaults: Vec<(String, usize)> = self
            .default_decls
            .iter()
            .flat_map(|(s, d)| {
                d.exprs
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| e.is_some())
                    .map(|(i, _)| (s.clone(), i))
            })
            .collect();
        with_defaults.sort();
        for (s, i) in with_defaults {
            let _ = self.field_default(&s, i, Span::default());
        }
        // 4. Bodies: the generics' own instances first, so that a generic
        // with errors is known before its other instances are checked (they
        // are checked quietly); then the functions, then each instance as it
        // is made.
        self.queue.extend(
            decls
                .iter()
                .filter(|(_, f, _)| f.body.is_some())
                .map(|(k, ..)| k.clone()),
        );
        let own: Vec<String> = self
            .queue
            .iter()
            .filter(|k| self.is_abstract_fn(k))
            .cloned()
            .collect();
        for key in own {
            let _ = self.ensure_body(&key, Span::default());
        }
        let mut i = 0;
        while i < self.queue.len() {
            let key = self.queue[i].clone();
            i += 1;
            let _ = self.ensure_body(&key, Span::default());
        }
        // Code is generated for the concrete functions and structs only.
        let mut fns = Vec::new();
        for key in std::mem::take(&mut self.queue) {
            let def = self.bodies.get_mut(&key).unwrap().take().unwrap();
            if !self.is_abstract_fn(&key) {
                fns.push(def);
            }
        }
        let mut structs = Vec::new();
        let mut enums = Vec::new();
        let mut ifaces = Vec::new();
        for item in &items {
            match item {
                ast::Item::Struct(s) => structs.push(self.structs[&s.name].clone()),
                ast::Item::Enum(e) => enums.push(self.enums[&e.name].clone()),
                ast::Item::Interface(i) => ifaces.push(self.ifaces[&i.name].clone()),
                ast::Item::ErrorSet(_)
                | ast::Item::Fn(_)
                | ast::Item::Test(_)
                | ast::Item::Global(_)
                | ast::Item::Link(_)
                | ast::Item::CImport(_) => {}
            }
        }
        let globals = self
            .global_decls
            .iter()
            .filter_map(|g| {
                let info = &self.globals[&g.name];
                Some(GlobalDef {
                    name: g.name.clone(),
                    ty: info.ty.clone(),
                    value: info.value.clone()?,
                })
            })
            .collect();
        for n in &self.instance_structs {
            if !self.abstract_structs.contains(n) {
                structs.push(self.structs[n].clone());
            }
        }
        let vtables = std::mem::take(&mut self.vtables)
            .into_iter()
            .filter(|(_, s)| !self.abstract_structs.contains(s))
            .collect();
        Program {
            structs,
            enums,
            externs,
            fns,
            errors: std::mem::take(&mut self.error_names),
            ifaces,
            vtables,
            alloc_impls: std::mem::take(&mut self.alloc_impls)
                .into_iter()
                .filter(|s| !self.abstract_structs.contains(s))
                .collect(),
            tests: tests
                .iter()
                .map(|t| TestDef {
                    module: t.module.clone(),
                    name: t.name.clone(),
                })
                .collect(),
            globals,
            optional_links: self
                .optional_links
                .iter()
                .filter(|l| !self.links.contains(l))
                .cloned()
                .collect(),
            links: std::mem::take(&mut self.links),
            layout: self.layout,
        }
    }

    /// Does the top-level `let` `g` name a type (`let NodePool = Pool(Node)`),
    /// so that it is a type alias? Its value has the shape of a type, and
    /// its name is a type, a type function or another alias.
    fn is_alias(&self, g: &ast::GlobalDecl, depth: usize) -> bool {
        if g.mutable || g.ty.is_some() || depth > MAX_ALIAS_DEPTH {
            return false;
        }
        match &g.init.kind {
            ExprKind::Type(_) => true,
            ExprKind::Call { name, .. } => self.generics.get(name).is_some_and(|f| is_type_fn(f)),
            ExprKind::Ident(n) => {
                self.is_type_name(n)
                    || self
                        .global_decls
                        .iter()
                        .find(|x| x.name == *n)
                        .is_some_and(|x| self.is_alias(x, depth + 1))
            }
            _ => false,
        }
    }

    /// The type that the alias `name` names, resolved the first time.
    fn alias(&mut self, name: &str, span: Span) -> Result<Type> {
        if let Some(t) = self.aliases.get(name) {
            return Ok(t.clone());
        }
        if self.aliases_active.iter().any(|a| a == name) {
            return err(
                E0053,
                span,
                format!("type alias `{name}` is defined in terms of itself"),
            );
        }
        let decl = self.alias_decls[name];
        self.aliases_active.push(name.to_string());
        let saved = (std::mem::take(&mut self.env), self.self_ty.take());
        let r = self.resolve_type(&expr_type(&decl.init).unwrap());
        (self.env, self.self_ty) = saved;
        self.aliases_active.pop();
        let ty = r.unwrap_or_else(|e| {
            self.report(e);
            Type::Error
        });
        self.aliases.insert(name.to_string(), ty.clone());
        Ok(ty)
    }

    /// The type and mutability of the global `name`. Its initializer is
    /// checked and computed the first time (`span` is the use).
    fn global(&mut self, name: &str, span: Span) -> Result<(Type, bool)> {
        if let Some(g) = self.globals.get(name) {
            return Ok((g.ty.clone(), g.mutable));
        }
        if self.globals_active.iter().any(|a| a == name) {
            return err(
                E0089,
                span,
                format!("`{name}` is defined in terms of itself"),
            );
        }
        let decl = *self.global_decls.iter().find(|g| g.name == name).unwrap();
        self.globals_active.push(name.to_string());
        let saved = (
            std::mem::take(&mut self.vars),
            std::mem::replace(&mut self.scopes, vec![HashMap::new()]),
            std::mem::replace(&mut self.ret, Type::Void),
            std::mem::take(&mut self.loop_depth),
            std::mem::take(&mut self.in_defer),
            self.self_ty.take(),
            std::mem::take(&mut self.in_test),
            std::mem::take(&mut self.env),
            std::mem::replace(&mut self.module, module_of(name).to_string()),
        );
        let r = match &decl.ty {
            Some(t) => self
                .resolve_type(t)
                .and_then(|ty| self.expect(&decl.init, &ty)),
            None => self.value(&decl.init, None),
        }
        .and_then(|t| Ok((self.fold(&t)?, t.ty)));
        (
            self.vars,
            self.scopes,
            self.ret,
            self.loop_depth,
            self.in_defer,
            self.self_ty,
            self.in_test,
            self.env,
            self.module,
        ) = saved;
        self.globals_active.pop();
        let (value, ty) = match r {
            Ok((v, ty)) => (Some(v), ty),
            Err(e) => {
                self.report(e);
                (None, Type::Error)
            }
        };
        let info = GlobalInfo {
            ty: ty.clone(),
            mutable: decl.mutable,
            kw_span: decl.kw_span,
            value,
        };
        self.globals.insert(name.to_string(), info);
        Ok((ty, decl.mutable))
    }

    /// The value of `e`, a global's initializer, computed now.
    fn fold(&self, e: &TExpr) -> Result<ConstVal> {
        let span = e.span;
        Ok(match &e.kind {
            TExprKind::Int(v) => ConstVal::Int(*v),
            TExprKind::Float(v) => ConstVal::Float(round_float(*v, &e.ty)),
            TExprKind::Bool(b) => ConstVal::Bool(*b),
            TExprKind::Str(s) => ConstVal::Str(s.clone()),
            TExprKind::Null => ConstVal::Null,
            TExprKind::EmptySlice => ConstVal::EmptySlice,
            TExprKind::FnRef { name, is_extern } => ConstVal::Fn(name.clone(), *is_extern),
            TExprKind::Zeroed => zero_of(&e.ty),
            TExprKind::ErrorValue(v) => ConstVal::Error(*v),
            TExprKind::Some(x) => ConstVal::Some(Box::new(self.fold(x)?)),
            TExprKind::Global(n) => match self.globals.get(n) {
                Some(g) if g.mutable => {
                    return err(
                        E0089,
                        span,
                        format!(
                            "`{n}` is a `var`: a global's initializer can use only `let` constants"
                        ),
                    );
                }
                Some(g) => g.value.clone().ok_or_else(silent)?,
                None => return Err(silent()),
            },
            TExprKind::StructLit(inits) if matches!(&e.ty, Type::Struct(n) if self.structs[n].is_union) =>
            {
                let (i, x) = &inits[0];
                ConstVal::Union(*i, Box::new(self.fold(x)?))
            }
            TExprKind::StructLit(inits) | TExprKind::Variant { fields: inits, .. } => {
                let mut vals = inits
                    .iter()
                    .map(|(i, x)| Ok((*i, self.fold(x)?)))
                    .collect::<Result<Vec<_>>>()?;
                vals.sort_by_key(|(i, _)| *i);
                let vals = vals.into_iter().map(|(_, v)| v).collect();
                match &e.kind {
                    TExprKind::Variant { index, .. } => ConstVal::Variant(*index, vals),
                    _ => ConstVal::Agg(vals),
                }
            }
            TExprKind::ArrayLit(elems) => {
                ConstVal::Agg(elems.iter().map(|x| self.fold(x)).collect::<Result<_>>()?)
            }
            TExprKind::ArrayRepeat(x) => match (&e.ty, self.fold(x)?) {
                (_, ConstVal::Zeroed) => ConstVal::Zeroed,
                (Type::Array(_, n) | Type::Simd(_, n), v) => ConstVal::Agg(vec![v; *n as usize]),
                _ => unreachable!(),
            },
            TExprKind::Field { base, index } => match self.fold(base)? {
                ConstVal::Agg(mut fields) => fields.swap_remove(*index),
                ConstVal::Zeroed => zero_of(&e.ty),
                ConstVal::Union(i, v) if i == *index => *v,
                ConstVal::Union(..) => {
                    return err(E0089, span, "this constant union holds another field");
                }
                _ => unreachable!(),
            },
            TExprKind::ArrayLen(base) => match base.ty {
                Type::Array(_, n) => ConstVal::Int(n as i128),
                _ => unreachable!(),
            },
            TExprKind::Index { base, index } if let Type::Array(_, n) = base.ty => {
                let (b, ConstVal::Int(i)) = (self.fold(base)?, self.fold(index)?) else {
                    unreachable!()
                };
                if i >= n as i128 {
                    return err(
                        E0089,
                        span,
                        format!("index {i} is out of bounds for length {n}"),
                    );
                }
                match b {
                    ConstVal::Agg(mut elems) => elems.swap_remove(i as usize),
                    _ => zero_of(&e.ty),
                }
            }
            TExprKind::Unary { op, expr } => match (op, self.fold(expr)?, &e.ty) {
                (UnOp::Not, ConstVal::Bool(b), _) => ConstVal::Bool(!b),
                (UnOp::Neg, ConstVal::Float(f), _) => ConstVal::Float(-f),
                (UnOp::Neg, ConstVal::Int(v), Type::Int(it)) => {
                    ConstVal::Int(fits(-v, *it, self.layout, span)?)
                }
                (UnOp::BitNot, ConstVal::Int(v), Type::Int(it)) => {
                    ConstVal::Int(wrap(!v, *it, self.layout))
                }
                _ => unreachable!(),
            },
            TExprKind::Binary { op, lhs, rhs } => {
                let (l, r) = (self.fold(lhs)?, self.fold(rhs)?);
                let op = *op;
                let cmp = |o: std::cmp::Ordering| match op {
                    BinOp::Eq => o.is_eq(),
                    BinOp::Ne => o.is_ne(),
                    BinOp::Lt => o.is_lt(),
                    BinOp::Le => o.is_le(),
                    BinOp::Gt => o.is_gt(),
                    _ => o.is_ge(),
                };
                match (l, r, &lhs.ty) {
                    (ConstVal::Int(a), ConstVal::Int(b), _) if op.is_comparison() => {
                        ConstVal::Bool(cmp(a.cmp(&b)))
                    }
                    (ConstVal::Int(a), ConstVal::Int(b), Type::Int(it)) => {
                        ConstVal::Int(fold_int(op, a, b, *it, self.layout, span)?)
                    }
                    (ConstVal::Float(a), ConstVal::Float(b), _) if op.is_comparison() => {
                        ConstVal::Bool(
                            a.partial_cmp(&b).is_some_and(cmp) || (op == BinOp::Ne && a.is_nan()),
                        )
                    }
                    (ConstVal::Float(a), ConstVal::Float(b), t) => {
                        let v = match op {
                            BinOp::Add => a + b,
                            BinOp::Sub => a - b,
                            BinOp::Mul => a * b,
                            _ => a / b,
                        };
                        if !v.is_finite() {
                            return err(
                                E0089,
                                span,
                                "the result of this float operation in a constant is not finite",
                            );
                        }
                        ConstVal::Float(round_float(v, t))
                    }
                    (ConstVal::Bool(a), ConstVal::Bool(b), _) => ConstVal::Bool(match op {
                        BinOp::And => a && b,
                        BinOp::Or => a || b,
                        BinOp::Eq => a == b,
                        _ => a != b,
                    }),
                    (ConstVal::Variant(a, _), ConstVal::Variant(b, _), _) => {
                        ConstVal::Bool((a == b) == (op == BinOp::Eq))
                    }
                    (ConstVal::Error(a), ConstVal::Error(b), _) => {
                        ConstVal::Bool((a == b) == (op == BinOp::Eq))
                    }
                    _ => {
                        return err(
                            E0089,
                            span,
                            "this comparison cannot be computed when the program is compiled",
                        );
                    }
                }
            }
            TExprKind::Cast(x) => match (self.fold(x)?, &e.ty) {
                (ConstVal::Bool(b), _) => ConstVal::Int(b as i128),
                (ConstVal::Int(v), Type::Int(it)) => {
                    ConstVal::Int(fits(v, *it, self.layout, span)?)
                }
                (ConstVal::Int(v), t) => ConstVal::Float(round_float(v as f64, t)),
                (ConstVal::Float(f), Type::Int(it)) => {
                    if !(f > it.min(self.layout) as f64 - 1.0
                        && f < it.max(self.layout) as f64 + 1.0)
                    {
                        return err(
                            E0089,
                            span,
                            format!("`{f}` does not fit in `{}`", it.name()),
                        );
                    }
                    ConstVal::Int(f.trunc() as i128)
                }
                (ConstVal::Float(f), t) => ConstVal::Float(round_float(f, t)),
                _ => unreachable!(),
            },
            _ => {
                return err(
                    E0089,
                    span,
                    "a global's initializer must be a constant: literals, `let` constants, operators, `as` casts, and array, struct and enum literals (no calls)",
                );
            }
        })
    }

    /// The default value of field `index` of struct `s`, checked the first
    /// time (`span` is the literal that needs it): a constant of the
    /// field's type, checked in the struct's module.
    fn field_default(&mut self, s: &str, index: usize, span: Span) -> Result<TExpr> {
        let key = (s.to_string(), index);
        if let Some(d) = self.defaults.get(&key) {
            return d.clone().ok_or_else(silent);
        }
        let (fname, ty) = self.structs[s].fields[index].clone();
        if self.defaults_active.contains(&key) {
            return err(
                E0089,
                span,
                format!(
                    "the default value of field `{fname}` of `{s}` is defined in terms of itself"
                ),
            );
        }
        let d = &self.default_decls[s];
        let (expr, env, module) = (d.exprs[index].unwrap(), d.env.clone(), d.module.clone());
        self.defaults_active.push(key.clone());
        let saved = (
            std::mem::take(&mut self.vars),
            std::mem::replace(&mut self.scopes, vec![HashMap::new()]),
            std::mem::replace(&mut self.ret, Type::Void),
            std::mem::take(&mut self.loop_depth),
            std::mem::take(&mut self.in_defer),
            self.self_ty.replace(s.to_string()),
            std::mem::take(&mut self.in_test),
            std::mem::replace(&mut self.env, env),
            std::mem::replace(&mut self.module, module),
        );
        let r = self.expect(expr, &ty).and_then(|t| match self.fold(&t) {
            Ok(_) => Ok(t),
            Err(e) if e.code == E0089 && e.msg.contains("global's initializer") => err(E0103, e.span, format!(
                "the default value of field `{fname}` must be a constant: literals, `let` constants, operators, `as`, `@zeroed`, and array, struct and enum literals (no calls)"
            )),
            Err(e) => Err(e),
        });
        (
            self.vars,
            self.scopes,
            self.ret,
            self.loop_depth,
            self.in_defer,
            self.self_ty,
            self.in_test,
            self.env,
            self.module,
        ) = saved;
        self.defaults_active.pop();
        let d = match r {
            Ok(t) => Some(t),
            Err(e) => {
                self.report(e);
                None
            }
        };
        self.defaults.insert(key, d.clone());
        d.ok_or_else(silent)
    }

    /// Check the generic `g` (once): its declaration, and its instance for
    /// its own parameters, through which its definition is checked with the
    /// parameters abstract (the bodies in phase 4). Done before any other
    /// instance of `g` is made, so that a generic with errors is known.
    fn define_generic(&mut self, g: &str) {
        if !self.defined.insert(g.to_string()) {
            return;
        }
        let f = self.generics[g];
        let failures = self.failures;
        self.generic_decl(f);
        if !f.is_extern {
            let own = self.own_params(f);
            if is_type_fn(f) {
                let _ = self.instantiate_type(g, own, f.span);
            } else {
                let _ = self.instantiate_fn(g, own, f.span);
            }
        }
        if self.failures > failures {
            self.failed.insert(g.to_string());
        }
    }

    /// Is the function `key` an instance for abstract type parameters?
    fn is_abstract_fn(&self, key: &str) -> bool {
        self.fn_decls[key]
            .env
            .iter()
            .any(|(_, t, _)| self.has_param(t))
    }

    /// Check the declaration of the generic `f`: its `comptime` parameters,
    /// its `where` constraints (which are recorded) and, for a type
    /// function, the form of its body.
    fn generic_decl(&mut self, f: &'a ast::FnDecl) {
        let g = &f.name;
        if f.is_extern {
            self.report(Error::new(
                E0076,
                f.span,
                "an `extern fn` cannot have `comptime` parameters",
            ));
        }
        let mut seen: Vec<(&str, Span)> = Vec::new();
        for p in &f.params {
            if !p.comptime {
                if is_type_fn(f) {
                    self.report(Error::new(
                        E0076,
                        p.span,
                        "a function that returns `type` can only have `comptime` parameters",
                    ));
                }
                continue;
            }
            if !matches!(&p.ty.kind, TypeExprKind::Named(n) if n == "type") {
                self.report(Error::new(
                    E0076,
                    p.ty.span,
                    "a `comptime` parameter must have type `type` (other compile-time values are not supported)",
                ));
            }
            // Other modules' top-level names are checked by module resolution.
            let top = if module_of(g).is_empty() {
                self.top_spans.get(&p.name).copied()
            } else {
                None
            };
            let prev = seen.iter().find(|(n, _)| *n == p.name).map(|x| x.1).or(top);
            if let Some(prev) = prev {
                self.report(Error::new(
                    E0016,
                    p.span,
                    format!(
                        "`{}` is already declared at {} (shadowing is not allowed)",
                        p.name,
                        self.place(prev, p.span)
                    ),
                ));
            }
            seen.push((&p.name, p.span));
        }
        for b in &f.bounds {
            if !f.params.iter().any(|p| p.comptime && p.name == b.param) {
                self.report(Error::new(
                    E0076,
                    b.span,
                    format!("`{}` is not a `comptime` parameter of `{g}`", b.param),
                ));
            } else if b.iface != INTEGER && b.iface != HASH && !self.ifaces.contains_key(&b.iface) {
                self.report(Error::new(
                    E0076,
                    b.iface_span,
                    format!(
                        "`{}` is not an interface (a `where` constraint names an interface)",
                        b.iface
                    ),
                ));
            } else {
                self.bounds
                    .entry((g.clone(), b.param.clone()))
                    .or_default()
                    .push(b.iface.clone());
            }
        }
        if is_type_fn(f) && !f.is_extern && type_fn_struct(f).is_none() {
            let stmts = &f.body.as_ref().unwrap().stmts;
            let returns_struct = |s: &ast::Stmt| {
                matches!(
                    &s.kind,
                    StmtKind::Return(Some(ast::Expr {
                        kind: ExprKind::StructType(_),
                        ..
                    }))
                )
            };
            let span = stmts
                .iter()
                .find(|s| !returns_struct(s))
                .or(stmts.get(1))
                .map_or(f.span, |s| s.span);
            self.report(Error::new(
                E0077,
                span,
                "the body of a function that returns `type` must be a single `return struct { ... }`",
            ));
        }
    }

    /// Is `t` a type parameter with `where T: Integer`?
    fn integer_param(&self, t: &Type) -> bool {
        matches!(t, Type::Param(g, p) if self.bounds.get(&(g.clone(), p.clone())).is_some_and(|v| v.iter().any(|i| i == INTEGER)))
    }

    /// Is `t` a type parameter with `where T: Hash` (or `Integer`, whose
    /// types all compare)?
    fn hash_param(&self, t: &Type) -> bool {
        matches!(t, Type::Param(g, p) if self.bounds.get(&(g.clone(), p.clone())).is_some_and(|v| v.iter().any(|i| i == HASH || i == INTEGER)))
    }

    /// Is `t` an integer type, or a type parameter with `where T: Integer`
    /// (which has the integer operators and literals; each instance is
    /// checked again with its integer type)?
    fn int_like(&self, t: &Type) -> bool {
        t.is_int() || self.integer_param(t)
    }

    /// The hint for an operator applied to a type parameter without
    /// `where T: Integer`.
    fn integer_hint(&self, t: &Type) -> String {
        match t {
            Type::Param(_, p) if !self.integer_param(t) => {
                format!(
                    " (`{p}` is a `comptime` type: add `where {p}: Integer` for the integer operators)"
                )
            }
            _ => String::new(),
        }
    }

    /// The generic `f`'s own `comptime` parameters, as abstract types.
    fn own_params(&self, f: &ast::FnDecl) -> Vec<Type> {
        f.params
            .iter()
            .filter(|p| p.comptime)
            .map(|p| Type::Param(f.name.clone(), p.name.clone()))
            .collect()
    }

    /// The `comptime` parameters of `f` bound to `args`.
    fn comptime_env(f: &ast::FnDecl, args: &[Type]) -> Vec<(String, Type, Span)> {
        f.params
            .iter()
            .filter(|p| p.comptime)
            .zip(args)
            .map(|(p, t)| (p.name.clone(), t.clone(), p.span))
            .collect()
    }

    /// Does `t` mention a type parameter?
    fn has_param(&self, t: &Type) -> bool {
        match t {
            Type::Param(..) => true,
            Type::Struct(n) => self.abstract_structs.contains(n),
            Type::Array(e, _)
            | Type::Slice(e, _)
            | Type::CPtr(e)
            | Type::Ptr(e, _)
            | Type::Optional(e)
            | Type::ErrorUnion(_, e) => self.has_param(e),
            Type::Fn(params, ret) => {
                params.iter().any(|p| self.has_param(p)) || self.has_param(ret)
            }
            _ => false,
        }
    }

    /// How deeply `t` nests generic instances and other types: 1 for a
    /// type without parts.
    fn type_depth(&self, t: &Type) -> usize {
        match t {
            Type::Array(e, _)
            | Type::Slice(e, _)
            | Type::CPtr(e)
            | Type::Ptr(e, _)
            | Type::Optional(e)
            | Type::ErrorUnion(_, e) => 1 + self.type_depth(e),
            Type::Fn(params, ret) => {
                1 + params
                    .iter()
                    .chain([&**ret])
                    .map(|t| self.type_depth(t))
                    .max()
                    .unwrap_or(0)
            }
            Type::Struct(n) => self.instance_depth.get(n).copied().unwrap_or(1),
            _ => 1,
        }
    }

    /// The depth of a new instance of `g` for `args`. Stop a generic that
    /// instantiates itself with larger and larger type arguments (`Nest(?T)`
    /// in `Nest(T)`), which would never end.
    fn instance_depth(&self, g: &str, args: &[Type], span: Span) -> Result<usize> {
        let depth = 1 + args.iter().map(|t| self.type_depth(t)).max().unwrap_or(0);
        if depth > MAX_INSTANCE_DEPTH {
            if self.failed.contains(g) {
                return Err(silent());
            }
            return err(
                E0080,
                span,
                format!(
                    "`{g}` instantiates itself with larger and larger type arguments (instances nested more than {MAX_INSTANCE_DEPTH} deep), which would never end"
                ),
            );
        }
        Ok(depth)
    }

    /// The name of a new instance: `Pool(Node)`. Instances for type
    /// parameters of different generics can print alike (`Pool(T)`); a
    /// later one gets a suffix.
    fn instance_name(&self, g: &str, args: &[Type]) -> String {
        let base = format!(
            "{g}({})",
            args.iter()
                .map(|a| a.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
        let mut name = base.clone();
        let mut n = 1;
        while self.instance_names.contains(&name) {
            n += 1;
            name = format!("{base}#{n}");
        }
        name
    }

    /// Check the `where` constraints of the generic `g` for the type
    /// arguments `args`, at the instantiation `span`. Before every method
    /// signature is known, the check waits.
    fn check_bounds(&mut self, g: &str, args: &[Type], span: Span) -> Result<()> {
        if !self.sigs_done {
            self.pending_bounds
                .push((g.to_string(), args.to_vec(), span));
            return Ok(());
        }
        let decl = self.generics[g];
        let params: Vec<&ast::Param> = decl.params.iter().filter(|p| p.comptime).collect();
        for b in &decl.bounds {
            let Some(i) = params.iter().position(|p| p.name == b.param) else {
                continue;
            };
            let integer = b.iface == INTEGER;
            let hash = b.iface == HASH;
            if i >= args.len() || !integer && !hash && !self.ifaces.contains_key(&b.iface) {
                continue;
            }
            let (p, iface) = (&b.param, &b.iface);
            let need = format!(
                "`where {p}: {iface}` of `{g}` at {}",
                self.place(b.span, span)
            );
            match &args[i] {
                t if integer && t.is_int() => {}
                t @ Type::Param(..) if hash && self.hash_param(t) => {}
                Type::Param(pg, pn) => {
                    if !self
                        .bounds
                        .get(&(pg.clone(), pn.clone()))
                        .is_some_and(|v| v.contains(iface))
                    {
                        return err(
                            E0079,
                            span,
                            format!("`{pn}` does not satisfy {need} (add `where {pn}: {iface}`)"),
                        );
                    }
                }
                t if integer => {
                    return err(
                        E0079,
                        span,
                        format!("`{t}` does not satisfy {need}: it is not an integer type"),
                    );
                }
                t if hash => {
                    if let Some(why) = self.why_not_comparable(t) {
                        let why = if why.is_empty() {
                            "`==` cannot compare it".to_string()
                        } else {
                            why
                        };
                        return err(E0079, span, format!("`{t}` does not satisfy {need}: {why}"));
                    }
                }
                Type::Struct(s) => {
                    if let Some(why) = self.why_not(s, iface) {
                        return err(E0079, span, format!("`{s}` does not satisfy {need}: {why}"));
                    }
                }
                t => {
                    return err(
                        E0079,
                        span,
                        format!("`{t}` does not satisfy {need}: it has no methods"),
                    );
                }
            }
        }
        Ok(())
    }

    /// The struct that the type function `g` returns for `args` (memoized:
    /// the same arguments give the same struct). The first time, its fields
    /// and method signatures are resolved with the `comptime` parameters
    /// bound to `args`, and its methods are queued for checking unless
    /// `args` are abstract (then only `g`'s own parameters are checked).
    fn instantiate_type(&mut self, g: &str, args: Vec<Type>, span: Span) -> Result<Type> {
        self.define_generic(g);
        let decl = self.generics[g];
        let want = decl.params.iter().filter(|p| p.comptime).count();
        if args.len() != want {
            return err(
                E0080,
                span,
                format!(
                    "type function `{g}` takes {want} argument(s) but {} were given",
                    args.len()
                ),
            );
        }
        // The definition is reported as wrong.
        let Some(sdecl) = type_fn_struct(decl) else {
            return Err(silent());
        };
        self.check_bounds(g, &args, span)?;
        let key = (g.to_string(), args);
        if let Some(n) = self.instances.get(&key) {
            return Ok(Type::Struct(n.clone()));
        }
        let depth = self.instance_depth(g, &key.1, span)?;
        let name = self.instance_name(g, &key.1);
        self.instance_names.insert(name.clone());
        if !key.1.iter().any(|t| self.has_param(t)) {
            self.instance_sites.insert(name.clone(), span);
        }
        self.instances.insert(key.clone(), name.clone());
        self.instance_structs.push(name.clone());
        self.instance_depth.insert(name.clone(), depth);
        self.structs.insert(
            name.clone(),
            StructDef {
                name: name.clone(),
                fields: vec![],
                is_union: false,
            },
        );
        let identity = key.1 == self.own_params(decl);
        let is_abstract = key.1.iter().any(|t| self.has_param(t));
        if is_abstract {
            self.abstract_structs.insert(name.clone());
        }
        let env = Self::comptime_env(decl, &key.1);
        let saved = (
            std::mem::replace(&mut self.env, env),
            self.self_ty.replace(name.clone()),
            self.quiet,
        );
        if !identity && self.failed.contains(g) {
            self.quiet = true;
        }
        let fields = self.fields(&sdecl.fields, &format!("struct `{name}`"));
        self.structs.get_mut(&name).unwrap().fields = fields;
        let exprs = default_exprs(&sdecl.fields);
        if exprs.iter().any(Option::is_some) {
            let decls = DefaultDecls {
                exprs,
                env: self.env.clone(),
                module: module_of(g).to_string(),
            };
            self.default_decls.insert(name.clone(), decls);
        }
        let mut keys: Vec<String> = Vec::new();
        for m in &sdecl.methods {
            if sdecl.fields.iter().any(|f| f.name == m.name) {
                self.report(Error::new(
                    E0015,
                    m.span,
                    format!(
                        "`{}` is both a field and a method of struct `{name}`",
                        m.name
                    ),
                ));
                continue;
            }
            let key = format!("{name}.{}", m.name);
            if keys.contains(&key) {
                self.report(Error::new(
                    E0015,
                    m.span,
                    format!("duplicate method `{}` in struct `{name}`", m.name),
                ));
                continue;
            }
            if let Some(p) = m.params.iter().find(|p| p.comptime) {
                self.report(Error::new(
                    E0076,
                    p.span,
                    "only a top-level function can have `comptime` parameters",
                ));
            }
            self.register(&key, m, Some(name.clone()), Some(g.to_string()));
            keys.push(key);
        }
        (self.env, self.self_ty, self.quiet) = saved;
        if identity || !is_abstract {
            self.queue.extend(keys);
        }
        if self.sigs_done {
            if let Err(e) = self.check_recursive(&name, &name, sdecl.span, &mut Vec::new()) {
                self.report(e);
            }
        } else {
            self.pending_recursive.push((name.clone(), sdecl.span));
        }
        Ok(Type::Struct(name))
    }

    /// The function key of the generic function `g` for the type arguments
    /// `args` (memoized), registered and queued like `instantiate_type`.
    fn instantiate_fn(&mut self, g: &str, args: Vec<Type>, span: Span) -> Result<String> {
        self.define_generic(g);
        self.check_bounds(g, &args, span)?;
        let key = (g.to_string(), args);
        if let Some(k) = self.instances.get(&key) {
            return Ok(k.clone());
        }
        let decl = self.generics[g];
        self.instance_depth(g, &key.1, span)?;
        let name = self.instance_name(g, &key.1);
        self.instance_names.insert(name.clone());
        if !key.1.iter().any(|t| self.has_param(t)) {
            self.instance_sites.insert(name.clone(), span);
        }
        self.instances.insert(key.clone(), name.clone());
        let identity = key.1 == self.own_params(decl);
        let is_abstract = key.1.iter().any(|t| self.has_param(t));
        let env = Self::comptime_env(decl, &key.1);
        let saved = (std::mem::replace(&mut self.env, env), self.quiet);
        if !identity && self.failed.contains(g) {
            self.quiet = true;
        }
        self.register(&name, decl, None, Some(g.to_string()));
        (self.env, self.quiet) = saved;
        if identity || !is_abstract {
            self.queue.push(name.clone());
        }
        Ok(name)
    }

    /// Resolve the signature of `f` and register it as the function `key`
    /// (`comptime` parameters are left out), with its context for checking
    /// the body. `owner` is the struct of a method; `generic` the generic
    /// it belongs to, which must spell out its error set.
    fn register(
        &mut self,
        key: &str,
        f: &'a ast::FnDecl,
        owner: Option<String>,
        generic: Option<String>,
    ) -> FnSig {
        let saved = std::mem::replace(&mut self.self_ty, owner.clone());
        let params: Vec<Type> = f
            .params
            .iter()
            .filter(|p| !p.comptime)
            .map(|p| self.resolve_or_report(&p.ty))
            .collect();
        let ret = match &f.ret {
            Some(
                t @ ast::TypeExpr {
                    kind: TypeExprKind::ErrorUnion(None, payload),
                    ..
                },
            ) if generic.is_some() => {
                let payload = match &payload.kind {
                    TypeExprKind::Named(n) if n == "void" => Type::Void,
                    _ => self.resolve_or_report(payload),
                };
                self.report(Error::new(E0076, t.span, format!(
                    "a generic function must spell out its error set: write `ErrorSet!{payload}`, not `!{payload}`"
                )));
                Type::Error
            }
            Some(t) => self.resolve_ret(key, f.is_pub, t),
            None => Type::Void,
        };
        self.self_ty = saved;
        if let Some(s) = &owner {
            let is_owner = |t: &Type| matches!(t, Type::Struct(n) | Type::Enum(n) if n == s);
            let recv = match params.first() {
                Some(t) if is_owner(t) => Recv::Value,
                Some(Type::Ptr(t, m)) if is_owner(t) => Recv::Ptr(*m),
                _ => Recv::Static,
            };
            self.recvs.insert(key.to_string(), recv);
        }
        let sig = FnSig {
            params,
            ret,
            is_extern: f.is_extern,
            link: f.link.clone(),
            variadic: f.variadic.is_some(),
        };
        self.fns.insert(key.to_string(), sig.clone());
        if f.body.is_some() {
            self.fn_decls.insert(
                key.to_string(),
                FnCtx {
                    decl: f,
                    owner,
                    env: self.env.clone(),
                    generic,
                },
            );
        }
        sig
    }

    /// The method signatures of an interface. The first parameter of each
    /// must be `self: *Self` or `self: *mut Self`.
    fn iface_methods(&mut self, i: &ast::InterfaceDecl) -> Vec<IfaceMethod> {
        let mut methods: Vec<IfaceMethod> = Vec::new();
        for m in &i.methods {
            if methods.iter().any(|x| x.name == m.name) {
                self.report(Error::new(
                    E0015,
                    m.span,
                    format!("duplicate method `{}` in interface `{}`", m.name, i.name),
                ));
                continue;
            }
            let mutable = match m.params.first().map(|p| &p.ty.kind) {
                Some(TypeExprKind::Ptr(t, mutable)) if matches!(&t.kind, TypeExprKind::Named(n) if n == "Self") => {
                    *mutable
                }
                _ => {
                    self.report(Error::new(E0073, m.span, format!(
                        "interface method `{}.{}` must take `self: *Self` or `self: *mut Self` as its first parameter",
                        i.name, m.name
                    )));
                    continue;
                }
            };
            if let Some(p) = m.params.iter().find(|p| p.comptime) {
                self.report(Error::new(
                    E0076,
                    p.span,
                    "only a top-level function can have `comptime` parameters",
                ));
                continue;
            }
            let params = m.params[1..]
                .iter()
                .map(|p| self.resolve_or_report(&p.ty))
                .collect();
            let ret = match &m.ret {
                None => Type::Void,
                Some(
                    t @ ast::TypeExpr {
                        kind: TypeExprKind::ErrorUnion(None, _),
                        ..
                    },
                ) => {
                    self.report(Error::new(
                        E0065,
                        t.span,
                        format!(
                            "interface method `{}.{}` must spell out its error set (`E!T`)",
                            i.name, m.name
                        ),
                    ));
                    Type::Error
                }
                Some(t) => self.resolve_ret(&m.name, false, t),
            };
            methods.push(IfaceMethod {
                name: m.name.clone(),
                mutable,
                params,
                ret,
            });
        }
        methods
    }

    /// The errors of every error set. Each new error name gets the next
    /// error value; a merged set (`A || B`) has every error of its parts.
    fn error_sets(&mut self, items: &[&'a ast::Item]) {
        let mut merges = HashMap::new();
        for item in items {
            let ast::Item::ErrorSet(e) = item else {
                continue;
            };
            let ast::ErrorSetKind::List(names) = &e.kind else {
                merges.insert(e.name.as_str(), e);
                continue;
            };
            if names.is_empty() {
                self.report(Error::new(
                    E0054,
                    e.span,
                    format!("error set `{}` has no errors", e.name),
                ));
            }
            let mut errors: Vec<String> = Vec::new();
            for (n, span) in names {
                if errors.contains(n) {
                    self.report(Error::new(
                        E0015,
                        *span,
                        format!("duplicate error `{n}` in error set `{}`", e.name),
                    ));
                    continue;
                }
                if !self.error_names.contains(n) {
                    self.error_names.push(n.clone());
                }
                errors.push(n.clone());
            }
            self.sets.insert(e.name.clone(), errors);
        }
        let mut done = Vec::new();
        for item in items {
            if let ast::Item::ErrorSet(e) = item
                && merges.contains_key(e.name.as_str())
            {
                self.merge_set(&e.name, &merges, &mut done, &mut Vec::new());
            }
        }
    }

    fn merge_set(
        &mut self,
        name: &str,
        merges: &HashMap<&str, &ast::ErrorSetDecl>,
        done: &mut Vec<String>,
        active: &mut Vec<String>,
    ) {
        if done.iter().any(|d| d == name) {
            return;
        }
        let decl = merges[name];
        if active.iter().any(|a| a == name) {
            self.report(Error::new(
                E0070,
                decl.span,
                format!("error set `{name}` is defined in terms of itself"),
            ));
            return;
        }
        active.push(name.to_string());
        let ast::ErrorSetKind::Merge(parts) = &decl.kind else {
            unreachable!()
        };
        let mut errors: Vec<String> = Vec::new();
        for (part, span) in parts {
            if merges.contains_key(part.as_str()) {
                self.merge_set(part, merges, done, active);
            }
            match self.set_name(part, *span) {
                Ok(set) => {
                    for n in &self.sets[&set] {
                        if !errors.contains(n) {
                            errors.push(n.clone());
                        }
                    }
                }
                Err(e) => self.report(e),
            }
        }
        active.pop();
        done.push(name.to_string());
        self.sets.insert(name.to_string(), errors);
    }

    /// `name` used as an error set.
    fn set_name(&self, name: &str, span: Span) -> Result<String> {
        if self.sets.contains_key(name) {
            Ok(name.to_string())
        } else if self.top_spans.contains_key(name) || Type::from_name(name, self.layout).is_some()
        {
            err(E0070, span, format!("`{name}` is not an error set"))
        } else {
            err(E0017, span, format!("unknown error set `{name}`"))
        }
    }

    /// The return type of `f`. Only here can a type be an error union, and
    /// only there can its payload be `void`. `!T` gets a new, empty inferred
    /// set that its body fills in.
    fn resolve_ret(&mut self, name: &str, is_pub: bool, t: &ast::TypeExpr) -> Type {
        let TypeExprKind::ErrorUnion(set, payload) = &t.kind else {
            return self.resolve_or_report(t);
        };
        let payload = match &payload.kind {
            TypeExprKind::Named(n) if n == "void" => Type::Void,
            _ => self.resolve_or_report(payload),
        };
        let set = match set {
            Some((name, span)) => match self.set_name(name, *span) {
                Ok(set) => set,
                Err(e) => {
                    self.report(e);
                    return Type::Error;
                }
            },
            None => {
                if is_pub && name != "main" {
                    self.report(Error::new(
                        E0065,
                        t.span,
                        format!("a `pub` function must spell out its error set: write `ErrorSet!{payload}`, not `!{payload}`"),
                    ));
                }
                let set = format!("{INFERRED}{name}");
                self.sets.insert(set.clone(), vec![]);
                set
            }
        };
        if payload == Type::Error {
            return Type::Error;
        }
        Type::ErrorUnion(set, Box::new(payload))
    }

    /// Fields of a struct or enum variant; `owner` names it in messages.
    fn fields(&mut self, decls: &[ast::FieldDecl], owner: &str) -> Vec<(String, Type)> {
        let mut fields: Vec<(String, Type)> = Vec::new();
        for f in decls {
            if fields.iter().any(|(n, _)| *n == f.name) {
                self.report(Error::new(
                    E0015,
                    f.span,
                    format!("duplicate field `{}` in {owner}", f.name),
                ));
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
            None => self.enums[name]
                .variants
                .iter()
                .flat_map(|v| v.fields.iter().map(|(_, t)| t))
                .collect(),
        }
    }

    fn check_recursive(
        &self,
        root: &str,
        name: &str,
        span: Span,
        path: &mut Vec<String>,
    ) -> Result<()> {
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
                    let kind = if self.structs.contains_key(root) {
                        "struct"
                    } else {
                        "enum"
                    };
                    return err(
                        E0053,
                        span,
                        format!("{kind} `{root}` contains itself by value (infinite size)"),
                    );
                }
                self.check_recursive(root, n, span, path)?;
            }
        }
        Ok(())
    }

    fn resolve_type(&mut self, t: &ast::TypeExpr) -> Result<Type> {
        Ok(match &t.kind {
            TypeExprKind::Named(n) => match Type::from_name(n, self.layout) {
                Some(ty) => ty,
                None if let Some((_, ty, _)) = self.env.iter().find(|(p, ..)| p == n) => ty.clone(),
                None if self.alias_decls.contains_key(n) => self.alias(n, t.span)?,
                None if self.structs.contains_key(n) => Type::Struct(n.clone()),
                None if self.enums.contains_key(n) => Type::Enum(n.clone()),
                None if self.sets.contains_key(n) => Type::ErrorSet(n.clone()),
                None if n == "Self" => match &self.self_ty {
                    Some(s) => self.named(s),
                    None => {
                        return err(
                            E0017,
                            t.span,
                            "`Self` can only be used inside a struct or enum (and as `*Self` or `*mut Self` in an interface method's `self` parameter)",
                        );
                    }
                },
                None if self.ifaces.contains_key(n) => {
                    return err(
                        E0017,
                        t.span,
                        format!("`{n}` is an interface, not a type: write `dyn {n}`"),
                    );
                }
                None if self.generics.get(n).is_some_and(|f| is_type_fn(f)) => {
                    return err(
                        E0080,
                        t.span,
                        format!("`{n}` is a type function: write `{n}(T)`"),
                    );
                }
                None if n == "type" => {
                    return err(
                        E0076,
                        t.span,
                        "`type` is only the type of a `comptime` parameter, or the return type of a type function",
                    );
                }
                None => {
                    let e = Error::new(E0017, t.span, format!("unknown type `{n}`"));
                    return Err(suggest(e, n, t.span, self.type_names()));
                }
            },
            TypeExprKind::Dyn(n) => {
                if !self.ifaces.contains_key(n) {
                    let what = if self.top_spans.contains_key(n)
                        || Type::from_name(n, self.layout).is_some()
                    {
                        "is not an interface"
                    } else {
                        "is not defined"
                    };
                    return err(E0017, t.span, format!("`dyn {n}`: `{n}` {what}"));
                }
                Type::Dyn(n.clone())
            }
            TypeExprKind::Generic(g, args) => {
                if !self.generics.get(g).is_some_and(|f| is_type_fn(f)) {
                    if self.generics.contains_key(g)
                        || self.top_spans.contains_key(g)
                        || Type::from_name(g, self.layout).is_some()
                    {
                        return err(
                            E0080,
                            t.span,
                            format!(
                                "`{g}` is not a type function (a function that returns `type`)"
                            ),
                        );
                    }
                    return err(E0017, t.span, format!("unknown type `{g}`"));
                }
                let args = args
                    .iter()
                    .map(|a| self.resolve_type(a))
                    .collect::<Result<Vec<_>>>()?;
                self.instantiate_type(g, args, t.span)?
            }
            TypeExprKind::Array(len, elem) => {
                let n = match len {
                    ast::ArrayLen::Lit(n) => *n,
                    ast::ArrayLen::Const(e) => self.array_len(e)?,
                };
                Type::Array(Box::new(self.resolve_type(elem)?), n)
            }
            TypeExprKind::Simd(len, elem) => {
                let n = match len {
                    ast::ArrayLen::Lit(n) => *n,
                    ast::ArrayLen::Const(e) => self.array_len(e)?,
                };
                let elem = self.resolve_type(elem)?;
                if !(elem.is_numeric() || elem == Type::Bool) {
                    return err(
                        E0109,
                        t.span,
                        format!("`simd[{n}]{elem}`: the lanes of a `simd` are numbers or `bool`"),
                    );
                }
                if !n.is_power_of_two() || !(2..=64).contains(&n) {
                    return err(
                        E0109,
                        t.span,
                        format!("`simd[{n}]{elem}`: a `simd` has 2, 4, 8, 16, 32 or 64 lanes"),
                    );
                }
                Type::Simd(Box::new(elem), n)
            }
            TypeExprKind::Slice(elem, mutable) => {
                Type::Slice(Box::new(self.resolve_type(elem)?), *mutable)
            }
            TypeExprKind::CPtr(elem) => Type::CPtr(Box::new(self.resolve_type(elem)?)),
            TypeExprKind::Ptr(elem, mutable) => {
                Type::Ptr(Box::new(self.resolve_type(elem)?), *mutable)
            }
            TypeExprKind::Optional(elem) => Type::Optional(Box::new(self.resolve_type(elem)?)),
            TypeExprKind::ErrorUnion(..) => {
                return err(
                    E0066,
                    t.span,
                    "error union types can only be used as a function's return type",
                );
            }
            TypeExprKind::Fn(params, ret) => {
                let params = params
                    .iter()
                    .map(|p| self.resolve_type(p))
                    .collect::<Result<Vec<_>>>()?;
                let ret = match ret.as_deref() {
                    None => Type::Void,
                    // A function type can name an error set, as a function does.
                    Some(ast::TypeExpr {
                        kind: TypeExprKind::ErrorUnion(Some((set, sp)), payload),
                        ..
                    }) => {
                        let set = self.set_name(set, *sp)?;
                        let payload = match &payload.kind {
                            TypeExprKind::Named(n) if n == "void" => Type::Void,
                            _ => self.resolve_type(payload)?,
                        };
                        Type::ErrorUnion(set, Box::new(payload))
                    }
                    Some(
                        r @ ast::TypeExpr {
                            kind: TypeExprKind::ErrorUnion(None, _),
                            ..
                        },
                    ) => {
                        return err(
                            E0065,
                            r.span,
                            "a function type must spell out its error set (`E!T`)",
                        );
                    }
                    Some(r) => self.resolve_type(r)?,
                };
                Type::Fn(params, Box::new(ret))
            }
        })
    }

    /// The value of the constant expression `e`, an array length: an
    /// integer that the program computes when it is compiled.
    fn array_len(&mut self, e: &ast::Expr) -> Result<u64> {
        let t = self.value(e, None)?;
        if !t.ty.is_int() {
            return err(
                E0099,
                e.span,
                format!("an array length must be an integer, found `{}`", t.ty),
            );
        }
        let not_constant = || {
            Error::new(
                E0099,
                e.span,
                "an array length must be a constant integer: a literal, a `let` constant, or arithmetic on them",
            )
        };
        match self.fold(&t) {
            Ok(ConstVal::Int(n)) if n < 0 => err(
                E0099,
                e.span,
                format!("an array length cannot be negative, found `{n}`"),
            ),
            Ok(ConstVal::Int(n)) => Ok(n as u64),
            Ok(_) => Err(not_constant()),
            Err(x) if x.code == E0089 && x.msg.contains("global's initializer") => {
                Err(not_constant())
            }
            Err(x) => Err(x),
        }
    }

    /// The struct or enum `name`.
    fn named(&self, name: &str) -> Type {
        if self.enums.contains_key(name) {
            Type::Enum(name.to_string())
        } else {
            Type::Struct(name.to_string())
        }
    }

    /// The enum that the name `n` stands for: the enum, `Self` in a method
    /// of an enum, or an alias of an enum.
    fn enum_name(&self, n: &str) -> Option<String> {
        let n = if n == "Self" {
            self.self_ty.as_deref()?
        } else {
            n
        };
        match self.aliases.get(n) {
            Some(Type::Enum(e)) => Some(e.clone()),
            _ => self.enums.contains_key(n).then(|| n.to_string()),
        }
    }

    /// Names that can be written as a type here, for spelling suggestions.
    fn type_names(&self) -> Vec<&str> {
        let named = self
            .structs
            .keys()
            .chain(self.enums.keys())
            .chain(self.sets.keys())
            .chain(self.alias_decls.keys());
        let named = named.filter(|n| !n.contains(['(', ' ']));
        TYPE_NAMES
            .iter()
            .copied()
            .chain(named.map(String::as_str))
            .chain(self.env.iter().map(|(n, ..)| n.as_str()))
            .collect()
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
        let prev = self
            .scopes
            .iter()
            .find_map(|s| s.get(name).map(|&(_, sp)| sp));
        let prev = prev.or_else(|| {
            self.env
                .iter()
                .find(|(n, ..)| n == name)
                .map(|&(_, _, sp)| sp)
        });
        // Top-level names of other modules are checked by module resolution.
        let top = if self.module.is_empty() {
            self.top_spans.get(name).copied()
        } else {
            None
        };
        if let Some(prev) = prev.or(top) {
            return err(
                E0016,
                span,
                format!(
                    "`{name}` is already declared at {} (shadowing is not allowed)",
                    self.place(prev, span)
                ),
            );
        }
        let id = self.vars.len();
        self.scopes
            .last_mut()
            .unwrap()
            .insert(name.to_string(), (id, span));
        self.vars.push(VarInfo {
            name: name.to_string(),
            ty,
            mutable,
            let_span: None,
        });
        Ok(id)
    }

    /// `declare`, reporting a failure; `None` if it failed. After a failure
    /// the name still refers to an error-typed variable here, so its uses
    /// are not reported again.
    fn declare_or_report(
        &mut self,
        name: &str,
        ty: Type,
        mutable: bool,
        span: Span,
    ) -> Option<VarId> {
        match self.declare(name, ty, mutable, span) {
            Ok(id) => Some(id),
            Err(e) => {
                self.report(e);
                let id = self.vars.len();
                self.scopes
                    .last_mut()
                    .unwrap()
                    .insert(name.to_string(), (id, span));
                self.vars.push(VarInfo {
                    name: name.to_string(),
                    ty: Type::Error,
                    mutable,
                    let_span: None,
                });
                None
            }
        }
    }

    fn lookup(&self, name: &str) -> Option<VarId> {
        self.scopes
            .iter()
            .rev()
            .find_map(|s| s.get(name).map(|&(id, _)| id))
    }

    /// Check the body of `name` if that is not done yet. A function with an
    /// inferred error set is checked before its callers, which need its set;
    /// `span` is the call that needs it.
    fn ensure_body(&mut self, name: &str, span: Span) -> Result<()> {
        match self.bodies.get(name) {
            Some(Some(_)) => return Ok(()),
            Some(None) => {
                return err(
                    E0069,
                    span,
                    format!(
                        "cannot infer the error set of `{name}`: it calls itself, directly or through other functions (write the error set, `E!T`)"
                    ),
                );
            }
            None => {}
        }
        self.bodies.insert(name.to_string(), None);
        // The instances of a generic with errors are checked quietly.
        let quiet = !self.is_abstract_fn(name)
            && self.fn_decls[name]
                .generic
                .as_ref()
                .is_some_and(|g| self.failed.contains(g))
            || self.unsatisfied.iter().any(|n| {
                name.strip_prefix(n.as_str())
                    .is_some_and(|m| m.starts_with('.'))
            });
        let saved_quiet = std::mem::replace(&mut self.quiet, quiet);
        let saved = (
            std::mem::take(&mut self.vars),
            std::mem::take(&mut self.scopes),
            self.ret.clone(),
            self.loop_depth,
            self.in_defer,
            self.failures,
            self.self_ty.take(),
            self.in_test,
            std::mem::take(&mut self.env),
            std::mem::take(&mut self.module),
        );
        let def = self.fn_body(name);
        // A generic's own instance with errors: the generic has errors.
        if self.failures > saved.5
            && self.is_abstract_fn(name)
            && let Some(g) = self.fn_decls[name].generic.clone()
        {
            self.failed.insert(g);
        }
        (
            self.vars,
            self.scopes,
            self.ret,
            self.loop_depth,
            self.in_defer,
            self.failures,
            self.self_ty,
            self.in_test,
            self.env,
            self.module,
        ) = saved;
        self.quiet = saved_quiet;
        self.bodies.insert(name.to_string(), Some(def));
        Ok(())
    }

    fn fn_body(&mut self, name: &str) -> FnDef {
        let ctx = self.fn_decls[name].clone();
        self.module = ctx.module(name);
        let FnCtx {
            decl: f,
            owner,
            env,
            ..
        } = ctx;
        let sig = self.fns[name].clone();
        self.self_ty = owner;
        self.env = env;
        self.in_test = is_test(name);
        let owner_site = self
            .self_ty
            .as_ref()
            .and_then(|s| self.instance_sites.get(s).map(|&at| (s.clone(), at)));
        self.in_instance = self
            .instance_sites
            .get(name)
            .map(|&at| (name.to_string(), at))
            .or(owner_site);
        self.vars.clear();
        self.scopes = vec![HashMap::new()];
        self.ret = sig.ret.clone();
        self.loop_depth = 0;
        self.in_defer = false;
        let failures = self.failures;
        let mut params = Vec::new();
        for (p, ty) in f.params.iter().filter(|p| !p.comptime).zip(&sig.params) {
            params.extend(self.declare_or_report(&p.name, ty.clone(), false, p.span));
        }
        let body = self.block(f.body.as_ref().unwrap());
        // A statement with an error may have been the `return`.
        if self.failures == failures && needs_value(&sig.ret) && !block_returns(&body, false) {
            self.report(Error::new(
                E0050,
                f.span,
                format!(
                    "function `{name}` must return a value of type `{}` on every path",
                    sig.ret
                ),
            ));
        }
        self.in_instance = None;
        FnDef {
            name: name.to_string(),
            params,
            ret: sig.ret,
            vars: std::mem::take(&mut self.vars),
            body,
            is_main: name == "main",
            export: f.is_extern,
        }
    }

    /// Statements with errors are reported and left out.
    fn block(&mut self, b: &ast::Block) -> Vec<TStmt> {
        self.block_with(b, None)
    }

    /// The block of a `catch` or `orelse` whose value is a `want`: its last
    /// statement may be an expression that gives the value
    /// (`TStmtKind::Value`). A `void` expression there (a call), a `match`
    /// with block arms and `unreachable` stay statements.
    fn value_block(&mut self, b: &ast::Block, want: &Type) -> Vec<TStmt> {
        self.block_with(b, Some(want))
    }

    fn block_with(&mut self, b: &ast::Block, value: Option<&Type>) -> Vec<TStmt> {
        self.scopes.push(HashMap::new());
        let mut out = Vec::new();
        for (i, s) in b.stmts.iter().enumerate() {
            if i == 0
                && let StmtKind::Expr(ast::Expr {
                    kind: ExprKind::Builtin { name, args },
                    span,
                }) = &s.kind
                && name == "setSafety"
            {
                match args.as_slice() {
                    [
                        ast::Expr {
                            kind: ExprKind::Bool(on),
                            ..
                        },
                    ] => out.push(TStmt {
                        kind: TStmtKind::SetSafety(*on),
                        span: s.span,
                    }),
                    _ => self.report(Error::new(
                        E0108,
                        *span,
                        "`@setSafety` takes `true` or `false`, written out",
                    )),
                }
                continue;
            }
            if let (Some(want), StmtKind::Expr(e), true) = (value, &s.kind, i + 1 == b.stmts.len())
                && !matches!(&strip_parens(e).kind, ExprKind::Unreachable)
                && !matches!(&strip_parens(e).kind, ExprKind::Match { arms, .. } if match_is_stmt(arms))
            {
                match self.tail_value(e, want) {
                    Ok(kind) => out.push(TStmt { kind, span: s.span }),
                    Err(e) => self.report(e),
                }
                continue;
            }
            match self.stmt(s) {
                Ok(t) => out.push(t),
                Err(e) => self.report(e),
            }
        }
        self.scopes.pop();
        out
    }

    /// The last expression `e` of a value block (see `value_block`).
    fn tail_value(&mut self, e: &ast::Expr, want: &Type) -> Result<TStmtKind> {
        let t = self.expr(e, Some(Self::hint(e, want)))?;
        match t.ty {
            Type::Error => Err(silent()),
            Type::Void => Ok(TStmtKind::Expr(t)),
            _ => {
                let t = self.handled(t)?;
                Ok(TStmtKind::Value(self.coerce(t, want)?))
            }
        }
    }

    fn stmt(&mut self, s: &ast::Stmt) -> Result<TStmt> {
        let kind = match &s.kind {
            StmtKind::Let {
                name,
                mutable,
                ty,
                init,
            } => {
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
                if let (Some(id), false) = (id, *mutable) {
                    self.vars[id].let_span = Some(s.span);
                }
                let init = init?;
                TStmtKind::Let(id.ok_or_else(silent)?, init)
            }
            // `_ = x`: evaluate `x` and drop its value (not an error union).
            StmtKind::Assign {
                target:
                    ast::Expr {
                        kind: ExprKind::Ident(n),
                        ..
                    },
                op: AssignOp::Set,
                value,
            } if n == "_" && self.lookup(n).is_none() => TStmtKind::Expr(self.value(value, None)?),
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
            StmtKind::If {
                cond,
                capture,
                then,
                els,
                err_capture: Some((ename, espan)),
            } => {
                let u = self.expr(cond, None).and_then(|u| match &u.ty {
                    Type::ErrorUnion(..) => Ok(u),
                    Type::Error => Err(silent()),
                    t => err(E0102, u.span, format!(
                        "`else |{ename}|` takes the error of an error union (`E!T`), but the condition has type `{t}`"
                    )),
                });
                let (set, payload) = match &u {
                    Ok(TExpr {
                        ty: Type::ErrorUnion(set, p),
                        ..
                    }) => (set.clone(), (**p).clone()),
                    _ => (String::new(), Type::Error),
                };
                self.scopes.push(HashMap::new());
                // `Err` when the capture has an error (reported).
                let value: Result<Option<VarId>> = match capture {
                    Some((n, sp)) if payload == Type::Void => {
                        self.report(Error::new(E0102, *sp, format!(
                            "the condition has type `{}`: there is no value to capture (write `if f() {{ }} else |{ename}| {{ }}`)",
                            u.as_ref().map_or(Type::Error, |u| u.ty.clone())
                        )));
                        self.declare_or_report(n, Type::Error, false, *sp);
                        Err(silent())
                    }
                    Some((n, sp)) => self
                        .declare_or_report(n, payload.clone(), false, *sp)
                        .map(Some)
                        .ok_or_else(silent),
                    None => Ok(None),
                };
                let then = self.block(then);
                self.scopes.pop();
                self.scopes.push(HashMap::new());
                let ety = if set.is_empty() {
                    Type::Error
                } else {
                    Type::ErrorSet(set)
                };
                let err_var = self.declare_or_report(ename, ety, false, *espan);
                let els = match els.as_deref() {
                    Some(ast::Stmt {
                        kind: StmtKind::Block(b),
                        ..
                    }) => self.block(b),
                    _ => unreachable!(),
                };
                self.scopes.pop();
                let (cond, value, err) = (u?, value?, err_var.ok_or_else(silent)?);
                TStmtKind::IfErr {
                    cond,
                    value,
                    then,
                    err,
                    els,
                }
            }
            StmtKind::If {
                cond,
                capture,
                then,
                els,
                err_capture: None,
            } => {
                let (cond, capture, then) = match capture {
                    None => (self.condition(cond, "if"), None, self.block(then)),
                    Some((name, name_span)) => {
                        let (c, inner) = self.optional_cond(cond, "if", name);
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
                        _ => self.stmt(e).map_or_else(
                            |e| {
                                self.report(e);
                                vec![]
                            },
                            |s| vec![s],
                        ),
                    },
                    None => vec![],
                };
                let cond = cond?;
                let capture = match capture {
                    Some(v) => Some(v.ok_or_else(silent)?),
                    None => None,
                };
                TStmtKind::If {
                    cond,
                    capture,
                    then,
                    els,
                }
            }
            StmtKind::While {
                cond,
                capture: None,
                body,
            } => {
                let cond = self.condition(cond, "while");
                self.loop_depth += 1;
                let body = self.block(body);
                self.loop_depth -= 1;
                TStmtKind::While {
                    cond: cond?,
                    capture: None,
                    body,
                }
            }
            StmtKind::While {
                cond,
                capture: Some((name, name_span)),
                body,
            } => {
                let (cond, inner) = self.optional_cond(cond, "while", name);
                self.scopes.push(HashMap::new());
                let v = self.declare_or_report(name, inner, false, *name_span);
                self.loop_depth += 1;
                let body = self.block(body);
                self.loop_depth -= 1;
                self.scopes.pop();
                TStmtKind::While {
                    cond: cond?,
                    capture: Some(v.ok_or_else(silent)?),
                    body,
                }
            }
            StmtKind::ForEach {
                index,
                elem: (elem, elem_span),
                seq,
                body,
            } => {
                let seq = self.value(seq, None).and_then(|t| match &t.ty {
                    Type::Array(..) | Type::Slice(..) => Ok(t),
                    Type::Int(_) => err(E0090, t.span, format!(
                        "cannot iterate over an integer `{}`: count with a range, `for {elem} in 0..n`",
                        t.ty
                    )),
                    ty => err(E0090, t.span, format!("`for` needs an array or a slice to iterate over, found `{ty}`")),
                });
                let elem_ty = match &seq {
                    Ok(TExpr {
                        ty: Type::Array(el, _) | Type::Slice(el, _),
                        ..
                    }) => (**el).clone(),
                    _ => Type::Error,
                };
                self.scopes.push(HashMap::new());
                let index = index
                    .as_ref()
                    .map(|(n, sp)| self.declare_or_report(n, USIZE, false, *sp));
                let elem = self.declare_or_report(elem, elem_ty, false, *elem_span);
                self.loop_depth += 1;
                let body = self.block(body);
                self.loop_depth -= 1;
                self.scopes.pop();
                let index = index.map(|v| v.ok_or_else(silent)).transpose()?;
                TStmtKind::ForEach {
                    index,
                    elem: elem.ok_or_else(silent)?,
                    seq: seq?,
                    body,
                }
            }
            StmtKind::For {
                var,
                var_span,
                lo,
                hi,
                body,
            } => {
                // A range of two integer literals is `usize`: loop counters are mostly indices.
                let int_lit = |e: &ast::Expr| {
                    is_untyped(e) && !matches!(strip_parens(e).kind, ExprKind::Float(_))
                };
                let hint = (int_lit(lo) && int_lit(hi)).then_some(&USIZE);
                let pair = self.operand_pair(lo, hi, hint).map_err(|mut e| {
                    if hint.is_some() && e.code == E0024 {
                        e.msg += " (a range of two literals is `usize`; for a signed counter write `(0 as i64)..n`)";
                    }
                    e
                });
                let range = pair.and_then(|(lo, hi)| {
                    if lo.ty != hi.ty {
                        return err(
                            E0023,
                            hi.span,
                            format!(
                                "range bounds have different types: `{}` and `{}`",
                                lo.ty, hi.ty
                            ),
                        );
                    }
                    if !lo.ty.is_int() {
                        return err(
                            E0046,
                            lo.span,
                            format!("range bounds must be integers, found `{}`", lo.ty),
                        );
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
                TStmtKind::For {
                    var: var.ok_or_else(silent)?,
                    lo,
                    hi,
                    body,
                }
            }
            StmtKind::Break | StmtKind::Continue => {
                let word = if matches!(s.kind, StmtKind::Break) {
                    "break"
                } else {
                    "continue"
                };
                if self.loop_depth == 0 {
                    if self.in_defer {
                        return err(
                            E0047,
                            s.span,
                            format!("`{word}` cannot leave a `defer` block"),
                        );
                    }
                    return err(E0047, s.span, format!("`{word}` outside of a loop"));
                }
                if word == "break" {
                    TStmtKind::Break
                } else {
                    TStmtKind::Continue
                }
            }
            StmtKind::Return(value) => {
                if self.in_defer {
                    return err(E0048, s.span, "`return` is not allowed inside `defer`");
                }
                let ret = self.ret.clone();
                match (value, &ret) {
                    (None, t) if !needs_value(t) => TStmtKind::Return(None),
                    (None, t) => {
                        return err(E0049, s.span, format!("missing return value of type `{t}`"));
                    }
                    // `return f()` of a call that returns nothing: the call,
                    // then `return`.
                    (Some(e), Type::Void) => {
                        let t = self.expr(e, None)?;
                        if t.ty != Type::Void {
                            return err(
                                E0049,
                                e.span,
                                "function has no return type but a value was returned",
                            );
                        }
                        Self::void_return(t, s.span)
                    }
                    // `return match x { ... }`: each arm is returned, so it
                    // may be an error union, an error or a payload value.
                    (Some(e), Type::ErrorUnion(..))
                        if matches!(strip_parens(e).kind, ExprKind::Match { .. }) =>
                    {
                        TStmtKind::Return(Some(self.match_expr(
                            strip_parens(e),
                            Some(&ret),
                            false,
                        )?))
                    }
                    (Some(e), Type::ErrorUnion(set, payload)) => {
                        let t = self
                            .expr(e, (**payload != Type::Void).then(|| Self::hint(e, payload)))?;
                        if t.ty == Type::Void && **payload == Type::Void {
                            Self::void_return(t, s.span)
                        } else {
                            TStmtKind::Return(Some(self.return_union(t, set, payload)?))
                        }
                    }
                    (Some(e), t) => TStmtKind::Return(Some(self.expect(e, t)?)),
                }
            }
            StmtKind::Defer(inner) | StmtKind::ErrDefer(inner) => {
                let on_err = matches!(s.kind, StmtKind::ErrDefer(_));
                if on_err && !matches!(self.ret, Type::ErrorUnion(..) | Type::Error) {
                    return err(
                        E0068,
                        s.span,
                        "`errdefer` never runs here: the function does not return an error union",
                    );
                }
                let saved = (self.loop_depth, self.in_defer);
                self.loop_depth = 0;
                self.in_defer = true;
                let body = match &inner.kind {
                    StmtKind::Block(b) => Ok(self.block(b)),
                    _ => self.stmt(inner).map(|s| vec![s]),
                };
                (self.loop_depth, self.in_defer) = saved;
                if on_err {
                    TStmtKind::ErrDefer(body?)
                } else {
                    TStmtKind::Defer(body?)
                }
            }
            StmtKind::Block(b) => TStmtKind::Block(self.block(b)),
            StmtKind::Expr(e) => TStmtKind::Expr(self.expr_stmt(e)?),
        };
        Ok(TStmt { kind, span: s.span })
    }

    /// `t` (of type `void`) and then `return`.
    fn void_return(t: TExpr, span: Span) -> TStmtKind {
        let call = TStmt {
            kind: TStmtKind::Expr(t),
            span,
        };
        TStmtKind::Block(vec![
            call,
            TStmt {
                kind: TStmtKind::Return(None),
                span,
            },
        ])
    }

    /// An expression used as a statement: a call, `try`, `catch` or `match`.
    /// An error union result must not be dropped.
    fn expr_stmt(&mut self, e: &ast::Expr) -> Result<TExpr> {
        match &e.kind {
            ExprKind::Match { .. } => self.match_expr(e, None, true),
            ExprKind::Call { .. }
            | ExprKind::CallExpr { .. }
            | ExprKind::MethodCall { .. }
            | ExprKind::Builtin { .. }
            | ExprKind::Try(_)
            | ExprKind::Unreachable
            | ExprKind::Catch { .. } => {
                let t = self.expr(e, None)?;
                self.handled(t)
            }
            _ => err(
                E0031,
                e.span,
                "expression value is not used (only calls can be statements)",
            ),
        }
    }

    /// Reject an error union that is not handled with `try` or `catch`.
    fn handled(&self, t: TExpr) -> Result<TExpr> {
        if matches!(t.ty, Type::ErrorUnion(..)) {
            let what = match &t.kind {
                TExprKind::Call { name, .. } | TExprKind::DynCall { method: name, .. } => {
                    format!("`{name}` returns")
                }
                TExprKind::Create { .. } => "`create` returns".to_string(),
                TExprKind::AllocSlice { old: None, .. } => "`alloc` returns".to_string(),
                TExprKind::AllocSlice { .. } => "`resize` returns".to_string(),
                _ => "this expression has type".to_string(),
            };
            return err(
                E0061,
                t.span,
                format!(
                    "unhandled error: {what} `{}` (handle the error with `try` or `catch`)",
                    t.ty
                ),
            );
        }
        Ok(t)
    }

    /// `return t` in a function returning `set!payload`: an error, an error
    /// union passed on as is, or a payload value.
    fn return_union(&mut self, t: TExpr, set: &str, payload: &Type) -> Result<TExpr> {
        let span = t.span;
        match &t.ty {
            Type::Error => Err(silent()),
            Type::Void => err(E0030, span, "this expression does not produce a value"),
            Type::ErrorSet(_) => {
                self.propagate(self.errors_of(&t), span)?;
                let ty = Type::ErrorUnion(set.to_string(), Box::new(payload.clone()));
                Ok(TExpr {
                    kind: TExprKind::ErrWrap(Box::new(t)),
                    ty,
                    span,
                })
            }
            // The payload may drop `mut` (`E![]mut T` as `E![]T`).
            Type::ErrorUnion(src, p) => {
                if **p != *payload && !drops_mut(p, payload) {
                    let want = Type::ErrorUnion(set.to_string(), Box::new(payload.clone()));
                    return Err(mismatch(span, &want, &t.ty));
                }
                let errors = self.sets[src]
                    .iter()
                    .map(|n| (src.clone(), n.clone()))
                    .collect();
                self.propagate(errors, span)?;
                Ok(TExpr {
                    ty: Type::ErrorUnion(set.to_string(), Box::new(payload.clone())),
                    ..t
                })
            }
            _ if *payload == Type::Void => err(
                E0049,
                span,
                format!(
                    "this function returns `{}`: return no value, or an error",
                    self.ret
                ),
            ),
            _ => {
                let t = self.coerce(t, payload)?;
                let ty = Type::ErrorUnion(set.to_string(), Box::new(payload.clone()));
                Ok(TExpr {
                    kind: TExprKind::OkWrap(Box::new(t)),
                    ty,
                    span,
                })
            }
        }
    }

    /// The errors (set, name) that the error value `t` can hold.
    fn errors_of(&self, t: &TExpr) -> Vec<(String, String)> {
        let Type::ErrorSet(set) = &t.ty else {
            unreachable!()
        };
        match t.kind {
            TExprKind::ErrorValue(v) => {
                vec![(set.clone(), self.error_names[v as usize - 1].clone())]
            }
            _ => self.sets[set]
                .iter()
                .map(|n| (set.clone(), n.clone()))
                .collect(),
        }
    }

    /// `errors` leave the current function (through `return` or `try`):
    /// add them to its inferred set, or check that its set has them.
    fn propagate(&mut self, errors: Vec<(String, String)>, span: Span) -> Result<()> {
        let Type::ErrorUnion(target, _) = &self.ret else {
            unreachable!()
        };
        let target = target.clone();
        if target.starts_with(INFERRED) {
            let set = self.sets.get_mut(&target).unwrap();
            for (_, n) in errors {
                if !set.contains(&n) {
                    set.push(n);
                }
            }
            return Ok(());
        }
        let missing: Vec<String> = errors
            .iter()
            .filter(|(_, n)| !self.sets[&target].contains(n))
            .map(|(s, n)| qualify(s, n))
            .collect();
        if missing.is_empty() {
            return Ok(());
        }
        let (s, are) = if missing.len() == 1 {
            ("", "is")
        } else {
            ("s", "are")
        };
        err(
            E0064,
            span,
            format!(
                "error{s} {} {are} not in this function's error set `{target}`",
                missing.join(", ")
            ),
        )
    }

    /// The parts of the error union `t`, for `try` and `catch` (`what`).
    fn union_parts(t: &TExpr, what: &str) -> Result<(String, Type)> {
        match &t.ty {
            Type::ErrorUnion(set, payload) => Ok((set.clone(), (**payload).clone())),
            Type::Error => Err(silent()),
            ty => {
                let hint = if matches!(ty, Type::Optional(_)) {
                    " (unwrap an optional with `if x |v| { }`)"
                } else {
                    ""
                };
                err(
                    E0063,
                    t.span,
                    format!("`{what}` needs an error union value (`E!T`), found `{ty}`{hint}"),
                )
            }
        }
    }

    /// The optional that `if x |name|` or `while x |name|` (`what`) unwraps,
    /// and the type of its value.
    fn optional_cond(&mut self, e: &ast::Expr, what: &str, name: &str) -> (Result<TExpr>, Type) {
        let c = self.expr(e, None).and_then(|c| match &c.ty {
            Type::ErrorUnion(..) if what == "if" => err(E0102, c.span, format!(
                "`if x |{name}|` on an error union (`{}`) needs `else |e| {{ ... }}`, which handles the error",
                c.ty
            )),
            Type::Void => err(E0030, c.span, "this expression does not produce a value"),
            Type::Error => Err(silent()),
            _ => self.handled(c),
        });
        let c = c.and_then(|c| match &c.ty {
            Type::Optional(_) => Ok(c),
            t => err(
                E0027,
                c.span,
                format!("`{what}` with `|{name}|` needs an optional (`?T`) value, found `{t}`"),
            ),
        });
        let inner = match &c {
            Ok(TExpr {
                ty: Type::Optional(t),
                ..
            }) => (**t).clone(),
            _ => Type::Error,
        };
        (c, inner)
    }

    fn condition(&mut self, e: &ast::Expr, what: &str) -> Result<TExpr> {
        let c = self.value(e, Some(&Type::Bool))?;
        if c.ty != Type::Bool {
            let hint = match c.ty {
                Type::Optional(_) => " (unwrap it with `if x |v| { }` or compare it with `null`)",
                Type::Simd(..) => {
                    " (a comparison of `simd` values is a mask: use `@all(m)` or `@any(m)`)"
                }
                _ => "",
            };
            return err(
                E0026,
                c.span,
                format!("`{what}` condition must be `bool`, found `{}`{hint}", c.ty),
            );
        }
        Ok(c)
    }

    /// Can `t` be assigned to (`addr_of` false) or borrowed with `&mut`
    /// (`addr_of` true)?
    fn check_mutable(&self, t: &TExpr, addr_of: bool) -> Result<()> {
        let what = if addr_of {
            "take `&mut` of"
        } else {
            "assign to"
        };
        match &t.kind {
            TExprKind::Var(id) => {
                let v = &self.vars[*id];
                if !v.mutable {
                    let e = Error::new(
                        E0032,
                        t.span,
                        format!(
                            "cannot {what} `{}`: it is immutable (declare it with `var`)",
                            v.name
                        ),
                    );
                    return Err(match v.let_span {
                        Some(s) => e.fix(
                            format!("declare `{}` with `var`", v.name),
                            s,
                            s.right(3),
                            "var",
                        ),
                        None => e,
                    });
                }
                Ok(())
            }
            TExprKind::Global(name) => {
                let g = &self.globals[name];
                if !g.mutable {
                    let e = Error::new(
                        E0032,
                        t.span,
                        format!("cannot {what} `{name}`: it is a constant (declare it with `var`)"),
                    );
                    let s = g.kw_span;
                    return Err(e.fix(
                        format!("declare `{name}` with `var`"),
                        s,
                        s.right(3),
                        "var",
                    ));
                }
                Ok(())
            }
            TExprKind::Field { base, .. } | TExprKind::Unwrap(base) => {
                self.check_mutable(base, addr_of)
            }
            // The slice type decides, not where the slice is stored.
            TExprKind::Index { base, .. } => match &base.ty {
                Type::Slice(_, true) => Ok(()),
                Type::Slice(el, false) => {
                    let what = if addr_of {
                        "take `&mut` of an element of"
                    } else {
                        "assign through"
                    };
                    err(
                        E0033,
                        t.span,
                        format!(
                            "cannot {what} a slice `{}`: its elements are read-only (use `[]mut {el}`)",
                            base.ty
                        ),
                    )
                }
                // A C pointer is unchecked: it does not track read-only data.
                Type::CPtr(_) => Ok(()),
                _ => self.check_mutable(base, addr_of),
            },
            TExprKind::Deref(p) => match &p.ty {
                Type::Ptr(inner, false) => err(
                    E0033,
                    t.span,
                    format!(
                        "cannot {what} data behind a read-only pointer `{}` (use `*mut {inner}`)",
                        p.ty
                    ),
                ),
                _ => Ok(()),
            },
            TExprKind::SliceLen(_) | TExprKind::ArrayLen(_) | TExprKind::SlicePtr(_) => err(
                E0034,
                t.span,
                "cannot assign to a slice or array's `.len`/`.ptr`",
            ),
            _ => err(E0034, t.span, "invalid assignment target"),
        }
    }

    /// Can we take the address of this expression (`&x`, or slicing an array)?
    fn is_lvalue(t: &TExpr) -> bool {
        match &t.kind {
            TExprKind::Var(_) | TExprKind::Global(_) | TExprKind::Deref(_) => true,
            TExprKind::Field { base, .. } | TExprKind::Unwrap(base) => Self::is_lvalue(base),
            TExprKind::Index { base, .. } => {
                matches!(base.ty, Type::Slice(..) | Type::CPtr(_)) || Self::is_lvalue(base)
            }
            _ => false,
        }
    }

    // ---------------------------------------------------------- expressions

    /// Check `e` and require its type to be `ty` (see `coerce`).
    fn expect(&mut self, e: &ast::Expr, ty: &Type) -> Result<TExpr> {
        if *ty == Type::Error {
            return self.value(e, None);
        }
        let t = self.value(e, Some(Self::hint(e, ty)))?;
        self.coerce(t, ty)
    }

    /// The type to check `e` with when `ty` is expected: `T` for `?T`, so
    /// that the value is wrapped; `null` and `match` need `?T` itself.
    fn hint<'t>(e: &ast::Expr, ty: &'t Type) -> &'t Type {
        match ty {
            Type::Optional(inner)
                if !matches!(
                    strip_parens(e).kind,
                    ExprKind::Null | ExprKind::Match { .. }
                ) =>
            {
                inner
            }
            _ => ty,
        }
    }

    /// Require `t` to have type `ty`. Four implicit coercions apply: a `T`
    /// is wrapped where `?T` is expected, a `*mut T` (`[]mut T`) is accepted
    /// where `*T` (`[]T`) is expected (also inside `?`), an error value is
    /// accepted by an error set that has all of its errors, and a pointer to
    /// a struct that implements `I` is accepted where `dyn I` is expected.
    /// One of the others can come before the wrapping: a `[]mut T` is
    /// accepted where `?[]T` is expected.
    fn coerce(&mut self, t: TExpr, ty: &Type) -> Result<TExpr> {
        if t.ty == *ty {
            return Ok(t);
        }
        if let Type::Optional(inner) = ty {
            let wrap = |t: TExpr| {
                let span = t.span;
                TExpr {
                    kind: TExprKind::Some(Box::new(t)),
                    ty: ty.clone(),
                    span,
                }
            };
            if **inner == t.ty {
                return Ok(wrap(t));
            }
            if !matches!(t.ty, Type::Optional(_) | Type::Error)
                && !matches!(**inner, Type::Optional(_))
            {
                match self.coerce(t.clone(), inner) {
                    Ok(t) => return Ok(wrap(t)),
                    Err(e) if e.code != E0023 => return Err(e),
                    Err(_) => {}
                }
            }
        }
        if drops_mut(&t.ty, ty) {
            return Ok(TExpr {
                ty: ty.clone(),
                ..t
            });
        }
        if let (Type::ErrorSet(want), Type::ErrorSet(_)) = (ty, &t.ty)
            && self
                .errors_of(&t)
                .iter()
                .all(|(_, n)| self.sets[want].contains(n))
        {
            return Ok(TExpr {
                ty: ty.clone(),
                ..t
            });
        }
        if *ty == Type::Builtin(BuiltinTy::Allocator) {
            if let Type::Ptr(inner, mutable) = &t.ty
                && let Type::Struct(s) = &**inner
            {
                let s = s.clone();
                if let Some(why) = self.why_not_allocator(&s) {
                    return err(
                        E0105,
                        t.span,
                        format!("`{s}` cannot be an `Allocator`: {why}"),
                    );
                }
                if !mutable {
                    return err(
                        E0105,
                        t.span,
                        format!(
                            "an `Allocator` is made from a `*mut {s}` (write `&mut x`): its `alloc` and `free` take `self: *mut Self`"
                        ),
                    );
                }
                if !self.alloc_impls.contains(&s) {
                    self.alloc_impls.push(s.clone());
                }
                let span = t.span;
                return Ok(TExpr {
                    kind: TExprKind::ToAllocator {
                        ptr: Box::new(t),
                        strukt: s,
                    },
                    ty: ty.clone(),
                    span,
                });
            }
            if let Type::Struct(_) = &t.ty {
                return err(
                    E0023,
                    t.span,
                    format!(
                        "mismatched types: expected `{ty}`, found `{}` (an `Allocator` points to its data: write `&mut x`)",
                        t.ty
                    ),
                );
            }
        }
        if let Type::Dyn(iface) = ty {
            if let Type::Ptr(inner, mutable) = &t.ty
                && let Type::Struct(s) = &**inner
            {
                let s = s.clone();
                self.implements(&s, iface, *mutable, t.span)?;
                let pair = (iface.clone(), s.clone());
                if !self.vtables.contains(&pair) {
                    self.vtables.push(pair);
                }
                let span = t.span;
                return Ok(TExpr {
                    kind: TExprKind::ToDyn {
                        ptr: Box::new(t),
                        strukt: s,
                    },
                    ty: ty.clone(),
                    span,
                });
            }
            if let Type::Struct(_) = &t.ty {
                return err(
                    E0023,
                    t.span,
                    format!(
                        "mismatched types: expected `{ty}`, found `{}` (a `dyn` value points to its data: write `&mut x` or `&x`)",
                        t.ty
                    ),
                );
            }
        }
        Err(mismatch(t.span, ty, &t.ty))
    }

    /// Does struct `s` implement interface `iface`, for a `dyn` made from a
    /// `*mut s` (`mutable`) or `*s`? It must have every method of the
    /// interface, with the same parameter and return types.
    fn implements(&self, s: &str, iface: &str, mutable: bool, span: Span) -> Result<()> {
        if let Some(why) = self.why_not(s, iface) {
            return err(
                E0072,
                span,
                format!("`{s}` does not implement `{iface}`: {why}"),
            );
        }
        for m in &self.ifaces[iface].methods {
            if !mutable && self.recvs.get(&format!("{s}.{}", m.name)) == Some(&Recv::Ptr(true)) {
                return err(
                    E0072,
                    span,
                    format!(
                        "a `dyn {iface}` made from a read-only pointer cannot call `{s}.{}`, which takes `self: *mut Self` (use `&mut`)",
                        m.name
                    ),
                );
            }
        }
        Ok(())
    }

    /// Why struct `s` cannot be an `Allocator`, or `None` if it can: it
    /// needs `fn alloc(self: *mut Self, size: usize, align: usize) ?[*]u8`
    /// and `fn free(self: *mut Self, p: [*]u8, size: usize)`.
    fn why_not_allocator(&self, s: &str) -> Option<String> {
        let me = Type::Ptr(Box::new(Type::Struct(s.to_string())), true);
        let bytes = Type::CPtr(Box::new(Type::Int(IntTy::U8)));
        let want = [
            (
                "alloc",
                vec![me.clone(), USIZE, USIZE],
                Type::Optional(Box::new(bytes.clone())),
                "fn alloc(self: *mut Self, size: usize, align: usize) ?[*]u8",
            ),
            (
                "free",
                vec![me, bytes, USIZE],
                Type::Void,
                "fn free(self: *mut Self, p: [*]u8, size: usize)",
            ),
        ];
        for (name, params, ret, shown) in want {
            match self.fns.get(&format!("{s}.{name}")) {
                None => return Some(format!("it has no method `{name}` (it needs `{shown}`)")),
                Some(sig) if sig.params != params || sig.ret != ret => {
                    return Some(format!("its method `{name}` must be `{shown}`"));
                }
                _ => {}
            }
        }
        None
    }

    /// Why struct `s` does not implement interface `iface`, or `None` if it
    /// does (through a `*mut s`).
    fn why_not(&self, s: &str, iface: &str) -> Option<String> {
        let show_recv = |r: Recv| match r {
            Recv::Value => "Self",
            Recv::Ptr(false) => "*Self",
            _ => "*mut Self",
        };
        let show = |recv: Option<&str>, params: &[Type], ret: &Type| {
            let ps: Vec<String> = recv
                .map(str::to_string)
                .into_iter()
                .chain(params.iter().map(|p| p.to_string()))
                .collect();
            if *ret == Type::Void {
                format!("fn({})", ps.join(", "))
            } else {
                format!("fn({}) {ret}", ps.join(", "))
            }
        };
        for m in &self.ifaces[iface].methods {
            let key = format!("{s}.{}", m.name);
            let (Some(sig), Some(&recv)) = (self.fns.get(&key), self.recvs.get(&key)) else {
                return Some(format!("it has no method `{}`", m.name));
            };
            let want = show(
                Some(if m.mutable { "*mut Self" } else { "*Self" }),
                &m.params,
                &m.ret,
            );
            if recv == Recv::Static || sig.params[1..] != m.params[..] || sig.ret != m.ret {
                let have = match recv {
                    Recv::Static => show(None, &sig.params, &sig.ret),
                    r => show(Some(show_recv(r)), &sig.params[1..], &sig.ret),
                };
                return Some(format!(
                    "method `{}` is `{have}`, but `{iface}` needs `{want}`",
                    m.name
                ));
            }
            if recv == Recv::Ptr(true) && !m.mutable {
                return Some(format!(
                    "method `{}` takes `self: *mut Self`, but `{iface}` declares `self: *Self`",
                    m.name
                ));
            }
        }
        None
    }

    /// Check an expression that must produce a value (not a void call, and
    /// not an unhandled error union).
    fn value(&mut self, e: &ast::Expr, expected: Option<&Type>) -> Result<TExpr> {
        let t = self.expr(e, expected)?;
        if t.ty == Type::Error {
            return Err(silent());
        }
        if t.ty == Type::Void {
            return err(E0030, t.span, "this expression does not produce a value");
        }
        self.handled(t)
    }

    /// The operands of `==` or `!=`: like `operand_pair`, except that an
    /// untyped literal or `null` compared with a `?T` is checked as a `T`.
    fn eq_operands(&mut self, l: &ast::Expr, r: &ast::Expr) -> Result<(TExpr, TExpr)> {
        let payload = |t: &Type| match t {
            Type::Optional(inner) => (**inner).clone(),
            t => t.clone(),
        };
        if is_untyped(l) && !is_untyped(r) {
            let rt = self.value(r, None)?;
            let lt = self.value(l, Some(&payload(&rt.ty)))?;
            Ok((lt, rt))
        } else {
            let lt = self.value(l, None)?;
            let rt = self.value(r, Some(&payload(&lt.ty)))?;
            Ok((lt, rt))
        }
    }

    /// `a == b` on error values: an error value written out (`E.A`) must be
    /// one that the other side's set has, or the comparison is always false.
    fn error_comparable(&self, a: &TExpr, b: &TExpr, span: Span) -> Result<()> {
        for (x, y) in [(a, b), (b, a)] {
            let (TExprKind::ErrorValue(v), Type::ErrorSet(set)) = (&x.kind, &y.ty) else {
                continue;
            };
            let name = &self.error_names[*v as usize - 1];
            if !self.sets[set].contains(name) {
                return err(
                    E0060,
                    span,
                    format!(
                        "this comparison is always false: {} has no error `{name}`",
                        describe_set(set)
                    ),
                );
            }
        }
        Ok(())
    }

    /// Why `==` cannot compare two values of type `t` (an empty reason if
    /// there is nothing more to say), or `None` if it can: numbers, `bool`,
    /// pointers, enums without fields, optionals of those, and structs whose
    /// fields all compare.
    fn why_not_comparable(&self, t: &Type) -> Option<String> {
        match t {
            Type::Int(_)
            | Type::Float(_)
            | Type::Bool
            | Type::Ptr(..)
            | Type::CPtr(_)
            | Type::ErrorSet(_) => None,
            Type::Enum(n) if self.enums[n].variants.iter().any(|v| !v.fields.is_empty()) => {
                Some("it has variants with fields (use `match`)".to_string())
            }
            Type::Enum(_) => None,
            Type::Simd(..) => Some(
                "`==` on a `simd` compares lane by lane (a mask): use `@all(a == b)`".to_string(),
            ),
            t @ Type::Param(_, p) if !self.hash_param(t) => Some(format!(
                "it is a `comptime` type (add `where {p}: Hash` to compare it)"
            )),
            Type::Param(..) => None,
            Type::Struct(n) if self.structs[n].is_union => Some("it is a union".to_string()),
            Type::Struct(n) => self.structs[n]
                .fields
                .iter()
                .find(|(_, ft)| self.why_not_comparable(ft).is_some())
                .map(|(f, ft)| {
                    format!("its field `{f}` has type `{ft}`, which `==` cannot compare")
                }),
            Type::Optional(inner) => {
                self.why_not_comparable(inner)
                    .map(|why| match why.strip_prefix("it has ") {
                        Some(rest) => format!("`{inner}` has {rest}"),
                        None if why.is_empty() => format!("`{inner}` cannot be compared"),
                        None => format!("`{inner}` cannot be compared: {why}"),
                    })
            }
            _ => Some(String::new()),
        }
    }

    /// Check two operands that must share a type; an untyped literal on either
    /// side takes the type of the other side.
    fn operand_pair(
        &mut self,
        l: &ast::Expr,
        r: &ast::Expr,
        hint: Option<&Type>,
    ) -> Result<(TExpr, TExpr)> {
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
                return Ok(TExpr {
                    kind: TExprKind::Float(v as f64),
                    ty: expected.unwrap().clone(),
                    span,
                });
            }
            Some(t @ Type::Int(_)) => t.clone(),
            // Checked again with the integer type of each instance.
            Some(t @ Type::Param(..)) if self.integer_param(t) => {
                return Ok(TExpr {
                    kind: TExprKind::Int(v),
                    ty: t.clone(),
                    span,
                });
            }
            Some(t) => {
                return err(
                    E0023,
                    span,
                    format!(
                        "mismatched types: expected `{t}`, found integer literal{}",
                        splat_hint(t)
                    ),
                );
            }
            None => I64,
        };
        let Type::Int(it) = ty else { unreachable!() };
        if v < it.min(self.layout) || v > it.max(self.layout) {
            return err(
                E0024,
                span,
                format!("integer literal `{v}` does not fit in `{ty}`"),
            );
        }
        Ok(TExpr {
            kind: TExprKind::Int(v),
            ty,
            span,
        })
    }

    fn float_lit(&self, v: f64, span: Span, expected: Option<&Type>) -> Result<TExpr> {
        let ty = match expected {
            Some(t @ Type::Float(_)) => t.clone(),
            Some(t) => {
                return err(
                    E0023,
                    span,
                    format!(
                        "mismatched types: expected `{t}`, found float literal{}",
                        splat_hint(t)
                    ),
                );
            }
            None => Type::Float(FloatTy::F64),
        };
        Ok(TExpr {
            kind: TExprKind::Float(v),
            ty,
            span,
        })
    }

    fn arith_operand_ok(&self, op: BinOp, ty: &Type, span: Span) -> Result<()> {
        if let Type::Simd(el, _) = ty {
            let ok = match op {
                BinOp::Add | BinOp::Sub | BinOp::Mul => el.is_numeric(),
                BinOp::Div => matches!(**el, Type::Float(_)),
                _ => false,
            };
            if !ok {
                let what = if **el == Type::Bool {
                    "a mask has no operators (test it with `@all`, `@any` or `m[i]`)"
                } else {
                    "a `simd` has `+ - *`, `/` on float lanes, and comparisons"
                };
                return err(
                    E0025,
                    span,
                    format!(
                        "operator `{}` cannot be applied to `{ty}`: {what}",
                        op.as_str()
                    ),
                );
            }
            return Ok(());
        }
        let ok = match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
                ty.is_numeric() || self.integer_param(ty)
            }
            BinOp::Rem
            | BinOp::AddWrap
            | BinOp::SubWrap
            | BinOp::MulWrap
            | BinOp::BitAnd
            | BinOp::BitOr
            | BinOp::BitXor
            | BinOp::Shl
            | BinOp::Shr => self.int_like(ty),
            _ => unreachable!(),
        };
        if !ok {
            return err(
                E0025,
                span,
                format!(
                    "operator `{}` cannot be applied to `{ty}`{}",
                    op.as_str(),
                    self.integer_hint(ty)
                ),
            );
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
                    return err(
                        E0028,
                        span,
                        format!(
                            "mismatched types: expected `{t}`, found `null` (only optional types `?T` can be `null`)"
                        ),
                    );
                }
                None => {
                    return err(
                        E0028,
                        span,
                        "cannot infer the type of `null` (add a type annotation, e.g. `let x: ?i32 = null`)",
                    );
                }
            },
            ExprKind::Match { .. } => return self.match_expr(e, expected, false),
            ExprKind::Str(s) => (
                TExprKind::Str(s.clone()),
                Type::Slice(Box::new(Type::Int(IntTy::U8)), false),
            ),
            ExprKind::Paren(inner) => return self.expr(inner, expected),
            ExprKind::Ident(name) => match self.lookup(name) {
                Some(id) => (TExprKind::Var(id), self.vars[id].ty.clone()),
                None if self.global_decls.iter().any(|g| g.name == *name) => {
                    let (ty, _) = self.global(name, span)?;
                    (TExprKind::Global(name.clone()), ty)
                }
                None if self.generics.get(name).is_some_and(|f| is_type_fn(f)) => {
                    return err(
                        E0080,
                        span,
                        format!("`{name}` is a type function: write `{name}(T)`"),
                    );
                }
                // A top-level function is a value; methods, tests and
                // instances have names with other characters.
                None if self.fns.contains_key(name) && !name.contains(['.', '#', '(']) => {
                    let sig = self.fns[name].clone();
                    if sig.variadic {
                        return err(
                            E0020,
                            span,
                            format!("the variadic C function `{name}` is not a value (call it)"),
                        );
                    }
                    if let Type::ErrorUnion(set, _) = &sig.ret
                        && set.starts_with(INFERRED)
                    {
                        self.ensure_body(name, span)?;
                    }
                    let kind = TExprKind::FnRef {
                        name: sig.c_key(name),
                        is_extern: sig.is_extern,
                    };
                    (kind, Type::Fn(sig.params, Box::new(sig.ret)))
                }
                None if self.generics.contains_key(name) => {
                    return err(
                        E0020,
                        span,
                        format!(
                            "the generic function `{name}` is not a value (call it with its type arguments)"
                        ),
                    );
                }
                None if self.is_type_name(name) => {
                    return err(E0074, span, format!("`{name}` is a type, not a value"));
                }
                None => {
                    let e = Error::new(E0018, span, format!("undefined variable `{name}`"));
                    return Err(suggest(
                        e,
                        name,
                        span,
                        self.scopes
                            .iter()
                            .flat_map(|s| s.keys().map(String::as_str)),
                    ));
                }
            },
            ExprKind::Call { name, args }
                if name == LOG_ERR || name == LOG_INFO || name == LOG_LINE =>
            {
                let prefix = match name.as_str() {
                    LOG_ERR => LogPrefix::Err,
                    LOG_INFO => LogPrefix::Info,
                    _ => LogPrefix::None,
                };
                return self.log(prefix, args, span);
            }
            // A call of a function value in a local or a global.
            ExprKind::Call { name, args }
                if self.lookup(name).is_some()
                    || self.global_decls.iter().any(|g| g.name == *name) =>
            {
                let callee = self.value(
                    &ast::Expr {
                        kind: ExprKind::Ident(name.clone()),
                        span,
                    },
                    None,
                )?;
                return self.call_value(callee, Some(name), args, span);
            }
            ExprKind::CallExpr { callee, args } => {
                let callee = self.value(callee, None)?;
                return self.call_value(callee, None, args, span);
            }
            ExprKind::Call { name, args } if name == TESTING_ALLOCATOR => {
                if !args.is_empty() {
                    return err(
                        E0029,
                        span,
                        format!(
                            "function `std.testing.allocator` takes 0 argument(s) but {} were given",
                            args.len()
                        ),
                    );
                }
                if !self.in_test {
                    return err(
                        E0075,
                        span,
                        "`std.testing.allocator()` can only be used in a `test` block (pass the allocator to other functions)",
                    );
                }
                let kind = TExprKind::Intrinsic {
                    op: Intrinsic::TestAllocator,
                    args: vec![],
                };
                return Ok(TExpr {
                    kind,
                    ty: Type::Builtin(BuiltinTy::Allocator),
                    span,
                });
            }
            ExprKind::Call { name, args } => {
                if let Some(&f) = self.generics.get(name) {
                    if is_type_fn(f) {
                        let t = self.type_arg(e, None)?;
                        return err(E0074, span, format!("`{t}` is a type, not a value"));
                    }
                    return self.generic_call(f, args, span);
                }
                if !self.fns.contains_key(name) {
                    let e = Error::new(E0019, span, format!("undefined function `{name}`"));
                    let fns = self
                        .fns
                        .keys()
                        .filter(|k| !k.contains(['.', '#', '(']))
                        .chain(self.generics.keys());
                    return Err(suggest(e, name, span, fns.map(String::as_str)));
                }
                return self.call(name, None, args, span);
            }
            ExprKind::Type(t) => {
                let t = self.resolve_type(t)?;
                return err(E0074, span, format!("`{t}` is a type, not a value"));
            }
            ExprKind::StructType(_) => {
                return err(
                    E0077,
                    span,
                    "`struct { ... }` can only be the value that a type function (returning `type`) returns",
                );
            }
            ExprKind::MethodCall { base, name, args } => {
                return self.method_call(base, name, args, span);
            }
            ExprKind::Builtin { name, args }
                if matches!(name.as_str(), "expect" | "expectEqual" | "testAllocator") =>
            {
                let n = match name.as_str() {
                    "expect" => 1,
                    "expectEqual" => 2,
                    _ => 0,
                };
                if args.len() != n {
                    let what = ["no arguments", "one argument", "two arguments"][n];
                    return err(E0022, span, format!("`@{name}` takes {what}"));
                }
                match name.as_str() {
                    "expect" => {
                        let c = self.expect(&args[0], &Type::Bool)?;
                        (
                            TExprKind::Intrinsic {
                                op: Intrinsic::Expect,
                                args: vec![c],
                            },
                            Type::Void,
                        )
                    }
                    "expectEqual" => {
                        let (want, got) = self.operand_pair(&args[0], &args[1], None)?;
                        if want.ty != got.ty
                            && !matches!((&want.ty, &got.ty), (Type::Slice(a, _), Type::Slice(b, _)) if a == b)
                        {
                            return err(
                                E0023,
                                span,
                                format!(
                                    "mismatched types in `@expectEqual`: `{}` and `{}` (use `as` to convert)",
                                    want.ty, got.ty
                                ),
                            );
                        }
                        let ok = match &want.ty {
                            Type::Int(_) | Type::Float(_) | Type::Bool => true,
                            Type::Slice(el, _) => **el == Type::Int(IntTy::U8),
                            _ => false,
                        };
                        if !ok {
                            return err(
                                E0022,
                                span,
                                format!(
                                    "`@expectEqual` cannot compare values of type `{}`",
                                    want.ty
                                ),
                            );
                        }
                        (
                            TExprKind::Intrinsic {
                                op: Intrinsic::ExpectEqual,
                                args: vec![want, got],
                            },
                            Type::Void,
                        )
                    }
                    _ => {
                        if !self.in_test {
                            return err(
                                E0075,
                                span,
                                "`@testAllocator()` can only be used in a `test` block (pass the allocator to other functions)",
                            );
                        }
                        (
                            TExprKind::Intrinsic {
                                op: Intrinsic::TestAllocator,
                                args: vec![],
                            },
                            Type::Builtin(BuiltinTy::Allocator),
                        )
                    }
                }
            }
            ExprKind::Builtin { name, args } if name.starts_with("atomic") => {
                return self.atomic(name, args, span);
            }
            ExprKind::Builtin { name, .. } if name == "setSafety" => {
                return err(
                    E0108,
                    span,
                    "`@setSafety` must be the first statement of a block (it applies to the whole block)",
                );
            }
            ExprKind::Builtin { name, args }
                if matches!(
                    name.as_str(),
                    "splat" | "reduceAdd" | "reduceMin" | "reduceMax" | "all" | "any" | "select"
                ) =>
            {
                return self.simd_builtin(name, args, span);
            }
            ExprKind::Builtin { name, args } if name == "hash" => {
                let [a] = args.as_slice() else {
                    return err(E0022, span, "`@hash` takes one argument, the value to hash");
                };
                let v = self.value(a, None)?;
                if let Some(why) = self.why_not_comparable(&v.ty) {
                    let why = match &v.ty {
                        Type::Slice(..) => {
                            "`==` cannot compare it (for `[]u8` keys, use `std.map.StrMap`)"
                                .to_string()
                        }
                        _ if why.is_empty() => "`==` cannot compare it".to_string(),
                        _ => why,
                    };
                    return err(
                        E0107,
                        v.span,
                        format!("`@hash` cannot hash a `{}`: {why}", v.ty),
                    );
                }
                (
                    TExprKind::Intrinsic {
                        op: Intrinsic::Hash,
                        args: vec![v],
                    },
                    Type::Int(IntTy::U64),
                )
            }
            ExprKind::Builtin { name, args } if name == "tagName" => {
                let [a] = args.as_slice() else {
                    return err(
                        E0022,
                        span,
                        "`@tagName` takes one argument, an error or enum value",
                    );
                };
                let v = self.value(a, None)?;
                if !matches!(v.ty, Type::ErrorSet(_) | Type::Enum(_)) {
                    return err(
                        E0116,
                        v.span,
                        format!(
                            "`@tagName` needs an error or enum value, found type `{}`",
                            v.ty
                        ),
                    );
                }
                (
                    TExprKind::Intrinsic {
                        op: Intrinsic::TagName,
                        args: vec![v],
                    },
                    Type::Slice(Box::new(Type::Int(IntTy::U8)), false),
                )
            }
            ExprKind::Builtin { name, args } if name == "hasLink" => {
                let lib = match args.as_slice() {
                    [
                        ast::Expr {
                            kind: ExprKind::Str(s),
                            ..
                        },
                    ] => String::from_utf8_lossy(s).into_owned(),
                    _ => {
                        return err(
                            E0100,
                            span,
                            "`@hasLink` takes a string literal, the name of a library: `@hasLink(\"libcurl\")`",
                        );
                    }
                };
                if !self.links.contains(&lib) && !self.optional_links.contains(&lib) {
                    return err(
                        E0100,
                        args[0].span,
                        format!(
                            "`@hasLink(\"{lib}\")`: no `@linkOptional(\"{lib}\")` (or `@link`) in the program"
                        ),
                    );
                }
                (TExprKind::HasLink(lib), Type::Bool)
            }
            ExprKind::Builtin { name, args }
                if matches!(
                    name.as_str(),
                    "sizeOf" | "zeroed" | "ptrCast" | "link" | "linkOptional"
                ) =>
            {
                return self.c_builtin(name, args, span);
            }
            ExprKind::Builtin { name, args } if name == "downcast" => {
                return self.downcast(args, span);
            }
            ExprKind::Builtin { name, .. } if name == "cimport" => {
                return err(
                    E0112,
                    span,
                    "`@cimport` is a declaration at the top level of a file, not a value: `let c = @cimport(\"zlib.h\")`, then `c.name`",
                );
            }
            ExprKind::Builtin { name, args } if name == "embedFile" => (
                TExprKind::Str(self.embed_file(args, span)?),
                Type::Slice(Box::new(Type::Int(IntTy::U8)), false),
            ),
            ExprKind::Builtin { name, args } => {
                let newline = match name.as_str() {
                    "print" => false,
                    "println" => true,
                    _ => return err(E0021, span, format!("unknown builtin `@{name}`")),
                };
                if args.len() > 1 || (!newline && args.is_empty()) {
                    let hint = if args.len() > 1 {
                        "; print each value with its own `@print`, or build the line with `std.text.Builder`"
                    } else {
                        ""
                    };
                    return err(
                        E0022,
                        span,
                        format!(
                            "`@{name}` takes {} argument{hint}",
                            if newline { "at most one" } else { "one" }
                        ),
                    );
                }
                let arg = match args.first() {
                    Some(a) => {
                        let t = self.value(a, None)?;
                        let printable = match &t.ty {
                            Type::Int(_) | Type::Float(_) | Type::Bool => true,
                            Type::Slice(el, _) => **el == Type::Int(IntTy::U8),
                            _ => false,
                        };
                        if !printable {
                            return err(
                                E0022,
                                t.span,
                                format!("`@{name}` cannot print a value of type `{}`", t.ty),
                            );
                        }
                        Some(Box::new(t))
                    }
                    None => None,
                };
                (TExprKind::Print { arg, newline }, Type::Void)
            }
            ExprKind::Binary {
                op: op @ (BinOp::Eq | BinOp::Ne),
                lhs,
                rhs,
            } if is_null(lhs) || is_null(rhs) => {
                let other = if is_null(lhs) { rhs } else { lhs };
                let t = self.value(other, None)?;
                if !matches!(t.ty, Type::Optional(_)) {
                    return err(
                        E0028,
                        span,
                        format!(
                            "only optional values (`?T`) can be compared with `null`, found `{}`",
                            t.ty
                        ),
                    );
                }
                let is_some = TExpr {
                    kind: TExprKind::IsSome(Box::new(t)),
                    ty: Type::Bool,
                    span,
                };
                match op {
                    BinOp::Ne => (is_some.kind, Type::Bool),
                    _ => (
                        TExprKind::Unary {
                            op: UnOp::Not,
                            expr: Box::new(is_some),
                        },
                        Type::Bool,
                    ),
                }
            }
            ExprKind::Binary { op, lhs, rhs } => {
                let op = *op;
                if matches!(op, BinOp::And | BinOp::Or) {
                    let l = self.expect(lhs, &Type::Bool)?;
                    let r = self.expect(rhs, &Type::Bool)?;
                    (
                        TExprKind::Binary {
                            op,
                            lhs: Box::new(l),
                            rhs: Box::new(r),
                        },
                        Type::Bool,
                    )
                } else {
                    let hint = if op.is_arith() {
                        expected.filter(|t| t.is_numeric() || self.integer_param(t))
                    } else {
                        None
                    };
                    let eq = matches!(op, BinOp::Eq | BinOp::Ne);
                    let (mut l, mut r) = if eq {
                        self.eq_operands(lhs, rhs)?
                    } else {
                        self.operand_pair(lhs, rhs, hint)?
                    };
                    // `?T == T`: the `T` is wrapped, and two `?T` compare.
                    if eq {
                        if matches!(&l.ty, Type::Optional(inner) if **inner == r.ty) {
                            r = self.coerce(r, &l.ty.clone())?;
                        } else if matches!(&r.ty, Type::Optional(inner) if **inner == l.ty) {
                            l = self.coerce(l, &r.ty.clone())?;
                        }
                    }
                    // `*mut T` and `*T` compare as pointers, and two error
                    // values by their error, whatever their sets.
                    let same_ptr = matches!((&l.ty, &r.ty), (Type::Ptr(a, _), Type::Ptr(b, _)) if a == b)
                        || matches!((&l.ty, &r.ty), (Type::ErrorSet(_), Type::ErrorSet(_)));
                    if eq && let (Type::ErrorSet(_), Type::ErrorSet(_)) = (&l.ty, &r.ty) {
                        self.error_comparable(&l, &r, span)?;
                    }
                    if l.ty != r.ty && !(same_ptr && eq) {
                        let hint = if l.ty.is_numeric() && r.ty.is_numeric() {
                            " (use `as` to convert)"
                        } else {
                            ""
                        };
                        return err(
                            E0023,
                            span,
                            format!(
                                "mismatched types in `{}`: `{}` and `{}`{hint}",
                                op.as_str(),
                                l.ty,
                                r.ty
                            ),
                        );
                    }
                    let ty = if op.is_arith() {
                        self.arith_operand_ok(op, &l.ty, span)?;
                        l.ty.clone()
                    } else if let Type::Simd(el, n) = &l.ty {
                        if **el == Type::Bool {
                            return err(
                                E0025,
                                span,
                                format!(
                                    "operator `{}` cannot be applied to `{}`: a mask has no operators (test it with `@all`, `@any` or `m[i]`)",
                                    op.as_str(),
                                    l.ty
                                ),
                            );
                        }
                        Type::Simd(Box::new(Type::Bool), *n)
                    } else {
                        let ok = match self.why_not_comparable(&l.ty) {
                            Some(why) if eq && !why.is_empty() => {
                                return err(
                                    E0025,
                                    span,
                                    format!(
                                        "operator `{}` cannot be applied to `{}`: {why}",
                                        op.as_str(),
                                        l.ty
                                    ),
                                );
                            }
                            None if eq => true,
                            _ => l.ty.is_numeric() || self.integer_param(&l.ty),
                        };
                        if !ok {
                            return err(
                                E0025,
                                span,
                                format!(
                                    "operator `{}` cannot be applied to `{}`{}",
                                    op.as_str(),
                                    l.ty,
                                    self.integer_hint(&l.ty)
                                ),
                            );
                        }
                        Type::Bool
                    };
                    (
                        TExprKind::Binary {
                            op,
                            lhs: Box::new(l),
                            rhs: Box::new(r),
                        },
                        ty,
                    )
                }
            }
            ExprKind::Unary {
                op: UnOp::Neg,
                expr,
            } => match &expr.kind {
                ExprKind::Int(v) => return self.int_lit(-(*v as i128), span, expected),
                ExprKind::Float(v) => return self.float_lit(-*v, span, expected),
                _ => {
                    let t = self.value(
                        expr,
                        expected.filter(|t| t.is_numeric() || self.integer_param(t)),
                    )?;
                    let ok = match &t.ty {
                        Type::Int(i) => i.signed(),
                        Type::Float(_) => true,
                        Type::Simd(el, _) => {
                            matches!(**el, Type::Float(_))
                                || matches!(**el, Type::Int(i) if i.signed())
                        }
                        t => self.integer_param(t),
                    };
                    if !ok {
                        return err(
                            E0025,
                            span,
                            format!("cannot negate a value of type `{}`", t.ty),
                        );
                    }
                    let ty = t.ty.clone();
                    (
                        TExprKind::Unary {
                            op: UnOp::Neg,
                            expr: Box::new(t),
                        },
                        ty,
                    )
                }
            },
            ExprKind::Unary {
                op: UnOp::BitNot,
                expr,
            } => {
                let t = self.value(expr, expected.filter(|t| self.int_like(t)))?;
                if !self.int_like(&t.ty) {
                    return err(
                        E0025,
                        span,
                        format!("operator `~` requires an integer, found `{}`", t.ty),
                    );
                }
                let ty = t.ty.clone();
                (
                    TExprKind::Unary {
                        op: UnOp::BitNot,
                        expr: Box::new(t),
                    },
                    ty,
                )
            }
            ExprKind::Unary {
                op: UnOp::Not,
                expr,
            } => {
                let t = self.value(expr, Some(&Type::Bool))?;
                if t.ty != Type::Bool {
                    return err(
                        E0025,
                        span,
                        format!("operator `!` requires `bool`, found `{}`", t.ty),
                    );
                }
                (
                    TExprKind::Unary {
                        op: UnOp::Not,
                        expr: Box::new(t),
                    },
                    Type::Bool,
                )
            }
            ExprKind::AddrOf { mutable, expr } => {
                let t = self.value(expr, None)?;
                if !Self::is_lvalue(&t) {
                    return err(
                        E0035,
                        span,
                        "cannot take the address of a temporary value (store it in a variable first)",
                    );
                }
                if let TExprKind::Index { base, .. } = &t.kind
                    && let Type::Simd(..) = base.ty
                {
                    return err(
                        E0109,
                        span,
                        format!(
                            "a lane of a `{}` has no address: copy it out (`let x = v[i]`)",
                            base.ty
                        ),
                    );
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
                    return err(
                        E0036,
                        span,
                        format!(
                            "cannot dereference a value of type `{}` (`.*` needs a pointer)",
                            t.ty
                        ),
                    );
                };
                let ty = (**pointee).clone();
                (TExprKind::Deref(Box::new(t)), ty)
            }
            ExprKind::Cast { expr, ty } => {
                let to = self.resolve_type(ty)?;
                let t = self.value(expr, None)?;
                // A `T` with `where T: Integer` converts like an integer; a
                // `bool` converts to an integer (false is 0, true is 1).
                let numeric = |ty: &Type| ty.is_numeric() || self.integer_param(ty);
                let bool_to_int = t.ty == Type::Bool && self.int_like(&to);
                if !bool_to_int && (!numeric(&t.ty) || !numeric(&to)) {
                    let hint = match (&t.ty, &to) {
                        (Type::Bool, Type::Float(_)) => {
                            format!(" (cast to an integer first: `(b as u8) as {to}`)")
                        }
                        (_, Type::Bool) if t.ty.is_numeric() => {
                            " (compare instead: `x != 0`)".to_string()
                        }
                        (Type::Param(_, p), _) if to.is_numeric() => {
                            format!(" (add `where {p}: Integer` to convert it like an integer)")
                        }
                        (_, Type::Param(_, p)) => {
                            format!(" (add `where {p}: Integer` to convert numbers to it)")
                        }
                        _ => " (only numeric casts are supported)".to_string(),
                    };
                    return err(
                        E0037,
                        span,
                        format!("cannot cast `{}` to `{to}`{hint}", t.ty),
                    );
                }
                (TExprKind::Cast(Box::new(t)), to)
            }
            ExprKind::Try(inner) => {
                if self.in_defer {
                    return err(E0048, span, "`try` is not allowed inside `defer`");
                }
                match &self.ret {
                    Type::ErrorUnion(..) => {}
                    Type::Error => return Err(silent()),
                    _ => {
                        return err(
                            E0062,
                            span,
                            "`try` can only be used in a function that returns an error union (`E!T` or `!T`)",
                        );
                    }
                }
                let t = self.expr(inner, None)?;
                let (set, payload) = Self::union_parts(&t, "try")?;
                let errors = self.sets[&set]
                    .iter()
                    .map(|n| (set.clone(), n.clone()))
                    .collect();
                self.propagate(errors, span)?;
                (TExprKind::Try(Box::new(t)), payload)
            }
            ExprKind::Catch {
                expr,
                capture,
                body,
            } => {
                let t = self.expr(expr, None)?;
                let (set, payload) = Self::union_parts(&t, "catch")?;
                self.scopes.push(HashMap::new());
                let capture = capture.as_ref().map(|(n, sp)| {
                    self.declare_or_report(n, Type::ErrorSet(set.clone()), false, *sp)
                });
                let failures = self.failures;
                let body = match &**body {
                    ArmBody::Block(b) => {
                        let stmts = if payload == Type::Void {
                            self.block(b)
                        } else {
                            self.value_block(b, &payload)
                        };
                        if payload != Type::Void
                            && self.failures == failures
                            && !block_returns(&stmts, true)
                            && !gives_value(&stmts)
                        {
                            Err(Error::new(
                                E0067,
                                span,
                                format!(
                                    "this `catch` block must end with a `{payload}` value or leave with `return`, `break` or `continue`"
                                ),
                            ))
                        } else {
                            Ok(TArmBody::Block(stmts))
                        }
                    }
                    ArmBody::Expr(x) if payload == Type::Void => {
                        self.expr_stmt(x).map(TArmBody::Expr)
                    }
                    ArmBody::Expr(x) => self.expect(x, &payload).map(TArmBody::Expr),
                };
                self.scopes.pop();
                let capture = match capture {
                    Some(v) => Some(v.ok_or_else(silent)?),
                    None => None,
                };
                (
                    TExprKind::Catch {
                        expr: Box::new(t),
                        capture,
                        body: Box::new(body?),
                    },
                    payload,
                )
            }
            ExprKind::Unreachable => (
                TExprKind::Unreachable,
                expected.cloned().unwrap_or(Type::Void),
            ),
            ExprKind::Is {
                expr,
                variant,
                variant_span,
            } => {
                let mut t = self.value(expr, None)?;
                // A pointer to an enum is tested through the pointer.
                if let Type::Ptr(pointee, _) = &t.ty {
                    let (ty, span) = ((**pointee).clone(), t.span);
                    t = TExpr {
                        kind: TExprKind::Deref(Box::new(t)),
                        ty,
                        span,
                    };
                }
                let Type::Enum(ename) = &t.ty else {
                    return err(
                        E0106,
                        span,
                        format!(
                            "`is` tests the variant of an enum value, found `{}`{}",
                            t.ty,
                            if matches!(t.ty, Type::ErrorSet(_)) {
                                " (compare error values with `==`)"
                            } else {
                                ""
                            }
                        ),
                    );
                };
                let def = &self.enums[ename];
                let Some(index) = def.variants.iter().position(|v| v.name == *variant) else {
                    let e = Error::new(
                        E0041,
                        *variant_span,
                        format!("enum `{ename}` has no variant `{variant}`"),
                    );
                    return Err(suggest(
                        e,
                        variant,
                        *variant_span,
                        def.variants.iter().map(|v| v.name.as_str()),
                    ));
                };
                (
                    TExprKind::IsVariant {
                        expr: Box::new(t),
                        index,
                    },
                    Type::Bool,
                )
            }
            ExprKind::Unwrap(inner) => {
                let t = self.value(inner, None)?;
                let Type::Optional(v) = &t.ty else {
                    return err(
                        E0098,
                        span,
                        format!("`.?` needs an optional value (`?T`), found `{}`", t.ty),
                    );
                };
                let ty = (**v).clone();
                (TExprKind::Unwrap(Box::new(t)), ty)
            }
            ExprKind::OrElse { expr, body } => {
                let t = self.expr(expr, None)?;
                let payload = match &t.ty {
                    Type::Optional(v) => (**v).clone(),
                    Type::Error => return Err(silent()),
                    ty => {
                        let hint = if matches!(ty, Type::ErrorUnion(..)) {
                            " (handle an error union with `catch`)"
                        } else {
                            ""
                        };
                        return err(
                            E0098,
                            t.span,
                            format!("`orelse` needs an optional value (`?T`), found `{ty}`{hint}"),
                        );
                    }
                };
                let failures = self.failures;
                // A fallback that is itself a `?T` (`a orelse b orelse c`)
                // makes the result a `?T`.
                let mut ty = payload.clone();
                let body = match &**body {
                    ArmBody::Block(b) => {
                        let stmts = self.value_block(b, &payload);
                        if self.failures == failures
                            && !block_returns(&stmts, true)
                            && !gives_value(&stmts)
                        {
                            Err(Error::new(
                                E0067,
                                span,
                                format!(
                                    "this `orelse` block must end with a `{payload}` value or leave with `return`, `break`, `continue` or `unreachable`"
                                ),
                            ))
                        } else {
                            Ok(TArmBody::Block(stmts))
                        }
                    }
                    ArmBody::Expr(x) if is_null(x) => {
                        ty = t.ty.clone();
                        self.expect(x, &ty).map(TArmBody::Expr)
                    }
                    ArmBody::Expr(x) => match self.value(x, Some(Self::hint(x, &payload))) {
                        Ok(v) if v.ty == t.ty && v.ty != payload => {
                            ty = t.ty.clone();
                            Ok(TArmBody::Expr(v))
                        }
                        Ok(v) => self.coerce(v, &payload).map(TArmBody::Expr),
                        Err(e) => Err(e),
                    },
                };
                (
                    TExprKind::OrElse {
                        expr: Box::new(t),
                        body: Box::new(body?),
                    },
                    ty,
                )
            }
            // `Set.Name`: an error value.
            ExprKind::Field { base, name } if matches!(&base.kind, ExprKind::Ident(n) if self.sets.contains_key(n)) =>
            {
                let ExprKind::Ident(set) = &base.kind else {
                    unreachable!()
                };
                if !self.sets[set].contains(name) {
                    return err(
                        E0060,
                        span,
                        format!("error set `{set}` has no error `{name}`"),
                    );
                }
                let v = self.error_names.iter().position(|n| n == name).unwrap() + 1;
                (TExprKind::ErrorValue(v as u16), Type::ErrorSet(set.clone()))
            }
            // `Enum.Variant` without payload.
            ExprKind::Field { base, name }
                if let ExprKind::Ident(n) = &base.kind
                    && self.lookup(n).is_none()
                    && let Some(ename) = self.enum_name(n) =>
            {
                let ename = &ename;
                let def = &self.enums[ename];
                let Some(index) = def.variants.iter().position(|v| v.name == *name) else {
                    return err(
                        E0041,
                        span,
                        format!("enum `{ename}` has no variant `{name}`"),
                    );
                };
                if !def.variants[index].fields.is_empty() {
                    return err(
                        E0042,
                        span,
                        format!(
                            "variant `{ename}.{name}` has fields; write `{ename}.{name} {{ ... }}`"
                        ),
                    );
                }
                (
                    TExprKind::Variant {
                        index,
                        fields: vec![],
                    },
                    Type::Enum(ename.clone()),
                )
            }
            ExprKind::Field { base, name } => {
                let mut b = self.value(base, None)?;
                // Field access dereferences one pointer level.
                if let Type::Ptr(pointee, _) = &b.ty {
                    let (ty, span) = ((**pointee).clone(), b.span);
                    b = TExpr {
                        kind: TExprKind::Deref(Box::new(b)),
                        ty,
                        span,
                    };
                }
                match &b.ty {
                    Type::Struct(sname) => {
                        let def = &self.structs[sname];
                        let Some(index) = def.fields.iter().position(|(n, _)| n == name) else {
                            if self.recvs.contains_key(&format!("{sname}.{name}")) {
                                return err(
                                    E0038,
                                    span,
                                    format!(
                                        "`{name}` is a method of struct `{sname}`, not a field: call it with `.{name}(...)`"
                                    ),
                                );
                            }
                            let e = Error::new(
                                E0038,
                                span,
                                format!("struct `{sname}` has no field `{name}`"),
                            );
                            return Err(suggest(
                                e,
                                name,
                                span,
                                def.fields.iter().map(|(n, _)| n.as_str()),
                            ));
                        };
                        let ty = def.fields[index].1.clone();
                        (
                            TExprKind::Field {
                                base: Box::new(b),
                                index,
                            },
                            ty,
                        )
                    }
                    Type::Param(_, p) => {
                        return err(
                            E0078,
                            span,
                            format!(
                                "`{p}` is a `comptime` type: its fields are not known (use the methods of a `where` constraint)"
                            ),
                        );
                    }
                    Type::Slice(..) if name == "len" => (TExprKind::SliceLen(Box::new(b)), USIZE),
                    Type::Slice(el, _) if name == "ptr" => {
                        let ty = Type::CPtr(el.clone());
                        (TExprKind::SlicePtr(Box::new(b)), ty)
                    }
                    Type::Array(..) if name == "len" => (TExprKind::ArrayLen(Box::new(b)), USIZE),
                    t => return err(E0038, span, format!("type `{t}` has no field `{name}`")),
                }
            }
            ExprKind::Index { base, index } => {
                let mut b = self.value(base, None)?;
                // Like field access, indexing dereferences a pointer to an array.
                if let Type::Ptr(pointee, _) = &b.ty
                    && let Type::Array(..) = **pointee
                {
                    let (ty, span) = ((**pointee).clone(), b.span);
                    b = TExpr {
                        kind: TExprKind::Deref(Box::new(b)),
                        ty,
                        span,
                    };
                }
                // A C pointer is indexed without a check.
                let elem = match &b.ty {
                    Type::Array(el, _)
                    | Type::Slice(el, _)
                    | Type::CPtr(el)
                    | Type::Simd(el, _) => (**el).clone(),
                    t => {
                        return err(
                            E0043,
                            span,
                            format!("cannot index into a value of type `{t}`"),
                        );
                    }
                };
                let i = self.value(index, Some(&USIZE))?;
                if !is_unsigned(&i.ty) {
                    return err(
                        E0044,
                        i.span,
                        format!("index must be an unsigned integer, found `{}`", i.ty),
                    );
                }
                (
                    TExprKind::Index {
                        base: Box::new(b),
                        index: Box::new(i),
                    },
                    elem,
                )
            }
            ExprKind::Slice { base, lo, hi } => {
                let b = self.value(base, None)?;
                // Slicing an array that could be borrowed with `&mut` gives a
                // `[]mut T`; a slice keeps its type.
                let (elem, mutable) = match &b.ty {
                    Type::Array(el, _) => {
                        if !Self::is_lvalue(&b) {
                            return err(
                                E0035,
                                span,
                                "can only slice an array stored in a variable",
                            );
                        }
                        ((**el).clone(), self.check_mutable(&b, true).is_ok())
                    }
                    Type::Slice(el, m) => ((**el).clone(), *m),
                    // An unchecked C pointer gets a length: `p[0..n]`.
                    Type::CPtr(el) => {
                        if lo.is_none() || hi.is_none() {
                            return err(
                                E0043,
                                span,
                                format!(
                                    "slicing a `{}` needs both bounds (`p[lo..hi]`): it has no length",
                                    b.ty
                                ),
                            );
                        }
                        ((**el).clone(), true)
                    }
                    t => return err(E0043, span, format!("cannot slice a value of type `{t}`")),
                };
                let mut bound = |x: &Option<Box<ast::Expr>>| -> Result<Option<Box<TExpr>>> {
                    match x {
                        None => Ok(None),
                        Some(x) => {
                            let t = self.value(x, Some(&USIZE))?;
                            if !is_unsigned(&t.ty) {
                                return err(
                                    E0044,
                                    t.span,
                                    format!(
                                        "slice bound must be an unsigned integer, found `{}`",
                                        t.ty
                                    ),
                                );
                            }
                            Ok(Some(Box::new(t)))
                        }
                    }
                };
                let lo = bound(lo)?;
                let hi = bound(hi)?;
                (
                    TExprKind::Slice {
                        base: Box::new(b),
                        lo,
                        hi,
                    },
                    Type::Slice(Box::new(elem), mutable),
                )
            }
            ExprKind::StructLit { ty, fields } => {
                let name = &match (&ty.kind, &self.self_ty) {
                    (TypeExprKind::Named(n), Some(s)) if n == "Self" => s.clone(),
                    (TypeExprKind::Named(n), _) if !self.alias_decls.contains_key(n) => n.clone(),
                    _ => match self.resolve_type(ty)? {
                        Type::Struct(n) => n,
                        t => return err(E0017, span, format!("`{t}` is not a struct")),
                    },
                };
                let Some(def) = self.structs.get(name).cloned() else {
                    if self.global_decls.iter().any(|g| g.name == *name) {
                        return err(E0017, span, format!("`{name}` is a constant, not a struct"));
                    }
                    let e = Error::new(E0017, span, format!("unknown struct `{name}`"));
                    let structs = self
                        .structs
                        .keys()
                        .filter(|n| !n.contains(['(', ' ']))
                        .map(String::as_str);
                    return Err(suggest(e, name, span, structs));
                };
                let inits = self.field_inits(
                    &def.fields,
                    fields,
                    &format!("struct `{name}`"),
                    name,
                    Some(name),
                    span,
                )?;
                (TExprKind::StructLit(inits), Type::Struct(name.clone()))
            }
            ExprKind::VariantLit {
                enum_name,
                variant,
                fields,
            } => {
                let Some(def) = self.enum_name(enum_name).map(|n| self.enums[&n].clone()) else {
                    return err(E0017, span, format!("unknown enum `{enum_name}`"));
                };
                let enum_name = &def.name;
                let Some(index) = def.variants.iter().position(|v| v.name == *variant) else {
                    return err(
                        E0041,
                        span,
                        format!("enum `{enum_name}` has no variant `{variant}`"),
                    );
                };
                let v = &def.variants[index];
                let lit = format!("{enum_name}.{variant}");
                if v.fields.is_empty() {
                    return err(
                        E0042,
                        span,
                        format!("variant `{lit}` has no fields; write `{lit}`"),
                    );
                }
                let inits = self.field_inits(
                    &v.fields,
                    fields,
                    &format!("variant `{lit}`"),
                    &lit,
                    None,
                    span,
                )?;
                (
                    TExprKind::Variant {
                        index,
                        fields: inits,
                    },
                    Type::Enum(enum_name.clone()),
                )
            }
            ExprKind::ArrayRepeat { ty, value } => {
                let aty = self.resolve_type(ty)?;
                let (Type::Array(el, _) | Type::Simd(el, _)) = &aty else {
                    unreachable!()
                };
                let v = self.expect(value, el)?;
                (TExprKind::ArrayRepeat(Box::new(v)), aty)
            }
            ExprKind::ArrayLit { ty, elems } => {
                let aty = self.resolve_type(ty)?;
                if let Type::Slice(..) = aty {
                    if !elems.is_empty() {
                        return err(
                            E0045,
                            span,
                            "a slice literal must be empty (`[]T{}`): put the elements in an array (`[N]T{...}`) and slice it",
                        );
                    }
                    return Ok(TExpr {
                        kind: TExprKind::EmptySlice,
                        ty: aty,
                        span,
                    });
                }
                let (Type::Array(el, n) | Type::Simd(el, n)) = &aty else {
                    unreachable!()
                };
                if elems.len() as u64 != *n {
                    let what = if matches!(aty, Type::Simd(..)) {
                        "`simd` literal"
                    } else {
                        "array literal"
                    };
                    return err(
                        E0045,
                        span,
                        format!(
                            "{what} has {} element(s) but type `{aty}` needs {n}",
                            elems.len()
                        ),
                    );
                }
                let elems = elems
                    .iter()
                    .map(|x| self.expect(x, el))
                    .collect::<Result<_>>()?;
                (TExprKind::ArrayLit(elems), aty)
            }
        };
        Ok(TExpr { kind, ty, span })
    }

    /// `std.log.err(format, args...)`, `std.log.info(...)` or
    /// `std.log.line(...)`: `format` is a
    /// string literal or a constant (top-level `let`) that is one, and each
    /// `{}` in it takes the next argument, an integer, float, `bool`, `[]u8`
    /// or error value.
    fn log(&mut self, prefix: LogPrefix, args: &[ast::Expr], span: Span) -> Result<TExpr> {
        let f = match prefix {
            LogPrefix::Err => "std.log.err",
            LogPrefix::Info => "std.log.info",
            LogPrefix::None => "std.log.line",
        };
        let Some(first) = args.first() else {
            return err(
                E0086,
                span,
                format!("`{f}` needs a format string: `{f}(\"text {{}}\", value)`"),
            );
        };
        let not_constant = || {
            err(
                E0086,
                first.span,
                format!(
                    "the format of `{f}` must be a string literal or constant (print a value with `{f}(\"{{}}\", value)`)"
                ),
            )
        };
        let format = match &strip_parens(first).kind {
            ExprKind::Str(s) => s.clone(),
            ExprKind::Ident(n)
                if self.lookup(n).is_none() && self.global_decls.iter().any(|g| g.name == *n) =>
            {
                let (_, mutable) = self.global(n, first.span)?;
                match &self.globals[n].value {
                    Some(ConstVal::Str(s)) if !mutable => s.clone(),
                    None => return Err(silent()),
                    _ => return not_constant(),
                }
            }
            _ => return not_constant(),
        };
        let mut pieces = vec![Vec::new()];
        let mut i = 0;
        while i < format.len() {
            if format[i..].starts_with(b"{}") {
                pieces.push(Vec::new());
                i += 2;
            } else {
                pieces.last_mut().unwrap().push(format[i]);
                i += 1;
            }
        }
        if pieces.len() != args.len() {
            let n = pieces.len() - 1;
            return err(
                E0086,
                span,
                format!(
                    "the format of `{f}` has {n} `{{}}` but {} argument(s) follow it",
                    args.len() - 1
                ),
            );
        }
        let mut targs = Vec::new();
        for a in &args[1..] {
            let t = self.value(a, None)?;
            let ok = match &t.ty {
                Type::Int(_) | Type::Float(_) | Type::Bool | Type::ErrorSet(_) => true,
                Type::Slice(el, _) => **el == Type::Int(IntTy::U8),
                _ => false,
            };
            if !ok {
                return err(
                    E0086,
                    t.span,
                    format!(
                        "`{f}` cannot format a value of type `{}` (integers, floats, `bool`, `[]u8` and errors can be formatted)",
                        t.ty
                    ),
                );
            }
            targs.push(t);
        }
        Ok(TExpr {
            kind: TExprKind::Log {
                prefix,
                pieces,
                args: targs,
            },
            ty: Type::Void,
            span,
        })
    }

    /// Does `name` name a type (not a value)?
    fn is_type_name(&self, name: &str) -> bool {
        Type::from_name(name, self.layout).is_some()
            || name == "Self"
            || self.env.iter().any(|(n, ..)| n == name)
            || self.alias_decls.contains_key(name)
            || self.structs.contains_key(name)
            || self.enums.contains_key(name)
            || self.sets.contains_key(name)
            || self.ifaces.contains_key(name)
    }

    /// A call of the function `key` (`f` or `Struct.method`). `recv` is the
    /// receiver of a method call, passed as the first argument.
    fn call(
        &mut self,
        key: &str,
        recv: Option<TExpr>,
        args: &[ast::Expr],
        span: Span,
    ) -> Result<TExpr> {
        let sig = self.fns[key].clone();
        // An instance of a generic for concrete types calls the methods of
        // its type arguments through their `where` constraints, which are
        // structural, like `dyn`: they need no `pub`. Its other calls were
        // checked with the generic itself.
        let instance = !self.env.is_empty() && !self.env.iter().any(|(_, t, _)| self.has_param(t));
        if let Some(ctx) = self.fn_decls.get(key)
            && !ctx.decl.is_pub
            && !instance
            && ctx.module(key) != self.module
        {
            let (module, what) = (
                ctx.module(key),
                if ctx.owner.is_some() { "method " } else { "" },
            );
            return err(
                E0082,
                span,
                format!("{what}`{key}` is not `pub` in module `{module}`"),
            );
        }
        if let Type::ErrorUnion(set, _) = &sig.ret
            && set.starts_with(INFERRED)
            && !sig.is_extern
        {
            self.ensure_body(key, span)?;
        }
        let skip = recv.is_some() as usize;
        let fixed = sig.params.len();
        if args.len() + skip != fixed && !(sig.variadic && args.len() + skip > fixed) {
            return err(
                E0029,
                span,
                format!(
                    "function `{key}` takes {}{} argument(s) but {} were given",
                    if sig.variadic { "at least " } else { "" },
                    fixed - skip,
                    args.len()
                ),
            );
        }
        let mut targs = Vec::new();
        if let Some(r) = recv {
            targs.push(self.coerce(r, &sig.params[0])?);
        }
        for (a, p) in args.iter().zip(&sig.params[skip..]) {
            targs.push(self.expect(a, p)?);
        }
        for a in &args[fixed - skip..] {
            targs.push(self.c_vararg(a)?);
        }
        Ok(TExpr {
            kind: TExprKind::Call {
                name: sig.c_key(key),
                is_extern: sig.is_extern,
                args: targs,
            },
            ty: sig.ret,
            span,
        })
    }

    /// `base.name(args)`: a static call `Type.name(args)`, a method call on a
    /// struct, a call through a `dyn`, or an allocator method.
    fn method_call(
        &mut self,
        base: &ast::Expr,
        name: &str,
        args: &[ast::Expr],
        span: Span,
    ) -> Result<TExpr> {
        // A type: a type name, a `comptime` parameter, or `F(T)`.
        let static_ty = match &base.kind {
            ExprKind::Ident(tn) if self.lookup(tn).is_none() && self.is_type_name(tn) => {
                if let Some(Type::Builtin(b)) = Type::from_name(tn, self.layout) {
                    return self.builtin_static(b, name, args, span);
                }
                match (
                    self.env.iter().find(|(n, ..)| n == tn),
                    tn.as_str(),
                    &self.self_ty,
                ) {
                    (Some((_, t, _)), ..) => Some(t.clone()),
                    (None, "Self", Some(s)) => Some(self.named(s)),
                    _ if self.alias_decls.contains_key(tn) => {
                        let t = self.alias(tn, base.span)?;
                        if let Type::Builtin(b) = t {
                            return self.builtin_static(b, name, args, span);
                        }
                        Some(t)
                    }
                    _ => Some(self.named(tn)),
                }
            }
            ExprKind::Call { name: g, .. }
                if self.generics.get(g).is_some_and(|f| is_type_fn(f)) =>
            {
                Some(self.type_arg(base, None)?)
            }
            _ => None,
        };
        if let Some(t) = static_ty {
            if let Type::Param(..) = t {
                return err(
                    E0078,
                    span,
                    format!(
                        "`{t}` is a `comptime` type: it has only the methods of its `where` constraints, called on a value (`x.{name}()`)"
                    ),
                );
            }
            let tn = match &t {
                Type::Struct(n) | Type::Enum(n) => n.clone(),
                t => t.to_string(),
            };
            let key = format!("{tn}.{name}");
            let what = match t {
                Type::Struct(_) if self.structs.contains_key(&tn) => "struct",
                Type::Enum(_) => "enum",
                _ => "type",
            };
            if what == "type" || !self.fns.contains_key(&key) {
                return err(
                    E0071,
                    span,
                    format!("{what} `{}` has no method `{name}`", t),
                );
            }
            return self.call(&key, None, args, span);
        }
        let recv = self.value(base, None)?;
        let inner = match &recv.ty {
            Type::Ptr(t, _) => (**t).clone(),
            t => t.clone(),
        };
        let deref = |r: TExpr| match &r.ty {
            Type::Ptr(t, _) => {
                let (ty, span) = ((**t).clone(), r.span);
                TExpr {
                    kind: TExprKind::Deref(Box::new(r)),
                    ty,
                    span,
                }
            }
            _ => r,
        };
        match inner {
            Type::Struct(s) | Type::Enum(s) => {
                let key = format!("{s}.{name}");
                let Some(&r) = self.recvs.get(&key) else {
                    let Some(def) = self.structs.get(&s) else {
                        return err(E0071, span, format!("enum `{s}` has no method `{name}`"));
                    };
                    // `s.f(args)` calls the function in the field `f`.
                    let fn_field = def.fields.iter().position(|(f, t)| {
                        f == name
                            && (matches!(t, Type::Fn(..))
                                || matches!(t, Type::Optional(i) if matches!(**i, Type::Fn(..))))
                    });
                    if let Some(index) = fn_field {
                        let ty = def.fields[index].1.clone();
                        let field = TExpr {
                            kind: TExprKind::Field {
                                base: Box::new(deref(recv)),
                                index,
                            },
                            ty,
                            span,
                        };
                        return self.call_value(field, Some(name), args, span);
                    }
                    let hint = if def.fields.iter().any(|(f, _)| f == name) {
                        " (it has a field of that name)"
                    } else {
                        ""
                    };
                    return err(
                        E0071,
                        span,
                        format!("struct `{s}` has no method `{name}`{hint}"),
                    );
                };
                let recv = match r {
                    Recv::Static => {
                        return err(
                            E0073,
                            span,
                            format!("`{key}` has no `self` parameter: call it as `{key}(...)`"),
                        );
                    }
                    Recv::Value => deref(recv),
                    Recv::Ptr(m) => self.receiver_ptr(recv, m, name)?,
                };
                self.call(&key, Some(recv), args, span)
            }
            Type::Dyn(iface) => {
                let recv = deref(recv);
                let Some(m) = self.ifaces[&iface]
                    .methods
                    .iter()
                    .find(|m| m.name == name)
                    .cloned()
                else {
                    return err(
                        E0071,
                        span,
                        format!("interface `{iface}` has no method `{name}`"),
                    );
                };
                if args.len() != m.params.len() {
                    return err(
                        E0029,
                        span,
                        format!(
                            "method `{iface}.{name}` takes {} argument(s) but {} were given",
                            m.params.len(),
                            args.len()
                        ),
                    );
                }
                let args = args
                    .iter()
                    .zip(&m.params)
                    .map(|(a, p)| self.expect(a, p))
                    .collect::<Result<_>>()?;
                let kind = TExprKind::DynCall {
                    recv: Box::new(recv),
                    method: name.to_string(),
                    args,
                };
                Ok(TExpr {
                    kind,
                    ty: m.ret,
                    span,
                })
            }
            Type::Builtin(b) => self.builtin_method(b, recv, name, args, span),
            // While a generic is checked: a method of a `where` constraint.
            Type::Param(g, p) => {
                let ifaces = self
                    .bounds
                    .get(&(g, p.clone()))
                    .cloned()
                    .unwrap_or_default();
                let found = ifaces
                    .iter()
                    .filter_map(|i| self.ifaces.get(i))
                    .find_map(|i| i.methods.iter().find(|m| m.name == name))
                    .cloned();
                let Some(m) = found else {
                    if ifaces.is_empty() {
                        return err(
                            E0078,
                            span,
                            format!(
                                "`{p}` has no method `{name}`: a `comptime` type has only the methods of its `where` constraints (add `where {p}: SomeInterface`)"
                            ),
                        );
                    }
                    return err(
                        E0078,
                        span,
                        format!(
                            "`{p}` has no method `{name}`: its constraints (`{}`) do not declare it",
                            ifaces.join("`, `")
                        ),
                    );
                };
                let recv = self.receiver_ptr(recv, m.mutable, name)?;
                if args.len() != m.params.len() {
                    return err(
                        E0029,
                        span,
                        format!(
                            "method `{p}.{name}` takes {} argument(s) but {} were given",
                            m.params.len(),
                            args.len()
                        ),
                    );
                }
                let mut targs = vec![recv];
                for (a, t) in args.iter().zip(&m.params) {
                    targs.push(self.expect(a, t)?);
                }
                Ok(TExpr {
                    kind: TExprKind::Call {
                        name: format!("{p}.{name}"),
                        is_extern: false,
                        args: targs,
                    },
                    ty: m.ret,
                    span,
                })
            }
            t => err(E0071, span, format!("type `{t}` has no method `{name}`")),
        }
    }

    /// The type that `e` names, as the argument of the `comptime` parameter
    /// `param`, or as a type function call `F(...)` (`param` is `None`).
    fn type_arg(&mut self, e: &ast::Expr, param: Option<&str>) -> Result<Type> {
        let local = matches!(&e.kind, ExprKind::Ident(n) if self.lookup(n).is_some());
        match expr_type(e) {
            Some(t) if !local => self.resolve_type(&t),
            _ => match param {
                Some(p) => err(
                    E0080,
                    e.span,
                    format!("expected a type for `comptime {p}`, found an expression"),
                ),
                None => err(E0080, e.span, "expected a type, found an expression"),
            },
        }
    }

    /// A call of the generic function `f`. The arguments of its `comptime`
    /// parameters are types; they select the instance that is called with
    /// the other arguments.
    fn generic_call(
        &mut self,
        f: &'a ast::FnDecl,
        args: &[ast::Expr],
        span: Span,
    ) -> Result<TExpr> {
        if args.len() != f.params.len() {
            return err(
                E0029,
                span,
                format!(
                    "function `{}` takes {} argument(s) but {} were given",
                    f.name,
                    f.params.len(),
                    args.len()
                ),
            );
        }
        let mut types = Vec::new();
        let mut values = Vec::new();
        for (p, a) in f.params.iter().zip(args) {
            if p.comptime {
                types.push(self.type_arg(a, Some(&p.name))?);
            } else {
                values.push(a.clone());
            }
        }
        let key = self.instantiate_fn(&f.name, types, span)?;
        self.call(&key, None, &values, span)
    }

    /// A call of the function value `callee`; `what` names it in messages.
    fn call_value(
        &mut self,
        callee: TExpr,
        what: Option<&str>,
        args: &[ast::Expr],
        span: Span,
    ) -> Result<TExpr> {
        let shown = what.map_or("this value".to_string(), |w| format!("`{w}`"));
        let (params, ret) = match &callee.ty {
            Type::Fn(params, ret) => (params.clone(), (**ret).clone()),
            Type::Optional(inner) if matches!(**inner, Type::Fn(..)) => {
                return err(
                    E0095,
                    span,
                    format!(
                        "cannot call {shown}: its type is `{}`, an optional (unwrap it first: `if {} |f| {{ f() }}`)",
                        callee.ty,
                        what.unwrap_or("x")
                    ),
                );
            }
            t => {
                return err(
                    E0095,
                    span,
                    format!("cannot call {shown}: its type is `{t}`, not a function type"),
                );
            }
        };
        if args.len() != params.len() {
            return err(
                E0029,
                span,
                format!(
                    "function value {}of type `{}` takes {} argument(s) but {} were given",
                    what.map_or(String::new(), |w| format!("`{w}` ")),
                    callee.ty,
                    params.len(),
                    args.len()
                ),
            );
        }
        let args = args
            .iter()
            .zip(&params)
            .map(|(a, p)| self.expect(a, p))
            .collect::<Result<_>>()?;
        Ok(TExpr {
            kind: TExprKind::CallValue {
                callee: Box::new(callee),
                args,
            },
            ty: ret,
            span,
        })
    }

    /// An argument for the `...` of a C function. Its type must be
    /// C-compatible; an untyped integer literal is a C `int` (`i32`). C
    /// promotes the value as for any `...` (`f32` to `double`, smaller
    /// integers and `bool` to `int`).
    fn c_vararg(&mut self, a: &ast::Expr) -> Result<TExpr> {
        let mut t = self.value(a, None)?;
        if is_untyped(a) && t.ty == I64 {
            t = self
                .value(a, Some(&Type::Int(IntTy::I32)))
                .map_err(|mut e| {
                    if e.code == E0024 {
                        e.msg += " (an integer literal passed to `...` is a C `int`: add `as i64`)";
                    }
                    e
                })?;
        }
        if !self.c_compat(&t.ty, false) {
            return err(
                E0052,
                t.span,
                format!(
                    "`{}` cannot be passed to the `...` of a C function: it is not C-compatible{}",
                    t.ty,
                    self.c_hint(&t.ty, false)
                ),
            );
        }
        Ok(t)
    }

    /// Can a value of type `t` cross into C with the same layout? An array
    /// only as a field (`field`): C passes an array argument by pointer.
    fn c_compat(&self, t: &Type, field: bool) -> bool {
        match t {
            Type::Int(_) | Type::Float(_) | Type::Bool | Type::Error => true,
            Type::Struct(n) => self.extern_structs.contains(n),
            Type::Array(e, _) => field && self.c_compat(e, true),
            Type::CPtr(e) | Type::Ptr(e, _) => self.c_compat(e, true),
            Type::Optional(e) => {
                matches!(**e, Type::CPtr(_) | Type::Ptr(..) | Type::Fn(..))
                    && self.c_compat(e, false)
            }
            Type::Fn(params, ret) => {
                params.iter().all(|p| self.c_compat(p, false))
                    && (**ret == Type::Void || self.c_compat(ret, false))
            }
            _ => false,
        }
    }

    /// Why `t` is not C-compatible, for a message (`field` as for `c_compat`).
    fn c_hint(&self, t: &Type, field: bool) -> String {
        match t {
            Type::Slice(el, _) => {
                format!(" (a slice is a pointer and a length: use `[*]{el}` and a `usize`)")
            }
            Type::Struct(n) if !n.contains('(') && !self.extern_structs.contains(n) => {
                format!(" (declare it `extern struct {n}` to give it C layout)")
            }
            Type::Array(el, _) if !field => {
                format!(" (C passes an array by pointer: use `*{t}` or `[*]{el}`)")
            }
            Type::Optional(_) => {
                " (only optional pointers are: `?*T`, `?[*]T` and `?fn(...)`)".to_string()
            }
            Type::Fn(..) => {
                " (the parameters and result of a function that C calls must be C-compatible)"
                    .to_string()
            }
            _ => String::new(),
        }
    }

    /// `@atomicLoad(p)`, `@atomicStore(p, v)`, `@atomicAdd(p, v)`,
    /// `@atomicSub(p, v)` and `@atomicCas(p, expected, new)` on a pointer to
    /// an integer (`*mut` to write).
    fn atomic(&mut self, name: &str, args: &[ast::Expr], span: Span) -> Result<TExpr> {
        let (op, n) = match name {
            "atomicLoad" => (Intrinsic::AtomicLoad, 1),
            "atomicStore" => (Intrinsic::AtomicStore, 2),
            "atomicAdd" => (Intrinsic::AtomicAdd, 2),
            "atomicSub" => (Intrinsic::AtomicSub, 2),
            "atomicCas" => (Intrinsic::AtomicCas, 3),
            _ => return err(E0021, span, format!("unknown builtin `@{name}`")),
        };
        if args.len() != n {
            let what = [
                "",
                "one argument, a pointer",
                "two arguments, a pointer and a value",
                "three arguments: a pointer, the expected value and the new one",
            ][n];
            return err(E0022, span, format!("`@{name}` takes {what}"));
        }
        let p = self.value(&args[0], None)?;
        let (elem, mutable) = match &p.ty {
            Type::Ptr(e, m) => ((**e).clone(), *m),
            t => {
                return err(
                    E0101,
                    p.span,
                    format!("`@{name}` needs a pointer to an integer (`&mut x`), found `{t}`"),
                );
            }
        };
        if !(elem.is_int() || self.integer_param(&elem)) {
            return err(
                E0101,
                p.span,
                format!("`@{name}` works on integers, found a pointer to `{elem}`"),
            );
        }
        if op != Intrinsic::AtomicLoad && !mutable {
            return err(
                E0101,
                p.span,
                format!(
                    "`@{name}` writes, so it needs a `*mut {elem}` (`&mut x`), found `{}`",
                    p.ty
                ),
            );
        }
        let mut targs = vec![p];
        for a in &args[1..] {
            targs.push(self.expect(a, &elem)?);
        }
        let ty = match op {
            Intrinsic::AtomicStore => Type::Void,
            Intrinsic::AtomicCas => Type::Bool,
            _ => elem,
        };
        Ok(TExpr {
            kind: TExprKind::Intrinsic { op, args: targs },
            ty,
            span,
        })
    }

    /// `@splat(simd[N]T, x)` (every lane `x`), `@reduceAdd(v)`,
    /// `@reduceMin(v)`, `@reduceMax(v)` (a lane type), `@all(m)`, `@any(m)`
    /// (a `bool` from a mask) and `@select(m, a, b)` (lane `i` from `a` where
    /// `m[i]`, else from `b`).
    fn simd_builtin(&mut self, name: &str, args: &[ast::Expr], span: Span) -> Result<TExpr> {
        let want = match name {
            "splat" => "two arguments: a `simd` type and the value of every lane",
            "select" => "three arguments: a mask and two vectors of the same type",
            "all" | "any" => "one argument, a mask (`simd[N]bool`)",
            _ => "one argument, a `simd` of numbers",
        };
        let n = match name {
            "splat" => 2,
            "select" => 3,
            _ => 1,
        };
        if args.len() != n {
            return err(E0022, span, format!("`@{name}` takes {want}"));
        }
        if name == "splat" {
            let local = matches!(&args[0].kind, ExprKind::Ident(n) if self.lookup(n).is_some());
            let ty = match expr_type(&args[0]) {
                Some(t) if !local => self.resolve_type(&t)?,
                _ => {
                    return err(
                        E0022,
                        args[0].span,
                        "the first argument of `@splat` must be a `simd` type, such as `simd[4]f32`",
                    );
                }
            };
            let Type::Simd(el, _) = &ty else {
                return err(
                    E0022,
                    args[0].span,
                    format!("`@splat` makes a `simd`, not a `{ty}`"),
                );
            };
            let v = self.expect(&args[1], &el.clone())?;
            return Ok(TExpr {
                kind: TExprKind::ArrayRepeat(Box::new(v)),
                ty,
                span,
            });
        }
        let v = self.value(&args[0], None)?;
        let el = match &v.ty {
            Type::Simd(el, _) => (**el).clone(),
            t => {
                return err(
                    E0022,
                    v.span,
                    format!("`@{name}` takes {want}, found `{t}`"),
                );
            }
        };
        let mask = el == Type::Bool;
        if matches!(name, "all" | "any" | "select") != mask {
            return err(
                E0022,
                v.span,
                format!("`@{name}` takes {want}, found `{}`", v.ty),
            );
        }
        let (op, ty, args) = match name {
            "all" => (Intrinsic::All, Type::Bool, vec![v]),
            "any" => (Intrinsic::Any, Type::Bool, vec![v]),
            "select" => {
                let (a, b) = self.operand_pair(&args[1], &args[2], None)?;
                if a.ty != b.ty {
                    return Err(mismatch(b.span, &a.ty, &b.ty));
                }
                match &a.ty {
                    Type::Simd(_, n) if Type::Simd(Box::new(Type::Bool), *n) == v.ty => {}
                    t => {
                        return err(
                            E0022,
                            a.span,
                            format!(
                                "`@select` with a `{}` chooses between two `simd[{}]T`, found `{t}`",
                                v.ty,
                                v_lanes(&v.ty)
                            ),
                        );
                    }
                }
                let ty = a.ty.clone();
                (Intrinsic::Select, ty, vec![v, a, b])
            }
            "reduceAdd" => (Intrinsic::ReduceAdd, el, vec![v]),
            "reduceMin" => (Intrinsic::ReduceMin, el, vec![v]),
            _ => (Intrinsic::ReduceMax, el, vec![v]),
        };
        Ok(TExpr {
            kind: TExprKind::Intrinsic { op, args },
            ty,
            span,
        })
    }

    /// `@sizeOf(T)`, `@zeroed(T)` and `@ptrCast(T, p)`. `@link` belongs at
    /// the top level.
    fn c_builtin(&mut self, name: &str, args: &[ast::Expr], span: Span) -> Result<TExpr> {
        if name == "link" || name == "linkOptional" {
            return err(
                E0094,
                span,
                format!("`@{name}` must be at the top level of a file, outside functions"),
            );
        }
        let cast = name == "ptrCast";
        if args.len() != 1 + cast as usize {
            let msg = if cast {
                "`@ptrCast` takes two arguments: the type and the pointer".to_string()
            } else {
                format!("`@{name}` takes one argument, a type")
            };
            return err(E0022, span, msg);
        }
        let local = matches!(&args[0].kind, ExprKind::Ident(n) if self.lookup(n).is_some());
        let ty = match expr_type(&args[0]) {
            Some(t) if !local => self.resolve_type(&t)?,
            _ if cast => {
                return err(
                    E0022,
                    args[0].span,
                    "the first argument of `@ptrCast` must be a type",
                );
            }
            _ => {
                return err(
                    E0022,
                    args[0].span,
                    format!("the argument of `@{name}` must be a type, such as `@{name}(i32)`"),
                );
            }
        };
        match name {
            "sizeOf" => Ok(TExpr {
                kind: TExprKind::SizeOf(ty),
                ty: USIZE,
                span,
            }),
            "zeroed" => {
                if let Some(why) = self.why_not_zeroable(&ty) {
                    return err(
                        E0093,
                        args[0].span,
                        format!("`@zeroed` cannot make a `{ty}`: {why}"),
                    );
                }
                Ok(TExpr {
                    kind: TExprKind::Zeroed,
                    ty,
                    span,
                })
            }
            _ => self.ptr_cast(ty, args[0].span, &args[1], span),
        }
    }

    /// Why `@zeroed(t)` cannot make a `t`, or `None`: numbers, `bool`,
    /// optional pointers, `extern struct`s and arrays of them have an
    /// all-zero value.
    fn why_not_zeroable(&self, t: &Type) -> Option<String> {
        match t {
            Type::Int(_) | Type::Float(_) | Type::Bool => None,
            Type::Optional(e) if matches!(**e, Type::Ptr(..) | Type::CPtr(_) | Type::Fn(..)) => None,
            Type::Array(e, _) => self.why_not_zeroable(e),
            Type::Struct(n) if self.extern_structs.contains(n) => self.structs[n].fields.iter().find_map(|(f, ft)| {
                let why = self.why_not_zeroable(ft)?;
                Some(match ft {
                    Type::Ptr(..) | Type::CPtr(_) | Type::Fn(..) => {
                        format!("its field `{f}` has type `{ft}`, which cannot be null (use `?{ft}`)")
                    }
                    _ => format!("its field `{f}` has type `{ft}`: {why}"),
                })
            }),
            Type::Struct(_) => Some("only an `extern struct` has a defined all-zero value".to_string()),
            Type::Ptr(..) | Type::CPtr(_) | Type::Fn(..) => Some(format!("it cannot be null (use `?{t}`)")),
            _ => Some("it makes numbers, `bool`, optional pointers, `extern struct`s and arrays of them".to_string()),
        }
    }

    /// `@embedFile("path")`: the bytes of the file at `path`, relative to
    /// the directory of the source file that names it, read now.
    fn embed_file(&self, args: &[ast::Expr], span: Span) -> Result<Vec<u8>> {
        let [arg] = args else {
            return err(
                E0022,
                span,
                "`@embedFile` takes one argument, a string literal: the path of the file",
            );
        };
        let ExprKind::Str(path) = &strip_parens(arg).kind else {
            return err(
                E0111,
                arg.span,
                "`@embedFile` takes a string literal, the path of the file: `@embedFile(\"ua.css\")`",
            );
        };
        let path = String::from_utf8_lossy(path).into_owned();
        if std::path::Path::new(&path).is_absolute() {
            return err(
                E0111,
                arg.span,
                format!(
                    "`@embedFile` takes a path relative to the source file, not the absolute `{path}` (the build would depend on this machine)"
                ),
            );
        }
        let file = self
            .files
            .get(span.file as usize)
            .map_or("", String::as_str);
        let full = std::path::Path::new(file)
            .parent()
            .unwrap_or(std::path::Path::new(""))
            .join(&path);
        let shown = full.to_string_lossy().into_owned();
        if full.is_dir() {
            return err(
                E0111,
                span,
                format!("`@embedFile` cannot read `{shown}`: it is a directory"),
            );
        }
        std::fs::read(&full).or_else(|e| match e.kind() {
            std::io::ErrorKind::NotFound => err(E0111, span, format!(
                "`@embedFile` cannot read `{shown}`: no such file (the path is relative to the directory of this source file)"
            )),
            _ => err(E0111, span, format!("`@embedFile` cannot read `{shown}`: {e}")),
        })
    }

    /// `@downcast(*S, d)` or `@downcast(*mut S, d)`: the `dyn I` value `d`
    /// as a `?*S` (`?*mut S`), `null` unless `d` was made from a pointer to
    /// an `S`. `*mut S` needs one of `S`'s methods for `I` to take `self:
    /// *mut Self`, which proves the `dyn` was made from `&mut`.
    fn downcast(&mut self, args: &[ast::Expr], span: Span) -> Result<TExpr> {
        let [to, arg] = args else {
            return err(
                E0022,
                span,
                "`@downcast` takes two arguments: the pointer type and the `dyn` value",
            );
        };
        let local = matches!(&to.kind, ExprKind::Ident(n) if self.lookup(n).is_some());
        let ty = match expr_type(to) {
            Some(t) if !local => self.resolve_type(&t)?,
            _ => {
                return err(
                    E0110,
                    to.span,
                    "the first argument of `@downcast` must be a pointer type, such as `*mut S`",
                );
            }
        };
        let (s, mutable) = match &ty {
            Type::Ptr(inner, m) if matches!(&**inner, Type::Struct(_)) => {
                let Type::Struct(s) = &**inner else {
                    unreachable!()
                };
                (s.clone(), *m)
            }
            _ => {
                return err(
                    E0110,
                    to.span,
                    format!(
                        "`@downcast` makes a pointer to a struct (`*S` or `*mut S`), not `{ty}`"
                    ),
                );
            }
        };
        let d = self.value(arg, None)?;
        let iface = match &d.ty {
            Type::Dyn(i) => i.clone(),
            Type::Optional(inner) if matches!(&**inner, Type::Dyn(_)) => {
                return err(
                    E0110,
                    d.span,
                    format!(
                        "`@downcast` takes a `dyn` value, found `{}` (unwrap it first: `opt.?`, or `if opt |d| {{ ... }}`)",
                        d.ty
                    ),
                );
            }
            t => {
                return err(
                    E0110,
                    d.span,
                    format!("`@downcast` takes a `dyn` value, found `{t}`"),
                );
            }
        };
        if let Some(why) = self.why_not(&s, &iface) {
            return err(
                E0110,
                span,
                format!(
                    "`@downcast({ty}, ...)` of a `dyn {iface}` is always `null`: `{s}` does not implement `{iface}` ({why})"
                ),
            );
        }
        if mutable
            && !self.ifaces[&iface]
                .methods
                .iter()
                .any(|m| self.recvs.get(&format!("{s}.{}", m.name)) == Some(&Recv::Ptr(true)))
        {
            return err(
                E0110,
                span,
                format!(
                    "`@downcast({ty}, ...)` of a `dyn {iface}`: the `dyn` may have been made from a read-only `&x`, since none of `{s}`'s `{iface}` methods takes `self: *mut Self` (downcast to `*{s}`)"
                ),
            );
        }
        let pair = (iface.clone(), s.clone());
        if !self.vtables.contains(&pair) {
            self.vtables.push(pair);
        }
        let ty = Type::Optional(Box::new(ty));
        Ok(TExpr {
            kind: TExprKind::Downcast {
                value: Box::new(d),
                iface,
                strukt: s,
            },
            ty,
            span,
        })
    }

    /// `@ptrCast(T, p)`: the pointer `p` (`*T`, `*mut T`, `[*]T` or `?` of
    /// one) as the pointer type `to` (written at `to_span`). It cannot make
    /// a read-only `*T` writable, or an optional pointer non-optional.
    fn ptr_cast(&mut self, to: Type, to_span: Span, arg: &ast::Expr, span: Span) -> Result<TExpr> {
        // Is the pointer type optional, and can it write?
        fn kind(t: &Type) -> Option<(bool, bool)> {
            match t {
                Type::Ptr(_, m) => Some((false, *m)),
                Type::CPtr(_) => Some((false, true)),
                Type::Optional(e) => kind(e).filter(|(opt, _)| !opt).map(|(_, w)| (true, w)),
                _ => None,
            }
        }
        // A function to another function type: the C function pointer is
        // called with other parameter types (`fn(*mut Job)` as `fn([*]u8)`).
        if let Type::Fn(..) = to {
            let f = self.value(arg, None)?;
            if !matches!(f.ty, Type::Fn(..)) {
                return err(
                    E0092,
                    f.span,
                    format!(
                        "`@ptrCast` to a function type converts a function, found `{}`",
                        f.ty
                    ),
                );
            }
            return Ok(TExpr {
                kind: TExprKind::PtrCast(Box::new(f)),
                ty: to,
                span,
            });
        }
        let Some((to_opt, _)) = kind(&to) else {
            return err(
                E0092,
                to_span,
                format!(
                    "`@ptrCast` converts to a pointer type (`*T`, `*mut T`, `[*]T` or `?` of one) or between function types, not to `{to}`"
                ),
            );
        };
        let p = self.value(arg, None)?;
        let Some((from_opt, from_writable)) = kind(&p.ty) else {
            return err(
                E0092,
                p.span,
                format!(
                    "`@ptrCast` converts a pointer, found `{}` (convert numbers with `as`)",
                    p.ty
                ),
            );
        };
        if from_opt && !to_opt {
            return err(
                E0092,
                span,
                format!(
                    "`@ptrCast` cannot turn the optional `{}` into `{to}`: unwrap it first (`if p |v| {{ ... }}`)",
                    p.ty
                ),
            );
        }
        let to_mut = matches!(&to, Type::Ptr(_, true))
            || matches!(&to, Type::Optional(e) if matches!(**e, Type::Ptr(_, true)));
        if !from_writable && to_mut {
            return err(
                E0092,
                span,
                format!(
                    "`@ptrCast` cannot make the read-only pointer `{}` writable (`{to}`)",
                    p.ty
                ),
            );
        }
        Ok(TExpr {
            kind: TExprKind::PtrCast(Box::new(p)),
            ty: to,
            span,
        })
    }

    /// The receiver `recv` of a method that takes `self: *Self` or (with
    /// `mutable`) `self: *mut Self`: a pointer is passed as is, a place as
    /// `&place` or `&mut place` (it must be mutable for `*mut Self`), and a
    /// temporary as a pointer to a copy of it.
    fn receiver_ptr(&self, recv: TExpr, mutable: bool, method: &str) -> Result<TExpr> {
        match &recv.ty {
            Type::Ptr(t, false) if mutable => err(
                E0033,
                recv.span,
                format!(
                    "method `{method}` takes `self: *mut Self`, but the receiver is a read-only pointer `{}` (use `*mut {t}`)",
                    recv.ty
                ),
            ),
            Type::Ptr(..) => Ok(recv),
            _ => {
                if mutable && Self::is_lvalue(&recv) {
                    self.check_mutable(&recv, true).map_err(|e| Error {
                        msg: format!("method `{method}` takes `self: *mut Self`: {}", e.msg),
                        ..e
                    })?;
                }
                let (ty, span) = (Type::Ptr(Box::new(recv.ty.clone()), mutable), recv.span);
                Ok(TExpr {
                    kind: TExprKind::AddrOf(Box::new(recv)),
                    ty,
                    span,
                })
            }
        }
    }

    /// `General.init()` and `Arena.init(backing)`.
    fn builtin_static(
        &mut self,
        b: BuiltinTy,
        name: &str,
        args: &[ast::Expr],
        span: Span,
    ) -> Result<TExpr> {
        let (op, params, ty) = match (b, name) {
            (BuiltinTy::General, "init") => (Intrinsic::GeneralInit, vec![], b),
            (BuiltinTy::Arena, "init") => (
                Intrinsic::ArenaInit,
                vec![Type::Builtin(BuiltinTy::Allocator)],
                b,
            ),
            _ => {
                return err(
                    E0071,
                    span,
                    format!("type `{}` has no method `{name}`", b.name()),
                );
            }
        };
        if args.len() != params.len() {
            return err(
                E0029,
                span,
                format!(
                    "function `{}.{name}` takes {} argument(s) but {} were given",
                    b.name(),
                    params.len(),
                    args.len()
                ),
            );
        }
        let args = args
            .iter()
            .zip(&params)
            .map(|(a, p)| self.expect(a, p))
            .collect::<Result<_>>()?;
        Ok(TExpr {
            kind: TExprKind::Intrinsic { op, args },
            ty: Type::Builtin(ty),
            span,
        })
    }

    /// The methods of `General` and `Arena` (`allocator`, `deinit`, and
    /// `reset` on `Arena`) and of `Allocator` (`create`, `destroy`, `alloc`, `free`).
    fn builtin_method(
        &mut self,
        b: BuiltinTy,
        recv: TExpr,
        name: &str,
        args: &[ast::Expr],
        span: Span,
    ) -> Result<TExpr> {
        let n = match (b, name) {
            (BuiltinTy::Allocator, "alloc") => 2,
            (BuiltinTy::Allocator, "resize") => 3,
            (BuiltinTy::Allocator, _) => 1,
            _ => 0,
        };
        let known = match b {
            BuiltinTy::Allocator => {
                matches!(name, "create" | "destroy" | "alloc" | "free" | "resize")
            }
            BuiltinTy::Arena => matches!(name, "allocator" | "deinit" | "init" | "reset"),
            _ => matches!(name, "allocator" | "deinit" | "init"),
        };
        if !known {
            return err(
                E0071,
                span,
                format!("type `{}` has no method `{name}`", b.name()),
            );
        }
        if name == "init" {
            return err(
                E0073,
                span,
                format!(
                    "`{0}.init` has no `self` parameter: call it as `{0}.init(...)`",
                    b.name()
                ),
            );
        }
        if args.len() != n {
            return err(
                E0029,
                span,
                format!(
                    "method `{}.{name}` takes {n} argument(s) but {} were given",
                    b.name(),
                    args.len()
                ),
            );
        }
        if b != BuiltinTy::Allocator {
            let general = b == BuiltinTy::General;
            let (op, ty) = match name {
                "allocator" if general => (
                    Intrinsic::GeneralAllocator,
                    Type::Builtin(BuiltinTy::Allocator),
                ),
                "allocator" => (
                    Intrinsic::ArenaAllocator,
                    Type::Builtin(BuiltinTy::Allocator),
                ),
                "reset" => (Intrinsic::ArenaReset, Type::Void),
                _ if general => (Intrinsic::GeneralDeinit, Type::Void),
                _ => (Intrinsic::ArenaDeinit, Type::Void),
            };
            let r = self.receiver_ptr(recv, true, name)?;
            return Ok(TExpr {
                kind: TExprKind::Intrinsic { op, args: vec![r] },
                ty,
                span,
            });
        }
        let alloc = match &recv.ty {
            Type::Ptr(t, _) => {
                let (ty, span) = ((**t).clone(), recv.span);
                TExpr {
                    kind: TExprKind::Deref(Box::new(recv)),
                    ty,
                    span,
                }
            }
            _ => recv,
        };
        if name == "create" {
            if let ExprKind::Ident(n) = &strip_parens(&args[0]).kind
                && self.lookup(n).is_none()
                && self.is_type_name(n)
            {
                return err(
                    E0074,
                    args[0].span,
                    format!(
                        "`{n}` is a type, not a value: `create` takes the initial value, e.g. `alloc.create({n} {{ ... }})`"
                    ),
                );
            }
            let value = self.value(&args[0], None)?;
            let ty = Type::ErrorUnion(
                ALLOC_ERROR.to_string(),
                Box::new(Type::Ptr(Box::new(value.ty.clone()), true)),
            );
            return Ok(TExpr {
                kind: TExprKind::Create {
                    alloc: Box::new(alloc),
                    value: Box::new(value),
                },
                ty,
                span,
            });
        }
        if name == "alloc" {
            if let ExprKind::Ident(n) = &strip_parens(&args[0]).kind
                && self.lookup(n).is_none()
                && self.is_type_name(n)
            {
                return err(
                    E0074,
                    args[0].span,
                    format!(
                        "`{n}` is a type, not a value: `alloc` takes the value of every element, e.g. `alloc.alloc({n} {{ ... }}, len)`"
                    ),
                );
            }
            let value = self.value(&args[0], None)?;
            let len = self.value(&args[1], Some(&USIZE))?;
            if !is_unsigned(&len.ty) {
                return err(
                    E0044,
                    len.span,
                    format!("the length must be an unsigned integer, found `{}`", len.ty),
                );
            }
            let ty = Type::ErrorUnion(
                ALLOC_ERROR.to_string(),
                Box::new(Type::Slice(Box::new(value.ty.clone()), true)),
            );
            let kind = TExprKind::AllocSlice {
                alloc: Box::new(alloc),
                value: Box::new(value),
                len: Box::new(len),
                old: None,
            };
            return Ok(TExpr { kind, ty, span });
        }
        if name == "resize" {
            let old = self.value(&args[0], None)?;
            let Type::Slice(elem, _) = &old.ty else {
                return err(
                    E0023,
                    old.span,
                    format!(
                        "mismatched types: `resize` needs a slice from `alloc`, found `{}`",
                        old.ty
                    ),
                );
            };
            let elem = (**elem).clone();
            let len = self.value(&args[1], Some(&USIZE))?;
            if !is_unsigned(&len.ty) {
                return err(
                    E0044,
                    len.span,
                    format!("the length must be an unsigned integer, found `{}`", len.ty),
                );
            }
            let value = self.expect(&args[2], &elem)?;
            let ty = Type::ErrorUnion(
                ALLOC_ERROR.to_string(),
                Box::new(Type::Slice(Box::new(elem), true)),
            );
            let kind = TExprKind::AllocSlice {
                alloc: Box::new(alloc),
                value: Box::new(value),
                len: Box::new(len),
                old: Some(Box::new(old)),
            };
            return Ok(TExpr { kind, ty, span });
        }
        if name == "free" {
            let s = self.value(&args[0], None)?;
            if !matches!(s.ty, Type::Slice(..)) {
                return err(
                    E0023,
                    s.span,
                    format!(
                        "mismatched types: `free` needs a slice from `alloc`, found `{}`",
                        s.ty
                    ),
                );
            }
            return Ok(TExpr {
                kind: TExprKind::Intrinsic {
                    op: Intrinsic::Free,
                    args: vec![alloc, s],
                },
                ty: Type::Void,
                span,
            });
        }
        let p = self.value(&args[0], None)?;
        if !matches!(p.ty, Type::Ptr(..)) {
            return err(
                E0023,
                p.span,
                format!(
                    "mismatched types: `destroy` needs a pointer from `create`, found `{}`",
                    p.ty
                ),
            );
        }
        Ok(TExpr {
            kind: TExprKind::Intrinsic {
                op: Intrinsic::Destroy,
                args: vec![alloc, p],
            },
            ty: Type::Void,
            span,
        })
    }

    /// Field initializers of a struct or variant literal, in source order,
    /// then the default values of the fields it leaves out (`strukt` is the
    /// struct of a struct literal). `owner` names the struct or variant in
    /// messages, `lit` the literal.
    fn field_inits(
        &mut self,
        def: &[(String, Type)],
        fields: &[ast::FieldInit],
        owner: &str,
        lit: &str,
        strukt: Option<&str>,
        span: Span,
    ) -> Result<Vec<(usize, TExpr)>> {
        let mut inits: Vec<(usize, TExpr)> = Vec::new();
        if let Some(s) = strukt
            && self.structs[s].is_union
            && fields.len() != 1
        {
            let at = fields.get(1).map_or(span, |f| f.span);
            let first = def.first().map_or("field", |(n, _)| n.as_str());
            return err(
                E0104,
                at,
                format!(
                    "a literal of the union `{s}` sets exactly one field: `{s} {{ {first}: ... }}`"
                ),
            );
        }
        for f in fields {
            let Some(index) = def.iter().position(|(n, _)| *n == f.name) else {
                let e = Error::new(E0038, f.span, format!("{owner} has no field `{}`", f.name));
                return Err(suggest(
                    e,
                    &f.name,
                    f.span,
                    def.iter().map(|(n, _)| n.as_str()),
                ));
            };
            if inits.iter().any(|(i, _)| *i == index) {
                return err(
                    E0040,
                    f.span,
                    format!("field `{}` is initialized twice", f.name),
                );
            }
            let v = self.expect(&f.value, &def[index].1)?;
            inits.push((index, v));
        }
        if strukt.is_some_and(|s| self.structs[s].is_union) {
            return Ok(inits);
        }
        for (i, (name, _)) in def.iter().enumerate() {
            if inits.iter().any(|(j, _)| *j == i) {
                continue;
            }
            let has_default = strukt
                .and_then(|s| self.default_decls.get(s))
                .is_some_and(|d| d.exprs[i].is_some());
            if !has_default {
                return err(
                    E0039,
                    span,
                    format!("missing field `{name}` in `{lit}` literal"),
                );
            }
            let v = self.field_default(strukt.unwrap(), i, span)?;
            inits.push((i, TExpr { span, ..v }));
        }
        Ok(inits)
    }

    /// `match` on an enum, an error value or an integer. As a statement
    /// (`is_stmt`) the arms produce no value; otherwise all arms must have
    /// the same type.
    fn match_expr(
        &mut self,
        e: &ast::Expr,
        expected: Option<&Type>,
        is_stmt: bool,
    ) -> Result<TExpr> {
        let ExprKind::Match { scrutinee, arms } = &e.kind else {
            unreachable!()
        };
        let s = self.value(scrutinee, None)?;
        // The cases to cover: the variants of the enum, or the errors of the set.
        let (def, set, cases) = match &s.ty {
            Type::Enum(ename) => {
                let def = self.enums[ename].clone();
                let cases: Vec<String> = def.variants.iter().map(|v| v.name.clone()).collect();
                (Some(def), None, cases)
            }
            Type::ErrorSet(set) => (None, Some(set.clone()), self.sets[set].clone()),
            Type::Int(it) => {
                let pats = Self::int_patterns(arms, *it, self.layout, e.span)?
                    .into_iter()
                    .map(|r| (Vec::new(), r))
                    .collect();
                return self.match_arms(e, s, None, pats, expected, is_stmt);
            }
            t => {
                return err(
                    E0055,
                    s.span,
                    format!("`match` needs an enum, error or integer value, found `{t}`"),
                );
            }
        };
        let kind = if def.is_some() { "variant" } else { "error" };
        let show = |n: &str| match (&def, &set) {
            (Some(d), _) => format!("`{}.{n}`", d.name),
            (_, set) => qualify(set.as_ref().unwrap(), n),
        };
        // Patterns, in source order.
        let mut variants = Vec::new();
        let mut covered = vec![false; cases.len()];
        let mut wildcard = false;
        for arm in arms {
            if wildcard {
                return err(
                    E0057,
                    arm.span,
                    "unreachable arm: the `_` arm above already matches everything",
                );
            }
            if let Some(p) = arm.values.first() {
                return err(
                    E0088,
                    p.span,
                    format!(
                        "a `match` on `{}` takes `.Name` patterns, not integers",
                        s.ty
                    ),
                );
            }
            if arm.variants.is_empty() {
                wildcard = true;
            }
            let mut mine = Vec::new();
            for v in &arm.variants {
                let name = &v.name;
                let Some(i) = cases.iter().position(|c| c == name) else {
                    return match (&def, &set) {
                        (Some(d), _) => err(
                            E0041,
                            v.span,
                            format!("enum `{}` has no variant `{name}`", d.name),
                        ),
                        (_, set) => err(
                            E0060,
                            v.span,
                            format!(
                                "{} has no error `{name}`",
                                describe_set(set.as_ref().unwrap())
                            ),
                        ),
                    };
                };
                if mine.contains(&i) {
                    return err(
                        E0057,
                        v.span,
                        format!(
                            "{kind} {} is already matched by an earlier pattern of this arm",
                            show(name)
                        ),
                    );
                }
                if covered[i] {
                    return err(
                        E0057,
                        v.span,
                        format!("{kind} {} is already matched by an earlier arm", show(name)),
                    );
                }
                covered[i] = true;
                mine.push(i);
            }
            variants.push(mine);
        }
        if !wildcard {
            let missing: Vec<String> = cases
                .iter()
                .zip(&covered)
                .filter(|(_, c)| !**c)
                .map(|(n, _)| show(n))
                .collect();
            if !missing.is_empty() {
                let s = if missing.len() == 1 { "" } else { "s" };
                return err(
                    E0056,
                    e.span,
                    format!(
                        "`match` is not exhaustive: missing {kind}{s} {}",
                        missing.join(", ")
                    ),
                );
            }
        }
        // A `match` on errors tests the error value.
        if set.is_some() {
            for v in variants.iter_mut().flatten() {
                *v = self
                    .error_names
                    .iter()
                    .position(|n| *n == cases[*v])
                    .unwrap()
                    + 1;
            }
        }
        let pats = variants.into_iter().map(|v| (v, Vec::new())).collect();
        self.match_arms(e, s, def, pats, expected, is_stmt)
    }

    /// The ranges of values that each arm of a `match` on the integer type
    /// `it` matches (empty for `_`). No value may be matched twice, and
    /// without `_` the arms must cover every value of `it`.
    fn int_patterns(
        arms: &[ast::MatchArm],
        it: IntTy,
        l: Layout,
        span: Span,
    ) -> Result<Vec<Vec<(i128, i128)>>> {
        let ty = Type::Int(it);
        let mut seen: Vec<(i128, i128)> = Vec::new();
        let mut out = Vec::new();
        let mut wildcard = false;
        for arm in arms {
            if wildcard {
                return err(
                    E0057,
                    arm.span,
                    "unreachable arm: the `_` arm above already matches everything",
                );
            }
            if let Some(v) = arm.variants.first() {
                return err(
                    E0088,
                    arm.span,
                    format!(
                        "a `match` on `{ty}` takes integer and character patterns, not `.{}`",
                        v.name
                    ),
                );
            }
            if arm.values.is_empty() {
                wildcard = true;
                out.push(Vec::new());
                continue;
            }
            let mut ranges = Vec::new();
            for p in &arm.values {
                let shown = if p.lo == p.hi {
                    format!("`{}`", p.lo)
                } else {
                    format!("`{}..={}`", p.lo, p.hi)
                };
                if p.lo > p.hi {
                    return err(
                        E0088,
                        p.span,
                        format!("the range {shown} is empty: its start is larger than its end"),
                    );
                }
                if p.lo < it.min(l) || p.hi > it.max(l) {
                    let v = if p.lo < it.min(l) { p.lo } else { p.hi };
                    return err(
                        E0024,
                        p.span,
                        format!("pattern value `{v}` does not fit in `{ty}`"),
                    );
                }
                if seen.iter().any(|&(a, b)| p.lo <= b && a <= p.hi) {
                    return err(
                        E0057,
                        p.span,
                        format!(
                            "pattern {shown} matches values that an earlier pattern already matches"
                        ),
                    );
                }
                seen.push((p.lo, p.hi));
                ranges.push((p.lo, p.hi));
            }
            out.push(ranges);
        }
        if !wildcard {
            seen.sort();
            let mut gaps = Vec::new();
            let mut next = it.min(l);
            for (a, b) in seen {
                if a > next {
                    gaps.push((next, a - 1));
                }
                next = next.max(b + 1);
            }
            if next <= it.max(l) {
                gaps.push((next, it.max(l)));
            }
            if !gaps.is_empty() {
                let shown: Vec<String> = gaps
                    .iter()
                    .take(3)
                    .map(|&(a, b)| {
                        if a == b {
                            format!("`{a}`")
                        } else {
                            format!("`{a}..={b}`")
                        }
                    })
                    .collect();
                let more = if gaps.len() > 3 { ", ..." } else { "" };
                return err(
                    E0056,
                    span,
                    format!(
                        "`match` on `{ty}` is not exhaustive: missing {}{more} (add a `_` arm)",
                        shown.join(", ")
                    ),
                );
            }
        }
        Ok(out)
    }

    /// Check the arms of the `match` `e` on `s`, whose patterns are checked:
    /// `pats` has each arm's variant (`def` is the enum) or error value, and
    /// its integer ranges.
    fn match_arms(
        &mut self,
        e: &ast::Expr,
        s: TExpr,
        def: Option<EnumDef>,
        pats: Vec<ArmPat>,
        expected: Option<&Type>,
        is_stmt: bool,
    ) -> Result<TExpr> {
        let ExprKind::Match { arms, .. } = &e.kind else {
            unreachable!()
        };
        // Without an expected type, the first arm that is not an untyped
        // literal or `null` decides the type of the `match`.
        let mut want = if is_stmt { None } else { expected.cloned() };
        let mut order: Vec<usize> = (0..arms.len()).collect();
        let unreachable = |a: &ast::MatchArm| matches!(&a.body, ArmBody::Expr(x) if matches!(strip_parens(x).kind, ExprKind::Unreachable));
        if !is_stmt
            && want.is_none()
            && let Some(i) = arms.iter().position(|a| {
                matches!(&a.body, ArmBody::Expr(x) if !is_untyped(x) && !is_null(x))
                    && !unreachable(a)
            })
        {
            order.remove(i);
            order.insert(0, i);
        }
        // `unreachable` takes the type of the other arms.
        order.sort_by_key(|&i| unreachable(&arms[i]));
        let mut checked: Vec<Option<TArm>> = vec![None; arms.len()];
        for i in order {
            self.scopes.push(HashMap::new());
            let arm = self.match_arm(def.as_ref(), &arms[i], &pats[i].0, want.as_ref(), is_stmt);
            self.scopes.pop();
            let mut arm = arm?;
            arm.ranges = pats[i].1.clone();
            if let (false, None, TArmBody::Expr(x)) = (is_stmt, &want, &arm.body) {
                want = Some(x.ty.clone());
            }
            checked[i] = Some(arm);
        }
        // Arms that all leave (`return`, `break`, `continue`) give no type.
        let ty = if is_stmt {
            Type::Void
        } else {
            want.unwrap_or(
                if arms.iter().any(|a| matches!(a.body, ArmBody::Block(_))) {
                    Type::Void
                } else {
                    Type::Error
                },
            )
        };
        let arms = checked.into_iter().map(Option::unwrap).collect();
        Ok(TExpr {
            kind: TExprKind::Match {
                scrutinee: Box::new(s),
                arms,
            },
            ty,
            span: e.span,
        })
    }

    /// One arm: declare the pattern variables, then check the body.
    /// `def` is `None` in a `match` on errors, where `variants` are error
    /// values, and on integers. An arm with several patterns binds nothing.
    fn match_arm(
        &mut self,
        def: Option<&EnumDef>,
        arm: &ast::MatchArm,
        variants: &[usize],
        want: Option<&Type>,
        is_stmt: bool,
    ) -> Result<TArm> {
        let mut bindings: Vec<(VarId, usize)> = Vec::new();
        for (p, &vi) in arm.variants.iter().zip(variants) {
            let Some(def) = def else {
                if p.fields.is_some() {
                    return err(
                        E0042,
                        p.span,
                        format!("an error has no fields; write `.{}`", p.name),
                    );
                }
                continue;
            };
            let v = &def.variants[vi];
            let what = format!("{}.{}", def.name, v.name);
            match (&p.fields, v.fields.is_empty()) {
                (None, true) => {}
                (None, false) => {
                    return err(
                        E0042,
                        p.span,
                        format!(
                            "variant `{what}` has fields; match them with `.{} {{ .. }}`",
                            v.name
                        ),
                    );
                }
                (Some(_), true) => {
                    return err(
                        E0042,
                        p.span,
                        format!("variant `{what}` has no fields; write `.{}`", v.name),
                    );
                }
                (Some(pats), false) => {
                    if let (Some(f), true) = (pats.first(), variants.len() > 1) {
                        return err(
                            E0097,
                            f.span,
                            format!(
                                "an arm with several patterns cannot bind fields: match `{what}` in an arm of its own to use `{}`",
                                f.binding
                            ),
                        );
                    }
                    for f in pats {
                        let Some(fi) = v.fields.iter().position(|(n, _)| *n == f.field) else {
                            return err(
                                E0038,
                                f.span,
                                format!("variant `{what}` has no field `{}`", f.field),
                            );
                        };
                        if bindings.iter().any(|&(_, x)| x == fi) {
                            return err(
                                E0040,
                                f.span,
                                format!("field `{}` is matched twice", f.field),
                            );
                        }
                        let id = self.declare(
                            &f.binding,
                            v.fields[fi].1.clone(),
                            false,
                            f.binding_span,
                        )?;
                        bindings.push((id, fi));
                    }
                    if !p.rest
                        && let Some((name, _)) = v
                            .fields
                            .iter()
                            .enumerate()
                            .find(|(i, _)| !bindings.iter().any(|&(_, f)| f == *i))
                            .map(|(_, f)| f)
                    {
                        return err(
                            E0058,
                            p.span,
                            format!(
                                "pattern for `{what}` does not mention field `{name}` (add `..` to ignore the rest)"
                            ),
                        );
                    }
                }
            }
        }
        let body = match &arm.body {
            ArmBody::Block(b) if is_stmt || is_jump_block(b) => TArmBody::Block(self.block(b)),
            ArmBody::Block(b) => {
                return err(
                    E0059,
                    b.span,
                    "a `match` that produces a value needs an expression after `=>`, not a block",
                );
            }
            ArmBody::Expr(x) if is_stmt => TArmBody::Expr(self.expr_stmt(x)?),
            ArmBody::Expr(x) => TArmBody::Expr(match want {
                // The `match` is returned (`return match ...`): the arm is
                // what `return` takes.
                Some(t @ Type::ErrorUnion(set, payload)) => {
                    let hint = match strip_parens(x).kind {
                        ExprKind::Unreachable => Some(t),
                        _ => (**payload != Type::Void).then(|| Self::hint(x, payload)),
                    };
                    let v = self.expr(x, hint)?;
                    self.return_union(v, set, payload)?
                }
                Some(t) => self.expect(x, t)?,
                None => self.value(x, None)?,
            }),
        };
        Ok(TArm {
            variants: variants.to_vec(),
            ranges: Vec::new(),
            bindings,
            body,
        })
    }
}
