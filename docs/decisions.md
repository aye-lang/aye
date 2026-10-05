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
- **`pub`** on `fn`, `struct`, `enum`, `error`, `interface` and on methods makes them visible to other modules (see Modules below); it also decides the error set rule below.
- **`main`:** the entry point must be `pub fn main()` in the root module, with no parameters, and with no return type or an error union with `void` (`!void`, `E!void`). The generated C `main` stores `argc`/`argv` for `std.os.args`, and returns 0, or 1 after reporting an error that `main` returned.
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
- **Generics:** a parameter `comptime T: type` takes a type, and a function with one is generic. A function whose return type is `type` is a type function; its body is exactly `return struct { ... }`, an anonymous struct with fields and methods (`Self` is the struct). `struct { ... }` is allowed nowhere else, so every named type still has a declaration that `rg` finds (`struct Name` or `fn Name(...) type`).
  - Only top-level functions have `comptime` parameters: not methods, interface methods or `extern fn`. A type function has only `comptime` parameters.
  - `where` follows the return type, as in Rust: `fn f(comptime T: type, x: T) i64 where T: Show {`. Each constraint is `Param: Interface`; two interfaces are written `T: A, T: B` (there is no `+`).
  - A call passes the type like any argument: `max(i64, a, b)`. `Pool(Node)` is a type wherever a type is written, and also before `.f(...)` and `{` in an expression: `Pool(Node).init(alloc, 16)`, `Handle(T) { index: i, generation: g }`. A type argument that does not read as an expression is written as a type: `f(*Node, p)`, `f([]u8, s)`.
  - There are no type aliases: `let P = Pool(Node)` is an error (a type is not a value).
- **The empty slice** is `[]T{}` (or `[]mut T{}`): a null pointer and length 0. A slice literal cannot have elements, because they would need storage that lives somewhere: put them in an array and slice it, or use `alloc.alloc`.
- **Test blocks:** `test "name" { ... }` at the top level. The body is checked like a `!void` function, so it can use `try`. Test names in a file must differ.
  - `ore build`, `ore run` and `ore emit-c` leave test blocks out (they are still type-checked) and need `pub fn main()`. `ore test` does not need `main`. `ore check` needs `main` unless the file has a test block.

## Types and checking

