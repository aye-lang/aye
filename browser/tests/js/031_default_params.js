function greet(name = 'world', punct = '!') { return 'hello ' + name + punct; }
console.log(greet(), greet('bob'), greet(undefined, '?'), greet(null));
function area(w, h = w) { return w * h; }
console.log(area(3), area(3, 4));
function f(a, b = a * 2, c = a + b) { return [a, b, c]; }
console.log(f(1), f(1, 1), f(1, undefined, 0));
let calls = 0;
function g(x = ++calls) { return x; }
console.log(g(), g(), g(10), calls);
function h(arr = []) { arr.push(1); return arr.length; }
console.log(h(), h(), h([5, 5]));
const arrow = (x, y = x + 1) => x * y;
console.log(arrow(3));
console.log(greet.length, area.length, f.length);
