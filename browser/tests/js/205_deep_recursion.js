function depth(n) { return n === 0 ? 0 : 1 + depth(n - 1); }
console.log(depth(5000));
try {
  (function forever() { forever(); })();
} catch (e) {
  console.log(e instanceof RangeError, e.message);
}
console.log('still running');
