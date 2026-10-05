//! Stable diagnostic codes.
//!
//! Every compile error carries one code (`E0001`, ...). This table is the
//! single source of truth: the code, a short title, an explanation, and a
//! complete program with the error together with its fixed version.
//! `ore explain <code>` prints an entry.
//!
//! Codes are stable: never renumber or reuse one. A new kind of error gets
//! the next free number at the end of the table.

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Code(pub u16);

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "E{:04}", self.0)
    }
}

pub struct Info {
    pub code: Code,
    pub title: &'static str,
    /// What the error means and how to fix it.
    pub text: &'static str,
    /// A complete program with this error.
    pub wrong: &'static str,
    /// The same program, fixed.
    pub fix: &'static str,
}

macro_rules! codes {
    ($($name:ident = $num:literal { $title:literal, $text:literal, $wrong:literal, $fix:literal, })*) => {
        $(pub const $name: Code = Code($num);)*
        /// Every code, in numeric order.
        pub const ALL: &[Info] = &[$(Info { code: Code($num), title: $title, text: $text, wrong: $wrong, fix: $fix }),*];
        #[cfg(test)]
        const NAMES: &[&str] = &[$(stringify!($name)),*];
    };
}

/// The text `ore explain` prints for `code` (such as `E0012`).
pub fn explain(code: &str) -> Option<String> {
    let info = ALL.iter().find(|i| i.code.to_string() == code)?;
    let indent = |s: &str| s.lines().map(|l| if l.is_empty() { "\n".to_string() } else { format!("    {l}\n") }).collect::<String>();
    Some(format!(
        "{}: {}\n\n{}\n\nWrong:\n\n{}\nFix:\n\n{}",
        info.code,
        info.title,
        info.text,
        indent(info.wrong),
        indent(info.fix)
    ))
}

codes! {
E0001 = 1 {
    "unexpected character",
    "The source contains a character that is not part of Ore's syntax.",
    r#"pub fn main() {
    let x = 1 # 2
}
"#,
    r#"pub fn main() {
    let x = 1 + 2
}
"#,
}
E0002 = 2 {
    "semicolon",
    "Ore ends a statement at the end of the line (Go's rule). An explicit `;` is an error: remove it, and put each statement on its own line.",
    r#"pub fn main() {
    let x = 1;
}
"#,
    r#"pub fn main() {
    let x = 1
}
"#,
}
E0003 = 3 {
    "invalid number literal",
    "A number literal is malformed: a letter right after the digits, no digits after `0x`, or a value too large (integers up to 2^64-1, floats up to the f64 range).",
    r#"pub fn main() {
    let x = 12abc
}
"#,
    r#"pub fn main() {
    let x = 12
}
"#,
}
E0004 = 4 {
    "invalid string or character literal",
    "A string or character literal is not closed on its line, or a character literal (`'a'`) does not hold exactly one character. Use a string (`\"ab\"`) for more than one.",
    r#"pub fn main() {
    let c = 'ab'
}
"#,
    r#"pub fn main() {
    let s = "ab"
}
"#,
}
E0005 = 5 {
    "invalid escape sequence",
    "Only `\\n`, `\\t`, `\\r`, `\\0`, `\\\\`, `\\\"`, `\\'` and `\\xHH` (two hex digits) are escapes. To write a backslash, use `\\\\`.",
    r#"pub fn main() {
    @println("a\qb")
}
"#,
    r#"pub fn main() {
    @println("a\\qb")
}
"#,
}
E0006 = 6 {
    "syntax error",
    "The parser found a token it did not expect. The message says what was expected. A common cause is a missing `)`, `}` or `,` (a multi-line call or literal needs a trailing comma).",
    r#"pub fn main() {
    let x = (1 + 2
}
"#,
    r#"pub fn main() {
    let x = (1 + 2)
}
"#,
}
E0007 = 7 {
    "not yet supported",
    "The feature is part of Ore but not of the stage0 compiler yet (for example `import`, `extern struct` or global variables), or the keyword is used where stage0 does not support it (`comptime` only marks a type parameter, `comptime T: type`). Write the program without it.",
    r#"import std

pub fn main() {
}
"#,
    r#"pub fn main() {
}
"#,
}
E0008 = 8 {
    "`else` on a new line",
    "A newline after `}` ends the `if` statement, so `else` must be on the same line as the `}` that closes the `if` block.",
    r#"pub fn main() {
    let x = 1
    if x > 0 {
        @println("positive")
    }
    else {
        @println("not positive")
    }
}
"#,
    r#"pub fn main() {
    let x = 1
    if x > 0 {
        @println("positive")
    } else {
        @println("not positive")
    }
}
"#,
}
E0009 = 9 {
    "chained comparison",
    "Comparisons do not chain: `a < b < c` is an error. Combine two comparisons with `&&`.",
    r#"pub fn main() {
    let x = 2
    let ok = 1 < x < 3
}
"#,
    r#"pub fn main() {
    let x = 2
    let ok = 1 < x && x < 3
}
"#,
}
E0010 = 10 {
    "declaration inside a function",
    "`fn`, `struct`, `enum`, `error` and `extern` declarations are only allowed at the top level of a file. Move the declaration out of the function.",
    r#"pub fn main() {
    fn helper() {
    }
}
"#,
    r#"fn helper() {
}

