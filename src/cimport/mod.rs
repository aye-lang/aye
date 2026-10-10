//! `@cimport`: Aye declarations generated from a C header with libclang.
//!
//! `let c = @cimport("zlib.h", "-DFOO=1")` parses the header (with the
//! flags, the `pkg-config --cflags` of the module's `@link` libraries and
//! the platform's include directories) and turns its declarations into the
//! source of an Aye module: `pub extern fn`, `pub extern struct` and
//! `extern union`, opaque structs, `pub let` constants for enum values and
//! `#define`d numbers and strings, and type aliases for typedefs. The module
//! loader parses that source like any other module and binds it to the name.
//!
//! The generated source is cached in `$AYE_CACHE_DIR/cimport` (default
//! `~/.cache/aye/cimport`), keyed by the header, the flags, the libraries
//! and the source file's directory, and checked against the modification
//! times and sizes of every file the header included. A cached header needs
//! no libclang. `aye cimport HEADER` prints the source.

mod clang;
mod generate;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Bump when the generated source changes, so old cache entries are not used.
const GENERATOR_VERSION: u32 = 2;

/// A header to import.
#[derive(Clone, Debug, Hash)]
pub struct Request {
    /// As written: `zlib.h`, `SDL2/SDL.h`, or a path relative to `dir`.
    pub header: String,
    /// `-I`, `-D`, `-U`, `-F` and `-std=` flags, as written.
    pub flags: Vec<String>,
    /// The directory of the source file: relative headers and `-I` paths
    /// start there.
    pub dir: PathBuf,
    /// The module's `@link` and `@linkOptional` libraries: their
    /// `pkg-config --cflags` are used.
    pub links: Vec<String>,
}

/// Why a header cannot be imported.
#[derive(Debug)]
pub enum Failure {
    /// libclang cannot be loaded (E0114).
    NoLibclang(String),
    /// The header is missing or does not compile (E0113).
    Header(String),
}

/// What `@cimport` makes of a header: the Aye source and the file it is
/// cached in (the name of the module's file in messages).
pub struct Imported {
    pub source: String,
    pub path: PathBuf,
}

/// Is libclang available (tests skip `@cimport` programs without it)?
pub fn libclang_available() -> bool {
    clang::lib().is_ok()
}

/// The flags that `@cimport` accepts: why `flag` is not one of them.
pub fn check_flag(flag: &str) -> Result<(), String> {
    let ok = ["-I", "-D", "-U", "-F"]
        .iter()
        .any(|p| flag.len() > 2 && flag.starts_with(p))
        || flag.starts_with("-std=") && flag.len() > 5;
    if ok {
        Ok(())
    } else {
        Err(format!(
            "`{flag}` is not a flag `@cimport` takes: an include directory (`-Iinclude`), a definition (`-DNAME` or `-DNAME=value`), `-UNAME`, a framework directory (`-Fdir`) or `-std=c11`"
        ))
    }
}

/// The header's declarations as Aye source: from the cache when it is
/// fresh, else generated and cached.
pub fn import(req: &Request) -> Result<Imported, Failure> {
    let dir = cache_dir();
    let stem: String = Path::new(&req.header)
        .file_stem()
        .map(|s| {
            s.to_string_lossy()
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
                .collect()
        })
        .unwrap_or_default();
    let mut h = DefaultHasher::new();
    (
        GENERATOR_VERSION,
        env!("CARGO_PKG_VERSION"),
        req,
        std::fs::canonicalize(&req.dir).unwrap_or_else(|_| req.dir.clone()),
    )
        .hash(&mut h);
    let path = dir.join(format!("{stem}-{:016x}.aye", h.finish()));
    let deps_path = path.with_extension("deps");
    if let (Ok(source), Ok(deps)) = (
        std::fs::read_to_string(&path),
        std::fs::read_to_string(&deps_path),
    ) && deps.lines().all(|l| {
        l.split_once(' ')
            .is_some_and(|(stamp, file)| stamp_of(Path::new(file)).as_deref() == Some(stamp))
    }) {
        return Ok(Imported { source, path });
    }
    let (source, files) = generate(req)?;
    let deps: String = files
        .iter()
        .filter_map(|f| Some(format!("{} {f}\n", stamp_of(Path::new(f))?)))
        .collect();
    // Written to a temporary file and renamed, so that concurrent builds
    // never read half a file. A cache that cannot be written is no error.
    if std::fs::create_dir_all(&dir).is_ok() {
        prune(&dir);
        let tmp = |p: &Path| PathBuf::from(format!("{}.{}.tmp", p.display(), std::process::id()));
        if std::fs::write(tmp(&path), &source).is_ok()
            && std::fs::write(tmp(&deps_path), &deps).is_ok()
        {
            let _ = std::fs::rename(tmp(&path), &path);
            let _ = std::fs::rename(tmp(&deps_path), &deps_path);
        }
    }
    Ok(Imported { source, path })
}

