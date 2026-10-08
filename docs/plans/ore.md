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
- O2: `target/release/ore fmt --check` passes over every `.ore` file,
  checked by `cargo test --release --test fmt` (P2.4).
- O3: `browser/COMPILER_GAPS.md` "## Open" has no entry: each is fixed
  or declined in `docs/decisions.md`; no workaround remains for a fixed
  gap.
- O4: `cargo test --release --test spec` passes (the spec within ~30k
  tokens), and an M5-style gate run is recorded in `docs/milestones.md`:
  a fresh session writes a CLI using only `ore spec` and `ore check`.
- O5: `docs/milestones.md` has a status table, a row per milestone.

**Ore complete** (the yardstick is Bun, see Reference)

- O6: std has `net` (TCP), `http` (HTTP/1.1 server and client), `json`,
  `path`, `process` (spawn, pipes, exit status, environment), wall-clock
  `time` with RFC 3339 and HTTP dates, `crypto` basics (SHA-256, HMAC,
  OS random bytes) and base64/hex; each has `test` blocks, a
  conformance area in `cargo test --release --test conformance`, and
  `///` docs that `ore doc --signatures std.<m>` prints.
- O7: an app with a local-path and a URL dependency builds with
  `ore build` alone; `ore.lock` pins content hashes; a mismatch fails;
  a rebuild works offline (tests of O4.2, O4.3).
- O8: `ore build --target` makes working programs for aarch64-macos,
  x86_64-macos, wasm32-wasi and wasm32-freestanding (JS exports and
  imports); the Linux triples compile; a std module a target lacks is a
  compile error naming the target (tests of O2.2–O2.4).
- O9: `ore test <dir>` runs every module of a tree, prints a time per
  test, stops a test at a time limit, keeps `--filter`, `--module`,
  `--json` and leak-is-failure (`ore test browser`; tests of O5.1–O5.2).
- O10: `ore lsp` (diagnostics, format, hover with `///`, go to
  definition) and `ore doc --html`, each with a test (O6.1–O6.3).
- O11: `cargo test --release --test conformance` prints passed/total per
  area and target: 100% on native and wasm32-wasi (impossible entries
  listed as such).
- O12: `docs/ore-bench.md` records a hello-world `std.http` server in
  `fast` mode at least as fast as Node 22's `http` (Go `net/http` as the
  second reference, `ab -k`, same machine), and the debug and fast
  build times of `browser/`.
- O13: server gate: a fresh agent with only `ore spec` and
  `ore doc --signatures std` writes a JSON REST service (std.http +
  std.json) that passes `ore check` within two rounds and a scripted
  `curl` check; recorded in `docs/milestones.md`.

Not part of done: self-hosting and the native backend (later stages of
`docs/design.md`), Windows, a package registry or version solving,
built-in database and cloud clients (`@cimport` covers them).

## Current state (2026-10-08)

- Rust stage0: `src/lexer.rs` → `parser.rs` → `module.rs` → `sema.rs` → `oir.rs` → `cgen.rs` → clang (`src/driver.rs`); native target only.
- Commands (`USAGE`, `src/main.rs:6`): build, run, emit-c, check, fix, test, fmt, doc --signatures, cimport, explain, spec.
- std: coro, diag, fs, heap, list, log, map, math, mem, net (IPv4 TCP, O1.3), os, pool, sort, sync, testing, text, thread, time. Missing for O6: http, json, path, process, wall clock, crypto, base64/hex, fs directories and stat.
- std docs (O1.2): new modules take the index form: one line in `docs/spec.md` §15 plus `///` on every `pub fn` (`tests/spec.rs`).
- Tests: `tests/golden.rs` (cases, errors, tests, fix, modules, ffi, cimport), `tests/fmt.rs`, `tests/spec.rs`, `tests/browser.rs`. `tests/conformance.rs` (13 areas, 29 programs, native; O1.1).
- Open gaps: #3 (ASan golden, P2.9), #50 (per-OS declarations; O8 needs it), #52 (methods as fn values), #53 (nested structs, generic methods).
- Toolchain: Homebrew LLVM 23 for ASan and wasm32 (flags in `done.md`, O0.1); Node 22, Go and `ab` are installed locally.
- Stale: `README.md:405-410` says `ore fmt`, `ore doc` and `Pool` are missing (O6.4).

