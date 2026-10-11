# Aye language reference (stage0)

Aye is a systems language without garbage collection: memory comes from
explicit `Allocator` values and is released with `defer`. The syntax is
Go-like (braces, `name: Type`, no semicolons), errors are values (Zig-style
error unions), and enums are tagged unions with exhaustive `match`
(Rust-style). This document describes exactly what the stage0 compiler
accepts. It is versioned with the compiler: `aye spec` prints its core and
lists the other sections, `aye spec <name>` prints one, `aye spec all` all.

Conventions: a block marked `aye` is a complete program that compiles; a
block marked `aye-error` is a program that the compiler rejects, shown to
illustrate a rule.

## 1. A first program

```aye
import std
import std.mem.Allocator

struct Counter {
    total: i64

    fn add(self: *mut Self, n: i64) {
        self.total += n
    }
}

fn sum(alloc: Allocator, n: usize) !i64 {
    let xs = try alloc.alloc(0 as i64, n)
    defer alloc.free(xs)
    for i in 0..n {
        xs[i] = i as i64 * 2
    }
    var c = Counter { total: 0 }
    for i in 0..xs.len {
        c.add(xs[i])
    }
    return c.total
}

pub fn main() !void {
    var gpa = std.heap.General.init()
    defer gpa.deinit()
    let total = try sum(gpa.allocator(), 4)
    @println(total)
}
```

`aye run first.aye` prints `12`. `gpa.deinit()` reports any memory that was
not freed.

## 2. Programs and commands

A program is a root module: one `.aye` file, or a directory whose `.aye`
files all belong to it (see section 12). The root module must declare
`pub fn main()` (no parameters; no return type or `!void`/`E!void`).

```
aye run <path> [--mode debug|safe|fast] [-- args...]  build to a temp file and run (args go to the program)
aye build <path> [-o out] [--mode m]                  build an executable (default name: the file or directory name)
    [--cflags "flags"]                                (build, run, test) more clang flags: --cflags -fsanitize=thread
    [-c]                                              (build) compile to an object file `<name>.o` without linking; any --target, no sysroot needed (a Linux target on macOS links only with a Linux sysroot)
    [--target t]                                      (build, run, test, emit-c, check) t: aarch64-macos x86_64-macos aarch64-linux x86_64-linux wasm32-wasip1 wasm32-freestanding; unknown E0117, std module the target lacks E0118; wasm32-wasip1 programs run under Node's `node:wasi`; wasm32-freestanding has no libc and no `main` is needed: `pub extern fn` is exported to JS, `main` is exported as `aye_main`, output and abort are the imports `env.aye_write(fd, ptr, len)` and `env.aye_abort()`; std also imports `env.aye_clock(id)`, `env.aye_fmt_double(ptr, cap, x, n)`, `strtod` and libm's `sin cos tan asin acos atan atan2 exp log log2 log10 pow fmod hypot` by name (`tests/wasm/loader.mjs` provides them)
aye check <path> [--json]                             report all compile errors; no C, no clang
aye fix <path>                                        apply the fix-its of `aye check` in place, repeat (at most 10 rounds)
aye test <path> [--filter text] [--json] [--time-limit s] [-j n] [--mode m]   run the `test` blocks of the root module (a directory: every module under it), each in its own process
aye emit-c <path> [--mode m]                          print the generated C
aye fmt [--check] <path>                              format in place (a directory: every .aye file under it); --check lists files that would change, exit 1
aye doc --signatures <path>                           print every pub declaration with its /// comment, no bodies; <path> may be std or std.text
aye cimport <header> [-Idir] [-DNAME[=value]] [--link lib]   print the Aye declarations that @cimport makes of a C header
aye explain <code>                                    explain an error code (E0001...) with a wrong and a fixed program
aye spec [<name> | all]                               print the core of this document and list the other sections; one section; all
```

`<path>` is a `.aye` file or a directory. A file is a module of its own:
other files in its directory are not compiled with it. Build modes: `debug`
(default; all runtime checks, error return traces, leak reports), `safe`
(optimized, all checks), `fast` (optimized, no checks). Exit status of `aye
run`: the program's status; 1 when `main` returns an error; 134 after a
panic.

`aye fmt` is the canonical layout: 4-space indentation, one space around
binary operators and after `,` and `:`, `[3]u8{1, 2}` but `Point { x: 1 }`.
It keeps every comment and every line break (they end statements), drops
extra blank lines (at most one in a row), and is idempotent. It refuses a
file that does not lex or whose brackets do not match. `aye doc
--signatures` prints a module's API (pub functions, structs with fields
and pub methods, enums, interfaces, error sets, constants) as compact
context; `aye doc --signatures std` prints all of std.

## 3. Lexical structure

- Source is UTF-8. Comments start with `//` and run to the end of the line.
  There are no block comments. `///` lines right above a declaration or
  member are its doc comment (`aye doc` shows them).
- Identifiers: a letter or `_`, then letters, digits or `_` (ASCII only).
- Keywords (31): `fn let var pub import struct enum error interface dyn
  match if else for while break continue return defer errdefer try catch
  orelse unreachable comptime where as test mut extern is`.
- Predeclared names (not keywords, cannot be declared again): `true false
  null Self type simd Integer Hash` and the primitive types `i8 i16 i32 i64 isize u8 u16 u32
  u64 usize f32 f64 bool` and C's integer types `c_short c_ushort c_int
  c_uint c_long c_ulong c_longlong c_ulonglong`. `in` is special only in a
  `for` header, `union` only after `extern`.
- Integer literals: `42`, `1_000`, `0xff`, `0o17`, `0b1010`. Float literals:
  `1.5`, `2.0e-3`, `1e9` (a float needs digits on both sides of `.`).
- Char literals `'a'`, `'\n'`, `'"'` are integers (the Unicode code point)
  and take the same escapes as strings.
- String literals `"text"` are single-line, have type `[]u8` (UTF-8 bytes,
  read-only), and are followed in memory by a NUL byte that `.len` does not
  count. Escapes: `\n \t \r \0 \\ \" \' \xHH`.
- Multi-line strings: consecutive lines that start (after indentation)
  with `\\`. Each line's text runs from after `\\` to the end of the line,
  as written (no escapes; `//` is text); the lines are joined with `\n`,
  with no newline after the last (add an empty `\\` line for one). It is a
  `[]u8` like any string. A `,` or closing bracket that follows goes on
  the next line:

```aye
let STYLE =
    \\body { margin: 8px }
    \\p { color: "red" }

pub fn main() {
    @println(STYLE)
    let lines = [_][]u8{
        \\a
        \\b
        ,
        "c",
    }
    @println(lines.len)
}
```

- Builtins start with `@`: `@print`, `@println`, `@expect`, `@expectEqual`,
  `@testAllocator`, `@sizeOf`, `@zeroed`, `@ptrCast`, `@downcast`,
  `@embedFile`, `@link`,
  `@linkOptional`, `@cimport`, `@hasLink`, `@atomicLoad`, `@atomicStore`, `@atomicAdd`,
  `@atomicSub`, `@atomicCas`, `@hash`, `@setSafety`, `@splat`,
  `@reduceAdd`, `@reduceMin`, `@reduceMax`, `@all`, `@any`, `@select`,
  `@tagName(x)` (the tag name of an error, enum or tagged-union value, a `[]u8`).

