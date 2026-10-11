# Plan

Goal: finish the language (Aye, formerly Ore: command `aye`,
extension `.aye`) to the done criteria in [`plans/aye.md`](plans/aye.md),
then resume the anyknown browser in its own repo (aye-lang/anyknown). The
language lane gets most slots. The browser is frozen (maintenance only) and
is the language's biggest integration test, run as the downstream check. The framework does one slice (F1)
as the acceptance test of `std.http`. Finished steps are in
[`plans/done.md`](plans/done.md).

## Resume here (handoff 2026-10-11 night)

A new session starts in `~/Documents/ayelang/aye`. The full suite was
green on main at dcd1f6b-era commits (see the last "Plan:" commit on
main for the exact hash); every criterion O6–O13 holds (2026-10-11,
`plans/done.md`). The state is this file's lane table,
[`plans/done.md`](plans/done.md), the lane files and `git worktree list`.
Branches to merge first, then the rest in order:

1. **O3.6d** (std.http streaming initial frame; dev-server heartbeat
   threads removed) is done on branch `worktree-agent-a6fb9cf154916ee79`
   (b0843f8, main merged, all targeted tests green, downstream not run).
   Merge it, run the full suite and `scripts/downstream.sh`, fill the
   `O36D` placeholder in `plans/done.md` and `plans/aye.md`, push.
2. **O2.6** (stack overflow on wasm) is WIP on
   `worktree-agent-a8d4905904c125e6d` (388a235): the prelude check and
   `--stack-first` are in; the V8 `RangeError` mapping in the two runners
   and the marker changes are left. The card in `plans/aye.md` has the
   exact list. Continue on that branch.
3. **O2.5a** (std casts audit, cimport `int64_t` mapping) was started
   with no commit; the card in `plans/aye.md` records the partial audit
   and the cimport bug to fix. Start fresh from main.
