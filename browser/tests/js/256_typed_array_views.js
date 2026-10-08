// Typed array views share one ArrayBuffer: offsets, unsigned reads,
// float views over the same bytes, DataView both endians, subarray
// shares and slice copies, and Lit's template digest (run with
// ANYKNOWN_JS_GC=stress).
function show(a) { return a.constructor.name + "[" + Array.from(a).join(",") + "]"; }

// views over one buffer
var buf = new ArrayBuffer(8);
var u8 = new Uint8Array(buf);
var u32 = new Uint32Array(buf);
u32[0] = 0x99887766;
u32[1] = 0x01020304;
console.log(show(u8), u8.length, u32.length, buf.byteLength);
console.log(u32[0], u32[0] === 2575857510, u32.buffer === buf, u8.buffer === u32.buffer);
u8[7] = 0xff;
console.log(u32[1], new Int32Array(buf)[1], new Int8Array(buf)[7]);

// the Uint32 high bit, from an unconverted assignment
var h = new Uint32Array(2);
h[0] = -1726504068;
h[1] = Math.imul(0x9e3779b1, 31) ^ 0xdeadbeef;
console.log(h[0], h[1], h[0] > 0, new Uint8Array(h.buffer).length);
var i16 = new Int16Array(1), u16 = new Uint16Array(1);
i16[0] = 40000; u16[0] = -1;
console.log(i16[0], u16[0]);
var c = new Uint8ClampedArray(3);
c[0] = 300; c[1] = -5; c[2] = 1.5;
console.log(show(c));

// offsets and lengths
var big = new ArrayBuffer(16);
var mid = new Uint16Array(big, 4, 3);
mid[0] = 0xabcd; mid[2] = 513;
console.log(mid.byteOffset, mid.byteLength, mid.length, show(new Uint8Array(big)));
var tail = new Uint8Array(big, 10);
console.log(tail.length, tail.byteOffset, tail[0], tail[1]);

// float views over the same bytes
var fb = new ArrayBuffer(8);
var f64 = new Float64Array(fb);
f64[0] = 1.5;
console.log(show(new Uint8Array(fb)), show(new Uint32Array(fb)));
var f32 = new Float32Array(fb);
f32[0] = 0.1; f32[1] = -2.25;
console.log(f32[0], f32[1], show(new Uint8Array(fb)), f64[0]);

// DataView, both endians
var dv = new DataView(new ArrayBuffer(16), 2);
dv.setUint32(0, 0xdeadbeef);
dv.setUint32(4, 0xdeadbeef, true);
dv.setInt16(8, -2);
dv.setFloat32(10, 3.5, true);
console.log(dv.byteOffset, dv.byteLength, show(new Uint8Array(dv.buffer)));
console.log(dv.getUint32(0), dv.getUint32(0, true), dv.getInt32(4, true), dv.getUint16(8), dv.getInt16(8), dv.getFloat32(10, true));
dv.setFloat64(0, -0.5);
console.log(dv.getFloat64(0), dv.getFloat64(0, true), dv.getUint8(0), dv.getInt8(0));

// subarray shares, slice copies
var base = new Uint8Array([1, 2, 3, 4, 5, 6]);
var sub = base.subarray(2, 5), sl = base.slice(2, 5);
sub[0] = 30; sl[1] = 40;
console.log(show(base), show(sub), show(sl), sub.byteOffset, sub.buffer === base.buffer, sl.buffer === base.buffer);
base.set([7, 8], 3);
console.log(show(sub), show(base.subarray(-2)));

// a typed array from another copies, ArrayBuffer.slice copies
var copy = new Uint8Array(base);
copy[0] = 99;
var bs = base.buffer.slice(1, 3);
console.log(base[0], copy[0], bs.byteLength, show(new Uint8Array(bs)));

// out of range and non-index keys
var r = new Uint8Array(2);
r[5] = 1; r[-1] = 2;
console.log(r[5], r.length, Object.keys(r).join(","), 1 in r, 2 in r, JSON.stringify(r));
console.log(ArrayBuffer.isView(r), ArrayBuffer.isView(dv), ArrayBuffer.isView(buf), Array.isArray(r));
console.log(r instanceof Uint8Array, r.constructor.BYTES_PER_ELEMENT);

// methods
var m = new Int8Array([5, -3, 2, 200]);
console.log(m.join("|"), m.indexOf(2), m.includes(-56), m.map(x => x * 2).join(","), show(m.filter(x => x > 0)));
m.sort();
console.log(show(m), [...m].length, m.reduce((a, b) => a + b, 0));
m.fill(1, 1, 3);
console.log(show(m), m.at(-1), Array.prototype.slice.call(m, 1).join(","));

// Lit's template digest
var strings = ["<div>", "</div>"];
var hashes = new Uint32Array(2).fill(5381);
for (var s of strings) for (var i = 0; i < s.length; i++) hashes[i % 2] = (hashes[i % 2] * 33) ^ s.charCodeAt(i);
var digest = String.fromCharCode(...new Uint8Array(hashes.buffer)).length + ":" + Array.from(new Uint8Array(hashes.buffer)).join(".");
console.log(digest, hashes[0], hashes[1]);
