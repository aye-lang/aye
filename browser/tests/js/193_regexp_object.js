const r = new RegExp('a+b', 'g');
console.log(r.source, r.flags, r.global, r.ignoreCase, r.multiline, String(r));
const r2 = new RegExp(r);
console.log(r2.source, r2.flags, r2 instanceof RegExp, typeof r2);
const esc = 'a.b*c'.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
console.log(esc, new RegExp(esc).test('a.b*c'), new RegExp(esc).test('axbbc'));
try { new RegExp('('); } catch (e) { console.log(e.name); }
console.log(/x/y.sticky, 'aab'.match(/a/g).length);
