//! Group GX's game patches: G29, G32, G49, G59, G65, H20 (with `ToggleMouseMovement`) and O14.

use super::*;
use crate::test_support::{call, engine, execute_of, int_arg};
use engine_core::extend::jip_patches;

const B: u32 = 0x0580_0000;
const LOG: u32 = B + 0xf000;
const ACTOR: u32 = B + 0x100;
const BASE: u32 = B + 0x200;
const CONTROLLER: u32 = B + 0x300;
const PROCESS_AT: u32 = B + 0x800;
const COLLISION: u32 = B + 0x1000;
const INPUT: u32 = B + 0x2000;
/// O14's world: the cinfo, its owner, the owner's reference, the shared shape and JIP's, the
/// player and its process word, the heap word handed out.
const INFO: u32 = B + 0x5000;
const OWNER: u32 = B + 0x5100;
const REFERENCE: u32 = B + 0x5200;
const SHARED: u32 = B + 0x5400;
const SHARED_HK: u32 = B + 0x5500;
const SHARED_VERTS: u32 = B + 0x5600;
const MADE: u32 = B + 0x6000;
const MADE_HK: u32 = B + 0x6100;
const MADE_VERTS: u32 = B + 0x6200;
const PLAYER: u32 = B + 0x7000;
const PLAYER_PROCESS: u32 = B + 0x8000;
const HEAP_WORD: u32 = B + 0x9000;

fn bump(e: &mut Engine, at: u32) {
    let n = e.mem.u32(at);
    e.mem.set_u32(at, n + 1);
}

/// JIP installed (or not) with the row on or off, and the stand-ins this file's handlers call.
fn world(installed: bool, row: bool) -> Engine {
    let mut e = engine();
    e.map(B, 0x1_0000);
    for page in [0x011e_0000, 0x0126_7000] {
        if !e.mem.is_mapped(page) {
            e.map(page, 0x1000);
        }
    }
    // The AI start (G29), the heap and the shape maker (O14), logged.
    e.register(AI_START, |e, a| {
        bump(e, LOG);
        e.mem.set_u32(LOG + 4, a[0]);
        e.mem.set_u32(LOG + 8, a[1]);
        Ret::default()
    });
    e.register(0x00aa_3e40, |e, _| {
        bump(e, LOG + 0x10);
        HEAP_WORD.into_ret()
    });
    e.register(MAKE_SHAPE, |e, a| {
        bump(e, LOG + 0x20);
        for (i, w) in a.iter().take(4).enumerate() {
            e.mem.set_u32(LOG + 0x24 + 4 * i as u32, *w);
        }
        MADE.into_ret()
    });
    // The settings read when the row goes on: no file (JIP's defaults: bSneakBoundingBoxFix on).
    e.register(0x00fd_f088, |_, _| Ret {
        eax: 0xffff_ffff,
        ..Ret::default()
    });
    if !e.is_registered(0x0061_a310) {
        e.register(0x0061_a310, |_, _| Ret::default());
    }
    crate::install_as(&mut e, installed);
    if installed {
        super::super::set_row(&mut e, row);
    }
    e
}

/// Not installed: no handler at the rows, the wraps' stand-ins answer, the command has none.
#[test]
fn gx_not_installed_nothing_is_put_in() {
    let mut e = world(false, false);
    for addr in AREA.fix_sites.iter().chain(AREA.jip_sites) {
        assert!(!e.site_owners(*addr).contains(&Owner::Jip), "{addr:08x}");
        assert_eq!(
            e.site(*addr, &[ACTOR, CONTROLLER, 0, 0, 0]),
            None,
            "{addr:08x}"
        );
    }
    assert_eq!(e.call(PAD_RUMBLE.addr(), &[0, 0]).eax, 0x5a5a);
    assert_eq!(e.call(MOUSE_MOVEMENT.addr(), &[INPUT, 1]).eax, 0x5a5a);
    assert_eq!(execute_of(&e, TOGGLE_MOUSE_MOVEMENT), 0);
}

