# Ore language reference (stage0)

Ore is a systems language without garbage collection: memory comes from
explicit `Allocator` values and is released with `defer`. The syntax is
Go-like (braces, `name: Type`, no semicolons), errors are values (Zig-style
error unions), and enums are tagged unions with exhaustive `match`
(Rust-style). This document describes exactly what the stage0 compiler
accepts. It is printed by `ore spec` and versioned with the compiler.

Conventions: a block marked `ore` is a complete program that compiles; a
block marked `ore-error` is a program that the compiler rejects, shown to
illustrate a rule.

## 1. A first program

```ore
import std
import std.mem.Allocator

struct Counter {
    total: i64

    fn add(self: *mut Self, n: i64) {
        self.total += n
    }
}

fn sum(alloc: Allocator, n: usize) !i64 {
    let xs = try alloc.alloc(0 as i64, n)
    defer alloc.free(xs)
    for i in 0..n {
        xs[i] = i as i64 * 2
    }
    var c = Counter { total: 0 }
    for i in 0..xs.len {
        c.add(xs[i])
    }
    return c.total
}

pub fn main() !void {
    var gpa = std.heap.General.init()
    defer gpa.deinit()
    let total = try sum(gpa.allocator(), 4)
    @println(total)
}
```

`ore run first.ore` prints `12`. `gpa.deinit()` reports any memory that was
not freed.

## 2. Programs and commands

A program is a root module: one `.ore` file, or a directory whose `.ore`
files all belong to it (see section 12). The root module must declare
`pub fn main()` (no parameters; no return type or `!void`/`E!void`).

```
ore run <path> [--mode debug|safe|fast] [-- args...]  build to a temp file and run (args go to the program)
ore build <path> [-o out] [--mode m]                  build an executable (default name: the file or directory name)
ore check <path> [--json]                             report all compile errors; no C, no clang
ore fix <path>                                        apply the fix-its of `ore check` in place, repeat (at most 10 rounds)
ore test <path> [--filter text] [--json] [--mode m]   run the root module's `test` blocks, each in its own process
ore emit-c <path> [--mode m]                          print the generated C
ore explain <code>                                    explain an error code (E0001...) with a wrong and a fixed program
ore spec                                              print this document
```

`<path>` is a `.ore` file or a directory. A file is a module of its own:
other files in its directory are not compiled with it. Build modes: `debug`
(default; all runtime checks, error return traces, leak reports), `safe`
(optimized, all checks), `fast` (optimized, no checks). Exit status of `ore
run`: the program's status; 1 when `main` returns an error; 134 after a
panic.

## 3. Lexical structure

- Source is UTF-8. Comments start with `//` and run to the end of the line.
  There are no block comments.
- Identifiers: a letter or `_`, then letters, digits or `_` (ASCII only).
- Keywords (28): `fn let var pub import struct enum error interface dyn
  match if else for while break continue return defer errdefer try catch
  comptime where as test mut extern`.
- Predeclared names (not keywords, cannot be declared again): `true false
  null Self type` and the primitive types `i8 i16 i32 i64 isize u8 u16 u32
  u64 usize f32 f64 bool`. `in` is special only in a `for` header.
- Integer literals: `42`, `1_000`, `0xff`, `0o17`, `0b1010`. Float literals:
  `1.5`, `2.0e-3`, `1e9` (a float needs digits on both sides of `.`).
- Char literals `'a'`, `'\n'`, `'"'` are integers (the Unicode code point)
  and take the same escapes as strings.
- String literals `"text"` are single-line, have type `[]u8` (UTF-8 bytes,
  read-only), and are followed in memory by a NUL byte that `.len` does not
  count. Escapes: `\n \t \r \0 \\ \" \' \xHH`.
- Builtins start with `@`: `@print`, `@println`, `@expect`, `@expectEqual`,
  `@testAllocator`.

