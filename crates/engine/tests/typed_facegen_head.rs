//! The differential test of the face family's head making (`engine_typed::facegen::head`) against
//! the translations: the EGM morphs moved into a head part (`0065a970`), the face tint summed from
//! the EGT morphs (`0065b410`) and its packing into pixel bytes (`0064ceb0`).
//!
//! The translations make calls by address per vertex or texel: the checked vectors' and iterators'
//! getters, reads and steps, the coordinates' row, the strided array's read and write (with its
//! bounds check that does nothing), `_ftol2`, the float floor. The typed loops make none, so those
//! callees are left out of the call comparison (`ignore_calls`); every other call (the lock, the
//! loads, the geometry's lock and unlock, the image's making, `GetCompressedImage`, the pixel
//! data's and the texture's), every byte of memory (the scratch blocks the translations free
//! included) and the heap compare. The other callees are stand-ins over the case's objects.
//!
//! The shapes are small (a head has about 1,400 vertices and a 256 by 256 tint; the loops do the
//! same per vertex and texel, and the translation's side logs every call). Floats keep the exe's
//! x87 steps at the thread's precision on both sides, so they compare bit for bit under both
//! control words; the values take the awkward ones (signed zeros, subnormals, the largest float,
//! NaNs, values past the integer range).

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::{
    memory_differences_beside_stack, memory_digest_beside_stack, Case, Diff, Input,
};
use engine_core::typed::leaf;
use engine_typed::facegen::{self, head};

/// The case's words the stand-ins read.
const TEST: u32 = 0x0300_0000;
const ARRAY: u32 = TEST;
const STRIDE: u32 = TEST + 4;
const VERTICES: u32 = TEST + 8;
const EXTRA_DATA: u32 = TEST + 0xc;
const DEFAULT_COORDS: u32 = TEST + 0x10;
const LOADED: u32 = TEST + 0x14;
const HAS_TEXT: u32 = TEST + 0x18;
const LOADED_DATA: u32 = TEST + 0x1c;
const LOCKS: u32 = TEST + 0x20;
const HAS_DATA: u32 = TEST + 0x24;
const KEEP_FLAGS: u32 = TEST + 0x28;
const MARKS: u32 = TEST + 0x2c;
const MIPMAPS: u32 = TEST + 0x30;
const PRINTS: u32 = TEST + 0x34;
const ASSERTS: u32 = TEST + 0x38;
const DEBUG_SEEN: u32 = TEST + 0x3c;
/// The pixel data's virtual destructor (a double) and its table.
const PIXEL_DELETE: u32 = TEST + 0x800;
const PIXEL_VTABLE: u32 = TEST + 0x900;

/// The getters, iterators and steps `0065a970`'s translation calls per morph or vertex.
const MESH_IGNORED: [u32; 11] = [
    0x0065_7c00, // the EGM morph set's count
    0x0065_7c20, // its checked element
    0x0065_7bc0, // a morph's vertex count
    0x0060_b370, // the coordinates' row
    0x0062_9ab0, // the row's element
    0x0044_a170, // the offsets vector's begin
    0x0064_dda0, // the checked iterator's read
    0x0065_dae0, // its six-byte step
    0x0053_4570, // the strided array's read
    0x0045_34f0, // its bounds check (does nothing)
    0x0068_15c0, // the stack point's constructor (a leaf that answers its `this`)
];

/// What `0065b410`'s translation calls per morph and channel between its texel sums.
const TINT_IGNORED: [u32; 6] = [
    0x0065_cea0, // the EGT morph set's count
    0x0065_cec0, // its checked element
    0x0065_cc70, // a channel's checked first byte
    0x0060_b370, // the coordinates' row
    0x0062_9ab0, // the row's element
    0x00ec_62c0, // `_ftol2`
];

/// What `0064ceb0`'s translation calls per pixel and channel.
const IMAGE_IGNORED: [u32; 3] = [
    0x0064_dbd0, // the values iterator's element (its checked read)
    0x0064_ddf0, // its sixteen-byte step
    0x0040_4040, // the float floor (with its `_ftol2`)
];

/// A proxy for the vector at `vector` (its first word; the proxy's first word is the vector), as
/// the checked iterators adopt it.
fn proxy(e: &mut Engine, vector: u32) {
    let proxy = e.mem.alloc(8);
    e.mem.set_u32(proxy, vector);
    e.mem.set_u32(vector, proxy);
}

fn ret(eax: u32) -> Ret {
    Ret {
        eax,
        ..Ret::default()
    }
}

fn counted(e: &mut Engine, at: u32) {
    let n = e.mem.u32(at);
    e.mem.set_u32(at, n + 1);
}

/// The exe's constants these read; the C runtime's SSE2 flag as the case says.
fn constants(e: &mut Engine, sse2: u32, coefficient_scale: f64, color_scale: u32) {
    e.map(0x0101_2000, 0x1000);
    e.mem.set_f64(0x0101_2060, 0.0);
    e.mem.set_f64(0x0101_2070, 1.0);
    e.map(0x0102_3000, 0x1000);
    e.mem.set_f64(0x0102_31d8, coefficient_scale);
    e.mem.set_f32(0x0102_3cd8, 255.0);
    e.map(0x0106_5000, 0x1000);
    e.mem.set_f32(0x0106_5b98, 0.0);
    e.mem.set_u32(0x0106_5b9c, color_scale);
    e.map(0x0101_6000, 0x1000);
    e.mem.set_f32(0x0101_6248, 1.0);
    e.map(0x0127_0000, 0x1000);
    e.mem.set_u32(0x0127_0a6c, sse2);
    e.map(0x011f_9000, 0x1000);
    e.map(0x011d_5000, 0x1000);
}

