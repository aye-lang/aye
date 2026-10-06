#!/usr/bin/env python3
"""Capture exact browser responses, replay offline, and summarize phase traces."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import statistics
import subprocess
import time

SITES = {
    "zol": "https://www.zol.com.cn/",
    "wikipedia": "https://ja.wikipedia.org/wiki/%E3%82%A6%E3%82%A7%E3%83%96%E3%83%96%E3%83%A9%E3%82%A6%E3%82%B6",
    "github": "https://github.com/rust-lang/rust",
    "youtube": "https://www.youtube.com/",
}
PHASES = ("fetch", "decode", "html", "scripts", "timers", "sheets", "css_parse",
          "cascade_read", "cascade", "images", "image_decode", "layout", "paint_build", "paint", "markdown",
          "requests", "prefetch")
TRACE = re.compile(r"ANYKNOWN_TIMING\t([^\t]+)\t(\d+)\t(.*)")


def run(binary, url, directory, record=False, sample=None, screenshot=False):
    env = dict(os.environ, ANYKNOWN_TIMING="1", ANYKNOWN_BENCHMARK="1")
    env.pop("ANYKNOWN_COOKIE_FILE", None)
    env.pop("ANYKNOWN_RECORD", None)
    env.pop("ANYKNOWN_REPLAY", None)
    env["ANYKNOWN_RECORD" if record else "ANYKNOWN_REPLAY"] = str(directory.resolve())
    command = [str(binary.resolve()), "dump", "--md", "--js", url]
    if screenshot:
        command = [str(binary.resolve()), "screenshot", url, "-o",
                   str(directory / "paint.png"), "--font", "fixed"]
    start = time.perf_counter()
    # Files rather than pipes: sample must not block a verbose child.
    with (directory / "last.stdout").open("wb") as out, (directory / "last.stderr").open("wb") as err:
        process = subprocess.Popen(command, env=env, stdout=out, stderr=err)
        sampler = None
        try:
            if sample:
                sampler = subprocess.Popen(["sample", str(process.pid), "5", "1", "-file", str(sample)],
                                           stdout=subprocess.DEVNULL)
            status = process.wait(timeout=300)
            wall = time.perf_counter() - start
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
            if sampler:
                try:
                    sampler.wait(timeout=30)
                except subprocess.TimeoutExpired:
                    sampler.kill()
                    sampler.wait()
    stdout = (directory / "last.stdout").read_bytes()
    stderr = (directory / "last.stderr").read_text(errors="replace")
    if status or "archive miss:" in stderr or "cannot record archive:" in stderr:
        problems = "\n".join(line for line in stderr.splitlines() if "archive miss:" in line or "cannot record archive:" in line)
        raise RuntimeError(f"{url}: exit {status}\n{problems}\n{stderr[-4000:]}")
    phases = {}
    scripts = []
    for match in TRACE.finditer(stderr):
        phase, ns, label = match.groups()
        seconds = int(ns) / 1e9
        if phase == "script":
            scripts.append({"seconds": seconds, "source": label})
        else:
            phases[phase] = phases.get(phase, 0.0) + seconds
    if screenshot:
        stdout = (directory / "paint.png").read_bytes()
    return {"wall": wall, "phases": phases, "sha256": hashlib.sha256(stdout).hexdigest(),
            "bytes": len(stdout), "top_scripts": sorted(scripts, key=lambda x: -x["seconds"])[:5]}


def summarize(runs):
    return {"wall": statistics.median(r["wall"] for r in runs),
            "phases": {p: statistics.median(r["phases"].get(p, 0) for r in runs) for p in PHASES},
            "sha256": runs[0]["sha256"], "bytes": runs[0]["bytes"],
            "top_scripts": min(runs, key=lambda r: abs(r["wall"] - statistics.median(x["wall"] for x in runs)))["top_scripts"],
            "runs": runs}


def print_result(name, result):
    print(f"{name:<17} {result['wall']:6.3f} " + " ".join(f"{result['phases'].get(p, 0):12.3f}" for p in PHASES))
    for script in result["top_scripts"]:
        print(f"  script {script['seconds']:.3f}s {script['source']}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("capture", "run"))
    parser.add_argument("--archive", type=Path, default=Path("target/page-load"))
    parser.add_argument("--binary", type=Path, default=Path("target/anyknown"))
    parser.add_argument("--compare", type=Path, help="baseline binary; alternate before/after to reduce machine-load bias")
    parser.add_argument("--site", action="append", choices=SITES)
    parser.add_argument("--repeat", type=int, default=3)
    parser.add_argument("--warmup", type=int, default=1, help="unmeasured loads per binary/site before replay runs")
    parser.add_argument("--json", type=Path)
    parser.add_argument("--sample", action="store_true", help="macOS sample of the first replay")
    parser.add_argument("--screenshot", action="store_true", help="also measure paint (separate output hash)")
    args = parser.parse_args()
    if args.repeat < 1 or args.warmup < 0:
        parser.error("--repeat must be positive and --warmup nonnegative")
    if args.mode == "capture" and (args.compare or args.screenshot or args.sample):
        parser.error("capture always records Markdown; compare, screenshot and sample are replay options")
    results = {}
    for name in args.site or SITES:
        directory = args.archive / name
        directory.mkdir(parents=True, exist_ok=True, mode=0o700)
        manifest_path = directory / "manifest.json"
        if args.mode == "capture":
            if manifest_path.exists() or list(directory.glob("*.response")):
                raise RuntimeError(f"{directory}: use a new directory for an immutable capture")
            result = run(args.binary, SITES[name], directory, record=True)
            resources = []
            for path in sorted(directory.glob("*.response")):
                data = path.read_bytes()
                fields = data.split(b"\n", 6)
                resources.append({"file": path.name, "url": fields[0].decode(),
                                  "status": int(fields[1]), "bytes": len(fields[6]),
                                  "sha256": hashlib.sha256(data).hexdigest()})
            manifest_path.write_text(json.dumps({"url": SITES[name], "capture": result,
                                                "resources": resources}, indent=2) + "\n")
            results[name] = result
            print(f"{name}: captured {len(resources)} requests, {result['wall']:.3f}s", flush=True)
            continue
        manifest = json.loads(manifest_path.read_text())
        for resource in manifest["resources"]:
            if hashlib.sha256((directory / resource["file"]).read_bytes()).hexdigest() != resource["sha256"]:
                raise RuntimeError(f"{directory}: changed resource {resource['file']}")
        binaries = {"before": args.compare, "after": args.binary} if args.compare else {"after": args.binary}
        runs = {label: [] for label in binaries}
        for label, binary in binaries.items():
            for _ in range(args.warmup):
                result = run(binary, manifest["url"], directory, screenshot=args.screenshot)
                print(f"{name} {label}: warmup: {result['wall']:.3f}s", flush=True)
        for i in range(args.repeat):
            order = list(binaries)
            if i % 2:
                order.reverse()
            for label in order:
                sample = directory / f"sample-{label}.txt" if args.sample and i == 0 else None
                result = run(binaries[label], manifest["url"], directory, sample=sample, screenshot=args.screenshot)
                runs[label].append(result)
                print(f"{name} {label}: run {i + 1}/{args.repeat}: {result['wall']:.3f}s", flush=True)
        all_runs = [r for batch in runs.values() for r in batch]
        if not args.screenshot and any(r["sha256"] != manifest["capture"]["sha256"] for r in all_runs):
            raise RuntimeError(f"{name}: replay Markdown differs from capture")
        if len({r["sha256"] for r in all_runs}) != 1:
            raise RuntimeError(f"{name}: output is not reproducible")
        summaries = {label: summarize(batch) for label, batch in runs.items()}
        results[name] = summaries if args.compare else summaries["after"]
    print("\nSeconds (medians); nested phases overlap, do not sum columns:")
    print("site                wall " + " ".join(f"{p:>12}" for p in PHASES))
    for name, result in results.items():
        if args.compare:
            for label, summary in result.items():
                print_result(f"{name} {label}", summary)
            print(f"  wall improvement: {1 - result['after']['wall'] / result['before']['wall']:.1%}")
        else:
            print_result(name, result)
    if args.json:
        args.json.parent.mkdir(parents=True, exist_ok=True)
        args.json.write_text(json.dumps(results, indent=2) + "\n")


if __name__ == "__main__":
    main()
