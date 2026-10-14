//! Compiler driver: runs the pipeline and invokes clang.

use crate::codes;
use crate::diag::{Diag, Error, Fix, Severity, Span, json_str, offset};
use crate::manifest::ManifestFile;
use crate::module::{self, SourceFile};
use crate::{ast, cgen, oir, sema};
use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Os {
    Macos,
    Linux,
    /// WebAssembly with WASI preview 1 (wasi-libc).
    Wasip1,
    /// WebAssembly with no libc: the host provides imports.
    Freestanding,
}

impl Os {
    /// The tag of the OS in `sema::TARGET_OS` (`@targetOs()`).
    pub fn tag(self) -> usize {
        match self {
            Os::Macos => 0,
            Os::Linux => 1,
            Os::Wasip1 => 2,
            Os::Freestanding => 3,
        }
    }
}

/// What a program is built for (`--target`).
#[derive(Debug, PartialEq, Eq)]
pub struct Target {
    pub name: &'static str,
    /// clang's `--target`.
    pub triple: &'static str,
    pub os: Os,
    /// The suffix of the executable (default output name of `build`).
    pub ext: &'static str,
    /// The std modules the target lacks (`net` is `std.net`).
    pub lacks: &'static [&'static str],
}

/// WASI preview 1 has no sockets, processes or threads (`std.sync` stays,
/// single-threaded); coroutines need stack switching.
const WASIP1_LACKS: &[&str] = &["net", "http", "process", "thread", "coro"];
/// The same, and no file system, arguments or environment.
const FREESTANDING_LACKS: &[&str] = &["net", "http", "process", "thread", "coro", "fs", "os"];

/// The targets, by their `--target` names.
pub const TARGETS: &[Target] = &[
    Target {
        name: "aarch64-macos",
        triple: "aarch64-apple-macos",
        os: Os::Macos,
        ext: "",
        lacks: &[],
    },
    Target {
        name: "x86_64-macos",
        triple: "x86_64-apple-macos",
        os: Os::Macos,
        ext: "",
        lacks: &[],
    },
    Target {
        name: "aarch64-linux",
        triple: "aarch64-linux-gnu",
        os: Os::Linux,
        ext: "",
        lacks: &[],
    },
    Target {
        name: "x86_64-linux",
        triple: "x86_64-linux-gnu",
        os: Os::Linux,
        ext: "",
        lacks: &[],
    },
    Target {
        name: "wasm32-wasip1",
        triple: "wasm32-wasip1",
        os: Os::Wasip1,
        ext: ".wasm",
        lacks: WASIP1_LACKS,
    },
    Target {
        name: "wasm32-freestanding",
        triple: "wasm32-unknown-unknown",
        os: Os::Freestanding,
        ext: ".wasm",
        lacks: FREESTANDING_LACKS,
    },
];

/// The target `name`, or the text of E0117.
pub fn find_target(name: &str) -> Result<&'static Target, String> {
    TARGETS.iter().find(|t| t.name == name).ok_or_else(|| {
        let names: Vec<&str> = TARGETS.iter().map(|t| t.name).collect();
        format!("unknown target `{name}`; targets are {}", names.join(", "))
    })
}

/// The target of the machine the compiler runs on (the default).
pub fn host_target() -> &'static Target {
    let host = match (std::env::consts::ARCH, std::env::consts::OS) {
        ("aarch64", "macos") => "aarch64-macos",
        ("x86_64", "macos") => "x86_64-macos",
        ("aarch64", "linux") => "aarch64-linux",
        _ => "x86_64-linux",
    };
    find_target(host).unwrap()
}

/// Homebrew's prefix of `formula`, if brew is there and the formula is
/// installed.
fn brew_prefix(formula: &str) -> Option<PathBuf> {
    let out = Command::new("brew")
        .args(["--prefix", formula])
        .stderr(Stdio::null())
        .output()
        .ok()?;
    let dir = PathBuf::from(String::from_utf8_lossy(&out.stdout).trim());
    (out.status.success() && dir.is_dir()).then_some(dir)
}

/// A directory of the WASI toolchain: the environment variable `var`, else
/// `sub` in the Homebrew prefix of `formula`.
fn wasi_dir(var: &str, formula: &str, sub: &str) -> Option<PathBuf> {
    match std::env::var_os(var) {
        Some(d) => Some(PathBuf::from(d)),
        None => brew_prefix(formula).map(|p| p.join(sub)),
    }
    .filter(|d| d.is_dir())
}

impl Target {
    pub fn is_wasm(&self) -> bool {
        matches!(self.os, Os::Wasip1 | Os::Freestanding)
    }

    /// The integer widths of this target.
    pub fn layout(&self) -> sema::Layout {
        if self.is_wasm() {
            sema::Layout::WASM32
        } else {
            sema::Layout::LP64
        }
    }

    fn is_host(&self) -> bool {
        self.name == host_target().name
    }

    /// The clang command for this target, with the flags that select it:
    /// `--target`, and for WASI the sysroot and resource directory
    /// (`AYE_WASI_SYSROOT`, `AYE_WASI_RESOURCE_DIR`, else Homebrew's
    /// `wasi-libc` and `wasi-runtimes`). wasm targets use the clang
    /// `AYE_WASI_CLANG` names (on Linux, wasi-sdk's: a distribution's lld
    /// may be too old for wasi-sdk's libc), else Homebrew's `llvm` clang,
    /// whose Apple counterpart has no wasm backend.
    fn clang(&self) -> Result<Command, String> {
        let mut cmd = if self.is_wasm() {
            let brew = || brew_prefix("llvm").map(|p| p.join("bin/clang"));
            let clang = std::env::var_os("AYE_WASI_CLANG")
                .map(PathBuf::from)
                .or_else(brew)
                .filter(|c| c.exists());
            Command::new(clang.unwrap_or("clang".into()))
        } else {
            Command::new("clang")
        };
        if self.os == Os::Wasip1 {
            let (Some(sysroot), Some(resources)) = (
                wasi_dir("AYE_WASI_SYSROOT", "wasi-libc", "share/wasi-sysroot"),
                wasi_dir(
                    "AYE_WASI_RESOURCE_DIR",
                    "wasi-runtimes",
                    "share/wasi-runtimes",
                ),
            ) else {
                return Err(format!(
                    "target {} needs the WASI sysroot and resource directory: set AYE_WASI_SYSROOT and AYE_WASI_RESOURCE_DIR, or `brew install llvm wasi-libc wasi-runtimes`",
                    self.name
                ));
            };
            cmd.arg(format!("--target={}", self.triple))
                .arg(format!("--sysroot={}", sysroot.display()))
                .arg(format!("-resource-dir={}", resources.display()));
        } else if !self.is_host() {
            cmd.arg(format!("--target={}", self.triple));
        }
        Ok(cmd)
    }

