# Plan: the Ore (Aye) lane

The language lane of [`docs/plan.md`](../plan.md): the compiler
(`src/`), std (`std/`), `tests/` and the toolchain. Primary lane: it
gets most slots. Its inputs are the criteria below, the open entries of
`browser/COMPILER_GAPS.md`, and F1.1 (the acceptance test of
`std.http`). Finished steps: [`done.md`](done.md).

## Done criteria

Criteria are named O1–O13 (not to be confused with phases O1.x–O7.x).
The browser resumes when O6–O9 hold.

**Stage0 close-out**

- O1: `cargo test --release` exits 0 on a clean checkout of `main`, on
  macOS and, after X1.2, on Ubuntu 24.04.
- O2: `target/release/aye fmt --check` passes over every `.aye` file,
  checked by `cargo test --release --test fmt` (P2.4).
- O3: `browser/COMPILER_GAPS.md` "## Open" has no entry: each is fixed
  or declined in `docs/decisions.md`; no workaround remains for a fixed
  gap.
- O4: `cargo test --release --test spec` passes (the spec within ~30k
  tokens), and an M5-style gate run is recorded in `docs/milestones.md`:
  a fresh session writes a CLI using only `aye spec` and `aye check`.
- O5: `docs/milestones.md` has a status table, a row per milestone.

**Aye complete** (the yardstick is Bun, see Reference)

- O6: std has `net` (TCP), `http` (HTTP/1.1 server and client), `json`,
  `path`, `process` (spawn, pipes, exit status, environment), wall-clock
  `time` with RFC 3339 and HTTP dates, `crypto` basics (SHA-256, HMAC,
  OS random bytes) and base64/hex; each has `test` blocks, a
  conformance area in `cargo test --release --test conformance`, and
  `///` docs that `aye doc --signatures std.<m>` prints.
- O7: an app with a local-path and a URL dependency builds with
  `aye build` alone; `aye.lock` pins content hashes; a mismatch fails;
  a rebuild works offline (tests of O4.2, O4.3).
- O8: `aye build --target` makes working programs for aarch64-macos,
  x86_64-macos, wasm32-wasi and wasm32-freestanding (JS exports and
  imports); the Linux triples compile; a std module a target lacks is a
  compile error naming the target (tests of O2.2–O2.4).
- O9: `aye test <dir>` runs every module of a tree, prints a time per
  test, stops a test at a time limit, keeps `--filter`, `--module`,
  `--json` and leak-is-failure (`aye test browser`; tests of O5.1–O5.2).
- O10: `aye lsp` (diagnostics, format, hover with `///`, go to
  definition) and `aye doc --html`, each with a test (O6.1–O6.3).
- O11: `cargo test --release --test conformance` prints passed/total per
  area and target: 100% on native and wasm32-wasi (impossible entries
  listed as such).
- O12: `docs/aye-bench.md` records a hello-world `std.http` server in
  `fast` mode at least as fast as Node 22's `http` (Go `net/http` as the
  second reference, `ab -k`, same machine), and the debug and fast
  build times of `browser/`.
- O13: server gate: a fresh agent with only `aye spec` and
  `aye doc --signatures std` writes a JSON REST service (std.http +
  std.json) that passes `aye check` within two rounds and a scripted
  `curl` check; recorded in `docs/milestones.md`.

Not part of done: self-hosting and the native backend (later stages of
`docs/design.md`), Windows, a package registry or version solving,
built-in database and cloud clients (`@cimport` covers them).

## Current state (2026-10-10)

