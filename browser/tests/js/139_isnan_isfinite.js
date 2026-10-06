console.log(isNaN(NaN), isNaN('abc'), isNaN('12'), isNaN(undefined), isNaN(null), isNaN({}));
console.log(Number.isNaN(NaN), Number.isNaN('abc'), Number.isNaN(undefined), Number.isNaN(0 / 0));
console.log(isFinite(1), isFinite('12'), isFinite(Infinity), isFinite(NaN), isFinite(null));
console.log(Number.isFinite(1), Number.isFinite('12'), Number.isFinite(-Infinity), Number.isFinite(1e308));
const arr = [1, NaN, 3];
console.log(arr.indexOf(NaN), arr.includes(NaN), arr.findIndex(Number.isNaN));
console.log(NaN === NaN, NaN !== NaN, [NaN].some((x) => x !== x));
