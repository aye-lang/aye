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
# t <name> <want status> <want stdout> <stdin>; no leak report wanted
t() {
    local name=$1 want=$2 exp=$3 inp=$4
    printf '%s' "$inp" | perl -e 'alarm 10; exec @ARGV' "$T/prog" >"$T/out" 2>"$T/err"
    local got=$?
    printf '%s' "$exp" >"$T/exp"
    if [ "$got" -ne "$want" ] || ! cmp -s "$T/out" "$T/exp" || grep -qi leak "$T/err"; then
        echo "FAIL $name: status $got (want $want)"
        diff "$T/exp" "$T/out" | head -n 10
        head -n 5 "$T/err"
        fail=1
    elif [ "$want" -eq 1 ] && ! grep -q 'invalid json' "$T/err"; then
        echo "FAIL $name: no 'invalid json' on stderr"; fail=1
    else
        echo "ok $name"
    fi
}
t example 0 $'apple 3\npear 2\ntotal 5\n' \
    '{"items": [{"name": "apple", "qty": 3}, {"qty": 2, "name": "pear"}]}'
t empty_items 0 $'total 0\n' '{"items":[]}'
t other_keys 0 $'a "q" \\ caf\xc3\xa9 -4\nb/\t 10\ntotal 6\n' '
{
  "meta": {"v": [1, -2.5e3, true, false, null, {"a": "}]"}, []], "s": "x\"y"},
  "items": [
    {"tags": ["x", {"deep": [[[]]]}], "name": "a \"q\" \\ café", "qty": -4},
    {"name": "b\/\t", "price": 0.5E+2, "qty": 10}
  ],
  "after": "items"
}
'
t truncated 1 '' '{"items": [{"name": "a", "qty": 1}'
t trailing_comma 1 '' '{"items": [{"name": "a", "qty": 1},]}'
t bad_literal 1 '' '{"items": [], "x": tru}'
t no_items 1 '' '{"things": []}'
t not_json 1 '' 'hello'
t trailing_garbage 1 '' '{"items": []} x'
exit $fail
