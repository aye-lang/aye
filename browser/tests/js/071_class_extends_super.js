class Animal {
  constructor(name) { this.name = name; }
  speak() { return `${this.name} makes a noise`; }
  get kind() { return 'animal'; }
  static create(n) { return new this(n); }
}
class Dog extends Animal {
  constructor(name, breed) { super(name); this.breed = breed; }
  speak() { return super.speak() + ' (woof)'; }
  get kind() { return 'dog/' + super.kind; }
}
const d = new Dog('Rex', 'lab');
console.log(d.speak(), d.kind, d.breed);
console.log(d instanceof Dog, d instanceof Animal);
console.log(Dog.create('Spot') instanceof Dog, Dog.create('Spot').speak());
console.log(Object.getPrototypeOf(Dog) === Animal, Object.getPrototypeOf(Dog.prototype) === Animal.prototype);
console.log(d);
class Puppy extends Dog {}
const p = new Puppy('Bit', 'mix');
console.log(p.speak(), p.constructor.name);
