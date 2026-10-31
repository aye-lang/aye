use aye::driver::{self, Mode, Root};
use aye::{codes, diag, doc, fmt, lsp};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "usage (<path> is a .aye file, or a directory: a module of all its .aye files; none means `.`):
  aye build <path> [-o out] [-c] [--mode debug|safe|fast] [--leak-check] [--cflags <flags>] [--no-link <lib>] [--target <target>]
  aye run <path> [--mode debug|safe|fast] [--leak-check] [--cflags <flags>] [--no-link <lib>] [--target <target>] [-- args...]
  aye emit-c <path> [--mode debug|safe|fast] [--target <target>]
  aye check <path> [--json] [--target <target>]
  aye fix <path>
  aye test <path> [--filter <text>] [--module <module>] [--json] [--time-limit <seconds>] [-j <n>] [--mode debug|safe|fast] [--leak-check] [--cflags <flags>] [--no-link <lib>] [--target <target>]
  (--leak-check: the leak, double-free and use-after-free checks of `General` in any mode; debug mode has them anyway)
  (--cflags: more flags for clang, such as \"-fsanitize=thread\")
  (--no-link: treat the `@linkOptional(\"<lib>\")` library as not installed: not linked, `@hasLink` false; repeatable)
  (-c: compile to an object file, `<path>.o`, without linking; for any --target, no sysroot needed)
  (AYE_SYSROOT=<dir>: a Linux sysroot, to link a Linux --target on macOS, with lld)
  (--target: aarch64-macos, x86_64-macos, aarch64-linux, x86_64-linux, wasm32-wasip1, wasm32-freestanding; default: this machine)
  aye fmt [--check] <path>      (<path>: a .aye file, or a directory searched recursively)
  aye doc --signatures <path>   (<path> may also be `std` or `std.<module>`)
  aye doc --html <path> [-o <dir>]   (HTML pages of the same documentation into <dir>, default `doc`)
  aye cimport <header> [-Idir] [-DNAME[=value]] [--link <lib>]   (print the Aye declarations `@cimport` makes of a C header)
  aye lsp                       (a language server on stdin and stdout: diagnostics and formatting)
  aye explain <code>
  aye version                   (also --version: `aye <version> (<commit> <date>)`)
  aye spec [<section> | all]    (no argument: the core of the reference and a list of the other sections)";

/// `aye --version`: `aye <Cargo.toml version> (<short commit hash> <commit
/// date>)`, the parenthesis left out when the build had no git checkout
/// (build.rs).
fn version_line() -> String {
    let v = env!("CARGO_PKG_VERSION");
    match (env!("AYE_GIT_SHA"), env!("AYE_GIT_DATE")) {
        ("", _) | (_, "") => format!("aye {v}"),
        (sha, date) => format!("aye {v} ({sha} {date})"),
    }
}

/// The language reference that `aye spec` prints.
const SPEC: &str = include_str!("../docs/spec.md");

/// The sections that `aye spec` prints with no argument, in this order:
/// what a first program needs, the std index and the pitfalls right after
/// the first program so that a read cut short still shows them. At most
/// 28000 bytes, which Claude Code shows inline.
const SPEC_CORE: &[&str] = &[
    "1",
    "15",
    "18",
    "4",
    "enums",
    "5",
    "conversions",
    "6",
    "7",
    "optionals",
    "8",
];

/// The `###` headings that start a section of their own, with its name.
/// Every `## N. Title` heading starts one too, and so does every
/// `### std.m` heading (named `std.m`).
const SPEC_SPLITS: &[(&str, &str)] = &[
    ("Functions", "functions"),
    ("Constants, globals and type aliases", "constants"),
    ("Structs and methods", "structs"),
    ("Enums (tagged unions)", "enums"),
    ("C functions and types", "c"),
    ("simd", "simd"),
    ("Conversions", "conversions"),
    ("Pointers", "pointers"),
    ("Optionals", "optionals"),
    ("match", "match"),
    ("A complete CLI", "cli"),
    ("Dependencies", "deps"),
];

/// When to read a section left out of the core, shown in its place in the
/// list of other sections instead of its title.
const SPEC_HINTS: &[(&str, &str)] = &[
    ("functions", "function values, fn types"),
    ("constants", "top-level let and var, type aliases"),
    ("structs", "methods, field defaults, static calls"),
    ("pointers", "*T, *mut T, [*]T, p.*"),
    ("match", "patterns, captures, exhaustive arms"),
    ("9", "allocators, Arena, leaks, custom allocators"),
    ("12", "import, pub, multi-file modules"),
];

struct Args {
    cmd: String,
    file: String,
    out: Option<String>,
    mode: Mode,
    json: bool,
    filter: String,
    /// `aye test --module m`: only the tests of module `m`.
    module: Option<String>,
    /// `aye run ... -- args`: the program's arguments.
    prog_args: Vec<String>,
    /// `--cflags "..."`: more flags for clang.
    cflags: Vec<String>,
    /// `--no-link lib`: `@linkOptional` libraries to treat as absent.
    no_link: Vec<String>,
    /// `--leak-check`: the checks of `General` in any mode.
    leak_check: bool,
    /// `--target name`.
    target: Option<String>,
    /// `aye build -c`: an object file, not linked.
    compile_only: bool,
    /// `aye test --time-limit s`: CPU seconds per test; 0: no limit.
    time_limit: u64,
    /// `aye test -j n`: test processes at once.
    jobs: usize,
}

fn parse_args() -> Result<Args, String> {
    let mut it = std::env::args().skip(1);
    let cmd = it.next().ok_or("missing command")?;
    let (mut file, mut out, mut mode, mut json, mut filter, mut module) =
        (None, None, Mode::Debug, false, String::new(), None);
    let mut prog_args = Vec::new();
    let mut cflags = Vec::new();
    let mut no_link = Vec::new();
    let mut leak_check = false;
    let mut target = None;
    let mut compile_only = false;
    let mut time_limit = 10;
    let mut jobs = std::thread::available_parallelism().map_or(1, |n| n.get());
    while let Some(a) = it.next() {
        match a.as_str() {
            "--" if cmd == "run" => prog_args.extend(it.by_ref()),
            "-o" if cmd == "build" => out = Some(it.next().ok_or("`-o` needs a path")?),
            "-c" if cmd == "build" => compile_only = true,
            "--json" if cmd == "check" || cmd == "test" => json = true,
            "--filter" if cmd == "test" => filter = it.next().ok_or("`--filter` needs a text")?,
            "--time-limit" if cmd == "test" => {
                time_limit = it
                    .next()
                    .and_then(|n| n.parse().ok())
                    .ok_or("`--time-limit` needs a whole number of seconds (0: no limit)")?
            }
            "-j" if cmd == "test" => {
                jobs = it
                    .next()
                    .and_then(|n| n.parse().ok())
                    .filter(|n| *n > 0)
                    .ok_or("`-j` needs a number of test processes, such as 4")?
            }
            "--module" if cmd == "test" => {
                module = Some(
                    it.next()
                        .ok_or("`--module` needs a module path, such as `html`")?,
                )
            }
            "--cflags" if matches!(cmd.as_str(), "build" | "run" | "test") => {
                cflags.extend(
                    it.next()
                        .ok_or("`--cflags` needs flags, such as \"-fsanitize=thread\"")?
                        .split_whitespace()
                        .map(String::from),
                );
            }
            "--no-link" if matches!(cmd.as_str(), "build" | "run" | "test") => no_link.push(
                it.next()
                    .ok_or("`--no-link` needs a library name, such as libcurl")?,
            ),
            "--leak-check" if matches!(cmd.as_str(), "build" | "run" | "test") => leak_check = true,
            "--target" if matches!(cmd.as_str(), "build" | "run" | "test" | "emit-c" | "check") => {
                target = Some(it.next().ok_or("`--target` needs a target name")?)
            }
            "--mode" => {
                let m = it.next().ok_or("`--mode` needs a value")?;
                mode = Mode::parse(&m).ok_or(format!("unknown mode `{m}`"))?;
            }
            _ if file.is_none() && !a.starts_with('-') => file = Some(a),
            _ => return Err(format!("unexpected argument `{a}`")),
        }
    }
    let file = file.unwrap_or_else(|| ".".to_string());
    Ok(Args {
        cmd,
        file,
        out,
        mode,
        json,
        filter,
        module,
        prog_args,
        cflags,
        no_link,
        leak_check,
        target,
        compile_only,
        time_limit,
        jobs,
    })
}

/// Rust ignores SIGPIPE, so `println!` panics when stdout is closed
/// (`aye spec | head`). Restore the default: the process ends quietly, like
/// any Unix tool.
#[cfg(unix)]
fn restore_sigpipe() {
    unsafe extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    // SAFETY: plain libc call, at startup before any thread exists.
    unsafe { signal(SIGPIPE, SIG_DFL) };
}

/// The error line for a `path` that does not exist, or `None`. A path that
/// does not end in `.aye` is taken for a directory. `cmd` is what `aye`
/// should be run with again, e.g. `check`.
fn missing_path(cmd: &str, path: &str) -> Option<String> {
    if Path::new(path).exists() {
        return None;
    }
    let (what, todo) = if path.ends_with(".aye") {
        ("file", "write the file")
    } else {
        ("directory", "create it and write its .aye files")
    };
    Some(format!(
        "error: {path}: no such {what}; {todo}, then run aye {cmd} {path}"
    ))
}

fn main() -> ExitCode {
    #[cfg(unix)]
    restore_sigpipe();
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let is_help = |a: &String| matches!(a.as_str(), "--help" | "-h");
    // `aye help`, `aye --help`, `aye <cmd> --help` (before a `--`).
    let cmd_help = argv.iter().skip(1).take_while(|a| *a != "--").any(is_help);
    if argv.first().is_some_and(|a| is_help(a) || a == "help") || cmd_help {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    if argv
        .first()
        .is_some_and(|a| matches!(a.as_str(), "--version" | "-V" | "version"))
    {
        println!("{}", version_line());
        return ExitCode::SUCCESS;
    }
    if std::env::args().nth(1).as_deref() == Some("spec") {
        return spec_cmd(std::env::args().skip(2).collect());
    }
    match std::env::args().nth(1).as_deref() {
        Some("fmt") => return fmt_cmd(std::env::args().skip(2).collect()),
        Some("doc") => return doc_cmd(std::env::args().skip(2).collect()),
        Some("cimport") => return cimport_cmd(std::env::args().skip(2).collect()),
        Some("lsp") => return lsp_cmd(std::env::args().skip(2).collect()),
        _ => {}
    }
    if std::env::args().nth(1).as_deref() == Some("explain") {
        let rest: Vec<String> = std::env::args().skip(2).collect();
        let [code] = &rest[..] else {
            eprintln!("aye: `explain` needs one error code\n{USAGE}");
            return ExitCode::from(2);
        };
        return match codes::explain(code) {
            Some(text) => {
                print!("{text}");
                ExitCode::SUCCESS
            }
            None => {
                eprintln!("aye: unknown error code `{code}`");
                ExitCode::from(2)
            }
        };
    }
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("aye: {e}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    if !matches!(
        args.cmd.as_str(),
        "build" | "run" | "emit-c" | "check" | "test" | "fix"
    ) {
        eprintln!("aye: unknown command `{}`\n{USAGE}", args.cmd);
        return ExitCode::from(2);
    }
    let target = match args.target.as_deref().map(driver::find_target) {
        Some(Ok(t)) => t,
        Some(Err(e)) => {
            eprintln!("error[{}]: {e}", codes::E0117);
            return ExitCode::FAILURE;
        }
        None => driver::host_target(),
    };
    if let Some(e) = missing_path(&args.cmd, &args.file) {
        eprintln!("{e}");
        return ExitCode::FAILURE;
    }
    if args.cmd == "test" {
        return test(&args, target);
    }
    let mut root = match Root::load(&args.file) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("aye: {e}");
            return ExitCode::FAILURE;
        }
    };
    root.target = target;
    root.no_link = args.no_link.clone();
    root.leak_check = args.leak_check;
    let app_name = root.manifest.as_ref().and_then(|m| m.name());
    if args.cmd == "check" {
        let res = driver::front_end(root, None).result;
        if args.json {
            let diags = match &res {
                Ok(_) => String::new(),
                Err(ds) => ds.iter().map(|d| d.to_json()).collect::<Vec<_>>().join(","),
            };
            println!(
                r#"{{"file":{},"diagnostics":[{diags}]}}"#,
                diag::json_str(&args.file)
            );
        } else if let Err(ds) = &res {
            for d in ds {
                eprintln!("{}", d.render());
            }
        }
        return if res.is_ok() {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };
    }
    if args.cmd == "fix" {
        return fix(&args.file);
    }
    let mut c = match driver::compile_root(root, args.mode) {
        Ok(c) => c,
        Err(ds) => {
            for d in ds {
                eprintln!("{}", d.render());
            }
            return ExitCode::FAILURE;
        }
    };
    if args.cmd == "emit-c" {
        print!("{}", c.c);
        return ExitCode::SUCCESS;
    }
    let exe = match (&args.cmd[..], &args.out) {
        ("build", Some(o)) => PathBuf::from(o),
        ("build", None) => PathBuf::from(format!(
            "{}{}",
            app_name.unwrap_or_else(|| Path::new(&args.file)
                .file_stem()
                .unwrap_or("main".as_ref())
                .to_string_lossy()
                .into_owned()),
            if args.compile_only { ".o" } else { target.ext }
        )),
        _ => std::env::temp_dir().join(format!("aye-run-{}{}", std::process::id(), target.ext)),
    };
    c.cflags = args.cflags.clone();
    let built = if args.compile_only {
        driver::cc_obj(&c, &exe, args.mode)
    } else {
        driver::cc(&c, &exe, args.mode)
    };
    if let Err(e) = built {
        eprintln!("aye: {e}");
        return ExitCode::FAILURE;
    }
    if args.cmd == "build" {
        return ExitCode::SUCCESS;
    }
    let status = target.run_command(&exe).args(&args.prog_args).status();
    let _ = std::fs::remove_file(&exe);
    driver::remove_dsym(&exe);
    match status {
        Ok(s) => match s.code() {
            Some(code) => ExitCode::from(code as u8),
            None => {
                use std::os::unix::process::ExitStatusExt;
                ExitCode::from(128u8.wrapping_add(s.signal().unwrap_or(0) as u8))
            }
        },
        Err(e) if target.is_wasm() && e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!(
                "aye: running a wasm program needs `node` (22 or later), which was not found"
            );
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("aye: cannot run {}: {e}", exe.display());
            ExitCode::FAILURE
        }
    }
}

