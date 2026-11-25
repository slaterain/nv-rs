//! The small records the extra data list unit's loader and save writer build and byte-swap
//! (`00413f40` to `00414470`, `004168a0` to `00416b80`, `004181c0`): the occlusion plane's centre,
//! rotation, setters and links, a primitive's colour and radii, the portal and radio records, the
//! navmesh portal extra data, the rotation matrix of an angle about an axis and its `sincos`.

use super::{copy_words, extra_init, EXTRA_PAYLOAD, SIMPLE_LIST_NEW};
use crate::anim_keys::narrow;
use crate::crt::low_half;
use engine_core::x87::{x87_fsincos32, x87_load32, x87_quiet_bits, x87_store32};
use engine_core::{args, Engine};

/// `Swap32(ptr, 0)` and `Swap16(ptr, 0)`.
const SWAP_DWORD: u32 = 0x0040_1080;
const SWAP_WORD: u32 = 0x0040_7a90;
/// The type 0x5a extra data's (the navmesh portal's) vtable.
const VTABLE_EXTRA_TYPE_5A: u32 = 0x0101_4618;
/// The flag bytes the occlusion plane's setters set: its vertices and its planes are stale.
const PLANE_DIRTY_VERTICES: u32 = 0xe4;
const PLANE_DIRTY_PLANES: u32 = 0xe5;
/// `2^63`: the bound of `FSINCOS`'s operand.
const TWO_POW_63: f64 = 9_223_372_036_854_775_808.0;

/// `count` words from `this` byte-swapped in place, in order (`Swap32`, by address).
fn swap_words(e: &mut Engine, this: u32, count: u32) {
    for index in 0..count {
        e.call(SWAP_DWORD, &args![this.wrapping_add(index * 4), 0u32]);
    }
}

/// The occlusion plane's vertices and planes marked stale.
#[inline]
fn plane_dirty(e: &mut Engine, this: u32) {
    e.mem.set_u8(this.wrapping_add(PLANE_DIRTY_VERTICES), 1);
    e.mem.set_u8(this.wrapping_add(PLANE_DIRTY_PLANES), 1);
}

/// The sine and the cosine of `angle` as `004169a0` stores them: the x87's `FSINCOS` on the loaded
/// `float`, each stored as a `float`; for a finite `|angle| >= 2^63` it pushes nothing, so the angle
/// is stored as the cosine and the empty register's default NaN as the sine.
fn sincos_bits(angle: f32) -> (u32, u32) {
    let bits = angle.to_bits();
    let loaded = x87_load32(bits);
    let (sine, cosine) = if loaded.is_finite() && loaded.abs() >= TWO_POW_63 {
        (0xffc0_0000, x87_store32(loaded))
    } else {
        x87_fsincos32(bits)
    };
    (x87_quiet_bits(sine), x87_quiet_bits(cosine))
}

// Typed from 00413f40
/// The address of the occlusion plane's centre (+8).
pub fn plane_center(_e: &mut Engine, this: u32) -> u32 {
    this.wrapping_add(8)
}

// Typed from 00413f60
/// The occlusion plane's rotation (nine words at +0x1c) copied to `out`. Answers `out`.
pub fn plane_rotation(e: &mut Engine, this: u32, out: u32) -> u32 {
    copy_words(e, this.wrapping_add(0x1c), out, 0, 36);
    out
}

// Typed from 00413f90
/// A `BGSPrimitive`'s colour (four words at +8) copied to `out`. Answers `out`.
pub fn primitive_color(e: &mut Engine, this: u32, out: u32) -> u32 {
    copy_words(e, this.wrapping_add(8), out, 0, 16);
    out
}

// Typed from 00413fc0
/// A `BGSPrimitive`'s radii (three words at +0x18) copied to `out`. Answers `out`.
pub fn primitive_radii(e: &mut Engine, this: u32, out: u32) -> u32 {
    copy_words(e, this.wrapping_add(0x18), out, 0, 12);
    out
}

// Typed from 00413ff0
/// The `float` at `source` stored at `this` through the x87 (a signalling NaN comes out quiet).
/// Answers `this`.
pub fn float_copy(e: &mut Engine, this: u32, source: u32) -> u32 {
    let bits = x87_quiet_bits(e.mem.u32(source));
    e.mem.set_u32(this, bits);
    this
}

// Typed from 00414010
/// A list node made empty: a null item and a null next. Answers `this`.
pub fn empty_node_new(e: &mut Engine, this: u32) -> u32 {
    e.mem.set_u32(this, 0);
    e.mem.set_u32(this.wrapping_add(4), 0);
    this
}

