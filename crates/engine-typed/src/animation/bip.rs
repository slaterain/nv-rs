//! The bip node's controller walk and a multi-target transform controller's bone updates, with the
//! small tests and setters they make on the controller, the transform and the bone.

use super::FAMILY;
use crate::{dispatch, pathing};
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::{args, typed_call, Engine};

/// The `NiRTTI` records of `NiMultiTargetTransformController` (`011f36bc`) and
/// `BSMultiTargetTreadTransfController` (`011f36e4`).
const MULTI_TARGET_TRANSFORM_RTTI: u32 = 0x011f_36bc;
const TREAD_TRANSFORM_RTTI: u32 = 0x011f_36e4;
/// The update data's constructor (`0043d410`, `thiscall(data, time, a, b)`, 0xc bytes).
const UPDATE_DATA_CONSTRUCTOR: u32 = 0x0043_d410;
/// `BSTransCon::UpdateAllButTarget` (`004f0380`, this family's [`update_all_but_target`]) and
/// `NiBlendTransformInterpolator::Update` (`00a41110`, the blend's, in the interpolators' slot
/// +0x8c): by address only, since kNVSE hooks both (`overridable.tsv`), so a built-in may replace them.
const UPDATE_ALL_BUT_TARGET: u32 = 0x004f_0380;
/// Vtable slots: `GetRTTI` (+8), a controller's `Update` (+0x94), an interpolator's transform
/// `Update` (+0x8c).
const SLOT_RTTI: u32 = 8;
const SLOT_UPDATE: u32 = 0x94;
const SLOT_TRANSFORM_UPDATE: u32 = 0x8c;
/// The multi-target controller's interpolators (+0x34, 0x30 bytes each, inline), targets (+0x38)
/// and target count (+0x3c, 16 bits).
const INTERPOLATORS: u32 = 0x34;
const INTERPOLATOR_SIZE: u32 = 0x30;
const TARGETS: u32 = 0x38;
const TARGET_COUNT: u32 = 0x3c;
/// The transform's constructor (`00494260`), its translation's test (`004942c0`) and the rotation
/// and scale getters (`0048d150`, `00495460` in ST0); the bone's translation and scale setters
/// (`00440460`, `00440490`).
const TRANSFORM_CONSTRUCTOR: u32 = 0x0049_4260;
const TRANSLATE_VALID: u32 = 0x0049_42c0;
const ROTATE_OF: u32 = 0x0048_d150;
const SCALE_OF: u32 = 0x0049_5460;
const SET_TRANSLATE: u32 = 0x0044_0460;
const SET_SCALE: u32 = 0x0044_0490;
/// `FLT_MAX` (`01023900`, a float) and `2.0` (`01011590`, a double).
const LARGEST: u32 = 0x0102_3900;
const TWO: u32 = 0x0101_1590;
/// A matrix column set from three floats (`004b4660`, `thiscall(matrix, column, x, y, z)`).
const MATRIX_SET_COLUMN: u32 = 0x004b_4660;

// Typed from 004f0040
/// `BSBipNode::UpdateAllButBip` (`thiscall(node, time)`): for each of the node's controllers (the
/// first `0043b230`, the next `004a8a90`), by its `GetRTTI` (slot 8): a multi-target transform
/// controller updates its bones (`004f0380`, with `time` and 0); a tread transform
/// controller gets `Update` (slot +0x94) with update data for `time`.
pub fn update_all_but_bip(e: &mut Engine, this: Ptr, time: f32) {
    let time_bits = time.to_bits();
    let mut controller = typed_call!(
        dispatch::FAMILY,
        0x0043_b230,
        dispatch::scene::first_controller(e, this.cast())
    )
    .addr();
    while controller != 0 {
        if e.vcall(controller, SLOT_RTTI, &args![]).u32() == MULTI_TARGET_TRANSFORM_RTTI {
            e.call(UPDATE_ALL_BUT_TARGET, &args![controller, time_bits, 0u32]);
        } else if e.vcall(controller, SLOT_RTTI, &args![]).u32() == TREAD_TRANSFORM_RTTI {
            e.with_stack(0xc, |e, data| {
                e.call(UPDATE_DATA_CONSTRUCTOR, &args![data, time_bits, 0u32, 0u32]);
                e.vcall(controller, SLOT_UPDATE, &args![data]);
            });
        }
        controller = typed_call!(
            dispatch::FAMILY,
            0x004a_8a90,
            dispatch::scene::next_controller(e, Ptr::new(controller))
        );
    }
}

