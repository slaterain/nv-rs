//! The differential tests of the geometry family (`engine_typed::geometry`) against the translations:
//! the renderer's packing of a geometry's arrays (`pack`), the bound, the data's setters and the morpher
//! (`bound`, `data`, `morph`), where the arrays begin and end (`lifecycle`), the tangent space (`tangent`)
//! and the pick of a geometry's triangles (`pick`). Each batch's stand-ins are described where its tests
//! begin; the first batch's here.
//!
//! What stands in: the declaration's, the geometry data's and the vertex buffer's virtual functions
//! (test functions past `.text` answering a word per slot from a table; the ones that fill words they
//! are given the addresses of fill them from the table too), the buffer data's update, the manager's
//! buffer check, new chip, lock and unlock, the derived vectors' pack, the skinned positions' pack, the
//! layout, the format set, the additional data's lookup, the critical section import and a converter
//! for the formats this family does not type. The C runtime's `memcpy`, `memset` and `_ftol2` are the
//! real translations on both sides.

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::{Case, Diff, Input};
use engine_core::{entry, Engine};
use engine_typed::geometry::{self, pack};

/// The stand-in virtual functions, one per slot, and the table of what each answers.
const VT_FN: u32 = 0x00ff_2000;
const RETS: u32 = 0x0300_0000;
/// The stand-in converter (formats this family does not type), and where it leaves its mark.
const OTHER_CONVERTER: u32 = 0x00ff_3000;
const MARK: u32 = RETS + 0x800;
const MAX_CALLS: usize = 20_000;

/// The exe's constants and tables these functions read.
const CHANNEL_SCALE: u32 = 0x0101_e568;
const WHITE_CHANNEL: u32 = 0x0102_3cd8;
const TEXTURE_SET_LIMIT: u32 = 0x011b_f558;
const WRITE_SIZES: u32 = 0x0126_f508;
const CONVERTER_TABLES: u32 = 0x0126_f550;
const SSE2_AVAILABLE: u32 = 0x0127_0a6c;

/// The family's converters by address.
const COPY_SAME: u32 = 0x00e7_8f80;
const COPY_NARROWER: u32 = 0x00e7_8d50;
const COPY_WIDER: u32 = 0x00e7_8e30;
const COLOUR: u32 = 0x00e7_9d00;
const FLOATS_TO_BYTES: u32 = 0x00e7_7950;
const FLOATS_TO_SHORTS: u32 = 0x00e7_6c30;

fn ret(eax: u32) -> Ret {
    Ret {
        eax,
        ..Ret::default()
    }
}

fn slot_answer<const S: u32>(e: &mut Engine, _: &[u32]) -> Ret {
    ret(e.mem.u32(RETS + S))
}

macro_rules! slots {
    ($e:ident; $($s:literal),* $(,)?) => { $( $e.register(VT_FN + $s, slot_answer::<$s>); )* };
}

fn set(e: &mut Engine, at: u32, value: u32) {
    e.mem.set_u32(at, value);
}

/// A vtable of 0x200 bytes of stand-ins.
fn vtable(e: &mut Engine) -> u32 {
    let vt = e.mem.alloc(0x200);
    for slot in (0..0x200).step_by(4) {
        set(e, vt + slot, VT_FN + slot);
    }
    vt
}

/// An object of `size` bytes with vtable `vt`.
fn obj(e: &mut Engine, size: u32, vt: u32) -> u32 {
    let o = e.mem.alloc(size);
    set(e, o, vt);
    o
}

/// A block of `bytes` bytes, each word a pattern moved by `seed`.
fn block(e: &mut Engine, bytes: u32, seed: u32) -> u32 {
    let b = e.mem.alloc(bytes);
    for w in 0..bytes / 4 {
        set(
            e,
            b + 4 * w,
            (seed << 20) ^ (w.wrapping_mul(0x9e37_79b9) >> 8),
        );
    }
    b
}

/// Floats the converters see beside the case's: ties of the channel rounding (0.5 * 255 is 127.5),
/// values that round, negatives, values past a byte and a 16-bit integer, and the ones they meet.
const FLOATS: [f32; 12] = [
    0.5,
    1.0,
    -0.25,
    0.999,
    0.003_921_568_6,
    0.25,
    2.0,
    -1.0,
    300.5,
    -70_000.0,
    65_535.5,
    0.0,
];

/// The constants and tables, the stand-in slots and the stand-in converter.
fn globals(e: &mut Engine) {
    for (addr, len) in [
        (0x0101_0000, 0x20000),
        (0x011b_f000, 0x1000),
        (0x0126_f000, 0x1000),
        (0x0127_0000, 0x1000),
        (RETS, 0x1000),
    ] {
        e.map(addr, len);
    }
    e.mem.set_u64(CHANNEL_SCALE, 255.0f64.to_bits());
    set(e, WHITE_CHANNEL, 255.0f32.to_bits());
    set(e, SSE2_AVAILABLE, 1);
    slots!(e; 0x30, 0x90, 0xa0, 0xa8);
    e.register(OTHER_CONVERTER, |e, a| {
        // Its mark: the packer entry's words folded, so a field written wrong shows.
        let mut fold = e.mem.u32(MARK);
        for w in 0..12 {
            fold = fold.rotate_left(5) ^ e.mem.u32(a[0] + 4 * w);
        }
        set(e, MARK, fold);
        ret(0x1234)
    });
}

// The converters.

/// What a converter test's packer entry holds.
#[derive(Clone, Copy)]
struct Packer {
    ty: u32,
    format: u32,
    count: u32,
    size: u32,
    stride: u32,
    write: u32,
    dstride: u32,
}

/// A converter's packer entry and its arrays. `source`: 0 none, 1 plain, 2 indexed. The source has
/// six elements `stride` bytes apart, its words from `fill`; the index array picks 4, 0, 2, 5, 1, 3;
/// the destination has room for six vertices and a tail, every byte 0xcd, so what a converter leaves
/// shows.
fn packer(e: &mut Engine, source: u32, p: Packer, fill: &dyn Fn(u32) -> u32) -> u32 {
    globals(e);
    let entry = e.mem.alloc(0x30);
    let src = if source == 0 {
        0
    } else {
        let bytes = 6 * p.stride.max(p.size) + 0x10;
        let s = e.mem.alloc(bytes);
        for w in 0..bytes / 4 {
            set(e, s + 4 * w, fill(w));
        }
        s
    };
    let index = if source == 2 {
        let i = e.mem.alloc(0x10);
        for (k, v) in [4u16, 0, 2, 5, 1, 3].iter().enumerate() {
            e.mem.set_u16(i + 2 * k as u32, *v);
        }
        i
    } else {
        0
    };
    let bytes = 6 * p.dstride.max(p.write).max(0x20) + 0x40;
    let dst = e.mem.alloc(bytes);
    for b in 0..bytes {
        e.mem.set_u8(dst + b, 0xcd);
    }
    set(e, entry, p.ty);
    set(e, entry + 4, p.format);
    set(e, entry + 8, p.count);
    set(e, entry + 0xc, index);
    set(e, entry + 0x10, src);
    set(e, entry + 0x14, p.size);
    set(e, entry + 0x18, p.stride);
    set(e, entry + 0x1c, p.write);
    set(e, entry + 0x20, p.dstride);
    set(e, entry + 0x24, 0x77);
    set(e, entry + 0x28, 0x6666_6666);
    set(e, entry + 0x2c, dst);
    entry
}

/// The source words of the float converters: the case's float in every third word, [`FLOATS`] between.
fn floats_with(value: f32) -> impl Fn(u32) -> u32 {
    move |w| {
        if w % 3 == 0 {
            value.to_bits()
        } else {
            FLOATS[(w as usize) % FLOATS.len()].to_bits()
        }
    }
}

/// A pattern of words for the plain copies.
fn pattern(w: u32) -> u32 {
    0x1111_1111u32.wrapping_mul(w + 1) ^ 0xa5
}

const SOURCES: &[u32] = &[0, 1, 2];
/// Counts: none, one, a few, all six; the high half of the count's word is not read.
const COUNTS: &[u32] = &[0, 1, 3, 0x0001_0006];

/// `00e78f80`: no source, plain, indexed; the copy sizes 8, 12, 16 and two through `memcpy`; strides
/// with a gap or none.
#[test]
fn copy_same() {
    Diff::new(entry!(0x00e7_8f80, pack::copy_same(u32) -> u32))
        .family(&geometry::GEOMETRY)
        .inputs(&[
            Input::Of(SOURCES),
            Input::Of(COUNTS),
            Input::Of(&[4, 8, 0xc, 0x10, 0x14]),
            Input::Of(&[0, 4]),
            Input::Of(&[0x10, 0x20]),
        ])
        .setup(|e, c| {
            let size = c.word(2);
            let p = Packer {
                ty: 2,
                format: 3,
                count: c.word(1),
                size,
                stride: size + c.word(3),
                write: size,
                dstride: c.word(4),
            };
            vec![packer(e, c.word(0), p, &pattern)]
        })
        .run()
        .assert_same();
}

/// `00e78d50`: the size a vertex below the element's.
#[test]
fn copy_narrower() {
    Diff::new(entry!(0x00e7_8d50, pack::copy_narrower(u32) -> u32))
        .family(&geometry::GEOMETRY)
        .inputs(&[
            Input::Of(SOURCES),
            Input::Of(COUNTS),
            Input::Of(&[4, 8, 0xc]),
            Input::Of(&[0x10, 0x14]),
            Input::Of(&[0x10, 0x20]),
        ])
        .setup(|e, c| {
            let p = Packer {
                ty: c.word(2) / 4 - 1,
                format: 4,
                count: c.word(1),
                size: 0x10,
                stride: c.word(3),
                write: c.word(2),
                dstride: c.word(4),
            };
            vec![packer(e, c.word(0), p, &pattern)]
        })
        .run()
        .assert_same();
}

