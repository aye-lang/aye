# Page-load performance

Measured 2026-10-07 on an Apple M2 Pro (10 CPUs, 32 GB), macOS 26.5.
The workload is `anyknown dump --md --js URL`, fast mode, fixed-metric
layout as used by the Markdown CLI. No JS interpreter, script-budget or
existing golden changes are included.

## Timing and offline replay

`ANYKNOWN_TIMING=1` writes tab-separated records to **stderr**:

```
info: ANYKNOWN_TIMING<TAB>phase<TAB>nanoseconds<TAB>source
```

Stdout is unchanged. Disabled hooks allocate nothing and do not read the
clock. `browser/perf/perf.ore` is shared by engine, CSS, net, Markdown
and paint. The runner aggregates phases, lists the five slowest document
scripts, and saves individual runs, medians and output hashes as JSON.
Each run also records the browser's CPU time (user+sys) and the 1-minute
load average; the `run` and `check` tables show median `wall`, `cpu` and
`load` columns, and `--compare` reports the CPU improvement (wall in
parentheses). A site whose CPU/wall is below 0.8 gets a warning: the
machine was too busy for wall time to mean anything, so compare CPU.

```sh
cargo build --release
browser/build.sh
# Choose a fresh directory; capture refuses to overwrite a snapshot.
python3 browser/tools/page_load.py capture --archive target/page-load-new
python3 browser/tools/page_load.py run --archive target/page-load-new \
    --repeat 7 --json target/page-load-new/after.json
# Alternating AB/BA order; one excluded warmup per binary and site:
python3 browser/tools/page_load.py run --archive target/page-load-new \
    --compare target/anyknown-perf-before --repeat 7 \
    --json target/page-load-new/comparison.json
```

`capture` refuses a page that misses the site's `sites.json` `md`
strings (a consent or bot page): it prints the strings, exits non-zero
and saves nothing, so a good archive is never replaced. It also writes
`reference.md` (the Markdown whose hash `run` compares, in
`manifest.json` `capture.sha256`).

When a dump change is intended (P4.25: dumps stop after two growth
rounds), `run` fails with "replay Markdown differs from capture". Then:

```sh
python3 browser/tools/page_load.py rebless --site youtube-search --archive target/sites
```

`rebless` replays each site with the binary, refuses (exit 1, archive
untouched) when the replay misses its `sites.json` strings, prints the
lines added/removed against the old `reference.md` (archives from before
P1.13 have none: it prints the line and byte counts instead), and stores the new Markdown and hash.

The URLs are in `browser/tools/page_load.py`. Without `--site` it loads
these four pages; `--site zol` (or `wikipedia`, `github`, `youtube`)
selects a subset, and `--site all` loads every site of
`docs/browser-sites.md`. The default archive is `target/sites`, one
directory per site. Only Python's
standard library is needed. Each load has a 300-second timeout.

`run --live` loads each site from the network instead (no archive; the
last output goes to `target/live/<site>`), `--repeat` times after the
warmup, and prints the same table: median wall, CPU and phases. The
Markdown is not compared, since the network serves today's page. The
CPU/wall warning fires on most live rows, because waiting for the
network is wall time without CPU; read the `load` column instead.

`--screens` (with `check` or `run`, replay or live) also takes a
screenshot of each site, 1280 px wide and its first 2000 px, into
`target/screens/<site>.png`, and writes the contact sheet
`target/screens/index.html`: a thumbnail per site that links to its
image, labelled with its status in `sites.json`, its verdict, its
console errors and what went wrong. Use it to see a site's symptom on
current `main` before a site step starts:

```sh
python3 browser/tools/page_load.py check --site all --screens
open target/screens/index.html
```

The screenshot fetches what `dump --md` does not (web fonts, images).
`capture` (since P3.40) also takes the screenshot, replaying what the
dump recorded and recording only the rest, so a new capture replays the
screenshot without misses. Archives captured before P3.40 miss those
resources; the misses are listed under the thumbnail, and the page is
painted without them.

### Archive semantics

- `ANYKNOWN_RECORD=DIR` saves the first response for each URL/POST body:
  status, MIME, charset, redirect destination, raw decompressed body, or
  fetch failure. Original URLs, HTML, CSS and JS are **not rewritten**.
