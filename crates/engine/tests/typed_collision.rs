//! The differential test of every typed function of the collision family (`engine_typed::collision`)
//! against its translation. The world: a node with a world transform from the case's floats, a blend
//! collision object or a collision object with its Havok body (an object with a vtable whose slots log
//! and answer from the words after them: the body's position and rotation slots write the case's
//! floats), the body's Havok object with its motion type and flags, the thread's update state.
//!
//! What stands in: the Havok calls (motion type, hard keyframe, the velocities' set-up, the Havok world,
//! the object and motion type tests, the blend's quaternion and transform helpers, the body's update
//! from the blend), `bhkCollisionObject::hktoNiTransform`, the motion notification, the vector finish
//! and the shared zero vector, the square root. Everything else runs translated.

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::{Case, Diff, Input};
use engine_core::typed::{MAIN_THREAD, OTHER_THREADS};
use engine_typed::collision::{self, blend, body};
use engine_typed::{culling, dispatch, pathing};

const VT_FN: u32 = 0x00ff_2000;
const VT_SLOTS: u32 = 0x200;
const LOG: u32 = 0x0300_0000;
const MAX_CALLS: usize = 5_000;
const FLOAT_SWEEP: usize = 32;

const ONE: u32 = 0x3f80_0000;
const TWO: u32 = 0x4000_0000;
const HALF: u32 = 0x3f00_0000;
const MINUS_ONE: u32 = 0xbf80_0000;
const THIRD: u32 = 0x3eaa_aaab;
const NAN: u32 = 0x7fc0_0000;

/// The real functions a test can put in a slot.
const BLEND_HK_TO_NI: u32 = 0x00c8_1e30;
const BLEND_UPDATE_WORLD_DATA: u32 = 0x00c8_17d0;
/// The Havok lock, the shared zero vector, the update state's offset in the thread block.
const LOCK: u32 = 0x0126_7b80;
const ZERO_VECTOR: u32 = 0x0126_7e30;
const TLS_UPDATE_STATE: u32 = 0x2c0;

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

fn log(e: &mut Engine, tag: u32, word: u32) {
    let n = e.mem.u32(LOG);
    if n >= 0x7f0 {
        return;
    }
    e.mem.set_u32(LOG + 4 + 8 * n, tag);
    e.mem.set_u32(LOG + 8 + 8 * n, word);
    e.mem.set_u32(LOG, n + 1);
}

fn bits(case: &Case, i: usize) -> u32 {
    case.f32(i).to_bits()
}

fn words(e: &mut Engine, at: u32, w: &[u32]) {
    for (i, v) in (0u32..).zip(w) {
        e.mem.set_u32(at + 4 * i, *v);
    }
}

fn block(e: &mut Engine, size: u32, w: &[u32]) -> u32 {
    let a = e.mem.alloc(size.max(4 * w.len() as u32));
    words(e, a, w);
    a
}

fn f64_arg(a: &[u32], i: usize) -> f64 {
    f64::from_bits(u64::from(a[i]) | u64::from(a[i + 1]) << 32)
}

/// A stand-in that logs its words under `TAG` and answers the word at `LOG + 0x3f00 + TAG & 0xff`
/// (so a test can set what a predicate answers).
fn logged<const TAG: u32>(e: &mut Engine, a: &[u32]) -> Ret {
    log(e, TAG, a.first().copied().unwrap_or(0));
    for w in a.iter().skip(1) {
        log(e, TAG | 0x1_0000, *w);
    }
    ret(e.mem.u32(LOG + 0x3f00 + 4 * (TAG & 0x3f)))
}

/// The answer a `logged::<TAG>` stand-in gives.
fn set_logged_answer(e: &mut Engine, tag: u32, answer: u32) {
    e.mem.set_u32(LOG + 0x3f00 + 4 * (tag & 0x3f), answer);
}