/// The callees outside the loops, over the case's words.
fn stand_ins(e: &mut Engine) {
    e.map(TEST, 0x1000);
    // The face generation lock and unlock: counted.
    e.register(0x0065_2140, |e, _| {
        counted(e, LOCKS);
        ret(0)
    });
    e.register(0x0065_2190, |e, _| {
        let n = e.mem.u32(LOCKS);
        e.mem.set_u32(LOCKS, n.wrapping_sub(1));
        ret(0)
    });
    e.register(0x0065_21e0, |e, _| ret(e.mem.u32(DEFAULT_COORDS)));
    e.register(0x0065_1f70, |e, _| ret(e.mem.u32(LOADED)));
    e.register(0x0065_2040, |e, _| ret(e.mem.u32(LOADED)));
    e.register(0x0040_48e0, |e, _| ret(e.mem.u32(HAS_TEXT)));
    // The EGM data's constructor and the EGT data's loader: the case's data.
    e.register(0x0065_9560, |e, _| ret(e.mem.u32(LOADED_DATA)));
    e.register(0x0065_9780, |e, _| ret(e.mem.u32(LOADED_DATA)));
    // `operator new`, `operator delete`, the game's allocation, `memset`.
    e.register(0x0040_1000, |e, a| ret(e.mem.alloc(a[0].max(1))));
    e.register(0x0040_1030, |e, a| {
        e.mem.free(a[0]);
        ret(0)
    });
    e.register(0x00aa_13e0, |e, a| ret(e.mem.alloc(a[0].max(1))));
    e.register(0x0040_3d30, |e, a| {
        let fill = vec![a[1].to_le_bytes()[0]; a[2] as usize];
        e.mem.write(a[0], &fill);
        ret(a[0])
    });
    // `NiPointer`: from a raw pointer, assignment (stores), destructor (nothing).
    e.register(0x0063_3c90, |e, a| {
        e.mem.set_u32(a[0], a[1]);
        ret(a[0])
    });
    e.register(0x0066_b0d0, |e, a| {
        e.mem.set_u32(a[0], a[1]);
        ret(a[0])
    });
    e.register(0x006e_5cc0, |e, a| {
        let other = e.mem.u32(a[1]);
        e.mem.set_u32(a[0], other);
        ret(a[0])
    });
    e.register(0x0045_cec0, |_, _| ret(0));
    // Messages and asserts: counted.
    e.register(0x005b_5e40, |e, _| {
        counted(e, PRINTS);
        ret(0)
    });
    e.register(0x00aa_fb90, |e, _| {
        counted(e, ASSERTS);
        ret(0)
    });
}

/// Float words a face's coordinates, offsets and colours take, with the awkward ones among them.
fn value(i: u32, k: u32, pattern: u32) -> u32 {
    let n = i
        .wrapping_mul(2_654_435_761)
        .wrapping_add(k * 97 + pattern * 13);
    match n % 29 {
        0 => 0x8000_0000,                 // -0.0
        1 => 0,                           // 0.0
        2 => 0x0000_0001,                 // the smallest subnormal
        3 => 0x7f7f_ffff,                 // the largest float
        4 if pattern == 2 => 0x7fc0_0000, // a NaN
        5 if pattern == 2 => 0xff80_0000, // negative infinity
        6 => 0x4f80_0000,                 // 2^32
        _ => ((n % 2001) as f32 / 97.0 - 10.0).to_bits(),
    }
}

/// A coefficient: small weights a face has, zeros, and the awkward ones.
fn coefficient(i: u32, pattern: u32) -> u32 {
    match (i * 7 + pattern) % 11 {
        0 => 0,
        1 => 0x8000_0000,
        2 if pattern == 2 => 0x7fc0_0000,
        3 => 0x4f00_0000, // 2^31
        4 => 0xbf80_0000, // -1.0
        _ => (((i * 37 + pattern * 11) % 200) as f32 / 100.0 - 1.0).to_bits(),
    }
}

/// A coordinate set at `set`: `count` coefficients, one per row.
fn coordinate_set(e: &mut Engine, set: u32, count: u32, pattern: u32, salt: u32) {
    let floats = e.mem.alloc(4 * count.max(1));
    for i in 0..count {
        e.mem
            .set_u32(floats + 4 * i, coefficient(i + salt, pattern));
    }
    e.mem.set_u32(set + 0xc, floats);
    e.mem.set_u32(set + 0x10, floats + 4 * count);
    e.mem.set_u32(set + 0x1c, 1);
    // The texture morph count `009611e0` reads.
    e.mem.set_u32(set + 0x18, count);
}

/// Face generation coordinates: the two EGM sets (+0, +0x20) and the two EGT sets (+0x40, +0x60).
fn coordinates(e: &mut Engine, counts: [u32; 4], pattern: u32) -> u32 {
    let coords = e.mem.alloc(0x80);
    for (k, &count) in (0u32..).zip(counts.iter()) {
        coordinate_set(e, coords + 0x20 * k, count, pattern, 3 * k);
    }
    coords
}

// ------------------------------------------------------------------------------------------------
// 0065a970: the EGM morphs
// ------------------------------------------------------------------------------------------------

