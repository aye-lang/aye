// Dense arrays, comparator calls, and callback argument passing.
function arrays() {
  var sum = 0;
  for (var round = 0; round < 80; round++) {
    var a = [];
    for (var i = 0; i < 1000; i++) a.push((i * 7919 + round * 17) % 1009);
    a.sort(function(a, b) { return a - b; });
    sum += a.map(function(x) { return x * 3 + 1; })
      .filter(function(x) { return x % 5 !== 0; })
      .reduce(function(a, b) { return a + b; }, 0);
  }
  return sum;
}
console.log(arrays());