/// The stand-in for any slot: logs `this` and its words; slots that are given an out pointer write
/// the four words after the vtable's slots there; answers the vtable's answer word.
fn any_slot(e: &mut Engine, a: &[u32]) -> Ret {
    let this = a.first().copied().unwrap_or(0);
    log(e, 0xffff, this);
    for w in a.iter().skip(1) {
        log(e, 0xfffe, *w);
    }
    let vt = e.mem.u32(this);
    if a.len() == 2 && a[1] >= 0x0100_0000 {
        // An out vector: a heap or stack address.
        for k in 0..4 {
            let w = e.mem.u32(vt + VT_SLOTS + 4 + 4 * k);
            e.mem.set_u32(a[1] + 4 * k, w);
        }
    }
    let answer = e.mem.u32(vt + VT_SLOTS);
    Ret {
        eax: answer,
        st0: f64::from(f32::from_bits(answer)),
        ..Ret::default()
    }
}

fn vtable(e: &mut Engine) -> u32 {
    let vt = e.mem.alloc(VT_SLOTS + 0x40);
    for slot in (0..VT_SLOTS).step_by(4) {
        e.mem.set_u32(vt + slot, VT_FN + slot);
    }
    vt
}

fn object(e: &mut Engine, size: u32) -> u32 {
    let o = e.mem.alloc(size);
    let vt = vtable(e);
    e.mem.set_u32(o, vt);
    o
}

fn set_slot(e: &mut Engine, object: u32, slot: u32, target: u32) {
    let vt = e.mem.u32(object);
    e.mem.set_u32(vt + slot, target);
}

/// What `object`'s stand-in slots answer, and the vector they write to an out pointer.
fn set_answers(e: &mut Engine, object: u32, answer: u32, vector: [u32; 4]) {
    let vt = e.mem.u32(object);
    e.mem.set_u32(vt + VT_SLOTS, answer);
    words(e, vt + VT_SLOTS + 4, &vector);
}

/// The exe's data, the stand-ins and the thread's update state.
fn stand_ins(e: &mut Engine, state: u32) {
    e.map(LOG, 0x4000);
    for page in [
        0x0101_1000,
        0x0101_2000,
        0x0101_7000,
        0x010e_c000,
        0x011a_9000,
        0x011a_f000,
        0x011c_5000,
        0x011f_4000,
        0x0126_7000,
    ] {
        e.map(page, 0x1000);
    }
    e.mem.set_u64(0x0101_1588, 0.5f64.to_bits());
    e.mem.set_u64(0x0101_2060, 0);
    e.mem.set_u64(0x0101_2070, 1.0f64.to_bits());
    // The velocity epsilon, the absolute-value mask, the length minimum.
    e.mem.set_u32(0x0101_7d00, 0x3a83_126f);
    words(e, 0x0101_7d10, &[0x7fff_ffff; 4]);
    e.mem.set_u64(0x0101_7cf8, 0.001f64.to_bits());
    words(e, 0x010e_c7f0, &[ONE; 4]);
    words(e, 0x011a_9e98, &[1, 2, 0]);
    e.mem.set_u32(0x011a_fe64, TWO);
    words(e, 0x011f_426c, &[0, 0, 0]);
    e.mem.set_u32(0x0126_7e6c, HALF);
    // The Havok scale the position conversion multiplies by.
    e.mem.set_u32(0x011c_582c, 0x3e12_4925);
    words(e, ZERO_VECTOR, &[0, 0, 0, 0]);
    words(e, LOCK, &[0, 0]);
    let tls = e.tls();
    e.mem.set_u32(tls + TLS_UPDATE_STATE, state);
    e.register(0x00ec_6040, |_, a| ret_f(f64_arg(a, 0).sqrt()));
    e.register(0x00ec_6054, |_, a| ret_f(f64_arg(a, 0).sqrt()));
    // The imports the lock and the thread id reach.
    e.register(0x00fd_f1bc, |e, a| {
        let old = e.mem.u32(a[0]);
        if old == a[2] {
            e.mem.set_u32(a[0], a[1]);
        }
        ret(old)
    });
    e.register(0x00fd_f1d0, |_, _| ret(0x1234));
    e.register(0x00c8_7a40, logged::<0x01>);
    e.register(0x0047_b470, logged::<0x02>);
    e.register(0x00c8_cce0, logged::<0x03>);
    e.register(0x00c8_e160, logged::<0x04>);
    e.register(0x0056_11c0, logged::<0x05>);
    e.register(0x0056_1690, logged::<0x06>);
    e.register(0x0056_17c0, logged::<0x07>);
    e.register(0x00c6_5f40, logged::<0x08>);
    e.register(0x00c6_5a80, logged::<0x09>);
    e.register(0x00c8_1aa0, logged::<0x0a>);
    e.register(0x0056_dff0, logged::<0x0b>);
    e.register(0x00cb_26e0, logged::<0x0c>);
    e.register(0x00cb_2850, logged::<0x0d>);
    e.register(0x00cb_2d90, logged::<0x0e>);
    e.register(0x0062_4b10, logged::<0x0f>);
    e.register(0x0045_8620, logged::<0x10>);
    e.register(0x00c8_de20, logged::<0x11>);
    // The vector finish answers its word plus 0x10; the shared zero vector.
    e.register(0x0045_c650, |e, a| {
        log(e, 0xc650, a[0]);
        ret(a[0] + 0x10)
    });
    e.register(0x0045_8b20, |_, _| ret(ZERO_VECTOR));
    for s in (0..VT_SLOTS).step_by(4) {
        e.register(VT_FN + s, any_slot);
    }
}

