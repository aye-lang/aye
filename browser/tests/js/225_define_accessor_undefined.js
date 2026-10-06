// Object.defineProperty with `get: undefined` (or `set`) removes that
// function (TypeScript decorators that turn a getter into a computed
// property do it); what the descriptor leaves out stays, in place.
const o = {};
Object.defineProperty(o, 'g', { get() { return 'old'; }, configurable: true, enumerable: true });
const d = Object.getOwnPropertyDescriptor(o, 'g');
d.get = undefined;
Object.defineProperty(o, 'g', d);
console.log(o.g, 'get' in Object.getOwnPropertyDescriptor(o, 'g'), typeof Object.getOwnPropertyDescriptor(o, 'g').get);
const p = {};
Object.defineProperty(p, 'h', { get() { return 1; }, set(v) { this._v = v; }, configurable: true });
Object.defineProperty(p, 'h', { set: undefined });
p.h = 5;
console.log(p.h, p._v, typeof Object.getOwnPropertyDescriptor(p, 'h').set);
const q = { a: 1 };
Object.defineProperty(q, 'b', { get() { return 2; }, configurable: true, enumerable: true });
q.c = 3;
Object.defineProperty(q, 'b', { get: undefined });
console.log(Object.keys(q).join(), q.b);
