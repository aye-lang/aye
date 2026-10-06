// super() takes the object a parent constructor returns as `this` (what
// HTMLElement's constructor does for custom elements), and new.target is
// the class `new` was applied to through every level.
const made = { tag: 'made' };
function Base() {
  console.log('Base new.target', new.target.name);
  made.kind = new.target.name;
  return made;
}
class A extends Base {
  field = 'a';
  constructor() {
    super();
    console.log('A this is made', this === made, this.field);
  }
}
class B extends A {
  extra = 'b';
}
class C extends B {
  constructor() {
    super();
    console.log('C', this === made, new.target === C, this.kind, this.field, this.extra);
  }
}
const a = new A();
console.log(a === made, a.kind);
const b = new B();
console.log(b === made, b.kind, b.extra);
const c = new C();
console.log(c === made, c.kind);
function Plain() { this.x = 1; }
class D extends Plain {
  constructor() { super(); this.y = 2; }
}
const d = new D();
console.log(d instanceof D, d.x, d.y, Object.getPrototypeOf(d) === D.prototype);
class E extends Base {
  constructor() {
    const f = () => super();
    f();
    console.log('E arrow super', this === made);
  }
}
new E();
console.log(Reflect.construct(A, [], B) === made, made.kind);
