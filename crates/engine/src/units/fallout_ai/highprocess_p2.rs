//! `fallout/ai/highprocess.cpp` (Xbox PDB source unit), part 2: its functions from `008d8720` up to
//! (not including) `008d9780` in FalloutNV.exe 1.4.0.525, translated
//! (docs/ENGINE_CRATE.md). The unit's shared layouts and helpers are in
//! [`super::highprocess`]; anything public there may be used here.
//!
//! Like the unit's first block, these are the process classes' one-field
//! accessors. The `MiddleHighProcess` ones use the Xbox PDB field names and
//! PC offsets (checked against the code); the doc comment of each names the
//! field and offset. Byte getters return the raw byte (`MOV AL`).

#[allow(unused_imports)]
use super::highprocess::*;
#[allow(unused_imports)]
use crate::prelude::*;

/// By-value removal from a `BSSimpleList` (`00905330`): `this` is the list
/// head, the argument the address of a word holding the item.
const LIST_REMOVE_ITEM: u32 = 0x0090_5330;
/// `NiPointer` getter (`00559450`): the first word of `this`.
const NI_POINTER_GET: u32 = 0x0055_9450;

// Translated from 008d8720 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::SetIsSummonedCreature` (Xbox PDB) stores its argument in `bSummonedCreature` at +0x18b.
pub fn middle_high_process_set_is_summoned_creature(e: &mut Engine, this: Ptr, value: u8) {
    e.mem.set_u8(this.addr() + 0x18b, value);
}

// Translated from 008d87e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::GetAll3DUpdateFlags` (Xbox PDB) returns `cUpdate3DModel` at +0x18c.
pub fn middle_high_process_get_all_3d_update_flags(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr() + 396)
}

// Translated from 008d8800 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::GetAttachedArrowList` (Xbox PDB) returns `pAttachedArrowList (BSSimpleList<ArrowProjectile *>*)` at +0x1ac.
pub fn middle_high_process_get_attached_arrow_list(e: &mut Engine, this: Ptr) -> Ptr {
    Ptr::new(e.mem.u32(this.addr() + 428))
}

// Translated from 008d8820 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::SetIronSights` (Xbox PDB) stores its argument in `bIronSights` at +0x228.
pub fn middle_high_process_set_iron_sights(e: &mut Engine, this: Ptr, value: u8) {
    e.mem.set_u8(this.addr() + 0x228, value);
}

// Translated from 008d8840 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::GetIronSights` (Xbox PDB) returns `bIronSights` at +0x228.
pub fn middle_high_process_get_iron_sights(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr() + 552)
}

// Translated from 008d8860 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::IsSustainedFire` (Xbox PDB) returns `bSustainedFire` at +0x1d9.
pub fn middle_high_process_is_sustained_fire(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr() + 473)
}

// Translated from 008d8880 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::SetSustainedFire` (Xbox PDB) stores its argument in `bSustainedFire` at +0x1d9.
pub fn middle_high_process_set_sustained_fire(e: &mut Engine, this: Ptr, value: u8) {
    e.mem.set_u8(this.addr() + 0x1d9, value);
}

// Translated from 008d88a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::SetBeginIdlesPlayed` (Xbox PDB) stores its argument in `bPlayedBeginIdles` at +0x19c.
pub fn middle_high_process_set_begin_idles_played(e: &mut Engine, this: Ptr, value: u8) {
    e.mem.set_u8(this.addr() + 0x19c, value);
}

// Translated from 008d88c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::GetBeginIdlesPlayed` (Xbox PDB) returns `bPlayedBeginIdles` at +0x19c.
pub fn middle_high_process_get_begin_idles_played(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr() + 412)
}

// Translated from 008d88e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::SetEndIdlesPlayed` (Xbox PDB) stores its argument in `bPlayedEndIdles` at +0x19d.
pub fn middle_high_process_set_end_idles_played(e: &mut Engine, this: Ptr, value: u8) {
    e.mem.set_u8(this.addr() + 0x19d, value);
}

// Translated from 008d8900 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::GetEndIdlesPlayed` (Xbox PDB) returns `bPlayedEndIdles` at +0x19d.
pub fn middle_high_process_get_end_idles_played(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr() + 413)
}

