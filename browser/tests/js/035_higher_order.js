const compose = (...fns) => (x) => fns.reduceRight((acc, f) => f(acc), x);
const pipe = (...fns) => (x) => fns.reduce((acc, f) => f(acc), x);
const inc = (x) => x + 1, dbl = (x) => x * 2;
console.log(compose(inc, dbl)(5), pipe(inc, dbl)(5));
const curry = (f) => (a) => (b) => f(a, b);
console.log(curry((a, b) => a - b)(10)(3));
function partial(fn, ...pre) { return (...rest) => fn(...pre, ...rest); }
console.log(partial(Math.max, 5)(1, 2), partial((a, b, c) => a + b + c, 'x', 'y')('z'));
const map = (arr, f) => { const out = []; for (const x of arr) out.push(f(x)); return out; };
console.log(map([1, 2, 3], dbl));
const memoize = (fn) => { const m = new Map(); return (n) => { if (!m.has(n)) m.set(n, fn(n)); return m.get(n); }; };
const slowSq = memoize((n) => { console.log('compute', n); return n * n; });
console.log(slowSq(4), slowSq(4));