/// A node (0xb0 bytes, no parent) with flags and a world transform and local transform from the
/// case's floats at `f` (scale) and `f + 1` (the first rotation entry).
fn node(e: &mut Engine, case: &Case, flags: u32, f: usize) -> u32 {
    let n = object(e, 0xb0);
    e.mem.set_u32(n + 0x30, flags);
    let table = [
        ONE, HALF, 0, THIRD, ONE, MINUS_ONE, 0, TWO, ONE, TWO, MINUS_ONE, HALF,
    ];
    for at in [n + 0x34, n + 0x68] {
        words(e, at + 4, &table[1..9]);
        words(e, at + 0x24, &table[9..12]);
        e.mem.set_u32(at, bits(case, f + 1));
        e.mem.set_u32(at + 0x30, bits(case, f));
    }
    n
}

/// A Havok body (an object whose slots +0xd4/+0xd8 write `position`) whose Havok object (+8, also
/// what the velocity getters take as its entity: the linear velocity at +0xf0, the angular at +0x1c0)
/// has the motion type `motion` (5: no Havok object) and flags.
fn havok_body(e: &mut Engine, motion: u32, flags: u32, position: [u32; 4]) -> u32 {
    let b = object(e, 0x100);
    set_answers(e, b, 0, position);
    if motion != 5 {
        let h = block(e, 0x200, &[0; 128]);
        e.mem.set_u8(h + 0xe8, motion as u8);
        e.mem.set_u32(h + 0x10, flags);
        e.mem.set_u32(b + 8, h);
    }
    b
}

const P3: [Input; 3] = [Input::F32, Input::F32Swept(ONE), Input::F32Swept(MINUS_ONE)];

fn inputs(parts: &[&[Input]]) -> Vec<Input> {
    parts.iter().flat_map(|p| p.iter().copied()).collect()
}

trait Common {
    fn common(self) -> Diff;
    /// One of [`BY_THREAD`]'s settings, kept in the comparison's own chain.
    fn by(self, run: fn(Diff) -> Diff) -> Diff;
}

