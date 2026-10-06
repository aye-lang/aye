// Identifiers may be written with \u escapes (Angular's `ɵprov` in
// YouTube's bundle: `H.id.ɵprov`); the name is the decoded one.
const o = { ɵprov: 'token', café: 1 };
console.log(o.ɵprov, o['ɵprov'], o.café, o.caf\u{e9});
var abc = 5;
console.log(abc, typeof \u{61}bc);
const obj = { bar: 2, get q() { return 'q'; } };
console.log(Object.keys(obj).join(','), obj.bar, obj.q);
function fn(x) { return x * 2; }
console.log(fn(21), fn.name);
class K { m() { return 'm'; } }
console.log(new K().m());
let err = '';
try { eval('var a\\u{110000}b'); } catch (e) { err = e.name; }
console.log(err);
