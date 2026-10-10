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
bad() { echo "FAIL $1"; head -n 5 "$T/err"; fail=1; }
# t <name> <input text> <want out file> <want stdout>
t() {
    local name=$1
    printf '%s' "$2" >"$T/in.txt"
    printf '%s' "$3" >"$T/want.txt"
    printf '%s' "$4" >"$T/want_stdout"
    printf 'old content\n' >"$T/out.txt"
    perl -e 'alarm 10; exec @ARGV' "$T/prog" in.txt out.txt >"$T/stdout" 2>"$T/err"
    local got=$?
    if [ "$got" -ne 0 ]; then bad "$name: status $got"; return; fi
    if ! cmp -s "$T/out.txt" "$T/want.txt"; then
        bad "$name: out file"; diff "$T/want.txt" "$T/out.txt" | head -n 10; return
    fi
    if ! cmp -s "$T/stdout" "$T/want_stdout"; then
        bad "$name: stdout"; diff "$T/want_stdout" "$T/stdout" | head -n 5; return
    fi
    if grep -qi leak "$T/err"; then bad "$name: leak"; return; fi
    echo "ok $name"
}
t example $'a\n\nc' $'1: a\n2: \n3: c\n' $'3 lines\n'
t trailing_newline $'one\ntwo\n' $'1: one\n2: two\n' $'2 lines\n'
t empty '' '' $'0 lines\n'
big=$(yes 'some text' | head -n 5000)
want=$(yes 'some text' | head -n 5000 | awk '{ print NR ": " $0 }')
t many "$big" "$want"$'\n' $'5000 lines\n'

rm -f "$T/out.txt"
perl -e 'alarm 10; exec @ARGV' "$T/prog" missing.txt out.txt >"$T/stdout" 2>"$T/err"
got=$?
if [ "$got" -ne 1 ]; then bad "missing: status $got (want 1)"
elif ! grep -q 'cannot open missing.txt' "$T/err"; then bad "missing: no 'cannot open missing.txt' on stderr"
elif [ -e "$T/out.txt" ]; then bad "missing: created out.txt"
else echo "ok missing"; fi

perl -e 'alarm 10; exec @ARGV' "$T/prog" in.txt >"$T/stdout" 2>"$T/err"
got=$?
if [ "$got" -ne 2 ]; then bad "usage: status $got (want 2)"
elif ! grep -q 'usage: numlines <in> <out>' "$T/err"; then bad "usage: no usage on stderr"
else echo "ok usage"; fi
exit $fail
