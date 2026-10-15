#!/bin/sh
# The posts example app in the dweller browser (F5.7): builds the posts app
# for production, starts the binary and looks at it with the browser.
#
#     sh scripts/framework-posts.sh
#
# Needs target/downstream/dweller (run scripts/downstream.sh first), else
# it prints `skipped: no dweller clone; run scripts/downstream.sh first`
# and exits 0. With AYE=<this checkout>/target/release/aye (override with
# AYE) it
#
#     aye run framework/build -- <copy of framework/examples/posts>
#     aye build browser                       (in the dweller clone)
#     dweller dump --md http://127.0.0.1:<port>/
#     dweller dump --md http://127.0.0.1:<port>/posts/hello
#     dweller dump --a11y http://127.0.0.1:<port>/posts/hello
#     dweller agent                       (navigate, type, type, click, snapshot)
#
# and checks the dumps: every post title in the Markdown of `/`, the title in
# the Markdown of the post and `0 comments` in its accessibility tree (the
# Markdown is the first `article`, and the comments come after it), then
# `1 comments` and the comment in the snapshot after the form is sent. It prints `dump / ok`, `dump post ok` and `form ok`, and exits 1 on
# the first mismatch, naming the step. The work is in target/framework-posts.
# The island is not run (the browser has no WebAssembly): the page shows the
# server's `Likes` text. This script is not part of `cargo test`.
set -eu
cd "$(dirname "$0")/.."
root=$(pwd)
AYE=${AYE:-$root/target/release/aye}
export AYE
clone=$root/target/downstream/dweller
work=$root/target/framework-posts

fail() {
    echo "framework-posts: FAIL $*" >&2
    exit 1
}

if [ ! -d "$clone/browser" ]; then
    echo "skipped: no dweller clone; run scripts/downstream.sh first"
    exit 0
fi
if [ ! -x "$AYE" ]; then
    echo "framework-posts: $AYE not found; run cargo build --release first" >&2
    exit 1
fi

rm -rf "$work"
mkdir -p "$work"
cp -R framework/examples/posts "$work/app"
cp "$work/app/data/posts.json" "$work/posts.json"

"$AYE" run framework/build -- "$work/app" -o "$work/posts" > "$work/build.log" 2>&1 ||
    { tail -n 30 "$work/build.log" >&2; fail "web build (see $work/build.log)"; }
(cd "$clone" && "$AYE" build browser -o "$work/dweller") > "$work/dweller-build.log" 2>&1 ||
    { tail -n 30 "$work/dweller-build.log" >&2; fail "aye build browser (see $work/dweller-build.log)"; }
ak=$work/dweller

(cd "$work" && exec ./posts --port 0 --posts "$work/posts.json") > "$work/server.out" 2>&1 &
server=$!
trap 'kill "$server" 2> /dev/null || true' EXIT
port=
n=0
while [ -z "$port" ] && [ "$n" -lt 100 ]; do
    port=$(sed -n 's/^listening on 127\.0\.0\.1:\([0-9]*\)$/\1/p' "$work/server.out")
    n=$((n + 1))
    [ -n "$port" ] || sleep 0.1
done
[ -n "$port" ] || fail "the posts binary did not print its port (see $work/server.out)"
base=http://127.0.0.1:$port

# has <file> <text> <step>: the file holds the text.
has() {
    grep -q -F -- "$2" "$1" || { echo "--- $1" >&2; cat "$1" >&2; fail "$3: no '$2'"; }
}

"$ak" dump --md "$base/" > "$work/home.md" 2> "$work/home.err" ||
    fail "dump /: dweller failed: $(cat "$work/home.err")"
for title in "Hello, world" "Less < More" "Posts live in a file"; do
    has "$work/home.md" "$title" "dump /"
done
echo "dump / ok"

"$ak" dump --md "$base/posts/hello" > "$work/post.md" 2> "$work/post.err" ||
    fail "dump post: dweller failed: $(cat "$work/post.err")"
has "$work/post.md" "Hello, world" "dump post"
# `dump --md` reads the first `article` only, and the comments are after it
# in the page: their count is in the accessibility tree.
"$ak" dump --a11y "$base/posts/hello" > "$work/post.a11y" 2> "$work/post.err" ||
    fail "dump post: dweller failed: $(cat "$work/post.err")"
has "$work/post.a11y" "0 comments" "dump post"
echo "dump post ok"

# The refs of the form: the first session only looks, the second one acts on
# the refs that a fresh load of the same page gives.
printf '{"id":1,"cmd":"navigate","url":"%s/posts/hello"}\n' "$base" |
    "$ak" agent > "$work/look.json" 2> "$work/look.err" ||
    fail "form: dweller agent failed: $(cat "$work/look.err")"
ref() {
    sed -n "s/.*$1 \\\\\"$2\\\\\" \\[\\(e[0-9]*\\)\\].*/\\1/p" "$work/look.json" | head -n 1
}
name=$(ref textbox Name)
body=$(ref textbox Comment)
submit=$(ref button Post)
[ -n "$name" ] && [ -n "$body" ] && [ -n "$submit" ] ||
    { cat "$work/look.json" >&2; fail "form: no name, comment or Post ref in the snapshot"; }

{
    printf '{"id":1,"cmd":"navigate","url":"%s/posts/hello"}\n' "$base"
    printf '{"id":2,"cmd":"type","ref":"%s","text":"Ann"}\n' "$name"
    printf '{"id":3,"cmd":"type","ref":"%s","text":"Nice post"}\n' "$body"
    printf '{"id":4,"cmd":"click","ref":"%s"}\n' "$submit"
    printf '{"id":5,"cmd":"snapshot"}\n'
} | "$ak" agent > "$work/form.json" 2> "$work/form.err" ||
    fail "form: dweller agent failed: $(cat "$work/form.err")"
if grep -q '"ok":false' "$work/form.json"; then
    cat "$work/form.json" >&2
    fail "form: a step failed"
fi
has "$work/form.json" "1 comments" "form"
has "$work/form.json" "Nice post" "form"
echo "form ok"
