function kind(x) {
  switch (x) {
    case 1:
    case 2:
      return 'small';
    case 3:
      return 'three';
    default:
      return 'other';
    case 'a':
      return 'letter';
  }
}
console.log([1, 2, 3, 4, 'a', '1'].map(kind));
function fall(n) {
  const out = [];
  switch (n) {
    case 0: out.push(0);
    case 1: out.push(1);
    default: out.push('d');
    case 2: out.push(2); break;
    case 3: out.push(3);
  }
  return out.join(',');
}
console.log(fall(0), fall(1), fall(2), fall(3), fall(9));
switch (true) {
  case 5 > 10: console.log('no'); break;
  case 5 > 1: console.log('yes'); break;
}
let x = 0;
switch (x) { case 0: { let y = 'block'; console.log(y); } }
switch (NaN) { case NaN: console.log('nan matched'); break; default: console.log('nan not matched'); }
