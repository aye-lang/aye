# Plan: the anyknown browser lane

The browser lane of [`docs/plan.md`](../plan.md): the work on `browser/`.
Phases P0, P1, P3, P4, P5 and P6 moved here from `docs/plan.md` on
2026-10-08 with their ids, so old reports still point at the right rows.
Steps run as `docs/plan.md`, "How an agent runs a step", says. Compiler
gaps found here go to `browser/COMPILER_GAPS.md` for the Ore lane
([`ore.md`](ore.md)).

## What "done" means

**Browser**

- W1: `cargo test` passes, `jsdom_gc_stress` included.
- W2: a scripted check over recorded archives of every listed site
  passes: no crash or timeout, JS errors at or under the expected count,
  expected text in the Markdown, and one agent task (click, type or
  read) reaching the expected page or text.
- W3: every listed site is ✅ in content (`check`) and visually (a screens review recorded in `sites.json`), except sites excluded as bot walls.
- W4: load-time targets are met (see Decisions).
- W5: the manual checklist has been run and recorded.
- W6: every B8 "What remains" item is done or declared out of scope.

W2 and W4 cannot be checked by a command yet (`browser/tools/page_load.py`
covers 4 sites), so measurement (P1) comes before the site and speed
phases.

## Phases

| Phase | Goal | Exit check |
|---|---|---|
| P0 | One clean `main` | `git branch` shows only `main`; one worktree; `cargo test` passes |
| P1 | "Done" is checked by a command | `page_load.py check --archive target/sites` prints a row per site, exits 0 only when all meet expectations |
| P3 | Sites correct (W2, W3) | `page_load.py check` exits 0; `browser-sites.md` rerun |
| P4 | Load times (W4) | `page_load.py run --live --repeat 5` medians meet the targets |
| P5 | B8 remains resolved (W6) | "What remains" empty or out of scope only |
| P6 | Release | manual checklist recorded; final site run; `main` tagged |

P3 needs P1. In P4 only P4.1 needs P1.

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
| P1.6 | `page_load.py` writes `last.stdout`/`last.stderr` into the archive folder: move them under `target/page-load-out/`; check the 0.8 CPU/wall warning on a quiet machine (tiny pages may fall below it from startup cost) and set a floor; live runs always fall below it (network wait is wall time without CPU), so mark live rows by another test | archives unchanged after `run`; no warning for `example` on a quiet machine | P1.5 | P2 |
| P1.7 | Replay is not deterministic: `youtube-search` requests different `i.ytimg.com` thumbnails per run under load (3 of 7 runs miss the archive). Find the wall-clock dependence (lazy loading, IntersectionObserver, dump scrolling, timer budget), make replay depend only on the virtual clock; then recapture and un-exclude `youtube-search` | 10 consecutive `check --site youtube-search` passes under load | P3.2 | P3 |
| P1.8 | Diagnose why a youtube-search dump costs 64–120 s CPU. Done: 42% YouTube's scheduler loops `while (Date.now() < deadline)` with a frozen benchmark `Date.now`; 37% YouTube's IntersectionObserver polyfill (installed because `IntersectionObserverEntry` is missing) | – | P1.7 | – |
| P1.9 | Global `IntersectionObserverEntry` (with `intersectionRatio`, `isIntersecting` on its prototype) in `browser/engine/prelude.ore`, so sites use our IntersectionObserver instead of a polyfill | youtube-search replay: no polyfill samples, CPU falls, Markdown unchanged; JS test | P1.7 | P1.10 |
| P1.10 | The virtual clock advances by a fixed amount per N steps inside a task (`tick()` in `browser/engine/script.ore`); benchmark `Date.now` = epoch + `ak.now()`, so time-based yields work and replay stays deterministic | scheduler tasks < 1M steps; 3 identical replays; CPU falls | P1.7 | P1.9 |
| P1.11 | Diagnose where youtube-search's remaining CPU goes (~80 s after P1.9; the scheduler now yields, but total work did not fall): JS-stack samples every 1M steps, as P1.8 did | ranked list of hot functions/bundles with shares; proposed fix steps | P1.7–P1.10 merged | P3 |
| P1.12 | Replay is deterministic but fragile: the virtual clock moves with the step count (P1.10), so any engine change shifts when time-batched beacons fire (P3.39 added one YouTube `log_event` POST by changing step counts, no behavior change). Archive lookup for a non-GET request falls back from method+URL+body to method+URL (the most recent recorded response), so extra or reordered beacons replay | youtube-search passes with P3.39 merged; a fixture with two POSTs to one URL, different bodies, one recorded | P3.40 | P3 |
| P1.13 | `page_load.py rebless --site X`: when dump behavior changes on purpose (P4.25 scroll budget), recompute an archive's reference Markdown/hash from a replay instead of recapturing (a live recapture of youtube-search on 2026-10-08 returned a page without results, 0.26 s CPU, likely a consent or bot page). `capture` refuses to save a page that fails the site's content expectations | `run --site youtube-search` exit 0 after rebless; a capture that misses the expected strings is rejected with a message | P4.25 | P3 |
| P1.14 | Visual status per site: `check` passes on Markdown while half the sites look broken (screens review 2026-10-08). Add a `visual` field to `browser/tools/sites.json` (ok / defects list / date reviewed), filled by a screens review; W3 counts a site ✅ only when both content and visual are ok | sites.json has `visual` for every site; `check` prints it | P1.4 | P3 |

