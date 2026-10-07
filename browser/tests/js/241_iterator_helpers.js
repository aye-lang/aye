// Iterator helpers (mistral.ai's module code calls
// `new Map(...).entries().forEach(...)`).
const m = new Map([['a', 1], ['b', 2], ['c', 3]]);
m.entries().forEach(([k, v], i) => console.log('forEach', k, v, i));
console.log('map', [...m.values().map((v, i) => v * 10 + i)].join());
console.log('filter', m.keys().filter(k => k !== 'b').toArray().join());
console.log('take drop', [1, 2, 3, 4, 5].values().drop(1).take(3).toArray().join());
console.log('flatMap', [1, 2].values().flatMap(x => [x, x * 100]).toArray().join());
console.log('reduce', [1, 2, 3].values().reduce((a, b) => a + b), [].values().reduce((a, b) => a + b, 'init'));
console.log('some every find', [1, 2, 3].values().some(x => x > 2), [1, 2, 3].values().every(x => x > 2), [1, 2, 3].values().find(x => x > 1));
function* naturals() { let n = 0; try { for (;;) yield n++; } finally { console.log('closed at', n); } }
console.log('lazy', naturals().map(x => x * x).filter(x => x % 2).take(3).toArray().join());
const h = naturals().map(x => x);
console.log(Object.prototype.toString.call(h), h.next().value, h.return().done, h.next().done);
console.log('found', naturals().find(x => x === 4));
try { [].values().reduce((a, b) => a); } catch (e) { console.log('empty reduce', e.name); }
try { [].values().take(-1); } catch (e) { console.log('take', e.name); }
try { [].values().map(1); } catch (e) { console.log('map', e.name); }
const plain = { i: 0, next() { return this.i < 2 ? { value: this.i++, done: false } : { done: true }; } };
const wrapped = Iterator.from(plain);
console.log('from', wrapped instanceof Iterator, wrapped.toArray().join(), Iterator.from([7, 8].values()).toArray().join());
console.log('string', Iterator.from('hi').toArray().join());
console.log('proto', Object.getPrototypeOf(Object.getPrototypeOf(h)) === Iterator.prototype, typeof Iterator.prototype.map);
