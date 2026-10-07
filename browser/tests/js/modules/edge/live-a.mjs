// Live bindings across a cycle: b changes a's binding through a's own
// function, and a sees b's later assignments.
import { bump, bVal, setB } from './live-b.mjs';
export let n = 0;
export function inc() { n++; }
bump();
setB('set by a');
log('live a', n, bVal);
