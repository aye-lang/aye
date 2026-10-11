# Plan: the framework lane

The framework lane of [`docs/plan.md`](../plan.md): a web framework in
Aye in the spirit of Next.js, under `framework/` (new). **F1.1 is done
(2026-10-10, ef6f47b):** the hello-world route was the acceptance test of
`std.http`. Everything after it is paused. The design is
[`docs/framework.md`](../framework.md) (F0.1, it answers the open
choices); gaps go to `browser/COMPILER_GAPS.md` (in aye-lang/anyknown) with the F step under
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
| F2 | Routing, pages, API routes, static files (FD1–FD4) | example app checks | resumed 2026-10-11: F1.1b/c, F1.2, F1.3, F2.1, F2.2 done; F1.2a, F1.3a, F2.2a, F2.1a done; F2 complete except FD7 `@embedFile` (production) |
| F3 | Dev server (FD5) | edit → new response, scripted | F3.1–F3.7 and F2.3 done 2026-10-11 (Chrome by hand still open); next F4/F5 cards |
| F4 | Client code (FD6) | an island under Node 22 and in Chrome | cards written (F4.1 to F4.5; Chrome by hand) |
| F5 | Production build, numbers, the example app (FD7–FD9) | FD7–FD9 checks | cards written (F5.1 to F5.9; F5.3 waits for O1.23) |

## Cards: F1

None open; F1.1 is in `done.md`.

## Cards: F3

F3 is the dev server (FD5), as `docs/framework.md` §5 and the phases
table say. That document puts forms, redirects and cookies in the
request/response API (§2, F2) and the posts app in FD9 (F5), and it has
no design for sessions. So: F2.3 below finishes the F2 API (it runs
first and nothing in F3.x waits for it); F3.1 to F3.7 are the dev
server; sessions and the posts app are in the Backlog. The one Aye-lane
need is O3.6b (see "F3 Aye-lane needs").

All cards edit `tests/framework.rs`, so two of them do not run in one
checkout at a time; in separate worktrees the main context merges that
file by hand. Tests run only the named case (`cargo test --release
--test framework <case>`); the full suite runs once on main.

#### F2.3: form POST, redirect and `Set-Cookie`
- lane: framework
- goal: `Response.redirect(url, status)` and `Response.setCookie(name, value)` exist, and a form POST to the pages example answers 303 with `Location` and `Set-Cookie`, and the page it points to shows what was posted.
- serves: FD3, FD9 (the form POST part)
- files: `framework/web/response.aye`, `framework/examples/pages/app/guestbook/` (new: `page.aye` lists the entries and the `last` cookie; a `route.aye` or `page.aye` POST adds one, per what `web gen` supports), `tests/framework.rs`, `tests/framework/pages/guestbook_*.http` (new)
- verify: `cargo test --release --test framework pages` → exit 0; new cases: `POST /guestbook` with `name=Ann+B` gives `303`, `Location: /guestbook?added=1`, `Set-Cookie: last=Ann%20B; Path=/; HttpOnly; SameSite=Lax`; `GET /guestbook?added=1` with that `Cookie` shows the entry, the escaped name and the notice from `req.query`
- depends on: –
- parallel with: F3.3 (not with the other F3.x on `tests/framework.rs`)
- where: local (the macOS build of the example)
- model: sonnet
- time box: 90 min
- notes: Entries live in a module-level `var` behind `std.sync.Mutex` (`docs/framework.md` §4, §6). §2 names `redirect(url, status)` and `setCookie` but no attributes; use the ones in the verify line and no options. `res.header("Set-Cookie", ...)` already allows repeats. Percent-encode the cookie value in `web` (std has only `percentDecode`).

#### F3.1: `web dev` starts the app
- lane: framework
- goal: `aye run framework/dev -- <app> --port N` generates `.web/`, checks and builds it, starts `.web/server --port N` and prints the server's `listening on 127.0.0.1:N` line.
- serves: FD5
- files: `framework/dev/main.aye` (new), `tests/framework.rs`
- verify: `cargo test --release --test framework dev_start` → exit 0; the test copies `framework/examples/pages` to a temp directory, runs the CLI with a free port, reads the `listening on` line, and `GET /about` equals `about.http`
- depends on: –
- parallel with: O3.6b, F3.3
- where: local (spawns `aye run` and `aye build`; the macOS link cost)
- model: sonnet
- time box: 120 min
- notes: Run gen as `aye run framework/gen -- <app> --web framework/web` through `std.process.spawn` (making gen importable is out of scope; each restart pays its compile, accept it). The `aye` binary is `$AYE` (default `aye`; the test sets it to `CARGO_BIN_EXE_aye`). After the first line, a thread copies the server's remaining output to the CLI's stdout so the pipe never fills. The test starts the CLI in its own process group and kills the group in `Drop`; otherwise a killed CLI leaves its server running.

