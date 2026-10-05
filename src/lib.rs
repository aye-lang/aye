//! Stage0 compiler for Ore: lexer -> parser (AST) -> sema (typed AST) -> OIR -> C.

pub mod diag;
pub mod lexer;
pub mod ast;
pub mod parser;
