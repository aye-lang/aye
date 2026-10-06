console.log(5 & 3, 5 | 3, 5 ^ 3, ~5, ~-1, ~0);
console.log(1 << 4, 256 >> 4, -256 >> 4, -256 >>> 28);
console.log(-1 >>> 0, -1 >>> 1, 1 << 31, 1 << 32, 1 << 33);
console.log(0xff & 0x0f, 0xf0 | 0x0f, 0xff ^ 0xaa);
console.log(~~3.7, ~~-3.7, 3.7 | 0, -3.7 | 0);
console.log('7' & '3', null | 0, undefined | 0, NaN | 0, Infinity | 0);
console.log(true << 2, [] | 0, '0x10' | 0);
