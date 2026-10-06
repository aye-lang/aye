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
  main.ore        CLI: window mode, `dump`, `agent`, `mcp`
  url/            URL parsing and resolution (WHATWG subset)
  net/            fetch: file:, data:, http(s) through libcurl, cache
  html/           tokenizer and tree builder (WHATWG subset), entities
  dom/            Node, Document, Pool(Node), queries, mutation; the
                  state of form controls (values, checkedness, focus)
  css/            tokenizer, parser, selectors, cascade, computed style
  layout/         block, inline (line breaking), lists, tables (simple),
                  flex (simple), text measurement
  engine/         Page (load, style, layout), Tab (history, scroll),
                  hit testing, find in page, images, default actions
                  and form submission
  image/          PNG (zlib inflate), GIF and baseline JPEG decoders
  paint/          display list, software rasterizer, glyph cache, PNG
  js/             lexer, parser, interpreter, GC, builtins, DOM bindings
  agent/          snapshot (a11y tree with refs), Markdown, actions,
                  JSON-lines and MCP stdio servers
  ui/             SDL2 window, chrome (address bar, tabs), input
  tests/          fixtures (HTML/CSS/JS) and golden outputs
```

## Milestones

Same rules as `milestones.md`: tests first; a milestone is done when its
acceptance tests pass. Tests run headless and offline (`file:` fixtures
and a local test server), so CI needs no network and no display.

**Status.** B1–B5 are done, and so are the cores of B6 and B7:

| | State | Tests (`cargo test --test browser`) |
|---|---|---|
| B1 fetch, HTML, DOM | done | 31 `dump --tree`/`--text` fixtures, `data:`, `http:` (local server) |
| B2 CSS and style | done | 25 `dump --style` fixtures |
| B3 layout | done | 31 `dump --layout` fixtures (fixed font), system-font check |
| B4 paint and window | done | 13 golden PNGs, PNG checks, headless window tests |
| B5 forms and images | done | form sessions over HTTP, window form scripts, image decoder unit tests, form and image screenshots |
| B6 JavaScript | engine core done | 212 conformance scripts (`anyknown js`) |
| B7 agent interface | core done | 17 snapshot/Markdown fixtures, 4 session and MCP transcripts |
| B8 the real web | not started | |

All the front ends share `engine/`: the window, `screenshot` and the
agent sessions load pages through `engine.load` and keep history in a
`Tab`. What remains:

- **B6:** DOM bindings (`document`, wrappers holding `Handle(Node)`),
  events and an event loop (`setTimeout`, `addEventListener`), running
  page scripts during load; then the DOM fixtures and the to-do app.
- **B7:** actions that reach the page through JS and forms (`eval`,
  real input and submission), `screenshot` and `tabs` commands, and the
  shared window (`--agent-socket`) where a human and an agent drive one
  page.
- **B8:** the real web (cookies, gzip, `<base>`, a list of public sites).

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
- Computed values: display, position (static/relative), color,
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
  `docs/browser-manual-checklist.md`. Simplifications: painting follows
  tree order (no z-index or stacking contexts), clips are rectangles,
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

**The engine core (done; DOM bindings next).** `anyknown js FILE` runs a
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
- *Not yet:* generators, async/Promise, Symbol, BigInt, Proxy, typed
  arrays, tagged templates, `new.target`, time zones (local time is UTC),
  full Unicode case mapping and normalization.
  A plain function call gets `this === undefined` (strict-mode rule), and
  top-level `var`/`function` are properties of the global object.

### B7: Agent interface

- `anyknown agent`: JSON lines on stdio. Commands: `navigate`, `back`,
  `forward`, `reload`, `snapshot` (accessibility tree with refs, roles,
  names, values, states), `read` (Markdown of the main content with
  link refs), `click`, `type`, `select`, `press`, `scroll`, `eval`,
  `screenshot`, `wait` (for load or a selector), `tabs`.
- `anyknown mcp`: the same commands as MCP tools over stdio JSON-RPC.
- `anyknown --agent-socket PATH` with the window: a human and an agent
  share the page; agent actions are highlighted in the window.
- **Acceptance:** scripted agent sessions over fixtures (fill and submit
  a form, follow a link by ref, read a JS-rendered list) with golden
  transcripts; an MCP handshake and `tools/list` golden.
- The protocol is in `docs/agent-protocol.md`. Done so far: snapshot,
  read, navigation, click, type, select, press, scroll, find and wait
  (forms and scripts record events), `dump --a11y` and `dump --md`.

### B8: The real web

- HTTPS sites render readably and agents can operate them: a fixed list
  of public pages (example.com, a Wikipedia article, Hacker News, a
  documentation site) checked manually and recorded in
  `docs/browser-sites.md`; cookies, gzip, `<meta charset>`, `<base>`,
  `<noscript>`.
