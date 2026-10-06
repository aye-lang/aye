const o = { get g() { return 1; }, set s(v) {}, get gs() { return 2; }, set gs(v) {} };
console.log(o);
class T { get computed() { return 42; } }
console.log(new T(), new T().computed);
