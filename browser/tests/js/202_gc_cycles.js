let made = 0;
for (let i = 0; i < 100000; i++) {
  const a = { name: 'a' }, b = { name: 'b' };
  a.other = b;
  b.other = a;
  made += a.other.other === a ? 1 : 0;
}
console.log(made);
