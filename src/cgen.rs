//! C backend: turns OIR into a single C translation unit for clang.
//!
//! The generated C includes only freestanding headers (stdint/stddef/stdbool)
//! so user `extern fn` declarations never clash with libc headers. The
//! runtime reaches libc through asm-labelled aliases (`ore_rt_printf` ->
//! `printf`). An `extern fn` is declared from its Ore signature, and an
//! `extern struct` is a C struct with its fields in declaration order (as
//! every struct is), so the C layout is what the Ore declaration says; it
//! must match the library's. A function type is a `typedef` of a C function
//! pointer, and a `pub extern fn` is defined under its own name, not
//! `static`. Overflow, bounds and cast checks are emitted as calls to small
//! inline helpers; `ORE_CHECKS` turns their checks off in `fast` mode.
//! `ORE_TRACE` (debug mode) keeps the error return trace that is printed when
//! `main` returns an error, and turns on the leak and double-free tracking of
//! `General`.
//!
//! In test mode the C `main` runs the test whose index is its argument.
//!
//! A source position reaches the runtime as two `int`s: the line, with the
//! file's index in its upper bits (`ORE_FILE`, `ORE_LINE`), and the column.

use crate::ast::BinOp;
use crate::diag::Span;
use crate::oir::*;
use crate::sema::{ConstVal, FloatTy, Intrinsic, IntTy, Type};
use std::collections::HashSet;
use std::fmt::Write;

const PRELUDE: &str = r#"#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#define ORE_RT static __attribute__((unused))
#define ORE_STR_(x) #x
#define ORE_STR(x) ORE_STR_(x)
#define ORE_SYM(n) ORE_STR(__USER_LABEL_PREFIX__) n
extern int ore_rt_printf(const char *, ...) __asm__(ORE_SYM("printf"));
extern int ore_rt_dprintf(int, const char *, ...) __asm__(ORE_SYM("dprintf"));
extern int ore_rt_fflush(void *) __asm__(ORE_SYM("fflush"));
extern void ore_rt_abort(void) __asm__(ORE_SYM("abort")) __attribute__((noreturn));
extern void ore_rt_exit(int) __asm__(ORE_SYM("exit")) __attribute__((noreturn));
extern void *ore_rt_malloc(size_t) __asm__(ORE_SYM("malloc"));
extern void ore_rt_free(void *) __asm__(ORE_SYM("free"));
extern int ore_rt_memcmp(const void *, const void *, size_t) __asm__(ORE_SYM("memcmp"));
extern int ore_rt_open(const char *, int, ...) __asm__(ORE_SYM("open"));
#define ORE_FILE(l) ore_files[(l) >> 20]
#define ORE_LINE(l) ((l) & 0xfffff)

ORE_RT __attribute__((noreturn, cold)) void ore_panic(int line, int col, const char *msg) {
    ore_rt_fflush(0);
    ore_rt_dprintf(2, "%s:%d:%d: panic: %s\n", ORE_FILE(line), ORE_LINE(line), col, msg);
    ore_rt_abort();
}
ORE_RT __attribute__((noreturn, cold)) void ore_panic_index(int line, int col, size_t i, size_t len) {
    ore_rt_fflush(0);
    ore_rt_dprintf(2, "%s:%d:%d: panic: index out of bounds: index %llu, len %llu\n", ORE_FILE(line), ORE_LINE(line), col,
                   (unsigned long long)i, (unsigned long long)len);
    ore_rt_abort();
}
ORE_RT __attribute__((noreturn, cold)) void ore_panic_slice(int line, int col, size_t lo, size_t hi, size_t len) {
    ore_rt_fflush(0);
    ore_rt_dprintf(2, "%s:%d:%d: panic: slice bounds out of range: [%llu..%llu] with len %llu\n", ORE_FILE(line),
                   ORE_LINE(line), col, (unsigned long long)lo, (unsigned long long)hi, (unsigned long long)len);
    ore_rt_abort();
}
ORE_RT inline size_t ore_idx(size_t i, size_t len, int line, int col) {
    if (ORE_CHECKS && i >= len) ore_panic_index(line, col, i, len);
    return i;
}
ORE_RT inline size_t ore_slice(size_t lo, size_t hi, size_t len, int line, int col) {
    if (ORE_CHECKS && (lo > hi || hi > len)) ore_panic_slice(line, col, lo, hi, len);
    return lo;
}
ORE_RT inline int64_t ore_cast_ss(int64_t v, int64_t lo, int64_t hi, int line, int col) {
    if (ORE_CHECKS && (v < lo || v > hi)) ore_panic(line, col, "integer cast out of range");
    return v;
}
ORE_RT inline int64_t ore_cast_su(int64_t v, uint64_t hi, int line, int col) {
    if (ORE_CHECKS && (v < 0 || (uint64_t)v > hi)) ore_panic(line, col, "integer cast out of range");
    return v;
}
ORE_RT inline uint64_t ore_cast_u(uint64_t v, uint64_t hi, int line, int col) {
    if (ORE_CHECKS && v > hi) ore_panic(line, col, "integer cast out of range");
    return v;
}
ORE_RT inline double ore_cast_fi(double v, double lo, double hi, int line, int col) {
    if (ORE_CHECKS && !(v > lo && v < hi)) ore_panic(line, col, "float to integer cast out of range");
    return v;
}

#define ORE_INT_OPS(N, T, UT, BITS, SIGNED, MIN)                                               \
    ORE_RT inline T ore_add_##N(T a, T b, int l, int c) {                                      \
        T r;                                                                                   \
        if (__builtin_add_overflow(a, b, &r) && ORE_CHECKS) ore_panic(l, c, "integer overflow"); \
        return r;                                                                              \
    }                                                                                          \
    ORE_RT inline T ore_sub_##N(T a, T b, int l, int c) {                                      \
        T r;                                                                                   \
        if (__builtin_sub_overflow(a, b, &r) && ORE_CHECKS) ore_panic(l, c, "integer overflow"); \
        return r;                                                                              \
    }                                                                                          \
    ORE_RT inline T ore_mul_##N(T a, T b, int l, int c) {                                      \
        T r;                                                                                   \
        if (__builtin_mul_overflow(a, b, &r) && ORE_CHECKS) ore_panic(l, c, "integer overflow"); \
        return r;                                                                              \
    }                                                                                          \
    ORE_RT inline T ore_div_##N(T a, T b, int l, int c) {                                      \
        if (ORE_CHECKS && b == 0) ore_panic(l, c, "division by zero");                         \
        if (ORE_CHECKS && SIGNED && a == MIN && b == (T)-1) ore_panic(l, c, "integer overflow"); \
        return (T)(a / b);                                                                     \
    }                                                                                          \
    ORE_RT inline T ore_rem_##N(T a, T b, int l, int c) {                                      \
        if (ORE_CHECKS && b == 0) ore_panic(l, c, "division by zero");                         \
        if (ORE_CHECKS && SIGNED && a == MIN && b == (T)-1) ore_panic(l, c, "integer overflow"); \
        return (T)(a % b);                                                                     \
    }                                                                                          \
    ORE_RT inline T ore_neg_##N(T a, int l, int c) {                                           \
        if (ORE_CHECKS && SIGNED && a == MIN) ore_panic(l, c, "integer overflow");             \
        return (T)-a;                                                                          \
    }                                                                                          \
    ORE_RT inline T ore_wadd_##N(T a, T b) { T r; __builtin_add_overflow(a, b, &r); return r; } \
    ORE_RT inline T ore_wsub_##N(T a, T b) { T r; __builtin_sub_overflow(a, b, &r); return r; } \
    ORE_RT inline T ore_wmul_##N(T a, T b) { T r; __builtin_mul_overflow(a, b, &r); return r; } \
    /* Shifts: the amount is masked when checks are off. A left shift is   \
       done unsigned and drops the bits shifted out; `>>` of a signed       \
       value is arithmetic. */                                              \
    ORE_RT inline T ore_shl_##N(T a, T b, int l, int c) {                                      \
        if (ORE_CHECKS && (uint64_t)b >= BITS) ore_panic(l, c, "shift amount out of range");   \
        return (T)((UT)a << ((UT)b & (BITS - 1)));                                             \
    }                                                                                          \
    ORE_RT inline T ore_shr_##N(T a, T b, int l, int c) {                                      \
        if (ORE_CHECKS && (uint64_t)b >= BITS) ore_panic(l, c, "shift amount out of range");   \
        return (T)(a >> ((UT)b & (BITS - 1)));                                                 \
    }
