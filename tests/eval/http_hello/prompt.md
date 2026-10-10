Write `hello`, an HTTP server, in the file `main.aye`.

`hello <port>` listens on `127.0.0.1:<port>` and serves HTTP/1.1 until it
is killed:

- `GET /hello` answers status 200 with the body `hello, world\n`.
- `GET /hello?name=<name>` answers status 200 with the body
  `hello, <name>\n` (`<name>` is letters and digits only; no decoding is
  needed).
- Any other path answers status 404 with the body `not found\n`.
- Every response has the header `Content-Type: text/plain`.

```
$ hello 8080 &
$ curl -s 'http://127.0.0.1:8080/hello?name=Ann'
hello, Ann
```

Note: here you cannot run `curl`, and `aye run` of a server does not
return, so do not start the server to try it.
