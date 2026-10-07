# Plan: finishing Ore stage0 and the anyknown browser

Work is planned as phases of steps. A step is one goal with one verify
command, small enough for one agent in one focused session (one
reviewable commit series). An agent gets exactly one step; whatever else
it finds goes into its report as a new step.

## What "done" means

**Ore stage0**

- O1: `cargo test` passes on a clean checkout of `main`.
- O2: `ore fmt --check` passes over every `.ore` file in the repo (M9).
- O3: every open entry in `browser/COMPILER_GAPS.md` is fixed or
  declined in `docs/decisions.md`; no workaround remains for a fixed gap.
- O4: `ore spec` fits the ~30k-token budget (`milestones.md`, part 2),
  and the M5-style gate passes again: a fresh session writes a CLI using
  only `ore spec` and `ore check`.
- O5: `docs/milestones.md` has a status table.

**Browser**

- W1: `cargo test` passes, `jsdom_gc_stress` included.
- W2: a scripted check over recorded archives of every listed site
  passes: no crash or timeout, JS errors at or under the expected count,
  expected text in the Markdown, and one agent task (click, type or
  read) reaching the expected page or text.
- W3: every listed site is ✅, except sites excluded as bot walls.
- W4: load-time targets are met (see Decisions).
- W5: the manual checklist has been run and recorded.
- W6: every B8 "What remains" item is done or declared out of scope.

W2 and W4 cannot be checked by a command yet (`browser/tools/page_load.py`
covers 4 sites), so measurement (P1) comes before the site and speed
phases. The Ore close-out touches `src/`, `std/` and `tests/`, so it runs
alongside the browser phases.

## Phases

| Phase | Goal | Exit check |
|---|---|---|
| P0 | One clean `main` | `git branch` shows only `main`; one worktree; `cargo test` passes |
| P1 | "Done" is checked by a command | `page_load.py check --archive target/sites` prints a row per site, exits 0 only when all meet expectations |
| P2 | Ore stage0 closed (O1–O5) | `cargo test`, `ore fmt --check`, spec token count, gate record in `milestones.md` |
| P3 | Sites correct (W2, W3) | `page_load.py check` exits 0; `browser-sites.md` rerun |
| P4 | Load times (W4) | `page_load.py run --live --repeat 5` medians meet the targets |
| P5 | B8 remains resolved (W6) | "What remains" empty or out of scope only |
| P6 | Release | manual checklist recorded; final site run; `main` tagged |

P2 runs alongside P1, P3, P4. P3 needs P1. In P4 only P4.1 needs P1.

## Steps

∥ = can run at the same time. The browser files that conflict most are
`browser/engine/prelude.ore`, `tests/browser.rs` and the goldens; steps
that touch them merge one after the other.

### P0: Integration

Done 2026-10-07.

| id | Goal | Verify | Deps | ∥ |
|---|---|---|---|---|
| P0.1 | Close the 3 leftover branches (text-transform, baseURI/decode(), Web Animations) | patch-identical on stage0 (`git cherry`); deleted | – | all |
| P0.2 | Modules: commit the import() error and namespace edge tests (243, 245) | the tests' `.out` match | – | P0.4 |
| P0.3 | Modules: cycle (233, 244) and re-export (234, 242) tests pass; close what fails | `cargo test --test browser js_conformance jsdom_fixtures jsdom_gc_stress` | P0.2 | P0.4 |
| P0.4 | Async images: background fetch → decode task → `complete`/`naturalWidth`/`load`/`error` → re-layout; pending `decode()` promises settle | full `cargo test`; replay timings not worse | – | P0.2–3 |
| P0.5 | Integrator merges async images | `cargo test` | P0.4 | – |
| P0.6 | `pi-dynimport` merges stage0 again, GPT-branch review (`practices/workflow.md`), integrator merges | `cargo test`; `browser/build.sh` | P0.3, P0.5 | – |
| P0.7 | Integrator updates `docs/browser.md` once: status, "Done so far" (modules, async images, text-transform, baseURI, Web Animations, layout queries), "What remains" | read the diff | P0.6 | – |
| P0.8 | Fast-forward `main`; delete branches and worktrees; point `practices/workflow.md` and the pi-codex skill at `main` | `git branch`; `git worktree list`; `grep -rn stage0-milestones docs .claude` empty | P0.7 | – |

