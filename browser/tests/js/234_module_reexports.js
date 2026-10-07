// Re-exports (`export * from`, `export {a as b} from`, `export * as ns
// from`), exported destructuring and default declarations.
globalThis.log = (...a) => console.log(...a);
(async () => {
  const all = await import('./modules/re/all.mjs');
  log('keys', JSON.stringify(Reflect.ownKeys(all).map(String)));
  log('values', all.x, all.same, all.renamed, all.yDefault, all.default, all.local, all['string name']);
  log('dup' in all, all.dup, 'z' in all, all.sub.y, all.sub.same, all.sub.default);
  log('self', all.self === all, Object.getOwnPropertyDescriptor(all, 'sub').value === all.sub);
  all.bump();
  log('live through re-export', all.counter, all.sub === await import('./modules/re/y.mjs'));
  const chain = await import('./modules/re/star-chain.mjs');
  log('chain', Object.keys(chain).join(), 'default' in chain);
  log('importer', (await import('./modules/re/importer.mjs')).seen);
  for (const name of ['ambiguous', 'missing', 'bad-reexport']) {
    try { await import('./modules/re/' + name + '.mjs'); log('no error', name); }
    catch (e) { log(name, e.name + ': ' + e.message); }
  }
  const d = await import('./modules/re/destructure.mjs');
  log('destructure', d.a, d.b0, d.b1, JSON.stringify(d.bRest), d.renamedD, JSON.stringify(d.others));
  log('array', d.first, d.third, d.fourth, d.missing, Object.keys(d).join());
  d.setFirst('changed');
  log('live', d.first);
  const defs = await import('./modules/re/defaults.mjs');
  log('defaults', defs.anonClass.name, defs.anonClass.who(), defs.anonFn.name, defs.anonFn());
  log('more', defs.arrow.name, defs.arrow(), defs.paren.name, defs.paren(), defs.named.name, defs.hoisted, typeof defs.named);
})().catch(e => log('fail', e));