// Translated from 008d8920 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::GetCreatureLipSynchAnim` (Xbox PDB) returns `pCreatureLipsynchAnim (LipSynchAnim*)` at +0x1a0.
pub fn middle_high_process_get_creature_lip_synch_anim(e: &mut Engine, this: Ptr) -> Ptr {
    Ptr::new(e.mem.u32(this.addr() + 416))
}

// Translated from 008d8940 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::GetCreatureLipSynchStartTime` (Xbox PDB) returns `iCreatureLipsynchStartTime` at +0x1a4.
pub fn middle_high_process_get_creature_lip_synch_start_time(e: &mut Engine, this: Ptr) -> u32 {
    e.mem.u32(this.addr() + 420)
}

// Translated from 008d89c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::GetRadiationMagicDelta` (Xbox PDB) returns `fRadiationMagicDelta` at +0x238.
pub fn middle_high_process_get_radiation_magic_delta(e: &mut Engine, this: Ptr) -> f32 {
    e.mem.f32(this.addr() + 568)
}

// Translated from 008d89e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::SetRadiationWaterDelta` (Xbox PDB) stores its argument in `fRadiationWaterDelta` at +0x23c.
pub fn middle_high_process_set_radiation_water_delta(e: &mut Engine, this: Ptr, value: f32) {
    e.mem.set_f32(this.addr() + 0x23c, value);
}

// Translated from 008d8a00 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::GetRadiationWaterDelta` (Xbox PDB) returns `fRadiationWaterDelta` at +0x23c.
pub fn middle_high_process_get_radiation_water_delta(e: &mut Engine, this: Ptr) -> f32 {
    e.mem.f32(this.addr() + 572)
}

// Translated from 008d8a20 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::SetRadiationDelta` (Xbox PDB) stores its argument in `fRadiationDelta` at +0x234.
pub fn middle_high_process_set_radiation_delta(e: &mut Engine, this: Ptr, value: f32) {
    e.mem.set_f32(this.addr() + 0x234, value);
}

// Translated from 008d8a40 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::GetLastAttackHitData` (Xbox PDB) returns `pLastAttackHitData (HitData*)` at +0x254.
pub fn middle_high_process_get_last_attack_hit_data(e: &mut Engine, this: Ptr) -> Ptr {
    Ptr::new(e.mem.u32(this.addr() + 596))
}

// Translated from 008d8a60 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::GetWeaponConditionStage` (Xbox PDB) returns `iWeaponConditionStage` at +0x244.
pub fn middle_high_process_get_weapon_condition_stage(e: &mut Engine, this: Ptr) -> i32 {
    e.mem.u32(this.addr() + 580) as i32
}

// Translated from 008d8a80 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::SetWeaponConditionStage` (Xbox PDB) stores its argument in `iWeaponConditionStage` at +0x244.
pub fn middle_high_process_set_weapon_condition_stage(e: &mut Engine, this: Ptr, value: i32) {
    e.mem.set_u32(this.addr() + 0x244, value as u32);
}

// Translated from 008d8aa0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::SetFaceNode` (Xbox PDB) stores its argument in `pFaceNode (BSFaceGenNiNode*)` at +0x248.
pub fn middle_high_process_set_face_node(e: &mut Engine, this: Ptr, value: Ptr) {
    e.mem.set_u32(this.addr() + 0x248, value.addr());
}

// Translated from 008d8ac0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::GetFaceSkinnedNode` (Xbox PDB) returns `pFaceNodeSkinned (BSFaceGenNiNode*)` at +0x24c.
pub fn middle_high_process_get_face_skinned_node(e: &mut Engine, this: Ptr) -> Ptr {
    Ptr::new(e.mem.u32(this.addr() + 588))
}

// Translated from 008d8ae0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::SetFaceSkinnedNode` (Xbox PDB) stores its argument in `pFaceNodeSkinned (BSFaceGenNiNode*)` at +0x24c.
pub fn middle_high_process_set_face_skinned_node(e: &mut Engine, this: Ptr, value: Ptr) {
    e.mem.set_u32(this.addr() + 0x24c, value.addr());
}

