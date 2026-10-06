const vals = [1, 10, 1e5, 1e15, 1e20, 1e21, 1e22, 1.5e21, 0.1, 0.01, 1e-6, 1e-7, 1.5e-7, 123456.789, 0.000123, 98765432109876543210];
for (const v of vals) console.log(String(v), JSON.stringify(v), String(-v));
console.log(1.2345678901234567, 0.30000000000000004, 1.7976931348623157e308);
console.log(4.94e-324, 2.2250738585072014e-308, 1 / 3 * 3);
