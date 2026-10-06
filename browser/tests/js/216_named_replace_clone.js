const date = /(?<y>\d{4})-(?<m>\d\d)(?<rest>x)?/g;
console.log('2024-05 and 1999-12'.replace(date, '$<m>/$<y>[$<rest>][$<none>]$<'));
console.log('2024-05'.replace(date, (all, y, m, rest, at, input, groups) => [all, y, m, rest, at, input, groups.y, groups.m].join(',')));
console.log('a-b'.replace(/(\w)-(\w)/, (all, a, b, at, input, groups) => typeof groups));
console.log('x'.replace('x', '$<y>'), 'ab'.replace(/(?<first>a)/, '[$<first>$<first'));
const src = { n: 1, s: 'two', nested: { list: [1, { deep: true }], when: new Date(0) }, re: /a+/gi, map: new Map([['k', { v: 1 }]]), set: new Set([1, 2]), err: new RangeError('bad'), boxed: Object(5), u: undefined, holes: [1, , 3] };
src.self = src;
src.shared = [src.nested, src.nested];
const c = structuredClone(src);
console.log(c !== src, c.self === c, c.nested !== src.nested, c.shared[0] === c.shared[1], c.shared[0] === c.nested);
console.log(c.n, c.s, c.nested.list, c.nested.when instanceof Date, c.nested.when.getTime());
console.log(c.re.source, c.re.flags, c.map.get('k'), c.map.get('k') !== src.map.get('k'), [...c.set]);
console.log(c.err instanceof RangeError, c.err.message, typeof c.boxed, c.boxed + 1, 'u' in c, c.holes.length, 1 in c.holes);
class P { constructor() { this.x = 1; } get y() { return 2; } }
const pc = structuredClone(new P());
console.log(pc instanceof P, pc.x, pc.y, structuredClone(5), structuredClone(null), structuredClone('s'));
for (const bad of [() => 1, { f() {} }, Symbol('s')]) {
  try { structuredClone(bad); console.log('cloned'); } catch (e) { console.log('not cloned'); }
}
