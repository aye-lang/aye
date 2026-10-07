globalThis.throwRuns = (globalThis.throwRuns || 0) + 1;
throw new Error('boom');
