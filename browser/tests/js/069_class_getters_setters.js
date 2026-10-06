class Temp {
  constructor(c) { this._c = c; }
  get f() { return this._c * 9 / 5 + 32; }
  set f(v) { this._c = (v - 32) * 5 / 9; }
  get c() { return this._c; }
}
const t = new Temp(100);
console.log(t.f, t.c);
t.f = 32;
console.log(t.c);
console.log(Object.keys(t), 'f' in t, t.hasOwnProperty('f'));
class Lazy {
  get value() {
    const v = 42;
    Object.defineProperty(this, 'value', { value: v });
    console.log('computed once');
    return v;
  }
}
const l = new Lazy();
console.log(l.value, l.value);
