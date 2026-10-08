# Stage0 milestones

| Milestone | Status | Date | What is left |
| --- | --- | --- | --- |
| M1 Data modeling | done | 2026-10-05 | nothing |
| M2 Error handling | done | 2026-10-05 | nothing |
| M3 Memory and tests | done | 2026-10-05 | nothing |
| M4 comptime generics, Pool, Handle | done | 2026-10-05 | nothing |
| M5 Modules and first real tool | done | 2026-10-08 | nothing; gate rerun passed (P2.5c) |
| M6 Operators, constants, control flow | done | 2026-10-06 | nothing |
| M7 C interop for real libraries | done | 2026-10-06 | nothing |
| M8 std for real programs | done | 2026-10-06 | nothing |
| M9 Formatter and docs | done | 2026-10-06 | nothing |
| M9.5 Ergonomics from the browser | done | 2026-10-06 | nothing |
| M9.6 Ergonomics, round 2 | done | 2026-10-06 | nothing |
| M9.7 Ergonomics and runtime, round 3 | done | 2026-10-06 | nothing |
| M9.8 Round 4 and design leftovers | done | 2026-10-06 | nothing |
| M9.9 Round 5 | done | 2026-10-06 | nothing |
| M10 Threads | partial | 2026-10-06 | ThreadSanitizer run: not possible on this Mac |
| M11 cimport | done | 2026-10-07 | nothing |
| M12 Coroutines | done | 2026-10-07 | nothing |

Each milestone ends with an acceptance program that must pass `cargo test`. A
milestone is done when its acceptance tests pass. Anything not listed is out of
scope for that milestone and goes to a later one. Every milestone also has one
toolchain item from the "AI 協作" section of `design.md`.

Work through the milestones in order. Before you start one, write its
acceptance tests first. They fail at that point; the milestone is done when they
pass.

## M1: Data modeling

- **Language:** `*T` and `*mut T` (non-null, with `&` and `&mut`), `?T` (with
  `null` and `if x |v| { }`), and `enum` with payloads plus exhaustive `match`.
- **Toolchain:** parser and sema error recovery, so that `aye check` reports
  all errors instead of only the first one.
- **Acceptance:**
  - Example 2 in `design.md` (`Shape` and `area`) compiles and runs without
    changes, apart from replacing `std.math.pi` with a literal.
  - A missing `match` arm is a compile error that names the missing variant.
  - A file with three independent errors reports all three.
- **Out of scope:** generics, error unions, and `match` on anything other than
  an enum.

## M2: Error handling

- **Language:** `error` sets, `E!T` and `!T`, `try`, `catch |e| { }`,
  `errdefer`, and `||` for merging error sets. A `pub` function must spell out
  its error set. `main` may return `!void`.
- **Toolchain:**
  - A stable error code (`E0001` and so on) on every diagnostic, also in the
    `--json` output.
  - `aye explain <code>`.
  - In debug mode, an error return trace printed when `main` returns an error.
- **Acceptance:**
  - A program that parses integers from a string literal: one golden test per
    error path, plus a test that `errdefer` runs only on the error path.
  - A `match e` on errors that misses a case is a compile error.
- **Out of scope:** error payloads (`*mut Diagnostics` comes later, with std).

## M3: Memory and tests

- **Language:** `interface` and `dyn`, which are needed for `Allocator`.
  Built-in `General` (backed by libc, with leak and double-free detection in
  debug mode) and `Arena`. `test "name" { }` blocks.
- **Toolchain:** `aye test [--filter] [--json]`. A test that leaks memory fails.
- **Acceptance:**
  - A program that builds a linked list with `Arena` and frees it with a single
    `deinit`.
  - A deliberately leaking test is reported as failed and names the allocation
    site.
- **Out of scope:** `Pool` and `Handle` (these need generics).

## M4: comptime generics, Pool, and Handle

