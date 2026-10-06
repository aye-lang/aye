// The Array methods that change their array work on any array-like
// object (jQuery: `push.apply(this, nodes)` on a jQuery object).
const push = Array.prototype.push;
const jq = { length: 0, jquery: '3' };
push.apply(jq, ['a', 'b']);
console.log(jq.length, jq[0], jq[1], jq.jquery);
console.log(push.call(jq, 'c'), jq[2], Object.keys(jq).join(','));
const q = { length: 3, 0: 'x', 1: 'y', 2: 'z' };
console.log(Array.prototype.pop.call(q), q.length, 2 in q);
console.log(Array.prototype.shift.call(q), q.length, q[0], 1 in q);
console.log(Array.prototype.unshift.call(q, 'u', 'v'), q[0], q[1], q[2], q.length);
const r = { length: 4, 0: 3, 1: 1, 2: 4, 3: 2 };
console.log(Array.prototype.sort.call(r) === r, r[0], r[1], r[2], r[3]);
console.log(Array.prototype.reverse.call(r) === r, r[0], r[3]);
console.log(Array.prototype.splice.call(r, 1, 2, 'm'), r.length, r[0], r[1], r[2], 3 in r);
const f = { length: 3 };
console.log(Array.prototype.fill.call(f, 0) === f, f[0], f[2], f.length);
function args() {
  push.call(arguments, 'more');
  return [arguments.length, arguments[2]];
}
console.log(args(1, 2));
console.log(push.call({}, 'one').valueOf(), Array.prototype.pop.call({}));
