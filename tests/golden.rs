//! Golden tests.
//!
//! - `tests/cases/X.ore`: compiled in debug, safe and fast mode, run, and
//!   stdout compared with `X.out`. If `X.panic` exists the program must abort
//!   with exactly that on stderr (checked in debug and safe mode only). If
//!   `X.stderr` exists, `main` must return an error: exit status 1, and
//!   stderr equal to `X.stderr` in debug mode; safe and fast mode print no
//!   error return trace, so there stderr must be the first line only.
//! - `tests/ffi/X.ore`: like `tests/cases`, for programs that link C
//!   libraries with `@link`; skipped when `pkg-config` does not know a
//!   package listed in `X.needs`. Programs run with `SDL_VIDEODRIVER=dummy`,
//!   so SDL2 needs no display.
//! - `tests/errors/X.ore`: must fail to compile with exactly the diagnostics
//!   in `X.err`, one per line, in order.
//! - `tests/tests/X.ore`: `ore test X.ore` (debug mode) must print exactly
//!   `X.out`, and `ore test --json X.ore` exactly `X.json` if it exists. The
//!   exit status must be 0 when no test failed, and 1 otherwise.
//! - `tests/fix/X.ore`: `ore fix X.ore` on a copy must leave exactly
//!   `X.fixed` and print exactly `X.out`. If `X.stderr` exists, errors
//!   remain: stderr must equal it and the exit status must be 1; otherwise
//!   stderr must be empty, the exit status 0, and `ore check` must accept
//!   the fixed file. If `X.json` exists, `ore check --json` on the original
//!   must print exactly that.
//! - `tests/modules/X/`: a directory module. In `tests/modules`, the binary
//!   runs `ore run X` (or the arguments in `X.args`, with `X.stdin` as
//!   stdin): stdout must equal `X.out`, stderr `X.stderr` (default empty)
//!   and the exit status `X.status` (default 0). If `X.err` exists, `ore
//!   check X` must fail with exactly that on stderr instead.
//! - `tests/example1/`: Example 1 of the design doc, the directory module
//!   `app`, run in `ok`, `missing` and `invalid` (each with its own
//!   `app.toml`, or none); `ore test app` runs its tests.
//! - `ore run`, `ore build -o`, `ore check`, `ore test --filter` and `ore
//!   test --module` are also run through the built binary.
//! - `ore build --cflags -fsanitize=thread`: the M10 fan-out program runs
//!   under ThreadSanitizer without a report (skipped where ThreadSanitizer
//!   does not run).

use ore::diag::Diag;
use ore::driver::{self, Mode};
use std::path::{Path, PathBuf};
use std::process::Command;

fn read(p: &Path) -> Option<String> {
    std::fs::read_to_string(p).ok()
}

