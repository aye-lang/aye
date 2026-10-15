# Adoption steps

Cards for project step X6 in [`docs/plan.md`](../plan.md): get Aye real
users other than its author. Ids use the prefix `A`. Rules and the card
template are in `docs/plan.md`. Written 2026-10-15 after the user said:
"update your website; you yourself need to make this language have
customers, not just me."

## Audit: the path a stranger takes today (2026-10-15, main dfed5e5)

1. **Discover.** The repo is public (since X4) with 0 stars, 0 forks, no
   topics and no homepage URL. Its description still says "Aye: a
   systems language for AI-written code, and the anyknown browser
   written in it": the old browser name, and the browser left this repo
   in P8.3. The org `aye-lang` has no description, blog or social
   account. Nothing has been posted anywhere.
2. **Website.** ayelang.dev is one page (`site/public/index.html`, 169
   lines, plain HTML and CSS, no JavaScript) served by a Cloudflare Worker
   with static assets (`site/wrangler.toml`, custom domains
   `ayelang.dev` and `www.ayelang.dev`). There is no GitHub Pages site
   (`gh api repos/aye-lang/aye/pages` is 404) and no deploy workflow:
   someone runs `cd site && wrangler deploy` by hand (`site/README.md`).
   The live page is **behind the repo**: it still says "the anyknown
   browser" and "The browser will be open source too, released after the
   language", and it lacks the std reference links; `/std/` is 404 live.
   Both the live page and the repo copy say "Aye is not released yet.
   There is no download and no public repository", show a "Pre-release"
   badge, and list as roadmap things that are done: the HTTP server and
   client, JSON, packages, wasm32 and Linux targets, the test runner,
   the language server and the web framework.
3. **Understand.** The site's pitch ("a systems language for code
   written by AI agents") is clear in 10 seconds. The README is not: it
   opens with "This repository holds stage0 ... a first slice of the
   language" and then 545 lines of command reference. The design
   rationale (`docs/design.md`, including the comparison table with Go,
   Zig, Rust and Odin) is written in Chinese. `docs/spec.md` (English)
   is written for agents, and is good, but it is a reference, not a pitch.
4. **Install.** No GitHub release and no tag (`gh release list` is
   empty; `Cargo.toml` says 0.1.0). The only paths are `cargo build
   --release` in a clone, or `cargo install --git
   https://github.com/aye-lang/aye aye` (written only in Dweller's
   README). Both need a Rust toolchain, and every build also needs
   `clang` on `PATH`, because Aye emits C. No install script, no
   Homebrew, no `aye --version`. Targets: `aarch64-macos`,
   `x86_64-macos`, `aarch64-linux`, `x86_64-linux`, `wasm32-wasip1`,
   `wasm32-freestanding` (README "Build and run"); CI runs Ubuntu 24.04
   x86_64 and macOS 15 arm64 only. Windows is out of scope (plan.md,
   "Platforms").
