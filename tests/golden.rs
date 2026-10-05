//! Golden tests.
//!
//! - `tests/cases/X.ore`: compiled in debug, safe and fast mode, run, and
//!   stdout compared with `X.out`. If `X.panic` exists the program must abort
//!   with exactly that on stderr (checked in debug and safe mode only). If
//!   `X.stderr` exists, `main` must return an error: exit status 1, and
//!   stderr equal to `X.stderr` in debug mode; safe and fast mode print no
//!   error return trace, so there stderr must be the first line only.
//! - `tests/errors/X.ore`: must fail to compile with exactly the diagnostics
//!   in `X.err`, one per line, in order.
//! - `tests/tests/X.ore`: `ore test X.ore` (debug mode) must print exactly
//!   `X.out`, and `ore test --json X.ore` exactly `X.json` if it exists. The
//!   exit status must be 0 when no test failed, and 1 otherwise.
//! - `ore run`, `ore check` and `ore test --filter` are also run through the
//!   built binary.

use ore::diag::Diag;
use ore::driver::{self, Mode};
use std::path::{Path, PathBuf};
use std::process::Command;

fn read(p: &Path) -> Option<String> {
    std::fs::read_to_string(p).ok()
}

fn cases(dir: &str) -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join(dir);
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "ore"))
        .collect();
    v.sort();
    v
}

/// Diagnostics as `ore check` prints them, one per line.
fn join(diags: &[Diag]) -> String {
    diags.iter().map(|d| d.to_string()).collect::<Vec<_>>().join("\n")
}

fn name(p: &Path) -> String {
    p.file_name().unwrap().to_string_lossy().into_owned()
}

fn run_case(path: &Path, work: &Path) -> Result<(), String> {
    let src = read(path).unwrap();
    let expected_out = read(&path.with_extension("out")).ok_or("missing .out file")?;
    let expected_panic = read(&path.with_extension("panic"));
    let expected_error = read(&path.with_extension("stderr"));
    let modes: &[Mode] = if expected_panic.is_some() {
        &[Mode::Debug, Mode::Safe]
    } else {
        &[Mode::Debug, Mode::Safe, Mode::Fast]
    };
    for &mode in modes {
        let c = driver::compile_to_c(&name(path), &src, mode).map_err(|d| join(&d))?;
        let exe = work.join(format!("{}-{mode:?}", path.file_stem().unwrap().to_string_lossy()));
        driver::cc(&c, &exe, mode)?;
        let out = Command::new(&exe).output().map_err(|e| e.to_string())?;
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        if stdout != expected_out {
            return Err(format!("[{mode:?}] stdout mismatch\n--- expected\n{expected_out}--- got\n{stdout}"));
        }
        if let Some(full) = &expected_error {
            let want = if mode == Mode::Debug { full.clone() } else { format!("{}\n", full.lines().next().unwrap_or("")) };
            if out.status.code() != Some(1) {
                return Err(format!("[{mode:?}] expected exit status 1, got {}", out.status));
            }
            if stderr != want {
                return Err(format!("[{mode:?}] stderr mismatch\n--- expected\n{want}--- got\n{stderr}"));
            }
            continue;
        }
        match &expected_panic {
            Some(p) => {
                if out.status.success() {
                    return Err(format!("[{mode:?}] expected a panic, but the program exited normally"));
                }
                if stderr != *p {
                    return Err(format!("[{mode:?}] stderr mismatch\n--- expected\n{p}--- got\n{stderr}"));
                }
            }
            None => {
                if !out.status.success() || !stderr.is_empty() {
                    return Err(format!("[{mode:?}] program failed ({}):\n{stderr}", out.status));
                }
            }
        }
    }
    Ok(())
}

fn error_case(path: &Path) -> Result<(), String> {
    let src = read(path).unwrap();
    let expected = read(&path.with_extension("err")).ok_or("missing .err file")?;
    match driver::compile_to_c(&name(path), &src, Mode::Debug) {
        Ok(_) => Err("compiled successfully, expected an error".into()),
        Err(d) if join(&d) == expected.trim_end() => Ok(()),
        Err(d) => Err(format!("error mismatch\n--- expected\n{}\n--- got\n{}", expected.trim_end(), join(&d))),
    }
}