/// Installed with the row off: G49 and H20 (JIP's own) are in, with the command; the fixes are
/// not (G59's wrap hands on: the stand-in answers even with rumble off).
#[test]
fn gx_installed_row_off_puts_only_jips_own_in() {
    let mut e = world(true, false);
    for addr in AREA.jip_sites {
        assert_eq!(
            e.site_owners(*addr).to_vec(),
            vec![Owner::Jip],
            "{addr:08x}"
        );
    }
    for addr in AREA.fix_sites {
        assert_eq!(
            e.site(*addr, &[ACTOR, CONTROLLER, 0, 0, 0]),
            None,
            "{addr:08x}"
        );
    }
    e.mem.set_u8(GAME_PAD_RUMBLE, 0);
    assert_eq!(e.call(PAD_RUMBLE.addr(), &[0, 0]).eax, 0x5a5a);
    e.mem.set_u32(INPUT + 0x1b24, 17);
    assert_eq!(
        e.call(MOUSE_MOVEMENT.addr(), &[INPUT, 1]).eax,
        17,
        "H20's wrap"
    );
    assert_ne!(execute_of(&e, TOGGLE_MOUSE_MOVEMENT), 0);
    let mut e = world(true, true);
    for addr in AREA.fix_sites {
        assert_eq!(
            e.site_owners(*addr).to_vec(),
            vec![Owner::Jip],
            "{addr:08x}"
        );
    }
    assert_ne!(
        e.jip_patches() & jip_patches::SNEAK_BOUNDING_BOX_FIX,
        0,
        "on by default"
    );
    e.mem.set_u8(GAME_PAD_RUMBLE, 0);
    assert_eq!(
        e.call(PAD_RUMBLE.addr(), &[0, 0]).eax,
        0,
        "G59 with the row"
    );
}

/// An actor whose base is a form of `kind` with flags byte `+0x36`, with a process whose word
/// `+0x434` is `word`, and the actor's byte `+0x1B0`.
fn actor(e: &mut Engine, kind: u8, flags: u8, byte: u8, word: u32) {
    e.mem.set_u32(ACTOR + 0x20, BASE);
    e.mem.set_u8(BASE + 4, kind);
    e.mem.set_u8(BASE + 0x36, flags);
    e.mem.set_u8(ACTOR + 0x1b0, byte);
    e.mem.set_u32(ACTOR + 0x68, PROCESS_AT);
    e.mem.set_u32(PROCESS_AT + 0x434, word);
    e.mem.set_u32(CONTROLLER + 0x414, 0x0012_3456);
    e.mem.set_u32(LOG, 0);
}

/// G29's mark: an immobile creature's controller gets bit 30 (the other bits kept); not a creature,
/// no immobile flag, or no controller: nothing; the AI start `(process, actor)` runs when the
/// actor's byte and the process word are 0, else not; an actor with no base: nothing at all.
/// Always answers 0.
#[test]
fn g29_marks_immobile_creatures() {
    let mut e = world(true, true);
    actor(&mut e, CREATURE, 0x80, 0, 0);
    assert_eq!(e.site(NO_FALL_MARK.addr(), &[ACTOR, CONTROLLER]), Some(0));
    assert_eq!(e.mem.u32(CONTROLLER + 0x414), 0x4012_3456);
    assert_eq!(
        (e.mem.u32(LOG), e.mem.u32(LOG + 4), e.mem.u32(LOG + 8)),
        (1, PROCESS_AT, ACTOR)
    );
    for (kind, flags, controller) in [(0x2a, 0x80, CONTROLLER), (CREATURE, 0x7f, CONTROLLER)] {
        actor(&mut e, kind, flags, 0, 0);
        e.site(NO_FALL_MARK.addr(), &[ACTOR, controller]);
        assert_eq!(
            e.mem.u32(CONTROLLER + 0x414),
            0x0012_3456,
            "{kind:x} {flags:x}"
        );
    }
    actor(&mut e, CREATURE, 0x80, 0, 0);
    assert_eq!(
        e.site(NO_FALL_MARK.addr(), &[ACTOR, 0]),
        Some(0),
        "no controller"
    );
    for (byte, word) in [(1u8, 0u32), (0, 5)] {
        actor(&mut e, CREATURE, 0x80, byte, word);
        e.site(NO_FALL_MARK.addr(), &[ACTOR, CONTROLLER]);
        assert_eq!(e.mem.u32(LOG), 0, "{byte} {word}");
    }
    actor(&mut e, CREATURE, 0x80, 0, 0);
    e.mem.set_u32(ACTOR + 0x20, 0);
    assert_eq!(e.site(NO_FALL_MARK.addr(), &[ACTOR, CONTROLLER]), Some(0));
    assert_eq!(e.mem.u32(LOG), 0, "no base");
    assert_eq!(e.mem.u32(CONTROLLER + 0x414), 0x0012_3456);
}

