console.log('tab:\tend');
console.log('new\nline');
console.log('quote: \' and \" and \\');
console.log('é', '\u{1F600}', '\x41', 'café' === 'café');
console.log('é'.length, 'é'.length, 'é' === 'é', 'e\u0301'.length, '\u00e9'.length);
console.log('\0'.length, '\0'.charCodeAt(0), 'a\
b');
console.log(JSON.stringify('tab\there\nnl "q" \\'));
console.log('\u{41}\u{42}', '\v\f'.length, '\r'.charCodeAt(0));
