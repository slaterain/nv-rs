//! The renderer's packing of a geometry's arrays into its vertex buffer: the shader declaration's
//! per-stream pack (its slot 0xac, which `PackGeometryBuffer` runs for each stream), the element pack
//! that picks an array for a declaration entry and hands it to a converter through a packer entry on
//! its stack, the converters that can receive one of the arrays (float sources: the positions, the
//! normals with the binormals and tangents after them, the colours and the texture coordinates), the
//! texture set getter, and the fixed-function packer (`PackUntransformedVB`) with its strided copy.
//!
//! The converters `00e78560` stores for the other source formats read a skin partition's weights and
//! bone indices or the additional data's streams, never these arrays: they stay calls by address.
//!
//! What stays as the exe has it: every call by address (the declaration's slots, the buffer data's
//! update, the manager's new buffer, lock and unlock, `memcpy`, `memset`, `_ftol2`, the additional
//! data's lookup, the derived vectors' pack `00a855f0`, the skinned positions' pack `00687d40`), and the
//! reads of a packer entry's fields where the exe reads them (again after each vertex where it does).

use super::layout::{
    DeclarationEntry, GeometryBufferData, GeometryData, PackerEntry, ShaderDeclaration,
    SkinPartition, StreamEntry, VBChip, VertexBufferManager,
};
use super::{low_byte, low_half, word_of, FAMILY};
use engine_core::ptr::Ptr;
use engine_core::x87::{fistp, fistp_chop, x87_load32, x87_store32};
use engine_core::{args, typed_call, Engine};

/// The C runtime's `memset` (`00ec61c0`), `memcpy` (`00ec44d0`) and `_ftol2` (`00ec62c0`, the value
/// in ST0), all `cdecl`.
const MEMSET: u32 = 0x00ec_61c0;
const MEMCPY: u32 = 0x00ec_44d0;
const FTOL2: u32 = 0x00ec_62c0;

type Entry = Ptr<PackerEntry>;

/// The packer entry's vertex count, as a converter reads it before each vertex.
fn vertex_count(e: &Engine, entry: Entry) -> u16 {
    e.get(entry, PackerEntry::usCount)
}

/// The source element of vertex `vertex` through the 16-bit index array: the index times the source
/// stride (read here) on from the source.
fn indexed(e: &Engine, entry: Entry, index: u32, source: u32, vertex: u16) -> u32 {
    let at = u32::from(e.mem.u16(index.wrapping_add(u32::from(vertex) * 2)));
    at.wrapping_mul(e.get(entry, PackerEntry::uiSourceStride))
        .wrapping_add(source)
}

/// A converter without a source: each vertex's slot filled with `byte` (`memset` of the size a vertex,
/// read before the first and again after each), the destination stepping by its stride. Answers the
/// bytes written.
fn fill_vertices(e: &mut Engine, entry: Entry, byte: u32) -> u32 {
    let mut destination = e.get(entry, PackerEntry::pDestination);
    if vertex_count(e, entry) == 0 {
        return 0;
    }
    let mut total = 0u32;
    let mut size = e.get(entry, PackerEntry::uiWriteSize);
    let mut vertex = 0u16;
    loop {
        e.call(MEMSET, &args![destination, byte, size]);
        size = e.get(entry, PackerEntry::uiWriteSize);
        let step = e.get(entry, PackerEntry::uiDestinationStride);
        destination = destination.wrapping_add(step);
        total = total.wrapping_add(size);
        vertex = vertex.wrapping_add(1);
        if vertex >= vertex_count(e, entry) {
            return total;
        }
    }
}

// Typed from 00e78f80
/// The converter to as many floats as the source has (`00e78560` stores it for each float type with
/// the format one above it), `cdecl` with the packer entry. Without a source each vertex's slot is
/// zeroed ([`fill_vertices`]). With an index array each vertex copies its indexed element (`memcpy` of
/// the element size, the sizes and strides read for each vertex). Without one, elements of 8, 12 and 16
/// bytes are copied a word at a time (sizes and strides read once; the answer is then the count times
/// the size a vertex), any other size through `memcpy`. Answers the bytes written: the size a vertex
/// for each vertex.
pub fn copy_same(e: &mut Engine, entry: u32) -> u32 {
    let entry = Entry::new(entry);
    let source = e.get(entry, PackerEntry::pSource);
    let index = e.get(entry, PackerEntry::pusIndex);
    let mut destination = e.get(entry, PackerEntry::pDestination);
    let mut total = 0u32;
    if source == 0 {
        return fill_vertices(e, entry, 0);
    }
    if index != 0 {
        let mut vertex = 0u16;
        while vertex < vertex_count(e, entry) {
            let size = e.get(entry, PackerEntry::uiElementSize);
            let element = indexed(e, entry, index, source, vertex);
            vertex = vertex.wrapping_add(1);
            e.call(MEMCPY, &args![destination, element, size]);
            let step = e.get(entry, PackerEntry::uiDestinationStride);
            destination = destination.wrapping_add(step);
            total = total.wrapping_add(e.get(entry, PackerEntry::uiWriteSize));
        }
        return total;
    }
    let write_size = e.get(entry, PackerEntry::uiWriteSize);
    let size = e.get(entry, PackerEntry::uiElementSize);
    let step = e.get(entry, PackerEntry::uiDestinationStride);
    let stride = e.get(entry, PackerEntry::uiSourceStride);
    let count = u32::from(vertex_count(e, entry));
    let words = match size {
        8 => 2u32,
        0xc => 3,
        0x10 => 4,
        _ => {
            let mut from = source;
            let mut vertex = 0u32;
            while vertex < u32::from(vertex_count(e, entry)) {
                let size = e.get(entry, PackerEntry::uiElementSize);
                e.call(MEMCPY, &args![destination, from, size]);
                destination = destination.wrapping_add(step);
                total = total.wrapping_add(write_size);
                vertex += 1;
                from = from.wrapping_add(stride);
            }
            return total;
        }
    };
    let mut from = source;
    for _ in 0..count {
        for word in 0..words {
            let value = e.mem.u32(from.wrapping_add(4 * word));
            e.mem.set_u32(destination.wrapping_add(4 * word), value);
        }
        from = from.wrapping_add(stride);
        destination = destination.wrapping_add(step);
    }
    count.wrapping_mul(write_size)
}

