console.log([3, 1, 10, 2, 21].sort());
console.log(['b', 'a', 'C', 'B', 'c'].sort());
console.log([true, false, 'x', 1, null, undefined, 'a'].sort());
console.log([undefined, 3, undefined, 1].sort(), [, 2, , 1].sort());
console.log(['é', 'e', 'z', 'a'].sort(), ['10', '9', '1'].sort());
console.log([-1, -10, 5, 0].sort(), [0.5, 0.25, 1e21, 1e-7].sort());
const a = [3, 2, 1];
console.log(a.sort() === a);
