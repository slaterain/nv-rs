//! Group GC, part 3: JIP's hit damage. **O1** (`bIgnoreDTDRFix`, on by default: `HOOK_INIT_JUMP`
//! of `HitData::ReduceDamage` `009b5a30` to `CalculateHitDamageHook`, `SetOptionalPatch(1)`) with
//! **O12** (`bLocalizedDTDR`, its flag), **G116** (`SetHitLocationHook` at `0x9B6E81`) and **G119**
//! (`RunAmmoImpactScriptHook` at `0x89ABDC`), `internal/patches_game.h` (JIP LN NVSE).
//!
//! O1 is a wrap of `009b5a30` put in with the row; with the switch's bit clear it hands the call
//! on (`call_next`), as JIP's `HOOK_SET` off restores the jump. The translations call
//! `009b5a30` by address (the five direct calls of `hitdata.rs` became calls by address, the
//! `overridable.tsv` ratchet 5 -> 0). JIP defers the calls returning to `0x9B5628` and `0x9B5707`
//! (`InitializeHitData_ov3` / `_ov4`, before their `SetTargetedLimb`): the port asks seams at
//! those two calls ([`DEFER_OV3`], [`DEFER_OV4`]) in place of JIP's return-address test, and
//! G116 runs the deferred calculation at `SetTargetedLimb`'s limb step.
//!
//! **Threads.** Hit damage runs on the main thread and on the game's AI threads (actors' hits,
//! `HitMe`); G116 and G119 run with it. No handler takes JIP's shared slot (`e.ext::<Jip>()`).
//! `s_VATSHitDT` and `s_condDRDTPenalty` are one run-wide lock-free slot ([`HitDamageState`],
//! `ext_free`, made in `install_fixes` before the threads start), as JIP's are one per process;
//! everything else is game memory and calls by address, except the deferral's word.
//!
//! **The deferral's word (`s_doCalcDamage`) is kept per thread** (a deliberate difference: JIP's
//! is one plain static for every thread). The deferral and the call that takes it always run on
//! one thread inside one call (`InitializeHitData_ov3` / `_ov4`: the deferred `ReduceDamage` at
//! `0x9B5623` / `0x9B5702`, then their `SetTargetedLimb` at once, whose limb step `0x9B6E81` runs
//! it), so nothing needs the word to cross threads; JIP's one static lets two hits built at once
//! on two threads cross (one then takes the other's word and limb, or none: no DT, no DR, no
//! minimum). Here each OS thread keeps its own word, keyed by the run (the run's id in
//! [`HitDamageState`], a process-wide count; the same in every thread's engine of a run): every game thread is its own OS thread, with stepped
//! threads (`thread_host`: one coroutine OS thread each, one engine handed between them) and
//! real threads (`NV_THREADS=real`: one engine each) alike, so on one thread the port does
//! exactly what JIP does, and a hit on another thread can neither take nor overwrite it.
//!
//! **A word left pending** (JIP's own behaviour, kept on one thread): `SetTargetedLimb` leaves at
//! once (`0x9B6688`) for a null target or one that is not a character, before the limb step, and
//! `_ov4` makes its `ReduceDamage` call without a target test; a deferral there stays pending.
//! JIP then has the NEXT `ReduceDamage` (any of the five callers) take it: that call's
//! calculation runs with the stale word in place of its own, and a deferral site reached while
//! it is pending does not defer (its call is made at once, before its limb is chosen, with the
//! stale word). The port does exactly that on the thread that left it; other threads are not
//! touched by it.

use crate::commands::havok_refs::{is_actor, is_projectile};
use engine_core::extend::{jip_patches, Owner, SiteSlot, WrapSlot};
use engine_core::prelude::*;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

/// O1's wrap: `HitData::ReduceDamage`.
pub const REDUCE_DAMAGE: WrapSlot = WrapSlot::new(Owner::Jip, 0x009b_5a30);
/// O1's deferral seams: `ReduceDamage`'s calls in `InitializeHitData_ov3` and `_ov4`.
pub const DEFER_OV3: SiteSlot = SiteSlot::new(Owner::Jip, 0x009b_5623);
pub const DEFER_OV4: SiteSlot = SiteSlot::new(Owner::Jip, 0x009b_5702);
/// G116's seam: `SetTargetedLimb`'s limb step.
pub const HIT_LOCATION: SiteSlot = SiteSlot::new(Owner::Jip, 0x009b_6e81);
/// G119's seam: the ammo's script effect in `Actor::HitMe`.
pub const AMMO_IMPACT_SCRIPT: SiteSlot = SiteSlot::new(Owner::Jip, 0x0089_abdc);

