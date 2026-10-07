// r1 -> r2 -> r3 -> r1, and r1 -> r3: a ring with a chord.
import { push } from './r2.mjs';
import './r3.mjs';
push('r1');
export { order } from './r3.mjs';
