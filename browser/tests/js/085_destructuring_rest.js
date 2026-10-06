const { a, ...others } = { a: 1, b: 2, c: 3 };
console.log(a, others);
const [x, ...ys] = [1];
console.log(x, ys);
const { ...copy } = { p: 1, q: [2] };
console.log(copy);
const [, ...tail] = 'abc';
console.log(tail);
function omit(obj, key) { const { [key]: _, ...r } = obj; return r; }
console.log(omit({ k1: 1, k2: 2, k3: 3 }, 'k2'));
const [[...inner], ...outer] = [[1, 2], [3], [4]];
console.log(inner, outer);
