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

## Baseline (TC0a, TC0b, 2026-11-01)

| What | aye | dweller |
|---|---|---|
| Merged PRs since 2026-10-02 | 22 | 234 |
| PR open to merge, median / p90 | 8.1 / 26.3 min | 70.0 / 4884 min |
| CI run, median / p90 | 6.3 / 8.3 min | 15.1 / 32.2 min |
| CI runs failed / cancelled | 21% / 26% | 17% / 5% |

Dweller local build (`browser/build.sh`, `--mode fast` = `-O2`, under
`capped.sh`, median of 3): front end (`aye emit-c`, 1,049,870 lines /
41 MB of C) 2 s, full build 8 s, rebuild after a one-comment edit 7 s
(no cache), `aye test browser --module js` (28 tests) 4 s.

TC0c, dweller PRs that have CI data (202 of 234; the API has no `ci`
runs before 2026-10-11): open to merge median 40.5 / p90 794 min; CI
running is 70% of a PR's time at the median, waiting after green is
0.9 min at the median but 118 min at p90; 1 `ci` run per PR at the
median, 2 at p90. A green `ci` run takes 32.7 min (median, last 30), and
nearly all of it is one step: `cargo test --release --test browser`,
15.8 min on macOS and 13.6 min on Ubuntu (median), against 0.2 min to
build the browser. 92 of 458 run attempts failed (20%): 12 flaky (the
same commit passed on a re-run), 80 real.

**What this changes.** Local compile is not the bottleneck: 7 s. The
browser test step is (about 95% of CI time), and its 20% failure rate
doubles a PR's CI at p90. Order: TC3 (test selection and result cache)
first, starting from the browser test step (TC3a inventory); TC5
(landing) for the waiting tail; the build cache (TC2) last. Reproduce:
`scratchpad/tc0a/run.sh` and `scratchpad/tc0c/run.sh` of session
f85d97e7 (gh only); TC0b commands are in the card.

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

#### TC0c: where a dweller PR's 70 minutes go
- lane: project
- goal: per merged dweller PR since 2026-10-02: open to first CI start, CI runs per PR and their total time, last green CI to merge; per CI job and step: median time; failed runs that passed on a re-run of the same commit (flaky) vs not
- serves: X7 main metric, order of TC3 / TC5
- files: none (report only)
- verify: numbers reproduced by one script with `gh` only
- depends on: TC0a (reuses its data)
- parallel with: all
- where: local (gh only, no build)
- model: sonnet
- time box: 45 min

#### TC3a: inventory of dweller's browser test step
- lane: project
- goal: list what `cargo test --release --test browser` in aye-lang/dweller runs: test groups, how many cases each, what source directories and data (goldens, site archives, test262, WPT) each reads, and whether the harness can already run a subset (filters, env vars)
- serves: TC3 design (can we select tests by changed Aye modules?)
- files: none (report only)
- verify: every claim names a file:line in dweller
- depends on: TC0c
- parallel with: all
- where: local (read only)
- model: sonnet
- time box: 45 min

## Backlog (cards written when their phase comes up)

- TC1 fact store core: content-addressed store, local and remote cache.
- TC2 `aye build` incremental: stable C partitions, object cache by partition hash; output bytes identical.
- TC3 `aye test` selection and result cache; mutation check catches every injected fault.
- TC4 IO as a parameter in std; pure tests cache forever.
- TC5 VCS: evidence on commits, disjoint changes land without re-run, per-function merge.
