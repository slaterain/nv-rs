//! The differential test of every typed function of the dispatch family (`engine_typed::dispatch`)
//! against its translation: here because the translations live in the part crates, and this crate
//! registers them all. The tests share one kind of world: a node with a vtable whose slots log, a
//! controller chain, a property list, children with bounds, a reference with loaded 3D, lists,
//! arrays, an inventory and an actor; transforms, matrices and points from the case's floats, swept
//! under both control words (the family's row is `Threads::Both`).
//!
//! What stands in: the callees outside the family that reach far (the list walk, the bound copy's
//! helper, the frustum's planes and stamp, the inventory's container and extra lists, the water and
//! height of an actor, D3DX's inverse, the arrays' element destructors and set-ups, the square root)
//! and the imports (interlocked operations, the thread id, `Sleep`). Everything else runs translated.

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::{Case, Diff, Input, Value};
use engine_core::typed::{MAIN_THREAD, OTHER_THREADS};
use engine_typed::dispatch::{self, math, refs, scene};
use engine_typed::{culling, pathing};

/// Where the test's virtual functions are registered: past `.text`, one per vtable slot.
const VT_FN: u32 = 0x00ff_2000;
/// The log the stand-ins write (a count, then `(tag, word)` pairs).
const LOG: u32 = 0x0300_0000;
const MAX_CALLS: usize = 5_000;
const FLOAT_SWEEP: usize = 32;

const ONE: u32 = 0x3f80_0000;
const TWO: u32 = 0x4000_0000;
const HALF: u32 = 0x3f00_0000;
const MINUS_ONE: u32 = 0xbf80_0000;
const THIRD: u32 = 0x3eaa_aaab;

/// The RTTI the cast looks for, and the chain an object's slot 8 answers with.
const RTTI: u32 = 0x0120_43f8;

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

/// The exe's data these functions read, and the stand-ins.
fn stand_ins(e: &mut Engine) {
    e.map(LOG, 0x4000);
    for page in [
        0x0101_7000,
        0x0101_d000,
        0x0102_0000,
        0x0102_4000,
        0x0118_9000,
        0x011a_3000,
        0x011a_9000,
        0x011c_1000,
        0x011c_5000,
        0x011c_6000,
        0x011d_e000,
        0x011d_f000,
        0x011e_0000,
        0x011f_4000,
        0x011f_6000,
    ] {
        e.map(page, 0x1000);
    }
    e.mem.set_u64(0x0101_7cf8, 0.001f64.to_bits());
    e.mem.set_u64(0x0102_41b0, f64::from(f32::MIN).to_bits());
    e.mem.set_u64(0x0102_0998, 5.0f64.to_bits());
    e.mem.set_u64(0x0101_de30, 0.75f64.to_bits());
    // The unit axes, the havok scale, the fixed translation.
    words(e, 0x011a_946c, &[ONE, 0, 0, 0, ONE, 0, 0, 0, ONE]);
    e.mem.set_u32(0x011c_582c, 0x3e12_4925);
    words(e, 0x011f_474c, &[ONE, TWO, HALF]);
    // The null bound.
    words(e, 0x011f_4288, &[ONE, TWO, HALF, TWO]);
    // The calendar and the player.
    words(e, 0x011e_0358, &[0x4140_0000, 0x7c2, 0xff]);
    e.mem.set_u32(0x011a_39c4, 7);
    // The imports.
    e.register(0x00fd_f1c0, |e, a| {
        let n = e.mem.u32(a[0]).wrapping_add(1);
        e.mem.set_u32(a[0], n);
        ret(n)
    });
    e.register(0x00fd_f1c4, |e, a| {
        let n = e.mem.u32(a[0]).wrapping_sub(1);
        e.mem.set_u32(a[0], n);
        ret(n)
    });
    e.register(0x00fd_f1bc, |e, a| {
        let old = e.mem.u32(a[0]);
        if old == a[2] {
            e.mem.set_u32(a[0], a[1]);
        }
        ret(old)
    });
    e.register(0x00fd_f1d0, |_, _| ret(0x1234));
    // The C runtime's thread data (the float routines' error path), as the pathing test has it.
    e.register(0x00ed_1165, |_, _| ret(0));
    // The arrays' vtable in the exe's data, each slot the stand-in for it.
    e.map(0x0106_b000, 0x1000);
    for slot in (0..0x20).step_by(4) {
        e.mem.set_u32(0x0106_b1a4 + slot, VT_FN + slot);
    }
    e.register(0x0040_fca0, |e, a| {
        // The other owner gives the lock back after a sleep; with `LOG + 0x3ff4` set, only after a
        // `Sleep(1)`, which comes after 10,000 tries (the calls compare in the harness's own log).
        if e.mem.u32(LOG + 0x3ff4) == 0 || a[0] == 1 {
            log(e, 0xfca0, a[0]);
            let lock = e.mem.u32(LOG + 0x3ff0);
            e.mem.set_u32(lock, 0);
        }
        ret(0)
    });
    // The square root (its tables aren't mapped here).
    e.register(0x00ec_6054, |_, a| {
        ret_f(f64::from_bits(u64::from(a[0]) | u64::from(a[1]) << 32).sqrt())
    });
    // The list walk: the node at the position, the position moved on.
    e.register(0x0057_cbe0, |e, a| {
        let node = e.mem.u32(a[1]);
        e.mem.set_u32(a[1], e.mem.u32(node + 4));
        log(e, 0xcbe0, node);
        ret(node)
    });
    // The frustum: its constructor, its set-up from a camera, the camera's stamp.
    e.register(0x0045_c620, |e, a| {
        log(e, 0xc620, a[0]);
        ret(a[0])
    });
    e.register(0x00a7_55d0, |e, a| {
        log(e, 0x55d0, a[1]);
        // Six planes whose constant comes from the camera: a bound far on one side of the third.
        for i in 0..6u32 {
            let c = if i == 2 {
                e.mem.u32(a[1] + 4)
            } else {
                0xc2c8_0000
            };
            words(e, a[0] + 16 * i, &[ONE, 0, 0, c]);
        }
        ret(0)
    });
    e.register(0x0082_5c00, |e, a| ret(e.mem.u32(a[0] + 4)));
    // The inventory: a reference's container, a container's count, an extra list's reference.
    e.register(0x0055_d310, |e, a| ret(e.mem.u32(a[0] + 0x20)));
    e.register(
        0x0048_2a90,
        |e, a| ret(e.mem.u32(a[0]) ^ a[1] & 0x8000_0003),
    );
    e.register(0x0041_c8d0, |e, a| ret(e.mem.u32(a[0] + 8)));
    e.register(0x0071_7e50, |e, a| ret(e.mem.u32(a[0] + 0x10)));
    // An actor's water height, parent cell, the cell's water and the actor's height.
    e.register(0x0057_b0a0, |e, a| {
        ret_f(f64::from(f32::from_bits(e.mem.u32(a[0] + 0x150))))
    });
    e.register(0x008d_6f30, |e, a| ret(e.mem.u32(a[0] + 0x40)));
    e.register(0x0045_18e0, |e, a| ret(e.mem.u32(a[0]) | 0x100));
    e.register(0x0088_53a0, |e, a| {
        ret_f(f64::from(f32::from_bits(e.mem.u32(a[0] + 0x154))))
    });
    e.register(0x0045_8b20, |_, _| ret(0x011f_426c));
    e.register(0x00fd_f428, |e, a| {
        log(e, 0x6dda, e.mem.u32(a[2] + 0x30));
        ret(a[0])
    });
    // The arrays' element destructors (their buffer free and set-up are the pathing family's).
    e.register(0x0072_ba80, |e, a| {
        log(e, 0xba80, a[2]);
        ret(0)
    });
    // A vtable's slots: each logs its slot, `this` and its words, and answers by slot.
    let slots: [(u32, AbiFn); 13] = [
        (4, slot::<4>),
        (8, slot::<8>),
        (0x90, slot::<0x90>),
        (0x94, slot::<0x94>),
        (0xa4, slot::<0xa4>),
        (0xa8, slot::<0xa8>),
        (0xac, slot::<0xac>),
        (0xb8, slot::<0xb8>),
        (0xc0, slot::<0xc0>),
        (0x100, slot::<0x100>),
        (0x1b8, slot::<0x1b8>),
        (0x1f4, slot::<0x1f4>),
        (0x420, slot::<0x420>),
    ];
    for s in (0..0x440).step_by(4) {
        e.register(VT_FN + s, any_slot);
    }
    for (s, f) in slots {
        e.register(VT_FN + s, f);
    }
}

/// The stand-in for any other slot: logs `this`.
fn any_slot(e: &mut Engine, a: &[u32]) -> Ret {
    log(e, 0xffff, a.first().copied().unwrap_or(0));
    ret(0)
}

/// The stand-in for vtable slot `S`.
fn slot<const S: u32>(e: &mut Engine, a: &[u32]) -> Ret {
    log(e, S, a[0]);
    for w in &a[1..] {
        log(e, S | 0x8000, *w);
    }
    match S {
        // The RTTI: a chain in the object's word at +0x40.
        8 => ret(e.mem.u32(a[0] + 0x40)),
        // An actor's position.
        0x1f4 => ret(a[0] + 0x30),
        0x100 => ret(e.mem.u32(a[0] + 0x44) & 1),
        0x420 => ret_f(1.5),
        _ => ret(0),
    }
}

