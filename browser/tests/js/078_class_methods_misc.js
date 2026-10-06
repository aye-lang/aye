class K {
  constructor() { this.items = []; }
  add(...xs) { this.items.push(...xs); return this; }
  ['dyn' + 'Method']() { return 'dynamic'; }
  'quoted name'() { return 'quoted'; }
  42() { return 'numeric'; }
}
const k = new K().add(1, 2).add(3);
console.log(k.items, k.dynMethod(), k['quoted name'](), k[42]());
console.log(Object.keys(K.prototype), Object.getOwnPropertyNames(K.prototype).length);
console.log(typeof K.prototype.add, K.prototype.constructor === K);
const m = k.add;
const k2 = { items: ['other'] };
console.log(m.call(k2, 'x').items);
