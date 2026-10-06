for (const x of [10, 20, 30]) console.log(x);
const chars = [];
for (const ch of 'héllo😀!') chars.push(ch);
console.log(chars, chars.length);
const codes = [];
for (const ch of '日本語') codes.push(ch.codePointAt(0).toString(16));
console.log(codes);
for (const x of []) console.log('never');
const arr = [1, 2, 3];
for (const x of arr) { if (x === 1) arr.push(4); }
console.log(arr);
let out = '';
for (const c of 'abc') { if (c === 'b') continue; out += c; }
console.log(out);
for (const x of [1, , 3]) console.log('hole', x);
for (const ch of 'á') console.log(ch.length);
