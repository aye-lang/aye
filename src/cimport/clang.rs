//! The part of libclang's C API that `@cimport` uses, loaded at run time
//! with `dlopen`, so the compiler builds and runs without libclang (only
//! `@cimport` needs it). The declarations follow `clang-c/Index.h`, which
//! has been stable since LLVM 3.x for everything used here.

#![allow(non_snake_case, non_camel_case_types)]

use std::ffi::{CStr, CString, c_char, c_int, c_longlong, c_uint, c_ulong, c_ulonglong, c_void};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CXString {
    data: *const c_void,
    flags: c_uint,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CXCursor {
    pub kind: c_int,
    xdata: c_int,
    data: [*const c_void; 3],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CXType {
    pub kind: c_int,
    data: [*mut c_void; 2],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CXSourceLocation {
    ptr_data: [*const c_void; 2],
    int_data: c_uint,
}

#[repr(C)]
pub struct CXUnsavedFile {
    pub name: *const c_char,
    pub contents: *const c_char,
    pub length: c_ulong,
}

pub type CXIndex = *mut c_void;
pub type CXTranslationUnit = *mut c_void;
pub type CXFile = *mut c_void;
pub type CXDiagnostic = *mut c_void;
pub type CXEvalResult = *mut c_void;
pub type CXCursorVisitor = extern "C" fn(CXCursor, CXCursor, *mut c_void) -> c_int;
pub type CXFieldVisitor = extern "C" fn(CXCursor, *mut c_void) -> c_int;
pub type CXInclusionVisitor = extern "C" fn(CXFile, *const CXSourceLocation, c_uint, *mut c_void);

// Cursor kinds.
pub const STRUCT_DECL: c_int = 2;
pub const UNION_DECL: c_int = 3;
pub const ENUM_DECL: c_int = 5;
pub const ENUM_CONSTANT_DECL: c_int = 7;
pub const FUNCTION_DECL: c_int = 8;
pub const VAR_DECL: c_int = 9;
pub const PARM_DECL: c_int = 10;
pub const TYPEDEF_DECL: c_int = 20;
pub const TYPE_REF: c_int = 43;
pub const MACRO_DEFINITION: c_int = 501;

// Type kinds.
pub const T_INVALID: c_int = 0;
pub const T_VOID: c_int = 2;
pub const T_BOOL: c_int = 3;
pub const T_CHAR_U: c_int = 4;
pub const T_UCHAR: c_int = 5;
pub const T_USHORT: c_int = 8;
pub const T_UINT: c_int = 9;
pub const T_ULONG: c_int = 10;
pub const T_ULONGLONG: c_int = 11;
pub const T_CHAR_S: c_int = 13;
pub const T_SCHAR: c_int = 14;
pub const T_SHORT: c_int = 16;
pub const T_INT: c_int = 17;
pub const T_LONG: c_int = 18;
pub const T_LONGLONG: c_int = 19;
pub const T_FLOAT: c_int = 21;
pub const T_DOUBLE: c_int = 22;
pub const T_POINTER: c_int = 101;
pub const T_RECORD: c_int = 105;
pub const T_ENUM: c_int = 106;
pub const T_TYPEDEF: c_int = 107;
pub const T_FUNCTION_NO_PROTO: c_int = 110;
pub const T_FUNCTION_PROTO: c_int = 111;
pub const T_CONSTANT_ARRAY: c_int = 112;
pub const T_INCOMPLETE_ARRAY: c_int = 114;
pub const T_ELABORATED: c_int = 119;

// Results.
pub const VISIT_CONTINUE: c_int = 1;
pub const LINKAGE_EXTERNAL: c_int = 4;
pub const EVAL_INT: c_int = 1;
pub const EVAL_FLOAT: c_int = 2;
pub const EVAL_STR_LITERAL: c_int = 4;
pub const SEVERITY_ERROR: c_int = 3;
// Parse options: a detailed preprocessing record (macro definitions),
// and no function bodies.
pub const PARSE_OPTIONS: c_uint = 0x01 | 0x40;

macro_rules! functions {
    ($($name:ident: fn($($arg:ty),*) $(-> $ret:ty)?;)*) => {
        /// libclang's functions.
        pub struct Lib {
            $(pub $name: unsafe extern "C" fn($($arg),*) $(-> $ret)?,)*
        }

        /// The functions of the library `handle`, or the first one it lacks.
        unsafe fn symbols(handle: *mut c_void) -> Result<Lib, String> {
            Ok(Lib {
                $($name: {
                    let p = unsafe { dlsym(handle, concat!(stringify!($name), "\0").as_ptr() as *const c_char) };
                    if p.is_null() {
                        return Err(format!("it has no `{}` (libclang is too old)", stringify!($name)));
                    }
                    unsafe { std::mem::transmute::<*mut c_void, unsafe extern "C" fn($($arg),*) $(-> $ret)?>(p) }
                },)*
            })
        }
    };
}

functions! {
    clang_createIndex: fn(c_int, c_int) -> CXIndex;
    clang_parseTranslationUnit: fn(CXIndex, *const c_char, *const *const c_char, c_int, *mut CXUnsavedFile, c_uint, c_uint) -> CXTranslationUnit;
    clang_disposeTranslationUnit: fn(CXTranslationUnit);
    clang_getTranslationUnitCursor: fn(CXTranslationUnit) -> CXCursor;
    clang_getNumDiagnostics: fn(CXTranslationUnit) -> c_uint;
    clang_getDiagnostic: fn(CXTranslationUnit, c_uint) -> CXDiagnostic;
    clang_getDiagnosticSeverity: fn(CXDiagnostic) -> c_int;
    clang_getDiagnosticSpelling: fn(CXDiagnostic) -> CXString;
    clang_getDiagnosticLocation: fn(CXDiagnostic) -> CXSourceLocation;
    clang_disposeDiagnostic: fn(CXDiagnostic);
    clang_getInclusions: fn(CXTranslationUnit, CXInclusionVisitor, *mut c_void);
    clang_getCString: fn(CXString) -> *const c_char;
    clang_disposeString: fn(CXString);
    clang_getFileName: fn(CXFile) -> CXString;
    clang_getFile: fn(CXTranslationUnit, *const c_char) -> CXFile;
    clang_getLocation: fn(CXTranslationUnit, CXFile, c_uint, c_uint) -> CXSourceLocation;
    clang_Location_isInSystemHeader: fn(CXSourceLocation) -> c_int;
    clang_getExpansionLocation: fn(CXSourceLocation, *mut CXFile, *mut c_uint, *mut c_uint, *mut c_uint);
    clang_visitChildren: fn(CXCursor, CXCursorVisitor, *mut c_void) -> c_uint;
    clang_Type_visitFields: fn(CXType, CXFieldVisitor, *mut c_void) -> c_uint;
    clang_getCursorKind: fn(CXCursor) -> c_int;
    clang_getCursorSpelling: fn(CXCursor) -> CXString;
    clang_getCursorUSR: fn(CXCursor) -> CXString;
    clang_getCursorLocation: fn(CXCursor) -> CXSourceLocation;
    clang_getCursorType: fn(CXCursor) -> CXType;
    clang_getCursorLinkage: fn(CXCursor) -> c_int;
    clang_getCursorDefinition: fn(CXCursor) -> CXCursor;
    clang_Cursor_isNull: fn(CXCursor) -> c_int;
    clang_Cursor_isAnonymous: fn(CXCursor) -> c_uint;
    clang_Cursor_isBitField: fn(CXCursor) -> c_uint;
    clang_Cursor_isMacroFunctionLike: fn(CXCursor) -> c_uint;
    clang_Cursor_isMacroBuiltin: fn(CXCursor) -> c_uint;
    clang_Cursor_isFunctionInlined: fn(CXCursor) -> c_uint;
    clang_Cursor_getMangling: fn(CXCursor) -> CXString;
    clang_Cursor_getArgument: fn(CXCursor, c_uint) -> CXCursor;
    clang_Cursor_getOffsetOfField: fn(CXCursor) -> c_longlong;
    clang_Cursor_Evaluate: fn(CXCursor) -> CXEvalResult;
    clang_EvalResult_getKind: fn(CXEvalResult) -> c_int;
    clang_EvalResult_getAsLongLong: fn(CXEvalResult) -> c_longlong;
    clang_EvalResult_isUnsignedInt: fn(CXEvalResult) -> c_uint;
    clang_EvalResult_getAsUnsigned: fn(CXEvalResult) -> c_ulonglong;
    clang_EvalResult_getAsDouble: fn(CXEvalResult) -> f64;
    clang_EvalResult_getAsStr: fn(CXEvalResult) -> *const c_char;
    clang_EvalResult_dispose: fn(CXEvalResult);
    clang_getEnumConstantDeclValue: fn(CXCursor) -> c_longlong;
    clang_getEnumConstantDeclUnsignedValue: fn(CXCursor) -> c_ulonglong;
    clang_getEnumDeclIntegerType: fn(CXCursor) -> CXType;
    clang_getTypedefDeclUnderlyingType: fn(CXCursor) -> CXType;
    clang_getTypeSpelling: fn(CXType) -> CXString;
    clang_getTypedefName: fn(CXType) -> CXString;
    clang_getCanonicalType: fn(CXType) -> CXType;
    clang_getPointeeType: fn(CXType) -> CXType;
    clang_getTypeDeclaration: fn(CXType) -> CXCursor;
    clang_getResultType: fn(CXType) -> CXType;
    clang_getCursorResultType: fn(CXCursor) -> CXType;
    clang_getNumArgTypes: fn(CXType) -> c_int;
    clang_getArgType: fn(CXType, c_uint) -> CXType;
    clang_isFunctionTypeVariadic: fn(CXType) -> c_uint;
    clang_isConstQualifiedType: fn(CXType) -> c_uint;
    clang_getArrayElementType: fn(CXType) -> CXType;
    clang_getArraySize: fn(CXType) -> c_longlong;
    clang_Type_getNamedType: fn(CXType) -> CXType;
    clang_Type_getSizeOf: fn(CXType) -> c_longlong;
    clang_Type_getAlignOf: fn(CXType) -> c_longlong;
}

unsafe extern "C" {
    fn dlopen(path: *const c_char, mode: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
    fn dlerror() -> *const c_char;
}

const RTLD_NOW: c_int = 2;

/// The library's file name on this system.
const LIB_NAME: &str = if cfg!(target_os = "macos") {
    "libclang.dylib"
} else {
    "libclang.so"
};

/// libclang, loaded the first time it is needed: the library, or why it
/// could not be loaded.
pub fn lib() -> Result<&'static Lib, String> {
    static LIB: OnceLock<Result<Lib, String>> = OnceLock::new();
    LIB.get_or_init(load).as_ref().map_err(|e| e.clone())
}

/// The places to look for libclang, in order. `LIBCLANG_PATH` (the
/// library or its directory) is the only one when it is set.
fn candidates() -> Vec<PathBuf> {
    let named = |p: PathBuf| if p.is_dir() { p.join(LIB_NAME) } else { p };
    if let Some(p) = std::env::var_os("LIBCLANG_PATH") {
        return vec![named(PathBuf::from(p))];
    }
    let mut v = Vec::new();
    let tool = |cmd: &str, args: &[&str]| {
        Command::new(cmd)
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    };
    if cfg!(target_os = "macos") {
        // The selected toolchain's clang: `.../usr/bin/clang` next to `.../usr/lib`.
        if let Some(clang) = tool("xcrun", &["--find", "clang"]) {
            if let Some(usr) = Path::new(&clang).parent().and_then(Path::parent) {
                v.push(usr.join("lib").join(LIB_NAME));
            }
        }
        v.push(PathBuf::from(
            "/Library/Developer/CommandLineTools/usr/lib/libclang.dylib",
        ));
        v.push(PathBuf::from("/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib/libclang.dylib"));
        v.push(PathBuf::from("/opt/homebrew/opt/llvm/lib/libclang.dylib"));
        v.push(PathBuf::from("/usr/local/opt/llvm/lib/libclang.dylib"));
    } else {
        if let Some(dir) = tool("llvm-config", &["--libdir"]) {
            v.push(Path::new(&dir).join(LIB_NAME));
        }
        for n in (10..=30).rev() {
            v.push(PathBuf::from(format!("/usr/lib/llvm-{n}/lib/libclang.so")));
            v.push(PathBuf::from(format!(
                "/usr/lib/llvm-{n}/lib/libclang.so.1"
            )));
        }
        v.push(PathBuf::from("/usr/lib64/libclang.so"));
        // The dynamic linker's own search path.
        v.push(PathBuf::from("libclang.so"));
        v.push(PathBuf::from("libclang.so.1"));
    }
    v
}

fn load() -> Result<Lib, String> {
    let mut tried = Vec::new();
    for path in candidates() {
        let bare = path.components().count() == 1;
        if !bare && !path.exists() {
            tried.push(path.display().to_string());
            continue;
        }
        let c = CString::new(path.to_string_lossy().as_bytes()).map_err(|e| e.to_string())?;
        let handle = unsafe { dlopen(c.as_ptr(), RTLD_NOW) };
        if handle.is_null() {
            let why = unsafe { dlerror() };
            let why = if why.is_null() {
                String::new()
            } else {
                unsafe { CStr::from_ptr(why) }
                    .to_string_lossy()
                    .into_owned()
            };
            if bare {
                tried.push(path.display().to_string());
            } else {
                tried.push(format!("{} ({why})", path.display()));
            }
            continue;
        }
        return unsafe { symbols(handle) }.map_err(|e| format!("{}: {e}", path.display()));
    }
    Err(format!("tried {}", tried.join(", ")))
}

impl Lib {
    /// A libclang string as a Rust string (and disposed).
    pub fn string(&self, s: CXString) -> String {
        unsafe {
            let p = (self.clang_getCString)(s);
            let out = if p.is_null() {
                String::new()
            } else {
                CStr::from_ptr(p).to_string_lossy().into_owned()
            };
            (self.clang_disposeString)(s);
            out
        }
    }

    pub fn children(&self, c: CXCursor) -> Vec<CXCursor> {
        extern "C" fn visit(c: CXCursor, _parent: CXCursor, data: *mut c_void) -> c_int {
            unsafe { (*(data as *mut Vec<CXCursor>)).push(c) };
            VISIT_CONTINUE
        }
        let mut v: Vec<CXCursor> = Vec::new();
        unsafe {
            (self.clang_visitChildren)(c, visit, &mut v as *mut Vec<CXCursor> as *mut c_void)
        };
        v
    }

    /// The fields of a record type, anonymous members included.
    pub fn fields(&self, t: CXType) -> Vec<CXCursor> {
        extern "C" fn visit(c: CXCursor, data: *mut c_void) -> c_int {
            unsafe { (*(data as *mut Vec<CXCursor>)).push(c) };
            VISIT_CONTINUE
        }
        let mut v: Vec<CXCursor> = Vec::new();
        unsafe {
            (self.clang_Type_visitFields)(t, visit, &mut v as *mut Vec<CXCursor> as *mut c_void)
        };
        v
    }

    /// Every file the translation unit includes, with the file that
    /// includes it (`None` for the main file's own includes).
    pub fn inclusions(&self, tu: CXTranslationUnit) -> Vec<(String, Option<String>)> {
        struct Data<'a> {
            lib: &'a Lib,
            out: Vec<(String, Option<String>)>,
        }
        extern "C" fn visit(
            file: CXFile,
            stack: *const CXSourceLocation,
            len: c_uint,
            data: *mut c_void,
        ) {
            let d = unsafe { &mut *(data as *mut Data) };
            if len == 0 {
                // The main file itself.
                return;
            }
            let name = d.lib.string(unsafe { (d.lib.clang_getFileName)(file) });
            // `stack[0]` is the `#include` that included `file`; the main
            // file's includes have a stack of one.
            let by = if len >= 2 {
                d.lib.file_of(unsafe { *stack })
            } else {
                None
            };
            d.out.push((name, by));
        }
        let mut d = Data {
            lib: self,
            out: Vec::new(),
        };
        unsafe { (self.clang_getInclusions)(tu, visit, &mut d as *mut Data as *mut c_void) };
        d.out
    }

    /// The file of a location (`None` for built-in definitions).
    pub fn file_of(&self, loc: CXSourceLocation) -> Option<String> {
        let mut file: CXFile = std::ptr::null_mut();
        unsafe {
            (self.clang_getExpansionLocation)(
                loc,
                &mut file,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if file.is_null() {
            None
        } else {
            Some(self.string(unsafe { (self.clang_getFileName)(file) }))
        }
    }

    /// The file, line and column of a location.
    pub fn place(&self, loc: CXSourceLocation) -> (String, u32, u32) {
        let mut file: CXFile = std::ptr::null_mut();
        let (mut line, mut col) = (0, 0);
        unsafe {
            (self.clang_getExpansionLocation)(
                loc,
                &mut file,
                &mut line,
                &mut col,
                std::ptr::null_mut(),
            )
        };
        let name = if file.is_null() {
            String::new()
        } else {
            self.string(unsafe { (self.clang_getFileName)(file) })
        };
        (name, line, col)
    }
}
