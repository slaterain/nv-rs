//! The differential test of every typed function of the particles family (`engine_typed::particles`)
//! against its translation: here because the translations live in the part crates, and this crate
//! registers them all. The tests share one kind of world: particle data with its arrays filled from
//! the case's floats, a particle system with a world transform and a modifier list, the modifiers
//! with their fields, an update controller with a target and a controller chain; every object has a
//! vtable of its own whose slots log and answer from the words after them, unless a test puts the
//! real function there (the modifiers, the system's updates, the data's particle removal), which the
//! typed side then calls directly.
//!
//! What stands in: the square roots (`00ec6040`, `00ec6054`) and `exp`, `NiParticlesData`'s particle
//! removal, the system reset, the data's change mark, the update hand-off's callees, the mesh and
//! strip bound updates, the memory manager and the thread id. Everything else runs translated.

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::{Case, Diff, Input};
use engine_core::typed::{MAIN_THREAD, OTHER_THREADS};
use engine_typed::particles::{self, modifiers, system};
use engine_typed::{culling, dispatch, pathing};

/// Where the test's virtual functions are registered: past `.text`, one per vtable slot.
const VT_FN: u32 = 0x00ff_2000;
/// The size of a test vtable, and where its slots' answers start after it.
const VT_SLOTS: u32 = 0x200;
/// The log the stand-ins write (a count, then `(tag, word)` pairs), and the words a test sets for
/// the hand-off's object (`LOG + 0x3ff0`).
const LOG: u32 = 0x0300_0000;
const HANDOFF_OBJECT_WORD: u32 = LOG + 0x3ff0;
const MAX_CALLS: usize = 5_000;
const FLOAT_SWEEP: usize = 32;

const ONE: u32 = 0x3f80_0000;
const TWO: u32 = 0x4000_0000;
const HALF: u32 = 0x3f00_0000;
const QUARTER: u32 = 0x3e80_0000;
const MINUS_ONE: u32 = 0xbf80_0000;
const THIRD: u32 = 0x3eaa_aaab;
const TEN: u32 = 0x4120_0000;

/// The real functions a test can put in a slot.
const UPDATE_SYSTEM: u32 = 0x00c1_ae40;
const DO_UPDATE_SYSTEM: u32 = 0x00c1_b260;
const REMOVE_PARTICLE: u32 = 0x00c2_4cc0;
const BOMB: u32 = 0x00c2_e670;
const POSITION: u32 = 0x00c2_8bc0;
const AGE_DEATH: u32 = 0x00c2_f010;
const ROTATION: u32 = 0x00c2_06b0;
const GROW_FADE: u32 = 0x00c2_a390;
const BOUND_UPDATE: u32 = 0x00c2_df20;
const SIMPLE_COLOR: u32 = 0x00c6_02e0;
/// The RTTI records the code compares with: `NiPSysEmitterCtlr`'s, the mesh and strip particle data's.
const EMITTER_CTLR_RTTI: u32 = 0x0120_24e8;
const MESH_PSYS_DATA_RTTI: u32 = 0x0120_27c0;
const STRIP_PSYS_DATA_RTTI: u32 = 0x0120_2680;
/// The rotation modifier's static limit and guard, the hand-off's switch, manager and thread.
const STATIC_LIMIT: u32 = 0x0120_2520;
const STATIC_GUARD: u32 = 0x0120_2524;
const HANDOFF_SWITCH: u32 = 0x0120_24d8;
const HANDOFF_THREAD: u32 = 0x0120_24dc;
const HANDOFF_MANAGER: u32 = 0x011f_5b04;
/// "No time yet", as a float.
const NO_TIME: u32 = 0xff7f_ffff;

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

/// A new block holding `w`.
fn block(e: &mut Engine, size: u32, w: &[u32]) -> u32 {
    let a = e.mem.alloc(size.max(4 * w.len() as u32));
    words(e, a, w);
    a
}

/// The double two argument words from `i` make.
fn f64_arg(a: &[u32], i: usize) -> f64 {
    f64::from_bits(u64::from(a[i]) | u64::from(a[i + 1]) << 32)
}

/// The exe's data these functions read, and the stand-ins.
fn stand_ins(e: &mut Engine) {
    e.map(LOG, 0x4000);
    for page in [
        0x0101_1000,
        0x0101_2000,
        0x0101_5000,
        0x0101_7000,
        0x0102_4000,
        0x010b_d000,
        0x010b_e000,
        0x010c_0000,
        0x011a_b000,
        0x011a_e000,
        0x011f_4000,
        0x011f_5000,
        0x011f_6000,
        0x0120_2000,
    ] {
        e.map(page, 0x1000);
    }
    // 0.5 and 0.0 as doubles, the start key times, "no time yet" as a double, the rotation's limit,
    // the smallest size, the merge's threshold, 2π, the hand-off's word, the zero point (not zero
    // here, so a copy of it shows), the length minimum.
    e.mem.set_u64(0x0101_1588, 0.5f64.to_bits());
    e.mem.set_u64(0x0101_2060, 0);
    e.mem.set_u32(0x0101_5f5c, NO_TIME);
    e.mem.set_u64(0x0101_7cf8, 0.001f64.to_bits());
    e.mem.set_u64(0x0102_41b0, f64::from(f32::MIN).to_bits());
    e.mem.set_u32(0x010b_db90, 31.415_928f32.to_bits());
    e.mem.set_u32(0x010b_e6a8, 0x7f7f_ffff);
    e.mem.set_u32(0x010c_09c0, QUARTER);
    e.mem.set_u32(0x011a_b3c0, 0x3586_37bd);
    e.mem.set_u32(0x011a_ea98, std::f32::consts::TAU.to_bits());
    e.mem.set_u32(0x011a_e6e0, 0x77);
    words(e, 0x011f_426c, &[HALF, 0, TWO]);
    // The square roots (not logged: the native bomb takes its own, so a log in memory would differ) and
    // `exp`.
    e.register(0x00ec_6040, |_, a| ret_f(f64_arg(a, 0).sqrt()));
    e.register(0x00ec_6054, |_, a| ret_f(f64_arg(a, 0).sqrt()));
    e.register(0x00ec_ae6c, |e, a| {
        log(e, 0xae6c, a[1]);
        ret_f(f64_arg(a, 0).exp())
    });
    // `NiParticlesData::RemoveParticle`: the count goes down.
    e.register(0x00a9_64c0, |e, a| {
        log(e, 0x64c0, a[1]);
        let n = e.mem.u16(a[0] + 0x50);
        e.mem.set_u16(a[0] + 0x50, n.wrapping_sub(1));
        ret(0)
    });
    // The system reset, the change mark, the thread id, the hand-off's object and set-up, the mesh
    // and strip bound updates.
    e.register(0x00c1_b120, logged::<0xb120>);
    e.register(0x00a6_7090, logged::<0x7090>);
    e.register(0x00c2_58e0, logged::<0x58e0>);
    e.register(0x00c2_dce0, logged::<0xdce0>);
    e.register(0x00c2_d880, logged::<0xd880>);
    e.register(0x00fd_f1d0, |_, _| ret(0x1234));
    e.register(0x00c2_5c40, |e, _| ret(e.mem.u32(HANDOFF_OBJECT_WORD)));
    // The memory manager: a block, or the free logged.
    e.register(0x00aa_3e40, |e, a| {
        log(e, 0x3e40, a[1]);
        ret(e.mem.alloc(a[1].min(0x400)))
    });
    e.register(0x00aa_4060, |e, a| {
        log(e, 0x4060, a[1]);
        ret(0)
    });
    for s in (0..VT_SLOTS).step_by(4) {
        e.register(VT_FN + s, any_slot);
    }
}