// Typed from 00e78d50
/// The converter to fewer floats than the source has (`00e78560` stores it for each float type with a
/// format more than one above it), `cdecl` with the packer entry: each vertex copies the size a vertex
/// (read before the first copy and again after each) from its source element (`memcpy`; indexed
/// through the index array when there is one, else the source stepping by its stride). Without a
/// source each slot is zeroed ([`fill_vertices`]). Answers the bytes written.
pub fn copy_narrower(e: &mut Engine, entry: u32) -> u32 {
    let entry = Entry::new(entry);
    let source = e.get(entry, PackerEntry::pSource);
    let index = e.get(entry, PackerEntry::pusIndex);
    if source == 0 {
        return fill_vertices(e, entry, 0);
    }
    let mut destination = e.get(entry, PackerEntry::pDestination);
    if vertex_count(e, entry) == 0 {
        return 0;
    }
    let mut total = 0u32;
    let mut current = source;
    let mut size = e.get(entry, PackerEntry::uiWriteSize);
    let mut vertex = 0u16;
    loop {
        let element = if index != 0 {
            indexed(e, entry, index, source, vertex)
        } else {
            current
        };
        e.call(MEMCPY, &args![destination, element, size]);
        size = e.get(entry, PackerEntry::uiWriteSize);
        if index == 0 {
            current = current.wrapping_add(e.get(entry, PackerEntry::uiSourceStride));
        }
        let step = e.get(entry, PackerEntry::uiDestinationStride);
        destination = destination.wrapping_add(step);
        total = total.wrapping_add(size);
        vertex = vertex.wrapping_add(1);
        if vertex >= vertex_count(e, entry) {
            return total;
        }
    }
}

/// Clears `count` words from `at` on (`REP STOSD`).
fn zero_words(e: &mut Engine, at: u32, count: u16) {
    for word in 0..u32::from(count) {
        e.mem.set_u32(at.wrapping_add(4 * word), 0);
    }
}

// Typed from 00e78e30
/// The converter to more floats than the source has (`00e78560` stores it for each float type with a
/// lower format, and for the formats 0x1d to 0x20), `cdecl` with the packer entry. Each vertex copies
/// its source element (`memcpy` of the element size; indexed through the index array when there is
/// one, else the source stepping by its stride) and clears the words after it: the type (the 16 bits
/// at +0) less the format (the 16 bits at +4) plus one, as 16 bits. Without a source each slot is
/// zeroed ([`fill_vertices`]). Answers the bytes written.
pub fn copy_wider(e: &mut Engine, entry: u32) -> u32 {
    let entry = Entry::new(entry);
    let pad = low_half(e.get(entry, PackerEntry::uiType))
        .wrapping_sub(low_half(e.get(entry, PackerEntry::uiFormat)))
        .wrapping_add(1);
    let source = e.get(entry, PackerEntry::pSource);
    let index = e.get(entry, PackerEntry::pusIndex);
    if source == 0 {
        return fill_vertices(e, entry, 0);
    }
    let mut destination = e.get(entry, PackerEntry::pDestination);
    let mut total = 0u32;
    let mut current = source;
    let mut vertex = 0u16;
    while vertex < vertex_count(e, entry) {
        let element = if index != 0 {
            indexed(e, entry, index, source, vertex)
        } else {
            current
        };
        let size = e.get(entry, PackerEntry::uiElementSize);
        e.call(MEMCPY, &args![destination, element, size]);
        let end = e
            .get(entry, PackerEntry::uiElementSize)
            .wrapping_add(destination);
        zero_words(e, end, pad);
        total = total.wrapping_add(e.get(entry, PackerEntry::uiWriteSize));
        if index == 0 {
            current = current.wrapping_add(e.get(entry, PackerEntry::uiSourceStride));
        }
        let step = e.get(entry, PackerEntry::uiDestinationStride);
        destination = destination.wrapping_add(step);
        vertex = vertex.wrapping_add(1);
    }
    total
}

/// The colour converter's constants: 255.0 as a float (`01023cd8`, every channel when there is no
/// source) and as a double (`0101e568`, the scale of each channel).
const WHITE_CHANNEL: u32 = 0x0102_3cd8;
const CHANNEL_SCALE: u32 = 0x0101_e568;

/// One colour channel: `FLD dword` of the float, `FMUL qword` by the scale, `FSTP dword` and
/// `FLD dword`, then `FISTP` (to nearest). The exe's scale is 255.0, eight significant bits: the
/// product of a float and it is exact in an `f64`, so rounding that once to a float gives what `FMUL`
/// and `FSTP` give under either control word. Only below the float's normal range does 24-bit
/// precision round twice, and `FISTP` makes 0 of any value there.
fn channel(bits: u32, scale: f64) -> u32 {
    let stored = x87_store32(x87_load32(bits) * scale);
    word_of(fistp(x87_load32(stored)))
}

/// Four channels packed as the exe packs them (`SHL 8` and `OR` of whole words, so a negative
/// channel's high bits reach the channels above it).
fn pack_channels(alpha: u32, red: u32, green: u32, blue: u32) -> u32 {
    (((((alpha << 8) | red) << 8) | green) << 8) | blue
}

