# Real-site checklist

The B8 list of public pages (`docs/browser.md`, B8), loaded with the
browser and checked by hand. Run of **2026-10-06**, at `d1c8800`
(all B8 slices merged), on macOS, built with `browser/build.sh` (`--mode
fast`). The network was in Taiwan, so Google answered in Traditional
Chinese.

## How each site was checked

- **Agent view:** `anyknown dump --md --js URL` (90 s limit). The load
  time below is this command's wall time: fetch, subresources, scripts
  and the event loop.
- **JS errors:** an `anyknown agent` session, `navigate` to the URL;
  the count is the `[error]` entries in its `console`.
- **Screen:** the window, headless (`SDL_VIDEODRIVER=dummy anyknown URL
  --width 1024 --height 768 --frames 30 --save OUT.png`), scripts on,
  system fonts. Every PNG was looked at.
- **Agent actions** (where noted): a `click` or a `type` with `submit`
  in an agent session.

Status: ✅ readable on the screen and in the agent view, small flaws
only; ⚠️ usable, but one view or part of the page is broken; ❌ the
page's content is not available.

## Sites

| Site | URL | Status | Load | JS errors | Notes |
|---|---|---|---|---|---|
| example.com | https://example.com/ | ✅ | 0.4 s | 0 | Text and layout right. The script-added translations: the Chinese line is boxes (no CJK font), Arabic is drawn left to right with unjoined letters (no bidi, no shaping). |
| Wikipedia (en) | https://en.wikipedia.org/wiki/Web_browser | ✅ | 2.9 s | 0 | Article, infobox image and headings render well. SVG logo and icons are empty boxes; the wordmark is cut off; "145 languages" overlaps an icon box. Markdown starts with 12 empty list items (hidden menu entries). Agent search on Special:Search works. |
| Hacker News | https://news.ycombinator.com/ | ✅ | 1.9 s | 0 | Looks like the real page. SVG logo is an empty box, vote arrows missing. In Markdown the whole story list is one table cell. Agent `click` on "new" works. |
| GitHub repository | https://github.com/rust-lang/rust | ⚠️ | 8.4 s | 0 | Header, tabs, file list and About panel render; the README is in the Markdown. SVG icons are empty boxes; the "last commit" columns stay empty (skeleton bars). Slowest site. Agent `click` on a link works. |
| MDN | https://developer.mozilla.org/en-US/docs/Web/HTML/Element/details | ⚠️ | 1.3 s | 0 | Markdown is clean. On screen the header is empty (inline SVG logo, nav missing), breadcrumb items run together, and the sidebar nav is squeezed into a narrow column that overlaps the article text. |
| Python docs | https://docs.python.org/3/tutorial/index.html | ✅ | 3.1 s | 1 | Renders like a desktop browser. `select.add is not a function` (theme switcher). Agent `click` to the next chapter works. |
| rustdoc (std) | https://doc.rust-lang.org/std/vec/struct.Vec.html | ✅ | 2.1 s | 2 | Sidebar, code highlighting and prose render well. SVG logo is a box. `HTMLLinkElement is not defined`, a TDZ error (`settingsDataset`): no search bar or settings. |
| BBC News | https://www.bbc.com/news | ⚠️ | 3.9 s | 5 | Nav bars and headlines readable, Markdown has the stories. Logo missing, lead images missing (grey placeholders), the right column runs past the viewport, headline web font falls back to serif. Errors include `de is not a function` (also on react.dev) and a piano SDK error. |
| CNN Lite | https://lite.cnn.com/ | ✅ | 0.6 s | 1 | Plain list of stories, renders right. `f is not a function`. |
| old Reddit | https://old.reddit.com/r/programming/ | ❌ | 3.7 s | 0 | The site redirects logged-out clients to `/login/?reason=lor2` (curl gets the same 302), and that login page renders blank. |
| Stack Overflow | https://stackoverflow.com/questions/11227809/... | ❌ | 0.4 s | 0 | Cloudflare challenge ("Just a moment...", "Enable JavaScript and cookies to continue"); it does not pass. curl with a Chrome User-Agent gets 403 too. |
| Lobsters | https://lobste.rs/ | ✅ | 1.9 s | 0 | Renders like the real page; a few symbol glyphs are boxes. |
| Go docs | https://go.dev/doc/effective_go | ⚠️ | 1.7 s | 3 | Article readable, Markdown complete. The off-canvas nav drawer (`position: fixed` + `transform: translateX(100%)`) is drawn open over the right half; Material icon names show as text ("arrow_drop_down"); the skip link is visible. jQuery fails (`S.fn.init is not a constructor`), `:scope` selector rejected by `querySelectorAll`. (Replaces www.gnu.org, which timed out for curl as well.) |
| DuckDuckGo HTML | https://html.duckduckgo.com/html/?q=ore+programming+language | ✅ | 4.4 s | 0 | Results readable; some icons are boxes. Agent `type` + submit (a POST form) works. |
| Google | https://www.google.com/ | ⚠️ | 0.9 s | 3 | Logo and box render, but the zh-TW text is all boxes (no CJK font) and the search box is not centred. `_._DumpException is not a function` (3x). |
| Wikipedia (ja) | https://ja.wikipedia.org/wiki/ウェブブラウザ | ⚠️ | 6.6 s | 0 | UTF-8 decoded right: the Markdown is full Japanese text. On screen every Japanese character is a box (no CJK font), so a person cannot read it. Same SVG and empty-list-item issues as en. |
| 阿部寛 (Shift_JIS) | http://abehiroshi.la.coocan.jp/ | ❌ | 0.2 s | 0 | Blank page: `<frameset>` is not supported (the frames end up nested in `body`, nothing shown). Shift_JIS is read as windows-1252: the title and the frame page (`top.htm`) are mojibake. |
| ZOL (GBK) | https://www.zol.com.cn/ | ❌ | 5.3 s | 4 | GBK read as windows-1252: all Chinese text is mojibake, on screen and in Markdown. Images and the layout grid render. jQuery fails (`S.fn.init is not a constructor`) and three follow-on errors. |
| YouTube | https://www.youtube.com/ | ❌ | 6.0 s | 5 | Only the grey loading skeleton; Markdown is empty. The app does not boot: `document.createElementNS is not a function`, `DocumentFragment is not defined`, `initCustomEvent is not a function`, no `performance` navigation timing (`responseStart`). |
| react.dev | https://react.dev/ | ⚠️ | 1.9 s | 2 | Renders well, including code panes and the live previews. SVG logo missing, the search box's key hint overlaps its label. In Markdown the code blocks lose their line breaks (each block is one line). `de is not a function` (2x). |

