#!/bin/sh
# Fast-mode end-to-end wall time (startup, parse, execution, teardown).
# BENCH_BIN can name a saved baseline; BENCH_RUNS defaults to 3.
set -eu
cd "$(dirname "$0")/../../.."
if [ -z "${BENCH_BIN:-}" ]; then
    browser/build.sh
fi
exec python3 - "$@" <<'PY'
import math, os, pathlib, shutil, statistics, subprocess, sys, time
binary = os.environ.get("BENCH_BIN", "target/anyknown")
runs = int(os.environ.get("BENCH_RUNS", "3"))
baseline = os.environ.get("BENCH_BASELINE")
if runs < 1:
    raise SystemExit("BENCH_RUNS must be at least 1")
paths = sorted(pathlib.Path("browser/tests/jsbench").glob("*.js"))
if len(sys.argv) > 1:
    paths = [p for p in paths if p.stem in sys.argv[1:]]
if not paths:
    raise SystemExit("no matching benchmarks")
header = "benchmark\tmedian_seconds\tchecksum"
if baseline:
    header = "benchmark\tbaseline_seconds\tseconds\tspeedup\tchecksum"
print(header, flush=True)
speedups = []
for path in paths:
    golden = path.with_suffix(".out").read_bytes()
    if shutil.which("node"):
        node = subprocess.run(["node", str(path)], capture_output=True, check=True)
        if node.stdout != golden:
            raise SystemExit(f"{path}: Node checksum differs from golden")
    times = {binary: []}
    if baseline:
        times[baseline] = []
    for i in range(runs):
        order = list(times)
        if i % 2:
            order.reverse()
        for exe in order:
            start = time.perf_counter()
            result = subprocess.run([exe, "js", str(path)], capture_output=True, timeout=300)
            times[exe].append(time.perf_counter() - start)
            if result.returncode or result.stdout != golden or result.stderr:
                raise SystemExit(f"{path} ({exe}): failed: {result.stdout!r} {result.stderr!r}")
    after = statistics.median(times[binary])
    if baseline:
        before = statistics.median(times[baseline])
        speedup = before / after
        speedups.append(speedup)
        print(f"{path.stem}\t{before:.6f}\t{after:.6f}\t{speedup:.3f}\t{golden.decode().strip()}", flush=True)
    else:
        print(f"{path.stem}\t{after:.6f}\t{golden.decode().strip()}", flush=True)
if speedups:
    mean = math.exp(sum(math.log(x) for x in speedups) / len(speedups))
    print(f"geometric_mean\t\t\t{mean:.3f}", flush=True)
PY
