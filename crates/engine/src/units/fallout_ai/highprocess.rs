//! `fallout/ai/highprocess.cpp` (Xbox PDB source unit), subsystem `fallout/ai`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! The first block of this unit is the process classes' one-field accessors
//! (`BaseProcess` > `LowProcess` > `MiddleLowProcess` > `MiddleHighProcess` >
//! `HighProcess`): each reads or writes one field of `this`. The field names
//! and offsets are the Xbox PDB's, checked against the PC code (they agree up
//! to `HighProcess::ePostAnimActon`). The classes' own layouts are not
//! declared here; the offsets are named constants.
//!
//! The two message-queue functions at the end are `BSTCommonMessageQueue`
//! template instances: a try-lock (compare-exchange of the word at +4 from 0
//! to 1) around the virtual `Push` / `Pop`.

#[allow(unused_imports)]
use crate::prelude::*;

/// `InterlockedCompareExchange(target, old, new)` wrapper (`0043b460`),
/// returning the previous value.
const COMPARE_EXCHANGE: u32 = 0x0043_b460;
/// `Error` (`0040fbe0`): a five-byte stub called around the locked section
/// (takes no arguments).
const ERROR_NO_OP: u32 = 0x0040_fbe0;

// Translated from 00407800 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::SetSavedAcquireObject` (Xbox PDB) stores its argument in `pSavedAcquireObject (ObjectstoAcquire*)` at +0x68.
pub fn middle_high_process_set_saved_acquire_object(e: &mut Engine, this: Ptr, value: Ptr) {
    e.mem.set_u32(this.addr() + 0x68, value.addr());
}

// Translated from 004727d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::GetPostAnimationActions` (Xbox PDB) returns `ePostAnimActon (BaseProcess::POSTANIM_ACTION)` at +0x424.
pub fn high_process_get_post_animation_actions(e: &mut Engine, this: Ptr) -> u32 {
    e.mem.u32(this.addr() + 0x424)
}

// Translated from 00502430 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::GetFireNode` (Xbox PDB) returns `pFireNode (NiAVObject*)` at +0x130.
pub fn middle_high_process_get_fire_node(e: &mut Engine, this: Ptr) -> Ptr {
    Ptr::new(e.mem.u32(this.addr() + 0x130))
}

// Translated from 00502c90 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleLowProcess::SetPackageEvaluateHour` (Xbox PDB) stores its argument in `iHourPackageEvaluated` at +0xb4.
pub fn middle_low_process_set_package_evaluate_hour(e: &mut Engine, this: Ptr, value: i32) {
    e.mem.set_u32(this.addr() + 0xb4, value as u32);
}

// Translated from 00505e50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::SetLastIdlePlayed` (Xbox PDB) stores its argument in `pLastIdlePlayed (TESIdleForm*)` at +0x10c.
pub fn middle_high_process_set_last_idle_played(e: &mut Engine, this: Ptr, value: Ptr) {
    e.mem.set_u32(this.addr() + 0x10c, value.addr());
}

// Translated from 005075f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `LowProcess::SetDeathTime` (Xbox PDB) stores its argument in `fDeathTime` at +0xa8.
pub fn low_process_set_death_time(e: &mut Engine, this: Ptr, value: f32) {
    e.mem.set_f32(this.addr() + 0xa8, value);
}

// Translated from 0051f510 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::GetDialogTarget` (Xbox PDB) returns `pDialogTarget (MobileObject*)` at +0x370.
pub fn high_process_get_dialog_target(e: &mut Engine, this: Ptr) -> Ptr {
    Ptr::new(e.mem.u32(this.addr() + 0x370))
}

// Translated from 0051f530 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map; its body stores its argument in `pDialogTarget (MobileObject*)` at +0x370.
pub fn fn_0051f530(e: &mut Engine, this: Ptr, value: Ptr) {
    e.mem.set_u32(this.addr() + 0x370, value.addr());
}

// Translated from 0051f550 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::GetHeadNode` (Xbox PDB) returns `pHeadNode (NiAVObject*)` at +0x218.
pub fn middle_high_process_get_head_node(e: &mut Engine, this: Ptr) -> Ptr {
    Ptr::new(e.mem.u32(this.addr() + 0x218))
}

// Translated from 00521630 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::GetLastHitData` (Xbox PDB) returns `pLastHitData (HitData*)` at +0x240.
pub fn middle_high_process_get_last_hit_data(e: &mut Engine, this: Ptr) -> Ptr {
    Ptr::new(e.mem.u32(this.addr() + 0x240))
}

