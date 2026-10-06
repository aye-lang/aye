// Rope construction/flattening, numeric formatting, and regexp matching.
function strings() {
  var sum = 0;
  for (var r = 0; r < 120; r++) {
    var s = "";
    for (var i = 0; i < 300; i++) s += "item-" + i + ":" + (i % 17) + ";";
    var words = s.match(/item-[0-9]+:[0-9]+/g);
    sum += words.length + s.replace(/item-/g, "x").length;
    sum += s.split(";").map(function(x) { return x.length; }).reduce(function(a, b) { return a + b; }, 0);
  }
  return sum;
}
console.log(strings());
