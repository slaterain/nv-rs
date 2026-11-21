//! lStewieAl's Tweaks built in (`crates/stewie`): which of its rows are on. Each change Tweaks makes
//! to the game's code is a row of its own (docs/FIXES.md, "lStewieAl's Tweaks"), one bit of a table
//! every engine of a game shares ([`Engine::stewie`]). A translation Tweaks patches tests its row's
//! bit at the patched place before anything else, with one load; with the bit clear it runs as the
//! exe's code. `crates/stewie` sets the bits at its install, before the game's threads start: all
//! clear unless the host's row `NV_STEWIE` is on, then each row on unless its own key (or Tweaks'
//! own `[Tweaks]` key for it) turns it off.
//!
//! The rows' numbers are fixed ranges per group ([`CRASH`], ...), so groups built side by side add
//! rows in ranges of their own; `crates/stewie`'s table names each row once and checks that no two
//! rows share a bit.
//!
//! Four agents build the rest side by side, each in files of its own (`measure/stewie`'s split by
//! game system): the crash fixes' rows stay in this file; share 1a's new rows (the engines and the
//! console) go in `world.rs`, share 1b's (forms, cells, scripts, saves and sound) in `forms.rs`,
//! share 2's (AI, actors and the player) in `ai.rs`, share 3's (menus and the HUD) in `menus.rs`
//! (beside this file), each in its own ranges of [`FIXES`] and
//! [`VNV_TWEAKS`], made with [`Row::share`]. Every row is reached here as `stewie::NAME`.

mod ai;
mod forms;
mod menus;
mod world;

pub use ai::*;
pub use forms::*;
pub use menus::*;
pub use world::*;

use crate::{args, Engine};
use std::sync::atomic::{
    AtomicBool, AtomicU32, AtomicU64,
    Ordering::{AcqRel, Acquire, Relaxed},
};
use std::sync::Mutex;

/// The table's words: 2,048 rows (Tweaks has 1,082 rows in `measure/stewie/PLAN.md`).
pub const WORDS: usize = 32;

/// One row's bit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Row(u16);

impl Row {
    /// Row `n` of the group starting at `group`.
    pub const fn at(group: u16, n: u16) -> Row {
        let index = group + n;
        assert!((index as usize) < WORDS * 64, "past the table's rows");
        Row(index)
    }

    /// Row `n` of a share's range `first..end` (`world.rs`, `forms.rs`, `ai.rs`, `menus.rs`): a row past the
    /// range's end does not compile (in a `const`).
    pub const fn share(first: u16, end: u16, n: u16) -> Row {
        assert!(first + n < end, "past the share's rows");
        Row::at(first, n)
    }

