# anyknown agent protocol

The anyknown browser gives AI agents the engine without pixels: the page
as an accessibility tree with refs, the main content as Markdown, and
actions on refs. Two stdio servers carry it:

- `anyknown agent [--width PX] [--height PX]`: JSON lines (this document,
  first part).
- `anyknown mcp [--width PX] [--height PX]`: a Model Context Protocol
  server (second part).

Both keep one session: the page shown, the history, the refs. The
viewport is 800x600 px unless given. Text is measured with the browser's
fixed-metric font, so boxes are approximate but stable. JavaScript does
not run yet (B6), and forms are not submitted yet (B5): clicks on buttons
and submissions are recorded as events and said so in the result.

For goldens and quick looks, the same views are on the command line:

```
anyknown dump --a11y [--boxes] [--json] [--width PX] FILE|URL
anyknown dump --md [--refs] [--width PX] FILE|URL
```

The code is in `browser/agent/`; the goldens in `browser/tests/agent/`.

## The snapshot

An accessibility tree, one node per line, children indented by two
spaces:

```
- navigation "Primary"
  - link "Docs" [e2] -> /docs
- main
  - heading "Welcome" [level=1] [e3]
  - paragraph: Some text.
  - paragraph
    - text: Start with the
    - link "installation guide" [e5] -> docs.html#install
  - textbox "Email" [e6] [required] [focused] value="me@example.com"
  - checkbox "Remember me" [e7] [checked]
  - combobox "Color" [e8] value="Green"
    - option "Red"
    - option "Green" [selected]
  - button "Below the fold" [e9] [offscreen]
```

A line is `- role "name" [level=N] [eN] [states] value="..." -> href
[box=x,y,w,h] [offscreen]: text`, each part only when it applies:

- **role**: from the element (`a href` link, `button`, `input` textbox,
  searchbox, spinbutton, slider, checkbox, radio, `select` combobox or
  listbox, `h1`-`h6` heading, `ul`/`ol` list, `li` listitem, `img`,
  `table`/`tr`/`td`/`th` table, row, cell, columnheader, rowheader, `nav`
  navigation, `main`, `header` banner, `footer` contentinfo, `aside`
  complementary, `form`, `section` with a name region, `article`, `p`
  paragraph, `blockquote`, `hr` separator, `details` group, `summary`
  button, `fieldset` group, `dialog`, `figure`, `progress` progressbar),
  or the first word of its `role` attribute. `text` is a run of text.
  Elements without a role (`div`, `span`, ...) are left out and their
  content joins their parent; `role=none` and `role=presentation` do the
  same. An element without a role that takes clicks or the focus
  (`onclick`, `tabindex`) is `generic`.
- **name**: `aria-labelledby`, `aria-label`, the label of a form control
  (`label for`, a wrapping `label`), `alt`, a table's `caption`, a
  fieldset's `legend`, a figure's `figcaption`, the value of an `input`
  button; for links, buttons, headings, cells and options their text;
  then `title`, then `placeholder`. Names longer than 150 bytes are cut
  with `…`.
