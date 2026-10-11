// Lists the imports and the exports of a wasm module, one sorted line each:
// `import env.dom_query` and `export mount`.
//
// Usage: node island_exports.mjs <file.wasm>

import { readFile } from "node:fs/promises";

const module = await WebAssembly.compile(await readFile(process.argv[2]));
const lines = [
  ...WebAssembly.Module.imports(module).map((i) => `import ${i.module}.${i.name}`),
  ...WebAssembly.Module.exports(module).map((e) => `export ${e.name}`),
];
console.log(lines.sort().join("\n"));
