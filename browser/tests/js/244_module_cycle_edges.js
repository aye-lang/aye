// Cycles: TDZ of default, class and const exports; errors kept by every
// module of a cycle; load failures that stay failures; star exports in a
// cycle; ambiguous namespace re-exports; live bindings; evaluation order.
globalThis.log = (...a) => console.log(...a);
async function attempt(path) {
  try { return await import('./modules/edge/' + path + '.mjs'); }
  catch (e) { log(path, e.name, e.message.includes('b failed') ? 'b failed' : ''); return e; }
}
(async () => {
  const a = await attempt('tdz-a');
  log('after', (await attempt('tdz-b')).later(), a.default);
  const e1 = await attempt('throw-a');
  const e2 = await attempt('throw-b');
  const e3 = await attempt('throw-outer');
  log('same', e1 === e2, e2 === e3, globalThis.throwBRuns, globalThis.throwARan);
  // Node's error for a missing file is not a browser's TypeError.
  for (const path of ['load-a', 'load-a', 'load-b']) {
    try { await import('./modules/edge/' + path + '.mjs'); log('never'); } catch (e) { log(path, e.code === 'ERR_MODULE_NOT_FOUND' || e.message === 'Failed to fetch module'); }
  }
  const s1 = await attempt('star-1');
  const s2 = await attempt('star-2');
  log('stars', Object.keys(s1).join(), Object.keys(s2).join());
  log('local wins', (await attempt('star-local')).dup);
  log('ns-stars', Object.keys(await attempt('ns-stars')).join());
  await attempt('ns-import');
  await attempt('ring-import');
  log('ring keys', Object.keys(await attempt('ring-1')).join());
  const live = await attempt('live-a');
  log('live after', live.n, (await attempt('live-b')).readN());
  await attempt('diamond');
  const t1 = await attempt('tla-throw-a');
  const t2 = await attempt('tla-throw-b');
  log('tla same', t1 === t2);
  const c1 = await import('./modules/counter.mjs');
  const c2 = await import('./modules/../modules/counter.mjs');
  const c3 = await import('./modules/counter.mjs?other');
  log('map by URL', c1 === c2, c1 === c3, globalThis.moduleEvaluations);
})().catch(e => log('fail', e));
