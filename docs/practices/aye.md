# Aye practices

How to write Aye that the stage0 compiler and the browser expect: errors,
optionals, memory, builtins, C interop, coroutines and build modes. Most
entries come from `browser/COMPILER_GAPS.md`, where an old workaround is
the bad case and the feature that replaced it the good one; the rules
themselves are in `docs/spec.md` and `docs/decisions.md`.

## Errors

### Map one error set onto another with `catch return`
**Context:** a function that returns `AllocError!T` calls one that returns
another set (the DOM bindings call the JS engine's `JsError!Value`).
**Bad:** a `catch` block at every call, removed in cff3f60:
```aye
fn jsPush(s: *mut Script, v: Value) AllocError!void {
    s.it.push(v) catch |e| {
        return AllocError.OutOfMemory
    }
}
```
**Good:** `catch` takes a bare `return`, `break` or `continue`, like
`orelse` (b78afde). `browser/engine/script.aye:722`:
```aye
fn jsPush(s: *mut Script, v: Value) AllocError!void {
    s.it.push(v) catch return AllocError.OutOfMemory
}
```
The same form gives a fallback: `let n = text.parseFloat(s) catch return
null` (`browser/layout/transform.aye:349`).
**Why:** one line per call, and the mapping is visible where it happens.

### End a `catch` or `orelse` block with its value
**Context:** "report the error, then go on with a fallback".
**Bad:** a `var` set in an `if ... else |e|`, the only form before d118815:
```aye
fn parseOr(s: []u8, old: i64) i64 {
    var v = old
    if parse(s) |n| {
        v = n
    } else |e| {
        std.log.info("keeping {}: {}", old, e)
    }
    return v
}
```
**Good:** the block's last expression is the replacement value:
```aye
fn parseOr(s: []u8, old: i64) i64 {
    let v = parse(s) catch |e| {
        std.log.info("keeping {}: {}", old, e)
        old
    }
    return v
}

fn findOr(s: []u8) i64 {
    let i = find(s) orelse {
        std.log.info("no match in {}", s)
        -1
    }
    return i
}
```
**Why:** the result is a `let`, and the fallback sits next to the call.
Only `catch` and `orelse` blocks give values; an `if` is still a statement
(`let x = if c { 1 } else { 2 }` is an error).

### `return match` without `try` in every arm
**Context:** an `E!T` function whose result is chosen by a `match` over
calls that return `E!T`.
**Bad:** `try` in each arm, removed in cff3f60:
```aye
return match act {
    .Back => try moveInHistory(s, tmp, -1)
    .Click { ref } => try click(s, tmp, ref)
    ...
}
```
**Good:** a directly returned `match` treats each arm as the operand of a
`return`: a value, an error or an `E!T` call (f7f4754).
`browser/agent/actions.aye:91`:
```aye
fn applyAction(s: *mut Session, tmp: Allocator, act: Action) AllocError!Outcome {
    return match act {
        .Navigate { target } => navigate(s, tmp, target)
        .Back => moveInHistory(s, tmp, -1)
```
**Why:** less noise, and `errdefer`s run as for `return f()`. It works
only for a `match` that is returned directly.

### Compare one error with `==`
**Context:** "is this error OutOfMemory (pass it on) or something to
report?", or a test that expects one error.
**Bad:** mapping each error to a number to compare them, removed in
cff3f60 (`browser/image/tests.aye`):
```aye
// Error values have no `==`: compare them through a number.
fn errorCode(e: ImageError) u8 {
    return match e {
        .OutOfMemory => 0
        .Invalid => 1
        .Unsupported => 2
    }
}
```
**Good:** `==` on error values (eedf75d). `browser/image/tests.aye:15`:
```aye
fn inflateFails(a: Allocator, stream: []u8, expected: ImageError) bool {
    let out = inflateZlib(a, stream) catch |e| {
        return e == expected
    }
    a.free(out)
    return false
}
```
**Why:** errors are identified by name across sets, so `==` is exact. Use
`match` when several errors need different handling.

### Spell the error set where inference cannot work
**Context:** `pub` functions, functions that call each other, and any
call that returns an error union.
**Bad:** each of these is rejected:
```aye
pub fn g(n: i64) !i64 { ... }      // E0065: a `pub` function must spell out its error set
fn ping(n: i64) !i64 { return pong(n - 1) }
fn pong(n: i64) !i64 { return ping(n) }   // E0069: cannot infer the error set of `ping`
let r = f(1)                       // E0061: unhandled error: `f` returns `E!i64`
```
**Good:** write `E!T` on `pub` and mutually recursive functions (the
agent's actions all spell `AllocError!Outcome`, see above), and handle a
union where it is made: `try`, `catch`, `if f() |v| { } else |e| { }`, or
`return f()`.
**Why:** an error union is not a value that can be stored or passed; only
these forms consume it.

## Optionals and enums

### Unwrap with `orelse` and `.?`
**Context:** an optional that must have a value for the rest of the
function, or a fallback value.
**Bad:** before b78afde, an `if x |v| { }` around the whole body, or a
dummy initializer such as `var fi: usize = 0` that a branch overwrites
(the adoption agency in `browser/html/parser.aye` still has one):
```aye
fn pointAt(ui: *mut Ui) AllocError!void {
    if ui.agents |a| {
        // the whole function, one level deeper
    }
}
```
**Good:** `browser/ui/agents.aye:749`:
```aye
    let a = ui.agents orelse return
    let slot = ui.slots[ui.active].?
```
`orelse` also chains (`a() orelse b() orelse "none"`), and `orelse
unreachable` states that `null` cannot happen.
**Why:** the happy path stays flat. `.?` panics with `unwrap of null` in
debug and safe mode; use it only when `null` is a bug.

### Compare an optional, test a variant: no helper
**Context:** "is the label UTF-8?" on a `?Encoding`; "did the action do
nothing?" on an enum whose other variants carry payloads.
**Bad:** helpers and catch-all arms, removed in cff3f60:
```aye
// `?Encoding == Encoding` does not compile (see COMPILER_GAPS.md).
fn labelIs(label: []u8, want: Encoding) bool {
    if fromLabel(label) |e| {
        return e == want
    }
    return false
}

match act {
    .Nothing => { ... }
    _ => {}
}
```
**Good:** `?T == T` (M9.5) and `x is .V` (6af69f6).
`browser/encoding/encoding.aye:408` and `browser/ui/forms.aye:105`:
```aye
    @expect(fromLabel("UTF-8") == Encoding.Utf8)

    if act is .Nothing {
```
**Why:** `?T == T` is false for `null`, which is what the helper did. `is`
binds nothing; use `match` when you need the payload.

## Memory

### Take an `Allocator`; use an arena for one lifetime
**Context:** any function that allocates; scratch data for one call or
one request.
**Bad:** freeing each temporary by hand on every path, or a module-level
allocator. Reads after free are not detected, so every manual `free` is a
chance to read freed memory later.
**Good:** the caller passes an `Allocator`; scratch work goes into an
arena freed with `defer`. `browser/ui/agents.aye:751`:
```aye
    var arena = Arena.init(ui.alloc)
    defer arena.deinit()
    let tmp = arena.allocator()
```
**Why:** one `deinit` frees everything, and debug-mode `General` reports
whatever the backing allocator still holds (`leak: N bytes allocated here
were never freed`). `General` and `Arena` must be `var`s, and they are
not thread-safe: one per thread.

### An arena owned by a returned struct lives on the heap
**Context:** a value that owns an arena and is returned (`layout.Tree`,
`css.Sheet`).
**Bad:** keeping the `Arena` by value. `arena.allocator()` points at the
local, and the returned copy does not move that pointer; this compiles
and crashed (exit 139) in a debug build:
```aye
fn build(backing: Allocator) AllocError!Tree {
    var arena = Arena.init(backing)
    let a = arena.allocator()          // points at the local `arena`
    return Tree { arena: arena, boxes: List(f64).init(a) }
}
```
**Good:** create the arena with the backing allocator and keep a
`*mut Arena`. `browser/layout/layout.aye:349` and `:273`:
```aye
    let arena = try alloc.create(Arena.init(alloc))
    let a = arena.allocator()

    pub fn deinit(self: *mut Self) {
        self.arena.deinit()
        self.backing.destroy(self.arena)
    }
```
**Why:** an `Allocator` is a data pointer and a vtable; the data must not
move while the allocator is in use.

### Do not keep pointers into a `List` or `Pool` across growth
**Context:** a `*mut Node` or `items()` slice held across a call that may
`append` or `insert`.
**Bad:** open gap 3 in `browser/COMPILER_GAPS.md`:
```aye
var xs = List(i64).init(alloc)
try xs.append(1)
let items = xs.items()
for i in 0..100 {
    try xs.append(i as i64)    // may move the elements
}
items[0] = 5                   // writes freed memory
```
Debug-mode `General` reports the write only later (`panic: use after
free: ... was written after it was freed`); a read is not reported.
**Good:** keep an index or a `Handle(T)` and fetch again after growing
(`xs.items()[at] = 5`), or copy the fields out first.
`browser/html/parser.aye:986`:
```aye
    fn cloneElement(self: *mut Self, h: NodeId) AllocError!NodeId {
        let n = self.doc.node(h)
        let name = n.name
        let attrs = n.attrs
        return self.doc.createElement(name, attrs)
    }
```
**Why:** growth reallocates; `Pool` handles carry a generation, raw
pointers do not.

### Field defaults, not uninitialized locals or `@zeroed`
**Context:** a struct with many fields that start at zero (layout's
`Box`), or a local set in branches.
**Bad:** both forms are declined (gap 5), and `Tree.newBox` still lists
all 35 fields:
```aye
var x: i64            // E0013: `var x` needs an initializer (`= value`)
let s = @zeroed(S)    // E0093: only an `extern struct` has a defined all-zero value
```
**Good:** field defaults (dbe20f8); leave them out of the literal:
```aye
struct Box {
    x: f64 = 0.0
    y: f64 = 0.0
    width: f64 = 0.0
    parent: ?*Box = null
    node: u32
}

let b = Box { node: 7, width: 100.0 }
```
For a local chosen in branches, use a `catch`/`orelse` block with a value
or `if f() |v| { } else |e| { }`.
**Why:** a default is a checked constant, and a field added later without
one still finds every literal (E0039). `@zeroed` stays for `extern
struct`s and arrays, where C reads the memory.

## Builtins and build modes

### `@downcast` a `dyn`, do not `@ptrCast` a stashed pointer
**Context:** code that holds a `dyn I` and needs the struct behind it
(the JS engine's natives need the page's `Script`, known as `dyn Host`).
**Bad:** a second, untyped field and an unchecked cast, removed in
cff3f60:
```aye
    s.it.host_data = @ptrCast([*]u8, s)

fn scriptOf(it: *mut Interp) *mut Script {
    return @ptrCast(*mut Script, it.host_data.?)
}
```
**Good:** `@downcast(*mut S, d)` gives `?*mut S`, `null` when `d` was made
from another struct (8a1d3e9). `browser/engine/script.aye:361`:
```aye
fn scriptOf(it: *mut Interp) *mut Script {
    return @downcast(*mut Script, it.host.?).?
}
```
**Why:** it compares vtables, so it is checked and costs nothing. It gives
`*mut S` only when `S` has a `*mut Self` method of the interface.

### `@embedFile` for data that has its own format
**Context:** a style sheet, a script, any file that other tools should
read as itself.
**Bad:** the file pasted into a `\\` line string; still the case in
`browser/js/lib.aye:9` ("no @embedFile"), and `css/ua.aye` keeps an array
of rule strings:
```aye
pub let LIB =
    \\(function () {
    \\'use strict';
```
**Good:** `@embedFile` (d286597), a `[]u8` constant read at compile time:
```aye
// The JavaScript source stays a .js file that JS tools can check.
pub let LIB = @embedFile("lib.js")
```
See `tests/modules/embed/main.aye:8`.
**Why:** the file stays checkable by its own tools, and `rg embedFile`
lists what the build reads. The path is relative to the source file;
absolute paths are refused.

### `@setSafety(false)` first in the hot block
**Context:** a measured hot loop (image decoding) that must be fast in the
debug build the browser uses.
**Bad:** the call in the middle of a block, as if it applied from that
line on:
```aye
fn sum(xs: []u8) u8 {
    var s: u8 = 0
    @setSafety(false)   // E0108: `@setSafety` must be the first statement of a block
```
**Good:** first statement of the block it covers (25cac16).
`tests/cases/set_safety.aye:5`:
```aye
fn sum(xs: []u8) u8 {
    @setSafety(false)
    var s: u8 = 0
    for x in xs {
        s += x
    }
    return s
}
```
**Why:** it removes the block's overflow, bounds, cast, shift and unwrap
checks (as `fast` mode does), not those of the functions it calls, and not
the allocator's debug checks. `+` then wraps: be sure that is wanted.

### Pick the build mode for the job
**Context:** testing, measuring, shipping.
**Bad:** timing a debug build. Debug mode gives every `General` block a
64-byte header, holds freed blocks in a 32 MiB quarantine and checks
every operation at `-O0`; a decode
loop took 0.54 s in debug and 0.05 s in safe mode (c6a8cfb). Equally bad:
reaching for `fast` to make a panic go away; the bug stays and becomes
undefined behavior.
**Good:**
```
aye test browser                 # debug (default): checks, error return traces, leak reports
aye run app --mode safe          # optimized, all checks still on
aye build app --mode fast        # optimized, no checks: for measurements and release
```
`browser/js/PERF.md` measures in `fast` mode.
**Why:** debug finds bugs (double free, use after free writes, leaks,
traces); `safe` keeps panics with optimized code; in `fast` overflow
wraps, out-of-bounds access and `std.coro` misuse are undefined behavior,
and a stack overflow is a plain crash.

## C interop

### `@cimport` a header instead of guessing its types
**Context:** calling a C library, especially one whose types differ
between macOS and Linux.
**Bad:** hand-written declarations with guessed layouts, replaced in
cb93543 (`browser/js/js.aye`):
```aye
    var attr = @zeroed([128]u8)       // pthread_attr_t, size guessed
    var thread = @zeroed([1]u64)      // pthread_t, guessed as u64

extern fn pthread_attr_init(attr: [*]u8) i32
extern fn pthread_create(t: [*]u64, attr: [*]u8, f: fn(?[*]u8) ?[*]u8, arg: ?[*]u8) i32
```
**Good:** `browser/js/js.aye:18` and `:55`:
```aye
let pthread = @cimport("pthread.h")

    var attr = @zeroed(pthread.pthread_attr_t)
    var thread = @zeroed(pthread.pthread_t)
```
**Why:** the header gives the platform's real types, and every use reads
`pthread.x`, so C names cannot clash. `aye cimport HEADER` shows what a
program sees. It needs libclang on a machine without the cache; vendor
the output (`aye cimport foo.h > foo.aye`) where that cannot be had.
libcurl, FreeType, HarfBuzz, SDL2 and Brotli keep narrow hand-written
declarations on purpose.

### `c_long` for C `long`, not `i64`
**Context:** hand-written `extern fn` or `extern struct` with C `long`,
`unsigned long` or `int`.
**Bad:** `num_faces: i64` for FreeType's `FT_Long`, replaced in cff3f60.
`browser/js/interp.aye:107` still declares `extern fn clock() u64`, while
`aye cimport time.h` gives `clock() c_ulong` on macOS (and `clock_t` is
`long` on Linux).
**Good:** the C integer names (1872d6b). `browser/font/freetype.aye:35`
and `:143`:
```aye
    num_faces: c_long

extern fn FT_Get_Advance(face: *mut FTFaceRec, glyph: u32, flags: i32, advance: *mut c_long) i32
```
**Why:** `c_long` is the same type as `i64` on LP64 today, so nothing
changes at run time; it records what the header says, and it is the one
place to change on a target where `long` is 32 bits.

### `extern union` for C unions
**Context:** a C union such as `SDL_Event`.
**Bad:** a struct of the right size and a `@ptrCast` to each event's
struct, replaced in cff3f60:
```aye
pub extern struct SDL_Event {
    kind: u32
    pad: u32
    data: [6]u64
}
```
**Good:** `extern union` (1872d6b). `browser/ui/sdl.aye:20`:
```aye
pub extern union SDL_Event {
    kind: u32
    window: SDL_WindowEvent
    key: SDL_KeyboardEvent
    ...
    padding: [7]u64
}
```
Callers take `&ev.key`, `&ev.motion` with no cast.
**Why:** C's layout comes from the declaration, and field access replaces
unchecked casts. A literal sets exactly one field (E0104); a union has no
`==`.

### A std `extern fn` must not reuse a libc name that a program may `@cimport`
**Context:** std declares a libc function by hand, and some program
`@cimport`s the header that declares the same function (the browser's
`archive.aye` imports dirent.h).
**Bad:** `extern fn closedir(dir: [*]u8) i32` in `std/fs/fs.aye`: with
`import std.fs` and `@cimport("dirent.h")` the program fails with
`E0096: C function `closedir` is declared twice with different
signatures`. `extern("closedir") fn cClosedir` does not help: the check
is on the C name.
**Good:** a prelude function under an `aye_std_` name that calls the libc
function through its `aye_rt_` alias (`aye_std_closedir`,
`aye_std_rewinddir` in `src/cgen.rs`):
```aye
extern fn aye_std_closedir(dir: [*]u8)
```
**Why:** the prelude's names are ours alone, so no header can declare
them. `aye check browser` finds this.

## Coroutines (`std.coro`)

### Resume a coroutine only on the thread that made it
**Context:** generators and async functions (the JS engine runs them on
`std.coro` since 34e46aa, instead of a thread per generator and two
channels), or any code mixing coroutines and threads.
**Bad:** handing a coroutine to another thread. Debug and safe mode panic;
`fast` mode has undefined behavior:
```aye
fn worker(j: *mut Job) {
    j.co.resume()   // panic: std.coro: resume of a coroutine on another thread than the one that made it
}
```
**Good:** each thread makes and resumes its own coroutines, with its own
allocator:
```aye
fn worker(j: *mut Job) {
    var gpa = General.init()
    defer gpa.deinit()
    var g = Gen {}
    let co = coro.create(Gen, gpa.allocator(), 64 << 10, run, &mut g) catch return
    defer co.deinit()
    co.resume()
    while !co.isDone() {
        j.sum += g.value
        co.resume()
    }
}
```
**Why:** the running coroutine, the stack bounds and the error trace are
thread-locals, and the C compiler may keep their addresses across the
switch. A coroutine and its resumer never run at the same time, so they
share data without locks.

### End a suspended coroutine before `deinit` if it must clean up
**Context:** a coroutine whose function has `defer`s or owns resources,
freed while it is suspended (a generator whose JS object was collected).
**Bad:** `deinit` of a suspended coroutine frees it where it stopped; its
`defer`s never run:
```aye
    co.resume()       // run() is now suspended in coro.yield()
    co.deinit()       // run()'s `defer g.closed = true` does not run
```
**Good:** resume it with a flag in `arg` so its function returns:
```aye
fn run(g: *mut Gen) {
    defer g.closed = true
    var i: u64 = 0
    while true {
        g.value = i
        coro.yield()
        if g.kill {
            return
        }
        i += 1
    }
}

    g.kill = true
    co.resume()       // run() returns, its defers run; then co.deinit()
```
The browser does this with `RESUME_KILL`: `destroyCoroutine`
(`browser/js/coroutine.aye:316`) resumes it, and `coYield`
(`browser/js/coroutine.aye:420`) then returns `JsError.Terminated`.
**Why:** unwinding a suspended stack would need every `yield` site to
handle an error; std leaves that to the code that knows how to stop.
