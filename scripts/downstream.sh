#!/bin/sh
# Downstream check (P8.3): builds and tests the anyknown browser
# (github.com/aye-lang/anyknown) with this checkout's compiler. Run it after
# a merge that touches src/ or std/, once `cargo build --release` is done.
#
#     sh scripts/downstream.sh
#
# Clones (or updates) target/downstream/anyknown at the rev in
# tests/downstream/ANYKNOWN_REV, then runs inside it, with
# AYE=<this checkout>/target/release/aye (override with AYE):
#
#     $AYE check browser
#     $AYE test browser
#     cargo test --release --test browser
#
# The output goes to target/downstream.log. Exit status 0 only when all
# three pass. Needs network and a GitHub login (`gh auth login`) while the
# repo is private. To move the pin, edit ANYKNOWN_REV (docs/plans/anyknown.md).
set -eu
cd "$(dirname "$0")/.."
root=$(pwd)
rev=$(tr -d ' \n' < tests/downstream/ANYKNOWN_REV)
dir=$root/target/downstream/anyknown
log=$root/target/downstream.log
AYE=${AYE:-$root/target/release/aye}
export AYE

if [ ! -x "$AYE" ]; then
    echo "downstream: $AYE not found; run cargo build --release first" >&2
    exit 1
fi

mkdir -p "$root/target/downstream"
: > "$log"

fetch() {
    if [ -d "$dir/.git" ]; then
        git -C "$dir" fetch --quiet origin
    elif command -v gh > /dev/null 2>&1; then
        gh repo clone aye-lang/anyknown "$dir" -- --quiet
    else
        git clone --quiet https://github.com/aye-lang/anyknown "$dir"
    fi
    git -C "$dir" checkout --quiet --detach "$rev"
}

step() {
    echo "== $*" >> "$log"
    if (cd "$dir" && "$@") >> "$log" 2>&1; then
        echo "downstream: ok   $*"
    else
        echo "downstream: FAIL $* (see $log)" >&2
        tail -n 30 "$log" >&2
        exit 1
    fi
}

echo "== fetch aye-lang/anyknown $rev" >> "$log"
if ! fetch >> "$log" 2>&1; then
    echo "downstream: FAIL clone or checkout of aye-lang/anyknown $rev (see $log)" >&2
    tail -n 30 "$log" >&2
    exit 1
fi
echo "downstream: anyknown $(git -C "$dir" rev-parse --short HEAD), compiler $AYE"

step "$AYE" check browser
step "$AYE" test browser
step cargo test --release --test browser
echo "downstream: all passed"