/// `00e78e30`: (type, format) pairs a wider destination takes, the words after the element cleared.
#[test]
fn copy_wider() {
    const PAIRS: [(u32, u32); 6] = [(0, 1), (1, 1), (2, 1), (3, 1), (3, 2), (2, 3)];
    Diff::new(entry!(0x00e7_8e30, pack::copy_wider(u32) -> u32))
        .family(&geometry::GEOMETRY)
        .inputs(&[
            Input::Of(SOURCES),
            Input::Of(COUNTS),
            Input::Of(&[0, 1, 2, 3, 4, 5]),
            Input::Of(&[0, 4]),
            Input::Of(&[0x10, 0x20]),
        ])
        .setup(|e, c| {
            let (ty, format) = PAIRS[c.word(2) as usize];
            let p = Packer {
                ty,
                format,
                count: c.word(1),
                size: 4 * format,
                stride: 4 * format + c.word(3),
                write: 4 * (ty + 1),
                dstride: c.word(4),
            };
            vec![packer(e, c.word(0), p, &pattern)]
        })
        .run()
        .assert_same();
}

/// `00e79d00`: the colour words of three and four floats (alpha 1.0 for three), every float the
/// harness has in the channels, and no source (the white word).
#[test]
fn colour() {
    Diff::new(entry!(0x00e7_9d00, pack::colour(u32) -> u32))
        .family(&geometry::GEOMETRY)
        .inputs(&[
            Input::Of(SOURCES),
            Input::Of(COUNTS),
            Input::Of(&[3, 4, 0x0001_0003]),
            Input::F32,
        ])
        .max_cases(512)
        .float_sweep(64)
        .setup(|e, c| {
            let p = Packer {
                ty: 4,
                format: c.word(2),
                count: c.word(1),
                size: 0x10,
                stride: 0x10,
                write: 4,
                dstride: 0x10,
            };
            vec![packer(e, c.word(0), p, &floats_with(c.f32(3)))]
        })
        .run()
        .assert_same();
}

/// `00e77950`: one to four floats (and a format past four) to bytes under chop, the rest of the four
/// bytes cleared.
#[test]
fn floats_to_bytes() {
    Diff::new(entry!(0x00e7_7950, pack::floats_to_bytes(Ptr) -> u32))
        .family(&geometry::GEOMETRY)
        .inputs(&[
            Input::Of(SOURCES),
            Input::Of(COUNTS),
            Input::Of(&[0, 1, 2, 3, 4, 5]),
            Input::F32,
        ])
        .max_cases(512)
        .float_sweep(64)
        .setup(|e, c| {
            let p = Packer {
                ty: 5,
                format: c.word(2),
                count: c.word(1),
                size: 0x10,
                stride: 0x14,
                write: 4,
                dstride: 8,
            };
            vec![packer(e, c.word(0), p, &floats_with(c.f32(3)))]
        })
        .run()
        .assert_same();
}

/// `00e76c30`: types 6 (two shorts) and 7 (four), one to four floats through `_ftol2` (both of its
/// paths), the rest of the width cleared.
#[test]
fn floats_to_shorts() {
    Diff::new(entry!(0x00e7_6c30, pack::floats_to_shorts(Ptr) -> u32))
        .family(&geometry::GEOMETRY)
        .inputs(&[
            Input::Of(SOURCES),
            Input::Of(COUNTS),
            Input::Of(&[6, 7]),
            Input::Of(&[0, 1, 2, 3, 4, 5]),
            Input::Flag,
            Input::F32,
        ])
        .max_cases(512)
        .float_sweep(64)
        .setup(|e, c| {
            let p = Packer {
                ty: c.word(2),
                format: c.word(3),
                count: c.word(1),
                size: 0x10,
                stride: 0x10,
                write: 8,
                dstride: 8,
            };
            let entry = packer(e, c.word(0), p, &floats_with(c.f32(5)));
            set(e, SSE2_AVAILABLE, c.word(4));
            vec![entry]
        })
        .run()
        .assert_same();
}

// The element pack.

/// The write sizes (`0126f508`) and the converter tables (`0126f550`): for each type a table whose +8
/// lists a converter by format as `00e78560` stores them for the float formats (the family's), the
/// stand-in for the integer formats 0x19 to 0x1c, none for format 0 and anything past 0x1c; type 7 has
/// no table.
fn converter_tables(e: &mut Engine) {
    for (kind, size) in [4u32, 8, 0xc, 0x10, 4, 4, 4, 8].iter().enumerate() {
        set(e, WRITE_SIZES + 4 * kind as u32, *size);
    }
    let root = e.mem.alloc(0x10);
    set(e, CONVERTER_TABLES, root);
    let kinds = e.mem.alloc(0x20);
    set(e, root + 4, kinds);
    for kind in 0..7u32 {
        let table = e.mem.alloc(0x10);
        let list = e.mem.alloc(0x21 * 4);
        set(e, table + 8, list);
        set(e, kinds + 4 * kind, table);
        for format in 1..=4u32 {
            let converter = match kind {
                0..=3 if format - 1 == kind => COPY_SAME,
                0..=3 if format - 1 < kind => COPY_WIDER,
                0..=3 => COPY_NARROWER,
                4 if format >= 3 => COLOUR,
                4 => OTHER_CONVERTER,
                5 => FLOATS_TO_BYTES,
                _ => FLOATS_TO_SHORTS,
            };
            set(e, list + 4 * format, converter);
        }
        for format in 0x19..=0x1c {
            set(e, list + 4 * format, OTHER_CONVERTER);
        }
    }
}

/// A geometry data of six vertices with every array (the normals three normals long), flags `flags`,
/// its additional data and its buffer data; with `vt` its vtable.
fn geometry_data(e: &mut Engine, vt: u32, flags: u32) -> u32 {
    let data = obj(e, 0x40, vt);
    e.mem.set_u16(data + 8, 6);
    e.mem.set_u16(data + 0xc, flags as u16);
    let floats = |e: &mut Engine, bytes: u32, seed: u32| {
        let b = e.mem.alloc(bytes);
        for w in 0..bytes / 4 {
            let value = FLOATS[((w + seed) as usize) % FLOATS.len()] * (1.0 + seed as f32);
            set(e, b + 4 * w, value.to_bits());
        }
        b
    };
    let vertices = floats(e, 6 * 12, 1);
    let normals = floats(e, 3 * 6 * 12, 2);
    let colours = floats(e, 6 * 16, 0);
    let uvs = floats(e, 6 * 8, 3);
    set(e, data + 0x20, vertices);
    set(e, data + 0x24, normals);
    set(e, data + 0x28, colours);
    set(e, data + 0x2c, uvs);
    let additional = e.mem.alloc(0x10);
    set(e, data + 0x30, additional);
    data
}

/// A skin partition of three vertices with `bones` bones a vertex (none: 0), its weights, vertex map
/// and, unless `palette` is clear, its bone palette.
fn partition(e: &mut Engine, bones: u32, palette: bool) -> u32 {
    if bones == 0 {
        return 0;
    }
    let p = e.mem.alloc(0x2c);
    let weights = block(e, 6 * 16, 7);
    set(e, p + 8, weights);
    let map = e.mem.alloc(0x10);
    for (k, v) in [2u16, 0, 5, 1].iter().enumerate() {
        e.mem.set_u16(map + 2 * k as u32, *v);
    }
    set(e, p + 0xc, map);
    let bone_palette = if palette { block(e, 0x20, 9) } else { 0 };
    set(e, p + 0x10, bone_palette);
    e.mem.set_u16(p + 0x1c, 3);
    e.mem.set_u16(p + 0x24, (bones - 1) as u16);
    p
}

/// The additional data's lookup `00a72720`: answers from the table whether it found the stream, and
/// fills the source (the data's colours, or null when the table says so), the format (from the table),
/// the element size and stride.
fn additional_lookup(e: &mut Engine) {
    e.register(0x00a7_2720, |e, a| {
        let found = e.mem.u32(RETS + 0x720);
        if found != 0 {
            let data = e.mem.u32(a[5]);
            let colours = e.mem.u32(data + 0x28);
            // Found with a null source when the table says so.
            let source = if e.mem.u32(RETS + 0x728) != 0 {
                0
            } else {
                colours
            };
            set(e, a[2], source);
            set(e, a[3], e.mem.u32(RETS + 0x724));
            set(e, a[6], 0xc);
            set(e, a[7], 0x10);
        }
        ret(found)
    });
}

/// `00e7a200`: every usage (none, positions, weights, bone indices, normals, colours, the first and
/// last texture sets, binormals, tangents, one past them, an additional stream), every type (the
/// colour type with usage 2 counts as type 5; type 7 has no table; 0x11 is unused), a partition with
/// one to four bones a vertex, five, or none, the data flags with and without the vectors after the
/// normals, the count's high half, the additional stream found (as a float format or the stand-in's)
/// or not.
#[test]
fn pack_entry() {
    Diff::new(entry!(
        0x00e7_a200,
        pack::pack_entry(Ptr, u32, u32, u32, u32, u32, u32, u32, u32, u32) -> u32
    ))
    .family(&geometry::GEOMETRY)
    .max_calls(MAX_CALLS)
    // Usage 15, and the weights or bone indices of a partition with no bones a vertex, leave the
    // format the exe reuses as the first word, the count: with its high half set the converter list
    // is read far past its end, on both sides.
    .expect_stops(|c| {
        let (usage, ty, bones, count) = (c.word(0), c.word(1), c.word(2), c.word(4));
        let packs = ty != 0x11 && ty != 7;
        let unset = usage == 15 || (bones == 1 && (usage == 1 || usage == 2));
        packs && unset && count > 0x20
    })
    .inputs(&[
        Input::Of(&[0xffff_ffff, 0, 1, 2, 3, 4, 5, 12, 13, 14, 15, 0x8000_0002]),
        Input::Of(&[0, 1, 2, 3, 4, 5, 6, 7, 0x11]),
        Input::Of(&[0, 1, 2, 3, 4, 5, 6]),
        Input::Of(&[0, 0xf000]),
        Input::Of(&[3, 0x0001_0003]),
        Input::Of(&[0, 3, 0x19]),
    ])
    .max_cases(1200)
    .setup(|e, c| {
        globals(e);
        converter_tables(e);
        additional_lookup(e);
        let vt = vtable(e);
        let data = geometry_data(e, vt, c.word(3));
        // Bones 6 means four with no palette.
        let bones = c.word(2);
        let skin = if bones == 6 {
            partition(e, 5, false)
        } else {
            partition(e, bones, true)
        };
        set(e, RETS + 0x720, u32::from(c.word(5) != 0));
        set(e, RETS + 0x724, c.word(5));
        let declaration = e.mem.alloc(0x1c);
        set(e, declaration + 4, 0x10);
        set(e, declaration + 8, c.word(0));
        set(e, declaration + 0xc, c.word(1));
        let destination = e.mem.alloc(0x200);
        for b in 0..0x200 {
            e.mem.set_u8(destination + b, 0xcd);
        }
        let this = e.mem.alloc(0x10);
        vec![
            this,
            c.word(4),
            data,
            0x5555,
            skin,
            declaration,
            0x20,
            destination,
            0x1_0001,
            0x99,
        ]
    })
    .run()
    .assert_same();
}

