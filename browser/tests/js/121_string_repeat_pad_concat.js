console.log('ab'.repeat(3), 'x'.repeat(0) === '', '-'.repeat(10));
console.log('5'.padStart(3, '0'), 'abc'.padStart(6), 'abc'.padStart(2), 'abc'.padStart(8, '12'));
console.log('5'.padEnd(3, '0') + '|', 'abc'.padEnd(6) + '|', 'abc'.padEnd(7, 'xy'));
console.log('a'.concat('b', 'c', 1, null), ''.concat([1, 2], {}));
console.log('x'.padStart(5, ''), '7'.padStart(2, '0') + ':' + '5'.padStart(2, '0'));
const table = [['name', 'qty'], ['apple', '3'], ['kiwi', '12']];
for (const [n, q] of table) console.log(n.padEnd(8, '.') + q.padStart(4));