#### F3.2: watch and restart
- lane: framework
- goal: the CLI polls the mtimes of `app/`, the other linked directories and `public/` every 250 ms; a changed `.aye` file stops the server and goes through generate, check, build, start again on the same port; a change in `public/` restarts nothing.
- serves: FD5
- files: `framework/dev/watch.aye` (new: `snapshot(alloc, dirs) -> Snapshot`, `changed(a, b) -> Kind` of `None | Reload | Restart`, with `test` blocks), `framework/dev/main.aye`, `tests/framework.rs`
- verify: `cargo test --release --test framework dev_restart` → exit 0; the test edits the page text in `app/about/page.aye` of its temp copy and polls (every 200 ms, at most 60 s) until `GET /about` returns the new text on the same port, then rewrites `public/style.css` and sees `GET /style.css` change with no restart (the CLI prints `restart` per restart; the test counts the lines); `aye test framework/dev` → exit 0
- depends on: F3.1
- parallel with: F3.3
- where: local
- model: sonnet
- time box: 120 min
- notes: Uses `std.fs.readDir`, `std.fs.mtime` and `std.time.sleep`; `std.net.listen` sets `SO_REUSEADDR`, so the port comes back at once. Stop is `Child.kill` (SIGTERM) then `wait`. A change is an added, removed or newer-mtime path; ignore `.web/`. Linked directories: the ones `web gen` links at the top of `.web/`.

#### F3.3: diagnostics to an error page
- lane: framework
- goal: a function turns the `diagnostics` array of `aye check --json .web` into an HTML error page whose paths point at `app/`, not at `.web/` symlinks.
- serves: FD5
- files: `framework/dev/diag.aye` (new: `errorPage(alloc, json: []u8, appDir: []u8) -> Error![]u8` with `test` blocks), `tests/framework.rs` (a `dev_units` test that runs `aye test framework/dev` as `web_units` does for `web`), `tests/framework/dev/` (new: a saved `--json` output with `.web/` paths and the expected page)
- verify: `cargo test --release --test framework dev_units` → exit 0; the saved output with two errors gives a page that has both messages with `app/posts/[slug]/page.aye:LINE:COL` and escaped text
- depends on: –
- parallel with: F3.1, F3.2, F2.3 (different files except `tests/framework.rs`)
- where: local
- model: sonnet
- time box: 90 min
- notes: Take the JSON shape from `aye check --json` on a broken copy of the pages example and save it; use `std.json` and `web.Html` for escaping. The mapping is no. 2 of "Foreseen language and std needs" in `docs/framework.md`: the loader names symlink paths, so the CLI maps them back through the links `web gen` wrote. A diagnostic outside `app/` keeps its path.

#### F3.4: a broken build is served and recovers
- lane: framework
- goal: when `aye check --json` fails, the CLI itself listens on the port and answers every request with the error page (status 500), keeps polling, and on the next change that builds, hands the port back to the new server.
- serves: FD5
- files: `framework/dev/main.aye`, `tests/framework.rs`, `tests/framework/dev/` (the expected page transcript)
- verify: `cargo test --release --test framework dev_broken` → exit 0; the test starts the CLI on a temp copy whose `app/about/page.aye` has a syntax error: `GET /about` is `500` with the file, line and message; the test fixes the file and polls (at most 60 s) until `GET /about` is the page again; it breaks the file a second time while the server runs and sees the error page again
- depends on: F3.2, F3.3
- parallel with: –
- where: local
- model: sonnet
- time box: 120 min
- notes: The CLI serves with `std.http` (`listen`, `run` on a thread, `stop`) and stops it before it starts the app. The status is not in `docs/framework.md`; use 500 with `Content-Type: text/html; charset=utf-8`. If `aye build` fails after `check` passed (a link error), show its stderr in the same page.

#### F3.5: dev mode in the app server
- lane: framework
- goal: the server started with `--dev` adds `<script src="/_web/dev.js"></script>` to every HTML page, serves `/_web/dev.js`, and answers `/_web/events` with a server-sent event stream that sends the build id first.
- serves: FD5
- files: `framework/web/serve.aye`, `framework/web/dev.aye` (new: the script text, the injection, the events route), `tests/framework.rs`, `tests/framework/pages/dev_*.http` (new)
- verify: `cargo test --release --test framework dev_script` → exit 0; the test starts the built pages example with `--port 0 --dev`: `GET /about` has the script tag once, before `</body>`; `GET /_web/dev.js` is `200 text/javascript`; a raw TCP `GET /_web/events` returns `200`, `Content-Type: text/event-stream` and `data: <id>\n\n` within 2 s, the same id on a second connection and another id after a second server start; without `--dev` both paths are 404
- depends on: O3.6b
- parallel with: F3.1 to F3.4 (different files; `tests/framework.rs` merges by hand)
- where: local
- model: sonnet
- time box: 120 min
- notes: Build id = the server's start time in ms (`std.time.now`), read once at start. `dev.js` opens `new EventSource("/_web/events")`, remembers the first id and calls `location.reload()` when a later message has another id. The injection point is not in `docs/framework.md`: before `</body>` when present, else at the end. `public/` is served from disk already, so dev mode changes nothing for static files.