/// `00a66960`: the texture coordinates whatever the set.
#[test]
fn texture_set() {
    Diff::new(entry!(0x00a6_6960, pack::texture_set(Ptr, u32) -> u32))
        .family(&geometry::GEOMETRY)
        .inputs(&[Input::Of(&[0, 5, 0xffff_ffff])])
        .setup(|e, c| {
            globals(e);
            let vt = vtable(e);
            let data = geometry_data(e, vt, 1);
            vec![data, c.word(0)]
        })
        .run()
        .assert_same();
}

/// `00e8c090`: none, one and three elements of 4 and 12 bytes, packed or with a gap.
#[test]
fn copy_strided() {
    Diff::new(entry!(
        0x00e8_c090,
        pack::copy_strided(Ptr, u32, u32, u32, u32, u32)
    ))
    .family(&geometry::GEOMETRY)
    .inputs(&[
        Input::Of(&[0, 1, 3]),
        Input::Of(&[4, 0xc]),
        Input::Of(&[0xc, 0x10]),
    ])
    .setup(|e, c| {
        globals(e);
        let source = block(e, 0x40, 3);
        let destination = block(e, 0x60, 4);
        vec![0x4444, destination, source, c.word(2), c.word(1), c.word(0)]
    })
    .run()
    .assert_same();
}

/// `00e8c1e0`'s world. `c`'s words: a skin object, the parts, the chip (0 none, 1 kept, 2 another
/// format, 3 too small, 4 not vertex data), a write address given, the buffer check's answer, the
/// buffer data's streams, which arrays the data has (bits: 1 positions, 2 normals, 4 colours), the data
/// flags' bit 0, the texture set limit, the unlock's answer, the count.
fn untransformed_world(e: &mut Engine, c: &Case) -> Vec<u32> {
    globals(e);
    let vt = vtable(e);
    e.register(0x00e8_c120, |e, a| {
        // The layout: the format, the stride and five offsets.
        for (k, value) in [0x152u32, 0x24, 0, 0x44, 0xc, 0x18, 0x1c]
            .iter()
            .enumerate()
        {
            set(e, a[3 + k], *value);
        }
        ret(0)
    });
    e.register(0x00e8_f010, |e, a| {
        set(e, MARK + 4, e.mem.u32(MARK + 4).rotate_left(3) ^ a[1]);
        ret(0)
    });
    e.register(0x00e8_bfa0, |e, _| ret(e.mem.u32(RETS + 0x600)));
    e.register(0x00e8_bff0, |e, _| ret(e.mem.u32(RETS + 0x608)));
    e.register(0x0068_7d40, |e, a| {
        let mut fold = e.mem.u32(MARK + 8);
        for w in a {
            fold = fold.rotate_left(7) ^ w;
        }
        set(e, MARK + 8, fold);
        ret(0)
    });
    e.register(0x00fd_f100, |e, a| {
        set(e, MARK + 0xc, a[0]);
        ret(0)
    });
    e.register(VT_FN + 0x34, |e, a| {
        for w in 0..6 {
            set(e, a[1] + 4 * w, e.mem.u32(RETS + 0x640 + 4 * w));
        }
        ret(0)
    });
    let arrays = c.word(6);
    let data = geometry_data(e, vt, c.word(7));
    for (bit, field) in [(1u32, 0x20u32), (2, 0x24), (4, 0x28)] {
        if arrays & bit == 0 {
            set(e, data + field, 0);
        }
    }
    let buffers = e.mem.alloc(0x54);
    set(e, buffers + 0x20, c.word(5));
    let strides = e.mem.alloc(8);
    set(e, buffers + 0x24, strides);
    let chips = e.mem.alloc(8);
    set(e, buffers + 0x28, chips);
    set(e, data + 0x34, buffers);
    let lock_block = e.mem.alloc(0x400);
    for b in 0..0x400 {
        e.mem.set_u8(lock_block + b, 0xcd);
    }
    let first_chip = e.mem.alloc(0x18);
    let first_buffer = obj(e, 0x10, vt);
    set(e, first_chip + 8, first_buffer);
    set(e, chips, first_chip);
    let chip = match c.word(2) {
        0 => 0,
        state => {
            let ch = e.mem.alloc(0x18);
            let buffer = obj(e, 0x10, vt);
            set(e, ch + 8, buffer);
            for (w, value) in [(0xc, 0x80), (0x10, 0x1000), (0x14, 0x200)] {
                set(e, ch + w, value);
            }
            set(e, RETS + 0x640, if state == 4 { 101 } else { 100 });
            set(e, RETS + 0x650, if state == 3 { 0x10 } else { 0x1000 });
            set(e, RETS + 0x654, if state == 2 { 0x153 } else { 0x152 });
            ch
        }
    };
    let manager = e.mem.alloc(0x98);
    let staging = block(e, 0x40, 5);
    let locked = e.mem.alloc(0x40);
    set(e, manager + 0x40, staging);
    set(e, manager + 0x48, locked);
    set(e, manager + 0x4c, 0x20);
    set(e, TEXTURE_SET_LIMIT, c.word(8));
    set(e, RETS + 0x90, c.word(10));
    set(e, RETS + 0x30, c.word(9));
    set(e, RETS + 0x600, c.word(4));
    set(e, RETS + 0x608, lock_block);
    let skin = if c.word(0) != 0 { obj(e, 0x10, vt) } else { 0 };
    let write = if c.word(3) != 0 { lock_block + 0x80 } else { 0 };
    vec![manager, data, skin, c.word(1), chip, write]
}

/// `00e8c1e0`: with and without a skin object, each part and all, each chip state, a write address or
/// none, the buffer check's answer, buffer data with and without streams, the arrays the data has, the
/// texture set bit and limit, the unlock failing or not, no vertices or three. A new buffer whose
/// buffer data has no streams leaves no chip, and the exe then reads the lock's words through a null
/// chip: those cases stop on both sides.
#[test]
fn pack_untransformed() {
    Diff::new(entry!(
        0x00e8_c1e0,
        pack::pack_untransformed(Ptr, Ptr, u32, u32, u32, u32) -> u32
    ))
    .family(&geometry::GEOMETRY)
    .translation_calls_plainly(&[0x00e8_c090])
    .max_calls(MAX_CALLS)
    .expect_stops(|c| {
        let vertices = c.word(10) != 0;
        let written = c.word(3) != 0;
        let kept = c.word(2) == 1;
        let made = c.word(4) != 0;
        vertices && !written && !kept && made && c.word(5) == 0
    })
    .inputs(&[
        Input::Flag,
        Input::Of(&[0, 1, 2, 4, 8, 0xf]),
        Input::Of(&[0, 1, 2, 3, 4]),
        Input::Flag,
        Input::Of(&[1, 0]),
        Input::Of(&[1, 0]),
        Input::Of(&[7, 0, 1, 2, 4]),
        Input::Of(&[1, 0]),
        Input::Of(&[1, 0, 2]),
        Input::Of(&[0, 0x8000_0000]),
        Input::Of(&[3, 0]),
    ])
    .max_cases(700)
    .setup(untransformed_world)
    .run()
    .assert_same();
}

// Batch 2: the bound, the geometry data's setters, the morpher.

use engine_typed::geometry::{bound, data, morph};

/// The bound's, the setters' and the morpher's constants: the empty bound's centre (`011f426c`), the
/// double 0.5 (`01011588`), the weight thresholds (`01017d00`, `0104ffb8`), the forced update's time
/// (`01096324`), the update task switch (`011f3725`) and owner (`011f5b04`); the square root
/// (`00ec6040`) stands in with the root of its argument.
const ZERO_POINT: u32 = 0x011f_426c;
const HALF: u32 = 0x0101_1588;
const WEIGHT_ABOVE: u32 = 0x0101_7d00;
const WEIGHT_BELOW: u32 = 0x0104_ffb8;
const FORCED_UPDATE_TIME: u32 = 0x0109_6324;
const UPDATE_TASKS_ENABLED: u32 = 0x011f_3725;
const UPDATE_TASK_OWNER: u32 = 0x011f_5b04;

fn bound_globals(e: &mut Engine) {
    globals(e);
    for page in [
        0x0104_f000u32,
        0x0109_6000,
        0x011f_3000,
        0x011f_4000,
        0x011f_5000,
    ] {
        e.map(page, 0x1000);
    }
    for (k, value) in [1.5f32, -2.0, 0.25].iter().enumerate() {
        set(e, ZERO_POINT + 4 * k as u32, value.to_bits());
    }
    e.mem.set_u64(HALF, 0.5f64.to_bits());
    set(e, WEIGHT_ABOVE, 0.001f32.to_bits());
    set(e, WEIGHT_BELOW, (-0.001f32).to_bits());
    set(e, FORCED_UPDATE_TIME, 3.25f32.to_bits());
    e.register(0x00ec_6040, |_, a| {
        let x = f64::from_bits(u64::from(a[0]) | (u64::from(a[1]) << 32));
        Ret {
            st0: x.sqrt(),
            ..Ret::default()
        }
    });
}

/// `count` points of floats from [`FLOATS`] spread out, the case's float at coordinate `at` (when
/// there is one).
fn points(e: &mut Engine, count: u32, at: u32, value: f32) -> u32 {
    let words = 3 * count.max(1);
    let p = e.mem.alloc(4 * words + 4);
    for w in 0..words {
        let spread = FLOATS[(w as usize) % FLOATS.len()] * (w as f32 - 7.0);
        set(e, p + 4 * w, spread.to_bits());
    }
    if at < words {
        set(e, p + 4 * at, value.to_bits());
    }
    p
}

