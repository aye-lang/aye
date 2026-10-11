//! `aye doc --signatures`: the pub API of modules as Aye declarations
//! without bodies, each with its doc comment (`///` lines right above it),
//! grouped by module. Meant as compact context for people and tools.
//!
//! Works on tokens (like `aye fmt`), so a module with type errors still
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
//!
//! `aye doc --html` writes the same documentation as HTML pages: see `html`.

use crate::fmt::{self, Br, Item, Role, Scan};
use crate::lexer::{Kw, Tok};
use crate::module::{STD_FILES, std_module_hint};
use std::path::{Path, PathBuf};

/// Modules as (module name, files as (name, text)).
type Modules = Vec<(String, Vec<(String, String)>)>;

/// The modules at `path` as (module name, files as (name, text)): `std`
/// (every std module) or `std.<name>` (one, from the embedded std), a `.aye`
/// file, or a directory (it and every directory under it with `.aye` files
/// is a module).
fn load(path: &str) -> Result<Modules, String> {
    let mut modules: Modules = Vec::new();
    if path == "std" || path.starts_with("std.") {
        for (module, file, src) in STD_FILES {
            if *module == path || module.starts_with(&format!("{path}.")) {
                modules.push((
                    module.to_string(),
                    vec![(file.to_string(), src.to_string())],
                ));
            }
        }
        if modules.is_empty() {
            let seg = path.strip_prefix("std.").unwrap_or(path);
            return Err(format!("aye: {}", std_module_hint(seg)));
        }
    } else {
        let p = Path::new(path);
        if p.is_dir() {
            collect(p, path, "", &mut modules)?;
            if modules.is_empty() {
                return Err(format!("aye: {path} has no .aye files"));
            }
        } else {
            let text =
                std::fs::read_to_string(p).map_err(|e| format!("aye: cannot read {path}: {e}"))?;
            modules.push((path.to_string(), vec![(path.to_string(), text)]));
        }
    }
    Ok(modules)
}

/// The documentation text of one module's files, as `signatures` prints it
/// after the module's header line.
fn module_text(files: &[(String, String)]) -> Result<String, String> {
    let mut out = String::new();
    for (name, text) in files {
        let scan = fmt::scan(text).map_err(|errs| {
            let names = std::slice::from_ref(name);
            errs.into_iter()
                .map(|e| e.with_file(names).render())
                .collect::<Vec<_>>()
                .join("\n")
        })?;
        file_doc(&scan, &mut out);
    }
    Ok(out)
}

/// The documentation of the modules at `path`: `std` (every std module) or
/// `std.<name>` (one, from the embedded std), a `.aye` file, or a directory
/// (it and every directory under it with `.aye` files is a module).
pub fn signatures(path: &str) -> Result<String, String> {
    let mut out = String::new();
    for (module, files) in &load(path)? {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&format!("// module {module}\n"));
        out.push_str(&module_text(files)?);
    }
    Ok(out)
}

/// The modules under the directory `dir`: `dir` itself if it has `.aye`
/// files, named `root` if it is the root module, else by its import path
/// `prefix`; then its subdirectories (`prefix.sub`), sorted.
fn collect(
    dir: &Path,
    root: &str,
    prefix: &str,
    out: &mut Vec<(String, Vec<(String, String)>)>,
) -> Result<(), String> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| format!("aye: cannot read {}: {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    entries.sort();
    let mut files = Vec::new();
    for p in entries
        .iter()
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "aye"))
    {
        let file = p.to_string_lossy().into_owned();
        let text =
            std::fs::read_to_string(p).map_err(|e| format!("aye: cannot read {file}: {e}"))?;
        files.push((file, text));
    }
    if !files.is_empty() {
        out.push((
            if prefix.is_empty() {
                root.to_string()
            } else {
                prefix.to_string()
            },
            files,
        ));
    }
    for p in entries.iter().filter(|p| p.is_dir()) {
        let sub = p.file_name().unwrap().to_string_lossy().into_owned();
        if !sub.starts_with('.') {
            let prefix = if prefix.is_empty() {
                sub
            } else {
                format!("{prefix}.{sub}")
            };
            collect(p, root, &prefix, out)?;
        }
    }
    Ok(())
}

