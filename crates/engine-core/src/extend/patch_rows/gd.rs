//! Group GD's seams and wraps (step 3.9: projectiles, explosions, combat AI, packages). Only group GD's PR edits this file; rows by
//! address, owner `super::JIP`, step "3.9-GD". Rows of the patches this group
//! writes the handlers of (its home rows), seams in other groups' files included.

use super::super::Entry;
use super::JIP;

/// The group's seams.
pub(in super::super) const SITES: &[Entry] = &[
    // G27, fn_00475b20: the explosion source of a destroyed projectile-like reference (the
    // shooter call, `mov eax, ecx` in JIP).
    Entry {
        addr: 0x0047_667d,
        owners: JIP,
        step: "3.9-GD",
    },
    // G62, HighProcess::CalculateMoveMode 008daa20: its three run tests, replaced by JIP's
    // PackageSetRunHook.
    Entry {
        addr: 0x008d_aa6f,
        owners: JIP,
        step: "3.9-GD",
    },
    // G101, the combat dialogue line 009839b0: a death response's speaker call, replaced by JIP's
    // DeathResponseFixHook (the nearest living ally).
    Entry {
        addr: 0x0098_4156,
        owners: JIP,
        step: "3.9-GD",
    },
    // G30, the explosion factory 009ac9c0: a projectile source's shooter and perk entry
    // (JIP's 53 bytes).
    Entry {
        addr: 0x009a_ca23,
        owners: JIP,
        step: "3.9-GD",
    },
    // G124, Explosion::Initialize 009acd60: the explosion-in-face image space, JIP's
    // GetExplosionInFaceIMODHook (none while its hit effects are off).
    Entry {
        addr: 0x009a_cec6,
        owners: JIP,
        step: "3.9-GD",
    },
    // G125, Explosion::Initialize 009acd60: the screen shake, JIP's ExplosionScreenShakeHook
    // (skipped while its hit effects are off).
    Entry {
        addr: 0x009a_d2c5,
        owners: JIP,
        step: "3.9-GD",
    },
    // O4 (bProjImpactDmgFix), 009b7cc0: the missile's flag 0x8000 call, replaced by JIP's
    // InitMissileFlagsHook.
    Entry {
        addr: 0x009b_7e14,
        owners: JIP,
        step: "3.9-GD",
    },
    // G117, the projectile's launch constructor 009bbef0: its projectile count, damage division
    // and mod scale (009bc241..009bc293), replaced by JIP's InitProjectileSetAmmoHook, which also
    // records the ammo, the split-beam flag and the count in the projectile.
    Entry {
        addr: 0x009b_c241,
        owners: JIP,
        step: "3.9-GD",
    },
    // G27, the projectile's explosion: its source (the shooter call, `mov eax, ecx` in JIP).
    Entry {
        addr: 0x009c_35f7,
        owners: JIP,
        step: "3.9-GD",
    },
    // G127, the mine trigger test 009c39e0: an actor's trigger, JIP's MineExplodeChanceHook (a
    // roll of the explode chance through entry point 4).
    Entry {
        addr: 0x009c_3bd6,
        owners: JIP,
        step: "3.9-GD",
    },
    // G91, CombatProcedureAttackRanged::Update 009d0a30: the reaction cooldown test, replaced by
    // JIP's ReactionCooldownCheckHook.
    Entry {
        addr: 0x009d_10f3,
        owners: JIP,
        step: "3.9-GD",
    },
    // G42, the backup package's point 009ed380: the move-away distance (JIP's variable).
    Entry {
        addr: 0x009e_d3f8,
        owners: JIP,
        step: "3.9-GD",
    },
    // G42, the backup package's point 009ed4b0: the move-away distance (JIP's variable).
    Entry {
        addr: 0x009e_d528,
        owners: JIP,
        step: "3.9-GD",
    },
    // G102, DialoguePackage::Speak 009ee0a0: the speaker's two state calls (JIP's 21 bytes).
    Entry {
        addr: 0x009e_e367,
        owners: JIP,
        step: "3.9-GD",
    },
    // G92, the sandbox scan callback 009f5070: the target's three flag tests, replaced by JIP's
    // IsValidAITargetHook.
    Entry {
        addr: 0x009f_50c4,
        owners: JIP,
        step: "3.9-GD",
    },
];

/// The group's wraps.
pub(in super::super) const WRAPS: &[Entry] = &[];
