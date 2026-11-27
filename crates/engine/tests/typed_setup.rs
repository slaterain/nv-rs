//! The differential test of every typed function of the set-up family (`engine_typed::setup`)
//! against its translation. The world: the shadow scene node (entry 0 of the table at `011f91c8`)
//! with its shadow-light list (real `NiTListItem`s, the fences, the iterator), its lights (each with
//! its `NiLight`, shadow scene and the fade, weight and count members), the player and the high
//! process list's actors (each with its own vtable whose process and kind slots answer from the
//! actor's own words), the camera root, the settings, the render flags and the globals the exe's
//! small getters read; and for the frame's target, the texture manager's target with its size slots.
//!
//! What stands in: the depth-pass test (its answers from the case, one bit a call), the renderer's
//! and the scene node's own passes, the rendering system's stages, the lock, the actor tests, the
//! lights' updates, the list's add, move and empty, the allocation, the interlocked words, the
//! menu-mode test `Main::UpdateOffscreenInterface` asks first, the lights family's object add and
//! the scene family's update (both by address here: their own tests cover them).
//!
//! The getters the typed code reads in place are left out of the translation's calls
//! ([`Diff::ignore_calls`]); what they read and write still compares, as all memory does.

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::{Case, Diff, Input};
use engine_typed::setup::{self, frame, lists, shadows, world};

const LOG: u32 = 0x0300_0000;
/// Where the stand-in slot functions live, one per slot.
const SLOT_FN: u32 = 0x00ff_4000;
const MAX_CALLS: usize = 50_000;

const ONE: u32 = 0x3f80_0000;
const HALF: u32 = 0x3f00_0000;
const TWO: u32 = 0x4000_0000;
const TENTH: u32 = 0x3dcc_cccd;
const NAN: u32 = 0x7fc0_0000;

/// The globals and settings the code reads (see `engine_typed::setup::shadows`).
const SCENE_NODE_TABLE: u32 = 0x011f_91c8;
const RENDERER: u32 = 0x011f_4748;
const TEXTURE_MANAGER: u32 = 0x011f_91a8;
const RENDER_FLAGS: u32 = 0x011f_91d8;
const DEPTH_STATE: u32 = 0x011d_ef08;
const DEPTH_REQUEST: u32 = 0x011d_ea2a;
const SETTING_SHADOWS: u32 = 0x011c_74d4;
const SETTING_KEEP_SAVED: u32 = 0x011c_71dc;
const SETTING_ADD_OBJECTS: u32 = 0x011c_7664;
const SETTING_LIMIT_INTERIOR: u32 = 0x011c_72d8;
const SETTING_LIMIT_EXTERIOR: u32 = 0x011c_72e4;
const WORLD: u32 = 0x011d_ea10;
const PLAYER: u32 = 0x011d_ea3c;
const VANITY_ACTIVE: u32 = 0x011e_07b8;
const HIGH_PROCESS_LIST: u32 = 0x011e_0e80;
const EXTRA_KEY: u32 = 0x0120_31e4;
const RANGE_SCALE: u32 = 0x0102_1928;
const CAMERA_ROOT: u32 = 0x011d_eb7c;
const SAVED_OBJECT: u32 = 0x011c_a438;
const WEIGHT_CUTOFF: u32 = 0x0108_2cb0;
const FADE_TIME: u32 = 0x011a_d834;
const FADE_LOW: u32 = 0x0101_ffa0;
const FADE_HIGH: u32 = 0x0106_b9e8;
const INTERFACE_OFFSCREEN: u32 = 0x011f_91a4;
const RATIO_SCALE: u32 = 0x0108_2d20;

/// The getters the typed code reads in place.
const READ_IN_PLACE: &[u32] = &[
    0x0052_5420,
    0x005b_f770,
    0x0045_0b80,
    0x0043_c4b0,
    0x004a_0ea0,
    0x0043_d4d0,
    0x0040_8d60,
    0x005f_36f0,
    0x0095_0090,
    0x004e_af60,
    0x0096_11e0,
    0x0047_2380,
    0x0050_ea90,
    0x0050_0940,
    0x0084_d030,
    0x0040_4010,
    0x0045_bbe0,
    0x0041_fd00,
    0x0054_f4c0,
    0x004e_20b0,
    0x004e_6560,
    0x0097_9260,
    0x0045_0f90,
    0x0045_6610,
    0x004e_a970,
    0x004e_6540,
    0x005b_f820,
    0x0045_c670,
    0x0066_29f0,
    0x008d_80e0,
    0x004e_6580,
    0x0045_bb80,
    0x0052_4c90,
];

fn ret(eax: u32) -> Ret {
    Ret {
        eax,
        ..Ret::default()
    }
}

fn log(e: &mut Engine, tag: u32, word: u32) {
    let n = e.mem.u32(LOG);
    if n >= 0x700 {
        return;
    }
    e.mem.set_u32(LOG + 4 + 8 * n, tag);
    e.mem.set_u32(LOG + 8 + 8 * n, word);
    e.mem.set_u32(LOG, n + 1);
}

/// A stand-in that logs its words under `TAG` and answers the word at `LOG + 0x3c00 + 4 * (TAG &
/// 0xff)`.
fn logged<const TAG: u32>(e: &mut Engine, a: &[u32]) -> Ret {
    log(e, TAG, a.first().copied().unwrap_or(0));
    for w in a.iter().skip(1) {
        log(e, TAG | 0x1000_0000, *w);
    }
    ret(e.mem.u32(LOG + 0x3c00 + 4 * (TAG & 0xff)))
}

fn set_logged(e: &mut Engine, tag: u32, value: u32) {
    e.mem.set_u32(LOG + 0x3c00 + 4 * (tag & 0xff), value);
}