/// A stand-in that logs its words under `TAG` and answers 0.
fn logged<const TAG: u32>(e: &mut Engine, a: &[u32]) -> Ret {
    log(e, TAG, a.first().copied().unwrap_or(0));
    for w in a.iter().skip(1) {
        log(e, TAG | 0x1_0000, *w);
    }
    ret(0)
}

/// The stand-in for any slot: logs `this` and its words, and answers with the object's answer word
/// (after its vtable's slots), in `EAX` and as a float in `ST0`.
fn any_slot(e: &mut Engine, a: &[u32]) -> Ret {
    let this = a.first().copied().unwrap_or(0);
    log(e, 0xffff, this);
    for w in a.iter().skip(1) {
        log(e, 0xfffe, *w);
    }
    let vt = e.mem.u32(this);
    let answer = e.mem.u32(vt + VT_SLOTS);
    Ret {
        eax: answer,
        st0: f64::from(f32::from_bits(answer)),
        ..Ret::default()
    }
}

/// A vtable whose every slot is the logging stand-in, followed by the answer words.
fn vtable(e: &mut Engine) -> u32 {
    let vt = e.mem.alloc(VT_SLOTS + 0x40);
    for slot in (0..VT_SLOTS).step_by(4) {
        e.mem.set_u32(vt + slot, VT_FN + slot);
    }
    vt
}

/// An object with a vtable, `size` bytes.
fn object(e: &mut Engine, size: u32) -> u32 {
    let o = e.mem.alloc(size);
    let vt = vtable(e);
    e.mem.set_u32(o, vt);
    o
}

/// Slot `slot` of `object`'s vtable set to `target`.
fn set_slot(e: &mut Engine, object: u32, slot: u32, target: u32) {
    let vt = e.mem.u32(object);
    e.mem.set_u32(vt + slot, target);
}

/// What every stand-in slot of `object` answers.
fn set_answer(e: &mut Engine, object: u32, answer: u32) {
    let vt = e.mem.u32(object);
    e.mem.set_u32(vt + VT_SLOTS, answer);
}

/// Particle data with `count` active particles of a capacity of `capacity`: positions, velocities,
/// ages, life spans, last updates and generations, sizes, radii, rotation angles and speeds and
/// colours, each from a table mixed with the case's floats from `f` (three of them).
fn particle_data(e: &mut Engine, case: &Case, count: u32, capacity: u32, f: usize) -> u32 {
    let d = object(e, 0x7c);
    let n = capacity.max(1);
    let table = [
        ONE,
        HALF,
        MINUS_ONE,
        TWO,
        THIRD,
        0,
        TEN,
        QUARTER,
        0x3fc0_0000,
        0xc000_0000,
        0x3dcc_cccd,
        0x4040_0000,
    ];
    let pick = |k: u32| -> u32 {
        match k % 5 {
            0 => bits(case, f),
            1 => bits(case, f + 1),
            2 => bits(case, f + 2),
            _ => table[(k as usize * 7) % table.len()],
        }
    };
    let vertices = e.mem.alloc(0xc * n);
    let infos = e.mem.alloc(0x1c * n);
    let sizes = e.mem.alloc(4 * n);
    let radii = e.mem.alloc(4 * n);
    let angles = e.mem.alloc(4 * n);
    let speeds = e.mem.alloc(4 * n);
    let colors = e.mem.alloc(0x10 * n);
    for i in 0..n {
        for k in 0..3 {
            e.mem.set_u32(vertices + 0xc * i + 4 * k, pick(3 * i + k));
        }
        let info = infos + 0x1c * i;
        words(
            e,
            info,
            &[
                pick(i + 1),
                pick(i + 2),
                pick(i + 3),
                pick(i + 4),
                pick(i + 5),
                pick(i + 6),
            ],
        );
        e.mem.set_u16(info + 0x18, (i % 3) as u16);
        e.mem.set_u16(info + 0x1a, 0x5a);
        e.mem.set_u32(sizes + 4 * i, pick(i + 7));
        e.mem.set_u32(radii + 4 * i, pick(i + 8));
        e.mem.set_u32(angles + 4 * i, pick(i + 9));
        e.mem.set_u32(speeds + 4 * i, pick(i + 10));
        for k in 0..4 {
            e.mem.set_u32(colors + 0x10 * i + 4 * k, 0x1000 + k);
        }
    }
    e.mem.set_u16(d + 0x08, capacity as u16);
    words(e, d + 0x10, &[ONE, TWO, HALF, THIRD]);
    e.mem.set_u32(d + 0x20, vertices);
    e.mem.set_u32(d + 0x28, colors);
    e.mem.set_u32(d + 0x4c, radii);
    e.mem.set_u16(d + 0x50, count as u16);
    e.mem.set_u32(d + 0x54, sizes);
    e.mem.set_u32(d + 0x5c, angles);
    e.mem.set_u32(d + 0x70, infos);
    e.mem.set_u32(d + 0x74, speeds);
    d
}

/// A transform of 13 floats: the case's at `i` (scale) and `i + 1` (first entry), the rest from a
/// table, at `at`.
fn transform_at(e: &mut Engine, at: u32, case: &Case, i: usize) {
    let table = [
        ONE, HALF, 0, THIRD, ONE, MINUS_ONE, 0, TWO, ONE, TWO, MINUS_ONE, HALF,
    ];
    words(e, at + 4, &table[1..9]);
    words(e, at + 0x24, &table[9..12]);
    e.mem.set_u32(at, bits(case, i + 1));
    e.mem.set_u32(at + 0x30, bits(case, i));
}

/// A particle system (0x110 bytes) with a world transform from the case's floats at `f`, `data` as
/// its model data, the modifiers in `modifiers` as its list and a parent chain of `parents` objects.
fn particle_system(e: &mut Engine, case: &Case, data: u32, modifiers: &[u32], f: usize) -> u32 {
    let s = object(e, 0x110);
    transform_at(e, s + 0x68, case, f);
    e.mem.set_u32(s + 0xb8, data);
    set_modifiers(e, s, modifiers);
    e.mem.set_u32(s + 0xd4, ONE);
    s
}