5. **Learn.** No tutorial or tour. No `examples/` directory (only
   `framework/examples/{hello,pages,posts}` and 415 test cases in
   `tests/cases`). The std reference exists (`aye doc --html`,
   `site/build.sh`) but is not live. Errors are a strength: stable codes,
   `aye explain`, fix-its, `aye check --json`. `aye lsp` does
   diagnostics, formatting, hover and go to definition (README "Editor
   support"), but there is no VS Code extension and no syntax grammar, so
   no editor highlights `.aye` and GitHub shows it as plain text.
6. **Proof.** Strong but scattered: the agent eval
   (`docs/aye-eval.md`: 15 of 15 runs passed, fresh Sonnet agents that
   learn Aye only from `aye spec` and the checker), Dweller (a web
   browser in Aye, public at aye-lang/dweller, built against every main
   push by the CI `downstream` job; 104,795 lines in 185 files on
   2026-10-08 per the site), the framework (`framework/`, design in
   `docs/framework.md`) and the benchmarks (`docs/aye-bench.md`). The
   benchmark headline (wrk, 2026-10-14, commit 0e7463d: Aye `std.http`
   146.0k req/s, Go `net/http` 70.3k, Node 22 16.6k) has caveats a
   reader must see: it is a hello-world `GET /`; the client shares the
   4-core VM with the server; Go measured 68k under `ab` on this VM but
   126k on another VM shape (F5.9 table); on 2026-10-11, before the
   worker pool (O3.9), Go beat Aye 99k to 39k; and the wrk script is
   "not committed". The hello server binary is 43,600 bytes (Go:
   8,215,565).
7. **Community.** Issues are on; Discussions are off. No issue
   templates, no code of conduct, no SECURITY.md, no CHANGELOG, no
   releases to watch or subscribe to. CONTRIBUTING.md (20 lines)
   describes the internal agent workflow (lanes, worktrees,
   `docs/plan.md`), not how a stranger reports a bug or sends a first
   PR. License is clear: MIT OR Apache-2.0.
8. **Playground.** None. Feasibility checked: the compiler crate has
   zero dependencies and embeds std and the spec (`include_str!` in
   `src/module.rs`, `src/main.rs`). `cargo build --lib --release
   --target wasm32-unknown-unknown` fails with only three errors, all
   Unix-only process code in `src/driver.rs` (lines 976, 1010, 1062:
   `std::os::unix`, `pre_exec`, `ExitStatus::from_raw`). So check, fix,
   fmt, explain and emit-c in the browser is about two cards. Running a
   program needs C to wasm in the browser (clang built to wasm, a large
   download, then Aye's existing `wasm32-wasip1` target and a WASI shim
   like `scripts/wasi-run.mjs`) or a server sandbox (costs money).

## Who adopts first

Honest ranking of who could plausibly use Aye in the next months, and why.

1. **Developers whose code is mostly written by coding agents** (Claude
   Code, Codex, Cursor) and who want native, GC-free programs: CLI
   tools, small HTTP services, glue around C libraries. What they get
   that Zig, Rust or C do not give them: the agent learns the whole
   language from one command (`aye spec`, under 30k tokens, every
   example checked by `tests/spec.rs`), errors are machine-readable with
   fixes (`aye check --json`, `aye fix`), and the result is one small
   binary. This is the only segment where Aye has a unique answer, so it
   leads the pitch. What stops them today: no binary install, no
   editor support, pre-1.0 (no stability promise).
2. **Language designers and hobbyists** (r/ProgrammingLanguages,
   lobste.rs, HN). They read the spec, star, file issues, and argue
   about the design; few ship with it. They are the discovery channel
   and the first contributors. They need: an English design rationale,
   the decision log, a playground.
3. **Zig and C programmers curious about explicit allocators with less
   ceremony.** Weaker: Zig has a mature community, and Aye is also
   pre-1.0. They come for the C interop (`@cimport`, direct calls) and
   stay only if the toolchain feels solid.

Not targets now: Windows users (out of scope), teams that need a
stability promise or a package ecosystem, frontend web developers.

## Positioning

**Pitch:** Aye is a small systems language without a garbage collector,
built so that an AI agent can learn it from one command and write
working native code.

**Proof points** (each one checkable in the repo, and each one shown with
its source link):

1. **Agents learn it cold.** Fresh Sonnet agents given only `aye spec`
   and the checker passed 15 of 15 runs of five tasks (grep, line
   counts, an HTTP server, JSON totals, word frequencies), with a median
   of 11 turns and 1 `aye check` call; the 15 runs cost 1.85 USD in all
   (`docs/aye-eval.md`, 2026-10-11, rerun with `scripts/eval.py`).
2. **It scales to a browser.** Dweller, a web browser written in Aye
   (HTML, CSS, a JavaScript engine, layout, painting, SDL2, FreeType and
   libcurl called through `@cimport`), about 105k lines, is built and
   tested against every compiler change on main (`.github/workflows/ci.yml`,
   job `downstream`).
3. **Small and fast binaries.** A 43.6 KB Aye hello HTTP server served
   146k req/s under `wrk` against 70k for Go `net/http` and 16.6k for
   Node 22 on one 4-core VM (`docs/aye-bench.md`, with its methodology
   and caveats: a hello-world microbenchmark, client on the same VM).

Words to avoid until they are true: "production ready", "stable",
"safe" without the qualifier "runtime-checked in debug and safe
builds", "faster than Go" as a general claim.

## Funnel

| Stage | A stranger needs | Today | Cards |
|---|---|---|---|
| Discover | A post, a link, a search hit that names the problem Aye solves | Nothing posted; stale repo description | A0, A18 |
| Understand (10 s) | Pitch, a real code sample, proof with links | Site pitch good but stale; README is a manual | A1, A3, A16 |
| Try (5 min) | One install command, hello world, an error that fixes itself | Needs Rust + clang + a clone | A4, A5, A6, A7, A12 |
| Build something | A tour, examples, std docs, editor support | No tour, no examples, std docs not live, no extension | A8, A9, A10, A11 |
| Stay | Releases to follow, a place to ask, a changelog, a reason to come back | No releases, no Discussions, no changelog | A14, A15, A20 |

## Gaps ranked by impact on adoption

1. **No install without a Rust toolchain** (no release, no binary, no
   script). Every try ends here. Cards A4 to A7.
2. **The website is stale and says the opposite of the truth** ("no
   download and no public repository", "anyknown", a roadmap of done
   work) and is deployed by hand from one machine. Cards A1, A2.
3. **No pitch-first entry in the repo**: README opens with "stage0 ...
   first slice", the design rationale is in Chinese, the repo
   description is stale. Cards A0, A3, A16.
4. **No learning path**: no tour, no examples, std reference not live.
   Cards A8, A9 (and A1 deploys `/std/`).
5. **No editor support a user can install** although `aye lsp` exists.
   Cards A10, A11.
6. **Proof is not reproducible by a stranger**: the wrk script is not
   committed, and there is no comparative eval against Zig or Rust. Cards
   A17, A19.
7. **Nothing to follow and nowhere to talk**: no releases, changelog,
   Discussions, templates. Cards A14, A15.
8. **No playground.** Feasible (see audit item 8), lower priority than
   install because the target user has a terminal. Cards A12, A13.

## Launch gate

Post nowhere (A18) before A0, A1, A2, A3, A5, A6, A8, A9, A14 and A15
are done and A21 (the fresh-eyes run) passes. A10 and A12 are wanted but
do not block the launch.

## Who does what

Under "Who decides" (plan.md) every repo decision is the assistant's,
including what the site says, when to tag a release and its version.
The user is needed only for: accounts the assistant cannot create
(Cloudflare API token, VS Code Marketplace and Open VSX publishers,
social accounts, lobste.rs invite), anything that spends money, and
posting under their name. Each card has a `needs user` line.

## Cards

#### A0: repo metadata
- lane: project
- goal: the GitHub repo says what Aye is and links the site: description "Aye: a small systems language without a GC, built so AI agents can learn it from one command", homepage https://ayelang.dev, topics (`programming-language`, `compiler`, `systems-programming`, `ai-agents`, `llm`, `aye`), Discussions on with the categories Q&A, Ideas, Show and tell, Announcements.
- serves: AD2
- files: none tracked
- verify: `gh repo view aye-lang/aye --json description,homepageUrl,hasDiscussionsEnabled,repositoryTopics` → the values above, `hasDiscussionsEnabled: true`
- depends on: –
- where: local (main context, `gh repo edit`)
- model: haiku
- time box: 10 min
- needs user: no (only if the token lacks admin on the repo)

#### A1: website rewrite
- lane: project
- goal: ayelang.dev tells a stranger in 10 seconds what Aye is, lets them install it in one command, and backs every claim with a link into the repo; nothing on it is stale.
- serves: AD2
- files: `site/public/index.html`, `site/public/style.css`, `site/public/why.html` (new), `site/public/bench.html` (new), `site/samples/*.aye`, `site/README.md`, `site/check.sh` (new)
- verify: `sh site/check.sh` → exit 0 (it runs `aye check` on every `site/samples/*.aye`, fails if `grep -riE 'anyknown|no public repository|not released|no download|pre-release' site/public` matches, and fails if a sample in `site/samples/` differs from its copy in the HTML after unescaping); then `sh site/build.sh && cd site && npx wrangler deploy --dry-run --outdir /tmp/ayelang-dryrun` → exit 0
- depends on: A6 for the real install command (until then the hero shows `cargo install --git https://github.com/aye-lang/aye aye` plus the clang note)
- parallel with: all but A2, A8, A12 (shared `site/`)
- where: local (screenshots of the page in light and dark, at 375 px and 1280 px wide)
- model: opus (copy and design judgment)
- time box: 90 min; split into A1a (index) and A1b (why, bench) if it does not fit
- needs user: no to write it. To deploy before A2: the user runs `cd site && wrangler deploy` once.
- notes: page plan below.

A1 page plan:

- **Hero.** One line: "A small systems language without a garbage
  collector, built so AI agents can learn it from one command." Under
  it: the install command in a copyable block, then "Get started" (the
  tour) and "GitHub" buttons. To the right (below on a phone): a real
  24-line sample from `site/samples/`, the tokenizer that is on the page
  today, shortened if needed, with the `check` / `fix` / `run`
  transcript beside it: the missing import found, fixed by `aye fix`,
  then run. This transcript is the best demo Aye has; keep it.
- **Why Aye.** Three short columns: "One spec, one prompt" (`aye spec`,
  under 30k tokens, every example compiled by the tests); "Errors that
  fix themselves" (codes, `aye explain`, fix-its, `--json`); "No hidden
  costs" (explicit allocators, `defer`, no GC, no exceptions, no
  macros, runtime checks in debug and safe builds). Then a link to
  `why.html`: the design principles and the Go / Zig / Rust / Odin table
  from `docs/design.md`, in English (A16 writes the source text).
