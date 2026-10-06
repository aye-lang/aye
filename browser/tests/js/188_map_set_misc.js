const cache = new Map();
function memoFib(n) {
  if (n < 2) return n;
  if (cache.has(n)) return cache.get(n);
  const v = memoFib(n - 1) + memoFib(n - 2);
  cache.set(n, v);
  return v;
}
console.log(memoFib(80), cache.size);
const seen = new Set();
const dupes = [5, 3, 5, 1, 3, 9].filter((x) => (seen.has(x) ? true : (seen.add(x), false)));
console.log(dupes, seen.size);
const grouped = new Map();
for (const w of ['apple', 'avocado', 'banana', 'blueberry', 'cherry']) {
  const k = w[0];
  if (!grouped.has(k)) grouped.set(k, []);
  grouped.get(k).push(w);
}
console.log(Object.fromEntries(grouped));
console.log(typeof Map, new Map() instanceof Map, Object.prototype.toString.call(new Set()));