pub fn main() {
    helper()
}
"#,
}
E0011 = 11 {
    "`extern fn` with a body",
    "An `extern fn` declares a C function; its body is in C. Remove the body.",
    r#"extern fn puts(s: [*]u8) i32 {
}

pub fn main() {
    puts("hi".ptr)
}
"#,
    r#"extern fn puts(s: [*]u8) i32

pub fn main() {
    puts("hi".ptr)
}
"#,
}
E0012 = 12 {
    "empty variant payload",
    "An enum variant without fields is written without braces: `Red`, not `Red {}`.",
    r#"enum Color { Red {}, Green }

pub fn main() {
}
"#,
    r#"enum Color { Red, Green }

pub fn main() {
}
"#,
}
E0013 = 13 {
    "missing initializer",
    "`let` and `var` always need an initial value (`= value`). There is no zero-initialization.",
    r#"pub fn main() {
    var x: i32
}
"#,
    r#"pub fn main() {
    var x: i32 = 0
}
"#,
}
E0014 = 14 {
    "builtin name redefined",
    "Primitive type names (`i32`, `u8`, `bool`, ...) and `true`, `false`, `null` cannot be used as the name of a declaration.",
    r#"struct u8 { a: i32 }

pub fn main() {
}
"#,
    r#"struct Byte { a: i32 }

pub fn main() {
}
"#,
}
E0015 = 15 {
    "duplicate definition",
    "A name is defined twice: two top-level declarations, two fields of one struct, two variants of one enum, or two errors of one error set. Rename or remove one.",
    r#"fn helper() {
}

fn helper() {
}

pub fn main() {
    helper()
}
"#,
    r#"fn helper() {
}

fn other() {
}

pub fn main() {
    helper()
}
"#,
}
E0016 = 16 {
    "shadowing",
    "A new binding cannot reuse a name that is already visible: a local of an enclosing block, a parameter, or a top-level name. Pick a different name.",
    r#"pub fn main() {
    let x = 1
    if x > 0 {
        let x = 2
        @println(x)
    }
}
"#,
    r#"pub fn main() {
    let x = 1
    if x > 0 {
        let y = 2
        @println(y)
    }
}
"#,
}
E0017 = 17 {
    "unknown type",
    "The name used as a type (or struct, enum or error set) is not declared. Check the spelling, or declare it at the top level.",
    r#"struct Point { x: i32, y: i32 }

fn show(p: Pont) {
}

pub fn main() {
}
"#,
    r#"struct Point { x: i32, y: i32 }

fn show(p: Point) {
}

pub fn main() {
}
"#,
}
E0018 = 18 {
    "undefined variable",
    "The name is not a variable or parameter visible here. Declare it with `let` or `var` first, or check the spelling.",
    r#"pub fn main() {
    @println(count)
}
"#,
    r#"pub fn main() {
    let count = 3
    @println(count)
}
"#,
}
E0019 = 19 {
    "undefined function",
    "No function with this name is declared. Check the spelling, or declare it (`fn name() { }`, or `extern fn` for a C function).",
    r#"pub fn main() {
    greet()
}
"#,
    r#"fn greet() {
    @println("hi")
}

pub fn main() {
    greet()
}
"#,
}
E0020 = 20 {
    "function used as a value",
    "Functions are not values. Call the function with `name(...)`.",
    r#"fn one() i32 {
    return 1
}

pub fn main() {
    let x = one
}
"#,
    r#"fn one() i32 {
    return 1
}

