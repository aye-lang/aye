const log = [];
try {
  log.push('try');
  throw new Error('x');
} catch (e) {
  log.push('catch ' + e.message);
} finally {
  log.push('finally');
}
console.log(log.join(' > '));
function f() {
  try { console.log('f try'); return 'from try'; }
  finally { console.log('f finally'); }
}
console.log(f());
function g() {
  try { throw 1; }
  catch (e) { console.log('g catch', e); return 'from catch'; }
  finally { console.log('g finally'); }
}
console.log(g());
for (let i = 0; i < 3; i++) {
  try { if (i === 1) continue; console.log('body', i); }
  finally { console.log('fin', i); }
}
