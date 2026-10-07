# Plan: the Ore lane

The Ore lane of [`docs/plan.md`](../plan.md): the compiler (`src/`), std
(`std/`), the tests in `tests/` and the toolchain. It takes its work from
three places: its own "done" list below, `browser/COMPILER_GAPS.md` (gaps
the browser and the framework lanes record), and the framework lane's
needs ([`framework.md`](framework.md)), which set the order of the phases.
Phase P2 moved here from `docs/plan.md` on 2026-10-08 with its ids; new
rows use the prefix `O` (O1.1, ...).

## What "done" means

**Stage0 close-out (phase P2, unchanged)**

- O1: `cargo test` passes on a clean checkout of `main`.
- O2: `ore fmt --check` passes over every `.ore` file in the repo (M9).
- O3: every open entry in `browser/COMPILER_GAPS.md` is fixed or
  declined in `docs/decisions.md`; no workaround remains for a fixed gap.
- O4: `ore spec` fits the ~30k-token budget (`milestones.md`, part 2),
  and the M5-style gate passes again: a fresh session writes a CLI using
  only `ore spec` and `ore check`.
- O5: `docs/milestones.md` has a status table.

**Ore complete** (the yardstick is Bun, see below; each item is checked
by a command or a recorded run)

- O6: std has `net` (TCP), `http` (HTTP/1.1 server and client), `json`,
  `path`, `process` (spawn, pipes, exit status, environment), wall-clock
  `time` with RFC 3339 and HTTP dates, `crypto` basics (SHA-256, HMAC,
  OS random bytes) and base64/hex, next to today's modules; each has
  `test` blocks, conformance programs (O11) and documentation (O1.2).
- O7: dependencies: an app with a local-path dependency and a URL
  dependency builds with `ore build` alone; `ore.lock` pins content
  hashes; a hash mismatch fails; a rebuild works offline from the cache.
- O8: targets: `ore build --target` makes working programs for
  aarch64-macos, x86_64-macos, wasm32-wasi and wasm32-freestanding (with
  exports to and imports from JS); the Linux triples compile. A target
  that lacks a std module (threads on wasm32) gives a compile error that
  names the target, not a link error.
- O9: `ore test` runs every module of a tree in one command, reports a
  time per test, stops a test at a time limit, and keeps today's
  `--filter`, `--module`, `--json` and leak-is-failure behavior.
- O10: tooling: `ore lsp` (diagnostics, format, hover with the `///`
  comment, go to definition) and `ore doc --html`; `ore fmt`, `ore fix`,
  `ore explain` and `ore spec` stay as they are.
- O11: the progress number: `cargo test --test conformance` prints
  passed/total per area and per target. Ore is complete at 100% on the
  native target and on wasm32-wasi (entries a target cannot have are
  listed as such, not counted as failures).
- O12: benchmarks recorded in `docs/ore-bench.md`: a hello-world
  `std.http` server in `fast` mode serves at least as many requests per
  second as Node 22's `http` module, with Go's `net/http` as the second
  reference (`ab -k`, same machine, quiet); the debug and fast build
  times of `browser/`.
- O13: the dogfooding gate again, for servers: a fresh agent, given only
  `ore spec` and `ore doc --signatures std`, writes a JSON REST service
  (std.http + std.json) that passes `ore check` within two rounds and a
  scripted `curl` check. Recorded in `docs/milestones.md`.

Not part of "done": self-hosting and the native backend (later stages
of `docs/design.md`), Windows, a package registry or version solving
(`design.md` rules them out for the first version), built-in database
and cloud clients (C libraries through `@cimport` cover them).

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
| Windows | 1.1, Apr 2024 | not planned (macOS only, see Decisions in `docs/plan.md`) | – |
| S3, Postgres (1.2); MySQL, Redis (1.3) | | out of scope; C libraries through `@cimport` | – |
| Full-stack dev server, HTML imports, hot reload (1.2–1.3) | | the framework lane | `framework.md` |

