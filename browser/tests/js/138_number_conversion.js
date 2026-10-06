console.log(Number(''), Number(' 12 '), Number('0x1f'), Number('1e3'), Number('abc'));
console.log(Number('12px'), Number('-0x10'), Number('0b101'), Number('0o17'), Number('.5'), Number('5.'));
console.log(Number('\n\t 7 \n'), Number('1,000'), Number('1_000'), Number('+12'), Number('--1'));
console.log(Number('Infinity'), Number('-Infinity'), Number('infinity'), Number(null), Number(undefined));
console.log(Number(true), Number(false), Number([]), Number([7]), Number([1, 2]), Number({}));
console.log(Number(new Date(Date.UTC(2000, 0, 1))), Number('1e-7'), Number('00012'));
