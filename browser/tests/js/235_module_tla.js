// Top-level await: async module evaluation (ES2022) orders siblings,
// settles cycles and caches rejections.
globalThis.log = (...a) => console.log(...a);
(async () => {
  await import('./modules/tla/main.mjs');
  log('-- main done');
  const x = await import('./modules/tla/x.mjs');
  log('-- cycle done', x.x);
  await import('./modules/tla/parents.mjs');
  log('-- parents done');
  let first;
  try { await import('./modules/tla/reject-parent.mjs'); } catch (e) { first = e; log('rejected', e.message); }
  try { await import('./modules/tla/reject.mjs'); } catch (e) { log('same error', e === first, globalThis.rejectRuns); }
  try { await import('./modules/tla/reject-parent.mjs'); } catch (e) { log('same again', e === first); }
  await import('./modules/tla/order.mjs');
  log('-- order done');
  const [p, q] = await Promise.all([import('./modules/tla/y.mjs'), import('./modules/tla/y.mjs')]);
  log('same namespace', p === q, p.y);
})().catch(e => log('fail', e));
