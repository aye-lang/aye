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
set up by the user elsewhere and is not part of this goal. (Since P8.3 the
`browser` parts of this check run inside the aye-lang/anyknown checkout of
`scripts/downstream.sh`.)

Done 2026-10-08 (X1.1–X1.7, `done.md`): the check above holds on
main b2c723d for arm64 and amd64. Linux load-timing flakes go to the
browser backlog (P7, `docs/plans/browser.md` in aye-lang/anyknown). Keep it green: a step that touches the C prelude,
`@cimport` or platform calls runs `scripts/linux.sh` before merging.

## X2: rename Ore → Aye

Done when: the command is `aye`, sources end in `.aye`, the crate,
docs, error texts and the cache directory say Aye, and the full suite
passes. Decided, not reopened: name Aye, command `aye`, extension `.aye`,
also "ayelang".

Done 2026-10-08: X2.1 wrote `scripts/rename-aye.py` (76eb976), X2.2
ran it (3042d53): 513 files moved, 321 rewritten. Kept on purpose:
page-content fixtures under `browser/tests` (now in aye-lang/anyknown), recorded search queries,
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
- files: `tests/browser.rs` (the test; moved to aye-lang/anyknown in P8.3), its golden PNG, whatever sets the URL (prefer a fixed `file:///fixture/...` style URL or a relative display over re-blessing per path)
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

## X4: go public

Make github.com/aye-lang/aye public. Decided 2026-10-12 under the
"Who decides" rule in `docs/plan.md`: go public now; the language does
not need to be finished first. aye-lang/anyknown stays private (the
user's product). This repo's history holds `browser/` up to P8.3; the
user said that old browser code may be public (2026-10-12).

Pre-public check, 2026-10-12 (cloud session, full history of aye main
da7ec99, 1325 commits): no GitHub, Anthropic, AWS, Google or Slack token
and no private key in any commit (`git log -p --all -G` over those
patterns); no `.env`-like file ever committed; `site/wrangler.toml`
holds no account id. Only personal data: author emails and one
`/Users/solemnis/...` path in `docs/aye-eval.md`, both kept. Links to
aye-lang/anyknown (README, `scripts/downstream.sh`) stay; they need a
login, as they do today.

#### X4.1: switch aye-lang/aye to public
- goal: Settings → General → Danger Zone → Change visibility on aye-lang/aye.
- notes: needs a repo admin; the cloud session has no API for repo settings, so this one step is the user's click.
