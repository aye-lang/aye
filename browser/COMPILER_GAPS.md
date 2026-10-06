# Compiler and language gaps met by the browser

Things the browser needed that Ore (stage0) or its std does not do, or
does in a way that hurt: the input for the compiler's ergonomics
milestones (M9.5 and M9.6 came from this list). One numbered list,
grouped by theme. Open gaps come first, ordered by what they cost the
browser (performance, correctness, code); then the ones declined by
design; then the fixed ones. Each entry: **Need**, **Repro**,
**Workaround** (what `browser/` does today), **Hit by** (component and
branch: B1 url/net/html/dom/encoding and the CLI, B2 css, B3
font/layout, B4 paint/engine/ui, B5 forms and images, B6 js, B7 agent)
and **Status**.

Fixed entries were checked against `target/release/ore` at the M9.6
head with the repro or the milestone's feature form (`ore run`, `ore
test`). "Browser still uses the workaround" means the old code is still
there and can now be simplified.

## Open

### Allocator and runtime

#### 1. Debug-mode `General` makes many small allocations quadratic; Ore code cannot implement `Allocator`

- **Need:** a garbage-collected heap that allocates and frees millions of
  small slices (property tables, elements, environments, strings).
- **Repro:**

  ```ore
  import std

  pub fn main() {
      var gpa = std.heap.General.init()
      defer gpa.deinit()
      let a = gpa.allocator()
      for i in 0..50000 {
          let s = a.alloc(0 as u8, 16) catch |e| {
              return
          }
          a.free(s)
      }
  }
  ```

  In debug mode `General` keeps a record per allocation for the whole
  run, never reuses freed memory, and `free` scans the records from the
  oldest, so each `free` costs O(allocations so far). Re-measured at
  HEAD: 2.2 s in debug, under 10 ms with `--mode fast` (B6 measured
  200000 iterations at about 30 s). `fib(22)` in the first JS engine
  took 40 s, almost all in `ore_general_free`. A pooling allocator
  cannot be handed to the code that allocates, because Ore code cannot
  implement `Allocator` (decisions.md).
- **Workaround:** `js/slabs.ore`: size-class free lists cut from
  16K-element chunks for the heap's slices; atoms packed in chunks;
  pooled scratch `text.Builder`s and key lists on the interpreter. The
  leak check still works (chunks are freed at `deinit`).
- **Suggestion:** a hash of live records (or an intrusive header) for
  `free`, and reuse (or poison and reuse) memory after N frees; or an
  `Allocator` that Ore code can implement.
- **Hit by:** js (B6).
- **Status:** open.

#### 2. A stack overflow is a silent segfault; the C stack is the only stack

- **Need:** deep recursion. The tree-walking interpreter uses about 7 KB
  of C stack per JavaScript call in a debug build, so 1100 nested calls
  overflowed the 8 MB main thread.
- **Repro:**

  ```ore
  fn rec(n: i64) i64 {
      var buf = @zeroed([1024]u8)
      buf[0] = 1
      if n == 0 {
          return 0
      }
      return rec(n - 1) + (buf[0] as i64)
  }

  pub fn main() {
      @println(rec(100000000))
  }
  ```

  Exit status 139 (SIGSEGV), no Ore diagnostic.
- **Workaround:** `js.runOnBigStack` starts the script on a pthread with
  a 1 GB stack through C interop (`@link("pthread")`,
  `pthread_attr_setstacksize`, an Ore function as the start routine, the
  job passed as `@ptrCast([*]u8, &mut job)`); the engine throws a
  RangeError at 10000 frames, like Node. Since B8 the engine also
  measures the C stack it has used (`Interp.markStackBase`: the address
  of a local at the start of a task, compared with one at each call) and
  throws the RangeError before the end of the room it was given. With
  no second stack to switch to (no coroutines or fibers), a generator or
  async function that stops in the middle of the tree walk runs on a
  thread of its own (`js/coroutine.ore`, `std.thread.spawnWithStack`
  with 256 MB reserved), and two `std.sync.Channel`s hand control back
  and forth so only one thread runs JavaScript at a time: a thread per
  live generator, 4000 at most.
- **Suggestion:** a guard page and a handler that panics with a message;
  `std.thread` with a stack size; stackful coroutines (a stack switch)
  in std.
- **Hit by:** js (B6, B8).
- **Status:** open. `std.thread` (with `spawnWithStack`) and
  `std.sync.Channel` exist now; no overflow check is planned.

#### 3. Pointers into a growing `List` or `Pool` dangle, undetected

- **Need:** keep a `*mut Node` or `*mut Box` across a call that may add
  to the container.
- **Repro:**

  ```ore
  var xs = List(i64).init(alloc)
  try xs.append(1)
  let items = xs.items()
  for i in 0..100 {
      try xs.append(i)    // may move the elements
  }
  items[0] = 5            // writes freed memory; nothing detects it
  ```

- **Workaround:** copy fields out before growing (`cloneElement` in
  `html/parser.ore` copies `name` and `attrs` before `createElement`);
  fetch the pointer again after any call that may grow (layout code
  re-reads `t.bx(i)` after every call, since `Tree.bx(i)` points into a
  `List(Box)`).
- **Suggestion:** a debug-mode check (a generation per allocation).
- **Hit by:** dom, html (B1), layout (B3).
- **Status:** open. M9.5 documented the trap in the spec's pitfalls; the
  check is not done.

#### 4. Every checked operation is a call in debug builds

- **Need:** decode images (inflate, Huffman, IDCT, color conversion) at
  a usable speed in the browser, which is built in debug mode.
- **Repro:** profile any tight loop of indexing and arithmetic in a
  debug build: each `+`, `*` and `xs[i]` is an out-of-line call
  (`ore_add_usize`, `ore_idx`). Decoding a 1024x768 noisy image takes
  about 180 ms (JPEG), 195 ms (PNG) and 80 ms (GIF) in debug, against
  40, 33 and 7 ms with `--mode safe`, which has the same checks inline.
- **Workaround:** the hot loops of `image/` were restructured (fewer
  indexings per pixel, table-driven Huffman decoding, a whole row at a
  time); pages keep images small.
- **Suggestion:** inline the checks (or emit them as macros) in debug
  mode too; only the error return traces need the calls.
- **Hit by:** image (B5).
- **Status:** open.

### Structs and initialization

#### 5. No default field values; a struct or local cannot start uninitialized

- **Need:** a 35-field layout `Box` that is mostly results starting at
  0; an `engine.Page` that gets its style sheets, computed styles and
  layout tree only when first laid out; locals set in branches.
- **Repro:**

  ```ore
  struct B { x: f64 = 0.0, y: f64 = 0.0 }   // E0006: expected `,`, newline or `}`

  struct S { a: i64, b: ?*S }

  pub fn main() {
      var x: i64                            // E0013: `var x` needs an initializer
      let s = @zeroed(S)                    // E0093: only an `extern struct` has a defined all-zero value
  }
  ```

- **Workaround:** `Tree.newBox` is the only `Box` literal and lists all
  35 fields (`inline.piece` is another such helper); `engine.Page` keeps
  `styling: ?*mut Styling` and `tree: ?*mut layout.Tree`, each allocated
  on its own (this also keeps the tree's pointers into the page stable);
  the agent's page lives on the heap (`alloc.create(Page { ... })`)
  because `layout.Tree` points to its document, styles and font, and
  its tree is a `?layout.Tree` filled in a second step and read with
  `p.tree.?`; dummy initializers such as `var fi: usize = 0` in the
  adoption agency algorithm (`html/parser.ore`).
- **Hit by:** html (B1), layout (B3), engine (B4), agent (B7).
- **Status:** open (no default values by spec).

### Generics and function values

#### 6. At most 64 instances of a generic in a whole program

- **Need:** a `List` of whatever each part of the browser keeps: the
  program passed 64 distinct `List(T)` when B5 and B6 were merged.
- **Repro:** a program that uses `List(T0)` ... `List(T64)` (65
  element types, no recursion) is E0080: "more than 64 instances of
  `std.list.List`: it instantiates itself with larger and larger type
  arguments". The limit is meant to stop runaway recursion (`Nest(?T)`
  in `Nest(T)`), but it counts every instance in the program
  (`check_instance_count` in `src/sema.rs`), and the message blames the
  line that happened to be the 65th (`agent/markdown.ore`, unchanged).
- **Workaround:** fewer element types: localStorage keeps its items as
  three strings each in a `List([]mut u8)` instead of a
  `List(StoreItem)` (`engine/window_api.ore`). The browser is now at
  the limit: the next new `List(T)` anywhere fails to compile.
- **Suggestion:** count only instances whose arguments grow (nested
  instances of the same generic), or count per instantiation chain.
- **Hit by:** engine, agent (B6 merging B5).
- **Status:** open; urgent (it blocks any new list type).

#### 7. Function values cannot carry state; a `dyn` cannot be turned back into its struct

- **Need:** the JavaScript engine's native functions are a table of
  `fn(*mut Interp, Args) JsError!Value`; the DOM bindings' natives need
  the page's `Script` (its document, wrappers, timers). The engine only
  knows the host as a `dyn Host`.
- **Repro:** a native cannot close over the script (no closures, spec
  §4), and there is no way to get the `*mut Script` back from the
  `dyn Host` it implements.
- **Workaround:** `Interp.host_data: ?[*]u8` holds
  `@ptrCast([*]u8, script)`; `scriptOf(it)` casts it back
  (`engine/script.ore`), unchecked. Settings that every page shares
  (`scripting_on`, `real_clock`, `local_store`) are module-level `var`s
  with setter functions.
- **Suggestion:** a context pointer typed by the generic that holds the
  table (a `fn(*mut C, ...)` with `C` known), or a checked downcast of
  `dyn I` to `*mut S`.
- **Hit by:** engine (B6).
- **Status:** open (no closures by spec; the downcast could be added).

### Error handling

#### 8. No way to branch on an error union and fall through

- **Need:** "use the URL after redirects if it parses, else keep the one
  we asked for", and "report the error, then go on with a fallback".
- **Repro:**

  ```ore
  if parse("x") |u| {          // E0006: expected `if` or `{` after `else`, found `|`
      @println(u)
  } else |e| {
      @println("err")
  }

  let u = parse("x") catch |e| {
      @println("report")
      old                      // E0031: expression value is not used; the block
  }                            // must end with return, break or continue
  ```

- **Workaround:** a helper that picks its input before parsing
  (`finalUrl` in `engine/engine.ore`), or returning from inside the
  `catch` (`mouseMoved` in `ui/window.ore`).
- **Hit by:** engine, ui (B4).
- **Status:** partly open. Works at HEAD (verified): `parse(s) catch old`
  and `parse(s) catch |e| match e { .Bad => old ... }` give a value
  without leaving the function. Missing: Zig's `if x |v| {} else |e| {}`,
  or a `catch` block that runs statements and then yields a value.

#### 9. Error values have no `==`

- **Need:** "is this fetch error OutOfMemory (pass it on) or something to
  report?"
- **Repro:**

  ```ore
  error E { A, B }

  fn f() E!i64 {
      return E.A
  }

  pub fn main() {
      let v = f() catch |e| {
          if e == E.A {        // E0025: operator `==` cannot be applied to `E`
              return
          }
          return
      }
  }
  ```

- **Workaround:** a helper with a `match` and `_` (`fetchOutOfMemory` in
  `agent/session.ore`, gone since ef319a6 moved sessions onto
  `engine.Page`); the image decoder tests map each error to a number
  with a `match` to compare it with the expected one. Enums without
  payloads have `==`; error values could too.
- **Hit by:** agent (B7), image (B5).
- **Status:** open.

#### 10. A `match` arm cannot produce the error union of an enclosing `return`

- **Repro:**

  ```ore
  fn run(a: Act, n: i64) E!i64 {
      return match a {
          .Back => back(n)     // E0061: unhandled error: `back` returns `E!i64`
          .Fwd => fwd(n)
      }
  }
  ```

- **Workaround:** `try` in every arm (`try back(s)`) in the agent's
  action dispatch.
- **Hit by:** agent (B7).
- **Status:** open.

#### 11. Mapping one error set onto another takes a `catch` at every call

- **Need:** the bindings call engine functions that return
  `JsError!Value` from code that returns `AllocError!...` (only
  `OutOfMemory` can happen there), dozens of times.
- **Repro:**

  ```ore
  fn oom(r: JsError!Value) AllocError!Value {   // E0066: error union types can only be used as a function's return type
      return r catch |e| {
          return AllocError.OutOfMemory
      }
  }
  ```

- **Workaround:** one small wrapper per callee (`wrapFor`,
  `stringFor`, `jsPush` in `engine/script.ore`), each with its own
  `catch |e| { return AllocError.OutOfMemory }`.
- **Suggestion:** `try f() orelse_error AllocError.OutOfMemory`, or let
  `catch` take an error value to return
  (`f() catch return AllocError.OutOfMemory`).
- **Hit by:** engine (B6).
- **Status:** open.

### Optionals

#### 12. `orelse` does not chain

- **Repro:**

  ```ore
  fn a() ?[]u8 {
      return null
  }

  pub fn main() {
      let s = a() orelse a() orelse "none"   // E0023: expected `[]u8`, found `?[]u8`
      @println(s)
  }
  ```

- **Workaround:** parentheses: "the alt, else the title, else nothing"
  for images in `agent/markdown.ore` is `x orelse (y orelse "")`. Right
  associativity (or `?T orelse ?T` giving `?T`) would read naturally.
- **Hit by:** agent (B7).
- **Status:** open.

### Enums

#### 13. No test for one variant of an enum with payloads

- **Need:** "did the default action do nothing?" for an
  `engine.Activation`, whose other variants carry payloads (a URL, a
  form and its submitter).
- **Repro:**

  ```ore
  enum Act {
      Nothing
      Follow { href: []u8 }
  }

  pub fn main() {
      let a = Act.Nothing
      if a == Act.Nothing {        // E0025: `==` needs an enum without payloads
          @println("nothing")
      }
  }
  ```

- **Workaround:** a statement `match` with an arm for the variant and
  `_ => {}` (`clickPage` in `ui/forms.ore`, `submitFrom` in
  `agent/actions.ore`).
- **Suggestion:** `==` against a payload-free variant of any enum, or
  an `is` test (`a is .Nothing`).
- **Hit by:** ui, agent (B5).
- **Status:** open.

### std and C interop

#### 14. No reading of a stream piece by piece (stdin lines)

- **Need:** the agent servers answer one JSON line at a time on stdin;
  `std.fs` only reads a whole file or stream (`readAll`), which never
  returns on an interactive stdin.
- **Repro:** `std.fs.stdin().read(buf)` is E0071 (`std.fs.File` has no
  method `read`).
- **Workaround:** `LineReader` in `agent/server.ore` over `extern fn
  read(fd: i32, ...)` (the signature std's `fs.ore` uses) with a
  `Builder` of pending bytes.
- **Suggestion:** `File.read(buf) ReadError!usize` and a buffered
  `readLine`.
- **Hit by:** agent (B7).
- **Status:** open.

#### 15. No C `long`

- **Repro:** `extern fn labs(x: c_long) c_long` is E0017 (unknown type).
- **Workaround:** `FT_Long`, `FT_Pos` and `FT_Fixed` are C `long` (64
  bits on macOS and Linux, 32 on Windows); `font/freetype.ore`
  hard-codes `i64` (LP64) in the prefix of `FT_FaceRec` and in the
  signatures of `FT_New_Face`/`FT_Get_Advance`.
- **Suggestion:** `c_long`/`c_ulong` that follow the target.
- **Hit by:** font (B3).
- **Status:** open.

#### 16. No `extern union`

- **Repro:** `extern union U { a: i32, b: i64 }` is E0006 (expected `fn`
  or `struct` after `extern`).
- **Workaround:** `ui/sdl.ore` declares the C union `SDL_Event` as 56
  bytes with the common `kind` field and casts `*SDL_Event` to each
  event's extern struct (`@ptrCast(*SDL_KeyboardEvent, ev)`). The size
  and alignment are written by hand; nothing checks that an event
  struct fits.
- **Suggestion:** `extern union`, or a compile-time assertion on
  `@sizeOf`.
- **Hit by:** ui (B4).
- **Status:** open.

#### 17. `std.math` has no infinity or NaN

- **Repro:** `std.math.inf`, `std.math.nan`, `std.math.isNan` are E0081
  (module `std.math` has no ...).
- **Workaround:** the JSON number writer tests `v != v || v - v != 0.0`
  and makes an overflowing number infinite with
  `1.7976931348623157e308 * 10.0` at run time; the JS engine writes
  `1.0 / 0.0` and `0.0 / 0.0` (`js/builtins_number.ore`).
- **Hit by:** agent (B7), js (B6).
- **Status:** open.

#### 18. `std.math` has no trigonometry and no integer `min`/`max`

- **Need:** the cosines of the JPEG IDCT; the smaller of two byte
  offsets.
- **Repro:** `std.math.cos(x)` is E0081 (module `std.math` has no
  `cos`); `std.math.min(a, b)` with `a, b: usize` is E0023 (expected
  `f64`).
- **Workaround:** the IDCT's constants are written out
  (`image/jpeg.ore`); `minUsize`/`maxUsize` helpers in
  `engine/caret.ore` (a first version converted to `f64` and back).
- **Suggestion:** `sin`, `cos`, `atan2`, `exp`, `log` from libm, and
  `min`/`max`/`clamp` for `T: Integer`.
- **Hit by:** image, engine (B5).
- **Status:** open.

### Modules

#### 19. The root module cannot be imported

- **Need:** share code of the root module (`browser/*.ore`: `main.ore`,
  `style.ore`, `dump.ore`, ...) with a sub-module.
- **Repro:** a directory program `ri/` whose `ri/agent/agent.ore` says
  `import ri`: `error[E0081]: cannot find module `ri` (no directory
  `ri/ri`; ...)`. Imports resolve below the root directory, and the root
  has no name.
- **Workaround:** none needed in practice: put shared code in a
  sub-module. `engine/` is one, and `agent/` can import it. At the B7
  merge, `agent/session.ore` copied `parseDocument`, `relativeUrl` and
  `agentSheets`, although `engine` already had `documentOf`,
  `urlOfInput` and `authorSheets`; ef319a6 replaced the copies with
  `import engine`. The root's own `load`/`toUrl`/`fileUrl` in
  `main.ore` still repeat `engine.documentOf`/`fileUrl`; the root can
  import `engine`, so that is browser cleanup, not a compiler gap.
- **Hit by:** agent (B7).
- **Status:** open (verified still impossible), low impact.

### Smaller things

#### 20. No repeat form for array initializers

- **Repro:** `[4]i64{7} ** 4` is E0006 (expected expression, found `*`).
- **Workaround:** an array of 13 `List`s (`free: [13]List([]mut T)` in
  `js/slabs.ore`) has 13 written-out initializers. A repeat form (Zig's
  `**`) would do.
- **Hit by:** js (B6).
- **Status:** open.

#### 21. No tuples or multiple results

- **Workaround:** small structs carry pairs out of functions (`MinMax`,
  `Flow`, `OpenFrag`, `KindOf` in layout). Fine, a bit verbose.
- **Hit by:** layout (B3).
- **Status:** open.

#### 22. No `@embedFile`

- **Need:** the user-agent style sheet as a plain `.css` file, and the
  900 lines of JavaScript that every page runs first
  (`engine/prelude.ore`) as a `.js` file that tools can check. Line
  strings (entry 40) now cover most of this need: the prelude is one,
  written from a `.js` file during development.
- **Hit by:** css (B2), engine (B6).
- **Status:** open.

#### 23. No discard `_ = f()`

- **Repro:** `_ = f()` is E0018 (`undefined variable _`).
- **Workaround:** a dropped result is fine as a statement
  (`try self.intern(s)`), so this only surprised.
- **Hit by:** js (B6).
- **Status:** open.

#### 24. Maps cannot have struct keys (a `Handle`)

- **Need:** the state of form controls and the decoded image of each
  element, keyed by node (`Handle(Node)`).
- **Repro:** `IntMap(NodeId, Control)` does not satisfy `K: Integer`;
  there is no `Map(K, V)` for structs with `==`.
- **Workaround:** `IntMap(u32, ...)` keyed by `handle.index`, with the
  generation stored in the value and compared by hand
  (`Controls.get` in `dom/controls.ore`, `layout.Images`), as the spec
  advises.
- **Hit by:** dom, layout (B5).
- **Status:** open (by the spec today).

## Declined by design

#### 25. A struct's methods must all be in its body (one file)

- **Need:** split a large type's code across the files of its module.
- **Workaround:** free functions taking `*mut S`: the parser, selector
  and cascade code of `css.Sheet` (`css.parseSheet(&mut sheet, src,
  media)`, not `sheet.parse(src)`); the chrome's `drawToolbar(ui)`
  because `Ui`'s methods are all in `ui/window.ore`; JS built-ins such
  as `fn arrayPush(it: *mut Interp, a: Args)`. `js/interp.ore` is still
  about 3300 lines with ~150 methods. The state of form controls is a
  field of `dom.Document`, but its operations live in
  `dom/controls.ore` as `dom.valueOf(doc, el)`, `dom.setChecked(doc,
  el, on)` (not `doc.valueOf(el)`), so that `dom.ore` stays the tree.
- **Hit by:** css (B2), ui (B4), dom (B5), js (B6).
- **Status:** declined: decisions.md ("A struct's methods stay in its
  body"), documented in M9.6. Of the declined items this one costs the
  most code.

#### 26. A top-level name in one file blocks locals in every file of the module

- **Repro:** with `fn px(v: f64) f64` in `app/a.ore`, `let px = v / 2.0`
  in `app/main.ore` is E0016 (no shadowing, spec §6).
- **Workaround:** distinctive names for module-level helpers
  (`numberOr` in css, `alen`, `appendWtf8` in js). Cases: `px` in
  `css/cascade.ore` broke locals in `media.ore` and `values.ore`; `fn
  screen` in `ui/chrome.ore` broke `let screen` in `ui/window.ore`; `fn
  len` in `js/builtins_array.ore` broke locals in `numconv.ore`; `fn
  label` in `ui/chrome.ore` broke `var label` in `ui/forms.ore`, whose
  `fn edited` broke `var edited` in `ui/window.ore` (renamed
  `afterEdit`). At the B5/B6 merge, `fn method` (a helper of
  `engine/dom_api.ore`) broke `let method` in `engine/forms.ore`
  (renamed `defMethod`); B6 could not name parameters `method` either.
- **Suggestion (B6):** exempt locals from clashing with functions of
  other files.
- **Hit by:** css (B2), ui (B4, B5), js and engine (B6).
- **Status:** the rule is by design; the message that did not name the
  other file is fixed (entry 45).

#### 27. An import name blocks the same local name

- **Repro:** `import url` then `let url = ...` is E0016.
- **Workaround:** the CLI calls its URL `location` (`main.ore`).
- **Hit by:** CLI (B1).
- **Status:** by design (spec §12). M9.5 fixed the message (verified:
  one error, "`std` is already the name of the import at 1:8 (shadowing
  is not allowed: rename the local)", no cascading E0085).

#### 28. Other declined items (clear messages, by design)

- `test` and `where` are keywords, so they cannot name a field or a
  local: the JS AST's `if`/`for` nodes call the condition `cond` (js,
  B6); a local cannot be called `where` (B4).
- `@testAllocator()` works only inside `test` blocks, so shared test
  helpers take an `Allocator` parameter (B1).
- Functions that call each other cannot have an inferred error set
  (E0069): the agent's actions (`clickNode` follows a label to its
  control, `press` clicks) all spell `AllocError!Outcome` (agent, B7).

## Fixed

Verified at HEAD with `target/release/ore`; repros in the entry text or
the milestone's feature form.

#### 29. Sub-modules that import siblings could not be tested

- **Hit by:** url, net, html (B1). **Status:** fixed in M9.5. Verified:
  a program `mt/` with `html` importing `dom`, tests in both: `ore test
  mt` runs `html: html parse` (which calls a private function) and `dom:
  dom node`; `ore test mt --module html` runs one module. `ore test
  mt/html` still fails (imports start at the directory given to `ore`;
  by design), and the message now says to pass `mt --module html`.
  Browser still uses the workaround: `browser/unit_tests.ore` holds the
  `html`/`net` tests in the root module, calling only `pub` functions.

#### 30. A `match` arm could not list several enum variants

- **Hit by:** html, CLI (B1), js (B6). **Status:** fixed in M9.5.
  Verified: `.A, .B => 1` / `.C => 2` prints 1 1 2. Browser still uses
  the workaround: duplicate `.Doctype => {}` / `.Comment => {}` arms in
  `dump.ore`; `=> 7` four times in the precedence table of
  `js/parser.ore`.

#### 31. `?T == T` did not compile, and the hint said to use `as`

- **Hit by:** encoding (B1), js (B6). **Status:** fixed in M9.5 (`?T ==
  T`, `?T == ?T`; the `as` hint only where `as` converts). Verified: `f()
  == E.A`, `f() == E.B`, `null == E.A`, `f() == f()`, `null == null`
  print true false false true true. Browser still uses the workaround:
  `labelIs` in `encoding.ore`, `Interp.hasClass` in js.

#### 32. No `orelse`, `.?` or `unreachable`

- **Hit by:** dom, html (B1), js (B6). **Status:** fixed in M9.5.
  Verified: `find(xs, w) orelse return 99`, `orelse unreachable`, `orelse
  { other(); return }` and `m.? + 1` all compile and run as expected.
  Browser still uses the workaround: `Document.node` keeps its dead
  fallback that reads `Pool`'s private fields (`dom/dom.ore`); the
  adoption agency's `var fi: usize = 0` / `if ... else { ...; return }`;
  JS natives that always throw end in `try it.typeError(...)` and a dead
  `return NONE`.

#### 33. Structs had no `==`; `Handle(T)` had no `eql`

- **Hit by:** dom, html (B1). **Status:** fixed in M9.5 (`==` on structs
  whose fields all compare). Verified: `P { x: 1, ok: true } == P { x: 1,
  ok: true }` is true, `!=` works, and two `Pool(i64)` handles compare
  (`h1 == h1` true, `h1 == h2` false). Browser still uses the
  workaround: `dom.same(a, b)` / `dom.isSame(?a, b)` (19 uses).

#### 34. Array literals had to spell their length

- **Hit by:** html (B1), css (B2). **Status:** fixed in M9.5 (`[_]T{...}`).
  Verified: `[_][]u8{"a", "b", "c"}` has `.len` 3. Browser still uses
  the workaround: hand-counted `[328]Entity` in `html/entities.ore` and
  `[50][]u8` in `css/ua.ore`; the tree builder's tag-name sets are
  space-separated strings searched with `isIn(name, "dd dt li ...")`
  (70 uses) instead of arrays.

#### 35. `return f()` of a void call in a `void` or `E!void` function

- **Hit by:** html (B1). **Status:** fixed in M9.5. Verified: `return
  g()` with `fn g()` in both an `E!void` and a `void` function.

#### 36. `std.log.err` needed a string literal as the format

- **Hit by:** CLI (B1). **Status:** fixed in M9.5. Verified:
  `std.log.err(USAGE)` with `let USAGE = "usage: x"` prints `error:
  usage: x`. Browser still uses the workaround: `std.log.err("{}",
  SHOT_USAGE)` in `shot.ore`, `std.log.err("{}", UI_USAGE)` in
  `ui/window.ore`.

#### 37. A `*mut Self` method could not be called on a temporary

- **Hit by:** CLI (B1). **Status:** fixed in M9.5. Verified:
  `std.text.split("k=v", "=").next() orelse "none"` gives `k`.

#### 38. Two implicit adjustments did not chain (`[]mut T` to `?[]T`, `E![]mut T` to `E![]T`)

- **Hit by:** css (B2). **Status:** fixed in M9.6. Verified: `fn get(s:
  []mut u8) ?[]u8 { return s }`, and an `AllocError![]u8` function
  returning an `AllocError![]mut u8` call. Browser still uses the
  workaround: `let values: []Value = v` in `css/cascade.ore`, `let owned
  = try b.toOwned()` in `css/tokenizer.ore`.

#### 39. `*mut S` did not convert to `?dyn I`

- **Hit by:** js (B6). **Status:** fixed in M9.6. Verified: `e.host =
  &mut h` with `host: ?dyn Host`, then `if e.host |d| { d.finalize(1) }`
  updates `h`. Browser still uses the workaround: `let d: dyn Host =
  &mut h` in `js/unit_tests.ore`.

#### 40. Multi-line string literals

- **Hit by:** css (B2). **Status:** fixed in M9.6 (`\\` line strings).
  Verified: a two-line `\\` constant prints both lines. Browser still
  uses the workaround: `css/ua.ore` is a `[50][]u8` with one rule per
  string. `@embedFile` stays open (entry 22).

#### 41. An array length could not be a constant

- **Hit by:** css (B2). **Status:** fixed in M9.6. Verified: `let N:
  usize = 3`, `[N]u8{1, 2, 3}` and `[N * 2]u8{...}` have lengths 3 and 6.
  Browser still uses the workaround: `css.PROPERTIES` is `[57]Property`
  and `css.NPROPS` repeats the 57.

#### 42. `defer if`

- **Hit by:** css (B2). **Status:** fixed in M9.6. Verified: `defer if
  open { ... }` runs at the end of the scope.

#### 43. A C library could not be optional (libcurl, FreeType, SDL2)

- **Need:** build the browser, and its headless commands and tests,
  where libcurl, FreeType or SDL2 is missing.
- **Hit by:** net (B2), font (B3), ui (B4). **Status:** fixed in M9.6
  (`@linkOptional("name")`, `@hasLink("name")`). Verified: a program
  with `@linkOptional("orenosuchlib")` builds and takes the `else`
  branch of `if @hasLink(...)` (the function calling the missing library
  is left out); the same with `libcurl` calls `curl_global_init` and
  prints `true`. Browser still uses the workaround: `@link("libcurl")`
  in `net/http.ore`, `@link("freetype2")` in `font/freetype.ore`,
  `@link("sdl2")` in `ui/sdl.ore`, so every build still needs all three
  (`net` could fall back to `FetchError.Unsupported`, `font.System` fail
  at run time).

#### 44. One C function had to have one signature in every module

- **Hit by:** engine (B4). **Status:** works at HEAD (no milestone
  lists it). Verified: `getcwd` declared returning `[*]u8` in the root
  and `?[*]u8` in a sub-module; both build and run. Browser still uses
  the workaround: `engine/engine.ore` declares `getcwd` returning
  `[*]u8` like `main.ore`.

#### 45. E0016 did not name the file of a declaration in another file

- **Hit by:** css (B2), ui (B4), js (B6). **Status:** fixed in M9.6.
  Verified: `already declared at sh/a.ore:1:4` (was `at 1:4`, which read
  as a line of the current file). The rule itself stays (entry 26).

#### 46. `@zeroed` of an array of `bool`

- **Hit by:** agent (B7). **Status:** works at HEAD (no milestone
  lists it). Verified: `@zeroed([64]bool)` compiles and
  `flags[3]` is `false`. Browser still uses the workaround: the JSON
  `Writer` keeps its per-level "a value was written" flags in a `u64`
  bitmask (at most 64 levels of nesting). `@zeroed` of a plain struct
  stays open (entry 5).