/// `aye lsp`: serve the Language Server Protocol on stdin and stdout.
fn lsp_cmd(args: Vec<String>) -> ExitCode {
    if let Some(a) = args.first() {
        eprintln!("aye: `lsp` takes no arguments, found `{a}`\n{USAGE}");
        return ExitCode::from(2);
    }
    let code = lsp::run(
        std::io::BufReader::new(std::io::stdin()),
        std::io::stdout().lock(),
    );
    ExitCode::from(code as u8)
}

/// `aye fmt [--check] <path>`: format a `.aye` file, or every `.aye` file
/// under a directory (hidden directories skipped), in place. With
/// `--check` nothing is written: the files that would change are listed
/// and the exit status is 1 if there is one. A file that does not lex is
/// reported and left alone (exit status 1).
fn fmt_cmd(args: Vec<String>) -> ExitCode {
    let check = args.iter().any(|a| a == "--check");
    let paths: Vec<&String> = args.iter().filter(|a| *a != "--check").collect();
    let [path] = &paths[..] else {
        eprintln!("aye: `fmt` needs one path\n{USAGE}");
        return ExitCode::from(2);
    };
    if path.starts_with('-') {
        eprintln!("aye: unexpected argument `{path}`\n{USAGE}");
        return ExitCode::from(2);
    }
    if let Some(e) = missing_path("fmt", path) {
        eprintln!("{e}");
        return ExitCode::FAILURE;
    }
    let mut files = Vec::new();
    if let Err(e) = aye_files(Path::new(path), &mut files) {
        eprintln!("aye: {e}");
        return ExitCode::FAILURE;
    }
    let mut failed = false;
    for f in files {
        let name = f.to_string_lossy().into_owned();
        let text = match std::fs::read_to_string(&f) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("aye: cannot read {name}: {e}");
                failed = true;
                continue;
            }
        };
        let out = match fmt::format(&text) {
            Ok(out) => out,
            Err(errs) => {
                for e in errs {
                    eprintln!("{}", e.with_file(std::slice::from_ref(&name)).render());
                }
                failed = true;
                continue;
            }
        };
        if out == text {
            continue;
        }
        if check {
            println!("{name}");
            failed = true;
        } else if let Err(e) = std::fs::write(&f, out) {
            eprintln!("aye: cannot write {name}: {e}");
            failed = true;
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// `path` if it is a file, else the `.aye` files under the directory
/// `path` (not in hidden directories), sorted.
fn aye_files(path: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    if !path.is_dir() {
        if !path.exists() {
            return Err(format!(
                "cannot read {}: no such file or directory",
                path.display()
            ));
        }
        out.push(path.to_path_buf());
        return Ok(());
    }
    let mut entries: Vec<PathBuf> = std::fs::read_dir(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    entries.sort();
    for p in entries {
        let hidden = p
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with('.'));
        if p.is_dir() && !hidden {
            aye_files(&p, out)?;
        } else if p.is_file() && p.extension().is_some_and(|e| e == "aye") {
            out.push(p);
        }
    }
    Ok(())
}

/// `aye doc --signatures <path>`: the pub declarations of the modules at
/// `path` (see `doc::signatures`). `aye doc --html <path> [-o <dir>]`: the
/// same as HTML pages in `dir` (see `doc::html`).
fn doc_cmd(args: Vec<String>) -> ExitCode {
    let html = args.first().is_some_and(|a| a == "--html");
    let (path, out_dir) = match &args[..] {
        [flag, path] if flag == "--signatures" || flag == "--html" => (path, None),
        [flag, path, o, dir] | [flag, o, dir, path]
            if flag == "--html" && o == "-o" && path != "-o" =>
        {
            (path, Some(dir))
        }
        _ if html => {
            eprintln!("aye: `doc --html` needs one path and optionally `-o <dir>`\n{USAGE}");
            return ExitCode::from(2);
        }
        [flag, ..] if flag != "--signatures" => {
            eprintln!("aye: unexpected argument `{flag}`\n{USAGE}");
            return ExitCode::from(2);
        }
        _ => {
            eprintln!("aye: `doc` needs `--signatures` or `--html` and one path\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    let is_std = path == "std" || path.starts_with("std.");
    let doc_cmd_text = if html {
        "doc --html"
    } else {
        "doc --signatures"
    };
    if let Some(e) = missing_path(doc_cmd_text, path).filter(|_| !is_std) {
        eprintln!("{e}");
        return ExitCode::FAILURE;
    }
    if html {
        let dir = std::path::Path::new(out_dir.map_or("doc", |d| d.as_str()));
        return match doc::html(path, dir) {
            Ok(n) => {
                println!("wrote {} ({n} modules)", dir.display());
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{e}");
                ExitCode::FAILURE
            }
        };
    }
    match doc::signatures(path) {
        Ok(text) => {
            print!("{text}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

/// A part of `docs/spec.md` that `aye spec <name>` prints.
struct SpecSection {
    /// `["4", "declarations"]` for `## 4. Declarations`, `["std.fs"]`.
    names: Vec<String>,
    /// The heading, or the `###` headings it holds: "C functions and
    /// types, C headers".
    title: String,
    text: &'static str,
}

/// `SPEC` cut at the headings that start a section (see `SPEC_SPLITS`).
/// The text before the first heading belongs to the first section, so the
/// sections, in order, are the whole document.
fn spec_sections() -> Vec<SpecSection> {
    let mut found: Vec<(usize, Vec<String>, String)> = Vec::new();
    let mut at = 0;
    let mut in_code = false;
    for line in SPEC.split_inclusive('\n') {
        let l = line.trim_end();
        let start = at;
        at += line.len();
        if l.starts_with("```") {
            in_code = !in_code;
        }
        if in_code {
            continue;
        }
        if let Some(h) = l.strip_prefix("## ") {
            let (num, title) = match h.split_once(". ") {
                Some((n, t)) if n.bytes().all(|b| b.is_ascii_digit()) => (Some(n), t),
                _ => (None, h),
            };
            // The first word of the title: "declarations", "first" for "A
            // first program", "pitfalls" for "Pitfalls (differences ...)".
            let word = title
                .split_whitespace()
                .find(|w| *w != "A")
                .unwrap_or(title);
            let word: String = word.chars().filter(|c| c.is_alphanumeric()).collect();
            let mut names: Vec<String> = num.map(String::from).into_iter().collect();
            names.push(word.to_lowercase());
            found.push((start, names, title.to_string()));
        } else if let Some(h) = l.strip_prefix("### ") {
            let split = SPEC_SPLITS
                .iter()
                .find(|(head, _)| *head == h)
                .map(|(_, n)| *n);
            if let Some(name) = split.or(h.starts_with("std.").then_some(h)) {
                found.push((start, vec![name.to_string()], h.to_string()));
            } else if let Some((_, names, title)) = found.last_mut()
                && !names[0].starts_with(|c: char| c.is_ascii_digit())
            {
                title.push_str(", ");
                title.push_str(h);
            }
        }
    }
    let ends: Vec<usize> = found
        .iter()
        .skip(1)
        .map(|f| f.0)
        .chain([SPEC.len()])
        .collect();
    let mut start = 0;
    let mut out = Vec::new();
    for ((_, names, title), end) in found.into_iter().zip(ends) {
        out.push(SpecSection {
            names,
            title,
            text: &SPEC[start..end],
        });
        start = end;
    }
    out
}

/// `aye spec`: the core sections (`SPEC_CORE`, in its order), then a list
/// of the others; `aye spec <name>`: one section, by number or name; `aye
/// spec all`: the whole document.
fn spec_cmd(args: Vec<String>) -> ExitCode {
    let sections = spec_sections();
    let core = |s: &SpecSection| s.names.iter().any(|n| SPEC_CORE.contains(&n.as_str()));
    match &args[..] {
        [] => {
            let text: String = SPEC_CORE
                .iter()
                .filter_map(|n| sections.iter().find(|s| s.names.iter().any(|m| m == n)))
                .map(|s| s.text)
                .collect();
            println!("{}\n", text.trim_end());
            println!("---");
            println!(
                "This is the core of the reference: sections {}.",
                SPEC_CORE.join(" ")
            );
            println!(
                "Other sections; `aye spec <name>` prints one, `aye spec all` the whole reference:"
            );
            for s in sections.iter().filter(|s| !core(s)) {
                let names = s.names.join(" ");
                let hint = SPEC_HINTS
                    .iter()
                    .find(|(n, _)| *n == s.names[0])
                    .map(|(_, h)| *h);
                match hint {
                    Some(h) => println!("  {names}: {h}"),
                    None if s.names.len() == 1 && s.title == s.names[0] => println!("  {names}"),
                    None => println!("  {names}: {}", s.title),
                }
            }
        }
        [all] if all == "all" => print!("{SPEC}"),
        [name] => match sections.iter().find(|s| s.names.contains(name)) {
            Some(s) => print!("{}", s.text),
            None => {
                eprintln!("aye: the reference has no section `{name}`; `aye spec` lists them");
                return ExitCode::from(2);
            }
        },
        _ => {
            eprintln!("aye: `spec` takes at most one section\n{USAGE}");
            return ExitCode::from(2);
        }
    }
    ExitCode::SUCCESS
}

/// `aye fix`: apply the fix-its that `aye check` reports, in place, and
/// check again, until no fix applies (at most 10 rounds). Each applied fix
/// is printed as `file:line:col: fixed: <help>`; then the errors that
/// remain are printed as `aye check` prints them. The exit status is 0 when
/// no error remains. Files of std and of dependencies are never written.
fn fix(path: &str) -> ExitCode {
    let check = || match Root::load(path) {
        Ok(root) => Ok(driver::front_end(root, None)),
        Err(e) => Err(e),
    };
    for _ in 0..10 {
        let checked = match check() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("aye: {e}");
                return ExitCode::FAILURE;
            }
        };
        let Err(ds) = &checked.result else { break };
        let mut any = false;
        for f in checked.files.iter().filter(|f| !f.std && !f.dep) {
            let mine: Vec<diag::Diag> = ds.iter().filter(|d| d.file == f.name).cloned().collect();
            let (fixed, applied) = driver::apply_fixes(&f.text, &mine);
            if applied.is_empty() {
                continue;
            }
            if let Err(e) = std::fs::write(&f.name, &fixed) {
                eprintln!("aye: cannot write {}: {e}", f.name);
                return ExitCode::FAILURE;
            }
            for (at, help) in applied {
                println!("{}:{}:{}: fixed: {help}", f.name, at.line, at.col);
            }
            any = true;
        }
        if !any {
            break;
        }
    }
    match check().map(|c| c.result) {
        Err(e) => {
            eprintln!("aye: {e}");
            ExitCode::FAILURE
        }
        Ok(Ok(_)) => ExitCode::SUCCESS,
        Ok(Err(ds)) => {
            for d in ds {
                eprintln!("{}", d.render());
            }
            ExitCode::FAILURE
        }
    }
}

/// `aye test`: one line per test (in test order, whatever `-j` is), and for a
/// failed test its output (stdout, then stderr) indented; or with `--json` one JSON document. The exit
/// status is 1 if a test failed or the file does not compile.
fn test(args: &Args, target: &'static driver::Target) -> ExitCode {
    let run = match driver::run_tests(
        &args.file,
        target,
        args.mode,
        &args.filter,
        args.module.as_deref(),
        &args.cflags,
        &args.no_link,
        args.leak_check,
        args.jobs,
        args.time_limit,
    ) {
        Ok(r) => r,
        Err(driver::TestError::Compile(ds)) => {
            if args.json {
                let diags: Vec<String> = ds.iter().map(|d| d.to_json()).collect();
                println!(
                    r#"{{"file":{},"diagnostics":[{}],"tests":[],"passed":0,"failed":0}}"#,
                    diag::json_str(&args.file),
                    diags.join(",")
                );
            } else {
                for d in ds {
                    eprintln!("{}", d.render());
                }
            }
            return ExitCode::FAILURE;
        }
        Err(driver::TestError::Run(e)) => {
            eprintln!("aye: {e}");
            return ExitCode::FAILURE;
        }
    };
    let results = &run.results;
    let failed = results.iter().filter(|r| !r.passed).count();
    let passed = results.len() - failed;
    if args.json {
        let tests: Vec<String> = results.iter().map(|r| r.to_json()).collect();
        println!(
            r#"{{"file":{},"diagnostics":[],"tests":[{}],"passed":{passed},"failed":{failed}}}"#,
            diag::json_str(&args.file),
            tests.join(",")
        );
    } else {
        for r in results {
            if let Some(driver::Failure::Compile(message)) = r.failures.first() {
                println!("test \"{}\" ... FAILED (compile)", r.name);
                eprintln!("{message}");
                continue;
            }
            match r.time_limit_note() {
                Some(note) => println!("test \"{}\" ... FAILED ({note})", r.name),
                None => println!(
                    "test \"{}\" ... {} ({} ms)",
                    r.name,
                    if r.passed { "ok" } else { "FAILED" },
                    r.ms
                ),
            }
            if !r.passed {
                for l in r.stdout.lines().chain(r.stderr.lines()) {
                    println!("    {l}");
                }
            }
        }
        println!(
            "\n{passed} passed; {failed} failed; {} modules; {} ms",
            run.modules, run.ms
        );
    }
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// `aye cimport HEADER [flags] [--link lib]`: print the Aye source that
/// `@cimport` generates for a header (bypassing the cache). The header and
/// relative `-I` paths are relative to the working directory.
fn cimport_cmd(args: Vec<String>) -> ExitCode {
    let mut header = None;
    let mut flags = Vec::new();
    let mut links = Vec::new();
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        if a == "--link" {
            match it.next() {
                Some(l) => links.push(l),
                None => {
                    eprintln!("aye: `--link` needs a library name\n{USAGE}");
                    return ExitCode::from(2);
                }
            }
        } else if a.starts_with('-') {
            if let Err(e) = aye::cimport::check_flag(&a) {
                eprintln!("aye: {e}");
                return ExitCode::from(2);
            }
            flags.push(a);
        } else if header.is_none() {
            header = Some(a);
        } else {
            eprintln!("aye: `cimport` takes one header\n{USAGE}");
            return ExitCode::from(2);
        }
    }
    let Some(header) = header else {
        eprintln!("aye: `cimport` needs a header, such as `zlib.h`\n{USAGE}");
        return ExitCode::from(2);
    };
    let req = aye::cimport::Request {
        header,
        flags,
        dir: PathBuf::from("."),
        links,
    };
    match aye::cimport::generate(&req) {
        Ok((source, _)) => {
            print!("{source}");
            ExitCode::SUCCESS
        }
        Err(f) => {
            let (code, msg) = match f {
                aye::cimport::Failure::NoLibclang(m) => (codes::E0114, m),
                aye::cimport::Failure::Header(m) => (codes::E0113, m),
            };
            eprintln!("error[{code}]: {msg}");
            ExitCode::FAILURE
        }
    }
}
