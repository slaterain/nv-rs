//! Share 2's rows (AI, actors and the player; their bits in `engine_core::extend::stewie::ai`), in
//! PLAN.md's order. Only share 2 adds rows here.

use super::Row;
use engine_core::extend::stewie;

/// Share 2's rows built.
pub const ROWS: &[Row] = &[
    Row {
        bit: stewie::CRIPPLED_LIMB_ANIMS,
        function: "patchFixCrippledLimbAnimsNotUpdatingIfLimbsFixed",
        plan: "S0147",
        key: "bFixCrippledLimbAnimsNotUpdatingIfLimbsFixed",
        tweaks: None,
        summary: "healing a crippled leg brings the normal walk back (the game kept the limping walk)",
    },
    Row {
        bit: stewie::INFINITE_NPC_RELOADS,
        function: "patchFixInfiniteNPCLoopingReloads",
        plan: "S0159",
        key: "bFixInfiniteNPCLoopingReloads",
        tweaks: None,
        summary: "an NPC with fewer rounds than its reload would load no longer reloads over and over",
    },
    Row {
        bit: stewie::MINE_DISARMED_LIGHT_LOADING,
        function: "MineDisarmedLightLoadingFix",
        plan: "S0180",
        key: "bMineDisarmedLightLoadingFix",
        tweaks: None,
        summary: "a mine's light shows correctly again after a save is loaded",
    },
    Row {
        bit: stewie::ENDURANCE_HEALTH_RESTORE,
        function: "EnduranceHealthRestoreFix",
        plan: "S0205",
        key: "bEnduranceHealthRestoreFix",
        tweaks: None,
        summary: "changes play: an Endurance drop kills an actor only at 0 health or less (the game killed at 1 or less), and no longer restores health",
    },
    Row {
        bit: stewie::CREATURE_DR,
        function: "CreatureDRFix",
        plan: "S0206",
        key: "bCreatureDRFix",
        // Tweaks installs it only when `[INI] bFixCreatureDoubledDR` (code default 1) is not 0.
        tweaks: Some(("INI", "bFixCreatureDoubledDR")),
        summary: "changes play: a creature's Damage Resistance is counted once, not twice (creatures take more damage than in the game)",
    },
    Row {
        bit: stewie::NON_HITSCAN_PROJECTILE_SPLASH,
        function: "NonHitscanProjectileSplashFix",
        plan: "S0207",
        key: "bNonHitscanProjectileSplashFix",
        tweaks: None,
        summary: "projectiles that are not hitscan (grenades, missiles, slow bullets) move their physics body too, so they splash water",
    },
    Row {
        bit: stewie::RAGDOLL_CONTROLLER_UPDATE,
        function: "RagdollControllerUpdateFix",
        plan: "S0217",
        key: "bRagdollControllerUpdateFix",
        tweaks: None,
        summary: "the player's leg IK is no longer reset when switching between first and third person",
    },
    Row {
        bit: stewie::DEATH_ANIM,
        function: "DeathAnimFix",
        plan: "S0221",
        key: "bDeathAnimFix",
        tweaks: None,
        summary: "a death animation plays even while a movement animation is easing out",
    },
    Row {
        bit: stewie::ARMOR_UPDATE,
        function: "ArmorUpdateFix",
        plan: "S0229",
        key: "bArmorUpdateFix",
        tweaks: None,
        summary: "changes play: an actor's (NPC's or creature's) DR and DT follow a change of its armour at once",
    },
];