/// The quaternion's comparisons run twice, split by thread (ADR-0014): under the other threads' control
/// word bit for bit (NaN payloads aside: the family copies floats plain where the exe moved them
/// through the x87), and under the main thread's within one ulp (plain `f32` rounds each step to 24
/// bits as the exe's code does there; the x87's wider exponent can part in the last bit). Two
/// main-thread differences outside float range are accepted by ADR-0014's tolerance and left
/// untested: a trace sum that overflows (`f32` gives infinity where the x87's 15-bit exponent stays
/// finite until the store, e.g. the diagonal (2^127, 2^127, -2^127)), and a denormal result (one
/// rounding in `f32` against the x87's two, at 24 bits and again at the store).
const BY_THREAD: [fn(Diff) -> Diff; 2] = [other_threads, main_thread];

/// [`BY_THREAD`]'s run under the other threads' control word.
fn other_threads(d: Diff) -> Diff {
    d.control_words(&[OTHER_THREADS]).allow_nan_payloads()
}

/// [`BY_THREAD`]'s run under the main thread's control word.
fn main_thread(d: Diff) -> Diff {
    d.control_words(&[MAIN_THREAD]).float_ulps(1)
}

/// The fixed rounding check runs under both control words bit for bit, without the float sweep: its
/// input is one where rounding every step to a float (the main thread) and rounding once (the other
/// threads) part, so maths in the wrong precision for the thread fails.
const ROUNDING_BY_THREAD: [fn(Diff) -> Diff; 2] = [other_threads_fixed, main_thread_exact];

/// [`ROUNDING_BY_THREAD`]'s run under the other threads' control word.
fn other_threads_fixed(d: Diff) -> Diff {
    other_threads(d).float_sweep(0)
}

/// [`ROUNDING_BY_THREAD`]'s run under the main thread's control word, bit for bit.
fn main_thread_exact(d: Diff) -> Diff {
    d.control_words(&[MAIN_THREAD]).float_sweep(0)
}

impl Common for Diff {
    fn by(self, run: fn(Diff) -> Diff) -> Diff {
        run(self)
    }

    fn common(self) -> Diff {
        self.family(&collision::COLLISION)
            .family(&dispatch::DISPATCH)
            .family(&pathing::PATHING)
            .family(&culling::CULLING)
            .family(&leaf::LEAVES)
            .goes_by_address(&[0x00ec_6040])
            // The native quaternion takes its square roots with `FSQRT`, not the CRT's `_CIsqrt`.
            .ignore_calls(&[0x00ec_6040])
            // What the dispatch family's translations call plainly (its PLAIN list): the Havok position's
            // helper and the body vector's part.
            .translation_calls_plainly(&[0x0040_19d0, 0x0040_8860, 0x004a_3e90, 0x0056_0e50])
            .max_calls(MAX_CALLS)
            .float_sweep(FLOAT_SWEEP)
            // The family copies plain the floats the exe only moves through the x87 (the body's
            // quaternion words, the blend amounts, the scale and the reciprocal's source), where the
            // x87 quiets a signalling NaN: payloads only, which the game can't see (ADR-0014).
            .allow_nan_payloads()
    }
}

#[test]
fn collision_quaternion_from_rotation() {
    // The diagonal: positive trace, each element the largest, NaN, two elements tied for the largest (a
    // 180-degree turn about (1, 1, 0) has the diagonal (0, 0, -1)), where the exe's strict "below"
    // picks the index; infinities of both signs whose trace is NaN (the other branch, which the exe's
    // test sends a NaN trace to, then gives numbers: an infinite root, a zero factor), one row for each
    // largest index; the rest from the case.
    const INF: u32 = 0x7f80_0000;
    const NEG_INF: u32 = 0xff80_0000;
    for base in BY_THREAD {
        Diff::new(entry!(
            0x00a6_df40,
            body::quaternion_from_rotation(Ptr<_>, Ptr<_>)
        ))
        .common()
        .by(base)
        .inputs(&inputs(&[
            &[Input::Of(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10])],
            &P3,
            &[Input::F32Swept(HALF), Input::F32Swept(THIRD)],
        ]))
        .max_cases(256)
        .setup(|e, case| {
            stand_ins(e, 0);
            let diagonals: [[u32; 3]; 11] = [
                [ONE, ONE, ONE],
                [TWO, MINUS_ONE, MINUS_ONE],
                [MINUS_ONE, TWO, MINUS_ONE],
                [MINUS_ONE, MINUS_ONE, TWO],
                [NAN, ONE, MINUS_ONE],
                [0, 0, MINUS_ONE],
                [MINUS_ONE, 0, 0],
                [0, MINUS_ONE, 0],
                [INF, NEG_INF, 0],
                [0, INF, NEG_INF],
                [NEG_INF, 0, INF],
            ];
            let d = diagonals[case.word(0) as usize];
            let m = block(
                e,
                0x24,
                &[
                    d[0],
                    bits(case, 1),
                    bits(case, 2),
                    bits(case, 3),
                    d[1],
                    bits(case, 4),
                    bits(case, 5),
                    HALF,
                    d[2],
                ],
            );
            let q = e.mem.alloc(0x10);
            vec![q, m]
        })
        .run()
        .assert_same();
    }
}

