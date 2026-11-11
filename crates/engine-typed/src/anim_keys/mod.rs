//! The animation keys family (stage 1): the code that reads and writes the key arrays an animation
//! keeps (`NiTransformData`'s rotation, position and scale keys, allocated while `LoadBinary` runs).
//! They hold about 12 MB on the smoothness route (BULK.md 2.1), all in the heap's address space, and
//! they are what stage 2 moves out of it next (NEXT-FAMILY.md), as plain byte buffers read by offset.
//! This first batch types the functions the touch census saw reaching the keys in Doc's house that
//! work on rotation keys and their quaternions:
//!
//! - [`quaternion`]: `NiQuaternion`'s constructor from four floats, its negation, `Slerp` and the
//!   parameter correction `Slerp` makes;
//! - [`rot_keys`]: `NiRotKey::GenInterp` (the key search and the key type's interpolation) and the
//!   preparation of a rotation key array (neighbours in one hemisphere, `W` clamped).
//!
//! What stays by address: the key types' interpolation functions (the table at `011f3cc0`),
//! `NiQuaternion::FastNormalize` (`00a4ff00`), and every other user of the keys (`LoadBinary`, the
//! stream reads that fill the arrays, the other key types) until a later batch types them.
//!
//! A key array is `count` keys of `size` bytes (the size the key type has, handed in as a byte), each
//! starting with its float time; a rotation key's quaternion follows at +4. [`Keys`] gives a key's
//! address, word and time. Not everything goes through it yet, which stage 2 has to change: the
//! copies of a key's quaternion (`GenInterp`'s first key, the preparation's negation copied
//! back) and the clamp of `W` read and write at those addresses with `e.mem`, and `GenInterp` hands
//! raw key addresses to the interpolation functions it calls by address.
//!
//! Stage 1: the keys stay in guest memory where the game made them. **Floats.** These run wherever
//! animations are loaded and updated, not only on the main thread: [`Threads::Both`]. What runs for
//! every animated bone every frame (`Slerp`, its parameter correction, and the key search and ratio
//! of `GenInterp`) is native maths by ADR-0014's float rule for such code: `f64` locals, each
//! operation in the exe's order, rounded to a float where the exe stores one ([`wide`], [`narrow`],
//! [`rounded`]). On the game's other threads (53-bit x87 precision) that is the exe's arithmetic
//! exactly, NaN payloads aside; on the main thread (24-bit precision) the exe rounds every operation
//! to 24 bits, and the results may differ in their last bits. The rest (the constructor, the
//! negation, the preparation's dot products and clamps, which run when keys are loaded) keeps the
//! exact helpers ([`engine_core::x87`]). Float fields are declared as their bits.

use crate::image;
use engine_core::ptr::Ptr;
use engine_core::typed::{Family, FamilyId, Threads};
use engine_core::{entry, AbiFn, Engine};

pub mod quaternion;
pub mod rot_keys;

/// This family's bit: what each `typed_call!` of it names.
pub const FAMILY: FamilyId = FamilyId::new(27);

/// `NV_TYPED=anim_keys:off` turns it off. On by default since 2026-11-11, on its differential tests,
/// planted mutants and review (no comparison run for native code, AGENT-RULES.md).
pub static ANIM_KEYS: Family = Family {
    name: "anim_keys",
    id: FAMILY,
    past_gate: true,
    threads: Threads::Both,
    stage2: false,
    funcs,
};

image! {
    /// `NiQuaternion` (Xbox PDB), 0x10 bytes; the PC offsets match.
    pub struct NiQuaternion / NiQuaternionImage: 0x10 {
        /// `m_fW` (Xbox PDB).
        0x00 m_fW: u32,
        /// `m_fX` (Xbox PDB).
        0x04 m_fX: u32,
        /// `m_fY` (Xbox PDB).
        0x08 m_fY: u32,
        /// `m_fZ` (Xbox PDB).
        0x0c m_fZ: u32,
    }
}

/// A key array: where its first key is and how far apart the keys are (the key type's size, the low
/// byte the code is handed). Stage 1 reads and writes the keys in guest memory, word by word.
#[derive(Clone, Copy, Debug)]
pub struct Keys {
    base: u32,
    size: u32,
}

impl Keys {
    pub fn new(base: u32, size: u32) -> Keys {
        Keys { base, size }
    }

    /// The address of key `k` (wrapping as the exe's index arithmetic does).
    pub fn at(self, k: u32) -> u32 {
        self.base.wrapping_add(k.wrapping_mul(self.size))
    }

    /// The word at `offset` in key `k`.
    pub fn word(self, e: &Engine, k: u32, offset: u32) -> u32 {
        e.mem.u32(self.at(k).wrapping_add(offset))
    }

    /// Key `k`'s time, widened to a native `f64` local.
    pub fn time(self, e: &Engine, k: u32) -> f64 {
        wide(self.word(e, k, 0))
    }
}

/// A `float` word as native code holds it: an `f64` local (ADR-0014's float rule for code that runs
/// on any thread). Every float is exact in an `f64`.
#[inline(always)]
pub fn wide(bits: u32) -> f64 {
    f64::from(f32::from_bits(bits))
}

/// An `f64` local rounded to the `float` that a store of it makes.
#[inline(always)]
#[allow(clippy::cast_possible_truncation)]
pub fn narrow(value: f64) -> f32 {
    value as f32
}

/// An `f64` local rounded as a store to a `float` and a load back round it.
#[inline(always)]
pub fn rounded(value: f64) -> f64 {
    f64::from(narrow(value))
}

/// The family's functions, by exe address.
#[rustfmt::skip]
fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00a6_db50, quaternion::from_floats(Ptr<NiQuaternion>, f32, f32, f32, f32) -> Ptr<NiQuaternion>),
        entry!(0x00a6_dbd0, quaternion::negate(Ptr<NiQuaternion>, Ptr<NiQuaternion>) -> Ptr<NiQuaternion>),
        entry!(0x00a6_dc30, quaternion::counter_warp(f32, f32) -> f64),
        entry!(0x00a6_e330, quaternion::slerp(Ptr<NiQuaternion>, f32, Ptr<NiQuaternion>, Ptr<NiQuaternion>)),
        entry!(0x00a2_8740, rot_keys::gen_interp(Ptr<NiQuaternion>, f32, Ptr, u32, u32, Ptr, u8) -> Ptr<NiQuaternion>),
        entry!(0x00a2_8ab0, rot_keys::prepare(Ptr, u32, u8)),
    ]
}
