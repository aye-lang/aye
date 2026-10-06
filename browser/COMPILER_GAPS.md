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

## B6 (JavaScript engine, `browser/js/`)

### 10. Debug-mode `General` makes a garbage-collected heap quadratic

Needed: a heap that allocates and frees millions of small slices
(property tables, elements, environments, strings) over a run.

In debug mode `std.heap.General` keeps a record per allocation for the
whole run, never reuses freed memory, and `free` scans the records from
the oldest, so each `free` costs O(allocations so far):

```ore
import std

pub fn main() {
    var gpa = std.heap.General.init()
    defer gpa.deinit()
    let a = gpa.allocator()
    for i in 0..200000 {
        let s = a.alloc(0 as u8, 16) catch |e| {
            return
        }
        a.free(s)
    }
}
```

takes about 30 s, quadratic (50000 iterations take 2 s), and keeps all
200000 blocks and records until `deinit`; `--mode fast` takes
milliseconds. `fib(22)` in the first engine took 40 s, almost all of
it in `ore_general_free`. Ore code cannot implement `Allocator` itself
(decisions.md), so a pooling allocator cannot be handed to the code that
allocates. Workaround: `js/slabs.ore`, size-class free lists cut from
16K-element chunks for the heap's slices; atoms packed in chunks; pooled
scratch `text.Builder`s and key lists on the interpreter. The leak check
still works (chunks are freed at `deinit`). Suggestion: a hash of live
records (or an intrusive header) for `free`, and freeing (or poisoning
and reusing) memory after N frees; or an `Allocator` that Ore code can
implement.

### 11. `*mut S` does not convert to `?dyn I`

```ore
interface Host {
    fn finalize(self: *mut Self, tag: u32)
}

struct DomHost {
    n: i64

    fn finalize(self: *mut Self, tag: u32) {
        self.n += 1
    }
}

struct Engine {
    host: ?dyn Host
}

pub fn main() {
    var h = DomHost { n: 0 }
    var e = Engine { host: null }
    e.host = &mut h
}
```

`error[E0023]: mismatched types: expected ?dyn Host, found *mut DomHost`.
`*mut S -> dyn I` and `T -> ?T` both convert, but not in one step.
Workaround: `let d: dyn Host = &mut h` then `e.host = d`.

### 12. One struct's methods must all be in one file

The interpreter is a struct with ~150 methods; Ore has no `impl` blocks in
other files, so `interp.ore` is 3000 lines. Built-ins are free functions
`fn arrayPush(it: *mut Interp, a: Args)` instead. With the no-shadowing
rule every such top-level name is then reserved in the whole module: a
helper `fn len(it, o)` in `builtins_array.ore` made every local `len` in
the module an error (`numconv.ore` has several). Workaround: methods for
the core, distinctive names (`alen`, `appendWtf8`) for module-level
helpers. Suggestion: allow methods of a struct in any file of its module
(the module is the namespace anyway), or exempt locals from clashing with
functions of other files.

### 13. The C stack is the only stack

The tree-walking interpreter recurses on the C stack; a debug build uses
about 7 KB of stack per JavaScript call, so 1100 nested calls overflowed
the 8 MB main thread (segfault, no Ore diagnostic). Ore has no threads in
std and no way to ask for a larger stack. Workaround: `js.runOnBigStack`
starts the script on a pthread with a 1 GB stack (C interop:
`pthread_attr_setstacksize`, an Ore function as the start routine, the
job passed through `@ptrCast([*]u8, &mut job)`), and the engine throws a
RangeError at 10000 frames, like Node. Suggestion: a stack-overflow check
that panics with a message (guard page + handler), and `std.thread`.

### 14. Smaller things

- `test` is a keyword, so an AST node cannot have a field `test`
  (JavaScript's `if`/`for` nodes call it `cond`).
- No discard: `_ = f()` is `undefined variable _`; a dropped result is
  fine as a statement (`try self.intern(s)`), so this only surprised.
- `?Class == Class` (entry 3) again: `it.classOf(v) == Class.Array`;
  workaround `it.hasClass(v, Class.Array)`.
- An array of 13 `List`s needs 13 written-out initializers
  (`[13]List([]mut T){List(...).init(alloc), ...}`); a repeat form
  (`[13]T{x} ** 13`, Zig's `**`) would do.
- No `unreachable`/`@panic` (entry 4): natives that always throw end in
  `try it.typeError(...)` followed by a dead `return NONE`.
- Enum match arms that list several variants (entry 2): the parser's
  precedence table repeats `=> 7` four times for `== != === !==`.
