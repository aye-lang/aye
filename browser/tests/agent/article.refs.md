# Notes on Ore

By [someone](file:///people/me) [e6], October 6

Ore is a systems language *without* garbage collection. Memory comes from explicit `Allocator` values, and errors are **values** too.

## Why another language?

Three reasons, in [no particular order](file://$ROOT/browser/tests/agent/article.html#order) [e8]:

1. Errors are values.
2. Enums are tagged unions.
3. No hidden allocation.

> Simple things should be simple.

```ore
fn main() {
    @println("hi")
}
```

![A figure](file://$ROOT/browser/tests/agent/img/fig.png)

Figure 1: a figure.

Read [the spec](https://example.com/spec?x=1&y=2) [e9] next.