/// The lines (1-based) that have tokens.
pub(crate) fn code_lines(scan: &Scan) -> Vec<bool> {
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
    while l <= scan.comments.len()
        && comment_only(l)
        && !scan.comments[l - 1].as_ref().unwrap().starts_with("///")
    {
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
        for c in doc_above(scan, &has_code, line) {
            out.push_str(indent);
            out.push_str(&c);
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
        let kw = d
            .iter()
            .skip(1)
            .find(|it| it.tok != Tok::Kw(Kw::Extern))
            .map(|it| &it.tok);
        let body = body(d);
        match (kw, body) {
            (Some(Tok::Kw(Kw::Fn)), Some(b)) => {
                out.push_str(&flat(&d[..b.0]));
                // A type function: `return struct { ... }`.
                let inner: Vec<usize> = (b.0 + 1..b.1)
                    .filter(|&i| d[i].role != Role::Semi)
                    .collect();
                if let [r, s, o, ..] = inner[..]
                    && d[r].tok == Tok::Kw(Kw::Return)
                    && d[s].tok == Tok::Kw(Kw::Struct)
                    && d[o].role == Role::Open(Br::Brace)
                {
                    let close = matching(d, o);
                    out.push_str(" {\n    return struct {\n");
                    let found = members(scan, &has_code, d, o + 1, close);
                    render_members(&found, false, "        ", out);
                    out.push_str("    }\n}");
                }
                out.push('\n');
            }
            (Some(Tok::Kw(Kw::Struct | Kw::Enum | Kw::Interface)), Some(b))
                if d[0].line != d[d.len() - 1].line =>
            {
                out.push_str(&flat(&d[..b.0]));
                out.push_str(" {\n");
                let all_methods = kw == Some(&Tok::Kw(Kw::Interface));
                let found = members(scan, &has_code, d, b.0 + 1, b.1);
                render_members(&found, all_methods, "    ", out);
                out.push_str("}\n");
            }
            // `pub extern union Name { ... }`
            (Some(Tok::Ident(u)), Some(b)) if u == "union" && d[0].line != d[d.len() - 1].line => {
                out.push_str(&flat(&d[..b.0]));
                out.push_str(" {\n");
                let all_methods = kw == Some(&Tok::Kw(Kw::Interface));
                let found = members(scan, &has_code, d, b.0 + 1, b.1);
                render_members(&found, all_methods, "    ", out);
                out.push_str("}\n");
            }
            _ => {
                out.push_str(&flat(d));
                out.push('\n');
            }
        }
    }
}

/// The `///` comment lines right above `line`, in order.
pub(crate) fn doc_above(scan: &Scan, has_code: &[bool], line: u32) -> Vec<String> {
    let comment_only = |l: usize| !has_code[l] && scan.comments[l - 1].is_some();
    let mut l = line as usize - 1;
    let mut lines = Vec::new();
    while l >= 1 && comment_only(l) && scan.comments[l - 1].as_ref().unwrap().starts_with("///") {
        lines.push(scan.comments[l - 1].clone().unwrap());
        l -= 1;
    }
    lines.reverse();
    lines
}

/// The ranges of the declarations or members in `items[from..to]`: split
/// at terminators and commas outside brackets; empty ones dropped.
pub(crate) fn split(items: &[Item], from: usize, to: usize) -> Vec<(usize, usize)> {
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

/// The index of the bracket that closes the one opened at `open`; the end
/// of `items` if there is none.
pub(crate) fn matching(items: &[Item], open: usize) -> usize {
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
pub(crate) fn body(d: &[Item]) -> Option<(usize, usize)> {
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

/// A member of a struct, enum or interface body: a field, a variant or a
/// method.
pub(crate) struct Member<'a> {
    /// Its tokens.
    pub items: &'a [Item],
    /// The index of `fn` in `items`, for a method.
    pub fn_at: Option<usize>,
    pub is_pub: bool,
    /// The member on one line; a method without its body.
    pub sig: String,
    /// The `///` lines above it, if it starts its line.
    pub doc: Vec<String>,
}

impl Member<'_> {
    /// The token of its name: a method's after `fn`, else the first
    /// identifier.
    pub fn name(&self) -> Option<&Item> {
        let name = match self.fn_at {
            Some(f) => self.items.get(f + 1),
            None => self.items.iter().find(|it| matches!(it.tok, Tok::Ident(_))),
        };
        name.filter(|n| matches!(n.tok, Tok::Ident(_)))
    }

    /// Does the declaration show it: a field or variant always, a method if
    /// it is pub or `all_methods` (an interface's).
    pub fn shown(&self, all_methods: bool) -> bool {
        self.fn_at.is_none() || self.is_pub || all_methods
    }
}

/// The members of a struct, enum or interface body `d[from..to]`.
pub(crate) fn members<'a>(
    scan: &Scan,
    has_code: &[bool],
    d: &'a [Item],
    from: usize,
    to: usize,
) -> Vec<Member<'a>> {
    let mut out = Vec::new();
    for (s, e) in split(d, from, to) {
        let m = &d[s..e];
        let fn_at = m.iter().take(2).position(|it| it.tok == Tok::Kw(Kw::Fn));
        // A doc comment belongs to a member that starts its line.
        let doc = if s == 0 || d[s - 1].role == Role::Semi || d[s - 1].line != m[0].line {
            doc_above(scan, has_code, m[0].line)
        } else {
            Vec::new()
        };
        let sig = match (fn_at, body(m)) {
            (Some(_), Some((b, _))) => flat(&m[..b]),
            _ => flat(m),
        };
        out.push(Member {
            items: m,
            fn_at,
            is_pub: m[0].tok == Tok::Kw(Kw::Pub),
            sig,
            doc,
        });
    }
    out
}

