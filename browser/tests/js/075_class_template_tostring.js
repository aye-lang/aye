class Money {
  constructor(amount, cur) { this.amount = amount; this.cur = cur; }
  toString() { return `${this.amount.toFixed(2)} ${this.cur}`; }
  valueOf() { return this.amount; }
}
const m = new Money(12.5, 'EUR');
console.log(`Total: ${m}`, String(m), m + 1, m > 10);
console.log([new Money(1, 'X'), new Money(2, 'Y')].join(' | '));
class NoToString { constructor() { this.v = 1; } }
console.log(`${new NoToString()}`);
class Tagged { get [Symbol_toStringTag_unused()]() { return 'x'; } }
function Symbol_toStringTag_unused() { return 'tag'; }
console.log(new Tagged().tag);