/// The shapes: (vertices, morphs in each set, the offsets of the last morph of set 0 short by).
const MESH_SHAPES: [(u32, [u32; 2], u32); 6] = [
    (0, [1, 1], 0),
    (3, [2, 1], 0),
    (9, [3, 2], 0),
    (14, [2, 3], 5),
    (22, [4, 1], 0),
    // A multiple of four: the points block's last group of four, with no single point after it.
    (12, [3, 2], 0),
];

/// The EGM data: two sets of morphs (0x1c bytes: the scale, then the offsets vector at +4) with
/// `vertices` offsets each (fewer for the last morph of set 0 by `short`).
fn egm(e: &mut Engine, vertices: u32, morphs: [u32; 2], short: u32, pattern: u32) -> u32 {
    let data = e.mem.alloc(0x34);
    for (set, &n) in (0u32..).zip(morphs.iter()) {
        let at = data + 4 + 0x18 * set;
        let block = e.mem.alloc(0x1c * n.max(1));
        e.mem.set_u32(at + 0xc, block);
        e.mem.set_u32(at + 0x10, block + 0x1c * n);
        for i in 0..n {
            let morph = block + 0x1c * i;
            e.mem.set_u32(morph, value(i, set + 40, pattern));
            // Room for the additional vertices past the count (three more).
            let count = if set == 0 && i + 1 == n && short > 0 {
                vertices.saturating_sub(short)
            } else {
                vertices + 3
            };
            let offsets = e.mem.alloc(6 * count.max(1));
            for v in 0..count {
                for axis in 0..3 {
                    let n = (v * 3 + axis)
                        .wrapping_mul(2_246_822_519)
                        .wrapping_add(i * 31 + set * 7 + pattern);
                    let offset = match n % 13 {
                        0 => i16::MAX,
                        1 => i16::MIN,
                        2 => 0,
                        _ => (n % 4001) as i16 - 2000,
                    };
                    e.mem.set_i16(offsets + 6 * v + 2 * axis, offset);
                }
            }
            let vector = morph + 4;
            e.mem.set_u32(vector + 0xc, offsets);
            e.mem.set_u32(vector + 0x10, offsets + 6 * count);
            proxy(e, vector);
        }
    }
    data
}

fn mesh_stand_ins(e: &mut Engine) {
    // The strided iterator's constructor: zeroed.
    e.register(0x0053_4550, |e, a| {
        for k in 0..3 {
            e.mem.set_u32(a[0] + 4 * k, 0);
        }
        ret(a[0])
    });
    // The geometry's data (+0xb8), its lock (the case's answer), its vertices (the case's array
    // and stride), the stride word, the vertex count, the morph extra data.
    e.register(0x0054_95f0, |e, a| ret(e.mem.u32(a[0] + 0xb8)));
    e.register(0x00a6_7360, |e, _| ret(e.mem.u32(HAS_DATA) & 1));
    e.register(0x00a6_7410, |e, a| {
        let (array, stride) = (e.mem.u32(ARRAY), e.mem.u32(STRIDE));
        e.mem.set_u32(a[1], array);
        e.mem.set_u32(a[1] + 4, stride);
        ret(0)
    });
    e.register(0x0071_7e50, |_, a| ret(a[0] + 4));
    e.register(0x0045_6650, |e, _| ret(e.mem.u32(VERTICES) & 0xffff));
    e.register(0x0065_3430, |e, _| ret(e.mem.u32(EXTRA_DATA)));
    e.register(0x0049_ec60, |e, _| ret(e.mem.u32(HAS_DATA) >> 1));
    e.register(0x0045_7b80, |e, _| ret(e.mem.u32(KEEP_FLAGS)));
    e.register(0x00a6_7090, |e, _| {
        counted(e, MARKS);
        ret(0)
    });
    e.register(0x00a6_73d0, |_, _| ret(0));
}