// Typed from 00e79d00
/// The colour converter (`00e78560` stores it for the colour type with the formats 3, 4 and 0x1d to
/// 0x20), `cdecl` with the packer entry: each vertex's colour as one word, alpha, red, green and blue
/// from the high byte down, from the four floats of its source element ([`channel`]; with the 16 bits
/// at +4 equal to 3 the alpha is 1.0), indexed through the index array when there is one. Without a
/// source every vertex gets the four channels of the float at `01023cd8`. Answers the bytes written.
pub fn colour(e: &mut Engine, entry: u32) -> u32 {
    let entry = Entry::new(entry);
    let mut source = e.get(entry, PackerEntry::pSource);
    let index = e.get(entry, PackerEntry::pusIndex);
    let mut destination = e.get(entry, PackerEntry::pDestination);
    let mut total = 0u32;
    if source == 0 {
        if vertex_count(e, entry) == 0 {
            return 0;
        }
        // `FLD dword` of the white channel and four `FST`/`FSTP` into the channel locals, each vertex
        // storing their four `FISTP`s: the same word for every vertex.
        let stored = x87_store32(x87_load32(e.global::<u32>(WHITE_CHANNEL)));
        let white = word_of(fistp(x87_load32(stored)));
        let packed = pack_channels(white, white, white, white);
        let mut vertex = 0u16;
        loop {
            vertex = vertex.wrapping_add(1);
            e.mem.set_u32(destination, packed);
            let step = e.get(entry, PackerEntry::uiDestinationStride);
            destination = destination.wrapping_add(step);
            total = total.wrapping_add(e.get(entry, PackerEntry::uiWriteSize));
            if vertex >= vertex_count(e, entry) {
                return total;
            }
        }
    }
    let opaque = low_half(e.get(entry, PackerEntry::uiFormat)) == 3;
    let scale = f64::from_bits(e.mem.u64(CHANNEL_SCALE));
    let mut vertex = 0u16;
    while vertex < vertex_count(e, entry) {
        let element = if index != 0 {
            indexed(e, entry, index, source, vertex)
        } else {
            source
        };
        // `FLD1` or `FLD dword [+0xc]`, then `FSTP dword` to a local: the alpha's bits.
        let alpha_bits = if opaque {
            1.0f32.to_bits()
        } else {
            x87_store32(x87_load32(e.mem.u32(element.wrapping_add(0xc))))
        };
        let alpha = channel(alpha_bits, scale);
        let red = channel(e.mem.u32(element), scale);
        let green = channel(e.mem.u32(element.wrapping_add(4)), scale);
        let blue = channel(e.mem.u32(element.wrapping_add(8)), scale);
        e.mem
            .set_u32(destination, pack_channels(alpha, red, green, blue));
        if index == 0 {
            source = source.wrapping_add(e.get(entry, PackerEntry::uiSourceStride));
        }
        let step = e.get(entry, PackerEntry::uiDestinationStride);
        destination = destination.wrapping_add(step);
        total = total.wrapping_add(e.get(entry, PackerEntry::uiWriteSize));
        vertex = vertex.wrapping_add(1);
    }
    total
}

/// The loop the integer converters share (the exe repeats it in each): without a source each vertex's
/// slot is zeroed ([`fill_vertices`]); with one, `element` packs each vertex from its source element
/// (indexed through the index array when there is one, else the source stepping by its stride) into
/// its destination. The size a vertex, the strides and the count are read again for each vertex.
/// Answers the bytes written.
fn pack_elements(
    e: &mut Engine,
    entry: Entry,
    mut element: impl FnMut(&mut Engine, u32, u32),
) -> u32 {
    let source = e.get(entry, PackerEntry::pSource);
    let index = e.get(entry, PackerEntry::pusIndex);
    if source == 0 {
        return fill_vertices(e, entry, 0);
    }
    let mut destination = e.get(entry, PackerEntry::pDestination);
    let mut total = 0u32;
    let mut from = source;
    let mut vertex = 0u16;
    while vertex < vertex_count(e, entry) {
        let at = if index != 0 {
            indexed(e, entry, index, source, vertex)
        } else {
            from
        };
        element(e, at, destination);
        total = total.wrapping_add(e.get(entry, PackerEntry::uiWriteSize));
        if index == 0 {
            from = from.wrapping_add(e.get(entry, PackerEntry::uiSourceStride));
        }
        let step = e.get(entry, PackerEntry::uiDestinationStride);
        destination = destination.wrapping_add(step);
        vertex = vertex.wrapping_add(1);
    }
    total
}

// Typed from 00e77950
/// The converter of floats to bytes (`00e78560` stores it for type 5 with the float formats), `cdecl`
/// with the packer entry: each vertex takes `n` components (the 16 bits at +4, at most 4), each a float
/// stored as an integer rounded toward zero (`FISTP` under the control word's chop bits) of which the
/// low byte is kept, and when fewer than four bytes were written the rest of the four is cleared with
/// `memset` ([`pack_elements`]). Answers the bytes written.
pub fn floats_to_bytes(e: &mut Engine, entry: Ptr) -> u32 {
    let entry = entry.cast::<PackerEntry>();
    let n = low_half(e.get(entry, PackerEntry::uiFormat)).min(4);
    pack_elements(e, entry, |e, from, to| {
        for k in 0..u32::from(n) {
            let value = fistp_chop(x87_load32(e.mem.u32(from.wrapping_add(4 * k))));
            e.mem.set_u8(to.wrapping_add(k), low_byte(word_of(value)));
        }
        if n < 4 {
            let rest = 4 - u32::from(n);
            e.call(MEMSET, &args![to.wrapping_add(u32::from(n)), 0u32, rest]);
        }
    })
}

// Typed from 00e76c30
/// The converter of floats to 16-bit integers (`00e78560` stores it for the types 6 and 7 with the
/// float formats), `cdecl` with the packer entry: each vertex takes `n` components (the 16 bits at +4,
/// at most the width: 2 for type 6, else 4), each a float converted by `_ftol2` (its low 16 bits), and
/// the rest of the width is zeroed in place ([`pack_elements`]). Answers the bytes written.
pub fn floats_to_shorts(e: &mut Engine, entry: Ptr) -> u32 {
    let entry = entry.cast::<PackerEntry>();
    let components = low_half(e.get(entry, PackerEntry::uiFormat));
    let width: u16 = if e.get(entry, PackerEntry::uiType) != 6 {
        4
    } else {
        2
    };
    let n = components.min(width);
    pack_elements(e, entry, |e, from, to| {
        for k in 0..u32::from(n) {
            let value = x87_load32(e.mem.u32(from.wrapping_add(4 * k)));
            let short = e.call(FTOL2, &args![value]).u16();
            e.mem.set_u16(to.wrapping_add(k * 2), short);
        }
        for k in u32::from(n)..u32::from(width) {
            e.mem.set_u16(to.wrapping_add(k * 2), 0);
        }
    })
}

