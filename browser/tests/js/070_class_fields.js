class Counter {
  count = 0;
  step = 2;
  label = 'c' + this.step;
  inc = () => { this.count += this.step; return this; };
  static instances = 0;
  static label = 'Counter';
  constructor() { Counter.instances++; }
}
const c = new Counter();
c.inc().inc();
console.log(c.count, c.label, Counter.instances, Counter.label);
console.log(c);
const d = new Counter();
console.log(Counter.instances, d.inc === c.inc);
class WithInit {
  a = 1;
  b;
  ['comp' + 'uted'] = 'yes';
}
console.log(new WithInit());