ORE_INT_OPS(i8, int8_t, uint8_t, 8, 1, INT8_MIN)
ORE_INT_OPS(i16, int16_t, uint16_t, 16, 1, INT16_MIN)
ORE_INT_OPS(i32, int32_t, uint32_t, 32, 1, INT32_MIN)
ORE_INT_OPS(i64, int64_t, uint64_t, 64, 1, INT64_MIN)
ORE_INT_OPS(isize, intptr_t, uintptr_t, 64, 1, INTPTR_MIN)
ORE_INT_OPS(u8, uint8_t, uint8_t, 8, 0, 0)
ORE_INT_OPS(u16, uint16_t, uint16_t, 16, 0, 0)
ORE_INT_OPS(u32, uint32_t, uint32_t, 32, 0, 0)
ORE_INT_OPS(u64, uint64_t, uint64_t, 64, 0, 0)
ORE_INT_OPS(usize, size_t, size_t, 64, 0, 0)

#define ORE_TRACE_MAX 64
static __attribute__((unused)) struct { int line, col; } ore_trace[ORE_TRACE_MAX];
static __attribute__((unused)) uint32_t ore_trace_len;
ORE_RT void ore_trace_push(int line, int col) {
    if (ore_trace_len < ORE_TRACE_MAX) {
        ore_trace[ore_trace_len].line = line;
        ore_trace[ore_trace_len].col = col;
    }
    ore_trace_len++;
}
/* `main` returned the error `e`: report it (with the trace in debug mode). */
ORE_RT int ore_main_error(uint16_t e) {
    ore_rt_fflush(0);
    ore_rt_dprintf(2, "error: %s\n", ore_error_names[e]);
    if (ORE_TRACE) {
        ore_rt_dprintf(2, "error return trace:\n");
        for (uint32_t i = 0; i < ore_trace_len && i < ORE_TRACE_MAX; i++) {
            ore_rt_dprintf(2, "%s:%d:%d\n", ORE_FILE(ore_trace[i].line), ORE_LINE(ore_trace[i].line), ore_trace[i].col);
        }
        if (ore_trace_len > ORE_TRACE_MAX) ore_rt_dprintf(2, "(%u more)\n", ore_trace_len - ORE_TRACE_MAX);
    }
    return 1;
}

/* Allocator: a data pointer and a vtable of byte-level functions. `line` and
   `col` are the Ore call that allocates or frees; leak and double-free
   reports name it. Alignment is at most 16, which malloc guarantees. */
typedef struct ore_alloc_vt {
    void *(*alloc)(void *self, size_t size, size_t align, int line, int col);
    void (*free)(void *self, void *p, int line, int col);
} ore_alloc_vt;
typedef struct ore_Allocator { void *ptr; const ore_alloc_vt *vt; } ore_Allocator;
ORE_RT void *ore_alloc(ore_Allocator a, size_t size, size_t align, int line, int col) {
    return a.vt->alloc(a.ptr, size, align, line, col);
}
ORE_RT void ore_free(ore_Allocator a, void *p, int line, int col) { a.vt->free(a.ptr, p, line, col); }
/* `n` elements of `size` bytes; null when the size overflows. */
ORE_RT void *ore_alloc_n(ore_Allocator a, size_t n, size_t size, size_t align, int line, int col) {
    if (size && n > SIZE_MAX / size) return 0;
    return ore_alloc(a, n * size, align, line, col);
}
/* Free a slice; an empty one was not allocated. */
ORE_RT void ore_free_n(ore_Allocator a, void *p, size_t n, int line, int col) {
    if (n) ore_free(a, p, line, col);
}

/* General: libc malloc/free. With `track` (debug mode, and the test
   allocator) every allocation gets a record with its site. Freed memory is
   kept until `deinit`, so addresses are not reused and every double free is
   caught. */
typedef struct ore_alloc_rec {
    struct ore_alloc_rec *next;
    void *p;
    size_t size;
    int line, col, free_line, free_col; /* free_line 0: not freed */
} ore_alloc_rec;
typedef struct ore_General { ore_alloc_rec *head, *tail; bool track; } ore_General;
ORE_RT void *ore_general_alloc(void *self, size_t size, size_t align, int line, int col) {
    ore_General *g = self;
    (void)align;
    void *p = ore_rt_malloc(size ? size : 1);
    if (!p || !g->track) return p;
    ore_alloc_rec *r = ore_rt_malloc(sizeof *r);
    if (!r) {
        ore_rt_free(p);
        return 0;
    }
    *r = (ore_alloc_rec){0, p, size, line, col, 0, 0};
    if (g->tail) g->tail->next = r; else g->head = r;
    g->tail = r;
    return p;
}
ORE_RT void ore_general_free(void *self, void *p, int line, int col) {
    ore_General *g = self;
    if (!g->track) {
        ore_rt_free(p);
        return;
    }
    for (ore_alloc_rec *r = g->head; r; r = r->next) {
        if (r->p != p) continue;
        if (r->free_line) {
            ore_rt_fflush(0);
            ore_rt_dprintf(2, "%s:%d:%d: panic: double free of memory allocated at %s:%d:%d and already freed at %s:%d:%d\n",
                           ORE_FILE(line), ORE_LINE(line), col, ORE_FILE(r->line), ORE_LINE(r->line), r->col,
                           ORE_FILE(r->free_line), ORE_LINE(r->free_line), r->free_col);
            ore_rt_abort();
        }
        r->free_line = line;
        r->free_col = col;
        return;
    }
    ore_panic(line, col, "free of memory that this allocator did not allocate");
}
static __attribute__((unused)) const ore_alloc_vt ore_general_vt = {ore_general_alloc, ore_general_free};
ORE_RT ore_General ore_general_init(void) { return (ore_General){0, 0, ORE_TRACE}; }
ORE_RT ore_Allocator ore_general_allocator(ore_General *g) { return (ore_Allocator){g, &ore_general_vt}; }
/* Report each allocation that was not freed, in allocation order, then
   free all memory. Returns the number of leaks. */
ORE_RT size_t ore_general_deinit(ore_General *g) {
    size_t leaks = 0;
    for (ore_alloc_rec *r = g->head, *next; r; r = next) {
        next = r->next;
        if (!r->free_line) {
            if (!leaks) ore_rt_fflush(0);
            ore_rt_dprintf(2, "%s:%d:%d: leak: %llu bytes allocated here were never freed\n", ORE_FILE(r->line),
                           ORE_LINE(r->line), r->col, (unsigned long long)r->size);
            leaks++;
        }
        ore_rt_free(r->p);
        ore_rt_free(r);
    }
    g->head = g->tail = 0;
    return leaks;
}

/* Arena: bump allocation in chunks from the backing allocator. `free` does
   nothing; `deinit` frees every chunk. */
