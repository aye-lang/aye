#!/usr/bin/env python3
"""Copy site/samples/*.aye into public/index.html, with syntax colors.

Every `<pre ... data-sample="NAME.aye"><code>...</code></pre>` block in
public/index.html gets the text of samples/NAME.aye, HTML-escaped and
wrapped in `<span class="k|t|s|n|c|b|f">` per token. The page has no
JavaScript; the colors are done here, at edit time.

    python3 site/highlight.py           rewrite public/index.html in place
    python3 site/highlight.py --check   exit 1 if a block differs from its
                                        sample, or a sample is not on the page
"""
import html
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
PAGE = os.path.join(HERE, "public", "index.html")
SAMPLES = os.path.join(HERE, "samples")

KEYWORDS = set("""
as break catch comptime continue defer else enum errdefer error extern false fn
for if import in interface let match mut null pub return struct test true try
type var where while
""".split())
TYPES = set("""
bool f32 f64 i8 i16 i32 i64 isize u8 u16 u32 u64 usize void noreturn
c_int c_uint c_long c_ulong c_char Self
""".split())

TOKEN = re.compile(r"""
    (?P<c>//[^\n]*)
  | (?P<s>"(?:\\.|[^"\\\n])*"|'(?:\\.|[^'\\\n])')
  | (?P<b>@[A-Za-z_]\w*)
  | (?P<n>\b(?:0x[0-9a-fA-F_]+|\d[\d_]*(?:\.\d+)?)\b)
  | (?P<i>[A-Za-z_]\w*)
""", re.VERBOSE)

BLOCK = re.compile(r'(<pre[^>]*\bdata-sample="([^"]+)"[^>]*><code>)(.*?)(</code></pre>)', re.S)


def highlight(src):
    out = []
    pos = 0
    prev = ""
    for m in TOKEN.finditer(src):
        out.append(html.escape(src[pos:m.start()], quote=False))
        text = m.group(0)
        kind = m.lastgroup
        if kind == "i":
            if text in KEYWORDS:
                kind = "k"
            elif text in TYPES or text[0].isupper():
                kind = "t"
            elif prev == "fn":
                kind = "f"
            else:
                kind = None
            prev = text
        esc = html.escape(text, quote=False)
        out.append(f'<span class="{kind}">{esc}</span>' if kind else esc)
        pos = m.end()
    out.append(html.escape(src[pos:], quote=False))
    return "".join(out)


def render(page):
    used = set()

    def repl(m):
        name = m.group(2)
        used.add(name)
        with open(os.path.join(SAMPLES, name)) as f:
            src = f.read().rstrip("\n")
        return m.group(1) + highlight(src) + m.group(4)

    return BLOCK.sub(repl, page), used


def main():
    check = "--check" in sys.argv[1:]
    with open(PAGE) as f:
        page = f.read()
    new, used = render(page)
    samples = {n for n in os.listdir(SAMPLES) if n.endswith(".aye")}
    unused = sorted(samples - used)
    if check:
        ok = True
        if new != page:
            print("site/highlight.py: public/index.html differs from samples/; run python3 site/highlight.py")
            ok = False
        for n in unused:
            print(f"site/highlight.py: samples/{n} is not on the page")
            ok = False
        sys.exit(0 if ok else 1)
    with open(PAGE, "w") as f:
        f.write(new)
    for n in unused:
        print(f"warning: samples/{n} is not on the page")


if __name__ == "__main__":
    main()
