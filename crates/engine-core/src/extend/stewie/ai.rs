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
/// S0137 `patchPreventAgroRadiusBreachedNPCsHolsteringWeapons` (`HighProcess::Update` `008eeec0`
/// (`0x8F037C`)): an actor whose guard warning radius was crossed keeps its weapon out.
pub const AGRO_RADIUS_HOLSTER: Row = Row::share(AI_FIXES, AI_FIXES_END, 9);
/// S0199.01 `MiscCrashFixes` (`HighProcess::ProcessDialogueActivate` `008e8600` (`0x8E941C`)): no
/// crash when the actor has no package.
pub const DIALOGUE_ACTIVATE_NO_PACKAGE: Row = Row::share(AI_FIXES, AI_FIXES_END, 10);
/// S0039 `fixWeaponIdleSoundsHavingInfiniteRange` (`MiddleHighProcess::ForceWeaponDrawnSheathed`
/// `009231d0` (`0x923820`)): in third person an NPC's weapon idle sound is no longer heard from
/// anywhere, and the player's own plays as in first person.
pub const WEAPON_IDLE_SOUND_RANGE: Row = Row::share(AI_FIXES, AI_FIXES_END, 11);
/// S0171 `KnockDownHandGripFix` (`MiddleHighProcess::UpdateKnockState` `00920150` (`0x920A91`)):
/// getting up from a knockdown queues the hand-grip idle.
pub const KNOCKDOWN_HAND_GRIP: Row = Row::share(AI_FIXES, AI_FIXES_END, 12);
/// S0146 `NPCThrowingAnimFix` (`CombatProcedureAttackGrenade::Update` `009cafe0` (`0x9CB337`)): an
/// NPC's throw plays the weapon's own attack animation.
pub const NPC_THROWING_ANIM: Row = Row::share(AI_FIXES, AI_FIXES_END, 13);
/// S0214 `CombatItemCooldownFix` (`CombatProcedureUseCombatItem::Update` `009daab0` (`0x9DAB61`)):
/// an NPC's restore item waits `fCombatItemRestoreTimer`, not `fCombatItemBuffTimer`.
pub const COMBAT_ITEM_COOLDOWN: Row = Row::share(AI_FIXES, AI_FIXES_END, 14);
/// S0228 `PatrolPackageFix` (`009f2a70` (`0x9F2ABB`)): no crash loading a patrol package with no
/// actor.
pub const PATROL_PACKAGE: Row = Row::share(AI_FIXES, AI_FIXES_END, 15);
/// S0133 `patchFixSpectatorThreatInfoUsingWrongWeaponType` (`009f6d50` (`0x9F6E76`)): a
/// spectator's threat reads the weapon's combat type.
pub const SPECTATOR_THREAT_WEAPON_TYPE: Row = Row::share(AI_FIXES, AI_FIXES_END, 16);
/// S0211 `ThrowableRotationFix` (`MissileProjectile::UpdateProjectile` `009b8030` (`0x9B8552`)): a
/// spinning throwable (its projectile's rotation flag) is not turned along its arc.
pub const THROWABLE_ROTATION: Row = Row::share(AI_FIXES, AI_FIXES_END, 17);
/// S0230 `DoubleSoundFix` (`BSAnimGroupSequence::PlaySounds` `004eef00` (`0x4EEF3C`)): a sequence
/// not yet updated plays no text-key sounds, so they are not heard twice.
pub const DOUBLE_SOUND: Row = Row::share(AI_FIXES, AI_FIXES_END, 18);
/// S0074 `patchFixCombatMissileDepthGamesettings` (`009ba660` (`0x9BA8AC`); `009b90b0`
/// (`0x9B9626`, `0x9B9AE3`)): the missile stick and impale depths read their settings each time.
pub const COMBAT_MISSILE_DEPTH_SETTINGS: Row = Row::share(AI_FIXES, AI_FIXES_END, 19);
/// S0024 `patchPreventFollowerNoAmmoTopicIfUsingRechargerWeapon` (`008a89a0` (`0x8A8DA5`)): a
/// companion with a recharging weapon says no out-of-ammo line.
pub const FOLLOWER_NO_AMMO_RECHARGER: Row = Row::share(AI_FIXES, AI_FIXES_END, 20);
/// S0041 `fixWeaponModGetDestructibleForm` (`BGSDestructibleObjectForm::GetDestructionForm`
/// `00475400` (`0x4754CF`)): a weapon mod's destructible data is read at its own offset.
pub const WEAPON_MOD_DESTRUCTIBLE: Row = Row::share(AI_FIXES, AI_FIXES_END, 21);
/// S0055 `patchFixZeroSpreadZeroVATSChance` (`CombatFormulas::GetVATSChanceToHit` `00646f70`
/// (`0x64747B`)): a weapon with no spread keeps a VATS chance.
pub const ZERO_SPREAD_VATS_CHANCE: Row = Row::share(AI_FIXES, AI_FIXES_END, 22);
/// S0189.01 `InventoryCalcWeaponDamageUseAmmoFix` (`CombatFormulas::CalcWeaponDamageForDisplay`
/// `006450f0` (`0x6451B6`, `0x645216`)): the shown damage passes the ammunition it is for.
pub const WEAPON_DAMAGE_DISPLAY_AMMO: Row = Row::share(AI_FIXES, AI_FIXES_END, 23);
/// S0189.02 `InventoryCalcWeaponDamageUseAmmoFix` (`CombatFormulas::CalcWeaponDamage` `00644ce0`
/// (`0x644EF3`, `0x644F00`, `0x644F79`)): the damage takes the projectile of the ammunition passed.
pub const WEAPON_DAMAGE_AMMO_PROJECTILE: Row = Row::share(AI_FIXES, AI_FIXES_END, 24);
/// S0089.01 `RememberAmmoCountOnLoad` (`MiddleHighProcess::LoadGame_ov2` `00927000` (`0x9275EA`)):
/// the player's saved count of loaded rounds is kept for the player's load to use (S0089.07).
pub const AMMO_COUNT_FROM_SAVE: Row = Row::share(AI_FIXES, AI_FIXES_END, 25);
/// S0089.02 `RememberAmmoCountOnLoad` (`PlayerCharacter::LoadGame` `0095a3b0` (`0x95BF7F`)): the
/// load no longer reloads the weapon.
pub const LOAD_GAME_NO_RELOAD: Row = Row::share(AI_FIXES, AI_FIXES_END, 26);
/// S0089.03 `RememberAmmoCountOnLoad` (`0095d850` (`0x95D9D6`)): the player's reload no longer
/// installs a new ammunition entry.
pub const RELOAD_NO_NEW_AMMO_ENTRY: Row = Row::share(AI_FIXES, AI_FIXES_END, 27);
/// S0089.06 `RememberAmmoCountOnLoad` (`00927000` (`0x927591`, `0x927596`)): the load restores the
/// player's equipped weapon even before the player has 3D.
pub const LOAD_GAME_PLAYER_WEAPON_WITHOUT_3D: Row = Row::share(AI_FIXES, AI_FIXES_END, 28);
/// S0089.07 `RememberAmmoCountOnLoad` (`0095a3b0` (`0x95C064`)): after the default ammunition, the
/// load takes the weapon's hotkeyed ammunition and the saved count, limited to what the player has.
pub const LOAD_GAME_AMMO_COUNT: Row = Row::share(AI_FIXES, AI_FIXES_END, 29);
/// S0089.08 `RememberAmmoCountOnLoad` (`0095d850` (`0x95D94F`), `0095d450` (`0x95D539`)): a
/// reload takes the remembered or hotkeyed ammunition only when the player has rounds of it.
pub const HOTKEY_AMMO_NEEDS_ROUNDS: Row = Row::share(AI_FIXES, AI_FIXES_END, 30);
/// S0089.09 `RememberAmmoCountOnLoad` (`AmmoSwapHelper` `009462c0` (`0x94643E`)): a swap that skips
/// the reload still makes the equipped ammunition the current type.
pub const SWAP_SKIP_SETS_AMMO_TYPE: Row = Row::share(AI_FIXES, AI_FIXES_END, 31);
