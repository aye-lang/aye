#!/bin/sh
# Prints the release notes of a tag (.github/workflows/release.yml):
#
#     sh scripts/release/notes.sh v0.1.0
#
# the section of CHANGELOG.md headed `## <version>` or `## [<version>]`
# (up to the next `## `) if there is one, else the annotated tag's message,
# then what the binaries need. Run in a checkout that has the tag.
set -eu

[ $# -eq 1 ] || { echo "usage: $0 <tag>" >&2; exit 2; }
tag=$1
version=${tag#v}
root=$(cd "$(dirname "$0")/../.." && pwd)

body=
if [ -f "$root/CHANGELOG.md" ]; then
    body=$(awk -v v="$version" '
        /^## / {
            if (on) exit
            h = $0
            sub(/^## +\[?v?/, "", h)
            if (index(h, v) == 1 && substr(h, length(v) + 1, 1) ~ /^($|[] ])/) { on = 1; next }
        }
        on && /^\[[^]]+\]: / { next }
        on { print }
    ' "$root/CHANGELOG.md")
fi
if [ -z "$(printf '%s' "$body" | tr -d '[:space:]')" ]; then
    # The annotated tag's message; empty for a lightweight tag.
    if [ "$(git -C "$root" cat-file -t "$tag" 2>/dev/null)" = tag ]; then
        body=$(git -C "$root" tag -l --format='%(contents)' "$tag")
    fi
fi
if [ -z "$(printf '%s' "$body" | tr -d '[:space:]')" ]; then
    body="Aye $version."
fi

printf '%s\n' "$body"
cat <<'TXT'

## Install

Download the tarball for your machine, check it against `SHA256SUMS`
(`shasum -a 256 -c SHA256SUMS --ignore-missing`), unpack it anywhere and put
its `bin/` on your `PATH`. `aye` compiles through clang, which must be
installed: on macOS `xcode-select --install`, on Ubuntu or Debian
`sudo apt install clang`. See
[docs/install.md](https://github.com/aye-lang/aye/blob/main/docs/install.md).

The Linux binaries are built on Ubuntu 24.04 and need its glibc (2.39) or
newer; on an older distribution build from source (`cargo build --release`).
TXT
