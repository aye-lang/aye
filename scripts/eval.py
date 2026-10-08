#!/usr/bin/env python3
"""AI-writability eval (O1.11).

Runs fresh headless Claude agents on the fixed tasks in tests/eval/<task>/.
Each agent gets an empty temp dir, the task's prompt.md, and a Bash tool
that may run only `aye` commands; it learns Aye from `aye spec`,
`aye check` and `aye explain`. The task's check.sh then builds and tests
the agent's main.aye.

  python3 scripts/eval.py --self-test     every solution.aye passes its
                                          check.sh; every check.sh rejects
                                          an empty program
  python3 scripts/eval.py --runs 1        run the agents, write the table to
                                          docs/aye-eval.md, transcripts to
                                          target/eval/<stamp>/

Exit status: 0 when the self-test passes, or when every agent run ended
with a result event (a task may still fail); 1 otherwise.
"""

import argparse
import concurrent.futures
import datetime
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TASKS_DIR = os.path.join(ROOT, "tests", "eval")
DOC = os.path.join(ROOT, "docs", "aye-eval.md")
FINDINGS = "## Findings"
EMPTY_PROGRAM = "pub fn main() {\n}\n"

SYSTEM_PROMPT = (
    "You are writing a program in Aye, a new compiled programming language "
    "that you have not seen before. Learn Aye only from the `aye` command: "
    "`aye spec` prints the language reference, `aye check <file>` reports "
    "compile errors, and `aye explain <code>` explains an error code. Do not "
    "look for Aye sources, examples or docs anywhere else."
)

PROMPT_FOOTER = """
Work in the current directory and write the program to `main.aye`. In Bash
you can run only `aye` commands, for example `aye spec`, `aye check
main.aye`, `aye explain E0023`, `aye run main.aye -- <args>` and
`aye run main.aye < input.txt`. Each Bash call must be one `aye` command:
`&&`, `;`, pipes and other commands are refused, so to give a program
input, Write a file and redirect it. Stop when `aye check main.aye` reports
no errors and you believe the program meets every requirement.
"""


def tasks(selected):
    names = sorted(
        d for d in os.listdir(TASKS_DIR)
        if os.path.isfile(os.path.join(TASKS_DIR, d, "check.sh"))
    )
    if selected:
        unknown = set(selected) - set(names)
        if unknown:
            sys.exit(f"unknown task(s): {', '.join(sorted(unknown))}")
        names = [n for n in names if n in selected]
    return names


def env_with(aye):
    env = dict(os.environ)
    env["PATH"] = os.path.dirname(aye) + os.pathsep + env.get("PATH", "")
    return env


def run_check(task, workdir, env):
    """Runs the task's check.sh on workdir; returns (passed, output)."""
    check = os.path.join(TASKS_DIR, task, "check.sh")
    try:
        p = subprocess.run(
            ["bash", check, workdir], env=env, capture_output=True,
            text=True, timeout=300,
        )
    except subprocess.TimeoutExpired:
        return False, "check.sh timed out\n"
    return p.returncode == 0, p.stdout + p.stderr


def self_test(names, env):
    ok = True
    for task in names:
        with tempfile.TemporaryDirectory(prefix=f"aye-eval-{task}-") as d:
            shutil.copy(os.path.join(TASKS_DIR, task, "solution.aye"),
                        os.path.join(d, "main.aye"))
            good, out = run_check(task, d, env)
            with open(os.path.join(d, "main.aye"), "w") as f:
                f.write(EMPTY_PROGRAM)
            empty, _ = run_check(task, d, env)
        status = "ok" if good and not empty else "FAIL"
        print(f"{status} {task}: solution {'passes' if good else 'FAILS'}, "
              f"empty program {'PASSES' if empty else 'rejected'}")
        if not good:
            print(out.rstrip())
        ok = ok and good and not empty
    print("self-test:", "ok" if ok else "FAILED")
    return 0 if ok else 1


AYE_CMD = re.compile(r"(?:^|[|;&(]\s*)aye\s+([a-z-]+)")


