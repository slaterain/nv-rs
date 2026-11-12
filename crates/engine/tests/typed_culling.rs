//! The differential test of every typed function of the culling family (`engine_typed::culling`)
//! against its translation: here because the translations live in the part crates, and this crate
//! registers them all. One scene serves every test: a culling process with six frustum planes (the
//! box `|x|, |y|, |z| <= 10`, normals inward), objects with world bounds and transforms, a node with
//! children (one slot empty), a multi-bound node and box, a compound frustum with a short program.
//! The float cases sweep the harness's operand classes (NaN, infinities, zeros, denormals, huge,
//! values that round) through centres, radii, plane constants, scales and matrix entries, under both
//! control words (the family's row is `Threads::Both`).
//!
//! What stands in: virtual functions the walk calls on objects, processes, accumulators and shapes
//! (test functions past `.text` that log what they get), the bound pool, the thread's scrap heap,
//! the skin instance's bound update and the interlocked imports. Everything else runs translated.

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::{Case, Diff, Input, Value};
use engine_core::typed::{MAIN_THREAD, OTHER_THREADS};
use engine_typed::culling::{self, bound, compound, planes, process, update, visible};
use engine_typed::dispatch;

/// Where the test's virtual functions are registered: past `.text`, one per vtable slot.
const VT_FN: u32 = 0x00ff_2000;
/// The log the stand-ins write (a count, then `(tag, word)` pairs), at a fixed guest address.
const LOG: u32 = 0x0300_0000;
/// The thread's scrap heap the stand-in answers.
const HEAP: u32 = 0x0300_2000;

/// Callees and imports.
const POOL_ALLOCATE: u32 = 0x00aa_3e40;
const SCRAP_HEAP: u32 = 0x00aa_42e0;
const SCRAP_ALLOCATE: u32 = 0x00aa_54a0;
const SCRAP_FREE: u32 = 0x00aa_5610;
const SKIN_BOUND: u32 = 0x00a8_6530;
const INCREMENT: u32 = 0x00fd_f1c0;
const DECREMENT: u32 = 0x00fd_f1c4;
const GETPTD_NOEXIT: u32 = 0x00ed_1165;

/// The real functions the scene's vtables point at.
const PROCESS: u32 = 0x00c4_ee90;
const APPEND_VIRTUAL: u32 = 0x00c4_f1d0;
const NODE_ON_VISIBLE: u32 = 0x00a5_dbe0;
const GEOMETRY_ON_VISIBLE: u32 = 0x00a7_fd90;
/// `NiPlane::NiPlane(normal, point)`: the typed `NiFrustumPlanes::Set` does its work in place.
const PLANE_THROUGH: u32 = 0x00a6_9990;

/// The most calls one run makes here is a few hundred: a version that loops stops at this many.
const MAX_CALLS: usize = 5_000;

/// Cases each test adds with every float argument at once a random float with a full mantissa (the
/// harness's float sweep): the ordinary cases vary one at a time against plain others, which misses a
/// rounding that only shows when several operands carry low bits (batch 2's Merge mutant).
const FLOAT_SWEEP: usize = 64;

/// Argument `i` of the case, a float, as its bits.
fn bits(case: &Case, i: usize) -> u32 {
    case.f32(i).to_bits()
}

/// The float bit patterns a scene value takes in the cases that don't sweep all of them: an
/// ordinary value, zero, a NaN.
const FEW_FLOATS: &[u32] = &[0x3f80_0000, 0, 0x7fc0_0000, 0xc120_0000];

fn ret(eax: u32) -> Ret {
    Ret {
        eax,
        ..Ret::default()
    }
}

/// Logs `(tag, word)`.
fn log(e: &mut Engine, tag: u32, word: u32) {
    let n = e.mem.u32(LOG);
    e.mem.set_u32(LOG + 4 + 8 * n, tag);
    e.mem.set_u32(LOG + 8 + 8 * n, word);
    e.mem.set_u32(LOG, n + 1);
}

/// The stand-ins and the constants the code reads from the exe's data.
fn stand_ins(e: &mut Engine) {
    e.map(LOG, 0x3000);
    // OnVisible, Process, AppendVirtual: logged.
    e.register(VT_FN + 0xd4, |e, a| {
        log(e, 0xd4, a[0]);
        ret(0)
    });
    e.register(VT_FN + 0x44, |e, a| {
        log(e, 0x44, a[1]);
        ret(0)
    });
    e.register(VT_FN + 0x4c, |e, a| {
        log(e, 0x4c, a[1]);
        ret(0)
    });
    // SetWorldBound: the word into +0x20.
    e.register(VT_FN + 0x78, |e, a| {
        e.mem.set_u32(a[0] + 0x20, a[1]);
        ret(0)
    });
    // The collision object's update, the accumulator's camera, a deleting destructor: logged.
    e.register(VT_FN + 0x90, |e, a| {
        log(e, 0x90, a[1]);
        ret(0)
    });
    e.register(VT_FN + 0x8c, |e, a| {
        log(e, 0x8c, a[1]);
        ret(0)
    });
    e.register(VT_FN + 0x04, |e, a| {
        log(e, 0x04, a[0]);
        ret(0)
    });
    // The accumulator's RegisterObject: its word at +0x10 says whether it accepts.
    e.register(VT_FN + 0x98, |e, a| {
        log(e, 0x98, a[1]);
        ret(e.mem.u32(a[0] + 0x10))
    });
    // A reference's fade node: its word at +0x10.
    e.register(VT_FN + 0x10, |e, a| ret(e.mem.u32(a[0] + 0x10)));
    // A shape's WithinFrustum: its word at +0x24 (past a box), garbage in the high bytes.
    e.register(VT_FN + 0x9c, |e, a| {
        log(e, 0x9c, a[1]);
        ret(e.mem.u32(a[0] + 0x24) | 0x5a00)
    });
    e.register(POOL_ALLOCATE, |e, a| ret(e.mem.alloc(a[1])));
    e.register(SCRAP_HEAP, |_, _| ret(HEAP));
    e.register(SCRAP_ALLOCATE, |e, a| {
        let block = e.mem.alloc(a[1].max(4));
        // What the heap leaves in a new block.
        for i in 0..a[1] / 4 {
            e.mem.set_u32(block + 4 * i, 0xcdcd_cdcd);
        }
        ret(block)
    });
    e.register(SCRAP_FREE, |e, a| {
        log(e, 0x5610, a[1]);
        ret(0)
    });
    // The skin's update: logged, and the bound's radius doubled as words so it shows.
    e.register(SKIN_BOUND, |e, a| {
        log(e, 0x6530, a[1]);
        let r = e.mem.u32(a[1] + 0xc);
        e.mem.set_u32(a[1] + 0xc, r ^ 0x0080_0000);
        ret(0)
    });
    e.register(INCREMENT, |e, a| {
        let n = e.mem.u32(a[0]).wrapping_add(1);
        e.mem.set_u32(a[0], n);
        ret(n)
    });
    e.register(DECREMENT, |e, a| {
        let n = e.mem.u32(a[0]).wrapping_sub(1);
        e.mem.set_u32(a[0], n);
        ret(n)
    });
    // The square root's error path (an infinite or NaN operand) sets `errno`: the thread has no
    // CRT data block here, so it goes to the static slot.
    e.register(GETPTD_NOEXIT, |_, _| ret(0));
    e.map(0x011c_1000, 0x1000);
    // The exe's constants: 0.5 and 0.0 as doubles, the merge threshold 1e-6, the scrap alignment;
    // the empty bound and the zero point are zero.
    e.map(0x0101_1000, 0x2000);
    e.mem.set_u64(0x0101_1588, 0.5f64.to_bits());
    e.mem.set_u64(0x0101_2060, 0);
    e.map(0x011a_b000, 0x1000);
    e.mem.set_u32(0x011a_b3c0, 0x3586_37bd);
    // The unit axes the box's corners are made from (`011a946c`, three points).
    e.map(0x011a_9000, 0x1000);
    for (i, w) in [ONE, 0, 0, 0, ONE, 0, 0, 0, ONE].iter().enumerate() {
        e.mem.set_u32(0x011a_946c + 4 * i as u32, *w);
    }
    // The double 1.0 the perspective planes add.
    e.mem.set_u64(0x0101_2070, 1.0f64.to_bits());
    e.map(0x010a_2000, 0x1000);
    e.mem.set_u32(0x010a_2720, 0x10);
    e.map(0x011f_4000, 0x3000);
    e.map(0x0101_f000, 0x1000);
    e.map(0x0109_c000, 0x1000);
}

/// A vtable of 0x100 slots, each the stand-in for its slot unless `real` says otherwise.
fn vtable(e: &mut Engine, real: &[(u32, u32)]) -> u32 {
    let vt = e.mem.alloc(0x100);
    for slot in (0..0x100).step_by(4) {
        let target = real
            .iter()
            .find(|(s, _)| *s == slot)
            .map_or(VT_FN + slot, |(_, a)| *a);
        e.mem.set_u32(vt + slot, target);
    }
    vt
}

/// A bound of four float bit patterns.
fn bound_of(e: &mut Engine, x: u32, y: u32, z: u32, r: u32) -> u32 {
    let b = e.mem.alloc(0x10);
    e.mem.set_u32(b, x);
    e.mem.set_u32(b + 4, y);
    e.mem.set_u32(b + 8, z);
    e.mem.set_u32(b + 0xc, r);
    b
}

