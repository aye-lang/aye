# Browser practices

Conventions and pitfalls of the browser code in `browser/`, learned from
merges, reviews and real sites. Line numbers are as of `b548a82`.

## Merging branches

Several branches change the browser at once. Most merge breakage came
from shared numberings and shared names.

### Append CSS properties at the end of the table
**Context:** adding a CSS property in `browser/css/css.ore`.
**Bad:** every property is a constant index into `PROPERTIES`, and
`NPROPS` sizes the value arrays. The layout, SVG, fonts and backgrounds
branches each appended at the same index, so every merge conflicted and
renumbered. `DIRECTION` was 67 in `e76386d`, 87 after the fonts merge
`71ff9e1` and 86 after the backgrounds merge `8b670fb`.
**Good:** append the row at the end of `PROPERTIES` (`css.ore:227`, last
row `css.ore:340`), add its constant after the last one (`css.ore:216`),
and raise `NPROPS` (`css.ore:108`) and the array length `[111]Property`
together. At a merge, renumber one side's block and check that each
constant still names its row (`b548a82` moved `TEXT_TRANSFORM` from 105
to 110, after the CSS-correctness properties). Elsewhere use the constant or
`css.propertyIndex(name)`, never a number.
**Why:** the index is the identity of a property in every computed style;
a stale number reads another property's value without any error.

### Host object tags are one shared numbering
**Context:** adding a kind of host object to the JS engine
(`browser/engine/script.ore:52`).
**Bad:** the JS/DOM branch took `TAG_LIST = 0x104` (`8875025`) while the
iframes branch took `TAG_WINDOW = 0x104` (`2d4dd7e`). The merge
`0e7d95d` moved the window to `0x105`.
**Good:** take the next free number and list it with the others:
```
let TAG_NODE: u32 = 1
let TAG_STYLE: u32 = 0x102
let TAG_DATASET: u32 = 0x103
let TAG_LIST: u32 = 0x104
let TAG_WINDOW: u32 = 0x105
```
(`script.ore:55-65`). After a merge, check that no two tags are equal.
**Why:** a tag decides how the engine reads a host object's fields; two
kinds with one tag dispatch to the wrong code.

### A module's files share one namespace
**Context:** adding a top-level function to a file in a module
directory (`paint/`, `css/`, ...).
**Bad:** the backgrounds branch added `fn fit` in
`browser/paint/background.ore`, and `paint.ore` had a parameter `fit` in
`fitted()`. A local cannot shadow a top-level name of its module, so the
merge `8b670fb` did not compile and renamed the function to `fitRadii`.
**Good:** give module-level helpers specific names (`fitRadii`,
`background.ore:75`) and build the browser after every merge. The rule is
in `docs/decisions.md:302`.
**Why:** short names like `fit`, `px` or `scale` are also common local
names, in any file of the module.

### Give a new struct field a default when one value is neutral
**Context:** adding a field to a struct that other branches build with
literals.
**Bad:** the images branch added `density` to `ImageRef`; the
backgrounds branch had added `putUrl`, which built an `ImageRef` without
it. After the merge `1c1d8bb` the browser did not compile until
`9c0eb47` added `density: 1.0`.
**Good:** when a neutral value exists, declare it in the struct
(`density: f64 = 1.0`), so literals that do not care can leave it out.
Leave the field required when every literal must choose; the compiler
then lists them all (`docs/decisions.md:318`).
**Why:** a default keeps another branch's new literals valid; a required
field is the right choice only when a silent default would be wrong.

## CSS

### Do not alias a shorthand to a longhand
**Context:** the alias table `logicalAlias` in `browser/css/values.ore:125`.
**Bad:** the layout branch (`451e86a`) aliased `mask` and `-webkit-mask`
to `mask-image`. When the backgrounds branch parsed `mask` as a real
shorthand (`values.ore:374`), the alias sent it to `mask-image` first, so
the merge `8b670fb` removed those pairs.
**Good:** aliases map one longhand name to another (logical to physical).
A shorthand gets its own expansion (`values.ore:262`).
**Why:** an alias runs before shorthand parsing, so it hides the
shorthand from the code that knows how to split it.

### Collect a property's resources from `::before` and `::after` too
**Context:** a property that loads something (images of
`background-image`, `mask-image`).
**Bad:** `styleUrls` fetched only the elements' own images, so a
background on a `::before` box never loaded (fixed in `13a070c`).
**Good:** also read the pseudo-element styles, as
`browser/engine/images.ore:331` does:
```
for which in 1..3 {
    if styles.pseudo(h, which as u8) |pv| {
        try css.layerUrls(alloc, pv[css.BACKGROUND_IMAGE], out)
```
**Why:** pseudo-elements have their own computed styles, outside the
element's values; sites use them for icons and decorations.

