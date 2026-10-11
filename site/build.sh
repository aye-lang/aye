#!/bin/sh
# Write the std reference (`aye doc --html std`) into site/public/std.
# Needs a built compiler: `cargo build --release` (or AYE=<path>).
set -eu
here=$(cd "$(dirname "$0")" && pwd)
aye=${AYE:-$here/../target/release/aye}
rm -rf "$here/public/std"
"$aye" doc --html std -o "$here/public/std"