// Translated from 008d8b00 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::GetHeadAnims` (Xbox PDB) returns `pAnimFace (NiAVObject*)` at +0x250.
pub fn middle_high_process_get_head_anims(e: &mut Engine, this: Ptr) -> Ptr {
    Ptr::new(e.mem.u32(this.addr() + 592))
}

// Translated from 008d8b80 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::GetCurrentFurniture` (Xbox PDB) returns `pCurrentFurniture (TESObjectREFR*)` at +0x140.
pub fn middle_high_process_get_current_furniture(e: &mut Engine, this: Ptr) -> Ptr {
    Ptr::new(e.mem.u32(this.addr() + 320))
}

// Translated from 008d8be0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::GetCurrentFurnitureIndex` (Xbox PDB) returns `cCurrentFurnitureIndex` at +0x144.
pub fn middle_high_process_get_current_furniture_index(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr() + 324)
}

// Translated from 008d8c20 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::GetTorsoNode` (Xbox PDB) returns `pTorsoNode (NiAVObject*)` at +0x21c.
pub fn middle_high_process_get_torso_node(e: &mut Engine, this: Ptr) -> Ptr {
    Ptr::new(e.mem.u32(this.addr() + 540))
}

// Translated from 008d8c40 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::SetAnimActionSuccess` (Xbox PDB) stores its argument in `iAnimActionSuccess` at +0x22a.
pub fn middle_high_process_set_anim_action_success(e: &mut Engine, this: Ptr, value: i16) {
    e.mem.set_u16(this.addr() + 0x22a, value as u16);
}

// Translated from 008d8740 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::Set3DUpdateFlag` (Xbox PDB) ORs the mask into `cUpdate3DModel` at +0x18c.
pub fn middle_high_process_set_3d_update_flag(e: &mut Engine, this: Ptr, mask: u32) {
    let flags = e.mem.u8(this.addr() + 0x18c) as u32;
    e.mem.set_u8(this.addr() + 0x18c, (flags | mask) as u8);
}

// Translated from 008d8770 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::Clear3DUpdateFlag` (Xbox PDB) updates `cUpdate3DModel` at +0x18c. The code
/// multiplies the flags by the complemented mask (`IMUL`, not `AND`) and stores the low byte;
/// reproduced as the code does it.
pub fn middle_high_process_clear_3d_update_flag(e: &mut Engine, this: Ptr, mask: u32) {
    let flags = e.mem.u8(this.addr() + 0x18c) as u32;
    e.mem
        .set_u8(this.addr() + 0x18c, flags.wrapping_mul(!mask) as u8);
}

// Translated from 008d87a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::ClearAll3DUpdateFlags` (Xbox PDB) zeroes `cUpdate3DModel` at +0x18c.
pub fn middle_high_process_clear_all_3d_update_flags(e: &mut Engine, this: Ptr) {
    e.mem.set_u8(this.addr() + 0x18c, 0);
}

// Translated from 008d87c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::Get3DUpdateFlag` (Xbox PDB): whether any bit of the mask is set in
/// `cUpdate3DModel` at +0x18c.
pub fn middle_high_process_get_3d_update_flag(e: &mut Engine, this: Ptr, mask: u32) -> bool {
    (e.mem.u8(this.addr() + 0x18c) as u32 & mask) != 0
}

// Translated from 008d8960 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::AddRadiationMagicDelta` (Xbox PDB) adds its argument to
/// `fRadiationMagicDelta` at +0x238 (x87: computed in `f64`, stored as `f32`).
pub fn middle_high_process_add_radiation_magic_delta(e: &mut Engine, this: Ptr, delta: f32) {
    let current = e.mem.f32(this.addr() + 0x238) as f64;
    e.mem
        .set_f32(this.addr() + 0x238, (current + delta as f64) as f32);
}

// Translated from 008d8990 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::RemoveRadiationMagicDelta` (Xbox PDB) subtracts its argument from
/// `fRadiationMagicDelta` at +0x238 (x87: computed in `f64`, stored as `f32`).
pub fn middle_high_process_remove_radiation_magic_delta(e: &mut Engine, this: Ptr, delta: f32) {
    let current = e.mem.f32(this.addr() + 0x238) as f64;
    e.mem
        .set_f32(this.addr() + 0x238, (current - delta as f64) as f32);
}

