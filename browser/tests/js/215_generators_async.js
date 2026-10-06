function* g(a) { const x = yield a + 1; console.log('got', x); try { yield 2; } finally { console.log('cleanup'); } return 3; }
const it = g(1);
console.log(it.next('ignored'), it.next('hello'), it.return(9), it.next());
console.log([...g(10)]);
function* inf() { let i = 0; while (true) yield i++; }
for (const v of inf()) { if (v > 3) break; console.log('v', v); }
function* del() { const r = yield* g(5); console.log('delegate result', r); yield 'end'; }
console.log([...del()]);
const t = g(0); t.next(); try { t.throw(new Error('boom')); } catch (e) { console.log('thrown out', e.message); }
console.log(Object.prototype.toString.call(g(0)), typeof g(0).next, g(0)[Symbol.iterator]() !== undefined);
class C { *[Symbol.iterator]() { yield 1; yield 2; } } console.log([...new C()]);
const o = { *gen() { yield 'm'; } }; console.log([...o.gen()]);
async function f(x) { const a = await x; const b = await Promise.resolve(a * 2); return a + b; }
f(3).then(v => console.log('async result', v));
async function bad() { await null; throw new TypeError('async fail'); }
bad().catch(e => console.log('caught', e.message));
const arrow = async () => { try { await Promise.reject(new Error('rej')); } catch (e) { return 'handled ' + e.message; } };
arrow().then(console.log);
console.log('sync end');
async function order() { console.log('o1'); await undefined; console.log('o3'); }
order(); console.log('o2');
async function* ag() { yield 1; await null; yield 2; }
(async () => { const out = []; for await (const v of ag()) out.push(v); console.log('for await', out); for await (const v of [Promise.resolve('a'), 'b']) out.push(v); console.log(out); })();
class K { static #count = 0; #x = 1; static inc() { return ++K.#count; } get x() { return this.#x; } has(o) { return #x in o; } static { K.ready = true; } }
const k = new K(); console.log(k.x, K.inc(), K.inc(), k.has(k), k.has({}), Object.keys(k), K.ready);
function tag(s, ...v) { return s.raw.join('|') + ':' + v.join(','); }
console.log(tag`a${1}b\n${2}c`, String.raw`x\ty`);
const same = () => tag2`x`; function tag2(s) { return s; } console.log(same() === same());
function NT() { return new.target === NT; } console.log(new NT() instanceof NT, NT());
