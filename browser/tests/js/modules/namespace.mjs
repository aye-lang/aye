import label, * as counter from './counter.mjs';
export const sameLabel = label === counter.default;
export function get() { return counter.count; }
