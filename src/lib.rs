//! Stage0 compiler for Aye: lexer -> parser (AST) -> modules (name
//! resolution) -> sema (typed AST) -> OIR -> C.

pub mod ast;
pub mod cgen;
pub mod cimport;
pub mod codes;
pub mod diag;
pub mod doc;
pub mod driver;
pub mod fetch;
pub mod fmt;
pub mod lexer;
pub mod lsp;
pub mod manifest;
pub mod module;
pub mod oir;
pub mod parser;
pub mod sema;
pub mod sha256;
