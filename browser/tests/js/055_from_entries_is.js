console.log(Object.fromEntries([['a', 1], ['b', 2]]));
console.log(Object.fromEntries(new Map([['x', 'X'], [1, 'one']])));
const prices = { apple: 1, pear: 2, kiwi: 3 };
const doubled = Object.fromEntries(Object.entries(prices).map(([k, v]) => [k, v * 2]));
console.log(doubled);
console.log(Object.fromEntries([]));
console.log(Object.is(1, 1), Object.is(NaN, 0 / 0), Object.is(0, -0), Object.is(-0, -0));
console.log(Object.is({}, {}), Object.is('a', 'a'), Object.is(null, undefined));
