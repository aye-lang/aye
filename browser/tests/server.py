# A local HTTP server for the browser's tests (tests/browser.rs). It
# serves the directory given as its argument on an ephemeral port of
# 127.0.0.1, prints the port on the first line of stdout, and adds a few
# routes that a file server cannot produce:
#
#   /redirect        302 to /basic.html
#   /redirect-style  302 to /style/link.html (relative URLs resolve
#                    against the final URL)
#   /redirect-loop   302 to itself
#   /agent           the request's User-Agent, as text/plain
#   /gzip            an HTML page, gzip-encoded when the client accepts it
#   /latin1          an HTML page in ISO-8859-1, declared by Content-Type
#   /echo            (GET and POST) an HTML page that shows the request:
#                    its method, path, Content-Type, the fields of the
#                    query or of the body (urlencoded, multipart or
#                    text/plain) as "name = value" items, and the raw
#                    body (CR written as \r)
#   /echo-redirect   (POST) 303 to /echo?redirected=1
import gzip
import html
import http.server
import sys
import urllib.parse

ROOT = sys.argv[1]


class Handler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=ROOT, **kwargs)

    def log_message(self, *args):
        pass

    def reply(self, status, headers, body=b""):
        self.send_response(status)
        for name, value in headers:
            self.send_header(name, value)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        path = self.path
        if path == "/redirect":
            self.reply(302, [("Location", "/basic.html")])
        elif path == "/redirect-style":
            self.reply(302, [("Location", "/style/link.html")])
        elif path == "/redirect-loop":
            self.reply(302, [("Location", "/redirect-loop")])
        elif path == "/agent":
            agent = self.headers.get("User-Agent", "")
            self.reply(200, [("Content-Type", "text/plain")], agent.encode() + b"\n")
        elif path == "/gzip":
            body = b"<p>" + b"compressed " * 50 + b"</p>"
            headers = [("Content-Type", "text/html; charset=utf-8")]
            if "gzip" in self.headers.get("Accept-Encoding", ""):
                body = gzip.compress(body)
                headers.append(("Content-Encoding", "gzip"))
            self.reply(200, headers, body)
        elif path == "/latin1":
            body = "<p>Café à la crème</p>".encode("latin-1")
            self.reply(200, [("Content-Type", "text/html; charset=ISO-8859-1")], body)
        elif path == "/echo" or path.startswith("/echo?"):
            self.echo("GET", b"")
        else:
            super().do_GET()

    def do_POST(self):
        length = int(self.headers.get("Content-Length", "0") or "0")
        body = self.rfile.read(length)
        if self.path == "/echo-redirect":
            self.reply(303, [("Location", "/echo?redirected=1")])
        elif self.path == "/echo" or self.path.startswith("/echo?"):
            self.echo("POST", body)
        else:
            self.reply(404, [("Content-Type", "text/plain")], b"not found\n")

    def echo(self, method, body):
        query = urllib.parse.urlsplit(self.path).query
        ctype = self.headers.get("Content-Type", "")
        fields = urllib.parse.parse_qsl(query, keep_blank_values=True)
        if method == "POST":
            fields = form_fields(ctype, body)
        items = "".join(
            "<li>%s = %s</li>" % (html.escape(k), html.escape(v)) for k, v in fields
        )
        raw = body.decode("utf-8", "replace").replace("\r", "\\r")
        page = (
            "<!doctype html><title>Echo</title><h1>%s %s</h1>"
            "<p>Content-Type: %s</p><ul>%s</ul><pre>%s</pre>"
            % (method, html.escape(self.path), html.escape(ctype), items, html.escape(raw))
        )
        self.reply(200, [("Content-Type", "text/html; charset=utf-8")], page.encode())


def form_fields(ctype, body):
    """The (name, value) pairs of a form body."""
    kind = ctype.split(";")[0].strip().lower()
    if kind == "application/x-www-form-urlencoded":
        return urllib.parse.parse_qsl(body.decode("utf-8", "replace"), keep_blank_values=True)
    if kind == "text/plain":
        pairs = []
        for line in body.decode("utf-8", "replace").split("\r\n"):
            if line:
                name, _, value = line.partition("=")
                pairs.append((name, value))
        return pairs
    if kind == "multipart/form-data":
        boundary = ctype.split("boundary=", 1)[1].strip().encode()
        pairs = []
        for part in body.split(b"--" + boundary)[1:]:
            if part.startswith(b"--"):
                break
            head, _, data = part.partition(b"\r\n\r\n")
            disposition = head.decode("utf-8", "replace")
            name = disposition.split('name="', 1)[1].split('"', 1)[0]
            if 'filename="' in disposition:
                name += ' (file "%s")' % disposition.split('filename="', 1)[1].split('"', 1)[0]
            pairs.append((name, data[:-2].decode("utf-8", "replace")))
        return pairs
    return []


server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
print(server.server_address[1], flush=True)
server.serve_forever()
