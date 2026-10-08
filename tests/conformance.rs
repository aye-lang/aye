//! Conformance harness (O1.1). `tests/conformance/<area>/X.aye` is built
//! and run for each target; its stdout must equal `X.out`, it must exit 0
//! and write nothing to stderr. Prints one row per area and target with
//! passed/total, and exits 1, naming the files, if any program fails.
//!
//! `harness = false` in Cargo.toml: this file has its own `main`.
//! Only the `native` target exists so far.

use aye::driver::{self, Mode};
use std::path::{Path, PathBuf};
use std::process::Command;

const TARGETS: &[&str] = &["native"];

type Results = Vec<(String, Result<(), String>)>;

fn programs(area: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(area)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "aye"))
        .collect();
    v.sort();
    v
}

fn run_native(path: &Path, work: &Path) -> Result<(), String> {
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    let src = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let want = std::fs::read_to_string(path.with_extension("out"))
        .map_err(|_| "missing .out file".to_string())?;
    let c = driver::compile_to_c(&name, &src, Mode::Debug).map_err(|d| {
        d.iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    })?;
    let area = path
        .parent()
        .unwrap()
        .file_name()
        .unwrap()
        .to_string_lossy();
    let exe = work.join(format!(
        "{area}-{}",
        path.file_stem().unwrap().to_string_lossy()
    ));
    driver::cc(&c, &exe, Mode::Debug)?;
    let out = Command::new(&exe).output().map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    if !out.status.success() || !stderr.is_empty() {
        return Err(format!("program failed ({}):\n{stderr}", out.status));
    }
    if stdout != want {
        return Err(format!(
            "stdout mismatch\n--- expected\n{want}--- got\n{stdout}"
        ));
    }
    Ok(())
}

fn run(target: &str, path: &Path, work: &Path) -> Result<(), String> {
    match target {
        "native" => run_native(path, work),
        _ => Err(format!("unknown target {target}")),
    }
}

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/conformance");
    let work = std::env::temp_dir().join(format!("aye-conformance-{}", std::process::id()));
    std::fs::create_dir_all(&work).unwrap();
    let mut areas: Vec<PathBuf> = std::fs::read_dir(&root)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_dir())
        .collect();
    areas.sort();

    // One row per (area, target).
    let rows: Vec<(String, &str, Results)> = std::thread::scope(|s| {
        let mut pending = Vec::new();
        for area in &areas {
            let area_name = area.file_name().unwrap().to_string_lossy().into_owned();
            for &target in TARGETS {
                let handles: Vec<_> = programs(area)
                    .into_iter()
                    .map(|p| {
                        let w = &work;
                        let label =
                            format!("{area_name}/{}", p.file_name().unwrap().to_string_lossy());
                        (label, s.spawn(move || run(target, &p, w)))
                    })
                    .collect();
                pending.push((area_name.clone(), target, handles));
            }
        }
        pending
            .into_iter()
            .map(|(a, t, hs)| {
                (
                    a,
                    t,
                    hs.into_iter()
                        .map(|(l, h)| (l, h.join().unwrap()))
                        .collect(),
                )
            })
            .collect()
    });
    let _ = std::fs::remove_dir_all(&work);

    println!("{:<10} {:<8} passed/total", "area", "target");
    let mut failures = Vec::new();
    for (area, target, results) in &rows {
        let passed = results.iter().filter(|(_, r)| r.is_ok()).count();
        println!("{area:<10} {target:<8} {passed}/{}", results.len());
        for (label, r) in results {
            if let Err(e) = r {
                failures.push(format!("{label} [{target}]\n{e}"));
            }
        }
    }
    let total: usize = rows.iter().map(|r| r.2.len()).sum();
    println!(
        "\nconformance: {} passed; {} failed",
        total - failures.len(),
        failures.len()
    );
    if !failures.is_empty() {
        println!("\nfailing:");
        for f in &failures {
            println!("{f}");
        }
        std::process::exit(1);
    }
}
