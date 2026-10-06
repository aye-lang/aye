console.log([1, 2].concat([3], 4, [[5]], 'six'));
console.log([].concat(), [1].concat([]), [[1]].concat([[2]]));
console.log([1, 2, 3].join(), [1, 2, 3].join(''), [1, 2, 3].join(' - '), [].join(), [null, undefined, 1].join('|'));
console.log([[1, [2]], 3].join(';'), [1.5, true, 'x', {}].join());
const r = [1, 2, 3];
const rr = r.reverse();
console.log(r, rr === r);
console.log([].reverse(), ['a'].reverse(), [1, 2, 3, 4].reverse());
console.log([1, 2, 3].toString(), String([[], [[]], [1]]));