- Rust stage0: `src/lexer.rs` → `parser.rs` → `module.rs` → `sema.rs` → `oir.rs` → `cgen.rs` → clang (`src/driver.rs`); `--target` (O2.2, O2.3): the host, the other native triples, wasm32-wasip1 (std prelude branch, `node:wasi` runner, golden and conformance coverage), wasm32-freestanding (O2.4: JS imports/exports, `tests/wasm/loader.mjs`).
- Commands (`USAGE`, `src/main.rs:6`): build, run, emit-c, check, fix, test, fmt, doc --signatures, cimport, explain, spec.
- std: coro, diag, fs, heap, http (O1.4a), list, log, map, math, mem, net (O1.3, O1.3b), os, path (O1.5a), pool, process (O1.6), sort, sync, testing, text, thread, time. json (O3.1), time with a wall clock (O3.2), crypto hashes (O3.3). json writer (O3.1a), random bytes and encoding (O3.3a), http client with https (O3.4, O3.5). **O6 holds** (checked 2026-10-10 on bf4c5d8, `done.md`).
- std docs (O1.2): new modules take the index form: one line in `docs/spec.md` §15 plus `///` on every `pub fn` (`tests/spec.rs`).
- Tests: `tests/golden.rs` (cases, errors, tests, fix, modules, ffi, cimport; `X.needs`/`X.tools` skip files), `tests/fmt.rs`, `tests/spec.rs`, `tests/browser.rs`, `tests/framework.rs`. `tests/conformance.rs` (18 areas, 51 programs, native).
- Open gaps: #50 (per-OS declarations; O8 needs it), #52 (methods as fn values), #53 (nested structs, generic methods).
- Toolchain: Homebrew LLVM 23 for ASan and wasm32 (flags in `done.md`, O0.1); Node 22, Go and `ab` are installed locally.

## Phases