/// A transform: rotation `m` (nine float patterns), translation, scale.
fn transform_at(e: &mut Engine, at: u32, m: [u32; 9], t: [u32; 3], s: u32) {
    for (i, w) in m.iter().enumerate() {
        e.mem.set_u32(at + 4 * i as u32, *w);
    }
    for (i, w) in t.iter().enumerate() {
        e.mem.set_u32(at + 0x24 + 4 * i as u32, *w);
    }
    e.mem.set_u32(at + 0x30, s);
}

/// A rotation of a few degrees about z, as float patterns (cos, sin of 0.1).
const ROTATION: [u32; 9] = [
    0x3f7e_8a6e,
    0xbdcc_6c67,
    0,
    0x3dcc_6c67,
    0x3f7e_8a6e,
    0,
    0,
    0,
    0x3f80_0000,
];
const ONE: u32 = 0x3f80_0000;
const TWO: u32 = 0x4000_0000;
const HALF_F: u32 = 0x3f00_0000;
const MINUS_TEN: u32 = 0xc120_0000;

/// The rotation's entries but `skip`, for the sweep to fill (the ordinary cases keep [`ROTATION`]).
fn swept_rotation(skip: usize) -> impl Iterator<Item = Input> {
    ROTATION
        .into_iter()
        .enumerate()
        .filter(move |(i, _)| *i != skip)
        .map(|(_, b)| Input::F32Swept(b))
}

/// The rotation from the case: entry `skip` from argument `at`, the other eight from argument `from`
/// on ([`swept_rotation`]).
fn rotation_from(case: &Case, skip: usize, at: usize, from: usize) -> [u32; 9] {
    let mut k = from;
    std::array::from_fn(|i| {
        if i == skip {
            bits(case, at)
        } else {
            k += 1;
            bits(case, k - 1)
        }
    })
}

/// The bound update tests' inputs: the centre's x, the radius, the scale, rotation entry `skip`
/// (arguments 0-3); swept: the centre's y and z (4, 5), the other rotation entries (6-13), the
/// translation (14-16).
fn bound_inputs(skip: usize) -> Vec<Input> {
    [Input::F32, Input::F32, Input::F32, Input::F32]
        .into_iter()
        .chain([TWO, MINUS_TEN].map(Input::F32Swept))
        .chain(swept_rotation(skip))
        .chain([ONE, 0x3dcc_cccd, MINUS_TEN].map(Input::F32Swept))
        .collect()
}

/// A transform from [`bound_inputs`]' arguments at `t`.
fn bound_transform_at(e: &mut Engine, t: u32, case: &Case, skip: usize) {
    let m = rotation_from(case, skip, 3, 6);
    let translation = [bits(case, 14), bits(case, 15), bits(case, 16)];
    transform_at(e, t, m, translation, bits(case, 2));
}

/// An object: `size` bytes, vtable `vt`, world bound `bound`, flags; its local transform the
/// rotation, its world transform scaled by 2 and moved.
fn object(e: &mut Engine, size: u32, vt: u32, bound: u32, flags: u32) -> u32 {
    let o = e.mem.alloc(size);
    e.mem.set_u32(o, vt);
    e.mem.set_u32(o + 4, 1);
    e.mem.set_u32(o + 0x20, bound);
    e.mem.set_u32(o + 0x30, flags);
    transform_at(e, o + 0x34, ROTATION, [ONE, TWO, HALF_F], ONE);
    transform_at(e, o + 0x68, ROTATION, [TWO, ONE, MINUS_TEN], TWO);
    o
}

/// Six planes, the box `|x|, |y|, |z| <= 10` with inward normals, and an active mask, at `at`.
fn planes_at(e: &mut Engine, at: u32, active: u32) {
    let normals: [[u32; 3]; 6] = [
        [ONE, 0, 0],
        [0xbf80_0000, 0, 0],
        [0, ONE, 0],
        [0, 0xbf80_0000, 0],
        [0, 0, ONE],
        [0, 0, 0xbf80_0000],
    ];
    for (i, n) in normals.iter().enumerate() {
        let p = at + 0x10 * i as u32;
        e.mem.set_u32(p, n[0]);
        e.mem.set_u32(p + 4, n[1]);
        e.mem.set_u32(p + 8, n[2]);
        e.mem.set_u32(p + 0xc, MINUS_TEN);
    }
    e.mem.set_u32(at + 0x60, active);
}

/// The scene's parts.
#[derive(Clone, Copy)]
#[allow(dead_code)]
struct Scene {
    /// A `BSCullingProcess` (0xc8 bytes): mode 0, all planes active, no compound frustum, an
    /// accumulator that accepts.
    process: u32,
    accumulator: u32,
    /// A node (0xac bytes) with four child slots: two geometries, an empty slot, a node.
    node: u32,
    children: [u32; 4],
    /// A geometry (0xc8 bytes) with model data.
    geometry: u32,
    /// The vtables: a node's, a geometry's (both real `OnVisible`), and plain stand-ins.
    node_vt: u32,
    geometry_vt: u32,
    stand_in_vt: u32,
}

fn scene(e: &mut Engine) -> Scene {
    stand_ins(e);
    let stand_in_vt = vtable(e, &[]);
    let node_vt = vtable(e, &[(0xd4, NODE_ON_VISIBLE)]);
    let geometry_vt = vtable(e, &[(0xd4, GEOMETRY_ON_VISIBLE)]);
    let process_vt = vtable(e, &[(0x44, PROCESS), (0x4c, APPEND_VIRTUAL)]);
    let accumulator = e.mem.alloc(0x20);
    e.mem.set_u32(accumulator, stand_in_vt);
    e.mem.set_u32(accumulator + 4, 1);
    e.mem.set_u32(accumulator + 0x10, 1);
    let process = e.mem.alloc(0xc8);
    e.mem.set_u32(process, process_vt);
    planes_at(e, process + 0x2c, 0x3f);
    e.mem.set_u32(process + 0xc4, accumulator);
    let g1_bound = bound_of(e, ONE, ONE, ONE, ONE);
    let g1 = object(e, 0xc8, geometry_vt, g1_bound, 0);
    let g2_bound = bound_of(e, 0x41a0_0000, 0, 0, ONE);
    let g2 = object(e, 0xc8, geometry_vt, g2_bound, 0);
    let inner_bound = bound_of(e, 0, 0, 0, TWO);
    let inner = object(e, 0xac, node_vt, inner_bound, 0);
    let node_bound = bound_of(e, 0, 0, 0, 0x4120_0000);
    let node = object(e, 0xac, node_vt, node_bound, 0);
    let slots = e.mem.alloc(0x10);
    let children = [g1, g2, 0, inner];
    for (i, c) in children.iter().enumerate() {
        e.mem.set_u32(slots + 4 * i as u32, *c);
    }
    e.mem.set_u32(node + 0x9c, stand_in_vt);
    e.mem.set_u32(node + 0xa0, slots);
    e.mem.set_u16(node + 0xa4, 4);
    e.mem.set_u16(node + 0xa6, 4);
    e.mem.set_u16(node + 0xa8, 3);
    for child in [g1, g2, inner] {
        e.mem.set_u32(child + 0x18, node);
    }
    // The geometry's model data: its bound at +0x10.
    let data = e.mem.alloc(0x40);
    e.mem.set_u32(data + 0x10, ONE);
    e.mem.set_u32(data + 0x14, TWO);
    e.mem.set_u32(data + 0x18, MINUS_TEN);
    e.mem.set_u32(data + 0x1c, HALF_F);
    e.mem.set_u32(g1 + 0xb8, data);
    e.mem.set_u32(g2 + 0xb8, data);
    Scene {
        process,
        accumulator,
        node,
        children,
        geometry: g1,
        node_vt,
        geometry_vt,
        stand_in_vt,
    }
}

// ---- Native float maths ----
//
// The family's maths is native (ADR-0014's float rule; `engine_typed::culling`, "Floats"): what also
// runs off the main thread in `f64` locals rounded where the exe stores a float, a single operation on
// two floats in plain `f32` (which rounds to the same float), the fade-node culler, which only the main
// thread runs, in `f32`. Under the other threads' control word (53-bit precision) the `f64` code is the
// exe's arithmetic bit for bit, so those comparisons take every input. Under the main thread's (24-bit
// precision, with the x87's wider exponent) the exe rounds every operation to 24 bits: a sum of products
// can differ from the native one in its last bits, by more ulps where it cancels to near zero. Those
// comparisons take the floats play has ([`unit`]) and say their tolerance; with `--nocapture` they print
// what they let through. A function that gives the exe's floats under both words for every input is
// compared under both.

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

/// A float as a bound's centre and radius, a rotation's entry, a scale, a plane's normal or a frustum's
/// edge has it in play, scaled down: its magnitude in 2^-4 to 4 (sign and mantissa kept), so that a
/// tolerance in value means something; a NaN the quiet NaN (play has no signalling NaNs or payloads:
/// the exe quiets a signalling NaN it copies through the x87 stack, native code copies it as it is).
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

