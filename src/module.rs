//! Modules and name resolution.
//!
//! A module is a directory: every `.ore` file in it shares one namespace,
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
//! the root module's items first, then the other modules' items. Only the
//! root module's `test` blocks are kept.
//!
//! Imports are per file: `import std.fs` binds `fs` in its file only. A
//! path such as `std.fs.open` is resolved one name at a time: a module's
//! item, or else its submodule (`std.fs` within `std`).

use crate::ast::{self, ArmBody, ExprKind, Item as AstItem, StmtKind, TypeExprKind};
use crate::codes::*;
use crate::diag::{Error, Span};
use crate::{lexer, parser};
use std::collections::HashMap;
use std::path::Path;

/// A source file of the program.
pub struct SourceFile {
    /// The name in diagnostics, `#line` directives and panic messages.
    pub name: String,
    pub text: String,
    /// Part of the embedded std (`ore fix` never writes it).
    pub std: bool,
}

/// std, embedded: (module, file name, source).
const STD_FILES: &[(&str, &str, &str)] = &[
    ("std.fs", "std/fs/fs.ore", include_str!("../std/fs/fs.ore")),
    ("std.heap", "std/heap/heap.ore", include_str!("../std/heap/heap.ore")),
    ("std.list", "std/list/list.ore", include_str!("../std/list/list.ore")),
    ("std.log", "std/log/log.ore", include_str!("../std/log/log.ore")),
    ("std.map", "std/map/map.ore", include_str!("../std/map/map.ore")),
    ("std.math", "std/math/math.ore", include_str!("../std/math/math.ore")),
    ("std.mem", "std/mem/mem.ore", include_str!("../std/mem/mem.ore")),
    ("std.os", "std/os/os.ore", include_str!("../std/os/os.ore")),
    ("std.pool", "std/pool/pool.ore", include_str!("../std/pool/pool.ore")),
    ("std.sort", "std/sort/sort.ore", include_str!("../std/sort/sort.ore")),
    ("std.testing", "std/testing/testing.ore", include_str!("../std/testing/testing.ore")),
    ("std.text", "std/text/text.ore", include_str!("../std/text/text.ore")),
    ("std.time", "std/time/time.ore", include_str!("../std/time/time.ore")),
];

/// Items of std that the compiler provides (their declarations would need
/// features Ore does not have yet): (module, name).
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
const BUILTIN_METHODS: &[&str] = &[
    "General.init",
    "General.allocator",
    "General.deinit",
    "Arena.init",
    "Arena.allocator",
    "Arena.deinit",
    "Allocator.create",
    "Allocator.destroy",
    "Allocator.alloc",
    "Allocator.free",
    "Allocator.resize",
];

/// The canonical name of the item `name` of the module `module` (`""` is
/// the root module).
pub fn canonical(module: &str, name: &str) -> String {
    if module.is_empty() { name.to_string() } else { format!("{}::{name}", module.replace('.', "::")) }
}

/// The pub functions and methods of std, as the spec writes them:
/// `std.fs.open`, `File.readAll`, `std.log.err`, `General.init`.
pub fn std_api() -> Vec<String> {
    let mut out = Vec::new();
    for (module, _, src) in STD_FILES {
        let file = parser::parse(lexer::lex(src).unwrap()).unwrap_or_else(|e| panic!("{module}: {e:?}"));
        for item in &file.items {
            match item {
                AstItem::Fn(f) if f.is_pub => {
                    out.push(format!("{module}.{}", f.name));
                    // A type function's methods: `List.append`.
                    if let Some(ast::Stmt { kind: StmtKind::Return(Some(e)), .. }) = f.body.as_ref().and_then(|b| b.stmts.first())
                        && let ExprKind::StructType(s) = &e.kind
                    {
                        out.extend(s.methods.iter().filter(|m| m.is_pub).map(|m| format!("{}.{}", f.name, m.name)));
                    }
                }
                AstItem::Struct(s) if s.is_pub => {
                    out.extend(s.methods.iter().filter(|m| m.is_pub).map(|m| format!("{}.{}", s.name, m.name)))
                }
                _ => {}
            }
        }
    }
    out.extend(BUILTINS.iter().filter(|(_, n)| n.starts_with(char::is_lowercase)).map(|(m, n)| format!("{m}.{n}")));
    out.extend(BUILTIN_METHODS.iter().map(|m| m.to_string()));
    out
}

/// The std module of the builtin `name` when a module uses it without an
/// import (`Allocator` is `std.mem.Allocator`).
fn std_home(name: &str) -> Option<&'static str> {
    BUILTINS.iter().find(|(_, n)| *n == name && n.starts_with(char::is_uppercase)).map(|(m, _)| *m)
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
    segs.iter().map(|(s, _)| s.as_str()).collect::<Vec<_>>().join(".")
}