// Typed from 00414030
/// A `REF_DECAL_DATA`'s seven words byte-swapped (`Swap32`).
pub fn swap_decal(e: &mut Engine, this: u32) {
    swap_words(e, this, 7);
}

// Typed from 004140f0
/// The occlusion plane record's nine words byte-swapped.
pub fn swap_plane(e: &mut Engine, this: u32) {
    swap_words(e, this, 9);
}

// Typed from 004141e0
/// The word at +0 (`Swap32`) and the 16-bit value at +4 (`Swap16`) byte-swapped.
pub fn swap_word_short(e: &mut Engine, this: u32) {
    e.call(SWAP_DWORD, &args![this, 0u32]);
    e.call(SWAP_WORD, &args![this.wrapping_add(4), 0u32]);
}

// Typed from 00414220
/// The first four words (a `RADIO_DATA`'s or a `WORLD_LOCATION`'s) byte-swapped.
pub fn swap_four_words(e: &mut Engine, this: u32) {
    swap_words(e, this, 4);
}

// Typed from 00414290
/// The primitive record's eight words byte-swapped.
pub fn swap_primitive(e: &mut Engine, this: u32) {
    swap_words(e, this, 8);
}

// Typed from 00414370
/// The occlusion plane reference record's four words byte-swapped.
pub fn swap_plane_refs(e: &mut Engine, this: u32) {
    swap_words(e, this, 4);
}

// Typed from 004143c0
/// The portal reference record zeroed (word 1, then word 0). Answers `this`.
pub fn portal_ref_new(e: &mut Engine, this: u32) -> u32 {
    e.mem.set_u32(this.wrapping_add(4), 0);
    e.mem.set_u32(this, 0);
    this
}

// Typed from 004143f0
/// The primitive record's constructor: the radii's (`006815c0`, empty), then the colour's four
/// zeros (`00414430`), both in place. Answers `this`.
pub fn primitive_new(e: &mut Engine, this: u32) -> u32 {
    for index in 0..4 {
        e.mem
            .set_u32(this.wrapping_add(EXTRA_PAYLOAD + index * 4), 0);
    }
    this
}

// Typed from 00414430
/// `NiColorA`'s constructor: four `float`s at +0 to +0xc through the x87. Answers `this`.
pub fn color_new(e: &mut Engine, this: u32, a: f32, b: f32, c: f32, d: f32) -> u32 {
    for (offset, value) in (0..16u32).step_by(4).zip([a, b, c, d]) {
        e.mem
            .set_u32(this.wrapping_add(offset), x87_quiet_bits(value.to_bits()));
    }
    this
}

// Typed from 00414470
/// The occlusion plane's constructor: its members' empty constructors (`006815c0` at +0, +8 and
/// +0x14) do nothing. Answers `this`.
pub fn plane_new(_e: &mut Engine, this: u32) -> u32 {
    this
}

// Typed from 004168a0
/// The rotation of `angle` radians about the axis `(x, y, z)` (Rodrigues' formula, nine `float`s at
/// `this`), its sine and cosine as `004169a0` gives them, in place; worked in `f64` and rounded to
/// `float` where the exe stores a temporary.
pub fn rotation_matrix(e: &mut Engine, this: u32, angle: f32, x: f32, y: f32, z: f32) {
    let (sine, cosine) = sincos_bits(angle);
    let sine = f64::from(f32::from_bits(sine));
    let cosine = f64::from(f32::from_bits(cosine));
    let (x, y, z) = (f64::from(x), f64::from(y), f64::from(z));
    let one_minus_cos = f64::from(narrow(1.0 - cosine));
    let xx = f64::from(narrow(x * x));
    let yy = f64::from(narrow(y * y));
    let zz = f64::from(narrow(z * z));
    let xy = f64::from(narrow(x * y * one_minus_cos));
    let xz = f64::from(narrow(x * z * one_minus_cos));
    let yz = f64::from(narrow(y * z * one_minus_cos));
    let x_sin = f64::from(narrow(x * sine));
    let y_sin = f64::from(narrow(y * sine));
    let z_sin = f64::from(narrow(z * sine));
    let matrix = [
        xx * one_minus_cos + cosine,
        xy + z_sin,
        xz - y_sin,
        xy - z_sin,
        yy * one_minus_cos + cosine,
        yz + x_sin,
        xz + y_sin,
        yz - x_sin,
        zz * one_minus_cos + cosine,
    ];
    for (offset, value) in (0..36u32).step_by(4).zip(matrix) {
        e.mem.set_f32(this.wrapping_add(offset), narrow(value));
    }
}

