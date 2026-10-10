# Compiler and language gaps met by the browser

Things the browser needed that Aye (stage0) or its std does not do, or
does in a way that hurt: the input for the compiler's ergonomics
milestones (M9.5 to M9.9 came from this list). One numbered list,
grouped by theme. Open gaps come first, ordered by what they cost the
browser (performance, correctness, code); then the ones declined by
design; then the fixed ones. Each entry: **Need**, **Repro**,
**Workaround** (what `browser/` does today), **Hit by** (component and
branch: B1 url/net/html/dom/encoding and the CLI, B2 css, B3
font/layout, B4 paint/engine/ui, B5 forms and images, B6 js, B7 agent,
B8 the real web) and **Status**. The framework lane
(`docs/plans/framework.md`) records its gaps here too, with the F step
under **Hit by**.

Every entry was checked again against `target/release/aye` at the M9.8
head (M9.9's re-check) with a small program per entry: the repro, or
the milestone's feature form (`aye run`, `aye test`); M9.9 then fixed
entries 7, 8 and 22. "Browser still uses the workaround" means the old
code is still there and can now be simplified; small local ones were
removed in cff3f60.

## Open

### Allocator and runtime

#### 3. Pointers into a growing `List` or `Pool` dangle

- **Need:** keep a `*mut Node` or `*mut Box` across a call that may add
  to the container.
- **Repro:**

  ```aye
  var xs = List(i64).init(alloc)
  try xs.append(1)
  let items = xs.items()
  for i in 0..100 {
      try xs.append(i as i64)    // may move the elements
  }
  items[0] = 5                   // writes freed memory
  ```

  Since M9.7 (34e41da) debug-mode `General` poisons freed blocks and
  keeps them in a quarantine, so this write is reported when the list
  is freed: `panic: use after free: memory allocated at
  std/list/list.aye:100:39 and freed here was written after it was
  freed`. A read through the stale pointer is not reported (it reads
  the poison bytes), and the report comes at the next check, not at the
  write.
- **Workaround:** refer by handle or index, not by pointer, as
  design.md asks: nodes by `NodeId` (a `Handle(Node)`; 216 `doc.node(h)`
  calls), layout boxes by `usize` index (333 `t.bx(i)` calls in 26
  files, each pointer "valid until the next box is added"). Copy fields
  out before growing (`cloneElement` in `html/parser.aye`), and fetch the
  pointer again after any call that may grow.
- **Hit by:** dom, html (B1), layout (B3).
- **Status:** partly fixed (writes, in debug mode). A compile-time rule
  and a fat pointer with a generation are declined in decisions.md
  (P2.3). Open: a stale read is not reported, and a write is reported
  late. AddressSanitizer would report both at the access (design.md
  names it for this), but debug-mode `General` (and `aye test`'s
  allocator) keep freed blocks in a quarantine that is still allocated
  memory for AddressSanitizer. Proposed step: under
  `__has_feature(address_sanitizer)` the runtime in `src/cgen.rs`
  poisons a quarantined block (`ASAN_POISON_MEMORY_REGION`) and
  unpoisons it before its poison check and `free`; a golden test in
  `tests/golden.rs`, like the ThreadSanitizer one (skipped where clang's
  AddressSanitizer does not run), builds the repro above with
  `--cflags -fsanitize=address` in debug and in safe mode and expects
  `heap-use-after-free` at the line of the read; the spec's `--cflags`
  line names `-fsanitize=address`. Verify: `cargo test --test golden`.

### C interop

#### 49. An `extern fn` cannot have an Aye name other than its C symbol

- **Need:** a module whose pub API reuses a C function's name: `std.net`
  exports `listen` and `connect` and calls C's `listen` and `connect`.
- **Repro:**

  ```aye
  extern fn close(fd: i32) i32
  pub fn close(x: i32) i32 { return x }   // E0015: `close` is defined more than once
  ```

- **Workaround:** the C prelude (`src/cgen.rs`) declares the symbols
  under other names (`aye_rt_listen` with `__asm__("listen")`) and
  exports `aye_std_listen` and `aye_std_connect`, which `std/net/net.aye`
  declares. A struct method of the same name is fine (`Conn.read` calls
  C's `read`). `@cimport` also avoids it (`sock.connect`), but needs
  libclang or the cache.
- **Hit by:** O1.3 (`std.net`).
- **Status:** fixed in O1.3a: `extern("connect") fn cConnect(...)` names
  the C symbol. `std/net` calls `listen` and `connect` this way; only
  `aye_std_errno` stays in the prelude (the errno symbol differs per OS).

#### 50. No per-OS declarations

- **Need:** declare a different `extern fn` or struct layout per target
  OS.
- **Workaround:** `aye_std_errno` stays in the C prelude (`src/cgen.rs`)
  because the errno symbol is `__error` on macOS and `__errno_location`
  on Linux. `std/net/net.aye` declares `struct addrinfo` with the fields
  `ai_canonname` and `ai_addr` as `first` and `second` (their order
  differs per OS) and picks at run time with `isDarwin()`. A field
  whose name differs per OS cannot be picked at run time: both branches
  are checked against the host's `@cimport`. So `std.fs.mtime` reads the
  modification time in the C prelude (`aye_std_mtime` in `src/cgen.rs`,
  the offset of `st_mtimespec` on macOS and `st_mtim` on Linux), and
  `browser/net/archive.aye` calls it; it takes `d_name`'s length with
  `strlen`, not macOS's `d_namlen` (X1.1).
- **Hit by:** O1.3b (`std.net`: errno, `struct addrinfo`); X1.1
  (`browser/net/archive.aye`: `st_mtimespec`, `d_namlen`); O1.5
  (`std.fs.stat`, `readDir`, `mkdir`, `remove`, `rename`: the layouts of
  `struct stat` and `struct dirent` and ENOTEMPTY's value differ, so
  `aye_std_lstat`, `aye_std_readdir` and `aye_std_pathop` are in the C
  prelude).
- **Status:** open. X1.1 chose the std helper over a language feature:
  per-OS declarations need a target-OS constant that the checker uses to
  skip the other OS's code (parser, checker, cgen, spec and tests), more
  than one step; the helper is one C function and a std function that
  programs want anyway.

#### 54. `@cimport` of a glibc header takes the wrong files

- **Need:** `@cimport("math.h")` and `@cimport("pthread.h")` declare the
  same things on Ubuntu 24.04 (glibc) as on macOS.
- **Repro:** on Linux, `aye cimport math.h` has constants but no
  function (`m.sqrt`: E0081), and a program with
  `@cimport("pthread.h")` that also uses `std.time` (`browser`) fails
  with E0096: `nanosleep` and `clock` are declared twice with different
  signatures.
- **Cause:** for a system header, the generator
  (`src/cimport/generate.rs`) took the header itself and the headers of
  the header's directory that it includes itself. glibc declares the
  math functions in `bits/mathcalls.h` (through `__MATHCALL`), in
  `/usr/include/aarch64-linux-gnu/bits/`, another directory: none came
  in. glibc's `pthread.h` and `time.h` both sit in `/usr/include`, and
  `pthread.h` includes `time.h` first, so `time.h`'s `clock` (`clock_t`
  is `long`) and `nanosleep` (`struct timespec *`) came in and clashed
  with `browser/js/interp.aye`'s `clock() u64` and `std/time`'s
  `nanosleep([*]i64, [*]i64)`. macOS escaped by accident: its
  `/usr/include/pthread.h` is a link to `pthread/pthread.h`, so its
  directory after `canonicalize` is `pthread/`.
- **Fix:** one rule on both OSes (`src/cimport/generate.rs`). A system
  header brings its own declarations and those of its parts that it
  includes itself: internal headers (a name with a leading `_`, macOS's
  `_time.h`; or a `bits/` directory, glibc's `bits/mathcalls.h`), and
  the headers of its directory when that is not the include directory it
  was found in (`SDL2/SDL.h`; macOS's `pthread.h` → `pthread/`). The
  other headers of `/usr/include` are headers of their own. On macOS the
  output stays the same but for `zlib.h` (no longer `zconf.h`'s
  `MAX_WBITS`, `MAX_MEM_LEVEL` and unused typedefs) and `unistd.h` (no
  `gethostuuid`, from `gethostuuid.h`), and gains `sys/stat.h`'s
  `S_IF*` (from `sys/_s_ifmt.h`).
- **Hit by:** X1.3 (first Linux run: `aye check browser`, golden
  `cimport/system.aye`, spec `aye_blocks_compile`, `codes` E0112).
- **Status:** fixed in X1.3; `tests/cimport/threads.aye` imports
  `pthread.h` beside `std.time`.

#### 51. No way to print an error or enum tag name

- **Need:** get the name of an error tag or enum tag as a string, for
  example to print it.
- **Workaround:** write a `match` that returns the string for each tag.
  `@println` takes one argument, so the name cannot be passed beside a
  tag value.
- **Hit by:** the std.http golden (`tests/cases/std_http.aye`), which
  prints names through its own `match`.
- **Status:** fixed in O1.7: `@tagName(x)` gives the name of an error,
  enum or tagged-union tag as a `[]u8`; `std_http` uses it.

#### 52. A method cannot be used as a function value

- **Need:** name a method as a plain function value, for example
  store `Self.f` as a handler.
- **Workaround:** declare the function at module level and use that.
  `Self.f` gives E0074.
- **Hit by:** std.http `Server` (the handler is stored as a plain fn).
- **Status:** open.

#### 53. Nested structs and generic methods are rejected

- **Need:** declare a struct inside another struct or function, and
  give a method its own type parameters.
- **Workaround:** nested structs give E0006 and generic methods give
  E0076. std.http `Server` is non-generic and erases `T` with
  `@ptrCast`, as `std.thread.spawn` does.
- **Hit by:** std.http `Server`.
- **Status:** open.

## Declined by design

#### 5. A local cannot start uninitialized; `@zeroed` makes only C structs

- **Need:** a 35-field layout `Box` that is mostly results starting at
  0; locals set in branches.
- **Repro:**

  ```aye
  struct S { a: i64, b: ?*S }

  pub fn main() {
      var x: i64                            // E0013: `var x` needs an initializer
      let s = @zeroed(S)                    // E0093: only an `extern struct` has a defined all-zero value
  }
  ```

- **Workaround:** `Tree.newBox` lists all 35 fields; dummy initializers
  such as `var fi: usize = 0` in the adoption agency algorithm
  (`html/parser.aye`).
- **Hit by:** html (B1), layout (B3), engine (B4), agent (B7).
- **Status:** the need is fixed, the two forms are declined. Struct field
  defaults (`x: f64 = 0.0`) came in M9.7 (dbe20f8; verified: `B { y:
  2.0 }` with `x` defaulted), which is what `Box` needed; the browser
  still lists every field. Locals without an initializer and `@zeroed`
  of an Aye struct are declined in decisions.md (M9.9): defaults, and
  `catch`/`orelse` blocks with a value, cover the cases met.

#### 19. The root module cannot be imported

- **Need:** share code of the root module (`browser/*.aye`) with a
  sub-module.
- **Workaround:** none needed: shared code goes into a sub-module
  (`engine/`), which both the root and `agent/` import. The root's own
  `load`/`toUrl`/`fileUrl` in `main.aye` still repeat
  `engine.documentOf`/`fileUrl`; that is browser cleanup.
- **Hit by:** agent (B7).
- **Status:** declined in M9.7 (67768e8; decisions.md: the import would
  always close a cycle). `import ri` from a module of the program `ri`
  is E0083 with the structure to use instead.

#### 21. No tuples or multiple results

- **Workaround:** small structs carry pairs out of functions (`MinMax`,
  `Flow`, `OpenFrag`, `KindOf` in layout). Fine, a bit verbose. About 30
  structs of two or three fields are returned by about 60 functions
  (`Flow` by 15, `MinMax` by 6); about 5 functions return a second value
  through a `*mut` parameter (`tileSize` in `paint/background.aye`).
- **Hit by:** layout (B3).
- **Status:** declined in decisions.md (P2.3, "Compiler gaps closed
  out"): a named struct is the one way, and its field names say what
  each value means. `fn f() (i64, i64)` stays E0006.

#### 25. A struct's methods must all be in its body (one file)

- **Need:** split a large type's code across the files of its module.
- **Workaround:** free functions taking `*mut S`: the parser, selector
  and cascade code of `css.Sheet` (`css.parseSheet(&mut sheet, src,
  media)`, not `sheet.parse(src)`); the chrome's `drawToolbar(ui)`
  because `Ui`'s methods are all in `ui/window.aye`; JS built-ins such
  as `fn arrayPush(it: *mut Interp, a: Args)`. `js/interp.aye` is still
  about 3300 lines with ~150 methods. The state of form controls is a
  field of `dom.Document`, but its operations live in
  `dom/controls.aye` as `dom.valueOf(doc, el)`, `dom.setChecked(doc,
  el, on)` (not `doc.valueOf(el)`), so that `dom.aye` stays the tree.
- **Hit by:** css (B2), ui (B4), dom (B5), js (B6).
- **Status:** declined: decisions.md ("A struct's methods stay in its
  body"), documented in M9.6. Of the declined items this one costs the
  most code.

#### 26. A top-level name in one file blocks locals in every file of the module

- **Repro:** with `fn px(v: f64) f64` in `app/a.aye`, `let px = v / 2.0`
  in `app/main.aye` is E0016 (no shadowing, spec §6).
- **Workaround:** distinctive names for module-level helpers
  (`numberOr` in css, `alen`, `appendWtf8` in js). Cases: `px` in
  `css/cascade.aye` broke locals in `media.aye` and `values.aye`; `fn
  screen` in `ui/chrome.aye` broke `let screen` in `ui/window.aye`; `fn
  len` in `js/builtins_array.aye` broke locals in `numconv.aye`; `fn
  label` in `ui/chrome.aye` broke `var label` in `ui/forms.aye`, whose
  `fn edited` broke `var edited` in `ui/window.aye` (renamed
  `afterEdit`). At the B5/B6 merge, `fn method` (a helper of
  `engine/dom_api.aye`) broke `let method` in `engine/forms.aye`
  (renamed `defMethod`); B6 could not name parameters `method` either.
- **Suggestion (B6):** exempt locals from clashing with functions of
  other files.
- **Hit by:** css (B2), ui (B4, B5), js and engine (B6).
- **Status:** the rule is by design; the message that did not name the
  other file is fixed (entry 45).

#### 27. An import name blocks the same local name

- **Repro:** `import url` then `let url = ...` is E0016.
- **Workaround:** the CLI calls its URL `location` (`main.aye`).
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

Verified with `target/release/aye` at the M9.8 head (or at the fix, for
later entries); repros in the entry text or the milestone's feature
form. The commit is the one that fixed the gap.

#### 1. Debug-mode `General` made many small allocations quadratic; Aye code could not implement `Allocator`

- **Need:** a garbage-collected heap that allocates and frees millions of
  small slices (property tables, elements, environments, strings).
- **Repro:** 200000 `alloc`/`free` pairs of 16 bytes with debug-mode
  `General`: each `free` scanned every record from the oldest (2.2 s
  for 50000 pairs; `fib(22)` in the first JS engine took 40 s, almost
  all in `aye_general_free`).
- **Hit by:** js (B6). **Status:** fixed in M9.7: O(1) alloc and free
  with a header per block and a quarantine (34e41da), and a struct with
  `alloc` and `free` methods is an `Allocator` (ee638c5). Verified: the
  200000 pairs run in 0.9 s including the compile. Browser still uses
  the workaround: `js/slabs.aye` (size-class free lists cut from
  chunks) for the heap's slices.

#### 2. The C stack was the only stack (no coroutines)

- **Need:** deep recursion, and generators or async functions that stop
  in the middle of the tree-walking interpreter. The interpreter uses
  about 7 KB of C stack per JavaScript call in a debug build, so 1100
  nested calls overflowed the 8 MB main thread.
- **Repro:** a recursion with a 1 KB local and no base case reached:
  exit status 139 (SIGSEGV), no Aye diagnostic, before M9.7. Now it
  prints `panic: stack overflow (deep recursion: run it on a thread with
  a larger stack, std.thread.spawnWithStack)`.
- **Workaround:** `js.runOnBigStack` starts the script on a thread with
  a 1 GB stack; the engine throws a RangeError at 10000 frames, like
  Node, and measures the C stack it has used (`Interp.markStackBase`).
  A generator or async function that stops in the middle of the tree
  walk runs on a `std.coro` coroutine (`js/coroutine.aye`, 256 MB
  reserved), resumed on the engine's thread (M12; before, a thread per
  live generator and two `std.sync.Channel`s).
- **Hit by:** js (B6, B8).
- **Status:** fixed. M9.7 made an overflow panic in debug and safe
  mode (756cc94) and added `std.thread.spawnWithStack` (915a6c0); M12
  added stackful coroutines, `std.coro` (9bc621a), and the engine runs
  generators and async functions on them (34e46aa). Deep recursion
  runs on a thread or a coroutine with a large stack.

#### 4. Every checked operation was a call in debug builds

- **Need:** decode images (inflate, Huffman, IDCT, color conversion) at
  a usable speed in the browser, which is built in debug mode.
- **Hit by:** image (B5). **Status:** fixed in M9.8 (c6a8cfb): checks
  are emitted inline; a decode-like loop went from 1.52 s to 0.54 s in
  debug mode, and `@setSafety(false)` (25cac16) turns a block's checks
  off. The restructured hot loops of `image/` stay (they are faster
  anyway).

#### 6. At most 64 instances of a generic in a whole program

- **Need:** a `List` of whatever each part of the browser keeps: the
  program passed 64 distinct `List(T)` when B5 and B6 were merged
  (E0080, "more than 64 instances of `std.list.List`", blaming whichever
  line was the 65th).
- **Hit by:** engine, agent (B6 merging B5); B7 worked around it until
  the fix (the window's tabs and agents add no new `List` type).
- **Status:** fixed in M9.8 (2eedc32): no limit on the instances of a
  generic. Browser still uses the workaround: localStorage keeps its
  items as three strings each in a `List([]mut u8)`
  (`engine/window_api.aye`).

#### 7. A `dyn` could not be turned back into its struct; function values cannot carry state

- **Need:** the JavaScript engine's native functions are a table of
  `fn(*mut Interp, Args) JsError!Value`; the DOM bindings' natives need
  the page's `Script`, which the engine only knows as a `dyn Host`.
- **Hit by:** engine (B6). **Status:** fixed in M9.9 (8a1d3e9:
  `@downcast(*mut S, d)`, a `?*mut S` checked against `d`'s vtable).
  Verified: `@downcast(*mut Page, h)` gives the page, `null` for a
  `dyn Host` made from another struct. Removed in cff3f60: `Interp.host_data`
  and the unchecked `@ptrCast` in `scriptOf` (`engine/script.aye`), which
  is now `@downcast(*mut Script, it.host.?).?`. Closures stay out by
  spec (§4): settings every page shares are still module-level `var`s.

#### 8. A `catch` block could not run statements and then give a value

- **Need:** "report the error, then go on with a fallback".
- **Hit by:** engine, ui (B4). **Status:** fixed: `if f() |v| { } else
  |e| { }` in M9.7 (f7f4754), and a `catch` or `orelse` block that ends
  with its value in M9.9 (d118815). Verified: `let v = parse(s) catch
  |e| { @println("report"); old }`. Browser still uses the workaround:
  `finalUrl` in `engine/engine.aye` picks its input before parsing.

#### 9. Error values had no `==`

- **Need:** "is this fetch error OutOfMemory (pass it on) or something to
  report?"
- **Hit by:** agent (B7), image (B5). **Status:** fixed in M9.7
  (eedf75d). Verified: `if e == E.A` in a `catch |e|` block. The
  browser's workaround (`image/tests.aye` mapped each error to a number
  to compare them) was removed in cff3f60.

#### 10. A `match` arm could not produce the error union of an enclosing `return`

- **Hit by:** agent (B7). **Status:** fixed in M9.7 (f7f4754: `return
  match` whose arms are error unions, errors or values). Verified:
  `return match a { .Back => back(n) .Fwd => fwd(n) }` in an `E!i64`
  function. The `try` in every arm of
  `applyAction` (`agent/actions.aye`) was removed in cff3f60.

#### 11. Mapping one error set onto another took a `catch` block at every call

- **Need:** the bindings call engine functions that return
  `JsError!Value` from code that returns `AllocError!...`.
- **Hit by:** engine (B6). **Status:** works since M9.5 (b78afde:
  `catch` takes `return`, `break` or `continue` like `orelse`).
  Verified: `let v = g() catch return AllocError.OutOfMemory`. The
  wrappers in `engine/script.aye` (`wrapFor`, `stringFor`, `jsPush`) use
  it since cff3f60: one line each instead of a block.

#### 12. `orelse` did not chain

- **Hit by:** agent (B7). **Status:** fixed in M9.7 (eedf75d). Verified:
  `a() orelse a() orelse "none"` prints `none`. The parentheses
  in `agent/markdown.aye` were removed in cff3f60.

#### 13. No test for one variant of an enum with payloads

- **Need:** "did the default action do nothing?" for an
  `engine.Activation`, whose other variants carry payloads.
- **Hit by:** ui, agent (B5). **Status:** fixed in M9.8 (6af69f6: `x is
  .V`). Verified: `if a is .Nothing` on an enum whose other variant has
  fields. `ui/forms.aye` tests `act is .Nothing` since cff3f60.

#### 14. No reading of a stream piece by piece (stdin lines)

- **Need:** the agent servers answer one JSON line at a time on stdin.
- **Hit by:** agent (B7). **Status:** fixed in M9.7 (d2a509d:
  `File.reader(alloc)`, `Reader.readLine()`, `File.read(buf)`).
  Verified: `std.fs.stdin().reader(a)` counts three piped lines. Browser
  still uses the workaround: `LineReader` in `agent/server.aye` over an
  `extern fn read`.

#### 15. No C `long`

- **Hit by:** font (B3), js (B8). **Status:** fixed in M9.7 (1872d6b:
  `c_int`, `c_long`, `c_ulong` and the other C integer names).
  Verified: `extern fn labs(x: c_long) c_long`. `font/freetype.aye`
  spells `FT_Long`, `FT_Pos` and `FT_Fixed` as `c_long` since cff3f60.
  Browser still uses the workaround: `js/interp.aye` declares `clock`
  returning `u64`.

#### 16. No `extern union`

- **Hit by:** ui (B4). **Status:** fixed in M9.7 (1872d6b). Verified:
  `extern union U { a: i32, b: i64 }`, `U { b: 5 }.a` is 5. Since
  cff3f60 `ui/sdl.aye` declares `SDL_Event` as an `extern union` of the
  events the window reads, and `ui/window.aye` and `ui/script.aye` take
  `&ev.key`, `&ev.motion`, ... instead of `@ptrCast`.

#### 17. `std.math` had no infinity or NaN

- **Hit by:** agent (B7), js (B6). **Status:** fixed in M9.7 (d2a509d:
  `std.math.inf()`, `nan()`, `isNan`, `isInf`). Verified. `agent/json.aye`
  uses `isNan`/`isInf` since cff3f60. Browser still uses the workaround:
  `1.0 / 0.0` and `0.0 / 0.0` in the JS engine (about 35 places).

#### 18. `std.math` had no trigonometry and no integer `min`/`max`

- **Hit by:** image, engine (B5). **Status:** fixed in M9.8 (bdd7d49:
  libm's `sin`, `cos`, `atan2`, `exp`, `log` and more, and
  `minOf`/`maxOf`/`absOf`/`clampOf` for any integer type). Verified:
  `std.math.cos(0.0)` and `std.math.minOf(usize, a, b)`. `minUsize`/`maxUsize`
  in `engine/caret.aye` were replaced in cff3f60; the IDCT's written-out
  constants in `image/jpeg.aye` stay (no need to compute them).

#### 20. No repeat form for array initializers

- **Hit by:** js (B6). **Status:** fixed in M9.7 (16ac1d9: `[N]T{value,
  ..}`). Verified: `[4]i64{7, ..}[3]` is 7. The 13 written-out
  initializers in `js/slabs.aye` were replaced in cff3f60.

#### 22. No `@embedFile`

- **Need:** the user-agent style sheet as a plain `.css` file, and the
  900 lines of JavaScript that every page runs first as a `.js` file
  that tools can check.
- **Hit by:** css (B2), engine (B6). **Status:** fixed in M9.9 (d286597:
  `@embedFile("path")`, relative to the source file). Verified: a
  directory program embeds files of its root and of a sub-module.
  Browser still uses the workaround: `engine/prelude.aye` is a line
  string written from a `.js` file, and `css/ua.aye` an array of rules
  (each change is hundreds of lines, left for the owners of those
  files).

#### 23. No discard `_ = f()`

- **Hit by:** js (B6). **Status:** fixed in M9.7 (eedf75d). Verified.
  No workaround in `browser/`.

#### 24. Maps could not have struct keys (a `Handle`)

- **Need:** the state of form controls and the decoded image of each
  element, keyed by node (`Handle(Node)`).
- **Hit by:** dom, layout (B5). **Status:** fixed in M9.8 (7b41ec9:
  `std.map.HashMap(K, V)` for any `K` that `==` compares, `@hash`).
  Verified: a `HashMap(K, i64)` with a two-field struct key. Browser
  still uses the workaround: `IntMap(u32, ...)` keyed by `handle.index`
  with the generation compared by hand (`dom/controls.aye`,
  `layout/controls.aye`).

#### 29. Sub-modules that import siblings could not be tested

- **Hit by:** url, net, html (B1). **Status:** fixed in M9.5. Verified:
  a program `mt/` with `html` importing `dom`, tests in both: `aye test
  mt` runs `html: html parse` (which calls a private function) and `dom:
  dom node`; `aye test mt --module html` runs one module. `aye test
  mt/html` still fails (imports start at the directory given to `aye`;
  by design), and the message now says to pass `mt --module html`.
  Browser still uses the workaround: `browser/unit_tests.aye` holds the
  `html`/`net` tests in the root module, calling only `pub` functions.

#### 30. A `match` arm could not list several enum variants

- **Hit by:** html, CLI (B1), js (B6). **Status:** fixed in M9.5.
  Verified: `.A, .B => 1` / `.C => 2` prints 1 1 2. `dump.aye`
  merges its two empty arms since cff3f60. Browser still uses the
  workaround: `=> 7` four times in the precedence table of
  `js/parser.aye`.

#### 31. `?T == T` did not compile, and the hint said to use `as`

- **Hit by:** encoding (B1), js (B6). **Status:** fixed in M9.5 (`?T ==
  T`, `?T == ?T`; the `as` hint only where `as` converts). Verified: `f()
  == E.A`, `f() == E.B`, `null == E.A`, `f() == f()`, `null == null`
  print true false false true true. `labelIs` in `encoding.aye` was
  removed in cff3f60. Browser still uses the workaround: `Interp.hasClass`
  in js.

#### 32. No `orelse`, `.?` or `unreachable`

- **Hit by:** dom, html (B1), js (B6). **Status:** fixed in M9.5.
  Verified: `find(xs, w) orelse return 99`, `orelse unreachable`, `orelse
  { other(); return }` and `m.? + 1` all compile and run as expected.
  Browser still uses the workaround: `Document.node` keeps its dead
  fallback that reads `Pool`'s private fields (`dom/dom.aye`); the
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
  the workaround: hand-counted `[328]Entity` in `html/entities.aye` and
  `[50][]u8` in `css/ua.aye`; the tree builder's tag-name sets are
  space-separated strings searched with `isIn(name, "dd dt li ...")`
  (70 uses) instead of arrays.

#### 35. `return f()` of a void call in a `void` or `E!void` function

- **Hit by:** html (B1). **Status:** fixed in M9.5. Verified: `return
  g()` with `fn g()` in both an `E!void` and a `void` function.

#### 36. `std.log.err` needed a string literal as the format

- **Hit by:** CLI (B1). **Status:** fixed in M9.5. Verified:
  `std.log.err(USAGE)` with `let USAGE = "usage: x"` prints `error:
  usage: x`. The `std.log.err("{}",
  X_USAGE)` calls of `main.aye`, `shot.aye`, `ui/window.aye`,
  `ui/socket.aye` and `agent/server.aye` were replaced in cff3f60.

#### 37. A `*mut Self` method could not be called on a temporary

- **Hit by:** CLI (B1). **Status:** fixed in M9.5. Verified:
  `std.text.split("k=v", "=").next() orelse "none"` gives `k`.

#### 38. Two implicit adjustments did not chain (`[]mut T` to `?[]T`, `E![]mut T` to `E![]T`)

- **Hit by:** css (B2). **Status:** fixed in M9.6. Verified: `fn get(s:
  []mut u8) ?[]u8 { return s }`, and an `AllocError![]u8` function
  returning an `AllocError![]mut u8` call. Both workarounds in
  `css/` were removed in cff3f60.

#### 39. `*mut S` did not convert to `?dyn I`

- **Hit by:** js (B6). **Status:** fixed in M9.6. Verified: `e.host =
  &mut h` with `host: ?dyn Host`, then `if e.host |d| { d.finalize(1) }`
  updates `h`. `js/unit_tests.aye` assigns `&mut h`
  since cff3f60.

#### 40. Multi-line string literals

- **Hit by:** css (B2). **Status:** fixed in M9.6 (`\\` line strings).
  Verified: a two-line `\\` constant prints both lines. Browser still
  uses the workaround: `css/ua.aye` is a `[50][]u8` with one rule per
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
  in `net/http.aye`, `@link("freetype2")` in `font/freetype.aye`,
  `@link("sdl2")` in `ui/sdl.aye`, so every build still needs all three
  (`net` could fall back to `FetchError.Unsupported`, `font.System` fail
  at run time).

#### 44. One C function had to have one signature in every module

- **Hit by:** engine (B4). **Status:** works at HEAD (no milestone
  lists it). Verified: `getcwd` declared returning `[*]u8` in the root
  and `?[*]u8` in a sub-module; both build and run. Browser still uses
  the workaround: `engine/engine.aye` declares `getcwd` returning
  `[*]u8` like `main.aye`.

#### 45. E0016 did not name the file of a declaration in another file

- **Hit by:** css (B2), ui (B4), js (B6). **Status:** fixed in M9.6.
  Verified: `already declared at sh/a.aye:1:4` (was `at 1:4`, which read
  as a line of the current file). The rule itself stays (entry 26).

#### 46. `@zeroed` of an array of `bool`

- **Hit by:** agent (B7). **Status:** works at HEAD (no milestone
  lists it). Verified: `@zeroed([64]bool)` compiles and
  `flags[3]` is `false`. The JSON `Writer`
  (`agent/json.aye`) keeps its per-level flags in `has: [64]bool =
  @zeroed([64]bool)` since cff3f60, instead of a `u64` bitmask. `@zeroed` of
  a plain struct is declined (entry 5).

#### 47. A pointer to an array could not be indexed

- **Need:** fill a C struct held as a byte array (a `sockaddr_un`, a
  `[110]u8` local) in a helper.
- **Repro:** `fn set(a: *mut [4]u8) { a[0] = 7 }` was E0043 ("cannot
  index into a value of type `*mut [4]u8`"); Zig and Rust index through
  the pointer.
- **Hit by:** ui (B7). **Status:** fixed (319aa97): indexing
  dereferences one pointer level when the pointee is an array, like
  field access (`a[i]`, `a[i] = v`, bounds-checked; writing through a
  `*[N]T` is E0033). `ui/socket.aye`'s `unixAddr` takes a
  `*mut [110]u8` since cff3f60. No other
  code in `browser/` works around it (helpers that take `x[..]` take a
  slice by design).

#### 48. Boolean-to-number casts were unavailable

- **Need:** store comparison results in an unboxed numeric expression
  stack without an enum value or a helper call per comparison.
- **Repro:** `fn f(b: bool) u8 { return b as u8 }` was E0037
  ("only numeric casts are supported").
- **Hit by:** js performance (B8). **Status:** fixed in P2.1 (5eef500):
  `b as T` gives 0 or 1 for every integer type and an Integer-bound
  `T`, also in constants (spec, Conversions). Int to bool and bool to
  float stay E0037, with hints (`n != 0`; `(b as u8) as f64`).
  `js/fast_expr.aye` stores booleans and comparison results as
  `(b as u8) as f64` since P2.2.