// Translated from 00521650 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map; its body stores its argument in `bIronSights (bool; the store is a whole word, so it also covers iAnimActionSuccess at +0x22a)` at +0x228.
pub fn fn_00521650(e: &mut Engine, this: Ptr, value: u32) {
    e.mem.set_u32(this.addr() + 0x228, value);
}

// Translated from 00521670 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map; its body stores its argument in `iMovementAnimations` at +0x22c.
pub fn fn_00521670(e: &mut Engine, this: Ptr, value: u32) {
    e.mem.set_u32(this.addr() + 0x22c, value);
}

// Translated from 00521690 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map; its body stores its argument in `pListItemstoEquipUnequip (BSSimpleList<QueuedItem *>*)` at +0x230.
pub fn fn_00521690(e: &mut Engine, this: Ptr, value: Ptr) {
    e.mem.set_u32(this.addr() + 0x230, value.addr());
}

// Translated from 005216b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map; its body stores its argument in `fRadiationDelta (copied as a raw word)` at +0x234.
pub fn fn_005216b0(e: &mut Engine, this: Ptr, value: u32) {
    e.mem.set_u32(this.addr() + 0x234, value);
}

// Translated from 005216d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map; its body stores its argument in `fRadiationMagicDelta (copied as a raw word)` at +0x238.
pub fn fn_005216d0(e: &mut Engine, this: Ptr, value: u32) {
    e.mem.set_u32(this.addr() + 0x238, value);
}

// Translated from 005216f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map; its body stores its argument in `fRadiationWaterDelta (copied as a raw word)` at +0x23c.
pub fn fn_005216f0(e: &mut Engine, this: Ptr, value: u32) {
    e.mem.set_u32(this.addr() + 0x23c, value);
}

// Translated from 00521710 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::SetHeadAnims` (Xbox PDB) stores its argument in `pAnimFace (NiAVObject*)` at +0x250.
pub fn middle_high_process_set_head_anims(e: &mut Engine, this: Ptr, value: Ptr) {
    e.mem.set_u32(this.addr() + 0x250, value.addr());
}

// Translated from 005224a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::GetFaceNode` (Xbox PDB) returns `pFaceNode (BSFaceGenNiNode*)` at +0x248.
pub fn middle_high_process_get_face_node(e: &mut Engine, this: Ptr) -> Ptr {
    Ptr::new(e.mem.u32(this.addr() + 0x248))
}

// Translated from 00566950 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::GetForceNextUpdate` (Xbox PDB) returns `bForceNextUpdate` at +0x18d.
pub fn middle_high_process_get_force_next_update(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u8(this.addr() + 0x18d) != 0
}

// Translated from 005a29b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::GetLastIdlePlayed` (Xbox PDB) returns `pLastIdlePlayed (TESIdleForm*)` at +0x10c.
pub fn middle_high_process_get_last_idle_played(e: &mut Engine, this: Ptr) -> Ptr {
    Ptr::new(e.mem.u32(this.addr() + 0x10c))
}

// Translated from 005a8060 (decompiled, FalloutNV.exe 1.4.0.525)
/// `LowProcess::SetNumberOfItemsActivated` (Xbox PDB) stores its argument in `iNumberItemsActivate` at +0x58.
pub fn low_process_set_number_of_items_activated(e: &mut Engine, this: Ptr, value: i32) {
    e.mem.set_u32(this.addr() + 0x58, value as u32);
}

// Translated from 005c8a30 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::SetBSBound` (Xbox PDB) stores its argument in `pBSBound (BSBound*)` at +0x224.
pub fn middle_high_process_set_bs_bound(e: &mut Engine, this: Ptr, value: Ptr) {
    e.mem.set_u32(this.addr() + 0x224, value.addr());
}

// Translated from 005ce8f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::IsCurrentWeaponMine` (Xbox PDB) returns `bWeaponMine` at +0x125.
pub fn middle_high_process_is_current_weapon_mine(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u8(this.addr() + 0x125) != 0
}

// Translated from 005f7590 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::GetCurrentAmmo` (Xbox PDB) returns `pAmmo (ItemChange*)` at +0x118.
pub fn middle_high_process_get_current_ammo(e: &mut Engine, this: Ptr) -> Ptr {
    Ptr::new(e.mem.u32(this.addr() + 0x118))
}