/// `00a7ee30`: no points (zero and a negative count), one, a few, the unrolled four and more; the
/// case's float at a coordinate of the first point or a later one, or none.
#[test]
fn compute_from_data() {
    Diff::new(entry!(0x00a7_ee30, bound::compute_from_data(Ptr, i32, Ptr)))
        .family(&geometry::GEOMETRY)
        .inputs(&[
            Input::Of(&[0xffff_ffff, 0, 1, 3, 4, 5, 9]),
            Input::Of(&[0, 2, 4, 13, 26, 0x100]),
            Input::F32,
        ])
        .max_cases(512)
        .float_sweep(64)
        .setup(|e, c| {
            bound_globals(e);
            let count = c.word(0);
            let n = if (count as i32) > 0 { count } else { 1 };
            let p = points(e, n, c.word(1), c.f32(2));
            let bound = e.mem.alloc(0x10);
            vec![bound, count, p]
        })
        .run()
        .assert_same();
}

/// `00a7ec90`: none, the unrolled groups and their tail; the scale and a planted float in the source
/// or the destination, every float the harness has and a sweep.
#[test]
fn add_scaled_points() {
    Diff::new(entry!(
        0x00a7_ec90,
        morph::add_scaled_points(Ptr, f32, Ptr, u32)
    ))
    .family(&geometry::GEOMETRY)
    .inputs(&[
        Input::Of(&[0, 1, 3, 4, 5, 8, 9]),
        Input::F32,
        Input::Of(&[0, 4, 13, 0x100, 0x104, 0x11a]),
        Input::F32,
    ])
    .max_cases(512)
    .float_sweep(64)
    .setup(|e, c| {
        globals(e);
        let count = c.word(0);
        let at = c.word(2);
        let source = points(e, count, if at < 0x100 { at } else { u32::MAX }, c.f32(3));
        let dest = points(
            e,
            count,
            if at >= 0x100 { at - 0x100 } else { u32::MAX },
            c.f32(3),
        );
        vec![dest, c.f32(1).to_bits(), source, count]
    })
    .run()
    .assert_same();
}

/// What the morpher's world is made with.
struct MorphCase {
    targets: u32,
    relative: bool,
    ignore_base: bool,
    items: bool,
    answer: u32,
    weight: f32,
    real_slot: bool,
    target_check: u32,
    missing_interpolator: bool,
    /// 0: the weights as below; 1 or 2: every item's weight and the interpolators' written weight exactly
    /// 0.001 or -0.001 (the thresholds, where `>=` and `<=` decide).
    boundary: u32,
}

/// The morpher's world: a controller (slot 0xc4 answers the item's interpolator from the table, 0xe8
/// the real `GetVertexData` or a stand-in answering an array) on a target geometry (slot 0x18 from
/// the table) whose data has six vertices, morph data of `targets` targets, relative or not, the items
/// with their weights or none, interpolators whose slot 0x9c writes `weight` and answers `answer`.
fn morph_world(e: &mut Engine, m: &MorphCase) -> u32 {
    bound_globals(e);
    let vt = vtable(e);
    slots!(e; 0x18, 0xe8, 0x94);
    e.register(VT_FN + 0xc4, |e, a| ret(e.mem.u32(RETS + 0x700 + 4 * a[1])));
    e.register(VT_FN + 0x9c, |e, a| {
        set(e, a[3], e.mem.u32(RETS + 0x9c0));
        ret(e.mem.u32(RETS + 0x9c))
    });
    let controller_vt = e.mem.alloc(0x200);
    for slot in (0..0x200).step_by(4) {
        let real = slot == 0xe8 && m.real_slot;
        set(
            e,
            controller_vt + slot,
            if real { 0x00a3_b010 } else { VT_FN + slot },
        );
    }
    let data = geometry_data(e, vt, 1);
    set(e, data + 0x30, 0);
    let target = obj(e, 0xc0, vt);
    set(e, target + 0xb8, data);
    let controller = obj(e, 0x44, controller_vt);
    set(e, controller + 0x2c, target);
    set(e, controller + 0x28, 2.5f32.to_bits());
    e.mem.set_u8(controller + 0x41, u8::from(m.ignore_base));
    let count = m.targets;
    let morph = e.mem.alloc(0x18);
    set(e, morph + 8, count);
    set(e, morph + 0xc, 6);
    e.mem.set_u8(morph + 0x14, u8::from(m.relative));
    let targets = e.mem.alloc(0x10 * count.max(1));
    for i in 0..count {
        let vertices = points(e, 6, u32::MAX, 0.0);
        set(e, targets + 0x10 * i, vertices);
    }
    set(e, morph + 0x10, targets);
    set(e, controller + 0x38, morph);
    if m.items {
        let items = e.mem.alloc(8 * count.max(1));
        for i in 0..count {
            let weight = match m.boundary {
                1 => 0.001f32,
                2 => -0.001,
                _ => [0.5f32, 0.0005, -0.75][i as usize % 3],
            };
            set(e, items + 8 * i + 4, weight.to_bits());
        }
        set(e, controller + 0x3c, items);
    }
    for i in 0..count {
        let interpolator = if i == 1 && m.missing_interpolator {
            0
        } else {
            obj(e, 0x10, vt)
        };
        set(e, RETS + 0x700 + 4 * i, interpolator);
    }
    set(e, RETS + 0x9c, m.answer);
    let written = match m.boundary {
        1 => 0.001f32,
        2 => -0.001,
        _ => m.weight,
    };
    set(e, RETS + 0x9c0, written.to_bits());
    set(e, RETS + 0x18, m.target_check);
    let other = points(e, 6, u32::MAX, 0.0);
    set(e, RETS + 0xe8, other);
    controller
}

/// `00a3aa10`: no targets, one or three; relative data or not; the base ignored or not; items or
/// none; the interpolators answering or not with every float the harness has; the slot 0xe8 real or a
/// stand-in; the target check either way; an item without an interpolator; the weights at exactly the
/// thresholds 0.001 and -0.001.
#[test]
fn gen_morph_interp() {
    Diff::new(entry!(0x00a3_aa10, morph::gen_morph_interp(Ptr, u32)))
        .family(&geometry::GEOMETRY)
        .max_calls(MAX_CALLS)
        .inputs(&[
            Input::Of(&[0, 1, 3]),
            Input::Flag,
            Input::Flag,
            Input::Of(&[1, 0]),
            Input::Of(&[1, 0]),
            Input::F32,
            Input::Flag,
            Input::Flag,
            Input::Flag,
            Input::Of(&[0, 1, 2]),
        ])
        .max_cases(600)
        .setup(|e, c| {
            let m = MorphCase {
                targets: c.word(0),
                relative: c.word(1) != 0,
                ignore_base: c.word(2) != 0,
                items: c.word(3) != 0,
                answer: c.word(4),
                weight: c.f32(5),
                real_slot: c.word(6) != 0,
                target_check: c.word(7),
                missing_interpolator: c.word(8) != 0,
                boundary: c.word(9),
            };
            let controller = morph_world(e, &m);
            vec![controller, 0x7777]
        })
        .run()
        .assert_same();
}

/// `00a3b010`: with and without a target, the data with positions or none (then a new array from the
/// memory manager, a stand-in making a block), the data flags kept.
#[test]
fn vertex_data() {
    Diff::new(entry!(0x00a3_b010, morph::vertex_data(Ptr) -> u32))
        .family(&geometry::GEOMETRY)
        .inputs(&[
            Input::Of(&[1, 0]),
            Input::Of(&[1, 0]),
            Input::Of(&[0, 1, 0xf001, 0xfffe]),
        ])
        .setup(|e, c| {
            bound_globals(e);
            e.register(0x00aa_3e40, |e, a| ret(e.mem.alloc(a[1].max(4))));
            let vt = vtable(e);
            let data = geometry_data(e, vt, c.word(2));
            if c.word(1) == 0 {
                set(e, data + 0x20, 0);
            }
            let target = if c.word(0) != 0 {
                let t = obj(e, 0xc0, vt);
                set(e, t + 0xb8, data);
                t
            } else {
                0
            };
            let controller = obj(e, 0x44, vt);
            set(e, controller + 0x2c, target);
            vec![controller]
        })
        .run()
        .assert_same();
}

/// `00a3b080`: morph data or none; the update data's byte +5; a forced update; `DontDoUpdate`'s
/// answer; always updating; a target or none; the update tasks off, refused, for a shared data word,
/// made or not, queued or run at once. The translation calls `GenMorphInterp` plainly.
#[test]
fn update() {
    Diff::new(entry!(0x00a3_b080, morph::update(Ptr, Ptr)))
        .family(&geometry::GEOMETRY)
        .translation_calls_plainly(&[0x00a3_aa10])
        .max_calls(MAX_CALLS)
        .inputs(&[
            Input::Of(&[1, 0]),
            Input::Flag,
            Input::Flag,
            Input::Flag,
            Input::Flag,
            Input::Of(&[1, 0]),
            Input::Flag,
            Input::Flag,
            Input::Of(&[1, 2]),
            Input::Flag,
            Input::Flag,
        ])
        .max_cases(500)
        .setup(|e, c| {
            // Two targets, items, the interpolators answering 0.5.
            let m = MorphCase {
                targets: 2,
                relative: false,
                ignore_base: false,
                items: true,
                answer: 1,
                weight: 0.5,
                real_slot: true,
                target_check: 1,
                missing_interpolator: false,
                boundary: 0,
            };
            let controller = morph_world(e, &m);
            let target = e.mem.u32(controller + 0x2c);
            let data = e.mem.u32(target + 0xb8);
            set(e, data + 4, c.word(8));
            if c.word(0) == 0 {
                set(e, controller + 0x38, 0);
            }
            if c.word(5) == 0 {
                set(e, controller + 0x2c, 0);
            }
            e.mem.set_u8(controller + 8, (c.word(2) as u8) << 5 | 0x11);
            e.mem.set_u8(controller + 0x42, c.word(4) as u8);
            let update = e.mem.alloc(8);
            set(e, update, 7.5f32.to_bits());
            e.mem.set_u8(update + 5, c.word(1) as u8);
            e.register(0x00a3_6250, |e, _| ret(e.mem.u32(RETS + 0x250)));
            set(e, RETS + 0x250, c.word(3));
            e.mem.set_u8(UPDATE_TASKS_ENABLED, c.word(6) as u8);
            e.register(0x0071_4a00, |e, _| ret(e.mem.u32(RETS + 0x4a0)));
            set(e, RETS + 0x4a0, c.word(7));
            let vt = e.mem.u32(target);
            let task = if c.word(9) != 0 { obj(e, 0x20, vt) } else { 0 };
            e.register(0x00a5_6f90, |e, _| ret(e.mem.u32(RETS + 0x6f0)));
            set(e, RETS + 0x6f0, task);
            e.register(0x00a5_6c60, |e, a| {
                set(
                    e,
                    MARK + 0x10,
                    a[0] ^ a[1].rotate_left(8) ^ a[2].rotate_left(16),
                );
                ret(0)
            });
            e.register(0x00a3_a730, |e, _| ret(e.mem.u32(RETS + 0x730)));
            set(e, RETS + 0x730, c.word(10));
            set(e, UPDATE_TASK_OWNER, 0x5150);
            vec![controller, update]
        })
        .run()
        .assert_same();
}

