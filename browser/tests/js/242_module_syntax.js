// Module syntax forms real bundles use: empty import lists, trailing
// commas, string names, `export * as "x"`, anonymous default classes and
// generators, `for await` at the top level.
(async () => {
  const f = await import('./modules/syntax/forms.mjs');
  console.log(f.summary);
  console.log(new f.default().who(), f.default.name, f.cDefault, Object.keys(f['every thing']).join());
  const g = await import('./modules/syntax/gen-default.mjs');
  console.log([...g.default()].join(), g.default.name);
})().catch(e => console.log('fail', e));
