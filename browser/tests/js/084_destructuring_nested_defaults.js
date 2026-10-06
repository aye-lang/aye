const config = { server: { host: 'localhost', ports: [80, 443] }, debug: undefined };
const {
  server: { host, ports: [http, https, extra = 8080] },
  debug = false,
  missing: { deep = 'fallback' } = {},
} = config;
console.log(host, http, https, extra, debug, deep);
let calls = 0;
const def = () => { calls++; return 'computed'; };
const { v1 = def(), v2 = def() } = { v1: 'given' };
console.log(v1, v2, calls);
const [{ a = 1 } = {}, [b = 2] = []] = [];
console.log(a, b);
const { a: { b: { c: cc = 'cdef' } = {} } = {} } = { a: { b: {} } };
console.log(cc);