/// The setters' world: a geometry data (slot 0x80 the group from the table, slot 0x90 the active
/// count) with its arrays, a group with a count; the memory manager's free, the group's destructor
/// and the sized delete stand in, each leaving its mark.
fn setter_world(e: &mut Engine, group: bool, group_count: u32, active: u32) -> u32 {
    bound_globals(e);
    slots!(e; 0x80);
    for addr in [0x00aa_4060u32, 0x00a8_a670, 0x00aa_1460] {
        e.register(addr, |e, a| {
            let mut fold = e.mem.u32(MARK + 0x14);
            for w in a {
                fold = fold.rotate_left(9) ^ w;
            }
            set(e, MARK + 0x14, fold);
            ret(0)
        });
    }
    let vt = vtable(e);
    let data = geometry_data(e, vt, 0x0f0f);
    let g = e.mem.alloc(0x10);
    set(e, g + 0xc, group_count);
    set(e, RETS + 0x80, if group { g } else { 0 });
    set(e, RETS + 0x90, active);
    data
}

/// New arrays for the setters: each of the four is the data's own when its bit in `changed` is clear,
/// else a new block; the positions null when `no_positions`.
fn new_arrays(e: &mut Engine, data: u32, changed: u32, no_positions: bool) -> [u32; 4] {
    let mut arrays = [0u32; 4];
    for (k, at) in [0x20u32, 0x24, 0x28, 0x2c].iter().enumerate() {
        arrays[k] = if changed & (1 << k) != 0 {
            points(e, 6, u32::MAX, 0.0)
        } else {
            e.mem.u32(data + at)
        };
    }
    if no_positions {
        arrays[0] = 0;
    }
    arrays
}

/// `00a670c0`: with a group (its count reaching zero or not) or without; none, one or every array
/// changed; the new positions null; the active count zero or six; the texture set count and flags; the
/// count's high half.
#[test]
fn replace() {
    Diff::new(entry!(
        0x00a6_70c0,
        data::replace(Ptr, u32, u32, u32, u32, u32, u32, u32)
    ))
    .family(&geometry::GEOMETRY)
    .inputs(&[
        Input::Flag,
        Input::Of(&[1, 3]),
        Input::Of(&[0, 1, 2, 4, 8, 0xf]),
        Input::Flag,
        Input::Of(&[0, 6]),
        Input::Of(&[0, 1, 3]),
        Input::Of(&[0, 0xf000, 0x1_0001]),
        Input::Of(&[6, 0x1_0006]),
    ])
    .max_cases(600)
    .setup(|e, c| {
        let data = setter_world(e, c.word(0) != 0, c.word(1), c.word(4));
        let a = new_arrays(e, data, c.word(2), c.word(3) != 0);
        vec![
            data,
            c.word(7),
            a[0],
            a[1],
            a[2],
            a[3],
            c.word(5),
            c.word(6),
        ]
    })
    .run()
    .assert_same();
}

/// `00a66b30`: the active count zero or six, the positions null or not, the texture set count, the
/// flags word and the count's high half.
#[test]
fn set_data() {
    Diff::new(entry!(
        0x00a6_6b30,
        data::set_data(Ptr, u32, Ptr, Ptr, Ptr, Ptr, u32, u32)
    ))
    .family(&geometry::GEOMETRY)
    .inputs(&[
        Input::Of(&[0, 6]),
        Input::Flag,
        Input::Of(&[0, 1, 3]),
        Input::Of(&[0, 0xf000, 0x1_0001]),
        Input::Of(&[6, 0x1_0006]),
    ])
    .setup(|e, c| {
        let data = setter_world(e, false, 1, c.word(0));
        let a = new_arrays(e, data, 0xf, c.word(1) != 0);
        vec![
            data,
            c.word(4),
            a[0],
            a[1],
            a[2],
            a[3],
            c.word(2),
            c.word(3),
        ]
    })
    .run()
    .assert_same();
}

/// `00a66bb0`: the arrays, the texture set count, the flags and the count's high half.
#[test]
fn replace_data() {
    Diff::new(entry!(
        0x00a6_6bb0,
        data::replace_data(Ptr, u32, u32, u32, u32, u32, u32, u32)
    ))
    .family(&geometry::GEOMETRY)
    .inputs(&[
        Input::Of(&[0, 1, 3]),
        Input::Of(&[0, 0xf000, 0x1_0001]),
        Input::Of(&[6, 0x1_0006]),
        Input::Of(&[0, 0xf]),
    ])
    .setup(|e, c| {
        let data = setter_world(e, false, 1, 6);
        let a = new_arrays(e, data, c.word(3), false);
        vec![
            data,
            c.word(2),
            a[0],
            a[1],
            a[2],
            a[3],
            c.word(0),
            c.word(1),
        ]
    })
    .run()
    .assert_same();
}

/// NaN bit patterns for the operand order of `00a7ec90`'s sums and products: quiet ones with no and
/// a small payload, the x87 default, signalling ones with a small and a large payload, and two numbers.
const NAN_ORDER: &[u32] = &[
    0x7fc0_0000,
    0x7fc0_0001,
    0xffc0_0000,
    0x7f80_0001,
    0x7fbf_ffff,
    0x3f80_0000,
    0x8000_0000,
];

/// `00a7ec90` with every word of the source and of the destination one NaN pattern and the scale
/// another: which NaN comes out of each product and sum (quiet over signalling, then the larger
/// payload) is where the unrolled and the single-point code's operand orders differ.
#[test]
fn add_scaled_points_nans() {
    Diff::new(entry!(
        0x00a7_ec90,
        morph::add_scaled_points(Ptr, f32, Ptr, u32)
    ))
    .family(&geometry::GEOMETRY)
    .inputs(&[
        Input::Of(&[1, 5]),
        Input::Of(NAN_ORDER),
        Input::Of(NAN_ORDER),
        Input::Of(NAN_ORDER),
    ])
    .max_cases(700)
    .setup(|e, c| {
        globals(e);
        let count = c.word(0);
        let fill = |e: &mut Engine, bits: u32| {
            let p = e.mem.alloc(12 * count + 4);
            for w in 0..3 * count {
                set(e, p + 4 * w, bits);
            }
            p
        };
        let source = fill(e, c.word(2));
        let dest = fill(e, c.word(3));
        vec![dest, c.word(1), source, count]
    })
    .run()
    .assert_same();
}

/// `00e7a200`'s binormals and tangents (usages 13 and 14): normals present or none, the data flags
/// without the method bits, with one and with all, the float types that copy them as they are and wider.
#[test]
fn pack_entry_derived() {
    Diff::new(entry!(
        0x00e7_a200,
        pack::pack_entry(Ptr, u32, u32, u32, u32, u32, u32, u32, u32, u32) -> u32
    ))
    .family(&geometry::GEOMETRY)
    .max_calls(MAX_CALLS)
    .inputs(&[
        Input::Of(&[13, 14]),
        Input::Flag,
        Input::Of(&[0, 0x1000, 0xf000]),
        Input::Of(&[2, 3]),
    ])
    .setup(|e, c| {
        globals(e);
        converter_tables(e);
        additional_lookup(e);
        let vt = vtable(e);
        let data = geometry_data(e, vt, c.word(2));
        if c.word(1) == 0 {
            set(e, data + 0x24, 0);
        }
        let declaration = e.mem.alloc(0x1c);
        set(e, declaration + 8, c.word(0));
        set(e, declaration + 0xc, c.word(3));
        let destination = e.mem.alloc(0x200);
        for b in 0..0x200 {
            e.mem.set_u8(destination + b, 0xcd);
        }
        let this = e.mem.alloc(0x10);
        vec![this, 6, data, 0, 0, declaration, 0x20, destination, 0, 0]
    })
    .run()
    .assert_same();
}

/// `00e7a200`'s additional data streams: not found, found as a float format, found as a format of the
/// stand-in converter, and found with a null source; the float types and the colour type.
#[test]
fn pack_entry_additional() {
    Diff::new(entry!(
        0x00e7_a200,
        pack::pack_entry(Ptr, u32, u32, u32, u32, u32, u32, u32, u32, u32) -> u32
    ))
    .family(&geometry::GEOMETRY)
    .max_calls(MAX_CALLS)
    .inputs(&[
        Input::Of(&[0x8000_0002, 0x8000_0000]),
        Input::Of(&[0, 3, 0x19, 0x103]),
        Input::Of(&[2, 3, 4]),
    ])
    .setup(|e, c| {
        globals(e);
        converter_tables(e);
        additional_lookup(e);
        let vt = vtable(e);
        let data = geometry_data(e, vt, 1);
        let mode = c.word(1);
        set(e, RETS + 0x720, u32::from(mode != 0));
        set(e, RETS + 0x724, mode & 0xff);
        set(e, RETS + 0x728, u32::from(mode > 0xff));
        let declaration = e.mem.alloc(0x1c);
        set(e, declaration + 8, c.word(0));
        set(e, declaration + 0xc, c.word(2));
        let destination = e.mem.alloc(0x200);
        for b in 0..0x200 {
            e.mem.set_u8(destination + b, 0xcd);
        }
        let this = e.mem.alloc(0x10);
        vec![
            this,
            3,
            data,
            0,
            0,
            declaration,
            0x20,
            destination,
            0x55,
            0x66,
        ]
    })
    .run()
    .assert_same();
}

// Batch 6: where the arrays begin and end.

use engine_typed::geometry::lifecycle;

/// The binary stream's read and write stand-ins, and where the read takes its answers from: one
/// queue each for the 1-, 2- and 4-byte reads (presence bytes and flags; counts and flag words; ids
/// and old presence words), each with its cursor, and a pattern for any longer read (an array).
const STREAM_READ: u32 = 0x00ff_4000;
const STREAM_WRITE: u32 = 0x00ff_4010;
const QUEUE_1: u32 = RETS + 0x100;
const QUEUE_2: u32 = RETS + 0x140;
const QUEUE_4: u32 = RETS + 0x180;
const CURSORS: u32 = RETS + 0x1c0;
const PATTERN: u32 = RETS + 0x1cc;
/// The sink the write stand-in appends to (its cursor at +0).
const SINK: u32 = 0x0310_0000;

