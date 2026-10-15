# Installing Aye

## From a release

Each release on [GitHub](https://github.com/aye-lang/aye/releases) has a
tarball per platform and a `SHA256SUMS` file:

| Platform | Tarball |
|---|---|
| macOS, Apple silicon | `aye-<version>-aarch64-apple-darwin.tar.gz` |
| macOS, Intel | `aye-<version>-x86_64-apple-darwin.tar.gz` |
| Linux x86_64 (glibc 2.39+, e.g. Ubuntu 24.04) | `aye-<version>-x86_64-unknown-linux-gnu.tar.gz` |
| Linux arm64 (glibc 2.39+) | `aye-<version>-aarch64-unknown-linux-gnu.tar.gz` |

```sh
shasum -a 256 -c SHA256SUMS --ignore-missing
tar -xzf aye-<version>-<target>.tar.gz
export PATH="$PWD/aye-<version>-<target>/bin:$PATH"
aye --version
```

The tarball unpacks to one directory that works wherever it is put:

```
aye-<version>-<target>/
  bin/aye          the compiler
  lib/aye/std/     std's sources (to read; `aye lsp` jumps into them)
  README.md  LICENSE-MIT  LICENSE-APACHE  INSTALL.txt
```

std is embedded in `bin/aye` and the C runtime is emitted into each
program's C, so `bin/aye` alone compiles programs; `lib/aye/std` is there
for reading and for the language server's go-to-definition.

On an older Linux (glibc before 2.39) build from source (below).

## clang is required

`aye build`, `run` and `test` compile the C they emit with `clang`, which
must be on `PATH` (`aye check`, `fmt`, `doc` and `spec` do not need it):

| System | Install |
|---|---|
| macOS | `xcode-select --install` (Apple clang from the Command Line Tools) |
| Ubuntu, Debian | `sudo apt install clang` |
| Fedora | `sudo dnf install clang` |
| Arch | `sudo pacman -S clang` |

`@cimport` also needs libclang, which comes with the Command Line Tools on
macOS and with the `clang` (or `libclang-dev`) package on Linux. The
WebAssembly targets need a WASI toolchain; see the README.

## Check it

```sh
cat > hello.aye <<'AYE'
pub fn main() {
    @println("hello")
}
AYE
aye run hello.aye     # hello
```

## From source

With Rust (cargo, edition 2024: Rust 1.85 or newer) and clang:

```sh
git clone https://github.com/aye-lang/aye
cd aye
cargo build --release
./target/release/aye --version
```

`aye --version` prints `aye <version> (<commit> <date>)`; a build outside a
git checkout prints `aye <version>`.

## Making a release

Maintainers: set the version in `Cargo.toml`, add its section to
`CHANGELOG.md`, merge, then push an annotated tag `v<version>` from main.
`.github/workflows/release.yml` builds the four tarballs on their own
runners, smoke-tests each unpacked (`scripts/release/smoke.sh`), and
publishes the release. To check a tarball by hand on Linux or macOS:

```sh
cargo build --release
sh scripts/release/package.sh x86_64-unknown-linux-gnu target/release/aye dist
sh scripts/release/smoke.sh dist/aye-*-x86_64-unknown-linux-gnu.tar.gz
```