    /// The command that runs the program `exe` built for this target:
    /// the program itself, or for WASI Node's `node:wasi` with
    /// `scripts/wasi-run.mjs` (stdio inherited, arguments and environment
    /// passed, the current directory and /tmp preopened), or for
    /// freestanding Node with `tests/wasm/loader.mjs`.
    pub fn run_command(&self, exe: &Path) -> Command {
        if !self.is_wasm() {
            return Command::new(exe);
        }
        let mut cmd = Command::new("node");
        // `--single-threaded-gc`: Node 22 segfaults in a GC thread once a
        // wasm heap passes 32 MiB.
        cmd.args([
            "--no-warnings",
            "--single-threaded-gc",
            "--input-type=module",
            "-e",
            if self.os == Os::Freestanding {
                JS_LOADER
            } else {
                WASI_RUNNER
            },
        ])
        .arg(exe);
        cmd
    }

    /// Can programs be built for this target here: a native one always
    /// (clang may still lack its sysroot), WASI when the sysroot is there
    /// and clang has the wasm32 backend.
    pub fn toolchain_available(&self) -> bool {
        let Ok(cmd) = self.clang() else { return false };
        !self.is_wasm()
            || Command::new(cmd.get_program())
                .arg("--print-targets")
                .output()
                .is_ok_and(|o| String::from_utf8_lossy(&o.stdout).contains("wasm32"))
    }
}

/// The Node script that runs a WASI program (`Target::run_command`).
const WASI_RUNNER: &str = include_str!("../scripts/wasi-run.mjs");

/// The Node script that runs a freestanding program (`tests/wasm/loader.mjs`).
const JS_LOADER: &str = include_str!("../tests/wasm/loader.mjs");

/// Is `node` there to run wasm programs?
pub fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

/// The root module of a program: its files, and the directory that
/// `import` paths start from.
pub struct Root {
    pub files: Vec<SourceFile>,
    pub base: PathBuf,
    /// The examples of doc comments are tests (`aye test`).
    pub doc_tests: bool,
    /// What the program is built for.
    pub target: &'static Target,
    /// The `aye.toml` of the program (`docs/decisions.md`, "Dependencies
    /// (O4)"), if there is one.
    pub manifest: Option<ManifestFile>,
    /// Where URL dependencies are kept; `None` is `$AYE_CACHE` and the
    /// other defaults (`fetch::cache_root`).
    pub cache: Option<PathBuf>,
    /// Record the member accesses sema resolves in `Checked::members`
    /// (`aye lsp`); off, the compiler does no work for them.
    pub members: bool,
    /// `--no-link lib`: the `@linkOptional` libraries to treat as absent
    /// even where they are installed.
    pub no_link: Vec<String>,
}

impl Root {
    /// The root module at `path`: a directory (every `.aye` file in it), or
    /// a single file, which is a module of its own (the other files in its
    /// directory are not part of it). Imports are found in the directory.
    pub fn load(path: &str) -> Result<Root, String> {
        Root::load_in(path, None)
    }

    /// Like `load`, with the `aye.toml` of the nearest directory from the
    /// root module's up to `top` (`aye test DIR`); without `top`, only the
    /// root module's own.
    pub fn load_in(path: &str, top: Option<&Path>) -> Result<Root, String> {
        let mut root = Root::load_files(path)?;
        root.manifest = match top {
            Some(top) => ManifestFile::nearest(&root.base, top)?,
            None => ManifestFile::read(&root.base)?,
        };
        Ok(root)
    }

    fn load_files(path: &str) -> Result<Root, String> {
        let p = Path::new(path);
        if !p.is_dir() {
            let text =
                std::fs::read_to_string(p).map_err(|e| format!("cannot read {path}: {e}"))?;
            return Ok(Root::single(path, &text));
        }
        let mut names: Vec<PathBuf> = std::fs::read_dir(p)
            .map_err(|e| format!("cannot read {path}: {e}"))?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|f| f.is_file() && f.extension().is_some_and(|e| e == "aye"))
            .collect();
        names.sort();
        if names.is_empty() {
            return Err(format!("{path} has no .aye files"));
        }
        let mut files = Vec::new();
        for f in names {
            let name = f.to_string_lossy().into_owned();
            let text =
                std::fs::read_to_string(&f).map_err(|e| format!("cannot read {name}: {e}"))?;
            files.push(SourceFile {
                name,
                text,
                std: false,
                dep: false,
            });
        }
        Ok(Root {
            files,
            base: p.to_path_buf(),
            doc_tests: false,
            target: host_target(),
            manifest: None,
            cache: None,
            members: false,
            no_link: Vec::new(),
        })
    }

    /// A root module of one file, `name`, with the text `src`.
    pub fn single(name: &str, src: &str) -> Root {
        let base = Path::new(name)
            .parent()
            .unwrap_or(Path::new(""))
            .to_path_buf();
        Root {
            files: vec![SourceFile {
                name: name.to_string(),
                text: src.to_string(),
                std: false,
                dep: false,
            }],
            base,
            doc_tests: false,
            target: host_target(),
            manifest: None,
            cache: None,
            members: false,
            no_link: Vec::new(),
        }
    }
}

/// The result of the front end: every file of the program (indexed by
/// `Span::file`) and the checked program or the errors.
pub struct Checked {
    pub files: Vec<SourceFile>,
    pub result: Result<sema::Program, Vec<Diag>>,
    /// With `Root::members`: the member accesses sema resolved, by the
    /// span of the member's name (`sema::check_members`); empty if sema
    /// did not run.
    pub members: Vec<(Span, sema::ResolvedMember)>,
}

