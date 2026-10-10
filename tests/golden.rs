//! Golden tests.
//!
//! - `tests/cases/X.aye`: compiled in debug, safe and fast mode, run, and
//!   stdout compared with `X.out`. If `X.panic` exists the program must abort
//!   with exactly that on stderr (checked in debug and safe mode only). If
//!   `X.stderr` exists, `main` must return an error: exit status 1, and
//!   stderr equal to `X.stderr` in debug mode; safe and fast mode print no
//!   error return trace, so there stderr must be the first line only.
//!   A case with `X.needs` (`pkg-config` packages) or `X.tools` (programs on
//!   `PATH`) is skipped when one is missing.
//! - `tests/ffi/X.aye`: like `tests/cases`, for programs that link C
//!   libraries with `@link`; skipped when `pkg-config` does not know a
//!   package listed in `X.needs`. Programs run with `SDL_VIDEODRIVER=dummy`,
//!   so SDL2 needs no display.
//! - `tests/cimport/X.aye`: programs with `@cimport`, run with `aye run`
//!   from the crate root with a cache directory of their own: stdout must
//!   equal `X.out`, and then `aye check` must pass without libclang (the
//!   cache). If `X.err` exists, `aye check` must fail with exactly that on
//!   stderr instead, with the variables of `X.env` (`NAME=value` lines) set.
//!   `aye cimport tests/cimport/X.h` must print exactly `X.gen` where that
//!   exists. Skipped without libclang, or when `pkg-config` does not know a
//!   package of `X.needs`.
//! - `tests/errors/X.aye`: must fail to compile with exactly the diagnostics
//!   in `X.err`, one per line, in order.
//! - `tests/tests/X.aye`: `aye test X.aye` (debug mode) must print exactly
//!   `X.out`, and `aye test --json X.aye` exactly `X.json` if it exists. The
//!   exit status must be 0 when no test failed, and 1 otherwise.
//! - `tests/fix/X.aye`: `aye fix X.aye` on a copy must leave exactly
//!   `X.fixed` and print exactly `X.out`. If `X.stderr` exists, errors
//!   remain: stderr must equal it and the exit status must be 1; otherwise
//!   stderr must be empty, the exit status 0, and `aye check` must accept
//!   the fixed file. If `X.json` exists, `aye check --json` on the original
//!   must print exactly that.
//! - `tests/modules/X/`: a directory module. In `tests/modules`, the binary
//!   runs `aye run X` (or the arguments in `X.args`, with `X.stdin` as
//!   stdin): stdout must equal `X.out`, stderr `X.stderr` (default empty)
//!   and the exit status `X.status` (default 0). If `X.err` exists, `aye
//!   check X` must fail with exactly that on stderr instead.
//! - `tests/example1/`: Example 1 of the design doc, the directory module
//!   `app`, run in `ok`, `missing` and `invalid` (each with its own
//!   `app.toml`, or none); `aye test app` runs its tests.
//! - `aye run`, `aye build -o`, `aye check`, `aye test --filter` and `aye
//!   test --module` are also run through the built binary.
//! - `perf`: the debug build of `tests/cases/general_churn.aye` finishes
//!   within 10 s (`General` allocates and frees in constant time).
//! - `aye build --cflags -fsanitize=thread`: the M10 fan-out program runs
//!   under ThreadSanitizer without a report (skipped where ThreadSanitizer
//!   does not run).

use aye::diag::Diag;
use aye::driver::{self, Mode};
use std::path::{Path, PathBuf};
use std::process::Command;

fn read(p: &Path) -> Option<String> {
    std::fs::read_to_string(p).ok()
}

/// The programs in `X.tools` that are not on `PATH`.
fn missing_tools(path: &Path) -> Vec<String> {
    let tools = read(&path.with_extension("tools")).unwrap_or_default();
    let path_var = std::env::var_os("PATH").unwrap_or_default();
    tools
        .split_whitespace()
        .filter(|t| !std::env::split_paths(&path_var).any(|d| d.join(t).is_file()))
        .map(String::from)
        .collect()
}

/// The packages in `X.needs` that `pkg-config` does not know.
fn missing_packages(path: &Path) -> Vec<String> {
    let needs = read(&path.with_extension("needs")).unwrap_or_default();
    let known = |p: &str| {
        Command::new("pkg-config")
            .args(["--exists", p])
            .status()
            .is_ok_and(|s| s.success())
    };
    needs
        .split_whitespace()
        .filter(|p| !known(p))
        .map(String::from)
        .collect()
}