#[test]
fn facegen_head_apply_coordinates() {
    Diff::new(entry!(
        0x0065_a970,
        head::apply_coordinates(Ptr<_>, u32, u32, u8) -> bool
    ))
    .family(&facegen::FACEGEN)
    .family(&leaf::LEAVES)
    .ignore_calls(&MESH_IGNORED)
    .inputs(&[
        Input::Of(&[0, 1, 2, 3, 4, 5]),
        // Strides: 8 is below a point (the points overlap: the element loops), the rest hold one.
        Input::Of(&[8, 12, 16, 32]),
        Input::Of(&[0, 1]),
        Input::Of(&[0, 1, 2]),
        Input::Flag,
        // How the mesh comes: 0 loaded; 1 the data made from its file; 2 no data; 3 no file name;
        // 4 no mesh data; 5 no coordinates; 6 the default coordinates; 7 no vertices.
        Input::Of(&[0, 1, 2, 3, 4, 5, 6, 7]),
        // The geometry's data test and keep flags: marked, not marked twice.
        Input::Of(&[3, 1, 0]),
        Input::Of(&[0, 0x4000]),
    ])
    .setup(|e, case: &Case| {
        constants(e, 1, 256.0, 0x3780_0000);
        stand_ins(e);
        mesh_stand_ins(e);
        let (vertices, morphs, short) = MESH_SHAPES[case.word(0) as usize];
        let stride = case.word(1);
        // A short morph lowers the count; the additional vertices past it would read its offsets
        // past their end (no head's data does), so that shape has none.
        let with_extra = case.word(2) == 1 && short == 0;
        let pattern = case.word(3);
        let apply_all = case.word(4);
        let how = case.word(5);
        e.mem.set_u32(HAS_DATA, case.word(6));
        e.mem.set_u32(KEEP_FLAGS, case.word(7));
        let array = e.mem.alloc(stride * vertices.max(1) + 16);
        for v in 0..vertices {
            for k in 0..stride / 4 {
                e.mem
                    .set_u32(array + stride * v + 4 * k, value(v, k + 5, pattern));
            }
        }
        e.mem.set_u32(ARRAY, if how == 7 { 0 } else { array });
        e.mem.set_u32(STRIDE, stride);
        e.mem.set_u32(VERTICES, vertices);
        if with_extra {
            let total = vertices + 3;
            let extra = e.mem.alloc(12 * total);
            for v in 0..total {
                for k in 0..3 {
                    e.mem
                        .set_u32(extra + 12 * v + 4 * k, value(v, k + 9, pattern));
                }
            }
            let data = e.mem.alloc(0x18);
            e.mem.set_u32(data + 0xc, extra);
            e.mem.set_u32(data + 0x10, total);
            e.mem.set_u32(EXTRA_DATA, data);
        }
        let data = egm(e, vertices, morphs, short, pattern);
        let coords = coordinates(e, [morphs[0], morphs[1], 0, 0], pattern);
        let mesh = e.mem.alloc(0x24);
        e.mem.set_u32(mesh, 0x0700_0000);
        match how {
            1 => {
                e.mem.set_u32(HAS_TEXT, 1);
                e.mem.set_u32(LOADED_DATA, data);
            }
            2 => e.mem.set_u32(HAS_TEXT, 1),
            3 => {}
            _ => {
                e.mem.set_u32(LOADED, 1);
                e.mem.set_u32(mesh + 8, data);
            }
        }
        let model = e.mem.alloc(0x10);
        e.mem.set_u32(model + 8, if how == 4 { 0 } else { mesh });
        let geometry = e.mem.alloc(0xc0);
        let geometry_data = e.mem.alloc(0x40);
        e.mem.set_u32(geometry + 0xb8, geometry_data);
        let given = match how {
            5 | 6 => 0,
            _ => coords,
        };
        e.mem
            .set_u32(DEFAULT_COORDS, if how == 6 { coords } else { 0 });
        vec![model, given, geometry, apply_all]
    })
    .max_cases(220)
    .max_calls(100_000)
    .run()
    .assert_same();
}

// ------------------------------------------------------------------------------------------------
// 0065b410: the face tint
// ------------------------------------------------------------------------------------------------

/// The shapes: (width, height, morphs, the row padding, a morph of another size at this index or
/// none).
const TINT_SHAPES: [(u32, u32, u32, u32, Option<u32>); 6] = [
    (1, 1, 1, 0, None),
    (4, 3, 3, 0, None),
    (5, 2, 4, 3, None),
    (8, 8, 2, 1, None),
    (4, 3, 3, 0, Some(1)),
    (0, 3, 2, 0, None),
];

/// The EGT data: set 0 with `morphs` morphs (0x58 bytes: the scale, width, height, padding, then
/// three channel byte vectors at +0x10, +0x28, +0x40), set 1 empty.
fn egt(
    e: &mut Engine,
    width: u32,
    height: u32,
    morphs: u32,
    padding: u32,
    odd: Option<u32>,
    pattern: u32,
) -> u32 {
    let data = e.mem.alloc(0x34);
    let block = e.mem.alloc(0x58 * morphs.max(1));
    e.mem.set_u32(data + 4 + 0xc, block);
    e.mem.set_u32(data + 4 + 0x10, block + 0x58 * morphs);
    for i in 0..morphs {
        let morph = block + 0x58 * i;
        let (w, h) = if odd == Some(i) {
            (width + 1, height)
        } else {
            (width, height)
        };
        e.mem.set_u32(morph, value(i, 50, pattern));
        e.mem.set_u32(morph + 4, w);
        e.mem.set_u32(morph + 8, h);
        e.mem.set_u32(morph + 0xc, padding);
        for channel in 0..3 {
            let len = (w + padding) * h.max(1) + 8;
            let bytes = e.mem.alloc(len);
            for k in 0..len {
                let n = k
                    .wrapping_mul(2_654_435_761)
                    .wrapping_add(i * 131 + channel * 17 + pattern);
                e.mem.set_u8(bytes + k, (n >> 13) as u8);
            }
            let vector = morph + 0x10 + 0x18 * channel;
            e.mem.set_u32(vector + 0xc, bytes);
            e.mem.set_u32(vector + 0x10, bytes + len);
        }
    }
    data
}

