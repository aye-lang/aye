//! Golden tests.
//!
//! - `tests/cases/X.ore`: compiled in debug, safe and fast mode, run, and
//!   stdout compared with `X.out`. If `X.panic` exists the program must abort
//!   with exactly that on stderr (checked in debug and safe mode only).
//! - `tests/errors/X.ore`: must fail to compile with the message in `X.err`.
//! - `ore run` and `ore check` are also run through the built binary.

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

fn name(p: &Path) -> String {
    p.file_name().unwrap().to_string_lossy().into_owned()
}

fn run_case(path: &Path, work: &Path) -> Result<(), String> {
    let src = read(path).unwrap();
    let expected_out = read(&path.with_extension("out")).ok_or("missing .out file")?;
    let expected_panic = read(&path.with_extension("panic"));
    let modes: &[Mode] = if expected_panic.is_some() {
        &[Mode::Debug, Mode::Safe]
    } else {
        &[Mode::Debug, Mode::Safe, Mode::Fast]
    };
    for &mode in modes {
        let c = driver::compile_to_c(&name(path), &src, mode).map_err(|d| d.to_string())?;
        let exe = work.join(format!("{}-{mode:?}", path.file_stem().unwrap().to_string_lossy()));
        driver::cc(&c, &exe, mode)?;
        let out = Command::new(&exe).output().map_err(|e| e.to_string())?;
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        if stdout != expected_out {
            return Err(format!("[{mode:?}] stdout mismatch\n--- expected\n{expected_out}--- got\n{stdout}"));
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
        Err(d) if d.to_string() == expected.trim_end() => Ok(()),
        Err(d) => Err(format!("error mismatch\n--- expected\n{}\n--- got\n{d}", expected.trim_end())),
    }
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
    let checks: &[(&[&str], bool, String, String)] = &[
        (&[ok], true, String::new(), String::new()),
        (&[bad], false, String::new(), format!("{bad}:2:16: error: unknown escape sequence `\\q`\n")),
        (&[ok, "--json"], true, format!("{{\"file\":\"{ok}\",\"diagnostics\":[]}}\n"), String::new()),
        (
            &[bad, "--json"],
            false,
            format!(
                "{{\"file\":\"{bad}\",\"diagnostics\":[{{\"severity\":\"error\",\"line\":2,\"col\":16,\
                 \"message\":\"unknown escape sequence `\\\\q`\"}}]}}\n"
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

fn main() {
    let work = std::env::temp_dir().join(format!("ore-golden-{}", std::process::id()));
    std::fs::create_dir_all(&work).unwrap();

    let run_cases = cases("cases");
    let err_cases = cases("errors");
    let results: Vec<(String, Result<(), String>)> = std::thread::scope(|s| {
        let mut handles = Vec::new();
        for p in &run_cases {
            let w = &work;
            handles.push((format!("cases/{}", name(p)), s.spawn(move || run_case(p, w))));
        }
        for p in &err_cases {
            handles.push((format!("errors/{}", name(p)), s.spawn(move || error_case(p))));
        }
        let w = &work;
        handles.push(("cli/ore run".to_string(), s.spawn(move || cli_run(w))));
        handles.push(("cli/ore check".to_string(), s.spawn(cli_check)));
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
