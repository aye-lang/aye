const obj = { a: 1, nested: { x: 2 } };
try { obj.foo(); } catch (e) { console.log(e.name + ': ' + e.message); }
try { obj.nested.bar(); } catch (e) { console.log(e.message); }
try { obj.a(); } catch (e) { console.log(e.message); }
const notFn = 5;
try { notFn(); } catch (e) { console.log(e.message); }
try { undefined.x; } catch (e) { console.log(e instanceof TypeError); }
try { new (function () {}.prototype.constructor)(); console.log('ok ctor'); } catch (e) { console.log('no'); }
try { new obj.a(); } catch (e) { console.log(e.name); }
