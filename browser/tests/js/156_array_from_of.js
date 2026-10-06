console.log(Array.from('abc'), Array.from({ length: 3 }), Array.from({ length: 3 }, (_, i) => i * i));
console.log(Array.from({ length: 2, 0: 'x', 1: 'y' }), Array.from([1, 2], (x) => x * 10));
console.log(Array.from(new Set([1, 1, 2])), Array.from(new Map([[1, 'a']])), Array.from('😀é'));
console.log(Array.of(7), Array.of(1, 2, 3), Array.of(), Array(3).length, Array(3, 4));
console.log(Array.isArray([]), Array.isArray({ length: 0 }), Array.isArray('abc'), Array.isArray(Array.prototype));
console.log(Array.from({ length: '2' }), Array.from(5), Array.from(null === null ? [] : 1));