- **Untyped literals:** an integer or char literal takes the type that the context expects. An integer literal can also become a float type. With no context, an integer literal is `i64` and a float literal is `f64`. In a binary operation, a literal operand takes the type of the other operand. A literal that does not fit its type is a compile error.
- **Char literals** (`'a'`) are untyped integer constants whose value is the Unicode code point.
- **String literals** have type `[]u8`. They are UTF-8, and the bytes are followed by a NUL that `len` does not count, so `"text".ptr` can be passed to C as `[*]u8`.
- **Index and slice bounds** must have an unsigned integer type (`u8` to `u64`, `usize`). Untyped literals become `usize`. Signed indices need `as`, so a negative index is never possible. The design doc's examples index with a `u32`.
- **Slice elements are read-only unless the slice type says `mut`.** `[]T` elements cannot be assigned or borrowed with `&mut`; `[]mut T` elements can, wherever the slice is stored (a `let` slice, a field behind a `*T`): the slice type decides, as a pointer type does. So `*Self` stays shallow: it forbids `self.slots = ...` but not `self.slots[i] = ...` when `slots` is a `[]mut T`. Array elements and struct fields can be assigned when the root binding is `var`.
  - A `[]mut T` is accepted where a `[]T` is expected, like `*mut T` for `*T`.
  - Slicing an array gives a `[]mut T` when the array could be borrowed with `&mut` (a `var`, or data behind a `*mut`), as in Zig; otherwise a `[]T`. Slicing a slice keeps its type. String literals are `[]u8`.
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
- **Allocators:** `Allocator` and the error set `AllocError { OutOfMemory }` are items of `std.mem`, and `General` and `Arena` of `std.heap` (they were predeclared before M5). They are built into the compiler, not declared in std's Ore source, for the reasons below; they are used like any std item, after `import std.mem.Allocator` or as `std.heap.General` after `import std`. Messages name them by path (`std.mem.Allocator`). Using one of these four names without an import (and without declaring it) is E0084, with a fix-it that adds the import; a module may declare its own `Allocator`.
  - `Allocator` is a fat pointer like `dyn`: a data pointer and a vtable of byte-level `alloc` and `free` functions. It is not `dyn` of a user-visible interface, because its typed methods take a type or any value, which an interface method cannot express without generics. Only `General` and `Arena` make one (`gpa.allocator()`); Ore code cannot implement it yet. `resize` waits for a container that grows.
  - `General.init()`, `Arena.init(backing)`; `x.allocator()` and `x.deinit()` take `self: *mut Self`, so the variable must be a `var`.
  - `alloc.create(value)` returns `AllocError!*mut T`, with the memory set to `value`. Ore has no uninitialized variables (`let` and `var` need an initializer), so heap memory starts initialized too. `alloc.create(Node)`, with a type, is an error that says to pass the initial value.
  - `alloc.destroy(p)` takes a `*T` or `*mut T`.
  - `alloc.alloc(value, n)` returns `AllocError![]mut T` with `n` elements, each set to a copy of `value`, for the same reason as `create(value)`: memory starts initialized. `n` is an unsigned integer. `alloc.alloc(T, n)`, with a type, is an error that says to pass the value. `n == 0` gives the empty slice without allocating.
  - `alloc.free(s)` takes a `[]T` or `[]mut T` from `alloc.alloc`; freeing an empty slice does nothing. Freeing a slice that does not start an allocation (`s[1..]`) is freeing memory the allocator did not allocate.
  - `alloc.alloc` and `alloc.free` stay allocator intrinsics: an interface method cannot take a type or any value, and `Allocator` is not an interface (see above). `resize` waits for a container that grows in place.
- **Test builtins:** `@expect(cond)` and `@expectEqual(expected, actual)`, in Zig's argument order. `@expectEqual` compares integers, floats, `bool` and `[]u8` (by bytes); a literal takes the type of the other side. They can be used in any function.
  - `@testAllocator()` is the leak-checked allocator of the running test. It can be used only in a `test` block; other functions take an `Allocator` parameter.
- **`extern fn` signatures** can use only integer, float, `bool` and `[*]T` types, and the function cannot be variadic. Calls go straight to the C symbol of the same name (the name without its module). Modules that declare the same C function must give it the same signature; one C declaration is emitted for it.
- **Slicing a `[*]T`** needs both bounds, `p[lo..hi]`, and gives a `[]T` (read-only elements). Only `lo <= hi` is checked: there is no length. It turns a C string or buffer into a slice (std uses it for `std.os.args`), as in Zig.
- **Generics are checked where they are defined** (the open question in the design doc, answered as it leans): each generic's body is checked once, with each `comptime` type as an abstract type parameter. A value of a type parameter `T` can be moved, copied, pointed to (`*T`, `*mut T`), wrapped (`?T`), stored in arrays and slices, and allocated (`alloc.create`, `alloc.alloc`). It has no fields, no operators (`==` too) and no methods, except the methods of the interfaces of its `where` constraints, called as on a struct (`x.area()`, with the interface's receiver: `self: *mut Self` needs a writable place). So an error in a generic is reported in the generic even if it is never instantiated.
  - An instantiation checks every constraint: the type argument must be a struct that implements the interface (structurally, as for `dyn`, through a `*mut T`). The error is at the instantiation (`Pool(Point)`, `twice(i32, 3)`), names the constraint and its position, and says why the type does not satisfy it. A type parameter of another generic satisfies a constraint only if it has the same one (`add where U: Show`).
  - Instances are memoized by their type arguments: `Pool(Node)` written twice is one type, and `Handle(Node)` and `Handle(Edge)` are different types even when `T` is not used in the struct. In messages an instance is named `Pool(Node)`.
  - Code is monomorphized: each instance is checked again with its concrete types (its errors are those of the generic, which were reported there), and gets its own C code. A generic is never emitted for its abstract parameters. Every method of a type function's struct is generated for each instance, used or not.
  - A generic function, and a method of a type function's struct, spells out its error set (`E!T`): an inferred set would depend on the body of each instance.
  - `comptime` parameters follow the shadowing rule like other names.
  - At most 64 instances of one generic: a generic that instantiates itself with a larger type (`Nest(?T)` in `Nest(T)`) would never end.
  - Out of scope (M4): `comptime` values other than types, `comptime` blocks and compile-time execution beyond type functions, and compile-time I/O.
