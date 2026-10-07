import './throw-a.mjs';
globalThis.throwBRuns = (globalThis.throwBRuns || 0) + 1;
throw new Error('b failed');
