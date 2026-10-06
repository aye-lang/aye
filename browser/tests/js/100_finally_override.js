function a() { try { return 'try'; } finally { return 'finally'; } }
console.log(a());
function b() { try { throw new Error('lost'); } finally { return 'swallowed'; } }
console.log(b());
function c() { try { throw new Error('first'); } catch (e) { throw new Error('second'); } finally { return 'fin wins'; } }
console.log(c());
function d() {
  let x = 1;
  try { return x; } finally { x = 2; }
}
console.log(d());
function e() {
  for (const i of [1, 2, 3]) {
    try { if (i === 2) return 'returned ' + i; } finally { if (i === 2) console.log('cleanup', i); }
  }
}
console.log(e());
function f() {
  while (true) { try { return 'never'; } finally { break; } }
  return 'after break';
}
console.log(f());
