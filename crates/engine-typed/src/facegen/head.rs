//! Making a head (`TESNPC::InitHead`, the race's head builder): the three loops that cost the most
//! while the head is made, on a loading thread that holds the face generation lock (`011d5a9c`)
//! the whole time, so main waits for them whenever it wants that lock (the model loader's walk).
//!
//! - [`apply_coordinates`]: the EGM morphs moved into a head part's vertices
//!   (`0065a970`; every morph of both sets, every vertex, for each part of a new head).
//! - [`apply_coordinate_texturing`]: the face's tint texture summed from the EGT morphs
//!   (`BSFaceGenModel::ApplyCoordinateTexturingToMesh`, `0065b410`; up to 50 morphs times three
//!   channels times every texel).
//! - [`get_compressed_image`]: that image packed into pixel bytes
//!   (`BSFaceGenImage::GetCompressedImage`, `0064ceb0`; every texel's channels clamped, floored
//!   and scaled, the bytes looked up by the floor when the image's range allows a table).
//!
//! The translations make several calls by address per vertex or texel (the checked iterators'
//! reads and steps, the strided array's read and write, the coordinates' row, `_ftol2`, the float
//! floor); here the same reads and writes are plain loads and stores in one loop, and the texel
//! sums run in host memory, written to the game's buffer once. A morph's points go the same way
//! (`morph_in_host`): its offsets read at once, the points morphed in host memory and written back,
//! when no check can fail and nothing the morph writes is read again; else point by point as the
//! exe's loops go (the race menu builds a head two or three times a click, about 13 morph calls
//! of up to 80 morphs over every vertex; a 2026-11-24 profile of the menu put this function's own
//! time first under its slider release). Every call the translations make
//! outside those loops is made, in the same order, with the same words, and every byte they
//! leave (scratch blocks included) is left the same.
//!
//! **Floats.** Heads are made on loading threads and on main (the race menu), so the float steps
//! give the x87's bits at the running thread's precision ([`Threads::Both`]). The morphs' one pass
//! (`morph_words`) reads the control word once a morph and takes plain steps (`f32` at main's 24
//! bits, `f64` at the loading threads' 53) wherever they give the same bits, the x87 helpers
//! elsewhere (a NaN, an infinity, a result outside the normal floats at 24 bits, another rounding
//! mode): a 2026-11-25 profile of the race menu still put this function's own time first in a
//! preset click (about 5 ms), when every coordinate went through two helpers (each reading the
//! control word and testing its result's range). The other loops keep the helpers, each a short
//! Rust function. Integer sums are integer sums (wrapping, so their order does not matter).
//!
//! **Checks.** The game's checked iterators call `_invalid_parameter_noinfo` (`00ec7c56`) when
//! read past their end; the loops here make that call at the same points (a head's data never
//! does it).
//!
//! [`Threads::Both`]: engine_core::typed::Threads::Both

// The float tests are NaN-aware as the exe's compares are (a NaN takes the `else` side).
#![allow(clippy::neg_cmp_op_on_partial_ord)]

use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::x87::{
    fistp_chop, fistp_chop64, x87_add, x87_load32, x87_m32, x87_mul, x87_quiet_bits, x87_store32,
    x87_sub,
};
use engine_core::{args, typed_call, Engine};

/// `_invalid_parameter_noinfo`: what a checked iterator or vector getter calls past its end.
const INVALID_PARAMETER: u32 = 0x00ec_7c56;
/// The C runtime's flag for its SSE2 bodies (`__sse2_available`), which `_ftol2` reads.
const SSE2_AVAILABLE: u32 = crate::crt::SSE2_AVAILABLE;

/// `BSFaceGenManager::GetFaceGenCoord` (`006521e0`), the face generation lock and unlock
/// (`00652140`, `00652190`) and the two "already loaded" tests (`00651f70` mesh, `00652040`
/// texture).
const GET_FACE_GEN_COORD: u32 = 0x0065_21e0;
const FACE_GEN_LOCK: u32 = 0x0065_2140;
const FACE_GEN_UNLOCK: u32 = 0x0065_2190;
const MESH_LOADED_TEST: u32 = 0x0065_1f70;
const TEXTURE_LOADED_TEST: u32 = 0x0065_2040;
/// A `BSStringT`'s text test, the EGM data's constructor (0x34 bytes) and the EGT data's loader.
const STRING_HAS_TEXT: u32 = 0x0040_48e0;
const EGM_DATA_CONSTRUCT: u32 = 0x0065_9560;
const EGT_DATA_LOAD: u32 = 0x0065_9780;
/// `operator new`, `operator delete`, the game's allocation (`00aa13e0`) and `memset`.
const OPERATOR_NEW: u32 = 0x0040_1000;
const OPERATOR_DELETE: u32 = 0x0040_1030;
const NI_ALLOCATE: u32 = 0x00aa_13e0;
const MEMSET: u32 = 0x0040_3d30;
/// `NiPointer`: constructor from a raw pointer, destructor, assignment.
const NI_POINTER_FROM_RAW: u32 = 0x0063_3c90;
const NI_POINTER_DESTRUCT: u32 = 0x0045_cec0;
const NI_POINTER_ASSIGN: u32 = 0x0066_b0d0;
/// The morph extra data of a geometry; the strided vertex iterator's constructor; the geometry's
/// data, its lock, vertex iterator, unlock, `MarkAsChanged` and keep flags; the geometry's
/// `0049ec60`; the iterator's stride word; the geometry's vertex count.
const MORPH_EXTRA_DATA: u32 = 0x0065_3430;
const VERTEX_ITERATOR_CONSTRUCT: u32 = 0x0053_4550;
const GEOMETRY_DATA: u32 = 0x0054_95f0;
const GEOMETRY_DATA_LOCK: u32 = 0x00a6_7360;
const GEOMETRY_DATA_VERTICES: u32 = 0x00a6_7410;
const GEOMETRY_DATA_UNLOCK: u32 = 0x00a6_73d0;
const GEOMETRY_DATA_MARK_AS_CHANGED: u32 = 0x00a6_7090;
const GEOMETRY_DATA_KEEP_FLAGS: u32 = 0x0045_7b80;
const GEOMETRY_HAS_DATA: u32 = 0x0049_ec60;
const ITERATOR_STRIDE_WORD: u32 = 0x0071_7e50;
const GEOMETRY_VERTEX_COUNT: u32 = 0x0045_6650;
/// `0.0` (`double`).
const ZERO: u32 = 0x0101_2060;
/// `BSFaceGenManager::GetDefaultBaseModulationTexture`; the iterator's constructor step
/// (`0065db40`, what `0065cf90` calls).
const DEFAULT_MODULATION_TEXTURE: u32 = 0x0060_45f0;
const ITERATOR_START: u32 = 0x0065_db40;
/// The coordinates' texture morph count (`009611e0`), the "texture coordinates" message and its
/// printer, `fr2Assert` and the model's source file name.
const COORD_TEXTURE_COUNT: u32 = 0x0096_11e0;
const TEXTURE_COUNT_MESSAGE: u32 = 0x0106_5ba0;
const PRINT_MESSAGE: u32 = 0x005b_5e40;
const FR2_ASSERT: u32 = 0x00aa_fb90;
const FACE_GEN_MODEL_SOURCE_FILE: u32 = 0x0106_5b00;
/// The image (0x2c bytes): constructor, width and height setters, colour vector, its resize, the
/// vector's begin iterator and the iterator's element; `GetCompressedImage` and its arguments.
const IMAGE_CONSTRUCT: u32 = 0x0064_ccf0;
const IMAGE_SET_WIDTH: u32 = 0x0070_37c0;
const IMAGE_SET_HEIGHT: u32 = 0x0061_cc40;
const IMAGE_COLORS: u32 = 0x0064_d880;
const IMAGE_RESIZE: u32 = 0x0064_db50;
const VECTOR_BEGIN: u32 = 0x0044_a170;
const ITERATOR_ELEMENT: u32 = 0x0064_dbd0;
const GET_COMPRESSED_IMAGE: u32 = 0x0064_ceb0;
const IMAGE_MIN: u32 = 0x0106_5b98;
const IMAGE_MAX: u32 = 0x0102_3cd8;
const IMAGE_SCALE: u32 = 0x0101_6248;
const IMAGE_SIZE_ARGUMENT: u32 = 0x011a_a2a0;
/// The colour scale (`float`) and the coefficient scale (`double`).
const COLOR_SCALE: u32 = 0x0106_5b9c;
const COEFFICIENT_SCALE: u32 = 0x0102_31d8;

/// The texel sums go through host memory up to this many texels (a face's is 256 by 256); a
/// larger image (never a head's) takes the game's own buffer texel by texel, as the exe does.
const HOST_TEXELS: u32 = 1 << 22;

/// `_invalid_parameter_noinfo` when `ok` is false, as a checked getter or step calls it.
#[inline]
fn check(e: &mut Engine, ok: bool) {
    if !ok {
        e.call(INVALID_PARAMETER, &args![]);
    }
}

/// `NiPointer::operator T*` on the pointer at `slot` (the leaf `00559450`).
fn pointer_get(e: &mut Engine, slot: u32) -> u32 {
    typed_call!(leaf::FAMILY, 0x0055_9450, leaf::word_at_0(e, slot))
}

/// `_ftol2` (`00ec62c0`) of `value`, its low word: with SSE2 (`__sse2_available`) `CVTTSD2SI`,
/// the value truncated or 0x80000000 for a NaN or beyond the `int` range; without it the x87 body,
/// the value truncated to 64 bits, whose low word is 0 for a NaN or beyond 2^63 (the integer
/// indefinite). The typed `crt::math::ftol2_sse` and `ftol2_x87` answer the same.
#[inline]
fn ftol(e: &Engine, value: f64) -> i32 {
    ftol_body(e.mem.u32(SSE2_AVAILABLE) != 0, value)
}

/// [`ftol`] with the flag already read: `sse2` when `__sse2_available` is set.
#[inline]
fn ftol_body(sse2: bool, value: f64) -> i32 {
    if sse2 {
        fistp_chop(value)
    } else {
        // The 64-bit store's indefinite (a NaN, or 2^63 and beyond) has a low word of 0.
        let [a, b, c, d, ..] = fistp_chop64(value).to_le_bytes();
        i32::from_le_bytes([a, b, c, d])
    }
}

/// A `std::vector`'s element count as these getters compute it: `(last - first) / size`, the
/// difference signed (`+0xc` first, `+0x10` last).
#[inline]
fn vector_count(e: &Engine, vector: u32, size: i32) -> u32 {
    let difference = e
        .mem
        .u32(vector.wrapping_add(0x10))
        .wrapping_sub(e.mem.u32(vector.wrapping_add(0xc))) as i32;
    (difference / size) as u32
}

/// A checked `std::vector` element (`0065cec0` with 0x58, `00657c20` with 0x1c): past the count
/// the invalid-parameter call, then the address all the same.
fn vector_element(e: &mut Engine, vector: u32, index: u32, size: u32) -> u32 {
    let count = vector_count(e, vector, size as i32);
    check(e, index < count);
    index
        .wrapping_mul(size)
        .wrapping_add(e.mem.u32(vector.wrapping_add(0xc)))
}

