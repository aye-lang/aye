// Two modules export their own namespace import of x: two bindings, so
// the name is ambiguous.
export * from './ns-1.mjs';
export * from './ns-2.mjs';
