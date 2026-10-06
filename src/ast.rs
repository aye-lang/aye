//! Untyped syntax tree produced by the parser.

use crate::diag::Span;

#[derive(Debug, Clone)]
pub struct File {
    pub imports: Vec<Import>,
    pub items: Vec<Item>,
}

/// `import a.b.c`: the path's names with their positions. It binds its
/// last name in the file.
#[derive(Debug, Clone)]
pub struct Import {
    pub path: Vec<(String, Span)>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Item {
    Fn(FnDecl),
    Struct(StructDecl),
    Enum(EnumDecl),
    ErrorSet(ErrorSetDecl),
    Interface(InterfaceDecl),
    Test(TestDecl),
    Global(GlobalDecl),
}

/// A top-level `let NAME = value` constant, `var name = value` global, or
/// `let Name = Type` type alias (sema tells them apart by the value).
#[derive(Debug, Clone)]
pub struct GlobalDecl {
    pub name: String,
    pub span: Span,
    pub is_pub: bool,
    /// `var` (true) or `let`.
    pub mutable: bool,
    /// The `let` or `var` keyword.
    pub kw_span: Span,
    pub ty: Option<TypeExpr>,
    pub init: Expr,
}

#[derive(Debug, Clone)]
pub struct FnDecl {
    pub name: String,
    pub span: Span,
    pub is_pub: bool,
    pub is_extern: bool,
    pub params: Vec<Param>,
    pub ret: Option<TypeExpr>,
    /// `where T: I, ...`: the constraints on the `comptime` parameters.
    pub bounds: Vec<Bound>,
    /// `None` for `extern fn`.
    pub body: Option<Block>,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub span: Span,
    pub ty: TypeExpr,
    /// `comptime T: type`
    pub comptime: bool,
}

/// `T: I` in a `where` clause: the type parameter `T` must implement the
/// interface `I`.
#[derive(Debug, Clone)]
pub struct Bound {
    pub param: String,
    pub span: Span,
    pub iface: String,
    pub iface_span: Span,
}

#[derive(Debug, Clone)]
pub struct StructDecl {
    pub name: String,
    pub span: Span,
    pub is_pub: bool,
    pub fields: Vec<FieldDecl>,
    /// Functions declared in the struct body.
    pub methods: Vec<FnDecl>,
}

/// `interface Name { fn m(self: *Self, ...) T ... }`. The methods have no body.
#[derive(Debug, Clone)]
pub struct InterfaceDecl {
    pub name: String,
    pub span: Span,
    pub is_pub: bool,
    pub methods: Vec<FnDecl>,
}

/// `test "name" { ... }`. The body is a function `test#N() !void`, where N
/// counts the tests of the file from 0.
#[derive(Debug, Clone)]
pub struct TestDecl {
    pub name: String,
    pub func: FnDecl,
}

#[derive(Debug, Clone)]
pub struct EnumDecl {
    pub name: String,
    pub span: Span,
    pub is_pub: bool,
    pub variants: Vec<VariantDecl>,
    /// Functions declared in the enum body.
    pub methods: Vec<FnDecl>,
}

/// `error Name { A, B }` or `error Name = X || Y`.
#[derive(Debug, Clone)]
pub struct ErrorSetDecl {
    pub name: String,
    pub span: Span,
    pub is_pub: bool,
    pub kind: ErrorSetKind,
}

#[derive(Debug, Clone)]
pub enum ErrorSetKind {
    /// The error names, with their positions.
    List(Vec<(String, Span)>),
    /// The merged sets, with their positions.
    Merge(Vec<(String, Span)>),
}

#[derive(Debug, Clone)]
pub struct VariantDecl {
    pub name: String,
    pub span: Span,
    /// Payload fields; empty for a variant without payload.
    pub fields: Vec<FieldDecl>,
}

#[derive(Debug, Clone)]
pub struct FieldDecl {
    pub name: String,
    pub span: Span,
    pub ty: TypeExpr,
}

#[derive(Debug, Clone)]
pub struct TypeExpr {
    pub kind: TypeExprKind,
    pub span: Span,
}

/// A name in a type, a `where` constraint or an error set merge can be a
/// path, `std.fs.File`: its parts are joined with `.`.
#[derive(Debug, Clone)]
pub enum TypeExprKind {
    Named(String),
    /// `[N]T`
    Array(u64, Box<TypeExpr>),
    /// `[]T` (mutable = false) or `[]mut T` (mutable = true).
    Slice(Box<TypeExpr>, bool),
    /// `[*]T`
    CPtr(Box<TypeExpr>),
    /// `*T` (mutable = false) or `*mut T` (mutable = true).
    Ptr(Box<TypeExpr>, bool),
    /// `?T`
    Optional(Box<TypeExpr>),
    /// `E!T` (with the set name and its position), or `!T` (inferred set).
    ErrorUnion(Option<(String, Span)>, Box<TypeExpr>),
    /// `dyn I`
    Dyn(String),
    /// `F(A, B)`: the type that the type function `F` returns.
    Generic(String, Vec<TypeExpr>),
}

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum StmtKind {
    /// `let` (mutable = false) or `var` (mutable = true).
    Let { name: String, mutable: bool, ty: Option<TypeExpr>, init: Expr },
    Assign { target: Expr, op: AssignOp, value: Expr },
    /// `capture` is the `v` of `if x |v| { }`.
    If { cond: Expr, capture: Option<(String, Span)>, then: Block, els: Option<Box<Stmt>> },
    /// `capture` is the `v` of `while x |v| { }`.
    While { cond: Expr, capture: Option<(String, Span)>, body: Block },
    /// `for var in lo..hi { body }`
    For { var: String, var_span: Span, lo: Expr, hi: Expr, body: Block },
    /// `for elem in seq { body }` or `for index, elem in seq { body }`.
    ForEach { index: Option<(String, Span)>, elem: (String, Span), seq: Expr, body: Block },
    Break,
    Continue,
    Return(Option<Expr>),
    Defer(Box<Stmt>),
    ErrDefer(Box<Stmt>),
    Block(Block),
    Expr(Expr),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Set,
    /// Compound assignment `x op= v`.
    Op(BinOp),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    AddWrap,
    SubWrap,
    MulWrap,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

impl BinOp {
    pub fn as_str(self) -> &'static str {
        match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Rem => "%",
            BinOp::AddWrap => "+%",
            BinOp::SubWrap => "-%",
            BinOp::MulWrap => "*%",
            BinOp::Eq => "==",
            BinOp::Ne => "!=",
            BinOp::Lt => "<",
            BinOp::Le => "<=",
            BinOp::Gt => ">",
            BinOp::Ge => ">=",
            BinOp::And => "&&",
            BinOp::Or => "||",
            BinOp::BitAnd => "&",
            BinOp::BitOr => "|",
            BinOp::BitXor => "^",
            BinOp::Shl => "<<",
            BinOp::Shr => ">>",
        }
    }

