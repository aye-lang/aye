// Namespace objects and import() promises: a `then` export, key order,
// property descriptors and the exotic object's methods, a fresh promise
// per call, shared top-level await across roots, import.meta.
globalThis.log = (...a) => console.log(...a);
(async () => {
  log('then', await import('./modules/edge/then.mjs'));
  const ns = await import('./modules/edge/keys.mjs');
  log('keys', JSON.stringify(Reflect.ownKeys(ns).map(String)));
  log('names', Object.getOwnPropertyNames(ns).join(), Object.getOwnPropertySymbols(ns).length);
  log('tag', JSON.stringify(Object.getOwnPropertyDescriptor(ns, Symbol.toStringTag)));
  log('uninit', ns.uninit, 'uninit' in ns, ns.hasOwnProperty, 'hasOwnProperty' in ns);
  log('missing', ns.nope, 'nope' in ns, Reflect.deleteProperty(ns, 'nope'), Reflect.deleteProperty(ns, 'a'));
  log('extensible', Object.isExtensible(ns), Reflect.preventExtensions(ns), Reflect.defineProperty(ns, 'n', { value: 1 }));
  log('define same', Reflect.defineProperty(ns, 'a', { value: 2, writable: true, enumerable: true, configurable: false }),
    Reflect.defineProperty(ns, 'a', { enumerable: false }), Reflect.defineProperty(ns, 'a', { get() {} }));
  log('symbol', Reflect.set(ns, Symbol.toStringTag, 'x'), ns[Symbol.toStringTag], Reflect.deleteProperty(ns, Symbol.toStringTag));
  log('spread', JSON.stringify({ ...ns }), JSON.stringify(Object.entries(ns)));
  const p1 = import('./modules/edge/keys.mjs');
  const p2 = import('./modules/edge/keys.mjs');
  log('promises', p1 instanceof Promise, p1 === p2, (await p1) === (await p2));
  await Promise.all([import('./modules/edge/tick-a.mjs'), import('./modules/edge/tick-b.mjs')]);
  log('shared runs', globalThis.sharedRuns);
  const meta = await import('./modules/edge/meta.mjs');
  log('meta', meta.url, meta.resolved, meta.proto);
})().catch(e => log('fail', e));
