# Plan: the framework lane

The framework lane of [`docs/plan.md`](../plan.md): a web framework in
Aye in the spirit of Next.js, under `framework/` (new). **F1.1 is done
(2026-10-10, ef6f47b):** the hello-world route was the acceptance test of
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

- `framework/web/` (request, response, `Html`, literal routes, `web.run`), `framework/examples/hello/` and `tests/framework.rs` exist (F1.1, ef6f47b).
- F0.1 design merged (0124e64); the user has not yet answered its three
  questions (`docs/framework.md`, "Questions for the user": names, the
  `design.md` wording, TypeScript). F1.1 does not depend on them.
- F1.1 needed no compiler gap; `Arena.reset` was not needed (`std.http` makes an arena per request).

## Phases

| Phase | Goal | Exit check | Status |
|---|---|---|---|
| F0 | Design | `docs/framework.md` answers the open choices | done (user questions open) |
| F1 | Hello-world route served by an Aye HTTP server | `cargo test --release --test framework` | done (F1.1, 2026-10-10) |
| F2 | Routing, pages, API routes, static files (FD1–FD4) | example app checks | resumed 2026-10-11: F1.1b/c in flight, then F1.2, F1.3 |
| F3 | Dev server (FD5) | edit → new response, scripted | paused |
| F4 | Client code (FD6) | an island under Node 22 and in Chrome | paused |
| F5 | Production build, numbers, the example app (FD7–FD9) | FD7–FD9 checks | paused |

## Cards: F1

None open; F1.1 is in `done.md`.

## Backlog

Paused; cards get written if the framework lane is resumed. Source in
brackets.

- F1.1b 405 with an `Allow` header for a known path and the wrong method (`docs/framework.md` §2); verify: a `POST /` case in `tests/framework.rs`.
- F1.1c `req.header(name)` and `HEAD` answered from `get` (`docs/framework.md` §2); verify: a `HEAD /` case in `tests/framework.rs`.
- F1.2 `web gen <app>` scans `app/` (Next.js names, `[slug]`) and writes `.web/`; `/`, `/about`, unknown path (needs O1.5, symlinks) (old F1 table; `docs/framework.md` F1).
- F1.3 one API route returning JSON with a dynamic segment, 405 for POST (needs O3.1) (old F1 table).
- F2–F5 rows: written when the lane resumes; the Aye steps each phase needs are in `docs/framework.md`, "Aye-lane steps by phase".
- Islands and dev reload in the anyknown browser need `WebAssembly` and `EventSource` there (neither exists); until then the framework tests with Node 22 and Chrome (`docs/framework.md` §5, §7).
