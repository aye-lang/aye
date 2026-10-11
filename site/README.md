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

Run it before a preview or a deploy: `public/index.html` links to `std/`.
`AYE=<path>` picks another compiler than `target/release/aye`.

## Preview

Open `public/index.html` in a browser, or serve it the way Cloudflare will:

```sh
cd site && npx wrangler dev
```

## Deploy

```sh
cd site && wrangler deploy --dry-run --outdir /tmp/ayelang-dryrun   # check, uploads nothing
cd site && wrangler deploy
```

`wrangler.toml` binds the Worker to `ayelang.dev` and `www.ayelang.dev` as
custom domains.

## Samples

Every program shown on the page is a copy of a file in `samples/`, and each
must pass the checker (exit 0):

```sh
target/release/aye check site/samples/tokens.aye
```

When a sample changes, update the copy in `public/index.html` (HTML-escape
`<`, `>` and `&`). The terminal transcript under it was made by deleting the
first line of `tokens.aye` and running `check`, `fix`, `check` and `run`.
