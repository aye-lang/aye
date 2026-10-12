# Plan: the Ore (Aye) lane

The language lane of [`docs/plan.md`](../plan.md): the compiler
(`src/`), std (`std/`), `tests/` and the toolchain. Primary lane: it
gets most slots. Its inputs are the criteria below, the open entries of
`browser/COMPILER_GAPS.md` (in aye-lang/anyknown), and F1.1 (the acceptance test of
`std.http`). Finished steps: [`done.md`](done.md).

## Done criteria

Criteria are named O1–O13 (not to be confused with phases O1.x–O7.x).
The browser resumes when O6–O9 hold, in its own repo aye-lang/anyknown
(split in P8; `scripts/downstream.sh` runs it against this compiler).

**Stage0 close-out**

- O1: `cargo test --release` exits 0 on a clean checkout of `main`, on
  macOS and, after X1.2, on Ubuntu 24.04.
- O2: `target/release/aye fmt --check` passes over every `.aye` file,
  checked by `cargo test --release --test fmt` (P2.4).
- O3: `browser/COMPILER_GAPS.md` (in aye-lang/anyknown) "## Open" has no entry: each is fixed
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
  `--json` and leak-is-failure (`aye test browser` in the downstream check; tests of O5.1–O5.2).
- O10: `aye lsp` (diagnostics, format, hover with `///`, go to
  definition) and `aye doc --html`, each with a test (O6.1–O6.3).
- O11: `cargo test --release --test conformance` prints passed/total per
  area and target: 100% on native and wasm32-wasi (impossible entries
  listed as such).
- O12: `docs/aye-bench.md` records a hello-world `std.http` server in
  `fast` mode at least as fast as Node 22's `http` (Go `net/http` as the
  second reference, `ab -k`, same machine), and the debug and fast
  build times of `browser/` (aye-lang/anyknown).
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
- Tests: `tests/golden.rs` (cases, errors, tests, fix, modules, ffi, cimport; `X.needs`/`X.tools` skip files), `tests/fmt.rs`, `tests/spec.rs`, `tests/framework.rs`; the browser's tests live in aye-lang/anyknown and run through `scripts/downstream.sh`. `tests/conformance.rs` (18 areas, 51 programs, native).
- Open gaps: #50 (per-OS declarations; O8 needs it), #52 (methods as fn values), #53 (nested structs, generic methods).
- Toolchain: Homebrew LLVM 23 for ASan and wasm32 (flags in `done.md`, O0.1); Node 22, Go and `ab` are installed locally.

## Phases

