const s = new Set([3, 1, 3, 2, 1]);
console.log(s.size, [...s], s.has(2), s.has(4));
s.add(4).add(4);
console.log(s.delete(1), s.delete(1), [...s]);
console.log([...new Set('mississippi')].join(''));
const a = new Set([1, 2, 3]), b = new Set([2, 3, 4]);
console.log([...a].filter((x) => b.has(x)), [...new Set([...a, ...b])], [...a].filter((x) => !b.has(x)));
s.forEach((v, v2) => console.log(v === v2, v));
console.log(new Set([NaN, NaN, 0, -0]).size);
