const wm = new WeakMap(); const k = {}; wm.set(k, 1); console.log(wm.get(k), wm.has({}), wm instanceof WeakMap);
try { wm.set(1, 2); } catch (e) { console.log(e.constructor.name, e.message); }
const ws = new WeakSet([k]); console.log(ws.has(k));
console.log(Reflect.ownKeys({ a: 1, [Symbol.iterator]: 2 }).length, Reflect.has({ x: 1 }, 'x'), Reflect.apply(Math.max, null, [1, 3, 2]));
class P { constructor(x) { this.x = x; } } console.log(Reflect.construct(P, [5]).x);
console.log([1, 2, 3, 4, 5].copyWithin(0, 3), [1, 2, 3, 4, 5].copyWithin(1, 0, 3));
console.log(String.raw({ raw: ['a', 'b', 'c'] }, 1, 2));
const d = new Date(Date.UTC(2020, 0, 31, 10, 20, 30));
d.setMonth(1); console.log(d.toISOString());
d.setFullYear(2021, 5, 15); console.log(d.toISOString());
d.setHours(23, 59); console.log(d.toISOString());
d.setDate(0); console.log(d.toISOString());
console.log(d.toUTCString(), '|', d.toDateString());
const u = new Uint8Array([1, 256, -1]); console.log(u.length, u[0], u[1], u[2], u instanceof Uint8Array, Uint8Array.BYTES_PER_ELEMENT);
const z = new Uint16Array(3); console.log(Array.from(z));
console.log(encodeURIComponent('a b&c/é'), encodeURI('http://x.y/a b?q=1&r=é#h'), decodeURIComponent('%E2%82%AC%20x'), decodeURI('%23%20'));
try { decodeURIComponent('%'); } catch (e) { console.log(e.name, e.message); }
console.log(escape('a b+é'), unescape('%u00e9%20'));
console.log(new Function('a', 'b', 'return a * b')(6, 7), Function('return 5')(), eval('1 + 2'), eval(42));
console.log(typeof eval('(function(){ return this })'));
var gx = 1; eval('var gy = gx + 1'); console.log(gy);
console.log(/(?<=\$)\d+/.exec('cost $42')[0], /(?<!a)b/.test('ab'), /(?<!a)b/.test('cb'));
console.log(/\p{L}+/u.exec('héllo wörld')[0], /\P{L}/u.test('abc'), /[\p{N}]/u.test('x5'));
console.log(/./s.dotAll, /./.dotAll, /a/u.unicode, /a/d.hasIndices);
console.log(new Intl.NumberFormat('en-US').format(1234567.891), new Intl.NumberFormat('en', { style: 'percent' }).format(0.25));
console.log((1234.5).toLocaleString(), new Intl.PluralRules('en').select(1), new Intl.ListFormat('en').format(['a', 'b', 'c']));
console.log(Object.getOwnPropertyDescriptors({ a: 1 }));
console.log(new AggregateError([1], 'm').errors, typeof new Error('x').stack);
const big = 'a' + 'x'.repeat(100000) + 'b'; console.log(/a.*b/.test(big), /a[\s\S]*?b/.exec(big)[0].length);
