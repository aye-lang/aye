function f() {
  console.log(arguments.length, arguments[0], arguments[2]);
  let s = 0;
  for (let i = 0; i < arguments.length; i++) s += arguments[i];
  return s;
}
console.log(f(1, 2, 3));
console.log(f());
function g(a, b) { return arguments.length + ':' + [a, b].join(','); }
console.log(g(1), g(1, 2, 3));
function toArr() { return Array.prototype.slice.call(arguments); }
console.log(toArr(4, 5, 6));
function fromArgs() { return Array.from(arguments).map((x) => x * 2); }
console.log(fromArgs(1, 2));
function outer() { const inner = () => arguments[0]; return inner('ignored'); }
console.log(outer('outer arg'));
function spreadArgs() { return [...arguments].reverse(); }
console.log(spreadArgs('a', 'b', 'c'));
