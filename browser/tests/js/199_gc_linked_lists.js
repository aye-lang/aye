function build(n) {
  let head = null;
  for (let i = 0; i < n; i++) head = { value: i, next: head };
  return head;
}
let total = 0;
for (let round = 0; round < 5; round++) {
  let list = build(100000);
  let count = 0;
  while (list) { count += list.value % 3; list = list.next; }
  total += count;
}
console.log(total);
