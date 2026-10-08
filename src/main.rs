use aye::driver::{self, Mode, Root};
use aye::{codes, diag, doc, fmt};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const USAGE: &str = "usage (<path> is a .aye file, or a directory: a module of all its .aye files):
  aye build <path> [-o out] [--mode debug|safe|fast] [--cflags <flags>]
  aye run <path> [--mode debug|safe|fast] [--cflags <flags>] [-- args...]
  aye emit-c <path> [--mode debug|safe|fast]
  aye check <path> [--json]
  aye fix <path>
  aye test <path> [--filter <text>] [--module <module>] [--json] [--mode debug|safe|fast] [--cflags <flags>]
  (--cflags: more flags for clang, such as \"-fsanitize=thread\")
  aye fmt [--check] <path>      (<path>: a .aye file, or a directory searched recursively)
  aye doc --signatures <path>   (<path> may also be `std` or `std.<module>`)
  aye cimport <header> [-Idir] [-DNAME[=value]] [--link <lib>]   (print the Aye declarations `@cimport` makes of a C header)
  aye explain <code>
  aye spec";

/// The language reference that `aye spec` prints.
const SPEC: &str = include_str!("../docs/spec.md");

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
}

fn parse_args() -> Result<Args, String> {
    let mut it = std::env::args().skip(1);
    let cmd = it.next().ok_or("missing command")?;
    let (mut file, mut out, mut mode, mut json, mut filter, mut module) =
        (None, None, Mode::Debug, false, String::new(), None);
    let mut prog_args = Vec::new();
    let mut cflags = Vec::new();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--" if cmd == "run" => prog_args.extend(it.by_ref()),
            "-o" if cmd == "build" => out = Some(it.next().ok_or("`-o` needs a path")?),
            "--json" if cmd == "check" || cmd == "test" => json = true,
            "--filter" if cmd == "test" => filter = it.next().ok_or("`--filter` needs a text")?,
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
            "--mode" => {
                let m = it.next().ok_or("`--mode` needs a value")?;
                mode = Mode::parse(&m).ok_or(format!("unknown mode `{m}`"))?;
            }
            _ if file.is_none() && !a.starts_with('-') => file = Some(a),
            _ => return Err(format!("unexpected argument `{a}`")),
        }
    }
    let file = file.ok_or("missing input file or directory")?;
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
    if std::env::args().nth(1).as_deref() == Some("spec") {
        if std::env::args().count() != 2 {
            eprintln!("aye: `spec` takes no arguments\n{USAGE}");
            return ExitCode::from(2);
        }
        print!("{SPEC}");
        return ExitCode::SUCCESS;
    }
    match std::env::args().nth(1).as_deref() {
        Some("fmt") => return fmt_cmd(std::env::args().skip(2).collect()),
        Some("doc") => return doc_cmd(std::env::args().skip(2).collect()),
        Some("cimport") => return cimport_cmd(std::env::args().skip(2).collect()),
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
    let root = match Root::load(&args.file) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("aye: {e}");
            return ExitCode::FAILURE;
        }
    };
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
    if args.cmd == "test" {
        return test(&args, root);
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
        ("build", None) => {
            PathBuf::from(Path::new(&args.file).file_stem().unwrap_or("main".as_ref()))
        }
        _ => std::env::temp_dir().join(format!("aye-run-{}", std::process::id())),
    };
    c.cflags = args.cflags.clone();
    if let Err(e) = driver::cc(&c, &exe, args.mode) {
        eprintln!("aye: {e}");
        return ExitCode::FAILURE;
    }
    if args.cmd == "build" {
        return ExitCode::SUCCESS;
    }
    let status = Command::new(&exe).args(&args.prog_args).status();
    let _ = std::fs::remove_file(&exe);
    match status {
        Ok(s) => match s.code() {
            Some(code) => ExitCode::from(code as u8),
            None => {
                use std::os::unix::process::ExitStatusExt;
                ExitCode::from(128u8.wrapping_add(s.signal().unwrap_or(0) as u8))
            }
        },
        Err(e) => {
            eprintln!("aye: cannot run {}: {e}", exe.display());
            ExitCode::FAILURE
        }
    }
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
/// `path` (see `doc::signatures`).
fn doc_cmd(args: Vec<String>) -> ExitCode {
    let [flag, path] = &args[..] else {
        eprintln!("aye: `doc` needs `--signatures` and one path\n{USAGE}");
        return ExitCode::from(2);
    };
    if flag != "--signatures" {
        eprintln!("aye: unexpected argument `{flag}`\n{USAGE}");
        return ExitCode::from(2);
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

/// `aye fix`: apply the fix-its that `aye check` reports, in place, and
/// check again, until no fix applies (at most 10 rounds). Each applied fix
/// is printed as `file:line:col: fixed: <help>`; then the errors that
/// remain are printed as `aye check` prints them. The exit status is 0 when
/// no error remains. Files of std are never written.
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
        for f in checked.files.iter().filter(|f| !f.std) {
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

/// `aye test`: one line per test, and for a failed test its output (stdout,
/// then stderr) indented; or with `--json` one JSON document. The exit
/// status is 1 if a test failed or the file does not compile.
fn test(args: &Args, root: Root) -> ExitCode {
    let results = match driver::run_tests(
        root,
        args.mode,
        &args.filter,
        args.module.as_deref(),
        &args.cflags,
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
        for r in &results {
            println!(
                "test \"{}\" ... {}",
                r.name,
                if r.passed { "ok" } else { "FAILED" }
            );
            if !r.passed {
                for l in r.stdout.lines().chain(r.stderr.lines()) {
                    println!("    {l}");
                }
            }
        }
        println!("\n{passed} passed; {failed} failed");
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
