//! Progress reporting and cancellation for long-running operations.

use std::sync::atomic::{AtomicBool, Ordering};

pub trait Reporter: Sync {
    /// `done` of `total` steps finished; `current` describes the step.
    fn progress(&self, done: usize, total: usize, current: &str);
    fn log(&self, line: String);
    fn cancelled(&self) -> bool {
        false
    }
}

/// Reporter that ignores everything (for tests and scripts).
pub struct Silent;

impl Reporter for Silent {
    fn progress(&self, _: usize, _: usize, _: &str) {}
    fn log(&self, _: String) {}
}

/// Reporter backed by closures and a cancel flag.
pub struct FnReporter<'a, P, L>
where
    P: Fn(usize, usize, &str) + Sync,
    L: Fn(String) + Sync,
{
    pub on_progress: P,
    pub on_log: L,
    pub cancel: &'a AtomicBool,
}

impl<P, L> Reporter for FnReporter<'_, P, L>
where
    P: Fn(usize, usize, &str) + Sync,
    L: Fn(String) + Sync,
{
    fn progress(&self, done: usize, total: usize, current: &str) {
        (self.on_progress)(done, total, current)
    }
    fn log(&self, line: String) {
        (self.on_log)(line)
    }
    fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }
}
