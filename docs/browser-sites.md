# Real-site checklist

The B8 list of public pages (`docs/browser.md`, B8), loaded with the
browser and checked by hand. Run of **2026-10-07**, at `e86268c`
(B7 screenshot and B8 iframes merged), on macOS, built with `browser/build.sh` (`--mode
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
| example.com | https://example.com/ | ✅ | 0.3 s | 0 | Text and layout right. The script-added translations now render: Chinese is drawn, Arabic is joined and right to left. |
| Wikipedia (en) | https://en.wikipedia.org/wiki/Web_browser | ✅ | 2.8 s | 0 | Looks like the real page: logo and icons (SVG) are drawn, article, infobox image and headings right. One figure further down is an empty white frame. Markdown is clean (no empty list items). Agent search on Special:Search works (typed, submitted, results page loaded). |
| Hacker News | https://news.ycombinator.com/ | ✅ | 1.3 s | 0 | Looks like the real page, logo drawn; the vote arrows are tiny. Markdown is a numbered list of stories. Agent `click` on "new" works (`/newest` loaded). |
| GitHub repository | https://github.com/rust-lang/rust | ⚠️ | 5.2 s | 0 | Header, tabs, icons, file list and About panel render. The "last commit" message and date columns and the commit bar stay as skeleton bars (the data does not load), and the Markdown has empty "Latest commit" and "History" sections. Agent `click` on "Contributing" works (it scrolls to `#`; the link is an in-page toggle). |
| MDN | https://developer.mozilla.org/en-US/docs/Web/HTML/Element/details | ✅ | 0.7 s | 0 | Header with logo, breadcrumb, "In this article" sidebar and the article lay out correctly. A blank 90 px band above the header; browser-support icons partly missing. |
| Python docs | https://docs.python.org/3/tutorial/index.html | ✅ | 0.9 s | 0 | Renders like a desktop browser, theme and version selectors work. Agent `click` to the next chapter works. |
| rustdoc (std) | https://doc.rust-lang.org/std/vec/struct.Vec.html | ✅ | 1.3 s | 0 | Sidebar, code highlighting and prose render well; the logo is a black blob. No search bar or settings button in the top bar. |
| BBC News | https://www.bbc.com/news | ⚠️ | 2.6 s | 0 | Markdown has the stories. On screen the page looks unstyled: the logo and the "NEWS" banner are huge SVG letters (each about 290 px, the page is 1000+ px before the first story), the menus are bullet lists and plain buttons, headlines are serif links. Story images do load (the first screenful has none). |
| CNN Lite | https://lite.cnn.com/ | ✅ | 0.6 s | 0 | Plain list of stories, renders right. |
| old Reddit | https://old.reddit.com/r/programming/ | ❌ | 4.1 s | 0 | The site redirects logged-out clients to `/login/?reason=lor2` (curl gets the same 302). The login page now draws its header and chat-bubble background but no login form; the Markdown is only a skip link. |
| Stack Overflow | https://stackoverflow.com/questions/11227809/... | ❌ | 0.4 s | 0 | Cloudflare challenge (HTTP 403, "Just a moment..."): blank page, empty Markdown. curl with a Chrome User-Agent gets 403 too. |
| Lobsters | https://lobste.rs/ | ✅ | 1.9 s | 0 | Renders like the real page, symbols and avatars included. |
| Go docs | https://go.dev/doc/effective_go | ✅ | 2.5 s | 0 | Header, breadcrumb, two-column table of contents right; the nav drawer is closed. Flaws: the breadcrumb separator is a grey box (icon font), the cookie bar covers the bottom. Markdown complete. |
| DuckDuckGo HTML | https://html.duckduckgo.com/html/?q=ore+programming+language | ✅ | 2.1 s | 0 | Results readable; a few favicons are empty boxes. Agent `type` + submit (a POST form) works. |
| Google | https://www.google.com/ | ✅ | 0.8 s | 0 | Logo, centred search box, buttons and the zh-TW text (CJK glyphs now drawn) all right. The two buttons touch. |
| Wikipedia (ja) | https://ja.wikipedia.org/wiki/ウェブブラウザ | ✅ | 5.3 s | 0 | Japanese text readable on screen and in Markdown. The browser-share pie chart (SVG) is drawn as oversized blue and orange blocks. |
| 阿部寛 (Shift_JIS) | http://abehiroshi.la.coocan.jp/ | ✅ | 0.8 s | 0 | Frameset works: menu frame and top frame laid out side by side, Shift_JIS decoded, photo and text right. The "最新情報" banner wraps in a narrow cell. Markdown has a section per frame. |
| ZOL (GBK) | https://www.zol.com.cn/ | ⚠️ | 16-20 s | 0 | GBK decoded: Chinese text, images, logos and the product grid render. The right edge is cut off (search box, nav, a column past 1024 px). Slowest site by far (16.3 s and 20.2 s in two runs). |
| YouTube | https://www.youtube.com/ | ❌ | 5.0 s | 0 | Only the grey loading skeleton; Markdown is empty. No script error is logged any more, but the app still does not render its feed. |
| react.dev | https://react.dev/ | ✅ | 3.6 s | 0 | Renders well: logo, search box, hero, buttons. Markdown code blocks keep their line breaks. |

**Totals:** 14 ✅, 3 ⚠️, 3 ❌ (20 sites). Previous run (2026-10-06,
`d1c8800`): 8 ✅, 7 ⚠️, 5 ❌. No crash and no timeout; the slowest run
was 20 s (ZOL), the others under 6 s. No site logs a JS error.

Changes: ⚠️ to ✅ (MDN, go.dev, Google, Wikipedia ja, react.dev); ❌ to
✅ (阿部寛); ❌ to ⚠️ (ZOL). All other sites keep their status (GitHub and
BBC stay ⚠️; Reddit, Stack Overflow and YouTube stay ❌).

## Top failure causes

Ranked by the number of sites hurt and how much of the page is lost.

1. **Bot walls and login redirects** (2 sites: Stack Overflow,
   Reddit): Cloudflare answers 403 with a challenge page; old Reddit
   redirects to a login page with no form. Both refuse curl too, so this
   is not purely the browser.
2. **A page that does not boot, with no error** (1 site: YouTube, and
   the missing commit data on GitHub): the app shows only its skeleton;
   the console has no `[error]`, so the cause (a request that never
   completes, a feature check, a missing API that is caught) is not
   diagnosed.
3. **CSS not applied or applied wrong on large sites** (BBC, ZOL,
   Wikipedia ja pie chart): BBC looks unstyled, with SVG wordmarks drawn
   at 290 px and menus as bullet lists; ZOL's content runs past the
   right edge; an SVG chart is drawn as oversized blocks. Probably
   `width`/`height` of inline SVG and overflow or sizing rules.
4. **Slow loads** (ZOL 16-20 s, GitHub 5.2 s, Wikipedia ja 5.3 s,
   YouTube 5.0 s): subresources and scripts of heavy pages.
5. **Small drawing gaps** (go.dev breadcrumb, MDN header band,
   rustdoc top bar, Wikipedia en blank figure, DuckDuckGo favicons):
   icon fonts, a missing search bar, empty frames for images or
   embedded media.
