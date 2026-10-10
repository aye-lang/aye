Write `wordfreq`, a command-line program, in the file `main.aye`.

`wordfreq` reads text from standard input and counts its words. A word is
a maximal run of ASCII letters (`a`-`z`, `A`-`Z`); every other byte
separates words. Words are compared without regard to case: print them in
lower case.

Print one line per distinct word, `<count> <word>`, the most frequent word
first; words with the same count in alphabetical order. Empty input prints
nothing. Exit status 0.

```
$ printf 'The cat and the hat.\nThe CAT sat; a cat!\n' | wordfreq
3 cat
3 the
1 a
1 and
1 hat
1 sat
```

The input may be large (megabytes) and the program must not leak memory.
