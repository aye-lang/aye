function A() {}
function B() {}
B.prototype = Object.create(A.prototype);
const b = new B();
console.log(b instanceof B, b instanceof A);
const savedProto = B.prototype;
B.prototype = {};
console.log(b instanceof B, b instanceof A);
B.prototype = savedProto;
console.log(b instanceof B);
const nul = Object.create(null);
console.log(nul instanceof Object);
console.log(Object.prototype.isPrototypeOf(b), Object.prototype.isPrototypeOf(nul));
try {
  console.log({} instanceof {});
} catch (e) {
  console.log(e.name);
}
console.log(5 instanceof Number, 'x' instanceof String);
