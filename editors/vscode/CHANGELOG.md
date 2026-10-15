# Changelog

## 0.1.0

- Syntax colors for `.aye` files: keywords, primitive types, `@builtins`,
  numbers (decimal, `0x`, `0o`, `0b`, `_` separators, floats), strings with
  escapes, multi-line `\\` strings, characters, `//` and `///` comments.
- Brackets, comment toggling, auto-closing pairs and indentation.
- Snippets: `main`, `fn`, `struct`, `enum`, `error`, `match`, `for`, `test`.
- Language server: starts `aye lsp` for diagnostics, quick fixes, formatting,
  hover and go to definition. Setting `aye.path`; command
  "Aye: Restart Language Server".
