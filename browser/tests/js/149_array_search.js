const a = [1, 2, 3, 2, 1, NaN];
console.log(a.indexOf(2), a.indexOf(2, 2), a.indexOf(9), a.indexOf(NaN), a.indexOf(1, -2));
console.log(a.lastIndexOf(2), a.lastIndexOf(1, 3), a.lastIndexOf(NaN));
console.log(a.includes(3), a.includes(NaN), a.includes('3'), a.includes(1, 5));
console.log([undefined].includes(), [, 1].includes(undefined), [, 1].indexOf(undefined));
const objs = [{ id: 1 }, { id: 2 }, { id: 3 }, { id: 2 }];
console.log(objs.find((o) => o.id === 2), objs.findIndex((o) => o.id === 2), objs.find((o) => o.id === 9), objs.findIndex((o) => o.id === 9));
console.log(objs.findLast((o) => o.id === 2) === objs[3], objs.findLastIndex((o) => o.id === 2), objs.findLast((o) => o.id > 5));
console.log([0, -0].indexOf(-0), ['a', 'b'].indexOf('b', -1));
