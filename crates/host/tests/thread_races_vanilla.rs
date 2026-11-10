//! The original (no row) with real OS threads meeting in the scrap-heap map and at the face-gen lock
//! (`support/thread_races.rs`): how often the races strike, reported and not required (a race need
//! not strike in a given run). Its own test binary: the helpers' pool is one per process. It runs
//! the original's race on purpose: if it ever flakes on CI, mark it ignored (the review of PR 414).

#[path = "support/thread_races.rs"]
mod support;

use std::time::Duration;

#[test]
fn threads_meeting_in_the_original_are_counted() {
    if let Some(counts) = support::run(false, 20_000, 100_000, Duration::from_secs(20)) {
        assert!(counts.rounds > 0);
    }
}
