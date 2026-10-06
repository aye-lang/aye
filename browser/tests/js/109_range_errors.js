try { (1.5).toFixed(101); } catch (e) { console.log(e.name, e instanceof RangeError); }
try { new Array(-1); } catch (e) { console.log(e.name, e.message); }
try { 'x'.repeat(-1); } catch (e) { console.log(e.name, e.message); }
try { 'x'.repeat(Infinity); } catch (e) { console.log(e.name); }
try { (255).toString(1); } catch (e) { console.log(e.name); }
try { (255).toString(37); } catch (e) { console.log(e.name); }
try { (1).toPrecision(0); } catch (e) { console.log(e.name); }
try { [].length = -1; } catch (e) { console.log(e.name); }
try { new Array(1.5); } catch (e) { console.log(e.name); }
console.log((1.5).toFixed(100).length);
