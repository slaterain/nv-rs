use super::testing::{self, hooks_finishing_with};
use super::*;
use crate::abi::Ret;

/// The halves' stand-ins add their address and words to the guest log at [`LOG`].
const LOG: u32 = 0x0300_0000;

fn logged(e: &mut Engine, addr: u32, a: &[u32]) {
    let n = e.mem.u32(LOG);
    let at = LOG + 4 + n * 16;
    e.mem.set_u32(at, addr);
    for (i, w) in (0u32..).zip(a.iter().take(3)) {
        e.mem.set_u32(at + 4 + 4 * i, *w);
    }
    e.mem.set_u32(LOG, n + 1);
}

fn first(e: &mut Engine, a: &[u32]) -> Ret {
    logged(e, WORLD_DRAW_FIRST, a);
    Ret::default()
}

fn second(e: &mut Engine, a: &[u32]) -> Ret {
    logged(e, WORLD_DRAW_SECOND, a);
    Ret::default()
}

fn log(e: &Engine) -> Vec<[u32; 4]> {
    (0..e.mem.u32(LOG))
        .map(|i| {
            let at = LOG + 4 + i * 16;
            [0, 4, 8, 12].map(|o| e.mem.u32(at + o))
        })
        .collect()
}

fn world() -> Engine {
    let mut e = Engine::new();
    e.map(LOG, 0x1000);
    for page in [
        VATS & !0xfff,
        NAVMESH_RENDER & !0xfff,
        LOADING_MENU & !0xfff,
    ] {
        if e.mem.try_read(page, &mut [0u8; 1]).is_err() {
            e.map(page, 0x1000);
        }
    }
    e.register(WORLD_DRAW_FIRST, first);
    e.register(WORLD_DRAW_SECOND, second);
    e
}

fn never_finish(_: &mut Engine) {
    panic!("no job was posted");
}

fn stop_cut(_: &mut Engine, _: u32, _: u32, _: bool) -> Option<Packet> {
    panic!("a cut in a serial frame")
}

/// Off: the two calls are the world pass's own, in order, with its words, and no join point does anything.
#[test]
fn off_the_halves_are_the_plain_calls() {
    let mut e = world();
    assert!(!is_on(&e));
    first_half(&mut e, 0x6000, 0x8800, 1, 0);
    finish(&mut e);
    idle(&mut e);
    save_or_load(&mut e, 0x0085_03b0);
    second_half(&mut e, 0x6000, 0x8800, 1);
    assert_eq!(
        log(&e),
        [
            [WORLD_DRAW_FIRST, 0x6000, 0x8800, 1],
            [WORLD_DRAW_SECOND, 0x6000, 0x8800, 1]
        ]
    );
    assert_eq!(counts(&e), Counts::default());
    assert!(!posted(&e));
}

/// The game's states that keep the draw on main, each alone: the plain calls in order, counted serial, no cut.
#[test]
fn each_serial_condition_keeps_the_draw_on_main() {
    type Set = fn(&mut Engine);
    let cases: [(&str, Set, u8); 5] = [
        ("overlay", |_| {}, 1),
        ("vats", |e| e.mem.set_u32(VATS + VATS_MODE, 1), 0),
        (
            "navmesh",
            |e| {
                let render = e.mem.alloc(0x220);
                e.mem.set_u8(render + NAVMESH_DRAWING, 1);
                e.mem.set_u32(NAVMESH_RENDER, render);
            },
            0,
        ),
        ("loading", |e| e.mem.set_u32(LOADING_MENU, 0x1234), 0),
        ("save", |e| save_or_load(e, 0x0085_03b0), 0),
    ];
    for (name, set, overlay) in cases {
        let mut e = world();
        let hooks = Hooks {
            cut: stop_cut,
            ..hooks_finishing_with(never_finish)
        };
        turn_on(&e, hooks);
        set(&mut e);
        first_half(&mut e, 0x6000, 0x8800, 1, overlay);
        second_half(&mut e, 0x6000, 0x8800, 1);
        let order: Vec<u32> = log(&e).iter().map(|l| l[0]).collect();
        assert_eq!(order, [WORLD_DRAW_FIRST, WORLD_DRAW_SECOND], "{name}");
        assert_eq!(counts(&e).serial, 1, "{name}");
        assert_eq!(counts(&e).posted + counts(&e).on_main, 0, "{name}");
        turn_off(&mut e);
    }
}

/// The navmesh object alone, without its drawing flag, is no reason.
#[test]
fn the_navmesh_object_without_its_flag_is_no_reason() {
    let mut e = world();
    let render = e.mem.alloc(0x220);
    e.mem.set_u32(NAVMESH_RENDER, render);
    assert_eq!(serial_reason(&e, false, 0), None);
    e.mem.set_u8(render + NAVMESH_DRAWING, 1);
    assert_eq!(serial_reason(&e, false, 0), Some("the navmesh is drawn"));
}

