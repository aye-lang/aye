# Done steps

One line per finished step: id, date, result, merge commit (when one
exists). "dropped: reason" marks a step that was superseded or is no
longer needed. Newest phases last; within a phase, by id. The lane plans
([`aye.md`](aye.md), [`browser.md`](browser.md),
[`framework.md`](framework.md)) list only open work.

## Aye lane

| id | date | result | merge |
|---|---|---|---|
| O0.1 | 2026-10-08 | Toolchain note: wasm32 builds with Homebrew LLVM: `$(brew --prefix llvm)/bin/clang --target=wasm32-wasip1 --sysroot=$(brew --prefix wasi-libc)/share/wasi-sysroot -resource-dir=$(brew --prefix wasi-runtimes)/share/wasi-runtimes` (the `wasm32-wasi` triple is gone; without `-resource-dir` the link misses `libclang_rt.builtins.a`); checked on a printf program; no wasm runtime installed yet | – |
| P2.1 | 2026-10-07 | Gap #48: bool-to-integer casts, error test, spec entry, `aye explain` | f4bd8ae |
| P2.2 | 2026-10-07 | Bool casts replace the #48 workarounds in `browser/js/fast_expr.aye` | 9fcc00a |
| P2.3 | 2026-10-07 | Tuples (#21) declined in `decisions.md`; the language fix for #3 decided (its golden is P2.9) | 0995340 |
| P2.5 | 2026-10-08 | `aye spec` ~24.6k tokens (cl100k) of ~30k; M5 gate rerun: a fresh agent wrote `wc` and a JSON tokenizer from the spec alone, `aye check` clean on round one; four loose spots in §6–§8 clarified (P2.5c) | ce06f67 |
| P2.7 | 2026-10-07 | The "flaky" `import()` build was a shared output path between two agents; builds are deterministic | 45d177c |
| P2.8 | 2026-10-07 | Agents build into their own worktree (`browser/build.sh` with no path) | 7bb3d03 |
| P2.10 | 2026-10-07 | `Document.node` uses `unreachable` for a stale handle (`browser/dom/dom.aye`) | 9c909eb |
| O1.2 | 2026-10-08 | std documented in tiers: the spec keeps the CLI modules in full and an index line per other module; `aye doc --signatures std.<m>` is their API; `tests/spec.rs` checks the index and `///` coverage (decision in `docs/decisions.md`, "How std is documented as it grows") | a3498da |
| O1.3 | 2026-10-08 | `std.net`: TCP `listen`/`accept`/`connect`/`read`/`readTimeout`/`write`/`close`, IPv4 only; golden `tests/cases/net_echo.aye`; gap #49 worked around with C helpers in `src/cgen.rs` (→ O1.3a) | 56967d4 |
| O1.9 | 2026-10-08 | `aye` restores the default SIGPIPE at startup: `aye spec \| head` ends quietly; `tests/pipe.rs` | 0c40314 |
| O1.10 | 2026-10-08 | `aye fmt` keeps `extern("x")` (no space); `--help`/`-h`/`help` and `<cmd> --help` print usage and exit 0 | 52b0828 |
| O1.11 | 2026-10-08 | AI-writability eval: `scripts/eval.py` (headless `claude -p`, sonnet, only `aye` tools), 5 tasks in `tests/eval/`, `--self-test`; first run 5/5 pass, results and failure causes in `docs/aye-eval.md` | bccd9bf |
| P2.4 | 2026-10-08 | Every `.aye` file formatted (9 changed, 5 excluded with reasons) and `cargo fmt` (1122 hunks); `tests/fmt.rs` keeps both | 1075c9b |
| O1.8 | 2026-10-08 | (its commit says O1.7, an id clash with the benchmark card) `@tagName(x)`: error, enum and tagged-union tag names as static `[]u8`; E0116 for other types; gap #51 fixed | 323b81d |
| O1.4a | 2026-10-08 | `std.http`: `listen(T, host, port, ctx, handler)` → `Server` with `port()`, `run(alloc)`, `stop()`; thread per connection, keep-alive, per-request arena for the handler; gaps #52, #53 opened; HEAD bodies and 408 left open | f68324a |
| O1.1 | 2026-10-08 | Conformance harness `tests/conformance/<area>/` + `cargo test --release --test conformance` table; 13 areas, 29/29 on native | c7212ac, ba7bb72 |
| O1.4 | 2026-10-08 | `std.http` request parser (pipelining, chunked, size limits → 413/414/431) and response writer; golden `std_http`; gap #51 (no tag-name builtin) opened | fd33758 |
| O1.3b | 2026-10-08 | `std.net` resolves host names (`getaddrinfo`) and IPv6 (`::1`); `UnknownHost` tag; gap #50 (per-OS declarations) opened | d51f147 |
| P2.6 | 2026-10-08 | `docs/milestones.md` opens with a status table: 17 milestones, 16 done, M10 partial (TSan not run on this Mac) | 9478de6 |
| O1.3a | 2026-10-08 | `extern("name") fn` takes a C link name; `std.net` calls libc `listen`/`connect` directly; E0115 for an empty name or a body; errno stays a tiny helper (its symbol differs per OS) | 7b893ee |
| O1.12 | 2026-10-10 | E0022 for `@println(a, b)` and E0006 for `and`/`or`/`not` name the fix (one `@print` per value or `std.text.Builder`; `&&`, `\|\|`, `!`); explanations, two §18 pitfalls, goldens `println_two_args`, `keyword_and`, `keyword_not` | f997b88 |
| O1.13 | 2026-10-10 | `aye spec` prints a ~38.8 KB core (§1, 4, 5, conversions, 6–9, 12, 15, 18) and lists the other sections; `aye spec <name>` (number, first title word, `std.<m>`), `aye spec all`; `SPEC_SPLITS` in `src/main.rs`; four tests in `tests/spec.rs`; §15 points HTTP servers at `std.http` with a hello server | cbd7baa |
| O1.5a | 2026-10-10 | `std.path`: `join`, `dirname`, `basename`, `extension`, `normalize`, `isUnder` (lexical, takes an allocator); golden `tests/tests/std_path.aye` (21 `test` blocks) | 2a963ad |
| O1.5 | 2026-10-10 | `std.fs`: `stat` (lstat: size, mtime, kind), `readDir`, `mkdir`, `remove`, `rename`, `PathError`; C prelude helpers (`aye_std_lstat`, `aye_std_pathop`, dirent wrappers; ENOTEMPTY unified); golden `std_fs_dir` in a temp directory; Linux arm64 golden 269/269; gap #50 hit again | 1303780 |
| O1.6 | 2026-10-10 | `std.process`: `spawn(alloc, argv, stdin, stdout, stderr)` (Inherit/Pipe), `Child.wait` → `Status`, `Child.kill` (SIGTERM), `closeStdin`; pipes are `std.fs.File`; `std.os.getenv`; C prelude `aye_std_spawn`, `aye_std_getenv`; golden `std_process`; Linux golden 270/270 | 94b176e |

## Browser lane

| id | date | result | merge |
|---|---|---|---|
| P0.1 | 2026-10-07 | Three leftover branches (text-transform, baseURI/decode(), Web Animations) closed | – |
| P0.2 | 2026-10-07 | Module tests 243, 245 committed | a9b0a01 |
| P0.3 | 2026-10-07 | Module cycle and re-export tests (233, 234, 242, 244) pass | a9b0a01 |
| P0.4 | 2026-10-07 | Async images: background fetch, decode task, load/error, relayout | 3765949 |
| P0.5 | 2026-10-07 | Async images merged | 3765949 |
| P0.6 | 2026-10-07 | `pi-dynimport` (ES modules) reviewed and merged | a9b0a01 |
| P0.7 | 2026-10-07 | `docs/browser.md` updated once after the B8 merges | c97b200 |
| P0.8 | 2026-10-07 | `main` is the integration branch; branches and worktrees cleaned | 2e108f5 |
| P1.1 | 2026-10-07 | `page_load.py --site all` covers every listed site; archives in `target/sites/` | 0603272 |
| P1.2 | 2026-10-07 | `browser/tools/sites.json` expectations and `page_load.py check` | 66759d6 |
| P1.3 | 2026-10-07 | One agent task per site (`check --agent`) | 54847a0 |
| P1.4 | 2026-10-07 | `run --live` and `--screens` | c8f73af |
| P1.5 | 2026-10-07 | `page_load.py` records child CPU time and the load average | 44b7bf4 |
| P1.7 | 2026-10-07 | Step-limited dump tasks; youtube-search replays deterministically (with P1.9, P1.10) | 1df6b08 |
| P1.8 | 2026-10-07 | Diagnose: youtube-search CPU was 42% a `Date.now` busy loop, 37% an IntersectionObserver polyfill | – |
| P1.9 | 2026-10-07 | Global `IntersectionObserverEntry`, so sites use ours, not a polyfill | 1df6b08 |
| P1.10 | 2026-10-07 | The virtual clock advances inside a task; benchmark `Date.now` follows it | b5364e0 |
| P1.11 | 2026-10-07 | Diagnose: youtube-search profile; gave P4.12–P4.16 | – |
| P1.12 | 2026-10-07 | Replay falls back to a recorded response for unrecorded non-GET bodies | 50f50c4 |
| P1.13 | 2026-10-08 | `page_load.py rebless`; `capture` refuses pages that fail expectations | b5b7c77 |
| P3.1 | 2026-10-07 | Diagnose: YouTube boots; it serves a cookieless visitor an empty home feed (same with curl) | – |
| P3.2 | 2026-10-07 | YouTube home `ok`; `youtube-search` site added | f656d70 |
| P3.3 | 2026-10-08 | dropped: GitHub data loads after P3.13, P3.20, P3.22, P3.26 | – |
| P3.4 | 2026-10-07 | Replaced-element sizing rules (the BBC and Wikipedia ja symptoms were stale) | 41c7c5e |
| P3.5 | 2026-10-07 | dropped: BBC menus did not reproduce on current main (stale symptom) | – |
| P3.6 | 2026-10-08 | dropped: folded into P3.44 (ZOL clips like Chrome) | – |
| P3.8a | 2026-10-08 | Screens review: go.dev icons and rustdoc logo render; rustdoc search → P3.49 | – |
| P3.8b | 2026-10-08 | Screens review: MDN band → P3.47, Wikipedia figure and DDG favicons → P3.45, Google buttons → P3.51 | – |
| P3.11 | 2026-10-07 | The debug perf golden limits CPU time, not wall time | ddd5a92 |
| P3.12 | 2026-10-07 | `http_images_background` waits on the browser, not on sleeps | 48be75c |
| P3.13 | 2026-10-07 | BigInt (GPT via pi) | c9a7b4b |
| P3.14 | 2026-10-08 | dropped: MDN hydration errors diagnosed by P3.47 (typed arrays, fix is P3.58) | – |
| P3.15 | 2026-10-07 | `Request`, full `Headers`, shared body | eb18a8a |
| P3.17 | 2026-10-07 | `fetch` sends any method, body and headers | 3ce1d6b |
| P3.18 | 2026-10-08 | Body mixin and a minimal `ReadableStream` | 62c40ff |
| P3.20 | 2026-10-07 | GitHub and Reddit archives recaptured | 0140fd6 |
| P3.22 | 2026-10-07 | Generic property descriptors keep accessors (GitHub renders again) | b25447a |
| P3.23 | 2026-10-07 | `failed_image_once` waits on the browser, not a sleep | 46b3789 |
| P3.24 | 2026-10-07 | XHR sends any method, body and headers | 7f0ec48 |
| P3.26 | 2026-10-07 | GitHub latest-commit: no engine bug (stale archive); expectations tightened | 9958676 |
| P3.27 | 2026-10-07 | Non-configurable properties follow ValidateAndApplyPropertyDescriptor | 8f06a35 |
| P3.28 | 2026-10-07 | Done inside P3.33 (`ToPropertyDescriptor` checks) | 2fbc144 |
| P3.29 | 2026-10-07 | History API: pushState, popstate, one session history | 0c3d537 |
| P3.30 | 2026-10-07 | DOMParser uses the HTML parser; GitHub and Mistral agent tasks pass (YouTube → P3.35) | 7bf29d7 |
| P3.33 | 2026-10-07 | Reflect methods return false only for rejected operations | 2fbc144 |
| P3.34 | 2026-10-08 | `pushState` with an empty URL; `wait4` error path in `tests/golden.rs` | 5b77c7b |
| P3.36 | 2026-10-07 | Parsed nodes keep their node document until adopted | 67e6009 |
| P3.37 | 2026-10-07 | Replaced elements follow the full CSS 2.1 §10.4 min/max table | 11a9b2e |
| P3.38 | 2026-10-07 | `window_relayout_after_image_load` waits on the first observer report | 4a24037 |
| P3.39 | 2026-10-07 | `setPrototypeOf` rejects cycles and non-extensible targets | 0fa8586 |
| P3.40 | 2026-10-07 | `capture` records what the screenshot needs (fonts, images) | ac62f7f |
| P3.43 | 2026-10-08 | Reflected URLs resolve against the current document base | 3598797 |
| P3.44 | 2026-10-08 | Overflowing flex and grid items have no automatic minimum (GitHub, YouTube search); DDG → P3.52; ZOL is the site's CSS | 01adb4b |
| P3.45 | 2026-10-08 | Screenshots run scripts at the painted viewport size (`browser/shot.aye`); ICO → P3.55, `<video poster>` → P3.56 | 7b9f7ed |
| P3.46 | 2026-10-08 | The `grid-template` shorthand (Wikipedia's page grid) | 41624b8 |
| P3.47 | 2026-10-08 | A closed `<dialog>` is not drawn (MDN); MDN band and BBC band are the sites' CSS; duplicates → P3.58, BBC NEWS width → P3.59 | 4413653 |
| P3.49 | 2026-10-08 | rustdoc: `::before{content:url()}` images and escaped CSS strings (the search input appears only on click, as on the real page) | b4c901b |
| P3.52 | 2026-10-08 | `text-overflow: ellipsis` and `-webkit-line-clamp` (fix: clamp counts lines through child blocks, 943ae56) | 3483049 |
| P3.55 | 2026-10-08 | ICO and CUR decoder | a8513aa |
| P3.57 | 2026-10-08 | `window_agents`: the agent socket appears only once it listens (0/30 failures under load; 2/30 in an earlier run unexplained, keep logs if it recurs) | 31b1242 |
| P3.48 | 2026-10-08 | `document.fonts`, `'prop' in el.style`, `super` fixes, iterable DOMTokenList; MDN and Mistral archives re-captured (the pages now load more); mdn 16 errors | ef3c6bc |
| P3.48b | 2026-10-08 | Bisect: mdn failure = archive misses from chunks the page now loads; Mistral 4 → 40 s = its real animations run (→ P3.63) | – |
| P3.48d | 2026-10-08 | Inline-style changes under a whole-document `:has()` dependency take the subtree restyle (Mistral 40 → 37 s) | 0d7dddf |
| P3.62 | 2026-10-08 | Practices: log the first exception or pending promise before guessing at timing; a fix that gets a page further can raise its failure counts | ac9721d |
| P3.58 | 2026-10-08 | Typed array views share one ArrayBuffer byte store; mdn 17 errors (P3.58a) | f05e682 |
| P4.6 | 2026-10-07 | A page fetches a failed image URL once | abeff4f |
| P4.7 | 2026-10-07 | Diagnose: GitHub's 41 s replay was machine load (CPU 3.4–3.7 s replay vs 3.7–3.9 s live) | – |
| P4.9 | 2026-10-07 | Computed styles reused when no style input changed | 607ceb8 |
| P4.12 | 2026-10-07 | Layout changes deduplicated before the full-restyle cap | 7efb230 |
| P4.13 | 2026-10-07 | The ancestor filter works outside shadow trees | 79cd008 |
| P4.15 | 2026-10-07 | A separate rule index for `::before` and `::after` | 75e3da3 |
| P4.16 | 2026-10-07 | Diagnose: JS interpreter profile on youtube-search; gave P4.17–P4.22 | – |
| P4.17 | 2026-10-07 | The shape table holds 262144 shapes | b7c1d39 |
| P4.18 | 2026-10-07 | Map and Set look up keys through a hash index | 4907663 |
| P4.19 | 2026-10-08 | The document base URL is cached per document | db665cf |
| P4.20 | 2026-10-08 | Property cache on DOM wrappers: hits 54% → 59%, CPU flat; not merged; kept the invalidation test and a practices entry | 4117076 (test only) |
| P4.22 | 2026-10-07 | Diagnose: ~1.7 GB live JS heap on youtube-search; lead (mutation log) checked in P4.23 | – |
| P4.23 | 2026-10-07 | Diagnose: memory follows DOM size (endless feed scrolling); gave P4.25 | – |
| P4.25 | 2026-10-08 | Dumps stop after two growth rounds on endless pages | f07db86 |
| P4.26 | 2026-10-08 | Diagnose: ~8 KB of our structures per element (computed style ~5.7 KB); gave P4.27–P4.29 | – |
| P4.27 | 2026-10-08 | Elements share identical computed-style arrays | 9179802 |

## Framework lane

| id | date | result | merge |
|---|---|---|---|
| F0.1 | 2026-10-08 | Design doc [`docs/framework.md`](../framework.md); open choices answered; the user's approval of its three questions is pending | 0124e64 |
| F1.1 | 2026-10-10 | `framework/web` (request, response, `Html`, literal route table, `web.run` over `std.http`), `framework/examples/hello/`, `tests/framework.rs` (`/` 200, `/nope` 404); no compiler gap hit | ef6f47b |

## Project lane

| id | date | result | merge |
|---|---|---|---|
| X3 v1 | 2026-10-08 | ayelang.dev landing page live on a Cloudflare Worker (`site/`) | d901896 |
| X2.1 | 2026-10-08 | `scripts/rename-aye.py` with `--dry-run`/`--apply`; trial apply passed the full suite | 76eb976 |
| X2.2 | 2026-10-08 | Ore → Aye: `aye` command, crate and binary, `.aye` files, `AYE_`/`aye_` names, docs; 513 moved, 321 rewritten | 3042d53 |
| X5.1 | 2026-10-08 | `window_forms_html5` loads a copy in `/tmp/anyknown-forms-html5/`, so its golden PNG no longer holds the checkout path | 4d4ea93 |
| X5.2 | 2026-10-08 | Checkout moved to `~/Documents/ayelang/aye`; full suite green there after `cargo clean --release -p aye` (test binaries kept the old `CARGO_BIN_EXE_aye` path) | – |
| X1.1 | 2026-10-08 | `std.fs.mtime(path) ?i64` (C prelude `aye_std_mtime`, `struct stat` offset per OS: 48 macOS, 88 Linux); `archive.aye` uses it and `strlen(d_name)`; gap #50 stays open for a language-level fix | 0e61757 |
| X1.2 | 2026-10-08 | `scripts/setup-ubuntu.sh`, `Containerfile`, `scripts/linux.sh` (Apple `container`, Ubuntu 24.04, target in a named volume); first Linux run recorded in `project.md` X1 | 02c42d3 |
| X1.4 | 2026-10-08 | `scripts/linux.sh`: git works in worktrees (common git dir mounted read-only), image tag carries a hash of `scripts/linux/Containerfile` + `setup-ubuntu.sh`, TSan runtime installed; Linux fmt 7/7, TSan golden runs | aec5554 |
| X1.3 | 2026-10-08 | `@cimport` of a system header takes its internal headers (`_*.h`, `bits/`) and its own subdirectory, not the rest of the include dir (gap #54): glibc `math.h` and `pthread.h` work; `GENERATOR_VERSION` 2 | 7a08c4b |
| X1.6 | 2026-10-08 | `Math.cbrt` is a port of fdlibm `cbrt` (`browser/js/cbrt.aye`): same result on glibc and macOS; more cbrt cases in `164_math_functions.js` | 2ea52ca |
| X1.7 | 2026-10-08 | `tests/golden.rs` runs cases on `available_parallelism()` workers instead of one thread per case: amd64 golden under Rosetta 264/264 (was a 29-min hang); macOS golden 132 s → 146 s | b2c723d |
| X1.5 | 2026-10-08 | X1 done: on main b2c723d, `scripts/linux.sh` arm64 and `LINUX_ARCH=amd64` (Rosetta): build, `aye check browser`, `cargo test --release` (browser 62/62, golden 264/264, all others ok) and `browser/build.sh` exit 0 | – |