typedef struct ore_arena_chunk { struct ore_arena_chunk *next; size_t size; } ore_arena_chunk;
typedef struct ore_Arena { ore_Allocator backing; ore_arena_chunk *chunks; size_t used; } ore_Arena;
#define ORE_ARENA_CHUNK 4096
ORE_RT void *ore_arena_alloc(void *self, size_t size, size_t align, int line, int col) {
    ore_Arena *a = self;
    ore_arena_chunk *c = a->chunks;
    if (c) {
        size_t off = (a->used + align - 1) & ~(align - 1);
        if (off <= c->size && size <= c->size - off) {
            a->used = off + size;
            return (char *)(c + 1) + off;
        }
    }
    size_t cap = size > ORE_ARENA_CHUNK ? size : ORE_ARENA_CHUNK;
    c = ore_alloc(a->backing, sizeof(ore_arena_chunk) + cap, 16, line, col);
    if (!c) return 0;
    c->next = a->chunks;
    c->size = cap;
    a->chunks = c;
    a->used = size;
    return c + 1;
}
ORE_RT void ore_arena_free(void *self, void *p, int line, int col) {}
static __attribute__((unused)) const ore_alloc_vt ore_arena_vt = {ore_arena_alloc, ore_arena_free};
ORE_RT ore_Arena ore_arena_init(ore_Allocator backing) { return (ore_Arena){backing, 0, 0}; }
ORE_RT ore_Allocator ore_arena_allocator(ore_Arena *a) { return (ore_Allocator){a, &ore_arena_vt}; }
ORE_RT void ore_arena_deinit(ore_Arena *a, int line, int col) {
    for (ore_arena_chunk *c = a->chunks, *next; c; c = next) {
        next = c->next;
        ore_free(a->backing, c, line, col);
    }
    a->chunks = 0;
    a->used = 0;
}

/* Tests: the leak-checked allocator, assertions, and the end of a test. A
   failed assertion ends the test process with status 1. */
static __attribute__((unused)) ore_General ore_test_gpa = {0, 0, true};
ORE_RT ore_Allocator ore_test_allocator(void) { return ore_general_allocator(&ore_test_gpa); }
#define ORE_ASSERT_FAILED(line, col, fmt, ...)                                                                  \
    do {                                                                                                         \
        ore_rt_fflush(0);                                                                                        \
        ore_rt_dprintf(2, "%s:%d:%d: assertion failed: " fmt "\n", ORE_FILE(line), ORE_LINE(line), col, __VA_ARGS__); \
        ore_rt_exit(1);                                                                                          \
    } while (0)
ORE_RT void ore_expect(bool c, int line, int col) {
    if (!c) ORE_ASSERT_FAILED(line, col, "%s", "condition is false");
}
ORE_RT void ore_expect_eq_i(int64_t want, int64_t got, int line, int col) {
    if (want != got) ORE_ASSERT_FAILED(line, col, "expected %lld, found %lld", (long long)want, (long long)got);
}
ORE_RT void ore_expect_eq_u(uint64_t want, uint64_t got, int line, int col) {
    if (want != got)
        ORE_ASSERT_FAILED(line, col, "expected %llu, found %llu", (unsigned long long)want, (unsigned long long)got);
}
ORE_RT void ore_expect_eq_f(double want, double got, int line, int col) {
    if (want != got) ORE_ASSERT_FAILED(line, col, "expected %g, found %g", want, got);
}
ORE_RT void ore_expect_eq_b(bool want, bool got, int line, int col) {
    if (want != got) ORE_ASSERT_FAILED(line, col, "expected %s, found %s", want ? "true" : "false", got ? "true" : "false");
}
ORE_RT void ore_expect_eq_s(const uint8_t *wp, size_t wn, const uint8_t *gp, size_t gn, int line, int col) {
    if (wn != gn || ore_rt_memcmp(wp, gp, wn) != 0)
        ORE_ASSERT_FAILED(line, col, "expected \"%.*s\", found \"%.*s\"", (int)wn, (const char *)wp, (int)gn, (const char *)gp);
}
/* A test returned `err` (0: no error): report it (with the trace in debug
   mode) and the leaks of the test allocator. Returns the exit status. */
ORE_RT int ore_test_end(uint16_t err) {
    int status = err ? ore_main_error(err) : 0;
    if (ore_general_deinit(&ore_test_gpa)) status = 1;
    return status;
}

ORE_RT void ore_print_i(int64_t v, bool nl) { ore_rt_printf(nl ? "%lld\n" : "%lld", (long long)v); }
ORE_RT void ore_print_u(uint64_t v, bool nl) { ore_rt_printf(nl ? "%llu\n" : "%llu", (unsigned long long)v); }
ORE_RT void ore_print_f(double v, bool nl) { ore_rt_printf(nl ? "%g\n" : "%g", v); }
ORE_RT void ore_print_b(bool v, bool nl) { ore_rt_printf(nl ? "%s\n" : "%s", v ? "true" : "false"); }
ORE_RT void ore_print_s(const uint8_t *p, size_t n, bool nl) {
    ore_rt_printf(nl ? "%.*s\n" : "%.*s", (int)n, (const char *)p);
}

/* std.log: each piece of the format and argument is written to stderr. */
ORE_RT void ore_log_s(const uint8_t *p, size_t n) { ore_rt_dprintf(2, "%.*s", (int)n, (const char *)p); }
ORE_RT void ore_log_i(int64_t v) { ore_rt_dprintf(2, "%lld", (long long)v); }
ORE_RT void ore_log_u(uint64_t v) { ore_rt_dprintf(2, "%llu", (unsigned long long)v); }
ORE_RT void ore_log_f(double v) { ore_rt_dprintf(2, "%g", v); }
ORE_RT void ore_log_b(bool v) { ore_rt_dprintf(2, "%s", v ? "true" : "false"); }
ORE_RT void ore_log_e(uint16_t e) { ore_rt_dprintf(2, "%s", ore_error_names[e]); }

/* Hooks that std calls through `extern fn`: the program's arguments, and
   `open` with a path that is not NUL-terminated (the fd, or -errno). */
static int ore_argc;
static char **ore_argv;
ORE_RT int32_t ore_std_argc(void) { return ore_argc; }
ORE_RT uint8_t *ore_std_argv(int32_t i) { return (uint8_t *)ore_argv[i]; }
#ifdef __APPLE__
extern int *ore_rt_errno_loc(void) __asm__(ORE_SYM("__error"));
#else
extern int *ore_rt_errno_loc(void) __asm__(ORE_SYM("__errno_location"));
#endif
ORE_RT int32_t ore_std_open(uint8_t *path, size_t len) {
    char *p = ore_rt_malloc(len + 1);
    if (!p) return -12;
    for (size_t i = 0; i < len; i++) p[i] = (char)path[i];
    p[len] = 0;
    int fd = ore_rt_open(p, 0);
    int err = fd < 0 ? *ore_rt_errno_loc() : 0;
    ore_rt_free(p);
    return fd < 0 ? -err : fd;
}
"#;

/// Emit the C translation unit for `p`. `files` are the Ore source names
/// (indexed by `Span::file`) used in `#line` directives and panic messages;
/// `checks` enables runtime checks and `trace` the error return trace (debug
/// mode). With `tests`, `main` runs one test instead of `ore_f_main`.
pub fn emit(p: &Program, files: &[String], checks: bool, trace: bool, tests: bool) -> String {
    let files = files.iter().map(|f| c_str(f.as_bytes())).collect();
    let mut g = Gen { out: String::new(), files, trace, loops: Vec::new(), next_label: 0 };
    g.program(p, checks, tests);
    g.out
}

/// A struct or function name as a C identifier. An instance of a generic
/// (`Pool(Node)`) starts with `0`, which no Ore name does, and has `_`
/// doubled and other characters as `_` and two hex digits.
fn c_name(name: &str) -> String {
    if name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return name.to_string();
    }
    let mut s = String::from("0");
    for c in name.chars() {
        match c {
            '_' => s.push_str("__"),
            c if c.is_ascii_alphanumeric() => s.push(c),
            c => {
                let _ = write!(s, "_{:02x}", c as u32);
            }
        }
    }
    s
}

/// The C symbol of an `extern fn`: its name without the module.
fn extern_symbol(name: &str) -> &str {
    name.rsplit("::").next().unwrap()
}

/// C name of the Ore function `name`: `f`, `Struct.method` or `test#N`.
fn fn_name(name: &str) -> String {
    if let Some(n) = name.strip_prefix("test#") {
        return format!("ore_t_{n}");
    }
    match name.rsplit_once('.') {
        Some((s, m)) => {
            let s = c_name(s);
            format!("ore_m_{}{s}_{m}", s.len())
        }
        None => format!("ore_f_{}", c_name(name)),
    }
}

