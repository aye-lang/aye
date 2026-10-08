# Plan

Goal: finish the language (Ore, being renamed Aye: command `aye`,
extension `.aye`) to the done criteria in [`plans/ore.md`](plans/ore.md),
then resume the anyknown browser in its own repo. The language lane gets
most slots. The browser is frozen (maintenance only) and is the
language's biggest integration test. The framework does one slice (F1)
as the acceptance test of `std.http`. Finished steps are in
[`plans/done.md`](plans/done.md).

## Lanes

| Lane | Priority | Current phase | Next step | Runs where |
|---|---|---|---|---|
| Ore/Aye language ([`plans/ore.md`](plans/ore.md)) | 1, most slots | P2 leftovers + O1 server basics | O1.3a | cloud once X1.2 is done; local until then |
| Browser ([`plans/browser.md`](plans/browser.md)) | 2, maintenance | PW wrap-up, then P7 maintenance | P3.48a, then P3.58 | local (archives in `target/sites/`, screenshots) |
| Framework ([`plans/framework.md`](plans/framework.md)) | 3, one slice | F1 (F1.1 only) | F1.1, after O1.4a | cloud |

Order of dispatch: the browser wrap-ups P3.48a, P3.62, P3.58 and
P3.58a first (P3.48, O1.2 and O1.3 are merged: 34f6f93, a3498da,
56967d4), with X1.1 beside them;
then lane cards in lane priority. Project steps go when their condition
holds: X1.2 once the user enables cloud, X2.2 at a point with no open
branch. The browser resumes when Ore criteria O6–O9 are met, in the
same step that splits `browser/` into its own repo (P8.1).

## Project

Steps not tied to one lane, in this order. Their cards are in
[`plans/project.md`](plans/project.md).

1. **X1 cloud enablement.** X1.1: `browser/net/archive.ore` reads
   `d_namlen` and `st_mtimespec` (macOS only, lines 114 and 131); make
   it build on Linux. X1.2:
   an Ubuntu 24.04 setup script in the repo, then the first full
   `cargo test --release` on the cloud VM (4 vCPU). Failures become
   new steps.
2. **X2 rename Ore → Aye.** X2.1 writes and dry-runs the rename script;
   X2.2 runs it as one isolated step: no other agent runs, no other
   branch is open.
3. **X3 website** aye.anyknown.com on GitHub Pages.
4. **X4 go public** (repo github.com/anyknown-com/aye, with the
   browser): only with the user's OK.

## Execution rules

Every agent reads this section, its one step card, and
[`practices/README.md`](practices/README.md) plus the practices file of
its area. Nothing else of the plan.

