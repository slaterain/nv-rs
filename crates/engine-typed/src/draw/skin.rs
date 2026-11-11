//! The skinned draw's bone set-up: the skinned shader classes' partition slot ([`bone_setup`], the batch
//! renderer's slot 0x64 call for each drawn partition, about 44 a frame in the crowded street) and the
//! renderer's bone-matrix fill it calls ([`bone_matrices`], which makes about 480 calls a frame there).
//!
//! The bones' world transforms are what the animation update wrote this frame (the packet); the matrix
//! buffer the fill writes, and the block it may allocate for it, are the draw's own.
//!
//! The translation of `00e6fe30` also runs two diagnostics that are not exe behaviour (`NV_SKIN_CHECK`'s
//! bone report and `NV_HEAD_CHECK`'s bone and skinning lines); with this family on those take no notes
//! from here: `NV_TYPED=draw:off` for them.

use super::{own, packet, FAMILY};
use crate::shader::{self, accumulator};
use engine_core::ptr::Ptr;
use engine_core::x87::{x87_load32, x87_m32, x87_mul, x87_quiet_bits, x87_store32, x87_sub};
use engine_core::{args, typed_call, Engine};

/// The bone constants' switch object (`011fec00`): with its byte +8 set, a shader without its own
/// bone constants still uploads them under it.
const BONE_SWITCH: u32 = 0x011f_ec00;
/// The current shader pass (`0126f74c`), whose +0x5c takes the constants (its slot 0x78).
const CURRENT_PASS: u32 = 0x0126_f74c;
/// `"BoneMatrix3"` (`010a1cac`), the name a shader's bone constant handle is asked by (its slot 0x70).
const BONE_MATRIX_NAME: u32 = 0x010a_1cac;

// Typed from 00b7a2c0
/// The skinned shader classes' partition set-up (their slot 0x64), `thiscall(shader, eight words: an
/// unread word, the skin instance, the partition, three unread words, the world transform, an unread
/// word)`: only when the partition has a switch to upload under (a partition, no +0x38, the switch
/// object `011fec00` present with its byte +8 set) or the shader's +0x34 is its own bone constants
/// (+0x80): the renderer's bone matrices for the skin with the world transform (3 registers a bone,
/// the world left out), then the constants of the partition's bones (its 16-bit count +0x20, its bone
/// indices +4) from the skin's buffer (+0x28) to the current pass's constants (+0x5c, slot 0x78),
/// under the shader's "BoneMatrix3" handle (slot 0x70 of +0x80) when they are its own, else under the
/// switch. Answers 0.
#[allow(clippy::too_many_arguments)]
pub fn bone_setup(
    e: &mut Engine,
    this: Ptr,
    _unused_0: u32,
    skin: u32,
    partition: u32,
    _unused_3: u32,
    _unused_4: u32,
    _unused_5: u32,
    world: u32,
    _unused_7: u32,
) -> u32 {
    let t = this.addr();
    let switch = e.global::<u32>(BONE_SWITCH);
    let switched = partition != 0
        && e.mem.u32(t.wrapping_add(0x38)) == 0
        && switch != 0
        && e.mem.u8(switch.wrapping_add(8)) != 0;
    if !switched && e.mem.u32(t.wrapping_add(0x34)) != e.mem.u32(t.wrapping_add(0x80)) {
        return 0;
    }
    let renderer = e.global::<u32>(packet::RENDERER);
    typed_call!(
        FAMILY,
        0x00e6_fe30,
        bone_matrices(
            e,
            Ptr::new(renderer),
            Ptr::new(skin),
            Ptr::new(world),
            0u8,
            3u32,
            1u8
        )
    );
    let own_bones = e.mem.u32(t.wrapping_add(0x80));
    let matrices = e.mem.u32(skin.wrapping_add(0x28));
    let constants = e.mem.u32(e.global::<u32>(CURRENT_PASS).wrapping_add(0x5c));
    let count = u32::from(e.mem.u16(partition.wrapping_add(0x20)));
    let indices = e.mem.u32(partition.wrapping_add(4));
    let head = if e.mem.u32(t.wrapping_add(0x34)) == own_bones {
        e.vcall(own_bones, 0x70, &args![BONE_MATRIX_NAME]).u32()
    } else {
        e.global::<u32>(BONE_SWITCH)
    };
    e.vcall(
        constants,
        0x78,
        &args![head, matrices, count, 3u32, indices],
    );
    0
}

/// `NiD3DHLSLVertexShader::GetEntryPoint` (engine map; `thiscall` on the renderer's +0x8bc).
const ENTRY_POINT: u32 = 0x00e8_bf90;
/// The other matrix builder (`00e6b8f0`, `cdecl(out, rotation, point, scale)`), for the second layout.
const MAKE_MATRIX_SECOND: u32 = 0x00e6_b8f0;
/// The point every bone row's translation is taken from (three floats at `011f474c`).
const ORIGIN: u32 = 0x011f_474c;
/// The lower scale bound (a double) and the upper one (a float).
const SCALE_LOW: u32 = 0x0106_d508;
const SCALE_HIGH: u32 = 0x0106_b9e0;
/// An `NiTransform`'s words (9 rotation floats, the translation, the scale).
const TRANSFORM_WORDS: u32 = 13;

