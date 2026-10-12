# Aye benchmarks (O7.1, criterion O12)

Measured 2026-10-11 on a cloud VM (Claude cloud session), commit `7babce6`.

- Machine: Intel Xeon @ 2.80GHz, 4 cores (1 thread each), 15 GiB RAM, Ubuntu 24.04, Linux 6.18.
- Tools: Node v22.22.0, Go 1.24.7, ApacheBench 2.3, clang from `scripts/setup-ubuntu.sh`.
- All three servers and `ab` ran on this same VM (127.0.0.1), one server at a time. `ab` is single-threaded and shares the 4 cores with the server, so absolute numbers are lower than a dedicated load generator would give; the comparison is what counts.

## Requests per second

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
time target/release/aye build browser -o target/anyknown_debug
time target/release/aye build --mode fast browser -o target/anyknown_fast
```

Sources: [`scripts/bench/hello_http.aye`](../scripts/bench/hello_http.aye) (port 8081),
[`hello_node.js`](../scripts/bench/hello_node.js) (8082), [`hello_go.go`](../scripts/bench/hello_go.go) (8083), driver [`run.sh`](../scripts/bench/run.sh).
All answer `GET /` with `hello\n` and a `Content-Type: text/plain` header.

## O12

O12 holds on this machine: the Aye `--mode fast` hello server does 39.1k req/s at `-c 64`
against 10.7k for Node 22 (3.7x), with a mean latency of 1.6 ms against 6.0 ms, and 16.0k against 5.4k req/s at `-c 1`.
Against the second reference, Go `net/http`, Aye is behind at `-c 64` (39.1k vs 99.3k, 0.4x) but ahead at `-c 1` (16.0k vs 8.4k, a lower latency floor).

Why Go wins under concurrency: `std.http.Server` starts a thread per connection (`std/http/http.aye`, `Server.run`) and
allocates a job and an arena per connection and request, while Go multiplexes goroutines over an epoll loop.
With 64 keep-alive connections on 4 cores shared with `ab`, 64 OS threads cost context switches. This is a single-VM, single-shape measurement (4 cores, loopback, tiny body), not a general claim.

Follow-up cards (proposed):
- O7.2: profile `std.http` at `-c 64` (`perf`), count context switches and per-request allocations.
- O7.3: `std.http` accept loop with a fixed worker pool instead of a thread per connection.
- O7.4: reuse the per-request arena across keep-alive requests on one connection.
- O7.5: rerun this file on a machine with 8+ cores and a separate load generator.

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
(4 cores, the table at the top) is still to do.

2026-10-11 O7.4a (macOS, local, load 5 to 8): one arena per keep-alive connection, reset after each request, instead of one per request. `ab -k -c 64 -n 100000`, median of 3, over 3 to 4 rounds: before 119k req/s (rounds 116.6k to 121.8k), after 124k (123.6k to 125.3k; one round at 66k while another job ran); `-c 1`: 48.0k to 48.9k before, 43.2k to 48.4k after. Within the noise: the hello handler allocates little, so the saving is not visible here.