## Phases

| Phase | Goal | Exit check | Status |
|---|---|---|---|
| P2 | Stage0 close-out (O1–O5) | the O1–O5 checks | open: P2.4, P2.9 |
| O1.x | Server basics: conformance harness, `std.net`, `std.http` server, fs/path, process | `cargo test --release --test conformance` prints the table; a hello-world `std.http` golden answers | current |
| O3.x | std breadth: JSON, time, crypto, HTTP client (O6) | the O6 check | next |
| O2.x | Targets incl. wasm32 (O8) | the O8 check | backlog |
| O5.x | `ore test` maturity (O9) | `ore test browser` | backlog |
| O4.x | Dependencies (O7) | the O7 check | backlog |
| O6.x | LSP and HTML docs (O10) | the O10 check | backlog |
| O7.x | Numbers and the gate (O11–O13) | the O11–O13 checks | backlog |

Steps that add std functions touch `docs/spec.md` §15 and the std
sources `tests/spec.rs` reads; they merge one after the other.

## Cards: P2 leftovers

#### P2.4: `ore fmt --check` over the repo
- lane: ore
- goal: every `.ore` file is formatted, and a test keeps it so.
- serves: O2
- files: `tests/fmt.rs` (a repo-wide check, with an explicit exclusion list for files that must not parse, such as `tests/errors`), any `.ore` file (formatting only)
- verify: `cargo test --release --test fmt` → exit 0, the new test listed as ok
- depends on: –
- parallel with: docs-only steps (it reformats many `.ore` files and conflicts with every branch that edits them)
- where: cloud
- model: sonnet
- time box: 45 min
- notes: runs right after X2.2 (decision 2026-10-08: names and style settle before more features). Also `cargo fmt` over the Rust sources (1116 hunks on 2026-10-08, never formatted) with a test or check that keeps it so; one commit per formatter so reviews stay readable.