| Phase | Goal | Exit check | Status |
|---|---|---|---|
| P2 | Stage0 close-out (O1–O5) | the O1–O5 checks | open: O3 (gaps #50, #52, #53) |
| O1.x | Server basics: conformance harness, `std.net`, `std.http` server, fs/path, process | `cargo test --release --test conformance` prints the table; a hello-world `std.http` golden answers | done 2026-10-10 |
| O3.x | std breadth: JSON, time, crypto, HTTP client (O6) | the O6 check | done 2026-10-10 (O6 holds) |
| O2.x | Targets incl. wasm32 (O8) | the O8 check | current |
| O5.x | `aye test` maturity (O9) | `aye test browser` | backlog |
| O4.x | Dependencies (O7) | the O7 check | backlog |
| O6.x | LSP and HTML docs (O10) | the O10 check | backlog |
| O7.x | Numbers and the gate (O11–O13) | the O11–O13 checks | backlog |

Steps that add std functions touch `docs/spec.md` §15 and the std
sources `tests/spec.rs` reads; they merge one after the other.

## Cards: P2 leftovers

## Cards: O1 server basics

#### O1.7: first HTTP benchmark
- lane: aye
- goal: hello world on `std.http` in `fast` mode against Go `net/http` and Node 22 `http`, `ab -k -c 50 -n 100000`; if Aye is below Node, propose a diagnose step.
- serves: O12
- files: `docs/aye-bench.md` (new), the three hello servers (under `tests/bench/`, new)
- verify: `test -s docs/aye-bench.md` → exit 0; the table has req/s for Aye, Node and Go, the machine and its load average
- depends on: O1.4a
- parallel with: no build/test agent on the same machine
- where: cloud (a VM to itself is quieter than the shared Mac; the setup script installs Node 22, Go, `ab`)
- model: sonnet
- time box: 45 min
- notes: three runs each, report the median.

## Cards: O4 dependencies (O4.1–O4.3 done; design in decisions.md "Dependencies (O4)")

## Cards: O2 targets (O2.1–O2.4 done; leftovers in the backlog)

## Cards: O3 std breadth (done; leftovers in the backlog)

## Backlog

Unordered; a card is written when its phase becomes next. Source in
brackets.

- O1.14 (gap #50, from X1.1) a target-OS constant the checker uses to skip the other OS's code, so per-OS declarations live in Aye; then remove `aye_std_errno`, `aye_std_mtime` offsets and the `isDarwin` checks in `std/net`, `browser/ui/socket.aye`, and the hand-written `clock() u64` in `browser/js/interp.aye:107` (`clock_t` is unsigned on macOS, signed on glibc). Needed by O8. Verify: `cargo test --release --test golden` and `aye check browser` on macOS and via `scripts/linux.sh`.
- O3.5a `std.net` exports `poll`/`PollFd` or a `Conn` wait method so the TLS transport can wait on `CURLINFO_ACTIVESOCKET` instead of a sleep backoff; a `std.log.debug` level or `std.http` `lastError` instead of `AYE_HTTP_DEBUG` (O3.5 report).
- O3.4b `std.net.connect` timeout (a golden against an unroutable address); the http client golden against an IPv6 (`::1`) server (O3.4 report).
- O3.3b `std.encoding` strict base64 (reject non-zero trailing bits) if tokens or signatures need it; the crypto conformance programs use `std.encoding.hexEncode` instead of private `hex` helpers (O3.3a report). Verify: `cargo test --release --test conformance`.
- O3.1c `std.text.Builder.appendFloatExact` (shortest round-trip float formatting, lifted from `std.json.writeNumber`); `tests/conformance/json/valid.aye` prints through `json.write` instead of its private `render` (O3.1a report). Verify: `cargo test --release --test conformance`.
- O3.2a `std.time.parseHttp` lenient forms (RFC 850, asctime) if the HTTP client needs them; a `Date` header helper in `std.http` from `now()` (O3.2 report).
- O3.1b `std.json.Value.asInt() ?i64` for exact integers above 2^53, if O6/O13 need it (O3.1 report).
- O1.6a `std.process`: suppress SIGPIPE on a write to an exited child's stdin (a `WriteError`, as `std.net` does per socket); `Child.killHard` (SIGKILL), custom environment and working directory when a caller needs them (O1.6 report). Verify: a golden that writes to an exited child.
- P8.x (browser, when it resumes) `browser/net/archive.aye` uses `std.fs.readDir`/`stat` instead of its `@cimport` of dirent.h and sys/stat.h (O1.5 report).
- O2.5a std casts on wasm32: audit the remaining `as usize` casts of 64-bit values in std (O2.5 found 23, all small values today) and `aye cimport` output (`size_t`→`usize`, `long`→`c_long`) under `--target wasm32-wasip1`. Verify: a `cimport` golden built for wasm.
- O2.5e a std golden that exercises the x86_64 Linux `struct stat` mode offset directly (PR #2 follow-up). Verify: `std_fs_dir` kinds on x86_64 Linux.
- P-flaky `tests/browser.rs` `window_relayout_after_image_load` failed once in three Linux full runs (observer callback logged before image `onload`), diagnose step (PR #1).
- O2.8a one-step Linux executable from macOS: `--sysroot` pass-through plus `-fuse-ld=lld` (O2.8 report; the `-c` path already satisfies O8). Verify: `aye build --target x86_64-linux hello.aye` links on macOS with a Linux sysroot set.
- O2.6 stack overflow on wasm: deep recursion traps with `memory access out of bounds`; a prelude or runner check that reports the native panic text (O2.3 report). Verify: `stack_overflow` drops `X.native`.
- Note (O2.3): Node 22.22.2 segfaults once a wasm heap passes ~32 MiB (plain C reproduces it); the runner uses `--single-threaded-gc`; Node 25 is fine.
- O1.17b closed (decisions.md "Round 2"): macOS syspolicyd assesses 2-3 new executables at a time, golden links 396, ~150 s of serial stall; `-Wl,-no_adhoc_codesign` kills the binary, `-mmacosx-version-min` no effect, mode overlap inside the noise. Decision: macOS golden stays; heavy verification runs on Linux (cloud lane, golden 84 s there); the user may try the Developer Tools exemption for the terminal app.
- O1.19a a value `match` whose arms all `return`/`break`/`continue` gets type `void`, so `let v = match ...` fails downstream with an unhelpful message (O1.19 report). Verify: an `errors` golden with a clear E-code.
- O1.20 `aye check <missing file>` says to write the file first (eval: http_hello probes `aye check` on a missing file every run; low value). Verify: a CLI golden.
- O1.17c `virtual_clock.html` is 22% of `jsdom_gc_stress` CPU; make the page cheaper under GC stress without losing what it tests.
- O3.6 `std.http` server round 2: query and form decoding, cookies, streamed responses, server-sent events, graceful stop (framework F2/F3, paused).
- O3.7 SQLite through `@cimport("sqlite3.h")` as a `tests/ffi` program (Bun table).
- O5.1b the first test of each root takes ~500 ms on macOS (first-exec cost, O5.1 report); measure and, if it is the exe's first launch, start the timer after a warm-up `--list` run. Verify: `treetests` first-test time drops below 50 ms.
- O5.2e parallel C compile: `cgen` emits one shared header (types, prototypes, externs, string/const tables) plus K body files with non-`static` functions (K = min(cores, 8), functions distributed round-robin or by size), `cc` runs K `clang -c` at once and one link; `emit-c` keeps a single file. Expected 6.3 s → ~2 s for the browser test build, debug `aye build browser` similar; CPU +15% (no dead-function dropping). Keep `-g`. Verify: golden, browser suite, `time aye test browser` median before/after. (O5.2d)
- O5.2f drop `-g` for `aye test` executables unless `--cflags` is given (−45% on top of O5.2e; no code reads DWARF). Decide after O5.2e.
- O5.2b per-test `ms` is inflated under `-j` by the first-exec cost of a fresh binary (~0.5 s on macOS, same cause as O1.17b and O5.1b); warm the binary once before fanning out. Verify: `treetests` times under 50 ms after the first.
- O6.2a lsp navigation leftovers (O6.2 report): share the declaration renderer between `src/doc.rs` and `src/lsp/nav.rs`; `x.method()`/fields of a value need types (sema records resolved member spans); block scopes; `aye.toml` deps and `@cimport` modules; cursor just past an identifier should count as on it. Verify: tests/lsp.rs cases.
- O6.3a `aye doc --html` leftovers (O6.3 report): struct member `///` comments stay inside the signature block instead of becoming paragraphs; root page name is the path stem (`index` → `index_module.html`); the website (X3, `site/`) should use it for the std reference. Verify: a `shapes` fixture change plus the site build.
- O6.1a `aye lsp` follow-ups (O6.1 report): end spans on `Diag` (the `token_len` lookup is a stopgap), a severity field, diagnostics of imported files as `relatedInformation`, fix-its as `textDocument/codeAction`, a debounce for large roots. Verify: tests/lsp.rs cases.
- O3.4b `std.http`: a `HEAD` response must carry the `Content-Length` the GET body would have, with no body; today `Server` writes the length from `res.body.len` and forbids a user `Content-Length`, so the framework's HEAD sends `Content-Length: 0` (F1.1c report). Add a HEAD mode (or a length override) and update `tests/framework/hello/head.http`. Verify: framework test and a std_http golden.
- O3.9 `std.http` under concurrency is 0.4x Go (thread per connection, arena per request; `aye-bench.md`): (a) profile at `-c 64` (context switches, allocations per request); (b) a fixed worker pool in the accept loop; (c) reuse the per-request arena across keep-alive requests; (d) rerun on 8+ cores with a separate load generator (cloud). Verify: `scripts/bench/run.sh` req/s before/after.
- O1.11b `scripts/eval.py --tasks a,b` rewrites the whole results table in `docs/aye-eval.md` with only the tasks it ran (O7.2 report); merge per task instead. Verify: a `--tasks rest_json` run keeps the other rows.
- O1.11c the eval harness `SYSTEM_PROMPT` lists `aye spec`, `aye check`, `aye explain` but not `aye doc --signatures <module>` (O7.2 report; O13 names it). Add it. Verify: a run's transcript shows the agent found `aye doc` from the system prompt.
- `std.fs.symlink` and `readLink` (framework F1.2, `docs/framework.md` need 6).
- `Arena.reset` keeping its first block (framework need 5; F1.1 did not need it, later phases may).
- Signals for graceful stop (framework F3, need 10).

## Reference

### Gaps (have / missing), 2026-10-08

| Area | Have | Missing |
|---|---|---|
| std: files | `std.fs` read/write, `readFile`, line reader | directories, `stat`/mtime, remove, rename, `std.path` |
| std: processes | `std.os.args`, `exit`; `std.thread` | spawn, pipes, exit status, environment |
| std: network | `std.net` IPv4 TCP (O1.3); the browser's own `browser/ui/socket.aye` and libcurl `browser/net/http.aye` | HTTP/1.1 server and client, HTTPS, host names, IPv6 |
| std: data | `std.text` (split, parse, UTF-8, `Builder`), `std.map` | JSON, base64/hex |
| std: time | monotonic clock, `sleep` | wall clock, calendar, RFC 3339 and HTTP dates |
| std: crypto | nothing | SHA-256, HMAC, OS random bytes |
| Packages | modules are directories; imports resolve under the root | path and URL dependencies, `aye.lock`, cache |
| Targets | native via clang; generated C compiles for x86_64 and aarch64 Linux (M12) | `--target`, wasm32-wasi, wasm32-freestanding |
| `aye test` | test blocks, `--filter`, `--module`, `--json`, leak = failure | whole-tree runs, timings, time limits |
| Tooling | `fmt`, `fix`, `explain`, `doc --signatures`, `check --json`, `spec` | LSP, HTML docs |
| Progress number | goldens, error tests, spec checks | conformance suite per area and target |
| Benchmarks | browser timings in `browser/engine/PERF.md` | Aye-level numbers (HTTP req/s, build times) |

The browser wrote for itself what std lacks: Unix sockets
(`browser/ui/socket.aye`), an HTTP client on libcurl
(`browser/net/http.aye`), JSON for JS values (`browser/js/json.aye`), a
URL parser (`browser/url/url.aye`), encodings (`browser/encoding/`).
Models for std, not code std can import.

### Bun as the yardstick

Bun grew from "a fast runtime" into one binary that is runtime, package
manager, bundler and test runner, with the server-side APIs built in,
and it measures itself with a compat suite and benchmarks:

| Bun | When | Aye counterpart | Criterion |
|---|---|---|---|
| Runtime, `bun run` | 0.1, Jul 2022 | `aye run` (have) | – |
| `Bun.serve` (HTTP, WebSocket), `fetch` | 0.1 → 1.0 | `std.net`, `std.http` server and client | O6 |
| `Bun.file`, `Bun.write` | 1.0, Sep 2023 | `std.fs` breadth, `std.path` | O6 |
| `bun:sqlite` | 0.1 | an `@cimport("sqlite3.h")` example, no std module (C is first-class, design principle 4) | O6 |
| `Bun.password`, crypto | 1.0 | `std.crypto` basics | O6 |
| Bun Shell (`Bun.$`) | 1.0.x, cross-platform in 1.1 | `std.process` | O6 |
| `bun install`; text `bun.lock` | 1.0; 1.2, Jan 2025 | `build.aye` deps by URL, `aye.lock` content hashes (`design.md`, 模組、套件與建置) | O7 |
| `bun build`, `--compile` single executables, cross-compiled | 0.6 → 1.2 | Aye already makes one executable; `--target`, wasm32 | O8 |
| `bun test`; concurrent tests | 1.0; 1.3, Oct 2025 | `aye test` | O9 |
| Node's test suite on every commit, % passing per module | 1.2 | conformance suite, passed/total per area and target | O11 |
| Benchmarks against Node | always | `docs/aye-bench.md` against Node and Go | O12 |
| Windows | 1.1, Apr 2024 | not planned | – |
| S3, Postgres (1.2); MySQL, Redis (1.3) | | out of scope; C libraries through `@cimport` | – |
| Full-stack dev server, HTML imports, hot reload (1.2–1.3) | | the framework lane | `framework.md` |

What does not carry over: Bun runs JavaScript and TypeScript and copies
Node's APIs; Aye compiles ahead of time, so there is no transpiler, no
module loader at run time and no compatibility target. Its compat suite
is its own conformance programs, and the toolchain matters more (LSP,
docs, `--target`) because `design.md` makes "one toolchain, like Go" and
"AI writes it right the first time" goals.

Sources: bun.com/blog/bun-v1.0, bun-v1.1, bun-v1.2, bun-v1.3.
