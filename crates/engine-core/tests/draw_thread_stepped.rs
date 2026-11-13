//! The drawing thread's seam with stepped threads (`engine_core::draw_thread`; the drawing helper is one per
//! process, so this binary holds one run and one test): the world draw posted at the first half's call runs when
//! main reaches its next join point, after the frame's tail, from the packet main cut before the tail; the second
//! half's call is skipped; a join point on the drawing side does nothing; a refused post draws on main in order; a
//! job's stop comes back at the join; the end of the process with a job posted does not hang.

use engine_core::draw_thread::{self, testing, Counts, Hooks, Packet};
use engine_core::real_threads::{self, on_draw_helper};
use engine_core::{Engine, Ret};
use std::cell::Cell;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering::SeqCst};
use std::sync::Mutex;

engine_core::assemble!();

/// A value the draw reads and the frame's tail changes, in guest memory.
const VALUE: u32 = 0x0300_0000;
/// Set: the first half stops (the job's stop).
const STOP: u32 = 0x0300_0004;
/// Set: the first half calls a join point (one reached from the drawing side).
const JOIN_INSIDE: u32 = 0x0300_0008;

/// What the halves did: (half, on the drawing helper, the value it read).
static DRAWN: Mutex<Vec<(u32, bool, u32)>> = Mutex::new(Vec::new());
static CLOSED: AtomicU32 = AtomicU32::new(0);
static JOINED: Mutex<Vec<u32>> = Mutex::new(Vec::new());
/// Set: the device cut posts a job of its own first, so the seam's post is refused.
static BUSY: AtomicBool = AtomicBool::new(false);

thread_local! {
    /// The packet the thread that draws holds (the draw family's thread-local, stood in).
    static HELD: Cell<Option<u32>> = const { Cell::new(None) };
}

/// What the draw reads: the held packet's copy, or guest memory without one.
fn read_value(e: &Engine) -> u32 {
    HELD.with(Cell::get).unwrap_or_else(|| e.mem.u32(VALUE))
}

fn first(e: &mut Engine, _: &[u32]) -> Ret {
    if e.mem.u32(STOP) != 0 {
        panic!("the first half stopped");
    }
    if e.mem.u32(JOIN_INSIDE) != 0 {
        draw_thread::finish(e);
        draw_thread::idle(e);
    }
    let v = read_value(e);
    DRAWN.lock().unwrap().push((1, on_draw_helper(), v));
    Ret::default()
}

fn second(e: &mut Engine, _: &[u32]) -> Ret {
    let v = read_value(e);
    DRAWN.lock().unwrap().push((2, on_draw_helper(), v));
    Ret::default()
}

fn serial(_: &Engine) -> Option<&'static str> {
    None
}

fn cut(e: &mut Engine, _: u32, _: u32, stepped: bool) -> Option<Packet> {
    assert!(stepped);
    assert!(!on_draw_helper(), "the cut is main's");
    Some(Box::new(e.mem.u32(VALUE)))
}

fn adopt(_: &mut Engine, p: Packet) -> Result<(), Packet> {
    if HELD.with(Cell::get).is_some() {
        return Err(p);
    }
    HELD.with(|h| h.set(Some(*p.downcast::<u32>().unwrap())));
    Ok(())
}

fn hand_back(_: &mut Engine) -> Option<Packet> {
    HELD.with(Cell::take).map(|v| Box::new(v) as Packet)
}

fn join(_: &mut Engine, p: Packet) {
    assert!(!on_draw_helper(), "the join is main's");
    JOINED.lock().unwrap().push(*p.downcast::<u32>().unwrap());
}

fn device_cut(e: &mut Engine, _: u32) -> bool {
    if BUSY.load(SeqCst) {
        assert!(real_threads::post_draw(e, Box::new(|_| {})));
    }
    true
}

fn device_close(_: &mut Engine) {
    assert!(on_draw_helper(), "the drawing side closes");
    CLOSED.fetch_add(1, SeqCst);
}

fn device_finish(e: &mut Engine) {
    real_threads::join_draw(e);
}

fn take_drawn() -> Vec<(u32, bool, u32)> {
    std::mem::take(&mut *DRAWN.lock().unwrap())
}

fn take_joined() -> Vec<u32> {
    std::mem::take(&mut *JOINED.lock().unwrap())
}