/// Copies an `NiTransform` (13 words) from `from` to `to`.
#[inline(always)]
fn copy_transform(e: &mut Engine, to: u32, from: u32) {
    for i in 0..TRANSFORM_WORDS {
        let word = e.mem.u32(from.wrapping_add(i * 4));
        e.mem.set_u32(to.wrapping_add(i * 4), word);
    }
}

/// `this * other` into `out` (the dispatch family's `NiTransform::operator*`, `0062c250`), answering
/// `out`.
#[inline(always)]
fn compose(e: &mut Engine, this: u32, out: u32, other: u32) -> u32 {
    typed_call!(
        crate::dispatch::FAMILY,
        0x0062_c250,
        crate::dispatch::math::compose(e, Ptr::new(this), Ptr::new(out), Ptr::new(other))
    )
    .addr()
}

/// Whether a bone's scale counts as "not 1": below the double at `0106d508` and above the float at
/// `0106b9e0` at once (`FCOM`, `TEST AH,5` twice).
#[inline(always)]
fn bone_scale_outside(e: &Engine, scale_bits: u32) -> bool {
    let scale = x87_load32(scale_bits);
    let below = scale < e.mem.f64(SCALE_LOW);
    if !below {
        return false;
    }
    let high = x87_load32(e.mem.u32(SCALE_HIGH));
    high < x87_m32(scale_bits)
}

/// A bone's three rows (`rows` points at the second row's start): the rotation times the scale, the
/// translation less the origin at `011f474c`, in the exe's x87 order.
fn write_rows(e: &mut Engine, rows: u32, transform: u32) {
    let f = |e: &Engine, off: u32| x87_load32(e.mem.u32(transform.wrapping_add(off)));
    let s = f(e, 0x30);
    let at = |off: u32| rows.wrapping_add(off);
    // `FMULP ST2`: the entry is the destination.
    e.mem.set_u32(
        at(0u32.wrapping_sub(0x20)),
        x87_store32(x87_mul(f(e, 0), s)),
    );
    for (target, source) in [
        (0u32.wrapping_sub(0x10), 0x0c),
        (0, 0x18),
        (0u32.wrapping_sub(0x1c), 0x04),
        (0u32.wrapping_sub(0xc), 0x10),
        (4, 0x1c),
        (0u32.wrapping_sub(0x18), 0x08),
        (0u32.wrapping_sub(8), 0x14),
    ] {
        e.mem
            .set_u32(at(target), x87_store32(x87_mul(f(e, source), s)));
    }
    let last = x87_m32(e.mem.u32(transform.wrapping_add(0x20)));
    e.mem.set_u32(at(8), x87_store32(x87_mul(s, last)));
    for (target, source, origin) in [
        (0u32.wrapping_sub(0x14), 0x24, ORIGIN),
        (0u32.wrapping_sub(4), 0x28, ORIGIN + 4),
        (0xc, 0x2c, ORIGIN + 8),
    ] {
        let value = x87_sub(f(e, source), x87_m32(e.mem.u32(origin)));
        e.mem.set_u32(at(target), x87_store32(value));
    }
}