/// The pathing family's functions the dispatch family calls: the point helpers, the absolute value,
/// and the word maps' key test and hash.
const PATHING_CALLED: &[u32] = &[
    0x0040_8840,
    0x0043_9e90,
    0x0043_9ef0,
    0x0045_7990,
    0x004a_0bd0,
    0x004a_3760,
    0x004a_7290,
    0x006b_81d0,
    0x006c_6a60,
];

/// What every test adds to its comparison: the families and the call budget.
trait Common {
    fn common(self) -> Diff;
    /// The same with the dispatch family on too: the walk's calls into dispatch (the transform product
    /// `0062c250`, the box's corners `00c39270`) go direct, as in play with every family on. The
    /// pathing functions dispatch calls go by address to their translations ([`PATHING_CALLED`]):
    /// their maths is native, and their own tests compare it (NaN payloads, where two NaNs meet,
    /// within the float rule). The translations dispatch reaches call its two counted helpers plainly.
    fn with_dispatch(self) -> Diff;
}

impl Common for Diff {
    fn common(self) -> Diff {
        self.family(&culling::CULLING)
            .family(&leaf::LEAVES)
            // The transform product and the box's corners are the dispatch family's, the skinned
            // bound the scene family's (their own tests).
            .goes_by_address(&[0x0062_c250, 0x00c3_9270, 0x00a8_6530])
            .max_calls(MAX_CALLS)
            .float_sweep(FLOAT_SWEEP)
    }

    fn with_dispatch(self) -> Diff {
        self.family(&culling::CULLING)
            .family(&leaf::LEAVES)
            .family(&dispatch::DISPATCH)
            .goes_by_address(PATHING_CALLED)
            .translation_calls_plainly(&[0x0040_19d0, 0x0040_8860])
            .max_calls(MAX_CALLS)
            .float_sweep(FLOAT_SWEEP)
    }
}

#[test]
fn plane_distance() {
    // Normal x, the constant, the point's x; the sweep also fills the normal's y and z and the
    // point's.
    Diff::new(entry!(
        0x004b_6160,
        bound::plane_distance(Ptr<_>, Ptr<_>) -> f32
    ))
    .common()
    // A NaN's payload as x87 and SSE pick it differently, and a signalling NaN copied as it is.
    .allow_nan_payloads()
    .inputs(&[
        Input::F32,
        Input::F32,
        Input::F32,
        Input::F32Swept(HALF_F),
        Input::F32Swept(0xbfc0_0000),
        Input::F32Swept(TWO),
        Input::F32Swept(ONE),
    ])
    .max_cases(256)
    .setup(|e, case| {
        stand_ins(e);
        let plane = bound_of(
            e,
            bits(case, 0),
            bits(case, 3),
            bits(case, 4),
            bits(case, 1),
        );
        let point = bound_of(e, bits(case, 2), bits(case, 5), bits(case, 6), 0);
        vec![plane, point]
    })
    .run()
    .assert_same();
}

#[test]
fn which_side() {
    // The centre's x, the radius, the plane's constant; the sweep also fills the centre's y and z
    // and the plane's normal.
    Diff::new(entry!(
        0x004b_6100,
        bound::which_side(Ptr<_>, Ptr<_>) -> u32
    ))
    .common()
    .inputs(&[
        Input::F32,
        Input::F32,
        Input::F32,
        Input::F32Swept(ONE),
        Input::F32Swept(0),
        Input::F32Swept(ONE),
        Input::F32Swept(0),
        Input::F32Swept(0),
    ])
    .max_cases(256)
    .setup(|e, case| {
        stand_ins(e);
        let b = bound_of(
            e,
            bits(case, 0),
            bits(case, 3),
            bits(case, 4),
            bits(case, 1),
        );
        let plane = bound_of(
            e,
            bits(case, 5),
            bits(case, 6),
            bits(case, 7),
            bits(case, 2),
        );
        vec![b, plane]
    })
    .run()
    .assert_same();
}

/// `merge`'s inputs: this centre's x, this radius, the other's centre x, the other's radius; swept:
/// both centres' y and z, so all three differences carry low bits at once.
const MERGE_INPUTS: [Input; 8] = [
    Input::F32,
    Input::F32,
    Input::F32,
    Input::F32,
    Input::F32Swept(ONE),
    Input::F32Swept(0),
    Input::F32Swept(0),
    Input::F32Swept(HALF_F),
];

/// A NaN as the one quiet NaN 0x7fc00000, any other float as it is. The merge hands the squared
/// distance of NaN centres to the square root, whose error path copies it through memory the
/// comparison reads byte for byte; x87 and SSE pick the payload of a NaN made from two differently, and
/// with one NaN in the inputs both pick the same.
fn quiet_nans(bits: u32) -> u32 {
    if (bits >> 23) & 0xff == 0xff && bits & 0x007f_ffff != 0 {
        0x7fc0_0000
    } else {
        bits
    }
}

/// A float as an integer from -64 to 64 (its bits modulo 129): centres whose differences square and
/// sum exactly at 24 bits, so a merge's squared distance, which goes to the square root as a call's
/// words (compared exactly), is the exe's on the main thread too.
fn small_int(bits: u32) -> u32 {
    (f32::from(u16::try_from(bits % 129).unwrap_or(0)) - 64.0).to_bits()
}

/// `merge`'s words for the main thread's comparison: the centres as [`small_int`]s, the radii in 2^-4
/// to 4 ([`unit`]).
fn merge_main_words(e: &mut Engine, case: &Case) -> Vec<u32> {
    stand_ins(e);
    let c = |i: usize| small_int(bits(case, i));
    let r = |i: usize| unit(bits(case, i));
    let a = bound_of(e, c(0), c(4), c(5), r(1));
    let b = bound_of(e, c(2), c(6), c(7), r(3));
    vec![a, b]
}

/// `merge`'s words for a case of [`MERGE_INPUTS`].
fn merge_words(e: &mut Engine, case: &Case) -> Vec<u32> {
    stand_ins(e);
    let a = bound_of(
        e,
        bits(case, 0),
        bits(case, 4),
        bits(case, 5),
        bits(case, 1),
    );
    let b = bound_of(
        e,
        bits(case, 2),
        bits(case, 6),
        bits(case, 7),
        bits(case, 3),
    );
    vec![a, b]
}

#[test]
fn merge() {
    // Under the other threads' control word bit for bit, every input (a NaN the one quiet NaN:
    // [`quiet_nans`]).
    Diff::new(entry!(0x00a7_f3f0, bound::merge(Ptr<_>, Ptr<_>)))
        .common()
        .other_threads()
        .inputs(&MERGE_INPUTS)
        .max_cases(384)
        .setup(|e, case| merge_words(e, &folded(case, quiet_nans)))
        .run()
        .assert_same();
    // Under the main thread's, the centres small integers and the radii in 2^-4 to 4
    // ([`merge_main_words`]): the parameter and the radius are sums and quotients the exe rounds at each
    // step and native code once, so the centre and the radius part by a few ulps of their size (up to
    // 256). Within 4 ulps, or 2^-14 in value.
    let report = Diff::new(entry!(0x00a7_f3f0, bound::merge(Ptr<_>, Ptr<_>)))
        .common()
        .main_thread(4)
        .float_abs(2f32.powi(-14))
        .inputs(&MERGE_INPUTS)
        .max_cases(384)
        .setup(merge_main_words)
        .run();
    println!("merge on the main thread: {report}");
    report.assert_same();
}

#[test]
fn do_culling() {
    // The bound: none, or its centre's x and radius; the active planes; the object's `OnVisible`: a stand-in, or the typed
    // node's or geometry's in its slot.
    Diff::new(entry!(0x00a6_94e0, process::do_culling(Ptr<_>, Ptr<_>)))
        .common()
        .inputs(&[
            Input::Of(&[0, 1]),
            Input::F32,
            Input::F32,
            Input::Of(&[0x3f, 0x2a, 0]),
            Input::Of(&[0, 1, 2]),
        ])
        .max_cases(256)
        .setup(|e, case| {
            let s = scene(e);
            let b = if case.word(0) == 0 {
                0
            } else {
                bound_of(e, case.f32(1).to_bits(), ONE, 0, case.f32(2).to_bits())
            };
            let vt = [s.stand_in_vt, s.node_vt, s.geometry_vt][case.word(4) as usize];
            let o = object(e, 0xc8, vt, b, 0);
            e.mem.set_u32(s.process + 0x8c, case.word(3));
            vec![s.process, o]
        })
        .run()
        .assert_same();
}

#[test]
fn bs_process() {
    // Mode; flags; compound frustum (none, in use, no operations); active planes.
    Diff::new(entry!(0x00c4_ee90, process::bs_process(Ptr<_>, Ptr<_>)))
        .common()
        .inputs(&[
            Input::Of(&[0, 1, 2, 3, 4]),
            Input::Of(&[0, 0x1000, 0x0010_1000, 0x800, 0x0010_0000]),
            Input::Of(&[0, 1, 2]),
            Input::Of(&[0x3f, 0]),
        ])
        .setup(|e, case| {
            let s = scene(e);
            e.mem.set_u32(s.process + 0x90, case.word(0));
            let b = bound_of(e, ONE, 0, 0, ONE);
            let o = object(e, 0xc8, s.stand_in_vt, b, case.word(1));
            let frustum = match case.word(2) {
                0 => 0,
                n => compound_frustum(e, if n == 1 { 6 } else { 0 }, 0, 0x3f),
            };
            e.mem.set_u32(s.process + 0xc0, frustum);
            e.mem.set_u32(s.process + 0x8c, case.word(3));
            vec![s.process, o]
        })
        .run()
        .assert_same();
}