/// The lines of `members` that a declaration shows (`Member::shown`):
/// each member after its doc comment, indented by `indent`.
pub(crate) fn render_members(
    members: &[Member],
    all_methods: bool,
    indent: &str,
    out: &mut String,
) {
    for m in members.iter().filter(|m| m.shown(all_methods)) {
        for c in m.doc.iter().chain([&m.sig]) {
            out.push_str(indent);
            out.push_str(c);
            out.push('\n');
        }
    }
}

/// Tokens on one line, without the trailing comma of a multi-line list.
pub(crate) fn flat(items: &[Item]) -> String {
    let keep = (0..items.len())
        .filter(|&i| {
            let next = items[i + 1..].iter().find(|it| it.role != Role::Semi);
            !(items[i].role == Role::Comma
                && next.is_some_and(|n| matches!(n.role, Role::Close(_))))
        })
        .map(|i| &items[i]);
    fmt::join(keep)
}

/// `aye doc --html`: write `index.html` (the module list) and one
/// `<module>.html` per module of `path` (see `signatures`) into `out_dir`,
/// which is created. Returns the number of modules.
pub fn html(path: &str, out_dir: &Path) -> Result<usize, String> {
    let mut modules = load(path)?;
    let std = path == "std" || path.starts_with("std.");
    // (module name, page file, documentation)
    let mut pages: Vec<(String, String, Page)> = Vec::new();
    for (module, files) in &mut modules {
        if !std && module == path {
            *module = root_name(path);
        }
        let page = parse_page(&module_text(files)?);
        pages.push((module.clone(), page_file(module), page));
    }
    pages.sort_by(|a, b| a.0.cmp(&b.0));
    if let Some((m, f, _)) = pages.iter().find(|p| p.1 == "index.html") {
        return Err(format!(
            "aye: module `{m}` has the page `{f}` of the module list"
        ));
    }
    for w in pages.windows(2) {
        if w[0].1 == w[1].1 {
            return Err(format!(
                "aye: modules `{}` and `{}` have the same page `{}`",
                w[0].0, w[1].0, w[0].1
            ));
        }
    }
    let list: Vec<(&str, &str)> = pages
        .iter()
        .map(|(m, f, _)| (m.as_str(), f.as_str()))
        .collect();
    std::fs::create_dir_all(out_dir)
        .map_err(|e| format!("aye: cannot create {}: {e}", out_dir.display()))?;
    let write = |file: &str, text: String| {
        let p = out_dir.join(file);
        std::fs::write(&p, text).map_err(|e| format!("aye: cannot write {}: {e}", p.display()))
    };
    write("index.html", index_html(&list))?;
    for (module, file, page) in &pages {
        write(file, module_html(module, page, &list))?;
    }
    Ok(pages.len())
}

