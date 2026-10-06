const s = Symbol('foo');
console.log(typeof s, s.toString(), String(s), s.description, Symbol().description);
const o = { [s]: 1, a: 2 };
console.log(Object.keys(o), o[s], JSON.stringify(o), s in o);
console.log(Symbol.for('k') === Symbol.for('k'), Symbol.keyFor(Symbol.for('k')), Symbol('x') === Symbol('x'));
console.log(typeof Symbol.iterator, Symbol.iterator.toString());
class Range { constructor(n) { this.n = n; } [Symbol.iterator]() { let i = 0, n = this.n; return { next: () => i < n ? { value: i++, done: false } : { value: undefined, done: true }, return() { console.log('closed'); return {}; } }; } }
console.log([...new Range(3)], Array.from(new Range(2)));
for (const x of new Range(5)) { if (x === 2) break; console.log('x', x); }
console.log(Math.max(...new Range(4)));
const it = [1, 2][Symbol.iterator](); console.log(it.next(), [...'ab'[Symbol.iterator]()]);
console.log(new Map([[1, 2]])[Symbol.iterator]().next().value);
try { `${s}`; } catch (e) { console.log(e.constructor.name, e.message); }
console.log(Object.getOwnPropertyNames(o));
