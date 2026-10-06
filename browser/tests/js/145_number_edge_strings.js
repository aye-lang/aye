console.log(String(1 / 3), String(2 / 3), String(1e-7 * 3), String(123e-20));
console.log(0.1.toString(), (1e21).toLocaleString === undefined, (100).toString().length);
console.log(Number('1e1000'), Number('-1e1000'), Number('1e-1000'));
console.log(5e-324 / 2, 5e-324 * 0.5, Number.MIN_VALUE / 2 === 0);
console.log(+(1e21 + ''), 1e21 + 1 === 1e21);
console.log([1.5, 2.25, -0.125].map(String), [1e300 * 1e300, -(1e300 * 1e300)].map(String));