pub fn main() {
    let x = one()
}
"#,
}
E0021 = 21 {
    "unknown builtin",
    "Stage0 has only the builtins `@print` and `@println`.",
    r#"pub fn main() {
    @printf("hi")
}
"#,
    r#"pub fn main() {
    @println("hi")
}
"#,
}
E0022 = 22 {
    "wrong builtin arguments",
    "`@print(x)` takes one argument and `@println(x)` at most one. They print integers, floats, `bool` and `[]u8` strings; print the fields of a struct one by one.",
    r#"struct P { x: i32 }

pub fn main() {
    let p = P { x: 1 }
    @println(p)
}
"#,
    r#"struct P { x: i32 }

pub fn main() {
    let p = P { x: 1 }
    @println(p.x)
}
"#,
}
E0023 = 23 {
    "mismatched types",
    "A value has a different type than the one expected here. Ore has no implicit conversions: convert numbers with `as`, or change the declared type.",
    r#"pub fn main() {
    let x: i32 = true
}
"#,
    r#"pub fn main() {
    let x: i32 = 1
}
"#,
}
E0024 = 24 {
    "integer literal out of range",
    "An integer literal does not fit in the type it is given. Use a larger type.",
    r#"pub fn main() {
    let b: u8 = 300
}
"#,
    r#"pub fn main() {
    let b: u16 = 300
}
"#,
}
E0025 = 25 {
    "invalid operand type",
    "The operator does not work on this type: arithmetic needs numbers (`%` and the wrapping operators need integers), unary `-` needs a signed integer or a float, and `!` needs `bool`.",
    r#"pub fn main() {
    let n: u32 = 5
    let m = -n
}
"#,
    r#"pub fn main() {
    let n: u32 = 5
    let m = -(n as i64)
}
"#,
}
E0026 = 26 {
    "condition is not `bool`",
    "An `if` or `while` condition must be a `bool`. Compare explicitly (`x != 0`). For an optional, use `if x |v| { }` or `x != null`.",
    r#"pub fn main() {
    let x = 1
    if x {
        @println(x)
    }
}
"#,
    r#"pub fn main() {
    let x = 1
    if x != 0 {
        @println(x)
    }
}
"#,
}
E0027 = 27 {
    "capture on a non-optional",
    "`if x |v| { }` unwraps an optional (`?T`). The value is not optional, so there is nothing to unwrap: use it directly.",
    r#"pub fn main() {
    let x: i32 = 5
    if x |v| {
        @println(v)
    }
}
"#,
    r#"pub fn main() {
    let x: ?i32 = 5
    if x |v| {
        @println(v)
    }
}
"#,
}
E0028 = 28 {
    "invalid use of `null`",
    "Only optional types (`?T`) can be `null`. Without a type annotation the type of `null` is unknown: write `let x: ?T = null`.",
    r#"pub fn main() {
    let p = null
}
"#,
    r#"pub fn main() {
    let p: ?i32 = null
}
"#,
}
E0029 = 29 {
    "wrong number of arguments",
    "A function is called with more or fewer arguments than it has parameters. Ore has no default arguments.",
    r#"fn add(a: i32, b: i32) i32 {
    return a + b
}

pub fn main() {
    @println(add(1))
}
"#,
    r#"fn add(a: i32, b: i32) i32 {
    return a + b
}

pub fn main() {
    @println(add(1, 2))
}
"#,
}
E0030 = 30 {
    "expression has no value",
    "A call to a function without a return type produces no value, so it cannot be stored or used in an expression. Call it as a statement.",
    r#"fn hello() {
    @println("hello")
}

pub fn main() {
    let x = hello()
}
"#,
    r#"fn hello() {
    @println("hello")
}

pub fn main() {
    hello()
}
"#,
}
E0031 = 31 {
    "unused expression",
    "Only calls (and `match`, `try` and `catch`) can be statements. A computed value that is not used is probably a mistake: store it or remove it.",
    r#"pub fn main() {
    let x = 1
    x + 1
}
"#,
    r#"pub fn main() {
    let x = 1
    let y = x + 1
}
"#,
}
E0032 = 32 {
    "immutable binding",
    "A `let` binding (and a parameter, pattern variable or capture) cannot be assigned or borrowed with `&mut`. Declare it with `var`.",
    r#"pub fn main() {
    let x = 1
    x = 2
}
"#,
    r#"pub fn main() {
    var x = 1
    x = 2
}
"#,
}
E0033 = 33 {
    "write through a read-only pointer or slice",
    "Data behind a `*T` pointer or in a slice `[]T` is read-only. Use a `*mut T` (made with `&mut x`) to write through a pointer, and a `[]mut T` (from slicing a `var` array, or from `alloc.alloc`) to write slice elements.",
    r#"struct P { x: i32 }

fn reset(p: *P) {
    p.x = 0
}

pub fn main() {
    var p = P { x: 1 }
    reset(&p)
}
"#,
    r#"struct P { x: i32 }

fn reset(p: *mut P) {
    p.x = 0
}

pub fn main() {
    var p = P { x: 1 }
    reset(&mut p)
}
"#,
}
E0034 = 34 {
    "invalid assignment target",
    "Only variables, fields, array elements and data behind a `*mut` pointer can be assigned. `.len` and `.ptr` are read-only.",
    r#"pub fn main() {
    var xs = [2]i32{1, 2}
    xs.len = 3
}
"#,
    r#"pub fn main() {
    var xs = [2]i32{1, 2}
    xs[0] = 3
}
"#,
}
E0035 = 35 {
    "address of a temporary",
    "`&` (and slicing an array) needs a value stored in a variable or other place. Store the temporary in a variable first.",
    r#"fn one() i32 {
    return 1
}