#[test]
fn collision_quaternion_rounding() {
    // The diagonal (0x3f1f4b68, 0x3f3e2695, 0x3f76f2d6): its trace rounded each step and rounded once
    // part, and so does w (0x3f6990f7 against 0x3f6990f8), so the quaternion computed in the wrong
    // precision for the thread fails. And the diagonal (0.75, 3 * 2^-25, -2^-50): the trace's double
    // sum is 0.75 + 3 * 2^-25 - 2^-50, which the exe's store rounds to 0.75 + 2^-24 before adding 1.0
    // (giving 1.75 at the tie); added unrounded it gives 1.75 + 2^-23, so on the other threads a trace
    // not rounded to a float at its store fails.
    //
    // Then the other branch (trace <= 0), one row for each largest index i (j and k the exe's next
    // indices: i = 0 reads (d_i, d_j, d_k) at diagonal places (0, 1, 2), i = 1 at (1, 2, 0), i = 2 at
    // (2, 0, 1)), where the radicand d_i - d_j - d_k + 1 rounded at every step (the main thread) and
    // rounded once at its store (the other threads) give different floats with different roots:
    // - i = 0, (0x3f0452ba, 0xa5b5baea, 0xbf3a73a0): once 0x400fb197, each step 0x400fb196; also
    //   d_i - d_j rounded early (0x400fb196) and the sum taken as (d_i + 1) - d_j - d_k (0x400fb196).
    // - i = 1, (0x3f9c1880, 0xa63d9423, 0xc000b40d): once 0x40876027, each step, early and that
    //   order all 0x40876026.
    // - i = 2, (0x3fb67004, 0xaed83069, 0xc03d17d3): once 0x40ac27eb, each step and early 0x40ac27ea.
    for base in ROUNDING_BY_THREAD {
        Diff::new(entry!(
            0x00a6_df40,
            body::quaternion_from_rotation(Ptr<_>, Ptr<_>)
        ))
        .common()
        .by(base)
        .inputs(&[Input::Of(&[0, 1, 2, 3, 4])])
        .setup(|e, case| {
            stand_ins(e, 0);
            let d = [
                [0x3f1f_4b68, 0x3f3e_2695, 0x3f76_f2d6],
                [0x3f40_0000, 0x33c0_0000, 0xa680_0000],
                [0x3f04_52ba, 0xa5b5_baea, 0xbf3a_73a0],
                [0xc000_b40d, 0x3f9c_1880, 0xa63d_9423],
                [0xaed8_3069, 0xc03d_17d3, 0x3fb6_7004],
            ][case.word(0) as usize];
            let m = block(e, 0x24, &[d[0], 0, 0, 0, d[1], 0, 0, 0, d[2]]);
            let q = e.mem.alloc(0x10);
            vec![q, m]
        })
        .run()
        .assert_same();
    }
}