/// A save's or a load's serial frames end at the next OnIdle.
#[test]
fn a_save_is_serial_until_the_next_idle() {
    let mut e = world();
    turn_on(&e, hooks_finishing_with(never_finish));
    let flag = |e: &Engine| with_run(e.kernel.id(), |r| r.save_or_load).unwrap();
    save_or_load(&mut e, 0x0085_03b0);
    assert!(flag(&e));
    assert_eq!(
        serial_reason(&e, flag(&e), 0),
        Some("a save or a load is in progress")
    );
    idle(&mut e);
    assert!(!flag(&e));
    assert_eq!(serial_reason(&e, flag(&e), 0), None);
    turn_off(&mut e);
}

/// The host's reason, and a helper that takes no job (a call log on is a per-call diagnostic), keep it on main.
#[test]
fn the_hosts_reason_and_a_refusing_helper_keep_the_draw_on_main() {
    fn reason(_: &Engine) -> Option<&'static str> {
        Some("test")
    }
    for host in [true, false] {
        let mut e = world();
        let mut hooks = Hooks {
            cut: stop_cut,
            ..hooks_finishing_with(never_finish)
        };
        if host {
            hooks.serial = reason;
        } else {
            e.call_log = Some(Vec::new());
        }
        turn_on(&e, hooks);
        first_half(&mut e, 0x6000, 0x8800, 1, 0);
        second_half(&mut e, 0x6000, 0x8800, 1);
        assert_eq!(log(&e).len(), 2, "host {host}");
        assert_eq!(counts(&e).serial, 1, "host {host}");
        turn_off(&mut e);
    }
}

/// Nothing cut (the draw family has nothing to hand off): the plain calls.
#[test]
fn nothing_cut_is_the_plain_calls() {
    fn no_cut(_: &mut Engine, _: u32, _: u32, _: bool) -> Option<Packet> {
        None
    }
    fn stop_device(_: &mut Engine, _: u32) -> bool {
        panic!("a device cut without a packet")
    }
    let mut e = world();
    turn_on(
        &e,
        Hooks {
            cut: no_cut,
            device_cut: stop_device,
            ..hooks_finishing_with(never_finish)
        },
    );
    first_half(&mut e, 0x6000, 0x8800, 1, 0);
    second_half(&mut e, 0x6000, 0x8800, 1);
    assert_eq!(log(&e).len(), 2);
    assert_eq!(counts(&e).serial, 1);
    turn_off(&mut e);
}

thread_local! {
    static HELD: std::cell::Cell<Option<u32>> = const { std::cell::Cell::new(None) };
    static JOINED_WITH: std::cell::Cell<Option<u32>> = const { std::cell::Cell::new(None) };
}

fn cut_value(e: &mut Engine, _: u32, _: u32, _: bool) -> Option<Packet> {
    Some(Box::new(e.mem.u32(LOG + 0x800)))
}

fn adopt_value(_: &mut Engine, p: Packet) -> Result<(), Packet> {
    let value = *p.downcast::<u32>().expect("the cut's value");
    HELD.with(|h| h.set(Some(value)));
    Ok(())
}

fn back_value(_: &mut Engine) -> Option<Packet> {
    HELD.with(|h| h.take()).map(|v| Box::new(v) as Packet)
}

fn join_value(_: &mut Engine, p: Packet) {
    let value = *p.downcast::<u32>().expect("the value back");
    JOINED_WITH.with(|j| j.set(Some(value)));
}

fn no_device_cut(_: &mut Engine, _: u32) -> bool {
    false
}

/// No device cut: both halves drawn on main at the first half's call with the packet adopted there, handed back
/// and joined at once; the second half's call skipped, that frame only. Nothing is posted.
#[test]
fn without_a_device_cut_the_halves_draw_on_main_with_the_packet() {
    let mut e = world();
    e.mem.set_u32(LOG + 0x800, 77);
    turn_on(
        &e,
        Hooks {
            cut: cut_value,
            adopt: adopt_value,
            hand_back: back_value,
            join: join_value,
            device_cut: no_device_cut,
            ..hooks_finishing_with(never_finish)
        },
    );
    first_half(&mut e, 0x6000, 0x8800, 1, 0);
    assert_eq!(log(&e).len(), 2, "both halves at the first half's call");
    second_half(&mut e, 0x6000, 0x8800, 1);
    assert_eq!(log(&e).len(), 2, "the second half's call skipped");
    assert_eq!(JOINED_WITH.with(|j| j.take()), Some(77));
    assert_eq!(counts(&e).on_main, 1);
    assert!(!posted(&e));
    second_half(&mut e, 0x6000, 0x8800, 1);
    assert_eq!(
        log(&e).len(),
        3,
        "the next frame's second half is the plain call"
    );
    turn_off(&mut e);
}

