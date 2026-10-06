const o = { a: undefined, n: null };
try { o.a.x; } catch (e) { console.log(e.name, e.message); }
try { o.n.y; } catch (e) { console.log(e.message); }
try { o.missing.deep; } catch (e) { console.log(e.message); }
try { o.a['key']; } catch (e) { console.log(e.message); }
try { o.a.x = 1; } catch (e) { console.log(e.message); }
try { let u; u.length; } catch (e) { console.log(e.message); }
console.log(o.a?.x, o.n?.y);
