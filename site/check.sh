#!/bin/sh
# Check the site before a deploy (card A1):
#   1. every samples/*.aye passes `aye check`;
#   2. every code block in public/index.html equals its sample
#      (site/highlight.py --check), and every sample is on the page;
#   3. no stale wording is left in public/ (the generated std/ is skipped).
# Needs a built compiler: `cargo build --release` (or AYE=<path>).
set -eu
here=$(cd "$(dirname "$0")" && pwd)
aye=${AYE:-$here/../target/release/aye}
for f in "$here"/samples/*.aye; do
    "$aye" check "$f"
done
python3 "$here/highlight.py" --check
if grep -riE --exclude-dir=std 'anyknow[n]|no public repository|not released|no download|pre-release' "$here/public"; then
    echo "site/check.sh: stale wording in site/public (above)" >&2
    exit 1
fi
echo "site/check.sh: ok"