/// A `thiscall` stand-in that logs its words under `TAG` and answers its object's word at `OFF`.
fn own_word<const TAG: u32, const OFF: u32>(e: &mut Engine, a: &[u32]) -> Ret {
    log(e, TAG, a[0]);
    for w in &a[1..] {
        log(e, TAG | 0x1000_0000, *w);
    }
    ret(e.mem.u32(a[0].wrapping_add(OFF)))
}

/// A slot stand-in: logs `this` and its words under the slot, answers the object's word at
/// `+0x100 + slot / 4 % 64 * 4`.
fn slot<const SLOT: u32>(e: &mut Engine, a: &[u32]) -> Ret {
    let this = a[0];
    log(e, 0x1_0000 | SLOT, this);
    for w in &a[1..] {
        log(e, 0x2_0000 | SLOT, *w);
    }
    ret(e.mem.u32(this + 0x100 + (SLOT / 4 % 64) * 4))
}

/// The slots the code calls: the destructor (+4) and the reference's node (also +4), the target's
/// sizes (+0x94, +0x98), the renderer's state set (+0xac) and group (+0xc8), the actor's process
/// (+0x1d0) and kind (+0x214).
const SLOTS: [u32; 7] = [0x4, 0x94, 0x98, 0xac, 0xc8, 0x1d0, 0x214];

/// An object (0x300 bytes, slots answer from +0x100) with its own vtable.
fn object(e: &mut Engine) -> u32 {
    let vt = e.mem.alloc(0x300);
    for s in SLOTS {
        e.mem.set_u32(vt + s, SLOT_FN + s);
    }
    let o = e.mem.alloc(0x300);
    e.mem.set_u32(o, vt);
    o
}

/// Sets what `slot` of `o` answers.
fn answer(e: &mut Engine, o: u32, slot: u32, value: u32) {
    e.mem.set_u32(o + 0x100 + (slot / 4 % 64) * 4, value);
}

/// The depth-pass test: answers bit `n` of the word at `LOG + 0x3a00` on its `n`th call.
fn depth_passes(e: &mut Engine, _a: &[u32]) -> Ret {
    let n = e.mem.u32(LOG + 0x3a04);
    e.mem.set_u32(LOG + 0x3a04, n + 1);
    log(e, 0x7f, n);
    ret((e.mem.u32(LOG + 0x3a00) >> n) & 1)
}

