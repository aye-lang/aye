Write `items`, a JSON REST service, in the file `main.aye`. Use `std.http`
for the server and `std.json` for JSON; run `aye doc --signatures std.http`
and `aye doc --signatures std.json` for their exact API.

`items <port>` listens on `127.0.0.1:<port>` and serves HTTP/1.1 until it
is killed. It keeps a list of items in memory, empty at the start. An item
is `{"id": <integer>, "name": "<string>"}`; ids are 1, 2, 3, ... in the
order of creation. The server handles several connections at once, so
guard the shared list.

- `GET /items` answers 200 with `{"items": [<item>, ...]}`, in order of
  creation (`{"items": []}` when there are none).
- `POST /items` with a JSON body `{"name": "<string>"}` appends an item and
  answers 201 with the new item. A body that is not JSON, or has no string
  `name`, answers 400 with `{"error": "bad request"}`.
- `GET /items/<id>` answers 200 with that item, or 404 with
  `{"error": "not found"}` when there is no such id.
- Any other path or method answers 404 with `{"error": "not found"}`.
- Every response has the header `Content-Type: application/json`. The
  name in a response must be valid JSON (escape quotes and backslashes);
  whitespace in the JSON you send is free.

```
$ items 8080 &
$ curl -s -X POST -d '{"name": "apple"}' http://127.0.0.1:8080/items
{"id":1,"name":"apple"}
$ curl -s http://127.0.0.1:8080/items/1
{"id":1,"name":"apple"}
```

Note: here you cannot run `curl`, and `aye run` of a server does not
return, so do not start the server to try it.
