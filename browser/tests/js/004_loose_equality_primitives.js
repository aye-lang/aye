const vals = [null, undefined, 0, '', '0', false, 'false', NaN];
const names = ['null', 'undefined', '0', "''", "'0'", 'false', "'false'", 'NaN'];
for (let i = 0; i < vals.length; i++) {
  let row = names[i] + ':';
  for (let j = 0; j < vals.length; j++) {
    row += ' ' + (vals[i] == vals[j] ? 'T' : '.');
  }
  console.log(row);
}
console.log(1 == '1', 1 == true, 2 == true, '1' == true, '' == 0, ' \t\n' == 0);
console.log(null == 0, undefined == 0, null == false, undefined == null);