fn tint_stand_ins(e: &mut Engine) {
    // The iterator's start: two zero words.
    e.register(0x0065_db40, |e, a| {
        e.mem.set_u32(a[0], 0);
        e.mem.set_u32(a[0] + 4, 0);
        ret(a[0])
    });
    e.register(0x0060_45f0, |_, _| ret(0x0700_0010));
    e.register(0x0096_11e0, |e, a| ret(e.mem.u32(a[0] + 0x18)));
    // The image: made (0x2c bytes), its width and height, its colour vector at +0xc, the resize to
    // `count` values of four floats each.
    e.register(0x0064_ccf0, |e, a| {
        e.mem.set_u32(a[0], 0x0700_0020);
        ret(a[0])
    });
    e.register(0x0070_37c0, |e, a| {
        e.mem.set_u32(a[0] + 0x24, a[1]);
        ret(0)
    });
    e.register(0x0061_cc40, |e, a| {
        e.mem.set_u32(a[0] + 0x28, a[1]);
        ret(0)
    });
    e.register(0x0064_d880, |_, a| ret(a[0] + 0xc));
    e.register(0x0064_db50, |e, a| {
        let values = e.mem.alloc(16 * a[1].max(1));
        for k in 0..a[1] {
            for word in 0..4 {
                let zero = e.mem.u32(a[2] + 4 * word);
                e.mem.set_u32(values + 16 * k + 4 * word, zero);
            }
        }
        e.mem.set_u32(a[0] + 0xc, values);
        e.mem.set_u32(a[0] + 0x10, values + 16 * a[1]);
        proxy(e, a[0]);
        ret(0)
    });
    // `GetCompressedImage`: its own test; here the texture it answers.
    e.register(0x0064_ceb0, |e, a| {
        let made = e.mem.alloc(8);
        e.mem.set_u32(made, a[0]);
        ret(made)
    });
}

#[test]
fn facegen_head_apply_coordinate_texturing() {
    Diff::new(entry!(
        0x0065_b410,
        head::apply_coordinate_texturing(Ptr<_>, u32, u32, u32) -> bool
    ))
    .family(&leaf::LEAVES)
    .ignore_calls(&TINT_IGNORED)
    .inputs(&[
        Input::Of(&[0, 1, 2, 3, 4, 5]),
        Input::Of(&[0, 1, 2]),
        // The morph limit: all, one, more than there are.
        Input::Of(&[0, 1, 9]),
        // How the texture data comes: 0 loaded; 1 made from its file; 2 no data from the file;
        // 3 no file name; 4 no texture data; 5 no coordinates; 6 the default coordinates; 7 the
        // coordinates have too few texture morphs.
        Input::Of(&[0, 1, 2, 3, 4, 5, 6, 7]),
        // `_ftol2`'s body: SSE2, the x87 one.
        Input::Of(&[1, 0]),
        // The coefficient scale (a double) and the colour scale (a float's bits).
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0x3780_0000, 0x3f80_0000, 0x7fc0_0000, 0x0000_0001]),
    ])
    .setup(|e, case: &Case| {
        let coefficient_scale = match case.word(5) {
            1 => -3.5e9,
            2 => 1.0e-3,
            _ => 256.0,
        };
        constants(e, case.word(4), coefficient_scale, case.word(6));
        stand_ins(e);
        tint_stand_ins(e);
        let (width, height, morphs, padding, odd) = TINT_SHAPES[case.word(0) as usize];
        let pattern = case.word(1);
        let how = case.word(3);
        let data = egt(e, width, height, morphs, padding, odd, pattern);
        let available = if how == 7 { morphs - 1 } else { morphs };
        let coords = coordinates(e, [0, 0, available, 0], pattern);
        let texture_data = e.mem.alloc(0xc);
        e.mem.set_u32(texture_data, 0x0700_0000);
        match how {
            1 => {
                e.mem.set_u32(HAS_TEXT, 1);
                e.mem.set_u32(LOADED_DATA, data);
            }
            2 => e.mem.set_u32(HAS_TEXT, 1),
            3 => {}
            _ => {
                e.mem.set_u32(LOADED, 1);
                e.mem.set_u32(texture_data + 8, data);
            }
        }
        let model = e.mem.alloc(0x10);
        e.mem
            .set_u32(model + 0xc, if how == 4 { 0 } else { texture_data });
        let texture = e.mem.alloc(4);
        let given = match how {
            5 | 6 => 0,
            _ => coords,
        };
        e.mem
            .set_u32(DEFAULT_COORDS, if how == 6 { coords } else { 0 });
        vec![model, given, texture, case.word(2)]
    })
    .max_cases(240)
    .max_calls(100_000)
    .run()
    .assert_same();
}

// ------------------------------------------------------------------------------------------------
// 0064ceb0: the compressed image
// ------------------------------------------------------------------------------------------------

fn image_stand_ins(e: &mut Engine) {
    // The values' emptiness test.
    e.register(0x0064_db00, |e, a| {
        ret(u32::from(e.mem.u32(a[0] + 0xc) == e.mem.u32(a[0] + 0x10)))
    });
    // The converter: made (a word in it), unmade, its mipmaps (new pixel data).
    e.register(0x00a7_b110, |e, a| {
        e.mem.set_u32(a[0], 0x0700_0030);
        ret(a[0])
    });
    e.register(0x00a7_aff0, |_, _| ret(0));
    e.register(0x00a7_8090, |e, _| {
        let made = e.mem.alloc(0x74);
        e.mem.set_u32(made, PIXEL_VTABLE);
        ret(made)
    });
    // The pixel data: made with a buffer of its size (three or four bytes a pixel by the format)
    // at +0x40, its bytes, its deletion (counted).
    e.register(0x00a7_c190, |e, a| {
        let channels = if a[3] == 0x011a_a2e8 { 3 } else { 4 };
        let bytes = e.mem.alloc((a[1] * a[2] * channels).max(1));
        e.mem.set_u32(a[0], PIXEL_VTABLE);
        e.mem.set_u32(a[0] + 0x40, bytes);
        ret(a[0])
    });
    e.register(0x0064_dac0, |e, a| ret(e.mem.u32(a[0] + 0x40)));
    e.register(PIXEL_DELETE, |e, a| {
        e.mem.set_u32(a[0], 0);
        counted(e, MARKS);
        ret(0)
    });
    e.mem.set_u32(PIXEL_VTABLE, PIXEL_DELETE);
    e.register(0x0045_4af0, |e, _| ret(e.mem.u32(MIPMAPS)));
    e.register(0x00b6_b940, |e, _| {
        counted(e, DEBUG_SEEN);
        ret(0)
    });
    e.register(0x00a5_ff10, |e, a| {
        let made = e.mem.alloc(8);
        e.mem.set_u32(made, a[0]);
        ret(made)
    });
    e.register(0x00b6_bbd0, |e, a| {
        let made = e.mem.alloc(8);
        e.mem.set_u32(made, a[0] ^ a[1]);
        ret(made)
    });
}

