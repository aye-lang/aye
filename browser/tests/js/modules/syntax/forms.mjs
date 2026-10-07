import {} from './base.mjs';
import def, {} from './base.mjs';
import base, * as all from './base.mjs';
import { a, b, } from './base.mjs';
import { "d name" as dName, Base } from './base.mjs';
export * as "every thing" from './base.mjs';
export { default as cDefault } from './base.mjs';
export default class extends Base { who() { return 'derived of ' + super.who(); } }
const got = [];
for await (const v of all.agen()) got.push(v);
export const summary = [def, base, a, b, dName, all.late, got.join('+')].join(' ');
