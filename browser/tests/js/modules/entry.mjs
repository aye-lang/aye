const namespace = await import('data:text/javascript,export const value=42;');
console.log('entry', namespace.value, import.meta.url.endsWith('/modules/entry.mjs'));
