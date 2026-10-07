// OrdinarySetPrototypeOf: the same prototype succeeds, a non-extensible
// object and a cycle fail. Object.setPrototypeOf and the __proto__ setter
// throw on failure; Reflect.setPrototypeOf returns false.
function t(f) {
  try { return String(f()); } catch (e) { return e.constructor.name; }
}
var p = {}, q = {};

// the same prototype: true even when not extensible
var ne = Object.preventExtensions(Object.create(p));
console.log(t(() => Reflect.setPrototypeOf(ne, p)), t(() => Object.setPrototypeOf(ne, p) === ne), t(() => { ne.__proto__ = p; return Object.getPrototypeOf(ne) === p; }));
// a different prototype on a non-extensible object fails
console.log(t(() => Reflect.setPrototypeOf(ne, q)), t(() => Object.setPrototypeOf(ne, q)), t(() => { ne.__proto__ = q; }), Object.getPrototypeOf(ne) === p);
console.log(t(() => Reflect.setPrototypeOf(ne, null)), t(() => Object.setPrototypeOf(Object.freeze({}), null)), t(() => Object.setPrototypeOf(Object.seal([]), {})));
var nn = Object.preventExtensions(Object.create(null));
console.log(t(() => Reflect.setPrototypeOf(nn, null)), t(() => Object.setPrototypeOf(nn, null) === nn), t(() => Reflect.setPrototypeOf(nn, p)));

// cycles fail
var a = {}, b = Object.create(a), c = Object.create(b);
console.log(t(() => Reflect.setPrototypeOf(a, a)), t(() => Reflect.setPrototypeOf(a, b)), t(() => Reflect.setPrototypeOf(a, c)));
console.log(t(() => Object.setPrototypeOf(a, c)), t(() => { a.__proto__ = c; }), Object.getPrototypeOf(a) === Object.prototype);
console.log(t(() => Reflect.setPrototypeOf(c, a)), Object.getPrototypeOf(c) === a);
// a function's chain too
function F() {}
console.log(t(() => Reflect.setPrototypeOf(Function.prototype, F)), t(() => Object.setPrototypeOf(Function.prototype, F)));
// the walk stops at a proxy: no cycle is seen through it
var x = {}, px = new Proxy(Object.create(x), {});
console.log(t(() => Reflect.setPrototypeOf(x, px)), Object.getPrototypeOf(x) === px);

// Object.prototype has an immutable prototype
console.log(t(() => Reflect.setPrototypeOf(Object.prototype, null)), t(() => Object.setPrototypeOf(Object.prototype, null) === Object.prototype));
console.log(t(() => Reflect.setPrototypeOf(Object.prototype, {})), t(() => Object.setPrototypeOf(Object.prototype, {})), t(() => { Object.prototype.__proto__ = {}; }), Object.getPrototypeOf(Object.prototype));
console.log(t(() => Object.prototype.__proto__ = null), Object.isExtensible(Object.prototype));

// primitives: RequireObjectCoercible, then the primitive back
console.log(t(() => Object.setPrototypeOf(1, null)), t(() => Object.setPrototypeOf('s', {})), t(() => Object.setPrototypeOf(true, Object.prototype)), t(() => typeof Object.setPrototypeOf(Symbol(), null)));
console.log(t(() => Object.setPrototypeOf(undefined, null)), t(() => Object.setPrototypeOf(null, {})), t(() => Object.setPrototypeOf(undefined)));
console.log(t(() => Object.setPrototypeOf(1, 2)), t(() => Object.setPrototypeOf(1)), t(() => Object.setPrototypeOf({}, 1)), t(() => Object.setPrototypeOf({})));
console.log(t(() => Reflect.setPrototypeOf(1, null)), t(() => Reflect.setPrototypeOf({}, 1)), t(() => Reflect.setPrototypeOf({})), t(() => Reflect.setPrototypeOf(Symbol(), null)), t(() => Reflect.setPrototypeOf({}, Symbol())), t(() => Object.setPrototypeOf({}, Symbol())));

// __proto__ setter: a non-object value is ignored, a primitive receiver too
var o = Object.create(p);
o.__proto__ = 5;
console.log(Object.getPrototypeOf(o) === p);
var setter = Object.getOwnPropertyDescriptor(Object.prototype, '__proto__').set;
console.log(t(() => setter.call(1, {})), t(() => setter.call(undefined, {})), t(() => setter.call(null, {})), t(() => setter.call(ne, q)));

// success changes the chain
var d = {};
console.log(Reflect.setPrototypeOf(d, q), Object.getPrototypeOf(d) === q, Object.setPrototypeOf(d, null) === d, Object.getPrototypeOf(d));

// a proxy's trap result: false fails
var pf = new Proxy({}, { setPrototypeOf() { return false; } });
var pt = new Proxy({}, { setPrototypeOf() { return 1; } });
console.log(t(() => Reflect.setPrototypeOf(pf, q)), t(() => Object.setPrototypeOf(pf, q)), t(() => { pf.__proto__ = q; }));
console.log(t(() => Reflect.setPrototypeOf(pt, q)), t(() => Object.setPrototypeOf(pt, q) === pt));
// no trap: the target's [[SetPrototypeOf]]
var pn = new Proxy(Object.preventExtensions({}), {});
console.log(t(() => Reflect.setPrototypeOf(pn, q)), t(() => Object.setPrototypeOf(pn, q)));
