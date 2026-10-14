# Aye benchmarks (O7.1, criterion O12)

Measured 2026-10-14 on a cloud VM (Claude cloud session), commit `0e7463d` (O7.5a). Since the 2026-10-11 table below, `std.http` got
a worker pool (O3.9), one arena per keep-alive connection (O7.4a), slow-handler give-away (O3.9b) and a `SO_REUSEPORT` listener
per worker (O3.9c).

- Machine: Intel Xeon @ 2.80GHz (`lscpu`), `nproc` 4 (1 thread per core), 15 GiB RAM, Ubuntu 24.04, Linux 6.18.44 (Firecracker microVM). Load average 0.3 before, 1.7 to 2.9 during (this session only; no steal time in `vmstat`).
- Tools: Node v22.22.0, Go 1.24.7, ApacheBench 2.3, `wrk` 4.1.0 (Ubuntu package), clang from `scripts/setup-ubuntu.sh`.
- All three servers and the client ran on this same VM (127.0.0.1), one server at a time, 1000 `ab -k -c 64` warm-up requests after start. The client shares the 4 cores with the server, so absolute numbers are lower than a dedicated load generator would give; the comparison is what counts. The same VM label (2.80 GHz) as the 2026-10-11 table, but every server is slower here than on the 2.10 GHz VM of F5.9 (Go 68k vs 126k under `ab`), so compare rows within one table only.

## Requests per second

### `ab` (the `scripts/bench/run.sh` commands)

`ab -k -c 64 -n 200000` three times (median shown, runs in the table), `-c 1` is `-n 50000` once. Two passes: aye, node, go,
then go, node, aye; the second pass is in brackets. Unpinned. No failed requests.

| server | -c 64 req/s (median of 3) | runs | mean latency (ms) | -c 1 req/s | -c 1 mean (ms) |
|---|---|---|---|---|---|
| Aye `std.http`, `--mode fast` | 80378 [80926] | 82351 / 80378 / 79330 [80926 / 82851 / 77756] | 0.796 | 15936 [14830] | 0.063 |
| Node 22 `http` | 7393 [7649] | 7236 / 7793 / 7393 [7401 / 7649 / 7752] | 8.657 | 4301 [4313] | 0.233 |
| Go 1.24 `net/http` | 67592 [69938] | 67592 / 68869 / 60563 [69938 / 73319 / 65381] | 0.947 | 7685 [7766] | 0.130 |

### `wrk` (multi-threaded client)

`ab` is single-threaded and was the limit in O3.9d and O3.9e, so the same servers were also measured with
`wrk -t2 -c64 -d10s --latency http://127.0.0.1:<port>/` (keep-alive), three runs per server, median shown; latency columns are
from the median run. No socket errors, no non-2xx.

| server | req/s (median of 3) | runs | p50 | p99 |
|---|---|---|---|---|
| Aye `std.http`, `--mode fast` | 145966 | 135443 / 145966 / 146872 | 0.224 ms | 3.92 ms |
| Node 22 `http` | 16556 | 15419 / 16569 / 16556 | 3.52 ms | 8.56 ms |
| Go 1.24 `net/http` | 70270 | 70270 / 71822 / 69976 | 0.593 ms | 7.69 ms |

Pinned with `taskset` (server on CPUs 0-1, `wrk` on 2-3), three runs:

| server | req/s (median of 3) | runs | p50 | p99 |
|---|---|---|---|---|
| Aye `std.http`, `--mode fast` | 146931 | 146931 / 156117 / 146250 | 0.238 ms | 6.31 ms |
| Node 22 `http` | 15663 | 15419 / 15663 / 18501 | 3.70 ms | 9.98 ms |
| Go 1.24 `net/http` | 51944 | 51944 / 43065 / 54406 | 1.16 ms | 5.02 ms |

