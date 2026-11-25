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
//!   and scaled).
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
//! keep the exact x87 helpers at the running thread's precision ([`Threads::Both`]): each is a
//! short Rust function, not a call by address, so they cost little against what the loops lose.
//! Integer sums are integer sums (wrapping, so their order does not matter).
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
    if e.mem.u32(SSE2_AVAILABLE) != 0 {
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
/// Answers the offsets iterator's end position, or `None` (nothing read or written) when a step
/// could take a check's failing side (no proxy, a position at or past the proxy's last element)
/// or when what the loops write could change what they read later (overlapping arrays, a stride
/// below a point, an address past 4 GB): then the element loops run, as the exe's do.
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
    let delta = |k: usize| i16::from_le_bytes([deltas[2 * k], deltas[2 * k + 1]]);
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
        for (k, word) in tail.iter_mut().enumerate() {
            *word = morphed(delta(k), weight, *word);
        }
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
        for (k, word) in points.iter_mut().enumerate() {
            *word = morphed(delta(k), weight, *word);
        }
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
#[inline]
fn float_floor(e: &Engine, value: f32) -> f64 {
    let mut truncated = f32::from_bits(x87_store32(f64::from(ftol(e, f64::from(value)))));
    let difference = f64::from(value) - f64::from(truncated);
    if !(difference >= 0.0) {
        truncated = f32::from_bits(x87_store32(f64::from(truncated) - 1.0));
    }
    f64::from(truncated)
}

/// One channel of a compressed pixel from the value's bits: clamped to `[min, max]` (below `min`
/// or unordered gives `min`; above `max` gives `max`), floored, less `min`, times `scale`,
/// truncated to an integer; its low byte.
#[inline]
fn compress_channel(e: &Engine, bits: u32, min: f32, max: f32, scale: f32) -> u8 {
    let value = x87_quiet_bits(bits);
    let clamped = if !(x87_load32(min.to_bits()) <= x87_load32(value)) {
        x87_quiet_bits(min.to_bits())
    } else if !(x87_load32(max.to_bits()) >= x87_load32(value)) {
        x87_quiet_bits(max.to_bits())
    } else {
        value
    };
    let floor = float_floor(e, f32::from_bits(x87_quiet_bits(clamped)));
    let shifted = x87_sub(floor, x87_m32(min.to_bits()));
    let scaled = x87_mul(shifted, x87_m32(scale.to_bits()));
    // The integer's low byte, as the exe stores it.
    fistp_chop(scaled).to_le_bytes()[0]
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
/// face's do); otherwise value by value with the iterator's checks, as the exe reads them.
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
        for (value, pixel) in floats
            .chunks_exact(16)
            .zip(out.chunks_exact_mut(channels as usize))
        {
            for (channel, byte) in pixel.iter_mut().enumerate() {
                let at = 4 * channel;
                let bits =
                    u32::from_le_bytes([value[at], value[at + 1], value[at + 2], value[at + 3]]);
                *byte = compress_channel(e, bits, min, max, scale);
            }
        }
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
            let byte = compress_channel(e, bits, min, max, scale);
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