/// The system's modifier list: an item (next, previous, modifier) for each.
fn set_modifiers(e: &mut Engine, system: u32, modifiers: &[u32]) {
    let mut next = 0;
    for &m in modifiers.iter().rev() {
        next = block(e, 0xc, &[next, 0, m]);
    }
    e.mem.set_u32(system + 0xc8, next);
}

/// A modifier of `size` bytes, active, its target `system`.
fn modifier(e: &mut Engine, size: u32, system: u32) -> u32 {
    let m = object(e, size);
    e.mem.set_u32(m + 0x10, system);
    e.mem.set_u8(m + 0x14, 1);
    m
}

/// A modifier whose `Update` slot holds `kind` (0: the stand-in), sized for its class, its fields
/// set so that its update runs through (the case's float at `f` among them), its target `system`.
fn modifier_of(e: &mut Engine, case: &Case, kind: u32, system: u32, f: usize) -> u32 {
    let size = match kind {
        BOMB | ROTATION => 0x38,
        AGE_DEATH | BOUND_UPDATE => 0x20,
        GROW_FADE => 0x2c,
        SIMPLE_COLOR => 0x60,
        _ => 0x40,
    };
    let m = modifier(e, size, system);
    if kind != 0 {
        set_slot(e, m, 0x8c, kind);
    }
    match kind {
        GROW_FADE => words(e, m + 0x18, &[ONE, 0, bits(case, f), 1, HALF]),
        BOUND_UPDATE => {
            e.mem.set_u16(m + 0x18, 2);
            let bounds = e.mem.alloc(0x40);
            e.mem.set_u32(m + 0x1c, bounds);
        }
        SIMPLE_COLOR => {
            for k in 0..18 {
                e.mem
                    .set_u32(m + 0x18 + 4 * k, [ONE, HALF, QUARTER][(k % 3) as usize]);
            }
        }
        BOMB => words(e, m + 0x1c, &[HALF, ONE, 0, TEN, bits(case, f), 1, 0]),
        _ => {}
    }
    m
}

/// The particle count, the capacity and the case's floats the arrays take.
const COUNTS: &[u32] = &[0, 1, 3];
/// Three floats: one from the operand classes, two swept.
const P3: [Input; 3] = [Input::F32, Input::F32Swept(ONE), Input::F32Swept(MINUS_ONE)];

fn inputs(parts: &[&[Input]]) -> Vec<Input> {
    parts.iter().flat_map(|p| p.iter().copied()).collect()
}

/// The dispatch family's functions this family's code calls: the bomb modifier's `invert_non_uniform`,
/// `compose`, `rotate` and `normalize`, and the controller walk's `first_controller`, `next_controller`
/// and `has_property_controllers`.
const DISPATCH_CALLED: &[u32] = &[
    0x004b_4880,
    0x0062_c250,
    0x004b_4500,
    0x004a_0c10,
    0x0043_b230,
    0x004a_8a90,
    0x004f_0000,
];

/// The culling family's functions this family's code calls: the bound update's merge of bounds.
const CULLING_CALLED: &[u32] = &[0x00a7_f3f0];

/// What every test adds to its comparison: the families, the call budget and the float sweep. The
/// square root's callers reach its stand-in by address (the crt family has its own test).
trait Common {
    fn common(self) -> Diff;
    /// [`Common::common`] with the dispatch and culling families' functions this family calls
    /// ([`DISPATCH_CALLED`], [`CULLING_CALLED`]) by address, to their translations on both sides.
    /// Their maths is native: a sum of products rounds once where the exe's 24-bit code rounds each
    /// step, so on the main thread's control word it parts from the exe in the last bits (their own
    /// tests compare it there, within their tolerances). This family's maths, compared on that word,
    /// runs them translated.
    fn callees_translated(self) -> Diff;
    /// One of [`BY_THREAD`]'s settings, kept in the comparison's own chain.
    fn by(self, run: fn(Diff) -> Diff) -> Diff;
}

impl Common for Diff {
    fn common(self) -> Diff {
        families(self.family(&dispatch::DISPATCH).family(&culling::CULLING))
    }

    fn callees_translated(self) -> Diff {
        families(
            self.goes_by_address(DISPATCH_CALLED)
                .goes_by_address(CULLING_CALLED),
        )
    }

    fn by(self, run: fn(Diff) -> Diff) -> Diff {
        run(self)
    }
}

/// [`Common`]'s families but dispatch and culling, and the rest it adds.
fn families(d: Diff) -> Diff {
    d.family(&particles::PARTICLES)
        .family(&pathing::PATHING)
        .family(&leaf::LEAVES)
        .goes_by_address(&[0x00ec_6040])
        // What the translations of the dispatch family's matrix functions and of the controller walk
        // call plainly (its PLAIN list).
        .translation_calls_plainly(&[
            0x0040_19d0,
            0x0040_8860,
            0x0047_6930,
            0x0047_6980,
            0x0047_69c0,
            0x004f_0000,
        ])
        // The native modifiers take their square roots with `FSQRT`, not the CRT's `_CIsqrt` the
        // translations call.
        .ignore_calls(&[0x00ec_6040])
        .max_calls(MAX_CALLS)
        .float_sweep(FLOAT_SWEEP)
}

/// Each comparison runs twice, split by thread as the dispatch family's tests (ADR-0014): under the other
/// threads' control word bit for bit (NaN payloads aside: the family copies floats plain where the exe
/// moved them through the x87), with the dispatch and culling families direct ([`other_threads`]); and
/// under the main thread's within one ulp, with their functions by address to their translations
/// ([`main_thread`]):
/// plain `f32` rounds each step to 24 bits as the exe's code does there, but the x87's wider exponent and
/// a double rounding can part in the last bit.
const BY_THREAD: [fn(Diff) -> Diff; 2] = [other_threads, main_thread];

/// [`BY_THREAD`]'s run under the other threads' control word.
fn other_threads(d: Diff) -> Diff {
    d.common()
        .control_words(&[OTHER_THREADS])
        .allow_nan_payloads()
}

/// [`BY_THREAD`]'s run under the main thread's control word.
fn main_thread(d: Diff) -> Diff {
    d.callees_translated()
        .control_words(&[MAIN_THREAD])
        .float_ulps(1)
}

/// The fixed rounding checks run under both control words bit for bit: each input is one where
/// rounding every step to a float (the main thread's 24 bits) and rounding once (the other threads'
/// 53) part, so maths in the wrong precision for the thread fails there, where [`BY_THREAD`]'s main
/// thread run would let its last bit through. Neither run sweeps floats.
const ROUNDING_BY_THREAD: [fn(Diff) -> Diff; 2] = [other_threads_fixed, main_thread_exact];