**Newlines end statements** (Go's rule). A newline is a statement
terminator when the last token on the line is an identifier, a literal,
`)`, `]`, `}`, `.*`, `return`, `break` or `continue`. An explicit `;` is an
error. Consequences:

- `else` must be on the same line as the `}` before it: `} else {`.
- A call, struct literal or array literal that spans lines needs a trailing
  comma before the closing `)` or `}` on its own line.
- A binary operator that continues an expression must end the line, not
  start the next one.

```ore
fn total(a: i64, b: i64, c: i64) i64 {
    return a +
        b +
        c
}

pub fn main() {
    let n = total(
        1,
        2,
        3,
    )
    if n > 5 {
        @println("big")
    } else {
        @println("small")
    }
}
```

```ore-error
pub fn main() {
    let n = 1
    if n > 0 {
        @println("positive")
    }
    else {
        @println("other")
    }
}
```

## 4. Declarations

Only declarations appear at the top level of a file, in any order: `fn`,
`extern fn`, `struct`, `enum`, `error`, `interface`, `let`, `var`, `test`
and `import`. Every declaration starts with its keyword and name, so `fn
name` finds a function's definition. `pub` makes a declaration visible to
other modules.

### Functions

```ore
fn add(a: i64, b: i64) i64 {
    return a + b
}

fn log2(x: u64) u64 {
    var n: u64 = 0
    var v = x
    while v > 1 {
        v /= 2
        n += 1
    }
    return n
}

fn hello() {
    @println("hello")
}

pub fn main() {
    hello()
    @println(add(1, 2) + log2(1024) as i64)
}
```

- Parameters are `name: Type` and immutable. The return type follows `)`;
  none means the function returns nothing. A function with a return type
  must `return` a value on every path.
- No overloading, default arguments, variadic functions, closures or
  function values. A function name is only called: `f(x)`.
- Functions may call each other in any order, and recurse.

### Constants, globals and type aliases

```ore
struct Point { x: i64, y: i64 }

let MAX_ITEMS: usize = 3
let PRIMES = [3]u32{2, 3, 5}
let ORIGIN = Point { x: 0, y: MAX_ITEMS as i64 * 2 }
let MASK: u32 = (1 << 8) - 1
let Bytes = []u8
var calls: i64 = 0

fn shout(s: Bytes) {
    calls += 1
    @println(s)
}

pub fn main() {
    var sum: u32 = 0
    for p in PRIMES {
        sum += p & MASK
    }
    @println(sum)
    shout("hi")
    @println(ORIGIN.y + calls)
}
```

- `let NAME = value` at the top level is a constant, `var name = value` a
  global variable. The value is computed by the compiler: literals, other
  `let` constants, operators, `as`, and array, struct and enum literals. No
  calls and no `var` globals; overflow there is a compile error. A global
  is initialized before `main` runs, in any declaration order.
- As for a local `let`, an untyped integer gives `i64`: write `let N:
  usize = 4` for an index bound.
- `let Name = Type` (a type name, `Pool(Node)`, `[]u8`, `*Node`, ...) is a
  type alias: another name for the same type, usable in types, literals
  (`Name { ... }`) and static calls (`Name.init()`).
- Constants cannot be assigned. No local may reuse a global's name.

### Structs and methods

```ore
struct Point {
    x: f64
    y: f64

    fn origin() Point {
        return Point { x: 0.0, y: 0.0 }
    }

    fn len2(self: Self) f64 {
        return self.x * self.x + self.y * self.y
    }

    fn scale(self: *mut Self, k: f64) {
        self.x *= k
        self.y *= k
    }
}

pub fn main() {
    var p = Point { x: 3.0, y: 4.0 }
    p.scale(2.0)
    @println(p.len2())
    @println(Point.origin().x)
    @println(Point.len2(p))
}
```

- Fields are `name: Type`, separated by newlines or `,`. Functions declared
  in the struct body are its methods; inside them `Self` is the struct.
- The first parameter decides how a method is called on a value: `self:
  Self` (a copy), `self: *Self` (read-only pointer) or `self: *mut Self`
  (writable). `x.m(args)` passes `x` (or `&x`/`&mut x` automatically when
  the method takes a pointer; `&mut` needs `x` to be a `var`). A method
  without such a parameter is called as `Type.f(args)`. Any method can be
  called as `Type.f(receiver, args)`.
- A struct literal names its type and lists every field once:
  `Point { x: 1.0, y: 2.0 }`. There are no default field values.
- A method cannot have the name of a field of its struct.

### Enums (tagged unions)

```ore
enum Shape {
    Circle { r: f64 }
    Rect { w: f64, h: f64 }
    Empty

    fn area(self: Self) f64 {
        return match self {
            .Circle { r } => 3.14159 * r * r
            .Rect { w, h } => w * h
            .Empty => 0.0
        }
    }

    fn unit() Self {
        return Self.Rect { w: 1.0, h: 1.0 }
    }
}

enum Dir { Left, Right }

pub fn main() {
    let c = Shape.Circle { r: 1.0 }
    @println(c.area())
    @println(Shape.unit().area())
    @println(Shape.area(Shape.Empty))
    let d = Dir.Left
    @println(d == Dir.Right)
}
```

A value always names the enum: `Shape.Empty`, `Shape.Circle { r: 1.0 }`
(there is no `.Circle` shorthand in expressions). A variant without payload
has no braces. Functions in the enum body are methods, as for structs;
`Self` is the enum. `==` and `!=` work only on enums whose variants all have
no fields; `match` the others.

### Error sets

`error Name { A, B }` declares a set of errors; `error All = A || B` merges
sets. See section 8.

### Interfaces

`interface Name { fn m(self: *Self, ...) T }`: see section 10.

### extern fn (C functions)

```ore
extern fn puts(s: [*]u8) i32

pub fn main() {
    let r = puts("from C".ptr)
}
```

`extern fn name(params) Ret` declares a C function; calls go to the C
symbol of the same name. Parameter and return types may only be integers,
floats, `bool` and C pointers `[*]T`. Variadic C functions cannot be
declared. A string literal's `.ptr` is a NUL-terminated `[*]u8`.

## 5. Types

| Type | Meaning |
| --- | --- |
| `i8 i16 i32 i64 isize` | signed integers (`isize` is 64-bit) |
| `u8 u16 u32 u64 usize` | unsigned integers (`usize` is 64-bit) |
| `f32 f64` | floats |
| `bool` | `true`, `false` |
| `[N]T` | array of N elements, a value (copied on assignment) |
| `[]T` | slice: pointer + length; elements read-only |
| `[]mut T` | slice whose elements can be assigned |
| `*T`, `*mut T` | non-null pointer, read-only or writable |
| `?T` | optional: a `T` or `null` |
| `[*]T` | unchecked C pointer, for `extern fn` only |
| `E!T`, `!T` | error union, only as a function's return type |
| `dyn I` | pointer to a value implementing interface `I`, plus vtable |
| struct, enum, error set names | declared types |
| `Pool(Node)` | instance of a type function (section 11) |

There is no string type: text is `[]u8`. There is no `void` type except as
the payload of an error union (`!void`).

### Conversions

There are no implicit conversions between types. Numeric conversions are
written `x as T` (only between numeric types):

- int to int panics (debug, safe) if the value does not fit; it never
  truncates silently.
- float to int truncates toward zero and panics on NaN or out of range.
- int to float and float to float are plain conversions.

`as` binds tighter than `*` and looser than unary operators: `a * b as f64`
is `a * (b as f64)`, `-x as f64` is `(-x) as f64`.

The only implicit adjustments: a `T` is accepted where `?T` is expected; a
`*mut T` where `*T` is expected; a `[]mut T` where `[]T` is expected; an
error value of one set where a set containing all its errors is expected;
a `*S` or `*mut S` where `dyn I` is expected (if struct `S` implements `I`).

### Untyped literals

An integer or char literal takes the type the context expects (a typed
`let`, a parameter, the other operand of a binary operator, an index). With
no context it is `i64`, except that a range of two literals is `usize`
(§6); a float literal is `f64`. An integer literal can
become a float type. A literal that does not fit its type is a compile
error. So `let n = 5` is `i64`, `let m: u8 = 5` is `u8`, and `x + 1` has
the type of `x`.

## 6. Statements

```ore
pub fn main() {
    let a = 10
    var b: i64 = 0
    b = a * 2
    b += 1
    var i: i64 = 0
    while i < 3 {
        i += 1
        if i == 2 {
            continue
        }
    }
    for k in 0..4 {
        if k == 3 {
            break
        }
        @print(k)
    }
    @println()
    var arr = [3]i64{1, 2, 3}
    for j, x in arr {
        arr[j] = x * 10
    }
    for x in arr[1..] {
        @println(x)
    }
    {
        let inner = b
        @println(inner)
    }
}
```

- `let x = v` binds an immutable name; `var x = v` a mutable one. A type
  may be given: `var x: u8 = 0`. An initializer is always required; there
  is no zero-initialization.
- Assignment: `=`, and `+= -= *= /= %= +%= -%= *%= &= |= ^= <<= >>=`. The
  target is a `var`, a field or element of a `var`, data behind a `*mut T`,
  or an element of a `[]mut T`.
- `if cond { } else if cond { } else { }`; the condition must be `bool`
  (no truthiness). Braces are required.
- `while cond { }`. There is no `do`/`loop`; use `while true { }`.
  `while x |v| { }` runs while the optional `x` (evaluated before each
  iteration) has a value `v`.
- `for i in lo..hi { }` counts: `i` goes from `lo` up to `hi - 1`
  (half-open), both bounds have the same integer type (`0..s.len` is
  `usize`), `hi` is evaluated once, and `i` is immutable. A range of two
  literals is `usize`, so `-1..3` is an error; for a signed counter write
  `(0 as i64)..4`.
- `for x in xs { }` and `for i, x in xs { }` walk an array or a slice: `x`
  is a copy of each element, `i` its `usize` index; `xs` is evaluated once.
  To change elements, assign `xs[i]`.
- `break` and `continue` apply to the innermost loop.
- `return`, `return value`.
- `defer stmt` runs `stmt` when the enclosing block is left: at its end,
  or by `return` (after the return value is computed), `break` or
  `continue`. Several defers run in reverse order. The statement is a call,
  an assignment or a block, and cannot contain `return` or `try`.
  `errdefer` (section 8) runs only when an error is returned.
- A block `{ ... }` opens a scope.
- An expression used as a statement must be a call, `try`, `catch` or
  `match`. A call's non-void result may be dropped, except an error union.

**No shadowing anywhere**: a new name (local, parameter, `for` variable,
capture, pattern binding) cannot reuse any name visible at that point,
including names of enclosing scopes, top-level declarations of the module
and imports. Sibling blocks may reuse a name. A capture is scoped to its
block or expression, so two `catch |e|` in one function may both use `e`.

```ore-error
fn f(x: i64) i64 {
    let x = 2
    return x
}

pub fn main() {
}
```

## 7. Expressions

Precedence, from loosest to tightest (binary operators are
left-associative; comparisons do not chain, `a < b < c` is an error):

| Level | Operators |
| --- | --- |
| 1 | `\|\|` |
| 2 | `&&` |
| 3 | `== != < <= > >=` |
| 4 | `catch` |
| 5 | `& \| ^` (not mixed without parentheses) |
| 6 | `<< >>` |
| 7 | `+ - +% -%` |
| 8 | `* / % *%` |
| 9 | `as` |
| 10 | prefix `- ! ~ & &mut try` |
| 11 | call `f()`, `x.f`, `x.m()`, `x[i]`, `x[a..b]`, `p.*` |

- Arithmetic `+ - * / %` panics on overflow and division by zero (debug and
  safe mode); `+% -% *%` wrap. Integer `/` truncates toward zero; `%` is for
  integers only. Unsigned subtraction below zero is an overflow: check
  first (`if i > 0 { i -= 1 }`). Both operands of a
  binary operator have the same type (use `as`).
- Bitwise `& | ^`, prefix `~` and shifts `<< >>` take integers; both
  operands have one type, also for a shift (`x << n` needs `n` of `x`'s
  type). Unlike C, `x & 1 == 0` is `(x & 1) == 0`. `a & b | c` is an
  error: write `(a & b) | c`. A shift by the bit width or more panics;
  `<<` drops the bits shifted out; `>>` of a signed value keeps the sign.
- In an `if` or `while` condition a `|` outside parentheses starts the
  capture `|v|`: write a bitwise or there as `if (a | b) != 0`.
- `==` and `!=` work on integers, floats, `bool`, pointers (`*T`, `[*]T`:
  the same address) and enums without payloads, and between an optional
  and `null`. Structs, arrays, slices and enums with payloads cannot be
  compared; compare `[]u8` contents with `std.mem.eql(a, b)`.
- `&&`, `||`, `!` take `bool` and short-circuit.
- Evaluation order is left to right everywhere.

### Arrays and slices

```ore
fn sum(xs: []i64) i64 {
    var t: i64 = 0
    for i in 0..xs.len {
        t += xs[i]
    }
    return t
}

pub fn main() {
    var arr = [4]i64{1, 2, 3, 4}
    arr[0] = 10
    let all = arr[..]
    all[1] = 20
    @println(sum(all))
    @println(sum(arr[2..]))
    @println(arr.len)
    let empty = []i64{}
    @println(sum(empty))
    let s = "hello"
    @println(s[1..3])
    @println(s[0])
}
```

- An array literal states its length: `[3]u8{1, 2, 3}` (no `[_]`). A slice
  literal can only be empty: `[]T{}`. Make other slices by slicing an array
  or with `alloc.alloc` (section 9).
- Indexing `x[i]` and slicing `x[lo..hi]`, `x[lo..]`, `x[..hi]`, `x[..]`
  are bounds-checked (panic). Indices are unsigned integers (`usize`
  usually); a signed index needs `as usize`.
- Slicing a `var` array gives `[]mut T`; slicing a `let` array gives `[]T`;
  slicing a slice keeps its type. Arrays must be in a variable to be sliced.
- `.len` (type `usize`) on arrays and slices; `.ptr` (type `[*]T`) on
  slices. A `[*]T` can be sliced with both bounds, `p[0..n]`, giving an
  unchecked `[]T`.

### Pointers

`&x` makes a `*T`, `&mut x` a `*mut T` (`x` must be a `var` or reached
through a `*mut`). `p.*` dereferences. Field access and method calls go
through one pointer level automatically: `p.x`, `p.x = 1` for `p: *mut S`.
`p == q` is true when both point to the same place. There is no pointer
arithmetic and no null pointer: use `?*T`. A temporary cannot be borrowed (`&f()` is an error).

### Optionals

```ore
fn find(xs: []i64, want: i64) ?usize {
    for i in 0..xs.len {
        if xs[i] == want {
            return i
        }
    }
    return null
}

pub fn main() {
    let arr = [3]i64{5, 6, 7}
    let r = find(arr[..], 6)
    if r |i| {
        @println(i)
    } else {
        @println("missing")
    }
    if r != null {
        @println("found")
    }
    var maybe: ?i64 = null
    maybe = 3
}
```

`if x |v| { } else { }` unwraps: `v` is the value, visible in the first
block; `while x |v| { }` loops while `x` has a value. `null` needs a known
optional type (`var m: ?i64 = null`). There is no `orelse` and no `.?`.

### match

`match x { pattern => body ... }` works on enums, error values and
integers, and must be exhaustive (or end with `_`). Enum patterns: `.V`,
`.V { a, b }`, `.V { a: name }`, `.V { a, .. }` (ignore the other fields),
`_`. Without `..` every payload field must be listed. Integer patterns:
`10`, `-1`, `'a'`, inclusive ranges `'a'..='z'`, several per arm
(`' ', '\t' =>`); without `_` they must cover every value of the type.
Arms are separated by newlines or `,`.

- As an expression, each arm is an expression (not a block), and all arms
  have the same type.
- As a statement, each arm is a block or a call.

```ore
enum Cmd {
    Move { dx: i64, dy: i64 }
    Say { text: []u8 }
    Quit
}

fn run(c: Cmd) {
    match c {
        .Move { dx, dy: down } => {
            @println(dx + down)
        }
        .Say { text } => @println(text)
        .Quit => @println("bye")
    }
}

fn isQuit(c: Cmd) bool {
    return match c {
        .Quit => true
        _ => false
    }
}

pub fn main() {
    run(Cmd.Move { dx: 1, dy: 2 })
    run(Cmd.Say { text: "hi" })
    @println(isQuit(Cmd.Quit))
}
```

```ore
fn class(c: u8) []u8 {
    return match c {
        'a'..='z', 'A'..='Z', '_' => "word"
        '0'..='9' => "digit"
        ' ', '\t', '\n' => "space"
        _ => "other"
    }
}

fn utf8Len(b: u8) usize {
    return match b {
        0x00..=0x7F => 1
        0x80..=0xBF => 0
        0xC0..=0xDF => 2
        0xE0..=0xEF => 3
        0xF0..=0xFF => 4
    }
}

pub fn main() {
    @println(class('x'))
    @println(utf8Len(0xE4))
}
```

`Name {` in an `if`, `while`, `for` or `match` header starts the body (as
in Go): put a struct literal there in parentheses.

## 8. Errors

Errors are values; there are no exceptions. A program bug is a panic.

```ore
error ParseError { Empty, BadDigit, Overflow }

pub fn parse(s: []u8) ParseError!u32 {
    if s.len == 0 {
        return ParseError.Empty
    }
    var n: u32 = 0
    for i in 0..s.len {
        let c = s[i]
        if c < '0' || c > '9' {
            return ParseError.BadDigit
        }
        if n > 400000000 {
            return ParseError.Overflow
        }
        n = n * 10 + (c - '0') as u32
    }
    return n
}

fn double(s: []u8) !u32 {
    let n = try parse(s)
    return n * 2
}

pub fn main() !void {
    @println(try double("21"))
    let a = parse("") catch 0
    @println(a)
    let b = parse("x1") catch |e| match e {
        .Empty => 1
        .BadDigit => 2
        .Overflow => 3
    }
    @println(b)
}
```

- `error Name { A, B }` declares an error set (it may span lines, one error
  per line with a trailing `,`); `error All = A || B` merges
  sets. An error value is written `Set.Name`. Errors are identified by name:
  `NotFound` in two sets is the same error.
- `E!T` is "an error of set `E`, or a `T`". `!T` lets the compiler infer the
  set from the function body; it is allowed only on functions that are not
  `pub` (and on `pub fn main() !void`). A `pub` function writes its set. A
  function with an inferred set cannot call itself (directly or through
  other inferred functions); write the set.
- Error unions exist only as return types: a call returning `E!T` must be
  handled immediately with `try`, `catch`, or `return f()` (from a function
  whose set has all its errors). It cannot be stored in a variable, passed,
  compared or dropped.
- `return Set.Name` returns an error; `return value` returns the payload.
- `try f()` gives the payload, or returns the error from the current
  function (which must return an error union whose set has the error).
- `f() catch value` gives `value` on error; `f() catch |e| expr` can use
  the error. `f() catch |e| { ... }` runs the block on error; when the
  payload is not `void` the block must end with `return`, `break` or
  `continue`. A `return` there returns from the enclosing function. `catch` binds looser than `+` and tighter than `==`.
- `match e { .A => ..., .B => ... }` on an error value must cover every
  error of its set (or use `_`).
- `errdefer stmt` runs when the scope is left by returning an error (a
  `return` of an error or a `try` that passes one on), never on success.
- When `main` returns an error, the program prints `error: Name` to stderr,
  in debug mode followed by `error return trace:` and one `file:line:col`
  per `return`/`try` the error passed through (where it started first), and
  exits with status 1.
- Errors carry no payload. Report details through a parameter or `std.log`.

## 9. Memory

There is no garbage collector and no hidden allocation: a function that
allocates takes an `Allocator`. Free with `defer`.

```ore
import std
import std.mem.Allocator
import std.mem.AllocError

struct Node {
    value: i64
    next: ?*mut Node
}

fn push(alloc: Allocator, head: ?*mut Node, v: i64) AllocError!*mut Node {
    return alloc.create(Node { value: v, next: head })
}

pub fn main() !void {
    var gpa = std.heap.General.init()
    defer gpa.deinit()
    var arena = std.heap.Arena.init(gpa.allocator())
    defer arena.deinit()
    let a = arena.allocator()

    var head: ?*mut Node = null
    for i in (0 as i64)..3 {
        head = try push(a, head, i)
    }
    var cur = head
    while cur != null {
        if cur |n| {
            @println(n.value)
            cur = n.next
        }
    }

    let buf = try gpa.allocator().alloc(0 as u8, 16)
    defer gpa.allocator().free(buf)
    buf[0] = 'x'
    let one = try gpa.allocator().create(42 as i64)
    defer gpa.allocator().destroy(one)
}
```

- `std.heap.General.init()` is a general allocator (libc malloc). In debug
  mode `deinit()` prints `file:line:col: leak: N bytes allocated here were
  never freed` for every allocation not freed, and a second free panics
  (`double free`). Leaks do not change the exit status of a program (they
  fail a test).
- `std.heap.Arena.init(backing)` allocates from `backing` and frees
  everything at once with `deinit()`; its `destroy`/`free` do nothing.
- `x.allocator()` gives the `Allocator` of a `General` or `Arena`; it and
  `deinit()` need `x` to be a `var`.
- On an `Allocator`: `create(value)` returns `AllocError!*mut T` holding
  `value`; `destroy(p)` frees it. `alloc(value, n)` returns
  `AllocError![]mut T` with `n` copies of `value`; `free(slice)` frees it
  (pass the whole slice from `alloc`, not a sub-slice that starts later).
  Memory is always initialized, so you pass a value, not a type:
  `alloc.create(Node { ... })`, `alloc.alloc(0 as u8, n)`.
- `alloc.resize(s, n, fill)` returns a slice of `n` elements: the first
  `min(s.len, n)` are copied from `s`, the rest are `fill`. On success `s`
  is freed (do not use it again); on `OutOfMemory` it is unchanged. It may
  move the elements: write `s = try alloc.resize(s, n, fill)`.
- `AllocError` is `error AllocError { OutOfMemory }`.
- Use-after-free is not detected. Prefer arenas for data with one lifetime.
- std has no growable list yet. To grow a buffer, `alloc` a larger slice,
  copy the elements (`std.mem.copy` for `[]u8`, a loop otherwise) and `free`
  the old one; keep the length used in a separate variable.

## 10. Interfaces and dyn

```ore
interface Shape {
    fn area(self: *Self) f64
}

struct Square {
    side: f64

    fn area(self: *Self) f64 {
        return self.side * self.side
    }
}

fn show(s: dyn Shape) {
    @println(s.area())
}

pub fn main() {
    var sq = Square { side: 3.0 }
    show(&sq)
    let d: dyn Shape = &sq
    @println(d.area())
}
```

- An interface lists method signatures; the first parameter is `self:
  *Self` or `self: *mut Self`, and an error union names its set.
- A struct implements an interface when it has every method with the same
  parameter and return types (structurally, as in Go; no `impl`). A method
  taking `self: *mut Self` needs the interface to declare `*mut Self` and
  the `dyn` to be made from `&mut x`.
- `dyn I` is made only from a pointer, `&x` or `&mut x`, where a `dyn I` is
  expected. It does not allocate. Calls go through a vtable.

## 11. Generics

```ore
interface Show {
    fn show(self: *Self)
}

fn max(comptime T: type, a: T, b: T, bigger: bool) T {
    if bigger {
        return a
    }
    return b
}

fn showAll(comptime T: type, items: []T) where T: Show {
    for i in 0..items.len {
        items[i].show()
    }
}

pub fn Pair(comptime T: type) type {
    return struct {
        first: T
        second: T

        pub fn swap(self: *mut Self) {
            let t = self.first
            self.first = self.second
            self.second = t
        }
    }
}

struct Num {
    v: i64

    fn show(self: *Self) {
        @println(self.v)
    }
}

pub fn main() {
    @println(max(i64, 3, 4, false))
    var p = Pair(i64) { first: 1, second: 2 }
    p.swap()
    @println(p.first)
    var nums = [2]Num{Num { v: 7 }, Num { v: 8 }}
    showAll(Num, nums[..])
}
```

- A `comptime T: type` parameter takes a type; the call passes it like an
  argument: `max(i64, a, b, true)`. A type that is not a plain name is
  written as a type: `f(*Node, p)`, `f([]u8, s)`.
- A function returning `type` is a type function; its body is exactly
  `return struct { ... }` and it has only `comptime` parameters. `Pair(i64)`
  is a type anywhere a type is written, and also in expressions before `{`
  and `.f(...)`: `Pair(i64) { ... }`, `Pool(Node).init(a)`.
- A generic is checked once where it is defined. A value of type `T` can be
  copied, moved, pointed to, stored in arrays, slices and optionals, and
  allocated; it has no fields, operators or methods except the methods of
  its `where` constraints. `where T: A, T: B` (no `+`) follows the return
  type. A type argument that does not satisfy a constraint is an error at
  the instantiation.
- Generic functions and methods of a type function's struct must spell out
  their error sets (`E!T`). A top-level `let P = Pair(i64)` names an
  instance (a type alias, section 4).

## 12. Modules and imports

- A module is a directory; all `.ore` files directly in it share one
  namespace (a file can use the other files' declarations without
  importing them). The root module is the directory or file given to `ore`.
- `import a.b` loads the module in directory `a/b` relative to the root
  module's directory, or a std module (`std`, `std.fs`, ...). The import
  binds its last name (`b`) in the importing file only. Use items as
  `b.name`.
- `import a.b.Name` imports a single item and binds `Name`.
- After `import std`, every std module is reachable by path:
  `std.fs.open(...)`, `std.heap.General.init()`.
- There are no glob imports and no renaming: every name from another module
  traces to one import line in the file.
- Only `pub` declarations (functions, structs, enums, error sets,
  interfaces, and `pub fn` methods) can be used from another module. Fields
  and enum variants are visible wherever their type is.
- An import name cannot also be declared in the module, and locals cannot
  shadow it. Modules cannot import each other in a cycle.
- `test` blocks of imported modules are not compiled.
- Errors and types from other modules are shown with their path in
  messages: `std.fs.File`, `std.mem.AllocError`.

A directory module `app/` with `app/main.ore` and `app/util/util.ore`:

```
// app/util/util.ore
pub fn twice(x: i64) i64 {
    return 2 * x
}

// app/main.ore
import util

pub fn main() {
    @println(util.twice(21))
}
```

`ore run app` prints `42`.

## 13. Tests

```ore
fn add(a: i64, b: i64) i64 {
    return a + b
}

test "add" {
    @expectEqual(4, add(2, 2))
    @expect(add(1, 1) == 2)
}

test "allocations are checked" {
    let alloc = @testAllocator()
    let xs = try alloc.alloc(0 as u8, 8)
    defer alloc.free(xs)
    @expectEqual(8, xs.len)
}
```

- `test "name" { }` at the top level; the body is like a `!void` function
  (it can use `try`). `ore test` runs each test of the root module in its
  own process; `ore build`/`ore run` leave tests out. A module with tests
  needs no `main` for `ore check` and `ore test`.
- `@expect(cond)` and `@expectEqual(expected, actual)` (integers, floats,
  `bool`, `[]u8`) fail the test with `file:line:col: assertion failed:
  expected X, found Y`.
- `@testAllocator()`, also written `std.testing.allocator()`, is a
  leak-checked `Allocator`, usable only in a test block. A leak, panic,
  failed assertion or returned error fails the test.

Output: `test "name" ... ok` or `... FAILED` followed by the test's indented
stdout and stderr, then `N passed; M failed`. `--filter text` runs tests
whose name contains `text`. `--json` prints one line:
`{"file":F,"diagnostics":[...],"tests":[{"name":N,"status":"pass"|"fail",
"failures":[...],"stdout":S,"stderr":S}],"passed":P,"failed":Q}`, where a
failure is `{"kind":"assertion"|"panic"|"leak","line":L,"col":C,"message":M}`,
`{"kind":"error","error":"Name","trace":[{"line":L,"col":C}]}` or
`{"kind":"crash","message":M}`.

## 14. Builtins

- `@print(x)`, `@println(x)`, `@println()`: write an integer, float (`%g`),
  `bool` or `[]u8` to stdout (buffered C stdio). A `u8` is an integer:
  `@print(s[i])` writes `104`; write one character with `@print(s[i..i + 1])`.
- `@expect`, `@expectEqual`, `@testAllocator`: section 13.

## 15. std

std is built into the compiler. Import it with `import std` (then use
`std.fs...`), `import std.fs` (then `fs...`) or by item (`import
std.mem.Allocator`).

### std.mem

```
Allocator                                  allocator interface value (section 9)
Allocator.create(value) AllocError!*mut T
Allocator.destroy(p: *T)
Allocator.alloc(value, n: usize) AllocError![]mut T
Allocator.free(s: []T)
Allocator.resize(s: []T, n: usize, fill: T) AllocError![]mut T   n elements: s's first, then fill; frees s
error AllocError { OutOfMemory }
std.mem.eql(a: []u8, b: []u8) bool         same length and bytes
std.mem.copy(dst: []mut u8, src: []u8)      copy src to the start of dst (panics if dst is shorter)
```

### std.heap

```
General.init() General                     libc allocator; leak and double-free reports in debug mode
General.allocator(self: *mut Self) Allocator
General.deinit(self: *mut Self)            reports leaks (debug), frees everything
Arena.init(backing: Allocator) Arena
Arena.allocator(self: *mut Self) Allocator
Arena.deinit(self: *mut Self)              frees everything the arena allocated
```

Written `std.heap.General.init()` after `import std`, or `General.init()`
after `import std.heap.General`.

### std.fs

```
error OpenError { NotFound, AccessDenied, OpenFailed }
error ReadError { ReadFailed, OutOfMemory }
error WriteError { WriteFailed }
struct File { fd: i32 }
std.fs.open(path: []u8) OpenError!File      open a file for reading
std.fs.stdin() File
std.fs.stdout() File
std.fs.stderr() File
File.readAll(self: Self, alloc: Allocator) ReadError![]mut u8   read to the end (empty file: empty slice); free with alloc.free
File.write(self: Self, bytes: []u8) WriteError!void              write everything (flushes @print output first)
File.close(self: Self)
```

### std.testing

```
std.testing.allocator() Allocator          the test's leak-checked allocator (as @testAllocator()); test blocks only
```

### std.log

```
std.log.err(format, args...)     writes "error: <message>\n" to stderr
std.log.info(format, args...)    writes "info: <message>\n" to stderr
```

`format` must be a string literal. Each `{}` in it is replaced by the next
argument: an integer, float, `bool`, `[]u8`, or an error value (written as
its name). The number of `{}` must equal the number of arguments. Other
characters, including lone braces, are written as they are. Pending `@print`
output is flushed first.

### std.os

```
std.os.args(alloc: Allocator) AllocError![][]u8   the arguments; [0] is the program; free the slice with alloc.free
std.os.exit(code: i32)                            exit now with this status (flushes @print output; deferred statements do not run)
```

To exit with a status and still free everything, do the work in a function
that returns the status, and call `deinit` yourself before `exit`:

```ore
import std
import std.mem.Allocator

fn run(alloc: Allocator) i32 {
    let args = std.os.args(alloc) catch |e| {
        std.log.err("out of memory")
        return 1
    }
    defer alloc.free(args)
    if args.len < 2 {
        std.log.err("usage: tool <file>")
        return 2
    }
    return 0
}

pub fn main() {
    var gpa = std.heap.General.init()
    let alloc = gpa.allocator()
    let status = run(alloc)
    gpa.deinit()
    std.os.exit(status)
}
```

### A complete CLI

```ore
import std
import std.mem.Allocator

fn countWords(text: []u8) usize {
    var words: usize = 0
    var inWord = false
    for i in 0..text.len {
        let c = text[i]
        if c == ' ' || c == '\n' || c == '\t' {
            inWord = false
        } else if !inWord {
            inWord = true
            words += 1
        }
    }
    return words
}

fn readInput(alloc: Allocator, args: [][]u8) ![]mut u8 {
    if args.len > 1 {
        let f = try std.fs.open(args[1])
        defer f.close()
        return f.readAll(alloc)
    }
    return std.fs.stdin().readAll(alloc)
}

pub fn main() !void {
    var gpa = std.heap.General.init()
    defer gpa.deinit()
    let alloc = gpa.allocator()
    let args = try std.os.args(alloc)
    defer alloc.free(args)
    let text = readInput(alloc, args) catch |e| {
        std.log.err("cannot read input: {}", e)
        return e
    }
    defer alloc.free(text)
    @print(countWords(text))
    @print(" ")
    @println(text.len)
}
```

## 16. Runtime behavior

- A failed check panics: it flushes stdout, writes `file:line:col: panic:
  <reason>` to stderr and aborts (status 134). Reasons: `integer overflow`,
  `division by zero`, `index out of bounds: index I, len N`, `slice bounds
  out of range`, `integer cast out of range`, `shift amount out of range`,
  `double free ...`.
- `fast` mode removes the checks: overflow wraps, out-of-bounds access is
  undefined behavior.
- Struct fields, arrays and slices are laid out by the compiler; there is
  no `extern struct` yet.

## 17. Diagnostics

Errors print as `file:line:col: error[E0023]: message`, sorted by file and
position, all of them (up to 50): the compiler recovers from errors. If
there are syntax or import errors, type checking does not run. Every error
has a stable code; `ore explain E0023` shows a wrong and a fixed program.

Some errors have a fix-it, printed as `file:line:col: help: <what it does>`
and applied by `ore fix`: `let` to `var` when a `let` is assigned; joining
`}` and `else`; a missing trailing `,` before a closing `)`/`}` on the next
line; renaming an unknown name to the only visible name one edit away;
adding `import std.mem.Allocator` (and `AllocError`, `std.heap.General`,
`std.heap.Arena`) when those names are used without an import.

`ore check --json` prints one line:

```
{"file":"f.ore","diagnostics":[{"severity":"error","code":"E0032","file":"f.ore","line":4,"col":9,
  "message":"cannot assign to `sum`: it is immutable (declare it with `var`)",
  "fixes":[{"line":2,"col":5,"end_line":2,"end_col":8,"replacement":"var"}]}]}
```

`diagnostics` is `[]` when the program compiles. Lines and columns are
1-based; a column counts characters. Each fix-it edit replaces the text of
the diagnostic's `file` from `line:col` up to (not including)
`end_line:end_col` with `replacement`; an insertion has equal start and
end. The edits of one diagnostic are applied together.

## 18. Pitfalls (differences from Go, Rust and Zig)

- No semicolons; `} else {` on one line; trailing `,` in multi-line calls
  and literals; an operator that continues a line ends it.
- No shadowing at all, unlike Rust (`let x = x + 1` is an error) and Go
  (`:=` in an inner scope). Pick a new name.
- `let` is immutable (Rust's `let`, not JavaScript's). Use `var` to assign,
  to take `&mut`, or to call a `*mut Self` method on a value.
- Every `var` needs an initializer: `var n: usize = 0`. No zero values.
- No implicit integer widening: `a + b` needs equal types; convert with
  `as`. Indices must be unsigned (`usize`); `.len` is `usize`.
- `for i in 0..4` gives a `usize` (as in Zig; Go's `range 4` gives
  `int`); write `(0 as i64)..4` for a signed counter.
- `for x in xs` and `for i, x in xs` (index first, as in Go) give copies
  of the elements. No `for (xs) |x|`, no C-style `for`.
- String literals are `[]u8` and read-only; there is no `+` on strings, no
  string formatting into memory, and `==` does not compare slices
  (`std.mem.eql`).
- Char literals are integers: `'a'` compares with a `u8` directly, and
  `@print` of a `u8` writes its number (print a one-byte slice instead).
- `match` is for enums, errors and integers (characters too); no
  `switch`, no matching on strings, no `..` (exclusive) range patterns.
- `&`, `|` and `^` bind tighter than `==` (unlike C) and cannot be mixed
  without parentheses; a shift amount has the type of the shifted value.
- `if` is a statement, not an expression: `let x = if c { 1 } else { 2 }`
  is an error. Declare `var x = 2` and assign in the `if`.
- A function with a return type needs a `return` at the end of every path;
  a `while true { }` loop does not count, so add a final `return` after it.
- Enum values are always qualified: `Color.Red`, never `.Red` (patterns use
  `.Red`). Structs, and enums with payloads, have no `==`.
- An error union cannot be stored: `let r = f()` for `f() E!T` is an error;
  write `try f()` or `f() catch ...`. `try` needs the enclosing function to
  return an error union.
- `pub` functions spell their error set: `pub fn f() E!T`, not `!T`.
- `alloc.create(value)` and `alloc.alloc(value, n)` take values, not types
  (`alloc.alloc(0 as u8, n)`, not `alloc.alloc(u8, n)`).
- Pointers are never null; optional pointers are `?*T`. Dereference is
  `p.*` (Zig), not `*p`.
- `dyn I` is made from `&x`, never from a value; it does not box.
- No `@import`, no `const` (a top-level `let` is a constant), no closures,
  no function pointers, no `orelse`, no `defer` with `return` inside.
- Output to stdout is `@print`/`@println` or `std.fs.stdout().write(bytes)`;
  `std.log` writes to stderr.
- `main` is `pub fn main()` or `pub fn main() !void`; arguments come from
  `std.os.args(alloc)`.
- A file argument compiles only that file; give the directory to compile a
  multi-file module.