/// The coefficient's address for morph `index` of a coordinate set (`0060b370` and `00629ab0`
/// with 0): the set's float vector checked for a first element (`006578b0` with 0), then row
/// `index` of `+0x1c` floats each.
fn coefficient_at(e: &mut Engine, set: u32, index: u32) -> u32 {
    let floats = (e
        .mem
        .u32(set.wrapping_add(0x10))
        .wrapping_sub(e.mem.u32(set.wrapping_add(0xc))) as i32)
        >> 2;
    check(e, floats as u32 != 0);
    let first = e.mem.u32(set.wrapping_add(0xc));
    let row = e.mem.u32(set.wrapping_add(0x1c));
    first.wrapping_add(index.wrapping_mul(row).wrapping_mul(4))
}

// ---------------------------------------------------------------------------------------------
// The EGM morphs (0065a970)
// ---------------------------------------------------------------------------------------------

/// A checked iterator's two tests (`0064dda0`'s and the steps'): it has a proxy (its first word;
/// the vector's own first word, which the iterator adopted), and its position is before the last
/// element of the proxy's container (the proxy's first word; its `+0x10`). Each failure calls the
/// invalid-parameter function and goes on.
fn check_iterator(e: &mut Engine, proxy: u32, at: u32) {
    check(e, proxy != 0);
    let owner = if proxy == 0 { 0 } else { e.mem.u32(proxy) };
    let last = e.mem.u32(owner.wrapping_add(0x10));
    check(e, at < last);
}

/// The checked iterator over a morph's vertex offsets (three `i16`s, six bytes each): `{proxy,
/// position}` as the vector's begin (`0044a170`) makes it; read (`0064dda0`) and stepped
/// (`0065dae0`) with [`check_iterator`]'s tests.
struct Offsets {
    proxy: u32,
    at: u32,
}

impl Offsets {
    /// The vector's begin (`0044a170`, `0065da80`): the vector, its first element in range, the
    /// vector's proxy adopted.
    fn begin(e: &mut Engine, vector: u32) -> Offsets {
        let first = e.mem.u32(vector.wrapping_add(0xc));
        let last = e.mem.u32(vector.wrapping_add(0x10));
        check(e, vector != 0 && first <= last);
        Offsets {
            proxy: e.mem.u32(vector),
            at: first,
        }
    }

    /// The three offsets at the position (the read's checks first).
    fn read(&self, e: &mut Engine) -> [i16; 3] {
        check_iterator(e, self.proxy, self.at);
        [
            e.mem.i16(self.at),
            e.mem.i16(self.at.wrapping_add(2)),
            e.mem.i16(self.at.wrapping_add(4)),
        ]
    }

    /// One component, read through its own checked read (the tail loops read each axis so).
    fn read_axis(&self, e: &mut Engine, axis: u32) -> i16 {
        check_iterator(e, self.proxy, self.at);
        e.mem.i16(self.at.wrapping_add(2 * axis))
    }

    fn step(&mut self, e: &mut Engine) {
        check_iterator(e, self.proxy, self.at);
        self.at = self.at.wrapping_add(6);
    }
}

/// One offset component times the weight added to a coordinate: `FILD` of the `i16`, `FMUL` of
/// the weight, `FADD` of the coordinate, `FSTP`.
#[inline]
fn morphed(offset: i16, weight: f64, coordinate: u32) -> u32 {
    x87_store32(x87_add(
        x87_mul(f64::from(offset), weight),
        x87_m32(coordinate),
    ))
}

/// How many coordinates [`morph_words`] takes at a time: its plain steps go into a block of this
/// many on the stack, kept when every one of them was exact.
const MORPH_BLOCK: usize = 64;

/// [`morphed`] for every word of `words`, with the offset of the same index in `offsets` (the
/// `i16`s as little-endian byte pairs). The x87 helpers read the control word twice a coordinate;
/// here it is read once, and when it rounds to nearest and the weight is finite the steps are
/// plain ones that give the same bits ([`single_step`] at 24-bit precision, main's;
/// [`double_step`] at 53-bit, the loading threads'). A coordinate whose plain step is not exact
/// takes [`morphed`]: a block of [`MORPH_BLOCK`] is kept only when all its steps were exact.
/// Answers how many coordinates took [`morphed`].
fn morph_words(words: &mut [u32], offsets: &[u8], weight: f64) -> usize {
    let control = engine_core::x87::control_word();
    if control & 0x0c00 != 0 || !weight.is_finite() {
        let mut helped = 0;
        for (word, offset) in words.iter_mut().zip(offsets.chunks_exact(2)) {
            *word = morphed(i16::from_le_bytes([offset[0], offset[1]]), weight, *word);
            helped += 1;
        }
        return helped;
    }
    if control & 0x0300 == 0 {
        // Exact: a finite weight is a float widened.
        let single = f32::from_bits(x87_store32(weight));
        morph_blocks(words, offsets, weight, |offset, word| {
            single_step(offset, single, word)
        })
    } else {
        morph_blocks(words, offsets, weight, |offset, word| {
            double_step(offset, weight, word)
        })
    }
}

/// [`morph_words`]' blocks with one plain step: each block's steps into a block on the stack, kept
/// when all were exact; else each coordinate whose step was not exact takes [`morphed`] (answered:
/// how many did).
#[inline(always)]
fn morph_blocks(
    words: &mut [u32],
    offsets: &[u8],
    weight: f64,
    step: impl Fn(i16, u32) -> (u32, bool),
) -> usize {
    let mut helped = 0;
    let offset_at = |pair: &[u8]| i16::from_le_bytes([pair[0], pair[1]]);
    for (words, offsets) in words
        .chunks_mut(MORPH_BLOCK)
        .zip(offsets.chunks(2 * MORPH_BLOCK))
    {
        let mut out = [0u32; MORPH_BLOCK];
        let mut exact = true;
        for ((out, &word), pair) in out
            .iter_mut()
            .zip(words.iter())
            .zip(offsets.chunks_exact(2))
        {
            let (value, ok) = step(offset_at(pair), word);
            *out = value;
            exact &= ok;
        }
        if exact {
            let n = words.len().min(offsets.len() / 2);
            words[..n].copy_from_slice(&out[..n]);
            continue;
        }
        for ((word, &value), pair) in words
            .iter_mut()
            .zip(out.iter())
            .zip(offsets.chunks_exact(2))
        {
            let offset = offset_at(pair);
            *word = if step(offset, *word).1 {
                value
            } else {
                helped += 1;
                morphed(offset, weight, *word)
            };
        }
    }
    helped
}

/// Whether a float step's result is one the x87 gives too at 24-bit precision: zero, or a normal
/// float above the smallest (a float rounded up to the smallest normal may have been below it,
/// where the x87, whose exponent goes further, keeps 24 bits a float can't), not infinite or a NaN.
#[inline(always)]
fn normal_or_zero(value: f32) -> bool {
    let size = value.abs();
    (value == 0.0) | ((size > f32::MIN_POSITIVE) & (size <= f32::MAX))
}

/// [`morphed`] at 24-bit precision, rounding to nearest, with a finite float weight: `FILD` of the
/// `i16` is exact, `FMUL` rounds the exact product to 24 bits and `FADD` the exact sum, which is
/// what a float multiply and add do whenever the result is a zero or a normal float (then `FSTP`
/// keeps it). With the answer, whether that held for the product and the sum (the coordinate not
/// infinite or a NaN either): else the x87's wider exponent or its NaN choice decides.
#[inline(always)]
fn single_step(offset: i16, weight: f32, word: u32) -> (u32, bool) {
    let coordinate = f32::from_bits(word);
    let product = f32::from(offset) * weight;
    let sum = product + coordinate;
    let ok = coordinate.is_finite() & normal_or_zero(product) & normal_or_zero(sum);
    (sum.to_bits(), ok)
}

/// [`morphed`] at 53-bit precision, rounding to nearest, with a finite weight: the product of an
/// `i16` and a float is exact in an `f64` (40 bits at most), `FADD` is the `f64` sum and `FSTP` its
/// rounding to a float, which are the helpers' own steps when no operand is a NaN (the weight is
/// finite; the coordinate is tested).
#[inline(always)]
fn double_step(offset: i16, weight: f64, word: u32) -> (u32, bool) {
    let coordinate = f32::from_bits(word);
    let sum = f64::from(offset) * weight + f64::from(coordinate);
    (x87_store32(sum), !coordinate.is_nan())
}

