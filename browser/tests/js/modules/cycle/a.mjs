// The entry of a two-module cycle: b runs first and calls back into a.
import { bv, useA } from './b.mjs';
import defaultFromB from './b.mjs';
log('a body: bv =', bv, '; b default =', defaultFromB);
export function fa() { return 'fa sees bv=' + bv; }
export let a = 'A';
export default function namedDefault() { return 'default fn'; }
log('a done:', useA(), fa());
