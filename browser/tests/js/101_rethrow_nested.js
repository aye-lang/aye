function parse(s) {
  try {
    if (s === '') throw new RangeError('empty');
    if (s === 'bad') throw new TypeError('bad input');
    return s.toUpperCase();
  } catch (e) {
    if (e instanceof RangeError) return 'default';
    throw e;
  }
}
console.log(parse('ok'), parse(''));
try { parse('bad'); } catch (e) { console.log('outer caught', e.name, e.message); }
try {
  try {
    throw new Error('inner');
  } finally {
    console.log('inner finally');
  }
} catch (e) {
  console.log('outer', e.message);
}
try {
  try { throw new Error('A'); }
  catch (e) { try { throw new Error(e.message + 'B'); } catch (e2) { throw new Error(e2.message + 'C'); } }
} catch (e) { console.log(e.message); }
