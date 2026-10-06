const s = 'Hello';
console.log(s.charAt(0), s.charAt(4), s.charAt(10) === '', s.charAt(-1) === '');
console.log(s.charCodeAt(0), s.charCodeAt(1), s.charCodeAt(99));
console.log(s[0], s[4], s[5], s['1']);
console.log(s.codePointAt(0), s.codePointAt(10));
console.log(s.at(0), s.at(-1), s.at(-5), s.at(5), s.at(-6));
console.log('😀'.codePointAt(0), '😀'.codePointAt(1), '😀'.charCodeAt(0), '😀'.charCodeAt(1));
console.log('😀'.at(0).length, '😀'.charAt(0).charCodeAt(0));
console.log(s.length, ''.length, 'a\nb'.length);
