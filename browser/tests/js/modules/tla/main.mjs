// c awaits; b does not depend on it and runs at once; a and main wait.
import './a.mjs';
import './b.mjs';
log('main');
