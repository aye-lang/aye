//! Hover and go to definition for `aye lsp`.
//!
//! A name after a dot (`p.len()`, `a.b.c`) is first looked up in what sema
//! resolved: the front end runs on the document with `Root::members`, and
//! the field or method of a struct or enum value it found for the name
//! (`driver::Checked::members`) is the answer. That covers locals,
//! parameters, globals and chains, whatever the type of the value before
//! the dot, even in a document with type errors elsewhere; it gives nothing
//! for a document that does not parse, or an access sema did not get to.
//!
//! Everything else works on tokens (like `aye fmt` and `aye doc`), from the
//! document's own tokens and the files of the modules it imports, in this
//! order:
//!
//! - a declaration whose name is the token itself (a method or a field in a
//!   struct, say),
//! - a local of the enclosing function: its parameter, or the `let`, `var`,
//!   `for` or capture name nearest above whose block the name is in,
//! - a top-level declaration of the document's own file,
//! - an `import` of the document: a module, or an item of a module.
//!
//! After a dot, `a.b.c` is resolved from the left: `a` as above (never a
//! local, whose type sema gives), then each name as a pub declaration or
//! submodule of a module, or a member (method, field, variant) of a struct,
//! enum or interface.
//!
//! A cursor just past the end of a name is on the name.
//!
//! Modules are the `.aye` files of the directory `import a.b` names under the
//! document's directory, under a path dependency's directory (the `aye.toml`
//! next to the document) if `a` is its key, or std. std is embedded in the
//! compiler; its files are located under this compiler's source tree
//! (`CARGO_MANIFEST_DIR`) if that is still on disk; if not, hover works and
//! definition gives `null`. URL dependencies are searched only through
//! sema.
//!
//! `c.name` where `c` is a top-level `let c = @cimport("header.h", flags)`
//! of the document is looked up in the module that `@cimport` generates
//! (sema flattens modules before it records members, so it has nothing on
//! these): hover shows the generated declaration (an `extern fn`,
//! `extern struct`, constant), and definition points into the generated
//! `.aye` file in the cache (`$AYE_CACHE_DIR/cimport`), which is made if the
//! header was not imported before. Without libclang and a cached import
//! there is nothing to show.

use super::utf16_col;
use crate::cimport;
use crate::diag::json_str;
use crate::doc::{self, body, code_lines, doc_above, flat, matching, split};
use crate::driver;
use crate::fmt::{self, Br, Item, Role, Scan};
use crate::lexer::{Kw, Tok};
use crate::manifest::{self, ManifestFile, Source};
use crate::module::STD_FILES;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// A declaration: where its name is, what it looks like and what its `///`
/// comment says.
#[derive(Clone)]
struct Decl {
    name: String,
    /// The name's line and column, 1-based; a column counts characters.
    line: u32,
    col: u32,
    is_pub: bool,
    /// The declaration without its body, as Aye code.
    sig: String,
    /// The `///` lines above it, without the slashes.
    doc: String,
    /// The fields, variants and methods of a struct, enum or interface.
    members: Vec<Decl>,
}

/// A source file with its declarations.
struct Src {
    /// `None` for a file that has no place on disk.
    uri: Option<String>,
    text: String,
    decls: Vec<Decl>,
    /// The `import`s as (the name they bind, the path).
    imports: Vec<(String, Vec<String>)>,
    /// The `let c = @cimport("header", flags...)` declarations: (name,
    /// header, flags).
    cimports: Vec<(String, String, Vec<String>)>,
    /// The libraries of `@link` and `@linkOptional`.
    links: Vec<String>,
    /// The directory of the file (relative headers start there); empty if
    /// unknown.
    dir: PathBuf,
}

/// What a name stands for.
#[derive(Clone)]
enum Found {
    /// A module: its path, and the URI of its first file.
    Module(String, Option<String>),
    Item(Rc<Src>, Decl),
    /// The module that `@cimport` made of a header: the generated source,
    /// which is a file in the cache.
    Generated(Rc<Src>),
}

