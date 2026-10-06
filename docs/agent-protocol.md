# anyknown agent protocol

The anyknown browser gives AI agents the engine without pixels: the page
as an accessibility tree with refs, the main content as Markdown, and
actions on refs. Two stdio servers carry it:

- `anyknown agent [--width PX] [--height PX] [--no-js]`: JSON lines
  (this document, first part).
- `anyknown mcp [--width PX] [--height PX] [--no-js]`: a Model Context
  Protocol server (second part).

A window can take agents too: `anyknown --agent-socket PATH [URL]`
serves both protocols on a Unix socket, on the pages a person sees and
uses at the same time, and `anyknown connect PATH [--mcp]` bridges stdio
to that socket for MCP clients ([A shared window](#a-shared-window)).

Both keep one session: the page shown, the history, the refs. The
viewport is 800x600 px unless given. Text is measured with the browser's
fixed-metric font, so boxes are approximate but stable. Forms work
(B5): the agent and the window share the state of the controls
(values, checkedness, selected options, the focus), and submitting a
form loads the response.

Pages run their scripts (B6; not with `--no-js`). Actions fire the
events a user's would, before their default action: `click` fires
`click` (a canceled one does nothing more), `type` and `select` fire
`input` and `change`, `press` fires `keydown` at the focused element
(else the body), typing fires `input`, a submission fires `submit`. A
navigation that a script asks for (`location.href = ...`, a link it
clicks, `history.back()`) happens after the action and is added to its
`result`. Time is the page's own: after an action only the timers that
are due run (a `setTimeout(f, 0)`); `wait` moves the clock forward,
taking no real time. Console messages (`[log] ...`, uncaught
exceptions as `[error] Uncaught ...`, alerts as `[alert] ...`) come
with the next response, as a `console` array (JSON lines) or a
`Console:` section (MCP).

For goldens and quick looks, the same views are on the command line:

```
anyknown dump --a11y [--boxes] [--json] [--js] [--width PX] FILE|URL
anyknown dump --md [--refs] [--js] [--width PX] FILE|URL
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
  or the first word of its `role` attribute. `text` is a run of text;
  preformatted text (`pre`, `white-space: pre-line` ...) keeps its line
  breaks: each line is a `text` node of its own (empty lines are left
  out). Elements without a role (`div`, `span`, ...) are left out and their
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
  `disabled`, `required`, `readonly`, `focused`, `invalid`.
- **value**: of text fields (passwords as `*`), the selected option of a
  combobox, sliders, spin buttons, color swatches, progress bars and meters.
  Numeric widgets also expose `min` and `max` when defined (sliders default
  to 0 and 100); the JSON tree carries these as strings.
- **href**: the `href` of a link as written.
- `[box=x,y,w,h]`: with `--boxes` / `"boxes":true`, the element's border
  box in document px (inline elements: the union of their fragments).
- `[offscreen]`: an element with a ref outside the viewport (after
  scrolling).
- `: text`: a node whose only content is text.

Left out: `display: none`, `visibility: hidden` (but visible children
stay), `aria-hidden=true`, `input type=hidden`, `script`, `style`,
`template`, `noscript`, media and `svg` without a label. The text of a
`label` that names a control is not repeated. A layout table (one with
`role=presentation`, a table inside, a single column, or no caption,
header cell or border) is not a `table`: its rows and cells are left
out and their content joins the parent.

The document of a frame (of a `<frameset>`, or an `iframe`) is a
`frame` node, named by the iframe's `title`, else the document's title,
and followed by its address, with the document's content under it:

```
- frame "Menu" -> http://example.com/menu.htm
  - link "Top" [e1] -> top.htm
- frame "Home" -> http://example.com/top.htm
  - heading "Welcome" [level=1] [e2]
```

The elements in a frame have refs and boxes (in the coordinates of the
page) like the page's own, and actions on them work in the frame's
document: its scripts get the events, a form submits from there. A link
(or form) with a `target` loads in the frame of that name, `_parent` in
the parent's frame, `_top` and `_blank` in the whole tab; without a
target, a link in a frame loads in that frame. A document loaded in a
frame keeps the page (the response has a `diff`, not a new `snapshot`),
and the refs of the frame's old document no longer work. `press` acts
in the frame of the element the agent focused last; `find` also
searches the frames; `scroll` to an element in a frame scrolls the frame
too.

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
fenced `pre` blocks (their line breaks kept, also those of `<br>` and
of blocks inside; the `language-x` of the `pre` or of a `code` inside),
`>` quotes, GFM tables (the first row is the header; `|` escaped),
images `![alt](absolute URL)` and `---` rules. A layout table (see the
snapshot) is read row by row: the cells of a row that are one paragraph
each make one paragraph, joined by spaces. List items with nothing
visible in them are left out. Whitespace collapses as in HTML; `<br>` is
a hard break (`\` at the end of the line). Form controls are left out.
The document of each frame (of a frameset, or an iframe) comes after a
heading `## Frame: URL`.

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
| `navigate` | `url`: absolute, relative to the current page, a file path, or a host name (`example.com` becomes `https://example.com`) | `result`, `url`, `title`, `snapshot` |
| `back`, `forward`, `reload` | | as `navigate` |
| `snapshot` | `boxes` (bool), `format`: `"text"` (default) or `"json"` | `url`, `title`, `snapshot` (text) or `tree` (JSON) |
| `read` | `refs` (bool, default true) | `url`, `title`, `markdown` |
| `click` | `ref` | `result`, `url`, `title`, `diff` or `snapshot` |
| `type` | `ref`, `text`, `submit` (bool: press Enter after) | as `click` |
| `select` | `ref` (a select), `option` (label or value; exact, then ignoring case) | as `click` |
| `press` | `key` | as `click` |
| `scroll` | `ref`, or `dy` (px, negative is up) | as `click` |
| `find` | `text` (ignoring ASCII case) | `result`: the matches |
| `wait` | `selector` (CSS; at most `ms`, 5000 by default), or `ms` alone, or neither (until no timer is left, at most 5 s of page time) | `result` ("found ... after N ms", or "waited N ms"); an error after the time without a match |
| `eval` | `expression`: a script; its last expression statement is the result | `result`: a string as it is, anything else as `console.log` shows it; an uncaught exception is the error |
| `tabs`, `tab_new`, `tab_select`, `tab_close`, `user_context` | | only in a window ([A shared window](#a-shared-window)) |
| `screenshot` | `ref` (the element's border box), or `full` (bool: the whole document, at most 16384 px tall); neither: the viewport as scrolled | `result` (what it shows and its size), `url`, `title`, `width`, `height`, `png` (the PNG file in base64) |

Scripts in a session (`todo.jsonl` in `browser/tests/agent/sessions/`):

```
{"id":2,"cmd":"type","ref":"e2","text":"Buy milk"}
{"id":3,"cmd":"click","ref":"e3"}
{"id":3,"ok":true,"result":"clicked button \"Add\" [e3]; a script canceled the submission",...,"diff":"...+   - listitem\n+     - checkbox \"Buy milk\" [e4]\n..."}
{"id":6,"cmd":"wait","selector":"#saved:not(.hidden)"}
{"id":6,"ok":true,"result":"found status after 300 ms",...}
{"id":9,"cmd":"eval","expression":"document.querySelectorAll('li').length"}
{"id":9,"ok":true,"result":"2",...}
```

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
  group); a submit button submits its form (GET or POST, as the form
  says) and the response is loaded in a new history entry; a reset
  button resets its form; ARIA checkboxes toggle `aria-checked`;
  `summary` opens or closes its `details`; a `label` clicks its
  control; an `option` is selected. Other buttons and elements record a
  `click on` event. The clicked element takes the focus when it can.
- **type**: replaces the value of a text field and focuses it (the
  attributes keep the defaults that a reset goes back to); with
  `submit`, then submits its form as Enter would. Not for disabled or
  read-only fields. Number and calendar inputs accept text too; date,
  time, datetime-local, month and week values are checked against their
  HTML formats when validating. File inputs show "No file chosen"; agents
  cannot upload files (neither `type` nor `eval` can select a local file).
  They submit an empty file part in multipart forms.
- **select**: selects one option; in a single select the others are
  unselected.
- **press**: on a focused text field a character is typed at the caret
  (at the end after `type`), Backspace and Delete delete one,
  ArrowLeft, ArrowRight, Home and End move the caret, Enter submits the
  form with its first submit button (a new line in a `textarea`);
  ArrowUp/ArrowDown step a number input, and all four arrow keys step a
  focused range slider, respecting min/max/step. Enter
  on a link or button and Space on a button, checkbox or radio button
  click it; Tab and
  Shift+Tab move the focus through focusable elements; Escape leaves
  the field; PageDown, PageUp, Space, ArrowDown, ArrowUp, Home and End
  scroll. Every key is recorded.
- **scroll**: to put an element at the top of the viewport, or by `dy`;
  clamped to the document. The result says which part of the document
  the viewport shows.
- **find**: at most 20 matches, each with 30 characters of context, the
  nearest element around it that has a ref, and `[offscreen]`.
- **screenshot**: paints the page as the window would, with the system's
  fonts (a shared window's own); the session's layout, refs and boxes
  do not change. Without system fonts, text shows as boxes.

Form submissions validate before firing `submit`, unless the form has
`novalidate` or the submitter has `formnovalidate`. An invalid submission
stays on the page, focuses the first invalid control whose `invalid` event
was not canceled, and reports `submission blocked: MESSAGE` in the action
response. Snapshots expose invalid state even before submitting. Scripts
can use the live `validity` state, `checkValidity()`, `reportValidity()`,
`setCustomValidity()`, `requestSubmit()`, `reset()` and `new FormData(form)`.
Patterns use the browser's Unicode (`u`) RegExp engine; Unicode `v`
character-set operations are not implemented. Calendar inputs have no
picker popup, color inputs are swatches, and datalist suggestions are
ignored.

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
  JSON-RPC error -32602. `browser_screenshot` puts an image block
  (`{"type": "image", "data": BASE64, "mimeType": "image/png"}`) before
  the text.
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
| `browser_wait` | `selector`, `ms` | as `wait` |
| `browser_eval` | `expression` | the value, as `eval` |
| `browser_screenshot` | `ref`, `full` | the image, then what it shows, `URL:`, `Title:` |

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

## A shared window

`anyknown --agent-socket PATH [FILE|URL]` opens the window and listens
on a Unix domain socket at `PATH` (a stale socket file there is
replaced; the file is removed when the window closes). Agents connect
to it while a person uses the window: they see the same pages and act
on them, and the person sees what they do.

- Each connection speaks JSON lines, as `anyknown agent`, unless its
  first line is the hello `{"mcp":true}` (no response) or a JSON-RPC
  message: then it speaks MCP, as `anyknown mcp`. At most 8 agents
  connect at once.
- The window serves requests between its own events, on its thread: a
  request never runs while the page handles the person's input.
- Commands act on the tab the person sees, unless the connection opened
  or selected another (`tab_new`, `tab_select`); then they act there,
  in the background, without changing the person's view.
- The agent sees the window's layout: its fonts and its viewport (the
  window's size under the toolbar). After the person navigates, the
  next request gives the new document new refs.
- `wait` takes real time here, while the person goes on using the
  window (without a selector or `ms`: until no timer is left, at most
  5 s). The response comes when the wait is over; the connection's
  later requests wait for it.

What the person sees and does:

- **Activity.** Before an action, the target element is outlined in
  purple with its ref (e.g. `e12`), fading out over about a second; an
  `AI` badge sits in the toolbar while an agent is connected; the
  status bar tells the last action (`agent: click button "Sign in"
  [e3]`).
- **Pause.** The toolbar's Pause button or Cmd+Shift+P (Ctrl+Shift+P)
  pauses the agents: actions (`navigate`, `back`, `forward`, `reload`,
  `click`, `type`, `select`, `press`, `scroll`, `eval` and the tab
  commands) fail with `"paused by user"`; looking (`snapshot`, `read`,
  `find`, `tabs`, `user_context`) still works. The same key resumes.
- **Pointing.** Cmd+Shift+K points the agents at the element under the
  mouse (else the focused one) and the selected text: every connection
  gets a notification, and `user_context` keeps it as `pointed`.
- **Focus.** `snapshot` adds `"focused":"e7"` (MCP: a `Focused:` line)
  when an element of the page has the focus: the person's focus and the
  agent's are the same.
- **Tabs.** Cmd+T opens a tab, Cmd+W closes the shown one (the last one
  closes the window), Cmd+1..8 show a tab and Cmd+9 the last one; with
  two tabs or more, a tab strip under the toolbar shows them (a click
  shows a tab, its `x` closes it). At most 16 tabs.

The notification, as a JSON line and as MCP (`notifications/message`,
whose `data` is the same object; `initialize` then also lists the
`logging` capability, and `logging/setLevel` returns `{}`):

```
{"event":"user_pointed","tab":1,"url":"file:///.../form.html","ref":"e1","text":"Sign up","element":"heading \"Sign up\" [e1]"}
{"jsonrpc":"2.0","method":"notifications/message","params":{"level":"notice","logger":"anyknown","data":{"event":"user_pointed",...}}}
```

The commands only a window has (MCP: `browser_` and the name):

| `cmd` | arguments | response fields |
| --- | --- | --- |
| `tabs` | | `result` ("2 tabs"), `tabs`: `[{"id":1,"url":..,"title":..,"shown":true,"agent":false},...]` (`shown`: the person sees it; `agent`: this connection acts on it) |
| `tab_new` | `show` (bool: also show it to the person) | `result`, `tab`: its id; the connection acts on it from now on |
| `tab_select` | `tab` (an id), `show` (bool) | `result`, `tab` |
| `tab_close` | `tab` (default: the one the connection acts on) | `result`; the last tab cannot be closed; connections on it go back to the shown tab |
| `user_context` | | `tab`, `url`, `title` and `scroll` (px) of the shown tab; `selection` (the selected text of the focused text field, else the find bar's query while it has matches; never a password); `hovered` and `focused`: `{"ref","element","text"}` or `null`; `pointed`: the last `user_pointed` event's members, or `null` |

Tab ids are numbers that are never reused. The argument is `tab`, not
`id`, which is the request's own. A session (from
`browser/tests/agent/window/shared.jsonl`; the window ran the scripted
events `agents:5 move:100,90 key:cmd+shift+k`):

```
{"id":5,"cmd":"snapshot"}
{"id":5,"ok":true,...,"snapshot":"...","focused":"e4"}
{"event":"user_pointed","tab":1,"url":"file:///.../form.html","ref":"e1","text":"Sign up","element":"heading \"Sign up\" [e1]"}
{"id":6,"cmd":"user_context"}
{"id":6,"ok":true,"result":"the user's context","tab":1,"url":"file:///.../form.html","title":"Sign up","scroll":0,"selection":"","hovered":{"ref":"e1","element":"heading \"Sign up\" [e1]","text":"Sign up"},"focused":{"ref":"e4","element":"checkbox \"Send me news\" [e4]","text":""},"pointed":{...}}
{"id":8,"cmd":"tab_new"}
{"id":8,"ok":true,"result":"opened tab 2","tab":2}
{"id":9,"cmd":"navigate","url":"browser/tests/agent/site/index.html"}
```

### `anyknown connect`

MCP clients start a server as a command and talk to its stdio.
`anyknown connect PATH [--mcp]` is that command for a window: it copies
stdin to the socket and the socket to stdout until either ends (when
stdin ends, the window answers what it was sent, then closes). With
`--mcp` it sends the hello first. With Claude Code:

```
anyknown --agent-socket /tmp/ak.sock https://example.com/ &
claude mcp add anyknown -- anyknown connect /tmp/ak.sock --mcp
```

The tools are those of `anyknown mcp` plus `browser_tabs`,
`browser_tab_new`, `browser_tab_select`, `browser_tab_close` and
`browser_user_context`, and the instructions tell the model that a
person shares the window.

### Headless (tests)

`--exit-when-idle` closes the window once the last agent has left, and
lets `--events`, `--state`, `--text` and `--save` work without
`--frames`. In `--events`, the word `agents:N` makes the events after it
wait until the agents made N requests (the MCP hello does not count);
`--state` then adds the tabs, whether the agents are paused, and the
status bar's activity line. `cargo test --test browser window_agents`
runs `browser/tests/agent/window/X.jsonl` with the events of
`X.events`, over the socket from the test or through `anyknown connect
--mcp` for `X.mcp.jsonl`, against `X.out`.
