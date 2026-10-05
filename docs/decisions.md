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
- **`pub`** is accepted on `fn`, `struct`, `enum` and `error` but has no effect until modules exist (apart from the error set rule below).
- **`main`:** the entry point must be `pub fn main()`, with no parameters, and with no return type or an error union with `void` (`!void`, `E!void`). The generated C `main` returns 0, or 1 after reporting an error that `main` returned.
- **Pointers:** `*T` is read-only, `*mut T` is writable. They are made with `&x` and `&mut x`.
  - `&mut x` needs a writable place: a `var`, or data behind a `*mut`. `&` of a temporary (`&f()`) is an error.
  - Dereference is `p.*`, as in Zig (`*` already starts the type).
  - Field access dereferences one pointer level, as in Go and Zig: `p.x`, and `p.x = 1` when `p: *mut T`.
  - There is no pointer arithmetic or pointer comparison.
- **Optionals:** `?T`, with the literal `null`. `null` is a predeclared identifier like `true`, so the keyword count stays at 28.
  - `if x |v| { } else { }` unwraps, as in Zig. `v` is immutable and visible only in the first block. `while x |v|` is not supported yet.
  - `x == null` and `x != null` work, as in Zig (and Go's `nil`).
- **Enums:** `enum Name { A, B { field: T, ... } }`. Variants are separated by `,` or newlines; payload fields are written like struct fields. A variant without payload has no `{}`.
  - A value always names the enum: `Shape.Circle { r: 1.0 }`, `Color.Red`. The shorthand `.Circle { }` is not supported, so there is one way to write it.
- **`match`:** `match x { pattern => body }`. Arms are separated by newlines or `,`. The scrutinee follows the header rule of `if` (no bare struct literal).
  - Patterns: `.V`, `.V { a, b: name, .. }` and `_`. A field binds a variable of the same name, or `field: name`. Without `..`, every field must be listed.
  - Matching a variant twice, and any arm after `_`, is an error.
  - As a statement, an arm body is a block or a call. As an expression, it is an expression (not a block), and all arms must have the same type. With no expected type, the first arm that is not an untyped literal or `null` decides the type.
- **New tokens:** `&`, `|`, `=>` and `.*`. `.*` ends a statement for semicolon insertion, like `)`.
- **Error sets:** `error Name { A, B }` at top level, with names separated by `,` or newlines. A merged set is a declaration too, `error All = A || B`, as in Zig's `const All = A || B`; `||` is not a type operator, so every set has a name that `rg "error All"` finds.
- **Error values** are written `Set.Name`, like enum values (`return ParseError.Empty`). In `match e`, an arm is `.Name`, like an enum pattern.
- **Error unions:** `E!T`, and `!T` with an inferred set. They are allowed only as a function's return type, not for parameters, fields or variables. `void` is a type name only as their payload (`!void`).
- **`try x`** is a prefix operator with the precedence of unary operators, as in Zig. It works only on error unions; `?T` is unwrapped with `if x |v|`.
- **`catch`:** `x catch |e| { block }` or `x catch |e| expr`; the capture `|e|` is optional. `catch` binds looser than `+` and tighter than comparisons, as in Zig: `f() catch 0 + 1 == 2` is `(f() catch (0 + 1)) == 2`.
  - With a block, the block runs on an error. If the payload `T` is not `void`, the block must end with `return`, `break` or `continue` on every path.
  - With an expression, the expression replaces the error: it must have type `T` (`f() catch 0`, `f() catch |e| match e { ... }`). If `T` is `void`, it must be a call.
- **`errdefer`** takes a block or a simple statement, like `defer`. It is an error in a function that does not return an error union, because it would never run.
- **Methods** are functions declared in a struct body (`fn` or `pub fn`), in any order with the fields. In the struct, `Self` names it. Enums have no methods yet.
  - The first parameter decides how a method takes its receiver, as in Zig: `self: Self` (a copy), `self: *Self` or `self: *mut Self`. The name `self` is a convention, not a rule. A function without such a parameter is called only as `Type.f(args)`.
  - `x.m(args)` passes the receiver as the first argument. For `*Self` and `*mut Self`, a pointer `x` is passed as it is, and a place `x` is passed as `&x` or `&mut x`; `&mut` needs a writable place, as for `&mut x` itself. A temporary is never borrowed implicitly (`make().get()` with `self: *Self` is an error). For `self: Self`, a pointer receiver is dereferenced (copied).
  - `Type.f(args)` calls any function of the struct with all arguments written out, also a method: `Counter.get(&c)`.
  - Method names are scoped to their struct: a method can share its name with a top-level function, but not with a field of its struct. Its parameters follow the shadowing rule.
- **Interfaces:** `interface Name { fn m(self: *Self, ...) T }`, one method signature per line, without bodies. The first parameter must be `self: *Self` or `self: *mut Self`, and an error union return type must name its set (`E!T`).
- **`dyn I`** is a type. A `dyn` value is made only by an implicit conversion: a `*T` or `*mut T` where `dyn I` is expected (`show(&mut sq)`, `let s: dyn Shape = &c`).
- **Test blocks:** `test "name" { ... }` at the top level. The body is checked like a `!void` function, so it can use `try`. Test names in a file must differ.
  - `ore build`, `ore run` and `ore emit-c` leave test blocks out (they are still type-checked) and need `pub fn main()`. `ore test` does not need `main`. `ore check` needs `main` unless the file has a test block.

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
- **Equality:** `==` and `!=` work on integers, floats and bools, and between an optional and `null`. Struct, array, slice, pointer and enum equality are not supported.
- **Unused call results:** an expression statement must be a call (or `try`, `catch` or `match`), and a non-void result may be dropped silently. Other expression statements are errors.
- **Error unions must be handled:** a call that returns `E!T` must be the operand of `try` or `catch`, or be returned as is (`return f()`) from a function whose error set has all its errors. It cannot be stored, passed, used in an expression, or dropped as a statement.
- **Implicit wrapping:** a `T` value is accepted where `?T` is expected (a typed `let`, an assignment, an argument, a `return`, a field). This wraps the value; it is not a conversion between types, so it does not break the "no implicit conversion" rule. `*mut T` is also accepted where `*T` is expected, as in Rust and Zig. There are no other implicit coercions.
- **`match` is exhaustive:** without `_`, every variant must have an arm. The error names the missing variants (`missing variant `Shape.Rect``). `match` works on enums and on error values; on errors, every error of the set needs an arm (`missing error `ParseError.Overflow``).
- **An error is identified by its name**, as in Zig: the same name in two sets is the same error. Every error name gets a program-wide value (a `u16`; 0 means no error) in order of first declaration. A merged set `A || B` has the errors of both, and an error value converts to any set that has all of its errors (`ParseError` to `LoadError`). A literal `Set.Name` counts as only that error.
- **Error set checking:** in an `E!T` function, every error that a `return` or a `try` can pass on must be in `E`. The inferred set of a `!T` function is the union of those errors; `return ParseError.Empty` adds only `Empty`. A `!T` function is checked before its callers, which need its set, so a `!T` function that calls itself, directly or through other `!T` functions, is an error.
- **`pub` functions spell out their error set** (`E!T`). `!T` is only for functions that are not `pub`, and for `pub fn main() !void`.
- **Diagnostic codes:** every diagnostic has a code (`E0001`, ...) from the table in `src/codes.rs`: a title, an explanation, a program with the error and the fixed program. A unit test checks that each wrong program reports its code and each fix compiles. Codes are never renumbered or reused; a new kind of error gets the next number. `ore explain <code>` prints an entry.
- **Pattern bindings and `|v|` captures** are immutable copies, and they follow the shadowing rule below.
- **Shadowing** is not allowed in any scope. A `let`/`var` binding, a parameter or a `for` variable is an error when its name is already visible: a local in the same or an enclosing block, a parameter of the function, or a top-level `fn`, `extern fn` or `struct`. Sibling blocks and different functions can reuse a name.
- **`isize` and `usize`** are 64-bit. Stage0 assumes an LP64 target.
- **Interface conformance is structural**, as in Go: a struct implements `I` when it has every method of `I`, with the same parameter types after `self` and the same return type. There is no `impl` declaration. It is checked where a pointer converts to `dyn I`, and the error names the missing or different method.
  - A method that takes `self: *mut Self` needs an interface method declared with `self: *mut Self`, and a `*mut T` (`&mut x`). Methods that take `self: *Self` or `self: Self` work from `&x` too.
  - Calls through a `dyn` need no mutability: whether it was made from `&mut` was checked at the conversion.
- **Allocators without generics:** `Allocator`, `General`, `Arena` and the error set `AllocError { OutOfMemory }` are predeclared names, like `i32`; they cannot be redefined. They move into `std.mem` and `std.heap` with modules (M5).
  - `Allocator` is a fat pointer like `dyn`: a data pointer and a vtable of byte-level `alloc` and `free` functions. It is not `dyn` of a user-visible interface, because its typed methods take a type or any value, which an interface method cannot express without generics. Only `General` and `Arena` make one (`gpa.allocator()`); Ore code cannot implement it yet. `resize` waits for a container that grows.
  - `General.init()`, `Arena.init(backing)`; `x.allocator()` and `x.deinit()` take `self: *mut Self`, so the variable must be a `var`.
  - `alloc.create(value)` returns `AllocError!*mut T`, with the memory set to `value`. Ore has no uninitialized variables (`let` and `var` need an initializer), so heap memory starts initialized too. `alloc.create(Node)`, with a type, is an error that says to pass the initial value.
  - `alloc.destroy(p)` takes a `*T` or `*mut T`.
  - Not in M3: `alloc.alloc(T, n)`, `alloc.free(slice)` and `[]mut T`. They need a rule for the initial value of each element, and with M4's generics they can be ordinary std functions on the byte-level vtable.
- **Test builtins:** `@expect(cond)` and `@expectEqual(expected, actual)`, in Zig's argument order. `@expectEqual` compares integers, floats, `bool` and `[]u8` (by bytes); a literal takes the type of the other side. They can be used in any function.
  - `@testAllocator()` is the leak-checked allocator of the running test. It can be used only in a `test` block; other functions take an `Allocator` parameter.
- **`extern fn` signatures** can use only integer, float, `bool` and `[*]T` types, and the function cannot be variadic. Calls go straight to the C symbol of the same name.
- **Error recovery:** every stage reports all of its errors, sorted by position and deduplicated, at most 50.
  - The lexer skips the rest of a bad line.
  - The parser skips a bad statement up to the end of its line (using brace depth and indentation), and a bad declaration up to the next line that starts with a declaration keyword.
  - If the lexer or parser reported errors, sema does not run, as in Go. A syntax error never causes type errors.
  - Sema drops a statement with an error and goes on. A variable whose type is unknown (bad initializer, unknown type, rejected declaration) gets an error type, and its uses are not reported again.

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
- **Optional layout:** `?*T` is a C pointer that may be null, the same size as `*T`. Every other `?T` is `struct { bool has; T val; }`.
- **Enum layout:** a `uint32_t` tag (the variant's index in the declaration) and a union with one struct per payload variant. `match` is lowered to an `if` chain on the tag; the last arm needs no test.
- **Error union layout:** `struct { uint16_t err; T val; }`, where `err` is 0 for no error; `!void` has only `err`. Unions with the same payload share one C type. An error set value is a `uint16_t`.
- **`errdefer` timing:** an `errdefer` runs, in LIFO order with the `defer`s, when its scope is left by a `return` of an error, by a `return f()` whose union holds an error, or by a `try` that passes an error on. It does not run at the normal end of a block, on `break` or `continue`, or on a `return` without an error.
- **Error return trace (debug mode only):** every `return` of an error and every `try` that passes one on adds its position. When `catch` handles an error, that is, its body ends normally or leaves with `break`, `continue` or a `return` without an error, the entries added since the `catch` started are dropped. Up to 64 entries are kept.
- **An error returned from `main`** flushes stdout, writes `error: <Name>` to stderr, and in debug mode then `error return trace:` and one `file:line:col` line per entry, where the error started first. The exit status is 1. Safe and fast mode record no trace, so they print only the first line.
- **Recursive types:** a struct or enum that contains itself by value (also through an array or `?T`) is an error. Through a pointer it is fine.
- **`#line` directives** come before every generated statement so that debug info points to the `.ore` source.
- **C names:** a function is `ore_f_name`, a method `ore_m_<len><Struct>_<method>` (the length keeps the names apart), and test `N` is `ore_t_N`.
- **`dyn I` layout:** `struct { void *ptr; const struct ore_VT_I *vt; }`. The vtable has one function pointer per interface method, in declaration order. Each (interface, struct) pair that is converted gets one static vtable of thunks; a thunk casts `ptr` back to the struct pointer (or copies the value for `self: Self`) and calls the method.
- **`General`** calls libc `malloc` and `free`.
  - In debug mode it records each allocation with the position of the `create` call. Freed memory is kept until `deinit`, so an address is never reused and every double free is caught: it panics at the second `destroy` with `double free of memory allocated at F:L:C and already freed at F:L:C`. Freeing memory that this allocator did not allocate also panics.
  - `deinit` writes `file:line:col: leak: N bytes allocated here were never freed` to stderr for each allocation that was not freed, in allocation order, then frees all memory. A leak does not change the exit status of a program; under `ore test` it fails the test.
  - In safe and fast mode there are no records: `destroy` is `free`, `deinit` does nothing, and a double free is undefined behavior. Use-after-free is not detected in any mode.
- **`Arena`** takes chunks of 4096 bytes (more for a larger value) from its backing allocator and bumps a pointer in the newest chunk. `destroy` does nothing; `deinit` gives every chunk back. A leaked arena shows up in its backing allocator as chunks allocated at the `create` that needed them.
- Alignment is at most 16 bytes, which `malloc` gives. When an allocator returns no memory, `create` returns `AllocError.OutOfMemory`.
- **`ore test`** compiles all tests of the file into one executable whose argument is the index of the test to run, and runs each selected test in its own process. So a panic, a failed assertion or a crash ends only that test.
  - A failed assertion writes `file:line:col: assertion failed: <reason>` to stderr and exits with status 1. The reason is `condition is false` or `expected X, found Y`.
  - After the test body returns, a returned error is reported as `main` reports it (with the error return trace in debug mode), and then the leaks of `@testAllocator()`. The test allocator records allocations in every mode.
  - A test passes when its process exits with status 0. The output format and the `--json` schema are in the README.

## Differences from the design doc (to resolve later)

- The doc's examples use `pub fn main() !void`. Stage0 also accepts `pub fn main()` with no return type.
- The doc shows structs only as anonymous type expressions (`struct { ... }`, returned from type functions) with newline-separated fields. Stage0 uses a named declaration, `struct Name { ... }`. If type functions arrive, this might become `let Name = struct { ... }`.
- The doc shows `defer` only with a call (`defer f.close()`). Stage0 also accepts an assignment or a block.
- The doc has no `@` builtins. Its logging goes through `std.log`, and `std` does not exist yet. `@print` and `@println` are a stage0-only convenience until `std` lands.
- The doc writes `std.heap.General` and `import std.mem.Allocator`. Stage0 predeclares `General`, `Arena` and `Allocator` until modules and `std` exist (M5).
- The doc's `Allocator` interface has `alloc`, `resize` and `free`. Stage0's vtable has `alloc` and `free`, and Ore code uses the typed `create` and `destroy`.
- The doc says `General` detects use-after-free in debug mode. Stage0 detects leaks and double frees only.
- The doc says tests use a debug allocator by default. Stage0 spells it `@testAllocator()`, a builtin, until `std.testing` exists. Examples in doc comments do not run as tests yet.
