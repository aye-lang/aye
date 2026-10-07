function error(f) { try { f(); console.log('NO ERROR'); } catch(e) { console.log(e.name); } }
for (const v of [1.5, NaN, Infinity]) error(() => BigInt(v));
for (const v of [undefined, null, Symbol('x')]) error(() => BigInt(v));
for (const v of ['1.5', '1n', '1_000', '-0xff', '0x', 'abc']) error(() => BigInt(v));
error(() => new BigInt(1)); error(() => +1n);
for (const f of [() => 1n+1, () => 1-1n, () => 1n*2, () => 1n/2, () => 1n%2, () => 1n**2, () => 1n&2, () => 1n|2, () => 1n^2, () => 1n<<2, () => 1n>>2, () => 1n>>>1n]) error(f);
error(() => 1n/0n); error(() => 1n%0n); error(() => 2n**-1n);
error(() => JSON.stringify(1n)); error(() => JSON.stringify({x: 1n})); error(() => JSON.stringify(Object(1n)));
error(() => Math.abs(1n)); error(() => Math.max(1n, 2n)); error(() => Math.sqrt(1n));
error(() => (1n).toString(1)); error(() => (1n).toString(37));
error(() => BigInt.prototype.valueOf.call(1)); error(() => BigInt.prototype.toString.call({}));
error(() => BigInt.asIntN(-1, 1n)); error(() => BigInt.asUintN(8, 1)); error(() => BigInt.asIntN(1n, 1n));
console.log(JSON.stringify(1n, (k,v) => String(v)));
BigInt.prototype.toJSON = function() { return this.toString(); }; console.log(JSON.stringify(1n), JSON.stringify(Object(2n)));