/// The name of the root module of a file or directory `path`: its stem or
/// its own name, so that it makes a file name. The stem `index` would be the
/// module list's page, so a file `index.aye` is named after its directory.
fn root_name(path: &str) -> String {
    let p = Path::new(path);
    let dir_name = |d: &Path| {
        d.canonicalize()
            .ok()
            .and_then(|c| c.file_name().map(|n| n.to_string_lossy().into_owned()))
    };
    let name = if p.is_dir() {
        dir_name(p)
    } else {
        p.file_stem()
            .map(|n| n.to_string_lossy().into_owned())
            .filter(|n| n != "index")
            .or_else(|| dir_name(p.parent().unwrap_or(Path::new("."))))
    };
    name.filter(|n| n != "index")
        .unwrap_or_else(|| "root".to_string())
}

/// The page file of a module: its name with other characters than letters,
/// digits, `.`, `_` and `-` replaced by `_`: `std.text.html`. `index.html`
/// is the module list's (`html` refuses a module that would use it).
fn page_file(module: &str) -> String {
    let stem: String = module
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("{stem}.html")
}

/// One module's documentation: its comment lines and its pub declarations.
struct Page {
    doc: Vec<String>,
    decls: Vec<Decl>,
}

/// A pub declaration: its `///` lines (without the slashes) and its
/// signature text.
struct Decl {
    doc: Vec<String>,
    code: String,
}

/// Parse the text of `module_text`: paragraphs separated by blank lines;
/// one of only `//` comment lines is module documentation, any other is a
/// declaration with the `///` lines at its start as its documentation.
fn parse_page(text: &str) -> Page {
    let mut page = Page {
        doc: Vec::new(),
        decls: Vec::new(),
    };
    let strip = |l: &str, n: usize| {
        let l = &l[n..];
        l.strip_prefix(' ').unwrap_or(l).to_string()
    };
    let mut group: Vec<&str> = Vec::new();
    for line in text.lines().chain(std::iter::once("")) {
        if !line.is_empty() {
            group.push(line);
            continue;
        }
        let docs = group.iter().take_while(|l| l.starts_with("///")).count();
        if docs == 0 && !group.is_empty() && group.iter().all(|l| l.starts_with("//")) {
            if !page.doc.is_empty() {
                page.doc.push(String::new());
            }
            page.doc.extend(group.iter().map(|l| strip(l, 2)));
        } else if docs < group.len() {
            page.decls.push(Decl {
                doc: group[..docs].iter().map(|l| strip(l, 3)).collect(),
                code: group[docs..].join("\n"),
            });
        }
        group.clear();
    }
    page
}

/// Split a signature into the lines to show and its documented members:
/// each `///` line inside it (a member's doc comment) goes, with the member's
/// line that follows, to the second list.
fn split_member_docs(code: &str) -> (String, Vec<(String, Vec<String>)>) {
    let (mut shown, mut members) = (Vec::new(), Vec::new());
    let mut doc: Vec<String> = Vec::new();
    for line in code.lines() {
        if let Some(d) = line.trim_start().strip_prefix("///") {
            doc.push(d.strip_prefix(' ').unwrap_or(d).to_string());
            continue;
        }
        if !doc.is_empty() {
            members.push((line.trim().to_string(), std::mem::take(&mut doc)));
        }
        shown.push(line);
    }
    (shown.join("\n"), members)
}

/// `text` with the characters HTML treats specially escaped.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

