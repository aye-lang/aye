const o = Object.freeze({ a: 1, nested: { b: 2 } });
try { o.a = 99; } catch (e) {}
try { o.c = 3; } catch (e) {}
try { delete o.a; } catch (e) {}
o.nested.b = 20;
console.log(o, Object.isFrozen(o), Object.isFrozen(o.nested));
const arr = Object.freeze([1, 2]);
try {
  arr.push(3);
} catch (e) {
  console.log(e.name);
}
console.log(arr);
console.log(Object.isFrozen({}), Object.isFrozen(Object.freeze({})), Object.isFrozen(1));
const s = Object.seal({ x: 1 });
s.x = 2;
try { s.y = 3; } catch (e) {}
try { delete s.x; } catch (e) {}
console.log(s, Object.isSealed(s), Object.isFrozen(s));
const ne = Object.preventExtensions({ k: 1 });
try { ne.z = 1; } catch (e) {}
console.log(ne, Object.isExtensible(ne), Object.isExtensible({}));
