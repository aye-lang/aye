#!/usr/bin/env python3
"""Rename the language from Ore to Aye (X2.1). Stdlib only; run from the repo root.

One-off, already run (X2.2). The `browser/` paths in its tables are the layout at that
time; `browser/` has since moved to aye-lang/anyknown (P8).

  python3 scripts/rename-aye.py --dry-run   change nothing, print a report
  python3 scripts/rename-aye.py --apply     git mv files, then rewrite file contents

Rules (case-sensitive), applied to every tracked text file that is not excluded:
  ORE_xxx            -> AYE_xxx     env vars, C prelude macros
  ore_xxx            -> aye_xxx     C runtime / helper symbols (ore_std_errno, ...)
  CARGO_BIN_EXE_ore  -> CARGO_BIN_EXE_aye
  Ore                -> Aye         whole word
  ore                -> aye         whole word: command, crate, `.ore`, ```ore fences,
                                    ~/.cache/ore, ore.lock, target/release/ore, ...
Whole word means: not preceded or followed by a letter, digit or underscore, so
"more", "store", "core", "ignore", "orelse" are never touched. The same word
rule renames tracked file names (x.ore -> x.aye, docs/plans/ore.md -> aye.md).
Every replacement keeps the length (3 letters), so error columns do not shift.
"""
import collections
import re
import subprocess
import sys

SELF = "scripts/rename-aye.py"

# Word-boundary pieces: a "letter" side may not touch [A-Za-z0-9_].
B = r"(?<![A-Za-z0-9_])"
E = r"(?![A-Za-z0-9_])"
RULES = [
    ("ORE_ identifier", re.compile(r"(?<![A-Za-z0-9])ORE_"), "AYE_"),
    ("ore_ identifier", re.compile(r"(?<![A-Za-z0-9])ore_"), "aye_"),
    ("-lore_ linker flag", re.compile(r"(?<=-l)ore_"), "aye_"),
    ("CARGO_BIN_EXE_ore", re.compile(r"CARGO_BIN_EXE_ore" + E), "CARGO_BIN_EXE_aye"),
    ("Ore (word)", re.compile(B + r"Ore" + E), "Aye"),
    ("ore (word)", re.compile(B + r"ore" + E), "aye"),
]
# One pass, so no replacement is rescanned by a later rule.
COMBINED = re.compile("|".join("(?:%s)" % r[1].pattern for r in RULES))

# Candidates that look like Ore but no rule changes (ore glued to _, digit, camel case).
CAND = re.compile(
    r"[A-Za-z0-9_]*(?:(?<![A-Za-z])(?:ore|Ore|ORE)(?![a-z])|[a-z0-9_]Ore|[a-z0-9]ORE)[A-Za-z0-9_]*"
)

# Phrases that state the rename itself ("Ore -> Aye", "Ore (Aye)", "Ore/Aye",
# "Ore, being renamed Aye") keep their old name; the rest of the line is renamed.
OLD_WORD = re.compile(B + r"(?:Ore|ore)" + E)
PHRASE = re.compile(r"\s*(?:→|->|/|\(|,?\s*(?:being\s+)?renamed(?:\s+to)?|,?\s*now)\s*[`\"*]*(?:Aye|aye)" + E)
# Page text of the browser fixtures (browser/tests/*.html) quoted outside them.
PAGE_TEXT = re.compile(r"The Ore Times|Written in Ore|Notes on Ore")
# The X2 cards of the project plan describe the rename; they must keep their text.
X2_FILE = "docs/plans/project.md"
# Recorded search query typed into real sites: data, not the language name.
QUERY = re.compile(r"\bore[ +]programming")

# Page content: fixtures the browser loads or its expected renderings.
FIXTURE_KEEP_SUFFIX = (".py", ".sh", ".aye", ".ore", ".rs", ".toml")


def sh(*args):
    return subprocess.run(args, check=True, capture_output=True).stdout


def tracked():
    return [p.decode() for p in sh("git", "ls-files", "-z").split(b"\0") if p]


