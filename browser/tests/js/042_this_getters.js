const rect = {
  w: 3, h: 4,
  get area() { return this.w * this.h; },
  set dims([w, h]) { this.w = w; this.h = h; },
};
console.log(rect.area);
rect.dims = [5, 6];
console.log(rect.area, rect.w);
const child = Object.create(rect);
child.w = 10;
console.log(child.area, rect.area);
const o = { n: 1, inc() { this.n++; return this; } };
console.log(o.inc().inc().n);
