console.log(-'', -null, -undefined, -[], -'-0');
console.log(Object.is(-'', -0), Object.is(-null, -0));
console.log(+'1e3', +'.5', +'5.', +'0b101', +'0o17', +'-0x10', +'Infinity', +'-Infinity');
console.log(!'false', !!'false', !!NaN, !!-1);
let x = 5;
console.log(-x, +x, ~x, !x, typeof -x);
console.log(-(-(-1)), +-+-1);
