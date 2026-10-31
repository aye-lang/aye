# Toolchain steps

Cards for project step X7 in [`docs/plan.md`](../plan.md). Ids use the
prefix `TC`. Rules and the card template are in `docs/plan.md`. Written
2026-11-01 after the user said: AI will write code very fast, so we want
our own alternative to git and large time cuts in compiling and testing,
while staying stable; we own a language, so be bolder.

## Goal

The time from an edit (by an agent) to "verified and on main" drops from
minutes to seconds, and main stays green.

Main metric: median edit-to-main time, from git and CI history (TC0a).

## Design: one content-addressed fact store

VCS, build and test are queries on one store whose keys are content
hashes.

- **Function-level hashes.** hash(f) = hash(normalized AST of f + hashes
  of the types and functions it uses). An edit to f invalidates only what
  reaches f. Sound for Aye: no macros, no reflection, the compiler sees
  the whole dependency graph.
- **IO as a parameter.** Like `Allocator`, file, network, clock and
  random access are passed in. A test that gets no IO is a pure function
  of its code hash: its result is cached forever and shared across
  machines. Tests that take IO are impure and always re-run. Breaking
  std change (TC4).
- **Commits carry evidence.** A commit records which function hashes
  compiled and which tests passed at which hashes. Two changes whose
  dependency cones do not intersect land without re-running: the merged
  evidence is the union. Merges are per function. Git object format stays
  underneath, GitHub stays the mirror (decision of 2026-10-17).
- **Early and shared.** A daemon builds and tests on save; CI produces
  cache entries, local work consumes them.

Stability: cache keys include compiler version, flags and target; main
periodically rebuilds and re-tests with an empty cache and compares
(a mismatch is a cache bug and becomes a card); builds are reproducible
(build twice, compare bytes); test selection is checked by mutation
(inject a fault, the selected tests must catch it); any doubt falls back
to a full run.

## Today (evidence, 2026-11-01)

- `aye build`: `compile()` in `src/driver.rs` regenerates the whole
  program's C on every build and compiles it in up to 8 parts at once,
  in temp files named by pid that are deleted after; nothing is kept.
- `aye test`: compiles all tests of the program into one executable on
  every run (`docs/decisions.md`, "`aye test`"); no result is reused.

## Lanes

- **A (this plan, first):** our own loop. TC0–TC5.
- **B (after TC2):** JS front end for business apps (React + TS):
  bundler and JS test selection and cache, module-level only (JS is
  dynamic). Baselines: Vite (with Vitest, `vitest --changed`) as the
  market default, Bun (`bun build`, `bun test`) as the speed bar. Same
  machine, same app, pinned versions, cold / warm / one-file edit, median
  of 5 runs, scripts published.

## Cards

#### TC0a: edit-to-main baseline from history
- lane: project
- goal: median and p90 of PR open-to-merge and of CI run time, last 30 days, aye-lang/aye and aye-lang/dweller
- serves: X7 main metric
- files: none (report only)
- verify: numbers reproduced by the `gh` commands in the report
- depends on: –
- parallel with: all
- where: local (gh only, no build)
- model: sonnet
- time box: 45 min

#### TC0b: build and test time baseline
- lane: project
- goal: Dweller full `--mode fast` build time split into front end (`aye emit-c`) and clang, the same after a one-function edit, and `aye test` time for `browser/js`
- serves: X7 main metric, TC2 design (is clang or the front end the cost?)
- files: none (report only)
- verify: 3 runs each under `scripts/capped.sh`, medians reported
- depends on: –
- parallel with: all but other local builds (capped.sh queue)
- where: local (macOS timing)
- model: sonnet
- time box: 45 min

## Backlog (cards written when their phase comes up)

- TC1 fact store core: content-addressed store, local and remote cache.
- TC2 `aye build` incremental: stable C partitions, object cache by partition hash; output bytes identical.
- TC3 `aye test` selection and result cache; mutation check catches every injected fault.
- TC4 IO as a parameter in std; pure tests cache forever.
- TC5 VCS: evidence on commits, disjoint changes land without re-run, per-function merge.