def rename_word(s):
    return re.sub(B + r"ore" + E, "aye", s)


def exclusion(path):
    """Return a reason string when the whole file is left alone, else None."""
    if path == SELF:
        return "the script itself"
    if path.startswith("browser/tests/") and not path.endswith(FIXTURE_KEEP_SUFFIX) \
            and path != "browser/tests/canvas/README.md":
        return "browser/tests page-content fixture"
    if path.startswith("browser/tools/agent_tasks/") and path.endswith(".jsonl"):
        return "recorded agent task data"
    return None


def read_text(path):
    try:
        data = open(path, "rb").read()
    except OSError:
        return None  # e.g. a broken symlink
    if b"\0" in data:
        return None
    try:
        return data.decode("utf-8")
    except UnicodeDecodeError:
        return None


def category(path):
    if path.startswith("tests/") or path.startswith("browser/tests/"):
        if path.endswith(".rs"):
            return "rust tests"
        if path.endswith((".aye", ".ore")):
            return "test programs (.ore)"
        return "test expectations (.out/.err/...)"
    if path.endswith(".rs"):
        return "rust compiler"
    if path.endswith((".ore", ".aye")):
        return "ore source (std, browser)"
    if path.endswith(".md"):
        return "docs (*.md)"
    return "config/scripts/other"


def plan(files):
    moves = []
    for p in files:
        if re.search(B + r"ore" + E, p):
            moves.append((p, rename_word(p)))
    edits = {}  # path -> new text
    stat = collections.defaultdict(lambda: [0, 0, 0])  # category -> files, lines, matches
    excluded_files = collections.defaultdict(list)  # reason -> [(path, matches)]
    skipped = collections.defaultdict(list)  # reason -> [path:line: text]
    candidates = collections.defaultdict(list)  # kind -> [path:line: token]
    ctx = collections.Counter()
    syms = collections.Counter()
    hist = {"docs/plans/done.md": 0, "docs/milestones.md": 0}
    for p in files:
        text = read_text(p)
        if text is None:
            continue
        reason = exclusion(p)
        if reason:
            n = len(OLD_WORD.findall(text)) + len(re.findall(r"\b(?:ORE_|ore_)\w+", text))
            if n:
                excluded_files[reason].append((p, n))
            continue
        out = []
        in_x2 = False
        changed_lines = matches = 0
        for i, line in enumerate(text.splitlines(keepends=True), 1):
            if p == X2_FILE and line.startswith("## "):
                in_x2 = line.startswith("## X2")
            hit = COMBINED.search(line)
            if hit:
                if in_x2:
                    skipped["X2 cards in docs/plans/project.md (describe the rename)"].append(
                        "%s:%d: %s" % (p, i, line.strip()[:110]))
                    out.append(line)
                    continue
                if QUERY.search(line):
                    skipped["recorded search query 'ore programming language'"].append(
                        "%s:%d: %s" % (p, i, line.strip()[:110]))
                    out.append(line)
                    continue

                def sub(m):
                    s = m.group(0)
                    if s == "Ore" and any(t.start() <= m.start() < t.end() for t in PAGE_TEXT.finditer(line)):
                        skipped["page text of a browser fixture, quoted outside it"].append(
                            "%s:%d: %s" % (p, i, line.strip()[:110]))
                        return s
                    if s in ("Ore", "ore") and PHRASE.match(line, m.end()):
                        skipped["phrase naming the rename (Ore -> Aye, Ore (Aye), ...)"].append(
                            "%s:%d: %s" % (p, i, line.strip()[:110]))
                        return s
                    ctx[(s, _context(line, m))] += 1
                    if s in ("ORE_", "ore_"):
                        syms[re.match(r"[A-Za-z0-9_]*", line[m.start():]).group(0)] += 1
                    return REPL[s]
                new_line, n = COMBINED.subn(sub, line)
                matches += n
                changed_lines += 1
                out.append(new_line)
                if p in hist:
                    hist[p] += 1
            else:
                out.append(line)
            for m in CAND.finditer(line):
                tok = m.group(0)
                if not COMBINED.search(tok):
                    candidates["ore glued to _, digit or camel case"].append(
                        "%s:%d: %s" % (p, i, tok))
        if matches:
            cat = category(p)
            stat[cat][0] += 1
            stat[cat][1] += changed_lines
            stat[cat][2] += matches
            edits[p] = "".join(out)
    return moves, edits, stat, excluded_files, skipped, candidates, ctx, syms, hist


