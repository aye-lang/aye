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
          "cascade_read", "cascade", "images", "layout", "paint_build", "paint", "markdown")
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
        try:
            if sample:
                subprocess.run(["sample", str(process.pid), "5", "1", "-file", str(sample)],
                               check=False, timeout=30, stdout=subprocess.DEVNULL)
            status = process.wait(timeout=300)
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
    wall = time.perf_counter() - start
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


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("capture", "run"))
    parser.add_argument("--archive", type=Path, default=Path("target/page-load"))
    parser.add_argument("--binary", type=Path, default=Path("target/anyknown"))
    parser.add_argument("--site", action="append", choices=SITES)
    parser.add_argument("--repeat", type=int, default=3)
    parser.add_argument("--json", type=Path)
    parser.add_argument("--sample", action="store_true", help="macOS sample of the first replay")
    parser.add_argument("--screenshot", action="store_true", help="also measure paint (separate output hash)")
    args = parser.parse_args()
    if args.repeat < 1:
        parser.error("--repeat must be positive")
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
        runs = []
        for i in range(args.repeat):
            sample = directory / "sample.txt" if args.sample and i == 0 else None
            result = run(args.binary, manifest["url"], directory, sample=sample, screenshot=args.screenshot)
            runs.append(result)
            print(f"{name}: run {i + 1}/{args.repeat}: {result['wall']:.3f}s", flush=True)
        if not args.screenshot and any(r["sha256"] != manifest["capture"]["sha256"] for r in runs):
            raise RuntimeError(f"{name}: replay Markdown differs from capture")
        if len({r["sha256"] for r in runs}) != 1:
            raise RuntimeError(f"{name}: output is not reproducible")
        result = {"wall": statistics.median(r["wall"] for r in runs),
                  "phases": {p: statistics.median(r["phases"].get(p, 0) for r in runs) for p in PHASES},
                  "sha256": runs[0]["sha256"], "bytes": runs[0]["bytes"],
                  "top_scripts": runs[0]["top_scripts"], "runs": runs}
        results[name] = result
    print("\nSeconds (medians); nested phases overlap, do not sum columns:")
    print("site       wall " + " ".join(f"{p:>12}" for p in PHASES))
    for name, r in results.items():
        print(f"{name:<10} {r['wall']:5.3f} " + " ".join(f"{r['phases'].get(p, 0):12.3f}" for p in PHASES))
        for script in r["top_scripts"]:
            print(f"  script {script['seconds']:.3f}s {script['source']}")
    if args.json:
        args.json.parent.mkdir(parents=True, exist_ok=True)
        args.json.write_text(json.dumps(results, indent=2) + "\n")


if __name__ == "__main__":
    main()
