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
//! animations are loaded and updated, not only on the main thread: [`Threads::Both`], and the x87
//! code keeps the exact helpers ([`engine_core::x87`]): each load, operation and store the exe's, in
//! its order, as the translations have them. Float fields are declared as their bits.

use crate::image;
use engine_core::ptr::Ptr;
use engine_core::typed::{Family, FamilyId, Threads};
use engine_core::x87::x87_load32;
use engine_core::{entry, AbiFn, Engine};

pub mod quaternion;
pub mod rot_keys;

/// This family's bit: what each `typed_call!` of it names.
pub const FAMILY: FamilyId = FamilyId::new(27);

/// `NV_TYPED=anim_keys`. Off until its gate (the recorded run, both presets, byte-identical with it
/// on against off): its differential tests, planted mutants and review are what it merges on.
pub static ANIM_KEYS: Family = Family {
    name: "anim_keys",
    id: FAMILY,
    past_gate: false,
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

    /// Key `k`'s time as `FLD dword` loads it.
    pub fn time(self, e: &Engine, k: u32) -> f64 {
        x87_load32(self.word(e, k, 0))
    }
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