fn stand_ins(e: &mut Engine) {
    e.map(LOG, 0x4000);
    for page in [
        0x0101_f000,
        0x0102_1000,
        0x0106_b000,
        0x0108_2000,
        0x011a_d000,
        0x011c_7000,
        0x011c_a000,
        0x011d_e000,
        0x011e_0000,
        0x011f_1000,
        0x011f_4000,
        0x011f_9000,
        0x0120_2000,
        0x0120_3000,
        0x0101_1000,
        0x0101_2000,
        0x0102_6000,
        0x0103_4000,
        0x0107_1000,
        0x011a_9000,
        0x011d_9000,
        0x011f_2000,
        0x011f_f000,
    ] {
        e.map(page, 0x1000);
    }
    macro_rules! each_slot {
        ($($s:literal)*) => { $( e.register(SLOT_FN + $s, slot::<$s>); )* };
    }
    each_slot!(0x4 0x94 0x98 0xac 0xc8 0x1d0 0x214);
    // The reference counts' interlocked imports.
    e.register(0x00fd_f1c0, |e, a| {
        let n = e.mem.u32(a[0]).wrapping_add(1);
        e.mem.set_u32(a[0], n);
        log(e, 0x100, a[0]);
        ret(n)
    });
    e.register(0x00fd_f1c4, |e, a| {
        let n = e.mem.u32(a[0]).wrapping_sub(1);
        e.mem.set_u32(a[0], n);
        log(e, 0x101, a[0]);
        ret(n)
    });
    // The allocation over the test heap (0 when the answer word says so) and the light's
    // constructor, which answers its memory.
    e.register(0x00aa_13e0, |e, a| {
        log(e, 0x102, a[0]);
        if e.mem.u32(LOG + 0x3a08) != 0 {
            return ret(0);
        }
        let o = object(e);
        ret(o)
    });
    e.register(0x00b9_fda0, |e, a| {
        log(e, 0x103, a[0]);
        ret(a[0])
    });
    e.register(0x005b_f7b0, depth_passes);
    // The menu-mode test: never in menu mode.
    e.register(0x0070_2360, |e, _| {
        log(e, 0x104, 0);
        ret(0)
    });
    // The player's 3D: its word +0x1e0 (third person) or +0x1e4 (first person).
    e.register(0x0095_0bb0, |e, a| {
        log(e, 0x105, a[0]);
        log(e, 0x105, a[1]);
        ret(e.mem.u32(a[0] + 0x1e0 + 4 * (a[1] & 1)))
    });
    for (addr, f) in [
        (
            0x00b5_b1f0u32,
            logged::<0x01> as fn(&mut Engine, &[u32]) -> Ret,
        ),
        (0x00b6_ee70, logged::<0x02>),
        (0x00b4_f570, logged::<0x03>),
        (0x00b5_b880, logged::<0x04>),
        (0x00ba_30f0, logged::<0x05>),
        (0x00ba_3130, logged::<0x06>),
        (0x0040_fbf0, logged::<0x07>),
        (0x0040_fba0, logged::<0x08>),
        (0x00a6_94a0, logged::<0x09>),
        (0x00b9_bb10, logged::<0x0a>),
        (0x00b9_d150, logged::<0x0b>),
        (0x00b9_e970, logged::<0x0c>),
        (0x00b9_dfc0, logged::<0x0d>),
        (0x00b9_fba0, logged::<0x0e>),
        (0x00ba_02b0, logged::<0x0f>),
        (0x0045_4b30, logged::<0x10>),
        (0x00b5_d300, logged::<0x11>),
        (0x00ba_0110, logged::<0x12>),
        (0x00a5_9c60, logged::<0x13>),
        (0x00b9_dcb0, logged::<0x14>),
        (0x00a5_a310, logged::<0x15>),
        (0x00c1_b4a0, logged::<0x16>),
        (0x00b7_02e0, logged::<0x17>),
        (0x00e7_4d40, logged::<0x18>),
        (0x00b6_d4c0, logged::<0x19>),
        (0x00b9_e770, logged::<0x1a>),
        (0x00b6_e110, logged::<0x1b>),
        (0x0087_31d0, logged::<0x1c>),
        (0x004d_ee10, logged::<0x1d>),
        (0x004d_ee70, logged::<0x1e>),
        (0x0071_2e60, logged::<0x20>),
        (0x00b6_b260, logged::<0x21>),
        (0x00b6_b8d0, logged::<0x22>),
        (0x00b6_b890, logged::<0x23>),
        (0x0040_4eb0, logged::<0x24>),
        (0x0040_4ee0, logged::<0x25>),
    ] {
        e.register(addr, f);
    }
    // The world frame's calls (answers set per test with `set_logged`).
    for (addr, f) in [
        (
            0x004d_c310u32,
            logged::<0x26> as fn(&mut Engine, &[u32]) -> Ret,
        ),
        (0x0068_3a60, logged::<0x27>),
        (0x0071_4a00, logged::<0x28>),
        (0x00a8_1a80, logged::<0x29>),
        (0x0071_4960, logged::<0x2a>),
        (0x0071_0ab0, logged::<0x2b>),
        (0x00c5_2020, logged::<0x2c>),
        (0x00b5_4000, logged::<0x2d>),
        (0x008d_6f50, logged::<0x2e>),
        (0x005d_8b10, logged::<0x2f>),
        (0x00a6_9400, logged::<0x33>),
        (0x0063_3c90, logged::<0x34>),
        (0x00ec_658f, logged::<0x35>),
        (0x0055_8310, logged::<0x36>),
        (0x0044_0460, logged::<0x37>),
        (0x0047_6c90, logged::<0x38>),
        (0x0054_68f0, logged::<0x39>),
        (0x0045_18e0, logged::<0x3a>),
        (0x005f_9ad0, logged::<0x3b>),
        (0x005f_9af0, logged::<0x3c>),
        (0x0070_ec90, logged::<0x3d>),
        (0x004e_1bc0, logged::<0x3e>),
        (0x004e_2190, logged::<0x3f>),
    ] {
        e.register(addr, f);
    }
    for (addr, f) in [
        (
            0x004e_2120u32,
            logged::<0x40> as fn(&mut Engine, &[u32]) -> Ret,
        ),
        (0x0087_49b0, logged::<0x41>),
        (0x0087_4ac0, logged::<0x42>),
        (0x0087_4b50, logged::<0x43>),
        (0x0070_5910, logged::<0x44>),
        (0x0055_85e0, logged::<0x45>),
        (0x0070_6e70, logged::<0x46>),
        (0x00b4_f540, logged::<0x47>),
        (0x0087_3200, logged::<0x48>),
        (0x004e_bbc0, logged::<0x49>),
        (0x00b8_b390, logged::<0x4a>),
        (0x0087_46d0, logged::<0x4b>),
        (0x005b_9b00, logged::<0x4c>),
        (0x00b6_3a90, logged::<0x4d>),
        (0x0087_4920, logged::<0x4e>),
        (0x0087_4c10, logged::<0x4f>),
        (0x0087_5110, logged::<0x50>),
        (0x0087_5e40, logged::<0x51>),
        (0x00a6_fdb0, logged::<0x52>),
        (0x00b8_b1e0, logged::<0x53>),
        (0x0087_5fd0, logged::<0x54>),
        (0x0045_3470, logged::<0x55>),
        (0x005b_ac20, logged::<0x56>),
        (0x0070_5a00, logged::<0x57>),
        (0x0087_6a20, logged::<0x58>),
        (0x0087_6830, logged::<0x59>),
        (0x00b5_40d0, logged::<0x5a>),
        (0x0071_48c0, logged::<0x5b>),
        (0x0087_6850, logged::<0x5c>),
    ] {
        e.register(addr, f);
    }
    // The actor tests and the process's node answer from the object's own words.
    e.register(0x008c_51c0, own_word::<0x30, 0x2f0>);
    e.register(0x0087_f200, own_word::<0x31, 0x2f4>);
    e.register(0x00a5_bdd0, own_word::<0x32, 0x2f8>);
    // The constants the getters and the code read.
    e.mem.set_u32(FADE_TIME, ONE);
    e.mem.set_u64(FADE_LOW, 0.1f64.to_bits());
    e.mem.set_u64(FADE_HIGH, 0.9f64.to_bits());
    e.mem.set_u64(WEIGHT_CUTOFF, 0.25f64.to_bits());
    e.mem.set_u64(RANGE_SCALE, 2.0f64.to_bits());
    e.mem.set_u64(RATIO_SCALE, 1.0f64.to_bits());
    e.mem.set_u32(RENDERER, 0x0123_4560);
    e.mem.set_u32(TEXTURE_MANAGER, 0x0123_4570);
    e.mem.set_u32(EXTRA_KEY, 0x77);
    e.mem.set_u32(0x011f_4288, 0);
}

/// The families every test of this file runs with.
trait Common {
    fn common(self) -> Self;
}