/// A compound frustum: the view frustum (the scene's box, `active` planes), two plane sets (the
/// box moved by 5 along x, and a small occluder box at the origin), and a program: a portal test on
/// set 0 (pass to 2, fail to 5), an occluder test on set 1 (pass to 4, fail to 5), pass, fail.
/// `ops` operations are in use (0: none); `skip` skips the view frustum.
fn compound_frustum(e: &mut Engine, ops: i32, skip: u8, active: u32) -> u32 {
    let f = e.mem.alloc(0xa4);
    let sets = e.mem.alloc(0x64 * 2);
    planes_at(e, sets, 0x3f);
    e.mem.set_u32(sets + 0xc, 0xc0a0_0000);
    planes_at(e, sets + 0x64, 0x3f);
    for i in 0..6 {
        e.mem.set_u32(sets + 0x64 + 0x10 * i + 0xc, 0xbf80_0000);
    }
    e.mem.set_u32(f + 8, sets);
    e.mem.set_u32(f + 0xc, 2);
    e.mem.set_u32(f + 0x10, 2);
    let program: [[u32; 3]; 6] = [
        [7, 2, 5],
        [0, 0, 0],
        [8, 4, 5],
        [1, 0, 0],
        [2, 0, 0],
        [3, 0, 0],
    ];
    let code = e.mem.alloc(0xc * 6);
    for (i, op) in program.iter().enumerate() {
        for (j, w) in op.iter().enumerate() {
            e.mem.set_u32(code + 0xc * i as u32 + 4 * j as u32, *w);
        }
    }
    e.mem.set_u32(f + 0x18, code);
    e.mem.set_u32(f + 0x1c, 6);
    e.mem.set_u32(f + 0x20, 6);
    e.mem.set_u32(f + 0x24, 1);
    e.mem.set_u32(f + 0x28, ops as u32);
    e.mem.set_u32(f + 0x2c, 0);
    planes_at(e, f + 0x30, active);
    e.mem.set_u8(f + 0xa0, skip);
    f
}

#[test]
fn append_virtual() {
    // Accumulator: none, accepts, refuses; the geometry's name: none or one.
    Diff::new(entry!(0x00c4_f1d0, process::append_virtual(Ptr<_>, Ptr<_>)))
        .common()
        .inputs(&[Input::Of(&[0, 1, 2]), Input::Flag])
        .setup(|e, case| {
            let s = scene(e);
            match case.word(0) {
                0 => e.mem.set_u32(s.process + 0xc4, 0),
                2 => e.mem.set_u32(s.accumulator + 0x10, 0),
                _ => {}
            }
            let name = if case.word(1) != 0 { 0x0101_f7b0 } else { 0 };
            e.mem.set_u32(s.geometry + 8, name);
            vec![s.process, s.geometry]
        })
        .run()
        .assert_same();
}

/// A camera: its world transform at +0x68 (the object's), its frustum at +0xdc, its position at
/// +0x8c.
fn camera(e: &mut Engine, s: &Scene, left: u32) -> u32 {
    let c = object(e, 0x120, s.stand_in_vt, 0, 0);
    let frustum = [left, ONE, ONE, 0xbf80_0000, ONE, 0x447a_0000, 0];
    for (i, w) in frustum.iter().enumerate() {
        e.mem.set_u32(c + 0xdc + 4 * i as u32, *w);
    }
    e.mem.set_u32(c + 0x8c, TWO);
    e.mem.set_u32(c + 0x90, ONE);
    e.mem.set_u32(c + 0x94, MINUS_TEN);
    c
}

#[test]
fn set_frustum() {
    // No camera or one; the frustum's left edge.
    // Under the other threads' control word bit for bit, every input.
    Diff::new(entry!(0x00a6_94a0, process::set_frustum(Ptr<_>, Ptr<_>)))
        .common()
        .other_threads()
        .ignore_calls(&[PLANE_THROUGH])
        .inputs(&[Input::Flag, Input::F32])
        .setup(|e, case| {
            let s = scene(e);
            let c = camera(e, &s, case.f32(1).to_bits());
            e.mem
                .set_u32(s.process + 0xc, if case.word(0) != 0 { c } else { 0 });
            e.mem.set_u32(s.process + 0x8c, 0x5);
            vec![s.process, c + 0xdc]
        })
        .run()
        .assert_same();
    // Under the main thread's, the floats in 2^-4 to 4 ([`unit`]): the planes as in
    // `frustum_planes_set`. Within 4 ulps, or 2^-12 in value.
    let report = Diff::new(entry!(0x00a6_94a0, process::set_frustum(Ptr<_>, Ptr<_>)))
        .common()
        .main_thread(4)
        .float_abs(2f32.powi(-12))
        .ignore_calls(&[PLANE_THROUGH])
        .inputs(&[Input::Flag, Input::F32])
        .setup(|e, case| {
            let case = &folded(case, unit);
            let s = scene(e);
            let c = camera(e, &s, case.f32(1).to_bits());
            e.mem
                .set_u32(s.process + 0xc, if case.word(0) != 0 { c } else { 0 });
            e.mem.set_u32(s.process + 0x8c, 0x5);
            vec![s.process, c + 0xdc]
        })
        .run();
    println!("set_frustum on the main thread: {report}");
    report.assert_same();
}

#[test]
fn bs_process_scene() {
    // Camera; scene; visible set; the accumulator: the process's, none (the default one), none at all.
    Diff::new(entry!(
        0x00c4_f070,
        process::bs_process_scene(Ptr<_>, u32, Ptr<_>, u32)
    ))
    .common()
    .ignore_calls(&[PLANE_THROUGH])
    .inputs(&[Input::Flag, Input::Flag, Input::Flag, Input::Of(&[0, 1, 2])])
    .setup(|e, case| {
        let s = scene(e);
        let c = camera(e, &s, 0xbf80_0000);
        let default = e.mem.alloc(0x10);
        e.mem.set_u32(0x011f_4748, default);
        match case.word(3) {
            1 => {
                e.mem.set_u32(s.process + 0xc4, 0);
                e.mem.set_u32(default + 8, s.accumulator);
            }
            2 => e.mem.set_u32(s.process + 0xc4, 0),
            _ => {}
        }
        let visible_set = e.mem.alloc(0x10);
        e.mem.set_u32(s.process + 8, 0x1234);
        vec![
            s.process,
            if case.word(0) != 0 { c } else { 0 },
            if case.word(1) != 0 { s.node } else { 0 },
            if case.word(2) != 0 { visible_set } else { 0 },
        ]
    })
    .run()
    .assert_same();
}

/// A float as play has it: its magnitude in 2^-12 to 2^12, so no product or sum of the fade culler
/// leaves the float's normal range (otherwise as [`unit`]).
fn play(bits: u32) -> u32 {
    fold(bits, 115, 24)
}

/// The fade culler's planes: general normals (no two mirror images, every component with a full
/// mantissa) and constants all different, the first the case's.
const FADE_NORMALS: [[f32; 3]; 6] = [
    [0.8, 0.36, 0.48],
    [-0.6, 0.48, 0.64],
    [0.0, -0.8, 0.6],
    [-0.48, -0.36, -0.8],
    [0.36, -0.48, 0.8],
    [-0.64, 0.6, -0.48],
];
const FADE_CONSTANTS: [f32; 5] = [-9.5, -8.25, -10.75, -7.0, -11.5];

/// The fade culler's inputs: a fade node or none; its bound's centre (x, y, z) and radius; the first
/// plane's constant; the bound as it comes or at the edge of its nearest plane.
const FADE: &[Input] = &[
    Input::Flag,
    Input::F32,
    Input::F32Swept(TWO),
    Input::F32Swept(MINUS_TEN),
    Input::F32,
    Input::F32Swept(0xc120_0000),
    Input::Of(&[0, 1]),
];

/// The fade culler's words: the scene's process with [`FADE_NORMALS`] and the constants, the fade node's
/// bound from the case (as play has its floats). At the edge, the radius is minus the smallest of the
/// six distances, each worked out in `f32` as the exe does on the main thread (`(ny cy + nx cx) + cz nz`
/// less the constant): that plane's test is then exactly an equality.
fn fade_words(e: &mut Engine, case: &Case) -> Vec<u32> {
    let s = scene(e);
    let f = |i: usize| f32::from_bits(play(bits(case, i)));
    let constant0 = f(5);
    let planes = s.process + 0x2c;
    for (k, normal) in (0u32..).zip(FADE_NORMALS) {
        let plane = planes + 0x10 * k;
        for (c, v) in (0u32..).zip(normal) {
            e.mem.set_f32(plane + 4 * c, v);
        }
        let constant = if k == 0 {
            constant0
        } else {
            FADE_CONSTANTS[k as usize - 1]
        };
        e.mem.set_f32(plane + 0xc, constant);
    }
    let (cx, cy, cz) = (f(1), f(2), f(3));
    let mut radius = f(4);
    if case.word(6) != 0 {
        let smallest = (0..6)
            .map(|k| {
                let n = FADE_NORMALS[k];
                let constant = if k == 0 {
                    constant0
                } else {
                    FADE_CONSTANTS[k - 1]
                };
                ((n[1] * cy + n[0] * cx) + cz * n[2]) - constant
            })
            .fold(f32::INFINITY, f32::min);
        radius = -smallest;
    }
    let b = bound_of(
        e,
        cx.to_bits(),
        cy.to_bits(),
        cz.to_bits(),
        radius.to_bits(),
    );
    let node = object(e, 0xac, s.stand_in_vt, b, 0x0010_0001);
    let reference = e.mem.alloc(0x20);
    e.mem.set_u32(reference, s.stand_in_vt);
    e.mem
        .set_u32(reference + 0x10, if case.word(0) != 0 { node } else { 0 });
    vec![s.process, reference]
}

