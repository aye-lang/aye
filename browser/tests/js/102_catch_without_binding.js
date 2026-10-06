let r;
try {
  JSON.parse('{bad');
  r = 'parsed';
} catch {
  r = 'failed';
}
console.log(r);
function safe(fn, dflt) { try { return fn(); } catch { return dflt; } }
console.log(safe(() => { throw 1; }, 'd'), safe(() => 5, 'd'));
try { null.x; } catch { console.log('caught null access'); } finally { console.log('done'); }
