const d = new Date(Date.UTC(2021, 11, 31, 23, 59, 58, 999));
console.log(d.toISOString(), d.getTime());
console.log(d.getUTCFullYear(), d.getUTCMonth(), d.getUTCDate(), d.getUTCDay(), d.getUTCHours(), d.getUTCMinutes(), d.getUTCSeconds(), d.getUTCMilliseconds());
const later = new Date(d.getTime() + 2);
console.log(later.toISOString(), later.getUTCFullYear());
console.log(new Date(0).toISOString(), new Date(-1).toISOString(), new Date(86400000 * 365).getUTCFullYear());
console.log(Date.UTC(2000, 1, 29), new Date(Date.UTC(2000, 1, 29)).getUTCDate(), new Date(Date.UTC(1900, 1, 29)).getUTCMonth());
