# Contributing

Build: Rust (cargo) and `clang` on `PATH` are required (see the README for
optional libraries such as libclang, libcurl and SDL2).

```sh
cargo build --release
cargo test                         # all suites
cargo test --test golden           # one suite: golden, browser, spec, fmt
browser/build.sh                   # build the browser (target/anyknown)
```

Workflow: work is split into lanes, phases and steps in
[`docs/plan.md`](docs/plan.md). One change is one step: one goal with one
verify command, done on a short-lived branch (a git worktree off `main`),
committed as a small reviewable series, then merged into `main`. Before you
change the compiler, std or the browser, read
[`docs/practices/`](docs/practices/README.md).

Contributions are dual licensed under MIT OR Apache-2.0 (see the README).