/// JIP's run-wide statics of the hit damage and the combat formulas: `s_VATSHitDT` (the last
/// threshold, read by JIP's VATS menu patch G121; `-FLT_MAX` until a hit; JIP sets it back to
/// `-FLT_MAX` when O1 is switched off in the game, which comes with G121, nothing reading it
/// before), `s_condDRDTPenalty` (the armour condition factor of O12 and G85, set by
/// `SetArmorConditionPenalty`; 1.0), `s_condDmgPenalty` (G84's weapon condition factor, set by
/// `SetConditionDamagePenalty`; 0.67) and `s_noMovementCombat` (G126, `ToggleNoMovementCombat`),
/// the floats as their bits; and the run's id ([`HitDamageState::run`]). JIP's
/// `s_doCalcDamage` is per thread ([`pending_word`]).
pub struct HitDamageState {
    vats_hit_dt: AtomicU32,
    condition_penalty: AtomicU32,
    damage_penalty: AtomicU32,
    no_movement_combat: AtomicBool,
    /// This run's id: a process-wide count, one per run's slot (every thread's engine of a run
    /// shares the slot, so the id). The deferral's word is kept by it: a run's engine dropped
    /// with a word pending leaves it to no later run on the same thread, as a kernel address,
    /// which a later run may get again, would.
    run: usize,
}

/// The ids [`HitDamageState::run`] takes, counted over the process.
static NEXT_RUN: AtomicUsize = AtomicUsize::new(1);

impl Default for HitDamageState {
    fn default() -> Self {
        HitDamageState {
            vats_hit_dt: AtomicU32::new((-f32::MAX).to_bits()),
            condition_penalty: AtomicU32::new(1.0f32.to_bits()),
            damage_penalty: AtomicU32::new(0.67f32.to_bits()),
            no_movement_combat: AtomicBool::new(false),
            run: NEXT_RUN.fetch_add(1, Ordering::Relaxed),
        }
    }
}

pub(super) fn state(e: &mut Engine) -> &HitDamageState {
    e.ext_free::<HitDamageState>()
}