What does not carry over: Bun runs JavaScript and TypeScript and copies
Node's APIs; Ore compiles ahead of time, so there is no transpiler, no
module loader at run time and no compatibility target. Its compat suite
is its own conformance programs, and the toolchain matters more (LSP,
docs, `--target`) because `design.md` makes "one toolchain, like Go" and
"AI writes it right the first time" goals.

Sources: bun.com/blog/bun-v1.0, bun-v1.1, bun-v1.2, bun-v1.3.

## Current state (2026-10-08)

- **Compiler:** Rust stage0, `src/lexer.rs` → `src/parser.rs` →
  `src/module.rs` → `src/sema.rs` → `src/oir.rs` → `src/cgen.rs` → clang,
  run by `src/driver.rs` (`Command::new("clang")`, `src/driver.rs:201`).
  It passes no `--target`: native code only.
- **Commands** (`USAGE`, `src/main.rs:6`): `build`, `run`, `emit-c`,
  `check [--json]`, `fix`, `test [--filter] [--module] [--json]`,
  `fmt [--check]`, `doc --signatures`, `cimport`, `explain`, `spec`;
  `--mode debug|safe|fast` and `--cflags`. Docs: `README.md`, "Build
  and run".
- **std** (`std/`, embedded in the compiler, ~2.2k lines): `coro`,
  `diag`, `fs` (open, create, exists, readFile, std streams, line
  reader; no directories, stat, remove or rename), `heap`, `list`,
  `log`, `map`, `math`, `mem`, `os` (`args`, `exit` only, no
  environment), `pool`, `sort`, `sync`, `testing`, `text`, `thread`,
  `time` (`monotonicMs`, `sleep`; no wall clock). No `net`, `http`,
  `json`, `path`, `process` or `crypto`.
- **What the browser wrote for itself:** Unix sockets through
  `extern fn` (`browser/ui/socket.ore`), an HTTP client on libcurl
  (`browser/net/http.ore`), JSON for JS values (`browser/js/json.ore`),
  a URL parser (`browser/url/url.ore`), encodings (`browser/encoding/`).
  Models for std, not code std can import.
- **Tests:** `cargo test` runs `tests/golden.rs` (104 programs in
  `tests/cases`, three modes; `tests/errors`, `tests/tests` through
  `ore test`, `tests/fix`, 21 `tests/modules`, `tests/example1`,
  `tests/ffi`, `tests/cimport`), `tests/fmt.rs`, `tests/spec.rs` (spec
  ≤ 30k tokens and names every pub function of std) and
  `tests/browser.rs`. `ore test` runs the root module's tests;
  `browser/unit_tests.ore` exists because modules that import others
  cannot be test roots.
- **Packages:** none. `README.md:408` lists `build.ore` and package
  dependencies as not supported; `build.ore` as `design.md` describes it
  needs compile-time execution, which stage0 has only for type functions
  (`README.md:405`).
- **wasm32:** Apple clang 17 here has no WebAssembly backend
  (`clang --print-targets` lists none); Homebrew LLVM with `lld` and a
  wasi-libc would be needed. The C prelude uses pthreads
  (`src/cgen.rs:535`), `mmap` (`src/cgen.rs:467`) and per-target stack
  switching for `std.coro` (M12). Node 22 is installed and can run
  wasm32-wasi programs (`node:wasi`); Go and `ab` are installed for
  benchmarks.
- **Gaps:** one open entry in `browser/COMPILER_GAPS.md` (#3, blocked,
  P2.9); #48 is listed as fixed.
- **Stale:** `README.md:405-410` ("Not yet supported") still says
  `ore fmt` and `ore doc` are missing and std has no `Pool` (O6.4).

## Gaps (have / missing)

