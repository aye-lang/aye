class Shape {
  constructor(n) { this.n = n; }
  get area() { return 0; }
  describe() { return `${this.n}: ${this.area.toFixed(1)}`; }
}
class Circle extends Shape {
  constructor(r) { super('circle'); this.r = r; }
  get area() { return Math.PI * this.r ** 2; }
}
class Rect extends Shape {
  constructor(w, h) { super('rect'); this.w = w; this.h = h; }
  get area() { return this.w * this.h; }
  set size([w, h]) { this.w = w; this.h = h; }
}
const shapes = [new Circle(1), new Rect(2, 3), new Shape('blob')];
console.log(shapes.map((s) => s.describe()));
const r = shapes[1];
r.size = [10, 10];
console.log(r.area, shapes.reduce((s, x) => s + x.area, 0).toFixed(3));