| Phase | Goal | Exit check | Status |
|---|---|---|---|
| P2 | Stage0 close-out (O1–O5) | the O1–O5 checks | open: O3 (gaps #50, #52, #53) |
| O1.x | Server basics: conformance harness, `std.net`, `std.http` server, fs/path, process | `cargo test --release --test conformance` prints the table; a hello-world `std.http` golden answers | done 2026-10-10 |
| O3.x | std breadth: JSON, time, crypto, HTTP client (O6) | the O6 check | done 2026-10-10 (O6 holds) |
| O2.x | Targets incl. wasm32 (O8) | the O8 check | current |
| O5.x | `aye test` maturity (O9) | `aye test browser` (downstream check) | backlog |
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
- ~~O3.4b `std.net.connect` timeout and the IPv6 http client golden~~ done 2026-10-12 as O3.4c (`net.connectTimeout`, `net_connect_timeout`, `std_http_client_ipv6`).
- ~~O3.3b `std.encoding` strict base64 and `hexEncode` in the crypto conformance programs~~ done 2026-10-12 (`QQ==` ok, `QR==` `InvalidCharacter`).
- ~~O3.1c `std.text.Builder.appendFloatExact`~~ done 2026-10-12 (`done.md`).
- O3.2a `std.time.parseHttp` lenient forms (RFC 850, asctime) if the HTTP client needs them; a `Date` header helper in `std.http` from `now()` (O3.2 report).
- O3.1b `std.json.Value.asInt() ?i64` for exact integers above 2^53, if O6/O13 need it (O3.1 report).
- O1.6a `std.process`: suppress SIGPIPE on a write to an exited child's stdin (a `WriteError`, as `std.net` does per socket); `Child.killHard` (SIGKILL), custom environment and working directory when a caller needs them (O1.6 report). Verify: a golden that writes to an exited child. (done 2026-10-12, e7f0004)
- P8.x (browser, when it resumes; aye-lang/anyknown) `browser/net/archive.aye` uses `std.fs.readDir`/`stat` instead of its `@cimport` of dirent.h and sys/stat.h (O1.5 report).
- P-flaky aye-lang/anyknown `tests/browser.rs` `window_relayout_after_image_load` failed once in three Linux full runs (observer callback logged before image `onload`), diagnose step (PR #1).
- O2.8a (done 2026-10-12) one-step Linux executable from macOS: `AYE_SYSROOT=<dir>` (env, like `AYE_WASI_SYSROOT`) gives clang `--sysroot` and `-fuse-ld=lld` when linking a Linux target on macOS; `scripts/linux-sysroot.sh arm64|amd64 <dir>` copies a sysroot out of ubuntu:24.04. Real link ran: aarch64-linux and x86_64-linux hello linked on macOS and printed hello in `container run ubuntu:24.04`; goldens `AYE_SYSROOT link` (skipped without the env var), README, help, spec §2.
- O2.6 (done 2026-10-12) stack overflow on wasm: debug/safe builds print the native `panic: stack overflow` text and exit 134, via an `AYE_STACK_CHECK()` prologue with `--stack-first` (large frames) and the runners mapping V8's `RangeError: Maximum call stack size exceeded` (small frames); `stack_overflow` runs on all three lanes; fast mode still traps (decisions.md).
- Note (O2.3): Node 22.22.2 segfaults once a wasm heap passes ~32 MiB (plain C reproduces it); the runner uses `--single-threaded-gc`; Node 25 is fine.
- O1.17b closed (decisions.md "Round 2"): macOS syspolicyd assesses 2-3 new executables at a time, golden links 396, ~150 s of serial stall; `-Wl,-no_adhoc_codesign` kills the binary, `-mmacosx-version-min` no effect, mode overlap inside the noise. Decision: macOS golden stays; heavy verification runs on Linux (cloud lane, golden 84 s there); the user may try the Developer Tools exemption for the terminal app.
- O1.19a (done 2026-10-12) a value `match` whose arms all `return`/`break`/`continue` gets type `void`, so `let v = match ...` fails downstream with an unhelpful message (O1.19 report). Done: new E0124 at the `match`; golden `errors/match_value_all_leave`.
- F-gen/O1.26 (done 2026-10-11, 39f6bb8) `web gen`: `page.aye` and `route.aye` in one directory share one generated module, so a page local named `post` clashes with the route's `pub fn post` (E0016, no shadowing). Either gen gives each file its own module, or shadowing of a module-level fn by a local is allowed. Verify: the posts page can name a local `post`. (framework lane leftover, F5.5)
- O1.25 (done 2026-10-11, 5226bcf) `@embedFile` codegen: emit bytes so clang prints no `-Wtrigraphs` warnings on binary content (use `\ooo` escapes for `?` or split after `?`, or emit an array initializer). Verify: `aye build` of a 2 MiB random embed prints nothing (F5.1's `gen_prod` fixture).
- O1.24 (done 2026-10-11, 4839422) `import a.b` loads only the module in directory `a/b` (spec §12), not every parent directory as a module; a parent is loaded only when its own items are referenced (`a.x`). Today `resolve_path` in `src/module.rs` loads each parent, so a wasm-only `islands/counter/client.aye` beside a shared `view/` breaks the native build (F4.1 spike). Verify: a golden with `a/b/b.aye` imported as `import a.b` while `a/bad.aye` has an unresolvable extern; F4.1's `island_build` stays green; F4.3 unblocked. Decision 2026-10-11: option (a) over moving the wasm root.
- O1.22a (done 2026-10-12) `orelse` should take the expected type the way `catch` does after O1.22 (`let s: []u8 = opt orelse "none"`); and a `noreturn` function kind so `die(...)` ends a block (F3.2 wrote `return false` after it). Verify: a golden each.
- O1.22 (done 2026-10-11) `catch` blocks: `unreachable` (and `return`/`panic`) should satisfy a non-void `catch` block's value requirement, and a `catch` on a discarded error-union result should accept `{}` (F3.2 had to write `return false` and a dummy struct value); a `[]u8` literal in a `catch` block should coerce to the `[]mut u8` the expression returns when the result is only read (F3.4, E0023). Verify: two goldens, one per shape.
- O3.6d (done 2026-10-11, 29b1770) `std.http` streams: let a handler write an initial frame when it returns a streaming response (`Response.stream()` ignores `body` today, so F3.5 sends the first `build` frame from a heartbeat thread); e.g. send `res.body` as the first chunk after the head. Verify: `http_sse` golden gains an initial frame; the F3.5 heartbeat can then stop. Also: a `catch |e| {}` on a value-returning call is E0067 (see O1.22).
- O1.21 global initializers may call `init`-style functions that sema can evaluate at compile time (`var lock = Mutex.init()` is E0089 today; F2.3 used a lazy lock behind an `Atomic(u32)` gate; `docs/framework.md` §4 and §6 assume a global `Mutex`): allow a module-level `var` initialized by a call to a function whose body is a struct literal of constants (or add `Mutex.INIT`-style constants in std.sync). Verify: a golden with a global `Mutex` used from two threads; the guestbook example loses its gate. (done 2026-10-12, 196856c)
- O1.23 (done 2026-10-11) `aye build --no-link <lib>` (repeatable): an optional library (`@linkOptional`) is treated as not installed, so `@hasLink` is false and it is not linked; `std.http` links libcurl whenever it is installed, which a production server (framework F5.3) does not want. A name that no `@linkOptional` declares is E0100. Verify: a golden builds the hello server with `--no-link libcurl` where libcurl is installed, `otool -L`/`ldd` of the result lists no libcurl, it answers a request, and `std.http.get("https://...")` returns the missing-TLS error.
- O1.20 (done 2026-10-11, 79f728c) `aye check <missing file>` says to write the file first (eval: http_hello probes `aye check` on a missing file every run; low value). Verify: a CLI golden.
- O1.17c `virtual_clock.html` is 22% of `jsdom_gc_stress` CPU; make the page cheaper under GC stress without losing what it tests.
- O3.6c (done 2026-10-12, a50bc56) graceful stop, the rest of O3.6 (O3.6a query/form/cookies, O3.6b server-sent events and O3.6d streamed responses are done): `std.os.onStop()`/`stopRequested()`, `Server.run` stops on SIGINT/SIGTERM and writes answered responses whole before closing. Verify: golden `http_stop`.
- O3.6b `std.http` server-sent events (what is left of O3.6 after O3.6a; framework F3.5): a handler can keep its response open as a stream (`res.events()` writes the headers `Content-Type: text/event-stream` and `Cache-Control: no-cache` without `Content-Length`; the server holds the connection in its `poll` set instead of a blocked handler), the server can send `data:` frames to the open streams (`Server.broadcast` or a handle kept by the handler), and `stop` closes them within `idleMs`. Verify: a golden (`http_sse`) where a client reads the first frame, then a frame sent later from another thread, a second request on another connection is answered meanwhile, and `stop` returns with the stream closed.
- O3.7 SQLite through `@cimport("sqlite3.h")` as a `tests/ffi` program (Bun table).
- O5.1b the first test of each root takes ~500 ms on macOS (first-exec cost, O5.1 report); measure and, if it is the exe's first launch, start the timer after a warm-up `--list` run. Verify: `treetests` first-test time drops below 50 ms.
- P-tmp about 7000 stale `aye-run-*.dSYM`/`aye-test-*.dSYM` folders sit in `$TMPDIR` from before O5.2g; delete them once no agent runs an old binary (`rm -rf $TMPDIR/aye-run-*.dSYM $TMPDIR/aye-test-*.dSYM`).
- O5.2f drop `-g` for `aye test` executables unless `--cflags` is given (−45% on top of O5.2e; no code reads DWARF). Decide after O5.2e.
- O5.2b per-test `ms` is inflated under `-j` by the first-exec cost of a fresh binary (~0.5 s on macOS, same cause as O1.17b and O5.1b); warm the binary once before fanning out. Verify: `treetests` times under 50 ms after the first.
- O6.2a lsp navigation leftovers (O6.2 report): done: member accesses through sema types (`x.method()`, fields, chains), path deps of `aye.toml`, past-the-end cursor, block scopes (sibling blocks and loops), one declaration renderer (`doc::render_decl`) for `aye doc --signatures` and hover. Left: `@cimport` modules (hover/definition of C declarations; sema records no member access on them) and URL deps outside sema. Verify: tests/lsp.rs cases.
- O6.3a (done 2026-10-11, 025ae16) `aye doc --html` leftovers (O6.3 report): struct member `///` comments stay inside the signature block instead of becoming paragraphs; root page name is the path stem (`index` → `index_module.html`); the website (X3, `site/`) should use it for the std reference. Verify: a `shapes` fixture change plus the site build.
- O6.1a `aye lsp` follow-ups (O6.1 report): done: `Diag::end` and `severity` (end set for unknown names, `;`, unexpected tokens, number literals, and expression type mismatches via `ast::Expr::end`; `token_len` is the fallback), `relatedInformation` for imports, `textDocument/codeAction` quickfixes. 2026-10-12: end spans for unknown struct, unknown struct-literal field, unknown enum variant and assignment to an immutable name (E0017, E0038, E0041, E0032); `didChange` is debounced 150 ms (reader thread + `recv_timeout`; flushed at `shutdown`). Left: the other codes still use the `token_len` fallback (declaration-level errors such as E0050, E0052, E0076, E0096, E0104, E0015, E0067, E0102, and module/manifest errors): they point at a declaration or statement, where no single name or expression span is the offender, or the AST keeps only the start. Verify: tests/lsp.rs cases.
- O3.9b `std.http` worker follow-ups. Done 2026-10-12 (docs/plans/done.md): a slow handler no longer holds up its worker (another worker takes its other connections after 20 ms), `ReadError.WouldBlock`, `Listener.fd()`/`Conn.fd()`, thundering herd measured (no `-k`: 4 workers 9.4k req/s, 1 worker 16.1k; docs/aye-bench.md). Left: O7.4 `Arena.reset` then one arena per connection; O7.5 rerun the 4-core cloud VM table; O3.9c (proposed) one listener per worker with `SO_REUSEPORT` on Linux, or an accept hand-off, against the herd. Verify: `scripts/bench/run.sh` req/s before/after.
- ~~O1.11b~~ done 2026-10-12 `scripts/eval.py --tasks a,b` rewrites the whole results table in `docs/aye-eval.md` with only the tasks it ran (O7.2 report); merge per task instead. Verify: a `--tasks rest_json` run keeps the other rows.
- ~~O1.11c~~ done 2026-10-12 the eval harness `SYSTEM_PROMPT` lists `aye spec`, `aye check`, `aye explain` but not `aye doc --signatures <module>` (O7.2 report; O13 names it). Add it. Verify: a run's transcript shows the agent found `aye doc` from the system prompt.

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
URL parser (`browser/url/url.aye`), encodings (`browser/encoding/`); that
code is now in aye-lang/anyknown.
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