4. **F5.9** (framework hello req/s) ran as a cloud routine on
   2026-10-11 16:05 UTC; look for its PR (`cloud/f59-bench`) and merge it.
   **F5.8** is merged (PR #4). F5 then has only the Chrome checks left
   (dev reload, island) and F1.2b (stray `.aye` in a route dir).
5. Then the backlog in `plans/aye.md` (O2.5e, O2.8a, O6.x leftovers,
   O3.6c, O7.5) and `plans/framework.md`.

Notes: `web dev` needs `aye` on `PATH` (or `AYE=`); dev CLI relative
app paths work since F3.1a. The macOS first-exec stall (O1.17b) is
unchanged; heavy verification goes to cloud Linux (rule 7).

## Lanes

| Lane | Priority | Current phase | Next step | Runs where |
|---|---|---|---|---|
| Aye language ([`plans/aye.md`](plans/aye.md)) | 1, most slots | O6–O13 all hold (2026-10-11); backlog and performance | O3.9a, O2.6, O2.8a next; site deploy needs `site/build.sh` first | local; cloud routines for Linux |
| Browser ([`plans/anyknown.md`](plans/anyknown.md)) | 2, maintenance | P8 split done (P8.1 to P8.3): `aye-lang/anyknown` repo; downstream check | `scripts/downstream.sh` after src/ or std/ merges; resumes in the anyknown repo | local (archives in `target/sites/`, screenshots) |
| Framework ([`plans/framework.md`](plans/framework.md)) | 3, one slice | resumed 2026-10-11 (O6–O9 hold) | F3 (dev server) done 2026-10-11; F4 islands next (F4.1 spike first) | local or cloud |

Order of dispatch (2026-10-08): names and style first. The rename
(X2, 3042d53), formatting (P2.4, 1075c9b) and the local move (X5) are
done, and so is Linux development (X1); next, lane cards in lane
priority (F1.1 merged 2026-10-10; Aye-lane cards next). Project steps go when their condition
holds. The browser resumes when Aye criteria O6–O9 are met, in the
same step that split `browser/` into its own repo (P8.2; P8.3 removed it here).

## Project

Steps not tied to one lane, in this order. Their cards are in
[`plans/project.md`](plans/project.md).

1. **X1 Linux development.** Done 2026-10-08 (X1.1–X1.7): the suite
   passes on Ubuntu 24.04 arm64 and amd64 through `scripts/linux.sh`.
2. **X2 rename Ore → Aye.** Done 2026-10-08 (3042d53, `scripts/rename-aye.py`).
3. **X3 website** ayelang.dev on Cloudflare Workers (`site/`); v1 live 2026-10-08.
4. **X4 go public** (repo github.com/aye-lang/aye, with the
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
   finishing a branch" (merge `main` in, `cargo build --release`, the
   test targets the card touches, and `sh scripts/downstream.sh` when the
   merge touches `src/` or `std/`). **Not the full
   suite**: main runs `cargo test --release` once after each merge, one
   run at a time (2026-10-10: parallel full suites took 34+ minutes).
   Docs-only cards skip the build and tests. The downstream check
   replaces the old browser criterion M1 (`cargo test --release --test
   browser` after every merge), which `cargo test` no longer covers
   ([`plans/anyknown.md`](plans/anyknown.md)).
6. **Do not edit** `docs/plan.md`, `docs/plans/*` or `docs/milestones.md`
   unless the card names it; main updates them once per batch
   ([workflow.md, "Parallel streams do not edit a shared status document"](practices/workflow.md#parallel-streams-do-not-edit-a-shared-status-document)).
   New compiler gaps go in `browser/COMPILER_GAPS.md` of aye-lang/anyknown.
7. **Linux lane (cloud).** A card with `where: cloud` runs in a Claude
   cloud session (a Linux VM from GitHub `aye-lang/aye`), never through
   the local Apple `container`: Linux-only bugs, the WASI toolchain on
   Linux (O2.5b), cross-link sysroots (O2.8), and the Linux full suite
   after each batch (`scripts/setup-ubuntu.sh`, then `cargo test
   --release`). It delivers a branch or PR; main reviews, merges, and
   runs the macOS suite once. `main` is pushed after every green suite
   so the cloud sees the current state (user, 2026-10-11: push freely).
8. **Diagnose, then fix.** A step whose cause is unknown is a diagnose
   step (cause written down, fix proposed); the fix is the next card.
8. **Browser symptoms:** before dispatching a browser fix, confirm the
   symptom on current anyknown `main` (`page_load.py check --screens`). P3.4 and
   P3.5 chased symptoms that no longer reproduced.
9. **Site expectations only tighten** (`browser/tools/sites.json` in aye-lang/anyknown). A
   site that loses content is a regression to fix, never a reason to
   change its expected strings (P3.20 did, for GitHub).
10. **Report:** commits (hash and one line each), the verify result, the
    full-suite result, follow-ups as proposed steps. A lesson goes into
    `docs/practices/` in the same commit as its fix.
11. **Cloud steps** (Ubuntu 24.04 VM, a fresh clone): if a tool is
    missing, run `sudo sh scripts/setup-ubuntu.sh` first. Work on a
    branch `<step id>-<slug>` from `main`, push it and open a pull
    request to `main` whose body is the report (rule 10). Never push to
    `main` or merge; main merges one PR at a time and runs the macOS
    checklist. The VM is Linux, so `scripts/linux.sh` is not used
    there; `scripts/downstream.sh` runs as is (it needs a GitHub login for
    the private anyknown repo).

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
  session; cards marked cloud run locally within the limit of 2 when
  the cloud is not in use. A `scripts/linux.sh` run counts as a build
  agent. Repeated runs under load (10×, 30×) for a flaky test
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
- lane: aye | browser | framework | project
- goal: <one sentence>
- serves: <done criterion id, e.g. O6 (Aye), W1 (browser), FD1 (framework)>
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
  github.com/aye-lang/aye, private until it goes public together
  with the browser (X4); license MIT OR Apache-2.0 (`LICENSE-MIT`,
  `LICENSE-APACHE`); site ayelang.dev (Cloudflare Workers, `site/`).
- **Platforms:** macOS (local) and Ubuntu 24.04 (Apple `container`
  locally via `scripts/linux.sh`, after X1; the user's cloud).
  Windows is out of scope. M10's ThreadSanitizer run is not possible on
  this Mac.
- **Integration branch** `main`; one gap list, `browser/COMPILER_GAPS.md` (in aye-lang/anyknown).
- **Step ids** are never renumbered: P/O (aye), P (browser), F
  (framework), X (project). Language choices: [`decisions.md`](decisions.md).
