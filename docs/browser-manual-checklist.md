# anyknown window: manual checklist

The window's behavior is tested headlessly (`tests/browser.rs`:
`window_frames` and `window_events`, which feed scripted input through
the same event handler with SDL's dummy driver). What needs a real
display and a person is below: run it once per milestone, on macOS and
on Linux when possible, and note the date, platform and any failures at
the end.

Build: `ore build browser -o anyknown` (SDL2, FreeType and libcurl
installed). Fixtures are under `browser/tests/`.

## Opening

- [ ] `./anyknown browser/tests/article.html` opens a window titled
      "Notes on Ore - anyknown", with the toolbar and the page.
- [ ] `./anyknown open browser/tests/article.html` does the same.
- [ ] `./anyknown https://example.com/` loads over HTTPS.
- [ ] `./anyknown browser/tests/missing.html` shows "Cannot load this
      page" with the reason, not a crash.
- [ ] The window does not use CPU while idle (Activity Monitor / `top`:
      near 0%).

## HiDPI

- [ ] On a Retina (or 200% scaled) display, text and borders are sharp,
      not blurry or pixel-doubled.
- [ ] Moving the window between a HiDPI and a normal display keeps it
      sharp and laid out the same (in CSS px).

## Text

- [ ] Body text, headings, bold, italic and monospace (`pre`, `code`)
      use proportional system fonts and look like a normal browser's.
- [ ] Text sits on its line (no clipped descenders, no overlaps); links
      are blue and underlined.

## Scrolling

- [ ] Mouse wheel and trackpad scroll the page smoothly in both
      directions (`browser/tests/ui/long.html`).
- [ ] Down/Up arrows scroll a line; PageDown/PageUp and Space/Shift+Space
      a screen; Home/End go to the top and bottom.
- [ ] Scrolling stops at the top and at the end of the page.

## Links and the status line

- [ ] Hovering a link changes the cursor to a hand and shows the link's
      full URL at the bottom left; leaving it restores the arrow and
      clears the status line.
- [ ] Hovering the address bar shows the I-beam cursor.
- [ ] Clicking a link loads it; relative links resolve against the page.
- [ ] A link to `#id` on the same page (long.html: "To the end") scrolls
      to the element without reloading.

## History

- [ ] The back button and Cmd+[ go back; the forward button and Cmd+] go
      forward; the buttons are grayed out when there is nowhere to go.
- [ ] Going back restores the scroll position the page was left at.
- [ ] The reload button and Cmd+R reload the page (edit the file between
      reloads to see it change) and keep the scroll position.

## Address bar

- [ ] Cmd+L focuses the address bar and selects the whole URL (blue
      ring, selection highlight); typing replaces it.
- [ ] Clicking in the address bar places the caret there; Left/Right,
      Home/End, Backspace and Delete edit as expected, also with non-ASCII
      text (type "café").
- [ ] Enter navigates: a URL, a file path, or a host name like
      `example.com` (becomes https://example.com/).
- [ ] Escape restores the current URL and gives focus back to the page.
- [ ] A long URL scrolls inside the field to keep the caret visible, and
      shows its start when the field loses focus.

## Find in page

- [ ] Cmd+F opens the find bar at the bottom; typing highlights every
      match (yellow), the current one in orange, with "N of M".
- [ ] Enter goes to the next match, Shift+Enter to the previous one,
      scrolling each into view; "No matches" when there are none.
- [ ] Escape closes the bar and removes the highlights.

## Window

- [ ] Resizing the window lays the page out again for the new width
      (text rewraps; `browser/tests/style/media.html` switches media
      rules at its breakpoint).
- [ ] Cmd+Q and the close button quit; the process exits with status 0
      and, in a debug build, prints no leak reports.

## Runs

| Date | Platform | Result |
| --- | --- | --- |
| | | |
