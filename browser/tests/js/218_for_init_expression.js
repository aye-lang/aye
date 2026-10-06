// The init expression of a `for` loop names the variables of the
// enclosing function (minified code: jQuery's extend, React's bundles).
var o = { name: 'o' };
o.self = function () {
  var a = 1;
  for (a = this; false;);
  return a === this;
};
console.log(o.self());

function count(n) {
  var i, s = 0;
  for (i = 0, s = 10; i < n; i++) s += i;
  return [i, s];
}
console.log(count(4));
console.log(typeof i, typeof s);

function closures() {
  let k = 0;
  const fs = [];
  for (k = 0; k < 3; k++) fs.push(() => k);
  return fs.map(f => f());
}
console.log(closures());

const extend = function () {
  var e, t, a = arguments[0] || {}, s = 1, u = arguments.length;
  for (s === u && (a = this, s--); s < u; s++)
    if (null != (e = arguments[s])) for (t in e) a[t] = e[t];
  return a;
};
const lib = { extend };
lib.extend({ x: 1, y: 2 });
console.log(lib.x, lib.y, extend({}, { z: 3 }).z);

function nested() {
  var out = [];
  for (var j = 0; j < 2; j++) {
    var m;
    for (m = j * 10; m < j * 10 + 2; m++) out.push(m);
  }
  return out;
}
console.log(nested());
