//! A skinned geometry's bound (`NiGeometry::UpdateWorldBound` hands it to the skin instance) and the
//! transform inverse it needs.

use super::FAMILY;
use crate::culling::layout::{NiBound, NiMatrix3, NiPoint3, NiTransform};
use crate::{culling, dispatch, pathing};
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// `NiSkinInstance`: the skin data (+8), the root parent (+0x10), the bones (+0x14).
const SKIN_DATA: u32 = 0x08;
const ROOT_PARENT: u32 = 0x10;
const BONES: u32 = 0x14;
/// `NiSkinData`: the root-parent-to-skin transform (+0xc), the bone data (+0x40) and their count
/// (+0x44); a bone's data is 0x4c bytes with its bound at +0x34.
const DATA_ROOT_TO_SKIN: u32 = 0x0c;
const DATA_BONE_DATA: u32 = 0x40;
const DATA_BONES: u32 = 0x44;
const BONE_DATA_SIZE: u32 = 0x4c;
const BONE_BOUND: u32 = 0x34;
/// An object's world transform (+0x68), its translation (+0x8c) and scale (+0x98).
const WORLD_TRANSFORM: u32 = 0x68;
const WORLD_TRANSLATE: u32 = 0x8c;
const WORLD_SCALE: u32 = 0x98;
/// `NiMatrix3::Inverse` into another matrix (`004b46a0`, `thiscall(this, out)`).
const MATRIX_INVERSE: u32 = 0x004b_46a0;

// Typed from 00a86530
/// The skinned bound (`thiscall(this, bound)`): the first bone's model bound placed by its world
/// transform (`00a7fdd0`), merged (`00a7f3f0`) with every other bone's, moved by the bone's world
/// translation alone when its world scale is exactly 1.0 (an `FCOMP` of 1.0 with the scale) and by
/// its whole world transform (`00a7fdd0`) otherwise; then moved into skin space (`00a7fea0`) by the
/// root parent's inverse world transform (`008b42e0`) times the data's root-parent-to-skin transform
/// (`0062c250`). The locals are the exe's: a bound (0x10) and two transforms (0x68).
pub fn skin_bound(e: &mut Engine, this: Ptr, bound: Ptr) {
    let base = this.addr();
    let bones = e.mem.u32(base + BONES);
    let data = e.mem.u32(base + SKIN_DATA);
    let first_bone = e.mem.u32(bones);
    let bone_data = e.mem.u32(data + DATA_BONE_DATA);
    typed_call!(
        culling::FAMILY,
        0x00a7_fdd0,
        culling::update::bound_update(
            e,
            bound.cast::<NiBound>(),
            Ptr::new(bone_data.wrapping_add(BONE_BOUND)),
            Ptr::new(first_bone.wrapping_add(WORLD_TRANSFORM))
        )
    );
    let count = e.mem.u32(e.mem.u32(base + SKIN_DATA) + DATA_BONES);
    e.with_stack(0x10, |e, local| {
        let local = local.addr();
        let mut source = bone_data.wrapping_add(BONE_DATA_SIZE + BONE_BOUND);
        let mut index = 1u32;
        while index < count {
            let bone = e.mem.u32(e.mem.u32(base + BONES).wrapping_add(4 * index));
            if e.mem.f32(bone.wrapping_add(WORLD_SCALE)) == 1.0 {
                // The radius passes through the x87 stack in the exe: copied plain.
                let radius = e.mem.u32(source + 0xc);
                e.mem.set_u32(local + 0xc, radius);
                for axis in 0..3 {
                    let value = e.mem.u32(source + 4 * axis);
                    e.mem.set_u32(local + 4 * axis, value);
                }
                // Each axis one sum of two floats, stored: plain `f32` on every thread (ADR-0014).
                let mut moved = [0f32; 3];
                for (axis, slot) in (0u32..).zip(moved.iter_mut()) {
                    *slot = e.mem.f32(bone.wrapping_add(WORLD_TRANSLATE + 4 * axis))
                        + e.mem.f32(local + 4 * axis);
                }
                for (axis, value) in (0u32..).zip(moved) {
                    e.mem.set_u32(local + 4 * axis, value.to_bits());
                }
            } else {
                typed_call!(
                    culling::FAMILY,
                    0x00a7_fdd0,
                    culling::update::bound_update(
                        e,
                        Ptr::new(local),
                        Ptr::new(source),
                        Ptr::new(bone.wrapping_add(WORLD_TRANSFORM))
                    )
                );
            }
            typed_call!(
                culling::FAMILY,
                0x00a7_f3f0,
                culling::bound::merge(e, bound.cast::<NiBound>(), Ptr::new(local))
            );
            index += 1;
            source = source.wrapping_add(BONE_DATA_SIZE);
        }
    });
    e.with_stack(0x68, |e, transforms| {
        let inverse = transforms.addr();
        let product = transforms.addr() + 0x34;
        let root = e.mem.u32(base + ROOT_PARENT);
        typed_call!(
            FAMILY,
            0x008b_42e0,
            transform_inverse(
                e,
                Ptr::new(root.wrapping_add(WORLD_TRANSFORM)),
                Ptr::new(inverse)
            )
        );
        let data = e.mem.u32(base + SKIN_DATA);
        let result = typed_call!(
            dispatch::FAMILY,
            0x0062_c250,
            dispatch::math::compose(
                e,
                Ptr::<NiTransform>::new(data.wrapping_add(DATA_ROOT_TO_SKIN)),
                Ptr::new(product),
                Ptr::new(inverse)
            )
        )
        .addr();
        let bytes = e.mem.bytes(result, 0x34);
        e.mem.write(inverse, &bytes);
        typed_call!(
            culling::FAMILY,
            0x00a7_fea0,
            culling::update::bound_transform(e, bound.cast::<NiBound>(), Ptr::new(inverse))
        );
    });
}