- **Install.** Tabs are JavaScript, so instead three short blocks:
  install script (macOS and Linux), Homebrew (after A7), from source
  (`cargo install`). State the clang requirement and how to get it
  (`xcode-select --install`; `apt install clang`). Say plainly: macOS
  and Linux, arm64 and x86_64; wasm32 as a target; no Windows.
- **Five-minute tour.** Four cards, each a 6 to 10 line sample with
  output: hello and `defer`; errors as values with `try`; an allocator
  and a `List`; a C call through `@cimport`. Each links into the full
  tour (A8).
- **Proof.** Three cards with numbers and source links: the agent eval
  (15/15, `docs/aye-eval.md`); Dweller (a screenshot of Dweller rendering
  a real page, re-measured line count, link to aye-lang/dweller); the
  framework and benchmarks (146k vs 70k vs 16.6k req/s, the binary
  sizes, and a "methodology" link to `bench.html`, which states machine,
  tools, commands and the caveats from the audit, and links
  `docs/aye-bench.md`).
- **Docs.** Links: tour, std reference (`/std/`, deployed), language
  reference (`aye spec`; GitHub link to `docs/spec.md`), error index
  (`aye explain`), editor setup, the decision log.
- **Community.** GitHub Discussions, issues, CONTRIBUTING, releases
  feed (`https://github.com/aye-lang/aye/releases.atom`), changelog.
