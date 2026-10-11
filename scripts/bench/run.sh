#!/bin/sh
# O7.1: ab -k against the three hello servers, one at a time.
# Usage: scripts/bench/run.sh   (from the repo root, after building target/hello_aye and target/hello_go)
cd "$(dirname "$0")/../.."
bench() { # name port cmd...
    name=$1; port=$2; shift 2
    "$@" & pid=$!
    sleep 1
    curl -s http://127.0.0.1:$port/ >/dev/null || echo "$name: no answer"
    ab -k -c 64 -n 1000 http://127.0.0.1:$port/ >/dev/null 2>&1
    for i in 1 2 3; do
        ab -k -c 64 -n 200000 http://127.0.0.1:$port/ > target/ab_${name}_$i.txt 2>&1
    done
    ab -k -c 1 -n 50000 http://127.0.0.1:$port/ > target/ab_${name}_c1.txt 2>&1
    kill $pid; wait $pid 2>/dev/null
    for f in target/ab_${name}_[123].txt target/ab_${name}_c1.txt; do
        echo "$name $(basename $f) $(grep -E 'Requests per second' $f | awk '{print $4}') $(grep -E 'Time per request.*mean\)' $f | head -1 | awk '{print $4}') failed=$(grep 'Failed requests' $f | awk '{print $3}')"
    done
}
bench aye 8081 target/hello_aye
bench node 8082 node scripts/bench/hello_node.js
bench go 8083 target/hello_go
