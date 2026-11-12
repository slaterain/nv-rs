//! The particles family (the update half's second group, stage 1): a particle system's update, from
//! its update controller down to the modifiers that move, age, turn, size, colour and bound the
//! particles. The systems sit in the actors' and the world's scene graphs: their controllers run in
//! the downward update pass (the typed controller walk calls the update controller directly), most of
//! them from the queued node updates main runs after the frame is shown. About 20 systems a frame in
//! the crowded Goodsprings street, a few hundred particles each (the profile of 2026-11-10).
//!
//! - [`system`]: `NiPSysUpdateCtlr::Update`, `NiParticleSystem::UpdateSystem` and `Do_UpdateSystem`,
//!   which runs each active modifier's `Update` (vtable +0x8c) with the time and the particle data;
//! - [`modifiers`]: the modifiers' `Update`s typed here (bomb, position, age and death with the data's
//!   particle removal, rotation, grow and fade, bound update, Bethesda's simple colour). Another
//!   modifier (gravity, a mesh or strip one, a mod's) goes through its vtable as before.
//!
//! Stage 1: the objects stay in guest memory where the game made them. Each function stays registered
//! at its exe address; a call the exe makes through a vtable slot is a `typed_call!` when the slot holds
//! the typed function ([`crate::slot_target`]), and goes through the vtable otherwise.
//!
//! **Floats.** The systems update inside the scene graph's downward pass, so they run wherever the
//! scene family's pass runs (the main thread, and the AI threads for the actors' animated scenes): the
//! row is [`Threads::Both`]. The family's float maths is native (ADR-0014; [`crate::native::Real`]): each
//! function is written once and runs as plain `f32` on main (the exe's 24-bit precision there rounds every
//! operation to a float) and as `f64` locals rounded at each float store on the AI threads (53 bits: the
//! exe's own arithmetic, bit for bit), chosen once per call from the thread's control word. The position
//! modifier, every step of which is one operation on two floats stored as a float, is plain `f32` on every
//! thread: one operation rounds to the same float at either precision. Operations stay
//! in the exe's order; the square roots are `FSQRT`'s, not the CRT's; comparisons are NaN-aware as the
//! exe's flag tests; floats the exe only moves through the x87 are copied plain (only a signalling NaN's
//! payload could show the difference). No x87 helper is left in the family. Float fields are declared as
//! their bits, as the code moves them.

use crate::culling::layout::{
    NiBound, NiBoundImage, NiGeometry, NiGeometryImage, NiPoint3, NiPoint3Image,
};
use crate::image;
use engine_core::ptr::{Inline, Ptr};
use engine_core::typed::{Family, FamilyId, Threads};
use engine_core::{entry, AbiFn};

pub mod modifiers;
pub mod system;

/// This family's bit: what each `typed_call!` of it names.
pub const FAMILY: FamilyId = FamilyId::new(20);

/// `NV_TYPED=particles`. Proved by its differential tests (each planted mutant caught) and the
/// byte-identical recorded street run in both presets (on against `NV_TYPED=particles:off,scene:off`,
/// the queued node updates being the scene family's), so on unless `NV_TYPED` says otherwise.
pub static PARTICLES: Family = Family {
    name: "particles",
    id: FAMILY,
    past_gate: true,
    threads: Threads::Both,
    stage2: false,
    funcs,
};

