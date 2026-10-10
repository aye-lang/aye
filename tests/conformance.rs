//! Conformance harness (O1.1, O11). `tests/conformance/<area>/X.aye` is
//! built and run for each target (`native`, and `wasm32-wasip1` under
//! `node:wasi` where the WASI toolchain and `node` are there); its stdout
//! must equal `X.out`, it must exit 0 and write nothing to stderr. Prints
//! one row per area and target with passed/total, and exits 1, naming the
//! files, if any program fails. A program that imports a std module the
//! target lacks (E0118) is impossible there: it is not counted, a row
//! without any other program shows `-`, and the programs are listed.
//!
//! `harness = false` in Cargo.toml: this file has its own `main`.

use aye::driver::{self, Mode, Root, Target};
use std::path::{Path, PathBuf};

const TARGETS: &[&str] = &["native", "wasm32-wasip1"];

/// `Ok(true)`: the program passed; `Ok(false)`: it cannot exist on the
/// target (it imports a module the target lacks).
type Outcome = Result<bool, String>;
type Results = Vec<(String, Outcome)>;

fn programs(area: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(area)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "aye"))
        .collect();
    v.sort();
    v
}

fn run_built(target: &'static Target, path: &Path, work: &Path) -> Outcome {
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    let src = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let want = std::fs::read_to_string(path.with_extension("out"))
        .map_err(|_| "missing .out file".to_string())?;
    let mut root = Root::single(&name, &src);
    root.target = target;
    let c = match driver::compile_root(root, Mode::Debug) {
        Ok(c) => c,
        Err(d) if d.iter().any(|d| d.code == aye::codes::E0118) => return Ok(false),
        Err(d) => {
            return Err(d
                .iter()
                .map(|d| d.to_string())
                .collect::<Vec<_>>()
                .join("\n"));
        }
    };
    let area = path
        .parent()
        .unwrap()
        .file_name()
        .unwrap()
        .to_string_lossy();
    let exe = work.join(format!(
        "{area}-{}-{}",
        path.file_stem().unwrap().to_string_lossy(),
        target.name
    ));
    driver::cc(&c, &exe, Mode::Debug)?;
    let out = target
        .run_command(&exe)
        .output()
        .map_err(|e| e.to_string())?;
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
    Ok(true)
}

fn run(target: &str, path: &Path, work: &Path) -> Outcome {
    match target {
        "native" => run_built(driver::host_target(), path, work),
        "wasm32-wasip1" => run_built(driver::find_target(target).unwrap(), path, work),
        _ => Err(format!("unknown target {target}")),
    }
}

/// Can programs run for `target` here?
fn available(target: &str) -> bool {
    target == "native"
        || driver::find_target(target)
            .is_ok_and(|t| t.toolchain_available() && driver::node_available())
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
            for &target in TARGETS.iter().filter(|t| available(t)) {
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

    println!("{:<10} {:<14} passed/total", "area", "target");
    let mut failures = Vec::new();
    let mut impossible = Vec::new();
    let (mut total, mut passed_all) = (0, 0);
    for (area, target, results) in &rows {
        let possible = results.iter().filter(|(_, r)| *r != Ok(false)).count();
        let passed = results.iter().filter(|(_, r)| *r == Ok(true)).count();
        if possible == 0 {
            println!("{area:<10} {target:<14} -");
        } else {
            println!("{area:<10} {target:<14} {passed}/{possible}");
        }
        total += possible;
        passed_all += passed;
        for (label, r) in results {
            match r {
                Err(e) => failures.push(format!("{label} [{target}]\n{e}")),
                Ok(false) => impossible.push(format!("{label} [{target}]")),
                Ok(true) => {}
            }
        }
    }
    for t in TARGETS.iter().filter(|t| !available(t)) {
        println!("{:<10} {t:<14} skipped: no toolchain or no node", "(all)");
    }
    println!(
        "\nconformance: {passed_all} passed; {} failed",
        total - passed_all
    );
    if !impossible.is_empty() {
        println!("\nimpossible (a std module the target lacks):");
        for i in &impossible {
            println!("{i}");
        }
    }
    if !failures.is_empty() {
        println!("\nfailing:");
        for f in &failures {
            println!("{f}");
        }
        std::process::exit(1);
    }
}