// Typed from 0065a970
/// Unnamed in the engine map (`thiscall` on a `BSFaceGenModel`, three words: the face generation
/// coordinates (the default ones from `GetFaceGenCoord` when 0), the geometry, the apply-all
/// byte): the EGM morphs moved into the geometry's vertices. 0 without mesh data (+8) or
/// coordinates. Under the face generation lock: the EGM data is loaded from its file when not yet
/// (0 without a file name or data); the geometry's vertices are locked into a strided iterator (0
/// without them); the vertex count is lowered to the smallest morph's. For both morph sets, each
/// morph whose weight (coefficient times the morph's scale, stored as a float) is not 0, or every
/// morph with the byte set: each vertex (four at a time, then the rest) moves by the morph's `i16`
/// offsets times the weight, read from and written back to the additional vertices (when the
/// geometry has morph extra data) or the geometry; past the vertex count the additional vertices
/// up to their count move too. Then the data is marked as changed unless its keep flags are
/// 0x4000 or the geometry's `0049ec60` is 0, the vertices are unlocked, and the answer is 1.
pub fn apply_coordinates(
    e: &mut Engine,
    this: Ptr,
    coordinates: u32,
    geometry: u32,
    apply_all: u8,
) -> bool {
    let model = this.addr();
    let mesh = |e: &Engine| e.mem.u32(model.wrapping_add(8));
    if mesh(e) == 0 {
        return false;
    }
    let coords = if coordinates == 0 {
        e.call(GET_FACE_GEN_COORD, &args![]).u32()
    } else {
        coordinates
    };
    if coords == 0 {
        return false;
    }
    e.call(FACE_GEN_LOCK, &args![]);
    if e.call(MESH_LOADED_TEST, &args![this]).u8() == 0 {
        if e.mem.u32(mesh(e).wrapping_add(8)) == 0 {
            let data = mesh(e);
            if e.call(STRING_HAS_TEXT, &args![data]).u32() == 0 {
                e.call(FACE_GEN_UNLOCK, &args![]);
                return false;
            }
            let block = e.call(OPERATOR_NEW, &args![0x34u32]).u32();
            let egm = if block != 0 {
                let data = mesh(e);
                let name = pointer_get(e, data);
                e.call(EGM_DATA_CONSTRUCT, &args![block, name]).u32()
            } else {
                0
            };
            let data = mesh(e);
            e.mem.set_u32(data.wrapping_add(8), egm);
        }
        if e.mem.u32(mesh(e).wrapping_add(8)) == 0 {
            e.call(FACE_GEN_UNLOCK, &args![]);
            return false;
        }
    }
    let extra_data = e.call(MORPH_EXTRA_DATA, &args![geometry]).u32();
    let extra = if extra_data != 0 {
        typed_call!(leaf::FAMILY, 0x0084_e3a0, leaf::word_at_c(e, extra_data))
    } else {
        0
    };
    // The translation keeps its vertex iterator in a heap block of 0x10 bytes.
    let locals = e.mem.alloc(0x10);
    let iterator = locals;
    e.call(VERTEX_ITERATOR_CONSTRUCT, &args![iterator]);
    let mut vertices = 0u32;
    if geometry != 0 {
        let data = e.call(GEOMETRY_DATA, &args![geometry]).u32();
        if data != 0 && e.call(GEOMETRY_DATA_LOCK, &args![data, 1u32]).u8() != 0 {
            e.call(GEOMETRY_DATA_VERTICES, &args![data, iterator]);
            // The word the iterator answers is kept in a local nothing reads.
            let word = e.call(ITERATOR_STRIDE_WORD, &args![iterator]).u32();
            let _stride = e.mem.u32(word);
            vertices = pointer_get(e, iterator);
        }
    }
    if vertices == 0 {
        e.call(FACE_GEN_UNLOCK, &args![]);
        e.mem.free(locals);
        return false;
    }
    let egm_set = |e: &Engine, set: u32| {
        e.mem
            .u32(mesh(e).wrapping_add(8))
            .wrapping_add(set * 0x18)
            .wrapping_add(4)
    };
    let mut count = u32::from(e.call(GEOMETRY_VERTEX_COUNT, &args![geometry]).u16());
    for set in 0..2u32 {
        let address = egm_set(e, set);
        let morphs = vector_count(e, address, 0x1c);
        for index in 0..morphs {
            let address = egm_set(e, set);
            let morph = vector_element(e, address, index, 0x1c);
            let offsets = vector_count(e, morph.wrapping_add(4), 6);
            if offsets < count {
                count = offsets;
            }
        }
    }
    let zero = e.mem.f64(ZERO);
    for set in 0..2u32 {
        let address = egm_set(e, set);
        let morphs = vector_count(e, address, 0x1c);
        for index in 0..morphs {
            let address = egm_set(e, set);
            let coefficient = coefficient_at(e, coords.wrapping_add(set << 5), index);
            let morph = vector_element(e, address, index, 0x1c);
            let weight = x87_store32(x87_mul(
                x87_m32(e.mem.u32(coefficient)),
                x87_m32(e.mem.u32(morph)),
            ));
            // FCOMP with 0.0, TEST AH,0x44, JP: applied unless equal (NaN applies).
            if x87_m32(weight) == zero && apply_all == 0 {
                continue;
            }
            let address = egm_set(e, set);
            let morph = vector_element(e, address, index, 0x1c);
            apply_morph(e, morph, weight, iterator, extra, extra_data, count);
        }
    }
    if e.call(GEOMETRY_HAS_DATA, &args![geometry]).u32() != 0 {
        let data = e.call(GEOMETRY_DATA, &args![geometry]).u32();
        if e.call(GEOMETRY_DATA_KEEP_FLAGS, &args![data]).u32() != 0x4000 {
            let data = e.call(GEOMETRY_DATA, &args![geometry]).u32();
            e.call(GEOMETRY_DATA_MARK_AS_CHANGED, &args![data, 1u32]);
        }
    }
    let data = e.call(GEOMETRY_DATA, &args![geometry]).u32();
    e.call(GEOMETRY_DATA_UNLOCK, &args![data]);
    e.call(FACE_GEN_UNLOCK, &args![]);
    e.mem.free(locals);
    true
}

/// The words at `at`, in order.
fn read_words(e: &Engine, at: u32, words: &mut [u32]) {
    for (k, word) in (0u32..).zip(words.iter_mut()) {
        *word = e.mem.u32(at.wrapping_add(4 * k));
    }
}

/// `words` stored at `at`, in order.
fn write_words(e: &Engine, at: u32, words: &[u32]) {
    for (k, &word) in (0u32..).zip(words.iter()) {
        e.mem.set_u32(at.wrapping_add(4 * k), word);
    }
}

/// The vertex at `index` of the strided iterator `{base, stride}` (the iterator's read
/// `00534570` and write `0065b390` address it so).
#[inline]
fn vertex_at(base: u32, stride: u32, index: u32) -> u32 {
    index.wrapping_mul(stride).wrapping_add(base)
}

/// One morph of [`apply_coordinates`]. The translation keeps its offsets iterator (8 bytes) and its
/// four points (0x40 bytes) in heap blocks made and freed per morph; they are made, filled as the
/// translation leaves them, and freed in the same order here, so the heap and its bytes match.
fn apply_morph(
    e: &mut Engine,
    morph: u32,
    weight: u32,
    iterator: u32,
    first_extra: u32,
    extra_data: u32,
    count: u32,
) {
    let weight = x87_m32(weight);
    let base = e.mem.u32(iterator);
    let stride = e.mem.u32(iterator.wrapping_add(4));
    let offsets_block = e.mem.alloc(8);
    let mut offsets = Offsets::begin(e, morph.wrapping_add(4));
    let points_block = e.mem.alloc(0x40);
    let layout = MorphLayout {
        base,
        stride,
        first_extra,
        extra_data,
        count,
    };
    match morph_in_host(e, &offsets, weight, &layout, points_block) {
        Some(end) => offsets.at = end,
        None => morph_by_element(e, &mut offsets, weight, &layout, points_block),
    }
    e.mem.set_u32(offsets_block, offsets.proxy);
    e.mem.set_u32(offsets_block.wrapping_add(4), offsets.at);
    e.mem.free(points_block);
    e.mem.free(offsets_block);
}

/// Where one morph of [`apply_coordinates`] reads and writes: the strided vertex array
/// (`{base, stride}`), the additional vertices' array and its data (0 without), the vertex count.
struct MorphLayout {
    base: u32,
    stride: u32,
    first_extra: u32,
    extra_data: u32,
    count: u32,
}

/// Whether `[a, a + a_len)` and `[b, b + b_len)` share a byte (64-bit ends: no wrapping).
fn overlaps(a: u64, a_len: u64, b: u64, b_len: u64) -> bool {
    a_len != 0 && b_len != 0 && a < b + b_len && b < a + a_len
}

/// One morph of [`apply_coordinates`] in one pass: the offsets read at once, every point morphed
/// in host memory and written back, the points block left as [`morph_by_element`] leaves it.
/// Answers the offsets iterator's end position, or `None` when a step could take a check's
/// failing side (no proxy, a position at or past the proxy's last element) or when what the loops
/// write could change what they read later (overlapping arrays, a stride below a point, an address
/// past 4 GB): then the element loops run, as the exe's do. Before a `None` nothing is written;
/// at most the words the element loops read anyway are read (the additional vertices' count, the
/// proxy's container and its end word).
fn morph_in_host(
    e: &mut Engine,
    offsets: &Offsets,
    weight: f64,
    layout: &MorphLayout,
    points_block: u32,
) -> Option<u32> {
    let MorphLayout {
        base,
        stride,
        first_extra,
        extra_data,
        count,
    } = *layout;
    if offsets.proxy == 0 || stride < 12 {
        return None;
    }
    // The additional vertices past the count: the extra loop's steps (it reads their total after
    // the vertex loops; nothing they write may change it).
    let total = if first_extra != 0 {
        e.mem.u32(extra_data.wrapping_add(0x10))
    } else {
        0
    };
    let steps = u64::from(count) + u64::from(total.saturating_sub(count));
    if steps == 0 {
        // No point and no step: the element loops read nothing either.
        return None;
    }
    let owner = e.mem.u32(offsets.proxy);
    let end_word = owner.wrapping_add(0x10);
    let last = e.mem.u32(end_word);
    let first = u64::from(offsets.at);
    // Every check is at `first + 6 k` for the steps `k` and passes while it is below `last`.
    if first + 6 * (steps - 1) >= u64::from(last) {
        return None;
    }
    let extras = u64::from(count.max(total));
    let vertex_span = if count == 0 {
        0
    } else {
        u64::from(stride) * (u64::from(count) - 1) + 12
    };
    let extra_span = if first_extra != 0 { 12 * extras } else { 0 };
    let limit = 1u64 << 32;
    if u64::from(base) + vertex_span > limit
        || u64::from(first_extra) + extra_span > limit
        || first + 6 * steps > limit
    {
        return None;
    }
    let (b, x) = (u64::from(base), u64::from(first_extra));
    let read_only = [
        (first, 6 * steps),
        (u64::from(offsets.proxy), 4),
        (u64::from(end_word), 4),
        (
            u64::from(extra_data.wrapping_add(0x10)),
            if first_extra != 0 { 4 } else { 0 },
        ),
    ];
    if overlaps(b, vertex_span, x, extra_span)
        || read_only
            .iter()
            .any(|&(at, len)| overlaps(b, vertex_span, at, len) || overlaps(x, extra_span, at, len))
    {
        return None;
    }
    // Both below 4 GB (tested above).
    let delta_bytes = u32::try_from(6 * steps).ok()?;
    let extra_bytes = u32::try_from(12 * extras).ok()?;
    let mut deltas = vec![0u8; delta_bytes as usize];
    e.mem.read(offsets.at, &mut deltas);
    let n = count as usize;
    // The points the vertex loops read: the additional vertices' first `count` when there are
    // any, else the vertex array's.
    let mut points = vec![0u32; 3 * n];
    let mut bytes = vec![0u8; 12];
    if first_extra != 0 {
        let mut all = vec![0u8; extra_bytes as usize];
        e.mem.read(first_extra, &mut all);
        let mut tail: Vec<u32> = all
            .chunks_exact(4)
            .map(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]]))
            .collect();
        morph_words(&mut tail, &deltas, weight);
        points.copy_from_slice(&tail[..3 * n]);
        let mut out = vec![0u8; tail.len() * 4];
        for (chunk, word) in out.chunks_exact_mut(4).zip(&tail) {
            chunk.copy_from_slice(&word.to_le_bytes());
        }
        write_points(e, base, stride, &points, &mut bytes);
        e.mem.write(first_extra, &out);
    } else {
        if stride == 12 {
            let mut all = vec![0u8; 12 * n];
            e.mem.read(base, &mut all);
            for (word, w) in points.iter_mut().zip(all.chunks_exact(4)) {
                *word = u32::from_le_bytes([w[0], w[1], w[2], w[3]]);
            }
        } else {
            for (v, point) in (0u32..).zip(points.chunks_exact_mut(3)) {
                e.mem.read(vertex_at(base, stride, v), &mut bytes);
                for (word, w) in point.iter_mut().zip(bytes.chunks_exact(4)) {
                    *word = u32::from_le_bytes([w[0], w[1], w[2], w[3]]);
                }
            }
        }
        morph_words(&mut points, &deltas, weight);
        write_points(e, base, stride, &points, &mut bytes);
    }
    // The points block as the element loops leave it: the last group of four, then the last
    // single point over its first three words.
    let groups = n / 4;
    if groups != 0 {
        write_words(e, points_block, &points[12 * (groups - 1)..12 * groups]);
    }
    if n % 4 != 0 {
        write_words(e, points_block, &points[3 * (n - 1)..3 * n]);
    }
    Some(offsets.at.wrapping_add(delta_bytes))
}

