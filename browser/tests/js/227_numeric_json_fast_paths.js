var fractions = [0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 0.875, -0.125, -0.5, 999999999999.125, 1000000000000.125];
console.log(fractions.map(x => String(x)).join(","));
console.log(JSON.stringify(fractions));
var decimals = ["-0", "0", "0.000000000000001", "-123.456789", "999999999999999", "123456789012345.6", "1e-10", "1e309", "9007199254740993"];
for (var i = 0; i < decimals.length; i++) {
  var n = JSON.parse(decimals[i]);
  console.log(String(n), Object.is(n, -0));
}
var text = '{"plain":"hello 世界","escape":"a\\nb\\tc\\u0000","surrogate":"\\ud800","pair":"\\ud83d\\ude00","slash":"\\\\\\\""}';
console.log(JSON.stringify(JSON.parse(text)));
var nums = [-0, 0, 1, -1, 2147483647, -2147483648, 4294967295, 4294967296, 1.9, -1.9, NaN, Infinity];
console.log(nums.map(x => x >>> 0).join(","));
console.log(nums.map(x => x | 0).join(","));
console.log(Object.is(-4 % 2, -0), Object.is(-0 % 3, -0), -2147483648 % -1, -7 % 3, 7 % -3, 1 % 0);