/// Doc comment lines as HTML: a blank line ends a paragraph; lines that
/// start with 4 spaces, and lines between fences, are `<pre>`; the rest is
/// plain text in `<p>`.
fn doc_html(lines: &[String]) -> String {
    fn flush(para: &mut Vec<String>, pre: &mut Vec<String>, out: &mut String) {
        if !para.is_empty() {
            out.push_str(&format!("<p>{}</p>\n", escape(&para.join("\n"))));
            para.clear();
        }
        if !pre.is_empty() {
            out.push_str(&format!("<pre>{}</pre>\n", escape(&pre.join("\n"))));
            pre.clear();
        }
    }
    let (mut out, mut para, mut pre) = (String::new(), Vec::new(), Vec::new());
    let mut fenced = false;
    for line in lines {
        if fenced {
            if line.trim_start().starts_with("```") {
                fenced = false;
                flush(&mut para, &mut pre, &mut out);
            } else {
                pre.push(line.trim_end().to_string());
            }
        } else if line.trim_start().starts_with("```") {
            flush(&mut para, &mut pre, &mut out);
            fenced = true;
        } else if line.trim().is_empty() {
            flush(&mut para, &mut pre, &mut out);
        } else if let Some(code) = line.strip_prefix("    ") {
            flush(&mut para, &mut Vec::new(), &mut out);
            pre.push(code.trim_end().to_string());
        } else {
            flush(&mut Vec::new(), &mut pre, &mut out);
            para.push(line.trim().to_string());
        }
    }
    flush(&mut para, &mut pre, &mut out);
    out
}

/// The name a declaration introduces: `eql` for `pub fn eql(...)`.
fn decl_name(code: &str) -> String {
    let first = code.lines().next().unwrap_or("");
    let mut words = first
        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|w| !w.is_empty() && !matches!(*w, "pub" | "extern"));
    let kind = words.next().unwrap_or("");
    words.next().unwrap_or(kind).to_string()
}

const STYLE: &str = "\
:root { color-scheme: light dark; --bg: #fff; --fg: #1d2127; --dim: #5b6470; --code: #f3f4f6; --line: #d8dce1; --link: #0b5cad; }
@media (prefers-color-scheme: dark) {
  :root { --bg: #14171b; --fg: #e3e6ea; --dim: #99a2ad; --code: #1f242a; --line: #333a42; --link: #7db4ee; }
}
body { margin: 0 auto; padding: 1rem 1rem 4rem; max-width: 46rem; background: var(--bg); color: var(--fg); font: 1rem/1.6 system-ui, sans-serif; }
a { color: var(--link); }
h1 { font-size: 1.8rem; margin: 1.5rem 0 1rem; }
h2 { font-size: 1.2rem; margin: 2.2rem 0 .6rem; padding-top: 1rem; border-top: 1px solid var(--line); }
pre { margin: .8rem 0; padding: .7rem 1rem; overflow-x: auto; background: var(--code); border-radius: 6px; font: .9rem/1.5 ui-monospace, monospace; }
code { font-family: ui-monospace, monospace; }
nav ul { display: flex; flex-wrap: wrap; gap: .3rem 1rem; margin: .6rem 0 0; padding: 0; list-style: none; }
nav [aria-current] { color: var(--fg); font-weight: 600; text-decoration: none; }
dt { margin: .8rem 0 .2rem; }
dd { margin: 0 0 0 1.5rem; }
dd p { margin: .2rem 0; }
header { padding-bottom: .8rem; border-bottom: 1px solid var(--line); }
";

fn page_html(title: &str, nav: &str, body: &str) -> String {
    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>{title}</title>\n<style>\n{STYLE}</style>\n</head>\n<body>\n\
         <header>\n{nav}</header>\n<main>\n{body}</main>\n</body>\n</html>\n"
    )
}

/// The link list of every module; the one `current` is marked.
fn nav_html(modules: &[(&str, &str)], current: Option<&str>) -> String {
    let mut out = String::from("<nav aria-label=\"Modules\">\n<ul>\n");
    for (module, file) in modules {
        let name = escape(module);
        if current == Some(module) {
            out.push_str(&format!(
                "<li><a href=\"{file}\" aria-current=\"page\">{name}</a></li>\n"
            ));
        } else {
            out.push_str(&format!("<li><a href=\"{file}\">{name}</a></li>\n"));
        }
    }
    out.push_str("</ul>\n</nav>\n");
    out
}

