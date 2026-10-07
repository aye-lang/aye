// A cycle whose member awaits: the cycle settles when y is done.
import { y } from './y.mjs';
log('x sees', y);
export const x = 'x';
