function classify(fn) {
  try { fn(); return 'no error'; } catch (e) {
    if (e instanceof TypeError) return 'TypeError';
    if (e instanceof RangeError) return 'RangeError';
    if (e instanceof ReferenceError) return 'ReferenceError';
    if (e instanceof SyntaxError) return 'SyntaxError';
    if (e instanceof Error) return 'Error';
    return 'non-error ' + typeof e;
  }
}
console.log(classify(() => null.x));
console.log(classify(() => new Array(-5)));
console.log(classify(() => nope));
console.log(classify(() => JSON.parse('{')));
console.log(classify(() => { throw new Error('e'); }));
console.log(classify(() => { throw 5; }));
console.log(classify(() => 1));
console.log(classify(() => { class X extends TypeError {} throw new X('x'); }));