fn cases(dir: &str) -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(dir);
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "aye"))
        .collect();
    v.sort();
    v
}

/// Diagnostics as `aye check` prints them, one per line.
fn join(diags: &[Diag]) -> String {
    diags
        .iter()
        .map(|d| d.to_string())
        .collect::<Vec<_>>()
        .join("\n")
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
        let exe = work.join(format!(
            "{}-{mode:?}",
            path.file_stem().unwrap().to_string_lossy()
        ));
        driver::cc(&c, &exe, mode)?;
        let out = Command::new(&exe)
            .env("SDL_VIDEODRIVER", "dummy")
            .output()
            .map_err(|e| e.to_string())?;
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        if stdout != expected_out {
            return Err(format!(
                "[{mode:?}] stdout mismatch\n--- expected\n{expected_out}--- got\n{stdout}"
            ));
        }
        if let Some(full) = &expected_error {
            let want = if mode == Mode::Debug {
                full.clone()
            } else {
                format!("{}\n", full.lines().next().unwrap_or(""))
            };
            if out.status.code() != Some(1) {
                return Err(format!(
                    "[{mode:?}] expected exit status 1, got {}",
                    out.status
                ));
            }
            if stderr != want {
                return Err(format!(
                    "[{mode:?}] stderr mismatch\n--- expected\n{want}--- got\n{stderr}"
                ));
            }
            continue;
        }
        match &expected_panic {
            Some(p) => {
                if out.status.success() {
                    return Err(format!(
                        "[{mode:?}] expected a panic, but the program exited normally"
                    ));
                }
                if stderr != *p {
                    return Err(format!(
                        "[{mode:?}] stderr mismatch\n--- expected\n{p}--- got\n{stderr}"
                    ));
                }
            }
            None => {
                if !out.status.success() || !stderr.is_empty() {
                    return Err(format!(
                        "[{mode:?}] program failed ({}):\n{stderr}",
                        out.status
                    ));
                }
            }
        }
    }
    Ok(())
}

/// A `tests/cases` program that needs C libraries or programs: skipped
/// when `pkg-config` does not know a package of `X.needs`, or a program of
/// `X.tools` is not on `PATH`.
fn tool_case(path: &Path, work: &Path) -> Result<(), String> {
    let mut missing = missing_packages(path);
    missing.extend(missing_tools(path));
    if !missing.is_empty() {
        println!(
            "skipped cases/{}: missing {}",
            name(path),
            missing.join(", ")
        );
        return Ok(());
    }
    run_case(path, work)
}

fn ffi_case(path: &Path, work: &Path) -> Result<(), String> {
    let missing = missing_packages(path);
    if !missing.is_empty() {
        println!(
            "skipped ffi/{}: pkg-config does not know {}",
            name(path),
            missing.join(", ")
        );
        return Ok(());
    }
    run_case(path, work)
}

/// Run the `aye` binary in the crate root with the environment `env`.
fn aye_env(
    args: &[&str],
    env: &[(String, String)],
) -> Result<(String, String, Option<i32>), String> {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_aye"));
    cmd.current_dir(env!("CARGO_MANIFEST_DIR")).args(args);
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.output().map_err(|e| e.to_string())?;
    Ok((
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    ))
}

fn cimport_case(path: &Path, work: &Path) -> Result<(), String> {
    let x = name(path);
    let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
    let rel = format!("tests/cimport/{x}");
    let mut env: Vec<(String, String)> = read(&path.with_extension("env"))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| {
            l.split_once('=')
                .map(|(k, v)| (k.to_string(), v.to_string()))
        })
        .collect();
    let without_libclang = env.iter().any(|(k, _)| k == "LIBCLANG_PATH");
    if !without_libclang && !aye::cimport::libclang_available() {
        println!("skipped cimport/{x}: no libclang");
        return Ok(());
    }
    let missing = missing_packages(path);
    if !missing.is_empty() {
        println!(
            "skipped cimport/{x}: pkg-config does not know {}",
            missing.join(", ")
        );
        return Ok(());
    }
    let cache = work.join(format!("cimport-cache-{stem}"));
    env.push((
        "AYE_CACHE_DIR".to_string(),
        cache.to_string_lossy().into_owned(),
    ));
    if let Some(want) = read(&path.with_extension("err")) {
        let (_, err, code) = aye_env(&["check", &rel], &env)?;
        if code != Some(1) || err != want {
            return Err(format!(
                "`aye check {rel}`: status {code:?}\n--- expected\n{want}--- got\n{err}"
            ));
        }
    } else {
        let want = read(&path.with_extension("out")).ok_or("missing .out file")?;
        let (out, err, code) = aye_env(&["run", &rel], &env)?;
        if code != Some(0) || out != want {
            return Err(format!(
                "`aye run {rel}`: status {code:?}, stderr:\n{err}\n--- expected\n{want}--- got\n{out}"
            ));
        }
        // From the cache, without libclang.
        env.push((
            "LIBCLANG_PATH".to_string(),
            "/nonexistent/libclang".to_string(),
        ));
        let (_, err, code) = aye_env(&["check", &rel], &env)?;
        if code != Some(0) {
            return Err(format!(
                "`aye check {rel}` without libclang (from the cache): status {code:?}\n{err}"
            ));
        }
    }
    let header = format!("tests/cimport/{stem}.h");
    if let Some(want) = read(&path.with_extension("gen")) {
        let (out, err, code) = aye_env(&["cimport", &header], &[])?;
        if code != Some(0) || out != want {
            return Err(format!(
                "`aye cimport {header}`: status {code:?}, stderr:\n{err}\n--- expected\n{want}--- got\n{out}"
            ));
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
        Err(d) => Err(format!(
            "error mismatch\n--- expected\n{}\n--- got\n{}",
            expected.trim_end(),
            join(&d)
        )),
    }
}

