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
# t <name> <want status> <want stdout> <stdin> [args...]
t() {
    local name=$1 want=$2 exp=$3 inp=$4
    shift 4
    printf '%s' "$inp" | perl -e 'alarm 10; exec @ARGV' "$T/prog" "$@" >"$T/out" 2>"$T/err"
    local got=$?
    printf '%s' "$exp" >"$T/exp"
    if [ "$got" -ne "$want" ] || ! cmp -s "$T/out" "$T/exp"; then
        echo "FAIL $name: status $got (want $want)"
        diff "$T/exp" "$T/out" | head -n 10
        fail=1
    else
        echo "ok $name"
    fi
}
IN=$'apple\nbanana\ncherry pie\napple pie'
t substring 0 $'2:banana\n' "$IN" an
t last_line_no_newline 0 $'3:cherry pie\n4:apple pie\n' "$IN" pie
t case_sensitive 1 '' "$IN" Apple
t no_match 1 '' "$IN" zzz
t empty_input 1 '' '' a
t no_args 2 '' "$IN"
grep -q 'usage: grepn <word>' "$T/err" || { echo "FAIL no_args: no usage on stderr"; fail=1; }
t two_args 2 '' "$IN" a b
exit $fail
