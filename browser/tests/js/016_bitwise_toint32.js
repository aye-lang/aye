console.log(2147483647 + 1 | 0, 2147483648 | 0, 4294967295 | 0, 4294967296 | 0);
console.log(4294967297 | 0, -2147483649 | 0, 1e10 | 0, -1e10 | 0);
console.log(1e21 | 0, 2 ** 53 | 0, (2 ** 53 + 2) | 0, 2 ** 32 + 5 >>> 0);
console.log(3000000000 >>> 0, 3000000000 >> 0, -3000000000 >>> 0);
console.log(123456789012 & 0xffffffff, 123456789012 >>> 0);
console.log(0x80000000 >> 31, 0x80000000 >>> 31, 0xffffffff >> 16, 0xffffffff >>> 16);
console.log(1.9999e9 | 0, -0.5 | 0, Object.is(-0 | 0, 0));
let h = 0;
for (const ch of 'hello world') h = (h * 31 + ch.charCodeAt(0)) | 0;
console.log(h);
let h2 = 2166136261;
for (const ch of 'abc') { h2 ^= ch.charCodeAt(0); h2 = Math.imul ? (h2 * 16777619) >>> 0 : 0; }
console.log(h2 > 0);