### P1: Measurement

| id | Goal | Verify | Deps | ∥ |
|---|---|---|---|---|
| P1.1 | `page_load.py --site all` covers every listed site; archives in `target/sites/` | `capture`, then `run --repeat 1` with 0 archive misses | P0 | P2 |
| P1.2 | `browser/tools/sites.json` expectations and a `check` subcommand | `check` reproduces the statuses in `browser-sites.md` | P1.1 | P2 |
| P1.3 | One agent task per site (JSON lines replayed through `anyknown agent`) | `check --agent` passes on HN, Wikipedia search, DuckDuckGo | P1.2 | P2 |
| P1.4 | `run --live` (medians, phase split) and `--screens` (contact sheet) | the live table prints | P1.1 | P1.2–3 |
| P1.5 | `page_load.py` records child CPU time (`getrusage` children delta) and the load average per run; warns when CPU/wall < 0.8. Speed targets compare CPU time, or wall time on a quiet machine | `run --site github` shows ~3.5 s CPU whatever the load | P1.2 | P2 |
| P1.6 | `page_load.py` writes `last.stdout`/`last.stderr` into the archive folder: move them under `target/page-load-out/`; check the 0.8 CPU/wall warning on a quiet machine (tiny pages may fall below it from startup cost) and set a floor | archives unchanged after `run`; no warning for `example` on a quiet machine | P1.5 | P2 |
| P1.7 | Replay is not deterministic: `youtube-search` requests different `i.ytimg.com` thumbnails per run under load (3 of 7 runs miss the archive). Find the wall-clock dependence (lazy loading, IntersectionObserver, dump scrolling, timer budget), make replay depend only on the virtual clock; then recapture and un-exclude `youtube-search` | 10 consecutive `check --site youtube-search` passes under load | P3.2 | P3 |

### P2: Ore close-out