thread_local! {
    /// `s_doCalcDamage` of this OS thread, by run ([`HitDamageState::run`]): 0, or a deferred
    /// `ReduceDamage` call's word + 1 (see the module's "Threads").
    static PENDING: std::cell::RefCell<Vec<(usize, u32)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// This thread's `s_doCalcDamage` in `e`'s run: 0 (none), or the deferred call's word + 1.
pub fn pending_word(e: &mut Engine) -> u32 {
    let run = state(e).run;
    PENDING.with(|p| {
        p.borrow()
            .iter()
            .find(|(r, _)| *r == run)
            .map_or(0, |(_, word)| *word)
    })
}

/// Sets this thread's `s_doCalcDamage` in `e`'s run (0 clears it).
fn set_pending(e: &mut Engine, word: u32) {
    let run = state(e).run;
    PENDING.with(|p| {
        let mut p = p.borrow_mut();
        p.retain(|(r, _)| *r != run);
        if word != 0 {
            p.push((run, word));
        }
    });
}

/// This thread's `s_doCalcDamage` in `e`'s run, read and cleared (JIP's read, then `= 0`).
fn take_pending(e: &mut Engine) -> u32 {
    let word = pending_word(e);
    if word != 0 {
        set_pending(e, 0);
    }
    word
}

/// `s_VATSHitDT`: the damage threshold the last hit used (for G121).
pub fn vats_hit_dt(e: &mut Engine) -> f32 {
    f32::from_bits(state(e).vats_hit_dt.load(Ordering::Relaxed))
}

/// `s_condDRDTPenalty` set (for `SetArmorConditionPenalty`, G85's command).
pub fn set_condition_penalty(e: &mut Engine, value: f32) {
    state(e)
        .condition_penalty
        .store(value.to_bits(), Ordering::Relaxed);
}

/// `s_condDRDTPenalty` (O12's and G85's factor).
pub fn condition_penalty(e: &mut Engine) -> f32 {
    f32::from_bits(state(e).condition_penalty.load(Ordering::Relaxed))
}

/// `s_condDmgPenalty` (G84's factor).
pub fn damage_penalty(e: &mut Engine) -> f32 {
    f32::from_bits(state(e).damage_penalty.load(Ordering::Relaxed))
}

/// `s_condDmgPenalty` set (for `SetConditionDamagePenalty`, G84's command).
pub fn set_damage_penalty(e: &mut Engine, value: f32) {
    state(e)
        .damage_penalty
        .store(value.to_bits(), Ordering::Relaxed);
}

/// `s_noMovementCombat` (G126).
pub fn no_movement_combat(e: &mut Engine) -> bool {
    state(e).no_movement_combat.load(Ordering::Relaxed)
}

/// `s_noMovementCombat` set (for `ToggleNoMovementCombat`, G126's command).
pub fn set_no_movement_combat(e: &mut Engine, on: bool) {
    state(e).no_movement_combat.store(on, Ordering::Relaxed);
}

pub(super) fn install_fixes(e: &mut Engine) {
    // The run-wide statics, made before the threads start (`ext_free`'s rule).
    e.ext_free::<HitDamageState>();
    e.wrap(REDUCE_DAMAGE, reduce_damage);
    e.on_site(DEFER_OV3, defer_reduce_damage);
    e.on_site(DEFER_OV4, defer_reduce_damage);
    e.on_site(HIT_LOCATION, set_hit_location);
    e.on_site(AMMO_IMPACT_SCRIPT, ammo_impact_script);
}

/// O1's wrap of `009b5a30` (thiscall: the hit, `RET 4`: the word `noBlock`): with the switch
/// `bIgnoreDTDRFix` clear, the next link (the exe's `ReduceDamage`); else JIP's
/// `CalculateHitDamageHook`, a pending deferred call's word taking the place of this call's.
fn reduce_damage(e: &mut Engine, w: &[u32]) -> Ret {
    if e.jip_patches() & jip_patches::IGNORE_DTDR_FIX == 0 {
        return e.call_next(REDUCE_DAMAGE.addr(), w);
    }
    let pending = take_pending(e);
    let no_block = if pending != 0 { pending - 1 } else { w[1] };
    calculate_hit_damage(e, w[0], no_block);
    Ret::default()
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: CalculateHitDamageHook
// (its `_ReturnAddress()` test).
/// O1's deferral at a `ReduceDamage` call returning to `0x9B5628` or `0x9B5707` (`a`: the hit, the
/// call's word): with no deferred call pending, the word + 1 is kept and the call is not made
/// (answer 0); with one pending, no answer (the call is made and takes the pending word). The
/// word is this thread's (the module's "Threads").
fn defer_reduce_damage(e: &mut Engine, a: &[u32]) -> Option<u32> {
    if pending_word(e) != 0 {
        return None;
    }
    set_pending(e, a[1].wrapping_add(1));
    Some(0)
}

/// `HitData` (JIP's `ActorHitData`) fields.
mod hit {
    pub const SOURCE: u32 = 0x00;
    pub const TARGET: u32 = 0x04;
    pub const PROJECTILE: u32 = 0x08;
    pub const HIT_LOCATION: u32 = 0x10;
    pub const HEALTH: u32 = 0x14;
    pub const WPN_BASE: u32 = 0x18;
    pub const FATIGUE: u32 = 0x1c;
    pub const LIMB: u32 = 0x20;
    pub const BLOCK_DT_MOD: u32 = 0x24;
    pub const ARMOR: u32 = 0x28;
    pub const WEAPON_DMG: u32 = 0x2c;
    pub const WEAPON: u32 = 0x30;
    pub const VATS_COMMAND: u32 = 0x54;
    pub const FLAGS: u32 = 0x58;
}

/// Actor, process, weapon, armour and ammo fields JIP reads (`nvse/GameObjects.h`,
/// `GameProcess.h`, `GameForms.h` (JIP LN NVSE)).
const FORM_TYPE: u32 = 0x04;
const REF_ID: u32 = 0x0c;
const PLAYER_REF_ID: u32 = 0x14;
const MAGIC_TARGET: u32 = 0x94;
const AV_OWNER: u32 = 0xa4;
const BASE_PROCESS: u32 = 0x68;
const LIFE_STATE: u32 = 0x108;
const IS_TEAMMATE: u32 = 0x18d;
const BIPED_ANIMS: u32 = 0x1b4;
const COUNTER_ATTACK_TIMER: u32 = 0xe28;
const PROCESS_LEVEL: u32 = 0x28;
const WEAPON_INFO: u32 = 0x114;
const IS_WEAPON_OUT: u32 = 0x135;
const CURRENT_ACTION: u32 = 0x2ec;
const ANIM_ACTION_BLOCK: i16 = 7;
const WEAPON_ATTACK_DAMAGE: u32 = 0xa0;
const WEAPON_TYPE: u32 = 0xf4;
const WEAPON_FLAGS1: u32 = 0x100;
const WEAPON_RESIST_TYPE: u32 = 0x16c;
const ARMOR_HEALTH: u32 = 0x6c;
const ARMOR_RATING: u32 = 0x178;
const ARMOR_DT: u32 = 0x17c;
const AMMO_FLAGS: u32 = 0xac;
const AMMO_EFFECTS: u32 = 0xd4;
/// `kFormType_Creature`; the biped's slot 2 (`slotData[2].armor`, 0x10-byte slots from `+0x2C`).
const FORM_TYPE_CREATURE: u8 = 0x3c;
const BIPED_SLOT_2: u32 = 0x2c + 2 * 0x10;
const VTBL_ARMOR: u32 = 0x0102_a62c;
/// Virtual slots: `MagicTarget::CannotBeHit`, `ActorValueOwner::GetActorValue`,
/// `Actor::GetIsParalyzed`.
const CANNOT_BE_HIT: u32 = 0x10;
const GET_ACTOR_VALUE: u32 = 0x0c;
const GET_IS_PARALYZED: u32 = 0x234;
/// Actor value codes.
const AV_CHARISMA: u32 = 0x08;
const AV_DAMAGE_RESIST: u32 = 0x12;
const AV_MELEE_WEAPONS: u32 = 0x26;
const AV_UNARMED: u32 = 0x2d;
const AV_DAMAGE_THRESHOLD: u32 = 0x4c;
/// `kAmmoEffect_*` and `kPerkEntry_*`.
const AMMO_DAMAGE: u32 = 0;
const AMMO_DR: u32 = 1;
const AMMO_DT: u32 = 2;
const AMMO_FATIGUE: u32 = 5;
const PERK_CALCULATE_WEAPON_DAMAGE: u32 = 0x00;
const PERK_DT_DEFENDER: u32 = 0x38;
const PERK_DT_ATTACKER: u32 = 0x3a;
/// The extras JIP walks to an equipped armour's health.
const EXTRA_CONTAINER_CHANGES: u32 = 0x15;
const EXTRA_HEALTH: u32 = 0x25;
/// Game settings' values (`GMST_FLT`: the setting + 4) and globals.
const MAX_ARMOR_RATING: u32 = 0x011d_1020 + 4;
const BLOCK_SKILL_BASE: u32 = 0x011c_e840 + 4;
const BLOCK_SKILL_MULT: u32 = 0x011c_fd8c + 4;
const COUNTER_ATTACK_TIMER_SETTING: u32 = 0x011d_0be4 + 4;
const MIN_DAM_MULTIPLIER: u32 = 0x011c_ed4c + 4;
const DAMAGE_TO_ARMOR_PERCENTAGE: u32 = 0x011c_f0fc + 4;
const VATS_PLAYER_DAMAGE_MULT: u32 = 0x011c_e9f0 + 4;
/// `fVATSTargetedLimbDamageMult`'s value, which G116's limb 0xE step reads directly.
const VATS_LIMB_DAMAGE_MULT: u32 = 0x011c_e5d0 + 4;
const THE_PLAYER: u32 = 0x011d_ea3c;
const FISTS_WEAPON: u32 = 0x011c_a278;
/// `VATSCameraData::Get()->mode`.
const VATS_MODE: u32 = 0x011f_2250 + 8;
/// The exe functions JIP calls.
const APPLY_AMMO_EFFECTS: u32 = 0x0059_a030;
const APPLY_PERK_MODIFIERS: u32 = 0x005e_58f0;
const GET_THRESHOLDED_AV: u32 = 0x0066_ef50;
const CHARACTER_BASE_DT: u32 = 0x008d_2110;
const CHARACTER_BASE_DR: u32 = 0x008d_22b0;
const BLOCK_TEST: u32 = 0x009a_6ae0;
const HUD_PLAYER_HIT: u32 = 0x0077_f420;
const HUD_TARGET_HIT: u32 = 0x0077_f490;
const FOLLOWER_BARK: u32 = 0x008d_5cb0;
const RUN_SCRIPT_EFFECT: u32 = 0x005a_c340;

fn f(e: &Engine, at: u32) -> f32 {
    e.mem.f32(at)
}

/// `ApplyAmmoEffects(kind, effects, value)` (cdecl, `ST0`): its float.
fn apply_ammo(e: &mut Engine, kind: u32, effects: u32, value: f32) -> f32 {
    e.call(APPLY_AMMO_EFFECTS, &[kind, effects, value.to_bits()])
        .f64() as f32
}

/// `owner->avOwner.GetActorValue(code)`.
fn actor_value(e: &mut Engine, actor: u32, code: u32) -> f32 {
    e.vcall(actor.wrapping_add(AV_OWNER), GET_ACTOR_VALUE, &[code])
        .f64() as f32
}

/// `ApplyPerkModifiers(entry, owner, filter1, filter2, value)` (`0x5E58F0`, cdecl).
fn perk(e: &mut Engine, entry: u32, owner: u32, filter1: u32, filter2: u32, value: u32) {
    e.call(
        APPLY_PERK_MODIFIERS,
        &[entry, owner, filter1, filter2, value],
    );
}

/// `ApplyPerkModifiers` on a local `valueMod = 0` (a guest word for the call): its value after.
fn perk_on_zero(e: &mut Engine, entry: u32, owner: u32, filter1: u32, filter2: u32) -> f32 {
    e.with_stack(8, |e, local| {
        e.mem.set_f32(local.addr(), 0.0);
        perk(e, entry, owner, filter1, filter2, local.addr());
        e.mem.f32(local.addr())
    })
}

/// `GetMin` / `GetMax` (`internal/base_defs.h`): `(a < b) ? a : b`, `(a > b) ? a : b`.
fn get_min(a: f32, b: f32) -> f32 {
    if a < b {
        a
    } else {
        b
    }
}
fn get_max(a: f32, b: f32) -> f32 {
    if a > b {
        a
    } else {
        b
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp:
// TESObjectREFR::GetContainerChangesEntry (asm).
/// The reference's container changes entry of `form` (`ExtraContainerChanges` -> data -> its
/// list, the first entry whose form is `form`), 0 for none.
fn changes_entry(e: &mut Engine, refr: u32, form: u32) -> u32 {
    let extra = crate::commands::extra::refr_extra(e, refr, EXTRA_CONTAINER_CHANGES);
    if extra == 0 {
        return 0;
    }
    let data = e.mem.u32(extra + 0xc);
    if data == 0 {
        return 0;
    }
    let mut node = e.mem.u32(data);
    while node != 0 {
        let entry = e.mem.u32(node);
        node = e.mem.u32(node + 4);
        if entry != 0 && e.mem.u32(entry + 8) == form {
            return entry;
        }
    }
    0
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: CalculateHitDamageHook
// (after its deferral test).
/// JIP's hit damage on `hit` (`noBlock`: the block is not tried), in single floats in the C++
/// source's order. Nothing for a target that is not an actor; all damage 0 for one that cannot be
/// hit. Else: the base damage kept; ammo effects (the projectile's own ammo, G117's record) on
/// the fatigue, DR and DT; DT and DR from the target's actor values (less the hit body part's
/// armour with O12, for characters and limbs 0-12, scaled by the armour's condition below half),
/// teammates' raised by the player's Charisma; the block (a high process blocking: DT plus the
/// block skill bonus, the flags); ammo and perk entries on the damage; DR; the DT perk entries; the
/// threshold kept for VATS; split beams divide the DT; automatic melee weapons lose a share of
/// their damage, others the DT; the minimum damage (with the hit marker and the follower bark);
/// the player's armour damage; the weapon's resistance; the fatigue's share; VATS for the player.
pub fn calculate_hit_damage(e: &mut Engine, h: u32, no_block: u32) {
    let target = e.mem.u32(h + hit::TARGET);
    if target == 0 || !is_actor(e, target) {
        return;
    }
    if e.vcall(target + MAGIC_TARGET, CANNOT_BE_HIT, &[]).bool() {
        e.mem.set_f32(h + hit::HEALTH, 0.0);
        e.mem.set_f32(h + hit::WPN_BASE, 0.0);
        e.mem.set_f32(h + hit::FATIGUE, 0.0);
        return;
    }
    let source = e.mem.u32(h + hit::SOURCE);
    let mut hit_weapon = e.mem.u32(h + hit::WEAPON);
    let health = e.mem.u32(h + hit::HEALTH);
    e.mem.set_u32(h + hit::WPN_BASE, health);
    let reference = e.mem.u32(h + hit::PROJECTILE);
    let projectile = if reference != 0 && is_projectile(e, reference) {
        reference
    } else {
        0
    };
    let ammo = if projectile != 0 {
        super::super::gd::projectile_ammo(e, projectile)
    } else {
        0
    };
    let mut ammo_effects = 0;
    if ammo != 0 && e.mem.u32(ammo + AMMO_EFFECTS) != 0 {
        ammo_effects = ammo + AMMO_EFFECTS;
        let fatigue = f(e, h + hit::FATIGUE);
        let fatigue = apply_ammo(e, AMMO_FATIGUE, ammo_effects, fatigue);
        e.mem.set_f32(h + hit::FATIGUE, fatigue);
    }
    let npc_perks = e.jip_patches() & jip_patches::NPC_PERKS != 0;
    let mut flags = 0u8;
    if source != 0 {
        if e.mem.u32(source + REF_ID) == PLAYER_REF_ID {
            flags = 1;
        } else if e.mem.u8(source + IS_TEAMMATE) != 0 {
            flags = 2;
        } else if npc_perks {
            flags = 4;
        }
    }
    if e.mem.u32(target + REF_ID) == PLAYER_REF_ID {
        flags |= 8;
    } else if e.mem.u8(target + IS_TEAMMATE) != 0 {
        flags |= 0x10;
    } else if npc_perks {
        flags |= 0x20;
    }
    let mut dmg_resist: f32;
    let mut dmg_threshold: f32;
    let cpy_threshold: f32;
    if hit_weapon != 0 && e.mem.u8(hit_weapon + WEAPON_FLAGS1) & 1 != 0 {
        dmg_resist = 1.0;
        dmg_threshold = 0.0;
        cpy_threshold = 0.0;
    } else {
        let (hit_loc_dt, hit_loc_dr) = localized_dt_dr(e, h, target);
        let mut value_mod = 1.0f32;
        if flags & 0x10 != 0 {
            let player = e.mem.u32(THE_PLAYER);
            let charisma = e
                .call(
                    GET_THRESHOLDED_AV,
                    &[player.wrapping_add(AV_OWNER), AV_CHARISMA],
                )
                .f64() as f32;
            value_mod += charisma * 0.05;
        }
        dmg_resist = (actor_value(e, target, AV_DAMAGE_RESIST) - hit_loc_dr) * value_mod;
        dmg_resist = get_min(dmg_resist, 100.0);
        if ammo_effects != 0 {
            dmg_resist = apply_ammo(e, AMMO_DR, ammo_effects, dmg_resist);
        }
        if dmg_resist <= 0.0 {
            dmg_resist = 1.0;
        } else {
            dmg_resist = 1.0 - get_min(dmg_resist, f(e, MAX_ARMOR_RATING)) * 0.01;
        }
        dmg_threshold = (actor_value(e, target, AV_DAMAGE_THRESHOLD) - hit_loc_dt) * value_mod;
        cpy_threshold = dmg_threshold;
        if ammo_effects != 0 {
            dmg_threshold = apply_ammo(e, AMMO_DT, ammo_effects, dmg_threshold);
        }
    }
    let process = e.mem.u32(target + BASE_PROCESS);
    if no_block == 0
        && process != 0
        && e.mem.u32(process + PROCESS_LEVEL) == 0
        && e.mem.i16(process + CURRENT_ACTION) == ANIM_ACTION_BLOCK
        && !e.vcall(target, GET_IS_PARALYZED, &[]).bool()
        && e.call(
            BLOCK_TEST,
            &[target, if projectile != 0 { projectile } else { source }, 0],
        )
        .bool()
    {
        dmg_threshold += block(e, h, target, process, hit_weapon, projectile, flags);
    }
    if hit_weapon != 0 {
        if ammo_effects != 0 {
            let health = f(e, h + hit::HEALTH);
            let health = apply_ammo(e, AMMO_DAMAGE, ammo_effects, health);
            e.mem.set_f32(h + hit::HEALTH, health);
        }
        if flags & 7 != 0 {
            perk(
                e,
                PERK_CALCULATE_WEAPON_DAMAGE,
                source,
                hit_weapon,
                target,
                h + hit::HEALTH,
            );
        }
    } else if source != 0 {
        hit_weapon = e.mem.u32(FISTS_WEAPON);
        e.mem.set_u32(h + hit::WEAPON, hit_weapon);
    }
    let health = f(e, h + hit::HEALTH) * dmg_resist;
    e.mem.set_f32(h + hit::HEALTH, health);
    if flags & 0x38 != 0 {
        let filter = if hit_weapon != 0 {
            hit_weapon
        } else {
            e.mem.u32(FISTS_WEAPON)
        };
        dmg_threshold += perk_on_zero(e, PERK_DT_DEFENDER, target, source, filter);
    }
    if flags & 7 != 0 {
        dmg_threshold -= perk_on_zero(e, PERK_DT_ATTACKER, source, hit_weapon, target);
    }
    state(e)
        .vats_hit_dt
        .store(dmg_threshold.to_bits(), Ordering::Relaxed);
    let flag_arg;
    if dmg_threshold > 0.0 {
        if projectile != 0
            && e.mem.u8(projectile + super::super::gd::HAS_SPLIT_BEAMS) != 0
            && e.mem.u8(projectile + super::super::gd::NUM_PROJECTILES) > 1
        {
            dmg_threshold /= f32::from(e.mem.u8(projectile + super::super::gd::NUM_PROJECTILES));
        }
        dmg_threshold *= dmg_resist;
        flag_arg = false;
    } else {
        dmg_threshold = 0.0;
        flag_arg = cpy_threshold > 0.0;
    }
    if hit_weapon != 0
        && e.mem.u8(hit_weapon + WEAPON_FLAGS1) & 2 != 0
        && e.mem.u8(hit_weapon + WEAPON_TYPE) <= 2
    {
        let mut attack = i32::from(e.mem.u16(hit_weapon + WEAPON_ATTACK_DAMAGE));
        if attack == 0 {
            attack = 1;
        }
        let value_mod = 1.0 - dmg_threshold / attack as f32;
        if value_mod > 0.0 {
            let health = f(e, h + hit::HEALTH) * value_mod;
            e.mem.set_f32(h + hit::HEALTH, health);
        }
    } else {
        let health = f(e, h + hit::HEALTH) - dmg_threshold;
        e.mem.set_f32(h + hit::HEALTH, health);
    }
    let minimum = f(e, MIN_DAM_MULTIPLIER) * f(e, h + hit::WPN_BASE);
    let dead = |e: &Engine| {
        let state = e.mem.u32(target + LIFE_STATE);
        state == 1 || state == 2
    };
    if f(e, h + hit::HEALTH) > minimum {
        let bits = e.mem.u32(h + hit::FLAGS);
        e.mem.set_u32(h + hit::FLAGS, bits | 0x8000_0000);
        if flag_arg {
            if flags & 8 != 0 {
                e.call(HUD_PLAYER_HIT, &[1]);
            } else if flags & 1 != 0 && !dead(e) {
                e.call(HUD_TARGET_HIT, &[1]);
            }
        }
    } else {
        e.mem.set_f32(h + hit::HEALTH, minimum);
        if flags & 8 != 0 {
            e.call(HUD_PLAYER_HIT, &[u32::from(flag_arg)]);
        } else if flags & 1 != 0 && !dead(e) {
            e.call(HUD_TARGET_HIT, &[u32::from(flag_arg)]);
        }
        if flags & 2 != 0 {
            e.call(FOLLOWER_BARK, &[source, 9]);
        }
    }
    if flags & 8 != 0 && f(e, h + hit::FATIGUE) <= 0.0 {
        let health = f(e, h + hit::HEALTH);
        let armor = (minimum * (1.0 - dmg_resist) + get_min(health, dmg_threshold))
            * f(e, DAMAGE_TO_ARMOR_PERCENTAGE);
        e.mem.set_f32(h + hit::ARMOR, armor);
    }
    if hit_weapon != 0 {
        let resist_type = e.mem.u32(hit_weapon + WEAPON_RESIST_TYPE);
        if resist_type as i32 != -1 && (ammo == 0 || e.mem.u8(ammo + AMMO_FLAGS) & 1 == 0) {
            let value_mod = actor_value(e, target, resist_type);
            if value_mod > 0.0 {
                let health = if value_mod < 100.0 {
                    f(e, h + hit::HEALTH) * (1.0 - value_mod * 0.01)
                } else {
                    0.0
                };
                e.mem.set_f32(h + hit::HEALTH, health);
            }
        }
    }
    let fatigue = f(e, h + hit::FATIGUE);
    let base = f(e, h + hit::WPN_BASE);
    if fatigue > 0.0 && base > 0.0 {
        let ratio = f(e, h + hit::HEALTH) / base;
        e.mem.set_f32(h + hit::FATIGUE, fatigue * ratio);
    }
    if flags & 8 != 0 && e.mem.u32(VATS_MODE) != 0 {
        let value_mod = f(e, VATS_PLAYER_DAMAGE_MULT);
        for field in [
            hit::HEALTH,
            hit::WPN_BASE,
            hit::FATIGUE,
            hit::LIMB,
            hit::ARMOR,
            hit::WEAPON_DMG,
        ] {
            let value = f(e, h + field) * value_mod;
            e.mem.set_f32(h + field, value);
        }
    }
}

/// O12 (`bLocalizedDTDR`): the hit body part's DT and DR to take off the target's (0, 0 without
/// the switch, for a creature, a limb outside 0-12 or no biped). The armour in biped slot 2 (a
/// `TESObjectARMO` the target has equipped) gives its DT and rating / 100, scaled by `1 - (0.5 -
/// condition) * penalty` below half condition; for a limb other than 1 the character's own base
/// DT and DR (`0x8D2110`, `0x8D22B0`) less those, at least 0, are taken off instead.
fn localized_dt_dr(e: &mut Engine, h: u32, target: u32) -> (f32, f32) {
    let location = e.mem.u32(h + hit::HIT_LOCATION) as i32;
    if e.jip_patches() & jip_patches::LOCALIZED_DTDR == 0
        || e.mem.u8(target + FORM_TYPE) == FORM_TYPE_CREATURE
        || !(0..=12).contains(&location)
    {
        return (0.0, 0.0);
    }
    let biped = e.mem.u32(target + BIPED_ANIMS);
    if biped == 0 {
        return (0.0, 0.0);
    }
    let mut hit_loc_dt = 0.0f32;
    let mut hit_loc_dr = 0.0f32;
    let armor = e.mem.u32(biped + BIPED_SLOT_2);
    if armor != 0 && e.mem.u32(armor) == VTBL_ARMOR {
        let entry = changes_entry(e, target, armor);
        if entry != 0 {
            let list = crate::commands::ln_fn_actor::equipped_extra(e, entry);
            if list != 0 {
                hit_loc_dt = f(e, armor + ARMOR_DT);
                hit_loc_dr = f32::from(e.mem.u16(armor + ARMOR_RATING)) * 0.01;
                let health = crate::commands::extra::get_by_type(e, list, EXTRA_HEALTH);
                if health != 0 {
                    let full = e.mem.u32(armor + ARMOR_HEALTH) as i32;
                    let value_mod = f(e, health + 0xc) / full as f32;
                    if value_mod < 0.5 {
                        let penalty =
                            f32::from_bits(state(e).condition_penalty.load(Ordering::Relaxed));
                        let value_mod = 1.0 - (0.5 - value_mod) * penalty;
                        hit_loc_dt *= value_mod;
                        hit_loc_dr *= value_mod;
                    }
                }
            }
        }
    }
    if location != 1 {
        let base_dt = e.call(CHARACTER_BASE_DT, &[target]).f64() as f32;
        hit_loc_dt = get_max(base_dt - hit_loc_dt, 0.0);
        let base_dr = e.call(CHARACTER_BASE_DR, &[target]).f64() as f32;
        hit_loc_dr = get_max(base_dr - hit_loc_dr, 0.0);
    }
    (hit_loc_dt, hit_loc_dr)
}

/// O1's block (the target's high process blocking the hit): the hit's flag 1, the player's
/// counter-attack timer, the block DT bonus (`fBlockSkillBase` + `fBlockSkillMult` x the target's
/// Unarmed or Melee Weapons; 0 for a ranged hit while the target holds nothing) and flag 2 (a
/// held weapon drawn). Answers what the threshold gains.
fn block(
    e: &mut Engine,
    h: u32,
    target: u32,
    process: u32,
    hit_weapon: u32,
    projectile: u32,
    flags: u8,
) -> f32 {
    let weapon_info = e.mem.u32(process + WEAPON_INFO);
    let mut no_info = weapon_info == 0;
    let p_weapon = if weapon_info != 0 {
        e.mem.u32(weapon_info + 8)
    } else {
        0
    };
    let bits = e.mem.u32(h + hit::FLAGS);
    e.mem.set_u32(h + hit::FLAGS, bits | 1);
    if flags & 8 != 0 {
        let player = e.mem.u32(THE_PLAYER);
        let timer = e.mem.u32(COUNTER_ATTACK_TIMER_SETTING);
        e.mem
            .set_u32(player.wrapping_add(COUNTER_ATTACK_TIMER), timer);
    }
    if no_info {
        if (hit_weapon == 0 || e.mem.u8(hit_weapon + WEAPON_TYPE) <= 2) && projectile == 0 {
            no_info = false;
        } else {
            e.mem.set_f32(h + hit::BLOCK_DT_MOD, 0.0);
        }
    }
    let mut gain = 0.0f32;
    if !no_info {
        let skill = if p_weapon == 0 || e.mem.u8(p_weapon + WEAPON_TYPE) == 0 {
            AV_UNARMED
        } else if e.mem.u8(p_weapon + WEAPON_TYPE) <= 2 {
            AV_MELEE_WEAPONS
        } else {
            0
        };
        let base = e.mem.u32(BLOCK_SKILL_BASE);
        e.mem.set_u32(h + hit::BLOCK_DT_MOD, base);
        if skill != 0 {
            let bonus = f(e, BLOCK_SKILL_MULT) * actor_value(e, target, skill);
            let value = f(e, h + hit::BLOCK_DT_MOD) + bonus;
            e.mem.set_f32(h + hit::BLOCK_DT_MOD, value);
        }
        gain = f(e, h + hit::BLOCK_DT_MOD);
    }
    let bits = e.mem.u32(h + hit::FLAGS);
    if p_weapon != 0 && e.mem.u8(process + IS_WEAPON_OUT) != 0 {
        e.mem.set_u32(h + hit::FLAGS, bits | 2);
    } else {
        e.mem.set_u32(h + hit::FLAGS, bits & !2);
    }
    gain
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: SetHitLocationHook.
/// G116 at `SetTargetedLimb`'s limb step (`a[0]`: the hit): a deferred `CalculateHitDamage`
/// runs first (its word: the pending one); then for limb 0xE the part is dropped, the health
/// damage moves to the limb damage (times `fVATSTargetedLimbDamageMult`'s value with a VATS
/// command, `mulss`; a plain copy without) and is cleared: answer 1. Other limbs: answer 0.
fn set_hit_location(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let h = a[0];
    let pending = take_pending(e);
    if pending != 0 {
        calculate_hit_damage(e, h, pending - 1);
    }
    if e.mem.u32(h + hit::HIT_LOCATION) != 0xe {
        return Some(0);
    }
    let mut damage = e.mem.u32(h + hit::HEALTH);
    e.mem.set_u32(h + hit::HEALTH, 0);
    if e.mem.u32(h + hit::VATS_COMMAND) != 0 {
        damage = (f32::from_bits(damage) * f(e, VATS_LIMB_DAMAGE_MULT)).to_bits();
    }
    e.mem.set_u32(h + hit::LIMB, damage);
    Some(1)
}

/// `TESAmmo`'s vtable and its script (`TESScriptableForm::script`, `+0xA0`).
const VTBL_AMMO: u32 = 0x0102_6064;
const AMMO_SCRIPT: u32 = 0xa0;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: RunAmmoImpactScriptHook.
/// G119 in `Actor::HitMe` (`a`: the hit, the actor hit): the ammo G117 recorded in the hit's
/// projectile (`+0x60`, read directly, no count test) when it is a `TESAmmo` with a script: the
/// script's effect started on the actor (`005ac340(script, actor, 0)`). Always answers (JIP's
/// jump skips the exe's current-ammo calls).
fn ammo_impact_script(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let projectile = e.mem.u32(a[0] + hit::PROJECTILE);
    if projectile != 0 {
        let ammo = e.mem.u32(projectile + super::super::gd::AMMO);
        if ammo != 0 && e.mem.u32(ammo) == VTBL_AMMO {
            let script = e.mem.u32(ammo + AMMO_SCRIPT);
            if script != 0 {
                e.call(RUN_SCRIPT_EFFECT, &[script, a[1], 0]);
            }
        }
    }
    Some(0)
}

#[cfg(test)]
#[path = "gc_damage_tests.rs"]
mod tests;