// Typed from 00e6fe30
/// The renderer's bone-matrix fill, `thiscall(renderer, skin instance, world transform, layout,
/// registers, skip the world)`: nothing when the skin's +0x18 is the renderer's vertex shader entry
/// point (`00e8bf90` on +0x8bc) and its +0x20 is `registers`. Otherwise the entry point goes to +0x18,
/// the buffer (+0x28) is made anew when the bones (the skin data +8's +0x44) times `registers` times 16
/// bytes outgrow its size (+0x24), +0x1c and +0x20 take the bone count and `registers` (3 sets the
/// layout). With 4 registers each 0x40-byte matrix gets three zeros (+0x30.. with the layout, else the
/// fourth column) and 1 at +0x3c. Then the root `T = data(+0xc) * inverse(skin root's +0x68)` and
/// `W = world * T`; for each bone `B = bone(+0x68) * bind (data +0x40, 0x4c each)`, `B = W * B` unless
/// told to skip the world, its matrix built (4 registers: `00e6b8f0` with the layout, else `00b71a40`)
/// or its three rows written. Last the render state's slot 0xf8 gets whether a scale was outside both
/// bounds (`T`'s, or a bone's +0x98).
///
/// The flag goes out as a pushed word whose upper three bytes are an uninitialised stack slot in the
/// exe; it is passed with them 0, as the translation passes it.
pub fn bone_matrices(
    e: &mut Engine,
    this: Ptr,
    skin: Ptr,
    world: Ptr,
    layout: u8,
    registers: u32,
    skip_world: u8,
) {
    let renderer = this.addr();
    let skin = skin.addr();
    let shader = e.mem.u32(renderer.wrapping_add(0x8bc));
    let current = e.mem.u32(skin.wrapping_add(0x18));
    let entry_point = e.call(ENTRY_POINT, &args![shader]).u32();
    if current == entry_point && registers == e.mem.u32(skin.wrapping_add(0x20)) {
        return;
    }
    let shader = e.mem.u32(renderer.wrapping_add(0x8bc));
    let entry_point = e.call(ENTRY_POINT, &args![shader]).u32();
    let capacity = e.mem.u32(skin.wrapping_add(0x24));
    e.mem.set_u32(skin.wrapping_add(0x18), entry_point);
    let data = e.mem.u32(skin.wrapping_add(8));
    let bones = e.mem.u32(data.wrapping_add(0x44));
    let mut buffer = e.mem.u32(skin.wrapping_add(0x28));
    let layout = if registers == 3 { 1 } else { layout };
    let size = bones.wrapping_mul(registers) << 4;
    if size > capacity {
        buffer = own::renew_bone_buffer(e, skin, buffer, size);
    }
    e.mem.set_u32(skin.wrapping_add(0x1c), bones);
    e.mem.set_u32(skin.wrapping_add(0x20), registers);
    if registers == 4 && bones != 0 {
        let zero = x87_store32(0.0);
        let one = x87_store32(1.0);
        let mut matrix = buffer;
        for _ in 0..bones {
            let cleared = if layout != 0 {
                [0x38, 0x34, 0x30]
            } else {
                [0x2c, 0x1c, 0x0c]
            };
            for offset in cleared {
                e.mem.set_u32(matrix.wrapping_add(offset), zero);
            }
            e.mem.set_u32(matrix.wrapping_add(0x3c), one);
            matrix = matrix.wrapping_add(0x40);
        }
    }
    let world = world.addr();
    e.with_stack(0xb4, |e, frame| {
        // The frame's three transforms ([ESP+0x1c], +0x5c, +0x90).
        let product = frame.addr() + 0x0c;
        let root = frame.addr() + 0x4c;
        let scratch = frame.addr() + 0x80;
        let skin_root = packet::skin_root(e, skin);
        typed_call!(
            crate::dispatch::FAMILY,
            0x004b_4880,
            crate::dispatch::math::invert_non_uniform(
                e,
                Ptr::new(skin_root.wrapping_add(0x68)),
                Ptr::new(product)
            )
        );
        let result = compose(e, data.wrapping_add(0xc), root, product);
        copy_transform(e, product, result);
        let result = compose(e, world, scratch, product);
        // The scale test reads `T`'s scale (+0x30).
        let root_scale = e.mem.u32(product + 0x30);
        copy_transform(e, root, result);
        let mut outside = false;
        let scale = x87_load32(root_scale);
        if scale < e.mem.f64(SCALE_LOW) {
            let high = x87_m32(e.mem.u32(SCALE_HIGH));
            // `FCOMP` with the float, `TEST AH,0x41`: set only when above.
            if scale > high {
                outside = true;
            }
        }
        let bone_list = packet::skin_bones(e, skin);
        let mut bind = e.mem.u32(data.wrapping_add(0x40));
        let mut matrix = buffer;
        let mut rows = buffer.wrapping_add(0x20);
        let mut bone = 0u32;
        while bone < bones {
            let node = packet::bone_node(e, bone_list, bone);
            compose(e, node.wrapping_add(0x68), product, bind);
            let node = packet::bone_node(e, bone_list, bone);
            if bone_scale_outside(e, packet::bone_scale(e, node)) {
                outside = true;
            }
            if skip_world == 0 {
                let result = compose(e, root, scratch, product);
                copy_transform(e, product, result);
            }
            if registers == 4 {
                let scale = x87_quiet_bits(e.mem.u32(product + 0x30));
                if layout != 0 {
                    e.call(
                        MAKE_MATRIX_SECOND,
                        &args![matrix, product, product + 0x24, scale],
                    );
                } else {
                    typed_call!(
                        shader::FAMILY,
                        0x00b7_1a40,
                        accumulator::make_matrix(
                            e,
                            matrix,
                            Ptr::new(product),
                            Ptr::new(product + 0x24),
                            f32::from_bits(scale)
                        )
                    );
                }
            } else {
                write_rows(e, rows, product);
            }
            bind = bind.wrapping_add(0x4c);
            bone += 1;
            matrix = matrix.wrapping_add(0x40);
            rows = rows.wrapping_add(0x30);
        }
        let state = e.mem.u32(renderer.wrapping_add(0x8b8));
        e.vcall(state, 0xf8, &args![u32::from(outside)]);
    });
}