/// G29's in-air test: bit 8 or bit 30 of the flags; bits 9, 29 and 31 alone do not count.
#[test]
fn g29_in_air_takes_the_no_fall_bit() {
    let mut e = world(true, true);
    for (flags, taken) in [
        (0x100u32, 1u32),
        (0x4000_0000, 1),
        (0x4000_0100, 1),
        (0x200, 0),
        (0x2000_0000, 0),
        (0x8000_0000, 0),
        (0xbfff_feff, 0),
    ] {
        assert_eq!(
            e.site(NO_FALL_TEST.addr(), &[CONTROLLER, flags]),
            Some(taken),
            "{flags:08x}"
        );
    }
}

/// G32: no Havok object ends it; else it ends unless the collision object's bit 6 is set.
#[test]
fn g32_fixed_body_moves_when_asked() {
    let mut e = world(true, true);
    e.mem.set_u8(COLLISION + 0xc, 0x40);
    assert_eq!(
        e.site(FIXED_BODY.addr(), &[COLLISION, 0]),
        Some(1),
        "no object"
    );
    assert_eq!(e.site(FIXED_BODY.addr(), &[COLLISION, BASE]), Some(0));
    e.mem.set_u8(COLLISION + 0xc, 0xbf);
    assert_eq!(e.site(FIXED_BODY.addr(), &[COLLISION, BASE]), Some(1));
}

/// G49: the four bytes from `+0xD54` cleared, the byte after them kept.
#[test]
fn g49_clears_the_word() {
    let mut e = world(true, false);
    e.mem.set_u32(PLAYER + 0xd54, 0xffff_ffff);
    e.mem.set_u8(PLAYER + 0xd58, 0x77);
    assert_eq!(e.site(MENU_MODE_BYTE.addr(), &[PLAYER]), Some(0));
    assert_eq!(e.mem.u32(PLAYER + 0xd54), 0);
    assert_eq!(e.mem.u8(PLAYER + 0xd58), 0x77);
}

/// G59: with `bGamePadRumble` off, 0 and the import not called; on, the import's answer.
#[test]
fn g59_no_rumble_while_off() {
    let mut e = world(true, true);
    e.mem.set_u8(GAME_PAD_RUMBLE, 0);
    assert_eq!(e.call(PAD_RUMBLE.addr(), &[1, 0x1234]).eax, 0);
    e.mem.set_u8(GAME_PAD_RUMBLE, 1);
    assert_eq!(e.call(PAD_RUMBLE.addr(), &[1, 0x1234]).eax, 0x5a5a);
    e.mem.set_u8(GAME_PAD_RUMBLE, 0x80);
    assert_eq!(e.call(PAD_RUMBLE.addr(), &[1, 0x1234]).eax, 0x5a5a);
}

/// G65: the state word 2, the wanted state 0xB, and the fall start the jump height times
/// `0x40DFF8D6` plus the height, in single floats.
#[test]
fn g65_jump_height_in_havok_units() {
    let mut e = world(true, true);
    e.mem.set_f32(CONTROLLER + 0x540, 1.37);
    e.mem.set_u32(CONTROLLER + 0x3f0, 9);
    e.mem.set_u32(CONTROLLER + 0x520, 9);
    let height = 52.25f32;
    assert_eq!(
        e.site(JUMP_HEIGHT.addr(), &[CONTROLLER, height.to_bits()]),
        Some(0)
    );
    let unit = f32::from_bits(0x40df_f8d6);
    assert_eq!(e.mem.f32(CONTROLLER + 0x544), 1.37f32 * unit + height);
    assert_eq!(e.mem.u32(CONTROLLER + 0x3f0), 2);
    assert_eq!(e.mem.u32(CONTROLLER + 0x520), 0xb);
}

fn axis(e: &mut Engine, kind: u32) -> u32 {
    e.call(MOUSE_MOVEMENT.addr(), &[INPUT, kind]).eax
}

