// Reflect's boolean methods return false only for a rejected operation;
// a bad target, a bad descriptor and a throwing trap or getter throw.
function t(f) {
  try { return String(f()); } catch (e) { return e.constructor.name; }
}
var frozen = Object.freeze({a: 1});
function g() { return 1; }

// Reflect.defineProperty: the target and ToPropertyDescriptor
console.log(t(() => Reflect.defineProperty(1, 'x', {value: 1})), t(() => Reflect.defineProperty(undefined, 'x', {})), t(() => Reflect.defineProperty('s', 'x', {})));
console.log(t(() => Reflect.defineProperty({}, 'x', 1)), t(() => Reflect.defineProperty({}, 'x', undefined)), t(() => Reflect.defineProperty({}, 'x')));
console.log(t(() => Reflect.defineProperty({}, 'x', {get enumerable() { throw new RangeError('boom'); }})));
console.log(t(() => Reflect.defineProperty({}, 'x', {get get() { throw new EvalError('boom'); }})));
console.log(t(() => Reflect.defineProperty({}, 'x', {get: 1})), t(() => Reflect.defineProperty({}, 'x', {set: {}})), t(() => Reflect.defineProperty({}, 'x', {get: null})));
console.log(t(() => Reflect.defineProperty({}, 'x', {get: g, value: 1})), t(() => Reflect.defineProperty({}, 'x', {set: undefined, writable: true})), t(() => Reflect.defineProperty({}, 'x', {get: undefined, value: undefined})));
console.log(t(() => Reflect.defineProperty({}, 'x', {get: undefined})), t(() => Reflect.defineProperty({}, 'x', {get: g, set: g})), t(() => Reflect.defineProperty({}, 'x', {value: 1, writable: true})));
console.log(t(() => Reflect.defineProperty({}, {toString() { throw new SyntaxError('key'); }}, {})));
// a rejected definition is false, a valid one true
console.log(t(() => Reflect.defineProperty(frozen, 'a', {value: 2})), t(() => Reflect.defineProperty(frozen, 'b', {value: 2})), t(() => Reflect.defineProperty(frozen, 'a', {value: 1})));
console.log(t(() => Reflect.defineProperty(frozen, 'a', {get: g})), t(() => Reflect.defineProperty(frozen, 'a', {configurable: true})), t(() => Reflect.defineProperty(Object.preventExtensions({}), 'x', {})));
// descriptor errors come before the rejection
console.log(t(() => Reflect.defineProperty(frozen, 'a', {get: 1})), t(() => Reflect.defineProperty(frozen, 'b', {value: 1, get: g})));
// ToPropertyDescriptor reads each field once, in order, before checking
var log = [];
var d = {};
['set', 'get', 'writable', 'value', 'configurable', 'enumerable'].forEach(function (k) {
  Object.defineProperty(d, k, {get: function () { log.push(k); return undefined; }, enumerable: true});
});
console.log(t(() => Reflect.defineProperty({}, 'x', d)), log.join());
log = [];
var o = {};
console.log(t(() => Reflect.defineProperty(o, 'x', {get enumerable() { log.push('e'); return true; }, get value() { log.push('v'); return 5; }})), log.join(), o.x, Object.keys(o).join());
// inherited descriptor fields count
console.log(t(() => Reflect.defineProperty(o, 'y', Object.create({value: 3, writable: true}))), o.y, Object.getOwnPropertyDescriptor(o, 'y').writable);
console.log(t(() => Reflect.defineProperty(o, 'z', Object.create({get: 7}))));
// proxies: a falsish trap is false, a throwing trap throws
var pf = new Proxy({}, {defineProperty: function () { return false; }});
var pt = new Proxy({}, {defineProperty: function () { throw new EvalError('trap'); }});
console.log(t(() => Reflect.defineProperty(pf, 'x', {value: 1})), t(() => Object.defineProperty(pf, 'x', {value: 1})), t(() => Reflect.defineProperty(pt, 'x', {value: 1})));
console.log(t(() => Reflect.defineProperty(new Proxy({}, {}), 'x', {value: 1})), t(() => Reflect.defineProperty(new Proxy(frozen, {}), 'a', {value: 2})));

// Object.defineProperty: the same ToPropertyDescriptor
console.log(t(() => Object.defineProperty({}, 'x', {get: 1})), t(() => Object.defineProperty({}, 'x', {set: 'f'})), t(() => Object.defineProperty({}, 'x', {get: g, writable: false})));
console.log(t(() => Object.defineProperty({}, 'x', {set: g, value: 1})), t(() => Object.defineProperties({}, {x: {get: 1}})), t(() => Object.create(null, {x: {value: 1, set: g}})));
console.log(t(() => Object.defineProperty(frozen, 'a', {value: 2})), t(() => Object.defineProperty(frozen, 'a', {get: 1})));

// Reflect.set
console.log(t(() => Reflect.set(1, 'x', 1)), t(() => Reflect.set(null, 'x', 1)));
console.log(t(() => Reflect.set({}, 'x', 1, pt)), t(() => Reflect.set({}, 'x', 1, pf)), t(() => Reflect.set({}, 'x', 1, Object.preventExtensions({}))));
var r = {};
console.log(t(() => Reflect.set({}, 'x', 1, r)), r.x, t(() => Reflect.set({}, 'a', 1, frozen)), t(() => Reflect.set({}, 'x', 1, 5)));

// Reflect.deleteProperty
console.log(t(() => Reflect.deleteProperty(1, 'x')), t(() => Reflect.deleteProperty(undefined, 'x')));
console.log(t(() => Reflect.deleteProperty({x: 1}, 'x')), t(() => Reflect.deleteProperty({}, 'x')), t(() => Reflect.deleteProperty(frozen, 'a')));
console.log(t(() => Reflect.deleteProperty(new Proxy({}, {deleteProperty: function () { throw new EvalError('trap'); }}), 'x')));

// Reflect.setPrototypeOf
console.log(t(() => Reflect.setPrototypeOf(1, null)), t(() => Reflect.setPrototypeOf({}, 1)), t(() => Reflect.setPrototypeOf({}, undefined)));
console.log(t(() => Reflect.setPrototypeOf({}, null)), t(() => Reflect.setPrototypeOf({}, Array.prototype)));

// Reflect.preventExtensions
console.log(t(() => Reflect.preventExtensions(1)), t(() => Reflect.preventExtensions(undefined)), t(() => Reflect.preventExtensions({})));