/// `aye test` on a file of `tests/tests`, run from the crate root so that
/// positions in the output start with `tests/tests/`.
fn aye_test(args: &[&str]) -> Result<(String, Option<i32>), String> {
    let out = Command::new(env!("CARGO_BIN_EXE_aye"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .arg("test")
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    let stderr = String::from_utf8_lossy(&out.stderr);
    if !stderr.is_empty() {
        return Err(format!(
            "`aye test {}` wrote to stderr:\n{stderr}",
            args.join(" ")
        ));
    }
    Ok((
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code(),
    ))
}

fn test_case(path: &Path) -> Result<(), String> {
    let rel = format!("tests/tests/{}", name(path));
    let expected = read(&path.with_extension("out")).ok_or("missing .out file")?;
    let status = if expected.ends_with("; 0 failed\n") {
        0
    } else {
        1
    };
    let mut runs = vec![(vec![rel.as_str()], expected)];
    if let Some(json) = read(&path.with_extension("json")) {
        runs.push((vec!["--json", rel.as_str()], json));
    }
    for (args, want) in runs {
        let (got, code) = aye_test(&args)?;
        if got != want {
            return Err(format!(
                "`aye test {}`: stdout mismatch\n--- expected\n{want}--- got\n{got}",
                args.join(" ")
            ));
        }
        if code != Some(status) {
            return Err(format!(
                "`aye test {}`: expected exit status {status}, got {code:?}",
                args.join(" ")
            ));
        }
    }
    Ok(())
}

/// Run the `aye` binary in `dir` and return (stdout, stderr, exit status).
fn aye(dir: &Path, args: &[&str]) -> Result<(String, String, Option<i32>), String> {
    let out = Command::new(env!("CARGO_BIN_EXE_aye"))
        .current_dir(dir)
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    Ok((
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    ))
}

fn fix_case(path: &Path, work: &Path) -> Result<(), String> {
    let file = name(path);
    let dir = work.join(format!(
        "fix-{}",
        path.file_stem().unwrap().to_string_lossy()
    ));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::copy(path, dir.join(&file)).map_err(|e| e.to_string())?;
    let want_file = read(&path.with_extension("fixed")).ok_or("missing .fixed file")?;
    let want_out = read(&path.with_extension("out")).ok_or("missing .out file")?;
    let want_err = read(&path.with_extension("stderr"));
    let (out, err, code) = aye(&dir, &["fix", &file])?;
    let got_file = read(&dir.join(&file)).unwrap();
    if got_file != want_file {
        return Err(format!(
            "fixed file mismatch\n--- expected\n{want_file}--- got\n{got_file}"
        ));
    }
    if out != want_out {
        return Err(format!(
            "`aye fix` stdout mismatch\n--- expected\n{want_out}--- got\n{out}"
        ));
    }
    if err != want_err.clone().unwrap_or_default() {
        return Err(format!("`aye fix` stderr mismatch\n--- got\n{err}"));
    }
    let want_code = if want_err.is_some() { 1 } else { 0 };
    if code != Some(want_code) {
        return Err(format!(
            "`aye fix`: expected exit status {want_code}, got {code:?}"
        ));
    }
    if want_err.is_none() {
        let (_, err, code) = aye(&dir, &["check", &file])?;
        if code != Some(0) {
            return Err(format!("`aye check` rejects the fixed file:\n{err}"));
        }
    }
    if let Some(want) = read(&path.with_extension("json")) {
        let rel = format!("tests/fix/{file}");
        let (out, _, _) = aye(
            Path::new(env!("CARGO_MANIFEST_DIR")),
            &["check", "--json", &rel],
        )?;
        if out != want {
            return Err(format!(
                "`aye check --json` mismatch\n--- expected\n{want}--- got\n{out}"
            ));
        }
    }
    Ok(())
}

/// Run the `aye` binary in `dir` with `stdin`, and return (stdout, stderr,
/// exit status).
fn aye_stdin(
    dir: &Path,
    args: &[&str],
    stdin: &str,
) -> Result<(String, String, Option<i32>), String> {
    use std::io::Write;
    let mut child = Command::new(env!("CARGO_BIN_EXE_aye"))
        .current_dir(dir)
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .map_err(|e| e.to_string())?;
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    Ok((
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    ))
}

/// The directory modules of `tests/modules`.
fn module_dirs() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/modules");
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_dir())
        .collect();
    v.sort();
    v
}

