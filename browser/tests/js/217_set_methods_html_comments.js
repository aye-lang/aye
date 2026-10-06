<!-- an HTML-like comment of a classic script
const a = new Set([1, 2, 3]), b = new Set([3, 2, 4]);
console.log(a.union(b), a.intersection(b), b.intersection(new Set([4, 3, 2, 1, 0])), a.difference(b), a.symmetricDifference(b));
console.log(a.isSubsetOf(new Set([0, 1, 2, 3])), a.isSupersetOf(new Set([1])), a.isDisjointFrom(new Set([9])), a.isDisjointFrom(b));
const m = new Map([[3, 'x'], [7, 'y']]);
console.log(a.union(m), a.intersection(m));
try { a.union([1]); } catch (e) { console.log(e.constructor.name); }
--> also a comment: `-->` first on a line
/* a block comment, then */ --> one more
async function* gen() { yield 'a'; yield 'b'; }
(async () => {
  console.log('fromAsync', await Array.fromAsync([1, Promise.resolve(2)], x => x * 10));
  console.log('fromAsync gen', await Array.fromAsync(gen()));
  console.log('fromAsync array-like', await Array.fromAsync({ length: 2, 0: 'x', 1: Promise.resolve('y') }));
})();
let x = 3;
x-->0 ? console.log('x-->0 is a decrement, then a comparison', x) : 0;
