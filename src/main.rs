use ore::{codes, diag};
use ore::driver::{self, Mode};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const USAGE: &str = "usage:
  ore build <file.ore> [-o out] [--mode debug|safe|fast]
  ore run <file.ore> [--mode debug|safe|fast]
  ore emit-c <file.ore> [--mode debug|safe|fast]
  ore check <file.ore> [--json]
  ore fix <file.ore>
  ore test <file.ore> [--filter <text>] [--json] [--mode debug|safe|fast]
  ore explain <code>";

struct Args {
    cmd: String,
    file: String,
    out: Option<String>,
    mode: Mode,
    json: bool,
    filter: String,
}

fn parse_args() -> Result<Args, String> {
    let mut it = std::env::args().skip(1);
    let cmd = it.next().ok_or("missing command")?;
    let (mut file, mut out, mut mode, mut json, mut filter) = (None, None, Mode::Debug, false, String::new());
    while let Some(a) = it.next() {
        match a.as_str() {
            "-o" if cmd == "build" => out = Some(it.next().ok_or("`-o` needs a path")?),
            "--json" if cmd == "check" || cmd == "test" => json = true,
            "--filter" if cmd == "test" => filter = it.next().ok_or("`--filter` needs a text")?,
            "--mode" => {
                let m = it.next().ok_or("`--mode` needs a value")?;
                mode = Mode::parse(&m).ok_or(format!("unknown mode `{m}`"))?;
            }
            _ if file.is_none() && !a.starts_with('-') => file = Some(a),
            _ => return Err(format!("unexpected argument `{a}`")),
        }
    }
    let file = file.ok_or("missing input file")?;
    Ok(Args { cmd, file, out, mode, json, filter })
}

fn main() -> ExitCode {
    if std::env::args().nth(1).as_deref() == Some("explain") {
        let rest: Vec<String> = std::env::args().skip(2).collect();
        let [code] = &rest[..] else {
            eprintln!("ore: `explain` needs one error code\n{USAGE}");
            return ExitCode::from(2);
        };
        return match codes::explain(code) {
            Some(text) => {
                print!("{text}");
                ExitCode::SUCCESS
            }
            None => {
                eprintln!("ore: unknown error code `{code}`");
                ExitCode::from(2)
            }
        };
    }
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("ore: {e}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    if !matches!(args.cmd.as_str(), "build" | "run" | "emit-c" | "check" | "test" | "fix") {
        eprintln!("ore: unknown command `{}`\n{USAGE}", args.cmd);
        return ExitCode::from(2);
    }
    let src = match std::fs::read_to_string(&args.file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("ore: cannot read {}: {e}", args.file);
            return ExitCode::FAILURE;
        }
    };
    if args.cmd == "check" {
        let res = driver::check(&args.file, &src);
        if args.json {
            let diags = match &res {
                Ok(_) => String::new(),
                Err(ds) => ds.iter().map(|d| d.to_json()).collect::<Vec<_>>().join(","),
            };
            println!(r#"{{"file":{},"diagnostics":[{diags}]}}"#, diag::json_str(&args.file));
        } else if let Err(ds) = &res {
            for d in ds {
                eprintln!("{}", d.render());
            }
        }
        return if res.is_ok() { ExitCode::SUCCESS } else { ExitCode::FAILURE };
    }
    if args.cmd == "fix" {
        return fix(&args.file, src);
    }
    if args.cmd == "test" {
        return test(&args, &src);
    }
    let c = match driver::compile_to_c(&args.file, &src, args.mode) {
        Ok(c) => c,
        Err(ds) => {
            for d in ds {
                eprintln!("{}", d.render());
            }
            return ExitCode::FAILURE;
        }
    };
    if args.cmd == "emit-c" {
        print!("{c}");
        return ExitCode::SUCCESS;
    }
    let exe = match (&args.cmd[..], &args.out) {
        ("build", Some(o)) => PathBuf::from(o),
        ("build", None) => PathBuf::from(Path::new(&args.file).file_stem().unwrap()),
        _ => std::env::temp_dir().join(format!("ore-run-{}", std::process::id())),
    };
    if let Err(e) = driver::cc(&c, &exe, args.mode) {
        eprintln!("ore: {e}");
        return ExitCode::FAILURE;
    }
    if args.cmd == "build" {
        return ExitCode::SUCCESS;
    }
    let status = Command::new(&exe).status();
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
            eprintln!("ore: cannot run {}: {e}", exe.display());
            ExitCode::FAILURE
        }
    }
}

/// `ore fix`: apply the fix-its that `ore check` reports, in place, and
/// check again, until no fix applies (at most 10 rounds). Each applied fix
/// is printed as `file:line:col: fixed: <help>`; then the errors that
/// remain are printed as `ore check` prints them. The exit status is 0 when
/// no error remains.
fn fix(file: &str, mut src: String) -> ExitCode {
    for _ in 0..10 {
        let Err(ds) = driver::check(file, &src) else { break };
        let (fixed, applied) = driver::apply_fixes(&src, &ds);
        if applied.is_empty() {
            break;
        }
        if let Err(e) = std::fs::write(file, &fixed) {
            eprintln!("ore: cannot write {file}: {e}");
            return ExitCode::FAILURE;
        }
        for (at, help) in applied {
            println!("{file}:{}:{}: fixed: {help}", at.line, at.col);
        }
        src = fixed;
    }
    match driver::check(file, &src) {
        Ok(_) => ExitCode::SUCCESS,
        Err(ds) => {
            for d in ds {
                eprintln!("{}", d.render());
            }
            ExitCode::FAILURE
        }
    }
}

/// `ore test`: one line per test, and for a failed test its output (stdout,
/// then stderr) indented; or with `--json` one JSON document. The exit
/// status is 1 if a test failed or the file does not compile.
fn test(args: &Args, src: &str) -> ExitCode {
    let results = match driver::run_tests(&args.file, src, args.mode, &args.filter) {
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
            eprintln!("ore: {e}");
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
            println!("test \"{}\" ... {}", r.name, if r.passed { "ok" } else { "FAILED" });
            if !r.passed {
                for l in r.stdout.lines().chain(r.stderr.lines()) {
                    println!("    {l}");
                }
            }
        }
        println!("\n{passed} passed; {failed} failed");
    }
    if failed == 0 { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}
