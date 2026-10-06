const v = JSON.parse('{"a":[1,2,{"b":null}],"c":"str","d":true,"e":-1.5e2,"f":{}}');
console.log(v, v.a[2].b, v.e);
console.log(JSON.parse('"\\u00e9\\u0041\\n\\ud83d\\ude00"'), JSON.parse('"\\/\\\\"'));
console.log(JSON.parse(' [ 1 , 2 ] '), JSON.parse('123'), JSON.parse('"x"'), JSON.parse('null'));
console.log(JSON.parse('{"__proto__": 1}').__proto__ === 1 || 'proto-handled');
console.log(JSON.parse('{"a":1,"a":2}'), JSON.parse('1E3'), JSON.parse('-0.5'));
console.log(Object.keys(JSON.parse('{"z":1,"10":2,"a":3,"2":4}')));
console.log(JSON.parse('[[[[[[1]]]]]]')[0][0][0][0][0][0]);
