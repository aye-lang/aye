const timer = {
  label: 'T',
  items: [1, 2, 3],
  show() { return this.items.map((i) => this.label + i); },
  showOld() { const self = this; return this.items.map(function (i) { return self.label + i; }); },
};
console.log(timer.show(), timer.showOld());
const obj = {
  val: 10,
  getArrow() { return () => this.val; },
};
const arrowFn = obj.getArrow();
console.log(arrowFn());
const other = { val: 99, f: arrowFn };
console.log(other.f());
console.log(arrowFn.call({ val: 5 }));
class Counter {
  constructor() { this.n = 0; this.inc = () => { this.n++; return this.n; }; }
}
const c = new Counter();
const inc = c.inc;
inc(); inc();
console.log(c.n);
