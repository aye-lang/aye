//! Compiler driver: runs the pipeline and invokes clang.

use crate::diag::Diag;
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

/// Run lexer, parser, sema, OIR lowering and C generation.
/// `file_name` is used in diagnostics, `#line` directives and panic messages.
pub fn compile_to_c(file_name: &str, src: &str, mode: Mode) -> Result<String, Diag> {
    let toks = lexer::lex(src).map_err(|e| e.with_file(file_name))?;
    let ast = parser::parse(toks).map_err(|e| e.with_file(file_name))?;
    let typed = sema::check(&ast).map_err(|e| e.with_file(file_name))?;
    let ir = oir::lower(&typed);
    Ok(cgen::emit(&ir, file_name, mode != Mode::Fast))
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
