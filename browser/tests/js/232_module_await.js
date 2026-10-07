(async function () {
  const [live, same] = await Promise.all([
    import('./modules/live.mjs'), import('./modules/live.mjs')
  ]);
  console.log('await', live === same, live.loadedFirst, live.initial, live.after, live.label, live.topThis);
  const counter = await import('./modules/counter.mjs');
  counter.increment();
  console.log('binding', live.read(), live.metaSame());
  const namespace = await import('./modules/namespace.mjs');
  console.log('namespace import', namespace.sameLabel, namespace.get());
  console.log('seal', Object.isSealed(counter), Object.isFrozen(counter));
  console.log('set', Reflect.set(counter, 'count', 99), counter.count);
  console.log('define', Reflect.defineProperty(counter, 'count', { value: counter.count }),
    Reflect.defineProperty(counter, 'count', { value: 99 }),
    Reflect.defineProperty(counter, 'count', { writable: false }));
  console.log('proto', Reflect.setPrototypeOf(counter, null), Reflect.setPrototypeOf(counter, {}));
  try { Object.freeze(counter); } catch (e) { console.log('freeze', e.name); }
  try { (function () { 'use strict'; counter.count = 99; })(); } catch (e) { console.log('strict set', e.name); }
  try { await import('./modules/missing-export.mjs'); } catch (e) { console.log('missing', e.name); }
  try { await import('./modules/undefined-export.mjs'); } catch (e) { console.log('undefined export', e.name); }
})();