- **ref** `[eN]`: interactive elements (links, buttons, text fields,
  checkboxes, radio buttons, selects, ARIA widgets, focusable
  generics) and headings. A ref names one node of one document: it stays
  the same across snapshots of that document, and refs are never reused
  in a session, so a ref from an earlier page is rejected ("unknown
  ref") instead of hitting another element. Refs are given in document
  order when a page loads.
- **states**: `checked`, `mixed`, `selected`, `expanded`, `collapsed`,
  `disabled`, `required`, `readonly`, `focused`.
- **value**: of text fields (passwords as `*`), the selected option of a
  combobox, sliders, spin buttons and progress bars.
- **href**: the `href` of a link as written.
- `[box=x,y,w,h]`: with `--boxes` / `"boxes":true`, the element's border
  box in document px (inline elements: the union of their fragments).
- `[offscreen]`: an element with a ref outside the viewport (after
  scrolling).
- `: text`: a node whose only content is text.

Left out: `display: none`, `visibility: hidden` (but visible children
stay), `aria-hidden=true`, `input type=hidden`, `script`, `style`,
`template`, `noscript`, media and `svg` without a label. The text of a
`label` that names a control is not repeated.

The JSON form (`--json`, `"format":"json"`) is an array of nodes
`{"role", "name", "ref", "level", "states": [...], "value", "url",
"box": {"x","y","w","h"}, "offscreen", "text", "children": [...]}`,
without the fields that do not apply.

## Read

`read` gives the main content as Markdown: the `main` element (or
`role=main`), else the first `article`, else the whole body; so
navigation, headers, footers and sidebars are dropped when the page says
where its content is. Headings `#`, paragraphs, lists (nested, with the
`start` of `ol`), links `[text](absolute URL)` followed by ` [eN]` when
refs are asked for, `*em*`, `**strong**`, `~~del~~`, `` `code` ``,
fenced `pre` blocks (with the `language-x` of a `code` inside), `>`
quotes, GFM tables (the first row is the header; `|` escaped), images
`![alt](absolute URL)` and `---` rules. Whitespace collapses as in HTML;
`<br>` is a hard break (`\` at the end of the line). Form controls are
left out.

## JSON lines

One request per line on stdin, one response per line on stdout, in
order. Empty lines are skipped. A request is an object with `"cmd"` and
an optional `"id"` (any JSON value), echoed in the response (`null` when
missing):

```
{"id":1,"cmd":"navigate","url":"https://example.com/"}
{"id":1,"ok":true,"result":"loaded https://example.com/","url":"https://example.com/","title":"Example Domain","snapshot":"- heading \"Example Domain\" [level=1] [e1]\n..."}
```

A failure is `{"id":..,"ok":false,"error":"why"}`: unknown refs, missing
arguments, pages that cannot be fetched, invalid JSON (`"id":null`). The
session goes on after an error.

| `cmd` | arguments | response fields |
| --- | --- | --- |
| `navigate` | `url`: absolute, relative to the current page, or a file path | `result`, `url`, `title`, `snapshot` |
| `back`, `forward`, `reload` | | as `navigate` |
| `snapshot` | `boxes` (bool), `format`: `"text"` (default) or `"json"` | `url`, `title`, `snapshot` (text) or `tree` (JSON) |
| `read` | `refs` (bool, default true) | `url`, `title`, `markdown` |
| `click` | `ref` | `result`, `url`, `title`, `diff` or `snapshot` |
| `type` | `ref`, `text`, `submit` (bool: press Enter after) | as `click` |
| `select` | `ref` (a select), `option` (label or value; exact, then ignoring case) | as `click` |
| `press` | `key` | as `click` |
| `scroll` | `ref`, or `dy` (px, negative is up) | as `click` |
| `find` | `text` (ignoring ASCII case) | `result`: the matches |
| `wait` | | `result` (returns at once: no scripts yet) |
| `eval`, `screenshot`, `tabs` | | not supported yet |

After an action, a response has `snapshot` when a new document is shown
(a link followed), else `diff` when the snapshot changed: the changed
lines, removed ones as `- old line` and added ones as `+ new line`:

```
{"id":5,"cmd":"click","ref":"e4"}
{"id":5,"ok":true,"result":"checked checkbox \"Send me news\" [e4]","url":"file:///.../form.html","title":"Sign up","diff":"-       - checkbox \"Send me news\" [e4]\n+       - checkbox \"Send me news\" [e4] [checked] [focused]\n"}
```

What the actions do:

- **click**: a link (or an element inside one) is followed; a link to
  a fragment of the same document scrolls to it and adds a history
  entry. Checkboxes toggle and radio buttons check (unchecking their
  group) through the `checked` attribute; ARIA checkboxes toggle
  `aria-checked`; `summary` opens or closes its `details`; a `label`
  clicks its control; an `option` is selected. Buttons, submit buttons
  and other elements record a `click on` or `submit of` event. The
  clicked element takes the focus when it can.
- **type**: replaces the value of a text field (the `value` attribute
  of an `input`, the text of a `textarea`) and focuses it. Not for
  disabled or read-only fields.
- **select**: selects one option (`selected` attribute); in a single
  select the others are unselected.
- **press**: on a focused text field a character is appended, Backspace
  deletes one, Enter submits (recorded; a new line in a `textarea`);
  Enter on a link or button and Space on a button, checkbox or radio
  button click it; Tab and
  Shift+Tab move the focus through focusable elements; Escape leaves
  the field; PageDown, PageUp, Space, ArrowDown, ArrowUp, Home and End
  scroll. Every key is recorded.
- **scroll**: to put an element at the top of the viewport, or by `dy`;
  clamped to the document. The result says which part of the document
  the viewport shows.
- **find**: at most 20 matches, each with 30 characters of context, the
  nearest element around it that has a ref, and `[offscreen]`.

## MCP

`anyknown mcp` is an MCP server over stdio: JSON-RPC 2.0 messages, one
per line. It supports protocol versions `2025-11-25`, `2025-06-18`,
`2025-03-26` and `2024-11-05` (the client's when it is one of them, else
`2025-06-18`), and the `tools` capability:

- `initialize` returns `protocolVersion`, `capabilities: {"tools":
  {"listChanged": false}}`, `serverInfo` (`anyknown`) and
  `instructions`.
- `notifications/initialized` and other notifications get no response.
- `ping` returns `{}`.
- `tools/list` returns the tools with JSON Schemas (`inputSchema`,
  `additionalProperties: false`) and a `readOnlyHint` annotation.
- `tools/call` returns `{"content": [{"type": "text", "text": ...}],
  "isError": bool}`. A tool that fails (no page, unknown ref, missing
  argument) is a result with `isError: true`; an unknown tool is the
  JSON-RPC error -32602.
- Other methods are -32601; invalid JSON is -32700; a message that is
  not an object (batches included) is -32600.

| tool | arguments | the text of the result |
| --- | --- | --- |
| `browser_navigate` | `url` | what happened, `URL:`, `Title:`, the snapshot |
| `browser_back`, `browser_forward`, `browser_reload` | | as `browser_navigate` |
| `browser_snapshot` | `boxes` | `URL:`, `Title:`, the snapshot |
| `browser_read` | `refs` | `URL:`, `Title:`, the Markdown |
| `browser_click` | `ref` | what happened, then the changed lines of the snapshot, or the new page as for `browser_navigate` |
| `browser_type` | `ref`, `text`, `submit` | as `browser_click` |
| `browser_select` | `ref`, `option` | as `browser_click` |
| `browser_press` | `key` | as `browser_click` |
| `browser_scroll` | `ref` or `dy` | as `browser_click` |
| `browser_find` | `text` | the matches |
| `browser_wait` | | that there is nothing to wait for |

A session:

```
→ {"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"c","version":"1"}}}
← {"jsonrpc":"2.0","id":0,"result":{"protocolVersion":"2025-06-18","capabilities":{"tools":{"listChanged":false}},"serverInfo":{"name":"anyknown","title":"anyknown browser","version":"0.1.0"},"instructions":"..."}}
→ {"jsonrpc":"2.0","method":"notifications/initialized"}
→ {"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"browser_navigate","arguments":{"url":"https://example.com/"}}}
← {"jsonrpc":"2.0","id":1,"result":{"content":[{"type":"text","text":"loaded https://example.com/\n\nURL: https://example.com/\nTitle: Example Domain\n\n- heading \"Example Domain\" [level=1] [e1]\n..."}],"isError":false}}
```

To use it from an MCP client, configure a stdio server whose command is
the `anyknown` executable with the argument `mcp`.
