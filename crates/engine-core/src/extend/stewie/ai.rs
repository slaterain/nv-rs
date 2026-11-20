//! Share 2's rows (AI, actors and the player: actors, processes, combat, packages, projectiles,
//! the player character, animation and the model loader). Only share 2 adds rows here (see
//! `super::world` for the rule). A row is `Row::share(AI_FIXES, AI_FIXES_END, n)` (or the
//! tweaks' pair), which does not compile past the share's last bit.

use super::Row;

/// Share 2's fix rows: bits `300..470`.
pub const AI_FIXES: u16 = super::forms::FORMS_FIXES_END;
/// One past share 2's last fix bit.
pub const AI_FIXES_END: u16 = 470;
/// Share 2's tweak rows: bits `768..896`.
pub const AI_TWEAKS: u16 = super::forms::FORMS_TWEAKS_END;
/// One past share 2's last tweak bit.
pub const AI_TWEAKS_END: u16 = 896;

/// S0147 `patchFixCrippledLimbAnimsNotUpdatingIfLimbsFixed`
/// (`Actor::MobilityConditionModifiedCallback` `008b9830` (`0x8B9950`)): a crippled leg that is
/// healed brings the normal walk back.
pub const CRIPPLED_LIMB_ANIMS: Row = Row::share(AI_FIXES, AI_FIXES_END, 0);
/// S0159 `patchFixInfiniteNPCLoopingReloads` (`008ba600` (`0x8BAB65`)): an NPC no longer reloads
/// over and over when it carries fewer rounds than the reload would load.
pub const INFINITE_NPC_RELOADS: Row = Row::share(AI_FIXES, AI_FIXES_END, 1);
/// S0205 `EnduranceHealthRestoreFix` (`ValueModifierEffect::Start` `0082c000` (`0x82C221`);
/// `Actor::EnduranceModifiedCallback` `008b9ea0` (`0x8B9EFC`)): an Endurance drop kills only an
/// actor with no health left (not one at 1 health), and no longer restores health.
pub const ENDURANCE_HEALTH_RESTORE: Row = Row::share(AI_FIXES, AI_FIXES_END, 2);
/// S0206 `CreatureDRFix` (`Creature`'s vtable slot `010874E8`, `008d4cc0`): a creature's Damage
/// Resistance is no longer counted twice.
pub const CREATURE_DR: Row = Row::share(AI_FIXES, AI_FIXES_END, 3);
/// S0207 `NonHitscanProjectileSplashFix` (`009bf370` (`0x9BF461`)): a projectile that is not
/// hitscan moves its rigid body too, so it splashes water.
pub const NON_HITSCAN_PROJECTILE_SPLASH: Row = Row::share(AI_FIXES, AI_FIXES_END, 4);
/// S0217 `RagdollControllerUpdateFix` (`008bcdf0` (`0x8BCED5`)): the player's leg IK is no longer
/// reset when switching to first person.
pub const RAGDOLL_CONTROLLER_UPDATE: Row = Row::share(AI_FIXES, AI_FIXES_END, 5);
/// S0221 `DeathAnimFix` (`008ba600` (`0x8BA6EA`)): a death animation plays even while a movement
/// sequence is easing out.
pub const DEATH_ANIM: Row = Row::share(AI_FIXES, AI_FIXES_END, 6);
/// S0229 `ArmorUpdateFix` (`008d4e80`, a jump at its entry): an actor's (NPC's or creature's) DR and
/// DT follow a change of its armour.
pub const ARMOR_UPDATE: Row = Row::share(AI_FIXES, AI_FIXES_END, 7);
/// S0180 `MineDisarmedLightLoadingFix` (`Projectile::LoadGame` `009c52f0` (`0x9C5723`)): a mine's
/// light shows again after a save is loaded.
pub const MINE_DISARMED_LIGHT_LOADING: Row = Row::share(AI_FIXES, AI_FIXES_END, 8);