/// Runs `converter` on the packer entry `entry`: a `typed_call!` when it is one of this family's
/// converters (so a Rust call while the family is on), else the call by address the exe makes.
fn convert(e: &mut Engine, converter: u32, entry: u32) -> u32 {
    match converter {
        0x00e7_8f80 => typed_call!(FAMILY, 0x00e7_8f80, copy_same(e, entry)),
        0x00e7_8d50 => typed_call!(FAMILY, 0x00e7_8d50, copy_narrower(e, entry)),
        0x00e7_8e30 => typed_call!(FAMILY, 0x00e7_8e30, copy_wider(e, entry)),
        0x00e7_9d00 => typed_call!(FAMILY, 0x00e7_9d00, colour(e, entry)),
        0x00e7_7950 => typed_call!(FAMILY, 0x00e7_7950, floats_to_bytes(e, Ptr::new(entry))),
        0x00e7_6c30 => typed_call!(FAMILY, 0x00e7_6c30, floats_to_shorts(e, Ptr::new(entry))),
        _ => e.call(converter, &args![entry]).u32(),
    }
}

/// What the element pack reads: the size each type writes a vertex (words at `0126f508`), the converter
/// tables (`0126f550`, its +4: a table a type, whose +8 lists the converters by format) and the
/// additional data's lookup `00a72720` (`thiscall` on the data's additional data, seven words: the
/// stream and the addresses of the source, format, byte, data, element size and stride; a byte).
const WRITE_SIZES: u32 = 0x0126_f508;
const CONVERTER_TABLES: u32 = 0x0126_f550;
const ADDITIONAL_DATA: u32 = 0x00a7_2720;

/// The source, element size, stride and format a declaration entry's usage picks.
struct Picked {
    source: u32,
    size: u32,
    stride: u32,
    format: u32,
}

impl Picked {
    /// A source of `size`-byte elements as many bytes apart.
    fn fixed(source: u32, size: u32, format: u32) -> Picked {
        Picked {
            source,
            size,
            stride: size,
            format,
        }
    }
}

// Typed from 00e7a200
/// The element pack (unnamed in the engine map; `thiscall` with nine words, `this` and the third word
/// not read): the vertex count (its low 16 bits), the geometry data, a word, the skin partition, the
/// declaration entry, the destination stride, the destination, a byte and a word for the converter.
/// An entry of type 0x11 (unused) answers 0. The type (4 with usage 2 counts as 5) picks the size a
/// vertex (`0126f508`) and the converter table (`0126f550` +4; none: 0). The usage picks the source:
/// -1 none (0); 0 the positions (12 bytes, format 3); 1 the partition's weights by its bones a vertex
/// (1 to 3 floats: formats 1 to 3; 4 floats as 12 bytes of a 16-byte stride, format 3; no partition:
/// 0); 2 its bone palette (no partition or palette: 0) by the same (bytes, formats 0x19 to 0x1c); 3
/// the normals (12 bytes, 3); 4 the colours (16 bytes, 4); 5 to 12 texture set `usage - 5` (`00a66960`,
/// 8 bytes, 2); 13 and 14 the binormals and tangents after the normals (the normals plus the vertex
/// count times 0x18 or 0xc, when there are normals and the data flags have any of the bits 0xf000;
/// else none), 12 bytes, 3; a negative usage asks the additional data (`00a72720`; failing: no source,
/// format 0). The converter for the format then runs ([`convert`]) on a packer entry built on the
/// stack, with the partition's vertex map as the index array unless the usage is 1 or 2 (or there is
/// no partition), and its answer is answered; 0 without a converter or a source.
///
/// Usages 15 and up, and partitions with other than 1 to 4 bones a vertex for usages 1 and 2, leave the
/// source, element size and stride words as the exe's stack held them; as the translation, they are 0
/// here and the format is the first word (the slot the exe reuses).
#[allow(clippy::too_many_arguments)]
pub fn pack_entry(
    e: &mut Engine,
    _this: Ptr,
    count: u32,
    data: u32,
    _unused: u32,
    partition: u32,
    declaration_entry: u32,
    destination_stride: u32,
    destination: u32,
    flag: u32,
    last: u32,
) -> u32 {
    let entry = Ptr::<DeclarationEntry>::new(declaration_entry);
    let entry_type = e.get(entry, DeclarationEntry::uiType);
    if entry_type == 0x11 {
        return 0;
    }
    let kind = if entry_type == 4 && e.get(entry, DeclarationEntry::uiUsage) == 2 {
        5
    } else {
        entry_type
    };
    let write_size = e.mem.u32(WRITE_SIZES.wrapping_add(kind.wrapping_mul(4)));
    let tables = e.global::<u32>(CONVERTER_TABLES);
    let tables = e.mem.u32(tables.wrapping_add(4));
    let converters = e.mem.u32(tables.wrapping_add(kind.wrapping_mul(4)));
    if converters == 0 {
        return 0;
    }
    let geometry = Ptr::<GeometryData>::new(data);
    let skin = Ptr::<SkinPartition>::new(partition);
    let usage = e.get(entry, DeclarationEntry::uiUsage);
    let unset = |source: u32| Picked {
        source,
        size: 0,
        stride: 0,
        format: count,
    };
    let after_normals = |e: &Engine, element: u32| {
        let normals = e.get(geometry, GeometryData::m_pkNormal);
        let source = if normals != 0 && e.get(geometry, GeometryData::m_usDataFlags) & 0xf000 != 0 {
            let vertices = u32::from(e.get(geometry, GeometryData::m_usVertices));
            normals.wrapping_add(vertices.wrapping_mul(element))
        } else {
            0
        };
        Picked::fixed(source, 0xc, 3)
    };
    let picked = match usage.wrapping_add(1) {
        0 => return 0,
        1 => Picked::fixed(e.get(geometry, GeometryData::m_pkVertex), 0xc, 3),
        2 => {
            if partition == 0 {
                return 0;
            }
            let source = e.get(skin, SkinPartition::m_pfWeights);
            match e.get(skin, SkinPartition::m_usBonesPerVertex) {
                1 => Picked::fixed(source, 4, 1),
                2 => Picked::fixed(source, 8, 2),
                3 => Picked::fixed(source, 0xc, 3),
                4 => Picked {
                    source,
                    size: 0xc,
                    stride: 0x10,
                    format: 3,
                },
                _ => unset(source),
            }
        }
        3 => {
            if partition == 0 {
                return 0;
            }
            let source = e.get(skin, SkinPartition::m_pucBonePalette);
            if source == 0 {
                return 0;
            }
            match e.get(skin, SkinPartition::m_usBonesPerVertex) {
                1 => Picked::fixed(source, 1, 0x19),
                2 => Picked::fixed(source, 2, 0x1a),
                3 => Picked::fixed(source, 3, 0x1b),
                4 => Picked::fixed(source, 4, 0x1c),
                _ => unset(source),
            }
        }
        4 => Picked::fixed(e.get(geometry, GeometryData::m_pkNormal), 0xc, 3),
        5 => Picked::fixed(e.get(geometry, GeometryData::m_pkColor), 0x10, 4),
        6..=13 => {
            let set = usage.wrapping_sub(5) & 0xffff;
            let source: u32 =
                typed_call!(FAMILY, 0x00a6_6960, texture_set(e, geometry.cast(), set));
            Picked::fixed(source, 8, 2)
        }
        14 => after_normals(e, 0x18),
        15 => after_normals(e, 0xc),
        _ if (usage as i32) < 0 => e.with_stack(0x18, |e, slots| {
            // The argument slots the lookup writes through: the destination word (the source it
            // answers), the first argument (the format), the byte and the geometry data, then the
            // element size and the stride.
            let slots = slots.addr();
            e.mem.set_u32(slots, destination);
            e.mem.set_u32(slots + 4, count);
            e.mem.set_u32(slots + 8, flag);
            e.mem.set_u32(slots + 0xc, data);
            let additional = e.get(geometry, GeometryData::m_spAdditionalGeomData);
            let found = e
                .call(
                    ADDITIONAL_DATA,
                    &args![
                        additional,
                        usage & 0x7fff_ffff,
                        slots,
                        slots + 4,
                        slots + 8,
                        slots + 0xc,
                        slots + 0x10,
                        slots + 0x14
                    ],
                )
                .bool();
            if found {
                Picked {
                    source: e.mem.u32(slots),
                    size: e.mem.u32(slots + 0x10),
                    stride: e.mem.u32(slots + 0x14),
                    format: e.mem.u32(slots + 4),
                }
            } else {
                Picked::fixed(0, 0, 0)
            }
        }),
        _ => unset(0),
    };
    let list = e.mem.u32(converters.wrapping_add(8));
    let converter = e.mem.u32(list.wrapping_add(picked.format.wrapping_mul(4)));
    if converter == 0 {
        return 0;
    }
    // The exe reads the usage again here, after the calls above.
    let usage = e.get(entry, DeclarationEntry::uiUsage);
    let indices = if partition == 0 || usage == 1 || usage == 2 {
        0
    } else {
        e.get(skin, SkinPartition::m_pusVertexMap)
    };
    if picked.source == 0 {
        return 0;
    }
    e.with_stack(0x30, |e, packer| {
        let p = packer.cast::<PackerEntry>();
        e.set(p, PackerEntry::uiType, kind);
        e.set(p, PackerEntry::uiFormat, picked.format);
        e.set(p, PackerEntry::usCount, low_half(count));
        e.set(p, PackerEntry::pusIndex, indices);
        e.set(p, PackerEntry::pSource, picked.source);
        e.set(p, PackerEntry::uiElementSize, picked.size);
        e.set(p, PackerEntry::uiSourceStride, picked.stride);
        e.set(p, PackerEntry::uiWriteSize, write_size);
        e.set(p, PackerEntry::uiDestinationStride, destination_stride);
        e.set(p, PackerEntry::uiLast, last);
        e.set(p, PackerEntry::ucFlag, low_byte(flag));
        e.set(p, PackerEntry::pDestination, destination);
        convert(e, converter, p.addr())
    })
}