impl Common for Diff {
    fn common(self) -> Diff {
        self.family(&setup::SETUP)
            .family(&leaf::LEAVES)
            .ignore_calls(READ_IN_PLACE)
            .goes_by_address(&[0x00ba_0110, 0x00a5_9c60])
            // The scene node family's members the set-up calls typed: its passes, the counted list's
            // `RemoveAll`, the list move and the shadow scene set.
            .goes_by_address(&[
                0x00b5_b1f0,
                0x00b5_b880,
                0x00e7_4d40,
                0x00b5_a8d0,
                0x00b5_af30,
            ])
            .max_calls(MAX_CALLS)
    }
}

/// A light's `NiLight` (or geometry): an object with its flags word.
fn flagged(e: &mut Engine, culled: bool) -> u32 {
    let o = object(e);
    e.mem.set_u32(o + 0x30, u32::from(culled) | 0x40);
    o
}

/// A shadow light from pattern `k`: its geometry (`spLight`) and source (`spShadowScene`), each
/// missing, culled or not; its target, elapsed time, weight and count; a dynamic shadow map (with its
/// reference count) or none.
fn light(e: &mut Engine, k: u32, floats: &[u32; 3]) -> u32 {
    let l = object(e);
    let geometry = match k % 3 {
        0 => 0,
        1 => flagged(e, false),
        _ => flagged(e, true),
    };
    let source = match (k / 3) % 3 {
        0 => 0,
        1 => flagged(e, false),
        _ => flagged(e, true),
    };
    if source != 0 {
        // The source's position and a bound.
        for (i, w) in [ONE, TWO, HALF].iter().enumerate() {
            e.mem.set_u32(source + 0x8c + 4 * i as u32, *w);
        }
        let bound = e.mem.alloc(0x10);
        e.mem.set_u32(bound + 0xc, ONE);
        e.mem.set_u32(source + 0x20, bound);
    }
    e.mem.set_u32(l + 0xf8, geometry);
    e.mem.set_u32(l + 0x128, source);
    e.mem.set_u32(l + 0xd8, floats[0]);
    e.mem.set_u32(l + 0xdc, floats[1]);
    e.mem.set_u32(l + 0xd4, floats[2]);
    e.mem
        .set_u16(l + 0x110, if (k / 9) % 4 == 3 { 0xff } else { 2 });
    if (k / 9) % 2 == 1 {
        let map = object(e);
        e.mem.set_u32(map + 4, (k / 18) % 2 + 1);
        e.mem.set_u32(l + 0x10c, map);
    }
    l
}

/// The float triples the lights take in turn (target, elapsed, weight).
const LIGHT_FLOATS: [[u32; 3]; 6] = [
    [0, HALF, ONE],
    [ONE, HALF, HALF],
    [TENTH, ONE, 0x3e00_0000],
    [0xbf80_0000, 0x3e80_0000, NAN],
    [0x3f66_6666, TWO, TWO],
    [NAN, NAN, 0x3e80_0000],
];

/// The shadow scene node holding `lights` in its shadow-light list (null elements where a light is
/// 0), the fences at items `active` and `fading` (none past the end), set as entry 0 of the table.
fn scene_node(e: &mut Engine, lights: &[u32], active: usize, fading: usize) -> u32 {
    let node = object(e);
    let mut items = Vec::new();
    for &l in lights {
        items.push(e.mem.alloc(0xc));
        let item = *items.last().unwrap();
        e.mem.set_u32(item + 8, l);
    }
    for (i, &item) in items.iter().enumerate() {
        let next = items.get(i + 1).copied().unwrap_or(0);
        let previous = if i == 0 { 0 } else { items[i - 1] };
        e.mem.set_u32(item, next);
        e.mem.set_u32(item + 4, previous);
    }
    e.mem
        .set_u32(node + 0xc0, items.first().copied().unwrap_or(0));
    e.mem
        .set_u32(node + 0xc4, items.last().copied().unwrap_or(0));
    e.mem.set_u32(node + 0xc8, items.len() as u32);
    e.mem
        .set_u32(node + 0xd0, items.get(active).copied().unwrap_or(0));
    e.mem
        .set_u32(node + 0xd4, items.get(fading).copied().unwrap_or(0));
    e.mem.set_u32(SCENE_NODE_TABLE, node);
    node
}

/// `n` lights from pattern `k`, every fifth one null.
fn lights(e: &mut Engine, n: u32, k: u32) -> Vec<u32> {
    (0..n)
        .map(|i| {
            if (i + k) % 5 == 4 {
                0
            } else {
                light(e, i * 7 + k, &LIGHT_FLOATS[((i + k) % 6) as usize])
            }
        })
        .collect()
}

#[test]
fn setup_first_and_next_shadow_light() {
    // The list's length; the pattern of its lights (some null, some without an `NiLight`). The walk:
    // the first call, then the next until it answers 0 or eight calls.
    Diff::new(entry!(
        0x00b5_afc0,
        lists::first_shadow_light(Ptr<_>) -> u32
    ))
    .common()
    .inputs(&[Input::Of(&[0, 1, 2, 5]), Input::Of(&[0, 1, 2, 3, 4])])
    .setup(|e, case: &Case| {
        stand_ins(e);
        let l = lights(e, case.word(0), case.word(1));
        vec![scene_node(e, &l, 9, 9)]
    })
    .run()
    .assert_same();
    Diff::new(entry!(0x00b5_b010, lists::next_shadow_light(Ptr<_>) -> u32))
        .common()
        .inputs(&[
            Input::Of(&[0, 1, 2, 5]),
            Input::Of(&[0, 1, 2, 3, 4]),
            Input::Of(&[0, 1, 2, 9]),
        ])
        .setup(|e, case: &Case| {
            stand_ins(e);
            let l = lights(e, case.word(0), case.word(1));
            let node = scene_node(e, &l, 9, 9);
            // The iterator on item `case.word(2)` (none past the end).
            let mut item = e.mem.u32(node + 0xc0);
            for _ in 0..case.word(2) {
                if item != 0 {
                    item = e.mem.u32(item);
                }
            }
            e.mem.set_u32(node + 0xcc, item);
            vec![node]
        })
        .run()
        .assert_same();
}

