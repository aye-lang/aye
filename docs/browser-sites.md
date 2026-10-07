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
page's content is not available; excluded: not judged.

**Update of 2026-10-07.** The statuses now follow
`browser/tools/sites.json`, which `python3 browser/tools/page_load.py
check` replays from archives captured that day (`--agent` also runs one
agent task per site; `browser/engine/PERF.md`). Load times and the
"JS errors" column are still those of the run by hand above; where the
replay logs errors, the note says so. Changes since that run: YouTube
❌ to ✅; Reddit and Stack Overflow ❌ to excluded (bot walls, plan
Decisions); `youtube-search` added, excluded until its replay is
deterministic (plan P1.7).

**Update of 2026-10-08.** `youtube-search` excluded to ✅: it replays
deterministically since tasks on the virtual clock are limited by steps
and the clock moves inside a task (plan P1.7–P1.10). GitHub's latest
commit and commit columns load (plan P3.26); BBC's and Wikipedia ja's
notes were rechecked after replaced-element sizing (plan P3.4); GitHub's and
Mistral's agent tasks pass (plan P3.30). The "Agent task" column is the
task `check --agent` runs (`browser/tools/agent_tasks/`): ✅ passes, ❌
the failure `sites.json` expects (`task_fails`), – no task.

## Sites

| Site | URL | Status | Load | JS errors | Agent task | Notes |
|---|---|---|---|---|---|---|
| example.com | https://example.com/ | ✅ | 0.3 s | 0 | click "Learn more" ✅ | Text and layout right. The script-added translations now render: Chinese is drawn, Arabic is joined and right to left. |
| Wikipedia (en) | https://en.wikipedia.org/wiki/Web_browser | ✅ | 2.8 s | 0 | search "HTML", open the article ✅ | Looks like the real page: logo and icons (SVG) are drawn, article, infobox image and headings right. One figure further down is an empty white frame. Markdown is clean (no empty list items). Agent search on Special:Search works (typed, submitted, results page loaded). |
| Hacker News | https://news.ycombinator.com/ | ✅ | 1.3 s | 0 | click "new" ✅ | Looks like the real page, logo drawn; the vote arrows are tiny. Markdown is a numbered list of stories. Agent `click` on "new" works (`/newest` loaded). |
| GitHub repository | https://github.com/rust-lang/rust | ⚠️ | 5.2 s | 0 | click Issues ✅ | Header, tabs, icons, file list and About panel render. The latest commit and the commit message and date columns load (plan P3.26: the earlier 400s on `/latest-commit` and `/tree-commit-info` came from an archive captured when fetch dropped the Accept header); the "History" section is empty. The replay logs no errors. Agent `click` on "Contributing" works (it scrolls to `#`; the link is an in-page toggle). |
| MDN | https://developer.mozilla.org/en-US/docs/Web/HTML/Element/details | ✅ | 0.7 s | 0 | click `<summary>` ✅ | Header with logo, breadcrumb, "In this article" sidebar and the article lay out correctly. A blank 90 px band above the header; browser-support icons partly missing. The replay logs 18 errors: 16 lit "Hydration value mismatch" and 2 "#<Object> is not iterable" (plan P3.14). |
| Python docs | https://docs.python.org/3/tutorial/index.html | ✅ | 0.9 s | 0 | click chapter 1 ✅ | Renders like a desktop browser, theme and version selectors work. Agent `click` to the next chapter works. |
| rustdoc (std) | https://doc.rust-lang.org/std/vec/struct.Vec.html | ✅ | 1.3 s | 0 | click "std" ✅ | Sidebar, code highlighting and prose render well; the logo is a black blob. No search bar or settings button in the top bar. |
| BBC News | https://www.bbc.com/news | ⚠️ | 2.6 s | 0 | click an article ✅ | Markdown has the stories. On screen the menus are bullet lists and plain buttons, headlines are serif links. The SVG wordmarks render at 140x40 and 100x26 as the page's CSS asks (the earlier 290 px came from a CSS-load failure). Story images do load (the first screenful has none). |
| CNN Lite | https://lite.cnn.com/ | ✅ | 0.6 s | 0 | click a story ✅ | Plain list of stories, renders right. |
| old Reddit | https://old.reddit.com/r/programming/ | excluded | 4.1 s | 0 | – | The site redirects logged-out clients to `/login/?reason=lor2` (curl gets the same 302). The login page now draws its header and chat-bubble background but no login form; the Markdown is only a skip link. |
| Stack Overflow | https://stackoverflow.com/questions/11227809/... | excluded | 0.4 s | 0 | – | Cloudflare challenge (HTTP 403, "Just a moment..."): blank page, empty Markdown. curl with a Chrome User-Agent gets 403 too. |
| Lobsters | https://lobste.rs/ | ✅ | 1.9 s | 0 | click "Recent" ✅ | Renders like the real page, symbols and avatars included. |
| Go docs | https://go.dev/doc/effective_go | ✅ | 2.5 s | 0 | click "Documentation" ✅ | Header, breadcrumb, two-column table of contents right; the nav drawer is closed. Flaws: the breadcrumb separator is a grey box (icon font), the cookie bar covers the bottom. Markdown complete. |
| DuckDuckGo HTML | https://html.duckduckgo.com/html/?q=ore+programming+language | ✅ | 2.1 s | 0 | type + submit ✅ | Results readable; a few favicons are empty boxes. Agent `type` + submit (a POST form) works. |
| Google | https://www.google.com/ | ✅ | 0.8 s | 0 | click 進階搜尋 ✅ | Logo, centred search box, buttons and the zh-TW text (CJK glyphs now drawn) all right. The two buttons touch. |
| Wikipedia (ja) | https://ja.wikipedia.org/wiki/ウェブブラウザ | ✅ | 5.3 s | 0 | search "HTML", open the article ✅ | Japanese text readable on screen and in Markdown. The browser-share chart is CSS border triangles, drawn correctly. |
| 阿部寛 (Shift_JIS) | http://abehiroshi.la.coocan.jp/ | ✅ | 0.8 s | 0 | click 映画出演 (frame) ✅ | Frameset works: menu frame and top frame laid out side by side, Shift_JIS decoded, photo and text right. The "最新情報" banner wraps in a narrow cell. Markdown has a section per frame. |
| ZOL (GBK) | https://www.zol.com.cn/ | ⚠️ | 16-20 s | 0 | click 新闻 ✅ | GBK decoded: Chinese text, images, logos and the product grid render. The right edge is cut off (search box, nav, a column past 1024 px). Slowest site by far (16.3 s and 20.2 s in two runs). |
| YouTube | https://www.youtube.com/ | ✅ | 5.0 s | 0 | search: ❌ a script cancels the submission | YouTube serves a cookieless visitor an empty feed (`feedNudgeRenderer`, "Try searching to get started"); we render exactly that. |
| YouTube search | https://www.youtube.com/results?search_query=rust+programming | ✅ | – | – | – | Added 2026-10-07 (plan P3.2). Replays deterministically since plan P1.7–P1.10: 5 of 5 checks gave the same Markdown with no archive misses. A dump stops after 2 growth rounds of the endless results (plan P4.25). |
| react.dev | https://react.dev/ | ✅ | 3.6 s | 0 | click "Learn React" ✅ | Renders well: logo, search box, hero, buttons. Markdown code blocks keep their line breaks. |
| Mistral blog (Tailwind 4, Astro) | https://mistral.ai/news/mistral-large-4/ | ⚠️ | 13 s | 2 | click "Blog" ✅ | Added 2026-10-07 at `ddaada2`. Looks like Chrome in its light scheme: Inter text, nav with the black Login button, a viewport-high dark hero with white "Back to Blog"/"Share this post" pills and "14 MIN READ", the article with its sticky model card. The hero title stays hidden (its three.js scene needs WebGL; both errors are the expected `getContext('webgl')` null). The embedded widgets (iframes) are broken: the CTF chart's rows overlap and the frames stay short (layout queries return 0), and the visual-intelligence viewers log 9 `setTransform` errors in a long `dump` (no 2D canvas). Markdown is complete. Load time varies with the network (6.5-16 s). The replay logs 3 errors: the 2 WebGL context errors and one 'reading load'. |

