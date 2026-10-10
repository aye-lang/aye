//! C backend: turns OIR into a single C translation unit for clang.
//!
//! The generated C includes only freestanding headers (stdint/stddef/stdbool)
//! so user `extern fn` declarations never clash with libc headers. The
//! runtime reaches libc through asm-labelled aliases (`aye_rt_printf` ->
//! `printf`). An `extern fn` is declared from its Aye signature, and an
//! `extern struct` is a C struct with its fields in declaration order (as
//! every struct is), so the C layout is what the Aye declaration says; it
//! must match the library's. A function type is a `typedef` of a C function
//! pointer, and a `pub extern fn` is defined under its own name, not
//! `static`. Overflow, bounds, shift, division and cast checks are emitted
//! inline, as an `if` before the statement that needs them (OIR operands are
//! locals and constants, so a check can read them again), and overflow of
//! `+ - *` as `__builtin_add_overflow` into the destination: no function call
//! at `-O0`, and the `#line` of the statement. `fast` mode and
//! `@setSafety(false)` blocks leave them out (`AYE_CHECKS` is 0 in `fast`
//! mode, for the runtime's own checks).
//! `AYE_TRACE` (debug mode) keeps the error return trace that is printed when
//! `main` returns an error, and turns on the leak, double-free and
//! use-after-free checks of `General`.
//!
//! In test mode the C `main` runs the test whose index is its argument.
//!
//! A source position reaches the runtime as two `int`s: the line, with the
//! file's index in its upper bits (`AYE_FILE`, `AYE_LINE`), and the column.

use crate::ast::BinOp;
use crate::diag::Span;
use crate::oir::*;
use crate::sema::{ConstVal, FloatTy, IntTy, Intrinsic, Type};
use std::collections::HashSet;
use std::fmt::Write;

const PRELUDE: &str = r#"#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#define AYE_RT static __attribute__((unused))
#define AYE_STR_(x) #x
#define AYE_STR(x) AYE_STR_(x)
#define AYE_SYM(n) AYE_STR(__USER_LABEL_PREFIX__) n
extern int aye_rt_printf(const char *, ...) __asm__(AYE_SYM("printf"));
extern int aye_rt_dprintf(int, const char *, ...) __asm__(AYE_SYM("dprintf"));
extern int aye_rt_fflush(void *) __asm__(AYE_SYM("fflush"));
extern void aye_rt_abort(void) __asm__(AYE_SYM("abort")) __attribute__((noreturn));
extern void aye_rt_exit(int) __asm__(AYE_SYM("exit")) __attribute__((noreturn));
extern void *aye_rt_malloc(size_t) __asm__(AYE_SYM("malloc"));
extern void aye_rt_free(void *) __asm__(AYE_SYM("free"));
extern int aye_rt_memcmp(const void *, const void *, size_t) __asm__(AYE_SYM("memcmp"));
extern int aye_rt_open(const char *, int, ...) __asm__(AYE_SYM("open"));
extern int aye_rt_access(const char *, int) __asm__(AYE_SYM("access"));
extern int aye_rt_clock_gettime(int, int64_t *) __asm__(AYE_SYM("clock_gettime"));
#if defined(__APPLE__) && defined(__x86_64__)
extern int aye_rt_stat(const char *, void *) __asm__(AYE_SYM("stat$INODE64"));
#else
extern int aye_rt_stat(const char *, void *) __asm__(AYE_SYM("stat"));
#endif
#if defined(__APPLE__) && defined(__x86_64__)
#define AYE_INODE64 "$INODE64"
#else
#define AYE_INODE64 ""
#endif
extern int aye_rt_lstat(const char *, void *) __asm__(AYE_SYM("lstat" AYE_INODE64));
extern int aye_rt_mkdir(const char *, unsigned) __asm__(AYE_SYM("mkdir"));
extern int aye_rt_remove(const char *) __asm__(AYE_SYM("remove"));
extern int aye_rt_rename(const char *, const char *) __asm__(AYE_SYM("rename"));
extern uint8_t *aye_rt_opendir(const char *) __asm__(AYE_SYM("opendir" AYE_INODE64));
extern uint8_t *aye_rt_readdir(uint8_t *) __asm__(AYE_SYM("readdir" AYE_INODE64));
extern int aye_rt_closedir(uint8_t *) __asm__(AYE_SYM("closedir"));
extern void aye_rt_rewinddir(uint8_t *) __asm__(AYE_SYM("rewinddir"));
#define AYE_FILE(l) aye_files[(l) >> 20]
#define AYE_LINE(l) ((l) & 0xfffff)

AYE_RT __attribute__((noreturn, cold)) void aye_panic(int line, int col, const char *msg) {
    aye_rt_fflush(0);
    aye_rt_dprintf(2, "%s:%d:%d: panic: %s\n", AYE_FILE(line), AYE_LINE(line), col, msg);
    aye_rt_abort();
}
AYE_RT __attribute__((noreturn, cold)) void aye_panic_index(int line, int col, size_t i, size_t len) {
    aye_rt_fflush(0);
    aye_rt_dprintf(2, "%s:%d:%d: panic: index out of bounds: index %llu, len %llu\n", AYE_FILE(line), AYE_LINE(line), col,
                   (unsigned long long)i, (unsigned long long)len);
    aye_rt_abort();
}
AYE_RT __attribute__((noreturn, cold)) void aye_panic_slice(int line, int col, size_t lo, size_t hi, size_t len) {
    aye_rt_fflush(0);
    aye_rt_dprintf(2, "%s:%d:%d: panic: slice bounds out of range: [%llu..%llu] with len %llu\n", AYE_FILE(line),
                   AYE_LINE(line), col, (unsigned long long)lo, (unsigned long long)hi, (unsigned long long)len);
    aye_rt_abort();
}
#define AYE_UNLIKELY(c) __builtin_expect(!!(c), 0)

/* `@hash`: each scalar is mixed in with SplitMix64's finalizer. A float
   hashes its bits, with -0.0 as 0.0 (they are `==`). */
AYE_RT uint64_t aye_hash_mix(uint64_t h, uint64_t v) {
    uint64_t x = h ^ (v + 0x9e3779b97f4a7c15ULL + (h << 6) + (h >> 2));
    x = (x ^ (x >> 30)) * 0xbf58476d1ce4e5b9ULL;
    x = (x ^ (x >> 27)) * 0x94d049bb133111ebULL;
    return x ^ (x >> 31);
}
AYE_RT uint64_t aye_hash_f64(double d) {
    uint64_t b;
    if (d == 0) d = 0;
    __builtin_memcpy(&b, &d, sizeof b);
    return b;
}

#define AYE_TRACE_MAX 64
/* Each thread has its own error return trace, and so does each coroutine
   (`aye_trace_buf` is the running coroutine's; null for the thread's). */
typedef struct aye_trace_ent { int line, col; } aye_trace_ent;
static _Thread_local __attribute__((unused)) aye_trace_ent aye_trace[AYE_TRACE_MAX];
static _Thread_local __attribute__((unused)) aye_trace_ent *aye_trace_buf;
static _Thread_local __attribute__((unused)) uint32_t aye_trace_len;
AYE_RT void aye_trace_push(int line, int col) {
    if (aye_trace_len < AYE_TRACE_MAX) {
        aye_trace_ent *t = aye_trace_buf ? aye_trace_buf : aye_trace;
        t[aye_trace_len].line = line;
        t[aye_trace_len].col = col;
    }
    aye_trace_len++;
}
/* `main` returned the error `e`: report it (with the trace in debug mode). */
AYE_RT int aye_main_error(uint16_t e) {
    aye_rt_fflush(0);
    aye_rt_dprintf(2, "error: %s\n", aye_error_names[e]);
    if (AYE_TRACE) {
        aye_rt_dprintf(2, "error return trace:\n");
        for (uint32_t i = 0; i < aye_trace_len && i < AYE_TRACE_MAX; i++) {
            aye_rt_dprintf(2, "%s:%d:%d\n", AYE_FILE(aye_trace[i].line), AYE_LINE(aye_trace[i].line), aye_trace[i].col);
        }
        if (aye_trace_len > AYE_TRACE_MAX) aye_rt_dprintf(2, "(%u more)\n", aye_trace_len - AYE_TRACE_MAX);
    }
    return 1;
}

/* Allocator: a data pointer and a vtable of byte-level functions. `line` and
   `col` are the Aye call that allocates or frees; leak and double-free
   reports name it. `free` gets the size that was allocated. Alignment is
   at most 16, which malloc guarantees. */
typedef struct aye_alloc_vt {
    void *(*alloc)(void *self, size_t size, size_t align, int line, int col);
    void (*free)(void *self, void *p, size_t size, int line, int col);
} aye_alloc_vt;
typedef struct aye_Allocator { void *ptr; const aye_alloc_vt *vt; } aye_Allocator;
AYE_RT void *aye_alloc(aye_Allocator a, size_t size, size_t align, int line, int col) {
    return a.vt->alloc(a.ptr, size, align, line, col);
}
AYE_RT void aye_free(aye_Allocator a, void *p, size_t size, int line, int col) { a.vt->free(a.ptr, p, size, line, col); }
/* `n` elements of `size` bytes; null when the size overflows. */
AYE_RT void *aye_alloc_n(aye_Allocator a, size_t n, size_t size, size_t align, int line, int col) {
    if (size && n > SIZE_MAX / size) return 0;
    return aye_alloc(a, n * size, align, line, col);
}
/* Free a slice of `n` elements of `size` bytes; an empty one was not
   allocated. */
AYE_RT void aye_free_n(aye_Allocator a, void *p, size_t n, size_t size, int line, int col) {
    if (n) aye_free(a, p, n * size, line, col);
}

/* General: libc malloc/free. With `track` (debug mode, and the test
   allocator) each block has a header before it: its allocator, size,
   state, and where it was allocated and freed. Live blocks are in a
   doubly linked list, oldest first, for the leak report. A freed block is
   filled with AYE_POISON and kept in a FIFO quarantine of at most
   AYE_QUARANTINE bytes before libc gets it back, so its address is not
   reused soon: a second free of it panics, and when it leaves the
   quarantine (or at `deinit`) a byte that is no longer AYE_POISON means it
   was written after it was freed. Allocation and free take constant time. */
#define AYE_LIVE 0x4f52454c49564521ull
#define AYE_FREED 0x4f52454652454544ull
#define AYE_POISON 0xdd
#define AYE_QUARANTINE ((size_t)32 << 20)
typedef struct __attribute__((aligned(16))) aye_alloc_hdr {
    struct aye_alloc_hdr *prev, *next; /* the live list, or (next) the quarantine */
    void *owner;
    size_t size;
    int line, col, free_line, free_col;
    uint64_t state; /* AYE_LIVE or AYE_FREED */
} aye_alloc_hdr;
typedef struct aye_General {
    aye_alloc_hdr *head, *tail;   /* live blocks, oldest first */
    aye_alloc_hdr *qhead, *qtail; /* freed blocks, oldest first */
    size_t qbytes;
    bool track;
} aye_General;
AYE_RT void *aye_general_alloc(void *self, size_t size, size_t align, int line, int col) {
    aye_General *g = self;
    (void)align;
    if (!g->track) return aye_rt_malloc(size ? size : 1);
    if (size > SIZE_MAX - sizeof(aye_alloc_hdr)) return 0;
    aye_alloc_hdr *h = aye_rt_malloc(sizeof *h + size);
    if (!h) return 0;
    *h = (aye_alloc_hdr){g->tail, 0, g, size, line, col, 0, 0, AYE_LIVE};
    if (g->tail) g->tail->next = h; else g->head = h;
    g->tail = h;
    return h + 1;
}
/* Give a quarantined block back to libc, after checking that nothing wrote
   to it since it was freed. */