/// What the per-stream pack calls: the buffer data's update `00e8f040` (`thiscall`, one word), the
/// manager's buffer check `00e8bfa0` (`thiscall`: the buffer data and the stream; a `bool`), the new
/// chip `00e69b40` (`stdcall`-like: the buffer data and the stream; answers the chip), the lock
/// `00e8bff0` (`thiscall` on the manager: the buffer, offset, size, flags; answers the address), the
/// unlock `00e8c0d0` (`thiscall` on the manager: the buffer; a `bool`) and the derived vectors' pack
/// `00a855f0` (nine words: the geometry data that packs them, its positions, normals and vertex count,
/// the four destinations and the stride).
const BUFFER_DATA_UPDATE: u32 = 0x00e8_f040;
const CHIP_CHECK: u32 = 0x00e8_bfa0;
const NEW_CHIP: u32 = 0x00e6_9b40;
const LOCK: u32 = 0x00e8_bff0;
const UNLOCK: u32 = 0x00e8_c0d0;
const PACK_DERIVED: u32 = 0x00a8_55f0;

/// `D3DFMT_VERTEXDATA` (100): the format a chip's buffer must describe itself with to be kept.
const VERTEX_DATA_FORMAT: u32 = 100;

/// What a declaration entry's usage does in the per-stream pack (the exe's byte table `00e76a68` and
/// jump table `00e76a4c`).
enum Usage {
    /// Packed always.
    Pack,
    /// Packed when the flags have this bit.
    Bit(u8),
    /// Positions, normals, binormals and tangents: packed when the flags have this bit, unless the
    /// second geometry word packs them itself, which then gets the address in this slot (0
    /// binormals, 1 tangents, 2 normals, 3 positions).
    Derived(u8, usize),
}

fn usage_case(usage: u32) -> Usage {
    match usage {
        0 => Usage::Derived(1, 3),
        3 => Usage::Derived(2, 2),
        4 => Usage::Bit(4),
        5..=12 => Usage::Bit(8),
        13 => Usage::Derived(2, 0),
        14 => Usage::Derived(2, 1),
        _ => Usage::Pack,
    }
}

/// With `NV_HEAD_CHECK` set, one line saying this family's per-stream pack takes none of that
/// diagnostic's notes (the translation of `00e76770` does): `NV_TYPED=geometry:off` for them.
pub fn note_head_check() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    if std::env::var_os("NV_HEAD_CHECK").is_some() {
        ONCE.call_once(|| {
            println!(
                "head check: the geometry family's stream pack takes no notes; NV_TYPED=geometry:off for them"
            );
        });
    }
}

