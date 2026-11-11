//! The vertex shader's constant setters (group B): `NiD3DVertexShader::SetShaderConstant` and
//! `SetShaderConstantArray` (the vertex shader's slots 0x74 and 0x78), which the shader constant maps
//! call for each constant they upload. A constant goes to the entry's own buffer when it has one,
//! otherwise to the device (condition 2: the constants the next draw reads).

use super::own;
use engine_core::ptr::Ptr;
use engine_core::Engine;

/// The constant kinds (`011f5fc4`, a word for each low byte of an entry's flags).
const CONSTANT_KINDS: u32 = 0x011f_5fc4;
/// The renderer's device (+0x10f8 of the shader's renderer, +0x28).
const RENDERER_DEVICE: u32 = 0x10f8;
/// `IDirect3DDevice9::SetVertexShaderConstantF`, `...I` and `...B` (slots 94, 96 and 98).
const SET_CONSTANTS_F: u32 = 0x178;
const SET_CONSTANTS_I: u32 = 0x180;
const SET_CONSTANTS_B: u32 = 0x188;

/// A constant map entry's fields: its flags (+0x14), register (+0x1c), register count (+0x20), size in
/// bytes (+0x28) and data (+0x30, its own buffer when the flags' top nibble is 2).
const ENTRY_FLAGS: u32 = 0x14;
const ENTRY_REGISTER: u32 = 0x1c;
const ENTRY_COUNT: u32 = 0x20;
const ENTRY_SIZE: u32 = 0x28;
const ENTRY_DATA: u32 = 0x30;

/// The device of the shader's renderer.
#[inline(always)]
fn device(e: &Engine, shader: u32) -> u32 {
    let renderer = e.mem.u32(shader.wrapping_add(0x28));
    e.mem.u32(renderer.wrapping_add(RENDERER_DEVICE))
}

/// The entry's constant kind (the table's word for the low byte of its flags).
#[inline(always)]
fn constant_kind(e: &Engine, entry: u32) -> u32 {
    let flags = e.mem.u32(entry.wrapping_add(ENTRY_FLAGS));
    e.mem.u32(CONSTANT_KINDS.wrapping_add((flags & 0xff) * 4))
}

/// Whether the entry keeps its constant in its own buffer (the flags' top nibble 2, a buffer).
#[inline(always)]
fn has_buffer(e: &Engine, entry: u32) -> bool {
    e.mem.u32(entry.wrapping_add(ENTRY_FLAGS)) & 0xf000_0000 == 0x2000_0000
        && e.mem.u32(entry.wrapping_add(ENTRY_DATA)) != 0
}

/// `size` bytes from `source` into the entry's buffer at `offset` (through [`own`]).
#[inline(always)]
fn copy_into_buffer(e: &mut Engine, entry: u32, offset: u32, source: u32) {
    let buffer = e.mem.u32(entry.wrapping_add(ENTRY_DATA));
    let size = e.mem.u32(entry.wrapping_add(ENTRY_SIZE));
    own::copy_constants(e, offset.wrapping_add(buffer), source, size);
}

// Typed from 00e89ba0
/// `NiD3DVertexShader::SetShaderConstant` (Xbox PDB; the vertex shader's slot 0x74), `thiscall(shader,
/// entry, data, count)`: no data means the entry's (+0x30), no count its +0x20 (1 when that is 0). An
/// entry with its own buffer gets the data copied in (its size +0x28) and answers true; otherwise the
/// device's boolean (kind 1), integer (kind 3) or float (any other kind) constants get the entry's
/// register (+0x1c), the data and the count, and the answer is whether the result is not negative.
pub fn set_constant(e: &mut Engine, this: Ptr, entry: Ptr, data: u32, count: u32) -> bool {
    let entry = entry.addr();
    let data = if data == 0 {
        e.mem.u32(entry.wrapping_add(ENTRY_DATA))
    } else {
        data
    };
    let mut count = count;
    if count == 0 {
        count = e.mem.u32(entry.wrapping_add(ENTRY_COUNT));
        if count == 0 {
            count = 1;
        }
    }
    let kind = constant_kind(e, entry);
    if has_buffer(e, entry) {
        copy_into_buffer(e, entry, 0, data);
        return true;
    }
    let register = e.mem.u32(entry.wrapping_add(ENTRY_REGISTER));
    let device = device(e, this.addr());
    let slot = match kind {
        1 => SET_CONSTANTS_B,
        3 => SET_CONSTANTS_I,
        _ => SET_CONSTANTS_F,
    };
    let result = own::device_vertex_constants_of(e, device, slot, register, data, count);
    i32::from_ne_bytes(result.to_ne_bytes()) >= 0
}

// Typed from 00e89c70
/// `NiD3DVertexShader::SetShaderConstantArray` (Xbox PDB; the vertex shader's slot 0x78),
/// `thiscall(shader, entry, data, count, registers, indices)`: only for entries of kind 8 or 9 (else
/// false). For each of the `count` 16-bit indices the source is `data + index · registers · 16`: an
/// entry with its own buffer gets it copied to `buffer + i · registers · 16` (its size +0x28), otherwise
/// the device's float constants get the register (+0x1c, `registers` further each time), the source
/// and `registers`. Answers true.
pub fn set_constant_array(
    e: &mut Engine,
    this: Ptr,
    entry: Ptr,
    data: u32,
    count: u32,
    registers: u32,
    indices: u32,
) -> bool {
    let entry = entry.addr();
    let kind = constant_kind(e, entry);
    if kind != 8 && kind != 9 {
        return false;
    }
    let mut register = e.mem.u32(entry.wrapping_add(ENTRY_REGISTER));
    let stride = registers << 4;
    let mut offset = 0u32;
    let mut i = 0u32;
    while i < count {
        let index = u32::from(e.mem.u16(indices.wrapping_add(i * 2)));
        let source = (index.wrapping_mul(registers) << 4).wrapping_add(data);
        if has_buffer(e, entry) {
            copy_into_buffer(e, entry, offset, source);
        } else {
            let device = device(e, this.addr());
            own::device_vertex_constants_of(
                e,
                device,
                SET_CONSTANTS_F,
                register,
                source,
                registers,
            );
        }
        offset = offset.wrapping_add(stride);
        i += 1;
        register = register.wrapping_add(registers);
    }
    true
}
