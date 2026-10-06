class MathUtil {
  static square(x) { return x * x; }
  static cube(x) { return x * MathUtil.square(x); }
  static create() { return new this(); }
  hello() { return 'instance'; }
}
console.log(MathUtil.square(4), MathUtil.cube(3), MathUtil.create().hello());
console.log(typeof new MathUtil().square, typeof MathUtil.square);
class Registry {
  static count = 0;
  static items = [];
  constructor(n) { Registry.count++; Registry.items.push(n); }
}
new Registry('a'); new Registry('b');
console.log(Registry.count, Registry.items);
class Sub extends MathUtil {}
console.log(Sub.square(5), Sub.create() instanceof Sub);
