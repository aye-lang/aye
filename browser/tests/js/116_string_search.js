const s = 'the quick brown fox jumps over the lazy dog';
console.log(s.indexOf('the'), s.indexOf('the', 1), s.indexOf('cat'), s.indexOf(''), s.indexOf('', 100));
console.log(s.lastIndexOf('the'), s.lastIndexOf('the', 30), s.lastIndexOf('o'), s.lastIndexOf('zzz'));
console.log(s.includes('fox'), s.includes('Fox'), s.includes('fox', 20), s.includes(''));
console.log(s.startsWith('the'), s.startsWith('quick', 4), s.startsWith('x'));
console.log(s.endsWith('dog'), s.endsWith('lazy', 39), s.endsWith(''));
console.log(s.search(/o/), s.search(/z+/), s.search(/QQ/));
console.log('aaa'.indexOf('aa', 1), 'aaa'.lastIndexOf('aa'), 'abc'.indexOf('c', -5));
