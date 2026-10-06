console.log((123.456).toPrecision(4), (123.456).toPrecision(2), (0.00012345).toPrecision(2), (123456).toPrecision(2));
console.log((1).toPrecision(5), (1e21).toPrecision(3), (99.99).toPrecision(3), (0).toPrecision(3));
console.log((123.456).toPrecision(), (1.5).toPrecision(1), (2.5).toPrecision(1), (-1.25).toPrecision(2));
console.log((123456).toExponential(), (123456).toExponential(2), (0.00015).toExponential(1), (0).toExponential(2));
console.log((1.5).toExponential(0), (-12.34).toExponential(3), (1e-10).toExponential(), (5).toExponential(3));
console.log((NaN).toPrecision(2), (Infinity).toExponential(2));
