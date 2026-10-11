# Plan: the anyknown browser lane

The browser lane of [`docs/plan.md`](../plan.md): the work on `browser/`.
**Maintenance mode since 2026-10-08:** no new site fixes or features.
The lane keeps `cargo test --release` green (the browser tests are the
language's biggest integration test), fixes breakage caused by language
changes, and records language gaps in `browser/COMPILER_GAPS.md`. It
resumes when Aye criteria O6–O9 hold ([`aye.md`](aye.md)), in the same
step that moves `browser/` into its own repo (P8.2). Finished steps:
[`done.md`](done.md).

## Done criteria

**Freeze (now)**

- M1: `cargo test --release --test browser` exits 0 on `main` after
  every merge, whichever lane made it.
- M2: the wrap-ups are merged: `git merge-base --is-ancestor <branch>
  main` exits 0 for `worktree-agent-ae5e6f75817463272` (P3.48) and
  `worktree-agent-a0b64f7b014b9b6ea` (P3.58), and `done.md` lists both.
  (The user deletes branches and worktrees.)
- M3: every limitation of Aye or std the browser works around has an
  entry in `browser/COMPILER_GAPS.md` with **Hit by**.

**Browser done (after the resume)**

- W1: `cargo test --release` passes, `jsdom_gc_stress` included.
- W2: `python3 browser/tools/page_load.py check --agent` over the
  recorded archives exits 0: no crash or timeout, JS errors at or under
  the expected count, expected Markdown text, one agent task per site.
- W3: every listed site is ✅ in content (`check`) and visually (the
  `visual` field of `browser/tools/sites.json`, P1.14), except sites
  excluded as bot walls.
- W4: load-time targets met over `run --live --repeat 5` (Decisions).
- W5: the manual checklist (`docs/browser-manual-checklist.md`) is run
  and recorded in its results table.
- W6: every `docs/browser.md` "What remains" item is done or declared
  out of scope.

## Current state (2026-10-08)

- Sites: `page_load.py check` passes on the recorded archives; the
  screens review of 2026-10-08 found the visual defects now in the
  backlog (P3.50–P3.61).
- Archives live in each checkout's `target/sites/` (untracked), so steps
  that replay sites or take screenshots run locally.
- P3.48 is not merged: the merge 34f6f93 passed `cargo test` but failed
  the site check (mdn FAIL, mistral 4 s → 40 s wall), so it was reset
  away. P3.48b bisects it. Its recaptured Mistral archive is in
  `.claude/worktrees/agent-ae5e6f75817463272/target/sites/mistral`.
- P3.58 is half done: WIP 34c2234 on `worktree-agent-a0b64f7b014b9b6ea`,
  not built (native `TypedArray` in `browser/js/typed.aye`, test
  `browser/tests/js/256_typed_array_views.js`).
- Open gaps the browser met: #3 (P2.9) and #49 (O1.3a, std only).

## Phases

| Phase | Goal | Exit check | Status |
|---|---|---|---|
| P0 | One clean `main` | – | done |
| P1 | "Done" checked by a command | `page_load.py check` | done (P1.6, P1.14 in backlog) |
| PW | Wrap-up before the freeze | M2 | current |
| P7 | Maintenance: tests green while the language changes | M1, M3 after every language merge | next (ongoing until P8) |
| P8 | Split `browser/` into its own repo and resume | the browser repo builds against Aye with a path or URL dependency (O7) and W1 passes there | after O6–O9 |
| P3 | Sites correct (W2, W3) | `page_load.py check --agent` exits 0; `docs/browser-sites.md` rerun | paused (backlog) |
| P4 | Load times (W4) | `run --live --repeat 5` medians meet the targets | paused (backlog) |
| P5 | B8 "What remains" resolved (W6) | the list is empty or out of scope | paused |
| P6 | Release | manual checklist recorded; final site run; tag | paused |

## P8 split plan

P8.1 was a dry run (2026-10-11, no repo created). `browser/` was copied
to a scratch directory with nothing else of this repo around it.
`aye check`, `aye build` and `aye test` on the copy exit 0 (131 tests, 15
modules), and the `tests/browser.rs` harness, run from a scratch root
against the copy, passed 62 of 62 with one changed line.

**What broke out of tree.** Almost nothing:

- `browser/` has no symlinks, no import of `std/` or `framework/` files
  (std is built into the compiler), and no source-level path into the
  repo. The only `@cimport`s are system headers (`pthread.h`,
  `dirent.h`, SDL2, FreeType, libcurl, HarfBuzz, brotli), found through
  `pkg-config` and `~/.cache/aye`.
- `aye build` named the binary after the directory (`browser`), not
  `anyknown`. Fixed: `browser/aye.toml` has `name = "anyknown"` and no
  `[deps]` (the browser imports no library besides std).
- `browser/build.sh` ran `cargo build` and `target/release/aye` from the
  parent directory, so it failed anywhere but the Aye repo. Fixed: it
  uses `$AYE`, else `aye` on the PATH, and runs cargo only when the
  parent directory is the Aye repo (`Cargo.toml` with `name = "aye"`).
- `browser/README.md` showed only the Aye repo build. Fixed: it shows
  the `aye build browser` and `AYE=... browser/build.sh` forms.
- `tests/browser.rs` is the one piece that still needs a change at split
  time (below): `aye()` uses `env!("CARGO_BIN_EXE_aye")`, which only
  exists in the Aye crate.

**Layout decision: keep the paths.** The new repo keeps the same
paths (`browser/...`, `tests/browser.rs`, `docs/browser.md`), so no path
in source comments, docs, `page_load.py`, `jsbench/run.sh` (they
`cd` three levels up and call `browser/build.sh`), `sites.json` or the
tests changes. `git subtree split` takes one prefix, so P8.2 uses `git
filter-repo --path` for several (as the backlog already said). Flattening
`browser/` to the repo root is a later, separate change.

**Files that move** (history kept):

- `browser/` whole: sources, `tests/` fixtures and goldens (including
  `tests/screenshot`, `tests/paint` PNGs and `tests/server.py`),
  `tools/page_load.py`, `tools/sites.json`, `tools/agent_tasks/`,
  `COMPILER_GAPS.md`, `README.md`, `aye.toml`, `build.sh`. There is no
  `scripts/page_load.py` and no tracked `sites/` directory: the site
  archives live in untracked `target/sites/`, so they do not move (copy
  them by hand when the lane resumes).
- `tests/browser.rs` and `tests/browser/perf.rs`.
- Docs: `docs/browser.md`, `docs/agent-protocol.md`,
  `docs/browser-sites.md`, `docs/browser-manual-checklist.md`,
  `docs/practices/browser.md`, `docs/plans/browser.md` (this file).

**What stays in the Aye repo** (changed in P8.3, after the new repo is
proven): `tests/common/mod.rs` (the new repo gets a copy; 79 lines, no
Aye internals), `docs/practices/workflow.md` and `aye.md`,
`scripts/setup-ubuntu.sh` (its comment names `browser/build.sh`; the
new repo gets its own copy of the apt line), `scripts/rename-aye.py`
(one-off, browser paths in its table), `README.md` (the Octane and
jQuery notices name `browser/tests/jsbench` and `jsdom/scripts`; the
notices move with the files), `CONTRIBUTING.md` (`browser/build.sh`),
`site/public/index.html` (a link to the new repo), comments in
`std/net/net.aye` and `std/http/http.aye` naming `browser/` files, and
`docs/decisions.md`, `docs/plan.md`, `docs/plans/aye.md` (mentions of
the browser as the largest program).

**How the new repo finds the compiler.** `aye` is a tool, not a library:
std is built in, so the browser has no `[deps]` entry. The repo pins the
compiler in a file `AYE_REV` (a commit of `aye-lang/aye`; the Aye repo
has no tags or releases yet) and installs it with `cargo install --git
https://github.com/aye-lang/aye --rev $(cat AYE_REV) --locked aye`.
Developers with a checkout of Aye set `AYE=/path/to/aye/target/release/aye`
(read by `browser/build.sh` and by the test harness). When the framework
(F4, P3.60) is split out, it comes in as `[deps] framework = { url =
..., }` in `browser/aye.toml` (O4.3); until then P8's exit check "builds
against Aye with a path or URL dependency" is met by the pinned
compiler, since a library dependency does not exist.

**How `tests/browser.rs` becomes the new repo's test.** It is a plain
Rust integration test: it runs `aye build browser -o <tmp>/anyknown`
once, then spawns that binary on fixtures under `browser/tests` and a
Python `server.py`; it also runs `aye test browser`. The new repo gets a
`Cargo.toml` with no dependencies (package `anyknown-tests`,
`publish = false`) and the three files above. One line changes: `aye()`
starts `std::env::var_os("AYE")` or `aye` instead of
`env!("CARGO_BIN_EXE_aye")`. `current_dir(root())` and every
`browser/...` path stay valid because the paths are kept. Run in a
scratch root with `AYE` set, this passed (62 tests, 191 s).

**P8.2 and later.** Run P8.2 when O6–O9 hold. It leaves the Aye repo
untouched; P8.3 removes the browser from it after P8.2 is verified.

#### P8.2: create `aye-lang/anyknown` with history
- lane: browser
- goal: a new repo `aye-lang/anyknown` holds `browser/`, its tests and docs with their git history, builds against a pinned Aye, and passes the browser tests in CI on macOS and Ubuntu.
- serves: W1 (and P8's exit check)
- files: in the new repo only: the moved files above, plus `Cargo.toml` (tests only), `AYE_REV`, `.github/workflows/ci.yml`, `scripts/setup-ubuntu.sh` (apt line copied), `README.md` at the root (one paragraph and the build commands), the one-line change in `tests/browser.rs`, and links fixed in `browser/README.md` (`../docs/practices/README.md` becomes `docs/practices/browser.md`)
- steps: (1) `git clone --no-local` the Aye repo to a scratch directory, then `git filter-repo --path browser --path tests/browser.rs --path tests/browser --path docs/browser.md --path docs/agent-protocol.md --path docs/browser-sites.md --path docs/browser-manual-checklist.md --path docs/practices/browser.md --path docs/plans/browser.md`; (2) copy `tests/common/mod.rs` and the apt line; (3) add `Cargo.toml`, `AYE_REV` (the current `main` of Aye), the `AYE` change and the CI file; (4) the user creates the repo and pushes (do not push or create it from an agent); (5) CI: matrix macOS and Ubuntu 24.04, install the SDL2, FreeType, libcurl, HarfBuzz, brotli packages and python3, `cargo install --git ... --rev $(cat AYE_REV) --locked aye` (cached), then `aye check browser`, `aye test browser` and `cargo test --release --test browser`.
- verify: in a fresh clone of the new repo with only the pinned `aye` installed, run one at a time: `aye check browser; echo $?` → 0; `aye test browser` → exit 0 and `131 passed; 0 failed; 15 modules` (or the then-current counts); `AYE=$(command -v aye) cargo test --release --test browser; echo $?` → 0 (62 tests); `aye build browser` writes `./anyknown`; `git log --oneline --follow browser/engine/sheets.aye | wc -l` is the same as in the Aye repo; the CI run is green on both systems.
- depends on: O6–O9 hold; no `browser/` step in flight (merge them first; a branch cut before the split is rebased by `git filter-repo`'s commit map or redone by hand)
- parallel with: nothing in `browser/`; Aye-lane steps may continue
- where: local (the filter-repo run and the pinned install); the CI run is on GitHub
- model: sonnet for the files; opus for the filter-repo path list check
- time box: 90 min
- notes: P8.3 (separate card, after this one): in the Aye repo delete the moved files, fix the stay-side mentions listed above, and add a downstream check (`scripts/downstream.sh`: clone `aye-lang/anyknown` at a rev recorded in `tests/downstream/ANYKNOWN_REV`, run `aye test browser` and the browser test with `AYE=target/release/aye`) that the merge agent runs after codegen, sema or std merges. That replaces M1, which `cargo test --release` no longer covers.

## Cards: PW wrap-up

All wrap-up cards are done (`done.md`); the lane is frozen.

## Cards: P7 maintenance (next, recurring)

Main writes a P7.n card from this form when a merge breaks the browser
tests or a language change removes the need for a workaround. No P7
card adds a site fix or a feature.

#### P7.n: browser breakage from <language step id>
- lane: browser
- goal: `cargo test --release --test browser` passes again after <step>, by changing `browser/` to the new language or std behavior (never by reverting the language change).
- serves: M1 (and M3 when a workaround goes or a gap is found)
- files: the failing area of `browser/`, `browser/COMPILER_GAPS.md`
- verify: `cargo test --release --test browser > target/browser.log 2>&1; echo $?` → 0
- depends on: <step> merged
- parallel with: all except other `browser/` steps in the same area
- where: cloud when no site replay is needed; local when the failure is in `page_load.py` or screenshots
- model: sonnet; opus when the cause is unknown (diagnose first, rule 7)
- time box: 45 min
- notes: "Fix the branch side when main removed an API" (`docs/practices/workflow.md`).

## Backlog

Paused until P8. Before any site item becomes a card, its symptom is
confirmed on current `main` (`docs/plan.md` rule 8). Source in brackets.

- P7 flaky `http_failed_image_fetched_once` under load: the failed image was requested twice (`GET /fail/once.png` ×2) in the full suite on main 0e61757 with three agents building; 3/3 alone pass. P3.23 removed a sleep; a second fetch still races. Same class on Linux arm64: `window_relayout_after_image_load` failed once in the full suite (main 71fd28a, an amd64 Rosetta run beside it), 3/3 alone pass. Diagnose step; verify 30× under load (merge-agent check) (main, 2026-10-08).
- P3.63 Mistral costs ~40 s CPU (was 4 s) since P3.48: its real animations run, and each forced layout re-runs layout over the whole document; the inline-style subtree restyle (0d7dddf) helps little because the main page notes other changes between writes, and one iframe uses container queries (`g.styles.queried`). Needs incremental layout; first log what the main page notes between layouts (P3.48b/d, 2026-10-08).
- P8.2, P8.3 split `browser/` into its own repo with `git filter-repo`, move `tests/browser.rs` and the browser docs with it (decision 2026-10-08; P8.1 dry run done 2026-10-11; cards in "P8 split plan").
- P1.6 `page_load.py` writes `last.stdout`/`last.stderr` into the archive folder; move them under `target/page-load-out/`; set a floor for the 0.8 CPU/wall warning (P1.5).
- P1.14 `visual` field per site in `sites.json` (ok / defects / date), printed by `check`; W3 needs it (screens review 2026-10-08).
- P3.7 Mistral: short iframe widgets, `setTransform` errors; recheck after P3.48 (old P3 table).
- P3.9 dump virtual clock: an `onload` set inside `setTimeout(0)` after `src` misses the load event (async images follow-up).
- P3.10 scripts run while parsing see an 800px viewport instead of `--width` (async images follow-up).
- P3.16 a script changing an img in another same-origin frame may record the failed load in the wrong page's cache (P4.6 follow-up).
- P3.19 URL parser accepts `http://[`; WHATWG IPv6 host parsing (P3.15).
- P3.21 fetch guards: immutable Response headers, forbidden Request header names, abort listener removed on settle (P3.15).
- P3.25 send `Origin` on non-GET and CORS-mode fetches (P3.17).
- P3.31 XHR `loadstart`/`progress` events and `xhr.upload` (P3.24).
- P3.32 a frame's `history.length` counts only its own entries; 1 during initial load (P3.29).
- P3.35 YouTube search agent task: expect results URL plus a title; recapture with the `youtubei/v1/search` POST (P3.30).
- P3.41 BBC ☰ menu button missing in replay; confirm, then diagnose (P1.4 screens).
- P3.42 Proxy invariant checks against the target (P3.39).
- P3.50 DuckDuckGo prints raw dates (`Date.parse` of 7-digit fractions, `Intl.DateTimeFormat`, `toLocaleDateString`) (screens review).
- P3.51 form controls: default-beveled buttons on Google, misaligned GitHub commit rows; confirm, then diagnose (screens review).
- P3.53 ellipsis and clamp follow-ups: right-to-left, `-webkit-line-clamp` into child blocks, unprefixed `line-clamp` (P3.52).
- P3.54 `grid-template` with area strings and the `grid` shorthand (P3.46).
- P3.56 `<video poster>` loaded and painted like an image; recapture wikipedia-en (P3.45).
- P3.59 intrinsic sizing passes no container height (`browser/layout/intrinsic.aye`, `cb_h = -1`); BBC NEWS `<h1>` 300px instead of 100px (P3.47).
- P3.60 move the JS lexer, parser and AST out of the `js` module for a bundler; pure move (framework F4, `docs/framework.md`).
- P3.61 `screenshot` runs the page as long as `dump` (Mistral's title animation caught half-way) (P3.48).
- P4.1 react.dev: diagnose live vs replay time (DNS/TLS, discovery, module waits, HTTP/2) (P1.4).
- P4.2 react.dev: fix the cause found by P4.1.
- P4.3 ZOL live: separate discovery waits from transfer waits (`browser/engine/PERF.md`).
- P4.4 YouTube startup scripts, 0.84 s of interpreter time (P3.2).
- P4.5 decode offscreen images at paint time (`PERF.md`).
- P4.8 ZOL and Mistral replays spend most time in timers; re-measure with CPU time, then diagnose (P1.1).
- P4.10 container-query re-pass restyles only container subtrees (P4.9).
- P4.11 partial restyle for DOM mutations incl. shadow roots and `:has()` (P4.10).
- P4.14 event dispatch and MutationObserver from prelude JS into native Aye (~15% of JS samples) (P1.11).
- P4.18b Map/Set iterators are snapshots; make them live (P4.18).
- P4.21 GC threshold factor (GC 14.5% of JS samples) (P4.16).
- P4.24 smaller JS objects, property storage that shrinks, GC threshold relative to live size (P4.23).
- P4.28 slab for unshared computed-style arrays (P4.27).
- P4.29 `Prop` 48 → 32 B, accessors stored separately (P4.26).
- P4.30 decision: next large JS speedup is a bytecode compiler; design doc first, its own phase after P3 (P4.16, P4.17, P4.20).
- P5 one step per `docs/browser.md` "What remains" item, each tied to a listed site or declared out of scope.
- P6.1 manual checklist on macOS, recorded; P6.2 final `docs/browser-sites.md` run; P6.3 docs pass and a tag.

## Decisions

- **Maintenance mode (2026-10-08):** see the top of this file and
  `docs/plan.md`, "Standing decisions".
- **Load-time target (W4):** per site, live median minus replay median
  ≤ 0.5 s over `run --live --repeat 5`; absolute targets in `sites.json`
  after P1.4 measured them.
- **Bot walls:** Stack Overflow and Reddit are excluded from W3 while
  they serve a bot wall; `sites.json` marks them `excluded`.
- **B8 remains:** an item no listed site needs is out of scope (P5).
