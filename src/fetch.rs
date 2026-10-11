//! URL dependencies: download with `curl`, hash, unpack with `tar`, and keep
//! the tree in the cache (`docs/decisions.md`, "Dependencies (O4)").
//!
//! The cache is `<cache>/sha256/<hex>/`, the dependency's top directory. A
//! download and its unpacking happen in `<cache>/tmp/` and the tree is
//! renamed into place after its hash is known, so a directory that exists is
//! complete.

use crate::sha256;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

/// Why a dependency is not had.
pub enum Fail {
    /// The download or the unpacking failed (E0121).
    Unavailable(String),
    /// The archive hashes to `got`, not to the locked hash (E0122).
    Mismatch { got: String },
}

/// A dependency in the cache.
pub struct Got {
    /// The dependency's top directory.
    pub dir: PathBuf,
    /// The SHA-256 of the archive.
    pub sha256: String,
}

/// The cache root: `$AYE_CACHE`, else `$XDG_CACHE_HOME/aye`, else
/// `$HOME/.cache/aye`.
pub fn cache_root_from(
    aye: Option<&str>,
    xdg: Option<&str>,
    home: Option<&str>,
) -> Option<PathBuf> {
    let set = |v: Option<&str>| v.filter(|v| !v.is_empty()).map(PathBuf::from);
    set(aye)
        .or_else(|| set(xdg).map(|p| p.join("aye")))
        .or_else(|| set(home).map(|p| p.join(".cache").join("aye")))
}

/// The cache root from the environment.
pub fn cache_root() -> Option<PathBuf> {
    let var = |k: &str| std::env::var(k).ok();
    cache_root_from(
        var("AYE_CACHE").as_deref(),
        var("XDG_CACHE_HOME").as_deref(),
        var("HOME").as_deref(),
    )
}

/// The directory of an archive with this hash.
pub fn cached_dir(cache: &Path, sha256: &str) -> PathBuf {
    cache.join("sha256").join(sha256)
}

/// A name no other call or process uses at once.
fn unique(what: &str) -> String {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    format!(
        "{what}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

fn run(cmd: &mut Command, what: &str) -> Result<(), String> {
    let out = cmd
        .output()
        .map_err(|e| format!("cannot run `{what}`: {e}"))?;
    if out.status.success() {
        return Ok(());
    }
    let err = String::from_utf8_lossy(&out.stderr);
    let err = err.trim();
    let status = out
        .status
        .code()
        .map_or("a signal".to_string(), |c| c.to_string());
    Err(if err.is_empty() {
        format!("`{what}` failed ({status})")
    } else {
        format!("`{what}` failed ({status}): {err}")
    })
}

/// Download `url`, unpack it, and move the tree into the cache. With
/// `locked`, a locked and cached archive is used without a download, and a
/// downloaded one must hash to it. Nothing enters the cache on a mismatch.
pub fn fetch(cache: &Path, url: &str, locked: Option<&str>) -> Result<Got, Fail> {
    if let Some(hex) = locked {
        let dir = cached_dir(cache, hex);
        if dir.is_dir() {
            return Ok(Got {
                dir,
                sha256: hex.to_string(),
            });
        }
    }
    let tmp = cache.join("tmp");
    let unavailable =
        |msg: String| Fail::Unavailable(format!("{msg} (cache directory {})", cache.display()));
    std::fs::create_dir_all(&tmp)
        .map_err(|e| unavailable(format!("cannot create {}: {e}", tmp.display())))?;
    let archive = tmp.join(unique("download"));
    let unpacked = tmp.join(unique("unpack"));
    let r = download(cache, url, locked, &archive, &unpacked);
    let _ = std::fs::remove_file(&archive);
    let _ = std::fs::remove_dir_all(&unpacked);
    r
}

fn download(
    cache: &Path,
    url: &str,
    locked: Option<&str>,
    archive: &Path,
    unpacked: &Path,
) -> Result<Got, Fail> {
    let unavailable =
        |msg: String| Fail::Unavailable(format!("{msg} (cache directory {})", cache.display()));
    run(
        Command::new("curl")
            .args([
                "-fsSL",
                "--proto",
                "=https,file",
                "--proto-redir",
                "=https",
                "-o",
            ])
            .arg(archive)
            .arg(url),
        "curl",
    )
    .map_err(|e| unavailable(format!("cannot download {url}: {e}")))?;
    let bytes = std::fs::read(archive)
        .map_err(|e| unavailable(format!("cannot read the download of {url}: {e}")))?;
    let hex = sha256::hex(&bytes);
    if locked.is_some_and(|l| l != hex) {
        return Err(Fail::Mismatch { got: hex });
    }
    let dest = cached_dir(cache, &hex);
    if dest.is_dir() {
        return Ok(Got {
            dir: dest,
            sha256: hex,
        });
    }
    std::fs::create_dir_all(unpacked)
        .map_err(|e| unavailable(format!("cannot create {}: {e}", unpacked.display())))?;
    run(
        Command::new("tar")
            .arg("-xzf")
            .arg(archive)
            .arg("-C")
            .arg(unpacked),
        "tar",
    )
    .map_err(|e| unavailable(format!("cannot unpack {url}: {e}")))?;
    // One top directory is the dependency's top directory.
    let mut top = unpacked.to_path_buf();
    let entries: Vec<PathBuf> = std::fs::read_dir(unpacked)
        .map_err(|e| unavailable(format!("cannot read {}: {e}", unpacked.display())))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    if let [one] = &entries[..] {
        if one.is_dir() {
            top = one.clone();
        }
    }
    std::fs::create_dir_all(dest.parent().unwrap_or(cache))
        .map_err(|e| unavailable(format!("cannot create {}: {e}", cache.display())))?;
    if let Err(e) = std::fs::rename(&top, &dest) {
        // Another process may have put the same tree there first.
        if !dest.is_dir() {
            return Err(unavailable(format!(
                "cannot move the tree to {}: {e}",
                dest.display()
            )));
        }
    }
    Ok(Got {
        dir: dest,
        sha256: hex,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_root_order() {
        let r = |a, x, h| cache_root_from(a, x, h);
        assert_eq!(r(Some("/a"), Some("/x"), Some("/h")), Some("/a".into()));
        assert_eq!(r(None, Some("/x"), Some("/h")), Some("/x/aye".into()));
        assert_eq!(r(Some(""), None, Some("/h")), Some("/h/.cache/aye".into()));
        assert_eq!(r(None, None, None), None);
    }
}
