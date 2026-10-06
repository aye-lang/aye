const start = Date.UTC(2024, 0, 1);
const end = Date.UTC(2024, 11, 31);
console.log((end - start) / 86400000 + 1);
const days = [];
for (let i = 0; i < 7; i++) days.push(new Date(start + i * 86400000).getUTCDay());
console.log(days);
const a = new Date(Date.UTC(2024, 1, 28)), b = new Date(Date.UTC(2024, 2, 1));
console.log((b - a) / 86400000, a < b, +a === a.getTime(), a.valueOf() === a.getTime());
console.log(new Date(Date.UTC(2024, 12, 1)).toISOString(), new Date(Date.UTC(2024, -1, 1)).toISOString());
