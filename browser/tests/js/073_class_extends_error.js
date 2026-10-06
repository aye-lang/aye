class ValidationError extends Error {
  constructor(field, msg) {
    super(msg);
    this.name = 'ValidationError';
    this.field = field;
  }
}
class NotFound extends Error {
  constructor(what) { super(`${what} not found`); this.name = 'NotFound'; }
}
try {
  throw new ValidationError('age', 'must be positive');
} catch (e) {
  console.log(e.name, e.message, e.field);
  console.log(e instanceof ValidationError, e instanceof Error, String(e));
}
const nf = new NotFound('user');
console.log(nf.message, nf.name, `${nf}`);
class Plain extends Error {}
const p = new Plain('plain msg');
console.log(p.name, p.message, p instanceof Plain, String(p));
console.log(Object.prototype.toString.call(nf));
