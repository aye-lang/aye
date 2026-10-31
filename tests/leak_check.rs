//! `--leak-check` (TC3h) turns on the leak, double-free and use-after-free
//! checks of `General` in any mode. `tests/leak_check/X.aye` is built with
//! `aye build` and run; with tracking its stderr contains `X.err`.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Build `tests/leak_check/<stem>.aye` with `args` and run it.
fn build_and_run(stem: &str, args: &[&str]) -> Output {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let exe: PathBuf = std::env::temp_dir().join(format!(
        "aye-leak-check-{}-{stem}-{}",
        std::process::id(),
        args.join("").replace('-', "")
    ));
    let built = Command::new(env!("CARGO_BIN_EXE_aye"))
        .current_dir(root)
        .arg("build")
        .arg(format!("tests/leak_check/{stem}.aye"))
        .args(args)
        .arg("-o")
        .arg(&exe)
        .output()
        .unwrap();
    assert!(
        built.status.success() && built.stderr.is_empty(),
        "aye build {stem} {args:?}: {}",
        String::from_utf8_lossy(&built.stderr)
    );
    let out = Command::new(&exe).output().unwrap();
    let _ = std::fs::remove_file(&exe);
    let _ = std::fs::remove_dir_all(exe.with_extension("dSYM"));
    out
}

fn golden(stem: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tests/leak_check/{stem}.err"));
    std::fs::read_to_string(p).unwrap()
}

#[test]
fn leak_check_safe_reports_leak() {
    let out = build_and_run("leak", &["--mode", "safe", "--leak-check"]);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "7\n");
    assert_eq!(String::from_utf8_lossy(&out.stderr), golden("leak"));
}

#[test]
fn safe_without_leak_check_prints_nothing() {
    let out = build_and_run("leak", &["--mode", "safe"]);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "7\n");
    assert_eq!(String::from_utf8_lossy(&out.stderr), "");
}

#[test]
fn leak_check_fast_double_free_panics() {
    let out = build_and_run("double_free", &["--mode", "fast", "--leak-check"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "{out:?}");
    assert!(stderr.contains(&golden("double_free")), "{stderr}");
}
