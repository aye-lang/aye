Write `jsontotal`, a command-line program, in the file `main.aye`.

`jsontotal` reads one JSON document (RFC 8259) from standard input. It is
an object with the key `"items"`, an array of objects; each item has a
string `"name"` and an integer `"qty"`. Print one line per item,
`<name> <qty>`, in array order, then `total <sum of qty>`.

```
$ echo '{"items": [{"name": "apple", "qty": 3}, {"qty": 2, "name": "pear"}]}' | jsontotal
apple 3
pear 2
total 5
```

- Keys may come in any order, and objects may have other keys whose values
  are any JSON value (strings, numbers such as `2.5e3`, `true`, `false`,
  `null`, arrays, objects, nested to any depth). Ignore them.
- Whitespace may appear wherever JSON allows it.
- Strings may contain the escapes `\"`, `\\`, `\/`, `\b`, `\f`, `\n`, `\r`,
  `\t` and `\uXXXX`; print names decoded, as UTF-8.
- `qty` may be negative.
- When the input is not valid JSON, or has no `"items"` array, print
  `invalid json` to standard error and exit with status 1 (print nothing
  to standard output).
- The program must not leak memory.
