// More layouts than the old 16384-shape table: objects past it still
// read and write their own and inherited properties correctly through
// warm caches, with and without the shared index (> 24 properties).
function make(i) {
  const o = {};
  o['k' + i] = i;
  o.a = i * 2;
  o.b = 'b' + i;
  return o;
}
const proto = { m() { return this.a + 1; } };
let sum = 0, bad = 0;
for (let i = 0; i < 40000; i++) {
  const o = make(i);
  Object.setPrototypeOf(o, proto);
  o.a = o.a + 1;
  sum += o.a + o.m() + o['k' + i];
  if (o.b !== 'b' + i) bad++;
}
console.log(sum, bad);
function wide(i) {
  const o = { ['w' + i]: i };
  for (let j = 0; j < 30; j++) o['p' + j] = j + i;
  return o;
}
let s2 = 0;
for (let i = 0; i < 3000; i++) {
  const o = wide(i);
  s2 += o.p0 + o.p29 + o['w' + i];
  delete o.p5;
  s2 += o.p6 + (o.p5 === undefined ? 1 : 0);
}
console.log(s2);