image! {
    /// `NiTimeController` (Xbox PDB), 0x34 bytes on PC as on the Xbox.
    pub struct NiTimeController / NiTimeControllerImage: 0x34 {
        0x00 vtable: u32,
        /// `NiRefObject::m_uiRefCount` (Xbox PDB).
        0x04 m_uiRefCount: u32,
        /// `m_uFlags` (Xbox PDB).
        0x08 m_uFlags: u16,
        0x0a padding_0a: u16,
        /// `m_fFrequency` (Xbox PDB), the float's bits; and so the times after it.
        0x0c m_fFrequency: u32,
        /// `m_fPhase` (Xbox PDB).
        0x10 m_fPhase: u32,
        /// `m_fLoKeyTime` (Xbox PDB).
        0x14 m_fLoKeyTime: u32,
        /// `m_fHiKeyTime` (Xbox PDB).
        0x18 m_fHiKeyTime: u32,
        /// `m_fStartTime` (Xbox PDB).
        0x1c m_fStartTime: u32,
        /// `m_fLastTime` (Xbox PDB).
        0x20 m_fLastTime: u32,
        /// `m_fWeightedLastTime` (Xbox PDB).
        0x24 m_fWeightedLastTime: u32,
        /// `m_fScaledTime` (Xbox PDB).
        0x28 m_fScaledTime: u32,
        /// `m_pkTarget` (Xbox PDB): the controlled `NiObjectNET`.
        0x2c m_pkTarget: u32,
        /// `m_spNext` (Xbox PDB): the next controller.
        0x30 m_spNext: u32,
    }

    /// `NiParticleSystem` (Xbox PDB), as far as its update reads it (0x110 bytes on PC).
    pub struct NiParticleSystem / NiParticleSystemImage: 0xdc {
        0x00 base: Inline<NiGeometry> = NiGeometryImage,
        0xc0 unknown_c0: u32,
        /// `m_bWorldSpace` (Xbox PDB).
        0xc4 m_bWorldSpace: u8,
        0xc5 padding_c5: [u8; 3],
        /// `m_kModifierList` (Xbox PDB): an `NiTPointerList`, its first item at +0 (each item: the
        /// next at +0, the modifier at +8).
        0xc8 m_kModifierList: [u32; 3],
        /// `m_fLastTime` (Xbox PDB), the float's bits.
        0xd4 m_fLastTime: u32,
        /// `m_bResetSystem` (Xbox PDB).
        0xd8 m_bResetSystem: u8,
        /// `m_bDynamicBounds` (Xbox PDB).
        0xd9 m_bDynamicBounds: u8,
        0xda padding_da: [u8; 2],
    }

    /// `NiPSysData` (Xbox PDB), 0x7c bytes: `NiGeometryData`'s fields to +0x40, then
    /// `NiParticlesData`'s, then its own.
    pub struct NiPSysData / NiPSysDataImage: 0x7c {
        0x00 vtable: u32,
        0x04 m_uiRefCount: u32,
        /// `NiGeometryData::m_usVertices` (Xbox PDB): the particle capacity.
        0x08 m_usVertices: u16,
        0x0a unknown_0a: u16,
        0x0c unknown_0c: u32,
        /// `NiGeometryData::m_kBound` (Xbox PDB): the model-space bound.
        0x10 m_kBound: Inline<NiBound> = NiBoundImage,
        /// `NiGeometryData::m_pkVertex` (Xbox PDB): the particles' positions (`NiPoint3`s).
        0x20 m_pkVertex: u32,
        /// `NiGeometryData::m_pkNormal` (Xbox PDB).
        0x24 m_pkNormal: u32,
        /// `NiGeometryData::m_pkColor` (Xbox PDB): `NiColorA`s, 16 bytes each.
        0x28 m_pkColor: u32,
        0x2c unknown_2c: [u32; 8],
        /// `NiParticlesData::m_pfRadii` (Xbox PDB).
        0x4c m_pfRadii: u32,
        /// `NiParticlesData::m_usActive` (Xbox PDB): the live particles.
        0x50 m_usActive: u16,
        0x52 unknown_52: u16,
        /// `NiParticlesData::m_pfSizes` (Xbox PDB).
        0x54 m_pfSizes: u32,
        /// `NiParticlesData::m_pkRotations` (Xbox PDB).
        0x58 m_pkRotations: u32,
        /// `NiParticlesData::m_pfRotationAngles` (Xbox PDB).
        0x5c m_pfRotationAngles: u32,
        /// `NiParticlesData::m_pkRotationAxes` (Xbox PDB).
        0x60 m_pkRotationAxes: u32,
        /// `NiParticlesData::m_pucTextureIndices` (Xbox PDB).
        0x64 m_pucTextureIndices: u32,
        /// `NiParticlesData::m_pkSubTextureOffsets` (Xbox PDB).
        0x68 m_pkSubTextureOffsets: u32,
        /// `NiParticlesData::m_ucSubTextureOffsetCount` (Xbox PDB).
        0x6c m_ucSubTextureOffsetCount: u8,
        0x6d padding_6d: [u8; 3],
        /// `m_pkParticleInfo` (Xbox PDB): an [`NiParticleInfo`] per particle.
        0x70 m_pkParticleInfo: u32,
        /// `m_pfRotationSpeeds` (Xbox PDB).
        0x74 m_pfRotationSpeeds: u32,
        /// `m_usNumAddedParticles` (Xbox PDB).
        0x78 m_usNumAddedParticles: u16,
        /// `m_usAddedParticlesBase` (Xbox PDB).
        0x7a m_usAddedParticlesBase: u16,
    }

    /// `NiParticleInfo` (Xbox PDB), 0x1c bytes, one per particle; floats as their bits.
    pub struct NiParticleInfo / NiParticleInfoImage: 0x1c {
        /// `m_kVelocity` (Xbox PDB).
        0x00 m_kVelocity: [u32; 3],
        /// `m_fAge` (Xbox PDB).
        0x0c m_fAge: u32,
        /// `m_fLifeSpan` (Xbox PDB).
        0x10 m_fLifeSpan: u32,
        /// `m_fLastUpdate` (Xbox PDB).
        0x14 m_fLastUpdate: u32,
        /// `m_usGeneration` (Xbox PDB).
        0x18 m_usGeneration: u16,
        /// `m_usCode` (Xbox PDB).
        0x1a m_usCode: u16,
    }

    /// `NiPSysModifier` (Xbox PDB), 0x18 bytes.
    pub struct NiPSysModifier / NiPSysModifierImage: 0x18 {
        0x00 vtable: u32,
        0x04 m_uiRefCount: u32,
        /// `m_kName` (Xbox PDB): an `NiFixedString` handle.
        0x08 m_kName: u32,
        /// `m_uiOrder` (Xbox PDB).
        0x0c m_uiOrder: u32,
        /// `m_pkTarget` (Xbox PDB): the `NiParticleSystem`.
        0x10 m_pkTarget: u32,
        /// `m_bActive` (Xbox PDB).
        0x14 m_bActive: u8,
        0x15 padding_15: [u8; 3],
    }

    /// `NiPSysBombModifier` (Xbox PDB), 0x38 bytes.
    pub struct NiPSysBombModifier / NiPSysBombModifierImage: 0x38 {
        0x00 base: Inline<NiPSysModifier> = NiPSysModifierImage,
        /// `m_pkBombObj` (Xbox PDB): an `NiAVObject`.
        0x18 m_pkBombObj: u32,
        /// `m_kBombAxis` (Xbox PDB).
        0x1c m_kBombAxis: Inline<NiPoint3> = NiPoint3Image,
        /// `m_fDecay` (Xbox PDB).
        0x28 m_fDecay: u32,
        /// `m_fDeltaV` (Xbox PDB).
        0x2c m_fDeltaV: u32,
        /// `m_eDecayType` (Xbox PDB): 0 none, 1 linear, 2 exponential.
        0x30 m_eDecayType: u32,
        /// `m_eSymmType` (Xbox PDB): 0 spherical, 1 cylindrical, 2 planar.
        0x34 m_eSymmType: u32,
    }

    /// `NiPSysAgeDeathModifier` (Xbox PDB), 0x20 bytes.
    pub struct NiPSysAgeDeathModifier / NiPSysAgeDeathModifierImage: 0x20 {
        0x00 base: Inline<NiPSysModifier> = NiPSysModifierImage,
        /// `m_bSpawnOnDeath` (Xbox PDB).
        0x18 m_bSpawnOnDeath: u8,
        0x19 padding_19: [u8; 3],
        /// `m_pkSpawnModifier` (Xbox PDB).
        0x1c m_pkSpawnModifier: u32,
    }

    /// `NiPSysGrowFadeModifier` (Xbox PDB), 0x2c bytes.
    pub struct NiPSysGrowFadeModifier / NiPSysGrowFadeModifierImage: 0x2c {
        0x00 base: Inline<NiPSysModifier> = NiPSysModifierImage,
        /// `m_fGrowTime` (Xbox PDB).
        0x18 m_fGrowTime: u32,
        /// `m_usGrowGeneration` (Xbox PDB).
        0x1c m_usGrowGeneration: u16,
        0x1e padding_1e: u16,
        /// `m_fFadeTime` (Xbox PDB).
        0x20 m_fFadeTime: u32,
        /// `m_usFadeGeneration` (Xbox PDB).
        0x24 m_usFadeGeneration: u16,
        0x26 padding_26: u16,
        /// `m_fBaseScale` (Xbox PDB).
        0x28 m_fBaseScale: u32,
    }

    /// `NiPSysBoundUpdateModifier` (Xbox PDB), 0x20 bytes.
    pub struct NiPSysBoundUpdateModifier / NiPSysBoundUpdateModifierImage: 0x20 {
        0x00 base: Inline<NiPSysModifier> = NiPSysModifierImage,
        /// `m_sUpdateSkip` (Xbox PDB).
        0x18 m_sUpdateSkip: i16,
        /// `m_usUpdateCount` (Xbox PDB).
        0x1a m_usUpdateCount: u16,
        /// `m_pkSkipBounds` (Xbox PDB): an `NiBound` per skip step.
        0x1c m_pkSkipBounds: u32,
    }

    /// `BSPSysSimpleColorModifier` (Xbox PDB), 0x60 bytes: three colours (`NiColorA`) and the age
    /// fractions where they start and end.
    pub struct BSPSysSimpleColorModifier / BSPSysSimpleColorModifierImage: 0x60 {
        0x00 base: Inline<NiPSysModifier> = NiPSysModifierImage,
        /// The three colours, red, green, blue and alpha each.
        0x18 colors: [u32; 12],
        /// The fade-in and fade-out fractions.
        0x48 fade_in: u32,
        0x4c fade_out: u32,
        /// Where colour 1 ends, colour 2 starts and ends, colour 3 starts.
        0x50 color1_end: u32,
        0x54 color2_start: u32,
        0x58 color2_end: u32,
        0x5c color3_start: u32,
    }
}

