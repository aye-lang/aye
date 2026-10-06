console.log(/(?:ab)+/.exec('ababab x')[0], /a{2,3}/.exec('aaaa')[0], /a{2,}?/.exec('aaaa')[0]);
console.log(/<.+>/.exec('<a><b>')[0], /<.+?>/.exec('<a><b>')[0], /x*/.exec('abc')[0].length);
console.log(/cat|dog|bird/.exec('my bird')[0], /(a|ab)(c|bcd)(d*)/.exec('abcd').slice(0, 4));
console.log(/(\w)\1/.exec('hello')[0], /(?<year>\d{4})-(?<month>\d\d)/.exec('2023-07').groups.year);
console.log(/\d+(?=px)/.exec('width: 100px')[0], /\d+(?!px)/.exec('100px 200em')[0]);
console.log(/[^aeiou\s]+/g.source, /HELLO/i.test('hello'), /^b/m.test('a\nb'));
console.log(/[\w.-]+/.exec('  x.y-z_1!')[0], /A/.test('A'), /[à-ü]/.test('é'), /\./.test('a'));