// Translated from 008d8b20 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::GetSitSleepState` (Xbox PDB): `cSitSleepState` at +0x13d, plus one when
/// virtual slot 0x618 reports bit 3 (0x8) and the state is 4 or 9.
pub fn middle_high_process_get_sit_sleep_state(e: &mut Engine, this: Ptr) -> u32 {
    let flags = e.vcall(this.addr(), 0x618, &args![]).u32();
    let state = e.mem.u8(this.addr() + 0x13d) as u32;
    if flags & 0x8 != 0 && (state == 4 || state == 9) {
        state + 1
    } else {
        state
    }
}

// Translated from 008d8ba0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::ClearCurrentFurniture` (Xbox PDB): removes the reference from the list
/// embedded at +0xd0 (`00905330`, by value) and, if it is `pCurrentFurniture` (+0x140), clears that.
pub fn middle_high_process_clear_current_furniture(e: &mut Engine, this: Ptr, furniture: Ptr) {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), furniture.addr());
        e.call(LIST_REMOVE_ITEM, &args![this.byte_add(0xd0), slot]);
    });
    if e.mem.u32(this.addr() + 0x140) == furniture.addr() {
        e.mem.set_u32(this.addr() + 0x140, 0);
    }
}

// Translated from 008d8c00 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::GetLightingShaderProperty` (Xbox PDB): the
/// `NiPointer<BSShaderPPLightingProperty>` `pLightingProperty` at +0x220, read through the
/// pointer getter (`00559450`).
pub fn middle_high_process_get_lighting_shader_property(e: &mut Engine, this: Ptr) -> Ptr {
    e.call(NI_POINTER_GET, &args![this.byte_add(0x220)]).ptr()
}

// Translated from 008d8c60 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::RemoveSpokenToActor` (Xbox PDB): removes the actor from the list
/// `pLastSpokeToList` (+0x264, `BSSimpleList<Actor *>*`) with the by-value list removal
/// (`00905330`).
pub fn high_process_remove_spoken_to_actor(e: &mut Engine, this: Ptr<HighProcess>, actor: Ptr) {
    let list = e.get(this, HighProcess::pLastSpokeToList);
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), actor.addr());
        e.call(LIST_REMOVE_ITEM, &args![list, slot]);
    });
}

// Translated from 008d8c80 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::GetNumberGuardsArresting` (Xbox PDB) returns the word at +0x39c
/// (`iNumberGuardsPersuing` in the Xbox PDB layout).
pub fn high_process_get_number_guards_arresting(e: &mut Engine, this: Ptr<HighProcess>) -> u32 {
    e.get(this, HighProcess::iNumberGuardsPersuing)
}

// Translated from 008d8ca0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::ModNumberGuardsArresting` (Xbox PDB) adds its argument (wrapping) to the word at
/// +0x39c.
pub fn high_process_mod_number_guards_arresting(
    e: &mut Engine,
    this: Ptr<HighProcess>,
    delta: u32,
) {
    let count = e.get(this, HighProcess::iNumberGuardsPersuing);
    e.set(
        this,
        HighProcess::iNumberGuardsPersuing,
        count.wrapping_add(delta),
    );
}

