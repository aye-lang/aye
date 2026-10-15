//! `aye --version` and `aye version` print `aye <Cargo.toml version>`, then
//! ` (<short commit hash> <commit date>)` when the build had a git checkout
//! (build.rs); the release workflow and the install script parse it.

use std::process::Command;

fn version(arg: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_aye"))
        .arg(arg)
        .output()
        .unwrap();
    assert!(out.status.success(), "aye {arg}: {out:?}");
    String::from_utf8(out.stdout).unwrap()
}

fn check(line: &str) {
    let want = format!("aye {}", env!("CARGO_PKG_VERSION"));
    let rest = line
        .strip_prefix(&want)
        .unwrap_or_else(|| panic!("{line:?} does not start with {want:?}"));
    if rest.is_empty() {
        return;
    }
    let inner = rest
        .strip_prefix(" (")
        .and_then(|r| r.strip_suffix(')'))
        .unwrap_or_else(|| panic!("{line:?}: expected ` (<hash> <date>)` after the version"));
    let (sha, date) = inner.split_once(' ').unwrap_or_else(|| panic!("{line:?}"));
    assert!(
        sha.len() >= 7 && sha.bytes().all(|b| b.is_ascii_hexdigit()),
        "{line:?}: bad hash {sha:?}"
    );
    let d: Vec<&str> = date.split('-').collect();
    assert!(
        d.len() == 3
            && [4, 2, 2].iter().zip(&d).all(|(n, p)| p.len() == *n)
            && d.iter().all(|p| p.bytes().all(|b| b.is_ascii_digit())),
        "{line:?}: bad date {date:?}"
    );
}

#[test]
fn version_format() {
    let a = version("--version");
    assert_eq!(a, version("version"));
    assert_eq!(a, version("-V"));
    assert!(a.ends_with('\n') && a.lines().count() == 1, "{a:?}");
    check(a.trim_end());
}

/// In this repository's checkout the hash and date are there.
#[test]
fn version_has_commit_in_checkout() {
    let root = env!("CARGO_MANIFEST_DIR");
    let head = Command::new("git")
        .args(["-C", root, "rev-parse", "--short=7", "HEAD"])
        .output();
    let Ok(head) = head else { return };
    if !head.status.success() {
        return;
    }
    let a = version("--version");
    assert!(
        a.contains(" ("),
        "built in a git checkout but no commit: {a:?}"
    );
}
