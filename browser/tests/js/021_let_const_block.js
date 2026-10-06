let x = 1;
{
  let x = 2;
  const y = 3;
  console.log('inner', x, y);
  {
    let x = 4;
    console.log('innermost', x);
  }
}
console.log('outer', x, typeof y);
if (true) {
  const z = 'z';
  console.log(z);
}
console.log(typeof z);
for (let i = 0; i < 2; i++) {
  let i2 = i * 10;
  console.log(i, i2);
}
console.log(typeof i);
const obj = { a: 1 };
obj.a = 2;
obj.b = 3;
console.log(obj);
