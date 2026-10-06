console.log('abc'.at(-1), 'a-b_c d'.split(/[-_ ]/));
console.log('Hello'.startsWith(''), 'x'.endsWith('x', 1), 'x'.endsWith('x', 0));
console.log('%'.charCodeAt(0), 'z'.codePointAt(0) - 'a'.codePointAt(0));
console.log('abcdef'.match(/c(d)e/)[1], 'no'.match(/x/));
console.log('aBc'.split('').map((c) => c === c.toUpperCase() ? c.toLowerCase() : c.toUpperCase()).join(''));
console.log('tom,dick,harry'.split(',').map((s) => s.length));
console.log(' a b '.split(' '), 'a\tb\nc'.split(/\s/));
console.log('abc'.localeCompare('abd') < 0, 'b'.localeCompare('a') > 0);