fn module_case(path: &Path) -> Result<(), String> {
    let dir = path.parent().unwrap();
    let x = name(path);
    let with = |ext: &str| read(&dir.join(format!("{x}.{ext}")));
    if let Some(want) = with("err") {
        let (_, err, code) = aye(dir, &["check", &x])?;
        if err != want || code != Some(1) {
            return Err(format!(
                "`aye check {x}`: {code:?}\n--- expected\n{want}--- got\n{err}"
            ));
        }
        return Ok(());
    }
    let args = with("args").unwrap_or_else(|| format!("run {x}"));
    let args: Vec<&str> = args.split_whitespace().collect();
    let (out, err, code) = aye_stdin(dir, &args, &with("stdin").unwrap_or_default())?;
    let want_out = with("out").ok_or("missing .out file")?;
    let want_err = with("stderr").unwrap_or_default();
    let want_code: i32 = with("status").map_or(0, |s| s.trim().parse().unwrap());
    if out != want_out || err != want_err || code != Some(want_code) {
        return Err(format!(
            "`aye {}`: {code:?} (expected {want_code})\n--- expected stdout\n{want_out}--- got\n{out}--- expected stderr\n{want_err}--- got\n{err}",
            args.join(" ")
        ));
    }
    Ok(())
}

/// Example 1 of the design doc: `readConfig` reads `app.toml` from the
/// working directory. Each of `ok`, `missing` and `invalid` is a working
/// directory with its expected stderr in `X.stderr` (default empty); stdout
/// is always empty. `aye test app` runs the tests in `app/config.aye`.
fn example1() -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/example1");
    for case in ["ok", "missing", "invalid"] {
        let want_err = read(&root.join(format!("{case}.stderr"))).unwrap_or_default();
        let want_code = if want_err.is_empty() { 0 } else { 1 };
        let (out, err, code) = aye(&root.join(case), &["run", "../app"])?;
        if !out.is_empty() || err != want_err || code != Some(want_code) {
            return Err(format!(
                "[{case}] `aye run ../app`: {code:?}\n--- stdout\n{out}--- expected stderr\n{want_err}--- got\n{err}"
            ));
        }
    }
    let want = read(&root.join("app.test.out")).ok_or("missing app.test.out")?;
    let (out, err, code) = aye(&root, &["test", "app"])?;
    if out != want || !err.is_empty() || code != Some(0) {
        return Err(format!(
            "`aye test app`: {code:?}\n--- expected\n{want}--- got\n{out}{err}"
        ));
    }
    Ok(())
}

/// `aye test --filter`: only the tests whose name contains the text run.
fn cli_test_filter() -> Result<(), String> {
    let f = "tests/tests/outcomes.aye";
    let want = format!(
        "test \"expect fails\" ... FAILED\n    before\n    {f}:28:5: assertion failed: condition is false\n\
         test \"expectEqual fails\" ... FAILED\n    {f}:33:5: assertion failed: expected 4, found 5\n\n0 passed; 2 failed\n"
    );
    let (got, code) = aye_test(&[f, "--filter", "expect"])?;
    if got != want || code != Some(1) {
        return Err(format!(
            "`aye test --filter expect`: {code:?}\n--- expected\n{want}--- got\n{got}"
        ));
    }
    let (got, code) = aye_test(&[f, "--filter", "no such test"])?;
    if got != "\n0 passed; 0 failed\n" || code != Some(0) {
        return Err(format!(
            "`aye test --filter` with no match: {code:?}\n{got}"
        ));
    }
    Ok(())
}