/// The hover for the position, or `null`: a Markdown value with the
/// declaration and its doc comment, and the range of the name.
pub fn hover(uri: &str, path: &str, text: &str, line: u32, character: u32) -> String {
    let Some((range, found)) = lookup(uri, path, text, line, character) else {
        return "null".to_string();
    };
    let value = match &found {
        Found::Item(_, d) if d.doc.is_empty() => format!("```aye\n{}\n```", d.sig),
        Found::Item(_, d) => format!("```aye\n{}\n```\n\n{}", d.sig, d.doc),
        Found::Module(m, _) => format!("```aye\nimport {m}\n```"),
        Found::Generated(_) => "```aye\n@cimport\n```".to_string(),
    };
    format!(
        r#"{{"contents":{{"kind":"markdown","value":{}}},"range":{range}}}"#,
        json_str(&value)
    )
}

/// The `Location` of the declaration that the position's name stands for,
/// or `null`.
pub fn definition(uri: &str, path: &str, text: &str, line: u32, character: u32) -> String {
    let Some((_, found)) = lookup(uri, path, text, line, character) else {
        return "null".to_string();
    };
    let (target, range) = match &found {
        Found::Generated(src) => (src.uri.as_ref(), Some(range_json(0, 0, 0))),
        Found::Module(_, uri) => (
            uri.as_ref(),
            Some(
                r#"{"start":{"line":0,"character":0},"end":{"line":0,"character":0}}"#.to_string(),
            ),
        ),
        Found::Item(src, d) => {
            let line_text = src
                .text
                .split('\n')
                .nth(d.line as usize - 1)
                .unwrap_or("")
                .trim_end_matches('\r');
            let start = d.col as usize - 1;
            let len = d.name.chars().count();
            let range = range_json(
                d.line - 1,
                utf16_col(line_text, start),
                utf16_col(line_text, start + len),
            );
            (src.uri.as_ref(), Some(range))
        }
    };
    match (target, range) {
        (Some(uri), Some(range)) => format!(r#"{{"uri":{},"range":{range}}}"#, json_str(uri)),
        _ => "null".to_string(),
    }
}

fn range_json(line: u32, start: u32, end: u32) -> String {
    format!(
        r#"{{"start":{{"line":{line},"character":{start}}},"end":{{"line":{line},"character":{end}}}}}"#
    )
}

/// The number of characters of `line` before the UTF-16 offset `utf16`.
fn char_index(line: &str, utf16: u32) -> usize {
    let mut units = 0;
    for (i, c) in line.chars().enumerate() {
        if units >= utf16 {
            return i;
        }
        units += c.len_utf16() as u32;
    }
    line.chars().count()
}

/// The identifier at the position (as the JSON range of the name) and what
/// it stands for.
fn lookup(uri: &str, path: &str, text: &str, line: u32, character: u32) -> Option<(String, Found)> {
    let scan = fmt::scan(text).ok()?;
    let items = &scan.items;
    let line_text = text.split('\n').nth(line as usize)?.trim_end_matches('\r');
    let at = char_index(line_text, character);
    let t = items.iter().position(|it| {
        let start = it.col as usize - 1;
        matches!(it.tok, Tok::Ident(_))
            && it.line == line + 1
            && start <= at
            && at <= start + it.text.chars().count()
    })?;
    let start = items[t].col as usize - 1;
    let range = range_json(
        line,
        utf16_col(line_text, start),
        utf16_col(line_text, start + items[t].text.chars().count()),
    );
    let base = Path::new(path)
        .parent()
        .unwrap_or(Path::new(""))
        .to_path_buf();
    let mut me = index(&scan, text, Some(uri.to_string()));
    me.dir = base.clone();
    let me = Rc::new(me);
    if t >= 1
        && items[t - 1].tok == Tok::Dot
        && let Some(found) = typed(path, text, &items[t], &me)
    {
        return Some((range, found));
    }
    let r = Resolver {
        deps: path_deps(&base),
        base,
    };
    // `a.b.` before the name.
    let mut chain: Vec<&str> = Vec::new();
    let mut k = t;
    while k >= 2 && items[k - 1].tok == Tok::Dot && matches!(items[k - 2].tok, Tok::Ident(_)) {
        chain.insert(0, items[k - 2].text.as_str());
        k -= 2;
    }
    // After a dot that does not follow a name (`f().x`, `.Variant`).
    if k >= 1 && items[k - 1].tok == Tok::Dot {
        return None;
    }
    let found = if chain.is_empty() {
        at_name(&me, &items[t])
            .or_else(|| local(items, t).map(|d| Found::Item(me.clone(), d)))
            .or_else(|| r.named(&me, &items[t].text))
    } else if local(items, k).is_some() {
        None
    } else {
        let mut found = r.named(&me, chain[0])?;
        for seg in chain[1..].iter().copied().chain([items[t].text.as_str()]) {
            found = r.member(&found, seg)?;
        }
        Some(found)
    };
    Some((range, found?))
}

/// The declaration (top-level or member) whose name is the token `it`.
fn at_name(src: &Rc<Src>, it: &Item) -> Option<Found> {
    decl_at(src, it.line, it.col).map(|d| Found::Item(src.clone(), d))
}

/// The declaration (top-level or member) whose name is at `line`, `col`.
fn decl_at(src: &Src, line: u32, col: u32) -> Option<Decl> {
    src.decls
        .iter()
        .flat_map(|d| std::iter::once(d).chain(&d.members))
        .find(|d| d.line == line && d.col == col)
        .cloned()
}

/// The member that sema resolved the name `it` after a dot in the document
/// (`path`, text `text`, declarations `me`) to: its declaration in the file
/// that declares it; sema's own rendering of it for a member the tokens do
/// not show as one (in a type function's struct).
fn typed(path: &str, text: &str, it: &Item, me: &Rc<Src>) -> Option<Found> {
    let mut root = super::root(path, text);
    root.members = true;
    let name = root.files[0].name.clone();
    let checked = driver::front_end(root, None);
    let file = checked.files.iter().position(|f| f.name == name)?;
    let (_, m) = checked
        .members
        .iter()
        .find(|(s, _)| s.file as usize == file && s.line == it.line && s.col == it.col)?;
    let at = m.decl;
    let src = if at.file as usize == file {
        me.clone()
    } else {
        let f = checked.files.get(at.file as usize)?;
        let disk = if f.std {
            Path::new(env!("CARGO_MANIFEST_DIR")).join(&f.name)
        } else {
            PathBuf::from(&f.name)
        };
        source(&f.text, disk.is_file().then(|| file_uri(&disk)))?
    };
    let decl = decl_at(&src, at.line, at.col).unwrap_or_else(|| {
        let doc = fmt::scan(&src.text)
            .map(|scan| strip_doc(&doc_above(&scan, &code_lines(&scan), at.line)))
            .unwrap_or_default();
        Decl {
            name: it.text.clone(),
            line: at.line,
            col: at.col,
            is_pub: true,
            sig: m.sig.clone(),
            doc,
            members: Vec::new(),
        }
    });
    Some(Found::Item(src, decl))
}

/// The path dependencies of the `aye.toml` in `dir`: (key, directory).
fn path_deps(dir: &Path) -> Vec<(String, PathBuf)> {
    let Ok(Some(mf)) = ManifestFile::read(dir) else {
        return Vec::new();
    };
    let Ok(m) = manifest::parse(&mf.file.text) else {
        return Vec::new();
    };
    m.deps
        .into_iter()
        .filter_map(|d| match d.source {
            Source::Path(p) => {
                let dir = mf.dir.join(p);
                Some((d.key, std::fs::canonicalize(&dir).unwrap_or(dir)))
            }
            Source::Url(_) => None,
        })
        .collect()
}

/// Finds modules and what they declare.
struct Resolver {
    /// The directory that `import` paths start from.
    base: PathBuf,
    /// The path dependencies: (key, directory).
    deps: Vec<(String, PathBuf)>,
}

impl Resolver {
    /// The files of the module `path`.
    fn module_files(&self, path: &str) -> Vec<Rc<Src>> {
        let mut out = Vec::new();
        if path == "std" || path.starts_with("std.") {
            for (module, name, text) in STD_FILES {
                if *module == path {
                    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join(name);
                    let uri = file.is_file().then(|| file_uri(&file));
                    out.extend(source(text, uri));
                }
            }
            return out;
        }
        let first = path.split('.').next().unwrap_or("");
        let dir = match self.deps.iter().find(|(key, _)| key == first) {
            Some((_, dep)) => {
                let rest = path.split_once('.').map_or("", |(_, r)| r);
                dep.join(rest.replace('.', "/"))
            }
            None => self.base.join(path.replace('.', "/")),
        };
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return out;
        };
        let mut files: Vec<PathBuf> = entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "aye"))
            .collect();
        files.sort();
        for f in files {
            if let Ok(text) = std::fs::read_to_string(&f) {
                out.extend(source(&text, Some(file_uri(&f))));
            }
        }
        out
    }

    fn module(&self, path: &str) -> Option<Found> {
        let files = self.module_files(path);
        if files.is_empty() && path != "std" {
            return None;
        }
        let uri = files.first().and_then(|s| s.uri.clone());
        Some(Found::Module(path.to_string(), uri))
    }

    /// `name` in `me`: a top-level declaration, else an import.
    fn named(&self, me: &Rc<Src>, name: &str) -> Option<Found> {
        if let Some(d) = me.decls.iter().find(|d| d.name == name) {
            return Some(Found::Item(me.clone(), d.clone()));
        }
        let (_, path) = me.imports.iter().find(|(alias, _)| alias == name)?;
        let mut found = self.module(&path[0])?;
        for seg in &path[1..] {
            found = self.member(&found, seg)?;
        }
        Some(found)
    }

    /// `from.seg`.
    fn member(&self, from: &Found, seg: &str) -> Option<Found> {
        match from {
            Found::Module(m, _) => {
                for src in self.module_files(m) {
                    if let Some(d) = src.decls.iter().find(|d| d.name == seg && d.is_pub) {
                        return Some(Found::Item(src.clone(), d.clone()));
                    }
                }
                self.module(&format!("{m}.{seg}"))
            }
            Found::Item(src, d) => match src.cimports.iter().find(|c| c.0 == d.name) {
                Some(c) => generated(src, c).and_then(|g| self.member(&Found::Generated(g), seg)),
                None => d
                    .members
                    .iter()
                    .find(|x| x.name == seg)
                    .map(|x| Found::Item(src.clone(), x.clone())),
            },
            Found::Generated(g) => g
                .decls
                .iter()
                .find(|d| d.name == seg && d.is_pub)
                .map(|d| Found::Item(g.clone(), d.clone())),
        }
    }
}

