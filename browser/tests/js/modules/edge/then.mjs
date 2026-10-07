// A namespace with a `then` export is a thenable: import() resolves
// with what `then` gives.
export function then(resolve) { resolve('from then'); }
