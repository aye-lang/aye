# Aye AI-writability eval

- Date: 2026-10-11; model: `sonnet` (`claude -p`, at most 30 turns); runs per task: 3.
- aye: `/Users/solemnis/Documents/ayelang/aye/target/release/aye`, from commit `0a3cca7` of its checkout, built 2026-10-11 16:55.
- Each run: a fresh agent in an empty short dir under `/tmp/aye-eval/`, tools Write, Edit, Read and `Bash(aye *)`; it learns Aye from `aye spec`, `aye check`, `aye explain`. The task's `tests/eval/<task>/check.sh` judges its `main.aye`.
- Rerun: `python3 scripts/eval.py --runs 1` (`--tasks a,b`, `--aye <binary>`); `--self-test` checks the tasks. Transcripts: `target/eval/20261011-165518/` (not in git).

| Task | Run | Pass | `aye check` calls | Turns | Input tokens | Output tokens | Cost (USD) | Wall (s) | Refused calls | std modules | End |
|---|---|---|---|---|---|---|---|---|---|---|---|
| cli_grep | 1 | yes | 2 | 12 | 145,811 | 1,516 | 0.11 | 33 | 0 | std.fs std.heap std.mem std.os std.text | success |
| cli_grep | 2 | yes | 2 | 11 | 144,975 | 1,574 | 0.11 | 32 | 0 | std.fs std.heap std.mem std.os std.text | success |
| cli_grep | 3 | yes | 1 | 11 | 125,060 | 1,524 | 0.11 | 30 | 0 | std.fs std.heap std.log std.mem std.os std.text | success |
| file_lines | 1 | yes | 2 | 17 | 220,271 | 2,655 | 0.14 | 46 | 0 | std.fs std.heap std.log std.mem std.os std.text | success |
| file_lines | 2 | yes | 1 | 11 | 146,489 | 1,972 | 0.12 | 33 | 0 | std.fs std.heap std.log std.mem std.os std.text | success |
| file_lines | 3 | yes | 1 | 18 | 215,905 | 2,872 | 0.12 | 52 | 0 | std.fs std.heap std.log std.mem std.os std.text | success |
| http_hello | 1 | yes | 2 | 8 | 91,792 | 1,518 | 0.10 | 25 | 0 | std.heap std.http std.log std.mem std.os std.text | success |
| http_hello | 2 | yes | 2 | 8 | 114,469 | 1,470 | 0.10 | 26 | 0 | std.heap std.http std.log std.mem std.os std.text | success |
| http_hello | 3 | yes | 2 | 8 | 114,544 | 1,541 | 0.10 | 27 | 0 | std.heap std.http std.log std.mem std.os std.text | success |
| json_total | 1 | yes | 2 | 13 | 279,988 | 9,033 | 0.22 | 86 | 0 | std.fs std.heap std.list std.mem std.os std.text | success |
| json_total | 2 | yes | 1 | 13 | 197,425 | 9,979 | 0.23 | 96 | 0 | std.fs std.heap std.list std.mem std.os std.text | success |
| json_total | 3 | yes | 1 | 10 | 165,639 | 2,432 | 0.12 | 44 | 0 | std.fs std.heap std.json std.mem std.os std.text | success |
| word_freq | 1 | yes | 1 | 9 | 84,997 | 1,662 | 0.09 | 32 | 0 | std.fs std.heap std.map std.sort | success |
| word_freq | 2 | yes | 0 | 8 | 85,062 | 1,702 | 0.09 | 31 | 0 | std.fs std.heap std.map std.mem std.sort | success |
| word_freq | 3 | yes | 1 | 9 | 84,997 | 1,614 | 0.09 | 35 | 0 | std.fs std.heap std.map std.sort | success |

Passed 15 of 15 runs.

## Findings (2026-10-11, 3 runs per task, `main` 0a3cca7)

15 of 15 runs passed; median 11 turns (mean 11.1, was 15.1) and 1 `aye check`
call per run (21 calls in all, same as 2026-10-10). Input tokens fell from
about 3.41M to 2.22M (-35%), cost from 2.47 to 1.85 USD, median wall time
33 s. Zero refused calls (was 15). Transcripts:
`target/eval/20261011-165518/`.

