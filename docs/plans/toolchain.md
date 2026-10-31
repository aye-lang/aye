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

TC3a result (2026-11-01): 146 tests (`tests/browser.rs` 138, `tests/browser/perf.rs` 8); every group but `aye_unit_tests` runs the one built `dweller` binary, so a path-based map from changed files to groups is not sound. The only subset tool is the libtest name filter; no per-test times in CI. Slow: `js_conformance` (288 cases, twice), `jsdom_*`, `window_*`.

Design decision: two tracks. Safe now: shard the step and record per-test times (TC3b). Bold, measured first: per-case dynamic dependencies at C-function level (Aye's C output is deterministic, so a changed function is a changed C function body), select only cases that executed a changed function; safe for deterministic tests (dynamic regression test selection).

#### TC3b: shard dweller's browser test step, record per-test times
- lane: project (change in aye-lang/dweller, card file `docs/plans/cards/TC3b.md` there)
- goal: Ubuntu 4 shards, macOS 2 shards, one final aggregate check; once-per-OS steps only on shard 0; per-test times in the log (`--report-time`)
- verify: PR CI green; browser step wall time per shard reported
- where: dweller PR, CI · model: code-writer (opus) · time box: 45 min

#### TC3c1: executed C functions per js_conformance case (spike)
- goal: per case, the set of C functions it executes (counted even when inlined), at dweller main; naming patterns of C functions (stable vs numbered)
- verify: summary numbers reproducible from the scratch script
- where: local (capped.sh) · model: opus · time box: 45 min

TC3c1 result (2026-11-01): clang profile counters (`--cflags -fprofile-instr-generate`, counted before inlining) + `xcrun llvm-profdata show`; all 288 cases, 0 mismatches, 19 s for the whole set. 7574 C functions; a case executes 940–1245 (median 972); union 1868; 926 run in every case (a commit touching one selects all). C names are stable (derived from Aye qualified names, no counters), but bodies carry `#line` and location constants, so diffs must mask them.

#### TC3c2: changed C functions per commit (spike, after TC3c1)
- goal: for the last 30 dweller commits touching `browser/`, the C functions whose body changed, and how often non-function C (types, globals) changed
- where: local (capped.sh) · model: sonnet · time box: 45 min

TC3c2 result (2026-11-01, last 30 first-parent commits touching `browser/`, coverage at main as the approximation): the normalizer passes its checks (a comment line → 0 changes; one flipped comparison → exactly that function). With "any changed type or global re-runs all", 27 of 30 commits select all 288 cases (types and `aye_g_*` data change in most commits). Functions only: 16 select 0, 13 select all 288, 1 selects 10. Cause: 926 of the ~970 functions a case runs are the shared JS-engine core. **Conclusion: per-case selection inside a group does not pay for Dweller; the unit that pays is the whole group** (half the commits would skip `js_conformance` entirely), and it needs item → function edges (types, globals) to be safe. Expected gain about 2x on the test step, not 10x. Next decision waits for per-test times from TC3b.

#### TC3c3: selection rate and safety check (after TC3c1, TC3c2) — on hold, see TC3c2 result
- goal: per commit, the share of js_conformance cases selected; for 3 commits every case whose result changed is in the selected set

TC3b result (2026-11-01, dweller PR #238, merged 6e93a60): `ci` wall time 16.6 min (was 32.7 median). Shards unbalanced: Ubuntu 2.4 / 3.4 / 16.5 / 2.6 min, macOS 14.1 / 8.3 min. Per-test times: `jsdom_gc_stress` alone 908 s Ubuntu, 674 s macOS; next `js_conformance` 113 s; all others ≤ 84 s.

#### TC3e: split `jsdom_gc_stress` into four tests (dweller PR)
#### TC3f: balance shards by measured time, greedy longest-first (dweller PR, parallel with TC3e)
Predicted after both: Ubuntu ~6 min, macOS ~11 min (macOS becomes the critical path).

Result (2026-11-01): #239 merged 4bbd47c, #240 merged 3749645. First main run with both (41232362999): `ci` wall **8.9 min** (was 32.7 median), Ubuntu shards 4.0 / 6.2 / 3.4 / 5.1 min, macOS 8.8 / 5.4 min. macOS 1/2 is the critical path.

#### TC3f2: regenerate `tests/browser-times.txt` from main run 41232362999 (dweller PR)
#### TC3g: experiment PR (not merged): harness builds the browser with `--mode safe`; CI times per shard and every test whose result changes
The harness builds with plain `aye build browser` (debug, `-O0`). Debug mode is also the leak check: the General allocator keeps block headers and writes leak reports to stderr at `deinit` (`docs/decisions.md`, General), and the tests require empty stderr. So `--mode safe` alone would drop leak and double-free detection; if TC3g shows a large gain, the next step is an Aye option for the debug allocator in `-O2` builds, then switch the harness.

## Backlog (cards written when their phase comes up)

- TC3g test build mode: the harness builds the browser in debug (`-O0`); measure the suite with `--mode safe` (`-O2` + checks) and list what debug adds that tests rely on (empty stderr also catches leak reports).

- TC3d flaky handling: 12 of 92 failed attempts were flaky; detect by same-commit re-run, quarantine with an owner card, never silently.

- TC1 fact store core: content-addressed store, local and remote cache.
- TC2 `aye build` incremental: stable C partitions, object cache by partition hash; output bytes identical.
- TC3 `aye test` selection and result cache; mutation check catches every injected fault.
- TC4 IO as a parameter in std; pure tests cache forever.
- TC5 VCS: evidence on commits, disjoint changes land without re-run, per-function merge.