    pub fn is_comparison(self) -> bool {
        matches!(self, BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge)
    }

    /// Arithmetic, bitwise and shift operators: both operands and the
    /// result have one numeric type.
    pub fn is_arith(self) -> bool {
        matches!(
            self,
            BinOp::Add
                | BinOp::Sub
                | BinOp::Mul
                | BinOp::Div
                | BinOp::Rem
                | BinOp::AddWrap
                | BinOp::SubWrap
                | BinOp::MulWrap
        ) || self.is_bitwise()
            || matches!(self, BinOp::Shl | BinOp::Shr)
    }

    /// `&`, `|` and `^`, which cannot be mixed without parentheses.
    pub fn is_bitwise(self) -> bool {
        matches!(self, BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
    /// `~x`
    BitNot,
}

#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    Int(u64),
    Float(f64),
    Bool(bool),
    Null,
    Str(Vec<u8>),
    Char(u32),
    Ident(String),
    Call { name: String, args: Vec<Expr> },
    /// `base.name(args)`: a method call, or `Type.name(args)`.
    MethodCall { base: Box<Expr>, name: String, args: Vec<Expr> },
    Builtin { name: String, args: Vec<Expr> },
    Binary { op: BinOp, lhs: Box<Expr>, rhs: Box<Expr> },
    Unary { op: UnOp, expr: Box<Expr> },
    /// `&x` (mutable = false) or `&mut x` (mutable = true).
    AddrOf { mutable: bool, expr: Box<Expr> },
    /// `p.*`
    Deref(Box<Expr>),
    Cast { expr: Box<Expr>, ty: TypeExpr },
    Field { base: Box<Expr>, name: String },
    Index { base: Box<Expr>, index: Box<Expr> },
    Slice { base: Box<Expr>, lo: Option<Box<Expr>>, hi: Option<Box<Expr>> },
    /// `Name { ... }` or `F(T) { ... }`; `ty` is `Named` or `Generic`.
    StructLit { ty: TypeExpr, fields: Vec<FieldInit> },
    /// `Enum.Variant { field: value, ... }`. A variant without payload is
    /// written `Enum.Variant` and parsed as `Field`. `enum_name` can be a
    /// path (`shapes.Shape`); `mod.Struct { ... }` is parsed as a
    /// `VariantLit` too, and module resolution makes it a `StructLit`.
    VariantLit { enum_name: String, variant: String, fields: Vec<FieldInit> },
    Match { scrutinee: Box<Expr>, arms: Vec<MatchArm> },
    /// `try x`
    Try(Box<Expr>),
    /// `x catch |e| body`; the body is a block or an expression.
    Catch { expr: Box<Expr>, capture: Option<(String, Span)>, body: Box<ArmBody> },
    /// `[N]T{a, b, c}`; `ty` is the array type. Also the empty slice
    /// `[]T{}`, where `ty` is a slice type.
    ArrayLit { ty: TypeExpr, elems: Vec<Expr> },
    Paren(Box<Expr>),
    /// `struct { ... }`: the value of a type function. The name is empty.
    StructType(StructDecl),
    /// A type that cannot be read as an expression (`*T`, `?T`, `[]T`,
    /// `[N]T`, `dyn I`), as a call argument for a `comptime` parameter.
    Type(TypeExpr),
}

#[derive(Debug, Clone)]
pub struct FieldInit {
    pub name: String,
    pub span: Span,
    pub value: Expr,
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    /// `None` for the `_` pattern and for integer patterns.
    pub variant: Option<String>,
    /// Integer and character patterns (`'a'`, `-1`, `'a'..='z'`), several
    /// per arm; empty for `_` and variant patterns.
    pub values: Vec<IntPat>,
    /// `.V { a, b: c, .. }`: the field patterns; `None` for `.V` and `_`.
    pub fields: Option<Vec<FieldPat>>,
    /// The pattern ends with `..`.
    pub rest: bool,
    pub body: ArmBody,
    pub span: Span,
}

/// An integer or character pattern: the inclusive range `lo..=hi`, or a
/// single value (`lo == hi`).
#[derive(Debug, Clone)]
pub struct IntPat {
    pub lo: i128,
    pub hi: i128,
    pub span: Span,
}

/// `field` or `field: binding` in a variant pattern.
#[derive(Debug, Clone)]
pub struct FieldPat {
    pub field: String,
    pub span: Span,
    pub binding: String,
    pub binding_span: Span,
}

#[derive(Debug, Clone)]
pub enum ArmBody {
    Expr(Expr),
    Block(Block),
}