- `ANYKNOWN_REPLAY=DIR` serves these files and disables HTTP prefetch.
  A miss fails locally, is printed, and **never falls through to HTTP**.
  With `ANYKNOWN_RECORD` set as well (capture's screenshot), a miss is
  fetched and recorded instead. A non-GET request (a POST beacon) whose
  body has no record gets the latest-recorded non-GET response for its
  URL, whatever its body; this prints `archive fallback:` and `check`
  shows it on a `fallback:` line, not as a miss (P1.12). GET requests
  match exactly.
  The runner rejects misses, changed resource hashes, capture/replay
  Markdown differences, and before/after output differences.
- The manifest lists every requested resource, body size and SHA-256.
  Each `.response` has six newline-delimited metadata fields, then body
  bytes. Filenames are stable 64-bit FNV hashes of URL and optional POST
  content type/body; reading also verifies the original URL.
- The runner sets `ANYKNOWN_BENCHMARK=1`: a benchmark-only engine prelude
  freezes JS Date at 2026-10-07 00:00 UTC and seeds Math.random with 1.
  Otherwise timestamp/random cache-busters cannot replay. Normal browsing
  retains the real builtins; the existing virtual timer clock is unchanged.
- This is an immutable benchmark snapshot, **not a general HTTP cache**.
  Response headers, cookie-jar updates, expiry and authenticated sessions
  are not replayed. The runner clears `ANYKNOWN_COOKIE_FILE`; avoid
  personalized pages. Keep snapshots private (site directories are mode
  0700). If dynamic state still changes requests/output, the runner fails
  rather than silently substituting another resource (the non-GET
  fallback above is reported).

The measured snapshots remain locally in `target/page-load-fixed/`
(ignored by git), with manifests, bodies, traces, `final.json` and samples.
Third-party bodies are not committed. `perf-results.json` beside this file
preserves all seven measured wall/phase runs and their shared output hashes.

| Site | Recorded requests | Body bytes | Markdown bytes |
|---|---:|---:|---:|
| zol | 818 | 10996175 | 102606 |
| wikipedia | 29 | 857473 | 61283 |
| github | 24 | 2837200 | 8355 |
| youtube | 17 | 16673389 | 0 |

YouTube's preserved Markdown is empty. This measures the existing
browser's load performance, not successful rendering of its JS application.

### Rebuilding the baseline

The baseline is `ec6961c` (timing/archive machinery only), with later
timer/request/image timing hooks, **without** performance optimizations.
The compiler, C optimization mode and frozen responses are the same.

```sh
mkdir -p target/perf-before-src
git archive ec6961c | tar -x -C target/perf-before-src
git diff ec6961c 8f3ace3 -- browser/net/net.ore browser/net/multi.ore \
    browser/engine/images.ore browser/engine/script.ore | \
    git apply --directory target/perf-before-src
target/release/ore build target/perf-before-src/browser \
    -o target/anyknown-perf-before --mode fast
```

Use a fresh source directory. Either binary replays the saved snapshot
offline, including frames, CSS/imports, dynamic scripts and images.

### Phase boundaries and paint

Phases are wall time, **not additive**: requests/prefetch overlap
fetch/scripts/sheets/images; CSS parse overlaps sheets and style-reading
scripts; lazy cascade overlaps scripts/timers; image decode is in images.
Startup scripts include prelude setup, fetching and load events. Timer
callbacks/checkpoints are separate. Individual `script` spans include
parsing, execution, microtasks and document.write (nested scripts overlap).

Cascade includes candidate-index construction, matching, declaration
competition, inline styles, inheritance, computed values and pseudos.
Layout includes tree construction/text measurement; frames report their
own phases. Document decode is encoding-to-UTF-8. `sheets=0` may mean a
reader's parsed sheet was transferred to full styling, not skipped CSS.
Startup/teardown and some frame synchronization are only in total wall.

Markdown does not paint. To benchmark the separate screenshot CLI:

```sh
python3 browser/tools/page_load.py run --archive target/page-load-fixed \
    --site wikipedia --screenshot --repeat 3
```

This uses screenshot defaults and fixed fonts, not the final scripted
Markdown document. PNG hashes must agree across repeats (and binaries
with `--compare`). Engine hooks expose display-list/raster timings whenever
that path runs.

## Offline results

Seven alternating before/after loads after one warmup per binary/site.
Capture and both binaries produce identical Markdown bytes. The optimized
binary is `b97bd1a`; raw measured runs are in `perf-results.json`.

| Site | Before (s) | After (s) | Faster | Before range (s) | After range (s) |
|---|---:|---:|---:|---|---|
| zol | 2.644 | 1.260 | 52.3% | 2.444–4.822 | 1.113–2.136 |
| wikipedia | 1.236 | 0.183 | 85.2% | 0.390–2.962 | 0.126–0.401 |
| github | 3.444 | 0.514 | 85.1% | 3.309–8.362 | 0.504–0.774 |
| youtube | 1.201 | 0.935 | 22.1% | 1.156–1.578 | 0.882–1.263 |

Concurrent agents were building/testing. Ranges are retained, especially
Wikipedia's large baseline scheduling outliers. Fresh executables also
had startup outliers, excluded by warmup. These are reproducible inputs
and measured medians, not isolated-machine performance guarantees.

| Phase, milliseconds (before → after) | ZOL | Wikipedia | GitHub | YouTube |
|---|---:|---:|---:|---:|
| Document/frame fetch | 1.01 → 0.93 | 0.27 → 0.34 | 0.30 → 0.34 | 0.86 → 0.85 |
| Document Unicode decode | 4.96 → 4.84 | 0.49 → 0.55 | 0.38 → 0.39 | 1.88 → 1.89 |
| HTML parse | 21.02 → 21.51 | 3.79 → 4.07 | 2.94 → 2.94 | 7.25 → 7.29 |
| Startup scripts + load events | 181.78 → 191.59 | 9.31 → 8.58 | 3.74 → 3.67 | 826.52 → 839.31 |
| Timer tasks + microtasks | 1235.83 → 146.44 | 0.00 → 0.00 | 0.00 → 0.00 | 0.23 → 0.19 |
| Author sheet load/assembly | 5.76 → 0.69 | 3.93 → 3.64 | 44.23 → 44.59 | 43.13 → 0.00 |
| Sheet parse (nested) | 4.30 → 3.18 | 3.13 → 2.91 | 34.23 → 35.49 | 110.63 → 73.62 |
| Lazy ancestor cascade (nested) | 168.12 → 47.31 | 0.00 → 0.00 | 0.00 → 0.00 | 2.60 → 10.15 |
| Full cascade + selector matching | 642.24 → 338.51 | 1117.90 → 89.72 | 3320.18 → 393.53 | 259.88 → 13.45 |
| Images: fetch + decode | 440.17 → 429.58 | 13.32 → 8.39 | 1.75 → 1.28 | 0.19 → 0.10 |
| Image decode (nested) | 348.83 → 351.64 | 6.29 → 6.09 | 0.40 → 0.38 | 0.00 → 0.00 |
| Layout | 22.23 → 22.00 | 13.36 → 4.66 | 7.42 → 3.85 | 0.37 → 0.24 |
| Display-list build | 0.00 → 0.00 | 0.00 → 0.00 | 0.00 → 0.00 | 0.00 → 0.00 |
| Raster paint | 0.00 → 0.00 | 0.00 → 0.00 | 0.00 → 0.00 | 0.00 → 0.00 |
| Markdown | 1.74 → 2.06 | 0.85 → 0.74 | 0.25 → 0.24 | 0.05 → 0.04 |
| All request calls (nested) | 33.20 → 32.78 | 1.38 → 1.34 | 3.01 → 3.04 | 10.12 → 8.94 |
| Concurrent prefetch batches (nested) | 0.01 → 0.01 | 0.00 → 0.00 | 0.00 → 0.00 | 0.00 → 0.01 |

ZOL is under five seconds **offline**. Wikipedia/GitHub exceed 30%
improvement. YouTube improves 22.1%, **not** 30%; about 90% of its remaining
wall is startup scripts. Interpreter work belongs to the concurrent agent.

## Profile and fixes, in measured order

`--sample --site zol --repeat 1` saves a macOS `sample` report
(`sample-after.txt`, also `sample-before.txt` with `--compare`).
Sampling perturbs execution; do not use sampled runs for the table.
The sampler is bounded/reaped; symbolication is outside child wall time.
Raw `sample PID 3 1 -file report.txt` works too.

1. **Full-DOM frame lookups.** The first ZOL sample's hottest non-idle
   leaf was `frameElements` (842 samples), ahead of `css.matchFrom`
   (533). Window/frame property reads repeatedly walked the document.
   A per-page live frame index caches document order. A follow-up still
   showed 446 frame-lookup leaf samples: image src/style writes invalidated
   the general DOM version. `Document.tree_version` now changes only
   with child/sibling links, not attributes/text data. Insert/remove/
   reorder and allocation-failure fallback preserve live-list behavior.
   ZOL's timer total drops from 1.236 s to 0.146 s.
2. **Cascade scans.** GitHub spent about 3.3 s in cascade. The new
   `css/rule_index.ore` selects required rightmost id/class/type keys;
   positive `:is()`/`:where()` alternatives are recursively indexed.
   Negative, attribute-only, universal, unsupported and depth-limited
   cases fall back. Candidates are deduplicated in source order; the
   original matcher still checks alternatives and their maximum matching
   specificity. Declaration/important logic, pseudos and quirks remain.
   Tests compare candidates with full matching and cover 2,000 unrelated
   rules, alternatives, case/whitespace and mutations.
3. **Duplicate navigation layout.** `newDocument` laid out, then
   `settleScripts` laid out again even with no due task. Skip the second
   pass only in that proven-clean case; actions/tasks still recompute.
4. **Repeated sheet parsing.** YouTube parsed the same large linked
   sheet for style readers and final layout. `sheet_cache.ore` fingerprints
   actual sources/media/order/first base href. Empty styles/preloads do
   not force reparsing; ordinary mutations retain parsed sheets, and full
   layout can take a reader sheet's ownership and compute fresh values.
   Viewport/source changes still reparse. A regression test covers media
   changes (the old reader key omitted media). YouTube sheet parsing drops
   from 111 ms to 74 ms; final author-sheet loading disappears.

The final ZOL sample no longer lists frame lookups or matching among hot
leaf stacks. Image decode/JPEG store dominate (151/110 samples, followed
by Huffman/receive); decode is still about 352 ms. System-font fallback
is not on fixed-font Markdown's hot path. No offscreen image was silently
skipped, and decoder/font output is unchanged.

## Live network check and remaining work

One separate normal live load per site (real Date/random, no archive)
gave these seconds. These are **not paired identical responses**, nor the
offline speedup table:

| Site | Original reported wall (s) | Current live wall (s) | Fetch | Scripts | Timers | Images | Full cascade |
|---|---:|---:|---:|---:|---:|---:|---:|
| zol | 16–20 | 20.399 | 3.863 | 8.647 | 2.561 | 4.855 | 0.319 |
| wikipedia | 5.3 | 2.502 | 0.716 | 0.140 | 0.000 | 1.344 | 0.069 |
| github | 5.2 | 2.694 | 0.925 | 0.004 | 0.000 | 0.459 | 0.602 |
| youtube | 5.0 | 3.879 | 0.890 | 2.896 | 0.000 | 0.000 | 0.015 |

The requested live ZOL target is **not met**: 20.399 s, with 9.228 s in
prefetch and 9.750 s in request calls (overlapping other phases). Only
0.357 s is image decoding. CPU improvements do not remove these
serial/dynamic waits. Wikipedia/GitHub are below the reported starting
times; YouTube's single live check also falls short of a 30% reduction.
Do not mistake offline sub-five-second ZOL for an internet-load guarantee.

Next, in priority order:

- Merge/rebenchmark the JS agent's interpreter work, retaining the same
  snapshots, budgets and output checks; YouTube startup dominates.
- Profile ZOL's live dynamic script/telemetry chain. Separate discovery
  and transfer waits before changing scheduling/concurrency; do not gain
  speed by dropping scripts/resources.
- Separate intrinsic image dimensions from raster decode, then defer
  offscreen decoding until paint. Preserve EXIF orientation, SVG sizes,
  density selection and invalid-image behavior; first add viewport/full-
  page PNG and frame-layout equivalence tests.
- Keep rule indices across computed-value invalidation if measurements
  justify it. A lazy ancestor query on huge sheets can spend more building
  an index than a one-off scan (YouTube's lazy-cascade time rose).
- Dirty-subtree restyle/layout requires ancestor/sibling/inheritance/custom
  property/pseudo dependencies, plus the concurrently changing Shadow DOM,
  flat tree and CSS features. Existing mutations are already batched until
  readers/tasks; subtree-only invalidation is not assumed safe here.
- Profile fallback/shaping caches on system-font screenshots, a different
  workload from fixed-font Markdown.
- Add response-header/cookie journals before authenticated benchmarks.

## Validation

- Release compiler build, `browser/build.sh` (fast) and
  `target/release/ore check browser`: passed.
- `cargo test --test browser`: **40 passed**, including unchanged goldens,
  GC stress, live-frame mutation, timing stdout/PNG equality, media
  invalidation, and replay with the HTTP server killed (CSS/JS/image,
  redirect/failure). Engine/CSS/DOM units cover source keys, candidate
  coverage and structural invalidation.
- Full `cargo test`: passed (31 compiler units, 40 browser tests,
  4 FFI tests, **242 goldens**, 5 spec tests). The existing ThreadSanitizer
  CLI case reports its platform skip; no new skips or golden changes.
- No compiler workaround was needed; `COMPILER_GAPS.md` is unchanged.
  `docs/browser.md` and `docs/browser-sites.md` are untouched.
