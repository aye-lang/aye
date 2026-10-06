let s = 'abc';
try { s[0] = 'z'; } catch (e) {}
console.log(s);
const t = s;
s += 'd';
console.log(s, t);
const up = s.toUpperCase();
console.log(s, up);
const big = 'x'.repeat(5);
console.log(big.replace('x', 'y'), big);
console.log(typeof s, s instanceof String, Object('s') instanceof String);
const p = 'prop';
try { p.custom = 1; } catch (e) {}
console.log(p.custom);
