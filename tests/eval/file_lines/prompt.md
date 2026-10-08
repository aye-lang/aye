Write `numlines`, a command-line program, in the file `main.aye`.

`numlines <in> <out>` reads the file `<in>` and writes the file `<out>`
(created, or replaced if it exists): every line of `<in>` prefixed with its
1-based number, a colon and a space, and ended with `\n`. Then it prints
`<n> lines` to standard output, where `<n>` is the number of lines.

```
$ printf 'a\n\nc' > in.txt
$ numlines in.txt out.txt
3 lines
$ cat out.txt
1: a
2: 
3: c
```

- Lines end with `\n`; the last line may have no `\n`. An empty file has 0
  lines (the output file is then empty).
- If `<in>` cannot be read, print `cannot open <in>` (with the path) to
  standard error and exit with status 1; do not create `<out>`.
- With a wrong number of arguments, print `usage: numlines <in> <out>` to
  standard error and exit with status 2.
- The program must not leak memory.