pub fn main() {
    let p = &one()
}
"#,
    r#"fn one() i32 {
    return 1
}

pub fn main() {
    let v = one()
    let p = &v
}
"#,
}
E0036 = 36 {
    "dereference of a non-pointer",
    "`.*` dereferences a pointer (`*T` or `*mut T`). The value is not a pointer.",
    r#"pub fn main() {
    let x = 1
    let y = x.*
}
"#,
    r#"pub fn main() {
    let x = 1
    let p = &x
    let y = p.*
}
"#,
}
E0037 = 37 {
    "invalid cast",
    "`as` converts only between numeric types. Other conversions are written out, for example with `if`.",
    r#"pub fn main() {
    let flag = true
    let n = flag as i32
}
"#,
    r#"pub fn main() {
    let flag = true
    var n: i32 = 0
    if flag {
        n = 1
    }
}
"#,
}
E0038 = 38 {
    "unknown field",
    "The struct, variant or type has no field with this name. Check the declaration for the right name. Arrays have `.len`; slices have `.len` and `.ptr`.",
    r#"struct P { x: i32, y: i32 }

pub fn main() {
    let p = P { x: 1, y: 2 }
    @println(p.z)
}
"#,
    r#"struct P { x: i32, y: i32 }

pub fn main() {
    let p = P { x: 1, y: 2 }
    @println(p.y)
}
"#,
}
E0039 = 39 {
    "missing field in a literal",
    "A struct or variant literal must give every field a value.",
    r#"struct P { x: i32, y: i32 }

pub fn main() {
    let p = P { x: 1 }
}
"#,
    r#"struct P { x: i32, y: i32 }

pub fn main() {
    let p = P { x: 1, y: 0 }
}
"#,
}
E0040 = 40 {
    "field given twice",
    "A field appears twice in one struct literal or pattern. Give each field once.",
    r#"struct P { x: i32, y: i32 }

pub fn main() {
    let p = P { x: 1, x: 2 }
}
"#,
    r#"struct P { x: i32, y: i32 }

pub fn main() {
    let p = P { x: 1, y: 2 }
}
"#,
}
E0041 = 41 {
    "unknown variant",
    "The enum has no variant with this name. Check the enum declaration.",
    r#"enum Color { Red, Green }

pub fn main() {
    let c = Color.Purple
}
"#,
    r#"enum Color { Red, Green }

pub fn main() {
    let c = Color.Red
}
"#,
}
E0042 = 42 {
    "variant payload mismatch",
    "A variant with fields is written `E.V { field: value }` and matched with `.V { field }` or `.V { .. }`; a variant without fields (and an error) is written `E.V` and matched with `.V`.",
    r#"enum Shape {
    Circle { r: f32 }
    Dot
}

pub fn main() {
    let s = Shape.Circle
}
"#,
    r#"enum Shape {
    Circle { r: f32 }
    Dot
}

pub fn main() {
    let s = Shape.Circle { r: 1.0 }
}
"#,
}
E0043 = 43 {
    "cannot index or slice",
    "Only arrays (`[N]T`) and slices (`[]T`) can be indexed and sliced.",
    r#"pub fn main() {
    let x = 5
    let y = x[0]
}
"#,
    r#"pub fn main() {
    let xs = [1]i32{5}
    let y = xs[0]
}
"#,
}
E0044 = 44 {
    "signed index",
    "Indices and slice bounds must have an unsigned integer type, so they are never negative. Convert a signed value with `as usize`.",
    r#"pub fn main() {
    let xs = [3]i32{1, 2, 3}
    let i: i32 = 1
    @println(xs[i])
}
"#,
    r#"pub fn main() {
    let xs = [3]i32{1, 2, 3}
    let i: i32 = 1
    @println(xs[i as usize])
}
"#,
}
E0045 = 45 {
    "array literal length",
    "An array literal `[N]T{...}` must have exactly N elements.",
    r#"pub fn main() {
    let xs = [3]i32{1, 2}
}
"#,
    r#"pub fn main() {
    let xs = [3]i32{1, 2, 3}
}
"#,
}
E0046 = 46 {
    "non-integer range",
    "The bounds of `for i in lo..hi` must be integers of the same type.",
    r#"pub fn main() {
    for i in 0.0..1.0 {
    }
}
"#,
    r#"pub fn main() {
    for i in 0..10 {
    }
}
"#,
}
E0047 = 47 {
    "`break` or `continue` outside a loop",
    "`break` and `continue` work only inside `while` and `for`, and they cannot leave a `defer` block.",
    r#"pub fn main() {
    break
}
"#,
    r#"pub fn main() {
    while true {
        break
    }
}
"#,
}
E0048 = 48 {
    "`return` or `try` inside `defer`",
    "Deferred code runs while the function is already leaving; it cannot `return` or propagate an error with `try`. Handle errors in deferred code with `catch`.",
    r#"pub fn main() {
    defer {
        return
    }
}
"#,
    r#"pub fn main() {
    defer {
        @println("done")
    }
}
"#,
}
E0049 = 49 {
    "wrong return value",
    "`return` must give a value exactly when the function has a return type (for `E!T`, when T is not `void`). Add the return type, or add or remove the value.",
    r#"fn f() {
    return 1
}

