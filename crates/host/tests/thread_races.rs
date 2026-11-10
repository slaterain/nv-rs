//! The thread-race row installed (`host::thread_races::install`), with real OS threads meeting in
//! the scrap-heap map and at the face-gen lock (`support/thread_races.rs`): nothing is lost.

#[path = "support/thread_races.rs"]
mod support;

use std::time::Duration;

#[test]
fn threads_meeting_lose_nothing_with_the_row() {
    let Some(counts) = support::run(true, 20_000, 100_000, Duration::from_secs(20)) else {
        return;
    };
    assert!(counts.rounds > 0);
    assert_eq!(counts.lost, 0, "rounds that lost a scrap heap with the row");
    assert_eq!(counts.depth, 0, "the face-gen depth with the row");
}
