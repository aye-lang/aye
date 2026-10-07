import * as counter from './counter.mjs';
export const staticCount = counter.count;
export async function load() { return import('./counter.mjs'); }
export function urlTail() { return import.meta.url.endsWith('/modules/nested.mjs'); }