/// The memory manager's allocate (a stand-in making a block) and free, and the callees the load and
/// save make that stand in, each leaving its mark.
fn lifecycle_globals(e: &mut Engine) {
    bound_globals(e);
    // The point length's square root (`004579e0`: a float in, its root in ST0), which the vector normalize
    // (`004a0c10`) reaches; the C runtime's own would reach the open `GetLastError` import for a negative
    // or NaN square.
    e.register(0x0045_79e0, |_, a| Ret {
        st0: f64::from(f32::from_bits(a[0]).sqrt()),
        ..Ret::default()
    });
    e.map(SINK, 0x10000);
    set(e, SINK, SINK + 0x10);
    e.register(0x00aa_3e40, |e, a| {
        ret(e.mem.alloc(a[1].clamp(4, 0x40_0000)))
    });
    for addr in [
        0x00aa_4060u32,
        0x00a5_d610,
        0x00a6_9ac0,
        0x00a6_45b0,
        0x00a2_9680,
        0x00a8_9af0,
        0x00eb_5410,
        0x00a5_d3d0,
        0x00a8_a670,
        0x00aa_1460,
    ] {
        e.register(addr, |e, a| {
            let mut fold = e.mem.u32(MARK + 0x18);
            for w in a {
                fold = fold.rotate_left(11) ^ w;
            }
            set(e, MARK + 0x18, fold);
            ret(0)
        });
    }
    e.register(0x00a6_34e0, |e, _| ret(e.mem.u32(RETS + 0x1d0)));
    e.register(0x00a6_34f0, |_, a| ret(a[1] ^ 0x5a5a));
    e.register(STREAM_READ, |e, a| {
        let (buffer, bytes) = (a[1], a[2]);
        let queue = match bytes {
            1 => Some((QUEUE_1, CURSORS)),
            2 => Some((QUEUE_2, CURSORS + 4)),
            4 => Some((QUEUE_4, CURSORS + 8)),
            _ => None,
        };
        match queue {
            Some((at, cursor)) => {
                let k = e.mem.u32(cursor);
                set(e, cursor, k + 1);
                let value = e.mem.u32(at + 4 * (k % 16));
                for b in 0..bytes {
                    e.mem.set_u8(buffer + b, (value >> (8 * b)) as u8);
                }
            }
            None => {
                let mut seed = e.mem.u32(PATTERN);
                for w in 0..bytes / 4 {
                    seed = seed.wrapping_mul(0x0101_0101).wrapping_add(0x3c6e_f35f);
                    set(
                        e,
                        buffer + 4 * w,
                        FLOATS[(seed >> 28) as usize % 12].to_bits() ^ (seed & 0xff),
                    );
                }
                set(e, PATTERN, seed);
            }
        }
        ret(bytes)
    });
    e.register(STREAM_WRITE, |e, a| {
        let (buffer, bytes) = (a[1], a[2]);
        let at = e.mem.u32(SINK);
        for b in 0..bytes.min(0x4000) {
            let byte = e.mem.u8(buffer + b);
            e.mem.set_u8(at + b, byte);
        }
        set(e, SINK, at + bytes.min(0x4000));
        ret(bytes)
    });
}

/// A stream object (vtable `vt`, its slot 0x30 a stand-in) with its file and header versions, a
/// name, and its two binary streams (read and write stand-ins).
fn stream(e: &mut Engine, vt: u32, file: u32, header: u32) -> u32 {
    let s = obj(e, 0x254, vt);
    set(e, s + 4, header);
    set(e, s + 0xd8, file);
    for (k, b) in b"head.nif\0".iter().enumerate() {
        e.mem.set_u8(s + 0xe0 + k as u32, *b);
    }
    let input = e.mem.alloc(0x10);
    set(e, input + 8, STREAM_READ);
    let output = e.mem.alloc(0x10);
    set(e, output + 0xc, STREAM_WRITE);
    set(e, s + 0x24c, input);
    set(e, s + 0x250, output);
    s
}

/// The data's vtable: the real `GetGroup` (`00a669c0`, the static group) in its slot 0x80.
fn data_vtable(e: &mut Engine) -> u32 {
    let vt = vtable(e);
    let own = e.mem.alloc(0x200);
    for slot in (0..0x200).step_by(4) {
        let word = if slot == 0x80 {
            0x00a6_69c0
        } else {
            e.mem.u32(vt + slot)
        };
        set(e, own + slot, word);
    }
    own
}

/// File versions around each test the load makes: before 4.1.0.0, before 5.0.0.10, before 10.0.0.2,
/// before 10.0.1.16, before 10.1.0.114, before 10.3.0.7, and the game's own (20.2.0.7).
const FILE_VERSIONS: &[u32] = &[
    0x0400_0002,
    0x0401_0000,
    0x0500_000a,
    0x0a00_0002,
    0x0a00_0110,
    0x0a01_0072,
    0x0a03_0007,
    0x1402_0007,
];

/// `00a67ea0`: every file version branch; header versions below 0x13, below 0x1b and the game's;
/// each array present or not (the presence queue), the save byte set or not, a group from the id or
/// none, data flags with the method bits and two to four texture sets, the count zero or six.
#[test]
fn load_binary() {
    Diff::new(entry!(0x00a6_7ea0, lifecycle::load_binary(Ptr, Ptr)))
        .family(&geometry::GEOMETRY)
        .max_calls(MAX_CALLS)
        .inputs(&[
            Input::Of(FILE_VERSIONS),
            Input::Of(&[0x12, 0x14, 0x22]),
            Input::Of(&[0x1f, 0, 0x15, 0x0a, 0x1d]),
            Input::Flag,
            Input::Of(&[0, 7]),
            Input::Of(&[0x0001, 0xf001, 0x0003, 0x0000, 0xf004]),
            Input::Of(&[6, 0]),
        ])
        .max_cases(700)
        .setup(|e, c| {
            lifecycle_globals(e);
            let vt = vtable(e);
            let s = stream(e, vt, c.word(0), c.word(1));
            // The 1-byte answers: keep, compress, then the presences by the mask's bits.
            let mask = c.word(2);
            let ones = [
                0x13,
                0x07,
                mask & 1,
                mask >> 1 & 1,
                mask >> 2 & 1,
                mask >> 3 & 1,
                mask >> 4 & 1,
            ];
            for (k, v) in ones.iter().enumerate() {
                set(e, QUEUE_1 + 4 * k as u32, *v);
            }
            // The 2-byte answers: the count, the data flags, the old flags, the dirty flags.
            let twos = [c.word(6), c.word(5), c.word(5) | 2, 0x4123];
            for (k, v) in twos.iter().enumerate() {
                set(e, QUEUE_2 + 4 * k as u32, *v);
            }
            // The 4-byte answers: the group id, then old presence words by the mask, the texture
            // word.
            let fours = [c.word(4), mask & 1, mask >> 1 & 1, mask >> 2 & 1, 0x7777];
            for (k, v) in fours.iter().enumerate() {
                set(e, QUEUE_4 + 4 * k as u32, *v);
            }
            let group = if c.word(4) != 0 {
                let g = e.mem.alloc(0x10);
                let pool = e.mem.alloc(0x2000);
                set(e, g + 8, pool);
                set(e, g + 0xc, 1);
                g
            } else {
                0
            };
            set(e, RETS + 0x1d0, group);
            let own_vt = data_vtable(e);
            let data = obj(e, 0x40, own_vt);
            e.mem.set_u8(data + 0x3c, c.word(3) as u8);
            set(e, 0x011f_4880, 0);
            vec![data, s]
        })
        .run()
        .assert_same();
}

/// `00a68520`: each array present or not, the save byte, texture coordinates with and without the
/// flags' bit 0, the count zero or six, the method bits, the dirty flags. The translation calls the
/// texture coordinates' write plainly.
#[test]
fn save_binary() {
    Diff::new(entry!(0x00a6_8520, lifecycle::save_binary(Ptr, Ptr)))
        .family(&geometry::GEOMETRY)
        .translation_calls_plainly(&[0x00a6_7ae0])
        .max_calls(MAX_CALLS)
        .inputs(&[
            Input::Of(&[0xf, 0, 1, 2, 4, 8, 7]),
            Input::Flag,
            Input::Of(&[0x0001, 0xf001, 0x0000, 0xf000]),
            Input::Of(&[6, 0]),
            Input::Of(&[0x7123, 0x0000]),
        ])
        .setup(|e, c| {
            lifecycle_globals(e);
            let vt = vtable(e);
            let s = stream(e, vt, 0x1402_0007, 0x22);
            let data = geometry_data(e, vt, c.word(2));
            for (bit, field) in [(1u32, 0x20u32), (2, 0x24), (4, 0x28), (8, 0x2c)] {
                if c.word(0) & bit == 0 {
                    set(e, data + field, 0);
                }
            }
            e.mem.set_u16(data + 8, c.word(3) as u16);
            e.mem.set_u16(data + 0xe, c.word(4) as u16);
            e.mem.set_u8(data + 0x38, 0x11);
            e.mem.set_u8(data + 0x39, 0x22);
            e.mem.set_u8(data + 0x3c, c.word(1) as u8);
            set(e, 0x011f_4880, 0x0bad_9999);
            vec![data, s]
        })
        .run()
        .assert_same();
}

/// `00a67ae0`: none, one and six texture coordinates.
#[test]
fn write_texture_coordinates() {
    Diff::new(entry!(
        0x00a6_7ae0,
        lifecycle::write_texture_coordinates(Ptr, u32, u32) -> u32
    ))
    .family(&geometry::GEOMETRY)
    .inputs(&[Input::Of(&[0, 1, 6])])
    .setup(|e, c| {
        lifecycle_globals(e);
        let vt = vtable(e);
        let s = stream(e, vt, 0x1402_0007, 0x22);
        let textures = points(e, 4, u32::MAX, 0.0);
        vec![s, textures, c.word(0)]
    })
    .run()
    .assert_same();
}

