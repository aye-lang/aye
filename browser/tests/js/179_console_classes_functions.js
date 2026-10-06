class Point { constructor(x, y) { this.x = x; this.y = y; } }
class Point3 extends Point { constructor(x, y, z) { super(x, y); this.z = z; } }
class Empty {}
console.log(new Point(1, 2), new Point3(1, 2, 3), new Empty());
console.log([new Point(0, 0)], { p: new Point(5, 6) });
function named() {}
const anon = function () {};
console.log(named, Point, Point3, Math.max);
console.log([named], { f: named });
console.log(Object.create(null), Object.assign(Object.create(null), { k: 'v' }));