/// Run the front end (lexer, parser, module resolution, sema) on a single
/// file. Used by `aye check` and the tests; requires `pub fn main()` unless
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
    let (files, ast) = module::load(
        root.files,
        &root.base,
        root.doc_tests,
        root.target,
        root.manifest,
        root.cache,
    );
    let names: Vec<String> = files.iter().map(|f| f.name.clone()).collect();
    let diags = |errs: Vec<Error>| {
        let mut d: Vec<Diag> = errs.into_iter().map(|e| e.with_file(&names)).collect();
        d.sort_by_key(|d| (d.span.file, d.span.line, d.span.col));
        d.dedup();
        d.truncate(MAX_DIAGS);
        d
    };
    let mut members = Vec::new();
    let result = ast.map_err(diags).and_then(|ast| {
        let need_main = need_main.unwrap_or_else(|| {
            !ast.items
                .iter()
                .any(|i| matches!(i, ast::Item::Test(t) if t.module.is_empty()))
        });
        let layout = root.target.layout();
        let result = if root.members {
            let (result, found) = sema::check_members(&ast, need_main, &names, layout);
            members = found;
            result
        } else {
            sema::check(&ast, need_main, &names, layout)
        };
        result.map_err(diags)
    });
    Checked {
        files,
        result,
        members,
    }
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
        let ranges: Vec<(usize, usize)> = f
            .edits
            .iter()
            .map(|e| (offset(src, e.start), offset(src, e.end)))
            .collect();
        let overlaps = ranges.iter().any(|&(s, e)| {
            edits
                .iter()
                .any(|&(ts, te, _)| (s < te && ts < e) || s == ts)
        });
        if overlaps {
            continue;
        }
        edits.extend(
            ranges
                .iter()
                .zip(&f.edits)
                .map(|(&(s, e), edit)| (s, e, edit.replacement.as_str())),
        );
        applied.push(f);
    }
    edits.sort_by_key(|&(s, ..)| std::cmp::Reverse(s));
    let mut out = src.to_string();
    for (s, e, r) in edits {
        out.replace_range(s..e, r);
    }
    (
        out,
        applied
            .iter()
            .map(|f| (f.edits[0].start, f.help.clone()))
            .collect(),
    )
}

/// A program in C, and the libraries it links with (`@link`).
pub struct CProgram {
    pub c: String,
    /// Where the functions are in `c`, to compile it in parts.
    pub parts: cgen::Parts,
    pub links: Vec<String>,
    /// More flags for clang (`--cflags`), such as `-fsanitize=thread`.
    pub cflags: Vec<String>,
    /// Keep the `-g` of the mode's flags (and run `dsymutil` after the link
    /// on macOS, a `.dSYM` next to the executable). `aye test` leaves it
    /// out unless `--cflags` is given.
    pub debug: bool,
    pub target: &'static Target,
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
    let target = root.target;
    let no_link = root.no_link.clone();
    // A freestanding program may only export functions.
    let checked = front_end(root, Some(target.os != Os::Freestanding));
    let mut typed = checked.result?;
    typed.fns.retain(|f| !sema::is_test(&f.name));
    typed.tests.clear();
    let links = linked(&typed, target, &no_link)?;
    let ir = oir::lower(&typed, &links, target.os.tag());
    let (c, parts) = cgen::emit(
        &ir,
        &file_names(&checked.files),
        mode != Mode::Fast,
        mode == Mode::Debug,
        false,
    );
    Ok(CProgram {
        c,
        parts,
        links,
        cflags: Vec::new(),
        debug: true,
        target,
    })
}

/// The libraries the program is linked with: those of `@link`, and those
/// of `@linkOptional` that are installed (none are on a wasm target) and
/// not named by `--no-link`. A `--no-link` name that no `@linkOptional`
/// declares is E0100.
fn linked(
    p: &sema::Program,
    target: &Target,
    no_link: &[String],
) -> Result<Vec<String>, Vec<Diag>> {
    let unknown: Vec<Diag> = no_link
        .iter()
        .filter(|n| !p.optional_links.contains(n))
        .map(|n| Diag {
            file: "<command line>".to_string(),
            span: Span::new(1, 1),
            end: None,
            severity: Severity::Error,
            code: codes::E0100,
            msg: format!("`--no-link {n}`: no `@linkOptional(\"{n}\")` in the program"),
            fix: None,
        })
        .collect();
    if !unknown.is_empty() {
        return Err(unknown);
    }
    Ok(p.links
        .iter()
        .chain(
            p.optional_links
                .iter()
                .filter(|l| !target.is_wasm() && !no_link.contains(l) && link_available(l)),
        )
        .cloned()
        .collect())
}

/// Is the library `name` (as `@link` names it) installed: a framework
/// that exists, a package that `pkg-config` knows, or a library that
/// clang can link with (`-lname`)?
pub fn link_available(name: &str) -> bool {
    if let Some(fw) = name.strip_prefix("framework:") {
        return ["/System/Library/Frameworks", "/Library/Frameworks"]
            .iter()
            .any(|d| Path::new(d).join(format!("{fw}.framework")).exists());
    }
    if Command::new("pkg-config")
        .args(["--exists", name])
        .status()
        .is_ok_and(|s| s.success())
    {
        return true;
    }
    let exe = std::env::temp_dir().join(format!("aye-link-{}-{name}", std::process::id()));
    let child = Command::new("clang")
        .args(["-x", "c", "-", "-o"])
        .arg(&exe)
        .arg(format!("-l{name}"))
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    let Ok(mut child) = child else { return false };
    let _ = child
        .stdin
        .take()
        .unwrap()
        .write_all(b"int main(void) { return 0; }\n");
    let ok = child.wait().is_ok_and(|s| s.success());
    let _ = std::fs::remove_file(&exe);
    ok
}

fn file_names(files: &[SourceFile]) -> Vec<String> {
    files.iter().map(|f| f.name.clone()).collect()
}