/// A vtable whose every slot is the logging stand-in for it.
fn vtable(e: &mut Engine) -> u32 {
    let vt = e.mem.alloc(0x440);
    for slot in (0..0x440).step_by(4) {
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

/// A list of `items`, the node of the first.
fn list(e: &mut Engine, items: &[u32]) -> u32 {
    let mut next = 0;
    for &item in items.iter().rev() {
        next = block(e, 8, &[item, next]);
    }
    next
}

/// A node (0xb0 bytes) with flags, `children` children (one of them null) with bounds whose radius
/// comes from `radius`, two controllers and a property list with one property with controllers.
fn node(e: &mut Engine, flags: u32, children: u32, radius: u32) -> u32 {
    let n = object(e, 0xb0);
    e.mem.set_u32(n + 0x30, flags);
    let bound = block(e, 0x10, &[ONE, TWO, HALF, THIRD]);
    e.mem.set_u32(n + 0x20, bound);
    let c2 = object(e, 0x40);
    let c1 = object(e, 0x40);
    e.mem.set_u32(c1 + 0x30, c2);
    e.mem.set_u32(n + 0xc, c1);
    let property = object(e, 0x40);
    let pc = object(e, 0x40);
    e.mem.set_u32(property + 0xc, pc);
    let other = object(e, 0x40);
    let props = list(e, &[property, other, 0]);
    e.mem.set_u32(n + 0x24, props);
    let base = e.mem.alloc(4 * children.max(1));
    for i in 0..children {
        let child = if i == 1 {
            0
        } else {
            let c = object(e, 0xb0);
            e.mem
                .set_u32(c + 0x30, [2, 0x12, 0x16, 0][(i & 3) as usize]);
            let r = if i == 0 { radius } else { TWO };
            let b = block(e, 0x10, &[TWO, ONE, HALF, r]);
            e.mem.set_u32(c + 0x20, b);
            c
        };
        e.mem.set_u32(base + 4 * i, child);
    }
    let vt = vtable(e);
    e.mem.set_u32(n + 0x9c, vt);
    e.mem.set_u32(n + 0xa0, base);
    e.mem.set_u16(n + 0xa4, children as u16);
    e.mem.set_u16(n + 0xa6, children as u16);
    e.mem.set_u16(n + 0xa8, children.saturating_sub(1) as u16);
    n
}

/// Update data: the flags at +4 and +5.
fn data(e: &mut Engine, case: &Case, i: usize) -> u32 {
    block(e, 8, &[ONE, case.word(i) & 0xffff])
}

/// A transform of 13 floats: the case's at `i` (scale) and `i + 1` (first entry), the rest from a
/// table.
fn transform(e: &mut Engine, case: &Case, i: usize) -> u32 {
    let table = [
        ONE, HALF, 0, THIRD, ONE, MINUS_ONE, 0, TWO, ONE, TWO, MINUS_ONE, HALF,
    ];
    let t = e.mem.alloc(0x34);
    words(e, t + 4, &table[1..9]);
    words(e, t + 0x24, &table[9..12]);
    e.mem.set_u32(t, bits(case, i + 1));
    e.mem.set_u32(t + 0x30, bits(case, i));
    t
}

/// A point of three case floats from `i`.
fn point(e: &mut Engine, case: &Case, i: usize) -> u32 {
    block(
        e,
        0xc,
        &[bits(case, i), bits(case, i + 1), bits(case, i + 2)],
    )
}

/// Three floats: one from the operand classes, two swept.
const P3: [Input; 3] = [Input::F32, Input::F32Swept(ONE), Input::F32Swept(MINUS_ONE)];

fn inputs(parts: &[&[Input]]) -> Vec<Input> {
    parts.iter().flat_map(|p| p.iter().copied()).collect()
}

/// The callees the translations a test reaches call plainly (a Rust call, not logged), by the
/// function under test: the typed side's calls to them are left out of its log.
const PLAIN: &[(u32, &[u32])] = &[
    (0x0047_68c0, &[0x0047_6930, 0x0047_6980, 0x0047_69c0]),
    (0x0047_6980, &[0x0047_69c0]),
    (0x004b_4880, &[0x0047_6930, 0x0047_6980, 0x0047_69c0]),
    (0x00c4_c2d0, &[0x0047_6930, 0x0047_6980, 0x0047_69c0]),
    (
        0x0043_b4f0,
        &[
            0x0043_b560,
            0x0043_b540,
            0x0043_b5b0,
            0x0043_b5f0,
            0x0043_b5d0,
        ],
    ),
    (0x0043_b560, &[0x0043_b5b0, 0x0043_b5f0, 0x0043_b5d0]),
    (0x0043_b5b0, &[0x0043_b5d0]),
    (0x0043_b5f0, &[0x0043_b5d0]),
    (0x004e_ff50, &[0x004f_0000]),
    // The scene family: what reaches the controllers or `NiNode`'s pass.
    (0x00a5_9f90, &[0x004f_0000]),
    (0x00b4_ded0, &[0x004f_0000]),
    (0x00c1_ad90, &[0x004f_0000]),
    (0x00a5_9c60, &[0x004f_0000]),
    (0x00a5_dd70, &[0x004f_0000]),
    (0x00a5_ded0, &[0x004f_0000]),
    (0x004b_5fc0, &[0x004b_5ff0, 0x004b_6100]),
    (0x004b_5ff0, &[0x004b_6100]),
    (0x004c_8f30, &[0x004b_ffb0, 0x004b_fba0]),
    (0x0065_3270, &[0x0065_3290]),
    (0x0043_b610, &[0x0065_3290]),
    (0x0056_0e10, &[0x0056_0e50]),
    (0x004a_3e00, &[0x004a_3e90]),
];

/// What every test adds to its comparison: the families, the plain callees, the call budget.
/// The culling family's functions other families call (by `typed_call!` or a vtable slot): the plane
/// side, the world transform and data, the frustum's set-up, the bounds' merge and updates, the
/// geometry's world bound, the box's frustum test and the compound frustum's camera.
const CULLING_CALLED: &[u32] = &[
    0x004b_6100,
    0x00a6_8bf0,
    0x00a6_8c60,
    0x00a6_94a0,
    0x00a7_f3f0,
    0x00a7_fdd0,
    0x00a7_fea0,
    0x00a8_0710,
    0x00c3_87f0,
    0x00c4_7980,
];

trait Common {
    fn common(self, function: u32) -> Diff;
    /// [`Common::common`] with the culling family's functions by address, to their translations on
    /// both sides. Its maths is native: what also runs off the main thread rounds a sum of products
    /// once where the exe's 24-bit code rounds each step, so on the main thread's control word it parts
    /// from the exe in the last bits (the culling family's own tests compare it there, within their
    /// tolerance). The scene family's maths, compared on that word, runs it translated.
    fn culling_translated(self, function: u32) -> Diff;
}

impl Common for Diff {
    fn common(self, function: u32) -> Diff {
        families(self.family(&culling::CULLING), function)
    }

    fn culling_translated(self, function: u32) -> Diff {
        families(self.goes_by_address(CULLING_CALLED), function)
    }
}

/// [`Common`]'s families but culling, the plain callees of `function`'s translation, and the rest.
fn families(d: Diff, function: u32) -> Diff {
    let mut callees = vec![0x0040_19d0, 0x0040_8860];
    if let Some((_, more)) = PLAIN.iter().find(|(f, _)| *f == function) {
        callees.extend_from_slice(more);
    }
    callees.retain(|c| *c != function);
    d.family(&dispatch::DISPATCH)
        .family(&engine_typed::scene::SCENE)
        .family(&pathing::PATHING)
        .family(&leaf::LEAVES)
        .translation_calls_plainly(&callees)
        // The draw family's `D3DXMatrixInverse` thunk, which the 4x4 inverse calls.
        .goes_by_address(&[0x00ee_6dda])
        .max_calls(MAX_CALLS)
        .float_sweep(FLOAT_SWEEP)
}

// ---- Native float maths ----
//
// The family's maths is native (ADR-0014's float rule; `engine_typed::dispatch`, "Floats"): `f64`
// locals rounded where the exe stores a float, and a single operation on two floats in plain `f32`,
// which rounds to the same float. Under the other threads' control word (53-bit precision) that is the
// exe's arithmetic bit for bit, so those comparisons take every input. Under the main thread's (24-bit
// precision, with the x87's wider exponent) the exe rounds every operation to 24 bits: a single
// operation gives the float `f32` gives while the result is a normal float, and a sum of products (the
// matrix and vector products) can differ in its last bits, by more ulps where it cancels to near zero.
// Where a function's result differs there, its main-thread comparison takes the floats play has
// ([`play`], [`unit`]) and says its tolerance; with `--nocapture` it prints what it let through. A
// function that gives the exe's floats under both words for every input is compared under both.

/// `bits` with its magnitude folded into the binades `low` to `low + span` (sign and mantissa kept);
/// zeros and infinities as they come; a NaN the quiet NaN 0x7fc00000.
fn fold(bits: u32, low: u32, span: u32) -> u32 {
    let exponent = (bits >> 23) & 0xff;
    if exponent == 0xff && bits & 0x007f_ffff != 0 {
        return 0x7fc0_0000;
    }
    if exponent == 0xff || bits & 0x7fff_ffff == 0 || (low..low + span).contains(&exponent) {
        return bits;
    }
    (bits & 0x807f_ffff) | ((low + exponent % span) << 23)
}

/// A float as play has it: its magnitude in 2^-12 to 2^12, so that no product of two leaves the
/// float's normal range (where the x87 keeps a wider exponent); a NaN the quiet NaN (play has no
/// signalling NaNs or payloads: the exe quiets a signalling NaN it copies through the x87 stack, native
/// code copies it as it is).
fn play(bits: u32) -> u32 {
    fold(bits, 115, 24)
}

/// A float as a rotation's entry, a scale or a short vector's component has it in play: its
/// magnitude in 2^-4 to 4 (otherwise as [`play`]).
fn unit(bits: u32) -> u32 {
    fold(bits, 123, 6)
}

/// The case with its floats folded by `f`, its words as they came.
fn folded(case: &Case, f: fn(u32) -> u32) -> Case {
    Case(
        case.0
            .iter()
            .map(|v| match *v {
                Value::F32(b) => Value::F32(f(b)),
                other => other,
            })
            .collect(),
    )
}

/// How the native maths is compared with its translation.
trait Native {
    /// Under the other threads' control word: bit for bit, NaN payloads aside (x87 and SSE pick a
    /// result NaN's payload differently, and the exe quiets a signalling NaN it copies).
    fn other_threads(self) -> Diff;
    /// Under the main thread's: floats within `ulps`, or bit for bit (NaN payloads aside) with 0.
    fn main_thread(self, ulps: u32) -> Diff;
}

impl Native for Diff {
    fn other_threads(self) -> Diff {
        self.control_words(&[OTHER_THREADS]).allow_nan_payloads()
    }

    fn main_thread(self, ulps: u32) -> Diff {
        let d = self.control_words(&[MAIN_THREAD]);
        if ulps == 0 {
            d.allow_nan_payloads()
        } else {
            d.float_ulps(ulps)
        }
    }
}

// ---- The arithmetic ----

/// `compose`'s words: two transforms of the case's floats, the first's translation x and the
/// second's y the case's too.
fn compose_words(e: &mut Engine, case: &Case) -> Vec<u32> {
    stand_ins(e);
    let a = transform(e, case, 0);
    let b = transform(e, case, 3);
    e.mem.set_u32(a + 0x24, bits(case, 2));
    e.mem.set_u32(b + 0x28, bits(case, 5));
    vec![a, e.mem.alloc(0x34), b]
}

#[test]
fn compose() {
    // Under the other threads' control word bit for bit, every input.
    Diff::new(entry!(
        0x0062_c250,
        math::compose(Ptr<_>, Ptr<_>, Ptr<_>) -> Ptr<_>
    ))
    .common(0x0062_c250)
    .other_threads()
    .inputs(&inputs(&[&P3, &P3]))
    .max_cases(128)
    .setup(compose_words)
    .run()
    .assert_same();
    // Under the main thread's, the case's floats in 2^-4 to 4 ([`unit`]): the rotation's entries and
    // the rotated translation are sums of three products, which the exe rounds at each step, at most
    // 4 * 2^-24 of the products' sizes (below 48 here) from the native sum; the translation scales
    // that by up to 4. Within 4 ulps, or 2^-13 in value.
    let report = Diff::new(entry!(
        0x0062_c250,
        math::compose(Ptr<_>, Ptr<_>, Ptr<_>) -> Ptr<_>
    ))
    .common(0x0062_c250)
    .main_thread(4)
    .float_abs(2f32.powi(-13))
    .inputs(&inputs(&[&P3, &P3]))
    .max_cases(128)
    .setup(|e, case| compose_words(e, &folded(case, unit)))
    .run();
    println!("compose on the main thread: {report}");
    report.assert_same();
}

/// `invert_non_uniform`'s words: a transform of the case's floats, its translation y the case's too.
fn invert_words(e: &mut Engine, case: &Case) -> Vec<u32> {
    stand_ins(e);
    let t = transform(e, case, 0);
    e.mem.set_u32(t + 0x28, bits(case, 2));
    vec![t, e.mem.alloc(0x34)]
}

#[test]
fn invert_non_uniform() {
    // Under the other threads' control word bit for bit, every input.
    Diff::new(entry!(
        0x004b_4880,
        math::invert_non_uniform(Ptr<_>, Ptr<_>)
    ))
    .common(0x004b_4880)
    .other_threads()
    .inputs(&P3)
    .setup(invert_words)
    .run()
    .assert_same();
    // Under the main thread's, in 2^-4 to 4 ([`unit`]): the translation is the rotated vector (sums
    // of three products, as in `rotate`) times the inverse scale, up to 16. Within 4 ulps, or 2^-12 in
    // value.
    let report = Diff::new(entry!(
        0x004b_4880,
        math::invert_non_uniform(Ptr<_>, Ptr<_>)
    ))
    .common(0x004b_4880)
    .main_thread(4)
    .float_abs(2f32.powi(-12))
    .inputs(&P3)
    .setup(|e, case| invert_words(e, &folded(case, unit)))
    .run();
    println!("invert_non_uniform on the main thread: {report}");
    report.assert_same();
}

/// `rotate`'s words: a matrix and a vector of the case's floats.
fn rotate_words(e: &mut Engine, case: &Case) -> Vec<u32> {
    stand_ins(e);
    let m = transform(e, case, 0);
    e.mem.set_u32(m + 0x14, bits(case, 2));
    vec![m, e.mem.alloc(0xc), point(e, case, 3)]
}

#[test]
fn rotate() {
    // Under the other threads' control word bit for bit, every input.
    Diff::new(entry!(
        0x004b_4500,
        math::rotate(Ptr<_>, Ptr<_>, Ptr<_>) -> Ptr<_>
    ))
    .common(0x004b_4500)
    .other_threads()
    .inputs(&inputs(&[&P3, &P3]))
    .max_cases(128)
    .setup(rotate_words)
    .run()
    .assert_same();
    // Under the main thread's, in 2^-4 to 4 ([`unit`]): each component a sum of three products the
    // exe rounds at each step, at most 4 * 2^-24 of the products' sizes (below 48) from the native
    // sum. Within 4 ulps, or 2^-16 in value.
    let report = Diff::new(entry!(
        0x004b_4500,
        math::rotate(Ptr<_>, Ptr<_>, Ptr<_>) -> Ptr<_>
    ))
    .common(0x004b_4500)
    .main_thread(4)
    .float_abs(2f32.powi(-16))
    .inputs(&inputs(&[&P3, &P3]))
    .max_cases(128)
    .setup(|e, case| rotate_words(e, &folded(case, unit)))
    .run();
    println!("rotate on the main thread: {report}");
    report.assert_same();
}

/// `multiply`'s words: two matrices of the case's floats.
fn multiply_words(e: &mut Engine, case: &Case) -> Vec<u32> {
    stand_ins(e);
    let a = transform(e, case, 0);
    let b = transform(e, case, 3);
    e.mem.set_u32(a + 0x20, bits(case, 2));
    e.mem.set_u32(b + 0x10, bits(case, 5));
    vec![a, e.mem.alloc(0x24), b]
}

#[test]
fn multiply() {
    // Under the other threads' control word bit for bit, every input.
    Diff::new(entry!(
        0x0043_f8d0,
        math::multiply(Ptr<_>, Ptr<_>, Ptr<_>) -> Ptr<_>
    ))
    .common(0x0043_f8d0)
    .other_threads()
    .inputs(&inputs(&[&P3, &P3]))
    .max_cases(128)
    .setup(multiply_words)
    .run()
    .assert_same();
    // Under the main thread's, in 2^-4 to 4 ([`unit`]): each entry as `rotate`'s components. Within 4
    // ulps, or 2^-16 in value.
    let report = Diff::new(entry!(
        0x0043_f8d0,
        math::multiply(Ptr<_>, Ptr<_>, Ptr<_>) -> Ptr<_>
    ))
    .common(0x0043_f8d0)
    .main_thread(4)
    .float_abs(2f32.powi(-16))
    .inputs(&inputs(&[&P3, &P3]))
    .max_cases(128)
    .setup(|e, case| multiply_words(e, &folded(case, unit)))
    .run();
    println!("multiply on the main thread: {report}");
    report.assert_same();
}

#[test]
fn transpose() {
    // Copies only: bit for bit under both control words, NaN payloads aside.
    Diff::new(entry!(
        0x0047_68c0,
        math::transpose(Ptr<_>, Ptr<_>) -> Ptr<_>
    ))
    .common(0x0047_68c0)
    .allow_nan_payloads()
    .inputs(&P3)
    .setup(|e, case| {
        stand_ins(e);
        let m = transform(e, case, 0);
        e.mem.set_u32(m + 0x1c, bits(case, 2));
        vec![m, e.mem.alloc(0x24)]
    })
    .run()
    .assert_same();
}

#[test]
fn row() {
    Diff::new(entry!(0x0047_6930, math::row(Ptr<_>, u32, Ptr<_>)))
        .common(0x0047_6930)
        .allow_nan_payloads()
        .inputs(&inputs(&[&P3, &[Input::Of(&[0, 1, 2])]]))
        .setup(|e, case| {
            stand_ins(e);
            let m = transform(e, case, 0);
            e.mem.set_u32(m + 0x18, bits(case, 2));
            vec![m, case.word(3), e.mem.alloc(0xc)]
        })
        .run()
        .assert_same();
}

#[test]
fn set_columns() {
    Diff::new(entry!(
        0x0047_6980,
        math::set_columns(Ptr<_>, Ptr<_>, Ptr<_>, Ptr<_>) -> Ptr<_>
    ))
    .common(0x0047_6980)
    .allow_nan_payloads()
    .inputs(&P3)
    .setup(|e, case| {
        stand_ins(e);
        let a = point(e, case, 0);
        let b = block(e, 0xc, &[bits(case, 2), ONE, bits(case, 0)]);
        vec![e.mem.alloc(0x24), a, b, a]
    })
    .run()
    .assert_same();
}

#[test]
fn set_column() {
    Diff::new(entry!(0x0047_69c0, math::set_column(Ptr<_>, u32, Ptr<_>)))
        .common(0x0047_69c0)
        .allow_nan_payloads()
        .inputs(&inputs(&[&P3, &[Input::Of(&[0, 1, 2])]]))
        .setup(|e, case| {
            stand_ins(e);
            let p = point(e, case, 0);
            vec![e.mem.alloc(0x24), case.word(3), p]
        })
        .run()
        .assert_same();
}

#[test]
fn transform_new() {
    Diff::new(entry!(0x0047_6a80, math::transform_new(Ptr<_>) -> Ptr<_>))
        .common(0x0047_6a80)
        .inputs(&[Input::Flag])
        .setup(|e, _| {
            stand_ins(e);
            vec![e.mem.alloc(0x34)]
        })
        .run()
        .assert_same();
}

/// `times`' words: a point and a scale of the case's floats.
fn times_words(e: &mut Engine, case: &Case) -> Vec<u32> {
    stand_ins(e);
    let p = point(e, case, 0);
    vec![p, e.mem.alloc(0xc), bits(case, 3)]
}

#[test]
fn times() {
    // Under the other threads' control word bit for bit, every input.
    Diff::new(entry!(
        0x0045_bb20,
        math::times(Ptr<_>, Ptr<_>, f32) -> Ptr<_>
    ))
    .common(0x0045_bb20)
    .other_threads()
    .inputs(&inputs(&[&P3, &[Input::F32]]))
    .max_cases(128)
    .setup(times_words)
    .run()
    .assert_same();
    // Under the main thread's, as play has the floats ([`play`]): each product of two floats rounds as
    // the exe's while it is a normal float (below that the exe rounds to 24 bits first, then to the
    // float). Bit for bit.
    Diff::new(entry!(
        0x0045_bb20,
        math::times(Ptr<_>, Ptr<_>, f32) -> Ptr<_>
    ))
    .common(0x0045_bb20)
    .main_thread(0)
    .inputs(&inputs(&[&P3, &[Input::F32]]))
    .max_cases(128)
    .setup(|e, case| times_words(e, &folded(case, play)))
    .run()
    .assert_same();
}

/// `normalize`'s words: a point of the case's floats, by the case's word 3 as it came, or scaled by
/// 2^-14 (the ordinary cases' (x, 1, -1) then has a length below the minimum, 0.001, which zeroes it)
/// or by 2^-9 (just above it).
fn normalize_words(e: &mut Engine, case: &Case) -> Vec<u32> {
    stand_ins(e);
    let p = point(e, case, 0);
    let scale = match case.word(3) {
        1 => 2f32.powi(-14),
        2 => 2f32.powi(-9),
        _ => return vec![p],
    };
    for i in 0..3 {
        let v = f32::from_bits(e.mem.u32(p + 4 * i)) * scale;
        e.mem.set_u32(p + 4 * i, v.to_bits());
    }
    vec![p]
}

/// `normalize`'s inputs: three floats and the scale of [`normalize_words`].
fn normalize_inputs() -> Vec<Input> {
    inputs(&[&P3, &[Input::Of(&[0, 1, 2])]])
}

#[test]
fn normalize() {
    // Under the other threads' control word bit for bit, every input.
    Diff::new(entry!(0x004a_0c10, math::normalize(Ptr<_>)))
        .common(0x004a_0c10)
        .other_threads()
        .inputs(&normalize_inputs())
        .setup(normalize_words)
        .run()
        .assert_same();
    // Under the main thread's, as play has the floats ([`play`]): the inverse length and each
    // product round as the exe's (normal floats). Bit for bit.
    Diff::new(entry!(0x004a_0c10, math::normalize(Ptr<_>)))
        .common(0x004a_0c10)
        .main_thread(0)
        .inputs(&normalize_inputs())
        .setup(|e, case| normalize_words(e, &folded(case, play)))
        .run()
        .assert_same();
}

#[test]
fn box_corners() {
    // The half extents times the unit axes (exact) and the corners' sums (the pathing family's):
    // bit for bit under both control words, NaN payloads aside.
    Diff::new(entry!(0x00c3_9270, math::box_corners(Ptr<_>, Ptr<_>)))
        .common(0x00c3_9270)
        .allow_nan_payloads()
        .inputs(&inputs(&[&P3, &P3]))
        .max_cases(128)
        .setup(|e, case| {
            stand_ins(e);
            let b = e.mem.alloc(0x24);
            words(e, b + 0xc, &[bits(case, 0), bits(case, 1), bits(case, 2)]);
            words(e, b + 0x18, &[bits(case, 3), bits(case, 4), bits(case, 5)]);
            vec![b, e.mem.alloc(0x60)]
        })
        .run()
        .assert_same();
}

/// `inverse_4x4`'s words: a transform of the case's floats, its translation z the case's too, and
/// whether the inverse is the fixed one.
fn inverse_4x4_words(e: &mut Engine, case: &Case) -> Vec<u32> {
    stand_ins(e);
    let t = transform(e, case, 0);
    e.mem.set_u32(t + 0x2c, bits(case, 2));
    vec![t, case.word(3), e.mem.alloc(0x40)]
}

#[test]
fn inverse_4x4() {
    // Under the other threads' control word bit for bit, every input.
    Diff::new(entry!(0x00c4_c2d0, math::inverse_4x4(Ptr<_>, u32, Ptr<_>)))
        .common(0x00c4_c2d0)
        .other_threads()
        .inputs(&inputs(&[&P3, &[Input::Of(&[0, 1])]]))
        .setup(inverse_4x4_words)
        .run()
        .assert_same();
    // Under the main thread's, in 2^-4 to 4 ([`unit`]): the inverse's translation as
    // `invert_non_uniform`'s; each rotation entry times the inverse scale rounds as the exe's. Within
    // 4 ulps, or 2^-12 in value.
    let report = Diff::new(entry!(0x00c4_c2d0, math::inverse_4x4(Ptr<_>, u32, Ptr<_>)))
        .common(0x00c4_c2d0)
        .main_thread(4)
        .float_abs(2f32.powi(-12))
        .inputs(&inputs(&[&P3, &[Input::Of(&[0, 1])]]))
        .setup(|e, case| inverse_4x4_words(e, &folded(case, unit)))
        .run();
    println!("inverse_4x4 on the main thread: {report}");
    report.assert_same();
}

/// `havok_vector`'s words: a point of the case's floats.
fn havok_vector_words(e: &mut Engine, case: &Case) -> Vec<u32> {
    stand_ins(e);
    let p = point(e, case, 0);
    vec![e.mem.alloc(0x10), p]
}

#[test]
fn havok_vector() {
    // Under the other threads' control word bit for bit, every input.
    Diff::new(entry!(
        0x004a_3e00,
        math::havok_vector(Ptr<_>, Ptr<_>) -> Ptr<_>
    ))
    .common(0x004a_3e00)
    .other_threads()
    .inputs(&P3)
    .setup(havok_vector_words)
    .run()
    .assert_same();
    // Under the main thread's, as play has the floats ([`play`]): each product rounds as the exe's.
    // Bit for bit.
    Diff::new(entry!(
        0x004a_3e00,
        math::havok_vector(Ptr<_>, Ptr<_>) -> Ptr<_>
    ))
    .common(0x004a_3e00)
    .main_thread(0)
    .inputs(&P3)
    .setup(|e, case| havok_vector_words(e, &folded(case, play)))
    .run()
    .assert_same();
}

/// `havok_scale`'s words: the scale (at `011c582c`) and the value, the case's floats.
fn havok_scale_words(e: &mut Engine, case: &Case) -> Vec<u32> {
    stand_ins(e);
    e.mem.set_u32(0x011c_582c, bits(case, 1));
    vec![bits(case, 0)]
}

#[test]
fn havok_scale() {
    // Under the other threads' control word bit for bit, every input.
    Diff::new(entry!(0x004a_3e90, math::havok_scale(f32) -> f32))
        .common(0x004a_3e90)
        .other_threads()
        .inputs(&[Input::F32, Input::F32Swept(0x3e12_4925)])
        .setup(havok_scale_words)
        .run()
        .assert_same();
    // Under the main thread's, as play has the floats ([`play`]): the product rounds as the exe's.
    // Bit for bit.
    Diff::new(entry!(0x004a_3e90, math::havok_scale(f32) -> f32))
        .common(0x004a_3e90)
        .main_thread(0)
        .inputs(&[Input::F32, Input::F32Swept(0x3e12_4925)])
        .setup(|e, case| havok_scale_words(e, &folded(case, play)))
        .run()
        .assert_same();
}

#[test]
fn lane() {
    Diff::new(entry!(0x0056_0d30, math::lane(Ptr<_>, u32) -> Ptr<_>))
        .common(0x0056_0d30)
        .inputs(&[Input::Word, Input::Word])
        .run()
        .assert_same();
}

#[test]
fn copy_16() {
    Diff::new(entry!(0x004a_3c90, math::copy_16(Ptr<_>, Ptr<_>) -> Ptr<_>))
        .common(0x004a_3c90)
        .inputs(&[Input::F32, Input::Word])
        .setup(|e, case| {
            let s = block(e, 0x10, &[bits(case, 0), case.word(1), 0xffff_ffff, 7]);
            vec![e.mem.alloc(0x10), s]
        })
        .run()
        .assert_same();
}

#[test]
fn point2_new() {
    Diff::new(entry!(
        0x0045_2dc0,
        math::point2_new(Ptr<_>, f32, f32) -> Ptr<_>
    ))
    .common(0x0045_2dc0)
    .allow_nan_payloads()
    .inputs(&[Input::F32, Input::F32])
    .setup(|e, case| vec![e.mem.alloc(8), bits(case, 0), bits(case, 1)])
    .run()
    .assert_same();
}

#[test]
fn points_close() {
    // A difference of two floats rounds as the exe's under both control words (one that falls below
    // the normal range is exact): bit for bit under both, NaN payloads aside.
    Diff::new(entry!(
        0x0049_e2f0,
        math::points_close(Ptr<_>, Ptr<_>, f32) -> bool
    ))
    .common(0x0049_e2f0)
    .allow_nan_payloads()
    .inputs(&inputs(&[&P3, &[Input::F32, Input::F32Swept(HALF)]]))
    .max_cases(128)
    .setup(|e, case| {
        stand_ins(e);
        let a = point(e, case, 0);
        let b = block(e, 0xc, &[bits(case, 3), bits(case, 1), MINUS_ONE]);
        vec![a, b, bits(case, 4)]
    })
    .run()
    .assert_same();
}

#[test]
fn floats_close() {
    // As `points_close`: bit for bit under both control words, NaN payloads aside.
    Diff::new(entry!(0x0049_e390, math::floats_close(f32, f32, f32) -> bool))
        .common(0x0049_e390)
        .allow_nan_payloads()
        .inputs(&[Input::F32, Input::F32Swept(ONE), Input::F32])
        .max_cases(128)
        .setup(|e, case| {
            stand_ins(e);
            vec![bits(case, 0), bits(case, 1), bits(case, 2)]
        })
        .run()
        .assert_same();
}

// ---- The scene graph ----

#[test]
fn first_controller() {
    Diff::new(entry!(
        0x0043_b230,
        scene::first_controller(Ptr<_>) -> Ptr<_>
    ))
    .common(0x0043_b230)
    .inputs(&[Input::Word])
    .setup(|e, case| vec![block(e, 0x40, &[0, 0, 0, case.word(0)])])
    .run()
    .assert_same();
}

#[test]
fn next_controller() {
    Diff::new(entry!(0x004a_8a90, scene::next_controller(Ptr<_>) -> u32))
        .common(0x004a_8a90)
        .inputs(&[Input::Word])
        .setup(|e, case| {
            let c = e.mem.alloc(0x40);
            e.mem.set_u32(c + 0x30, case.word(0));
            vec![c]
        })
        .run()
        .assert_same();
}

/// Flags for the controller and child tests: with and without 0x40000, bits 0 to 4, 0xd and 0x13.
const NODE_FLAGS: &[u32] = &[
    0x0004_0000,
    0,
    0x0008_0004,
    0x0008_2005,
    0x0004_001f,
    0x0008_0002,
];

#[test]
fn update_object_controllers() {
    Diff::new(entry!(
        0x004e_ff50,
        scene::update_object_controllers(Ptr<_>, Ptr<_>, u8)
    ))
    .common(0x004e_ff50)
    .inputs(&[Input::Of(NODE_FLAGS), Input::Flag])
    .setup(|e, case| {
        stand_ins(e);
        let n = node(e, case.word(0), 0, ONE);
        let d = block(e, 8, &[ONE, 0]);
        vec![n, d, case.word(1)]
    })
    .run()
    .assert_same();
}

#[test]
fn has_property_controllers() {
    Diff::new(entry!(
        0x004f_0000,
        scene::has_property_controllers(Ptr<_>) -> bool
    ))
    .common(0x004f_0000)
    .inputs(&[Input::Of(NODE_FLAGS)])
    .setup(|e, case| {
        let n = block(e, 0x40, &[0; 16]);
        e.mem.set_u32(n + 0x30, case.word(0));
        vec![n]
    })
    .run()
    .assert_same();
}

#[test]
fn child_count() {
    Diff::new(entry!(0x0043_b480, scene::child_count(Ptr<_>) -> u16))
        .common(0x0043_b480)
        .inputs(&[Input::Of(&[0, 1, 3, 0xffff])])
        .setup(|e, case| {
            stand_ins(e);
            let n = node(e, 0, 0, ONE);
            e.mem.set_u16(n + 0xa6, case.word(0) as u16);
            vec![n]
        })
        .run()
        .assert_same();
}

#[test]
fn child() {
    Diff::new(entry!(0x0043_b4a0, scene::child(Ptr<_>, u32) -> Ptr<_>))
        .common(0x0043_b4a0)
        .inputs(&[Input::Of(&[0, 1, 2])])
        .setup(|e, case| {
            stand_ins(e);
            vec![node(e, 0, 3, ONE), case.word(0)]
        })
        .run()
        .assert_same();
}

#[test]
fn array_size() {
    Diff::new(entry!(0x0065_8930, scene::array_size(Ptr<_>) -> u16))
        .common(0x0065_8930)
        .inputs(&[Input::Word])
        .setup(|e, case| vec![block(e, 0x10, &[0, 0, case.word(0) << 16])])
        .run()
        .assert_same();
}

/// The children and radius inputs of the downward passes.
const PASS: [Input; 4] = [
    Input::Of(NODE_FLAGS),
    Input::Of(&[3, 0, 1, 4]),
    Input::Of(&[ONE, 0, 0x7fc0_0000, 0x8000_0000]),
    Input::Of(&[0x0001, 0, 0x0100, 0x0101]),
];

#[test]
fn update_downward() {
    Diff::new(entry!(
        0x00a5_dd70,
        scene::update_downward(Ptr<_>, u32, u32)
    ))
    .common(0x00a5_dd70)
    .inputs(&inputs(&[&PASS, &[Input::Of(&[0, 1, 6])]]))
    .setup(|e, case| {
        stand_ins(e);
        let n = node(e, case.word(0), case.word(1), case.word(2));
        vec![n, data(e, case, 3), case.word(4)]
    })
    .run()
    .assert_same();
}

/// `update_downward` with the node's slot 0xb8 holding the typed `UpdateWorldData` (`00a68c60`) and each child's
/// slot 0xa4 holding the typed `UpdateDownwardPass` (`00a5dd70`): both calls made direct (the plain test has
/// stand-ins there).
#[test]
fn update_downward_typed_slots() {
    Diff::new(entry!(
        0x00a5_dd70,
        scene::update_downward(Ptr<_>, u32, u32)
    ))
    .common(0x00a5_dd70)
    .inputs(&inputs(&[&PASS, &[Input::Of(&[0, 1, 6])]]))
    .setup(|e, case| {
        stand_ins(e);
        let n = node(e, case.word(0), case.word(1), case.word(2));
        let vt = e.mem.u32(n);
        e.mem.set_u32(vt + 0xb8, 0x00a6_8c60);
        let base = e.mem.u32(n + 0xa0);
        for i in 0..case.word(1) {
            let child = e.mem.u32(base + 4 * i);
            if child != 0 {
                let vt = e.mem.u32(child);
                e.mem.set_u32(vt + 0xa4, 0x00a5_dd70);
                e.mem.set_u32(vt + 0xb8, 0x00a6_8c60);
            }
        }
        vec![n, data(e, case, 3), case.word(4)]
    })
    .run()
    .assert_same();
}

#[test]
fn update_selected_downward() {
    Diff::new(entry!(
        0x00a5_ded0,
        scene::update_selected_downward(Ptr<_>, u32, u32)
    ))
    .common(0x00a5_ded0)
    .inputs(&inputs(&[&PASS, &[Input::Of(&[0, 1, 6])]]))
    .setup(|e, case| {
        stand_ins(e);
        let n = node(e, case.word(0) | 8, case.word(1), case.word(2));
        vec![n, data(e, case, 3), case.word(4)]
    })
    .run()
    .assert_same();
}

#[test]
fn update_transform_and_bounds() {
    Diff::new(entry!(
        0x00a5_e0b0,
        scene::update_transform_and_bounds(Ptr<_>, u32)
    ))
    .common(0x00a5_e0b0)
    .inputs(&PASS)
    .setup(|e, case| {
        stand_ins(e);
        let n = node(e, case.word(0), case.word(1), case.word(2));
        vec![n, data(e, case, 3)]
    })
    .run()
    .assert_same();
}

/// A renderer: mode at +0x200, immediate flag at +0x208.
fn renderer(e: &mut Engine, case: &Case) -> u32 {
    let r = object(e, 0x210);
    e.mem.set_u32(r + 0x200, case.word(0));
    e.mem.set_u32(r + 0x208, case.word(1));
    r
}

#[test]
fn geometry_render_immediate() {
    Diff::new(entry!(
        0x00a8_0610,
        scene::geometry_render_immediate(Ptr<_>, Ptr<_>)
    ))
    .common(0x00a8_0610)
    .inputs(&[Input::Of(&[1, 2, 0]), Input::Flag])
    .setup(|e, case| {
        stand_ins(e);
        let n = node(e, 0, 0, ONE);
        vec![n, renderer(e, case)]
    })
    .run()
    .assert_same();
}

#[test]
fn tri_strips_render_immediate() {
    Diff::new(entry!(
        0x00a7_1d70,
        scene::tri_strips_render_immediate(Ptr<_>, Ptr<_>)
    ))
    .common(0x00a7_1d70)
    .inputs(&[Input::Of(&[1, 2, 0, 3]), Input::Of(&[1, 0, 0x101])])
    .setup(|e, case| {
        stand_ins(e);
        let n = node(e, 0, 0, ONE);
        vec![n, renderer(e, case)]
    })
    .run()
    .assert_same();
}

#[test]
fn reference_3d() {
    Diff::new(entry!(0x0043_fcd0, scene::reference_3d(Ptr<_>) -> Ptr<_>))
        .common(0x0043_fcd0)
        .inputs(&[Input::Of(&[1, 0, 2])])
        .setup(|e, case| {
            let r = e.mem.alloc(0x70);
            if case.word(0) == 1 {
                let loaded = block(e, 0x18, &[0, 0, 0, 0, 0, 0x0abc_0000]);
                e.mem.set_u32(r + 0x64, loaded);
            }
            let tls = e.tls();
            e.mem.set_u32(tls + 0x260, 0x0def_0000);
            if case.word(0) == 2 {
                e.mem.set_u32(tls + 0x264, r);
            }
            vec![r]
        })
        .run()
        .assert_same();
}

/// A model handle: a holder whose entry (+8) holds its value at +0x10, the value's node at +0x1c.
fn handle(e: &mut Engine, case: &Case) -> u32 {
    let entry = block(e, 0x80, &[0; 32]);
    e.mem.set_u32(entry + 0x2c, 0x0aaa_0000);
    block(e, 0x10, &[0, 0, if case.word(0) == 0 { 0 } else { entry }])
}

#[test]
fn loaded_3d() {
    Diff::new(entry!(
        0x0043_b4f0,
        scene::loaded_3d(Ptr<_>, Ptr<_>) -> Ptr<_>
    ))
    .common(0x0043_b4f0)
    .inputs(&[Input::Flag])
    .setup(|e, case| {
        let h = handle(e, case);
        vec![h, e.mem.alloc(4)]
    })
    .run()
    .assert_same();
}

#[test]
fn handle_3d() {
    Diff::new(entry!(0x0043_b540, scene::handle_3d(Ptr<_>) -> u32))
        .common(0x0043_b540)
        .inputs(&[Input::Word])
        .setup(|e, case| vec![block(e, 0x20, &[0, 0, 0, 0, 0, 0, 0, case.word(0)])])
        .run()
        .assert_same();
}

#[test]
fn loaded_handle() {
    Diff::new(entry!(0x0043_b560, scene::loaded_handle(Ptr<_>) -> u32))
        .common(0x0043_b560)
        .inputs(&[Input::Flag])
        .setup(|e, case| vec![handle(e, case)])
        .run()
        .assert_same();
}

#[test]
fn lock_loaded() {
    Diff::new(entry!(0x0043_b5b0, scene::lock_loaded(Ptr<_>)))
        .common(0x0043_b5b0)
        .inputs(&[Input::Word])
        .run()
        .assert_same();
}

#[test]
fn loaded_lock() {
    Diff::new(entry!(0x0043_b5d0, scene::loaded_lock(Ptr<_>) -> Ptr<_>))
        .common(0x0043_b5d0)
        .inputs(&[Input::Word])
        .run()
        .assert_same();
}

#[test]
fn unlock_loaded() {
    Diff::new(entry!(0x0043_b5f0, scene::unlock_loaded(Ptr<_>)))
        .common(0x0043_b5f0)
        .inputs(&[Input::Word])
        .run()
        .assert_same();
}

/// An object whose RTTI chain (slot 8, its word at +0x40) holds `rtti` or not, with a collision
/// object at +0x1c of the same kind.
fn kinded(e: &mut Engine, case: &Case, i: usize) -> u32 {
    let base = block(e, 8, &[0, if case.word(i) == 1 { RTTI } else { 0 }]);
    let own = block(e, 8, &[0, base]);
    let o = object(e, 0x48);
    e.mem
        .set_u32(o + 0x40, if case.word(i) == 2 { RTTI } else { own });
    o
}

#[test]
fn collision_object() {
    Diff::new(entry!(
        0x0043_b610,
        scene::collision_object(Ptr<_>) -> Ptr<_>
    ))
    .common(0x0043_b610)
    .inputs(&[Input::Of(&[1, 0, 2, 3])])
    .setup(|e, case| {
        stand_ins(e);
        let o = e.mem.alloc(0x20);
        let c = if case.word(0) == 3 {
            0
        } else {
            kinded(e, case, 0)
        };
        e.mem.set_u32(o + 0x1c, c);
        vec![o]
    })
    .run()
    .assert_same();
}

#[test]
fn collision_word() {
    Diff::new(entry!(0x0068_38b0, scene::collision_word(Ptr<_>) -> u32))
        .common(0x0068_38b0)
        .inputs(&[Input::Word])
        .setup(|e, case| vec![block(e, 0x20, &[0, 0, 0, 0, 0, 0, 0, case.word(0)])])
        .run()
        .assert_same();
}

#[test]
fn cast_if() {
    Diff::new(entry!(0x0065_3270, scene::cast_if(u32, u32) -> u32))
        .common(0x0065_3270)
        .inputs(&[Input::Of(&[1, 0, 2, 3])])
        .setup(|e, case| {
            stand_ins(e);
            let o = if case.word(0) == 3 {
                0
            } else {
                kinded(e, case, 0)
            };
            vec![RTTI, o]
        })
        .run()
        .assert_same();
}

#[test]
fn cast() {
    Diff::new(entry!(0x0065_3290, scene::cast(Ptr<_>, u32) -> Ptr<_>))
        .common(0x0065_3290)
        .inputs(&[Input::Of(&[1, 0, 2])])
        .setup(|e, case| {
            stand_ins(e);
            vec![kinded(e, case, 0), RTTI]
        })
        .run()
        .assert_same();
}

#[test]
fn is_kind_of() {
    Diff::new(entry!(0x0065_32c0, scene::is_kind_of(Ptr<_>, u32) -> bool))
        .common(0x0065_32c0)
        .inputs(&[Input::Of(&[1, 0, 2])])
        .setup(|e, case| {
            stand_ins(e);
            vec![kinded(e, case, 0), RTTI]
        })
        .run()
        .assert_same();
}

#[test]
fn esize_of_children() {
    Diff::new(entry!(0x0045_3470, scene::esize_of_children(Ptr<_>) -> u32))
        .common(0x0045_3470)
        .inputs(&[Input::Of(&[0, 1, 3, 4])])
        .setup(|e, case| {
            stand_ins(e);
            vec![node(e, 0, case.word(0), ONE)]
        })
        .run()
        .assert_same();
}

#[test]
fn esize() {
    Diff::new(entry!(0x0099_38b0, scene::esize(Ptr<_>) -> u16))
        .common(0x0099_38b0)
        .inputs(&[Input::Word])
        .setup(|e, case| vec![block(e, 0x10, &[0, 0, 0, case.word(0)])])
        .run()
        .assert_same();
}

/// A camera at a given stamp; the frustum kept for it or another.
fn camera(e: &mut Engine, case: &Case) -> u32 {
    let c = block(e, 0x10, &[0, case.f32(0).to_bits()]);
    let owner = 0x011f_6394;
    e.mem.set_u32(owner + 4, 5);
    e.mem.set_u32(0x011c_6394, case.word(1));
    if case.word(1) & 1 != 0 {
        e.mem
            .set_u32(0x011c_632c, if case.word(2) == 0 { c } else { 0x1234 });
        e.mem.set_u32(0x0118_90e0, case.word(2) & 5);
        for i in 0..6u32 {
            words(e, 0x011c_6330 + 16 * i, &[ONE, 0, 0, MINUS_ONE]);
        }
    }
    c
}

/// A bound from case floats: centre x and radius.
fn bound(e: &mut Engine, case: &Case) -> u32 {
    block(e, 0x10, &[bits(case, 3), ONE, HALF, bits(case, 4)])
}

/// The frustum inputs: the camera's constant, the guard, the stamp, the bound's x and radius.
const FRUSTUM: [Input; 5] = [
    Input::F32Swept(0xc120_0000),
    Input::Of(&[1, 0]),
    Input::Of(&[0, 5, 1]),
    Input::F32,
    Input::F32Swept(TWO),
];

#[test]
fn in_frustum() {
    Diff::new(entry!(
        0x004b_5ff0,
        scene::in_frustum(Ptr<_>, Ptr<_>) -> bool
    ))
    .common(0x004b_5ff0)
    .inputs(&inputs(&[&FRUSTUM, &[Input::Flag]]))
    .setup(|e, case| {
        stand_ins(e);
        let c = if case.word(5) == 1 {
            camera(e, case)
        } else {
            0
        };
        vec![bound(e, case), c]
    })
    .run()
    .assert_same();
}

#[test]
fn bound_in_frustum() {
    Diff::new(entry!(
        0x004b_5fc0,
        scene::bound_in_frustum(Ptr<_>, Ptr<_>) -> bool
    ))
    .common(0x004b_5fc0)
    .inputs(&inputs(&[&FRUSTUM, &[Input::Flag]]))
    .setup(|e, case| {
        stand_ins(e);
        let o = if case.word(5) == 1 {
            let n = node(e, 0, 0, ONE);
            let b = bound(e, case);
            e.mem.set_u32(n + 0x20, b);
            n
        } else {
            0
        };
        vec![o, camera(e, case)]
    })
    .run()
    .assert_same();
}

#[test]
fn plane() {
    Diff::new(entry!(0x004b_4cf0, scene::plane(Ptr<_>, u32) -> Ptr<_>))
        .common(0x004b_4cf0)
        .inputs(&[Input::Word, Input::Word])
        .run()
        .assert_same();
}

#[test]
fn word_8() {
    Diff::new(entry!(0x0062_0b80, scene::word_8(u32) -> u32))
        .common(0x0062_0b80)
        .inputs(&[Input::Word])
        .setup(|e, case| vec![block(e, 0xc, &[0, 0, case.word(0)])])
        .run()
        .assert_same();
}

#[test]
fn character_proxy() {
    Diff::new(entry!(0x004a_e750, scene::character_proxy(Ptr<_>) -> u32))
        .common(0x004a_e750)
        .inputs(&[Input::Word, Input::Flag])
        .setup(|e, case| {
            let o = block(e, 0xc, &[0, 0, case.word(0)]);
            vec![if case.word(1) == 1 { o } else { 0 }]
        })
        .run()
        .assert_same();
}

#[test]
fn body_vector() {
    Diff::new(entry!(0x0056_0e10, scene::body_vector(Ptr<_>) -> Ptr<_>))
        .common(0x0056_0e10)
        .inputs(&[Input::Flag])
        .setup(|e, case| {
            stand_ins(e);
            let entity = e.mem.alloc(0x200);
            vec![block(
                e,
                0xc,
                &[0, 0, if case.word(0) == 1 { entity } else { 0 }],
            )]
        })
        .run()
        .assert_same();
}

#[test]
fn body_vector_of() {
    Diff::new(entry!(0x0056_0e50, scene::body_vector_of(Ptr<_>) -> Ptr<_>))
        .common(0x0056_0e50)
        .inputs(&[Input::Word])
        .run()
        .assert_same();
}

#[test]
fn part_e0() {
    Diff::new(entry!(0x009d_9f40, scene::part_e0(Ptr<_>) -> Ptr<_>))
        .common(0x009d_9f40)
        .inputs(&[Input::Word])
        .run()
        .assert_same();
}

// ---- References, locks, lists and getters ----

#[test]
fn increment() {
    Diff::new(entry!(0x0040_b460, refs::increment(Ptr<_>) -> i32))
        .common(0x0040_b460)
        .inputs(&[Input::Word])
        .setup(|e, case| {
            stand_ins(e);
            vec![block(e, 4, &[case.word(0)])]
        })
        .run()
        .assert_same();
}

#[test]
fn decrement() {
    Diff::new(entry!(0x0040_19a0, refs::decrement(Ptr<_>) -> i32))
        .common(0x0040_19a0)
        .inputs(&[Input::Word])
        .setup(|e, case| {
            stand_ins(e);
            vec![block(e, 4, &[case.word(0)])]
        })
        .run()
        .assert_same();
}

#[test]
fn add_reference() {
    Diff::new(entry!(0x0040_f6e0, refs::add_reference(Ptr<_>)))
        .common(0x0040_f6e0)
        .inputs(&[Input::Word])
        .setup(|e, case| {
            stand_ins(e);
            let o = object(e, 8);
            e.mem.set_u32(o + 4, case.word(0));
            vec![o]
        })
        .run()
        .assert_same();
}

#[test]
fn release() {
    Diff::new(entry!(0x0040_1970, refs::release(Ptr<_>)))
        .common(0x0040_1970)
        .inputs(&[Input::Word])
        .setup(|e, case| {
            stand_ins(e);
            let o = object(e, 8);
            e.mem.set_u32(o + 4, case.word(0));
            vec![o]
        })
        .run()
        .assert_same();
}

#[test]
fn release_held() {
    Diff::new(entry!(0x0045_cec0, refs::release_held(Ptr<_>)))
        .common(0x0045_cec0)
        .inputs(&[Input::Of(&[1, 2, 0])])
        .setup(|e, case| {
            stand_ins(e);
            let o = object(e, 8);
            e.mem.set_u32(o + 4, case.word(0));
            vec![block(e, 4, &[if case.word(0) == 0 { 0 } else { o }])]
        })
        .run()
        .assert_same();
}

#[test]
fn ref_object_new() {
    Diff::new(entry!(0x0049_68b0, refs::ref_object_new(Ptr<_>) -> Ptr<_>))
        .common(0x0049_68b0)
        .inputs(&[Input::Word])
        .setup(|e, case| {
            stand_ins(e);
            e.mem.set_u32(0x011f_4400, case.word(0));
            vec![block(e, 8, &[0xdead, 0xbeef])]
        })
        .run()
        .assert_same();
}

#[test]
fn ref_object_drop() {
    Diff::new(entry!(0x0049_6910, refs::ref_object_drop(Ptr<_>)))
        .common(0x0049_6910)
        .inputs(&[Input::Word])
        .setup(|e, case| {
            stand_ins(e);
            e.mem.set_u32(0x011f_4400, case.word(0));
            vec![block(e, 8, &[0xdead, 0xbeef])]
        })
        .run()
        .assert_same();
}

#[test]
fn current_thread() {
    Diff::new(entry!(0x0040_fc90, refs::current_thread() -> u32))
        .common(0x0040_fc90)
        .inputs(&[])
        .setup(|e, _| {
            stand_ins(e);
            vec![]
        })
        .run()
        .assert_same();
}

#[test]
fn compare_exchange() {
    Diff::new(entry!(
        0x0043_b460,
        refs::compare_exchange(Ptr<_>, i32, i32) -> i32
    ))
    .common(0x0043_b460)
    .inputs(&[Input::Of(&[0, 5]), Input::Of(&[0, 5]), Input::Word])
    .setup(|e, case| {
        stand_ins(e);
        vec![block(e, 4, &[case.word(0)]), case.word(1), case.word(2)]
    })
    .run()
    .assert_same();
}

#[test]
fn lock() {
    Diff::new(entry!(0x0040_fbf0, refs::lock(Ptr<_>, u32)))
        .common(0x0040_fbf0)
        .inputs(&[Input::Of(&[0x1234, 0, 0x99]), Input::Word])
        .setup(|e, case| {
            stand_ins(e);
            let l = block(e, 8, &[case.word(0), case.word(1)]);
            // `Sleep` lets the other owner go at once (the long wait: `lock_sleeps_a_millisecond_after_ten_thousand_tries`).
            e.mem.set_u32(LOG + 0x3ff0, l);
            vec![l, 0]
        })
        .run()
        .assert_same();
}

#[test]
fn unlock() {
    Diff::new(entry!(0x0040_fba0, refs::unlock(Ptr<_>)))
        .common(0x0040_fba0)
        .inputs(&[Input::Word])
        .setup(|e, case| vec![block(e, 8, &[0x1234, case.word(0)])])
        .run()
        .assert_same();
}

#[test]
fn nothing_2() {
    Diff::new(entry!(0x0045_34f0, refs::nothing_2(u32, u32)))
        .common(0x0045_34f0)
        .inputs(&[Input::Word, Input::Word])
        .run()
        .assert_same();
}

#[test]
fn nothing_1() {
    Diff::new(entry!(0x0048_3710, refs::nothing_1(u32)))
        .common(0x0048_3710)
        .inputs(&[Input::Word])
        .run()
        .assert_same();
}

#[test]
fn at_10() {
    Diff::new(entry!(0x0046_0140, refs::at_10(Ptr<_>) -> Ptr<_>))
        .common(0x0046_0140)
        .inputs(&[Input::Word])
        .run()
        .assert_same();
}

#[test]
fn store_word() {
    Diff::new(entry!(0x008c_71b0, refs::store_word(Ptr<_>, u32) -> Ptr<_>))
        .common(0x008c_71b0)
        .inputs(&[Input::Word])
        .setup(|e, case| vec![e.mem.alloc(4), case.word(0)])
        .run()
        .assert_same();
}

/// A list of three entries whose first words are 7, `form` and 9 (the middle one null when `hole`),
/// each with a word at +4 and +8.
fn entries(e: &mut Engine, form: u32, hole: bool) -> u32 {
    let a = block(e, 0xc, &[7, 0xffff_fffe, 0x70]);
    let b = block(e, 0xc, &[form, 3, 0x80]);
    let c = block(e, 0xc, &[9, 0, 0x90]);
    list(e, &[a, if hole { 0 } else { b }, c])
}

#[test]
fn reaction() {
    Diff::new(entry!(0x0048_c1b0, refs::reaction(Ptr<_>, u32) -> u32))
        .common(0x0048_c1b0)
        .inputs(&[Input::Of(&[0x55, 0, 9, 0x66]), Input::Flag])
        .setup(|e, case| {
            stand_ins(e);
            let l = entries(e, 0x55, case.word(1) == 0);
            vec![block(e, 0x14, &[0, 0, 0, 0, l]), case.word(0)]
        })
        .run()
        .assert_same();
}

#[test]
fn detection_state() {
    Diff::new(entry!(
        0x008f_6650,
        refs::detection_state(Ptr<_>, u32, i32) -> Ptr<_>
    ))
    .common(0x008f_6650)
    .inputs(&[
        Input::Of(&[0x55, 7, 9, 0x66]),
        Input::Of(&[0, 1]),
        Input::Flag,
    ])
    .setup(|e, case| {
        stand_ins(e);
        let p = e.mem.alloc(0x264);
        let l = entries(e, 0x55, case.word(2) == 0);
        let m = entries(e, 0x66, false);
        e.mem.set_u32(p + 0x25c, l);
        e.mem.set_u32(p + 0x260, m);
        vec![p, case.word(0), case.word(1)]
    })
    .run()
    .assert_same();
}

#[test]
fn array_item() {
    Diff::new(entry!(0x0096_8670, refs::array_item(Ptr<_>, u32) -> u32))
        .common(0x0096_8670)
        .inputs(&[Input::Of(&[0, 1, 2])])
        .setup(|e, case| {
            let data = block(e, 0xc, &[0x11, 0x22, 0x33]);
            vec![block(e, 0x10, &[0, data, 3, 3]), case.word(0)]
        })
        .run()
        .assert_same();
}

/// Inventory changes: their list of entries (one for `0x55` with a count and extra lists whose
/// references have ids), their reference with a container whose count depends on the form.
fn inventory(e: &mut Engine, case: &Case) -> u32 {
    let reference = block(e, 0x10, &[0, 0, 0, 0x4242]);
    let other = block(e, 0x10, &[0, 0, 0, 0x4343]);
    let x1 = block(e, 0xc, &[0, 0, other]);
    let x2 = block(e, 0xc, &[0, 0, reference]);
    let x0 = block(e, 0xc, &[0, 0, 0]);
    let extras = list(e, &[x0, x1, 0, x2]);
    let entry = block(e, 0xc, &[extras, case.word(1), 0x55]);
    let first = block(e, 0xc, &[0, 2, 0x66]);
    let changes = list(e, &[first, 0, entry]);
    let container = block(e, 4, &[case.word(2)]);
    let owner = block(e, 0x24, &[0; 9]);
    e.mem.set_u32(owner + 0x20, container);
    block(
        e,
        0x14,
        &[changes, if case.word(3) == 1 { owner } else { 0 }],
    )
}

const INVENTORY: [Input; 4] = [
    Input::Of(&[0x55, 0x66, 0x77]),
    Input::Of(&[0, 3, 0xffff_fffe]),
    Input::Of(&[0, 0x55, 0xffff_ffaa]),
    Input::Flag,
];

#[test]
fn item_count() {
    Diff::new(entry!(0x004c_8f30, refs::item_count(Ptr<_>, u32) -> i32))
        .common(0x004c_8f30)
        .inputs(&INVENTORY)
        .setup(|e, case| {
            stand_ins(e);
            vec![inventory(e, case), case.word(0)]
        })
        .run()
        .assert_same();
}

#[test]
fn container_of() {
    Diff::new(entry!(0x004b_ffb0, refs::container_of(Ptr<_>) -> u32))
        .common(0x004b_ffb0)
        .inputs(&INVENTORY)
        .setup(|e, case| {
            stand_ins(e);
            vec![inventory(e, case)]
        })
        .run()
        .assert_same();
}

#[test]
fn entry_for() {
    Diff::new(entry!(
        0x004b_fba0,
        refs::entry_for(Ptr<_>, u32, u32, u32) -> u32
    ))
    .common(0x004b_fba0)
    .inputs(&inputs(&[
        &INVENTORY,
        &[Input::Of(&[0, 0x4242, 0x4343, 0x9999])],
    ]))
    .setup(|e, case| {
        stand_ins(e);
        vec![inventory(e, case), case.word(0), 1, case.word(4)]
    })
    .run()
    .assert_same();
}

/// An actor: the flag at +0x14d, the water height (+0x150) and its height (+0x154), its position,
/// its cell (+0x40) and whether the cell has water.
fn actor(e: &mut Engine, case: &Case) -> u32 {
    let a = object(e, 0x160);
    e.mem.set_u8(a + 0x14d, case.word(0) as u8);
    e.mem.set_u32(a + 0x150, bits(case, 1));
    e.mem.set_u32(a + 0x154, bits(case, 2));
    words(e, a + 0x30, &[ONE, TWO, bits(case, 3)]);
    let cell = block(e, 4, &[case.word(4)]);
    e.mem
        .set_u32(a + 0x40, if case.word(4) == 2 { 0 } else { cell });
    e.mem.set_u32(a + 0x108, case.word(5));
    e.mem.set_u32(0x011d_f880, case.word(6));
    a
}

const ACTOR: [Input; 7] = [
    Input::Of(&[1, 0, 0x100]),
    Input::F32,
    Input::F32Swept(0x42c8_0000),
    Input::F32Swept(0x4248_0000),
    Input::Of(&[1, 0, 2]),
    Input::Of(&[2, 1, 6, 0]),
    Input::Of(&[0, 3, 4, 0xffff_ffff]),
];

/// `location_on_reference`'s words: an actor of the case's floats.
fn location_words(e: &mut Engine, case: &Case) -> Vec<u32> {
    stand_ins(e);
    vec![actor(e, case)]
}

#[test]
fn location_on_reference() {
    // Under the other threads' control word bit for bit, every input.
    Diff::new(entry!(
        0x008a_e4c0,
        refs::location_on_reference(Ptr<_>) -> u32
    ))
    .common(0x008a_e4c0)
    .other_threads()
    .inputs(&ACTOR)
    .max_cases(128)
    .setup(location_words)
    .run()
    .assert_same();
    // Under the main thread's, as play has the floats ([`play`]): the ceiling and the rise are a
    // `double` step rounded to a float, which native code rounds twice (to 53 bits, then to 24) where
    // the exe rounds once: one ulp where the first rounding lands exactly halfway; the raised height
    // is the exe's from those. Within 1 ulp.
    let report = Diff::new(entry!(
        0x008a_e4c0,
        refs::location_on_reference(Ptr<_>) -> u32
    ))
    .common(0x008a_e4c0)
    .main_thread(1)
    .inputs(&ACTOR)
    .max_cases(128)
    .setup(|e, case| location_words(e, &folded(case, play)))
    .run();
    println!("location_on_reference on the main thread: {report}");
    report.assert_same();
}

#[test]
fn position() {
    Diff::new(entry!(0x0043_6aa0, refs::position(Ptr<_>) -> Ptr<_>))
        .common(0x0043_6aa0)
        .inputs(&[Input::Word])
        .run()
        .assert_same();
}

#[test]
fn byte_14d() {
    Diff::new(entry!(0x005a_2030, refs::byte_14d(Ptr<_>) -> u8))
        .common(0x005a_2030)
        .inputs(&ACTOR)
        .max_cases(16)
        .setup(|e, case| {
            stand_ins(e);
            vec![actor(e, case)]
        })
        .run()
        .assert_same();
}

#[test]
fn in_state() {
    Diff::new(entry!(0x0088_44f0, refs::in_state(Ptr<_>, bool) -> bool))
        .common(0x0088_44f0)
        .inputs(&inputs(&[&ACTOR, &[Input::Flag]]))
        .max_cases(32)
        .setup(|e, case| {
            stand_ins(e);
            vec![actor(e, case), case.word(7)]
        })
        .run()
        .assert_same();
}

#[test]
fn word_108() {
    Diff::new(entry!(0x004f_8960, refs::word_108(Ptr<_>) -> u32))
        .common(0x004f_8960)
        .inputs(&ACTOR)
        .max_cases(16)
        .setup(|e, case| {
            stand_ins(e);
            vec![actor(e, case)]
        })
        .run()
        .assert_same();
}

#[test]
fn compute_last_time_processed() {
    Diff::new(entry!(
        0x0090_7650,
        refs::compute_last_time_processed(Ptr<_>)
    ))
    .common(0x0090_7650)
    .allow_nan_payloads()
    .inputs(&[Input::F32, Input::Of(&[0x7f, 0x80, 0xff, 1])])
    .setup(|e, case| {
        stand_ins(e);
        e.mem.set_u32(0x011e_0358, bits(case, 0));
        e.mem.set_u8(0x011e_0360, case.word(1) as u8);
        vec![e.mem.alloc(0x28)]
    })
    .run()
    .assert_same();
}

#[test]
fn set_hour() {
    Diff::new(entry!(0x0069_3d50, refs::set_hour(Ptr<_>, f32)))
        .common(0x0069_3d50)
        .allow_nan_payloads()
        .inputs(&[Input::F32])
        .setup(|e, case| vec![e.mem.alloc(0x28), bits(case, 0)])
        .run()
        .assert_same();
}

#[test]
fn set_date() {
    Diff::new(entry!(0x0070_37c0, refs::set_date(Ptr<_>, u32)))
        .common(0x0070_37c0)
        .inputs(&[Input::Word])
        .setup(|e, case| vec![e.mem.alloc(0x28), case.word(0)])
        .run()
        .assert_same();
}

#[test]
fn compass_order() {
    Diff::new(entry!(
        0x0077_b300,
        refs::compass_order(Ptr<_>, Ptr<_>) -> i32
    ))
    .common(0x0077_b300)
    .inputs(&inputs(&[&P3, &[Input::F32, Input::Of(&[1, 0, 2, 3])]]))
    .max_cases(128)
    .setup(|e, case| {
        stand_ins(e);
        let actor_at = |e: &mut Engine, p: [u32; 3]| {
            let a = object(e, 0x40);
            words(e, a + 0x30, &p);
            a
        };
        let player = actor_at(e, [0, 0, 0]);
        e.mem.set_u32(0x011d_ea3c, player);
        let first = actor_at(e, [bits(case, 0), bits(case, 1), bits(case, 2)]);
        let second = actor_at(e, [bits(case, 3), bits(case, 1), MINUS_ONE]);
        let s1 = block(e, 4, &[if case.word(4) == 2 { 0 } else { first }]);
        let s2 = block(e, 4, &[second]);
        match case.word(4) {
            0 => vec![0, s2],
            3 => vec![s1, 0],
            _ => vec![s1, s2],
        }
    })
    .run()
    .assert_same();
}

#[test]
fn object_factor() {
    Diff::new(entry!(0x0086_a050, refs::object_factor(u32) -> f64))
        .common(0x0086_a050)
        .inputs(&[Input::Of(&[1, 0, 2])])
        .setup(|e, case| {
            stand_ins(e);
            let o = object(e, 0x48);
            e.mem.set_u32(o + 0x44, case.word(0));
            let s = e.mem.alloc(0xd0);
            e.mem
                .set_u32(s + 0xcc, if case.word(0) == 2 { 0 } else { o });
            vec![s]
        })
        .run()
        .assert_same();
}

#[test]
fn word_cc() {
    Diff::new(entry!(0x009a_d610, refs::word_cc(Ptr<_>) -> Ptr<_>))
        .common(0x009a_d610)
        .inputs(&[Input::Word])
        .setup(|e, case| {
            let s = e.mem.alloc(0xd0);
            e.mem.set_u32(s + 0xcc, case.word(0));
            vec![s]
        })
        .run()
        .assert_same();
}

#[test]
fn array_clear() {
    Diff::new(entry!(0x0084_54f0, refs::array_clear(Ptr<_>, u8)))
        .common(0x0084_54f0)
        .inputs(&[Input::Of(&[0x40, 0]), Input::Of(&[1, 0, 0x100])])
        .setup(|e, case| {
            stand_ins(e);
            vec![
                block(e, 0x10, &[0x0106_b1a4, case.word(0), 3, 8]),
                case.word(1),
            ]
        })
        .run()
        .assert_same();
}

#[test]
fn order_array_new() {
    Diff::new(entry!(0x0069_aee0, refs::order_array_new(Ptr<_>) -> Ptr<_>))
        .common(0x0069_aee0)
        .inputs(&[Input::Flag])
        .setup(|e, _| {
            stand_ins(e);
            vec![e.mem.alloc(0x10)]
        })
        .run()
        .assert_same();
}

#[test]
fn info_array_drop() {
    Diff::new(entry!(0x0069_af10, refs::info_array_drop(Ptr<_>) -> Ptr<_>))
        .common(0x0069_af10)
        .inputs(&[Input::Of(&[0x40, 0])])
        .setup(|e, case| {
            stand_ins(e);
            vec![block(e, 0x10, &[0, case.word(0), 3, 8])]
        })
        .run()
        .assert_same();
}

#[test]
fn extra_kind() {
    Diff::new(entry!(0x004f_1540, refs::extra_kind(Ptr<_>) -> u8))
        .common(0x004f_1540)
        .inputs(&[Input::Word])
        .setup(|e, case| vec![block(e, 8, &[0, case.word(0)])])
        .run()
        .assert_same();
}

// ---- Fixed values at the main thread's 24-bit precision ----
//
// The differential tests compare the typed versions with their translations, so a reading of the exe
// both share would pass them. These check the three sums the batch's translation fixes are about
// against values worked out by hand from the x87's rules: under 24-bit precision each `FADD` and `FMUL`
// rounds to 24 bits (ties to even) before the next, under 53-bit to 53 bits, and `FSTP` to a float.
// The typed versions are native maths (`f64` locals rounded where the exe stores a float): they give
// the 53-bit results under both control words, one ulp from the exe's on the main thread here, where
// the float rule accepts it (ADR-0014). The engine has the translations only
// ([`Engine::new_untyped_for_test`]): with the family on, as it is by default, a call by address would
// reach the typed version.

/// 2^-24 as a float.
const TWO_POW_M24: u32 = 0x3380_0000;

/// Runs `f` under the control word `word` (the thread's, as the CRT keeps it), then back to 53 bits.
fn under(e: &mut Engine, word: u16, f: impl FnOnce(&mut Engine) -> u32) -> u32 {
    engine_core::units::crt::set_x87_control_word(e, word);
    let r = f(e);
    engine_core::units::crt::set_x87_control_word(e, 0x027f);
    r
}

/// `(1 + 2^-24) + 2^-24`: at 24 bits each sum is a tie that goes to the even 1.0; at 53 bits the sum
/// is exact, 1 + 2^-23, the float after 1.0.
const TIE_SUM: [(u16, u32); 2] = [(0x007f, ONE), (0x027f, ONE + 1)];

#[test]
fn multiply_at_24_bits_rounds_each_sum() {
    let mut e = Engine::new_untyped_for_test();
    stand_ins(&mut e);
    let a = block(
        &mut e,
        0x24,
        &[ONE, TWO_POW_M24, TWO_POW_M24, 0, 0, 0, 0, 0, 0],
    );
    let b = block(&mut e, 0x24, &[ONE, 0, 0, ONE, 0, 0, ONE, 0, 0]);
    for (word, expected) in TIE_SUM {
        let out = e.mem.alloc(0x24);
        let got = under(&mut e, word, |e| {
            e.call(0x0043_f8d0, &engine_core::args![a, out, b]);
            e.mem.u32(out)
        });
        assert_eq!(got, expected, "translation, control word {word:04x}");
        let out = e.mem.alloc(0x24);
        let got = under(&mut e, word, |e| {
            math::multiply(e, Ptr::new(a), Ptr::new(out), Ptr::new(b));
            e.mem.u32(out)
        });
        assert_eq!(got, ONE + 1, "typed, control word {word:04x}");
    }
}

#[test]
fn rotate_at_24_bits_rounds_each_sum() {
    let mut e = Engine::new_untyped_for_test();
    stand_ins(&mut e);
    let m = block(
        &mut e,
        0x24,
        &[ONE, TWO_POW_M24, TWO_POW_M24, 0, 0, 0, 0, 0, 0],
    );
    let v = block(&mut e, 0xc, &[ONE, ONE, ONE]);
    for (word, expected) in TIE_SUM {
        let out = e.mem.alloc(0xc);
        let got = under(&mut e, word, |e| {
            e.call(0x004b_4500, &engine_core::args![m, out, v]);
            e.mem.u32(out)
        });
        assert_eq!(got, expected, "translation, control word {word:04x}");
        let out = e.mem.alloc(0xc);
        let got = under(&mut e, word, |e| {
            math::rotate(e, Ptr::new(m), Ptr::new(out), Ptr::new(v));
            e.mem.u32(out)
        });
        assert_eq!(got, ONE + 1, "typed, control word {word:04x}");
    }
}

/// The products' sums in the exe's order, `(a0 b0 + a1 b1) + a2 b2`, under both control words: with the
/// row (2^31, -2^31, 1 + 2^-23) and the vector (1, 1, 1) the first sum cancels exactly to 0, so the
/// entry is 1 + 2^-23, where `a0 b0 + (a1 b1 + a2 b2)` rounds -2^31 + 1 + 2^-23 to 53 bits (a tie that
/// goes to the even -(2^31 - 1)) and gives 1.0. The matrix product's first entry is the same sum, with
/// the column (1, 1, 1).
#[test]
fn products_sum_in_the_exe_order() {
    let mut e = Engine::new_untyped_for_test();
    stand_ins(&mut e);
    let row = [0x4f00_0000, 0xcf00_0000, ONE + 1];
    let m = block(&mut e, 0x24, &[row[0], row[1], row[2], 0, 0, 0, 0, 0, 0]);
    let v = block(&mut e, 0xc, &[ONE, ONE, ONE]);
    let b = block(&mut e, 0x24, &[ONE, 0, 0, ONE, 0, 0, ONE, 0, 0]);
    for word in [0x007f, 0x027f] {
        let out = e.mem.alloc(0xc);
        let got = under(&mut e, word, |e| {
            e.call(0x004b_4500, &engine_core::args![m, out, v]);
            e.mem.u32(out)
        });
        assert_eq!(got, ONE + 1, "rotate, translation, control word {word:04x}");
        let out = e.mem.alloc(0xc);
        let got = under(&mut e, word, |e| {
            math::rotate(e, Ptr::new(m), Ptr::new(out), Ptr::new(v));
            e.mem.u32(out)
        });
        assert_eq!(got, ONE + 1, "rotate, typed, control word {word:04x}");
        let out = e.mem.alloc(0x24);
        let got = under(&mut e, word, |e| {
            e.call(0x0043_f8d0, &engine_core::args![m, out, b]);
            e.mem.u32(out)
        });
        assert_eq!(
            got,
            ONE + 1,
            "multiply, translation, control word {word:04x}"
        );
        let out = e.mem.alloc(0x24);
        let got = under(&mut e, word, |e| {
            math::multiply(e, Ptr::new(m), Ptr::new(out), Ptr::new(b));
            e.mem.u32(out)
        });
        assert_eq!(got, ONE + 1, "multiply, typed, control word {word:04x}");
    }
}

/// An actor's height `h = k * 2^-52` with `k = (2^54 + 2^30 + 1) / 3`, so `0.75 * h` is exactly
/// `1 + 2^-24 + 2^-54`. Under 24 bits `FMUL` rounds that up (above the tie) to 1 + 2^-23; under
/// 53 bits it rounds to 1 + 2^-24 (a quarter of a unit is dropped) and `FSTP` then breaks the tie to
/// the even 1.0. The typed version's `f64` product rounds as the 53-bit one: 1.0 under both words.
#[test]
fn location_on_reference_at_24_bits_rounds_the_rise_once() {
    let mut e = Engine::new_untyped_for_test();
    stand_ins(&mut e);
    e.register(0x0057_b0a0, |_, _| ret_f(1000.0));
    e.register(0x0088_53a0, |_, _| {
        let k = ((1u64 << 54) + (1 << 30) + 1) / 3;
        ret_f(k as f64 * f64::powi(2.0, -52))
    });
    e.register(0x0045_18e0, |_, _| ret(1));
    let actor = e.mem.alloc(0x160);
    e.mem.set_u8(actor + 0x14d, 1);
    let cell = e.mem.alloc(4);
    e.mem.set_u32(actor + 0x40, cell);
    for (word, expected) in [(0x007f, ONE + 1), (0x027f, ONE)] {
        let got = under(&mut e, word, |e| {
            let slot = e.call(0x008a_e4c0, &engine_core::args![actor]).u32();
            e.mem.u32(slot + 8)
        });
        assert_eq!(got, expected, "translation, control word {word:04x}");
        let got = under(&mut e, word, |e| {
            let slot = refs::location_on_reference(e, Ptr::new(actor));
            e.mem.u32(slot + 8)
        });
        assert_eq!(got, ONE, "typed, control word {word:04x}");
    }
}

/// The lock's long wait, which the differential test leaves out (10,000 tries, each call compared
/// with a hash of memory, is too slow there): an owner that lets go only at a `Sleep(1)` gets 10,000
/// `Sleep(0)`s, then one `Sleep(1)`, then the lock is this thread's with a count of 1. The translation
/// and the typed version, counted the same way, on an engine with the translations only
/// ([`Engine::new_untyped_for_test`]): with the family on, as it is by default, the call by address
/// would reach the typed version.
#[test]
fn lock_sleeps_a_millisecond_after_ten_thousand_tries() {
    fn run(typed: bool) -> (u32, u32, u32, u32) {
        let mut e = Engine::new_untyped_for_test();
        stand_ins(&mut e);
        // `Sleep`: counts by its argument; the owner lets go at a `Sleep(1)`.
        e.register(0x0040_fca0, |e, a| {
            let at = LOG + 0x3f00 + 4 * a[0].min(1);
            e.mem.set_u32(at, e.mem.u32(at) + 1);
            if a[0] == 1 {
                let lock = e.mem.u32(LOG + 0x3ff0);
                e.mem.set_u32(lock, 0);
            }
            ret(0)
        });
        let l = block(&mut e, 8, &[0x98, 5]);
        e.mem.set_u32(LOG + 0x3ff0, l);
        if typed {
            refs::lock(&mut e, Ptr::new(l), 0);
        } else {
            e.call(0x0040_fbf0, &engine_core::args![l, 0u32]);
        }
        let sleeps = (e.mem.u32(LOG + 0x3f00), e.mem.u32(LOG + 0x3f04));
        (sleeps.0, sleeps.1, e.mem.u32(l), e.mem.u32(l + 4))
    }
    let expected = (10_000, 1, 0x1234, 1);
    assert_eq!(run(false), expected, "translation");
    assert_eq!(run(true), expected, "typed");
}

// ---- The word map's lookup (the save's references map) ----

/// A word map (`NiTMapBase<UInt32, ..>`, 0x10 bytes and a word at +0x40 for the stand-ins' compare)
/// with vtable `vt`, `buckets` buckets and a node (next, key, value) for each of `keys` put at the
/// head of its key's bucket chain, as the map's insert does; the value of a key is key ^ 0x5a5a.
fn word_map(e: &mut Engine, vt: u32, buckets: u32, keys: &[u32]) -> u32 {
    let map = e.mem.alloc(0x44);
    let table = e.mem.alloc(4 * buckets.max(1));
    words(e, map, &[vt, buckets, table, keys.len() as u32]);
    for &k in keys {
        let at = table + 4 * (k % buckets.max(1));
        let head = e.mem.u32(at);
        let node = block(e, 0xc, &[head, k, k ^ 0x5a5a]);
        e.mem.set_u32(at, node);
    }
    map
}

const MAP_KEYS: &[u32] = &[0, 1, 5, 36, 37, 38, 74, 111, 0xffff_ffff];

/// The save's references map: slots 4 and 8 hold the word map's hash and compare (`006c6a60`,
/// `006b81d0`), both called directly; found or not, at the head, inside or at the end of a chain,
/// one bucket or 37.
#[test]
fn map_lookup_word_map() {
    Diff::new(entry!(
        0x0085_3130,
        refs::map_lookup(Ptr<_>, u32, Ptr<u32>) -> u8
    ))
    .common(0x0085_3130)
    .inputs(&[
        Input::Of(&[1, 5, 37]),
        Input::Of(&[0, 1, 37, 74, 111, 2, 0xffff_ffff]),
        Input::Of(&[0, 4, 9]),
    ])
    .setup(|e, case| {
        stand_ins(e);
        let vt = vtable(e);
        e.mem.set_u32(vt + 4, 0x006c_6a60);
        e.mem.set_u32(vt + 8, 0x006b_81d0);
        let n = case.word(2) as usize;
        let map = word_map(e, vt, case.word(0), &MAP_KEYS[..n]);
        vec![map, case.word(1), block(e, 4, &[0x1234_5678])]
    })
    .run()
    .assert_same();
}

/// Any other map: the hash and the compare by address through the vtable (the stand-ins answer
/// bucket 0, and "equal" by the map's word at +0x40), in the exe's order.
#[test]
fn map_lookup_other_slots() {
    Diff::new(entry!(
        0x0085_3130,
        refs::map_lookup(Ptr<_>, u32, Ptr<u32>) -> u8
    ))
    .common(0x0085_3130)
    .inputs(&[Input::Of(&[0, 1, 3]), Input::Flag, Input::Word])
    .setup(|e, case| {
        stand_ins(e);
        let vt = vtable(e);
        let keys: Vec<u32> = (0..case.word(0)).map(|i| 7 * i).collect();
        let map = word_map(e, vt, 1, &keys);
        e.mem.set_u32(map + 0x40, case.word(1));
        vec![map, case.word(2), block(e, 4, &[0x1234_5678])]
    })
    .run()
    .assert_same();
}

// ---- The scene family (engine_typed::scene) ----

/// A geometry (0xc0 bytes) with flags, a world bound, a local transform from the case's floats at
/// `f`, model data with a bound (+0xb8) and no skin.
fn geometry(e: &mut Engine, case: &Case, flags: u32, f: usize) -> u32 {
    let g = object(e, 0xc0);
    e.mem.set_u32(g + 0x30, flags);
    let bound = block(e, 0x10, &[ONE, TWO, HALF, THIRD]);
    e.mem.set_u32(g + 0x20, bound);
    let t = transform(e, case, f);
    let local = e.mem.bytes(t, 0x34);
    e.mem.write(g + 0x34, &local);
    let model = block(e, 0x20, &[0, 0, 0, 0, HALF, ONE, TWO, bits(case, f + 2)]);
    e.mem.set_u32(g + 0xb8, model);
    g
}

/// Slot `slot` of `object`'s vtable set to `target`.
fn set_slot(e: &mut Engine, object: u32, slot: u32, target: u32) {
    let vt = e.mem.u32(object);
    e.mem.set_u32(vt + slot, target);
}

/// The update data's flag words: controllers (+4), particles skipped (+5).
const DATA_FLAGS: &[u32] = &[0, 1, 0x100, 0x101];

#[test]
fn scene_object_update_downward() {
    // The object's flags, the data's, the world data and bound slots stand-ins or the typed ones.
    // Under the other threads' control word bit for bit, every input.
    Diff::new(entry!(
        0x00a5_9f90,
        engine_typed::scene::walk::object_update_downward(Ptr<_>, Ptr<_>, u32)
    ))
    .common(0x00a5_9f90)
    .other_threads()
    .inputs(&inputs(&[
        &[
            Input::Of(NODE_FLAGS),
            Input::Of(DATA_FLAGS),
            Input::Of(&[0, 1, 2, 3]),
        ],
        &P3,
    ]))
    .max_cases(256)
    .setup(|e, case| {
        stand_ins(e);
        let g = geometry(e, case, case.word(0), 3);
        if case.word(2) & 1 != 0 {
            set_slot(e, g, 0xb8, 0x00a6_8c60);
        }
        if case.word(2) & 2 != 0 {
            set_slot(e, g, 0xbc, 0x00a8_0710);
        }
        vec![g, block(e, 8, &[ONE, case.word(1)]), 0x55]
    })
    .run()
    .assert_same();
    // Under the main thread's with the culling family by address ([`Common::culling_translated`]): bit for bit.
    let report = Diff::new(entry!(
        0x00a5_9f90,
        engine_typed::scene::walk::object_update_downward(Ptr<_>, Ptr<_>, u32)
    ))
    .culling_translated(0x00a5_9f90)
    .main_thread(0)
    .inputs(&inputs(&[
        &[
            Input::Of(NODE_FLAGS),
            Input::Of(DATA_FLAGS),
            Input::Of(&[0, 1, 2, 3]),
        ],
        &P3,
    ]))
    .max_cases(256)
    .setup(|e, case| {
        stand_ins(e);
        let g = geometry(e, case, case.word(0), 3);
        if case.word(2) & 1 != 0 {
            set_slot(e, g, 0xb8, 0x00a6_8c60);
        }
        if case.word(2) & 2 != 0 {
            set_slot(e, g, 0xbc, 0x00a8_0710);
        }
        vec![g, block(e, 8, &[ONE, case.word(1)]), 0x55]
    })
    .run();
    println!("scene_object_update_downward on the main thread: {report}");
    report.assert_same();
}

#[test]
fn scene_fade_update_downward() {
    Diff::new(entry!(
        0x00b4_ded0,
        engine_typed::scene::walk::fade_update_downward(Ptr<_>, u32, u32)
    ))
    .common(0x00b4_ded0)
    .inputs(&inputs(&[&PASS, &[Input::Of(&[0, 1, 6])]]))
    .setup(|e, case| {
        stand_ins(e);
        let n = node(e, case.word(0), case.word(1), case.word(2));
        vec![n, data(e, case, 3), case.word(4)]
    })
    .run()
    .assert_same();
}

#[test]
fn scene_particles_update_downward() {
    Diff::new(entry!(
        0x00c1_ad90,
        engine_typed::scene::walk::particles_update_downward(Ptr<_>, Ptr<_>, u32)
    ))
    .common(0x00c1_ad90)
    .inputs(&inputs(&[
        &[Input::Of(NODE_FLAGS), Input::Of(DATA_FLAGS), Input::Flag],
        &P3,
    ]))
    .max_cases(128)
    .setup(|e, case| {
        stand_ins(e);
        let g = geometry(e, case, case.word(0), 3);
        if case.word(2) != 0 {
            set_slot(e, g, 0xbc, 0x00a8_0710);
        }
        vec![g, block(e, 8, &[ONE, case.word(1)]), 7]
    })
    .run()
    .assert_same();
}

/// The classes a downward pass reaches by slot +0xa4: a stand-in, `NiNode`'s, a plain object's, the
/// fade node's, the particle system's.
const DOWNWARD_SLOTS: &[u32] = &[0, 0x00a5_dd70, 0x00a5_9f90, 0x00b4_ded0, 0x00c1_ad90];

/// A downward pass stand-in (registered at [`CHANGES_AFTER`]) that gives the object a new object at
/// +0x18 while it runs: the update's entry reads +0x18 after the pass.
const CHANGES_AFTER: u32 = 0x00ff_1e00;

fn changes_after(e: &mut Engine, a: &[u32]) -> Ret {
    log(e, 0x1e00, a[0]);
    let after = object(e, 0x40);
    e.mem.set_u32(a[0] + 0x18, after);
    ret(0)
}

/// The classes the update's entry meets by slot +0xa4: those of [`DOWNWARD_SLOTS`] and the stand-in
/// that changes +0x18.
const UPDATE_SLOTS: &[u32] = &[
    0,
    0x00a5_dd70,
    0x00a5_9f90,
    0x00b4_ded0,
    0x00c1_ad90,
    CHANGES_AFTER,
];

#[test]
fn scene_update() {
    // The object's class (slot +0xa4), its flags, the data's flags, an object at +0x18 or none.
    Diff::new(entry!(
        0x00a5_9c60,
        engine_typed::scene::walk::update(Ptr<_>, u32)
    ))
    .common(0x00a5_9c60)
    .inputs(&[
        Input::Of(UPDATE_SLOTS),
        Input::Of(NODE_FLAGS),
        Input::Of(DATA_FLAGS),
        Input::Flag,
    ])
    .setup(|e, case| {
        stand_ins(e);
        e.register(CHANGES_AFTER, changes_after);
        let n = node(e, case.word(1), 3, ONE);
        if case.word(0) != 0 {
            set_slot(e, n, 0xa4, case.word(0));
        }
        if case.word(3) != 0 {
            let after = object(e, 0x40);
            e.mem.set_u32(n + 0x18, after);
        }
        vec![n, block(e, 8, &[ONE, case.word(2)])]
    })
    .run()
    .assert_same();
}

/// `NiNode`'s typed pass with children of every class the scene family takes (child `i` gets
/// `DOWNWARD_SLOTS[(i + shift) % 5]`): each child's pass a Rust call when the family is on.
#[test]
fn update_downward_scene_children() {
    Diff::new(entry!(
        0x00a5_dd70,
        scene::update_downward(Ptr<_>, u32, u32)
    ))
    .common(0x00a5_dd70)
    .inputs(&inputs(&[
        &PASS,
        &[Input::Of(&[0, 1, 6]), Input::Of(&[0, 1, 2, 3, 4])],
    ]))
    .setup(|e, case| {
        stand_ins(e);
        let n = node(e, case.word(0), case.word(1), case.word(2));
        let base = e.mem.u32(n + 0xa0);
        for i in 0..case.word(1) {
            let child = e.mem.u32(base + 4 * i);
            let class = DOWNWARD_SLOTS[((i + case.word(5)) % 5) as usize];
            if child != 0 && class != 0 {
                set_slot(e, child, 0xa4, class);
            }
        }
        vec![n, data(e, case, 3), case.word(4)]
    })
    .run()
    .assert_same();
}

/// A skin instance with `bones` bones (at least one in the list, which the bound always reads): the
/// skin data (+8) with the root-to-skin transform (+0xc, the case's floats at `f`: scale, first
/// entry), the bone data (+0x40, each 0x4c bytes with its bound at +0x34: bone 0's radius the case's
/// float at `f + 2`, bone 1's `radius1`, the others 2.0) and the count (+0x44); the root parent
/// (+0x10) and the bones (+0x14), objects with world transforms (scale at +0x98): bone 1's `scale`,
/// bone 2's 1.0, the others 2.0.
fn skin(e: &mut Engine, case: &Case, bones: u32, scale: u32, radius1: u32, f: usize) -> u32 {
    let n = bones.max(1);
    let data = e.mem.alloc(0x48);
    let to_skin = transform(e, case, f);
    let to_skin = e.mem.bytes(to_skin, 0x34);
    e.mem.write(data + 0xc, &to_skin);
    let bone_data = e.mem.alloc(0x4c * n);
    for i in 0..n {
        let r = match i {
            0 => bits(case, f + 2),
            1 => radius1,
            _ => TWO,
        };
        words(e, bone_data + 0x4c * i + 0x34, &[HALF, ONE, MINUS_ONE, r]);
    }
    e.mem.set_u32(data + 0x40, bone_data);
    e.mem.set_u32(data + 0x44, bones);
    let with_world = |e: &mut Engine, scale: u32, at: usize| {
        let o = object(e, 0xb0);
        let t = transform(e, case, at);
        let w = e.mem.bytes(t, 0x34);
        e.mem.write(o + 0x68, &w);
        e.mem.set_u32(o + 0x98, scale);
        o
    };
    let root = with_world(e, TWO, f + 1);
    let list = e.mem.alloc(4 * n);
    for i in 0..n {
        let s = match i {
            1 => scale,
            2 => ONE,
            _ => TWO,
        };
        let b = with_world(e, s, f);
        e.mem.set_u32(list + 4 * i, b);
    }
    block(e, 0x1c, &[0, 0, data, 0, root, list, 0])
}

/// The merge's constants in the exe's data: 0.5 and 0.0 as doubles, the threshold 1e-6.
fn merge_constants(e: &mut Engine) {
    e.map(0x0101_1000, 0x2000);
    e.mem.set_u64(0x0101_1588, 0.5f64.to_bits());
    e.mem.set_u64(0x0101_2060, 0);
    e.map(0x011a_b000, 0x1000);
    e.mem.set_u32(0x011a_b3c0, 0x3586_37bd);
}

/// The bone-1 radii the skin tests try: an ordinary one, a quiet and a signalling NaN (the fast path
/// copies the radius through the x87).
const RADII: &[u32] = &[TWO, 0x7fc0_0000, 0x7f80_0001];

#[test]
fn scene_skin_bound() {
    // The bones (0: only the first bone's bound); bone 1's world scale (1.0 among the values takes
    // the fast path); bone 1's radius; the root-to-skin scale, the transforms' first entries and bone
    // 0's radius.
    Diff::new(entry!(
        0x00a8_6530,
        engine_typed::scene::skin::skin_bound(Ptr<_>, Ptr<_>)
    ))
    .common(0x00a8_6530)
    // The transform product copies the scale as it is (a signalling NaN radius's scale keeps its payload).
    .allow_nan_payloads()
    .inputs(&inputs(&[
        &[
            Input::Of(&[0, 1, 2, 3]),
            Input::Of(&[ONE, TWO, HALF, 0x7fc0_0000, 0]),
            Input::Of(RADII),
        ],
        &P3,
    ]))
    .max_cases(256)
    .setup(|e, case| {
        stand_ins(e);
        merge_constants(e);
        let s = skin(e, case, case.word(0), case.word(1), case.word(2), 3);
        let bound = block(e, 0x10, &[ONE, TWO, HALF, THIRD]);
        vec![s, bound]
    })
    .run()
    .assert_same();
}

/// The culling family's `NiGeometry::UpdateWorldBound` of a skinned geometry: with the scene family
/// on, its call of the skin instance's bound is a Rust call.
#[test]
fn scene_skin_bound_from_culling() {
    // The bones; bone 1's world scale; the geometry's flags (bit 19 forces the update; without it
    // the old radius decides); the case's floats.
    // Under the other threads' control word bit for bit, every input.
    Diff::new(entry!(
        0x00a8_0710,
        culling::update::geometry_update_world_bound(Ptr<_>)
    ))
    .common(0x00a8_0710)
    .other_threads()
    .inputs(&inputs(&[
        &[
            Input::Of(&[1, 2, 3]),
            Input::Of(&[ONE, TWO]),
            Input::Of(&[0, 0x0008_0000, 0x0048_0000]),
        ],
        &P3,
    ]))
    .max_cases(96)
    .setup(|e, case| {
        stand_ins(e);
        merge_constants(e);
        let g = geometry(e, case, case.word(2), 3);
        let s = skin(e, case, case.word(0), case.word(1), TWO, 3);
        e.mem.set_u32(g + 0xbc, s);
        vec![g]
    })
    .run()
    .assert_same();
    // Under the main thread's, the floats in 2^-4 to 4 ([`unit`]): the bound moved by the world transform (the
    // culling family's `bound_transform`, its rows' sums of products in `f64`). Within 4 ulps, or 2^-13 in value.
    let report = Diff::new(entry!(
        0x00a8_0710,
        culling::update::geometry_update_world_bound(Ptr<_>)
    ))
    .common(0x00a8_0710)
    .main_thread(4)
    .float_abs(2f32.powi(-13))
    .inputs(&inputs(&[
        &[
            Input::Of(&[1, 2, 3]),
            Input::Of(&[ONE, TWO]),
            Input::Of(&[0, 0x0008_0000, 0x0048_0000]),
        ],
        &P3,
    ]))
    .max_cases(96)
    .setup(|e, case| {
        let case = &folded(case, unit);
        stand_ins(e);
        merge_constants(e);
        let g = geometry(e, case, case.word(2), 3);
        let s = skin(e, case, case.word(0), case.word(1), TWO, 3);
        e.mem.set_u32(g + 0xbc, s);
        vec![g]
    })
    .run();
    println!("scene_skin_bound_from_culling on the main thread: {report}");
    report.assert_same();
}

#[test]
fn scene_transform_inverse() {
    Diff::new(entry!(
        0x008b_42e0,
        engine_typed::scene::skin::transform_inverse(Ptr<_>, Ptr<_>)
    ))
    .common(0x008b_42e0)
    .inputs(&inputs(&[&P3, &[Input::F32Swept(TWO)]]))
    .max_cases(128)
    .setup(|e, case| {
        stand_ins(e);
        let t = transform(e, case, 0);
        e.mem.set_u32(t + 0x28, bits(case, 2));
        e.mem.set_u32(t + 0x2c, bits(case, 3));
        vec![t, e.mem.alloc(0x34)]
    })
    .run()
    .assert_same();
}

// ---- The scene family: the queued node updates (engine_typed::scene::queue) ----

/// The pending list (an item and the next node, inline) and its lock, the update time.
const PENDING_LIST: u32 = 0x011c_a174;
const PENDING_LOCK: u32 = 0x011c_a1a0;
const UPDATE_TIME: u32 = 0x011c_3c08;
/// A slot function answering `this`: an object's "as a node", "as a geometry" or "parent" for the
/// classes that have one.
const AS_SELF: u32 = 0x00ff_1f00;

/// The queue's world: the list's `IsEmpty` and `RemoveHead` over the inline list, the smart pointer's
/// copy, the reference of a node (the word at the node's +0x40), the cell's update for it.
fn queue_world(e: &mut Engine) {
    stand_ins(e);
    e.map(0x011c_a000, 0x1000);
    e.map(0x011c_3000, 0x1000);
    e.mem.set_u32(UPDATE_TIME, HALF);
    e.register(AS_SELF, |e, a| {
        log(e, 0x1f00, a[0]);
        ret(a[0])
    });
    e.register(0x004a_4460, |e, a| {
        let empty = e.mem.u32(a[0]) == 0 && e.mem.u32(a[0] + 4) == 0;
        ret(u32::from(empty))
    });
    e.register(0x004e_e8a0, |e, a| {
        log(e, 0xe8a0, e.mem.u32(a[0]));
        let next = e.mem.u32(a[0] + 4);
        if next != 0 {
            let (item, after) = (e.mem.u32(next), e.mem.u32(next + 4));
            words(e, a[0], &[item, after]);
        } else {
            e.mem.set_u32(a[0], 0);
        }
        ret(0)
    });
    e.register(0x0055_9a40, |e, a| {
        let held = e.mem.u32(a[1]);
        e.mem.set_u32(a[0], held);
        log(e, 0x9a40, held);
        ret(a[0])
    });
    e.register(0x0056_f930, |e, a| {
        log(e, 0xf930, a[0]);
        ret(e.mem.u32(a[0] + 0x40))
    });
    e.register(0x0054_a070, |e, a| {
        log(e, 0xa070, a[1]);
        ret(0)
    });
}

/// A queued object of `kind`: 0 a plain object, 1 a geometry (its "as a geometry" slot answers
/// it), 2 a node (its "as a node" slot answers it) whose children are a geometry, a null slot and a
/// plain object; with `parent` its parent slot answers it; its flags; with `reference` the reference
/// its node finds (whose word +0x40 is a cell when `cell` is set).
fn queued(e: &mut Engine, kind: u32, parent: bool, flags: u32, reference: bool, cell: bool) -> u32 {
    let o = if kind == 2 {
        let n = node(e, flags, 3, ONE);
        let base = e.mem.u32(n + 0xa0);
        let first = e.mem.u32(base);
        set_slot(e, first, 0x30, AS_SELF);
        set_slot(e, n, 0x0c, AS_SELF);
        n
    } else {
        let o = object(e, 0xb0);
        e.mem.set_u32(o + 0x30, flags);
        if kind == 1 {
            set_slot(e, o, 0x30, AS_SELF);
        }
        o
    };
    if parent {
        set_slot(e, o, 0x10, AS_SELF);
    }
    e.mem.set_u32(o + 4, 1);
    if reference {
        let r = object(e, 0x80);
        let c = if cell { object(e, 0x40) } else { 0 };
        e.mem.set_u32(r + 0x40, c);
        e.mem.set_u32(o + 0x40, r);
    }
    o
}

#[test]
fn scene_run_queued() {
    // The queued count; the first's kind; parents; the waiting flag; a reference; its cell.
    Diff::new(entry!(
        0x0055_2570,
        engine_typed::scene::queue::run_queued()
    ))
    .common(0x0055_2570)
    .inputs(&[
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0, 1, 2]),
        Input::Flag,
        Input::Of(&[0, 0x0040_0000, 0x0040_0002]),
        Input::Flag,
        Input::Flag,
    ])
    .max_cases(192)
    .setup(|e, case| {
        queue_world(e);
        let count = case.word(0);
        let parent = case.word(2) != 0;
        let (reference, cell) = (case.word(4) != 0, case.word(5) != 0);
        let first = queued(e, case.word(1), parent, case.word(3), reference, cell);
        let second = queued(e, 1, true, 0x0040_0000, true, true);
        match count {
            0 => words(e, PENDING_LIST, &[0, 0]),
            1 => words(e, PENDING_LIST, &[first, 0]),
            _ => {
                let next = block(e, 8, &[second, 0]);
                words(e, PENDING_LIST, &[first, next]);
            }
        }
        words(e, PENDING_LOCK, &[0, 0]);
        vec![]
    })
    .run()
    .assert_same();
}