/// [`ROUNDING_BY_THREAD`]'s run under the other threads' control word: [`other_threads`].
fn other_threads_fixed(d: Diff) -> Diff {
    other_threads(d).float_sweep(0)
}

/// [`ROUNDING_BY_THREAD`]'s run under the main thread's control word: [`main_thread`] without the ulp.
fn main_thread_exact(d: Diff) -> Diff {
    d.callees_translated()
        .control_words(&[MAIN_THREAD])
        .float_sweep(0)
}

/// The fixed checks' one case: the three floats the particle data's arrays take (each check sets the
/// fields it reads).
const FIXED: [Input; 3] = [
    Input::F32Swept(ONE),
    Input::F32Swept(HALF),
    Input::F32Swept(TWO),
];

// ---- The modifiers ----

#[test]
fn particles_position() {
    // The count; the time; the particles' floats.
    for base in BY_THREAD {
        Diff::new(entry!(
            0x00c2_8bc0,
            modifiers::position(Ptr<_>, f32, Ptr<_>)
        ))
        .by(base)
        .inputs(&inputs(&[&[Input::Of(COUNTS), Input::F32], &P3]))
        .max_cases(160)
        .setup(|e, case| {
            stand_ins(e);
            let d = particle_data(e, case, case.word(0), 3, 2);
            vec![0x55, bits(case, 1), d]
        })
        .run()
        .assert_same();
    }
}

#[test]
fn particles_age_death() {
    // The count; spawning on death; a spawn modifier; the data's removal the typed one or a stand-in;
    // the time; the particles' floats.
    for base in BY_THREAD {
        Diff::new(entry!(
            0x00c2_f010,
            modifiers::age_death(Ptr<_>, f32, Ptr<_>)
        ))
        .by(base)
        .inputs(&inputs(&[
            &[
                Input::Of(COUNTS),
                Input::Flag,
                Input::Flag,
                Input::Flag,
                Input::F32,
            ],
            &P3,
        ]))
        .max_cases(192)
        .setup(|e, case| {
            stand_ins(e);
            let d = particle_data(e, case, case.word(0), 3, 5);
            if case.word(3) != 0 {
                set_slot(e, d, 0xa4, REMOVE_PARTICLE);
            }
            let s = particle_system(e, case, d, &[], 5);
            let m = modifier(e, 0x20, s);
            e.mem.set_u8(m + 0x18, case.word(1) as u8);
            if case.word(2) != 0 {
                let spawner = object(e, 0x40);
                e.mem.set_u32(m + 0x1c, spawner);
            }
            vec![m, bits(case, 4), d]
        })
        .run()
        .assert_same();
    }
}

#[test]
fn particles_remove_particle() {
    // The count; the index word (its high half kept for the inner call); rotation speeds or none.
    for base in BY_THREAD {
        Diff::new(entry!(0x00c2_4cc0, modifiers::remove_particle(Ptr<_>, u32)))
            .by(base)
            .inputs(&inputs(&[
                &[
                    Input::Of(&[1, 2, 3]),
                    Input::Of(&[0, 1, 2, 0x0001_0001]),
                    Input::Flag,
                ],
                &P3,
            ]))
            .setup(|e, case| {
                stand_ins(e);
                let d = particle_data(e, case, case.word(0), 3, 3);
                if case.word(2) == 0 {
                    e.mem.set_u32(d + 0x74, 0);
                }
                vec![d, case.word(1)]
            })
            .run()
            .assert_same();
    }
}

#[test]
fn particles_rotation() {
    // Angles or none; the count; the static's guard set or not (a set guard keeps the limit 10, a
    // clear one sets 31.4); the time; the first particle's angle (20 loses 2π three times, 50 passes
    // either limit); the angles and speeds.
    for base in BY_THREAD {
        Diff::new(entry!(
            0x00c2_06b0,
            modifiers::rotation(Ptr<_>, f32, Ptr<_>)
        ))
        .by(base)
        .inputs(&inputs(&[
            &[
                Input::Flag,
                Input::Of(COUNTS),
                Input::Flag,
                Input::Of(&[ONE, TEN, 0x4248_0000, 0x7fc0_0000, MINUS_ONE]),
                Input::Of(&[HALF, 0x41a0_0000, 0x4248_0000, 0x7fc0_0000]),
            ],
            &P3,
        ]))
        .max_cases(192)
        .setup(|e, case| {
            stand_ins(e);
            let d = particle_data(e, case, case.word(1), 3, 5);
            let angles = e.mem.u32(d + 0x5c);
            e.mem.set_u32(angles, case.word(4));
            if case.word(0) == 0 {
                e.mem.set_u32(d + 0x5c, 0);
            }
            e.mem.set_u32(STATIC_GUARD, case.word(2));
            e.mem.set_u32(STATIC_LIMIT, TEN);
            vec![0x55, case.word(3), d]
        })
        .run()
        .assert_same();
    }
    // Rounding: 3.5 - 0.5 = 3 times a speed just past -2/3, plus the angle 2, is 0 when the product
    // rounds to a float first and -2^-24 when the sum rounds once.
    for base in ROUNDING_BY_THREAD {
        Diff::new(entry!(
            0x00c2_06b0,
            modifiers::rotation(Ptr<_>, f32, Ptr<_>)
        ))
        .by(base)
        .inputs(&FIXED)
        .setup(|e, case| {
            stand_ins(e);
            let d = particle_data(e, case, 1, 1, 0);
            let info = e.mem.u32(d + 0x70);
            e.mem.set_u32(info + 0x14, HALF);
            let speeds = e.mem.u32(d + 0x74);
            e.mem.set_u32(speeds, 0xbf2a_aaab);
            let angles = e.mem.u32(d + 0x5c);
            e.mem.set_u32(angles, TWO);
            e.mem.set_u32(STATIC_GUARD, 1);
            e.mem.set_u32(STATIC_LIMIT, TEN);
            vec![0x55, 0x4060_0000, d]
        })
        .run()
        .assert_same();
    }
}

