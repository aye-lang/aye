// The entry of a cycle whose exports b reads before a runs: a default
// expression, a class and a const are in their TDZ, a function is not.
import { probe } from './tdz-b.mjs';
export default 6 * 7;
export class K {}
export const c = 'c';
export function f() { return 'f'; }
log('a runs; b saw', probe);
