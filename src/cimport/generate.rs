//! A C header's declarations as Ore source (see `mod.rs` for the rules).
//!
//! Two translation units: the first includes the header and yields its
//! functions, records, enums, typedefs and macro definitions; the second
//! also declares `__auto_type __ore_m_N = (MACRO);` for each object-like
//! macro, so that libclang evaluates the ones that are constants.

use super::clang::*;
use std::collections::{HashMap, HashSet};
use std::ffi::{CStr, CString, c_int};
use std::path::PathBuf;

/// Generate the source for `header`, included by the line `include`.
pub fn run(lib: &Lib, header: &str, include: &str, args: &[String]) -> Result<(String, Vec<String>), String> {
    let index = unsafe { (lib.clang_createIndex)(0, 0) };
    let tu = parse(lib, index, include, args)?;
    if let Some(e) = errors(lib, tu, header) {
        unsafe { (lib.clang_disposeTranslationUnit)(tu) };
        return Err(e);
    }
    let inclusions = lib.inclusions(tu);
    let mut files: Vec<String> = inclusions.iter().map(|(f, _)| f.clone()).collect();
    files.sort();
    files.dedup();
    let canon = |s: &str| std::fs::canonicalize(s).unwrap_or_else(|_| PathBuf::from(s));
    let Some(main_name) = inclusions.iter().find(|(_, by)| by.is_none()).map(|(f, _)| f.clone()) else {
        return Err(format!("`{header}` included nothing"));
    };
    // A header of a library (found next to the source, or through `-I`)
    // brings the declarations of every header it includes, but not those of
    // system headers. A system header (`zlib.h`, `math.h`) brings its own
    // and those of the headers of its directory that it includes itself.
    let system = {
        let name = CString::new(main_name.as_str()).unwrap_or_default();
        unsafe {
            let file = (lib.clang_getFile)(tu, name.as_ptr());
            !file.is_null() && (lib.clang_Location_isInSystemHeader)((lib.clang_getLocation)(tu, file, 1, 1)) != 0
        }
    };
    let main = canon(&main_name);
    let dir = main.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let mut scope: HashSet<PathBuf> = HashSet::new();
    scope.insert(main.clone());
    for (f, by) in &inclusions {
        let f = canon(f);
        if by.as_deref().map(canon).as_ref() == Some(&main) && f.parent() == Some(dir.as_path()) {
            scope.insert(f);
        }
    }
    let mut g = Gen {
        lib,
        system,
        scope,
        files: HashMap::new(),
        items: Vec::new(),
        recs: HashMap::new(),
        rec_order: Vec::new(),
        rec_names: HashSet::new(),
        anon_names: HashMap::new(),
        ordinary: HashMap::new(),
        seen: HashSet::new(),
        hint: None,
        macros: Vec::new(),
    };
    let top = lib.children(unsafe { (lib.clang_getTranslationUnitCursor)(tu) });
    g.prepass(&top);
    for c in &top {
        g.top_level(*c);
    }
    let values = g.macro_values(index, include, args);
    // The records' cursors belong to the first translation unit.
    let source = g.render(header, &values);
    unsafe { (lib.clang_disposeTranslationUnit)(tu) };
    Ok((source, files))
}

fn parse(lib: &Lib, index: CXIndex, source: &str, args: &[String]) -> Result<CXTranslationUnit, String> {
    let name = CString::new("ore_cimport.c").unwrap();
    let contents = CString::new(source).map_err(|e| e.to_string())?;
    let cargs: Vec<CString> = args.iter().map(|a| CString::new(a.as_str()).unwrap_or_default()).collect();
    let ptrs: Vec<*const std::ffi::c_char> = cargs.iter().map(|a| a.as_ptr()).collect();
    let mut unsaved = CXUnsavedFile { name: name.as_ptr(), contents: contents.as_ptr(), length: source.len() as _ };
    let tu = unsafe {
        (lib.clang_parseTranslationUnit)(index, name.as_ptr(), ptrs.as_ptr(), ptrs.len() as c_int, &mut unsaved, 1, PARSE_OPTIONS)
    };
    if tu.is_null() { Err("libclang could not parse it".to_string()) } else { Ok(tu) }
}

