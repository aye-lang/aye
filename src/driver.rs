//! Compiler driver: runs the pipeline and invokes clang.

use crate::diag::{Diag, Error, Fix, Span, json_str, offset};
use crate::module::{self, SourceFile};
use crate::{ast, cgen, oir, sema};
use std::io::Write;
use std::path::{Path, PathBuf};
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

/// The root module of a program: its files, and the directory that
/// `import` paths start from.
pub struct Root {
    pub files: Vec<SourceFile>,
    pub base: PathBuf,
}

impl Root {
    /// The root module at `path`: a directory (every `.ore` file in it), or
    /// a single file, which is a module of its own (the other files in its
    /// directory are not part of it). Imports are found in the directory.
    pub fn load(path: &str) -> Result<Root, String> {
        let p = Path::new(path);
        if !p.is_dir() {
            let text = std::fs::read_to_string(p).map_err(|e| format!("cannot read {path}: {e}"))?;
            return Ok(Root::single(path, &text));
        }
        let mut names: Vec<PathBuf> = std::fs::read_dir(p)
            .map_err(|e| format!("cannot read {path}: {e}"))?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|f| f.is_file() && f.extension().is_some_and(|e| e == "ore"))
            .collect();
        names.sort();
        if names.is_empty() {
            return Err(format!("{path} has no .ore files"));
        }
        let mut files = Vec::new();
        for f in names {
            let name = f.to_string_lossy().into_owned();
            let text = std::fs::read_to_string(&f).map_err(|e| format!("cannot read {name}: {e}"))?;
            files.push(SourceFile { name, text, std: false });
        }
        Ok(Root { files, base: p.to_path_buf() })
    }

    /// A root module of one file, `name`, with the text `src`.
    pub fn single(name: &str, src: &str) -> Root {
        let base = Path::new(name).parent().unwrap_or(Path::new("")).to_path_buf();
        Root { files: vec![SourceFile { name: name.to_string(), text: src.to_string(), std: false }], base }
    }
}

/// The result of the front end: every file of the program (indexed by
/// `Span::file`) and the checked program or the errors.
pub struct Checked {
    pub files: Vec<SourceFile>,
    pub result: Result<sema::Program, Vec<Diag>>,
}

/// Run the front end (lexer, parser, module resolution, sema) on a single
/// file. Used by `ore check` and the tests; requires `pub fn main()` unless
/// the file has `test` blocks.
pub fn check(file_name: &str, src: &str) -> Result<sema::Program, Vec<Diag>> {
    front_end(Root::single(file_name, src), None).result
}

/// The front end. `need_main` says whether the root module must have
/// `pub fn main()`; `None` means "unless it has `test` blocks".
///
/// Each stage recovers from errors and reports all of them, sorted by
/// file and position. A stage runs only if the previous one had no errors,
/// so a syntax error never causes follow-up type errors.
pub fn front_end(root: Root, need_main: Option<bool>) -> Checked {
    let (files, ast) = module::load(root.files, &root.base);
    let names: Vec<String> = files.iter().map(|f| f.name.clone()).collect();
    let diags = |errs: Vec<Error>| {
        let mut d: Vec<Diag> = errs.into_iter().map(|e| e.with_file(&names)).collect();
        d.sort_by_key(|d| (d.span.file, d.span.line, d.span.col));
        d.dedup();
        d.truncate(MAX_DIAGS);
        d
    };
    let result = ast.map_err(diags).and_then(|ast| {
        let need_main =
            need_main.unwrap_or_else(|| !ast.items.iter().any(|i| matches!(i, ast::Item::Test(t) if t.module.is_empty())));
        sema::check(&ast, need_main).map_err(diags)
    });
    Checked { files, result }
}

