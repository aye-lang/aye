//! Compiler driver: runs the pipeline and invokes clang.

use crate::diag::{Diag, Error};
use crate::{cgen, lexer, oir, parser, sema};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// All runtime checks, no optimization.
    Debug,
    /// Optimized, all runtime checks.
    Safe,
    /// Optimized, no runtime checks.
    Fast,
}

impl Mode {
    pub fn parse(s: &str) -> Option<Mode> {
        match s {
            "debug" => Some(Mode::Debug),
            "safe" => Some(Mode::Safe),
            "fast" => Some(Mode::Fast),
            _ => None,
        }
    }

    fn clang_flags(self) -> &'static [&'static str] {
        match self {
            Mode::Debug => &["-O0", "-g"],
            Mode::Safe => &["-O2", "-g"],
            Mode::Fast => &["-O2"],
        }
    }
}

/// At most this many diagnostics are reported.
const MAX_DIAGS: usize = 50;

/// Run the front end (lexer, parser, sema). Every compile error comes from
/// here; OIR lowering and C generation cannot fail. Used by `ore check`.
///
/// Each stage recovers from errors and reports all of them, sorted by
/// position. A stage runs only if the previous one had no errors, so a
/// syntax error never causes follow-up type errors.
pub fn check(file_name: &str, src: &str) -> Result<sema::Program, Vec<Diag>> {
    let diags = |errs: Vec<Error>| {
        let mut d: Vec<Diag> = errs.into_iter().map(|e| e.with_file(file_name)).collect();
        d.sort_by_key(|d| (d.span.line, d.span.col));
        d.dedup();
        d.truncate(MAX_DIAGS);
        d
    };
    let toks = lexer::lex(src).map_err(diags)?;
    let ast = parser::parse(toks).map_err(diags)?;
    sema::check(&ast).map_err(diags)
}

/// Run the front end, OIR lowering and C generation.
/// `file_name` is used in diagnostics, `#line` directives and panic messages.
pub fn compile_to_c(file_name: &str, src: &str, mode: Mode) -> Result<String, Vec<Diag>> {
    let typed = check(file_name, src)?;
    let ir = oir::lower(&typed);
    Ok(cgen::emit(&ir, file_name, mode != Mode::Fast, mode == Mode::Debug))
}

/// Compile generated C to an executable with clang.
pub fn cc(c_src: &str, out: &Path, mode: Mode) -> Result<(), String> {
    let mut cmd = Command::new("clang");
    cmd.args(["-std=gnu11", "-fwrapv", "-fno-strict-aliasing", "-Wno-incompatible-library-redeclaration"])
        .args(mode.clang_flags())
        .args(["-x", "c", "-", "-o"])
        .arg(out)
        .stdin(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| format!("failed to run clang: {e}"))?;
    child.stdin.take().unwrap().write_all(c_src.as_bytes()).map_err(|e| e.to_string())?;
    let status = child.wait().map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("clang failed on the generated C (this is a compiler bug)".to_string());
    }
    Ok(())
}
