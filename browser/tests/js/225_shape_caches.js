// Warm a shared-layout site, then change descriptors and prototype links.
function read(o) { return o.x; }
function write(o, x) { o.x = x; return o.x; }
function call(o) { return o.method(); }
var a = {x:1, y:2}, b = {x:3, y:4};
console.log(read(a), read(b), write(a, 5), read(b));
Object.defineProperty(a, "x", {get:function() { return this.y + 10; }, set:function(x) { this.y = x; }, configurable:true});
console.log(read(a), write(a, 7), read(b));
Object.defineProperty(a, "x", {value:9, writable:false});
console.log(write(a, 99));
delete b.x;
b.z = 8;
b.x = 12;
console.log(read(b), Object.keys(b).join(","));
var p = {method:function() { return this.x + 1; }};
var q = {method:function() { return this.x + 20; }};
var c = Object.create(p); c.x = 2;
console.log(call(c));
Object.setPrototypeOf(c, q);
console.log(call(c));
q.method = function() { return 30; };
console.log(call(c));
c.method = function() { return 40; };
console.log(call(c));
delete c.method;
console.log(call(c));
var frozen = {x:11, y:2};
Object.freeze(frozen);
console.log(write(frozen, 17), read(frozen));
var big = {};
for (var i = 0; i < 50; i++) big["p" + i] = i;
big.x = 100;
console.log(read(big), Object.keys(big).length);
delete big.p4;
delete big.x;
big.x = 200;
console.log(read(big), big.p49, Object.keys(big).length);
var proxy = new Proxy({x:1}, {get:function(t, k) { return k === "x" ? 42 : t[k]; }});
console.log(read(proxy));
var list = [1, 2];
function length(o) { return o.length; }
console.log(length({length:17}), length(list), length("hello"));
console.log(read({y:1, x:2}), read({x:4, y:5}));