/// The morphed points written to the strided vertex array: in one write when the points are packed
/// (a stride of 12), else a point (12 bytes) at a time.
fn write_points(e: &Engine, base: u32, stride: u32, points: &[u32], bytes: &mut [u8]) {
    if stride == 12 {
        let mut all = vec![0u8; 4 * points.len()];
        for (chunk, word) in all.chunks_exact_mut(4).zip(points) {
            chunk.copy_from_slice(&word.to_le_bytes());
        }
        e.mem.write(base, &all);
        return;
    }
    for (v, point) in (0u32..).zip(points.chunks_exact(3)) {
        for (chunk, word) in bytes.chunks_exact_mut(4).zip(point) {
            chunk.copy_from_slice(&word.to_le_bytes());
        }
        e.mem.write(vertex_at(base, stride, v), bytes);
    }
}

/// One morph of [`apply_coordinates`] as the exe's loops go: four points at a time, then one at a
/// time, then the additional vertices, every offset read and step through the checked iterator.
fn morph_by_element(
    e: &mut Engine,
    offsets: &mut Offsets,
    weight: f64,
    layout: &MorphLayout,
    points_block: u32,
) {
    let MorphLayout {
        base,
        stride,
        first_extra,
        extra_data,
        count,
    } = *layout;
    let mut extra = first_extra;
    // What the points block holds at the end: the last group of four, then the last single one
    // over its first three words.
    let mut last_four: Option<[u32; 12]> = None;
    let mut last_one: Option<[u32; 3]> = None;
    let mut vertex = 0u32;
    while vertex.wrapping_add(3) < count {
        // Four points, three words each, in the order the exe keeps them.
        let mut points = [0u32; 12];
        if extra != 0 {
            read_words(e, extra, &mut points);
        } else {
            for (point, words) in (0u32..).zip(points.chunks_exact_mut(3)) {
                read_words(
                    e,
                    vertex_at(base, stride, vertex.wrapping_add(point)),
                    words,
                );
            }
        }
        let mut deltas = [[0i16; 3]; 4];
        for delta in deltas.iter_mut() {
            *delta = offsets.read(e);
            offsets.step(e);
        }
        for (words, delta) in points.chunks_exact_mut(3).zip(deltas.iter()) {
            for (word, &offset) in words.iter_mut().zip(delta.iter()) {
                *word = morphed(offset, weight, *word);
            }
        }
        for (point, words) in (0u32..).zip(points.chunks_exact(3)) {
            write_words(
                e,
                vertex_at(base, stride, vertex.wrapping_add(point)),
                words,
            );
        }
        if extra != 0 {
            write_words(e, extra, &points);
            extra = extra.wrapping_add(0x30);
        }
        last_four = Some(points);
        vertex = vertex.wrapping_add(4);
    }
    while vertex < count {
        let mut point = [0u32; 3];
        if extra != 0 {
            read_words(e, extra, &mut point);
        } else {
            read_words(e, vertex_at(base, stride, vertex), &mut point);
        }
        for (axis, word) in (0u32..).zip(point.iter_mut()) {
            let offset = offsets.read_axis(e, axis);
            *word = morphed(offset, weight, *word);
        }
        write_words(e, vertex_at(base, stride, vertex), &point);
        if extra != 0 {
            write_words(e, extra, &point);
            extra = extra.wrapping_add(0xc);
        }
        last_one = Some(point);
        vertex = vertex.wrapping_add(1);
        offsets.step(e);
    }
    if extra != 0 {
        // The additional vertices' count (`0065b3e0` with 0: the extra data's +0x10).
        let total = e.mem.u32(extra_data.wrapping_add(0x10));
        let mut vertex = count;
        while vertex < total {
            for axis in 0..3u32 {
                let offset = offsets.read_axis(e, axis);
                let address = extra.wrapping_add(4 * axis);
                let value = morphed(offset, weight, e.mem.u32(address));
                e.mem.set_u32(address, value);
            }
            extra = extra.wrapping_add(0xc);
            vertex = vertex.wrapping_add(1);
            offsets.step(e);
        }
    }
    if let Some(points) = last_four {
        write_words(e, points_block, &points);
    }
    if let Some(point) = last_one {
        write_words(e, points_block, &point);
    }
}

// ---------------------------------------------------------------------------------------------
// The face tint (0065b410)
// ---------------------------------------------------------------------------------------------

/// The EGT data's morph set `set` (`+4 + 0x18 * set` of the data at the texture data's +8).
fn egt_set(e: &Engine, model: u32, set: u32) -> u32 {
    let texture_data = e.mem.u32(model.wrapping_add(0xc));
    e.mem
        .u32(texture_data.wrapping_add(8))
        .wrapping_add(set * 0x18)
        .wrapping_add(4)
}

// Typed from 0065b410
/// `BSFaceGenModel::ApplyCoordinateTexturingToMesh` (engine map; `thiscall`, three words: the
/// face generation coordinates (the default ones when 0), the `NiPointer` that receives the
/// texture, the morph limit (0 for all)). The pointer first takes the default base modulation
/// texture; 0 without texture data or coordinates. Under the face generation lock the EGT data is
/// loaded from its file when not yet (0 without a name or data); 0 when its first morph has no
/// width or height, or when the coordinates have fewer texture morphs than either EGT set (with a
/// message). An image (0x2c bytes) of the first morph's width and height gets its colours (zero)
/// resized; an `i32` sum per texel channel adds, for the first set's morphs up to the limit, each
/// morph's signed byte deltas times `ftol(scale * 256) * ftol(coefficient * 256)` (`fr2Assert`
/// when a morph's size differs); each colour channel becomes the sum times `01065b9c`; the sums'
/// buffer is freed and the pointer takes `GetCompressedImage` of the image. Answers 1.
pub fn apply_coordinate_texturing(
    e: &mut Engine,
    this: Ptr,
    coordinates: u32,
    texture: u32,
    limit: u32,
) -> bool {
    let model = this.addr();
    // The translation's locals live in a heap block of 0x20 bytes: the image pointer, the
    // iterator, four zero floats.
    let locals = e.mem.alloc(0x20);
    let image_pointer = locals;
    let iterator = locals + 4;
    e.call(NI_POINTER_FROM_RAW, &args![image_pointer, 0u32]);
    e.call(ITERATOR_START, &args![iterator]);
    let default = e.call(DEFAULT_MODULATION_TEXTURE, &args![]).u32();
    e.call(NI_POINTER_ASSIGN, &args![texture, default]);
    let done = |e: &mut Engine, answer: bool| {
        e.call(NI_POINTER_DESTRUCT, &args![image_pointer]);
        e.mem.free(locals);
        answer
    };
    let done_unlocked = |e: &mut Engine| {
        e.call(FACE_GEN_UNLOCK, &args![]);
        e.call(NI_POINTER_DESTRUCT, &args![image_pointer]);
        e.mem.free(locals);
        false
    };
    let data = |e: &Engine| e.mem.u32(model.wrapping_add(0xc));
    if data(e) == 0 {
        return done(e, false);
    }
    let coords = if coordinates == 0 {
        e.call(GET_FACE_GEN_COORD, &args![]).u32()
    } else {
        coordinates
    };
    if coords == 0 {
        return done(e, false);
    }
    e.call(FACE_GEN_LOCK, &args![]);
    if e.call(TEXTURE_LOADED_TEST, &args![this]).u8() == 0 {
        if e.mem.u32(data(e).wrapping_add(8)) == 0 {
            let texture_data = data(e);
            if e.call(STRING_HAS_TEXT, &args![texture_data]).u32() == 0 {
                return done_unlocked(e);
            }
            let texture_data = data(e);
            let name = pointer_get(e, texture_data);
            let egt = e.call(EGT_DATA_LOAD, &args![name]).u32();
            let texture_data = data(e);
            e.mem.set_u32(texture_data.wrapping_add(8), egt);
        }
        if e.mem.u32(data(e).wrapping_add(8)) == 0 {
            return done_unlocked(e);
        }
    }
    if e.mem.u32(data(e).wrapping_add(8)) == 0 {
        return done_unlocked(e);
    }
    let (mut width, mut height) = (0u32, 0u32);
    let set0 = egt_set(e, model, 0);
    if vector_count(e, set0, 0x58) != 0 {
        let first = vector_element(e, set0, 0, 0x58);
        width = e.mem.u32(first.wrapping_add(4));
        let first = vector_element(e, egt_set(e, model, 0), 0, 0x58);
        height = e.mem.u32(first.wrapping_add(8));
    }
    if width == 0 || height == 0 {
        return done_unlocked(e);
    }
    for set in 0..2u32 {
        let available = e
            .call(
                COORD_TEXTURE_COUNT,
                &args![coords.wrapping_add(0x40 + (set << 5))],
            )
            .u32();
        let address = egt_set(e, model, set);
        if available < vector_count(e, address, 0x58) {
            e.call(PRINT_MESSAGE, &args![TEXTURE_COUNT_MESSAGE]);
            return done_unlocked(e);
        }
    }
    let zero = locals + 0xc;
    for word in 0..4 {
        e.mem.set_f32(zero + 4 * word, 0.0);
    }
    let block = e.call(NI_ALLOCATE, &args![0x2cu32]).u32();
    let image = if block != 0 {
        e.call(IMAGE_CONSTRUCT, &args![block, 0u32]).u32()
    } else {
        0
    };
    e.call(NI_POINTER_ASSIGN, &args![image_pointer, image]);
    let image = pointer_get(e, image_pointer);
    e.call(IMAGE_SET_WIDTH, &args![image, width]);
    let image = pointer_get(e, image_pointer);
    e.call(IMAGE_SET_HEIGHT, &args![image, height]);
    let image = pointer_get(e, image_pointer);
    let colors = e.call(IMAGE_COLORS, &args![image]).u32();
    e.call(
        IMAGE_RESIZE,
        &args![colors, width.wrapping_mul(height), zero],
    );
    let texels = width.wrapping_mul(height);
    let size = texels.wrapping_mul(3).saturating_mul(4);
    let sums = e.call(OPERATOR_NEW, &args![size]).u32();
    let cleared = (width << 2).wrapping_mul(height).wrapping_mul(3);
    e.call(MEMSET, &args![sums, 0u32, cleared]);

    // Only the first EGT set is summed.
    let set = 0u32;
    let mut morphs = vector_count(e, egt_set(e, model, set), 0x58);
    if limit != 0 {
        morphs = limit;
        let available = vector_count(e, egt_set(e, model, set), 0x58);
        if morphs > available {
            morphs = available;
        }
    }
    let scale = e.mem.f64(COEFFICIENT_SCALE);
    // A morph of another size takes `fr2Assert`, which has to see the sums so far: then (never a
    // head's EGT) the sums go texel by texel in place.
    let first_morph = e.mem.u32(egt_set(e, model, set).wrapping_add(0xc));
    let host = u64::from(texels) * 12 == u64::from(cleared)
        && texels <= HOST_TEXELS
        && (0..morphs).all(|index| {
            let morph = first_morph.wrapping_add(index.wrapping_mul(0x58));
            e.mem.u32(morph.wrapping_add(4)) == width && e.mem.u32(morph.wrapping_add(8)) == height
        });
    let planes = if host {
        Some(sum_in_host(
            e, model, coords, set, morphs, width, height, scale,
        ))
    } else {
        sum_in_place(e, model, coords, set, morphs, width, height, scale, sums);
        None
    };
    if let Some(planes) = &planes {
        // The sums as the exe's buffer holds them: per texel, the three channels.
        let n = texels as usize;
        let mut bytes = vec![0u8; n * 12];
        for texel in 0..n {
            for channel in 0..3 {
                let at = (texel * 3 + channel) * 4;
                bytes[at..at + 4].copy_from_slice(&planes[channel * n + texel].to_le_bytes());
            }
        }
        e.mem.write(sums, &bytes);
    }
    for channel in 0..3u32 {
        let factor = x87_m32(x87_quiet_bits(e.mem.u32(COLOR_SCALE)));
        let element = e.with_stack(8, |e, begin| {
            let first = e.call(VECTOR_BEGIN, &args![colors, begin.addr()]).u32();
            let words = [e.mem.u32(first), e.mem.u32(first + 4)];
            e.mem.set_u32(iterator, words[0]);
            e.mem.set_u32(iterator + 4, words[1]);
            e.call(ITERATOR_ELEMENT, &args![iterator]).u32()
        });
        for y in 0..height {
            for x in 0..width {
                let texel = y.wrapping_mul(width).wrapping_add(x);
                let sum =
                    match &planes {
                        Some(planes) => planes[channel as usize * texels as usize + texel as usize],
                        None => e.mem.i32(sums.wrapping_add(
                            texel.wrapping_mul(3).wrapping_add(channel).wrapping_mul(4),
                        )),
                    };
                let value = x87_store32(x87_mul(f64::from(sum), factor));
                let target = element
                    .wrapping_add(texel.wrapping_mul(4).wrapping_add(channel).wrapping_mul(4));
                e.mem.set_u32(target, value);
            }
        }
    }
    e.call(OPERATOR_DELETE, &args![sums]);
    let image = pointer_get(e, image_pointer);
    let x = x87_quiet_bits(e.mem.u32(IMAGE_MIN));
    let y = x87_quiet_bits(e.mem.u32(IMAGE_MAX));
    let z = x87_quiet_bits(e.mem.u32(IMAGE_SCALE));
    let compressed = e
        .call(
            GET_COMPRESSED_IMAGE,
            &args![image, x, y, z, IMAGE_SIZE_ARGUMENT],
        )
        .u32();
    e.call(NI_POINTER_ASSIGN, &args![texture, compressed]);
    e.call(FACE_GEN_UNLOCK, &args![]);
    done(e, true)
}

