#!/bin/sh
# Packages a built compiler as a release tarball (.github/workflows/release.yml):
#
#     sh scripts/release/package.sh <target> <aye binary> <out dir>
#
# writes <out dir>/aye-<version>-<target>.tar.gz, <version> from
# `<aye binary> --version`. The tarball holds one directory,
# aye-<version>-<target>/:
#
#     bin/aye          the compiler; std is embedded in it, and the C
#                      runtime is emitted into each program's C, so it
#                      needs nothing else at run time but clang
#     lib/aye/std/     std's sources, for reading and for `aye lsp`'s
#                      go-to-definition (src/lsp/nav.rs finds them
#                      beside bin/)
#     README.md, LICENSE-MIT, LICENSE-APACHE, INSTALL.txt
#
# so it works unpacked anywhere; put bin/ on PATH.
set -eu

[ $# -eq 3 ] || { echo "usage: $0 <target> <aye binary> <out dir>" >&2; exit 2; }
target=$1
bin=$2
out=$3
root=$(cd "$(dirname "$0")/../.." && pwd)

version=$("$bin" --version | sed -n 's/^aye \([^ ]*\).*/\1/p')
[ -n "$version" ] || { echo "$0: cannot read the version from $bin --version" >&2; exit 1; }
name=aye-$version-$target

mkdir -p "$out"
out=$(cd "$out" && pwd)
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
dir=$stage/$name
mkdir -p "$dir/bin" "$dir/lib/aye"
cp "$bin" "$dir/bin/aye"
chmod 755 "$dir/bin/aye"
cp -R "$root/std" "$dir/lib/aye/std"
cp "$root/README.md" "$root/LICENSE-MIT" "$root/LICENSE-APACHE" "$dir/"
cat > "$dir/INSTALL.txt" <<TXT
$("$bin" --version)

Put bin/ on your PATH, for example:

    export PATH="/path/to/$name/bin:\$PATH"

aye compiles through clang, which must be installed:

    macOS:          xcode-select --install
    Ubuntu/Debian:  sudo apt install clang
    Fedora:         sudo dnf install clang
    Arch:           sudo pacman -S clang

Then: aye run hello.aye. See docs/install.md in the repository.
TXT
(cd "$stage" && tar -czf "$out/$name.tar.gz" "$name")
echo "$out/$name.tar.gz"
