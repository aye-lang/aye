#!/bin/sh
# Checks a release tarball the way a user meets it (release.yml runs it on
# each target's runner; run it by hand before tagging):
#
#     sh scripts/release/smoke.sh <aye-<version>-<target>.tar.gz>
#
# unpacks it into a fresh temporary directory outside the checkout, then
# with only that tree and the system clang: `aye --version` names the
# tarball's version, hello world runs and builds, and a program using std
# (embedded in the binary) and the debug allocator prints its result.
set -eu

[ $# -eq 1 ] || { echo "usage: $0 <tarball>" >&2; exit 2; }
tarball=$(cd "$(dirname "$1")" && pwd)/$(basename "$1")
name=$(basename "$tarball" .tar.gz)
version=$(echo "$name" | sed -n 's/^aye-\([^-]*\)-.*/\1/p')

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
cd "$tmp"
tar -xzf "$tarball"
aye=$tmp/$name/bin/aye
for f in bin/aye lib/aye/std/fs/fs.aye README.md INSTALL.txt; do
    [ -e "$name/$f" ] || { echo "smoke: $name/$f missing" >&2; exit 1; }
done

v=$("$aye" --version)
echo "$v"
case $v in
    "aye $version"|"aye $version ("*")") ;;
    *) echo "smoke: --version printed '$v', want aye $version" >&2; exit 1 ;;
esac

mkdir app && cd app
cat > hello.aye <<'AYE'
pub fn main() {
    @println("hello")
}
AYE
out=$("$aye" run hello.aye)
[ "$out" = hello ] || { echo "smoke: aye run hello.aye printed '$out'" >&2; exit 1; }
"$aye" build hello.aye -o hello
[ "$(./hello)" = hello ] || { echo "smoke: ./hello failed" >&2; exit 1; }
# std (embedded) and the C runtime, with the debug allocator.
cat > first.aye <<'AYE'
import std

pub fn main() !void {
    var gpa = std.heap.General.init()
    defer gpa.deinit()
    let alloc = gpa.allocator()
    let xs = try alloc.alloc(0 as i64, 4)
    defer alloc.free(xs)
    for i in 0..xs.len {
        xs[i] = i as i64 * 2
    }
    @println(xs[0] + xs[1] + xs[2] + xs[3])
}
AYE
out=$("$aye" run first.aye)
[ "$out" = 12 ] || { echo "smoke: aye run first.aye printed '$out'" >&2; exit 1; }
echo "smoke: $name ok"
