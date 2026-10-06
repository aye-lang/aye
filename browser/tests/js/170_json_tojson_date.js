const d = new Date(Date.UTC(2021, 2, 4, 5, 6, 7, 89));
console.log(JSON.stringify(d), JSON.stringify({ d }), d.toJSON());
const custom = { val: 3, toJSON() { return { wrapped: this.val }; } };
console.log(JSON.stringify(custom), JSON.stringify([custom]));
const withKey = { toJSON(key) { return 'key=' + key; } };
console.log(JSON.stringify({ outer: withKey, arr: [withKey] }));
class Temp { constructor(c) { this.c = c; } toJSON() { return `${this.c}C`; } }
console.log(JSON.stringify({ t: new Temp(20) }));
console.log(JSON.stringify(new Date(NaN)));
