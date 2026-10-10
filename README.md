# Aye

Aye is a systems programming language: no garbage collector, no borrow
checker, no exceptions, no macros. Memory is managed through explicit
`Allocator` parameters and `defer`/`errdefer`. The syntax is Go-like (braces,
`name: Type`, no semicolons). Integer overflow and out-of-bounds access trap in
checked builds.

Design doc: [`docs/design.md`](docs/design.md)

This repository holds **stage0**: a compiler written in Rust that emits C and
builds it with clang. It implements a first slice of the language (see
[Supported subset](#supported-subset)).

## Build and run

Requirements: Rust (cargo), and `clang` on `PATH`. Developed on macOS arm64
with Apple clang 17. Programs that `@link` a C library find it through
`pkg-config` when it knows the name; the tests in `tests/ffi` use libcurl and
SDL2 (`brew install sdl2`) and are skipped when `pkg-config` lacks them.
`@cimport` reads C headers with libclang, which the compiler loads only
when it needs it (it comes with the Command Line Tools on macOS; on Linux,
the clang or `libclang` packages); the tests in `tests/cimport` are skipped
without it. On Ubuntu 24.04, `sh scripts/setup-ubuntu.sh` installs all of
this; on a Mac, `scripts/linux.sh <command>` runs a command in an Ubuntu
24.04 container (see `docs/practices/workflow.md`, "Test on Linux").

```sh
cargo build --release
./target/release/aye run tests/cases/hello.aye
```

Commands (`<path>` is a `.aye` file, or a directory: a module of all its
`.aye` files; see [Modules](#modules-and-std)):

```
aye build <path> [-o out] [--mode debug|safe|fast]    # executable (default name: file stem or directory name)
aye run <path> [--mode debug|safe|fast] [-- args...]  # build to a temp file and run it with args
aye emit-c <path> [--mode debug|safe|fast]            # print the generated C
aye check <path> [--json]                             # front end only: report errors, no C, no clang
aye fix <path>                                        # apply the fix-its of `aye check` in place
aye test <path> [--filter <text>] [--module <m>] [--json] [--time-limit <s>] [-j <n>] [--mode debug|safe|fast]   # run the `test` blocks of every module
aye fmt [--check] <path>                              # format in place (a directory: every .aye file under it)
aye doc --signatures <path>                           # every pub declaration with its /// comment (also `std`, `std.text`)
aye cimport <header> [-Idir] [-DNAME[=value]] [--link lib]   # print the Aye declarations `@cimport` makes of a C header
aye explain <code>                                    # explain an error code, with a wrong and a fixed example
aye spec [<section> | all]                            # print the core of the language reference (docs/spec.md), one section, or all
```

`build`, `run` and `test` take `--cflags "flags"` for more clang flags, such
as `--cflags -fsanitize=thread`. `--target <t>` (`aarch64-macos`,
`x86_64-macos`, `aarch64-linux`, `x86_64-linux`, `wasm32-wasip1`,
`wasm32-freestanding`; default: this machine) builds for another target.

`aye fmt` works on tokens and keeps every comment and line break; it sets
indentation (4 spaces) and spacing, keeps at most one blank line in a row and
lines up runs of trailing comments. It is idempotent and never changes the
tokens (so `aye check` reports the same errors, on the same lines unless a
blank line went away). `--check` writes nothing, lists the files that would
change and exits 1 if there is one. A file that does not lex or whose brackets
do not match is reported and left alone. `aye doc --signatures` prints the pub
API of a file, a directory module and its submodules, or the embedded std:
signatures without bodies, fields and variants, pub methods, and each
declaration's `///` doc comment, grouped by module. It is meant as context for
an AI agent next to `aye spec`.

`aye spec` prints the core of [`docs/spec.md`](docs/spec.md), a
self-contained language reference meant to be given to an AI agent before
it writes Aye: the first program, the std index and the pitfalls first,
then syntax, types, statements, expressions and errors, at most 28000
bytes so a coding agent sees it inline, then a list of the other sections
(with a hint of when to read the ones left out, such as `9 memory` and
`12 modules`). `aye spec <section>` prints one (`aye spec 11`,
`aye spec generics`, `aye spec std.fs`), `aye spec all` the whole
document. Every ` ```aye ` program in it is checked by `tests/spec.rs`.

| Mode    | clang flags | Runtime checks (overflow, bounds, casts, div by zero) |
|---------|-------------|--------------------------------------------------------|
| `debug` | `-O0 -g`    | on (default)                                           |
| `safe`  | `-O2 -g`    | on                                                     |
| `fast`  | `-O2`       | off                                                    |

`@setSafety(false)` as the first statement of a block turns its checks off in
any mode, for a hot loop that has been measured. All modes compile with
`-std=gnu11 -fwrapv -fno-strict-aliasing`. Checks are
generated by Aye, not left to C semantics. A failed check prints
`file:line:col: panic: <reason>` to stderr and aborts. When `main` returns an
error, the program prints `error: <Name>` to stderr and exits with status 1; in
debug mode an error return trace follows, one `file:line:col` per `return` and
`try` that the error passed through.

Compile errors are reported as `file:line:col: error[E0023]: <message>`. Every
error has a stable code; `aye explain E0023` prints what it means, a program
with the error and the fixed program (the table is `src/codes.rs`). The
compiler recovers from errors and reports all of them (up to 50), sorted by
position.

### Fix-its

Some errors come with a fix-it: an edit that is certain to fix them. The
error line is then followed by `file:line:col: help: <what the fix does>`,
at the position of the edit:

```
f.aye:4:9: error[E0032]: cannot assign to `sum`: it is immutable (declare it with `var`)
f.aye:2:5: help: declare `sum` with `var`
```

| Error | Fix-it |
|-------|--------|
| E0032: assignment to (or `&mut` of) a `let` binding | `let` becomes `var` |
| E0008: `else` on the line after the `}` of an `if` | join the lines: `} else {` (only when nothing but the line break is between them) |
| E0006: a multi-line call, struct literal or array literal without a trailing `,` | add the `,` (only when the closing `)` or `}` is on the next line) |
| E0017, E0018, E0019, E0038: unknown type, variable, function or field | rename it to the one name in scope that is one edit away (a character inserted, deleted or replaced, or two adjacent characters swapped); not for names shorter than 3 characters |
| E0084: `Allocator`, `AllocError`, `General` or `Arena` used without an import | add `import std.mem.Allocator` (or `std.mem.AllocError`, `std.heap.General`, `std.heap.Arena`) after the file's last import, or at its top |

`aye fix <path>` applies every fix-it to the files in place (never to std), checks again,
and repeats while there is something to fix (at most 10 rounds), because a
syntax fix can reveal type errors. A fix whose edits overlap an edit applied
before it in the same round waits for the next round. Each applied fix is
printed to stdout as `file:line:col: fixed: <what it did>`; the errors that
remain go to stderr as `aye check` prints them. The exit status is 0 when the
file compiles afterwards and 1 otherwise.

### `aye check --json`

`aye check --json` prints one line to stdout (wrapped here):

```
{"file":"f.aye","diagnostics":[{"severity":"error","code":"E0032","file":"f.aye","line":4,"col":9,
  "message":"cannot assign to `sum`: it is immutable (declare it with `var`)",
  "fixes":[{"line":2,"col":5,"end_line":2,"end_col":8,"replacement":"var"}]}]}
```

- `diagnostics` is `[]` when the program compiles.
- Each diagnostic's `file` is the file of the error (the top-level `file`
  is the path given to `aye check`); its fix-it edits only that file.
- `line` and `col` are 1-based; a column counts characters (Unicode scalar
  values) from the start of the line, and a tab counts as one.
- `fixes` lists the edits of the error's fix-it, applied together (`[]` when
  it has none). Each edit replaces the text from `line:col` up to, not
  including, `end_line:end_col` with `replacement` (positions in the file as
  it is now). An insertion has the same start and end; a deletion has an
  empty `replacement`. The edits of one error never overlap.

## Test blocks (`aye test`)

`test "name" { ... }` blocks sit next to the code. `aye build` and `aye run`
leave them out; `aye test` runs each one of every module of the program
(std aside; only those whose name contains the `--filter` text, and with
`--module html` only those of module `html`) in its own process, so a panic
ends only that test. A test of another module than the root shows as
`test "html: name"`. With a
directory, `aye test dir` also runs the modules under it that nothing
imports, each module once. A test
fails when it panics, fails an assertion, returns an error, or leaks memory
from `@testAllocator()`:

```aye
struct Node { value: i64, next: ?*mut Node }

test "leaks a node" {
    let alloc = @testAllocator()
    let n = try alloc.create(Node { value: 2, next: null })
    @expectEqual(2, n.value)
}
```

```
$ aye test list.aye
test "leaks a node" ... FAILED (3 ms)
    list.aye:5:23: leak: 16 bytes allocated here were never freed

0 passed; 1 failed; 1 modules; 212 ms
```

Each test line ends with the wall time of its process; the last line counts the
modules that ran a test and the time of the whole run.

`--time-limit <seconds>` (default 10; 0: none) limits the CPU time of each
test: a test over it is killed and fails with `FAILED (time limit: 10.4 s CPU
of 10 s)`; `-j <n>` (default: the number of CPUs) runs up to `n` tests at
once, and the output stays in test order.

A ```` ```aye ```` block in a `///` doc comment is a test body too (`test
"doc: square (line 4)"`), so the examples in the documentation keep working.

A failed test's stdout and then its stderr follow its line, indented. The
reasons on stderr are `file:L:C: assertion failed: expected 4, found 5` (or
`condition is false`), `file:L:C: panic: <reason>`, `file:L:C: leak: ...`,
and `error: <Name>` with the error return trace (debug mode). The exit
status is 1 if a test failed or the file does not compile, and 0 otherwise.

`aye test --json` prints one line to stdout (wrapped here):

```
{"file":"list.aye","diagnostics":[],
 "tests":[{"name":"leaks a node","status":"fail","ms":3,"cpu_ms":2,
           "failures":[{"kind":"leak","line":5,"col":23,"message":"16 bytes allocated here were never freed"}],
           "stdout":"","stderr":"list.aye:5:23: leak: 16 bytes allocated here were never freed\n"}],
 "passed":0,"failed":1}
```

- `diagnostics`: the compile errors, as in `aye check --json`; then `tests` is
  empty.
- `status`: `"pass"` or `"fail"`; `ms` and `cpu_ms`: the wall and CPU time
  of the test's process; `"timed_out":true` is there when the time limit
  killed it.
- `failures`, in the order they happened:
  - `{"kind":"assertion"|"panic"|"leak","line":L,"col":C,"message":"..."}`
  - `{"kind":"error","error":"Name","trace":[{"line":L,"col":C},...]}`: the
    test returned an error; `trace` is empty outside debug mode.
  - `{"kind":"crash","message":"terminated by signal 11"}`: the test ended
    in another way.
- `stdout`, `stderr`: the test's output.

## Generics

A function with `comptime T: type` parameters is generic; a function that
returns `type` is a type function, whose body is `return struct { ... }`:

```aye
interface Shape {
    fn area(self: *Self) f64
}

fn totalArea(comptime T: type, items: []T) f64 where T: Shape {
    var sum = 0.0
    for i in 0..items.len {
        sum += items[i].area()
    }
    return sum
}

pub fn Pair(comptime T: type) type {
    return struct {
        first: T
        second: T

        fn swap(self: *mut Self) {
            let t = self.first
            self.first = self.second
            self.second = t
        }
    }
}
```

`totalArea(Square, squares)` passes the type like any argument, and
`Pair(i64)` is a type: `Pair(i64) { first: 1, second: 2 }`,
`Pool(Node).init(alloc, 16)`. The same arguments give the same type. A
generic is checked once, where it is defined: a value of type `T` can be
moved, copied and pointed to, and its methods are those of its `where`
constraints. The built-in constraint `where T: Integer` admits only
integer types and gives a `T` the integer operators, literals and `as`
(each instance is checked again with its type); `where T: Hash` admits
the types `==` compares and gives a `T` `==` and `@hash(x)`. A type argument that does
not satisfy a constraint is an error at the instantiation. The generated C
has one copy per instance.
[`tests/cases/pool.aye`](tests/cases/pool.aye) is the design doc's
`Pool(T)` and `Handle(T)`.

## Modules and std

A module is a directory: all `.aye` files in it share one namespace, and
only `pub` items (and `pub fn` methods) are visible to other modules.
`aye <cmd> dir` compiles the directory module `dir`; `aye <cmd> file.aye`
compiles that file alone as a module (its sibling files are not part of
it). `import a.b` loads the module in the directory `a/b` under the root
module's directory, and binds `b` in the importing file; `import a.b.Name`
imports one item. `std` is embedded in the compiler (sources in
[`std/`](std)): `std.fs` (`open`, `create`, `exists`, `readFile`,
`stdin`/`stdout`/`stderr`, `File.readAll`/`read`/`write`/`close`,
`File.reader` and `Reader.readLine`), `std.mem`
(`Allocator`, `AllocError`, `eql`, `copy`, `set`, `indexOf`), `std.heap`
(`General`, `Arena`), `std.list` (`List(T)`), `std.map` (`StrMap(V)`,
`IntMap(K, V)`, `HashMap(K, V)`), `std.text` (`Builder`, search, `split`, parsing, UTF-8),
`std.math`, `std.sort` (`sort(T, items, less)`), `std.pool` (`Pool(T)`,
`Handle(T)`), `std.time`, `std.thread` (`spawn`, `spawnWithStack`,
`Thread.join`), `std.sync`
(`Mutex`, `Channel(T)`, `Atomic(T)`), `std.coro` (stackful coroutines:
`create`, `Coroutine.resume`, `yield`, `isDone`, `deinit`),
`std.testing`, `std.log` (`err`, `info`), `std.diag` (`Diagnostics`, the
`*mut Diagnostics` convention for detailed errors) and `std.os` (`args`,
`exit`).
`aye doc --signatures std` lists every function. Cyclic imports are an error, so no module
imports the root module: code that both need goes into a module of its own.

```aye
import std
import std.mem.Allocator

fn readConfig(alloc: Allocator, path: []u8) ![]mut u8 {
    let file = try std.fs.open(path)
    defer file.close()
    return file.readAll(alloc)
}

pub fn main() !void {
    var gpa = std.heap.General.init()
    defer gpa.deinit()
    let bytes = readConfig(gpa.allocator(), "app.toml") catch |e| {
        std.log.err("config: {}", e)
        return e
    }
    defer gpa.allocator().free(bytes)
}
```

[`tests/example1/app`](tests/example1/app) is the design doc's Example 1
(`readConfig`) as a directory module.

## Example

```aye
extern fn puts(s: [*]u8) i32

struct Point { x: i32, y: i32 }

fn dot(a: Point, b: Point) i32 {
    return a.x * b.x + a.y * b.y
}

pub fn main() {
    defer @println("bye")
    puts("hello".ptr)
    let p = Point { x: 1, y: 2 }
    @println(dot(p, p))
    var xs = [4]i64{1, 2, 3, 4}
    for i in 0..xs.len {
        xs[i] *= 10
    }
    let tail = xs[2..]
    @println(tail[0] + tail[1])
}
```

## Supported subset

- Top level: `fn`, `pub fn`, `extern fn` (C functions), `struct Name { ... }`,
  fields with default values (`w: f64 = 100.0`),
  `extern struct Name { ... }` (C layout), `extern union Name { ... }`, `pub extern fn` with a body
  (exported to C), `@link("lib")`,
  `enum Name { A, B { field: T } }` (tagged union, with methods), `error Name { A, B }` and
  `error All = A || B` (error sets), `interface Name { ... }`, `test "name" { }`,
  `import a.b` and `import a.b.Name`, constants `let NAME = value` and globals
  `var name = value` (computed at compile time), type aliases `let Name = Type`.
- Modules: directory modules, `pub` visibility across modules, paths
  `mod.Name` in expressions and types, the embedded `std` (above).
- Generics: `comptime T: type` parameters, type functions
  (`fn Pool(comptime T: type) type { return struct { ... } }`), and
  `where T: Interface, ...` (or the built-in `Integer` and `Hash`) after the return type. A call passes types for
  the `comptime` parameters: `max(i64, a, b)`; `Pool(Node)` is a type.
  The entry point is `pub fn main()` or `pub fn main() !void`.
- Statements: `let`, `var`, `=` and compound assignment (`+= -= *= /= %= +%= -%= *%= &= |= ^= <<= >>=`),
  `if`/`else`, `while`, `for i in lo..hi`, `for x in seq`, `for i, x in seq`,
  `break`, `continue`, `return`, `defer`, `errdefer` (of a call, assignment,
  `if` or block), nested blocks, call
  statements, `if x |v| { }` and `while x |v| { }` (unwrap a `?T`), `if f() |v| { } else |e| { }`
  (branch on an error union), `_ = x`, `match` statements.
- Expressions: integer/float/bool/string/char literals, multi-line strings
  (`\\` lines), `+ - * / %` (checked),
  `+% -% *%` (wrapping), bitwise `& | ^ ~` and shifts `<< >>` (checked shift
  amount), comparisons (also of pointers, payload-less enums and error values), `&& || !`, unary `-`, calls, struct
  literals, field access, `as` numeric casts, arrays `[3]i32{1, 2, 3}`,
  bounds-checked indexing, slicing `a[lo..hi]` (also `p[lo..hi]` of a `[*]T`,
  unchecked), `.len`, `.ptr`, `&x`, `&mut x`,
  `p.*` (field access and array indexing through a pointer dereference it),
  `null`,
  `x == null`, enum values `Shape.Circle { r: 1.0 }` / `Color.Red`,
  exhaustive `match` with patterns `.V`, `.V { a, b: c, .. }` and `_`, and on
  integers with `1`, `'a'`, `'a'..='z'` and several patterns per arm,
  error values `ParseError.Empty`, `try f()`, `f() catch |e| { ... }` (a
  block that ends with the replacement value, or leaves) and
  `f() catch value`, and exhaustive `match e` on errors, method calls
  `x.m(...)` and `Type.f(...)`, struct literals of a type function's
  struct `Pair(i64) { ... }`, the empty slice `[]T{}`, the repeat literal
  `[64]bool{false, ..}`, `x orelse y` (chaining through optionals; `x
  orelse { ...; y }`), function values
  (`let f: fn(i64) i64 = twice`, `f(1)`, `s.callback(x)`), `x is .V` (does
  the enum `x` hold variant `V`), `@hash(x)`, and on `simd[N]T` vectors
  lane-wise `+ - * /` and comparisons (a mask), `v[i]`, `@splat`,
  `@reduceAdd`, `@reduceMin`, `@reduceMax`, `@all`, `@any`, `@select`,
  and `@embedFile("ua.css")` (a file's bytes as a `[]u8` constant, read
  when the program is compiled).
- Methods: `fn` or `pub fn` in a struct or enum body, with `Self` and a receiver
  `self: Self`, `self: *Self` or `self: *mut Self`.
- Interfaces: `interface Name { fn m(self: *Self, ...) T }`, implemented by
  any struct that has the methods; `dyn Name` values made from `&x` or
  `&mut x`, called through a vtable, and turned back into their struct
  with `@downcast(*mut S, d)` (`null` for another type).
- Memory: `var gpa = std.heap.General.init()` (libc; leak, double-free and
  use-after-free-write reports in debug mode, constant time per call),
  `std.heap.Arena.init(backing)`, a struct with `alloc` and `free` methods
  (`&mut slab` is an `Allocator`), `x.allocator()`,
  `x.deinit()`; on a `std.mem.Allocator`: `try alloc.create(value)` (`AllocError!*mut T`),
  `alloc.destroy(p)`, `try alloc.alloc(value, n)` (`AllocError![]mut T`,
  every element set to `value`) and `alloc.free(slice)`.
- Tests: `test "name" { }`, `@expect(cond)`, `@expectEqual(expected, actual)`,
  `@testAllocator()`, and ```` ```aye ```` examples in `///` doc comments.
- Safety: `@setSafety(false)` as the first statement of a block turns
  off its runtime checks.
- C interop: `extern fn` with C-compatible types (also `extern struct`s by
  value, pointers to them, function types as callbacks, and `...` for
  variadic functions such as `snprintf`), `[*]T` indexing and slicing,
  `?[*]T`, `@ptrCast(T, p)`, `@sizeOf(T)`, `@zeroed(T)`, and `@link("name")`
  (`pkg-config` flags, `-lname` or `framework:Name`) for `aye build`, `run`
  and `test`; `@linkOptional("name")` with `@hasLink("name")` for a library
  the program can do without; `let c = @cimport("zlib.h")` declares a C
  header's functions, structs, unions, enum constants, typedefs and
  `#define`d constants, read with libclang and cached in
  `~/.cache/aye/cimport`.
- Types: `i8..i64 u8..u64 isize usize f32 f64 bool` (and `c_int`,
  `c_long`, ... for C), structs, enums, error
  sets, `[N]T` (`N` a constant), `simd[N]T` (vectors), `[]T`, `[]mut T` (writable elements), `*T`, `*mut T`, `?T`,
  `[*]T` (FFI pointer), `fn(A, B) R`, `dyn I`, `std.mem.Allocator`, `std.heap.General`,
  `std.heap.Arena`, instances
  of type functions (`Pool(Node)`), and, as a
  function's return type only, error unions `E!T` and `!T` (inferred set; not
  on a `pub fn`).
