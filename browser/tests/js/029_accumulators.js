function makeAcc(start) {
  let total = start;
  return (n) => (total += n);
}
const acc = makeAcc(100);
acc(10);
console.log(acc(-5));
function makeCounterFactory() {
  let made = 0;
  return function () {
    made++;
    let count = 0;
    return { id: made, next: () => ++count };
  };
}
const factory = makeCounterFactory();
const a = factory(), b = factory();
a.next(); a.next(); b.next();
console.log(a.id, a.next(), b.id, b.next());
const memo = (() => {
  const cache = {};
  let hits = 0;
  return { sq(n) { if (n in cache) { hits++; return cache[n]; } return (cache[n] = n * n); }, hits: () => hits };
})();
memo.sq(4); memo.sq(4); memo.sq(5); memo.sq(4);
console.log(memo.hits(), memo.sq(5));
