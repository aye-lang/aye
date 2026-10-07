globalThis.sharedRuns = (globalThis.sharedRuns || 0) + 1;
log('shared start');
for (let i = 0; i < 5; i++) await null;
export const s = 'shared';
log('shared end');
