//! Shared by the test binaries that run cases on several threads
//! (`golden`, `conformance`, `browser`); each has `mod common;`.

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

/// How many worker threads a test binary uses: `AYE_TEST_JOBS` (a
/// positive integer) or, when it is unset, `available_parallelism()`.
pub fn jobs() -> usize {
    match std::env::var("AYE_TEST_JOBS") {
        Ok(v) => match v.parse::<usize>() {
            Ok(n) if n > 0 => n,
            _ => panic!("AYE_TEST_JOBS must be a positive integer, got {v:?}"),
        },
        Err(_) => std::thread::available_parallelism().map_or(1, |n| n.get()),
    }
}

/// `f` on every item, on at most `jobs()` threads that each take the next
/// item when they finish one; the results are in the order of `items`.
#[allow(dead_code)]
pub fn run_queue<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let next = AtomicUsize::new(0);
    let done = Mutex::new(Vec::with_capacity(items.len()));
    std::thread::scope(|s| {
        for _ in 0..jobs().min(items.len()) {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(item) = items.get(i) else { break };
                    let r = f(item);
                    done.lock().unwrap().push((i, r));
                }
            });
        }
    });
    let mut done = done.into_inner().unwrap();
    done.sort_by_key(|d| d.0);
    done.into_iter().map(|(_, r)| r).collect()
}
