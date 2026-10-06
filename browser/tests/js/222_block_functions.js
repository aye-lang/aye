// Sloppy mode code: a function declared in a block is also a var of the
// enclosing function (Annex B.3.3), set when the block runs (ZOL's
// `if (typeof get_cookie == "undefined") { function get_cookie() {} }`).
console.log(typeof early);
if (typeof early === 'undefined') {
  function early() { return 'early'; }
}
console.log(early());
function f() {
  const before = typeof inner;
  {
    function inner() { return 1; }
  }
  return [before, typeof inner, inner()];
}
console.log(f());
function g(flag) {
  if (flag) {
    function pick() { return 'a'; }
  } else {
    function pick() { return 'b'; }
  }
  return pick();
}
console.log(g(true), g(false));
function h() {
  let shadow = 'let';
  {
    function shadow() {}
  }
  return typeof shadow;
}
console.log(h());
function s() {
  'use strict';
  {
    function hidden() {}
  }
  return typeof hidden;
}
console.log(s());
function loop() {
  const out = [];
  for (let i = 0; i < 2; i++) {
    function step() { return i; }
    out.push(step());
  }
  return [out, typeof step];
}
console.log(loop());
switch (1) {
  case 1:
    function fromSwitch() { return 'switch'; }
}
console.log(fromSwitch());
