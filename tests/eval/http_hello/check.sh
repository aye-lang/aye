#!/bin/bash
# check.sh <dir>: build <dir>/main.aye with `aye` from PATH, start it on a
# free port and test it with curl. Exit 0 when every case passes.
set -u
D=$(cd "$1" && pwd)
T=$(mktemp -d)
PID=
trap '[ -n "$PID" ] && kill "$PID" 2>/dev/null; rm -rf "$T"' EXIT
cd "$T"
aye build "$D/main.aye" -o "$T/prog" >"$T/build.log" 2>&1 || {
    echo "FAIL build"; tail -n 20 "$T/build.log"; exit 1
}
PORT=$(python3 -c 'import socket; s = socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1])')
"$T/prog" "$PORT" >"$T/stdout" 2>"$T/err" &
PID=$!
up=0
for _ in $(seq 1 50); do
    if ! kill -0 "$PID" 2>/dev/null; then break; fi
    if curl -s -o /dev/null "http://127.0.0.1:$PORT/"; then up=1; break; fi
    sleep 0.2
done
if [ "$up" -ne 1 ]; then
    echo "FAIL start: no server on port $PORT"; head -n 5 "$T/err"; exit 1
fi
fail=0
# t <name> <path> <want status> <want body>
t() {
    local name=$1 path=$2 want=$3
    printf '%s' "$4" >"$T/want"
    local got
    got=$(curl -s -m 5 -D "$T/headers" -o "$T/body" -w '%{http_code}' "http://127.0.0.1:$PORT$path")
    if [ "$got" != "$want" ]; then
        echo "FAIL $name: status $got (want $want)"; fail=1
    elif ! cmp -s "$T/body" "$T/want"; then
        echo "FAIL $name: body"; diff "$T/want" "$T/body" | head -n 5; fail=1
    elif ! tr -d '\r' <"$T/headers" | grep -qi '^content-type: text/plain'; then
        echo "FAIL $name: no Content-Type: text/plain"; fail=1
    else
        echo "ok $name"
    fi
}
t hello /hello 200 $'hello, world\n'
t name '/hello?name=Ann' 200 $'hello, Ann\n'
t name_digits '/hello?name=bob42' 200 $'hello, bob42\n'
t not_found /nope 404 $'not found\n'
t root / 404 $'not found\n'
t again /hello 200 $'hello, world\n'
exit $fail