| id | Goal | Verify | Deps | ∥ |
|---|---|---|---|---|
| P2.1 | Gap #48: `b as u8` and other integer casts, error test, spec entry, `ore explain` | `cargo test --test golden` | P0 | P1, P3 |
| P2.2 | Remove the #48 workaround in `browser/js/fast_expr.ore` | `cargo test --test browser js_conformance` | P2.1 | not with JS steps |
| P2.3 | Decline #3 and #21 in `decisions.md`; move them in `COMPILER_GAPS.md` | read the diff | – | all |
| P2.4 | `ore fmt --check` over the repo; fix the drift | `ore fmt --check` exits 0; `cargo test` | P0 | P3 after it merges |
| P2.5 | Spec token count; rerun the M5 gate; findings become gaps | record in `milestones.md` | P2.1 | all |
| P2.6 | Status table in `milestones.md` | read the diff | P2.3–5 | – |
| P2.7 | Diagnose a flaky build: once, a fresh build failed every `import()` with "import is not defined" and a rebuild of the same source fixed it (seen on `pi-dynimport`, 2026-10-07) | repeated clean builds + `browser/tests/js/23*` reproduce or rule it out; cause written down | P0 | all |
| P2.8 | Agents build into their own worktree: the pi-codex skill's task template and `pi_run.py` build with `browser/build.sh` (no shared output path) or name the output after the worktree (cause of P2.7) | `grep` the skill for shared output paths; one pi dry run | P2.7 | all |
| P2.9 | **Blocked (2026-10-07):** AddressSanitizer hangs at start-up on this machine (Apple clang 17, macOS 26.5, arm64; x86_64 under Rosetta dies with SIGILL), like ThreadSanitizer. Needs Homebrew LLVM or Linux. Keep the debug free queue (`src/cgen.rs:147-240`, catches double free and write-after-free) and poison its blocks only under ASan. Gap #3: under AddressSanitizer the runtime (`src/cgen.rs`) poisons the freed blocks it holds back in debug and `ore test`, so a read through a stale `List`/`Pool` pointer reports `heap-use-after-free`; golden test modelled on the ThreadSanitizer one, debug and safe mode | `cargo test --test golden` | P2.3 | P1, P3 |
| P2.10 | Stale comment at `browser/dom/dom.ore:192` says Ore has no `unreachable` (gap #32 is fixed); use it | `ore check browser`; `cargo test --test browser` | – | all |

### P3: Sites

Each step adds an offline fixture and a site expectation. A "diagnose"
step only finds and writes down the cause; the fix is the next step.

| id | Goal | Verify | ∥ |
|---|---|---|---|
| P3.1 | YouTube: diagnose why the feed does not boot. Done: it boots; YouTube serves a cookieless visitor an empty home feed (`feedNudgeRenderer`, no continuation), same with curl | – | – |
| P3.2 | YouTube: set the home page `ok`; add a `youtube-search` site (`results?search_query=rust+programming`, ~16 `videoRenderer` items live), capture it, and record what it shows; fixes for that page become new steps | `check --site youtube,youtube-search`; the search Markdown lists video titles | P3.1 | P3.4–8 |
| P3.3 | GitHub: diagnose, then fix, commit data that never loads | `check --site github` | P3.4–6 |
| P3.4 | Inline SVG sizing (BBC wordmarks at 290 px, Wikipedia ja pie chart) | `dump --layout` fixture; `check --site bbc,wikipedia-ja` | P3.1–3 |
| P3.5 | BBC menus drawn as bullet lists | style fixture; `--screens` | P3.6 |
| P3.6 | ZOL content runs past the right edge | layout fixture | P3.5 |
| P3.7 | Mistral: short iframe widgets, `setTransform` errors | `check --site mistral` | P3.4–6 |
| P3.8a | Drawing gaps 1: go.dev icon font, rustdoc logo and search bar | `--screens` | not with P3.4 |
| P3.8b | Drawing gaps 2: MDN header band, Wikipedia en figure, DuckDuckGo favicons, Google buttons | `--screens` | P3.8a |
| P3.9 | Dump virtual clock: an `onload` set inside `setTimeout(0)` after `src` misses the image's load event (a real browser catches it) | new dump fixture | P3.4–8 |
| P3.10 | Window: scripts run while parsing see an 800px viewport instead of `--width` | window test with `--width` | P3.4–8 |
| P3.11 | A golden run once hit its 10 s debug-build timeout under load; find the slow golden or raise its budget | repeated `cargo test --test golden` under load | all |
| P3.12 | `http_images_background` (GC stress) failed once at machine load ~45 and passed on rerun; find the timing assumption | repeated runs under load | all |
| P3.13 | GitHub: `BigInt is not defined` (P1.2 check) | `check --site github` errors 0; a JS test for the BigInt basics added | P3.3 |
| P3.14 | MDN: diagnose 16 lit "Hydration value mismatch" and 2 "not iterable" errors (P1.2 check) | cause written down | P3.4–8 |
| P3.15 | `Request` global missing (Reddit; general fetch API) | JS test; `check --site reddit` error gone | P3.4–8 |
| P3.16 | A script that changes an img in another same-origin frame may record the failed load in the wrong page's cache (P4.6 follow-up) | frame fixture | P4.6 |
| P3.17 | `fetch` sends only GET: support every method with `body` (string, URLSearchParams, FormData, Blob, ArrayBuffer), request headers and the `Content-Type` defaults; CORS-safe subset is enough | net fixture: POST/PUT/DELETE echo through the test server | P3.15 | P3.4–8 |
| P3.18 | Body mixin: `arrayBuffer()`, `blob()`, `formData()` on Request and Response; `body` as a minimal `ReadableStream` (`getReader().read()`) | jsdom test vs Node 22 | P3.17 | P3.4–8 |
| P3.19 | URL parser accepts `http://[` (invalid IPv6 host); follow WHATWG host parsing for IPv6 | URL test vs Node 22 | – | all |
| P3.20 | Recapture the Reddit and GitHub archives: after P3.15 (Request) Reddit fetches `svc/events/preload-head` and GitHub makes ~10 requests the old archives lack; reset their `sites.json` expectations | `check --site reddit,github` no archive misses | – | all |
| P3.22 | GitHub: React logs "Illegal invocation" after P3.13/P3.15 (a host method called with the wrong `this`); diagnose, then fix | `check --site github` errors 0 | P3.20 | P3.4–8 |
| P3.23 | `failed_image_once.html` relies on a fixed 0.3 s server delay (`/slow/wait.txt`), like P3.12 did; make it wait on the browser's state | the test under synthetic load, 30 runs | P3.12 | all |
| P3.24 | XMLHttpRequest sends only GET: `send(body)` and `setRequestHeader` on the native path P3.17 gave `fetch` | net fixture: XHR POST/PUT echo | P3.17 | P3.4–8 |
| P3.25 | Send `Origin` on non-GET fetches (and CORS-mode GETs) as the Fetch spec says | echo test checks `Origin` | P3.17 | P3.24 |
| P3.26 | GitHub: "Cannot retrieve latest commit" — the page requests `/latest-commit` and `/tree-commit-info` without the branch and gets 400; diagnose which value we compute wrong, then fix | `check --site github` Markdown has the latest-commit line | P3.22 | P3.4–8 |
| P3.27 | `ValidateAndApplyPropertyDescriptor` for non-configurable properties: a generic descriptor on a non-configurable accessor throws but should succeed when nothing changes; a non-configurable data property can still become an accessor or `configurable: true` but must throw | JS test vs Node 22 (both directions, strict and sloppy) | P3.22 | all |
| P3.28 | `ToPropertyDescriptor`: reject a non-callable `get`/`set` and a descriptor mixing `value`/`writable` with `get`/`set` (TypeError) | JS test vs Node 22 | P3.27 | all |
| P3.29 | History API: `history.pushState`/`replaceState` are no-ops (`browser/engine/prelude.ore:1416`). Implement the session history entries (URL, state, title), `location` updates, `popstate` on back/forward, `history.length`/`state`, and the agent's current URL following it | jsdom/window test; `check --agent` react task checks the URL again | P1.3 | P3.4–8 |
| P3.30 | After P3.29: diagnose why the GitHub (Issues), Mistral (Blog) and YouTube (search) agent tasks do not navigate (a script cancels the click/submit); fix or split into steps; remove their `task_fails` | `check --agent` passes on the three | P3.29 | P3.4–8 |
| P3.31 | XHR events and upload: fire `loadstart`/`progress` (and on `xhr.upload`), add the `upload` object | net fixture with event order vs the spec | P3.24 | P3.4–8 |
| P3.32 | History: a frame's `history.length` counts only its own entries (should be the tab's joint session history); `history.length` is 1 during the initial load | frame fixture | P3.29 | P3.4–8 |
| P3.33 | `Reflect.defineProperty` wraps the whole call in a catch-all (`browser/js/lib.ore`): a non-object target must throw TypeError and a throwing getter in the descriptor must propagate; only a rejected definition returns false | JS test vs Node 22 | P3.27 | all |
| P3.34 | `pushState(s, '', '')` keeps the current fragment; the spec resolves `''` against the base URL. Also `tests/golden.rs` `output_and_cpu_within`: a `wait4` error returns without killing the child | jsdom history test; golden helper review | P3.29, P3.11 | all |
| P3.35 | YouTube search agent task: the task should expect the results URL plus a result title (an in-page route change never reports "loaded"); recapture with the `youtubei/v1/search` POST. If its body varies per run (client nonce), make archive lookup for non-GET match on method+URL with a body-hash fallback, and say so in PERF.md | `check --agent --site youtube` passes 5× | P3.30, P1.7 | P3.4–8 |
| P3.36 | A node removed from a `DOMParser` document reports the main document as `ownerDocument`; it should keep the parsed one (node document is set by adoption, not by the current root) | jsdom test | P3.30 | all |
| P3.37 | CSS 2.1 §10.4 min/max table for replaced elements with a ratio: the two rows where both sizes violate max (or both min) and the height constraint is the tighter one break the ratio (P3.4 follow-up) | layout fixture for all table rows | P3.4 | not with other `layout/` steps |
| P3.21 | Fetch guards: Response headers read-only (`immutable` guard), Request headers drop forbidden names, a Request's abort listener on its source signal is removed when the request settles | jsdom test vs Node 22 | P3.15 | P3.17 after it merges |