- Output: `@print(x)` / `@println(x)` / `@println()` for integers, floats,
  bools and `[]u8` strings; `std.fs.stdout().write(bytes)`; `std.log.err` and
  `std.log.info` (to stderr, with `{}` placeholders); or call libc through
  `extern fn`.

Not yet supported (reported as errors): `comptime` values other than types
and compile-time execution beyond type functions, error payloads, error
unions outside return types, `match` on strings, pointer arithmetic, enums
as `dyn` values, closures, C bit fields (`@cimport` makes such records
opaque), `build.aye` and package dependencies, renaming imports. std has
`coro`, `crypto`, `diag`, `encoding`, `fs`, `heap`, `http`, `json`, `list`,
`log`, `map`, `math`, `mem`, `net`, `os`, `path`, `pool`, `process`, `sort`,
`sync`, `testing`, `text`, `thread` and `time`; `aye spec` indexes them and
`aye doc --signatures std.<m>` prints a module's API.

See [`docs/grammar.ebnf`](docs/grammar.ebnf) for the grammar and
[`docs/decisions.md`](docs/decisions.md) for choices not fixed by the design doc.

## Compiler pipeline

`src/lexer.rs` → `src/parser.rs` (`src/ast.rs`) → `src/module.rs` (loads
modules, resolves names across them, checks `pub` and cycles; one AST) →
`src/sema.rs` (typed AST) →
`src/oir.rs` (OIR: flattened typed IR; `defer` expanded, checks explicit) →
`src/cgen.rs` (C) → clang, driven by `src/driver.rs`. Diagnostic codes and
their explanations are in `src/codes.rs`.

