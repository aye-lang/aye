function Person(name) {
  this.name = name;
  this.say = function () { return 'I am ' + this.name; };
}
const p = new Person('Zed');
console.log(p.name, p.say());
function Ret() { this.a = 1; return { b: 2 }; }
console.log(new Ret());
function RetPrim() { this.a = 1; return 5; }
console.log(new RetPrim());
const o = {
  inner: { val: 'inner', f() { return this.val; } },
  val: 'outer',
};
console.log(o.inner.f());
const g = o.inner.f;
console.log(g.call(o));