/// `segs[..n]` as an identifier with field accesses after it.
fn rebuild(first: String, span: Span, rest: &[(String, Span)]) -> ast::Expr {
    let mut e = ast::Expr { kind: ExprKind::Ident(first), span };
    for (name, sp) in rest {
        e = ast::Expr { kind: ExprKind::Field { base: Box::new(e), name: name.clone() }, span: *sp };
    }
    e
}

struct Loader<'a> {
    /// The root module's directory: `import a.b` is `base/a/b`.
    base: &'a Path,
    files: Vec<SourceFile>,
    /// Each file's syntax tree; `None` after a syntax error.
    asts: Vec<Option<ast::File>>,
    modules: Vec<Module>,
    by_path: HashMap<String, Option<usize>>,
    /// The modules each module uses, with the first import or use.
    edges: Vec<Vec<(usize, Span)>>,
    errors: Vec<Error>,
    /// The module and file being resolved, and the file's imports.
    cur: usize,
    file: usize,
    imports: HashMap<String, (Binding, Span)>,
    last_import: Option<Span>,
}

/// Load the root module (`root`, in the directory `base`) and every module
/// it uses, and resolve names. Returns every file of the program, indexed
/// by `Span::file`, and the program's AST or the errors.
pub fn load(root: Vec<SourceFile>, base: &Path) -> (Vec<SourceFile>, Result<ast::File, Vec<Error>>) {
    let mut l = Loader {
        base,
        files: Vec::new(),
        asts: Vec::new(),
        modules: Vec::new(),
        by_path: HashMap::new(),
        edges: Vec::new(),
        errors: Vec::new(),
        cur: 0,
        file: 0,
        imports: HashMap::new(),
        last_import: None,
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

impl Loader<'_> {
    fn report(&mut self, e: Error) {
        if !e.msg.is_empty() {
            self.errors.push(e);
        }
    }

    /// Parse the files of a new module and register its items.
    fn add_module(&mut self, path: String, files: Vec<SourceFile>) -> usize {
        let m = self.modules.len();
        let mut module = Module { path: path.clone(), files: Vec::new(), items: HashMap::new(), broken: false };
        for f in files {
            let id = self.files.len();
            let set = |s: &mut Span| s.file = id as u16;
            let parsed = lexer::lex(&f.text)
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
                            AstItem::Test(_) | AstItem::Link(_) => continue,
                        };
                        module.items.entry(name.clone()).or_insert(Decl { is_pub, kind, span });
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
                        .map(|(_, name, text)| SourceFile { name: name.to_string(), text: text.to_string(), std: true })
                        .collect(),
                )
            }
        } else {
            let dir = self.base.join(path.replace('.', "/"));
            match std::fs::read_dir(&dir) {
                Ok(entries) => {
                    let mut paths: Vec<_> = entries
                        .filter_map(|e| e.ok().map(|e| e.path()))
                        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "ore"))
                        .collect();
                    paths.sort();
                    let mut files = Vec::new();
                    for p in paths {
                        let name = p.to_string_lossy().into_owned();
                        match std::fs::read_to_string(&p) {
                            Ok(text) => files.push(SourceFile { name, text, std: false }),
                            Err(e) => self.report(Error::new(E0081, span, format!("cannot read {name}: {e}"))),
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
        BUILTINS.iter().find(|(p, n)| *p == module.path && *n == name).map(|_| (canonical(&module.path, name), Kind::Other, true))
    }

    /// The current module uses module `m` (first at `span`).
    fn edge(&mut self, m: usize, span: Span) {
        if m != self.cur && !self.edges[self.cur].iter().any(|&(t, _)| t == m) {
            self.edges[self.cur].push((m, span));
        }
    }

    fn resolve_module(&mut self, m: usize) {
        for i in 0..self.modules[m].files.len() {
            let f = self.modules[m].files[i];
            let Some(mut ast) = self.asts[f].take() else { continue };
            self.cur = m;
            self.file = f;
            self.imports = HashMap::new();
            self.last_import = ast.imports.last().map(|i| i.span);
            for imp in &ast.imports {
                self.import(imp);
            }
            for item in &mut ast.items {
                if m != 0 && matches!(item, AstItem::Test(_)) {
                    continue;
                }
                self.item(item);
            }
            self.asts[f] = Some(ast);
        }
    }

    /// `import a.b.c`: bind `c` in the current file to a module or an item.
    fn import(&mut self, imp: &ast::Import) {
        let (name, name_span) = imp.path.last().unwrap().clone();
        let b = self.import_target(&imp.path);
        if let Some((_, prev)) = self.imports.get(&name) {
            let e = Error::new(E0015, name_span, format!("`{name}` is imported more than once (first at {}:{})", prev.line, prev.col));
            self.report(e);
            return;
        }
        if let Some(d) = self.modules[self.cur].items.get(&name) {
            let msg = format!(
                "the import of `{name}` clashes with the declaration of `{name}` at {}:{} (rename the declaration)",
                d.span.line, d.span.col
            );
            self.report(Error::new(E0015, name_span, msg));
            return;
        }
        self.imports.insert(name, (b, name_span));
    }

    fn import_target(&mut self, path: &[(String, Span)]) -> Binding {
        let (first, span) = &path[0];
        let Some(mut q) = self.module(first, *span) else {
            let dir = self.base.join(first);
            let msg = format!("cannot find module `{first}` (no directory `{}`)", dir.display());
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
                    self.report(Error::new(E0081, path[i + 1].1, format!("`{shown}` is not a module")));
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

    fn not_pub(&self, name: &str, m: usize, span: Span) -> Error {
        Error::new(E0082, span, format!("`{name}` is not `pub` in module `{}`", self.modules[m].path))
    }

    /// A name written without a path.
    fn resolve_name(&mut self, name: &str, span: Span) -> Result<Res, Error> {
        let module = &self.modules[self.cur];
        if module.items.contains_key(name) {
            return Ok(Res::Item(canonical(&module.path, name), module.items[name].kind));
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
            let e = Error::new(E0084, span, format!("`{name}` is not declared: it is `{home}.{name}` (add `{import}`)"));
            let (at, text) = match self.last_import {
                Some(s) => (Span { line: s.line + 1, col: 1, file: s.file }, format!("{import}\n")),
                None => (Span { line: 1, col: 1, file: self.file as u16 }, format!("{import}\n\n")),
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
        let Res::Module(mut q) = r else { return Ok((r, 1)) };
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
                None => return Err(Error::new(E0081, *sp, format!("module `{}` has no `{seg}`", self.modules[q].path))),
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
            (Res::Item(..), n) => {
                Error::new(E0085, span, format!("`{}` is not a module, so `{name}` is not a type", join(&segs[..n])))
            }
            (Res::Module(_), _) => Error::new(E0085, span, format!("`{name}` is a module, not a type")),
            (Res::Unresolved, _) if segs.len() > 1 => self.not_imported(&segs[0].0, span),
            (Res::Unresolved, _) => return None,
        };
        self.report(e);
        None
    }

    fn not_imported(&self, name: &str, span: Span) -> Error {
        let hint = if STD_FILES.iter().any(|(m, ..)| *m == format!("std.{name}")) {
            format!(" (add `import std.{name}`)")
        } else {
            format!(" (add `import {name}`)")
        };
        Error::new(E0081, span, format!("`{name}` is not an imported module{hint}"))
    }

    /// A new name: a parameter, local, capture or pattern binding. It
    /// cannot shadow an import or (outside the root module, where sema
    /// checks it) a top-level name of the module.
    fn bind(&mut self, name: &str, span: Span) {
        let prev = self.imports.get(name).map(|(_, s)| *s).or_else(|| match self.cur {
            0 => None,
            m => self.modules[m].items.get(name).map(|d| d.span),
        });
        if let Some(prev) = prev {
            let msg = format!("`{name}` is already declared at {}:{} (shadowing is not allowed)", prev.line, prev.col);
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
            AstItem::Link(_) => {}
        }
    }

    fn struct_decl(&mut self, s: &mut ast::StructDecl) {
        for f in &mut s.fields {
            self.ty(&mut f.ty);
        }
        for m in &mut s.methods {
            self.func(m);
        }
    }

    fn func(&mut self, f: &mut ast::FnDecl) {
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
            TypeExprKind::Array(_, e)
            | TypeExprKind::Slice(e, _)
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
            StmtKind::If { cond, capture, then, els } => {
                self.expr(cond);
                if let Some((n, sp)) = capture {
                    self.bind(n, *sp);
                }
                self.block(then);
                if let Some(e) = els {
                    self.stmt(e);
                }
            }
            StmtKind::While { cond, capture, body } => {
                self.expr(cond);
                if let Some((n, sp)) = capture {
                    self.bind(n, *sp);
                }
                self.block(body);
            }
            StmtKind::ForEach { index, elem, seq, body } => {
                self.expr(seq);
                for (n, sp) in index.iter().chain([&*elem]) {
                    self.bind(n, *sp);
                }
                self.block(body);
            }
            StmtKind::For { var, var_span, lo, hi, body } => {
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
                Ok(Res::Module(_)) => {
                    self.report(Error::new(E0085, span, format!("`{n}` is a module, not a value (use one of its items: `{n}.name`)")))
                }
                Ok(Res::Unresolved) => {}
                Err(err) => self.report(err),
            },
            ExprKind::Call { name, args } => {
                match self.resolve_name(name, span) {
                    Ok(Res::Item(c, _)) => *name = c,
                    Ok(Res::Module(_)) => self.report(Error::new(E0085, span, format!("`{name}` is a module, not a function"))),
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
                        e.kind = ExprKind::Call { name: c, args: std::mem::take(args) };
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
                    let ExprKind::Field { base, .. } = &mut e.kind else { unreachable!() };
                    self.expr(base);
                    return;
                }
                match self.resolve_path(&segs) {
                    Ok((Res::Item(c, _), n)) => *e = rebuild(c, segs[n - 1].1, &segs[n..]),
                    Ok((Res::Module(_), _)) => {
                        let msg = format!("`{}` is a module, not a value (use one of its items)", join(&segs));
                        self.report(Error::new(E0085, span, msg));
                    }
                    Ok((Res::Unresolved, _)) => {}
                    Err(err) => self.report(err),
                }
            }
            ExprKind::VariantLit { enum_name, variant, fields } => {
                for f in fields.iter_mut() {
                    self.expr(&mut f.value);
                }
                let mut segs = split_path(enum_name, span);
                segs.push((variant.clone(), span));
                match self.resolve_path(&segs) {
                    Ok((Res::Item(c, Kind::Struct), n)) if n == segs.len() => {
                        let ty = ast::TypeExpr { kind: TypeExprKind::Named(c), span };
                        e.kind = ExprKind::StructLit { ty, fields: std::mem::take(fields) };
                    }
                    Ok((Res::Item(c, _), n)) if n == segs.len() - 1 => *enum_name = c,
                    Ok((Res::Item(..), n)) if n == segs.len() => {
                        let msg = format!("`{}` is not a struct", join(&segs));
                        self.report(Error::new(E0085, span, msg));
                    }
                    Ok((Res::Item(..), n)) => {
                        let msg = format!("`{}` is not a module, so `{}` is not an enum", join(&segs[..n]), enum_name);
                        self.report(Error::new(E0085, span, msg));
                    }
                    Ok((Res::Module(_), _)) => {
                        let msg = format!("`{}` is a module, not a struct", join(&segs));
                        self.report(Error::new(E0085, span, msg));
                    }
                    Ok((Res::Unresolved, _)) if segs.len() > 2 => self.report(self.not_imported(&segs[0].0, span)),
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
            ExprKind::Binary { lhs, rhs, .. } | ExprKind::Index { base: lhs, index: rhs } => {
                self.expr(lhs);
                self.expr(rhs);
            }
            ExprKind::Unary { expr, .. }
            | ExprKind::AddrOf { expr, .. }
            | ExprKind::Deref(expr)
            | ExprKind::Try(expr)
            | ExprKind::Paren(expr) => self.expr(expr),
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
                    for p in arm.fields.iter().flatten() {
                        self.bind(&p.binding, p.binding_span);
                    }
                    self.arm_body(&mut arm.body);
                }
            }
            ExprKind::Catch { expr, capture, body } => {
                self.expr(expr);
                if let Some((n, sp)) = capture {
                    self.bind(n, *sp);
                }
                self.arm_body(body);
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
                    let names: Vec<String> =
                        stack[i..].iter().chain([&t]).map(|&x| format!("`{}`", self.modules[x].path)).collect();
                    self.report(Error::new(E0083, span, format!("import cycle: {}", names.join(" -> "))));
                }
                _ => {}
            }
        }
        stack.pop();
        state[m] = 2;
    }

    // -------------------------------------------------------------- merge

    /// One AST: the root module's items, then every other module's items
    /// with canonical names. Root tests are numbered across its files;
    /// other modules' tests are left out.
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
                        AstItem::Test(t) if m == 0 => {
                            t.func.name = format!("test#{tests}");
                            tests += 1;
                        }
                        AstItem::Test(_) => continue,
                        AstItem::Fn(x) => x.name = canonical(&path, &x.name),
                        AstItem::Struct(x) => x.name = canonical(&path, &x.name),
                        AstItem::Enum(x) => x.name = canonical(&path, &x.name),
                        AstItem::ErrorSet(x) => x.name = canonical(&path, &x.name),
                        AstItem::Interface(x) => x.name = canonical(&path, &x.name),
                        AstItem::Global(x) => x.name = canonical(&path, &x.name),
                        AstItem::Link(_) => {}
                    }
                    items.push(item);
                }
            }
        }
        ast::File { imports: Vec::new(), items }
    }
}
