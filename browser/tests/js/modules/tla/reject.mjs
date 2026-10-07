globalThis.rejectRuns = (globalThis.rejectRuns || 0) + 1;
await 0;
throw new Error('late');
