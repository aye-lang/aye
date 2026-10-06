class Point {
  constructor(x, y) { this.x = x; this.y = y; }
  add(o) { return new Point(this.x + o.x, this.y + o.y); }
  toString() { return `(${this.x}, ${this.y})`; }
}
const p = new Point(1, 2).add(new Point(3, 4));
console.log(p.x, p.y, String(p), `${p}`, p + '');
console.log(p);
console.log(typeof Point, p instanceof Point, p.constructor === Point);
console.log(Object.keys(p), Object.getOwnPropertyNames(Point.prototype));
try {
  Point(1, 2);
} catch (e) {
  console.log(e.name);
}
class Empty {}
console.log(new Empty(), Object.keys(new Empty()).length);
