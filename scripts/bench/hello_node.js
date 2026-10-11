// Benchmark reference (O7.1): Node `http` hello world on port 8082.
const http = require('http');
http.createServer((req, res) => {
  res.writeHead(200, { 'Content-Type': 'text/plain' });
  res.end('hello\n');
}).listen(8082, '127.0.0.1');
