import { value, count, increment } from './value.mjs';
const same = await import('./sub/../value.mjs');
increment();
document.getElementById('module').textContent =
  'module ' + value + ' live ' + count + ' once ' + moduleRuns +
  ' meta ' + import.meta.url.endsWith('/dynamic/outer.mjs') +
  ' same-meta ' + (import.meta === import.meta);
export async function load() { return import('./value.mjs'); }
