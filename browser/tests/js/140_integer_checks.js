console.log(Number.isInteger(5), Number.isInteger(5.0), Number.isInteger(5.5), Number.isInteger('5'), Number.isInteger(Infinity));
console.log(Number.isInteger(2 ** 60), Number.isInteger(-0), Number.isInteger(NaN));
console.log(Number.isSafeInteger(2 ** 53 - 1), Number.isSafeInteger(2 ** 53), Number.isSafeInteger(-(2 ** 53 - 1)), Number.isSafeInteger(1.5));
console.log(Number.MAX_SAFE_INTEGER, Number.MIN_SAFE_INTEGER, Number.MAX_SAFE_INTEGER + 2);
console.log(Number.EPSILON, Number.EPSILON > 0, 0.1 + 0.2 - 0.3 < Number.EPSILON, 1 + Number.EPSILON > 1);
console.log(Number.POSITIVE_INFINITY, Number.NEGATIVE_INFINITY, Number.NaN);
console.log(Math.abs(0.3 - (0.1 + 0.2)) < Number.EPSILON);
