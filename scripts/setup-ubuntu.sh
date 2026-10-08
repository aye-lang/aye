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
    clang lld libclang-dev \
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

$SUDO apt-get clean
$SUDO rm -rf /var/lib/apt/lists/*
echo "setup-ubuntu: done"
