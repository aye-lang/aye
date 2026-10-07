// A `/` after the `)` of an `if`, `while` or `for` head starts a regex
// (minified code: `else if(o<128)/[!#$&-;=?-Z_a-z~]/.test(c)`); after
// other `)` it divides.
const s = 'abc';
if (s.length) /b/.test(s) && console.log('if regex');
while (false) /x/g;
for (let i = 0; i < 1; i++) /a/.test(s) && console.log('for regex');
if (s) /[!#$&-;=?-Z_a-z~]/.test('q') ? console.log('class regex') : 0;
const d = (6) / 2 / 1;
console.log('divide', d, (s.length)/3, (s.length) /3/ 1);
function f(a) { return a; }
console.log(f(8)/2, [4][0]/2);