pub fn main() {
    f()
}
"#,
    r#"fn f() i32 {
    return 1
}

pub fn main() {
    f()
}
"#,
}
E0050 = 50 {
    "missing return",
    "A function with a return type must end every path with `return`.",
    r#"fn sign(x: i32) i32 {
    if x < 0 {
        return -1
    }
}

pub fn main() {
    @println(sign(5))
}
"#,
    r#"fn sign(x: i32) i32 {
    if x < 0 {
        return -1
    }
    return 1
}

pub fn main() {
    @println(sign(5))
}
"#,
}
E0051 = 51 {
    "invalid `main`",
    "A program needs `pub fn main()` with no parameters. It returns nothing, or an error union with `void` (`!void`); an error returned from `main` ends the program with status 1.",
    r#"fn main() {
}
"#,
    r#"pub fn main() {
}
"#,
}
E0052 = 52 {
    "type not allowed in `extern fn`",
    "An `extern fn` signature can use only integer, float, `bool` and `[*]T` types. Pass a string as `s.ptr` with type `[*]u8`.",
    r#"extern fn strlen(s: []u8) usize

pub fn main() {
}
"#,
    r#"extern fn strlen(s: [*]u8) usize

pub fn main() {
}
"#,
}
E0053 = 53 {
    "recursive type",
    "A struct or enum that contains itself by value would have infinite size. Hold it through a pointer, such as `?*Node`.",
    r#"struct Node { value: i32, next: ?Node }

pub fn main() {
}
"#,
    r#"struct Node { value: i32, next: ?*Node }

pub fn main() {
}
"#,
}
E0054 = 54 {
    "empty enum or error set",
    "An enum needs at least one variant, and an error set at least one error.",
    r#"enum Empty {}

pub fn main() {
}
"#,
    r#"enum Empty { Only }

pub fn main() {
}
"#,
}
E0055 = 55 {
    "`match` on an unsupported type",
    "`match` works on enum values and error values. Use `if` for other types.",
    r#"pub fn main() {
    let x = 5
    match x {
        _ => @println(x)
    }
}
"#,
    r#"pub fn main() {
    let x = 5
    if x == 5 {
        @println(x)
    }
}
"#,
}
E0056 = 56 {
    "non-exhaustive `match`",
    "A `match` must handle every variant of the enum, or every error of the error set. Add an arm for each missing one (the message lists them), or a `_` arm.",
    r#"enum Color { Red, Green }

fn name(c: Color) []u8 {
    return match c {
        .Red => "red"
    }
}

pub fn main() {
}
"#,
    r#"enum Color { Red, Green }

fn name(c: Color) []u8 {
    return match c {
        .Red => "red"
        .Green => "green"
    }
}

pub fn main() {
}
"#,
}
E0057 = 57 {
    "unreachable `match` arm",
    "An arm can never run: its variant or error is already matched by an earlier arm, or it comes after `_`. Remove it.",
    r#"enum Color { Red, Green }

pub fn main() {
    let c = Color.Red
    match c {
        .Red => @println("red")
        .Green => @println("green")
        .Red => @println("again")
    }
}
"#,
    r#"enum Color { Red, Green }

pub fn main() {
    let c = Color.Red
    match c {
        .Red => @println("red")
        .Green => @println("green")
    }
}
"#,
}
E0058 = 58 {
    "pattern does not list every field",
    "A variant pattern names every field, or ends with `..` to ignore the rest.",
    r#"enum Shape { Rect { w: f32, h: f32 } }

fn width(s: Shape) f32 {
    return match s {
        .Rect { w } => w
    }
}

pub fn main() {
}
"#,
    r#"enum Shape { Rect { w: f32, h: f32 } }

fn width(s: Shape) f32 {
    return match s {
        .Rect { w, .. } => w
    }
}

pub fn main() {
}
"#,
}
E0059 = 59 {
    "block arm in a `match` value",
    "When a `match` produces a value, each arm is an expression, not a block. To run statements, use `match` as a statement and assign or return in the arms.",
    r#"enum Color { Red, Green }

fn code(c: Color) i32 {
    return match c {
        .Red => {
            1
        }
        .Green => 2
    }
}

pub fn main() {
}
"#,
    r#"enum Color { Red, Green }

fn code(c: Color) i32 {
    return match c {
        .Red => 1
        .Green => 2
    }
}

pub fn main() {
}
"#,
}
E0060 = 60 {
    "unknown error",
    "The error set has no error with this name. Check the `error` declaration, or add the error to it.",
    r#"error ParseError { Empty, InvalidDigit }

fn check(s: []u8) ParseError!void {
    if s.len == 0 {
        return ParseError.Missing
    }
}

pub fn main() {
}
"#,
    r#"error ParseError { Empty, InvalidDigit }

fn check(s: []u8) ParseError!void {
    if s.len == 0 {
        return ParseError.Empty
    }
}

pub fn main() {
}
"#,
}
E0061 = 61 {
    "unhandled error",
    "A call that returns an error union (`E!T`) cannot be used as a plain value or ignored. Handle the error: `try f()` passes it on to the caller, `f() catch |e| { ... }` handles it here, and `f() catch value` replaces it with a value.",
    r#"error E { Failed }

fn f() E!u32 {
    return 1
}

pub fn main() {
    let x = f()
}
"#,
    r#"error E { Failed }

fn f() E!u32 {
    return 1
}

pub fn main() {
    let x = f() catch 0
}
"#,
}
E0062 = 62 {
    "`try` outside an error-returning function",
    "`try` returns the error to the caller, so the function itself must return an error union (`E!T`, or `!T` for a non-`pub` function). Otherwise, handle the error with `catch`.",
    r#"error E { Failed }

fn f() E!u32 {
    return 1
}

fn g() u32 {
    return try f()
}

pub fn main() {
}
"#,
    r#"error E { Failed }

fn f() E!u32 {
    return 1
}

fn g() u32 {
    return f() catch 0
}

pub fn main() {
}
"#,
}
E0063 = 63 {
    "`try` or `catch` on a value that is not an error union",
    "`try` and `catch` work only on error unions (`E!T`). An optional (`?T`) is unwrapped with `if x |v| { }`; any other value needs neither.",
    r#"error E { Failed }

fn find() ?u32 {
    return null
}

fn f() E!u32 {
    let x = try find()
    return x
}

pub fn main() {
}
"#,
    r#"error E { Failed }

fn find() ?u32 {
    return null
}

fn f() E!u32 {
    if find() |x| {
        return x
    }
    return E.Failed
}

pub fn main() {
}
"#,
}
E0064 = 64 {
    "error not in the function's error set",
    "A function with an explicit error set `E!T` can return only errors of `E`, by `return` or `try`. Add the error to the set, merge sets with `error All = A || B`, or handle the error with `catch`.",
    r#"error ReadError { NotFound }
error ParseError { Empty }

fn read() ReadError!u32 {
    return ReadError.NotFound
}

fn parse() ParseError!u32 {
    return try read()
}

pub fn main() {
}
"#,
    r#"error ReadError { NotFound }
error ParseError { Empty }
error LoadError = ReadError || ParseError

fn read() ReadError!u32 {
    return ReadError.NotFound
}

fn parse() LoadError!u32 {
    return try read()
}

pub fn main() {
}
"#,
}
E0065 = 65 {
    "`pub` function with an inferred error set",
    "A `pub` function must spell out its error set (`E!T`), so that callers can see which errors to handle without reading its body. `!T` is only for functions that are not `pub` (and for `pub fn main() !void`).",
    r#"error E { Failed }

pub fn f() !u32 {
    return E.Failed
}

pub fn main() {
}
"#,
    r#"error E { Failed }

pub fn f() E!u32 {
    return E.Failed
}

pub fn main() {
}
"#,
}
E0066 = 66 {
    "error union outside a return type",
    "Error union types (`E!T`, `!T`) are only allowed as the return type of a function. Handle the error before storing or passing the value.",
    r#"error E { Failed }

fn show(x: E!u32) {
}

pub fn main() {
}
"#,
    r#"error E { Failed }

fn show(x: u32) {
}

pub fn main() {
}
"#,
}
E0067 = 67 {
    "`catch` block without a value",
    "When the call returns a value (`E!T` with T not `void`), a `catch` block must leave with `return`, `break` or `continue`. To give a replacement value instead, write `catch <expression>`, for example `catch 0` or `catch |e| match e { ... }`.",
    r#"error E { Failed }

fn f() E!u32 {
    return 1
}

pub fn main() {
    let x = f() catch |e| {
        @println("failed")
    }
}
"#,
    r#"error E { Failed }

fn f() E!u32 {
    return 1
}

pub fn main() {
    let x = f() catch |e| {
        @println("failed")
        return
    }
}
"#,
}
E0068 = 68 {
    "`errdefer` in a function that cannot fail",
    "`errdefer` runs only when the function returns an error. In a function that does not return an error union it would never run: use `defer`.",
    r#"fn f() u32 {
    errdefer @println("cleanup")
    return 1
}

