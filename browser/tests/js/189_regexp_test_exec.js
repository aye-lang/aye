const re = /(\d{4})-(\d{2})-(\d{2})/;
const m = re.exec('Date: 2024-03-15 end');
console.log(m[0], m[1], m[2], m[3], m.index, m.input.length);
console.log(re.test('no date'), re.exec('x'), /^abc$/.test('abc'), /^abc$/.test('abcd'));
console.log(/colou?r/.test('color'), /colou?r/.test('colour'), /a.c/.test('a\nc'), /a.c/s.test('a\nc'));
console.log(/[A-Z][a-z]+/.exec('hello World')[0], /\bcat\b/.test('concat'), /\bcat\b/.test('a cat!'));
console.log(/^\s*$/.test('  \t '), /\S+@\S+\.\w+/.test('mail me@x.org now'));