/// The reciprocal of `scale`: one division of floats stored at once, plain `f32` on every thread
/// (ADR-0014; the translation's `f64` quotient rounded to a float is the same correctly rounded float).
fn reciprocal(scale: f32) -> f32 {
    1.0 / scale
}

// Typed from 008b42e0
/// `NiTransform::Invert` (`thiscall(this, out)`): the rotation's inverse (`004b46a0`), the scale's
/// reciprocal, and the translation `reciprocal * (out.rotation * -translation)` (`004a0bd0`,
/// `004b4500`, `004a3760`) through a 0x30-byte local.
pub fn transform_inverse(e: &mut Engine, this: Ptr, out: Ptr) {
    e.call(MATRIX_INVERSE, &args![this, out]);
    let scale = e.mem.f32(this.addr() + 0x30);
    let inverse_scale = reciprocal(scale);
    e.mem.set_f32(out.addr() + 0x30, inverse_scale);
    e.with_stack(0x30, |e, scratch| {
        let first = scratch.addr();
        let negated = typed_call!(
            pathing::FAMILY,
            0x004a_0bd0,
            pathing::point::negated(e, Ptr::<NiPoint3>::new(this.addr() + 0x24), Ptr::new(first))
        );
        let rotated = typed_call!(
            dispatch::FAMILY,
            0x004b_4500,
            dispatch::math::rotate(
                e,
                Ptr::<NiMatrix3>::new(out.addr()),
                Ptr::new(first + 0x0c),
                negated
            )
        );
        let scaled = typed_call!(
            pathing::FAMILY,
            0x004a_3760,
            pathing::point::scaled(e, Ptr::new(first + 0x18), inverse_scale, rotated)
        )
        .addr();
        for offset in [0, 4, 8] {
            let word = e.mem.u32(scaled + offset);
            e.mem.set_u32(out.addr() + 0x24 + offset, word);
        }
    });
}
