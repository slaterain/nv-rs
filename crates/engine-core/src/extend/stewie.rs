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

use crate::{args, Engine};
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

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
/// S1016 (Engine Optimizations, `Inlines::SaveLoad`: `Write32(0x845558, 0x2735)`): the changes map's
/// buckets 10,037, not the exe's 5,039 (`00845550`). It changes the order a save walks and writes the
/// changed forms, so it is a fixes row.
pub const CHANGES_MAP_BUCKETS: Row = Row::at(FIXES, 0);

/// Tweaks' messages it prints once a run ([`Engine::stewie_once`]).
pub mod once {
    /// S0173's "Failed to resolve movement sequence" line.
    pub const MOVEMENT_SEQUENCE: u32 = 0;
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

/// The table: one bit a row, and the once-a-run messages' flags.
#[derive(Debug)]
pub(crate) struct Bits([AtomicU64; WORDS], AtomicU64);

impl Default for Bits {
    fn default() -> Self {
        Bits(
            std::array::from_fn(|_| AtomicU64::new(0)),
            AtomicU64::new(0),
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
}
