# Project steps

Cards for the Project section of [`docs/plan.md`](../plan.md): work not
tied to one lane. Ids use the prefix `X`. Rules and the card template
are in `docs/plan.md`.

## X1: cloud enablement

Done when: `cargo test --release` passes on the Claude Code cloud VM
(Ubuntu 24.04, 4 vCPU) after running the repo's setup script (criterion O1 in
[`aye.md`](aye.md)).

#### X1.1: archive.aye builds on Linux
- lane: project
- goal: `browser/net/archive.aye` stops using macOS-only struct fields, so `aye check browser` can pass on Linux.
- serves: criterion O1 (the full suite on Ubuntu 24.04)
- files: `browser/net/archive.aye`; maybe `std/` or `src/cgen.rs` (see notes)
- verify: `target/release/aye check browser && cargo test --release --test browser` → exit 0 (macOS must stay green; Linux is checked in X1.2)
- depends on: –
- parallel with: all except steps in `browser/net/`
- where: local (no Linux machine until X1.2)
- model: opus (design choice: Aye has no compile-time OS switch)
- time box: 45 min
- notes: two fields: `entry.d_namlen` (line 114; Linux `dirent` has none, use the NUL in `d_name`) and `info.st_mtimespec` (lines 131-132; Linux: `st_mtim`). `@cimport` drops static inline functions and field macros (`docs/spec.md` §`@cimport`); `browser/ui/socket.aye:48` decides macOS at run time with `uname`, which cannot choose a field name. Candidates: a std-side helper as `src/cgen.rs` does for `aye_std_listen`, or an mtime function in `std.fs` (part of O1.5).

#### X1.2: Ubuntu 24.04 setup script and the first cloud run
- lane: project
- goal: a setup script installs everything the full suite needs on a fresh Ubuntu 24.04 VM; the first `cargo test --release` there is recorded.
- serves: criterion O1
- files: `scripts/setup-ubuntu.sh` (new); `README.md` "Build and run" gets one line pointing to it
- verify: `sh scripts/setup-ubuntu.sh > target/setup.log 2>&1; echo $?` → 0, then `cargo test --release > target/test.log 2>&1; echo $?` → the exit code and every failing test are quoted in the report (failures become new X1.x steps, not fixed here)
- depends on: X1.1 merged and pushed to origin; cloud enabled by the user
- parallel with: all
- where: cloud (the step is the cloud setup)
- model: sonnet
- time box: 45 min
- notes: packages the tree needs: clang, lld, libclang-dev (`@cimport`), pkg-config, libcurl4-openssl-dev, libfreetype-dev, libsdl2-dev, fonts-dejavu-core (`browser/font/freetype.aye:170`), python3, Node 22 (apt's nodejs is older), Go and apache2-utils (`ab`) for O1.7. Rust from rustup.

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
