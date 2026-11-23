//! The "border's Havok work on the physics step" row (`engine_havok::units::bshavok::bhkworld`, `NV_BORDER_HAVOK`):
//! the drop of the Havok world's add-lock count that a grid shift leaves to the frame's physics step. The translated
//! `bhkWorld::Update` (`00c6ae70`), the drop (`00c6b540`) and the raise, add and removal functions run as they are;
//! the step's Havok side (`hkpWorld`'s time and step calls) and the batch's own work (`00c674d0`) are doubles.
//!
//! An integration test of its own (not beside `bhkworld.rs`'s unit tests): it links only what it calls, so it builds
//! where the crate's whole unit-test program does not fit in memory.

use engine_havok::units::bshavok::bhkworld::{
    border_havok_on, border_havok_turn_on, defer_lock_drop, finish_deferred_lock_drops,
    lock_drop_deferred, BhkWorld,
};
use engine_havok::{args, Engine, Ptr, Ret};

engine_core::assemble!(engine_havok::units::funcs, engine_gamebryo::units::funcs);

/// `bhkWorld::Update`, the drop, the raise, the three adds and the three removals.
const UPDATE: u32 = 0x00c6_ae70;
const DROP: u32 = 0x00c6_b540;
const RAISE: u32 = 0x00c6_6300;
const ADDS_AND_REMOVALS: [u32; 6] = [
    0x00c6_b0a0,
    0x00c6_b320,
    0x00c6_b3c0,
    0x00c6_8bf0,
    0x00c6_8cb0,
    0x00c6_8d50,
];
/// The batch of queued bodies (`00c674d0`: `(world, entities, count)`).
const BATCH: u32 = 0x00c6_74d0;
/// The step's globals: its length and time, its scale and factor, the batch limit, the once-a-frame byte.
const STEP_LENGTH: u32 = 0x011a_fe68;
const STEP_TIME: u32 = 0x011a_fe64;
const STEP_SCALE: u32 = 0x0126_7b38;
const STEP_FACTOR: u32 = 0x011a_c3a0;
const BATCH_LIMIT: u32 = 0x011a_fe80;
const BATCH_DONE: u32 = 0x0126_77a2;
/// The `hkpWorld` calls of the step (doubles), and the step itself.
const STEP_CALLS: [u32; 9] = [
    0x00c9_1120,
    0x00c9_1140,
    0x00c9_1020,
    0x00c9_1040,
    0x00c9_1bd0,
    0x00c9_1030,
    0x00c9_3ae0,
    0x00cb_f6b0,
    0x00ec_62c0,
];
const STEP_DELTA_TIME: u32 = 0x00c9_1b10;
const STEP_DONE: u32 = 0x00c9_1060;
/// The world's slot `0x94` (its `hkpWorld`), a double reading the word at `+0x98`.
const GETTER: u32 = 0x0400_0094;

fn ret(eax: u32) -> Ret {
    Ret {
        eax,
        ..Ret::default()
    }
}

fn engine() -> Engine {
    let mut e = Engine::new();
    e.map(0x0100_0000, 0x0030_0000);
    e.map(0x00fd_f000, 0x1000);
    e
}

fn world_with(e: &mut Engine, hk_world: u32) -> Ptr<BhkWorld> {
    let this: Ptr<BhkWorld> = e.new_object();
    let mut slots = vec![0; 0x30];
    slots[0x94 / 4] = GETTER;
    e.put_vtable(0x0400_0000, &slots);
    e.mem.set_u32(this.addr(), 0x0400_0000);
    e.mem.set_u32(this.addr() + 0x98, hk_world);
    e.register_double(GETTER, |e, a| ret(e.mem.u32(a[0] + 0x98)));
    this
}

fn quiet(e: &mut Engine, addresses: &[u32]) {
    for &address in addresses {
        e.register_double(address, |_, _| Ret::default());
    }
}

/// An enabled world with an `hkpWorld` whose step is doubled: it steps twice.
fn stepping_world(e: &mut Engine) -> Ptr<BhkWorld> {
    let world = e.mem.alloc(0x400);
    let this = world_with(e, world);
    e.set(this, BhkWorld::bEnabled, true);
    e.set_global(STEP_LENGTH, 0.5f32);
    e.set_global(STEP_TIME, 0.25f32);
    e.set_global(STEP_SCALE, 60.0f32);
    e.set_global(STEP_FACTOR, 0.5f32);
    quiet(e, &STEP_CALLS);
    quiet(e, &[STEP_DELTA_TIME]);
    let steps = e.mem.alloc(8);
    e.register_double(STEP_DONE, move |e, _| {
        let n = e.mem.u32(steps);
        e.mem.set_u32(steps, n + 1);
        ret((n >= 2) as u32)
    });
    this
}

fn log(e: &mut Engine) {
    e.call_log = Some(vec![]);
}

fn calls(e: &mut Engine) -> Vec<(u32, Vec<u32>)> {
    e.call_log.take().unwrap()
}

fn addresses(calls: &[(u32, Vec<u32>)]) -> Vec<u32> {
    calls.iter().map(|(address, _)| *address).collect()
}

fn calls_to(calls: &[(u32, Vec<u32>)], address: u32) -> Vec<Vec<u32>> {
    calls
        .iter()
        .filter(|(a, _)| *a == address)
        .map(|(_, args)| args.clone())
        .collect()
}