### P4: Load time

| id | Goal | Verify | Deps | ∥ |
|---|---|---|---|---|
| P4.1 | react.dev: diagnose where live time exceeds replay (DNS/TLS waits, script discovery, per-level module waits, HTTP/2 multiplexing) | `run --live --site react` phase table | P1.4 | P3 |
| P4.2 | react.dev: fix the cause | live target met | P4.1 | P3, not with `net/` steps |
| P4.3 | ZOL live: separate discovery waits from transfer waits (`browser/engine/PERF.md`) | live table | P1.4 | P4.4 |
| P4.4 | YouTube startup scripts (0.84 s of interpreter time) | `run --site youtube` | P3.2 | P4.3 |
| P4.5 | Decode offscreen images at paint time (`PERF.md`) | PNG equality; replay medians | P0 | P4.3 |
| P4.6 | `loadFailed` forgets a failed image once no request holds it, so a later render fetches it again; keep failures per URL for the page | replay with a failing image: one fetch | P0 | P4.3 |
| P4.7 | GitHub: diagnose why replay takes 41 s against 16 s live (P1.1 run). Done: machine load (40–58 on 10 cores), not replay work; CPU 3.4–3.7 s replay vs 3.7–3.9 s live | – | P1.1 | – |
| P4.9 | Skip full restyles when the DOM version and viewport are unchanged since the last `computeStyles` (GitHub restyles ~8× per load; 92% of CPU in `css::computeStyles`): `engine/engine.ore`, `engine/containers.ore`, `agent/session.ore`, `engine/geometry.ore` | GitHub: ≤ 4 cascade spans, CPU < 2 s, Markdown hash unchanged | P1.5 | P3 (not with `css/` steps) |
| P4.10 | Container-query re-pass restyles only the container subtrees, not the document (after P4.9 GitHub still does 9 full cascades, 4.27 s CPU on the 16:31 archive) | GitHub cascade spans and CPU fall; Markdown unchanged | P4.9 | P3 (not with `css/` steps) |
| P4.11 | Partial restyle (only the changed subtree) for DOM mutations, including pages with shadow roots and `:has()` | GitHub < 2 s CPU on the current archive; `check --site all` byte-identical | P4.10 | P3 (not with `css/` steps) |
| P4.8 | ZOL (88 s) and Mistral (30 s) replays spend most of their time in timers; first re-measure with CPU time (P1.5) since the P1.1 run was on a loaded machine, then diagnose | `run --site zol,mistral` phase table with CPU | P1.5 | P4.9 |

