//! The differential test of the fade node's `OnVisible` (`engine_typed::culling::fade`) against its
//! translation. The world: a fade node with its distances, fades, radius, type and frame counter; a
//! culling process whose camera has a position and a LOD adjust; the fade statics (on, the
//! multipliers, times, thresholds, distance multiplier, most increment), the frame time and the fade
//! counter; the most-alpha callback (none, or one answering the case's alpha).
//!
//! What stands in: the alpha set down the subtree, the accumulator's kind test, the alpha grouping
//! and the callback. `NiNode::OnVisible` runs typed (the node has no bound, so it walks nothing).
//! The float getters and `_CIsqrt` are left out of the translation's calls: the typed code does their
//! work in place.

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::{Case, Diff, Input};
use engine_typed::culling::{self, fade};

const LOG: u32 = 0x0300_0000;
const CALLBACK: u32 = 0x00ff_6000;

const ONE: u32 = 0x3f80_0000;
const HALF: u32 = 0x3f00_0000;
const TWO: u32 = 0x4000_0000;

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

fn logged<const TAG: u32>(e: &mut Engine, a: &[u32]) -> Ret {
    log(e, TAG, a.first().copied().unwrap_or(0));
    for w in a.iter().skip(1) {
        log(e, TAG | 0x1000_0000, *w);
    }
    ret(e.mem.u32(LOG + 0x3c00 + 4 * (TAG & 0xff)))
}

fn words(e: &mut Engine, at: u32, w: &[u32]) {
    for (i, v) in (0u32..).zip(w) {
        e.mem.set_u32(at + 4 * i, *v);
    }
}

fn bits(case: &Case, i: usize) -> u32 {
    case.f32(i).to_bits()
}

/// The multipliers per type: positive ones, a zero (type 3) and a negative one (type 7).
const MULTIPLIERS: [u32; 11] = [
    ONE,
    HALF,
    TWO,
    0,
    0x4040_0000,
    ONE,
    ONE,
    0xbf80_0000,
    ONE,
    ONE,
    ONE,
];

#[test]
fn culling_fade_on_visible() {
    // Fading on; the flags (0x8000, 0x4000, 0x10000000 in turn); the type; the frame counter against
    // the node's (snap or not); the radius (under or over 600); the most alpha (no callback, 0, a half,
    // 1) and the accumulator's answer; floats for the camera's distance, the current fade and the
    // frame time.
    Diff::new(entry!(0x00b4_e490, fade::on_visible(Ptr<_>, Ptr<_>)))
        .family(&culling::CULLING)
        .family(&leaf::LEAVES)
        // `_CIsqrt` and the two float getters: the typed function does them in place.
        .ignore_calls(&[0x00ec_6040, 0x0040_ebd0, 0x0040_4010])
        .inputs(&[
            Input::Flag,
            Input::Of(&[0, 0x8000, 0x4000, 0x1000_0000, 0x1000_4000]),
            Input::Of(&[0, 1, 3, 6, 7, 8, 10]),
            Input::Flag,
            Input::Flag,
            Input::Of(&[0, 1, 2, 3, 4]),
            Input::F32,
            Input::F32Swept(HALF),
            Input::F32Swept(0x3c88_8889),
        ])
        .max_cases(600)
        .float_sweep(64)
        .setup(|e, case: &Case| {
            e.map(LOG, 0x4000);
            for page in [
                0x011a_d000,
                0x011f_6000,
                0x011f_9000,
                0x011f_4000,
                0x011f_a000,
                0x0101_d000,
                0x0101_7000,
            ] {
                e.map(page, 0x1000);
            }
            for (addr, f) in [
                (
                    0x00b6_bb30u32,
                    logged::<0x01> as fn(&mut Engine, &[u32]) -> Ret,
                ),
                (0x0043_b300, logged::<0x02>),
                (0x00b6_3500, logged::<0x03>),
                (0x00b6_3090, logged::<0x04>),
            ] {
                e.register(addr, f);
            }
            // `_CIsqrt`: the root of the double it is handed, in ST0 (the C runtime's own error
            // path on a NaN reaches statics this test does not lay out).
            e.register(0x00ec_6040, |_, a| Ret {
                st0: f64::from_bits(u64::from(a[0]) | (u64::from(a[1]) << 32)).sqrt(),
                ..Ret::default()
            });
            // The statics.
            e.mem.set_u8(0x011a_d7b4, case.word(0) as u8);
            words(e, 0x011a_d7b8, &MULTIPLIERS);
            words(
                e,
                0x011a_d7e4,
                &[ONE, TWO, 0x3e80_0000, 0x3f40_0000, ONE, 0x3e00_0000],
            );
            e.mem.set_u32(0x011f_63a0, bits(case, 8));
            e.mem.set_u32(0x011f_9138, 100);
            e.mem.set_u32(0x0101_df44, 0x3e4c_cccd);
            e.mem.set_u32(0x0101_786c, 0x4416_0000);
            // The most-alpha callback: none, or one answering 0, a half or 1 (in ST0).
            fn most_alpha<const ALPHA: u32>(e: &mut Engine, a: &[u32]) -> Ret {
                log(e, 0x05, a[0]);
                Ret {
                    st0: f64::from(f32::from_bits(ALPHA)),
                    ..Ret::default()
                }
            }
            let callback: Option<fn(&mut Engine, &[u32]) -> Ret> = match case.word(5) {
                1 => Some(most_alpha::<0>),
                2 => Some(most_alpha::<HALF>),
                3 => Some(most_alpha::<ONE>),
                4 => Some(most_alpha::<0x3f7f_0000>),
                _ => None,
            };
            if let Some(callback) = callback {
                e.register(CALLBACK, callback);
                e.mem.set_u32(0x011f_9134, CALLBACK);
            }
            e.mem.set_u32(LOG + 0x3c00 + 8, case.word(5) & 1);
            // The accumulator holder.
            let holder = e.mem.alloc(0x10);
            let accumulator = e.mem.alloc(0x40);
            e.mem.set_u32(holder + 8, accumulator);
            e.mem.set_u32(0x011f_4748, holder);
            // The camera and the process.
            let camera = e.mem.alloc(0x120);
            words(e, camera + 0x8c, &[bits(case, 6), ONE, 0]);
            e.mem.set_u32(camera + 0x110, ONE);
            let process = e.mem.alloc(0xc8);
            e.mem.set_u32(process + 0xc, camera);
            // The node (no bound: its children are not walked).
            let node = e.mem.alloc(0xe4);
            let vtable = e.mem.alloc(0x100);
            e.mem.set_u32(vtable + 0xd4, 0x00b4_e490);
            e.mem.set_u32(node, vtable);
            e.mem.set_u32(node + 0x30, case.word(1));
            words(e, node + 0x8c, &[0, 0, 0]);
            words(e, node + 0xac, &[0x4120_0000, 0x42c8_0000]);
            e.mem.set_u32(node + 0xb8, bits(case, 7));
            e.mem.set_u32(
                node + 0xbc,
                if case.word(4) == 1 {
                    0x4120_0000
                } else {
                    0x4500_0000
                },
            );
            e.mem.set_u32(node + 0xc4, case.word(2));
            e.mem
                .set_u32(node + 0xc8, if case.word(3) == 1 { 50 } else { 99 });
            vec![node, process]
        })
        .run()
        .assert_same();
}
