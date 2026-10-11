#!/bin/bash
# check.sh <dir>: build <dir>/main.aye with `aye` from PATH, start it on a
# free port and run a scripted curl sequence against it. JSON bodies are
# compared after parsing (whitespace and key order are free). Exit 0 when
# every case passes.
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
# t <name> <want status> <want json> <curl args...>   (the URL path is the last argument)
t() {
    local name=$1 want=$2 json=$3
    shift 3
    local path=${*: -1}
    set -- "${@:1:$#-1}"
    local got
    got=$(curl -s -m 5 -D "$T/headers" -o "$T/body" -w '%{http_code}' "$@" "http://127.0.0.1:$PORT$path")
    if [ "$got" != "$want" ]; then
        echo "FAIL $name: status $got (want $want)"; head -c 300 "$T/body"; echo; fail=1
    elif ! tr -d '\r' <"$T/headers" | grep -qi '^content-type: application/json'; then
        echo "FAIL $name: no Content-Type: application/json"; fail=1
    elif ! python3 -c '
import json, sys
want = json.loads(sys.argv[1])
got = json.load(open(sys.argv[2], encoding="utf-8"))
sys.exit(0 if got == want else 1)' "$json" "$T/body" 2>/dev/null; then
        echo "FAIL $name: body"; echo "want $json"; head -c 300 "$T/body"; echo; fail=1
    else
        echo "ok $name"
    fi
}
t empty 200 '{"items": []}' /items
t post1 201 '{"id": 1, "name": "apple"}' -X POST -H 'Content-Type: application/json' -d '{"name": "apple"}' /items
t post2 201 '{"id": 2, "name": "pear \"x\""}' -X POST -H 'Content-Type: application/json' -d '{ "name" : "pear \"x\"" }' /items
t list 200 '{"items": [{"id": 1, "name": "apple"}, {"id": 2, "name": "pear \"x\""}]}' /items
t one 200 '{"id": 2, "name": "pear \"x\""}' /items/2
t missing 404 '{"error": "not found"}' /items/99
t zero 404 '{"error": "not found"}' /items/0
t word 404 '{"error": "not found"}' /items/abc
t other 404 '{"error": "not found"}' /nope
t bad_json 400 '{"error": "bad request"}' -X POST -d 'not json' /items
t no_name 400 '{"error": "bad request"}' -X POST -d '{"title": "x"}' /items
t list_again 200 '{"items": [{"id": 1, "name": "apple"}, {"id": 2, "name": "pear \"x\""}]}' /items
exit $fail
