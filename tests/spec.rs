//! `docs/spec.md`, the language reference that `aye spec` prints, cannot rot:
//! every ```aye block is a complete program that `aye check` accepts, every
//! ```aye-error block is one that it rejects, it indexes every std module
//! and names every pub function of the modules it lists in full, std's pub
//! functions have doc comments (what `aye doc --signatures std.m` prints for
//! the others), and it stays within about 30k tokens.

use std::process::Command;

const SPEC: &str = include_str!("../docs/spec.md");

/// The code blocks whose fence is exactly "```lang": (line, code).
fn blocks(lang: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut lines = SPEC.lines().enumerate();
    while let Some((i, l)) = lines.next() {
        let Some(fence) = l.trim_start().strip_prefix("```") else { continue };
        let mut code = String::new();
        for (_, l) in lines.by_ref() {
            if l.trim_start().starts_with("```") {
                break;
            }
            code.push_str(l);
            code.push('\n');
        }
        if fence == lang {
            out.push((i + 1, code));
        }
    }
    out
}

#[test]
fn aye_blocks_compile() {
    let progs = blocks("aye");
    assert!(progs.len() >= 10, "only {} ```aye blocks", progs.len());
    for (line, code) in progs {
        // `@cimport` reads C headers with libclang (or its cache).
        if code.contains("@cimport") && !aye::cimport::libclang_available() {
            println!("skipped docs/spec.md:{line}: no libclang");
            continue;
        }
        if let Err(ds) = aye::driver::check("spec.aye", &code) {
            let ds: Vec<String> = ds.iter().map(|d| d.to_string()).collect();
            panic!("docs/spec.md:{line}: the block does not compile:\n{}\n{code}", ds.join("\n"));
        }
    }
}

#[test]
fn aye_error_blocks_fail() {
    let progs = blocks("aye-error");
    assert!(!progs.is_empty());
    for (line, code) in progs {
        assert!(aye::driver::check("spec.aye", &code).is_err(), "docs/spec.md:{line}: the block compiles:\n{code}");
    }
}

/// The index of section 15: the first code block after "## 15. std", one
/// line per std module, starting with the module's name.
fn std_index() -> Vec<&'static str> {
    let rest = &SPEC[SPEC.find("\n## 15. std\n").expect("no section 15")..];
    let block = &rest[rest.find("\n```\n").expect("no index block") + 5..];
    block[..block.find("\n```\n").unwrap()].lines().collect()
}

/// Every std module has a line in the index of section 15. A module with a
/// `### std.m` section is marked "below"; any other names the command that
/// prints its API, `aye doc --signatures std.m`.
#[test]
fn spec_indexes_every_std_module() {
    let index = std_index();
    for (module, _, _) in aye::module::STD_FILES {
        let line = index.iter().find(|l| l.split_whitespace().next() == Some(module));
        let line = line.unwrap_or_else(|| panic!("the index in docs/spec.md section 15 has no line for {module}"));
        let full = SPEC.contains(&format!("\n### {module}\n"));
        let want = if full { "below".to_string() } else { format!("aye doc --signatures {module}") };
        assert!(line.ends_with(&want), "the index line of {module} does not end with `{want}`: {line}");
    }
}

/// A module with a `### std.m` section names each of its pub functions and
/// methods as `name(`, such as `std.fs.open(` and `File.readAll(`.
#[test]
fn spec_sections_name_every_function() {
    let api = aye::module::std_api();
    let has = |m: &str, f: &str| api.iter().any(|(am, af)| am == m && af == f);
    assert!(has("std.fs", "std.fs.open") && has("std.fs", "File.readAll") && has("std.heap", "General.init"), "{api:?}");
    let missing: Vec<&String> = api
        .iter()
        .filter(|(m, f)| SPEC.contains(&format!("\n### {m}\n")) && !SPEC.contains(&format!("{f}(")))
        .map(|(_, f)| f)
        .collect();
    assert!(missing.is_empty(), "docs/spec.md does not mention {missing:?}");
}

/// Every pub function and method in std's source has a `///` comment right
/// above it, which `aye doc --signatures` prints with the signature.
#[test]
fn std_functions_have_doc_comments() {
    let mut missing = Vec::new();
    for (_, file, src) in aye::module::STD_FILES {
        let lines: Vec<&str> = src.lines().map(str::trim).collect();
        for (i, l) in lines.iter().enumerate() {
            if l.starts_with("pub fn ") && !(i > 0 && lines[i - 1].starts_with("///")) {
                missing.push(format!("{file}:{}: {l}", i + 1));
            }
        }
    }
    assert!(missing.is_empty(), "pub functions without a /// comment:\n{}", missing.join("\n"));
}

/// About 30k tokens, counted as 4 characters per token.
#[test]
fn spec_is_small() {
    let tokens = SPEC.chars().count() / 4;
    assert!(tokens <= 30_000, "docs/spec.md is about {tokens} tokens");
}

#[test]
fn aye_spec_prints_the_spec() {
    let out = Command::new(env!("CARGO_BIN_EXE_aye")).arg("spec").output().unwrap();
    assert!(out.status.success());
    assert_eq!(String::from_utf8_lossy(&out.stdout), SPEC);
}
