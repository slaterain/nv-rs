//! The drawing thread's seam with real threads (`engine_core::draw_thread`; one drawing helper per process, so this
//! binary holds one run and one test, repeated by running the binary again): the world draw posted at the first
//! half's call runs on the `nv-draw` helper beside main's tail, from the packet main cut before the tail, and is
//! joined at the next join point; the second half's call is skipped.

use engine_core::draw_thread::{self, Hooks, Packet};
use engine_core::real_threads::{self, on_draw_helper};
use engine_core::{Engine, Ret};
use std::cell::Cell;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering::SeqCst};
use std::sync::Mutex;

engine_core::assemble!();

const VALUE: u32 = 0x0300_0000;

static DRAWN: Mutex<Vec<(u32, bool, u32)>> = Mutex::new(Vec::new());
static JOINED: Mutex<Vec<u32>> = Mutex::new(Vec::new());
static CLOSED: AtomicU32 = AtomicU32::new(0);
/// The first half waits for this (main's tail has started) before it reads: it really runs beside main.
static TAIL_STARTED: AtomicBool = AtomicBool::new(false);

thread_local! {
    static HELD: Cell<Option<u32>> = const { Cell::new(None) };
}

fn read_value(e: &Engine) -> u32 {
    HELD.with(Cell::get).unwrap_or_else(|| e.mem.u32(VALUE))
}

fn first(e: &mut Engine, _: &[u32]) -> Ret {
    while !TAIL_STARTED.load(SeqCst) {
        std::thread::yield_now();
    }
    // A join point reached on the drawing side while main has not joined yet: nothing (it would wait for itself).
    draw_thread::finish(e);
    draw_thread::idle(e);
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
    assert!(!stepped);
    Some(Box::new(e.mem.u32(VALUE)))
}

fn adopt(_: &mut Engine, p: Packet) -> Result<(), Packet> {
    HELD.with(|h| h.set(Some(*p.downcast::<u32>().unwrap())));
    Ok(())
}

fn hand_back(_: &mut Engine) -> Option<Packet> {
    HELD.with(Cell::take).map(|v| Box::new(v) as Packet)
}

fn join(_: &mut Engine, p: Packet) {
    assert!(!on_draw_helper());
    JOINED.lock().unwrap().push(*p.downcast::<u32>().unwrap());
}

fn device_cut(_: &mut Engine, _: u32) -> bool {
    true
}

fn device_close(_: &mut Engine) {
    CLOSED.fetch_add(1, SeqCst);
}

fn device_finish(e: &mut Engine) {
    real_threads::join_draw(e);
}

#[test]
fn with_real_threads_the_world_draw_runs_beside_the_tail_and_joins() {
    let mut e = Engine::new();
    e.start_real_clock();
    real_threads::start(&mut e).expect("real threads on");
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
    for frame in 0..30u32 {
        TAIL_STARTED.store(false, SeqCst);
        e.mem.set_u32(VALUE, 1000 + frame);
        draw_thread::first_half(&mut e, 0x6000, 0x8800, 1, 0);
        assert!(draw_thread::posted(&e), "frame {frame}");
        draw_thread::second_half(&mut e, 0x6000, 0x8800, 1);
        // The tail: it changes what the draw reads while the draw may be running.
        e.mem.set_u32(VALUE, 7);
        TAIL_STARTED.store(true, SeqCst);
        for i in 0..100 {
            e.mem.set_u32(VALUE, i);
        }
        draw_thread::idle(&mut e);
        assert_eq!(
            std::mem::take(&mut *DRAWN.lock().unwrap()),
            [(1, true, 1000 + frame), (2, true, 1000 + frame)],
            "frame {frame}"
        );
        assert_eq!(
            std::mem::take(&mut *JOINED.lock().unwrap()),
            [1000 + frame],
            "frame {frame}"
        );
        assert!(!draw_thread::posted(&e) && !real_threads::draw_posted());
    }
    assert_eq!(CLOSED.load(SeqCst), 30);
    assert_eq!(draw_thread::counts(&e).joined, 30);
    draw_thread::turn_off(&mut e);
}
