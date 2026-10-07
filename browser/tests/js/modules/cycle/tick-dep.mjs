Promise.resolve().then(() => log('tick-dep microtask'));
log('tick-dep body');
