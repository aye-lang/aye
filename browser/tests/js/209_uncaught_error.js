function fail() { throw new Error('boom'); }
try { fail(); } catch (e) { console.log('caught once:', e.message); }
fail();