/// Like `compile_root`, for a test executable: its argument is the index
/// of the test to run. Also returns the tests, the file names and the
/// directories of the program's modules (std's aside).
pub fn compile_tests_to_c(
    root: Root,
    mode: Mode,
) -> Result<(CProgram, Vec<sema::TestDef>, Vec<String>, Vec<PathBuf>), Vec<Diag>> {
    let target = root.target;
    let no_link = root.no_link.clone();
    let checked = front_end(
        Root {
            doc_tests: true,
            ..root
        },
        Some(false),
    );
    let typed = checked.result?;
    let links = linked(&typed, target, &no_link)?;
    let ir = oir::lower(&typed, &links, target.os.tag());
    let files = file_names(&checked.files);
    let mut dirs: Vec<PathBuf> = Vec::new();
    for f in checked.files.iter().filter(|f| !f.std && !f.dep) {
        let dir = Path::new(&f.name)
            .parent()
            .filter(|d| !d.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let dir = std::fs::canonicalize(dir).unwrap_or(dir.to_path_buf());
        if !dirs.contains(&dir) {
            dirs.push(dir);
        }
    }
    let (c, parts) = cgen::emit(&ir, &files, mode != Mode::Fast, mode == Mode::Debug, true);
    Ok((
        CProgram {
            c,
            parts,
            links,
            cflags: Vec::new(),
            debug: true,
            target,
        },
        typed.tests,
        files,
        dirs,
    ))
}

/// The outcome of one test.
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub stdout: String,
    pub stderr: String,
    /// Why the test failed, read from its stderr.
    pub failures: Vec<Failure>,
    /// The wall time of the test's process.
    pub ms: u64,
    /// The CPU time (user + system) of the test's process.
    pub cpu_ms: u64,
    /// The test was killed for going over the time limit.
    pub timed_out: bool,
}

/// What `aye test` ran.
pub struct TestRun {
    pub results: Vec<TestResult>,
    /// How many modules had a test that ran.
    pub modules: usize,
    /// The wall time of the whole run, compiling included.
    pub ms: u64,
}

pub enum Failure {
    /// `kind` is `panic`, `assertion` or `leak`.
    At {
        kind: &'static str,
        line: u32,
        col: u32,
        message: String,
    },
    /// The test returned an error; the trace is empty outside debug mode.
    Error {
        name: String,
        trace: Vec<(u32, u32)>,
    },
    /// Ended in another way, such as a signal.
    Crash(String),
    /// The root (a directory under `aye test DIR`) did not compile; the
    /// message holds its diagnostics.
    Compile(String),
}

pub enum TestError {
    Compile(Vec<Diag>),
    /// clang or the test executable could not run.
    Run(String),
}

/// The directories under `dir` (`dir` first) that have `.aye` files: the
/// roots of `aye test DIR`. Directories whose name starts with `.` are left
/// out.
fn module_roots(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| format!("cannot read {}: {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    entries.sort();
    if entries
        .iter()
        .any(|f| f.is_file() && f.extension().is_some_and(|e| e == "aye"))
    {
        out.push(dir.to_path_buf());
    }
    for d in entries.iter().filter(|d| {
        d.is_dir()
            && !d
                .file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with('.'))
    }) {
        module_roots(d, out)?;
    }
    Ok(())
}

