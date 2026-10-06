console.log(1 < 2, 2 < 1, 2 <= 2, 3 >= 4, 5 > 4);
console.log('a' < 'b', 'B' < 'a', 'abc' < 'abd', 'ab' < 'abc', '10' < '9');
console.log('10' < 9, 10 < '9', '10' > '9');
console.log(null < 1, null >= 0, null > 0, undefined < 1, undefined >= 0);
console.log(NaN < 1, NaN >= 1, NaN == NaN, NaN != NaN);
console.log([2] > 1, [1, 2] > 1, true > false, 'a' > 1);
console.log(Infinity > 1e308, -Infinity < -1e308);
const o = { valueOf() { return 10; } };
console.log(o > 5, o < 20, o >= 10);
