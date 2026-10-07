import d, { K, c, f } from './tdz-a.mjs';
import * as ns from './tdz-a.mjs';
function attempt(read) { try { return typeof read(); } catch (e) { return e.name; } }
export const probe = [attempt(() => d), attempt(() => K), attempt(() => c), attempt(() => f),
  'c' in ns, attempt(() => ns.c), attempt(() => ns.default)].join(' ');
export function later() { return [d, K.name, c, f()].join(' '); }