/// Apply the fix-its of `diags` to `src`, in order of position: a fix is
/// applied unless one of its edits overlaps an edit applied before (two
/// insertions at one position overlap too); a fix equal to an applied one
/// is skipped. Returns the new text and, for each applied fix, the position
/// of its first edit and its description.
pub fn apply_fixes(src: &str, diags: &[Diag]) -> (String, Vec<(Span, String)>) {
    let mut fixes: Vec<&Fix> = diags.iter().filter_map(|d| d.fix.as_ref()).collect();
    fixes.sort_by_key(|f| (f.edits[0].start.line, f.edits[0].start.col));
    let mut applied: Vec<&Fix> = Vec::new();
    let mut edits: Vec<(usize, usize, &str)> = Vec::new();
    for f in fixes {
        if applied.contains(&f) {
            continue;
        }
        let ranges: Vec<(usize, usize)> = f.edits.iter().map(|e| (offset(src, e.start), offset(src, e.end))).collect();
        let overlaps = ranges.iter().any(|&(s, e)| edits.iter().any(|&(ts, te, _)| (s < te && ts < e) || s == ts));
        if overlaps {
            continue;
        }
        edits.extend(ranges.iter().zip(&f.edits).map(|(&(s, e), edit)| (s, e, edit.replacement.as_str())));
        applied.push(f);
    }
    edits.sort_by_key(|&(s, ..)| std::cmp::Reverse(s));
    let mut out = src.to_string();
    for (s, e, r) in edits {
        out.replace_range(s..e, r);
    }
    (out, applied.iter().map(|f| (f.edits[0].start, f.help.clone())).collect())
}

/// A program in C, and the libraries it links with (`@link`).
pub struct CProgram {
    pub c: String,
    pub links: Vec<String>,
}

/// Run the front end, OIR lowering and C generation for an executable of a
/// single file; see `compile_root`.
pub fn compile_to_c(file_name: &str, src: &str, mode: Mode) -> Result<CProgram, Vec<Diag>> {
    compile_root(Root::single(file_name, src), mode)
}

/// Run the front end, OIR lowering and C generation for an executable;
/// `test` blocks are left out. File names are used in diagnostics, `#line`
/// directives and panic messages.
pub fn compile_root(root: Root, mode: Mode) -> Result<CProgram, Vec<Diag>> {
    let checked = front_end(root, Some(true));
    let mut typed = checked.result?;
    typed.fns.retain(|f| !sema::is_test(&f.name));
    typed.tests.clear();
    let ir = oir::lower(&typed);
    let c = cgen::emit(&ir, &file_names(&checked.files), mode != Mode::Fast, mode == Mode::Debug, false);
    Ok(CProgram { c, links: typed.links })
}

fn file_names(files: &[SourceFile]) -> Vec<String> {
    files.iter().map(|f| f.name.clone()).collect()
}