/// `ore test` on a file of `tests/tests`, run from the crate root so that
/// positions in the output start with `tests/tests/`.
fn ore_test(args: &[&str]) -> Result<(String, Option<i32>), String> {
    let out = Command::new(env!("CARGO_BIN_EXE_ore"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .arg("test")
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    let stderr = String::from_utf8_lossy(&out.stderr);
    if !stderr.is_empty() {
        return Err(format!("`ore test {}` wrote to stderr:\n{stderr}", args.join(" ")));
    }
    Ok((String::from_utf8_lossy(&out.stdout).into_owned(), out.status.code()))
}

fn test_case(path: &Path) -> Result<(), String> {
    let rel = format!("tests/tests/{}", name(path));
    let expected = read(&path.with_extension("out")).ok_or("missing .out file")?;
    let status = if expected.ends_with("; 0 failed\n") { 0 } else { 1 };
    let mut runs = vec![(vec![rel.as_str()], expected)];
    if let Some(json) = read(&path.with_extension("json")) {
        runs.push((vec!["--json", rel.as_str()], json));
    }
    for (args, want) in runs {
        let (got, code) = ore_test(&args)?;
        if got != want {
            return Err(format!("`ore test {}`: stdout mismatch\n--- expected\n{want}--- got\n{got}", args.join(" ")));
        }
        if code != Some(status) {
            return Err(format!("`ore test {}`: expected exit status {status}, got {code:?}", args.join(" ")));
        }
    }
    Ok(())
}

/// `ore test --filter`: only the tests whose name contains the text run.
fn cli_test_filter() -> Result<(), String> {
    let f = "tests/tests/outcomes.ore";
    let want = format!(
        "test \"expect fails\" ... FAILED\n    before\n    {f}:28:5: assertion failed: condition is false\n\
         test \"expectEqual fails\" ... FAILED\n    {f}:33:5: assertion failed: expected 4, found 5\n\n0 passed; 2 failed\n"
    );
    let (got, code) = ore_test(&[f, "--filter", "expect"])?;
    if got != want || code != Some(1) {
        return Err(format!("`ore test --filter expect`: {code:?}\n--- expected\n{want}--- got\n{got}"));
    }
    let (got, code) = ore_test(&[f, "--filter", "no such test"])?;
    if got != "\n0 passed; 0 failed\n" || code != Some(0) {
        return Err(format!("`ore test --filter` with no match: {code:?}\n{got}"));
    }
    Ok(())
}

/// The `ore run` CLI end to end.
fn cli_run(work: &Path) -> Result<(), String> {
    let _ = work;
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/cases/hello.ore");
    let out = Command::new(env!("CARGO_BIN_EXE_ore")).arg("run").arg(&file).output().map_err(|e| e.to_string())?;
    let expected = read(&file.with_extension("out")).unwrap();
    if !out.status.success() || String::from_utf8_lossy(&out.stdout) != expected {
        return Err(format!("`ore run` failed: {}", String::from_utf8_lossy(&out.stderr)));
    }
    Ok(())
}

/// The `ore check` CLI: exit code, stdout and stderr, text and `--json`.
fn cli_check() -> Result<(), String> {
    let ok = "tests/cases/hello.ore";
    let bad = "tests/errors/unknown_escape.ore";
    let three = "tests/errors/three_errors.ore";
    let diag = |code: &str, line: u32, col: u32, msg: &str| {
        format!(r#"{{"severity":"error","code":"{code}","line":{line},"col":{col},"message":"{msg}"}}"#)
    };
    let checks: &[(&[&str], bool, String, String)] = &[
        (&[ok], true, String::new(), String::new()),
        (&[bad], false, String::new(), format!("{bad}:2:16: error[E0005]: unknown escape sequence `\\q`\n")),
        (&[ok, "--json"], true, format!("{{\"file\":\"{ok}\",\"diagnostics\":[]}}\n"), String::new()),
        (
            &[bad, "--json"],
            false,
            format!(
                "{{\"file\":\"{bad}\",\"diagnostics\":[{{\"severity\":\"error\",\"code\":\"E0005\",\"line\":2,\"col\":16,\
                 \"message\":\"unknown escape sequence `\\\\q`\"}}]}}\n"
            ),
            String::new(),
        ),
        (
            &[three, "--json"],
            false,
            format!(
                "{{\"file\":\"{three}\",\"diagnostics\":[{},{},{}]}}\n",
                diag("E0023", 4, 12, "mismatched types: expected `i64`, found `i32`"),
                diag("E0038", 9, 16, "struct `P` has no field `y`"),
                diag("E0018", 11, 14, "undefined variable `missing`"),
            ),
            String::new(),
        ),
    ];
    for (args, success, stdout, stderr) in checks {
        let out = Command::new(env!("CARGO_BIN_EXE_ore"))
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .arg("check")
            .args(*args)
            .output()
            .map_err(|e| e.to_string())?;
        let (got_out, got_err) = (String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
        if out.status.success() != *success || got_out != *stdout || got_err != *stderr {
            return Err(format!(
                "`ore check {}`: {}\n--- stdout\n{got_out}--- stderr\n{got_err}",
                args.join(" "),
                out.status
            ));
        }
    }
    Ok(())
}

/// `ore explain <code>`: known code (exit 0), unknown code (exit 2).
fn cli_explain() -> Result<(), String> {
    let run = |code: &str| Command::new(env!("CARGO_BIN_EXE_ore")).args(["explain", code]).output();
    let out = run("E0056").map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&out.stdout);
    if !out.status.success() || !text.starts_with("E0056: ") || !text.contains("Wrong:") || !text.contains("Fix:") {
        return Err(format!("`ore explain E0056`: {}\n{text}", out.status));
    }
    let out = run("E9999").map_err(|e| e.to_string())?;
    let err = String::from_utf8_lossy(&out.stderr);
    if out.status.code() != Some(2) || !out.stdout.is_empty() || err != "ore: unknown error code `E9999`\n" {
        return Err(format!("`ore explain E9999`: {}\n{err}", out.status));
    }
    Ok(())
}

fn main() {
    let work = std::env::temp_dir().join(format!("ore-golden-{}", std::process::id()));
    std::fs::create_dir_all(&work).unwrap();

    let run_cases = cases("cases");
    let err_cases = cases("errors");
    let test_cases = cases("tests");
    let results: Vec<(String, Result<(), String>)> = std::thread::scope(|s| {
        let mut handles = Vec::new();
        for p in &run_cases {
            let w = &work;
            handles.push((format!("cases/{}", name(p)), s.spawn(move || run_case(p, w))));
        }
        for p in &err_cases {
            handles.push((format!("errors/{}", name(p)), s.spawn(move || error_case(p))));
        }
        for p in &test_cases {
            handles.push((format!("tests/{}", name(p)), s.spawn(move || test_case(p))));
        }
        let w = &work;
        handles.push(("cli/ore run".to_string(), s.spawn(move || cli_run(w))));
        handles.push(("cli/ore check".to_string(), s.spawn(cli_check)));
        handles.push(("cli/ore explain".to_string(), s.spawn(cli_explain)));
        handles.push(("cli/ore test --filter".to_string(), s.spawn(cli_test_filter)));
        handles.into_iter().map(|(n, h)| (n, h.join().unwrap())).collect()
    });
    let _ = std::fs::remove_dir_all(&work);

    let mut failed = 0;
    for (n, r) in &results {
        match r {
            Ok(()) => println!("test {n} ... ok"),
            Err(e) => {
                failed += 1;
                println!("test {n} ... FAILED\n{e}");
            }
        }
    }
    println!("\ngolden: {} passed; {failed} failed", results.len() - failed);
    if failed > 0 {
        std::process::exit(1);
    }
}
