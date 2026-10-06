//! `ore doc --signatures`: the pub API of modules as Ore declarations
//! without bodies, each with its doc comment (`///` lines right above it),
//! grouped by module. Meant as compact context for people and tools.
//!
//! Works on tokens (like `ore fmt`), so a module with type errors still
//! gets its documentation. Per declaration:
//!
//! - `pub fn`: the signature. For a type function (`return struct {...}`)
//!   also the struct's fields and pub methods.
//! - `pub struct`, `pub enum`: the fields or variants (visible wherever the
//!   type is) and the pub methods' signatures.
//! - `pub interface`: the method signatures.
//! - `pub error`, `pub let`, `pub var`: the whole declaration on one line.
//!
//! Modules are named by their import path (`shapes`, `std.text`); the root
//! module by the path given. A module's leading comment block (before the first blank line, unless it
//! is a `///` comment) is printed as the module's documentation.

use crate::fmt::{self, Br, Item, Role, Scan};
use crate::lexer::{Kw, Tok};
use crate::module::STD_FILES;
use std::path::{Path, PathBuf};

/// The documentation of the modules at `path`: `std` (every std module) or
/// `std.<name>` (one, from the embedded std), a `.ore` file, or a directory
/// (it and every directory under it with `.ore` files is a module).
pub fn signatures(path: &str) -> Result<String, String> {
    // (module name, files as (name, text))
    let mut modules: Vec<(String, Vec<(String, String)>)> = Vec::new();
    if path == "std" || path.starts_with("std.") {
        for (module, file, src) in STD_FILES {
            if *module == path || module.starts_with(&format!("{path}.")) {
                modules.push((module.to_string(), vec![(file.to_string(), src.to_string())]));
            }
        }
        if modules.is_empty() {
            return Err(format!("ore: no std module `{path}`"));
        }
    } else {
        let p = Path::new(path);
        if p.is_dir() {
            collect(p, path, "", &mut modules)?;
            if modules.is_empty() {
                return Err(format!("ore: {path} has no .ore files"));
            }
        } else {
            let text = std::fs::read_to_string(p).map_err(|e| format!("ore: cannot read {path}: {e}"))?;
            modules.push((path.to_string(), vec![(path.to_string(), text)]));
        }
    }
    let mut out = String::new();
    for (module, files) in &modules {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&format!("// module {module}\n"));
        for (name, text) in files {
            let scan = fmt::scan(text).map_err(|errs| {
                let names = std::slice::from_ref(name);
                errs.into_iter().map(|e| e.with_file(names).render()).collect::<Vec<_>>().join("\n")
            })?;
            file_doc(&scan, &mut out);
        }
    }
    Ok(out)
}

/// The modules under the directory `dir`: `dir` itself if it has `.ore`
/// files, named `root` if it is the root module, else by its import path
/// `prefix`; then its subdirectories (`prefix.sub`), sorted.
fn collect(
    dir: &Path,
    root: &str,
    prefix: &str,
    out: &mut Vec<(String, Vec<(String, String)>)>,
) -> Result<(), String> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| format!("ore: cannot read {}: {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    entries.sort();
    let mut files = Vec::new();
    for p in entries.iter().filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "ore")) {
        let file = p.to_string_lossy().into_owned();
        let text = std::fs::read_to_string(p).map_err(|e| format!("ore: cannot read {file}: {e}"))?;
        files.push((file, text));
    }
    if !files.is_empty() {
        out.push((if prefix.is_empty() { root.to_string() } else { prefix.to_string() }, files));
    }
    for p in entries.iter().filter(|p| p.is_dir()) {
        let sub = p.file_name().unwrap().to_string_lossy().into_owned();
        if !sub.starts_with('.') {
            let prefix = if prefix.is_empty() { sub } else { format!("{prefix}.{sub}") };
            collect(p, root, &prefix, out)?;
        }
    }
    Ok(())
}

