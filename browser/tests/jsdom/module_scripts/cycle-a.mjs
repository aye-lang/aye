import { b } from './cycle-b.mjs';
export function a() { return 'a'; }
note('module cycle ' + b);
