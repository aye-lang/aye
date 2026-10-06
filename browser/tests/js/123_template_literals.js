const a = 5, b = 10;
console.log(`sum: ${a + b}, product: ${a * b}`);
console.log(`nested ${`inner ${a > b ? 'big' : `small ${b - a}`}`}`);
console.log(`multi
line
string`);
const items = ['x', 'y'];
console.log(`items: ${items.map((i) => `<${i}>`).join('')}`);
console.log(`${null} ${undefined} ${[1, [2, 3]]} ${{}} ${true}`);
console.log(`escape \` backtick and \${not interpolated} and \\`);
const fn = (n) => `n=${n}`;
console.log(`${fn(1)}${fn(2)}`, `${1}${2}` === '12');
console.log(`a
  b`.length, `A\x42\n`.length);
