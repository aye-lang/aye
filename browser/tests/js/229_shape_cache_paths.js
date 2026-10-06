// Warm property and method caches, then change what they read: a
// function's own name/length over a parent's, __proto__, accessors.
function A() {}
Object.defineProperty(A, 'name', { value: 'Renamed' });
Object.defineProperty(A, 'length', { value: 7 });
function B(x, y) {}
Object.setPrototypeOf(B, A);
const out = [];
for (let i = 0; i < 3; i++) out.push(B.name + '/' + B.length);
console.log(out.join(' '));
class P { static get tag() { return 'p'; } }
Object.defineProperty(P, 'name', { value: 'PP' });
class Q extends P {}
let s = '';
for (let i = 0; i < 3; i++) s += Q.name + ' ';
console.log(s);
// __proto__ setter changes proto: method cache must follow
const p1 = { m() { return 1; } }, p2 = { m() { return 2; } };
const o = { a: 1 };
o.__proto__ = p1;
let r = [];
for (let i = 0; i < 4; i++) { if (i == 2) o.__proto__ = p2; r.push(o.m()); }
console.log(r.join(','));
// defineProperty get: undefined on cached accessor
const g = { get v() { return 5; }, set v(x) {} };
let t = [];
for (let i = 0; i < 4; i++) { if (i == 2) Object.defineProperty(g, 'v', { get: undefined }); t.push(String(g.v)); }
console.log(t.join(','));
// data -> accessor in place, cached slot
const d = { x: 1 };
let u = [];
for (let i = 0; i < 4; i++) { if (i == 2) Object.defineProperty(d, 'x', { get() { return 9; } }); u.push(d.x); }
console.log(u.join(','));
// Reflect.construct with newTarget: proto method cache
class C1 { who() { return 'c1'; } }
class C2 { who() { return 'c2'; } }
let w = [];
for (let i = 0; i < 4; i++) { const x = Reflect.construct(C1, [], i < 2 ? C1 : C2); w.push(x.who()); }
console.log(w.join(','));
// frozen / non-writable writes
const f = { k: 1 };
for (let i = 0; i < 3; i++) { if (i == 1) Object.freeze(f); f.k = 10 + i; }
console.log(f.k);
const nw = { k: 1 };
for (let i = 0; i < 3; i++) { if (i == 1) Object.defineProperty(nw, 'k', { writable: false }); nw.k = 10 + i; }
console.log(nw.k);
// delete then re-add
const dd = { a: 1, b: 2 };
let v = [];
for (let i = 0; i < 4; i++) { if (i == 2) { delete dd.a; dd.a = 5; } v.push(dd.a + dd.b); }
console.log(v.join(','));