- **Status line** in the footer, not a banner: "Aye is pre-1.0: the
  language and std still change between releases. Releases follow
  semver from 0.1." Plus the license.
- **Remove:** the "Pre-release" badge, "not released yet ... no
  download and no public repository", the roadmap list (move what is
  really open to a short "What is next" line that links `docs/plan.md`),
  every "anyknown", the long "One toolchain" definition list (keep five
  commands in the tour; the rest links to the README reference).
- **Design direction.** Keep what works: static HTML and CSS, no
  JavaScript on content pages, system fonts, the warm off-white and the
  dark palette in `style.css` (define the tokens once on `:root`, dark
  under `prefers-color-scheme`). Add: a two-column hero on wide screens,
  a mono face for code (`ui-monospace` stack), syntax colors for code
  samples done at build time (a small span-per-token pass in
  `site/check.sh` or by hand; no client-side highlighter), one accent
  color, generous whitespace, a max width of about 72ch for prose. No
  stock art, no gradients, no mascots. The logo is the existing
  `favicon.svg` mark.
- **Deploy path.** Today: `wrangler deploy` by hand. After A2: a push
  to main that touches `site/` deploys it.

#### A2: deploy the site from CI
- lane: project
- goal: a push to main that changes `site/` (or `std/`, for the std reference) builds the compiler, runs `site/check.sh` and `site/build.sh`, and deploys with `wrangler deploy`; the live page equals `site/public/index.html`.
- serves: AD2
- files: `.github/workflows/site.yml` (new), `site/README.md`
- verify: after a merge, `gh run list --workflow site.yml --limit 1` → success, and `curl -sSL https://ayelang.dev/ | diff - site/public/index.html` → no output, and `curl -s -o /dev/null -w '%{http_code}' https://ayelang.dev/std/` → 200
- depends on: A1
- where: local
- model: sonnet
- time box: 30 min
- needs user: **yes**: create a Cloudflare API token with "Edit Cloudflare Workers" for the account that owns ayelang.dev, and add it and the account id as repo secrets `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` (`gh secret set`).