pub fn main() {
}
"#,
    r#"fn f() u32 {
    defer @println("cleanup")
    return 1
}

pub fn main() {
}
"#,
}
E0069 = 69 {
    "recursive inferred error set",
    "The error set of a `!T` function is inferred from its body, which is not possible when the function calls itself, directly or through other `!T` functions. Write the error set: `E!T`.",
    r#"error E { Zero }

fn count(n: u32) !u32 {
    if n == 0 {
        return E.Zero
    }
    return count(n - 1)
}

pub fn main() {
}
"#,
    r#"error E { Zero }

fn count(n: u32) E!u32 {
    if n == 0 {
        return E.Zero
    }
    return count(n - 1)
}

pub fn main() {
}
"#,
}
E0070 = 70 {
    "invalid error set",
    "The left side of `!` in `E!T` and every part of `error All = A || B` must be an error set (declared with `error`), and a merged set cannot be defined in terms of itself.",
    r#"error E { Failed }
enum Shape { Circle }
error All = E || Shape

pub fn main() {
}
"#,
    r#"error E { Failed }
error F { Lost }
error All = E || F

pub fn main() {
}
"#,
}
E0071 = 71 {
    "unknown method",
    "The type has no method with this name. Methods are the functions declared in a struct body; `dyn I` has the methods of interface `I`; `Allocator` has `create`, `destroy`, `alloc` and `free`, and `General` and `Arena` have `init`, `allocator` and `deinit`. Check the spelling, or declare the method in the struct.",
    r#"struct Counter {
    n: i32
}

