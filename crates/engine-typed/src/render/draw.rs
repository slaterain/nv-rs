//! The renderer's packed-geometry draw (`00e74840`, its vtable +0x1b8): the Direct3D draw calls of a
//! geometry whose buffers the stream set-up has bound ([`super::stream`]), about 1,150 times a frame in
//! the crowded street, with the renderer's test for a skinned geometry (`00e72c20`) and the shader's
//! "is a pass set" slot it asks first (`00e80580`).
//!
//! `NV_HEAD_CHECK`'s note of the drawn geometry is the translation's: with this on it needs
//! `NV_TYPED=render:off`, as the stream set-up's notes do.

use super::FAMILY;
use crate::draw::capture;
use engine_core::ptr::Ptr;
use engine_core::x87::x87_quiet_bits;
use engine_core::{args, typed_call, Engine};

/// The renderer's fields the draw reads and writes: the byte that skips drawing (+0x6fc), the device
/// (+0x288), the copy of the drawn object's bound (centre at +0x664, radius at +0x670), and for the
/// skinned test the word +0x6f0 and the byte +0x5d5.
const SKIP_DRAWING: u32 = 0x6fc;
const DEVICE: u32 = 0x288;
const DRAWN_BOUND: u32 = 0x664;
/// `IDirect3DDevice9::DrawPrimitive` (slot 81) and `DrawIndexedPrimitive` (slot 82).
const DEVICE_DRAW_PRIMITIVE: u32 = 0x144;
const DEVICE_DRAW_INDEXED_PRIMITIVE: u32 = 0x148;
/// The bound `00e74840` copies when the geometry has none (`011f4288`).
const DEFAULT_BOUND: u32 = 0x011f_4288;
/// The current shader pass (`0126f74c`), and the shader's slot 0x90 that asks whether there is one
/// (`00e80580` in every `NiD3DShader` class).
const CURRENT_PASS: u32 = 0x0126_f74c;
const SLOT_PASS_IS_SET: u32 = 0x90;
const PASS_IS_SET: u32 = 0x00e8_0580;

// Typed from 00e80580
/// Unnamed in the engine map (a shader's vtable +0x90; `this` is not read): 1 when the current pass
/// (`0126f74c`) is set, else 0.
pub fn pass_is_set(e: &mut Engine, _this: u32) -> u32 {
    u32::from(capture::own_at::<u32>(e, CURRENT_PASS) != 0)
}

// Typed from 00e72c20
/// Unnamed in the engine map: `thiscall`, two words (a geometry and an object or 0). False when the
/// geometry's data (+0xb8) has a word at +0x34; true when the chain +0xbc, +0xc, +0xc ends in an
/// object with a word at +0x28 (false when the chain breaks), or when the renderer's virtual +0xa0
/// answers bit 1. Then with the object (or the geometry's +0xc0): true when the word at +0x6f0 is
/// above `0xfffe0000` and its virtual +0xd4 answers true, or when the byte at +0x5d5 is set and its
/// virtual +0xbc answers an object whose virtual +0xa8 answers true; else false.
pub fn is_skinned(e: &mut Engine, this: Ptr, geometry: Ptr, object: u32) -> bool {
    let t = this.addr();
    let g = geometry.addr();
    let data = e.mem.u32(g.wrapping_add(0xb8));
    if e.mem.u32(data.wrapping_add(0x34)) != 0 {
        return false;
    }
    let first = e.mem.u32(g.wrapping_add(0xbc));
    if first == 0 {
        return false;
    }
    let second = e.mem.u32(first.wrapping_add(0xc));
    if second == 0 {
        return false;
    }
    let third = e.mem.u32(second.wrapping_add(0xc));
    if e.mem.u32(third.wrapping_add(0x28)) != 0 {
        return true;
    }
    if e.vcall(t, 0xa0, &args![]).u32() & 2 != 0 {
        return true;
    }
    let mut target = object;
    if target == 0 {
        target = e.mem.u32(g.wrapping_add(0xc0));
    }
    if e.mem.u32(t.wrapping_add(0x6f0)) > 0xfffe_0000
        && target != 0
        && e.vcall(target, 0xd4, &args![]).u8() != 0
    {
        return true;
    }
    if e.mem.u8(t.wrapping_add(0x5d5)) != 0
        && target != 0
        && e.vcall(target, 0xbc, &args![]).u32() != 0
    {
        let inner = e.vcall(target, 0xbc, &args![]).u32();
        if e.vcall(inner, 0xa8, &args![]).u8() != 0 {
            return true;
        }
    }
    false
}

/// One strip (or the whole list) of `data` drawn indexed: its primitive count is the strip's length
/// less 2 from the lengths array (+0x4c), or the count at +0x40 without one. Answers the count.
#[inline(always)]
fn strip_count(e: &Engine, data: u32, strip: u32) -> u32 {
    let lengths = e.mem.u32(data.wrapping_add(0x4c));
    if lengths != 0 {
        u32::from(e.mem.u16(lengths.wrapping_add(strip.wrapping_mul(2)))).wrapping_sub(2)
    } else {
        e.mem.u32(data.wrapping_add(0x40))
    }
}