### Map a sizing attribute to a hint, not to a layout fallback
**Context:** an HTML or SVG attribute that sizes an element (`width`
and `height` on `img`, `svg`, `video`, `iframe`).
**Bad:** `replacedDims` read the attributes when the CSS `width` or
`height` was `auto`. Layout cannot tell an explicit `height: auto` from
no rule, so `img { max-width: 100%; height: auto }` kept the
attribute's height: 400x500 for a 1000x500 image (fixed in P3.4).
**Good:** write the attribute as a presentational hint
(`browser/css/hints.ore`, `sizeHint`, `svgSizeHint`), so that the
cascade lets any author value win. Layout reads only the computed style.
**Why:** a hint has the lowest author priority; only the cascade knows
whether a rule set the property.

## Paint

### A new display item kind needs an arm in `transformed()` and `moved()`
**Context:** adding a variant to `Item` (`browser/paint/paint.ore:71`).
**Bad:** the SVG branch added `Item.Svg`, the layout branch added
`transformed()` without it, and the layout merge `7b975d5` did not compile
until `4091dd2`. The backgrounds merge `8b670fb` had to add `Tiles`,
`Shadow`, `Layer`, `Mask` and `EndLayer` to both functions.
**Good:** when you add a variant, handle it in `transformed()`
(`paint.ore:936`), `moved()` (`paint.ore:1038`) and `render()`
(`paint.ore:1093`). These matches have no `_` arm, so the
compiler finds every one after a merge.
**Why:** a transformed or scrolled box that drops an item paints in the
wrong place, or not at all.

## Script and web APIs

### Grep the prelude before adding a web API
**Context:** adding a class or global to the JS that every page runs
(`browser/engine/prelude.ore`, `form_prelude.ore`).
**Bad:** the forms branch defined its own `Blob`, `File` and `FormData`
after the prelude. They shadowed the real ones, and `slice()`,
`arrayBuffer()` and `FileReader` stopped working (fixed in `e9fd680`).
**Good:** `rg "class FormData" browser/engine` first, and extend the
existing class (`prelude.ore:2046`, `:2066`, `:2119`). The form prelude now
keeps only `ValidityState` (`form_prelude.ore:7`).
**Why:** a later definition silently wins; the tests of the new branch
pass while the old API's users break.

