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
| F4 | Client code (FD6) | an island under Node 22 and in Chrome | paused |
| F5 | Production build, numbers, the example app (FD7–FD9) | FD7–FD9 checks | paused |

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
- verify: `cargo test --release --test framework dev_units` → exit 0; the saved output with two errors gives a page that has both messages with `app/blog/[slug]/page.aye:LINE:COL` and escaped text
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

## Backlog

Paused; cards get written if the framework lane is resumed. Source in
brackets.

- F1.2a `web gen` leftovers: `[...rest]` and `(group)` directories, `layout.aye`, `not-found.aye`, percent-decoding of path segments; replace the `--web <dir>` flag and the `.web/web` symlink with an `aye.toml` path dependency on `framework/web` (O4.2 exists). Verify: `pages` test cases.
- F1.3a API-route error bodies: `web.run`'s `fail()` writes plain text for every error; an error returned from a `route.aye` handler should be `{"error":"..."}` (docs/framework.md), which needs the route kind in the table; also scan `route.aye` exports with the parser (or `aye doc --signatures`) instead of `pub fn get(` line matching; update docs/framework.md F1.3 names (`items/[id]`). Verify: a route that returns an error gives a JSON body in the `pages` test.
- F2 cards (write next): `[...rest]` and `(group)` (F1.2a); a page that returns `Error.NotFound` should render `not-found.aye` (catch it in the generated wrappers) and nested `not-found.aye` per directory (F2.2a); query/form parsing is O3.6a; static-file `ETag`/304 once O3.4b lands; production `@embedFile` for `public/` (FD7). F2.1 static files done.
- F3 sessions: `docs/framework.md` has no session design (§2 gives `cookie` and `setCookie` only). Write the section first (a signed cookie with `std.crypto.hmacSha256` and `constantTimeEqual`, `web.Session` get/set, the key from the environment or `randomBytes` at start, the `Set-Cookie` attributes), then the card. Verify: a transcript where a tampered cookie reads as no session.
- F5 posts example (FD9): the blog with its posts read from a file at start-up and form submissions kept in memory behind a `Mutex` (`docs/framework.md` §6), after F2.3; checked by transcripts plus the `anyknown dump --md` script.
- F4–F5 rows: written when the lane resumes; the Aye steps each phase needs are in `docs/framework.md`, "Aye-lane steps by phase".
- Islands and dev reload in the anyknown browser need `WebAssembly` and `EventSource` there (neither exists); until then the framework tests with Node 22 and Chrome (`docs/framework.md` §5, §7).
