//! Lane C1's tests: the character controller commands (`jip_fn_actor` and `jip_fn_actor_p2`),
//! the last hit's node, VATS targetability, `AttachAshPileEx`, `UpdatePlayer3D`,
//! `PushActorAwayAlt`, `DismemberLimb`, the map marker commands (`jip_fn_misc_ref`),
//! `GetSessionTime` (`jip_fn_utility`) and `PlaySound3DNode` (`jip_fn_sound`), each called by
//! its opcode. Scenes at the offsets the modules' docs give (each confirmed in the exe), game
//! functions as stand-ins that record their words. Every expectation is taken from JIP's command
//! bodies and helpers (functions_jip/jip_fn_actor.h, jip_fn_misc_ref.h, jip_fn_utility.h,
//! jip_fn_sound.h, nvse/GameObjects.cpp, nvse/GameTypes.h, internal/havok.cpp, Ni_types.cpp,
//! utility.cpp), never from the port's output.

use super::*;
use crate::commands::test_support::*;
use crate::test_support::LIST;

const AREA: u32 = 0x0500_0000;
const AREA_SIZE: u32 = 0x10_0000;
const ACTOR: u32 = AREA + 0x1000;
const ACTOR2: u32 = AREA + 0x1800;
const PLAIN: u32 = AREA + 0x2000;
const PROCESS: u32 = AREA + 0x3000;
const CTRL: u32 = AREA + 0x4000;
const PROXY: u32 = AREA + 0x5000;
const ACTOR_VT: u32 = AREA + 0x8000;
const PLAIN_VT: u32 = AREA + 0x8800;
const PROC_VT: u32 = AREA + 0x9000;
const PROJ_VT: u32 = AREA + 0x9800;
const BASE_VT: u32 = AREA + 0xa000;
const BASE: u32 = AREA + 0xb000;
const LOCALS: u32 = LIST + 0x100;
const STANDIN: u32 = 0x0090_0400;
const RETURN_TRUE: u32 = 0x008d_0360;
const KEY_AV_OBJECT: u32 = 0x011f_4280;
const FADE_NODE_VT: u32 = 0x010a_8f90;
const THE_PLAYER_AT: u32 = 0x011d_ea3c;

