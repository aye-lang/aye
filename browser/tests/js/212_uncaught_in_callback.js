[1, 2, 3].forEach((n) => {
  console.log('n =', n);
  if (n === 2) throw new RangeError('too big: ' + n);
});
