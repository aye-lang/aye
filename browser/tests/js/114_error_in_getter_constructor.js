const o = { get bad() { throw new Error('getter failed'); } };
try { o.bad; } catch (e) { console.log(e.message); }
class Strict {
  constructor(v) { if (v < 0) throw new RangeError('negative: ' + v); this.v = v; }
}
try { new Strict(-1); } catch (e) { console.log(e.name, e.message); }
console.log(new Strict(3).v);
const s = { set v(x) { throw new TypeError('readonly'); } };
try { s.v = 1; } catch (e) { console.log(e.message); }
try { `${{ toString() { throw new Error('in toString'); } }}`; } catch (e) { console.log(e.message); }
try { [1, 2].map(function () { return this.x.y; }, {}); } catch (e) { console.log(e.name); }
function deep(n) { if (n === 0) throw new Error('bottom'); return deep(n - 1); }
try { deep(100); } catch (e) { console.log(e.message); }
