const ctx = { mult: 3 };
console.log([1, 2, 3].map(function (x) { return x * this.mult; }, ctx));
const seen = [];
[10, 20].forEach(function (x) { seen.push(this.tag + x); }, { tag: '#' });
console.log(seen);
console.log([1, 2, 3, 4].filter(function (x) { return x > this.min; }, { min: 2 }));
class Btn {
  constructor(l) { this.label = l; }
  handle() { return 'clicked ' + this.label; }
}
const b = new Btn('save');
const run = (cb) => cb();
console.log(run(() => b.handle()), run(b.handle.bind(b)));
