function f() {}
console.log(Object.getOwnPropertyNames(f).includes("prototype"));
console.log(f.hasOwnProperty("prototype"), f.prototype.constructor === f, f.prototype === f.prototype);
function g() {}
Object.freeze(g);
var gp = g.prototype;
console.log(gp.constructor === g, g.prototype === gp, Object.isFrozen(g));
var gd = Object.getOwnPropertyDescriptor(g, "prototype");
console.log(gd.value === gp, gd.writable, gd.enumerable, gd.configurable);
function h() {}
Object.defineProperty(h, "prototype", {writable:false});
console.log(h.prototype.constructor === h, Object.getOwnPropertyDescriptor(h, "prototype").writable);
function replaced() {}
replaced.prototype = {x:17};
console.log(new replaced().x, new replaced() instanceof replaced);
function d() {}
var dp = Object.getOwnPropertyDescriptor(d, "prototype").value;
console.log(dp === d.prototype, dp.constructor === d);
var functions = [];
for (var i = 0; i < 2000; i++) functions.push(function() { return i; });
console.log(functions[10].prototype.constructor === functions[10], functions[1999].prototype !== functions[10].prototype);
var arrow = x => x;
console.log(arrow.hasOwnProperty("prototype"));
function shadow() {}
var inherited = Object.create(shadow);
console.log(inherited.prototype.constructor === shadow);
function invalid() {}
Object.freeze(invalid);
try { Object.defineProperty(invalid, "prototype", {value:{}}); } catch(e) { console.log(e.name); }
var method = ({m() {}}).m;
var reads = 0;
Object.defineProperty(Function.prototype, "prototype", {get:function() { reads++; return {}; }, configurable:true});
Object.defineProperty(method, "prototype", {value:{x:4}, configurable:true});
console.log(reads, method.prototype.x);
delete Function.prototype.prototype;
var getter = function() { reads++; return 99; };
Object.defineProperty(method, "prototype", {get:getter, configurable:true});
console.log(Object.getOwnPropertyDescriptor(method, "prototype").get === getter, reads);
console.log(method.prototype, reads);
