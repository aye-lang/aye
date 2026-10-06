console.log(parseFloat('3.14'), parseFloat('  3.14abc'), parseFloat('.5'), parseFloat('-.5e2'), parseFloat('abc'));
console.log(parseFloat('1e3'), parseFloat('1e'), parseFloat('1.2.3'), parseFloat('Infinityx'), parseFloat('-Infinity'));
console.log(parseFloat('0x10'), parseFloat(''), parseFloat('  '), parseFloat('5.'), parseFloat('1_000'));
console.log(parseFloat('1e400'), parseFloat('-0'), parseFloat('1e-400'), parseFloat([' 2.5 ']));
console.log(parseFloat('0.1') + parseFloat('0.2'), Number.parseFloat === parseFloat);