/// C names of the vtable of struct `s` for interface `i`, and of the thunk
/// for its method `m`.
fn vtable_name(i: &str, s: &str) -> String {
    let i = c_name(i);
    format!("ore_vt_{}{i}_{}", i.len(), c_name(s))
}

fn thunk_name(i: &str, s: &str, m: &str) -> String {
    let (i, s) = (c_name(i), c_name(s));
    format!("ore_th_{}{i}_{}{s}_{m}", i.len(), s.len())
}

struct Gen {
    out: String,
    /// The source file names as C string literals.
    files: Vec<String>,
    trace: bool,
    /// For each enclosing loop: the label `continue` jumps to, if any.
    loops: Vec<Option<String>>,
    next_label: usize,
}

const C_KEYWORDS: &[&str] = &[
    "auto", "break", "case", "char", "const", "continue", "default", "do", "double", "else", "enum", "extern",
    "float", "for", "goto", "if", "inline", "int", "long", "register", "restrict", "return", "short", "signed",
    "sizeof", "static", "struct", "switch", "typedef", "union", "unsigned", "void", "volatile", "while", "bool",
];

/// C name of a struct field (C keywords get a trailing underscore).
fn c_field(name: &str) -> String {
    if C_KEYWORDS.contains(&name) { format!("{name}_") } else { name.to_string() }
}

fn c_str(bytes: &[u8]) -> String {
    let mut s = String::from("\"");
    for &b in bytes {
        match b {
            b'"' => s.push_str("\\\""),
            b'\\' => s.push_str("\\\\"),
            b'\n' => s.push_str("\\n"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7e => s.push(b as char),
            _ => {
                let _ = write!(s, "\\{b:03o}");
            }
        }
    }
    s.push('"');
    s
}

/// Integer constant as a C expression of type `long long` or `unsigned long long`.
fn c_int(v: i128) -> String {
    if v == i64::MIN as i128 {
        "(-9223372036854775807LL - 1)".to_string()
    } else if v < 0 {
        format!("({v}LL)")
    } else if v <= i64::MAX as i128 {
        format!("{v}LL")
    } else {
        format!("{v}ULL")
    }
}

fn c_float(v: f64) -> String {
    let s = format!("{v:?}");
    if s.contains(['.', 'e', 'E']) { s } else { format!("{s}.0") }
}

fn int_c(it: IntTy) -> &'static str {
    match it {
        IntTy::I8 => "int8_t",
        IntTy::I16 => "int16_t",
        IntTy::I32 => "int32_t",
        IntTy::I64 => "int64_t",
        IntTy::Isize => "intptr_t",
        IntTy::U8 => "uint8_t",
        IntTy::U16 => "uint16_t",
        IntTy::U32 => "uint32_t",
        IntTy::U64 => "uint64_t",
        IntTy::Usize => "size_t",
    }
}

fn mangle(t: &Type) -> String {
    match t {
        Type::Int(i) => i.name().to_string(),
        Type::Float(FloatTy::F32) => "f32".into(),
        Type::Float(FloatTy::F64) => "f64".into(),
        Type::Bool => "bool".into(),
        Type::Void => "void".into(),
        Type::Array(e, n) => format!("A{n}_{}", mangle(e)),
        // `[]T` and `[]mut T` share one C type.
        Type::Slice(e, _) => format!("S_{}", mangle(e)),
        Type::CPtr(e) => format!("P_{}", mangle(e)),
        Type::Ptr(e, false) => format!("R_{}", mangle(e)),
        Type::Ptr(e, true) => format!("RM_{}", mangle(e)),
        Type::Optional(e) => format!("O_{}", mangle(e)),
        Type::Struct(n) => format!("s_{}", c_name(n)),
        Type::Enum(n) => format!("e_{}", c_name(n)),
        // Every error set is a `uint16_t`, so the set is not part of the name.
        Type::ErrorSet(_) => "err".into(),
        Type::ErrorUnion(_, t) => format!("U_{}", mangle(t)),
        Type::Dyn(i) => format!("D_{}", c_name(i)),
        Type::Builtin(b) => b.c_name().into(),
        // Each part with its length first, so that the name is unambiguous.
        Type::Fn(params, ret) => {
            let parts: String = params.iter().chain([&**ret]).map(|t| {
                let m = mangle(t);
                format!("{}{m}", m.len())
            }).collect();
            format!("F{}_{parts}", params.len())
        }
        Type::Param(..) | Type::Error => unreachable!(),
    }
}

fn c_type(t: &Type) -> String {
    match t {
        Type::Int(i) => int_c(*i).into(),
        Type::Float(FloatTy::F32) => "float".into(),
        Type::Float(FloatTy::F64) => "double".into(),
        Type::Bool => "bool".into(),
        Type::Void => "void".into(),
        Type::ErrorSet(_) => "uint16_t".into(),
        Type::CPtr(e) | Type::Ptr(e, _) => format!("{}*", c_type(e)),
        Type::Optional(e) if is_ptr_optional(t) => c_type(e),
        Type::Array(..)
        | Type::Slice(..)
        | Type::Struct(_)
        | Type::Enum(_)
        | Type::Optional(_)
        | Type::ErrorUnion(..)
        | Type::Dyn(_)
        | Type::Builtin(_)
        | Type::Fn(..) => format!("ore_{}", mangle(t)),
        Type::Param(..) | Type::Error => unreachable!(),
    }
}

/// Types emitted as a C struct.
fn is_composite(t: &Type) -> bool {
    match t {
        Type::Array(..) | Type::Slice(..) | Type::Struct(_) | Type::Enum(_) | Type::ErrorUnion(..) | Type::Dyn(_) => true,
        Type::Optional(_) => !is_ptr_optional(t),
        _ => false,
    }
}

/// An integer constant of type `it`.
fn c_int_of(v: i128, it: IntTy) -> String {
    if (-(1i128 << 31) < v) && (v < (1i128 << 31)) {
        format!("(({}){v})", int_c(it))
    } else {
        format!("(({}){})", int_c(it), c_int(v))
    }
}

/// A float constant of type `ft`.
fn c_float_of(v: f64, ft: FloatTy) -> String {
    match ft {
        FloatTy::F32 => format!("((float){})", c_float(v)),
        FloatTy::F64 => c_float(v),
    }
}

/// The C name of the global `name`.
fn global_name(name: &str) -> String {
    format!("ore_g_{}", c_name(name))
}

/// A position for the runtime: the line with the file index in its upper
/// bits, and the column.
fn line_col(span: Span) -> String {
    format!("{}, {}", (span.file as u32) << 20 | span.line, span.col)
}

impl Gen {
    fn line(&mut self, indent: usize, s: &str) {
        for _ in 0..indent {
            self.out.push_str("    ");
        }
        self.out.push_str(s);
        self.out.push('\n');
    }

