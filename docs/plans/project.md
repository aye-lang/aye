# Project steps

Cards for the Project section of [`docs/plan.md`](../plan.md): work not
tied to one lane. Ids use the prefix `X`. Rules and the card template
are in `docs/plan.md`.

## X1: Linux development

Done when (goal set 2026-10-08): on Ubuntu 24.04 (an Apple `container`
on this Mac, arm64 native; amd64 under Rosetta as the last check),
`scripts/setup-ubuntu.sh` then `cargo build --release`,
`target/release/aye check browser`, `cargo test --release` and
`browser/build.sh` all exit 0, run by `scripts/linux.sh` from any
worktree (criterion O1 in [`aye.md`](aye.md)). The cloud environment is
set up by the user elsewhere and is not part of this goal.

First Linux run (X1.2, arm64, before X1.1 was merged): build ok;
`aye check browser`, the 62 browser tests and `browser/build.sh` failed
on the archive fields (X1.1, since merged) and then on the cimport clash
below; lib 30/31, conformance 29/29, fmt 6/7, golden 263/264, spec 6/7.

#### X1.3: `@cimport` on glibc
- lane: project
- goal: on Ubuntu 24.04, `@cimport` of `pthread.h` no longer clashes with std's own `clock`/`nanosleep` declarations (E0096 "declared twice with different signatures"), and `@cimport("math.h")` exposes `sqrt`, `floor`, `pow`, `atan2` (E0081 today).
- serves: criterion O1
- files: `src/cimport/`, maybe `std/` declarations or `src/sema.rs` (whichever the diagnosis points to); tests under `tests/`
- verify: `scripts/linux.sh sh -c 'cargo build --release && target/release/aye check browser && cargo test --release --test golden --test spec && cargo test --release --lib codes'` → exit 0; macOS: `cargo test --release` → exit 0
- depends on: –
- parallel with: X1.4
- where: local (`scripts/linux.sh`)
- model: opus (diagnose first: write the cause in `browser/COMPILER_GAPS.md` as a new gap, then fix)
- time box: 45 min
- notes: suspects: glibc declares math functions through `__MATHCALL` macros and `__REDIRECT`; libclang may lack its resource headers (`stddef.h`) for the llvm-18 packages; std declares `clock() u64` where glibc says `clock_t clock(void)` (i64). Failing: golden `cimport/system.aye`, `codes::tests::examples_are_accurate` (E0112), spec `aye_blocks_compile` (docs/spec.md:524).

#### X1.5: full Linux run, arm64 and amd64
- lane: project
- goal: the X1 done check passes on arm64 and amd64; failures become new X1.x cards.
- serves: criterion O1
- files: none expected (report only), `docs/practices/workflow.md` if a lesson
- verify: `scripts/linux.sh sh -c 'cargo build --release && target/release/aye check browser && cargo test --release && browser/build.sh'` → 0, then the same with `LINUX_ARCH=amd64`
- depends on: X1.3
- parallel with: no other build agent
- where: local
- model: sonnet
- time box: 45 min (amd64 under Rosetta is slow)

## X2: rename Ore → Aye

Done when: the command is `aye`, sources end in `.aye`, the crate,
docs, error texts and the cache directory say Aye, and the full suite
passes. Decided, not reopened: name Aye, command `aye`, extension `.aye`,
also "ayelang".

Done 2026-10-08: X2.1 wrote `scripts/rename-aye.py` (76eb976), X2.2
ran it (3042d53): 513 files moved, 321 rewritten. Kept on purpose:
page-content fixtures under `browser/tests`, recorded search queries,
and lines that describe the rename.

## X3: website

ayelang.dev, a Cloudflare Worker serving `site/public` (deploy:
`cd site && wrangler deploy`, see `site/README.md`). v1 (landing page)
went live 2026-10-08. Next: the spec and std docs (`aye doc --html`,
O6.3, when it exists).

## X5: local move

#### X5.1: path-independent `window_forms_html5`
- lane: project
- goal: the `forms_html5.validation.png` golden no longer contains the checkout path (the URL bar shows the file path), so the test passes from any directory.
- serves: X5
- files: `tests/browser.rs` (the test), its golden PNG, whatever sets the URL (prefer a fixed `file:///fixture/...` style URL or a relative display over re-blessing per path)
- verify: `cargo test --release --test browser window_forms_html5` → exit 0 from the main checkout and from a copy under `/tmp` (`git worktree add /tmp/x51 HEAD`)
- depends on: P2.4
- parallel with: none of the src/ or tests/ steps
- where: local
- model: sonnet
- time box: 30 min
- notes: found by X2.1's trial run.

#### X5.2: move the checkout
- lane: project
- goal: the repo lives at `~/Documents/ayelang/aye`.
- serves: X5
- files: none tracked
- verify: in the new folder, `cargo test --release > target/test.log 2>&1; echo $?` → 0
- depends on: X5.1; no agent running, no worktree open
- where: local (main context, a few commands)
- notes: `mv`, then copy the memory dir to the new project key; the user restarts Claude Code there.

## X4: go public (later)

Make github.com/aye-lang/aye public together with the browser. Needs
the user's OK; nothing is pushed public before it.