/// The shapes: (width, height, values short by).
const IMAGE_SHAPES: [(u32, u32, u32); 4] = [(1, 1, 0), (3, 2, 0), (8, 4, 0), (2, 2, 4)];

/// The range and scale arguments the cases try (float bits).
const RANGES: [u32; 7] = [
    0,           // 0.0
    0x437f_0000, // 255.0
    0x3f80_0000, // 1.0
    0xc2c8_0000, // -100.0
    0x7fc0_0000, // a NaN
    0x4f80_0000, // 2^32
    0x8000_0001, // a negative subnormal
];

#[test]
fn facegen_head_get_compressed_image() {
    Diff::new(entry!(
        0x0064_ceb0,
        head::get_compressed_image(Ptr<_>, f32, f32, f32, u32) -> u32
    ))
    .family(&leaf::LEAVES)
    .ignore_calls(&IMAGE_IGNORED)
    .inputs(&[
        Input::Of(&[0, 1, 2, 3]),
        Input::Of(&RANGES),
        Input::Of(&[0x437f_0000, 0x3f80_0000, 0, 0x7fc0_0000, 0x4f80_0000]),
        Input::Of(&[
            0x3f80_0000,
            0x4380_0000,
            0xbf80_0000,
            0x7f80_0000,
            0x3e80_0000,
        ]),
        // The size argument's query: three channels; four, the made texture; four, resized.
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0, 1, 2]),
        // `_ftol2`'s body; the mipmap setting; the debug flag; a texture already there.
        Input::Of(&[1, 0]),
        Input::Flag,
        Input::Flag,
        Input::Flag,
    ])
    .setup(|e, case: &Case| {
        constants(e, case.word(6), 256.0, 0x3780_0000);
        stand_ins(e);
        image_stand_ins(e);
        e.mem.set_u32(MIPMAPS, case.word(7));
        e.mem.set_u8(0x011f_94b0, case.word(8) as u8);
        let (width, height, short) = IMAGE_SHAPES[case.word(0) as usize];
        let pattern = case.word(5);
        let pixels = width * height;
        let count = pixels.saturating_sub(short);
        let values = e.mem.alloc(16 * count.max(1));
        for k in 0..count {
            for word in 0..4 {
                let bits = match (k * 4 + word + pattern) % 5 {
                    0 => value(k, word, pattern),
                    // Ordinary colour values around the range.
                    _ => (((k * 53 + word * 29 + pattern * 7) % 600) as f32 / 2.0 - 20.0).to_bits(),
                };
                e.mem.set_u32(values + 16 * k + 4 * word, bits);
            }
        }
        let image = e.mem.alloc(0x2c);
        e.mem.set_u32(image, 0x0700_0020);
        e.mem
            .set_u32(image + 8, if case.word(9) == 1 { 0x0700_0040 } else { 0 });
        e.mem.set_u32(image + 0xc + 0xc, values);
        e.mem.set_u32(image + 0xc + 0x10, values + 16 * count);
        proxy(e, image + 0xc);
        e.mem.set_u32(image + 0x24, width);
        e.mem.set_u32(image + 0x28, height);
        let size = e.mem.alloc(8);
        e.mem.set_u32(size + 4, case.word(4));
        vec![image, case.word(1), case.word(2), case.word(3), size]
    })
    .max_cases(300)
    .max_calls(100_000)
    .run()
    .assert_same();
}

// ------------------------------------------------------------------------------------------------
// The checks that fail: translation and typed version on two engines made alike
// ------------------------------------------------------------------------------------------------

/// The invalid-parameter calls (`_invalid_parameter_noinfo`, `00ec7c56`), counted.
const INVALID: u32 = TEST + 0x40;

/// `_invalid_parameter_noinfo` as a counter that goes on (as the C runtime's goes on to report and
/// the caller then reads on, which a release build does when no handler stops it).
fn count_invalid(e: &mut Engine) {
    e.register(0x00ec_7c56, |e, _| {
        counted(e, INVALID);
        ret(0)
    });
}

/// Runs the translation at `typed.0` and, on a second engine made alike, the typed version, on the
/// words `setup` answers, and compares what the Diff cannot when a checked getter fails (the
/// translation's invalid-parameter calls lie under the getters the Diff leaves out, the typed
/// version's are its own): the answer, how many invalid-parameter calls each made (at least one),
/// the heap's top and free bytes, and every byte of memory beside the stack. Under both control
/// words.
fn both_ways(typed: (u32, AbiFn), setup: impl Fn(&mut Engine) -> Vec<u32>) {
    both_ways_checking(typed, setup, true);
}

