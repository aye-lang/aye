console.log(JSON.stringify({ a: 1, b: 'two', c: true, d: null, e: [1, 2], f: { g: {} } }));
console.log(JSON.stringify([1, 'a', null, true, [], {}]));
console.log(JSON.stringify('str'), JSON.stringify(42), JSON.stringify(null), JSON.stringify(true));
console.log(JSON.stringify(undefined), JSON.stringify(() => 1), typeof JSON.stringify(undefined));
console.log(JSON.stringify([undefined, () => 1, NaN, Infinity, -0]));
console.log(JSON.stringify({ u: undefined, f() {}, n: NaN, i: -Infinity, ok: 1 }));
console.log(JSON.stringify('quote " backslash \\ newline \n tab \t ctrl \u0001 é 😀'));
console.log(JSON.stringify({ 'key with "quotes"': 1 }), JSON.stringify({ 2: 'b', 1: 'a' }));
console.log(JSON.stringify(new Map([[1, 2]])), JSON.stringify(new Set([1])), JSON.stringify([new Date(0)]));
console.log(JSON.stringify('  '), JSON.stringify('\ud800'));
