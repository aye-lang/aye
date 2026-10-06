console.log(parseInt('42'), parseInt('  42  '), parseInt('42px'), parseInt('px42'), parseInt(''));
console.log(parseInt('0x1F'), parseInt('0x1F', 16), parseInt('1F', 16), parseInt('0x'), parseInt('-0x10'));
console.log(parseInt('101', 2), parseInt('777', 8), parseInt('z', 36), parseInt('Zz', 36), parseInt('2', 2));
console.log(parseInt('12', 0), parseInt('12', 1), parseInt('12', 37), parseInt('3.99'), parseInt('-3.99'));
console.log(parseInt('1e3'), parseInt(1e21), parseInt(0.0000005), parseInt(null), parseInt('Infinity'));
console.log(parseInt('  -17abc'), parseInt('+5'), parseInt('0012'), parseInt('9007199254740993'));
console.log(['1', '2', '3'].map(parseInt), Number.parseInt === parseInt);
