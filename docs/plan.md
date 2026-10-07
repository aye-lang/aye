# Plan: Ore, the anyknown browser and the framework

Work runs in three lanes at the same time. Each lane has its own plan
file of phases and steps. A step is one goal with one verify command,
small enough for one agent in one focused session (one reviewable commit
series). An agent gets exactly one step; whatever else it finds goes
into its report as a new step, in the lane it belongs to.

## Lanes

| Lane | Plan | Slot | Current phase |
|---|---|---|---|
| Ore: make the language and toolchain complete | [`plans/ore.md`](plans/ore.md) | 1 | P2 (open rows), then O1 |
| Browser: the anyknown browser | [`plans/browser.md`](plans/browser.md) | 2 | P3 and P4 |
| Framework: a Next.js-like web framework in Ore (`framework/`) | [`plans/framework.md`](plans/framework.md) | 3 | F0 |

A slot is one of the three build-and-test agents that may run at once
(`docs/practices/workflow.md`, "At most three agents run builds and
tests at once"); each lane has one. Docs-only and diagnose-only steps
can run beside them. Every lane's steps are short-lived worktrees off
`main`, merged one at a time by the integrator.

## Cross-lane dependencies

- Gaps flow to the Ore lane: the browser and the framework record what
  Ore or std lacks in `browser/COMPILER_GAPS.md` (**Hit by** names the
  step); the Ore lane fixes or declines them (O3 in `plans/ore.md`).
- F1.1 needs O1.4 (`std.http` server); F1.2 needs O1.5 (fs, path);
  F1.3 needs O3.1 (JSON).
- The dev server (F3) needs O1.6 (processes) and O3.6 (server-sent
  events, cookies, forms).
- Client code (F4) needs O2.4 (wasm32-freestanding) for Ore islands, or
  O4.2 (path dependencies) plus a browser-lane step that moves the JS
  lexer and parser into their own module, for a bundler.
- Testing the framework's islands and dev reload in the anyknown browser
  would need `WebAssembly` and `EventSource` there (neither exists); the
  framework tests with Node 22 and Chrome instead.
- Steps that change `src/` or `std/` can break `browser/`: every lane
  runs the full check of rule 4 below.

## How an agent runs a step

0. Time box: 45 minutes per step. When it runs out, the agent stops,
   commits what is coherent on its own, and reports its findings; the
   rest becomes new steps. A step whose cause is unknown is a diagnose
   step first; its fix is the next step. Repeated runs under load (10×,
   30×) belong to the integrator's merge check, not to every step.
   (P4.9 took 2 h 17 min and P1.7 over 1 h 40 min before this rule.)

1. Input: step id, goal, verify, deps. Own worktree off `main`.
   GPT only through the pi-codex skill.
2. Read `docs/practices/README.md` and the practices file for the area.
3. Test or fixture first; small commits. Do not edit `docs/browser.md`,
   `docs/browser-sites.md` or this file; `docs/milestones.md` only in
   P2.6. New compiler gaps go in `browser/COMPILER_GAPS.md`.
4. Before finishing: merge the integration branch in (list each
   conflict in the merge message); `cargo build --release`,
   `target/release/ore check browser`, full `cargo test`,
   `browser/build.sh` with no output path (it writes `target/anyknown`
   in your worktree; never a shared path such as the scratchpad).
   `browser/tools/sites.json` expectations only tighten. If a site loses
   content it had, that is a regression to fix, never a reason to swap
   its expected strings (P3.20 did, for GitHub's "Unable to load page").
5. Report: commits, the verify output, the full test result, a "Done so
   far" paragraph, and follow-ups as new step proposals. A lesson goes
   into `docs/practices/` with its fix.
6. Integrator: one merge at a time, `cargo test` after each, shared docs
   once per batch, then deletes the branch and worktree.

"This file" in rule 3 includes the lane plans in `docs/plans/`; the
integrator updates them. Lane-specific decisions are in each lane's
plan.

## Decisions

- **Lanes (2026-10-08):** Ore, Browser and Framework run in parallel,
  one build slot each. The single plan was split into `docs/plans/`;
  P-ids kept their rows (P0, P1, P3–P6 in `browser.md`, P2 in
  `ore.md`), new rows use `O` and `F`.
- **Compiler gaps:** one list, `browser/COMPILER_GAPS.md`, for every
  lane.
- **Integration branch:** `main` (since P0.8, 2026-10-07).
- **Platform:** macOS only. There is no remote and no CI. M10's
  ThreadSanitizer run is recorded as not possible on this machine.
