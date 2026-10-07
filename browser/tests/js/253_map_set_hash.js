// Map and Set keys by SameValueZero through the hash index: NaN, +0/-0,
// mixed types, many keys in insertion order, changes during iteration,
// and object keys kept across collections (run with ANYKNOWN_JS_GC=stress).

// NaN and +0/-0
var m = new Map();
m.set(NaN, "nan").set(0, "zero");
console.log(m.get(NaN), m.get(0 / 0), m.get(-0), m.has(-0), m.size);
m.set(-0, "minus");
console.log(m.size, m.get(0), Object.is([...m.keys()][1], 0));
var s = new Set([NaN, NaN, 0, -0, +0]);
console.log(s.size, s.has(NaN), s.has(-0), Object.is([...s][1], 0));
console.log(m.delete(-0), m.has(0), m.size);

// mixed types: no coercion between them
var mx = new Map();
var o1 = {}, o2 = {}, sym = Symbol("k"), big = 10n;
var keys = [1, "1", true, "true", null, undefined, "null", o1, o2, sym, big, 1.5, -1.5, Infinity, -Infinity, "", 0];
keys.forEach((k, i) => mx.set(k, i));
console.log(mx.size, keys.every((k, i) => mx.get(k) === i));
console.log(mx.get(10n), mx.get(1n), mx.get({}), mx.get(Symbol("k")), mx.get("1"), mx.get(1));
var built = "a" + "b".repeat(3);
mx.set("abbb", "flat");
console.log(mx.get(built), mx.get("ab" + "bb"));
console.log(new Set([1, "1", 1, "1", 1.0, o1, o1]).size);

// 10k keys: insert, delete, re-add go to the end
var big1 = new Map();
for (var i = 0; i < 10000; i++) big1.set("k" + i, i);
for (var i = 0; i < 10000; i += 2) big1.delete("k" + i);
console.log(big1.size, big1.get("k1"), big1.get("k2"), big1.has("k9999"));
for (var i = 0; i < 10000; i += 4) big1.set("k" + i, -i);
big1.set("k1", "again");
var ks = [...big1.keys()];
console.log(big1.size, ks[0], ks[1], ks[4999], ks[5000], ks[5001], ks[ks.length - 1]);
console.log(big1.get("k1"), big1.get("k4"), big1.get("k6"));
var nums = new Set();
for (var i = 0; i < 10000; i++) nums.add(i * 0.5);
for (var i = 0; i < 10000; i++) if (i % 3) nums.delete(i * 0.5);
nums.add(1); nums.add(0.5);
var vs = [...nums];
console.log(nums.size, vs.slice(0, 4).join(), vs.slice(-3).join(), nums.has(1.5), nums.has(4999.5));
big1.clear();
console.log(big1.size, big1.get("k1"), [...big1].length);
big1.set("x", 1);
console.log([...big1.entries()].join());

// deletion and additions during forEach
var fm = new Map([["a", 1], ["b", 2], ["c", 3], ["d", 4]]);
var seen = [];
fm.forEach((v, k) => {
  seen.push(k);
  if (k === "a") { fm.delete("b"); fm.set("e", 5); }
  if (k === "c") { fm.delete("a"); fm.set("a", 6); }
});
console.log(seen.join(), [...fm.keys()].join());
// many deletions during forEach (compaction must not skip entries)
var dm = new Map();
for (var i = 0; i < 100; i++) dm.set(i, i);
var visited = 0;
dm.forEach((v, k) => { visited++; if (k < 50) for (var j = 0; j < 100; j++) if (j !== k + 1) dm.delete(j); });
console.log(visited, dm.size);
var ds = new Set();
for (var i = 0; i < 100; i++) ds.add(i);
var sv = [];
ds.forEach((k) => { sv.push(k); if (k % 10 === 0) for (var j = k + 1; j < k + 8; j++) ds.delete(j); });
console.log(sv.length, sv.slice(0, 6).join());
// clear during forEach, then add
var cm = new Map([[1, 1], [2, 2], [3, 3]]);
var cseen = [];
cm.forEach((v, k) => { cseen.push(k); if (k === 1) { cm.clear(); cm.set(9, 9); cm.set(10, 10); } });
console.log(cseen.join(), [...cm.keys()].join());
var cs = new Set([1, 2, 3]);
var csn = [];
cs.forEach((k) => { csn.push(k); if (k === 2) { cs.clear(); cs.add(7); } });
console.log(csn.join(), [...cs].join());

// for-of over a Map or Set walks a snapshot (iterToArray); entries
// changed during the loop follow in a later step.
var it2 = new Set([1, 2, 3]).values();
console.log(it2.next().value, [...it2].join());

// object keys survive collections
var om = new Map(), os = new Set(), objs = [];
for (var i = 0; i < 2000; i++) {
  var ob = { i: i };
  objs.push(ob);
  om.set(ob, i);
  os.add(ob);
  var junk = [{}, {}, "x" + i];
}
var ok = 0;
for (var i = 0; i < 2000; i++) if (om.get(objs[i]) === i && os.has(objs[i])) ok++;
console.log(ok, om.has({ i: 0 }), os.size);
for (var i = 0; i < 2000; i += 2) { om.delete(objs[i]); os.delete(objs[i]); }
objs = objs.filter((o, i) => i % 2);
for (var i = 0; i < 5000; i++) var junk2 = { a: [i] };
var ok2 = 0;
for (var o of objs) if (om.get(o) === o.i && os.has(o)) ok2++;
console.log(ok2, om.size, os.size);

// WeakMap and WeakSet
var wm = new WeakMap(), ws = new WeakSet(), wk = {};
wm.set(wk, 1); ws.add(wk);
console.log(wm.get(wk), ws.has(wk), wm.has({}), wm.delete(wk), wm.has(wk));
