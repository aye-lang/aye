class A {
  who() { return 'A'; }
  chain() { return this.who(); }
  static s() { return 'sA'; }
}
class B extends A {
  who() { return 'B<' + super.who(); }
  static s() { return 'sB<' + super.s(); }
}
class C extends B {
  who() { return 'C<' + super.who(); }
}
console.log(new C().chain(), new B().chain(), new A().chain());
console.log(C.s(), B.s());
const objs = [new A(), new B(), new C()];
console.log(objs.map((o) => o.who()));
const m = { __proto__: { greet() { return 'proto greet'; } }, greet() { return 'own+' + super.greet(); } };
console.log(m.greet());
