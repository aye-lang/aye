const data = { name: 'x', list: [1, [2, 3], { k: 'v' }], empty: {}, emptyArr: [], n: null };
console.log(JSON.stringify(data, null, 2));
console.log(JSON.stringify(data, null, '\t'));
console.log(JSON.stringify([1, 2], null, 4));
console.log(JSON.stringify({ a: 1 }, null, '--'));
console.log(JSON.stringify({ a: [1] }, null, 20) === JSON.stringify({ a: [1] }, null, 10));
console.log(JSON.stringify({ a: 1 }, null, 0), JSON.stringify({ a: 1 }, null, ''));
