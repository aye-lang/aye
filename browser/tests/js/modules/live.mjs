// Dependencies run before any statements, even with imports at the end.
export const loadedFirst = globalThis.moduleEvaluations;
export const initial = count;
increment();
await Promise.resolve();
export const after = count;
export function read() { return count; }
export function metaSame() { return import.meta === import.meta; }
export const topThis = this === undefined;
import label, { count, increment } from './counter.mjs';
export { label };
