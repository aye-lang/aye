// Sloppy mode code: a function called without `this` gets the global
// object (Google's `(function(_){ ... this.gbar_ ... })()` pattern);
// strict mode code ("use strict", class bodies) gets undefined.
function sloppy() { return this === globalThis; }
console.log(sloppy(), sloppy.call(null), sloppy.call(undefined), sloppy.call(1) === false);
globalThis.g = 'global property';
console.log((function () { return this.g; })());
const o = { m() { return this; } };
const m = o.m;
console.log(o.m() === o, m() === globalThis);
function strict() { 'use strict'; return this; }
console.log(strict(), strict.call(null), typeof strict.call(1));
function outer() {
  'use strict';
  return (function inner() { return this; })();
}
console.log(outer());
class C {
  static s() { return this; }
  m() { return (function () { return this; })(); }
}
const s = C.s;
console.log(s(), new C().m());
const arrow = () => this;
console.log(typeof arrow());
console.log([1].map(function () { return this === globalThis; })[0], [1].map(function () { return this; }, 'x')[0] == 'x');
function late() {
  var x = 1;
  'use strict';
  return this === globalThis;
}
console.log(late());
