//! The differential test of the HUD's per-frame update (`HUDMainMenu::DoIdle`, `00770430`,
//! `engine_typed::hud::idle`) against its translation (`engine-fallout-interface`'s
//! `hudmainmenu.rs`). The world: the player (heading, position, its process), the HUD main menu
//! with a real tile at every offset the update reads (their traits made and set before the run),
//! the clocks, the damage-threshold timers, the enemy target and the VATS list, the interface mode
//! and its actor, the quest reminder's state, the subtitles and the corner messages, the settings
//! and the `.rdata` constants.
//!
//! What stands in: every sub-update the update calls by address (each logs its words), the value's
//! own setters (they log what the tile setters hand them), the trait name lookups, the tiles' fades
//! and reaction lists, the clock, the screen size answers. JIP LN's three places (H21 at the HUD
//! tiles' update, O14 at the sneak test, G10 at the message time) are asked with JIP off and on.
//!
//! The typed update does its few float sums in `f32` at the main thread's precision: the floats
//! compare within a few ulps.

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::{Case, Diff, Input};
use engine_core::typed::leaf;
use engine_typed::{crt, dispatch, hud, pathing};

const LOG: u32 = 0x0300_0000;
const VT_FN: u32 = 0x00ff_2000;
const ULPS: u32 = 4;
/// The HUD's clock (`this + 0x224`) and the game timer's tick counts.
const HUD_NOW: u32 = 20_000;
const GAME_NOW: u32 = 5_000;

fn ret(eax: u32) -> Ret {
    Ret {
        eax,
        ..Ret::default()
    }
}

fn ret_f(st0: f64) -> Ret {
    Ret {
        st0,
        ..Ret::default()
    }
}

/// Appends `words` to the log under `tag`.
fn log(e: &mut Engine, tag: u32, words: &[u32]) {
    let n = e.mem.u32(LOG);
    e.mem.set_u32(LOG + 4 + 4 * n, tag);
    for (i, w) in (1u32..).zip(words) {
        e.mem.set_u32(LOG + 4 + 4 * (n + i), *w);
    }
    e.mem.set_u32(LOG, n + 1 + words.len() as u32);
}

/// A stand-in that logs its words under its address and answers `answer`.
fn logging(e: &mut Engine, at: u32, answer: Ret) {
    e.register_double(at, move |e, a| {
        log(e, at, a);
        answer
    });
}

/// A tile with no traits (its trait array's allocate, free and reallocate are stand-ins).
fn tile(e: &mut Engine) -> u32 {
    let vt = e.mem.alloc(0x10);
    for (slot, at) in [(4u32, 0x00fd_0104u32), (8, 0x00fd_0108), (0xc, 0x00fd_010c)] {
        e.mem.set_u32(vt + slot, at);
    }
    let t = e.mem.alloc(0x40);
    e.mem.set_u32(t + 0x10, vt);
    let buffer = e.mem.alloc(16 * 4);
    e.mem.set_u32(t + 0x14, buffer);
    e.mem.set_u32(t + 0x1c, 16);
    t
}

/// The tile's trait `id` made and its float set to `value` (the value setters are stand-ins).
fn set_trait(e: &mut Engine, t: u32, id: u32, value: f32) {
    let v = e.call(0x00a0_1000, &args![t, id]).u32();
    e.mem.set_f32(v + 8, value);
}

