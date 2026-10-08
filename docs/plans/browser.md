# Plan: the anyknown browser lane

The browser lane of [`docs/plan.md`](../plan.md): the work on `browser/`.
**Maintenance mode since 2026-10-08:** no new site fixes or features.
The lane keeps `cargo test --release` green (the browser tests are the
language's biggest integration test), fixes breakage caused by language
changes, and records language gaps in `browser/COMPILER_GAPS.md`. It
resumes when Aye criteria O6–O9 hold ([`aye.md`](aye.md)), in the same
step that moves `browser/` into its own repo (P8.1). Finished steps:
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
- P8.1 split `browser/` into its own repo with `git filter-repo`, depend on Aye through O4.2/O4.3, move `tests/browser.rs` and the browser docs with it (decision 2026-10-08).
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