#### A3: README for a stranger
- lane: project
- goal: README.md starts with the pitch, a 15-line sample, the install command, links (site, tour, spec, Discussions) and the three proof points; the command and test reference moves to `docs/reference.md` unchanged, linked from the README.
- serves: AD2
- files: `README.md`, `docs/reference.md` (new), `CONTRIBUTING.md` (link fix only)
- verify: `head -40 README.md` contains the pitch line, an ` ```aye ` sample and the install command; `grep -c 'stage0' README.md` → 0; the sample passes `target/release/aye check` when saved to a file
- depends on: A6 (install line; use `cargo install` until then)
- parallel with: all
- where: cloud
- model: opus
- time box: 45 min
- needs user: no

#### A4: `aye --version`
- lane: aye
- goal: `aye --version` and `aye version` print `aye <cargo version> (<git short hash> <date>)`, the hash and date set at build time (a `build.rs` reading `git rev-parse`, falling back to `unknown` outside a checkout); the release job and the install script rely on it.
- serves: AD1
- files: `build.rs` (new), `src/main.rs`, a test in `tests/` for the format
- verify: `cargo test --release --test golden` → exit 0 and `target/release/aye --version` → a line matching `^aye 0\.1\.0 \([0-9a-f]{7,} 20[0-9-]+\)$`
- depends on: –
- where: cloud
- model: sonnet
- time box: 30 min
- needs user: no

#### A5: release workflow and v0.1.0
- lane: project
- goal: pushing a tag `v*` builds `aye` on four runners (macos-15 for `aarch64-apple-darwin`, macos-15 with `--target x86_64-apple-darwin` or macos-13 for x86_64, ubuntu-24.04 for `x86_64-unknown-linux-gnu`, ubuntu-24.04-arm for `aarch64-unknown-linux-gnu`), smoke-tests each (`aye --version`, `aye run tests/cases/hello.aye` where the arch can run), and publishes a GitHub Release with `aye-<version>-<target>.tar.gz`, a `SHA256SUMS` file and notes from `CHANGELOG.md`; then tag v0.1.0.
- serves: AD1, AD5
- files: `.github/workflows/release.yml` (new), `CHANGELOG.md` (A15 seeds it)
- verify: `gh release view v0.1.0 --repo aye-lang/aye --json assets --jq '.assets[].name'` → four tarballs and `SHA256SUMS`; on a Mac: download the arm64 tarball, `shasum -a 256 -c`, `./aye run hello.aye` → `hello`
- depends on: A4, A15
- where: local (the tag push) and CI
- model: sonnet
- time box: 60 min
- needs user: no
- notes: the Linux binaries link glibc of Ubuntu 24.04; say so in the notes (older distros: build from source). Linux arm64 is not in the test CI; add it as a fourth shard later if releases show problems.

#### A6: install script
- lane: project
- goal: `curl -fsSL https://ayelang.dev/install.sh | sh` detects OS and arch, downloads the latest release tarball, checks it against `SHA256SUMS`, installs `aye` to `~/.aye/bin` (or `$AYE_INSTALL`), prints the PATH line to add, and warns with the exact fix when `clang` is missing; POSIX sh, no bash.
- serves: AD1
- files: `site/public/install.sh` (new), `.github/workflows/release.yml` (a job that runs the script from the release on ubuntu-24.04 and macos-15 after publishing), `site/README.md`
- verify: `shellcheck site/public/install.sh` → exit 0; in an Ubuntu 24.04 container with clang: `sh site/public/install.sh && ~/.aye/bin/aye run hello.aye` → `hello`; the release workflow's install job → success
- depends on: A5 (and A2, or a manual deploy, for the URL)
- where: local (container via `scripts/linux.sh`)
- model: sonnet
- time box: 45 min
- needs user: no

