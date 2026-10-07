# Canvas 2D tests

The implementation is in `engine/canvas_api.ore` and
`engine/canvas_prelude.ore`. Bitmap ownership is in the page's image
cache. SVG supplies path parsing, curve flattening, stroke polygons,
antialiased coverage and geometric hit tests; paint supplies image
sampling, blur and PNG encoding; font and TextPainter supply shaping,
fallback and glyph coverage. There is no separate canvas rasterizer.

Paths reach the native side as segments (or path data) and become SVG
pictures directly; stroke attributes are parsed as SVG markup once per
distinct value. A draw renders, paints and composites only its area:
the bounds of its shape or text mask, grown by the stroke, the shadow
blur and a two-pixel margin, within the canvas and the clip paths'
bounds (copy, source-in, source-out, destination-in and
destination-atop use the whole clip). Transparent pixels are stored as
0,0,0,0, also those written by putImageData. `canvasbench/run.sh`
measures drawing calls.

## Offline coverage

- `jsdom/canvas_api.html` runs `api.js`: 62 explicit assertions on context
  identity, resets, validated state, compositing pixels, transforms, dash
  state, Path2D, winding rules and hit tests, clipping, ImageData, text
  metrics, image/canvas/ImageBitmap sources, patterns, gradients, PNG
  data URLs and asynchronous PNG blobs.
- `jsdom/canvas_damaged.html` runs `damaged.js`: 119 operations with
  non-finite and huge arguments, malformed SVG paths, negative radii and
  rectangles, zero-sized and oversized canvases, and getters that resize
  the bitmap during argument conversion. Both fixtures also run
  under the browser harness's GC-stress mode.
- `paint/canvas_*.html`: five inspected PNG goldens for shapes,
  gradients and transforms; joins, caps and dashes; text; scaled/cropped
  images and patterns; nested/even-odd clipping and shadows. The text
  golden deliberately uses the browser's fixed-metric box glyphs.
- `canvas_origin_clean` and `canvas/origin.html`: local HTTP checks of
  same-origin reads, cross-origin and redirected image taint, propagation
  through canvas/ImageBitmap, refused exports and dimension resets.
- `agent/canvas.html`: accessibility names and Markdown fallback or
  bitmap-size descriptions.
- `canvas_animation` in `tests/browser.rs`: three requestAnimationFrame
  draws compared against an immediate drawing, after decoding the PNG
  and excluding the window's address bar.

Run `cargo test --test browser`. Only bless intentional changes, and
inspect every changed PNG.

## Limits and approximations

Bitmaps are limited to 16384 pixels per side and 16,777,216 pixels total
(64 MiB), with larger canvases retaining their attribute dimensions but
having no drawable backing store. Pixel reads are transparent there;
PNG serialization returns `data:,`. Width/height writes reset pixels,
paths and drawing state, including writes of the existing value.

The Porter-Duff composite operations (source-over/in/out/atop,
destination-over/in/out/atop, copy, xor), lighter and multiply are
supported. Other blend modes are ignored by the property setter. Shadow blur uses the paint module's box
blur. Shadows are separate draws, so their interaction with non-default
compositing is not fully conformant, and image shadows use the destination
rectangle rather than the source's alpha. strokeText uses a dilated/eroded
glyph mask, not font outlines. Hanging/ideographic baselines approximate
font ascent/descent. Text has system-font shaping and fallback but does
not yet share layout's loaded web-font faces. Nonuniformly transformed
strokes use SVG's area-scale approximation.

OffscreenCanvas is a detached canvas sharing the same implementation,
not a worker/transferable object. createImageBitmap snapshots an image or
canvas; crop/resize options and Blob/ImageData inputs are not implemented.
Cross-origin image draws (including redirected images) taint the canvas
and propagate through canvas/ImageBitmap sources. Pixel reads and PNG
exports then throw SecurityError until a dimension reset. Image CORS
validation is not implemented, so a crossorigin attribute cannot grant
read access to an otherwise cross-origin image.

## Scratch integration check

Chart.js 4.4.8 (downloaded outside the repo, not a committed dependency)
was exercised with a nonresponsive, nonanimated bar chart and a curved
line chart. A 60-frame headless window screenshot showed four colored
bars, line points, axes, grid lines, labels and legends. A system-font
smoke screenshot also checked ordinary readable canvas text.