#[test]
fn collision_linear_velocity() {
    // No controller, a controller without a proxy, a proxy.
    Diff::new(entry!(0x0056_0d80, body::linear_velocity(Ptr<_>) -> Ptr<_>))
        .common()
        .inputs(&[Input::Of(&[0, 1, 2])])
        .setup(|e, case| {
            stand_ins(e, 0);
            let r = block(e, 0x40, &[0; 16]);
            if case.word(0) != 0 {
                let proxy = if case.word(0) == 2 {
                    e.mem.alloc(0x200)
                } else {
                    0
                };
                let controller = block(e, 0x10, &[0, 0, proxy]);
                e.mem.set_u32(r + 8, controller);
            }
            vec![r]
        })
        .run()
        .assert_same();
}

#[test]
fn collision_linear_velocity_of() {
    Diff::new(entry!(
        0x0056_0dc0,
        body::linear_velocity_of(Ptr<_>) -> Ptr<_>
    ))
    .common()
    .inputs(&[Input::Of(&[0x0300_2000, 0x0300_3000])])
    .setup(|e, case| {
        stand_ins(e, 0);
        vec![case.word(0)]
    })
    .run()
    .assert_same();
}

/// The world of one `NitohkTransform` case, from `w` = [the parts (0: neither, 1: a node, 2: a node
/// and a body), the body's motion type (5: no Havok object), the Havok world set, the object's flags
/// word, the node's flags, the body near, the velocities (bit 0: the linear one within the epsilon,
/// bit 1: the angular one), the tests that answer true (1: the object's, 2: the motion type's)], the
/// node's floats from the case's float `f`.
fn ni_to_hk_world(e: &mut Engine, case: &Case, w: [u32; 8], f: usize) -> Vec<u32> {
    let [parts, motion, world, object_flags, node_flags, near, velocities, tests] = w;
    stand_ins(e, 0);
    set_logged_answer(e, 0x01, world);
    set_logged_answer(e, 0x02, u32::from(tests == 1));
    set_logged_answer(e, 0x03, u32::from(tests == 2));
    let c = block(e, 0x14, &[0; 5]);
    e.mem.set_u16(c + 0xc, object_flags as u16);
    if parts >= 1 {
        let n = node(e, case, node_flags, f);
        e.mem.set_u32(c + 8, n);
    }
    if parts == 2 {
        // The body's position is compared with the node's translation (2, -1, 0.5) in Havok's units
        // (a seventh of it): the origin is within 1.0 of that (near), (1, 2, 3) is not (far).
        let position = if near != 0 {
            [0, 0, 0, 0]
        } else {
            [ONE, TWO, 0x4040_0000, 0]
        };
        let b = havok_body(e, motion, 0, position);
        // The linear (+0xf0) and angular (+0x1c0) velocities: zero (within the epsilon) or (1, 0, 0).
        let h = e.mem.u32(b + 8);
        if h != 0 {
            for (bit, at) in [(1, 0xf0), (2, 0x1c0)] {
                let v = if velocities & bit != 0 {
                    [0, 0, 0, 0]
                } else {
                    [ONE, 0, 0, 0]
                };
                words(e, h + at, &v);
            }
        }
        e.mem.set_u32(c + 0x10, b);
    }
    vec![c]
}

#[test]
fn collision_ni_to_hk() {
    // A node or none; a body or none; its motion type (5: none); the Havok world set; the object's
    // move bit; the node's flags; the body far or near; the velocities within the epsilon; the
    // object and motion type tests; floats.
    Diff::new(entry!(0x00c6_5b00, body::ni_to_hk(Ptr<_>)))
        .common()
        .inputs(&inputs(&[
            &[
                Input::Of(&[0, 1, 2]),
                Input::Of(&[1, 5]),
                Input::Flag,
                Input::Of(&[0, 0x40]),
                Input::Of(&[0, 0x8, 0x10, 0x0040_0000, 0x0800_0000]),
                Input::Flag,
                Input::Of(&[0, 1, 2, 3]),
                Input::Of(&[0, 1, 2]),
            ],
            &P3,
        ]))
        .max_cases(256)
        .setup(|e, case| {
            let w = core::array::from_fn(|i| case.word(i));
            ni_to_hk_world(e, case, w, 8)
        })
        .run()
        .assert_same();
    // The path a body with a Havok object, no move bit and no node flags takes: near or far, each
    // pair of velocities, each test; floats.
    Diff::new(entry!(0x00c6_5b00, body::ni_to_hk(Ptr<_>)))
        .common()
        .inputs(&inputs(&[
            &[
                Input::Flag,
                Input::Flag,
                Input::Of(&[0, 1, 2, 3]),
                Input::Of(&[0, 1, 2]),
            ],
            &P3,
        ]))
        .max_cases(256)
        .setup(|e, case| {
            let w = [
                2,
                1,
                case.word(0),
                0,
                0,
                case.word(1),
                case.word(2),
                case.word(3),
            ];
            ni_to_hk_world(e, case, w, 4)
        })
        .run()
        .assert_same();
}

