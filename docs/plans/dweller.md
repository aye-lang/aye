# Plan: the Dweller browser lane

The browser lane of [`docs/plan.md`](../plan.md) now lives in
[github.com/aye-lang/dweller](https://github.com/aye-lang/dweller)
(formerly aye-lang/anyknown; open source since 2026-10-15). P8 split `browser/` out of this repo with its history (P8.1
dry run, P8.2 repo created at 3c01043 with CI green on macOS and Ubuntu,
P8.3 removal here, [`done.md`](done.md)). The lane's plan, backlog, site
list and browser docs are in that repo (`docs/plans/browser.md`,
`docs/browser.md`, `docs/browser-sites.md`, `docs/agent-protocol.md`,
`docs/practices/browser.md`); the paths inside it are unchanged.

The lane stays in maintenance mode: no site fixes or features until the
Aye criteria hold and the user resumes it.

## What stays in the Aye repo

`tests/common/mod.rs` (dweller has a copy), `docs/practices/workflow.md`
and `aye.md` (their entries cite `browser/` paths from before the split),
`scripts/setup-ubuntu.sh` (the apt packages for libcurl, FreeType and SDL2),
`site/public/index.html` (a link to the repo), comments in `std/net/net.aye`
and `std/http/http.aye` that name `browser/` files, and the history in
`docs/decisions.md`, `docs/plan.md` and `docs/plans/*` that mentions the
browser as the largest program written in Aye.

## How dweller finds the compiler

`aye` is a tool, not a library: std is built in, so the browser has no
`[deps]` entry. dweller pins the compiler in its file `AYE_REV` (a commit
of `aye-lang/aye`) and installs it with `cargo install --git
https://github.com/aye-lang/aye --rev $(cat AYE_REV) --locked aye`. A
developer with an Aye checkout sets `AYE=/path/to/aye/target/release/aye`
(read by `browser/build.sh` and by the test harness `tests/browser.rs`).

## The downstream check (replaces M1)

M1 was "`cargo test --release --test browser` exits 0 on `main` after every
merge". `cargo test` here no longer builds the browser, so the browser is
checked against the compiler by `scripts/downstream.sh`:

- `tests/downstream/DWELLER_REV` holds the dweller commit the check
  uses (3c01043 at P8.3).
- `sh scripts/downstream.sh` clones `aye-lang/dweller` (plain `git
  clone`, so it needs network only) or updates the clone
  under `target/downstream/dweller`, checks out that commit, then runs in
  it, with `AYE=<checkout>/target/release/aye`: `aye check browser`,
  `aye test browser` and `cargo test --release --test browser`. It stops at
  the first failure, prints the tail of the log, and exits non-zero. The log
  is `target/downstream.log`. It needs `cargo build --release` first.
- Who runs it: CI (job `downstream` in `.github/workflows/ci.yml`, Linux,
  in parallel with the golden shards) on every push and PR that is not
  docs-only. Run it locally (also on a cloud VM) when changing `src/` or
  `std/`
  ([`practices/workflow.md`](../practices/workflow.md), "Checklist before
  finishing a branch"); macOS is covered only by the local Mac.
- When it fails: the compiler change broke the browser, or the browser
  relied on something the change removed. Fix the Aye side if the change is
  wrong. If the change is right (a deliberate removal), fix dweller in its
  repo (rule: "Fix the branch side when main removed an API"), push it, and
  move `DWELLER_REV` to the fixed commit in the same Aye merge. Never
  revert a language change to keep the browser green.
- Moving the pin otherwise: set `DWELLER_REV` to the new dweller `main`
  and run the script once. dweller moves its own compiler pin (`AYE_REV`)
  on its side when it needs a newer Aye.

## Cards

Cards of this lane are written in the dweller repo
(`docs/plans/browser.md` there). A card that needs a change here (a
compiler gap, a std addition) goes to [`aye.md`](aye.md) as a normal Aye
card; gaps are recorded in `browser/COMPILER_GAPS.md` of dweller.
