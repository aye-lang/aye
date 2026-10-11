//! `aye fmt` and `aye doc`.
//!
//! - `tests/fmt/X.in` formats to exactly `X.out`, and `X.out` is already
//!   formatted.
//! - Every `.aye` file under `tests/` and `std/` that `aye fmt` accepts
//!   formats idempotently, and `aye check` gives the formatted file the
//!   same diagnostics (code, message, and line after the formatter's line
//!   map).
//! - The `aye fmt --check` and `aye doc --signatures` commands.

use aye::diag::Diag;
use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn aye_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            aye_files(&p, out);
        } else if p.extension().is_some_and(|e| e == "aye") {
            out.push(p);
        }
    }
}

#[test]
fn golden_fmt() {
    let dir = root().join("tests/fmt");
    let mut n = 0;
    let mut entries: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for input in entries
        .iter()
        .filter(|p| p.extension().is_some_and(|e| e == "in"))
    {
        let src = std::fs::read_to_string(input).unwrap();
        let want = std::fs::read_to_string(input.with_extension("out")).unwrap();
        let got = aye::fmt::format(&src).unwrap_or_else(|e| panic!("{}: {e:?}", input.display()));
        assert_eq!(got, want, "{}", input.display());
        assert_eq!(
            aye::fmt::format(&want).unwrap(),
            want,
            "{}.out is not formatted",
            input.display()
        );
        n += 1;
    }
    assert!(n >= 5, "only {n} fmt cases");
}

/// (code, line, message) of the diagnostics in `file`, lines through `map`.
fn keys(diags: &[Diag], file: &str, map: Option<&[u32]>) -> Vec<(String, u32, String)> {
    diags
        .iter()
        .filter(|d| d.file == file)
        .map(|d| {
            let line = map.map_or(d.span.line, |m| m[d.span.line as usize - 1]);
            (d.code.to_string(), line, d.msg.clone())
        })
        .collect()
}

#[test]
fn idempotent_and_same_diagnostics() {
    let mut files = Vec::new();
    aye_files(&root().join("tests"), &mut files);
    aye_files(&root().join("std"), &mut files);
    let (mut formatted, mut refused) = (0, Vec::new());
    for path in &files {
        let name = path.to_string_lossy().into_owned();
        let src = std::fs::read_to_string(path).unwrap();
        let Ok((once, map)) = aye::fmt::format_mapped(&src) else {
            refused.push(name);
            continue;
        };
        formatted += 1;
        assert_eq!(
            aye::fmt::format(&once).unwrap(),
            once,
            "{name}: not idempotent"
        );
        let before = aye::driver::check(&name, &src).err().unwrap_or_default();
        let after = aye::driver::check(&name, &once).err().unwrap_or_default();
        assert_eq!(
            keys(&before, &name, Some(&map)),
            keys(&after, &name, None),
            "{name}: diagnostics differ"
        );
        assert_eq!(
            before.len(),
            after.len(),
            "{name}: diagnostics in other files differ"
        );
    }
    assert!(formatted >= 150, "only {formatted} files formatted");
    // Files that do not lex or whose brackets do not match.
    assert!(
        refused.iter().all(|f| {
            f.contains("/tests/errors/")
                || f.contains("/tests/modules/single/")
                || f.ends_with("/tests/fix/semicolons.aye")
        }),
        "{refused:?}"
    );
}

fn aye(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_aye"))
        .args(args)
        .output()
        .unwrap();
    let s = |b: &[u8]| String::from_utf8_lossy(b).into_owned();
    (
        out.status.code().unwrap_or(-1),
        s(&out.stdout),
        s(&out.stderr),
    )
}

