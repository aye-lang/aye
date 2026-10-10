# Aye AI-writability eval

- Date: 2026-10-08; model: `sonnet` (`claude -p`, at most 30 turns); runs per task: 1.
- aye: `/Users/solemnis/Documents/ayelang/aye/target/release/aye`, from commit `3cff579` of its checkout, built 2026-10-08 15:12.
- Each run: a fresh agent in an empty temp dir, tools Write, Edit, Read and `Bash(aye *)`; it learns Aye from `aye spec`, `aye check`, `aye explain`. The task's `tests/eval/<task>/check.sh` judges its `main.aye`.
- Rerun: `python3 scripts/eval.py --runs 1` (`--tasks a,b`, `--aye <binary>`); `--self-test` checks the tasks. Transcripts: `target/eval/20261008-160024/` (not in git).

| Task | Run | Pass | `aye check` calls | Turns | Input tokens | Output tokens | Cost (USD) | Wall (s) | Refused calls | End |
|---|---|---|---|---|---|---|---|---|---|---|
| cli_grep | 1 | yes | 3 | 13 | 136,657 | 2,649 | 0.09 | 66 | 0 | success |
| file_lines | 1 | yes | 3 | 15 | 251,586 | 2,354 | 0.19 | 64 | 0 | success |
| http_hello | 1 | yes | 1 | 7 | 43,559 | 2,356 | 0.06 | 43 | 0 | success |
| json_total | 1 | yes | 1 | 9 | 260,602 | 8,766 | 0.34 | 96 | 0 | success |
| word_freq | 1 | yes | 1 | 9 | 182,740 | 2,011 | 0.18 | 49 | 0 | success |

Passed 5 of 5 runs.

## Findings

All five passed; the only lost rounds were guesses from other languages
that `aye check` caught but did not answer:

- `@println(n, ":", line)` (Go/Python habit): E0022 "`@println` takes at
  most one argument" cost a round in cli_grep and file_lines (and in the
  dev run). Neither the message nor §18 says to use `@print` per value or
  a `std.text.Builder`.
- `while i < n and s[i] != c` (cli_grep): E0006 "expected `{`, found
  identifier `and`" names no fix; it could say "use `&&`".
- `aye spec` (81.5 KB) is too long for Claude Code's inline tool output: every
  agent read it from the saved file in chunks or by grep. That re-read
  context is most of the 44k-261k input tokens per task.
- http_hello never found `std.http`: §15 lists it only as "aye doc
  --signatures std.http", so the agent hand-wrote HTTP on `std.net`.
  file_lines guessed `std.process` (missing, O1.6). json_total wrote a
  full JSON parser by hand in one round: the cost of the missing std.json
  is effort and tokens (8.8k output, the most of any task), not failures.
- Agents tested little: cli_grep never ran its program, and nothing can
  run the server. One run per task cannot show a pass rate; rerun with
  `--runs 3` before ranking fixes by it.
