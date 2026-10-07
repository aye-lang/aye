// ValidateAndApplyPropertyDescriptor on non-configurable properties.
function tryDefine(o, k, d) {
  try { Object.defineProperty(o, k, d); return 'ok'; } catch (e) { return e.name + ': ' + e.message; }
}
function show(o, k) {
  var d = Object.getOwnPropertyDescriptor(o, k);
  if ('get' in d) return 'accessor ' + typeof d.get + ' ' + typeof d.set + ' ' + d.enumerable + ' ' + d.configurable;
  return 'data ' + d.value + ' ' + d.writable + ' ' + d.enumerable + ' ' + d.configurable;
}
// a non-configurable accessor
var g = function () { return 1; }, s = function (v) {};
var a = {};
Object.defineProperty(a, 'x', {get: g, set: s, enumerable: true});
console.log(tryDefine(a, 'x', {enumerable: true}), show(a, 'x'));
console.log(tryDefine(a, 'x', {configurable: false}), show(a, 'x'));
console.log(tryDefine(a, 'x', {}), show(a, 'x'));
console.log(tryDefine(a, 'x', {enumerable: false}), show(a, 'x'));
console.log(tryDefine(a, 'x', {configurable: true}), show(a, 'x'));
console.log(tryDefine(a, 'x', {get: g}), tryDefine(a, 'x', {set: s}), tryDefine(a, 'x', {get: g, set: s, enumerable: true}));
console.log(tryDefine(a, 'x', {get: function () { return 2; }}), a.x);
console.log(tryDefine(a, 'x', {set: undefined}), show(a, 'x'));
console.log(tryDefine(a, 'x', {value: 1}), tryDefine(a, 'x', {writable: false}), show(a, 'x'));
// an accessor without a setter keeps `set: undefined`
var b = {};
Object.defineProperty(b, 'y', {get: g});
console.log(tryDefine(b, 'y', {set: undefined}), tryDefine(b, 'y', {get: g, set: undefined}), show(b, 'y'));
// a non-configurable writable data property
var c = {};
Object.defineProperty(c, 'v', {value: 1, writable: true});
console.log(tryDefine(c, 'v', {value: 2}), show(c, 'v'));
console.log(tryDefine(c, 'v', {get: g}), show(c, 'v'));
console.log(tryDefine(c, 'v', {set: s}), show(c, 'v'));
console.log(tryDefine(c, 'v', {configurable: true}), show(c, 'v'));
console.log(tryDefine(c, 'v', {enumerable: true}), show(c, 'v'));
console.log(tryDefine(c, 'v', {enumerable: false, configurable: false}), show(c, 'v'));
console.log(tryDefine(c, 'v', {writable: false, value: 3}), show(c, 'v'));
// now non-writable
console.log(tryDefine(c, 'v', {writable: true}), show(c, 'v'));
console.log(tryDefine(c, 'v', {value: 4}), show(c, 'v'));
console.log(tryDefine(c, 'v', {value: 3, writable: false}), show(c, 'v'));
// SameValue: NaN equals NaN, +0 differs from -0
var n = {};
Object.defineProperty(n, 'nan', {value: NaN});
Object.defineProperty(n, 'zero', {value: 0});
console.log(tryDefine(n, 'nan', {value: NaN}), tryDefine(n, 'zero', {value: -0}), tryDefine(n, 'zero', {value: 0}));
var obj = {};
Object.defineProperty(n, 'o', {value: obj});
console.log(tryDefine(n, 'o', {value: obj}), tryDefine(n, 'o', {value: {}}));
// Object.defineProperties
console.log((function () {
  try { Object.defineProperties(a, {x: {enumerable: true}}); return 'ok'; } catch (e) { return e.name; }
})(), (function () {
  try { Object.defineProperties(c, {w: {value: 1}, v: {configurable: true}}); return 'ok'; } catch (e) { return e.name; }
})(), show(c, 'w'));
// Reflect.defineProperty returns false instead of throwing
console.log(Reflect.defineProperty(a, 'x', {enumerable: true}), Reflect.defineProperty(a, 'x', {enumerable: false}));
console.log(Reflect.defineProperty(c, 'v', {get: g}), Reflect.defineProperty(c, 'v', {writable: true}), Reflect.defineProperty(c, 'v', {value: 3}));
// frozen and sealed objects
var f = Object.freeze({p: 1, get q() { return 2; }});
console.log(tryDefine(f, 'p', {value: 1}), tryDefine(f, 'p', {value: 2}), tryDefine(f, 'p', {writable: true}), show(f, 'p'));
console.log(tryDefine(f, 'p', {get: g}), tryDefine(f, 'p', {enumerable: false}), tryDefine(f, 'p', {enumerable: true}));
console.log(tryDefine(f, 'q', {enumerable: true}), tryDefine(f, 'q', {configurable: true}), tryDefine(f, 'q', {value: 2}), show(f, 'q'));
console.log(tryDefine(f, 'r', {value: 1}));
var se = Object.seal({p: 1});
console.log(tryDefine(se, 'p', {value: 5}), tryDefine(se, 'p', {writable: false}), tryDefine(se, 'p', {writable: true}), show(se, 'p'));
console.log(tryDefine(se, 'p', {get: g}), tryDefine(se, 'p', {configurable: true}), show(se, 'p'));
