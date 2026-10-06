// Proxy: traps for property access, calls and construction; the target
// without a trap. Piano's SDK (BBC) and reactive libraries (Vue, MobX,
// Immer) need it.
const log = [];
const target = { a: 1, b: 2 };
const p = new Proxy(target, {
  get(t, k, r) { log.push('get ' + String(k)); return k in t ? t[k] : 'none'; },
  set(t, k, v) { log.push('set ' + k); t[k] = v * 10; return true; },
  has(t, k) { return k === 'secret' || k in t; },
  deleteProperty(t, k) { log.push('delete ' + k); delete t[k]; return true; },
});
console.log(p.a, p.zz, 'secret' in p, 'a' in p, 'q' in p);
p.c = 3;
console.log(target.c, delete p.b, 'b' in target);
console.log(log.join(', '));
// No traps: the target itself.
const plain = new Proxy([1, 2, 3], {});
plain.push(4);
console.log(plain.length, plain[3], Array.isArray(plain), JSON.stringify(plain), typeof plain);
// ownKeys and getOwnPropertyDescriptor: Object.keys, for-in, spread, JSON.
const keyed = new Proxy({ x: 1, y: 2, hidden: 3 }, {
  ownKeys(t) { return Reflect.ownKeys(t).filter(k => k !== 'hidden'); },
});
const seen = [];
for (const k in keyed) seen.push(k);
console.log(Object.keys(keyed).join(), seen.join(), JSON.stringify(keyed), JSON.stringify({ ...keyed }), Object.entries(keyed).length);
// apply and construct.
function add(a, b) { return a + b; }
const traced = new Proxy(add, { apply(t, thisArg, args) { return 'called ' + t(...args); } });
console.log(traced(2, 3), typeof traced, traced.call(null, 1, 1), traced.name, traced.length);
class Point { constructor(x) { this.x = x; } }
const P = new Proxy(Point, { construct(t, args, nt) { const o = new t(args[0] * 2); o.made = true; return o; } });
const pt = new P(5);
console.log(pt.x, pt.made, pt instanceof Point, new (new Proxy(Point, {}))(7).x);
// The prototype: instanceof, getPrototypeOf trap.
const withProto = new Proxy(new Point(1), {});
console.log(withProto instanceof Point, Object.getPrototypeOf(withProto) === Point.prototype);
const fakeProto = new Proxy({}, { getPrototypeOf() { return Array.prototype; } });
console.log(Object.getPrototypeOf(fakeProto) === Array.prototype);
// defineProperty, getOwnPropertyDescriptor, hasOwnProperty.
const defs = [];
const dp = new Proxy({}, {
  defineProperty(t, k, d) { defs.push(k); return Reflect.defineProperty(t, k, d); },
  getOwnPropertyDescriptor(t, k) { return k === 'ghost' ? { value: 'boo', enumerable: true, configurable: true } : Reflect.getOwnPropertyDescriptor(t, k); },
});
Object.defineProperty(dp, 'v', { value: 9, enumerable: true });
console.log(defs.join(), dp.v, dp.hasOwnProperty('ghost'), Object.getOwnPropertyDescriptor(dp, 'ghost').value, Object.hasOwn(dp, 'v'));
// Revocable.
const { proxy, revoke } = Proxy.revocable({ k: 1 }, {});
console.log(proxy.k);
revoke();
let err = '';
try { proxy.k; } catch (e) { err = e.constructor.name + ': ' + e.message; }
console.log(err);
// Errors: without new, non-object arguments, a throwing trap.
const errs = [];
try { Proxy({}, {}); } catch (e) { errs.push(e.constructor.name); }
try { new Proxy(1, {}); } catch (e) { errs.push(e.constructor.name); }
try { new Proxy({}, { get() { throw new RangeError('trap'); } }).x; } catch (e) { errs.push(e.message); }
try { new Proxy({}, { has() { throw new Error('has'); } }); 'x' in new Proxy({}, { has() { throw new Error('has'); } }); } catch (e) { errs.push(e.message); }
console.log(errs.join());
// A proxy as a prototype; nested proxies; the piano SDK's pattern.
const base = new Proxy({}, { get(t, k) { return 'from proxy ' + String(k); } });
const child = Object.create(base);
child.own = 1;
console.log(child.own, child.other);
const nested = new Proxy(new Proxy({ deep: 'yes' }, {}), {});
console.log(nested.deep, 'deep' in nested);
let H;
const J = new Proxy({}, { get: function (e, n) { return n === 'isv' || H; }, apply: function () { return J; } });
H = function () { return J; };
console.log(J.isv, typeof J.other, J.other() === J);
console.log(Object.prototype.toString.call(new Proxy({}, {})), String(new Proxy(function f() {}, {})).includes('native code'));