AYE_RT void aye_general_release(aye_alloc_hdr *h) {
    const unsigned char *b = (const unsigned char *)(h + 1);
    size_t i = 0;
    for (; i + 8 <= h->size; i += 8) {
        uint64_t w;
        __builtin_memcpy(&w, b + i, 8);
        if (w != 0xddddddddddddddddull) break;
    }
    for (; i < h->size; i++) {
        if (b[i] != AYE_POISON) {
            aye_rt_fflush(0);
            aye_rt_dprintf(2, "%s:%d:%d: panic: use after free: memory allocated at %s:%d:%d and freed here was written after it was freed\n",
                           AYE_FILE(h->free_line), AYE_LINE(h->free_line), h->free_col, AYE_FILE(h->line), AYE_LINE(h->line), h->col);
            aye_rt_abort();
        }
    }
    h->state = 0;
    aye_rt_free(h);
}
AYE_RT void aye_general_free(void *self, void *p, size_t size, int line, int col) {
    aye_General *g = self;
    (void)size;
    if (!g->track) {
        aye_rt_free(p);
        return;
    }
    aye_alloc_hdr *h = (aye_alloc_hdr *)p - 1;
    if ((uintptr_t)p % 16 || h->owner != g || (h->state != AYE_LIVE && h->state != AYE_FREED)) {
        aye_panic(line, col, "free of memory that this allocator did not allocate");
    }
    if (h->state == AYE_FREED) {
        aye_rt_fflush(0);
        aye_rt_dprintf(2, "%s:%d:%d: panic: double free of memory allocated at %s:%d:%d and already freed at %s:%d:%d\n",
                       AYE_FILE(line), AYE_LINE(line), col, AYE_FILE(h->line), AYE_LINE(h->line), h->col,
                       AYE_FILE(h->free_line), AYE_LINE(h->free_line), h->free_col);
        aye_rt_abort();
    }
    if (h->prev) h->prev->next = h->next; else g->head = h->next;
    if (h->next) h->next->prev = h->prev; else g->tail = h->prev;
    h->state = AYE_FREED;
    h->free_line = line;
    h->free_col = col;
    __builtin_memset(h + 1, AYE_POISON, h->size);
    h->prev = h->next = 0;
    if (g->qtail) g->qtail->next = h; else g->qhead = h;
    g->qtail = h;
    g->qbytes += sizeof *h + h->size;
    while (g->qbytes > AYE_QUARANTINE) {
        aye_alloc_hdr *old = g->qhead;
        g->qhead = old->next;
        if (!g->qhead) g->qtail = 0;
        g->qbytes -= sizeof *old + old->size;
        aye_general_release(old);
    }
}
static __attribute__((unused)) const aye_alloc_vt aye_general_vt = {aye_general_alloc, aye_general_free};
AYE_RT aye_General aye_general_init(void) { return (aye_General){.track = AYE_TRACE}; }
AYE_RT aye_Allocator aye_general_allocator(aye_General *g) { return (aye_Allocator){g, &aye_general_vt}; }
/* Report each allocation that was not freed, in allocation order, then
   free all memory (checking the quarantined blocks). Returns the number of
   leaks. */
AYE_RT size_t aye_general_deinit(aye_General *g) {
    size_t leaks = 0;
    for (aye_alloc_hdr *h = g->head, *next; h; h = next) {
        next = h->next;
        if (!leaks) aye_rt_fflush(0);
        aye_rt_dprintf(2, "%s:%d:%d: leak: %llu bytes allocated here were never freed\n", AYE_FILE(h->line),
                       AYE_LINE(h->line), h->col, (unsigned long long)h->size);
        leaks++;
        h->state = 0;
        aye_rt_free(h);
    }
    for (aye_alloc_hdr *h = g->qhead, *next; h; h = next) {
        next = h->next;
        aye_general_release(h);
    }
    *g = (aye_General){.track = g->track};
    return leaks;
}

/* Arena: bump allocation in chunks from the backing allocator. `free` does
   nothing; `deinit` frees every chunk. */
typedef struct aye_arena_chunk { struct aye_arena_chunk *next; size_t size; } aye_arena_chunk;
typedef struct aye_Arena { aye_Allocator backing; aye_arena_chunk *chunks; size_t used; } aye_Arena;
#define AYE_ARENA_CHUNK 4096
AYE_RT void *aye_arena_alloc(void *self, size_t size, size_t align, int line, int col) {
    aye_Arena *a = self;
    aye_arena_chunk *c = a->chunks;
    if (c) {
        size_t off = (a->used + align - 1) & ~(align - 1);
        if (off <= c->size && size <= c->size - off) {
            a->used = off + size;
            return (char *)(c + 1) + off;
        }
    }
    size_t cap = size > AYE_ARENA_CHUNK ? size : AYE_ARENA_CHUNK;
    c = aye_alloc(a->backing, sizeof(aye_arena_chunk) + cap, 16, line, col);
    if (!c) return 0;
    c->next = a->chunks;
    c->size = cap;
    a->chunks = c;
    a->used = size;
    return c + 1;
}
AYE_RT void aye_arena_free(void *self, void *p, size_t size, int line, int col) {}
static __attribute__((unused)) const aye_alloc_vt aye_arena_vt = {aye_arena_alloc, aye_arena_free};
AYE_RT aye_Arena aye_arena_init(aye_Allocator backing) { return (aye_Arena){backing, 0, 0}; }
AYE_RT aye_Allocator aye_arena_allocator(aye_Arena *a) { return (aye_Allocator){a, &aye_arena_vt}; }
AYE_RT void aye_arena_deinit(aye_Arena *a, int line, int col) {
    for (aye_arena_chunk *c = a->chunks, *next; c; c = next) {
        next = c->next;
        aye_free(a->backing, c, sizeof(aye_arena_chunk) + c->size, line, col);
    }
    a->chunks = 0;
    a->used = 0;
}

/* Tests: the leak-checked allocator, assertions, and the end of a test. A
   failed assertion ends the test process with status 1. */
static __attribute__((unused)) aye_General aye_test_gpa = {.track = true};
AYE_RT aye_Allocator aye_test_allocator(void) { return aye_general_allocator(&aye_test_gpa); }
#define AYE_ASSERT_FAILED(line, col, fmt, ...)                                                                  \
    do {                                                                                                         \
        aye_rt_fflush(0);                                                                                        \
        aye_rt_dprintf(2, "%s:%d:%d: assertion failed: " fmt "\n", AYE_FILE(line), AYE_LINE(line), col, __VA_ARGS__); \
        aye_rt_exit(1);                                                                                          \
    } while (0)
AYE_RT void aye_expect(bool c, int line, int col) {
    if (!c) AYE_ASSERT_FAILED(line, col, "%s", "condition is false");
}
AYE_RT void aye_expect_eq_i(int64_t want, int64_t got, int line, int col) {
    if (want != got) AYE_ASSERT_FAILED(line, col, "expected %lld, found %lld", (long long)want, (long long)got);
}
AYE_RT void aye_expect_eq_u(uint64_t want, uint64_t got, int line, int col) {
    if (want != got)
        AYE_ASSERT_FAILED(line, col, "expected %llu, found %llu", (unsigned long long)want, (unsigned long long)got);
}
AYE_RT void aye_expect_eq_f(double want, double got, int line, int col) {
    if (want != got) AYE_ASSERT_FAILED(line, col, "expected %g, found %g", want, got);
}
AYE_RT void aye_expect_eq_b(bool want, bool got, int line, int col) {
    if (want != got) AYE_ASSERT_FAILED(line, col, "expected %s, found %s", want ? "true" : "false", got ? "true" : "false");
}
AYE_RT void aye_expect_eq_s(const uint8_t *wp, size_t wn, const uint8_t *gp, size_t gn, int line, int col) {
    if (wn != gn || aye_rt_memcmp(wp, gp, wn) != 0)
        AYE_ASSERT_FAILED(line, col, "expected \"%.*s\", found \"%.*s\"", (int)wn, (const char *)wp, (int)gn, (const char *)gp);
}
/* A test returned `err` (0: no error): report it (with the trace in debug
   mode) and the leaks of the test allocator. Returns the exit status. */
AYE_RT int aye_test_end(uint16_t err) {
    int status = err ? aye_main_error(err) : 0;
    if (aye_general_deinit(&aye_test_gpa)) status = 1;
    return status;
}

AYE_RT void aye_print_i(int64_t v, bool nl) { aye_rt_printf(nl ? "%lld\n" : "%lld", (long long)v); }
AYE_RT void aye_print_u(uint64_t v, bool nl) { aye_rt_printf(nl ? "%llu\n" : "%llu", (unsigned long long)v); }
AYE_RT void aye_print_f(double v, bool nl) { aye_rt_printf(nl ? "%g\n" : "%g", v); }
AYE_RT void aye_print_b(bool v, bool nl) { aye_rt_printf(nl ? "%s\n" : "%s", v ? "true" : "false"); }
AYE_RT void aye_print_s(const uint8_t *p, size_t n, bool nl) {
    aye_rt_printf(nl ? "%.*s\n" : "%.*s", (int)n, (const char *)p);
}

/* std.log: each piece of the format and argument is written to stderr. */
AYE_RT void aye_log_s(const uint8_t *p, size_t n) { aye_rt_dprintf(2, "%.*s", (int)n, (const char *)p); }
AYE_RT void aye_log_i(int64_t v) { aye_rt_dprintf(2, "%lld", (long long)v); }
AYE_RT void aye_log_u(uint64_t v) { aye_rt_dprintf(2, "%llu", (unsigned long long)v); }
AYE_RT void aye_log_f(double v) { aye_rt_dprintf(2, "%g", v); }
AYE_RT void aye_log_b(bool v) { aye_rt_dprintf(2, "%s", v ? "true" : "false"); }
AYE_RT void aye_log_e(uint16_t e) { aye_rt_dprintf(2, "%s", aye_error_names[e]); }

/* Hooks that std calls through `extern fn`: the program's arguments;
   `open` for reading or `create` (write, create, truncate) with a path that
   is not NUL-terminated (the fd, or -errno); `exists`; `mtime`; and the
   monotonic clock in nanoseconds. Flag and clock values, and where
   `struct stat` keeps the modification time, differ between macOS and
   Linux. */
static int aye_argc;
static char **aye_argv;
AYE_RT int32_t aye_std_argc(void) { return aye_argc; }
AYE_RT uint8_t *aye_std_argv(int32_t i) { return (uint8_t *)aye_argv[i]; }
#ifdef __APPLE__
extern int *aye_rt_errno_loc(void) __asm__(AYE_SYM("__error"));
#else
extern int *aye_rt_errno_loc(void) __asm__(AYE_SYM("__errno_location"));
#endif
#ifdef __APPLE__
#define AYE_O_CREATE 0x601 /* O_WRONLY | O_CREAT | O_TRUNC */
#define AYE_CLOCK_MONOTONIC 6
#define AYE_ST_MTIME 6 /* st_mtimespec, in int64_t: byte 48 */
#define AYE_ST_SIZE 12 /* st_size: byte 96 */
#define AYE_ST_MODE 4 /* st_mode, 16 bits: byte 4 */
#define AYE_ENOTEMPTY 66
#define AYE_D_TYPE 20 /* dirent: d_type, then d_name */
#else
#define AYE_O_CREATE 01101
#define AYE_CLOCK_MONOTONIC 1
#define AYE_ST_MTIME 11 /* st_mtim, byte 88 on x86_64 and aarch64 */
#define AYE_ST_SIZE 6 /* st_size: byte 48 on both */
#ifdef __x86_64__
#define AYE_ST_MODE 24 /* st_mode, low 16 bits: byte 24 */
#else
#define AYE_ST_MODE 16 /* byte 16 on aarch64 */
#endif
#define AYE_ENOTEMPTY 39
#define AYE_D_TYPE 18
#endif
static __attribute__((unused)) char *aye_c_path(uint8_t *path, size_t len) {
    char *p = aye_rt_malloc(len + 1);
    if (!p) return 0;
    for (size_t i = 0; i < len; i++) p[i] = (char)path[i];
    p[len] = 0;
    return p;
}
static __attribute__((unused)) int32_t aye_std_open_flags(uint8_t *path, size_t len, int flags) {
    char *p = aye_c_path(path, len);
    if (!p) return -12;
    int fd = aye_rt_open(p, flags, 0644);
    int err = fd < 0 ? *aye_rt_errno_loc() : 0;
    aye_rt_free(p);
    return fd < 0 ? -err : fd;
}
AYE_RT int32_t aye_std_open(uint8_t *path, size_t len) { return aye_std_open_flags(path, len, 0); }
AYE_RT int32_t aye_std_create(uint8_t *path, size_t len) { return aye_std_open_flags(path, len, AYE_O_CREATE); }
AYE_RT bool aye_std_exists(uint8_t *path, size_t len) {
    char *p = aye_c_path(path, len);
    if (!p) return false;
    int r = aye_rt_access(p, 0);
    aye_rt_free(p);
    return r == 0;
}
/* std.fs.mtime: the modification time in nanoseconds since 1970, in
   `*ns`; false if `stat` fails. Gap #50 (no per-OS declarations): the
   field is `st_mtimespec` on macOS and `st_mtim` on Linux, so Aye code
   cannot name it. `st` is larger than both OSes' `struct stat` (144
   bytes). */
AYE_RT bool aye_std_mtime(uint8_t *path, size_t len, int64_t *ns) {
    char *p = aye_c_path(path, len);
    if (!p) return false;
    int64_t st[32];
    int r = aye_rt_stat(p, st);
    aye_rt_free(p);
    if (r != 0) return false;
    *ns = st[AYE_ST_MTIME] * 1000000000 + st[AYE_ST_MTIME + 1];
    return true;
}
/* std.fs: stat, readDir, mkdir, remove and rename. `-errno` as a result
   keeps the numbers that are the same on both OSes; ENOTEMPTY (66 on macOS,
   39 on Linux) is -39 on both. */
static __attribute__((unused)) int32_t aye_std_fail(void) {
    int e = *aye_rt_errno_loc();
    return e == AYE_ENOTEMPTY ? -39 : -e;
}
/* `lstat` of the path (`stat` if `follow`) into `out`: the size, the
   modification time in nanoseconds since 1970 and the S_IFMT bits of the
   mode (the same values on both OSes); 0, or -errno. */
