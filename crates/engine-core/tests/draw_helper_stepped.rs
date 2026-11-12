//! The drawing helper with stepped threads (`real_threads::post_draw`; one helper per process, so
//! this binary holds the one stepped run): a posted job does not run until its poster waits for
//! it, so the job's place in the run is the poster's wait and a run stays deterministic.

use engine_core::real_threads::{draw_posted, join_draw, post_draw, wait_draw};
use engine_core::Engine;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

engine_core::assemble!();

#[test]
fn with_stepped_threads_a_job_runs_when_its_poster_waits() {
    let mut e = Engine::new();
    let ran = Arc::new(AtomicU32::new(0));
    let job_ran = ran.clone();
    assert!(post_draw(
        &mut e,
        Box::new(move |_| {
            job_ran.fetch_add(1, Ordering::SeqCst);
        })
    ));
    assert!(draw_posted());
    // However long the poster goes on, the job waits for it.
    for _ in 0..20 {
        std::thread::sleep(std::time::Duration::from_millis(5));
        assert_eq!(
            ran.load(Ordering::SeqCst),
            0,
            "ran before its poster waited"
        );
    }
    assert!(wait_draw(&e));
    assert_eq!(ran.load(Ordering::SeqCst), 1);
    assert!(join_draw(&e));
    assert!(!draw_posted());
    // The next one too.
    let job_ran = ran.clone();
    assert!(post_draw(
        &mut e,
        Box::new(move |_| {
            job_ran.fetch_add(1, Ordering::SeqCst);
        })
    ));
    std::thread::sleep(std::time::Duration::from_millis(20));
    assert_eq!(ran.load(Ordering::SeqCst), 1);
    assert!(join_draw(&e));
    assert_eq!(ran.load(Ordering::SeqCst), 2);
}
