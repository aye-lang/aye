#!/bin/sh
# Builds the anyknown browser for real use: `--mode fast` (optimized, no
# safety checks; the tests build it in debug mode). The output goes to
# target/anyknown in this checkout (your worktree) unless a path is
# given; a relative path is taken from the checkout's root. Do not give a
# path outside your worktree or a fixed name that other agents share (the
# session scratchpad): another agent's build can replace your binary
# before your tests run.
#
#     browser/build.sh [OUT] [--mode safe|fast|debug]
#
# `--mode safe` keeps the bounds and overflow checks (a panic names the
# line) at about the speed of `fast`; use it to look into a crash on a
# real site.
set -e
cd "$(dirname "$0")/.."
out=target/anyknown
mode=fast
while [ $# -gt 0 ]; do
    case "$1" in
        --mode) mode="$2"; shift 2 ;;
        *) out="$1"; shift ;;
    esac
done
cargo build --release --quiet
mkdir -p "$(dirname "$out")"
target/release/aye build browser -o "$out" --mode "$mode"
echo "$out ($mode)"
