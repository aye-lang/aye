//! Golden tests.
//!
//! - `tests/cases/X.aye`: compiled in debug, safe and fast mode, run, and
//!   stdout compared with `X.out`. If `X.panic` exists the program must abort
//!   with exactly that on stderr (checked in debug and safe mode only). If
//!   `X.stderr` exists, `main` must return an error: exit status 1, and
//!   stderr equal to `X.stderr` in debug mode; safe and fast mode print no
//!   error return trace, so there stderr must be the first line only.
//!   A case with `X.needs` (`pkg-config` packages, or the word `ipv6`: a
//!   TCP socket can bind `[::1]`) or `X.tools` (programs on `PATH`) is
//!   skipped when one is missing.
//!   With the WASI toolchain and `node` there, each case also runs built for
//!   wasm32-wasip1 under `node:wasi` (`scripts/wasi-run.mjs`), `tests/tests`
//!   too, against the same goldens (`X.wasm32-wasip1.out` instead of
//!   `X.out` if it exists); the line `golden (wasm32-wasip1): N
//!   passed; M skipped` counts them. Skipped: a program that imports a std
//!   module the target lacks (E0118), and one with `X.native` (an empty
//!   marker file), `X.needs` or `X.tools`.
//!   A case with `X.freestanding` (a marker whose first line is the
//!   arguments for the loader, if any) also runs built for
//!   wasm32-freestanding under `tests/wasm/loader.mjs`, in every mode: with
//!   no arguments `main` runs and the goldens are `X.wasm32-freestanding.out`
//!   if it exists, else `X.out`, and `X.panic` (a
//!   panic exits with status 134); with arguments the loader calls that
//!   export, and stdout is `X.freestanding.out`. The line `golden
//!   (wasm32-freestanding): N passed` counts them.
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
//!   in `X.err`, one per line, in order. With `X.args` (such as `--target
//!   wasm32-wasip1`), `aye check X.aye <args>` must print exactly `X.err` on
//!   stderr and exit with status 1.
//! - `tests/tests/X.aye`: `aye test X.aye` (debug mode) must print exactly
//!   `X.out`, and `aye test --json X.aye` exactly `X.json` if it exists;
//!   with `X.args` (such as `--time-limit 1`) these come before the file.
//!   The exit status must be 0 when no test failed, and 1 otherwise. Wall
//!   and CPU times (`(12 ms)`, `"ms":12`, `"cpu_ms":12`, `time limit: 1.0 s`)
//!   are replaced by `N` before the comparison, here and for `aye test` in
//!   the other cases. `aye test -j 1` and `-j 4` on `std_testing.aye` must
//!   both print its `X.out`.
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
//! - `tests/deps/local/`: an app with a path dependency (`app/aye.toml` names
//!   `util = { path = "../util" }`). `aye run` in `app` (no path) prints
//!   `app.out`; `aye test` in `app` prints `app.test.out` (the dependency's
//!   tests are not the app's), `aye test` in `local` prints `all.test.out`;
//!   `aye build` names the executable after `name` in `aye.toml`.
//! - `tests/deps/errors/X/`: `aye check` in `X/app` (or the command in
//!   `X/expected.args`) must print exactly `X/expected.err` on stderr and
//!   exit with status 1, and leave every file of `X` as it was (`aye fix`
//!   never writes a dependency).
//! - `aye run`, `aye build -o`, `aye check`, `aye test --filter` and `aye
//!   test --module` are also run through the built binary.
//! - `tests/asan/X.aye`: built with `--cflags -fsanitize=address` in debug
//!   and safe mode, the program reads freed memory and must die with a
//!   report that contains each line of `X.asan` (`debug: ` and `safe: `
//!   lines count in that mode only), printing nothing. Skipped
//!   where AddressSanitizer does not run (Homebrew's LLVM is tried when the
//!   clang on `PATH` has none).
//! - `perf`: the debug build of `tests/cases/general_churn.aye` finishes
//!   within 10 s (`General` allocates and frees in constant time).
//! - `aye build --cflags -fsanitize=thread`: the M10 fan-out program runs
//!   under ThreadSanitizer without a report (skipped where ThreadSanitizer
//!   does not run; Homebrew's LLVM is tried when the clang on `PATH` has
//!   none).

use aye::diag::Diag;
use aye::driver::{self, Mode, Root, Target};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

mod common;

/// How many wasm32-wasip1 jobs skipped themselves.
static WASM_SKIPPED: AtomicUsize = AtomicUsize::new(0);

/// The wasm32-wasip1 target, if programs can be built and run for it here
/// (the WASI toolchain and `node`).
fn wasm_target() -> Option<&'static Target> {
    let t = driver::find_target("wasm32-wasip1").unwrap();
    (t.toolchain_available() && driver::node_available()).then_some(t)
}

