// Every re-export form. `dup` is ambiguous (x and y), `same` is not (both
// are x's x), and `export *` never re-exports a default.
export * from './x.mjs';
export * from './y.mjs';
export * from './z.mjs';
export * from './x.mjs';
export { y as renamed, default as yDefault } from './y.mjs';
export * as sub from './y.mjs';
export { default } from './x.mjs';
import * as self from './all.mjs';
export { self };
const local = 'local';
export { local, local as "string name" };
