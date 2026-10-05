//! Compiler driver: runs the pipeline and invokes clang.

use crate::diag::{Diag, Error, json_str};
use crate::{ast, cgen, lexer, oir, parser, sema};
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
/// here; OIR lowering and C generation cannot fail. Used by `ore check`,
/// which requires `pub fn main()` unless the file has `test` blocks.
pub fn check(file_name: &str, src: &str) -> Result<sema::Program, Vec<Diag>> {
    front_end(file_name, src, None)
}

/// The front end. `need_main` says whether the file must have
/// `pub fn main()`; `None` means "unless it has `test` blocks".
///
/// Each stage recovers from errors and reports all of them, sorted by
/// position. A stage runs only if the previous one had no errors, so a
/// syntax error never causes follow-up type errors.
fn front_end(file_name: &str, src: &str, need_main: Option<bool>) -> Result<sema::Program, Vec<Diag>> {
    let diags = |errs: Vec<Error>| {
        let mut d: Vec<Diag> = errs.into_iter().map(|e| e.with_file(file_name)).collect();
        d.sort_by_key(|d| (d.span.line, d.span.col));
        d.dedup();
        d.truncate(MAX_DIAGS);
        d
    };
    let toks = lexer::lex(src).map_err(diags)?;
    let ast = parser::parse(toks).map_err(diags)?;
    let need_main = need_main.unwrap_or_else(|| !ast.items.iter().any(|i| matches!(i, ast::Item::Test(_))));
    sema::check(&ast, need_main).map_err(diags)
}

/// Run the front end, OIR lowering and C generation for an executable;
/// `test` blocks are left out.
/// `file_name` is used in diagnostics, `#line` directives and panic messages.
pub fn compile_to_c(file_name: &str, src: &str, mode: Mode) -> Result<String, Vec<Diag>> {
    let mut typed = front_end(file_name, src, Some(true))?;
    typed.fns.retain(|f| !sema::is_test(&f.name));
    typed.tests.clear();
    let ir = oir::lower(&typed);
    Ok(cgen::emit(&ir, file_name, mode != Mode::Fast, mode == Mode::Debug, false))
}

/// Like `compile_to_c`, for a test executable: its argument is the index
/// of the test to run. Also returns the test names.
pub fn compile_tests_to_c(file_name: &str, src: &str, mode: Mode) -> Result<(String, Vec<String>), Vec<Diag>> {
    let typed = front_end(file_name, src, Some(false))?;
    let ir = oir::lower(&typed);
    Ok((cgen::emit(&ir, file_name, mode != Mode::Fast, mode == Mode::Debug, true), typed.tests))
}

/// The outcome of one test.
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub stdout: String,
    pub stderr: String,
    /// Why the test failed, read from its stderr.
    pub failures: Vec<Failure>,
}

pub enum Failure {
    /// `kind` is `panic`, `assertion` or `leak`.
    At { kind: &'static str, line: u32, col: u32, message: String },
    /// The test returned an error; the trace is empty outside debug mode.
    Error { name: String, trace: Vec<(u32, u32)> },
    /// Ended in another way, such as a signal.
    Crash(String),
}

pub enum TestError {
    Compile(Vec<Diag>),
    /// clang or the test executable could not run.
    Run(String),
}

/// `ore test`: compile the tests of a file and run each one whose name
/// contains `filter` in its own process, so that a panic ends only that
/// test.
pub fn run_tests(file_name: &str, src: &str, mode: Mode, filter: &str) -> Result<Vec<TestResult>, TestError> {
    let (c, names) = compile_tests_to_c(file_name, src, mode).map_err(TestError::Compile)?;
    let exe = std::env::temp_dir().join(format!("ore-test-{}", std::process::id()));
    cc(&c, &exe, mode).map_err(TestError::Run)?;
    let mut results = Vec::new();
    for (i, name) in names.into_iter().enumerate() {
        if !name.contains(filter) {
            continue;
        }
        let out = match Command::new(&exe).arg(i.to_string()).output() {
            Ok(o) => o,
            Err(e) => {
                let _ = std::fs::remove_file(&exe);
                return Err(TestError::Run(format!("cannot run {}: {e}", exe.display())));
            }
        };
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        let passed = out.status.success();
        let mut failures = parse_failures(file_name, &stderr);
        if !passed && failures.is_empty() {
            use std::os::unix::process::ExitStatusExt;
            failures.push(Failure::Crash(match (out.status.code(), out.status.signal()) {
                (Some(c), _) => format!("exit status {c}"),
                (_, s) => format!("terminated by signal {}", s.unwrap_or(0)),
            }));
        }
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        results.push(TestResult { name, passed, stdout, stderr, failures });
    }
    let _ = std::fs::remove_file(&exe);
    Ok(results)
}

/// The failures that a test process reported on stderr: lines
/// `file:L:C: panic: ...`, `file:L:C: assertion failed: ...`,
/// `file:L:C: leak: ...`, and `error: Name` followed by the error return
/// trace.
fn parse_failures(file_name: &str, stderr: &str) -> Vec<Failure> {
    // `L:C` and the rest of a line that starts with `file:`.
    let pos = |l: &str| -> Option<(u32, u32, String)> {
        let rest = l.strip_prefix(file_name)?.strip_prefix(':')?;
        let mut parts = rest.splitn(3, ':');
        let line = parts.next()?.parse().ok()?;
        let col = parts.next()?.parse().ok()?;
        Some((line, col, parts.next().unwrap_or("").to_string()))
    };
    let mut out = Vec::new();
    let mut lines = stderr.lines().peekable();
    while let Some(l) = lines.next() {
        if let Some(name) = l.strip_prefix("error: ") {
            let mut trace = Vec::new();
            if lines.peek() == Some(&"error return trace:") {
                lines.next();
                while let Some((line, col, rest)) = lines.peek().and_then(|l| pos(l)) {
                    if !rest.is_empty() {
                        break;
                    }
                    trace.push((line, col));
                    lines.next();
                }
            }
            out.push(Failure::Error { name: name.to_string(), trace });
            continue;
        }
        let Some((line, col, rest)) = pos(l) else { continue };
        for (prefix, kind) in [(" panic: ", "panic"), (" assertion failed: ", "assertion"), (" leak: ", "leak")] {
            if let Some(message) = rest.strip_prefix(prefix) {
                out.push(Failure::At { kind, line, col, message: message.to_string() });
            }
        }
    }
    out
}

impl TestResult {
    /// One entry of the `tests` array in `ore test --json`.
    pub fn to_json(&self) -> String {
        let failures: Vec<String> = self
            .failures
            .iter()
            .map(|f| match f {
                Failure::At { kind, line, col, message } => {
                    format!(r#"{{"kind":"{kind}","line":{line},"col":{col},"message":{}}}"#, json_str(message))
                }
                Failure::Error { name, trace } => {
                    let t: Vec<String> = trace.iter().map(|(l, c)| format!(r#"{{"line":{l},"col":{c}}}"#)).collect();
                    format!(r#"{{"kind":"error","error":{},"trace":[{}]}}"#, json_str(name), t.join(","))
                }
                Failure::Crash(m) => format!(r#"{{"kind":"crash","message":{}}}"#, json_str(m)),
            })
            .collect();
        format!(
            r#"{{"name":{},"status":"{}","failures":[{}],"stdout":{},"stderr":{}}}"#,
            json_str(&self.name),
            if self.passed { "pass" } else { "fail" },
            failures.join(","),
            json_str(&self.stdout),
            json_str(&self.stderr)
        )
    }
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
