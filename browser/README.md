# anyknown

A web browser written in Ore, for people and AI agents at once. The
design, the milestones and their state are in `docs/browser.md`; the
agent protocol is in `docs/agent-protocol.md`.

## Build and run

From the repository root, with SDL2, FreeType and libcurl installed
(`pkg-config` must find them); HarfBuzz (text shaping: Arabic, Indic
scripts, icon font ligatures) and libbrotlidec (WOFF 2 web fonts) are
used when they are installed. The first build also needs libclang, for the
`@cimport` of `pthread.h` in `js/js.ore` (macOS has it with the Command
Line Tools; later builds read the declarations from `~/.cache/ore`):

```
cargo build --release
target/release/ore build browser -o anyknown
./anyknown https://example.com/                 # a window
./anyknown dump --md https://example.com/       # the page as Markdown
./anyknown dump --a11y https://example.com/     # the accessibility tree
./anyknown agent                                # JSON lines on stdio
./anyknown mcp                                  # an MCP server on stdio
```

`cargo test --test browser` runs the browser's tests (headless, offline).

Before coding, read [`docs/practices/`](../docs/practices/README.md)
(`browser.md` for this directory); when you learn a lesson that will come
up again, add an entry there.

## Use with Claude Code / MCP clients

Two ways, depending on whether a person should see the page.

**Without a window.** `anyknown mcp` is an MCP server on stdio: the
agent gets its own invisible browser.

```
claude mcp add anyknown -- /path/to/anyknown mcp
```

**In your window.** Open a window that listens for agents, then let the
client connect to it through `anyknown connect`, a bridge between stdio
and the window's socket:

```
/path/to/anyknown --agent-socket /tmp/ak.sock https://example.com/ &
claude mcp add anyknown -- /path/to/anyknown connect /tmp/ak.sock --mcp
```

The agent then works in the tab you see (or in a background tab it
opens with `browser_tab_new`), and you see what it does:

- its target element is outlined with its ref (`e12`) for a moment, an
  `AI` badge shows in the toolbar while it is connected, and the status
  bar tells its last action;
- **Pause** in the toolbar, or Cmd+Shift+P, stops it: its actions fail
  with "paused by user" until you resume (it can still look);
- Cmd+Shift+K points it at the element under the mouse (or the focused
  one) and your selection: it gets a `user_pointed` notification, and
  `browser_user_context` tells it what you select, hover, focus and
  pointed at;
- Cmd+T, Cmd+W and Cmd+1..9 open, close and show tabs.

Any MCP client that starts stdio servers works the same way: the command
is `anyknown` with the arguments `connect /tmp/ak.sock --mcp`. Without
`--mcp`, the connection speaks the JSON lines of `anyknown agent`.
Socket paths are limited to about 100 bytes.