/// `00a67c70`: with and without the object at `011f4748`, the additional data (its count reaching
/// zero or not) or none, the static group (its count reaching zero or not) or none, each array set or
/// not.
#[test]
fn destruct() {
    Diff::new(entry!(0x00a6_7c70, lifecycle::destruct(Ptr)))
        .family(&geometry::GEOMETRY)
        .inputs(&[
            Input::Flag,
            Input::Of(&[0, 1, 2]),
            Input::Of(&[0, 1, 3]),
            Input::Of(&[0xf, 0, 1, 6]),
        ])
        .setup(|e, c| {
            lifecycle_globals(e);
            let vt = vtable(e);
            slots!(e; 0x04, 0xf0);
            e.register(0x00fd_f1c4, |e, a| {
                let count = e.mem.u32(a[0]).wrapping_sub(1);
                set(e, a[0], count);
                ret(count)
            });
            let renderer = if c.word(0) != 0 { obj(e, 0x10, vt) } else { 0 };
            set(e, 0x011f_4748, renderer);
            let data = geometry_data(e, vt, 1);
            let additional = if c.word(1) != 0 {
                let a = obj(e, 0x10, vt);
                set(e, a + 4, c.word(1));
                a
            } else {
                0
            };
            set(e, data + 0x30, additional);
            let group = if c.word(2) != 0 {
                let g = e.mem.alloc(0x10);
                set(e, g + 0xc, c.word(2));
                g
            } else {
                0
            };
            set(e, 0x011f_4880, group);
            for (bit, field) in [(1u32, 0x20u32), (2, 0x24), (4, 0x28), (8, 0x2c)] {
                if c.word(3) & bit == 0 {
                    set(e, data + field, 0);
                }
            }
            vec![data]
        })
        .run()
        .assert_same();
}

/// `00a669f0`: normals present or not, no method or one, the flag's low byte set, clear, or set only
/// above it.
#[test]
fn create_normals() {
    Diff::new(entry!(0x00a6_69f0, lifecycle::create_normals(Ptr, u32)))
        .family(&geometry::GEOMETRY)
        .inputs(&[
            Input::Flag,
            Input::Of(&[0x0001, 0xf001, 0x1000]),
            Input::Of(&[0, 1, 0x100]),
            Input::Of(&[6, 0]),
        ])
        .setup(|e, c| {
            lifecycle_globals(e);
            let vt = vtable(e);
            let data = geometry_data(e, vt, c.word(1));
            if c.word(0) == 0 {
                set(e, data + 0x24, 0);
            }
            e.mem.set_u16(data + 8, c.word(3) as u16);
            vec![data, c.word(2)]
        })
        .run()
        .assert_same();
}

/// `00a66a60`: the method already set or changed to none, one or another; normals present or not.
#[test]
fn set_normal_method() {
    Diff::new(entry!(0x00a6_6a60, lifecycle::set_normal_method(Ptr, u32)))
        .family(&geometry::GEOMETRY)
        .inputs(&[
            Input::Of(&[0x0001, 0x1001, 0xf001]),
            Input::Of(&[0, 0x1000, 0xf000]),
            Input::Flag,
            Input::Of(&[6, 0]),
        ])
        .setup(|e, c| {
            lifecycle_globals(e);
            let vt = vtable(e);
            let data = geometry_data(e, vt, c.word(0));
            if c.word(2) == 0 {
                set(e, data + 0x24, 0);
            }
            e.mem.set_u16(data + 8, c.word(3) as u16);
            vec![data, c.word(1)]
        })
        .run()
        .assert_same();
}

/// `00a67000`: normals present or not.
#[test]
fn release_normals() {
    Diff::new(entry!(0x00a6_7000, lifecycle::release_normals(Ptr)))
        .family(&geometry::GEOMETRY)
        .inputs(&[Input::Flag])
        .setup(|e, c| {
            lifecycle_globals(e);
            let vt = vtable(e);
            let data = geometry_data(e, vt, 1);
            if c.word(0) == 0 {
                set(e, data + 0x24, 0);
            }
            vec![data]
        })
        .run()
        .assert_same();
}

// Batch 3: the tangent space.

use engine_typed::geometry::tangent;

/// `00b54b60`'s world: a geometry (slot 0x1c its triangle shape from the table, slot 0x20 whether it
/// is strips) with a name or none, its data of six vertices (slot 0xac the triangle indices from a
/// list holding a repeated vertex), a strip list, the tangent space extra data or none, and the
/// callees that stand in: the name compare, the new object and its constructor, the fixed strings, the
/// extra data's lookup and removal, the formatter. `c`'s words: the extra data, the shape, strips,
/// normals, the triangle count, the name, the case's float and where it goes.
fn tangent_world(e: &mut Engine, c: &Case) -> u32 {
    lifecycle_globals(e);
    e.map(0x0109_b000, 0x1000);
    set(e, 0x0109_b220, 0x0abc_0000);
    let vt = vtable(e);
    slots!(e; 0x00, 0x1c, 0x20);
    for addr in [0x00ec_7ec0u32, 0x00a5_be90, 0x0040_6d00, 0x00fd_f1c4] {
        e.register(addr, |e, a| {
            let mut fold = e.mem.u32(MARK + 0x1c);
            for w in a {
                fold = fold.rotate_left(13) ^ w;
            }
            set(e, MARK + 0x1c, fold);
            ret(0)
        });
    }
    e.register(0x00aa_13e0, |e, a| ret(e.mem.alloc(a[0].max(4))));
    e.register(0x00b6_6810, |e, a| {
        let vt = e.mem.u32(RETS + 0x800 + 0x20);
        set(e, a[0], vt);
        ret(a[0])
    });
    e.register(0x00a5_b690, |e, _| {
        let s = e.mem.alloc(0x20);
        ret(s + 8)
    });
    e.register(0x00a5_bdd0, |e, _| ret(e.mem.u32(RETS + 0x5bd)));
    e.register(0x00a7_5f10, |e, _| ret(e.mem.u32(RETS + 0x5f1)));
    // The data's triangle query (slot 0xac): (triangle, &i0, &i1, &i2) from a list of six triangles,
    // the third with a repeated vertex.
    e.register(VT_FN + 0xac, |e, a| {
        const TRIANGLES: [[u32; 3]; 6] = [
            [0, 1, 2],
            [2, 1, 3],
            [3, 3, 4],
            [4, 5, 0],
            [5, 1, 4],
            [1, 0, 5],
        ];
        let t = TRIANGLES[(a[1] % 6) as usize];
        for k in 0..3 {
            e.mem.set_u16(a[2 + k], t[k] as u16);
        }
        ret(0)
    });
    set(e, RETS + 0x820, vt);
    let data = geometry_data(e, vt, 0x0001);
    // The case's float at one of the positions' or texture coordinates' words.
    let at = c.word(7);
    let (positions, coordinates) = (e.mem.u32(data + 0x20), e.mem.u32(data + 0x2c));
    if at < 18 {
        set(e, positions + 4 * at, c.f32(6).to_bits());
    } else if at < 30 {
        set(e, coordinates + 4 * (at - 18), c.f32(6).to_bits());
    }
    if c.word(3) == 0 {
        set(e, data + 0x24, 0);
    }
    e.mem.set_u16(data + 0x40, c.word(4) as u16);
    let strip = e.mem.alloc(0x20);
    for (k, v) in [0u16, 1, 2, 3, 4, 5, 0, 2, 4].iter().enumerate() {
        e.mem.set_u16(strip + 2 * k as u32, *v);
    }
    set(e, RETS + 0x5f1, strip);
    let geometry = obj(e, 0xc0, vt);
    let name = if c.word(5) != 0 {
        let n = e.mem.alloc(8);
        e.mem.write(n, b"STBBrock");
        n
    } else {
        0
    };
    set(e, geometry + 8, name);
    set(e, geometry + 0xb8, data);
    set(e, RETS + 0x1c, if c.word(1) != 0 { geometry } else { 0 });
    set(e, RETS + 0x20, c.word(2));
    let existing = if c.word(0) != 0 {
        let x = e.mem.alloc(0x10);
        let halves = points(e, 12, u32::MAX, 0.0);
        set(e, x + 0xc, halves);
        x
    } else {
        0
    };
    set(e, RETS + 0x5bd, existing);
    geometry
}

/// `00b54b60`: with the extra data or without; a shape or none; strips or a list; normals or none;
/// no triangles, two, six; a name or none; the case's float at a position or texture coordinate (or
/// nowhere), every float the harness has and a sweep.
#[test]
fn create_tangent_space() {
    Diff::new(entry!(0x00b5_4b60, tangent::create_tangent_space(u32) -> u32))
        .family(&geometry::GEOMETRY)
        .max_calls(MAX_CALLS)
        .inputs(&[
            Input::Of(&[0, 1]),
            Input::Of(&[1, 0]),
            Input::Flag,
            Input::Of(&[1, 0]),
            Input::Of(&[0, 2, 6]),
            Input::Flag,
            Input::F32,
            Input::Of(&[0, 5, 19, 26, 99]),
        ])
        .max_cases(600)
        .float_sweep(48)
        .setup(|e, c| {
            let geometry = tangent_world(e, c);
            vec![geometry]
        })
        .run()
        .assert_same();
}

// Batch 3: the pick of a geometry's triangles.

use engine_core::typed::leaf;
use engine_typed::geometry::pick;

/// What the pick tests make: the ray, the pick, the geometry.
struct Picked {
    origin: u32,
    direction: u32,
    pick: u32,
    geometry: u32,
}