#[test]
fn setup_drop_shadow_maps() {
    // The list's length and its lights' pattern (maps held once or twice, none, no `NiLight`).
    Diff::new(entry!(0x00b5_b060, lists::drop_lit_shadow_maps(Ptr<_>)))
        .common()
        .inputs(&[Input::Of(&[0, 1, 3, 6]), Input::Of(&[0, 1, 2, 3, 4, 9])])
        .setup(|e, case: &Case| {
            stand_ins(e);
            let l = lights(e, case.word(0), case.word(1));
            vec![scene_node(e, &l, 1, 0)]
        })
        .run()
        .assert_same();
    Diff::new(entry!(0x00b5_c3c0, lists::drop_shadow_maps(Ptr<_>)))
        .common()
        .inputs(&[Input::Of(&[0, 1, 3, 6]), Input::Of(&[0, 1, 2, 3, 4, 9])])
        .setup(|e, case: &Case| {
            stand_ins(e);
            let l = lights(e, case.word(0), case.word(1));
            vec![scene_node(e, &l, 1, 0)]
        })
        .run()
        .assert_same();
}

#[test]
fn setup_shadow_light_of() {
    // The list (empty, or lights some of which have the object as their shadow scene); which object
    // is asked for (one in the list, the null object, a new one); the object's reference count.
    Diff::new(entry!(
        0x00b5_cbd0,
        lists::shadow_light_of(Ptr<_>, u32) -> u32
    ))
    .common()
    .inputs(&[
        Input::Of(&[0, 3, 6]),
        Input::Of(&[0, 1, 2]),
        // The allocation succeeds (a failed one makes the exe and both versions read the null
        // light's members: a stop, nothing to compare).
        Input::Of(&[1]),
        Input::Of(&[1, 2]),
    ])
    .setup(|e, case: &Case| {
        stand_ins(e);
        let l = lights(e, case.word(0), 1);
        // Every light in the shadow list has its `NiLight` (a found one has its flags changed).
        for &x in l.iter().filter(|&&x| x != 0) {
            if e.mem.u32(x + 0xf8) == 0 {
                let ni = flagged(e, false);
                e.mem.set_u32(x + 0xf8, ni);
            }
        }
        let node = scene_node(e, &l, 9, 9);
        let object = match case.word(1) {
            0 => l
                .iter()
                .rev()
                .find(|&&x| x != 0)
                .map(|&x| e.mem.u32(x + 0x128))
                .unwrap_or(0),
            1 => 0,
            _ => {
                let o = object(e);
                e.mem.set_u32(o + 4, case.word(3));
                o
            }
        };
        e.mem.set_u32(LOG + 0x3a08, case.word(2) ^ 1);
        e.mem.set_u32(0x011f_9ea4, 1);
        vec![node, object]
    })
    .run()
    .assert_same();
}

#[test]
fn setup_sort_shadow_lights() {
    // The list's length and lights; the fences (both, neither, only one); the light given (one of the
    // list or none); the fade time; floats for the lights' targets and elapsed times.
    Diff::new(entry!(0x00b5_cde0, lists::sort_shadow_lights(Ptr<_>, u32)))
        .common()
        .inputs(&[
            Input::Of(&[0, 2, 6, 9]),
            Input::Of(&[0, 1, 2, 3]),
            Input::Of(&[0, 1, 2]),
            Input::Of(&[ONE, HALF, 0, NAN]),
            Input::F32,
            Input::F32Swept(HALF),
        ])
        .max_cases(400)
        .float_sweep(64)
        .setup(|e, case: &Case| {
            stand_ins(e);
            let mut l = lights(e, case.word(0), case.word(1));
            // Every light in the shadow list has its `NiLight` (the sort reads its flags).
            for &x in l.iter().filter(|&&x| x != 0) {
                if e.mem.u32(x + 0xf8) == 0 {
                    let ni = flagged(e, false);
                    e.mem.set_u32(x + 0xf8, ni);
                }
            }
            // The case's floats on the second light.
            if let Some(&x) = l.iter().skip(1).find(|&&x| x != 0) {
                e.mem.set_u32(x + 0xd8, case.f32(4).to_bits());
                e.mem.set_u32(x + 0xdc, case.f32(5).to_bits());
            }
            let (active, fading) = match case.word(1) {
                0 => (2, 1),
                1 => (9, 9),
                2 => (9, 0),
                _ => (1, 9),
            };
            let node = scene_node(e, &l, active, fading);
            e.mem.set_u32(FADE_TIME, case.word(3));
            let given = match case.word(2) {
                0 => 0,
                1 => l.iter().copied().find(|&x| x != 0).unwrap_or(0),
                _ => l.pop().unwrap_or(0),
            };
            vec![node, given]
        })
        .run()
        .assert_same();
}