- **Fix-its:** a fix-it is offered only where the edit is certain to fix the error: `let` to `var` for an assignment or `&mut` of a `let` binding; joining `}` and `else` when only a line break is between them and the statement before is an `if` without `else`; the missing trailing `,` of a multi-line call or literal when the closer follows on the next line; renaming an unknown type, variable, function or field to the only visible name one edit away (insert, delete, replace, or swap two adjacent characters), for names of at least 3 characters. Missing `match` arms get no fix-it: the patterns would be certain but no arm body is (there is no placeholder such as `todo`), and an empty arm would hide the missing case. `create(T)` with a type keeps its hint in the message: the initial value is not known.
- **Error recovery:** every stage reports all of its errors, sorted by position and deduplicated, at most 50.
  - The lexer skips the rest of a bad line.
  - The parser skips a bad statement up to the end of its line (using brace depth and indentation), and a bad declaration up to the next line that starts with a declaration keyword.
  - If the lexer or parser reported errors, sema does not run, as in Go. A syntax error never causes type errors.
  - Sema drops a statement with an error and goes on. A variable whose type is unknown (bad initializer, unknown type, rejected declaration) gets an error type, and its uses are not reported again.

## Modules (M5)

- **A module is a directory** (Go's rule): every `.ore` file directly in it belongs to it, and they share one namespace, so a file uses the others' declarations without imports. A name declared twice in a module is E0015, also across files.
- **The root module** is what the command gets: `ore build/run/check/test/fix dir` compiles the directory module `dir`; `ore ... file.ore` compiles that file as a module of its own, and the other files in its directory are not part of it (so a directory of single-file programs, such as `tests/cases`, keeps working). `ore build dir` names the executable after the directory.
- **`import a.b`** finds the module in the directory `a/b` under the root module's directory, from any module (paths are rooted, as in a Go module, not relative to the importing file). `std` and `std.*` are embedded in the compiler (the sources in `std/`, through `include_str!`), so `ore` works from any directory; a directory named `std` is never imported.
  - `import a.b` binds `b` in the importing file, and `import a.b.Name` binds the single item `Name` (which must be `pub`). There is no glob and no renaming. Imports are per file, as in Go.
  - After `import a`, a path reaches `a`'s items and also its submodules: `std.fs.open` after `import std`, as in Zig. A path is resolved one name at a time; when a module has both an item and a subdirectory of one name, the item wins. So every name from another module traces to one import line in the file.
  - A name imported twice, or imported and also declared in the module, is E0015. A local, parameter, capture or pattern binding cannot shadow an import (E0016), nor, in a module other than the root, a top-level name of its module (module resolution checks that; sema checks the root module as before).
  - An unknown module or member is E0081, an item that is not `pub` E0082, an import cycle E0083 (reported at the import or use that closes it: `` import cycle: `a` -> `b` -> `a` ``), a module used as a value or type E0085. A module that has syntax errors is not resolved, and lookups into it are not reported.
- **Visibility:** only `pub` items (functions, structs, enums, error sets, interfaces) and `pub fn` methods can be used from another module. Struct fields and enum variants are visible wherever their type is, as in Zig: `pub` on fields would add a rule to every struct literal. A method that is used only through an interface of another module, by `dyn` or a `where` constraint, needs no `pub`, because conformance is structural; an instance of a generic is not checked again for `pub`.
- **Canonical names:** module resolution rewrites every name of a module other than the root to `a::b::Name` (`std::fs::open`, `util::Thing`; methods stay `Thing.method`), so all modules become one AST for sema, and names of the root module stay as written. Messages show `::` as `.`: `std.fs.File`. Only the root module's `test` blocks are compiled; they are numbered across its files.
- **Order of checking:** the lexer and parser run on each file, module resolution on the program; if any of them reports errors, sema does not run. Diagnostics are sorted by file (root files first, in name order, then the modules in load order) and position.
- **Positions:** a `Span` has a file index. Diagnostics, `#line` directives, panics, leak reports and error return traces name the file of each position (`std/fs/fs.ore:70:9` for std). The runtime gets a position as two `int`s, with the file index in the line's upper bits.

## std (M5)

std is small, written in Ore over `extern fn` into libc, and embedded in the compiler. `ore spec` lists its whole API; `tests/spec.rs` checks that the spec names every pub function.

- **`std.fs`:** `open(path) OpenError!File` (read-only), `stdin()`, `stdout()`, `stderr()`; `File` has `readAll(alloc) ReadError![]mut u8`, `write(bytes) WriteError!void` and `close()`, all taking `self: Self`, so that Example 1's `let file` can call them. `error OpenError { NotFound, AccessDenied, OpenFailed }`, `error ReadError { ReadFailed, OutOfMemory }`, `error WriteError { WriteFailed }`. `File.write` flushes C stdio first, so it stays in order with `@print`. `readAll` gives the empty slice for no input (nothing to free), and otherwise a slice that starts its allocation, for `alloc.free`.
- **`std.mem`:** `Allocator`, `AllocError` (built in), `eql(a, b)` and `copy(dst, src)` on bytes.
- **`std.heap`:** `General` and `Arena` (built in).
- **`std.log`:** `err(format, args...)` and `info(format, args...)` write `error: <message>` and `info: <message>` and a newline to stderr, after flushing stdout. Ore has no variadic functions, so both are compiler builtins, written as calls of std items: the format must be a string literal; each `{}` in it takes the next argument, which must be an integer, float, `bool`, `[]u8`/`[]mut u8`, or an error value (written as its name); the count must match (E0086). There is no other formatting and no escape for `{}`. When `main` returns the error afterwards, as in Example 1, the error shows twice: once in the log line, once in the `error: Name` report.
- **`std.os`:** `args(alloc) AllocError![][]u8` (the slice is allocated, the arguments are not copied) and `exit(code)`, which is libc's `exit` declared `pub extern fn` (stdio is flushed; defers do not run).
- **Runtime hooks:** std calls three C functions of the runtime through `extern fn`: `ore_std_argc`, `ore_std_argv` (the arguments that the C `main` stored) and `ore_std_open` (a path slice that is not NUL-terminated, to `open`, returning the descriptor or `-errno`, because `errno` is a macro in C).
- `ore run <path> -- args` passes `args` to the program.

## Toolchain (M5)

- **`ore spec`** prints `docs/spec.md`, embedded in the binary: one Markdown language reference, meant to be all that an AI agent needs to write Ore. It must stay within about 20k tokens (counted as characters / 4; it is about 9k). In it, a ` ```ore ` block is a complete program that `ore check` accepts, and a ` ```ore-error ` block one that it rejects; `tests/spec.rs` checks both, and that every pub function and method of std is named as `name(`. Other fenced blocks are not checked.
- **`ore check --json`:** each diagnostic has a `"file"`, the file of its position and of its fix-it's edits. `ore fix` applies the fixes of each file to that file and never writes std.
- `ore test --json` failures have no file yet (a test's failure in another file of the module shows only its line and column there; the text output names the file).
- Code examples of `ore explain` can be several files: lines `// file: path` start each one, and the code table's test writes them to a directory and checks `main.ore`.

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
- **No libc headers:** the generated C includes only `stdint.h`, `stddef.h` and `stdbool.h`, so user `extern fn` declarations cannot conflict with libc headers. The runtime calls libc through asm-labelled aliases such as `ore_rt_printf`; `errno` is read through `__error` (macOS) or `__errno_location` (Linux).
- **Value types:** arrays are emitted as C structs (`struct { T a[N]; }`), so they are copied by value like structs.
- **Optional layout:** `?*T` is a C pointer that may be null, the same size as `*T`. Every other `?T` is `struct { bool has; T val; }`.
- **Enum layout:** a `uint32_t` tag (the variant's index in the declaration) and a union with one struct per payload variant. `match` is lowered to an `if` chain on the tag; the last arm needs no test.
- **Error union layout:** `struct { uint16_t err; T val; }`, where `err` is 0 for no error; `!void` has only `err`. Unions with the same payload share one C type. An error set value is a `uint16_t`.
- **`errdefer` timing:** an `errdefer` runs, in LIFO order with the `defer`s, when its scope is left by a `return` of an error, by a `return f()` whose union holds an error, or by a `try` that passes an error on. It does not run at the normal end of a block, on `break` or `continue`, or on a `return` without an error.
- **Error return trace (debug mode only):** every `return` of an error and every `try` that passes one on adds its position. When `catch` handles an error, that is, its body ends normally or leaves with `break`, `continue` or a `return` without an error, the entries added since the `catch` started are dropped. Up to 64 entries are kept.
- **An error returned from `main`** flushes stdout, writes `error: <Name>` to stderr, and in debug mode then `error return trace:` and one `file:line:col` line per entry, where the error started first. The exit status is 1. Safe and fast mode record no trace, so they print only the first line.
- **Recursive types:** a struct or enum that contains itself by value (also through an array or `?T`) is an error. Through a pointer it is fine.
- **`#line` directives** come before every generated statement so that debug info points to the `.ore` source.
- **C names:** a function is `ore_f_name`, a method `ore_m_<len><Struct>_<method>` (the length keeps the names apart), and test `N` is `ore_t_N`. The name of an instance (`Pool(Node)`, `max(i64)`) is written in C as `0` (which starts no Ore name) followed by the name with `_` doubled and other characters as `_` and two hex digits: `ore_s_0Pool_28Node_29`.
- **`alloc.alloc`** asks the allocator for `n * sizeof(T)` bytes (`OutOfMemory` when that overflows), then stores `value` into every element. Leak and double-free tracking treat the allocation like one from `create`, at the position of the `alloc` call.
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
- The doc has no `@` builtins. `@print` and `@println` remain the way to format numbers to stdout: `std` has no formatting into memory yet. `std.log` writes to stderr.
- Example 1 (`readConfig` and `main`) compiles and runs verbatim (`tests/example1/app/main.ore`); `Config` is supplied in `config.ore` of the same directory module. `std.heap.General`, `std.mem.Allocator` and `std.log.err` are compiler builtins exposed as std items (see std above), not Ore declarations.
- The doc's `ore doc --signatures <module>` does not exist yet; `ore spec` includes the signatures of all of std.
- The doc's `Allocator` interface has `alloc`, `resize` and `free`. Stage0's vtable has `alloc` and `free`, and Ore code uses the typed `create`, `destroy`, `alloc` and `free`.
- Example 3 declares `slots: []Slot(T)` and then takes `&mut self.slots[h.index]`. With slice elements read-only by default this does not compile, so `tests/cases/pool.ore` declares `slots: []mut Slot(T)`; the rest of `get` is the doc's text. The doc's `Pool(T)` is in `std`; stage0's std has no `Pool` yet, so the program declares `Pool`, `Slot` and `Handle` itself. Its `Pool` keeps the allocator that `init` takes, allocates on the first `insert` (there is no value to fill the slots with before), and moves to a larger allocation when full.
- The doc does not say where `where` goes; stage0 puts it after the return type, as Rust does.
- The doc says `General` detects use-after-free in debug mode. Stage0 detects leaks and double frees only.
- The doc says tests use a debug allocator by default. Stage0 spells it `@testAllocator()`, a builtin, until `std.testing` exists. Examples in doc comments do not run as tests yet.
