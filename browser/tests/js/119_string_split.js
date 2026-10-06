console.log('a,b,c'.split(','), 'a,b,c'.split(',', 2), 'a,b,c'.split(',', 0));
console.log('abc'.split(''), 'abc'.split(), ''.split(','), ''.split(''));
console.log('a,,b,'.split(','), ',a'.split(','));
console.log('one  two three'.split(' '), 'one two'.split('one'));
console.log('a1b2c3'.split(/\d/), 'a1b2c3'.split(/(\d)/), 'x'.split('x'));
console.log('2020-01-02'.split('-').map(Number));
console.log('héllo'.split(''), 'a😀b'.split('').length);
console.log('a, b ,c'.split(/\s*,\s*/));