| Area | Have | Missing |
|---|---|---|
| std: files | `std.fs` read/write, `readFile`, line reader | directories, `stat`/mtime, remove, rename, `std.path` |
| std: processes | `std.os.args`, `exit`; `std.thread` | spawn, pipes, exit status, environment |
| std: network | nothing (browser: `browser/ui/socket.ore`, libcurl) | TCP, HTTP/1.1 server and client, HTTPS |
| std: data | `std.text` (split, parse, UTF-8, `Builder`), `std.map` | JSON, base64/hex |
| std: time | monotonic clock, `sleep` | wall clock, calendar, RFC 3339 and HTTP dates |
| std: crypto | nothing | SHA-256, HMAC, OS random bytes |
| Documentation of std | `tests/spec.rs` makes the spec name every pub function | a way to grow std past the 30k-token spec budget (~24.6k used, P2.5) |
| Packages | modules are directories; imports resolve under the root | path and URL dependencies, `ore.lock`, cache |
| Targets | native via clang; generated C compiles for x86_64 and aarch64 Linux (M12) | `--target`, wasm32-wasi, wasm32-freestanding with JS imports/exports |
| `ore test` | test blocks, `--filter`, `--module`, `--json`, leak = failure | whole-tree runs, timings, time limits |
| Tooling | `fmt`, `fix`, `explain`, `doc --signatures`, `check --json`, `spec` | LSP, HTML docs |
| Progress number | goldens, error tests, spec checks | conformance suite with counts per area and target |
| Benchmarks | browser timings in `browser/engine/PERF.md` | Ore-level numbers (HTTP req/s, build times) |

## Phases

Ordered by what the framework lane needs first: an HTTP server (F1),
then files and processes (F2, the dev server), then wasm32 (F4, client
code).

| Phase | Goal | Exit check |
|---|---|---|
| P2 | Ore stage0 closed (O1–O5) | `cargo test`, `ore fmt --check`, spec token count, gate record in `milestones.md` |
| O1.x | Server basics: conformance harness, std docs plan, `std.net`, `std.http` server, fs/path, process | a hello-world `std.http` program answers `curl`; conformance table prints |
| O2.x | Targets incl. wasm32 | O8; wasm32-wasi conformance count printed |
| O3.x | std breadth: JSON, time, crypto, HTTP client | O6 |
| O4.x | Dependencies | O7 |
| O5.x | `ore test` maturity | O9 |
| O6.x | LSP and HTML docs | O10 |
| O7.x | Numbers and the gate | O11–O13 |

P2 runs alongside everything. O1 comes first; O2 and O3 can run side by
side once O1.2 is decided.

## Steps

∥ = can run at the same time. Steps that add std functions touch
`docs/spec.md` (or what O1.2 puts in its place) and `tests/spec.rs`;
they merge one after the other.

### P2: Ore close-out

