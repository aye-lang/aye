console.log(add(2, 3));
function add(a, b) {
  return a + b;
}
console.log(typeof later, typeof expr);
var expr = function () { return 'expr'; };
console.log(expr());
function later() { return 'later'; }
function outer() {
  return inner();
  function inner() { return 'inner hoisted'; }
}
console.log(outer());
function dup() { return 1; }
function dup() { return 2; }
console.log(dup());
