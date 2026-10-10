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
- O2.5 C types follow the target: `c_long` is 32-bit on wasm32 (and `usize`-sized C types), so `c_union` and `extern_struct` can drop their `X.native` markers (O2.3 report). Verify: those goldens `ok` on wasm.
- O2.7 freestanding runtime gaps (O2.4 sweep): libm and `strtod` in the prelude or as `env` imports, an `env.aye_clock` import for `std.time`, a `snprintf` for `std.json`, single-thread `std.sync` stubs; then more cases get `X.freestanding`. Verify: `std_json` and `math` cases `ok` on wasm32-freestanding.
- O2.8 Linux triples from a non-matching host: the link needs a sysroot (O2.2 report), so `aye build --target x86_64-linux` on a Mac fails at `ld`; add a compile-only path (clang `-c`, or `--emit obj`) or a documented sysroot env var. Verify: a golden that compiles hello for both Linux triples on macOS.
- O2.6 stack overflow on wasm: deep recursion traps with `memory access out of bounds`; a prelude or runner check that reports the native panic text (O2.3 report). Verify: `stack_overflow` drops `X.native`.
- Note (O2.3): Node 22.22.2 segfaults once a wasm heap passes ~32 MiB (plain C reproduces it); the runner uses `--single-threaded-gc`; Node 25 is fine.
- O1.17 suite speed: `tests/browser.rs` takes 283 s of the ~10-minute suite and golden/browser each spawn `available_parallelism()` workers; measure what dominates, add an `AYE_TEST_JOBS` cap honoured by golden, conformance and browser, and report the time per test binary. Verify: `cargo test --release` wall time on a quiet machine, before and after.
- O1.15a `aye doc std.<x>` for an unknown module prints the same module list and suggestion as E0081 (`src/doc.rs`; O1.15 report). Verify: a CLI golden.
- O1.16 `aye fix` removes `;` statement terminators (E0002; one eval run spent ~12 turns on one-line edits). Verify: a `tests/fix` golden.
- O3.6 `std.http` server round 2: query and form decoding, cookies, streamed responses, server-sent events, graceful stop (framework F2/F3, paused).
- O3.7 SQLite through `@cimport("sqlite3.h")` as a `tests/ffi` program (Bun table).
- O4.1 manifest and `aye.lock` design; `docs/framework.md` already fixed "declarative file, not a `build.aye` run" (old O4 table).
- O4.2 local path dependencies: a root imports a module from another root (old O4 table; also the browser split, P8.1).
- O4.3 URL dependencies: fetch, hash check against `aye.lock`, offline build from the cache (old O4 table).
- O5.1a `aye test <dir>`: a subdirectory that is not imported and does not compile on its own fails the whole run (O5.1 report); decide: skip with a note, or a `--strict`. Verify: a golden with a broken lonely module.
- O5.1b the first test of each root takes ~500 ms on macOS (first-exec cost, O5.1 report); measure and, if it is the exe's first launch, start the timer after a warm-up `--list` run. Verify: `treetests` first-test time drops below 50 ms.
- O5.2 per-test CPU-time limit and `-j` (old O5 table; `practices/workflow.md` "A speed test limits CPU time").
- O6.1 `aye lsp` diagnostics and formatting over JSON-RPC (old O6 table).
- O6.2 `aye lsp` hover with `///` and go to definition (old O6 table).
- O6.3 `aye doc --html <path>` (old O6 table; the website X3 uses it).
- O7.1 `docs/aye-bench.md` complete: req/s rerun, `browser/` build times, binary sizes (old O7 table).
- O7.2 server gate O13, recorded in `docs/milestones.md` (old O7 table).
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
