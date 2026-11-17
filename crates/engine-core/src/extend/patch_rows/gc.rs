//! Group GC's seams and wraps (step 3.9: hit damage, detection, combat formulas). Only group GC's PR edits this file; rows by
//! address, owner `super::JIP`, step "3.9-GC". Rows of the patches this group
//! writes the handlers of (its home rows), seams in other groups' files included.

use super::super::Entry;
use super::JIP;

/// The group's seams.
pub(in super::super) const SITES: &[Entry] = &[
    // G14 (a fix), fn_0088db20: the last word of UnEquipObject for a worn item it takes off.
    Entry {
        addr: 0x0088_dcdc,
        owners: JIP,
        step: "3.9-GC",
    },
    // G120 (a fix), fn_0088e8d0: the hit blood's impact data (CreateHitBloodHook).
    Entry {
        addr: 0x0088_ebaa,
        owners: JIP,
        step: "3.9-GC",
    },
    // G119 (a fix), Actor::HitMe 0089a920: the ammo's script effect, from the projectile's own ammo (RunAmmoImpactScriptHook).
    Entry {
        addr: 0x0089_abdc,
        owners: JIP,
        step: "3.9-GC",
    },
    // G99 (JIP's own), Actor::HitMe: the worn armour's damage, capped by SetDamageToArmorMaxPercent (DoHitArmorDamageHook).
    Entry {
        addr: 0x0089_d2ff,
        owners: JIP,
        step: "3.9-GC",
    },
    // G98 (a fix), Actor::Kill 0089d900: the teammate test before the kill challenges (KillChallengeFixHook).
    Entry {
        addr: 0x0089_dc0e,
        owners: JIP,
        step: "3.9-GC",
    },
    // G33 (JIP's own), Actor::Kill: the player's killer stored.
    Entry {
        addr: 0x0089_e790,
        owners: JIP,
        step: "3.9-GC",
    },
    // G33, Actor::Kill: the player's killer kept after the progress export.
    Entry {
        addr: 0x0089_f351,
        owners: JIP,
        step: "3.9-GC",
    },
    // G126 (JIP's own), fn_008a03a0: the player's control flag test, false with ToggleNoMovementCombat (GetControlFlagHook).
    Entry {
        addr: 0x008a_04b2,
        owners: JIP,
        step: "3.9-GC",
    },
    // G53, Actor::GetDetectionLevelAgainstActor 008a0d10: its three calls of the detected
    // actor's teammate test (0056_6950), replaced by JIP's DetectionTeammateHook.
    Entry {
        addr: 0x008a_0dab,
        owners: JIP,
        step: "3.9-GC",
    },
    Entry {
        addr: 0x008a_1291,
        owners: JIP,
        step: "3.9-GC",
    },
    Entry {
        addr: 0x008a_131d,
        owners: JIP,
        step: "3.9-GC",
    },
    // G53, the same function: the detected actor's level word (its actor value owner's virtual
    // +0x28), replaced by JIP's DetectionGetLevelHook.
    Entry {
        addr: 0x008a_147c,
        owners: JIP,
        step: "3.9-GC",
    },
    // G18 (a fix), Actor::SetLifeState 008a1800: the change mark 0x400 becomes 0x404.
    Entry {
        addr: 0x008a_19cf,
        owners: JIP,
        step: "3.9-GC",
    },
    // O19 (bCreatureSpreadFix), fn_008b0dd0: the iron sights test in the gun skill value (CreatureSpreadFixHook).
    Entry {
        addr: 0x008b_0fbf,
        owners: JIP,
        step: "3.9-GC",
    },
    // G118 (a fix), fn_008c2190: the weapon test before the projectile's own weapon (the jnz removed).
    Entry {
        addr: 0x008c_227f,
        owners: JIP,
        step: "3.9-GC",
    },
    // O1 (bIgnoreDTDRFix), HitData::InitializeHitData_ov3 009b5170: its ReduceDamage call, deferred to SetTargetedLimb.
    Entry {
        addr: 0x009b_5623,
        owners: JIP,
        step: "3.9-GC",
    },
    // O1, HitData::InitializeHitData_ov4 009b5650: the same.
    Entry {
        addr: 0x009b_5702,
        owners: JIP,
        step: "3.9-GC",
    },
    // G77 (JIP's own), HitData::ReduceDamage 009b5a30: the minimum damage store, with the armour-penetrated flag.
    Entry {
        addr: 0x009b_6335,
        owners: JIP,
        step: "3.9-GC",
    },
    // G116 (a fix), HitData::SetTargetedLimb 009b6620: a deferred CalculateHitDamage, then limb 0xE (SetHitLocationHook).
    Entry {
        addr: 0x009b_6e81,
        owners: JIP,
        step: "3.9-GC",
    },
];

/// The group's wraps.
pub(in super::super) const WRAPS: &[Entry] = &[
    // G109 (a fix): BGSImpactDataSet's impact data of a material (GetImpactDataHook).
    Entry {
        addr: 0x0058_e9d0,
        owners: JIP,
        step: "3.9-GC",
    },
    // G111 + O11 (a fix): the damage a shot does to the weapon (GetDamageToWeaponHook).
    Entry {
        addr: 0x0064_6260,
        owners: JIP,
        step: "3.9-GC",
    },
    // G85 (JIP's own): CombatFormulas::CalcArmorRating, the condition factor times SetArmorConditionPenalty's (GetArmorConditionPenaltyHook).
    Entry {
        addr: 0x0064_6360,
        owners: JIP,
        step: "3.9-GC",
    },
    // G84 (JIP's own): a weapon's condition damage factor, times SetConditionDamagePenalty's (GetConditionDamagePenaltyHook).
    Entry {
        addr: 0x0064_6d00,
        owners: JIP,
        step: "3.9-GC",
    },
    // O1 + O12 (bIgnoreDTDRFix, bLocalizedDTDR): HitData::ReduceDamage, JIP's hit damage (CalculateHitDamageHook).
    Entry {
        addr: 0x009b_5a30,
        owners: JIP,
        step: "3.9-GC",
    },
];