Pinning did not make the passes steadier here (Aye 146k to 156k pinned vs 135k to 147k unpinned; Go 43k to 54k pinned vs 70k to
72k unpinned) and costs Go, which loses two of its four cores, so the unpinned table is the headline.

Script used (not committed): start the server, `curl` once, the warm-up, then the `ab` or `wrk` runs above, then SIGTERM.

## 2026-10-11 table (thread per connection, commit `7babce6`)

Measured 2026-10-11 on a cloud VM, before O3.9: `std.http.Server` started a thread per connection.
Machine: Intel Xeon @ 2.80GHz, 4 cores (1 thread each), 15 GiB RAM, Ubuntu 24.04, Linux 6.18. Node v22.22.0, Go 1.24.7, ApacheBench 2.3.
`ab -k -c 64 -n 200000` three times after 1000 warm-up requests (median shown; runs in the table note); `-c 1` is `-n 50000` once.

| server | -c 64 req/s (median of 3) | runs | mean latency (ms) | -c 1 req/s | -c 1 mean (ms) | failed |
|---|---|---|---|---|---|---|
| Aye `std.http`, `--mode fast` | 39142 | 41631 / 39142 / 38024 | 1.635 | 16029 | 0.062 | 0 |
| Node 22 `http` | 10709 | 10709 / 10020 / 10859 | 5.976 | 5384 | 0.186 | 0 |
| Go 1.24 `net/http` | 99284 | 99284 / 88924 / 101272 | 0.645 | 8450 | 0.118 | 0 |

## 2026-10-11 framework hello and pages /about (F5.9, Ubuntu 24.04)

Same flags as above (`ab -k -c 64 -n 200000` three times after 1000 warm-up requests, median; `-c 1` is `-n 50000` once), one server at a time, `ab` on the same VM.
Machine: Intel Xeon @ 2.10GHz, `nproc` 4, 15 GiB, Linux 6.18.44 (`fc` kernel: a Firecracker microVM, cloud session), Ubuntu 24.04. Load average before the run 1.07, after 1.83 (nothing else running).
This is a different VM shape from the table at the top (2.10 GHz, not 2.80 GHz) and `std.http` has had the O3.9 worker pool since then, so compare rows within this table only.
Node v22, Go 1.24, ApacheBench 2.3. The `framework` hello is `framework/examples/hello` built with `aye build --mode fast` (a hand-written route table, no `app/` tree, so `framework/build` does not apply); the pages `/about` server is `framework/examples/pages` built with `aye run framework/build -- ... --mode fast`.

| server | -c 64 req/s (median of 3) | runs | mean latency (ms) | -c 1 req/s | -c 1 mean (ms) | failed |
|---|---|---|---|---|---|---|
| Aye `std.http` hello, `--mode fast` | 128045 | 128045 / 132103 / 126346 | 0.500 | 26326 | 0.038 | 0 |
| Node 22 `http` | 20003 | 19139 / 20003 / 20851 | 3.200 | 8885 | 0.113 | 0 |
| Go 1.24 `net/http` | 126280 | 123991 / 126280 / 131988 | 0.507 | 14229 | 0.070 | 0 |
| Aye framework hello `/`, `--mode fast` | 127707 | 121547 / 128781 / 127707 | 0.501 | 25848 | 0.039 | 0 |
| Aye framework pages `/about`, `--mode fast` | 121143 | 121143 / 117114 / 129186 | 0.528 | 25534 | 0.039 | 0 |

The framework hello page does 127.7k req/s at `-c 64`, 6.4x Node 22 and within 0.3% of the bare `std.http` hello (the three runs of each overlap), so the framework's routing and response code add no measurable cost at this size. The pages example's `/about` (a template page in a 11-route app, 269,680-byte executable) is 5% lower (121.1k, inside the run-to-run spread of about ±5%). Run with `scripts/bench/run.sh`, which now builds all five servers (ports 8081 to 8085, `ab` output in `target/ab_*.txt`).

