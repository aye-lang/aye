# Agent workflow

The internal workflow of the agents and maintainers who work on this repo,
moved here from `CONTRIBUTING.md`. Outside contributors want
[`CONTRIBUTING.md`](../../CONTRIBUTING.md).

Build: Rust (cargo) and `clang` on `PATH` are required (see [`docs/cli.md`](../cli.md)
for optional libraries such as libclang, libcurl and SDL2).

```sh
cargo build --release
cargo test                         # all suites
cargo test --test golden           # one suite: golden, spec, fmt
sh scripts/downstream.sh           # build and test dweller against this compiler
```

Workflow: work is split into lanes, phases and steps in
[`docs/plan.md`](../plan.md). One change is one step: one goal with one
verify command, done on a short-lived branch (a git worktree off `main`),
committed as a small reviewable series, then merged into `main`. Before you
change the compiler or std, read
[`docs/practices/`](README.md).

Contributions are dual licensed under MIT OR Apache-2.0 (see the README).