- **Language:** `comptime T: type` parameters, functions that return `type`,
  and `where` constraints, checked when the generic is defined.
- **Toolchain:** fix-it suggestions in `--json` output, and `aye fix` to apply
  them.
- **Acceptance:**
  - Example 3 in `design.md` (`Pool(T)` and `Handle(T)`) compiles.
  - Dereferencing a stale handle returns `null` instead of reading garbage.
- **Out of scope:** comptime I/O and general comptime execution beyond type
  functions.

## M5: Modules and first real tool (dogfooding gate)

- **Language:** directory modules, `import`, `pub` visibility, and a minimal
  `std` (`fs`, `log`, `mem`, and enough of `heap` for the examples).
- **Toolchain:** `aye spec`, the language reference in one file of about 20k
  tokens.
- **Acceptance:**
  - Example 1 in `design.md` (`readConfig`) compiles and runs.
  - **The gate test:** a fresh AI session gets only the `aye spec` output and
    then writes a small CLI in Aye, such as `wc` or a JSON tokenizer, using
    `aye check` feedback alone. Record how many check rounds it took and which
    errors were hardest to fix. Feed the findings back into `design.md`.
- **Gate rerun, 2026-10-08 (P2.5c):** a fresh agent with only `aye spec`
  (~24.4k tokens cl100k, budget ~30k) wrote `wc` and a JSON tokenizer.
  Each passed `aye check` in the first round; `wc` matched `/usr/bin/wc`
  on 4 inputs, and the tokenizer rejected 11 invalid inputs with
  positioned errors. It found four places where the spec was silent or
  loose (the compiler was right in each). The spec text now covers them:
  - §8: `return g()` cannot pass an `E!void` call up from an `E!bool`
    function (E0023); write `try g()` and then `return true`.
  - §8: `catch { ... }` without `|e|`; §6: `for _ in 0..3` discards the
    loop variable.
  - §7: a statement `match` arm is a block or an expression statement (a
    call, `try`, `catch`, `match`, `unreachable`); `=> return 1` needs a
    block.
  - §8: an error passed on into a larger set keeps its name, so
    `catch |e|` can compare it: `e == LexError.Invalid`.

# Stage0 milestones, part 2: enough language for the browser

M1 to M5 made Aye usable for small CLIs. M6 to M10 add what the anyknown
browser (`docs/browser.md`) needs: bit operations for hashing and decoding,
constants, integer `match` for tokenizers, real C interop for SDL2,
FreeType and libcurl, and a std with lists, maps and text. The same rules
apply: acceptance tests first, a milestone is done when they pass, and
`docs/spec.md` documents every user-visible addition (the spec budget is
raised to about 30k tokens for this).

## M6: Operators, constants and control flow

- **Language:**
  - Bitwise `& | ^` (binary), `~` (prefix), shifts `<< >>`, and the
    compound forms `&= |= ^= <<= >>=`. Integers only, both operands the
    same type. They bind tighter than comparisons and looser than `+ -`
    (shifts tighter than `& | ^`); mixing `&`, `|` and `^` without
    parentheses is a compile error. A shift by the bit width or more
    panics in debug and safe mode. In an `if`/`while` header and after
    `catch`, a `|` at the top level starts a capture, so a bitwise or
    there needs parentheses.
  - Top-level `let NAME = expr` constants (literals, arithmetic on
    constants, arrays and struct literals of constants) and top-level
    `var` globals with constant initializers. `let Name = TypeExpr` is a
    type alias (`let NodePool = Pool(Node)`).
  - `match` on integers and chars: literal patterns, several patterns per
    arm (`'a', 'b' =>`), inclusive ranges (`'a'..='z' =>`), and `_`, which
    is required unless the arms cover every value.
  - `while opt |v| { }`.
  - Methods on enums (`fn` in an enum body, with `Self`).
  - `==` and `!=` on pointers (identity), `[*]T`, and enums whose
    variants have no payload.
  - `for x in slice { }` and `for i, x in slice { }`.