    /// Its bit's number in the table.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// The groups' first rows. M1a: the crash, null-check, use-after-free and leak fixes VNV's INI has
/// on (`measure/stewie/STATUS-1118.md`, 80 rows).
pub const CRASH: u16 = 0;
/// The rest of the engine fixes (M1) and the hook points they need.
pub const FIXES: u16 = 128;
/// VNV's tweaks (M2).
pub const VNV_TWEAKS: u16 = 640;
/// The other switches (M3).
pub const OTHER_TWEAKS: u16 = 1024;

const _: () = assert!(CRASH < FIXES && FIXES < VNV_TWEAKS && VNV_TWEAKS < OTHER_TWEAKS);
// The shares' ranges follow one another and fill the fixes' and the tweaks' groups.
const _: () = assert!(
    WORLD_FIXES == FIXES
        && WORLD_FIXES < WORLD_FIXES_END
        && WORLD_FIXES_END == FORMS_FIXES
        && FORMS_FIXES < FORMS_FIXES_END
        && FORMS_FIXES_END == AI_FIXES
        && AI_FIXES < AI_FIXES_END
        && AI_FIXES_END == MENUS_FIXES
        && MENUS_FIXES < MENUS_FIXES_END
        && MENUS_FIXES_END == VNV_TWEAKS
        && WORLD_TWEAKS == VNV_TWEAKS
        && WORLD_TWEAKS < WORLD_TWEAKS_END
        && WORLD_TWEAKS_END == FORMS_TWEAKS
        && FORMS_TWEAKS < FORMS_TWEAKS_END
        && FORMS_TWEAKS_END == AI_TWEAKS
        && AI_TWEAKS < AI_TWEAKS_END
        && AI_TWEAKS_END == MENUS_TWEAKS
        && MENUS_TWEAKS < MENUS_TWEAKS_END
        && MENUS_TWEAKS_END == OTHER_TWEAKS
);

/// S0079, Tweaks' `patchDialogSpeechNullCheck`: `0x935E35`, in the shared actor's clear
/// (`00935d90`), asks `NiObjectNET::GetController` (`00a5c570`) of the speaker's 3D, which is null
/// for a speaker whose 3D is not loaded; Tweaks calls `NiObject_FindType_NullChecked` there, which
/// answers null for no node.
pub const DIALOG_SPEECH_NULL_CHECK: Row = Row::at(CRASH, 0);
/// S0042 `fixExplosionWaterNullCall` (`004e9d40` at `0x4EA743`): the water's reflected explosion
/// with a null `Get3D` slot.
pub const EXPLOSION_WATER_NULL_CALL: Row = Row::at(CRASH, 1);
/// S0064 `patchFixHandleMultiboundsCrashWithNullCell` (`0057ab70` at `0x57ABA4`): a multibound
/// reference with no parent cell.
pub const HANDLE_MULTIBOUNDS_NULL_CELL: Row = Row::at(CRASH, 2);
/// S0068 `fixHandleStealingCrash` (`Actor::StealAlarm` `008bfa40` at `0x8BFBC1`): a theft with no
/// owner.
pub const HANDLE_STEALING_CRASH: Row = Row::at(CRASH, 3);
/// S0078 `patchNullPlayerCellImposterCheck` (`004f2640` at `0x4F269F`): no cell at the player's
/// location.
pub const NULL_PLAYER_CELL_IMPOSTER_CHECK: Row = Row::at(CRASH, 4);
/// S0106 `patchFixNullHitLocationNodeExplosionCrash` (`009b90b0` at `0x9B9387`): a limb cut off
/// with no limb node.
pub const NULL_HIT_LOCATION_NODE_EXPLOSION: Row = Row::at(CRASH, 5);
/// S0124 `patchProcessActivateAddNullCheckForPackage` (`ProcessActivate` `00915f10` at
/// `0x9164D9`): no current package.
pub const PROCESS_ACTIVATE_NULL_PACKAGE: Row = Row::at(CRASH, 6);
/// S0140 `patchFixPatrolMissingNullCheck` (`ProcessPatrolTravelingToNextRef` `0092b030` at
/// `0x92B076`): no patrol reference.
pub const PATROL_MISSING_NULL_CHECK: Row = Row::at(CRASH, 7);
/// S0169 `patchFixRareNullTeleportDoorCrash` (`006f36d0` at `0x6F37AD`): a null door in the
/// teleport door search.
pub const RARE_NULL_TELEPORT_DOOR: Row = Row::at(CRASH, 8);
/// S0174 `patchFixSetActorRefEssentialNullCheck` (`SetActorRefEssential` `005dfd30` at
/// `0x5DFD6A`): the command called with no reference.
pub const SET_ACTOR_REF_ESSENTIAL_NULL_CHECK: Row = Row::at(CRASH, 9);
/// S0881 `patchFixSavingNullCellCrash` (`SaveInitialData` `0084eb80` at `0x84EE1B`): a created
/// reference saved with neither worldspace nor cell.
pub const SAVING_NULL_CELL: Row = Row::at(CRASH, 10);
/// S0022.14 `MenuFixes` (`InventoryMenu::DoClick` `00780140` at `0x780474`): the use click with no
/// item selected.
pub const INVENTORY_USE_NO_ITEM_JUMP: Row = Row::at(CRASH, 11);
/// S0022.15 `MenuFixes`, `QuantityMenuCloseNullInventorySelectionCheck`
/// (`InventoryMenu::DropCurrentItem` `00780c50` at `0x780C76`): a drop with no item selected.
pub const QUANTITY_MENU_CLOSE_NO_ITEM: Row = Row::at(CRASH, 12);
/// S0022.16 `MenuFixes` (`InventoryMenu::SelectCurrentItem` `00780d60` at `0x780D70`): equipping
/// with no item selected.
pub const INVENTORY_SELECT_NO_ITEM: Row = Row::at(CRASH, 13);
/// S0022.17 `MenuFixes` (`ContainerMenu::DoIdle` `0075eac0` at `0x75EB5C`): an entry with no form.
pub const CONTAINER_MOUSEOVER_NO_FORM: Row = Row::at(CRASH, 14);
/// S0022.30 `MenuFixes`, `GetMenuVisiblityStateIfActiveTile` (`InterfaceManager::Idle` `0070c4a0`
/// at `0x70D44A` and `0x70D58D`): the menu under the pointer while no tile is.
pub const CONSOLE_WHILE_MENUS_LOAD: Row = Row::at(CRASH, 15);
/// S0022.36 `MenuFixes` (`ListBox::GetAt` `007a6fa0` at `0x7A7025`): a list node with no item.
pub const LIST_BOX_GET_AT_NULL_ITEM: Row = Row::at(CRASH, 16);
/// S0022.56 `MenuFixes`, `BeginsWithNonNull` (`00719580` at `0x7195D1` and `0x7195EB`): a tile with
/// no name.
pub const TILE_LABEL_NULL_NAME: Row = Row::at(CRASH, 17);
/// S0057 `patchFixLevelGreaterThan99Crash` (`HUDMainMenu::UpdateXPMeter` `0077c4e0` at `0x77D258`
/// and `0x77D28D`): a level of 100 or more printed into 3 bytes.
pub const LEVEL_GREATER_THAN_99: Row = Row::at(CRASH, 18);
/// S0139 `patchFixCreditsCrashIfNoLoadedSave` (`CreditsMenu::Close` `0075f750` at `0x75F78E`): no
/// player name.
pub const CREDITS_NO_LOADED_SAVE: Row = Row::at(CRASH, 19);
/// S0182 `HolotapeEmptySubtitleCrashFix` (`MapMenu` `0079a660` at `0x79A868` and `0x79AAEB`): a
/// holotape's subtitle removed from an empty list.
pub const HOLOTAPE_EMPTY_SUBTITLE: Row = Row::at(CRASH, 20);
/// S0043 `patchFixNullWeaponOrWeaponInfoReloadLoopCrash`: `Actor::PickAnimations` (`00895110` at
/// `0x8956EB`, the reload loop's step) with no weapon or no weapon entry, and the queued idle flags'
/// reload (`008ba600` at `0x8BAC12`, not the player) with no weapon.
pub const RELOAD_LOOP_NULL_WEAPON: Row = Row::at(CRASH, 21);
/// S0150 `patchFixMissingAnimNullCheck` (`Actor::PickAnimations` at `0x89520A` and `0x895350`): no
/// sequence in the attack slot of the actor's or the player's first-person animation.
pub const MISSING_ANIM_NULL_CHECK: Row = Row::at(CRASH, 22);
/// S0172 `patchFixAnimDataUpdateReloadLoopNullCheck` (`AnimData::Update` `00491180` at `0x4913C4`):
/// the queued reload with no weapon.
pub const ANIM_DATA_RELOAD_LOOP_NULL: Row = Row::at(CRASH, 23);
/// S0173 `patchAnimDataUpdateCrashCheck` (`AnimData::Update` at `0x492EBD`): a movement sequence
/// with no controlled blocks.
pub const ANIM_DATA_MOVEMENT_SEQUENCE: Row = Row::at(CRASH, 24);
/// S0181 `NullAnimRemoveRecurseFix` (`00496a50` at `0x496B6C`, `004f0640` at `0x4F064E`, `004f06d0`
/// at `0x4F06E4`): an idle's add-on object that is gone, removed from the animation's controllers.
pub const NULL_ANIM_REMOVE_RECURSE: Row = Row::at(CRASH, 25);
/// S0186 `BipedAnimNullItemError` (`QueuedHelmet::Attach` `0043fb80` at `0x43FBE4`): a helmet's
/// model for a biped slot that holds no item.
pub const BIPED_ANIM_NULL_ITEM: Row = Row::at(CRASH, 26);
/// S0202 `patchEquipAmmoOnNullBaseProcessCrashFix` (`Actor::EquipObject` `0088c830` at
/// `0x88CDE7`): ammunition equipped on an actor with no process.
pub const EQUIP_AMMO_NULL_PROCESS: Row = Row::at(CRASH, 27);
/// S0027 `patchFixPipboyEquipCrash` (`Actor::EquipObject` at `0x88CDA3`, `FOPipboyManager::OnMenuOpen`
/// `007f8010` at `0x7F8059`): the Pip-Boy reset when a Pip-Boy item is equipped.
pub const PIPBOY_EQUIP_CRASH: Row = Row::at(CRASH, 28);
/// The Pip-Boy manager's "reset me" byte (`FOPipboyManager` +0x16C), which row S0027 sets in place of
/// the reset: the script command (`005db4c0`) sets it too, the main loop's AI tasks (`0086f840`) read
/// it and the reset itself (`007fa990`) clears it. One definition for the AI's equip and the
/// interface's menu open.
pub const PIPBOY_RESET_PENDING: u32 = 0x16c;
/// S0126 `patchFixNullProjectileInCombatStateUpdateInventory` (`009993c0` at `0x99A1C8` and
/// `0x99A1DC`): a combat weapon whose projectile is null.
pub const COMBAT_STATE_NULL_PROJECTILE: Row = Row::at(CRASH, 29);
/// S0127 `patchFixRecipeMenuMissingAmmoInfoNullCheck` (`007284f0` at `0x72872D`): crafting
/// ammunition while the crafter has no ammunition entry.
pub const RECIPE_MENU_NULL_AMMO_INFO: Row = Row::at(CRASH, 30);
/// S0089.04 `RememberAmmoCountOnLoad` (`0095d850` at `0x95DBB4`): the player's reload with no
/// equipped ammunition entry.
pub const RELOAD_NULL_AMMO_ENTRY: Row = Row::at(CRASH, 31);
/// S0089.05 `RememberAmmoCountOnLoad` (`00954610` at `0x9546EE`): the player dropping the weapon's
/// ammunition with no equipped ammunition entry.
pub const DROP_AMMO_NULL_ENTRY: Row = Row::at(CRASH, 32);
/// S0018 `fixUGridsCrashAndAllow3AsMinimum` (`TES::LoadGame` `004595e0` (`0x4596D3`); `004bc4c0` (`0x4BC501`, `0x4BC505`)): loading a save made with more grids than the game now loads no longer crashes; uGridsToLoad 3 is kept.
pub const UGRIDS_LOAD_NULL_CELL: Row = Row::at(CRASH, 33);
/// S0045 `patchFixGetItemValueCrashForModdedWeapons` (`ItemChange::GetItemValue` `004bd400` (`0x4BD48D`, `0x4BD4B5`, `0x4BD4DD`)): a weapon whose mod flags name a mod the weapon no longer has no longer crashes when its value is asked.
pub const ITEM_VALUE_NULL_WEAPON_MOD: Row = Row::at(CRASH, 34);
/// S0049 `patchFixUnresolvedRaceCrash` (`TESRaceForm::InitItem` `0048b920` (`0x48B9D9`, `0x48B9E8`, `0x48B9F1`); `00609220` (`0x60936F`)): an NPC whose race is missing (a removed plugin's) gets CaucasianVanilla instead of crashing.
pub const UNRESOLVED_RACE: Row = Row::at(CRASH, 35);
/// S0067 `fixGetWeaponDamageCrashOnThrowingWeapons` (`CalcWeaponDamageForDisplay` `006450f0` (`0x64529D`)): a throwing weapon's damage shown in a menu no longer crashes.
pub const THROWING_WEAPON_AMMO_EFFECTS: Row = Row::at(CRASH, 36);
/// S0077 `patchBodyPartDataNullChecks` (`BGSBodyPartData` `005e55e0` (`0x5E5734`); `ShowChildLimbs` `005e5750` (`0x5E58C6`)): hiding or showing limbs whose node chain runs out before the root no longer crashes.
pub const BODY_PART_NULL_PARENT: Row = Row::at(CRASH, 37);
/// S0093.07 `PerkMenuDescriptionTypoFix` (`BGSPerk::BuildDescriptionString` `005ebac0` (`0x5EC026`)): a perk description too long for the menu's buffer is cut to fit instead of crashing.
pub const PERK_DESCRIPTION_TOO_LONG: Row = Row::at(CRASH, 38);
/// S0199.05 `MiscCrashFixes` (`TESNPC::Activate` `00607990` (`0x607A7D`)): an NPC activated by something that is not an actor, or by an actor with no AI process, no longer crashes.
pub const NPC_ACTIVATE_NULL_PROCESS: Row = Row::at(CRASH, 39);
/// S0199.06 `MiscCrashFixes` (`InventoryChanges` `004c0cf0` (`0x4C0DD4`)): taking off a worn item that may not be removed, with no actor given, no longer crashes.
pub const REMOVE_WORN_NULL_ACTOR: Row = Row::at(CRASH, 40);
/// S0199.09 `MiscCrashFixes` (`004d66d0` (`0x4D6AA5`); `NiAVObject::GetViewerStrings` `00a5ad00` (`0xA5B0C8`)): an empty entry of an object's property list is skipped in its viewer strings and in ShowSceneGraph's controller test; ShowSceneGraph's tree still stops at its Properties list, as with Tweaks.
pub const SHOW_SCENE_GRAPH_NULL: Row = Row::at(CRASH, 41);
/// S0107.03 `MemoryLeakFixes` (`Actor::InitiateSearchForAttacker` `008ad970` (`0x8ADAEC`)): an actor searching for its attacker no longer loses 16 bytes each time.
pub const PACKAGE_TARGET_LEAK: Row = Row::at(CRASH, 42);
/// S0130.01 `ContinuousBeamProjectileCrashFix` (`BeamProjectile` destructor body `00978f70` (`0x978FB5`)): an actor no longer keeps pointing at its continuous beam after the beam is gone.
pub const BEAM_SHOOTER_POINTER: Row = Row::at(CRASH, 43);
/// S0130.03 `ContinuousBeamProjectileCrashFix` (`ContinuousBeamProjectile` `009ab390` (`0x9AB55F`)): loading a save while an NPC fires a continuous beam no longer crashes.
pub const BEAM_NPC_NULL_NODE: Row = Row::at(CRASH, 44);
/// S0183 `FollowerTopicUseAfterFreeFix` (`~Actor` `0087dcb0` (`0x87DDD0`)): an actor that goes away no longer leaves a follower bark naming it.
pub const FOLLOWER_BARK_USE_AFTER_FREE: Row = Row::at(CRASH, 45);
/// S0199.03 `MiscCrashFixes` (`Actor::DecreaseAmmo` `008a89a0` (`0x8A8B0B`)): an actor whose ammunition entry is taken while it fires (a script on JIP's DecreaseAmmo event) no longer crashes.
pub const AMMO_ENTRY_GONE: Row = Row::at(CRASH, 46);
/// S0199.10 `MiscCrashFixes` (`VirtualActorPathHandler::UseDoorAtNode` `009ead90` (`0x9EAF04`)): an NPC away from a path node with no door no longer crashes.
pub const USE_DOOR_NULL_DOOR: Row = Row::at(CRASH, 47);
/// S0199.11 `MiscCrashFixes` (`PlayerCharacter::Update` `0093e860` (`0x943DE1`)): the player's regenerating ammunition with no ammunition type equipped no longer crashes.
pub const REGEN_AMMO_NO_TYPE: Row = Row::at(CRASH, 48);
/// S0036 `patchRemoveInvalidCasinoDataChipsInSlotMachineMenu` (`SlotMachineMenu::Create` `007c0a40` (`0x7C0AAE`), `RouletteMenu::Create` `007bbe20` (`0x7BBE72`), `BlackJackMenu::Create` `00733630` (`0x73369E`)): a casino record whose plugin is gone is removed when a gambling menu opens; the slot machines no longer refuse to open on it.
pub const CASINO_INVALID_CHIPS: Row = Row::at(CRASH, 49);
/// S0196.01 `RouletteMenuFixes` (`RouletteMenu` destructor `007b9020` (`0x7B93C2`)): closing the roulette table no longer empties its freed bet list a second time.
pub const ROULETTE_BET_LIST_DOUBLE_FREE: Row = Row::at(CRASH, 50);
/// S0196.02 `RouletteMenuFixes` (`RouletteMenu` destructor `007b9020` (`0x7B92D7`)): the bets still placed when the roulette table closes are freed.
pub const ROULETTE_BETS_FREED_ON_CLOSE: Row = Row::at(CRASH, 51);
/// S0196.03 `RouletteMenuFixes` (`RouletteMenu::DoIdle` `007bd2a0` (`0x7BE034`)): each roulette bet is freed once it is settled.
pub const ROULETTE_BET_FREED_ON_USE: Row = Row::at(CRASH, 52);
/// S0199.08 `MiscCrashFixes` (`CompanionWheelMenu::Do_UseStim` `00756490` (`0x756771`)): a stimpak given from the companion wheel is no longer also removed from the companion.
pub const COMPANION_STIMPAK_REMOVED_TWICE: Row = Row::at(CRASH, 53);
/// S0199.12 `MiscCrashFixes` (`ListBox::BuildFromContainer` `00719ef0` (`0x71A019`, `0x71A02D`)): an item with no name part in an inventory list no longer crashes.
pub const LIST_BOX_NULL_NAME: Row = Row::at(CRASH, 54);
/// S0037 `fixNullParticleSystemsCrash` (`BSMasterParticleSystem::ChildHasActiveParticles` `00c5eb90` (`0xC5EBD9`)): a particle system child with no particle data no longer crashes.
pub const PARTICLE_SYSTEM_NULL_DATA: Row = Row::at(CRASH, 55);
/// S0048 `patchFixNullFaceGenCrash` (`BSFaceGenModel::LoadModelMesh` `00659ce0` (`0x65A125`)): a face model with no geometry for its tri file no longer crashes.
pub const FACE_GEN_NULL_GEOMETRY: Row = Row::at(CRASH, 56);
/// S0108 `patchFixNullNifOwnerCrash` (`NiControllerSequence` activation `00a34f20` (`0xA34FCB`, `0xA35003`)): an animation sequence with no owner no longer crashes when it starts.
pub const SEQUENCE_NULL_OWNER: Row = Row::at(CRASH, 57);
/// S0199.04 `MiscCrashFixes` (`hkpEntity::activate` `00c9c1d0` (`0xC9C1DD`)): a physics body with no simulation island no longer crashes when it is woken.
pub const HKP_ENTITY_NULL_ISLAND: Row = Row::at(CRASH, 58);
/// S0199.07 `MiscCrashFixes` (`TESObjectCELL::AddReference` `00548230` (`0x54867C`)): a reference whose cell is gone while it is added no longer crashes.
pub const ADD_REFERENCE_NULL_CELL: Row = Row::at(CRASH, 59);
/// S0107.01 `MemoryLeakFixes` (`Controls::Rumble` `00a255b0` (`0xA25603`)): the controller's vibration no longer loses 36 bytes each time.
pub const RUMBLE_STATES_LEAK: Row = Row::at(CRASH, 60);
/// S0107.04 `MemoryLeakFixes` (`PlayerCharacter::FocusOnActor` `00953060` (`0x953253`, `0x9532B0`)): turning to an actor's head in dialogue no longer loses 4 bytes each time.
pub const FOCUS_SPEAKER_NAME_LEAK: Row = Row::at(CRASH, 61);
/// S0023 `patchPrintBSXCrash` (`00c80090` (`0xC800D5`)): a model node with no first child no longer crashes the entity test; Tweaks' console line names it.
pub const INVALID_FADE_NODE: Row = Row::at(CRASH, 62);
/// S0031 `patchAddNifShapeNullChecks` (`00d21960` (`0xD21960`)): a rigid body whose shape is missing (a model's blocks out of order) no longer crashes; Tweaks' console line names the model.
pub const NULL_HKP_SHAPE: Row = Row::at(CRASH, 63);
/// S0199.02 `MiscCrashFixes` (`008b52a0` (`0x8B558C`, `0x8B5B13`, `0x8B606B`)): cutting off a limb whose node the model lacks no longer crashes.
pub const DISMEMBER_NULL_LIMB: Row = Row::at(CRASH, 64);
/// S0155 `patchNullAudioCrash` (`00aec530` (`0xAECF39`, `0xAED074`)): a sound with an effect whose buffer was never made no longer crashes when it is prepared.
pub const NULL_AUDIO: Row = Row::at(CRASH, 65);
/// S0225 `SamplerStateLeakFix` (`NiD3DTextureStage::ConfigureStage` `00e7eb00`): a texture stage whose texture is unchanged but whose wrap or clamp differs from the device's is set up again instead of keeping the last material's addressing.
pub const SAMPLER_STATE_LEAK: Row = Row::at(CRASH, 66);
/// S0136 `patchTryCatchingQueuedTextureLoadCrash` (`QueuedTexture::Run` `0043c150` (`0x43C20A`), `004436c0` (`0x44373F`), `00443af0` (`0x443B57`)): a queued texture whose data cannot be read (already freed) counts as not loaded instead of crashing; Tweaks' console line says so once.
pub const QUEUED_TEXTURE_LOAD: Row = Row::at(CRASH, 67);
/// S0022.43 `MenuFixes` (`Font::MakeIconsTriShape` `00a14da0` (`0xA151D1`, `0xA15164`)): a font button with no icon texture path no longer crashes: the icon shape is hidden and the path is interface\shared\empty.dds.
pub const FONT_ICON_NO_TEXTURE: Row = Row::at(CRASH, 68);
/// S0022.48 `MenuFixes` (`00730950` (`0x730A00`)): the repair menu's target item is not freed while the menu is open (Tweaks' trade: that entry leaks instead of a use-after-free).
pub const REPAIR_TARGET_KEPT: Row = Row::at(CRASH, 69);
/// S0022.49 `MenuFixes` (`00716010` (`0x7160A2`)): a reference being freed is taken off the console's pickable references too, not only off the HUD's crosshair.
pub const SELECTABLE_REF_FREED: Row = Row::at(CRASH, 70);
/// S0025 `patchFixCrashWhenControllerReconnectsInHackingMenu` (`HackingMenu::MakePasswordFile` `00768aa0` (`0x7693F5`)): the hacking menu always sets its character-selection tile, so a controller connected later can pick characters.
pub const HACKING_SELECTION_TILE: Row = Row::at(CRASH, 71);
/// S1016 (Engine Optimizations, `Inlines::SaveLoad`: `Write32(0x845558, 0x2735)`): the changes map's
/// buckets 10,037, not the exe's 5,039 (`00845550`). It changes the order a save walks and writes the
/// changed forms, so it is a fixes row.
pub const CHANGES_MAP_BUCKETS: Row = Row::at(FIXES, 0);

/// Tweaks' messages it prints once a run ([`Engine::stewie_once`]).
pub mod once {
    /// S0173's "Failed to resolve movement sequence" line.
    pub const MOVEMENT_SEQUENCE: u32 = 0;
    /// S0136's "Prevented crash when texture failed to load" line.
    pub const TEXTURE_LOAD: u32 = 1;
}

/// Tweaks' DLL statics the game's code is handed a pointer to: guest blocks kept for the whole run
/// ([`Engine::stewie_kept`]), one per static, made the first time a row asks.
pub mod kept {
    /// S0107.01's `static VibrationStates states` (0x24 bytes): the rumble effect the controller's
    /// queue is handed each time, in place of a new block every vibration.
    pub const RUMBLE_STATES: usize = 0;
    /// S0107.04's `static NiFixedString strSpeaker` (4 bytes: the handle of "Bip01 Speaker").
    pub const SPEAKER_NAME: usize = 1;
    /// S0089.01's `int g_savedAmmoCount` (4 bytes): the player's saved count of loaded rounds plus
    /// one (0: none, Tweaks' -1), read by S0089.07.
    pub const SAVED_AMMO_COUNT: usize = 2;
    /// How many.
    pub const COUNT: usize = 3;
}

/// The last thing each of Tweaks' warnings printed (its functions' statics), its print queue and
/// the missing-data guard's switch, shared by every thread's engine.
#[derive(Debug)]
pub(crate) struct Prints {
    /// `[Tweaks] bInvalidBSFadeNodePrints` (Tweaks' code default 1): S0023 prints only with it.
    fade_prints: AtomicBool,
    /// S0023's `BSXCrashPrint` statics: `lastRefID` and `lastMeshPath` (a guest address), 0 at
    /// start.
    fade_ref: AtomicU32,
    fade_mesh: AtomicU32,
    /// S0031's `ShowNullHkpShapeWarning` `static char lastModelName[100]`: the last name printed,
    /// as its `snprintf` copied it (at most 99 bytes).
    shape_model: Mutex<Vec<u8>>,
    /// Tweaks' print queue (`queuedConsoleMessages`, in its copy of xNVSE's `GameAPI.cpp`): the lines
    /// printed off the main thread or before the console exists, printed by [`print_queued`].
    queued: Mutex<Vec<Vec<u8>>>,
    /// S0050's `NullLODDataPrintID` `static TESObjectREFR* lastRef`: the reference it saw last, 0
    /// at start.
    lod_ref: AtomicU32,
    /// Tweaks' slips not copied ([`Engine::stewie_slip_guard`]): on with the fixes preset's
    /// missing-data guard row (`NV_NVSE_NULL_GUARD`, the host's install), off in vanilla.
    slip_guard: AtomicBool,
}

impl Default for Prints {
    fn default() -> Self {
        Prints {
            fade_prints: AtomicBool::new(true),
            fade_ref: AtomicU32::new(0),
            fade_mesh: AtomicU32::new(0),
            shape_model: Mutex::new(Vec::new()),
            queued: Mutex::new(Vec::new()),
            lod_ref: AtomicU32::new(0),
            slip_guard: AtomicBool::new(false),
        }
    }
}

/// `__RTDynamicCast` and the type descriptors xNVSE's `TESForm::GetModelPath` casts between.
const DYNAMIC_CAST: u32 = 0x00ec_43fb;
const RTTI_TES_FORM: u32 = 0x0118_3028;
const RTTI_TES_MODEL: u32 = 0x0118_31e8;
/// The model file the loader is reading (Tweaks' `*(char**)0x11F94C0`).
const LOADING_MODEL_NAME: u32 = 0x011f_94c0;

/// xNVSE's `TESForm::GetModelPath`: the form's `TESModel` (a dynamic cast) asked its path (virtual
/// slot 5, `+0x14`: after `BaseFormComponent`'s four and `Destroy`); 0 for none or an empty text.
fn model_path(e: &mut Engine, form: u32) -> u32 {
    let model = e
        .call(
            DYNAMIC_CAST,
            &args![form, 0u32, RTTI_TES_FORM, RTTI_TES_MODEL, 0u32],
        )
        .u32();
    if model == 0 {
        return 0;
    }
    let path = e.vcall(model, 0x14, &args![]).u32();
    if path != 0 && e.mem.u8(path) != 0 {
        path
    } else {
        0
    }
}

/// S0023, Tweaks' `BSXCrashPrint(node)`, for a model node (`BSFadeNode`) the entity test found no
/// first child of: the node's linked reference (`+0xCC`) and its form id (`+0xC`, `0xFFFFFFFF`
/// for none); for an id other than the last one seen, while `[Tweaks] bInvalidBSFadeNodePrints`
/// is on, "[Tweaks] Invalid BSFadeNode Detected - Linked Obj ID: %08X (%s)" with the reference's
/// base form's (`+0x20`) model path ("<unknown mesh>" without one), unless that path is the one
/// printed last (the same text address). The id is remembered either way. As with Tweaks only an
/// immediate repeat is skipped: two broken references checked in turn print each time.
pub fn invalid_fade_node_print(e: &mut Engine, node: u32) {
    let bits = std::sync::Arc::clone(&e.extend.stewie);
    let prints = &bits.3;
    let linked = e.mem.u32(node + 0xcc);
    let id = if linked != 0 {
        e.mem.u32(linked + 0xc)
    } else {
        0xffff_ffff
    };
    if id == prints.fade_ref.load(Relaxed) {
        return;
    }
    if prints.fade_prints.load(Relaxed) {
        let mut path = 0;
        if linked != 0 {
            let base = e.mem.u32(linked + 0x20);
            if base != 0 {
                path = model_path(e, base);
            }
        }
        if path == 0 || path != prints.fade_mesh.load(Relaxed) {
            let format = b"[Tweaks] Invalid BSFadeNode Detected - Linked Obj ID: %08X (%s)";
            if path != 0 {
                console_print(e, format, &[id, path]);
            } else {
                with_text(e, b"<unknown mesh>", |e, unknown| {
                    console_print(e, format, &[id, unknown]);
                });
            }
            prints.fade_mesh.store(path, Relaxed);
        }
    }
    prints.fade_ref.store(id, Relaxed);
}

/// S0031, Tweaks' `ShowNullHkpShapeWarning` (its answer is 0): the model the loader is reading
/// ("unknown" for none or an empty name), unless it is the name printed last, goes to the console
/// as "hkpRigidBody error: %s has a missing shape (unordered blocks?)" (formatted into Tweaks'
/// `MAX_PATH` buffer: at most 259 bytes), with the console's loading-screen test passed over for
/// the line, as Tweaks' byte at `0x71D0DC` does; the name is remembered as its 100-byte copy
/// keeps it (99 bytes).
pub fn null_hkp_shape_warning(e: &mut Engine) {
    let at = e.mem.u32(LOADING_MODEL_NAME);
    let mut name = Vec::new();
    if at != 0 {
        let mut p = at;
        while e.mem.u8(p) != 0 {
            name.push(e.mem.u8(p));
            p += 1;
        }
    }
    if name.is_empty() {
        name = b"unknown".to_vec();
    }
    {
        let last = e.extend.stewie.3.shape_model.lock().unwrap();
        if *last == name {
            return;
        }
    }
    let mut line = b"hkpRigidBody error: ".to_vec();
    line.extend_from_slice(&name);
    line.extend_from_slice(b" has a missing shape (unordered blocks?)");
    line.truncate(259);
    with_text(e, &line, |e, text| {
        e.set_stewie_print_ungated(true);
        console_print(e, b"%s", &[text]);
        e.set_stewie_print_ungated(false);
    });
    name.truncate(99);
    *e.extend.stewie.3.shape_model.lock().unwrap() = name;
}

/// `ConsoleManager_GetSingleton` (`0071b160`, with true) and `ConsoleManager::Print` (`0071d0a0`,
/// `thiscall`: a format and a `va_list`): what Tweaks' `Console_Print` (xNVSE's) calls.
const CONSOLE_INSTANCE: u32 = 0x0071_b160;
const CONSOLE_PRINT: u32 = 0x0071_d0a0;

/// Tweaks' `Console_Print(format, ...)`: the console's print with `format` and a `va_list` of
/// `words` (each a number or a guest string's address the format reads), both on the guest stack.
/// Nothing when there is no console.
pub fn console_print(e: &mut Engine, format: &[u8], words: &[u32]) {
    let console = e.call(CONSOLE_INSTANCE, &args![1u32]).u32();
    if console == 0 {
        return;
    }
    let text = (format.len() as u32 + 1).next_multiple_of(4);
    e.with_stack(text + 4 * words.len() as u32, |e, block| {
        let at = block.addr();
        for (i, b) in format.iter().enumerate() {
            e.mem.set_u8(at + i as u32, *b);
        }
        e.mem.set_u8(at + format.len() as u32, 0);
        let list = at + text;
        for (i, word) in words.iter().enumerate() {
            e.mem.set_u32(list + 4 * i as u32, *word);
        }
        e.call(CONSOLE_PRINT, &args![console, at, list]);
    });
}

/// The console singleton's word (`*(ConsoleManager**)0x11D8CE8`): 0 until the console exists.
const CONSOLE_SINGLETON: u32 = 0x011d_8ce8;

/// Tweaks' `ConsoleQueueOrPrint(text)` (in its copy of xNVSE's `GameAPI.cpp`): on the main thread
/// with the console made, `Console_Print("%s", text)`; otherwise the text waits in Tweaks' queue
/// for [`print_queued`].
pub fn console_queue_or_print(e: &mut Engine, text: &[u8]) {
    if e.mem.u32(CONSOLE_SINGLETON) != 0 && crate::thread_host::on_main(e) {
        with_text(e, text, |e, at| console_print(e, b"%s", &[at]));
    } else {
        e.extend.stewie.3.queued.lock().unwrap().push(text.to_vec());
    }
}

/// Tweaks' `handleQueuedNonMainThreadPrints` (its call at `0x870662`, in the main loop's idle step
/// `00870610`): every queued line printed with "%s", the queue emptied; nothing when it is empty.
pub fn print_queued(e: &mut Engine) {
    let lines = std::mem::take(&mut *e.extend.stewie.3.queued.lock().unwrap());
    for line in lines {
        with_text(e, &line, |e, at| console_print(e, b"%s", &[at]));
    }
}

/// S0136, Tweaks' `NiTextureQueuedLoadHook(texture)` in place of `0043c430`: the texture's data
/// (`+0x24`), 0 without one, else the byte at the data's `+0x5B` (`unk58[3]`: the same reads as the
/// exe's function), inside Tweaks' `__try`: a read of memory that is not there (a texture already
/// freed) answers 0 and prints "[Tweaks] Prevented crash when texture failed to load..." once a
/// game, through [`console_queue_or_print`] (texture loads also run on the loader's threads).
pub fn queued_texture_state(e: &mut Engine, texture: u32) -> u32 {
    let data_at = texture.wrapping_add(0x24);
    if e.mem.is_mapped(data_at) {
        let data = e.mem.u32(data_at);
        if data == 0 {
            return 0;
        }
        let byte_at = data.wrapping_add(0x5b);
        if e.mem.is_mapped(byte_at) {
            return u32::from(e.mem.u8(byte_at));
        }
    }
    if e.stewie_once(once::TEXTURE_LOAD) {
        console_queue_or_print(
            e,
            b"[Tweaks] Prevented crash when texture failed to load...",
        );
    }
    0
}

/// The two sampler states S0225 compares (`0126f918`: `D3DSAMP_ADDRESSU`, `D3DSAMP_ADDRESSV`), the
/// map from a sampler state to its slot (`0126f92c`, a halfword each; 5 or more: not one the stage
/// group keeps), the render state cache whose sampler getter Tweaks calls (`0126f99c`) and that
/// getter's slot in its class's vtable (`010f095c`, `00e910f0`).
const VALID_SAMPLER_STATES: u32 = 0x0126_f918;
const SAMPLER_STATE_SLOTS: u32 = 0x0126_f92c;
const SAMPLER_RENDER_STATE: u32 = 0x0126_f99c;
const RENDER_STATE_GET_SAMPLER_STATE: u32 = 0x010f_095c;

/// S0225, Tweaks' `CheckSamplerStateChanged(stage)`: for `ADDRESSU` and `ADDRESSV`, the value the
/// stage's group (`NiD3DTextureStage` +0x10, `NiD3DTextureStageGroup`) holds for its stage (the
/// current sampler array at +0x90 when +0xA4's valid byte is set, else the saved one at +0x6C when
/// +0x80's is; neither: that state is passed over) against the render state cache's current value
/// for the stage (+4) and state: true at the first that differs.
pub fn sampler_state_changed(e: &mut Engine, stage: u32) -> bool {
    let group = e.mem.u32(stage + 0x10);
    let stage_number = e.mem.u32(stage + 4);
    for i in 0..2 {
        let state = e.mem.u32(VALID_SAMPLER_STATES + 4 * i);
        let slot = u32::from(e.mem.u16(SAMPLER_STATE_SLOTS + 2 * state));
        if slot > 4 {
            continue;
        }
        let wanted = if e.mem.u8(group + 0xa4 + slot) != 0 {
            e.mem.u32(group + 0x90 + 4 * slot)
        } else if e.mem.u8(group + 0x80 + slot) != 0 {
            e.mem.u32(group + 0x6c + 4 * slot)
        } else {
            continue;
        };
        let getter = e.mem.u32(RENDER_STATE_GET_SAMPLER_STATE);
        let cache = e.mem.u32(SAMPLER_RENDER_STATE);
        let current = e.call(getter, &args![cache, stage_number, state]).u32();
        if current != wanted {
            return true;
        }
    }
    false
}

/// `text` with a 0 after it on the guest stack for `f` (its address).
fn with_text(e: &mut Engine, text: &[u8], f: impl FnOnce(&mut Engine, u32)) {
    e.with_stack((text.len() as u32 + 1).next_multiple_of(4), |e, block| {
        let at = block.addr();
        for (i, b) in text.iter().enumerate() {
            e.mem.set_u8(at + i as u32, *b);
        }
        e.mem.set_u8(at + text.len() as u32, 0);
        f(e, at);
    });
}

/// The table: one bit a row, the once-a-run messages' flags, the kept blocks' addresses (0 until
/// made) and the Havok warnings' last lines.
#[derive(Debug)]
pub(crate) struct Bits(
    [AtomicU64; WORDS],
    AtomicU64,
    [AtomicU32; kept::COUNT],
    Prints,
);

impl Default for Bits {
    fn default() -> Self {
        Bits(
            std::array::from_fn(|_| AtomicU64::new(0)),
            AtomicU64::new(0),
            std::array::from_fn(|_| AtomicU32::new(0)),
            Prints::default(),
        )
    }
}

impl Engine {
    /// Whether Tweaks' row `row` is on. One load; a translation reads it at the patched place
    /// before anything else.
    #[inline]
    pub fn stewie(&self, row: Row) -> bool {
        let word = self.extend.stewie.0[row.index() / 64].load(Relaxed);
        word & (1 << (row.index() % 64)) != 0
    }

