# ayelang.dev

The language's website: static files in `public/`, served by a Cloudflare
Worker with static assets (`wrangler.toml`). Plain HTML and CSS, no
JavaScript. One build step, `build.sh`, writes the std reference into
`public/std/` (not committed).

## Build

```sh
cargo build --release     # once, from the repo root
site/build.sh             # runs `aye doc --html std -o site/public/std`
```

`wrangler deploy` and `wrangler dev` run `build.sh` themselves (`[build]` in
`wrangler.toml`), so `/std/` is always deployed; they need the compiler built
first. `AYE=<path>` picks another compiler than `target/release/aye`.

## Check

```sh
sh site/check.sh
```

Exit 0 means: every `samples/*.aye` passes `aye check`; every code block in
`public/index.html` equals its sample and every sample is on the page
(`highlight.py --check`); and no stale wording (the old browser name, "no public
repository", "pre-release", ...) is left in `public/`.

## Preview

```sh
cd site/public && python3 -m http.server 8000     # or: cd site && npx wrangler dev
```

## Deploy

```sh
sh site/check.sh
cd site && wrangler deploy --dry-run --outdir /tmp/ayelang-dryrun   # check, uploads nothing
cd site && wrangler deploy
```

`wrangler.toml` binds the Worker to `ayelang.dev` and `www.ayelang.dev` as
custom domains.

## Samples and syntax colors

Every program on the page is a file in `samples/`. In `public/index.html` a
sample is an empty `<pre class="code" data-sample="NAME.aye"><code></code></pre>`
block (or one already filled); `python3 site/highlight.py` fills every such
block from its file, HTML-escaped and colored with `<span>`s (the page has no
JavaScript). So: edit the sample, run `highlight.py`, run `check.sh`. The
"prints ..." lines next to the samples are written by hand: rerun
`aye run samples/NAME.aye` when a sample changes.

The terminal transcript in "Why Aye" was made by deleting the first line of
`sum.aye` and running `aye check`, `aye fix`, `aye check` and `aye run` in the
file's directory.

## Install section

Until the install script (card A6 in `docs/plans/adoption.md`) is released,
the page shows the from-source path (`cargo install --git ...`). The spot for
the one-line installer is the `INSTALL SCRIPT (card A6)` comment in
`public/index.html`.