// Translated from 005f8bd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::SetCommandingActor` (Xbox PDB) stores its argument in `pCommandingActor (Actor*)` at +0x158.
pub fn middle_high_process_set_commanding_actor(e: &mut Engine, this: Ptr, value: Ptr) {
    e.mem.set_u32(this.addr() + 0x158, value.addr());
}

// Translated from 005f9b50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::GetCommandingActor` (Xbox PDB) returns `pCommandingActor (Actor*)` at +0x158.
pub fn middle_high_process_get_commanding_actor(e: &mut Engine, this: Ptr) -> Ptr {
    Ptr::new(e.mem.u32(this.addr() + 0x158))
}

// Translated from 00602150 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::GetAnimation` (Xbox PDB) returns `pAnimation (Animation*)` at +0x1c0.
pub fn middle_high_process_get_animation(e: &mut Engine, this: Ptr) -> Ptr {
    Ptr::new(e.mem.u32(this.addr() + 0x1c0))
}

// Translated from 006214b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `HighProcess::GetCurrentProcessIdle` (Xbox PDB) returns `pIdleToPlay (TESIdleForm*)` at +0x350.
pub fn high_process_get_current_process_idle(e: &mut Engine, this: Ptr) -> Ptr {
    Ptr::new(e.mem.u32(this.addr() + 0x350))
}

// Translated from 006286d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `LowProcess::GetNumberOfItemsActivated` (Xbox PDB) returns `iNumberItemsActivate` at +0x58.
pub fn low_process_get_number_of_items_activated(e: &mut Engine, this: Ptr) -> i32 {
    e.mem.u32(this.addr() + 0x58) as i32
}

// Translated from 00645c40 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::GetScriptRefractionPower` (Xbox PDB) returns `fScriptRefractPower` at +0x174.
pub fn middle_high_process_get_script_refraction_power(e: &mut Engine, this: Ptr) -> f32 {
    e.mem.f32(this.addr() + 0x174)
}

// Translated from 00649b80 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::GetIsSummonedCreature` (Xbox PDB) returns `bSummonedCreature` at +0x18b.
pub fn middle_high_process_get_is_summoned_creature(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u8(this.addr() + 0x18b) != 0
}

// Translated from 006546c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::SetIdleDoneOnce` (Xbox PDB) stores its argument in `bDoneOnce` at +0xe0.
pub fn middle_high_process_set_idle_done_once(e: &mut Engine, this: Ptr, value: u8) {
    e.mem.set_u8(this.addr() + 0xe0, value);
}

// Translated from 006733e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `LowProcess::GetSecondGenericLocation` (Xbox PDB) returns `pGenericSecondLocation (TESObjectREFR*)` at +0x48.
pub fn low_process_get_second_generic_location(e: &mut Engine, this: Ptr) -> Ptr {
    Ptr::new(e.mem.u32(this.addr() + 0x48))
}

// Translated from 00673400 (decompiled, FalloutNV.exe 1.4.0.525)
/// `LowProcess::SetSecondGenericLocation` (Xbox PDB) stores its argument in `pGenericSecondLocation (TESObjectREFR*)` at +0x48.
pub fn low_process_set_second_generic_location(e: &mut Engine, this: Ptr, value: Ptr) {
    e.mem.set_u32(this.addr() + 0x48, value.addr());
}

// Translated from 007ac9d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::SetWeaponEnchantmentVisuals` (Xbox PDB) stores its argument in `pCurrentWeaponEffect (TESEffectShader*)` at +0x16c.
pub fn middle_high_process_set_weapon_enchantment_visuals(e: &mut Engine, this: Ptr, value: Ptr) {
    e.mem.set_u32(this.addr() + 0x16c, value.addr());
}

// Translated from 008041a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `LowProcess::GetGenericLocation` (Xbox PDB) returns `pGenericLocation (TESObjectREFR*)` at +0x44.
pub fn low_process_get_generic_location(e: &mut Engine, this: Ptr) -> Ptr {
    Ptr::new(e.mem.u32(this.addr() + 0x44))
}

// Translated from 00812870 (decompiled, FalloutNV.exe 1.4.0.525)
/// `LowProcess::GetEssentialDownTimer` (Xbox PDB) returns `fEssentialDownTimer` at +0xa4.
pub fn low_process_get_essential_down_timer(e: &mut Engine, this: Ptr) -> f32 {
    e.mem.f32(this.addr() + 0xa4)
}