    fn program(&mut self, p: &Program, checks: bool, tests: bool) {
        self.out.push_str("/* Generated by the Ore stage0 compiler. Do not edit. */\n");
        let _ = writeln!(self.out, "#define ORE_CHECKS {}", checks as u8);
        let _ = writeln!(self.out, "#define ORE_TRACE {}", self.trace as u8);
        let _ = writeln!(self.out, "static const char *const ore_files[] = {{{}}};", self.files.join(", "));
        let names: Vec<String> =
            std::iter::once("\"\"".to_string()).chain(p.errors.iter().map(|n| c_str(n.as_bytes()))).collect();
        let _ = writeln!(self.out, "static const char *const ore_error_names[] = {{{}}};", names.join(", "));
        self.out.push_str(PRELUDE);
        self.types(p);
        // The vtable type of each interface: one function pointer per method.
        for i in &p.ifaces {
            let mut s = format!("struct ore_VT_{} {{", c_name(&i.name));
            for m in &i.methods {
                let params: String = m.params.iter().map(|t| format!(", {}", c_type(t))).collect();
                let _ = write!(s, " {} (*{})(void *{params});", c_type(&m.ret), c_field(&m.name));
            }
            s.push_str(" };");
            self.line(0, &s);
        }

        self.out.push('\n');
        // Modules that declare the same C function share one declaration.
        let mut declared = HashSet::new();
        for e in &p.externs {
            let mut params: Vec<String> = e.params.iter().map(c_type).collect();
            if e.variadic {
                params.push("...".to_string());
            }
            let params = if params.is_empty() { "void".to_string() } else { params.join(", ") };
            let decl = format!("extern {} {}({params});", c_type(&e.ret), extern_symbol(&e.name));
            if declared.insert(decl.clone()) {
                self.line(0, &decl);
            }
        }
        // Prototypes come before the globals, whose values can be functions.
        for f in &p.funcs {
            let sig = self.signature(f);
            let _ = writeln!(self.out, "{sig};");
        }
        for g in &p.globals {
            let init = const_init(p, &g.value, &g.ty);
            let _ = writeln!(self.out, "static __attribute__((unused)) {} {} = {init};", c_type(&g.ty), global_name(&g.name));
        }
        for (i, s) in &p.vtables {
            self.vtable(p, i, s);
        }
        for f in &p.funcs {
            self.func(f);
        }
        if tests {
            self.out.push_str("\nint main(int argc, char **argv) {\n    int n = 0;\n    ore_argc = argc;\n    ore_argv = argv;\n    if (argc != 2) return 2;\n");
            self.out.push_str("    for (const char *s = argv[1]; *s; s++) n = n * 10 + (*s - '0');\n    switch (n) {\n");
            for i in 0..p.tests.len() {
                let _ = writeln!(self.out, "    case {i}: return ore_test_end(ore_t_{i}().err);");
            }
            self.out.push_str("    }\n    return 2;\n}\n");
            return;
        }
        let main = p.funcs.iter().find(|f| f.is_main).unwrap();
        if let Type::ErrorUnion(..) = main.ret {
            let _ = write!(
                self.out,
                "\nint main(int argc, char **argv) {{\n    ore_argc = argc;\n    ore_argv = argv;\n    {} r = ore_f_main();\n    return r.err ? ore_main_error(r.err) : 0;\n}}\n",
                c_type(&main.ret)
            );
        } else {
            self.out.push_str("\nint main(int argc, char **argv) {\n    ore_argc = argc;\n    ore_argv = argv;\n    ore_f_main();\n    return 0;\n}\n");
        }
    }

    /// The vtable of struct `s` for interface `i`, and a thunk per method that
    /// passes the data pointer as the method's receiver.
    fn vtable(&mut self, p: &Program, i: &str, s: &str) {
        let iface = p.ifaces.iter().find(|x| x.name == i).unwrap();
        let mut thunks = Vec::new();
        for m in &iface.methods {
            let f = p.funcs.iter().find(|f| f.name == format!("{s}.{}", m.name)).unwrap();
            let st = c_type(&Type::Struct(s.to_string()));
            let recv = match f.locals[f.params[0]].ty {
                Type::Ptr(..) => format!("({st} *)self"),
                _ => format!("*({st} *)self"),
            };
            let params: String = m.params.iter().enumerate().map(|(k, t)| format!(", {} a{k}", c_type(t))).collect();
            let args: String = (0..m.params.len()).map(|k| format!(", a{k}")).collect();
            let ret = if m.ret == Type::Void { "" } else { "return " };
            let th = thunk_name(i, s, &m.name);
            let _ = writeln!(
                self.out,
                "static {} {th}(void *self{params}) {{ {ret}{}({recv}{args}); }}",
                c_type(&m.ret),
                fn_name(&f.name)
            );
            thunks.push(th);
        }
        let _ = writeln!(self.out, "static const struct ore_VT_{} {} = {{{}}};", c_name(i), vtable_name(i, s), thunks.join(", "));
    }

    /// Typedefs for every array, slice and struct type, in dependency order.
    fn types(&mut self, p: &Program) {
        let mut all: Vec<Type> = Vec::new();
        let mut seen = HashSet::new();
        let mut add = |t: &Type, all: &mut Vec<Type>| collect_type(t, &mut seen, all);
        add(&Type::Slice(Box::new(Type::Int(IntTy::U8)), false), &mut all);
        for s in &p.structs {
            add(&Type::Struct(s.name.clone()), &mut all);
            for (_, t) in &s.fields {
                add(t, &mut all);
            }
        }
        for e in &p.enums {
            add(&Type::Enum(e.name.clone()), &mut all);
            for (_, t) in e.variants.iter().flat_map(|v| &v.fields) {
                add(t, &mut all);
            }
        }
        for e in &p.externs {
            for t in e.params.iter().chain(std::iter::once(&e.ret)) {
                add(t, &mut all);
            }
        }
        for g in &p.globals {
            add(&g.ty, &mut all);
        }
        for f in &p.funcs {
            add(&f.ret, &mut all);
            for l in &f.locals {
                add(&l.ty, &mut all);
            }
            let mut sized = Vec::new();
            sizeof_types(&f.body, &mut sized);
            for t in &sized {
                add(t, &mut all);
            }
        }
        for m in p.ifaces.iter().flat_map(|i| &i.methods) {
            for t in m.params.iter().chain(std::iter::once(&m.ret)) {
                add(t, &mut all);
            }
        }
        // Types that differ only in their error sets share one C type.
        let mut names = HashSet::new();
        let composite: Vec<&Type> = all.iter().filter(|t| is_composite(t) && names.insert(c_type(t))).collect();
        self.out.push('\n');
        for t in &composite {
            let n = c_type(t);
            let _ = writeln!(self.out, "typedef struct {n} {n};");
        }
        // Function pointer types, after the types they name (`all` lists a
        // type after its parts) and before the structs that hold them.
        let mut fns = HashSet::new();
        for t in all.iter().filter(|t| matches!(t, Type::Fn(..))) {
            let Type::Fn(params, ret) = t else { unreachable!() };
            let n = c_type(t);
            if fns.insert(n.clone()) {
                let ps: Vec<String> = params.iter().map(c_type).collect();
                let ps = if ps.is_empty() { "void".to_string() } else { ps.join(", ") };
                let _ = writeln!(self.out, "typedef {} (*{n})({ps});", c_type(ret));
            }
        }
        // Definitions: a type's by-value components must be complete first.
        let mut done = HashSet::new();
        for t in &composite {
            self.define(t, p, &mut done);
        }
    }

    fn define(&mut self, t: &Type, p: &Program, done: &mut HashSet<String>) {
        if !is_composite(t) || !done.insert(c_type(t)) {
            return;
        }
        let name = c_type(t);
        match t {
            Type::Array(e, n) => {
                self.define(e, p, done);
                let _ = writeln!(self.out, "struct {name} {{ {} a[{n}]; }};", c_type(e));
            }
            Type::Slice(e, _) => {
                let _ = writeln!(self.out, "struct {name} {{ {}* ptr; size_t len; }};", c_type(e));
            }
            Type::Dyn(i) => {
                let _ = writeln!(self.out, "struct {name} {{ void *ptr; const struct ore_VT_{} *vt; }};", c_name(i));
            }
            Type::Struct(sname) => {
                let def = p.structs.iter().find(|s| s.name == *sname).unwrap();
                for (_, ft) in &def.fields {
                    self.define(ft, p, done);
                }
                let mut s = format!("struct {name} {{");
                for (fname, ft) in &def.fields {
                    let _ = write!(s, " {} {};", c_type(ft), c_field(fname));
                }
                s.push_str(" };");
                self.line(0, &s);
            }
            Type::Optional(e) => {
                self.define(e, p, done);
                let _ = writeln!(self.out, "struct {name} {{ bool has; {} val; }};", c_type(e));
            }
            // The error value (0: no error), then the payload.
            Type::ErrorUnion(_, e) if **e == Type::Void => {
                let _ = writeln!(self.out, "struct {name} {{ uint16_t err; }};");
            }
            Type::ErrorUnion(_, e) => {
                self.define(e, p, done);
                let _ = writeln!(self.out, "struct {name} {{ uint16_t err; {} val; }};", c_type(e));
            }
            // Tag, then a union with one struct per variant that has a payload.
            Type::Enum(ename) => {
                let def = p.enums.iter().find(|e| e.name == *ename).unwrap();
                for (_, ft) in def.variants.iter().flat_map(|v| &v.fields) {
                    self.define(ft, p, done);
                }
                let mut s = format!("struct {name} {{ uint32_t tag;");
                if def.variants.iter().any(|v| !v.fields.is_empty()) {
                    s.push_str(" union {");
                    for v in def.variants.iter().filter(|v| !v.fields.is_empty()) {
                        s.push_str(" struct {");
                        for (fname, ft) in &v.fields {
                            let _ = write!(s, " {} {};", c_type(ft), c_field(fname));
                        }
                        let _ = write!(s, " }} {};", c_field(&v.name));
                    }
                    s.push_str(" } u;");
                }
                s.push_str(" };");
                self.line(0, &s);
            }
            _ => unreachable!(),
        }
    }

