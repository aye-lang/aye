try { const [a] = undefined; } catch (e) { console.log(e.name); }
try { const { a } = undefined; } catch (e) { console.log(e.name); }
try { const [a] = {}; } catch (e) { console.log(e.name); }
try { const { a: { b } } = { a: null }; } catch (e) { console.log(e.name); }
const { a = 1 } = { a: null };
console.log(a);
const [b = 2] = [null];
console.log(b);
const { length: len = 0 } = [];
console.log(len);
