console.log(v);
var v = 5;
console.log(v);
function f() {
  console.log('in f', w);
  if (true) {
    var w = 'w';
  }
  console.log('after', w);
  for (var k = 0; k < 3; k++) {}
  console.log('k', k);
}
f();
var v = 6;
console.log(v);
var counter;
console.log(counter);
function g() {
  var a = 1;
  {
    var a = 2;
  }
  return a;
}
console.log(g());
