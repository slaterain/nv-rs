//! Tests of the per-mod player controls through the commands' records: the per-mod sets, their
//! union in the engine's word, the immediate effects, the reset on a load and the reload wrap.

use super::super::test_support::*;
use super::*;
use std::cell::RefCell;

thread_local! {
    static CALLS: RefCell<Vec<(u32, Vec<u32>)>> = const { RefCell::new(Vec::new()) };
}

fn calls() -> Vec<(u32, Vec<u32>)> {
    CALLS.with(|c| std::mem::take(&mut *c.borrow_mut()))
}

fn eax(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

/// The player at `0x0200_1000` (its mover at `0x0200_2000`, whose flags are 0x401), a data
/// handler naming two mods, the calling script mod 1's; every game call the effects make
/// recorded.
const PC: u32 = 0x0200_1000;
const MOVER: u32 = 0x0200_2000;

fn setup() -> Engine {
    CALLS.with(|c| c.borrow_mut().clear());
    let mut e = engine_with_table();
    e.map(PLAYER & !0xfff, 0x1000);
    e.map(0x011c_3000, 0x1000);
    e.map(THIRD_PERSON_ZOOM & !0xfff, 0x1000);
    e.map(VATS_CAMERA & !0xfff, 0x1000);
    e.mem.set_u32(PLAYER, PC);
    e.mem.set_u32(PC + ACTOR_MOVER, MOVER);
    e.mem.set_u32(MOVER, MOVER + 0x400);
    e.mem.set_u32(MOVER + 0x400 + MOVER_GET_FLAGS, 0x0060_1000);
    e.register(0x0060_1000, |_, _| eax(0x401));
    e.register(UPDATE_HUD_STATE, record::<UPDATE_HUD_STATE>);
    e.register(SET_WANTS_WEAPON_OUT, record::<SET_WANTS_WEAPON_OUT>);
    e.register(UPDATE_CAMERA, record::<UPDATE_CAMERA>);
    e.register(SET_MOVE_FLAGS, record::<SET_MOVE_FLAGS>);
    e.register(AIM_WEAPON, record::<AIM_WEAPON>);
    e.register(SET_BLOCKING, record::<SET_BLOCKING>);
    // The calling script's first file is "ModA.esp".
    let file = 0x0200_3000;
    e.mem.write(file + 0x20, b"ModA.esp\0");
    e.mem.set_u32(SCRIPT + 0x10, file);
    e
}

/// A game function the effects call: its first three words recorded under its address.
fn record<const ADDR: u32>(_: &mut Engine, a: &[u32]) -> Ret {
    let mut v = a.to_vec();
    v.resize(3, 0);
    CALLS.with(|c| c.borrow_mut().push((ADDR, v[..3].to_vec())));
    eax(0)
}

fn as_mod(e: &mut Engine, name: &str) {
    let file = 0x0200_3000;
    e.mem.write(file + 0x20, format!("{name}\0").as_bytes());
}

/// `DisablePlayerControlsAlt`: the seven arguments (movement, Pip-Boy, fighting, POV default 1;
/// looking, rollover text, sneaking default 0) as the game's bits, kept for the calling mod and
/// in the word; the immediate effects only for controls not disabled already. A mod's flags in
/// any case of its name are one set. `EnablePlayerControlsAlt` takes them away from that mod
/// only, and the word is the union of what is left.
#[test]
fn per_mod_sets_and_their_union() {
    let mut e = setup();
    assert_eq!(e.controls_alt(), 0);
    call(&mut e, 0x1661, 0, &[]);
    assert_eq!(e.controls_alt(), VANILLA_DEFAULT);
    let made = calls();
    // Movement: the HUD's disabled-controls state; fighting: the weapon put away; POV: first
    // person and the camera (no player node: the zoom height 0).
    assert_eq!(
        made[0],
        (UPDATE_HUD_STATE, vec![HUD_PLAYER_DISABLED_CONTROLS, 0, 0])
    );
    assert_eq!(made[1], (SET_WANTS_WEAPON_OUT, vec![PC, 0, 0]));
    assert_eq!(made.len(), 2);
    // Again: nothing new, no effects.
    call(&mut e, 0x1661, 0, &[]);
    assert!(calls().is_empty());
    // Another mod disables sneaking and V.A.T.S. (the Ex form's bit field).
    as_mod(&mut e, "ModB.esp");
    call(&mut e, 0x1664, 0, &[n((SNEAKING | ENTER_VATS) as i32)]);
    assert_eq!(e.controls_alt(), VANILLA_DEFAULT | SNEAKING | ENTER_VATS);
    let made = calls();
    assert_eq!(made[0], (UPDATE_HUD_STATE, vec![HUD_RECALCULATE, 0, 0]));
    assert_eq!(made[1], (SET_MOVE_FLAGS, vec![MOVER, 0x001, 0]));
    // Mod B, as "MODB.ESP", enables everything of its own: mod A's set stays.
    as_mod(&mut e, "MODB.ESP");
    call(&mut e, 0x1665, 0, &[]);
    assert_eq!(e.controls_alt(), VANILLA_DEFAULT);
    // Mod A enables movement only (the seven arguments, enable defaults 1 for the last three).
    as_mod(&mut e, "ModA.esp");
    calls();
    call(
        &mut e,
        0x1662,
        0,
        &[n(1), n(0), n(0), n(0), n(0), n(0), n(0)],
    );
    assert_eq!(e.controls_alt(), PIPBOY | FIGHTING | POV);
    // The enabling effect is told mod A's whole set as it was: movement comes back.
    assert_eq!(calls(), [(UPDATE_HUD_STATE, vec![HUD_RECALCULATE, 0, 0])]);
}

/// `GetPlayerControlsDisabledAlt` and its `Ex` form: by the calling mod, any mod, any mod or the
/// game's own byte, the game only; another way answers 0.
#[test]
fn asking_what_is_disabled() {
    let mut e = setup();
    call(&mut e, 0x1664, 0, &[n(JUMPING as i32)]);
    e.mem.set_u8(PC + PLAYER_DISABLED, MOVEMENT as u8);
    let ask =
        |e: &mut Engine, how: i32, flags: u32| call(e, 0x1666, 0, &[n(how), n(flags as i32)]).1;
    assert_eq!(ask(&mut e, 0, JUMPING), 1.0);
    assert_eq!(ask(&mut e, 1, JUMPING), 1.0);
    assert_eq!(ask(&mut e, 1, MOVEMENT), 0.0);
    assert_eq!(ask(&mut e, 2, MOVEMENT), 1.0);
    assert_eq!(ask(&mut e, 3, MOVEMENT), 1.0);
    assert_eq!(ask(&mut e, 3, JUMPING), 0.0);
    assert_eq!(ask(&mut e, 4, JUMPING), 0.0);
    as_mod(&mut e, "Other.esp");
    assert_eq!(ask(&mut e, 0, JUMPING), 0.0);
    // The seven-argument form: movement only asked of any mod or the game.
    let got = call(
        &mut e,
        0x1663,
        0,
        &[n(2), n(1), n(0), n(0), n(0), n(0), n(0), n(0)],
    )
    .1;
    assert_eq!(got, 1.0);
}

/// A new game (and a load, which runs the same reset) forgets every mod's controls: the word is
/// 0 again and the next mod starts from nothing. As in xNVSE's `ResetOnLoad`, the word is
/// cleared before the enabling effects are told, so they find nothing of the mods' to take back
/// and the HUD is not told (the game's own load sets the HUD up again).
#[test]
fn a_new_game_resets_them() {
    let mut e = setup();
    call(&mut e, 0x1664, 0, &[n((MOVEMENT | RUNNING) as i32)]);
    calls();
    crate::saveload::new_game(&mut e);
    assert_eq!(e.controls_alt(), 0);
    assert!(state(&mut e).per_mod.is_empty());
    assert!(calls().is_empty());
    // Disabling movement again is a change once more: its effect runs.
    call(&mut e, 0x1664, 0, &[n(MOVEMENT as i32)]);
    assert_eq!(e.controls_alt(), MOVEMENT);
    assert_eq!(
        calls(),
        [(UPDATE_HUD_STATE, vec![HUD_PLAYER_DISABLED_CONTROLS, 0, 0])]
    );
}

/// `PlayerCharacter::Reload` through the wrap: with reloading disabled, an animated reload
/// (types 1 and 2) outside V.A.T.S. playback answers 0 and the game's function does not run;
/// type 0 (equipping a loaded weapon), V.A.T.S. playback, or nothing disabled reach it.
#[test]
fn reload_is_refused_while_disabled() {
    let mut e = setup();
    e.register(RELOAD_FN, |_, a| {
        CALLS.with(|c| c.borrow_mut().push((RELOAD_FN, a[..3].to_vec())));
        eax(1)
    });
    install(&mut e);
    let reload = |e: &mut Engine, kind: u32| {
        e.call(RELOAD_FN, &args![PC, 0x0200_4000u32, kind, 0u32, 0u32])
            .u32()
            & 0xff
    };
    assert_eq!(reload(&mut e, 1), 1);
    assert_eq!(calls().len(), 1);
    call(&mut e, 0x1664, 0, &[n(RELOAD as i32)]);
    calls();
    assert_eq!(reload(&mut e, 1), 0);
    assert_eq!(reload(&mut e, 2), 0);
    assert!(calls().is_empty());
    assert_eq!(reload(&mut e, 0), 1);
    assert_eq!(calls().len(), 1);
    e.mem.set_u32(VATS_CAMERA + VATS_MODE, VATS_PLAYBACK);
    assert_eq!(reload(&mut e, 1), 1);
    assert_eq!(calls().len(), 1);
    assert_eq!(e.wraps_at(RELOAD_FN), [Owner::Nvse]);
}
