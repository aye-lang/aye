// JSON modules (import attributes) and import() options.
(async () => {
  const user = await import('./modules/json/user.mjs');
  console.log('static', user.summary);
  const ns = await import('./modules/json/data.json', { with: { type: 'json' } });
  console.log('dynamic', ns.default.nested.ok, ns.default === (await import('./modules/json/data.json', { with: { type: 'json' } })).default);
  for (const options of [null, 1, { with: 1 }, { with: { type: 1 } }]) {
    try { await import('./modules/json/data.json', options); console.log('no error'); }
    catch (e) { console.log('options', e.name); }
  }
  try { await import('./modules/json/data.json', { with: { type: 'css' } }); } catch (e) { console.log('type', e.name); }
  console.log('trailing comma', typeof (await import('./modules/json/user.mjs',)).summary);
})().catch(e => console.log('fail', e));