## Build times of `browser/`

The browser is in aye-lang/anyknown since P8.3; the commands below run in the checkout `scripts/downstream.sh` makes at `target/downstream/anyknown`.

Wall clock, three runs each, `target/release/aye build` (fast mirrors `browser/build.sh`).

| mode | runs (s) | median (s) |
|---|---|---|
| debug (default) | 30.3 / 25.3 / 25.6 | 25.6 |
| `--mode fast` | 93.9 / 96.0 / 92.2 | 93.9 |

## Binary sizes

| binary | bytes |
|---|---|
| `anyknown_debug` | 12,988,952 |
| `anyknown_fast` | 3,977,880 |
| hello server, Aye fast | 43,600 |
| hello server, Go | 8,215,565 |

## How to reproduce

```sh
sh scripts/setup-ubuntu.sh; . /etc/profile.d/aye-wasi.sh
cargo build --release
target/release/aye build --mode fast scripts/bench/hello_http.aye -o target/hello_aye
go build -o target/hello_go scripts/bench/hello_go.go
scripts/bench/run.sh            # ab output lands in target/ab_*.txt
sudo apt-get install -y wrk     # the wrk table: one server at a time
wrk -t2 -c64 -d10s --latency http://127.0.0.1:8081/
time target/release/aye build browser -o target/anyknown_debug
time target/release/aye build --mode fast browser -o target/anyknown_fast
```

Sources: [`scripts/bench/hello_http.aye`](../scripts/bench/hello_http.aye) (port 8081),
[`hello_node.js`](../scripts/bench/hello_node.js) (8082), [`hello_go.go`](../scripts/bench/hello_go.go) (8083), driver [`run.sh`](../scripts/bench/run.sh).
All answer `GET /` with `hello\n` and a `Content-Type: text/plain` header.

## O12

