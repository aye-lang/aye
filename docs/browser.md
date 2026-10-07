# anyknown browser

A web browser written in Ore, for two kinds of users at once:

- **Humans** get a window: address bar, back and forward, scrolling,
  links, forms, and pages with CSS and JavaScript.
- **AI agents** get the same engine without pixels: a page as clean
  Markdown, an accessibility tree whose interactive elements carry
  stable refs (`[e12] link "Docs"`), and actions on those refs (`click
  e12`, `type e7 "query"`). Agents talk to it over stdio as an MCP
  server or as JSON lines. A human window can also accept an agent, so
  both can drive one page and see what the other did.

It is the P3 validation of design.md: DOM nodes live in `Pool(Node)`, JS
holds `Handle(Node)`, everything is allocated explicitly, and the engine
only calls C for the platform (SDL2 for windows and input, FreeType for
glyphs, libcurl for HTTP and TLS). Parsing, style, layout, painting,
JavaScript and the agent protocol are Ore.

## Layout of the code

```
browser/
  main.ore        CLI: window mode, `dump`, `agent`, `mcp`, `connect`
  url/            URL parsing and resolution (WHATWG subset)
  net/            fetch: file:, data:, http(s) through libcurl, cache
  html/           tokenizer and tree builder (WHATWG subset), entities
  dom/            Node, Document, Pool(Node), queries, mutation; the
                  state of form controls (values, checkedness, focus)
  css/            tokenizer, parser, selectors, cascade, computed style
  layout/         block, inline (line breaking, UAX 14, bidi), lists,
                  tables, flex, grid, floats, positioned and sticky
                  boxes, transforms, framesets, text measurement
  engine/         Page (load, style, layout), Tab (history, scroll),
                  hit testing, find in page, images, default actions
                  and form submission, page scripts (DOM bindings,
                  prelude, event loop)
  image/          PNG (zlib inflate), GIF, JPEG (baseline, progressive,
                  CMYK) and WebP (VP8L, VP8) decoders
  svg/            SVG parsing and rasterizing (paths, shapes, gradients)
  font/           FreeType faces, fallback chain, HarfBuzz shaping
  webfont/        WOFF and WOFF 2 decoding for `@font-face`
  encoding/       WHATWG encodings: windows-1252 and the CJK ones
  paint/          display list, software rasterizer, glyph cache,
                  backgrounds, masks, shadows, PNG
  js/             lexer, parser, interpreter, GC, builtins, host interface
  agent/          snapshot (a11y tree with refs), Markdown, actions,
                  JSON-lines and MCP stdio servers, a window's agents
  ui/             SDL2 window, chrome (address bar, tabs), input, the
                  agent socket (`--agent-socket`, `connect`)
  tests/          fixtures (HTML/CSS/JS) and golden outputs
```

## Milestones

Same rules as `milestones.md`: tests first; a milestone is done when its
acceptance tests pass. Tests run headless and offline (`file:` fixtures
and a local test server), so CI needs no network and no display.

**Status.** B1–B7 are done; B8 is under way:

| | State | Tests (`cargo test --test browser`) |
|---|---|---|
| B1 fetch, HTML, DOM | done | 31 `dump --tree`/`--text` fixtures, `data:`, `http:` (local server) |
| B2 CSS and style | done | 25 `dump --style` fixtures |
| B3 layout | done | 31 `dump --layout` fixtures (fixed font), system-font check |
| B4 paint and window | done | 13 golden PNGs, PNG checks, headless window tests |
| B5 forms and images | done | form sessions over HTTP, window form scripts, image decoder unit tests, form and image screenshots |
| B6 JavaScript | done | 212 conformance scripts (`anyknown js`), 30 DOM fixtures (`dump --tree --js`, also under GC stress), a to-do app session, window script test, GC unit tests |
| B7 agent interface | done | 17 snapshot/Markdown fixtures, session and MCP transcripts (with `eval` and `wait`), shared-window sessions over its socket (JSON lines, MCP through `anyknown connect`, pause, pointing, tabs) |
| B8 the real web | in progress | net over the local server (`net_http`, the `net_cookies` and `base` agent sessions), HTML/CSS fixtures (templates, quirks, hints, `var()`/`calc()`, at-rules, `@import`), layout fixtures of site shapes (floats, positioned boxes, grid, `details`), hit testing by layer (`window_hit_layers`), JS conformance scripts for generators, async, Symbol and the web APIs |

