# Stage0 syntax and semantics decisions

Choices made in the stage0 compiler where the design doc did not fix the
answer. Each one can be revisited. The grammar is in `grammar.ebnf`.

## Syntax

- **Struct declaration:** `struct Name { field: T, ... }` at top level, with optional `pub`. Fields are separated by `,` or by newlines, and a trailing `,` is allowed.
- **`for` loop:** the only form is `for i in lo..hi { }`. The range is half-open. `lo` and `hi` must have the same integer type, and `hi` is evaluated once, before the first iteration. `i` is immutable. `in` is a contextual identifier, not a keyword, so the keyword count stays at 28.
- **Ranges** (`..`) are not general expressions. They appear only in a `for` header and in slicing.
- **Slicing:** `a[lo..hi]`, `a[lo..]`, `a[..hi]` and `a[..]`.
- **Semicolons:** Go's insertion rule. A terminator is inserted after an identifier, a literal, `)`, `]`, `}`, `return`, `break` or `continue`. An explicit `;` is a lexical error.
  - `else` must be on the same line as the `}` before it.
  - A multi-line call, struct literal or array literal needs a trailing comma.
  - A binary operator that continues an expression must end the line.
- **Struct literals in headers:** `Name {` in an `if`, `while` or `for` header is parsed as the start of the body, as in Go. Put the literal in parentheses to use it there.
- **Comparisons do not chain.** `a < b < c` is a parse error.
- **`as`** binds looser than unary operators and tighter than `*`. So `-x as f32` means `(-x) as f32`, and `a * b as f32` means `a * (b as f32)`.
- **`defer`** takes a block or a simple statement (a call or an assignment). A deferred body cannot `return`, and it cannot `break` or `continue` out of itself.
- **`true` and `false`** are predeclared identifiers, not keywords. The parser reads them as literals.
- **Initializers:** `let` and `var` must always have one (`var x: T = v`). There is no zero-initialization yet.
- **Array literals** need an explicit length: `[3]i32{1, 2, 3}`. There is no `[_]T` form yet.
- **`pub`** is accepted on `fn` and `struct` but has no effect until modules exist.
- **`main`:** the entry point must be `pub fn main()`, with no parameters and no return type. The generated C `main` returns 0.

## Types and checking

- **Untyped literals:** an integer or char literal takes the type that the context expects. An integer literal can also become a float type. With no context, an integer literal is `i64` and a float literal is `f64`. In a binary operation, a literal operand takes the type of the other operand. A literal that does not fit its type is a compile error.
- **Char literals** (`'a'`) are untyped integer constants whose value is the Unicode code point.
- **String literals** have type `[]u8`. They are UTF-8, and the bytes are followed by a NUL that `len` does not count, so `"text".ptr` can be passed to C as `[*]u8`.
- **Index and slice bounds** must have an unsigned integer type (`u8` to `u64`, `usize`). Untyped literals become `usize`. Signed indices need `as`, so a negative index is never possible. The design doc's examples index with a `u32`.
- **Slice elements are read-only.** You cannot assign through a slice yet. Write access will come with `[]mut T`. Array elements and struct fields can be assigned when the root binding is `var`.
- **Slicing an array** requires the array to be stored in a variable or other place, not a temporary value.
- **`.len`** works on arrays and slices and has type `usize`. **`.ptr`** works on slices and has type `[*]T`.
- **`as` casts** are numeric only.
  - int to int is checked: the program panics in debug and safe mode when the value does not fit the target type. It never truncates silently.
  - float to int is also checked: the program panics on NaN and on values out of range. The conversion truncates toward zero.
  - int to float and float to float use plain C conversion.
- **Integer arithmetic:**
  - `+ - * / %` and unary `-` panic on overflow. `/` and `%` also panic on division by zero. `MIN / -1` and `MIN % -1` count as overflow.
  - `+% -% *%` wrap.
  - `%` is integer-only.
  - There are no bitwise or shift operators yet.
- **Equality:** `==` and `!=` work on integers, floats and bools. Struct, array and slice equality are not supported.
- **Unused call results:** an expression statement must be a call, and a non-void result may be dropped silently. Other expression statements are errors.
- **Shadowing** is not allowed in any scope. A `let`/`var` binding, a parameter or a `for` variable is an error when its name is already visible: a local in the same or an enclosing block, a parameter of the function, or a top-level `fn`, `extern fn` or `struct`. Sibling blocks and different functions can reuse a name.
- **`isize` and `usize`** are 64-bit. Stage0 assumes an LP64 target.
- **`extern fn` signatures** can use only integer, float, `bool` and `[*]T` types, and the function cannot be variadic. Calls go straight to the C symbol of the same name.
- **Errors:** the compiler reports the first error and stops.

## Semantics and codegen

- **Evaluation order is left to right:** call arguments, binary operands, struct literal fields (in source order) and array elements. OIR fixes this order by computing every operand into a temporary.
- **Defer timing:**
  - `return` evaluates its value first and then runs the pending defers, innermost first.
  - `break` and `continue` run the defers of every scope up to and including the loop body.
- **Output builtins:** `@print(x)`, `@println(x)` and `@println()` print an integer, float (`%g`), bool or `[]u8`. They write through libc stdio, so their output stays in order with `extern fn puts`/`putchar`.
- **Panics:**
  - A panic flushes stdout and then writes `file:line:col: panic: <reason>` to stderr.
  - It ends with `abort()`, which gives exit status 134 from `ore run`.
  - For an arithmetic panic, the position is the operator. For an index or slice panic, it is the `[`.
- **No libc headers:** the generated C includes only `stdint.h`, `stddef.h` and `stdbool.h`, so user `extern fn` declarations cannot conflict with libc headers. The runtime calls libc through asm-labelled aliases such as `ore_rt_printf`.
- **Value types:** arrays are emitted as C structs (`struct { T a[N]; }`), so they are copied by value like structs.
- **`#line` directives** come before every generated statement so that debug info points to the `.ore` source.

## Differences from the design doc (to resolve later)

- The doc's examples use `pub fn main() !void`. Error unions are out of scope for stage0, so `main` is `pub fn main()`.
- The doc shows structs only as anonymous type expressions (`struct { ... }`, returned from type functions) with newline-separated fields. Stage0 uses a named declaration, `struct Name { ... }`. If type functions arrive, this might become `let Name = struct { ... }`.
- The doc shows `defer` only with a call (`defer f.close()`). Stage0 also accepts an assignment or a block.
- The doc has no `@` builtins. Its logging goes through `std.log`, and `std` does not exist yet. `@print` and `@println` are a stage0-only convenience until `std` lands.
