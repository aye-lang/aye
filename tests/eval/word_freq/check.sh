#!/bin/bash
# check.sh <dir>: build <dir>/main.aye with `aye` from PATH and test it.
# Exit 0 when every case passes.
set -u
D=$(cd "$1" && pwd)
T=$(mktemp -d)
trap 'rm -rf "$T"' EXIT
cd "$T"
aye build "$D/main.aye" -o "$T/prog" >"$T/build.log" 2>&1 || {
    echo "FAIL build"; tail -n 20 "$T/build.log"; exit 1
}
fail=0
# t <name> <want stdout> <stdin file>; status 0 and no leak report wanted
t() {
    local name=$1 exp=$2 inp=$3
    perl -e 'alarm 20; exec @ARGV' "$T/prog" <"$inp" >"$T/out" 2>"$T/err"
    local got=$?
    printf '%s' "$exp" >"$T/exp"
    if [ "$got" -ne 0 ] || ! cmp -s "$T/out" "$T/exp" || grep -qi leak "$T/err"; then
        echo "FAIL $name: status $got"
        diff "$T/exp" "$T/out" | head -n 10
        head -n 5 "$T/err"
        fail=1
    else
        echo "ok $name"
    fi
}
printf 'The cat and the hat.\nThe CAT sat; a cat!\n' >"$T/in1"
t example $'3 cat\n3 the\n1 a\n1 and\n1 hat\n1 sat\n' "$T/in1"
printf '' >"$T/in2"
t empty '' "$T/in2"
printf '...,, 123\n' >"$T/in3"
t no_words '' "$T/in3"
printf 'zeta Beta  alpha\tgamma-beta\nAlpha2zeta' >"$T/in4"
t ties $'2 alpha\n2 beta\n2 zeta\n1 gamma\n' "$T/in4"
yes 'alpha beta beta' | head -n 100000 >"$T/in5"
t large $'200000 beta\n100000 alpha\n' "$T/in5"
exit $fail
