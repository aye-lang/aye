try {
  [1, 2, 3].map((x) => { if (x === 2) throw new Error('map fail at ' + x); return x; });
} catch (e) { console.log(e.message); }
try {
  [1, 2].forEach((x) => { throw new TypeError('forEach ' + x); });
} catch (e) { console.log(e.name, e.message); }
try {
  [3, 1, 2].sort((a, b) => { throw new RangeError('cmp'); });
} catch (e) { console.log(e.name, e.message); }
try {
  JSON.parse('{"a":1,"b":2}', (k, v) => { if (k === 'b') throw new Error('reviver ' + v); return v; });
} catch (e) { console.log(e.message); }
try {
  [1, 2, 3].reduce((acc, x) => { if (x === 3) throw 'reduce str'; return acc + x; });
} catch (e) { console.log(e); }
try {
  'abc'.replace(/b/, () => { throw new Error('replacer'); });
} catch (e) { console.log(e.message); }
try {
  JSON.stringify({ toJSON() { throw new Error('toJSON'); } });
} catch (e) { console.log(e.message); }
const results = [];
for (const fn of [() => [0].filter(() => { throw 1; }), () => [0].find(() => { throw 2; }), () => [0].some(() => { throw 3; })]) {
  try { fn(); } catch (e) { results.push(e); }
}
console.log(results);
