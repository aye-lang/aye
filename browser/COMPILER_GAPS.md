# Compiler and language gaps met by the browser

Things the browser needed that Ore (stage0) or its std does not do, or
does in a way that hurt. Each entry: what was needed, a minimal repro,
and the workaround in `browser/`. Newest last.

## B1

### 1. Sub-modules that import siblings cannot be tested on their own

Needed: unit tests next to the code of `html/` and `net/`.

`import dom` in `browser/html/parser.ore` resolves against the root
module's directory, so `ore test browser/html` fails before any test runs:

```
browser/html/parser.ore:9:8: error[E0081]: cannot find module `dom` (no directory `browser/html/dom`)
```

and `test` blocks of non-root modules are never compiled, so tests in
`html/` or `net/` can never run. Workaround: those tests live in the root
module (`browser/unit_tests.ore`, run by `ore test browser`) and only call
`pub` functions; modules without sibling imports (`url`, `dom`,
`encoding`) keep their own tests. Suggestion: `ore test browser
--module html` (compile the program, run one module's tests), or resolve
imports from a project root marker.

### 2. A `match` arm cannot list several enum variants

```ore
enum E { A, B, C }

fn f(e: E) i64 {
    return match e {
        .A, .B => 1
        .C => 2
    }
}
```

`error[E0006]: expected `=>`, found `,`` — integer patterns allow
`' ', '\t' =>`, enum patterns do not. Workaround: duplicate arms
(`.Doctype => {}` / `.Comment => {}` in `dump.ore`), a helper method
(`textNearHead` in `html/parser.ore`), or `_`.

### 3. `?T == T` does not compile, and the hint is wrong

```ore
enum E { A, B }

fn f() ?E {
    return E.A
}

pub fn main() {
    @println(f() == E.A)
}
```

`error[E0023]: mismatched types in `==`: `?E` and `E` (use `as` to
convert)` — `as` cannot convert an optional. Since `T` converts to `?T`
implicitly elsewhere, `opt == value` would be natural. Workaround:
`if f() |e| { return e == E.A }` (`labelIs` in `encoding.ore`).

### 4. No `orelse`, `.?` or `unreachable`

Needed: "this optional is known to be set" and "this cannot happen".
`Pool(T).get` returns `?*mut T`, but a `Document` never removes nodes, so
every handle is valid. With no `.?`, no `orelse` and no `@panic` /
`unreachable`, `Document.node` needs a dead fallback path:

```ore
pub fn node(self: *Self, h: NodeId) *mut Node {
    if self.nodes.get(h) |n| {
        return n
    }
    return &mut self.nodes.slots[h.index].value   // reads Pool's private fields
}
```

The same shape shows up for "take the value or leave the function", e.g.
the adoption agency algorithm:

```ore
var fi: usize = 0
if self.lastActive(subject) |i| {
    fi = i
} else {
    self.anyOtherEnd(subject)
    return
}
```

(a local needs a dummy initializer because `var x: T` without `= value`
is an error). An `orelse return` / `orelse unreachable` would make each
of these one line.

### 5. Structs have no `==`; `Handle(T)` has no `eql`

```ore
struct P { x: i64 }

pub fn main() {
    let a = P { x: 1 }
    let b = P { x: 1 }
    @println(a == b)
}
```

`error[E0025]: operator `==` cannot be applied to `P``. Comparing two
`Handle(Node)` is the most common operation of the DOM. Workaround:
`dom.same(a, b)` and `dom.isSame(?a, b)`. Suggestion: `==` on structs
whose fields are all integers/bools, or `Handle.eql` in std.pool.

### 6. Array literals must spell their length

```ore
let NAMES = [_][]u8{"a", "b"}
```

`error[E0006]: expected array length (integer literal), `]` or `*`, found
identifier `_``. The entity table (`html/entities.ore`, 328 entries) is
generated and its length counted by hand; a wrong count is an error, so
it is safe, just tedious. For the many tag-name sets of the tree builder
this was bad enough that they are space-separated strings searched with
`text.split` (`isIn(name, "dd dt li ...")`) instead of `[N][]u8` arrays.

### 7. `return f()` of a void call in an `E!void` function

```ore
fn g() {
}

fn f() !void {
    return g()
}
```

`error[E0030]: this expression does not produce a value` — while
`return h()` for `h() E!void` is fine and common. Confusing when a helper
changes from `!void` to plain void. Workaround: `g()` then `return`.

### 8. Pointers into a `Pool` or `List` dangle after it grows

