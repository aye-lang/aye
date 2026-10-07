// Generic descriptors (no value/writable/get/set) keep the property's kind.
var o = {};
Object.defineProperty(o, 'a', {configurable: true, get: function () { return 1; }, set: function (v) {}});
Object.defineProperty(o, 'a', {enumerable: false});
var d = Object.getOwnPropertyDescriptor(o, 'a');
console.log(typeof d.get, typeof d.set, 'value' in d, d.enumerable, d.configurable, o.a);
Object.defineProperty(o, 'a', {enumerable: true});
d = Object.getOwnPropertyDescriptor(o, 'a');
console.log(typeof d.get, 'value' in d, d.enumerable, o.a);
var p = {b: 2};
Object.defineProperty(p, 'b', {enumerable: false});
d = Object.getOwnPropertyDescriptor(p, 'b');
console.log(d.value, d.writable, d.enumerable, d.configurable, 'get' in d);
Object.defineProperty(p, 'b', {configurable: false});
d = Object.getOwnPropertyDescriptor(p, 'b');
console.log(d.value, d.writable, d.enumerable, d.configurable);
// a generic descriptor on a new property makes a data property with undefined
Object.defineProperty(p, 'c', {enumerable: true});
d = Object.getOwnPropertyDescriptor(p, 'c');
console.log(d.value, d.writable, d.enumerable, d.configurable, 'get' in d);
// only get changes; set kept
Object.defineProperty(o, 'a', {get: function () { return 3; }});
d = Object.getOwnPropertyDescriptor(o, 'a');
console.log(typeof d.set, o.a);
// Object.defineProperties and Reflect.defineProperty too
Object.defineProperties(o, {a: {enumerable: true}});
console.log(typeof Object.getOwnPropertyDescriptor(o, 'a').get, o.a);
console.log(Reflect.defineProperty(o, 'a', {enumerable: false}), typeof Object.getOwnPropertyDescriptor(o, 'a').get, o.a);
// on a class instance, through the prototype's accessor (React's input tracker)
class C { get v() { return this._v; } set v(x) { this._v = x; } }
var c = new C();
var r = Object.getOwnPropertyDescriptor(C.prototype, 'v');
var g = r.get, s = r.set;
Object.defineProperty(c, 'v', {configurable: true, get: function () { return g.call(this); }, set: function (x) { s.call(this, x); }});
Object.defineProperty(c, 'v', {enumerable: r.enumerable});
c.v = 'x';
console.log(typeof c.v, c.v);
// an accessor becoming a data property: value undefined, writable false
var q = {};
Object.defineProperty(q, 'x', {configurable: true, enumerable: true, get: function () { return 5; }});
Object.defineProperty(q, 'x', {writable: true});
d = Object.getOwnPropertyDescriptor(q, 'x');
console.log(d.value, d.writable, d.enumerable, d.configurable, 'get' in d);
Object.defineProperty(q, 'y', {configurable: true, get: function () { return 6; }});
Object.defineProperty(q, 'y', {value: 7});
d = Object.getOwnPropertyDescriptor(q, 'y');
console.log(d.value, d.writable, d.enumerable, d.configurable, 'set' in d);
