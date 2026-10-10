# Aye AI-writability eval

- Date: 2026-10-10; model: `sonnet` (`claude -p`, at most 30 turns); runs per task: 3.
- aye: `/Users/solemnis/Documents/ayelang/aye/target/release/aye`, from commit `c96dc27` of its checkout, built 2026-10-10 17:07.
- Each run: a fresh agent in an empty temp dir, tools Write, Edit, Read and `Bash(aye *)`; it learns Aye from `aye spec`, `aye check`, `aye explain`. The task's `tests/eval/<task>/check.sh` judges its `main.aye`.
- Rerun: `python3 scripts/eval.py --runs 1` (`--tasks a,b`, `--aye <binary>`); `--self-test` checks the tasks. Transcripts: `target/eval/20261010-170713/` (not in git).

| Task | Run | Pass | `aye check` calls | Turns | Input tokens | Output tokens | Cost (USD) | Wall (s) | Refused calls | std modules | End |
|---|---|---|---|---|---|---|---|---|---|---|---|
| cli_grep | 1 | yes | 2 | 18 | 148,195 | 2,112 | 0.10 | 43 | 1 | std.fs std.heap std.mem std.os std.text | success |
| cli_grep | 2 | yes | 3 | 19 | 208,746 | 2,343 | 0.11 | 53 | 1 | std.fs std.heap std.os | success |
| cli_grep | 3 | yes | 1 | 9 | 182,843 | 2,117 | 0.16 | 31 | 1 | std.fs std.heap std.log std.mem std.os | success |
| file_lines | 1 | yes | 1 | 12 | 255,940 | 3,154 | 0.18 | 41 | 1 | std.fs std.heap std.mem std.os std.text | success |
| file_lines | 2 | yes | 1 | 12 | 209,107 | 2,940 | 0.17 | 38 | 1 | std.fs std.heap std.log std.mem std.os std.text | success |
| file_lines | 3 | yes | 1 | 11 | 218,441 | 2,744 | 0.15 | 37 | 1 | std.fs std.heap std.log std.mem std.os std.text | success |
| http_hello | 1 | yes | 4 | 33 | 304,452 | 5,635 | 0.15 | 77 | 1 | std.heap std.mem std.net std.os | success |
| http_hello | 2 | yes | 1 | 12 | 271,063 | 4,476 | 0.20 | 47 | 1 | std.heap std.mem std.net std.os | success |
| http_hello | 3 | yes | 1 | 11 | 275,096 | 2,905 | 0.21 | 43 | 1 | std.heap std.http std.log std.mem std.os std.text | success |
| json_total | 1 | yes | 1 | 10 | 227,170 | 2,156 | 0.15 | 34 | 0 | std.fs std.heap std.json std.log std.mem std.os | success |
| json_total | 2 | yes | 1 | 23 | 214,172 | 3,272 | 0.11 | 55 | 1 | std.fs std.heap std.json std.mem std.os std.text | success |
| json_total | 3 | yes | 1 | 11 | 267,505 | 2,492 | 0.16 | 38 | 1 | std.fs std.heap std.json std.mem std.os std.text | success |
| word_freq | 1 | yes | 1 | 12 | 312,841 | 3,329 | 0.24 | 43 | 2 | std.fs std.heap std.list std.map std.mem std.sort | success |
| word_freq | 2 | yes | 0 | 12 | 163,003 | 2,877 | 0.15 | 32 | 2 | std.fs std.heap std.list std.map std.mem std.sort | success |
| word_freq | 3 | yes | 1 | 11 | 151,158 | 1,904 | 0.13 | 28 | 0 | std.fs std.heap std.list std.map std.mem std.sort | success |

Passed 15 of 15 runs.

## Findings (2026-10-10, 3 runs per task, `main` c96dc27)

15 of 15 runs passed; median 12 turns and 1 `aye check` call per run
(2026-10-08: 7-15 turns, 1-3 checks). What still costs turns, by
transcript (`target/eval/20261010-170713/`):

- **The spec core does not reach the agent.** `aye spec` is 38.6 KB;
  Claude Code's Bash tool shows at most about 30 KB inline and saved the
  rest to a file ("Output too large (38.6KB). Full output saved to
  ...tool-results/..."). Every run then tried to `Read` that file, which
  `--restricted` refuses (that is the "Refused calls" column, 1 per run),
  and fell back to `aye spec all` (7 runs), `aye spec --help` (5),
  `aye spec <name>` (`std.fs` 8, `std.os` 7, `std.text` 4, ...) and
  `aye doc --signatures` (8 calls in one run). §15 and §18 sit at the end
  of the core, so the std index and the pitfalls were the part most often
  not seen. The core must fit the inline limit: about 28 KB (O1.13a).
- **`std.http` reached 1 of 3 http_hello runs;** the other two hand-wrote
  HTTP on `std.net`, for the reason above. `std.json.parse` was used in
  3 of 3 json_total runs (2026-10-08: hand-written parser, 8.8k output
  tokens; now 2.2-3.3k). file_lines used `std.fs` and `std.os.args` in
  3 of 3.
- **The O1.12 hints work:** E0022 (`@println(a, b)`) in 2 runs and E0006
  `and` in 2 runs were each fixed in the next edit. No `or`/`not` cases.
- **Guessed names:** `std.io` (3 runs), `std.str`, `std.fmt`, `std.parse`,
  `std.string` (one run), spec sections `net`, `strings`, `arrays`,
  `stdlib`. The error for an unknown std module could list the modules.
- **`;` terminators** (E0002) cost http_hello run 1 about 12 turns of
  one-line edits (33 turns in all). `aye fix` could offer removing them.
- **Harness:** 11 of the 15 refused calls were writes to a mistyped copy of
  the long temp path (`_T` for `/T`). A short working directory
  (`/tmp/aye-eval/<task>-<n>`) would remove that noise (O1.11a).
