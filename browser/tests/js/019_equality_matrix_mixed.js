const items = [0, 1, '1', true, [], [1], null, undefined, 'x', { valueOf: () => 1 }];
const label = ['0', '1', "'1'", 'true', '[]', '[1]', 'null', 'undef', "'x'", 'obj1'];
for (let i = 0; i < items.length; i++) {
  const eq = [];
  for (let j = 0; j < items.length; j++) if (i !== j && items[i] == items[j]) eq.push(label[j]);
  console.log(label[i].padEnd(6), '==', eq.join(' ') || '-');
}
