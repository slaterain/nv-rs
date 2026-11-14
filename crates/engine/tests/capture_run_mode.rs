//! The draw family's run-wide capture mode (export E5 of the drawing thread's seam), in a test binary of its own:
//! `capture::set_mode` is process-wide, so the typed tests that rely on the mode being off never run beside it.

use engine::engine_typed::draw::capture::{self, Mode};

/// The run-wide mode overrides `NV_DRAW_THREAD` (unset here: off), each value as set; a thread's own mode wins over
/// it; `None` gives the variable's back.
#[test]
fn the_run_wide_mode_overrides_the_variable_and_a_threads_own_wins() {
    assert_eq!(capture::mode(), Mode::Off);
    for mode in [Mode::On, Mode::Poison, Mode::Off] {
        capture::set_mode(Some(mode));
        assert_eq!(capture::mode(), mode);
    }
    capture::set_mode(Some(Mode::On));
    capture::set_thread_mode(Some(Mode::Off));
    assert_eq!(capture::mode(), Mode::Off);
    assert_eq!(capture::thread_mode(), Some(Mode::Off));
    capture::set_thread_mode(None);
    assert_eq!(capture::mode(), Mode::On);
    capture::set_mode(None);
    assert_eq!(capture::mode(), Mode::Off);
}
