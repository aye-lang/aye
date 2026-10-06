const m = new Map();
for (let i = 0; i < 5000; i++) m.set('k' + i, { n: i });
let s = 0;
for (const [k, v] of m) s += v.n;
console.log(m.size, s, m.get('k4999').n);
let big = [];
for (let r = 0; r < 20; r++) {
  big = [];
  for (let i = 0; i < 20000; i++) big.push([i, String(i)]);
}
console.log(big.length, big[19999][1]);
