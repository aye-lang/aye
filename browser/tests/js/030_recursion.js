function fib(n) { return n < 2 ? n : fib(n - 1) + fib(n - 2); }
console.log([0, 1, 2, 3, 10, 20].map(fib));
function fact(n) { return n <= 1 ? 1 : n * fact(n - 1); }
console.log(fact(5), fact(10), fact(20), fact(25));
function isEven(n) { return n === 0 ? true : isOdd(n - 1); }
function isOdd(n) { return n === 0 ? false : isEven(n - 1); }
console.log(isEven(10), isOdd(7), isEven(7));
function sumTree(t) { return t.v + (t.kids || []).reduce((s, k) => s + sumTree(k), 0); }
console.log(sumTree({ v: 1, kids: [{ v: 2 }, { v: 3, kids: [{ v: 4 }] }] }));
function hanoi(n, a, b, c, moves) { if (n === 0) return moves; hanoi(n - 1, a, c, b, moves); moves.push(a + c); return hanoi(n - 1, b, a, c, moves); }
console.log(hanoi(3, 'A', 'B', 'C', []).join(' '));
function depth(n) { return n === 0 ? 0 : 1 + depth(n - 1); }
console.log(depth(5000));
function perms(s) { if (s.length <= 1) return [s]; const out = []; for (let i = 0; i < s.length; i++) for (const p of perms(s.slice(0, i) + s.slice(i + 1))) out.push(s[i] + p); return out; }
console.log(perms('abc'));