All the front ends share `engine/`: the window, `screenshot` and the
agent sessions load pages through `engine.load` and keep history in a
`Tab`. What remains:

- **B6 leftovers:** modules, layout queries (`getBoundingClientRect` is
  all zeros), custom elements and Shadow DOM (in progress),
  `MutationObserver` reporting (in progress).
- **B8:** event listeners on another frame's nodes, moving nodes
  between documents, frame history entries; calendar, color and file
  pickers, `datalist`; rotations and skews other than quarter turns are
  not drawn; sticky only against the viewport; no public suffix list for
  cookies (a bare top-level name is refused, `co.uk` is not); no AVIF;
  page loads are slow on big sites (in progress), and so is the JS
  interpreter (in progress). The list of public sites is in
  [`docs/browser-sites.md`](browser-sites.md) (20 sites; run of
  2026-10-07: 14 ✅, 3 ⚠️, 3 ❌; the run of 2026-10-06, before the work
  below, had 8 ✅, 7 ⚠️, 5 ❌).

The compiler gaps the browser met, and which of them M9.5/M9.6 fixed,
are in `browser/COMPILER_GAPS.md`.

### B1: Fetch, HTML, DOM, text

- URL parsing and relative resolution; fetch for `file:`, `data:`,
  `http:` and `https:` (libcurl), with redirects and charset sniffing
  (UTF-8 and Latin-1).
- HTML tokenizer (data, tags, attributes, comments, doctype, character
  references, raw text for `script`/`style`, RCDATA for `title`/
  `textarea`) and a tree builder with implied `html`/`head`/`body`,
  void elements, auto-closing `p`/`li`/`td`/... and misnested formatting
  tags handled simply. It never fails.
- DOM in `Pool(Node)`: elements, text, comments; attributes;
  `getElementById`, `getElementsByTagName`, parent/children/siblings.
- **Acceptance:** `anyknown dump --tree FILE` and `anyknown dump --text
  FILE` golden tests on 20 fixtures, including malformed HTML.

### B2: CSS and style

- CSS tokenizer and parser (rules, declarations, `@media` with
  `screen`/width conditions, other at-rules skipped).
- Selectors: type, `.class`, `#id`, `*`, attribute (`[a]`, `[a=v]`),
  descendant, child, sibling combinators, `:first-child`, `:last-child`,
  `:not()`, `:hover`, `:link`, lists.
- Cascade with specificity, `!important`, inline `style`, a UA
  stylesheet, inheritance, `inherit`/`initial`.
- Computed values: display, position (static, relative, absolute,
  fixed; sticky as relative), float, clear, z-index, color,
  background-color, font-size/weight/style/family, line-height,
  margin/padding/border (widths, colors, styles), width/height/min/max,
  text-align, text-decoration, white-space, list-style-type, visibility,
  overflow; units px, em, rem, %, vw/vh, named and hex/rgb() colors.
- **Acceptance:** `anyknown dump --style` golden tests.

### B3: Layout

- Block formatting (margins with collapsing between siblings), inline
  formatting with line breaking and `white-space`, `text-align`, list
  markers, `<br>`, `<hr>`, replaced elements with intrinsic size
  (`img` placeholder boxes), simple tables (auto column widths), simple
  flexbox (row/column, gap, justify-content, align-items, flex-grow).
- Text measurement through FreeType with a bundled-path fallback font;
  headless tests use a fixed-metric font so goldens are stable.
- **Acceptance:** `anyknown dump --layout` golden tests (box tree with
  coordinates).

### B4: Paint and window

- Display list (rects, borders, text runs, images); software rasterizer
  into an RGBA buffer with alpha-blended glyphs from a glyph cache.
- `anyknown screenshot FILE -o out.png` (PNG encoder in Ore).
- SDL2 window: address bar, back/forward/reload, scrolling (wheel and
  keys), hover cursor on links, clicking links, window resize, Cmd+L,
  Cmd+[ and Cmd+], find in page (Cmd+F).
- **Acceptance:** screenshot golden tests compared by pixel hash; a
  manual checklist run once per milestone for the window.
