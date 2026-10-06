function deepClone(v) {
  if (Array.isArray(v)) return v.map(deepClone);
  if (v && typeof v === 'object') { const o = {}; for (const k of Object.keys(v)) o[k] = deepClone(v[k]); return o; }
  return v;
}
const src = { a: [1, { b: 2 }], c: { d: { e: 'deep' } } };
const cl = deepClone(src);
cl.a[1].b = 99;
console.log(src.a[1].b, cl.a[1].b, JSON.stringify(cl));
function deepEqual(x, y) {
  if (x === y) return true;
  if (typeof x !== 'object' || typeof y !== 'object' || !x || !y) return false;
  const kx = Object.keys(x), ky = Object.keys(y);
  return kx.length === ky.length && kx.every((k) => deepEqual(x[k], y[k]));
}
console.log(deepEqual(src, deepClone(src)), deepEqual(src, cl), deepEqual([1, [2]], [1, [2]]));
const path = (o, p) => p.split('.').reduce((a, k) => a?.[k], o);
console.log(path(src, 'c.d.e'), path(src, 'a.1.b'), path(src, 'x.y'));
