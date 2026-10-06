// Native JSON parsing/stringification, key enumeration, and allocations.
function json() {
  var rows = [], sum = 0;
  for (var i = 0; i < 200; i++)
    rows.push({id:i, name:"item-" + i, active:i % 3 === 0, tags:["alpha", "beta", "gamma"], data:{x:i * 7, y:i / 2}});
  var text = JSON.stringify({rows:rows, title:"round trip", version:1});
  for (var r = 0; r < 1000; r++) {
    var obj = JSON.parse(text);
    sum += obj.rows[r % 200].data.x + text.length;
    text = JSON.stringify(obj);
  }
  return sum;
}
console.log(json());