// Typed from 00e76770
/// The shader declaration's per-stream pack (its slot 0xac; unnamed in the engine map), `thiscall`
/// with nine words: the geometry data, a second geometry word (data that packs the derived vectors
/// itself; read for nothing else), the skin partition, the flags, the chip, the stream, a locked
/// address, and two words for the converters. The vertex count is the partition's (without one, the
/// data's slot 0x90); with none the chip is answered. The declaration's slot 0xb0 must answer set
/// (it gives a word for `00e8f040` and another), else 0. The buffer data (the partition's, else the
/// data's) is updated, takes the stream's stride and, with the renderer's byte +0x5d4 set, the byte the
/// declaration's slot 0xa8 answers. With a locked address the chip is used as it is; otherwise the chip
/// is kept when its vertex buffer describes itself (`GetDesc`, slot 0x34) as vertex data big enough
/// for the count times the declaration's slot 0xa0 for the stream, else a new one is made (`00e8bfa0`
/// must answer set, else 0; `00e69b40`; the flags become 0xfff), and it is locked (`00e8bff0`). Each
/// declaration entry of the stream is then packed ([`pack_entry`]) at the locked address plus its
/// offset: the derived vectors (usages 0, 3, 13, 14) only with their flag bit and, with the second word
/// and no partition, kept for `00a855f0` instead, which then packs them; usages 4 and 5 to 12 need the
/// flag bits 4 and 8. Without a locked address the chip's buffer is unlocked (`00e8c0d0`; 0 when that
/// answers 0). Answers the chip.
///
/// The translation also takes the `NV_HEAD_CHECK` diagnostic's notes here (not exe behaviour); this
/// version does not ([`note_head_check`]).
#[allow(clippy::too_many_arguments)]
pub fn pack_stream(
    e: &mut Engine,
    this: Ptr,
    data: u32,
    derived: u32,
    partition: u32,
    flags: u32,
    chip: u32,
    stream: u32,
    locked: u32,
    eighth: u32,
    ninth: u32,
) -> u32 {
    let declaration = this.cast::<ShaderDeclaration>();
    let base = this.addr();
    let geometry = Ptr::<GeometryData>::new(data);
    let skin = Ptr::<SkinPartition>::new(partition);
    let count = if partition != 0 {
        u32::from(e.get(skin, SkinPartition::m_usVertices))
    } else {
        e.vcall(data, 0x90, &args![]).u32() & 0xffff
    };
    if count == 0 {
        return chip;
    }
    // The locals passed by address: the two words of slot 0xb0 (+0, +4) and the vertex buffer's
    // description (+8: format, +0x18: size).
    e.with_stack(0x20, |e, locals| {
        let locals = locals.addr();
        let first = locals;
        let second = locals + 4;
        let description = locals + 8;
        if !e.vcall(base, 0xb0, &args![first, second]).bool() {
            return 0;
        }
        let buffer_data = if partition != 0 {
            e.get(skin, SkinPartition::m_pkBuffData)
        } else {
            e.get(geometry, GeometryData::m_pkBuffData)
        };
        let buffers = Ptr::<GeometryBufferData>::new(buffer_data);
        let word = e.mem.u32(first);
        e.call(BUFFER_DATA_UPDATE, &args![buffer_data, word]);
        if stream < e.get(buffers, GeometryBufferData::m_uiStreamCount) {
            let streams = e.get(declaration, ShaderDeclaration::m_pkStreamEntries);
            let stream_entry =
                Ptr::<StreamEntry>::new(streams.wrapping_add(stream.wrapping_shl(4)));
            let stride = e.get(stream_entry, StreamEntry::uiStride);
            let strides = e.get(buffers, GeometryBufferData::m_puiVertexStride);
            e.mem
                .set_u32(strides.wrapping_add(stream.wrapping_mul(4)), stride);
        }
        let renderer = e.get(declaration, ShaderDeclaration::m_pkRenderer);
        if e.mem.u8(renderer.wrapping_add(0x5d4)) != 0 {
            let byte = low_byte(e.vcall(base, 0xa8, &args![]).u32());
            e.set(buffers, GeometryBufferData::m_bSoftwareVP, byte);
        }
        let mut unlock = true;
        let mut flags = flags;
        let mut result = chip;
        let mut address = locked;
        if locked != 0 {
            unlock = false;
        } else {
            let mut keep = false;
            if chip != 0 {
                let buffer = e.get(Ptr::<VBChip>::new(chip), VBChip::pVB);
                if buffer != 0 {
                    e.vcall(buffer, 0x34, &args![description]);
                    if e.mem.u32(description) == VERTEX_DATA_FORMAT {
                        let per_vertex = e.vcall(base, 0xa0, &args![stream]).u32();
                        if e.mem.u32(description + 0x10) >= per_vertex.wrapping_mul(count) {
                            keep = true;
                        }
                    }
                }
            }
            if !keep {
                let manager = e.get(declaration, ShaderDeclaration::m_pkVBManager);
                if !e
                    .call(CHIP_CHECK, &args![manager, buffer_data, stream])
                    .bool()
                {
                    return 0;
                }
                result = e.call(NEW_CHIP, &args![buffer_data, stream]).u32();
                flags = 0xfff;
            }
            let manager = e.get(declaration, ShaderDeclaration::m_pkVBManager);
            let used = Ptr::<VBChip>::new(result);
            let buffer = e.get(used, VBChip::pVB);
            let offset = e.get(used, VBChip::uiOffset);
            let size = e.get(used, VBChip::uiSize);
            let lock_flags = e.get(used, VBChip::uiLockFlags);
            address = e
                .call(LOCK, &args![manager, buffer, offset, size, lock_flags])
                .u32();
        }
        let packs_itself = derived != 0 && partition == 0;
        let stream_entry = e
            .get(declaration, ShaderDeclaration::m_pkStreamEntries)
            .wrapping_add(stream.wrapping_shl(4));
        // The derived vectors' addresses, by their slot.
        let mut kept = [0u32; 4];
        if stream_entry != 0 {
            let stream_ptr = Ptr::<StreamEntry>::new(stream_entry);
            let mut index = 0u32;
            let mut offset = 0u32;
            while index < e.get(declaration, ShaderDeclaration::m_uiMaxStreamEntryCount) {
                let entries = e.get(stream_ptr, StreamEntry::pEntries);
                let at = Ptr::<DeclarationEntry>::new(entries.wrapping_add(offset));
                let target = e.get(at, DeclarationEntry::uiOffset).wrapping_add(address);
                let pack = match usage_case(e.get(at, DeclarationEntry::uiUsage)) {
                    Usage::Pack => true,
                    Usage::Bit(bit) => low_byte(flags) & bit != 0,
                    Usage::Derived(bit, slot) => {
                        if low_byte(flags) & bit == 0 {
                            false
                        } else if packs_itself {
                            kept[slot] = target;
                            false
                        } else {
                            true
                        }
                    }
                };
                if pack {
                    let step = e.get(stream_ptr, StreamEntry::uiStride);
                    let _: u32 = typed_call!(
                        FAMILY,
                        0x00e7_a200,
                        pack_entry(
                            e,
                            this,
                            count,
                            data,
                            derived,
                            partition,
                            at.addr(),
                            step,
                            target,
                            eighth,
                            ninth
                        )
                    );
                }
                index += 1;
                offset = offset.wrapping_add(0x1c);
            }
        }
        if packs_itself {
            let step = e.get(Ptr::<StreamEntry>::new(stream_entry), StreamEntry::uiStride);
            let vertices = e.get(geometry, GeometryData::m_pkVertex);
            let normals = e.get(geometry, GeometryData::m_pkNormal);
            let vertex_count = u32::from(e.get(geometry, GeometryData::m_usVertices));
            e.call(
                PACK_DERIVED,
                &args![
                    derived,
                    vertices,
                    normals,
                    vertex_count,
                    kept[3],
                    kept[2],
                    kept[1],
                    kept[0],
                    step
                ],
            );
        }
        if unlock {
            let manager = e.get(declaration, ShaderDeclaration::m_pkVBManager);
            let buffer = e.get(Ptr::<VBChip>::new(result), VBChip::pVB);
            if !e.call(UNLOCK, &args![manager, buffer]).bool() {
                return 0;
            }
        }
        result
    })
}