#[test]
fn collision_hk_to_ni() {
    // The blend amount (0, 1, a half, NaN); the second amount zero or not; the position flag; the update
    // state; the motion mode it last set (the target's, 4, another); a Havok object or none; an owner
    // (whose word +0x18 is 0 or not) or none; floats.
    Diff::new(entry!(0x00c8_1e30, blend::hk_to_ni(Ptr<_>)))
        .common()
        .inputs(&inputs(&[
            &[
                Input::Of(&[0, ONE, HALF, NAN]),
                Input::Of(&[0, HALF]),
                Input::Flag,
                Input::Of(&[0, 1, 2]),
                Input::Of(&[1, 4, 2]),
                Input::Flag,
                Input::Of(&[0, 1, 2]),
            ],
            &P3,
        ]))
        .max_cases(256)
        // Blend mode 0 reads the Havok object's position through its vtable: without one both sides stop there
        // (the game never has a part-keyframed object without one).
        .expect_stops(|case| {
            let (amount, second, flag, state) =
                (case.word(0), case.word(1), case.word(2), case.word(3));
            let mode_0 = (amount == 0 && state != 1 && second != 0 && flag == 0)
                || ((amount == HALF || amount == NAN) && state != 2);
            case.word(5) == 0 && mode_0
        })
        .setup(|e, case| {
            stand_ins(e, case.word(3));
            let c = object(e, 0x28);
            let n = node(e, case, 0, 7);
            e.mem.set_u32(c + 8, n);
            e.mem.set_u8(c + 0xd, case.word(2) as u8);
            if case.word(5) != 0 {
                let h = havok_body(e, 1, 0, [bits(case, 7), bits(case, 8), ONE, 0]);
                set_answers(e, h, 0, [bits(case, 7), bits(case, 8), ONE, HALF]);
                e.mem.set_u32(c + 0x10, h);
            }
            words(e, c + 0x18, &[case.word(0), case.word(1), case.word(4)]);
            if case.word(6) != 0 {
                let owner = block(e, 0x20, &[0; 8]);
                e.mem.set_u32(owner + 0x18, case.word(6) - 1);
                e.mem.set_u32(c + 0x24, owner);
            }
            vec![c]
        })
        .run()
        .assert_same();
}

#[test]
fn collision_hk_to_ni_reciprocal_source() {
    // Blend mode 0 (a half amount outside update state 2, a Havok object) with the reciprocal's source
    // zero (the 1.0 the exe passes then) or not, and with the position flag set or not (the second
    // amount passed or 1.0); floats.
    Diff::new(entry!(0x00c8_1e30, blend::hk_to_ni(Ptr<_>)))
        .common()
        .inputs(&inputs(&[&[Input::Of(&[0, TWO, THIRD]), Input::Flag], &P3]))
        .setup(|e, case| {
            stand_ins(e, 0);
            e.mem.set_u32(0x011a_fe64, case.word(0));
            let c = object(e, 0x28);
            let n = node(e, case, 0, 3);
            e.mem.set_u32(c + 8, n);
            e.mem.set_u8(c + 0xd, case.word(1) as u8);
            let h = havok_body(e, 1, 0, [bits(case, 3), bits(case, 4), ONE, 0]);
            set_answers(e, h, 0, [bits(case, 3), bits(case, 4), ONE, HALF]);
            e.mem.set_u32(c + 0x10, h);
            words(e, c + 0x18, &[HALF, THIRD, 1]);
            vec![c]
        })
        .run()
        .assert_same();
}

