function intro(greeting, punct) { return greeting + ', ' + this.name + punct; }
const p = { name: 'Ann' };
console.log(intro.call(p, 'Hi', '!'), intro.apply(p, ['Hello', '?']));
console.log(Math.max.apply(null, [3, 7, 2]), Math.min.call(null, 4, 1, 8));
const arrLike = { 0: 'a', 1: 'b', length: 2 };
console.log(Array.prototype.join.call(arrLike, '-'), Array.prototype.map.call(arrLike, (x) => x.toUpperCase()));
console.log(Object.prototype.toString.call([]), Object.prototype.toString.call(null), Object.prototype.toString.call(1));
function sumThis(...n) { return this.base + n.reduce((a, b) => a + b, 0); }
console.log(sumThis.call({ base: 100 }, 1, 2), sumThis.apply({ base: 0 }, [5, 5]));
console.log(String.prototype.toUpperCase.call('abc'));
