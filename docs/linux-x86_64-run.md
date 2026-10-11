# Full suite on x86_64 Linux (O2.5c)

Ubuntu 24.04.5, x86_64, Firecracker VM (kernel cmdline `ipv6.disable=1`),
`scripts/setup-ubuntu.sh`, then `cargo test --release` one binary at a time.

| Target | Exit | Summary | Seconds |
|---|---|---|---|
| `cargo build --release` | 0 | Finished | 16 |
| `aye check browser` | 0 | - | 2 |
| `browser/build.sh` | 0 | - | 82 |
| `--lib` | 0 | ok. 32 passed | 12 |
| `--test browser` | 0 | ok. 62 passed | 265 |
| `--test conformance` | 0 | 94 passed; 0 failed | 5 |
| `--test fmt` | 0 | ok. 7 passed | 2 |
| `--test framework` | 0 | ok. 1 passed | 1 |
| `--test golden` | 1 | 444 passed; 1 failed (after fix; 443/2 before) | 170 |
| `--test pipe` | 0 | ok. 2 passed | 0 |
| `--test spec` | 0 | ok. 10 passed | 1 |

Remaining failure at that run: `cases/net_names.aye` needed an IPv6
loopback (`::1`), which this VM disables. Since O2.5d the `::1` part is
`cases/net_ipv6.aye` with a `needs ipv6` marker, skipped on such hosts.