def parse_stream(path):
    """Summarizes a stream-json transcript. Bash calls that were refused
    (permission_denials) do not count as `aye` commands."""
    s = {"checks": 0, "cmds": {}, "turns": None, "in_tok": None,
         "out_tok": None, "cost": None, "end": "no result", "denied": 0}
    bash = {}
    denied = set()
    with open(path) as f:
        for line in f:
            try:
                e = json.loads(line)
            except json.JSONDecodeError:
                continue
            if e.get("type") == "assistant":
                for c in e.get("message", {}).get("content", []):
                    if c.get("type") == "tool_use" and c.get("name") == "Bash":
                        bash[c.get("id")] = c.get("input", {}).get("command", "")
            elif e.get("type") == "result":
                denied = {d.get("tool_use_id") for d in e.get("permission_denials") or []}
                u = e.get("usage") or {}
                s["turns"] = e.get("num_turns")
                s["in_tok"] = (u.get("input_tokens", 0)
                               + u.get("cache_creation_input_tokens", 0)
                               + u.get("cache_read_input_tokens", 0))
                s["out_tok"] = u.get("output_tokens")
                s["cost"] = e.get("total_cost_usd")
                s["end"] = e.get("subtype", "?")
                s["denied"] = len(denied)
    for id, command in bash.items():
        if id in denied:
            continue
        for sub in AYE_CMD.findall(command):
            s["cmds"][sub] = s["cmds"].get(sub, 0) + 1
            if sub == "check":
                s["checks"] += 1
    return s


def run_agent(task, run, args, env, outdir):
    prompt = open(os.path.join(TASKS_DIR, task, "prompt.md")).read() + PROMPT_FOOTER
    workdir = tempfile.mkdtemp(prefix=f"aye-eval-{task}-")
    base = os.path.join(outdir, f"{task}-{run}")
    cmd = [
        args.claude, "-p", prompt,
        "--model", args.model,
        "--output-format", "stream-json", "--verbose",
        "--max-turns", str(args.max_turns),
        "--tools", "Bash,Edit,Read,Write",
        "--allowedTools", "Write", "Edit", "Read", "Bash(aye *)",
        "--permission-mode", "dontAsk",
        "--safe-mode", "--strict-mcp-config", "--restricted",
        "--no-session-persistence",
        "--append-system-prompt", SYSTEM_PROMPT,
    ]
    start = time.time()
    with open(base + ".jsonl", "w") as out, open(base + ".err", "w") as err:
        try:
            subprocess.run(cmd, cwd=workdir, env=env, stdout=out, stderr=err,
                           stdin=subprocess.DEVNULL, timeout=args.timeout)
        except subprocess.TimeoutExpired:
            err.write(f"eval.py: killed after {args.timeout} s\n")
    wall = time.time() - start
    s = parse_stream(base + ".jsonl")
    main = os.path.join(workdir, "main.aye")
    if os.path.exists(main):
        shutil.copy(main, base + ".aye")
        passed, check_out = run_check(task, workdir, env)
    else:
        passed, check_out = False, "no main.aye\n"
    with open(base + ".check.txt", "w") as f:
        f.write(check_out)
    shutil.rmtree(workdir, ignore_errors=True)
    s.update(task=task, run=run, passed=passed, wall=wall)
    print(f"{task} run {run}: {'pass' if passed else 'FAIL'}, "
          f"{s['checks']} checks, {s['turns']} turns, {wall:.0f} s, "
          f"end {s['end']}", flush=True)
    return s


def git_rev(path):
    try:
        return subprocess.run(
            ["git", "-C", path, "rev-parse", "--short", "HEAD"],
            capture_output=True, text=True, check=True,
        ).stdout.strip()
    except (subprocess.CalledProcessError, OSError):
        return "unknown"


def fmt_int(v):
    return "–" if v is None else f"{v:,}"