#### A7: Homebrew tap
- lane: project
- goal: `brew install aye-lang/tap/aye` installs the release binary on macOS (arm64, x86_64) and Linux; the release workflow updates the formula's version and checksums.
- serves: AD1
- files: new repo `aye-lang/homebrew-tap` (`Formula/aye.rb`), `.github/workflows/release.yml` (a job that opens or pushes the formula bump)
- verify: on a Mac `brew install aye-lang/tap/aye && aye --version` → the release version; `brew test aye` → exit 0 (the formula's test runs `aye run` on a hello file)
- depends on: A5
- where: local
- model: sonnet
- time box: 45 min
- needs user: **maybe**: creating the repo in the org needs org repo-create rights, and the release job needs a token with write access to the tap repo (a fine-grained PAT as secret `TAP_TOKEN`). If the assistant's `gh` token cannot do either, the user does those two clicks.
- notes: on macOS the formula needs no clang dependency (Xcode Command Line Tools); on Linux add `depends_on "llvm"` only if a fresh Linuxbrew lacks clang.

#### A8: the tour
- lane: project
- goal: an 8-page tour on the site that takes someone who knows one C-family language from hello world to a small CLI tool with tests: 1 hello, `let`/`var`, functions; 2 structs, enums, `match`; 3 errors, `try`, `errdefer`; 4 memory: allocators, `defer`, the debug allocator's leak report; 5 slices, strings, `List` and `Map`; 6 modules and std; 7 `test` blocks and `aye test`; 8 C interop with `@cimport`. Each page: prose under 400 words, runnable samples with their output, one exercise.
- serves: AD3
- files: `site/samples/tour/*.aye` (new), `site/public/tour/*.html` (new), `site/check.sh` (samples run and their output matches the page)
- verify: `sh site/check.sh` → exit 0 (now also `aye run` of every tour sample, output compared with `site/samples/tour/*.out`)
- depends on: A1 (layout, `check.sh`)
- parallel with: A9, A10
- where: cloud
- model: opus
- time box: 90 min; split per four pages if needed
- needs user: no

#### A9: examples directory
- lane: project
- goal: `examples/` holds 8 small, commented programs a user copies from: `wc`, `grep`, a JSON pretty-printer, an HTTP JSON API with `std.http`, a framework app (copy of `framework/examples/pages` with a README line), SQLite through `@cimport`, a `wasm32-wasip1` program, a library with `test` blocks; each has a one-paragraph README section and expected output; CI checks and runs them.
- serves: AD3
- files: `examples/**` (new), `examples/README.md` (new), `tests/golden.rs` or a new `tests/examples.rs` (run each with `.out`), `README.md` (one link)
- verify: `cargo test --release --test examples` → exit 0
- depends on: –
- parallel with: all
- where: cloud
- model: sonnet
- time box: 60 min
- needs user: no

#### A10: VS Code extension
- lane: project
- goal: `editors/vscode/` is a VS Code extension: a TextMate grammar for `.aye` (keywords, types, strings, chars, numbers, comments, `@builtins`), language configuration (brackets, comments, indentation), and an LSP client that starts `aye lsp` (setting `aye.path`); the release workflow attaches `aye-vscode-<version>.vsix`.
- serves: AD4
- files: `editors/vscode/**` (new), `.github/workflows/release.yml` (package with `npx @vscode/vsce package`)
- verify: `cd editors/vscode && npm ci && npx @vscode/vsce package` → a `.vsix`; `code --install-extension aye-vscode-*.vsix`, open `site/samples/tokens.aye`, delete its first line → an E0084 diagnostic appears (screenshot)
- depends on: –
- parallel with: all
- where: local (VS Code on the Mac)
- model: sonnet
- time box: 60 min
- needs user: no
- notes: the same grammar file serves later for GitHub highlighting (linguist needs wide use first) and for the site's build-time highlighting (A1).

#### A11: publish the VS Code extension
- lane: project
- goal: the extension is installable by name from the VS Code Marketplace and Open VSX; the release workflow publishes new versions.
- serves: AD4
- files: `.github/workflows/release.yml`
- verify: `npx @vscode/vsce show aye-lang.aye` → the current version; `curl -s https://open-vsx.org/api/aye-lang/aye | grep -o '"version":"[^"]*"'` → the same
- depends on: A10, A5
- where: local
- model: haiku
- time box: 20 min
- needs user: **yes**: create the Marketplace publisher `aye-lang` (Azure DevOps PAT) and an Open VSX account and namespace, and store the tokens as secrets `VSCE_PAT` and `OVSX_PAT`.

#### A12: playground, phase 1 (check, fix, format in the browser)
- lane: aye
- goal: ayelang.dev/play/ is an editor page where `aye check`, `aye fix`, `aye fmt`, `aye explain` and `aye emit-c` run in the browser on the compiler built to wasm, with diagnostics shown inline and shareable links (source in the URL hash); no server.
- serves: AD3
- files: `src/driver.rs` (`#[cfg(unix)]` on the three Unix-only spots; a `cfg(not(unix))` stub that returns an error), `src/play.rs` (new, `#[cfg(target_arch = "wasm32")]` `extern "C"` functions taking source bytes and returning JSON; no new crates), `site/public/play/` (new; plain JS, a `<textarea>` or a small vendored editor), `site/build.sh` (build the wasm), `.github/workflows/site.yml`
- verify: `cargo build --lib --release --target wasm32-unknown-unknown` → exit 0; `cargo test --release` → exit 0; a Node script `node site/play/test.mjs` loads the wasm, checks `site/samples/tokens.aye` minus its first line, and prints the E0084 diagnostic
- depends on: A1, A2
- parallel with: not with A1 or A8 (shared `site/`)
- where: cloud
- model: opus
- time box: 90 min; split A12a (compiler builds to wasm with the API, Node test) and A12b (the page) if needed
- needs user: no

#### A13: playground, phase 2 (run): spike
- lane: aye
- goal: decide, with numbers, how the playground runs programs: measure download size and compile time of a clang built to wasm (for example YoWASP clang or a wasi-sdk clang to wasm build) compiling `aye emit-c --target wasm32-wasip1` output of `tokens.aye` in Node and in Chromium, and run the result with a WASI shim based on `scripts/wasi-run.mjs`; write the decision in this file.
- serves: AD3
- files: this file (decision), a spike branch only
- verify: the decision paragraph names download MB, cold compile ms and warm compile ms, and picks in-browser clang, a server runner, or "check only" with a reason
- depends on: A12
- where: local
- model: opus
- time box: 60 min
- needs user: no for the spike. **Yes** if it picks a server runner (hosting cost and abuse policy).

#### A14: community files
- lane: project
- goal: a stranger knows where to ask, how to report and how to send a first PR: issue templates (bug, with `aye --version` and the smallest program; "an agent wrote Aye the spec did not prepare it for", with the agent's code and `aye check --json` output; feature idea pointing to Discussions), `CODE_OF_CONDUCT.md` (Contributor Covenant 2.1), `SECURITY.md` (report privately through GitHub security advisories), and a CONTRIBUTING.md whose first half is for humans (build, test one suite, the PR flow, what makes a good first PR, where help is wanted) with the agent workflow moved below.
- serves: AD5
- files: `.github/ISSUE_TEMPLATE/*.yml` (new), `.github/ISSUE_TEMPLATE/config.yml` (new; Discussions link), `CODE_OF_CONDUCT.md` (new), `SECURITY.md` (new), `CONTRIBUTING.md`
- verify: `gh api repos/aye-lang/aye/community/profile --jq .health_percentage` → 100
- depends on: A0
- parallel with: all
- where: cloud
- model: sonnet
- time box: 30 min
- needs user: no (the code of conduct names the user's contact email only with their consent; otherwise it points to GitHub's report-abuse flow and Discussions moderators)

#### A15: changelog and news
- lane: project
- goal: `CHANGELOG.md` (Keep a Changelog format) seeded with 0.1.0, written from `docs/plans/done.md` as user-visible changes; the site gets `/news/` with one post per release and an Atom feed `site/public/feed.xml`; the release notes come from the changelog section.
- serves: AD5
- files: `CHANGELOG.md` (new), `site/public/news/` (new), `site/public/feed.xml` (new)
- verify: `xmllint --noout site/public/feed.xml` → exit 0; `grep -c '^## \[0.1.0\]' CHANGELOG.md` → 1
- depends on: A1
- parallel with: all but A1
- where: cloud
- model: sonnet
- time box: 45 min
- needs user: no

#### A16: design rationale in English
- lane: project
- goal: `docs/why.md` is an English version of the principles, positioning and comparison table of `docs/design.md` (Go, Zig, Rust, Odin: what Aye takes and what it leaves), plus "what Aye is not" from its non-goals; A1's `why.html` is built from it. `docs/design.md` stays as it is.
- serves: AD2
- files: `docs/why.md` (new)
- verify: every claim in `docs/why.md` about a feature has an example that passes `aye check` (` ```aye ` blocks checked by a loop in the card's report), and the comparison table has the same rows as the one in `docs/design.md`
- depends on: –
- parallel with: all
- where: cloud
- model: opus
- time box: 45 min
- needs user: no

#### A17: reproducible benchmark
- lane: aye
- goal: a stranger reproduces the headline with one command: `scripts/bench/wrk.sh` (committed; the 2026-10-14 script was not) builds the three servers, runs `wrk -t2 -c64 -d10s --latency` three times each and prints a Markdown table; a `workflow_dispatch` job `bench.yml` runs it on `ubuntu-24.04` and uploads the table, so the site can quote a run anyone can rerun.
- serves: AD2
- files: `scripts/bench/wrk.sh` (new), `.github/workflows/bench.yml` (new), `docs/aye-bench.md` (a new dated section with the GitHub runner numbers, whatever they show)
- verify: `sh scripts/bench/wrk.sh` on the cloud VM → a table with three rows and no socket errors; `gh workflow run bench.yml` then the run → success with the artifact
- depends on: –
- parallel with: all
- where: cloud
- model: sonnet
- time box: 45 min
- needs user: no
- notes: the site quotes whichever numbers this produces, even if Aye's lead shrinks on a 4-vCPU shared runner.

#### A18: launch drafts
- lane: project
- goal: drafts ready for the user to post, each honest about pre-1.0 status and leading with the agent angle: a launch post for `/news/` ("Aye: a systems language an agent learns from one command"), a Show HN title and first comment, an r/ProgrammingLanguages post (design-focused: the spec budget, fix-its, allocators without a borrow checker, what failed), a lobste.rs submission (tags `plt`, `compilers`, `ai`), and a 5-post thread for Bluesky/Mastodon/X; plus a list of likely questions with answers (why not Zig, why C as a backend, why no borrow checker, Windows, stability).
- serves: AD6
- files: `docs/launch/*.md` (new)
- verify: every number in the drafts matches its source file (the card's report lists claim → file:line)
- depends on: the launch gate (A0, A1, A2, A3, A5, A6, A8, A9, A14, A15, A21)
- where: cloud
- model: opus
- time box: 60 min
- needs user: **yes** to post: HN and Reddit under their accounts, lobste.rs needs an invite, social accounts are theirs. The assistant never posts.

#### A19: comparative agent eval
- lane: aye
- goal: the eval harness (`scripts/eval.py`) runs the same five tasks with the same model and limits in Zig and Rust (the agent learns each from its usual docs, with web access off, as for Aye), 3 runs each; the table goes into `docs/aye-eval.md` whatever it shows, and the site and launch posts quote it only if it is a fair comparison.
- serves: AD2
- files: `scripts/eval.py`, `tests/eval/*/check.sh` (language-neutral checks), `docs/aye-eval.md`
- verify: `python3 scripts/eval.py --lang zig --runs 3` and `--lang rust --runs 3` → exit 0 with a results table each
- depends on: –
- where: local (needs `claude -p`)
- model: opus
- time box: 90 min
- needs user: no (about 10 USD of model use, inside the normal budget)

#### A20: adoption numbers
- lane: project
- goal: `scripts/adoption-stats.sh` prints stars, forks, watchers, release downloads per asset, issues and PRs opened by people outside the org, and Discussions threads, from `gh api`; the numbers go into this file under "Numbers" once a week.
- serves: AD6
- files: `scripts/adoption-stats.sh` (new), this file
- verify: `sh scripts/adoption-stats.sh` → exit 0 and a table
- depends on: A5
- where: local
- model: haiku
- time box: 20 min
- needs user: no for GitHub numbers. **Yes** for site visits: turn on Cloudflare Web Analytics for ayelang.dev (no cookies; one dashboard toggle).

#### A21: fresh-eyes run
- lane: project
- goal: a fresh agent session with no repo access, acting as a newcomer, starts at https://ayelang.dev, installs Aye in a clean Ubuntu 24.04 container and on the Mac, follows the tour, and builds a small CLI tool; it reports every point where it was stuck, confused, or met a stale or wrong fact, with the minutes it took to the first `aye run`.
- serves: AD1, AD3
- files: this file (the findings, as new cards)
- verify: the report shows time to first `aye run` under 5 minutes on both systems and no stale fact; every finding is a card or a fix
- depends on: A1, A2, A6, A8
- where: local
- model: sonnet
- time box: 45 min
- needs user: no

## Done criteria

- **AD1** A stranger on macOS or Linux (arm64 or x86_64) goes from the
  site to `aye run hello.aye` printing `hello` in under 5 minutes with no
  Rust toolchain: install script or Homebrew, checked by CI after each
  release (A6) and by A21.
- **AD2** The site and README state the pitch above the fold, every
  number on them links to the file it comes from, and `site/check.sh`
  finds no stale phrase; the live site equals `main` (A2).
- **AD3** The tour and examples teach enough to write a CLI tool and an
  HTTP service; every sample on the site and in `examples/` is checked
  and run in CI.
- **AD4** The VS Code extension installs by name and shows highlighting
  and `aye lsp` diagnostics.
- **AD5** Releases with notes, a changelog, a feed, Discussions and
  issue templates exist; community profile health 100%.
- **AD6** Outcome, not a gate: the first issue or PR from someone outside
  the org, and a weekly number trend (A20). A launch (A18) happens only
  after the launch gate.

## Order

1. Now, in parallel: A0 (main context), A4, A9, A10, A14, A16, A17.
2. Then: A1 (needs A16 for `why.html`), A15, A5 (needs A4, A15).
3. Then: A2 (needs the user's Cloudflare secret), A6, A3, A7, A8.
4. Then: A12, A21, A19, A20; A13 after A12.
5. Launch: A18 drafts once the launch gate holds; the user posts; A11
   when the user has the publisher accounts.

## Numbers

None yet (A20).
