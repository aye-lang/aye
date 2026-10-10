//! Modules and name resolution.
//!
//! A module is a directory: every `.aye` file in it shares one namespace,
//! and only its `pub` items are visible to other modules. The root module
//! is a directory, or a single file. `import a.b` finds the module in the
//! directory `a/b` under the root module's directory, except that `std`
//! and its modules are embedded in the compiler.
//!
//! This pass loads the root module and every module it uses, checks
//! imports, `pub` visibility and import cycles, and rewrites each name that
//! refers to an item of a module other than the root to its canonical name,
//! `std::fs::open` or `util::Thing` (sema shows `::` as `.`). Names of the
//! root module stay as they are written. The result is one AST for sema:
//! the root module's items first, then the other modules' items. The
//! `test` blocks of every module but std's are kept. For `aye test`, each
//! ```aye block of a `///` doc comment outside std is a `test` too.
//!
//! Imports are per file: `import std.fs` binds `fs` in its file only. A
//! path such as `std.fs.open` is resolved one name at a time: a module's
//! item, or else its submodule (`std.fs` within `std`).

use crate::ast::{self, ArmBody, ExprKind, Item as AstItem, StmtKind, TypeExprKind};
use crate::codes::*;
use crate::diag::{Error, Span};
use crate::{cimport, lexer, parser};
use std::collections::HashMap;
use std::path::Path;

/// A source file of the program.
pub struct SourceFile {
    /// The name in diagnostics, `#line` directives and panic messages.
    pub name: String,
    pub text: String,
    /// Part of the embedded std (`aye fix` never writes it).
    pub std: bool,
}

/// std, embedded: (module, file name, source).
pub const STD_FILES: &[(&str, &str, &str)] = &[
    (
        "std.coro",
        "std/coro/coro.aye",
        include_str!("../std/coro/coro.aye"),
    ),
    (
        "std.crypto",
        "std/crypto/crypto.aye",
        include_str!("../std/crypto/crypto.aye"),
    ),
    (
        "std.diag",
        "std/diag/diag.aye",
        include_str!("../std/diag/diag.aye"),
    ),
    ("std.fs", "std/fs/fs.aye", include_str!("../std/fs/fs.aye")),
    (
        "std.heap",
        "std/heap/heap.aye",
        include_str!("../std/heap/heap.aye"),
    ),
    (
        "std.http",
        "std/http/http.aye",
        include_str!("../std/http/http.aye"),
    ),
    (
        "std.json",
        "std/json/json.aye",
        include_str!("../std/json/json.aye"),
    ),
    (
        "std.list",
        "std/list/list.aye",
        include_str!("../std/list/list.aye"),
    ),
    (
        "std.log",
        "std/log/log.aye",
        include_str!("../std/log/log.aye"),
    ),
    (
        "std.map",
        "std/map/map.aye",
        include_str!("../std/map/map.aye"),
    ),
    (
        "std.math",
        "std/math/math.aye",
        include_str!("../std/math/math.aye"),
    ),
    (
        "std.mem",
        "std/mem/mem.aye",
        include_str!("../std/mem/mem.aye"),
    ),
    (
        "std.net",
        "std/net/net.aye",
        include_str!("../std/net/net.aye"),
    ),
    ("std.os", "std/os/os.aye", include_str!("../std/os/os.aye")),
    (
        "std.path",
        "std/path/path.aye",
        include_str!("../std/path/path.aye"),
    ),
    (
        "std.pool",
        "std/pool/pool.aye",
        include_str!("../std/pool/pool.aye"),
    ),
    (
        "std.process",
        "std/process/process.aye",
        include_str!("../std/process/process.aye"),
    ),
    (
        "std.sort",
        "std/sort/sort.aye",
        include_str!("../std/sort/sort.aye"),
    ),
    (
        "std.sync",
        "std/sync/sync.aye",
        include_str!("../std/sync/sync.aye"),
    ),
    (
        "std.testing",
        "std/testing/testing.aye",
        include_str!("../std/testing/testing.aye"),
    ),
    (
        "std.text",
        "std/text/text.aye",
        include_str!("../std/text/text.aye"),
    ),
    (
        "std.thread",
        "std/thread/thread.aye",
        include_str!("../std/thread/thread.aye"),
    ),
    (
        "std.time",
        "std/time/time.aye",
        include_str!("../std/time/time.aye"),
    ),
];

/// Items of std that the compiler provides (their declarations would need
/// features Aye does not have yet): (module, name).
const BUILTINS: &[(&str, &str)] = &[
    ("std.mem", "Allocator"),
    ("std.mem", "AllocError"),
    ("std.heap", "General"),
    ("std.heap", "Arena"),
    ("std.log", "err"),
    ("std.log", "info"),
    ("std.testing", "allocator"),
];

/// The methods of the builtin types, for `std_api`.
const BUILTIN_METHODS: &[(&str, &str)] = &[
    ("std.heap", "General.init"),
    ("std.heap", "General.allocator"),
    ("std.heap", "General.deinit"),
    ("std.heap", "Arena.init"),
    ("std.heap", "Arena.allocator"),
    ("std.heap", "Arena.deinit"),
    ("std.mem", "Allocator.create"),
    ("std.mem", "Allocator.destroy"),
    ("std.mem", "Allocator.alloc"),
    ("std.mem", "Allocator.free"),
    ("std.mem", "Allocator.resize"),
];

/// The canonical name of the item `name` of the module `module` (`""` is
/// the root module).
pub fn canonical(module: &str, name: &str) -> String {
    if module.is_empty() {
        name.to_string()
    } else {
        format!("{}::{name}", module.replace('.', "::"))
    }
}