### P5 and P6

- P5: one step per item in `docs/browser.md` "What remains". Before an
  item gets a step, it is tied to a listed site that needs it, or it is
  declared out of scope there.
- P6.1: run the manual checklist on macOS and record it. P6.2: final
  `browser-sites.md` run. P6.3: integrator's docs pass and a tag.

## How an agent runs a step

1. Input: step id, goal, verify, deps. Own worktree off `main`.
   GPT only through the pi-codex skill.
2. Read `docs/practices/README.md` and the practices file for the area.
3. Test or fixture first; small commits. Do not edit `docs/browser.md`,
   `docs/browser-sites.md` or this file; `docs/milestones.md` only in
   P2.6. New compiler gaps go in `browser/COMPILER_GAPS.md`.
4. Before finishing: merge the integration branch in (list each
   conflict in the merge message); `cargo build --release`,
   `target/release/ore check browser`, full `cargo test`,
   `browser/build.sh` with no output path (it writes `target/anyknown`
   in your worktree; never a shared path such as the scratchpad).
   `browser/tools/sites.json` expectations only tighten. If a site loses
   content it had, that is a regression to fix, never a reason to swap
   its expected strings (P3.20 did, for GitHub's "Unable to load page").
5. Report: commits, the verify output, the full test result, a "Done so
   far" paragraph, and follow-ups as new step proposals. A lesson goes
   into `docs/practices/` with its fix.
6. Integrator: one merge at a time, `cargo test` after each, shared docs
   once per batch, then deletes the branch and worktree.

## Decisions

- **Load-time target (W4):** for each site, live median minus replay
  median ≤ 0.5 s, over `run --live --repeat 5`. Absolute targets per
  site are set in `sites.json` after P1.4 measures them.
- **Bot walls:** Stack Overflow and Reddit are excluded from W3 while
  they serve a bot wall; `sites.json` marks them `excluded`.
- **B8 remains:** an item no listed site needs is out of scope (P5).
- **Integration branch:** `main` (since P0.8, 2026-10-07).
- **Platform:** macOS only. There is no remote and no CI. M10's
  ThreadSanitizer run is recorded as not possible on this machine.
