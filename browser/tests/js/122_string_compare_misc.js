console.log(Math.sign('a'.localeCompare('b')), Math.sign('b'.localeCompare('a')), 'a'.localeCompare('a'));
console.log(['banana', 'apple', 'cherry'].sort((a, b) => a.localeCompare(b)));
console.log(String.fromCharCode(72, 101, 108, 108, 111), String.fromCharCode(0xd83d, 0xde00));
console.log(String.fromCodePoint(0x1f600, 65), String.fromCharCode(65 + 65536));
console.log('abc' < 'abd', 'Z' < 'a', 'é' > 'z');
console.log('a-b-c'.split('-').reverse().join('-'), [...'stressed'].reverse().join(''));
console.log('abc'.valueOf(), typeof new String('x').valueOf(), new String('xy').length);
