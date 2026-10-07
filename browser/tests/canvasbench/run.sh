#!/bin/sh
# Canvas 2D micro benchmark: milliseconds spent in drawing calls (median
# of BENCH_RUNS, default 3) and a hash of each workload's PNG.
# BENCH_BIN names a saved binary instead of building target/anyknown;
# BENCH_BASELINE interleaves an older binary, prints speedups and fails
# when a workload's pixels differ. Arguments select workloads.
set -eu
cd "$(dirname "$0")/../../.."
if [ -z "${BENCH_BIN:-}" ]; then
    browser/build.sh
fi
exec python3 - "$@" <<'PY'
import os, pathlib, statistics, subprocess, sys
binary = os.environ.get("BENCH_BIN", "target/anyknown")
runs = int(os.environ.get("BENCH_RUNS", "3"))
baseline = os.environ.get("BENCH_BASELINE")
page = pathlib.Path("browser/tests/canvasbench/bench.html").resolve().as_uri()
if len(sys.argv) > 1:
    page += "#" + ",".join(sys.argv[1:])
def once(exe):
    result = subprocess.run([exe, "dump", "--text", "--js", page], capture_output=True, text=True, timeout=600)
    rows = {}
    for line in result.stderr.splitlines():
        parts = line.split("\t")
        if len(parts) == 5 and parts[0].endswith("canvasbench"):
            rows[parts[1]] = (int(parts[2]), float(parts[3]), parts[4])
    if result.returncode or not rows:
        raise SystemExit(f"{exe}: failed: {result.stderr[-2000:]}")
    return rows
exes = [binary] + ([baseline] if baseline else [])
samples = {exe: [] for exe in exes}
for i in range(runs):
    order = exes if i % 2 == 0 else exes[::-1]
    for exe in order:
        samples[exe].append(once(exe))
def summary(exe):
    out = {}
    for name, (calls, _, digest) in samples[exe][0].items():
        if any(s[name][2] != digest for s in samples[exe]):
            raise SystemExit(f"{exe}: {name}: pixels differ between runs")
        out[name] = (calls, statistics.median(s[name][1] for s in samples[exe]), digest)
    return out
after = summary(binary)
if baseline:
    before = summary(baseline)
    print("workload\tcalls\tbaseline_ms\tms\tspeedup\thash")
    for name, (calls, ms, digest) in after.items():
        b = before[name]
        if b[2] != digest:
            raise SystemExit(f"{name}: pixels differ from the baseline ({b[2]} vs {digest})")
        print(f"{name}\t{calls}\t{b[1]:.0f}\t{ms:.0f}\t{b[1] / max(ms, 1.0):.1f}\t{digest}")
else:
    print("workload\tcalls\tms\tms_per_call\thash")
    for name, (calls, ms, digest) in after.items():
        print(f"{name}\t{calls}\t{ms:.0f}\t{ms / calls:.3f}\t{digest}")
PY
