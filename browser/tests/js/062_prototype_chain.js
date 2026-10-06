function Animal(name) { this.name = name; }
Animal.prototype.describe = function () { return 'animal ' + this.name; };
function Dog(name, breed) { Animal.call(this, name); this.breed = breed; }
Dog.prototype = Object.create(Animal.prototype);
Dog.prototype.constructor = Dog;
Dog.prototype.bark = function () { return this.name + ' woofs'; };
Dog.prototype.describe = function () { return 'dog/' + Animal.prototype.describe.call(this); };
const d = new Dog('Fido', 'lab');
console.log(d.bark(), d.describe(), d.breed);
console.log(d instanceof Dog, d instanceof Animal, d instanceof Object);
console.log(Object.getPrototypeOf(Object.getPrototypeOf(d)) === Animal.prototype);
let depth = 0;
for (let p = d; p; p = Object.getPrototypeOf(p)) depth++;
console.log('chain length', depth);
console.log(Animal.prototype.isPrototypeOf(d), Dog.prototype.isPrototypeOf(new Animal('x')));
