//! The differential test of every typed function of the HUD family (`engine_typed::hud`) against
//! its translation (`engine-fallout-interface`'s `hudmainmenu.rs`). The world: the player (heading,
//! position, sneaking, the experience reward, its own map marker), its process, the HUD main menu
//! (width, half angle, the quest targets), the compass tile and its strip, the three tile pools with
//! real tiles (their traits made and set by the typed tile setters of `dispatch`), the map markers
//! with their references and map data, the quest targets, the VATS actors with their processes and
//! actor values, the blink and range settings, the game timer and the `.rdata` constants.
//!
//! What stands in: the value's own `SetFloat` and `SetString` (they log what reaches them), the
//! trait name table, the player's and the references' virtual slots, the marker data's getters and
//! setters, the discovery's message, event and stat, the marker helper, the quest target list and
//! its records, the pools' iterators, the VATS list, its count and sort, the heading of a vector,
//! the clamp, the menu's render test. JIP LN's two places in the compass are asked with the
//! extenders off and on (its name answering, and the location-discovered event with a user).
//!
//! The typed code does its float maths in `f32` at the main thread's precision where the
//! translation works in doubles: the floats compare within a few ulps.

use engine as _;
use engine_core::extend::{Hook, Owner, SiteSlot};
use engine_core::prelude::*;
use engine_core::typed::diff::{Case, Diff, Input};
use engine_core::typed::leaf;
use engine_typed::{crt, dispatch, hud, multibound, pathing};

const LOG: u32 = 0x0300_0000;
const VT_FN: u32 = 0x00ff_2000;
const ULPS: u32 = 4;

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