// Translated from 0087d7d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MiddleHighProcess::SetCreatureLipSynchStartTime` (Xbox PDB) stores its argument in `iCreatureLipsynchStartTime` at +0x1a4.
pub fn middle_high_process_set_creature_lip_synch_start_time(
    e: &mut Engine,
    this: Ptr,
    value: u32,
) {
    e.mem.set_u32(this.addr() + 0x1a4, value);
}

// Translated from 006ec0d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSTCommonMessageQueue<ActorPathingMessage>::TryPush` (Xbox PDB): takes
/// the queue's lock (compare-exchange of the word at +4 from 0 to 1); when
/// it is free, calls the virtual `Push` (vtable +0x14) with `message` and
/// releases the lock (1 back to 0). Returns `Push`'s result, or false when
/// the lock was taken.
pub fn bst_common_message_queue_actor_pathing_message_try_push(
    e: &mut Engine,
    this: Ptr,
    message: Ptr,
) -> bool {
    try_locked_vcall(e, this, 0x14, message)
}

// Translated from 006ec390 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSTCommonMessageQueue<ActorPathingMessage>::TryPop` (Xbox PDB): the same
/// try-lock around the virtual `Pop` (vtable +0x18).
pub fn bst_common_message_queue_actor_pathing_message_try_pop(
    e: &mut Engine,
    this: Ptr,
    message: Ptr,
) -> bool {
    try_locked_vcall(e, this, 0x18, message)
}

