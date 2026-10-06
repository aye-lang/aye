function Animal(name) { this.name = name; }
Animal.prototype.speak = function () { return this.name + ' makes a sound'; };
Animal.prototype.legs = 4;
const a = new Animal('Rex');
console.log(a.speak(), a.legs, a.hasOwnProperty('legs'));
console.log(a.constructor === Animal, Animal.prototype.constructor === Animal);
console.log(Object.getPrototypeOf(a) === Animal.prototype);
Animal.prototype.legs = 3;
console.log(a.legs);
a.legs = 5;
console.log(a.legs, Animal.prototype.legs, new Animal('b').legs);
console.log(Object.keys(a), a instanceof Animal);
console.log(a);
