import { a } from './cycle-a.mjs';
export const b = 'b sees ' + a();
document.getElementById('cycle').textContent = b;
