//! Stage0 compiler for Ore: lexer -> parser (AST) -> sema (typed AST) -> OIR -> C.

pub mod ast;
pub mod cgen;
pub mod codes;
pub mod diag;
pub mod driver;
pub mod lexer;
pub mod oir;
pub mod parser;
pub mod sema;
