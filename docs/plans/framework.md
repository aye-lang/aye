# Plan: the framework lane

The framework lane of [`docs/plan.md`](../plan.md): a web framework in
Aye in the spirit of Next.js, under `framework/` (new). **Only F1.1 is
active (2026-10-08):** the hello-world route is the acceptance test of
`std.http`. Everything after it is paused. The design is
[`docs/framework.md`](../framework.md) (F0.1, it answers the open
choices); gaps go to `browser/COMPILER_GAPS.md` with the F step under
**Hit by**. Finished steps: [`done.md`](done.md).

## Done criteria

- FD1: routing from the file tree: a tree under the app's routes
  directory maps to URL paths, with dynamic segments and a catch-all; an
  unknown path gets the app's 404 page.
- FD2: server-side rendering: a page route returns HTML built on the
  server; text is escaped unless the code marks it raw.
- FD3: API routes: a handler per method returns JSON (`std.json`) and
  reads the query, a JSON or form body and cookies; another method gets
  405.
- FD4: static assets under the app's public directory, with the right
  `Content-Type`, `ETag` and 304 answers; no path leaves the directory;
  content-hashed names in the production build.
- FD5: dev server: one command watches the app, regenerates routes,
  rebuilds with `aye build`, restarts the server and reloads open pages;
  a compile error shows in the page (from `aye check --json`).
- FD6: client code: an island runs in the browser, written in Aye for
  wasm32-freestanding with generated JS glue, or in JS and bundled.
- FD7: production build: one command makes one server executable that
  runs without the Aye toolchain.
- FD8: a hello page's requests per second in `docs/aye-bench.md` next
  to `std.http`'s own number (O12).
- FD9: an example app (a blog: list and detail pages, an API route, a
  form POST, one island) passes its scripted checks, and the anyknown
  browser shows its text (`anyknown dump --md`).

Each FDn is checked by the test its phase adds to
`cargo test --release --test framework`. F1.1 adds that test and the
first part of FD1/FD2 (literal routes, a page, the 404).

## Current state (2026-10-08)

- `framework/` and `tests/framework.rs` do not exist yet.
- F0.1 design merged (0124e64); the user has not yet answered its three
  questions (`docs/framework.md`, "Questions for the user": names, the
  `design.md` wording, TypeScript). F1.1 does not depend on them.
- F1.1 waits on O1.4 and O1.4a (`std.http`).

## Phases

| Phase | Goal | Exit check | Status |
|---|---|---|---|
| F0 | Design | `docs/framework.md` answers the open choices | done (user questions open) |
| F1 | Hello-world route served by an Aye HTTP server | `cargo test --release --test framework` | current: F1.1 only |
| F2 | Routing, pages, API routes, static files (FD1–FD4) | example app checks | paused |
| F3 | Dev server (FD5) | edit → new response, scripted | paused |
| F4 | Client code (FD6) | an island under Node 22 and in Chrome | paused |
| F5 | Production build, numbers, the example app (FD7–FD9) | FD7–FD9 checks | paused |

## Cards: F1

#### F1.1: framework skeleton and the hello app
- lane: framework
- goal: the `web` module (request, response, `Html`, a route table with literal segments, `web.run` over `std.http`) and `framework/examples/hello/` with a hand-written route table, served and checked by a test.
- serves: FD1, FD2 (literal routes and the 404); acceptance test of `std.http` (O6)
- files: `framework/web/` (new), `framework/examples/hello/` (new, `web` is a symlink to `../../web` until O4.2), `tests/framework.rs` (new), `tests/framework/hello/root.http`, `tests/framework/hello/missing.http` (new)
- verify: `cargo test --release --test framework` → exit 0: `200` and the page for `/`, `404` for `/nope`
- depends on: O1.4a
- parallel with: Aye-lane steps that do not change `std/http/`
- where: cloud
- model: sonnet
- time box: 45 min
- notes: the exact files, commands and expected `curl -i` output are in `docs/framework.md`, "F1: the first vertical slice". Gaps it meets (context pointer for the handler, port 0, `Arena.reset`) go to `browser/COMPILER_GAPS.md`.

## Backlog

Paused; cards get written if the framework lane is resumed. Source in
brackets.

- F1.2 `web gen <app>` scans `app/` (Next.js names, `[slug]`) and writes `.web/`; `/`, `/about`, unknown path (needs O1.5, symlinks) (old F1 table; `docs/framework.md` F1).
- F1.3 one API route returning JSON with a dynamic segment, 405 for POST (needs O3.1) (old F1 table).
- F2–F5 rows: written when the lane resumes; the Aye steps each phase needs are in `docs/framework.md`, "Aye-lane steps by phase".
- Islands and dev reload in the anyknown browser need `WebAssembly` and `EventSource` there (neither exists); until then the framework tests with Node 22 and Chrome (`docs/framework.md` §5, §7).