/// The module that `@cimport` makes of `c` in `src`: its generated source,
/// from the cache (made if the header was not imported before; none if
/// libclang is missing or the header is wrong).
fn generated(src: &Src, c: &(String, String, Vec<String>)) -> Option<Rc<Src>> {
    if src.dir.as_os_str().is_empty() {
        return None;
    }
    let req = cimport::Request {
        header: c.1.clone(),
        flags: c.2.clone(),
        dir: src.dir.clone(),
        links: src.links.clone(),
    };
    let imp = cimport::import(&req).ok()?;
    source(&imp.source, Some(file_uri(&imp.path)))
}

/// The source `text` and its declarations; none if it does not lex.
fn source(text: &str, uri: Option<String>) -> Option<Rc<Src>> {
    let scan = fmt::scan(text).ok()?;
    Some(Rc::new(index(&scan, text, uri)))
}

/// `file:` URI of an absolute (or working-directory relative) path.
fn file_uri(path: &Path) -> String {
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default().join(path)
    };
    let mut out = String::from("file://");
    for b in abs.to_string_lossy().bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~' | b'/') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn index(scan: &Scan, text: &str, uri: Option<String>) -> Src {
    let items = &scan.items;
    let has_code = code_lines(scan);
    let mut decls = Vec::new();
    let mut imports = Vec::new();
    let mut cimports = Vec::new();
    let mut links = Vec::new();
    for (s, e) in split(items, 0, items.len()) {
        let d = &items[s..e];
        let strs = || {
            d.iter()
                .filter_map(|it| match &it.tok {
                    Tok::Str(b) => Some(String::from_utf8_lossy(b).into_owned()),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        if let Tok::Builtin(b) = &d[0].tok
            && (b == "link" || b == "linkOptional")
        {
            links.extend(strs());
        } else if d.len() > 3
            && d[0].tok == Tok::Kw(Kw::Let)
            && d[3].tok == Tok::Builtin("cimport".into())
            && let Tok::Ident(name) = &d[1].tok
        {
            let mut a = strs().into_iter();
            if let Some(h) = a.next() {
                cimports.push((name.clone(), h, a.collect()));
            }
        }
        if d[0].tok == Tok::Kw(Kw::Import) {
            let path: Vec<String> = d[1..]
                .iter()
                .filter(|it| matches!(it.tok, Tok::Ident(_)))
                .map(|it| it.text.clone())
                .collect();
            if let Some(alias) = path.last() {
                imports.push((alias.clone(), path.clone()));
            }
        } else if let Some(decl) = declaration(scan, &has_code, d) {
            decls.push(decl);
        }
    }
    Src {
        uri,
        text: text.to_string(),
        decls,
        imports,
        cimports,
        links,
        dir: PathBuf::new(),
    }
}

/// The top-level declaration `d` (its tokens), if it declares a name.
fn declaration(scan: &Scan, has_code: &[bool], d: &[Item]) -> Option<Decl> {
    let mut k = 0;
    while k < d.len() && matches!(d[k].tok, Tok::Kw(Kw::Pub | Kw::Extern)) {
        k += 1;
        // `extern("name") fn`
        if d[k - 1].tok == Tok::Kw(Kw::Extern)
            && d.get(k).is_some_and(|it| it.role == Role::Open(Br::Call))
        {
            k = matching(d, k) + 1;
        }
    }
    let kw = &d.get(k)?.tok;
    let is_type = match kw {
        Tok::Kw(Kw::Struct | Kw::Enum | Kw::Interface) => true,
        Tok::Ident(u) => u == "union",
        _ => false,
    };
    let is_fn = *kw == Tok::Kw(Kw::Fn);
    if !is_type && !is_fn && !matches!(kw, Tok::Kw(Kw::Error | Kw::Let | Kw::Var)) {
        return None;
    }
    let name = d.get(k + 1).filter(|it| matches!(it.tok, Tok::Ident(_)))?;
    let body = body(d);
    let (sig, members) = match body {
        Some(b) if is_fn => (flat(&d[..b.0]), Vec::new()),
        Some(b) if is_type => {
            let interface = *kw == Tok::Kw(Kw::Interface);
            let found = doc::members(scan, has_code, d, b.0 + 1, b.1);
            let mut lines = String::new();
            doc::render_members(&found, interface, "    ", &mut lines);
            let members = found
                .iter()
                .filter_map(|m| {
                    let n = m.name()?;
                    Some(Decl {
                        name: n.text.clone(),
                        line: n.line,
                        col: n.col,
                        is_pub: m.is_pub,
                        sig: m.sig.clone(),
                        doc: strip_doc(&m.doc),
                        members: Vec::new(),
                    })
                })
                .collect();
            let sig = if d[0].line != d[d.len() - 1].line {
                format!("{} {{\n{lines}}}", flat(&d[..b.0]))
            } else {
                flat(d)
            };
            (sig, members)
        }
        _ if matches!(kw, Tok::Kw(Kw::Let | Kw::Var)) && d[0].line != d[d.len() - 1].line => {
            let first: Vec<&Item> = d.iter().take_while(|it| it.line == d[0].line).collect();
            (format!("{} ...", fmt::join(first)), Vec::new())
        }
        _ => (flat(d), Vec::new()),
    };
    Some(Decl {
        name: name.text.clone(),
        line: name.line,
        col: name.col,
        is_pub: d[0].tok == Tok::Kw(Kw::Pub),
        sig,
        doc: strip_doc(&doc_above(scan, has_code, d[0].line)),
        members,
    })
}

/// The local named like token `t`, declared above it in the function that
/// encloses it, whose scope `t` is in: a parameter, or a name after `let`,
/// `var`, `for` or in a capture.
fn local(items: &[Item], t: usize) -> Option<Decl> {
    let name = &items[t].text;
    // The innermost function: its `fn`, the `)` of its parameters, its `}`.
    let (f, close) = (0..items.len())
        .filter_map(|f| {
            let name = items.get(f + 1)?;
            let open = items.get(f + 2)?;
            if items[f].tok != Tok::Kw(Kw::Fn)
                || !matches!(name.tok, Tok::Ident(_))
                || open.role != Role::Open(Br::FnParams)
            {
                return None;
            }
            let close = matching(items, f + 2);
            // The body: a `{` after the return type.
            let mut j = close + 1;
            while j < items.len() && !matches!(items[j].role, Role::Semi | Role::Comma) {
                match items[j].role {
                    Role::Open(Br::Brace) => {
                        let end = matching(items, j);
                        return (f < t && t <= end).then_some((f, close));
                    }
                    Role::Open(_) => j = matching(items, j),
                    _ => {}
                }
                j += 1;
            }
            None
        })
        .max_by_key(|&(f, _)| f)?;
    let mut depth = 0usize;
    let mut found = Vec::new();
    for k in f + 2..=t {
        match items[k].role {
            Role::Open(_) => depth += 1,
            Role::Close(_) => depth = depth.saturating_sub(1),
            _ => {}
        }
        if items[k].text != *name || !matches!(items[k].tok, Tok::Ident(_)) {
            continue;
        }
        let prev = &items[k - 1];
        let binds = if k < close {
            depth == 1 && items.get(k + 1).is_some_and(|n| n.tok == Tok::Colon)
        } else {
            matches!(prev.tok, Tok::Kw(Kw::Let | Kw::Var | Kw::For)) || prev.role == Role::CapOpen
        };
        if binds {
            found.push(k);
        }
    }
    let k = found
        .into_iter()
        .rev()
        .find(|&k| k < close || in_scope(items, k, t))?;
    let sig = if k < close {
        // `name: T`, up to the comma or the `)`.
        let start = if items[k - 1].tok == Tok::Kw(Kw::Comptime) {
            k - 1
        } else {
            k
        };
        let mut depth = 0usize;
        let mut end = k;
        while end < items.len() {
            match items[end].role {
                Role::Open(_) => depth += 1,
                Role::Close(_) if depth == 0 => break,
                Role::Close(_) => depth -= 1,
                Role::Comma if depth == 0 => break,
                _ => {}
            }
            end += 1;
        }
        flat(&items[start..end])
    } else if items[k - 1].role == Role::CapOpen {
        format!("|{name}|")
    } else {
        // The statement's first line, without the `{` that opens its block.
        let line = items[k - 1].line;
        let end = items[k - 1..]
            .iter()
            .position(|it| it.role == Role::Semi || it.line != line)
            .map_or(items.len(), |p| k - 1 + p);
        let mut stmt = &items[k - 1..end];
        if stmt
            .last()
            .is_some_and(|it| it.role == Role::Open(Br::Brace))
        {
            stmt = &stmt[..stmt.len() - 1];
        }
        flat(stmt)
    };
    Some(Decl {
        name: name.clone(),
        line: items[k].line,
        col: items[k].col,
        is_pub: false,
        sig,
        doc: String::new(),
        members: Vec::new(),
    })
}

/// Is the token `t` in the scope of the local named at `k` (after the
/// parameters)? A `for` name's, and that of a capture before a `{`, is that
/// block; a `let` or `var` name's the rest of its block.
fn in_scope(items: &[Item], k: usize, t: usize) -> bool {
    let in_block = |b: Option<usize>| b.is_some_and(|b| b < t && t <= matching(items, b));
    if items[k - 1].tok == Tok::Kw(Kw::For) {
        return in_block((k..t).find(|&j| items[j].role == Role::Open(Br::Brace)));
    }
    if items[k - 1].role == Role::CapOpen
        && items.get(k + 1).is_some_and(|it| it.role == Role::CapClose)
        && items
            .get(k + 2)
            .is_some_and(|it| it.role == Role::Open(Br::Brace))
    {
        return in_block(Some(k + 2));
    }
    let mut depth = 0i32;
    for it in &items[k..t] {
        match it.role {
            Role::Open(_) => depth += 1,
            Role::Close(_) => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            _ => {}
        }
    }
    true
}

/// The text of `///` lines.
fn strip_doc(lines: &[String]) -> String {
    lines
        .iter()
        .map(|l| {
            let rest = l.strip_prefix("///").unwrap_or(l);
            rest.strip_prefix(' ').unwrap_or(rest)
        })
        .collect::<Vec<_>>()
        .join("\n")
}