- **Toolchain:** none beyond diagnostics with codes for every new error.
- **Acceptance:**
  - A golden program that computes FNV-1a and CRC-32 of a string and
    decodes UTF-8 to code points with `match` on byte ranges.
  - A golden program with a global constant table, a global counter, a
    type alias and an enum method.
  - Error tests: mixing `&` and `|` without parentheses; a non-exhaustive
    integer `match` without `_`.

## M7: C interop for real libraries

- **Language:**
  - `extern struct Name { ... }` with C layout; fields are C-compatible
    types (integers, floats, `bool`, `[*]T`, `?[*]T`, arrays, other extern
    structs, function pointers).
  - `extern fn` may take and return extern structs by value, `*T`,
    `*mut T` and `?*T` to them, and `...` (variadic) for functions like
    `snprintf`.
  - Function types `fn(A, B) R`: a top-level non-generic function is a
    value of that type, can be stored and called, and can be passed to C
    when all its parameter and return types are C-compatible
    (`pub extern fn` exports one under its own name).
  - `[*]T`: unchecked indexing `p[i]`, slicing `p[0..n]` to `[]mut T`,
    comparison with `null` when the type is `?[*]T`, and `@ptrCast(T, p)`
    between C pointer types. `s.ptr` on any slice.
  - `@sizeOf(T)` and `@zeroed(T)` (all-zero value of an extern struct or
    a numeric type).
  - `@link("name")` at the top level of a module: if `pkg-config` knows
    `name`, its cflags and libs are used, otherwise `-lname`. Also
    `@link("framework:Cocoa")` on macOS.
- **Toolchain:** `aye build` and `aye run` pass the collected link flags
  to clang; `aye build -o PATH`.
- **Acceptance:**
  - A golden program that calls `qsort` with an Aye comparator and
    `snprintf` with a float.
  - A program (not run in CI if SDL2 is missing) that opens an SDL2
    window, fills a pixel buffer from Aye and exits after one frame.
  - A golden program that fetches `file://` through libcurl into a buffer
    with an Aye write callback.

## M8: std for real programs

- **std:**
  - `std.list.List(T)`: `init(alloc)`, `deinit`, `append`, `pop`,
    `insert`, `orderedRemove`, `swapRemove`, `clear`, `items` (a
    `[]mut T`), `len`.
  - `std.map`: a hash map with `[]u8` keys and one with integer keys
    (`StrMap(V)`, `IntMap(K, V)`): `put`, `get`, `getPtr`, `remove`,
    `contains`, iteration.
  - `std.text`: `Builder` (append bytes, chars, ints, floats; `toOwned`),
    `eql`, `startsWith`, `endsWith`, `indexOf`, `split` iterator, `trim`,
    `toLower`, `parseInt`, `parseFloat`, `eqlIgnoreCase`, and UTF-8
    `decode`/`encode`.
  - `std.math`: `pi`, `sqrt`, `floor`, `ceil`, `round`, `pow`, `min`,
    `max`, `abs`, `clamp`.
  - `std.fs`: `create`, write to files, `exists`, `readFile(alloc, path)`.
  - `std.pool`: `Pool(T)` and `Handle(T)` from design.md Example 3.
  - `std.time`: monotonic milliseconds and `sleep`.
  - `Allocator.resize`; `std.mem.copy`, `set`, `indexOf`.
  - `std.testing` with the debug allocator.
- **Acceptance:** golden programs for each module, and a word-frequency
  CLI (read a file, split, count in a `StrMap`, sort, print the top 10).

## M9: Formatter and docs

- **Toolchain:** `aye fmt [--check] PATH` (idempotent, keeps comments) and
  `aye doc --signatures PATH` (every `pub` declaration with its doc
  comment, for AI context).