    fn local_name(f: &Func, id: LocalId) -> String {
        match &f.locals[id].name {
            Some(n) if f.locals[id].global => global_name(n),
            Some(n) => format!("{n}_{id}"),
            None => format!("t{id}"),
        }
    }

    fn signature(&self, f: &Func) -> String {
        let params: Vec<String> =
            f.params.iter().map(|&id| format!("{} {}", c_type(&f.locals[id].ty), Self::local_name(f, id))).collect();
        let params = if params.is_empty() { "void".to_string() } else { params.join(", ") };
        if f.export {
            return format!("{} {}({params})", c_type(&f.ret), extern_symbol(&f.name));
        }
        format!("static {} {}({params})", c_type(&f.ret), fn_name(&f.name))
    }

    fn func(&mut self, f: &Func) {
        let sig = self.signature(f);
        let _ = writeln!(self.out, "\n{sig} {{");
        for (id, l) in f.locals.iter().enumerate() {
            if !f.params.contains(&id) && !l.global {
                let s = format!("{} {};", c_type(&l.ty), Self::local_name(f, id));
                self.line(1, &s);
            }
        }
        self.block(f, &f.body, 1);
        self.out.push_str("}\n");
    }

    fn block(&mut self, f: &Func, b: &Block, ind: usize) {
        for s in b {
            self.stmt(f, s, ind);
        }
    }

    fn stmt(&mut self, f: &Func, s: &Stmt, ind: usize) {
        let _ = writeln!(self.out, "#line {} {}", s.span.line, self.files[s.span.file as usize]);
        match &s.kind {
            StmtKind::Assign(place, rv) => {
                let line = format!("{} = {};", self.place(f, place), self.rvalue(f, rv));
                self.line(ind, &line);
            }
            StmtKind::Eval(rv) => {
                let line = format!("{};", self.rvalue(f, rv));
                self.line(ind, &line);
            }
            StmtKind::If { cond, then, els } => {
                let c = self.operand(f, cond);
                match (then.is_empty(), els.is_empty()) {
                    (false, _) => {
                        self.line(ind, &format!("if ({c}) {{"));
                        self.block(f, then, ind + 1);
                        if !els.is_empty() {
                            self.line(ind, "} else {");
                            self.block(f, els, ind + 1);
                        }
                    }
                    (true, false) => {
                        self.line(ind, &format!("if (!{c}) {{"));
                        self.block(f, els, ind + 1);
                    }
                    (true, true) => self.line(ind, "{"),
                }
                self.line(ind, "}");
            }
            StmtKind::Loop { body, cont } => {
                let label = if cont.is_empty() {
                    None
                } else {
                    self.next_label += 1;
                    Some(format!("ore_continue_{}", self.next_label))
                };
                self.line(ind, "for (;;) {");
                self.loops.push(label.clone());
                self.block(f, body, ind + 1);
                self.loops.pop();
                if let Some(l) = label {
                    self.line(ind, &format!("{l}:;"));
                    self.block(f, cont, ind + 1);
                }
                self.line(ind, "}");
            }
            StmtKind::Break => self.line(ind, "break;"),
            StmtKind::Continue => match self.loops.last().cloned().flatten() {
                Some(l) => self.line(ind, &format!("goto {l};")),
                None => self.line(ind, "continue;"),
            },
            StmtKind::Return(None) => self.line(ind, "return;"),
            StmtKind::Return(Some(op)) => {
                let line = format!("return {};", self.operand(f, op));
                self.line(ind, &line);
            }
            StmtKind::TracePush(span) if self.trace => {
                self.line(ind, &format!("ore_trace_push({});", line_col(*span)));
            }
            StmtKind::TraceRestore(t) if self.trace => {
                self.line(ind, &format!("ore_trace_len = {};", Self::local_name(f, *t)));
            }
            StmtKind::TracePush(_) | StmtKind::TraceRestore(_) => {}
        }
    }

    fn operand(&self, f: &Func, op: &Operand) -> String {
        match op {
            Operand::Local(id) => Self::local_name(f, *id),
            Operand::Int(v, it) => c_int_of(*v, *it),
            Operand::Float(v, ft) => c_float_of(*v, *ft),
            Operand::Bool(b) => b.to_string(),
            Operand::Str(s) => format!("((ore_S_u8){{(uint8_t*){}, {}}})", c_str(s), s.len()),
            Operand::Fn { name, is_extern: true } => extern_symbol(name).to_string(),
            Operand::Fn { name, is_extern: false } => fn_name(name),
        }
    }

    fn place(&self, f: &Func, p: &Place) -> String {
        let mut s = Self::local_name(f, p.local);
        for proj in &p.proj {
            match proj {
                Proj::Field(name) => {
                    s.push('.');
                    s.push_str(&c_field(name));
                }
                Proj::Index { index, len: Len::Array(n), span } => {
                    s = format!("{s}.a[ore_idx({}, {n}, {})]", self.operand(f, index), line_col(*span));
                }
                Proj::Index { index, len: Len::Slice, span } => {
                    s = format!("{s}.ptr[ore_idx({}, {s}.len, {})]", self.operand(f, index), line_col(*span));
                }
                Proj::Index { index, len: Len::Unchecked, .. } => s = format!("{s}[{}]", self.operand(f, index)),
                Proj::Deref => s = format!("(*{s})"),
            }
        }
        s
    }

