const Foo = class {
  hi() { return 'anon class'; }
};
console.log(new Foo().hi(), Foo.name);
const Bar = class Named {
  who() { return Named.name; }
};
console.log(new Bar().who(), Bar.name, typeof Named);
function mixin(Base) {
  return class extends Base {
    mixed() { return 'mixed into ' + super.base(); }
  };
}
class Base0 { base() { return 'base0'; } }
const M = mixin(Base0);
console.log(new M().mixed(), new M() instanceof Base0);
const classes = [1, 2].map((n) => class { get n() { return n; } });
console.log(new classes[0]().n, new classes[1]().n);
