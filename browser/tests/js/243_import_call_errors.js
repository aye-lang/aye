// import(): an exception while evaluating the specifier or the options
// expression is thrown at once; converting the specifier to a string or
// validating the options rejects the promise.
let p;
try { p = import((function () { throw 'argument'; })()); console.log('no sync throw'); } catch (e) { console.log('sync', e); }
try { p = import('./x.mjs', (function () { throw 'options'; })()); console.log('no sync throw'); } catch (e) { console.log('sync', e); }
try { p = import({ toString() { throw 'tostring'; } }); console.log('no sync throw'); p.catch(e => console.log('rejected', e)); } catch (e) { console.log('sync', e); }
try { p = import('./x.mjs', 5); console.log('no sync throw'); p.catch(e => console.log('rejected', e.name)); } catch (e) { console.log('sync', e); }
