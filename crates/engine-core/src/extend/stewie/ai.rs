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
/// S0019 `patchAllowAPRegenModifiersWithNoEquippedWeapon` (`Actor::RestoreActionPoints` `0088b660`
/// (`0x88B6A1`)): with no weapon equipped the action point regeneration perks see the fists.
pub const AP_REGEN_NO_WEAPON_FISTS: Row = Row::share(AI_FIXES, AI_FIXES_END, 32);
/// S0094 `patchFixDisintegrationsStat` (`Actor::ApplyCriticalStage` `008a1a70` (`0x8A1B1B`,
/// `0x8A1B51`, `0x8A1B6E`)): the player's goo and ash kills are counted when the critical stage
/// starts, not when it ends.
pub const DISINTEGRATIONS_STAT: Row = Row::share(AI_FIXES, AI_FIXES_END, 33);
/// S0112 `patchFixBloodyMessPreventingExplosionGibbing` (`008b43a0` (`0x8B4580`)): a hit with no
/// body part takes the explosion's parts even when the attacker has Bloody Mess.
pub const BLOODY_MESS_EXPLOSION_GIBBING: Row = Row::share(AI_FIXES, AI_FIXES_END, 34);
/// S0149 `VATSFatalCheckDifficulty` (`HitData::DetermineOutcome` `009b73d0` (`0x9B754A`)): the
/// kill test of a hit multiplies its damage by the difficulty's `fDiffMultHPByPC`.
pub const VATS_FATAL_CHECK_DIFFICULTY: Row = Row::share(AI_FIXES, AI_FIXES_END, 35);
/// S0188 `ShotProjectileResistanceFix` (`Actor::DamageEquipment` `00891360` (`0x891951`)): the
/// explosion of a shot-off explosive names its weapon as its source.
pub const SHOT_PROJECTILE_RESISTANCE: Row = Row::share(AI_FIXES, AI_FIXES_END, 36);
/// S0204 `KillCommandDuplicateRewardXpFix` (`KillActor` `005be2a0` (`0x5BE37C`); `Actor::Kill`
/// `0089d900` (`0x89E5BC`)): the console's `kill` gives the player its experience once.
pub const KILL_COMMAND_DUPLICATE_XP: Row = Row::share(AI_FIXES, AI_FIXES_END, 37);
/// S0241 `ActorGetHeadingTargetFix` (`Actor::GetHeadingTarget` `008a2ed0` (`0x8A2F83`)): the
/// package target within `fAIMaxHeadTrackDistance` is the heading target.
pub const HEADING_TARGET_DISTANCE: Row = Row::share(AI_FIXES, AI_FIXES_END, 38);
/// S0660 `patchFixExplosionPushDirection` (`Actor::HitMe` `0089a760` (`0x89C8E3`)): an
/// explosion's knockback pushes away from the explosion, not from whoever made it.
pub const EXPLOSION_PUSH_DIRECTION: Row = Row::share(AI_FIXES, AI_FIXES_END, 39);
/// S0664 `patchFixKillChallengeSourceWeapon` (`Actor::Kill` `0089d900` (`0x89DB50`)): the kill
/// challenges take the weapon of the killed actor's last hit.
pub const KILL_CHALLENGE_SOURCE_WEAPON: Row = Row::share(AI_FIXES, AI_FIXES_END, 40);
/// S0782 `ShootableNonPlayableExplosives` (`Actor::DamageEquipment` `00891360` (`0x891483`,
/// `0x891759`)): an unplayable explosive an NPC carries can be shot off and set off.
pub const HIDDEN_FRAGS: Row = Row::share(AI_FIXES, AI_FIXES_END, 41);
/// S0022.21 `MenuFixes` (`PlayerCharacter::Update` `0093e860` (`0x94328E`)): activating a workbench
/// or a reloading bench plays no "no activate" sound.
pub const WORKBENCH_NO_ACTIVATE_SOUND: Row = Row::share(AI_FIXES, AI_FIXES_END, 42);
/// S0052 `patchPreventAimingFOVDuringKillcams` (`0095de30` (`0x95E088`)): no aiming field of view
/// during a cinematic kill camera.
pub const NO_AIMING_FOV_IN_KILLCAMS: Row = Row::share(AI_FIXES, AI_FIXES_END, 43);
/// S0153 `patchFixWeaponNifRoundNotCullingInFirstPerson` (`PlayerCharacter::DropObject` `00954610`
/// (`0x954703`)): the dropped round's node is hidden on the first-person model too.
pub const NIF_ROUND_FIRST_PERSON: Row = Row::share(AI_FIXES, AI_FIXES_END, 44);
/// S0164 `patchPreventLevelupInTestAllCells` (`PlayerCharacter::Update` `0093e860` (`0x9439D4`)):
/// no level-up during `TestAllCells`.
pub const NO_LEVELUP_IN_TEST_ALL_CELLS: Row = Row::share(AI_FIXES, AI_FIXES_END, 45);
/// S0209 `PreventAutoVanityModeIfSpecialIdlesArePlaying` (`PlayerCharacter::Update` `0093e860`
/// (`0x943593`)): no automatic vanity camera during a first-person special idle.
pub const NO_AUTO_VANITY_IN_SPECIAL_IDLE: Row = Row::share(AI_FIXES, AI_FIXES_END, 46);
/// S0210 `FixSetIsFiringAutomaticWeaponNotClearing` (`00948310` (`0x949838`, `0x949862`)): the
/// player's automatic-firing flag clears.
pub const AUTOMATIC_FIRING_CLEARS: Row = Row::share(AI_FIXES, AI_FIXES_END, 47);
/// S0212 `PlayerWaitDoubleHealingFix` (`PlayerCharacter::PlayerSleep` `0094df80` (`0x94E09E`)):
/// waiting heals the player once.
pub const WAIT_SINGLE_HEALING: Row = Row::share(AI_FIXES, AI_FIXES_END, 48);
/// S0224 `PreventJumpingWhenFastTraveling` (`PlayerCharacter::Update` `0093e860` (`0x942148`)): no
/// jump while the map has a marker selected.
pub const NO_JUMP_WHEN_FAST_TRAVELING: Row = Row::share(AI_FIXES, AI_FIXES_END, 49);
/// S0111 `patchFixLoopingReloadAmmoSwapInstantRefill` (`PlayerCharacter::Update` `0093e860`
/// (`0x940B10`); `0095d450` (`0x95D714`)): swapping ammunition on a weapon that reloads round by
/// round no longer refills its clip.
pub const AMMO_SWAP_NO_INSTANT_REFILL: Row = Row::share(AI_FIXES, AI_FIXES_END, 50);
/// S0121 `patchPreventReloadingIfClipIsOverfilled` (`0095d850` (`0x95DAE5`, `0x95DC01`),
/// `0095d450` (`0x95D65D`)): a clip holding as many rounds as its size or more is not reloaded.
pub const NO_RELOAD_OVERFILLED_CLIP: Row = Row::share(AI_FIXES, AI_FIXES_END, 51);
/// S0130.02 `ContinuousBeamProjectileCrashFix` (`Actor::Update` `00888b50` (`0x889869`)): an NPC's
/// continuous beam keeps firing while its attack animation runs and fades when it ends. Acts
/// only with `BEAM_SHOOTER_POINTER` (S0130.01) on too.
pub const NPC_CONTINUOUS_BEAM: Row = Row::share(AI_FIXES, AI_FIXES_END, 52);
/// S0130.04 `ContinuousBeamProjectileCrashFix` (`PlayerCharacter::Update` `0093e860` (`0x9440C2`);
/// `Projectile::AddImpact` `009bf5a0` (`0x9BFD6B`)): in god mode the player's continuous beam fires
/// without ammunition.
pub const BEAM_GODMODE_FIRING: Row = Row::share(AI_FIXES, AI_FIXES_END, 53);
/// S0130.05 `ContinuousBeamProjectileCrashFix` (`PlayerCharacter::Update` `0093e860`
/// (`0x944152`)): the player's continuous beam uses its ammunition in the player's update, so
/// firing at the sky uses ammunition too. Acts only with `BEAM_NO_PLAYER_AMMO_IN_IMPACT` on too
/// (Tweaks installs the ammunition parts together).
pub const BEAM_PLAYER_AMMO_IN_UPDATE: Row = Row::share(AI_FIXES, AI_FIXES_END, 54);
/// S0130.06 `ContinuousBeamProjectileCrashFix` (`Projectile::AddImpact` `009bf5a0` (`0x9BFD0E`,
/// `0x9BFCC1`); `PlayerCharacter::Update` `0093e860` (`0x944127`)): the player's continuous beam
/// uses no ammunition where it hits (only with `BEAM_PLAYER_AMMO_IN_UPDATE` on too), and stops when
/// the weapon is put away.
pub const BEAM_NO_PLAYER_AMMO_IN_IMPACT: Row = Row::share(AI_FIXES, AI_FIXES_END, 55);
/// S0198 `VanityCamRotationFix` (`009445b0` (`0x945D53`); `PlayerCharacter::UpdateCamera`
/// `0094ae40` (`0x94B521`)): the vanity camera's turn stays within a circle and its pitch within
/// 89 degrees.
pub const VANITY_CAM_ROTATION: Row = Row::share(AI_FIXES, AI_FIXES_END, 56);
/// S0489 `patchSpinWeaponsAlwaysPlaySound` (`Actor::Update` `00888b50` (`0x8893F8`, `0x889422`,
/// `0x8895A8`)): a weapon with a looping attack sound plays it again after a short stop.
pub const SPIN_WEAPON_SOUND: Row = Row::share(AI_FIXES, AI_FIXES_END, 57);
/// S0926 `CombatSoundCooldownFix` (`009839b0` (`0x983A37`, `0x9841CC`); `Actor::HitMe` `0089a760`
/// (`0x89D42B`, `0x89D4ED`)): the combat line and hit sound cooldowns start only when a line was
/// said.
pub const COMBAT_SOUND_COOLDOWN: Row = Row::share(AI_FIXES, AI_FIXES_END, 58);
/// S0122 `patchSpeakingToSleepingGenericNPCsSkipsDialogueMenu` (`HighProcess::Update` `008eeec0`
/// (`0x8EF5A1`)): an NPC running a dialogue package keeps wanting to speak to the player.
pub const DIALOGUE_PACKAGE_KEEPS_TALK: Row = Row::share(AI_FIXES, AI_FIXES_END, 59);
/// S0132 `patchBumpingCompanionsDoesntAlertEnemies` (`PlayerCharacter::Update` `0093e860`
/// (`0x93EF1D`)): bumping into a companion no longer alerts anyone.
pub const BUMPED_COMPANION_NO_ALERT: Row = Row::share(AI_FIXES, AI_FIXES_END, 60);
/// S0148 `patchResetForceFireWeaponWhenReloadJamming` (`Actor::StartAttack` `00893a40`
/// (`0x8940A5`)): a jam or power attack group clears the process' forced firing.
pub const JAM_CLEARS_FORCE_FIRE: Row = Row::share(AI_FIXES, AI_FIXES_END, 61);
/// S0157 `patchAllowWaitingOnMidairFurniture` (`PlayerCharacter::Update` `0093e860`
/// (`0x9424FD`)): the player may wait while seated on furniture above the ground.
pub const WAIT_ON_MIDAIR_FURNITURE: Row = Row::share(AI_FIXES, AI_FIXES_END, 62);
/// S0165 `patchThresholdThrowingVelocityStrength` (`009bca60` (`0x9BCEA7`)): a thrown weapon's
/// speed uses the player's Strength held within its limits.
pub const THROW_STRENGTH_CLAMPED: Row = Row::share(AI_FIXES, AI_FIXES_END, 63);
/// S0166 `patchThresholdExplosivesWeaponDamage` (`009b0f80` (`0x9B1033`)): an explosion's skill
/// bonus uses the skill held within its limits.
pub const EXPLOSIVES_SKILL_CLAMPED: Row = Row::share(AI_FIXES, AI_FIXES_END, 64);
/// S0179 `GodmodeSkipsPowerArmorTraining` (`Actor::EquipObject` `0088c830` (`0x88C8DA`)): in god
/// mode the player can wear power armour without the training.
pub const GODMODE_POWER_ARMOR: Row = Row::share(AI_FIXES, AI_FIXES_END, 65);
/// S0192 `StoreWeaponModFlagsOnProjectiles` (`009bbef0` (`0x9BC0D2`, `0x9BC1D2`)): a projectile
/// keeps its weapon's mod flags.
pub const PROJECTILE_WEAPON_MOD_FLAGS: Row = Row::share(AI_FIXES, AI_FIXES_END, 66);
/// S0223 `ToggleDetectCommandAffectsMines` (`009c39e0` (`0x9C39FF`)): with detection turned off
/// by the console, mines are not set off.
pub const DETECTION_OFF_MINES: Row = Row::share(AI_FIXES, AI_FIXES_END, 67);
/// S0096 `patchTCLMovement` (`PlayerMover::Update` `009e9e50` (`0x9EA1CE`);
/// `PlayerCharacter::Update` `0093e860` (`0x942793`, `0x9427AA`)): with collision off, keyboard
/// flying moves along the view and the jump control rises.
pub const TCL_MOVEMENT: Row = Row::share(AI_FIXES, AI_FIXES_END, 68);
/// S0097 `patchRunAndGunDontLetAPBeNegative` (`PlayerCharacter`'s vtable slot `0108AD7C`
/// (`0093ab30`); `0095d850` (`0x95DD7C`)): action points spent for an action never go below 0,
/// and a reload charges them only as a reload.
pub const RUN_AND_GUN_AP: Row = Row::share(AI_FIXES, AI_FIXES_END, 69);
/// S0410 `GodmodePreventsLegCrippleSound` (`Actor::Move` `008a62b0` (`0x8A655C`, `0x8A6607`)): a
/// fall plays no landing sound or pain for the player in god mode.
pub const GODMODE_NO_LEG_CRIPPLE_SOUND: Row = Row::share(AI_FIXES, AI_FIXES_END, 70);
/// S0082.01 `NoAmmoGodmode` (`Actor::StartAttack` `00893a40` (`0x893C37`, `0x893B44`);
/// `PlayerCharacter::Update` `0093e860` (`0x941DD4`); `00948310` (`0x9492B5`, `0x94928F`);
/// `VATSMenu` `007ec810` (`0x7EDC3D`, `0x7EDCA3`, `0x7EDCE5`, `0x7EDD04`)): in god mode the player
/// fires without ammunition or action points, in and out of VATS.
pub const NO_AMMO_GODMODE: Row = Row::share(AI_FIXES, AI_FIXES_END, 71);
/// S0329 `patchLeftRightMovementCancelsOut` (`PlayerCharacter::Update` `0093e860` (`0x941BBA`,
/// `0x941C11`, `0x941C67`, `0x941CBD`)): holding two opposite movement keys moves neither way.
pub const OPPOSITE_KEYS_CANCEL: Row = Row::share(AI_FIXES, AI_FIXES_END, 72);
/// S0404 `patchDelayLevelupAfterCombat` (`PlayerCharacter::Update` `0093e860` (`0x943990`)): the
/// level-up menu waits 3 seconds after combat ends.
pub const DELAY_LEVELUP_AFTER_COMBAT: Row = Row::share(AI_FIXES, AI_FIXES_END, 73);
