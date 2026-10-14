# Plan

Goal: finish the language (Aye, formerly Ore: command `aye`,
extension `.aye`) to the done criteria in [`plans/aye.md`](plans/aye.md),
then resume the anyknown browser in its own repo (aye-lang/anyknown). The
language lane gets most slots. The browser is frozen (maintenance only) and
is the language's biggest integration test, run as the downstream check. The framework does one slice (F1)
as the acceptance test of `std.http`. Finished steps are in
[`plans/done.md`](plans/done.md).

## Resume here (handoff 2026-10-14, written 13:10 +08)

A new session starts in `~/Documents/ayelang/aye`. Cloud is the
integrator (rule 7): each step runs as a one-time cloud routine
(`RemoteTrigger`, environment `env_01CZsU4Eo82wo62Hpf2skA2q`, Ubuntu
24.04 x86_64) that merges `main` in, runs the Linux suite in foreground
chunks, and fast-forwards `main`; GitHub CI is the full-suite gate. The
local Mac pulls, does macOS-only work through PRs (merged with
`gh pr merge --squash` once all six CI jobs pass; the repo has no branch
protection, so never use `--auto`), and runs `sh scripts/downstream.sh`
after std/src changes.

CI since 2026-10-14 (#13): `golden` runs in three shards per OS
(`AYE_GOLDEN_SHARD=i/3`, tests/golden.rs; shard 1 also runs the other
test targets), wasi-sdk is cached, and a push that changes only docs
(everything under `docs/` except `docs/spec.md`, root `*.md`) runs no CI.
A run takes about 4 minutes (was 7.5).

Merged on 2026-10-14 (all in `plans/done.md`): O1.28 (`Type.m` as a
function value, gap 52), O1.29 (generic methods, gap 53b), O3.5a
(`std.net.wait`, TLS waits on the active socket, `http.lastError`),
O3.9e (wrk: no keep-alive skew, no code), O5.2b and O5.1b (warm each
root's test executable: 450 ms to 5 ms), O5.2f (`aye test` without `-g`:
-14% wall on `aye test browser`), CI sharding (#13), the lsp debounce
test made deterministic (#11, #14). Criterion O3: gaps 49-56 fixed or
declined (decisions.md "Compiler gaps closed out, round 2 (O3)"), and
anyknown#1 moved its gap file's "## Open" to empty. O6.1c (lsp end spans)
merged from the cloud.

Anyknown maintenance the same day, each a PR merged on anyknown's own CI
(macOS + ubuntu): #2, #4 and #8 fix the three known flaky image tests
(image load/error fired in the finishing task; a dropped in-flight load
is held, one fetch per document; the test server's `/held-now` waits for
every named image), #3 O1.17c, #5 O1.14c (`@targetOs()` in the browser), #6
P8.x (`std.fs.readDir` in archive.aye). `tests/downstream/ANYKNOWN_REV`
points at anyknown main after these. Framework browser checks (island,
dev reload) passed in headless `agent-browser` (plans/framework.md F3,
F4); the no-browser hook allows that shim, so such checks are agent
work. Check `gh run list` and remote `<step>-<slug>` branches on resume.

Cloud-prompt rules (learned): the session ends when its turn ends, so
every long command runs in the foreground with a 10 minute timeout,
split into chunks; start by resetting the local main to origin/main;
source `/etc/profile.d/aye-wasi.sh`; `cargo fmt` before committing;
`scripts/downstream.sh` cannot reach the private anyknown repo from the
VM (skipped, say so; the Mac runs it). The scheduler fires `run_once_at`
routines up to 30 min late; a manual `run` makes a late one fire twice.
Sessions often leave their merged branch on origin: prune them.

State: all criteria O1–O13 hold; the Aye backlog in `plans/aye.md` has
no open card; framework F0–F5 are done. Next, per the goal at the top:
resume the anyknown browser in its own repo (its `docs/plans/browser.md`
backlog: P3.63, P1.6, P1.14, ...), with this repo in maintenance and
performance mode (new cards from downstream needs or measurements).

## Lanes

| Lane | Priority | Current phase | Next step | Runs where |
|---|---|---|---|---|
| Aye language ([`plans/aye.md`](plans/aye.md)) | 1, most slots | O1–O13 hold (2026-10-14); backlog empty | maintenance; cards from downstream needs | cloud routines; local Mac for macOS-only PRs |
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
4. **X4 go public** (aye-lang/aye only): decided 2026-10-12, go
   public now. The pre-public check passed (`plans/project.md` X4); left
   is the visibility switch (a repo admin setting). aye-lang/anyknown
   stays private: it is the user's product.

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
7. **Cloud is the integrator (user, 2026-10-12).** The work runs in
   Claude cloud sessions (Ubuntu 24.04 VM, a fresh clone of
   `aye-lang/aye`), merges included: the session that finishes a step
   reviews it, merges it into `main` and pushes `main` (rule 11). Only
   one integrator: the local Mac session does not merge while the cloud
   owns `main`; it pulls first. macOS is checked by the CI macOS job
   (`.github/workflows/ci.yml`, free once the repo is public); until it
   runs, a change to macOS-only paths (`cfg!(target_os = "macos")`:
   cimport's sysroot and libclang lookup, the Mach-O and
   `x86_64-macos` targets) waits for a green macOS run before merging.
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
    branch `<step id>-<slug>` from `main`. To merge: the commit audit
    below, `main` merged into the branch, the Linux full suite (`cargo
    test --release`; `scripts/downstream.sh` when `src/` or `std/`
    changed), then a fast-forward push of `main`; the report (rule 10)
    is the merge commit's or the last commit's message. A PR is only
    for a change someone should see before it lands. The VM is Linux,
    so `scripts/linux.sh` is not used there; `scripts/downstream.sh`
    needs read access to the private anyknown repo (the session's
    GitHub access), and never writes to it.

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

- **Who decides (user, 2026-10-12):** the AI develops this language on
  its own and makes every decision in this repo (aye-lang/aye), going
  public included. Do not ask the user; decide, write the decision down
  (here, in a lane plan or in `decisions.md`) and go on. The user only
  stops work when their token budget runs out. Merges and pushes to
  `main` are the cloud session's too (rule 7).
- **anyknown is the user's (user, 2026-10-12):** aye-lang/anyknown is
  the user's product and outside the AI's decisions: it stays private,
  its license, visibility and product choices are the user's. From
  here it is read only (`scripts/downstream.sh`); a change it needs is
  proposed to the user, not pushed.
- **Lanes and priority (2026-10-08):** language first; browser frozen
  until O6–O9, then split into its own repo (git filter-repo) and
  resumed; framework does F1 only. Lane decisions are in each lane plan.
- **Name and repo:** Ore → Aye (`aye`, `.aye`, "ayelang"); repo
  github.com/aye-lang/aye, going public on its own (X4, decided
  2026-10-12; aye-lang/anyknown stays private); license MIT OR Apache-2.0 (`LICENSE-MIT`,
  `LICENSE-APACHE`); site ayelang.dev (Cloudflare Workers, `site/`).
- **Platforms:** macOS (local) and Ubuntu 24.04 (Apple `container`
  locally via `scripts/linux.sh`, after X1; the user's cloud).
  Windows is out of scope. M10's ThreadSanitizer run is not possible on
  this Mac.
- **Integration branch** `main`; one gap list, `browser/COMPILER_GAPS.md` (in aye-lang/anyknown).
- **Step ids** are never renumbered: P/O (aye), P (browser), F
  (framework), X (project). Language choices: [`decisions.md`](decisions.md).
