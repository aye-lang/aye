let s = 0;
for (let i = 1; i <= 100; i++) s += i;
console.log(s);
const rows = [];
for (let i = 1; i <= 3; i++) {
  let row = '';
  for (let j = 1; j <= 3; j++) row += (i * j).toString().padStart(3);
  rows.push(row);
}
console.log(rows.join('\n'));
for (let i = 10; i > 0; i -= 4) console.log('down', i);
const primes = [];
for (let n = 2; primes.length < 10; n++) {
  let ok = true;
  for (let d = 2; d * d <= n; d++) if (n % d === 0) { ok = false; break; }
  if (ok) primes.push(n);
}
console.log(primes);
