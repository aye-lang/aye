// Runs a WASI preview 1 module under Node's `node:wasi` (`aye run`,
// `aye test` and the goldens with `--target wasm32-wasip1`):
//
//     node --no-warnings --single-threaded-gc --input-type=module -e "$(cat scripts/wasi-run.mjs)" prog.wasm args...
//
// stdin, stdout and stderr are the process's; the arguments and the
// environment are passed; the current directory (as `.` and by its own
// path) and /tmp are preopened. The exit status is the program's. A trap
// from `abort` (a panic, after its message was printed) exits 134, like
// the SIGABRT of a native program; any other trap prints the error and
// exits 1.
import { readFile } from 'node:fs/promises';
import { WASI } from 'node:wasi';

const [file, ...args] = process.argv.slice(1);
const cwd = process.cwd();
const wasi = new WASI({
  version: 'preview1',
  args: [file, ...args],
  env: process.env,
  preopens: { '.': '.', [cwd]: cwd, '/tmp': '/tmp' },
  returnOnExit: true,
});
try {
  const { instance } = await WebAssembly.instantiate(await readFile(file), wasi.getImportObject());
  process.exitCode = wasi.start(instance);
} catch (e) {
  if (e instanceof WebAssembly.RuntimeError && e.message === 'unreachable') {
    process.exitCode = 134;
  } else {
    console.error(e);
    process.exitCode = 1;
  }
}
