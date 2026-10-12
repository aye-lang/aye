#!/bin/sh
# Installs everything the full Aye suite needs on Ubuntu 24.04 (arm64 or
# amd64): `cargo build --release`, `cargo test --release` and
# `scripts/downstream.sh` (the anyknown browser; the libcurl, FreeType and
# SDL2 packages below are for it). Run as root or with sudo access.
# Used by the Containerfile (scripts/linux.sh) and by the cloud VM.
#
#     sh scripts/setup-ubuntu.sh
#
# Rust comes from rustup into /usr/local (RUSTUP_HOME, CARGO_HOME), with
# the tools linked in /usr/local/bin, so every user finds them.
#
# The WASI toolchain (wasm32-wasip1 goldens and conformance) is the
# wasi-sdk release in /opt/wasi-sdk: its clang, wasm-ld, sysroot and
# wasm32 builtins, the same wasi-libc as Homebrew's on macOS. apt's
# wasi-libc is a 2023 snapshot (no inet_ntoa, no single-threaded pthread
# stubs), and apt's lld 18 cannot link wasi-sdk's libc. src/driver.rs
# finds it through AYE_WASI_CLANG, AYE_WASI_SYSROOT and
# AYE_WASI_RESOURCE_DIR: the Containerfile sets them; on a VM
# /etc/profile.d/aye-wasi.sh does, for login shells.

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
    libcurl4-openssl-dev libfreetype-dev libsdl2-dev libsqlite3-dev \
    fonts-dejavu-core python3 apache2-utils golang-go

# wasi-sdk (see the header). /opt/wasi-sdk/resource is its clang
# resource directory under a name without the clang version.
WASI_SDK=34
if [ ! -x /opt/wasi-sdk/bin/clang ]; then
    case "$(uname -m)" in aarch64) arch=arm64 ;; *) arch=x86_64 ;; esac
    curl -fsSL "https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-$WASI_SDK/wasi-sdk-$WASI_SDK.0-$arch-linux.tar.gz" \
        -o /tmp/wasi-sdk.tar.gz
    $SUDO rm -rf /opt/wasi-sdk "/opt/wasi-sdk-$WASI_SDK.0-$arch-linux"
    $SUDO tar -xzf /tmp/wasi-sdk.tar.gz -C /opt
    $SUDO mv "/opt/wasi-sdk-$WASI_SDK.0-$arch-linux" /opt/wasi-sdk
    rm -f /tmp/wasi-sdk.tar.gz
    $SUDO ln -sfn "$(ls -d /opt/wasi-sdk/lib/clang/* | head -n 1)" /opt/wasi-sdk/resource
fi
printf 'export AYE_WASI_CLANG=/opt/wasi-sdk/bin/clang\nexport AYE_WASI_SYSROOT=/opt/wasi-sdk/share/wasi-sysroot\nexport AYE_WASI_RESOURCE_DIR=/opt/wasi-sdk/resource\n' \
    | $SUDO tee /etc/profile.d/aye-wasi.sh >/dev/null

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

$SUDO apt-get clean
$SUDO rm -rf /var/lib/apt/lists/*
echo "setup-ubuntu: done"