#[test]
fn particles_grow_fade() {
    // The count; the grow and fade generations (matching particle 0's or not); the grow time, the
    // fade time and the base scale.
    for base in BY_THREAD {
        Diff::new(entry!(
            0x00c2_a390,
            modifiers::grow_fade(Ptr<_>, f32, Ptr<_>)
        ))
        .by(base)
        .inputs(&inputs(&[
            &[
                Input::Of(COUNTS),
                Input::Of(&[0, 1, 7]),
                Input::Of(&[0, 1, 7]),
            ],
            &P3,
            &[Input::F32Swept(HALF)],
        ]))
        .max_cases(192)
        .setup(|e, case| {
            stand_ins(e);
            let d = particle_data(e, case, case.word(0), 3, 3);
            let s = particle_system(e, case, d, &[], 3);
            let m = modifier(e, 0x2c, s);
            words(e, m + 0x18, &[bits(case, 3), case.word(1), bits(case, 4)]);
            e.mem.set_u32(m + 0x24, case.word(2));
            e.mem.set_u32(m + 0x28, bits(case, 6));
            vec![m, ONE, d]
        })
        .run()
        .assert_same();
    }
    // Generation 2 (particle 2's) matching, and the zero-time guards: particle 2's age is -1 and its
    // life span -2, so a zero grow time is above its age and a zero fade time above its remaining life,
    // and only the guards keep the factors from dividing by zero.
    for base in BY_THREAD {
        Diff::new(entry!(
            0x00c2_a390,
            modifiers::grow_fade(Ptr<_>, f32, Ptr<_>)
        ))
        .by(base)
        .inputs(&inputs(&[
            &[
                Input::Of(&[2, 7]),
                Input::Of(&[2, 7]),
                Input::Of(&[0, ONE]),
                Input::Of(&[0, ONE]),
            ],
            &P3,
        ]))
        .max_cases(256)
        .setup(|e, case| {
            stand_ins(e);
            let d = particle_data(e, case, 3, 3, 4);
            let info = e.mem.u32(d + 0x70) + 2 * 0x1c;
            e.mem.set_u32(info + 0x0c, MINUS_ONE);
            e.mem.set_u32(info + 0x10, 0xc000_0000);
            let s = particle_system(e, case, d, &[], 4);
            let m = modifier(e, 0x2c, s);
            words(e, m + 0x18, &[case.word(2), case.word(0), case.word(3)]);
            e.mem.set_u32(m + 0x24, case.word(1));
            e.mem.set_u32(m + 0x28, bits(case, 4));
            vec![m, ONE, d]
        })
        .run()
        .assert_same();
    }
    // Rounding: particle 0 (generation 0, age 1/3) grows over 0.5, so its size is 2/3 times 0.9 plus
    // the base scale 0.1: 0x3f333334 with the product rounded to a float first, 0x3f333333 rounded
    // once.
    for base in ROUNDING_BY_THREAD {
        Diff::new(entry!(
            0x00c2_a390,
            modifiers::grow_fade(Ptr<_>, f32, Ptr<_>)
        ))
        .by(base)
        .inputs(&FIXED)
        .setup(|e, case| {
            stand_ins(e);
            let d = particle_data(e, case, 1, 1, 0);
            let info = e.mem.u32(d + 0x70);
            words(e, info + 0x0c, &[THIRD, ONE]);
            let s = particle_system(e, case, d, &[], 0);
            let m = modifier(e, 0x2c, s);
            // The grow time and generation, the fade time and generation (7: none), the base scale.
            words(e, m + 0x18, &[HALF, 0, 0, 7, 0x3dcc_cccd]);
            vec![m, ONE, d]
        })
        .run()
        .assert_same();
    }
}

#[test]
fn particles_bound_update() {
    // The count (60 makes the step's limit 2); the skip (-1 sets it from the capacity); the update
    // count; plain, mesh or strip data; the particles' floats.
    for base in BY_THREAD {
        Diff::new(entry!(
            0x00c2_df20,
            modifiers::bound_update(Ptr<_>, f32, Ptr<_>)
        ))
        .by(base)
        .inputs(&inputs(&[
            &[
                Input::Of(&[0, 1, 3, 60]),
                Input::Of(&[0xffff, 1, 2, 3]),
                Input::Of(&[0, 1, 2]),
                Input::Of(&[0, MESH_PSYS_DATA_RTTI, STRIP_PSYS_DATA_RTTI]),
            ],
            &P3,
        ]))
        .max_cases(256)
        .setup(|e, case| {
            stand_ins(e);
            let count = case.word(0);
            let d = particle_data(e, case, count, count.max(3), 4);
            // The data's RTTI chain: its own record, whose base is the asked one (none for plain data).
            let chain = block(e, 8, &[0x0120_2558, case.word(3)]);
            set_answer(e, d, chain);
            let s = particle_system(e, case, d, &[], 4);
            let m = modifier(e, 0x20, s);
            let skip = case.word(1) as u16;
            e.mem.set_u16(m + 0x18, skip);
            // The update count stays below the skip bounds the modifier has: a skip of -1 makes as few
            // as one new one.
            let update_count = if skip == 0xffff { 0 } else { case.word(2) };
            e.mem.set_u16(m + 0x1a, update_count as u16);
            let bounds = e.mem.alloc(0x10 * 4);
            for i in 0..4 {
                words(
                    e,
                    bounds + 0x10 * i,
                    &[ONE, TWO, HALF, [0, ONE, TWO, 0][i as usize]],
                );
            }
            e.mem.set_u32(m + 0x1c, bounds);
            vec![m, bits(case, 4), d]
        })
        .run()
        .assert_same();
    }
    // The merge's zero-radius skip: 100 or more particles make the step's limit 3 or more, so with a
    // skip of 3 or 4 the bounds after the first merge, bound 2's radius zero (left out unless the
    // update count is 2 and has just written it).
    for base in BY_THREAD {
        Diff::new(entry!(
            0x00c2_df20,
            modifiers::bound_update(Ptr<_>, f32, Ptr<_>)
        ))
        .by(base)
        .inputs(&inputs(&[
            &[
                Input::Of(&[100, 150, 200]),
                Input::Of(&[3, 4]),
                Input::Of(&[0, 1, 2]),
            ],
            &P3,
        ]))
        .max_cases(128)
        .setup(|e, case| {
            stand_ins(e);
            let count = case.word(0);
            let d = particle_data(e, case, count, count, 3);
            let chain = block(e, 8, &[0x0120_2558, 0]);
            set_answer(e, d, chain);
            let s = particle_system(e, case, d, &[], 3);
            let m = modifier(e, 0x20, s);
            e.mem.set_u16(m + 0x18, case.word(1) as u16);
            e.mem.set_u16(m + 0x1a, case.word(2) as u16);
            let bounds = e.mem.alloc(0x10 * 4);
            for i in 0..4 {
                words(
                    e,
                    bounds + 0x10 * i,
                    &[ONE, TWO, HALF, [ONE, TWO, 0, HALF][i as usize]],
                );
            }
            e.mem.set_u32(m + 0x1c, bounds);
            vec![m, bits(case, 3), d]
        })
        .run()
        .assert_same();
    }
    // Rounding: one particle at (1.7, 1.9, 2.6) of radius 0.7 makes a box 1.4 wide on each axis,
    // whose half diagonal parts in its last bit between its sum of squares rounded each step to a float
    // and rounded once.
    for base in ROUNDING_BY_THREAD {
        Diff::new(entry!(
            0x00c2_df20,
            modifiers::bound_update(Ptr<_>, f32, Ptr<_>)
        ))
        .by(base)
        .inputs(&FIXED)
        .setup(|e, case| {
            stand_ins(e);
            let d = particle_data(e, case, 1, 1, 0);
            let chain = block(e, 8, &[0x0120_2558, 0]);
            set_answer(e, d, chain);
            let vertices = e.mem.u32(d + 0x20);
            words(e, vertices, &[0x3fd9_999a, 0x3ff3_3334, 0x4026_6667]);
            let sizes = e.mem.u32(d + 0x54);
            e.mem.set_u32(sizes, ONE);
            let radii = e.mem.u32(d + 0x4c);
            e.mem.set_u32(radii, 0x3f33_3333);
            let s = particle_system(e, case, d, &[], 0);
            let m = modifier(e, 0x20, s);
            // A skip of 1 from update 0: one bound, nothing merged.
            e.mem.set_u16(m + 0x18, 1);
            e.mem.set_u16(m + 0x1a, 0);
            let bounds = block(e, 0x10, &[ONE, TWO, HALF, ONE]);
            e.mem.set_u32(m + 0x1c, bounds);
            vec![m, ONE, d]
        })
        .run()
        .assert_same();
    }
}

