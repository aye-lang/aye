function Foo() {}
const f = new Foo();
console.log(f instanceof Foo, f instanceof Object, [] instanceof Array, [] instanceof Object);
console.log({} instanceof Array, (() => 1) instanceof Function, new Date(0) instanceof Date);
class A {}
class B extends A {}
console.log(new B() instanceof A, new A() instanceof B);
const obj = { a: 1, b: undefined };
console.log('a' in obj, 'b' in obj, 'c' in obj, 'toString' in obj);
const arr = [10, 20];
console.log(0 in arr, 2 in arr, 'length' in arr, '1' in arr);
console.log('x' in Object.create({ x: 1 }));
