# BigInt

BigInt is a primitive (`Value.Big`), not a Number or a JS object. The
payload is immutable sign/magnitude, little-endian base-2^16 limbs, in a
flat GC byte cell. The sign is the first byte; zero has no limbs and no
negative sign. `gc.ore` traces the payload through the existing byte-cell
collector. Object wrappers use `Class.BigInt` / `ObjData.Prim`.

`bigint.ore` decodes operands into arena-owned scratch limbs before
allocating a result. Arithmetic uses carry/borrow addition, schoolbook
multiplication, binary long division, two's-complement bitwise operations,
and exponentiation by squaring. No arithmetic goes through f64. Literal
spelling is retained as an atom, including radix prefixes and separators,
rather than rounded during lexing. Temporary operands that must survive
user coercion are rooted on the interpreter's stack.

## Supported

- Decimal, hexadecimal, octal and binary literals, with validated numeric
  separators; `typeof`, truthiness and value equality (including Map/Set).
- `BigInt` conversions from integer Numbers, strings, booleans and objects;
  non-integer Numbers throw RangeError. Construction with `new` throws
  TypeError. Object coercion honors `Symbol.toPrimitive` and ordinary
  `valueOf`/`toString` ordering.
- Prototype `toString` (radices 2–36), `valueOf`, and default/en-US
  `toLocaleString` grouping. `asIntN` / `asUintN` use ToIndex and ToBigInt.
- Arithmetic, signed bitwise operations and shifts, unary negation and
  complement, compound assignments, and prefix/postfix updates.
  Number/BigInt arithmetic mixing, unary plus and unsigned right shift
  throw TypeError; zero division/remainder and negative powers throw
  RangeError.
- Exact relational and loose-equality comparisons with Numbers (including
  fractions, NaN and infinity) and strings. Strict equality distinguishes
  Number from BigInt.
- String/template/property-key conversions, explicit Number conversion
  with single ties-to-even rounding, wrappers, console inspection, and
  JSON errors (after `toJSON` and replacer hooks). Implicit ToNumber,
  including Math's native functions, rejects BigInt.

## Limits / follow-ups

- No inline small-value specialization yet: small and large values use the
  same GC-managed limb representation.
- Multiplication, powers, left shifts and fixed-width results have an
  8,388,608-bit resource ceiling and throw RangeError when exceeded. Large
  fixed widths that preserve an existing value do not allocate padding.
- Locale formatting follows the engine's existing English-only Number
  support. Other locales and Intl formatting options are not implemented.
- **BigInt64Array / BigUint64Array are not implemented by this step.** The
  pre-existing JS typed-array facsimiles in `lib.ore` still convert their
  BigInt-named variants to Numbers. They do not provide integer-index
  coercion or shared buffer views; changing their constructor converters
  alone would not constitute correct BigInt typed arrays.

## Verification

`246_bigint_core`, `247_bigint_errors`, and `248_bigint_edges` are Node 22
output goldens. They include cross-limb/cross-sign arithmetic, exact f64
comparison and rounding boundaries, coercion side effects, wrappers,
JSON hooks, errors and large-width reductions. Run all three with both
`target/anyknown js FILE` and `ANYKNOWN_JS_GC=stress`.
