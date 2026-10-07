// Microtasks queued by sync modules run after them; a TLA module resumes
// in a later job.
import './order-sync.mjs';
import './order-async.mjs';
log('order body');
