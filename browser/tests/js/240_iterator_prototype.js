// %IteratorPrototype% sits between the built-in iterators (and
// generators) and Object.prototype, as the abstract Iterator's prototype.
// A polyfill that finds it there must not touch Object.prototype.
const IP = Object.getPrototypeOf(Object.getPrototypeOf([][Symbol.iterator]()));
console.log(IP === Iterator.prototype, IP !== Object.prototype, Object.getPrototypeOf(IP) === Object.prototype);
console.log(Object.getPrototypeOf(Object.getPrototypeOf(new Map().keys())) === IP,
  Object.getPrototypeOf(Object.getPrototypeOf(new Set().values())) === IP,
  Object.getPrototypeOf(Object.getPrototypeOf(''[Symbol.iterator]())) === IP);
const genProto = Object.getPrototypeOf(function* () {}.prototype);
console.log(Object.getPrototypeOf(genProto) === IP, IP[Symbol.iterator].call(5) === 5 || typeof IP[Symbol.iterator].call(5));
console.log(typeof Iterator, Iterator.prototype.constructor === Iterator, Object.prototype.toString.call(IP));
try { new Iterator(); } catch (e) { console.log('abstract', e.name); }
class Counter extends Iterator { next() { return { done: true, value: undefined }; } }
console.log('subclass', new Counter() instanceof Iterator, [...new Counter()].length);
const own = []; for (const k in {}) own.push(k); console.log('object keys', own.length);