/// The drop of the add-lock count a grid shift left is the step's first call, with its flag, before the step's own
/// entity tail; it hands over the shift's batch (the last 100 queued) exactly as the drop on main would have, once.
#[test]
fn a_deferred_lock_drop_is_the_first_thing_the_step_does() {
    let mut e = engine();
    let this = stepping_world(&mut e);
    // The batch itself is a double here: the drop hands it over and shortens the queue.
    quiet(&mut e, &[BATCH]);
    border_havok_turn_on(&e);
    assert!(border_havok_on(&e));
    let array = e.mem.alloc(0x400 * 4);
    e.set(this, BhkWorld::pEntityBatch, Ptr::new(array));
    e.set(this, BhkWorld::iEntityBatchCount, 250);
    e.set_global(BATCH_LIMIT, 100u32);
    e.set_global(BATCH_DONE, 0u8);
    e.mem.set_u32(this.addr() + 0x18, 1);
    defer_lock_drop(&e, this.addr(), 1);
    assert!(lock_drop_deferred(&e, this.addr()));
    log(&mut e);
    assert!(e.call(UPDATE, &args![this]).bool());
    let seen = calls(&mut e);
    let order = addresses(&seen);
    let drop_at = order.iter().position(|a| *a == DROP).unwrap();
    assert!(order[..drop_at].iter().all(|a| *a == UPDATE), "{order:x?}");
    assert_eq!(calls_to(&seen, DROP), [[this.addr(), 1]]);
    // The drop's batch: the last 100 of the 250, once (the step's own tail sees the frame's batch done).
    assert_eq!(
        calls_to(&seen, BATCH),
        [[this.addr(), array + 150 * 4, 100]]
    );
    assert_eq!(e.get(this, BhkWorld::iEntityBatchCount), 150);
    assert_eq!(e.mem.u32(this.addr() + 0x18), 0);
    let batch_at = order.iter().position(|a| *a == BATCH).unwrap();
    let step_at = order.iter().position(|a| *a == STEP_DELTA_TIME).unwrap();
    assert!(batch_at < step_at);
    assert!(!lock_drop_deferred(&e, this.addr()));
    // Nothing left: the next step runs no drop.
    log(&mut e);
    e.call(UPDATE, &args![this]);
    assert!(calls_to(&calls(&mut e), DROP).is_empty());
}

/// Any use of the add-lock count (a raise, a drop, an add, a removal) runs a deferred drop first, so the count's
/// operations keep the exe's order; a finish runs every drop of its own run, oldest first, and no other run's.
#[test]
fn any_use_of_the_lock_count_runs_a_deferred_drop_first() {
    let mut e = engine();
    let this = world_with(&mut e, 0).addr();
    let other = world_with(&mut e, 0).addr();
    border_havok_turn_on(&e);
    // A raise: the drop (1 -> 0), then the raise (0 -> 1).
    e.mem.set_u32(this + 0x18, 1);
    defer_lock_drop(&e, this, 1);
    log(&mut e);
    e.call(RAISE, &args![this]);
    let seen = calls(&mut e);
    assert_eq!(addresses(&seen)[..2], [RAISE, DROP]);
    assert_eq!(calls_to(&seen, DROP), [[this, 1]]);
    assert_eq!(e.mem.u32(this + 0x18), 1);
    // A drop: the deferred one first (2 -> 1), then this one (1 -> 0).
    e.mem.set_u32(this + 0x18, 2);
    defer_lock_drop(&e, this, 1);
    log(&mut e);
    e.call(DROP, &args![this, 0u32]);
    assert_eq!(calls_to(&calls(&mut e), DROP), [[this, 0], [this, 1]]);
    assert_eq!(e.mem.u32(this + 0x18), 0);
    // An add and a removal of each kind.
    for function in ADDS_AND_REMOVALS {
        e.mem.set_u32(this + 0x18, 1);
        defer_lock_drop(&e, this, 1);
        log(&mut e);
        e.call(function, &args![this, 0u32]);
        let seen = calls(&mut e);
        assert_eq!(calls_to(&seen, DROP), [[this, 1]], "{function:08x}");
        assert!(!lock_drop_deferred(&e, this));
    }
    // A finish: this run's drops, oldest first; another run's stay.
    let mut second = engine();
    let elsewhere = world_with(&mut second, 0).addr();
    border_havok_turn_on(&second);
    e.mem.set_u32(this + 0x18, 1);
    e.mem.set_u32(other + 0x18, 1);
    second.mem.set_u32(elsewhere + 0x18, 1);
    defer_lock_drop(&e, other, 1);
    defer_lock_drop(&e, this, 1);
    defer_lock_drop(&second, elsewhere, 1);
    log(&mut e);
    finish_deferred_lock_drops(&mut e);
    assert_eq!(calls_to(&calls(&mut e), DROP), [[other, 1], [this, 1]]);
    assert!(lock_drop_deferred(&second, elsewhere));
    log(&mut second);
    finish_deferred_lock_drops(&mut second);
    assert_eq!(calls_to(&calls(&mut second), DROP).len(), 1);
    // Nothing left: a finish calls nothing.
    log(&mut e);
    finish_deferred_lock_drops(&mut e);
    assert!(calls(&mut e).is_empty());
}
