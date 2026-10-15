# Releases

## Each release updates the changelog

[`CHANGELOG.md`](../../CHANGELOG.md) follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Every
user-visible change (language, std, a command, the framework, a release
artifact) adds a line under `## [Unreleased]` in the PR that makes it.

To release version X.Y.Z:

1. Rename `## [Unreleased]` to `## [X.Y.Z] - YYYY-MM-DD`, add a fresh empty
   `## [Unreleased]` above it, and update the link references at the
   bottom of the file.
2. Commit, then push an annotated tag `vX.Y.Z`.
3. [`release.yml`](../../.github/workflows/release.yml) builds the tarballs
   and publishes the release. Its notes are the matching `## [X.Y.Z]`
   section ([`scripts/release/notes.sh`](../../scripts/release/notes.sh));
   with no such section the tag message is used instead.
