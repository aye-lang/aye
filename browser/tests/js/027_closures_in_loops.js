const fs1 = [];
for (var i = 0; i < 3; i++) fs1.push(() => i);
console.log(fs1.map((f) => f()));
const fs2 = [];
for (let j = 0; j < 3; j++) fs2.push(() => j);
console.log(fs2.map((f) => f()));
const fs3 = [];
for (var k = 0; k < 3; k++) fs3.push(((kk) => () => kk)(k));
console.log(fs3.map((f) => f()));
const fs4 = [];
for (const v of ['a', 'b', 'c']) fs4.push(() => v);
console.log(fs4.map((f) => f()));
const fs5 = [];
for (let m = 0; m < 3; m++) { fs5.push(() => m); m++; }
console.log(fs5.map((f) => f()));
const fs6 = [];
let w = 0;
while (w < 3) { const cw = w; fs6.push(() => cw * 10); w++; }
console.log(fs6.map((f) => f()));