#[test]
fn particles_set_update_skip() {
    for base in BY_THREAD {
        Diff::new(entry!(0x00c2_d720, modifiers::set_update_skip(Ptr<_>, u32)))
            .by(base)
            .inputs(&[Input::Of(&[0, 1, 2, 3, 0xffff, 0x0001_0002, 0x0008_0004])])
            .setup(|e, case| {
                stand_ins(e);
                let m = object(e, 0x20);
                let old = e.mem.alloc(0x10);
                e.mem.set_u32(m + 0x1c, old);
                vec![m, case.word(0)]
            })
            .run()
            .assert_same();
    }
}

#[test]
fn particles_simple_color() {
    // Colours or none; the count; the colour points (span 1 zero or not, span 2 zero or not) and the
    // fade points; the particles' floats.
    for base in BY_THREAD {
        Diff::new(entry!(
            0x00c6_02e0,
            modifiers::simple_color(Ptr<_>, u32, Ptr<_>)
        ))
        .by(base)
        .inputs(&inputs(&[
            &[
                Input::Flag,
                Input::Of(COUNTS),
                Input::Of(&[0, 1]),
                Input::Of(&[0, 1]),
            ],
            &P3,
            &[Input::F32Swept(QUARTER), Input::F32Swept(0x3f40_0000)],
        ]))
        .max_cases(192)
        .setup(|e, case| {
            stand_ins(e);
            let d = particle_data(e, case, case.word(1), 3, 4);
            if case.word(0) == 0 {
                e.mem.set_u32(d + 0x28, 0);
            }
            let s = particle_system(e, case, d, &[], 4);
            let m = modifier(e, 0x60, s);
            for k in 0..12 {
                e.mem.set_u32(
                    m + 0x18 + 4 * k,
                    [ONE, HALF, 0, TWO][(k % 4) as usize] ^ (k << 8),
                );
            }
            // Fade in and out, colour 1's end, colour 2's start and end, colour 3's start.
            let (c1_end, c2_start) = if case.word(2) != 0 {
                (bits(case, 7), bits(case, 7))
            } else {
                (bits(case, 7), HALF)
            };
            let (c2_end, c3_start) = if case.word(3) != 0 {
                (0x3f40_0000, 0x3f40_0000)
            } else {
                (bits(case, 8), 0x3f66_6666)
            };
            words(
                e,
                m + 0x48,
                &[QUARTER, bits(case, 8), c1_end, c2_start, c2_end, c3_start],
            );
            vec![m, 0x55, d]
        })
        .run()
        .assert_same();
    }
    // Rounding: particle 0 (age 1/3) blends colour 1 into 2 at 2/3 and particle 1 (age 0.8) colour 2
    // into 3 at 0.2, their alphas likewise through the fade in and the fade out; with these colours
    // every component parts in its last bit between rounding each step to a float and rounding once.
    for base in ROUNDING_BY_THREAD {
        Diff::new(entry!(
            0x00c6_02e0,
            modifiers::simple_color(Ptr<_>, u32, Ptr<_>)
        ))
        .by(base)
        .inputs(&FIXED)
        .setup(|e, case| {
            stand_ins(e);
            let d = particle_data(e, case, 2, 2, 0);
            let info = e.mem.u32(d + 0x70);
            words(e, info + 0x0c, &[THIRD, ONE]);
            words(e, info + 0x1c + 0x0c, &[0x3f4c_cccd, ONE]);
            let s = particle_system(e, case, d, &[], 0);
            let m = modifier(e, 0x60, s);
            // Colours 1, 2 and 3.
            let colors: [[u32; 4]; 3] = [
                [0x3dcc_cccd, 0x3e4c_cccd, 0x3e99_999a, 0x3dcc_cccd], // 0.1, 0.2, 0.3, 0.1
                [0x3f33_3333, 0x3dcc_cccd, 0x3d4c_cccd, 0x3f33_3333], // 0.7, 0.1, 0.05, 0.7
                [0x3e99_999a, 0x3f26_6666, 0x3f26_6666, 0x3e99_999a], // 0.3, 0.65, 0.65, 0.3
            ];
            words(e, m + 0x18, &colors.concat());
            // Fade in 0.5, fade out 0.75; colour 1 ends at 0, colour 2 starts at 0.5 and ends at 0.75,
            // colour 3 starts at 1.
            words(e, m + 0x48, &[HALF, 0x3f40_0000, 0, HALF, 0x3f40_0000, ONE]);
            vec![m, 0x55, d]
        })
        .run()
        .assert_same();
    }
}

/// `particles_bomb`'s inputs: the count; a bomb object or none; the decay type; the symmetry type; the
/// time, the decay and delta-v; the particles' floats.
fn bomb_inputs() -> Vec<Input> {
    inputs(&[
        &[
            Input::Of(COUNTS),
            Input::Flag,
            Input::Of(&[0, 1, 2]),
            Input::Of(&[0, 1, 2, 3]),
            Input::F32Swept(TEN),
            Input::F32Swept(0x40a0_0000),
        ],
        &P3,
    ])
}

/// `particles_bomb`'s words for a case of [`bomb_inputs`].
fn bomb_words(e: &mut Engine, case: &Case) -> Vec<u32> {
    stand_ins(e);
    let d = particle_data(e, case, case.word(0), 3, 6);
    let s = particle_system(e, case, d, &[], 6);
    let m = modifier(e, 0x38, s);
    if case.word(1) != 0 {
        let o = object(e, 0xb0);
        transform_at(e, o + 0x68, case, 7);
        e.mem.set_u32(m + 0x18, o);
    }
    words(
        e,
        m + 0x1c,
        &[
            HALF,
            bits(case, 7),
            ONE,
            bits(case, 5),
            TWO,
            case.word(2),
            case.word(3),
        ],
    );
    vec![m, bits(case, 4), d]
}