- **Acceptance:** `aye fmt` is idempotent on every `.aye` file in the repo
  and the golden tests still pass after formatting all of them.

## M9.5: Ergonomics from the browser

The browser written in Aye recorded what hurt (`browser/COMPILER_GAPS.md`
on its branch). Fix those in the compiler and std.

- **Language:** several variants per `match` arm (`.A, .B =>`); `[_]T{...}`
  (length from the elements); `==` between `?T` and `T` (and two `?T`), and
  on structs whose fields all compare; `orelse` (a default, a block, or
  `return`/`break`/`continue`), `.?` and `unreachable`; `return f()` of a
  `void` call in a `void` or `E!void` function; a method that takes
  `*Self` or `*mut Self` called on a temporary (`split(s, ",").next()`).
- **std:** `std.log.err`/`info` accept a string constant as the format.
- **Toolchain:** `aye test DIR` runs the tests of every module of the
  program, `--module PATH` those of one module.
- **Diagnostics:** the `as` hint only where `as` converts; a local named
  like an import says so (and its uses are not reported again).
- **Docs:** pointers into a growing `List` or `Pool` in the spec's
  pitfalls.
- **Acceptance:** golden and error tests for each item, `aye explain`
  texts for the new codes, a directory program whose sibling modules have
  tests, and `aye fmt` idempotent on the new syntax.

## M9.6: Ergonomics, round 2

The browser's second list (`browser/COMPILER_GAPS.md`, entries 10 to 15).

- **Language:** one implicit adjustment followed by wrapping into an
  optional (`[]mut T` to `?[]T`, `*mut S` to `?dyn I`), and dropping `mut`
  in an error union's payload (`E![]mut T` returned as `E![]T`);
  multi-line string literals (`\\` line strings); a constant as an array
  length (`[N]T`, `[N]T{...}`, `[mod.N]T`); `defer if`.
- **C interop:** `@linkOptional("name")` and `@hasLink("name")`: a program
  builds where an optional library is missing, and an `if @hasLink(...)`
  branch that cannot run is left out of the program.
- **Diagnostics:** "already declared at" names the file when the
  declaration is in another file of the module.
- **Docs:** why a struct's methods cannot be split across files.
- **Acceptance:** golden and error tests for each item, `aye explain`
  texts for the new codes, and `aye fmt` idempotent on the new syntax.

## M9.7: Ergonomics and runtime, round 3

The browser's third list (`browser/COMPILER_GAPS.md`: the JavaScript
engine, B6 and B7).

- **Runtime:** debug-mode `General` allocates and frees in constant time
  (a header per block, a list of live blocks, a 32 MiB quarantine of
  poisoned freed blocks), keeps its leak and double-free reports, and
  reports a write after free; a stack overflow panics (`panic: stack
  overflow`) in debug and safe mode.
- **Language:** `if f() |v| { } else |e| { }` on an error union (E0102);
  `return match` whose arms are error unions, errors or values; struct
  field defaults (`w: f64 = 100.0`, E0103); `orelse` chains through
  optionals; `==` on error values; `_ = x`; the repeat literal
  `[N]T{value, ..}`.
- **C interop:** `extern union` (E0104); `c_int`, `c_long`, `c_ulong` and
  the other C integer names; `@zeroed` as a constant (globals, defaults).
- **std:** `std.thread.spawnWithStack` (and an 8 MiB stack for `spawn`);
  `File.reader` and `Reader.readLine`, `File.read`; `std.math.inf`, `nan`,
  `isNan`, `isInf`; a struct with `alloc` and `free` methods is an
  `Allocator` (E0105; `free` gets the allocated size).
- **Modules:** importing the root module is E0083 with the structure to
  use instead (a module of its own for shared code).
- **Acceptance:** golden and error tests for each item, `aye explain`
  texts for the new codes, `aye fmt` on the new syntax, and a debug build
  that does a million allocation and free pairs plus an allocating
  `fib(25)` within 10 s (before: 963 s and 55 s; after: 0.07 s and
  0.02 s).