pub fn main() {
    var c = Counter { n: 0 }
    c.bump()
}
"#,
    r#"struct Counter {
    n: i32

    fn bump(self: *mut Self) {
        self.n += 1
    }
}

pub fn main() {
    var c = Counter { n: 0 }
    c.bump()
}
"#,
}
E0072 = 72 {
    "interface not implemented",
    "A pointer to a struct converts to `dyn I` only when the struct has every method of interface `I`, with the same parameter and return types; no declaration is needed. A method that takes `self: *mut Self` needs a `*mut` pointer (`&mut x`) and an interface method declared with `self: *mut Self`.",
    r#"interface Shape {
    fn area(self: *Self) f64
}

struct Square {
    side: f64
}

fn show(s: dyn Shape) {
    @println(s.area())
}

pub fn main() {
    var sq = Square { side: 2.0 }
    show(&sq)
}
"#,
    r#"interface Shape {
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
    var sq = Square { side: 2.0 }
    show(&sq)
}
"#,
}
E0073 = 73 {
    "invalid method receiver",
    "`x.f()` calls a method whose first parameter is `self: Self`, `self: *Self` or `self: *mut Self`; a function in a struct without such a parameter is called as `Type.f()`. An interface method must take `self: *Self` or `self: *mut Self` first.",
    r#"struct Counter {
    n: i32

    fn new() Self {
        return Self { n: 0 }
    }
}

pub fn main() {
    var c = Counter { n: 1 }
    let d = c.new()
}
"#,
    r#"struct Counter {
    n: i32

    fn new() Self {
        return Self { n: 0 }
    }
}

pub fn main() {
    let d = Counter.new()
}
"#,
}
E0074 = 74 {
    "type used as a value",
    "A type name appears where a value is expected. Write a value of the type instead, such as a struct literal `Node { ... }`. `alloc.create` takes the initial value: `alloc.create(Node { ... })`, not `alloc.create(Node)`.",
    r#"struct Point { x: i32, y: i32 }

pub fn main() {
    let p = Point
}
"#,
    r#"struct Point { x: i32, y: i32 }

pub fn main() {
    let p = Point { x: 0, y: 0 }
}
"#,
}
E0075 = 75 {
    "test builtin outside a `test` block",
    "`@testAllocator()` is the leak-checked allocator of a `test` block, so it can only be used in one. Other functions take an `Allocator` parameter; pass it the test allocator.",
    r#"fn make(n: i64) AllocError!*mut i64 {
    return @testAllocator().create(n)
}

test "make" {
    let p = try make(1)
}
"#,
    r#"fn make(alloc: Allocator, n: i64) AllocError!*mut i64 {
    return alloc.create(n)
}

test "make" {
    let alloc = @testAllocator()
    let p = try make(alloc, 1)
    defer alloc.destroy(p)
}
"#,
}
E0076 = 76 {
    "invalid generic declaration",
    "A `comptime` parameter has type `type` (`comptime T: type`); stage0 has no other compile-time values. A type function (returning `type`) takes only `comptime` parameters, and only top-level functions have them. A `where` constraint names a `comptime` parameter and an interface (`where T: Shape`). A generic function spells out its error set (`E!T`).",
    r#"fn first(comptime n: i64, xs: []i64) i64 {
    return xs[0]
}