def write_doc(results, args, outdir):
    aye_real = os.path.realpath(args.aye)
    built = datetime.datetime.fromtimestamp(os.path.getmtime(aye_real))
    rel_out = os.path.relpath(outdir, ROOT)
    lines = [
        "# Aye AI-writability eval",
        "",
        f"- Date: {datetime.date.today().isoformat()}; model: `{args.model}` "
        f"(`claude -p`, at most {args.max_turns} turns); runs per task: {args.runs}.",
        f"- aye: `{aye_real}`, from commit `{git_rev(os.path.dirname(aye_real))}` "
        f"of its checkout, built {built:%Y-%m-%d %H:%M}.",
        "- Each run: a fresh agent in an empty temp dir, tools Write, Edit, Read "
        "and `Bash(aye *)`; it learns Aye from `aye spec`, `aye check`, "
        "`aye explain`. The task's `tests/eval/<task>/check.sh` judges its "
        "`main.aye`.",
        "- Rerun: `python3 scripts/eval.py --runs 1` (`--tasks a,b`, "
        "`--aye <binary>`); `--self-test` checks the tasks. Transcripts: "
        f"`{rel_out}/` (not in git).",
        "",
        "| Task | Run | Pass | `aye check` calls | Turns | Input tokens | "
        "Output tokens | Cost (USD) | Wall (s) | Refused calls | End |",
        "|---|---|---|---|---|---|---|---|---|---|---|",
    ]
    for r in results:
        cost = "–" if r["cost"] is None else f"{r['cost']:.2f}"
        lines.append(
            f"| {r['task']} | {r['run']} | {'yes' if r['passed'] else 'no'} | "
            f"{r['checks']} | {fmt_int(r['turns'])} | {fmt_int(r['in_tok'])} | "
            f"{fmt_int(r['out_tok'])} | {cost} | {r['wall']:.0f} | {r['denied']} | {r['end']} |"
        )
    passed = sum(1 for r in results if r["passed"])
    lines += ["", f"Passed {passed} of {len(results)} runs.", ""]
    findings = f"{FINDINGS}\n\n(Not written yet: read the transcripts.)\n"
    if os.path.exists(DOC):
        old = open(DOC).read()
        i = old.find(FINDINGS)
        if i >= 0:
            findings = old[i:]
    with open(DOC, "w") as f:
        f.write("\n".join(lines) + findings)


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--self-test", action="store_true")
    ap.add_argument("--runs", type=int, default=1, help="runs per task")
    ap.add_argument("--tasks", default="", help="comma-separated task names")
    ap.add_argument("--aye", default=os.path.join(ROOT, "target", "release", "aye"))
    ap.add_argument("--claude", default=shutil.which("claude")
                    or os.path.expanduser("~/.local/bin/claude"))
    ap.add_argument("--model", default="sonnet")
    ap.add_argument("--max-turns", type=int, default=30)
    ap.add_argument("--timeout", type=int, default=1200, help="seconds per run")
    ap.add_argument("--jobs", type=int, default=5, help="runs at once")
    args = ap.parse_args()

    args.aye = os.path.abspath(args.aye)
    if not os.access(args.aye, os.X_OK):
        sys.exit(f"no aye binary at {args.aye} (build it, or pass --aye)")
    env = env_with(args.aye)
    names = tasks([t for t in args.tasks.split(",") if t])

    if args.self_test:
        return self_test(names, env)

    stamp = datetime.datetime.now().strftime("%Y%m%d-%H%M%S")
    outdir = os.path.join(ROOT, "target", "eval", stamp)
    os.makedirs(outdir)
    jobs = [(t, r) for t in names for r in range(1, args.runs + 1)]
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.jobs) as ex:
        futures = [ex.submit(run_agent, t, r, args, env, outdir) for t, r in jobs]
        results = [f.result() for f in futures]
    with open(os.path.join(outdir, "results.json"), "w") as f:
        json.dump(results, f, indent=1)
    write_doc(results, args, outdir)
    print(f"wrote {os.path.relpath(DOC, ROOT)}; transcripts in {outdir}")
    return 0 if all(r["end"] != "no result" for r in results) else 1


if __name__ == "__main__":
    sys.exit(main())
