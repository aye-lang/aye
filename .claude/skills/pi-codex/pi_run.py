#!/usr/bin/env python3
"""Run one pi (GPT via Codex OAuth) task under a watchdog.

pi runs non-interactively (-p --mode json, stdin closed). Its event stream
is logged, a compact progress log and a status file are kept up to date,
and the run is killed and resumed when it stops making progress:

  - no event for --idle seconds while the model is thinking or writing,
  - no event for --tool-idle seconds while a tool (bash) is running,
  - more than --wall seconds in total.

A stalled or failed run is resumed in the same pi session (up to --retries
times) with a note saying why it was stopped. The last line on stdout is a
JSON summary; the full final answer is in <logdir>/result.md.

Exit codes: 0 done, 2 failed (API errors after retries), 124 stalled.
"""
import argparse, json, os, signal, subprocess, sys, threading, time

DEFAULT_MODEL = "openai/gpt-6.1-sol"

RULES = """\
You run unattended inside a watchdog. Nobody can answer questions.
- Never ask the user anything; decide and continue. If truly blocked,
  write the final report explaining the blocker and stop.
- Never run interactive commands (editors, pagers, `git rebase -i`,
  `git add -p`, REPLs, servers in the foreground, `less`, `top`).
- Give every build or test command the bash tool's `timeout` argument
  (for example 1800 for a full test suite). Commands that print nothing
  for a long time look like a hang and get your run killed.
- Run long-lived processes (servers) in the background with `&` and kill
  them when done.
- Build the browser with `browser/build.sh` and no output path: it
  writes target/anyknown inside your worktree. Never build to a path
  outside the worktree (`/tmp`, a scratchpad) or to a fixed shared name;
  another agent's build can replace your binary before your tests run.
- Commit your work in small logical commits as you go, so nothing is
  lost if the run is stopped.
- Before coding, read docs/practices/ (README.md and the file for your
  area). When you learn a lesson that will come up again, add an entry
  there in the same commit as the fix.
- End with a final report (at most 300 words): commits (hash + one line),
  test results, what remains.
"""

NONINTERACTIVE_ENV = {
    "GIT_EDITOR": "true", "EDITOR": "true", "VISUAL": "true",
    "GIT_TERMINAL_PROMPT": "0", "GIT_PAGER": "cat", "PAGER": "cat",
    "CI": "1", "HOMEBREW_NO_AUTO_UPDATE": "1", "NO_COLOR": "1",
}


def now():
    return time.time()


def stamp():
    return time.strftime("%H:%M:%S")


