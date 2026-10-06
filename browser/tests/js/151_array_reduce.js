const a = [1, 2, 3, 4];
console.log(a.reduce((s, x) => s + x), a.reduce((s, x) => s + x, 10), a.reduce((p, x) => p * x, 1));
console.log(a.reduceRight((s, x) => s + x, ''), a.reduce((s, x) => s + x, ''));
console.log([[1, 2], [3], [4, 5]].reduce((acc, x) => acc.concat(x), []));
console.log(['a', 'b', 'a'].reduce((m, x) => ({ ...m, [x]: (m[x] || 0) + 1 }), {}));
console.log([5].reduce((s, x) => s + x), [].reduce((s, x) => s + x, 'init'));
try { [].reduce((s, x) => s + x); } catch (e) { console.log(e.name); }
const trace = [];
[1, 2, 3].reduce((acc, x, i) => { trace.push(`${acc}+${x}@${i}`); return acc + x; });
console.log(trace);
console.log([3, 9, 2].reduce((m, x) => Math.max(m, x), -Infinity));
console.log([[1, 2], [3, 4]].reduceRight((acc, x) => acc.concat(x), []));