#[test]
fn setup_sort_share_at_the_low_limit() {
    // A fading light whose share of the fade lands exactly on `low` (0.25, a float), just below
    // and just above it: only a share ABOVE `low` moves the light after the active fence. The
    // target at least `high` (1.0) and the elapsed time below the fade time (1.0), so the share is
    // the elapsed time.
    Diff::new(entry!(0x00b5_cde0, lists::sort_shadow_lights(Ptr<_>, u32)))
        .common()
        .inputs(&[
            Input::Of(&[0x3e80_0000, 0x3e7f_ffff, 0x3e80_0001, 0x3f40_0000]),
            Input::Of(&[ONE]),
        ])
        .setup(|e, case: &Case| {
            stand_ins(e);
            e.mem.set_u64(FADE_LOW, 0.25f64.to_bits());
            let l: Vec<u32> = (0..4)
                .map(|i| {
                    let ni = flagged(e, false);
                    let x = object(e);
                    e.mem.set_u32(x + 0xf8, ni);
                    e.mem
                        .set_u32(x + 0xd8, if i == 2 { case.word(1) } else { HALF });
                    e.mem
                        .set_u32(x + 0xdc, if i == 2 { case.word(0) } else { TWO });
                    x
                })
                .collect();
            // The active fence on the last light, so a light moved after it changes the order.
            vec![scene_node(e, &l, 3, 0), 0]
        })
        .run()
        .assert_same();
}

/// The world `prepare` runs in, from the case: the depth-pass answers, the request and the state;
/// the shadows setting and the caster limit (interior or exterior); the player's tests; the actors;
/// the lights; the saved object and its setting; the add-objects setting.
fn prepare_world(e: &mut Engine, case: &Case) -> Vec<u32> {
    stand_ins(e);
    let k = case.word(5);
    e.mem.set_u32(LOG + 0x3a00, case.word(0));
    e.mem.set_u8(DEPTH_REQUEST, (case.word(1) & 1) as u8);
    e.mem.set_u8(DEPTH_STATE, (case.word(1) >> 1) as u8);
    e.mem.set_u8(SETTING_SHADOWS + 4, (case.word(2) & 1) as u8);
    let world = e.mem.alloc(0x40);
    e.mem.set_u32(world + 0x34, case.word(3) & 1);
    e.mem.set_u32(WORLD, world);
    let limit = case.word(3) >> 1;
    e.mem.set_u32(SETTING_LIMIT_INTERIOR + 4, limit);
    e.mem.set_u32(SETTING_LIMIT_EXTERIOR + 4, limit + (k & 1));
    e.mem.set_u32(RENDER_FLAGS, 0x31);
    // The player: seen (+0x64a) or in vanity mode, refractive or not, its two models (the first-person
    // one missing in some cases).
    let player = object(e);
    e.mem.set_u8(player + 0x64a, (case.word(4) & 1) as u8);
    e.mem.set_u8(VANITY_ACTIVE, ((case.word(4) >> 1) & 1) as u8);
    e.mem.set_u32(player + 0x2f0, (case.word(4) >> 2) & 1);
    let third = flagged(e, false);
    let first = if k % 3 == 2 {
        0
    } else {
        flagged(e, k % 2 == 1)
    };
    e.mem.set_u32(player + 0x1e0, third);
    e.mem.set_u32(player + 0x1e4, first);
    e.mem.set_u32(PLAYER, player);
    // The actors: each with a process (or none), the process's +0x18, the tests, the kind, a node
    // (or none: the zero point) and a bound of the process.
    let n = 6 + k % 3;
    for i in 0..n {
        let actor = object(e);
        let bits = (i * 5 + k) % 32;
        let process = if bits % 7 == 6 { 0 } else { object(e) };
        answer(e, actor, 0x1d0, process);
        answer(e, actor, 0x214, if bits % 5 == 4 { 4 } else { 1 });
        e.mem.set_u32(actor + 0x2f0, u32::from(bits % 9 == 8));
        e.mem.set_u32(actor + 0x2f4, u32::from(bits % 6 != 5));
        if process != 0 {
            e.mem.set_u32(process + 0x18, u32::from(bits % 8 != 7));
            let node = if bits % 4 == 3 { 0 } else { e.mem.alloc(0x30) };
            if node != 0 {
                let f = [ONE, 0x4120_0000, HALF, NAN, 0xc0a0_0000][(bits % 5) as usize];
                e.mem.set_u32(node + 0x18, f);
                e.mem.set_u32(node + 0x1c, TWO);
                e.mem.set_u32(node + 0x20, HALF);
            }
            e.mem.set_u32(process + 0x2f8, node);
            let bound = e.mem.alloc(0x10);
            let radius = [HALF, 0x4180_0000, 0x4120_0000, NAN][(bits % 4) as usize];
            e.mem.set_u32(bound + 0xc, radius);
            e.mem.set_u32(process + 0x20, bound);
        }
        e.mem.set_u32(HIGH_PROCESS_LIST + 0x88 + 4 * i, actor);
    }
    e.mem.set_u32(HIGH_PROCESS_LIST + 0x150, n);
    // The camera root with its conversation target and sky.
    let root = object(e);
    let target = object(e);
    let sky = object(e);
    e.mem.set_u32(root + 0xb4, target);
    e.mem.set_u32(root + 0xac, sky);
    e.mem.set_u32(CAMERA_ROOT, root);
    // The saved object (or none) and its setting; the add-objects setting.
    let saved = if case.word(6) & 1 == 1 {
        flagged(e, case.word(6) & 2 != 0)
    } else {
        0
    };
    e.mem.set_u32(SAVED_OBJECT, saved);
    e.mem
        .set_u8(SETTING_KEEP_SAVED + 4, ((case.word(6) >> 2) & 1) as u8);
    e.mem.set_u8(SETTING_ADD_OBJECTS + 4, (k & 1) as u8);
    let l = lights(e, 3 + k % 5, k);
    scene_node(e, &l, 1, 0);
    e.mem.set_u32(0x011f_9ea4, 1);
    vec![object(e)]
}

