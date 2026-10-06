// Object.prototype.__proto__ (Annex B): reading gives the prototype,
// assigning an object or null changes it (ShadyDOM turns fragments into
// shadow roots this way); other values are ignored.
const base = { hello() { return 'hi'; } };
const o = {};
o.__proto__ = base;
console.log(o.hello(), o.__proto__ === base, Object.getPrototypeOf(o) === base);
o.__proto__ = 5;
console.log(o.__proto__ === base);
const bare = { x: 1 };
bare.__proto__ = null;
console.log(Object.getPrototypeOf(bare), '__proto__' in bare, bare.__proto__);
const lit = { __proto__: base, y: 2 };
console.log(lit.hello(), Object.keys(lit).join(), Object.prototype.hasOwnProperty.call(Object.prototype, '__proto__'));
class A { a() { return 'a'; } }
const arr = [1, 2];
arr.__proto__ = A.prototype;
console.log(arr.a(), arr instanceof A, Array.isArray(arr), (1).__proto__ === Number.prototype, 'x'.__proto__ === String.prototype);
console.log(Object.getOwnPropertyDescriptor(Object.prototype, '__proto__').enumerable, JSON.stringify(Object.keys({})));