/// `aye test`: compile the tests of every module of the program (std's
/// aside) and run each one in its own process, so that a panic ends only
/// that test. `path` is a file, or a directory: then every directory under
/// it with `.aye` files is a root, and a module that several roots reach is
/// tested once. Only tests whose shown name (`name`, `module: name`, the
/// module relative to `path`) contains `filter` run, and with `module`,
/// only those of that module. `cflags` are more flags for clang; `no_link`
/// are `@linkOptional` libraries to treat as absent. Up to
/// `jobs` test processes run at once; `results` stay in test order. A test
/// process that uses more than `time_limit` seconds of CPU time (0: no
/// limit) is killed and fails. Of several directory roots, one that does
/// not compile is a failed result (`Failure::Compile`) and the run goes on;
/// a single root that does not compile is an error.
pub fn run_tests(
    path: &str,
    target: &'static Target,
    mode: Mode,
    filter: &str,
    module: Option<&str>,
    cflags: &[String],
    no_link: &[String],
    jobs: usize,
    time_limit: u64,
) -> Result<TestRun, TestError> {
    let start = Instant::now();
    let top = Path::new(path);
    let mut roots = Vec::new();
    if top.is_dir() {
        module_roots(top, &mut roots).map_err(TestError::Run)?;
        if roots.is_empty() {
            return Err(TestError::Run(format!("{path} has no .aye files")));
        }
    } else {
        roots.push(top.to_path_buf());
    }
    let roots_n = roots.len();
    let exe = std::env::temp_dir().join(format!("aye-test-{}", std::process::id()));
    let mut reached: HashSet<PathBuf> = HashSet::new();
    let mut with_tests: Vec<String> = Vec::new();
    let mut ran: HashSet<String> = HashSet::new();
    let mut results = Vec::new();
    for dir in roots {
        let canon = std::fs::canonicalize(&dir).unwrap_or(dir.clone());
        if top.is_dir() && reached.contains(&canon) {
            continue;
        }
        let mut root = Root::load_in(&dir.to_string_lossy(), top.is_dir().then_some(top))
            .map_err(TestError::Run)?;
        root.target = target;
        root.no_link = no_link.to_vec();
        let base = root.base.clone();
        let prefix: Vec<String> = base
            .strip_prefix(top)
            .map(|p| {
                p.components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        let (mut c, tests, files, dirs) = match compile_tests_to_c(root, mode) {
            Ok(r) => r,
            Err(ds) if roots_n > 1 => {
                // A root that does not compile is one failed entry; the
                // other roots still run.
                let name = if prefix.is_empty() {
                    path.to_string()
                } else {
                    prefix.join(".")
                };
                if name.contains(filter) && module.is_none_or(|m| m == prefix.join(".")) {
                    let message = ds.iter().map(|d| d.render()).collect::<Vec<_>>().join("\n");
                    results.push(TestResult {
                        name,
                        passed: false,
                        stdout: String::new(),
                        stderr: String::new(),
                        failures: vec![Failure::Compile(message)],
                        ms: 0,
                        cpu_ms: 0,
                        timed_out: false,
                    });
                }
                continue;
            }
            Err(ds) => return Err(TestError::Compile(ds)),
        };
        c.cflags = cflags.to_vec();
        // The failure positions of a test come from constants, not from
        // DWARF; only `--cflags` (a sanitizer's report) needs the symbols,
        // and `-g` is 45% of the compile time of a big test executable.
        c.debug = !cflags.is_empty();
        let mut todo = Vec::new();
        for (i, mut test) in tests.into_iter().enumerate() {
            let at =
                std::fs::canonicalize(base.join(test.module.replace('.', "/"))).unwrap_or_default();
            if reached.contains(&at) {
                continue;
            }
            test.module = prefix
                .iter()
                .map(String::as_str)
                .chain(Some(test.module.as_str()).filter(|m| !m.is_empty()))
                .collect::<Vec<_>>()
                .join(".");
            if !test.module.is_empty() && !with_tests.contains(&test.module) {
                with_tests.push(test.module.clone());
            }
            let name = test.shown();
            if name.contains(filter) && module.is_none_or(|m| m == test.module) {
                todo.push((i, name, test.module));
            }
        }
        reached.extend(dirs);
        if todo.is_empty() {
            continue;
        }
        if let Err(e) = cc(&c, &exe, mode) {
            remove_dsym(&exe);
            return Err(TestError::Run(e));
        }
        warm_up(c.target, &exe);
        let done = run_parallel(jobs, todo.len(), |k| {
            let (i, name, _) = &todo[k];
            run_one(c.target, &exe, *i, name.clone(), &files, time_limit)
        });
        let _ = std::fs::remove_file(&exe);
        remove_dsym(&exe);
        for ((_, _, module), r) in todo.into_iter().zip(done) {
            ran.insert(module);
            results.push(r.map_err(TestError::Run)?);
        }
    }
    if let Some(m) = module
        && results.is_empty()
        && !with_tests.iter().any(|t| t == m)
    {
        let list = if with_tests.is_empty() {
            "none".to_string()
        } else {
            with_tests
                .iter()
                .map(|t| format!("`{t}`"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        return Err(TestError::Run(format!(
            "no tests in module `{m}` (modules with tests: {list})"
        )));
    }
    Ok(TestRun {
        results,
        modules: ran.len(),
        ms: start.elapsed().as_millis() as u64,
    })
}

/// Run `f(0)` .. `f(n - 1)` on up to `jobs` threads; the results are in
/// the order of the indices.
fn run_parallel<T: Send>(jobs: usize, n: usize, f: impl Fn(usize) -> T + Sync) -> Vec<T> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let next = AtomicUsize::new(0);
    let mut done: Vec<(usize, T)> = std::thread::scope(|s| {
        let workers: Vec<_> = (0..jobs.clamp(1, n.max(1)))
            .map(|_| {
                s.spawn(|| {
                    let mut mine = Vec::new();
                    loop {
                        let k = next.fetch_add(1, Ordering::Relaxed);
                        if k >= n {
                            return mine;
                        }
                        mine.push((k, f(k)));
                    }
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|w| w.join().unwrap())
            .collect()
    });
    done.sort_by_key(|(k, _)| *k);
    done.into_iter().map(|(_, t)| t).collect()
}

/// `SIGKILL`, and `SIGXCPU`, which some systems send at the CPU limit.
const SIGKILL: i32 = 9;
const SIGXCPU: i32 = 24;

/// Run test `i` of `exe` in its own process and report it. The process may
/// use `time_limit` seconds of CPU time (user + system, from `wait4`; 0: no
/// limit): the kernel kills it at that limit (`RLIMIT_CPU`). It is also
/// killed after 10 times the limit (at least 60 s) of wall time, so that a
/// test that sleeps cannot hang the run.
/// The first launch of a new executable is slow on macOS (`syspolicyd`
/// assesses it, ~0.5 s): one launch without arguments (it exits at once)
/// before the timed test processes. Native targets only; wasm runs under
/// node.
fn warm_up(target: &Target, exe: &Path) {
    if !target.is_wasm() {
        let _ = target
            .run_command(exe)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

fn run_one(
    target: &Target,
    exe: &Path,
    i: usize,
    name: String,
    files: &[String],
    time_limit: u64,
) -> Result<TestResult, String> {
    use std::io::Read;
    use std::os::unix::process::{CommandExt, ExitStatusExt};
    use std::time::Duration;
    // `tv_usec` is an int on macOS and a long on Linux: 16 bytes either way,
    // and the low half holds it on little-endian machines.
    #[repr(C)]
    struct Timeval {
        sec: i64,
        usec: i32,
    }
    #[repr(C)]
    struct Rusage {
        utime: Timeval,
        stime: Timeval,
        rest: [i64; 14],
    }
    #[repr(C)]
    struct Rlimit {
        cur: u64,
        max: u64,
    }
    unsafe extern "C" {
        fn wait4(pid: i32, status: *mut i32, options: i32, rusage: *mut Rusage) -> i32;
        fn setrlimit(resource: i32, limit: *const Rlimit) -> i32;
    }
    const WNOHANG: i32 = 1;
    const RLIMIT_CPU: i32 = 0;
    let fail = |e: std::io::Error| format!("cannot run {}: {e}", exe.display());
    let mut cmd = target.run_command(exe);
    cmd.arg(i.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if time_limit > 0 {
        // SAFETY: `setrlimit` is async-signal-safe, as `pre_exec` requires.
        unsafe {
            cmd.pre_exec(move || {
                // The hard limit stays above the soft one: with the two
                // equal, Linux sends `SIGKILL` at the limit (and accounts the
                // CPU a little under it), with it above, `SIGXCPU`.
                let l = Rlimit {
                    cur: time_limit,
                    max: time_limit + 1,
                };
                setrlimit(RLIMIT_CPU, &l);
                Ok(())
            })
        };
    }
    let began = Instant::now();
    let mut child = cmd.spawn().map_err(fail)?;
    let pid = child.id() as i32;
    // The pipes are read while the test runs, so that its output cannot
    // fill them and block it.
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let out_reader = std::thread::spawn(move || {
        let mut v = Vec::new();
        let _ = stdout.read_to_end(&mut v);
        v
    });
    let err_reader = std::thread::spawn(move || {
        let mut v = Vec::new();
        let _ = stderr.read_to_end(&mut v);
        v
    });
    let wall = Duration::from_secs((time_limit * 10).max(60));
    let (mut status, mut ru) = (0, unsafe { std::mem::zeroed::<Rusage>() });
    let mut hung = false;
    let mut nap = 1;
    loop {
        match unsafe { wait4(pid, &mut status, WNOHANG, &mut ru) } {
            0 => {}
            r if r == pid => break,
            _ => return Err(format!("cannot wait for {}", exe.display())),
        }
        if time_limit > 0 && began.elapsed() >= wall {
            hung = true;
            let _ = child.kill();
            unsafe { wait4(pid, &mut status, 0, &mut ru) };
            break;
        }
        std::thread::sleep(Duration::from_millis(nap));
        nap = (nap * 2).min(10);
    }
    let ms = began.elapsed().as_millis() as u64;
    let time = |t: &Timeval| t.sec as u64 * 1000 + t.usec as u64 / 1000;
    let cpu_ms = time(&ru.utime) + time(&ru.stime);
    let status = std::process::ExitStatus::from_raw(status);
    let stdout = String::from_utf8_lossy(&out_reader.join().unwrap()).into_owned();
    let stderr = String::from_utf8_lossy(&err_reader.join().unwrap()).into_owned();
    let signal = status.signal();
    // Only the CPU limit sends `SIGXCPU`, and Linux delivers it with the
    // accounted CPU time still a little under the limit. A `SIGKILL` arriving
    // at up to a tick (tens of ms) under the limit counts as well.
    let timed_out = time_limit > 0
        && (hung
            || cpu_ms > time_limit * 1000
            || signal == Some(SIGXCPU)
            || (cpu_ms + 100 >= time_limit * 1000 && signal == Some(SIGKILL)));
    let passed = status.success() && !timed_out;
    let mut failures = parse_failures(files, &stderr);
    if timed_out {
        failures.push(Failure::Crash(if hung {
            format!(
                "time limit: {:.1} s wall of {} s",
                ms as f64 / 1000.0,
                wall.as_secs()
            )
        } else {
            format!(
                "time limit: {:.1} s CPU of {time_limit} s",
                cpu_ms as f64 / 1000.0
            )
        }));
    } else if !passed && failures.is_empty() {
        failures.push(Failure::Crash(match (status.code(), signal) {
            (Some(c), _) => format!("exit status {c}"),
            (_, s) => format!("terminated by signal {}", s.unwrap_or(0)),
        }));
    }
    Ok(TestResult {
        name,
        passed,
        stdout,
        stderr,
        failures,
        ms,
        cpu_ms,
        timed_out,
    })
}

/// The failures that a test process reported on stderr: lines
/// `file:L:C: panic: ...`, `file:L:C: assertion failed: ...`,
/// `file:L:C: leak: ...`, and `error: Name` followed by the error return
/// trace. `files` are the program's files.
fn parse_failures(files: &[String], stderr: &str) -> Vec<Failure> {
    // `L:C` and the rest of a line that starts with `file:`.
    let pos = |l: &str| -> Option<(u32, u32, String)> {
        let rest = files
            .iter()
            .find_map(|f| l.strip_prefix(f.as_str())?.strip_prefix(':'))?;
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
            out.push(Failure::Error {
                name: name.to_string(),
                trace,
            });
            continue;
        }
        let Some((line, col, rest)) = pos(l) else {
            continue;
        };
        for (prefix, kind) in [
            (" panic: ", "panic"),
            (" assertion failed: ", "assertion"),
            (" leak: ", "leak"),
        ] {
            if let Some(message) = rest.strip_prefix(prefix) {
                out.push(Failure::At {
                    kind,
                    line,
                    col,
                    message: message.to_string(),
                });
            }
        }
    }
    out
}

impl TestResult {
    /// For a test that went over the time limit, how: `time limit: 10.4 s
    /// CPU of 10 s`.
    pub fn time_limit_note(&self) -> Option<&str> {
        self.failures.iter().find_map(|f| match f {
            Failure::Crash(m) if self.timed_out => Some(m.as_str()),
            _ => None,
        })
    }

    /// One entry of the `tests` array in `aye test --json`.
    pub fn to_json(&self) -> String {
        let failures: Vec<String> = self
            .failures
            .iter()
            .map(|f| match f {
                Failure::At {
                    kind,
                    line,
                    col,
                    message,
                } => {
                    format!(
                        r#"{{"kind":"{kind}","line":{line},"col":{col},"message":{}}}"#,
                        json_str(message)
                    )
                }
                Failure::Error { name, trace } => {
                    let t: Vec<String> = trace
                        .iter()
                        .map(|(l, c)| format!(r#"{{"line":{l},"col":{c}}}"#))
                        .collect();
                    format!(
                        r#"{{"kind":"error","error":{},"trace":[{}]}}"#,
                        json_str(name),
                        t.join(",")
                    )
                }
                Failure::Crash(m) => format!(r#"{{"kind":"crash","message":{}}}"#, json_str(m)),
                Failure::Compile(m) => {
                    format!(r#"{{"kind":"compile","message":{}}}"#, json_str(m))
                }
            })
            .collect();
        format!(
            r#"{{"name":{},"status":"{}","ms":{},"cpu_ms":{}{},"failures":[{}],"stdout":{},"stderr":{}}}"#,
            json_str(&self.name),
            if self.passed { "pass" } else { "fail" },
            self.ms,
            self.cpu_ms,
            if self.timed_out {
                r#","timed_out":true"#
            } else {
                ""
            },
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
        match Command::new("pkg-config")
            .args(["--cflags", "--libs", l])
            .output()
        {
            Ok(o) if o.status.success() => {
                flags.extend(
                    String::from_utf8_lossy(&o.stdout)
                        .split_whitespace()
                        .map(String::from),
                );
            }
            _ => flags.push(format!("-l{l}")),
        }
    }
    flags
}

/// Compile generated C to an executable with clang, linked with the
/// program's libraries.
pub fn cc(p: &CProgram, out: &Path, mode: Mode) -> Result<(), String> {
    compile(p, out, mode, true)
}

/// Compile generated C to an object file (`clang -c`), without linking.
/// The generated C includes no libc header, so a foreign target needs no
/// sysroot for this.
pub fn cc_obj(p: &CProgram, out: &Path, mode: Mode) -> Result<(), String> {
    compile(p, out, mode, false)
}

/// `cc` and `cc_obj`: `link` is whether clang links.
fn compile(p: &CProgram, out: &Path, mode: Mode, link: bool) -> Result<(), String> {
    // The C goes to a file, not to clang's stdin: a pipe holds 64 KiB, and
    // clang may exit (a bad flag) before it has read all of the source,
    // which would kill this process with SIGPIPE.
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let name = format!(
        "aye-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    );
    // A big program's C is compiled in parts at once, one per core (up to
    // 8), then linked: one clang uses one core. Not for an object file nor
    // for another target (wasm's link is its own).
    if link && p.target.is_host() {
        let k = std::thread::available_parallelism().map_or(1, |n| n.get().min(8));
        let header = format!("{name}.h");
        if let Some((h, units)) = p.parts.split(&p.c, &header, k) {
            return compile_parts(p, &name, &h, &units, out, mode);
        }
    }
    let src = std::env::temp_dir().join(format!("{name}.c"));
    std::fs::write(&src, &p.c).map_err(|e| e.to_string())?;
    let result = cc_file(p, &src, out, mode, link);
    let _ = std::fs::remove_file(&src);
    result
}

/// `compile` for the C in parts (`cgen::Parts::split`): the header and
/// each unit go to files named after `name`, each unit is compiled to an
/// object by its own clang, all at once, and the objects are linked.
fn compile_parts(
    p: &CProgram,
    name: &str,
    header: &str,
    units: &[String],
    out: &Path,
    mode: Mode,
) -> Result<(), String> {
    let dir = std::env::temp_dir();
    let h = dir.join(format!("{name}.h"));
    let srcs: Vec<PathBuf> = (0..units.len())
        .map(|i| dir.join(format!("{name}-{i}.c")))
        .collect();
    let objs: Vec<PathBuf> = (0..units.len())
        .map(|i| dir.join(format!("{name}-{i}.o")))
        .collect();
    let result = (|| {
        std::fs::write(&h, header).map_err(|e| e.to_string())?;
        for (src, unit) in srcs.iter().zip(units) {
            std::fs::write(src, unit).map_err(|e| e.to_string())?;
        }
        // The flags are those of one clang that compiles and links; each
        // step uses only some of them.
        let compiled = std::thread::scope(|s| {
            let jobs: Vec<_> = srcs
                .iter()
                .zip(&objs)
                .map(|(src, obj)| {
                    s.spawn(move || {
                        let mut cmd = clang_cmd(p, mode, false)?;
                        cmd.args(["-Wno-unused-command-line-argument", "-x", "c"])
                            .arg(src)
                            .arg("-o")
                            .arg(obj);
                        cmd.status()
                            .map_err(|e| format!("failed to run clang: {e}"))
                    })
                })
                .collect();
            jobs.into_iter()
                .map(|j| j.join().unwrap())
                .collect::<Result<Vec<_>, String>>()
        })?;
        if !compiled.iter().all(|s| s.success()) {
            return Err(clang_failed(p, true));
        }
        let mut cmd = clang_cmd(p, mode, true)?;
        cmd.arg("-Wno-unused-command-line-argument")
            .args(&objs)
            .arg("-o")
            .arg(out);
        link_args(p, &mut cmd);
        let status = cmd
            .status()
            .map_err(|e| format!("failed to run clang: {e}"))?;
        if !status.success() {
            return Err(clang_failed(p, true));
        }
        // clang runs `dsymutil` itself only when it also compiles: without
        // it, the debug information stays in the objects, which are
        // removed.
        if p.debug && p.target.os == Os::Macos && debug_info(mode, &p.cflags) {
            let status = Command::new("dsymutil")
                .arg(out)
                .status()
                .map_err(|e| format!("failed to run dsymutil: {e}"))?;
            if !status.success() {
                return Err(clang_failed(p, true));
            }
        }
        Ok(())
    })();
    let _ = std::fs::remove_file(&h);
    for f in srcs.iter().chain(&objs) {
        let _ = std::fs::remove_file(f);
    }
    result
}

/// Remove the `.dSYM` folder that `dsymutil` (or clang) left next to the
/// executable `exe`.
pub fn remove_dsym(exe: &Path) {
    let mut dsym = exe.as_os_str().to_owned();
    dsym.push(".dSYM");
    let _ = std::fs::remove_dir_all(dsym);
}

/// Do these flags give debug information (the last `-g` flag, as clang
/// reads them)?
fn debug_info(mode: Mode, cflags: &[String]) -> bool {
    mode.clang_flags()
        .iter()
        .copied()
        .chain(cflags.iter().map(String::as_str))
        .rfind(|f| f.starts_with("-g"))
        .is_some_and(|f| f != "-g0")
}

/// `compile` for the C source in the file `src`.
fn cc_file(p: &CProgram, src: &Path, out: &Path, mode: Mode, link: bool) -> Result<(), String> {
    let mut cmd = clang_cmd(p, mode, link)?;
    cmd.args(["-x", "c"]).arg(src).arg("-o").arg(out);
    if link {
        link_args(p, &mut cmd);
    }
    let status = cmd
        .status()
        .map_err(|e| format!("failed to run clang: {e}"))?;
    if !status.success() {
        return Err(clang_failed(p, link));
    }
    Ok(())
}

/// The clang command with the flags of the target, the mode and
/// `--cflags`, before the input; `link` is whether it links.
fn clang_cmd(p: &CProgram, mode: Mode, link: bool) -> Result<Command, String> {
    let freestanding = p.target.os == Os::Freestanding;
    let mut cmd = p.target.clang()?;
    cmd.args(["-std=gnu11", "-fwrapv", "-fno-strict-aliasing"]);
    if !link {
        cmd.arg("-c");
    }
    // `-pthread`: std.thread and std.sync call pthreads (wasi-libc has
    // none, and `-pthread` breaks its link).
    if !p.target.is_wasm() {
        cmd.arg("-pthread");
    } else if link {
        // wasm-ld's stack is 64 KiB, with no guard: 8 MiB, like a native
        // main thread's. It is placed first, at addresses 0 to
        // `__global_base`: the prologue check of `AYE_STACK_CHECK` relies
        // on that.
        cmd.args(["-Wl,-z,stack-size=8388608", "-Wl,--stack-first"]);
    }
    // No libc and no entry point: the prelude has its own runtime, and the
    // exported functions are the roots. An `extern fn` without a body (or a
    // libc function the runtime lacks) becomes an import from `env`.
    if freestanding {
        cmd.args(["-DAYE_FREESTANDING", "-ffreestanding", "-nostdlib"]);
        if link {
            cmd.args(["-Wl,--no-entry", "-Wl,--allow-undefined"]);
        }
    }
    // An object for another OS or architecture: clang's hosted `stdint.h`
    // does an `#include_next` into the host's libc headers, which are not
    // the target's (glibc's `bits/libc-header-start.h`). Freestanding uses
    // clang's own headers, which the three the prelude includes need.
    if !link && !p.target.is_wasm() && !p.target.is_host() {
        cmd.arg("-ffreestanding");
    }
    if link {
        let sysroot = std::env::var_os("AYE_SYSROOT").map(PathBuf::from);
        cmd.args(cross_link_args(p.target, host_target(), sysroot.as_deref()));
    }
    cmd.arg("-Wno-incompatible-library-redeclaration")
        .args(mode.clang_flags().iter().filter(|f| p.debug || **f != "-g"))
        .args(&p.cflags);
    Ok(cmd)
}

/// The flags that link for a Linux `target` on a macOS `host`: the Linux
/// sysroot (`AYE_SYSROOT`: a copy of the target's `/usr/include`, `/usr/lib`
/// and `/lib`) and lld, since Apple's ld64 links only Mach-O. None without a
/// sysroot (clang then fails, and the error suggests `-c`).
fn cross_link_args(target: &Target, host: &Target, sysroot: Option<&Path>) -> Vec<String> {
    match sysroot {
        Some(dir) if target.os == Os::Linux && host.os == Os::Macos => vec![
            format!("--sysroot={}", dir.display()),
            "-fuse-ld=lld".to_string(),
        ],
        _ => Vec::new(),
    }
}

/// The libraries at the end of a clang command that links.
fn link_args(p: &CProgram, cmd: &mut Command) {
    // std.math calls libm.
    if p.target.os != Os::Freestanding {
        cmd.arg("-lm");
    }
    // `-x none` ends `-x c`, so that a library file among the link flags is
    // not read as C (clang warns that it is unused when there is none).
    // `-lm` is always there.
    let links: Vec<String> = p.links.iter().filter(|l| *l != "m").cloned().collect();
    let flags = link_flags(&links);
    if !flags.is_empty() {
        cmd.args(["-Wno-unused-command-line-argument", "-x", "none"])
            .args(flags);
    }
}

/// The error for a clang that failed; `link` is whether it linked.
fn clang_failed(p: &CProgram, link: bool) -> String {
    if link && !p.links.is_empty() {
        return format!(
            "clang failed: check that the libraries of `@link` ({}) are installed (if they are, this is a compiler bug)",
            p.links.join(", ")
        );
    }
    if !p.target.is_host() {
        let hint = if link {
            if p.target.os == Os::Linux
                && host_target().os == Os::Macos
                && std::env::var_os("AYE_SYSROOT").is_none()
            {
                "; to link, set AYE_SYSROOT to a Linux sysroot (lld links); `aye build -c` compiles to an object file without linking, which needs no sysroot"
            } else {
                "; `aye build -c` compiles to an object file without linking, which needs no sysroot"
            }
        } else {
            ""
        };
        return format!(
            "clang failed for target {} (check that its toolchain and sysroot are installed; if they are, this is a compiler bug{hint})",
            p.target.name
        );
    }
    "clang failed on the generated C (this is a compiler bug)".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warm_up_launches_a_native_executable_once_and_skips_wasm() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("aye-warm-up-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("exe");
        let log = dir.join("launches");
        std::fs::write(
            &exe,
            format!("#!/bin/sh\necho \"$#\" >> '{}'\nexit 2\n", log.display()),
        )
        .unwrap();
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();
        warm_up(find_target("wasm32-wasip1").unwrap(), &exe);
        assert!(!log.exists(), "wasm runs under node: no warm-up launch");
        warm_up(find_target("aarch64-macos").unwrap(), &exe);
        // One launch, with no arguments (a test index would run a test).
        assert_eq!(std::fs::read_to_string(&log).unwrap(), "0\n");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cross_link_args_for_linux_from_macos() {
        let t = |n| find_target(n).unwrap();
        let dir = Path::new("/sr");
        assert_eq!(
            cross_link_args(t("x86_64-linux"), t("aarch64-macos"), Some(dir)),
            ["--sysroot=/sr", "-fuse-ld=lld"]
        );
        assert_eq!(
            cross_link_args(t("aarch64-linux"), t("x86_64-macos"), Some(dir)),
            ["--sysroot=/sr", "-fuse-ld=lld"]
        );
        assert!(cross_link_args(t("x86_64-linux"), t("aarch64-macos"), None).is_empty());
        assert!(cross_link_args(t("x86_64-linux"), t("aarch64-linux"), Some(dir)).is_empty());
        assert!(cross_link_args(t("x86_64-macos"), t("aarch64-macos"), Some(dir)).is_empty());
        assert!(cross_link_args(t("wasm32-wasip1"), t("aarch64-macos"), Some(dir)).is_empty());
    }

    #[test]
    fn link_flags_for_frameworks_libraries_and_packages() {
        let flags =
            |names: &[&str]| link_flags(&names.iter().map(|n| n.to_string()).collect::<Vec<_>>());
        assert_eq!(
            flags(&["framework:Cocoa", "aye_no_such_library"]),
            ["-framework", "Cocoa", "-laye_no_such_library"]
        );
        // A package that pkg-config knows gives its flags.
        if Command::new("pkg-config")
            .args(["--exists", "libcurl"])
            .status()
            .is_ok_and(|s| s.success())
        {
            assert!(flags(&["libcurl"]).iter().any(|f| f == "-lcurl"));
        }
    }
}
