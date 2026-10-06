try {
  const x = [...123];
} catch (e) {
  console.log(e.name);
}
try {
  Math.max(...null);
} catch (e) {
  console.log(e.name);
}
console.log({ ...123 }, { ...true }, { ...'ab' });
const holes = [, 'b'];
console.log([...holes], Object.keys([...holes]));
function count() { return arguments.length; }
console.log(count(...[undefined, undefined]), count(...[]));
console.log(Array.of(...'abc'));
