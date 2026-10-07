# Workflow practices

How work gets from a worktree into `main`: commits, merges,
tests, several agents at once, and compiler gaps. Each entry comes from
a merge or a review in this repo's history.

## Commits

### Commit small and often
**Context:** any agent working in a worktree, Claude or GPT.
**Bad:** a session limit once stopped several agents at the same time,
in the middle of their work. What they had committed survived; their
uncommitted, half-done merges had to be read and finished by hand.
**Good:** one commit per logical step, tests passing. The pi watchdog
gives GPT the same rule (`.claude/skills/pi-codex/pi_run.py:33`):
```
- Commit your work in small logical commits as you go, so nothing is
  lost if the run is stopped.
```
When you pick up a worktree after a stop, run `git status` first: a
`MERGE_HEAD` means a merge was left half done.
**Why:** a commit survives the agent; a dirty tree has to be read and
finished by hand.

### Say how each conflict was resolved
**Context:** a merge that had conflicts.
**Bad:** a bare `Merge branch 'x'` message; the next reader cannot tell
which side won or why.
**Good:** list each conflict in the merge message, as d1c8800, 0e7d95d
and f68328a do:
```
Conflicts: browser/engine/sheets.ore keeps the net branch's base URL,
Referer and prefetch and the CSS branch's @import ...
tests/browser.rs keeps both net_http and noscript_with_scripts.
```
**Why:** most conflicts here are two streams adding to the same list
(`tests/browser.rs`, the prelude, goldens); the right answer is usually
"keep both", and the message proves nothing was dropped.

## Merging

