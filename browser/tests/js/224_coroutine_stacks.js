// Generators and async functions on coroutine stacks (std.coro): many
// alive at once, nested resumes, generators dropped while suspended,
// deep recursion inside a generator, and long async chains.
function* count(n) { for (let i = 0; i < n; i++) yield i; }
let s = 0;
for (const v of count(20000)) s += v;
console.log('count', s);

const live = [];
for (let i = 0; i < 200; i++) { const g = count(5); g.next(); live.push(g); }
let t = 0;
for (const g of live) for (const v of g) t += v;
console.log('live', t);

for (let i = 0; i < 300; i++) { const g = count(10); g.next(); g.next(); }
console.log('dropped');

function* outer() {
  const inner = count(3);
  yield inner.next().value;
  for (const v of inner) yield v * 10;
  yield* count(2);
}
console.log('nested', [...outer()].join(','));

function* tree(d) { if (d === 0) { yield 1; return; } yield* tree(d - 1); yield* tree(d - 1); }
let leaves = 0;
for (const v of tree(8)) leaves += v;
console.log('tree', leaves);

function depth(n) { return n === 0 ? 0 : 1 + depth(n - 1); }
function* deep() { yield depth(2000); try { yield depth(1e6); } catch (e) { yield e instanceof RangeError; } }
console.log('deep', [...deep()].join(','));

function* closer() { try { yield 1; yield 2; } finally { console.log('closed'); } }
for (const v of closer()) { if (v === 1) break; }
const th = closer(); th.next();
try { th.throw(new Error('x')); } catch (e) { console.log('throw', e.message, th.next().done); }

async function leaf(i) { await null; return i; }
async function mid(i) { return (await leaf(i)) + (await leaf(i + 1)); }
(async () => {
  let sum = 0;
  for (let i = 0; i < 2000; i++) sum += await mid(i);
  console.log('async', sum);
  const all = await Promise.all(Array.from({ length: 100 }, (_, i) => mid(i)));
  console.log('all', all.reduce((a, b) => a + b, 0));
  async function* agen() { for (let i = 0; i < 50; i++) { await null; yield i; } }
  let n = 0;
  for await (const v of agen()) n += v;
  console.log('for await', n);
})();
