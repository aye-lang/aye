// Escaping environments, object literals, and short-lived closures.
function factory(seed) {
  var state = { x: seed, y: seed + 1, count: 0 };
  return function(delta) {
    state.x += delta;
    state.y = state.x * 2 + state.count++;
    return { x: state.x, y: state.y, read: function() { return state.y; } };
  };
}
function closures() {
  var sum = 0;
  for (var i = 0; i < 300000; i++) {
    var f = factory(i % 97);
    for (var j = 0; j < 4; j++) {
      var o = f(j);
      sum += o.x + o.y + o.read();
    }
  }
  return sum;
}
console.log(closures());
