//! Where `NiGeometryData`'s arrays begin and end: the load from a stream (`LoadBinary`, which makes
//! the arrays and reads them; the deep copy FaceGen makes each head with loads through it too), the
//! save to a stream (`SaveBinary`, the deep copy's other half, with its texture coordinates' write),
//! the destructor body (which gives the arrays up), and the normals' own lifecycle (`CreateNormals`,
//! the normal method's setter, which moves them into a new array, and their release).
//!
//! A stream's reads and writes are calls by address through the binary stream's function words
//! (`m_pfnRead`, `m_pfnWrite`), with the element size in a stack word, as the exe makes them.

use super::data::release_group;
use super::layout::{BinaryStream, GeometryData, Stream};
use super::{low_byte, low_half};
use engine_core::ptr::Ptr;
use engine_core::{args, Engine};

/// The memory manager (`011f6238`) and its allocate and free (`00aa3e40`, `00aa4060`, `thiscall`,
/// one word each).
const MEMORY_MANAGER: u32 = 0x011f_6238;
const ALLOCATE: u32 = 0x00aa_3e40;
const FREE: u32 = 0x00aa_4060;
/// `NiGeometryData`'s static group (`m_pkGroup`, `011f4880`), its vtable (`0109bf9c`), and the
/// object at `011f4748` the destructor hands `this` to (its slot 0xf0).
const GEOMETRY_GROUP: u32 = 0x011f_4880;
const VTABLE: u32 = 0x0109_bf9c;
const RENDERER: u32 = 0x011f_4748;
/// `InterlockedDecrement` (import slot `00fdf1c4`, `stdcall`, the address).
const INTERLOCKED_DECREMENT: u32 = 0x00fd_f1c4;
/// `NiObject`'s `LoadBinary` (`00a5d610`) and destructor body (`00a5d3d0`), the base step
/// `SaveBinary` starts with (`00a89af0`), `NiStream::GetGroupFromID` (`00a634e0`) and the group's id
/// (`00a634f0`), the link step from 10.3.0.7 (`00a645b0`), `NiSphere`'s `LoadBinary` and
/// `SaveBinary` (`00a69ac0`, `00eb5410`), all `thiscall`.
const OBJECT_LOAD_BINARY: u32 = 0x00a5_d610;
const OBJECT_DESTRUCT: u32 = 0x00a5_d3d0;
const OBJECT_SAVE_STEP: u32 = 0x00a8_9af0;
const GET_GROUP_FROM_ID: u32 = 0x00a6_34e0;
const GROUP_ID: u32 = 0x00a6_34f0;
const LINK_STEP: u32 = 0x00a6_45b0;
const SPHERE_LOAD_BINARY: u32 = 0x00a6_9ac0;
const SPHERE_SAVE_BINARY: u32 = 0x00eb_5410;
/// `memset` (`00ec61c0`), the vector constructor iterator (`00401050`, `stdcall`: the array, the
/// element size, the count, the constructor) with the colours' constructor (`004a7800`), the read of
/// the texture sets past the first into a scratch block (`00a67aa0`, `cdecl`: the stream, the block,
/// the count) and the warning it is followed by (`00a29680`, `cdecl`, its format at `0109c040`).
const MEMSET: u32 = 0x00ec_61c0;
const VECTOR_CONSTRUCT: u32 = 0x0040_1050;
const COLOUR_CONSTRUCT: u32 = 0x004a_7800;
const READ_EXTRA_SETS: u32 = 0x00a6_7aa0;
const WARNING: u32 = 0x00a2_9680;
const EXTRA_SETS_FORMAT: u32 = 0x0109_c040;

/// The data's slot 0x80: its group (`GetGroup`).
const SLOT_GET_GROUP: u32 = 0x80;
/// The normal method bits of the data flags (binormals and tangents after the normals).
const METHOD_BITS: u32 = 0xf000;

/// One transfer through a binary stream's function word at `slot` (+8 read, +0xc write): `(binary,
/// buffer, bytes, &element size, 1)` with the element size in a stack word, and the stream's position
/// moved on by the answer. Answers the answer.
fn transfer(e: &mut Engine, binary: u32, slot: u32, buffer: u32, bytes: u32, size: u32) -> u32 {
    let function = e.mem.u32(binary.wrapping_add(slot));
    e.with_stack(4, |e, element| {
        e.mem.set_u32(element.addr(), size);
        let moved = e
            .call(
                function,
                &args![binary, buffer, bytes, element.addr(), 1u32],
            )
            .u32();
        let binary = Ptr::<BinaryStream>::new(binary);
        let position = e.get(binary, BinaryStream::m_uiPosition);
        e.set(
            binary,
            BinaryStream::m_uiPosition,
            position.wrapping_add(moved),
        );
        moved
    })
}

