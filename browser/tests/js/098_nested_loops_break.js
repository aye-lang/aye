const grid = [[1, 2, 3], [4, 5, 6], [7, 8, 9]];
let diag = 0;
for (let i = 0; i < 3; i++) for (let j = 0; j < 3; j++) if (i === j) diag += grid[i][j];
console.log(diag);
const transposed = grid[0].map((_, c) => grid.map((r) => r[c]));
console.log(transposed);
let pairs = 0;
for (let a = 0; a < 10; a++) {
  for (let b = a; b < 10; b++) {
    if (a + b > 12) break;
    if ((a + b) % 2) continue;
    pairs++;
  }
}
console.log(pairs);
let x = 0;
while (x < 100) { x++; if (x % 7 !== 0) continue; if (x > 30) break; console.log('mult7', x); }
