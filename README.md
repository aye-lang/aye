# Aye

**Aye is a small systems language without a garbage collector, built so
that an AI agent can learn it from one command and write working native
code.**

Memory is managed through explicit `Allocator` parameters and
`defer`/`errdefer`; errors are values; there are no exceptions, no macros
and no borrow checker. The syntax is Go-like. The compiler emits C and
builds it with clang into one small native binary.

Install: `curl -fsSL https://ayelang.dev/install.sh | sh` (macOS and
Linux; Aye also needs clang, see [Install](#install)).

```aye
import std.heap.General
import std.list.List
import std.log
import std.text

// Sum the numbers in a comma-separated line, skipping bad ones.
pub fn main() !void {
    var gpa = General.init()
    defer gpa.deinit() // debug builds report any leak here
    var nums = List(i64).init(gpa.allocator())
    defer nums.deinit()

    var parts = text.split("3,4,oops,5", ",")
    while parts.next() |part| {
        let n = text.parseInt(part) catch |e| {
            log.err("skipped {}: {}", part, e)
            continue
        }
        try nums.append(n)
    }
    var sum: i64 = 0
    for n in nums.items() {
        sum += n
    }
    @println(sum)
}
```

```
$ aye run sum.aye
error: skipped oops: Invalid
12
```

## Why Aye

- **Agents learn it cold.** `aye spec` prints the core language in
  under 30k tokens, and every example in it is compiled by the tests.
  Fresh Sonnet agents given only `aye spec` and the checker passed 15 of
  15 runs of five tasks (grep, line counts, an HTTP server, JSON totals,
  word frequencies), with a median of 11 turns and 1 `aye check` call,
  for 1.85 USD in all. Source: [`docs/aye-eval.md`](docs/aye-eval.md)
  (2026-10-11; rerun it with [`scripts/eval.py`](scripts/eval.py)).
- **It scales to a browser.** [Dweller](https://github.com/aye-lang/dweller)
  is a web browser written in Aye (HTML, CSS, a JavaScript engine,
  layout, painting; SDL2, FreeType and libcurl called through
  `@cimport`), about 112k lines. CI builds and tests it against every
  compiler change on main (job `downstream` in
  [`.github/workflows/ci.yml`](.github/workflows/ci.yml)).
- **Small, fast binaries.** A 43.6 KB Aye hello HTTP server served 146k
  req/s under `wrk`, against 70k for Go `net/http` (an 8.2 MB binary) and
  16.6k for Node 22, on one 4-core VM. Read the caveats in
  [`docs/aye-bench.md`](docs/aye-bench.md) before quoting it: it is a
  hello-world `GET /`, the client shares the VM with the server, Go
  measured much higher on another VM shape, and the wrk script is not
  committed yet.

Errors are built for agents and people alike: every error has a stable
code (`aye explain E0032`), many come with a fix-it that `aye fix`
applies, and `aye check --json` gives all of it as JSON.

## Install

Aye runs on macOS and Linux, arm64 and x86_64 (and builds for
`wasm32-wasip1`). Windows is not supported. Install the latest release:

```sh
curl -fsSL https://ayelang.dev/install.sh | sh
```

The [script](site/public/install.sh) downloads the release tarball for
your platform from [GitHub](https://github.com/aye-lang/aye/releases),
checks it against `SHA256SUMS`, unpacks it into `~/.aye` (`AYE_HOME` picks
another directory, `AYE_VERSION=v0.1.0` a release), and prints the line to
add to your shell profile, `export PATH="$HOME/.aye/bin:$PATH"`. It never
uses sudo; run it again to upgrade. The Linux binaries need glibc 2.39 or
newer (Ubuntu 24.04).

Every Aye build also needs `clang` on `PATH`:

- **macOS:** `xcode-select --install` (clang comes with the Command Line
  Tools).
- **Ubuntu / Debian:** `sudo apt install clang`.
- **Fedora:** `sudo dnf install clang`.

**Build from source** (needs [Rust](https://rustup.rs) 1.85 or newer, and
clang; also works on an older Linux):

```sh
cargo install --git https://github.com/aye-lang/aye aye
```

[`docs/install.md`](docs/install.md) has the details.

Optional: libclang for `@cimport` of C headers (it comes with the macOS
Command Line Tools; on Linux, the `libclang` package of your distro), and
`pkg-config` for linking C libraries. On Ubuntu 24.04,
[`scripts/setup-ubuntu.sh`](scripts/setup-ubuntu.sh) installs everything
the test suite uses.

## Hello world

```sh
printf 'pub fn main() {\n    @println("hello, world")\n}\n' > hello.aye
aye run hello.aye
aye build --mode fast hello.aye && ./hello
```

Next: `aye spec` prints the language reference, `aye check` and `aye fix`
find and fix errors, `aye test` runs `test` blocks, `aye fmt` formats, and
`aye lsp` is a language server for any editor.

## Learn more

- **Language reference:** [`docs/spec.md`](docs/spec.md) (also `aye spec`).
- **Standard library:** [`std/`](std); `aye doc --signatures std` lists
  every function, `aye doc --html std` writes HTML pages.
- **Commands, test runner and supported language:**
  [`docs/cli.md`](docs/cli.md).
- **Web framework:** [`framework/`](framework), with examples in
  [`framework/examples/`](framework/examples) and its design in
  [`docs/framework.md`](docs/framework.md).
- **Design:** [`docs/design.md`](docs/design.md) and the decision log
  [`docs/decisions.md`](docs/decisions.md).
- **Website:** [ayelang.dev](https://ayelang.dev).
- **Questions and ideas:**
  [GitHub Discussions](https://github.com/aye-lang/aye/discussions); bugs
  go to [issues](https://github.com/aye-lang/aye/issues). See
  [`CONTRIBUTING.md`](CONTRIBUTING.md) to send a change.

A tour of the language is being written; it will be linked here.

## Status

Aye is pre-1.0: the language and std still change, and there is no
stability promise yet. There are no tagged releases yet. The roadmap is
[`docs/plan.md`](docs/plan.md).

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT), at your option. Third-party files keep their own
notices. The Octane benchmark ports and jQuery that the dweller browser tests use live in
[aye-lang/dweller](https://github.com/aye-lang/dweller), with their notices.

Unless you explicitly state otherwise, any contribution you submit for
inclusion in this project, as defined in the Apache-2.0 license, is dual
licensed as above, without any additional terms or conditions.
