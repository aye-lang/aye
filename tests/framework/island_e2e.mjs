// Plays the browser for an island of a running app (docs/framework.md FD6):
// fetches `GET <path>` (`/counter` unless given), reads the `[data-island]`
// div (name, props and inner markup) and the `/_web/islands.js` script tag,
// fetches the script, and runs it in a `vm` sandbox with a fake `document`
// built from that div and the real `fetch` for `/_web/islands/<name>.wasm`.
// Then it clicks the island's button twice.
//
// Usage: node island_e2e.mjs <url> [<path>]
//
// Prints `text=<text of the count>` (the island's `span`, or its button when
// it has none) and exits 0 when the page, the script and the module come
// from the server and the clicks work, 1 with the reason otherwise. Against
// the pages example the text is `Count: 5` (the island starts at 3); against
// `/posts/hello` of the posts example it is `♥ 2`.

import vm from "node:vm";

const base = new URL(process.argv[2]);
const pagePath = process.argv[3] ?? "/counter";

function fail(reason) {
  console.error(reason);
  process.exit(1);
}

/** The text of an HTML attribute value, with its entities decoded. */
function unescape(text) {
  return text
    .replaceAll("&lt;", "<")
    .replaceAll("&gt;", ">")
    .replaceAll("&quot;", '"')
    .replaceAll("&#39;", "'")
    .replaceAll("&amp;", "&");
}

/** The part of an element that the glue and the island use. */
class Element {
  constructor(tag, attrs = {}, children = [], text = "") {
    this.tag = tag;
    this.attrs = attrs;
    this.children = children;
    this.textContent = text;
    this.listeners = {};
  }

  getAttribute(name) {
    return this.attrs[name] ?? null;
  }

  querySelector(sel) {
    for (const child of this.children) {
      if (child.tag === sel) return child;
      const found = child.querySelector(sel);
      if (found) return found;
    }
    return null;
  }

  addEventListener(name, fn) {
    (this.listeners[name] ??= []).push(fn);
  }

  click() {
    for (const fn of this.listeners.click ?? []) fn({ type: "click" });
  }
}

/** The elements of `inner`, which is `<tag>text</tag>` pieces side by side. */
function children(inner) {
  const found = [];
  for (const m of inner.matchAll(/<([a-z]+)>([^<]*)<\/\1>/g)) {
    found.push(new Element(m[1], {}, [], unescape(m[2])));
  }
  return found;
}

async function get(path) {
  const res = await fetch(new URL(path, base));
  if (!res.ok) fail(`GET ${path}: status ${res.status}`);
  return res.text();
}

const page = await get(pagePath);
const div = page.match(/<div data-island="([^"]*)" data-props="([^"]*)">(.*?)<\/div>/s);
if (!div) fail("no [data-island] div in the page");
const root = new Element(
  "div",
  { "data-island": unescape(div[1]), "data-props": unescape(div[2]) },
  children(div[3]),
);
const tag = page.match(/<script[^>]*\ssrc="([^"]*\/_web\/islands\.js)"/);
if (!tag) fail("no /_web/islands.js script in the page");
const script = await get(tag[1]);

const errors = [];
const sandbox = {
  document: {
    readyState: "complete",
    addEventListener() {},
    querySelectorAll: (sel) => (sel === "[data-island]" ? [root] : []),
  },
  fetch: (url) => fetch(new URL(url, base)),
  console: { log() {}, error: (...a) => errors.push(a.join(" ")) },
  TextDecoder,
  TextEncoder,
};
vm.runInNewContext(script, sandbox);

const button = root.querySelector("button");
const span = root.querySelector("span") ?? button;
if (!button) fail("the island has no button: " + div[3]);
// The glue is asynchronous: wait for the listener or an error.
for (let i = 0; i < 1000 && !button.listeners.click && errors.length === 0; i++) {
  await new Promise((r) => setTimeout(r, 10));
}
if (!button.listeners.click) fail("the button has no click listener; errors: " + errors.join(" | "));
button.click();
button.click();
if (errors.length > 0) fail("errors: " + errors.join(" | "));
console.log("text=" + span.textContent);