/// Does the program import a std module that wasm32-wasip1 lacks (E0118)?
fn lacks_module(path: &Path) -> bool {
    let Some(src) = read(path) else { return false };
    let mut root = Root::single(&name(path), &src);
    root.target = driver::find_target("wasm32-wasip1").unwrap();
    driver::front_end(root, None)
        .result
        .is_err_and(|ds| ds.iter().any(|d| d.code == aye::codes::E0118))
}

/// Why `path` does not run on wasm32-wasip1, if it does not.
fn wasm_skip_reason(path: &Path) -> Option<&'static str> {
    if path.with_extension("native").exists() {
        Some("X.native")
    } else if path.with_extension("needs").exists() || path.with_extension("tools").exists() {
        Some("X.needs or X.tools")
    } else if lacks_module(path) {
        Some("imports a std module the target lacks")
    } else {
        None
    }
}

/// The wasm32-freestanding target, if programs can be built and run for it
/// here (clang with the wasm32 backend, and `node`).
fn freestanding_target() -> Option<&'static Target> {
    let t = driver::find_target("wasm32-freestanding").unwrap();
    (t.toolchain_available() && driver::node_available()).then_some(t)
}

/// `f` for a program, unless it does not run on wasm32-wasip1.
fn wasm_job(kind: &str, path: &Path, f: impl FnOnce() -> Result<(), String>) -> Result<(), String> {
    if let Some(why) = wasm_skip_reason(path) {
        println!("skipped wasm32-wasip1 {kind}/{}: {why}", name(path));
        WASM_SKIPPED.fetch_add(1, Ordering::Relaxed);
        return Ok(());
    }
    f()
}

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

/// Whether a TCP socket can bind the IPv6 loopback (`ipv6.disable=1` hosts
/// cannot).
fn ipv6_loopback() -> bool {
    std::net::TcpListener::bind("[::1]:0").is_ok()
}

/// The packages in `X.needs` that `pkg-config` does not know (the word
/// `ipv6` is not a package).
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
        .filter(|p| *p != "ipv6" && !known(p))
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
    run_case_for(path, work, driver::host_target())
}