/// The body both `Try` functions share.
fn try_locked_vcall(e: &mut Engine, this: Ptr, slot: u32, message: Ptr) -> bool {
    let lock = this.addr() + 4;
    let previous = e.call(COMPARE_EXCHANGE, &args![lock, 0u32, 1u32]).u32();
    if previous != 0 {
        return false;
    }
    e.call(ERROR_NO_OP, &args![]);
    let result = e.vcall(this.addr(), slot, &args![message]).bool();
    e.call(ERROR_NO_OP, &args![]);
    e.call(COMPARE_EXCHANGE, &args![lock, 1u32, 0u32]);
    result
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(
            0x00407800,
            middle_high_process_set_saved_acquire_object(Ptr, Ptr)
        ),
        entry!(0x004727d0, high_process_get_post_animation_actions(Ptr) -> u32),
        entry!(0x00502430, middle_high_process_get_fire_node(Ptr) -> Ptr),
        entry!(
            0x00502c90,
            middle_low_process_set_package_evaluate_hour(Ptr, i32)
        ),
        entry!(
            0x00505e50,
            middle_high_process_set_last_idle_played(Ptr, Ptr)
        ),
        entry!(0x005075f0, low_process_set_death_time(Ptr, f32)),
        entry!(0x0051f510, high_process_get_dialog_target(Ptr) -> Ptr),
        entry!(0x0051f530, fn_0051f530(Ptr, Ptr)),
        entry!(0x0051f550, middle_high_process_get_head_node(Ptr) -> Ptr),
        entry!(0x00521630, middle_high_process_get_last_hit_data(Ptr) -> Ptr),
        entry!(0x00521650, fn_00521650(Ptr, u32)),
        entry!(0x00521670, fn_00521670(Ptr, u32)),
        entry!(0x00521690, fn_00521690(Ptr, Ptr)),
        entry!(0x005216b0, fn_005216b0(Ptr, u32)),
        entry!(0x005216d0, fn_005216d0(Ptr, u32)),
        entry!(0x005216f0, fn_005216f0(Ptr, u32)),
        entry!(0x00521710, middle_high_process_set_head_anims(Ptr, Ptr)),
        entry!(0x005224a0, middle_high_process_get_face_node(Ptr) -> Ptr),
        entry!(0x00566950, middle_high_process_get_force_next_update(Ptr) -> bool),
        entry!(0x005a29b0, middle_high_process_get_last_idle_played(Ptr) -> Ptr),
        entry!(
            0x005a8060,
            low_process_set_number_of_items_activated(Ptr, i32)
        ),
        entry!(0x005c8a30, middle_high_process_set_bs_bound(Ptr, Ptr)),
        entry!(0x005ce8f0, middle_high_process_is_current_weapon_mine(Ptr) -> bool),
        entry!(0x005f7590, middle_high_process_get_current_ammo(Ptr) -> Ptr),
        entry!(
            0x005f8bd0,
            middle_high_process_set_commanding_actor(Ptr, Ptr)
        ),
        entry!(0x005f9b50, middle_high_process_get_commanding_actor(Ptr) -> Ptr),
        entry!(0x00602150, middle_high_process_get_animation(Ptr) -> Ptr),
        entry!(0x006214b0, high_process_get_current_process_idle(Ptr) -> Ptr),
        entry!(0x006286d0, low_process_get_number_of_items_activated(Ptr) -> i32),
        entry!(0x00645c40, middle_high_process_get_script_refraction_power(Ptr) -> f32),
        entry!(0x00649b80, middle_high_process_get_is_summoned_creature(Ptr) -> bool),
        entry!(0x006546c0, middle_high_process_set_idle_done_once(Ptr, u8)),
        entry!(0x006733e0, low_process_get_second_generic_location(Ptr) -> Ptr),
        entry!(
            0x00673400,
            low_process_set_second_generic_location(Ptr, Ptr)
        ),
        entry!(
            0x007ac9d0,
            middle_high_process_set_weapon_enchantment_visuals(Ptr, Ptr)
        ),
        entry!(0x008041a0, low_process_get_generic_location(Ptr) -> Ptr),
        entry!(0x00812870, low_process_get_essential_down_timer(Ptr) -> f32),
        entry!(
            0x0087d7d0,
            middle_high_process_set_creature_lip_synch_start_time(Ptr, u32)
        ),
        entry!(0x006ec0d0, bst_common_message_queue_actor_pathing_message_try_push(Ptr, Ptr) -> bool),
        entry!(0x006ec390, bst_common_message_queue_actor_pathing_message_try_pop(Ptr, Ptr) -> bool),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A zeroed object big enough for every class here (`HighProcess` is
    /// 0x46c bytes).
    fn process(e: &mut Engine) -> Ptr {
        Ptr::new(e.mem.alloc(0x500))
    }

    #[test]
    fn test_middle_high_process_set_saved_acquire_object() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.call(0x00407800, &args![this, Ptr::<()>::new(0x1234_5678)]);
        assert_eq!(e.mem.u32(this.addr() + 0x68), 0x1234_5678);
        assert_eq!(
            e.mem.u32(this.addr() + 0x68 + 8),
            0,
            "only the one field is written"
        );
    }

    #[test]
    fn test_high_process_get_post_animation_actions() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.mem.set_u32(this.addr() + 0x424, 0x1234_5678);
        let r = e.call(0x004727d0, &args![this]);
        assert_eq!(r.u32(), 0x1234_5678);
    }

    #[test]
    fn test_middle_high_process_get_fire_node() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.mem.set_u32(this.addr() + 0x130, 0x1234_5678);
        let r = e.call(0x00502430, &args![this]);
        assert_eq!(r.u32(), 0x1234_5678);
    }

    #[test]
    fn test_middle_low_process_set_package_evaluate_hour() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.call(0x00502c90, &args![this, -5i32]);
        assert_eq!(e.mem.u32(this.addr() + 0xb4), -5i32 as u32);
        assert_eq!(
            e.mem.u32(this.addr() + 0xb4 + 8),
            0,
            "only the one field is written"
        );
    }

    #[test]
    fn test_middle_high_process_set_last_idle_played() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.call(0x00505e50, &args![this, Ptr::<()>::new(0x1234_5678)]);
        assert_eq!(e.mem.u32(this.addr() + 0x10c), 0x1234_5678);
        assert_eq!(
            e.mem.u32(this.addr() + 0x10c + 8),
            0,
            "only the one field is written"
        );
    }

    #[test]
    fn test_low_process_set_death_time() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.call(0x005075f0, &args![this, 2.5f32]);
        assert_eq!(e.mem.f32(this.addr() + 0xa8), 2.5);
        assert_eq!(
            e.mem.u32(this.addr() + 0xa8 + 8),
            0,
            "only the one field is written"
        );
    }

    #[test]
    fn test_high_process_get_dialog_target() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.mem.set_u32(this.addr() + 0x370, 0x1234_5678);
        let r = e.call(0x0051f510, &args![this]);
        assert_eq!(r.u32(), 0x1234_5678);
    }

    #[test]
    fn test_fn_0051f530() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.call(0x0051f530, &args![this, Ptr::<()>::new(0x1234_5678)]);
        assert_eq!(e.mem.u32(this.addr() + 0x370), 0x1234_5678);
        assert_eq!(
            e.mem.u32(this.addr() + 0x370 + 8),
            0,
            "only the one field is written"
        );
    }

    #[test]
    fn test_middle_high_process_get_head_node() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.mem.set_u32(this.addr() + 0x218, 0x1234_5678);
        let r = e.call(0x0051f550, &args![this]);
        assert_eq!(r.u32(), 0x1234_5678);
    }

    #[test]
    fn test_middle_high_process_get_last_hit_data() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.mem.set_u32(this.addr() + 0x240, 0x1234_5678);
        let r = e.call(0x00521630, &args![this]);
        assert_eq!(r.u32(), 0x1234_5678);
    }

    #[test]
    fn test_fn_00521650() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.call(0x00521650, &args![this, 0x1234_5678u32]);
        assert_eq!(e.mem.u32(this.addr() + 0x228), 0x1234_5678);
        assert_eq!(
            e.mem.u32(this.addr() + 0x228 + 8),
            0,
            "only the one field is written"
        );
    }

    #[test]
    fn test_fn_00521670() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.call(0x00521670, &args![this, 0x1234_5678u32]);
        assert_eq!(e.mem.u32(this.addr() + 0x22c), 0x1234_5678);
        assert_eq!(
            e.mem.u32(this.addr() + 0x22c + 8),
            0,
            "only the one field is written"
        );
    }

    #[test]
    fn test_fn_00521690() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.call(0x00521690, &args![this, Ptr::<()>::new(0x1234_5678)]);
        assert_eq!(e.mem.u32(this.addr() + 0x230), 0x1234_5678);
        assert_eq!(
            e.mem.u32(this.addr() + 0x230 + 8),
            0,
            "only the one field is written"
        );
    }

    #[test]
    fn test_fn_005216b0() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.call(0x005216b0, &args![this, 0x1234_5678u32]);
        assert_eq!(e.mem.u32(this.addr() + 0x234), 0x1234_5678);
        assert_eq!(
            e.mem.u32(this.addr() + 0x234 + 8),
            0,
            "only the one field is written"
        );
    }

    #[test]
    fn test_fn_005216d0() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.call(0x005216d0, &args![this, 0x1234_5678u32]);
        assert_eq!(e.mem.u32(this.addr() + 0x238), 0x1234_5678);
        assert_eq!(
            e.mem.u32(this.addr() + 0x238 + 8),
            0,
            "only the one field is written"
        );
    }

    #[test]
    fn test_fn_005216f0() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.call(0x005216f0, &args![this, 0x1234_5678u32]);
        assert_eq!(e.mem.u32(this.addr() + 0x23c), 0x1234_5678);
        assert_eq!(
            e.mem.u32(this.addr() + 0x23c + 8),
            0,
            "only the one field is written"
        );
    }

    #[test]
    fn test_middle_high_process_set_head_anims() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.call(0x00521710, &args![this, Ptr::<()>::new(0x1234_5678)]);
        assert_eq!(e.mem.u32(this.addr() + 0x250), 0x1234_5678);
        assert_eq!(
            e.mem.u32(this.addr() + 0x250 + 8),
            0,
            "only the one field is written"
        );
    }

    #[test]
    fn test_middle_high_process_get_face_node() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.mem.set_u32(this.addr() + 0x248, 0x1234_5678);
        let r = e.call(0x005224a0, &args![this]);
        assert_eq!(r.u32(), 0x1234_5678);
    }

    #[test]
    fn test_middle_high_process_get_force_next_update() {
        let mut e = Engine::new();
        let this = process(&mut e);
        assert!(!e.call(0x00566950, &args![this]).bool());
        e.mem.set_u8(this.addr() + 0x18d, 1);
        assert!(e.call(0x00566950, &args![this]).bool());
    }

    #[test]
    fn test_middle_high_process_get_last_idle_played() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.mem.set_u32(this.addr() + 0x10c, 0x1234_5678);
        let r = e.call(0x005a29b0, &args![this]);
        assert_eq!(r.u32(), 0x1234_5678);
    }

    #[test]
    fn test_low_process_set_number_of_items_activated() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.call(0x005a8060, &args![this, -5i32]);
        assert_eq!(e.mem.u32(this.addr() + 0x58), -5i32 as u32);
        assert_eq!(
            e.mem.u32(this.addr() + 0x58 + 8),
            0,
            "only the one field is written"
        );
    }

    #[test]
    fn test_middle_high_process_set_bs_bound() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.call(0x005c8a30, &args![this, Ptr::<()>::new(0x1234_5678)]);
        assert_eq!(e.mem.u32(this.addr() + 0x224), 0x1234_5678);
        assert_eq!(
            e.mem.u32(this.addr() + 0x224 + 8),
            0,
            "only the one field is written"
        );
    }

    #[test]
    fn test_middle_high_process_is_current_weapon_mine() {
        let mut e = Engine::new();
        let this = process(&mut e);
        assert!(!e.call(0x005ce8f0, &args![this]).bool());
        e.mem.set_u8(this.addr() + 0x125, 1);
        assert!(e.call(0x005ce8f0, &args![this]).bool());
    }

    #[test]
    fn test_middle_high_process_get_current_ammo() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.mem.set_u32(this.addr() + 0x118, 0x1234_5678);
        let r = e.call(0x005f7590, &args![this]);
        assert_eq!(r.u32(), 0x1234_5678);
    }

    #[test]
    fn test_middle_high_process_set_commanding_actor() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.call(0x005f8bd0, &args![this, Ptr::<()>::new(0x1234_5678)]);
        assert_eq!(e.mem.u32(this.addr() + 0x158), 0x1234_5678);
        assert_eq!(
            e.mem.u32(this.addr() + 0x158 + 8),
            0,
            "only the one field is written"
        );
    }

    #[test]
    fn test_middle_high_process_get_commanding_actor() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.mem.set_u32(this.addr() + 0x158, 0x1234_5678);
        let r = e.call(0x005f9b50, &args![this]);
        assert_eq!(r.u32(), 0x1234_5678);
    }

    #[test]
    fn test_middle_high_process_get_animation() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.mem.set_u32(this.addr() + 0x1c0, 0x1234_5678);
        let r = e.call(0x00602150, &args![this]);
        assert_eq!(r.u32(), 0x1234_5678);
    }

    #[test]
    fn test_high_process_get_current_process_idle() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.mem.set_u32(this.addr() + 0x350, 0x1234_5678);
        let r = e.call(0x006214b0, &args![this]);
        assert_eq!(r.u32(), 0x1234_5678);
    }

    #[test]
    fn test_low_process_get_number_of_items_activated() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.mem.set_u32(this.addr() + 0x58, 0x1234_5678);
        let r = e.call(0x006286d0, &args![this]);
        assert_eq!(r.u32(), 0x1234_5678);
    }

    #[test]
    fn test_middle_high_process_get_script_refraction_power() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.mem.set_f32(this.addr() + 0x174, 2.5);
        assert_eq!(e.call(0x00645c40, &args![this]).f32(), 2.5);
    }

    #[test]
    fn test_middle_high_process_get_is_summoned_creature() {
        let mut e = Engine::new();
        let this = process(&mut e);
        assert!(!e.call(0x00649b80, &args![this]).bool());
        e.mem.set_u8(this.addr() + 0x18b, 1);
        assert!(e.call(0x00649b80, &args![this]).bool());
    }

    #[test]
    fn test_middle_high_process_set_idle_done_once() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.call(0x006546c0, &args![this, 1u8]);
        assert_eq!(e.mem.u8(this.addr() + 0xe0), 1);
        assert_eq!(
            e.mem.u32(this.addr() + 0xe0 + 8),
            0,
            "only the one field is written"
        );
    }

    #[test]
    fn test_low_process_get_second_generic_location() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.mem.set_u32(this.addr() + 0x48, 0x1234_5678);
        let r = e.call(0x006733e0, &args![this]);
        assert_eq!(r.u32(), 0x1234_5678);
    }

    #[test]
    fn test_low_process_set_second_generic_location() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.call(0x00673400, &args![this, Ptr::<()>::new(0x1234_5678)]);
        assert_eq!(e.mem.u32(this.addr() + 0x48), 0x1234_5678);
        assert_eq!(
            e.mem.u32(this.addr() + 0x48 + 8),
            0,
            "only the one field is written"
        );
    }

    #[test]
    fn test_middle_high_process_set_weapon_enchantment_visuals() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.call(0x007ac9d0, &args![this, Ptr::<()>::new(0x1234_5678)]);
        assert_eq!(e.mem.u32(this.addr() + 0x16c), 0x1234_5678);
        assert_eq!(
            e.mem.u32(this.addr() + 0x16c + 8),
            0,
            "only the one field is written"
        );
    }

    #[test]
    fn test_low_process_get_generic_location() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.mem.set_u32(this.addr() + 0x44, 0x1234_5678);
        let r = e.call(0x008041a0, &args![this]);
        assert_eq!(r.u32(), 0x1234_5678);
    }

    #[test]
    fn test_low_process_get_essential_down_timer() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.mem.set_f32(this.addr() + 0xa4, 2.5);
        assert_eq!(e.call(0x00812870, &args![this]).f32(), 2.5);
    }

    #[test]
    fn test_middle_high_process_set_creature_lip_synch_start_time() {
        let mut e = Engine::new();
        let this = process(&mut e);
        e.call(0x0087d7d0, &args![this, 0x1234_5678u32]);
        assert_eq!(e.mem.u32(this.addr() + 0x1a4), 0x1234_5678);
        assert_eq!(
            e.mem.u32(this.addr() + 0x1a4 + 8),
            0,
            "only the one field is written"
        );
    }

    /// A queue object (lock word at +4, vtable at +0) whose virtual slot at
    /// `slot` is a double answering `answer`; the compare-exchange is the
    /// real operation on memory, and the stub is a no-op.
    fn queue_with_slot(e: &mut Engine, slot: u32, answer: u32) -> Ptr {
        let queue = Ptr::new(e.mem.alloc(8));
        let mut slots = [0u32; 8];
        slots[(slot / 4) as usize] = 0x7000_0000;
        e.put_vtable(0x7100_0000, &slots);
        e.mem.set_u32(queue.addr(), 0x7100_0000);
        e.register(COMPARE_EXCHANGE, |e, a| {
            let old = e.mem.u32(a[0]);
            if old == a[1] {
                e.mem.set_u32(a[0], a[2]);
            }
            old.into_ret()
        });
        e.register(ERROR_NO_OP, |_, _| Ret::default());
        e.register_double(0x7000_0000, move |e, a| {
            // The lock is held while the virtual runs.
            assert_eq!(e.mem.u32(a[0] + 4), 1);
            Ret {
                eax: answer,
                ..Ret::default()
            }
        });
        queue
    }

    #[test]
    fn test_try_push_calls_push_under_the_lock() {
        let mut e = Engine::new();
        let queue = queue_with_slot(&mut e, 0x14, 1);
        e.call_log = Some(vec![]);
        let message = Ptr::<()>::new(0x2222);
        let r = e.call(0x006ec0d0, &args![queue, message]);
        assert!(r.bool());
        assert_eq!(e.mem.u32(queue.addr() + 4), 0, "lock released");
        let log = e.call_log.take().unwrap();
        assert!(log.contains(&(0x7000_0000, vec![queue.addr(), 0x2222])));
    }

    #[test]
    fn test_try_push_returns_false_when_locked() {
        let mut e = Engine::new();
        let queue = queue_with_slot(&mut e, 0x14, 1);
        e.mem.set_u32(queue.addr() + 4, 1);
        e.call_log = Some(vec![]);
        let r = e.call(0x006ec0d0, &args![queue, Ptr::<()>::new(0x2222)]);
        assert!(!r.bool());
        assert_eq!(e.mem.u32(queue.addr() + 4), 1, "lock untouched");
        let log = e.call_log.take().unwrap();
        assert!(!log.iter().any(|(a, _)| *a == 0x7000_0000));
    }

    #[test]
    fn test_try_pop_calls_pop_under_the_lock() {
        let mut e = Engine::new();
        let queue = queue_with_slot(&mut e, 0x18, 0);
        e.call_log = Some(vec![]);
        let r = e.call(0x006ec390, &args![queue, Ptr::<()>::new(0x3333)]);
        assert!(!r.bool(), "Pop's own answer is passed through");
        assert_eq!(e.mem.u32(queue.addr() + 4), 0);
        let log = e.call_log.take().unwrap();
        assert!(log.contains(&(0x7000_0000, vec![queue.addr(), 0x3333])));
    }

    #[test]
    fn test_try_pop_returns_false_when_locked() {
        let mut e = Engine::new();
        let queue = queue_with_slot(&mut e, 0x18, 1);
        e.mem.set_u32(queue.addr() + 4, 1);
        assert!(!e
            .call(0x006ec390, &args![queue, Ptr::<()>::new(0x3333)])
            .bool());
    }
}
