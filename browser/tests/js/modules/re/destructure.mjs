const src = { a: 1, b: [2, 3, 30, 31], c: { d: 4 }, e: 5, f: 6 };
export const { a, b: [b0, b1 = 'dflt', ...bRest], c: { d: renamedD }, ...others } = src;
export let [first, , third, fourth = 'fourth default'] = ['one', 'two', 'three'];
export var { missing = 'default value' } = {};
export function setFirst(v) { first = v; }
