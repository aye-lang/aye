const root = new Error('root problem');
const wrapped = new Error('wrapper', { cause: root });
console.log(wrapped.message, wrapped.cause === root, wrapped.cause.message);
const e2 = new TypeError('typed', { cause: 'just a string' });
console.log(e2.cause);
const e3 = new Error('no cause');
console.log('cause' in e3, e3.cause);
const e4 = new Error('opts', {});
console.log('cause' in e4);
function load() {
  try { JSON.parse('{x'); } catch (err) { throw new Error('load failed', { cause: err }); }
}
try { load(); } catch (e) { console.log(e.message, '<-', e.cause.name); }
