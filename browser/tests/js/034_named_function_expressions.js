const fact = function me(n) { return n <= 1 ? 1 : n * me(n - 1); };
console.log(fact(6), typeof me);
const f = function inner() { return typeof inner; };
console.log(f());
console.log(fact.name, f.name);
const anon = function () {};
const arrow = () => {};
console.log(anon.name, arrow.name);
const o = { method() {}, prop: function () {}, named: function nm() {} };
console.log(o.method.name, o.prop.name, o.named.name);
function decl() {}
console.log(decl.name, (function () {}).name === '');