/// The pub functions and methods of std as (module, name), the name as the
/// spec writes it: (`std.fs`, `std.fs.open`), (`std.fs`, `File.readAll`),
/// (`std.log`, `std.log.err`), (`std.heap`, `General.init`).
pub fn std_api() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (module, _, src) in STD_FILES {
        let file =
            parser::parse(lexer::lex(src).unwrap()).unwrap_or_else(|e| panic!("{module}: {e:?}"));
        let mut push = |name: String| out.push((module.to_string(), name));
        for item in &file.items {
            match item {
                AstItem::Fn(f) if f.is_pub => {
                    push(format!("{module}.{}", f.name));
                    // A type function's methods: `List.append`.
                    if let Some(ast::Stmt {
                        kind: StmtKind::Return(Some(e)),
                        ..
                    }) = f.body.as_ref().and_then(|b| b.stmts.first())
                        && let ExprKind::StructType(s) = &e.kind
                    {
                        s.methods
                            .iter()
                            .filter(|m| m.is_pub)
                            .for_each(|m| push(format!("{}.{}", f.name, m.name)));
                    }
                }
                AstItem::Struct(s) if s.is_pub => s
                    .methods
                    .iter()
                    .filter(|m| m.is_pub)
                    .for_each(|m| push(format!("{}.{}", s.name, m.name))),
                _ => {}
            }
        }
    }
    let builtins = BUILTINS
        .iter()
        .filter(|(_, n)| n.starts_with(char::is_lowercase));
    out.extend(builtins.map(|(m, n)| (m.to_string(), format!("{m}.{n}"))));
    out.extend(
        BUILTIN_METHODS
            .iter()
            .map(|(m, n)| (m.to_string(), n.to_string())),
    );
    out
}

/// The std module of the builtin `name` when a module uses it without an
/// import (`Allocator` is `std.mem.Allocator`).
fn std_home(name: &str) -> Option<&'static str> {
    BUILTINS
        .iter()
        .find(|(_, n)| *n == name && n.starts_with(char::is_uppercase))
        .map(|(m, _)| *m)
}

/// What `mod.Name { ... }` is: a struct literal when `Name` is a struct or
/// a top-level `let` (an alias of a struct), else a variant literal.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Struct,
    Other,
}

struct Decl {
    is_pub: bool,
    kind: Kind,
    span: Span,
}

struct Module {
    /// `""` for the root, `util`, `std.fs`.
    path: String,
    files: Vec<usize>,
    items: HashMap<String, Decl>,
    /// `let c = @cimport(...)` declarations, with their files.
    cimports: Vec<(ast::CImportDecl, usize)>,
    /// The libraries of `@link` and `@linkOptional` (`@cimport` uses their
    /// `pkg-config --cflags`).
    links: Vec<String>,
    /// A file of the module has syntax errors: names in it are not known,
    /// so lookups that fail are not reported.
    broken: bool,
}

#[derive(Clone)]
enum Binding {
    Module(usize),
    /// Canonical name, kind.
    Item(String, Kind),
    /// The import has an error.
    Bad,
}

enum Res {
    Module(usize),
    Item(String, Kind),
    /// Not a top-level name or an import: a local, a type parameter, a
    /// predeclared name, or unknown (sema decides).
    Unresolved,
}

/// An error that was already reported.
fn silent() -> Error {
    Error::new(Code(0), Span::default(), "")
}

/// `std.fs.File` at `span`: each name with its position.
fn split_path(path: &str, span: Span) -> Vec<(String, Span)> {
    let mut col = span.col;
    path.split('.')
        .map(|s| {
            let at = Span { col, ..span };
            col += s.chars().count() as u32 + 1;
            (s.to_string(), at)
        })
        .collect()
}

/// `a.b.c` written as an identifier and field accesses.
fn expr_path(e: &ast::Expr) -> Option<Vec<(String, Span)>> {
    match &e.kind {
        ExprKind::Ident(n) => Some(vec![(n.clone(), e.span)]),
        ExprKind::Field { base, name } => {
            let mut p = expr_path(base)?;
            p.push((name.clone(), e.span));
            Some(p)
        }
        _ => None,
    }
}

fn join(segs: &[(String, Span)]) -> String {
    segs.iter()
        .map(|(s, _)| s.as_str())
        .collect::<Vec<_>>()
        .join(".")
}

/// `segs[..n]` as an identifier with field accesses after it.
fn rebuild(first: String, span: Span, rest: &[(String, Span)]) -> ast::Expr {
    let mut e = ast::Expr {
        kind: ExprKind::Ident(first),
        span,
    };
    for (name, sp) in rest {
        e = ast::Expr {
            kind: ExprKind::Field {
                base: Box::new(e),
                name: name.clone(),
            },
            span: *sp,
        };
    }
    e
}

struct Loader<'a> {
    /// The root module's directory: `import a.b` is `base/a/b`.
    base: &'a Path,
    /// Add the doc comments' examples as tests (`aye test`).
    doc_tests: bool,
    files: Vec<SourceFile>,
    /// Each file's syntax tree; `None` after a syntax error.
    asts: Vec<Option<ast::File>>,
    modules: Vec<Module>,
    by_path: HashMap<String, Option<usize>>,
    /// The modules made by `@cimport`, by request (`None` after an error).
    cimported: HashMap<String, Option<usize>>,
    /// The modules each module uses, with the first import or use.
    edges: Vec<Vec<(usize, Span)>>,
    errors: Vec<Error>,
    /// The module and file being resolved, and the file's imports.
    cur: usize,
    file: usize,
    imports: HashMap<String, (Binding, Span)>,
    last_import: Option<Span>,
    /// Import names that a local of the current function was given (an
    /// error): its uses are the local's, not reported again.
    shadowed: Vec<String>,
}

/// Load the root module (`root`, in the directory `base`) and every module
/// it uses, and resolve names. Returns every file of the program, indexed
/// by `Span::file`, and the program's AST or the errors.
pub fn load(
    root: Vec<SourceFile>,
    base: &Path,
    doc_tests: bool,
) -> (Vec<SourceFile>, Result<ast::File, Vec<Error>>) {
    let mut l = Loader {
        base,
        doc_tests,
        files: Vec::new(),
        asts: Vec::new(),
        modules: Vec::new(),
        by_path: HashMap::new(),
        cimported: HashMap::new(),
        edges: Vec::new(),
        errors: Vec::new(),
        cur: 0,
        file: 0,
        imports: HashMap::new(),
        last_import: None,
        shadowed: Vec::new(),
    };
    l.add_module(String::new(), root);
    let mut m = 0;
    while m < l.modules.len() {
        if !l.modules[m].broken {
            l.resolve_module(m);
        }
        m += 1;
    }
    l.check_cycles();
    if !l.errors.is_empty() {
        return (l.files, Err(l.errors));
    }
    let ast = l.merge();
    (l.files, Ok(ast))
}

