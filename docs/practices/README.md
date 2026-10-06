# Practices

Lessons this project learned the hard way: good cases, bad cases and the
practices that follow from them. It is written for the agents that do
most of the coding here (Claude sub agents, and GPT through pi), and for
people too.

Every agent reads it before coding. It is short on purpose: read the
file for the area you work in, and skim the others.

- [`ore.md`](ore.md): Ore language idioms. Errors, optionals,
  allocators, builtins, C interop, coroutines, safety and build modes.
- [`browser.md`](browser.md): conventions and pitfalls of the browser
  code in `browser/`.
- [`workflow.md`](workflow.md): developing, testing, merging, and
  working with several agents at once.

The language itself is in `docs/spec.md` (`ore spec`), the choices
behind it in `docs/decisions.md`, and the browser plan in
`docs/browser.md`. This directory does not repeat them; it records what
went wrong and what to do instead.

## Entry format

Each entry has the same shape:

```
### <short title>
**Context:** when this applies.
**Bad:** what went wrong (commit hash if known) + snippet.
**Good:** what to do + snippet / path:line.
**Why:** one or two sentences.
```

Snippets are real code, cited as `path:line` or by commit hash, or a
minimal example that compiles (`target/release/ore check` it before you
add it). Line numbers drift; when one is wrong, fix it.

## Maintaining it

- When a review, a merge or a bug teaches a lesson that will come up
  again, add an entry in the same commit as the fix.
- Put it in the file of its area, under the closest heading. One lesson
  per entry.
- Keep entries short: a few lines of text and a snippet of a few lines.
- When an entry stops being true (the compiler gained the feature, the
  code moved, the rule changed), update it or remove it in the commit
  that changed things.
- Add only lessons you can point to: a commit, a file or a test.
