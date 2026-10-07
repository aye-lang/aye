import { n, inc } from './live-a.mjs';
export let bVal = 'initial';
export function bump() { inc(); inc(); }
export function setB(v) { bVal = v; }
export function readN() { return n; }
