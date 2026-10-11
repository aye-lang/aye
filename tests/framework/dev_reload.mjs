// Plays the browser for the dev reload of `web dev` (docs/framework.md
// section 5): fetches `/` from the server, evaluates the page's
// `/_web/dev.js` in a fake `window`, `document` and `location`, and follows
// `/_web/events` with an SSE client. `location.reload()` loads `/` again
// (and runs the script again, as a browser does).
//
// Usage: node dev_reload.mjs <url> [reloads] [idle seconds]
//
// Prints `connected` when the script has read the first build id of a page
// load, and `reloads=N` after each reload. Exits 0 after `reloads` reloads
// (default 2), 1 when nothing happens for the idle time (default 20 s).
//
// Node 22 has no global `EventSource` without a flag, so a small client
// over `fetch` stands in: it reconnects 200 ms after the stream ends or the
// connection fails, as a browser does.

import vm from "node:vm";

const base = new URL(process.argv[2]);
const want = Number(process.argv[3] ?? 2);
const idleMs = Number(process.argv[4] ?? 20) * 1000;

let reloads = 0;
let open = [];
let timer;

function progress() {
  clearTimeout(timer);
  timer = setTimeout(() => {
    console.error(`no progress for ${idleMs / 1000} s (reloads=${reloads})`);
    process.exit(1);
  }, idleMs);
}

function say(line) {
  console.log(line);
  progress();
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/** A server-sent events client with the part of `EventSource` that dev.js uses. */
class EventSource {
  constructor(url) {
    this.url = new URL(url, base);
    this.listeners = {};
    this.closed = false;
    this.seen = false;
    open.push(this);
    this.run();
  }

  addEventListener(name, fn) {
    (this.listeners[name] ??= []).push(fn);
  }

  close() {
    this.closed = true;
  }

  dispatch(name, data) {
    for (const fn of this.listeners[name] ?? []) {
      fn({ type: name, data });
    }
    if (name === "build" && !this.seen) {
      this.seen = true;
      say("connected");
    }
  }

  async run() {
    while (!this.closed) {
      try {
        const res = await fetch(this.url, {
          headers: { Accept: "text/event-stream" },
        });
        const decoder = new TextDecoder();
        let buf = "";
        for await (const chunk of res.body) {
          if (this.closed) return;
          buf += decoder.decode(chunk, { stream: true }).replaceAll("\r\n", "\n");
          let end;
          while ((end = buf.indexOf("\n\n")) >= 0) {
            const frame = buf.slice(0, end);
            buf = buf.slice(end + 2);
            let name = "message";
            const data = [];
            for (const line of frame.split("\n")) {
              if (line.startsWith("event:")) name = line.slice(6).trim();
              else if (line.startsWith("data:")) data.push(line.slice(5).trim());
            }
            this.dispatch(name, data.join("\n"));
          }
        }
      } catch (e) {
        // The server is down or restarting: try again.
      }
      await sleep(200);
    }
  }
}

/** `GET path`, retried while the server does not answer. */
async function get(path) {
  for (;;) {
    try {
      const res = await fetch(new URL(path, base));
      return { status: res.status, text: await res.text() };
    } catch (e) {
      await sleep(200);
    }
  }
}

/** Loads `/`, and runs the page's dev script in a fresh fake window. */
async function load() {
  for (const source of open) source.close();
  open = [];
  const page = await get("/");
  const tag = page.text.match(/<script[^>]*\ssrc="([^"]*\/_web\/dev\.js)"/);
  if (!tag) {
    console.error(`no dev.js script in the page (status ${page.status})`);
    process.exit(1);
  }
  const script = await get(tag[1]);
  const location = {
    href: base.href,
    origin: base.origin,
    reload() {
      reloads += 1;
      say(`reloads=${reloads}`);
      if (reloads >= want) process.exit(0);
      load();
    },
  };
  const window = { location, EventSource };
  const document = { readyState: "complete", addEventListener() {} };
  vm.runInNewContext(script.text, { window, document, location, EventSource });
}

progress();
await load();
