# Standalone JS benchmarks

Run `browser/tests/jsbench/run.sh` from any directory (requires Python 3).
It builds
`target/anyknown` in fast mode, verifies each stdout checksum (also with
Node when installed), and reports the median of three end-to-end wall
times in seconds. Startup, parsing, execution, and teardown are included.
Use `BENCH_RUNS=5` for more samples, `BENCH_BIN=/path/to/saved/binary`
to compare builds without rebuilding, and arguments to select workloads
(e.g. `run.sh richards methods`). Set `BENCH_BASELINE=/path/to/old/binary`
to interleave both builds, alternating their order each round, and print
per-workload speedups and their geometric mean. Both binaries must have
been built in fast mode. A mismatch or timeout is a failure.

Richards, DeltaBlue, and NavierStokes are standalone ports from
https://github.com/chromium/octane (the original algorithms are unchanged).
Their source files retain the upstream license notices; DeltaBlue is
GPL-2.0-or-later, separately from the engine. The Octane harness is removed,
assertions retained, and deterministic checksums/iteration counts added.
DeltaBlue's browser `alert` failures throw instead. NavierStokes runs the
original 128×128, 20-iteration solver for 15 frames and checks 77.
The other five workloads are written for this suite.

Initial baseline: Apple M2 Pro, arm64 macOS, fast (-O2), compiler/repo
`5dd6afb`. Median of three runs; Node v22.22.2 agrees with all checksums.

| workload | seconds | checksum |
| --- | ---: | ---: |
| array | 0.115265 | 96751118 |
| closures | 0.830995 | 306584040 |
| deltablue | 0.188738 | 2282550 |
| json | 0.253661 | 20672500 |
| methods | 0.128533 | 267460185993 |
| navier-stokes | 4.306529 | 77 |
| richards | 0.692682 | 2916400 |
| strings | 1.245596 | 650400 |

These are whole-process measurements, not Octane scores. The machine is
shared with other agents; use repeated/interleaved runs for comparisons.
See `browser/js/PERF.md` for the final comparison and profiling.
