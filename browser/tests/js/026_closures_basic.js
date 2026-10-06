function makeAdder(n) {
  return function (x) { return x + n; };
}
const add5 = makeAdder(5);
const add10 = makeAdder(10);
console.log(add5(1), add10(1), add5(add10(0)));
function counter() {
  let c = 0;
  return { inc: () => ++c, dec: () => --c, get: () => c };
}
const c1 = counter(), c2 = counter();
c1.inc(); c1.inc(); c2.dec();
console.log(c1.get(), c2.get());
function once(fn) {
  let done = false, result;
  return (...args) => {
    if (!done) { done = true; result = fn(...args); }
    return result;
  };
}
const init = once((x) => { console.log('init', x); return x * 2; });
console.log(init(3), init(4));
let shared = 'a';
const read = () => shared;
shared = 'b';
console.log(read());
