import data from './data.json' with { type: 'json' };
import * as ns from './data.json' with { type: 'json' };
export const summary = data.name + ' ' + data.list.length + ' ' + (ns.default === data) + ' ' + Object.keys(ns).join();
