try {
  console.log(a);
  let a = 1;
} catch (e) {
  console.log(e.name);
}
function f() {
  try {
    return b;
  } catch (e) {
    return e instanceof ReferenceError;
  }
}
const b = 2;
console.log(f());
let ok = 'fine';
{
  try {
    console.log(ok);
    let ok = 'shadow';
  } catch (e) {
    console.log('tdz in block', e.name);
  }
}
console.log(typeof undeclaredVar);