### P3: Sites

Before a site step starts, its symptom is confirmed on current `main` (P1.4 `--screens`): P3.4 and P3.5 chased symptoms from an old manual run that no longer reproduced.

Each step adds an offline fixture and a site expectation. A "diagnose"
step only finds and writes down the cause; the fix is the next step.

| id | Goal | Verify | ∥ |
|---|---|---|---|
| P3.1 | YouTube: diagnose why the feed does not boot. Done: it boots; YouTube serves a cookieless visitor an empty home feed (`feedNudgeRenderer`, no continuation), same with curl | – | – |
| P3.2 | YouTube: set the home page `ok`; add a `youtube-search` site (`results?search_query=rust+programming`, ~16 `videoRenderer` items live), capture it, and record what it shows; fixes for that page become new steps | `check --site youtube,youtube-search`; the search Markdown lists video titles | P3.1 | P3.4–8 |
| P3.3 | GitHub: diagnose, then fix, commit data that never loads | `check --site github` | P3.4–6 |
| P3.4 | Inline SVG sizing. Done: the BBC 290 px wordmark and the Wikipedia ja chart did not reproduce on current main (stale symptoms); two replaced-element sizing rules fixed instead | – | – |
| P3.5 | BBC menus as bullet lists. Done: does not reproduce on current main (the row predates b69c8d4, CSSOM rules reach the cascade) | – | – |
| P3.6 | ZOL content runs past the right edge: confirmed 2026-10-08 (screens review); folded into P3.44 | – | – |
| P3.7 | Mistral: short iframe widgets, `setTransform` errors | `check --site mistral` | P3.4–6 |
| P3.8a | Drawing gaps 1. Screens review 2026-10-08: go.dev icons and the rustdoc logo render; the rustdoc search input is missing (→ P3.49) | – | – |
| P3.8b | Drawing gaps 2. Screens review 2026-10-08: MDN band (→ P3.47), Wikipedia en figure and DuckDuckGo favicons (→ P3.45) confirmed; Google buttons render default-beveled (→ P3.51) | – | – |
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
| P3.26 | GitHub latest-commit. Done: no engine bug; the repo-root route has no branch, so the URL is right; the 400 came from an archive captured before P3.17 sent the page's headers. Expectations tightened | – | – | – |
| P3.27 | `ValidateAndApplyPropertyDescriptor` for non-configurable properties: a generic descriptor on a non-configurable accessor throws but should succeed when nothing changes; a non-configurable data property can still become an accessor or `configurable: true` but must throw | JS test vs Node 22 (both directions, strict and sloppy) | P3.22 | all |
| P3.28 | Done in P3.33. `ToPropertyDescriptor`: reject a non-callable `get`/`set` and a descriptor mixing `value`/`writable` with `get`/`set` (TypeError) | JS test vs Node 22 | P3.27 | all |
| P3.29 | History API: `history.pushState`/`replaceState` are no-ops (`browser/engine/prelude.ore:1416`). Implement the session history entries (URL, state, title), `location` updates, `popstate` on back/forward, `history.length`/`state`, and the agent's current URL following it | jsdom/window test; `check --agent` react task checks the URL again | P1.3 | P3.4–8 |
| P3.30 | After P3.29: diagnose why the GitHub (Issues), Mistral (Blog) and YouTube (search) agent tasks do not navigate (a script cancels the click/submit); fix or split into steps; remove their `task_fails` | `check --agent` passes on the three | P3.29 | P3.4–8 |
| P3.31 | XHR events and upload: fire `loadstart`/`progress` (and on `xhr.upload`), add the `upload` object | net fixture with event order vs the spec | P3.24 | P3.4–8 |
| P3.32 | History: a frame's `history.length` counts only its own entries (should be the tab's joint session history); `history.length` is 1 during the initial load | frame fixture | P3.29 | P3.4–8 |
| P3.33 | `Reflect.defineProperty` wraps the whole call in a catch-all (`browser/js/lib.ore`): a non-object target must throw TypeError and a throwing getter in the descriptor must propagate; only a rejected definition returns false | JS test vs Node 22 | P3.27 | all |
| P3.34 | `pushState(s, '', '')` keeps the current fragment; the spec resolves `''` against the base URL. Also `tests/golden.rs` `output_and_cpu_within`: a `wait4` error returns without killing the child | jsdom history test; golden helper review | P3.29, P3.11 | all |
| P3.35 | YouTube search agent task: the task should expect the results URL plus a result title (an in-page route change never reports "loaded"); recapture with the `youtubei/v1/search` POST. If its body varies per run (client nonce), make archive lookup for non-GET match on method+URL with a body-hash fallback, and say so in PERF.md | `check --agent --site youtube` passes 5× | P3.30, P1.7 | P3.4–8 |
| P3.36 | A node removed from a `DOMParser` document reports the main document as `ownerDocument`; it should keep the parsed one (node document is set by adoption, not by the current root) | jsdom test | P3.30 | all |
| P3.37 | CSS 2.1 §10.4 min/max table for replaced elements with a ratio: the two rows where both sizes violate max (or both min) and the height constraint is the tighter one break the ratio (P3.4 follow-up) | layout fixture for all table rows | P3.4 | not with other `layout/` steps |
| P3.38 | `window_relayout_after_image_load` fails under machine load (an ordering race; passed 3 of 3 reruns). Make it wait on the browser's state like P3.12/P3.23 | the test 30× under synthetic load | – | all |
| P3.39 | `Object.setPrototypeOf` / `__proto__` accept a non-extensible object and prototype cycles; both must throw TypeError (`Reflect.setPrototypeOf` returns false) | JS test vs Node 22 | P3.33 | all |
| P3.40 | Archives lack web fonts and images: `capture` records what `dump` requests, but `screenshot` also needs fonts and images (BBC: 38 misses, serif headlines, empty image frames). Capture also runs the screenshot path (or records every subresource the page names) | `check --site all --screens` with no archive misses | P1.4 | P3 |
| P3.41 | BBC: the ☰ menu button seen live is missing in the replay; confirm on current main, then diagnose | screenshot | P1.4 | P3 |
| P3.42 | Proxy invariant checks are missing (ECMA-262 [[GetPrototypeOf]], [[SetPrototypeOf]], [[DefineOwnProperty]], [[Get]] etc. validate trap results against the target) | JS test vs Node 22 | P3.39 | all |
| P3.43 | URL attributes reflected to JS (`img.src`, `a.href`, `link.href`, `script.src`, `form.action`, …) resolve against the load-time `Page.base`, not the current document base: a `<base>` inserted or changed by JS is ignored. Resolve with `documentBase` (cached since P4.19) | jsdom test vs Chrome semantics | P4.19 | P3.4–8 |
| P3.44 | Right-edge overflow. Done 2026-10-08: GitHub (grid `1fr` column ignored `min-width:0; overflow:hidden` of its items) and YouTube search (`overflow-y:hidden` must compute `overflow-x` to `auto`, so the flex item has no automatic minimum) fixed; DuckDuckGo needs `text-overflow: ellipsis` (→ P3.52); ZOL is not a bug (its CSS sets `body{min-width:1400px}`, Chrome clips too) | – | – | – |
| P3.45 | Missing images. Done 2026-10-08: `screenshot` ran scripts in an 800×600 viewport and painted at 1280×2000, so script lazy loading never loaded lower images (YouTube, ZOL) — fixed in `browser/shot.ore`; DuckDuckGo favicons are ICO (→ P3.55); Wikipedia's "figure" is a `<video poster>` (→ P3.56) | – | – | – |
| P3.46 | Wikipedia squeezed text. Done 2026-10-08: the page grid uses the `grid-template` shorthand, which we dropped (TOC column 577 px instead of 196, text column 383 instead of 752); `grid-template: none | rows / columns` supported | – | – | – |
| P3.47 | Empty bands and duplicates: MDN grey band under the banner, a tiny empty search pill plus a second search box, "English (US)" twice, empty "Try it"; BBC ~280px band above NEWS — diagnose | cause per site | – | P3.44 |
| P3.48 | Mistral shows a dark hero with no headline or body text (animation start state opacity 0? content behind a hydration step?); "Back to Blog" overlapped by a "0%" widget — diagnose | cause | – | P3.44 |
| P3.49 | rustdoc: no search input (shows "Search / Settings / Help / Summary" as text) — diagnose (custom element `rustdoc-search`, form control rendering, script-inserted input) | cause | – | all |
| P3.50 | DuckDuckGo prints raw dates `2026-07-20T00:00:00.0000000`: a Date parse/format path (`Date.parse` of 7-digit fractional seconds, `Intl.DateTimeFormat`, `toLocaleDateString`) — test vs Node 22 | JS test; DuckDuckGo shows formatted dates | – | all |
| P3.51 | Form controls: buttons render default-beveled where sites style them (Google), GitHub commit-message rows misaligned — confirm against real renders, then diagnose | screenshot comparison | P3.44 | P3.45 |
| P3.52 | `text-overflow: ellipsis` for single lines in boxes that clip their overflow (`browser/css/css.ore` property, `browser/layout/inline.ore`) | fixture; DuckDuckGo titles and YouTube-search descriptions end in "…" on screens | P3.44 | P3.45 |
| P3.53 | Ellipsis and clamp follow-ups: right-to-left ellipsis, `-webkit-line-clamp` reaching into child blocks, unprefixed `line-clamp` | fixtures | P3.52 | all |
| P3.54 | `grid-template` with area strings and the `grid` shorthand (dropped today; no listed site needs them yet) | fixtures | P3.46 | all |
| P3.55 | ICO decoder (`browser/image/ico.ore`, sniffed in `image.ore`): PNG entries → `decodePng`; BMP entries 4/8/32 bpp with the AND mask; pick the entry closest to the needed size | fixture; DuckDuckGo favicons on screens | P3.45 | all |
| P3.56 | `<video poster>`: load it like an image (`isImageElement` in `browser/engine/images.ore`) and paint it in the video box; recapture wikipedia-en | fixture; Wikipedia en figure on screens | P3.45 | P3.55 |
| P3.57 | `window_agents` "connection refused" under load. Done 2026-10-08: `listenAt` (`browser/ui/socket.ore`) bound before listening, so the socket file appeared before it accepted; now binds a temporary path, listens, renames. 0/30 failures under load after the fix; 2/30 in an earlier run stay unexplained (output not kept) — if it recurs, keep the logs | – | – | – |
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
| P4.12 | youtube-search (P1.11: ~77 s CPU, ~53% in JS layout queries): 104 queries fall back to a full restyle because `restyle_all` is set. Find and fix what sets it (`noteLayoutChangeAnywhere` callers, `MAX_RESTYLE` overflow, `restyleRoots` returning false; `engine/geometry.ore`, `cssom.ore`, `dom_api.ore`, `script.ore`) | full restyles 104 → near 0; CPU falls; Markdown unchanged | P1.11 | P3 (not with `css/`) |
| P4.13 | Cheaper full restyle: `RuleIndex_candidates`, `pickPseudoRules`; or restyle only the queried element's ancestors and subtree for a geometry query | `sample` share and replay CPU; `check --site all` byte-identical | P4.12 | P3 (not with `css/`) |
| P4.14 | Move the prelude's event dispatch (`invoke`/`removeListener`) and MutationObserver (`onMutations`/`interested`) from JS into native Ore (~15% of JS samples on youtube-search) | JS steps and CPU fall; all event/MO tests pass | P1.11 | P4.12 |
| P4.15 | Don't recompute rule candidates for `::before`/`::after` per element (`pickPseudoRules`, `RuleIndex_candidates`; P4.13 profile) | replay CPU; `check --site all` byte-identical | P4.13 | P3 (not with `css/`) |
| P4.16 | Diagnose the JS interpreter on youtube-search (~48 s of ~50 s CPU is in timers running JS after P4.12/P4.13): which operations dominate `Interp_eval` (property lookup and inline caches, calls and frames, closures/environments, string building, array methods in the prelude, GC) | ranked list with shares; proposed steps ≤ 45 min each | P1.11 | all |
| P4.17 | Shape table runs out: `MAX_SHAPES` (16384, `browser/js/shapes.ore`) is exhausted within ~4M property reads on youtube-search, so 53% of cached-site reads hit shapeless objects and the per-site cache hits only 27%. Raise or remove the cap (and make shapeless fallback rare) | no shapeless fallbacks on youtube-search; cache hit rate and CPU | P4.16 | P4.18 |
| P4.18 | Map/Set `findKey` scans every entry with `sameValueZero` (`builtins_misc.ore`): add a hash index keeping insertion order | JS test vs Node 22 (NaN, ±0, objects, strings); CPU | P4.16 | P4.17 |
| P4.18b | Map/Set iterators are snapshots: for-of over a Map/Set that changes mid-loop differs from the spec (live iterators: deleted entries skipped, appended entries visited) | JS test vs Node 22 | P4.18 | all |
| P4.19 | `engine::documentBase` walks the whole document on every JS image load: cache it per DOM version (`engine/engine.ore`) | `<base>` changes still apply (test); CPU | P4.16 | P4.17 |
| P4.20 | Property cache on DOM wrappers and prototype chains. Done 2026-10-08, not merged: hits 54% → 59% but youtube-search CPU flat (12.21 → 12.39 s), like P4.17. Kept: the invalidation test and a practices entry | – | – | – |
| P4.21 | GC is 14.5% of JS samples (38 collections, up to 5M live objects; marking ~10%): tune the threshold factor (`gc.ore`) | GC share and CPU; GC stress tests pass | P4.17 | all |
| P4.22 | Done: the JS heap is ~1.7 GB of live objects (+45% slack in power-of-two property storage), growing to 3.4M objects (~110K YouTube component instances, 208K signals, 82K DOM wrappers for a page showing 43 videos); lead: the DOM mutation log keeps removed subtrees alive. Memory: a youtube-search replay peaks at ~4.8 GB RSS (up to 5M live JS objects). Diagnose what holds the memory (JS heap by object kind, strings, DOM, layout boxes, images, archive buffers, shapes) | ranked breakdown; proposed steps | P4.16 | all |
| P4.23 | Mutation-log lead. Done: wrong (the log is emptied after every DOM call). The memory follows the DOM size: the dump scrolls up to 20 steps, and YouTube's endless results load ~10 more pages (100K nodes, 4.69 GB; no scrolling 1.0 GB, 4 steps 2.1 GB) | – | – | – |
| P4.25 | Dump scroll budget for endless pages: stop once the page has grown by lazy loading in 2 scroll rounds (decision: an agent reads what a user sees near the top, not 10 pages of a feed). Pages that load content once still scroll as today | youtube-search CPU and peak RSS fall; every other site's Markdown byte-identical; scroll tests pass | P4.23 | P4.26 |
| P4.26 | Done: our structures cost ~8 KB per element — computed style ~5.7 KB (a full 2.66 KB array per element in 4 KB arena chunks), layout boxes ~1.4 KB, DOM ~0.67 KB, JS wrapper ~0.6 KB; most of YouTube's 47 KB/node is its own JS objects. Diagnose the per-DOM-node cost: ~47 KB and ~30 JS objects per node on youtube-search (Chrome: ~1–2 KB). Measure per-element memory by structure (computed style records, layout boxes, wrappers, attribute storage, text runs, shaping caches) | ranked per-node breakdown; proposed steps | P4.23 | P4.25 |
| P4.27 | Share identical computed-style arrays between elements (hash-consed, copy on write; `css/cascade.ore` `Styles`) | synthetic 40K-element page: style bytes per element ≤ 1 KB; style goldens and `check --site all` byte-identical; CPU not worse | P4.26 | not with `css/` |
| P4.28 | If P4.27 leaves unshared arrays: allocate them from a slab instead of one 4 KB chunk each | style slope ≤ 3 KB on unique-style pages | P4.27 | not with `css/` |
| P4.29 | `Prop` 48 → 32 B: data properties carry no setter field (accessors stored separately) | GC tests; RSS | P4.26 | P4.27 |
| P4.30 | Decision, not a step yet: property lookup is no longer the bottleneck (P4.17, P4.20 raised cache hits without moving CPU); the remaining JS cost is the tree-walking interpreter itself (~30% of samples in `Interp_eval`/`exec`/`invoke`). The next large speedup is a bytecode compiler for the JS engine — plan it as its own phase after P3, with a design doc first | design doc in docs/ | P4.16 | – |
| P4.24 | Smaller JS objects and property storage that shrinks (no permanent power-of-two slack); GC threshold relative to live size | peak RSS and CPU; GC stress tests | P4.23 | P4.21 |
| P4.8 | ZOL (88 s) and Mistral (30 s) replays spend most of their time in timers; first re-measure with CPU time (P1.5) since the P1.1 run was on a loaded machine, then diagnose | `run --site zol,mistral` phase table with CPU | P1.5 | P4.9 |

### P5 and P6

- P5: one step per item in `docs/browser.md` "What remains". Before an
  item gets a step, it is tied to a listed site that needs it, or it is
  declared out of scope there.
- P6.1: run the manual checklist on macOS and record it. P6.2: final
  `browser-sites.md` run. P6.3: integrator's docs pass and a tag.

## Decisions

- **Load-time target (W4):** for each site, live median minus replay
  median ≤ 0.5 s, over `run --live --repeat 5`. Absolute targets per
  site are set in `sites.json` after P1.4 measures them.
- **Bot walls:** Stack Overflow and Reddit are excluded from W3 while
  they serve a bot wall; `sites.json` marks them `excluded`.
- **B8 remains:** an item no listed site needs is out of scope (P5).