/// A vtable whose slots are registered stand-ins at `VT_FN + base + slot`; returns its address.
fn vtable(e: &mut Engine, base: u32) -> u32 {
    let vt = e.mem.alloc(0x500);
    for slot in (0..0x500).step_by(4) {
        e.mem.set_u32(vt + slot, VT_FN + base + slot);
    }
    vt
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

/// A list of nodes (element, next) holding `items`; its first node (0 for none).
fn list(e: &mut Engine, items: &[u32]) -> u32 {
    let mut next = 0;
    for item in items.iter().rev() {
        let node = e.mem.alloc(8);
        e.mem.set_u32(node, *item);
        e.mem.set_u32(node + 4, next);
        next = node;
    }
    next
}

/// The pages and constants every test reads, and the stand-ins below the compass.
fn world(e: &mut Engine) {
    for (base, size) in [
        (LOG, 0x4000),
        (0x011c_d000, 0x1000),
        (0x011d_0000, 0x6000),
        (0x011d_9000, 0x6000),
        (0x011e_0000, 0x1000),
        (0x011f_2000, 0x2000),
        (0x011f_6000, 0x1000),
        (0x0101_1000, 0x1000),
        (0x0101_2000, 0x1000),
        (0x0101_e000, 0x1000),
        (0x0101_f000, 0x1000),
        (0x0102_1000, 0x1000),
        (0x0102_f000, 0x1000),
        (0x0103_c000, 0x1000),
        (0x0107_0000, 0x1000),
        (0x0107_3000, 0x1000),
        (0x0126_f000, 0x1000),
        (0x0127_0000, 0x1000),
    ] {
        e.map(base, size);
    }
    for (at, value) in [
        (0x0101_2060, 0.0),
        (0x0101_2070, 1.0),
        (0x0101_1590, 2.0),
        (0x0102_1928, 3.0),
        (0x0101_e568, 255.0),
        (0x0102_f248, 57.295_779_513_082_32),
        (0x0103_c348, 360.0),
        (0x0107_0c80, 0.15),
        (0x0102_17d8, 0.8),
        (0x0102_f078, 10_000.0),
        (0x0107_3568, 70.0),
        (0x0101_ff58, -std::f64::consts::PI),
        (0x0101_ff48, 2.0 * std::f64::consts::PI),
        (0x0101_ff40, std::f64::consts::PI),
        (0x0107_34f8, 1.45),
    ] {
        e.mem.set_f64(at, value);
    }
    e.mem.set_f32(0x0107_3608, 180.0);
    // SSE2 present: the typed `_ftol2_sse` truncates.
    e.mem.set_u32(0x0127_0a6c, 1);
    // The tiles: critical section, atexit, memory manager, trait arrays.
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
    // The value's own setters log what the tile setters hand them.
    logging(e, 0x00a0_a270, Ret::default());
    logging(e, 0x00a0_a300, Ret::default());
    // `Tile::AddUserTrait`: the name's low word plus 10,000.
    e.register_double(0x00a0_0940, |e, a| {
        log(e, 0x00a0_0940, a);
        ret(10_000 + (a[0] & 0xfff))
    });
    // The settings: blink periods 30, 10 and 20 (squared 900, 100, 400), their timings and
    // delays; the ranges; the tick cap; the experience; the float settings.
    for (at, value) in [
        (0x011d_1d54u32, 30i32),
        (0x011d_30d8, 10),
        (0x011d_4d44, 20),
        (0x011d_2bd4, 2),
        (0x011d_3c94, 5),
        (0x011d_30f0, 9),
        (0x011d_24f0, 700),
        (0x011d_46cc, 300),
        (0x011d_3924, 100),
        (0x011d_12cc, 40),
        (0x011d_0bd8, 30),
        (0x011d_32d0, 2),
        (0x011d_0364, 25),
        (0x011c_d7d8, 35),
    ] {
        e.mem.set_i32(at + 4, value);
    }
    e.mem.set_f32(0x011d_9774 + 4, 0.75);
    e.mem.set_f32(0x011c_d7d8 + 4, 35.0);
    e.mem.set_f32(0x011c_dcbc + 4, 1.5);
    e.register_double(0x0040_3e20, |_, a| ret(a[0] + 4));
    // The timer: frame time 0.016, ticks 5,000.
    e.mem.set_f32(0x011f_6394 + 0xc, 0.016);
    e.mem.set_u32(0x011f_6394 + 0x14, 5_000);
    e.mem.set_f32(0x011d_979c, 200.0);
}

/// The families and the comparison settings every test of this file uses.
trait Families {
    fn families(self) -> Diff;
}

impl Families for Diff {
    fn families(self) -> Diff {
        self.family(&hud::HUD)
            .family(&dispatch::DISPATCH)
            .family(&pathing::PATHING)
            .family(&crt::CRT)
            .family(&multibound::MULTIBOUND)
            .family(&leaf::LEAVES)
            // The typed code reads the settings, interpolates and takes the differences and the squared
            // length in its own body: the translation's getter (`0043d4d0`), interpolation (`004b3ab0`),
            // point difference (`00439ef0`) and squared length (`004a7290`) calls are left out.
            .ignore_calls(&[0x0043_d4d0, 0x004b_3ab0, 0x0043_9ef0, 0x004a_7290])
            // The translated tile getters and setters (reached by address from the translated compass)
            // find and make the trait value and insert it by plain Rust calls; the typed ones by
            // `typed_call!` (as in `menu_tiles`).
            .translation_calls_plainly(&[0x00a0_0f30, 0x00a0_1000, 0x00a0_cd50, 0x0040_8860])
            .float_ulps(ULPS)
    }
}

// ---------------------------------------------------------------------------------------------
// The small functions.

#[test]
fn set_strip_diff() {
    Diff::new(engine_core::entry!(
        0x0077_a450,
        hud::compass::set_strip(Ptr<_>, f32, f32)
    ))
    .families()
    .inputs(&[Input::F32, Input::F32])
    .setup(|e, case| {
        world(e);
        let strip = e.mem.alloc(0x90);
        vec![strip, case.f32(0).to_bits(), case.f32(1).to_bits()]
    })
    .run()
    .assert_same();
}

#[test]
fn player_marker_diff() {
    Diff::new(engine_core::entry!(
        0x0077_a400,
        hud::compass::player_marker(Ptr<_>) -> Ptr
    ))
    .families()
    .inputs(&[Input::Of(&[0, 1, 3])])
    .setup(|e, case| {
        world(e);
        let me = e.mem.alloc(0x720);
        e.mem.set_u32(me + 0x6f4, 0x0300_3f00);
        e.mem.set_u32(me + 0x708 + 8, case.word(0));
        // The list's data (+4): its first entry, read by the real `00558dd0`.
        let entry = e.mem.alloc(0x10);
        e.mem.set_u32(entry, 0x0300_3f40);
        e.mem.set_u32(me + 0x708 + 4, entry);
        vec![me]
    })
    .run()
    .assert_same();
}

/// The player: its vtable's heading (+0x2bc, the case's float), position (+0x1f4), sneaking
/// (+0x22c) and reward (+0x488); its own marker list (+0x708) empty and fallback (+0x6f4) `own`.
fn player(e: &mut Engine, heading: f32, position: [f32; 3], sneaking: bool, own: u32) -> u32 {
    let vt = vtable(e, 0);
    let me = e.mem.alloc(0x720);
    e.mem.set_u32(me, vt);
    e.mem.set_u32(me + 0x6f4, own);
    let at = e.mem.alloc(12);
    for (i, v) in (0u32..).zip(position) {
        e.mem.set_f32(at + 4 * i, v);
    }
    e.register_double(VT_FN + 0x2bc, move |e, a| {
        log(e, VT_FN + 0x2bc, a);
        ret_f(f64::from(heading))
    });
    e.register_double(VT_FN + 0x1f4, move |_, _| ret(at));
    e.register_double(VT_FN + 0x22c, move |_, _| ret(u32::from(sneaking)));
    e.register_double(VT_FN + 0x488, |e, a| {
        log(e, VT_FN + 0x488, a);
        Ret::default()
    });
    e.mem.set_u32(0x011d_ea3c, me);
    me
}

/// The heading of a vector (`004b1550`): a stand-in `0.5 x - 0.25 y`, logged with the vector's
/// three floats (its address is a stack slot, placed differently on the two sides).
fn heading_of(e: &mut Engine) {
    e.register_double(0x004b_1550, |e, a| {
        let vector = [e.mem.u32(a[0]), e.mem.u32(a[0] + 4), e.mem.u32(a[0] + 8)];
        log(e, 0x004b_1550, &vector);
        let (x, y) = (e.mem.f32(a[0]), e.mem.f32(a[0] + 4));
        ret_f(0.5 * f64::from(x) - 0.25 * f64::from(y))
    });
}

#[test]
fn compass_angle_diff() {
    Diff::new(engine_core::entry!(
        0x0077_8b80,
        hud::compass::compass_angle(f32, f32, f32, f32, f32, f32) -> f32
    ))
    .families()
    .inputs(&[
        Input::F32,
        Input::F32Swept(0x4120_0000),
        Input::F32Swept(0x3f80_0000),
        Input::F32Swept(0xc0a0_0000),
        Input::F32Swept(0x4000_0000),
        Input::F32Swept(0),
        Input::Of(&[0, 0x4049_0fdb, 0xc049_0fdb, 0x40c9_0fdb, 0x3f00_0000]),
    ])
    .float_sweep(64)
    .setup(|e, case| {
        world(e);
        heading_of(e);
        player(e, f32::from_bits(case.word(6)), [0.0; 3], false, 0);
        (0..6).map(|i| case.f32(i).to_bits()).collect()
    })
    .run()
    .assert_same();
}

#[test]
fn blink_tile_diff() {
    // The tile's `_AlphaDown` (none, 0 rising, 1 falling, 3 held, 2 other); its alpha; its next
    // time; the period (clamped to 100 or 900, below or above the breakpoint 400); the amplitude;
    // the HUD's alpha (0, 200, NaN); the guard of `_AlphaDown` set or not.
    Diff::new(engine_core::entry!(
        0x0077_8c20,
        hud::compass::blink_tile(Ptr<_>, f32, f32)
    ))
    .families()
    .inputs(&[
        Input::Of(&[0xffff_ffff, 0, 1, 3, 2]),
        Input::Of(&[0, 0x4248_0000, 0xc000_0000, 0x4348_0000]),
        Input::Of(&[0, 0x459c_4000, 0x45fa_0000]),
        Input::Of(&[
            0x42c8_0000,
            0x4120_0000,
            0x43c8_0000,
            0x4461_0000,
            0x4500_0000,
            0x7fc0_0000,
        ]),
        Input::Of(&[0x4348_0000, 0x4220_0000, 0]),
        Input::Of(&[0x4348_0000, 0, 0x7fc0_0000]),
        Input::Flag,
    ])
    .max_cases(240)
    .setup(|e, case| {
        world(e);
        e.mem.set_u32(0x011d_979c, case.word(5));
        if case.word(6) == 1 {
            e.mem.set_u32(0x011d_9e58, 1);
            e.mem.set_u32(0x011d_9e54, 10_000 + 0x500);
        }
        let t = tile(e);
        // The tile's traits, made and their floats written (the value setters are stand-ins).
        let alpha_down = 10_000 + 0x500;
        let set = |e: &mut Engine, id: u32, bits: u32| {
            let value = e.call(0x00a0_1000, &args![t, id]).u32();
            e.mem.set_u32(value + 8, bits);
        };
        if case.word(0) != 0xffff_ffff {
            set(e, alpha_down, (case.word(0) as f32).to_bits());
        }
        set(e, 0xfa9, case.word(1));
        set(e, 0x100e, case.word(2));
        e.mem.set_u32(LOG, 0);
        vec![t, case.word(3), case.word(4)]
    })
    .run()
    .assert_same();
}

// ---------------------------------------------------------------------------------------------
// The compass.

/// A map marker's reference: vtable (+0x1f4 its position, +0x48 the changed mark), flags word +8.
fn reference(e: &mut Engine, base: u32, position: [f32; 3], flags: u32) -> u32 {
    let vt = vtable(e, base);
    let r = e.mem.alloc(0x60);
    e.mem.set_u32(r, vt);
    e.mem.set_u32(r + 8, flags);
    let at = e.mem.alloc(12);
    for (i, v) in (0u32..).zip(position) {
        e.mem.set_f32(at + 4 * i, v);
    }
    e.register_double(VT_FN + base + 0x1f4, move |_, _| ret(at));
    e.register_double(VT_FN + base + 0x48, move |e, a| {
        log(e, VT_FN + base + 0x48, a);
        Ret::default()
    });
    r
}

/// The world of a compass case. `w`: [0] sneaking; [1] the markers' pattern; [2] the pools'
/// tiles (0, 1, 2 or 6 each); [3] JIP (0 off, 1 the discover event with a user, 2 JIP's name);
/// [4] the menu shown, the opacity fixed, the HUD 200 wide (bits 0, 1, 2); [5] the VATS actors'
/// pattern (0x40: the first two out of range); [6] the actor mode; [7] the player's heading (bits).
/// `f`: the player's x.
fn compass_world(e: &mut Engine, case: &Case) -> Vec<u32> {
    world(e);
    heading_of(e);
    let w = |i: usize| case.word(i);
    let heading = f32::from_bits(case.word(7));
    let px = case.f32(8);
    let own = reference(e, 0x900, [px + 30.0, 5.0, 2.0], 0);
    let me = player(e, heading, [px, 1.0, 0.0], w(0) == 1, own);
    // The process: its heading 0.25, the actor mode.
    let process = e.mem.alloc(0x40);
    let actor_mode = w(6);
    e.register_double(0x008d_6f30, move |e, a| {
        log(e, 0x008d_6f30, a);
        // An actor whose word +0x5c is 1 has no process.
        if e.mem.u32(a[0] + 0x5c) == 1 {
            ret(0)
        } else {
            ret(process)
        }
    });
    e.register_double(0x0055_5ad0, |e, a| {
        log(e, 0x0055_5ad0, a);
        ret_f(0.25)
    });
    e.register_double(0x0042_5fd0, move |e, a| {
        log(e, 0x0042_5fd0, a);
        ret(actor_mode)
    });
    // The HUD menu.
    let menu = e.mem.alloc(0x300);
    e.mem.set_u32(0x011d_96c0, menu);
    // Wider than 175 with bit 2, so the quest marker's overshoot cap can apply.
    e.mem
        .set_i32(menu + 0x214, if w(4) & 4 != 0 { 200 } else { 100 });
    e.mem.set_i32(menu + 0x218, 70);
    e.mem.set_u32(0x011f_2250 + 8, (w(4) >> 1) & 1);
    // The compass tile: its strip (slot +0x20) a block, or none.
    let compass = e.mem.alloc(0x40);
    let cvt = vtable(e, 0xa00);
    e.mem.set_u32(compass, cvt);
    let strip = if w(1) & 1 == 0 { e.mem.alloc(0x90) } else { 0 };
    e.register_double(VT_FN + 0xa00 + 0x20, move |_, _| ret(strip));
    // The pools: landmarks and ticks from their first node (+4), targets from +8.
    let sizes = [0u32, 1, 2, 6];
    let n = sizes[(w(2) & 3) as usize];
    let pool = |e: &mut Engine, at: u32| -> u32 {
        let tiles: Vec<u32> = (0..n).map(|_| tile(e)).collect();
        let p = e.mem.alloc(0x10);
        let first = list(e, &tiles);
        e.mem.set_u32(p + at, first);
        p
    };
    let landmarks = pool(e, 4);
    let objectives = pool(e, 8);
    let ticks = pool(e, 4);
    for at in [0x0057_cbe0u32, 0x0068_3520] {
        e.register_double(at, move |e, a| {
            log(e, at, a);
            let node = e.mem.u32(a[1]);
            e.mem.set_u32(a[1], e.mem.u32(node + 4));
            ret(node)
        });
    }
    // The map markers: four, placed by the pattern: inside the radius or not, visible, a travel
    // marker, a hidden reference (flag 0x800), out of range.
    let pattern = w(1);
    let mut records = vec![];
    for k in 0..4u32 {
        let x = px + [3.0, 15.0, -20.0, 500.0][k as usize];
        let y = [2.0, -6.0, 9.0, 1.0][k as usize];
        let flags = if k == 1 && pattern & 2 != 0 { 0x800 } else { 0 };
        let r = reference(e, 0x100 * (k + 1), [x, y, 1.0], flags);
        e.mem.set_f32(r + 0x40, [5.0, 20.0, 8.0, 3.0][k as usize]);
        let data = e.mem.alloc(8);
        // Visible (byte 0) and travel (byte 1) by the pattern.
        e.mem.set_u8(data, u8::from(pattern & (4 << k) != 0));
        e.mem.set_u8(data + 1, u8::from(pattern & (0x40 << k) != 0));
        let record = e.mem.alloc(8);
        e.mem.set_u32(record, data);
        e.mem.set_u32(record + 4, r);
        records.push(record);
    }
    let markers = list(e, &records);
    e.register_double(0x0094_8030, move |e, a| {
        log(e, 0x0094_8030, a);
        ret(markers)
    });
    e.register_double(0x0043_8ef0, |e, a| ret(u32::from(e.mem.u8(a[0] + 1))));
    e.register_double(0x0043_8ed0, |e, a| ret(u32::from(e.mem.u8(a[0]))));
    e.register_double(0x0044_de40, |e, a| {
        log(e, 0x0044_de40, a);
        e.mem.set_u8(a[0], 1);
        Ret::default()
    });
    e.register_double(0x0044_de80, |e, a| {
        log(e, 0x0044_de80, a);
        e.mem.set_u8(a[0] + 1, 1);
        Ret::default()
    });
    e.register_double(0x0056_8cb0, |e, a| ret_f(f64::from(e.mem.f32(a[0] + 0x40))));
    e.register_double(0x0043_8eb0, |e, a| {
        log(e, 0x0043_8eb0, a);
        ret(0x0300_3e00)
    });
    e.register_double(0x0040_3df0, |e, a| {
        log(e, 0x0040_3df0, a);
        ret(0x0300_3e40)
    });
    for at in [
        0x0076_b960u32,
        0x005f_5950,
        0x004d_5c60,
        0x0077_2aa0,
        0x0083_fd60,
    ] {
        logging(e, at, Ret::default());
    }
    // The marker helper also logs the state it is handed (reference, two bytes).
    e.register_double(0x0077_2aa0, |e, a| {
        let state = [e.mem.u32(a[0]), e.mem.u32(a[0] + 4) & 0xffff];
        log(e, 0x0077_2aa0, &state);
        Ret::default()
    });
    // JIP.
    match w(3) {
        1 => {
            e.on_site(SiteSlot::new(Owner::Jip, 0x0077_955d), |e, a| {
                log(e, 0x0077_955d, a);
                None
            });
            e.add_hook_user(Hook::LocationDiscover, true);
        }
        2 => {
            e.set_jip_patches(engine_core::extend::jip_patches::INSTALLED);
            e.on_site_for_test(0x0077_951e, |e, a| {
                log(e, 0x0077_951e, a);
                Some(0x0300_3e80)
            });
        }
        _ => {}
    }
    // The quest targets: three records, the second an actor (form type 0x1c), the third none.
    let mut targets = vec![];
    for k in 0..3u32 {
        let r = if k == 2 {
            0
        } else {
            reference(e, 0x500 + 0x100 * k, [px - 10.0 * k as f32, 25.0, 0.0], 0)
        };
        if r != 0 {
            e.mem.set_u8(r + 4, if k == 1 { 0x1c } else { 0x20 });
        }
        let record = e.mem.alloc(8);
        e.mem.set_u32(record, r);
        targets.push(record);
    }
    let target_list = list(e, &targets);
    e.register_double(0x0095_2ba0, move |_, _| ret(target_list));
    e.register_double(0x005c_bb70, |e, a| ret(e.mem.u32(a[0])));
    e.register_double(0x0040_1170, |e, a| ret(u32::from(e.mem.u8(a[0] + 4))));
    let shown_byte = e.mem.alloc(4);
    e.mem.set_u8(shown_byte, 1);
    e.register_double(0x0040_8d60, move |_, _| ret(shown_byte));
    e.register_double(0x0040_ebd0, |e, a| {
        log(e, 0x0040_ebd0, a);
        let (x, y) = (f32::from_bits(a[0]), f32::from_bits(a[1]));
        ret_f(f64::from(x.min(y)))
    });
    e.register_double(0x00a0_3c90, move |_, _| ret(compass));
    let shown = w(4) & 1;
    e.register_double(0x0059_bb30, move |_, _| ret(shown));
    // The VATS actors: three, by the pattern: with 3D or not, no process, inactive, an actor value
    // that asks the entry point, hostile.
    let vats_pattern = w(5);
    let mut actors = vec![];
    for k in 0..3u32 {
        let base = 0xb00 + 0x100 * k;
        // With bit 0x40 the first two are 37 and 45 units away: past the actor mode's tick range (35)
        // but inside the landmark range (40), and past both.
        let along = match (vats_pattern & 0x40 != 0, k) {
            (true, 0) => 37.0,
            (true, 1) => 45.0,
            _ => 4.0 * (k + 1) as f32,
        };
        let a = reference(e, base, [px + along, -3.0, 0.0], 0);
        let has_3d = u32::from(vats_pattern & (1 << k) == 0);
        e.register_double(VT_FN + base + 0x1d0, move |_, _| ret(has_3d));
        e.mem
            .set_u32(a + 0x5c, u32::from(vats_pattern == 7 && k == 0));
        // The actor value owner at +0xa4.
        let avt = vtable(e, base + 0x80);
        e.mem.set_u32(a + 0xa4, avt);
        let asks = u32::from(vats_pattern & (8 << k) != 0);
        e.register_double(VT_FN + base + 0x80 + 8, move |_, args| {
            ret(if args[1] == 0x31 { asks } else { 0 })
        });
        let record = e.mem.alloc(8);
        e.mem.set_u32(record, a);
        e.mem.set_u8(record + 4, u8::from(k == 1));
        e.mem
            .set_u8(record + 5, u8::from(vats_pattern == 6 && k == 2));
        actors.push(record);
    }
    let vats = list(e, &actors);
    e.register_double(0x0059_5d10, move |_, _| ret(vats));
    e.register_double(0x005a_e380, |_, _| ret(3));
    let active = u32::from(vats_pattern != 5);
    e.register_double(0x0045_0ff0, move |_, _| ret(active));
    e.register_double(0x005e_58f0, |e, a| {
        log(e, 0x005e_58f0, a);
        e.mem.set_f32(a[2], 1.0);
        Ret::default()
    });
    let _ = me;
    vec![compass, landmarks, objectives, ticks]
}

#[test]
fn update_compass_diff() {
    Diff::new(engine_core::entry!(
        0x0077_9070,
        hud::compass::update_compass(Ptr<_>, Ptr<_>, Ptr<_>, Ptr<_>)
    ))
    .families()
    .inputs(&[
        Input::Flag,
        Input::Of(&[0, 1, 2, 0x3c, 0x5e, 0x3ff]),
        Input::Of(&[0, 1, 2, 3]),
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0, 1, 2, 3, 4, 5]),
        Input::Of(&[0, 1, 2, 5, 6, 7, 0x38, 0x40, 0x41]),
        Input::Flag,
        // The headings: 14 and 13.4 put the player's own marker in the strip (inside and past the
        // inner width), 5 and 4.4 the hostile tick.
        Input::Of(&[
            0x3f00_0000,
            0x4160_0000,
            0x4156_6666,
            0x40a0_0000,
            0x408c_cccd,
        ]),
        Input::F32Swept(0x4120_0000),
    ])
    // About five seconds a case on this machine's debug build: every value of each word once and
    // random combinations to 72, and four all-random float cases.
    .max_cases(72)
    .float_sweep(4)
    .max_calls(200_000)
    .setup(compass_world)
    .run()
    .assert_same();
}