#[test]
fn collision_update_world_data() {
    // The update state; the set byte; the Havok object: none, without or with the flag bit 6; the flags
    // byte (move, keyframed); the blend amount; the motion mode; a node or none; the slot +0xa4 the
    // typed hktoNiTransform or a stand-in.
    Diff::new(entry!(0x00c8_17d0, blend::update_world_data(Ptr<_>, u32)))
        .common()
        .inputs(&inputs(&[
            &[
                Input::Of(&[0, 2, 3]),
                Input::Flag,
                Input::Of(&[0, 1, 2]),
                Input::Of(&[0, 1, 0x40, 0x41]),
                Input::Of(&[ONE, HALF, NAN]),
                Input::Of(&[0, 1]),
                Input::Flag,
                Input::Flag,
            ],
            &P3,
        ]))
        .max_cases(256)
        // The typed `hktoNiTransform` in the slot blends the body into the node for an amount that is
        // neither 0 nor 1 outside update state 2, which reads the node and the Havok object: without
        // either both sides stop there (the game never has a part-keyframed object without them).
        .expect_stops(|case| {
            let w = |i| case.word(i);
            let (state, havok, amount) = (w(0), w(2), w(4));
            let runs = state != 3
                && w(1) == 0
                && havok != 2
                && w(3) == 1
                && (havok != 0 || amount == HALF || w(5) == 1)
                && w(7) != 0;
            let blends = (amount == HALF || amount == NAN) && state != 2;
            runs && blends && (havok == 0 || w(6) == 0)
        })
        .setup(|e, case| {
            stand_ins(e, case.word(0));
            let c = object(e, 0x28);
            if case.word(6) != 0 {
                let n = node(e, case, 0, 8);
                e.mem.set_u32(c + 8, n);
            }
            e.mem.set_u16(c + 0xc, case.word(3) as u16);
            e.mem.set_u8(c + 0x14, case.word(1) as u8);
            if case.word(2) != 0 {
                // The Havok object (+0x10): its word +0x10 is what the flag test reads.
                let h = havok_body(e, 1, 0, [ONE, TWO, HALF, 0]);
                e.mem
                    .set_u32(h + 0x10, if case.word(2) == 2 { 0x40 } else { 0 });
                e.mem.set_u32(c + 0x10, h);
            }
            words(e, c + 0x18, &[case.word(4), 0, case.word(5)]);
            if case.word(7) != 0 {
                set_slot(e, c, 0xa4, BLEND_HK_TO_NI);
            }
            vec![c, 0x55]
        })
        .run()
        .assert_same();
}

/// The culling family's `NiAVObject::UpdateWorldData` with a collision object whose slot +0x90 holds
/// the blend collision object's update, which it calls directly.
#[test]
fn collision_world_data_walk() {
    Diff::new(entry!(
        0x00a6_8c60,
        culling::update::update_world_data(Ptr<_>, u32)
    ))
    .common()
    .inputs(&inputs(&[&[Input::Of(&[0, 2]), Input::Flag], &P3]))
    .setup(|e, case| {
        stand_ins(e, case.word(0));
        let n = node(e, case, 0, 2);
        let c = object(e, 0x28);
        e.mem.set_u32(c + 8, n);
        e.mem.set_u16(c + 0xc, 0x40);
        if case.word(1) != 0 {
            set_slot(e, c, 0x90, BLEND_UPDATE_WORLD_DATA);
        }
        e.mem.set_u32(n + 0x1c, c);
        vec![n, 0x55]
    })
    .run()
    .assert_same();
}