/// [`both_ways`], with `failing` false for a case whose checks all pass (no invalid-parameter
/// call on either side).
fn both_ways_checking(typed: (u32, AbiFn), setup: impl Fn(&mut Engine) -> Vec<u32>, failing: bool) {
    for word in [0x027fu16, 0x007f] {
        let mut sides = Vec::new();
        for typed_side in [false, true] {
            let mut e = Engine::new_untyped_for_test();
            engine_core::x87::set_control_word(word);
            let words = setup(&mut e);
            count_invalid(&mut e);
            // The stack for locals made before the call, as the Diff makes it: a side that never
            // uses it would otherwise leave the heap a megabyte lower.
            e.with_stack(4, |_, _| ());
            if typed_side {
                e.register(typed.0, typed.1);
            }
            let answer = e.call(typed.0, &words);
            sides.push((e, answer.eax));
        }
        let (a, b) = (&sides[0].0, &sides[1].0);
        assert_eq!(
            sides[0].1, sides[1].1,
            "the answer, control word {word:04x}"
        );
        assert_eq!(
            a.mem.u32(INVALID) > 0,
            failing,
            "whether a check failed (a failing case that does not tests nothing)"
        );
        assert_eq!(
            a.mem.u32(INVALID),
            b.mem.u32(INVALID),
            "invalid-parameter calls"
        );
        assert_eq!(
            (a.mem.heap_top(), a.mem.heap_available()),
            (b.mem.heap_top(), b.mem.heap_available()),
            "the heap"
        );
        let differences = memory_differences_beside_stack(a, b, 8);
        assert!(
            differences.is_empty(),
            "memory differs at {differences:08x?}, control word {word:04x}"
        );
        assert_eq!(memory_digest_beside_stack(a), memory_digest_beside_stack(b));
    }
}

/// A head part for [`facegen_head_failing_checks_of_the_morphs`]: `vertices` in the geometry,
/// `offsets` per morph (the EGM data is built for `offsets - 3`), `total` additional vertices (none
/// when not above `vertices`), the set 0 coordinates with `floats` coefficients.
fn failing_mesh(e: &mut Engine, vertices: u32, offsets: u32, total: u32, floats: u32) -> Vec<u32> {
    constants(e, 1, 256.0, 0x3780_0000);
    stand_ins(e);
    mesh_stand_ins(e);
    let stride = 16;
    let array = e.mem.alloc(stride * vertices + 16);
    for v in 0..vertices {
        for k in 0..4 {
            e.mem
                .set_u32(array + stride * v + 4 * k, value(v, k + 5, 1));
        }
    }
    e.mem.set_u32(ARRAY, array);
    e.mem.set_u32(STRIDE, stride);
    e.mem.set_u32(VERTICES, vertices);
    e.mem.set_u32(HAS_DATA, 3);
    if total > vertices {
        let extra = e.mem.alloc(12 * total);
        for v in 0..total {
            for k in 0..3 {
                e.mem.set_u32(extra + 12 * v + 4 * k, value(v, k + 9, 1));
            }
        }
        let data = e.mem.alloc(0x18);
        e.mem.set_u32(data + 0xc, extra);
        e.mem.set_u32(data + 0x10, total);
        e.mem.set_u32(EXTRA_DATA, data);
    }
    let data = egm(e, offsets - 3, [3, 2], 0, 1);
    let coords = coordinates(e, [floats, 2, 0, 0], 1);
    let mesh = e.mem.alloc(0x24);
    e.mem.set_u32(mesh + 8, data);
    e.mem.set_u32(LOADED, 1);
    let model = e.mem.alloc(0x10);
    e.mem.set_u32(model + 8, mesh);
    let geometry = e.mem.alloc(0xc0);
    let geometry_data = e.mem.alloc(0x40);
    e.mem.set_u32(geometry + 0xb8, geometry_data);
    vec![model, coords, geometry, 1]
}

#[test]
fn facegen_head_failing_checks_of_the_morphs() {
    let typed = || {
        entry!(
            0x0065_a970,
            head::apply_coordinates(Ptr<_>, u32, u32, u8) -> bool
        )
    };
    // The offsets end before the additional vertices do: their reads and steps past the end.
    both_ways(typed(), |e| failing_mesh(e, 9, 10, 12, 3));
    // The set 0 coordinates have no coefficient: each morph's row read past the empty vector.
    both_ways(typed(), |e| failing_mesh(e, 6, 9, 0, 0));
}

/// A head part whose additional vertices lie one point into its own packed vertex array (no
/// head's data does): each group of the loops reads points an earlier group wrote, so the one-pass
/// path must step aside for the element loops, which then leave every byte as the translation's.
/// The checks all pass. With the additional vertices one point past the array's end instead (no
/// overlap), the one-pass path itself runs against the translation.
fn shifted_mesh(e: &mut Engine, shift: u32) -> Vec<u32> {
    constants(e, 1, 256.0, 0x3780_0000);
    stand_ins(e);
    mesh_stand_ins(e);
    let (vertices, stride) = (9u32, 12u32);
    let total = vertices + 3;
    let points = vertices + total + 2;
    // Plain finite points, morph scales and weights: a NaN or an infinity would spread to every
    // point after a morph or two and hide in what order the points were written.
    let array = e.mem.alloc(stride * points + 16);
    for v in 0..points {
        for k in 0..3 {
            let point = (v * 3 + k) as f32 * 0.5 + 1.0;
            e.mem.set_f32(array + stride * v + 4 * k, point);
        }
    }
    e.mem.set_u32(ARRAY, array);
    e.mem.set_u32(STRIDE, stride);
    e.mem.set_u32(VERTICES, vertices);
    e.mem.set_u32(HAS_DATA, 3);
    let data = e.mem.alloc(0x18);
    e.mem.set_u32(data + 0xc, array + stride * shift);
    e.mem.set_u32(data + 0x10, total);
    e.mem.set_u32(EXTRA_DATA, data);
    let egm_data = egm(e, vertices, [3, 2], 0, 1);
    for (set, n) in [(0u32, 3u32), (1, 2)] {
        let block = e.mem.u32(egm_data + 4 + 0x18 * set + 0xc);
        for i in 0..n {
            e.mem.set_f32(block + 0x1c * i, 0.25);
        }
    }
    // Pattern 1's weights: -0.0, -0.52 and -1.0.
    let coords = coordinates(e, [3, 2, 0, 0], 1);
    let mesh = e.mem.alloc(0x24);
    e.mem.set_u32(mesh + 8, egm_data);
    e.mem.set_u32(LOADED, 1);
    let model = e.mem.alloc(0x10);
    e.mem.set_u32(model + 8, mesh);
    let geometry = e.mem.alloc(0xc0);
    let geometry_data = e.mem.alloc(0x40);
    e.mem.set_u32(geometry + 0xb8, geometry_data);
    vec![model, coords, geometry, 1]
}

