# Browser practices

Conventions and pitfalls of the browser code in `browser/`, learned from
merges, reviews and real sites. Line numbers are as of `0f2d643`.

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
**Good:** append the row at the end of `PROPERTIES` (`css.ore:214`, last
row `css.ore:320`), add its constant after the last one (`css.ore:203`),
and raise `NPROPS` (`css.ore:106`) and the array length `[105]Property`
together. At a merge, renumber one side's block and check that each
constant still names its row. Elsewhere use the constant or
`css.propertyIndex(name)`, never a number.
**Why:** the index is the identity of a property in every computed style;
a stale number reads another property's value without any error.

### Host object tags are one shared numbering
**Context:** adding a kind of host object to the JS engine
(`browser/engine/script.ore:51`).
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
(`script.ore:54-64`). After a merge, check that no two tags are equal.
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
**Context:** the alias table `logicalAlias` in `browser/css/values.ore:117`.
**Bad:** the layout branch (`451e86a`) aliased `mask` and `-webkit-mask`
to `mask-image`. When the backgrounds branch parsed `mask` as a real
shorthand (`values.ore:359`), the alias sent it to `mask-image` first, so
the merge `8b670fb` removed those pairs.
**Good:** aliases map one longhand name to another (logical to physical).
A shorthand gets its own expansion (`values.ore:247`).
**Why:** an alias runs before shorthand parsing, so it hides the
shorthand from the code that knows how to split it.

### Collect a property's resources from `::before` and `::after` too
**Context:** a property that loads something (images of
`background-image`, `mask-image`).
**Bad:** `styleUrls` fetched only the elements' own images, so a
background on a `::before` box never loaded (fixed in `13a070c`).
**Good:** also read the pseudo-element styles, as
`browser/engine/images.ore:327` does:
```
for which in 1..3 {
    if styles.pseudo(h, which as u8) |pv| {
        try css.layerUrls(alloc, pv[css.BACKGROUND_IMAGE], out)
```
**Why:** pseudo-elements have their own computed styles, outside the
element's values; sites use them for icons and decorations.

## Paint

### A new display item kind needs an arm in `transformed()` and `moved()`
**Context:** adding a variant to `Item` (`browser/paint/paint.ore:68`).
**Bad:** the SVG branch added `Item.Svg`, the layout branch added
`transformed()` without it, and the layout merge `7b975d5` did not compile
until `4091dd2`. The backgrounds merge `8b670fb` had to add `Tiles`,
`Shadow`, `Layer`, `Mask` and `EndLayer` to both functions.
**Good:** when you add a variant, handle it in `transformed()`
(`paint.ore:846`), `moved()` (`paint.ore:931`) and `render()`
(`paint.ore:985`). These matches have no `_` arm, so the
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
existing class (`prelude.ore:2039`, `:2059`, `:2112`). The form prelude now
keeps only `ValidityState` (`form_prelude.ore:7`).
**Why:** a later definition silently wins; the tests of the new branch
pass while the old API's users break.

### A missing feature fails like a browser with the feature off
**Context:** a web API the engine cannot provide yet (canvas drawing,
WebGL, ...).
**Bad:** an absent method throws `TypeError` and stops the whole script;
a fake object that does nothing fails later, far from the cause.
**Good:** answer as a real browser does when the feature is unavailable.
`getContext()` returns null for every type
(`browser/engine/prelude.ore:828-830`, added in `8d269de`):
```
\\// canvas: no drawing yet, so no context (as for an unknown type).
\\const CanvasP = HTMLCanvasElement.prototype;
\\hidden(CanvasP, 'getContext', function () { return null; });
```
**Why:** site scripts already handle the "not supported" answer and take
their fallback path.

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
**Good:** goldens use `--font fixed` (`tests/browser.rs:352`). A test of
`--font system` checks the PNG's size and that it differs from the fixed
font's, and skips without a system font (`tests/browser.rs:433`).
**Why:** the fixed-metric test font is part of the repo; system fonts
are not.

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

### Build in safe mode to find a crash
**Context:** the browser crashes on a real site.
**Good:** `browser/build.sh --mode safe` keeps bounds and overflow
checks at about the speed of `fast`, so a panic names the line
(`browser/build.sh:6-9`). The default `fast` build has no checks.
**Why:** a fast build turns a bounds error into silent memory damage
far from its cause.

## Performance

### No interpreter and no arena allocation per call on hot paths
**Context:** code that style, layout or paint calls for every element or
every frame.
**Bad:** both came with the forms branch (`d2161f6`, `a95e259`) and
are still open at `0f2d643`:
- `formPattern` creates a whole JS interpreter for each `pattern` check
  (`browser/engine/form_patterns.ore:13`), and `:valid`/`:invalid`
  matching runs it for each element on each style pass
  (`browser/css/selector.ore:962`).
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
result; that tooling (`browser/tools/page_load.py`,
`browser/engine/PERF.md`) is on the unmerged branch `pi-loadperf`
(`13617f8`) and not merged yet.
**Why:** a page load spends its time in network, parsing, style and
layout too; a faster interpreter alone may not change it.