/// `aye test DIR --module PATH` runs the tests of one module of the
/// program; a module that imports a sibling, given on its own, gets a hint.
fn cli_test_module() -> Result<(), String> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/modules");
    let runs: &[(&[&str], &str, &str, i32)] = &[
        (
            &["test", "modtests", "--module", "dom"],
            "test \"dom: basic\" ... ok\n\n1 passed; 0 failed\n",
            "",
            0,
        ),
        (
            &["test", "modtests", "--module", "nope"],
            "",
            "aye: no tests in module `nope` (modules with tests: `html`, `dom`)\n",
            1,
        ),
        (
            &["test", "modtests/html"],
            "",
            "modtests/html/html.aye:1:8: error[E0081]: cannot find module `dom` (no directory `modtests/html/dom`; imports are \
             found from the directory given to `aye`, and `modtests/dom` exists: give `modtests` instead, with `--module html` for `aye test`)\n",
            1,
        ),
    ];
    for (args, want_out, want_err, want_code) in runs {
        let (out, err, code) = aye(&dir, args)?;
        if out != *want_out || err != *want_err || code != Some(*want_code) {
            return Err(format!(
                "`aye {}`: {code:?}\n--- stdout\n{out}--- stderr\n{err}",
                args.join(" ")
            ));
        }
    }
    Ok(())
}

