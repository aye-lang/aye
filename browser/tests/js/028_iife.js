const r = (function () { return 'iife'; })();
console.log(r);
const r2 = (() => 42)();
console.log(r2);
const mod = (function () {
  let priv = 0;
  function bump() { priv += 1; }
  return { bump, value() { return priv; } };
})();
mod.bump(); mod.bump();
console.log(mod.value(), typeof priv);
(function named(n) {
  if (n > 0) { console.log('n', n); named(n - 1); }
})(3);
console.log((function (a, b) { return a * b; })(6, 7));
!function () { console.log('bang iife'); }();