// Typed from 00a66960
/// `NiGeometryData::GetTextureSet` (Xbox PDB), `thiscall` with the set: answers the texture
/// coordinates (`m_pkTexture`); the set is not read (the PC keeps one set: the loader warns about more
/// and drops them).
pub fn texture_set(e: &mut Engine, this: Ptr, _set: u32) -> u32 {
    e.get(this.cast::<GeometryData>(), GeometryData::m_pkTexture)
}

// Typed from 00e8c090
/// The fixed-function packer's strided copy (unnamed in the engine map; `thiscall` on the manager,
/// which it does not read, five words): `count` elements of `size` bytes from consecutive source
/// elements into destination elements `stride` bytes apart, a `memcpy` each.
pub fn copy_strided(
    e: &mut Engine,
    _manager: Ptr,
    destination: u32,
    source: u32,
    stride: u32,
    size: u32,
    count: u32,
) {
    let (mut to, mut from) = (destination, source);
    for _ in 0..count {
        e.call(MEMCPY, &args![to, from, size]);
        to = to.wrapping_add(stride);
        from = from.wrapping_add(size);
    }
}

/// What the fixed-function packer calls: the manager's vertex layout `00e8c120` (`thiscall`, nine
/// words: the geometry data, 0 and the addresses of the format, stride and the position, unused,
/// normal, colour and texture offsets it fills), the buffer data's format `00e8f010` (`thiscall`, one
/// word), the new buffer `00e8bfa0` and the lock `00e8bff0` (above), the skinned positions' pack
/// `00687d40` (`thiscall` on the skin object: the geometry data, the position and normal destinations,
/// 0, 0, the stride) and the imported `LeaveCriticalSection` (import slot `00fdf100`).
const VERTEX_LAYOUT: u32 = 0x00e8_c120;
const SET_FORMAT: u32 = 0x00e8_f010;
const PACK_SKINNED_POSITIONS: u32 = 0x0068_7d40;
const LEAVE_CRITICAL_SECTION: u32 = 0x00fd_f100;
/// The most texture sets (`011bf558`).
const TEXTURE_SET_LIMIT: u32 = 0x011b_f558;