/// Remove cache entries not written for 30 days (a header that is still
/// imported is written again when it changes, so old entries are stale).
fn prune(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let month = std::time::Duration::from_secs(30 * 24 * 3600);
    for e in entries.flatten() {
        let old = e
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age > month);
        if old {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

/// The cache directory: `$AYE_CACHE_DIR/cimport`, else under
/// `$XDG_CACHE_HOME/aye` or `~/.cache/aye`, else the temporary directory.
fn cache_dir() -> PathBuf {
    let env = |k: &str| {
        std::env::var_os(k)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    };
    let base = env("AYE_CACHE_DIR")
        .or_else(|| env("XDG_CACHE_HOME").map(|d| d.join("aye")))
        .or_else(|| env("HOME").map(|d| d.join(".cache").join("aye")))
        .unwrap_or_else(|| std::env::temp_dir().join("aye-cache"));
    base.join("cimport")
}

/// A file's modification time and size, which a cache entry must match.
fn stamp_of(p: &Path) -> Option<String> {
    let m = std::fs::metadata(p).ok()?;
    let t = m
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?;
    Some(format!(
        "{}.{:09}:{}",
        t.as_secs(),
        t.subsec_nanos(),
        m.len()
    ))
}

/// Generate the Aye source of a header, without the cache: the source and
/// every file the header included.
pub fn generate(req: &Request) -> Result<(String, Vec<String>), Failure> {
    let lib = clang::lib().map_err(|e| {
        Failure::NoLibclang(format!(
            "cannot load libclang, which `@cimport` needs to read C headers ({e}); install the Xcode command line tools or LLVM (libclang), or set LIBCLANG_PATH to the library or its directory"
        ))
    })?;
    let args = clang_args(req);
    let local = req.dir.join(&req.header);
    let include = if local.is_file() {
        let abs = std::fs::canonicalize(&local).unwrap_or_else(|_| local.clone());
        format!("#include \"{}\"\n", abs.display())
    } else {
        format!("#include <{}>\n", req.header)
    };
    generate::run(lib, &req.header, &include, &args).map_err(Failure::Header)
}

/// The clang flags for a request: the request's own (relative `-I` and
/// `-F` paths from its directory), the `pkg-config --cflags` of its
/// libraries, then the platform's: the macOS SDK (`xcrun --show-sdk-path`)
/// and Homebrew's include directory.
fn clang_args(req: &Request) -> Vec<String> {
    let mut args = Vec::new();
    for f in &req.flags {
        match f.get(..2) {
            Some(p @ ("-I" | "-F")) => {
                let path = Path::new(f[2..].trim());
                let path = if path.is_absolute() {
                    path.to_path_buf()
                } else {
                    req.dir.join(path)
                };
                args.push(format!("{p}{}", path.display()));
            }
            _ => args.push(f.trim().to_string()),
        }
    }
    for l in &req.links {
        if l.starts_with("framework:") {
            continue;
        }
        if let Ok(o) = Command::new("pkg-config").args(["--cflags", l]).output()
            && o.status.success()
        {
            let out = String::from_utf8_lossy(&o.stdout).into_owned();
            args.extend(
                out.split_whitespace()
                    .filter(|f| {
                        ["-I", "-D", "-U", "-F", "-isystem"]
                            .iter()
                            .any(|p| f.starts_with(p))
                    })
                    .map(String::from),
            );
        }
    }
    if cfg!(target_os = "macos") && !args.iter().any(|a| a.starts_with("-isysroot")) {
        let sdk = std::env::var("SDKROOT")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| {
                let o = Command::new("xcrun")
                    .args(["--show-sdk-path"])
                    .output()
                    .ok()
                    .filter(|o| o.status.success())?;
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            });
        if let Some(sdk) = sdk {
            args.push(format!("-isysroot{sdk}"));
        }
    }
    let brew = std::env::var("HOMEBREW_PREFIX")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            ["/opt/homebrew", "/usr/local"]
                .iter()
                .map(PathBuf::from)
                .find(|p| p.join("include").is_dir() && p.join("bin/brew").exists())
        });
    if let Some(brew) = brew {
        args.push(format!("-I{}", brew.join("include").display()));
    }
    args
}
