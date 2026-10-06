function f(a, b, c) { return [a, b, c]; }
console.log(f(...[1, 2, 3]), f(...[1], ...[2, 3]), f(0, ...[1]), f(...'xy'));
const nums = [5, 1, 9, -3];
console.log(Math.max(...nums), Math.min(...nums), Math.max(...[]), Math.min(...[]));
console.log(Math.max(...nums, 100), String.fromCharCode(...[72, 105]));
const big = Array.from({ length: 10000 }, (_, i) => i);
console.log(Math.max(...big));
console.log(new Date(...[Date.UTC(2020, 0, 1)]).toISOString());
const args = [1, 2];
console.log(f(...args, ...args));
console.log([].concat(...[[1], [2, [3]]]));
