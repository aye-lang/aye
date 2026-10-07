export const url = import.meta.url.slice(import.meta.url.lastIndexOf('/modules/'));
export const resolved = import.meta.resolve('./x/../y.mjs').slice(import.meta.url.lastIndexOf('/modules/'));
export const proto = Object.getPrototypeOf(import.meta);
