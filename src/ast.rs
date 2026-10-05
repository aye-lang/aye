//! Untyped syntax tree produced by the parser.

use crate::diag::Span;

#[derive(Debug, Clone)]
pub struct File {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone)]
pub enum Item {
    Fn(FnDecl),
    Struct(StructDecl),
}

#[derive(Debug, Clone)]
pub struct FnDecl {
    pub name: String,
    pub span: Span,
    pub is_pub: bool,
    pub is_extern: bool,
    pub params: Vec<Param>,
    pub ret: Option<TypeExpr>,
    /// `None` for `extern fn`.
    pub body: Option<Block>,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub span: Span,
    pub ty: TypeExpr,
}

#[derive(Debug, Clone)]
pub struct StructDecl {
    pub name: String,
    pub span: Span,
    pub is_pub: bool,
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

#[derive(Debug, Clone)]
pub enum TypeExprKind {
    Named(String),
    /// `[N]T`
    Array(u64, Box<TypeExpr>),
    /// `[]T`
    Slice(Box<TypeExpr>),
    /// `[*]T`
    CPtr(Box<TypeExpr>),
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
    If { cond: Expr, then: Block, els: Option<Box<Stmt>> },
    While { cond: Expr, body: Block },
    /// `for var in lo..hi { body }`
    For { var: String, var_span: Span, lo: Expr, hi: Expr, body: Block },
    Break,
    Continue,
    Return(Option<Expr>),
    Defer(Box<Stmt>),
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
        }
    }

    pub fn is_comparison(self) -> bool {
        matches!(self, BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge)
    }

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
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
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
    Str(Vec<u8>),
    Char(u32),
    Ident(String),
    Call { name: String, args: Vec<Expr> },
    Builtin { name: String, args: Vec<Expr> },
    Binary { op: BinOp, lhs: Box<Expr>, rhs: Box<Expr> },
    Unary { op: UnOp, expr: Box<Expr> },
    Cast { expr: Box<Expr>, ty: TypeExpr },
    Field { base: Box<Expr>, name: String },
    Index { base: Box<Expr>, index: Box<Expr> },
    Slice { base: Box<Expr>, lo: Option<Box<Expr>>, hi: Option<Box<Expr>> },
    StructLit { name: String, fields: Vec<FieldInit> },
    /// `[N]T{a, b, c}`; `ty` is the array type.
    ArrayLit { ty: TypeExpr, elems: Vec<Expr> },
    Paren(Box<Expr>),
}

#[derive(Debug, Clone)]
pub struct FieldInit {
    pub name: String,
    pub span: Span,
    pub value: Expr,
}
