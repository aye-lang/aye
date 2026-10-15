#!/bin/sh
# Installs the Aye compiler from a GitHub release (card A6):
#
#     curl -fsSL https://ayelang.dev/install.sh | sh
#
# Detects the OS and CPU, downloads aye-<version>-<target>.tar.gz and
# SHA256SUMS from https://github.com/aye-lang/aye/releases, checks the
# tarball's checksum, and unpacks it under $AYE_HOME (default ~/.aye):
#
#     ~/.aye/aye-<version>-<target>/   the release, as it unpacks
#     ~/.aye/bin/aye                   symlink to its bin/aye
#     ~/.aye/lib/aye                   symlink to its lib/aye (std's sources)
#
# Running it again installs the latest release and removes the old one.
# It never uses sudo and never edits shell profiles: it prints the line to
# add. Aye compiles through C, so it also checks for clang.
#
# Environment:
#   AYE_VERSION   a release tag to install (v0.1.0), instead of the latest
#   AYE_HOME      where to install (default $HOME/.aye)
#   AYE_UNAME_S, AYE_UNAME_M   override `uname -s` / `uname -m` (for tests)
set -eu

repo=aye-lang/aye

say() { printf 'aye-install: %s\n' "$*"; }
die() { printf 'aye-install: error: %s\n' "$*" >&2; exit 1; }

from_source="Build from source instead (Rust and clang): https://github.com/aye-lang/aye/blob/main/docs/install.md#from-source"

# The release target for this machine.
os=${AYE_UNAME_S:-$(uname -s)}
arch=${AYE_UNAME_M:-$(uname -m)}
case $os in
    Darwin) os_part=apple-darwin ;;
    Linux) os_part=unknown-linux-gnu ;;
    MINGW* | MSYS* | CYGWIN* | Windows*)
        die "Windows is not supported. Aye runs on macOS and Linux; on Windows, use WSL (https://learn.microsoft.com/windows/wsl/install) and run this script inside it." ;;
    *) die "unsupported OS '$os': release binaries exist for macOS and Linux only. $from_source" ;;
esac
case $arch in
    x86_64 | amd64) arch_part=x86_64 ;;
    arm64 | aarch64) arch_part=aarch64 ;;
    *) die "unsupported CPU '$arch' on $os: release binaries exist for x86_64 and arm64 only. $from_source" ;;
esac
target=$arch_part-$os_part