Not a missing feature but a trap the compiler cannot see: `doc.node(h)`
returns `*mut Node` into the pool's slots, and the next `insert` may move
them. `cloneElement` in `html/parser.ore` copies `name` and `attrs` into
locals before calling `createElement` for this reason. Use-after-free is
documented as undetected (spec §9); a debug-mode check (generation of the
allocation) would catch it.

### 9. Smaller things

- `std.log.err(USAGE)` with `let USAGE = "..."`: `error[E0086]: the
  format of `std.log.err` must be a string literal`. A constant string
  could be accepted. Workaround: `std.log.err("{}", USAGE)`.
- An import name blocks the same local name everywhere in the file:
  with `import url`, `let url = ...` is `E0016` (plus a second, cascading
  `E0085` on its use). The CLI calls its URL `location`.
- `text.split(s, "=").next()` is `E0035` (cannot call a `*mut Self`
  method on a temporary); clear message, one more `var` line.
- `@testAllocator()` only inside `test` blocks: shared test helpers take
  an `Allocator` parameter (clear message; by design).

## B2

### 10. Two implicit adjustments do not chain

```ore
fn get(s: []mut u8) ?[]u8 {
    return s
}
```

`error[E0023]: mismatched types: expected `?[]u8`, found `[]mut u8``,
although `[]mut T` to `[]T` and `T` to `?T` are each implicit. The same
for an error union's payload:

```ore
fn copy(a: Allocator) AllocError![]mut u8 {
    return a.alloc(0 as u8, 4)
}

fn get(a: Allocator) AllocError![]u8 {
    return copy(a)
}
```

`expected `std.mem.AllocError![]u8`, found `std.mem.AllocError![]mut u8``
— this one hits every `return builder.toOwned()` from a function that
returns read-only bytes. Workaround: a typed local
(`let values: []Value = v` in `Styles.get`, `let owned = try
b.toOwned()` in `css/tokenizer.ore`).

### 11. No multi-line string literals

```ore
pub fn main() {
    let s = "a
b"
}
```

`error[E0004]: unterminated string literal`. The user-agent style sheet
(`css/ua.ore`, 50 rules) is an array with one rule per string, its length
counted by hand (gap 6), and parsed string by string, so no rule may span
two strings. A raw multi-line literal (Zig's `\\` lines) would let it be
plain CSS; so would `@embedFile("ua.css")`.

### 12. An array length cannot be a constant

```ore
let N: usize = 3
let XS = [N]u8{1, 2, 3}
```

`error[E0006]: expected array length (integer literal), `]` or `*`, found
identifier `N``. The property table `css.PROPERTIES` is `[42]Property`
and `css.NPROPS` repeats the 42; they must be changed together.

### 13. A name declared in one file of a module blocks locals in the others, and the error does not say where

With `fn px(v: f64) f64` in `app/a.ore`:

```ore
// app/main.ore
fn half(v: f64) f64 {
    let px = v / 2.0
    return px
}
```

`app/main.ore:2:5: error[E0016]: `px` is already declared at 1:4
(shadowing is not allowed)` — `1:4` is in `a.ore`, but the message reads
as `main.ore:1:4`. In `css/` (eight files, one namespace) a helper `px` in
`cascade.ore` broke locals in `media.ore` and `values.ore`. The no-
shadowing rule is by design (spec §6); the missing file name is the bug.
Workaround: distinctive names for top-level helpers (`numberOr`).

### 14. No way to make a C library optional

`@link("libcurl")` in `net/http.ore` links libcurl into every build of the
browser, and there is no conditional compilation, so the browser (and all
its tests, not just the HTTP ones) cannot be built where libcurl is
missing. Workaround: none; libcurl is a hard requirement (docs/browser.md
lists it as a platform library anyway). Something like
`@link("libcurl", optional)` with `@hasLibrary("libcurl")`, or build
tags, would let `net/http.ore` fall back to `FetchError.Unsupported`.

### 15. Smaller things

- `defer if cond { ... }` is `error[E0006]: expected expression, found
  `if``; `defer { if cond { ... } }` works (spec: a call, an assignment
  or a block). A bare `if` statement could be accepted.
- Methods live in the struct body, so a struct's methods cannot be split
  across the files of its module: the parser, selector and cascade code
  for `css.Sheet` are free functions taking `*mut Sheet`
  (`css.parseSheet(&mut sheet, src, media)`, not `sheet.parse(src)`).

## B3

### 16. FreeType is now a hard requirement too

Same root as gap 14: `browser/font/freetype.ore` has
`@link("freetype2")`, so the browser no longer builds where FreeType is
missing, although the layout tests only need the fixed-metric font.
`tests/browser.rs` can only skip the `--font system` smoke test when no
font *file* is found (`System.init` returns `FontError.NoFont`), not when
the library is absent. Workaround: none (docs/browser.md lists FreeType as
a platform library). An optional link plus `@hasLibrary("freetype2")`
would let `font.System` fail at run time instead.