    /// Turns `row` on or off, in one atomic step (`crates/stewie`'s install, before the threads
    /// start).
    pub fn set_stewie(&self, row: Row, on: bool) {
        let word = &self.extend.stewie.0[row.index() / 64];
        let bit = 1u64 << (row.index() % 64);
        if on {
            word.fetch_or(bit, Relaxed);
        } else {
            word.fetch_and(!bit, Relaxed);
        }
    }

    /// For a message Tweaks prints once a run (its `static bool hasShownError`): true the first
    /// time `message` ([`once`]) is asked in this game (every thread's engine shares the flags),
    /// false after.
    pub fn stewie_once(&self, message: u32) -> bool {
        let bit = 1u64 << message;
        self.extend.stewie.1.fetch_or(bit, Relaxed) & bit == 0
    }

    /// The guest block of Tweaks' static `which` ([`kept`]): `size` bytes, zeroed, made the first
    /// time it is asked and kept for the run; every thread's engine gets the same block.
    pub fn stewie_kept(&mut self, which: usize, size: u32) -> u32 {
        let at = self.extend.stewie.2[which].load(Acquire);
        if at != 0 {
            return at;
        }
        let block = self.mem.alloc(size);
        match self.extend.stewie.2[which].compare_exchange(0, block, AcqRel, Acquire) {
            Ok(_) => block,
            Err(other) => {
                self.mem.free(block);
                other
            }
        }
    }

