class MyError extends Error {
  constructor(message) { super(message); this.name = 'MyError'; }
}
console.log(new MyError('x') instanceof Error);
throw new MyError('message');
