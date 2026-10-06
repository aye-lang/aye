const shape = {
  init(name) { this.name = name; return this; },
  describe() { return this.name + ' with area ' + this.area(); },
};
const square = Object.create(shape);
square.setSide = function (s) { this.side = s; return this; };
square.area = function () { return this.side * this.side; };
const sq = Object.create(square).init('sq').setSide(3);
console.log(sq.describe());
console.log(shape.isPrototypeOf(sq), square.isPrototypeOf(sq), sq.isPrototypeOf(square));
console.log(Object.keys(sq));
const circle = Object.create(shape, { area: { value() { return Math.round(Math.PI * this.r * this.r); } } });
const c = Object.create(circle);
c.r = 2;
c.init('circle');
console.log(c.describe());
