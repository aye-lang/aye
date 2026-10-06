function sum(...nums) { return nums.reduce((a, b) => a + b, 0); }
console.log(sum(), sum(1), sum(1, 2, 3, 4));
function tag(first, ...rest) { return first + ':' + rest.join('|'); }
console.log(tag('a'), tag('a', 'b', 'c'));
const f = (...args) => args.length;
console.log(f(), f(undefined), f(1, 2, 3));
function isArr(...r) { return Array.isArray(r); }
console.log(isArr());
console.log(sum.length, tag.length);
const max = (...xs) => xs.length ? Math.max(...xs) : null;
console.log(max(3, 9, 2), max());