## M9.8: Round 4 and design leftovers

The browser's fourth list (B5: image decoding, layout) and promises of
`docs/design.md` that were not implemented yet.

- **Runtime:** runtime checks are emitted inline (`__builtin_*_overflow`
  into the destination, an `if` before the statement) instead of calls
  that clang does not inline at `-O0`; `@setSafety(false)` as the first
  statement of a block turns its checks off (E0108).
- **Language:** `x is .V` tests an enum's variant (E0106); `where T:
  Integer` gives `T` the integer operators and literals, each instance
  checked again with its type; the built-in `where T: Hash` and `@hash(x)`
  for every type `==` compares (E0107); `simd[N]T` vectors with lane-wise
  `+ - * /`, masks from comparisons, `v[i]`, `@splat`, `@reduceAdd`,
  `@reduceMin`, `@reduceMax`, `@all`, `@any`, `@select` (E0109); no limit
  on the number of instances of a generic (only on their nesting).
- **std:** `std.math` trigonometry, `exp`, logarithms, `hypot`, `fmod`,
  `trunc`, `sign`, and `minOf`/`maxOf`/`absOf`/`clampOf` for any integer
  type; `std.map.HashMap(K, V)` with struct keys; `std.diag.Diagnostics`
  for detailed errors (`diags: *mut Diagnostics`).
- **Tests:** `aye test` runs the ```` ```aye ```` blocks of `///` doc
  comments as tests, with errors and failures at their lines.
- **Acceptance:** golden and error tests for each item, `aye explain`
  texts for E0106 to E0109, `aye fmt` on the new syntax, and a decode-like
  loop (RGB to gray over 4 MiB, 8 rounds) in debug mode at 0.54 s instead
  of 1.52 s (0.55 s with no checks at all at `-O0`; safe mode 0.05 s).

## M9.9: Round 5

The browser's gap list (`browser/COMPILER_GAPS.md`) re-checked at the
M9.8 head: most open entries were fixed by M9.5 to M9.8, and these are
what remained.

- **Language:** a `catch` or `orelse` block may end with an expression,
  which is the block's value (`catch |e| { report(e); old }`), where it had
  to end with `return`, `break` or `continue`; `@downcast(*mut S, d)` turns
  a `dyn I` back into a pointer to its struct, `null` when `d` holds
  another type (E0110).
- **Toolchain:** `@embedFile("path")` is the bytes of a file as a `[]u8`
  constant, read when the program is compiled, with the path relative to
  the source file (E0111).
- **Docs:** decisions for what stays open: tuples, stackful coroutines, a
  check for pointers into a `List` that grew, uninitialized locals, and
  `@zeroed` of an Aye struct.
- **Acceptance:** golden and error tests for each item, a directory
  program that embeds files from two directories, `aye explain` texts for
  E0110 and E0111, and `aye fmt` idempotent on the new forms.

## M10: Threads

- **std:** `std.thread` (`spawn`, `join`), `std.sync` (`Mutex`,
  `Channel(T)`, `Atomic(T)`).
- **Acceptance:** a golden program that fans work out to four threads
  through a `Channel` and joins them; CI runs it under ThreadSanitizer.
- **Done as:** pthreads through runtime hooks in the C prelude, atomics as
  builtins (`@atomicLoad` ... `@atomicCas`, C11 `__atomic`), `@ptrCast`
  between function types, and `aye build --cflags` for
  `-fsanitize=thread` (the golden test skips the ThreadSanitizer run where
  clang's ThreadSanitizer does not work).

## M11: cimport

C headers read with libclang instead of declarations written by hand, as
`design.md` plans for the second stage of C interop.

- **Language:** `let c = @cimport("header.h", "-Idir", "-DNAME=1")` at the
  top level makes a module of the header's functions (`pub extern fn`,
  variadic ones too), structs and unions (`extern struct`/`extern union`
  with fields, or opaque for bit-fields, packed layout and undefined
  records), enum constants, typedefs (also those of other headers that
  the declarations use) and `#define`d numbers and strings (E0112 for
  the form). The include path has the `pkg-config --cflags` of the
  module's `@link` libraries, the macOS SDK and Homebrew's headers.
