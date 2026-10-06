const base = { greet() { return 'hello from base'; }, kind: 'base' };
const obj = { __proto__: base, own: 1 };
console.log(obj.greet(), obj.kind, Object.keys(obj), Object.getPrototypeOf(obj) === base);
const nul = { __proto__: null, a: 1 };
console.log(Object.getPrototypeOf(nul), 'toString' in nul);
const child = { __proto__: obj, deep: true };
console.log(child.greet(), child.own, base.isPrototypeOf(child));
const computed = { ['__proto__']: 5 };
console.log(Object.keys(computed), Object.getPrototypeOf(computed) === Object.prototype);