1. **One step per agent.** The agent does its card, commits, reports,
   and stops. It never starts another step. What else it finds goes into
   the report as a proposed step (lane, goal, verify). Test or fixture
   first; commit small and often
   ([workflow.md, "Commit small and often"](practices/workflow.md#commit-small-and-often)).
2. **Time box** 45 minutes unless the card says less. At the limit:
   commit what is coherent, report, stop.
3. **Stop rules.** The same verify fails twice in a row: stop and report
   what was tried. Past ~150 turns: commit what works, report, stop.
   Waiting on a build or a background job: one blocking wait with a
   timeout, never a poll per turn. Build and test output goes to a log
   file; read it with `tail -n 30` or `grep`.
4. **Verify** is the card's command. The report quotes its real exit
   code and the summary line of its output (for example `test result:
   ok. 257 passed`), not "tests pass".
5. **Before finishing a branch** run the checklist in
   [`practices/workflow.md`](practices/workflow.md), "Checklist before
   finishing a branch" (merge `main` in, `cargo build --release`,
   `ore check browser`, full `cargo test --release`, `browser/build.sh`).
   Docs-only cards skip the build and tests.
6. **Do not edit** `docs/plan.md`, `docs/plans/*`, `docs/browser.md`,
   `docs/browser-sites.md`, or `docs/milestones.md` unless the card names
   it; main updates them once per batch
   ([workflow.md, "Parallel streams do not edit docs/browser.md"](practices/workflow.md#parallel-streams-do-not-edit-docsbrowsermd)).
   New compiler gaps go in `browser/COMPILER_GAPS.md`.
7. **Diagnose, then fix.** A step whose cause is unknown is a diagnose
   step (cause written down, fix proposed); the fix is the next card.
8. **Browser symptoms:** before dispatching a browser fix, confirm the
   symptom on current `main` (`page_load.py check --screens`). P3.4 and
   P3.5 chased symptoms that no longer reproduced.
9. **Site expectations only tighten** (`browser/tools/sites.json`). A
   site that loses content is a regression to fix, never a reason to
   change its expected strings (P3.20 did, for GitHub).
10. **Report:** commits (hash and one line each), the verify result, the
    full-suite result, follow-ups as proposed steps. A lesson goes into
    `docs/practices/` in the same commit as its fix.

Merging (main's side):

- **Commit audit** before a merge: `git log --oneline main..<branch>`;
  every commit belongs to the step, none is a stray WIP or revert pair.
- **One merge per integrator agent**, then the full tests:
  [workflow.md, "One merge at a time, then the full tests"](practices/workflow.md#one-merge-at-a-time-then-the-full-tests);
  conflicts are listed in the merge message
  ([workflow.md, "Say how each conflict was resolved"](practices/workflow.md#say-how-each-conflict-was-resolved)).
  A branch that already contains current `main` and passed the full
  suite merges without new tests (git only); otherwise the merge agent
  merges `main` in first and runs the full suite.
- **Screenshots before and after** for a merge that changes rendering:
  `page_load.py check --screens` on the touched sites, both contact
  sheets named in the report.
- **Parallelism:** the local machine runs at most 2 build/test agents
  at once, and only local-only work once cloud is enabled
  ([workflow.md, "At most two local agents run builds and tests at once"](practices/workflow.md#at-most-two-local-agents-run-builds-and-tests-at-once)).
  Docs-only and git-only agents run beside them. Cloud agents run on
  the Claude Code cloud VM (Ubuntu 24.04, 4 vCPU), one step per
  session; until X1.2 is done, cards marked cloud run locally within
  the limit of 2. Repeated runs under load (10×, 30×) for a flaky test
  are the merge agent's check, not every step's.
- **Model per step:** git only (commit, merge without tests, cleanup,
  push) → haiku git-runner. A merge that needs build and tests → sonnet,
  one merge per agent. Implementation with a clear spec → sonnet.
  Diagnosis or design judgment → opus. Mechanical scripts → haiku
  script-runner. GPT only through the pi-codex skill
  ([workflow.md, "Run GPT only through the pi-codex skill"](practices/workflow.md#run-gpt-only-through-the-pi-codex-skill)).

## Step card template

Lane plans keep cards for the current and next phase only; later work
is a one-line backlog entry until its phase comes up.

```
#### <id>: <short title>
- lane: ore | browser | framework | project
- goal: <one sentence>
- serves: <done criterion id, e.g. O6 (Ore), W1 (browser), FD1 (framework)>
- files: <paths expected to change; (new) for new files>
- verify: `<exact command>` → <expected exit code / output line>
- depends on: <step ids, or –>
- parallel with: <step ids or "all"; "not with X" for shared files>
- where: cloud | local (<reason>)
- model: haiku | sonnet | opus
- time box: 45 min
- notes: ≤ 3 lines, only what the agent cannot find in the code
```

A card whose work does not fit one agent in the time box with one
verify is split before dispatch. A card that serves no done criterion
goes to the backlog or to `done.md` as dropped.

## Standing decisions

- **Lanes and priority (2026-10-08):** language first; browser frozen
  until O6–O9, then split into its own repo (git filter-repo) and
  resumed; framework does F1 only. Lane decisions are in each lane plan.
- **Name and repo:** Ore → Aye (`aye`, `.aye`, "ayelang"); repo
  github.com/anyknown-com/aye, private until it goes public together
  with the browser (X4); license MIT OR Apache-2.0 (`LICENSE-MIT`,
  `LICENSE-APACHE`); site aye.anyknown.com via GitHub Pages.
- **Platforms:** macOS (local) and Ubuntu 24.04 (cloud, after X1).
  Windows is out of scope. M10's ThreadSanitizer run is not possible on
  this Mac.
- **Integration branch** `main`; one gap list, `browser/COMPILER_GAPS.md`.
- **Step ids** are never renumbered: P/O (ore), P (browser), F
  (framework), X (project). Language choices: [`decisions.md`](decisions.md).