// Typed from 00e74840
/// Unnamed in the engine map (the renderer's vtable +0x1b8): draws a packed geometry (`thiscall`,
/// one word). Nothing with the byte at +0x6fc set or for data (+0xb8) without vertices (virtual
/// +0x90). The bound (+0x20, or the one at `011f4288`) is copied to +0x664 after the shader's virtual
/// +0x90 is called. A skinned geometry (`00e72c20`) draws each partition's strips (or its triangle
/// list) with `DrawIndexedPrimitive` (type 5 when the first partition has strips, else 4; the
/// partition's base vertex +0x38 of its data, its vertex count, running start indices); else the
/// renderer data (+0x34 of the geometry data) draws indexed (with an index buffer at +0x34) or not
/// (`DrawPrimitive`). Then the data's word +0xe keeps only its top four bits.
pub fn draw_packed(e: &mut Engine, this: Ptr, geometry: Ptr) {
    let t = this.addr();
    let g = geometry.addr();
    if e.mem.u8(t.wrapping_add(SKIP_DRAWING)) != 0 {
        return;
    }
    let data = e.mem.u32(g.wrapping_add(0xb8));
    if crate::draw::shapes::vertex_count(e, data) == 0 {
        return;
    }
    let skin = e.mem.u32(g.wrapping_add(0xbc));
    let bound = match e.mem.u32(g.wrapping_add(0x20)) {
        0 => DEFAULT_BOUND,
        bound => bound,
    };
    let center_y = e.mem.u32(bound.wrapping_add(4));
    let center_z = e.mem.u32(bound.wrapping_add(8));
    let center_x = e.mem.u32(bound);
    let radius = e.mem.u32(bound.wrapping_add(0xc));
    let skinned = is_skinned(e, this, geometry, 0);
    let render_data = e
        .mem
        .u32(e.mem.u32(g.wrapping_add(0xb8)).wrapping_add(0x34));
    let shader = e.mem.u32(g.wrapping_add(0xc0));
    // The answer is not used.
    if crate::slot_target(e, shader, SLOT_PASS_IS_SET) == PASS_IS_SET {
        let _: u32 = typed_call!(FAMILY, 0x00e8_0580, pass_is_set(e, shader));
    } else {
        e.vcall(shader, SLOT_PASS_IS_SET, &args![]);
    }
    e.mem
        .set_u32(t.wrapping_add(DRAWN_BOUND + 0xc), x87_quiet_bits(radius));
    e.mem.set_u32(t.wrapping_add(DRAWN_BOUND), center_x);
    e.mem.set_u32(t.wrapping_add(DRAWN_BOUND + 4), center_y);
    e.mem.set_u32(t.wrapping_add(DRAWN_BOUND + 8), center_z);
    if skinned {
        let instance = e.mem.u32(skin.wrapping_add(0xc));
        let mut remaining = e.mem.u32(instance.wrapping_add(8));
        let partitions = e.mem.u32(instance.wrapping_add(0xc));
        let kind = u32::from(e.mem.u16(partitions.wrapping_add(0x22)) != 0) + 4;
        if remaining != 0 {
            let mut at = partitions.wrapping_add(0x1c);
            loop {
                let part_data = e.mem.u32(at.wrapping_add(0xc));
                let mut strip = 0u32;
                let mut start = 0u32;
                while strip < e.mem.u32(part_data.wrapping_add(0x48)) {
                    let count = strip_count(e, part_data, strip);
                    let vertices = u32::from(e.mem.u16(at));
                    let device = e.mem.u32(t.wrapping_add(DEVICE));
                    let base = e.mem.u32(part_data.wrapping_add(0x38));
                    e.vcall(
                        device,
                        DEVICE_DRAW_INDEXED_PRIMITIVE,
                        &args![kind, base, 0u32, vertices, start, count],
                    );
                    strip += 1;
                    start = start.wrapping_add(count).wrapping_add(2);
                }
                at = at.wrapping_add(0x2c);
                remaining = remaining.wrapping_sub(1);
                if remaining == 0 {
                    break;
                }
            }
        }
    } else if e.mem.u32(render_data.wrapping_add(0x34)) != 0 {
        let mut strip = 0u32;
        let mut start = 0u32;
        while strip < e.mem.u32(render_data.wrapping_add(0x48)) {
            let count = strip_count(e, render_data, strip);
            let vertices = e.mem.u32(render_data.wrapping_add(0x18));
            let device = e.mem.u32(t.wrapping_add(DEVICE));
            let base = e.mem.u32(render_data.wrapping_add(0x38));
            let kind = e.mem.u32(render_data.wrapping_add(0x3c));
            e.vcall(
                device,
                DEVICE_DRAW_INDEXED_PRIMITIVE,
                &args![kind, base, 0u32, vertices, start, count],
            );
            strip += 1;
            start = start.wrapping_add(count).wrapping_add(2);
        }
    } else {
        let count = e.mem.u32(render_data.wrapping_add(0x40));
        let first = e.mem.u32(render_data.wrapping_add(0x38));
        let device = e.mem.u32(t.wrapping_add(DEVICE));
        let kind = e.mem.u32(render_data.wrapping_add(0x3c));
        e.vcall(device, DEVICE_DRAW_PRIMITIVE, &args![kind, first, count]);
    }
    let flags = e.mem.u16(data.wrapping_add(0xe));
    e.mem.set_u16(data.wrapping_add(0xe), flags & 0xf000);
}
