---
name: pi-codex
description: Delegate an implementation task in this repo to GPT (openai/gpt-6.1-sol through pi with Codex OAuth) running unattended in its own git worktree, under a watchdog that kills and resumes it when it stops making progress. Use when a work stream can run in parallel with the Claude sub agents, or when a second model's take helps.
---

# pi-codex: GPT as a co-developer, without silent stalls

`pi_run.py` (next to this file) runs `pi -p --mode json` with stdin closed,
a non-interactive environment (`GIT_EDITOR=true`, `PAGER=cat`, ...) and
extra system rules (never ask, never run interactive commands, give long
commands a bash `timeout`, commit as you go, read and extend
`docs/practices/`, end with a report). A watchdog
thread kills the process group when:

| condition | default |
|---|---|
| no event while the model works (`--idle`) | 900 s |
| no event while a tool runs (`--tool-idle`) | 2400 s |
| total time (`--wall`) | 6 h |

A killed or failed run is resumed in the same pi session (`--retries`,
default 3) with a note saying why it was stopped. API errors back off
1, 4, 9 minutes. So a run either finishes, or exits within the limits
above with status `stalled`/`failed` — never hours of silence.

## Steps

1. **Worktree.** Ask the `git-runner` agent to create one from the
   integration branch:
   `git worktree add .claude/worktrees/pi-<name> -b pi-<name> main`.
2. **Task file.** Write the prompt to the scratchpad (`<scratch>/pi-<name>.md`).
   Include: repo orientation (browser/README.md, docs to read, build and
   test commands), the task with acceptance tests, files not to edit
   (docs/browser.md, docs/browser-sites.md), the commit message format
   ending with
   `Co-Authored-By: GPT-6.1-sol via pi <noreply@openai.com>`, and that all
   of `cargo test --test browser` and the full `cargo test` must pass.
   One work stream per run; keep it to what a strong engineer does in a
   few hours.
3. **Launch** with Bash `run_in_background: true` (you are notified when it
   exits; do not poll):
   ```
   python3 .claude/skills/pi-codex/pi_run.py \
     --cwd /Users/solemnis/Documents/anyknown-com/ore/.claude/worktrees/pi-<name> \
     --task-file <scratch>/pi-<name>.md --name <name>
   ```
4. **Progress on demand** (small, safe to read):
   `cat ~/.cache/pi-runs/<name>/status.json` and
   `tail -20 ~/.cache/pi-runs/<name>/progress.log`.
   Never read `events.jsonl` into the main context.
5. **On exit** the last stdout line is a JSON summary (`status`,
   `session`, `result_tail`); the full report is
   `~/.cache/pi-runs/<name>/result.md`.
   - `done`: verify like any sub agent's work (spot-check commits, have a
     merge agent merge and run the full tests).
   - `stalled`/`failed`: read `progress.log` tail, fix the cause (task too
     big, a hanging command, an outage), then resume with
     `--resume-session <session> --task-file <note.md>` or hand the
     worktree to a Claude agent to finish.

## Notes

- Model: `--model openai/gpt-6.1-sol` (default), `--thinking high`
  (default; `xhigh` for hard design work).
- Check auth first if a run fails at once:
  `pi auth check --model openai/gpt-6.1-sol`.
- Runs keep their pi sessions under `~/.cache/pi-runs/<name>/sessions`,
  so `pi --session-dir ... --session <id>` can reopen one interactively.