/// A posted job is joined once, by the first join point that comes (`finish`, `idle` or `save_or_load`).
#[test]
fn every_join_point_joins_a_posted_job_once() {
    thread_local! {
        static FINISHED: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    }
    fn count(_: &mut Engine) {
        FINISHED.with(|f| f.set(f.get() + 1));
    }
    type Join = fn(&mut Engine);
    fn save(e: &mut Engine) {
        save_or_load(e, 0x0085_03b0);
    }
    for join in [finish as Join, idle, save] {
        let mut e = world();
        turn_on(&e, hooks_finishing_with(count));
        testing::mark_posted(&e);
        assert!(posted(&e));
        join(&mut e);
        join(&mut e);
        assert_eq!(FINISHED.with(|f| f.replace(0)), 1);
        assert!(!posted(&e));
        assert_eq!(counts(&e).joined, 1);
        turn_off(&mut e);
    }
}

/// The job's stop (resumed by the device's join) comes back from the join point, and nothing is posted after it.
#[test]
fn a_job_that_stopped_stops_the_join_point() {
    fn stop(_: &mut Engine) {
        panic!("the job stopped");
    }
    let mut e = world();
    turn_on(&e, hooks_finishing_with(stop));
    testing::mark_posted(&e);
    let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| idle(&mut e)));
    assert!(out.is_err());
    assert!(!posted(&e));
    turn_off(&mut e);
}

/// The helper the engine crates' join tests use: true only when the call joins first.
#[test]
fn joins_first_tells_a_joining_entry_from_one_that_does_not() {
    fn joins(e: &mut Engine, _: &[u32]) -> Ret {
        finish(e);
        panic!("went on past the join")
    }
    fn reads_first(e: &mut Engine, _: &[u32]) -> Ret {
        e.mem.u32(0);
        Ret::default()
    }
    fn returns(_: &mut Engine, _: &[u32]) -> Ret {
        Ret::default()
    }
    let mut e = Engine::new();
    e.register(0x0040_1000, joins);
    e.register(0x0040_2000, reads_first);
    e.register(0x0040_3000, returns);
    assert!(testing::joins_first(&mut e, 0x0040_1000, &[]));
    assert!(!testing::joins_first(&mut e, 0x0040_2000, &[]));
    assert!(!testing::joins_first(&mut e, 0x0040_3000, &[]));
    assert!(!is_on(&e));
}

/// The device the cut names: the renderer's (+0x288), 0 without a renderer.
#[test]
fn the_device_is_the_renderers() {
    let mut e = world();
    if e.mem.try_read(RENDERER & !0xfff, &mut [0u8; 1]).is_err() {
        e.map(RENDERER & !0xfff, 0x1000);
    }
    assert_eq!(device(&e), 0);
    let renderer = e.mem.alloc(0x300);
    e.mem.set_u32(renderer + RENDERER_DEVICE, 0xd3d9);
    e.mem.set_u32(RENDERER, renderer);
    assert_eq!(device(&e), 0xd3d9);
}

/// A family in its dual run keeps the draw on main: the helper takes no job (`draw_post_ready`, as `post_draw`).
#[test]
fn a_dual_family_keeps_the_draw_on_main() {
    use crate::typed::{Family, FamilyId, Threads};
    fn member(_: &mut Engine, _: &[u32]) -> Ret {
        Ret::default()
    }
    fn funcs() -> Vec<(u32, crate::AbiFn)> {
        vec![(0x0040_5000, member as crate::AbiFn)]
    }
    static DUAL: Family = Family {
        name: "draw_thread_dual_test",
        id: FamilyId::new(31),
        past_gate: true,
        threads: Threads::Main,
        stage2: true,
        funcs,
    };
    let mut e = Engine::new_untyped_for_test();
    e.typed_apply_for_test(&[&DUAL], "draw_thread_dual_test:dual");
    assert!(e.typed_any_dual());
    assert!(!real_threads::draw_post_ready(&e));
    e.map(LOG, 0x1000);
    e.register(WORLD_DRAW_FIRST, first);
    e.register(WORLD_DRAW_SECOND, second);
    turn_on(
        &e,
        Hooks {
            cut: stop_cut,
            ..hooks_finishing_with(never_finish)
        },
    );
    first_half(&mut e, 0x6000, 0x8800, 1, 0);
    second_half(&mut e, 0x6000, 0x8800, 1);
    assert_eq!(log(&e).len(), 2);
    assert_eq!(counts(&e).serial, 1);
    turn_off(&mut e);
}