/// A morph's weight and its three channel factors, or `None` when the weight truncates to 0 (the
/// morph adds nothing): `ftol(coefficient * scale)`, then per channel `ftol(morph scale * scale)`
/// times it.
fn morph_factors(
    e: &mut Engine,
    coords: u32,
    set: u32,
    morph: u32,
    index: u32,
    scale: f64,
) -> Option<[i32; 3]> {
    let coefficient = coefficient_at(e, coords.wrapping_add(0x40 + (set << 5)), index);
    let weight = ftol(e, x87_mul(x87_m32(e.mem.u32(coefficient)), scale));
    if weight == 0 {
        return None;
    }
    let factor = ftol(e, x87_mul(x87_m32(e.mem.u32(morph)), scale)).wrapping_mul(weight);
    // The exe reads the morph's scale again for each channel; it does not change between them.
    Some([factor; 3])
}

/// A morph's channel data (`0065cc70` with 0 on `morph + 0x10 + 0x18 * channel`): its first byte,
/// the byte vector checked for one.
fn channel_data(e: &mut Engine, morph: u32, channel: u32) -> u32 {
    let vector = morph.wrapping_add(0x10 + 0x18 * channel);
    let bytes = e
        .mem
        .u32(vector.wrapping_add(0x10))
        .wrapping_sub(e.mem.u32(vector.wrapping_add(0xc)));
    check(e, bytes != 0);
    e.mem.u32(vector.wrapping_add(0xc))
}

/// The sums of [`apply_coordinate_texturing`] in host memory, one plane per channel, every morph
/// of the size of the first.
#[allow(clippy::too_many_arguments)]
fn sum_in_host(
    e: &mut Engine,
    model: u32,
    coords: u32,
    set: u32,
    morphs: u32,
    width: u32,
    height: u32,
    scale: f64,
) -> Vec<i32> {
    let n = width as usize * height as usize;
    let mut planes = vec![0i32; 3 * n];
    let mut row_bytes = vec![0u8; width as usize];
    for index in 0..morphs {
        let morph = vector_element(e, egt_set(e, model, set), index, 0x58);
        let Some(factors) = morph_factors(e, coords, set, morph, index, scale) else {
            continue;
        };
        let row_step = e
            .mem
            .u32(morph.wrapping_add(4))
            .wrapping_add(e.mem.u32(morph.wrapping_add(0xc)));
        for ((channel, &factor), plane) in
            (0u32..).zip(factors.iter()).zip(planes.chunks_exact_mut(n))
        {
            let mut row = channel_data(e, morph, channel);
            for line in plane.chunks_exact_mut(width as usize) {
                e.mem.read(row, &mut row_bytes);
                for (sum, &delta) in line.iter_mut().zip(&row_bytes) {
                    *sum = i32::from(delta as i8)
                        .wrapping_mul(factor)
                        .wrapping_add(*sum);
                }
                row = row.wrapping_add(row_step);
            }
        }
    }
    planes
}

