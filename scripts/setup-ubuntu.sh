#!/bin/sh
# Installs everything the full Aye suite needs on Ubuntu 24.04 (arm64 or
# amd64): `cargo build --release`, `aye check browser`, `cargo test
# --release` and `browser/build.sh`. Run as root or with sudo access.
# Used by the Containerfile (scripts/linux.sh) and by the cloud VM.
#
#     sh scripts/setup-ubuntu.sh
#
# Rust comes from rustup into /usr/local (RUSTUP_HOME, CARGO_HOME), with
# the tools linked in /usr/local/bin, so every user finds them.
set -eu

if [ "$(id -u)" -eq 0 ]; then
    SUDO=
else
    SUDO=sudo
fi

export DEBIAN_FRONTEND=noninteractive
$SUDO apt-get update
$SUDO apt-get install -y --no-install-recommends \
    ca-certificates curl git gnupg build-essential pkg-config \
    clang lld libclang-dev libclang-rt-18-dev \
    libcurl4-openssl-dev libfreetype-dev libsdl2-dev \
    fonts-dejavu-core python3 apache2-utils golang-go

# Node 22 (apt's nodejs is older).
if ! command -v node >/dev/null 2>&1 || ! node --version | grep -q '^v22\.'; then
    curl -fsSL https://deb.nodesource.com/setup_22.x -o /tmp/nodesource.sh
    $SUDO sh /tmp/nodesource.sh
    rm -f /tmp/nodesource.sh
    $SUDO apt-get install -y --no-install-recommends nodejs
fi

# Rust from rustup (Cargo.toml needs edition 2024).
if ! command -v cargo >/dev/null 2>&1; then
    export RUSTUP_HOME=/usr/local/rustup CARGO_HOME=/usr/local/cargo
    curl -fsSL https://sh.rustup.rs -o /tmp/rustup-init.sh
    $SUDO env RUSTUP_HOME="$RUSTUP_HOME" CARGO_HOME="$CARGO_HOME" \
        sh /tmp/rustup-init.sh -y --profile minimal --no-modify-path
    rm -f /tmp/rustup-init.sh
    for t in cargo rustc rustup; do
        $SUDO ln -sf "$CARGO_HOME/bin/$t" "/usr/local/bin/$t"
    done
fi

# wasi-sdk (clang with the wasm32 backend, WASI sysroot and resource
# directory) for the wasm32-wasip1 target. Apple's clang has no wasm
# backend and Ubuntu's clang ships no WASI libc. The env file below is
# read by login shells; `. /etc/profile.d/aye-wasi.sh` in others.
WASI_SDK_VERSION=34
case "$(uname -m)" in
    x86_64) WASI_SDK_ARCH=x86_64 ;;
    aarch64|arm64) WASI_SDK_ARCH=arm64 ;;
    *) echo "setup-ubuntu: no wasi-sdk for $(uname -m)" >&2; exit 1 ;;
esac
if [ ! -x /opt/wasi-sdk/bin/clang ]; then
    WASI_TMP=$(mktemp -d)
    curl -fsSL -o "$WASI_TMP/wasi-sdk.tar.gz" \
        "https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-$WASI_SDK_VERSION/wasi-sdk-$WASI_SDK_VERSION.0-$WASI_SDK_ARCH-linux.tar.gz"
    $SUDO mkdir -p /opt/wasi-sdk
    $SUDO tar xzf "$WASI_TMP/wasi-sdk.tar.gz" -C /opt/wasi-sdk --strip-components=1
    rm -rf "$WASI_TMP"
fi
WASI_RES=$(ls -d /opt/wasi-sdk/lib/clang/* | head -n 1)
printf 'export AYE_WASI_CLANG=/opt/wasi-sdk/bin/clang\nexport AYE_WASI_SYSROOT=/opt/wasi-sdk/share/wasi-sysroot\nexport AYE_WASI_RESOURCE_DIR=%s\n' "$WASI_RES" \
    | $SUDO tee /etc/profile.d/aye-wasi.sh >/dev/null

$SUDO apt-get clean
$SUDO rm -rf /var/lib/apt/lists/*
echo "setup-ubuntu: done"
