let sum = 0;
for (let i = 0; i < 1000000; i++) {
  const o = { a: i, b: [i, i + 1] };
  sum += o.b[1] - o.a;
}
console.log(sum);
