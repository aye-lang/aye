let ok = true;
for (let i = 0; i < 1000; i++) { const r = Math.random(); if (r < 0 || r >= 1) ok = false; }
console.log('random in range', ok);
const dice = new Set();
for (let i = 0; i < 2000; i++) dice.add(Math.floor(Math.random() * 6) + 1);
console.log([...dice].sort());
console.log(typeof Math, Object.prototype.toString.call(Math));
console.log(Math.max(...[1, 2, 3].map((x) => x ** 2)), Math.min.apply(null, [4, 2, 8]));
const clamp = (x, lo, hi) => Math.min(hi, Math.max(lo, x));
console.log(clamp(5, 0, 3), clamp(-1, 0, 3), clamp(2, 0, 3));
console.log(Math.round(123.456 * 100) / 100, Math.floor(Math.log10(12345)) + 1);
