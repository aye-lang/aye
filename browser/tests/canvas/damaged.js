const canvas = document.getElementById('canvas');
const ctx = canvas.getContext('2d');
let count = 0;
function attempt(f) {
  try {
    f();
  } catch (e) {
    if (!(e instanceof Error)) throw e;
  }
  count++;
}
for (const n of [NaN, Infinity, -Infinity, 1e300, -1e300]) {
  attempt(() => ctx.fillRect(n, n, n, n));
  attempt(() => ctx.strokeRect(n, n, n, n));
  attempt(() => ctx.clearRect(n, n, n, n));
  attempt(() => ctx.moveTo(n, n));
  attempt(() => ctx.lineTo(n, n));
  attempt(() => ctx.bezierCurveTo(n, n, n, n, n, n));
  attempt(() => ctx.quadraticCurveTo(n, n, n, n));
  attempt(() => ctx.arc(n, n, n, n, n));
  attempt(() => ctx.arcTo(n, n, n, n, n));
  attempt(() => ctx.ellipse(n, n, n, n, n, n, n));
  attempt(() => ctx.roundRect(n, n, n, n, n));
  attempt(() => ctx.transform(n, n, n, n, n, n));
  attempt(() => ctx.fillText('damaged', n, n, n));
  attempt(() => ctx.getImageData(n, n, n, n));
  attempt(() => ctx.createImageData(n, n));
  attempt(() => ctx.createLinearGradient(n, n, n, n));
  attempt(() => ctx.createRadialGradient(n, n, n, n, n, n));
  attempt(() => ctx.createConicGradient(n, n, n));
  attempt(() => ctx.isPointInPath(n, n));
  attempt(() => ctx.isPointInStroke(n, n));
}
for (const p of ['M1e300 1e300L-1e300 -1e300Z', 'M0 0C1e300 0 0 1e300 5 5', 'M0 0A1e300 1e300 0 0 0 4 4', 'not a path']) {
  attempt(() => ctx.fill(new Path2D(p)));
  attempt(() => ctx.stroke(new Path2D(p)));
}
attempt(() => ctx.arc(0, 0, -1, 0, 1));
attempt(() => ctx.roundRect(0, 0, 5, 5, [-1]));
attempt(() => ctx.fillRect(5, 5, -10, -10));
attempt(() => ctx.getImageData(5, 5, -10, -10));
attempt(() => ctx.createImageData(100000, 100000));
canvas.width = 0;
canvas.height = 0;
attempt(() => ctx.fillRect(0, 0, 100, 100));
if (canvas.toDataURL() !== 'data:,') throw new Error('zero canvas');
canvas.width = 100000;
canvas.height = 100000;
attempt(() => ctx.fillRect(0, 0, 100000, 100000));
attempt(() => ctx.drawImage(canvas, 0, 0));
if (canvas.toDataURL() !== 'data:,') throw new Error('oversize canvas not refused');
canvas.setAttribute('width', '-1');
canvas.setAttribute('height', '-1');
if (canvas.width !== 300 || canvas.height !== 150) throw new Error('invalid attributes');
attempt(() => ctx.fillRect(0, 0, 2, 2));
const reentrant = new ImageData(1, 1);
reentrant.data = {
  get 0() {
    canvas.width = 0;
    return 255;
  }
};
attempt(() => ctx.putImageData(reentrant, 0, 0));
canvas.width = 10;
const paint = ctx.createLinearGradient(0, 0, 10, 0);
Object.defineProperty(paint, 'kind', {
  get() {
    canvas.height = 0;
    return 1;
  }
});
ctx.fillStyle = paint;
attempt(() => ctx.fillRect(0, 0, 10, 10));
document.getElementById('results').textContent = count + ' damaged-input operations survived';