    fn rvalue(&self, f: &Func, rv: &Rvalue) -> String {
        match rv {
            Rvalue::Use(op) => self.operand(f, op),
            Rvalue::Load(p) => self.place(f, p),
            Rvalue::Binary { op, lhs, rhs, ty, span } => {
                let (a, b) = (self.operand(f, lhs), self.operand(f, rhs));
                let helper = match op {
                    BinOp::Add => "add",
                    BinOp::Sub => "sub",
                    BinOp::Mul => "mul",
                    BinOp::Div => "div",
                    BinOp::Rem => "rem",
                    BinOp::AddWrap => "wadd",
                    BinOp::SubWrap => "wsub",
                    BinOp::MulWrap => "wmul",
                    BinOp::Shl => "shl",
                    BinOp::Shr => "shr",
                    BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor => {
                        return format!("(({})({a} {} {b}))", c_type(ty), op.as_str());
                    }
                    // Enums without payloads compare by tag.
                    _ if matches!(ty, Type::Enum(_)) => return format!("({a}.tag {} {b}.tag)", op.as_str()),
                    _ => return format!("({a} {} {b})", op.as_str()),
                };
                match ty {
                    Type::Int(it) if helper.starts_with('w') => format!("ore_{helper}_{}({a}, {b})", it.name()),
                    Type::Int(it) => format!("ore_{helper}_{}({a}, {b}, {})", it.name(), line_col(*span)),
                    _ => format!("({a} {} {b})", op.as_str()),
                }
            }
            Rvalue::Neg { val, ty, span } => {
                let v = self.operand(f, val);
                match ty {
                    Type::Int(it) => format!("ore_neg_{}({v}, {})", it.name(), line_col(*span)),
                    _ => format!("(-{v})"),
                }
            }
            Rvalue::Not(op) => format!("(!{})", self.operand(f, op)),
            Rvalue::BitNot { val, ty } => format!("(({})~{})", c_type(ty), self.operand(f, val)),
            Rvalue::InRanges { val, ranges, ty } => {
                let v = self.operand(f, val);
                let tests: Vec<String> = ranges
                    .iter()
                    .map(|&(lo, hi)| {
                        let (l, h) = (c_int_of(lo, *ty), c_int_of(hi, *ty));
                        match (lo == hi, lo == ty.min(), hi == ty.max()) {
                            (true, ..) => format!("{v} == {l}"),
                            (_, true, true) => "1".to_string(),
                            (_, true, _) => format!("{v} <= {h}"),
                            (_, _, true) => format!("{v} >= {l}"),
                            _ => format!("({v} >= {l} && {v} <= {h})"),
                        }
                    })
                    .collect();
                format!("({})", tests.join(" || "))
            }
            Rvalue::Cast { val, from, to, span } => self.cast(&self.operand(f, val), from, to, *span),
            Rvalue::Call { name, is_extern, args } => {
                let args: Vec<String> = args.iter().map(|a| self.operand(f, a)).collect();
                let name = if *is_extern { extern_symbol(name).to_string() } else { fn_name(name) };
                format!("{name}({})", args.join(", "))
            }
            Rvalue::CallValue { callee, args } => {
                let args: Vec<String> = args.iter().map(|a| self.operand(f, a)).collect();
                format!("{}({})", self.operand(f, callee), args.join(", "))
            }
            Rvalue::SizeOf(t) => format!("((size_t)sizeof({}))", c_type(t)),
            // A struct is zeroed by bytes, so its padding is zero too.
            Rvalue::Zeroed(t) if is_composite(t) => {
                format!("({{ {} z_; __builtin_memset(&z_, 0, sizeof z_); z_; }})", c_type(t))
            }
            Rvalue::Zeroed(t) => format!("(({})0)", c_type(t)),
            Rvalue::PtrCast { val, ty } => format!("(({}){})", c_type(ty), self.operand(f, val)),
            Rvalue::DynCall { recv, method, args } => {
                let r = self.operand(f, recv);
                let args: String = args.iter().map(|a| format!(", {}", self.operand(f, a))).collect();
                format!("{r}.vt->{}({r}.ptr{args})", c_field(method))
            }
            Rvalue::ToDyn { ptr, iface, strukt } => {
                format!("((ore_D_{}){{(void *){}, &{}}})", c_name(iface), self.operand(f, ptr), vtable_name(iface, strukt))
            }
            Rvalue::Alloc { alloc, ty, span } => {
                let t = c_type(ty);
                format!("(({t} *)ore_alloc({}, sizeof({t}), _Alignof({t}), {}))", self.operand(f, alloc), line_col(*span))
            }
            // A statement expression: allocate, then fill every element.
            Rvalue::AllocSlice { alloc, value, elem, len, span } => {
                let (t, v) = (c_type(elem), self.operand(f, value));
                let opt = c_type(&Type::Optional(Box::new(Type::Slice(Box::new(elem.clone()), true))));
                format!(
                    "({{ {opt} r_ = {{0}}; size_t n_ = (size_t){}; {t} *p_ = n_ ? ({t} *)ore_alloc_n({}, n_, sizeof({t}), _Alignof({t}), {}) : 0; \
                     if (p_ || !n_) {{ for (size_t i_ = 0; i_ < n_; i_++) p_[i_] = {v}; r_.has = 1; r_.val.ptr = p_; r_.val.len = n_; }} r_; }})",
                    self.operand(f, len),
                    self.operand(f, alloc),
                    line_col(*span)
                )
            }
            Rvalue::EmptySlice(ty) => format!("(({}){{0, 0}})", c_type(ty)),
            Rvalue::Intrinsic { op, args, span } => self.intrinsic(f, *op, args, *span),
            Rvalue::Print { arg: None, .. } => "ore_rt_printf(\"\\n\")".to_string(),
            Rvalue::Print { arg: Some((Operand::Str(s), _)), newline } => {
                format!("ore_print_s((const uint8_t*){}, {}, {newline})", c_str(s), s.len())
            }
            Rvalue::Print { arg: Some((op, ty)), newline } => {
                let v = self.operand(f, op);
                match ty {
                    Type::Int(it) if it.signed() => format!("ore_print_i({v}, {newline})"),
                    Type::Int(_) => format!("ore_print_u({v}, {newline})"),
                    Type::Float(_) => format!("ore_print_f({v}, {newline})"),
                    Type::Bool => format!("ore_print_b({v}, {newline})"),
                    _ => format!("ore_print_s(({v}).ptr, ({v}).len, {newline})"),
                }
            }
            // A statement expression: flush stdout, then write the prefix,
            // each piece and argument, and a newline to stderr.
            Rvalue::Log { error, pieces, args } => {
                let mut s = format!("({{ ore_rt_fflush(0); ore_rt_dprintf(2, \"{}: \");", if *error { "error" } else { "info" });
                for (i, piece) in pieces.iter().enumerate() {
                    if !piece.is_empty() {
                        let _ = write!(s, " ore_log_s((const uint8_t*){}, {});", c_str(piece), piece.len());
                    }
                    let Some((op, ty)) = args.get(i) else { continue };
                    let v = self.operand(f, op);
                    let _ = match ty {
                        Type::Int(it) if it.signed() => write!(s, " ore_log_i({v});"),
                        Type::Int(_) => write!(s, " ore_log_u({v});"),
                        Type::Float(_) => write!(s, " ore_log_f({v});"),
                        Type::Bool => write!(s, " ore_log_b({v});"),
                        Type::ErrorSet(_) => write!(s, " ore_log_e({v});"),
                        _ => write!(s, " ore_log_s(({v}).ptr, ({v}).len);"),
                    };
                }
                s.push_str(" ore_rt_dprintf(2, \"\\n\"); })");
                s
            }
            Rvalue::StructLit { name, fields } => {
                let fs: Vec<String> = fields.iter().map(|o| self.operand(f, o)).collect();
                format!("(({}){{{}}})", c_type(&Type::Struct(name.clone())), fs.join(", "))
            }
            Rvalue::Variant { enum_name, tag, variant, fields } => {
                let t = c_type(&Type::Enum(enum_name.clone()));
                if fields.is_empty() {
                    return format!("(({t}){{.tag = {tag}}})");
                }
                let fs: Vec<String> = fields.iter().map(|o| self.operand(f, o)).collect();
                format!("(({t}){{.tag = {tag}, .u.{} = {{{}}}}})", c_field(variant), fs.join(", "))
            }
            Rvalue::AddrOf(p) => format!("(&{})", self.place(f, p)),
            Rvalue::Some { val, ty } => {
                let v = self.operand(f, val);
                if is_ptr_optional(ty) { v } else { format!("(({}){{true, {v}}})", c_type(ty)) }
            }
            Rvalue::Null(ty) => {
                if is_ptr_optional(ty) {
                    format!("(({})0)", c_type(ty))
                } else {
                    format!("(({}){{0}})", c_type(ty))
                }
            }
            Rvalue::IsSome { place, ty } => {
                let p = self.place(f, place);
                if is_ptr_optional(ty) { format!("({p} != 0)") } else { format!("{p}.has") }
            }
            Rvalue::ArrayLit { ty, elems } => {
                let es: Vec<String> = elems.iter().map(|o| self.operand(f, o)).collect();
                format!("(({}){{{{{}}}}})", c_type(ty), es.join(", "))
            }
            Rvalue::IsErr(p) => format!("({}.err != 0)", self.place(f, p)),
            Rvalue::Ok { val: None, ty } => format!("(({}){{0}})", c_type(ty)),
            Rvalue::Ok { val: Some(v), ty } => format!("(({}){{0, {}}})", c_type(ty), self.operand(f, v)),
            Rvalue::Err { err, ty } => format!("(({}){{.err = {}}})", c_type(ty), self.operand(f, err)),
            Rvalue::TraceLen => if self.trace { "ore_trace_len" } else { "0" }.to_string(),
            Rvalue::Slice { base, len, elem, lo, hi, span } => {
                let b = self.place(f, base);
                let (ptr, n) = match len {
                    Len::Array(n) => (format!("{b}.a"), n.to_string()),
                    Len::Slice => (format!("{b}.ptr"), format!("{b}.len")),
                    // No length: only `lo <= hi` is checked.
                    Len::Unchecked => (b.clone(), self.operand(f, hi.as_ref().unwrap())),
                };
                let lo = self.operand(f, lo);
                let hi = hi.as_ref().map(|h| self.operand(f, h)).unwrap_or_else(|| n.clone());
                format!(
                    "(({}){{{ptr} + ore_slice({lo}, {hi}, {n}, {}), (size_t){hi} - (size_t){lo}}})",
                    c_type(&Type::Slice(Box::new(elem.clone()), false)),
                    line_col(*span)
                )
            }
        }
    }

