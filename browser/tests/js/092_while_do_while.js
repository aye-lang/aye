let i = 0;
while (i < 3) { console.log('while', i); i++; }
let j = 10;
do { console.log('do', j); j++; } while (j < 3);
let n = 27, steps = 0;
while (n !== 1) { n = n % 2 ? 3 * n + 1 : n / 2; steps++; }
console.log('collatz steps', steps);
let k = 0;
while (true) { if (++k > 4) break; }
console.log(k);
let s = '';
do s += 'x'; while (s.length < 5);
console.log(s);
let m = 0;
for (;;) { m += 2; if (m >= 10) break; }
console.log(m);
