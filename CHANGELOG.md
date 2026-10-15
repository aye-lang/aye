# Changelog

All notable changes to Aye are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/); before 1.0 the language and std
may still change between minor versions.

## [Unreleased]

### Tools

- A VS Code extension, `editors/vscode/`: syntax colors, brackets,
  indentation and snippets for `.aye`, and the `aye lsp` language server
  (errors, quick fixes, formatting, hover, go to definition). Releases
  attach it as `aye-vscode-<version>.vsix`.

## [0.1.0] - 2026-10-15

The first release: a stage0 compiler that emits C and builds it with clang,
a standard library, the tools around it, and a web framework written in Aye.

### Language

- Structs, enums with payloads and exhaustive `match`, tagged unions.
- Non-null pointers `*T` and `*mut T`, optionals `?T`, slices and arrays.
- Errors as values: `error` sets, `E!T`, `try`, `catch`, `errdefer`; stable
  error codes on every diagnostic.
- Explicit allocators, `defer`, comptime generics, `Pool` and `Handle`,
  modules and imports, constants, operators and control flow.
- C interop: `extern`, `@link`, `@cimport` of C headers.
- Threads (`std.thread`, `std.sync`) and coroutines (`std.coro`).
- Debug, safe and fast build modes; leak and stale-handle checks in debug.
- `aye.toml` dependencies by path or URL, pinned in `aye.lock`.

### Standard library

24 modules: `coro`, `crypto` (SHA-1, SHA-256, HMAC, random), `diag`,
`encoding` (base64, hex), `fs`, `heap`, `http` (client with HTTPS through
libcurl, and server), `json`, `list`, `log`, `map`, `math`, `mem`, `net`,
`os`, `path`, `pool`, `process`, `sort`, `sync`, `testing`, `text`, `thread`
and `time`. `aye doc --signatures std.<module>` prints the API of each.

### Tools

- `aye check` (all errors in one run, `--json` output) and `aye fix`
  (applies the fix-its).
- `aye fmt` formats on tokens and keeps comments.
- `aye test` runs `test` blocks, over a directory, in parallel, with a CPU
  time limit and `--json`.
- `aye lsp`, a language server: diagnostics, formatting, hover and go to
  definition.
- `aye spec`, the language reference in under 30k tokens, by section;
  `aye explain <code>`; `aye doc` (signatures and HTML); `aye cimport`.
- `aye --version`.

### Targets

- Native: macOS and Linux, arm64 and x86_64.
- `wasm32-wasip1` (run under Node's `node:wasi`) and `wasm32-freestanding`;
  object files for any target with `aye build -c`.

### Framework

- `framework/`: a file-system routed web server framework (`web gen`,
  `web dev`): pages, API routes, layouts, `not-found`, dynamic and catch-all
  segments, static files with ETag, redirects and cookies.

### Distribution

- Release tarballs for macOS arm64 and x86_64 and Linux arm64 and x86_64,
  with `SHA256SUMS`; see [docs/install.md](https://github.com/aye-lang/aye/blob/main/docs/install.md).

[Unreleased]: https://github.com/aye-lang/aye/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/aye-lang/aye/releases/tag/v0.1.0
