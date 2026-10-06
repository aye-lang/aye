#!/bin/sh
# Fast-mode end-to-end wall time (startup, parse, execution, teardown).
# BENCH_BIN can name a saved baseline; BENCH_RUNS defaults to 3.
set -eu
cd "$(dirname "$0")/../../.."
if [ -z "${BENCH_BIN:-}" ]; then
    browser/build.sh
fi
exec python3 - "$@" <<'PY'
import os, pathlib, shutil, statistics, subprocess, sys, time
binary = os.environ.get("BENCH_BIN", "target/anyknown")
runs = int(os.environ.get("BENCH_RUNS", "3"))
paths = sorted(pathlib.Path("browser/tests/jsbench").glob("*.js"))
if len(sys.argv) > 1:
    paths = [p for p in paths if p.stem in sys.argv[1:]]
print("benchmark\tmedian_seconds\tchecksum", flush=True)
for path in paths:
    golden = path.with_suffix(".out").read_bytes()
    if shutil.which("node"):
        node = subprocess.run(["node", str(path)], capture_output=True, check=True)
        if node.stdout != golden:
            raise SystemExit(f"{path}: Node checksum differs from golden")
    times = []
    for _ in range(runs):
        start = time.perf_counter()
        result = subprocess.run([binary, "js", str(path)], capture_output=True, timeout=300)
        times.append(time.perf_counter() - start)
        if result.returncode or result.stdout != golden or result.stderr:
            raise SystemExit(f"{path}: failed: {result.stdout!r} {result.stderr!r}")
    print(f"{path.stem}\t{statistics.median(times):.6f}\t{golden.decode().strip()}", flush=True)
PY
