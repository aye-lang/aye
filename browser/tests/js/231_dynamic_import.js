// Runs as a classic script, including imports from later function calls.
console.log('start');
function later() { return import('./modules/counter.mjs'); }
(async function () {
  const [a, b] = await Promise.all([later(), later()]);
  console.log('namespace', Object.keys(a).join(','), Object.getPrototypeOf(a) === null);
  console.log('shape', Object.prototype.toString.call(a), Object.isExtensible(a));
  console.log('once', a === b, globalThis.moduleEvaluations);
  console.log('values', a.count, a.default, a.z);
  a.increment();
  console.log('live', a.count, b.count);
  console.log('descriptor', JSON.stringify(Object.getOwnPropertyDescriptor(a, 'count')));
  const nested = await import('./modules/nested.mjs');
  console.log('nested', nested.staticCount, (await nested.load()) === a, nested.urlTail());
  try { await import(Symbol('bad')); } catch (e) { console.log('symbol', e.name); }
  try { await import({ toString() { throw 'coercion'; } }); } catch (e) { console.log('coercion', e); }
  try { await import((function () { throw 'argument'; })()); } catch (e) { console.log('argument', e); }
  try { await import('./modules/syntax.mjs'); } catch (e) { console.log('parse', e.name); }
  for (let i = 0; i < 2; i++) {
    try { await import('./modules/thrown.mjs'); } catch (e) { console.log('throw', e); }
  }
  console.log('throw once', globalThis.throwEvaluations);
  for (const source of ['import', 'var import = 1', 'import()', 'import(...[])', 'import.meta', 'new import("x")']) {
    try { new Function(source); } catch (e) { console.log('syntax', e.name); }
  }
})();
console.log('end');
