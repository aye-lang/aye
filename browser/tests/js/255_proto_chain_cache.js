// Reads found 2+ prototypes up are cached per site; warm each site,
// then change the chain: add/change/delete on a prototype, a key added
// in between, setPrototypeOf, accessor replaced, function name/length.
const read = (o) => o.k;
const call = (o) => o.m();
function warm(f, o) { let r; for (let i = 0; i < 5; i++) r = f(o); return r; }
const C = { k: 'c', m() { return 'mc'; } };
const B = Object.create(C);
const A = Object.create(B);
const o = Object.create(A);
const log = [];
log.push(warm(read, o), warm(call, o));
C.k = 'c2';                                   // value changed
log.push(warm(read, o));
B.k = 'b';                                    // key added in between
log.push(warm(read, o));
delete B.k;                                   // delete (dictionary)
log.push(warm(read, o));
delete C.k;                                   // deleted at the holder
log.push(String(warm(read, o)));
C.k = 'c3';                                   // added back
log.push(warm(read, o));
A.m = function () { return 'ma'; };           // shadowed one level up
log.push(warm(call, o));
delete A.m;
log.push(warm(call, o));
const D = { k: 'd', m() { return 'md'; } };
Object.setPrototypeOf(B, D);                  // link changed mid-chain
log.push(warm(read, o), warm(call, o));
o.__proto__ = Object.create(Object.create(C)); // receiver's link changed
log.push(warm(read, o));
console.log(log.join(' '));
// accessor 3 levels up, replaced after warm-up
const G = { get v() { return 1; } };
const g = Object.create(Object.create(Object.create(G)));
const rv = (x) => x.v;
const acc = [warm(rv, g)];
Object.defineProperty(G, 'v', { get() { return 2; }, configurable: true });
acc.push(warm(rv, g));
Object.defineProperty(G, 'v', { value: 3, configurable: true });
acc.push(warm(rv, g));
Object.defineProperty(Object.getPrototypeOf(g), 'v', { get() { return 4; } });
acc.push(warm(rv, g));
console.log(acc.join(' '));
// class hierarchy: methods 1, 2 and 3 levels up, then patched
class P0 { a() { return 'a0'; } b() { return 'b0'; } }
class P1 extends P0 { b() { return 'b1'; } }
class P2 extends P1 {}
class P3 extends P2 {}
const x = new P3();
const ra = (y) => y.a() + y.b();
const cls = [warm(ra, x)];
P0.prototype.a = function () { return 'a0x'; };
cls.push(warm(ra, x));
P2.prototype.a = function () { return 'a2'; };
cls.push(warm(ra, x));
delete P1.prototype.b;
cls.push(warm(ra, x));
Object.setPrototypeOf(P2.prototype, Object.prototype);
cls.push(String(warm((y) => y.b, x)));
console.log(cls.join(' '));
// function name/length through a chain of functions
function F(p, q) {}
const H = Object.create(Object.create(F));
const nl = (y) => y.name + y.length;
console.log(warm(nl, H), warm((y) => typeof y.call, H));
// Object.prototype key and hasOwnProperty via deep chains
const deep = Object.create(Object.create(Object.create({})));
const ts = (y) => y.hasOwnProperty('z');
const xr = (y) => y.extra;
const dl = [warm(ts, deep)];
Object.prototype.extra = 'e';
dl.push(warm(xr, deep));
Object.prototype.extra = 'f';
dl.push(warm(xr, deep));
delete Object.prototype.extra;
dl.push(String(warm(xr, deep)));
console.log(dl.join(' '));
// set through a deep cached site still honours a setter up the chain
let seen = 0;
const S = { set w(v) { seen = v; }, get w() { return seen; } };
const s = Object.create(Object.create(Object.create(S)));
for (let i = 0; i < 4; i++) { s.w = i; s.w += 10; }
console.log(seen, Object.keys(s).length);