    /// `[Tweaks] bInvalidBSFadeNodePrints` (`crates/stewie`'s install): whether S0023 prints.
    pub fn set_stewie_fade_node_prints(&self, on: bool) {
        self.extend.stewie.3.fade_prints.store(on, Relaxed);
    }

    /// Whether S0023 prints ([`Engine::set_stewie_fade_node_prints`]).
    pub fn stewie_fade_node_prints(&self) -> bool {
        self.extend.stewie.3.fade_prints.load(Relaxed)
    }

    /// The missing-data guard row's part in Tweaks' rows (the host turns it on with
    /// `NV_NVSE_NULL_GUARD`, every thread's engine shares it): where a row copies a Tweaks slip
    /// that reads data that is not there (S0237's unchecked child index), the guard passes over the
    /// missing data. Off, every row is exactly Tweaks.
    pub fn set_stewie_slip_guard(&self, on: bool) {
        self.extend.stewie.3.slip_guard.store(on, Relaxed);
    }

    /// Whether the missing-data guard is on for Tweaks' rows ([`Engine::set_stewie_slip_guard`]).
    pub fn stewie_slip_guard(&self) -> bool {
        self.extend.stewie.3.slip_guard.load(Relaxed)
    }

    /// Whether the console's print passes over its loading-screen test for this line (S0031's
    /// warning, as Tweaks' byte at `0x71D0DC` while it prints): `fn_0071d0a0` asks it.
    pub fn stewie_print_ungated(&self) -> bool {
        self.extend.stewie_print_ungated
    }

