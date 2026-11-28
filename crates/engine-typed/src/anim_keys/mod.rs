//! The animation keys family (stage 1): the code that reads and writes the key arrays an animation
//! keeps (`NiTransformData`'s rotation, position and scale keys, `NiFloatData`'s keys), allocated
//! while `LoadBinary` runs. They hold about 12 MB on the smoothness route (BULK.md 2.1), all in the
//! heap's address space, and they are what stage 2 moves out of it next (NEXT-FAMILY.md), as plain
//! byte buffers read by offset. Every animated bone's transform comes from them every frame:
//!
//! - [`search`]: the key search the `GenInterp` functions share and the ratio between its two keys;
//! - [`float_keys`]: `NiFloatKey::GenInterp` and the float key types' interpolations (linear,
//!   Bezier, TCB, step), with the cubic Hermite value and the linear blend they share;
//! - [`pos_keys`]: `NiPosKey::GenInterp` and the position key types' interpolations (none, linear,
//!   Bezier, TCB);
//! - [`rot_keys`]: `NiRotKey::GenInterp`, the rotation key types' interpolations (identity, linear,
//!   Bezier, TCB, step) and the preparation of a rotation key array (neighbours in one hemisphere,
//!   `W` clamped);
//! - [`quaternion`]: `NiQuaternion`'s constructor from four floats, its negation, `Slerp`, the
//!   parameter correction `Slerp` makes, `FastNormalize` and the spherical quadrangle
//!   interpolation.
//!
//! Each `GenInterp` does a known key type's interpolation in place (a table word holding the
//! function this family has for it), with no call; so is the spherical quadrangle interpolation the
//! Bezier and TCB rotation keys use. What stays by address: the Euler rotation keys (`FSINCOS`), a
//! table word this family does not know, and every other user of the keys (`LoadBinary`, the stream
//! reads that fill the arrays, the colour, boolean and text keys).
//!
//! A key array is `count` keys of `size` bytes (the size the key type has, handed in as a byte),
//! each starting with its float time; a float key's value, a position key's point or a rotation
//! key's quaternion follows at +4. [`Keys`] gives a key's address, word and time.
//!
//! Stage 1: the keys stay in guest memory where the game made them. **Floats.** These run wherever
//! animations are loaded and updated, the AI threads and main: [`Threads::Both`]. What runs every
//! frame (the ratios, the interpolations, `Slerp`, its correction and `FastNormalize`) is native
//! maths at the running thread's precision (`crate::native::Real`: `f32` on main, 24 bits, where
//! every x87 operation rounds to a float; `f64` on the AI threads, 53 bits, rounded to a float only
//! where the exe stores one), so it is the exe's arithmetic on both, NaN payloads aside (an x87
//! load quiets a signalling NaN). The rest (the constructor, the negation, the preparation's dot
//! products and clamps, which run when keys are loaded) keeps the exact helpers
//! ([`engine_core::x87`]). Float fields are declared as their bits.

use crate::image;
use engine_core::ptr::Ptr;
use engine_core::typed::{Family, FamilyId, Threads};
use engine_core::{entry, AbiFn, Engine};

pub mod float_keys;
pub mod pos_keys;
pub mod quaternion;
pub mod rot_keys;
pub mod search;

/// This family's bit: what each `typed_call!` of it names.
pub const FAMILY: FamilyId = FamilyId::new(27);

/// `NV_TYPED=anim_keys:off` turns it off. On by default since 2026-11-11, on its differential
/// tests, planted mutants and review (no comparison run for native code, AGENT-RULES.md).
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
        entry!(0x00a4_ff00, quaternion::fast_normalize(Ptr)),
        entry!(0x00a6_e430, quaternion::squad(Ptr<NiQuaternion>, f32, Ptr<NiQuaternion>, Ptr<NiQuaternion>, Ptr<NiQuaternion>, Ptr<NiQuaternion>)),
        entry!(0x00a2_86e0, rot_keys::identity(f32, Ptr, Ptr, Ptr)),
        entry!(0x00a2_ab90, rot_keys::linear(f32, Ptr, Ptr, Ptr) -> u32),
        entry!(0x00a2_8d90, rot_keys::bezier(f32, Ptr, Ptr, Ptr)),
        entry!(0x00a2_bc60, rot_keys::tcb(f32, Ptr, Ptr, Ptr) -> u32),
        entry!(0x00a2_cf20, rot_keys::step(f32, Ptr, Ptr, Ptr)),
        entry!(0x00a2_6b40, float_keys::gen_interp(f32, Ptr, u32, u32, Ptr, u8) -> f64),
        entry!(0x00a4_2050, float_keys::lerp(f32, f32, f32) -> f64),
        entry!(0x00a4_2070, float_keys::hermite(f32, f32, f32, f32, f32) -> f64),
        entry!(0x00a2_a560, float_keys::linear(f32, Ptr, Ptr, Ptr)),
        entry!(0x00a2_6fe0, float_keys::bezier(f32, Ptr, Ptr, Ptr)),
        entry!(0x00a2_c610, float_keys::tcb(f32, Ptr, Ptr, Ptr)),
        entry!(0x00a2_d200, float_keys::step(f32, Ptr, Ptr, Ptr)),
        entry!(0x00a2_7490, pos_keys::gen_interp(Ptr, f32, Ptr, u32, u32, Ptr, u8) -> Ptr),
        entry!(0x00a2_7470, pos_keys::none(f32, Ptr, Ptr, Ptr)),
        entry!(0x00a2_a7f0, pos_keys::linear(f32, Ptr, Ptr, Ptr)),
        entry!(0x00a2_7c80, pos_keys::bezier(f32, Ptr, Ptr, Ptr)),
        entry!(0x00a2_af70, pos_keys::tcb(f32, Ptr, Ptr, Ptr)),
    ]
}
