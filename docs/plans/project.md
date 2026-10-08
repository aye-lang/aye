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

#### X2.1: rename script, dry run
- lane: project
- goal: a script that performs the whole rename (git mv of every `.ore` file, `Cargo.toml` package and binary name, `ore` command in tests and docs, `ORE_` names (environment variables, C prelude macros), `~/.cache/ore`, the planned `ore.lock` in docs), with a `--dry-run` that lists every change.
- serves: X2's done line
- files: `scripts/rename-aye.py` (new)
- verify: `python3 scripts/rename-aye.py --dry-run > target/rename.txt; echo $?` → 0; the report gives the count of files moved and lines changed, and lists every match it chose not to change (for example "ore" inside other words, `browser/` site data)
- depends on: –
- parallel with: all (changes no tracked file but the script)
- where: cloud
- model: sonnet
- time box: 45 min
- notes: about 471 tracked `.ore` files and ~317 files that contain the word "ore" (2026-10-08). Words such as "more", "store", "before" must not change; match `\bore\b`, `Ore`, `.ore`, `ORE_` only.

#### X2.2: run the rename
- lane: project
- goal: run the X2.1 script on `main`, fix what the full suite finds, merge.
- serves: X2's done line
- files: the whole tree
- verify: `cargo build --release && target/release/aye check browser && cargo test --release > target/test.log 2>&1; echo $?` → 0
- depends on: X2.1; every other branch merged or parked; no other agent running
- parallel with: none (isolated step)
- where: local or cloud, whichever holds the archives the browser tests need (`cargo test` needs none; local only if `page_load.py` is rerun)
- model: sonnet
- time box: 45 min
- notes: after it merges, main updates `docs/plan.md` and lane plans to say `aye`; old step ids stay.

## X3: website

ayelang.dev, a Cloudflare Worker serving `site/public` (deploy:
`cd site && wrangler deploy`, see `site/README.md`). v1 (landing page)
went live 2026-10-08. Next: the spec and std docs (`aye doc --html`,
O6.3, when it exists).

## X4: go public (later)

Make github.com/aye-lang/aye public together with the browser. Needs
the user's OK; nothing is pushed public before it.