#### P2.9: AddressSanitizer golden for gap #3
- lane: ore
- goal: under ASan the runtime poisons the freed blocks debug mode and `ore test` hold back, so a read through a stale `List`/`Pool` pointer reports `heap-use-after-free`; a golden checks it in debug and safe mode.
- serves: O3 (gap #3)
- files: `src/cgen.rs` (the debug free queue, `src/cgen.rs:147-240`), `tests/golden.rs`, a case under `tests/`, `browser/COMPILER_GAPS.md` #3
- verify: `cargo test --release --test golden` → exit 0, the new ASan case ok (skipped with a message when no ASan clang is found)
- depends on: –
- parallel with: not with other `src/cgen.rs` steps
- where: local now (Apple clang's ASan hangs here; Homebrew LLVM 23 works: `$(brew --prefix llvm)/bin/clang -fsanitize=address`); cloud after X1.2 (Linux clang has ASan)
- model: sonnet
- time box: 45 min
- notes: model the golden on the ThreadSanitizer one in `tests/golden.rs`. Keep the debug free queue (it catches double free and write-after-free); poison only under ASan. Also try the TSan golden with Homebrew clang (`-fsanitize=thread`): if it runs, M10's last item (milestones.md table) becomes done.

## Cards: O1 server basics

#### O1.5: std.fs breadth
- lane: ore
- goal: `stat` (size, mtime, kind), `readDir`, `mkdir`, `remove`, `rename` in `std.fs`.
- serves: O6
- files: `std/fs/fs.ore`, `docs/spec.md` (fs stays in full in the spec), `tests/cases/`
- verify: `cargo test --release --test golden` → exit 0, with goldens in a temp directory
- depends on: –
- parallel with: O1.3b, O1.4, O1.6 (merge one after the other)
- where: cloud
- model: sonnet
- time box: 45 min
- notes: `stat` must work on macOS and Linux (`st_mtimespec` vs `st_mtim`, the X1.1 problem); if X1.1 chose a helper, reuse it.

#### O1.5a: std.path
- lane: ore
- goal: `join`, `dirname`, `basename`, `extension`, `normalize`, and a check that a path stays under a root.
- serves: O6
- files: `std/path/` (new), `docs/spec.md` §15 (index line)
- verify: `cargo test --release --test golden` → exit 0, with `test` blocks covering `..`, `.`, trailing and doubled slashes
- depends on: –
- parallel with: all std steps (merge one after the other)
- where: cloud
- model: sonnet
- time box: 45 min
- notes: –

#### O1.6: std.process and the environment
- lane: ore
- goal: `spawn(argv)` with stdin/stdout/stderr piped or inherited, `wait` → exit status, `kill`; `std.os.getenv`.
- serves: O6
- files: `std/process/` (new), `std/os/os.ore`, `docs/spec.md`, `tests/cases/`
- verify: `cargo test --release --test golden` → exit 0, with a golden that runs `/bin/echo`, pipes through `cat`, and reads a non-zero exit status
- depends on: –
- parallel with: O1.3b, O1.4, O1.5 (merge one after the other)
- where: cloud
- model: sonnet
- time box: 45 min
- notes: if the time box runs out, commit `spawn`/`wait` and report `getenv` and `kill` as the follow-up.

#### O1.7: first HTTP benchmark
- lane: ore
- goal: hello world on `std.http` in `fast` mode against Go `net/http` and Node 22 `http`, `ab -k -c 50 -n 100000`; if Ore is below Node, propose a diagnose step.
- serves: O12
- files: `docs/ore-bench.md` (new), the three hello servers (under `tests/bench/`, new)
- verify: `test -s docs/ore-bench.md` → exit 0; the table has req/s for Ore, Node and Go, the machine and its load average
- depends on: O1.4a
- parallel with: no build/test agent on the same machine
- where: cloud (a VM to itself is quieter than the shared Mac; the setup script installs Node 22, Go, `ab`)
- model: sonnet
- time box: 45 min
- notes: three runs each, report the median.

## Cards: O3 std breadth (next)

#### O3.1: std.json parse
- lane: ore
- goal: parse JSON into a tree in an arena, with typed accessors that give clear errors.
- serves: O6
- files: `std/json/` (new), `docs/spec.md` §15, `tests/conformance/json/`
- verify: `cargo test --release --test conformance` → exit 0, area `json` with the RFC 8259 valid and invalid cases
- depends on: –
- parallel with: O3.2, O3.3 (merge one after the other)
- where: cloud
- model: sonnet
- time box: 45 min
- notes: `browser/js/json.ore` parses JSON for JS values; a model, not importable.

#### O3.1a: std.json writer
- lane: ore
- goal: write JSON (escaping, numbers, nesting) into a `text.Builder`.
- serves: O6
- files: `std/json/`
- verify: `cargo test --release --test conformance` → exit 0; round trip parse → write → parse equal on the json area
- depends on: O3.1
- parallel with: O3.2, O3.3
- where: cloud
- model: sonnet
- time box: 45 min
- notes: –

#### O3.2: wall clock in std.time
- lane: ore
- goal: `now` (Unix ms), UTC calendar fields, RFC 3339 and IMF-fixdate (HTTP `Date`) format and parse.
- serves: O6
- files: `std/time/time.ore`, `tests/cases/` or `tests/conformance/time/`
- verify: `cargo test --release --test conformance` → exit 0, area `time` with fixed timestamps (leap years, 1970, 2038+)
- depends on: –
- parallel with: O3.1, O3.3
- where: cloud
- model: sonnet
- time box: 45 min
- notes: –

#### O3.3: std.crypto hashes
- lane: ore
- goal: SHA-256, SHA-1 (WebSocket handshake) and HMAC-SHA-256.
- serves: O6
- files: `std/crypto/` (new), `docs/spec.md` §15
- verify: `cargo test --release --test conformance` → exit 0, area `crypto` with the FIPS 180-4 and RFC 4231 vectors
- depends on: –
- parallel with: O3.1, O3.2
- where: cloud
- model: sonnet
- time box: 45 min
- notes: –

#### O3.3a: random bytes, constant-time compare, base64, hex
- lane: ore
- goal: OS random bytes (`getentropy`), constant-time compare, base64 and hex encode/decode.
- serves: O6
- files: `std/crypto/`, `std/text/` or a new module (the step decides, says why)
- verify: `cargo test --release --test conformance` → exit 0, with the RFC 4648 vectors
- depends on: O3.3
- parallel with: O3.1, O3.2
- where: cloud
- model: sonnet
- time box: 45 min
- notes: –

#### O3.4: std.http client (plain HTTP)
- lane: ore
- goal: request, response, chunked bodies over `std.net`; redirects off by default.
- serves: O6
- files: `std/http/`
- verify: `cargo test --release --test golden` → exit 0, with a golden against the O1.4a server
- depends on: O1.4a
- parallel with: O3.1–O3.3
- where: cloud
- model: sonnet
- time box: 45 min
- notes: –

#### O3.4a: decide HTTPS for the client
- lane: ore
- goal: decide how HTTPS comes (libcurl through `@link` as `browser/net/http.ore` does, or the system TLS library on macOS and Linux) and write it down.
- serves: O6
- files: `docs/decisions.md`
- verify: `grep -n 'HTTPS' docs/decisions.md` → the new entry with options, decision, and the O3.5 scope
- depends on: –
- parallel with: all (docs only)
- where: cloud
- model: opus
- time box: 30 min
- notes: `docs/framework.md` "Decisions taken before F0": the client keeps libcurl until O3.4/O3.5 decide; servers end TLS at a reverse proxy.

#### O3.5: HTTPS for the client
- lane: ore
- goal: HTTPS as O3.4a decided.
- serves: O6
- files: `std/http/`
- verify: `cargo test --release --test golden` → exit 0, with a fetch from a local TLS test server
- depends on: O3.4, O3.4a
- parallel with: O3.1–O3.3
- where: cloud
- model: sonnet
- time box: 45 min
- notes: –

## Backlog

Unordered; a card is written when its phase becomes next. Source in
brackets.

- O2.1 diagnose wasm32: toolchain and what the C prelude needs (pthreads, `mmap`, `std.coro` stacks, 32-bit `usize`); target matrix in `docs/decisions.md` (old O2 table; flags in done.md O0.1).
- O2.2 `--target <arch-os>` on build, run, test, emit-c, unknown target = error with code (old O2 table).
- O2.3 wasm32-wasi prelude without threads, coroutines or `mmap`; goldens and conformance under Node 22 `node:wasi` (old O2 table).
- O2.4 wasm32-freestanding: JS exports and imports, page allocator on `memory.grow` (old O2 table).
- O3.6 `std.http` server round 2: query and form decoding, cookies, streamed responses, server-sent events, graceful stop (framework F2/F3, paused).
- O3.7 SQLite through `@cimport("sqlite3.h")` as a `tests/ffi` program (Bun table).
- O4.1 manifest and `ore.lock` design; `docs/framework.md` already fixed "declarative file, not a `build.ore` run" (old O4 table).
- O4.2 local path dependencies: a root imports a module from another root (old O4 table; also the browser split, P8.1).
- O4.3 URL dependencies: fetch, hash check against `ore.lock`, offline build from the cache (old O4 table).
- O5.1 `ore test <dir>` over every module, time per test, summary line; retires `browser/unit_tests.ore` (old O5 table).
- O5.2 per-test CPU-time limit and `-j` (old O5 table; `practices/workflow.md` "A speed test limits CPU time").
- O6.1 `ore lsp` diagnostics and formatting over JSON-RPC (old O6 table).
- O6.2 `ore lsp` hover with `///` and go to definition (old O6 table).
- O6.3 `ore doc --html <path>` (old O6 table; the website X3 uses it).
- O6.4 `README.md:405-410` "Not yet supported" is stale; docs only, tiny (old O6 table).
- O7.1 `docs/ore-bench.md` complete: req/s rerun, `browser/` build times, binary sizes (old O7 table).
- O7.2 server gate O13, recorded in `docs/milestones.md` (old O7 table).
- `std.fs.symlink` and `readLink` (framework F1.2, `docs/framework.md` need 6).
- `Arena.reset` keeping its first block (framework F1.1, need 5).
- Signals for graceful stop (framework F3, need 10).

## Reference

### Gaps (have / missing), 2026-10-08

| Area | Have | Missing |
|---|---|---|
| std: files | `std.fs` read/write, `readFile`, line reader | directories, `stat`/mtime, remove, rename, `std.path` |
| std: processes | `std.os.args`, `exit`; `std.thread` | spawn, pipes, exit status, environment |
| std: network | `std.net` IPv4 TCP (O1.3); the browser's own `browser/ui/socket.ore` and libcurl `browser/net/http.ore` | HTTP/1.1 server and client, HTTPS, host names, IPv6 |
| std: data | `std.text` (split, parse, UTF-8, `Builder`), `std.map` | JSON, base64/hex |
| std: time | monotonic clock, `sleep` | wall clock, calendar, RFC 3339 and HTTP dates |
| std: crypto | nothing | SHA-256, HMAC, OS random bytes |
| Packages | modules are directories; imports resolve under the root | path and URL dependencies, `ore.lock`, cache |
| Targets | native via clang; generated C compiles for x86_64 and aarch64 Linux (M12) | `--target`, wasm32-wasi, wasm32-freestanding |
| `ore test` | test blocks, `--filter`, `--module`, `--json`, leak = failure | whole-tree runs, timings, time limits |
| Tooling | `fmt`, `fix`, `explain`, `doc --signatures`, `check --json`, `spec` | LSP, HTML docs |
| Progress number | goldens, error tests, spec checks | conformance suite per area and target |
| Benchmarks | browser timings in `browser/engine/PERF.md` | Ore-level numbers (HTTP req/s, build times) |

The browser wrote for itself what std lacks: Unix sockets
(`browser/ui/socket.ore`), an HTTP client on libcurl
(`browser/net/http.ore`), JSON for JS values (`browser/js/json.ore`), a
URL parser (`browser/url/url.ore`), encodings (`browser/encoding/`).
Models for std, not code std can import.

### Bun as the yardstick

Bun grew from "a fast runtime" into one binary that is runtime, package
manager, bundler and test runner, with the server-side APIs built in,
and it measures itself with a compat suite and benchmarks:

| Bun | When | Ore counterpart | Criterion |
|---|---|---|---|
| Runtime, `bun run` | 0.1, Jul 2022 | `ore run` (have) | – |
| `Bun.serve` (HTTP, WebSocket), `fetch` | 0.1 → 1.0 | `std.net`, `std.http` server and client | O6 |
| `Bun.file`, `Bun.write` | 1.0, Sep 2023 | `std.fs` breadth, `std.path` | O6 |
| `bun:sqlite` | 0.1 | an `@cimport("sqlite3.h")` example, no std module (C is first-class, design principle 4) | O6 |
| `Bun.password`, crypto | 1.0 | `std.crypto` basics | O6 |
| Bun Shell (`Bun.$`) | 1.0.x, cross-platform in 1.1 | `std.process` | O6 |
| `bun install`; text `bun.lock` | 1.0; 1.2, Jan 2025 | `build.ore` deps by URL, `ore.lock` content hashes (`design.md`, 模組、套件與建置) | O7 |
| `bun build`, `--compile` single executables, cross-compiled | 0.6 → 1.2 | Ore already makes one executable; `--target`, wasm32 | O8 |
| `bun test`; concurrent tests | 1.0; 1.3, Oct 2025 | `ore test` | O9 |
| Node's test suite on every commit, % passing per module | 1.2 | conformance suite, passed/total per area and target | O11 |
| Benchmarks against Node | always | `docs/ore-bench.md` against Node and Go | O12 |
| Windows | 1.1, Apr 2024 | not planned | – |
| S3, Postgres (1.2); MySQL, Redis (1.3) | | out of scope; C libraries through `@cimport` | – |
| Full-stack dev server, HTML imports, hot reload (1.2–1.3) | | the framework lane | `framework.md` |

What does not carry over: Bun runs JavaScript and TypeScript and copies
Node's APIs; Ore compiles ahead of time, so there is no transpiler, no
module loader at run time and no compatibility target. Its compat suite
is its own conformance programs, and the toolchain matters more (LSP,
docs, `--target`) because `design.md` makes "one toolchain, like Go" and
"AI writes it right the first time" goals.

Sources: bun.com/blog/bun-v1.0, bun-v1.1, bun-v1.2, bun-v1.3.
