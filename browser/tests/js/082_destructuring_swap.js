let a = 1, b = 2;
[a, b] = [b, a];
console.log(a, b);
let x = 1, y = 2, z = 3;
[x, y, z] = [z, x, y];
console.log(x, y, z);
const arr = [5, 6, 7];
[arr[0], arr[2]] = [arr[2], arr[0]];
console.log(arr);
const o = { p: 'P', q: 'Q' };
[o.p, o.q] = [o.q, o.p];
console.log(o);
let m, n;
[m = 10, n = m * 2] = [];
console.log(m, n);
function bubble(xs) {
  const r = [...xs];
  for (let i = 0; i < r.length; i++) for (let j = 0; j < r.length - 1 - i; j++) if (r[j] > r[j + 1]) [r[j], r[j + 1]] = [r[j + 1], r[j]];
  return r;
}
console.log(bubble([5, 3, 8, 1, 9, 2]));
