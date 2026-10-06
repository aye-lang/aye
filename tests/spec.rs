//! `docs/spec.md`, the language reference that `ore spec` prints, cannot rot:
//! every ```ore block is a complete program that `ore check` accepts, every
//! ```ore-error block is one that it rejects, the reference names every pub
//! function of std, and it stays within about 30k tokens.

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
fn ore_blocks_compile() {
    let progs = blocks("ore");
    assert!(progs.len() >= 10, "only {} ```ore blocks", progs.len());
    for (line, code) in progs {
        if let Err(ds) = ore::driver::check("spec.ore", &code) {
            let ds: Vec<String> = ds.iter().map(|d| d.to_string()).collect();
            panic!("docs/spec.md:{line}: the block does not compile:\n{}\n{code}", ds.join("\n"));
        }
    }
}

#[test]
fn ore_error_blocks_fail() {
    let progs = blocks("ore-error");
    assert!(!progs.is_empty());
    for (line, code) in progs {
        assert!(ore::driver::check("spec.ore", &code).is_err(), "docs/spec.md:{line}: the block compiles:\n{code}");
    }
}

/// Each pub function and method of std appears as `name(`, such as
/// `std.fs.open(` and `File.readAll(`.
#[test]
fn spec_names_every_std_function() {
    let api = ore::module::std_api();
    assert!(api.iter().any(|f| f == "std.fs.open") && api.iter().any(|f| f == "File.readAll"), "{api:?}");
    let missing: Vec<&String> = api.iter().filter(|f| !SPEC.contains(&format!("{f}("))).collect();
    assert!(missing.is_empty(), "docs/spec.md does not mention {missing:?}");
}

/// About 30k tokens, counted as 4 characters per token.
#[test]
fn spec_is_small() {
    let tokens = SPEC.chars().count() / 4;
    assert!(tokens <= 30_000, "docs/spec.md is about {tokens} tokens");
}

#[test]
fn ore_spec_prints_the_spec() {
    let out = Command::new(env!("CARGO_BIN_EXE_ore")).arg("spec").output().unwrap();
    assert!(out.status.success());
    assert_eq!(String::from_utf8_lossy(&out.stdout), SPEC);
}