# The Linux binaries link glibc 2.39 (Ubuntu 24.04).
if [ "$os" = Linux ] && [ -z "${AYE_UNAME_S:-}" ]; then
    glibc=$(getconf GNU_LIBC_VERSION 2>/dev/null | sed -n 's/^glibc \([0-9]*\.[0-9]*\).*/\1/p') || glibc=
    if [ -z "$glibc" ]; then
        say "warning: could not find glibc (musl?); the Linux binaries need glibc 2.39 or newer."
    else
        major=${glibc%%.*}
        minor=${glibc#*.}
        if [ "$major" -lt 2 ] || { [ "$major" -eq 2 ] && [ "$minor" -lt 39 ]; }; then
            die "this system has glibc $glibc; the Linux binaries need glibc 2.39 or newer (Ubuntu 24.04, Debian 13, Fedora 40). $from_source"
        fi
    fi
fi

# Downloads: curl, else wget.
if command -v curl >/dev/null 2>&1; then
    fetch() { curl --proto '=https' --tlsv1.2 -fsSL "$1"; }
    fetch_to() { curl --proto '=https' --tlsv1.2 -fsSL -o "$2" "$1"; }
elif command -v wget >/dev/null 2>&1; then
    fetch() { wget -qO- "$1"; }
    fetch_to() { wget -qO "$2" "$1"; }
else
    die "need curl or wget to download the release"
fi

if command -v sha256sum >/dev/null 2>&1; then
    sha256() { sha256sum "$1" | cut -d' ' -f1; }
elif command -v shasum >/dev/null 2>&1; then
    sha256() { shasum -a 256 "$1" | cut -d' ' -f1; }
else
    die "need sha256sum or shasum to check the download"
fi

# The release tag: $AYE_VERSION, else the latest release (GitHub API, then
# the /releases/latest redirect if the API is rate-limited).
if [ -n "${AYE_VERSION:-}" ]; then
    tag=v${AYE_VERSION#v}
else
    tag=$(fetch "https://api.github.com/repos/$repo/releases/latest" 2>/dev/null |
        sed -n 's/^ *"tag_name": *"\([^"]*\)".*/\1/p' | head -n 1) || tag=
    if [ -z "$tag" ] && command -v curl >/dev/null 2>&1; then
        tag=$(curl -fsSLI -o /dev/null -w '%{url_effective}' "https://github.com/$repo/releases/latest" 2>/dev/null |
            sed -n 's|.*/releases/tag/\(v[^/]*\)$|\1|p') || tag=
    fi
    [ -n "$tag" ] || die "could not find the latest release of $repo (set AYE_VERSION=v0.1.0 to pick one)"
fi
version=${tag#v}
name=aye-$version-$target
base=https://github.com/$repo/releases/download/$tag

home=${AYE_HOME:-${HOME:?HOME is not set; set AYE_HOME}/.aye}
mkdir -p "$home"
home=$(cd "$home" && pwd)

tmp=$(mktemp -d "$home/.install.XXXXXX")
trap 'rm -rf "$tmp"' EXIT
trap 'exit 1' HUP INT PIPE TERM

say "installing aye $version ($target) into $home"
fetch_to "$base/$name.tar.gz" "$tmp/$name.tar.gz" ||
    die "could not download $base/$name.tar.gz (is $tag a release with a $target build?)"
fetch_to "$base/SHA256SUMS" "$tmp/SHA256SUMS" || die "could not download $base/SHA256SUMS"

want=$(awk -v f="$name.tar.gz" '$2 == f || $2 == "*" f { print $1 }' "$tmp/SHA256SUMS")
[ -n "$want" ] || die "SHA256SUMS of $tag has no line for $name.tar.gz"
got=$(sha256 "$tmp/$name.tar.gz")
[ "$got" = "$want" ] || die "checksum mismatch for $name.tar.gz: got $got, want $want"
say "checksum ok"

mkdir "$tmp/unpack"
tar -xzf "$tmp/$name.tar.gz" -C "$tmp/unpack"
[ -x "$tmp/unpack/$name/bin/aye" ] || die "$name.tar.gz has no $name/bin/aye"

# Swap in the new tree, point the symlinks at it, then remove old releases.
rm -rf "${home:?}/$name"
mv "$tmp/unpack/$name" "$home/$name"
mkdir -p "$home/bin" "$home/lib"
ln -sfn "../$name/bin/aye" "$home/bin/aye"
ln -sfn "../$name/lib/aye" "$home/lib/aye"
for old in "$home"/aye-*-*; do
    [ -d "$old" ] && [ "$old" != "$home/$name" ] && rm -rf "$old"
done

"$home/bin/aye" --version >/dev/null 2>&1 || die "$home/bin/aye does not run on this system. $from_source"
say "installed $("$home/bin/aye" --version)"

# PATH: say what to add, do not edit profiles.
case :${PATH}: in
    *":$home/bin:"*) say "$home/bin is already on PATH" ;;
    *)
        case ${SHELL:-} in
            */zsh) profile="${HOME:-}/.zshrc" ;;
            */bash) profile="${HOME:-}/.bashrc" ;;
            */fish) profile="${HOME:-}/.config/fish/config.fish" ;;
            *) profile="${HOME:-}/.profile" ;;
        esac
        echo
        echo "Add aye to PATH: put this line in $profile, then open a new shell:"
        echo
        case ${SHELL:-} in
            */fish) echo "    fish_add_path $home/bin" ;;
            *) echo "    export PATH=\"$home/bin:\$PATH\"" ;;
        esac
        ;;
esac

# clang compiles the C that `aye build`, `run` and `test` emit.
if ! command -v clang >/dev/null 2>&1; then
    echo
    echo "aye needs clang on PATH to build and run programs (aye check, fmt, doc and spec work without it). Install it with:"
    echo
    if [ "$os" = Darwin ]; then
        echo "    xcode-select --install"
    else
        id=
        # shellcheck disable=SC1091
        [ -r /etc/os-release ] && id=$(. /etc/os-release && echo "${ID:-} ${ID_LIKE:-}")
        case " $id " in
            *" ubuntu "* | *" debian "*) echo "    sudo apt install clang" ;;
            *" fedora "* | *" rhel "*) echo "    sudo dnf install clang" ;;
            *" arch "*) echo "    sudo pacman -S clang" ;;
            *" opensuse"* | *" suse "*) echo "    sudo zypper install clang" ;;
            *) echo "    your distribution's clang package (e.g. sudo apt install clang)" ;;
        esac
    fi
fi

echo
echo "Try it:"
echo
printf '%s\n' "    printf 'pub fn main() {\\n    @println(\"hello\")\\n}\\n' > hello.aye"
echo "    aye run hello.aye"
