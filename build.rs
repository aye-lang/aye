//! Embeds the git commit of the build for `aye --version`: `AYE_GIT_SHA`
//! (short hash) and `AYE_GIT_DATE` (commit date, YYYY-MM-DD), both empty
//! outside a git checkout (a crates.io or tarball build) or without `git`.

use std::path::Path;
use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?.trim().to_string();
    (!s.is_empty()).then_some(s)
}

fn main() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    // Only this repository's own checkout counts: a source tarball unpacked
    // inside some other git repository must not take that repository's hash.
    let top = git(&["-C", &manifest, "rev-parse", "--show-toplevel"]);
    let ours = top.is_some_and(|t| {
        Path::new(&t).canonicalize().ok() == Path::new(&manifest).canonicalize().ok()
    });
    let (sha, date) = if ours {
        (
            git(&["-C", &manifest, "rev-parse", "--short=7", "HEAD"]),
            git(&["-C", &manifest, "log", "-1", "--format=%cs", "HEAD"]),
        )
    } else {
        (None, None)
    };
    println!(
        "cargo:rustc-env=AYE_GIT_SHA={}",
        sha.as_deref().unwrap_or("")
    );
    println!(
        "cargo:rustc-env=AYE_GIT_DATE={}",
        date.as_deref().unwrap_or("")
    );
    // Rebuild when HEAD moves (a checkout, a commit on the current branch).
    if ours {
        for p in ["HEAD", "logs/HEAD"] {
            if let Some(f) = git(&["-C", &manifest, "rev-parse", "--git-path", p]) {
                let f = Path::new(&manifest).join(f);
                println!("cargo:rerun-if-changed={}", f.display());
            }
        }
    }
    println!("cargo:rerun-if-changed=build.rs");
}