/// Bone `index` from its interpolator's transform at the time (the interpolator's slot +0x8c): its
/// translation, rotation and scale when valid. False when the interpolator gave nothing.
fn update_target(
    e: &mut Engine,
    this: Ptr,
    index: u32,
    time_bits: u32,
    target: u32,
    transform: Ptr,
) -> bool {
    let interpolators = e.mem.u32(this.addr().wrapping_add(INTERPOLATORS));
    let interpolator = interpolators.wrapping_add(index.wrapping_mul(INTERPOLATOR_SIZE));
    let gave = e
        .vcall(
            interpolator,
            SLOT_TRANSFORM_UPDATE,
            &args![time_bits, target, transform],
        )
        .bool();
    if !gave {
        return false;
    }
    if e.call(TRANSLATE_VALID, &args![transform]).bool() {
        let translate = typed_call!(leaf::FAMILY, 0x0068_15c0, leaf::same_word(e, transform));
        e.call(SET_TRANSLATE, &args![target, translate]);
    }
    if typed_call!(FAMILY, 0x004f_02e0, rotation_valid(e, transform)) {
        let rotate = e.call(ROTATE_OF, &args![transform]).ptr();
        typed_call!(
            FAMILY,
            0x004f_0160,
            set_rotation(e, Ptr::new(target), rotate)
        );
    }
    if typed_call!(FAMILY, 0x004f_0320, scale_valid(e, transform)) {
        let scale = e.call(SCALE_OF, &args![transform]).f64();
        e.call(SET_SCALE, &args![target, to_float(scale).to_bits()]);
    }
    true
}

// Typed from 004f0380
/// `BSTransCon::UpdateAllButTarget` (`thiscall(controller, time, inverse)`): when active
/// ([`controller_active`]) with targets, each bone but the last (non-null) whose flag 2
/// ([`flag_2`]) is set when `inverse` is 0, or clear when it is not, is updated from its
/// interpolator's transform; then the last bone, when non-null and (with `inverse` 0) flag 2 is
/// set. One transform (`00494260`, on the stack) is reused for all.
pub fn update_all_but_target(e: &mut Engine, this: Ptr, time: f32, inverse: u8) {
    if typed_call!(FAMILY, 0x004f_05a0, controller_active(e, this)) & 0xff == 0
        || e.mem.u16(this.addr().wrapping_add(TARGET_COUNT)) == 0
    {
        return;
    }
    let time_bits = time.to_bits();
    e.with_stack(0x20, |e, transform| {
        e.call(TRANSFORM_CONSTRUCTOR, &args![transform]);
        let last = i32::from(e.mem.u16(this.addr().wrapping_add(TARGET_COUNT))) - 1;
        let target_at = |e: &Engine, i: i32| {
            let targets = e.mem.u32(this.addr().wrapping_add(TARGETS));
            e.mem.u32(targets.wrapping_add((i as u32).wrapping_mul(4)))
        };
        let mut i = 0i32;
        while i < last {
            let target = target_at(e, i);
            let pick = target != 0 && {
                let set = typed_call!(FAMILY, 0x004f_0140, flag_2(e, Ptr::new(target)));
                if inverse == 0 {
                    set
                } else {
                    !set
                }
            };
            if pick {
                update_target(e, this, i as u32, time_bits, target, transform);
            }
            i += 1;
        }
        let target = target_at(e, i);
        if target == 0
            || (inverse == 0 && !typed_call!(FAMILY, 0x004f_0140, flag_2(e, Ptr::new(target))))
        {
            return;
        }
        update_target(e, this, i as u32, time_bits, target, transform);
    });
}

// Typed from 004f05a0
/// `thiscall(controller)`: 1 when the controller's 16-bit flags word (+8) shares a bit with the
/// mask 8 ([`flags_share`]), else 0.
pub fn controller_active(e: &mut Engine, this: Ptr) -> u32 {
    typed_call!(FAMILY, 0x004f_0360, flags_share(e, this, 8))
}

// Typed from 004f0360
/// `thiscall(object, mask)`: 1 when the 16-bit word at +8 shares a bit with the low 16 bits of
/// `mask`, else 0.
pub fn flags_share(e: &mut Engine, this: Ptr, mask: u32) -> u32 {
    let flags = e.mem.u16(this.addr().wrapping_add(8));
    u32::from(flags & low_16(mask) != 0)
}

/// A float store of an `f64` local (ADR-0014's float rule): rounded to the nearest float.
#[allow(clippy::cast_possible_truncation)]
fn to_float(value: f64) -> f32 {
    value as f32
}

