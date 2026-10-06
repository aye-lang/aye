function show(a, b) { return this.x + ':' + a + ':' + b; }
const bound = show.bind({ x: 'X' });
console.log(bound(1, 2));
const bound1 = show.bind({ x: 'Y' }, 'first');
console.log(bound1('second'), bound1('s', 'ignored'));
const rebound = bound.bind({ x: 'Z' });
console.log(rebound(3, 4));
console.log(bound.call({ x: 'W' }, 5, 6));
const obj = { v: 42, get() { return this.v; } };
const unbound = obj.get;
const fixed = unbound.bind(obj);
console.log(fixed());
const add = (a, b) => a + b;
console.log(add.bind(null, 10)(5));
console.log(typeof bound, bound.name, show.bind(null).length, show.bind(null, 1).length);
const btn = { label: 'ok', handlers: [] };
btn.handlers.push(function () { return this.label; }.bind(btn));
console.log(btn.handlers[0]());
