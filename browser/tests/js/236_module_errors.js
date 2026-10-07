// Module errors: evaluation errors are cached per module, link errors
// run nothing, early errors reject, and imports are immutable live views.
globalThis.log = (...a) => console.log(...a);
async function attempt(name) {
  try { await import('./modules/err/' + name + '.mjs'); log(name, 'ok'); return null; }
  catch (e) { log(name, e.name); return e; }
}
(async () => {
  const e1 = await attempt('parent');
  const e2 = await attempt('throws');
  log('same error', e1 === e2, e1.message, globalThis.throwRuns);
  await attempt('link-fail');
  await attempt('syntax-parent');
  log('side effects', globalThis.sideRuns);
  await attempt('side');
  log('side effects', globalThis.sideRuns);
  await attempt('link-fail');
  await attempt('dup-export');
  await attempt('undeclared');
  await attempt('dup-import');
  await attempt('assign');
})().catch(e => log('fail', e));
