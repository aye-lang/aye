const c = 1;
try {
  c = 2;
} catch (e) {
  console.log(e.name, e.message);
}
console.log(c);
try {
  const d = [1];
  d.push(2);
  console.log(d);
  d = [];
} catch (e) {
  console.log(e instanceof TypeError);
}
for (const k of [1, 2]) console.log('k', k);
try {
  for (const i = 0; i < 2; i++) console.log('i', i);
} catch (e) {
  console.log('loop', e.name);
}
