function test(mode) {
  const log = [];
  try {
    log.push('t');
    if (mode === 'throw') throw new Error('boom');
    if (mode === 'return') return log.concat('ret').join('');
  } catch (e) {
    log.push('c');
  } finally {
    log.push('f');
  }
  log.push('after');
  return log.join('');
}
console.log(test('normal'), test('throw'), test('return'));
let counter = 0;
function withCleanup() { try { counter++; return counter; } finally { counter += 10; } }
console.log(withCleanup(), counter);
const out = [];
for (let i = 0; i < 4; i++) {
  try { if (i % 2) throw i; out.push('ok' + i); } catch (x) { out.push('err' + x); continue; } finally { out.push('f' + i); }
  out.push('end' + i);
}
console.log(out.join(' '));
