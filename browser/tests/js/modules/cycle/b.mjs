import { fa, a } from './a.mjs';
import def from './a.mjs';
import * as nsA from './a.mjs';
log('b body:', fa(), def.name, def());
let seen;
try { seen = a; } catch (e) { seen = e.name; }
log('b reads a before it ran:', seen);
try { seen = nsA.a; } catch (e) { seen = e.name; }
log('b reads nsA.a before it ran:', seen, Reflect.ownKeys(nsA).length);
try { Object.keys(nsA); } catch (e) { log('keys of a namespace with a TDZ export:', e.name); }
try { Object.getOwnPropertyDescriptor(nsA, 'a'); } catch (e) { log('descriptor of a TDZ export:', e.name); }
export var bv = 'B-var';
export function useA() { return a; }
export default 'b default';