| id | Goal | Verify | Deps | ∥ |
|---|---|---|---|---|
| P2.1 | Gap #48: `b as u8` and other integer casts, error test, spec entry, `ore explain` | `cargo test --test golden` | P0 | P1, P3 |
| P2.2 | Remove the #48 workaround in `browser/js/fast_expr.ore` | `cargo test --test browser js_conformance` | P2.1 | not with JS steps |
| P2.3 | Decline #3 and #21 in `decisions.md`; move them in `COMPILER_GAPS.md` | read the diff | – | all |
| P2.4 | `ore fmt --check` over the repo; fix the drift | `ore fmt --check` exits 0; `cargo test` | P0 | P3 after it merges |
| P2.5 | Spec token count and M5 gate. Done 2026-10-08: `ore spec` ~24.6k tokens (cl100k) of ~30k; a fresh agent wrote `wc` and a JSON tokenizer from the spec alone, each passing `ore check` on the first round; four loose spots in §6–§8 clarified (P2.5c) | – | – | – |
| P2.6 | Status table in `milestones.md` | read the diff | P2.3–5 | – |
| P2.7 | Diagnose a flaky build: once, a fresh build failed every `import()` with "import is not defined" and a rebuild of the same source fixed it (seen on `pi-dynimport`, 2026-10-07) | repeated clean builds + `browser/tests/js/23*` reproduce or rule it out; cause written down | P0 | all |
| P2.8 | Agents build into their own worktree: the pi-codex skill's task template and `pi_run.py` build with `browser/build.sh` (no shared output path) or name the output after the worktree (cause of P2.7) | `grep` the skill for shared output paths; one pi dry run | P2.7 | all |
| P2.9 | **Unblocked 2026-10-08:** Homebrew LLVM 23.1.2 is installed (not linked); `$(brew --prefix llvm)/bin/clang -fsanitize=address -g` reports `heap-use-after-free` on a C test. Use that clang for the sanitizer golden. Was blocked (2026-10-07): AddressSanitizer hangs at start-up on this machine (Apple clang 17, macOS 26.5, arm64; x86_64 under Rosetta dies with SIGILL), like ThreadSanitizer. Needs Homebrew LLVM or Linux. Keep the debug free queue (`src/cgen.rs:147-240`, catches double free and write-after-free) and poison its blocks only under ASan. Gap #3: under AddressSanitizer the runtime (`src/cgen.rs`) poisons the freed blocks it holds back in debug and `ore test`, so a read through a stale `List`/`Pool` pointer reports `heap-use-after-free`; golden test modelled on the ThreadSanitizer one, debug and safe mode | `cargo test --test golden` | P2.3 | P1, P3 |
| O0.1 | Toolchain note: wasm32 builds with Homebrew LLVM: `$(brew --prefix llvm)/bin/clang --target=wasm32-wasip1 --sysroot=$(brew --prefix wasi-libc)/share/wasi-sysroot -resource-dir=$(brew --prefix wasi-runtimes)/share/wasi-runtimes` (the `wasm32-wasi` triple is gone; without `-resource-dir` the link misses `libclang_rt.builtins.a`). No wasm runtime yet: the wasm target step installs `wasmtime` to run tests | – | checked 2026-10-08 on a printf program | – | – |
| P2.10 | Stale comment at `browser/dom/dom.ore:192` says Ore has no `unreachable` (gap #32 is fixed); use it | `ore check browser`; `cargo test --test browser` | – | all |

### O1: Server basics

| id | Goal | Files | Verify | Deps | ∥ |
|---|---|---|---|---|---|
| O1.1 | Conformance harness: `tests/conformance/<area>/*.ore` with `.out`, run per target; prints passed/total per area and target. Seed it with a few programs per existing std module | `tests/conformance.rs` (new), `tests/conformance/` (new) | `cargo test --test conformance` prints the table | – | all |
| O1.2 | Decide how std is documented as it grows: `tests/spec.rs` makes the spec name every pub std function within ~30k tokens (~24.6k used). For example the spec keeps the language and a one-line index per std module, `ore doc --signatures std.<m>` is the API, and the test checks doc coverage instead. Entry in `docs/decisions.md`, test changed to match | `docs/decisions.md`, `tests/spec.rs`, `docs/spec.md` | `cargo test --test spec` | – | all |
| O1.3 | `std.net`: TCP `listen`, `Listener.accept`, `connect`, `Conn.read`/`write`/`close`, read timeouts through `poll`; the macOS/Linux constants chosen as `browser/ui/socket.ore` does | `std/net/` (new), spec | golden: a thread serves, `main` connects and echoes 1 MB | – | O1.5–6 |
| O1.4 | `std.http` server: HTTP/1.1 request parsing (request line, headers, `Content-Length` and chunked bodies, keep-alive, size limits → 400/413), a response writer, `serve(alloc, addr, handler)` with a thread per connection | `std/http/` (new), spec | golden: the server on a thread, requests sent over `std.net` (two on one connection, one malformed); `curl` by hand | O1.2, O1.3 | O1.5–6 |
| O1.5 | `std.fs` breadth and `std.path`: `stat` (size, mtime, kind), `readDir`, `mkdir`, `remove`, `rename`; `join`, `dirname`, `basename`, `extension`, `normalize`, a check that a path stays under a root (static files) | `std/fs/fs.ore`, `std/path/` (new), spec | goldens in a temp directory | O1.2 | O1.3–4 |
| O1.6 | `std.process` and the environment: `spawn(argv)` with stdin/stdout/stderr piped or inherited, `wait` → exit status, `kill`; `std.os.getenv` | `std/process/` (new), `std/os/os.ore`, spec | golden: run `/bin/echo`, pipe through `cat`, read an exit status | O1.2 | O1.3–4 |
| O1.7 | First HTTP benchmark: hello world on `std.http` in `fast` mode against Go `net/http` and Node 22 `http`, `ab -k -c 50 -n 100000`, quiet machine; if Ore is below Node, the next step is a diagnose step | `docs/ore-bench.md` (new) | the table is recorded | O1.4 | all |

### O2: Targets, wasm32 included

| id | Goal | Files | Verify | Deps | ∥ |
|---|---|---|---|---|---|
| O2.1 | Diagnose wasm32: which toolchain (Homebrew `llvm` and `lld`, wasi-libc or wasi-sdk; Apple clang 17 has no WebAssembly backend) and what the C prelude needs on wasm32 (pthreads `src/cgen.rs:535`, `mmap` `src/cgen.rs:467`, `std.coro` stack switching, 32-bit `usize`, `@link`/`@cimport`). Write the target matrix (which std modules each target has) and how std code tells targets apart | `docs/decisions.md` | `ore emit-c tests/cases/hello.ore` compiled with `--target=wasm32-wasi`; every error listed with its cause | – | O1, O3 |
| O2.2 | `--target <arch-os>` on `build`, `run`, `test`, `emit-c`: aarch64-macos, x86_64-macos, x86_64-linux, aarch64-linux, wasm32-wasi, wasm32-freestanding; an unknown target is an error with a code and `ore explain` | `src/main.rs`, `src/driver.rs`, `src/codes.rs` | `ore build --target x86_64-macos` runs under Rosetta; Linux C compiles with `clang -c`; error test | O2.1 | O1, O3 |
| O2.3 | wasm32-wasi: a prelude without threads, coroutines or `mmap`; `std.thread`/`std.coro`/`std.sync` are a compile error naming the target; goldens and conformance run under Node 22 (`node:wasi`), skipped when the toolchain is missing | `src/cgen.rs`, `tests/golden.rs`, `tests/conformance.rs` | the wasm32-wasi row of the conformance table | O2.2, O1.1 | O1, O3 |
| O2.4 | wasm32-freestanding for web pages: export Ore functions to JS and import JS functions (syntax decided here, spec entry, error codes), a page allocator on `memory.grow` | `src/parser.rs`, `src/sema.rs`, `src/cgen.rs`, `std/heap/`, spec | Node 22 loads the `.wasm`, calls an export that calls an import; error tests | O2.3 | O1, O3 |

### O3: std breadth

| id | Goal | Files | Verify | Deps | ∥ |
|---|---|---|---|---|---|
| O3.1 | `std.json`: parse into a tree (arena), typed accessors with clear errors, a writer (escaping, into `text.Builder`) | `std/json/` (new), spec | valid and invalid cases from RFC 8259; conformance area `json` | O1.2 | O2 |
| O3.2 | Wall clock in `std.time`: `now` (Unix ms), UTC calendar fields, RFC 3339 and IMF-fixdate (HTTP `Date`) format and parse | `std/time/time.ore`, spec | goldens with fixed timestamps | O1.2 | O2 |
| O3.3 | `std.crypto` basics: SHA-256, SHA-1 (WebSocket handshake), HMAC-SHA-256, OS random bytes (`getentropy`), constant-time compare; base64 and hex | `std/crypto/` (new), `std/text/` or a new module, spec | FIPS 180-4, RFC 4231 and RFC 4648 test vectors | O1.2 | O2 |
| O3.4 | `std.http` client over `std.net` (plain HTTP: request, response, chunked, redirects off by default); decide how HTTPS comes (libcurl through `@link` as the browser does, or the system TLS library) and write it down | `std/http/`, `docs/decisions.md` | golden against the O1.4 server | O1.4 | O2 |
| O3.5 | HTTPS for the client as O3.4 decided | `std/http/` | fetch of a local TLS test server | O3.4 | O2 |
| O3.6 | `std.http` server, round 2 (framework needs): query and form decoding, cookies, streamed and chunked responses, server-sent events, graceful stop | `std/http/` | goldens per feature | O1.4 | O2 |
| O3.7 | SQLite without a std module: a `tests/ffi` program that uses `@cimport("sqlite3.h")` (the system library), skipped without it | `tests/ffi/` | `cargo test --test golden` | – | all |

### O4: Dependencies

| id | Goal | Files | Verify | Deps | ∥ |
|---|---|---|---|---|---|
| O4.1 | Design: the manifest and `ore.lock`. `design.md` wants `build.ore` run at compile time; stage0 has no compile-time execution beyond type functions, so decide a first form (a declarative `build.ore` subset that is read, not run, or a data file) for path and URL dependencies, the cache location, and what belongs in std rather than in packages | `docs/decisions.md` | the entry, reviewed by the user | – | all |
| O4.2 | Local path dependencies: a root imports a module from another root through the manifest (the framework app imports `framework/`; a bundler could import the browser's JS parser) | `src/module.rs`, `src/driver.rs` | `tests/modules` case with two roots | O4.1 | O2, O3 |
| O4.3 | URL dependencies: fetch an archive into the cache, check its hash against `ore.lock`, build offline from the cache | `src/`, new code for fetching | test with a local fixture URL; a hash mismatch fails with a code | O4.2 | O2, O3 |

### O5: `ore test` maturity

| id | Goal | Files | Verify | Deps | ∥ |
|---|---|---|---|---|---|
| O5.1 | `ore test <dir>` runs the tests of every module under it (so `browser/unit_tests.ore` is no longer needed), with a time per test and a summary line | `src/main.rs`, `src/driver.rs` | `tests/tests` goldens; `ore test browser` | – | O1–O3 |
| O5.2 | A time limit per test (CPU time, as `docs/practices/workflow.md` "A speed test limits CPU time" says) and modules run in parallel (`-j`) | `src/driver.rs` | golden: an endless test is reported as timed out | O5.1 | O1–O3 |

### O6: Tooling

| id | Goal | Files | Verify | Deps | ∥ |
|---|---|---|---|---|---|
| O6.1 | `ore lsp`: diagnostics (the `check --json` path) and formatting (`src/fmt.rs`) over stdin JSON-RPC | `src/lsp.rs` (new), `src/main.rs` | a Rust test drives a session | – | all |
| O6.2 | `ore lsp`: hover with the `///` comment (`src/doc.rs`) and go to definition | `src/lsp.rs` | the same test, more requests | O6.1 | all |
| O6.3 | `ore doc --html <path>`: static pages for a module and for std | `src/doc.rs` | golden HTML for a small module | O1.2 | all |
| O6.4 | `README.md:405-410` ("Not yet supported") is stale: it says `ore fmt` and `ore doc` are missing and std has no `Pool`; bring it in line with what exists | `README.md` | read the diff | – | all |

### O7: Numbers and the gate

| id | Goal | Files | Verify | Deps | ∥ |
|---|---|---|---|---|---|
| O7.1 | `docs/ore-bench.md` complete: HTTP req/s (O1.7 rerun), debug and fast build times of `browser/`, binary sizes; rerun at the end of each phase | `docs/ore-bench.md` | the table | O1.7 | all |
| O7.2 | Server gate (O13): a fresh agent, given only `ore spec` and `ore doc --signatures std`, writes a JSON REST service; record the rounds and errors in `docs/milestones.md` | `docs/milestones.md` | `ore check` within two rounds; scripted `curl` check passes | O3.1, O3.6 | all |
