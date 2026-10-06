function Stack() { this.items = []; }
Stack.prototype.push = function (x) { this.items.push(x); return this; };
Stack.prototype.pop = function () { return this.items.pop(); };
Stack.prototype.peek = function () { return this.items[this.items.length - 1]; };
Object.defineProperty(Stack.prototype, 'size', { get() { return this.items.length; } });
const s1 = new Stack(), s2 = new Stack();
s1.push(1).push(2).push(3);
s2.push('a');
console.log(s1.pop(), s1.peek(), s1.size, s2.size, s1.push === s2.push);
Stack.prototype.clear = function () { this.items.length = 0; };
s1.clear();
console.log(s1.size);
console.log(Object.keys(s1), 'size' in s1);
