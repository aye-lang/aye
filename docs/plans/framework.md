# Plan: the framework lane

The framework lane of [`docs/plan.md`](../plan.md): a web server
framework written in Ore, in the spirit of Next.js, under `framework/`
in this repo (new). Rows use the prefix `F`. Most early steps wait on
the Ore lane ([`ore.md`](ore.md)); gaps found here go to
`browser/COMPILER_GAPS.md` with **Hit by** naming the F step.

## What "done" means

- FD1: routing from the file tree: a tree under the app's routes
  directory (layout decided in F0) maps to URL paths, with dynamic
  segments and a catch-all; an unknown path gets the app's 404 page.
- FD2: server-side rendering: a page route returns HTML built on the
  server; text is escaped unless the code marks it raw.
- FD3: API routes: a handler per method returns JSON (`std.json`) and
  reads the query, a JSON or form body and cookies; another method gets
  405.
- FD4: static assets: files under the app's public directory are served
  with the right `Content-Type`, `ETag` and 304 answers, and no path
  leaves the directory; the production build gives them content-hashed
  names.
- FD5: dev server: one command watches the app, regenerates the routes,
  rebuilds with `ore build`, restarts the server and reloads open pages;
  a compile error shows in the page (from `ore check --json`).
- FD6: client code: an interactive part of a page (an "island") runs in
  the browser, written in Ore and compiled to wasm32-freestanding with
  generated JS glue, or written in JS and bundled (F0 decides which comes
  first).
- FD7: production build: one command makes one server executable (assets
  embedded with `@embedFile` or next to it) that runs without the Ore
  toolchain.
- FD8: a hello page's requests per second are recorded in
  `docs/ore-bench.md` next to `std.http`'s own number (O12).
- FD9: an example app (a blog: list and detail pages, an API route, a
  form POST, one island) passes its scripted checks, and the anyknown
  browser shows its text (`anyknown dump --md`).

## Open choices (answered in F0)

Answered in [`docs/framework.md`](../framework.md) (F0.1).

1. **Route layout.** In Ore a directory is a module and the files of a
   directory share one namespace (gap #26: a top-level name in one file
   blocks locals in every file). So "one file per route" either means a
   directory per route (`routes/blog/slug/` with `pub fn get`), or the
   framework generates code that renames per file. Dynamic segments
   (`[slug]`) are not valid module names in either case.
2. **Wiring routes.** Ore has no reflection and no loading of code at
   run time, so the route table is generated Ore (scan the tree, write a
   module that imports every route and dispatches) and then built, or
   written by hand. Generated is the Next.js experience.
3. **Templates.** Ore has no macros and no JSX. Options: builder
   functions in plain Ore (`h.div(...)`), template files compiled to Ore
   by the framework (like Go's templ), or templates read at run time.
   Compiled templates keep type checking and `ore check` errors; builder
   functions need no tool.
4. **Server model.** A thread per connection (`std.thread`, what O1.4
   starts with), a `poll` loop, or `std.coro` coroutines per request.
5. **Client code.** Ore → wasm32 islands need O2.4; JS needs either
   no bundler (serve ES modules as they are; every browser loads them) or
   a bundler.
6. **Reuse of the browser's JS parser for a bundler.** Recommendation:
   reuse, later, not now. Reuse because one conformant JS parser is
   cheaper to keep than two: `browser/js/lexer.ore`, `parser.ore` and
   `ast.ore` import only std, and the AST keeps source spans
   (`Span`, `browser/js/ast.ore:226`), which import/export rewriting
   needs. Not now because (a) the first slices need no bundler (wasm
   islands, or unbundled ES modules), (b) `framework/` cannot import
   `browser/js/` today (imports resolve under the root module's
   directory; O4.2 adds path dependencies), and (c) `js` is one module
   with the interpreter and GC, so the parser files would first move into
   a submodule of their own, a browser-lane step. A copy would fork the
   parser and is ruled out.
7. **Dev reload.** Restart the server process on each rebuild (simple,
   Ore compiles ahead of time) and reload pages through server-sent
   events (no WebSocket server needed). The anyknown browser has no
   `EventSource` and no `WebAssembly` today, so dev reload and islands
   are tested with Node 22 and checked by hand in Chrome; adding them to
   the browser is a browser-lane step, not a framework dependency.
8. **The command and how an app depends on the framework.** Name of the
   CLI; until O4.2 the example app lives inside `framework/` as part of
   its root.
9. **`docs/design.md` non-goal.** It says Ore does not replace
   JavaScript and writes only the engine ("不取代 JavaScript"). Ore code
   running in web pages through wasm changes that; F0 proposes new
   wording for the user to approve.

## Phases

| Phase | Goal | Exit check |
|---|---|---|
| F0 | Design | `docs/framework.md` answers the open choices; the user approved it |
| F1 | First vertical slice: a hello-world route served by an Ore HTTP server | `curl` gets the page from the built app |
| F2 | Routing, pages, API routes, static files (FD1–FD4) | example app checks |
| F3 | Dev server (FD5) | edit → new response, scripted |
| F4 | Client code (FD6) | an island works under Node 22 and in Chrome |
| F5 | Production build, numbers, the example app (FD7–FD9) | FD7–FD9 checks |

F0 can run now. F1 waits on O1.4; F2 on O1.5 and O3.1; F3 on O1.6 and
O3.6; F4 on O2.4 (wasm islands) or O4.2 (a bundler reusing the browser's
parser).

## Steps

∥ = can run at the same time. F0 is docs-only and needs no build slot.

### F0: Design

| id | Goal | Files | Verify | Deps | ∥ |
|---|---|---|---|---|---|
| F0.1 | Design doc answering the open choices above with tradeoffs, a sketch of an app's tree and of a route's code, and the list of Ore-lane steps each phase needs. Done 2026-10-08: [`docs/framework.md`](../framework.md), waiting for the user's approval | `docs/framework.md` (new) | read by the user; every Ore step it names exists in `ore.md` | – | all |

### F1: First vertical slice

| id | Goal | Files | Verify | Deps | ∥ |
|---|---|---|---|---|---|
| F1.1 | `framework/` skeleton: the `web` module (request, response, `Html`, a route table with literal segments, `web.run` over `std.http`) and `framework/examples/hello/` with a hand-written route table (a `web` symlink until O4.2); a test that builds it, starts it on a free port and checks the response (`docs/framework.md`, "F1") | `framework/web/`, `framework/examples/hello/` (new), `tests/framework.rs`, `tests/framework/hello/*.http` | `cargo test --test framework`: `200` and the page for `/`, `404` for `/nope` | F0.1, O1.4 | O-lane steps |
| F1.2 | Route generation: `web gen <app>` scans `app/` (Next.js names, `[slug]`) and writes the build root `.web/` (symlinks, router, `main.ore`); `ore build .web`; two static routes and the 404 page | `framework/cli/` (new), `framework/examples/hello/` | test: `/`, `/about`, an unknown path | F1.1, O1.5 | O-lane steps |
| F1.3 | One API route returning JSON with a dynamic segment (`app/api/hello/[name]/route.ore`) | `framework/web/`, `framework/examples/hello/` | transcript checks status, `Content-Type` and body; `405` for `POST` | F1.2, O3.1 | O-lane steps |

Later phases get their rows when F0 is approved.