Median turns per task (2026-10-10 in brackets): cli_grep 11 (19),
file_lines 17 (12), http_hello 8 (12), json_total 13 (11), word_freq 9 (12).
file_lines and json_total did not improve; see below.

- **The core now arrives whole.** `aye spec` printed 26,951 bytes in all 15
  runs, no "Output too large", no `Read` of a saved file, no `aye spec all`,
  no `aye spec --help`. Reading was `aye spec` plus 2-4 named sections
  (`std.fs` 12 runs, `std.text` 12, `std.os` 9, `std.json` 3 and `std.http` 3
  via `aye doc --signatures`, `std.map`/`std.sort` 3 each, `std.mem` 2,
  `std.log` 2, `std.list` 2).
  Unknown section names: `aye spec cli` (1), `aye spec 14` (1); the first
  printed no module list that the agent acted on, it moved on.
- **`std.http` now reached 3 of 3 http_hello runs** (was 1 of 3); all three
  wrote the program in one Write and passed with 8 turns. No guessed std
  module names at all (was 6 runs), so O1.15 had nothing to do this time.
- **Compile errors almost vanished:** 1 error in 15 runs, E0006 `found
  return` (a `return` as a match-arm expression, json_total run 1), fixed in
  one edit. No E0002 (`;`), no E0022, no `and`/`or`; `aye fix` was used 0
  times and was not needed. The E0006/E0022 hints and `;` removal are
  therefore untested here, not disproved.
- **What still costs turns, ranked:**
  1. **`std.log.err` prefixes `error: `** (4 runs: cli_grep 1, 2; file_lines
     1, 3). The agents ran the usage/missing-file case, saw `error: usage:
     ...`, and rewrote to `std.fs.stderr().write` (file_lines 3 wrote a
     4-edit `eprint` helper: 17-18 turns, the two highest in the run). The
     checks accept the prefix, so this is polish the agents added, but it is
     6-9 turns per affected run.
  2. **Probing `aye check` before the program exists** (3 runs, http_hello
     1-3: `aye check main.aye` on a missing file, `aye check /dev/null`
     twice). Wasted checks, 1 turn each; they are why http_hello shows 2
     checks.
  3. **json_total hand-parses JSON in 2 of 3 runs.** All three read the
     `std.json` signatures, but runs 1 and 2 (9.0k and 10.0k output tokens,
     86-96 s) wrote their own parser (hex helpers, escapes) and never
     imported `std.json`; run 3 used it (2.4k tokens). 2026-10-10 was 3 of
     3 using `std.json`, so this is a regression; the signatures may not
     show how to get a decoded string out of a `Value`.
  4. **`return` in a match arm** (1 run, E0006 without a fix hint).
- **Harness:** the short `/tmp/aye-eval/<stamp>/<task>-<n>` dir (O1.11a)
  removed the mistyped-path refusals. Note the 2026-10-11 binary was built
  from `0a3cca7`; `main` moved while the run was in flight.

Proposed cards:

- O1.17 (done as O1.18a: `std.log.line`) `std.log.err` / stderr line helper: either document the `error: `
  prefix in the §15 `std.log` line, or add `std.fs.eprintln` (no prefix).
  Verify: rerun cli_grep and file_lines x3; zero `std.fs.stderr` rewrites.
- O1.18 E0006 on `return`/`break` in a match arm names the fix ("use a
  block: `_ => { return x }`", or allow it). Verify: `aye check` on a
  fixture shows the hint; one test in the diagnostics suite.
- O1.19 `aye check` on a missing file says "cannot read main.aye" plus
  "write it first; `aye spec` shows the entry point" (low value; skip if
  the probe stays 3 of 15). Verify: unit test of the message.
- O1.20 Find why runs 1 and 2 skipped `std.json` (read the transcripts'
  reasoning) and add a short `std.json` example (parse, get string/number)
  to the spec core or the §15 line. Verify: json_total x3 output tokens under 4k
  per run.

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
