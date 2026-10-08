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

#### X1.2: Ubuntu setup script, `scripts/linux.sh`, first Linux run
- lane: project
- goal: `scripts/setup-ubuntu.sh` installs everything the suite needs on a fresh Ubuntu 24.04; `scripts/linux.sh <cmd...>` runs a command on the current worktree inside an Apple `container` built from it; the first full Linux run is recorded.
- serves: criterion O1
- files: `scripts/setup-ubuntu.sh`, `scripts/linux.sh`, `scripts/linux/Containerfile` (new); `README.md` "Build and run" gets one line; `docs/practices/workflow.md` one short section on running the Linux check
- verify: `scripts/linux.sh true; echo $?` → 0 (image built, worktree synced), then `scripts/linux.sh sh -c 'cargo build --release && target/release/aye check browser && cargo test --release' > target/linux-test.log 2>&1; echo $?` → the exit code and every failing test are quoted in the report (failures become new X1.x steps, not fixed here)
- depends on: –
- parallel with: all; counts as one of the two local build/test agents
- where: local (Apple `container` 1.0, image `ubuntu:24.04` already pulled)
- model: sonnet
- time box: 45 min
- notes: packages: clang, lld, libclang-dev (`@cimport`), pkg-config, libcurl4-openssl-dev, libfreetype-dev, libsdl2-dev, fonts-dejavu-core (`browser/font/freetype.aye:170`), python3, rsync, git, Node 22 (apt's nodejs is older), Go and apache2-utils (`ab`) for O1.7; Rust from rustup. `linux.sh`: one long-lived container per worktree (name from the worktree basename), worktree mounted read-only and rsynced to `/work` excluding `target/`, so `/work/target` and the cargo cache persist inside the container and the macOS `target/` is never touched; `--cpus 4 --memory 8G`; `LINUX_ARCH=amd64` selects an amd64 image under Rosetta.

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