### One merge at a time, then the full tests
**Context:** integrating a finished branch into `main`.
**Bad:** several merges in flight in the main checkout; a conflict or a
test failure cannot be pinned on one branch.
**Good:** work happens in worktrees; one merge agent merges one branch,
runs the full tests, and only then does the next one start
(`.claude/skills/pi-codex/SKILL.md:54`: "have a merge agent merge and
run the full tests"). The first-parent history of `main`
is one merge per step (`git log --first-parent --oneline`).
**Why:** each merge commit is then a known-good state to go back to.

### Merge main into the branch before you finish
**Context:** a branch that has run for hours while others merged.
**Bad:** the HTML5 forms branch recorded two agent goldens before
`stage0-milestones` changed how `pre` text is snapshotted; they failed
only after the merge and were fixed on the integration branch (7081aa9:
"The forms branch recorded two agent goldens before stage0 split pre
text into one line per line break").
**Good:** merge the integration branch in, fix what breaks on your side,
rerun the tests, as 13edac4 ("Merge stage0-milestones into M12") and
7946372 do:
```
git merge main
```
**Why:** the branch's own test run then covers what the integrator will
merge, and the integrator's merge is trivial.

### Fix the branch side when main removed an API
**Context:** main renamed or removed a function that a branch's new code
or tests still call.
**Bad:** cff3f60 removed the workaround `labelIs` from
`browser/encoding/encoding.ore` (gap #31 was fixed); the B8 encodings
branch had added tests with it (7e93451):
```
@expect(labelIs("Shift_JIS", Encoding.ShiftJis))
```
**Good:** the merge (576414f) rewrote the branch's lines to the current
API, `browser/encoding/encoding.ore:462`:
```
@expect(fromLabel("Shift_JIS") == Encoding.ShiftJis)
```
**Why:** main's removal was deliberate; bringing the old API back would
undo it.

## Multi-agent

### Parallel streams do not edit docs/browser.md
**Context:** several branches of the same milestone at once.
**Bad:** the B8 and B7b branches both edited `docs/browser.md`
(3776f64, e3a285e); d1c8800
had to merge the status table by hand ("Conflicts: docs/browser.md keeps
B7 done (without `screenshot`) and the B8 row and remains").
**Good:** each stream ends its report with a "Done so far" paragraph;
the integrator edits the shared docs once after the merges (5dd6afb:
"B8 work merged so far"). The pi task file names the files not to edit
(`.claude/skills/pi-codex/SKILL.md:34`: docs/browser.md,
docs/browser-sites.md).
**Why:** a status document edited by every branch conflicts on every
merge, and each resolution can silently lose a line.

### Run GPT only through the pi-codex skill
**Context:** handing a work stream to GPT (openai/gpt-6.1-sol) via pi.
**Bad:** a bare `pi` run can hang for hours with no output.
**Good:** launch `.claude/skills/pi-codex/pi_run.py` in background, in
its own worktree `pi-<name>` from `main`
(`SKILL.md:30`, `SKILL.md:43`). The watchdog kills the run after 900 s
with no event, 2400 s inside one tool, or 6 h in total, resumes the
same session up to 3 times, and backs off 1, 4, 9 minutes on API
errors (`pi_run.py:191`, `pi_run.py:220`). Read only `status.json` and the tail of
`progress.log`.
**Why:** a run then either finishes or exits with `stalled`/`failed`
within known limits.

### Review a GPT branch like any other, before and after the merge
**Context:** a pi run reports `done`.
**Bad:** merged GPT branches needed follow-up fixes:
- 6757c37 (HTML5 forms) defined its own `Blob`, `File` and `FormData`,
  which "hid slice(), arrayBuffer() and the bytes FileReader reads";
  fixed in e9fd680.
- The same branch wrote one-line `if` and `match` blocks unlike the
  code around them; fixed in e64d41c.
- b2193b7 (JS performance) let a prototype cache hit answer a
  function's `name` or `length`; fixed with a test in 0f2d643.
**Good:** spot-check the commits for duplicates of existing code, the
style of the surrounding file, cache or fast paths without a test, and
tests changed to pass rather than code fixed. Then have a merge agent
merge and run the full tests.
**Why:** the run's own report says its tests pass; it cannot tell you
what it duplicated or missed.

### Check a GPT report's "not supported" list against the branch
**Context:** planning follow-up work from a pi run's final report.
**Bad:** the pi-dynimport report said module cycles and re-exports were
not supported; later commits of the same run (814b1cf, fff3760) had
implemented both, with tests (`233_module_cycles.js`,
`234_module_reexports.js`). The follow-up task asked to implement them
again.
**Good:** before planning, read the branch's tests and docs
(`browser/js/MODULES.md`) and run the feature; write edge-case tests
against Node instead (`244`, `245` found two real bugs).
**Why:** a report written mid-run goes stale as the run continues.

### Build into your own worktree, not a shared scratch path
**Context:** sub agents of one session share its scratchpad directory.
**Bad:** on 2026-10-07 the pi-dynimport review agent built
`ore build browser -o $SCRATCH/ak` at 03:25:47Z. At 03:25:55Z the
async-image agent, in another worktree (865305f, before ES modules),
built `-o $SCRATCH/ak` too and finished last. The next test run used the
other branch's binary: every `import()` threw "import is not defined".
A rebuild "fixed" it, and the plan logged a flaky build (P2.7). Builds
are deterministic: 10 clean builds and 3 under load gave the same C and
binary, and all 23x/24x JS tests passed.
**Good:** `browser/build.sh` with no path (it writes `target/anyknown`
in its own worktree), or a path named after the worktree:
`-o $SCRATCH/ak-$(basename "$PWD")`.
**Why:** a shared output path lets another agent replace your binary
between your build and your test, with no sign of it.

### At most three agents run builds and tests at once
**Context:** launching parallel step agents that build and test.
**Bad:** on 2026-10-07 five parallel step agents pushed the load
average to 40–58 on 10 cores. GitHub replay measured 41 s wall for
3.5 s of CPU (P4.7), and `http_images_background` failed under GC
stress, passing on rerun (P1.2).
**Good:** cap concurrent build/test agents at three; diagnose-only or
docs steps can run beside them. Speed numbers use CPU time (P1.5) or a
quiet machine. The three slots are one per lane (Ore, Browser,
Framework); each lane's steps are in `docs/plans/` (`docs/plan.md`).
**Why:** wall-clock numbers and timing-sensitive tests are wrong on an
overloaded machine.

### A speed test limits CPU time, not wall time
**Context:** a test that fails when a program gets slower.
**Bad:** golden `perf/General` killed the debug build after 10 s of
wall time; at load 30–50 it failed and passed on rerun, though the
program needs 0.13 s of CPU (P3.11).
**Good:** `output_and_cpu_within` in `tests/golden.rs` takes the child's
CPU time from `wait4`; `perf_general` limits it to 2 s and keeps a 120 s
wall limit only to catch a hang.
**Why:** load stretches wall time many times over, but not the CPU time
the program itself uses.

## Compiler gaps

### Record a compiler limitation in browser/COMPILER_GAPS.md
**Context:** the browser needs something Ore or std does not do, or does
badly.
**Bad:** a silent local workaround; the language never hears of it.
**Good:** add a numbered entry with **Need**, **Repro**,
**Workaround**, **Hit by** and **Status**
(`browser/COMPILER_GAPS.md:8`). The language milestones take their work
from that list (`docs/milestones.md:186`, M9.5: "The browser written in
Ore recorded what hurt ... Fix those in the compiler and std").
**Why:** the browser is Ore's test case; its gap list is the language's
to-do list.

### Remove the workaround once the milestone lands
**Context:** a milestone has fixed a gap the browser worked around.
**Bad:** the old code stays; COMPILER_GAPS.md still says "Browser still
uses the workaround" (for example #31, `Interp.hasClass` in js,
`browser/COMPILER_GAPS.md:401`).
**Good:** replace the workaround and update the entry's status, as
cff3f60 ("Gaps: drop browser workarounds for gaps that are fixed") and
cb93543 (`browser/js/js.ore:18`, `let pthread = @cimport("pthread.h")`
instead of hand-written prototypes with a 128-byte guess) do.
**Why:** a workaround left behind keeps the cost the fix removed and
teaches the next agent the old idiom.

## Checklist before finishing a branch

- Everything is committed; `git status` is clean.
- `git merge main`, conflicts resolved and listed in the
  merge message.
- `cargo build --release`
- `target/release/ore check browser` (prints nothing on success)
- `cargo test` (the full suite; `cargo test --test browser` alone is
  not enough)
- `browser/build.sh` (fast mode, to `target/anyknown` in this
  worktree; `--mode safe|fast|debug` is optional; an output path, if
  any, stays inside the worktree, see "Build into your own worktree")
- New compiler gaps are in `browser/COMPILER_GAPS.md`; workarounds for
  fixed gaps are gone.
- The report has a "Done so far" paragraph; shared docs
  (`docs/browser.md`, `docs/browser-sites.md`) are left to the
  integrator.
