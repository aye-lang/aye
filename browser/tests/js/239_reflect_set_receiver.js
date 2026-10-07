// Reflect.set with a receiver that is a proxy (Vue's reactive `set` trap
// passes it): data properties are defined on the receiver, whose [[Set]]
// is not called again; setters run with it as `this`.
const target = { a: 1, get g() { return this.a; }, set s(v) { this.a = v; } };
const log = [];
const p = new Proxy(target, {
  set(t, k, v, r) { log.push('set ' + String(k)); return Reflect.set(t, k, v, r); },
  defineProperty(t, k, d) { log.push('define ' + String(k)); return Reflect.defineProperty(t, k, d); },
});
p.a = 2; p.b = 3; p.s = 4;
console.log(log.join(', '), JSON.stringify(target));
const frozen = Object.freeze({ x: 1 });
console.log(Reflect.set({}, 'y', 1, frozen), Reflect.set({ get g() { return 1; } }, 'g', 1, {}), Reflect.set({}, 'n', 1, 5));
const recv = {};
console.log(Reflect.set({ z: 1 }, 'z', 5, recv), recv.z, Object.keys(recv).join());
const ro = Object.defineProperty({}, 'w', { value: 1, writable: false });
console.log(Reflect.set(ro, 'w', 2, {}), Reflect.set({}, 'w', 2, ro));