- Done: `anyknown screenshot FILE|URL -o out.png [--width W] [--height
  H] [--font fixed|system] [--full]` and `anyknown [open] FILE|URL`.
  Golden PNGs (`browser/tests/paint/`, fixed font, compared byte for
  byte), PNG checks, and headless window tests (SDL's dummy driver,
  `--frames N` with scripted `--events`, `--state`, `--save`; see
  `browser/ui/script.ore`). The interactive checks are in
  `docs/browser-manual-checklist.md`. Simplifications: positioned boxes
  are painted after the rest of their layer in z-index order (negative
  ones first) and hit testing searches in the reverse order, but there
  are no other stacking contexts (opacity, transforms), clips are rectangles,
  the 3-D border styles are solid, and the style sheets are parsed again
  when the window is resized.

### B5: Forms and images

- `input` (text, password, checkbox, radio, submit), `button`,
  `textarea`, `select`; focus and keyboard editing; form submission by
  GET and POST (urlencoded).
- Images: PNG (zlib inflate in Ore) and GIF first frame; JPEG through
  a later milestone.
- **Acceptance:** a fixture form submitted to the local test server
  produces the expected request; image fixtures render to golden
  screenshots.
- Done:
  - *State* (`dom/controls.ore`): one place for what the window, the
    agent and scripts share: `doc.controls` keyed by node (a field's
    value and dirty flag, checkedness, the selectedness of options, the
    caret and selection, a field's scroll offset) and the focus.
    Attributes keep the defaults; a reset forgets the state. Functions:
    `dom.valueOf`/`setValue`, `isChecked`/`setChecked` (radio groups
    exclusive), `selectedOption`/`selectOption`/`isSelected`/
    `selectedIndex`, `formOwner`, `isDisabled` (fieldsets, optgroups),
    `resetForm`, and editing (`insertText`, `deleteBackward`,
    `deleteForward`, `moveCaret`, `selectAll`, `setCaret`). CSS matches
    `:checked`, `:disabled`, `:enabled` and `:focus` against it.
  - *Default actions* (`engine/forms.ore`), for the event loop to call
    after dispatching (unless the default was prevented):
    `engine.activate(page, node)` returns what the caller does next:
    `Follow { target }` (a link), `Submit { form, submitter }` (fire
    `submit`, then `engine.submitForm`), `OpenSelect { el }`, `Changed`
    (a checkbox, radio button, reset button, label, summary, or the
    focus moved: style and lay out again) or `Nothing`.
    `engine.implicitSubmit(page, field)` is Enter in a text field (the
    first submit button, or no button when the form has one field).
    `engine.submitForm` builds the request: GET (fields in the query)
    or POST as application/x-www-form-urlencoded, multipart/form-data
    (a fixed boundary; file inputs send an empty file) or text/plain;
    `formaction`, `formmethod`, `formenctype`; the submitter's name and
    value, an image button's `x`/`y`. `engine.navigateTo(tab, sub)`
    loads it (POST through `net.fetchWith` with a body and
    `Content-Type`; a redirect after a POST continues with a GET). The
    history keeps URLs only: going back to a POST result loads it by
    GET. Focus: `setFocus`, `nextFocus` (Tab order with `tabindex`).
  - *Layout and paint* (`layout/controls.ore`): text fields, selects and
    input buttons are inline-blocks with generated text (bullets for a
    password, the placeholder in gray, the selected option's label);
    `size`, `cols`, `rows` give default sizes; a `select` with
    `multiple` or `size` > 1 is a list box with its selected options
    highlighted; a `textarea` wraps; text fields scroll to their caret;
    checkboxes and radio buttons are drawn replaced boxes; disabled
    controls are gray (UA sheet). `fieldset`/`legend` are plain blocks
    with a border (the legend does not sit on the border).
  - *Window* (`ui/forms.ore`): click, Tab and Shift+Tab focus; a focus
    ring; typing (SDL text input, so IME and UTF-8), Backspace, Delete,
    arrows, Home/End, Shift to select, Cmd+A/C/X/V (SDL clipboard);
    Enter submits (a new line in a `textarea`), Space toggles and
    presses; a `select` opens a dropdown over the page (arrows, Enter,
    Escape, clicks). `--state` adds the focused element and its value.
  - *Agent*: `click` on a submit button and `type` with `submit` submit
    for real; `type`, `select`, `press` and the snapshot use the same
    state as the window.
  - *Images* (`image/`, `engine/images.ore`): PNG (inflate with stored,
    fixed and dynamic Huffman blocks; every color type and bit depth,
    `PLTE`, `tRNS`, Adam7), GIF (first frame, LZW, transparency,
    interlace), baseline JPEG (Huffman, IDCT, YCbCr, chroma
    subsampling, restart markers). Images are fetched once per URL
    (relative, `data:`) before each layout for elements that have none
    yet; the natural size and aspect ratio feed layout (`width`/`height`
    attributes and CSS), and paint scales them (bilinear, alpha
    blended). Not yet: `srcset`, `background-image`, progressive JPEG,
    animation.
  - Tests: `agent_forms_http` (sessions over the local server's
    `/echo`, which shows the request), `window_forms`, `fill_form`,
    the `image` module's decoder tests (PngSuite-like fixtures made by
    `browser/tests/image/make_fixtures.py`), the `paint/forms`,
    `paint/decoded` and `layout/images` goldens.

### B6: JavaScript

- Lexer, parser and tree-walking interpreter for an ES5 core plus
  `let`/`const`, arrow functions, template literals, classes,
  destructuring (simple), spread, `for...of`; closures, prototypes,
  exceptions; `Object`, `Array`, `String`, `Number`, `Math`, `JSON`,
  `Date.now`, `console`.
- Mark-and-sweep GC over the interpreter's own heap.
- DOM bindings following design.md's JS boundary: wrappers hold
  `Handle(Node)` and a `js_refs` count. `document.getElementById`,
  `querySelector(All)`, `createElement`, `appendChild`,
  `removeChild`, `textContent`, `innerHTML` (set), `setAttribute`,
  `classList`, `style.*`, `addEventListener` (click, input, submit,
  keydown, DOMContentLoaded, load), `setTimeout`, `fetch` (simple).
- **Acceptance:** a JS conformance suite of 150 small scripts with
  golden console output; DOM fixtures whose `dump --tree` after scripts
  matches goldens; a to-do app fixture that works in the window.

**The engine core (done).** `anyknown js FILE` runs a
script; `browser/tests/js/` holds the conformance scripts with Node's
output as goldens.

- *Pipeline:* tokens (all up front) → AST in one `List(Node)` per engine
  → a resolver that gives every variable an environment slot (`depth`,
  `slot`) → a tree-walking interpreter. Exceptions are the Ore error
  `JsError.Thrown` (the value in `Interp.exception`), so they pass through
  native functions with `try`.
- *Strings:* WTF-8 bytes (UTF-8 that can hold lone surrogates) with the
  UTF-16 length cached, as decided in design.md: the DOM and the engine
  share UTF-8, and `length`, `charCodeAt` and indices follow UTF-16.
  ASCII strings index their bytes; others are decoded when an operation
  needs positions. String comparison is by bytes (code point order,
  which differs from UTF-16 order only between astral characters and
  U+E000..U+FFFF). A concatenation of 64 bytes or more makes a rope that
  is copied flat only when its bytes are needed, so `s += x` in a loop
  is linear.
- *Heap and GC:* objects, strings and environments are `u32` indices into
  three lists with free lists; mark and sweep from the roots (global
  object, intrinsics, environments, the value stack, frames, pending
  exception and return value, host roots), every 2× live allocations.
  A value held across an allocation must be on the interpreter's stack
  (calls take `[f, this, args...]` from it). `ANYKNOWN_JS_GC=stress`
  collects at every allocation to check that discipline.
- *Host interface:* native functions are a table of `fn(*mut Interp,
  Args) JsError!Value`; `Interp.newHostObject(proto, tag, index,
  generation)` makes a wrapper that holds a `Handle(Node)` as three
  integers, and `Interp.host: ?dyn Host` gets `finalize(tag, index,
  generation)` when the collector frees one (the `js_refs` decrement).
  `addRoot`/`removeRoot` keep values (event listeners) alive.
- *Generators and async functions* (B8, `js/coroutine.ore`): a call
  that can stop in the middle of the tree walk runs on a `std.coro`
  coroutine (a C stack of its own, M12; a thread and two channels
  before), with its own value stack and frames, on the engine's thread. Promise
  and the job queue are the engine's (`js/lib.ore`, `Interp.queueJob`);
  `await` hands the value to a driver written in JavaScript that resumes
  the coroutine when the promise settles; `for await` and async
  generators go through the same driver.
- *Not yet:* BigInt, Proxy, typed arrays, time zones (local time is
  UTC), full Unicode case mapping and normalization.
  A plain function call gets `this === undefined` (strict-mode rule);
  a script's top-level `this` is the global object, and top-level
  `var`/`function` are properties of it. HTML-like comments (`<!--`,
  and `-->` first on a line) are comments, as in classic scripts.

**Pages (done).** `engine/script.ore` gives every page an engine
(`Page.script`) when scripting is on: the window and the agent turn it
on (`--no-js` turns it off), `dump` with `--js`.

- *Loading:* after the parse, the `script` elements run in document
  order (inline, or `src` fetched relative to the page); external
  `defer` and `async` scripts run after the others (async as defer);
  `type=module` and data types are skipped; then DOMContentLoaded
  (`readyState` "interactive") and load ("complete"). An uncaught
  exception is a console message and the next script runs; so is a
  promise still rejected with no handler at the end of a microtask
  checkpoint ("Uncaught (in promise) ..."). With
  scripts on, `noscript` is `display: none` (a UA rule).
- *Bindings:* a node's wrapper is a host object holding its
  `Handle(Node)`, one per node (`Script.wrappers`), each counting in
  the node's `js_refs` (also the `style` and `dataset` objects). A
  collected wrapper decrements it (`Host.finalize`), and a tree out of
  the document is freed once no node of it is held
  (`Document.releaseIfUnused`). The collector asks the host to keep
  alive (`Host.trace`) the wrappers of nodes in the document and in any
  tree a reachable wrapper is in, so listeners and expando properties
  last as long as their nodes. A stale handle is a TypeError, never a
  use after free. Natives (`dom_api.ore`): Node (tree, text,
  `appendChild`/`insertBefore`/`removeChild`/`replaceChild`/`remove`,
  `cloneNode`, `contains`), Element (attributes, `id`, `className`,
  `innerHTML` get/set through the HTML parser with the context's table
  or select wrapping, `outerHTML`, `children`, `matches`, `closest`,
  `querySelector(All)` through the CSS selector matcher,
  `getElementsBy*`), Document (`body`, `head`, `title` get/set,
  `readyState`, `activeElement`, `currentScript`, `create*`), and form
  state through `doc.controls` (`value`, `checked`, `selected`,
  `selectedIndex`, focus). `style.camelCase` and `dataset.camelCase`
  are named host properties (`Host.getNamed`/`setNamed`) over the
  `style` and `data-*` attributes. Lists are arrays made at the call
  (not live).
- *Prelude* (`prelude.ore`, JavaScript): EventTarget with capture,
  target (capturing listeners first) and bubble phases through the
  window, `once`, `stopPropagation`, `stopImmediatePropagation`,
  `preventDefault`; Event, CustomEvent, MouseEvent, KeyboardEvent,
  FocusEvent, InputEvent; `onclick`-style properties and attributes
  (compiled when first needed, last writer wins, `return false`
  cancels); `append`/`prepend`/`before`/`after`/`replaceWith`,
  `insertAdjacentHTML`, `classList`, reflected attributes, `click()`
  (the click event, then `engine.activate` unless canceled), `focus()`;
  timers, `requestAnimationFrame`, `queueMicrotask`, `location`
  (setting it navigates after the script returns), `history`,
  `localStorage` (in memory, per origin, while a page is open) and
  `sessionStorage`, `alert`/`confirm`/`prompt` (console messages, never
  blocking), microtasks (the engine's job queue, which Promise uses),
  and `fetch` (GET, synchronous underneath, settling in a microtask;
  `text()`, `json()`, an aborted `signal`).
- *More web APIs* (B8, prelude): `URL` and `URLSearchParams` (parsed
  by `url/`), `atob`/`btoa`, `TextEncoder`/`TextDecoder` (UTF-8),
  `EventTarget` as a constructor, `AbortController`/`AbortSignal`,
  `matchMedia` (the CSS media queries against the viewport),
  `XMLHttpRequest` (GET, on the same fetch), `crypto.getRandomValues`
  and `randomUUID` (from `Math.random`), `requestIdleCallback`,
  `reportError`, `Image`, `Option`, `origin`, and `customElements`
  (definitions kept, elements never upgraded).
  `IntersectionObserver` checks its targets against the layout and the
  scroll offsets (`engine/geometry.ore`), so content below the fold is
  not intersecting until scrolled into view; `ResizeObserver` reports
  each target once. `loading="lazy"` images and iframes load at once,
  as if in view.
- *Event loop:* timers (ordered by due time, then by scheduling) and
  microtasks after every task. Time is virtual for `dump` and the agent
  (the loop jumps to the next timer; `dump --js` runs until no timer is
  left, at most 30 s of page time) and real in the window, whose
  `SDL_WaitEventTimeout` wakes for the next timer. Clicks and keys
  (window, agent) fire their events first and skip the default action
  when canceled; checkbox toggles fire `input` and `change`; a submit
  fires `submit`. A scroll of the window by the user, an agent or a
  script fires `scroll` at the document (bubbling to the window) as a
  task; the agent's viewport follows a script's `scrollTo`.
- *Scrolling through for dumps:* `dump --js` (every mode, `--md` and
  `--a11y` too) then scrolls through the page as a reader would, so
  that what scripts load when it comes into view (observed sections,
  lazy images, infinite lists) is in the dump
  (`engine.scrollThrough`): 500 ms of page time where the page is,
  then a step of one viewport height down (firing `scroll`) and 500 ms
  again, until the bottom is shown and the document stopped growing,
  or after 20 steps (`--scroll-steps N`; `--no-scroll` turns it off).
  The window then goes back to the top at once, without the page
  reacting, so content it would drop out of view stays in the dump. It
  is deterministic (virtual time); a page that grows on every scroll
  ends at the cap. The agent's `read` and `snapshot` do not scroll:
  an agent scrolls with `scroll`, then `wait`. After scripts changed the document (`domVersion`),
  the front end styles (style sheets parsed again) and lays out again;
  a navigation they asked for (`takeNavigation`) happens then.
- *Console:* messages ("[log] ...", "[error] Uncaught ...") go to
  stderr for `dump --js` and the window, and with the agent's
  responses (`console`).
- *Stack:* page scripts run on the main thread, so a task may use 5 MB
  of its stack and nest 3000 calls (a RangeError beyond).
- *Budget:* a task (a script, a timer callback, an event, the
  microtasks of a checkpoint) that uses more than 2 s of processor time
  or 100M steps
  ends with a console error, which no `catch` or `finally` sees.

### B7: Agent interface

- `anyknown agent`: JSON lines on stdio. Commands: `navigate`, `back`,
  `forward`, `reload`, `snapshot` (accessibility tree with refs, roles,
  names, values, states), `read` (Markdown of the main content with
  link refs), `click`, `type`, `select`, `press`, `scroll`, `eval`,
  `screenshot`, `wait` (for load or a selector), `tabs`.
- `anyknown mcp`: the same commands as MCP tools over stdio JSON-RPC.
- `anyknown --agent-socket PATH` with the window: a human and an agent
  share the page; agent actions are highlighted in the window.
  `anyknown connect PATH [--mcp]` bridges stdio to that socket, so an
  MCP client can drive the window a person sees.
- **Acceptance:** scripted agent sessions over fixtures (fill and submit
  a form, follow a link by ref, read a JS-rendered list) with golden
  transcripts; an MCP handshake and `tools/list` golden.
- The protocol is in `docs/agent-protocol.md`. Done so far: snapshot,
  read, navigation, click, type, select, press, scroll, find, `eval`
  and `wait` (scripts run; actions fire their events), `dump --a11y`
  and `dump --md` (`--js` to run scripts).
- The shared window (`ui/agents.ore`, `ui/socket.ore`, `ui/tabs.ore`,
  `agent/shared.ore`): the window polls a non-blocking Unix socket
  between its events, so requests run on its thread; each tab is an
  agent session laid out with the window's fonts. Tabs (Cmd+T, Cmd+W,
  Cmd+1..9, a tab strip) and the commands `tabs`, `tab_new`,
  `tab_select`, `tab_close` (an agent can work in a background tab);
  visible activity (a fading outline with the ref, an `AI` badge, the
  status line); pause (button, Cmd+Shift+P: actions fail with "paused
  by user"); Cmd+Shift+K sends a `user_pointed` event, and
  `user_context` returns the selection, the hovered and focused refs,
  the scroll offset and the URL. `wait` takes real time there.

### B8: The real web

- HTTPS sites render readably and agents can operate them: a fixed list
  of public pages (example.com, a Wikipedia article, Hacker News, a
  documentation site) checked manually and recorded in
  [`docs/browser-sites.md`](browser-sites.md); cookies, gzip, `<meta charset>`, `<base>`,
  `<noscript>`.
- Done so far:
  - *Net* (`net/session.ore`, `net/cookies.ore`, `net/http.ore`): one
    session per process with one libcurl handle (connections, TLS
    sessions and DNS kept), gzip/deflate/br, a cookie jar (Domain, Path,
    Expires, Max-Age, Secure, HttpOnly, `__Secure-`/`__Host-`;
    `ANYKNOWN_COOKIE_FILE` keeps persistent cookies between runs,
    `document.cookie` reads and writes it), a subresource cache,
    redirects followed by hand (each hop stores its cookies), Accept by
    kind, Accept-Language, Referer (strict-origin-when-cross-origin), a
    User-Agent with "anyknown" (also `navigator.userAgent`), 32 MB / 16
    MB size limits, TLS errors. `<base href>` for links, forms, images,
    sheets, scripts, `fetch` and `location`. A page's images, sheets and
    scripts are fetched concurrently (libcurl multi, 6 connections per
    host). The `<meta charset>` prescan follows WHATWG's (all the labels
    of windows-1252 and UTF-8).
  - *HTML and CSS:* `template` contents in an inert fragment, foster
    parenting, quirks mode from the doctype, `noscript` following
    scripting in the dumps, svg/math text kept out of them;
    presentational hints (`bgcolor`, `font`, `align`, `width`, table
    `border`, `cellpadding`, ...) and legacy color parsing; custom
    properties and `var()`, `calc()`/`min()`/`max()`/`clamp()` (also in
    media queries); `@supports`, `@layer` (as plain rules), `@import`
    (relative to the importing sheet, at most 4 deep), media queries
    level 4, `hsl()`, `light-dark()`, system colors; `font-family`
    names map to sans, serif or monospace faces, serif by default.
  - *Layout and paint:* floats and `clear`; `position: absolute` and
    `fixed` (out of flow, laid out last in the containing block);
    `display: grid` (track lists with `fr`, `minmax`, `repeat`, areas,
    line placement, auto-placement); replaced boxes (svg, iframe,
    embed, object, video, canvas) sized 300x150 by default; z-index
    layers in paint and hit testing; frames shown as links; a closed
    `details` shows only its `summary`.
  - *JavaScript:* generators and async functions, Symbol and the
    iterator protocol, `Function` and `eval` of strings, the web APIs
    above, a processor-time budget per task (B6, above).
  - *JS and DOM APIs for real sites:* the init of a `for` loop is
    resolved (jQuery 3, minified bundles); Annex B block functions;
    sloppy-mode `this`; `\u` escapes in identifiers; `Proxy`; Array
    methods on array-likes. `createElementNS`, `DocumentFragment`,
    `CustomEvent`/`initCustomEvent`, the Event constructors, interface
    globals per tag that work with `instanceof` and prototype patching;
    select `add`/`remove`/`options`/`selectedIndex`, `:scope`,
    `performance` timing/navigation/mark/measure, live collections,
    `document.write` while parsing, TreeWalker/NodeIterator, Range,
    DOMParser, Blob/File/FileReader, FormData, MessageChannel,
    constructable style sheets, `a` URL parts, Navigator;
    `getComputedStyle` styles only the element and its ancestors. The
    checklist's scripted sites (Python docs, rustdoc, CNN Lite,
    react.dev, YouTube, go.dev, BBC, Google, ZOL) log no errors.
  - *Layout of current CSS:* `transform` and `transform-origin` (the
    translation moves the box after layout; scales, flips and quarter
    turns are drawn and hit-tested); a transformed box is a layer and
    the containing block of its absolute and fixed descendants.
    `position: fixed` boxes stay in the viewport while scrolling and
    `sticky` ones stick within their parent (`layout/scroll.ore`).
    `::before`/`::after` with `content` (strings, `attr()`, quotes);
    `:is()`/`:where()`; logical properties; `aspect-ratio`;
    `object-fit`; `clip: rect()` and `clip-path: inset()`. Percentage
    table columns and `<center>` around auto-width tables; grid gaps
    under spanning items; flex items that clip their overflow have no
    automatic minimum.
  - *SVG* (`svg/`): inline `<svg>` and SVG images (`<img>`, CSS
    images, `data:` URLs) become pictures: `path` (all commands, arcs as
    cubics), `rect` (rounded), `circle`, `ellipse`, `line`, `polyline`,
    `polygon`, `g`, nested `svg`, `use`/`symbol`, `transform`, `viewBox`
    and `preserveAspectRatio`. Presentation attributes are hints in the
    cascade, with the properties `fill`, `stroke`, their opacities,
    `stroke-width`, `fill-rule`, caps, joins, dashes, `opacity`,
    `stop-color`; `currentColor` works. Linear and radial gradients are
    paint servers; group `opacity` is drawn into its own layer;
    `clip-path` from a `clipPath`. Curves are flattened, strokes become
    polygons, fills use antialiased scanlines (nonzero and even-odd).
  - *Backgrounds:* `background-image` (images and linear, radial,
    conic and repeating gradients), several layers, `-position`,
    `-size`, `-repeat` (including space and round), `-origin`, `-clip`,
    a basic `fixed` attachment, the `background` shorthand; images in
    style sheets are fetched with the page's and resolved against their
    sheet. `mask-image` (and `mask`, `-webkit-` names), `border-radius`,
    blurred outer and inset `box-shadow`.
  - *Images:* progressive JPEG, Adobe CMYK and YCCK, EXIF orientation;
    WebP lossless (VP8L) and lossy (VP8, bit-exact with libwebp) with
    VP8X, ALPH alpha and the first frame of an animation; formats are
    sniffed. `picture` sources, `srcset` (x and w) and `sizes` pick an
    image's URL; image requests accept WebP.
  - *Fonts and text* (`font/`, `webfont/`, `css/fontface.ore`,
    `layout/bidi.ore`, `layout/linebreak.ore`): a character the font
    lacks comes from a chain of system fallback fonts (PingFang,
    Hiragino, Apple SD Gothic Neo, Arial Unicode, Apple Symbols, STIX,
    Apple Color Emoji; Noto and DejaVu on Linux). Text is shaped by
    HarfBuzz when it is installed (complex scripts, emoji sequences,
    ligatures), and layout measures with the same shaping. East Asian
    line breaks follow basic UAX 14; bidi basics (`direction`, `dir`,
    levels, lines reordered). `@font-face` in TTF, OTF, WOFF and WOFF 2
    (Brotli), matched by family, weight, style and unicode-range, icon
    font ligatures included.
  - *Encodings and frames:* Shift_JIS, EUC-JP, ISO-2022-JP, GBK,
    gb18030, Big5 and EUC-KR (WHATWG decoders and encoders, tables
    generated from the index files) in the meta prescan and
    Content-Type; forms submit in the page's encoding or
    `accept-charset`. `<frameset>`/`<frame>`: the frameset replaces the
    body, rows and columns (px, %, *) are laid out, each frame's
    document is loaded, laid out, painted and scrolled in its
    rectangle; `--md` puts each frame under `## Frame: URL`, the
    snapshot has `frame` nodes.
  - *Agent Markdown:* invisible list items are dropped, layout tables
    are read row by row, `pre` keeps its line breaks (also in the
    snapshot).
  - *Iframes:* `srcdoc`, `src` or about:blank load as pages of their
    own; `sandbox` turns off scripts and gives the frame its own
    origin; at most 3 deep and 16 frames per tab. Scripts use
    `parent`, `top`, `frames[i]`, `frameElement`, `contentWindow` and
    `contentDocument` (same origin), `postMessage` between frames and
    iframe `load` events; iframes added by scripts, or whose `src`
    changes, load at the next layout. Links and forms follow their
    `target`. The snapshot and `read` give refs inside frames and agent
    actions work there; in the window, clicks reach the frame's
    document.
  - *HTML5 forms* (by GPT-6.1-sol through pi): text-like and calendar
    inputs (date, time, datetime-local, month, week), number controls,
    draggable range sliders, color swatches, file placeholders,
    `output`, `progress` and `meter`. Constraint validation (required,
    pattern, lengths, numeric and calendar bounds and steps, email and
    URL mismatches, custom errors) with live `ValidityState`,
    `checkValidity`/`reportValidity`/`setCustomValidity`,
    `requestSubmit`, `reset` and FormData from the form's entries.
    Invalid submissions fire `invalid`, focus the first invalid control
    and show a tooltip in the window or a message to the agent. Form
    owners (`form=`), labels, disabled fieldsets, submitter overrides
    (`formaction`, `formmethod`, `formenctype`, `formnovalidate`),
    `multipart/form-data`, the state pseudo-classes, and the a11y roles
    and states (spinbutton, slider, required, invalid).