AYE_RT int32_t aye_std_lstat(uint8_t *path, size_t len, bool follow, int64_t *out) {
    char *p = aye_c_path(path, len);
    if (!p) return -12;
    int64_t st[32];
    int r = follow ? aye_rt_stat(p, st) : aye_rt_lstat(p, st);
    int32_t err = r != 0 ? aye_std_fail() : 0;
    aye_rt_free(p);
    if (r != 0) return err;
    out[0] = st[AYE_ST_SIZE];
    out[1] = st[AYE_ST_MTIME] * 1000000000 + st[AYE_ST_MTIME + 1];
    out[2] = *(uint16_t *)((char *)st + AYE_ST_MODE) & 0170000;
    return 0;
}
/* `mkdir` (op 0), `remove` (op 1) of `path`, and `rename` (op 2) of `path`
   to `to`; 0, or -errno. */
AYE_RT int32_t aye_std_pathop(int32_t op, uint8_t *path, size_t len, uint8_t *to, size_t tolen) {
    char *p = aye_c_path(path, len);
    char *q = op == 2 ? aye_c_path(to, tolen) : 0;
    int32_t err = -12;
    if (p && (op != 2 || q)) {
        int r = op == 0 ? aye_rt_mkdir(p, 0777) : op == 1 ? aye_rt_remove(p) : aye_rt_rename(p, q);
        err = r != 0 ? aye_std_fail() : 0;
    }
    aye_rt_free(p);
    aye_rt_free(q);
    return err;
}
/* `opendir`: the directory, or null with -errno in `*err`. */
AYE_RT uint8_t *aye_std_opendir(uint8_t *path, size_t len, int32_t *err) {
    char *p = aye_c_path(path, len);
    if (!p) {
        *err = -12;
        return 0;
    }
    uint8_t *d = aye_rt_opendir(p);
    if (!d) *err = aye_std_fail();
    aye_rt_free(p);
    return d;
}
/* `readdir`: the next entry's NUL-terminated name, and its `d_type` in
   `*kind`; null at the end. */
AYE_RT uint8_t *aye_std_readdir(uint8_t *dir, int32_t *kind) {
    uint8_t *e = aye_rt_readdir(dir);
    if (!e) return 0;
    *kind = e[AYE_D_TYPE];
    return e + AYE_D_TYPE + 1;
}
/* `closedir` and `rewinddir` under our own names: a program that
   `@cimport`s dirent.h declares the libc names itself (E0096). */
AYE_RT void aye_std_closedir(uint8_t *dir) { aye_rt_closedir(dir); }
AYE_RT void aye_std_rewinddir(uint8_t *dir) { aye_rt_rewinddir(dir); }
/* std.net: errno. Its C symbol differs per OS (`__error`, `__errno_location`),
   and Aye has no per-OS declarations, so this helper stays. */
AYE_RT int32_t aye_std_errno(void) { return *aye_rt_errno_loc(); }
/* std.os.getenv: the value (not a copy), or null. `name` is not
   NUL-terminated. */
extern char *aye_rt_getenv(const char *) __asm__(AYE_SYM("getenv"));
AYE_RT uint8_t *aye_std_getenv(uint8_t *name, size_t len) {
    char *p = aye_c_path(name, len);
    if (!p) return 0;
    char *v = aye_rt_getenv(p);
    aye_rt_free(p);
    return (uint8_t *)v;
}
/* std.process.spawn: `args` holds `argc` NUL-terminated strings one after
   another; `modes` has two bits per stream, stdin lowest (0: inherit, 1: a
   pipe). `out` gets the process id, then the parent's end of the stdin,
   stdout and stderr pipes (-1 where there is none). Returns 0 or the errno
   of the failure, including that of `execvp` in the child, which reports it
   through a close-on-exec pipe. Every pipe end is close-on-exec; `dup2`
   clears the flag on the child's 0, 1 and 2. */
extern int aye_rt_pipe(int *) __asm__(AYE_SYM("pipe"));
extern int aye_rt_close(int) __asm__(AYE_SYM("close"));
extern int aye_rt_dup2(int, int) __asm__(AYE_SYM("dup2"));
extern int aye_rt_fork(void) __asm__(AYE_SYM("fork"));
extern int aye_rt_execvp(const char *, char *const *) __asm__(AYE_SYM("execvp"));
extern long aye_rt_read_fd(int, void *, size_t) __asm__(AYE_SYM("read"));
extern long aye_rt_write_fd(int, const void *, size_t) __asm__(AYE_SYM("write"));
extern int aye_rt_waitpid(int, int *, int) __asm__(AYE_SYM("waitpid"));
extern int aye_rt_fcntl(int, int, ...) __asm__(AYE_SYM("fcntl"));
extern void aye_rt_exit_now(int) __asm__(AYE_SYM("_exit")) __attribute__((noreturn));
AYE_RT int32_t aye_std_spawn(uint8_t *args, int32_t argc, int32_t modes, int32_t *out) {
    int pp[3][2] = {{-1, -1}, {-1, -1}, {-1, -1}};
    int ep[2] = {-1, -1};
    int err = 0;
    int pid = -1;
    char **argv = aye_rt_malloc(((size_t)argc + 1) * sizeof(char *));
    if (!argv) return 12;
    char *a = (char *)args;
    for (int i = 0; i < argc; i++) {
        argv[i] = a;
        while (*a) a++;
        a++;
    }
    argv[argc] = 0;
    for (int s = 0; s < 3; s++) {
        if (((modes >> (2 * s)) & 3) == 0) continue;
        if (aye_rt_pipe(pp[s]) != 0) { err = *aye_rt_errno_loc(); goto fail; }
        aye_rt_fcntl(pp[s][0], 2, 1);
        aye_rt_fcntl(pp[s][1], 2, 1);
    }
    if (aye_rt_pipe(ep) != 0) { err = *aye_rt_errno_loc(); goto fail; }
    aye_rt_fcntl(ep[0], 2, 1);
    aye_rt_fcntl(ep[1], 2, 1);
    aye_rt_fflush(0);
    pid = aye_rt_fork();
    if (pid < 0) { err = *aye_rt_errno_loc(); goto fail; }
    if (pid == 0) {
        for (int s = 0; s < 3; s++) {
            if (pp[s][0] < 0) continue;
            if (aye_rt_dup2(s == 0 ? pp[s][0] : pp[s][1], s) < 0) goto child_fail;
        }
        aye_rt_execvp(argv[0], argv);
    child_fail:;
        int e = *aye_rt_errno_loc();
        aye_rt_write_fd(ep[1], &e, sizeof e);
        aye_rt_exit_now(127);
    }
    aye_rt_close(ep[1]);
    ep[1] = -1;
    long n;
    do {
        n = aye_rt_read_fd(ep[0], &err, sizeof err);
    } while (n < 0 && *aye_rt_errno_loc() == 4);
    if (n == (long)sizeof err) {
        int st;
        while (aye_rt_waitpid(pid, &st, 0) < 0 && *aye_rt_errno_loc() == 4) {}
        goto fail;
    }
    err = 0;
    aye_rt_close(ep[0]);
    out[0] = pid;
    for (int s = 0; s < 3; s++) {
        out[s + 1] = -1;
        if (pp[s][0] < 0) continue;
        aye_rt_close(s == 0 ? pp[s][0] : pp[s][1]);
        out[s + 1] = s == 0 ? pp[s][1] : pp[s][0];
    }
    aye_rt_free(argv);
    return 0;
fail:
    for (int s = 0; s < 3; s++) {
        if (pp[s][0] >= 0) { aye_rt_close(pp[s][0]); aye_rt_close(pp[s][1]); }
    }
    if (ep[0] >= 0) aye_rt_close(ep[0]);
    if (ep[1] >= 0) aye_rt_close(ep[1]);
    aye_rt_free(argv);
    return err ? err : 5;
}
AYE_RT double aye_std_inf(void) { return __builtin_inf(); }
AYE_RT double aye_std_nan(void) { return __builtin_nan(""); }
AYE_RT uint64_t aye_std_monotonic_ns(void) {
    int64_t ts[2] = {0, 0};
    aye_rt_clock_gettime(AYE_CLOCK_MONOTONIC, ts);
    return (uint64_t)ts[0] * 1000000000u + (uint64_t)ts[1];
}

/* Stack overflow (debug and safe mode): a SIGSEGV/SIGBUS handler, running
   on an alternate signal stack, reports a fault within 1 MiB of the end of
   the thread's stack as `panic: stack overflow`; any other fault crashes
   as it would without the handler. Each thread records the top of its
   stack (an address in its first frame) and its size: the main thread's
   from `getrlimit`, another thread's from `aye_thread_spawn`. The C
   structures differ between macOS and Linux and are written out here. */
#ifdef __APPLE__
#define AYE_SIGBUS 10
#define AYE_SA_FLAGS 0x0041 /* SA_ONSTACK | SA_SIGINFO */
#define AYE_SS_DISABLE 4
#define AYE_SI_ADDR 24
typedef struct { void *sp; size_t size; int flags; } aye_stack_t;
typedef struct { void *action; uint32_t mask; int flags; } aye_sigaction_t;
#else
#define AYE_SIGBUS 7
#define AYE_SA_FLAGS 0x08000004 /* SA_ONSTACK | SA_SIGINFO */
#define AYE_SS_DISABLE 2
#define AYE_SI_ADDR 16
typedef struct { void *sp; int flags; size_t size; } aye_stack_t;
typedef struct { void *action; uint64_t mask[16]; int flags; void *restorer; } aye_sigaction_t;
#endif
#define AYE_SIGSEGV 11
#define AYE_ALTSTACK ((size_t)128 << 10)
extern int aye_rt_sigaltstack(const aye_stack_t *, aye_stack_t *) __asm__(AYE_SYM("sigaltstack"));
extern int aye_rt_sigaction(int, const aye_sigaction_t *, aye_sigaction_t *) __asm__(AYE_SYM("sigaction"));
extern int aye_rt_getrlimit(int, uint64_t *) __asm__(AYE_SYM("getrlimit"));
extern long aye_rt_write(int, const void *, size_t) __asm__(AYE_SYM("write"));
static _Thread_local __attribute__((unused)) uintptr_t aye_stack_top, aye_stack_size;
static _Thread_local __attribute__((unused)) void *aye_altstack;
/* std.coro: a coroutine (see `aye_coro_init` below). While one runs,
   `aye_stack_top` and `aye_stack_size` are its stack's, so a fault in the
   guard region below its stack is reported as a stack overflow too. */
#define AYE_CORO_GUARD ((size_t)1 << 20)
#define AYE_CORO_MAGIC 0x4f5245434f524f21ull
enum { AYE_CORO_SUSPENDED = 1, AYE_CORO_RUNNING, AYE_CORO_DONE };
typedef struct aye_coro {
    uint64_t magic;
    void *sp;                /* the coroutine's stack pointer while it is suspended */
    void *back;              /* the resumer's while the coroutine runs */
    struct aye_coro *parent; /* the coroutine that resumed it; null: the thread */
    uint8_t *map;            /* the mapping: the guard region, then the stack */
    size_t map_size;
    void (*f)(uint8_t *);
    uint8_t *arg;
    const void *thread;      /* the thread that made it */
    uintptr_t saved_top, saved_size;
    aye_trace_ent *saved_trace;
    uint32_t saved_trace_len, trace_len;
    int state;
    aye_trace_ent trace[AYE_TRACE ? AYE_TRACE_MAX : 1];
} aye_coro;
static _Thread_local __attribute__((unused)) aye_coro *aye_coro_current;
static _Thread_local __attribute__((unused)) char aye_coro_thread; /* its address names the thread */
/* Each thread keeps up to AYE_CORO_CACHE freed stacks for the next
   coroutines of the same size, with only their top AYE_CORO_HOT bytes
   still taken (the rest is given back with madvise): making a coroutine
   then needs no system call. */
#define AYE_CORO_CACHE 16
#define AYE_CORO_HOT ((size_t)256 << 10)
extern void *aye_rt_mmap(void *, size_t, int, int, int, int64_t) __asm__(AYE_SYM("mmap"));
extern int aye_rt_munmap(void *, size_t) __asm__(AYE_SYM("munmap"));
extern int aye_rt_mprotect(void *, size_t, int) __asm__(AYE_SYM("mprotect"));
extern int aye_rt_madvise(void *, size_t, int) __asm__(AYE_SYM("madvise"));
static _Thread_local __attribute__((unused)) struct { uint8_t *map; size_t size; } aye_coro_cache[AYE_CORO_CACHE];
static _Thread_local __attribute__((unused)) uint32_t aye_coro_cached;
/* Unmap the stacks the thread keeps (it is ending). */
AYE_RT void aye_coro_drain(void) {
    while (aye_coro_cached) {
        aye_coro_cached--;
        aye_rt_munmap(aye_coro_cache[aye_coro_cached].map, aye_coro_cache[aye_coro_cached].size);
    }
}
static void aye_on_fault(int sig, void *info, void *ctx) {
    (void)ctx;
    uintptr_t addr = *(uintptr_t *)((char *)info + AYE_SI_ADDR);
    uintptr_t near = (uintptr_t)1 << 20;
    uintptr_t lo = aye_stack_top > aye_stack_size ? aye_stack_top - aye_stack_size : 0;
    if (aye_stack_top && addr < aye_stack_top && addr + near >= lo && addr < lo + near) {
        static const char msg[] =
            "panic: stack overflow (deep recursion: run it on a thread with a larger stack, std.thread.spawnWithStack)\n";
        static const char coro_msg[] =
            "panic: stack overflow (deep recursion in a coroutine: create it with a larger stack, std.coro.create)\n";
        aye_rt_fflush(0);
        if (aye_coro_current) aye_rt_write(2, coro_msg, sizeof coro_msg - 1);
        else aye_rt_write(2, msg, sizeof msg - 1);
        aye_rt_abort();
    }
    aye_sigaction_t dfl = {0};
    aye_rt_sigaction(sig, &dfl, 0);
}
/* The current thread's stack has `size` bytes, starting at the caller's
   frame: give it an alternate signal stack for the handler. */
