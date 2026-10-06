const keep = [];
let acc = 0;
for (let i = 0; i < 200000; i++) {
  const f = (x) => x + i;
  acc += f(1);
  if (i % 50000 === 0) keep.push(f);
}
console.log(acc, keep.length, keep.map((f) => f(0)));