    /// Sets [`Engine::stewie_print_ungated`] (S0031's warning, around its one line).
    pub fn set_stewie_print_ungated(&mut self, on: bool) {
        self.extend.stewie_print_ungated = on;
    }

    /// Every row off.
    pub fn clear_stewie(&self) {
        for word in &self.extend.stewie.0 {
            word.store(0, Relaxed);
        }
    }

    /// How many rows are on.
    pub fn stewie_rows_on(&self) -> u32 {
        self.extend
            .stewie
            .0
            .iter()
            .map(|w| w.load(Relaxed).count_ones())
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_row_is_one_bit_shared_by_the_threads_engines() {
        let e = Engine::new();
        let other = Row::at(OTHER_TWEAKS, 70);
        assert!(!e.stewie(DIALOG_SPEECH_NULL_CHECK));
        e.set_stewie(DIALOG_SPEECH_NULL_CHECK, true);
        assert!(e.stewie(DIALOG_SPEECH_NULL_CHECK));
        assert!(!e.stewie(other), "only its own bit");
        e.set_stewie(other, true);
        assert_eq!(e.stewie_rows_on(), 2);
        // A real thread's engine sees the same table.
        let thread = e.extend.for_thread();
        assert!(thread.stewie.0[0].load(Relaxed) & 1 != 0);
        e.set_stewie(DIALOG_SPEECH_NULL_CHECK, false);
        assert!(!e.stewie(DIALOG_SPEECH_NULL_CHECK));
        assert!(e.stewie(other));
        assert_eq!(thread.stewie.0[0].load(Relaxed) & 1, 0);
        e.clear_stewie();
        assert_eq!(e.stewie_rows_on(), 0);
    }

    #[test]
    fn the_groups_fit_the_table_in_order() {
        assert_eq!(Row::at(OTHER_TWEAKS, 1023).index(), WORDS * 64 - 1);
    }

    #[test]
    fn a_once_message_is_once_a_game_on_every_threads_engine() {
        let e = Engine::new();
        let thread = e.extend.for_thread();
        assert!(e.stewie_once(once::MOVEMENT_SEQUENCE));
        assert!(!e.stewie_once(once::MOVEMENT_SEQUENCE));
        assert_eq!(thread.stewie.1.load(Relaxed), 1);
        // Clearing the rows leaves the messages as they are, and a new game starts with none shown.
        e.clear_stewie();
        assert!(!e.stewie_once(once::MOVEMENT_SEQUENCE));
        assert!(Engine::new().stewie_once(once::MOVEMENT_SEQUENCE));
    }

    #[test]
    fn the_console_print_hands_the_format_and_its_words_to_the_console() {
        use crate::abi::Ret;
        use std::sync::Mutex;
        static SEEN: Mutex<Vec<(Vec<u8>, Vec<u32>)>> = Mutex::new(Vec::new());
        let mut e = Engine::new();
        e.register(CONSOLE_INSTANCE, |_, a| {
            assert_eq!(a[0], 1);
            Ret {
                eax: 0x4000,
                ..Ret::default()
            }
        });
        e.register(CONSOLE_PRINT, |e, a| {
            assert_eq!(a[0], 0x4000);
            let mut text = vec![];
            let mut at = a[1];
            while e.mem.u8(at) != 0 {
                text.push(e.mem.u8(at));
                at += 1;
            }
            let words = (0..2).map(|i| e.mem.u32(a[2] + 4 * i)).collect();
            SEEN.lock().unwrap().push((text, words));
            Ret::default()
        });
        console_print(&mut e, b"Failed %s (%08X)", &[0x1234, 0xbeef]);
        assert_eq!(
            SEEN.lock().unwrap().as_slice(),
            [(b"Failed %s (%08X)".to_vec(), vec![0x1234, 0xbeef])]
        );
        // No console: nothing printed.
        e.register(CONSOLE_INSTANCE, |_, _| Ret::default());
        console_print(&mut e, b"x", &[]);
        assert_eq!(SEEN.lock().unwrap().len(), 1);
    }

    /// The text at `at` up to its 0.
    fn text(e: &Engine, mut at: u32) -> Vec<u8> {
        let mut out = vec![];
        while e.mem.u8(at) != 0 {
            out.push(e.mem.u8(at));
            at += 1;
        }
        out
    }

    #[test]
    fn a_kept_block_is_made_once_zeroed_and_shared() {
        let mut e = Engine::new();
        let a = e.stewie_kept(kept::RUMBLE_STATES, 0x24);
        assert_ne!(a, 0);
        assert!((0..0x24).all(|i| e.mem.u8(a + i) == 0));
        e.mem.set_u32(a, 7);
        assert_eq!(
            e.stewie_kept(kept::RUMBLE_STATES, 0x24),
            a,
            "the same block"
        );
        assert_eq!(e.mem.u32(a), 7, "not made or zeroed again");
        let b = e.stewie_kept(kept::SPEAKER_NAME, 4);
        assert!(b != 0 && b != a);
        // Every thread's engine holds the same addresses.
        let thread = e.extend.for_thread();
        assert_eq!(thread.stewie.2[kept::RUMBLE_STATES].load(Relaxed), a);
    }

    /// S0023's print: once per linked reference, not again for the model path printed last, with
    /// "<unknown mesh>" and id FFFFFFFF for a node with no reference, nothing with
    /// `bInvalidBSFadeNodePrints` off (the id still remembered).
    #[test]
    fn the_invalid_fade_node_line_is_printed_as_tweaks_prints_it() {
        use crate::abi::Ret;
        static LINES: Mutex<Vec<(Vec<u8>, u32, Vec<u8>)>> = Mutex::new(Vec::new());
        const PATH_SLOT: u32 = 0x00ff_5000;
        let mut e = Engine::new();
        e.register(CONSOLE_INSTANCE, |_, _| Ret {
            eax: 0x4000,
            ..Ret::default()
        });
        e.register(CONSOLE_PRINT, |e, a| {
            let line = (text(e, a[1]), e.mem.u32(a[2]), text(e, e.mem.u32(a[2] + 4)));
            LINES.lock().unwrap().push(line);
            Ret::default()
        });
        // The cast answers the form + 0x30 as its model; the model's path is its word at +4.
        e.register(DYNAMIC_CAST, |_, a| {
            assert_eq!((a[2], a[3]), (RTTI_TES_FORM, RTTI_TES_MODEL));
            Ret {
                eax: a[0] + 0x30,
                ..Ret::default()
            }
        });
        e.register(PATH_SLOT, |e, a| Ret {
            eax: e.mem.u32(a[0] + 4),
            ..Ret::default()
        });
        let vtable = e.mem.alloc(0x20);
        e.mem.set_u32(vtable + 0x14, PATH_SLOT);
        let path = e.mem.alloc(16);
        for (i, b) in b"rock.nif\0".iter().enumerate() {
            e.mem.set_u8(path + i as u32, *b);
        }
        let base = e.mem.alloc(0x40);
        e.mem.set_u32(base + 0x30, vtable);
        e.mem.set_u32(base + 0x34, path);
        let reference = |e: &mut Engine, id: u32| {
            let r = e.mem.alloc(0x24);
            e.mem.set_u32(r + 0xc, id);
            e.mem.set_u32(r + 0x20, base);
            r
        };
        let node = |e: &mut Engine, linked: u32| {
            let n = e.mem.alloc(0xe4);
            e.mem.set_u32(n + 0xcc, linked);
            n
        };
        let format = b"[Tweaks] Invalid BSFadeNode Detected - Linked Obj ID: %08X (%s)".to_vec();
        let first = reference(&mut e, 0x1234);
        let a = node(&mut e, first);
        invalid_fade_node_print(&mut e, a);
        invalid_fade_node_print(&mut e, a);
        assert_eq!(
            LINES.lock().unwrap().as_slice(),
            [(format.clone(), 0x1234, b"rock.nif".to_vec())],
            "once for the reference"
        );
        // Another reference with the same model: the path printed last, no line.
        let second = reference(&mut e, 0x5678);
        let b = node(&mut e, second);
        invalid_fade_node_print(&mut e, b);
        assert_eq!(LINES.lock().unwrap().len(), 1);
        // No reference: id FFFFFFFF and the unknown mesh.
        let c = node(&mut e, 0);
        invalid_fade_node_print(&mut e, c);
        assert_eq!(
            LINES.lock().unwrap()[1],
            (format.clone(), 0xffff_ffff, b"<unknown mesh>".to_vec())
        );
        // Prints off: nothing, the id remembered (back on, the same reference prints nothing).
        e.set_stewie_fade_node_prints(false);
        invalid_fade_node_print(&mut e, a);
        e.set_stewie_fade_node_prints(true);
        invalid_fade_node_print(&mut e, a);
        assert_eq!(LINES.lock().unwrap().len(), 2);
        // A new id after an unknown mesh prints again with the model's path.
        invalid_fade_node_print(&mut e, b);
        assert_eq!(LINES.lock().unwrap()[2].1, 0x5678);
        // Only an immediate repeat is skipped: two references with other models checked in turn
        // (one with its own model, then a, then the first again) print each time.
        let tree = e.mem.alloc(16);
        for (i, ch) in b"tree.nif\0".iter().enumerate() {
            e.mem.set_u8(tree + i as u32, *ch);
        }
        let other_base = e.mem.alloc(0x40);
        e.mem.set_u32(other_base + 0x30, vtable);
        e.mem.set_u32(other_base + 0x34, tree);
        let third = e.mem.alloc(0x24);
        e.mem.set_u32(third + 0xc, 0x9abc);
        e.mem.set_u32(third + 0x20, other_base);
        let d = node(&mut e, third);
        for n in [d, a, d] {
            invalid_fade_node_print(&mut e, n);
        }
        let lines = LINES.lock().unwrap();
        let tail: Vec<(u32, Vec<u8>)> = lines[3..].iter().map(|l| (l.1, l.2.clone())).collect();
        assert_eq!(
            tail,
            [
                (0x9abc, b"tree.nif".to_vec()),
                (0x1234, b"rock.nif".to_vec()),
                (0x9abc, b"tree.nif".to_vec())
            ]
        );
    }

    /// S0031's warning: the loader's model name (or "unknown"), once until another model is
    /// named, printed past the console's loading-screen test; a name of 100 bytes or more is
    /// printed each time (Tweaks' 100-byte copy keeps 99 and never matches it).
    #[test]
    fn the_null_shape_warning_names_the_model_once() {
        use crate::abi::Ret;
        static LINES: Mutex<Vec<(Vec<u8>, bool)>> = Mutex::new(Vec::new());
        let mut e = Engine::new();
        e.register(CONSOLE_INSTANCE, |_, _| Ret {
            eax: 0x4000,
            ..Ret::default()
        });
        e.register(CONSOLE_PRINT, |e, a| {
            assert_eq!(text(e, a[1]), b"%s");
            let line = (text(e, e.mem.u32(a[2])), e.stewie_print_ungated());
            LINES.lock().unwrap().push(line);
            Ret::default()
        });
        e.mem.map(LOADING_MODEL_NAME & !0xfff, 0x1000);
        let name = |e: &mut Engine, s: &[u8]| {
            let at = e.mem.alloc(s.len() as u32 + 1);
            for (i, b) in s.iter().enumerate() {
                e.mem.set_u8(at + i as u32, *b);
            }
            e.mem.set_u32(LOADING_MODEL_NAME, at);
        };
        let line = |n: &str| {
            format!("hkpRigidBody error: {n} has a missing shape (unordered blocks?)").into_bytes()
        };
        name(&mut e, b"meshes/a.nif");
        null_hkp_shape_warning(&mut e);
        null_hkp_shape_warning(&mut e);
        assert_eq!(
            LINES.lock().unwrap().as_slice(),
            [(line("meshes/a.nif"), true)]
        );
        assert!(!e.stewie_print_ungated(), "gated again after the line");
        e.mem.set_u32(LOADING_MODEL_NAME, 0);
        null_hkp_shape_warning(&mut e);
        null_hkp_shape_warning(&mut e);
        name(&mut e, b"");
        null_hkp_shape_warning(&mut e);
        assert_eq!(LINES.lock().unwrap().len(), 2);
        assert_eq!(LINES.lock().unwrap()[1].0, line("unknown"));
        let long = "x".repeat(100);
        name(&mut e, long.as_bytes());
        null_hkp_shape_warning(&mut e);
        null_hkp_shape_warning(&mut e);
        assert_eq!(
            LINES.lock().unwrap().len(),
            4,
            "a name of 100 bytes every time"
        );
        assert_eq!(LINES.lock().unwrap()[3].0, line(&long));
    }

    /// S0225's comparison: ADDRESSU, then ADDRESSV; the group's current value when its valid byte
    /// is set, else its saved one, else that state is passed over; a state the slot map puts at 5
    /// or more is passed over; true at the first that differs from the render state's.
    #[test]
    fn the_sampler_state_compare_reads_as_tweaks_does() {
        use crate::abi::Ret;
        const GETTER: u32 = 0x00ff_6000;
        let mut e = Engine::new();
        for (at, len) in [
            (VALID_SAMPLER_STATES, 8u32),
            (SAMPLER_STATE_SLOTS, 0x10),
            (SAMPLER_RENDER_STATE, 4),
            (RENDER_STATE_GET_SAMPLER_STATE, 4),
        ] {
            e.mem.map(at, len);
        }
        e.mem.set_u32(VALID_SAMPLER_STATES, 1);
        e.mem.set_u32(VALID_SAMPLER_STATES + 4, 2);
        e.mem.set_u16(SAMPLER_STATE_SLOTS + 2, 0);
        e.mem.set_u16(SAMPLER_STATE_SLOTS + 4, 1);
        // The render state's value for (stage, state): the word at cache + 4 * state.
        let cache = e.mem.alloc(0x10);
        e.mem.set_u32(SAMPLER_RENDER_STATE, cache);
        e.mem.set_u32(RENDER_STATE_GET_SAMPLER_STATE, GETTER);
        e.register(GETTER, |e, a| {
            assert_eq!(a[1], 2, "the stage's number");
            Ret {
                eax: e.mem.u32(a[0] + 4 * a[2]),
                ..Ret::default()
            }
        });
        let group = e.mem.alloc(0xb8);
        let stage = e.mem.alloc(0x68);
        e.mem.set_u32(stage + 4, 2);
        e.mem.set_u32(stage + 0x10, group);
        // Neither state held by the group: nothing differs.
        e.mem.set_u32(cache + 4, 7);
        assert!(!sampler_state_changed(&mut e, stage));
        // ADDRESSU current 7 = the device's; ADDRESSV saved 3, the device's 0: differs.
        e.mem.set_u8(group + 0xa4, 1);
        e.mem.set_u32(group + 0x90, 7);
        assert!(!sampler_state_changed(&mut e, stage));
        e.mem.set_u8(group + 0x81, 1);
        e.mem.set_u32(group + 0x6c + 4, 3);
        assert!(sampler_state_changed(&mut e, stage), "the second state");
        e.mem.set_u32(cache + 8, 3);
        assert!(!sampler_state_changed(&mut e, stage));
        // The current value wins over the saved one.
        e.mem.set_u8(group + 0xa5, 1);
        e.mem.set_u32(group + 0x94, 4);
        assert!(sampler_state_changed(&mut e, stage), "current over saved");
        // A state mapped to slot 5 or more is not one the group keeps.
        e.mem.set_u16(SAMPLER_STATE_SLOTS + 4, 5);
        assert!(!sampler_state_changed(&mut e, stage));
    }

    /// S0136's read: the texture's data byte, 0 without data, and for memory that is not there 0
    /// with Tweaks' line once a game, queued off the main thread or before the console exists and
    /// printed by the main loop's drain.
    #[test]
    fn the_queued_texture_state_reads_as_tweaks_try_does() {
        use crate::abi::Ret;
        static LINES: Mutex<Vec<Vec<u8>>> = Mutex::new(Vec::new());
        let mut e = Engine::new();
        e.register(CONSOLE_INSTANCE, |_, _| Ret {
            eax: 0x4000,
            ..Ret::default()
        });
        e.register(CONSOLE_PRINT, |e, a| {
            assert_eq!(text(e, a[1]), b"%s");
            LINES.lock().unwrap().push(text(e, e.mem.u32(a[2])));
            Ret::default()
        });
        e.mem.map(CONSOLE_SINGLETON & !0xfff, 0x1000);
        let texture = e.mem.alloc(0x30);
        assert_eq!(queued_texture_state(&mut e, texture), 0, "no data");
        let data = e.mem.alloc(0x60);
        e.mem.set_u32(texture + 0x24, data);
        e.mem.set_u8(data + 0x5b, 3);
        assert_eq!(queued_texture_state(&mut e, texture), 3);
        assert!(LINES.lock().unwrap().is_empty());
        // Data that is not there (a texture freed under the loader), then no texture at all: 0,
        // and the line once, queued while the console does not exist yet.
        e.mem.unmap(0, 0x1000);
        e.mem.set_u32(texture + 0x24, 0x0000_0100);
        assert_eq!(queued_texture_state(&mut e, texture), 0);
        assert_eq!(queued_texture_state(&mut e, 0), 0);
        assert!(LINES.lock().unwrap().is_empty(), "queued, not printed");
        print_queued(&mut e);
        let line = b"[Tweaks] Prevented crash when texture failed to load...".to_vec();
        assert_eq!(*LINES.lock().unwrap(), vec![line.clone()]);
        print_queued(&mut e);
        assert_eq!(LINES.lock().unwrap().len(), 1, "the queue emptied");
        // With the console made, on the main thread, a line prints at once.
        e.mem.set_u32(CONSOLE_SINGLETON, 0x4000);
        console_queue_or_print(&mut e, b"now");
        assert_eq!(LINES.lock().unwrap()[1], b"now".to_vec());
        print_queued(&mut e);
        assert_eq!(LINES.lock().unwrap().len(), 2);
    }

    /// Review of #842: off the main thread the line waits even with the console made (Tweaks'
    /// `isMainThread` test), and the main loop's drain prints it once.
    #[test]
    fn a_line_printed_off_the_main_thread_waits_for_the_drain() {
        use crate::abi::Ret;
        static LINES: Mutex<Vec<Vec<u8>>> = Mutex::new(Vec::new());
        let mut e = Engine::new();
        e.register(CONSOLE_INSTANCE, |_, _| Ret {
            eax: 0x4000,
            ..Ret::default()
        });
        e.register(CONSOLE_PRINT, |e, a| {
            LINES.lock().unwrap().push(text(e, e.mem.u32(a[2])));
            Ret::default()
        });
        e.mem.map(CONSOLE_SINGLETON & !0xfff, 0x1000);
        e.mem.set_u32(CONSOLE_SINGLETON, 0x4000);
        e.kernel.threads.current = 1;
        assert!(!crate::thread_host::on_main(&e));
        e.call_log = Some(vec![]);
        console_queue_or_print(&mut e, b"from a worker");
        assert!(e
            .call_log
            .as_ref()
            .unwrap()
            .iter()
            .all(|(a, _)| *a != CONSOLE_PRINT));
        assert!(LINES.lock().unwrap().is_empty(), "queued, not printed");
        e.kernel.threads.current = 0;
        print_queued(&mut e);
        print_queued(&mut e);
        assert_eq!(*LINES.lock().unwrap(), vec![b"from a worker".to_vec()]);
    }

    /// S0050's line: the reference's id and its last file's name (`GetFile(-1)`), an empty name
    /// without a file; not again for the reference seen last; at most 255 bytes; queued before the
    /// console exists.
    #[test]
    fn the_lod_warning_names_the_reference_and_its_last_file_once_in_a_row() {
        use crate::abi::Ret;
        static LINES: Mutex<Vec<Vec<u8>>> = Mutex::new(Vec::new());
        static FILE: AtomicU32 = AtomicU32::new(0);
        let mut e = Engine::new();
        e.register(CONSOLE_INSTANCE, |_, _| Ret {
            eax: 0x4000,
            ..Ret::default()
        });
        e.register(CONSOLE_PRINT, |e, a| {
            LINES.lock().unwrap().push(text(e, e.mem.u32(a[2])));
            Ret::default()
        });
        e.register(0x0048_4e60, |_, a| {
            assert_eq!(a[1], u32::MAX, "the last file");
            Ret {
                eax: FILE.load(Relaxed),
                ..Ret::default()
            }
        });
        e.mem.map(CONSOLE_SINGLETON & !0xfff, 0x1000);
        e.mem.set_u32(CONSOLE_SINGLETON, 0x4000);
        let first = e.mem.alloc(0x20);
        e.mem.set_u32(first + 0xc, 0x0012_ab34);
        let second = e.mem.alloc(0x20);
        e.mem.set_u32(second + 0xc, 0xff00_0001);
        let file = e.mem.alloc(0x200);
        for (i, b) in b"Mod.esp".iter().enumerate() {
            e.mem.set_u8(file + 0x20 + i as u32, *b);
        }
        FILE.store(file, Relaxed);
        lod_warning(&mut e, first);
        lod_warning(&mut e, first);
        FILE.store(0, Relaxed);
        lod_warning(&mut e, second);
        lod_warning(&mut e, first);
        assert_eq!(
            *LINES.lock().unwrap(),
            vec![
                b"Warning: reference 0012AB34 has LOD but is disabled by plugin Mod.esp".to_vec(),
                b"Warning: reference FF000001 has LOD but is disabled by plugin ".to_vec(),
                b"Warning: reference 0012AB34 has LOD but is disabled by plugin ".to_vec(),
            ]
        );
        // A long name: the line is cut at 255 bytes, as Tweaks' buffer cuts it.
        for i in 0..0x103 {
            e.mem.set_u8(file + 0x20 + i, b'x');
        }
        FILE.store(file, Relaxed);
        lod_warning(&mut e, second);
        assert_eq!(LINES.lock().unwrap()[3].len(), 255);
        // Before the console exists the line waits for the main loop's drain.
        e.mem.set_u32(CONSOLE_SINGLETON, 0);
        lod_warning(&mut e, first);
        assert_eq!(LINES.lock().unwrap().len(), 4);
        print_queued(&mut e);
        assert_eq!(LINES.lock().unwrap().len(), 5);
    }
}
