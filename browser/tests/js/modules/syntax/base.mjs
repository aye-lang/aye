export class Base { who() { return 'base'; } }
export const a = 1, b = 2;
export let late;
late = 'assigned later';
export async function* agen() { yield 1; yield 2; }
const c = 3, d = 4;
export { c as default, d as "d name", };
export {};
