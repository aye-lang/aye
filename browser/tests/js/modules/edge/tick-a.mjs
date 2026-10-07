// Two import() calls of different roots that share a module with
// top-level await: the shared module runs once, both wait for it.
import { s } from './tick-shared.mjs';
log('tick-a', s);