**Totals:** 8 ✅, 7 ⚠️, 5 ❌ (20 sites). No crash and no timeout in
the browser; every run ended in under 9 s.

## Top failure causes

Ranked by the number of sites they hurt; each maps to the remaining
work in `docs/browser.md` ("What remains" under the status table) or is
marked as not yet listed there.

1. **Missing DOM and web APIs, so site scripts throw** (9 sites: Python
   docs, rustdoc, BBC, CNN Lite, go.dev, Google, ZOL, YouTube,
   react.dev). Seen: `createElementNS`, `DocumentFragment`,
   `HTMLLinkElement` and other interface globals, `initCustomEvent`,
   `HTMLSelectElement.add`, the `:scope` selector, `performance`
   timing; jQuery 3 does not initialise (`S.fn.init is not a
   constructor`, 2 sites); minified bundles fail with `de`/`f is not a
   function` (not diagnosed). Partly the *B6 leftovers* (modules,
   `getComputedStyle`, live NodeLists); most of the missing APIs are not
   listed yet. YouTube is the case where this makes the page empty.
2. **SVG is not painted** (8 sites: Wikipedia en and ja, Hacker News,
   GitHub, MDN, rustdoc, BBC, react.dev): `<svg>` and `.svg` images are
   sized boxes, so logos and icons are empty. Not listed yet (B8 sizes
   replaced boxes only).
3. **No font fallback for CJK and symbols, no bidi or shaping** (5
   sites: example.com, Google, Wikipedia ja, Lobsters, DuckDuckGo):
   characters the sans/serif/mono faces lack are boxes; Arabic is drawn
   unjoined left to right. Web fonts and icon fonts are not loaded
   either (BBC, go.dev). Not listed yet.
4. **Layout of current responsive CSS** (4 sites: MDN, BBC, go.dev,
   Google): no CSS `transform` (go.dev's drawer is drawn open), a
   sidebar overlapping the article (MDN), a column past the viewport
   (BBC), a centred form that is not (Google). Partly the *B8 `fixed`
   and `sticky`* item; `transform` is not listed yet.
5. **Agent Markdown flaws** (4 sites: Wikipedia en and ja, Hacker News,
   react.dev): empty `- ` items for hidden menu entries, a layout table
   kept as one cell, `<pre>` code blocks joined into one line. Not
   listed yet (B7 view).

Smaller causes: legacy charsets (2 sites: Shift_JIS, GBK; the *B8 "no
Shift_JIS, GBK or Big5"* item), bot walls (2 sites: Reddit's logged-out
redirect, Cloudflare's challenge; both refuse curl too, so not purely
the browser), `<frameset>` (1 site; not listed yet).