#[test]
fn setup_prepare() {
    // The depth-pass answers (one bit a call); the request and remembered state; the shadows setting;
    // the world's interior word and the limit (0 to 3); the player's tests; the pattern of the
    // actors, lights and models; the saved object.
    Diff::new(entry!(0x0087_1290, shadows::prepare(Ptr<_>)))
        .common()
        .inputs(&[
            Input::Of(&[7, 0, 1, 2, 3, 5, 6]),
            Input::Of(&[0, 1, 2, 3]),
            Input::Flag,
            Input::Of(&[2, 0, 1, 3, 4, 5, 6, 7]),
            Input::Of(&[0, 1, 2, 3, 4, 5, 6, 7]),
            Input::Of(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]),
            Input::Of(&[0, 1, 3, 5, 7]),
        ])
        .max_cases(600)
        .setup(prepare_world)
        .run()
        .assert_same();
}

#[test]
fn setup_frame_target() {
    // The interface's offscreen flag; `skip_new_target`; the screen (none or one); the renderer's own
    // group; the target's sizes against the renderer's (the same, the width or the height apart, no
    // size object); floats in the ratio's scale.
    Diff::new(entry!(
        0x0087_2f50,
        frame::frame_target(Ptr<_>, u32, u32, u8, u8) -> u32
    ))
    .common()
    .inputs(&[
        Input::Flag,
        Input::Flag,
        Input::Flag,
        Input::Flag,
        Input::Of(&[0, 1, 2, 3]),
        Input::Of(&[0x3ff0_0000, 0x3fe0_0000, 0x4000_0000]),
    ])
    .setup(|e, case: &Case| {
        stand_ins(e);
        e.mem.set_u8(INTERFACE_OFFSCREEN, case.word(0) as u8);
        e.mem.set_u64(RATIO_SCALE, u64::from(case.word(5)) << 32);
        let renderer = object(e);
        answer(e, renderer, 0xc8, 0x55);
        // The target from the manager, its first `NiPointer` holding the object with the sizes.
        let target = object(e);
        let sizes = object(e);
        let (w, h) = match case.word(4) {
            0 => (640, 480),
            1 => (320, 480),
            _ => (640, 240),
        };
        answer(e, sizes, 0x94, w);
        answer(e, sizes, 0x98, h);
        if case.word(4) != 3 {
            e.mem.set_u32(target + 0x30, sizes);
        }
        set_logged(e, 0x1b, target);
        set_logged(e, 0x1d, 640);
        set_logged(e, 0x1e, 480);
        let root = object(e);
        let v = object(e);
        e.mem.set_u32(root + 0xac, v);
        e.mem.set_u32(CAMERA_ROOT, root);
        set_logged(e, 0x21, 0x66);
        let screen = if case.word(2) == 1 { object(e) } else { 0 };
        vec![object(e), renderer, screen, case.word(1), case.word(3)]
    })
    .run()
    .assert_same();
}