- **Toolchain:** libclang is loaded with `dlopen` when a header is first
  read, so the compiler neither links nor needs it otherwise; the
  generated declarations are cached in `~/.cache/aye/cimport`
  (`$AYE_CACHE_DIR`) until a file the header includes changes, and a
  cached header needs no libclang. `aye cimport HEADER` prints the
  generated source. E0113 for a header that is not found or has errors,
  E0114 when libclang is missing, each with an `aye explain` text.
- **Browser:** `browser/js/js.aye` declares its POSIX threads from
  `pthread.h` (`pthread_attr_t` and `pthread_t` were a 128-byte and a
  `u64` guess).
- **Acceptance:** a golden header with every mapped kind and the skipped
  ones (`tests/cimport/basics.h`, its generated source compared too);
  programs that call zlib and `math.h`, and start a thread through
  `pthread.h`; each run again without libclang from the cache; error
  tests for the form, a missing header, a header with errors and a
  missing libclang; `aye fmt` on `@cimport`; the browser builds and
  `cargo test --test browser` passes.
- **Done as:** generated Aye source parsed as an ordinary module, so the
  checker, `aye check --json` and the C generator are unchanged; see
  `docs/decisions.md`, "C headers (M11)", for the mapping, the cache and
  what happens without libclang.

## M12: Coroutines

The last open question of `design.md` (async or coroutines), decided with
the browser's experience: its generators and async functions ran on a
thread each, handing control back and forth through two channels.

- **std:** `std.coro`: `create(T, alloc, stack, f, arg)` makes a stackful
  coroutine that runs `f(arg)` on a stack of its own; `Coroutine.resume`,
  `std.coro.yield`, `isDone` and `deinit`. No language change:
  `async`/`await` stay out of the language.
- **Runtime:** the stack switch is assembly per target in the C prelude
  (aarch64 and x86_64, macOS and Linux; the callee-saved registers,
  FP/SIMD ones too); stacks are mmap'd with a 1 MiB `PROT_NONE` guard
  region, and each thread keeps up to 16 freed ones for reuse. Running
  out of a coroutine's stack panics with `panic: stack overflow` (debug and
  safe mode) as a thread does; each coroutine has its own error return
  trace. Debug and safe mode panic on resuming a finished, running or freed
  coroutine, on resuming it on another thread than the one that made it,
  and on `yield` outside a coroutine.
- **Browser:** `js/coroutine.aye` runs generators and async functions on
  `std.coro` coroutines instead of threads, with the same interface to the
  interpreter.
- **Acceptance:** golden programs for a generator, ping-pong (integer and
  floating-point values live across switches), nested coroutines, the
  guard region (deep recursion in a coroutine panics), 10k coroutines alive
  at once and 10k made and freed, `General` allocations in coroutines,
  coroutines on threads, the error return trace, and each misuse panic;
  the spec's `std.coro` section; a JavaScript conformance script of many
  live and dropped generators, nested resumes, deep recursion and async
  chains, also under `ANYKNOWN_JS_GC=stress`. Runs as x86_64 under
  Rosetta; the generated C compiles for x86_64 and aarch64 Linux. No new
  compile-time checks, so no new error codes.
- **Done as:** see `docs/decisions.md`, "Coroutines (M12)". In the
  browser, `fast` mode: iterating a generator 1e6 times took 4.7 to 7.7 s
  and takes 0.31 s; 100k two-value generators 4.3 s, now 0.39 s; 100k
  async chains of three functions 13 to 15 s, now 2.7 s.
