function f({ name, age = 0, tags: [firstTag = 'none'] = [] }) {
  return `${name}/${age}/${firstTag}`;
}
console.log(f({ name: 'a' }), f({ name: 'b', age: 3, tags: ['t1', 't2'] }));
function g([a, b] = [1, 2], { c } = { c: 3 }) { return a + b + c; }
console.log(g(), g([10, 20]), g(undefined, { c: 100 }));
const pts = [{ x: 1, y: 2 }, { x: 3, y: 4 }];
console.log(pts.map(({ x, y }) => x * y));
console.log(Object.entries({ a: 1, b: 2 }).map(([k, v]) => k + v));
function opts({ verbose = false, level = 1, ...others } = {}) { return [verbose, level, others]; }
console.log(opts(), opts({ verbose: true, extra: 'x' }));
const swapFn = ([a, b]) => [b, a];
console.log(swapFn([1, 2]));