/// `BSFadeNodeCuller::Process`: main-thread code in plain `f32`, the exe's 24-bit arithmetic there,
/// so on the main thread's word only, bit for bit (NaN payloads aside), with play's floats. General
/// planes (a symmetric box would mirror a wrong sign away) and bounds at a plane's edge (the behind test
/// an exact equality, which a `>` or a distance rounded otherwise breaks).
#[test]
fn fade_culler_process() {
    Diff::new(entry!(
        0x00bc_2b30,
        process::fade_culler_process(Ptr<_>, u32)
    ))
    .common()
    .main_thread(0)
    .inputs(FADE)
    .max_cases(256)
    .setup(fade_words)
    .run()
    .assert_same();
}

/// A multi-bound box: centre `(cx, 0, 0)`, half extents `(1, 1, hz)`; `answer` is what its
/// `WithinFrustum` stand-in says.
fn aabb(e: &mut Engine, s: &Scene, cx: u32, hz: u32, result: u32, answer: u32) -> u32 {
    let b = e.mem.alloc(0x28);
    e.mem.set_u32(b, s.stand_in_vt);
    e.mem.set_u32(b + 4, 1);
    e.mem.set_u32(b + 8, result);
    e.mem.set_u32(b + 0xc, cx);
    e.mem.set_u32(b + 0x18, ONE);
    e.mem.set_u32(b + 0x1c, ONE);
    e.mem.set_u32(b + 0x20, hz);
    e.mem.set_u32(b + 0x24, answer);
    b
}

#[test]
fn within_frustum() {
    // The box's centre x, its third half extent, the active planes.
    Diff::new(entry!(
        0x00c3_87f0,
        process::within_frustum(Ptr<_>, Ptr<_>) -> bool
    ))
    .common()
    .inputs(&[Input::F32, Input::F32, Input::Of(&[0x3f, 0x2, 0])])
    .max_cases(192)
    .setup(|e, case| {
        let s = scene(e);
        let b = aabb(e, &s, case.f32(0).to_bits(), case.f32(1).to_bits(), 0, 0);
        e.mem.set_u32(s.process + 0x8c, case.word(2));
        vec![b, s.process + 0x2c]
    })
    .run()
    .assert_same();
}

#[test]
fn cull() {
    // Flags: app-culled or not; the process: the real one, or a stand-in.
    Diff::new(entry!(0x00a5_9e00, visible::cull(Ptr<_>, Ptr<_>)))
        .common()
        .inputs(&[Input::Of(&[0, 1, 0x1001, 0x800]), Input::Flag])
        .setup(|e, case| {
            let s = scene(e);
            e.mem.set_u32(s.geometry + 0x30, case.word(0));
            let process = if case.word(1) != 0 {
                s.process
            } else {
                let p = e.mem.alloc(0xc8);
                e.mem.set_u32(p, s.stand_in_vt);
                p
            };
            vec![s.geometry, process]
        })
        .run()
        .assert_same();
}

#[test]
fn node_on_visible() {
    // The node's bound: none or with a radius; the real process or a stand-in.
    Diff::new(entry!(
        0x00a5_dbe0,
        visible::node_on_visible(Ptr<_>, Ptr<_>)
    ))
    .common()
    .inputs(&[Input::Of(&[0, 1]), Input::F32, Input::Flag])
    .setup(|e, case| {
        let s = scene(e);
        let b = if case.word(0) != 0 {
            bound_of(e, 0, 0, 0, case.f32(1).to_bits())
        } else {
            0
        };
        e.mem.set_u32(s.node + 0x20, b);
        let process = if case.word(2) != 0 {
            s.process
        } else {
            let p = e.mem.alloc(0xc8);
            e.mem.set_u32(p, s.stand_in_vt);
            p
        };
        vec![s.node, process]
    })
    .run()
    .assert_same();
}

#[test]
fn geometry_on_visible() {
    Diff::new(entry!(
        0x00a7_fd90,
        visible::geometry_on_visible(Ptr<_>, Ptr<_>)
    ))
    .common()
    // The accumulator's answer; the process: the real one (`AppendVirtual` typed in its slot) or a stand-in.
    .inputs(&[Input::Flag, Input::Flag])
    .setup(|e, case| {
        let s = scene(e);
        e.mem.set_u32(s.accumulator + 0x10, case.word(0));
        let process = if case.word(1) != 0 {
            s.process
        } else {
            let p = e.mem.alloc(0xc8);
            e.mem.set_u32(p, s.stand_in_vt);
            p
        };
        vec![s.geometry, process]
    })
    .run()
    .assert_same();
}

#[test]
fn multibound_node_on_visible() {
    // Process mode; non-null children; a multi-bound; the shape's last verdict; its answer.
    Diff::new(entry!(
        0x00c4_6f60,
        visible::multibound_node_on_visible(Ptr<_>, Ptr<_>)
    ))
    .common()
    .inputs(&[
        Input::Of(&[0, 1, 3, 4, 5]),
        Input::Flag,
        Input::Flag,
        Input::Of(&[0, 1, 2, 7]),
        Input::Flag,
    ])
    .setup(|e, case| {
        let s = scene(e);
        let node = e.mem.alloc(0xb4);
        let bytes = e.mem.bytes(s.node, 0xac).to_vec();
        for (i, b) in bytes.iter().enumerate() {
            e.mem.set_u8(node + i as u32, *b);
        }
        e.mem
            .set_u16(node + 0xa8, if case.word(1) != 0 { 3 } else { 0 });
        let shape = aabb(e, &s, 0, ONE, case.word(3), case.word(4));
        let multibound = e.mem.alloc(0x10);
        e.mem.set_u32(multibound + 0xc, shape);
        e.mem
            .set_u32(node + 0xac, if case.word(2) != 0 { multibound } else { 0 });
        e.mem.set_u32(node + 0xb0, 2);
        e.mem.set_u32(s.process + 0x90, case.word(0));
        vec![node, s.process]
    })
    .run()
    .assert_same();
}

/// `bound_update`'s words for a case of [`bound_inputs`]`(4)`: the source bound, its transform, the
/// bound written.
fn bound_update_words(e: &mut Engine, case: &Case) -> Vec<u32> {
    stand_ins(e);
    let from = bound_of(
        e,
        bits(case, 0),
        bits(case, 4),
        bits(case, 5),
        bits(case, 1),
    );
    let t = e.mem.alloc(0x34);
    bound_transform_at(e, t, case, 4);
    let out = bound_of(e, 0, 0, 0, 0);
    vec![out, from, t]
}

#[test]
fn bound_update() {
    // The source centre's x, its radius, the scale, a matrix entry; the sweep fills the rest of the
    // centre, the matrix and the translation. Under the other threads' control word bit for bit, every
    // input.
    Diff::new(entry!(
        0x00a7_fdd0,
        update::bound_update(Ptr<_>, Ptr<_>, Ptr<_>)
    ))
    .common()
    .other_threads()
    .inputs(&bound_inputs(4))
    .max_cases(384)
    .setup(bound_update_words)
    .run()
    .assert_same();
    // Under the main thread's, every float in 2^-4 to 4 ([`unit`]): each row's dot product is a sum of
    // three products the exe rounds at each step, at most 4 * 2^-24 of the products' sizes (below 48)
    // from the native sum, then times the scale (below 4) and plus the translation. Within 4 ulps, or
    // 2^-13 in value.
    let report = Diff::new(entry!(
        0x00a7_fdd0,
        update::bound_update(Ptr<_>, Ptr<_>, Ptr<_>)
    ))
    .common()
    .main_thread(4)
    .float_abs(2f32.powi(-13))
    .inputs(&bound_inputs(4))
    .max_cases(384)
    .setup(|e, case| bound_update_words(e, &folded(case, unit)))
    .run();
    println!("bound_update on the main thread: {report}");
    report.assert_same();
}

/// `bound_transform`'s words for a case of [`bound_inputs`]`(1)`: the bound, its transform.
fn bound_transform_words(e: &mut Engine, case: &Case) -> Vec<u32> {
    stand_ins(e);
    let b = bound_of(
        e,
        bits(case, 0),
        bits(case, 4),
        bits(case, 5),
        bits(case, 1),
    );
    let t = e.mem.alloc(0x34);
    bound_transform_at(e, t, case, 1);
    vec![b, t]
}

