//! Shared by the test binaries that run cases on several threads
//! (`golden`, `conformance`, `browser`); each has `mod common;`.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex};

/// How many worker threads a test binary uses: `AYE_TEST_JOBS` (a
/// positive integer) or, when it is unset, `per_core` threads for each
/// core of `available_parallelism()`. CPU-bound runners pass 1; `golden`
/// passes 2 because the first run of each new executable waits about
/// 0.4 s at no CPU on macOS, so one thread per core leaves cores idle.
pub fn jobs_for(per_core: usize) -> usize {
    match std::env::var("AYE_TEST_JOBS") {
        Ok(v) => match v.parse::<usize>() {
            Ok(n) if n > 0 => n,
            _ => panic!("AYE_TEST_JOBS must be a positive integer, got {v:?}"),
        },
        Err(_) => per_core * std::thread::available_parallelism().map_or(1, |n| n.get()),
    }
}

/// `jobs_for(1)`.
pub fn jobs() -> usize {
    jobs_for(1)
}

/// Items being run by all `run_queue` calls of this process.
static RUNNING: Mutex<usize> = Mutex::new(0);
static FREED: Condvar = Condvar::new();

/// A slot in `RUNNING`, given back on drop (also when `f` panics).
struct Slot;

impl Slot {
    fn take(limit: usize) -> Slot {
        let mut n = RUNNING.lock().unwrap_or_else(|e| e.into_inner());
        while *n >= limit {
            n = FREED.wait(n).unwrap_or_else(|e| e.into_inner());
        }
        *n += 1;
        Slot
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        *RUNNING.lock().unwrap_or_else(|e| e.into_inner()) -= 1;
        FREED.notify_one();
    }
}

/// `f` on every item, on threads that each take the next item when they
/// finish one; the results are in the order of `items`. Calls that run at
/// the same time (libtest runs `#[test]` fns in parallel) share one budget:
/// together they run `f` on at most `jobs()` items at once.
#[allow(dead_code)]
pub fn run_queue<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let next = AtomicUsize::new(0);
    let done = Mutex::new(Vec::with_capacity(items.len()));
    let limit = jobs();
    let limit = &limit;
    std::thread::scope(|s| {
        for _ in 0..(*limit).min(items.len()) {
            s.spawn(|| {
                loop {
                    let slot = Slot::take(*limit);
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(item) = items.get(i) else { break };
                    let r = f(item);
                    drop(slot);
                    done.lock().unwrap().push((i, r));
                }
            });
        }
    });
    let mut done = done.into_inner().unwrap();
    done.sort_by_key(|d| d.0);
    done.into_iter().map(|(_, r)| r).collect()
}