#### F3.6: the CLI answers the dev endpoints while the build is broken
- lane: framework
- goal: the error page of F3.4 carries the same script, and the CLI serves `/_web/dev.js` and `/_web/events` with its own ids, so an open page shows the error and reloads when the build is fixed.
- serves: FD5
- files: `framework/dev/main.aye`, `framework/dev/diag.aye` (script tag in the page), `tests/framework.rs`
- verify: `cargo test --release --test framework dev_reload_cycle` → exit 0; with the app broken, a raw `GET /_web/events` returns `text/event-stream` and an id; the test fixes the file and the reconnect yields an event with a different id (the new server's) within 60 s
- depends on: F3.4, F3.5
- parallel with: –
- where: local
- model: sonnet
- time box: 90 min
- notes: One source for the script text (F3.5's, imported). The CLI's id is the time its error page was built, so each new error also reloads open pages. The CLI stops its server (`Server.stop`, which closes the event streams) before it starts the app server; EventSource then reconnects to the app.

#### F3.7: reload under Node 22
- lane: framework
- goal: a Node 22 script plays the browser: it follows `/_web/events`, and an edit of a page gives a reload event and the new page text, through a restart and through a broken build.
- serves: FD5
- files: `tests/framework/dev_reload.mjs` (new), `tests/framework.rs` (runs `node` on it; the test is skipped with a message when Node is older than 22)
- verify: `cargo test --release --test framework dev_reload` → exit 0; the script prints `reload`, the new text, `error page`, `reload`, the fixed text, in that order
- depends on: F3.6
- parallel with: –
- where: local
- model: sonnet
- time box: 90 min
- notes: Node 22 has no `EventSource` without a flag; read `fetch` with `response.body`, split on blank lines, and reconnect in a loop as a browser does. Chrome by hand goes in the report, not in the test (`docs/framework.md` §7).

## F3 Aye-lane needs

Checked with grep of `std/http/http.aye`, `std/crypto/crypto.aye`,
`std/process`, `std/fs` and `std/net`:

- **Needed, missing:** a held-open response for server-sent events.
  `std.http` writes a `Response` with a `Content-Length` after the
  handler returns, and a handler that blocks holds up the other
  connections of its worker thread. Written as O3.6b in `plans/aye.md`;
  F3.5 waits for it.
- **Exists, no card needed:** `std.process` `spawn`/`kill`/`wait` (O1.6);
  `std.fs` `readDir`, `stat`, `mtime`; `std.time.sleep` and `now`;
  `std.json` (O3.1); `SO_REUSEADDR` in `std.net.listen`; `aye check
  --json`; `std.crypto` `hmacSha256`, `constantTimeEqual` and
  `randomBytes` (a signed cookie needs nothing else); the 303 and other
  redirect reasons in `std.http.reason`. A `Set-Cookie` header is an
  ordinary header (`Response.headers` is a list).
- **Not needed in F3:** signal handling (gap no. 10) and graceful stop.
  A restart kills the child with SIGTERM and EventSource reconnects; a
  CLI stopped from a terminal takes its server with it because they
  share the process group. `Child.tryWait` and a working directory for
  `spawn` (O1.6a) are not needed: the CLI passes absolute paths and
  waits only after `kill`.

## Cards: F4

F4 is client code (FD6), as the phases table and `docs/framework.md`
("Client code (F4)") say; the production build is F5. FD6 allows "an
island in Aye or in JS and bundled"; F4 does the Aye island, because the
posts app (FD9) needs one and everything it needs exists (O2.1 to O2.4,
O2.7). The bundler and TypeScript wait for the browser-lane parser split
(Backlog). Plain JS in `public/` already works (F2.1).

The shape, from the doc: an island is a directory built twice, natively
for SSR and for wasm32-freestanding for the browser; the page writes
`web.island(h, "counter", props)`; a script loads the `.wasm` and calls
its `mount`. The doc leaves five things open; the cards choose, and each
choice is in the card that makes it:

- **No `web` in the island.** `web` imports `std.http`, which
  wasm32-freestanding lacks (E0118 at the import), so an island module
  cannot import `web`; `render` writes into a `std.text.Builder`, not a
  `web.Html` (F4.1).
- **Two modules, not one.** A body-less `extern fn` (a wasm import) in a
  module the native build compiles is an undefined symbol at link time,
  and the target-OS constant (O1.14) does not exist. So the island
  directory has `view/` (std only: props, `render`, shared text; both
  builds use it) and `client.aye` in the directory itself (the externs
  and exports; only the wasm build, rooted at the island directory,
  compiles it) (F4.1).
- **Props are JSON text** (`std.json` runs on freestanding, O2.7) in a
  `data-props` attribute (F4.3).
- **The DOM contract** is three imports and two exports (F4.2).
- **Where the files go:** `gen` builds `islands/<name>/` into
  `.web/client/<name>.wasm`; the server serves that directory under
  `/_web/islands/` and the glue at `/_web/islands.js` (F4.4).

All cards edit `tests/framework.rs` (merge by hand in separate
worktrees) and run only the named case.

#### F4.1: the counter island on both targets
- lane: framework
- goal: `framework/examples/pages/islands/counter/` has `view/view.aye` (props from JSON, `render(b, props)` writing the button and the count, `label(n)`) and `client.aye` (`dom_*` imports, exports `mount` and `event`), the native side passes its `test` blocks and `aye build --target wasm32-freestanding` of the directory gives a module with exactly the agreed imports and exports.
- serves: FD6
- files: `framework/examples/pages/islands/counter/view/view.aye` (new, with `test` blocks), `framework/examples/pages/islands/counter/client.aye` (new), `tests/framework.rs` (`island_build`), `tests/framework/island_exports.mjs` (new: lists a module's imports and exports with `WebAssembly.Module.imports/exports`)
- verify: `aye test framework/examples/pages/islands/counter/view` → exit 0, and `cargo test --release --test framework island_build` → exit 0; the test builds the directory for wasm32-freestanding and the script finds the exports `mount`, `event`, `memory`, `malloc` and the imports `dom_query`, `dom_set_text`, `dom_on` (plus the runtime's own `env.aye_write`, `env.aye_abort`) and no other
- depends on: –
- parallel with: F5.1, F5.2 (not on `tests/framework.rs`)
- where: local (wasm32 needs the Homebrew LLVM setup of O0.1)
- model: sonnet
- time box: 90 min
- notes: Contract: `dom_query(root: i32, sel: [*]u8, len: usize) i32` (0 = none), `dom_set_text(el: i32, s: [*]u8, len: usize)`, `dom_on(el: i32, name: [*]u8, len: usize, id: i32)`; JS calls back `event(id: i32)`; `mount(root: i32, props: [*]u8, len: usize)`. Three questions to answer and record in the report: does `import islands.counter.view` compile without loading `client.aye`; does a `pub extern fn` of the build root export; does the build root need a `main`. If the second or third fails, stop and write the O-card, do not work around it. `render` returns `AllocError!void`.

#### F4.2: the browser glue
- lane: framework
- goal: `framework/web/islands.js` (embedded in `web` with `@embedFile`, no build step) finds every `[data-island]` element, loads `/_web/islands/<name>.wasm`, supplies the three `dom_*` imports over a handle table, writes the props into wasm memory through the exported `malloc`, calls `mount(root, ptr, len)`, and forwards listener events to `event(id)`.
- serves: FD6
- files: `framework/web/islands.js` (new), `framework/web/islands.aye` (new: `pub fn islandsScript() []u8`), `tests/framework.rs` (`islands_glue`), `tests/framework/island_dom.mjs` (new: a fake `document` with `querySelector`, `textContent`, `addEventListener`, `click()`, and a `fetch` that returns a given `.wasm` file)
- verify: `cargo test --release --test framework islands_glue` → exit 0; the test builds F4.1's wasm into a temp directory, runs the script under `node` (skipped with a message when Node is older than 22): after the glue runs, the span reads `Count: 3`, two clicks give `Count: 5`, and an element whose `.wasm` fetch fails logs one `console.error` and leaves the others working; ``islandsScript()` in `framework/web/islands.aye` (inline, like `dev.js`)` → exit 0
- depends on: F4.1
- parallel with: F4.3 (different files except `tests/framework.rs`)
- where: local
- model: sonnet
- time box: 120 min
- notes: ES module with no imports, `export` nothing; it runs on load, so a page needs only `<script defer src="/_web/islands.js">`. `dom_set_text` is `textContent`; `dom_on` is `addEventListener(name, () => exports.event(id))`. `window` and `document` are used only inside functions, so the file parses and loads under Node with the fake.

#### F4.3: `web.island` writes the markup and the script tag
- lane: framework
- goal: `web.island(h, name, props, render)` writes `<div data-island="NAME" data-props="PROPS">` (both escaped), the island's server-rendered HTML and `</div>`, and marks the response so the server adds `<script defer src="/_web/islands.js"></script>` once before `</body>` (at the end when there is none).
- serves: FD6, FD2
- files: `framework/web/islands.aye`, `framework/web/response.aye` (the flag), `framework/web/serve.aye` (the injection and `GET /_web/islands.js`, `Content-Type: text/javascript; charset=utf-8`), `framework/web/dev.aye` (share `devInject`'s search for `</body>`), `framework/examples/pages/app/counter/page.aye` (new), `tests/framework.rs`, `tests/framework/pages/counter*.http` (new)
- verify: `cargo test --release --test framework pages` → exit 0 and `cargo test --release --test framework web_units` → exit 0; new cases: `GET /counter` has the island `div` with `data-props="{&quot;start&quot;:3}"`, `<button` and `Count: 3` inside it, the script tag once before `</body>`; `GET /about` has no script tag; `GET /_web/islands.js` is `200 text/javascript; charset=utf-8` and equals `islandsScript()` (the glue is an inline string in `framework/web/islands.aye`, no `.js` file); two islands on a page give one script tag (`test` block)
- depends on: F4.1
- parallel with: F4.2
- where: local
- model: sonnet
- time box: 90 min
- notes: `render` is `fn(*mut Builder, []u8) AllocError!void`, passed as a function value (no closures); `web.island` passes `h.buf`. The page imports `islands.counter.view` and `web`; the island never imports `web`. With `--dev` both tags are injected (dev first). The route `/_web/islands.js` is always on, not only with `--dev`.

#### F4.4: `web gen` builds the islands and the server serves them
- lane: framework
- goal: `web gen` finds every `islands/<name>/client.aye` of an app, runs `aye build --target wasm32-freestanding islands/<name> -o .web/client/<name>.wasm`, passes the absolute path of `.web/client/` to the server, and the server answers `GET /_web/islands/<name>.wasm` from it with `Content-Type: application/wasm`; `web dev` needs no change because it runs gen on each restart.
- serves: FD6
- files: `framework/gen/main.aye`, `framework/gen/scan.aye`, `framework/web/serve.aye` (`runApp` takes the client directory), `framework/web/files.aye` (the `wasm` MIME type), `framework/examples/pages/` (the island is already there), `tests/framework.rs`, `tests/framework/pages/counter_wasm.http` (new)
- verify: `cargo test --release --test framework pages` → exit 0; new cases: `GET /_web/islands/counter.wasm` is `200 application/wasm` with an `ETag` and a body that starts with `\0asm`, `GET /_web/islands/nope.wasm` and `GET /_web/islands/../x` are 404 and 400 (`serveFile` rules), `HEAD` has no body; `cargo test --release --test framework dev_start` still → exit 0
- depends on: F4.2, F4.3
- parallel with: –
- where: local
- model: sonnet
- time box: 120 min
- notes: Keep `run` and `runApp` callable as now (default client dir empty = nothing served); use `serveFile` with a prefix strip, not a second file reader. A failing island build is a gen error with the compiler's text, which the dev CLI already shows as the error page. `.web/client/` is also where F5.1 reads the files to embed.

#### F4.5: an island in a Node 22 browser, and a dev edit
- lane: framework
- goal: a Node 22 script plays the browser against the running pages example: it fetches `/counter`, reads the island markup, loads `/_web/islands.js` and the `.wasm` from the server, clicks twice and sees the count; under `web dev`, editing `islands/counter/view/view.aye` and fetching again gives the new label.
- serves: FD6
- files: `tests/framework/island_e2e.mjs` (new: takes the base URL, builds a fake DOM from the page's `data-island` elements, imports the served glue), `tests/framework.rs` (`island_e2e`; `dev_island` for the edit)
- verify: `cargo test --release --test framework island_e2e` → exit 0 (prints `Count: 3`, `Count: 5`), then `cargo test --release --test framework dev_island` → exit 0 (the label text `Count: ` becomes `Total: ` in a temp copy; the script, rerun after the restart, prints `Total: 3`; at most 90 s)
- depends on: F4.4
- parallel with: –
- where: local
- model: sonnet
- time box: 120 min
- notes: The glue is imported from the served text (`data:` URL import or a temp file), not from the repo, so the test covers the route. The fake DOM is F4.2's file. FD6 also says "in Chrome": the main context runs the page by hand (claude-in-chrome) after this card and puts the result in the F4 report; it is not a test (`docs/framework.md` §7).

## Cards: F5

F5 is the production build, the numbers and the example app (FD7 to FD9,
FD4's hashed names). Choices where `docs/framework.md` is silent, each in
the card that makes it:

- **Production = no `--dev`.** The dev server is `.web/server --dev`
  (section 5); production is the same program built `--mode fast` from a
  `main.aye` that embeds `public/` and ignores `--dev`. No `--prod` flag
  in the server; the flag of the generator and the build command is
  `--prod`/`build`.
- **The command** is `aye run framework/build -- <app> -o <file>` (the
  same shape as `framework/dev`; the name `web build` is a placeholder
  like `web dev`).
- **Hashed names** are `name.<first 8 hex of the SHA-256>.ext`; a page
  asks for one with `web.assetUrl("/style.css")`, which is the path itself
  in dev. Only `public/` files are hashed; the island glue and `.wasm`
  files under `/_web/` keep their names.
- **Embedded files** answer with `ETag` (the full hash, quoted) and no
  `Last-Modified` (a build has no mtime to give); the hashed URL adds
  `Cache-Control: public, max-age=31536000, immutable`, the plain URL
  stays `max-age=0`.
- **No start-up hook exists** (the doc has none and adding one is a
  feature it does not describe), so the posts app reads its file on the
  first request that needs it, behind the `Mutex`, from the path after
  `--posts` (default `posts.json` in the working directory).

#### F5.1: `web gen --prod` embeds `public/`
- lane: framework
- goal: `aye run framework/gen -- <app> --prod` also writes `.web/assets.aye`: one `@embedFile` per regular file of `public/` and of `.web/client/`, a table with the URL path, the hashed path, the MIME type, the bytes and the ETag of each, and a `main.aye` that serves the table.
- serves: FD7, FD4
- files: `framework/gen/main.aye`, `framework/gen/assets.aye` (new: walk, hash with `std.crypto.sha256` and `std.encoding`, write the source), `tests/framework.rs` (`prod_gen`)
- verify: `cargo test --release --test framework prod_gen` → exit 0; on a temp copy of the pages example plus a generated 2 MiB `public/big.bin`: `.web/assets.aye` has one `@embedFile` per file (style.css, img/logo.svg, big.bin, the counter wasm), the hashed names equal `sha256sum | cut -c1-8` of the files, `aye check .web` → exit 0 and `aye build .web` finishes in under 60 s; a dotfile and a symlink in `public/` are left out
- depends on: F4.4
- parallel with: F5.2 (not on `tests/framework.rs`)
- where: local
- model: sonnet
- time box: 120 min
- notes: `.web/public` is a link to the app's `public/` so `@embedFile("public/style.css")` is relative to `.web/assets.aye`; a path with a quote or backslash is a gen error (as `publicDir`). If a module-level `pub let table = [..]` of structs with `@embedFile` constants is refused, generate `pub fn table() []Asset` that builds it; report which. Without `--prod` gen writes what it writes today.

#### F5.2: serve the embedded files, hashed names, no dev
- lane: framework
- goal: `web.runProd(alloc, port, routes, assets, notFound)` serves the asset table from memory (200, `ETag`, 304 on `If-None-Match`, `HEAD`, the immutable header on hashed URLs), `web.assetUrl(path)` returns the hashed path in production and the path itself elsewhere, and `--dev` is ignored.
- serves: FD7, FD4
- files: `framework/web/assets.aye` (new: `Asset`, `serveAsset`, `assetUrl`), `framework/web/serve.aye` (`runProd`), `framework/examples/pages/app/layout.aye` (`href` from `web.assetUrl("/style.css")`), `tests/framework.rs`, `tests/framework/pages_prod/*.http` (new)
- verify: `cargo test --release --test framework prod_serve` → exit 0 and `cargo test --release --test framework pages` → exit 0 (unchanged output); the test generates with `--prod`, builds, starts and checks: `GET /` links `/style.<8hex>.css`, that URL is `200` with `Cache-Control: ... immutable` and the ETag `"<64 hex>"`, `/style.css` is `200` with `max-age=0`, `If-None-Match` gives `304` with no body, `GET /_web/events` and `GET /_web/dev.js` are 404 even with `--dev` on the command line, a traversal path is 400 or 404 as in dev
- depends on: F5.1
- parallel with: –
- where: local
- model: sonnet
- time box: 120 min
- notes: The table is a module-level `var` set once by `runProd` before the first thread starts and read-only after (no lock needed; say so in the comment). `assetUrl` allocates from `req.arena` or returns a slice of the table, whichever the agent finds simpler. Reuse `mimeOf` and `etagListed` from `files.aye`.

#### F5.3: `framework/build`: one command, one executable
- lane: framework
- goal: `aye run framework/build -- <app> -o <file>` runs gen with `--prod`, `aye check` and `aye build --mode fast --no-link libcurl .web -o <file>`, and prints the size; the result runs with the app tree gone and no Aye toolchain on `PATH`.
- serves: FD7
- files: `framework/build/main.aye` (new), `tests/framework.rs` (`build_prod`)
- verify: `cargo test --release --test framework build_prod` → exit 0; the test builds the pages example, copies the executable alone into an empty temp directory, deletes the app copy, starts it from that directory with `PATH` set to `/usr/bin:/bin` and `--port 0`, and gets the F5.2 transcripts; the dynamic library list (`otool -L` on macOS) holds only `/usr/lib/libSystem.B.dylib`; a broken page makes the command exit 1 with the compiler's text and no file written
- depends on: F5.2, O1.23
- parallel with: –
- where: local (macOS; the Linux `ldd` arm is F5.8)
- model: sonnet
- time box: 120 min
- notes: Copy the spawn helpers of `framework/dev/main.aye` (`capture`, `die`) instead of importing them; a shared module needs path dependencies and is not worth it for two files. `$AYE` as in dev. The build output of `.web/` stays for debugging. The server not making HTTPS client calls is why libcurl is dropped; an app that calls `std.http.get` over https gets the client's own error in production (record this in the card's report).

#### F5.4: the posts app: data and pages
- lane: framework
- goal: `framework/examples/posts/` is an app with `app/layout.aye`, `app/not-found.aye`, `/`, `/`, `/posts/[slug]`, `/about` and `public/style.css`, its posts read from the JSON file after `--posts` on first use, and golden transcripts for each page.
- serves: FD9
- files: `framework/examples/posts/` (new: `app/`, `lib/posts/posts.aye` with a `Mutex`-guarded cache and `test` blocks, `data/posts.json` with 3 posts, `public/style.css`), `tests/framework.rs` (`posts_pages`), `tests/framework/posts/*.http` (new)
- verify: `aye test framework/examples/posts/lib/posts` → exit 0 and `cargo test --release --test framework posts_pages` → exit 0; cases: `/` lists the 3 titles as links, `/blog/<slug>` shows the title and the body (`post.body_html` is raw, the title escaped; one post has `<` in its title), `/blog/missing` renders `not-found.aye` with 404, `/about`, `/nope` 404; a missing or malformed `--posts` file gives a 500 page and one log line, then the next request retries
- depends on: F5.2
- parallel with: F5.3
- where: local
- model: sonnet
- time box: 120 min
- notes: The JSON has `slug`, `title`, `body_html` per post (parsed with `std.json`); slugs are compared as bytes. The cache is a module-level `var` behind `std.sync.Mutex` with the lazy gate the guestbook uses (O1.21 is open). Mirror `framework/examples/pages/lib/guestbook` for style. Pages use `web.assetUrl` for the style sheet, so the same transcripts hold in dev and production builds up to that one `href` (the test scrubs it).

#### F5.5: comments: form POST and API route
- lane: framework
- goal: `POST /blog/<slug>` (a `route.aye` `post` next to the page) adds a comment held in memory and answers 303 to `/blog/<slug>?added=1`; the detail page lists the comments and the notice; `GET /api/posts` and `GET /api/posts/[slug]` return JSON and `POST /api/posts/[slug]` takes a JSON comment.
- serves: FD9, FD3
- files: `framework/examples/posts/app/posts/[slug]/{page,route}.aye`, `framework/examples/posts/app/api/posts/route.aye`, `framework/examples/posts/app/api/posts/[slug]/route.aye` (new), `framework/examples/posts/lib/posts/posts.aye`, `tests/framework.rs`, `tests/framework/posts/*.http`
- verify: `cargo test --release --test framework posts_pages` → exit 0; new cases: the form POST (`name=Ann+B&text=Hi%21`) gives `303`, `Location: /blog/hello?added=1`; `GET` of that location lists `Ann B` and `Hi!` escaped and the notice; an empty field is `400`, a field over the limit `413`; `GET /api/posts` is `[{"slug":...,"title":...}]`; `POST /api/posts/hello` with `{"name":"Bo","text":"yo"}` gives `201` and the stored comment as JSON, and the detail page shows it; a bad JSON body is `{"error":"BadRequest"}`; a `PUT` is `405` with `Allow`
- depends on: F5.4
- parallel with: –
- where: local
- model: sonnet
- time box: 120 min
- notes: Same limits and error names as `app/guestbook/route.aye`. `201` is the card's choice (the doc says only that a handler sets `res.status`). Comments are per slug in one list behind the same `Mutex`; no limit on the number of comments beyond the guestbook's pattern of `Error.Conflict` at 1000. The transcripts run in order against one server (the test says so, as `PAGES_ETAG_CASES` do).

#### F5.6: the island on the post page
- lane: framework
- goal: the post page has a `likes` island (a button and a count that starts at the post's `likes` from the JSON and goes up on click, in the browser only) built by gen, and the same transcripts hold with it.
- serves: FD9, FD6
- files: `framework/examples/posts/islands/likes/{view/view.aye,client.aye}` (new, a copy of the counter's shape), `framework/examples/posts/app/posts/[slug]/page.aye`, `framework/examples/posts/data/posts.json` (`likes`), `tests/framework.rs`, `tests/framework/posts/*.http`, `tests/framework/island_e2e.mjs` (a `--text` option for the expected label)
- verify: `cargo test --release --test framework posts_pages` → exit 0 (the detail transcript holds the island `div`) and `cargo test --release --test framework posts_island` → exit 0 (F4.5's script against the posts server: `Likes: 4` after the click from 3)
- depends on: F5.5, F4.5
- parallel with: –
- where: local
- model: sonnet
- time box: 90 min
- notes: Nothing is stored: a reload shows the JSON's count again (the doc's data scope is a file and memory; a persisted like would be a feature it does not describe). Copy the counter, do not share it: an island is built from its own directory.

#### F5.7: the posts app in the anyknown browser, dev and production
- lane: framework
- goal: `scripts/framework-posts.sh` builds the posts app for production, starts it, and checks the anyknown browser's `dump --md` of `/` and of a post, and a form POST through `anyknown agent` JSON lines; `posts_prod` checks the production binary against the F5.4 to F5.6 transcripts.
- serves: FD9, FD7
- files: `scripts/framework-posts.sh` (new), `tests/framework.rs` (`posts_prod`), `tests/framework/posts/anyknown_*.md` (new: the expected Markdown dumps)
- verify: `cargo test --release --test framework posts_prod` → exit 0 (the production binary from F5.3 gives the same transcripts as the dev build, apart from the scrubbed `href` and ETag lines) and `scripts/framework-posts.sh` → exit 0, printing `dump / ok`, `dump post ok`, `form ok` (the dump contains every post title; after `navigate`, `type name`, `type text`, `click`, the post page dump holds the new comment)
- depends on: F5.6, F5.3
- parallel with: –
- where: local (the browser build is `scripts/downstream.sh`, about 30 s in debug mode; it stays out of `tests/framework.rs` so a browser regression does not read as a framework failure, `docs/framework.md` §7)
- model: sonnet
- time box: 120 min
- notes: The `agent` JSON line shapes are in `docs/agent-protocol.md` of aye-lang/anyknown. If the browser cannot do a step, the script prints which step and the card ends with that listed as a browser-lane gap (P-card), not worked around. The island is not run in the anyknown browser (no `WebAssembly` there, `docs/framework.md` §7); its text is the server-rendered `Likes: 3`.

#### F5.8: the framework on Ubuntu 24.04
- lane: framework
- goal: `cargo test --release --test framework` passes on Ubuntu 24.04 x86_64 and the production posts binary's `ldd` shows only `libc`, `libm` and the loader.
- serves: FD7
- files: `tests/framework.rs` (an `ldd` arm next to `otool -L` in `build_prod`, chosen by `cfg!(target_os)`), whatever small fix the failures need in `framework/`
- verify: `cargo test --release --test framework` → exit 0 in the cloud session; the report lists the test names and the `ldd` output
- depends on: F5.7
- parallel with: F5.9 (not at the same time: both want the VM quiet)
- where: cloud (Linux; `scripts/setup-ubuntu.sh` installs the toolchain)
- model: sonnet
- time box: 120 min
- notes: Expect Linux differences in `std.fs` link handling and `stat` (the O2.5 notes). Fix only what is small and inside `framework/` or the test; anything in `std/` or `src/` is an O-card in the report. Stop rule: two failing runs of the same test → report.

#### F5.9: the hello page's requests per second
- lane: framework
- goal: `docs/aye-bench.md` has rows for the framework's hello page (`framework/examples/hello`, production build, `--mode fast`) and for `/about` of the pages example, next to the `std.http` hello row, with the machine and its load.
- serves: FD8
- files: `docs/aye-bench.md`, `scripts/bench/run.sh` (a `framework` entry on port 8084)
- verify: `scripts/bench/run.sh` → prints req/s lines for `aye`, `node`, `go` and `framework`, and `grep -c framework docs/aye-bench.md` → at least 3; the table row has three runs and the median at `-c 64` and `-c 1`
- depends on: F5.3, F5.8
- parallel with: –
- where: cloud (a quiet VM; O7.1's setup)
- model: sonnet
- time box: 90 min
- notes: Same flags as O7.1 (`ab -k -c 64 -n 200000`, three runs, median). The hello example needs the production build path of F5.3 (`web gen --prod` on an app with no `public/` must work; F5.1 covers an empty table). FD8 asks only for the number, not that it beats Node; if the framework is below Node's 10.7k, the report says so and proposes a diagnose step.

## F4/F5 Aye-lane needs

Checked with grep of `src/sema.rs`, `src/driver.rs`, `src/codes.rs`,
`std/http`, `std/crypto`, `std/encoding`, `docs/decisions.md` (Targets
O2) and the tests under `tests/modules/embed*`:

- **Needed, missing:** a way to build without an optional library that is
  installed. `std.http` declares `@linkOptional("libcurl")` for its
  client, and `aye build` links every optional library that is installed
  (`linked` in `src/driver.rs`), so a production server built on a
  machine with libcurl needs libcurl, and its dependencies, where it
  runs. Written as O1.23 in `plans/aye.md`; F5.3 waits for it.
- **Answered by F4.1, no card unless it fails:** whether a submodule
  imports without its parent directory, whether a `pub extern fn` of the
  build root of a wasm32-freestanding build is exported, and whether
  that build needs a `main` (`docs/decisions.md` says a program with no
  `main` is accepted). If one fails, F4.1 stops and an O-card is written
  then.
- **Exists, no card needed:** `@embedFile("path")` (a string literal
  relative to the source file, a `[]u8` constant, a regular file; E0111
  otherwise, spec §15 and `tests/modules/embed`); wasm32-freestanding
  with body-less `extern fn` as `env` imports, `pub extern fn` as
  exports, an exported `malloc` and `memory` (O2.4, O2.7, O2.7b);
  `std.json` on freestanding (O2.7 `json_roundtrip`); `std.crypto.sha256`
  and `std.encoding` hex for the content hashes (O3.3, O3.3a);
  `std.fs.readFile` and `std.json` for the posts file; `Arena.reset` and
  one arena per connection (O7.4, O7.4a); the Linux triples compile and
  the cloud runs the suite (O2.2, O2.5b, O2.5c, O2.8); the benchmark
  driver (O7.1); path dependencies (O4.2; the cards do not use them).
- **Not needed in F4/F5:** a target-OS constant (O1.14; the two-module
  island layout avoids it); `@embedFile` of a directory (need 9 of
  `docs/framework.md`; the generator writes one per file); a start-up
  hook (not in the doc; the posts app reads its file lazily); signal
  handling and graceful stop (O3.6c; a production server is stopped with
  SIGTERM, which ends the process); a global `Mutex` initializer (O1.21;
  the posts cache uses the guestbook's lazy gate, as F2.3 did); a fully
  static executable (the check is "only libc and libm", which `ldd` and
  `otool -L` show).

## Backlog

Paused; cards get written if the framework lane is resumed. Source in
brackets.

- F1.2a `web gen` leftovers: `[...rest]` and `(group)` directories, `layout.aye`, `not-found.aye`, percent-decoding of path segments; replace the `--web <dir>` flag and the `.web/web` symlink with an `aye.toml` path dependency on `framework/web` (O4.2 exists). Verify: `pages` test cases.
- F1.3a API-route error bodies: `web.run`'s `fail()` writes plain text for every error; an error returned from a `route.aye` handler should be `{"error":"..."}` (docs/framework.md), which needs the route kind in the table; also scan `route.aye` exports with the parser (or `aye doc --signatures`) instead of `pub fn get(` line matching; update docs/framework.md F1.3 names (`items/[id]`). Verify: a route that returns an error gives a JSON body in the `pages` test.
- F2 cards (write next): `[...rest]` and `(group)` (F1.2a); a page that returns `Error.NotFound` should render `not-found.aye` (catch it in the generated wrappers) and nested `not-found.aye` per directory (F2.2a); query/form parsing is O3.6a; static-file `ETag`/304 once O3.4b lands; production `@embedFile` for `public/` (FD7; now F5.1). F2.1 static files done.
- F3 sessions: `docs/framework.md` has no session design (§2 gives `cookie` and `setCookie` only). Write the section first (a signed cookie with `std.crypto.hmacSha256` and `constantTimeEqual`, `web.Session` get/set, the key from the environment or `randomBytes` at start, the `Set-Cookie` attributes), then the card. Verify: a transcript where a tampered cookie reads as no session.
- F4 bundler for JS islands (FD6's "or in JS and bundled"): reuses the browser's JS parser, which needs the browser-lane step that moves `browser/js/lexer.aye`, `parser.aye` and `ast.aye` out of the `js` module (not yet a card, `docs/framework.md` "Aye-lane steps by phase"). Not in F4: the Aye island already meets FD6, and plain JS modules in `public/` work from F2.1. Verify when written: a JS island imported by a page runs under Node 22.
- F4 TypeScript (type-stripping in the shared parser): waits for the bundler and for the user's answer to question 3 of `docs/framework.md`.
- F5 later: TLS in the server (the documented deploy is behind Caddy or nginx), a database layer (§6: out of scope), sessions (above).
- Islands and dev reload in the anyknown browser need `WebAssembly` and `EventSource` there (neither exists); until then the framework tests with Node 22 and Chrome (`docs/framework.md` §5, §7).
