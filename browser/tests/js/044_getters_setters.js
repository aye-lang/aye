const temp = {
  _c: 0,
  get f() { return this._c * 9 / 5 + 32; },
  set f(v) { this._c = (v - 32) * 5 / 9; },
};
temp.f = 212;
console.log(temp._c, temp.f);
const log = [];
const obj = {
  _v: 1,
  get v() { log.push('get'); return this._v; },
  set v(x) { log.push('set ' + x); this._v = x; },
};
obj.v = obj.v + 1;
obj.v++;
console.log(obj._v, log);
const ro = { get only() { return 'r'; } };
try { ro.only = 'changed'; } catch (e) {}
console.log(ro.only);
const desc = Object.getOwnPropertyDescriptor(temp, 'f');
console.log(typeof desc.get, typeof desc.set, desc.enumerable, desc.configurable);
console.log(Object.keys(temp));