/// The lines (1-based) that have tokens.
fn code_lines(scan: &Scan) -> Vec<bool> {
    let mut has = vec![false; scan.comments.len() + 1];
    for it in scan.items.iter().filter(|it| it.role != Role::Semi) {
        has[it.line as usize] = true;
    }
    has
}

fn file_doc(scan: &Scan, out: &mut String) {
    let has_code = code_lines(scan);
    let comment_only = |l: usize| !has_code[l] && scan.comments[l - 1].is_some();
    // The leading comment block, if a blank line ends it.
    let mut l = 1;
    while l <= scan.comments.len() && comment_only(l) && !scan.comments[l - 1].as_ref().unwrap().starts_with("///") {
        l += 1;
    }
    let blank_after = l > scan.comments.len() || (!has_code[l] && scan.comments[l - 1].is_none());
    if l > 1 && blank_after {
        for c in scan.comments[..l - 1].iter().flatten() {
            out.push_str(c);
            out.push('\n');
        }
    }
    let doc = |line: u32, indent: &str, out: &mut String| {
        let mut l = line as usize - 1;
        let mut lines = Vec::new();
        while l >= 1 && comment_only(l) && scan.comments[l - 1].as_ref().unwrap().starts_with("///") {
            lines.push(scan.comments[l - 1].as_ref().unwrap());
            l -= 1;
        }
        for c in lines.iter().rev() {
            out.push_str(indent);
            out.push_str(c);
            out.push('\n');
        }
    };
    let items = &scan.items;
    for decl in split(items, 0, items.len()) {
        let d = &items[decl.0..decl.1];
        if d[0].tok != Tok::Kw(Kw::Pub) {
            continue;
        }
        out.push('\n');
        doc(d[0].line, "", out);
        let kw = d.iter().skip(1).find(|it| it.tok != Tok::Kw(Kw::Extern)).map(|it| &it.tok);
        let body = body(d);
        match (kw, body) {
            (Some(Tok::Kw(Kw::Fn)), Some(b)) => {
                out.push_str(&flat(&d[..b.0]));
                // A type function: `return struct { ... }`.
                let inner: Vec<usize> = (b.0 + 1..b.1).filter(|&i| d[i].role != Role::Semi).collect();
                if let [r, s, o, ..] = inner[..]
                    && d[r].tok == Tok::Kw(Kw::Return)
                    && d[s].tok == Tok::Kw(Kw::Struct)
                    && d[o].role == Role::Open(Br::Brace)
                {
                    let close = matching(d, o);
                    out.push_str(" {\n    return struct {\n");
                    members(d, o + 1, close, false, "        ", &doc, out);
                    out.push_str("    }\n}");
                }
                out.push('\n');
            }
            (Some(Tok::Kw(Kw::Struct | Kw::Enum | Kw::Interface)), Some(b)) if d[0].line != d[d.len() - 1].line => {
                out.push_str(&flat(&d[..b.0]));
                out.push_str(" {\n");
                let all_methods = kw == Some(&Tok::Kw(Kw::Interface));
                members(d, b.0 + 1, b.1, all_methods, "    ", &doc, out);
                out.push_str("}\n");
            }
            // `pub extern union Name { ... }`
            (Some(Tok::Ident(u)), Some(b)) if u == "union" && d[0].line != d[d.len() - 1].line => {
                out.push_str(&flat(&d[..b.0]));
                out.push_str(" {\n");
                let all_methods = kw == Some(&Tok::Kw(Kw::Interface));
                members(d, b.0 + 1, b.1, all_methods, "    ", &doc, out);
                out.push_str("}\n");
            }
            _ => {
                out.push_str(&flat(d));
                out.push('\n');
            }
        }
    }
}

