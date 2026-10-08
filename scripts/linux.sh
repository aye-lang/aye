#!/bin/sh
# Runs a command in an Ubuntu 24.04 container (Apple `container` CLI) with
# this checkout mounted at /work. With no command, opens a shell.
#
#     scripts/linux.sh cargo test --release
#     scripts/linux.sh sh -c 'cargo build --release && target/release/aye check browser'
#     LINUX_ARCH=amd64 scripts/linux.sh true     # other architecture (slow, emulated)
#
# The image `aye-dev:ubuntu24[-amd64]` is built from the Containerfile on first
# use (`scripts/setup-ubuntu.sh` runs inside). Linux build output lives in the
# volume `aye-linux-<checkout>-target`, mounted over /work/target, so it never
# mixes with the macOS target/ of the checkout. `LINUX_CPUS` (default 4) and
# `LINUX_MEMORY` (default 8G) size the container. Remove the volume with
# `container volume rm aye-linux-<checkout>-target`.
set -eu
cd "$(dirname "$0")/.."
root=$(pwd)
base=$(basename "$root")
arch=${LINUX_ARCH:-arm64}
image=aye-dev:ubuntu24
[ "$arch" = arm64 ] || image=$image-$arch
name=aye-linux-$base
vol=$name-target

if ! container image inspect "$image" >/dev/null 2>&1; then
    container build --arch "$arch" -t "$image" -f Containerfile .
fi
container volume inspect "$vol" >/dev/null 2>&1 || container volume create "$vol" >/dev/null
container rm -f "$name" >/dev/null 2>&1 || true

if [ $# -eq 0 ]; then
    set -- bash
fi
tty=
[ -t 0 ] && [ -t 1 ] && tty=-it
exec container run $tty --rm --name "$name" --arch "$arch" \
    --cpus "${LINUX_CPUS:-4}" --memory "${LINUX_MEMORY:-8G}" \
    --mount "type=bind,source=$root,target=/work" \
    --volume "$vol:/work/target" \
    -w /work "$image" "$@"
