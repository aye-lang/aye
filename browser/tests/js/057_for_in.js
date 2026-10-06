const proto = { p1: 'proto1', p2: 'proto2' };
const o = Object.create(proto);
o.a = 1;
o.b = 2;
const all = [], own = [];
for (const k in o) {
  all.push(k);
  if (Object.prototype.hasOwnProperty.call(o, k)) own.push(k);
}
console.log(all, own);
Object.defineProperty(o, 'hid', { value: 1, enumerable: false });
const keys = [];
for (const k in o) keys.push(k);
console.log(keys);
for (const i in ['x', 'y']) console.log(i, typeof i);
for (const c in 'ab') console.log('c', c);
for (const k in null) console.log('never');
const shadow = Object.create({ s: 'proto' });
shadow.s = 'own';
const sk = [];
for (const k in shadow) sk.push(k);
console.log(sk);