#[test]
fn bound_transform() {
    // The centre's x, the radius, the scale, a matrix entry; the sweep fills the rest of the centre,
    // the matrix and the translation. Under the other threads' control word bit for bit, every input.
    Diff::new(entry!(0x00a7_fea0, update::bound_transform(Ptr<_>, Ptr<_>)))
        .common()
        .other_threads()
        .inputs(&bound_inputs(1))
        .max_cases(384)
        .setup(bound_transform_words)
        .run()
        .assert_same();
    // Under the main thread's, as `bound_update`'s. Within 4 ulps, or 2^-13 in value.
    let report = Diff::new(entry!(0x00a7_fea0, update::bound_transform(Ptr<_>, Ptr<_>)))
        .common()
        .main_thread(4)
        .float_abs(2f32.powi(-13))
        .inputs(&bound_inputs(1))
        .max_cases(384)
        .setup(|e, case| bound_transform_words(e, &folded(case, unit)))
        .run();
    println!("bound_transform on the main thread: {report}");
    report.assert_same();
}

#[test]
fn geometry_update_world_bound() {
    // A world bound or none; flags; the bound's radius; a skin.
    Diff::new(entry!(
        0x00a8_0710,
        update::geometry_update_world_bound(Ptr<_>)
    ))
    .common()
    .inputs(&[
        Input::Flag,
        Input::Of(&[0, 0x0008_0000, 0x0008_2000, 0x0040_0000, 0x0048_0000]),
        Input::Of(FEW_FLOATS),
        Input::Flag,
    ])
    .setup(|e, case| {
        let s = scene(e);
        let g = s.geometry;
        let b = if case.word(0) != 0 {
            bound_of(e, ONE, ONE, ONE, case.word(2))
        } else {
            0
        };
        e.mem.set_u32(g + 0x20, b);
        e.mem.set_u32(g + 0x30, case.word(1));
        e.mem
            .set_u32(g + 0xbc, if case.word(3) != 0 { 0x0300_1000 } else { 0 });
        vec![g]
    })
    .run()
    .assert_same();
}

#[test]
fn set_center_and_radius() {
    // The radius, the centre's x.
    Diff::new(entry!(
        0x004a_51d0,
        update::set_center_and_radius(Ptr<_>, Ptr<_>, f32)
    ))
    .common()
    // A NaN's payload as x87 and SSE pick it differently, and a signalling NaN copied as it is.
    .allow_nan_payloads()
    .inputs(&[
        Input::F32,
        Input::F32,
        Input::F32Swept(ONE),
        Input::F32Swept(TWO),
    ])
    .setup(|e, case| {
        stand_ins(e);
        let b = bound_of(e, 0, 0, 0, 0);
        let c = bound_of(e, bits(case, 1), bits(case, 2), bits(case, 3), 0);
        vec![b, c, case.f32(0).to_bits()]
    })
    .run()
    .assert_same();
}

#[test]
fn ensure_world_bound() {
    Diff::new(entry!(0x0054_67e0, update::ensure_world_bound(Ptr<_>)))
        .common()
        .inputs(&[Input::Flag])
        .setup(|e, case| {
            let s = scene(e);
            if case.word(0) == 0 {
                e.mem.set_u32(s.geometry + 0x20, 0);
            }
            vec![s.geometry]
        })
        .run()
        .assert_same();
}

#[test]
fn node_update_world_bound() {
    // Flags; the first child's radius; the last child's radius; the node's own bound or none.
    // Under the other threads' control word bit for bit, every input.
    Diff::new(entry!(0x00a5_e2d0, update::node_update_world_bound(Ptr<_>)))
        .common()
        .other_threads()
        .inputs(&[
            Input::Of(&[0x0008_0000, 0, 0x0008_2000]),
            Input::F32,
            Input::Of(FEW_FLOATS),
            Input::Flag,
        ])
        .max_cases(256)
        .setup(|e, case| {
            let s = scene(e);
            e.mem.set_u32(s.node + 0x30, case.word(0));
            let first = e.mem.u32(s.children[0] + 0x20);
            e.mem.set_u32(first + 0xc, case.f32(1).to_bits());
            let last = e.mem.u32(s.children[3] + 0x20);
            e.mem.set_u32(last + 0xc, case.word(2));
            if case.word(3) == 0 {
                e.mem.set_u32(s.node + 0x20, 0);
            }
            vec![s.node]
        })
        .run()
        .assert_same();
    // Under the main thread's, the floats in 2^-4 to 4 ([`unit`]), the children's centres small
    // integers and the last child's radius 0, so one merge runs (see `merge`: its squared distance
    // exact, the rest within the tolerance; a second merge would hand the root the first's moved centre,
    // whose last bits are native, as the call's words). Within 4 ulps, or 2^-14 in value.
    let report = Diff::new(entry!(0x00a5_e2d0, update::node_update_world_bound(Ptr<_>)))
        .common()
        .main_thread(4)
        .float_abs(2f32.powi(-14))
        .inputs(&[
            Input::Of(&[0x0008_0000, 0, 0x0008_2000]),
            Input::F32,
            Input::Of(FEW_FLOATS),
            Input::Flag,
        ])
        .max_cases(256)
        .setup(|e, case| {
            let case = &folded(case, unit);
            let s = scene(e);
            e.mem.set_u32(s.node + 0x30, case.word(0));
            let first = e.mem.u32(s.children[0] + 0x20);
            e.mem.set_u32(first + 0xc, case.f32(1).to_bits());
            let last = e.mem.u32(s.children[3] + 0x20);
            e.mem.set_u32(last + 0xc, case.word(2));
            if case.word(3) == 0 {
                e.mem.set_u32(s.node + 0x20, 0);
            }
            e.mem.set_u32(last + 0xc, 0);
            for (child, centre) in [
                (0, [1.0f32, 2.0, 3.0]),
                (1, [4.0, -2.0, 1.0]),
                (3, [-3.0, 5.0, 2.0]),
            ] {
                let bound = e.mem.u32(s.children[child] + 0x20);
                for (k, v) in (0u32..).zip(centre) {
                    e.mem.set_f32(bound + 4 * k, v);
                }
            }
            vec![s.node]
        })
        .run();
    println!("node_update_world_bound on the main thread: {report}");
    report.assert_same();
}

#[test]
fn update_world_transform() {
    // A parent or none; flags; a parent's matrix entry.
    Diff::new(entry!(0x00a6_8bf0, update::update_world_transform(Ptr<_>)))
        .common()
        .inputs(&[Input::Flag, Input::Of(&[0, 0x200]), Input::F32])
        .setup(|e, case| {
            let s = scene(e);
            let g = s.geometry;
            e.mem.set_u32(s.node + 0x68 + 4, case.f32(2).to_bits());
            if case.word(0) == 0 {
                e.mem.set_u32(g + 0x18, 0);
            }
            e.mem.set_u32(g + 0x30, case.word(1));
            vec![g]
        })
        .run()
        .assert_same();
}

#[test]
fn update_world_data() {
    // A collision object or none.
    Diff::new(entry!(0x00a6_8c60, update::update_world_data(Ptr<_>, u32)))
        .common()
        .inputs(&[Input::Flag])
        .setup(|e, case| {
            let s = scene(e);
            if case.word(0) != 0 {
                let collision = e.mem.alloc(0x10);
                e.mem.set_u32(collision, s.stand_in_vt);
                e.mem.set_u32(s.geometry + 0x1c, collision);
            }
            vec![s.geometry, 0x77]
        })
        .run()
        .assert_same();
}

#[test]
fn plane_through() {
    // The normal's y, the point's x.
    // Under the other threads' control word bit for bit, every input.
    Diff::new(entry!(
        0x00a6_9990,
        update::plane_through(Ptr<_>, Ptr<_>, Ptr<_>) -> Ptr
    ))
    .common()
    .other_threads()
    .inputs(&[
        Input::F32,
        Input::F32,
        Input::F32Swept(HALF_F),
        Input::F32Swept(0xbf80_0000),
        Input::F32Swept(TWO),
        Input::F32Swept(MINUS_TEN),
    ])
    .setup(|e, case| {
        stand_ins(e);
        let plane = bound_of(e, 0, 0, 0, 0);
        let n = bound_of(e, bits(case, 2), bits(case, 0), bits(case, 3), 0);
        let p = bound_of(e, bits(case, 1), bits(case, 4), bits(case, 5), 0);
        vec![plane, n, p]
    })
    .run()
    .assert_same();
    // Under the main thread's, the floats in 2^-4 to 4 ([`unit`]): the constant is a sum of three
    // products the exe rounds at each step, at most 4 * 2^-24 of the products' sizes (below 48) from
    // the native sum. Within 4 ulps, or 2^-16 in value.
    let report = Diff::new(entry!(
        0x00a6_9990,
        update::plane_through(Ptr<_>, Ptr<_>, Ptr<_>) -> Ptr
    ))
    .common()
    .main_thread(4)
    .float_abs(2f32.powi(-16))
    .inputs(&[
        Input::F32,
        Input::F32,
        Input::F32Swept(HALF_F),
        Input::F32Swept(0xbf80_0000),
        Input::F32Swept(TWO),
        Input::F32Swept(MINUS_TEN),
    ])
    .setup(|e, case| {
        let case = &folded(case, unit);
        stand_ins(e);
        let plane = bound_of(e, 0, 0, 0, 0);
        let n = bound_of(e, bits(case, 2), bits(case, 0), bits(case, 3), 0);
        let p = bound_of(e, bits(case, 1), bits(case, 4), bits(case, 5), 0);
        vec![plane, n, p]
    })
    .run();
    println!("plane_through on the main thread: {report}");
    report.assert_same();
}