// Typed from 00e8c1e0
/// `NiXenonVertexBufferManager::PackUntransformedVB` (engine map name), `thiscall` with five words:
/// the geometry data, a skin object or null, the parts to pack (1 positions, 2 normals, 4 colours, 8
/// texture coordinates), a chip to reuse, and a write address when the caller has locked.
///
/// Without active vertices (the data's slot 0x90, 16 bits) the chip is answered at once. Otherwise the
/// layout (`00e8c120`) gives the format, stride and offsets; the buffer data gets the format (`00e8f010`)
/// and, when it has streams, the stride as its first. With a write address the chip is used as it is
/// and nothing is unlocked. Otherwise the chip is kept when its vertex buffer describes itself
/// (`GetDesc`) as vertex data of the same format and at least the count times the stride; if not, a new
/// buffer is made (`00e8bfa0`, 0 answered on failure) and its first chip (or null) used, with every
/// part packed (0xfff); the chip is locked (`00e8bff0`).
///
/// Then, from the write address: with a skin object, positions (and normals when the data has them) go
/// through `00687d40`; without one, positions and normals are copied as 12-byte elements
/// ([`copy_strided`]). Colours (four floats each) are stored as `a << 24 | r << 16 | g << 8 | b` (each
/// times 255 rounded, [`channel`]) and the texture sets (the data flags' bit 0, at most `011bf558`) as
/// 8-byte pairs from [`texture_set`], each at its offset in every vertex. Last, when the manager
/// locked: its pending copy (the staging block to the locked address) is made and cleared, its lock
/// left and the vertex buffer unlocked (0 answered on failure). Answers the chip.
pub fn pack_untransformed(
    e: &mut Engine,
    this: Ptr,
    data: Ptr,
    skin: u32,
    parts: u32,
    chip: u32,
    write: u32,
) -> u32 {
    let manager = this.cast::<VertexBufferManager>();
    let geometry = data.cast::<GeometryData>();
    let count = u32::from(e.vcall(data.addr(), 0x90, &args![]).u16());
    if count == 0 {
        return chip;
    }
    let normals = e.get(geometry, GeometryData::m_pkNormal);
    let vertices = e.get(geometry, GeometryData::m_pkVertex);
    let mut texture_sets = u32::from(e.get(geometry, GeometryData::m_usDataFlags)) & 1;
    let limit = e.global::<u32>(TEXTURE_SET_LIMIT);
    let colours = e.get(geometry, GeometryData::m_pkColor);
    if texture_sets > limit {
        texture_sets = limit;
    }
    // The layout's outputs (format, stride, position, unused, normal, colour and texture offsets: the
    // exe's locals), then the vertex buffer's description.
    e.with_stack(0x1c + 0x18, |e, frame| {
        let frame = frame.addr();
        let [format_at, stride_at, position_at, unused_at, normal_at, colour_at, texture_at] =
            [0u32, 4, 8, 0xc, 0x10, 0x14, 0x18].map(|offset| frame + offset);
        let description = frame + 0x1c;
        e.mem.set_u32(format_at, 0);
        e.mem.set_u32(stride_at, 0);
        e.call(
            VERTEX_LAYOUT,
            &args![
                this.addr(),
                data.addr(),
                0u32,
                format_at,
                stride_at,
                position_at,
                unused_at,
                normal_at,
                colour_at,
                texture_at
            ],
        );
        let buffer_data = e.get(geometry, GeometryData::m_pkBuffData);
        let buffers = Ptr::<GeometryBufferData>::new(buffer_data);
        let format = e.mem.u32(format_at);
        e.call(SET_FORMAT, &args![buffer_data, format]);
        if e.get(buffers, GeometryBufferData::m_uiStreamCount) > 0 {
            let strides = e.get(buffers, GeometryBufferData::m_puiVertexStride);
            let stride = e.mem.u32(stride_at);
            e.mem.set_u32(strides, stride);
        }
        let mut parts = parts;
        let mut locked_here = true;
        let mut used = chip;
        let mut out = write;
        if write != 0 {
            locked_here = false;
        } else {
            let mut reuse = false;
            if chip != 0 {
                let buffer = e.get(Ptr::<VBChip>::new(chip), VBChip::pVB);
                if buffer != 0 {
                    e.vcall(buffer, 0x34, &args![description]);
                    reuse = e.mem.u32(description) == VERTEX_DATA_FORMAT
                        && e.mem.u32(description + 0x14) == e.mem.u32(format_at)
                        && e.mem.u32(description + 0x10)
                            >= (count & 0xffff).wrapping_mul(e.mem.u32(stride_at));
                }
            }
            if !reuse {
                if !e
                    .call(CHIP_CHECK, &args![this.addr(), buffer_data, 0u32])
                    .bool()
                {
                    return 0;
                }
                used = if e.get(buffers, GeometryBufferData::m_uiStreamCount) > 0 {
                    e.mem.u32(e.get(buffers, GeometryBufferData::m_ppkVBChip))
                } else {
                    0
                };
                parts = 0xfff;
            }
            let chosen = Ptr::<VBChip>::new(used);
            let words = args![
                this.addr(),
                e.get(chosen, VBChip::pVB),
                e.get(chosen, VBChip::uiOffset),
                e.get(chosen, VBChip::uiSize),
                e.get(chosen, VBChip::uiLockFlags)
            ];
            out = e.call(LOCK, &words).u32();
        }
        let stride = |e: &Engine| e.mem.u32(stride_at);
        if skin != 0 {
            if vertices != 0 && parts & 1 != 0 {
                let positions = e.mem.u32(position_at).wrapping_add(out);
                let normal_out = if normals != 0 {
                    e.mem.u32(normal_at).wrapping_add(out)
                } else {
                    0
                };
                let words = args![
                    skin,
                    data.addr(),
                    positions,
                    normal_out,
                    0u32,
                    0u32,
                    stride(e)
                ];
                e.call(PACK_SKINNED_POSITIONS, &words);
            }
        } else {
            if vertices != 0 && parts & 1 != 0 {
                let to = e.mem.u32(position_at).wrapping_add(out);
                let step = stride(e);
                typed_call!(
                    FAMILY,
                    0x00e8_c090,
                    copy_strided(e, this, to, vertices, step, 0xc, count)
                );
            }
            if normals != 0 && parts & 2 != 0 {
                let to = e.mem.u32(normal_at).wrapping_add(out);
                let step = stride(e);
                typed_call!(
                    FAMILY,
                    0x00e8_c090,
                    copy_strided(e, this, to, normals, step, 0xc, count)
                );
            }
        }
        if colours != 0 && parts & 4 != 0 {
            let scale = f64::from_bits(e.mem.u64(CHANNEL_SCALE));
            let mut to = e.mem.u32(colour_at).wrapping_add(out);
            let mut from = colours;
            for _ in 0..count {
                let alpha = channel(e.mem.u32(from.wrapping_add(0xc)), scale);
                let red = channel(e.mem.u32(from), scale);
                let green = channel(e.mem.u32(from.wrapping_add(4)), scale);
                let blue = channel(e.mem.u32(from.wrapping_add(8)), scale);
                from = from.wrapping_add(0x10);
                e.mem.set_u32(to, pack_channels(alpha, red, green, blue));
                to = to.wrapping_add(stride(e));
            }
        }
        if parts & 8 != 0 && texture_sets > 0 {
            let mut base = out.wrapping_add(e.mem.u32(texture_at));
            for set in 0..texture_sets {
                let mut from: u32 = typed_call!(FAMILY, 0x00a6_6960, texture_set(e, data, set));
                let mut to = base;
                for _ in 0..count {
                    let low = e.mem.u32(from);
                    e.mem.set_u32(to, low);
                    let high = e.mem.u32(from.wrapping_add(4));
                    e.mem.set_u32(to.wrapping_add(4), high);
                    to = to.wrapping_add(stride(e));
                    from = from.wrapping_add(8);
                }
                base = base.wrapping_add(8);
            }
        }
        if locked_here {
            let size = e.get(manager, VertexBufferManager::uiLockedSize);
            let source = e.get(manager, VertexBufferManager::pStaging);
            let buffer = e.get(Ptr::<VBChip>::new(used), VBChip::pVB);
            let destination = e.get(manager, VertexBufferManager::pLocked);
            e.call(MEMCPY, &args![destination, source, size]);
            e.set(manager, VertexBufferManager::pLocked, 0);
            e.set(manager, VertexBufferManager::uiLockedSize, 0);
            e.call(
                LEAVE_CRITICAL_SECTION,
                &args![this.addr().wrapping_add(0x80)],
            );
            if e.vcall(buffer, 0x30, &args![]).i32() < 0 {
                return 0;
            }
        }
        used
    })
}
