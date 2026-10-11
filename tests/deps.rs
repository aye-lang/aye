//! URL dependencies, `aye.lock` and the cache (O4.3).
//!
//! `tests/deps/url/tinyjson-1.2/` is packed with `tar -czf` into a temporary
//! directory; the app's `aye.toml` is `aye.toml.in` with `@TARBALL@`
//! replaced by that file, so the dependency is a `file://` URL and nothing
//! uses the network. `AYE_CACHE` is a temporary directory, never the real
//! cache.

use std::path::{Path, PathBuf};
use std::process::Command;

fn url_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/deps/url")
}

struct Fx {
    dir: PathBuf,
    tarball: PathBuf,
}

impl Fx {
    /// A new directory with the app and, if `pack`, the tarball.
    fn new(name: &str, pack: bool) -> Fx {
        let dir = std::env::temp_dir().join(format!("aye-deps-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("app")).unwrap();
        let tarball = dir.join("tinyjson-1.2.tar.gz");
        if pack {
            let status = Command::new("tar")
                .env("COPYFILE_DISABLE", "1")
                .arg("-czf")
                .arg(&tarball)
                .arg("-C")
                .arg(url_dir())
                .arg("tinyjson-1.2")
                .status()
                .unwrap();
            assert!(status.success(), "tar -czf failed");
        }
        let app = url_dir().join("app");
        std::fs::copy(app.join("main.aye"), dir.join("app/main.aye")).unwrap();
        let toml = std::fs::read_to_string(app.join("aye.toml.in")).unwrap();
        let toml = toml.replace("@TARBALL@", tarball.to_str().unwrap());
        std::fs::write(dir.join("app/aye.toml"), toml).unwrap();
        Fx { dir, tarball }
    }

    fn app(&self) -> PathBuf {
        self.dir.join("app")
    }

    fn cache(&self) -> PathBuf {
        self.dir.join("cache")
    }

    fn lock(&self) -> Option<String> {
        std::fs::read_to_string(self.app().join("aye.lock")).ok()
    }

    /// `aye ARGS` in the app, with the cache in the temporary directory.
    fn aye(&self, args: &[&str]) -> (String, String, Option<i32>) {
        let out = Command::new(env!("CARGO_BIN_EXE_aye"))
            .current_dir(self.app())
            .env("AYE_CACHE", self.cache())
            .env_remove("XDG_CACHE_HOME")
            .args(args)
            .output()
            .unwrap();
        (
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
            out.status.code(),
        )
    }
}

impl Drop for Fx {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn first_build_locks_then_builds_offline_then_rejects_a_changed_archive() {
    let fx = Fx::new("lock", true);
    let want = std::fs::read_to_string(url_dir().join("app.out")).unwrap();
    let hex = aye::sha256::hex(&std::fs::read(&fx.tarball).unwrap());

    // 1. The first build downloads, locks and prints the hash on stderr,
    // so stdout is the program's output alone.
    let (out, err, code) = fx.aye(&["run"]);
    assert_eq!(
        (out, err, code),
        (
            want.clone(),
            format!("aye.lock: tinyjson sha256 {hex}\n"),
            Some(0)
        )
    );
    let lock = fx.lock().unwrap();
    assert_eq!(
        lock,
        format!(
            "# Written by aye build. Commit it; do not edit it.\n[deps]\ntinyjson = {{ url = \"file://{}\", sha256 = \"{hex}\" }}\n",
            fx.tarball.display()
        )
    );
    assert!(
        fx.cache()
            .join("sha256")
            .join(&hex)
            .join("tinyjson.aye")
            .is_file()
    );

    // 2. Locked and cached: without the tarball the build still works, so
    // `curl` did not run, and the lock is not written again.
    let backup = fx.dir.join("backup.tar.gz");
    std::fs::rename(&fx.tarball, &backup).unwrap();
    let (out, err, code) = fx.aye(&["run"]);
    assert_eq!((out, err, code), (want, String::new(), Some(0)));
    assert_eq!(fx.lock().unwrap(), lock);

    // 3. A locked hash that the archive does not have: E0122, and neither
    // the lock nor the cache changes.
    let zero = "0".repeat(64);
    let bad = lock.replace(&hex, &zero);
    std::fs::write(fx.app().join("aye.lock"), &bad).unwrap();
    std::fs::remove_dir_all(fx.cache()).unwrap();
    std::fs::rename(&backup, &fx.tarball).unwrap();
    let (out, err, code) = fx.aye(&["check"]);
    assert_eq!((out.as_str(), code), ("", Some(1)), "{err}");
    assert!(err.contains("aye.toml:5:1: error[E0122]"), "{err}");
    assert!(err.contains(&hex) && err.contains(&zero), "{err}");
    assert_eq!(fx.lock().unwrap(), bad);
    assert!(!fx.cache().join("sha256").exists());
}

#[test]
fn a_dependency_that_cannot_be_downloaded_is_e0121() {
    let fx = Fx::new("missing", false);
    let (out, err, code) = fx.aye(&["check"]);
    assert_eq!((out.as_str(), code), ("", Some(1)), "{err}");
    assert!(err.contains("aye.toml:5:1: error[E0121]"), "{err}");
    assert!(err.contains("cache directory"), "{err}");
    assert!(fx.lock().is_none());
}

#[test]
fn http_is_e0119() {
    let fx = Fx::new("http", false);
    let toml = std::fs::read_to_string(fx.app().join("aye.toml")).unwrap();
    let toml = toml.replace("file://", "http://example.com");
    std::fs::write(fx.app().join("aye.toml"), toml).unwrap();
    let (out, err, code) = fx.aye(&["check"]);
    assert_eq!((out.as_str(), code), ("", Some(1)), "{err}");
    assert!(err.contains("aye.toml:5:14: error[E0119]"), "{err}");
    assert!(fx.lock().is_none());
    assert!(!fx.cache().exists());
}