thread_local! {
    static CALLED: std::cell::RefCell<Vec<(u32, Vec<u32>)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

fn record(addr: u32, w: &[u32]) {
    CALLED.with(|c| c.borrow_mut().push((addr, w.to_vec())));
}

fn called(addr: u32) -> Vec<Vec<u32>> {
    CALLED.with(|c| {
        c.borrow()
            .iter()
            .filter(|x| x.0 == addr)
            .map(|x| x.1.clone())
            .collect()
    })
}

fn eax(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

/// A stand-in at `addr` recording its words and answering the word at `answer` (both constant
/// expressions, so the closure captures nothing).
macro_rules! standin {
    ($e:expr, $addr:expr, $answer:expr) => {
        $e.register($addr, |e, w| {
            record($addr, w);
            eax(e.mem.u32($answer))
        })
    };
}

fn map_once(e: &mut Engine, at: u32, size: u32) {
    if !e.mem.is_mapped(at) {
        e.map(at, size);
    }
}

/// An actor (`ACTOR`) with a level 0 process and a controller; a plain reference (`PLAIN`).
fn setup() -> Engine {
    CALLED.with(|c| c.borrow_mut().clear());
    let mut e = engine();
    e.map(AREA, AREA_SIZE);
    map_once(&mut e, 0x011d_e000, 0x1000);
    e.mem.set_u32(ACTOR_VT + 0x100, RETURN_TRUE);
    e.mem.set_u32(PLAIN_VT + 0x100, 0x0047_c850);
    e.mem.set_u32(PROJ_VT + 0x100, 0x0047_c850);
    e.mem.set_u32(PROJ_VT + 0x224, RETURN_TRUE);
    for a in [ACTOR, ACTOR2] {
        e.mem.set_u32(a, ACTOR_VT);
        e.mem.set_u8(a + 4, 0x3b);
        e.mem.set_u32(a + 0x20, BASE);
    }
    e.mem.set_u32(BASE, BASE_VT);
    e.mem.set_u8(BASE + 4, 0x2a);
    e.mem.set_u32(PLAIN, PLAIN_VT);
    e.mem.set_u8(PLAIN + 4, 0x3a);
    e.mem.set_u32(ACTOR + 0xc, 0x0100_0001);
    e.mem.set_u32(ACTOR + 0x68, PROCESS);
    e.mem.set_u32(PROCESS, PROC_VT);
    e.mem.set_u32(PROCESS + 0x138, CTRL);
    e.mem.set_u32(CTRL + 8, PROXY);
    e
}

fn run(e: &mut Engine, opcode: u32, this: u32, args: &[Vec<u8>]) -> f64 {
    e.mem.set_f64(RESULT, 0.0);
    let (ok, result) = call_on(e, opcode, this, args);
    assert_eq!(ok, 1, "{opcode:04x} answers true");
    result
}

fn axis(a: u8) -> Vec<u8> {
    vec![a]
}

fn v(id: u16) -> Vec<u8> {
    let mut b = vec![b'f'];
    b.extend_from_slice(&id.to_le_bytes());
    b
}

fn locals(e: &mut Engine, n: u32) {
    for i in 0..n {
        let local = LOCALS + 0x100 + i * 0x10;
        e.mem.set_u32(local, i + 1);
        e.mem.set_f64(local + 8, -1.0);
        let node = LOCALS + i * 8;
        e.mem.set_u32(node, local);
        e.mem
            .set_u32(node + 4, if i + 1 < n { node + 8 } else { 0 });
    }
    e.mem.set_u32(LIST + 0xc, LOCALS);
}

fn vars(e: &Engine) -> [f64; 3] {
    [0, 1, 2].map(|i| e.mem.f64(LOCALS + 0x100 + i * 0x10 + 8))
}

fn set4(e: &mut Engine, at: u32, v: [f32; 4]) {
    for (i, x) in v.iter().enumerate() {
        e.mem.set_f32(at + 4 * i as u32, *x);
    }
}

fn text(e: &mut Engine, at: u32, t: &[u8]) -> u32 {
    e.mem.write(at, t);
    e.mem.set_u8(at + t.len() as u32, 0);
    at
}

// ---- the controller: jip_fn_actor ----

#[test]
fn fall_times_read_their_own_fields() {
    let mut e = setup();
    e.mem.set_f32(CTRL + 0x548, 1.5);
    e.mem.set_f32(CTRL + 0x530, 2.25);
    e.mem.set_f32(CTRL + 0x544, 9.0);
    assert_eq!(run(&mut e, 0x276c, ACTOR, &[]), 1.5);
    assert_eq!(run(&mut e, 0x276d, ACTOR, &[]), 2.25);
}

#[test]
fn the_controller_needs_an_actor_with_a_process_of_level_0_or_1() {
    let mut e = setup();
    e.mem.set_f32(CTRL + 0x548, 1.5);
    e.mem.set_u8(PROCESS + 0x28, 1);
    assert_eq!(run(&mut e, 0x276c, ACTOR, &[]), 1.5);
    e.mem.set_u8(PROCESS + 0x28, 2);
    assert_eq!(run(&mut e, 0x276c, ACTOR, &[]), 0.0);
    e.mem.set_u8(PROCESS + 0x28, 0);
    // Not an actor: a plain reference with the same process word.
    e.mem.set_u32(PLAIN + 0x68, PROCESS);
    assert_eq!(run(&mut e, 0x276c, PLAIN, &[]), 0.0);
    // No process, no controller, no reference.
    e.mem.set_u32(ACTOR2 + 0x68, 0);
    assert_eq!(run(&mut e, 0x276c, ACTOR2, &[]), 0.0);
    e.mem.set_u32(PROCESS + 0x138, 0);
    assert_eq!(run(&mut e, 0x276c, ACTOR, &[]), 0.0);
    assert_eq!(run(&mut e, 0x276c, 0, &[]), 0.0);
}

#[test]
fn reset_fall_time_stores_the_z_and_zero() {
    let mut e = setup();
    e.mem.set_f32(ACTOR + 0x34, 55.0);
    e.mem.set_f32(ACTOR + 0x38, 123.5);
    e.mem.set_f32(CTRL + 0x544, 7.0);
    e.mem.set_f32(CTRL + 0x548, 3.0);
    e.mem.set_f32(CTRL + 0x530, 4.0);
    assert_eq!(run(&mut e, 0x276e, ACTOR, &[]), 0.0);
    assert_eq!(e.mem.f32(CTRL + 0x544), 123.5);
    assert_eq!(e.mem.f32(CTRL + 0x548), 0.0);
    assert_eq!(e.mem.f32(CTRL + 0x530), 4.0, "the pitch timer is left");
    // Without a controller nothing is written.
    e.mem.set_f32(CTRL + 0x548, 3.0);
    e.mem.set_u8(PROCESS + 0x28, 3);
    run(&mut e, 0x276e, ACTOR, &[]);
    assert_eq!(e.mem.f32(CTRL + 0x548), 3.0);
}

#[test]
fn actor_velocity_by_axis_and_its_length() {
    let mut e = setup();
    set4(&mut e, CTRL + 0x4f0, [3.0, 4.0, 12.0, 2.0]);
    assert_eq!(run(&mut e, 0x27b4, ACTOR, &[axis(b'X')]), 3.0);
    assert_eq!(run(&mut e, 0x27b4, ACTOR, &[axis(b'Y')]), 4.0);
    assert_eq!(run(&mut e, 0x27b4, ACTOR, &[axis(b'Z')]), 12.0);
    let s = (3.0f32 * 3.0 + 4.0 * 4.0) + (12.0 * 12.0 + 2.0 * 2.0);
    let want = super::super::ln_fn_math::newton_sqrt(s);
    assert_eq!(run(&mut e, 0x27b4, ACTOR, &[]), f64::from(want));
    // Another byte than X, Y, Z: nothing (JIP would index past the vector).
    assert_eq!(run(&mut e, 0x27b4, ACTOR, &[axis(b'W')]), 0.0);
    e.mem.set_u8(PROCESS + 0x28, 2);
    assert_eq!(run(&mut e, 0x27b4, ACTOR, &[axis(b'X')]), 0.0);
}

#[test]
fn in_air_is_state_bit_1() {
    let mut e = setup();
    for (state, want) in [
        (0u32, 0.0),
        (1, 0.0),
        (2, 1.0),
        (3, 1.0),
        (4, 0.0),
        (6, 1.0),
    ] {
        e.mem.set_u32(CTRL + 0x3f0, state);
        assert_eq!(run(&mut e, 0x27b5, ACTOR, &[]), want, "{state}");
        let eval = e.mem.u32(nvse::table::record_of(&e, 0x27b5) + 0x20);
        e.mem.set_f64(RESULT, 0.0);
        e.call(eval, &[ACTOR, 0, 0, RESULT]);
        assert_eq!(e.mem.f64(RESULT), want, "eval {state}");
    }
    e.mem.set_u8(PROCESS + 0x28, 2);
    assert_eq!(run(&mut e, 0x27b5, ACTOR, &[]), 0.0);
}

/// A body whose `NiAVObject` property is a `BSFadeNode` holding `refr` (+0xCC).
fn body_of(e: &mut Engine, at: u32, refr: u32) -> u32 {
    let node = at + 0x200;
    e.mem.set_u32(node, FADE_NODE_VT);
    e.mem.set_u32(node + 0xcc, refr);
    let props = at + 0x300;
    e.mem.set_u32(props, KEY_AV_OBJECT);
    e.mem.set_u32(props + 8, node);
    e.mem.set_u32(at + 0x78, props);
    e.mem.set_u32(at + 0x7c, 1);
    at
}

#[test]
fn object_under_feet_is_the_bodys_reference() {
    let mut e = setup();
    e.mem.set_u32(PLAIN + 0xc, 0x0004_5678);
    let body = body_of(&mut e, AREA + 0x6000, PLAIN);
    e.mem.set_u32(CTRL + 0x60c, body);
    // The support count is not consulted (unlike the contact commands).
    e.mem.set_u8(CTRL + 0x608, 1);
    run(&mut e, 0x27ba, ACTOR, &[]);
    assert_eq!(form_result(&e), 0x0004_5678);
    // No body under the feet, or a body without a reference: nothing.
    e.mem.set_u32(CTRL + 0x60c, 0);
    run(&mut e, 0x27ba, ACTOR, &[]);
    assert_eq!(form_result(&e), 0);
    e.mem.set_u32(body + 0x200 + 0xcc, 0);
    e.mem.set_u32(CTRL + 0x60c, body);
    run(&mut e, 0x27ba, ACTOR, &[]);
    assert_eq!(form_result(&e), 0);
}

// ---- the controller: jip_fn_actor_p2 ----

#[test]
fn tilt_angles_in_degrees_for_x_and_y_only() {
    let mut e = setup();
    e.mem.set_f32(CTRL + 0x550, 0.5);
    e.mem.set_f32(CTRL + 0x554, -0.25);
    e.mem.set_f32(CTRL + 0x558, 9.0);
    assert_eq!(
        run(&mut e, 0x28db, ACTOR, &[axis(b'X')]),
        0.5 * super::super::DBL_180_D_PI
    );
    assert_eq!(
        run(&mut e, 0x28db, ACTOR, &[axis(b'Y')]),
        -0.25 * super::super::DBL_180_D_PI
    );
    assert_eq!(run(&mut e, 0x28db, ACTOR, &[axis(b'Z')]), 0.0);
}

#[test]
fn tilt_angle_set_in_radians_as_floats() {
    let mut e = setup();
    e.mem.set_f32(CTRL + 0x558, 9.0);
    assert_eq!(run(&mut e, 0x28dc, ACTOR, &[axis(b'X'), z(90.0)]), 0.0);
    assert_eq!(
        e.mem.f32(CTRL + 0x550),
        90.0f32 * super::super::FLT_PI_D_180
    );
    run(&mut e, 0x28dc, ACTOR, &[axis(b'Y'), z(-45.0)]);
    assert_eq!(
        e.mem.f32(CTRL + 0x554),
        -45.0f32 * super::super::FLT_PI_D_180
    );
    run(&mut e, 0x28dc, ACTOR, &[axis(b'Z'), z(10.0)]);
    assert_eq!(e.mem.f32(CTRL + 0x558), 9.0, "Z writes nothing");
    e.mem.set_u8(PROCESS + 0x28, 2);
    run(&mut e, 0x28dc, ACTOR, &[axis(b'X'), z(1.0)]);
    assert_eq!(
        e.mem.f32(CTRL + 0x550),
        90.0f32 * super::super::FLT_PI_D_180
    );
}

#[test]
fn gravity_read_and_stored() {
    let mut e = setup();
    e.mem.set_f32(CTRL + 0x54c, 1.0);
    assert_eq!(run(&mut e, 0x28e7, ACTOR, &[]), 1.0);
    assert_eq!(run(&mut e, 0x28e8, ACTOR, &[z(2.5)]), 0.0);
    assert_eq!(e.mem.f32(CTRL + 0x54c), 2.5);
    assert_eq!(run(&mut e, 0x28e7, ACTOR, &[]), 2.5);
    e.mem.set_u8(PROCESS + 0x28, 2);
    assert_eq!(run(&mut e, 0x28e7, ACTOR, &[]), 0.0);
    run(&mut e, 0x28e8, ACTOR, &[z(7.0)]);
    assert_eq!(e.mem.f32(CTRL + 0x54c), 2.5);
}

#[test]
fn actor_velocity_set_on_the_proxy() {
    let mut e = setup();
    set4(&mut e, PROXY + 0x10, [1.0, 2.0, 3.0, 4.0]);
    set4(&mut e, CTRL + 0x4f0, [5.0; 4]);
    assert_eq!(run(&mut e, 0x2842, ACTOR, &[axis(b'Y'), z(7.0)]), 0.0);
    run(&mut e, 0x2842, ACTOR, &[axis(b'X'), z(-1.0)]);
    run(&mut e, 0x2842, ACTOR, &[axis(b'Z'), z(0.5)]);
    for (i, want) in [-1.0f32, 7.0, 0.5, 4.0].into_iter().enumerate() {
        assert_eq!(e.mem.f32(PROXY + 0x10 + 4 * i as u32), want, "{i}");
    }
    assert_eq!(
        e.mem.f32(CTRL + 0x4f4),
        5.0,
        "the controller's own velocity is left"
    );
    // No proxy: nothing written where one would be.
    e.mem.set_u32(CTRL + 8, 0);
    run(&mut e, 0x2842, ACTOR, &[axis(b'Y'), z(9.0)]);
    assert_eq!(e.mem.f32(PROXY + 0x14), 7.0);
}

#[test]
fn actor_velocity_alt_world_and_local() {
    let mut e = setup();
    locals(&mut e, 3);
    let root = AREA + 0x7000;
    let loaded = AREA + 0x7400;
    e.mem.set_u32(loaded + 0x14, root);
    e.mem.set_u32(ACTOR + 0x64, loaded);
    set4(&mut e, CTRL + 0x4f0, [1.0, 2.0, 3.0, 99.0]);
    let m = [0.5f32, 0.25, 2.0, -1.0, 0.0, 4.0, 3.0, 1.5, -2.0];
    for (i, x) in m.iter().enumerate() {
        e.mem.set_f32(root + 0x68 + 4 * i as u32, *x);
    }
    let args = |local: i32| vec![v(1), v(2), v(3), n(local)];
    assert_eq!(run(&mut e, 0x2912, ACTOR, &args(0)), 1.0);
    assert_eq!(vars(&e), [1.0, 2.0, 3.0]);
    assert_eq!(run(&mut e, 0x2912, ACTOR, &args(1)), 1.0);
    let want: [f64; 3] =
        std::array::from_fn(|i| f64::from((m[i] * 1.0 + m[3 + i] * 2.0) + m[6 + i] * 3.0));
    assert_eq!(vars(&e), want);
    // Without 3D: nothing, the variables left.
    locals(&mut e, 3);
    e.mem.set_u32(ACTOR + 0x64, 0);
    assert_eq!(run(&mut e, 0x2912, ACTOR, &args(0)), 0.0);
    assert_eq!(vars(&e), [-1.0; 3]);
    // Without a controller: nothing.
    e.mem.set_u32(ACTOR + 0x64, loaded);
    e.mem.set_u8(PROCESS + 0x28, 2);
    assert_eq!(run(&mut e, 0x2912, ACTOR, &args(0)), 0.0);
    assert_eq!(vars(&e), [-1.0; 3]);
}

// ---- the last hit's node ----

const HIT: u32 = AREA + 0xc000;
const PROJ: u32 = AREA + 0xc100;
const GET_HIT_DATA: u32 = STANDIN;

fn hit_scene(e: &mut Engine) {
    e.mem.set_u32(PROC_VT + 0x770, GET_HIT_DATA);
    e.mem.set_u32(AREA + 0xf000, HIT);
    standin!(e, GET_HIT_DATA, AREA + 0xf000);
    e.mem.set_u32(HIT + 8, PROJ);
    e.mem.set_u32(PROJ, PROJ_VT);
    // Impacts: another actor's (with a body), one on this actor without a body, then this
    // actor's with a body whose node is named.
    let (i1, i2, i3) = (AREA + 0xd000, AREA + 0xd100, AREA + 0xd200);
    e.mem.set_u32(i1, ACTOR2);
    let b1 = body_named(e, AREA + 0xe000, b"Other");
    e.mem.set_u32(i1 + 0x1c, b1);
    e.mem.set_u32(i2, ACTOR);
    e.mem.set_u32(i3, ACTOR);
    let b3 = body_named(e, AREA + 0xe800, b"Bip01 Head");
    e.mem.set_u32(i3 + 0x1c, b3);
    let (n2, n3) = (AREA + 0xd800, AREA + 0xd810);
    e.mem.set_u32(PROJ + 0x88, i1);
    e.mem.set_u32(PROJ + 0x8c, n2);
    e.mem.set_u32(n2, i2);
    e.mem.set_u32(n2 + 4, n3);
    e.mem.set_u32(n3, i3);
    e.mem.set_u32(n3 + 4, 0);
}

fn body_named(e: &mut Engine, at: u32, name: &[u8]) -> u32 {
    let node = at + 0x200;
    let t = text(e, at + 0x400, name);
    e.mem.set_u32(node + 8, t);
    let props = at + 0x300;
    e.mem.set_u32(props, KEY_AV_OBJECT);
    e.mem.set_u32(props + 8, node);
    e.mem.set_u32(at + 0x78, props);
    e.mem.set_u32(at + 0x7c, 1);
    at
}

fn hit_node(e: &mut Engine, this: u32) -> Vec<u8> {
    let r = run(e, 0x28d2, this, &[]);
    string_result(e, r)
}

#[test]
fn hit_node_is_the_first_body_on_this_actor() {
    let mut e = setup();
    hit_scene(&mut e);
    assert_eq!(hit_node(&mut e, ACTOR), b"Bip01 Head");
    // A null name is "NULL".
    e.mem.set_u32(AREA + 0xe800 + 0x200 + 8, 0);
    assert_eq!(hit_node(&mut e, ACTOR), b"NULL");
}

#[test]
fn hit_node_needs_a_projectile_and_a_hit() {
    let mut e = setup();
    hit_scene(&mut e);
    // The hit's projectile is not a projectile.
    e.mem.set_u32(PROJ, PLAIN_VT);
    assert_eq!(hit_node(&mut e, ACTOR), b"NULL");
    e.mem.set_u32(PROJ, PROJ_VT);
    e.mem.set_u32(HIT + 8, 0);
    assert_eq!(hit_node(&mut e, ACTOR), b"NULL");
    e.mem.set_u32(HIT + 8, PROJ);
    e.mem.set_u32(AREA + 0xf000, 0);
    assert_eq!(hit_node(&mut e, ACTOR), b"NULL");
    e.mem.set_u32(AREA + 0xf000, HIT);
    assert_eq!(hit_node(&mut e, PLAIN), b"NULL");
    // Another actor's impact is that actor's.
    e.mem.set_u32(ACTOR2 + 0x68, PROCESS);
    assert_eq!(hit_node(&mut e, ACTOR2), b"Other");
}

// ---- VATS, ash piles, the player's 3D ----

#[test]
fn vats_targetable_flips_the_flag_and_the_hooks_users() {
    use engine_core::extend::Hook;
    let mut e = setup();
    assert_eq!(run(&mut e, 0x2854, ACTOR, &[n(0)]), 0.0);
    assert_eq!(e.mem.u8(ACTOR + 0x106), 2);
    assert_eq!(e.hook_users(Hook::AddVATSTarget), 1);
    // Asked again: already so.
    run(&mut e, 0x2854, ACTOR, &[n(0)]);
    assert_eq!(e.hook_users(Hook::AddVATSTarget), 1);
    e.mem.set_u8(ACTOR + 0x106, 2 | 0x41);
    run(&mut e, 0x2854, ACTOR, &[n(5)]);
    assert_eq!(e.mem.u8(ACTOR + 0x106), 0x41, "only bit 1 flips");
    assert_eq!(e.hook_users(Hook::AddVATSTarget), 0);
    run(&mut e, 0x2854, ACTOR, &[n(1)]);
    assert_eq!(e.hook_users(Hook::AddVATSTarget), 0);
    // Not an actor: nothing.
    run(&mut e, 0x2854, PLAIN, &[n(0)]);
    assert_eq!(e.mem.u8(PLAIN + 0x106), 0);
    assert_eq!(e.hook_users(Hook::AddVATSTarget), 0);
}

const ATTACH_ASH_PILE_FN: u32 = 0x005d_b870;

fn alt_ash_pile(e: &mut Engine) -> u32 {
    e.ext_free::<crate::game_patches::gh2::AltAshPile>()
        .0
        .load(std::sync::atomic::Ordering::Relaxed)
}

#[test]
fn attach_ash_pile_ex_keeps_the_form_and_runs_the_games_command() {
    let mut e = setup();
    let pile = AREA + 0x6000;
    e.mem.set_u8(pile + 4, 0x15);
    e.mem.set_u8(pile + 6, 0x20);
    set_ref(&mut e, 1, pile);
    e.mem.set_u32(AREA + 0xf010, 1);
    standin!(&mut e, ATTACH_ASH_PILE_FN, AREA + 0xf010);
    let mut words = crate::test_support::line(&mut e, 0x2840, &[r(1)]);
    words[2] = ACTOR;
    let execute = crate::test_support::execute_of(&e, 0x2840);
    e.call(execute, &words);
    assert_eq!(alt_ash_pile(&mut e), pile);
    assert_eq!(called(ATTACH_ASH_PILE_FN), vec![words.to_vec()]);
    // The game's answer is the command's.
    e.mem.set_u32(AREA + 0xf010, 0);
    assert_eq!(e.call(execute, &words).eax & 0xff, 0);
}

#[test]
fn attach_ash_pile_ex_needs_an_actor_and_an_ash_pile_form() {
    let mut e = setup();
    let pile = AREA + 0x6000;
    e.mem.set_u8(pile + 4, 0x15);
    e.mem.set_u8(pile + 6, 0x1f);
    set_ref(&mut e, 1, pile);
    standin!(&mut e, ATTACH_ASH_PILE_FN, AREA + 0xf010);
    assert_eq!(run(&mut e, 0x2840, ACTOR, &[r(1)]), 0.0);
    e.mem.set_u8(pile + 6, 0x20);
    assert_eq!(run(&mut e, 0x2840, PLAIN, &[r(1)]), 0.0);
    assert!(called(ATTACH_ASH_PILE_FN).is_empty());
    assert_eq!(alt_ash_pile(&mut e), 0);
}

const QUEUE_REFERENCE: u32 = 0x0044_4850;
const MODEL_LOADER_AT: u32 = 0x011c_3b3c;

#[test]
fn update_player_3d_queues_the_player_once() {
    let mut e = setup();
    map_once(&mut e, 0x011c_3000, 0x1000);
    let player = AREA + 0x6000;
    let loaded = AREA + 0x6800;
    e.mem.set_u32(player + 0xc, 0x14);
    e.mem.set_u32(player + 0x64, loaded);
    e.mem.set_u32(loaded + 0x14, AREA + 0x6c00);
    e.mem.set_u32(THE_PLAYER_AT, player);
    e.mem.set_u32(MODEL_LOADER_AT, 0x0bad_0001);
    standin!(&mut e, QUEUE_REFERENCE, AREA + 0xf020);
    assert_eq!(run(&mut e, 0x28c7, 0, &[]), 0.0);
    assert_eq!(e.mem.u8(player + 0x5f) & 1, 1);
    assert_eq!(
        called(QUEUE_REFERENCE),
        vec![vec![0x0bad_0001, player, 1, 0]]
    );
    // Already marked: not again.
    run(&mut e, 0x28c7, 0, &[]);
    assert_eq!(called(QUEUE_REFERENCE).len(), 1);
    // No player: nothing.
    e.mem.set_u32(THE_PLAYER_AT, 0);
    run(&mut e, 0x28c7, 0, &[]);
    assert_eq!(called(QUEUE_REFERENCE).len(), 1);
}

// ---- PushActorAwayAlt ----

const ACTOR_VALUE_FN: u32 = 0x0066_ef50;
const PUSH_AWAY: u32 = STANDIN + 0x10;

fn push_scene(e: &mut Engine) {
    map_once(e, 0x011c_e000, 0x2000);
    e.mem.set_f32(0x011c_ea6c, 0.5);
    e.mem.set_f32(0x011c_e664, 1.0);
    e.mem.set_f32(0x011c_f9c0, 2.0);
    e.mem.set_f32(0x011c_fa20, 3.0);
    e.register(ACTOR_VALUE_FN, |_, w| {
        record(ACTOR_VALUE_FN, w);
        Ret {
            st0: 6.0,
            ..Ret::default()
        }
    });
    e.mem.set_u32(PROC_VT + 0x418, PUSH_AWAY);
    standin!(e, PUSH_AWAY, AREA + 0xf030);
    set4(e, ACTOR + 0x30, [10.0, 20.0, 30.0, 0.0]);
}

/// JIP's `AdjustPushForce` with the scene's settings and actor value 6.
fn adjusted(force: f32) -> f32 {
    let scale = 6.0f32 * 10.0 * 0.5 + 1.0;
    let offset = f32::from_bits((force.to_bits() & 0x8000_0000) ^ 2.0f32.to_bits());
    (force * 3.0 + offset) * scale
}

#[test]
fn push_actor_away_alt_relative_and_absolute() {
    let mut e = setup();
    push_scene(&mut e);
    let args = |force: f64, abs: i32| vec![z(force), z(1.0), z(-2.0), z(0.5), n(abs)];
    assert_eq!(run(&mut e, 0x288a, ACTOR, &args(4.0, 0)), 0.0);
    let p = |x: f32| x.to_bits();
    assert_eq!(called(ACTOR_VALUE_FN), vec![vec![ACTOR + 0xa4, 0xa]]);
    assert_eq!(
        called(PUSH_AWAY),
        vec![vec![
            PROCESS,
            ACTOR,
            p(11.0),
            p(18.0),
            p(30.5),
            p(adjusted(4.0))
        ]]
    );
    run(&mut e, 0x288a, ACTOR, &args(-1.5, 1));
    assert_eq!(
        called(PUSH_AWAY)[1],
        vec![PROCESS, ACTOR, p(1.0), p(-2.0), p(0.5), p(adjusted(-1.5))]
    );
    // Only the force given: the point is the actor's own.
    run(&mut e, 0x288a, ACTOR, &[z(2.0)]);
    assert_eq!(
        called(PUSH_AWAY)[2],
        vec![PROCESS, ACTOR, p(10.0), p(20.0), p(30.0), p(adjusted(2.0))]
    );
}

#[test]
fn push_actor_away_alt_needs_a_high_process_actor() {
    let mut e = setup();
    push_scene(&mut e);
    e.mem.set_u8(PROCESS + 0x28, 1);
    run(&mut e, 0x288a, ACTOR, &[z(4.0)]);
    e.mem.set_u8(PROCESS + 0x28, 0);
    e.mem.set_u32(PLAIN + 0x68, PROCESS);
    run(&mut e, 0x288a, PLAIN, &[z(4.0)]);
    e.mem.set_u32(ACTOR2 + 0x68, 0);
    run(&mut e, 0x288a, ACTOR2, &[z(4.0)]);
    assert!(called(PUSH_AWAY).is_empty());
    assert!(called(ACTOR_VALUE_FN).is_empty());
}

// ---- DismemberLimb ----

const DISMEMBERED_FN: u32 = 0x0057_3090;
const ESSENTIAL_FN: u32 = 0x0087_f3d0;
const SLOT_1A0: u32 = STANDIN + 0x20;
const BPD_FN: u32 = STANDIN + 0x30;
const LIMB_TEST_FN: u32 = 0x0045_6630;
const SET_DISMEMBERED_FN: u32 = 0x0057_2fc0;
const PIECE_FN: u32 = 0x008b_52a0;
const A_DISMEMBERED: u32 = AREA + 0xf100;
const A_ESSENTIAL: u32 = AREA + 0xf104;
const A_1A0: u32 = AREA + 0xf108;
const A_BPD: u32 = AREA + 0xf10c;
const A_LIMB: u32 = AREA + 0xf110;
const BPD: u32 = AREA + 0xa800;
const PART: u32 = AREA + 0xa900;
const LIMB: u32 = AREA + 0xaa00;

fn dismember_scene(e: &mut Engine) {
    standin!(e, DISMEMBERED_FN, A_DISMEMBERED);
    standin!(e, ESSENTIAL_FN, A_ESSENTIAL);
    e.mem.set_u32(ACTOR_VT + 0x1a0, SLOT_1A0);
    standin!(e, SLOT_1A0, A_1A0);
    e.mem.set_u32(BASE_VT + 0x180, BPD_FN);
    standin!(e, BPD_FN, A_BPD);
    standin!(e, LIMB_TEST_FN, A_LIMB);
    standin!(e, SET_DISMEMBERED_FN, AREA + 0xf114);
    standin!(e, PIECE_FN, AREA + 0xf118);
    e.mem.set_u32(A_BPD, BPD);
    e.mem.set_u32(BPD + 0x34 + 4 * 3, PART);
    e.mem.set_u8(PART + 0x60, 8);
    e.mem.set_u8(PART + 0x63, 0x1b);
    e.mem.set_u32(PROCESS + 0x1dc + 4 * 3, LIMB);
}

fn dismembered(_: &Engine) -> (Vec<Vec<u32>>, Vec<Vec<u32>>) {
    (called(SET_DISMEMBERED_FN), called(PIECE_FN))
}

#[test]
fn dismember_limb_sets_the_part_and_makes_the_piece() {
    let mut e = setup();
    dismember_scene(&mut e);
    assert_eq!(run(&mut e, 0x2833, ACTOR, &[n(3), n(7)]), 0.0);
    assert_eq!(called(DISMEMBERED_FN), vec![vec![ACTOR, 3]]);
    assert_eq!(called(ESSENTIAL_FN), vec![vec![ACTOR]]);
    assert_eq!(called(SLOT_1A0), vec![vec![ACTOR, 0]]);
    assert_eq!(called(BPD_FN), vec![vec![BASE]]);
    assert_eq!(called(LIMB_TEST_FN), vec![vec![LIMB, 1]]);
    assert_eq!(
        dismembered(&e),
        (
            vec![vec![ACTOR, 3, 0, 1, 3, 1]],
            vec![vec![ACTOR, 0x1b, 3, 1, 1, 1, 0]]
        )
    );
    // Not exploded; flag 1 alone also counts.
    e.mem.set_u8(PART + 0x60, 1);
    run(&mut e, 0x2833, ACTOR, &[n(3)]);
    assert_eq!(dismembered(&e).0[1], vec![ACTOR, 3, 0, 1, 3, 0]);
    assert_eq!(dismembered(&e).1[1], vec![ACTOR, 0x1b, 3, 0, 1, 1, 0]);
}

#[test]
fn dismember_limb_refusals() {
    let mut e = setup();
    dismember_scene(&mut e);
    let none = |e: &mut Engine, what: &str| {
        let before = dismembered(e);
        run(e, 0x2833, ACTOR, &[n(3), n(1)]);
        assert_eq!(dismembered(e), before, "{what}");
        assert!(before.0.is_empty(), "{what}");
    };
    // Part 14 is past 13: no call at all.
    run(&mut e, 0x2833, ACTOR, &[n(14)]);
    assert!(called(DISMEMBERED_FN).is_empty());
    e.mem.set_u32(A_DISMEMBERED, 1);
    none(&mut e, "already dismembered");
    e.mem.set_u32(A_DISMEMBERED, 0x100);
    e.mem.set_u32(A_ESSENTIAL, 1);
    none(&mut e, "essential");
    e.mem.set_u32(A_ESSENTIAL, 0);
    e.mem.set_u32(A_1A0, 1);
    none(&mut e, "slot 0x1A0");
    e.mem.set_u32(A_1A0, 0);
    e.mem.set_u32(A_BPD, 0);
    none(&mut e, "no body part data");
    e.mem.set_u32(A_BPD, BPD);
    e.mem.set_u32(BPD + 0x34 + 4 * 3, 0);
    none(&mut e, "no part");
    e.mem.set_u32(BPD + 0x34 + 4 * 3, PART);
    e.mem.set_u8(PART + 0x60, 0x06);
    none(&mut e, "neither severable nor explodable");
    e.mem.set_u8(PART + 0x60, 9);
    e.mem.set_u8(PROCESS + 0x28, 1);
    none(&mut e, "not a high process");
    e.mem.set_u8(PROCESS + 0x28, 0);
    e.mem.set_u32(PROCESS + 0x1dc + 4 * 3, 0);
    none(&mut e, "no limb");
    e.mem.set_u32(PROCESS + 0x1dc + 4 * 3, LIMB);
    e.mem.set_u32(A_LIMB, 1);
    none(&mut e, "limb flag");
    e.mem.set_u32(A_LIMB, 0x100);
    run(&mut e, 0x2833, PLAIN, &[n(3)]);
    assert!(dismembered(&e).0.is_empty(), "not an actor");
    // Everything in place again (answers with only high bits set count as false).
    run(&mut e, 0x2833, ACTOR, &[n(3)]);
    assert_eq!(dismembered(&e).0.len(), 1);
}

// ---- map markers ----

const PLAYER_REF: u32 = AREA + 0x6000;

fn marker_scene(e: &mut Engine) -> u32 {
    e.mem.set_u32(THE_PLAYER_AT, PLAYER_REF);
    // Two teammates: ACTOR2 and another.
    let tm2 = AREA + 0x6800;
    e.mem.set_u32(PLAYER_REF + 0x5fc, ACTOR2);
    e.mem.set_u32(PLAYER_REF + 0x600, tm2);
    e.mem.set_u32(tm2, AREA + 0x6900);
    e.mem.set_u32(tm2 + 4, 0);
    // ACTOR persistent; a plain reference with an activator base.
    e.mem.set_u32(ACTOR + 8, 0x400);
    let acti = AREA + 0x6a00;
    e.mem.set_u8(acti + 4, 0x15);
    e.mem.set_u32(PLAIN + 0x20, acti);
    e.ext::<crate::Jip>().ui_hooks.map_markers_list
}

fn listed(e: &Engine, list: u32) -> Vec<u32> {
    super::super::list_items(e, Ptr::new(list))
}

fn markers_users(e: &Engine) -> u32 {
    e.hook_users(engine_core::extend::Hook::CreateMapMarkers)
}

#[test]
fn add_ref_map_marker_links_the_teammates_and_counts() {
    let mut e = setup();
    let list = marker_scene(&mut e);
    assert_ne!(list, 0);
    assert_eq!(run(&mut e, 0x27e9, ACTOR, &[]), 0.0);
    assert_eq!(listed(&e, list), vec![ACTOR, ACTOR2, AREA + 0x6900]);
    assert_eq!(e.mem.u32(list + 4), PLAYER_REF + 0x5fc);
    assert_eq!(markers_users(&e), 1);
    run(&mut e, 0x27e9, PLAIN, &[]);
    assert_eq!(listed(&e, list), vec![PLAIN, ACTOR, ACTOR2, AREA + 0x6900]);
    assert_eq!(markers_users(&e), 2);
    // Already listed (an own marker, or a teammate past them): nothing.
    run(&mut e, 0x27e9, ACTOR, &[]);
    e.mem.set_u32(ACTOR2 + 8, 0x400);
    run(&mut e, 0x27e9, ACTOR2, &[]);
    assert_eq!(listed(&e, list).len(), 4);
    assert_eq!(markers_users(&e), 2);
    assert_eq!(run(&mut e, 0x27eb, PLAIN, &[]), 1.0);
    assert_eq!(run(&mut e, 0x27eb, ACTOR, &[]), 1.0);
    assert_eq!(
        run(&mut e, 0x27eb, ACTOR2, &[]),
        0.0,
        "a teammate is not an own marker"
    );
}

#[test]
fn add_ref_map_marker_wants_a_reference_valid_for_hooks() {
    let mut e = setup();
    let list = marker_scene(&mut e);
    // A non-persistent actor, an inventory item's reference, a projectile.
    e.mem.set_u32(ACTOR + 8, 0);
    run(&mut e, 0x27e9, ACTOR, &[]);
    let weap = AREA + 0x6b00;
    e.mem.set_u8(weap + 4, 0x28);
    e.mem.set_u32(PLAIN + 0x20, weap);
    run(&mut e, 0x27e9, PLAIN, &[]);
    let proj = AREA + 0x6c00;
    e.mem.set_u32(proj, PROJ_VT);
    e.mem.set_u32(proj + 0x20, AREA + 0x6a00);
    e.mem.set_u8(AREA + 0x6a00 + 4, 0x15);
    run(&mut e, 0x27e9, proj, &[]);
    assert_eq!(listed(&e, list), vec![0]);
    assert_eq!(markers_users(&e), 0);
    assert_eq!(run(&mut e, 0x27eb, ACTOR, &[]), 0.0);
}

#[test]
fn remove_ref_map_marker_unlinks_and_the_last_empties() {
    let mut e = setup();
    let list = marker_scene(&mut e);
    run(&mut e, 0x27e9, ACTOR, &[]);
    run(&mut e, 0x27e9, PLAIN, &[]);
    let third = AREA + 0x7800;
    e.mem.set_u32(third, PLAIN_VT);
    e.mem.set_u32(third + 0x20, AREA + 0x6a00);
    run(&mut e, 0x27e9, third, &[]);
    assert_eq!(listed(&e, list)[..3], [third, PLAIN, ACTOR]);
    // A teammate past the own markers is not removed.
    assert_eq!(run(&mut e, 0x27ea, ACTOR2, &[]), 0.0);
    assert_eq!(markers_users(&e), 3);
    // The middle one, then the first, then the last.
    run(&mut e, 0x27ea, PLAIN, &[]);
    assert_eq!(listed(&e, list), vec![third, ACTOR, ACTOR2, AREA + 0x6900]);
    assert_eq!(freed().len(), 1);
    run(&mut e, 0x27ea, third, &[]);
    assert_eq!(listed(&e, list), vec![ACTOR, ACTOR2, AREA + 0x6900]);
    assert_eq!(markers_users(&e), 1);
    assert_eq!(run(&mut e, 0x27eb, ACTOR, &[]), 1.0);
    run(&mut e, 0x27ea, ACTOR, &[]);
    assert_eq!((e.mem.u32(list), e.mem.u32(list + 4)), (0, 0));
    assert_eq!(markers_users(&e), 0);
    assert_eq!(run(&mut e, 0x27eb, ACTOR, &[]), 0.0);
    // None left: nothing to remove.
    run(&mut e, 0x27ea, ACTOR, &[]);
    assert_eq!(markers_users(&e), 0);
}

#[test]
fn map_markers_reset_before_a_load() {
    let mut e = setup();
    let list = marker_scene(&mut e);
    run(&mut e, 0x27e9, ACTOR, &[]);
    run(&mut e, 0x27e9, PLAIN, &[]);
    assert_eq!(markers_users(&e), 2);
    super::super::jip_fn_misc_ref::pre_load(&mut e);
    assert_eq!((e.mem.u32(list), e.mem.u32(list + 4)), (0, 0));
    assert_eq!(freed().len(), 1, "the own nodes but the first freed");
    assert_eq!(markers_users(&e), 0);
    assert_eq!(run(&mut e, 0x27eb, ACTOR, &[]), 0.0);
    // The count starts again from 0: the next marker links the teammates again.
    run(&mut e, 0x27e9, PLAIN, &[]);
    assert_eq!(listed(&e, list), vec![PLAIN, ACTOR2, AREA + 0x6900]);
}

// ---- GetSessionTime ----

const TICKS_FN: u32 = 0x0045_7fe0;
const A_TICKS: u32 = AREA + 0xf200;

#[test]
fn session_time_counts_from_the_deferred_start() {
    let mut e = setup();
    standin!(&mut e, TICKS_FN, A_TICKS);
    e.mem.set_u32(A_TICKS, 5000);
    // Before the start: from 0.
    assert_eq!(run(&mut e, 0x272e, 0, &[]), 5000.0 * 0.001);
    super::super::jip_fn_utility::deferred_init(&mut e);
    e.mem.set_u32(A_TICKS, 7500);
    assert_eq!(run(&mut e, 0x272e, 0, &[]), 2500.0 * 0.001);
    // The clock wraps; the difference is read as signed.
    e.mem.set_u32(A_TICKS, 0xffff_fff0);
    super::super::jip_fn_utility::deferred_init(&mut e);
    e.mem.set_u32(A_TICKS, 0x10);
    assert_eq!(run(&mut e, 0x272e, 0, &[]), 32.0 * 0.001);
    e.mem.set_u32(A_TICKS, 0xffff_ff00);
    assert_eq!(run(&mut e, 0x272e, 0, &[]), -240.0 * 0.001);
    assert!(called(TICKS_FN).iter().all(|w| w.is_empty()));
}

// ---- PlaySound3DNode ----

const INIT_SOUND_FN: u32 = 0x00ae_5870;
const SET_POS_FN: u32 = 0x00ad_8b60;
const SET_NODE_FN: u32 = 0x00ad_8f20;
const PLAY_FN: u32 = 0x00ad_8830;
const A_KEY: u32 = AREA + 0xf300;

fn sound_scene(e: &mut Engine) -> (u32, u32) {
    let sound = AREA + 0x6000;
    e.mem.set_u8(sound + 4, 0x0d);
    e.mem.set_u32(sound + 0xc, 0x0001_2345);
    set_ref(e, 1, sound);
    let root = AREA + 0x7000;
    let head = AREA + 0x7200;
    let other = AREA + 0x7400;
    e.mem.set_u32(AREA + 0xf400 + 0xc, 0x0068_15c0);
    for (n, name) in [(root, b"Root" as &[u8]), (head, b"Head"), (other, b"Head")] {
        e.mem.set_u32(n, AREA + 0xf400);
        let t = text(e, n + 0x100, name);
        e.mem.set_u32(n + 8, t);
    }
    // Root's children: Head; `other` (same name) is not under it.
    let kids = AREA + 0x7600;
    e.mem.set_u32(kids, head);
    e.mem.set_u32(root + 0xa0, kids);
    e.mem.set_u16(root + 0xa6, 1);
    set4(e, head + 0x8c, [1.0, 2.0, 3.0, 4.0]);
    set4(e, root + 0x8c, [7.0, 8.0, 9.0, 0.0]);
    let loaded = AREA + 0x7800;
    e.mem.set_u32(loaded + 0x14, root);
    e.mem.set_u32(PLAIN + 0x64, loaded);
    e.register(INIT_SOUND_FN, |e, w| {
        record(INIT_SOUND_FN, w);
        assert_eq!(
            [e.mem.u32(w[1]), e.mem.u32(w[1] + 4), e.mem.u32(w[1] + 8)],
            [u32::MAX, 0, 0],
            "a fresh handle"
        );
        let key = e.mem.u32(A_KEY);
        e.mem.set_u32(w[1], key);
        eax(w[1])
    });
    standin!(e, SET_POS_FN, AREA + 0xf310);
    standin!(e, SET_NODE_FN, AREA + 0xf310);
    standin!(e, PLAY_FN, AREA + 0xf310);
    (sound, head)
}

#[test]
fn play_sound_3d_node_plays_at_the_named_node() {
    let mut e = setup();
    let (_, head) = sound_scene(&mut e);
    e.mem.set_u32(A_KEY, 77);
    assert_eq!(run(&mut e, 0x291e, PLAIN, &[r(1), s(b"Head")]), 0.0);
    let init = called(INIT_SOUND_FN);
    assert_eq!(init.len(), 1);
    let handle = init[0][1];
    assert_eq!(init[0], vec![0x011f_6ef0, handle, 0x0001_2345, 0x102]);
    let p = |x: f32| x.to_bits();
    assert_eq!(
        called(SET_POS_FN),
        vec![vec![handle, p(1.0), p(2.0), p(3.0)]]
    );
    assert_eq!(called(SET_NODE_FN), vec![vec![handle, head]]);
    assert_eq!(called(PLAY_FN), vec![vec![handle, 0]]);
    // An empty name: the root node.
    run(&mut e, 0x291e, PLAIN, &[r(1), s(b"")]);
    assert_eq!(called(SET_POS_FN)[1][1..], [p(7.0), p(8.0), p(9.0)]);
}

#[test]
fn play_sound_3d_node_needs_a_key_and_a_node() {
    let mut e = setup();
    sound_scene(&mut e);
    e.mem.set_u32(A_KEY, u32::MAX);
    run(&mut e, 0x291e, PLAIN, &[r(1), s(b"Head")]);
    assert_eq!(called(INIT_SOUND_FN).len(), 1);
    assert!(called(SET_POS_FN).is_empty() && called(PLAY_FN).is_empty());
    e.mem.set_u32(A_KEY, 5);
    run(&mut e, 0x291e, PLAIN, &[r(1), s(b"Hand")]);
    run(&mut e, 0x291e, ACTOR, &[r(1), s(b"Head")]);
    assert_eq!(called(INIT_SOUND_FN).len(), 1, "no node, no 3D: nothing");
    assert!(called(PLAY_FN).is_empty());
}

// ---- GetGroundMaterial ----

const GET_LAND_FN: u32 = 0x0054_6fb0;
const COORD_FN: u32 = 0x0053_b550;
const TEXTURE_FN: u32 = 0x0053_a630;
const LAND: u32 = AREA + 0x6000;
const TEXTURE: u32 = AREA + 0x6400;
const CELL: u32 = AREA + 0x6800;

fn ground_scene(e: &mut Engine) {
    e.mem.set_u32(ACTOR + 0x40, CELL);
    set4(e, ACTOR + 0x30, [1.0, 2.0, 3.0, 0.0]);
    e.mem.set_u32(AREA + 0xf500, LAND);
    standin!(e, GET_LAND_FN, AREA + 0xf500);
    e.register(COORD_FN, |e, w| {
        record(COORD_FN, w);
        // The quadrant word (+0x18) and the vertex word (+0x3C): JIP takes their low byte and
        // low 16 bits.
        e.mem.set_u32(w[1] + 0x18, 0x0000_0103);
        e.mem.set_u32(w[1] + 0x3c, 0x0002_0457);
        eax(0)
    });
    e.mem.set_u32(AREA + 0xf504, TEXTURE);
    standin!(e, TEXTURE_FN, AREA + 0xf504);
    e.mem.set_u8(TEXTURE + 0x1c, 5);
}

fn ground(e: &mut Engine, this: u32) -> (f64, f64) {
    let exec = run(e, 0x28e0, this, &[]);
    let eval = e.mem.u32(nvse::table::record_of(e, 0x28e0) + 0x20);
    e.mem.set_f64(RESULT, 7.0);
    e.call(eval, &[this, 0, 0, RESULT]);
    (exec, e.mem.f64(RESULT))
}

#[test]
fn ground_material_from_the_support_material() {
    let mut e = setup();
    ground_scene(&mut e);
    let table = super::super::jip_fn_projectile::MATERIAL_CONVERT;
    e.mem.set_u32(CTRL + 0x434, 9);
    let want = f64::from(table[9]);
    assert_eq!(ground(&mut e, ACTOR), (want, want));
    e.mem.set_u32(CTRL + 0x434, 31);
    let want = f64::from(table[31]);
    assert_eq!(ground(&mut e, ACTOR), (want, want));
    e.mem.set_u32(CTRL + 0x434, 32);
    assert_eq!(ground(&mut e, ACTOR), (-1.0, -1.0));
    assert!(called(GET_LAND_FN).is_empty());
    // In the air (state bit 1): -1.
    e.mem.set_u32(CTRL + 0x434, 9);
    e.mem.set_u32(CTRL + 0x3f0, 2);
    assert_eq!(ground(&mut e, ACTOR), (-1.0, -1.0));
    e.mem.set_u32(CTRL + 0x3f0, 1);
    assert_eq!(ground(&mut e, ACTOR).0, f64::from(table[9]));
    // No controller or not an actor: -1.
    assert_eq!(ground(&mut e, PLAIN), (-1.0, -1.0));
    e.mem.set_u8(PROCESS + 0x28, 2);
    assert_eq!(ground(&mut e, ACTOR), (-1.0, -1.0));
}

#[test]
fn ground_material_on_land_is_the_texture_under_the_actor() {
    let mut e = setup();
    ground_scene(&mut e);
    e.mem.set_u32(CTRL + 0x434, 0x24);
    let table = super::super::jip_fn_projectile::MATERIAL_CONVERT;
    let want = f64::from(table[5]);
    assert_eq!(ground(&mut e, ACTOR), (want, want));
    assert_eq!(called(GET_LAND_FN)[0], vec![CELL]);
    let coord = &called(COORD_FN)[0];
    assert_eq!((coord[0], coord[2], coord[3]), (LAND, ACTOR + 0x30, 1));
    assert_eq!(called(TEXTURE_FN)[0], vec![LAND, 3, 0x457]);
    // A material byte past 31: -1.
    e.mem.set_u8(TEXTURE + 0x1c, 0x20);
    assert_eq!(ground(&mut e, ACTOR), (-1.0, -1.0));
    // No texture, no land, no cell: -1.
    e.mem.set_u8(TEXTURE + 0x1c, 5);
    e.mem.set_u32(AREA + 0xf504, 0);
    assert_eq!(ground(&mut e, ACTOR), (-1.0, -1.0));
    e.mem.set_u32(AREA + 0xf504, TEXTURE);
    e.mem.set_u32(AREA + 0xf500, 0);
    assert_eq!(ground(&mut e, ACTOR), (-1.0, -1.0));
    let lands = called(GET_LAND_FN).len();
    e.mem.set_u32(ACTOR + 0x40, 0);
    assert_eq!(ground(&mut e, ACTOR), (-1.0, -1.0));
    assert_eq!(called(GET_LAND_FN).len(), lands, "no cell: no land asked");
}

// ---- MoveAwayFromPlayer ----

const CURRENT_PACKAGE: u32 = STANDIN + 0x40;
const SLOT_28: u32 = STANDIN + 0x50;
const SET_PACKAGE: u32 = STANDIN + 0x60;
const BACKUP_CTOR_FN: u32 = 0x009e_d030;
const BACKUP_INIT_FN: u32 = 0x009e_d0d0;
const A_RUNNING: u32 = AREA + 0xf600;

fn backup_scene(e: &mut Engine) -> u32 {
    let player = AREA + 0x6000;
    e.mem.set_u32(THE_PLAYER_AT, player);
    set4(e, player + 0x30, [100.0, 200.0, 300.0, 0.0]);
    e.mem.set_u32(PROC_VT + 0x27c, CURRENT_PACKAGE);
    standin!(e, CURRENT_PACKAGE, A_RUNNING);
    e.mem.set_u32(PROC_VT + 0x28, SLOT_28);
    standin!(e, SLOT_28, AREA + 0xf610);
    e.mem.set_u32(ACTOR_VT + 0x2f4, SET_PACKAGE);
    standin!(e, SET_PACKAGE, AREA + 0xf610);
    e.register(BACKUP_CTOR_FN, |e, w| {
        record(BACKUP_CTOR_FN, w);
        e.mem.set_u32(w[0] + 0x1c, 0x10);
        e.mem.set_u32(w[0] + 0x80, 0);
        eax(w[0])
    });
    standin!(e, BACKUP_INIT_FN, AREA + 0xf610);
    player
}

#[test]
fn move_away_from_player_starts_a_backup_package() {
    let mut e = setup();
    let player = backup_scene(&mut e);
    let cell = AREA + 0x6800;
    e.mem.set_u32(ACTOR + 0x40, cell);
    assert_eq!(run(&mut e, 0x2781, ACTOR, &[]), 0.0);
    assert_eq!(allocations(), vec![0x8c]);
    let package = called(BACKUP_CTOR_FN)[0][0];
    assert_eq!(
        called(BACKUP_INIT_FN),
        vec![vec![package, ACTOR, cell, player]]
    );
    assert_eq!(e.mem.u32(package + 0x18), 0x30);
    assert_eq!(e.mem.u32(package + 0x1c), 0x10 | (1 << 26));
    assert_eq!(e.mem.u32(package + 0x80), 0, "a cell: the point is left");
    assert_eq!(called(SLOT_28), vec![vec![PROCESS]]);
    assert_eq!(called(SET_PACKAGE), vec![vec![ACTOR, package, 1, 1]]);
    // No cell: the player's position is the point.
    e.mem.set_u32(ACTOR + 0x40, 0);
    run(&mut e, 0x2781, ACTOR, &[]);
    let package = called(BACKUP_CTOR_FN)[1][0];
    assert_eq!(called(BACKUP_INIT_FN)[1], vec![package, ACTOR, 0, player]);
    let point: Vec<f32> = (0..3).map(|i| e.mem.f32(package + 0x80 + 4 * i)).collect();
    assert_eq!(point, [100.0, 200.0, 300.0]);
}

#[test]
fn move_away_from_player_refusals() {
    let mut e = setup();
    backup_scene(&mut e);
    for life in [3u32, 5] {
        e.mem.set_u32(ACTOR + 0x108, life);
        run(&mut e, 0x2781, ACTOR, &[]);
    }
    e.mem.set_u32(ACTOR + 0x108, 4);
    e.mem.set_u8(PROCESS + 0x28, 1);
    run(&mut e, 0x2781, ACTOR, &[]);
    e.mem.set_u8(PROCESS + 0x28, 0);
    let running = AREA + 0x6c00;
    e.mem.set_u32(A_RUNNING, running);
    e.mem.set_u8(running + 0x20, 0x1a);
    run(&mut e, 0x2781, ACTOR, &[]);
    run(&mut e, 0x2781, PLAIN, &[]);
    assert!(allocations().is_empty());
    assert!(called(SET_PACKAGE).is_empty());
    // Another running package type: it goes on.
    e.mem.set_u8(running + 0x20, 0x1b);
    run(&mut e, 0x2781, ACTOR, &[]);
    assert_eq!(called(SET_PACKAGE).len(), 1);
}