/// Reads `bytes` into `buffer` from the stream being loaded (`m_pkIstr`).
fn read(e: &mut Engine, stream: Ptr<Stream>, buffer: u32, bytes: u32, size: u32) {
    let binary = e.get(stream, Stream::m_pkIstr);
    transfer(e, binary, 8, buffer, bytes, size);
}

/// Writes `bytes` from `buffer` to the stream being saved (`m_pkOstr`).
fn write(e: &mut Engine, stream: Ptr<Stream>, buffer: u32, bytes: u32, size: u32) {
    let binary = e.get(stream, Stream::m_pkOstr);
    transfer(e, binary, 0xc, buffer, bytes, size);
}

/// Writes one byte held in a stack local (only its low byte set, as the exe stores it).
fn write_byte(e: &mut Engine, stream: Ptr<Stream>, value: u8) {
    e.with_stack(4, |e, local| {
        e.mem.set_u8(local.addr(), value);
        write(e, stream, local.addr(), 1, 1);
    });
}

/// Writes one word held in a stack local (`bytes` of it).
fn write_local(e: &mut Engine, stream: Ptr<Stream>, value: u32, bytes: u32) {
    e.with_stack(4, |e, local| {
        e.mem.set_u32(local.addr(), value);
        write(e, stream, local.addr(), bytes, bytes);
    });
}

/// The file version (`m_uiNifFileVersion`) and the header's version word (+4).
fn version(e: &Engine, stream: Ptr<Stream>) -> u32 {
    e.get(stream, Stream::m_uiNifFileVersion)
}
fn header_version(e: &Engine, stream: Ptr<Stream>) -> u32 {
    e.get(stream, Stream::m_uiHeaderVersion)
}

fn allocate(e: &mut Engine, size: u32) -> u32 {
    e.call(ALLOCATE, &args![MEMORY_MANAGER, size]).u32()
}

fn free(e: &mut Engine, block: u32) {
    e.call(FREE, &args![MEMORY_MANAGER, block]);
}

/// The block of one array of `bytes` bytes: from the data's group (slot 0x80, asked twice as the exe
/// does), whose free word (+8) moves on by `bytes` and whose count (+0xc) goes up, or else from the
/// memory manager (`size`, the count times the element size, all ones on overflow).
fn group_or_allocate(e: &mut Engine, data: u32, bytes: u32, size: u32) -> u32 {
    if e.vcall(data, SLOT_GET_GROUP, &args![]).u32() == 0 {
        return allocate(e, size);
    }
    let group = e.vcall(data, SLOT_GET_GROUP, &args![]).u32();
    let block = e.mem.u32(group.wrapping_add(8));
    let count = e.mem.u32(group.wrapping_add(0xc)).wrapping_add(1);
    e.mem.set_u32(group.wrapping_add(0xc), count);
    e.mem
        .set_u32(group.wrapping_add(8), block.wrapping_add(bytes));
    block
}

/// An array's presence as `LoadBinary` reads it: before 4.1.0.0 the array word itself (4 bytes into
/// `field`) and its being set; from 4.1.0.0 one byte into a stack local whose low byte starts as
/// `initial` (the exe reuses an argument's slot, so a read that writes nothing leaves that byte).
fn presence(e: &mut Engine, stream: Ptr<Stream>, field: u32, initial: u8) -> bool {
    if version(e, stream) < 0x0401_0000 {
        read(e, stream, field, 4, 4);
        e.mem.u32(field) != 0
    } else {
        e.with_stack(4, |e, local| {
            e.mem.set_u8(local.addr(), initial);
            read(e, stream, local.addr(), 1, 1);
            e.mem.u8(local.addr()) != 0
        })
    }
}

/// Whether `LoadBinary` reads an array's bytes: the header's version below 0x13, or
/// `m_bSaveVertexData` set.
fn reads_bytes(e: &Engine, data: Ptr<GeometryData>, stream: Ptr<Stream>) -> bool {
    header_version(e, stream) < 0x13 || e.get(data, GeometryData::m_bSaveVertexData) != 0
}