// Typed from 004169a0
/// `sincos` (`cdecl(angle, sine, cosine)`): the sine of `angle` stored at `sine`, then the cosine at
/// `cosine`, as the x87's `FSINCOS` gives them.
pub fn sincos(e: &mut Engine, angle: f32, sine: u32, cosine: u32) {
    let (sine_bits, cosine_bits) = sincos_bits(angle);
    e.mem.set_u32(sine, sine_bits);
    e.mem.set_u32(cosine, cosine_bits);
}

// Typed from 004169d0
/// The word at +0x1d8 (the data handler's region list).
pub fn word_1d8(e: &mut Engine, this: u32) -> u32 {
    e.mem.u32(this.wrapping_add(0x1d8))
}

// Typed from 004169f0
/// `BSOcclusionPlane`'s centre set: three words from `center` to +8, then stale.
pub fn plane_set_center(e: &mut Engine, this: u32, center: u32) {
    copy_words(e, center, this.wrapping_add(8), 0, 12);
    plane_dirty(e, this);
}

// Typed from 00416a30
/// `BSOcclusionPlane`'s size set: two words from `half_extents` to +0x14, then stale.
pub fn plane_set_size(e: &mut Engine, this: u32, half_extents: u32) {
    let first = e.mem.u32(half_extents);
    let second = e.mem.u32(half_extents.wrapping_add(4));
    e.mem.set_u32(this.wrapping_add(0x14), first);
    e.mem.set_u32(this.wrapping_add(0x18), second);
    plane_dirty(e, this);
}

// Typed from 00416a70
/// `BSOcclusionPlane`'s rotation set: nine words from `rotation` to +0x1c, then stale.
pub fn plane_set_rotation(e: &mut Engine, this: u32, rotation: u32) {
    copy_words(e, rotation, this.wrapping_add(0x1c), 0, 36);
    plane_dirty(e, this);
}

// Typed from 00416ab0
/// The old navmesh portal record (a word and a 16-bit value) zeroed. Answers `this`.
pub fn portal_record_new(e: &mut Engine, this: u32) -> u32 {
    e.mem.set_u32(this, 0);
    e.mem.set_u16(this.wrapping_add(4), 0);
    this
}

// Typed from 00416ad0
/// The navmesh portal extra data's constructor (type 0x5a, `thiscall(this, record by value)`): the
/// base with the type, its vtable, and the record (the mesh word and the triangle's 16 bits) copied
/// to +0xc as `0069a690` copies it, in place. Answers `this`.
pub fn portal_extra_new(e: &mut Engine, this: u32, word: u32, short_part: u32) -> u32 {
    extra_init(e, this, 0x5a);
    e.mem.set_u32(this, VTABLE_EXTRA_TYPE_5A);
    e.mem.set_u32(this.wrapping_add(EXTRA_PAYLOAD), word);
    e.mem
        .set_u16(this.wrapping_add(EXTRA_PAYLOAD + 4), low_half(short_part));
    this
}

// Typed from 00416b40
/// `RADIO_DATA`'s constructor: radius 0.0, range type 1, static 0.0, no position reference. Answers
/// `this`.
pub fn radio_data_new(e: &mut Engine, this: u32) -> u32 {
    e.mem.set_u32(this, 0);
    e.mem.set_u32(this.wrapping_add(4), 1);
    e.mem.set_u32(this.wrapping_add(8), 0);
    e.mem.set_u32(this.wrapping_add(0x0c), 0);
    this
}

// Typed from 00416b80
/// `RoomLinkedRefData`'s constructor: its two lists' heads at +0 and +8 (`0096a2d0`). Answers
/// `this`.
pub fn room_links_new(e: &mut Engine, this: u32) -> u32 {
    e.call(SIMPLE_LIST_NEW, &args![this]);
    e.call(SIMPLE_LIST_NEW, &args![this.wrapping_add(8)]);
    this
}

// Typed from 004181c0
/// `BSOcclusionPlane`'s linked plane `index` (+0xec, no bound) set.
pub fn plane_set_linked(e: &mut Engine, this: u32, index: u32, linked_plane: u32) {
    e.mem.set_u32(
        this.wrapping_add(0xec).wrapping_add(index.wrapping_mul(4)),
        linked_plane,
    );
}