/// The `aye run` CLI end to end.
fn cli_run(work: &Path) -> Result<(), String> {
    let _ = work;
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/cases/hello.aye");
    let out = Command::new(env!("CARGO_BIN_EXE_aye"))
        .arg("run")
        .arg(&file)
        .output()
        .map_err(|e| e.to_string())?;
    let expected = read(&file.with_extension("out")).unwrap();
    if !out.status.success() || String::from_utf8_lossy(&out.stdout) != expected {
        return Err(format!(
            "`aye run` failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(())
}

/// `aye build -o PATH` links the libraries of `@link`, and the program
/// runs (from the crate root, where it finds its data file).
fn cli_build_link(work: &Path) -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = root.join("tests/ffi/curl_file.aye");
    if !missing_packages(&src).is_empty() {
        println!("skipped cli/aye build -o: pkg-config does not know libcurl");
        return Ok(());
    }
    let exe = work.join("curl_file_cli");
    let (_, err, code) = aye(
        root,
        &[
            "build",
            "tests/ffi/curl_file.aye",
            "-o",
            exe.to_str().unwrap(),
        ],
    )?;
    if code != Some(0) {
        return Err(format!("`aye build -o` failed: {code:?}\n{err}"));
    }
    let out = Command::new(&exe)
        .current_dir(root)
        .output()
        .map_err(|e| e.to_string())?;
    let want = read(&src.with_extension("out")).unwrap();
    if !out.status.success() || String::from_utf8_lossy(&out.stdout) != want {
        return Err(format!(
            "the program of `aye build -o` printed:\n{}",
            String::from_utf8_lossy(&out.stdout)
        ));
    }
    Ok(())
}

/// Run `cmd` and wait at most `secs` seconds for it (`None` on a timeout:
/// it is killed).
fn output_within(cmd: &mut Command, secs: u64) -> Option<std::process::Output> {
    use std::process::Stdio;
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    let start = std::time::Instant::now();
    loop {
        if child.try_wait().ok()?.is_some() {
            return child.wait_with_output().ok();
        }
        if start.elapsed().as_secs() >= secs {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

/// Like `output_within`, and also the CPU time (user + system) the child used,
/// from `wait4`. Unlike wall time, load on the machine does not change it.
/// The output is read after the child exits: it must fit in a pipe.
fn output_and_cpu_within(
    cmd: &mut Command,
    secs: u64,
) -> Option<(std::process::Output, std::time::Duration)> {
    use std::io::Read;
    use std::os::unix::process::ExitStatusExt;
    use std::process::Stdio;
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
    unsafe extern "C" {
        fn wait4(pid: i32, status: *mut i32, options: i32, rusage: *mut Rusage) -> i32;
    }
    const WNOHANG: i32 = 1;
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    let pid = child.id() as i32;
    let (mut status, mut ru) = (0, unsafe { std::mem::zeroed::<Rusage>() });
    let start = std::time::Instant::now();
    loop {
        match unsafe { wait4(pid, &mut status, WNOHANG, &mut ru) } {
            0 => {}
            r if r == pid => break,
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
        if start.elapsed().as_secs() >= secs {
            let _ = child.kill();
            unsafe { wait4(pid, &mut status, 0, &mut ru) };
            return None;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    child.stdout.take()?.read_to_end(&mut stdout).ok()?;
    child.stderr.take()?.read_to_end(&mut stderr).ok()?;
    let time =
        |t: &Timeval| Duration::from_secs(t.sec as u64) + Duration::from_micros(t.usec as u64);
    let status = std::process::ExitStatus::from_raw(status);
    Some((
        std::process::Output {
            status,
            stdout,
            stderr,
        },
        time(&ru.utime) + time(&ru.stime),
    ))
}

/// Does clang's ThreadSanitizer work here: a C program with two threads,
/// built with `-fsanitize=thread`, runs cleanly?
fn tsan_works(work: &Path) -> bool {
    let src = work.join("tsan_probe.c");
    let exe = work.join("tsan_probe");
    let c = "#include <pthread.h>\nstatic void *f(void *p) { return p; }\n\
             int main(void) { pthread_t t; pthread_create(&t, 0, f, 0); return pthread_join(t, 0); }\n";
    if std::fs::write(&src, c).is_err() {
        return false;
    }
    let built = Command::new("clang")
        .args(["-fsanitize=thread", "-pthread"])
        .arg(&src)
        .arg("-o")
        .arg(&exe)
        .status();
    if !built.is_ok_and(|s| s.success()) {
        return false;
    }
    output_within(&mut Command::new(&exe), 30)
        .is_some_and(|o| o.status.success() && o.stderr.is_empty())
}

/// M10: the fan-out program built with `--cflags -fsanitize=thread` runs
/// without a ThreadSanitizer report and prints what it prints without it.
fn cli_tsan(work: &Path) -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    // The flags reach clang: one it does not know fails the build.
    let bad = work.join("hello_bad_flag");
    let (_, err, code) = aye(
        root,
        &[
            "build",
            "tests/cases/hello.aye",
            "--cflags",
            "-fno-such-aye-flag",
            "-o",
            bad.to_str().unwrap(),
        ],
    )?;
    if code != Some(1) || !err.contains("clang failed") {
        return Err(format!(
            "`aye build --cflags -fno-such-aye-flag`: {code:?}\n{err}"
        ));
    }
    if !tsan_works(work) {
        println!(
            "skipped cli/aye build --cflags -fsanitize=thread: clang's ThreadSanitizer does not run here"
        );
        return Ok(());
    }
    let exe = work.join("threads_fanout_tsan");
    let args = [
        "build",
        "tests/cases/threads_fanout.aye",
        "--cflags",
        "-fsanitize=thread",
        "-o",
        exe.to_str().unwrap(),
    ];
    let (_, err, code) = aye(root, &args)?;
    if code != Some(0) {
        return Err(format!("`aye {}` failed: {code:?}\n{err}", args.join(" ")));
    }
    let out = output_within(&mut Command::new(&exe), 120)
        .ok_or("the program did not finish within 120 s")?;
    let want = read(&root.join("tests/cases/threads_fanout.out")).unwrap();
    let (stdout, stderr) = (
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    if !out.status.success() || stdout != want || !stderr.is_empty() {
        return Err(format!(
            "under ThreadSanitizer: {}\n--- stdout\n{stdout}--- stderr\n{stderr}",
            out.status
        ));
    }
    Ok(())
}

/// M9.7: `General` in debug mode allocates and frees in constant time:
/// `tests/cases/general_churn.aye` (a million allocation and free pairs,
/// and a recursion that allocates per call), built in debug mode, uses at
/// most 2 s of CPU time (0.13 s on 2026-10-07). Before M9.7 each free
/// scanned every allocation so far. CPU time, not wall time: under load
/// the wall time grew past the old 10 s limit (P3.11).
fn perf_general(work: &Path) -> Result<(), String> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/cases/general_churn.aye");
    let c = driver::compile_to_c(&name(&src), &read(&src).unwrap(), Mode::Debug)
        .map_err(|d| join(&d))?;
    let exe = work.join("general_churn_perf");
    driver::cc(&c, &exe, Mode::Debug)?;
    // The wall-time limit only catches a hang.
    let (out, cpu) = output_and_cpu_within(&mut Command::new(&exe), 120)
        .ok_or("the debug build did not finish within 120 s")?;
    if !out.status.success() {
        return Err(format!(
            "the debug build failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    if cpu > std::time::Duration::from_secs(2) {
        return Err(format!(
            "the debug build used {} ms of CPU time; the limit is 2000 ms",
            cpu.as_millis()
        ));
    }
    println!("perf/General: {} ms CPU", cpu.as_millis());
    Ok(())
}

/// The `aye check` CLI: exit code, stdout and stderr, text and `--json`.
fn cli_check() -> Result<(), String> {
    let ok = "tests/cases/hello.aye";
    let bad = "tests/errors/unknown_escape.aye";
    let three = "tests/errors/three_errors.aye";
    let fixable = "tests/fix/let_var.aye";
    let diag = |code: &str, line: u32, col: u32, msg: &str| {
        format!(
            r#"{{"severity":"error","code":"{code}","file":"{three}","line":{line},"col":{col},"message":"{msg}","fixes":[]}}"#
        )
    };
    let checks: &[(&[&str], bool, String, String)] = &[
        (&[ok], true, String::new(), String::new()),
        (
            &[bad],
            false,
            String::new(),
            format!("{bad}:2:16: error[E0005]: unknown escape sequence `\\q`\n"),
        ),
        (
            &[ok, "--json"],
            true,
            format!("{{\"file\":\"{ok}\",\"diagnostics\":[]}}\n"),
            String::new(),
        ),
        (
            &[bad, "--json"],
            false,
            format!(
                "{{\"file\":\"{bad}\",\"diagnostics\":[{{\"severity\":\"error\",\"code\":\"E0005\",\"file\":\"{bad}\",\"line\":2,\"col\":16,\
                 \"message\":\"unknown escape sequence `\\\\q`\",\"fixes\":[]}}]}}\n"
            ),
            String::new(),
        ),
        (
            &[three, "--json"],
            false,
            format!(
                "{{\"file\":\"{three}\",\"diagnostics\":[{},{},{}]}}\n",
                diag(
                    "E0023",
                    4,
                    12,
                    "mismatched types: expected `i64`, found `i32`"
                ),
                diag("E0038", 9, 16, "struct `P` has no field `y`"),
                diag("E0018", 11, 14, "undefined variable `missing`"),
            ),
            String::new(),
        ),
        (
            &[fixable],
            false,
            String::new(),
            format!(
                "{fixable}:4:9: error[E0032]: cannot assign to `sum`: it is immutable (declare it with `var`)\n\
                 {fixable}:2:5: help: declare `sum` with `var`\n\
                 {fixable}:11:14: error[E0019]: undefined function `totl`\n\
                 {fixable}:11:14: help: change `totl` to `total`\n"
            ),
        ),
    ];
    for (args, success, stdout, stderr) in checks {
        let out = Command::new(env!("CARGO_BIN_EXE_aye"))
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .arg("check")
            .args(*args)
            .output()
            .map_err(|e| e.to_string())?;
        let (got_out, got_err) = (
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        );
        if out.status.success() != *success || got_out != *stdout || got_err != *stderr {
            return Err(format!(
                "`aye check {}`: {}\n--- stdout\n{got_out}--- stderr\n{got_err}",
                args.join(" "),
                out.status
            ));
        }
    }
    Ok(())
}

/// A compile error in a doc comment's example is reported where it is in
/// the comment; `aye run` and `aye check` ignore the examples.
fn cli_doc_example_error(work: &Path) -> Result<(), String> {
    let src = "struct S {\n    x: i64\n\n    /// ```aye\n    /// let y: bool = S.one()\n    /// ```\n    pub fn one() i64 {\n        return 1\n    }\n}\n\npub fn main() {}\n";
    std::fs::write(work.join("doc_error.aye"), src).map_err(|e| e.to_string())?;
    let want = "doc_error.aye:5:25: error[E0023]: mismatched types: expected `bool`, found `i64`\n";
    let (out, err, code) = aye(work, &["test", "doc_error.aye"])?;
    if !out.is_empty() || err != want || code != Some(1) {
        return Err(format!(
            "`aye test doc_error.aye`: {code:?}\n--- expected stderr\n{want}--- got\n{out}{err}"
        ));
    }
    let (out, err, code) = aye(work, &["check", "doc_error.aye"])?;
    if !out.is_empty() || !err.is_empty() || code != Some(0) {
        return Err(format!("`aye check doc_error.aye`: {code:?}\n{out}{err}"));
    }
    Ok(())
}

/// `aye explain <code>`: known code (exit 0), unknown code (exit 2).
fn cli_explain() -> Result<(), String> {
    let run = |code: &str| {
        Command::new(env!("CARGO_BIN_EXE_aye"))
            .args(["explain", code])
            .output()
    };
    let out = run("E0056").map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&out.stdout);
    if !out.status.success()
        || !text.starts_with("E0056: ")
        || !text.contains("Wrong:")
        || !text.contains("Fix:")
    {
        return Err(format!("`aye explain E0056`: {}\n{text}", out.status));
    }
    let out = run("E9999").map_err(|e| e.to_string())?;
    let err = String::from_utf8_lossy(&out.stderr);
    if out.status.code() != Some(2)
        || !out.stdout.is_empty()
        || err != "aye: unknown error code `E9999`\n"
    {
        return Err(format!("`aye explain E9999`: {}\n{err}", out.status));
    }
    Ok(())
}

fn main() {
    let work = std::env::temp_dir().join(format!("aye-golden-{}", std::process::id()));
    std::fs::create_dir_all(&work).unwrap();

    let run_cases = cases("cases");
    let err_cases = cases("errors");
    let test_cases = cases("tests");
    let fix_cases = cases("fix");
    let ffi_cases = cases("ffi");
    let cimport_cases = cases("cimport");
    let mod_cases = module_dirs();
    type Job<'a> = Box<dyn FnOnce() -> Result<(), String> + Send + 'a>;
    fn add<'a>(
        jobs: &mut Vec<(String, Job<'a>)>,
        n: String,
        f: impl FnOnce() -> Result<(), String> + Send + 'a,
    ) {
        jobs.push((n, Box::new(f)));
    }
    let results: Vec<(String, Result<(), String>)> = {
        let mut jobs = Vec::new();
        for p in &run_cases {
            let w = &work;
            add(&mut jobs, format!("cases/{}", name(p)), move || {
                tool_case(p, w)
            });
        }
        for p in &ffi_cases {
            let w = &work;
            add(&mut jobs, format!("ffi/{}", name(p)), move || {
                ffi_case(p, w)
            });
        }
        for p in &err_cases {
            add(&mut jobs, format!("errors/{}", name(p)), move || {
                error_case(p)
            });
        }
        for p in &cimport_cases {
            let w = &work;
            add(&mut jobs, format!("cimport/{}", name(p)), move || {
                cimport_case(p, w)
            });
        }
        for p in &test_cases {
            add(&mut jobs, format!("tests/{}", name(p)), move || {
                test_case(p)
            });
        }
        for p in &fix_cases {
            let w = &work;
            add(&mut jobs, format!("fix/{}", name(p)), move || {
                fix_case(p, w)
            });
        }
        for p in &mod_cases {
            add(&mut jobs, format!("modules/{}", name(p)), move || {
                module_case(p)
            });
        }
        add(&mut jobs, "example1".to_string(), example1);
        let w = &work;
        add(&mut jobs, "cli/aye run".to_string(), move || cli_run(w));
        add(&mut jobs, "cli/aye build -o".to_string(), move || {
            cli_build_link(w)
        });
        add(&mut jobs, "cli/aye check".to_string(), cli_check);
        add(
            &mut jobs,
            "cli/aye build --cflags -fsanitize=thread".to_string(),
            move || cli_tsan(w),
        );
        add(&mut jobs, "cli/aye explain".to_string(), cli_explain);
        add(
            &mut jobs,
            "cli/aye test: an error in a doc example".to_string(),
            move || cli_doc_example_error(w),
        );
        add(
            &mut jobs,
            "perf/General in debug mode".to_string(),
            move || perf_general(w),
        );
        add(
            &mut jobs,
            "cli/aye test --filter".to_string(),
            cli_test_filter,
        );
        add(
            &mut jobs,
            "cli/aye test --module".to_string(),
            cli_test_module,
        );
        // At most `available_parallelism()` cases run at once.
        let queue = std::sync::Mutex::new(jobs.into_iter().enumerate());
        let done = std::sync::Mutex::new(Vec::new());
        let workers = std::thread::available_parallelism().map_or(1, |n| n.get());
        std::thread::scope(|s| {
            for _ in 0..workers {
                s.spawn(|| {
                    while let Some((i, (n, f))) = {
                        let next = queue.lock().unwrap().next();
                        next
                    } {
                        done.lock().unwrap().push((i, n, f()));
                    }
                });
            }
        });
        let mut done = done.into_inner().unwrap();
        done.sort_by_key(|d| d.0);
        done.into_iter().map(|(_, n, r)| (n, r)).collect()
    };
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
    println!(
        "\ngolden: {} passed; {failed} failed",
        results.len() - failed
    );
    if failed > 0 {
        std::process::exit(1);
    }
}