/// A ```aye block of a `///` doc comment: the test's name, the line of
/// the opening fence, and each line of code with its line number and the
/// number of characters before the code (the comment's `/// `).
struct DocExample {
    name: String,
    fence: u32,
    lines: Vec<(u32, u32, String)>,
}

/// The ```aye blocks of the `///` doc comments of `text`, each named
/// after the declaration that the comment documents.
fn doc_examples(text: &str) -> Vec<DocExample> {
    let mut out: Vec<DocExample> = Vec::new();
    let mut open: Option<DocExample> = None;
    // The examples whose declaration has not been seen yet.
    let mut unnamed = 0;
    for (i, line) in text.lines().enumerate() {
        let n = i as u32 + 1;
        let doc = line
            .trim_start()
            .strip_prefix("///")
            .filter(|r| !r.starts_with('/'));
        let Some(rest) = doc else {
            open = None;
            if unnamed > 0 && !line.trim().is_empty() {
                let name = decl_name(line);
                let len = out.len();
                for e in &mut out[len - unnamed..] {
                    e.name = format!("doc: {name} (line {})", e.fence);
                }
                unnamed = 0;
            }
            continue;
        };
        let code = rest.strip_prefix(' ').unwrap_or(rest);
        let indent = (line.chars().count() - code.chars().count()) as u32;
        match open.as_mut() {
            Some(_) if code.trim() == "```" => {
                out.push(open.take().unwrap());
                unnamed += 1;
            }
            Some(e) => e.lines.push((n, indent, code.to_string())),
            None if code.trim() == "```aye" => {
                open = Some(DocExample {
                    name: format!("doc: line {n}"),
                    fence: n,
                    lines: Vec::new(),
                });
            }
            None => {}
        }
    }
    out
}

/// The name declared on `line` (`pub fn square(...)` is `square`).
fn decl_name(line: &str) -> String {
    let mut words = line
        .split_whitespace()
        .skip_while(|w| matches!(*w, "pub" | "extern"));
    let kw = words.next().unwrap_or("");
    let rest = if matches!(
        kw,
        "fn" | "struct" | "enum" | "interface" | "error" | "let" | "var" | "union"
    ) {
        words.next().unwrap_or("")
    } else {
        kw
    };
    let name: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    if name.is_empty() {
        "example".to_string()
    } else {
        name
    }
}

/// `toks` (the tokens of `text`) followed by a `test` block for each
/// example of the doc comments of `text`. The example's tokens keep their
/// place in the comment; the rest of the `test` is at the opening fence.
fn with_doc_tests(
    text: &str,
    mut toks: Vec<lexer::Token>,
) -> std::result::Result<Vec<lexer::Token>, Vec<Error>> {
    let Some(eof) = toks.pop() else {
        return Ok(toks);
    };
    for ex in doc_examples(text) {
        let code: Vec<&str> = ex.lines.iter().map(|(_, _, c)| c.as_str()).collect();
        let src = format!("test \"{}\" {{\n{}\n}}\n", ex.name, code.join("\n"));
        let place = |s: &mut Span| match ex.lines.get((s.line as usize).wrapping_sub(2)) {
            Some(&(line, indent, _)) => {
                *s = Span {
                    line,
                    col: s.col + indent,
                    file: s.file,
                }
            }
            None => {
                *s = Span {
                    line: ex.fence,
                    col: 1,
                    file: s.file,
                }
            }
        };
        let mut more = lexer::lex(&src).map_err(|errs| {
            errs.into_iter()
                .map(|mut e| {
                    place(&mut e.span);
                    e.fix = None;
                    e
                })
                .collect::<Vec<_>>()
        })?;
        more.pop();
        for t in &mut more {
            place(&mut t.span);
            place(&mut t.end);
        }
        toks.extend(more);
    }
    toks.push(eof);
    Ok(toks)
}