#[test]
fn the_world_draw_runs_after_the_tail_from_the_packet_cut_before_it() {
    let mut e = Engine::new();
    e.map(VALUE, 0x1000);
    e.register(draw_thread::WORLD_DRAW_FIRST, first);
    e.register(draw_thread::WORLD_DRAW_SECOND, second);
    draw_thread::turn_on(
        &e,
        Hooks {
            serial,
            cut,
            adopt,
            hand_back,
            join,
            device_cut,
            device_close,
            device_finish,
        },
    );

    // Frames: the cut sees 10 + frame, the tail then writes 99; the draw must see the cut's value.
    for frame in 0..3u32 {
        e.mem.set_u32(VALUE, 10 + frame);
        draw_thread::first_half(&mut e, 0x6000, 0x8800, 1, 0);
        assert!(draw_thread::posted(&e));
        // Stepped threads: the job waits for main's next join point however long the tail takes.
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(
            take_drawn().is_empty(),
            "frame {frame}: drawn before the join"
        );
        draw_thread::second_half(&mut e, 0x6000, 0x8800, 1);
        e.mem.set_u32(VALUE, 99);
        assert!(
            take_drawn().is_empty(),
            "frame {frame}: the second half's call was skipped"
        );
        draw_thread::idle(&mut e);
        assert_eq!(
            take_drawn(),
            [(1, true, 10 + frame), (2, true, 10 + frame)],
            "frame {frame}: both halves on the helper, from the cut's copy"
        );
        assert_eq!(
            take_joined(),
            [10 + frame],
            "frame {frame}: handed back and joined on main"
        );
        assert!(!draw_thread::posted(&e));
    }
    assert_eq!(CLOSED.swap(0, SeqCst), 3);
    assert_eq!(
        draw_thread::counts(&e),
        Counts {
            posted: 3,
            on_main: 0,
            serial: 0,
            joined: 3
        }
    );

    // Every early join point joins the same way (here `finish`, as the entries call it).
    e.mem.set_u32(VALUE, 20);
    draw_thread::first_half(&mut e, 0x6000, 0x8800, 1, 0);
    e.mem.set_u32(VALUE, 98);
    draw_thread::finish(&mut e);
    assert_eq!(take_drawn(), [(1, true, 20), (2, true, 20)]);
    draw_thread::second_half(&mut e, 0x6000, 0x8800, 1);
    assert!(
        take_drawn().is_empty(),
        "a join before the second half's call still skips it"
    );
    take_joined();

    // A join point reached on the drawing side does nothing (it would wait for itself).
    e.mem.set_u32(JOIN_INSIDE, 1);
    e.mem.set_u32(VALUE, 30);
    draw_thread::first_half(&mut e, 0x6000, 0x8800, 1, 0);
    draw_thread::second_half(&mut e, 0x6000, 0x8800, 1);
    draw_thread::idle(&mut e);
    assert_eq!(take_drawn(), [(1, true, 30), (2, true, 30)]);
    assert_eq!(take_joined(), [30]);
    e.mem.set_u32(JOIN_INSIDE, 0);

    // The post refused after the cut (the helper busy): the device's cut closed, both halves on main in the serial
    // order with the packet that was cut, joined at once; nothing left posted.
    BUSY.store(true, SeqCst);
    e.mem.set_u32(VALUE, 40);
    draw_thread::first_half(&mut e, 0x6000, 0x8800, 1, 0);
    BUSY.store(false, SeqCst);
    assert_eq!(take_drawn(), [(1, false, 40), (2, false, 40)]);
    assert_eq!(take_joined(), [40]);
    assert!(!draw_thread::posted(&e) && !real_threads::draw_posted());
    draw_thread::second_half(&mut e, 0x6000, 0x8800, 1);
    assert!(take_drawn().is_empty());
    assert_eq!(draw_thread::counts(&e).on_main, 1);

    // The job's stop comes back at the join point, and the next frame draws again.
    e.mem.set_u32(STOP, 1);
    draw_thread::first_half(&mut e, 0x6000, 0x8800, 1, 0);
    draw_thread::second_half(&mut e, 0x6000, 0x8800, 1);
    let stopped =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| draw_thread::idle(&mut e)));
    assert!(stopped.is_err(), "the job's stop comes back");
    assert!(!draw_thread::posted(&e) && !real_threads::draw_posted());
    e.call_stack.clear();
    e.mem.set_u32(STOP, 0);
    take_drawn();
    take_joined();
    e.mem.set_u32(VALUE, 50);
    draw_thread::first_half(&mut e, 0x6000, 0x8800, 1, 0);
    draw_thread::idle(&mut e);
    assert_eq!(take_drawn(), [(1, true, 50), (2, true, 50)]);

    // The end of the process with a job posted: a stepped job not started is dropped unrun, and the join point
    // after it finds nothing to wait for.
    take_joined();
    draw_thread::second_half(&mut e, 0x6000, 0x8800, 1);
    draw_thread::first_half(&mut e, 0x6000, 0x8800, 1, 0);
    assert_eq!(
        real_threads::finish_draw_at_exit(std::time::Duration::from_secs(2)),
        real_threads::DrawAtExit::Dropped
    );
    draw_thread::idle(&mut e);
    assert!(take_drawn().is_empty(), "dropped unrun");
    assert!(take_joined().is_empty());
    assert!(!draw_thread::posted(&e));

    // Turned off: the plain calls again, on main.
    draw_thread::turn_off(&mut e);
    draw_thread::first_half(&mut e, 0x6000, 0x8800, 1, 0);
    draw_thread::second_half(&mut e, 0x6000, 0x8800, 1);
    assert_eq!(take_drawn(), [(1, false, 50), (2, false, 50)]);
    let _ = testing::JOINED;
}