#[test]
fn active_state() {
    // The last plane set in use; the reserved size.
    Diff::new(entry!(0x00c4_9050, compound::active_state(Ptr<_>) -> u32))
        .common()
        .inputs(&[Input::Of(&[0xffff_ffff, 0, 1, 5]), Input::Of(&[0, 1, 2, 8])])
        .setup(|e, case| {
            stand_ins(e);
            let f = compound_frustum(e, 6, 0, 0x2b);
            e.mem.set_u32(f + 0x24, case.word(0));
            e.mem.set_u32(f + 0x10, case.word(1));
            e.mem.set_u32(e.mem.u32(f + 8) + 0x60, 0x11);
            vec![f]
        })
        .run()
        .assert_same();
}

#[test]
fn set_active_state() {
    Diff::new(entry!(0x00c4_9100, compound::set_active_state(Ptr<_>, u32)))
        .common()
        .inputs(&[Input::Of(&[0xffff_ffff, 0, 1, 5]), Input::Of(&[0, 1, 2, 8])])
        .setup(|e, case| {
            stand_ins(e);
            let f = compound_frustum(e, 6, 0, 0x2b);
            e.mem.set_u32(f + 0x24, case.word(0));
            e.mem.set_u32(f + 0x10, case.word(1));
            let state = e.mem.alloc(0x20);
            for i in 0..8 {
                e.mem.set_u32(state + 4 * i, 0x100 + i);
            }
            vec![f, state]
        })
        .run()
        .assert_same();
}

#[test]
fn compound_process() {
    // The object's radius (or no bound); skipping the view frustum; operations in use; the
    // object's centre x.
    Diff::new(entry!(
        0x00c4_91a0,
        compound::process(Ptr<_>, Ptr<_>) -> bool
    ))
    .common()
    .inputs(&[
        Input::Of(&[0x3f80_0000, 0, 0x7fc0_0000, 0xffff_ffff]),
        Input::Flag,
        Input::Of(&[0, 6, 4]),
        Input::F32,
    ])
    .max_cases(256)
    .setup(|e, case| {
        let s = scene(e);
        let b = if case.word(0) == 0xffff_ffff {
            0
        } else {
            bound_of(e, case.f32(3).to_bits(), 0, 0, case.word(0))
        };
        let o = object(e, 0xc8, s.stand_in_vt, b, 0);
        let ops = i32::try_from(case.word(2)).unwrap_or(0);
        let f = compound_frustum(e, ops, u8::from(case.word(1) != 0), 0x3f);
        if ops == 4 {
            // The program starts at the occluder test.
            e.mem.set_u32(f + 0x2c, 2);
        }
        vec![f, o]
    })
    .run()
    .assert_same();
}

#[test]
fn set_camera() {
    // Under the other threads' control word bit for bit, every input.
    Diff::new(entry!(0x00c4_7980, compound::set_camera(Ptr<_>, u32)))
        .common()
        .other_threads()
        .ignore_calls(&[PLANE_THROUGH])
        .inputs(&[Input::Flag, Input::F32])
        .setup(|e, case| {
            let s = scene(e);
            let f = compound_frustum(e, 6, 1, 0x3f);
            e.mem.set_u8(f, 1);
            let c = camera(e, &s, case.f32(1).to_bits());
            vec![f, if case.word(0) != 0 { c } else { 0 }]
        })
        .run()
        .assert_same();
    // Under the main thread's, the floats in 2^-4 to 4 ([`unit`]): the planes as in
    // `frustum_planes_set`. Within 4 ulps, or 2^-12 in value.
    let report = Diff::new(entry!(0x00c4_7980, compound::set_camera(Ptr<_>, u32)))
        .common()
        .main_thread(4)
        .float_abs(2f32.powi(-12))
        .ignore_calls(&[PLANE_THROUGH])
        .inputs(&[Input::Flag, Input::F32])
        .setup(|e, case| {
            let case = &folded(case, unit);
            let s = scene(e);
            let f = compound_frustum(e, 6, 1, 0x3f);
            e.mem.set_u8(f, 1);
            let c = camera(e, &s, case.f32(1).to_bits());
            vec![f, if case.word(0) != 0 { c } else { 0 }]
        })
        .run();
    println!("set_camera on the main thread: {report}");
    report.assert_same();
}

#[test]
fn walk_of_the_scene() {
    // The whole walk from the scene form of Process: every function of the path at once, the
    // node's children inside, outside and on the frustum, with the left edge swept.
    // Under the other threads' control word bit for bit, every input.
    Diff::new(entry!(
        0x00c4_f070,
        process::bs_process_scene(Ptr<_>, u32, Ptr<_>, u32)
    ))
    .common()
    .other_threads()
    .ignore_calls(&[PLANE_THROUGH])
    .inputs(&[Input::F32, Input::Of(FEW_FLOATS)])
    .max_cases(64)
    .setup(|e, case| {
        let s = scene(e);
        let c = camera(e, &s, case.f32(0).to_bits());
        let inner = s.children[3];
        let b = e.mem.u32(inner + 0x20);
        e.mem.set_u32(b + 0xc, case.word(1));
        vec![s.process, c, s.node, 0]
    })
    .run()
    .assert_same();
    // Under the main thread's, the floats in 2^-4 to 4 ([`unit`]): the planes as in
    // `frustum_planes_set`; the walk's verdicts read them. Within 4 ulps, or 2^-12 in value.
    let report = Diff::new(entry!(
        0x00c4_f070,
        process::bs_process_scene(Ptr<_>, u32, Ptr<_>, u32)
    ))
    .common()
    .main_thread(4)
    .float_abs(2f32.powi(-12))
    .ignore_calls(&[PLANE_THROUGH])
    .inputs(&[Input::F32, Input::Of(FEW_FLOATS)])
    .max_cases(64)
    .setup(|e, case| {
        let case = &folded(case, unit);
        let s = scene(e);
        let c = camera(e, &s, case.f32(0).to_bits());
        let inner = s.children[3];
        let b = e.mem.u32(inner + 0x20);
        e.mem.set_u32(b + 0xc, case.word(1));
        vec![s.process, c, s.node, 0]
    })
    .run();
    println!("walk_of_the_scene on the main thread: {report}");
    report.assert_same();
}

#[test]
fn frustum_planes_set() {
    // Orthographic or not; one edge (left, also the near distance); a rotation entry; the
    // translation's x. The plane constructor is done in place. The sweep fills the other edges
    // (arguments 4-7), the other rotation entries (8-15) and the translation's y and z (16, 17).
    // Under the other threads' control word bit for bit, every input.
    Diff::new(entry!(
        0x00a7_4e10,
        planes::frustum_planes_set(Ptr<_>, Ptr<_>, Ptr<_>)
    ))
    .common()
    .other_threads()
    .ignore_calls(&[PLANE_THROUGH])
    .inputs(
        &[Input::Flag, Input::F32, Input::F32, Input::F32]
            .into_iter()
            .chain([ONE, 0x3f40_0000, 0xbf40_0000, 0x447a_0000].map(Input::F32Swept))
            .chain(swept_rotation(0))
            .chain([TWO, MINUS_TEN].map(Input::F32Swept))
            .collect::<Vec<_>>(),
    )
    .max_cases(320)
    .setup(|e, case| {
        stand_ins(e);
        let planes = e.mem.alloc(0x64);
        let frustum = e.mem.alloc(0x1c);
        let edges = [
            bits(case, 1),
            bits(case, 4),
            bits(case, 5),
            bits(case, 6),
            bits(case, 1),
            bits(case, 7),
        ];
        for (i, w) in edges.iter().enumerate() {
            e.mem.set_u32(frustum + 4 * i as u32, *w);
        }
        e.mem.set_u8(frustum + 0x18, u8::from(case.word(0) != 0));
        let t = e.mem.alloc(0x34);
        let m = rotation_from(case, 0, 2, 8);
        transform_at(
            e,
            t,
            m,
            [bits(case, 3), bits(case, 16), bits(case, 17)],
            ONE,
        );
        vec![planes, frustum, t]
    })
    .run()
    .assert_same();
    // Under the main thread's, the floats in 2^-4 to 4 ([`unit`]): each plane's constant is a sum of
    // three products the exe rounds at each step (the products below 160 with the normals and points
    // these floats make), and an edge's length a `double` step rounded twice natively. Within 4 ulps,
    // or 2^-12 in value.
    let report = Diff::new(entry!(
        0x00a7_4e10,
        planes::frustum_planes_set(Ptr<_>, Ptr<_>, Ptr<_>)
    ))
    .common()
    .main_thread(4)
    .float_abs(2f32.powi(-12))
    .ignore_calls(&[PLANE_THROUGH])
    .inputs(
        &[Input::Flag, Input::F32, Input::F32, Input::F32]
            .into_iter()
            .chain([ONE, 0x3f40_0000, 0xbf40_0000, 0x447a_0000].map(Input::F32Swept))
            .chain(swept_rotation(0))
            .chain([TWO, MINUS_TEN].map(Input::F32Swept))
            .collect::<Vec<_>>(),
    )
    .max_cases(320)
    .setup(|e, case| {
        let case = &folded(case, unit);
        stand_ins(e);
        let planes = e.mem.alloc(0x64);
        let frustum = e.mem.alloc(0x1c);
        let edges = [
            bits(case, 1),
            bits(case, 4),
            bits(case, 5),
            bits(case, 6),
            bits(case, 1),
            bits(case, 7),
        ];
        for (i, w) in edges.iter().enumerate() {
            e.mem.set_u32(frustum + 4 * i as u32, *w);
        }
        e.mem.set_u8(frustum + 0x18, u8::from(case.word(0) != 0));
        let t = e.mem.alloc(0x34);
        let m = rotation_from(case, 0, 2, 8);
        transform_at(
            e,
            t,
            m,
            [bits(case, 3), bits(case, 16), bits(case, 17)],
            ONE,
        );
        vec![planes, frustum, t]
    })
    .run();
    println!("frustum_planes_set on the main thread: {report}");
    report.assert_same();
}