impl Loader<'_> {
    fn report(&mut self, e: Error) {
        if !e.msg.is_empty() {
            self.errors.push(e);
        }
    }

    /// Parse the files of a new module and register its items.
    fn add_module(&mut self, path: String, files: Vec<SourceFile>) -> usize {
        let m = self.modules.len();
        let mut module = Module {
            path: path.clone(),
            files: Vec::new(),
            items: HashMap::new(),
            cimports: Vec::new(),
            links: Vec::new(),
            broken: false,
        };
        for f in files {
            let id = self.files.len();
            let set = |s: &mut Span| s.file = id as u16;
            let doc_tests = self.doc_tests && !f.std;
            let parsed = lexer::lex(&f.text)
                .and_then(|toks| {
                    if doc_tests {
                        with_doc_tests(&f.text, toks)
                    } else {
                        Ok(toks)
                    }
                })
                .and_then(|mut toks| {
                    for t in &mut toks {
                        set(&mut t.span);
                        set(&mut t.end);
                    }
                    parser::parse(toks)
                })
                .map_err(|errs| {
                    errs.into_iter().map(|mut e| {
                        set(&mut e.span);
                        for edit in e.fix.iter_mut().flat_map(|f| &mut f.edits) {
                            set(&mut edit.start);
                            set(&mut edit.end);
                        }
                        e
                    })
                });
            let ast = match parsed {
                Ok(ast) => {
                    for item in &ast.items {
                        let (name, span, is_pub, kind) = match item {
                            AstItem::Fn(f) => (&f.name, f.span, f.is_pub, Kind::Other),
                            AstItem::Struct(s) => (&s.name, s.span, s.is_pub, Kind::Struct),
                            AstItem::Enum(e) => (&e.name, e.span, e.is_pub, Kind::Other),
                            AstItem::ErrorSet(e) => (&e.name, e.span, e.is_pub, Kind::Other),
                            AstItem::Interface(i) => (&i.name, i.span, i.is_pub, Kind::Other),
                            AstItem::Global(g) => (&g.name, g.span, g.is_pub, Kind::Struct),
                            AstItem::Link(l) => {
                                module.links.push(l.name.clone());
                                continue;
                            }
                            AstItem::CImport(c) => {
                                module.cimports.push((c.clone(), id));
                                continue;
                            }
                            AstItem::Test(_) => continue,
                        };
                        module
                            .items
                            .entry(name.clone())
                            .or_insert(Decl { is_pub, kind, span });
                    }
                    Some(ast)
                }
                Err(errs) => {
                    self.errors.extend(errs);
                    module.broken = true;
                    None
                }
            };
            module.files.push(id);
            self.files.push(f);
            self.asts.push(ast);
        }
        self.modules.push(module);
        self.edges.push(Vec::new());
        self.by_path.insert(path, Some(m));
        m
    }

    /// The module `path` (`util.sub`, `std.fs`), loaded the first time;
    /// `None` if there is none. `span` is the use, for read errors.
    fn module(&mut self, path: &str, span: Span) -> Option<usize> {
        if let Some(&m) = self.by_path.get(path) {
            return m;
        }
        let files = if path == "std" || path.starts_with("std.") {
            if path != "std" && !STD_FILES.iter().any(|(m, ..)| *m == path) {
                None
            } else {
                Some(
                    STD_FILES
                        .iter()
                        .filter(|(m, ..)| *m == path)
                        .map(|(_, name, text)| SourceFile {
                            name: name.to_string(),
                            text: text.to_string(),
                            std: true,
                        })
                        .collect(),
                )
            }
        } else {
            let dir = self.base.join(path.replace('.', "/"));
            match std::fs::read_dir(&dir) {
                Ok(entries) => {
                    let mut paths: Vec<_> = entries
                        .filter_map(|e| e.ok().map(|e| e.path()))
                        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "aye"))
                        .collect();
                    paths.sort();
                    let mut files = Vec::new();
                    for p in paths {
                        let name = p.to_string_lossy().into_owned();
                        match std::fs::read_to_string(&p) {
                            Ok(text) => files.push(SourceFile {
                                name,
                                text,
                                std: false,
                            }),
                            Err(e) => self.report(Error::new(
                                E0081,
                                span,
                                format!("cannot read {name}: {e}"),
                            )),
                        }
                    }
                    Some(files)
                }
                Err(_) => None,
            }
        };
        match files {
            Some(files) => Some(self.add_module(path.to_string(), files)),
            None => {
                self.by_path.insert(path.to_string(), None);
                None
            }
        }
    }

    /// The item `name` of module `m`: canonical name, kind, `pub`.
    fn item_of(&self, m: usize, name: &str) -> Option<(String, Kind, bool)> {
        let module = &self.modules[m];
        if let Some(d) = module.items.get(name) {
            return Some((canonical(&module.path, name), d.kind, d.is_pub));
        }
        BUILTINS
            .iter()
            .find(|(p, n)| *p == module.path && *n == name)
            .map(|_| (canonical(&module.path, name), Kind::Other, true))
    }

    /// The current module uses module `m` (first at `span`).
    fn edge(&mut self, m: usize, span: Span) {
        if m != self.cur && !self.edges[self.cur].iter().any(|&(t, _)| t == m) {
            self.edges[self.cur].push((m, span));
        }
    }

    fn resolve_module(&mut self, m: usize) {
        self.cur = m;
        let cimports = self.cimports(m);
        for i in 0..self.modules[m].files.len() {
            let f = self.modules[m].files[i];
            let Some(mut ast) = self.asts[f].take() else {
                continue;
            };
            self.cur = m;
            self.file = f;
            // A `@cimport` binds its name in every file of the module.
            self.imports = cimports
                .iter()
                .map(|(n, b, sp)| (n.clone(), (b.clone(), *sp)))
                .collect();
            self.last_import = ast.imports.last().map(|i| i.span);
            for imp in &ast.imports {
                self.import(imp);
            }
            for item in &mut ast.items {
                if self.files[f].std && matches!(item, AstItem::Test(_)) {
                    continue;
                }
                self.item(item);
            }
            self.asts[f] = Some(ast);
        }
    }

    /// The `@cimport`s of module `m`: each name with the generated module
    /// (made the first time a program imports a header with those flags
    /// from that directory) and the declaration's position.
    fn cimports(&mut self, m: usize) -> Vec<(String, Binding, Span)> {
        let decls = self.modules[m].cimports.clone();
        let links = self.modules[m].links.clone();
        let mut out: Vec<(String, Binding, Span)> = Vec::new();
        for (d, file) in decls {
            if let Some((_, _, prev)) = out.iter().find(|(n, ..)| *n == d.name) {
                let msg = format!(
                    "`{}` is declared more than once (first at {})",
                    d.name,
                    self.place(*prev, d.span)
                );
                self.report(Error::new(E0015, d.span, msg));
                continue;
            }
            if let Some(prev) = self.modules[m].items.get(&d.name) {
                let msg = format!(
                    "the `@cimport` named `{}` clashes with the declaration at {} (rename one)",
                    d.name,
                    self.place(prev.span, d.span)
                );
                self.report(Error::new(E0015, d.span, msg));
                continue;
            }
            let dir = Path::new(&self.files[file].name)
                .parent()
                .unwrap_or(Path::new(""))
                .to_path_buf();
            let req = cimport::Request {
                header: d.header.clone(),
                flags: d.flags.clone(),
                dir,
                links: links.clone(),
            };
            let key = format!("{req:?}");
            let q = match self.cimported.get(&key) {
                Some(&q) => q,
                None => {
                    let q = match cimport::import(&req) {
                        Ok(imp) => {
                            let path = self.cimport_path(&d.name);
                            let file = SourceFile {
                                name: imp.path.to_string_lossy().into_owned(),
                                text: imp.source,
                                std: false,
                            };
                            let q = self.add_module(path.clone(), vec![file]);
                            // Not a module that `import` can find.
                            self.by_path.remove(&path);
                            Some(q)
                        }
                        Err(cimport::Failure::NoLibclang(msg)) => {
                            self.report(Error::new(E0114, d.call_span, msg));
                            None
                        }
                        Err(cimport::Failure::Header(msg)) => {
                            self.report(Error::new(E0113, d.call_span, msg));
                            None
                        }
                    };
                    self.cimported.insert(key, q);
                    q
                }
            };
            let b = match q {
                Some(q) => {
                    self.edge(q, d.span);
                    Binding::Module(q)
                }
                None => Binding::Bad,
            };
            out.push((d.name, b, d.span));
        }
        out
    }

    /// The path of a module made by `@cimport` and named `name`: `name`,
    /// unless a module or a directory already has it.
    fn cimport_path(&self, name: &str) -> String {
        let mut path = name.to_string();
        let mut n = 2;
        while self.modules.iter().any(|m| m.path == path)
            || self.by_path.contains_key(&path)
            || self.base.join(&path).is_dir()
        {
            path = format!("{name}{n}");
            n += 1;
        }
        path
    }

    /// `import a.b.c`: bind `c` in the current file to a module or an item.
    fn import(&mut self, imp: &ast::Import) {
        let (name, name_span) = imp.path.last().unwrap().clone();
        let b = self.import_target(&imp.path);
        if let Some((_, prev)) = self.imports.get(&name) {
            let e = Error::new(
                E0015,
                name_span,
                format!(
                    "`{name}` is imported more than once (first at {}:{})",
                    prev.line, prev.col
                ),
            );
            self.report(e);
            return;
        }
        if let Some(d) = self.modules[self.cur].items.get(&name) {
            let msg = format!(
                "the import of `{name}` clashes with the declaration of `{name}` at {} (rename the declaration)",
                self.place(d.span, name_span)
            );
            self.report(Error::new(E0015, name_span, msg));
            return;
        }
        self.imports.insert(name, (b, name_span));
    }

    fn import_target(&mut self, path: &[(String, Span)]) -> Binding {
        let (first, span) = &path[0];
        let Some(mut q) = self.module(first, *span) else {
            if let Some(msg) = self.root_import(first) {
                self.report(Error::new(E0083, *span, msg));
                return Binding::Bad;
            }
            let dir = self.base.join(first);
            let msg = format!(
                "cannot find module `{first}` (no directory `{}`{})",
                dir.display(),
                self.sibling_hint(first)
            );
            self.report(Error::new(E0081, *span, msg));
            return Binding::Bad;
        };
        for (i, (seg, sp)) in path.iter().enumerate().skip(1) {
            if self.modules[q].broken {
                return Binding::Bad;
            }
            if let Some((canon, kind, is_pub)) = self.item_of(q, seg) {
                let shown = join(&path[..=i]);
                if i + 1 < path.len() {
                    self.report(Error::new(
                        E0081,
                        path[i + 1].1,
                        format!("`{shown}` is not a module"),
                    ));
                    return Binding::Bad;
                }
                if !is_pub {
                    self.report(self.not_pub(seg, q, *sp));
                    return Binding::Bad;
                }
                self.edge(q, *span);
                return Binding::Item(canon, kind);
            }
            let sub = format!("{}.{seg}", self.modules[q].path);
            match self.module(&sub, *sp) {
                Some(s) => q = s,
                None => {
                    let msg = format!("module `{}` has no `{seg}`", self.modules[q].path);
                    self.report(Error::new(E0081, *sp, msg));
                    return Binding::Bad;
                }
            }
        }
        self.edge(q, *span);
        Binding::Module(q)
    }

    /// When `import first` names the root module (its directory, or its
    /// file when it is one file): why that cannot be. Every other module is
    /// imported by the root, directly or not, so it would be a cycle.
    fn root_import(&self, first: &str) -> Option<String> {
        let files = &self.modules[0].files;
        let stem = |f: &usize| {
            Path::new(&self.files[*f].name)
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
        };
        let (shown, dir) = if files.len() == 1 && stem(&files[0]).as_deref() == Some(first) {
            (self.files[files[0]].name.clone(), self.base.join("shared"))
        } else {
            let base = std::fs::canonicalize(self.base).ok()?;
            if base.file_name()?.to_string_lossy() != first {
                return None;
            }
            // A file of a directory with other `.aye` files is not that directory.
            let aye_files = std::fs::read_dir(&base)
                .ok()?
                .filter(|e| {
                    e.as_ref()
                        .is_ok_and(|e| e.path().extension().is_some_and(|x| x == "aye"))
                })
                .count();
            if aye_files != files.len() {
                return None;
            }
            (self.base.display().to_string(), self.base.join("shared"))
        };
        Some(format!(
            "`{first}` is the root module ({shown}), which cannot be imported: it imports every other module of the program, directly or not, so this would be an import cycle; move what both need into a module of its own (a directory such as `{}`) and import that from both",
            dir.display()
        ))
    }

    /// When the module `first` is not found, but a directory above the
    /// root module's has it: the root module is likely a module of a
    /// larger program there.
    fn sibling_hint(&self, first: &str) -> String {
        for top in self.base.ancestors().skip(1) {
            // `""` is the working directory.
            let shown = if top.as_os_str().is_empty() {
                Path::new(".")
            } else {
                top
            };
            if !shown.join(first).is_dir() {
                continue;
            }
            let Ok(rel) = self.base.strip_prefix(top) else {
                break;
            };
            let module: Vec<String> = rel
                .iter()
                .map(|c| c.to_string_lossy().into_owned())
                .collect();
            return format!(
                "; imports are found from the directory given to `aye`, and `{}` exists: give `{}` instead, with `--module {}` for `aye test`",
                top.join(first).display(),
                shown.display(),
                module.join(".")
            );
        }
        String::new()
    }

    fn not_pub(&self, name: &str, m: usize, span: Span) -> Error {
        Error::new(
            E0082,
            span,
            format!("`{name}` is not `pub` in module `{}`", self.modules[m].path),
        )
    }

    /// A name written without a path.
    fn resolve_name(&mut self, name: &str, span: Span) -> Result<Res, Error> {
        if self.shadowed.iter().any(|n| n == name) {
            return Ok(Res::Unresolved);
        }
        let module = &self.modules[self.cur];
        if module.items.contains_key(name) {
            return Ok(Res::Item(
                canonical(&module.path, name),
                module.items[name].kind,
            ));
        }
        if let Some((b, _)) = self.imports.get(name) {
            return match b {
                Binding::Module(q) => Ok(Res::Module(*q)),
                Binding::Item(c, k) => Ok(Res::Item(c.clone(), *k)),
                Binding::Bad => Err(silent()),
            };
        }
        if let Some(home) = std_home(name) {
            let import = format!("import {home}.{name}");
            let e = Error::new(
                E0084,
                span,
                format!("`{name}` is not declared: it is `{home}.{name}` (add `{import}`)"),
            );
            let (at, text) = match self.last_import {
                Some(s) => (
                    Span {
                        line: s.line + 1,
                        col: 1,
                        file: s.file,
                    },
                    format!("{import}\n"),
                ),
                None => (
                    Span {
                        line: 1,
                        col: 1,
                        file: self.file as u16,
                    },
                    format!("{import}\n\n"),
                ),
            };
            return Err(e.fix(format!("add `{import}`"), at, at, text));
        }
        Ok(Res::Unresolved)
    }

    /// A path `a.b.c`: what its first `n` names refer to, and `n`. A module
    /// is followed into its items and submodules; an item ends the path
    /// (the rest are fields, variants or methods).
    fn resolve_path(&mut self, segs: &[(String, Span)]) -> Result<(Res, usize), Error> {
        let r = self.resolve_name(&segs[0].0, segs[0].1)?;
        let Res::Module(mut q) = r else {
            return Ok((r, 1));
        };
        let mut i = 1;
        while i < segs.len() {
            if self.modules[q].broken {
                return Err(silent());
            }
            let (seg, sp) = &segs[i];
            if let Some((canon, kind, is_pub)) = self.item_of(q, seg) {
                if !is_pub && q != self.cur {
                    return Err(self.not_pub(seg, q, *sp));
                }
                self.edge(q, *sp);
                return Ok((Res::Item(canon, kind), i + 1));
            }
            let sub = format!("{}.{seg}", self.modules[q].path);
            match self.module(&sub, *sp) {
                Some(s) => q = s,
                None => {
                    return Err(Error::new(
                        E0081,
                        *sp,
                        format!("module `{}` has no `{seg}`", self.modules[q].path),
                    ));
                }
            }
            i += 1;
        }
        Ok((Res::Module(q), i))
    }

    /// A name in a type, a `where` constraint or an error set merge: its
    /// canonical name, or `None` to leave it as it is.
    fn type_name(&mut self, name: &str, span: Span) -> Option<String> {
        let segs = split_path(name, span);
        let r = match self.resolve_path(&segs) {
            Ok(r) => r,
            Err(e) => {
                self.report(e);
                return None;
            }
        };
        let e = match r {
            (Res::Item(c, _), n) if n == segs.len() => return Some(c),
            (Res::Item(..), n) => Error::new(
                E0085,
                span,
                format!(
                    "`{}` is not a module, so `{name}` is not a type",
                    join(&segs[..n])
                ),
            ),
            (Res::Module(_), _) => {
                Error::new(E0085, span, format!("`{name}` is a module, not a type"))
            }
            (Res::Unresolved, _) if segs.len() > 1 => self.not_imported(&segs[0].0, span),
            (Res::Unresolved, _) => return None,
        };
        self.report(e);
        None
    }

    /// The position `at` in a message about `here`, with the file's name
    /// when it is another file.
    fn place(&self, at: Span, here: Span) -> String {
        let names: Vec<String> = self.files.iter().map(|f| f.name.clone()).collect();
        crate::sema::place(&names, at, here)
    }

    fn not_imported(&self, name: &str, span: Span) -> Error {
        let hint = if STD_FILES.iter().any(|(m, ..)| *m == format!("std.{name}")) {
            format!(" (add `import std.{name}`)")
        } else {
            format!(" (add `import {name}`)")
        };
        Error::new(
            E0081,
            span,
            format!("`{name}` is not an imported module{hint}"),
        )
    }

    /// A new name: a parameter, local, capture or pattern binding. It
    /// cannot shadow an import or (outside the root module, where sema
    /// checks it) a top-level name of the module.
    fn bind(&mut self, name: &str, span: Span) {
        if let Some(&(_, prev)) = self.imports.get(name) {
            let msg = format!(
                "`{name}` is already the name of the import at {}:{} (shadowing is not allowed: rename the local)",
                prev.line, prev.col
            );
            self.report(Error::new(E0016, span, msg));
            self.shadowed.push(name.to_string());
            return;
        }
        let prev = match self.cur {
            0 => None,
            m => self.modules[m].items.get(name).map(|d| d.span),
        };
        if let Some(prev) = prev {
            let msg = format!(
                "`{name}` is already declared at {} (shadowing is not allowed)",
                self.place(prev, span)
            );
            self.report(Error::new(E0016, span, msg));
        }
    }

    // ------------------------------------------------------------ walking

    fn item(&mut self, item: &mut AstItem) {
        match item {
            AstItem::Fn(f) => self.func(f),
            AstItem::Struct(s) => self.struct_decl(s),
            AstItem::Enum(e) => {
                for v in &mut e.variants {
                    for f in &mut v.fields {
                        self.ty(&mut f.ty);
                    }
                }
                for m in &mut e.methods {
                    self.func(m);
                }
            }
            AstItem::Global(g) => {
                if let Some(t) = &mut g.ty {
                    self.ty(t);
                }
                self.expr(&mut g.init);
            }
            AstItem::ErrorSet(e) => {
                if let ast::ErrorSetKind::Merge(parts) = &mut e.kind {
                    for (name, span) in parts {
                        if let Some(c) = self.type_name(name, *span) {
                            *name = c;
                        }
                    }
                }
            }
            AstItem::Interface(i) => {
                for m in &mut i.methods {
                    self.func(m);
                }
            }
            AstItem::Test(t) => self.func(&mut t.func),
            AstItem::Link(_) | AstItem::CImport(_) => {}
        }
    }

    fn struct_decl(&mut self, s: &mut ast::StructDecl) {
        for f in &mut s.fields {
            self.ty(&mut f.ty);
            if let Some(d) = &mut f.default {
                self.expr(d);
            }
        }
        for m in &mut s.methods {
            self.func(m);
        }
    }

    fn func(&mut self, f: &mut ast::FnDecl) {
        self.shadowed.clear();
        for p in &mut f.params {
            if f.body.is_some() {
                self.bind(&p.name, p.span);
            }
            self.ty(&mut p.ty);
        }
        if let Some(t) = &mut f.ret {
            self.ty(t);
        }
        for b in &mut f.bounds {
            if let Some(c) = self.type_name(&b.iface, b.iface_span) {
                b.iface = c;
            }
        }
        if let Some(body) = &mut f.body {
            self.block(body);
        }
    }

    fn ty(&mut self, t: &mut ast::TypeExpr) {
        let span = t.span;
        match &mut t.kind {
            TypeExprKind::Named(n) | TypeExprKind::Dyn(n) => {
                if let Some(c) = self.type_name(n, span) {
                    *n = c;
                }
            }
            TypeExprKind::Generic(g, args) => {
                if let Some(c) = self.type_name(g, span) {
                    *g = c;
                }
                for a in args {
                    self.ty(a);
                }
            }
            TypeExprKind::ErrorUnion(set, payload) => {
                if let Some((s, sp)) = set
                    && let Some(c) = self.type_name(s, *sp)
                {
                    *s = c;
                }
                self.ty(payload);
            }
            TypeExprKind::Array(len, e) | TypeExprKind::Simd(len, e) => {
                if let ast::ArrayLen::Const(x) = len {
                    self.expr(x);
                }
                self.ty(e);
            }
            TypeExprKind::Slice(e, _)
            | TypeExprKind::CPtr(e)
            | TypeExprKind::Ptr(e, _)
            | TypeExprKind::Optional(e) => self.ty(e),
            TypeExprKind::Fn(params, ret) => {
                for p in params {
                    self.ty(p);
                }
                if let Some(r) = ret {
                    self.ty(r);
                }
            }
        }
    }

    fn block(&mut self, b: &mut ast::Block) {
        for s in &mut b.stmts {
            self.stmt(s);
        }
    }

    fn stmt(&mut self, s: &mut ast::Stmt) {
        let span = s.span;
        match &mut s.kind {
            StmtKind::Let { name, ty, init, .. } => {
                if let Some(t) = ty {
                    self.ty(t);
                }
                self.expr(init);
                self.bind(name, span);
            }
            StmtKind::Assign { target, value, .. } => {
                self.expr(target);
                self.expr(value);
            }
            StmtKind::If {
                cond,
                capture,
                then,
                els,
                err_capture,
            } => {
                self.expr(cond);
                for (n, sp) in capture.iter().chain(err_capture.iter()) {
                    self.bind(n, *sp);
                }
                self.block(then);
                if let Some(e) = els {
                    self.stmt(e);
                }
            }
            StmtKind::While {
                cond,
                capture,
                body,
            } => {
                self.expr(cond);
                if let Some((n, sp)) = capture {
                    self.bind(n, *sp);
                }
                self.block(body);
            }
            StmtKind::ForEach {
                index,
                elem,
                seq,
                body,
            } => {
                self.expr(seq);
                for (n, sp) in index.iter().chain([&*elem]) {
                    self.bind(n, *sp);
                }
                self.block(body);
            }
            StmtKind::For {
                var,
                var_span,
                lo,
                hi,
                body,
            } => {
                self.expr(lo);
                self.expr(hi);
                self.bind(var, *var_span);
                self.block(body);
            }
            StmtKind::Break | StmtKind::Continue | StmtKind::Return(None) => {}
            StmtKind::Return(Some(e)) | StmtKind::Expr(e) => self.expr(e),
            StmtKind::Defer(inner) | StmtKind::ErrDefer(inner) => self.stmt(inner),
            StmtKind::Block(b) => self.block(b),
        }
    }

    fn arm_body(&mut self, b: &mut ArmBody) {
        match b {
            ArmBody::Expr(e) => self.expr(e),
            ArmBody::Block(b) => self.block(b),
        }
    }

    fn expr(&mut self, e: &mut ast::Expr) {
        let span = e.span;
        match &mut e.kind {
            ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::Bool(_)
            | ExprKind::Null
            | ExprKind::Str(_)
            | ExprKind::Char(_) => {}
            ExprKind::Ident(n) => match self.resolve_name(n, span) {
                Ok(Res::Item(c, _)) => *n = c,
                Ok(Res::Module(_)) => self.report(Error::new(
                    E0085,
                    span,
                    format!("`{n}` is a module, not a value (use one of its items: `{n}.name`)"),
                )),
                Ok(Res::Unresolved) => {}
                Err(err) => self.report(err),
            },
            ExprKind::Call { name, args } => {
                match self.resolve_name(name, span) {
                    Ok(Res::Item(c, _)) => *name = c,
                    Ok(Res::Module(_)) => self.report(Error::new(
                        E0085,
                        span,
                        format!("`{name}` is a module, not a function"),
                    )),
                    Ok(Res::Unresolved) => {}
                    Err(err) => self.report(err),
                }
                self.args(args);
            }
            ExprKind::CallExpr { callee, args } => {
                self.expr(callee);
                self.args(args);
            }
            ExprKind::MethodCall { base, name, args } => {
                self.args(args);
                let Some(mut segs) = expr_path(base) else {
                    self.expr(base);
                    return;
                };
                segs.push((name.clone(), span));
                match self.resolve_path(&segs) {
                    Ok((Res::Item(c, _), n)) if n == segs.len() => {
                        e.kind = ExprKind::Call {
                            name: c,
                            args: std::mem::take(args),
                        };
                    }
                    Ok((Res::Item(c, _), n)) => {
                        let first = segs[n - 1].1;
                        **base = rebuild(c, first, &segs[n..segs.len() - 1]);
                    }
                    Ok((Res::Module(_), _)) => {
                        let msg = format!("`{}` is a module, not a function", join(&segs));
                        self.report(Error::new(E0085, span, msg));
                    }
                    Ok((Res::Unresolved, _)) => {}
                    Err(err) => self.report(err),
                }
            }
            ExprKind::Field { .. } => {
                let segs = expr_path(e).unwrap_or_default();
                if segs.is_empty() {
                    let ExprKind::Field { base, .. } = &mut e.kind else {
                        unreachable!()
                    };
                    self.expr(base);
                    return;
                }
                match self.resolve_path(&segs) {
                    Ok((Res::Item(c, _), n)) => *e = rebuild(c, segs[n - 1].1, &segs[n..]),
                    Ok((Res::Module(_), _)) => {
                        let msg = format!(
                            "`{}` is a module, not a value (use one of its items)",
                            join(&segs)
                        );
                        self.report(Error::new(E0085, span, msg));
                    }
                    Ok((Res::Unresolved, _)) => {}
                    Err(err) => self.report(err),
                }
            }
            ExprKind::VariantLit {
                enum_name,
                variant,
                fields,
            } => {
                for f in fields.iter_mut() {
                    self.expr(&mut f.value);
                }
                let mut segs = split_path(enum_name, span);
                segs.push((variant.clone(), span));
                match self.resolve_path(&segs) {
                    Ok((Res::Item(c, Kind::Struct), n)) if n == segs.len() => {
                        let ty = ast::TypeExpr {
                            kind: TypeExprKind::Named(c),
                            span,
                        };
                        e.kind = ExprKind::StructLit {
                            ty,
                            fields: std::mem::take(fields),
                        };
                    }
                    Ok((Res::Item(c, _), n)) if n == segs.len() - 1 => *enum_name = c,
                    Ok((Res::Item(..), n)) if n == segs.len() => {
                        let msg = format!("`{}` is not a struct", join(&segs));
                        self.report(Error::new(E0085, span, msg));
                    }
                    Ok((Res::Item(..), n)) => {
                        let msg = format!(
                            "`{}` is not a module, so `{}` is not an enum",
                            join(&segs[..n]),
                            enum_name
                        );
                        self.report(Error::new(E0085, span, msg));
                    }
                    Ok((Res::Module(_), _)) => {
                        let msg = format!("`{}` is a module, not a struct", join(&segs));
                        self.report(Error::new(E0085, span, msg));
                    }
                    Ok((Res::Unresolved, _)) if segs.len() > 2 => {
                        self.report(self.not_imported(&segs[0].0, span))
                    }
                    Ok((Res::Unresolved, _)) => {}
                    Err(err) => self.report(err),
                }
            }
            ExprKind::StructLit { ty, fields } => {
                self.ty(ty);
                for f in fields {
                    self.expr(&mut f.value);
                }
            }
            ExprKind::Builtin { args, .. } => self.args(args),
            ExprKind::Binary { lhs, rhs, .. }
            | ExprKind::Index {
                base: lhs,
                index: rhs,
            } => {
                self.expr(lhs);
                self.expr(rhs);
            }
            ExprKind::Unary { expr, .. }
            | ExprKind::AddrOf { expr, .. }
            | ExprKind::Deref(expr)
            | ExprKind::Try(expr)
            | ExprKind::Unwrap(expr)
            | ExprKind::Is { expr, .. }
            | ExprKind::Paren(expr) => self.expr(expr),
            ExprKind::Unreachable => {}
            ExprKind::OrElse { expr, body } => {
                self.expr(expr);
                self.arm_body(body);
            }
            ExprKind::Cast { expr, ty } => {
                self.expr(expr);
                self.ty(ty);
            }
            ExprKind::Slice { base, lo, hi } => {
                self.expr(base);
                for x in [lo, hi].into_iter().flatten() {
                    self.expr(x);
                }
            }
            ExprKind::Match { scrutinee, arms } => {
                self.expr(scrutinee);
                for arm in arms {
                    for p in arm.variants.iter().flat_map(|v| v.fields.iter().flatten()) {
                        self.bind(&p.binding, p.binding_span);
                    }
                    self.arm_body(&mut arm.body);
                }
            }
            ExprKind::Catch {
                expr,
                capture,
                body,
            } => {
                self.expr(expr);
                if let Some((n, sp)) = capture {
                    self.bind(n, *sp);
                }
                self.arm_body(body);
            }
            ExprKind::ArrayRepeat { ty, value } => {
                self.ty(ty);
                self.expr(value);
            }
            ExprKind::ArrayLit { ty, elems } => {
                self.ty(ty);
                self.args(elems);
            }
            ExprKind::StructType(s) => self.struct_decl(s),
            ExprKind::Type(t) => self.ty(t),
        }
    }

    fn args(&mut self, args: &mut [ast::Expr]) {
        for a in args {
            self.expr(a);
        }
    }

    // ------------------------------------------------------------- cycles

    /// Report each import cycle at the import (or use) that closes it.
    fn check_cycles(&mut self) {
        let mut state = vec![0u8; self.modules.len()];
        let mut stack = Vec::new();
        self.visit(0, &mut state, &mut stack);
    }

    fn visit(&mut self, m: usize, state: &mut Vec<u8>, stack: &mut Vec<usize>) {
        state[m] = 1;
        stack.push(m);
        for (t, span) in self.edges[m].clone() {
            match state[t] {
                0 => self.visit(t, state, stack),
                1 => {
                    let i = stack.iter().position(|&x| x == t).unwrap();
                    let names: Vec<String> = stack[i..]
                        .iter()
                        .chain([&t])
                        .map(|&x| format!("`{}`", self.modules[x].path))
                        .collect();
                    self.report(Error::new(
                        E0083,
                        span,
                        format!("import cycle: {}", names.join(" -> ")),
                    ));
                }
                _ => {}
            }
        }
        stack.pop();
        state[m] = 2;
    }

    // -------------------------------------------------------------- merge

    /// One AST: the root module's items, then every other module's items
    /// with canonical names. Tests are numbered across the program.
    fn merge(&mut self) -> ast::File {
        let mut items = Vec::new();
        let mut tests = 0;
        for m in 0..self.modules.len() {
            let path = self.modules[m].path.clone();
            for i in 0..self.modules[m].files.len() {
                let f = self.modules[m].files[i];
                let ast = self.asts[f].take().unwrap();
                for mut item in ast.items {
                    match &mut item {
                        AstItem::Test(_) if self.files[f].std => continue,
                        AstItem::Test(t) => {
                            t.func.name = canonical(&path, &format!("test#{tests}"));
                            t.module = path.clone();
                            tests += 1;
                        }
                        AstItem::Fn(x) => x.name = canonical(&path, &x.name),
                        AstItem::Struct(x) => x.name = canonical(&path, &x.name),
                        AstItem::Enum(x) => x.name = canonical(&path, &x.name),
                        AstItem::ErrorSet(x) => x.name = canonical(&path, &x.name),
                        AstItem::Interface(x) => x.name = canonical(&path, &x.name),
                        AstItem::Global(x) => x.name = canonical(&path, &x.name),
                        AstItem::Link(_) => {}
                        AstItem::CImport(_) => continue,
                    }
                    items.push(item);
                }
            }
        }
        ast::File {
            imports: Vec::new(),
            items,
        }
    }
}