AYE_RT __attribute__((noinline)) void aye_stack_guard(size_t size) {
    if (!AYE_CHECKS) return;
    char here;
    aye_stack_top = (uintptr_t)&here;
    aye_stack_size = size;
    aye_altstack = aye_rt_malloc(AYE_ALTSTACK);
    if (!aye_altstack) return;
    aye_stack_t ss = {0};
    ss.sp = aye_altstack;
    ss.size = AYE_ALTSTACK;
    aye_rt_sigaltstack(&ss, 0);
}
AYE_RT void aye_stack_unguard(void) {
    if (!aye_altstack) return;
    aye_stack_t ss = {0};
    ss.flags = AYE_SS_DISABLE;
    aye_rt_sigaltstack(&ss, 0);
    aye_rt_free(aye_altstack);
    aye_altstack = 0;
}
/* At the start of `main`: the handler, and the main thread's guard. An
   unlimited stack (Linux) is taken as 8 MiB. */
AYE_RT void aye_stack_guard_main(void) {
    if (!AYE_CHECKS) return;
#ifndef __wasm32__ /* no signals and no guard page: a stack overflow traps */
    aye_sigaction_t sa = {0};
    sa.action = (void *)aye_on_fault;
    sa.flags = AYE_SA_FLAGS;
    aye_rt_sigaction(AYE_SIGSEGV, &sa, 0);
    aye_rt_sigaction(AYE_SIGBUS, &sa, 0);
    uint64_t lim[2] = {0, 0};
    size_t size = (size_t)8 << 20;
    if (aye_rt_getrlimit(3, lim) == 0 && lim[0] < ((uint64_t)1 << 40)) size = (size_t)lim[0];
    aye_stack_guard(size);
#endif
}

/* std.thread and std.sync: pthreads. A `pthread_t` is 8 bytes on macOS and
   Linux; a mutex or condition variable lives on the C heap (128 bytes holds
   either on both), so an Aye value that refers to it can be copied. */
extern int aye_rt_pthread_create(uint64_t *, const void *, void *(*)(void *), void *) __asm__(AYE_SYM("pthread_create"));
extern int aye_rt_pthread_join(uint64_t, void **) __asm__(AYE_SYM("pthread_join"));
extern int aye_rt_pthread_mutex_init(void *, const void *) __asm__(AYE_SYM("pthread_mutex_init"));
extern int aye_rt_pthread_mutex_lock(void *) __asm__(AYE_SYM("pthread_mutex_lock"));
extern int aye_rt_pthread_mutex_unlock(void *) __asm__(AYE_SYM("pthread_mutex_unlock"));
extern int aye_rt_pthread_mutex_destroy(void *) __asm__(AYE_SYM("pthread_mutex_destroy"));
extern int aye_rt_pthread_cond_init(void *, const void *) __asm__(AYE_SYM("pthread_cond_init"));
extern int aye_rt_pthread_cond_wait(void *, void *) __asm__(AYE_SYM("pthread_cond_wait"));
extern int aye_rt_pthread_cond_signal(void *) __asm__(AYE_SYM("pthread_cond_signal"));
extern int aye_rt_pthread_cond_broadcast(void *) __asm__(AYE_SYM("pthread_cond_broadcast"));
extern int aye_rt_pthread_cond_destroy(void *) __asm__(AYE_SYM("pthread_cond_destroy"));
extern int aye_rt_pthread_attr_init(void *) __asm__(AYE_SYM("pthread_attr_init"));
extern int aye_rt_pthread_attr_setstacksize(void *, size_t) __asm__(AYE_SYM("pthread_attr_setstacksize"));
extern int aye_rt_pthread_attr_destroy(void *) __asm__(AYE_SYM("pthread_attr_destroy"));
struct aye_thread_start { void (*f)(uint8_t *); uint8_t *arg; size_t stack; };
static void *aye_thread_main(void *p) {
    struct aye_thread_start s = *(struct aye_thread_start *)p;
    aye_rt_free(p);
    aye_stack_guard(s.stack);
    s.f(s.arg);
    aye_coro_drain();
    aye_stack_unguard();
    return 0;
}
/* Start `f(arg)` on a thread with a stack of at least `stack` bytes: a
   multiple of 64 KiB (macOS wants a multiple of the page size), and at
   least 256 KiB. A `pthread_attr_t` fits in 128 bytes on macOS and Linux. */
AYE_RT bool aye_thread_spawn(void (*f)(uint8_t *), uint8_t *arg, size_t stack, uint64_t *out) {
    if (stack > SIZE_MAX - 0xffff) return false;
    stack = stack < ((size_t)256 << 10) ? (size_t)256 << 10 : (stack + 0xffff) & ~(size_t)0xffff;
    struct aye_thread_start *s = aye_rt_malloc(sizeof *s);
    if (!s) return false;
    s->f = f;
    s->arg = arg;
    s->stack = stack;
    _Alignas(16) unsigned char attr[128];
    aye_rt_pthread_attr_init(attr);
    bool ok = aye_rt_pthread_attr_setstacksize(attr, stack) == 0 && aye_rt_pthread_create(out, attr, aye_thread_main, s) == 0;
    aye_rt_pthread_attr_destroy(attr);
    if (!ok) aye_rt_free(s);
    return ok;
}
AYE_RT void aye_thread_join(uint64_t t) { aye_rt_pthread_join(t, 0); }
AYE_RT __attribute__((noreturn, cold)) void aye_sync_oom(void) {
    aye_rt_fflush(0);
    aye_rt_dprintf(2, "panic: out of memory for a mutex or condition variable\n");
    aye_rt_abort();
}
AYE_RT uint8_t *aye_mutex_new(void) {
    void *m = aye_rt_malloc(128);
    if (!m) aye_sync_oom();
    aye_rt_pthread_mutex_init(m, 0);
    return m;
}
AYE_RT void aye_mutex_lock(uint8_t *m) { aye_rt_pthread_mutex_lock(m); }
AYE_RT void aye_mutex_unlock(uint8_t *m) { aye_rt_pthread_mutex_unlock(m); }
AYE_RT void aye_mutex_free(uint8_t *m) {
    aye_rt_pthread_mutex_destroy(m);
    aye_rt_free(m);
}
AYE_RT uint8_t *aye_cond_new(void) {
    void *c = aye_rt_malloc(128);
    if (!c) aye_sync_oom();
    aye_rt_pthread_cond_init(c, 0);
    return c;
}
AYE_RT void aye_cond_wait(uint8_t *c, uint8_t *m) { aye_rt_pthread_cond_wait(c, m); }
AYE_RT void aye_cond_signal(uint8_t *c) { aye_rt_pthread_cond_signal(c); }
AYE_RT void aye_cond_broadcast(uint8_t *c) { aye_rt_pthread_cond_broadcast(c); }
AYE_RT void aye_cond_free(uint8_t *c) {
    aye_rt_pthread_cond_destroy(c);
    aye_rt_free(c);
}

/* std.coro: stackful coroutines. A coroutine's stack is an mmap'd region
   with AYE_CORO_GUARD bytes of PROT_NONE at its low end (only the pages
   that are used are taken). `aye_coro_switch(save, to)`, assembly per
   target, pushes the callee-saved registers (x19-x28, fp, lr and d8-d15 on
   aarch64; rbx, rbp, r12-r15, MXCSR and the x87 control word on x86_64),
   stores the stack pointer in `*save`, loads `to` and pops them. A new
   coroutine's stack holds such a frame whose return address is
   `aye_coro_start`, which calls `aye_coro_entry(c)` with c and the entry
   point taken from two of the restored registers. The control block is
   allocated by std.coro with the program's allocator. A coroutine runs
   only on the thread that made it: the running coroutine, the stack bounds
   of the overflow check and the error return trace are thread-locals
   switched with it, and the C compiler may keep a thread-local's address
   in a frame across a switch. */
#if defined(__aarch64__)
__asm__(".text\n"
        ".p2align 4\n"
        AYE_SYM("aye_coro_switch") ":\n"
        "    sub sp, sp, #160\n"
        "    stp x19, x20, [sp, #0]\n"
        "    stp x21, x22, [sp, #16]\n"
        "    stp x23, x24, [sp, #32]\n"
        "    stp x25, x26, [sp, #48]\n"
        "    stp x27, x28, [sp, #64]\n"
        "    stp x29, x30, [sp, #80]\n"
        "    stp d8, d9, [sp, #96]\n"
        "    stp d10, d11, [sp, #112]\n"
        "    stp d12, d13, [sp, #128]\n"
        "    stp d14, d15, [sp, #144]\n"
        "    mov x9, sp\n"
        "    str x9, [x0]\n"
        "    mov sp, x1\n"
        "    ldp x19, x20, [sp, #0]\n"
        "    ldp x21, x22, [sp, #16]\n"
        "    ldp x23, x24, [sp, #32]\n"
        "    ldp x25, x26, [sp, #48]\n"
        "    ldp x27, x28, [sp, #64]\n"
        "    ldp x29, x30, [sp, #80]\n"
        "    ldp d8, d9, [sp, #96]\n"
        "    ldp d10, d11, [sp, #112]\n"
        "    ldp d12, d13, [sp, #128]\n"
        "    ldp d14, d15, [sp, #144]\n"
        "    add sp, sp, #160\n"
        "    ret\n"
        ".p2align 4\n"
        AYE_SYM("aye_coro_start") ":\n"
        "    mov x0, x19\n"
        "    blr x20\n"
        "    brk #0\n");
#define AYE_CORO_FRAME 160
#define AYE_CORO_SLOT_ARG 0   /* x19 */
#define AYE_CORO_SLOT_ENTRY 1 /* x20 */
#define AYE_CORO_SLOT_RET 11  /* x30 */
#define AYE_CORO_OK 1
#elif defined(__x86_64__)
__asm__(".text\n"
        ".p2align 4\n"
        AYE_SYM("aye_coro_switch") ":\n"
        "    pushq %rbp\n"
        "    pushq %rbx\n"
        "    pushq %r12\n"
        "    pushq %r13\n"
        "    pushq %r14\n"
        "    pushq %r15\n"
        "    subq $8, %rsp\n"
        "    stmxcsr (%rsp)\n"
        "    fnstcw 4(%rsp)\n"
        "    movq %rsp, (%rdi)\n"
        "    movq %rsi, %rsp\n"
        "    ldmxcsr (%rsp)\n"
        "    fldcw 4(%rsp)\n"
        "    addq $8, %rsp\n"
        "    popq %r15\n"
        "    popq %r14\n"
        "    popq %r13\n"
        "    popq %r12\n"
        "    popq %rbx\n"
        "    popq %rbp\n"
        "    ret\n"
        ".p2align 4\n"
        AYE_SYM("aye_coro_start") ":\n"
        "    movq %r12, %rdi\n"
        "    callq *%r13\n"
        "    ud2\n");
#define AYE_CORO_FRAME 64
#define AYE_CORO_SLOT_ARG 4   /* r12 */
#define AYE_CORO_SLOT_ENTRY 3 /* r13 */
#define AYE_CORO_SLOT_RET 7   /* the return address */
#define AYE_CORO_OK 1
#else
#define AYE_CORO_OK 0
#endif
#if AYE_CORO_OK
extern void aye_coro_switch(void **save, void *to) __asm__(AYE_SYM("aye_coro_switch"));
extern void aye_coro_start(void) __asm__(AYE_SYM("aye_coro_start"));
#endif
#ifdef __APPLE__
#define AYE_MAP_STACK 0x1002 /* MAP_PRIVATE | MAP_ANON */
#define AYE_MADV_FREE 5      /* MADV_FREE */
#else
#define AYE_MAP_STACK 0x4022 /* MAP_PRIVATE | MAP_ANONYMOUS | MAP_NORESERVE */
#define AYE_MADV_FREE 4      /* MADV_DONTNEED */
#endif
AYE_RT __attribute__((noreturn, cold)) void aye_coro_panic(const char *msg) {
    aye_rt_fflush(0);
    aye_rt_dprintf(2, "panic: std.coro: %s\n", msg);
    aye_rt_abort();
}
AYE_RT size_t aye_coro_words(void) { return (sizeof(aye_coro) + 7) / 8; }
/* Back to the code that resumed `c`. */
AYE_RT void aye_coro_leave(aye_coro *c) {
#if AYE_CORO_OK
    aye_coro_current = c->parent;
    if (AYE_CHECKS) {
        aye_stack_top = c->saved_top;
        aye_stack_size = c->saved_size;
    }
    if (AYE_TRACE) {
        c->trace_len = aye_trace_len;
        aye_trace_buf = c->saved_trace;
        aye_trace_len = c->saved_trace_len;
    }
    aye_coro_switch(&c->sp, c->back);
#endif
}
static __attribute__((noreturn, unused)) void aye_coro_entry(aye_coro *c) {
    c->f(c->arg);
    c->state = AYE_CORO_DONE;
    aye_coro_leave(c);
    __builtin_trap();
}
/* A coroutine in `block` (aye_coro_words() words) that runs `f(arg)` on a
   stack of at least `stack` bytes; false when there is no memory for it. */
