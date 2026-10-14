# The framework: design (F0)

A web server framework written in Aye, in the spirit of Next.js, under
`framework/` (new). This document answers the open choices of
[`plans/framework.md`](plans/framework.md) and fixes the first vertical
slice (F1). It is a proposal until the user approves it (F0's exit
check). Names in it (`web` for the library module and the command,
`.web/` for the generated build root) are placeholders: see "Questions
for the user".

The dweller browser, which this document cites for code and protocol
(`browser/...`, `docs/agent-protocol.md`, `browser/COMPILER_GAPS.md`), has lived in
[aye-lang/dweller](https://github.com/aye-lang/dweller) since P8.3; those paths are
relative to that repo.

Inputs: [`plan.md`](plan.md) (lanes), [`plans/aye.md`](plans/aye.md)
(the O steps cited below), [`design.md`](design.md) (principles,
non-goals), [`spec.md`](spec.md) §8 (errors), §12 (modules), §15
(`std.thread`, `std.sync`, `std.coro`), [`decisions.md`](decisions.md)
("Modules (M5)", "Threads (M10)", "Coroutines (M12)").

## Decisions taken before F0

Recorded here, not reopened:

- **Manifest:** a declarative file, read by the compiler, not a
  `build.aye` run at compile time. Its format is O4.1's to decide. The
  framework adds no config file of its own: app settings (port, limits)
  are Aye code in the app, so they are type-checked.
- **HTTP:** `std.http` serves plain HTTP first (O1.4); TLS comes later.
  The documented deploy is behind a reverse proxy (Caddy or nginx) that
  ends TLS. The HTTP client keeps using libcurl, as the browser does
  (`browser/net/http.aye`), until O3.4/O3.5 decide otherwise.
- **Client code:** both (a) Aye compiled to wasm32 for client islands
  and (b) plain JS/TS assets, bundled later. New wording for design.md's
  non-goal is proposed below.
- **Platforms:** macOS is supported; Linux must compile; Windows is out
  of scope.
- **Bundler:** when built, it reuses the browser's JS parser
  (`browser/js/lexer.aye`, `parser.aye`, `ast.aye`), after a
  browser-lane step moves them out of the `js` module and O4.2 adds path
  dependencies. No copy.

## What Aye gives and lacks today

Facts the choices below rest on (checked against the stage0 compiler on
2026-10-08 where marked):

- A directory is one module; its files share one namespace (spec §12),
  and a top-level name of one file blocks locals in the others
  (`browser/COMPILER_GAPS.md` #26). `import a.b` resolves under the root
  module's directory only; there are no path dependencies until O4.2.
- Directory names that are imported must be identifiers: `[slug]` and
  `about-us` cannot be imported. `_slug` can (checked).
- No reflection, no loading of code at run time, no closures, no macros,
  no variadic functions (spec §4, §18). Function values are C function
  pointers (`fn(A, B) R`, spec §5); a struct field of type
  `fn(*Req, *mut Res) E!void` holding a route module's `pub fn` works
  (checked).
- Errors carry no payload and are identified by name across sets
  (spec §8); a `pub fn` writes its error set.
- The module loader follows directory symlinks, and `aye check --json`
  then names the symlink's path, not the target's (checked).
- Concurrency is library only (design.md, 並行模型): `std.thread`
  (`spawn`, 8 MiB reserved stacks), `std.sync` (`Mutex`, `Channel`,
  `Atomic`), `std.coro` (stackful, runs only on the thread that made it,
  aarch64/x86_64 on macOS and Linux). design.md lists a `std.event`
  loop; it does not exist in `std/`. Allocators are not thread-safe
  (decisions.md, "Threads (M10)").
- `std.heap.Arena` has `init`, `allocator`, `deinit`; no reset.
- `@embedFile` takes one string literal, relative to the source file.

## 1. Route layout

The tree the user writes follows Next.js's App Router:

```
blog/                       an app
  app/
    layout.aye              pub fn layout: wraps every page
    page.aye                /
    not-found.aye           the 404 page
    about/page.aye          /about
    blog/page.aye           /blog
    blog/[slug]/page.aye    /blog/<slug>
    docs/[...path]/page.aye /docs/<a>/<b>/...
    api/posts/route.aye     GET and POST /api/posts
  lib/posts/posts.aye       shared code: `import lib.posts`
  public/logo.png           /logo.png
  .web/                     generated build root, not checked in
```

File names inside a directory are free (only directory names are
imported), so `not-found.aye` is fine. Each role has its own function
name (`page`, `layout`, `notFound`, `get`, `post`, ...), so the files of
one directory never clash (#26). A directory with both `page.aye` and a
`get` in `route.aye` is an error from the generator, as in Next.js.
Any other `.aye` file in a route directory (a `helper.aye` beside
`page.aye`) belongs to no module and is an error from the generator; shared
code goes in a module outside `app/` (`lib/`), imported by the route files.

Options:

- **A. Identifier directories in the user's tree.** `blog/_slug/`,
  `docs/__path/`; the app directory is the Aye root; the generator writes
  one router module into it. Error paths are the real files. But
  `/about-us`, `/for` or `/2026` cannot be routes without an escape rule
  (Aye identifiers have no `-`; keywords and leading digits are out).
- **B. Next.js names in the user's tree, a generated build root.** The
  generator writes `.web/`: a `main.aye`, a `routes/` module with the
  table, and one module directory per app file with identifier names
  (`.web/r/r3_blog_slug_page/page.aye -> ../../../app/blog/[slug]/page.aye`;
  roles `page`, `route`, `layout`, `notfound`, so a page local may share a
  name with a route's `pub fn`), plus one symlink per
  other top-level directory of the app (`.web/lib -> ../lib`) and one to
  the framework (`.web/web`). `aye check` and `aye build` run on `.web/`.
  Any URL segment works; the CLI maps `.web/r/...` paths in diagnostics
  back to `app/...`.
- **C. A hand-written route table.** No generator; the user lists routes
  in `main.aye`. Simple and explicit, but not the Next.js experience,
  and the table drifts from the tree.

**Recommendation: B.** Any URL segment works with no escape rule to
learn, and an AI that knows Next.js writes `app/blog/[slug]/page.aye`
without being told (design principle 6). The cost is all in the
generator (a symlink farm, a path map for diagnostics), which is the
framework's code, not the user's. `.web/` plays the part of `.next/`.
C stays as what F1.1 hand-writes before the generator exists.

Mapping of segments: a literal directory is a literal segment; `[name]`
is one segment bound to `name`; `[...name]` is the rest (one or more
segments); `(group)` directories are left out of the URL, as in Next.js.
A `(group)` directory's `layout.aye` still wraps the pages below it, and
`[...name]` must be the last segment. Segments are percent-decoded after
the path is split at `/` (so `%2F` is not a separator), for matching and
in `param`; `param("rest")` joins a catch-all's segments with `/`.
Matching order: literal before `[name]` before `[...name]`. A `name`
that is not an Aye identifier is a generator error. The symlink names
are `r<index>_<sanitized path>`, so they are identifiers and never
keywords.

## 2. Request and response API

Every handler has one signature, which is also the route table's
function type:

```aye
pub fn get(req: *web.Request, res: *mut web.Response) web.Error!void
```

`page` has the same signature (it is the route's GET); `post`, `put`,
`patch`, `delete` likewise. `HEAD` is answered from `get`; another
method gets 405 with `Allow`.

- **`web.Request`:** `method` (an enum), `path`, `body`, `arena` (an
  `Allocator`), and methods `param(name) []u8` (panics in debug and safe
  mode on a name that is not a segment of the matched route: a bug, not
  a request error), `header(name) ?[]u8`, `query(name)` and
  `form(name)` (`Error!?[]u8`, percent-decoded into `arena`),
  `cookie(name) ?[]u8` (as sent); with O3.1 `json()`.
- **`web.Response`:** `status: u16` (200 by default),
  `header(name, value)`, `text(s)`, `html() *mut web.Html`, `redirect(url,
  status)`; with O3.1 `json()` (a writer into the body); with O3.6
  `setCookie`, `stream()` (chunked) and `events()` (server-sent events).
- **Memory:** `req.arena` is an `Arena` made for the request and freed
  after the response is written (design.md's per-frame arena, per
  request). Handlers allocate from it and never free.
- **Errors:** `pub error Error { BadRequest, Unauthorized, Forbidden,
  NotFound, MethodNotAllowed, Conflict, PayloadTooLarge, OutOfMemory,
  Internal }`, mapped to 400, 401, 403, 404, 405, 409, 413, 500, 500. A
  request that matches no route and no `public/` file gets the app's
  `not-found.aye` (404, inside the root layout; without the file, a plain
  404). A `page.aye` handler that returns `NotFound` gets the same page
  (the generated wrapper of the page catches it and discards what the
  page had written; without a `not-found.aye`, the plain 404). Any other
  error of a `page.aye` is a plain-text page of its status. An error of a
  `route.aye` handler is answered as JSON, `{"error":"<name>"}` with the
  status of the error (`{"error":"NotFound"}` with 404); the route table
  tells them apart with `Route.kind` (`Kind.Page` or `Kind.Api`), set by
  `web gen`. A 405 for a method the file lacks stays the plain page. A
  handler that wants another status sets `res.status` and returns
  normally.
  Errors of other sets are handled where they happen:
  `std.fs.open(p) catch |e| { std.log.err("open {}: {}", p, e); return
  web.Error.Internal }`. Because errors are identified by name, a
  `try` of `std.fs.open` would pass `NotFound` on as a 404; the fixed
  set makes that a compile error instead of a surprise.
- **Streaming:** pages are buffered (the layout wraps the finished body,
  and status and headers can change until the handler returns). API
  routes stream with `res.stream()` or `res.events()` once O3.6 exists;
  after the headers are sent, an error closes the connection and is
  logged.

Options considered: a handler returning a `web.Response` value (pure,
but streaming needs a second form and the body is built before the
status is known anyway); a single `*mut web.Ctx` (one parameter, but it
hides which parts a handler reads); errors of any set per route (the
generated code could not tell which names it may compare). **The
recommendation is the signature above**: one function type for every
route, so the table is plain data, and a wrong error set is an
`aye check` error in the route's own file.

## 3. Rendering

SSR only until F4. Options:

- **A. A typed HTML writer in plain Aye.** `web.Html` writes into the
  response body: `open(tag)`, `attr(name, value)`, `text(s)`,
  `close(tag)`, `void(tag)` (`img`, `br`), `raw(s)`. `text` and `attr`
  escape; `raw` is the only way to write markup unescaped (FD2). In
  debug and safe mode it checks that tags close in order and are known
  HTML elements (a typo panics with the tag), so a mistake shows in the
  first test.
- **B. Template files compiled to Aye** (Go's templ): HTML with holes,
  turned into Aye by the generator. Pleasant to write, still
  type-checked, but a second syntax to learn and to keep, and its errors
  need a position map back to the template.
- **C. JSX-like syntax in Aye.** A language change for one library;
  against design principles 3 and 5 (no macros, one way to write a
  thing). Not proposed.
- **D. Templates read at run time** (mustache). Not type-checked; errors
  at run time. Not proposed.

**Recommendation: A**, no new syntax and no tool. It is wordier than
JSX; if the example app (F5) shows the cost is real, B becomes a step
then, generating code that calls the same `web.Html`.

A page and a layout:

```aye
// app/blog/[slug]/page.aye
import lib.posts
import web

pub fn page(req: *web.Request, res: *mut web.Response) web.Error!void {
    let post = posts.find(req.param("slug")) orelse return web.Error.NotFound
    let h = res.html()
    try h.open("article")
    try h.open("h1")
    try h.text(post.title)
    try h.close("h1")
    try h.raw(post.body_html) // trusted: written by the site's authors
    try h.close("article")
}

// app/layout.aye
import web

pub fn layout(req: *web.Request, h: *mut web.Html, body: []u8) web.Error!void {
    try h.raw("<!doctype html>")
    try h.open("title")
    try h.text("Blog")
    try h.close("title")
    try h.raw(body) // the page, already escaped by its own Html
}
```

Aye has no closures, so a layout gets the rendered page as bytes rather
than a callback. Nested layouts are applied inside out: `web gen` writes,
for each page under a layout, a function that calls the page, then
`web.wrap(req, res, <layout>)` for each layout from the innermost to
`app/layout.aye`. `route.aye` functions are never wrapped.

Hydration and islands come in F4 (section "Client code").

## 4. Concurrency

What exists: threads, mutexes, channels, atomics and stackful
coroutines (spec §15); no event loop and no `poll` wrapper (O1.3 adds
read timeouts through `poll`). Options:

- **A. A thread per connection** (what O1.4 builds). Blocking reads and
  writes, keep-alive with an idle timeout, a cap on connections (503
  past it). Each connection thread has its own `General` (allocators are
  not thread-safe) and makes an `Arena` per request. Shared app state is
  a module-level `var` behind a `std.sync.Mutex`, itself a global
  (`var lock = Mutex.init()`, O1.21; globals exist, spec §4; there are
  no closures to carry it).
- **B. N threads, each with a kqueue/epoll loop and a `std.coro`
  coroutine per connection.** A read that would block yields to the
  loop. Handlers stay blocking-style code, because the coroutines are
  stackful (decisions.md, "Coroutines (M12)"); each connection costs a
  reserved stack (64 KiB minimum) instead of a thread.
- **C. One event loop with callbacks.** Handlers could not block, which
  the signature above assumes they can. Not proposed.

**Recommendation: A now, B only if numbers ask for it.** A is O1.4's
plan and enough for `ab -k -c 50` (O1.7, O12). B can replace it inside
`std.http` without changing a handler. A coroutine must stay on its
thread, so B is one loop per thread, never work stealing.

## 5. Dev server

`web dev <app> [--port N]` (F3):

1. Generate `.web/` (section 1).
2. `aye check --json .web`; on errors, serve an error page with the
   diagnostics, paths mapped back to `app/` (FD5).
3. `aye build .web -o .web/server` (debug mode).
4. Start `.web/server --port N --dev` (O1.6 `spawn`).
5. Poll the mtimes of `app/`, the other linked directories and
   `public/` every 250 ms (O1.5 `stat`, `readDir`). On a change of an
   `.aye` file: stop the server (`kill`, `wait`) and go to 1. On a change
   in `public/`: only reload, as dev mode serves `public/` from disk.

Reload: in dev mode every page gets `<script src="/_web/dev.js">`. It
opens an `EventSource` on `/_web/events`, which sends the build id when
it connects. When the server restarts, the browser reconnects on its
own, gets a new id and reloads. While the build is broken, the CLI
itself answers on the port (it has `std.http` too) with the error page
and the same two endpoints, so the page shows the error and reloads when
it is fixed. No proxy and no WebSocket server.

Options considered: building the app as a shared library and swapping
it in the running server (keeps connections; needs a `--shared` build
mode Aye lacks and state handover); watching with kqueue or FSEvents
(faster to notice, but per-platform; polling `stat` is the same code on
macOS and Linux). **Recommendation: restart and poll**, as above.

## 6. Data and databases

**Out of the framework's scope.** C libraries are first-class (design
principle 4): SQLite through `@cimport("sqlite3.h")`, as O3.7 shows in
`tests/ffi/`, with no framework API around it. The example app keeps its
posts in a file it reads on the first request that needs them and its
form submissions in memory behind a global `Mutex`. A database layer is a later decision, if ever.

## 7. Testing

- **Unit:** `test` blocks in `web` (route matching, `Html` escaping,
  header and cookie parsing), run as `browser/unit_tests.aye` runs the
  browser's until O5.1 runs a whole tree.
- **Golden HTTP transcripts (primary):** `tests/framework.rs` builds an
  example app with the `aye` that `cargo test` built, starts it on a
  free port (it prints the port it got), sends each request of
  `tests/framework/<app>/*.http` over a TCP socket and compares the
  response with the expected one (`Date` masked). It is part of
  `cargo test`, so rule 4 of `plan.md` covers it.
- **The dweller browser as client:** FD9 runs
  `dweller dump --md http://127.0.0.1:<port>/` on the example app, and
  the form POST through `dweller agent` JSON lines (`navigate`, `type`,
  `click`, `docs/agent-protocol.md`). This dogfoods the browser's
  networking, HTML parsing and forms on pages we control. It stays in
  the FD9 script, not in `tests/framework.rs`, so a browser regression
  does not read as a framework failure.
- **Not with the browser:** islands and dev reload need `WebAssembly`
  and `EventSource`, which the browser lacks. They are checked with a
  Node 22 script (SSE client, `.wasm` instantiation) and by hand in
  Chrome. The dev reload script is `tests/framework/dev_reload.mjs`
  (run by the `dev_reload_node` test). By hand, per release of `web dev`:
  1. `aye run framework/dev -- framework/examples/pages`, open the printed
     address in Chrome.
  2. Change the heading in `app/about/page.aye` and save: the open
     `/about` page reloads by itself and shows the new heading.
  3. Break the file (`raw(5)`): the page reloads to the error page; fix it:
     the page reloads to the working page.

**Recommendation:** transcripts first (F1.1 starts them), the browser
from F5.

## Client code (F4) and the design.md non-goal

Order: plain JS works from F2 at no cost (ES modules in `public/` are
static files; every browser loads them unbundled). Aye islands come
after O2.4. The bundler comes after O4.2 and the browser-lane parser
split; TypeScript is type-stripping added to that shared parser, not a
second parser.

An island is a module (`islands/counter/`) built twice: natively, where
its `render(h: *mut web.Html, props)` is called by the page for SSR, and
for wasm32-freestanding, where its exports handle events through
imported DOM functions. The page writes `try web.island(h, "counter",
props)`, which renders the HTML inside a `data-island` element with its
props; a generated `/_web/islands.js` loads `counter.wasm` and calls its
`mount`. The export and import syntax is O2.4's.

**Proposed wording** for design.md's non-goal (today: 不取代 JavaScript：
網頁內容的腳本語言仍是 JS，Aye 只寫引擎本身。), taken 2026-10-12 (see "Decided questions"):

> 不取代 JavaScript：網頁的腳本語言仍是 JS，dweller 引擎照規格實作它。Aye
> 寫引擎、伺服器與工具；Aye 也能編譯成 wasm32 在網頁裡執行（框架的
> island），那是 WebAssembly 的一般用法，網頁作者不必改寫 JS，Aye 也不提供
> JS 以外的網頁 API。

(English: Aye does not replace JavaScript. JS stays the scripting
language of web pages, and the dweller engine implements it to the
spec. Aye writes the engine, servers and tools; Aye can also be compiled
to wasm32 and run in pages (the framework's islands), which is ordinary
WebAssembly: page authors need not rewrite their JS, and Aye offers no
web API beside JS's.)

## Aye-lane steps by phase

| Phase | Needs (ids in `plans/aye.md`) |
|---|---|
| F1 | O1.2 and O1.3 (through O1.4), O1.4 (F1.1), O1.5 (F1.2), O3.1 (F1.3) |
| F2 | O1.5 (static files, a path stays under `public/`), O3.1, O3.6 (query, form, cookies), O3.2 (`Date`, `Last-Modified`), O3.3 (content hashes for `ETag` and hashed names) |
| F3 | O1.6 (spawn, kill, wait), O3.6 (server-sent events, graceful stop), O3.1 (reading `aye check --json`) |
| F4 | O2.1 to O2.4 (wasm islands); O4.1 and O4.2 plus a browser-lane step (bundler) |
| F5 | O2.2 (Linux triples compile), O1.7 and O7.1 (FD8 numbers), O4.1 and O4.2 (the app depends on `web` through the manifest, not a symlink) |

Not yet a row anywhere: the browser-lane step that moves
`browser/js/lexer.aye`, `parser.aye` and `ast.aye` into a module of
their own (proposed in the report of this step).

## F1: the first vertical slice

Goal: `curl` gets a page from a built app (F1's exit check).

**F1.1 (needs O1.4).** Files:

- `framework/web/` (module `web`): `request.aye`, `response.aye`,
  `html.aye`, `route.aye` (`struct Route { method, pattern, handler }`
  and the matcher, literal segments only in F1.1), `serve.aye`
  (`web.run(alloc, port, routes, public)` over `std.http`, an arena per request,
  `--port`, prints `listening on 127.0.0.1:<port>`) and `files.aye`
  (`serveFile`: `public/` files when no route matches, F2.1; `ETag`,
  `Last-Modified` and 304 for `If-None-Match` and `If-Modified-Since`, F2.1a).
- `framework/examples/hello/main.aye`: a hand-written table of two
  routes and `main`; `home.aye` with `pub fn page`.
- `framework/examples/hello/web`: a symlink to `../../web` (until O4.2).
- `tests/framework.rs`, `tests/framework/hello/root.http`,
  `tests/framework/hello/missing.http`.

Commands and expected output:

```
cargo build --release
target/release/aye build framework/examples/hello -o target/hello
target/hello --port 0            # prints: listening on 127.0.0.1:<port>
curl -i http://127.0.0.1:<port>/
HTTP/1.1 200 OK
Content-Type: text/html; charset=utf-8
Content-Length: 58

<!doctype html><title>Hello</title><h1>Hello from Aye</h1>
curl -i http://127.0.0.1:<port>/nope       # HTTP/1.1 404 Not Found
cargo test --test framework
```

**F1.2 (needs F1.1, O1.5).** `framework/cli/` (the `web` command):
`web gen <app> --web <dir of the web module>` scans `<app>/app/` and
writes `.web/` as in section 1; the hello app's routes move to
`app/page.aye`, `app/about/page.aye`, `app/not-found.aye`. The test runs
`web gen`, then `aye build`, and checks `/`, `/about` and an unknown
path. Symlinks are made with `std.fs.symlink` (and read back with
`std.fs.readLink`, O1.27). `--web` goes away with O4.2.

**F1.3 (needs F1.2, O3.1).** `app/api/items/[id]/route.aye` with
`pub fn get`; the transcript checks `200`, `Content-Type:
application/json` and `{"id":1,"name":"Anchor"}` for `/api/items/1`,
`404` and `{"error":"NotFound"}` for `/api/items/9` (the handler returns
`Error.NotFound`), and 405 for a `POST`. `web gen` reads what a
`route.aye` exports from `aye doc --signatures` (the `aye` named by `AYE`,
else the one on `PATH`).

## Foreseen language and std needs

To go into `browser/COMPILER_GAPS.md` (Hit by: the F step) when a step
meets them:

1. **Directory module names must be identifiers.** `[slug]`, `about-us`
   cannot be imported. Worked around by the symlink root (section 1);
   recorded so the Aye lane knows why `.web/` exists.
2. **`aye check --json` names symlink paths.** The CLI maps them back;
   a loader that reported the target path would remove the map. (F3)
3. **No closures.** Layouts take the body as bytes; shared state is a
   global behind a `Mutex`; `std.http.serve` must take a context pointer
   next to the handler, the `std.thread.spawn(T, f, arg)` shape. (O1.4,
   F1.1)
4. **`std.http` listens on port 0 and reports the address**, so tests
   get a free port without a race; `SO_REUSEADDR`, so the dev server can
   take the port back at once. (O1.3, O1.4, F1.1, F3)
5. **`Arena.reset`** keeping its first block, so a connection reuses one
   arena per request instead of mapping new memory each time. (F1.1, O1.7)
6. **`std.fs.symlink`** and **`readLink`**: done (O1.27). (F1.2)
7. **Errors carry no payload.** A 400 that says which field was wrong
   goes through the response (`res.status`, a body), not the error;
   fine, but worth a `docs/practices/` entry when F2 writes forms.
8. **One module, two targets.** An island's `render` (native) and its
   event code (wasm32) share types; std code needs a documented way to
   tell targets apart (O2.1), and a module that imports `std.thread` must
   fail on wasm32 with an error that names the target (O8). (F4)
9. **`@embedFile` of a directory** is not needed: the generator writes
   one `@embedFile` per asset of `public/` for FD7. Recorded as a
   non-need.
10. **Signals.** Graceful stop on `SIGTERM` (O3.6) needs a way to catch a
    signal; `std.os` has none. (F3)

## Decided questions

Decided 2026-10-12 (the AI decides, `docs/plan.md` "Who decides"):

1. **Names:** the framework is Aye Web; its command stays `web` and its
   build directory `.web/`, as the code, tests and docs already use.
2. **design.md wording:** the proposed non-goal above is taken as is
   (`design.md`, 目標與非目標).
3. **TypeScript:** type-stripping only, erasable syntax as Node 22
   does it; `enum`, `namespace` and parameter properties are an error
   that names the construct. Full TS needs code generation, a second
   compiler the framework does not want.
