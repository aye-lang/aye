const o = {
  name: 'obj',
  hello() { return 'hi from ' + this.name; },
  nested: { name: 'nested', who() { return this.name; } },
};
console.log(o.hello(), o.nested.who(), o['hello']());
const other = { name: 'other', hello: o.hello };
console.log(other.hello());
function shared() { return this.v * 2; }
const a = { v: 1, f: shared }, b = { v: 21, f: shared };
console.log(a.f(), b.f());
const chain = {
  total: 0,
  add(n) { this.total += n; return this; },
};
console.log(chain.add(1).add(2).add(3).total);
