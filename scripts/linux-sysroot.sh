#!/bin/sh
# Copies a Linux sysroot (glibc, gcc's crt files and libgcc, /usr/include)
# out of an Ubuntu 24.04 container (Apple `container` CLI), for linking
# Linux executables on macOS:
#
#     scripts/linux-sysroot.sh arm64 target/sysroot-arm64     # or amd64
#     AYE_SYSROOT=$PWD/target/sysroot-arm64 aye build --target aarch64-linux hello.aye
set -eu
arch=${1:?usage: scripts/linux-sysroot.sh arm64|amd64 <dir>}
out=${2:?usage: scripts/linux-sysroot.sh arm64|amd64 <dir>}
case $arch in
arm64) triple=aarch64-linux-gnu ;;
amd64) triple=x86_64-linux-gnu ;;
*) echo "unknown arch $arch (arm64 or amd64)" >&2; exit 2 ;;
esac
mkdir -p "$out"
out=$(cd "$out" && pwd)
# `h`: follow symlinks, so the dynamic loaders (/usr/lib/ld-linux-*,
# /lib64/ld-linux-*) are files; the package's crt files and libraries come
# from libc6-dev and libgcc-13-dev.
container run --rm --arch "$arch" -v "$out:/out" ubuntu:24.04 sh -c "
    apt-get update -qq >/dev/null && apt-get install -y -qq libc6-dev libgcc-13-dev >/dev/null
    cd / && tar chf - usr/include usr/lib/$triple usr/lib/gcc/$triple usr/lib/ld-linux-* \$( [ -d lib64 ] && echo lib64 ) 2>/dev/null | tar xf - -C /out
"
[ -e "$out/lib" ] || ln -s usr/lib "$out/lib"
echo "$out"