class Run:
    def __init__(self, args, logdir):
        self.args = args
        self.logdir = logdir
        self.session = args.resume_session
        self.last_event = now()
        self.tool = None          # name of the running tool, if any
        self.tool_cmd = ""
        self.turns = 0
        self.tools = 0
        self.final_text = ""
        self.api_error = ""
        self.started = now()
        self.status = "starting"
        self.lock = threading.Lock()
        self.events = open(os.path.join(logdir, "events.jsonl"), "a")
        self.progress = open(os.path.join(logdir, "progress.log"), "a")

    def log(self, line):
        self.progress.write(f"{stamp()} {line}\n")
        self.progress.flush()

    def write_status(self, **extra):
        st = {
            "status": self.status, "session": self.session,
            "turns": self.turns, "tools": self.tools,
            "running_tool": self.tool, "tool_cmd": self.tool_cmd[:200],
            "idle_s": round(now() - self.last_event),
            "elapsed_s": round(now() - self.started),
            "logdir": self.logdir,
        }
        st.update(extra)
        tmp = os.path.join(self.logdir, "status.json.tmp")
        with open(tmp, "w") as f:
            json.dump(st, f, indent=1)
        os.replace(tmp, os.path.join(self.logdir, "status.json"))

    def on_event(self, e):
        t = e.get("type")
        with self.lock:
            self.last_event = now()
        if t == "session":
            self.session = e.get("id") or self.session
            self.log(f"session {self.session}")
        elif t == "turn_start":
            self.turns += 1
        elif t == "tool_execution_start":
            self.tools += 1
            self.tool = e.get("toolName")
            a = e.get("args") or {}
            self.tool_cmd = a.get("command") or a.get("path") or json.dumps(a)[:200]
            self.log(f"tool {self.tool}: {self.tool_cmd[:160]!r}")
        elif t == "tool_execution_end":
            self.tool = None
            self.tool_cmd = ""
        elif t == "message_end":
            m = e.get("message") or {}
            if m.get("role") == "assistant":
                if m.get("stopReason") == "error" or m.get("errorMessage"):
                    self.api_error = str(m.get("errorMessage") or "error")[:500]
                    self.log(f"API ERROR {self.api_error}")
                texts = [c.get("text", "") for c in m.get("content") or []
                         if c.get("type") == "text"]
                text = "\n".join(x for x in texts if x).strip()
                if text:
                    self.final_text = text
                    self.log("say " + text[:200].replace("\n", " "))

    def attempt(self, prompt):
        cmd = ["pi", "-p", "--mode", "json", "--model", self.args.model,
               "--thinking", self.args.thinking, "-na",
               "--session-dir", os.path.join(self.logdir, "sessions"),
               "--append-system-prompt", RULES]
        if self.session:
            cmd += ["--session", self.session]
        else:
            cmd += ["--name", self.args.name]
        cmd += ["--", prompt]
        env = dict(os.environ, **NONINTERACTIVE_ENV)
        err = open(os.path.join(self.logdir, "stderr.log"), "a")
        p = subprocess.Popen(cmd, cwd=self.args.cwd, env=env,
                             stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                             stderr=err, start_new_session=True, text=True,
                             bufsize=1)
        self.status = "running"
        self.api_error = ""
        self.last_event = now()
        killed = [None]

        def watchdog():
            while p.poll() is None:
                time.sleep(5)
                idle = now() - self.last_event
                limit = self.args.tool_idle if self.tool else self.args.idle
                reason = None
                if idle > limit:
                    what = f"tool {self.tool} ({self.tool_cmd[:120]!r})" if self.tool else "the model"
                    reason = f"no progress for {int(idle)} s while waiting on {what}"
                elif now() - self.started > self.args.wall:
                    reason = f"wall-clock limit of {self.args.wall} s reached"
                self.write_status()
                if reason:
                    killed[0] = reason
                    self.log("KILL " + reason)
                    try:
                        os.killpg(p.pid, signal.SIGTERM)
                        time.sleep(10)
                        os.killpg(p.pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                    return

        threading.Thread(target=watchdog, daemon=True).start()
        for line in p.stdout:
            self.events.write(line)
            self.events.flush()
            try:
                self.on_event(json.loads(line))
            except json.JSONDecodeError:
                pass
        p.wait()
        err.close()
        self.tool = None
        return p.returncode, killed[0]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--cwd", required=True, help="directory pi works in (a git worktree)")
    ap.add_argument("--task-file", required=True, help="file holding the task prompt")
    ap.add_argument("--name", required=True, help="short run name (log dir, session name)")
    ap.add_argument("--model", default=DEFAULT_MODEL)
    ap.add_argument("--thinking", default="high")
    ap.add_argument("--idle", type=int, default=900)
    ap.add_argument("--tool-idle", type=int, default=2400)
    ap.add_argument("--wall", type=int, default=6 * 3600)
    ap.add_argument("--retries", type=int, default=3)
    ap.add_argument("--resume-session", default=None)
    ap.add_argument("--logroot", default=os.path.expanduser("~/.cache/pi-runs"))
    args = ap.parse_args()

    logdir = os.path.join(args.logroot, args.name)
    os.makedirs(logdir, exist_ok=True)
    prompt = open(args.task_file).read()
    run = Run(args, logdir)
    run.log(f"start model={args.model} cwd={args.cwd}")

    tries = 0
    code, why = run.attempt(prompt)
    while True:
        if why is None and code == 0 and not run.api_error:
            run.status = "done"
            break
        if tries >= args.retries or now() - run.started > args.wall:
            run.status = "stalled" if why else "failed"
            break
        tries += 1
        if why:
            note = (f"Your previous run was stopped by the watchdog: {why}. "
                    "Do not rerun a command that hangs; give long commands the bash "
                    "`timeout` argument. Check `git status` and continue the task "
                    "from where you stopped.")
        else:
            wait = 60 * tries * tries
            run.log(f"retry in {wait} s after exit={code} error={run.api_error!r}")
            time.sleep(wait)
            note = ("Your previous run ended with an error "
                    f"({run.api_error or 'exit ' + str(code)}). "
                    "Check `git status` and continue the task from where you stopped.")
        if not run.session:
            run.status = "failed"
            break
        run.log(f"resume #{tries}")
        code, why = run.attempt(note)

    with open(os.path.join(logdir, "result.md"), "w") as f:
        f.write(run.final_text + "\n")
    run.write_status(exit_code=code, retries_used=tries)
    run.log(f"end status={run.status}")
    print(json.dumps({
        "status": run.status, "session": run.session, "logdir": logdir,
        "retries": tries, "elapsed_min": round((now() - run.started) / 60, 1),
        "result_tail": run.final_text[-1500:],
    }, ensure_ascii=False))
    sys.exit({"done": 0, "stalled": 124}.get(run.status, 2))


if __name__ == "__main__":
    main()