/// The placed particle's inputs: the count; the decay type; the symmetry type; where particle 0 is;
/// the particles' floats.
fn bomb_placed_inputs() -> Vec<Input> {
    inputs(&[
        &[
            Input::Of(&[1, 3]),
            Input::Of(&[0, 1, 2]),
            Input::Of(&[0, 1, 2, 3]),
            Input::Of(&[0, 1, 2]),
        ],
        &P3,
    ])
}

/// The placed particle's words for a case of [`bomb_placed_inputs`].
fn bomb_placed_words(e: &mut Engine, case: &Case) -> Vec<u32> {
    stand_ins(e);
    let d = particle_data(e, case, case.word(0), 3, 4);
    let s = particle_system(e, case, d, &[], 4);
    let m = modifier(e, 0x38, s);
    words(
        e,
        m + 0x1c,
        &[0, 0, ONE, TEN, TWO, case.word(1), case.word(2)],
    );
    let vertex = e.mem.u32(d + 0x20);
    let info = e.mem.u32(d + 0x70);
    match case.word(3) {
        0 => words(e, vertex, &[HALF, 0, TWO]),
        1 => words(e, vertex, &[HALF, 0, 0x4040_0000]),
        _ => {}
    }
    let last = if case.word(3) == 2 { TEN } else { 0 };
    e.mem.set_u32(info + 0x14, last);
    vec![m, TEN, d]
}

#[test]
fn particles_bomb() {
    // Each comparison per thread ([`BY_THREAD`]).
    for base in BY_THREAD {
        Diff::new(entry!(0x00c2_e670, modifiers::bomb(Ptr<_>, f32, Ptr<_>)))
            .by(base)
            .inputs(&bomb_inputs())
            .max_cases(256)
            .setup(bomb_words)
            .run()
            .assert_same();
    }
    // Without a bomb object (the bomb at the zero point (0.5, 0, 2)) and the axis (0, 0, 1), particle 0
    // placed where the floats rarely put it: at the bomb (zero distance, and for symmetry 1 a zero
    // length off the axis), on the axis one away (symmetry 1's zero length at a nonzero distance), or
    // with its last update equal to the time (skipped); each decay and symmetry type; the time 10.
    for base in BY_THREAD {
        Diff::new(entry!(0x00c2_e670, modifiers::bomb(Ptr<_>, f32, Ptr<_>)))
            .by(base)
            .inputs(&bomb_placed_inputs())
            .max_cases(256)
            .setup(bomb_placed_words)
            .run()
            .assert_same();
    }
}

// ---- The system ----

/// The modifier kinds a system's list holds in [`particles_do_update_system`]: a stand-in (0) or
/// one of the typed ones in its update slot.
const MODIFIER_KINDS: &[u32] = &[
    0,
    POSITION,
    AGE_DEATH,
    ROTATION,
    GROW_FADE,
    BOUND_UPDATE,
    SIMPLE_COLOR,
    BOMB,
];

#[test]
fn particles_do_update_system() {
    // The list (empty, or two modifiers of kinds by the case, the second kept for last or not, the
    // first inactive or not); the reset flag; the last time (one, "no time yet", above the time); the
    // data's radius zero or not and whether its stand-ins answer a radius after; the time; floats.
    for base in BY_THREAD {
        Diff::new(entry!(0x00c1_b260, system::do_update_system(Ptr<_>, f32)))
            .by(base)
            .inputs(&inputs(&[
                &[
                    Input::Of(&[0, 1, 2, 3, 4, 5, 6, 7]),
                    Input::Of(&[0, 1, 2]),
                    Input::Flag,
                    Input::Of(&[ONE, NO_TIME, TEN]),
                    Input::Flag,
                    Input::F32Swept(TWO),
                ],
                &P3,
            ]))
            .max_cases(256)
            .setup(|e, case| {
                stand_ins(e);
                let d = particle_data(e, case, 3, 3, 6);
                if case.word(4) != 0 {
                    e.mem.set_u32(d + 0x1c, 0);
                }
                let s = particle_system(e, case, d, &[], 6);
                if case.word(0) != 0 || case.word(1) != 0 {
                    let kind = MODIFIER_KINDS[case.word(0) as usize];
                    let first = modifier_of(e, case, kind, s, 7);
                    if case.word(1) == 1 {
                        e.mem.set_u8(first + 0x14, 0);
                    }
                    // Slot +0xa0 answers the vtable's word: the second is kept for last when the case says.
                    let second = modifier_of(e, case, 0, s, 7);
                    set_answer(e, second, u32::from(case.word(1) == 2));
                    set_modifiers(e, s, &[first, second]);
                }
                e.mem.set_u8(s + 0xd8, case.word(2) as u8);
                e.mem.set_u32(s + 0xd4, case.word(3));
                // A parent chain: the first answers 0, the second nonzero.
                let p2 = object(e, 0x40);
                set_answer(e, p2, 1);
                let p1 = object(e, 0x40);
                e.mem.set_u32(p1 + 0x18, p2);
                e.mem.set_u32(s + 0x18, p1);
                vec![s, bits(case, 5)]
            })
            .run()
            .assert_same();
    }
}

#[test]
fn particles_update_system() {
    // The hand-off's switch, manager (none, its byte clear, set), the data's radius above zero or
    // not, the thread its own or another, the hand-off's object or none, the manager's answer; the
    // system's slot the typed `Do_UpdateSystem` or a stand-in; the time.
    for base in BY_THREAD {
        Diff::new(entry!(0x00c1_ae40, system::update_system(Ptr<_>, f32)))
            .by(base)
            .inputs(&inputs(&[
                &[
                    Input::Flag,
                    Input::Of(&[0, 1, 2]),
                    Input::Flag,
                    Input::Flag,
                    Input::Flag,
                    Input::Flag,
                    Input::Flag,
                ],
                &P3,
            ]))
            .max_cases(256)
            .setup(|e, case| {
                stand_ins(e);
                let d = particle_data(e, case, 1, 1, 7);
                e.mem
                    .set_u32(d + 0x1c, if case.word(2) != 0 { ONE } else { 0 });
                let s = particle_system(e, case, d, &[], 7);
                if case.word(6) != 0 {
                    set_slot(e, s, 0xf4, DO_UPDATE_SYSTEM);
                }
                e.mem.set_u8(HANDOFF_SWITCH, case.word(0) as u8);
                let manager = if case.word(1) != 0 {
                    let m = object(e, 0x1c0);
                    e.mem.set_u8(m + 0x1b0, (case.word(1) == 2) as u8);
                    set_answer(e, m, case.word(5));
                    m
                } else {
                    0
                };
                e.mem.set_u32(HANDOFF_MANAGER, manager);
                e.mem.set_u32(
                    HANDOFF_THREAD,
                    if case.word(3) != 0 { 0x1234 } else { 0x99 },
                );
                let object = if case.word(4) != 0 {
                    object(e, 0x40)
                } else {
                    0
                };
                e.mem.set_u32(HANDOFF_OBJECT_WORD, object);
                vec![s, bits(case, 7)]
            })
            .run()
            .assert_same();
    }
}