/// The pages and constants, and the stand-ins below the tiles.
fn world(e: &mut Engine) {
    for (base, size) in [
        (LOG, 0x4000),
        (0x011c_f000, 0x1000),
        (0x011d_0000, 0x6000),
        (0x011d_9000, 0x6000),
        (0x011f_2000, 0x2000),
        (0x011f_6000, 0x1000),
        (0x0101_1000, 0x1000),
        (0x0101_2000, 0x1000),
        (0x0101_6000, 0x1000),
        (0x0101_7000, 0x1000),
        (0x0101_d000, 0x1000),
        (0x0101_e000, 0x1000),
        (0x0102_1000, 0x1000),
        (0x0102_f000, 0x1000),
        (0x0103_5000, 0x1000),
        (0x0107_3000, 0x1000),
        (0x0127_0000, 0x1000),
    ] {
        e.map(base, size);
    }
    for (at, value) in [
        (0x0101_2060, 0.0),
        (0x0101_1590, 2.0),
        (0x0101_e568, 255.0),
        (0x0102_f248, 57.295_779_513_082_32),
        (0x0101_7b70, 1000.0),
        (0x0101_db88, 7.5),
        (0x0107_3480, 3000.0),
        (0x0103_57e8, 20.0),
    ] {
        e.mem.set_f64(at, value);
    }
    for (at, value) in [
        (0x0101_62c0u32, 0.5f32),
        (0x0101_6248, 0.25),
        (0x0102_1f44, 0.75),
    ] {
        e.mem.set_f32(at, value);
    }
    // SSE2 present: the typed `_ftol2_sse` truncates.
    e.mem.set_u32(0x0127_0a6c, 1);
    // ... and `GetLastError` (`00fdf094`), which the C runtime's `fabs` reaches on a NaN.
    for stub in [
        0x00fd_f05c,
        0x00fd_f100,
        0x00ec_658f,
        0x00aa_4060,
        0x00fd_f094,
    ] {
        e.register_double(stub, |_, _| Ret::default());
    }
    e.register_double(0x00aa_3e40, |e, a| ret(e.mem.alloc(a[1].max(4))));
    e.register_double(0x00fd_0104, |e, a| ret(e.mem.alloc(a[1].max(1) * 4)));
    e.register_double(0x00fd_0108, |_, _| Ret::default());
    e.register_double(0x00fd_010c, |e, a| {
        let moved = e.mem.alloc(a[2].max(1) * 4);
        let bytes = e.mem.bytes(a[1], e.mem.u32(a[0] + 8) * 4);
        e.mem.write(moved, &bytes);
        ret(moved)
    });
    logging(e, 0x00a0_a270, Ret::default());
    logging(e, 0x00a0_a300, Ret::default());
    // The float settings' value address (`00403e20`): the setting's +4. The HUD opacity 0.75, a
    // new target's timeout 1.5 s; the reminder delay 3 s (an integer).
    e.register_double(0x0040_3e20, |_, a| ret(a[0] + 4));
    e.mem.set_f32(0x011d_9774 + 4, 0.75);
    e.mem.set_f32(0x011c_f87c + 4, 1.5);
    e.mem.set_i32(0x011d_2220 + 4, 3);
    // A trait's id by its name (`00a01860`): the name's low word plus 10,000.
    e.register_double(0x00a0_1860, |e, a| {
        log(e, 0x00a0_1860, a);
        ret(10_000 + (a[0] & 0xfff))
    });
    e.mem.set_f32(0x011d_979c, 200.0);
}

/// The families and the comparison settings every test of this file uses.
trait Families {
    fn families(self) -> Diff;
}

impl Families for Diff {
    fn families(self) -> Diff {
        self.family(&dispatch::DISPATCH)
            .family(&pathing::PATHING)
            .family(&crt::CRT)
            .family(&leaf::LEAVES)
            // The typed update reads the reminder delay's integer itself: the translation's getter
            // (`0043d4d0`) is left out.
            .ignore_calls(&[0x0043_d4d0])
            .translation_calls_plainly(&[0x00a0_0f30, 0x00a0_1000, 0x00a0_cd50, 0x0040_8860])
            // The main thread's word (the `hud` family's threads).
            .control_words(&[0x007f])
            .float_ulps(ULPS)
    }
}

/// The HUD's tile offsets the update reads.
const TILES: [u32; 25] = [
    0x64, 0x68, 0x7c, 0x80, 0x84, 0x88, 0x8c, 0x90, 0x94, 0x98, 0x9c, 0xa8, 0x114, 0x118, 0x120,
    0x128, 0x134, 0x154, 0x158, 0x15c, 0x160, 0x170, 0x174, 0x180, 0x2c,
];

