const s = 'JavaScript';
console.log(s.slice(4), s.slice(0, 4), s.slice(-6), s.slice(-6, -3), s.slice(5, 2), s.slice(100));
console.log(s.substring(4), s.substring(0, 4), s.substring(4, 0), s.substring(-3, 2), s.substring(NaN, 3));
console.log(s.substr(4), s.substr(0, 4), s.substr(-6, 3), s.substr(2, 0), s.substr(20, 2) === '');
console.log(s.slice(), s.slice(2, undefined), s.substring(undefined, 4));
console.log(s.slice(1.7, 4.2), s.substring('2', '5'));
