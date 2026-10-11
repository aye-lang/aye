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

## Build times of `browser/`

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