/// This part's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x008d8740, middle_high_process_set_3d_update_flag(Ptr, u32)),
        entry!(
            0x008d8770,
            middle_high_process_clear_3d_update_flag(Ptr, u32)
        ),
        entry!(
            0x008d87a0,
            middle_high_process_clear_all_3d_update_flags(Ptr)
        ),
        entry!(0x008d87c0, middle_high_process_get_3d_update_flag(Ptr, u32) -> bool),
        entry!(
            0x008d8960,
            middle_high_process_add_radiation_magic_delta(Ptr, f32)
        ),
        entry!(
            0x008d8990,
            middle_high_process_remove_radiation_magic_delta(Ptr, f32)
        ),
        entry!(0x008d8b20, middle_high_process_get_sit_sleep_state(Ptr) -> u32),
        entry!(
            0x008d8ba0,
            middle_high_process_clear_current_furniture(Ptr, Ptr)
        ),
        entry!(0x008d8c00, middle_high_process_get_lighting_shader_property(Ptr) -> Ptr),
        entry!(
            0x008d8c60,
            high_process_remove_spoken_to_actor(Ptr<HighProcess>, Ptr)
        ),
        entry!(
            0x008d8c80,
            high_process_get_number_guards_arresting(Ptr<HighProcess>) -> u32
        ),
        entry!(
            0x008d8ca0,
            high_process_mod_number_guards_arresting(Ptr<HighProcess>, u32)
        ),
        entry!(
            0x008d8720,
            middle_high_process_set_is_summoned_creature(Ptr, u8)
        ),
        entry!(0x008d87e0, middle_high_process_get_all_3d_update_flags(Ptr) -> u8),
        entry!(0x008d8800, middle_high_process_get_attached_arrow_list(Ptr) -> Ptr),
        entry!(0x008d8820, middle_high_process_set_iron_sights(Ptr, u8)),
        entry!(0x008d8840, middle_high_process_get_iron_sights(Ptr) -> u8),
        entry!(0x008d8860, middle_high_process_is_sustained_fire(Ptr) -> u8),
        entry!(0x008d8880, middle_high_process_set_sustained_fire(Ptr, u8)),
        entry!(
            0x008d88a0,
            middle_high_process_set_begin_idles_played(Ptr, u8)
        ),
        entry!(0x008d88c0, middle_high_process_get_begin_idles_played(Ptr) -> u8),
        entry!(
            0x008d88e0,
            middle_high_process_set_end_idles_played(Ptr, u8)
        ),
        entry!(0x008d8900, middle_high_process_get_end_idles_played(Ptr) -> u8),
        entry!(0x008d8920, middle_high_process_get_creature_lip_synch_anim(Ptr) -> Ptr),
        entry!(0x008d8940, middle_high_process_get_creature_lip_synch_start_time(Ptr) -> u32),
        entry!(0x008d89c0, middle_high_process_get_radiation_magic_delta(Ptr) -> f32),
        entry!(
            0x008d89e0,
            middle_high_process_set_radiation_water_delta(Ptr, f32)
        ),
        entry!(0x008d8a00, middle_high_process_get_radiation_water_delta(Ptr) -> f32),
        entry!(
            0x008d8a20,
            middle_high_process_set_radiation_delta(Ptr, f32)
        ),
        entry!(0x008d8a40, middle_high_process_get_last_attack_hit_data(Ptr) -> Ptr),
        entry!(0x008d8a60, middle_high_process_get_weapon_condition_stage(Ptr) -> i32),
        entry!(
            0x008d8a80,
            middle_high_process_set_weapon_condition_stage(Ptr, i32)
        ),
        entry!(0x008d8aa0, middle_high_process_set_face_node(Ptr, Ptr)),
        entry!(0x008d8ac0, middle_high_process_get_face_skinned_node(Ptr) -> Ptr),
        entry!(
            0x008d8ae0,
            middle_high_process_set_face_skinned_node(Ptr, Ptr)
        ),
        entry!(0x008d8b00, middle_high_process_get_head_anims(Ptr) -> Ptr),
        entry!(0x008d8b80, middle_high_process_get_current_furniture(Ptr) -> Ptr),
        entry!(0x008d8be0, middle_high_process_get_current_furniture_index(Ptr) -> u8),
        entry!(0x008d8c20, middle_high_process_get_torso_node(Ptr) -> Ptr),
        entry!(
            0x008d8c40,
            middle_high_process_set_anim_action_success(Ptr, i16)
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    fn process(e: &mut Engine) -> Ptr {
        Ptr::new(e.mem.alloc(0x500))
    }

    /// A word getter returns the word at `offset`.
    fn check_getter(addr: u32, offset: u32, value: u32) {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.mem.set_u32(this.addr() + offset, value);
        assert_eq!(e.call(addr, &args![this]).u32(), value);
    }

    /// A one-byte getter returns the byte at `offset` (and not its neighbour).
    fn check_byte_getter(addr: u32, offset: u32) {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.mem.set_u8(this.addr() + offset, 0x5a);
        e.mem.set_u8(this.addr() + offset + 1, 0xff);
        assert_eq!(e.call(addr, &args![this]).u8(), 0x5a);
    }

    /// A word setter stores its argument at `offset` and nothing around it.
    fn check_setter(addr: u32, offset: u32, value: u32) {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.call(addr, &args![this, value]);
        assert_eq!(e.mem.u32(this.addr() + offset), value);
        assert_eq!(e.mem.u32(this.addr() + offset + 4), 0);
        assert_eq!(e.mem.u32(this.addr() + offset - 4), 0);
    }

    /// A 16-bit setter stores the low half of the word.
    fn check_word16_setter(addr: u32, offset: u32) {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.call(addr, &args![this, -2i16]);
        assert_eq!(e.mem.u16(this.addr() + offset), 0xfffe);
        assert_eq!(e.mem.u16(this.addr() + offset + 2), 0);
    }

    /// A byte setter stores one byte at `offset`.
    fn check_byte_setter(addr: u32, offset: u32) {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.call(addr, &args![this, 0x78u8]);
        assert_eq!(e.mem.u8(this.addr() + offset), 0x78);
        assert_eq!(e.mem.u8(this.addr() + offset + 1), 0);
        assert_eq!(e.mem.u8(this.addr() + offset - 1), 0);
    }

    fn check_float_getter(addr: u32, offset: u32) {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.mem.set_f32(this.addr() + offset, 2.5);
        assert_eq!(e.call(addr, &args![this]).f32(), 2.5);
    }

    fn check_float_setter(addr: u32, offset: u32) {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.call(addr, &args![this, 1.25f32]);
        assert_eq!(e.mem.f32(this.addr() + offset), 1.25);
    }

    /// Doubles the by-value list removal, recording (list, item) per call.
    fn record_list_removals(e: &mut Engine) -> Rc<RefCell<Vec<(u32, u32)>>> {
        let seen: Rc<RefCell<Vec<(u32, u32)>>> = Rc::new(RefCell::new(vec![]));
        let log = seen.clone();
        e.register_double(LIST_REMOVE_ITEM, move |e, a| {
            log.borrow_mut().push((a[0], e.mem.u32(a[1])));
            Ret::default()
        });
        seen
    }

    #[test]
    fn test_middle_high_process_set_3d_update_flag() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.mem.set_u8(this.addr() + 0x18c, 0x01);
        e.call(0x008d8740, &args![this, 0x04u32]);
        assert_eq!(e.mem.u8(this.addr() + 0x18c), 0x05);
        assert_eq!(e.mem.u8(this.addr() + 0x18d), 0);
    }

    #[test]
    fn test_middle_high_process_clear_3d_update_flag() {
        let mut e = Engine::new();
        let this = process(&mut e);
        // The code multiplies by the complemented mask: 1 * !1 = 0xfffffffe.
        e.mem.set_u8(this.addr() + 0x18c, 0x01);
        e.call(0x008d8770, &args![this, 0x01u32]);
        assert_eq!(e.mem.u8(this.addr() + 0x18c), 0xfe);
        e.mem.set_u8(this.addr() + 0x18c, 0x00);
        e.call(0x008d8770, &args![this, 0x01u32]);
        assert_eq!(e.mem.u8(this.addr() + 0x18c), 0x00);
        e.mem.set_u8(this.addr() + 0x18c, 0x03);
        e.call(0x008d8770, &args![this, 0xffff_ffffu32]);
        assert_eq!(e.mem.u8(this.addr() + 0x18c), 0x00, "x * !(-1) = 0");
    }

    #[test]
    fn test_middle_high_process_clear_all_3d_update_flags() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.mem.set_u8(this.addr() + 0x18c, 0xff);
        e.mem.set_u8(this.addr() + 0x18d, 0xff);
        e.call(0x008d87a0, &args![this]);
        assert_eq!(e.mem.u8(this.addr() + 0x18c), 0);
        assert_eq!(e.mem.u8(this.addr() + 0x18d), 0xff);
    }

    #[test]
    fn test_middle_high_process_get_3d_update_flag() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.mem.set_u8(this.addr() + 0x18c, 0x06);
        assert!(e.call(0x008d87c0, &args![this, 0x04u32]).bool());
        assert!(!e.call(0x008d87c0, &args![this, 0x01u32]).bool());
        assert!(!e.call(0x008d87c0, &args![this, 0x100u32]).bool());
    }

    #[test]
    fn test_middle_high_process_add_radiation_magic_delta() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.mem.set_f32(this.addr() + 0x238, 2.5);
        e.call(0x008d8960, &args![this, 1.25f32]);
        assert_eq!(e.mem.f32(this.addr() + 0x238), 3.75);
        assert_eq!(e.mem.f32(this.addr() + 0x23c), 0.0);
    }

    #[test]
    fn test_middle_high_process_remove_radiation_magic_delta() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.mem.set_f32(this.addr() + 0x238, 2.5);
        e.call(0x008d8990, &args![this, 1.25f32]);
        assert_eq!(e.mem.f32(this.addr() + 0x238), 1.25);
        assert_eq!(e.mem.f32(this.addr() + 0x23c), 0.0);
    }

    #[test]
    fn test_middle_high_process_get_sit_sleep_state() {
        let mut e = Engine::new();
        let this = process(&mut e);
        let flags = Rc::new(Cell::new(0u32));
        let seen = flags.clone();
        let mut slots = vec![0u32; 0x187];
        slots[0x618 / 4] = 0x0300_0000;
        e.put_vtable(0x0200_0000, &slots);
        e.mem.set_u32(this.addr(), 0x0200_0000);
        e.register_double(0x0300_0000, move |_, _| seen.get().into_ret());
        for (state, flag_word, expected) in [
            (4u8, 0x8u32, 5u32),
            (9, 0x8, 10),
            (4, 0x0, 4),
            (4, 0x7, 4),
            (5, 0x8, 5),
            (0, 0xff, 0),
        ] {
            e.mem.set_u8(this.addr() + 0x13d, state);
            flags.set(flag_word);
            assert_eq!(e.call(0x008d8b20, &args![this]).u32(), expected);
        }
    }

    #[test]
    fn test_middle_high_process_clear_current_furniture() {
        let mut e = Engine::new();
        let this = process(&mut e);
        let seen = record_list_removals(&mut e);
        e.mem.set_u32(this.addr() + 0x140, 0x1111);
        e.call(0x008d8ba0, &args![this, Ptr::<()>::new(0x2222)]);
        assert_eq!(*seen.borrow(), vec![(this.addr() + 0xd0, 0x2222)]);
        assert_eq!(
            e.mem.u32(this.addr() + 0x140),
            0x1111,
            "another reference stays"
        );
        e.call(0x008d8ba0, &args![this, Ptr::<()>::new(0x1111)]);
        assert_eq!(seen.borrow().len(), 2);
        assert_eq!(e.mem.u32(this.addr() + 0x140), 0);
    }

    #[test]
    fn test_middle_high_process_get_lighting_shader_property() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.register(NI_POINTER_GET, |e, a| e.mem.u32(a[0]).into_ret());
        e.mem.set_u32(this.addr() + 0x220, 0x4444);
        assert_eq!(e.call(0x008d8c00, &args![this]).u32(), 0x4444);
    }

    #[test]
    fn test_high_process_remove_spoken_to_actor() {
        let mut e = Engine::new();
        let this = process(&mut e);
        let seen = record_list_removals(&mut e);
        e.mem.set_u32(this.addr() + 0x264, 0x5000);
        e.call(0x008d8c60, &args![this, Ptr::<()>::new(0x6666)]);
        assert_eq!(*seen.borrow(), vec![(0x5000, 0x6666)]);
    }

    #[test]
    fn test_high_process_get_number_guards_arresting() {
        check_getter(0x008d8c80, 0x39c, 7);
    }

    #[test]
    fn test_high_process_mod_number_guards_arresting() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.mem.set_u32(this.addr() + 0x39c, 5);
        e.call(0x008d8ca0, &args![this, 3u32]);
        assert_eq!(e.mem.u32(this.addr() + 0x39c), 8);
        e.call(0x008d8ca0, &args![this, (-9i32) as u32]);
        assert_eq!(e.mem.u32(this.addr() + 0x39c), 0xffff_ffff);
    }

    #[test]
    fn test_middle_high_process_set_is_summoned_creature() {
        check_byte_setter(0x008d8720, 0x18b);
    }

    #[test]
    fn test_middle_high_process_get_all_3d_update_flags() {
        check_byte_getter(0x008d87e0, 0x18c);
    }

    #[test]
    fn test_middle_high_process_get_attached_arrow_list() {
        check_getter(0x008d8800, 0x1ac, 0x1234_5678);
    }

    #[test]
    fn test_middle_high_process_set_iron_sights() {
        check_byte_setter(0x008d8820, 0x228);
    }

    #[test]
    fn test_middle_high_process_get_iron_sights() {
        check_byte_getter(0x008d8840, 0x228);
    }

    #[test]
    fn test_middle_high_process_is_sustained_fire() {
        check_byte_getter(0x008d8860, 0x1d9);
    }

    #[test]
    fn test_middle_high_process_set_sustained_fire() {
        check_byte_setter(0x008d8880, 0x1d9);
    }

    #[test]
    fn test_middle_high_process_set_begin_idles_played() {
        check_byte_setter(0x008d88a0, 0x19c);
    }

    #[test]
    fn test_middle_high_process_get_begin_idles_played() {
        check_byte_getter(0x008d88c0, 0x19c);
    }

    #[test]
    fn test_middle_high_process_set_end_idles_played() {
        check_byte_setter(0x008d88e0, 0x19d);
    }

    #[test]
    fn test_middle_high_process_get_end_idles_played() {
        check_byte_getter(0x008d8900, 0x19d);
    }

    #[test]
    fn test_middle_high_process_get_creature_lip_synch_anim() {
        check_getter(0x008d8920, 0x1a0, 0x1234_5678);
    }

    #[test]
    fn test_middle_high_process_get_creature_lip_synch_start_time() {
        check_getter(0x008d8940, 0x1a4, 0x1234_5678);
    }

    #[test]
    fn test_middle_high_process_get_radiation_magic_delta() {
        check_float_getter(0x008d89c0, 0x238);
    }

    #[test]
    fn test_middle_high_process_set_radiation_water_delta() {
        check_float_setter(0x008d89e0, 0x23c);
    }

    #[test]
    fn test_middle_high_process_get_radiation_water_delta() {
        check_float_getter(0x008d8a00, 0x23c);
    }

    #[test]
    fn test_middle_high_process_set_radiation_delta() {
        check_float_setter(0x008d8a20, 0x234);
    }

    #[test]
    fn test_middle_high_process_get_last_attack_hit_data() {
        check_getter(0x008d8a40, 0x254, 0x1234_5678);
    }

    #[test]
    fn test_middle_high_process_get_weapon_condition_stage() {
        check_getter(0x008d8a60, 0x244, 0x1234_5678);
    }

    #[test]
    fn test_middle_high_process_set_weapon_condition_stage() {
        check_setter(0x008d8a80, 0x244, 0x1234_5678);
    }

    #[test]
    fn test_middle_high_process_set_face_node() {
        check_setter(0x008d8aa0, 0x248, 0x1234_5678);
    }

    #[test]
    fn test_middle_high_process_get_face_skinned_node() {
        check_getter(0x008d8ac0, 0x24c, 0x1234_5678);
    }

    #[test]
    fn test_middle_high_process_set_face_skinned_node() {
        check_setter(0x008d8ae0, 0x24c, 0x1234_5678);
    }

    #[test]
    fn test_middle_high_process_get_head_anims() {
        check_getter(0x008d8b00, 0x250, 0x1234_5678);
    }

    #[test]
    fn test_middle_high_process_get_current_furniture() {
        check_getter(0x008d8b80, 0x140, 0x1234_5678);
    }

    #[test]
    fn test_middle_high_process_get_current_furniture_index() {
        check_byte_getter(0x008d8be0, 0x144);
    }

    #[test]
    fn test_middle_high_process_get_torso_node() {
        check_getter(0x008d8c20, 0x21c, 0x1234_5678);
    }

    #[test]
    fn test_middle_high_process_set_anim_action_success() {
        check_word16_setter(0x008d8c40, 0x22a);
    }
}
