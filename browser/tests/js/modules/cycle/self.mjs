// A module that imports itself.
import * as me from './self.mjs';
import { value as mine } from './self.mjs';
export function read() { return mine; }
export const value = 'self value';
export const same = () => me.value === mine;