/// H20: the axes by `s_mouseMovementState` (7 at first), the wheel unless its direction's key is
/// disabled, 0 for types above 3 and for type 0 (bit 31 of the state, unless a script set it: then
/// the word before the x axis, as JIP's `BT` reads it); `ToggleMouseMovement` answers bits 0 and 1
/// and sets the argument with bit 2.
#[test]
fn h20_mouse_movement_as_jip() {
    let mut e = world(true, false);
    e.mem.set_u32(INPUT + 0x1b20, 0x99);
    e.mem.set_u32(INPUT + 0x1b24, 11);
    e.mem.set_u32(INPUT + 0x1b28, -7i32 as u32);
    assert_eq!((axis(&mut e, 1), axis(&mut e, 2)), (11, -7i32 as u32));
    assert_eq!(
        (axis(&mut e, 0), axis(&mut e, 4), axis(&mut e, 0x8000_0003)),
        (0, 0, 0)
    );
    assert_eq!(call(&mut e, TOGGLE_MOUSE_MOVEMENT, &[]), (1, 3.0));
    assert_eq!(call(&mut e, TOGGLE_MOUSE_MOVEMENT, &[int_arg(1)]), (1, 3.0));
    assert_eq!((axis(&mut e, 1), axis(&mut e, 2)), (11, 0));
    assert_eq!(call(&mut e, TOGGLE_MOUSE_MOVEMENT, &[]), (1, 1.0));
    call(&mut e, TOGGLE_MOUSE_MOVEMENT, &[int_arg(2)]);
    assert_eq!((axis(&mut e, 1), axis(&mut e, 2)), (0, -7i32 as u32));
    call(&mut e, TOGGLE_MOUSE_MOVEMENT, &[int_arg(-1)]);
    assert_eq!(axis(&mut e, 0), 0x99, "BT's bit 31");
    // The wheel.
    e.mem.set_u32(INPUT + 0x1b2c, 120);
    assert_eq!(axis(&mut e, 3), 120);
    crate::input::set_key_disable_state(&mut e, WHEEL_UP, true);
    assert_eq!(axis(&mut e, 3), 0, "wheel up disabled");
    e.mem.set_u32(INPUT + 0x1b2c, -120i32 as u32);
    assert_eq!(axis(&mut e, 3), -120i32 as u32, "down still on");
    crate::input::set_key_disable_state(&mut e, WHEEL_DOWN, true);
    assert_eq!(axis(&mut e, 3), 0, "wheel down disabled");
    crate::input::set_key_disable_state(&mut e, WHEEL_UP, false);
    assert_eq!(axis(&mut e, 3), 0);
    e.mem.set_u32(INPUT + 0x1b2c, 0);
    assert_eq!(axis(&mut e, 3), 0);
}

/// The shared shape with five vertices (lanes of mixed signs, a NaN and -0.0), the cinfo's owner's
/// reference of form id `id` and teammate byte `teammate`, and JIP's shape to be made.
fn o14_world(id: u32, teammate: u8) -> Engine {
    let e = world(true, true);
    e.mem.set_u32(0x0126_7be0, SHARED);
    e.mem.set_u32(SHARED + 8, SHARED_HK);
    e.mem.set_u32(SHARED_HK + 0x40, SHARED_VERTS);
    e.mem.set_u32(MADE + 8, MADE_HK);
    e.mem.set_u32(MADE_HK + 0x40, MADE_VERTS);
    e.mem.set_u32(MADE + 4, 0);
    for vertex in 0..5u32 {
        for lane in 0..4u32 {
            let value = match (vertex, lane) {
                (0, 0) => f32::NAN,
                (1, 1) => -0.0,
                (2, 2) => 6.5,
                _ => (vertex as f32 - 2.0) * 3.25 + lane as f32,
            };
            e.mem
                .set_f32(SHARED_VERTS + 0x20 + vertex * 0x30 + lane * 4, value);
        }
    }
    e.mem.set_u32(INFO + 0x80, OWNER);
    e.mem.set_u32(OWNER + 0xcc, REFERENCE);
    e.mem.set_u32(REFERENCE + 0xc, id);
    e.mem.set_u8(REFERENCE + 0x18d, teammate);
    e
}

fn shape_words(e: &Engine, verts: u32) -> Vec<u32> {
    (0..5u32)
        .flat_map(|v| (0..4u32).map(move |l| verts + 0x20 + v * 0x30 + l * 4))
        .map(|at| e.mem.u32(at))
        .collect()
}

fn ask_shape(e: &mut Engine) -> Option<u32> {
    e.site(
        CONTROLLER_SHAPE.addr(),
        &[
            INFO,
            B + 0xa2c,
            B + 0xa40,
            9.5f32.to_bits(),
            0.75f32.to_bits(),
        ],
    )
}