REPL = {"ORE_": "AYE_", "ore_": "aye_", "Ore": "Aye", "ore": "aye",
        "CARGO_BIN_EXE_ore": "CARGO_BIN_EXE_aye"}


def _context(line, m):
    """One char before, the match, and the word after it."""
    before = line[max(0, m.start() - 1):m.start()]
    after = re.match(r"\s\w+|[^\w\s]?", line[m.end():]).group(0)
    return (before + m.group(0) + after).replace("\n", " ")


def report(res):
    moves, edits, stat, excluded_files, skipped, candidates, ctx, syms, hist = res
    p = print
    p("== rename Ore -> Aye: dry run ==")
    p("files to move (git mv): %d" % len(moves))
    kinds = collections.Counter(("*.ore -> *.aye" if a.endswith(".ore") else "other") for a, _ in moves)
    for k, v in sorted(kinds.items()):
        p("  %-20s %d" % (k, v))
    for a, b in moves:
        if not a.endswith(".ore"):
            p("  %s -> %s" % (a, b))
    p("")
    p("content changes per category (files / lines / replacements):")
    tf = tl = tm = 0
    for k, (f, l, m) in sorted(stat.items()):
        p("  %-36s %5d %7d %7d" % (k, f, l, m))
        tf += f; tl += l; tm += m
    p("  %-36s %5d %7d %7d" % ("TOTAL", tf, tl, tm))
    p("")
    p("history files renamed in prose too (lines changed): " + ", ".join("%s=%d" % kv for kv in hist.items()))
    p("  (commit hashes never match a rule, so they stay as they are)")
    p("")
    p("== NOT changed: whole files ==")
    for reason, lst in sorted(excluded_files.items()):
        p("%s: %d files, %d candidate matches" % (reason, len(lst), sum(n for _, n in lst)))
        for path, n in sorted(lst):
            p("    %s (%d)" % (path, n))
    p("")
    p("== NOT changed: lines (file:line) ==")
    for reason, lst in sorted(skipped.items()):
        p("%s: %d" % (reason, len(lst)))
        for x in lst:
            p("    " + x)
    p("")
    p("== NOT changed: look-alikes no rule matches (review) ==")
    for reason, lst in sorted(candidates.items()):
        p("%s: %d" % (reason, len(lst)))
        for x in lst:
            p("    " + x)
    p("")
    p("== ORE_ / ore_ identifiers renamed (%d distinct) ==" % len(syms))
    for k, v in sorted(syms.items()):
        p("  %-40s -> %-40s %d" % (k, re.sub(r"^ORE_", "AYE_", re.sub(r"^ore_", "aye_", k)), v))
    p("")
    p("== 20 most frequent replacement contexts (pattern, +-14 chars) ==")
    for (tok, c), n in ctx.most_common(20):
        p("  %6d  %-18s %s" % (n, tok, c))


def apply(res):
    moves, edits = res[0], res[1]
    for path, text in edits.items():
        with open(path, "w", encoding="utf-8", newline="") as f:
            f.write(text)
    for a, b in moves:
        subprocess.run(["git", "mv", a, b], check=True)
    print("moved %d files, rewrote %d files" % (len(moves), len(edits)))


def main():
    if sys.argv[1:] not in (["--dry-run"], ["--apply"]):
        sys.exit(__doc__)
    res = plan(tracked())
    if sys.argv[1] == "--dry-run":
        report(res)
    else:
        apply(res)


if __name__ == "__main__":
    main()
