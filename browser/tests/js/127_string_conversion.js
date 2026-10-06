console.log(String(1e21), String(0.000001), String(1e-7), String(-0), (123).toString());
console.log(String(Symbol_free_value()), [1, 'a', null, undefined].join('/'));
function Symbol_free_value() { return [1, [2, [3]]]; }
console.log(`${[]}|${[[]]}|${[null]}|${[undefined, 1]}`);
console.log((255).toString(16), (255).toString(2), (-255).toString(36), (0.5).toString(2));
console.log(parseInt('ff', 16), Number('0b11'), +'0o7');
console.log('abc'.toString(), String('x'.concat(1)));
console.log(String(new Date(Date.UTC(2020, 0, 1)).getTime()));
