# Canvas 2D micro benchmark

`browser/tests/canvasbench/run.sh` builds `target/anyknown` in fast mode
and runs `bench.html` with `dump --text --js`. Each workload draws into a
fresh 800x600 canvas, in tasks small enough for the script budget; only
the drawing calls are timed (`Date.now`), and each workload's PNG is
hashed. It prints the median of three runs (`BENCH_RUNS`). Arguments
select workloads (`run.sh fillRect paths`), `BENCH_BIN` names a saved
binary, and `BENCH_BASELINE` interleaves an older one, prints speedups
and fails when a workload's pixels differ from it.

fillText uses the system fonts, so its hash depends on the machine.

Apple M2 Pro, shared with other work; milliseconds for all calls:

| workload | calls | e8e3e19 | 43e319a |
| --- | ---: | ---: | ---: |
| fillRect (12x10, 8 colors) | 1000 | 6362 | 10 |
| paths (arcs, polylines, curves; fill and stroke) | 300 | 2659 | 35 |
| fillText (12px) | 1000 | 7430 | 27 |
| drawImage (64x64 canvas, scaled and not) | 300 | 1812 | 19 |
| clipped (r=150 circles in a 300x200 clip) | 200 | 1663 | 43 |
| imageData (100x100 get and put) | 100 | 1403 | 45 |

The hashes of both builds are equal.