/// Like `compile_root`, for a test executable: its argument is the index
/// of the test to run. Also returns the tests and the file names.
pub fn compile_tests_to_c(root: Root, mode: Mode) -> Result<(CProgram, Vec<sema::TestDef>, Vec<String>), Vec<Diag>> {
    let checked = front_end(root, Some(false));
    let typed = checked.result?;
    let ir = oir::lower(&typed);
    let files = file_names(&checked.files);
    let c = cgen::emit(&ir, &files, mode != Mode::Fast, mode == Mode::Debug, true);
    Ok((CProgram { c, links: typed.links }, typed.tests, files))
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

/// `ore test`: compile the tests of every module of the program (std's
/// aside) and run each one in its own process, so that a panic ends only
/// that test. Only tests whose shown name (`name`, `module: name`)
/// contains `filter` run, and with `module`, only those of that module.
pub fn run_tests(root: Root, mode: Mode, filter: &str, module: Option<&str>) -> Result<Vec<TestResult>, TestError> {
    let (c, tests, files) = compile_tests_to_c(root, mode).map_err(TestError::Compile)?;
    if let Some(m) = module
        && !tests.iter().any(|t| t.module == m)
    {
        let mut with: Vec<String> = Vec::new();
        for t in tests.iter().filter(|t| !t.module.is_empty()) {
            let shown = format!("`{}`", t.module);
            if !with.contains(&shown) {
                with.push(shown);
            }
        }
        let list = if with.is_empty() { "none".to_string() } else { with.join(", ") };
        return Err(TestError::Run(format!("no tests in module `{m}` (modules with tests: {list})")));
    }
    let exe = std::env::temp_dir().join(format!("ore-test-{}", std::process::id()));
    cc(&c, &exe, mode).map_err(TestError::Run)?;
    let mut results = Vec::new();
    for (i, test) in tests.into_iter().enumerate() {
        let name = test.shown();
        if !name.contains(filter) || module.is_some_and(|m| m != test.module) {
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
        let mut failures = parse_failures(&files, &stderr);
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
/// trace. `files` are the program's files.
fn parse_failures(files: &[String], stderr: &str) -> Vec<Failure> {
    // `L:C` and the rest of a line that starts with `file:`.
    let pos = |l: &str| -> Option<(u32, u32, String)> {
        let rest = files.iter().find_map(|f| l.strip_prefix(f.as_str())?.strip_prefix(':'))?;
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

/// The clang flags for the libraries of `@link`: `-framework Name` for
/// `framework:Name`, the `--cflags --libs` of a package that `pkg-config`
/// knows, and `-lname` otherwise.
pub fn link_flags(links: &[String]) -> Vec<String> {
    let mut flags = Vec::new();
    for l in links {
        if let Some(fw) = l.strip_prefix("framework:") {
            flags.extend(["-framework".to_string(), fw.to_string()]);
            continue;
        }
        match Command::new("pkg-config").args(["--cflags", "--libs", l]).output() {
            Ok(o) if o.status.success() => {
                flags.extend(String::from_utf8_lossy(&o.stdout).split_whitespace().map(String::from));
            }
            _ => flags.push(format!("-l{l}")),
        }
    }
    flags
}

/// Compile generated C to an executable with clang, linked with the
/// program's libraries.
pub fn cc(p: &CProgram, out: &Path, mode: Mode) -> Result<(), String> {
    let mut cmd = Command::new("clang");
    cmd.args(["-std=gnu11", "-fwrapv", "-fno-strict-aliasing", "-Wno-incompatible-library-redeclaration"])
        .args(mode.clang_flags())
        .args(["-x", "c", "-", "-o"])
        .arg(out)
        // std.math calls libm.
        .arg("-lm")
        .stdin(Stdio::piped());
    // `-x none` ends `-x c`, so that a library file among the link flags is
    // not read as C (clang warns that it is unused when there is none).
    let flags = link_flags(&p.links);
    if !flags.is_empty() {
        cmd.args(["-Wno-unused-command-line-argument", "-x", "none"]).args(flags);
    }
    let mut child = cmd.spawn().map_err(|e| format!("failed to run clang: {e}"))?;
    child.stdin.take().unwrap().write_all(p.c.as_bytes()).map_err(|e| e.to_string())?;
    let status = child.wait().map_err(|e| e.to_string())?;
    if !status.success() {
        if !p.links.is_empty() {
            return Err(format!(
                "clang failed: check that the libraries of `@link` ({}) are installed (if they are, this is a compiler bug)",
                p.links.join(", ")
            ));
        }
        return Err("clang failed on the generated C (this is a compiler bug)".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_flags_for_frameworks_libraries_and_packages() {
        let flags = |names: &[&str]| link_flags(&names.iter().map(|n| n.to_string()).collect::<Vec<_>>());
        assert_eq!(flags(&["framework:Cocoa", "ore_no_such_library"]), ["-framework", "Cocoa", "-lore_no_such_library"]);
        // A package that pkg-config knows gives its flags.
        if Command::new("pkg-config").args(["--exists", "libcurl"]).status().is_ok_and(|s| s.success()) {
            assert!(flags(&["libcurl"]).iter().any(|f| f == "-lcurl"));
        }
    }
}