#[test]
fn particles_update_ctlr() {
    // The target's controller chain (none, a plain one, a plain one then an emitter controller);
    // the emitter controller's flags; the last time ("no time yet", one, the time's); the emitter's
    // life span; the controller's flags (active, forced, scaled); the target's slot the typed
    // `UpdateSystem` or a stand-in; the time.
    for base in BY_THREAD {
        Diff::new(entry!(0x00c2_71c0, system::update_ctlr(Ptr<_>, Ptr<_>)))
            .by(base)
            .inputs(&inputs(&[
                &[
                    Input::Of(&[0, 1, 2]),
                    Input::Of(&[0, 2, 4]),
                    Input::Of(&[NO_TIME, ONE, HALF]),
                    Input::Of(&[0, 0x08, 0x48, 0x88, 0xc8]),
                    Input::Flag,
                    Input::F32Swept(QUARTER),
                ],
                &P3,
            ]))
            .max_cases(256)
            .setup(|e, case| {
                stand_ins(e);
                let d = particle_data(e, case, 1, 1, 6);
                let s = particle_system(e, case, d, &[], 6);
                if case.word(4) != 0 {
                    set_slot(e, s, 0xf0, UPDATE_SYSTEM);
                }
                // The controllers: each answers its RTTI chain for slot 8, the emitter controller's a record
                // whose base is `NiPSysEmitterCtlr`'s.
                let plain_rtti = block(e, 8, &[0x0120_0000, 0]);
                let emitter_rtti = block(e, 8, &[0x0120_0004, EMITTER_CTLR_RTTI]);
                let emitter = block(e, 0x50, &[0; 20]);
                e.mem.set_u32(emitter + 0x48, bits(case, 5));
                let mut chain = 0;
                if case.word(0) == 2 {
                    let ec = object(e, 0x40);
                    set_answer(e, ec, emitter_rtti);
                    e.mem.set_u8(ec + 8, case.word(1) as u8);
                    e.mem.set_u32(ec + 0x3c, emitter);
                    chain = ec;
                }
                if case.word(0) >= 1 {
                    let pc = object(e, 0x40);
                    set_answer(e, pc, plain_rtti);
                    e.mem.set_u32(pc + 0x30, chain);
                    chain = pc;
                }
                e.mem.set_u32(s + 0x0c, chain);
                let c = object(e, 0x34);
                // DontDoUpdate's scaled time comes from the controller's slot +0xa4.
                set_answer(e, c, bits(case, 6));
                e.mem.set_u16(c + 8, case.word(3) as u16);
                e.mem.set_u32(c + 0x20, case.word(2));
                e.mem.set_u32(c + 0x28, HALF);
                e.mem.set_u32(c + 0x2c, s);
                let time = block(e, 0xc, &[bits(case, 6), 1, 0]);
                vec![c, time]
            })
            .run()
            .assert_same();
    }
    // Rounding: the emitter's life span 1 against the time 1.5 less a last time just below 0.5. The
    // difference rounded to a float is 1, at 53 bits just above it: only there does the last time
    // jump to the time, so that `DontDoUpdate` skips the system's update.
    for base in ROUNDING_BY_THREAD {
        Diff::new(entry!(0x00c2_71c0, system::update_ctlr(Ptr<_>, Ptr<_>)))
            .by(base)
            .inputs(&FIXED)
            .setup(|e, case| {
                stand_ins(e);
                let d = particle_data(e, case, 1, 1, 0);
                let s = particle_system(e, case, d, &[], 0);
                let emitter_rtti = block(e, 8, &[0x0120_0004, EMITTER_CTLR_RTTI]);
                let emitter = block(e, 0x50, &[0; 20]);
                e.mem.set_u32(emitter + 0x48, ONE);
                let ec = object(e, 0x40);
                set_answer(e, ec, emitter_rtti);
                e.mem.set_u32(ec + 0x3c, emitter);
                e.mem.set_u32(s + 0x0c, ec);
                let c = object(e, 0x34);
                set_answer(e, c, 0x3fc0_0000);
                // Active; the last time, the scaled time, the target.
                e.mem.set_u16(c + 8, 0x08);
                e.mem.set_u32(c + 0x20, 0x3eff_ffff);
                e.mem.set_u32(c + 0x28, HALF);
                e.mem.set_u32(c + 0x2c, s);
                let time = block(e, 0xc, &[0x3fc0_0000, 1, 0]);
                vec![c, time]
            })
            .run()
            .assert_same();
    }
}

/// The typed controller walk (the dispatch family's `004eff50`) with a particle system's update
/// controller in its chain: its `Update` slot holds `00c271c0`, which the walk calls directly.
#[test]
fn particles_controller_walk() {
    for base in BY_THREAD {
        Diff::new(entry!(
            0x004e_ff50,
            dispatch::scene::update_object_controllers(Ptr<_>, Ptr<_>, u8)
        ))
        .by(base)
        .inputs(&inputs(&[&[Input::Flag, Input::Flag], &P3]))
        .max_cases(64)
        .setup(|e, case| {
            stand_ins(e);
            let d = particle_data(e, case, 3, 3, 2);
            let m = modifier(e, 0x40, 0);
            set_slot(e, m, 0x8c, POSITION);
            let s = particle_system(e, case, d, &[m], 2);
            e.mem.set_u32(m + 0x10, s);
            set_slot(e, s, 0xf0, UPDATE_SYSTEM);
            set_slot(e, s, 0xf4, DO_UPDATE_SYSTEM);
            let c = object(e, 0x34);
            // The controller is in its own target's chain: its slot 8 answers a plain RTTI record.
            let rtti = block(e, 8, &[0x0120_0000, 0]);
            set_answer(e, c, rtti);
            e.mem.set_u16(c + 8, 0x08);
            e.mem.set_u32(c + 0x20, ONE);
            e.mem.set_u32(c + 0x28, bits(case, 2));
            e.mem.set_u32(c + 0x2c, s);
            if case.word(0) != 0 {
                set_slot(e, c, 0x94, 0x00c2_71c0);
            }
            // A second, plain controller after it.
            let c2 = object(e, 0x34);
            e.mem.set_u32(c + 0x30, c2);
            e.mem.set_u32(s + 0x0c, c);
            let data = block(e, 0xc, &[bits(case, 3), 1, 0]);
            vec![s, data, case.word(1)]
        })
        .run()
        .assert_same();
    }
}
