const users = [{ id: 1, name: 'a', roles: ['admin'] }, { id: 2, name: 'b', roles: [] }];
for (const { id, name, roles: [role = 'guest'] } of users) console.log(id, name, role);
for (const [k, v] of Object.entries({ one: 1, two: 2 })) console.log(k, v);
for (const [k, v] of new Map([['m', 1]])) console.log('map', k, v);
for (const [i, ch] of [...'xyz'].entries()) console.log(i, ch);
const pairs = [[1, 2], [3, 4]];
let sum = 0;
for (const [a, b] of pairs) sum += a * b;
console.log(sum);
for (let [a, , c = 'C'] of [[1, 2], [4, 5, 6]]) { a *= 10; console.log(a, c); }