pub fn main() {
}
"#,
    r#"fn first(comptime T: type, xs: []T) T {
    return xs[0]
}

pub fn main() {
}
"#,
}
E0077 = 77 {
    "invalid type function",
    "The body of a type function (a function that returns `type`) is a single `return struct { ... }`. The struct's fields and methods can use the `comptime` parameters, and `Self` names the struct. An anonymous `struct { ... }` is allowed nowhere else; declare a named struct with `struct Name { ... }`.",
    r#"fn Box(comptime T: type) type {
    let size = 1
    return struct {
        value: T
    }
}

pub fn main() {
}
"#,
    r#"fn Box(comptime T: type) type {
    return struct {
        value: T
    }
}

pub fn main() {
}
"#,
}
E0078 = 78 {
    "type parameter used without a constraint",
    "A generic is checked once, where it is defined, with its `comptime` types unknown. A value of such a type `T` can be moved, copied and pointed to, but it has no fields, operators or methods. A method can be called when a constraint `where T: I` names an interface `I` that declares it; every instantiation must then implement `I`.",
    r#"interface Area {
    fn area(self: *Self) f64
}

fn total(comptime T: type, items: []T) f64 {
    var sum = 0.0
    for i in 0..items.len {
        sum += items[i].area()
    }
    return sum
}

pub fn main() {
}
"#,
    r#"interface Area {
    fn area(self: *Self) f64
}

fn total(comptime T: type, items: []T) f64 where T: Area {
    var sum = 0.0
    for i in 0..items.len {
        sum += items[i].area()
    }
    return sum
}

pub fn main() {
}
"#,
}
E0079 = 79 {
    "unsatisfied `where` constraint",
    "A generic is instantiated with a type that does not implement the interface of one of its `where` constraints. Give the type the missing method, with the interface's signature, or use another type. A generic that instantiates another one with its own parameter needs the same constraint on that parameter.",
    r#"interface Area {
    fn area(self: *Self) f64
}

struct Square {
    side: f64
}

fn twice(comptime T: type, x: T) f64 where T: Area {
    return x.area() * 2.0
}

pub fn main() {
    let a = twice(Square, Square { side: 2.0 })
}
"#,
    r#"interface Area {
    fn area(self: *Self) f64
}

struct Square {
    side: f64

    fn area(self: *Self) f64 {
        return self.side * self.side
    }
}

fn twice(comptime T: type, x: T) f64 where T: Area {
    return x.area() * 2.0
}

pub fn main() {
    let a = twice(Square, Square { side: 2.0 })
}
"#,
}
E0080 = 80 {
    "invalid type arguments",
    "A type function names a type only with its arguments, `Box(i64)`, and only a type function takes type arguments. The argument of a `comptime` parameter must be a type, and the number of arguments must match the parameters.",
    r#"fn Box(comptime T: type) type {
    return struct {
        value: T
    }
}

fn unbox(b: Box) i64 {
    return 0
}

pub fn main() {
}
"#,
    r#"fn Box(comptime T: type) type {
    return struct {
        value: T
    }
}

fn unbox(b: Box(i64)) i64 {
    return b.value
}

pub fn main() {
}
"#,
}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver;

    #[test]
    fn codes_are_numbered_in_order() {
        for (i, info) in ALL.iter().enumerate() {
            assert_eq!(info.code.0 as usize, i + 1);
            assert_eq!(info.code.to_string(), NAMES[i]);
        }
    }

    /// Each wrong example reports its code, and each fix compiles.
    #[test]
    fn examples_are_accurate() {
        for info in ALL {
            match driver::check("wrong.ore", info.wrong) {
                Ok(_) => panic!("{}: the wrong example compiles", info.code),
                Err(ds) => assert!(
                    ds.iter().any(|d| d.code == info.code),
                    "{}: the wrong example reports {:?}",
                    info.code,
                    ds.iter().map(|d| d.to_string()).collect::<Vec<_>>()
                ),
            }
            if let Err(ds) = driver::check("fix.ore", info.fix) {
                panic!("{}: the fix does not compile: {:?}", info.code, ds.iter().map(|d| d.to_string()).collect::<Vec<_>>());
            }
        }
    }

    #[test]
    fn explain_formats_an_entry() {
        let text = explain("E0002").unwrap();
        assert!(text.starts_with("E0002: semicolon\n\n"), "{text}");
        assert!(text.contains("Wrong:\n\n    pub fn main() {\n        let x = 1;\n    }\n"), "{text}");
        assert!(explain("E9999").is_none());
    }
}
