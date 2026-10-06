// The numeric recipe may retry only before any observable operation.
function arithmetic(a, b, c) { return a + b * c; }
console.log(arithmetic(2, 3, 4), arithmetic("x", 3, 4), arithmetic(2, undefined, 4));
function member(o) { return o.x + o.y * 2; }
var reads = 0;
var o = {x:3, get y() { reads++; return 7; }};
console.log(member(o), reads);
var proxy = new Proxy({x:4, y:6}, {get:function(t, k) { reads++; return t[k]; }});
console.log(member(proxy), reads);
function dense(a, i) { return a[i] * 2 + 1; }
console.log(dense([2, 5], 1), dense([2, 5], -1), dense([2, 5], 0.5));
Array.prototype[1] = 9;
console.log(dense([1, , 3], 1));
delete Array.prototype[1];
function make(n) {
  return function(delta) { return n + delta * 2; };
}
var f = make(5), g = make(20);
console.log(f(2), g(2), f(3));
function identity(x) { return x; }
console.log(identity(undefined), identity(null), identity(true), identity("str"), identity({x:1}).x);
function construct(x) { this.x = x; }
construct.prototype.read = function() { return this.x; };
console.log(new construct(17).read());
function arrow(n) { return x => n + x; }
console.log(arrow(10)(3));
function varUpdate(x) { var before = x++; return [before, x]; }
console.log(varUpdate(-0).map(x => Object.is(x, -0)).join(","));
try { const x = 1; x++; } catch(e) { console.log(e.name); }
function dead() { if (true) { return x + 1; let x = 2; } }
try { dead(); } catch(e) { console.log(e.name); }
var literals = [];
for (var i = 0; i < 100; i++) literals.push({x:i, nested:{name:"n" + i}, f:make(i)});
console.log(literals[0].f(2), literals[99].nested.name, literals[99].x);
console.log(({x:1, x:2}).x, ({["x"]:3}).x, ({...{x:4}}).x);