/// The world frame's world, from the case (on top of `stand_ins`): the player's first-person model
/// culled or not; the world's object +0x68 for the accumulator and its mode; the camera root and
/// the two `NiPointer` globals; the water reflections' setting and answers; the HUD's margins and
/// the Eyefinity bytes; the water group's level; the image-space bit 9 and its setting; the
/// interface's offscreen flag and the held object; the picked reference.
fn world_world(e: &mut Engine, case: &Case) -> Vec<u32> {
    stand_ins(e);
    let k = case.word(1);
    let main = object(e);
    e.mem.set_u8(main + 0x9c, (k & 1) as u8);
    // The player and its first-person model (culled from bit 1).
    let player = object(e);
    let model = flagged(e, k & 2 != 0);
    e.mem.set_u32(player + 0x1e4, model);
    let v = flagged(e, false);
    e.mem.set_u32(player + 0x1e0, v);
    e.mem.set_u32(PLAYER, player);
    // The world: +0x34 from bit 2, +0x68 an object whose +0x28 is the accumulator (from bit 3).
    let world = object(e);
    e.mem.set_u32(world + 0x34, (k >> 2) & 1);
    let highest = object(e);
    let accumulator = if k & 8 != 0 { flagged(e, false) } else { 0 };
    if accumulator != 0 {
        // The accumulator's owner through its `NiPointer` +0xc.
        let v = flagged(e, k & 0x10 != 0);
        e.mem.set_u32(accumulator + 0xc, v);
    }
    e.mem.set_u32(highest + 0x28, accumulator);
    e.mem.set_u32(world + 0x68, highest);
    e.mem.set_u32(WORLD, world);
    set_logged(e, 0x38, 2 + (k >> 5) % 2);
    // The camera root, its child, and the two `NiPointer` globals' objects.
    let root = object(e);
    let v = object(e);
    e.mem.set_u32(root + 0xac, v);
    let v = object(e);
    e.mem.set_u32(root + 0xb4, v);
    e.mem.set_u32(CAMERA_ROOT, root);
    for slot in [0x011d_eb34u32, 0x011d_eda4] {
        let o = object(e);
        e.mem.set_u32(slot, o);
    }
    let v = object(e);
    set_logged(e, 0x36, v);
    e.mem.set_u8(0x011d_ed80 + 4, ((k >> 6) & 1) as u8);
    // The water reflections.
    e.mem.set_u8(0x011c_7adc + 4, (case.word(2) & 1) as u8);
    set_logged(e, 0x39, (case.word(2) >> 1) & 1);
    set_logged(e, 0x3a, (case.word(2) >> 2) & 1);
    let water = e.mem.alloc(0x10);
    e.mem.set_u32(water, case.word(2) % 5 * 4);
    set_logged(e, 0x3d, water);
    let scene = object(e);
    e.mem.set_u32(SCENE_NODE_TABLE, scene);
    // The margins and the Eyefinity bytes.
    let hud = if case.word(3) & 1 != 0 {
        let h = e.mem.alloc(0x200);
        e.mem.set_u8(h + 0x1fc, ((case.word(3) >> 1) & 1) as u8);
        h
    } else {
        0
    };
    e.mem.set_u32(0x011d_96c0, hud);
    e.mem.set_u8(0x011c_70ed, ((case.word(3) >> 2) & 1) as u8);
    e.mem.set_u8(0x011c_70ee, 1);
    e.mem.set_u32(0x011d_ea90, 0x3faa_aaab);
    e.mem.set_u64(0x0101_1588, 0.75f64.to_bits());
    set_logged(e, 0x1d, 1280);
    set_logged(e, 0x1e, 720);
    // The water group, bit 9 and its setting, the refraction answers.
    let group = e.mem.alloc(0x80);
    e.mem.set_u32(
        group + 0x70,
        [0, ONE, NAN, 0xbf80_0000][(case.word(4) % 4) as usize],
    );
    set_logged(e, 0x49, group);
    set_logged(e, 0x4a, (case.word(4) >> 2) & 1);
    e.mem
        .set_u8(0x011d_eadc + 4, ((case.word(4) >> 3) & 1) as u8);
    set_logged(e, 0x4c, (case.word(4) >> 1) & 1);
    set_logged(e, 0x4d, case.word(4) & 1);
    let v = object(e);
    e.mem.set_u32(main + 0x88, v);
    // The interface: offscreen flag, the held object and its answer, the pipboy.
    e.mem.set_u8(INTERFACE_OFFSCREEN, (case.word(5) & 1) as u8);
    let held = if case.word(5) & 2 != 0 { object(e) } else { 0 };
    e.mem.set_u32(0x011c_7810, held);
    set_logged(e, 0x55, (case.word(5) >> 2) & 1);
    set_logged(e, 0x57, (case.word(5) >> 3) & 1);
    // The picked reference: none, or one whose +0x28 is a reference with a node.
    let picked = if case.word(6) != 0 {
        let p = object(e);
        let reference = if case.word(6) == 2 { object(e) } else { 0 };
        if reference != 0 {
            let node = object(e);
            words_at(e, node + 0x8c, &[ONE, TWO, HALF]);
            answer(e, reference, 0x4, node);
        }
        e.mem.set_u32(p + 0x28, reference);
        p
    } else {
        0
    };
    e.mem.set_u32(0x011d_ea20, picked);
    let renderer = object(e);
    e.mem.set_u32(RENDERER, renderer);
    let v = object(e);
    e.mem.set_u32(0x011f_91ac, v);
    // The offscreen target the texture manager hands out (its sizes object missing: size 0).
    let v = object(e);
    set_logged(e, 0x1b, v);
    set_logged(e, 0x26, case.word(1) >> 7);
    let target = if case.word(5) & 0x10 != 0 {
        object(e)
    } else {
        0
    };
    vec![main, target, case.word(0)]
}

fn words_at(e: &mut Engine, at: u32, w: &[u32]) {
    for (i, v) in (0u32..).zip(w) {
        e.mem.set_u32(at + 4 * i, *v);
    }
}

#[test]
fn setup_world_frame() {
    // `skip_world`; the pattern of the player, world, accumulator and camera; the water
    // reflections; the margins; the water group and refraction; the interface; the picked reference.
    Diff::new(entry!(0x0087_0bd0, world::world_frame(Ptr<_>, u32, u8)))
        .common()
        .ignore_calls(&[
            0x004e_3270,
            0x0040_3e20,
            0x0043_c490,
            0x0070_5100,
            0x0070_9cb0,
            0x004d_e080,
        ])
        .inputs(&[
            Input::Flag,
            Input::Of(&[0, 8, 0x18, 0x1f, 0x48, 0x2a, 0x7f, 0xc5]),
            Input::Of(&[0, 1, 3, 5, 7]),
            Input::Of(&[0, 1, 3, 7]),
            Input::Of(&[0, 1, 5, 0xe, 0xf, 6]),
            Input::Of(&[0, 1, 3, 7, 0xf, 0x10, 0x15]),
            Input::Of(&[0, 1, 2]),
        ])
        .max_cases(500)
        .setup(world_world)
        .run()
        .assert_same();
}

#[test]
fn setup_world_parts() {
    // The three set-up calls on their own, over the same world.
    let inputs = [
        Input::Flag,
        Input::Of(&[0, 8, 0x18, 0x1f, 0x48, 0x2a, 0x7f, 0xc5]),
        Input::Of(&[0, 1, 3, 5, 7]),
        Input::Of(&[0]),
        Input::Of(&[0]),
        Input::Of(&[0]),
        Input::Of(&[0]),
    ];
    let first_only = |e: &mut Engine, case: &Case| {
        let mut v = world_world(e, case);
        v.truncate(1);
        set_logged(e, 0x27, case.word(0));
        set_logged(e, 0x28, case.word(2) & 1);
        v
    };
    Diff::new(entry!(0x0087_2ad0, world::frame_begin(Ptr<_>)))
        .common()
        .inputs(&inputs)
        .setup(first_only)
        .run()
        .assert_same();
    Diff::new(entry!(0x0087_2b00, world::camera_setup(Ptr<_>) -> u32))
        .common()
        .ignore_calls(&[0x0043_c490])
        .inputs(&inputs)
        .setup(first_only)
        .run()
        .assert_same();
    Diff::new(entry!(0x0087_27d0, world::water_stages(Ptr<_>)))
        .common()
        .inputs(&inputs)
        .setup(first_only)
        .run()
        .assert_same();
}
