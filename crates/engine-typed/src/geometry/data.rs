//! `NiGeometryData`'s setters that take a set of arrays: `Replace` (which gives up the old ones: the
//! renderer's release of the arrays a static mesh does not keep after packing goes through it),
//! `SetData` and `ReplaceData` (which only store them). The first two compute the bound over the new
//! positions.

use super::bound;
use super::layout::GeometryData;
use super::{low_half, FAMILY};
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// The memory manager (`011f6238`) and its free (`00aa4060`, `thiscall`, the block); a group's
/// destructor body (`00a8a670`, `thiscall`) and the sized delete (`00aa1460`, `cdecl`: the block and
/// its size).
const MEMORY_MANAGER: u32 = 0x011f_6238;
const FREE: u32 = 0x00aa_4060;
const GROUP_DESTRUCT: u32 = 0x00a8_a670;
const SIZED_DELETE: u32 = 0x00aa_1460;

/// The data's slots: 0x80 its group (`GetGroup`), 0x90 its active vertex count.
const SLOT_GET_GROUP: u32 = 0x80;
const SLOT_ACTIVE_VERTEX_COUNT: u32 = 0x90;

/// A group's use given up: its count (+0xc) down by one, and at zero the group destroyed and its 16
/// bytes deleted.
fn release_group(e: &mut Engine, group: u32) {
    let count = e.mem.u32(group.wrapping_add(0xc)).wrapping_sub(1);
    e.mem.set_u32(group.wrapping_add(0xc), count);
    if count == 0 {
        e.call(GROUP_DESTRUCT, &args![group]);
        e.call(SIZED_DELETE, &args![group, 0x10u32]);
    }
}

/// The data flags after a set of arrays: the old flags' bits 0x0ffe, the texture set count's bit 0
/// and the low 16 bits of `flags`.
fn data_flags(old: u16, texture_sets: u32, flags: u32) -> u16 {
    (old & 0x0ffe) | low_half(texture_sets & 1) | low_half(flags)
}

// Typed from 00a670c0
/// `NiGeometryData::Replace` (unnamed in the engine map; `thiscall`, `RET 0x1c`, `ReplaceData`'s
/// words): replaces the arrays and gives up the old ones. For each of the four arrays that changes:
/// with a group (slot 0x80) one use of the group is given up (asked again each time); without one the
/// old array is freed (`00aa4060`, a null one too). Then the positions and the count, the normals, the
/// bound (`NiBound::ComputeFromData` with the active vertex count, slot 0x90, when it and the
/// positions are not zero), the colours, the texture coordinates and the data flags.
#[allow(clippy::too_many_arguments)]
pub fn replace(
    e: &mut Engine,
    this: Ptr,
    vertex_count: u32,
    vertices: u32,
    normals: u32,
    colours: u32,
    textures: u32,
    texture_sets: u32,
    flags: u32,
) {
    let data = this.cast::<GeometryData>();
    let base = this.addr();
    let arrays = [
        (GeometryData::m_pkVertex, vertices),
        (GeometryData::m_pkNormal, normals),
        (GeometryData::m_pkColor, colours),
        (GeometryData::m_pkTexture, textures),
    ];
    if e.vcall(base, SLOT_GET_GROUP, &args![]).u32() != 0 {
        for (field, new) in arrays {
            if new != e.get(data, field) {
                let group = e.vcall(base, SLOT_GET_GROUP, &args![]).u32();
                release_group(e, group);
            }
        }
    } else {
        for (field, new) in arrays {
            let old = e.get(data, field);
            if new != old {
                e.call(FREE, &args![MEMORY_MANAGER, old]);
            }
        }
    }
    e.set(data, GeometryData::m_pkVertex, vertices);
    e.set(data, GeometryData::m_usVertices, low_half(vertex_count));
    e.set(data, GeometryData::m_pkNormal, normals);
    let active = e.vcall(base, SLOT_ACTIVE_VERTEX_COUNT, &args![]).u16();
    if active != 0 {
        let positions = e.get(data, GeometryData::m_pkVertex);
        if positions != 0 {
            typed_call!(
                FAMILY,
                0x00a7_ee30,
                bound::compute_from_data(
                    e,
                    Ptr::new(base + 0x10),
                    i32::from(active),
                    Ptr::new(positions)
                )
            );
        }
    }
    e.set(data, GeometryData::m_pkColor, colours);
    let flags = data_flags(
        e.get(data, GeometryData::m_usDataFlags),
        texture_sets,
        flags,
    );
    e.set(data, GeometryData::m_pkTexture, textures);
    e.set(data, GeometryData::m_usDataFlags, flags);
}

// Typed from 00a66b30
/// `NiGeometryData::SetData` (Xbox PDB; `thiscall`, seven words): the count, positions and normals
/// are stored; when the active vertex count (slot 0x90) is not 0 and there are positions the bound is
/// computed from them; then the texture coordinates and colours are stored and the data flags made
/// from the old ones' bits 0x0ffe, the texture set count's bit 0 and the seventh word.
#[allow(clippy::too_many_arguments)]
pub fn set_data(
    e: &mut Engine,
    this: Ptr,
    count: u32,
    vertices: Ptr,
    normals: Ptr,
    colours: Ptr,
    textures: Ptr,
    texture_sets: u32,
    method: u32,
) {
    let data = this.cast::<GeometryData>();
    let base = this.addr();
    e.set(data, GeometryData::m_usVertices, low_half(count));
    e.set(data, GeometryData::m_pkVertex, vertices.addr());
    e.set(data, GeometryData::m_pkNormal, normals.addr());
    let active = e.vcall(base, SLOT_ACTIVE_VERTEX_COUNT, &args![]).u16();
    let positions = e.get(data, GeometryData::m_pkVertex);
    if active != 0 && positions != 0 {
        typed_call!(
            FAMILY,
            0x00a7_ee30,
            bound::compute_from_data(
                e,
                Ptr::new(base + 0x10),
                i32::from(active),
                Ptr::new(positions)
            )
        );
    }
    let old = e.get(data, GeometryData::m_usDataFlags);
    e.set(data, GeometryData::m_pkTexture, textures.addr());
    e.set(data, GeometryData::m_pkColor, colours.addr());
    e.set(
        data,
        GeometryData::m_usDataFlags,
        data_flags(old, texture_sets, method),
    );
}

// Typed from 00a66bb0
/// `NiGeometryData::ReplaceData` (Xbox PDB; `thiscall`, `RET 0x1c`): stores the count and the four
/// arrays and makes the data flags from the old ones' bits 0x0ffe, the texture set count's bit 0 and
/// the low 16 bits of `flags`. Nothing is freed.
#[allow(clippy::too_many_arguments)]
pub fn replace_data(
    e: &mut Engine,
    this: Ptr,
    vertex_count: u32,
    vertices: u32,
    normals: u32,
    colours: u32,
    textures: u32,
    texture_sets: u32,
    flags: u32,
) {
    let data = this.cast::<GeometryData>();
    e.set(data, GeometryData::m_usVertices, low_half(vertex_count));
    e.set(data, GeometryData::m_pkVertex, vertices);
    e.set(data, GeometryData::m_pkNormal, normals);
    e.set(data, GeometryData::m_pkColor, colours);
    e.set(data, GeometryData::m_pkTexture, textures);
    let old = e.get(data, GeometryData::m_usDataFlags);
    e.set(
        data,
        GeometryData::m_usDataFlags,
        data_flags(old, texture_sets, flags),
    );
}
