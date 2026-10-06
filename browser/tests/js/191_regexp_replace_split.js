console.log('John Smith'.replace(/(\w+)\s(\w+)/, '$2, $1'));
console.log('aaa bbb'.replace(/b/g, 'X'), 'aaa'.replace(/a/, '[$&]'), 'x-y'.replace(/-/, "$'$`"));
console.log('3 apples and 5 pears'.replace(/\d+/g, (n) => n * 2));
console.log('camelCaseString'.replace(/([A-Z])/g, (m, c, off) => '_' + c.toLowerCase() + off));
console.log('a, b;c  d'.split(/[,;\s]+/), 'a1b22c'.split(/\d+/), 'abc'.split(/(b)/));
console.log('one  two'.split(/\s/), ''.split(/,/), 'abc'.split(''), 'a,b,c'.split(/,/, 2));
console.log('Hello'.search(/l/), 'Hello'.search(/z/), 'x'.replaceAll(/x/g, 'yy'));