## Tests

```sh
cargo test
```

`tests/golden.rs` compiles every `tests/cases/*.aye` in debug, safe and fast
mode, runs it and compares stdout with `*.out`; when a `*.panic` file exists
the program must abort with exactly that stderr (debug and safe only). When a
`*.stderr` file exists, `main` must return an error: exit status 1, and stderr
equal to the file in debug mode, or to its first line (no trace) in safe and
fast mode. `tests/ffi/*.aye` are run the same way, with `SDL_VIDEODRIVER=dummy`,
and skipped when `pkg-config` does not know a package of `*.needs`. Every
`tests/errors/*.aye` must fail to compile with exactly the
diagnostics in `*.err`, one per line. For every `tests/tests/*.aye`,
`aye test` must print exactly `*.out` (and `aye test --json` exactly `*.json`,
if it exists), with exit status 1 when a test failed. For every
`tests/fix/*.aye`, `aye fix` on a copy must leave exactly `*.fixed` and print
exactly `*.out`; when `*.stderr` exists, errors remain and stderr must equal
it, otherwise `aye check` must accept the fixed file. `aye check --json` on
the original must print exactly `*.json`, if it exists. Each directory
`tests/modules/X/` is a directory module run with `aye run X` (or the
arguments in `X.args`, with `X.stdin`): stdout must equal `X.out`, stderr
`X.stderr` and the exit status `X.status`; or, when `X.err` exists, `aye
check X` must fail with exactly that. `tests/example1` runs the design
doc's Example 1 with an `app.toml` that is valid, missing and invalid.
`tests/spec.rs` checks `docs/spec.md`: its ` ```aye ` programs compile, its
` ```aye-error ` programs do not, it names every pub function of std, and it
stays within 30k tokens.

Before you change the compiler, std or the browser, read
[`docs/practices/`](docs/practices/README.md): good and bad cases from this
project. When you learn a lesson that will come up again, add an entry there.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT), at your option. Third-party files keep their own
notices: the Octane benchmark ports in `browser/tests/jsbench/` (see its
README) and jQuery in `browser/tests/jsdom/scripts/`.

Unless you explicitly state otherwise, any contribution you submit for
inclusion in this project, as defined in the Apache-2.0 license, is dual
licensed as above, without any additional terms or conditions.
