console.log(String(123), String(-1.5), String(true), String(null), String(undefined));
console.log(String([1, [2, 3]]), String({}), String([]), String([null, undefined]));
console.log(Number('42'), Number(true), Number(null), Number(undefined), Number([]), Number(['7']));
console.log(Boolean(0), Boolean(''), Boolean('0'), Boolean([]), Boolean({}), Boolean(NaN), Boolean(-0));
console.log(1 + 2 + '3', '1' + 2 + 3, 1 + +'2', '5' - - '2');
console.log(`${1}${null}${undefined}${[1, 2]}${true}`);
const o = { toString() { return '7'; } };
console.log(o * 2, o + 1, +o);
console.log([10, 1, 3].toString(), [1, 2].join() + 3);
