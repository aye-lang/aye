function error(f) { try { f(); console.log('NO ERROR'); } catch (e) { console.log(e.name); } }
for (const s of ['1.0n', '1e2n', '01n', '0_1n', '0x_1n', '1__2n', '1_n', '0b2n', '1nfoo']) error(() => eval(s));
console.log(eval('0x1_2n'), eval('0o1_2n'), eval('0b1_0n'));
console.log(BigInt('\u00a0\u2028+123\ufeff'), BigInt('-0'), BigInt('000123'));
console.log({0xffn: 'hex', 1_000n: 'decimal'}['255']);
console.log(Object(123n), [1n, -2n], Object.is(123n, BigInt('123')));
console.log(BigInt({valueOf() { return 123n; }}), BigInt({toString() { return '456'; }}));
const exotic = {[Symbol.toPrimitive](hint) { console.log('hint', hint); return 3n; }};
console.log(BigInt(exotic), exotic * 2n, exotic + 2n, String(exotic));
error(() => BigInt({[Symbol.toPrimitive]() { return {}; }}));
console.log(1n == Symbol('x'));
console.log(Object(3n) + 4n, Object(9n) / 2n, -Object(3n), ~Object(-3n));
error(() => +Object(1n)); error(() => Object(1n) * 2);
let o = {x: 1n}; console.log(o.x++, ++o.x, o['x']--, --o['x']);
let set = 0n; let p = {get x() { return set; }, set x(v) { set = v; }}; console.log(p.x++, ++p.x, set);
for (const v of [0n, 1n, -1n, 128n, -129n, 65536n, -65536n, (1n<<200n)-1n]) {
  for (const bits of [0, 1, 7, 8, 16, 17, 64, 100, 300]) console.log(bits, BigInt.asIntN(bits, v), BigInt.asUintN(bits, v));
}
console.log(BigInt.asIntN(9007199254740991, -1n), BigInt.asUintN(9007199254740991, 1n));
console.log(BigInt.asIntN(NaN, '123'), BigInt.asIntN(8.9, true));
console.log(1n == null, 0n == false, 0n == '', 0n == ' ', 2n != '2.5');
for (const n of [0, 1.5, -1.5, NaN, Infinity, -Infinity, 9007199254740992]) console.log(1n < n, 1n <= n, 1n > n, 1n >= n, n < 1n, n <= 1n, n > 1n, n >= 1n);
for (let i = 0n; i < 10n; i++) {
  const n = (1n<<100n) + (i<<47n) + (1n<<46n);
  console.log(Number(n), Number(n+1n), Number(n-1n), Number(-n));
}
console.log(Number((1n<<1024n)-(1n<<971n)-1n), Number((1n<<1024n)-(1n<<971n)), Number(1n<<2000n));
// Deterministic cross-sign, cross-limb arithmetic, all checked against Node.
let seed = 12345n;
for (let i = 0; i < 32; i++) {
  seed = (seed * 6364136223846793005n + 1442695040888963407n) & ((1n<<128n)-1n);
  const a = i%2 ? -seed : seed;
  const b = (seed>>61n) * (i%3 ? 1n : -1n) + 1n;
  console.log(a+b, a-b, a*b, a/b, a%b, a&b, a|b, a^b, a>>BigInt(i), a<<BigInt(i));
}
// Coercion may allocate and invoke script while earlier operands are rooted.
const allocate = () => { let xs = []; for (let i = 0; i < 50; i++) xs.push({i}); return 2n; };
console.log(BigInt('12345678901234567890') * {valueOf: allocate});
console.log(BigInt('12345678901234567890') < {valueOf: allocate});
error(() => JSON.stringify([1n, Object(2n)], (k,v) => typeof v === 'bigint' ? String(v) : v));
console.log(JSON.stringify([1n, Object(2n)], (k,v) => typeof v === 'bigint' || v instanceof BigInt ? String(v) : v));