/// The family's functions, by exe address.
#[rustfmt::skip]
fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00c2_71c0, system::update_ctlr(Ptr<NiTimeController>, Ptr)),
        entry!(0x00c1_ae40, system::update_system(Ptr<NiParticleSystem>, f32)),
        entry!(0x00c1_b260, system::do_update_system(Ptr<NiParticleSystem>, f32)),
        entry!(0x00c2_e670, modifiers::bomb(Ptr<NiPSysBombModifier>, f32, Ptr<NiPSysData>)),
        entry!(0x00c2_8bc0, modifiers::position(Ptr, f32, Ptr<NiPSysData>)),
        entry!(0x00c2_f010, modifiers::age_death(Ptr<NiPSysAgeDeathModifier>, f32, Ptr<NiPSysData>)),
        entry!(0x00c2_4cc0, modifiers::remove_particle(Ptr<NiPSysData>, u32)),
        entry!(0x00c2_06b0, modifiers::rotation(Ptr, f32, Ptr<NiPSysData>)),
        entry!(0x00c2_a390, modifiers::grow_fade(Ptr<NiPSysGrowFadeModifier>, f32, Ptr<NiPSysData>)),
        entry!(0x00c2_df20, modifiers::bound_update(Ptr<NiPSysBoundUpdateModifier>, f32, Ptr<NiPSysData>)),
        entry!(0x00c2_d720, modifiers::set_update_skip(Ptr<NiPSysBoundUpdateModifier>, u32)),
        entry!(0x00c6_02e0, modifiers::simple_color(Ptr<BSPSysSimpleColorModifier>, u32, Ptr<NiPSysData>)),
    ]
}
