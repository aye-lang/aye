try { missingVar; } catch (e) { console.log(e.name, e.message); }
try { missingFn(); } catch (e) { console.log(e.message); }
try { const y = undefinedThing + 1; } catch (e) { console.log(e.message, e instanceof ReferenceError); }
try { (() => notHere.prop)(); } catch (e) { console.log(e.message); }
console.log(typeof missingVar);
