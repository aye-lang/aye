# Stage0 milestones

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
- **Toolchain:** parser and sema error recovery, so that `ore check` reports
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
  - `ore explain <code>`.
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
- **Toolchain:** `ore test [--filter] [--json]`. A test that leaks memory fails.
- **Acceptance:**
  - A program that builds a linked list with `Arena` and frees it with a single
    `deinit`.
  - A deliberately leaking test is reported as failed and names the allocation
    site.
- **Out of scope:** `Pool` and `Handle` (these need generics).

## M4: comptime generics, Pool, and Handle

- **Language:** `comptime T: type` parameters, functions that return `type`,
  and `where` constraints, checked when the generic is defined.
- **Toolchain:** fix-it suggestions in `--json` output, and `ore fix` to apply
  them.
- **Acceptance:**
  - Example 3 in `design.md` (`Pool(T)` and `Handle(T)`) compiles.
  - Dereferencing a stale handle returns `null` instead of reading garbage.
- **Out of scope:** comptime I/O and general comptime execution beyond type
  functions.

## M5: Modules and first real tool (dogfooding gate)

- **Language:** directory modules, `import`, `pub` visibility, and a minimal
  `std` (`fs`, `log`, `mem`, and enough of `heap` for the examples).
- **Toolchain:** `ore spec`, the language reference in one file of about 20k
  tokens.
- **Acceptance:**
  - Example 1 in `design.md` (`readConfig`) compiles and runs.
  - **The gate test:** a fresh AI session gets only the `ore spec` output and
    then writes a small CLI in Ore, such as `wc` or a JSON tokenizer, using
    `ore check` feedback alone. Record how many check rounds it took and which
    errors were hardest to fix. Feed the findings back into `design.md`.