/// The sums of [`apply_coordinate_texturing`] in the game's buffer, texel by texel as the exe adds
/// them (a morph of another size, or an image too large for host memory).
#[allow(clippy::too_many_arguments)]
fn sum_in_place(
    e: &mut Engine,
    model: u32,
    coords: u32,
    set: u32,
    morphs: u32,
    width: u32,
    height: u32,
    scale: f64,
    sums: u32,
) {
    for index in 0..morphs {
        let morph = vector_element(e, egt_set(e, model, set), index, 0x58);
        if e.mem.u32(morph.wrapping_add(4)) != width {
            e.call(FR2_ASSERT, &args![FACE_GEN_MODEL_SOURCE_FILE, 0x48au32]);
        }
        let morph = vector_element(e, egt_set(e, model, set), index, 0x58);
        if e.mem.u32(morph.wrapping_add(8)) != height {
            e.call(FR2_ASSERT, &args![FACE_GEN_MODEL_SOURCE_FILE, 0x48bu32]);
        }
        let Some(factors) = morph_factors(e, coords, set, morph, index, scale) else {
            continue;
        };
        let row_step = e
            .mem
            .u32(morph.wrapping_add(4))
            .wrapping_add(e.mem.u32(morph.wrapping_add(0xc)));
        for (channel, &factor) in (0u32..).zip(factors.iter()) {
            let mut row = channel_data(e, morph, channel);
            for y in 0..height {
                for x in 0..width {
                    let slot = y
                        .wrapping_mul(width)
                        .wrapping_add(x)
                        .wrapping_mul(3)
                        .wrapping_add(channel);
                    let delta = i32::from(e.mem.i8(row.wrapping_add(x)));
                    let address = sums.wrapping_add(slot.wrapping_mul(4));
                    let sum = delta.wrapping_mul(factor).wrapping_add(e.mem.i32(address));
                    e.mem.set_i32(address, sum);
                }
                row = row.wrapping_add(row_step);
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The compressed image (0064ceb0)
// ---------------------------------------------------------------------------------------------

/// The texture pointer at +8 of the image: assignment from a raw pointer, from another, its
/// constructor and destructor.
const TEXTURE_SET: u32 = 0x0066_b0d0;
const TEXTURE_ASSIGN: u32 = 0x006e_5cc0;
const TEXTURE_CONSTRUCT: u32 = 0x0063_3c90;
const TEXTURE_DESTRUCT: u32 = 0x0045_cec0;
/// The values container (+0xc): its emptiness test and begin iterator.
const VALUES_EMPTY: u32 = 0x0064_db00;
const VALUES_BEGIN: u32 = 0x0044_a170;
/// `NiDevImageConverter`: constructor, destructor, `GenerateMipmapLevels`.
const CONVERTER_CONSTRUCT: u32 = 0x00a7_b110;
const CONVERTER_DESTRUCT: u32 = 0x00a7_aff0;
const CONVERTER_MIPMAPS: u32 = 0x00a7_8090;
/// `NiPixelData::NiPixelData`, the three- and four-channel formats, the pixel bytes.
const PIXEL_DATA_CONSTRUCT: u32 = 0x00a7_c190;
const FORMAT_THREE_CHANNELS: u32 = 0x011a_a2e8;
const FORMAT_FOUR_CHANNELS: u32 = 0x011a_a2a0;
const PIXEL_BYTES: u32 = 0x0064_dac0;
/// The mipmap setting's test and its object; the debug texture flag and hook;
/// `NiSourceTexture::Create` with its preferences; the texture resize for the size argument.
const MIPMAP_SETTING: u32 = 0x0045_4af0;
const MIPMAP_SETTING_OBJECT: u32 = 0x011d_59ac;
const DEBUG_TEXTURE_FLAG: u32 = 0x011f_94b0;
const DEBUG_TEXTURE: u32 = 0x00b6_b940;
const SOURCE_TEXTURE_CREATE: u32 = 0x00a5_ff10;
const TEXTURE_PREFERENCES: u32 = 0x011a_9598;
const TEXTURE_RESIZE: u32 = 0x00b6_bbd0;

/// The float floor `00404040` of the float `value`: its `_ftol2` back as a float (through a
/// `double`), one less when the value minus that is below 0.0 or unordered; as `ST0` holds it.
/// `sse2` is `_ftol2`'s body ([`ftol_body`]).
#[inline]
fn float_floor(sse2: bool, value: f32) -> f64 {
    let mut truncated = f32::from_bits(x87_store32(f64::from(ftol_body(sse2, f64::from(value)))));
    let difference = f64::from(value) - f64::from(truncated);
    if !(difference >= 0.0) {
        truncated = f32::from_bits(x87_store32(f64::from(truncated) - 1.0));
    }
    f64::from(truncated)
}

/// One channel of a compressed pixel from the value's bits: clamped to `[min, max]` (below `min`
/// or unordered gives `min`; above `max` gives `max`), floored, less `min`, times `scale`,
/// truncated to an integer; its low byte. `sse2` is `_ftol2`'s body ([`ftol_body`]).
#[inline]
fn compress_channel(sse2: bool, bits: u32, min: f32, max: f32, scale: f32) -> u8 {
    let value = x87_quiet_bits(bits);
    let clamped = if !(x87_load32(min.to_bits()) <= x87_load32(value)) {
        x87_quiet_bits(min.to_bits())
    } else if !(x87_load32(max.to_bits()) >= x87_load32(value)) {
        x87_quiet_bits(max.to_bits())
    } else {
        value
    };
    let floor = float_floor(sse2, f32::from_bits(x87_quiet_bits(clamped)));
    let shifted = x87_sub(floor, x87_m32(min.to_bits()));
    let scaled = x87_mul(shifted, x87_m32(scale.to_bits()));
    // The integer's low byte, as the exe stores it.
    fistp_chop(scaled).to_le_bytes()[0]
}

/// Below this size every float the clamp can give is truncated exactly by both `_ftol2` bodies
/// and comes back from the integer exactly, so the float floor is the plain integer floor.
const FLOOR_EXACT: f32 = 16_777_216.0;
/// The most entries a [`FloorTable`] holds (a face tint's range, -255 to 255, needs 511).
const FLOOR_TABLE_ENTRIES: i32 = 4096;

/// The integer floor of the float with the bits `word`, below [`FLOOR_EXACT`] in size: the whole
/// part of the magnitude, negated for a negative value and one less when that has a fraction
/// (`-0.0` gives 0). Integer steps only.
#[inline]
fn floor_int(word: u32) -> i32 {
    let negative = word >> 31 != 0;
    let magnitude = word & 0x7fff_ffff;
    // The biased exponent: below 127 the size is below 1; at most 150 below 2^24.
    let exponent = magnitude >> 23;
    if exponent < 127 {
        return if negative && magnitude != 0 { -1 } else { 0 };
    }
    let shift = 150 - exponent;
    let significand = (magnitude & 0x007f_ffff) | 0x0080_0000;
    let whole = (significand >> shift) as i32;
    let fraction = significand & ((1 << shift) - 1) != 0;
    match (negative, fraction) {
        (false, _) => whole,
        (true, false) => -whole,
        (true, true) => -whole - 1,
    }
}

/// [`compress_channel`]'s bytes for one image's `min`, `max` and `scale`, by the floor of the
/// clamped value. With `min` and `max` below [`FLOOR_EXACT`] in size (so finite) and `min <= max`,
/// the clamped value lies between them, so the float floor is its integer floor k, from
/// `floor(min)` to `floor(max)` (a `-0.0` floors to `+0.0`, which k = 0 gives too), and the byte
/// depends on k alone. The entry for k is [`compress_channel`] of k itself: k clamps to `min` only
/// when it is `floor(min)` below `min`, whose floor is k again. The same steps on the same thread
/// under the same control word give the same byte, NaNs, infinities and the edges included.
struct FloorTable {
    min: f32,
    max: f32,
    /// `floor(min)`, the first entry's k.
    lowest: i32,
    bytes: Vec<u8>,
}

impl FloorTable {
    /// The table, or `None` when a guard says the floor is not the integer floor or the range
    /// needs more than [`FLOOR_TABLE_ENTRIES`] entries.
    fn new(sse2: bool, min: f32, max: f32, scale: f32) -> Option<FloorTable> {
        if !(min.abs() < FLOOR_EXACT && max.abs() < FLOOR_EXACT && min <= max) {
            return None;
        }
        let floors = floor_int(min.to_bits())..=floor_int(max.to_bits());
        if floors.end() - floors.start() >= FLOOR_TABLE_ENTRIES {
            return None;
        }
        Some(FloorTable {
            min,
            max,
            lowest: *floors.start(),
            // Each k is below 2^24 in size: exact as a float.
            bytes: floors
                .map(|k| compress_channel(sse2, (k as f32).to_bits(), min, max, scale))
                .collect(),
        })
    }

    /// The byte of the value with the bits `bits`: clamped as [`compress_channel`] clamps (the
    /// compares see a float as the x87 loads it, exactly), then its entry.
    #[inline]
    fn byte(&self, bits: u32) -> u8 {
        let value = f32::from_bits(bits);
        let clamped = if !(self.min <= value) {
            self.min
        } else if !(self.max >= value) {
            self.max
        } else {
            value
        };
        self.bytes[self.index(floor_int(clamped.to_bits()))]
    }

    /// The entry of the floor `k`, at least the lowest.
    #[inline]
    fn index(&self, k: i32) -> usize {
        (k - self.lowest) as usize
    }
}

/// The one pass's bytes: `channels` bytes a value into `out` from the values in `floats` (four
/// floats a value), from the image's [`FloorTable`] when its `(min, max, scale)` allows one, else
/// channel by channel through [`compress_channel`]. Answers whether the table made them.
fn pack_values(
    sse2: bool,
    floats: &[u8],
    channels: usize,
    (min, max, scale): (f32, f32, f32),
    out: &mut [u8],
) -> bool {
    let table = FloorTable::new(sse2, min, max, scale);
    for (value, pixel) in floats.chunks_exact(16).zip(out.chunks_exact_mut(channels)) {
        for (channel, byte) in pixel.iter_mut().enumerate() {
            let at = 4 * channel;
            let bits = u32::from_le_bytes([value[at], value[at + 1], value[at + 2], value[at + 3]]);
            *byte = match &table {
                Some(table) => table.byte(bits),
                None => compress_channel(sse2, bits, min, max, scale),
            };
        }
    }
    table.is_some()
}

// Typed from 0064ceb0
/// `BSFaceGenImage::GetCompressedImage` (Xbox PDB; `thiscall`, four words: the float range `min`,
/// `max`, the float `scale` and a size argument). Without a texture yet (+8), with values (+0xC
/// not empty), a width (+0x24) and height (+0x28) and `max > min`: builds `NiPixelData` of the
/// image's size (0x74 bytes, three channels when the size argument's query `00726070` answers 0,
/// else four) and fills it from the values in order, each channel compressed to a byte
/// (`(floor(clamp(value)) - min) * scale`, truncated), the checked iterator read and stepped per
/// pixel as the exe's is. With the mipmap setting the converter's `GenerateMipmapLevels` makes
/// the texture's pixel data and the original is deleted; the debug hook sees the pixel data when
/// its flag is set; `NiSourceTexture::Create` makes the texture. When the size query answers 1 or
/// 0 the texture becomes the image's, else the image gets its resize; an image still without a
/// texture then takes the made one. Answers the image's texture.
pub fn get_compressed_image(
    e: &mut Engine,
    this: Ptr,
    min: f32,
    max: f32,
    scale: f32,
    size: u32,
) -> u32 {
    let image = this.addr();
    let texture = image + 8;
    let values = image + 0xc;
    let ready = pointer_get(e, texture) == 0
        && e.call(VALUES_EMPTY, &args![values]).u8() == 0
        && e.mem.u32(image + 0x24) != 0
        && e.mem.u32(image + 0x28) != 0
        && x87_load32(max.to_bits()) > x87_load32(min.to_bits());
    if ready {
        // The converter (0xA7C bytes), the made texture's pointer, the iterator, a scratch pair:
        // the translation's frame.
        e.with_stack(0xa7c + 4 + 8 + 8, |e, frame| {
            let converter = frame.addr();
            let made = converter + 0xa7c;
            let iterator = made + 4;
            let scratch = iterator + 8;
            e.call(CONVERTER_CONSTRUCT, &args![converter]);
            e.call(TEXTURE_CONSTRUCT, &args![made, 0u32]);
            let begin = e.call(VALUES_BEGIN, &args![values, scratch]).u32();
            let word = e.mem.u32(begin);
            e.mem.set_u32(iterator, word);
            let word = e.mem.u32(begin + 4);
            e.mem.set_u32(iterator + 4, word);
            let pixels = e
                .mem
                .u32(image + 0x24)
                .wrapping_mul(e.mem.u32(image + 0x28));
            let three = typed_call!(leaf::FAMILY, 0x0072_6070, leaf::word_at_4(e, size)) == 0;
            let (format, channels) = if three {
                (FORMAT_THREE_CHANNELS, 3u32)
            } else {
                (FORMAT_FOUR_CHANNELS, 4u32)
            };
            let block = e.call(NI_ALLOCATE, &args![0x74u32]).u32();
            let pixel_data = if block == 0 {
                0
            } else {
                let width = e.mem.u32(image + 0x24);
                let height = e.mem.u32(image + 0x28);
                e.call(
                    PIXEL_DATA_CONSTRUCT,
                    &args![block, width, height, format, 1u32, 1u32],
                )
                .u32()
            };
            let bytes = e.call(PIXEL_BYTES, &args![pixel_data, 0u32, 0u32]).u32();
            compress_pixels(
                e, iterator, scratch, bytes, pixels, channels, min, max, scale,
            );
            let source = if e.call(MIPMAP_SETTING, &args![MIPMAP_SETTING_OBJECT]).u8() != 0 {
                let mipmaps = e
                    .call(CONVERTER_MIPMAPS, &args![converter, pixel_data, 0u32])
                    .u32();
                if pixel_data != 0 {
                    e.vcall(pixel_data, 0, &args![1u32]);
                }
                mipmaps
            } else {
                pixel_data
            };
            if e.mem.u8(DEBUG_TEXTURE_FLAG) != 0 {
                e.call(DEBUG_TEXTURE, &args![source]);
            }
            let created = e
                .call(
                    SOURCE_TEXTURE_CREATE,
                    &args![source, 0u32, TEXTURE_PREFERENCES],
                )
                .u32();
            e.call(TEXTURE_SET, &args![made, created]);
            let query = typed_call!(leaf::FAMILY, 0x0072_6070, leaf::word_at_4(e, size));
            if query == 1 || typed_call!(leaf::FAMILY, 0x0072_6070, leaf::word_at_4(e, size)) == 0 {
                e.call(TEXTURE_ASSIGN, &args![texture, made]);
            } else {
                let raw = pointer_get(e, made);
                let resized = e.call(TEXTURE_RESIZE, &args![raw, size]).u32();
                e.call(TEXTURE_SET, &args![texture, resized]);
            }
            if pointer_get(e, texture) == 0 {
                e.call(TEXTURE_ASSIGN, &args![texture, made]);
            }
            e.call(TEXTURE_DESTRUCT, &args![made]);
            e.call(CONVERTER_DESTRUCT, &args![converter]);
        });
    }
    pointer_get(e, texture)
}

/// The pixel loop of [`get_compressed_image`]: `pixels` pixels of `channels` bytes from the values
/// at the checked iterator `{proxy, position}` at `iterator` (four floats a value), the
/// iterator stepped after each pixel with its previous words left in `scratch` (`0064dbf0`). In
/// one pass with one write of the bytes when every value lies before the end of the proxy's container (as a
/// face's do), each byte from the image's [`FloorTable`] when its range allows one
/// ([`pack_values`]; a face tint's, -255 to 255 at a scale of 0.5, does: 511 entries made instead
/// of a float floor for every channel of its 256 by 256 texels); otherwise value by value with the
/// iterator's checks, as the exe reads them.
#[allow(clippy::too_many_arguments)]
fn compress_pixels(
    e: &mut Engine,
    iterator: u32,
    scratch: u32,
    bytes: u32,
    pixels: u32,
    channels: u32,
    min: f32,
    max: f32,
    scale: f32,
) {
    if pixels == 0 {
        return;
    }
    let proxy = e.mem.u32(iterator);
    let start = e.mem.u32(iterator + 4);
    let in_range = proxy != 0 && {
        let owner = e.mem.u32(proxy);
        let end = e.mem.u32(owner.wrapping_add(0x10));
        let last = u64::from(start) + 16 * (u64::from(pixels) - 1);
        last < u64::from(end)
    };
    if in_range {
        let n = pixels as usize;
        let mut floats = vec![0u8; n * 16];
        e.mem.read(start, &mut floats);
        let mut out = vec![0u8; n * channels as usize];
        // No call between the channels here: `_ftol2`'s flag and the control word stay as read.
        let sse2 = e.mem.u32(SSE2_AVAILABLE) != 0;
        pack_values(
            sse2,
            &floats,
            channels as usize,
            (min, max, scale),
            &mut out,
        );
        e.mem.write(bytes, &out);
        let last = start.wrapping_add(16 * (pixels - 1));
        e.mem.set_u32(iterator + 4, last.wrapping_add(16));
        e.mem.set_u32(scratch, proxy);
        e.mem.set_u32(scratch + 4, last);
        return;
    }
    let mut out = bytes;
    for _ in 0..pixels {
        for channel in 0..channels {
            let (proxy, at) = (e.mem.u32(iterator), e.mem.u32(iterator + 4));
            check_iterator(e, proxy, at);
            let bits = e.mem.u32(at.wrapping_add(4 * channel));
            let sse2 = e.mem.u32(SSE2_AVAILABLE) != 0;
            let byte = compress_channel(sse2, bits, min, max, scale);
            e.mem.set_u8(out.wrapping_add(channel), byte);
        }
        out = out.wrapping_add(channels);
        let (proxy, at) = (e.mem.u32(iterator), e.mem.u32(iterator + 4));
        check_iterator(e, proxy, at);
        e.mem.set_u32(iterator + 4, at.wrapping_add(0x10));
        e.mem.set_u32(scratch, proxy);
        e.mem.set_u32(scratch + 4, at);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A morph of `vertices` packed points at `stride` with `total` additional vertices (none at
    /// 0) and offsets for every step, each in a block of its own, every check passing: the
    /// one-pass path's own case. Answers the engine-side words: the offsets iterator, the layout,
    /// the points block.
    fn morph(
        e: &mut Engine,
        vertices: u32,
        stride: u32,
        total: u32,
    ) -> (Offsets, MorphLayout, u32) {
        let base = e.mem.alloc(stride * vertices + 64);
        for v in 0..vertices {
            for k in 0..3 {
                e.mem
                    .set_f32(base + stride * v + 4 * k, (v * 3 + k) as f32 + 0.5);
            }
        }
        let steps = vertices.max(total);
        let data = e.mem.alloc(6 * steps + 8);
        for k in 0..3 * steps {
            e.mem
                .set_i16(data + 2 * k, i16::try_from(k).unwrap() * 37 - 500);
        }
        // The offsets vector (its proxy at +0, first +0xc, last +0x10) and its proxy.
        let vector = e.mem.alloc(0x18);
        let proxy = e.mem.alloc(8);
        e.mem.set_u32(proxy, vector);
        e.mem.set_u32(vector, proxy);
        e.mem.set_u32(vector + 0xc, data);
        e.mem.set_u32(vector + 0x10, data + 6 * steps);
        let (first_extra, extra_data) = if total == 0 {
            (0, 0)
        } else {
            let extra = e.mem.alloc(12 * total + 8);
            for k in 0..3 * total {
                e.mem.set_f32(extra + 4 * k, k as f32 - 7.25);
            }
            let extra_data = e.mem.alloc(0x18);
            e.mem.set_u32(extra_data + 0xc, extra);
            e.mem.set_u32(extra_data + 0x10, total);
            (extra, extra_data)
        };
        let offsets = Offsets { proxy, at: data };
        let layout = MorphLayout {
            base,
            stride,
            first_extra,
            extra_data,
            count: vertices,
        };
        (offsets, layout, e.mem.alloc(0x40))
    }

    /// The bytes of the points and the additional vertices, to see that a refused pass wrote
    /// nothing.
    fn written(e: &Engine, layout: &MorphLayout, total: u32) -> Vec<u8> {
        let mut bytes = e.mem.bytes(layout.base, layout.stride * layout.count + 12);
        if layout.first_extra != 0 {
            bytes.extend(e.mem.bytes(layout.first_extra, 12 * total));
        }
        bytes
    }

    /// Runs the pass and answers whether it was taken; a refused one must leave the points.
    fn taken(
        e: &mut Engine,
        offsets: &Offsets,
        layout: &MorphLayout,
        block: u32,
        total: u32,
    ) -> bool {
        let before = written(e, layout, total);
        let answer = morph_in_host(e, offsets, -0.25, layout, block);
        if answer.is_none() {
            assert_eq!(written(e, layout, total), before, "a refused pass wrote");
        }
        answer.is_some()
    }

    /// The integer floor from the bits is the float's floor below 2^24 in size (`-0.0` and `0.0`
    /// alike): a spread over every exponent, sign and fraction.
    #[test]
    fn the_integer_floor_is_the_floats_floor() {
        let mut seen = 0;
        for high in 0..=u16::MAX {
            let top = u32::from(high) << 16;
            for word in [top, top | u32::from(high), top | 0xffff, top | 1] {
                let value = f32::from_bits(word);
                if value.abs() < FLOOR_EXACT {
                    assert_eq!(floor_int(word) as f32, value.floor(), "{word:08x}");
                    seen += 1;
                }
            }
        }
        assert!(seen > 100_000, "{seen}");
    }

    /// The bits a channel may hold that the floor table must answer as [`compress_channel`] does:
    /// the special ones, a spread over every bit pattern, and each integer around `min` and `max`
    /// with its neighbours and halves.
    fn channel_values(min: f32, max: f32) -> Vec<u32> {
        let mut out = vec![
            0x0000_0000, // 0.0
            0x8000_0000, // -0.0
            0x7f80_0000, // infinity
            0xff80_0000, // -infinity
            0x7fc0_0000, // a quiet NaN
            0xffc0_0000, // a negative quiet NaN
            0x7f80_0001, // a signalling NaN
            0xff80_0001, // a negative signalling NaN
            0x0000_0001, // the smallest subnormal
            0x8000_0001, // a negative subnormal
            0x807f_ffff, // the largest negative subnormal
            0x7f7f_ffff, // the largest float
            0xff7f_ffff, // the lowest float
            0x4b80_0000, // 2^24
            0x4f00_0000, // 2^31
            0xcf00_0000, // -2^31
            0x4f80_0000, // 2^32
            0x5f00_0000, // 2^63
        ];
        out.extend((0..=u16::MAX).map(|i| u32::from(i) * 0x0001_0001));
        for k in floor_int(min.to_bits()) - 2..=floor_int(max.to_bits()) + 2 {
            let integral = (k as f32).to_bits();
            out.extend([integral.wrapping_sub(1), integral, integral + 1]);
            out.extend([(k as f32 + 0.5).to_bits(), (k as f32 - 0.5).to_bits()]);
        }
        out.extend([min.to_bits(), max.to_bits()]);
        out
    }

    /// The table's byte is [`compress_channel`]'s for every value, under both control words and
    /// both `_ftol2` bodies, in the face tint's range (-255 to 255, scale 0.5) and ranges with
    /// fractions at either end, a single value, signed zeros, the most entries near 2^24, scales of every sign, an infinite and a NaN scale.
    #[test]
    fn the_floor_table_gives_each_channel_the_float_steps_byte() {
        let ranges: [(f32, f32, f32); 15] = [
            // A face tint's (`ApplyCoordinateTexturingToMesh`, the one caller).
            (-255.0, 255.0, 0.5),
            (0.0, 255.0, 1.0),
            (0.5, 255.0, 1.0),
            (-0.25, 0.5, 256.0),
            (-100.0, 255.0, -1.0),
            (-100.0, 1.0, 0.25),
            (-0.0, 0.0, 1.0),
            (2.5, 2.5, 7.0),
            (0.0, 255.0, f32::INFINITY),
            (0.0, 255.0, f32::NAN),
            (f32::from_bits(0x8000_0001), 255.0, 1.0),
            (-16_777_215.0, -16_773_120.0, 1.0),
            (16_773_120.0, 16_777_215.0, 3.0),
            (0.0, 4095.5, 0.0625),
            (-3.75, 300.25, 0.8),
        ];
        for word in [0x027fu16, 0x007f] {
            engine_core::x87::set_control_word(word);
            for sse2 in [true, false] {
                for (min, max, scale) in ranges {
                    let table = FloorTable::new(sse2, min, max, scale).expect("a table");
                    for bits in channel_values(min, max) {
                        assert_eq!(
                            table.byte(bits),
                            compress_channel(sse2, bits, min, max, scale),
                            "{bits:08x} in {min} to {max}, scale {scale}, sse2 {sse2}, {word:04x}"
                        );
                    }
                }
            }
        }
        engine_core::x87::set_control_word(engine_core::x87::DEFAULT_CONTROL_WORD);
    }

    /// The table is made for a face's range and the edges the guards allow, and refused by each
    /// guard alone.
    #[test]
    fn the_floor_table_is_made_for_a_face_and_refused_by_each_guard() {
        let made = |min: f32, max: f32| FloorTable::new(true, min, max, 1.0).map(|t| t.bytes.len());
        assert_eq!(made(-255.0, 255.0), Some(511), "a face tint's");
        assert_eq!(made(0.0, 255.0), Some(256));
        assert_eq!(made(-0.25, 0.5), Some(2));
        assert_eq!(made(-0.0, 0.0), Some(1));
        assert_eq!(made(2.5, 2.5), Some(1));
        assert_eq!(made(0.0, 4095.5), Some(4096));
        assert_eq!(made(-16_777_215.0, -16_773_120.0), Some(4096));
        assert_eq!(made(16_773_120.0, 16_777_215.0), Some(4096));
        assert_eq!(made(f32::NAN, 255.0), None, "min a NaN");
        assert_eq!(made(0.0, f32::NAN), None, "max a NaN");
        assert_eq!(made(f32::NEG_INFINITY, 255.0), None, "min infinite");
        assert_eq!(made(0.0, f32::INFINITY), None, "max infinite");
        assert_eq!(made(-16_777_216.0, -16_777_000.0), None, "min at 2^24");
        assert_eq!(made(16_777_000.0, 16_777_216.0), None, "max at 2^24");
        assert_eq!(made(1.0, 0.0), None, "min above max");
        assert_eq!(made(0.0, 4096.0), None, "one entry past the most");
        assert_eq!(made(-0.5, 4095.0), None, "one entry past the most, below 0");
    }

    /// A face tint's values take the table in the one pass (its range from the one caller,
    /// `ApplyCoordinateTexturingToMesh`: -255 to 255, scale 0.5), with the float steps' bytes, three
    /// and four channels a value, both `_ftol2` bodies; a range the table cannot hold goes channel by
    /// channel and says so.
    #[test]
    fn a_face_tint_takes_the_floor_table() {
        let (min, max, scale) = (-255.0f32, 255.0f32, 0.5f32);
        let mut words = channel_values(min, max);
        words.truncate(words.len() / 4 * 4);
        let floats: Vec<u8> = words.iter().flat_map(|word| word.to_le_bytes()).collect();
        for channels in [3usize, 4] {
            for sse2 in [true, false] {
                let mut out = vec![0u8; words.len() / 4 * channels];
                assert!(
                    pack_values(sse2, &floats, channels, (min, max, scale), &mut out),
                    "the table, {channels} channels"
                );
                for (value, pixel) in words.chunks_exact(4).zip(out.chunks_exact(channels)) {
                    for (channel, byte) in pixel.iter().enumerate() {
                        let word = value[channel];
                        assert_eq!(
                            *byte,
                            compress_channel(sse2, word, min, max, scale),
                            "{word:08x}"
                        );
                    }
                }
            }
        }
        let mut out = [0u8; 3];
        assert!(!pack_values(
            true,
            &floats[..16],
            3,
            (0.0, 100_000.0, 1.0),
            &mut out
        ));
    }

    /// The one-pass path runs only when the element loops' result cannot differ: each guard that
    /// sends a morph back to them, alone.
    #[test]
    fn the_one_pass_morph_steps_aside_for_each_guard() {
        // The plain cases run it: with and without additional vertices, a multiple of four.
        for (vertices, total) in [(8u32, 0u32), (8, 11), (9, 0)] {
            let mut e = Engine::new();
            let (offsets, layout, block) = morph(&mut e, vertices, 12, total);
            assert!(
                taken(&mut e, &offsets, &layout, block, total),
                "{vertices} {total}"
            );
        }
        // A stride below a point: the points overlap each other.
        let mut e = Engine::new();
        let (offsets, layout, block) = morph(&mut e, 8, 8, 0);
        assert!(!taken(&mut e, &offsets, &layout, block, 0), "stride 8");
        // No proxy; a last check at the proxy's last element.
        let mut e = Engine::new();
        let (mut offsets, layout, block) = morph(&mut e, 8, 12, 0);
        let proxy = offsets.proxy;
        offsets.proxy = 0;
        assert!(!taken(&mut e, &offsets, &layout, block, 0), "no proxy");
        offsets.proxy = proxy;
        let vector = e.mem.u32(proxy);
        let last = e.mem.u32(vector + 0x10);
        e.mem.set_u32(vector + 0x10, last - 6);
        assert!(
            !taken(&mut e, &offsets, &layout, block, 0),
            "a failing check"
        );
        // The offsets inside the written points (copied there, the vector's words moved).
        let mut e = Engine::new();
        let (mut offsets, layout, block) = morph(&mut e, 8, 12, 0);
        let vector = e.mem.u32(offsets.proxy);
        let bytes = e.mem.bytes(offsets.at, 48);
        e.mem.write(layout.base + 12, &bytes);
        offsets.at = layout.base + 12;
        e.mem.set_u32(vector + 0xc, offsets.at);
        e.mem.set_u32(vector + 0x10, offsets.at + 48);
        assert!(
            !taken(&mut e, &offsets, &layout, block, 0),
            "offsets in the points"
        );
        // The proxy's word inside the written points.
        let mut e = Engine::new();
        let (mut offsets, layout, block) = morph(&mut e, 8, 12, 0);
        let vector = e.mem.u32(offsets.proxy);
        offsets.proxy = layout.base + 20;
        e.mem.set_u32(offsets.proxy, vector);
        assert!(
            !taken(&mut e, &offsets, &layout, block, 0),
            "the proxy in the points"
        );
        // The container's end word inside the written points (the container read through a
        // proxy whose word points 0x10 before it).
        let mut e = Engine::new();
        let (offsets, layout, block) = morph(&mut e, 8, 12, 0);
        let vector = e.mem.u32(offsets.proxy);
        let last = e.mem.u32(vector + 0x10);
        let owner = layout.base + 36 - 0x10;
        e.mem.set_u32(offsets.proxy, owner);
        e.mem.set_u32(owner + 0x10, last);
        assert!(
            !taken(&mut e, &offsets, &layout, block, 0),
            "the end word in the points"
        );
        // The additional vertices' count inside the written points, and inside the additional
        // vertices themselves.
        for inside_extra in [false, true] {
            let mut e = Engine::new();
            let (offsets, mut layout, block) = morph(&mut e, 8, 12, 11);
            let first = layout.first_extra;
            let at = if inside_extra {
                first + 24
            } else {
                layout.base + 48
            };
            layout.extra_data = at - 0x10;
            e.mem.set_u32(at - 4, first);
            e.mem.set_u32(at, 11);
            assert!(
                !taken(&mut e, &offsets, &layout, block, 11),
                "the count {inside_extra}"
            );
        }
        // The additional vertices overlapping the points, and the offsets inside them.
        let mut e = Engine::new();
        let (offsets, mut layout, block) = morph(&mut e, 8, 12, 11);
        layout.first_extra = layout.base + 12;
        assert!(
            !taken(&mut e, &offsets, &layout, block, 11),
            "the additional vertices in the points"
        );
        let mut e = Engine::new();
        let (mut offsets, layout, block) = morph(&mut e, 8, 12, 11);
        let vector = e.mem.u32(offsets.proxy);
        let bytes = e.mem.bytes(offsets.at, 66);
        let inside = layout.first_extra + 12;
        e.mem.write(inside, &bytes);
        offsets.at = inside;
        e.mem.set_u32(vector + 0xc, inside);
        e.mem.set_u32(vector + 0x10, inside + 66);
        assert!(
            !taken(&mut e, &offsets, &layout, block, 11),
            "offsets in the additional vertices"
        );
    }

    /// Runs `f` with the running thread's x87 control word set to `word`, then sets it back.
    fn under(word: u16, f: impl FnOnce()) {
        let before = engine_core::x87::control_word();
        engine_core::x87::set_control_word(word);
        f();
        engine_core::x87::set_control_word(before);
    }

    /// Float words over every sign, exponent and fraction (NaNs and infinities among them).
    fn spread(seed: u32, n: usize) -> Vec<u32> {
        let mut x = seed;
        (0..n)
            .map(|_| {
                x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                x ^ (x >> 13)
            })
            .collect()
    }

    /// [`morph_words`] and [`morphed`] coordinate by coordinate on the same words, under the
    /// running control word: the same bits everywhere. Answers how many took the helpers.
    fn morph_both(offsets: &[i16], weight: u32, words: &[u32]) -> usize {
        let bytes: Vec<u8> = offsets.iter().flat_map(|o| o.to_le_bytes()).collect();
        let weight_value = x87_m32(weight);
        let mut plain = words.to_vec();
        let helped = morph_words(&mut plain, &bytes, weight_value);
        let control = engine_core::x87::control_word();
        for (k, ((&word, &offset), &got)) in words.iter().zip(offsets).zip(&plain).enumerate() {
            let want = morphed(offset, weight_value, word);
            assert_eq!(
                got, want,
                "coordinate {k}: offset {offset}, weight {weight:08x}, word {word:08x}, \
                 control word {control:04x}"
            );
        }
        helped
    }

    /// The plain steps give the helpers' bits under every precision and rounding mode: weights and
    /// coordinates over every exponent (the awkward ones named), the offsets' ends, and
    /// coordinates near the product's negation (sums near zero, where whether the product was
    /// rounded to 24 bits before the sum shows, and the subnormal sums).
    #[test]
    fn the_plain_morph_steps_give_the_helpers_bits() {
        let named = [
            0,
            0x8000_0000,
            1,
            0x8000_0001,
            0x007f_ffff,
            0x0080_0000,
            0x0080_0001,
            0x0700_0000,
            0x3f80_0000,
            0xbf80_0000,
            0x3eaa_aaab,
            0x4f00_0000,
            0x7f00_0000,
            0x7f7f_ffff,
            0xff7f_ffff,
            0x7f80_0000,
            0x7fc0_0000,
            0x7f80_0001,
        ];
        let weights: Vec<u32> = named.into_iter().chain(spread(7, 150)).collect();
        let offsets: Vec<i16> = (0..300u32)
            .map(|k| match k % 11 {
                0 => i16::MAX,
                1 => i16::MIN,
                2 => 0,
                3 => 1,
                4 => -1,
                _ => i16::try_from(k.wrapping_mul(2_654_435_761) % 65_536 / 2).unwrap() - 16_384,
            })
            .collect();
        let head = |k: u32| ((k % 2001) as f32 / 97.0 - 10.0).to_bits();
        // The plain steps were taken somewhere under each word that rounds to nearest.
        for control in [0x027f, 0x007f, 0x037f, 0x0c7f, 0x047f, 0x0e7f, 0x087f] {
            under(control, || {
                let mut plain = 0;
                for (i, &weight) in (0u32..).zip(weights.iter()) {
                    let w = x87_m32(weight);
                    let scattered = spread(i + 11, offsets.len());
                    let words: Vec<u32> = (0u32..)
                        .zip(offsets.iter())
                        .zip(scattered.iter())
                        .map(|((k, &offset), &bits)| match k % 4 {
                            0 => bits,
                            1 => head(k + i),
                            _ => (morphed(offset, w, 0) ^ 0x8000_0000)
                                .wrapping_add(k % 5)
                                .wrapping_sub(2),
                        })
                        .collect();
                    plain += words.len() - morph_both(&offsets, weight, &words);
                }
                assert_eq!(
                    plain > 0,
                    control & 0x0c00 == 0,
                    "control word {control:04x}"
                );
            });
        }
    }

    /// A head's morph takes the plain steps for every coordinate at both of the game's control
    /// words; a NaN coordinate takes the helpers for itself alone, a NaN weight or another
    /// rounding mode for all.
    #[test]
    fn a_heads_morph_takes_the_plain_steps() {
        let offsets: Vec<i16> = (0..1500u32)
            .map(|k| i16::try_from(k * 37 % 4001).unwrap() - 2000)
            .collect();
        let words: Vec<u32> = (0..1500u32)
            .map(|k| ((k % 2001) as f32 / 97.0 - 10.0).to_bits())
            .collect();
        let mut with_nan = words.clone();
        with_nan[70] = 0x7fc0_0001;
        for control in [0x027f, 0x007f] {
            under(control, || {
                for weight in [0.000_123f32, -0.5, 1.0e-4, 0.031, 0.0, -0.0] {
                    assert_eq!(
                        morph_both(&offsets, weight.to_bits(), &words),
                        0,
                        "weight {weight}, control word {control:04x}"
                    );
                }
                assert_eq!(morph_both(&offsets, 0.25f32.to_bits(), &with_nan), 1);
                assert_eq!(morph_both(&offsets, 0x7fc0_0000, &words), words.len());
            });
        }
        under(0x0c7f, || {
            assert_eq!(morph_both(&offsets, 0.25f32.to_bits(), &words), words.len());
        });
    }
}
