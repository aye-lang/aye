const proto = { greet() { return 'hi ' + this.n; } };
const o = Object.create(proto);
o.n = 'x';
console.log(o.greet(), Object.getPrototypeOf(o) === proto, o.hasOwnProperty('greet'));
const bare = Object.create(null);
bare.k = 1;
console.log(Object.keys(bare), typeof bare.toString, 'k' in bare);
const withProps = Object.create(proto, {
  n: { value: 'defined', enumerable: true },
  hidden: { value: 42 },
});
console.log(withProps.greet(), withProps.hidden, Object.keys(withProps));
const chain = Object.create(Object.create(Object.create({ deep: 'yes' })));
console.log(chain.deep);
