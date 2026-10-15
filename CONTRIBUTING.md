# Contributing

Thanks for looking. Questions and ideas go to
[Discussions](https://github.com/aye-lang/aye/discussions); bugs go to
[issues](https://github.com/aye-lang/aye/issues) (the templates ask for
`aye --version`, your OS and the smallest program). Please read the
[Code of Conduct](CODE_OF_CONDUCT.md). Report security problems privately
([SECURITY.md](SECURITY.md)). Contributions are dual licensed under
MIT OR Apache-2.0, like the rest of the repo.

## Build

You need Rust (cargo) and `clang` on `PATH`. Optional: libclang for
`@cimport`, libcurl and SDL2 (see [`docs/cli.md`](docs/cli.md)). On Ubuntu
24.04, [`scripts/setup-ubuntu.sh`](scripts/setup-ubuntu.sh) installs
everything the tests use.

```sh
cargo build --release
./target/release/aye run tests/cases/hello.aye
```

## Test

```sh
cargo test --release                    # everything; slow (about 10 minutes)
cargo test --release --test golden      # run-and-compare programs
./target/release/aye test <file.aye>    # one program's test blocks
cargo test --release --test spec        # docs/spec.md stays correct
cargo test --release --test fmt         # every .aye file is formatted
cargo test --release --test conformance # language/std conformance programs
cargo test --release --lib              # compiler unit tests
```

Run the suites your change touches; CI runs the rest. Run `--test fmt`
whenever you add or change a `.aye` file, and `cargo fmt` for Rust.

## Style

- Rust: `cargo fmt`; match the surrounding code. The compiler has no
  dependencies; keep it that way.
- Aye: `aye fmt <path>` formats in place. Read
  [`docs/practices/aye.md`](docs/practices/aye.md) for idioms (errors,
  optionals, allocators).
- Keep comments and docs short and factual. Std functions that are `pub`
  need `///` comments.

## How errors, the spec and goldens work

- **Errors** have a stable code (`E0032`). The table, with the text
  `aye explain E0032` prints, is [`src/codes.rs`](src/codes.rs). A new
  diagnostic gets a new code with a wrong and a fixed example, and a case
  in `tests/errors/`.
- **Spec:** [`docs/spec.md`](docs/spec.md) is what `aye spec` prints.
  `tests/spec.rs` compiles every ```` ```aye ```` block, checks that every
  ```` ```aye-error ```` block is rejected, and checks that each std module
  is indexed. Change the language, change the spec in the same PR.
- **Goldens:** a program plus its expected output in
  [`tests/`](tests). `tests/cases/X.aye` is run in debug, safe and fast
  mode and its stdout compared with `X.out` (`X.panic` and `X.stderr` for
  programs that abort or return an error). `tests/errors/X.aye` must fail
  with exactly the diagnostics in `X.err`. `tests/tests/X.aye` is run with
  `aye test` and compared with `X.out`. `tests/fix/` covers `aye fix`.
  The full list of kinds is the comment at the top of
  [`tests/golden.rs`](tests/golden.rs). To add a case, write the `.aye`,
  check the output by hand, and save it as the expected file.

## Sending a change

1. Open an issue or Discussion first for anything larger than a bug fix,
   so you do not build something that will be declined (many things are, on
   purpose: see [`docs/design.md`](docs/design.md) and
   [`docs/decisions.md`](docs/decisions.md)).
2. Branch from `main`, keep one change per PR, commit in small steps.
3. Add a test that fails without your change. Run the suites above.
4. Update docs (`docs/spec.md`, `docs/cli.md`) and add a line under
   Unreleased in [`CHANGELOG.md`](CHANGELOG.md) for user-visible changes.
5. Open the PR and fill in the template. CI must be green.

Good first PRs: a clearer error message with its golden, a missing `///`
comment, a docs fix, a small std function with `test` blocks.

The agents that do most of the coding here follow a longer workflow, in
[`docs/practices/`](docs/practices/README.md); you do not need it.
