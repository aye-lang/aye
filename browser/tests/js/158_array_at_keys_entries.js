const a = ['a', 'b', 'c'];
console.log(a.at(0), a.at(-1), a.at(5), a.at(-4), a.at(1.7));
console.log([...a.keys()], [...a.values()], [...a.entries()]);
for (const [i, v] of a.entries()) console.log(i, v);
console.log([...[, 'x'].keys()], [...[, 'x'].entries()]);
const it = a.keys();
console.log(it.next().value, it.next().value, it.next().value, it.next().done);