/// The packages in `X.needs` that `pkg-config` does not know.
fn missing_packages(path: &Path) -> Vec<String> {
    let needs = read(&path.with_extension("needs")).unwrap_or_default();
    let known = |p: &str| Command::new("pkg-config").args(["--exists", p]).status().is_ok_and(|s| s.success());
    needs.split_whitespace().filter(|p| !known(p)).map(String::from).collect()
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
        let out = Command::new(&exe).env("SDL_VIDEODRIVER", "dummy").output().map_err(|e| e.to_string())?;
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

fn ffi_case(path: &Path, work: &Path) -> Result<(), String> {
    let missing = missing_packages(path);
    if !missing.is_empty() {
        println!("skipped ffi/{}: pkg-config does not know {}", name(path), missing.join(", "));
        return Ok(());
    }
    run_case(path, work)
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

/// Run the `ore` binary in `dir` and return (stdout, stderr, exit status).
fn ore(dir: &Path, args: &[&str]) -> Result<(String, String, Option<i32>), String> {
    let out = Command::new(env!("CARGO_BIN_EXE_ore")).current_dir(dir).args(args).output().map_err(|e| e.to_string())?;
    Ok((
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    ))
}

fn fix_case(path: &Path, work: &Path) -> Result<(), String> {
    let file = name(path);
    let dir = work.join(format!("fix-{}", path.file_stem().unwrap().to_string_lossy()));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::copy(path, dir.join(&file)).map_err(|e| e.to_string())?;
    let want_file = read(&path.with_extension("fixed")).ok_or("missing .fixed file")?;
    let want_out = read(&path.with_extension("out")).ok_or("missing .out file")?;
    let want_err = read(&path.with_extension("stderr"));
    let (out, err, code) = ore(&dir, &["fix", &file])?;
    let got_file = read(&dir.join(&file)).unwrap();
    if got_file != want_file {
        return Err(format!("fixed file mismatch\n--- expected\n{want_file}--- got\n{got_file}"));
    }
    if out != want_out {
        return Err(format!("`ore fix` stdout mismatch\n--- expected\n{want_out}--- got\n{out}"));
    }
    if err != want_err.clone().unwrap_or_default() {
        return Err(format!("`ore fix` stderr mismatch\n--- got\n{err}"));
    }
    let want_code = if want_err.is_some() { 1 } else { 0 };
    if code != Some(want_code) {
        return Err(format!("`ore fix`: expected exit status {want_code}, got {code:?}"));
    }
    if want_err.is_none() {
        let (_, err, code) = ore(&dir, &["check", &file])?;
        if code != Some(0) {
            return Err(format!("`ore check` rejects the fixed file:\n{err}"));
        }
    }
    if let Some(want) = read(&path.with_extension("json")) {
        let rel = format!("tests/fix/{file}");
        let (out, _, _) = ore(Path::new(env!("CARGO_MANIFEST_DIR")), &["check", "--json", &rel])?;
        if out != want {
            return Err(format!("`ore check --json` mismatch\n--- expected\n{want}--- got\n{out}"));
        }
    }
    Ok(())
}

/// Run the `ore` binary in `dir` with `stdin`, and return (stdout, stderr,
/// exit status).
fn ore_stdin(dir: &Path, args: &[&str], stdin: &str) -> Result<(String, String, Option<i32>), String> {
    use std::io::Write;
    let mut child = Command::new(env!("CARGO_BIN_EXE_ore"))
        .current_dir(dir)
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    child.stdin.take().unwrap().write_all(stdin.as_bytes()).map_err(|e| e.to_string())?;
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
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).filter(|p| p.is_dir()).collect();
    v.sort();
    v
}

fn module_case(path: &Path) -> Result<(), String> {
    let dir = path.parent().unwrap();
    let x = name(path);
    let with = |ext: &str| read(&dir.join(format!("{x}.{ext}")));
    if let Some(want) = with("err") {
        let (_, err, code) = ore(dir, &["check", &x])?;
        if err != want || code != Some(1) {
            return Err(format!("`ore check {x}`: {code:?}\n--- expected\n{want}--- got\n{err}"));
        }
        return Ok(());
    }
    let args = with("args").unwrap_or_else(|| format!("run {x}"));
    let args: Vec<&str> = args.split_whitespace().collect();
    let (out, err, code) = ore_stdin(dir, &args, &with("stdin").unwrap_or_default())?;
    let want_out = with("out").ok_or("missing .out file")?;
    let want_err = with("stderr").unwrap_or_default();
    let want_code: i32 = with("status").map_or(0, |s| s.trim().parse().unwrap());
    if out != want_out || err != want_err || code != Some(want_code) {
        return Err(format!(
            "`ore {}`: {code:?} (expected {want_code})\n--- expected stdout\n{want_out}--- got\n{out}--- expected stderr\n{want_err}--- got\n{err}",
            args.join(" ")
        ));
    }
    Ok(())
}

/// Example 1 of the design doc: `readConfig` reads `app.toml` from the
/// working directory. Each of `ok`, `missing` and `invalid` is a working
/// directory with its expected stderr in `X.stderr` (default empty); stdout
/// is always empty. `ore test app` runs the tests in `app/config.ore`.
fn example1() -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/example1");
    for case in ["ok", "missing", "invalid"] {
        let want_err = read(&root.join(format!("{case}.stderr"))).unwrap_or_default();
        let want_code = if want_err.is_empty() { 0 } else { 1 };
        let (out, err, code) = ore(&root.join(case), &["run", "../app"])?;
        if !out.is_empty() || err != want_err || code != Some(want_code) {
            return Err(format!("[{case}] `ore run ../app`: {code:?}\n--- stdout\n{out}--- expected stderr\n{want_err}--- got\n{err}"));
        }
    }
    let want = read(&root.join("app.test.out")).ok_or("missing app.test.out")?;
    let (out, err, code) = ore(&root, &["test", "app"])?;
    if out != want || !err.is_empty() || code != Some(0) {
        return Err(format!("`ore test app`: {code:?}\n--- expected\n{want}--- got\n{out}{err}"));
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

/// `ore test DIR --module PATH` runs the tests of one module of the
/// program; a module that imports a sibling, given on its own, gets a hint.
fn cli_test_module() -> Result<(), String> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/modules");
    let runs: &[(&[&str], &str, &str, i32)] = &[
        (&["test", "modtests", "--module", "dom"], "test \"dom: basic\" ... ok\n\n1 passed; 0 failed\n", "", 0),
        (&["test", "modtests", "--module", "nope"], "", "ore: no tests in module `nope` (modules with tests: `html`, `dom`)\n", 1),
        (
            &["test", "modtests/html"],
            "",
            "modtests/html/html.ore:1:8: error[E0081]: cannot find module `dom` (no directory `modtests/html/dom`; imports are \
             found from the directory given to `ore`, and `modtests/dom` exists: give `modtests` instead, with `--module html` for `ore test`)\n",
            1,
        ),
    ];
    for (args, want_out, want_err, want_code) in runs {
        let (out, err, code) = ore(&dir, args)?;
        if out != *want_out || err != *want_err || code != Some(*want_code) {
            return Err(format!("`ore {}`: {code:?}\n--- stdout\n{out}--- stderr\n{err}", args.join(" ")));
        }
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

/// `ore build -o PATH` links the libraries of `@link`, and the program
/// runs (from the crate root, where it finds its data file).
fn cli_build_link(work: &Path) -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = root.join("tests/ffi/curl_file.ore");
    if !missing_packages(&src).is_empty() {
        println!("skipped cli/ore build -o: pkg-config does not know libcurl");
        return Ok(());
    }
    let exe = work.join("curl_file_cli");
    let (_, err, code) = ore(root, &["build", "tests/ffi/curl_file.ore", "-o", exe.to_str().unwrap()])?;
    if code != Some(0) {
        return Err(format!("`ore build -o` failed: {code:?}\n{err}"));
    }
    let out = Command::new(&exe).current_dir(root).output().map_err(|e| e.to_string())?;
    let want = read(&src.with_extension("out")).unwrap();
    if !out.status.success() || String::from_utf8_lossy(&out.stdout) != want {
        return Err(format!("the program of `ore build -o` printed:\n{}", String::from_utf8_lossy(&out.stdout)));
    }
    Ok(())
}

/// Run `cmd` and wait at most `secs` seconds for it (`None` on a timeout:
/// it is killed).
fn output_within(cmd: &mut Command, secs: u64) -> Option<std::process::Output> {
    use std::process::Stdio;
    let mut child = cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().ok()?;
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
    let built = Command::new("clang").args(["-fsanitize=thread", "-pthread"]).arg(&src).arg("-o").arg(&exe).status();
    if !built.is_ok_and(|s| s.success()) {
        return false;
    }
    output_within(&mut Command::new(&exe), 30).is_some_and(|o| o.status.success() && o.stderr.is_empty())
}

/// M10: the fan-out program built with `--cflags -fsanitize=thread` runs
/// without a ThreadSanitizer report and prints what it prints without it.
fn cli_tsan(work: &Path) -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    // The flags reach clang: one it does not know fails the build.
    let bad = work.join("hello_bad_flag");
    let (_, err, code) = ore(root, &["build", "tests/cases/hello.ore", "--cflags", "-fno-such-ore-flag", "-o", bad.to_str().unwrap()])?;
    if code != Some(1) || !err.contains("clang failed") {
        return Err(format!("`ore build --cflags -fno-such-ore-flag`: {code:?}\n{err}"));
    }
    if !tsan_works(work) {
        println!("skipped cli/ore build --cflags -fsanitize=thread: clang's ThreadSanitizer does not run here");
        return Ok(());
    }
    let exe = work.join("threads_fanout_tsan");
    let args = ["build", "tests/cases/threads_fanout.ore", "--cflags", "-fsanitize=thread", "-o", exe.to_str().unwrap()];
    let (_, err, code) = ore(root, &args)?;
    if code != Some(0) {
        return Err(format!("`ore {}` failed: {code:?}\n{err}", args.join(" ")));
    }
    let out = output_within(&mut Command::new(&exe), 120).ok_or("the program did not finish within 120 s")?;
    let want = read(&root.join("tests/cases/threads_fanout.out")).unwrap();
    let (stdout, stderr) = (String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    if !out.status.success() || stdout != want || !stderr.is_empty() {
        return Err(format!("under ThreadSanitizer: {}\n--- stdout\n{stdout}--- stderr\n{stderr}", out.status));
    }
    Ok(())
}

/// The `ore check` CLI: exit code, stdout and stderr, text and `--json`.
fn cli_check() -> Result<(), String> {
    let ok = "tests/cases/hello.ore";
    let bad = "tests/errors/unknown_escape.ore";
    let three = "tests/errors/three_errors.ore";
    let fixable = "tests/fix/let_var.ore";
    let diag = |code: &str, line: u32, col: u32, msg: &str| {
        format!(r#"{{"severity":"error","code":"{code}","file":"{three}","line":{line},"col":{col},"message":"{msg}","fixes":[]}}"#)
    };
    let checks: &[(&[&str], bool, String, String)] = &[
        (&[ok], true, String::new(), String::new()),
        (&[bad], false, String::new(), format!("{bad}:2:16: error[E0005]: unknown escape sequence `\\q`\n")),
        (&[ok, "--json"], true, format!("{{\"file\":\"{ok}\",\"diagnostics\":[]}}\n"), String::new()),
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
                diag("E0023", 4, 12, "mismatched types: expected `i64`, found `i32`"),
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
    let fix_cases = cases("fix");
    let ffi_cases = cases("ffi");
    let mod_cases = module_dirs();
    let results: Vec<(String, Result<(), String>)> = std::thread::scope(|s| {
        let mut handles = Vec::new();
        for p in &run_cases {
            let w = &work;
            handles.push((format!("cases/{}", name(p)), s.spawn(move || run_case(p, w))));
        }
        for p in &ffi_cases {
            let w = &work;
            handles.push((format!("ffi/{}", name(p)), s.spawn(move || ffi_case(p, w))));
        }
        for p in &err_cases {
            handles.push((format!("errors/{}", name(p)), s.spawn(move || error_case(p))));
        }
        for p in &test_cases {
            handles.push((format!("tests/{}", name(p)), s.spawn(move || test_case(p))));
        }
        for p in &fix_cases {
            let w = &work;
            handles.push((format!("fix/{}", name(p)), s.spawn(move || fix_case(p, w))));
        }
        for p in &mod_cases {
            handles.push((format!("modules/{}", name(p)), s.spawn(move || module_case(p))));
        }
        handles.push(("example1".to_string(), s.spawn(example1)));
        let w = &work;
        handles.push(("cli/ore run".to_string(), s.spawn(move || cli_run(w))));
        handles.push(("cli/ore build -o".to_string(), s.spawn(move || cli_build_link(w))));
        handles.push(("cli/ore check".to_string(), s.spawn(cli_check)));
        handles.push(("cli/ore build --cflags -fsanitize=thread".to_string(), s.spawn(move || cli_tsan(w))));
        handles.push(("cli/ore explain".to_string(), s.spawn(cli_explain)));
        handles.push(("cli/ore test --filter".to_string(), s.spawn(cli_test_filter)));
        handles.push(("cli/ore test --module".to_string(), s.spawn(cli_test_module)));
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
