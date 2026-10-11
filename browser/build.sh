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
# In the Aye repository it builds the compiler first and uses
# target/release/aye. Anywhere else (the anyknown repository) it uses the
# `aye` named by $AYE, or `aye` on the PATH.
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
if [ -z "${AYE:-}" ]; then
    if grep -qs '^name = "aye"$' Cargo.toml; then
        cargo build --release --quiet
        AYE=target/release/aye
    else
        AYE=aye
    fi
fi
mkdir -p "$(dirname "$out")"
"$AYE" build browser -o "$out" --mode "$mode"
echo "$out ($mode)"