**Newlines end statements** (Go's rule). A newline is a statement
terminator when the last token on the line is an identifier, a literal,
`)`, `]`, `}`, `.*`, `return`, `break` or `continue` (after a line of a
multi-line string, only when the next line does not start with `\\`, `,`,
`)`, `]` or `}`). An explicit `;` is an error. Consequences:

- `else` must be on the same line as the `}` before it: `} else {`.
- A call, struct literal or array literal that spans lines needs a trailing
  comma before the closing `)` or `}` on its own line.
- A binary operator that continues an expression must end the line, not
  start the next one.

```aye
fn total(a: i64, b: i64, c: i64) i64 {
    return a +
        b +
        c
}

pub fn main() {
    let n = total(
        1,
        2,
        3,
    )
    if n > 5 {
        @println("big")
    } else {
        @println("small")
    }
}
```

```aye-error
pub fn main() {
    let n = 1
    if n > 0 {
        @println("positive")
    }
    else {
        @println("other")
    }
}
```

## 4. Declarations

Only declarations appear at the top level of a file, in any order: `fn`,
`extern fn`, `struct`, `extern struct`, `enum`, `error`, `interface`, `let`,
`var`, `test`, `import` and `@link`. Every declaration starts with its keyword and name, so `fn
name` finds a function's definition. `pub` makes a declaration visible to
other modules.

### Functions

```aye
fn add(a: i64, b: i64) i64 {
    return a + b
}

fn log2(x: u64) u64 {
    var n: u64 = 0
    var v = x
    while v > 1 {
        v /= 2
        n += 1
    }
    return n
}

fn hello() {
    @println("hello")
}

pub fn main() {
    hello()
    @println(add(1, 2) + log2(1024) as i64)
}
```

- Parameters are `name: Type` and immutable. The return type follows `)`;
  none means the function returns nothing. A function with a return type
  must `return` a value on every path. In a function that returns nothing
  (or `E!void`), `return f()` of a call that returns nothing calls it and
  returns.
- No overloading, default arguments, variadic Aye functions or closures.
- Functions may call each other in any order, and recurse.
- A top-level function that is not generic is a value of type `fn(A, B) R`
  (`fn(A)` without a result). Store it in a variable, field, array, global
  or optional (`?fn(A) R`, nullable), pass it, return it and call it:
  `f(x)`, `s.field(x)` (when `s` has no method of that name), `get()(x)`.
  Methods and generic functions are not values.

```aye
struct Button {
    label: []u8
    onClick: fn(i64) i64
}

fn twice(x: i64) i64 {
    return x * 2
}

fn apply(f: fn(i64) i64, x: i64) i64 {
    return f(x)
}

pub fn main() {
    let b = Button { label: "ok", onClick: twice }
    @println(b.onClick(4))
    var g: ?fn(i64) i64 = null
    g = twice
    if g |h| {
        @println(apply(h, 5))
    }
}
```

### Constants, globals and type aliases

```aye
struct Point { x: i64, y: i64 }

let MAX_ITEMS: usize = 3
let PRIMES = [3]u32{2, 3, 5}
let ORIGIN = Point { x: 0, y: MAX_ITEMS as i64 * 2 }
let MASK: u32 = (1 << 8) - 1
let Bytes = []u8
var calls: i64 = 0

fn shout(s: Bytes) {
    calls += 1
    @println(s)
}

pub fn main() {
    var sum: u32 = 0
    for p in PRIMES {
        sum += p & MASK
    }
    @println(sum)
    shout("hi")
    @println(ORIGIN.y + calls)
}
```

- `let NAME = value` at the top level is a constant, `var name = value` a
  global variable. The value is computed by the compiler: literals, other
  `let` constants, operators, `as`, and array, struct and enum literals. No
  calls and no `var` globals; overflow there is a compile error. A global
  is initialized before `main` runs, in any declaration order.
- As for a local `let`, an untyped integer gives `i64`: write `let N:
  usize = 4` for an index bound.
- `let Name = Type` (a type name, `Pool(Node)`, `[]u8`, `*Node`, ...) is a
  type alias: another name for the same type, usable in types, literals
  (`Name { ... }`) and static calls (`Name.init()`).
- Constants cannot be assigned. No local may reuse a global's name.

### Structs and methods

```aye
struct Point {
    x: f64
    y: f64

    fn origin() Point {
        return Point { x: 0.0, y: 0.0 }
    }

    fn len2(self: Self) f64 {
        return self.x * self.x + self.y * self.y
    }

    fn scale(self: *mut Self, k: f64) {
        self.x *= k
        self.y *= k
    }
}

pub fn main() {
    var p = Point { x: 3.0, y: 4.0 }
    p.scale(2.0)
    @println(p.len2())
    @println(Point.origin().x)
    @println(Point.len2(p))
}
```

- Fields are `name: Type`, separated by newlines or `,`. Functions declared
  in the struct body are its methods; inside them `Self` is the struct.
- The first parameter decides how a method is called on a value: `self:
  Self` (a copy), `self: *Self` (read-only pointer) or `self: *mut Self`
  (writable). `x.m(args)` passes `x` (or `&x`/`&mut x` automatically when
  the method takes a pointer; `&mut` needs `x` to be a `var`). A method
  without such a parameter is called as `Type.f(args)`. Any method can be
  called as `Type.f(receiver, args)`.
- A struct literal names its type and lists every field once:
  `Point { x: 1.0, y: 2.0 }`, except fields with a default value, `w: f64
  = 100.0`, which it may leave out (`Box { h: 2.0 }`, `Box {}`). A default
  is a constant like a top-level `let` (literals, constants, operators,
  `as`, `@zeroed(T)`, array, struct and enum literals; no calls: E0103).
  Enum variant fields have no defaults.
- A method cannot have the name of a field of its struct.
- A struct's methods are all in its body, so one file shows the whole
  type (what `aye doc` prints, and what a reader or an AI needs to read).
  Code in other files of the module takes the struct as a parameter
  (`fn parseSheet(sheet: *mut Sheet, src: []u8)`).

```aye
struct Box {
    x: f64 = 0.0
    w: f64 = 100.0
    h: f64
    open: [8]bool = @zeroed([8]bool)
}

pub fn main() {
    let b = Box { h: 20.0 }
    @println(b.x + b.w + b.h)
}
```

### Enums (tagged unions)

```aye
enum Shape {
    Circle { r: f64 }
    Rect { w: f64, h: f64 }
    Empty

    fn area(self: Self) f64 {
        return match self {
            .Circle { r } => 3.14159 * r * r
            .Rect { w, h } => w * h
            .Empty => 0.0
        }
    }

    fn unit() Self {
        return Self.Rect { w: 1.0, h: 1.0 }
    }
}

enum Dir { Left, Right }

pub fn main() {
    let c = Shape.Circle { r: 1.0 }
    @println(c.area())
    @println(Shape.unit().area())
    @println(Shape.area(Shape.Empty))
    let d = Dir.Left
    @println(d == Dir.Right)
}
```

A value always names the enum: `Shape.Empty`, `Shape.Circle { r: 1.0 }`
(there is no `.Circle` shorthand in expressions). A variant without payload
has no braces. Functions in the enum body are methods, as for structs;
`Self` is the enum. `==` and `!=` work only on enums whose variants all have
no fields; `match` the others.

### Error sets

`error Name { A, B }` declares a set of errors; `error All = A || B` merges
sets. See section 8.

### Interfaces

`interface Name { fn m(self: *Self, ...) T }`: see section 10.

### C functions and types

```aye
@link("m")

extern struct Vec2 {
    x: f64
    y: f64
}

extern fn puts(s: [*]u8) i32
extern fn snprintf(buf: [*]u8, size: usize, format: [*]u8, ...) i32
extern fn qsort(base: [*]i32, n: usize, size: usize, cmp: fn([*]i32, [*]i32) i32)
extern fn hypot(x: f64, y: f64) f64

fn byValue(a: [*]i32, b: [*]i32) i32 {
    return a[0] - b[0]
}

pub extern fn aye_length(v: *Vec2) f64 {
    return hypot(v.x, v.y)
}

pub fn main() {
    puts("from C".ptr)
    var xs = [3]i32{3, 1, 2}
    qsort(xs[..].ptr, xs.len, @sizeOf(i32), byValue)
    var buf = @zeroed([32]u8)
    let v = Vec2 { x: 3.0, y: 4.0 }
    let n = snprintf(buf[..].ptr, buf.len, "%d %.1f".ptr, xs[0], aye_length(&v))
    @println(buf[0..n as usize])
}
```

- `extern fn name(params) Ret` declares a C function; calls go to the C
  symbol `name`. Aye emits the declaration from this signature (no C
  headers), so it must match the library's. Modules that declare one C
  function give it the same signature.
- `extern("link") fn name(...)` calls the C symbol `link` under the Aye
  name `name` (not on a function with a body; an empty name is E0115).
- C-compatible types, the only ones allowed in an `extern fn`: integers
  (`i32` is C `int`, `usize` is `size_t`; `c_int`, `c_long`, `c_ulong`,
  ... name C's types: other names for `i32`, `i64`, `u64`, ...; `c_long`
  and `c_ulong` follow the target, 64-bit on macOS and Linux, 32-bit on
  wasm32), floats, `bool`, pointers to
  C-compatible types (`*T`, `*mut T`, `[*]T`), optional pointers (`?*T`,
  `?[*]T`, `?fn(...)`: C pointers that may be null), `extern struct`s by
  value, and `fn(A) R` of C-compatible types (a callback). C's `void *` is
  `?[*]u8`. A slice is passed as `s.ptr` and `s.len`; a string literal's
  `.ptr` is NUL-terminated.
- `...` ends the parameters of a variadic C function. The arguments after
  it must be C-compatible; C promotes them (`f32` to `double`, small
  integers to `int`), and an untyped integer literal is an `i32`, so an
  `i64` needs `%lld` and a `usize` `%zu`.
- `extern struct Name { fields }` has C layout: fields in order, with C
  alignment and padding. Fields are C-compatible types, or arrays of them.
  Declare the fields the C header has (a prefix is enough when C owns the
  value and Aye reads it through a pointer); an `extern struct` with no
  fields is an opaque C type, used behind pointers.
- `extern union Name { fields }` is a C union: the fields (C-compatible)
  share one piece of memory, a literal sets exactly one of them
  (`Event { key: k }`, E0104), and reading another field reads the same
  bytes, as in C (`SDL_Event`). It has no `==` and no field defaults.
- `pub extern fn` with a body exports an Aye function to C under its own
  name (C-compatible signature). Without `pub`, a body is an error.
- `@link("name")` at the top level links a C library: the flags of
  `pkg-config --cflags --libs name` when it knows `name`, else `-lname`;
  `@link("framework:Cocoa")` on macOS. `aye build`, `run` and `test` link
  the libraries of every module.
- `@linkOptional("name")` links the library only where it is installed
  (`pkg-config` knows it, or clang can link `-lname`), so the program
  still builds where it is missing. `@hasLink("name")` is a `bool` known
  at build time (E0100 if no `@linkOptional`/`@link` declares `name`). In
  `if @hasLink("name") { ... } else { ... }` (also with `!`, `&&`, `||`)
  both branches are checked, but only the one that runs is built; a
  function that nothing built calls is left out of the program, so code
  that calls the library may sit in helpers:

```aye
@linkOptional("libcurl")

extern fn curl_global_init(flags: i64) i32

fn start() bool {
    return curl_global_init(3) == 0
}

pub fn main() {
    if @hasLink("libcurl") {
        @println(start())
    } else {
        @println("no libcurl: fetching is unsupported")
    }
}
```

`[*]T` is an unchecked C pointer: `p[i]` reads and writes without a bounds
check, `p[lo..hi]` (both bounds) gives a `[]mut T`, and it does not track
read-only data. `?[*]T` compares with `null` and unwraps with `if p |q|`.
`@ptrCast(T, p)` converts between pointer types (`*T`, `*mut T`, `[*]T`, `?`
of one), for example `@ptrCast([*]u8, &mut state)` for a C `void *` and
back, or between function types (`@ptrCast(fn([*]u8), f)` for a C
callback); it cannot make a `*T` writable or an optional non-optional.
`@sizeOf(T)` is a `usize`; `@zeroed(T)` is the all-zero value of a number,
`bool` (`false`), optional pointer, `extern struct`, `extern union` or
array of them (`@zeroed([64]bool)`, `@zeroed([4][4]u8)`), and a constant
(usable in a global or a field default).

### C headers

`@cimport` declares a C header's functions, types and constants, read
with libclang, instead of writing `extern fn`s by hand:

```aye
@link("zlib")

let z = @cimport("zlib.h")
let m = @cimport("math.h")

pub fn main() {
    let text = "hello, hello, hello"
    var packed = @zeroed([64]u8)
    var size: z.uLongf = packed.len as z.uLongf
    if z.compress(packed[..].ptr, &mut size, text.ptr, text.len as z.uLong) == z.Z_OK {
        @println(size)
    }
    @println(m.sqrt(2.0))
}
```

- `let name = @cimport("header.h", flags...)` is a top-level declaration:
  the whole value of a `let` that is not `pub`, typed or a `var` (E0112).
  The header and the flags are string literals. `name` is a module of
  the header's declarations, visible in every file of the module:
  `z.compress`, `z.Z_OK`, `z.z_stream`.
- Flags: `-Idir` (relative to the source file's directory), `-DNAME`,
  `-DNAME=value`, `-UNAME`, `-Fdir` (macOS frameworks) and `-std=c11`.
- The header is looked for next to the source file, then on the include
  path: the `-I` flags, the `pkg-config --cflags` of the module's `@link`
  and `@linkOptional` libraries, the system's headers (the macOS SDK) and
  Homebrew's. Not found, or with errors: E0113. `@cimport` declares; the
  library is still linked with `@link`.
- What it declares (`aye cimport header.h` prints it):
  - functions: `pub extern fn` with C's types (`int` is `c_int`, `size_t`
    `usize`, `char *` and `void *` `?[*]u8`, `const T *` `?*T`, `T *`
    `?*mut T`, a function pointer `?fn(...)`), `...` when variadic;
  - structs and unions: `extern struct`/`extern union` with their fields,
    or opaque (`extern struct Name {}`, for pointers only) when the header
    does not define them or they have bit-fields, `packed` or `aligned`
    layout, or a field Aye cannot express; a tag that clashes with
    another name is `struct_name`;
  - enums: a `pub let` per constant, of the enum's integer type, and the
    enum's name as an alias of that type;
  - typedefs: aliases (`z.uLong`), also those of other headers that the
    declarations use (`pthread_t`);
  - `#define`s whose value is a number or a string: `pub let` constants
    with C's type of the value (`c_int`, `c_long`, `f64`...).

  Static and inline functions, C variables, function-like macros and
  names that are Aye keywords are left out (a comment in the generated
  source says which).
- libclang is loaded when a header is first read: next to the selected
  clang on macOS (the Command Line Tools), with `llvm-config` or in
  `/usr/lib/llvm-N` on Linux, or from `$LIBCLANG_PATH`. Missing: E0114.
  The declarations are cached in `~/.cache/aye/cimport`
  (`$AYE_CACHE_DIR/cimport`) until a file the header includes changes; a
  cached header needs no libclang.

## 5. Types

| Type | Meaning |
| --- | --- |
| `i8 i16 i32 i64 isize` | signed integers (`isize` has the target's pointer width: 64-bit, 32-bit on wasm32) |
| `u8 u16 u32 u64 usize` | unsigned integers (`usize` likewise) |
| `f32 f64` | floats |
| `bool` | `true`, `false` |
| `[N]T` | array of N elements, a value (copied on assignment) |
| `simd[N]T` | vector of N lanes (2 to 64, a power of two) of a number or `bool`; operators work per lane |
| `[]T` | slice: pointer + length; elements read-only |
| `[]mut T` | slice whose elements can be assigned |
| `*T`, `*mut T` | non-null pointer, read-only or writable |
| `?T` | optional: a `T` or `null` |
| `[*]T` | unchecked C pointer (index, slice with both bounds) |
| `fn(A, B) R` | function value (a C function pointer) |
| `E!T`, `!T` | error union, only as a function's return type |
| `dyn I` | pointer to a value implementing interface `I`, plus vtable |
| struct, enum, error set names | declared types |
| `Pool(Node)` | instance of a type function (section 11) |

There is no string type: text is `[]u8`. There is no `void` type except as
the payload of an error union (`!void`).

### simd

`simd[N]T` is the only type besides numbers with arithmetic operators (a C
vector). Two vectors of the same type combine lane by lane with `+ - *`
(integer lanes wrap, unchecked), `/` on float lanes, and unary `-`.
Comparisons give a mask `simd[N]bool`, which has no operators: test it
with `@all(m)`, `@any(m)` or `m[i]`, or use it in `@select(m, a, b)` (lane
`i` of `a` where `m[i]`, else of `b`). `v[i]` reads or writes a lane
(bounds checked; a lane has no address). `@splat(simd[N]T, x)` sets every
lane; `@reduceAdd(v)`, `@reduceMin(v)`, `@reduceMax(v)` combine the lanes.
A number is not a vector (`v * 2.0` is an error: `v * @splat(simd[4]f32,
2.0)`), and `==` cannot compare whole vectors (`@all(a == b)` does). Errors:
E0109.

```aye
fn dot(a: simd[4]f32, b: simd[4]f32) f32 {
    return @reduceAdd(a * b)
}

pub fn main() {
    var v = simd[4]f32{1.0, 2.0, 3.0, 4.0}
    v[0] = 5.0
    let big = v > @splat(simd[4]f32, 2.5)
    @println(dot(v, v))
    @println(@any(big))
    @println(@reduceMax(@select(big, v, simd[4]f32{0.0, ..})))
}
```

### Conversions

There are no implicit conversions between types. Numeric conversions are
written `x as T` (between numeric types, and from `bool` to an integer
type):

- int to int panics (debug, safe) if the value does not fit; it never
  truncates silently.
- float to int truncates toward zero and panics on NaN or out of range.
- int to float and float to float are plain conversions.
- bool to int gives `0` for `false` and `1` for `true`, for every integer
  type (and a `T` with `where T: Integer`). There is no cast to `bool`
  (compare: `n != 0`) and none from `bool` to a float (go through an
  integer: `(b as u8) as f64`); both are E0037.

`as` binds tighter than `*` and looser than unary operators: `a * b as f64`
is `a * (b as f64)`, `-x as f64` is `(-x) as f64`.

The only implicit adjustments: a `T` is accepted where `?T` is expected; a
`*mut T` where `*T` is expected; a `[]mut T` where `[]T` is expected; an
error value of one set where a set containing all its errors is expected;
a `*S` or `*mut S` where `dyn I` is expected (if struct `S` implements `I`).
One of the others may come before the wrapping (a `[]mut T` where `?[]T`
is expected, a `*mut S` where `?dyn I` is), and `mut` is also dropped
inside `?` and in a returned error union's payload (`E![]mut T` returned
from an `E![]T` function). An `E!T` is never an `E!?T`.

### Untyped literals

An integer or char literal takes the type the context expects (a typed
`let`, a parameter, the other operand of a binary operator, an index). With
no context it is `i64`, except that a range of two literals is `usize`
(§6); a float literal is `f64`. An integer literal can
become a float type. A literal that does not fit its type is a compile
error. So `let n = 5` is `i64`, `let m: u8 = 5` is `u8`, and `x + 1` has
the type of `x`.

## 6. Statements

```aye
pub fn main() {
    let a = 10
    var b: i64 = 0
    b = a * 2
    b += 1
    var i: i64 = 0
    while i < 3 {
        i += 1
        if i == 2 {
            continue
        }
    }
    for k in 0..4 {
        if k == 3 {
            break
        }
        @print(k)
    }
    @println()
    var arr = [3]i64{1, 2, 3}
    for j, x in arr {
        arr[j] = x * 10
    }
    for x in arr[1..] {
        @println(x)
    }
    {
        let inner = b
        @println(inner)
    }
}
```

- `let x = v` binds an immutable name; `var x = v` a mutable one. A type
  may be given: `var x: u8 = 0`. An initializer is always required; there
  is no zero-initialization.
- Assignment: `=`, and `+= -= *= /= %= +%= -%= *%= &= |= ^= <<= >>=`. The
  target is a `var`, a field or element of a `var`, data behind a `*mut T`,
  or an element of a `[]mut T`.
- `if cond { } else if cond { } else { }`; the condition must be `bool`
  (no truthiness). Braces are required.
- `while cond { }`. There is no `do`/`loop`; use `while true { }`.
  `while x |v| { }` runs while the optional `x` (evaluated before each
  iteration) has a value `v`.
- `for i in lo..hi { }` counts: `i` goes from `lo` up to `hi - 1`
  (half-open), both bounds have the same integer type (`0..s.len` is
  `usize`), `hi` is evaluated once, and `i` is immutable. A range of two
  literals is `usize`, so `-1..3` is an error; for a signed counter write
  `(0 as i64)..4`.
- `for x in xs { }` and `for i, x in xs { }` walk an array or a slice: `x`
  is a copy of each element, `i` its `usize` index; `xs` is evaluated once.
  To change elements, assign `xs[i]`.
- `_` discards a loop variable: `for _ in 0..3 { }`, `for _, x in xs { }`,
  `for i, _ in xs { }`.
- `break` and `continue` apply to the innermost loop.
- `return`, `return value`.
- `defer stmt` runs `stmt` when the enclosing block is left: at its end,
  or by `return` (after the return value is computed), `break` or
  `continue`. Several defers run in reverse order. The statement is a call,
  an assignment, an `if` (`defer if open { close() }`) or a block, and
  cannot contain `return` or `try`.
  `errdefer` (section 8) runs only when an error is returned.
- A block `{ ... }` opens a scope.
- An expression used as a statement must be a call, `try`, `catch` or
  `match`. A call's non-void result may be dropped, except an error union.
  `_ = x` evaluates any `x` and drops its value (not an error union).

**No shadowing anywhere**: a new name (local, parameter, `for` variable,
capture, pattern binding) cannot reuse any name visible at that point,
including names of enclosing scopes, top-level declarations of the module
(in any of its files: the error names the file) and imports. Sibling blocks may reuse a name. A capture is scoped to its
block or expression, so two `catch |e|` in one function may both use `e`.

```aye-error
fn f(x: i64) i64 {
    let x = 2
    return x
}

pub fn main() {
}
```

## 7. Expressions

Precedence, from loosest to tightest (binary operators are
left-associative; comparisons do not chain, `a < b < c` is an error):

| Level | Operators |
| --- | --- |
| 1 | `\|\|` |
| 2 | `&&` |
| 3 | `== != < <= > >=` |
| 4 | `catch`, `orelse` |
| 5 | `& \| ^` (not mixed without parentheses) |
| 6 | `<< >>` |
| 7 | `+ - +% -%` |
| 8 | `* / % *%` |
| 9 | `as` |
| 10 | prefix `- ! ~ & &mut try` |
| 11 | call `f()`, `x.f`, `x.m()`, `x[i]`, `x[a..b]`, `p.*`, `x.?` |

- Arithmetic `+ - * / %` panics on overflow and division by zero (debug and
  safe mode); `+% -% *%` wrap. Integer `/` truncates toward zero; `%` is for
  integers only. Unsigned subtraction below zero is an overflow: check
  first (`if i > 0 { i -= 1 }`). Both operands of a
  binary operator have the same type (use `as`).
- Bitwise `& | ^`, prefix `~` and shifts `<< >>` take integers; both
  operands have one type, also for a shift (`x << n` needs `n` of `x`'s
  type). Unlike C, `x & 1 == 0` is `(x & 1) == 0`. `a & b | c` is an
  error: write `(a & b) | c`. A shift by the bit width or more panics;
  `<<` drops the bits shifted out; `>>` of a signed value keeps the sign.
- In an `if` or `while` condition a `|` outside parentheses starts the
  capture `|v|`: write a bitwise or there as `if (a | b) != 0`.
- `==` and `!=` work on integers, floats, `bool`, pointers (`*T`, `[*]T`:
  the same address), enums without payloads, error values (`e ==
  FetchError.NotFound`, also between sets: the same error; E0060 when the
  set cannot hold it), structs whose fields all
  compare (field by field: `h1 == h2` for two `Handle(T)`), and optionals
  of those: `?T == ?T` (equal when both are `null` or both hold equal
  values), `?T == T` (`opt == E.A` is false for `null`), and `?T == null`.
  Arrays, slices and enums with payloads cannot be compared; compare `[]u8`
  contents with `std.mem.eql(a, b)`.
- `&&`, `||`, `!` take `bool` and short-circuit.
- Evaluation order is left to right everywhere.

### Arrays and slices

```aye
fn sum(xs: []i64) i64 {
    var t: i64 = 0
    for i in 0..xs.len {
        t += xs[i]
    }
    return t
}

pub fn main() {
    var arr = [4]i64{1, 2, 3, 4}
    arr[0] = 10
    let all = arr[..]
    all[1] = 20
    @println(sum(all))
    @println(sum(arr[2..]))
    @println(arr.len)
    let empty = []i64{}
    @println(sum(empty))
    let s = "hello"
    @println(s[1..3])
    @println(s[0])
}
```

- An array literal states its length, `[3]u8{1, 2, 3}`, or takes it from
  the elements, `[_]u8{1, 2, 3}` (only in a literal: a type needs `[3]u8`).
  `[64]bool{false, ..}` repeats one value (evaluated once) in every element:
  `[13]List(u8){List(u8).init(a), ..}`.
  A length is a constant integer: a literal, a top-level `let` constant
  (`[N]T`, `[css.N]T` for a `pub` one of another module) or arithmetic on
  them (`[N * 2]T`); never a local, a `var` or a call (E0099).
  A slice literal can only be empty: `[]T{}`. Make other slices by slicing
  an array or with `alloc.alloc` (section 9).
- Indexing `x[i]` and slicing `x[lo..hi]`, `x[lo..]`, `x[..hi]`, `x[..]`
  are bounds-checked (panic). Indices are unsigned integers (`usize`
  usually); a signed index needs `as usize`.
- Slicing a `var` array gives `[]mut T`; slicing a `let` array gives `[]T`;
  slicing a slice keeps its type. Arrays must be in a variable to be sliced.
- `.len` (type `usize`) on arrays and slices; `.ptr` (type `[*]T`) on
  slices. A `[*]T` can be sliced with both bounds, `p[0..n]`, giving an
  unchecked `[]T`.

### Pointers

`&x` makes a `*T`, `&mut x` a `*mut T` (`x` must be a `var` or reached
through a `*mut`). `p.*` dereferences. Field access and method calls go
through one pointer level automatically: `p.x`, `p.x = 1` for `p: *mut S`.
So does indexing a pointer to an array: `a[i]`, `a[i] = 1` for `a: *mut [N]T`.
`p == q` is true when both point to the same place. There is no pointer
arithmetic and no null pointer: use `?*T`. A temporary cannot be borrowed
(`&f()` is an error), but a method that takes `*Self` or `*mut Self` can
be called on one, `text.split(s, ",").next()`: it gets a pointer to a copy.

### Optionals

```aye
fn find(xs: []i64, want: i64) ?usize {
    for i in 0..xs.len {
        if xs[i] == want {
            return i
        }
    }
    return null
}

pub fn main() {
    let arr = [3]i64{5, 6, 7}
    let r = find(arr[..], 6)
    if r |i| {
        @println(i)
    } else {
        @println("missing")
    }
    if r != null {
        @println("found")
    }
    var maybe: ?i64 = null
    maybe = 3
    @println(maybe.? + 1)
}

fn sumFound(xs: []i64, wants: []i64) i64 {
    var t: i64 = 0
    for w in wants {
        let i = find(xs, w) orelse continue
        t += xs[i]
    }
    return t
}

fn indexOr(xs: []i64, want: i64) usize {
    return find(xs, want) orelse xs.len
}
```

`if x |v| { } else { }` unwraps: `v` is the value, visible in the first
block; `while x |v| { }` loops while `x` has a value. `null` needs a known
optional type (`var m: ?i64 = null`).

- `x orelse d` is the value of `x`, or `d` (a `T`, evaluated only when `x`
  is `null`). Instead of `d`: `return [value]`, `break`, `continue`, or a
  block that ends with one of them, `unreachable` or a value (`x orelse {
  log(); d }`: the last line, computed before the block's `defer`s run).
  `catch` takes the same.
  When `d` is a `?T` itself, the result is a `?T`, so `orelse` chains:
  `alt(i) orelse title(i) orelse ""` is a `[]u8`.
- `x.?` is the value of `x`; a `null` panics (`unwrap of null`). It is a
  place when `x` is: `opt.?.count += 1`.
- `unreachable` panics (`reached unreachable code`); it can stand for a
  value of any type (`_ => unreachable`, `x orelse unreachable`). A
  function may end with it instead of a `return`. In `fast` mode these
  checks are gone, and reaching them is undefined behavior.

### match

`match x { pattern => body ... }` works on enums, error values and
integers, and must be exhaustive (or end with `_`). Enum patterns: `.V`,
`.V { a, b }`, `.V { a: name }`, `.V { a, .. }` (ignore the other fields),
`_`. Without `..` every payload field must be listed. Several variants or
errors per arm, `.A, .B { .. } =>`, bind no fields. Integer patterns:
`10`, `-1`, `'a'`, inclusive ranges `'a'..='z'`, several per arm
(`' ', '\t' =>`); without `_` they must cover every value of the type.
Arms are separated by newlines or `,`.

- As an expression, each arm is an expression (not a block), and all arms
  have the same type.
- As a statement, each arm is a block or an expression statement: a call
  (or builtin), `try`, `catch`, `match` or `unreachable`
  (`.A => try self.f()`). `return` and assignments need a block:
  `.A => { return 1 }`.
- `x is .V` is `true` when the enum value `x` (or the enum that the
  pointer `x` points to) holds the variant `V`. It binds no fields (use
  `match` for them) and sits with the comparisons: `(x is .V) == b`,
  `!(x is .V)`.

```aye
enum Cmd {
    Move { dx: i64, dy: i64 }
    Say { text: []u8 }
    Quit
}

fn run(c: Cmd) {
    match c {
        .Move { dx, dy: down } => {
            @println(dx + down)
        }
        .Say { text } => @println(text)
        .Quit => @println("bye")
    }
}

fn isQuit(c: Cmd) bool {
    return match c {
        .Quit => true
        _ => false
    }
}

pub fn main() {
    run(Cmd.Move { dx: 1, dy: 2 })
    run(Cmd.Say { text: "hi" })
    @println(isQuit(Cmd.Quit))
    let c = Cmd.Say { text: "x" }
    @println(c is .Say)
}
```

```aye
fn class(c: u8) []u8 {
    return match c {
        'a'..='z', 'A'..='Z', '_' => "word"
        '0'..='9' => "digit"
        ' ', '\t', '\n' => "space"
        _ => "other"
    }
}

fn utf8Len(b: u8) usize {
    return match b {
        0x00..=0x7F => 1
        0x80..=0xBF => 0
        0xC0..=0xDF => 2
        0xE0..=0xEF => 3
        0xF0..=0xFF => 4
    }
}

pub fn main() {
    @println(class('x'))
    @println(utf8Len(0xE4))
}
```

`Name {` in an `if`, `while`, `for` or `match` header starts the body (as
in Go): put a struct literal there in parentheses.

## 8. Errors

Errors are values; there are no exceptions. A program bug is a panic.

```aye
error ParseError { Empty, BadDigit, Overflow }

pub fn parse(s: []u8) ParseError!u32 {
    if s.len == 0 {
        return ParseError.Empty
    }
    var n: u32 = 0
    for i in 0..s.len {
        let c = s[i]
        if c < '0' || c > '9' {
            return ParseError.BadDigit
        }
        if n > 400000000 {
            return ParseError.Overflow
        }
        n = n * 10 + (c - '0') as u32
    }
    return n
}

fn double(s: []u8) !u32 {
    let n = try parse(s)
    return n * 2
}

pub fn main() !void {
    @println(try double("21"))
    let a = parse("") catch 0
    @println(a)
    let b = parse("x1") catch |e| match e {
        .Empty => 1
        .BadDigit => 2
        .Overflow => 3
    }
    @println(b)
}
```

- `error Name { A, B }` declares an error set (it may span lines, one error
  per line with a trailing `,`); `error All = A || B` merges
  sets. An error value is written `Set.Name`. Errors are identified by name:
  `NotFound` in two sets is the same error.
- `E!T` is "an error of set `E`, or a `T`". `!T` lets the compiler infer the
  set from the function body; it is allowed only on functions that are not
  `pub` (and on `pub fn main() !void`). A `pub` function writes its set. A
  function with an inferred set cannot call itself (directly or through
  other inferred functions); write the set.
- Error unions exist only as return types: a call returning `E!T` must be
  handled immediately with `try`, `catch`, or `return f()` (from a function
  whose set has all its errors). It cannot be stored in a variable, passed,
  compared or dropped.
  `return f()` also needs the same payload type: in an `E!bool` function,
  `return g()` with `g` returning `E!void` is E0023; write `try g()` and
  then `return true`.
- `return Set.Name` returns an error; `return value` returns the payload.
- `try f()` gives the payload, or returns the error from the current
  function (which must return an error union whose set has the error).
- `f() catch value` gives `value` on error; `f() catch |e| expr` can use
  the error. `f() catch |e| { ... }` runs the block on error; when the
  payload is not `void` the block must end with the replacement value (an
  expression as its last line: `catch |e| { report(e); old }`) or leave
  with `return`, `break` or `continue`. A `return` there returns from the enclosing function. `catch` binds looser than `+` and tighter than `==`.
  `f() catch { ... }` (no `|e|`) is the same when the error is not needed.
- An error passed on into a larger set (by `try` or `return`) keeps its
  name, so `catch |e|` compares it with an error of any set that has it:
  `run() catch |e| { if e == LexError.Invalid { ... } }`, where `run`
  returns `All!void` and `error All = LexError || Mem`.
- `if f() |v| { A } else |e| { B }` branches on an error union without
  leaving the function: `A` gets the payload `v`, `B` the error `e` (for
  `E!void`: `if f() { A } else |e| { B }`). The `else |e|` is required
  (E0102): it handles the error, and `B` may simply end.
- `return match x { ... }` in an `E!T` function: each arm is what a
  `return` takes, a `T`, an error value or an `E!T` call (no `try` needed).
- `match e { .A => ..., .B => ... }` on an error value must cover every
  error of its set (or use `_`).
- `errdefer stmt` runs when the scope is left by returning an error (a
  `return` of an error or a `try` that passes one on), never on success.
- When `main` returns an error, the program prints `error: Name` to stderr,
  in debug mode followed by `error return trace:` and one `file:line:col`
  per `return`/`try` the error passed through (where it started first), and
  exits with status 1.
- Errors carry no payload. Report details through a `diags: *mut
  Diagnostics` parameter (`std.diag`, section 15) or `std.log`.

```aye
import std

error UrlError { Empty, NoScheme }

fn parse(s: []u8) UrlError![]u8 {
    if s.len == 0 {
        return UrlError.Empty
    }
    return s
}

fn finalUrl(asked: []u8, redirected: []u8) []u8 {
    var url = asked
    if parse(redirected) |u| {
        url = u
    } else |e| {
        std.log.info("keeping {}: {}", asked, e)
    }
    return url
}

enum Act { Open, Check, Fail }

fn run(a: Act, s: []u8) ![]u8 {
    return match a {
        .Open => finalUrl(s, "")
        .Check => parse(s)
        .Fail => UrlError.NoScheme
    }
}

pub fn main() !void {
    @println(finalUrl("a", "b"))
    @println(try run(Act.Check, "x"))
}
```

## 9. Memory

There is no garbage collector and no hidden allocation: a function that
allocates takes an `Allocator`. Free with `defer`.

```aye
import std
import std.mem.Allocator
import std.mem.AllocError

struct Node {
    value: i64
    next: ?*mut Node
}

fn push(alloc: Allocator, head: ?*mut Node, v: i64) AllocError!*mut Node {
    return alloc.create(Node { value: v, next: head })
}

pub fn main() !void {
    var gpa = std.heap.General.init()
    defer gpa.deinit()
    var arena = std.heap.Arena.init(gpa.allocator())
    defer arena.deinit()
    let a = arena.allocator()

    var head: ?*mut Node = null
    for i in (0 as i64)..3 {
        head = try push(a, head, i)
    }
    var cur = head
    while cur != null {
        if cur |n| {
            @println(n.value)
            cur = n.next
        }
    }

    let buf = try gpa.allocator().alloc(0 as u8, 16)
    defer gpa.allocator().free(buf)
    buf[0] = 'x'
    let one = try gpa.allocator().create(42 as i64)
    defer gpa.allocator().destroy(one)
}
```

- `std.heap.General.init()` is a general allocator (libc malloc). In debug
  mode `deinit()` prints `file:line:col: leak: N bytes allocated here were
  never freed` for every allocation not freed. Leaks do not change the exit
  status of a program (they fail a test). Freed memory is filled with
  `0xDD` bytes and held back (the last 32 MiB freed) before it is reused: a
  second free of it panics (`double free`), and a write to it panics (`use
  after free`, at the free) when it is reused or at `deinit`. Allocation
  and free take constant time.
- `std.heap.Arena.init(backing)` allocates from `backing` and frees
  everything at once with `deinit()`; its `destroy`/`free` do nothing.
- `x.allocator()` gives the `Allocator` of a `General` or `Arena`; it and
  `deinit()` need `x` to be a `var`.
- On an `Allocator`: `create(value)` returns `AllocError!*mut T` holding
  `value`; `destroy(p)` frees it. `alloc(value, n)` returns
  `AllocError![]mut T` with `n` copies of `value`; `free(slice)` frees it
  (pass the whole slice from `alloc`, not a shorter one: an allocator
  gets its size).
  Memory is always initialized, so you pass a value, not a type:
  `alloc.create(Node { ... })`, `alloc.alloc(0 as u8, n)`.
- `alloc.resize(s, n, fill)` returns a slice of `n` elements: the first
  `min(s.len, n)` are copied from `s`, the rest are `fill`. On success `s`
  is freed (do not use it again); on `OutOfMemory` it is unchanged. It may
  move the elements: write `s = try alloc.resize(s, n, fill)`.
- `AllocError` is `error AllocError { OutOfMemory }`.
- A struct with `fn alloc(self: *mut Self, size: usize, align: usize)
  ?[*]u8` (null: out of memory) and `fn free(self: *mut Self, p: [*]u8,
  size: usize)` is an `Allocator` too: write `&mut slab` where an
  `Allocator` is expected (E0105). `free` gets the size that was
  allocated; `align` is at most 16.

```aye
import std.heap.General
import std.mem.Allocator

struct Counting {
    backing: Allocator
    bytes: usize = 0

    fn alloc(self: *mut Self, size: usize, align: usize) ?[*]u8 {
        let s = self.backing.alloc(0 as u8, size) catch |e| {
            return null
        }
        self.bytes += size
        return s.ptr
    }

    fn free(self: *mut Self, p: [*]u8, size: usize) {
        self.bytes -= size
        self.backing.free(p[0..size])
    }
}

pub fn main() !void {
    var gpa = General.init()
    defer gpa.deinit()
    var c = Counting { backing: gpa.allocator() }
    let a: Allocator = &mut c
    let xs = try a.alloc(0 as u64, 8)
    @println(c.bytes)
    a.free(xs)
}
```
- Reads after free are not detected (they see `0xDD` bytes in debug mode).
  Prefer arenas for data with one lifetime.
- For a growable array use `std.list.List(T)`; for text, `std.text.Builder`.

## 10. Interfaces and dyn

```aye
interface Shape {
    fn area(self: *Self) f64
}

struct Square {
    side: f64

    fn area(self: *Self) f64 {
        return self.side * self.side
    }
}

fn show(s: dyn Shape) {
    @println(s.area())
}

pub fn main() {
    var sq = Square { side: 3.0 }
    show(&sq)
    let d: dyn Shape = &sq
    @println(d.area())
}
```

- An interface lists method signatures; the first parameter is `self:
  *Self` or `self: *mut Self`, and an error union names its set.
- A struct implements an interface when it has every method with the same
  parameter and return types (structurally, as in Go; no `impl`). A method
  taking `self: *mut Self` needs the interface to declare `*mut Self` and
  the `dyn` to be made from `&mut x`.
- `dyn I` is made only from a pointer, `&x` or `&mut x`, where a `dyn I` is
  expected. It does not allocate. Calls go through a vtable.
- `@downcast(*S, d)` turns the `dyn I` value `d` back into a pointer to its
  struct: a `?*S` that is `null` when `d` was made from another type (it
  compares `d`'s vtable with `S`'s for `I`). `S` must implement `I`, and
  `@downcast(*mut S, d)` needs one of `S`'s methods for `I` to take `self:
  *mut Self`, so that the `dyn` was made from `&mut x` (E0110). A library
  that keeps only a `dyn Host` lets the host get its own struct back:
  `@downcast(*mut Page, it.host.?)`.

## 11. Generics

```aye
interface Show {
    fn show(self: *Self)
}

fn max(comptime T: type, a: T, b: T, bigger: bool) T {
    if bigger {
        return a
    }
    return b
}

fn showAll(comptime T: type, items: []T) where T: Show {
    for i in 0..items.len {
        items[i].show()
    }
}

pub fn Pair(comptime T: type) type {
    return struct {
        first: T
        second: T

        pub fn swap(self: *mut Self) {
            let t = self.first
            self.first = self.second
            self.second = t
        }
    }
}

struct Num {
    v: i64

    fn show(self: *Self) {
        @println(self.v)
    }
}

pub fn main() {
    @println(max(i64, 3, 4, false))
    var p = Pair(i64) { first: 1, second: 2 }
    p.swap()
    @println(p.first)
    var nums = [2]Num{Num { v: 7 }, Num { v: 8 }}
    showAll(Num, nums[..])
}
```

- A `comptime T: type` parameter takes a type; the call passes it like an
  argument: `max(i64, a, b, true)`. A type that is not a plain name is
  written as a type: `f(*Node, p)`, `f([]u8, s)`.
- A function returning `type` is a type function; its body is exactly
  `return struct { ... }` and it has only `comptime` parameters. `Pair(i64)`
  is a type anywhere a type is written, and also in expressions before `{`
  and `.f(...)`: `Pair(i64) { ... }`, `Pool(Node).init(a)`.
- A generic is checked once where it is defined. A value of type `T` can be
  copied, moved, pointed to, stored in arrays, slices and optionals, and
  allocated; it has no fields, operators or methods except those of its
  `where` constraints. `where T: A, T: B` (no `+`) follows the return
  type. A type argument that does not satisfy a constraint is an error at
  the instantiation.
- `where T: Integer` is built in: only integer types satisfy it. A `T`
  then has the integer operators (arithmetic, bitwise, shifts,
  comparisons), integer literals take type `T`, and `as` converts between
  `T` and any numeric type (panicking if the value does not fit). Each
  instance is checked again with its type: `-x` or the literal `300` with
  `T = u8` is an error that names the instance.
- `where T: Hash` is built in: the types that `==` compares satisfy it
  (section 7), and a `T` then has `==`, `!=` and `@hash`. `@hash(x)` is a
  `u64` from any such value, equal for equal values (`0.0` and `-0.0`
  too); otherwise E0107. A slice, array, union or enum with fields does
  not hash.

```aye
fn total(comptime T: type, xs: []T) i64 where T: Integer {
    var s: i64 = 0
    for x in xs {
        s += x as i64
    }
    return s
}

fn countAbove(comptime T: type, xs: []T, limit: T) T where T: Integer {
    var n: T = 0
    for x in xs {
        if x > limit {
            n += 1
        }
    }
    return n
}

pub fn main() {
    var b = [2]u8{200, 100}
    @println(total(u8, b[..]))
    @println(countAbove(u8, b[..], 150))
}
```
- Generic functions and methods of a type function's struct must spell out
  their error sets (`E!T`). A top-level `let P = Pair(i64)` names an
  instance (a type alias, section 4).

## 12. Modules and imports

- A module is a directory; all `.aye` files directly in it share one
  namespace (a file can use the other files' declarations without
  importing them). The root module is the directory or file given to `aye`.
- `import a.b` loads the module in directory `a/b` relative to the root
  module's directory, or a std module (`std`, `std.fs`, ...). The import
  binds its last name (`b`) in the importing file only. Use items as
  `b.name`.
- `import a.b.Name` imports a single item and binds `Name`.
- After `import std`, every std module is reachable by path:
  `std.fs.open(...)`, `std.heap.General.init()`.
- There are no glob imports and no renaming: every name from another module
  traces to one import line in the file.
- Only `pub` declarations (functions, structs, enums, error sets,
  interfaces, and `pub fn` methods) can be used from another module. Fields
  and enum variants are visible wherever their type is.
- An import name cannot also be declared in the module, and locals cannot
  shadow it (`import url` and `let url = ...` is an error: rename the
  local). Modules cannot import each other in a cycle, so no module can
  import the root module (which imports all of them, directly or not):
  put what the root and a module both need in a module of its own
  (`app/shared/`) and import it from both (E0083).
- The `test` blocks of every module are checked with the program (`aye
  check`, `aye build`) and run by `aye test` (section 13); std's are not.
- Errors and types from other modules are shown with their path in
  messages: `std.fs.File`, `std.mem.AllocError`.

A directory module `app/` with `app/main.aye` and `app/util/util.aye`:

```
// app/util/util.aye
pub fn twice(x: i64) i64 {
    return 2 * x
}

// app/main.aye
import util

pub fn main() {
    @println(util.twice(21))
}
```

`aye run app` prints `42`.

## 13. Tests

```aye
fn add(a: i64, b: i64) i64 {
    return a + b
}

test "add" {
    @expectEqual(4, add(2, 2))
    @expect(add(1, 1) == 2)
}

test "allocations are checked" {
    let alloc = @testAllocator()
    let xs = try alloc.alloc(0 as u8, 8)
    defer alloc.free(xs)
    @expectEqual(8, xs.len)
}
```

- `test "name" { }` at the top level; the body is like a `!void` function
  (it can use `try`) of its module, so it can call private functions.
  `aye test dir` runs each test of every module of the program in its own
  process (a module's test shows as `test "html: name"`); `--module html`
  runs only the tests of module `html`. Give the program's directory, not
  the module's: imports start there. Every directory under `dir` with
  `.aye` files is also a root, so a module that nothing imports is tested
  too, once (shown as `test "sub.mod: name"`, the path from `dir`); a
  root that does not compile is one failed entry, `test "sub.mod" ...
  FAILED (compile)`, and the run goes on. `aye
  build`/`aye run` leave tests out. A root module with tests needs no
  `main` for `aye check` and `aye test`.
- A ```` ```aye ```` block in a `///` doc comment (outside std) is a test
  too, for `aye test` only: its lines are the body of a test of that file,
  named `doc: square (line 4)` after the declaration below the comment and
  the line of the opening fence. Errors and failures point into the
  comment. Other fences (```` ```text ````) do not run.
- `@expect(cond)` and `@expectEqual(expected, actual)` (integers, floats,
  `bool`, `[]u8`) fail the test with `file:line:col: assertion failed:
  expected X, found Y`.
- `@testAllocator()`, also written `std.testing.allocator()`, is a
  leak-checked `Allocator`, usable only in a test block. A leak, panic,
  failed assertion or returned error fails the test.

Output: `test "name" ... ok (12 ms)` or `... FAILED (12 ms)`, the wall time
of the test's process, followed by the test's indented stdout and stderr,
then `N passed; M failed; K modules; T ms` (K modules ran a test; T is the
whole run, compiling included). `--filter text` runs tests
whose name contains `text`. `--time-limit s` (default 10; 0: none) kills a
test that uses more than `s` seconds of CPU time and fails it (`FAILED (time
limit: 10.4 s CPU of 10 s)`; `"timed_out":true` in JSON); `-j n` (default:
the CPUs) runs up to `n` tests at once, printed in test order. `--json`
prints one line:
`{"file":F,"diagnostics":[...],"tests":[{"name":N,"status":"pass"|"fail",
"ms":T,"cpu_ms":T,"failures":[...],"stdout":S,"stderr":S}],"passed":P,"failed":Q}`, where a
failure is `{"kind":"assertion"|"panic"|"leak","line":L,"col":C,"message":M}`,
`{"kind":"error","error":"Name","trace":[{"line":L,"col":C}]}` or
`{"kind":"crash","message":M}`.

## 14. Builtins

- `@print(x)`, `@println(x)`, `@println()`: write an integer, float (`%g`),
  `bool` or `[]u8` to stdout (buffered C stdio). A `u8` is an integer:
  `@print(s[i])` writes `104`; write one character with `@print(s[i..i + 1])`.
- `@expect`, `@expectEqual`, `@testAllocator`: section 13.
- `@atomicLoad`, `@atomicStore`, `@atomicAdd`, `@atomicSub`, `@atomicCas`:
  section 15 (std.sync).
- `@sizeOf(T)`, `@zeroed(T)`, `@ptrCast(T, p)`, `@link("lib")`,
  `@linkOptional("lib")`, `@hasLink("lib")`, `@cimport("header.h")`:
  section 4.
- `@hash(x)`: section 11 (`where T: Hash`).
- `@downcast(*S, d)`: section 10.
- `@embedFile("path")`: the bytes of the file at `path` as a `[]u8`
  constant, read when the program is compiled (`let UA =
  @embedFile("ua.css")`, also in a function). The argument is a string
  literal, the path is relative to the directory of the source file that
  names it, and an absolute path, a missing file or a directory is E0111.
  The bytes are kept as they are (no newline is added or removed).
- `@splat`, `@reduceAdd`, `@reduceMin`, `@reduceMax`, `@all`, `@any`,
  `@select`: section 5 (simd).
- `@setSafety(false)`, `@setSafety(true)`: section 16.

## 15. std

std is built into the compiler. Import it with `import std` (then use
`std.fs...`), `import std.fs` (then `fs...`) or by item (`import
std.mem.Allocator`).

The modules. Those marked "below" are listed in full in this section,
which `aye spec std.fs` (the module's name) prints for one; for the
others, the command on their line prints every pub declaration with its
documentation (from std's source, so it is always current):

```
std.mem       Allocator, AllocError, byte slices: eql, copy, set, indexOf    below
std.heap      allocators: General (leak checks in debug), Arena              below
std.fs        files, stdin/stdout/stderr, readFile, line reader              below
std.list      List(T), a growable array                                      below
std.map       StrMap, IntMap, HashMap                                        below
std.text      bytes as text: compare, search, split, parse, UTF-8, Builder   below
std.math      f64 functions (libm), generic minOf, maxOf, absOf, clampOf     below
std.sort      sort(T, items, less), in place                                 below
std.pool      Pool(T): values in reusable slots, generational Handle(T)      aye doc --signatures std.pool
std.time      clocks, sleep, UTC DateTime, RFC 3339 and HTTP dates           aye doc --signatures std.time
std.thread    OS threads: spawn, spawnWithStack (deep recursion), join       aye doc --signatures std.thread
std.sync      Mutex, Channel(T), Atomic(T)                                   below
std.coro      coroutines: create, resume, yield                              below
std.testing   the leak-checked allocator of a test                           below
std.log       err, info: formatted lines to stderr                           below
std.diag      Diagnostics: coded messages over byte ranges, lineCol          below
std.os        args, getenv, exit                                             below
std.process   child processes: spawn (piped or inherited streams), wait, kill  aye doc --signatures std.process
std.path      paths: join, dirname, basename, extension, normalize, isUnder  aye doc --signatures std.path
std.net       TCP over IPv4/IPv6: listen, connect, Listener, Conn (HTTP: std.http)  below
std.http      HTTP/1.1 server and client (https: libcurl): listen, Server.run, get, post; parse  aye doc --signatures std.http
std.json      JSON (RFC 8259): parse into an arena tree; write, stringify    aye doc --signatures std.json
std.crypto    SHA-256, SHA-1, HMAC-SHA-256, randomBytes, constantTimeEqual   aye doc --signatures std.crypto
std.encoding  base64, base64url, hex: encode and decode (RFC 4648)           aye doc --signatures std.encoding
```

An HTTP server is `std.http`, not raw `std.net`. `http.listen(T, host,
port, ctx, handler)` gives a `Server`; `s.run(alloc)` serves, a thread per
connection, until `s.stop()`. The handler reads `req.method`,
`req.target`, `req.body` and `req.header(name)`, and fills `res.status`
(200), `res.headers` and `res.body`; its `alloc` is an arena that lives
until the response is written. The client is `http.get(arena,
"http://host:8080/path")` (also `post`, `request`, `requestWith`): a
`Response` with `status`, `headers`, `body` and `header(name)`, allocated
in the arena; redirects are followed only on request; `https://` goes
through libcurl (`HttpsUnsupported` where it is not linked).

```aye
import std.heap.General
import std.http
import std.http.Request
import std.http.Response
import std.mem.Allocator

struct App {
    name: []u8
}

fn hello(app: *mut App, req: *Request, res: *mut Response, alloc: Allocator) {
    res.body = app.name
}

pub fn main() !void {
    var gpa = General.init()
    defer gpa.deinit()
    var app = App { name: "hello\n" }
    var s = try http.listen(App, "127.0.0.1", 8080, &mut app, hello)
    try s.run(gpa.allocator())
}
```

### std.mem

```
Allocator                                  allocator interface value (section 9)
Allocator.create(value) AllocError!*mut T
Allocator.destroy(p: *T)
Allocator.alloc(value, n: usize) AllocError![]mut T
Allocator.free(s: []T)
Allocator.resize(s: []T, n: usize, fill: T) AllocError![]mut T   n elements: s's first, then fill; frees s
error AllocError { OutOfMemory }
std.mem.eql(a: []u8, b: []u8) bool         same length and bytes
std.mem.copy(dst: []mut u8, src: []u8)      copy src to the start of dst (panics if dst is shorter); may overlap
std.mem.set(dst: []mut u8, b: u8)           every byte of dst becomes b
std.mem.indexOf(s: []u8, b: u8) ?usize      the first index of the byte b
```

### std.heap

```
General.init() General                     libc allocator; leak, double-free and use-after-free reports in debug mode
General.allocator(self: *mut Self) Allocator
General.deinit(self: *mut Self)            reports leaks (debug), frees everything
Arena.init(backing: Allocator) Arena
Arena.allocator(self: *mut Self) Allocator
Arena.deinit(self: *mut Self)              frees everything the arena allocated
```

Written `std.heap.General.init()` after `import std`, or `General.init()`
after `import std.heap.General`.

### std.fs

```
error OpenError { NotFound, AccessDenied, OpenFailed }
error ReadError { ReadFailed, OutOfMemory }
error WriteError { WriteFailed }
error ReadFileError = OpenError || ReadError
error PathError { NotFound, AccessDenied, NotADirectory, Exists, NotEmpty, PathFailed }
error ReadDirError = PathError || AllocError
enum Kind { File, Dir, Symlink, Other }
struct Stat { size: u64, mtime: i64, kind: Kind }
struct DirEntry { name: []u8, kind: Kind }
struct DirEntries { items: []mut DirEntry }   (and private fields)
struct File { fd: i32 }
std.fs.open(path: []u8) OpenError!File      open a file for reading
std.fs.create(path: []u8) OpenError!File    create (or empty) a file for writing
std.fs.exists(path: []u8) bool              is there a file or directory at path?
std.fs.mtime(path: []u8) ?i64               its modification time in ns since 1970; null if stat fails
std.fs.readFile(alloc: Allocator, path: []u8) ReadFileError![]mut u8   the whole file; free with alloc.free
std.fs.stat(path: []u8) PathError!Stat                  size, mtime (ns since 1970) and kind; a symlink is not followed
std.fs.statFollow(path: []u8) PathError!Stat            like stat, but follows symlinks
std.fs.readDir(alloc: Allocator, path: []u8) ReadDirError!DirEntries   the entries, no "." or ".."; unordered; free with deinit
std.fs.mkdir(path: []u8) PathError!void                  create a directory (the parent must exist)
std.fs.remove(path: []u8) PathError!void                 delete a file, symlink or empty directory
std.fs.rename(from: []u8, to: []u8) PathError!void       rename or move; replaces an existing "to"
std.fs.stdin() File
std.fs.stdout() File
std.fs.stderr() File
File.readAll(self: Self, alloc: Allocator) ReadError![]mut u8   read to the end (empty file: empty slice); free with alloc.free
File.read(self: Self, buf: []mut u8) ReadError!usize            at most buf.len bytes; 0 at the end
File.reader(self: Self, alloc: Allocator) Reader                 buffered, for readLine
File.write(self: Self, bytes: []u8) WriteError!void              write everything (flushes @print output first)
File.close(self: Self)
struct Reader                                                    a file and a buffer from alloc
Reader.readLine(self: *mut Self) ReadError!?[]u8   the next line without \n or \r\n; null at the end; valid until the next call
Reader.deinit(self: *mut Self)
DirEntries.deinit(self: Self)                       free the entries and their names
```

`readDir` gives the entries in the order the file system does; sort them
for a stable output. A `DirEntry`'s kind is `Other` where the file system
does not say what the entry is.

`readLine` reads only what it needs, so an interactive program answers
each line as it arrives:

```aye
import std
import std.mem.Allocator

fn echo(alloc: Allocator) !void {
    var r = std.fs.stdin().reader(alloc)
    defer r.deinit()
    while try r.readLine() |line| {
        @println(line)
    }
}

pub fn main() !void {
    var gpa = std.heap.General.init()
    defer gpa.deinit()
    try echo(gpa.allocator())
}
```

### std.list

```
std.list.List(T)                           a growable array (do not use its fields)
List.init(alloc: Allocator) List(T)        empty; allocates on the first append
List.deinit(self: *mut Self)
List.len(self: *Self) usize
List.items(self: *Self) []mut T            the elements; valid until the next append/insert
List.append(self: *mut Self, value: T) AllocError!void
List.pop(self: *mut Self) ?T               remove the last; null when empty
List.insert(self: *mut Self, i: usize, value: T) AllocError!void   i <= len()
List.orderedRemove(self: *mut Self, i: usize) T                    shifts the rest down
List.swapRemove(self: *mut Self, i: usize) T                       moves the last into i
List.clear(self: *mut Self)                keeps the memory
```

Index errors panic (bounds checks). The list grows by doubling, through
`Allocator.resize`.

### std.map

```
std.map.StrMap(V)                          []u8 keys, NOT copied: they must outlive the map unchanged
StrMap.init(alloc: Allocator) StrMap(V)
StrMap.deinit(self: *mut Self)
StrMap.len(self: *Self) usize
StrMap.put(self: *mut Self, key: []u8, value: V) AllocError!void   add or replace
StrMap.get(self: *Self, key: []u8) ?V
StrMap.getPtr(self: *Self, key: []u8) ?*mut V                      valid until the next put/remove
StrMap.contains(self: *Self, key: []u8) bool
StrMap.remove(self: *mut Self, key: []u8) bool                     false when absent
StrMap.iterator(self: *Self) StrMapIterator(V)
std.map.StrMapIterator(V)
StrMapIterator.next(self: *mut Self) ?StrEntry(V)                  any order
std.map.StrEntry(V) { key: []u8, value: *mut V }
std.map.IntMap(K, V) where K: Integer      integer keys, any sign: IntMap(u32, V)
IntMap.init(alloc: Allocator) IntMap(K, V)
IntMap.deinit(self: *mut Self)
IntMap.len(self: *Self) usize
IntMap.put(self: *mut Self, key: K, value: V) AllocError!void
IntMap.get(self: *Self, key: K) ?V
IntMap.getPtr(self: *Self, key: K) ?*mut V
IntMap.contains(self: *Self, key: K) bool
IntMap.remove(self: *mut Self, key: K) bool
IntMap.iterator(self: *Self) IntMapIterator(K, V)
std.map.IntMapIterator(K, V)
IntMapIterator.next(self: *mut Self) ?IntEntry(K, V)
std.map.IntEntry(K, V) { key: K, value: *mut V }
std.map.HashMap(K, V) where K: Hash        any key `==` compares: structs, enums, optionals
HashMap.init(alloc: Allocator) HashMap(K, V)
HashMap.deinit(self: *mut Self)
HashMap.len(self: *Self) usize
HashMap.put(self: *mut Self, key: K, value: V) AllocError!void
HashMap.get(self: *Self, key: K) ?V
HashMap.getPtr(self: *Self, key: K) ?*mut V
HashMap.contains(self: *Self, key: K) bool
HashMap.remove(self: *mut Self, key: K) bool
HashMap.iterator(self: *Self) HashMapIterator(K, V)
std.map.HashMapIterator(K, V)
HashMapIterator.next(self: *mut Self) ?HashEntry(K, V)
std.map.HashEntry(K, V) { key: K, value: *mut V }
```

`HashMap` finds a key by `@hash` and `==`: a struct key (a node handle)
is compared field by field.

```aye
import std.heap.General
import std.map.IntMap
import std.map.StrMap

pub fn main() !void {
    var gpa = General.init()
    defer gpa.deinit()
    var ages = StrMap(i64).init(gpa.allocator())
    defer ages.deinit()
    try ages.put("ann", 31)
    if ages.getPtr("ann") |p| {
        p.* += 1
    }
    var it = ages.iterator()
    while it.next() |e| {
        @println(e.key)
        @println(e.value.*)
    }
    var names = IntMap(u32, []u8).init(gpa.allocator())
    defer names.deinit()
    try names.put(7, "seven")
    @println(names.contains(7))
}
```

### std.text

```
error ParseError { Invalid, Overflow }
std.text.eql(a: []u8, b: []u8) bool
std.text.eqlIgnoreCase(a: []u8, b: []u8) bool      ASCII letters only
std.text.startsWith(s: []u8, prefix: []u8) bool
std.text.endsWith(s: []u8, suffix: []u8) bool
std.text.indexOf(s: []u8, needle: []u8) ?usize     first match; an empty needle is at 0
std.text.trim(s: []u8) []u8                        without ASCII whitespace at both ends
std.text.toLower(s: []mut u8)                      ASCII, in place
std.text.split(s: []u8, sep: []u8) Split           "a,,b" by "," gives "a", "", "b"
Split.next(self: *mut Self) ?[]u8
std.text.parseInt(s: []u8) ParseError!i64          decimal, optional + or -
std.text.parseFloat(s: []u8) ParseError!f64        [+-]digits[.digits][e[+-]digits], at most 63 bytes
struct Decoded { code: u32, len: usize }
std.text.decode(s: []u8) Decoded                   first UTF-8 code point; invalid: U+FFFD, len 1; empty: len 0
std.text.encode(code: u32, out: []mut u8) usize    UTF-8 into out (needs up to 4 bytes); invalid code: U+FFFD
struct Builder                                     bytes in memory from one allocator
Builder.init(alloc: Allocator) Builder
Builder.deinit(self: *mut Self)
Builder.bytes(self: *Self) []u8                    valid until the next append
Builder.len(self: *Self) usize
Builder.clear(self: *mut Self)
Builder.append(self: *mut Self, s: []u8) AllocError!void
Builder.appendByte(self: *mut Self, b: u8) AllocError!void
Builder.appendChar(self: *mut Self, code: u32) AllocError!void    UTF-8 encoded
Builder.appendInt(self: *mut Self, v: i64) AllocError!void
Builder.appendUint(self: *mut Self, v: u64) AllocError!void
Builder.appendFloat(self: *mut Self, v: f64) AllocError!void      the text of @print (%g)
Builder.toOwned(self: *mut Self) AllocError![]mut u8              exact-size copy; free with alloc.free; builder empty after
```

Iterate split pieces with `while it.next() |piece| { }` (`it` a `var`).
Iterate code points with `decode(s[i..])` and `i += d.len`.

### std.math

```
std.math.pi                                f64 constant
std.math.sqrt(x: f64) f64                  libm, as floor, ceil, round (half away from zero) and pow
std.math.floor(x: f64) f64
std.math.ceil(x: f64) f64
std.math.round(x: f64) f64
std.math.pow(x: f64, y: f64) f64
std.math.min(a: f64, b: f64) f64
std.math.max(a: f64, b: f64) f64
std.math.abs(x: f64) f64
std.math.clamp(x: f64, lo: f64, hi: f64) f64
std.math.inf() f64                         positive infinity; -inf() is negative
std.math.nan() f64                         a NaN (not equal to itself)
std.math.isNan(x: f64) bool
std.math.isInf(x: f64) bool                either sign
std.math.trunc(x: f64) f64                 toward zero; also libm:
std.math.fmod(x: f64, y: f64) f64          remainder with the sign of x
std.math.sin(x: f64) f64                   radians, as cos, tan
std.math.cos(x: f64) f64
std.math.tan(x: f64) f64
std.math.asin(x: f64) f64                  NaN outside -1..=1, as acos
std.math.acos(x: f64) f64
std.math.atan(x: f64) f64
std.math.atan2(y: f64, x: f64) f64         angle of (x, y), -pi..=pi
std.math.exp(x: f64) f64
std.math.log(x: f64) f64                   natural; as log2, log10
std.math.log2(x: f64) f64
std.math.log10(x: f64) f64
std.math.hypot(x: f64, y: f64) f64         sqrt(x*x + y*y) without overflow
std.math.sign(x: f64) f64                  -1, 0 or 1; NaN for NaN
std.math.minOf(comptime T: type, a: T, b: T) T where T: Integer
std.math.maxOf(comptime T: type, a: T, b: T) T where T: Integer
std.math.absOf(comptime T: type, x: T) T where T: Integer        panics on the minimum of a signed T
std.math.clampOf(comptime T: type, x: T, lo: T, hi: T) T where T: Integer
```

The float functions take `f64` (convert an `f32` with `as`); the integer
ones any integer type: `std.math.minOf(i32, a, b)`.

### std.sort

```
std.sort.sort(comptime T: type, items: []mut T, less: fn(T, T) bool)   in place, by less
```

`less(a, b)` says whether `a` goes before `b`, a strict order (`less(a, a)`
is false). The sort is not stable (break ties in `less`), allocates
nothing and takes O(n log n) time: insertion sort up to 12 elements, heap
sort above. `less` is a function value (section 4): a top-level function.

```aye
import std

fn byLen(a: []u8, b: []u8) bool {
    return a.len < b.len
}

pub fn main() {
    var words = [3][]u8{"ccc", "a", "bb"}
    std.sort.sort([]u8, words[..], byLen)
    @println(words[0])
}
```

### std.sync

```
std.sync.Mutex                             a lock; copies are the same lock
Mutex.init() Mutex
Mutex.lock(self: *Self)                    wait for the lock and take it
Mutex.unlock(self: *Self)
Mutex.deinit(self: *Self)                  once, when no thread uses it
std.sync.Channel(T)                        a bounded queue between threads; share a *mut Channel(T)
Channel.init(alloc: Allocator, capacity: usize) AllocError!Channel(T)
Channel.send(self: *mut Self, value: T) ChannelError!void   waits while full; Closed after close
Channel.recv(self: *mut Self) ?T           waits while empty; null once closed and empty
Channel.close(self: *mut Self)             wake every waiting thread
Channel.deinit(self: *mut Self)
std.sync.ChannelError { Closed }
std.sync.Atomic(T)                         an integer shared without a lock (where T: Integer)
Atomic.init(value: T) Atomic(T)
Atomic.load(self: *Self) T
Atomic.store(self: *mut Self, value: T)
Atomic.add(self: *mut Self, n: T) T        returns the value before; wraps
Atomic.sub(self: *mut Self, n: T) T        returns the value before; wraps
Atomic.compareSwap(self: *mut Self, expected: T, new: T) bool   set to new if it is expected
```

Every atomic operation is sequentially consistent. They are also builtins
on a pointer to an integer: `@atomicLoad(&x)`, `@atomicStore(&mut x, v)`,
`@atomicAdd(&mut x, v)`, `@atomicSub(&mut x, v)` (the value before),
`@atomicCas(&mut x, expected, new)` (E0101 otherwise). A program fanning
work out to threads:

```aye
import std.heap.General
import std.sync.Channel
import std.thread

fn work(jobs: *mut Channel(u64)) {
    while jobs.recv() |n| {
        @println(n * n)
    }
}

pub fn main() !void {
    var gpa = General.init()
    defer gpa.deinit()
    var jobs = try Channel(u64).init(gpa.allocator(), 4)
    defer jobs.deinit()
    var ts = [2]?thread.Thread{null, null}
    for i in 0..2 {
        ts[i] = try thread.spawn(Channel(u64), work, &mut jobs)
    }
    for n in 1..5 {
        try jobs.send(n as u64)
    }
    jobs.close()
    for t in ts {
        t.?.join()
    }
}
```

The order of the lines it prints varies from run to run.

### std.coro

```
std.coro.create(comptime T: type, alloc: Allocator, stack: usize, f: fn(*mut T), arg: *mut T) AllocError!Coroutine   f(arg) on a stack of its own
Coroutine.resume(self: Self)               run it until it yields or f returns
Coroutine.isDone(self: Self) bool          has f returned?
Coroutine.deinit(self: Self)               free it (once); a suspended one is freed where it stopped
std.coro.yield()                           stop the running coroutine; its resume returns
```

A coroutine is a function that stops in the middle, at any depth of
calls, and is continued later. `create` does not run `f`; each `resume`
runs it from where it stopped until the next `yield` or until `f`
returns. Values go in and out through `arg`. Copies of a `Coroutine` are
the same coroutine. A coroutine may make and resume others; `yield` goes
back to whoever resumed the running one. A generator:

```aye
import std.coro
import std.heap.General

struct Squares {
    n: u64
    value: u64
}

fn squares(s: *mut Squares) {
    for i in 1..s.n + 1 {
        s.value = i * i
        coro.yield()
    }
}

pub fn main() !void {
    var gpa = General.init()
    defer gpa.deinit()
    var s = Squares { n: 4, value: 0 }
    let co = try coro.create(Squares, gpa.allocator(), 64 << 10, squares, &mut s)
    defer co.deinit()
    while true {
        co.resume()
        if co.isDone() {
            break
        }
        @println(s.value)
    }
}
```

- The stack has at least `stack` bytes (a multiple of 64 KiB, at least
  64 KiB), reserved, not taken, with a 1 MiB guard region below it:
  running out of it panics with `panic: stack overflow` in debug and safe
  mode. `alloc` holds the coroutine's bookkeeping, so one never freed is a
  leak in debug mode. OutOfMemory also when no address space is left.
- A coroutine runs only on the thread that made it (each thread may run
  its own). It and its resumer never run at the same time, so they share
  data without `std.sync`. Each coroutine has its own error return trace.
- `deinit` of a suspended coroutine does not run the `defer`s of its
  function; to clean up, have it return (resume it with a flag in `arg`).
- Debug and safe mode panic (no position) on `resume` of a finished,
  running (itself or one that resumed it) or freed coroutine, on `resume`
  on another thread, on `yield` outside a coroutine, and on `deinit` of a
  running one; in `fast` mode these are undefined behavior.
- Supported on aarch64 and x86_64, macOS and Linux; elsewhere `create`
  panics.

### std.testing

```
std.testing.allocator() Allocator          the test's leak-checked allocator (as @testAllocator()); test blocks only
```

### std.log

```
std.log.err(format, args...)     writes "error: <message>\n" to stderr
std.log.info(format, args...)    writes "info: <message>\n" to stderr
```

`format` must be a string literal or a string constant (`let USAGE =
"..."`, also another module's `pub let`). Each `{}` in it is replaced by
the next argument: an integer, float, `bool`, `[]u8`, or an error value
(written as its name). The number of `{}` must equal the number of arguments. Other
characters, including lone braces, are written as they are. Pending `@print`
output is flushed first.

### std.diag

```
std.diag.Diagnostic { code: u32, start: usize, end: usize, message: []u8 }   start..end: bytes of the input
std.diag.Diagnostics
Diagnostics.init(alloc: Allocator) Diagnostics
Diagnostics.deinit(self: *mut Self)                 frees the messages too
Diagnostics.add(self: *mut Self, code: u32, start: usize, end: usize, message: []u8) AllocError!void   copies message
Diagnostics.len(self: *Self) usize
Diagnostics.items(self: *Self) []Diagnostic         in order; valid until the next add/clear
Diagnostics.clear(self: *mut Self)
std.diag.LineCol { line: usize, col: usize }        from 1; col counts bytes
std.diag.lineCol(text: []u8, offset: usize) LineCol
```

The convention for detailed errors: the function takes `diags: *mut
Diagnostics`, adds one per problem (it may go on after one), and returns
its error; the caller reads `diags.items()` in the `catch` block.

```aye
import std
import std.diag
import std.diag.Diagnostics
import std.heap.General

error ParseError { Invalid }

fn parse(src: []u8, diags: *mut Diagnostics) !u32 {
    var n: u32 = 0
    for i, c in src {
        if c < '0' || c > '9' {
            try diags.add(1, i, i + 1, "not a digit")
            continue
        }
        n = n * 10 + (c - '0') as u32
    }
    if diags.len() > 0 {
        return ParseError.Invalid
    }
    return n
}

pub fn main() !void {
    var gpa = General.init()
    defer gpa.deinit()
    var diags = Diagnostics.init(gpa.allocator())
    defer diags.deinit()
    let src = "4x2"
    let n = parse(src, &mut diags) catch |e| {
        for d in diags.items() {
            let at = diag.lineCol(src, d.start)
            std.log.err("{}:{}: {}", at.line, at.col, d.message)
        }
        return e
    }
    @println(n)
}
```

### std.os

```
std.os.args(alloc: Allocator) AllocError![][]u8   the arguments; [0] is the program; free the slice with alloc.free
std.os.getenv(name: []u8) ?[]u8                   the variable's value (points into the environment; do not free); null if unset
std.os.exit(code: i32)                            exit now with this status (flushes @print output; deferred statements do not run)
```

To exit with a status and still free everything, do the work in a function
that returns the status, and call `deinit` yourself before `exit`:

```aye
import std
import std.mem.Allocator

fn run(alloc: Allocator) i32 {
    let args = std.os.args(alloc) catch |e| {
        std.log.err("out of memory")
        return 1
    }
    defer alloc.free(args)
    if args.len < 2 {
        std.log.err("usage: tool <file>")
        return 2
    }
    return 0
}

pub fn main() {
    var gpa = std.heap.General.init()
    let alloc = gpa.allocator()
    let status = run(alloc)
    gpa.deinit()
    std.os.exit(status)
}
```

### std.net

TCP over IPv4 and IPv6. A host is an IPv4 address (`0.0.0.0` to listen
on every interface), an IPv6 address (`::1`) or a name, resolved with
`getaddrinfo` (`connect` tries each address); a name that does not
resolve is UnknownHost, an empty one InvalidAddress. `listen("localhost")`
binds 127.0.0.1. Writes never raise SIGPIPE.

```
std.net.listen(host: []u8, port: u16) ListenError!Listener   port 0: the system picks one
std.net.connect(host: []u8, port: u16) ConnectError!Conn
Listener.accept(self: Self) AcceptError!Conn   waits for the next connection
Listener.port(self: Self) u16                  the port it listens on
Listener.close(self: Self)
Conn.read(self: Self, buf: []mut u8) ReadError!usize   waits; at most buf.len bytes; 0 when the peer closed
Conn.readTimeout(self: Self, buf: []mut u8, ms: u32) ReadError!usize   read, waiting at most ms (poll)
Conn.write(self: Self, bytes: []u8) WriteError!void    write everything
Conn.close(self: Self)
error ListenError { InvalidAddress, AddressInUse, AccessDenied, UnknownHost, ListenFailed }
error ConnectError { InvalidAddress, UnknownHost, ConnectionRefused, Unreachable, TimedOut, ConnectFailed }
error AcceptError { AcceptFailed }
error ReadError { TimedOut, ConnectionReset, ReadFailed }
error WriteError { ConnectionClosed, WriteFailed }
```

A server listens on port 0, serves on a `std.thread`, and its tests
connect to `Listener.port()` (`tests/cases/net_echo.aye`, `net_names.aye`).

### A complete CLI

```aye
import std
import std.mem.Allocator

fn countWords(text: []u8) usize {
    var words: usize = 0
    var inWord = false
    for i in 0..text.len {
        let c = text[i]
        if c == ' ' || c == '\n' || c == '\t' {
            inWord = false
        } else if !inWord {
            inWord = true
            words += 1
        }
    }
    return words
}

fn readInput(alloc: Allocator, args: [][]u8) ![]mut u8 {
    if args.len > 1 {
        let f = try std.fs.open(args[1])
        defer f.close()
        return f.readAll(alloc)
    }
    return std.fs.stdin().readAll(alloc)
}

pub fn main() !void {
    var gpa = std.heap.General.init()
    defer gpa.deinit()
    let alloc = gpa.allocator()
    let args = try std.os.args(alloc)
    defer alloc.free(args)
    let text = readInput(alloc, args) catch |e| {
        std.log.err("cannot read input: {}", e)
        return e
    }
    defer alloc.free(text)
    @print(countWords(text))
    @print(" ")
    @println(text.len)
}
```

## 16. Runtime behavior

- A failed check panics: it flushes stdout, writes `file:line:col: panic:
  <reason>` to stderr and aborts (status 134). Reasons: `integer overflow`,
  `division by zero`, `index out of bounds: index I, len N`, `slice bounds
  out of range`, `integer cast out of range`, `shift amount out of range`,
  `unwrap of null`, `reached unreachable code`, `double free ...`, `use
  after free ...`.
- Running out of stack (deep recursion) panics with `panic: stack
  overflow` (no position) in debug and safe mode, on any thread; in `fast`
  mode it is a crash. A thread's stack is 8 MiB unless `spawnWithStack`
  asks for more; a coroutine's is what `std.coro.create` asks for.
- Misuse of `std.coro` panics with `panic: std.coro: <reason>` (no
  position) in debug and safe mode: `yield outside a coroutine`, `resume
  of a coroutine that has finished` (or `is running`, `was freed`, `on
  another thread than the one that made it`).
- `fast` mode removes the checks: overflow wraps, out-of-bounds access is
  undefined behavior.
- `@setSafety(false)`, as the first statement of a block (a function body
  too), removes the checks of that block and the blocks in it, as `fast`
  mode does; `@setSafety(true)` in an inner block brings back the build
  mode's checks. The argument is `true` or `false` written out (E0108).
  Functions called from the block keep their checks.
- Struct fields, arrays and slices are laid out by the compiler; only an
  `extern struct` promises C layout.

## 17. Diagnostics

Errors print as `file:line:col: error[E0023]: message`, sorted by file and
position, all of them (up to 50): the compiler recovers from errors. If
there are syntax or import errors, type checking does not run. Every error
has a stable code; `aye explain E0023` shows a wrong and a fixed program.

Some errors have a fix-it, printed as `file:line:col: help: <what it does>`
and applied by `aye fix`: `let` to `var` when a `let` is assigned; joining
`}` and `else`; a missing trailing `,` before a closing `)`/`}` on the next
line; renaming an unknown name to the only visible name one edit away;
adding `import std.mem.Allocator` (and `AllocError`, `std.heap.General`,
`std.heap.Arena`) when those names are used without an import.

`aye check --json` prints one line:

```
{"file":"f.aye","diagnostics":[{"severity":"error","code":"E0032","file":"f.aye","line":4,"col":9,
  "message":"cannot assign to `sum`: it is immutable (declare it with `var`)",
  "fixes":[{"line":2,"col":5,"end_line":2,"end_col":8,"replacement":"var"}]}]}
```

`diagnostics` is `[]` when the program compiles. Lines and columns are
1-based; a column counts characters. Each fix-it edit replaces the text of
the diagnostic's `file` from `line:col` up to (not including)
`end_line:end_col` with `replacement`; an insertion has equal start and
end. The edits of one diagnostic are applied together.

## 18. Pitfalls (differences from Go, Rust and Zig)

- No semicolons; `} else {` on one line; trailing `,` in multi-line calls
  and literals; an operator that continues a line ends it.
- No shadowing at all, unlike Rust (`let x = x + 1` is an error) and Go
  (`:=` in an inner scope). Pick a new name.
- `let` is immutable (Rust's `let`, not JavaScript's). Use `var` to assign,
  to take `&mut`, or to call a `*mut Self` method on a value.
- Every `var` needs an initializer: `var n: usize = 0`. No zero values.
- No implicit integer widening: `a + b` needs equal types; convert with
  `as`. Indices must be unsigned (`usize`); `.len` is `usize`.
- `for i in 0..4` gives a `usize` (as in Zig; Go's `range 4` gives
  `int`); write `(0 as i64)..4` for a signed counter.
- `for x in xs` and `for i, x in xs` (index first, as in Go) give copies
  of the elements. No `for (xs) |x|`, no C-style `for`.
- String literals are `[]u8` and read-only; there is no `+` on strings
  (build text with `std.text.Builder`), and `==` does not compare slices
  (`std.text.eql`).
- Char literals are integers: `'a'` compares with a `u8` directly, and
  `@print` of a `u8` writes its number (print a one-byte slice instead).
- `match` is for enums, errors and integers (characters too); no
  `switch`, no matching on strings, no `..` (exclusive) range patterns.
- `&`, `|` and `^` bind tighter than `==` (unlike C) and cannot be mixed
  without parentheses; a shift amount has the type of the shifted value.
- `if` is a statement, not an expression: `let x = if c { 1 } else { 2 }`
  is an error. Declare `var x = 2` and assign in the `if`.
- A function with a return type needs a `return` at the end of every path;
  a `while true { }` loop does not count, so add a final `return` after it.
- Enum values are always qualified: `Color.Red`, never `.Red` (patterns use
  `.Red`). Enums with payloads, arrays and slices have no `==`.
- An error union cannot be stored: `let r = f()` for `f() E!T` is an error;
  write `try f()` or `f() catch ...`. `try` needs the enclosing function to
  return an error union.
- `pub` functions spell their error set: `pub fn f() E!T`, not `!T`.
- `alloc.create(value)` and `alloc.alloc(value, n)` take values, not types
  (`alloc.alloc(0 as u8, n)`, not `alloc.alloc(u8, n)`).
- Pointers are never null; optional pointers are `?*T`. Dereference is
  `p.*` (Zig), not `*p`.
- `dyn I` is made from `&x`, never from a value; it does not box.
- A pointer or slice into a `List` or `Pool` (`list.items()`,
  `pool.get(h).?`) dangles after the next `append`, `insert` or `resize`,
  which may move the elements; nothing detects it. Copy the fields you
  need, or fetch the pointer again after growing; keep indices or
  `Handle(T)`s across calls that grow.
- No `@import`, no `const` (a top-level `let` is a constant), no closures
  (a function value cannot capture locals), no `defer` with `return`
  inside.
- C's `void *` is `?[*]u8`; pass `&mut x` where C wants a pointer, and
  `@ptrCast` it when C wants a `void *`.
- `@print`/`@println` take one value: `@println(a, b)` is an error. Print
  each value with its own `@print`, or build the line with `std.text.Builder`.
- Logic is `&&`, `||`, `!`; `and`, `or` and `not` are not keywords.
- Output to stdout is `@print`/`@println` or `std.fs.stdout().write(bytes)`;
  `std.log` writes to stderr.
- `main` is `pub fn main()` or `pub fn main() !void`; arguments come from
  `std.os.args(alloc)`.
- A file argument compiles only that file; give the directory to compile a
  multi-file module.
