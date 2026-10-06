class Base {
  field = 'base field';
  constructor() { this.fromCtor = this.describe(); }
  describe() { return 'Base.describe'; }
}
class Derived extends Base {
  extra = 'derived field';
  describe() { return 'Derived.describe extra=' + this.extra; }
}
const d = new Derived();
console.log(d.fromCtor);
console.log(d.field, d.extra);
class NoCtor extends Base {}
console.log(new NoCtor().field);
class ArgsPass extends Array {}
const ap = new ArgsPass();
ap.push(1, 2);
console.log(ap.length, ap instanceof Array, Array.isArray(ap));
try {
  class Bad extends Base { constructor() { this.x = 1; super(); } }
  new Bad();
} catch (e) {
  console.log(e.name);
}