### 17. No C `long`

```ore
extern struct FTFaceRec {
    num_faces: i64   // FT_Long is C `long`
}
```

`FT_Long`, `FT_Pos` and `FT_Fixed` are C `long`, which is 64 bits on
macOS and Linux but 32 on Windows; Ore has `i32` for `int` and `usize` for
`size_t` but nothing for `long`, so the prefix of `FT_FaceRec` and the
signatures of `FT_New_Face`/`FT_Get_Advance` hard-code `i64` (LP64).
Suggestion: `c_long`/`c_ulong` aliases that follow the target.

### 18. Smaller things

- Struct literals list every field (spec: no default values). The layout
  `Box` has 35 fields, mostly results that start at 0; `Tree.newBox` is
  the only literal, and a new field means one more line there. Per-field
  defaults (`x: f64 = 0.0`) would remove the helper (`inline.piece` is
  another).
- No tuples or multiple results: small structs carry pairs out of
  functions (`MinMax`, `Flow`, `OpenFrag`, `KindOf`). Fine, a bit verbose.
- Gap 8 again: `Tree.bx(i)` returns `*mut Box` into `List(Box)`; it must
  be fetched again after anything that may append a box (only the tree
  builder does), so layout code re-reads `t.bx(i)` after every call
  instead of keeping a pointer across calls.

## B7

### 19. Error values have no `==`

```ore
error E { A, B }

fn f() E!i64 {
    return E.A
}

pub fn main() {
    let v = f() catch |e| {
        if e == E.A {
            return
        }
        return
    }
}
```

`error[E0025]: operator `==` cannot be applied to `E``. Needed: "is this
fetch error OutOfMemory (pass it on) or something to report?" in
`agent/session.ore`. Workaround: a helper with a `match` and `_`
(`fetchOutOfMemory`). Enums without payloads have `==`; error values
could too.

### 20. `orelse` does not chain through optionals

```ore
fn a() ?[]u8 {
    return null
}

pub fn main() {
    let s = a() orelse a() orelse "none"
    @println(s)
}
```

`error[E0023]: mismatched types: expected `[]u8`, found `?[]u8``: the
left `orelse` wants a `[]u8` on its right. "The alt, else the title, else
nothing" for images in `agent/markdown.ore` is written `x orelse (y
orelse "")`. Right associativity (or `?T orelse ?T` giving `?T`) would
read naturally.

### 21. No line-by-line reading of stdin

`std.fs` reads a whole file or stream (`readAll`), which never returns
for an interactive stdin, but the agent servers answer one JSON line at
a time. Workaround: `LineReader` in `agent/server.ore` over `extern fn
read(fd: i32, ...)` (the same signature std's `fs.ore` uses) with a
`Builder` of pending bytes. A `File.read(buf) ReadError!usize` and a
buffered `readLine` in std would replace it.

### 22. The root module cannot be imported

`browser/agent` needs what `browser/main.ore` and `browser/style.ore`
already do (a file path or URL to a `Url`, a response to a document, the
author style sheets of a document), but those live in the root module,
and a module cannot import the root. They are copied into
`agent/session.ore` (`relativeUrl`, `parseDocument`, `agentSheets`).
Moving them into a module (`page/`) is the real fix and is left for the
unification with the window's tabs.

### 23. Smaller things

- A `match` arm cannot produce an error union for the enclosing
  `return`: `return match act { .Back => back(s) ... }` is `E0061`
  (unhandled error) until every arm says `try back(s)`.
- The actions call each other (`clickNode` follows a label to its
  control, `press` clicks), so their error set cannot be inferred
  (`E0069`); they all spell `AllocError!Outcome`. Clear message, as
  documented.
- std.math has no infinity, NaN, `isNan` or `isInf`: the JSON number
  writer tests `v != v || v - v != 0.0`, and an overflowing number gets
  infinity from `1.7976931348623157e308 * 10.0` at run time.
- `@zeroed` covers numbers, pointers and arrays of them, not `[64]bool`:
  the JSON `Writer` keeps its per-level "a value was written" flags in a
  `u64` bitmask (so at most 64 levels of nesting).
- `layout.Tree` points to its document, styles and font, so a page that
  owns all four must stay on the heap (`alloc.create(Page { ... })`) and
  build the tree after it is there; with no uninitialized fields, the
  tree is a `?layout.Tree` filled in a second step and read with
  `p.tree.?`.
