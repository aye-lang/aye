const wm = new WeakMap(); const k = {}; wm.set(k, 1); console.log(wm.get(k), wm.has({}), wm instanceof WeakMap);
try { wm.set(1, 2); } catch (e) { console.log(e.constructor.name, e.message); }
const ws = new WeakSet([k]); console.log(ws.has(k), ws.has(1), ws.delete(k), ws.has(k));
console.log(Object.prototype.toString.call(wm), String(new WeakRef(k)), String(new FinalizationRegistry(() => {})));
console.log(wm.get(1), wm.delete(1), wm.delete(k), wm.has(k), wm.get(k));
const s = Symbol('s'); wm.set(s, 'sym'); console.log(wm.get(s));
try { wm.set(Symbol.for('r'), 1); } catch (e) { console.log(e.constructor.name, e.message); }
try { WeakMap(); } catch (e) { console.log(e.constructor.name, e.message); }
try { WeakMap.prototype.get.call({}, k); } catch (e) { console.log(e.constructor.name); }
class Sub extends WeakMap { constructor(e) { super(e); this.extra = 1; } }
const sub = new Sub([[k, 'x']]); console.log(sub.get(k), sub.extra, sub instanceof WeakMap, sub instanceof Sub);
const r = new WeakRef(k); console.log(r.deref() === k, WeakRef.length, FinalizationRegistry.length, WeakMap.length, WeakSet.length);
const fr = new FinalizationRegistry(h => console.log('cleanup', h));
console.log(fr.register({}, 'h1', k), fr.unregister(k), fr.unregister(k));
try { fr.register(k, k); } catch (e) { console.log(e.constructor.name); }
try { fr.register(1, 2); } catch (e) { console.log(e.constructor.name); }
try { fr.register({}, 2, 3); } catch (e) { console.log(e.constructor.name); }
try { fr.unregister(3); } catch (e) { console.log(e.constructor.name); }
try { new WeakRef(1); } catch (e) { console.log(e.constructor.name); }
try { new FinalizationRegistry(1); } catch (e) { console.log(e.constructor.name); }
const wm2 = new WeakMap([[k, 1], [s, 2]]); console.log(wm2.get(k), wm2.get(s));
try { new WeakMap([1]); } catch (e) { console.log(e.constructor.name); }
console.log(Object.getOwnPropertyNames(WeakMap.prototype).sort().join(), Object.getOwnPropertyNames(FinalizationRegistry.prototype).sort().join());
console.log(typeof WeakMap, WeakMap.name, new WeakRef(s).deref() === s);
