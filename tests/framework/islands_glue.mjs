// Plays the browser for the islands glue (docs/framework.md FD6): takes the
// text of `islandsScript()` from `framework/web/islands.aye`, runs it in a
// `vm` sandbox with a fake `document` holding the counter island and an
// island whose `.wasm` is missing, and a `fetch` that serves the real
// module. Then it clicks the counter's button twice.
//
// Usage: node islands_glue.mjs <islands.aye> <counter.wasm>
//
// Prints `Count: 5` and exits 0 when the counter starts at `Count: 3`, two
// clicks give `Count: 5`, the missing island logs one `console.error` and
// nothing else is logged. Exits 1 with the reason otherwise.

import { readFile } from "node:fs/promises";
import vm from "node:vm";

const [aye, wasmFile] = process.argv.slice(2);

/** The string literal that `islandsScript` returns, as text. */
function scriptText(source) {
  const m = source.match(/pub fn islandsScript\(\) \[\]u8 \{\n    return ("(?:[^"\\\n]|\\.)*")\n\}/);
  if (!m) fail("no islandsScript in " + aye);
  return JSON.parse(m[1]);
}

function fail(reason) {
  console.error(reason);
  process.exit(1);
}

/** The part of an element that the glue and the island use. */
class Element {
  constructor(tag, attrs = {}, children = []) {
    this.tag = tag;
    this.attrs = attrs;
    this.children = children;
    this.textContent = "";
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

const span = new Element("span");
span.textContent = "Count: 3";
const button = new Element("button");
const counter = new Element("div", { "data-island": "counter", "data-props": '{"start":3}' }, [
  button,
  span,
]);
const missing = new Element("div", { "data-island": "missing", "data-props": "{}" });
const islands = [counter, missing];

const wasm = await readFile(wasmFile);
const fetched = [];
const fakeFetch = async (url) => {
  fetched.push(url);
  if (url === "/_web/islands/counter.wasm") {
    return { ok: true, status: 200, arrayBuffer: async () => wasm.buffer.slice(wasm.byteOffset, wasm.byteOffset + wasm.byteLength) };
  }
  return { ok: false, status: 404, arrayBuffer: async () => new ArrayBuffer(0) };
};

const logged = [];
const errors = [];
const sandbox = {
  document: {
    readyState: "complete",
    addEventListener() {},
    querySelectorAll: (sel) => (sel === "[data-island]" ? islands : []),
  },
  fetch: fakeFetch,
  console: { log: (...a) => logged.push(a.join(" ")), error: (...a) => errors.push(a.join(" ")) },
  TextDecoder,
  TextEncoder,
};
vm.runInNewContext(scriptText(await readFile(aye, "utf8")), sandbox);

// The glue is asynchronous: wait for the listener and the error.
for (let i = 0; i < 500 && !(button.listeners.click && errors.length > 0); i++) {
  await new Promise((r) => setTimeout(r, 10));
}
if (!button.listeners.click) fail("the counter's button has no click listener; errors: " + errors.join(" | "));
if (span.textContent !== "Count: 3") fail("before the clicks the text is " + JSON.stringify(span.textContent));
button.click();
button.click();
if (span.textContent !== "Count: 5") fail("after two clicks the text is " + JSON.stringify(span.textContent));
if (errors.length !== 1 || !errors[0].includes("missing")) {
  fail("expected one console.error about the missing island, got: " + JSON.stringify(errors));
}
if (logged.length > 0) fail("unexpected output: " + JSON.stringify(logged));
if (!fetched.includes("/_web/islands/counter.wasm") || !fetched.includes("/_web/islands/missing.wasm")) {
  fail("fetched " + JSON.stringify(fetched));
}
console.log(span.textContent);
