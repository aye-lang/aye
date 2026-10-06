const o = { a: 1, b: { c: 2, d: [3, 4] }, e: undefined };
const { a, b: { c, d: [d0, d1] } } = o;
console.log(a, c, d0, d1);
const { a: renamed, z = 'default', e = 'e-default' } = o;
console.log(renamed, z, e);
const { a: aa, ...rest } = o;
console.log(aa, rest);
const key = 'b';
const { [key]: dyn } = o;
console.log(dyn);
let x, y;
({ x, y = 5 } = { x: 1 });
console.log(x, y);
const { length } = 'hello';
console.log(length);
const { toFixed } = 1;
console.log(typeof toFixed);
try {
  const { q } = null;
} catch (e) {
  console.log(e.name);
}