/// `c`'s words for [`pick_world`]: the class (0 general, 1 single strip, 2 strips of two, 3 shape),
/// the intersect type (0 the centre, 1 or 2 the triangles), a skin, the coordinate type, the ray-test
/// answers (a bit a triangle), the returns (bits: 1 texture, 2 normal, 4 smooth normal, 8 colour),
/// the locks (bits: 1 vertices through the additional data, 2 textures, 4 normals, 8 colours), the
/// arrays the data keeps (bits: 1 positions, 2 normals, 4 colours, 8 texture coordinates), stop at
/// the first hit, the case's float and where it goes.
fn pick_world(e: &mut Engine, c: &Case) -> Picked {
    lifecycle_globals(e);
    e.map(0x011a_9000, 0x1000);
    e.map(0x0126_f000, 0x1000);
    let vt = vtable(e);
    slots!(e; 0x08, 0x8c);
    // The stand-ins: the ray-triangle test answers from the table's bits and writes its point,
    // distance and weights; the data's locks fill their blocks from the table; the strip and triangle
    // lists come from the table; the class test answers from the table; the record's set marks.
    e.register(0x00ea_db90, |e, a| {
        let k = e.mem.u32(RETS + 0x900);
        set(e, RETS + 0x900, k + 1);
        let hit = (e.mem.u32(RETS + 0x904) >> (k % 8)) & 1;
        if hit != 0 {
            let value = e.mem.u32(RETS + 0x908);
            for (slot, bits) in [
                (a[6], 0x3f80_0000u32),
                (a[6] + 4, value),
                (a[6] + 8, 0x4000_0000),
                (a[7], 0x4120_0000),
                (a[8], 0x3e80_0000),
                (a[9], 0x3e00_0000),
            ] {
                set(e, slot, bits);
            }
        }
        ret(hit)
    });
    // The locks: the vertices' (data, block), the texture coordinates' (data, 0, block), the
    // normals' and the colours' (data, block) fill the block from the table.
    fn fill_lock(e: &mut Engine, block: u32, at: u32) -> Ret {
        set(e, block, e.mem.u32(RETS + at));
        set(e, block + 4, e.mem.u32(RETS + at + 4));
        ret(0)
    }
    e.register(0x00a6_7410, |e, a| fill_lock(e, a[1], 0x910));
    e.register(0x00a6_7720, |e, a| fill_lock(e, a[2], 0x920));
    e.register(0x00a6_75b0, |e, a| fill_lock(e, a[1], 0x930));
    e.register(0x00a6_7660, |e, a| fill_lock(e, a[1], 0x940));
    e.register(0x00a6_7360, |e, _| ret(e.mem.u32(RETS + 0x950)));
    e.register(0x00a6_73d0, |e, a| {
        set(e, MARK + 0x20, e.mem.u32(MARK + 0x20) ^ a[0]);
        ret(0)
    });
    e.register(0x00a7_5f10, |e, _| ret(e.mem.u32(RETS + 0x954)));
    e.register(0x00a7_b850, |e, _| ret(e.mem.u32(RETS + 0x958)));
    e.register(0x0043_b300, |e, _| ret(e.mem.u32(RETS + 0x95c)));
    e.register(0x00e9_8aa0, |e, a| {
        set(e, a[0] + 0x40, a[1]);
        ret(0)
    });
    e.register(0x0053_4780, |e, a| {
        for w in 0..4 {
            set(e, a[2] + 4 * w, (0.25f32 * (a[1] + w) as f32).to_bits());
        }
        ret(0)
    });
    e.register(VT_FN + 0xac, |e, a| {
        const TRIANGLES: [[u16; 3]; 4] = [[0, 1, 2], [2, 1, 3], [3, 4, 5], [5, 4, 0]];
        let t = TRIANGLES[(a[1] % 4) as usize];
        for k in 0..3 {
            e.mem.set_u16(a[2 + k], t[k]);
        }
        ret(0)
    });
    let class = c.word(0);
    // The data, its arrays as kept, its additional data for the vertex lock.
    let keeps = c.word(7);
    let data = geometry_data(e, vt, 0x0001);
    for (bit, field) in [(1u32, 0x20u32), (2, 0x24), (4, 0x28), (8, 0x2c)] {
        if keeps & bit == 0 {
            set(e, data + field, 0);
        }
    }
    let positions = e.mem.u32(data + 0x20);
    let at = c.word(10);
    if positions != 0 && at < 18 {
        set(e, positions + 4 * at, c.f32(9).to_bits());
    }
    let locks = c.word(6);
    let additional = obj(e, 0x10, vt);
    set(e, data + 0x30, additional);
    set(e, RETS + 0x8c, locks & 1);
    set(e, RETS + 0x950, locks & 1);
    let locked_positions = points(e, 6, u32::MAX, 0.0);
    set(e, RETS + 0x910, locked_positions);
    set(e, RETS + 0x914, 12);
    for (bit, at, stride) in [(2u32, 0x920u32, 8u32), (4, 0x930, 12), (8, 0x940, 16)] {
        if locks & bit != 0 {
            let block = points(e, 8, u32::MAX, 0.0);
            set(e, RETS + at, block);
            set(e, RETS + at + 4, stride);
        }
    }
    e.mem.set_u16(data + 0x44, if class == 2 { 2 } else { 1 });
    let strip = e.mem.alloc(0x20);
    for (k, v) in [0u16, 1, 2, 2, 3, 4, 5, 0].iter().enumerate() {
        e.mem.set_u16(strip + 2 * k as u32, *v);
    }
    set(e, RETS + 0x954, strip);
    let list = e.mem.alloc(0x20);
    for (k, v) in [0u16, 1, 2, 3, 4, 5, 2, 4, 0].iter().enumerate() {
        e.mem.set_u16(list + 2 * k as u32, *v);
    }
    set(e, RETS + 0x958, list);
    set(e, RETS + 0x95c, u32::from(class == 3));
    set(e, RETS + 0xa8, 3);
    // The class chain: the strips' record for classes 1 and 2, else a record with no parent.
    let record = if class == 1 || class == 2 {
        0x011f_4a20
    } else {
        let r = e.mem.alloc(8);
        set(e, r + 4, 0);
        r
    };
    set(e, RETS + 0x08, record);
    // The geometry: its bound centre words, rotation, translation and scale, data and skin.
    let geometry = obj(e, 0xc0, vt);
    let centre = points(e, 2, u32::MAX, 0.0);
    set(e, geometry + 0x20, centre);
    let rotation = [
        1.0f32, 0.0, 0.0, 0.0, 0.8, -0.6, 0.0, 0.6, 0.8, 5.0, -3.0, 2.0, 1.5,
    ];
    for (k, v) in rotation.iter().enumerate() {
        set(e, geometry + 0x68 + 4 * k as u32, v.to_bits());
    }
    set(e, geometry + 0xb8, data);
    set(e, geometry + 0xbc, c.word(2));
    // The pick: types, the returns, the results' pool with eight records ready.
    let p = e.mem.alloc(0x40);
    set(e, p, 1);
    set(e, p + 4, c.word(8));
    set(e, p + 8, c.word(1));
    set(e, p + 0xc, c.word(3));
    e.mem.set_u8(p + 0x10, 1);
    let returns = c.word(5);
    for (k, bit) in [1u32, 2, 4, 8].iter().enumerate() {
        e.mem
            .set_u8(p + 0x30 + k as u32, u8::from(returns & bit != 0));
    }
    let records = e.mem.alloc(0x20);
    for k in 0..8 {
        let r = e.mem.alloc(0x44);
        set(e, records + 4 * k, r);
    }
    set(e, p + 0x18 + 4, records);
    e.mem.set_u16(p + 0x18 + 8, 8);
    e.mem.set_u16(p + 0x18 + 0xa, 8);
    set(e, RETS + 0x904, c.word(4));
    set(e, RETS + 0x908, c.f32(9).to_bits());
    let origin = points(e, 1, u32::MAX, 0.0);
    let direction = e.mem.alloc(0xc);
    for (k, v) in [0.0f32, 0.6, 0.8].iter().enumerate() {
        set(e, direction + 4 * k as u32, v.to_bits());
    }
    Picked {
        origin,
        direction,
        pick: p,
        geometry,
    }
}

const PICK_INPUTS: &[Input] = &[
    Input::Of(&[0, 1, 2, 3]),
    Input::Of(&[1, 0, 2]),
    Input::Of(&[0, 1]),
    Input::Of(&[0, 1]),
    Input::Of(&[0x5, 0, 0xff, 0x2]),
    Input::Of(&[0xf, 0, 0xb, 0x3, 0x8]),
    Input::Of(&[0, 0xf, 0x1, 0x6]),
    Input::Of(&[0xf, 0, 0x1, 0xe, 0x9]),
    Input::Of(&[0, 1]),
    Input::F32,
    Input::Of(&[0, 4, 13, 99]),
];

/// `00e9b4f0`: each class (the general triangles, a single strip and two strips, a shape), the
/// centre for intersect type 0 or a skin, each coordinate type, ray tests hitting some, none or all,
/// each return, each lock, the arrays kept or not, stopping at the first hit or not, the case's float
/// in a position. The translation calls the strips' and the shape's picks and the record adder
/// plainly. The leaves family is named too: the point helpers the picks call make points through
/// its typed constructor (`00416870`).
#[test]
fn pick_geometry() {
    Diff::new(entry!(
        0x00e9_b4f0,
        pick::pick_geometry(Ptr, Ptr, Ptr, Ptr) -> bool
    ))
    .family(&geometry::GEOMETRY)
    .family(&leaf::LEAVES)
    .translation_calls_plainly(&[0x00e9_93d0, 0x00e9_a280, 0x00e9_9330])
    .max_calls(MAX_CALLS)
    .inputs(PICK_INPUTS)
    .max_cases(800)
    .float_sweep(32)
    .setup(|e, c| {
        let w = pick_world(e, c);
        vec![w.origin, w.direction, w.pick, w.geometry]
    })
    .run()
    .assert_same();
}

/// `00e993d0`, called directly: the strips' pick over the same inputs.
#[test]
fn pick_strips() {
    Diff::new(entry!(0x00e9_93d0, pick::pick_strips(Ptr, Ptr, Ptr, Ptr) -> bool))
        .family(&geometry::GEOMETRY)
        .family(&leaf::LEAVES)
        .translation_calls_plainly(&[0x00e9_9330])
        .max_calls(MAX_CALLS)
        .inputs(PICK_INPUTS)
        .max_cases(500)
        .setup(|e, c| {
            let w = pick_world(e, c);
            vec![w.origin, w.direction, w.pick, w.geometry]
        })
        .run()
        .assert_same();
}

/// `00e9a280`, called directly: the shape's pick over the same inputs.
#[test]
fn pick_shape() {
    Diff::new(entry!(0x00e9_a280, pick::pick_shape(Ptr, Ptr, Ptr, Ptr) -> bool))
        .family(&geometry::GEOMETRY)
        .family(&leaf::LEAVES)
        .translation_calls_plainly(&[0x00e9_9330])
        .max_calls(MAX_CALLS)
        .inputs(PICK_INPUTS)
        .max_cases(500)
        .setup(|e, c| {
            let w = pick_world(e, c);
            vec![w.origin, w.direction, w.pick, w.geometry]
        })
        .run()
        .assert_same();
}