/// `within_frustum` with the dispatch family on: its call of the box's corners (`00c39270`) is a Rust
/// call on the direct run. The box's corners copy the half extents as they are (native maths: a
/// signalling NaN keeps its payload).
#[test]
fn within_frustum_with_dispatch() {
    Diff::new(entry!(
        0x00c3_87f0,
        process::within_frustum(Ptr<_>, Ptr<_>) -> bool
    ))
    .with_dispatch()
    .allow_nan_payloads()
    .inputs(&[Input::F32, Input::F32, Input::Of(&[0x3f, 0x2, 0])])
    .max_cases(192)
    .setup(|e, case| {
        let s = scene(e);
        let b = aabb(e, &s, case.f32(0).to_bits(), case.f32(1).to_bits(), 0, 0);
        e.mem.set_u32(s.process + 0x8c, case.word(2));
        vec![b, s.process + 0x2c]
    })
    .run()
    .assert_same();
}

/// `update_world_transform` with the dispatch family on: its call of the transform product
/// (`0062c250`) is a Rust call on the direct run. Under the other threads' control word only: the
/// product is native maths, whose sums of products round once where the exe's 24-bit code rounds each
/// step, so on the main thread's word it parts from the exe in the last bits (the dispatch family's own
/// tests compare it there, within their tolerance); `update_world_transform` runs this function on both
/// words with the product by address.
#[test]
fn update_world_transform_with_dispatch() {
    Diff::new(entry!(0x00a6_8bf0, update::update_world_transform(Ptr<_>)))
        .with_dispatch()
        .control_words(&[0x027f])
        .allow_nan_payloads()
        .inputs(&[Input::Flag, Input::Of(&[0, 0x200]), Input::F32])
        .setup(|e, case| {
            let s = scene(e);
            let g = s.geometry;
            e.mem.set_u32(s.node + 0x68 + 4, case.f32(2).to_bits());
            if case.word(0) == 0 {
                e.mem.set_u32(g + 0x18, 0);
            }
            e.mem.set_u32(g + 0x30, case.word(1));
            vec![g]
        })
        .run()
        .assert_same();
}

// ---- The sums of products in the exe's order ----
//
// The differential tests compare the typed versions with their translations on generated floats, where
// another order of an `f64` sum of float products almost never rounds to another float. These check the
// order with values worked out by hand: the terms 2^31, -2^31 and 1 + 2^-23 cancel exactly to 1 + 2^-23
// when the first two are summed first, and give 1.0 when the last two are (-2^31 + 1 + 2^-23 rounds to
// 53 bits as a tie, to the even -(2^31 - 1)). On an engine with the translations only
// ([`Engine::new_untyped_for_test`]): with the family on, as it is by default, a call by address would
// reach the typed version.

/// Runs `f` under the control word `word` (the thread's, as the CRT keeps it), then back to 53 bits.
fn under(e: &mut Engine, word: u16, f: impl FnOnce(&mut Engine) -> u32) -> u32 {
    engine_core::units::crt::set_x87_control_word(e, word);
    let r = f(e);
    engine_core::units::crt::set_x87_control_word(e, 0x027f);
    r
}

/// 2^31, -2^31 and 1 + 2^-23 as floats.
const CANCEL: [u32; 3] = [0x4f00_0000, 0xcf00_0000, 0x3f80_0001];

/// `plane_through` sums `(ny py + nx px) + nz pz`; `bound_transform` its first row `(e01 y + e00 x) + e02 z`
/// and its others in order, `bound_update` its first row `(e01 y + x e00) + e02 z` (each centre component
/// checked); `frustum_planes_set` the near plane's constant `(n1 t1 + n0 t0) + n2 t2`
/// (its normal the rotation's first column, its point the translation with a near distance of 0).
#[test]
fn sums_in_the_exe_order() {
    let mut e = Engine::new_untyped_for_test();
    stand_ins(&mut e);
    let [big, minus_big, one_up] = CANCEL;
    // The plane through (1, 1, 1) with the normal (nx, ny, nz) = (-2^31, 2^31, 1 + 2^-23).
    let normal = bound_of(&mut e, minus_big, big, one_up, 0);
    let point = bound_of(&mut e, ONE, ONE, ONE, 0);
    // The bound at (1, 1, 1) moved by rows (e00, e01, e02) = (-2^31, 2^31, 1 + 2^-23), (2^31, -2^31,
    // 1 + 2^-23), (2^31, -2^31, 1 + 2^-23), scale 1, no translation.
    let t = e.mem.alloc(0x34);
    transform_at(
        &mut e,
        t,
        [
            minus_big, big, one_up, big, minus_big, one_up, big, minus_big, one_up,
        ],
        [0, 0, 0],
        ONE,
    );
    // The frustum: perspective, edges 1, near 0 (the near plane through the translation (1, 1, 1)); its
    // transform's first column (m00, m10, m20) = (-2^31, 2^31, 1 + 2^-23).
    let frustum = e.mem.alloc(0x1c);
    for (i, w) in [ONE, ONE, ONE, ONE, 0, ONE].iter().enumerate() {
        e.mem.set_u32(frustum + 4 * i as u32, *w);
    }
    let ft = e.mem.alloc(0x34);
    transform_at(
        &mut e,
        ft,
        [minus_big, 0, 0, big, ONE, 0, one_up, 0, ONE],
        [ONE, ONE, ONE],
        ONE,
    );
    for word in [0x007f, 0x027f] {
        let plane = bound_of(&mut e, 0, 0, 0, 0);
        let got = under(&mut e, word, |e| {
            e.call(PLANE_THROUGH, &engine_core::args![plane, normal, point]);
            e.mem.u32(plane + 0xc)
        });
        assert_eq!(
            got, one_up,
            "plane_through, translation, control word {word:04x}"
        );
        let plane = bound_of(&mut e, 0, 0, 0, 0);
        let got = under(&mut e, word, |e| {
            update::plane_through(e, Ptr::new(plane), Ptr::new(normal), Ptr::new(point));
            e.mem.u32(plane + 0xc)
        });
        assert_eq!(got, one_up, "plane_through, typed, control word {word:04x}");
        for typed in [false, true] {
            let b = bound_of(&mut e, ONE, ONE, ONE, ONE);
            under(&mut e, word, |e| {
                if typed {
                    update::bound_transform(e, Ptr::new(b), Ptr::new(t));
                } else {
                    e.call(0x00a7_fea0, &engine_core::args![b, t]);
                }
                0
            });
            for k in 0..3u32 {
                assert_eq!(
                    e.mem.u32(b + 4 * k),
                    one_up,
                    "bound_transform's centre {k}, typed {typed}, control word {word:04x}"
                );
            }
            // `bound_update` from the same bound into another: its first row sums `e01 y + x e00`
            // first.
            let from = bound_of(&mut e, ONE, ONE, ONE, ONE);
            let out = bound_of(&mut e, 0, 0, 0, 0);
            under(&mut e, word, |e| {
                if typed {
                    update::bound_update(e, Ptr::new(out), Ptr::new(from), Ptr::new(t));
                } else {
                    e.call(0x00a7_fdd0, &engine_core::args![out, from, t]);
                }
                0
            });
            for k in 0..3u32 {
                assert_eq!(
                    e.mem.u32(out + 4 * k),
                    one_up,
                    "bound_update's centre {k}, typed {typed}, control word {word:04x}"
                );
            }
            let planes = e.mem.alloc(0x64);
            let constant = under(&mut e, word, |e| {
                if typed {
                    planes::frustum_planes_set(
                        e,
                        Ptr::new(planes),
                        Ptr::new(frustum),
                        Ptr::new(ft),
                    );
                } else {
                    e.call(0x00a7_4e10, &engine_core::args![planes, frustum, ft]);
                }
                e.mem.u32(planes + 0xc)
            });
            assert_eq!(
                constant, one_up,
                "frustum_planes_set, typed {typed}, control word {word:04x}"
            );
        }
    }
}