fn index_html(modules: &[(&str, &str)]) -> String {
    let nav = "<a href=\"index.html\">Aye documentation</a>\n";
    let mut body = String::from("<h1>Aye documentation</h1>\n<h2>Modules</h2>\n");
    body.push_str(&nav_html(modules, None));
    page_html("Aye documentation", nav, &body)
}

fn module_html(module: &str, page: &Page, modules: &[(&str, &str)]) -> String {
    let name = escape(module);
    let mut nav = String::from("<a href=\"index.html\">Aye documentation</a>\n");
    nav.push_str(&nav_html(modules, Some(module)));
    let mut body = format!("<h1>{name}</h1>\n");
    body.push_str(&doc_html(&page.doc));
    for d in &page.decls {
        let id = escape(&decl_name(&d.code));
        let (code, members) = split_member_docs(&d.code);
        body.push_str(&format!(
            "<h2 id=\"{id}\">{id}</h2>\n<pre><code>{}</code></pre>\n",
            escape(&code)
        ));
        body.push_str(&doc_html(&d.doc));
        if !members.is_empty() {
            body.push_str("<dl>\n");
            for (sig, doc) in &members {
                body.push_str(&format!(
                    "<dt><code>{}</code></dt>\n<dd>\n{}</dd>\n",
                    escape(sig),
                    doc_html(doc)
                ));
            }
            body.push_str("</dl>\n");
        }
    }
    page_html(&format!("{name} - Aye documentation"), &nav, &body)
}

#[cfg(test)]
mod tests {
    #[test]
    fn std_text_signatures() {
        let out = super::signatures("std.text").unwrap();
        assert!(
            out.starts_with("// module std.text\n// std.text: byte strings"),
            "{out}"
        );
        assert!(
            out.contains(
                "\n/// Do `a` and `b` have the same bytes?\npub fn eql(a: []u8, b: []u8) bool\n"
            ),
            "{out}"
        );
        assert!(
            out.contains("pub struct Split {\n    rest: []u8\n"),
            "{out}"
        );
        assert!(out.contains("    /// The next piece; null after the last one.\n    pub fn next(self: *mut Self) ?[]u8\n}"), "{out}");
        assert!(!out.contains("fn lower("), "{out}");
    }

    #[test]
    fn type_function_members() {
        let out = super::signatures("std.list").unwrap();
        assert!(
            out.contains("pub fn List(comptime T: type) type {\n    return struct {\n"),
            "{out}"
        );
        assert!(
            out.contains("        pub fn append(self: *mut Self, value: T) AllocError!void\n"),
            "{out}"
        );
    }

    #[test]
    fn doc_comment_blocks() {
        let lines: Vec<String> = [
            "Compare <a> & \"b\".",
            "Second line.",
            "",
            "New paragraph.",
            "    indented()",
            "    more <b>",
            "Back to text.",
            "```",
            "fenced & raw",
            "",
            "still fenced",
            "```",
        ]
        .iter()
        .map(|l| l.to_string())
        .collect();
        assert_eq!(
            super::doc_html(&lines),
            "<p>Compare &lt;a&gt; &amp; &quot;b&quot;.\nSecond line.</p>\n\
             <p>New paragraph.</p>\n\
             <pre>indented()\nmore &lt;b&gt;</pre>\n\
             <p>Back to text.</p>\n\
             <pre>fenced &amp; raw\n\nstill fenced</pre>\n"
        );
    }

    #[test]
    fn page_files_and_names() {
        assert_eq!(super::page_file("std.text"), "std.text.html");
        assert_eq!(super::page_file("a/b c"), "a_b_c.html");
        assert_eq!(super::page_file("index"), "index.html");
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/doc/shapes");
        let file = |name: &str| root.join(name).to_string_lossy().into_owned();
        assert_eq!(super::root_name(&file("shapes.aye")), "shapes");
        assert_eq!(super::root_name(&file("index.aye")), "shapes");
        assert_eq!(super::root_name(&root.to_string_lossy()), "shapes");
        assert_eq!(super::decl_name("pub fn eql(a: []u8) bool"), "eql");
        assert_eq!(super::decl_name("pub extern union U {"), "U");
        assert_eq!(super::decl_name("pub let MAX: i64 = 3"), "MAX");
    }
}
