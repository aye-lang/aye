const d = new Date('2020-01-02T03:04:05.006Z');
console.log(d.getTime(), d.toISOString(), JSON.stringify({ when: d }));
console.log(new Date('2020-06-01T00:00:00Z').getUTCDay(), Date.parse('2001-09-09T01:46:40Z'));
console.log(isNaN(new Date('not a date').getTime()), String(new Date(NaN)));
console.log(JSON.stringify([new Date(NaN)]), new Date(2e15).toISOString() === undefined);