/// The world of a case. `w`: [0] the player (0 with its process, 1 none, 2 no process); [1] the
/// 200 ms gate (0 passed, 1 not yet, 2 the first call); [2] the damage timers (0 unset, 1 recent,
/// 2 old, 3 one of each); [3] JIP (0 off, 1 fixes in with H21 and G10 answering, 2 the sneak fix
/// answering 1, 3 answering 0x100); [4] sneaking (bit 0), the sneak meter shown (bit 1), the fades
/// finished (bit 2); [5] the region text and subtitle: shown (bits 0, 1), past their timeout (bit
/// 2), the hold trait (bit 3), the compass shown (bit 4); [6] the enemy: there (bit 0), in VATS
/// (bit 1), dead (bit 2), interface mode 4 (bit 3), its tile's state 2 (bit 4), perceived (bit 5),
/// detecting (bit 6), sneaking (bit 7); [7] quests: forced reminders (bit 0), the timer clear (bit
/// 1), the game state 2 (bit 2), kill (bit 3), the subtitles: none (bit 4), two (bit 5), no current
/// (bit 6); [8] messages: none (0), one with its tiles clear (1), one shown and timed out (2), one
/// shown, not timed out (3). `f`: the player's heading, the screen width.
fn idle_world(e: &mut Engine, case: &Case) -> Vec<u32> {
    world(e);
    let w = |i: usize| case.word(i);
    let heading = case.f32(9);
    let screen = case.f32(10);
    // The HUD menu (also `this`), its tiles.
    let menu = e.mem.alloc(0x300);
    e.mem.set_u32(0x011d_96c0, menu);
    for offset in TILES {
        let t = tile(e);
        e.mem.set_u32(menu + offset, t);
    }
    let at = |e: &Engine, offset: u32| e.mem.u32(menu + offset);
    // The player and its process.
    let vt = e.mem.alloc(0x500);
    for slot in (0..0x500).step_by(4) {
        e.mem.set_u32(vt + slot, VT_FN + slot);
    }
    let me = e.mem.alloc(0x100);
    e.mem.set_u32(me, vt);
    let position = e.mem.alloc(12);
    e.register_double(VT_FN + 0x2bc, move |e, a| {
        log(e, VT_FN + 0x2bc, a);
        ret_f(f64::from(heading))
    });
    e.register_double(VT_FN + 0x1f4, move |_, _| ret(position));
    let sneaking_answer = u32::from(w(6) & 0x80 != 0);
    e.register_double(VT_FN + 0x22c, move |e, a| {
        log(e, VT_FN + 0x22c, a);
        ret(sneaking_answer)
    });
    e.register_double(VT_FN + 0x100, |e, a| {
        log(e, VT_FN + 0x100, a);
        ret(1)
    });
    if w(0) != 1 {
        e.mem.set_u32(0x011d_ea3c, me);
    }
    let process = u32::from(w(0) != 2) * 0x0300_3000;
    logging(e, 0x008d_6f30, ret(process));
    // The clocks: the HUD's own (+0x224) and the game timer.
    e.register_double(0x0082_5c00, move |e, a| {
        log(e, 0x0082_5c00, a);
        ret(if a[0] == menu + 0x224 {
            HUD_NOW
        } else {
            GAME_NOW
        })
    });
    match w(1) {
        0 => {
            e.mem.set_u32(0x011d_97c8, 1);
            e.mem.set_u32(0x011d_97c4, HUD_NOW - 500);
        }
        1 => {
            e.mem.set_u32(0x011d_97c8, 1);
            e.mem.set_u32(0x011d_97c4, HUD_NOW - 100);
        }
        _ => {}
    }
    let (first, second) = [
        (u32::MAX, u32::MAX),
        (GAME_NOW - 500, GAME_NOW - 900),
        (GAME_NOW - 1500, 100),
        (GAME_NOW - 200, GAME_NOW - 4000),
    ][w(2) as usize];
    e.mem.set_u32(menu + 0x24c, first);
    e.mem.set_u32(menu + 0x250, second);
    // The screen: width, right edge, compass width, inset (each an `ST0` answer).
    for (at_fn, value) in [
        (0x0071_5da0u32, f64::from(screen)),
        (0x0071_5d40, f64::from(screen) - 10.5),
        (0x0071_77c0, 64.25),
        (0x0071_7820, 12.0),
    ] {
        e.register_double(at_fn, move |e, a| {
            log(e, at_fn, a);
            ret_f(value)
        });
    }
    // The tiles' traits.
    for (offset, id, value) in [
        (0x118u32, 0xfb0u32, 40.0f32),
        (0x114, 0xfb0, 33.5),
        (0x114, 0xfb1, 101.25),
        (0x9c, 0xfa3, 0.0),
        (0xa8, 0xfa3, 0.0),
        (0x134, 0xfa3, 1.0),
    ] {
        let t = at(e, offset);
        set_trait(e, t, id, value);
    }
    let fades = w(5);
    for (bit, offset) in [(1u32, 0x64u32), (2, 0x68)] {
        let t = at(e, offset);
        set_trait(e, t, 0xfa9, if fades & bit != 0 { 180.0 } else { 0.0 });
        let start = if fades & 4 != 0 { 1000.0 } else { 4500.0 };
        set_trait(e, t, 0x1004, start);
    }
    let hold = 10_000 + (0x0107_3474 & 0xfff);
    if fades & 8 != 0 {
        let t = at(e, 0x64);
        set_trait(e, t, hold, 1.0);
    }
    let t = at(e, 0x128);
    set_trait(e, t, 0xfa3, if fades & 16 != 0 { 1.0 } else { 0.0 });
    let sneak = w(4);
    let t = at(e, 0x88);
    set_trait(e, t, 0xfa9, if sneak & 2 != 0 { 150.0 } else { 0.0 });
    let finished = u32::from(sneak & 4 != 0);
    logging(e, 0x00a0_7fc0, ret(finished));
    logging(e, 0x0049_97b0, ret(sneak & 1));
    // The enemy.
    let enemy_bits = w(6);
    let enemy = if enemy_bits & 1 != 0 {
        let evt = e.mem.alloc(0x300);
        for slot in (0..0x300).step_by(4) {
            e.mem.set_u32(evt + slot, VT_FN + slot);
        }
        let enemy = e.mem.alloc(0x40);
        e.mem.set_u32(enemy, evt);
        enemy
    } else {
        0
    };
    e.mem.set_u32(menu + 0x21c, enemy);
    e.mem.set_u32(menu + 0x244, HUD_NOW - 3000);
    let record = e.mem.alloc(8);
    e.mem.set_u32(record, enemy);
    e.mem.set_u8(record + 4, u8::from(enemy_bits & 2 != 0));
    let node = e.mem.alloc(8);
    e.mem.set_u32(node, record);
    let vats = e.mem.alloc(8);
    e.mem.set_u32(vats, node);
    e.register_double(0x0059_5d10, move |_, _| ret(vats));
    logging(e, 0x0047_7ba0, ret(u32::from(enemy_bits & 4 != 0)));
    e.mem
        .set_u32(0x011f_2250 + 8, if enemy_bits & 8 != 0 { 4 } else { 0 });
    e.mem.set_u32(0x011f_21cc, enemy);
    let t = at(e, 0x120);
    set_trait(e, t, 0xff4, if enemy_bits & 16 != 0 { 2.0 } else { 1.0 });
    let perceived = e.mem.alloc(8);
    e.mem.set_u8(perceived + 4, 1);
    let perceived = if enemy_bits & 0x20 != 0 { perceived } else { 0 };
    logging(e, 0x0096_7090, ret(perceived));
    logging(e, 0x008b_c700, ret(u32::from(enemy_bits & 0x40 != 0)));
    logging(e, 0x0049_3bb0, ret(enemy_bits & 1));
    logging(e, 0x0077_2840, ret_f(0.375));
    let last = e.mem.alloc(0x10);
    e.mem.set_u32(menu + 0x1b8, last);
    e.mem.set_u8(0x011d_97c0, 1);
    logging(e, 0x0057_9690, ret(u32::from(enemy_bits & 2 != 0)));
    // Quests.
    let quests = w(7);
    e.mem.set_u8(menu + 0x205, u8::from(quests & 1 != 0));
    e.mem
        .set_u32(menu + 0x248, if quests & 2 != 0 { 0 } else { 100 });
    logging(e, 0x0045_8030, ret(if quests & 4 != 0 { 2 } else { 1 }));
    e.mem.set_u8(menu + 0x254, u8::from(quests & 8 != 0));
    logging(e, 0x0076_b610, ret(u32::from(quests & 0x10 != 0)));
    let count = if quests & 0x20 != 0 { 2 } else { 1 };
    let buffer = e.mem.alloc(8);
    let mut subtitles = vec![];
    for k in 0..count {
        let s = e.mem.alloc(0x110);
        e.mem.set_u32(s + 0x104, HUD_NOW - 2000);
        e.mem.set_f32(s + 0x108, [1.5f32, 4.0][k as usize]);
        e.mem.set_u32(buffer + 4 * k, s);
        subtitles.push(s);
    }
    e.mem.set_u32(menu + 0x198 + 4, buffer);
    e.mem.set_u32(menu + 0x198 + 8, count);
    e.mem.set_u32(
        menu + 0x1a8,
        if quests & 0x40 != 0 { 0 } else { subtitles[0] },
    );
    // The array's remove (`006bf8f0`) takes the element out, so the loop ends.
    e.register_double(0x006b_f8f0, |e, a| {
        log(e, 0x006b_f8f0, a);
        let n = e.mem.u32(a[0] + 8);
        let base = e.mem.u32(a[0] + 4);
        for i in a[1]..n.saturating_sub(1) {
            let next = e.mem.u32(base + 4 * (i + 1));
            e.mem.set_u32(base + 4 * i, next);
        }
        e.mem.set_u32(a[0] + 8, n.saturating_sub(1));
        Ret::default()
    });
    let next_text = e.mem.alloc(0x10);
    logging(e, 0x0077_5230, ret(next_text));
    logging(e, 0x005c_bb50, ret(0x0300_3100));
    // Messages.
    let messages = w(8);
    logging(e, 0x005a_e380, ret(u32::from(messages != 0)));
    let t = at(e, 0x94);
    set_trait(e, t, 0xfa9, if messages >= 2 { 100.0 } else { 0.0 });
    let message = e.mem.alloc(0x420);
    let text = if w(3) == 1 { 0x204 } else { 0x104 };
    e.mem.set_f32(
        message + text + 0x208,
        if messages == 3 { 60.0 } else { 2.0 },
    );
    e.mem.set_u32(menu + 0x18c, message);
    e.mem
        .set_u32(menu + 0x194, if messages == 2 { 0 } else { HUD_NOW - 5000 });
    // Every other sub-update by address.
    for at_fn in [
        0x0077_1520u32,
        0x0077_9070,
        0x0077_da60,
        0x0077_21c0,
        0x0077_e750,
        0x0077_f700,
        0x00a0_7530,
        0x00a0_7c60,
        0x0071_5c60,
        0x0077_1540,
        0x0077_e4f0,
        0x0077_32d0,
        0x0077_a650,
        0x0077_c4e0,
        0x0077_4090,
        0x004d_1360,
        0x0077_39f0,
        0x0077_3190,
        0x0077_5a00,
        0x0077_01e0,
        0x0077_36d0,
        0x0077_f500,
        0x0077_f5e0,
        0x0077_5670,
        0x0040_1030,
        0x0077_56e0,
    ] {
        logging(e, at_fn, Ret::default());
    }
    // JIP.
    match w(3) {
        1 => {
            e.set_jip_patches(engine_core::extend::jip_patches::FIXES);
            e.on_site_for_test(0x0077_04ae, |e, a| {
                log(e, 0x0077_04ae, a);
                None
            });
            e.on_site_for_test(0x0077_14da, |e, a| {
                log(e, 0x0077_14da, a);
                Some(0x204)
            });
        }
        2 => {
            e.set_jip_patches(engine_core::extend::jip_patches::SNEAK_BOUNDING_BOX_FIX);
            e.on_site_for_test(0x0077_0b0e, |e, a| {
                log(e, 0x0077_0b0e, a);
                Some(1)
            });
        }
        3 => {
            // Only the low byte is the answer: 0x100 is "not sneaking".
            e.set_jip_patches(engine_core::extend::jip_patches::SNEAK_BOUNDING_BOX_FIX);
            e.on_site_for_test(0x0077_0b0e, |e, a| {
                log(e, 0x0077_0b0e, a);
                Some(0x100)
            });
        }
        _ => {}
    }
    e.mem.set_u32(LOG, 0);
    vec![menu]
}

#[test]
fn do_idle_diff() {
    Diff::new(engine_core::entry!(0x0077_0430, hud::idle::do_idle(Ptr<_>)))
        .families()
        .inputs(&[
            Input::Of(&[0, 1, 2]),
            Input::Of(&[0, 1, 2]),
            Input::Of(&[0, 1, 2, 3]),
            Input::Of(&[0, 1, 2, 3]),
            Input::Of(&[0, 1, 2, 3, 4, 6, 7]),
            Input::Of(&[0, 1, 2, 3, 7, 0xb, 0x18, 0x1a, 0x1f]),
            Input::Of(&[0, 1, 3, 0x13, 0x17, 0x19, 0x33, 0x53, 0x93, 0x0b, 0xf3]),
            Input::Of(&[0, 1, 2, 4, 6, 8, 0x10, 0x20, 0x60, 0x21]),
            Input::Of(&[0, 1, 2, 3]),
            Input::F32Swept(0x3f00_0000),
            Input::F32Swept(0x44a0_0000),
        ])
        .max_cases(96)
        .float_sweep(4)
        .setup(idle_world)
        .run()
        .assert_same();
}