// Typed from 00a67ea0
/// `NiGeometryData::LoadBinary` (Xbox PDB; `thiscall`, the stream). After `NiObject::LoadBinary`:
/// from 10.1.0.114 the group's id (`GetGroupFromID`, into the static group), the vertex count, from
/// 10.0.1.16 the keep and compress flags; then the positions, the data flags (from 10.0.0.2; three
/// times the normals when a method bit is set), the normals, the bound (`NiSphere::LoadBinary`), the
/// colours (made with their constructor `004a7800` when the memory manager allocates them), the data
/// flags before 5.0.0.10 (only bit 0 kept, or read whole up to 10.0.0.1), the texture word before
/// 4.1.0.0, then for header versions below 0x1b the texture set count from the data flags, the texture
/// coordinates (the sets past the first read into a scratch block by `00a67aa0`, freed, and a warning
/// printed), from 5.0.0.10 the dirty flags and from 10.3.0.7 the link step `00a645b0`. Each array
/// comes from the group or the memory manager; its bytes are read when the header's version is below
/// 0x13 or `m_bSaveVertexData` is set (the positions are zeroed otherwise).
pub fn load_binary(e: &mut Engine, this: Ptr, stream: Ptr) {
    let data = this.cast::<GeometryData>();
    let base = this.addr();
    let s = stream.cast::<Stream>();
    e.call(OBJECT_LOAD_BINARY, &args![this, stream]);
    if version(e, s) >= 0x0a01_0072 {
        let id = e.with_stack(4, |e, local| {
            read(e, s, local.addr(), 4, 4);
            e.mem.u32(local.addr())
        });
        let group = e.call(GET_GROUP_FROM_ID, &args![stream, id]).u32();
        e.mem.set_u32(GEOMETRY_GROUP, group);
    }
    read(e, s, base + 8, 2, 2);
    if version(e, s) >= 0x0a00_0110 {
        read(e, s, base + 0x38, 1, 1);
        read(e, s, base + 0x39, 1, 1);
    }

    // The positions (the presence byte's local is the stream argument's own slot).
    if presence(e, s, base + 0x20, low_byte(stream.addr())) {
        let count = u32::from(e.get(data, GeometryData::m_usVertices));
        let block = group_or_allocate(e, base, count * 0xc, count.saturating_mul(0xc));
        e.set(data, GeometryData::m_pkVertex, block);
        let bytes = u32::from(e.get(data, GeometryData::m_usVertices)) * 0xc;
        let positions = e.get(data, GeometryData::m_pkVertex);
        if reads_bytes(e, data, s) {
            read(e, s, positions, bytes, 4);
        } else {
            e.call(MEMSET, &args![positions, 0u32, bytes]);
        }
    }

    let mut normal_count = u32::from(e.get(data, GeometryData::m_usVertices));
    if version(e, s) >= 0x0a00_0002 {
        read(e, s, base + 0xc, 2, 2);
        if u32::from(e.get(data, GeometryData::m_usDataFlags)) & METHOD_BITS != 0 {
            normal_count *= 3;
        }
    }
    if presence(e, s, base + 0x24, 0) {
        let block = group_or_allocate(
            e,
            base,
            normal_count.wrapping_mul(0xc),
            normal_count.saturating_mul(0xc),
        );
        e.set(data, GeometryData::m_pkNormal, block);
        if reads_bytes(e, data, s) {
            let normals = e.get(data, GeometryData::m_pkNormal);
            read(e, s, normals, normal_count.wrapping_mul(0xc), 4);
        }
    }

    e.call(SPHERE_LOAD_BINARY, &args![base + 0x10, stream]);

    if presence(e, s, base + 0x28, 0) {
        let block = if e.vcall(base, SLOT_GET_GROUP, &args![]).u32() != 0 {
            let bytes = u32::from(e.get(data, GeometryData::m_usVertices)) << 4;
            let group = e.vcall(base, SLOT_GET_GROUP, &args![]).u32();
            let block = e.mem.u32(group + 8);
            let uses = e.mem.u32(group + 0xc).wrapping_add(1);
            e.mem.set_u32(group + 0xc, uses);
            e.mem.set_u32(group + 8, bytes.wrapping_add(block));
            block
        } else {
            let count = u32::from(e.get(data, GeometryData::m_usVertices));
            let block = allocate(e, count.saturating_mul(0x10));
            if block != 0 {
                e.call(
                    VECTOR_CONSTRUCT,
                    &args![block, 0x10u32, count, COLOUR_CONSTRUCT],
                );
            }
            block
        };
        e.set(data, GeometryData::m_pkColor, block);
        if reads_bytes(e, data, s) {
            let bytes = u32::from(e.get(data, GeometryData::m_usVertices)) << 4;
            let colours = e.get(data, GeometryData::m_pkColor);
            read(e, s, colours, bytes, 4);
        }
    }

    // Before 5.0.0.10 the flags are read into the local that held the normal count (and later the
    // texture set count).
    let file_version = version(e, s);
    if file_version < 0x0500_000a {
        let word = e.with_stack(4, |e, local| {
            e.mem.set_u32(local.addr(), normal_count);
            read(e, s, local.addr(), 2, 2);
            e.mem.u32(local.addr())
        });
        let flags = e.get(data, GeometryData::m_usDataFlags);
        e.set(
            data,
            GeometryData::m_usDataFlags,
            flags ^ ((flags ^ low_half(word)) & 1),
        );
    } else if file_version < 0x0a00_0002 {
        read(e, s, base + 0xc, 2, 2);
    }
    if version(e, s) < 0x0401_0000 {
        read(e, s, base + 0x2c, 4, 4);
    }
    let sets = if header_version(e, s) < 0x1b {
        let flags = u32::from(e.get(data, GeometryData::m_usDataFlags));
        let sets = flags & 0x1f;
        if low_half(sets) > 1 {
            e.set(data, GeometryData::m_usDataFlags, low_half(flags | 1));
        }
        sets
    } else {
        0
    };

    let flags = u32::from(e.get(data, GeometryData::m_usDataFlags));
    if flags & 1 != 0 {
        let count = (flags & 1).wrapping_mul(u32::from(e.get(data, GeometryData::m_usVertices)));
        let block = group_or_allocate(e, base, count.wrapping_mul(8), count.saturating_mul(8));
        e.set(data, GeometryData::m_pkTexture, block);
        if reads_bytes(e, data, s) {
            let textures = e.get(data, GeometryData::m_pkTexture);
            read(e, s, textures, count.wrapping_mul(8), 4);
            if low_half(sets) > 1 {
                let vertices = u32::from(e.get(data, GeometryData::m_usVertices));
                let extra = u32::from(low_half(sets) - 1).wrapping_mul(vertices);
                let scratch = allocate(e, extra.saturating_mul(8));
                e.call(READ_EXTRA_SETS, &args![stream, scratch, extra]);
                free(e, scratch);
                e.call(
                    WARNING,
                    &args![
                        EXTRA_SETS_FORMAT,
                        stream.addr() + 0xe0,
                        extra.wrapping_mul(8) >> 10
                    ],
                );
            }
        }
    }

    if version(e, s) >= 0x0500_000a {
        read(e, s, base + 0xe, 2, 2);
    }
    if version(e, s) >= 0x0a03_0007 {
        e.call(LINK_STEP, &args![stream]);
    }
}