/// The header's errors as a message, if it has any.
fn errors(lib: &Lib, tu: CXTranslationUnit, header: &str) -> Option<String> {
    let mut found = Vec::new();
    for i in 0..unsafe { (lib.clang_getNumDiagnostics)(tu) } {
        let d = unsafe { (lib.clang_getDiagnostic)(tu, i) };
        if unsafe { (lib.clang_getDiagnosticSeverity)(d) } >= SEVERITY_ERROR {
            let msg = lib.string(unsafe { (lib.clang_getDiagnosticSpelling)(d) });
            let (file, line, col) = lib.place(unsafe { (lib.clang_getDiagnosticLocation)(d) });
            found.push((file, line, col, msg));
        }
        unsafe { (lib.clang_disposeDiagnostic)(d) };
    }
    let (file, _, _, msg) = found.first()?;
    if file == "ore_cimport.c" && msg.contains("not found") {
        return Some(format!(
            "C header `{header}` not found: it is looked for next to the source file, then on the include path (add a directory with \"-Idir\", or `@link` the library so that its `pkg-config --cflags` are used)"
        ));
    }
    // Paths under the working directory are shown relative to it.
    let cwd = std::env::current_dir().ok().and_then(|d| std::fs::canonicalize(d).ok());
    let rel = |f: &str| match &cwd {
        Some(d) => std::path::Path::new(f).strip_prefix(d).map(|p| p.display().to_string()).unwrap_or_else(|_| f.to_string()),
        None => f.to_string(),
    };
    let shown: Vec<String> = found.iter().take(3).map(|(f, l, c, m)| format!("{}:{l}:{c}: {m}", rel(f))).collect();
    let more = if found.len() > 3 { format!(" (and {} more)", found.len() - 3) } else { String::new() };
    Some(format!("C header `{header}` has errors: {}{more}", shown.join("; ")))
}

/// Ore's keywords and the other names a declaration cannot take.
fn reserved(name: &str) -> bool {
    crate::lexer::is_keyword(name)
        || matches!(name, "true" | "false" | "null" | "self" | "Self" | "_")
        || crate::sema::is_type_name(name)
}

fn ident(name: &str) -> bool {
    let mut cs = name.chars();
    cs.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_') && cs.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// An Ore declaration, or a comment about a C declaration that has none.
enum Item {
    Fn { name: String, params: Vec<(String, String)>, ret: Option<String>, variadic: bool },
    Alias { name: String, ty: String },
    Const { name: String, ty: String, value: String },
    Record(String),
    Macro { name: String, index: usize },
    Note(String),
}

struct Rec {
    name: String,
    union: bool,
    /// The definition, or a declaration when there is none.
    decl: CXCursor,
    defined: bool,
    in_scope: bool,
    /// The fields (name, Ore type), or why the record is opaque.
    layout: Option<Result<Vec<(String, String)>, String>>,
    computing: bool,
    placed: bool,
}

/// What a top-level C name is, for collisions with struct tags: the record
/// a typedef names (`typedef struct foo foo`), if any.
type Ordinary = Option<String>;

struct Gen<'a> {
    lib: &'a Lib,
    /// The header is a system header: only the files of `scope` are in
    /// scope (else every file that is not a system header).
    system: bool,
    scope: HashSet<PathBuf>,
    /// For each file name: is it in scope?
    files: HashMap<String, bool>,
    items: Vec<Item>,
    /// Records by USR.
    recs: HashMap<String, Rec>,
    rec_order: Vec<String>,
    rec_names: HashSet<String>,
    /// Anonymous records and enums named by a typedef: USR to name.
    anon_names: HashMap<String, String>,
    /// Functions, variables, typedefs and enum constants of the whole
    /// translation unit.
    ordinary: HashMap<String, Ordinary>,
    /// Functions, typedefs and enums already done (redeclarations).
    seen: HashSet<String>,
    /// The name for an anonymous record met as the type of a field.
    hint: Option<String>,
    macros: Vec<String>,
}

/// A macro's value: Ore type and literal.
type Value = Option<(String, String)>;