AYE_RT bool aye_coro_init(uint64_t *block, size_t stack, void (*f)(uint8_t *), uint8_t *arg) {
#if AYE_CORO_OK
    aye_coro *c = (aye_coro *)block;
    if (stack > SIZE_MAX - AYE_CORO_GUARD - 0xffff) return false;
    stack = stack < ((size_t)64 << 10) ? (size_t)64 << 10 : (stack + 0xffff) & ~(size_t)0xffff;
    size_t size = AYE_CORO_GUARD + stack;
    uint8_t *m = 0;
    for (uint32_t i = aye_coro_cached; i-- > 0;) {
        if (aye_coro_cache[i].size == size) {
            m = aye_coro_cache[i].map;
            aye_coro_cache[i] = aye_coro_cache[--aye_coro_cached];
            break;
        }
    }
    if (!m) {
        m = aye_rt_mmap(0, size, 3 /* PROT_READ | PROT_WRITE */, AYE_MAP_STACK, -1, 0);
        if (m == (uint8_t *)-1) return false;
        if (aye_rt_mprotect(m, AYE_CORO_GUARD, 0 /* PROT_NONE */) != 0) {
            aye_rt_munmap(m, size);
            return false;
        }
    }
    __builtin_memset(c, 0, sizeof *c);
    c->magic = AYE_CORO_MAGIC;
    c->map = m;
    c->map_size = size;
    c->f = f;
    c->arg = arg;
    c->thread = &aye_coro_thread;
    c->state = AYE_CORO_SUSPENDED;
    void **frame = (void **)(m + size - AYE_CORO_FRAME);
    __builtin_memset(frame, 0, AYE_CORO_FRAME);
#if defined(__x86_64__)
    ((uint32_t *)frame)[0] = 0x1f80; /* MXCSR: all exceptions masked, round to nearest */
    ((uint16_t *)frame)[2] = 0x037f; /* x87: the same */
#endif
    frame[AYE_CORO_SLOT_ARG] = c;
    frame[AYE_CORO_SLOT_ENTRY] = (void *)aye_coro_entry;
    frame[AYE_CORO_SLOT_RET] = (void *)aye_coro_start;
    c->sp = frame;
    return true;
#else
    (void)block, (void)stack, (void)f, (void)arg;
    aye_coro_panic("coroutines are not supported on this target");
#endif
}
AYE_RT void aye_coro_resume(uint64_t *block) {
#if AYE_CORO_OK
    aye_coro *c = (aye_coro *)block;
    if (AYE_CHECKS) {
        if (c->magic != AYE_CORO_MAGIC) aye_coro_panic("resume of a coroutine that was freed");
        if (c->state == AYE_CORO_DONE) aye_coro_panic("resume of a coroutine that has finished");
        if (c->state == AYE_CORO_RUNNING) aye_coro_panic("resume of a coroutine that is running");
        if (c->thread != &aye_coro_thread) aye_coro_panic("resume of a coroutine on another thread than the one that made it");
        c->saved_top = aye_stack_top;
        c->saved_size = aye_stack_size;
        aye_stack_top = (uintptr_t)(c->map + c->map_size);
        aye_stack_size = c->map_size - AYE_CORO_GUARD;
    }
    if (AYE_TRACE) {
        c->saved_trace = aye_trace_buf;
        c->saved_trace_len = aye_trace_len;
        aye_trace_buf = c->trace;
        aye_trace_len = c->trace_len;
    }
    c->parent = aye_coro_current;
    c->state = AYE_CORO_RUNNING;
    aye_coro_current = c;
    aye_coro_switch(&c->back, c->sp);
#endif
}
AYE_RT void aye_coro_yield(void) {
    aye_coro *c = aye_coro_current;
    if (AYE_CHECKS && !c) aye_coro_panic("yield outside a coroutine");
    c->state = AYE_CORO_SUSPENDED;
    aye_coro_leave(c);
}
AYE_RT bool aye_coro_done(uint64_t *block) {
    aye_coro *c = (aye_coro *)block;
    if (AYE_CHECKS && c->magic != AYE_CORO_MAGIC) aye_coro_panic("isDone of a coroutine that was freed");
    return c->state == AYE_CORO_DONE;
}
AYE_RT void aye_coro_free(uint64_t *block) {
    aye_coro *c = (aye_coro *)block;
    if (AYE_CHECKS) {
        if (c->magic != AYE_CORO_MAGIC) aye_coro_panic("deinit of a coroutine that was freed");
        if (c->state == AYE_CORO_RUNNING) aye_coro_panic("deinit of a coroutine that is running");
    }
    c->magic = 0;
    if (aye_coro_cached == AYE_CORO_CACHE) {
        aye_rt_munmap(c->map, c->map_size);
        return;
    }
    if (c->map_size - AYE_CORO_GUARD > AYE_CORO_HOT) {
        aye_rt_madvise(c->map + AYE_CORO_GUARD, c->map_size - AYE_CORO_GUARD - AYE_CORO_HOT, AYE_MADV_FREE);
    }
    aye_coro_cache[aye_coro_cached].map = c->map;
    aye_coro_cache[aye_coro_cached].size = c->map_size;
    aye_coro_cached++;
}
"#;

/// Emit the C translation unit for `p`. `files` are the Aye source names
/// (indexed by `Span::file`) used in `#line` directives and panic messages;
/// `checks` enables runtime checks and `trace` the error return trace (debug
/// mode). With `tests`, `main` runs one test instead of `aye_f_main`.
pub fn emit(p: &Program, files: &[String], checks: bool, trace: bool, tests: bool) -> String {
    let files = files.iter().map(|f| c_str(f.as_bytes())).collect();
    let mut g = Gen {
        out: String::new(),
        files,
        trace,
        checks,
        mode_checks: checks,
        enum_names: Vec::new(),
        loops: Vec::new(),
        next_label: 0,
    };
    g.program(p, checks, tests);
    g.out
}

/// A struct or function name as a C identifier. An instance of a generic
/// (`Pool(Node)`) starts with `0`, which no Aye name does, and has `_`
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

/// C name of the Aye function `name`: `f`, `Struct.method` or `test#N`.
fn fn_name(name: &str) -> String {
    if let Some(n) = name
        .rsplit("::")
        .next()
        .and_then(|n| n.strip_prefix("test#"))
    {
        return format!("aye_t_{n}");
    }
    match name.rsplit_once('.') {
        Some((s, m)) => {
            let s = c_name(s);
            format!("aye_m_{}{s}_{m}", s.len())
        }
        None => format!("aye_f_{}", c_name(name)),
    }
}

/// C names of the vtable of struct `s` for interface `i`, and of the thunk
/// for its method `m`.
fn vtable_name(i: &str, s: &str) -> String {
    let i = c_name(i);
    format!("aye_vt_{}{i}_{}", i.len(), c_name(s))
}

fn thunk_name(i: &str, s: &str, m: &str) -> String {
    let (i, s) = (c_name(i), c_name(s));
    format!("aye_th_{}{i}_{}{s}_{m}", i.len(), s.len())
}

struct Gen {
    out: String,
    /// The source file names as C string literals.
    files: Vec<String>,
    trace: bool,
    /// Emit runtime checks: not in `fast` mode, nor in a `@setSafety(false)`
    /// block.
    checks: bool,
    /// The build mode has runtime checks (not `fast`).
    mode_checks: bool,
    /// For each enclosing loop: the label `continue` jumps to, if any.
    loops: Vec<Option<String>>,
    /// Each enum's variant names, for `@tagName`.
    enum_names: Vec<(String, Vec<String>)>,
    next_label: usize,
}

const C_KEYWORDS: &[&str] = &[
    "auto", "break", "case", "char", "const", "continue", "default", "do", "double", "else",
    "enum", "extern", "float", "for", "goto", "if", "inline", "int", "long", "register",
    "restrict", "return", "short", "signed", "sizeof", "static", "struct", "switch", "typedef",
    "union", "unsigned", "void", "volatile", "while", "bool",
];

/// C name of a struct field (C keywords get a trailing underscore).
fn c_field(name: &str) -> String {
    if C_KEYWORDS.contains(&name) {
        format!("{name}_")
    } else {
        name.to_string()
    }
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
    if s.contains(['.', 'e', 'E']) {
        s
    } else {
        format!("{s}.0")
    }
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
        Type::Simd(e, n) => format!("V{n}_{}", mangle(e)),
        // `[]T` and `[]mut T` share one C type.
        Type::Slice(e, _) => format!("S_{}", mangle(e)),
        Type::CPtr(e) => format!("P_{}", mangle(e)),
        // `*T` and `*mut T` too (`E!*mut T` is returned as `E!*T`).
        Type::Ptr(e, _) => format!("R_{}", mangle(e)),
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
            let parts: String = params
                .iter()
                .chain([&**ret])
                .map(|t| {
                    let m = mangle(t);
                    format!("{}{m}", m.len())
                })
                .collect();
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
        | Type::Simd(..)
        | Type::Slice(..)
        | Type::Struct(_)
        | Type::Enum(_)
        | Type::Optional(_)
        | Type::ErrorUnion(..)
        | Type::Dyn(_)
        | Type::Builtin(_)
        | Type::Fn(..) => format!("aye_{}", mangle(t)),
        Type::Param(..) | Type::Error => unreachable!(),
    }
}

/// Is `t` an `extern union`?
fn is_union(p: &Program, t: &Type) -> bool {
    matches!(t, Type::Struct(n) if p.structs.iter().any(|s| s.name == *n && s.is_union))
}

/// Types emitted as a C struct (or union).
fn is_composite(t: &Type) -> bool {
    match t {
        Type::Array(..)
        | Type::Slice(..)
        | Type::Struct(_)
        | Type::Enum(_)
        | Type::ErrorUnion(..)
        | Type::Dyn(_) => true,
        Type::Optional(_) => !is_ptr_optional(t),
        _ => false,
    }
}

/// The size in bytes of a lane of a `simd` (a number).
fn lane_size(t: &Type) -> usize {
    match t {
        Type::Int(it) => it.bits() as usize / 8,
        Type::Float(FloatTy::F32) => 4,
        _ => 8,
    }
}

/// The unsigned C type of the width of `it`.
fn uint_c(it: IntTy) -> &'static str {
    match it.bits() {
        8 => "uint8_t",
        16 => "uint16_t",
        32 => "uint32_t",
        _ => "uint64_t",
    }
}