O12 holds on this machine (2026-10-14, `0e7463d`): the Aye `--mode fast` hello server does 146.0k req/s under `wrk -t2 -c64`
against 16.6k for Node 22 (8.8x) and 70.3k for Go `net/http` (2.1x), with a p50 latency of 0.22 ms against 3.5 ms and 0.59 ms.
Under `ab -k -c 64` (the criterion's tool) it is 80.4k against 7.4k for Node (10.9x) and 67.6k for Go (1.19x), and at `-c 1`
15.9k against 4.3k and 7.7k req/s. Aye is now ahead of Go at `-c 64` too, where the 2026-10-11 table had it at 0.4x.

Why it changed: the 2026-10-11 server started a thread per connection (64 OS threads on 4 cores shared with `ab`); O3.9 replaced
that with a pool of one worker per CPU, O7.4a reuses one arena per connection and O3.9c gives each worker its own listener. With
`ab` the Aye number is `ab`'s single thread (O3.9d: `ab` at 97% of one CPU), which is why `wrk` shows 1.8x more from the same
server; Node and Go also gain under `wrk` (2.2x, 1.04x). This is a single-VM, single-shape measurement (4 cores, loopback,
tiny body, client on the same cores), not a general claim.

2026-10-11 analysis, kept for history: Go won under concurrency because `std.http.Server` started a thread per connection and
allocated a job and an arena per connection and request, while Go multiplexes goroutines over an epoll loop.

Follow-up cards (proposed):
- O7.2: profile `std.http` at `-c 64` (`perf`), count context switches and per-request allocations.
- O7.3: `std.http` accept loop with a fixed worker pool instead of a thread per connection.
- O7.4: reuse the per-request arena across keep-alive requests on one connection.
- O7.5: rerun this file on a machine with 8+ cores and a separate load generator. The 4-core cloud VM part is done (O7.5a, 2026-10-14, the table at the top); the 8+ core machine with a separate load generator stays open (the cloud VM has 4 cores and no second machine).

## 2026-10-11 worker pool (macOS, local)

O3.9 replaced the thread per connection with a fixed pool: `Server.workers` threads (the CPU count, 2 to 64, set before `run`),
each polling the shared listener and the connections it accepted; one worker accepts at a time, after a `poll` that does not wait.
"before" is `04af3d9`, "after" is this branch; same `scripts/bench/hello_http.aye`, `--mode fast`,
`ab -k -c 64 -n 100000` three times after 1000 warm-up requests, `-c 1` once (`-n 50000` on macOS, `-n 30000` on Linux). No failed requests anywhere.

MacBook (10 cores: 6 performance, 4 efficiency; macOS 26.5), host, `ab` on the same machine. Other agents were building and testing at the same time
(`uptime` load average 11 to 15 for these two rounds), so the noise is about ±15%:

| round (load) | server | -c 64 runs (req/s) | median | -c 1 req/s |
|---|---|---|---|---|
| 3 (15.5) | before | 104625 / 84207 / 95886 | 95886 | 42719 |
| 3 (14.6) | after (10 workers) | 102154 / 109451 / 105224 | 105224 | 35488 |
| 4 (11.1) | after (10 workers) | 92119 / 85562 / 79796 | 85562 | 32971 |
| 4 (11.4) | before | 101906 / 106339 / 98622 | 101906 | 37518 |

On the macOS host the two are the same within the noise: single-threaded `ab` is the limit (one earlier `before` round at load 5 gave
120718 median and 52154 at `-c 1`). With `workers = 2`, 64 keep-alive connections are still all served (74.7k req/s, 0 failed), and so are
5000 requests without keep-alive at `-c 16`.

Linux container on the same Mac (`scripts/linux.sh`, Ubuntu 24.04 arm64, 5 vCPUs, load 1.2; Go 1.22.2 from the image), servers in order
before, after, Go, after, before:

| server | -c 64 runs, first pass (req/s) | second pass | medians | -c 1 req/s |
|---|---|---|---|---|
| before (thread per connection) | 137204 / 132077 / 139395 | 147786 / 163128 / 136304 | 137204, 147786 | 31354, 33506 |
| after (5 workers) | 236800 / 217537 / 236115 | 262703 / 223102 / 249150 | 236115, 249150 | 30165, 37741 |
| Go `net/http` | 168947 / 312386 / 305329 | | 305329 | 18548 |

On Linux the pool is 1.7x the thread per connection at `-c 64` (about 0.8x Go, from 0.45x), and `-c 1` is unchanged. The cloud VM rerun
(4 cores, the table at the top) is done (O7.5a, 2026-10-14).

2026-10-11 O7.4a (macOS, local, load 5 to 8): one arena per keep-alive connection, reset after each request, instead of one per request. `ab -k -c 64 -n 100000`, median of 3, over 3 to 4 rounds: before 119k req/s (rounds 116.6k to 121.8k), after 124k (123.6k to 125.3k; one round at 66k while another job ran); `-c 1`: 48.0k to 48.9k before, 43.2k to 48.4k after. Within the noise: the hello handler allocates little, so the saving is not visible here.

## 2026-10-12 slow handlers (O3.9b, Ubuntu 24.04 cloud VM)

O3.9b lets another worker take the connections of a worker whose handler runs for more than 20 ms (docs/decisions.md). Same VM shape
as the F5.9 table (`nproc` 4, Linux 6.18.44 Firecracker, Ubuntu 24.04), `scripts/bench/hello_http.aye` built `--mode fast`, the
commands of `scripts/bench/run.sh` (`ab -k -c 64 -n 200000` three times after 1000 warm-up requests; `-c 1 -n 50000` once), then
`ab -c 64 -n 50000` without `-k`. "before" is `e643c63` (main), "after" this branch. Three passes (before/after, before/after,
after/before), load average 0.7 to 2.5 (this session only; nothing else ran). No failed requests anywhere.

| pass | server | -c 64 runs (req/s) | median | -c 1 req/s | no -k, -c 64 req/s |
|---|---|---|---|---|---|
| 1 | before | 93044 / 81993 / 85797 | 85797 | 15948 | 9202 |
| 1 | after | 95147 / 96498 / 89326 | 95147 | 16131 | 9202 |
| 2 | before | 100957 / 105847 / 90160 | 100957 | 16952 | 9720 |
| 2 | after | 87722 / 99042 / 87953 | 87953 | 15299 | 9290 |
| 3 | after | 86286 / 92481 / 100982 | 92481 | 16861 | 9889 |
| 3 | before | 88315 / 96069 / 99632 | 96069 | 17620 | 9802 |

Median of the pass medians: before 96069, after 92481 (-3.7%); `-c 1` 16952 and 16131 (-4.8%). The passes disagree in sign
(+11%, -13%, -3.7%), so the change is inside the run-to-run noise of this VM (about ±10%), under the 10% limit.

Thundering herd: without `-k` (`ab -c 64 -n 50000`, three runs) the hello server does 15214 / 16227 / 16069 req/s with
`workers = 1` and 9392 / 9829 / 9226 with the default 4 workers: every worker `poll`s the listener, so each new connection wakes
all of them and only one accepts (`tryAccept`); 4 workers are 42% slower than 1 for a new connection per request. Proposed
follow-up: one listener per worker with `SO_REUSEPORT` on Linux (the kernel picks the worker), or an accept hand-off.

## 2026-10-12 accept herd (O3.9c, Ubuntu 24.04 cloud VM)

O3.9c gives each `std.http` worker its own listener on Linux (`SO_REUSEPORT`, docs/decisions.md), so the kernel hands a new
connection to one worker instead of waking all four. `nproc` 4, Linux 6.18.44 Firecracker, Ubuntu 24.04, load average 1.9 to
2.6 (this session only), `scripts/bench/hello_http.aye` built `--mode fast`, 1000 `-k` warm-up requests, then `ab -c 64 -n
100000` without `-k` three times and `ab -k -c 64 -n 200000` three times (the `run.sh` command). "before" is `228c89a` (main),
"after" this branch. Six passes, alternating order; passes 1 and 4 ran only one side. No failed requests anywhere.

| pass | server | no -k, -c 64 runs (req/s) | median | -k -c 64 runs (req/s) | median |
|---|---|---|---|---|---|
| 1 | before | 9066 / 8982 / 9025 | 9025 | 108560 / 107927 / 112898 | 108560 |
| 1 | after | 12534 / 12145 / 12172 | 12172 | 87143 / 86187 / 85724 | 86187 |
| 2 | before | 9121 / 9479 / 9188 | 9188 | 110731 / 106086 / 93067 | 106086 |
| 2 | after | 10728 / 13046 / 12479 | 12479 | 90591 / 89993 / 100163 | 90591 |
| 3 | before | 9441 / 8960 / 9077 | 9077 | 93826 / 90471 / 82150 | 90471 |
| 3 | after | 11920 / 12259 / 12130 | 12130 | 97274 / 105949 / 101287 | 101287 |
| 4 | after | 12509 / 12524 / 13351 | 12524 | 96494 / 99185 / 91816 | 96494 |
| 5 | after | 11909 / 13386 / 13111 | 13111 | 85065 / 87112 / 102800 | 87112 |
| 5 | before | 8970 / 9138 / 9253 | 9138 | 109480 / 92747 / 90761 | 92747 |
| 6 | before | 9280 / 9344 / 8990 | 9280 | 93895 / 108970 / 103002 | 103002 |
| 6 | after | 12484 / 13046 / 12441 | 12484 | 91049 / 107041 / 99284 | 99284 |

Median of the pass medians: without `-k` 9138 → 12482 req/s (+37%); with `-k` 103002 → 93543 (-9.2%, under the 10% limit).
The paired `-k` passes disagree in sign (-15%, +12%, -6%, -3.6%), inside this VM's ±10% noise, but the after side is lower more
often than not. A likely cause: the kernel hashes the 64 keep-alive connections over the four listeners, so some worker gets
more than 16, where the herd let an idle worker win them. Still below `workers = 1` (16.1k in O3.9b) without `-k`: each
connection costs a `makeOpen` under the server-wide `openLock` and a `General` of its own. Accepting up to 16 waiting
connections per wake instead of one measured the same (12706 / 104698 in one pass) and was dropped.

## 2026-10-12 open lock (O3.9d, Ubuntu 24.04 cloud VM)

O3.9d asked whether `makeOpen` and `closeOpen` under the server-wide `openLock` hold back new connections without `-k`. Upper
bound first: a build of `a6339c4` with the lock taken out of both (safe only in `--mode fast`, where `General` is plain
malloc/free) against `a6339c4` itself. `nproc` 4, Linux 6.18.44 Firecracker, Ubuntu 24.04, load average 1.5 to 2.4 (this session
only), `scripts/bench/hello_http.aye` built `--mode fast`, 1000 `-k` warm-up requests, then `ab -c 64 -n 100000` without `-k`
three times and `ab -k -c 64 -n 200000` three times. No failed requests anywhere.

| pass | server | no -k, -c 64 runs (req/s) | median | -k -c 64 runs (req/s) | median |
|---|---|---|---|---|---|
| 1 | before | 12015 / 11386 / 11437 | 11437 | 86782 / 88498 / 90921 | 88498 |
| 2 | no lock | 11733 / 12053 / 12321 | 12053 | 95821 / 93887 / 89988 | 93887 |
| 2 | before | 11868 / 11964 / 11804 | 11864 | 91698 / 80277 / 75168 | 80277 |
| 3 | before | 10440 / 11849 / 11321 | 11321 | 77972 / 79914 / 73451 | 77972 |
| 3 | no lock | 11334 / 11521 / 11190 | 11334 | 81618 / 87135 / 85328 | 85328 |

Paired without `-k`: +1.6% and +0.1%, inside this VM's noise. The client is the limit: during a no-`-k` run `ab` uses 97% of one
CPU while the server uses about 55% of one CPU over its 4 workers (`top`), and two `ab -c 32 -n 100000` at once reach 12674 + 13364
and 13151 + 12548 req/s before, 14135 + 13997 and 12398 + 12729 without the lock (about 26k together either way). The lock is
taken twice per connection, uncontended at this rate, so it was left as is. To see server-side gains on the accept path, a
multi-threaded client (`wrk`, or several `ab`) is needed first.

## 2026-10-14 keep-alive skew under `SO_REUSEPORT` (O3.9e, Ubuntu 24.04 cloud VM)

O3.9e asked whether the kernel's hash of keep-alive connections over the per-worker listeners (O3.9c) leaves a worker idle while
another queues. `nproc` 4, Linux 6.18.44 Firecracker, Ubuntu 24.04, load average 1.0 to 4.0 (this session only), main `cd75835`,
`wrk` 4.1.0 (Ubuntu package), `scripts/bench/hello_http.aye` built `--mode fast`. Each run starts a fresh server, sends one `curl`,
then `wrk -t2 -c64 -d10s http://127.0.0.1:8081/` (keep-alive, wrk's default) or the same with `-H 'Connection: close'`; the
server gets SIGTERM after it. Variants: `aye` is the bench hello (4 workers); `w1` the same with `s.workers = 1` (and
`os.onStop()`); `cnt4`/`cnt1` the same built with a temporary, uncommitted counter in `workerMain` (requests served and
connections accepted per worker, printed at stop). CPU per worker thread is `utime + stime` from `/proc/<pid>/task/*/stat`
over the run. No socket errors and no non-2xx anywhere.

Unpinned, three passes (req/s; median last):

| server | keep-alive | median | `Connection: close` | median |
|---|---|---|---|---|
| aye, 4 workers | 147999 / 157481 / 155966 | 155966 | 27536 / 27812 / 28564 | 27812 |
| cnt4, 4 workers | 160567 / 162850 / 159252 | 160567 | 28356 / 30210 / 30087 | 30087 |
| w1, 1 worker | 79074 / 87334 / 68492 | 79074 | 31517 / 31584 / 31927 | 31584 |
| cnt1, 1 worker | 67249 / 80592 / 60847 | 67249 | 32673 / 32505 / 32564 | 32564 |

Pinned with `taskset` (server on CPUs 0-1, `wrk` on 2-3), three passes:

| server | keep-alive | median | `Connection: close` | median |
|---|---|---|---|---|
| cnt4, 4 workers | 189705 / 178625 / 181646 | 181646 | 30838 / 29438 / 29068 | 29438 |
| cnt1, 1 worker | 63101 / 67091 / 59905 | 63101 | 33031 / 31755 / 32781 | 32781 |

Per worker, keep-alive, `cnt4` (requests served / connections; max/min of requests; requests per connection; CPU per worker):

| run | worker 0 | worker 1 | worker 2 | worker 3 | max/min | req per conn | CPU |
|---|---|---|---|---|---|---|---|
| unpinned 1 | 396122 / 16 | 297252 / 11 | 392267 / 17 | 536308 / 22 | 1.80 | 23.1k to 27.0k | 37 to 54% |
| unpinned 2 | 241665 / 10 | 599723 / 25 | 463812 / 18 | 325529 / 13 | 2.48 | 24.0k to 25.8k | 35 to 58% |
| unpinned 3 | 379162 / 18 | 328316 / 13 | 493760 / 20 | 393508 / 15 | 1.50 | 21.1k to 26.2k | 40 to 51% |
| pinned 1 | 480406 / 18 | 410127 / 12 | 529062 / 19 | 481208 / 17 | 1.29 | 26.7k to 34.2k | 44 to 51% |
| pinned 2 | 485073 / 18 | 551089 / 20 | 316606 / 10 | 438276 / 18 | 1.74 | 24.3k to 31.7k | 40 to 51% |
| pinned 3 | 444269 / 14 | 447412 / 15 | 562181 / 25 | 366149 / 12 | 1.54 | 22.5k to 31.7k | 42 to 53% |

(Connection counts include the one `curl`.) With `Connection: close` the workers serve within 4% of each other in every run.

Decision: no code change. The request skew (1.3x to 2.5x) is the connection skew of the hash (10 to 25 connections per worker,
64 in all), not a worker that queues: unpinned, every connection gets about the same rate whichever worker holds it (requests per
connection within 8 to 24%), and no worker thread goes above 58% of a CPU, so the worker with 25 connections is not the limit;
the client's per-connection round trip is. Pinned to two CPUs, the four workers split those CPUs evenly (40 to 53% each), so a
worker with more connections gives each a little less (22.5k vs 31.7k in pass 3), but the CPUs are full and moving connections
would not add requests. Keep-alive with 4 workers is about twice the 1-worker number (156k to 182k vs 63k to 79k median), far
from "clearly below". A hand-off between workers would cost a lock or a wake per moved connection for no throughput here.
O3.9c's -9% `ab -k` result was `ab`'s single thread: `wrk` gets 156k to 182k from the same server. `taskset` made the keep-alive
passes steadier (179k to 190k pinned vs 148k to 163k unpinned) and leaves the decision as it is.

Aside, not this card: with `Connection: close` 4 workers (28k to 30k) are slightly below 1 worker (31.5k to 33k) in every pass,
pinned or not; each worker uses about 27% of a CPU, so the server is not CPU-bound there either (likely the kernel's connection
setup and `wrk`'s reconnects).
