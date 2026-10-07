// A local export hides the same name of a star export, without ambiguity.
export * from '../re/x.mjs';
export * from '../re/y.mjs';
export const dup = 'local dup';
