Write `grepn`, a command-line program, in the file `main.aye`.

`grepn <word>` reads lines from standard input and prints every line that
contains `<word>` (a case-sensitive substring), prefixed with its 1-based
line number and a colon:

```
$ printf 'apple\nbanana\ncherry pie\n' | grepn an
2:banana
```

- Lines end with `\n`; the last line may have no `\n`. Print each matching
  line followed by `\n`.
- Exit status 0 when at least one line matched, 1 when none did.
- With no argument, or with more than one, print `usage: grepn <word>` to
  standard error and exit with status 2.