// Typed from 00a67ae0
/// The texture coordinates' write (unnamed in the engine map; `cdecl`: the stream, the array, the
/// count): `count * 8` bytes of the array to the stream being saved, in 4-byte elements (the element
/// size in a stack word: the exe keeps it in the stream argument's slot). Answers what the write
/// function answered.
pub fn write_texture_coordinates(e: &mut Engine, stream: Ptr, textures: u32, count: u32) -> u32 {
    let binary = e.get(stream.cast::<Stream>(), Stream::m_pkOstr);
    transfer(e, binary, 0xc, textures, count.wrapping_mul(8), 4)
}

// Typed from 00a68520
/// `NiGeometryData::SaveBinary` (Xbox PDB; `thiscall`, the stream): the base step `00a89af0`, the
/// group's id (`00a634f0`), the vertex count, the keep and compress flags, a presence byte and (when
/// present and `m_bSaveVertexData` is set) the positions; bit 0 of the data flags cleared when there
/// are no texture coordinates, and the flags written; the normals' presence and bytes (three times as
/// many with a method bit), the bound (`NiSphere::SaveBinary`), the colours' presence and bytes, the
/// texture coordinates ([`write_texture_coordinates`], when the flags' bit 0, the vertex count, the
/// array and `m_bSaveVertexData` all allow), the dirty flags' bits 0x7000 and the additional data's
/// link (the stream's slot 0x30).
pub fn save_binary(e: &mut Engine, this: Ptr, stream: Ptr) {
    let data = this.cast::<GeometryData>();
    let base = this.addr();
    let s = stream.cast::<Stream>();
    e.call(OBJECT_SAVE_STEP, &args![this, stream]);
    let group = e.mem.u32(GEOMETRY_GROUP);
    let id = e.call(GROUP_ID, &args![stream, group]).u32();
    write_local(e, s, id, 4);
    write(e, s, base + 8, 2, 2);
    write(e, s, base + 0x38, 1, 1);
    write(e, s, base + 0x39, 1, 1);

    let saves = |e: &Engine| e.get(data, GeometryData::m_bSaveVertexData) != 0;
    let has_positions = e.get(data, GeometryData::m_pkVertex) != 0;
    write_byte(e, s, u8::from(has_positions));
    if has_positions && saves(e) {
        let bytes = u32::from(e.get(data, GeometryData::m_usVertices)) * 0xc;
        let positions = e.get(data, GeometryData::m_pkVertex);
        write(e, s, positions, bytes, 4);
    }

    if e.get(data, GeometryData::m_pkTexture) == 0 {
        let flags = e.get(data, GeometryData::m_usDataFlags);
        e.set(data, GeometryData::m_usDataFlags, flags & 0xfffe);
    }
    write(e, s, base + 0xc, 2, 2);
    let mut normal_count = u32::from(e.get(data, GeometryData::m_usVertices));
    if u32::from(e.get(data, GeometryData::m_usDataFlags)) & METHOD_BITS != 0 {
        normal_count *= 3;
    }
    let has_normals = e.get(data, GeometryData::m_pkNormal) != 0;
    write_byte(e, s, u8::from(has_normals));
    if has_normals && saves(e) {
        let normals = e.get(data, GeometryData::m_pkNormal);
        write(e, s, normals, normal_count * 0xc, 4);
    }

    e.call(SPHERE_SAVE_BINARY, &args![base + 0x10, stream]);

    let has_colours = e.get(data, GeometryData::m_pkColor) != 0;
    write_byte(e, s, u8::from(has_colours));
    if has_colours && saves(e) {
        let bytes = u32::from(e.get(data, GeometryData::m_usVertices)) << 4;
        let colours = e.get(data, GeometryData::m_pkColor);
        write(e, s, colours, bytes, 4);
    }

    let texture_count = u32::from(e.get(data, GeometryData::m_usDataFlags) & 1)
        * u32::from(e.get(data, GeometryData::m_usVertices));
    if texture_count != 0 {
        let textures = e.get(data, GeometryData::m_pkTexture);
        if textures != 0 && saves(e) {
            let _: u32 = engine_core::typed_call!(
                super::FAMILY,
                0x00a6_7ae0,
                write_texture_coordinates(e, stream, textures, texture_count)
            );
        }
    }

    let dirty = u32::from(e.get(data, GeometryData::m_usDirtyFlags) & 0x7000);
    write_local(e, s, dirty, 2);
    let additional = e.get(data, GeometryData::m_spAdditionalGeomData);
    e.vcall(stream.addr(), 0x30, &args![additional]);
}