    fn intrinsic(&self, f: &Func, op: Intrinsic, args: &[(Operand, Type)], span: Span) -> String {
        let a: Vec<String> = args.iter().map(|(o, _)| self.operand(f, o)).collect();
        let lc = line_col(span);
        match op {
            Intrinsic::GeneralInit => "ore_general_init()".into(),
            Intrinsic::GeneralAllocator => format!("ore_general_allocator({})", a[0]),
            Intrinsic::GeneralDeinit => format!("ore_general_deinit({})", a[0]),
            Intrinsic::ArenaInit => format!("ore_arena_init({})", a[0]),
            Intrinsic::ArenaAllocator => format!("ore_arena_allocator({})", a[0]),
            Intrinsic::ArenaDeinit => format!("ore_arena_deinit({}, {lc})", a[0]),
            Intrinsic::Destroy => format!("ore_free({}, (void *){}, {lc})", a[0], a[1]),
            Intrinsic::Free => format!("ore_free_n({0}, (void *)({1}).ptr, ({1}).len, {lc})", a[0], a[1]),
            Intrinsic::TestAllocator => "ore_test_allocator()".into(),
            Intrinsic::Expect => format!("ore_expect({}, {lc})", a[0]),
            Intrinsic::ExpectEqual => match &args[0].1 {
                Type::Int(it) if it.signed() => format!("ore_expect_eq_i({}, {}, {lc})", a[0], a[1]),
                Type::Int(_) => format!("ore_expect_eq_u({}, {}, {lc})", a[0], a[1]),
                Type::Float(_) => format!("ore_expect_eq_f({}, {}, {lc})", a[0], a[1]),
                Type::Bool => format!("ore_expect_eq_b({}, {}, {lc})", a[0], a[1]),
                _ => format!("ore_expect_eq_s(({0}).ptr, ({0}).len, ({1}).ptr, ({1}).len, {lc})", a[0], a[1]),
            },
        }
    }

    fn cast(&self, v: &str, from: &Type, to: &Type, span: Span) -> String {
        let to_c = c_type(to);
        let lc = line_col(span);
        match (from, to) {
            (Type::Int(fi), Type::Int(ti)) => {
                if fi.min() >= ti.min() && fi.max() <= ti.max() {
                    format!("(({to_c}){v})")
                } else if fi.signed() && ti.signed() {
                    format!("(({to_c})ore_cast_ss((int64_t){v}, {}, {}, {lc}))", c_int(ti.min()), c_int(ti.max()))
                } else if fi.signed() {
                    format!("(({to_c})ore_cast_su((int64_t){v}, {}, {lc}))", c_int(ti.max()))
                } else {
                    format!("(({to_c})ore_cast_u((uint64_t){v}, {}, {lc}))", c_int(ti.max()))
                }
            }
            (Type::Float(_), Type::Int(ti)) => {
                let lo = if ti.signed() { ti.min() as f64 - 1.0 } else { -1.0 };
                let hi = ti.max() as f64 + 1.0;
                format!("(({to_c})ore_cast_fi((double){v}, {}, {}, {lc}))", c_float(lo), c_float(hi))
            }
            _ => format!("(({to_c}){v})"),
        }
    }
}

/// The C initializer of a global of type `ty` with the value `v`.
fn const_init(p: &Program, v: &ConstVal, ty: &Type) -> String {
    let list = |vals: &[ConstVal], tys: &[&Type]| -> String {
        vals.iter().zip(tys).map(|(v, t)| const_init(p, v, t)).collect::<Vec<_>>().join(", ")
    };
    match (v, ty) {
        (ConstVal::Int(n), Type::Int(it)) => c_int_of(*n, *it),
        (ConstVal::Float(x), Type::Float(ft)) => c_float_of(*x, *ft),
        (ConstVal::Bool(b), _) => b.to_string(),
        (ConstVal::Str(s), _) => format!("{{(uint8_t*){}, {}}}", c_str(s), s.len()),
        (ConstVal::EmptySlice, _) => "{0, 0}".to_string(),
        (ConstVal::Error(e), _) => e.to_string(),
        (ConstVal::Fn(name, true), _) => extern_symbol(name).to_string(),
        (ConstVal::Fn(name, false), _) => fn_name(name),
        (ConstVal::Null, t) if is_ptr_optional(t) => "0".to_string(),
        (ConstVal::Null, _) => "{0}".to_string(),
        (ConstVal::Some(x), Type::Optional(inner)) => format!("{{true, {}}}", const_init(p, x, inner)),
        (ConstVal::Agg(elems), Type::Array(el, _)) => format!("{{{{{}}}}}", list(elems, &vec![&**el; elems.len()])),
        (ConstVal::Agg(fields), Type::Struct(name)) => {
            let def = p.structs.iter().find(|s| s.name == *name).unwrap();
            format!("{{{}}}", list(fields, &def.fields.iter().map(|(_, t)| t).collect::<Vec<_>>()))
        }
        (ConstVal::Variant(tag, fields), Type::Enum(name)) => {
            let v = &p.enums.iter().find(|e| e.name == *name).unwrap().variants[*tag];
            if fields.is_empty() {
                return format!("{{.tag = {tag}}}");
            }
            let tys: Vec<&Type> = v.fields.iter().map(|(_, t)| t).collect();
            format!("{{.tag = {tag}, .u.{} = {{{}}}}}", c_field(&v.name), list(fields, &tys))
        }
        _ => unreachable!("constant {v:?} of type {ty}"),
    }
}

/// The types of `@sizeOf` in `b`: no value of them may exist.
fn sizeof_types(b: &Block, out: &mut Vec<Type>) {
    for s in b {
        match &s.kind {
            StmtKind::Assign(_, Rvalue::SizeOf(t)) | StmtKind::Eval(Rvalue::SizeOf(t)) => out.push(t.clone()),
            StmtKind::If { then: a, els: b, .. } | StmtKind::Loop { body: a, cont: b } => {
                sizeof_types(a, out);
                sizeof_types(b, out);
            }
            _ => {}
        }
    }
}

fn collect_type(t: &Type, seen: &mut HashSet<Type>, all: &mut Vec<Type>) {
    if !seen.insert(t.clone()) {
        return;
    }
    match t {
        Type::Array(e, _)
        | Type::Slice(e, _)
        | Type::CPtr(e)
        | Type::Ptr(e, _)
        | Type::Optional(e)
        | Type::ErrorUnion(_, e) => collect_type(e, seen, all),
        Type::Fn(params, ret) => {
            for t in params.iter().chain([&**ret]) {
                collect_type(t, seen, all);
            }
        }
        _ => {}
    }
    all.push(t.clone());
}