/// `run_case` for a program built for `target`.
fn run_case_for(path: &Path, work: &Path, target: &'static Target) -> Result<(), String> {
    let src = read(path).unwrap();
    let expected_out = read(&path.with_extension(format!("{}.out", target.name)))
        .or_else(|| read(&path.with_extension("out")))
        .ok_or("missing .out file")?;
    let expected_panic = read(&path.with_extension("panic"));
    let expected_error = read(&path.with_extension("stderr"));
    let modes: &[Mode] = if expected_panic.is_some() {
        &[Mode::Debug, Mode::Safe]
    } else {
        &[Mode::Debug, Mode::Safe, Mode::Fast]
    };
    for &mode in modes {
        let mut root = Root::single(&name(path), &src);
        root.target = target;
        let c = driver::compile_root(root, mode).map_err(|d| join(&d))?;
        let exe = work.join(format!(
            "{}-{mode:?}-{}",
            path.file_stem().unwrap().to_string_lossy(),
            target.name
        ));
        driver::cc(&c, &exe, mode)?;
        let out = target
            .run_command(&exe)
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
/// when `pkg-config` does not know a package of `X.needs`, `X.needs` lists
/// `ipv6` and the host has no IPv6 loopback, or a program of `X.tools` is
/// not on `PATH`.
fn tool_case(path: &Path, work: &Path) -> Result<(), String> {
    let needs = read(&path.with_extension("needs")).unwrap_or_default();
    if needs.split_whitespace().any(|p| p == "ipv6") && !ipv6_loopback() {
        println!("skipped cases/{}: no IPv6 loopback", name(path));
        return Ok(());
    }
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
    if let Some(args) = read(&path.with_extension("args")) {
        let file = name(path);
        let mut argv = vec!["check", file.as_str()];
        argv.extend(args.split_whitespace());
        let (_, err, code) = aye(path.parent().unwrap(), &argv)?;
        if code == Some(1) && err.trim_end() == expected.trim_end() {
            return Ok(());
        }
        return Err(format!(
            "`aye {}`: status {code:?}\n--- expected\n{}\n--- got\n{}",
            argv.join(" "),
            expected.trim_end(),
            err.trim_end()
        ));
    }
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

/// `aye test` prints wall times: `(12 ms)` after a test, `; 34 ms` after the
/// summary, `"ms":12` in JSON; and CPU times: `"cpu_ms":12` in JSON, `time
/// limit: 1.0 s CPU of 1 s` after a test that went over the limit. Every
/// such number becomes `N`.
fn scrub_ms(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find(|c: char| c.is_ascii_digit()) {
        let (head, tail) = rest.split_at(i);
        let n = tail
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(tail.len());
        let (digits, after) = tail.split_at(n);
        out.push_str(head);
        if head.ends_with("time limit: ") && after.starts_with('.') {
            // The measured CPU seconds, `1.0`.
            let frac = after[1..]
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(after.len() - 1);
            out.push('N');
            rest = &after[1 + frac..];
            continue;
        }
        if head.ends_with("\"ms\":")
            || head.ends_with("\"cpu_ms\":")
            || after == " ms"
            || after.starts_with(" ms)")
            || after.starts_with(" ms\n")
        {
            out.push('N');
        } else {
            out.push_str(digits);
        }
        rest = after;
    }
    out.push_str(rest);
    out
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
        scrub_ms(&String::from_utf8_lossy(&out.stdout)),
        out.status.code(),
    ))
}

fn test_case(path: &Path) -> Result<(), String> {
    test_case_for(path, &[])
}

/// `test_case` with more arguments for `aye test` (`--target`).
fn test_case_for(path: &Path, extra: &[&str]) -> Result<(), String> {
    let rel = format!("tests/tests/{}", name(path));
    let expected = read(&path.with_extension("out")).ok_or("missing .out file")?;
    let status = if expected.contains("; 0 failed; ") {
        0
    } else {
        1
    };
    let file_args = read(&path.with_extension("args")).unwrap_or_default();
    let mut runs = vec![(vec![rel.as_str()], expected)];
    if let Some(json) = read(&path.with_extension("json")) {
        runs.push((vec!["--json", rel.as_str()], json));
    }
    for (mut args, want) in runs {
        args.splice(
            0..0,
            extra.iter().copied().chain(file_args.split_whitespace()),
        );
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

/// `aye test -j 1` and `-j 4` print the same, in test order: `std_testing`.
fn test_jobs_case() -> Result<(), String> {
    let rel = "tests/tests/std_testing.aye";
    let want = read(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/tests/std_testing.out"))
        .ok_or("missing std_testing.out")?;
    for j in ["1", "4"] {
        let (got, code) = aye_test(&["-j", j, rel])?;
        if got != want || code != Some(1) {
            return Err(format!(
                "`aye test -j {j} {rel}`: expected std_testing.out and exit status 1, got status {code:?} and\n{got}"
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
    let out = if args[0] == "test" {
        scrub_ms(&out)
    } else {
        out
    };
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
    let out = scrub_ms(&out);
    if out != want || !err.is_empty() || code != Some(0) {
        return Err(format!(
            "`aye test app`: {code:?}\n--- expected\n{want}--- got\n{out}{err}"
        ));
    }
    Ok(())
}

/// Every file under `dir` as (path relative to `dir`, text), sorted.
fn tree(dir: &Path) -> Vec<(String, String)> {
    fn walk(base: &Path, dir: &Path, out: &mut Vec<(String, String)>) {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        entries.sort();
        for p in entries {
            if p.is_dir() {
                walk(base, &p, out);
            } else {
                let rel = p.strip_prefix(base).unwrap().to_string_lossy().into_owned();
                out.push((rel, std::fs::read_to_string(&p).unwrap_or_default()));
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out
}

/// Copy the directory `from` to `to`.
fn copy_tree(from: &Path, to: &Path) {
    for (rel, text) in tree(from) {
        let p = to.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
}

/// `tests/deps/local`: an app and its path dependency.
fn deps_local(work: &Path) -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/deps/local");
    let want = read(&root.join("app.out")).ok_or("missing app.out")?;
    let (out, err, code) = aye(&root.join("app"), &["run"])?;
    if out != want || !err.is_empty() || code != Some(0) {
        return Err(format!(
            "`aye run`: {code:?}\n--- expected\n{want}--- got\n{out}{err}"
        ));
    }
    for (dir, file) in [("app", "app.test.out"), (".", "all.test.out")] {
        let want = read(&root.join(file)).ok_or(format!("missing {file}"))?;
        let (out, err, code) = aye(&root.join(dir), &["test"])?;
        let out = scrub_ms(&out);
        if out != want || !err.is_empty() || code != Some(0) {
            return Err(format!(
                "`aye test` in {dir}: {code:?}\n--- expected\n{want}--- got\n{out}{err}"
            ));
        }
    }
    let copy = work.join("deps-local");
    let _ = std::fs::remove_dir_all(&copy);
    copy_tree(&root, &copy);
    let (_, err, code) = aye(&copy.join("app"), &["build"])?;
    if code != Some(0) || !copy.join("app/app").is_file() {
        return Err(format!(
            "`aye build` did not make `app` (the name in aye.toml): {code:?}\n{err}"
        ));
    }
    let _ = std::fs::remove_dir_all(&copy);
    Ok(())
}

/// A case of `tests/deps/errors`.
fn deps_error_case(dir: &Path, work: &Path) -> Result<(), String> {
    let x = name(dir);
    let want = read(&dir.join("expected.err")).ok_or("missing expected.err")?;
    let args = read(&dir.join("expected.args")).unwrap_or_else(|| "check".to_string());
    let args: Vec<&str> = args.split_whitespace().collect();
    let copy = work.join(format!("deps-errors-{x}"));
    let _ = std::fs::remove_dir_all(&copy);
    copy_tree(dir, &copy);
    let (_, err, code) = aye(&copy.join("app"), &args)?;
    let changed = tree(&copy) != tree(dir);
    let _ = std::fs::remove_dir_all(&copy);
    if err != want || code != Some(1) {
        return Err(format!(
            "`aye {}`: {code:?}\n--- expected\n{want}--- got\n{err}",
            args.join(" ")
        ));
    }
    if changed {
        return Err(format!("`aye {}` wrote a file", args.join(" ")));
    }
    Ok(())
}

/// `aye test --filter`: only the tests whose name contains the text run.
fn cli_test_filter() -> Result<(), String> {
    let f = "tests/tests/outcomes.aye";
    let want = format!(
        "test \"expect fails\" ... FAILED (N ms)\n    before\n    {f}:28:5: assertion failed: condition is false\n\
         test \"expectEqual fails\" ... FAILED (N ms)\n    {f}:33:5: assertion failed: expected 4, found 5\n\n0 passed; 2 failed; 1 modules; N ms\n"
    );
    let (got, code) = aye_test(&[f, "--filter", "expect"])?;
    if got != want || code != Some(1) {
        return Err(format!(
            "`aye test --filter expect`: {code:?}\n--- expected\n{want}--- got\n{got}"
        ));
    }
    let (got, code) = aye_test(&[f, "--filter", "no such test"])?;
    if got != "\n0 passed; 0 failed; 0 modules; N ms\n" || code != Some(0) {
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
            "test \"dom: basic\" ... ok (N ms)\n\n1 passed; 0 failed; 1 modules; N ms\n",
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
        let out = scrub_ms(&out);
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

/// `aye build --target x86_64-macos` makes a Mach-O x86_64 executable; it
/// runs when Rosetta is there. Skipped off macOS.
fn cli_target_x86_64(work: &Path) -> Result<(), String> {
    if !cfg!(target_os = "macos") {
        println!("skipped cli/aye build --target x86_64-macos: not macOS");
        return Ok(());
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let exe = work.join("hello_x86_64");
    let (_, err, code) = aye(
        root,
        &[
            "build",
            "--target",
            "x86_64-macos",
            "tests/cases/hello.aye",
            "-o",
            exe.to_str().unwrap(),
        ],
    )?;
    if code != Some(0) {
        return Err(format!(
            "`aye build --target x86_64-macos`: {code:?}\n{err}"
        ));
    }
    // A Mach-O 64-bit header: magic `cf fa ed fe`, then the CPU type
    // (x86_64: `07 00 00 01`).
    let bytes = std::fs::read(&exe).map_err(|e| e.to_string())?;
    if bytes[..8] != [0xcf, 0xfa, 0xed, 0xfe, 0x07, 0x00, 0x00, 0x01] {
        return Err(format!("not a Mach-O x86_64 file: {:02x?}", &bytes[..8]));
    }
    if !Command::new("arch")
        .args(["-x86_64", "true"])
        .status()
        .is_ok_and(|s| s.success())
    {
        println!("skipped running cli/aye build --target x86_64-macos: no Rosetta");
        return Ok(());
    }
    let out = Command::new("arch")
        .arg("-x86_64")
        .arg(&exe)
        .output()
        .map_err(|e| e.to_string())?;
    let want = read(&root.join("tests/cases/hello.out")).unwrap();
    if !out.status.success() || String::from_utf8_lossy(&out.stdout) != want {
        return Err(format!(
            "the x86_64 program printed:\n{}",
            String::from_utf8_lossy(&out.stdout)
        ));
    }
    Ok(())
}

/// `aye build --target <arch>-linux -c` makes an ELF 64-bit relocatable
/// object of `e_machine` `machine` without linking, so without a Linux
/// sysroot.
fn cli_target_linux_obj(work: &Path, arch: &str, machine: u16) -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let target = format!("{arch}-linux");
    let obj = work.join(format!("hello_{arch}.o"));
    let (_, err, code) = aye(
        root,
        &[
            "build",
            "--target",
            &target,
            "-c",
            "tests/cases/hello.aye",
            "-o",
            obj.to_str().unwrap(),
        ],
    )?;
    if code != Some(0) {
        return Err(format!("`aye build --target {target} -c`: {code:?}\n{err}"));
    }
    // The ELF header: magic, class 2 (64-bit), little endian, type 1
    // (relocatable) at offset 16 and the machine at offset 18.
    let bytes = std::fs::read(&obj).map_err(|e| e.to_string())?;
    let want_machine = machine.to_le_bytes();
    if bytes[..6] != [0x7f, b'E', b'L', b'F', 2, 1]
        || bytes[16..18] != [1, 0]
        || bytes[18..20] != want_machine
    {
        return Err(format!(
            "not an ELF 64-bit {arch} object: {:02x?}",
            &bytes[..20]
        ));
    }
    Ok(())
}

/// Linking for a Linux target on macOS fails (clang has no Linux sysroot
/// there), and the message names the target and `-c`. Skipped off macOS,
/// and where a Linux sysroot is installed and the link works.
fn cli_target_linux_link_error(work: &Path) -> Result<(), String> {
    if !cfg!(target_os = "macos") {
        println!("skipped cli/aye build --target x86_64-linux: not macOS");
        return Ok(());
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let exe = work.join("hello_linux_link");
    let (_, err, code) = aye(
        root,
        &[
            "build",
            "--target",
            "x86_64-linux",
            "tests/cases/hello.aye",
            "-o",
            exe.to_str().unwrap(),
        ],
    )?;
    if code == Some(0) {
        println!("skipped cli/aye build --target x86_64-linux: a Linux sysroot is installed");
        return Ok(());
    }
    if code != Some(1)
        || !err.contains("aye: clang failed for target x86_64-linux (")
        || !err.contains("`aye build -c`")
    {
        return Err(format!(
            "`aye build --target x86_64-linux` failed without the message: {code:?}\n{err}"
        ));
    }
    Ok(())
}

/// `aye build --target wasm32-wasip1` links `hello.aye` into a WebAssembly
/// module (bytes `\0asm`). Skipped without the WASI toolchain.
fn cli_target_wasm(work: &Path) -> Result<(), String> {
    let wasi = driver::find_target("wasm32-wasip1").unwrap();
    if !wasi.toolchain_available() {
        println!("skipped cli/aye build --target wasm32-wasip1: no WASI toolchain");
        return Ok(());
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let wasm = work.join("hello.wasm");
    let (_, err, code) = aye(
        root,
        &[
            "build",
            "--target",
            "wasm32-wasip1",
            "tests/cases/hello.aye",
            "-o",
            wasm.to_str().unwrap(),
        ],
    )?;
    if code != Some(0) {
        return Err(format!(
            "`aye build --target wasm32-wasip1`: {code:?}\n{err}"
        ));
    }
    let bytes = std::fs::read(&wasm).map_err(|e| e.to_string())?;
    if bytes[..4] != *b"\0asm" {
        return Err(format!("not a wasm file: {:02x?}", &bytes[..4]));
    }
    // `aye run` runs it under Node when `node` is there.
    if !Command::new("node")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
    {
        println!("skipped running cli/aye build --target wasm32-wasip1: no node");
        return Ok(());
    }
    let (out, err, code) = aye(
        root,
        &["run", "--target", "wasm32-wasip1", "tests/cases/hello.aye"],
    )?;
    let want = read(&root.join("tests/cases/hello.out")).unwrap();
    if code != Some(0) || out != want {
        return Err(format!(
            "`aye run --target wasm32-wasip1`: {code:?}\n{out}{err}"
        ));
    }
    Ok(())
}

/// A `tests/cases` program with `X.freestanding`, built for
/// wasm32-freestanding and run by `tests/wasm/loader.mjs` (see the top of
/// this file for the goldens).
fn freestanding_case(path: &Path, work: &Path, target: &'static Target) -> Result<(), String> {
    let src = read(path).unwrap();
    let marker = read(&path.with_extension("freestanding")).unwrap_or_default();
    let args: Vec<&str> = marker
        .lines()
        .next()
        .unwrap_or("")
        .split_whitespace()
        .collect();
    let golden = if args.is_empty() {
        "out"
    } else {
        "freestanding.out"
    };
    let expected_out = read(&path.with_extension(format!("{}.out", target.name)))
        .filter(|_| args.is_empty())
        .or_else(|| read(&path.with_extension(golden)))
        .ok_or("missing golden")?;
    let expected_panic = read(&path.with_extension("panic")).filter(|_| args.is_empty());
    let expected_error = read(&path.with_extension("stderr")).filter(|_| args.is_empty());
    let modes: &[Mode] = if expected_panic.is_some() {
        &[Mode::Debug, Mode::Safe]
    } else {
        &[Mode::Debug, Mode::Safe, Mode::Fast]
    };
    for &mode in modes {
        let mut root = Root::single(&name(path), &src);
        root.target = target;
        let c = driver::compile_root(root, mode).map_err(|d| join(&d))?;
        let exe = work.join(format!(
            "{}-{mode:?}-{}",
            path.file_stem().unwrap().to_string_lossy(),
            target.name
        ));
        driver::cc(&c, &exe, mode)?;
        let out = target
            .run_command(&exe)
            .args(&args)
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
            if out.status.code() != Some(1) || stderr != want {
                return Err(format!(
                    "[{mode:?}] expected exit status 1 and stderr\n{want}--- got {}\n{stderr}",
                    out.status
                ));
            }
            continue;
        }
        match &expected_panic {
            Some(p) => {
                if out.status.code() != Some(134) || stderr != *p {
                    return Err(format!(
                        "[{mode:?}] expected exit status 134 and stderr\n{p}--- got {}\n{stderr}",
                        out.status
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

/// `aye build --target wasm32-freestanding` links a program with no `main`,
/// only `pub extern fn add`, into a module with no WASI imports; the loader
/// calls `add 2 3` and prints 5. Skipped without clang's wasm32 backend or
/// `node`.
fn cli_target_freestanding(work: &Path) -> Result<(), String> {
    if freestanding_target().is_none() {
        println!("skipped cli/aye build --target wasm32-freestanding: no wasm32 clang or no node");
        return Ok(());
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = work.join("export_only.aye");
    std::fs::write(
        &src,
        "pub extern fn add(a: i32, b: i32) i32 {\n    return a + b\n}\n",
    )
    .map_err(|e| e.to_string())?;
    let wasm = work.join("export_only.wasm");
    let (_, err, code) = aye(
        root,
        &[
            "build",
            "--target",
            "wasm32-freestanding",
            src.to_str().unwrap(),
            "-o",
            wasm.to_str().unwrap(),
        ],
    )?;
    if code != Some(0) {
        return Err(format!(
            "`aye build --target wasm32-freestanding`: {code:?}\n{err}"
        ));
    }
    let bytes = std::fs::read(&wasm).map_err(|e| e.to_string())?;
    if bytes[..4] != *b"\0asm" {
        return Err(format!("not a wasm file: {:02x?}", &bytes[..4]));
    }
    if bytes.windows(13).any(|w| w == b"wasi_snapshot") {
        return Err("the module imports WASI".to_string());
    }
    let out = Command::new("node")
        .args([
            "tests/wasm/loader.mjs",
            wasm.to_str().unwrap(),
            "add",
            "2",
            "3",
        ])
        .current_dir(root)
        .output()
        .map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    if !out.status.success() || stdout != "5\n" {
        return Err(format!(
            "loader.mjs add 2 3: {}\n{stdout}{}",
            out.status,
            String::from_utf8_lossy(&out.stderr)
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
    let Some(dir) = sanitizer_clang(work, "cli", "thread") else {
        println!(
            "skipped cli/aye build --cflags -fsanitize=thread: clang's ThreadSanitizer does not run here"
        );
        return Ok(());
    };
    let exe = work.join("threads_fanout_tsan");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_aye"));
    cmd.current_dir(root)
        .args(["build", "tests/cases/threads_fanout.aye"])
        .args(["--cflags", "-fsanitize=thread", "-o"])
        .arg(&exe);
    clang_first(&mut cmd, &dir)?;
    let built = cmd.output().map_err(|e| e.to_string())?;
    if !built.status.success() {
        return Err(format!(
            "`aye build --cflags -fsanitize=thread` failed: {}\n{}",
            built.status,
            String::from_utf8_lossy(&built.stderr)
        ));
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

/// The `clang` whose sanitizer `kind` (`address` or `thread`) works here, as
/// the directory to put first on `PATH` (`None` for the clang already on
/// it): a C program with a bug the sanitizer reports (a read of freed
/// memory, or a data race between two threads), built with
/// `-fsanitize=kind`, must die with its report within 30 s (a hang counts
/// as not working). The clang on `PATH` is tried first, then Homebrew's
/// LLVM (Apple's sanitizers hang on some Macs).
fn sanitizer_clang(work: &Path, tag: &str, kind: &str) -> Option<Option<PathBuf>> {
    let src = work.join(format!("{tag}_{kind}_probe.c"));
    let (c, report) = if kind == "thread" {
        (
            "#include <pthread.h>\nint g;\nstatic void *f(void *p) { g = 1; return p; }\n\
             int main(void) { pthread_t t; pthread_create(&t, 0, f, 0); g = 2; pthread_join(t, 0); return 0; }\n",
            "ThreadSanitizer: data race",
        )
    } else {
        (
            "#include <stdlib.h>\nint main(void) { volatile char *p = malloc(16); free((void *)p); return p[0]; }\n",
            "heap-use-after-free",
        )
    };
    std::fs::write(&src, c).ok()?;
    let brew = Command::new("brew")
        .args(["--prefix", "llvm"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| Path::new(String::from_utf8_lossy(&o.stdout).trim()).join("bin"));
    for dir in [None, brew] {
        let exe = work.join(format!("{tag}_{kind}_probe"));
        let clang = dir.as_ref().map_or("clang".into(), |d| d.join("clang"));
        let built = Command::new(clang)
            .arg(format!("-fsanitize={kind}"))
            .args(["-g", "-pthread"])
            .arg(&src)
            .arg("-o")
            .arg(&exe)
            .status();
        if !built.is_ok_and(|s| s.success()) {
            continue;
        }
        let reported = output_within(&mut Command::new(&exe), 30).is_some_and(|o| {
            !o.status.success() && String::from_utf8_lossy(&o.stderr).contains(report)
        });
        if reported {
            return Some(dir);
        }
    }
    None
}

/// Put the directory `sanitizer_clang` found first on `cmd`'s `PATH`.
fn clang_first(cmd: &mut Command, dir: &Option<PathBuf>) -> Result<(), String> {
    if let Some(d) = dir {
        let old = std::env::var_os("PATH").unwrap_or_default();
        let path =
            std::env::join_paths(std::iter::once(d.clone()).chain(std::env::split_paths(&old)))
                .map_err(|e| e.to_string())?;
        cmd.env("PATH", path);
    }
    Ok(())
}

/// P2.9 (gap #3): `tests/asan/X.aye` reads memory that a `List` or `Pool`
/// freed when it grew. Built with `--cflags -fsanitize=address` in debug
/// mode (where the freed block waits in the quarantine) and in safe mode,
/// it must die with AddressSanitizer's report, which contains every line of
/// `X.asan` (a line starting `debug: ` or `safe: ` only counts in that
/// mode), and print nothing. Debug mode reports `use-after-poison`, since
/// the runtime poisons the quarantined block; safe mode has no quarantine,
/// so the block really is freed: `heap-use-after-free`. Skipped where
/// AddressSanitizer does not run.
fn asan_case(p: &Path, work: &Path) -> Result<(), String> {
    let stem = p.file_stem().unwrap().to_string_lossy().into_owned();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let Some(dir) = sanitizer_clang(work, &stem, "address") else {
        println!("skipped asan/{stem}: clang's AddressSanitizer does not run here");
        return Ok(());
    };
    let want = read(&p.with_extension("asan")).ok_or("missing .asan file")?;
    for mode in ["debug", "safe"] {
        let exe = work.join(format!("{stem}_asan_{mode}"));
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_aye"));
        cmd.current_dir(root)
            .args(["build", p.to_str().unwrap(), "--mode", mode])
            .args(["--cflags", "-fsanitize=address", "-o"])
            .arg(&exe);
        clang_first(&mut cmd, &dir)?;
        let built = cmd.output().map_err(|e| e.to_string())?;
        if !built.status.success() {
            return Err(format!(
                "{stem} ({mode}): build failed\n{}",
                String::from_utf8_lossy(&built.stderr)
            ));
        }
        let out = output_within(&mut Command::new(&exe), 120)
            .ok_or(format!("{stem} ({mode}): did not finish within 120 s"))?;
        let (stdout, stderr) = (
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        );
        let prefix = format!("{mode}: ");
        let missing: Vec<&str> = want
            .lines()
            .filter(|l| {
                !l.starts_with("debug: ") && !l.starts_with("safe: ") || l.starts_with(&prefix)
            })
            .map(|l| l.strip_prefix(&prefix).unwrap_or(l))
            .filter(|l| !stderr.contains(l))
            .collect();
        if out.status.success() || !stdout.is_empty() || !missing.is_empty() {
            return Err(format!(
                "{stem} ({mode}): {}, missing {missing:?}\n--- stdout\n{stdout}--- stderr\n{stderr}",
                out.status
            ));
        }
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

/// `aye doc` for an unknown std module lists the std modules and names the
/// close one, as E0081 does for `import std.<x>`.
fn cli_doc_unknown_std() -> Result<(), String> {
    let out = Command::new(env!("CARGO_BIN_EXE_aye"))
        .args(["doc", "--signatures", "std.tex"])
        .output()
        .map_err(|e| e.to_string())?;
    let err = String::from_utf8_lossy(&out.stderr);
    let want = "aye: unknown std module `std.tex`; std has: coro, crypto, diag, encoding, fs, heap, http, json, list, log, map, math, mem, net, os, path, pool, process, sort, sync, testing, text, thread, time\ndid you mean `std.text`?\n";
    if out.status.code() != Some(1) || !out.stdout.is_empty() || err != want {
        return Err(format!(
            "`aye doc --signatures std.tex`: {}\n--- expected stderr\n{want}--- got\n{err}",
            out.status
        ));
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
    let asan_cases = cases("asan");
    let mod_cases = module_dirs();
    let deps_errors: Vec<PathBuf> = {
        let mut v: Vec<PathBuf> =
            std::fs::read_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/deps/errors"))
                .unwrap()
                .map(|e| e.unwrap().path())
                .collect();
        v.sort();
        v
    };
    let wasm = wasm_target();
    let freestanding = freestanding_target();
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
        add(&mut jobs, "tests/-j".to_string(), test_jobs_case);
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
        if let Some(t) = wasm {
            for p in &run_cases {
                let w = &work;
                add(&mut jobs, format!("wasm/cases/{}", name(p)), move || {
                    wasm_job("cases", p, || run_case_for(p, w, t))
                });
            }
            for p in &test_cases {
                add(&mut jobs, format!("wasm/tests/{}", name(p)), move || {
                    wasm_job("tests", p, || {
                        test_case_for(p, &["--target", "wasm32-wasip1"])
                    })
                });
            }
        } else {
            println!("skipped wasm32-wasip1 goldens: no WASI toolchain or no node");
        }
        if let Some(t) = freestanding {
            for p in run_cases
                .iter()
                .filter(|p| p.with_extension("freestanding").exists())
            {
                let w = &work;
                add(
                    &mut jobs,
                    format!("freestanding/cases/{}", name(p)),
                    move || freestanding_case(p, w, t),
                );
            }
        } else {
            println!("skipped wasm32-freestanding goldens: no wasm32 clang or no node");
        }
        add(&mut jobs, "example1".to_string(), example1);
        let w = &work;
        add(&mut jobs, "deps/local".to_string(), move || deps_local(w));
        for p in &deps_errors {
            add(&mut jobs, format!("deps/errors/{}", name(p)), move || {
                deps_error_case(p, w)
            });
        }
        let w = &work;
        add(&mut jobs, "cli/aye run".to_string(), move || cli_run(w));
        add(&mut jobs, "cli/aye build -o".to_string(), move || {
            cli_build_link(w)
        });
        add(
            &mut jobs,
            "cli/aye build --target x86_64-macos".to_string(),
            move || cli_target_x86_64(w),
        );
        add(
            &mut jobs,
            "cli/aye build --target x86_64-linux -c".to_string(),
            move || cli_target_linux_obj(w, "x86_64", 0x3e),
        );
        add(
            &mut jobs,
            "cli/aye build --target aarch64-linux -c".to_string(),
            move || cli_target_linux_obj(w, "aarch64", 0xb7),
        );
        add(
            &mut jobs,
            "cli/aye build --target x86_64-linux: the link error".to_string(),
            move || cli_target_linux_link_error(w),
        );
        add(
            &mut jobs,
            "cli/aye build --target wasm32-wasip1".to_string(),
            move || cli_target_wasm(w),
        );
        add(
            &mut jobs,
            "cli/aye build --target wasm32-freestanding".to_string(),
            move || cli_target_freestanding(w),
        );
        add(&mut jobs, "cli/aye check".to_string(), cli_check);
        add(
            &mut jobs,
            "cli/aye build --cflags -fsanitize=thread".to_string(),
            move || cli_tsan(w),
        );
        for p in &asan_cases {
            let w = &work;
            add(&mut jobs, format!("asan/{}", name(p)), move || {
                asan_case(p, w)
            });
        }
        add(&mut jobs, "cli/aye explain".to_string(), cli_explain);
        add(
            &mut jobs,
            "cli/aye doc: an unknown std module".to_string(),
            cli_doc_unknown_std,
        );
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
        // At most `common::jobs_for(2)` cases run at once.
        let queue = std::sync::Mutex::new(jobs.into_iter().enumerate());
        let done = std::sync::Mutex::new(Vec::new());
        let workers = common::jobs_for(2);
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
    if wasm.is_some() {
        let total = results
            .iter()
            .filter(|(n, _)| n.starts_with("wasm/"))
            .count();
        let bad = results
            .iter()
            .filter(|(n, r)| n.starts_with("wasm/") && r.is_err())
            .count();
        let skipped = WASM_SKIPPED.load(Ordering::Relaxed);
        println!(
            "golden (wasm32-wasip1): {} passed; {skipped} skipped; {bad} failed",
            total - bad - skipped
        );
    }
    if freestanding.is_some() {
        let total = results
            .iter()
            .filter(|(n, _)| n.starts_with("freestanding/"))
            .count();
        let bad = results
            .iter()
            .filter(|(n, r)| n.starts_with("freestanding/") && r.is_err())
            .count();
        println!(
            "golden (wasm32-freestanding): {} passed; {bad} failed",
            total - bad
        );
    }
    if failed > 0 {
        std::process::exit(1);
    }
}
