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
import gzip
import http.server
import sys

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
        else:
            super().do_GET()


server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
print(server.server_address[1], flush=True)
server.serve_forever()