// Typed from 00a67c70
/// The `NiGeometryData` destructor body (unnamed in the engine map; `thiscall`; the deleting
/// destructor `00a68800` runs it). Sets the class's vtable, hands `this` to the slot 0xf0 of the
/// object at `011f4748` when there is one, releases the additional data (`InterlockedDecrement` on its
/// count, its slot 4 at zero) and clears it, then gives up the four arrays: with the static group
/// set, one group use for each array that is set; without one each array is freed (`00aa4060`, a null
/// one too). Then the additional data is released again when set, and the `NiObject` destructor body
/// `00a5d3d0` ends it.
pub fn destruct(e: &mut Engine, this: Ptr) {
    let data = this.cast::<GeometryData>();
    let base = this.addr();
    e.set(data, GeometryData::vtable, VTABLE);
    let renderer = e.mem.u32(RENDERER);
    if renderer != 0 {
        e.vcall(renderer, 0xf0, &args![base]);
    }
    let release = |e: &mut Engine, object: u32| {
        let count = e
            .call(INTERLOCKED_DECREMENT, &args![object.wrapping_add(4)])
            .u32();
        if count == 0 {
            e.vcall(object, 4, &args![]);
        }
    };
    let additional = e.get(data, GeometryData::m_spAdditionalGeomData);
    if additional != 0 {
        release(e, additional);
        e.set(data, GeometryData::m_spAdditionalGeomData, 0);
    }
    let arrays = [
        GeometryData::m_pkVertex,
        GeometryData::m_pkNormal,
        GeometryData::m_pkColor,
        GeometryData::m_pkTexture,
    ];
    if e.mem.u32(GEOMETRY_GROUP) == 0 {
        for field in arrays {
            let array = e.get(data, field);
            free(e, array);
        }
    } else {
        for field in arrays {
            if e.get(data, field) != 0 {
                let group = e.mem.u32(GEOMETRY_GROUP);
                release_group(e, group);
            }
        }
    }
    let additional = e.get(data, GeometryData::m_spAdditionalGeomData);
    if additional != 0 {
        release(e, additional);
    }
    e.call(OBJECT_DESTRUCT, &args![this]);
}