impl Gen<'_> {
    fn s(&self, x: CXString) -> String {
        self.lib.string(x)
    }

    fn spelling(&self, c: CXCursor) -> String {
        self.s(unsafe { (self.lib.clang_getCursorSpelling)(c) })
    }

    fn usr(&self, c: CXCursor) -> String {
        self.s(unsafe { (self.lib.clang_getCursorUSR)(c) })
    }

    fn kind(&self, c: CXCursor) -> c_int {
        unsafe { (self.lib.clang_getCursorKind)(c) }
    }

    fn type_spelling(&self, t: CXType) -> String {
        self.s(unsafe { (self.lib.clang_getTypeSpelling)(t) })
    }

    fn canonical(&self, t: CXType) -> CXType {
        unsafe { (self.lib.clang_getCanonicalType)(t) }
    }

    fn decl_of(&self, t: CXType) -> CXCursor {
        unsafe { (self.lib.clang_getTypeDeclaration)(t) }
    }

    /// `t` without `struct`/`enum` sugar.
    fn strip(&self, mut t: CXType) -> CXType {
        while t.kind == T_ELABORATED {
            t = unsafe { (self.lib.clang_Type_getNamedType)(t) };
        }
        t
    }

    fn in_scope(&mut self, c: CXCursor) -> bool {
        let loc = unsafe { (self.lib.clang_getCursorLocation)(c) };
        let Some(file) = self.lib.file_of(loc) else { return false };
        if let Some(&b) = self.files.get(&file) {
            return b;
        }
        let b = if self.system {
            self.scope.contains(&std::fs::canonicalize(&file).unwrap_or_else(|_| PathBuf::from(&file)))
        } else {
            file != "ore_cimport.c" && unsafe { (self.lib.clang_Location_isInSystemHeader)(loc) } == 0
        };
        self.files.insert(file, b);
        b
    }

    fn anonymous(&self, c: CXCursor) -> bool {
        let s = self.spelling(c);
        s.is_empty() || s.contains(' ') || s.contains('(') || unsafe { (self.lib.clang_Cursor_isAnonymous)(c) } != 0
    }

    /// Names that struct tags and enum aliases must not take, and the names
    /// of anonymous records and enums.
    fn prepass(&mut self, top: &[CXCursor]) {
        for &c in top {
            match self.kind(c) {
                FUNCTION_DECL | VAR_DECL => {
                    self.ordinary.insert(self.spelling(c), None);
                }
                TYPEDEF_DECL => {
                    let name = self.spelling(c);
                    let under = self.strip(unsafe { (self.lib.clang_getTypedefDeclUnderlyingType)(c) });
                    let mut names = None;
                    if under.kind == T_RECORD || under.kind == T_ENUM {
                        let d = self.decl_of(under);
                        let usr = self.usr(d);
                        if self.anonymous(d) {
                            self.anon_names.entry(usr.clone()).or_insert_with(|| name.clone());
                        }
                        names = Some(usr);
                    }
                    self.ordinary.entry(name).or_insert(names);
                }
                ENUM_DECL => {
                    for k in self.lib.children(c) {
                        if self.kind(k) == ENUM_CONSTANT_DECL {
                            self.ordinary.insert(self.spelling(k), None);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn top_level(&mut self, c: CXCursor) {
        let kind = self.kind(c);
        if !matches!(kind, FUNCTION_DECL | VAR_DECL | TYPEDEF_DECL | ENUM_DECL | STRUCT_DECL | UNION_DECL | MACRO_DEFINITION) {
            return;
        }
        if !self.in_scope(c) {
            return;
        }
        match kind {
            FUNCTION_DECL => self.function(c),
            VAR_DECL => {
                let name = self.spelling(c);
                if self.seen.insert(name.clone()) {
                    self.items.push(Item::Note(format!("skipped `{name}`: a C variable (Ore declares only C functions)")));
                }
            }
            TYPEDEF_DECL => self.typedef(c),
            ENUM_DECL => self.enumeration(c),
            STRUCT_DECL | UNION_DECL => {
                let usr = self.usr(c);
                if usr.is_empty() {
                    return;
                }
                let def = unsafe { (self.lib.clang_getCursorDefinition)(c) };
                let is_def = unsafe { (self.lib.clang_Cursor_isNull)(def) } == 0;
                // Placed at the definition, or at the first declaration of a
                // record that the headers do not define.
                if is_def && self.lib.place(unsafe { (self.lib.clang_getCursorLocation)(def) }) != self.lib.place(unsafe { (self.lib.clang_getCursorLocation)(c) }) {
                    return;
                }
                self.rec_name(c);
                let r = self.recs.get_mut(&usr).unwrap();
                r.in_scope = true;
                if !r.placed {
                    r.placed = true;
                    self.items.push(Item::Record(usr));
                }
            }
            MACRO_DEFINITION => {
                let name = self.spelling(c);
                let builtin = unsafe { (self.lib.clang_Cursor_isMacroBuiltin)(c) } != 0;
                let fn_like = unsafe { (self.lib.clang_Cursor_isMacroFunctionLike)(c) } != 0;
                let reserved_c = name.starts_with("__") || (name.starts_with('_') && name[1..].starts_with(|c: char| c.is_ascii_uppercase()));
                if builtin || fn_like || reserved_c || !ident(&name) || reserved(&name) || self.ordinary.contains_key(&name) {
                    return;
                }
                if self.macros.contains(&name) {
                    return;
                }
                self.macros.push(name.clone());
                self.items.push(Item::Macro { name, index: self.macros.len() - 1 });
            }
            _ => {}
        }
    }

    // ------------------------------------------------------------ records

    /// The Ore name of a record, registering it.
    fn rec_name(&mut self, decl: CXCursor) -> String {
        let usr = self.usr(decl);
        if let Some(r) = self.recs.get(&usr) {
            return r.name.clone();
        }
        let union = self.kind(decl) == UNION_DECL;
        let def = unsafe { (self.lib.clang_getCursorDefinition)(decl) };
        let defined = unsafe { (self.lib.clang_Cursor_isNull)(def) } == 0;
        let tag = self.spelling(decl);
        let anonymous = self.anonymous(decl);
        let mut name = if anonymous {
            self.anon_names.get(&usr).cloned().or_else(|| self.hint.take()).unwrap_or_else(|| "anon".to_string())
        } else {
            tag
        };
        let clash = !anonymous && self.ordinary.get(&name).is_some_and(|o| o.as_deref() != Some(usr.as_str()));
        if clash || !ident(&name) || reserved(&name) {
            name = format!("{}_{name}", if union { "union" } else { "struct" });
        }
        let base = name.clone();
        let mut n = 2;
        while self.rec_names.contains(&name) {
            name = format!("{base}{n}");
            n += 1;
        }
        self.rec_names.insert(name.clone());
        let decl = if defined { def } else { decl };
        self.recs.insert(usr.clone(), Rec { name: name.clone(), union, decl, defined, in_scope: false, layout: None, computing: false, placed: false });
        self.rec_order.push(usr);
        name
    }

    /// A record used in a type: its name, or why it cannot be used by
    /// value when `by_value`.
    fn rec_ref(&mut self, decl: CXCursor, by_value: bool) -> Result<String, String> {
        let name = self.rec_name(decl);
        if by_value {
            let usr = self.usr(decl);
            if let Err(e) = self.layout(&usr) {
                let kw = if self.recs[&usr].union { "union" } else { "struct" };
                return Err(format!("`{kw} {name}` is opaque ({e})"));
            }
        }
        Ok(name)
    }

    /// The fields of a record, or why it has to be opaque: no definition,
    /// bit-fields, a field type Ore has no equivalent for, or a layout that
    /// is not C's natural one (packed or aligned).
    fn layout(&mut self, usr: &str) -> Result<Vec<(String, String)>, String> {
        let r = &self.recs[usr];
        if let Some(l) = &r.layout {
            return l.clone();
        }
        if !r.defined {
            return Err("the headers do not define it".to_string());
        }
        if r.computing {
            return Err("it contains itself".to_string());
        }
        let (decl, union, rname) = (r.decl, r.union, r.name.clone());
        self.recs.get_mut(usr).unwrap().computing = true;
        let result = self.compute_layout(decl, union, &rname);
        let r = self.recs.get_mut(usr).unwrap();
        r.computing = false;
        r.layout = Some(result.clone());
        result
    }

    fn compute_layout(&mut self, decl: CXCursor, union: bool, rname: &str) -> Result<Vec<(String, String)>, String> {
        let lib = self.lib;
        let t = unsafe { (lib.clang_getCursorType)(decl) };
        let (size, align) = unsafe { ((lib.clang_Type_getSizeOf)(t), (lib.clang_Type_getAlignOf)(t)) };
        let fields = lib.fields(t);
        let mut out = Vec::new();
        let (mut off, mut max_size, mut max_align) = (0i64, 0i64, 1i64);
        for (i, &f) in fields.iter().enumerate() {
            let fname = self.spelling(f);
            // An anonymous member's field has no name (or a description).
            let fname = if ident(&fname) { fname } else { String::new() };
            let shown = if fname.is_empty() { format!("anon{i}") } else { fname.clone() };
            if unsafe { (lib.clang_Cursor_isBitField)(f) } != 0 {
                return Err(format!("bit-field `{shown}`"));
            }
            let ft = unsafe { (lib.clang_getCursorType)(f) };
            if self.canonical(ft).kind == T_INCOMPLETE_ARRAY && i + 1 == fields.len() {
                // A flexible array member adds no size; it is left out.
                continue;
            }
            self.hint = Some(format!("{rname}_{shown}"));
            let ty = self.ty(ft, true);
            self.hint = None;
            let ty = ty.map_err(|e| format!("field `{shown}`: {e}"))?;
            let (fs, fa) = unsafe { ((lib.clang_Type_getSizeOf)(ft), (lib.clang_Type_getAlignOf)(ft)) };
            if fs < 0 || fa <= 0 {
                return Err(format!("field `{shown}` has no size"));
            }
            let at = unsafe { (lib.clang_Cursor_getOffsetOfField)(f) };
            let natural = if union { 0 } else { (off + fa - 1) / fa * fa };
            if at != natural * 8 {
                return Err("it is packed or has an aligned field".to_string());
            }
            off = natural + fs;
            max_size = max_size.max(fs);
            max_align = max_align.max(fa);
            let name = if fname.is_empty() {
                format!("anon{i}")
            } else if !ident(&fname) || reserved(&fname) {
                format!("{fname}_")
            } else {
                fname
            };
            out.push((name, ty));
        }
        let end = if union { max_size } else { off };
        if (end + max_align - 1) / max_align * max_align != size || align != max_align && !fields.is_empty() {
            return Err("it is packed or aligned".to_string());
        }
        Ok(out)
    }

    // -------------------------------------------------------------- types

    /// The Ore type of a C type. A record used `by_value` must not be
    /// opaque. `void` is "void" (only a return type takes it).
    fn ty(&mut self, t: CXType, by_value: bool) -> Result<String, String> {
        let lib = self.lib;
        let simple = match t.kind {
            T_VOID => "void",
            T_BOOL => "bool",
            T_CHAR_S | T_CHAR_U | T_UCHAR => "u8",
            T_SCHAR => "i8",
            T_SHORT => "c_short",
            T_USHORT => "c_ushort",
            T_INT => "c_int",
            T_UINT => "c_uint",
            T_LONG => "c_long",
            T_ULONG => "c_ulong",
            T_LONGLONG => "c_longlong",
            T_ULONGLONG => "c_ulonglong",
            T_FLOAT => "f32",
            T_DOUBLE => "f64",
            _ => "",
        };
        if !simple.is_empty() {
            return Ok(simple.to_string());
        }
        match t.kind {
            T_TYPEDEF => {
                let name = self.s(unsafe { (lib.clang_getTypedefName)(t) });
                match name.as_str() {
                    "size_t" | "uintptr_t" => return Ok("usize".to_string()),
                    "ssize_t" | "ptrdiff_t" | "intptr_t" => return Ok("isize".to_string()),
                    _ => {}
                }
                let under = unsafe { (lib.clang_getTypedefDeclUnderlyingType)(self.decl_of(t)) };
                self.ty(under, by_value)
            }
            T_ELABORATED => self.ty(self.strip(t), by_value),
            T_POINTER => self.pointer(unsafe { (lib.clang_getPointeeType)(t) }),
            T_RECORD => self.rec_ref(self.decl_of(t), by_value),
            T_ENUM => {
                let it = unsafe { (lib.clang_getEnumDeclIntegerType)(self.decl_of(t)) };
                self.ty(it, by_value)
            }
            T_CONSTANT_ARRAY => {
                let n = unsafe { (lib.clang_getArraySize)(t) };
                let elem = self.ty(unsafe { (lib.clang_getArrayElementType)(t) }, true)?;
                Ok(format!("[{n}]{elem}"))
            }
            T_FUNCTION_PROTO | T_FUNCTION_NO_PROTO => Err("a function type (Ore has function pointers only)".to_string()),
            _ => {
                let c = self.canonical(t);
                if c.kind != t.kind && c.kind != T_INVALID {
                    return self.ty(c, by_value);
                }
                Err(format!("the C type `{}` has no Ore equivalent", self.type_spelling(t)))
            }
        }
    }

    /// The type of a parameter: an array is a pointer to its first element.
    fn param(&mut self, t: CXType) -> Result<String, String> {
        let c = self.canonical(t);
        if c.kind == T_CONSTANT_ARRAY || c.kind == T_INCOMPLETE_ARRAY {
            return self.pointer(unsafe { (self.lib.clang_getArrayElementType)(c) });
        }
        self.ty(t, true)
    }

    /// A pointer to `p`: bytes for `void *` and `char *`, a function type
    /// for a function pointer, else a single-item pointer (`*mut T`, or
    /// `*T` to `const`), optional since C pointers may be null.
    fn pointer(&mut self, p: CXType) -> Result<String, String> {
        let lib = self.lib;
        let c = self.canonical(p);
        match c.kind {
            T_VOID | T_CHAR_S | T_CHAR_U | T_UCHAR => Ok("?[*]u8".to_string()),
            T_SCHAR => Ok("?[*]i8".to_string()),
            T_FUNCTION_NO_PROTO => Ok("?[*]u8".to_string()),
            T_FUNCTION_PROTO => {
                if unsafe { (lib.clang_isFunctionTypeVariadic)(c) } != 0 {
                    return Err("a pointer to a variadic function".to_string());
                }
                let mut params = Vec::new();
                for i in 0..unsafe { (lib.clang_getNumArgTypes)(c) }.max(0) {
                    params.push(self.param(unsafe { (lib.clang_getArgType)(c, i as u32) })?);
                }
                let ret = self.ty(unsafe { (lib.clang_getResultType)(c) }, true)?;
                let ret = if ret == "void" { String::new() } else { format!(" {ret}") };
                Ok(format!("?fn({}){ret}", params.join(", ")))
            }
            _ => {
                let inner = self.ty(p, false)?;
                let constant = unsafe { (lib.clang_isConstQualifiedType)(p) != 0 || (lib.clang_isConstQualifiedType)(c) != 0 };
                Ok(format!("{}{inner}", if constant { "?*" } else { "?*mut " }))
            }
        }
    }

    // ------------------------------------------------------- declarations

    fn function(&mut self, c: CXCursor) {
        let lib = self.lib;
        let name = self.spelling(c);
        if !self.seen.insert(name.clone()) {
            return;
        }
        let skip = |why: String| Item::Note(format!("skipped `{name}`: {why}"));
        if unsafe { (lib.clang_getCursorLinkage)(c) } != LINKAGE_EXTERNAL || unsafe { (lib.clang_Cursor_isFunctionInlined)(c) } != 0 {
            self.items.push(skip("an inline function (no symbol to link)".to_string()));
            return;
        }
        let mangled = self.s(unsafe { (lib.clang_Cursor_getMangling)(c) });
        let symbol = mangled.trim_start_matches('\u{1}');
        if symbol != name && symbol != format!("_{name}") {
            self.items.push(skip(format!("its symbol is `{symbol}` (an asm label)")));
            return;
        }
        if !ident(&name) || reserved(&name) {
            self.items.push(skip("the name is an Ore keyword".to_string()));
            return;
        }
        let t = unsafe { (lib.clang_getCursorType)(c) };
        let mut params = Vec::new();
        let mut variadic = false;
        if t.kind == T_FUNCTION_PROTO {
            for i in 0..unsafe { (lib.clang_getNumArgTypes)(t) }.max(0) as u32 {
                // The parameter's own type: the function type of a C
                // library function (`strlen`) is the builtin one, without
                // `size_t`.
                let arg = unsafe { (lib.clang_Cursor_getArgument)(c, i) };
                let pname = self.spelling(arg);
                let at = unsafe { (lib.clang_getCursorType)(arg) };
                let at = if at.kind == T_INVALID { unsafe { (lib.clang_getArgType)(t, i) } } else { at };
                match self.param(at) {
                    Ok(ty) => params.push((pname, ty)),
                    Err(e) => {
                        self.items.push(skip(format!("parameter {}: {e}", i + 1)));
                        return;
                    }
                }
            }
            variadic = unsafe { (lib.clang_isFunctionTypeVariadic)(t) } != 0;
        }
        // The result type as written (`size_t`), which is the first type
        // reference before the parameters.
        let mut rt = unsafe { (lib.clang_getCursorResultType)(c) };
        for k in lib.children(c) {
            match self.kind(k) {
                TYPE_REF => {
                    let written = unsafe { (lib.clang_getCursorType)(k) };
                    if written.kind == T_TYPEDEF && self.canonical(written).kind == self.canonical(rt).kind {
                        rt = written;
                    }
                    break;
                }
                PARM_DECL => break,
                _ => {}
            }
        }
        let ret = match self.ty(rt, true) {
            Ok(r) if r == "void" => None,
            Ok(r) => Some(r),
            Err(e) => {
                self.items.push(skip(format!("the result: {e}")));
                return;
            }
        };
        self.items.push(Item::Fn { name, params, ret, variadic });
    }

    fn typedef(&mut self, c: CXCursor) {
        let name = self.spelling(c);
        if !ident(&name) || reserved(&name) || !self.seen.insert(format!("typedef {name}")) {
            return;
        }
        let under = unsafe { (self.lib.clang_getTypedefDeclUnderlyingType)(c) };
        let stripped = self.strip(under);
        if stripped.kind == T_RECORD && self.rec_name(self.decl_of(stripped)) == name {
            return;
        }
        let canonical = self.canonical(under);
        if matches!(canonical.kind, T_FUNCTION_PROTO | T_FUNCTION_NO_PROTO) || canonical.kind == T_VOID {
            return;
        }
        let ty = match name.as_str() {
            "size_t" | "uintptr_t" => Ok("usize".to_string()),
            "ssize_t" | "ptrdiff_t" | "intptr_t" => Ok("isize".to_string()),
            _ => self.ty(under, true).or_else(|_| self.ty(under, false)),
        };
        match ty {
            Ok(ty) => self.items.push(Item::Alias { name, ty }),
            Err(e) => self.items.push(Item::Note(format!("skipped `{name}`: {e}"))),
        }
    }

    fn enumeration(&mut self, c: CXCursor) {
        let usr = self.usr(c);
        if !self.seen.insert(usr) {
            return;
        }
        let it_c = unsafe { (self.lib.clang_getEnumDeclIntegerType)(c) };
        let Ok(it) = self.ty(it_c, true) else { return };
        let unsigned = matches!(self.canonical(it_c).kind, T_BOOL | T_CHAR_U | T_UCHAR | T_USHORT | T_UINT | T_ULONG | T_ULONGLONG);
        let tag = self.spelling(c);
        if !self.anonymous(c) && ident(&tag) && !reserved(&tag) && !self.ordinary.contains_key(&tag) {
            self.items.push(Item::Alias { name: tag, ty: it.clone() });
        }
        for k in self.lib.children(c) {
            if self.kind(k) != ENUM_CONSTANT_DECL {
                continue;
            }
            let name = self.spelling(k);
            if !ident(&name) || reserved(&name) {
                continue;
            }
            let value = if unsigned {
                unsafe { (self.lib.clang_getEnumConstantDeclUnsignedValue)(k) }.to_string()
            } else {
                unsafe { (self.lib.clang_getEnumConstantDeclValue)(k) }.to_string()
            };
            self.items.push(Item::Const { name, ty: it.clone(), value });
        }
    }

    // ------------------------------------------------------------- macros

    /// The values of the macros that are constants: integers, floats and
    /// strings.
    fn macro_values(&mut self, index: CXIndex, include: &str, args: &[String]) -> Vec<Value> {
        let mut values: Vec<Value> = vec![None; self.macros.len()];
        if self.macros.is_empty() {
            return values;
        }
        let mut src = include.to_string();
        for (i, m) in self.macros.iter().enumerate() {
            // No parentheses: libclang evaluates a string literal only
            // when it is the whole initializer.
            src.push_str(&format!("__auto_type __ore_m_{i} = {m};\n"));
        }
        let mut args = args.to_vec();
        args.push("-ferror-limit=0".to_string());
        let Ok(tu) = parse(self.lib, index, &src, &args) else { return values };
        let lib = self.lib;
        // Macros that are not constants are errors here (a type, a call, a
        // statement); libclang must not evaluate those (it may crash).
        let mut bad = HashSet::new();
        for d in 0..unsafe { (lib.clang_getNumDiagnostics)(tu) } {
            let d = unsafe { (lib.clang_getDiagnostic)(tu, d) };
            if unsafe { (lib.clang_getDiagnosticSeverity)(d) } >= SEVERITY_ERROR {
                let (file, line, _) = lib.place(unsafe { (lib.clang_getDiagnosticLocation)(d) });
                if file == "ore_cimport.c" {
                    bad.insert(line);
                }
            }
            unsafe { (lib.clang_disposeDiagnostic)(d) };
        }
        for c in lib.children(unsafe { (lib.clang_getTranslationUnitCursor)(tu) }) {
            if self.kind(c) != VAR_DECL {
                continue;
            }
            let Some(i) = self.spelling(c).strip_prefix("__ore_m_").and_then(|n| n.parse::<usize>().ok()) else { continue };
            // The include is line 1 (and has no newline inside).
            if i >= values.len() || bad.contains(&(i as u32 + 2)) {
                continue;
            }
            let t = unsafe { (lib.clang_getCursorType)(c) };
            let canonical = self.canonical(t);
            let r = unsafe { (lib.clang_Cursor_Evaluate)(c) };
            if r.is_null() {
                continue;
            }
            let kind = unsafe { (lib.clang_EvalResult_getKind)(r) };
            values[i] = match kind {
                EVAL_INT => match self.ty(t, true) {
                    Ok(ty) if ty == "bool" => {
                        Some((ty, if unsafe { (lib.clang_EvalResult_getAsLongLong)(r) } != 0 { "true" } else { "false" }.to_string()))
                    }
                    Ok(ty) if !ty.starts_with(['?', '[']) => {
                        let unsigned = unsafe { (lib.clang_EvalResult_isUnsignedInt)(r) } != 0;
                        let v = if unsigned {
                            unsafe { (lib.clang_EvalResult_getAsUnsigned)(r) }.to_string()
                        } else {
                            unsafe { (lib.clang_EvalResult_getAsLongLong)(r) }.to_string()
                        };
                        Some((ty, v))
                    }
                    _ => None,
                },
                EVAL_FLOAT => {
                    let v = unsafe { (lib.clang_EvalResult_getAsDouble)(r) };
                    match canonical.kind {
                        T_FLOAT if v.is_finite() => Some(("f32".to_string(), float(v as f32 as f64, true))),
                        T_DOUBLE if v.is_finite() => Some(("f64".to_string(), float(v, false))),
                        _ => None,
                    }
                }
                EVAL_STR_LITERAL => {
                    let p = unsafe { (lib.clang_EvalResult_getAsStr)(r) };
                    (!p.is_null()).then(|| (String::new(), string(unsafe { CStr::from_ptr(p) }.to_bytes())))
                }
                _ => None,
            };
            unsafe { (lib.clang_EvalResult_dispose)(r) };
        }
        unsafe { (lib.clang_disposeTranslationUnit)(tu) };
        values
    }

    // ------------------------------------------------------------- output

    fn render(&mut self, header: &str, values: &[Value]) -> String {
        // Records met only in types (from other headers, or nested ones).
        let mut i = 0;
        while i < self.rec_order.len() {
            let usr = self.rec_order[i].clone();
            let r = &self.recs[&usr];
            if r.in_scope && r.defined {
                let _ = self.layout(&usr);
            }
            i += 1;
        }
        let mut items = std::mem::take(&mut self.items);
        let unplaced: Vec<String> = self.rec_order.iter().filter(|u| !self.recs[*u].placed).cloned().collect();
        if !unplaced.is_empty() {
            items.push(Item::Note("Types that the declarations above use, from other headers or nested in a struct.".to_string()));
            items.extend(unplaced.into_iter().map(Item::Record));
        }
        // Every top-level name, for parameters (no local may reuse one).
        let mut taken: HashSet<String> = self.rec_names.clone();
        let mut keep = Vec::new();
        for it in items {
            let name = match &it {
                Item::Fn { name, .. } | Item::Alias { name, .. } | Item::Const { name, .. } => Some(name.clone()),
                Item::Macro { name, index } => values[*index].as_ref().map(|_| name.clone()),
                _ => None,
            };
            if let Some(n) = name {
                if !taken.insert(n) {
                    continue;
                }
            }
            if matches!(&it, Item::Macro { index, .. } if values[*index].is_none()) {
                continue;
            }
            keep.push(it);
        }
        let mut out = format!(
            "// Generated by `ore cimport` from \"{header}\" with libclang: do not edit.\n// Declarations that Ore cannot express are listed as comments.\n"
        );
        let mut last_block = true;
        for it in &keep {
            let block = matches!(it, Item::Record(usr) if self.recs[usr].layout.as_ref().is_some_and(|l| l.as_ref().is_ok_and(|f| !f.is_empty())));
            if block || last_block {
                out.push('\n');
            }
            last_block = block;
            match it {
                Item::Fn { name, params, ret, variadic } => {
                    let mut ps: Vec<String> = Vec::new();
                    let mut used = HashSet::new();
                    for (i, (p, ty)) in params.iter().enumerate() {
                        let p = if ident(p) && !reserved(p) && !taken.contains(p) && used.insert(p.clone()) { p.clone() } else { format!("a{i}") };
                        ps.push(format!("{p}: {ty}"));
                    }
                    if *variadic {
                        ps.push("...".to_string());
                    }
                    let ret = ret.as_ref().map(|r| format!(" {r}")).unwrap_or_default();
                    out.push_str(&format!("pub extern fn {name}({}){ret}\n", ps.join(", ")));
                }
                Item::Alias { name, ty } => out.push_str(&format!("pub let {name} = {ty}\n")),
                Item::Const { name, ty, value } => out.push_str(&format!("pub let {name}: {ty} = {value}\n")),
                Item::Macro { name, index } => {
                    let (ty, v) = values[*index].as_ref().unwrap();
                    if ty.is_empty() {
                        out.push_str(&format!("pub let {name} = {v}\n"));
                    } else {
                        out.push_str(&format!("pub let {name}: {ty} = {v}\n"));
                    }
                }
                Item::Note(n) => out.push_str(&format!("// {n}\n")),
                Item::Record(usr) => {
                    let r = &self.recs[usr];
                    let kw = if r.union { "extern union" } else { "extern struct" };
                    match &r.layout {
                        Some(Ok(fields)) if !fields.is_empty() => {
                            out.push_str(&format!("pub {kw} {} {{\n", r.name));
                            for (f, ty) in fields {
                                out.push_str(&format!("    {f}: {ty}\n"));
                            }
                            out.push_str("}\n");
                        }
                        Some(Err(e)) if r.defined => {
                            out.push_str(&format!("// `{}` is opaque: {e}.\npub extern struct {} {{}}\n", r.name, r.name))
                        }
                        _ => out.push_str(&format!("pub extern struct {} {{}}\n", r.name)),
                    }
                }
            }
        }
        out
    }
}

/// A float literal Ore reads back as the same value.
fn float(v: f64, single: bool) -> String {
    let s = if single { format!("{:?}", v as f32) } else { format!("{v:?}") };
    if s.contains('e') && !s.contains('.') {
        // `1e300` has no point: Ore needs one before the exponent.
        s.replacen('e', ".0e", 1)
    } else {
        s
    }
}

/// An Ore string literal of `bytes`.
fn string(bytes: &[u8]) -> String {
    let mut s = String::from("\"");
    for &b in bytes {
        match b {
            b'"' => s.push_str("\\\""),
            b'\\' => s.push_str("\\\\"),
            b'\n' => s.push_str("\\n"),
            b'\t' => s.push_str("\\t"),
            b'\r' => s.push_str("\\r"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    s.push('"');
    s
}