#[test]
fn scene_update_geometries() {
    // A null node, a plain object, a geometry, a node with children.
    Diff::new(entry!(
        0x0055_26f0,
        engine_typed::scene::queue::update_geometries(Ptr<_>)
    ))
    .common(0x0055_26f0)
    .inputs(&[Input::Of(&[0, 1, 2, 3]), Input::Of(NODE_FLAGS)])
    .setup(|e, case| {
        queue_world(e);
        let o = match case.word(0) {
            0 => 0,
            k => queued(e, k - 1, false, case.word(1), false, false),
        };
        vec![o]
    })
    .run()
    .assert_same();
}

#[test]
fn scene_waiting() {
    Diff::new(entry!(
        0x0055_26b0,
        engine_typed::scene::queue::waiting(Ptr<_>) -> bool
    ))
    .common(0x0055_26b0)
    .inputs(&[Input::Of(&[0, 0x0040_0000, 0x0080_0000, 0xffff_ffff])])
    .setup(|e, case| {
        let o = block(e, 0x40, &[0; 16]);
        e.mem.set_u32(o + 0x30, case.word(0));
        vec![o]
    })
    .run()
    .assert_same();
}

#[test]
fn scene_set_waiting() {
    Diff::new(entry!(
        0x0055_26d0,
        engine_typed::scene::queue::set_waiting(Ptr<_>, bool)
    ))
    .common(0x0055_26d0)
    .inputs(&[
        Input::Of(&[0, 0x0040_0000, 0x0080_0001]),
        Input::Of(&[0, 1, 0x100]),
    ])
    .setup(|e, case| {
        stand_ins(e);
        let o = object(e, 0x40);
        e.mem.set_u32(o + 0x30, case.word(0));
        vec![o, case.word(1)]
    })
    .run()
    .assert_same();
}
