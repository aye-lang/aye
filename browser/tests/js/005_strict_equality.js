console.log(1 === 1, 1 === '1', null === undefined, null === null, undefined === undefined);
console.log(NaN === NaN, 0 === -0, +0 === -0);
const a = { x: 1 };
const b = { x: 1 };
const c = a;
console.log(a === b, a === c, a == b, a == c);
console.log([] === [], 'abc' === 'ab' + 'c');
console.log(1 !== 2, '1' !== 1, a !== c);
console.log(Object.is(NaN, NaN), Object.is(0, -0), Object.is('x', 'x'));
