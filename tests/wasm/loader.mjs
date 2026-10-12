// Runs a wasm32-freestanding Aye program (`aye run` and the goldens with
// `--target wasm32-freestanding`):
//
//     node tests/wasm/loader.mjs prog.wasm            # calls aye_main, if exported
//     node tests/wasm/loader.mjs prog.wasm add 2 3    # calls the export add(2, 3), prints 5
//
// The imports of the module are `env.aye_write(fd, ptr, len)` (bytes to
// stdout for fd 1, else to stderr) and `env.aye_abort()` (exit status
// 134, like the SIGABRT of a native program, after the panic message was
// written), and these, by their libc or runtime names:
//
//     aye_clock(id) -> f64          milliseconds: id 0 Date.now(), 1 performance.now()
//     aye_fmt_double(ptr, cap, x, n) -> len   x.toExponential(n - 1) and a NUL (std.json, ecvt)
//     strtod(ptr, end) -> f64       the longest numeric prefix of a C string (inf, nan too);
//                                   `*end` is set unless `end` is 0
//     sin cos tan asin acos atan atan2 sinh cosh tanh asinh acosh atanh
//     exp expm1 log log1p log2 log10 pow cbrt hypot fmod   from Math (C's pow and fmod
//                                   edge cases: pow(1, y) and pow(x, 0) are 1)
//
// `sqrt`, `floor`, `ceil`, `trunc`, `fabs`, `round` and `copysign` are
// wasm instructions in the runtime. Any other import is a function that
// throws when called. The exit status is the result of `aye_main`
// (nonzero when `main` returned an error); a trap (`exit`, after
// flushing) exits 1. When V8 runs out of its own call stack (a RangeError,
// before the program's stack is used up), the native stack overflow panic
// message is printed and the exit status is 134.
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
const text = (ptr) => {
  const bytes = new Uint8Array(memory.buffer);
  let end = ptr;
  while (bytes[end]) end++;
  return Buffer.from(bytes.subarray(ptr, end)).toString('latin1');
};
const NUMBER = /^[ \t\n\v\f\r]*([+-]?(?:inf(?:inity)?|nan|(?:\d+\.?\d*|\.\d+)(?:e[+-]?\d+)?))/i;
const strtod = (ptr, end) => {
  const m = NUMBER.exec(text(ptr));
  if (end) new DataView(memory.buffer).setUint32(end, ptr + (m ? m[0].length : 0), true);
  if (!m) return 0;
  const t = m[1].toLowerCase().replace(/^\+/, '');
  if (t.endsWith('nan')) return NaN;
  return t.endsWith('inf') || t.endsWith('infinity') ? (t[0] === '-' ? -Infinity : Infinity) : Number(t);
};
const provided = {
  aye_write: write,
  aye_abort() {
    throw new Abort();
  },
  aye_clock: (id) => (id === 0 ? Date.now() : performance.now()),
  aye_fmt_double(ptr, cap, x, n) {
    const bytes = Buffer.from(x.toExponential(n - 1) + '\0');
    new Uint8Array(memory.buffer).set(bytes.subarray(0, cap), ptr);
    return Math.min(bytes.length - 1, cap - 1);
  },
  strtod,
  pow: (x, y) => (x === 1 || y === 0 || (x === -1 && Math.abs(y) === Infinity) ? 1 : Math.pow(x, y)),
  fmod: (x, y) => x % y,
  ...Object.fromEntries(
    'sin cos tan asin acos atan atan2 sinh cosh tanh asinh acosh atanh exp expm1 log log1p log2 log10 cbrt hypot'
      .split(' ')
      .map((n) => [n, Math[n]]),
  ),
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
  } else if (e instanceof RangeError && e.message === 'Maximum call stack size exceeded') {
    process.stderr.write('panic: stack overflow (deep recursion: run it on a thread with a larger stack, std.thread.spawnWithStack)\n');
    process.exitCode = 134;
  } else {
    console.error(e);
    process.exitCode = 1;
  }
}
