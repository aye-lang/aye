#!/bin/sh
# O7.1: ab -k against the three hello servers, one at a time.
# F5.9 adds the framework hello page (8084, /) and the pages example's /about (8085).
# Usage: scripts/bench/run.sh   (from the repo root; it builds target/hello_aye, target/hello_go and the
# framework servers itself, with target/release/aye)
cd "$(dirname "$0")/../.."
bench() { # name port path cmd...
    name=$1; port=$2; path=$3; shift 3
    "$@" & pid=$!
    sleep 1
    curl -s http://127.0.0.1:$port$path >/dev/null || echo "$name: no answer"
    ab -k -c 64 -n 1000 http://127.0.0.1:$port$path >/dev/null 2>&1
    for i in 1 2 3; do
        ab -k -c 64 -n 200000 http://127.0.0.1:$port$path > target/ab_${name}_$i.txt 2>&1
    done
    ab -k -c 1 -n 50000 http://127.0.0.1:$port$path > target/ab_${name}_c1.txt 2>&1
    kill $pid; wait $pid 2>/dev/null
    for f in target/ab_${name}_[123].txt target/ab_${name}_c1.txt; do
        echo "$name $(basename $f) $(grep -E 'Requests per second' $f | awk '{print $4}') $(grep -E 'Time per request.*mean\)' $f | head -1 | awk '{print $4}') failed=$(grep 'Failed requests' $f | awk '{print $3}')"
    done
}

AYE=$PWD/target/release/aye; export AYE
mkdir -p target/bench
"$AYE" build --mode fast scripts/bench/hello_http.aye -o target/hello_aye || exit 1
go build -o target/hello_go scripts/bench/hello_go.go || exit 1
# The hello example is a hand-written program (no app/ tree), so it is built with `aye build`;
# the pages example goes through framework/build, which passes --mode fast on.
"$AYE" build --mode fast framework/examples/hello -o target/bench/hello-prod || exit 1
"$AYE" run framework/build -- framework/examples/pages -o target/bench/pages-prod --mode fast || exit 1
bench aye 8081 / target/hello_aye
bench node 8082 / node scripts/bench/hello_node.js
bench go 8083 / target/hello_go
bench framework 8084 / target/bench/hello-prod --port 8084
bench pages_about 8085 /about target/bench/pages-prod --port 8085
