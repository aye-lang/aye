console.log({ a: { b: { c: { d: 1 } } } });
console.log([[[[1]]]], [[[]]], [{}]);
console.log({ list: [1, 2, { deep: [3, { deeper: true }] }] });
console.log({ empty: {}, arr: [], nested: { e: [] } });
const tree = { name: 'root', children: [{ name: 'a', children: [] }, { name: 'b', children: [{ name: 'c' }] }] };
console.log(tree);
