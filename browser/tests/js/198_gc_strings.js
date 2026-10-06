let total = 0;
let last = '';
for (let round = 0; round < 200; round++) {
  let s = '';
  for (let i = 0; i < 1000; i++) s += String.fromCharCode(97 + (i % 26));
  total += s.length;
  last = s;
}
console.log(total, last.slice(0, 30), last.length);
const parts = [];
for (let i = 0; i < 100000; i++) parts.push('item' + i);
console.log(parts.join(',').length, parts[99999]);
