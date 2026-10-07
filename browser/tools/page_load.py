#!/usr/bin/env python3
"""Capture exact browser responses, replay offline, summarize phase traces, and check sites.json."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import statistics
import subprocess
import time

# The sites of docs/browser-sites.md, in its order. --site all loads every one.
SITES = {
    "example": "https://example.com/",
    "wikipedia-en": "https://en.wikipedia.org/wiki/Web_browser",
    "hn": "https://news.ycombinator.com/",
    "github": "https://github.com/rust-lang/rust",
    "mdn": "https://developer.mozilla.org/en-US/docs/Web/HTML/Element/details",
    "python-docs": "https://docs.python.org/3/tutorial/index.html",
    "rustdoc": "https://doc.rust-lang.org/std/vec/struct.Vec.html",
    "bbc": "https://www.bbc.com/news",
    "cnn-lite": "https://lite.cnn.com/",
    "reddit": "https://old.reddit.com/r/programming/",
    "stackoverflow": "https://stackoverflow.com/questions/11227809/why-is-processing-a-sorted-array-faster-than-processing-an-unsorted-array",
    "lobsters": "https://lobste.rs/",
    "go-docs": "https://go.dev/doc/effective_go",
    "duckduckgo": "https://html.duckduckgo.com/html/?q=ore+programming+language",
    "google": "https://www.google.com/",
    "wikipedia": "https://ja.wikipedia.org/wiki/%E3%82%A6%E3%82%A7%E3%83%96%E3%83%96%E3%83%A9%E3%82%A6%E3%82%B6",
    "abehiroshi": "http://abehiroshi.la.coocan.jp/",
    "zol": "https://www.zol.com.cn/",
    "youtube": "https://www.youtube.com/",
    "youtube-search": "https://www.youtube.com/results?search_query=rust+programming",
    "react": "https://react.dev/",
    "mistral": "https://mistral.ai/news/mistral-large-4/",
}
# Without --site: the four heavy pages of browser/engine/PERF.md.
DEFAULT_SITES = ("zol", "wikipedia", "github", "youtube")
PHASES = ("fetch", "decode", "html", "scripts", "timers", "sheets", "css_parse",
          "cascade_read", "cascade", "images", "image_decode", "layout", "paint_build", "paint", "markdown",
          "requests", "prefetch")
TRACE = re.compile(r"ANYKNOWN_TIMING\t([^\t]+)\t(\d+)\t(.*)")
EXPECTATIONS = Path(__file__).with_name("sites.json")
# Below this CPU/wall ratio the machine was too busy for wall time to mean anything.
BUSY_RATIO = 0.8


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
            # Only the browser is reaped here (the sampler later), so the delta is its CPU time.
            before = resource.getrusage(resource.RUSAGE_CHILDREN)
            status = process.wait(timeout=300)
            wall = time.perf_counter() - start
            after = resource.getrusage(resource.RUSAGE_CHILDREN)
            cpu = (after.ru_utime - before.ru_utime) + (after.ru_stime - before.ru_stime)
            load = os.getloadavg()[0]
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
    return {"wall": wall, "cpu": cpu, "load": load, "phases": phases, "sha256": hashlib.sha256(stdout).hexdigest(),
            "bytes": len(stdout), "top_scripts": sorted(scripts, key=lambda x: -x["seconds"])[:5]}


def summarize(runs):
    return {"wall": statistics.median(r["wall"] for r in runs),
            "cpu": statistics.median(r["cpu"] for r in runs),
            "load": statistics.median(r["load"] for r in runs),
            "phases": {p: statistics.median(r["phases"].get(p, 0) for r in runs) for p in PHASES},
            "sha256": runs[0]["sha256"], "bytes": runs[0]["bytes"],
            "top_scripts": min(runs, key=lambda r: abs(r["cpu"] - statistics.median(x["cpu"] for x in runs)))["top_scripts"],
            "runs": runs}


def busy_warning(name, result):
    """A warning line when CPU/wall < BUSY_RATIO, else None."""
    ratio = result["cpu"] / result["wall"] if result["wall"] else 1.0
    if ratio < BUSY_RATIO:
        return (f"  warning: {name}: cpu/wall {ratio:.2f} < {BUSY_RATIO} (load {result['load']:.1f}): "
                "the machine was too busy for wall time to mean anything; compare cpu")
    return None


def print_result(name, result):
    print(f"{name:<17} {result['wall']:6.3f} {result['cpu']:6.3f} {result['load']:5.1f} "
          + " ".join(f"{result['phases'].get(p, 0):12.3f}" for p in PHASES))
    for script in result["top_scripts"]:
        print(f"  script {script['seconds']:.3f}s {script['source']}")
    warning = busy_warning(name, result)
    if warning:
        print(warning)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("capture", "run", "check"),
                        help="check: replay each site against browser/tools/sites.json")
    parser.add_argument("--archive", type=Path, default=Path("target/sites"), help="one directory per site")
    parser.add_argument("--binary", type=Path, default=Path("target/anyknown"))
    parser.add_argument("--compare", type=Path, help="baseline binary; alternate before/after to reduce machine-load bias")
    parser.add_argument("--site", action="append",
                        help=f"repeatable or comma-separated, or all; default {', '.join(DEFAULT_SITES)} (check: all)")
    parser.add_argument("--repeat", type=int, default=3)
    parser.add_argument("--warmup", type=int, default=1, help="unmeasured loads per binary/site before replay runs")
    parser.add_argument("--json", type=Path)
    parser.add_argument("--sample", action="store_true", help="macOS sample of the first replay")
    parser.add_argument("--screenshot", action="store_true", help="benchmark the screenshot CLI instead (fixed font, separate output hash)")
    args = parser.parse_args()
    if args.repeat < 1 or args.warmup < 0:
        parser.error("--repeat must be positive and --warmup nonnegative")
    if args.mode == "capture" and (args.compare or args.screenshot or args.sample):
        parser.error("capture always records Markdown; compare, screenshot and sample are replay options")
    names = [name for value in args.site or () for name in value.split(",") if name]
    for name in names:
        if name != "all" and name not in SITES:
            parser.error(f"unknown site {name}; one of {', '.join(SITES)}, all")
    if "all" in names or (args.mode == "check" and not names):
        sites = list(SITES)
    else:
        sites = names or DEFAULT_SITES
    if args.mode == "check":
        return check(args, sites)
    results = {}
    failures = {}
    for name in sites:
        try:
            result = load_site(args, name)
        except (RuntimeError, OSError) as error:
            failures[name] = str(error).splitlines()[0]
            print(f"{name}: FAILED: {error}", flush=True)
            continue
        results[name] = result
    print("\nSeconds (medians; cpu = user+sys of the browser, load = 1-minute average);\n"
          "nested phases are wall time and overlap, do not sum columns:")
    print("site                wall    cpu  load " + " ".join(f"{p:>12}" for p in PHASES))
    for name in sites:
        if name in failures:
            print(f"{name:<17} FAILED {failures[name]}")
        elif args.compare:
            for label, summary in results[name].items():
                print_result(f"{name} {label}", summary)
            before, after = results[name]["before"], results[name]["after"]
            print(f"  cpu improvement: {1 - after['cpu'] / before['cpu']:.1%} "
                  f"(wall {1 - after['wall'] / before['wall']:.1%})")
        else:
            print_result(name, results[name])
    if args.json:
        args.json.parent.mkdir(parents=True, exist_ok=True)
        args.json.write_text(json.dumps(results, indent=2) + "\n")
    if failures:
        raise SystemExit(f"{len(failures)} of {len(sites)} sites failed: {', '.join(failures)}")


def console_errors(binary, url, directory):
    """The [error] console entries of an agent session that navigates to url on the replay."""
    env = dict(os.environ, ANYKNOWN_BENCHMARK="1", ANYKNOWN_REPLAY=str(directory.resolve()))
    env.pop("ANYKNOWN_COOKIE_FILE", None)
    env.pop("ANYKNOWN_RECORD", None)
    env.pop("ANYKNOWN_TIMING", None)
    request = json.dumps({"id": 1, "cmd": "navigate", "url": url}) + "\n"
    process = subprocess.run([str(binary.resolve()), "agent"], input=request.encode(),
                             capture_output=True, env=env, timeout=300)
    stderr = process.stderr.decode(errors="replace")
    misses = [line for line in stderr.splitlines() if "archive miss:" in line]
    if process.returncode or misses:
        raise RuntimeError(f"{url}: agent exit {process.returncode}\n" + "\n".join(misses or stderr.splitlines()[-5:]))
    errors = []
    for line in process.stdout.decode(errors="replace").splitlines():
        response = json.loads(line)
        if not response.get("ok"):
            raise RuntimeError(f"{url}: agent navigate: {response.get('error')}")
        errors += [entry for entry in response.get("console", []) if entry.startswith("[error]")]
    return errors


def check(args, sites):
    """Replays each site and compares it with sites.json; exits 1 when a judged site misses."""
    expectations = json.loads(EXPECTATIONS.read_text())["sites"]
    if set(expectations) != set(SITES):
        raise SystemExit(f"{EXPECTATIONS}: sites differ from SITES: {sorted(set(expectations) ^ set(SITES))}")
    print(f"{'site':<14} {'status':<9} {'errors':>6} {'wall':>7} {'cpu':>7} {'load':>5}  result  missing", flush=True)
    failed = []
    for name in sites:
        expect = expectations[name]
        judged = expect["status"] != "excluded"
        directory = args.archive / name
        try:
            url = json.loads((directory / "manifest.json").read_text())["url"]
            result = run(args.binary, url, directory)
            markdown = (directory / "last.stdout").read_text(errors="replace")
            errors = console_errors(args.binary, url, directory)
        except (RuntimeError, OSError, ValueError, subprocess.TimeoutExpired) as error:
            print(f"{name:<14} {expect['status']:<9} {'-':>6} {'-':>7} {'-':>7} {'-':>5}  "
                  f"{'FAIL' if judged else 'skip':<6}  {str(error).splitlines()[0]}", flush=True)
            if judged:
                failed.append(name)
            continue
        missing = [text for text in expect["md"] if text not in markdown]
        meets = not missing and len(errors) <= expect["max_errors"]
        verdict = ("pass" if meets else "FAIL") if judged else "skip"
        if verdict == "FAIL":
            failed.append(name)
        count = f"{len(errors)}/{expect['max_errors']}"
        print(f"{name:<14} {expect['status']:<9} {count:>6} {result['wall']:6.2f}s {result['cpu']:6.2f}s "
              f"{result['load']:5.1f}  {verdict:<6}  {', '.join(map(repr, missing)) or '-'}", flush=True)
        warning = busy_warning(name, result)
        if warning:
            print(warning, flush=True)
        if len(errors) > expect["max_errors"]:
            for entry in errors:
                print(f"  {entry.splitlines()[0][:200]}")
    if failed:
        raise SystemExit(f"{len(failed)} of {len(sites)} sites miss their expectations: {', '.join(failed)}")


def load_site(args, name):
    """Capture or replay one site; returns its result for the table."""
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
        print(f"{name}: captured {len(resources)} requests, {result['wall']:.3f}s wall, {result['cpu']:.3f}s cpu", flush=True)
        return result
    manifest = json.loads(manifest_path.read_text())
    for resource in manifest["resources"]:
        if hashlib.sha256((directory / resource["file"]).read_bytes()).hexdigest() != resource["sha256"]:
            raise RuntimeError(f"{directory}: changed resource {resource['file']}")
    binaries = {"before": args.compare, "after": args.binary} if args.compare else {"after": args.binary}
    runs = {label: [] for label in binaries}
    for label, binary in binaries.items():
        for _ in range(args.warmup):
            result = run(binary, manifest["url"], directory, screenshot=args.screenshot)
            print(f"{name} {label}: warmup: {result['wall']:.3f}s wall, {result['cpu']:.3f}s cpu", flush=True)
    for i in range(args.repeat):
        order = list(binaries)
        if i % 2:
            order.reverse()
        for label in order:
            sample = directory / f"sample-{label}.txt" if args.sample and i == 0 else None
            result = run(binaries[label], manifest["url"], directory, sample=sample, screenshot=args.screenshot)
            runs[label].append(result)
            print(f"{name} {label}: run {i + 1}/{args.repeat}: {result['wall']:.3f}s wall, "
                  f"{result['cpu']:.3f}s cpu, load {result['load']:.1f}", flush=True)
    all_runs = [r for batch in runs.values() for r in batch]
    if not args.screenshot and any(r["sha256"] != manifest["capture"]["sha256"] for r in all_runs):
        raise RuntimeError(f"{name}: replay Markdown differs from capture")
    if len({r["sha256"] for r in all_runs}) != 1:
        raise RuntimeError(f"{name}: output is not reproducible")
    summaries = {label: summarize(batch) for label, batch in runs.items()}
    return summaries if args.compare else summaries["after"]


if __name__ == "__main__":
    main()
