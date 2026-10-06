// Shallow matches on long subjects; deep preferred paths must retry.
var long = "x".repeat(4000);
console.log(/x/.exec(long).index, long.match(/x/g).length);
console.log(/(x+)|(x)/.exec(long)[0].length);
console.log(/x+?$/.exec(long)[0].length);
console.log(long.replace(/x/g, "y").length);
console.log(/(?=(x+))x/.exec(long)[1].length);
console.log(/z/.test(long), /x+$/.test(long));
