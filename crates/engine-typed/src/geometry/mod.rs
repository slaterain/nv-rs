//! The geometry family (family 23): the code that reads and writes the arrays of `NiGeometryData` (the
//! positions, the normals with the binormals and tangents after them, the colours and the texture
//! coordinates), typed so that a later step (stage 2 of ADR-0014) can keep those arrays outside the
//! game's 32-bit memory: about 23 to 40 MB of the guest heap on the census route, and what mesh mods
//! grow. Stage 2 can only start once every reader and writer of the arrays is typed; this family
//! grows to that, a batch at a time:
//!
//! - [`pack`]: the renderer's packing of a geometry's arrays into its vertex buffer (the hottest reader:
//!   about 52 million reads on the census route): the shader declaration's per-stream pack, the element
//!   pack that picks an array, the converters that can receive one, the texture set getter, and the
//!   fixed-function packer with its strided copy.
//!
//! Stage 1: the arrays and the objects stay in guest memory where the game made them and are read
//! through their [`layout`]. Each function stays registered at its exe address; a call the exe makes
//! through a function pointer is a `typed_call!` when the pointer holds one of this family's functions
//! (a Rust call while the family is on), the same call by address otherwise.
//!
//! **Threads.** The renderer packs on the thread that draws (the main thread, a render helper with the
//! parallel walk) and when the loaders precache geometry, so the family is [`Threads::Both`]: the colour
//! channels' products are exact in an `f64` under either control word (see `pack`'s channel), and the
//! float to integer stores go through `fistp` and `fistp_chop` or the exe's own `_ftol2`.

use engine_core::ptr::Ptr;
use engine_core::typed::{Family, FamilyId, Threads};
use engine_core::{entry, AbiFn};

pub mod layout;
pub mod pack;

/// This family's bit: what each `typed_call!` of it names.
pub const FAMILY: FamilyId = FamilyId::new(23);

/// `NV_TYPED=geometry`. Not past its gate yet, so off unless `NV_TYPED` names it.
pub static GEOMETRY: Family = Family {
    name: "geometry",
    id: FAMILY,
    past_gate: false,
    threads: Threads::Both,
    stage2: false,
    funcs,
};

/// The low 16 bits of a word (a field the exe reads as a `u16`, a count it stores as one).
#[allow(clippy::cast_possible_truncation)]
#[inline(always)]
pub(crate) fn low_half(word: u32) -> u16 {
    word as u16
}

/// A 32-bit integer's bits as a word (a `FISTP` result the code shifts and ors as a word).
#[inline(always)]
pub(crate) fn word_of(value: i32) -> u32 {
    value as u32
}

/// The low byte of a word (a byte the exe stores from a register).
#[allow(clippy::cast_possible_truncation)]
#[inline(always)]
pub(crate) fn low_byte(word: u32) -> u8 {
    word as u8
}

/// The family's functions, by exe address.
#[rustfmt::skip]
fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        // The per-stream pack and the element pack.
        entry!(0x00e7_6770, pack::pack_stream(Ptr, u32, u32, u32, u32, u32, u32, u32, u32, u32) -> u32),
        entry!(0x00e7_a200, pack::pack_entry(Ptr, u32, u32, u32, u32, u32, u32, u32, u32, u32) -> u32),
        // The converters that can receive an array.
        entry!(0x00e7_8f80, pack::copy_same(u32) -> u32),
        entry!(0x00e7_8d50, pack::copy_narrower(u32) -> u32),
        entry!(0x00e7_8e30, pack::copy_wider(u32) -> u32),
        entry!(0x00e7_9d00, pack::colour(u32) -> u32),
        entry!(0x00e7_7950, pack::floats_to_bytes(Ptr) -> u32),
        entry!(0x00e7_6c30, pack::floats_to_shorts(Ptr) -> u32),
        // The texture set getter.
        entry!(0x00a6_6960, pack::texture_set(Ptr, u32) -> u32),
        // The fixed-function packer and its strided copy.
        entry!(0x00e8_c1e0, pack::pack_untransformed(Ptr, Ptr, u32, u32, u32, u32) -> u32),
        entry!(0x00e8_c090, pack::copy_strided(Ptr, u32, u32, u32, u32, u32)),
    ]
}
