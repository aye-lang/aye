const thrown = [42, 'str', null, undefined, { code: 7 }, [1, 2], true];
for (const t of thrown) {
  try {
    throw t;
  } catch (e) {
    console.log(typeof e, e === t, e instanceof Error);
  }
}
try { throw { name: 'Custom', message: 'obj' }; } catch (e) { console.log(e.name, e.message); }
function thrower() { throw 'from function'; }
try { thrower(); } catch (e) { console.log(e); }