/// `add`, `sub` or `mul`: the name of the `__builtin_*_overflow` of `op`.
fn overflow_builtin(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "add",
        BinOp::Sub => "sub",
        _ => "mul",
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
    format!("aye_g_{}", c_name(name))
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
        self.enum_names = p
            .enums
            .iter()
            .map(|e| {
                (
                    e.name.clone(),
                    e.variants.iter().map(|v| v.name.clone()).collect(),
                )
            })
            .collect();
        self.out
            .push_str("/* Generated by the Aye stage0 compiler. Do not edit. */\n");
        let _ = writeln!(self.out, "#define AYE_CHECKS {}", checks as u8);
        let _ = writeln!(self.out, "#define AYE_TRACE {}", self.trace as u8);
        let _ = writeln!(
            self.out,
            "static const char *const aye_files[] = {{{}}};",
            self.files.join(", ")
        );
        let names: Vec<String> = std::iter::once("\"\"".to_string())
            .chain(p.errors.iter().map(|n| c_str(n.as_bytes())))
            .collect();
        let _ = writeln!(
            self.out,
            "static const char *const aye_error_names[] = {{{}}};",
            names.join(", ")
        );
        self.out.push_str(PRELUDE);
        self.types(p);
        // The vtable type of each interface: one function pointer per method.
        for i in &p.ifaces {
            let mut s = format!("struct aye_VT_{} {{", c_name(&i.name));
            for m in &i.methods {
                let params: String = m
                    .params
                    .iter()
                    .map(|t| format!(", {}", c_type(t)))
                    .collect();
                let _ = write!(
                    s,
                    " {} (*{})(void *{params});",
                    c_type(&m.ret),
                    c_field(&m.name)
                );
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
            let params = if params.is_empty() {
                "void".to_string()
            } else {
                params.join(", ")
            };
            let decl = format!(
                "extern {} {}({params});",
                c_type(&e.ret),
                extern_symbol(&e.name)
            );
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
            let _ = writeln!(
                self.out,
                "static __attribute__((unused)) {} {} = {init};",
                c_type(&g.ty),
                global_name(&g.name)
            );
        }
        for (i, s) in &p.vtables {
            self.vtable(p, i, s);
        }
        // A struct used as an `Allocator`: thunks from the runtime's vtable
        // to its `alloc` and `free` (the position is not passed on).
        for s in &p.alloc_impls {
            if !p.funcs.iter().any(|x| x.name == format!("{s}.alloc")) {
                continue;
            }
            let (a, fr) = (
                fn_name(&format!("{s}.alloc")),
                fn_name(&format!("{s}.free")),
            );
            let (st, n) = (c_type(&Type::Struct(s.clone())), c_name(s));
            let _ = writeln!(
                self.out,
                "static void *aye_at_alloc_{n}(void *self, size_t size, size_t align, int line, int col) {{ return {a}(({st} *)self, size, align); }}"
            );
            let _ = writeln!(
                self.out,
                "static void aye_at_free_{n}(void *self, void *p, size_t size, int line, int col) {{ {fr}(({st} *)self, (uint8_t *)p, size); }}"
            );
            let _ = writeln!(
                self.out,
                "static const aye_alloc_vt aye_at_vt_{n} = {{aye_at_alloc_{n}, aye_at_free_{n}}};"
            );
        }
        for f in &p.funcs {
            self.func(f);
        }
        if tests {
            self.out.push_str("\nint main(int argc, char **argv) {\n    int n = 0;\n    aye_argc = argc;\n    aye_argv = argv;\n    aye_stack_guard_main();\n    if (argc != 2) return 2;\n");
            self.out.push_str("    for (const char *s = argv[1]; *s; s++) n = n * 10 + (*s - '0');\n    switch (n) {\n");
            for i in 0..p.tests.len() {
                let _ = writeln!(
                    self.out,
                    "    case {i}: return aye_test_end(aye_t_{i}().err);"
                );
            }
            self.out.push_str("    }\n    return 2;\n}\n");
            return;
        }
        let main = p.funcs.iter().find(|f| f.is_main).unwrap();
        if let Type::ErrorUnion(..) = main.ret {
            let _ = write!(
                self.out,
                "\nint main(int argc, char **argv) {{\n    aye_argc = argc;\n    aye_argv = argv;\n    aye_stack_guard_main();\n    {} r = aye_f_main();\n    return r.err ? aye_main_error(r.err) : 0;\n}}\n",
                c_type(&main.ret)
            );
        } else {
            self.out.push_str("\nint main(int argc, char **argv) {\n    aye_argc = argc;\n    aye_argv = argv;\n    aye_stack_guard_main();\n    aye_f_main();\n    return 0;\n}\n");
        }
    }

    /// The vtable of struct `s` for interface `i`, and a thunk per method that
    /// passes the data pointer as the method's receiver.
    fn vtable(&mut self, p: &Program, i: &str, s: &str) {
        let iface = p.ifaces.iter().find(|x| x.name == i).unwrap();
        let mut thunks = Vec::new();
        for m in &iface.methods {
            let f = p
                .funcs
                .iter()
                .find(|f| f.name == format!("{s}.{}", m.name))
                .unwrap();
            let st = c_type(&Type::Struct(s.to_string()));
            let recv = match f.locals[f.params[0]].ty {
                Type::Ptr(..) => format!("({st} *)self"),
                _ => format!("*({st} *)self"),
            };
            let params: String = m
                .params
                .iter()
                .enumerate()
                .map(|(k, t)| format!(", {} a{k}", c_type(t)))
                .collect();
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
        let _ = writeln!(
            self.out,
            "static const struct aye_VT_{} {} = {{{}}};",
            c_name(i),
            vtable_name(i, s),
            thunks.join(", ")
        );
    }

    /// Typedefs for every array, slice and struct type, in dependency order.
    fn types(&mut self, p: &Program) {
        let mut all: Vec<Type> = Vec::new();
        let mut seen = HashSet::new();
        let mut add = |t: &Type, all: &mut Vec<Type>| collect_type(t, &mut seen, all);
        add(
            &Type::Slice(Box::new(Type::Int(IntTy::U8)), false),
            &mut all,
        );
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
        let composite: Vec<&Type> = all
            .iter()
            .filter(|t| is_composite(t) && names.insert(c_type(t)))
            .collect();
        self.out.push('\n');
        for t in &composite {
            let n = c_type(t);
            let kind = if is_union(p, t) { "union" } else { "struct" };
            let _ = writeln!(self.out, "typedef {kind} {n} {n};");
        }
        // `simd[N]T`: a C vector (of `int8_t` for a mask), at most 16-byte
        // aligned, as allocators align.
        let mut vecs = HashSet::new();
        for t in all.iter().filter(|t| matches!(t, Type::Simd(..))) {
            let Type::Simd(e, n) = t else { unreachable!() };
            let n = *n as usize;
            let (lane, size) = if **e == Type::Bool {
                ("int8_t".to_string(), n)
            } else {
                (c_type(e), n * lane_size(e))
            };
            if vecs.insert(c_type(t)) {
                let _ = writeln!(
                    self.out,
                    "typedef {lane} {} __attribute__((vector_size({size}), aligned({})));",
                    c_type(t),
                    size.min(16)
                );
            }
        }
        // Function pointer types, after the types they name (`all` lists a
        // type after its parts) and before the structs that hold them.
        let mut fns = HashSet::new();
        for t in all.iter().filter(|t| matches!(t, Type::Fn(..))) {
            let Type::Fn(params, ret) = t else {
                unreachable!()
            };
            let n = c_type(t);
            if fns.insert(n.clone()) {
                let ps: Vec<String> = params.iter().map(c_type).collect();
                let ps = if ps.is_empty() {
                    "void".to_string()
                } else {
                    ps.join(", ")
                };
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
                let _ = writeln!(
                    self.out,
                    "struct {name} {{ {}* ptr; size_t len; }};",
                    c_type(e)
                );
            }
            Type::Dyn(i) => {
                let _ = writeln!(
                    self.out,
                    "struct {name} {{ void *ptr; const struct aye_VT_{} *vt; }};",
                    c_name(i)
                );
            }
            Type::Struct(sname) => {
                let def = p.structs.iter().find(|s| s.name == *sname).unwrap();
                for (_, ft) in &def.fields {
                    self.define(ft, p, done);
                }
                let mut s = format!(
                    "{} {name} {{",
                    if def.is_union { "union" } else { "struct" }
                );
                for (fname, ft) in &def.fields {
                    let _ = write!(s, " {} {};", c_type(ft), c_field(fname));
                }
                s.push_str(" };");
                self.line(0, &s);
            }
            Type::Optional(e) => {
                self.define(e, p, done);
                let _ = writeln!(
                    self.out,
                    "struct {name} {{ bool has; {} val; }};",
                    c_type(e)
                );
            }
            // The error value (0: no error), then the payload.
            Type::ErrorUnion(_, e) if **e == Type::Void => {
                let _ = writeln!(self.out, "struct {name} {{ uint16_t err; }};");
            }
            Type::ErrorUnion(_, e) => {
                self.define(e, p, done);
                let _ = writeln!(
                    self.out,
                    "struct {name} {{ uint16_t err; {} val; }};",
                    c_type(e)
                );
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
        let params: Vec<String> = f
            .params
            .iter()
            .map(|&id| format!("{} {}", c_type(&f.locals[id].ty), Self::local_name(f, id)))
            .collect();
        let params = if params.is_empty() {
            "void".to_string()
        } else {
            params.join(", ")
        };
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
        let _ = writeln!(
            self.out,
            "#line {} {}",
            s.span.line, self.files[s.span.file as usize]
        );
        match &s.kind {
            StmtKind::Assign(place, rv) => {
                let mut checks = Vec::new();
                self.place_checks(f, place, &mut checks);
                self.rvalue_checks(f, rv, &mut checks);
                for c in &checks {
                    self.line(ind, c);
                }
                let p = self.place(f, place);
                let line = match rv {
                    // Overflow is checked as the result is stored.
                    Rvalue::Binary {
                        op: op @ (BinOp::Add | BinOp::Sub | BinOp::Mul),
                        lhs,
                        rhs,
                        ty: Type::Int(_),
                        span,
                    } if self.checks => {
                        let (a, b) = (self.operand(f, lhs), self.operand(f, rhs));
                        format!(
                            "if (AYE_UNLIKELY(__builtin_{}_overflow({a}, {b}, &{p}))) aye_panic({}, \"integer overflow\");",
                            overflow_builtin(*op),
                            line_col(*span)
                        )
                    }
                    _ => format!("{p} = {};", self.rvalue(f, rv)),
                };
                self.line(ind, &line);
            }
            StmtKind::Eval(rv) => {
                let mut checks = Vec::new();
                self.rvalue_checks(f, rv, &mut checks);
                for c in &checks {
                    self.line(ind, c);
                }
                // A value dropped with `_ = x` is cast to `void` (no warning).
                let line = match rv {
                    Rvalue::Use(_) | Rvalue::Load(_) => format!("(void){};", self.rvalue(f, rv)),
                    _ => format!("{};", self.rvalue(f, rv)),
                };
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
                    Some(format!("aye_continue_{}", self.next_label))
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
                self.line(ind, &format!("aye_trace_push({});", line_col(*span)));
            }
            StmtKind::TraceRestore(t) if self.trace => {
                self.line(
                    ind,
                    &format!("aye_trace_len = {};", Self::local_name(f, *t)),
                );
            }
            StmtKind::TracePush(_) | StmtKind::TraceRestore(_) => {}
            StmtKind::Check {
                cond: Some(c),
                msg,
                span,
            } => {
                if self.checks {
                    let line = format!(
                        "if (AYE_UNLIKELY(!{})) aye_panic({}, \"{msg}\");",
                        self.operand(f, c),
                        line_col(*span)
                    );
                    self.line(ind, &line);
                }
            }
            StmtKind::Safety { on, body } => {
                let saved = self.checks;
                self.checks = *on && self.mode_checks;
                self.block(f, body, ind);
                self.checks = saved;
            }
            StmtKind::Check {
                cond: None,
                msg,
                span,
            } => {
                if self.checks {
                    self.line(ind, &format!("aye_panic({}, \"{msg}\");", line_col(*span)));
                } else {
                    self.line(ind, "__builtin_unreachable();");
                }
            }
        }
    }

    fn operand(&self, f: &Func, op: &Operand) -> String {
        match op {
            Operand::Local(id) => Self::local_name(f, *id),
            Operand::Int(v, it) => c_int_of(*v, *it),
            Operand::Float(v, ft) => c_float_of(*v, *ft),
            Operand::Bool(b) => b.to_string(),
            Operand::Str(s) => format!("((aye_S_u8){{(uint8_t*){}, {}}})", c_str(s), s.len()),
            Operand::Fn {
                name,
                is_extern: true,
            } => extern_symbol(name).to_string(),
            Operand::Fn {
                name,
                is_extern: false,
            } => fn_name(name),
        }
    }

    /// The place `p` as a C lvalue; its bounds checks are `place_checks`.
    fn place(&self, f: &Func, p: &Place) -> String {
        let mut s = Self::local_name(f, p.local);
        for proj in &p.proj {
            s = self.project(f, s, proj);
        }
        s
    }

    fn project(&self, f: &Func, s: String, proj: &Proj) -> String {
        match proj {
            Proj::Field(name) => format!("{s}.{}", c_field(name)),
            Proj::Index {
                index,
                len: Len::Array(_),
                ..
            } => format!("{s}.a[{}]", self.operand(f, index)),
            Proj::Index {
                index,
                len: Len::Lanes(_),
                ..
            } => format!("{s}[{}]", self.operand(f, index)),
            Proj::Index {
                index,
                len: Len::Slice,
                ..
            } => format!("{s}.ptr[{}]", self.operand(f, index)),
            Proj::Index {
                index,
                len: Len::Unchecked,
                ..
            } => format!("{s}[{}]", self.operand(f, index)),
            Proj::Deref => format!("(*{s})"),
        }
    }

    /// The bounds checks of the place `p`, in order (a later length can be
    /// read through an earlier index).
    fn place_checks(&self, f: &Func, p: &Place, out: &mut Vec<String>) {
        if !self.checks {
            return;
        }
        let mut s = Self::local_name(f, p.local);
        for proj in &p.proj {
            if let Proj::Index { index, len, span } = proj {
                let n = match len {
                    Len::Array(n) | Len::Lanes(n) if matches!(index, Operand::Int(v, _) if (*v as u64) < *n) => {
                        None
                    }
                    Len::Array(n) | Len::Lanes(n) => Some(n.to_string()),
                    Len::Slice => Some(format!("{s}.len")),
                    Len::Unchecked => None,
                };
                if let Some(n) = n {
                    let i = self.operand(f, index);
                    out.push(format!(
                        "if (AYE_UNLIKELY((size_t){i} >= {n})) aye_panic_index({}, {i}, {n});",
                        line_col(*span)
                    ));
                }
            }
            s = self.project(f, s, proj);
        }
    }

    /// The checks of the rvalue `rv` other than overflow of `+ - *`: its
    /// places' bounds, slice bounds, division by zero, shift amounts,
    /// negation and casts.
    fn rvalue_checks(&self, f: &Func, rv: &Rvalue, out: &mut Vec<String>) {
        if !self.checks {
            return;
        }
        let panic = |cond: String, span: Span, msg: &str| {
            format!(
                "if (AYE_UNLIKELY({cond})) aye_panic({}, \"{msg}\");",
                line_col(span)
            )
        };
        match rv {
            Rvalue::Load(p)
            | Rvalue::AddrOf(p)
            | Rvalue::IsSome { place: p, .. }
            | Rvalue::IsErr(p) => self.place_checks(f, p, out),
            Rvalue::Slice {
                base,
                len,
                lo,
                hi,
                span,
                ..
            } => {
                self.place_checks(f, base, out);
                let b = self.place(f, base);
                let n = match len {
                    Len::Array(n) | Len::Lanes(n) => n.to_string(),
                    Len::Slice => format!("{b}.len"),
                    // No length: only `lo <= hi` is checked.
                    Len::Unchecked => self.operand(f, hi.as_ref().unwrap()),
                };
                let lo = self.operand(f, lo);
                let hi = hi
                    .as_ref()
                    .map(|h| self.operand(f, h))
                    .unwrap_or_else(|| n.clone());
                out.push(format!(
                    "if (AYE_UNLIKELY((size_t){lo} > (size_t){hi} || (size_t){hi} > {n})) aye_panic_slice({}, {lo}, {hi}, {n});",
                    line_col(*span)
                ));
            }
            // A constant divisor other than 0 and -1 needs no check.
            Rvalue::Binary {
                op: BinOp::Div | BinOp::Rem,
                lhs,
                rhs,
                ty: Type::Int(it),
                span,
            } => {
                if matches!(rhs, Operand::Int(v, _) if *v != 0 && *v != -1) {
                    return;
                }
                let (a, b) = (self.operand(f, lhs), self.operand(f, rhs));
                out.push(panic(format!("{b} == 0"), *span, "division by zero"));
                if it.signed() {
                    out.push(panic(
                        format!("{a} == {} && {b} == -1", c_int_of(it.min(), *it)),
                        *span,
                        "integer overflow",
                    ));
                }
            }
            Rvalue::Binary {
                op: BinOp::Shl | BinOp::Shr,
                rhs,
                ty: Type::Int(it),
                span,
                ..
            } => {
                if !matches!(rhs, Operand::Int(v, _) if *v >= 0 && *v < it.bits() as i128) {
                    let cond = format!("(uint64_t){} >= {}", self.operand(f, rhs), it.bits());
                    out.push(panic(cond, *span, "shift amount out of range"));
                }
            }
            Rvalue::Neg {
                val,
                ty: Type::Int(it),
                span,
            } if it.signed() => {
                out.push(panic(
                    format!("{} == {}", self.operand(f, val), c_int_of(it.min(), *it)),
                    *span,
                    "integer overflow",
                ));
            }
            Rvalue::Cast {
                val,
                from,
                to,
                span,
            } => {
                let v = self.operand(f, val);
                match (from, to) {
                    (Type::Int(fi), Type::Int(ti)) => {
                        let mut conds = Vec::new();
                        if fi.min() < ti.min() {
                            conds.push(format!("(int64_t){v} < {}", c_int(ti.min())));
                        }
                        if fi.max() > ti.max() {
                            let cast = if fi.signed() { "int64_t" } else { "uint64_t" };
                            conds.push(format!("({cast}){v} > {}", c_int(ti.max())));
                        }
                        if !conds.is_empty() {
                            out.push(panic(
                                conds.join(" || "),
                                *span,
                                "integer cast out of range",
                            ));
                        }
                    }
                    (Type::Float(_), Type::Int(ti)) => {
                        let lo = if ti.signed() {
                            ti.min() as f64 - 1.0
                        } else {
                            -1.0
                        };
                        let hi = ti.max() as f64 + 1.0;
                        let cond = format!(
                            "!((double){v} > {} && (double){v} < {})",
                            c_float(lo),
                            c_float(hi)
                        );
                        out.push(panic(cond, *span, "float to integer cast out of range"));
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn rvalue(&self, f: &Func, rv: &Rvalue) -> String {
        match rv {
            Rvalue::Use(op) => self.operand(f, op),
            Rvalue::Load(p) => self.place(f, p),
            // Integer arithmetic wraps (`-fwrapv`): the checks come before
            // the statement, and overflow of `+ - *` is checked as the
            // result is stored (a statement expression elsewhere).
            Rvalue::Binary {
                op,
                lhs,
                rhs,
                ty,
                span,
            } => {
                let (a, b) = (self.operand(f, lhs), self.operand(f, rhs));
                match (op, ty) {
                    (BinOp::Add | BinOp::Sub | BinOp::Mul, Type::Int(it)) if self.checks => {
                        format!(
                            "({{ {} r_; if (AYE_UNLIKELY(__builtin_{}_overflow({a}, {b}, &r_))) aye_panic({}, \"integer overflow\"); r_; }})",
                            int_c(*it),
                            overflow_builtin(*op),
                            line_col(*span)
                        )
                    }
                    (BinOp::Add | BinOp::AddWrap, Type::Int(it)) => {
                        format!("(({})({a} + {b}))", int_c(*it))
                    }
                    (BinOp::Sub | BinOp::SubWrap, Type::Int(it)) => {
                        format!("(({})({a} - {b}))", int_c(*it))
                    }
                    (BinOp::Mul | BinOp::MulWrap, Type::Int(it)) => {
                        format!("(({})({a} * {b}))", int_c(*it))
                    }
                    (BinOp::Div | BinOp::Rem, Type::Int(it)) => {
                        format!("(({})({a} {} {b}))", int_c(*it), op.as_str())
                    }
                    // The amount is masked: a left shift drops the bits
                    // shifted out, and `>>` of a signed value is arithmetic.
                    (BinOp::Shl, Type::Int(it)) => {
                        let ut = uint_c(*it);
                        format!(
                            "(({})(({ut}){a} << (({ut}){b} & {}u)))",
                            int_c(*it),
                            it.bits() - 1
                        )
                    }
                    (BinOp::Shr, Type::Int(it)) => {
                        format!(
                            "(({})({a} >> (({}){b} & {}u)))",
                            int_c(*it),
                            uint_c(*it),
                            it.bits() - 1
                        )
                    }
                    (BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor, _) => {
                        format!("(({})({a} {} {b}))", c_type(ty), op.as_str())
                    }
                    // Enums without payloads compare by tag.
                    (_, Type::Enum(_)) => format!("({a}.tag {} {b}.tag)", op.as_str()),
                    // A lane comparison gives -1 or 0 per lane, as a mask.
                    (
                        BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge,
                        Type::Simd(_, n),
                    ) => {
                        format!(
                            "__builtin_convertvector(({a} {} {b}), {})",
                            op.as_str(),
                            c_type(&Type::Simd(Box::new(Type::Bool), *n))
                        )
                    }
                    _ => format!("({a} {} {b})", op.as_str()),
                }
            }
            Rvalue::Neg { val, ty, .. } => {
                let v = self.operand(f, val);
                match ty {
                    Type::Int(it) => format!("(({})-{v})", int_c(*it)),
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
            Rvalue::Cast { val, to, .. } => format!("(({}){})", c_type(to), self.operand(f, val)),
            Rvalue::Call {
                name,
                is_extern,
                args,
            } => {
                let args: Vec<String> = args.iter().map(|a| self.operand(f, a)).collect();
                let name = if *is_extern {
                    extern_symbol(name).to_string()
                } else {
                    fn_name(name)
                };
                format!("{name}({})", args.join(", "))
            }
            Rvalue::CallValue { callee, args } => {
                let args: Vec<String> = args.iter().map(|a| self.operand(f, a)).collect();
                format!("{}({})", self.operand(f, callee), args.join(", "))
            }
            Rvalue::SizeOf(t) => format!("((size_t)sizeof({}))", c_type(t)),
            // A struct is zeroed by bytes, so its padding is zero too.
            Rvalue::Zeroed(t) if is_composite(t) => {
                format!(
                    "({{ {} z_; __builtin_memset(&z_, 0, sizeof z_); z_; }})",
                    c_type(t)
                )
            }
            Rvalue::Zeroed(t) => format!("(({})0)", c_type(t)),
            Rvalue::PtrCast { val, ty } => format!("(({}){})", c_type(ty), self.operand(f, val)),
            Rvalue::DynCall { recv, method, args } => {
                let r = self.operand(f, recv);
                let args: String = args
                    .iter()
                    .map(|a| format!(", {}", self.operand(f, a)))
                    .collect();
                format!("{r}.vt->{}({r}.ptr{args})", c_field(method))
            }
            Rvalue::Downcast {
                val,
                iface,
                strukt,
                ty,
            } => {
                let d = self.operand(f, val);
                format!(
                    "({d}.vt == &{} ? ({}){d}.ptr : NULL)",
                    vtable_name(iface, strukt),
                    c_type(ty)
                )
            }
            Rvalue::ToDyn { ptr, iface, strukt } => {
                format!(
                    "((aye_D_{}){{(void *){}, &{}}})",
                    c_name(iface),
                    self.operand(f, ptr),
                    vtable_name(iface, strukt)
                )
            }
            Rvalue::ToAllocator { ptr, strukt } => {
                format!(
                    "((aye_Allocator){{(void *){}, &aye_at_vt_{}}})",
                    self.operand(f, ptr),
                    c_name(strukt)
                )
            }
            Rvalue::Alloc { alloc, ty, span } => {
                let t = c_type(ty);
                format!(
                    "(({t} *)aye_alloc({}, sizeof({t}), _Alignof({t}), {}))",
                    self.operand(f, alloc),
                    line_col(*span)
                )
            }
            // A statement expression: allocate, then fill every element.
            // `resize` (with `old`) copies the old elements first and frees
            // the old slice.
            Rvalue::AllocSlice {
                alloc,
                value,
                elem,
                len,
                old,
                span,
            } => {
                let (t, v, a, lc) = (
                    c_type(elem),
                    self.operand(f, value),
                    self.operand(f, alloc),
                    line_col(*span),
                );
                let opt = c_type(&Type::Optional(Box::new(Type::Slice(
                    Box::new(elem.clone()),
                    true,
                ))));
                let (copy, free) = match old {
                    Some(o) => (
                        format!(
                            "__typeof__({0}) o_ = {0}; for (; i_ < n_ && i_ < o_.len; i_++) p_[i_] = o_.ptr[i_]; ",
                            self.operand(f, o)
                        ),
                        format!("aye_free_n({a}, (void *)o_.ptr, o_.len, sizeof({t}), {lc}); "),
                    ),
                    None => (String::new(), String::new()),
                };
                format!(
                    "({{ {opt} r_ = {{0}}; size_t n_ = (size_t){}; {t} *p_ = n_ ? ({t} *)aye_alloc_n({a}, n_, sizeof({t}), _Alignof({t}), {lc}) : 0; \
                     if (p_ || !n_) {{ size_t i_ = 0; {copy}for (; i_ < n_; i_++) p_[i_] = {v}; {free}r_.has = 1; r_.val.ptr = p_; r_.val.len = n_; }} r_; }})",
                    self.operand(f, len),
                )
            }
            Rvalue::EmptySlice(ty) => format!("(({}){{0, 0}})", c_type(ty)),
            Rvalue::Intrinsic { op, args, span } => self.intrinsic(f, *op, args, *span),
            Rvalue::Print { arg: None, .. } => "aye_rt_printf(\"\\n\")".to_string(),
            Rvalue::Print {
                arg: Some((Operand::Str(s), _)),
                newline,
            } => {
                format!(
                    "aye_print_s((const uint8_t*){}, {}, {newline})",
                    c_str(s),
                    s.len()
                )
            }
            Rvalue::Print {
                arg: Some((op, ty)),
                newline,
            } => {
                let v = self.operand(f, op);
                match ty {
                    Type::Int(it) if it.signed() => format!("aye_print_i({v}, {newline})"),
                    Type::Int(_) => format!("aye_print_u({v}, {newline})"),
                    Type::Float(_) => format!("aye_print_f({v}, {newline})"),
                    Type::Bool => format!("aye_print_b({v}, {newline})"),
                    _ => format!("aye_print_s(({v}).ptr, ({v}).len, {newline})"),
                }
            }
            // A statement expression: flush stdout, then write the prefix,
            // each piece and argument, and a newline to stderr.
            Rvalue::Log {
                error,
                pieces,
                args,
            } => {
                let mut s = format!(
                    "({{ aye_rt_fflush(0); aye_rt_dprintf(2, \"{}: \");",
                    if *error { "error" } else { "info" }
                );
                for (i, piece) in pieces.iter().enumerate() {
                    if !piece.is_empty() {
                        let _ = write!(
                            s,
                            " aye_log_s((const uint8_t*){}, {});",
                            c_str(piece),
                            piece.len()
                        );
                    }
                    let Some((op, ty)) = args.get(i) else {
                        continue;
                    };
                    let v = self.operand(f, op);
                    let _ = match ty {
                        Type::Int(it) if it.signed() => write!(s, " aye_log_i({v});"),
                        Type::Int(_) => write!(s, " aye_log_u({v});"),
                        Type::Float(_) => write!(s, " aye_log_f({v});"),
                        Type::Bool => write!(s, " aye_log_b({v});"),
                        Type::ErrorSet(_) => write!(s, " aye_log_e({v});"),
                        _ => write!(s, " aye_log_s(({v}).ptr, ({v}).len);"),
                    };
                }
                s.push_str(" aye_rt_dprintf(2, \"\\n\"); })");
                s
            }
            Rvalue::StructLit { name, fields } => {
                let fs: Vec<String> = fields.iter().map(|o| self.operand(f, o)).collect();
                format!(
                    "(({}){{{}}})",
                    c_type(&Type::Struct(name.clone())),
                    fs.join(", ")
                )
            }
            Rvalue::UnionLit { name, field, val } => {
                format!(
                    "(({}){{.{} = {}}})",
                    c_type(&Type::Struct(name.clone())),
                    c_field(field),
                    self.operand(f, val)
                )
            }
            Rvalue::Variant {
                enum_name,
                tag,
                variant,
                fields,
            } => {
                let t = c_type(&Type::Enum(enum_name.clone()));
                if fields.is_empty() {
                    return format!("(({t}){{.tag = {tag}}})");
                }
                let fs: Vec<String> = fields.iter().map(|o| self.operand(f, o)).collect();
                format!(
                    "(({t}){{.tag = {tag}, .u.{} = {{{}}}}})",
                    c_field(variant),
                    fs.join(", ")
                )
            }
            Rvalue::AddrOf(p) => format!("(&{})", self.place(f, p)),
            Rvalue::Some { val, ty } => {
                let v = self.operand(f, val);
                if is_ptr_optional(ty) {
                    v
                } else {
                    format!("(({}){{true, {v}}})", c_type(ty))
                }
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
                if is_ptr_optional(ty) {
                    format!("({p} != 0)")
                } else {
                    format!("{p}.has")
                }
            }
            Rvalue::ArrayRepeat {
                ty: ty @ Type::Simd(_, n),
                val,
            } => {
                let v = self.operand(f, val);
                format!("(({}){{{}}})", c_type(ty), vec![v; *n as usize].join(", "))
            }
            Rvalue::ArrayLit {
                ty: ty @ Type::Simd(..),
                elems,
            } => {
                let es: Vec<String> = elems.iter().map(|o| self.operand(f, o)).collect();
                format!("(({}){{{}}})", c_type(ty), es.join(", "))
            }
            Rvalue::ArrayRepeat { ty, val } => {
                let Type::Array(_, n) = ty else {
                    unreachable!()
                };
                let t = c_type(ty);
                format!(
                    "({{ {t} r_; for (size_t i_ = 0; i_ < {n}; i_++) r_.a[i_] = {}; r_; }})",
                    self.operand(f, val)
                )
            }
            Rvalue::ArrayLit { ty, elems } => {
                let es: Vec<String> = elems.iter().map(|o| self.operand(f, o)).collect();
                format!("(({}){{{{{}}}}})", c_type(ty), es.join(", "))
            }
            Rvalue::IsErr(p) => format!("({}.err != 0)", self.place(f, p)),
            Rvalue::Ok { val: None, ty } => format!("(({}){{0}})", c_type(ty)),
            Rvalue::Ok { val: Some(v), ty } => {
                format!("(({}){{0, {}}})", c_type(ty), self.operand(f, v))
            }
            Rvalue::Err { err, ty } => {
                format!("(({}){{.err = {}}})", c_type(ty), self.operand(f, err))
            }
            Rvalue::TraceLen => if self.trace { "aye_trace_len" } else { "0" }.to_string(),
            Rvalue::Slice {
                base,
                len,
                elem,
                lo,
                hi,
                ..
            } => {
                let b = self.place(f, base);
                let (ptr, n) = match len {
                    Len::Array(n) | Len::Lanes(n) => (format!("{b}.a"), n.to_string()),
                    Len::Slice => (format!("{b}.ptr"), format!("{b}.len")),
                    Len::Unchecked => (b.clone(), self.operand(f, hi.as_ref().unwrap())),
                };
                let lo = self.operand(f, lo);
                let hi = hi.as_ref().map(|h| self.operand(f, h)).unwrap_or(n);
                format!(
                    "(({}){{{ptr} + (size_t){lo}, (size_t){hi} - (size_t){lo}}})",
                    c_type(&Type::Slice(Box::new(elem.clone()), false))
                )
            }
        }
    }

    fn intrinsic(&self, f: &Func, op: Intrinsic, args: &[(Operand, Type)], span: Span) -> String {
        let a: Vec<String> = args.iter().map(|(o, _)| self.operand(f, o)).collect();
        let lc = line_col(span);
        match op {
            Intrinsic::GeneralInit => "aye_general_init()".into(),
            Intrinsic::GeneralAllocator => format!("aye_general_allocator({})", a[0]),
            Intrinsic::GeneralDeinit => format!("aye_general_deinit({})", a[0]),
            Intrinsic::ArenaInit => format!("aye_arena_init({})", a[0]),
            Intrinsic::ArenaAllocator => format!("aye_arena_allocator({})", a[0]),
            Intrinsic::ArenaDeinit => format!("aye_arena_deinit({}, {lc})", a[0]),
            Intrinsic::Destroy => {
                format!("aye_free({0}, (void *){1}, sizeof(*{1}), {lc})", a[0], a[1])
            }
            Intrinsic::Free => format!(
                "aye_free_n({0}, (void *)({1}).ptr, ({1}).len, sizeof(*({1}).ptr), {lc})",
                a[0], a[1]
            ),
            Intrinsic::TestAllocator => "aye_test_allocator()".into(),
            Intrinsic::Expect => format!("aye_expect({}, {lc})", a[0]),
            Intrinsic::ExpectEqual => match &args[0].1 {
                Type::Int(it) if it.signed() => {
                    format!("aye_expect_eq_i({}, {}, {lc})", a[0], a[1])
                }
                Type::Int(_) => format!("aye_expect_eq_u({}, {}, {lc})", a[0], a[1]),
                Type::Float(_) => format!("aye_expect_eq_f({}, {}, {lc})", a[0], a[1]),
                Type::Bool => format!("aye_expect_eq_b({}, {}, {lc})", a[0], a[1]),
                _ => format!(
                    "aye_expect_eq_s(({0}).ptr, ({0}).len, ({1}).ptr, ({1}).len, {lc})",
                    a[0], a[1]
                ),
            },
            Intrinsic::Hash => unreachable!("lowered to HashMix"),
            // A loop over the lanes: clang vectorizes it at `-O2`.
            Intrinsic::ReduceAdd | Intrinsic::ReduceMin | Intrinsic::ReduceMax => {
                let Type::Simd(e, n) = &args[0].1 else {
                    unreachable!()
                };
                let step = match op {
                    Intrinsic::ReduceAdd => "r_ = r_ + v_[i_]",
                    Intrinsic::ReduceMin => "r_ = v_[i_] < r_ ? v_[i_] : r_",
                    _ => "r_ = v_[i_] > r_ ? v_[i_] : r_",
                };
                format!(
                    "({{ {} v_ = {}; {} r_ = v_[0]; for (int i_ = 1; i_ < {n}; i_++) {step}; r_; }})",
                    c_type(&args[0].1),
                    a[0],
                    c_type(e)
                )
            }
            Intrinsic::All | Intrinsic::Any => {
                let Type::Simd(_, n) = &args[0].1 else {
                    unreachable!()
                };
                let (init, step) = if op == Intrinsic::All {
                    ("1", "r_ = r_ && v_[i_]")
                } else {
                    ("0", "r_ = r_ || v_[i_]")
                };
                format!(
                    "({{ {} v_ = {}; bool r_ = {init}; for (int i_ = 0; i_ < {n}; i_++) {step}; r_; }})",
                    c_type(&args[0].1),
                    a[0]
                )
            }
            Intrinsic::Select => {
                let Type::Simd(_, n) = &args[1].1 else {
                    unreachable!()
                };
                let t = c_type(&args[1].1);
                format!(
                    "({{ {} m_ = {}; {t} a_ = {}, b_ = {}, r_; for (int i_ = 0; i_ < {n}; i_++) r_[i_] = m_[i_] ? a_[i_] : b_[i_]; r_; }})",
                    c_type(&args[0].1),
                    a[0],
                    a[1],
                    a[2]
                )
            }
            Intrinsic::TagName => match &args[0].1 {
                Type::Enum(n) => {
                    let names = &self.enum_names.iter().find(|(e, _)| e == n).unwrap().1;
                    let table: Vec<String> = names
                        .iter()
                        .map(|v| format!("{{(uint8_t*){}, {}}}", c_str(v.as_bytes()), v.len()))
                        .collect();
                    format!(
                        "({{ static const aye_S_u8 n_[] = {{{}}}; n_[({}).tag]; }})",
                        table.join(", "),
                        a[0]
                    )
                }
                _ => format!(
                    "({{ const char *n_ = aye_error_names[{}]; (aye_S_u8){{(uint8_t*)n_, __builtin_strlen(n_)}}; }})",
                    a[0]
                ),
            },
            Intrinsic::HashMix => {
                let bits = match &args[1].1 {
                    Type::Float(_) => format!("aye_hash_f64({})", a[1]),
                    Type::Enum(_) => format!("(uint64_t)({}).tag", a[1]),
                    Type::Int(_) | Type::Bool | Type::ErrorSet(_) => format!("(uint64_t){}", a[1]),
                    _ => format!("(uint64_t)(uintptr_t){}", a[1]),
                };
                format!("aye_hash_mix({}, {bits})", a[0])
            }
            Intrinsic::AtomicLoad => format!("__atomic_load_n({}, __ATOMIC_SEQ_CST)", a[0]),
            Intrinsic::AtomicStore => {
                format!("__atomic_store_n({}, {}, __ATOMIC_SEQ_CST)", a[0], a[1])
            }
            Intrinsic::AtomicAdd => {
                format!("__atomic_fetch_add({}, {}, __ATOMIC_SEQ_CST)", a[0], a[1])
            }
            Intrinsic::AtomicSub => {
                format!("__atomic_fetch_sub({}, {}, __ATOMIC_SEQ_CST)", a[0], a[1])
            }
            Intrinsic::AtomicCas => format!(
                "({{ {} aye_expected = {}; __atomic_compare_exchange_n({}, &aye_expected, {}, 0, __ATOMIC_SEQ_CST, __ATOMIC_SEQ_CST); }})",
                c_type(&args[1].1),
                a[1],
                a[0],
                a[2]
            ),
        }
    }
}

/// The C initializer of a global of type `ty` with the value `v`.
fn const_init(p: &Program, v: &ConstVal, ty: &Type) -> String {
    let list = |vals: &[ConstVal], tys: &[&Type]| -> String {
        vals.iter()
            .zip(tys)
            .map(|(v, t)| const_init(p, v, t))
            .collect::<Vec<_>>()
            .join(", ")
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
        (ConstVal::Null, _) | (ConstVal::Zeroed, _) => "{0}".to_string(),
        (ConstVal::Some(x), Type::Optional(inner)) => {
            format!("{{true, {}}}", const_init(p, x, inner))
        }
        (ConstVal::Agg(elems), Type::Array(el, _)) => {
            format!("{{{{{}}}}}", list(elems, &vec![&**el; elems.len()]))
        }
        (ConstVal::Agg(elems), Type::Simd(el, _)) => {
            format!("{{{}}}", list(elems, &vec![&**el; elems.len()]))
        }
        (ConstVal::Agg(fields), Type::Struct(name)) => {
            let def = p.structs.iter().find(|s| s.name == *name).unwrap();
            format!(
                "{{{}}}",
                list(
                    fields,
                    &def.fields.iter().map(|(_, t)| t).collect::<Vec<_>>()
                )
            )
        }
        (ConstVal::Union(i, x), Type::Struct(name)) => {
            let (f, t) = &p.structs.iter().find(|s| s.name == *name).unwrap().fields[*i];
            format!("{{.{} = {}}}", c_field(f), const_init(p, x, t))
        }
        (ConstVal::Variant(tag, fields), Type::Enum(name)) => {
            let v = &p.enums.iter().find(|e| e.name == *name).unwrap().variants[*tag];
            if fields.is_empty() {
                return format!("{{.tag = {tag}}}");
            }
            let tys: Vec<&Type> = v.fields.iter().map(|(_, t)| t).collect();
            format!(
                "{{.tag = {tag}, .u.{} = {{{}}}}}",
                c_field(&v.name),
                list(fields, &tys)
            )
        }
        _ => unreachable!("constant {v:?} of type {ty}"),
    }
}

/// The types of `@sizeOf` in `b`: no value of them may exist.
fn sizeof_types(b: &Block, out: &mut Vec<Type>) {
    for s in b {
        match &s.kind {
            StmtKind::Assign(_, Rvalue::SizeOf(t)) | StmtKind::Eval(Rvalue::SizeOf(t)) => {
                out.push(t.clone())
            }
            StmtKind::If {
                then: a, els: b, ..
            }
            | StmtKind::Loop { body: a, cont: b } => {
                sizeof_types(a, out);
                sizeof_types(b, out);
            }
            StmtKind::Safety { body, .. } => sizeof_types(body, out),
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
        | Type::Simd(e, _)
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
