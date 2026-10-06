const o = {};
Object.defineProperty(o, 'fixed', { value: 1, writable: false, enumerable: true, configurable: false });
try { o.fixed = 2; } catch (e) {}
console.log(o.fixed);
Object.defineProperty(o, 'hidden', { value: 'h' });
console.log(Object.keys(o), o.hidden, JSON.stringify(o));
let backing = 5;
Object.defineProperty(o, 'computed', { get() { return backing * 2; }, set(v) { backing = v; }, enumerable: true });
o.computed = 10;
console.log(o.computed, backing);
console.log(Object.getOwnPropertyDescriptor(o, 'fixed'));
console.log(Object.getOwnPropertyDescriptor(o, 'hidden'));
try {
  Object.defineProperty(o, 'fixed', { value: 3 });
} catch (e) {
  console.log(e.name);
}
Object.defineProperties(o, { p1: { value: 1, enumerable: true }, p2: { value: 2, enumerable: true } });
console.log(o.p1 + o.p2, Object.keys(o));