/// The ranges of the declarations or members in `items[from..to]`: split
/// at terminators and commas outside brackets; empty ones dropped.
fn split(items: &[Item], from: usize, to: usize) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut start = from;
    for (i, it) in items.iter().enumerate().take(to).skip(from) {
        match it.role {
            Role::Open(_) => depth += 1,
            Role::Close(_) => depth = depth.saturating_sub(1),
            Role::Semi | Role::Comma if depth == 0 => {
                if start < i {
                    out.push((start, i));
                }
                start = i + 1;
            }
            _ => {}
        }
    }
    if start < to {
        out.push((start, to));
    }
    out
}

/// The index of the bracket that closes the one opened at `open`.
fn matching(items: &[Item], open: usize) -> usize {
    let mut depth = 0usize;
    for (i, it) in items.iter().enumerate().skip(open) {
        match it.role {
            Role::Open(_) => depth += 1,
            Role::Close(_) => {
                depth -= 1;
                if depth == 0 {
                    return i;
                }
            }
            _ => {}
        }
    }
    items.len()
}

/// The first `{` outside brackets in a declaration, and its `}`.
fn body(d: &[Item]) -> Option<(usize, usize)> {
    let mut depth = 0usize;
    for (i, it) in d.iter().enumerate() {
        match it.role {
            Role::Open(Br::Brace) if depth == 0 => return Some((i, matching(d, i))),
            Role::Open(_) => depth += 1,
            Role::Close(_) => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    None
}

/// The members of a struct, enum or interface body `d[from..to]`: fields
/// and variants, and the signatures of methods (only the pub ones unless
/// `all_methods`), each with its doc comment.
fn members(
    d: &[Item],
    from: usize,
    to: usize,
    all_methods: bool,
    indent: &str,
    doc: &dyn Fn(u32, &str, &mut String),
    out: &mut String,
) {
    for (s, e) in split(d, from, to) {
        let m = &d[s..e];
        let is_pub = m[0].tok == Tok::Kw(Kw::Pub);
        let is_fn = m.iter().take(2).any(|it| it.tok == Tok::Kw(Kw::Fn));
        if is_fn && !is_pub && !all_methods {
            continue;
        }
        // A doc comment belongs to a member that starts its line.
        if s == 0 || d[s - 1].role == Role::Semi || d[s - 1].line != m[0].line {
            doc(m[0].line, indent, out);
        }
        let sig = match (is_fn, body(m)) {
            (true, Some((b, _))) => &m[..b],
            _ => m,
        };
        out.push_str(indent);
        out.push_str(&flat(sig));
        out.push('\n');
    }
}

/// Tokens on one line, without the trailing comma of a multi-line list.
fn flat(items: &[Item]) -> String {
    let keep = (0..items.len())
        .filter(|&i| {
            let next = items[i + 1..].iter().find(|it| it.role != Role::Semi);
            !(items[i].role == Role::Comma && next.is_some_and(|n| matches!(n.role, Role::Close(_))))
        })
        .map(|i| &items[i]);
    fmt::join(keep)
}

#[cfg(test)]
mod tests {
    #[test]
    fn std_text_signatures() {
        let out = super::signatures("std.text").unwrap();
        assert!(out.starts_with("// module std.text\n// std.text: byte strings"), "{out}");
        assert!(out.contains("\n/// Do `a` and `b` have the same bytes?\npub fn eql(a: []u8, b: []u8) bool\n"), "{out}");
        assert!(out.contains("pub struct Split {\n    rest: []u8\n"), "{out}");
        assert!(out.contains("    /// The next piece; null after the last one.\n    pub fn next(self: *mut Self) ?[]u8\n}"), "{out}");
        assert!(!out.contains("fn lower("), "{out}");
    }

    #[test]
    fn type_function_members() {
        let out = super::signatures("std.list").unwrap();
        assert!(out.contains("pub fn List(comptime T: type) type {\n    return struct {\n"), "{out}");
        assert!(out.contains("        pub fn append(self: *mut Self, value: T) AllocError!void\n"), "{out}");
    }
}
