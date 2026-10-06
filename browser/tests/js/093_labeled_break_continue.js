outer: for (let i = 0; i < 3; i++) {
  for (let j = 0; j < 3; j++) {
    if (j === 1) continue outer;
    if (i === 2) break outer;
    console.log('pair', i, j);
  }
}
let found = null;
search: for (const row of [[1, 2], [3, 4], [5, 6]]) {
  for (const v of row) {
    if (v * v > 10) { found = v; break search; }
  }
}
console.log('found', found);
let count = 0;
loop1: while (count < 10) {
  count++;
  let inner = 0;
  do {
    inner++;
    if (inner === 2) continue loop1;
  } while (inner < 5);
}
console.log('count', count);
a: for (const x of [1, 2]) { b: for (const y of [1, 2]) { if (y === 2) continue a; console.log(x, y); } }