#[test]
fn fmt_command() {
    let dir = std::env::temp_dir().join(format!("aye-fmt-{}", std::process::id()));
    let sub = dir.join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    let messy = sub.join("a.aye");
    let clean = dir.join("b.aye");
    std::fs::write(&messy, "pub fn main() {\n@println(1+2)\n}\n").unwrap();
    std::fs::write(&clean, "pub fn main() {\n    @println(1)\n}\n").unwrap();
    let d = dir.to_str().unwrap();
    let (code, out, _) = aye(&["fmt", "--check", d]);
    assert_eq!((code, out.trim()), (1, messy.to_str().unwrap()));
    assert_eq!(
        std::fs::read_to_string(&messy).unwrap(),
        "pub fn main() {\n@println(1+2)\n}\n"
    );
    let (code, out, _) = aye(&["fmt", d]);
    assert_eq!((code, out.as_str()), (0, ""));
    assert_eq!(
        std::fs::read_to_string(&messy).unwrap(),
        "pub fn main() {\n    @println(1 + 2)\n}\n"
    );
    assert_eq!(aye(&["fmt", "--check", d]).0, 0);
    // A file that does not lex is reported and left alone.
    std::fs::write(&clean, "let x = 1;\n").unwrap();
    let (code, _, err) = aye(&["fmt", clean.to_str().unwrap()]);
    assert_eq!(code, 1);
    assert!(err.contains("b.aye:1:10: error[E0002]"), "{err}");
    assert_eq!(std::fs::read_to_string(&clean).unwrap(), "let x = 1;\n");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn doc_command() {
    let (code, out, err) = aye(&["doc", "--signatures", "std"]);
    assert_eq!(code, 0, "{err}");
    for m in ["std.fs", "std.mem", "std.text", "std.time"] {
        assert!(
            out.contains(&format!("// module {m}\n")),
            "{m} missing:\n{out}"
        );
    }
    assert!(out.contains("pub fn open("), "{out}");
    assert!(
        !out.contains("    let ") && !out.contains("    if "),
        "a body leaked:\n{out}"
    );
    let (code, out, _) = aye(&[
        "doc",
        "--signatures",
        root().join("tests/modules/multi").to_str().unwrap(),
    ]);
    assert_eq!(code, 0);
    assert!(
        out.contains(
            "\n// module shapes\n\npub struct Counter {\n    n: i64\n    pub fn init() Counter\n"
        ),
        "{out}"
    );
    let (code, _, err) = aye(&["doc", "--signatures", "std.nope"]);
    assert_eq!(code, 1);
    assert!(err.contains("unknown std module"), "{err}");
}

#[test]
fn help_prints_usage_and_exits_0() {
    for args in [
        &["--help"][..],
        &["-h"],
        &["help"],
        &["fmt", "--help"],
        &["build", "-h"],
    ] {
        let (code, out, _) = aye(args);
        assert_eq!(code, 0, "{args:?}");
        assert!(out.contains("usage"), "{args:?}: {out}");
    }
}

/// Tracked `.aye` files that `aye fmt --check` must not be asked about.
const FMT_EXCLUDED: &[(&str, &str)] = &[
    ("tests/errors/line_strings.aye", "does not lex on purpose"),
    (
        "tests/errors/syntax_recovery.aye",
        "does not parse on purpose",
    ),
    ("tests/errors/unknown_escape.aye", "does not lex on purpose"),
    ("tests/fix/semicolons.aye", "has `;` terminators on purpose"),
    (
        "tests/modules/single/broken.aye",
        "brackets do not match on purpose",
    ),
];

#[test]
fn repo_is_formatted() {
    let out = Command::new("git")
        .args(["ls-files", "*.aye"])
        .current_dir(root())
        .output()
        .unwrap();
    assert!(out.status.success(), "git ls-files failed");
    let mut unformatted = Vec::new();
    let mut n = 0;
    for f in String::from_utf8(out.stdout).unwrap().lines() {
        if FMT_EXCLUDED.iter().any(|(x, _)| *x == f) {
            continue;
        }
        n += 1;
        let (code, _, err) = aye(&["fmt", "--check", root().join(f).to_str().unwrap()]);
        if code != 0 {
            unformatted.push(format!("{f} {err}"));
        }
    }
    assert!(n >= 400, "only {n} files checked");
    assert!(
        unformatted.is_empty(),
        "run `aye fmt` on:\n{}",
        unformatted.join("\n")
    );
}

#[test]
fn rust_is_formatted() {
    let out = match Command::new("cargo")
        .args(["fmt", "--check"])
        .current_dir(root())
        .output()
    {
        Ok(o) => o,
        Err(e) => return eprintln!("skipped: cargo fmt not runnable: {e}"),
    };
    let err = String::from_utf8_lossy(&out.stderr);
    if err.contains("rustfmt") && err.contains("not installed") {
        return eprintln!("skipped: rustfmt is not installed");
    }
    let hunks = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| l.starts_with("Diff in "))
        .count();
    assert!(
        out.status.success(),
        "cargo fmt --check: {hunks} unformatted hunks; run `cargo fmt`\n{err}"
    );
}
