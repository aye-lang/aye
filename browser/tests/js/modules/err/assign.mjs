import { v, setV } from './plain.mjs';
import * as ns from './plain.mjs';
try { v = 2; } catch (e) { log('assign import', e.name); }
try { v++; } catch (e) { log('update import', e.name); }
try { ns.v = 2; } catch (e) { log('assign namespace', e.name); }
try { delete ns.v; } catch (e) { log('delete namespace', e.name); }
const reads = [];
for (let i = 0; i < 3; i++) { setV(i * 10); reads.push(v + 1, typeof v); }
log('live reads', reads.join(), (() => v)(), ns.v);
log('meta', typeof import.meta, Object.getPrototypeOf(import.meta), import.meta.url.endsWith('/err/assign.mjs'));
log('resolve', import.meta.resolve('./plain.mjs').endsWith('/err/plain.mjs'));
log('this', this, typeof arguments);
