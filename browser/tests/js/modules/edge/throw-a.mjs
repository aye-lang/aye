// A cycle whose second module throws: both modules keep the error.
import './throw-b.mjs';
globalThis.throwARan = true;
