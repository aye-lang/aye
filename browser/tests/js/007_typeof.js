console.log(typeof 1, typeof 1.5, typeof NaN, typeof Infinity);
console.log(typeof 'x', typeof '', typeof true, typeof undefined);
console.log(typeof null, typeof {}, typeof [], typeof function () {});
console.log(typeof (() => 1), typeof class {}, typeof Math, typeof JSON);
console.log(typeof notDeclaredAnywhere, typeof new Number(3), typeof String(3));
console.log(typeof typeof 1, typeof Date, typeof new Date(0));
let u;
console.log(typeof u, typeof console.log, typeof Object.keys);