### A missing feature fails like a browser with the feature off
**Context:** a web API the engine cannot provide yet (WebGL, ...).
**Bad:** an absent method throws `TypeError` and stops the whole script;
a fake object that does nothing fails later, far from the cause.
**Good:** answer as a real browser does when the feature is unavailable.
`getContext()` returns null for every type but `'2d'`
(`browser/engine/canvas_prelude.ore:634-635`; before the canvas merge,
`8d269de` returned null for `'2d'` too):
```
\\P.getContext = function (kind) {
\\  if (String(kind) !== '2d') return null;
```
**Why:** site scripts already handle the "not supported" answer and take
their fallback path (the Mistral blog's three.js hero, `518b0d3`).

### A fast path must answer in the slow path's order
**Context:** caches and inline caches in the JS engine (`browser/js/`).
**Bad:** the property cache took a function's `name` or `length` from its
parent constructor's prototype, but `getFrom` answers a function's own
`name` and `length` before the prototype (fixed in `0f2d643`, after the
JS performance merge `b2193b7`).
**Good:** before filling a cache, mirror every rule of the slow lookup
(`browser/js/shapes.ore:163-167`), and add a JS test of the edge cases
(`browser/tests/js/229_shape_cache_paths.js`).
**Why:** a cache hit skips the slow path's special cases; the benchmark
still passes while a site gets the wrong value.

### "Illegal invocation" may be a JS engine bug: find the `this` first
**Context:** a site logs `TypeError: Illegal invocation` from a native.
**Bad:** P3.22 looked like a brand check of a new web API (Request,
fetch). The native was the input `value` getter, called with `this`
undefined: `Object.defineProperty(el, 'value', {enumerable: false})`
had turned React's own accessor into a data property holding the
getter, so `el.value` was a function and `setState` called it.
**Good:** print what `this` is where the native throws (`thisNode` in
`browser/engine/dom_api.ore`), then reduce the caller to a Node-checked
test (`browser/tests/js/249_generic_descriptors.js`). A descriptor
without value, writable, get and set changes only the flags
(`definePropertyFrom`, `browser/js/builtins.ore`). On a non-configurable
property, check each field as ValidateAndApplyPropertyDescriptor does
(`redefinable`; `250_nonconfigurable_descriptors.js`), not a shortcut:
P3.22's fix still threw for a no-op `{enumerable}` there (P3.27).
**Why:** a native's brand check is often only where a wrong value
surfaces; the bug can be far from it, in plain JS semantics.

### A created document is not `instanceof Document`
**Context:** an API that takes a Document (XHR `send`, serializers).
**Bad:** P3.24's first `send(body)` tested `body instanceof Document`;
`document.implementation.createHTMLDocument()` returns a plain stand-in
object (`createHTMLDocument`, `browser/engine/prelude.ore`), so it was
sent as `"[object Object]"`, text/plain.
**Good:** test `typeof body === 'object' && body.nodeType === 9`, and
read the stand-in's `documentElement`, not its `childNodes` (it has none)
(`XMLHttpRequest.send`, `browser/engine/prelude.ore`).
**Why:** the page has one real document; every other one is a stand-in
with only a few of Document's members.

### A parsed document is another document to the page's scripts
**Context:** a stand-in document (DOMParser, `createHTMLDocument`) that
an SPA router fills from fetched HTML.
**Bad:** P3.30: DOMParser cut `<head>`/`<body>` out with regexes, and
the first `<body` on mistral.ai was in a script's comment; its nodes
said `ownerDocument === document`, so Turbo (GitHub) never called
`importNode`, its `<turbo-frame>` stayed un-upgraded and the frame
counted as missing (`turbo:reload` "missing_frame").
**Good:** parse with the HTML parser (`ak.parseDocument`,
`parseDocumentElement` in `browser/engine/markup.ore`); a parsed node's
`ownerDocument` is the stand-in until it is inserted elsewhere
(`parsedDocs`, `browser/engine/prelude.ore`); test it in
`browser/tests/jsdom/dom_parser.html`.
**Why:** routers check where a node came from before they adopt it; a
stand-in that claims the page's document skips their import path.

### Root native return values across mutation hooks
**Context:** a DOM native creates a JS value, then changes the document.
**Bad:** `styleRemoveProperty` kept its return string in an Ore local
while `changed()` called the prelude's mutation hook; GC stress reused
the string's cell and returned "style" instead of "none".
**Good:** push the return value before a mutation and pop it afterwards
(`browser/engine/dom_api.ore`, `styleRemoveProperty`). Image requests use
the same mutation log as observers, so hooks now run even without a
page-created MutationObserver. The `style_props.html` fixture covers this.
**Why:** Ore locals are not GC roots; a hook can allocate or invoke JS
before the native returns.

### Preserve exact numeric types through shared coercion paths
**Context:** adding a numeric primitive to the JS engine.
**Bad:** lexer `10n` used to discard the suffix and store an f64; routing
operators through `toNumber` would either round BigInts or reject valid
BigInt arithmetic. Accumulating limbs into an f64 also rounds more than
once (`Number` halfway cases in `248_bigint_edges.js`).
**Good:** keep literal digits as atoms, use `numericValue` (ToNumeric) for
operators/updates, and reserve `toNumber` for consumers that must reject
BigInt (Math, unary plus). Explicit `Number` uses a single guard/sticky,
ties-to-even rounding step. Trace the new primitive's payload in
`gc.ore`; run the Node goldens under GC stress.
**Why:** the syntax, coercion, fast paths, equality and GC visitors must
agree on the type. A correct arithmetic helper alone is not a primitive.

### Capture an import's referrer in its syntax, not the active script
**Context:** dynamic imports inside functions, timers and promise callbacks.
**Bad:** resolving against document.currentScript at call time, or treating
`import` as a callable global. Later calls have no current script, and
module callbacks need the module URL rather than the document URL.
**Good:** `Node.ImportCall` holds the referrer URL atom from parsing, and
static imports, `import()` and module scripts share the loader's module
map (`browser/js/lib.ore`). `231_dynamic_import.js` checks delayed calls,
nested imports and failures.
**Why:** a function's source context outlives the script's execution.

### Link a module graph before running any of it
**Context:** ES modules (`browser/js/modules.ore`, the loader in `lib.ore`).
**Bad:** the first loader (`df16130`) ran each module as an async function
that awaited its imports one at a time and built its namespace when it
returned. Cycles never settled, a microtask turn ran between modules, a
missing export was found after the dependencies' side effects, and
hoisted functions were unreachable from the other modules of a cycle.
**Good:** load the whole graph, link it (every import bound to a live cell
before any code runs), then evaluate it with the spec's DFS over strongly
connected components (InnerModuleLinking, InnerModuleEvaluation and the
async module algorithms). Check against Node: `233_module_cycles.js`,
`235_module_tla.js`, `236_module_errors.js`.
**Why:** bundler output has cycles and re-exports everywhere; only the
spec's algorithm runs it in the order its authors tested.

### Module live bindings must bypass ordinary-value fast paths
**Context:** static imported identifiers and namespace properties.
**Bad:** numeric expression recipes returned the internal import cell;
property write caches treated namespace getters as ordinary accessors and
silently ignored strict assignments. A class check on every identifier
read (`df16130`) made all scripts pay for the cells.
**Good:** the resolver marks import bindings `IDENT_IMPORT`: only those
reads unwrap a cell, and `compileFastExprs` skips them. `cacheable`
declines ModuleNamespace objects; the slow path implements namespace
writes, deletes and descriptors (`232_module_await.js`, `236`).
**Why:** an internal cell is not an observable JavaScript value.

### Take back a load's partial state when it fails
**Context:** a cache whose entries mark work as done (the module records'
`deps` in `loadGraph`, `browser/js/lib.ore`).
**Bad:** `loadGraph` gave a module its `deps` before the rest of its graph
loaded. When a deeper fetch failed, the next `import()` of the same root
saw `deps` set, skipped loading, and linking read a null `deps`:
"Cannot read properties of null" instead of the fetch's TypeError.
**Good:** record what the call set and undo it on failure (6488d64):
```
try { loadLevels(root, given); }
catch (e) { for (const m of given) m.deps = null; throw e; }
```
`244_module_cycle_edges.js` imports a failing graph twice.
**Why:** "done" must mean the whole job succeeded; a retry trusts it.

### Keep the caller's environment rooted when switching to the global one
**Context:** a native that makes a closure in the global scope
(`moduleRecord` in `browser/js/modules.ore`).
**Bad:** setting `it.env = NONE` around `makeClosure` let a collection
free the calling JS function's environment; under `ANYKNOWN_JS_GC=stress`
the loader then wrote into a freed environment.
**Good:** push the saved environment on `it.outer_envs` while `it.env` is
`NONE`, as `runScript` does, and run new module fixtures under GC stress.
**Why:** `it.env` is a root only while it names the environment.

### Keep one decoder and version each observable image request
**Context:** image state exposed to JS and images used by layout/canvas.
**Bad:** `decode()` formerly fulfilled for any nonempty src, including
broken images; canvas dimensions synchronously fetched images and
overrode the prelude's getters.
**Good:** `loadElementImage` in `browser/engine/images.ore` shares source
selection and the URL cache. The prelude owns request state, invalidates
old decode waiters, and checks request identity before each event task.
Canvas fixtures await `decode()` before reading dimensions or drawing.
**Why:** a URL cache is not per-element request state. A replaced request
must not publish dimensions or fire a stale load/error event.

### Keep a page's memory of a shared load in the page
**Context:** state of a background load (`browser/net/loader.ore`)
that the page must keep after the load ends.
**Bad:** `loadFailed` asked the load, which goes away with its last
user. Two img requests dropped after their load failed: the next
request fetched the URL again (P4.6, `net/failed_image_once.html`).
**Good:** `releaseImageLoad` (`browser/engine/images.ore:344`) copies a
failure into the page's `ImageCache` (`noteFailed`) before releasing
the load; a new page has a new cache.
**Why:** loads are per session and shared; what a page remembers must
live as long as the page, no longer.

### A load event stops at the document
**Context:** dispatching an event at a node (`dispatch` in
`browser/engine/prelude.ore`), now that each image fires its own load
event when its background load ends.
**Bad:** the event path went from the document on to the window for
every type, so `addEventListener('load', f, true)` on the window ran
`f` for each image as well as for the page.
**Good:** DOM's "get the parent" of a document is null for `load`
(`eventParent` and the path without shadow roots, `56ac713`); the
document's capturing listeners still see the images' loads
(`net/bg_image_events.html`).
**Why:** pages put capturing load listeners on the window to run once;
an image's load reaching them runs page setup again.

### A page's scripts change the history before the tab shows it
**Context:** state that scripts change and the front end keeps (the
session history: `browser/engine/history.ore`, `tab.ore`).
**Bad:** `Tab` kept the entries, and a page joined them only after
`load` returned (`tab.push(target, p)`). SPAs call `replaceState` or
`pushState` while they load, so a second list in the page would have
been needed, and the agent and window back/forward always loaded the
entry again (P3.29).
**Good:** a page owns a `SessionHistory` from `newPage` (one entry,
`Page.history`); the tab moves its entries into its own when it shows
the page (`adoptPushed`, or `adoptAt` for a traversal) and points the
page at it (`Page.shared`); every caller asks `p.sessionHistory()`.
An entry's `doc` (the page id) says whether back stays in the document.
**Why:** scripts run inside `load`; whatever they change must have a
home before the tab has the page, and exactly one home after.

## Code style

### Write blocks and comments like the surrounding code
**Context:** any change in `browser/`, especially from another model or
branch.
**Bad:** the forms branch wrote one-line blocks
(`if text.eql(t, "file") { return "" }`) and new struct fields without
comments; `e64d41c` expanded them and added the comments.
**Good:** one statement per line in a block, and a comment on each new
field like its neighbours (`browser/dom/controls.ore:31`, `:58-61`).
**Why:** `ore fmt` keeps line breaks as written, so style is the
author's job, and the field comments are the only place ownership
("owned", "null for none") is written down.

## Tests

### Bless goldens only after reading the diff
**Context:** a golden test fails (`tests/browser.rs`: dump, layout,
agent and screenshot fixtures).
**Bad:** re-blessing everything after a merge hides regressions along
with the intended changes.
**Good:** run `cargo test --test browser`, read each difference, then
`ANYKNOWN_BLESS=1 cargo test --test browser` (`tests/browser.rs:14`) and
check that `git diff` shows only the expected lines. `7081aa9` is an
example: two forms goldens changed only in the lines of `pre` text, and
the commit says so.
**Why:** a golden is only a test while someone reads what changed.

### Pixel goldens use the fixed font
**Context:** screenshot and layout tests.
**Bad:** text drawn with the system's fonts differs between machines, so
a pixel or layout golden of it fails on another machine.
**Good:** goldens use `--font fixed` (`tests/browser.rs:355`). A test of
`--font system` checks the PNG's size and that it differs from the fixed
font's, and skips without a system font (`tests/browser.rs:436`).
**Why:** the fixed-metric test font is part of the repo; system fonts
are not.

### Let the test server wait for the event, not for a time
**Context:** a test that checks what is in flight at the server (loads
in the background, concurrency) while the browser runs.
**Bad:** `net/bg_images.html` asked `/slow-now`, which counted the
`/slow/` images 0.1 s after it arrived; each image was answered 0.3 s
after its own arrival. When the browser stalled more than 0.2 s between
starting the images and sending the request (load 40+, GC stress), all
had been answered: "in flight: none" (P3.12; reproduced with the
browser at `taskpolicy -c background` under 40 `yes` processes).
**Good:** the images use `/held/`, which answers only after a
`/held-now` request counted it; `/held-now` waits until one is in flight
(`browser/tests/server.py`, `HELD_CHANGED`). The waits stop after
`HOLD_LIMIT` s, which only a broken browser reaches.
**Why:** a sleep only says "probably happened by now"; a loaded machine
breaks every such window, and the next run passes.

### Let the page release what the window test waits for
**Context:** a `--frames N` window test whose order depends on when an
image arrives (`window_relayout_after_image_load`, P3.38).
**Bad:** the image came from `/slow/` (0.3 s after the request) and the
window drew 60 frames of about 10 ms; a first frame later than the load
left no "below visible: true" before the load.
**Good:** `net/window_relayout.html` takes the image from `/held/` and
asks `/held-now` from the observer's first report, so the load is after
that report on any machine; the frame budget (400) only has to be large.
**Why:** the page's own state, not a clock, orders the two events.

## Debugging a real site

### See console output with `dump --js`, not the headless window
**Context:** a page's scripts fail and you need their messages.
**Bad:** the window with `--frames N` (or `--exit-when-idle`) turns the
console echo off for tests (`browser/ui/window.ore:183-184`), so errors
do not appear.
**Good:** `anyknown dump --text --js URL` runs the scripts and timers and
writes console messages, alerts and uncaught exceptions to stderr
(`browser/main.ore:253-256`). For minified scripts, set
`ANYKNOWN_JS_TRACE=1`: each error message then shows the start of the
source of the running functions (`browser/js/gc.ore:22`).
**Why:** the headless window is quiet so that test output stays
deterministic.

### Reproduce window input with an event script
**Context:** a bug that needs clicks, scrolling or keys.
**Good:** script the input instead of clicking by hand:
```
anyknown URL --frames 30 --events "click:120,40 wheel:-5 key:cmd+f" --state --save out.png
```
The words are documented in `browser/ui/script.ore:1-18`; `--state`
prints the tab's state at the end, `--save` writes the last frame, and
`--text` prints the document's text.
**Why:** a scripted run can be repeated after each change and turned
into a test in `tests/browser.rs`.

### Trace a canceled click from the page's own listener lists
**Context:** an agent `click` or `submit` says "a script canceled" it
and the URL does not change (an SPA router).
**Good:** in one `anyknown agent` session on the replay: `eval` a script
that wraps each `l.fn` of `node.__listeners` on the target, its
ancestors, `document` and `window` (log the call, and `defaultPrevented`
after it) and wraps `fetch` and `History.prototype.pushState`; then
`click`, then `eval` the log. Listen for the router's own events
(`turbo:*`, `astro:*`), and keep the log in `localStorage` when the page
may load a new document. P3.30 found the exception after the router's
fetch on GitHub and mistral.ai this way.
**Why:** the router cancels the click first and fails later in a promise;
only the order of its steps shows where.

### Vary the headers before blaming the URL of a 4xx
**Context:** an archived XHR or fetch answered 4xx and the page shows its
error text.
**Bad:** the GitHub note blamed the URL for the 400s on `/latest-commit`
and `/tree-commit-info` (no branch). The route `/:owner/:repo` has no
branch, so Chrome asks the same URL (`commitish:e.branch` in
`code-view-*.js`). The archive was captured before P3.17, when fetch
dropped the page's `Accept: application/json`; curl without it gets 400,
with it 200.
**Good:** find the URL's inputs in the bundle, then curl the URL live
with and without the page's headers; recapture with the current binary
before changing the engine (P3.26).
**Why:** a server's 4xx can answer a missing header as well as a wrong
path, and an old archive keeps an old binary's requests.

### Build in safe mode to find a crash
**Context:** the browser crashes on a real site.
**Good:** `browser/build.sh --mode safe` keeps bounds and overflow
checks at about the speed of `fast`, so a panic names the line
(`browser/build.sh:10-14`). The default `fast` build has no checks.
**Why:** a fast build turns a bounds error into silent memory damage
far from its cause.

## Performance

### No interpreter and no arena allocation per call on hot paths
**Context:** code that style, layout or paint calls for every element or
every frame.
**Bad:** both came with the forms branch (`d2161f6`, `a95e259`) and
are still open at `b548a82`:
- `formPattern` creates a whole JS interpreter for each `pattern` check
  (`browser/engine/form_patterns.ore:13`), and `:valid`/`:invalid`
  matching runs it for each element on each style pass
  (`browser/css/selector.ore:1422`).
- `dom.valueOf` builds the value of color and range inputs in the
  document's string arena (`browser/dom/controls.ore:377`, `:394`), and
  paint calls it every frame (`browser/paint/form_widgets.ore:11`). The
  arena frees nothing until the document goes.
**Good:** compile once and cache by input (the pattern text), or keep a
result in the control's state. In paint, use a scratch allocator that is
reset per frame, or no allocation.
**Why:** both costs grow with the page and with time; an arena that
grows every frame is a leak.

### Measure end to end before claiming a speedup
**Context:** performance work on the engine or on JS.
**Bad:** the JS work measured a 1.95x geometric mean on its benchmarks,
but `anyknown dump --md --js https://react.dev/` was not faster
(`browser/js/PERF.md:5-6`, `:53-61`).
**Good:** report the micro benchmarks and a real page load, with
interleaved runs of the baseline and the change, as `browser/js/PERF.md`
does. Use offline request archives so the network does not decide the
result: `browser/tools/page_load.py` and `browser/engine/PERF.md`
(merged in `44f5483`).
**Why:** a page load spends its time in network, parsing, style and
layout too; a faster interpreter alone may not change it.

### A forced layout must not cost a full cascade
**Context:** an API that makes scripts wait for styles or layout
(geometry, getComputedStyle, observers).
**Bad:** `0c810b7` styled the whole document again for the first
geometry query after any DOM change. YouTube's Polymer code writes and
reads in turns: 137 forced layouts of 50-130 ms each made the watch
replay 13.6 s instead of 4.2 s, scripts hit their time limit, and the
related list stayed empty.
**Good:** measure the count and cost per call first (`ANYKNOWN_TIMING=1`
phases `script_layout`, `cascade`, `restyle`), then keep the forced
layout's styles and style again only what the change can reach
(`browser/engine/geometry.ore` `restyleRoots`,
`browser/css/invalidation.ore`), with a fixture that compares each kind
of change with the full layout (`browser/tests/jsdom/layout_restyle.html`).
Ignore changes to trees out of the document and attributes set to their
value. Make the cascade itself cheap where it matters
(`browser/css/rule_index.ore`, `browser/css/ancestor_filter.ore`).
**Why:** frameworks interleave writes and reads by design; each read is
correct to force a layout, so its cost has to follow the size of the
change, not of the document.

### Keep styles by a fingerprint of their inputs, not by a version
**Context:** skipping a restyle (or other cached work) when nothing it
depends on changed.
**Bad:** `Page.restyle` styled the whole document on every call, since
`domVersion` misses changes no script makes: the agent sets
`aria-checked` and a `details`' `open` itself
(`browser/agent/actions.ore:324`, `:337`), and focus and control values
live in `doc.controls`. GitHub's replay styled its document 7 times.
**Good:** `css.styleInputs` (`browser/css/inputs.ore`) hashes all that
the cascade reads besides the sheets; `computeStyles` keeps it in
`Styles.inputs`, and `stylesCurrent` (`browser/engine/engine.ore`)
compares. Of `class` it takes only what selectors can see, so a class
no rule names costs no cascade. A fixture that checks reuse logs into a
JS array and writes the page once at the end
(`browser/tests/jsdom/layout_unseen.html`): its own log would change the
page between reads.
**Why:** a fingerprint sees every writer, also the ones nobody counted.

### Replay must not change what a page sees
**Context:** offline replays (`ANYKNOWN_REPLAY`) for performance work.
**Bad:** the archive answered `data:` URLs too and missed them; once img
elements reported errors, YouTube re-set its placeholder on every error
and the replay ran 30,000 image requests (`99432a0`).
**Good:** when a replay is much slower than expected, compare the phase
counts (`timers n=`) with a previous binary and look for archive misses
(`archive miss:` on stderr) before optimizing the engine.
**Why:** a replay artifact looks like an engine regression and hides the
real numbers.

### Record a second run's extra requests on top of the first run's replay
**Context:** an archive must serve more than one command (the dump and
the screenshot).
**Bad:** `capture` recorded only the dump; the screenshot replayed BBC
with 38 misses (Reith fonts, story images), so the page painted in a
fallback serif with empty image frames (P3.40).
**Good:** capture's screenshot runs with both `ANYKNOWN_REPLAY` and
`ANYKNOWN_RECORD` on the same directory (`screen(record=True)`,
`browser/tools/page_load.py`): it is served the dump's responses and
fetches and records only its misses (`archiveRead`,
`browser/net/archive.ore`).
**Why:** a second live run gets a newer page (new story images), so it
would record images the archived HTML does not name.

### On the virtual clock, count a task's budget in steps, not time
**Context:** anything that decides what a page sees in `dump` or the
agent: task budgets, timers, scrolling, observers.
**Bad:** a task's budget was 2 s of processor time on every clock. On
the `youtube-search` replay, nine ShadyCSS and data-binding tasks ran
out of it after 3 to 15 million steps, depending on the machine's load
(`clock()` also counts the loader threads); each run rendered other
results and asked for other thumbnails, and `check` passed 3 runs in 7
(P1.7). Each run's set of requested URLs differed.
**Good:** on the virtual clock, tasks and microtask checkpoints count
steps alone (`enter`, `checkpointOver`, `browser/engine/script.ore`);
the window keeps the time budget. To find such an input, log the set
of requested URLs per run (`ANYKNOWN_TIMING=1` `requests` lines) with
and without `yes > /dev/null` load, and diff the sets.
**Why:** the same page and virtual clock must give the same requests;
an archive records one run's requests, and a replay that asks for others
misses.

### On the virtual clock, let a task's steps move time
**Context:** code that reads the clock inside one task: schedulers with a
time budget, `while (Date.now() < end)` loops, idle callbacks.
**Bad:** the virtual clock moved only between tasks, and the benchmark
pinned `Date.now` to one instant; YouTube's scheduler (`do { job }
while (queue && Date.now() < start + budget)`) never yielded, and one
task drained the whole queue: 3 to 15 million steps, 42% of a
`youtube-search` dump's CPU (P1.8).
**Good:** `tick` (`browser/engine/script.ore`) adds 1 ms per
`STEPS_PER_MS` (1,000,000) steps the page ran, in whole ms, so a short
task still sees the clock stand still; the benchmark's `Date` is its
epoch plus `ak.now()` (`browser/engine/benchmark.ore`). Fixtures:
`jsdom/virtual_clock.html`, `benchmark/date_now.html`. Too slow a pace
is a regression too: at 100,000 steps per ms YouTube's work used up
`dump`'s 30 s of page time before its results rendered (27 videos of
35). Replay heavy sites (`youtube-search`, zol, react) when you change it.
**Why:** a clock that never moves inside a task turns every time budget
into "run everything"; counting steps keeps it deterministic.

### A request the browser starts sending is missing from old archives
**Context:** a change that lets pages make requests they could not
before (fetch methods, new APIs), checked with `page_load.py check`.
**Bad:** after P3.17 let fetch POST, the replays of mdn, python-docs,
react and youtube failed on `archive miss:` for their POST beacons and
APIs (`/pong/get`, `/api/event`, `g/collect`, `youtubei/v1/guide`): the
archives were captured by a browser that rejected them in JS. The
markdown was the same as before; the page sees a failed fetch either way.
**Good:** list the new misses in the report and propose a recapture
step; do not recapture or loosen the miss check in the feature branch.
Archive names hash the URL, the method (but GET) and the body
(`archivePath`, `browser/net/archive.ore`).
**Why:** an archive records what one binary asked for; a binary that asks
for more needs a new capture, not a replay that hides the gap.

### Replay a step-timed beacon by URL, not by body
**Context:** a replay's non-GET requests (beacons, logging APIs) whose
bodies or count follow the virtual clock, which follows the step count.
**Bad:** a no-op engine change made `youtube-search` send one more
`POST /youtubei/v1/log_event?alt=json`, with a body the archive lacked:
`archive miss:` and `check` failed, though the page saw the same thing.
**Good:** a non-GET request without an exact record gets the latest
non-GET record of its URL and logs `archive fallback:`, which
`page_load.py check` prints as `fallback:`, not a miss (`fallback`,
`browser/net/archive.ore`; test `archive_replays_unrecorded_post_bodies`,
`tests/browser/perf.rs`). A GET still matches exactly (P1.12).
**Why:** a beacon's answer does not depend on its body; requiring the
exact body makes every engine change look like a regression.

### An agent task's replay can miss beacons a recapture cannot fix
**Context:** `page_load.py check --agent` replays a site's agent task
(`browser/tools/agent_tasks/`) on an archive that `capture` recorded live.
**Bad:** ZOL's task reached the news page in both runs, yet the replay
missed `stat.zol.com.cn/adrs/iparea.php?r=0.236...` and two other
beacons: their URLs hold `Math.random()` values, and live, scripts run
in the order the network delivers them, so the seeded sequence differs.
A second capture would only record other values.
**Good:** `agent_task` reports misses on a `task:` line and judges only
the steps' `expect`; `check` (the page load itself) still fails on misses.
**Why:** a check that fails on every capture gets ignored; the steps say
whether the agent got where it was going.

### Script-visible network requests must not be serial
**Context:** an API that lets scripts observe a resource's state (img
`complete`/`naturalWidth`/`load`, fetch, XHR) on the synchronous fetch.
**Bad:** `c40a1ea` fetched each img's image in its own microtask, one at
a time inside the scripts phase: lobste.rs's 25 avatars took 25 round
trips (8 s instead of 2 s). A replay hides it: every request is 0 ms.
**Good:** start a background load (`net.startLoad`,
`browser/net/loader.ore`) and deliver the result in a task once it
ended (`ak.imageTask`, `a018c5c`; `8ae98cd` batched a turn's requests
into one `net.prefetch` before that). Check a live run's `requests`
phase for serial runs of the same host as well as the replay. The test
server's `/slow/` route counts concurrent requests
(`http_images_concurrent`) and `/held-now` the `/held/` ones in flight
(`http_images_background`).
**Why:** a real browser loads in parallel and never blocks on it; a
blocking fetch costs one round trip per request.

### One event-loop turn runs only the tasks queued before it
**Context:** the window's frame loop and timers on the real clock.
**Bad:** `runDueTimers` ran timers until none was due; four canvas
animation loops in a mistral.ai iframe, each forcing a layout (real
geometry since `0c810b7`), took longer than their 16 ms interval, so
the loop never ended and `--frames 30` hung (`f126cc6`).
**Good:** a turn stops at the timers scheduled before it began; the
window draws between turns (`window_timers_yield_to_frames`). When a
window run hangs, sample it: a `timers` count far above the `paint`
count with `ANYKNOWN_TIMING=1` is this starvation.
**Why:** a page may always have a due task; frames must still come.