**Totals (2026-10-08, `sites.json`):** 16 ✅, 4 ⚠️, 2 excluded (22
sites, Mistral and YouTube search included); 2026-10-07: 15 ✅, 4 ⚠️,
3 excluded. Totals of the run by hand:
14 ✅, 3 ⚠️, 3 ❌ (20 sites). Previous run (2026-10-06,
`d1c8800`): 8 ✅, 7 ⚠️, 5 ❌. No crash and no timeout; the slowest run
was 20 s (ZOL), the others under 6 s. In the run by hand no site logs
a JS error.

Changes: ⚠️ to ✅ (MDN, go.dev, Google, Wikipedia ja, react.dev); ❌ to
✅ (阿部寛); ❌ to ⚠️ (ZOL). All other sites keep their status (GitHub and
BBC stay ⚠️; Reddit, Stack Overflow and YouTube stay ❌).

The Mistral blog was added after this run (the totals of the run by
hand do not count it). Before its fixes it logged `getAnimations`,
`addEventListener` and `URL` errors and drew a serif, unstyled page: Tailwind 4's theme
(`:root,:host`), its `oklch()` palette, `100dvh` heroes and
`text-transform` were not supported, and stretched flex items were not
laid out at their stretched height.

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
   diagnosed. Since diagnosed: YouTube's skeleton is the empty feed it
   serves a cookieless visitor; GitHub's commit data failed in an
   archive captured when fetch dropped the Accept header, and loads now
   (plan P3.26).
3. **CSS not applied or applied wrong on large sites** (BBC, ZOL):
   BBC's menus are bullet lists; ZOL's content runs past the right edge.
   Since rechecked: BBC's 290 px SVG wordmarks came from a CSS-load
   failure and Wikipedia ja's chart is drawn correctly (plan P3.4).
4. **Slow loads** (ZOL 16-20 s, GitHub 5.2 s, Wikipedia ja 5.3 s,
   YouTube 5.0 s): subresources and scripts of heavy pages.
5. **Small drawing gaps** (go.dev breadcrumb, MDN header band,
   rustdoc top bar, Wikipedia en blank figure, DuckDuckGo favicons):
   icon fonts, a missing search bar, empty frames for images or
   embedded media.
