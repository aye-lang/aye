// Runs a wasm32-freestanding Aye program (`aye run` and the goldens with
// `--target wasm32-freestanding`):
//
//     node tests/wasm/loader.mjs prog.wasm            # calls aye_main, if exported
//     node tests/wasm/loader.mjs prog.wasm add 2 3    # calls the export add(2, 3), prints 5
//
// The imports of the module are `env.aye_write(fd, ptr, len)` (bytes to
// stdout for fd 1, else to stderr) and `env.aye_abort()` (exit status
// 134, like the SIGABRT of a native program, after the panic message was
// written). Any other import is a function that throws when called. The
// exit status is the result of `aye_main` (nonzero when `main` returned an
// error); a trap (`exit`, after flushing) exits 1.
import { readFile } from 'node:fs/promises';
import { writeSync } from 'node:fs';

class Abort extends Error {}

// Under `node -e` the arguments start at argv[1], else at argv[2].
const argv = process.argv.slice(import.meta.url.endsWith('loader.mjs') ? 2 : 1);
const [file, name, ...args] = argv;

let memory;
const write = (fd, ptr, len) => {
  const bytes = new Uint8Array(memory.buffer, ptr, len);
  for (let done = 0; done < len; ) done += writeSync(fd === 2 ? 2 : 1, bytes, done);
};
const provided = {
  aye_write: write,
  aye_abort() {
    throw new Abort();
  },
};

try {
  const module = await WebAssembly.compile(await readFile(file));
  const env = {};
  for (const { module: m, name: n, kind } of WebAssembly.Module.imports(module)) {
    if (m === 'env' && kind === 'function') {
      env[n] =
        provided[n] ??
        (() => {
          throw new Error(`unresolved import env.${n}`);
        });
    }
  }
  const { exports } = await WebAssembly.instantiate(module, { env });
  memory = exports.memory;
  if (name !== undefined) {
    const result = exports[name](...args.map(Number));
    if (result !== undefined) console.log(String(result));
  } else if (exports.aye_main) {
    process.exitCode = exports.aye_main(0, 0);
  }
} catch (e) {
  if (e instanceof Abort) {
    process.exitCode = 134;
  } else if (e instanceof WebAssembly.RuntimeError && e.message === 'unreachable') {
    process.exitCode = 1;
  } else {
    console.error(e);
    process.exitCode = 1;
  }
}
