// A cycle with top-level await whose second module rejects.
import './tla-throw-b.mjs';
log('never a');