/// The low 16 bits of a word, as the exe's 16-bit `TEST` takes them.
#[allow(clippy::cast_possible_truncation)]
fn low_16(word: u32) -> u16 {
    word as u16
}

// Typed from 004f0140
/// `thiscall(object)`: whether the object's flags (+0x30) have bit 2 (`00456630`).
pub fn flag_2(e: &mut Engine, this: Ptr) -> bool {
    typed_call!(
        leaf::FAMILY,
        0x0045_6630,
        leaf::flags_at_30_any(e, this, 2u32)
    )
}

// Typed from 004f02e0
/// `thiscall(transform)`: the rotation's test: whether its second float (`006b9130` on `this +
/// 0xc`, the pathing family's) is not `-FLT_MAX` (a NaN counts as not equal).
pub fn rotation_valid(e: &mut Engine, this: Ptr) -> bool {
    let value = typed_call!(
        pathing::FAMILY,
        0x006b_9130,
        pathing::search::second_float(e, this.byte_add(0xc).cast())
    );
    value != -f64::from(f32::from_bits(e.mem.u32(LARGEST)))
}

// Typed from 004f0320
/// `thiscall(transform)`: the scale's test: whether the float at +0x1c is not `-FLT_MAX` (a NaN
/// counts as not equal).
pub fn scale_valid(e: &mut Engine, this: Ptr) -> bool {
    f32::from_bits(e.mem.u32(this.addr().wrapping_add(0x1c))) != -f32::from_bits(e.mem.u32(LARGEST))
}

// Typed from 004f0160
/// `thiscall(bone, quaternion)`: the bone's rotation matrix (+0x34) from the quaternion
/// ([`to_rotation`]).
pub fn set_rotation(e: &mut Engine, this: Ptr, quaternion: Ptr) {
    typed_call!(
        FAMILY,
        0x004f_0180,
        to_rotation(e, quaternion, this.byte_add(0x34))
    );
}

// Typed from 004f0180
/// `NiQuaternion::ToRotation` (`thiscall(quaternion, matrix)`): the rotation of the quaternion
/// `(w, x, y, z)` written column by column (`004b4660`): with `x2 = 2x`, `y2 = 2y`, `z2 = 2z` and the
/// products `wx = x2 w` .. `zz = z2 z` (each rounded to a float, as the exe stores them), column 0
/// is `(1 - (yy + zz), xy + wz, xz - wy)`, column 1 `(xy - wz, 1 - (xx + zz), yz + wx)`, column 2
/// `(xz + wy, yz - wx, 1 - (xx + yy))`, each column's third value computed first.
///
/// Native float maths (ADR-0014's float rule; the animation runs on the AI threads as well as main):
/// `f64` locals, rounded to a float at each of the exe's float stores. A product of two floats is
/// exact in an `f64`, so the products round once as the exe's do on every thread; a sum of two can
/// round twice on main's 24-bit precision where the exe rounds once, within an ulp.
pub fn to_rotation(e: &mut Engine, this: Ptr, matrix: Ptr) {
    let q = this.addr();
    let two = f64::from_bits(e.global::<u64>(TWO));
    let load = |e: &Engine, at: u32| f64::from(f32::from_bits(e.mem.u32(at)));
    let (w, x, y, z) = (load(e, q), load(e, q + 4), load(e, q + 8), load(e, q + 0xc));
    let (x2, y2, z2) = (to_float(x * two), to_float(y * two), to_float(z * two));
    let product = |factor: f32, by: f64| f64::from(to_float(f64::from(factor) * by));
    let (wx, wy, wz) = (product(x2, w), product(y2, w), product(z2, w));
    let (xx, xy, xz) = (product(x2, x), product(y2, x), product(z2, x));
    let (yy, yz, zz) = (product(y2, y), product(z2, y), product(z2, z));
    let sub = |a: f64, b: f64| to_float(a - b);
    let add = |a: f64, b: f64| to_float(a + b);
    let one_minus = |a: f64, b: f64| to_float(1.0 - (a + b));
    let columns = [
        [one_minus(yy, zz), add(xy, wz), sub(xz, wy)],
        [sub(xy, wz), one_minus(xx, zz), add(yz, wx)],
        [add(xz, wy), sub(yz, wx), one_minus(xx, yy)],
    ];
    for (column, [c0, c1, c2]) in (0u32..).zip(columns) {
        e.call(
            MATRIX_SET_COLUMN,
            &args![matrix, column, c0.to_bits(), c1.to_bits(), c2.to_bits()],
        );
    }
}