#[test]
fn facegen_head_morphs_reading_their_own_writes() {
    let typed = || {
        entry!(
            0x0065_a970,
            head::apply_coordinates(Ptr<_>, u32, u32, u8) -> bool
        )
    };
    // One point in: the additional vertices overlap the vertex array (the element loops).
    both_ways_checking(typed(), |e| shifted_mesh(e, 1), false);
    // Past the array's last vertex: apart (the one-pass path).
    both_ways_checking(typed(), |e| shifted_mesh(e, 9), false);
}

/// A face tint for [`facegen_head_failing_checks_of_the_tint`]: 4 by 3 texels, three morphs; the
/// second morph's last channel empty when `empty_channel`, the coefficients' vector empty (its
/// count still three) when `empty_floats`.
fn failing_tint(e: &mut Engine, empty_channel: bool, empty_floats: bool) -> Vec<u32> {
    constants(e, 1, 256.0, 0x3780_0000);
    stand_ins(e);
    tint_stand_ins(e);
    let data = egt(e, 4, 3, 3, 1, None, 1);
    if empty_channel {
        let morph = e.mem.u32(data + 4 + 0xc) + 0x58;
        let vector = morph + 0x10 + 0x18 * 2;
        let first = e.mem.u32(vector + 0xc);
        e.mem.set_u32(vector + 0x10, first);
    }
    let coords = coordinates(e, [0, 0, 3, 0], 1);
    if empty_floats {
        let set = coords + 0x40;
        let first = e.mem.u32(set + 0xc);
        e.mem.set_u32(set + 0x10, first);
    }
    let texture_data = e.mem.alloc(0xc);
    e.mem.set_u32(texture_data + 8, data);
    e.mem.set_u32(LOADED, 1);
    let model = e.mem.alloc(0x10);
    e.mem.set_u32(model + 0xc, texture_data);
    let texture = e.mem.alloc(4);
    vec![model, coords, texture, 0]
}

#[test]
fn facegen_head_failing_checks_of_the_tint() {
    let typed = || {
        entry!(
            0x0065_b410,
            head::apply_coordinate_texturing(Ptr<_>, u32, u32, u32) -> bool
        )
    };
    both_ways(typed(), |e| failing_tint(e, true, false));
    both_ways(typed(), |e| failing_tint(e, false, true));
}

/// An image for [`facegen_head_failing_checks_of_the_image`]: 3 by 2 pixels with `values` values
/// in a block with room for all six (the reads past the vector's end stay in it); its vector with
/// no proxy when `no_proxy` (the page at 0 mapped, so the missing container's last reads as 0).
fn failing_image(e: &mut Engine, values: u32, no_proxy: bool) -> Vec<u32> {
    constants(e, 1, 256.0, 0x3780_0000);
    stand_ins(e);
    image_stand_ins(e);
    let block = e.mem.alloc(16 * 6);
    for k in 0..6u32 {
        for word in 0..4u32 {
            let bits = (((k * 53 + word * 29) % 600) as f32 / 2.0 - 20.0).to_bits();
            e.mem.set_u32(block + 16 * k + 4 * word, bits);
        }
    }
    let image = e.mem.alloc(0x2c);
    e.mem.set_u32(image + 0xc + 0xc, block);
    e.mem.set_u32(image + 0xc + 0x10, block + 16 * values);
    if no_proxy {
        e.map(0, 0x1000);
    } else {
        proxy(e, image + 0xc);
    }
    e.mem.set_u32(image + 0x24, 3);
    e.mem.set_u32(image + 0x28, 2);
    let size = e.mem.alloc(8);
    vec![image, 0, 255.0f32.to_bits(), 1.0f32.to_bits(), size]
}

#[test]
fn facegen_head_failing_checks_of_the_image() {
    let typed = || {
        entry!(
            0x0064_ceb0,
            head::get_compressed_image(Ptr<_>, f32, f32, f32, u32) -> u32
        )
    };
    // Short by one and by two: the value-by-value path, its last reads and steps past the end.
    both_ways(typed(), |e| failing_image(e, 5, false));
    both_ways(typed(), |e| failing_image(e, 4, false));
    // No proxy: every read and step fails a test.
    both_ways(typed(), |e| failing_image(e, 6, true));
}
