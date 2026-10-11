//! Hover and go to definition for `aye lsp`.
//!
//! Both work on tokens (like `aye fmt` and `aye doc`), not on the typed
//! program: sema keeps no definition spans, resolves names while it lowers
//! the program, and gives nothing at all for a document with a type error,
//! which is what a document being typed usually has. So a name is resolved
//! here, from the document's own tokens and the files of the modules it
//! imports, in this order:
//!
//! - a declaration whose name is the token itself (a method or a field in a
//!   struct, say),
//! - a local of the enclosing function: its parameter, or the `let`, `var`,
//!   `for` or capture name nearest above (block scopes are not tracked),
//! - a top-level declaration of the document's own file,
//! - an `import` of the document: a module, or an item of a module.
//!
//! After a dot, `a.b.c` is resolved from the left: `a` as above (never a
//! local, whose type is not known), then each name as a pub declaration or
//! submodule of a module, or a member (method, field, variant) of a struct,
//! enum or interface. Anything else, such as the method of a value, has no
//! answer.
//!
//! Modules are the `.aye` files of the directory `import a.b` names under the
//! document's directory, or std. std is embedded in the compiler; its files
//! are located under this compiler's source tree (`CARGO_MANIFEST_DIR`) if
//! that is still on disk; if not, hover works and definition gives `null`.
//! Dependencies of `aye.toml` are not searched.

use super::utf16_col;
use crate::diag::json_str;
use crate::fmt::{self, Br, Item, Role, Scan};
use crate::lexer::{Kw, Tok};
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
}

/// What a name stands for.
#[derive(Clone)]
enum Found {
    /// A module: its path, and the URI of its first file.
    Module(String, Option<String>),
    Item(Rc<Src>, Decl),
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
            && at < start + it.text.chars().count()
    })?;
    let start = items[t].col as usize - 1;
    let range = range_json(
        line,
        utf16_col(line_text, start),
        utf16_col(line_text, start + items[t].text.chars().count()),
    );
    let me = Rc::new(index(&scan, text, Some(uri.to_string())));
    let r = Resolver {
        base: Path::new(path)
            .parent()
            .unwrap_or(Path::new(""))
            .to_path_buf(),
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
    src.decls
        .iter()
        .flat_map(|d| std::iter::once(d).chain(&d.members))
        .find(|d| d.line == it.line && d.col == it.col)
        .map(|d| Found::Item(src.clone(), d.clone()))
}

/// Finds modules and what they declare.
struct Resolver {
    /// The directory that `import` paths start from.
    base: PathBuf,
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
        let dir = self.base.join(path.replace('.', "/"));
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
            Found::Item(src, d) => d
                .members
                .iter()
                .find(|x| x.name == seg)
                .map(|x| Found::Item(src.clone(), x.clone())),
        }
    }
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
    for (s, e) in split(items, 0, items.len()) {
        let d = &items[s..e];
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
            let (lines, members) = members(scan, has_code, d, b.0 + 1, b.1, interface);
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

/// The members of a struct, enum or interface body `d[from..to]`, and the
/// lines that show them in the declaration: fields and variants, and the
/// signatures of methods (only the pub ones unless `all_methods`), each
/// with its doc comment.
fn members(
    scan: &Scan,
    has_code: &[bool],
    d: &[Item],
    from: usize,
    to: usize,
    all_methods: bool,
) -> (String, Vec<Decl>) {
    let mut out = String::new();
    let mut found = Vec::new();
    for (s, e) in split(d, from, to) {
        let m = &d[s..e];
        let is_pub = m[0].tok == Tok::Kw(Kw::Pub);
        let at_fn = m.iter().take(2).position(|it| it.tok == Tok::Kw(Kw::Fn));
        // A doc comment belongs to a member that starts its line.
        let doc = if s == 0 || d[s - 1].role == Role::Semi || d[s - 1].line != m[0].line {
            doc_above(scan, has_code, m[0].line)
        } else {
            Vec::new()
        };
        let sig = match (at_fn, body(m)) {
            (Some(_), Some((b, _))) => flat(&m[..b]),
            _ => flat(m),
        };
        let name = match at_fn {
            Some(f) => m.get(f + 1),
            None => m.iter().find(|it| matches!(it.tok, Tok::Ident(_))),
        };
        if let Some(n) = name.filter(|n| matches!(n.tok, Tok::Ident(_))) {
            found.push(Decl {
                name: n.text.clone(),
                line: n.line,
                col: n.col,
                is_pub,
                sig: sig.clone(),
                doc: strip_doc(&doc),
                members: Vec::new(),
            });
        }
        if at_fn.is_some() && !is_pub && !all_methods {
            continue;
        }
        for c in doc {
            out.push_str(&format!("    {c}\n"));
        }
        out.push_str(&format!("    {sig}\n"));
    }
    (out, found)
}

/// The local named like token `t`, declared above it in the function that
/// encloses it: a parameter, or a name after `let`, `var`, `for` or in a
/// capture.
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
    let mut found = None;
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
            found = Some(k);
        }
    }
    let k = found?;
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

/// The lines (1-based) that have tokens.
fn code_lines(scan: &Scan) -> Vec<bool> {
    let mut has = vec![false; scan.comments.len() + 1];
    for it in scan.items.iter().filter(|it| it.role != Role::Semi) {
        has[it.line as usize] = true;
    }
    has
}

/// The `///` comment lines right above `line`, in order.
fn doc_above(scan: &Scan, has_code: &[bool], line: u32) -> Vec<String> {
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

/// The index of the bracket that closes the one opened at `open`; the end
/// of `items` if there is none.
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

/// Tokens on one line, without the trailing comma of a multi-line list.
fn flat(items: &[Item]) -> String {
    let keep = (0..items.len())
        .filter(|&i| {
            let next = items[i + 1..].iter().find(|it| it.role != Role::Semi);
            !(items[i].role == Role::Comma
                && next.is_some_and(|n| matches!(n.role, Role::Close(_))))
        })
        .map(|i| &items[i]);
    fmt::join(keep)
}