/// The normals a vertex has: 3 with a method bit set (binormals and tangents after them), else 1.
fn normals_per_vertex(e: &Engine, data: Ptr<GeometryData>) -> u32 {
    if u32::from(e.get(data, GeometryData::m_usDataFlags)) & METHOD_BITS != 0 {
        3
    } else {
        1
    }
}

// Typed from 00a669f0
/// `NiGeometryData::CreateNormals` (Xbox PDB; `thiscall`, a flag word): without normals, a new array
/// of the vertex count's normals (three a vertex with a method bit), 12 bytes each (all ones on
/// overflow); when the flag's low byte is set the normals are cleared (`memset`).
pub fn create_normals(e: &mut Engine, this: Ptr, initialize: u32) {
    let data = this.cast::<GeometryData>();
    let per_vertex = normals_per_vertex(e, data);
    if e.get(data, GeometryData::m_pkNormal) == 0 {
        let count = u32::from(e.get(data, GeometryData::m_usVertices)).wrapping_mul(per_vertex);
        let normals = allocate(e, count.saturating_mul(12));
        e.set(data, GeometryData::m_pkNormal, normals);
    }
    if low_byte(initialize) != 0 {
        let count = u32::from(e.get(data, GeometryData::m_usVertices)).wrapping_mul(per_vertex);
        let normals = e.get(data, GeometryData::m_pkNormal);
        e.call(MEMSET, &args![normals, 0u32, count.wrapping_mul(12)]);
    }
}

// Typed from 00a66a60
/// The normal method's setter (unnamed in the engine map; `thiscall`, the method word): nothing when
/// the method bits already equal it; otherwise they take it (the flags' low 12 bits kept) and, when
/// there are normals, a new array (one normal a vertex, or three with a method) gets the old normals
/// word by word, the first binormal cleared when there is a method; the old array is freed and the new
/// one kept.
pub fn set_normal_method(e: &mut Engine, this: Ptr, method: u32) {
    let data = this.cast::<GeometryData>();
    let flags = u32::from(e.get(data, GeometryData::m_usDataFlags));
    if flags & METHOD_BITS == method {
        return;
    }
    e.set(
        data,
        GeometryData::m_usDataFlags,
        low_half((flags & 0xfff) | method),
    );
    if e.get(data, GeometryData::m_pkNormal) == 0 {
        return;
    }
    let per_vertex = if method != 0 { 3 } else { 1 };
    let count = u32::from(e.get(data, GeometryData::m_usVertices)).wrapping_mul(per_vertex);
    let normals = allocate(e, count.saturating_mul(12));
    let mut index = 0u32;
    while index < u32::from(e.get(data, GeometryData::m_usVertices)) {
        let old = e
            .get(data, GeometryData::m_pkNormal)
            .wrapping_add(12 * index);
        for offset in [0u32, 4, 8] {
            let word = e.mem.u32(old + offset);
            e.mem
                .set_u32(normals.wrapping_add(12 * index + offset), word);
        }
        index += 1;
    }
    if method != 0 {
        let vertices = u32::from(e.get(data, GeometryData::m_usVertices));
        let binormal = normals.wrapping_add(12 * vertices);
        for offset in [0u32, 4, 8] {
            e.mem.set_u32(binormal + offset, 0);
        }
    }
    let old = e.get(data, GeometryData::m_pkNormal);
    free(e, old);
    e.set(data, GeometryData::m_pkNormal, normals);
}

// Typed from 00a67000
/// The normals' release (unnamed in the engine map; `thiscall`): when there are normals they are
/// freed through the memory manager and the word cleared.
pub fn release_normals(e: &mut Engine, this: Ptr) {
    let data = this.cast::<GeometryData>();
    let normals = e.get(data, GeometryData::m_pkNormal);
    if normals != 0 {
        free(e, normals);
        e.set(data, GeometryData::m_pkNormal, 0);
    }
}
