# Done steps

One line per finished step: id, date, result, merge commit (when one
exists). "dropped: reason" marks a step that was superseded or is no
longer needed. Newest phases last; within a phase, by id. The lane plans
([`aye.md`](aye.md), [`anyknown.md`](anyknown.md),
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
| O3.1 | 2026-10-10 | `std.json`: RFC 8259 parser into an `Arena` (`Value` enum, accessors `get`/`at`/`as*`/`isNull`, `ParseError` tags, `parseDiag` with a `std.diag` diagnostic, `maxDepth` 512; numbers are `f64`); conformance area `json` 3/3 (33/33 total), golden `tests/tests/std_json.aye` | c967ebe |
| O3.2 | 2026-10-10 | `std.time`: `now()` (Unix ms), `DateTime` (UTC, weekday 0 = Sunday), `fromUnixMs`/`toUnixMs`, RFC 3339 format/parse (offsets normalized, fractional seconds), IMF-fixdate format/parse; conformance area `time` 5/5 (36/36 total), golden `tests/tests/std_time.aye` | 516b6bd |
| O3.3 | 2026-10-10 | `std.crypto`: streaming `Sha256`/`Sha1`/`HmacSha256` and one-shot `sha256`/`sha1`/`hmacSha256`, pure Aye, fixed-size array results; conformance area `crypto` 3/3 (FIPS 180-4, RFC 4231; 39/39 total), golden `tests/tests/std_crypto.aye`; 1M "a" in 0.01 s fast | 461f44a |
| O3.4a | 2026-10-10 | HTTPS for the client decided in `docs/decisions.md`: libcurl in connect-only mode under `@linkOptional("libcurl")` with hand-written externs, O3.4's HTTP/1.1 on top; system TLS and pure-Aye TLS rejected; O3.5 scope written | c104e09 |
| O3.1a | 2026-10-10 | `std.json` writer: `write`, `writePretty`, `stringify`, `equals`, `writeNumber` (shortest round-trip digits, `JSON.stringify` layout, NaN/inf as null); conformance `json/roundtrip` (40/40 total), 6 more test blocks | 4a4ad33 |
| O3.3a | 2026-10-10 | `std.crypto.randomBytes` (`getentropy`) and `constantTimeEqual`; new `std.encoding`: base64, base64url, hex encode/decode (RFC 4648 vectors); conformance areas `crypto` 4/4 and `encoding` 4/4 (45/45 total), golden `tests/tests/std_encoding.aye` | 8ae042a |
| O6.4 | 2026-10-10 | `README.md` "Not yet supported" paragraph: `aye fmt`, `aye doc`, `Pool`, lists and string building are no longer listed as missing; names the 24 std modules and `aye doc --signatures` | (main) |
| O3.4 | 2026-10-10 | `std.http` client: `request`/`requestWith`/`get`/`post`, `parseUrl`, `ClientOptions` (timeout per wait, body limit, redirects off by default), `ClientError`, `Response.header`; Content-Length, chunked and read-until-close bodies; `https://` is `HttpsUnsupported` until O3.5; golden `std_http_client` (O1.4a server + hand-written server), 7 unit tests | d649ec8 |
| O1.12a | 2026-10-10 | E0006 hint goldens for `or` and `and` outside `if`; the `not` hint fires only when an operand follows `not` (`keyword_not_var` has none) | 8e5e383 |
| O1.11 rerun | 2026-10-10 | `scripts/eval.py --runs 3` on c96dc27: 15/15 pass, median 12 turns and 1 check; `std.json` used 3/3, `std.http` 1/3; the 38.6 KB spec core exceeded Claude Code's inline output limit (→ O1.13a, O1.11a); findings in `docs/aye-eval.md` | (docs) |
| O3.5 | 2026-10-10 | `https://` in the `std.http` client over libcurl connect-only (`@linkOptional("libcurl")`, private `Stream`, checks always on, `caFile`, `TlsFailed`, `HttpsUnsupported` without libcurl); golden `std_http_client_tls` against `openssl s_server` (macOS and Linux); `tests/golden.rs` learns `X.needs`/`X.tools` skip files | 53672fc |
| O1.13a | 2026-10-10 | `aye spec` core 26847 bytes (cap 28000), order §1, 15, 18, 4 intro, enums, 5, conversions, 6, 7, arrays/slices, optionals, 8; functions, constants, structs, pointers, match, §9, §12 leave the core with a trailer hint each; eval http_hello: pass, 8 turns, 0 refused calls | 93c73ed |
| O1.5b | 2026-10-10 | `std.fs.readDir` falls back to `lstat` for entries without a type; `statFollow(path)` follows symlinks (`stat` keeps lstat semantics; C helper takes a follow flag); golden `std_fs_dir` covers file, directory and dangling links; macOS and Linux golden 280/280 | b0399cb |
| O3.8 | 2026-10-10 | O6 close-out: conformance areas `path` and `process` (3/3 each, 51/51 total), test goldens `tests/tests/std_net.aye` and `std_process.aye` (golden 282/282); every O6 module has `test` blocks, a conformance area and `///` docs | bf4c5d8 |
| O6 check | 2026-10-10 | On bf4c5d8: `cargo test --release` exit 0 (browser 62/62, conformance 51/51, golden 282/282, spec 10/10), `aye spec` 26886 bytes, `aye doc --signatures std.<m>` prints for net, http, json, path, process, time, crypto, encoding: **O6 holds** | – |
| O1.11a | 2026-10-10 | `scripts/eval.py` runs agents in `/tmp/aye-eval/<stamp>/<task>-<n>` (0700, `--keep`), table gains a std-modules column; word_freq run 0 refused calls | a4834d2 |
| O2.1 | 2026-10-10 | Diagnose: wasm32 targets in `docs/decisions.md` "Targets (O2)": emitted C compiles for wasm32-wasip1 with Homebrew LLVM + wasi-libc, failures are link errors and wasi constants; hello runs under Node 22 `node:wasi` after stubbing `sigaction`/`sigaltstack`/`getrlimit`; `-pthread` must go; matrix per prelude feature; cards O2.2–O2.4 | a260ccb |
| P2.9 | 2026-10-10 | Under ASan the debug free queue poisons quarantined blocks: stale `List`/`Pool` reads report `use-after-poison` (debug) and `heap-use-after-free` (safe); `tests/asan/` goldens, `asan_clang` detection (PATH, then Homebrew LLVM); macOS and Linux golden 284/284; gap #3 fixed; TSan also runs with Homebrew clang | 194aff5 |
| O2.2 | 2026-10-10 | `--target` on build/run/test/emit-c/check with a target table (4 native triples, wasm32-wasip1 `.wasm` via Homebrew clang + wasi-libc, wasm32-freestanding waits for O2.4); E0117 unknown target, E0118 std module not on this target; `aye run --target wasm32-wasip1` through `node:wasi`; signal calls gated by `#ifndef __wasm32__`; goldens x86_64-macos build, wasm hello link, `errors/target_*` via `X.args`; macOS and Linux golden 288/288 | 5723001 |
| P2.9a | 2026-10-10 | `sanitizer_clang` (PATH clang, then Homebrew LLVM, probed with a 30 s timeout) serves ASan and TSan; the ThreadSanitizer golden runs on this Mac and on Linux: M10 complete | 1b70497 |
| O1.15 | 2026-10-10 | E0081 on an unknown `std.<x>` lists the std modules (from `STD_FILES`) and suggests a close name; goldens `errors/std_unknown`, `std_unknown_near`; golden 290/290 | 7e9b85c |
| O2.3 | 2026-10-10 | wasm32-wasip1: C prelude branch (wasi open flags, stat/dirent offsets, errno mapping, clock ids, `aye_std_realtime`, `aye_std_ecvt`, 8 MiB stack), `scripts/wasi-run.mjs` embedded for `aye run`/`aye test --target wasm32-wasip1`, goldens also run on wasm (98 passed, 30 skipped, `X.native` markers), conformance gets a wasm32-wasip1 column (100% on every possible area; http, net, process pipes/status, sync threads impossible); Linux skips cleanly | 6692ad6 |
| O2.4 | 2026-10-10 | wasm32-freestanding: prelude runtime (bump allocator on `memory.grow` with size-class free lists, `memcpy`/`memset`/`strlen`/`puts`/small `printf`, `__multi3`), imports `env.aye_write` and `env.aye_abort`, `pub extern fn` exported by name, `main` exported as `aye_main`, loader `tests/wasm/loader.mjs` embedded for `aye run`; golden mode `X.freestanding` (3 passed), build test checks no WASI import; 68 of the cases sweep pass, libm/strtod/clock/snprintf/pthread are runtime gaps (O2.7) | ec88c60 |
| O5.1 | 2026-10-10 | `aye test <dir>` runs every module under a directory (each subdirectory with `.aye` files is a root, modules reached earlier run once), `test "name" ... ok (N ms)` per test, summary `N passed; M failed; K modules; T ms`, `"ms"` in `--json`; `aye test browser` is one run (131 tests, 15 modules) in tests/browser.rs; golden `tests/modules/treetests` with `ms` scrubbed; README and spec §13 | 6bc605d |
| O1.17 | 2026-10-10 | suite speed: `tests/common/mod.rs` with `jobs()` (`AYE_TEST_JOBS`, default `available_parallelism()`) and `run_queue()`; golden, conformance (was one thread per program, uncapped) and browser (`js_conformance`, `jsdom_fixtures`, `jsdom_gc_stress`, slowest pages first) use it; browser 287 s to 171 s, golden 355 s to 333 s and latency-bound (first exec of each new executable ~0.4 s at 0 CPU; jobs=24 gives 180 s) | 926b963 |
| O2.5 | 2026-10-10 | integer widths follow the target: `sema::Layout` (pointer bits, `long` bits) from the Target; `bits`/`min`/`max`, range checks, folding, exhaustiveness, casts and shifts use it; `c_long`/`c_ulong` are 32-bit on wasm32; `u64`→`usize` casts are now checked on wasm (this exposed and fixed `std.map` masking after the cast); per-target golden output `X.<target>.out`; `c_union`, `extern_struct` drop `X.native`, new `target_widths`; spec §5 and C interop say widths follow the target | 78423f7 |
| O5.2 | 2026-10-10 | `aye test --time-limit <s>` (default 10, CPU time via `RLIMIT_CPU` and `wait4` rusage, wall guard 10x, `FAILED (time limit: ...)`, `"cpu_ms"` and `"timed_out"` in `--json`) and `-j <n>` (default cores, results in test order); fixture `tests/tests/time_limit` (native), `test_jobs_case` compares `-j 1` and `-j 4`; README and spec §13 | 91fb479 |
| O1.17a | 2026-10-10 | `tests/common/mod.rs`: `jobs_for(per_core)`, `run_queue` takes a slot from a process-wide budget so concurrent callers never exceed `jobs()`; golden defaults to 2 jobs per core (inconclusive: golden stays ~380 s at 30-50% CPU, the bottleneck is the first exec of each new executable, O1.17b) | fefa325 |
| O2.7 | 2026-10-10 | wasm32-freestanding runtime: libm via `__builtin_*` (sqrt, floor, ceil, trunc, fabs, copysign, round) and `env.Math` imports (sin...fmod, pow), `env.strtod`, `env.aye_clock`, `env.aye_fmt_double` (ecvt), busy-wait `nanosleep`, single-thread pthread mutex/cond stubs; loader and decisions updated; `X.freestanding` on 8 more cases plus `json_roundtrip`, `sync_single`; freestanding golden 3 → 11, sweep 73 → 78 of 116; Linux golden shows a pre-existing O5.2 failure (O5.2c) | 4652d03 |
| O5.2c | 2026-10-11 | `aye test --time-limit` on Linux: a `SIGXCPU` death counts as the time limit even when the accounted CPU is a little under it (Linux delivers it early); `tests/time_limit.aye` still failed there (Linux sends SIGKILL when soft == hard); PR #1 sets hard = soft + 1 so SIGXCPU fires | 48a4043, a5292d9 |
| O1.16 | 2026-10-11 | `aye fix` removes `;` statement terminators in one pass (E0002 fix in `src/lexer.rs`, lexing continues after a `;`; `a; b` becomes a newline plus the indentation; strings untouched); golden `tests/fix/semicolons`; lexer unit test | 0a784c1 |
| O1.15a | 2026-10-11 | `aye doc std.<x>` for an unknown module prints the std module list and the closest name, shared with E0081 via `module::std_module_hint`; golden `cli/aye doc: an unknown std module` | 01660b6 |
| O5.1a | 2026-10-11 | `aye test <dir>` with more than one root: a root that fails to compile is one `FAILED (compile)` entry (`Failure::Compile`, JSON kind `compile`), the run continues, exit 1; a single-root directory still reports the compile error directly (keeps the `give modtests instead` hint); golden `tests/modules/treebroken`; README and spec §13 | 2975beb |
| O2.5b | 2026-10-11 | Linux gets the WASI toolchain: `scripts/setup-ubuntu.sh` installs wasi-sdk 34 into `/opt/wasi-sdk` (apt's wasi-libc is a 2023 snapshot and lld 18 cannot link wasi-sdk's libc), `AYE_WASI_CLANG` env var in `src/driver.rs`, Containerfile and `/etc/profile.d/aye-wasi.sh` set the `AYE_WASI_*` vars; Linux full suite 290 s all green incl. wasm32-wasip1 104 and freestanding 11 (PR #1) | a5292d9 |
| O2.7a | 2026-10-11 | freestanding: golden honours `X.stderr`/exit 1; prelude gains `abs`, `labs`, `div`, `strchr`, `qsort` (heap sort), `malloc`/`free`/`calloc`/`realloc` as libc symbols, `snprintf` with `%.Nf`; 10 more `X.freestanding` markers (freestanding golden 11 → 21, sweep 85 → 90 of 118); `cc` writes the C to a temp file instead of a stdin pipe (a >64 KiB source plus an early clang failure killed `aye` with SIGPIPE) | 59fd176 |
| O1.16a | 2026-10-11 | `aye fix`: a `;` inside `(` or `[` (C-style `for (a; b; c)`, `[1; 2]`) gets E0002 but no fix; lexer counts `(`/`[` depth; golden `tests/fix/semicolons_brackets` (listed in `FMT_EXCLUDED`) | 1989d11 |
| O2.5c | 2026-10-11 | first full suite on x86_64 Linux (cloud routine, Ubuntu 24.04.5 VM): all green except `net_names` (VM has `ipv6.disable=1`, O2.5d); `aye test --time-limit`: a SIGKILL within 100 ms of the CPU limit counts as the time limit (x86_64 kernels account 999 of 1000 ms); `docs/linux-x86_64-run.md` has the per-binary table (PR #2) | bf2d43a |
| O2.8 | 2026-10-11 | `aye build -c` compiles an object for any `--target` without linking (`cc_obj`: no link flags, `@link` ignored), default `<stem>.o`; the generated C needs only clang's own headers, so the Linux triples compile on macOS with no sysroot; link failures for a non-host target mention `-c`; goldens check the ELF header (x86_64 0x3e, aarch64 0xb7) and the link error text; README and spec §2 | 6b951b7 |
| O4.1 | 2026-10-11 | dependency design (`decisions.md` "Dependencies (O4)", spec §12 "Dependencies"): `aye.toml` (strict TOML subset: `name`, `[deps]` with `path`/`url`), `import key[.sub]` into the dep's top directory, deps see only std and themselves, `aye.lock` (`key = { url, sha256 }`, hex, sorted, committed), cache `$AYE_CACHE`/`$XDG_CACHE_HOME/aye`/`~/.cache/aye` by `sha256/<hex>/`, first URL form `https://`/`file://` `.tar.gz` via curl+tar, transitive deps deferred (E0123), codes E0119–E0123 reserved | eace90c |
| O2.5d | 2026-10-11 | `net_names` is IPv4-only (`localhost` + unknown name); the `::1` round trip is `net_ipv6` with `X.needs` `ipv6`, which the golden harness probes by binding `[::1]:0` and skips with `no IPv6 loopback`; conformance `net` uses no `::1` | 6969401 |
| O4.2 | 2026-10-11 | local path dependencies: `src/manifest.rs` (TOML-subset `aye.toml` reader, nearest manifest up to DIR), dependency keys in `src/module.rs` (`import util.x` into the dep, deps see only std and themselves, `util::x` canonical names, dep tests and doc examples skipped), `aye build` names the executable after `name`, no path means `.`, `aye fix` skips dep files, codes E0119–E0123 in the table (E0122 example checked by O4.3), `aye spec deps`, design.md pointer; goldens `tests/deps/local` and 8 `tests/deps/errors` cases | c27b6f1 |
| O2.7b | 2026-10-11 | freestanding: `inet_ntoa` in the prelude, `malloc`/`free`/`calloc`/`realloc` exported to JS (`AYE_EXPORT`), the lane reads `X.wasm32-freestanding.out` for no-argument cases, `extern_struct` passes, 70 more `X.freestanding` markers (freestanding golden 21 → 91; sweep 91 of 118, the rest E0118 or `target_widths`) | 9ed19d4 |
| O1.18 | 2026-10-11 | spec §15 json line names `parse, get, asString, asNumber, at; write, stringify`; new `### std.json` section with a 4-line example and the signature block (2 of 3 eval runs hand-parsed JSON); the `std.log` half is O1.18a | df44a62 |
| O4.3 | 2026-10-11 | URL dependencies: `src/fetch.rs` (curl `--proto =https,file`, SHA-256 in `src/sha256.rs` with FIPS 180 vectors, `tar -xzf` into `<cache>/tmp/`, rename into `<cache>/sha256/<hex>/`; cache root `$AYE_CACHE`/`$XDG_CACHE_HOME/aye`/`~/.cache/aye`), `aye.lock` read/write in `src/manifest.rs` (sorted, temp file + rename, `aye.lock: <key> sha256 <hex>`), E0121/E0122 in `src/module.rs`, E0122 example verified with a real tarball; `tests/deps.rs` covers first build, offline rebuild, hash mismatch, missing file, `http://`; README paragraph | 3a989a2 |
| O1.19 | 2026-10-11 | `return`, `break` and `continue` are allowed as a match-arm expression (parser routes them through the `catch`/`orelse` fallback block; sema types such an arm as the other arms', like `unreachable`); spec §7 bullets; golden `match_arm_jump`; E0059 explanation names the exception | aed3da9 |
| O1.18a | 2026-10-11 | `std.log.line(format, args...)`: stderr, newline, no prefix (`LogPrefix` enum Err/Info/None through sema, oir, cgen); §15 clause says `err`/`info` prefix `error: `/`info: `; `### std.log` example; golden `tests/modules/log_line` with `.stderr` | 1dcccac |
| O2.8b | 2026-10-11 | `aye build -c` for a non-host, non-wasm target passes `-ffreestanding`, so clang's own `stdint.h`/`stddef.h`/`stdbool.h` are used and no glibc headers of the foreign arch are needed (x86_64 host → aarch64-linux and the reverse); Linux container golden 545/545; decisions bullet | c3292f7 |
| O6.1 | 2026-10-11 | `aye lsp`: JSON-RPC over stdio (`src/lsp.rs`: minimal JSON reader, `Content-Length` framing), full-sync `didOpen`/`didChange`/`didSave`/`didClose` → `publishDiagnostics` for that file (same root resolution as `aye check`, editor buffer replaces the file text, compiler panics caught, 1-based char columns → 0-based UTF-16, `help:` appended), `textDocument/formatting` as one whole-document edit; `tests/lsp.rs` (3 tests); README "Editor support"; spec §2 | 9d63f87 |
| O6.3 | 2026-10-11 | `aye doc --html <path> [-o <dir>]`: `index.html` plus one page per module (signature in `<pre><code>`, `///` as paragraphs/`<pre>`, escaped, embedded stylesheet with a dark variant, no scripts); shared module loading with `--signatures`; golden compares the whole output tree of `tests/doc/shapes` against `tests/doc/expected/`; README and spec §2 | 6b87419 |
| O6.2 | 2026-10-11 | `aye lsp` hover (signature as `aye` code plus `///`) and go to definition (locals, top-level declarations, imports, `a.b.c` paths, std via the repo's `std/` tree) resolved on tokens (`src/lsp/nav.rs`), so they work while the file has errors; `tests/lsp.rs` 4 tests; README sentence | 088eaee |
| O7.2 | 2026-10-11 | O13 server gate: eval task `tests/eval/rest_json` (JSON REST service on std.http + std.json, 12-case `curl` check); 3 runs, all pass with one `aye check` and no error round (stamp 20261011-181047); recorded in `docs/milestones.md` and `docs/aye-eval.md` | c5a4f73 |
| O5.2d | 2026-10-11 | diagnosis (`decisions.md` "aye test compile time"): the browser test build is 27.6 MB / 742k lines of C, one single-core `clang -O0 -g` 6.3 s (parse+sema 0.83 s, `-g` adds 2.7 s, `#line` 1 s); splitting into 4–8 translation units compiled in parallel measured 1.4–2.0 s. Fix card O5.2e | e8465cb |
| O7.1 | 2026-10-11 | `docs/aye-bench.md` (cloud 4-core Xeon VM, PR #3): `std.http` hello in fast mode 39.1k req/s at `ab -k -c 64` vs Node 22 10.7k vs Go 1.24 99.3k; `-c 1` Aye 16.0k, Node 5.4k, Go 8.4k; browser build debug 25.6 s / fast 93.9 s, binaries 13.0 MB / 4.0 MB; sources in `scripts/bench/` | 6c80ff3 |
| F1.1b/c | 2026-10-11 | framework: 405 with `Allow` (a `get` route lists `GET, HEAD`), `HEAD` answered from `get` with an empty body, `Request.headers` and case-insensitive `req.header(name)` with a `test` block; `tests/framework.rs` cases are (method, path, file) plus a `web_units` test; `head.http`, `not_allowed.http` | a41bd47 |
| F1.2 | 2026-10-11 | `web gen` (`aye run framework/gen -- <app> [--web <dir>]`) scans `app/` (page.aye, `[slug]`) and writes `.web/` (main, routes table, `r/` symlinks, top-level dir links, `web` link); `[name]` route patterns, `req.param`; example `framework/examples/pages`; `tests/framework.rs` `pages` test; `std.fs.symlink` with a golden; gaps #55, #56 recorded | 9c9745e |
| F1.3 | 2026-10-11 | API routes: `web gen` scans `route.aye` exports (`get`/`post`/`put`/`patch`/`delete`) into per-method routes; `Response.json(*json.Value)` and `jsonText`; example `app/api/items/[id]/route.aye` (JSON, 404 JSON, no `post` → 405 `Allow: GET, HEAD`); `pages` test cases `api_item`, `api_item_post` | 2728a9f |
| F2.1 | 2026-10-11 | static files (FD4): `web.run(..., public)` serves `public/` for GET/HEAD after routes (`framework/web/files.aye`: extension table with charset, `Cache-Control: public, max-age=0`, 400 on `.`/`..`/`\` segments, only regular files, no symlinks); `web gen` bakes the absolute `public` path; example assets; 5 `pages` cases | bc44b9b |
| O5.2e | 2026-10-11 | parallel C compile: programs with ≥ 200 functions are split into a header plus K units (K = min(cores, 8), balanced by size; `AYE_FN`/`AYE_DATA`/`AYE_INIT` linkage macros, shared data in unit 0, per-unit `#line`), K `clang -c` in parallel, one link, `dsymutil` on macOS; one file for `emit-c`, `-c`, non-host targets; `aye build browser` 8.0 → 2.1 s, `aye test browser` 9.7 → 4.7 s; golden passes with the threshold forced to 1 | a339949 |
| O3.4b | 2026-10-11 | `std.http.Server` answers HEAD itself: status, headers and the `Content-Length` of the GET body, no body (`send` takes a `head` flag); the framework no longer empties the body; framework transcripts carry the real lengths; `std_http_serve` golden gains HEAD on keep-alive then GET; spec §15 clause | c1b04ee |
| O5.2g | 2026-10-11 | `aye test` runs `dsymutil` only with `--cflags`; `aye test` and `aye run` delete their temp `.dSYM` (`remove_dsym`); with a private `TMPDIR` the count stays 0 | 062c95e |
| P8.1 | 2026-10-11 | `browser/` builds, checks and tests from outside the repo unchanged (131 tests, 15 modules); `browser/aye.toml` names the binary `anyknown`; `build.sh` uses `$AYE` or PATH; P8 split plan and the P8.2/P8.3 cards in `plans/browser.md` | 4839e71 |
| F2.2 | 2026-10-11 | `layout.aye` (nested, applied inside out via generated page wrappers) and `not-found.aye` (404 for unmatched paths after routes and `public/`); `web.wrap`, `web.runApp`; pages example and transcripts | f06d90b |
| O3.9 | 2026-10-11 | `std.http` worker pool: `Server.workers` (default CPU count, 2–64), each worker `poll`s the shared listener plus its own keep-alive connections, one accepter at a time; `std.os.cpuCount`, `std.net.PollFd` public; Linux 5 vCPU `-c 64` 137k → 236k req/s (Go 305k); macOS within noise | b87d173 |
| O3.6a | 2026-10-11 | `std.http` query, form and cookie decoding: `percentDecode`, `formDecode`, `queryValue`, `formValue`, `cookieValue`, `Request.path/query/form/cookie`; `web.Request.query/form/cookie`; `/api/echo` route and transcripts; `tests/tests/std_http_decode.aye` | a960609 |
| P8.2 | 2026-10-11 | `aye-lang/anyknown` created (private) with `git filter-repo` history (627 commits; `browser/`, `tests/browser.rs`, `tests/browser/`, browser docs; paths unchanged), pinned to Aye b240c35 (`AYE_REV`), `Cargo.toml` tests-only, CI green on macOS and Ubuntu 24.04 (run 38137293064, `AYE_TOKEN` set); local: `aye test browser` 131/131, browser test 62/62 | no Aye commit |
| O7.4 | 2026-10-11 | `Arena.reset()` intrinsic: frees all but the oldest block, rewinds the cursor, keeps the backing allocator; ASan poisons the kept block and unpoisons on alloc; golden `arena_reset`, conformance `heap/arena_reset`, ASan `arena_reset_stale_read`; spec §9 and §15 | 0674de8 |
| F1.3a | 2026-10-11 | API-route errors answered as `{"error":"<name>"}` with the mapped status (`Route.kind` Page/Api); a page returning `Error.NotFound` renders `not-found.aye` (F2.2a); `web gen` scans exports with `aye doc --signatures`; `items/[id]` and `blog/missing` transcripts | 931efae |
| F2.1a | 2026-10-11 | static files: `ETag` (size-mtime), `Last-Modified`, `If-None-Match` (list, `W/`, `*`) and `If-Modified-Since` give 304 with no body; harness scrubs the two headers and a two-step ETag case | 79292be |
| F1.2a | 2026-10-11 | `[...rest]` catch-all (one or more segments, sorted last), `(group)` directories (no URL segment, layouts apply), percent-decoding after splitting on `/` for matching and `param`; `docs/[...path]`, `(marketing)/pricing` | 4155eff |
| O7.4a | 2026-10-11 | `std.http`: one arena per keep-alive connection (`Open.arena`, `reset` after each response and before the `Incomplete` return); golden sends 20 requests on one connection; macOS bench within noise (119k → 124k at `-c 64`) | b502b5a |
| O5.2h | 2026-10-11 | tree-shake already existed (`oir::reachable`, M9.6): `aye build browser` has no dead functions; when compiling tests `main` is no longer a root (test C −15%, CPU −9%); golden `tests/tree_shake` checks the generated C | 51f843e |
| O6.1a | 2026-10-11 | `Diag` end spans (unknown names, unexpected tokens, `;`, number literals, type mismatches) and `severity`; `aye check --json` gains `end_line`/`end_col`; LSP `codeAction` quickfixes from fix-its; import errors as `relatedInformation`; lsp 7 tests | 25cd48e |
| P8.3 | 2026-10-11 | `browser/`, `tests/browser*`, and the browser docs removed from the Aye repo (they live in aye-lang/anyknown); `docs/plans/anyknown.md` lane file; `scripts/downstream.sh` + `tests/downstream/ANYKNOWN_REV` replace M1 (3 min: check, 131 unit tests, 62 browser tests); stay-side mentions fixed | 1c88f43 |
| O6.2a | 2026-10-11 | LSP hover and definition through types: sema records resolved member accesses (`Root::members` flag, off by default), `x.method()`, `x.field`, chains, type-function members, path deps; past-the-end cursor; block scopes; declaration renderer shared between `doc.rs` and `lsp/nav.rs`; lsp 10 tests | O62A |
| O4.3a/b | 2026-10-11 | the `aye.lock:` notice goes to stderr (stdout equals program output, `tests/deps.rs` checks both); `aye.lock` is read and validated whenever it exists, a malformed one is E0119 even without URL deps (`tests/deps/errors/bad_lock`); decisions and spec §12 updated | 033996d |
| O1.15b | 2026-10-11 | std module suggestion (E0081 and `aye doc`): `closest_std_module` picks the smallest edit distance (ties: shorter, then alphabetical), no first-two-letters rule; distance ≤ 2 only when 2·distance ≤ shorter length, distance 3 only for names ≥ 7 letters, so `std.txt`/`std.jsno` suggest `text`/`json` while `std.io`/`std.string` get none; goldens `std_unknown_{omitted,swapped,far}` | c61c137 |
| O3.1b | 2026-10-11 | `aye_std_ecvt` uses `snprintf("%.*e")` on Linux too (glibc `ecvt` rounds the 17th digit, `DBL_MAX` did not round-trip through `std.json`) | 48a4043 |

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
