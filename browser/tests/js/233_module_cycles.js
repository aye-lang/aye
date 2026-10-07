// Cyclic static imports: functions are hoisted across the cycle, let
// bindings stay in their TDZ until their module runs, and each module
// runs once, after its dependencies outside the cycle.
globalThis.log = (...a) => console.log(...a);
(async () => {
  const a = await import('./modules/cycle/a.mjs');
  log('keys', Object.keys(a).join());
  log('fa', a.fa());
  const b = await import('./modules/cycle/b.mjs');
  log('useA', b.useA());
  const r = await import('./modules/cycle/r1.mjs');
  log('ring', r.order.join(' '));
  const self = await import('./modules/cycle/self.mjs');
  log('self', self.read(), self.same());
  await import('./modules/cycle/ticks.mjs');
  log('after ticks');
})().catch(e => log('fail', e));
