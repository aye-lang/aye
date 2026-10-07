// A named default function is hoisted and bound under its own name too.
export const hoisted = typeof callMe === 'function' && callMe();
export default function callMe() { return 'named'; }
callMe = 'reassigned';