fn sneak(e: &mut Engine, word: u32) -> Option<u32> {
    e.mem.set_u32(PLAYER + 0x190, PLAYER_PROCESS);
    e.mem.set_u32(PLAYER_PROCESS + 0x94, word);
    e.site(SNEAK_TEST.addr(), &[PLAYER])
}

/// O14's two seams together: for the player, JIP's shape is made once (the shape maker with the
/// frame's `A`, `B`, radius and step height; its count up), kept in JIP's word, and copied from
/// after; a sneak test then lowers its vertices (each lane not below 0 less 6, a NaN and a negative
/// kept, -0.0 lowered) and standing puts the kept set back, each once per change (the state byte
/// `+0xD57`).
#[test]
fn o14_the_shape_and_the_sneak_test_together() {
    let mut e = o14_world(0x14, 0);
    // Before any shape: the test answers, the byte stays.
    e.mem.set_u8(PLAYER + 0xd57, 0);
    assert_eq!(sneak(&mut e, 0x400), Some(1));
    assert_eq!(e.mem.u8(PLAYER + 0xd57), 0, "no shape yet");
    let kept = shape_words(&e, SHARED_VERTS);
    assert_eq!(ask_shape(&mut e), Some(HEAP_WORD));
    assert_eq!(e.mem.u32(HEAP_WORD), MADE);
    assert_eq!(e.mem.u32(MADE + 4), 1, "its count up");
    assert_eq!(
        (0..5)
            .map(|i| e.mem.u32(LOG + 0x20 + 4 * i))
            .collect::<Vec<_>>(),
        vec![1, B + 0xa2c, B + 0xa40, 9.5f32.to_bits(), 0.75f32.to_bits()]
    );
    assert_eq!(ask_shape(&mut e), Some(HEAP_WORD), "made once");
    assert_eq!((e.mem.u32(LOG + 0x20), e.mem.u32(LOG + 0x10)), (1, 1));
    let lowered: Vec<u32> = kept
        .iter()
        .map(|w| {
            let v = f32::from_bits(*w);
            if v >= 0.0 {
                (v - 6.0).to_bits()
            } else {
                *w
            }
        })
        .collect();
    assert_eq!(lowered[4 + 1], (-6.0f32).to_bits(), "-0.0 lowered");
    assert!(f32::from_bits(lowered[0]).is_nan(), "NaN kept");
    for (word, sneaking, set) in [
        (0x400u32, 1u32, &lowered),
        (0x400 | 0x3, 1, &lowered),
        (0x0, 0, &kept),
        (0xc00, 0, &kept),
        (0x400, 1, &lowered),
    ] {
        e.mem.set_u32(MADE_VERTS + 0x20, 0xdead);
        assert_eq!(sneak(&mut e, word), Some(sneaking), "{word:x}");
        assert_eq!(e.mem.u8(PLAYER + 0xd57), sneaking as u8);
        if e.mem.u32(MADE_VERTS + 0x20) != 0xdead {
            assert_eq!(&shape_words(&e, MADE_VERTS), set, "{word:x}");
        }
    }
    // The same state twice writes nothing.
    e.mem.set_u32(MADE_VERTS + 0x20, 0xdead);
    assert_eq!(sneak(&mut e, 0x400), Some(1));
    assert_eq!(e.mem.u32(MADE_VERTS + 0x20), 0xdead);
    // A change writes the whole set.
    assert_eq!(sneak(&mut e, 0), Some(0));
    assert_eq!(shape_words(&e, MADE_VERTS), kept);
    // No process word: 0, nothing written.
    e.mem.set_u32(PLAYER + 0x190, 0);
    assert_eq!(e.site(SNEAK_TEST.addr(), &[PLAYER]), Some(0));
}

/// O14's shape seam: anyone else gets the shared `NiPointer`, a teammate JIP's, no owner the
/// shared one.
#[test]
fn o14_shape_for_the_player_and_teammates_only() {
    let mut e = o14_world(0x15, 0);
    assert_eq!(ask_shape(&mut e), Some(0x0126_7be0));
    assert_eq!(e.mem.u32(LOG + 0x20), 0, "nothing made");
    let mut e = o14_world(0x15, 1);
    assert_eq!(ask_shape(&mut e), Some(HEAP_WORD));
    let mut e = o14_world(0x14, 0);
    e.mem.set_u32(INFO + 0x80, 0);
    assert_eq!(ask_shape(&mut e), Some(0x0126_7be0));
}
