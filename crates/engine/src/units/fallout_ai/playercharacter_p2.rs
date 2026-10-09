//! `fallout/ai/playercharacter.cpp` (Xbox PDB source unit), part 2: its functions from `0094c490` up to
//! (not including) `0095d090` in FalloutNV.exe 1.4.0.525, translated
//! (docs/ENGINE_CRATE.md). The unit's shared layouts and helpers are in
//! [`super::playercharacter`]; anything public there may be used here.
//!
//! This session covers `0094c490` to `009500a0` (40 functions): the
//! actor-value modifier accessors, the spell and scroll selection, the
//! parent-cell change handler (`0094cae0`), `ReturnToLastKnownGoodPosition`,
//! the one-hour wait step (`0094df80`), `Load3D`, `ExportProgressData`,
//! `RewardKarma` and the vanity-mode reset.
//!
//! Notes for the next session: this part continues at `00950110`
//! (`PlayerCharacter::SetFirstPerson`).
//!
//! Conventions: the `ActorValueOwner` sits at `PlayerCharacter + 0xa4` and the
//! `MagicCaster` at `+0x88`, so a few functions receive those interior
//! pointers as `this` and subtract the offset. Many callees are `__thiscall`
//! functions that return with a plain `RET` while the caller has pushed words
//! for the next call; every call was read from the disassembly with that in
//! mind. The compiler's exception-unwinding frames, the stack-cookie checks
//! and the debug scope object of `Load3D` are not translated, except that the
//! scope object is constructed and destroyed as the game does.

#[allow(unused_imports)]
use super::playercharacter::*;
#[allow(unused_imports)]
use crate::prelude::*;
use crate::types::{BSSimpleList, NiPoint3};

// ---------------------------------------------------------------------
// Globals
// ---------------------------------------------------------------------

/// `PlayerCharacter *`.
const PLAYER_POINTER: u32 = 0x011d_ea3c;
/// The global object whose word at `+0x68` (read through `008d8520`) is handed
/// to `Sky::Update` and whose `00451530` test guards the border-region check.
const SKY_HOLDER_POINTER: u32 = 0x011d_ea10;
/// A global object pointer tested by `005dc960` and `00678ce0` before the
/// "called before the game was over" log line of `0094eb40`.
const GAME_STATE_POINTER: u32 = 0x011d_ea0c;
/// The `ModelLoader` singleton pointer (`ECX` of `ModelLoader::LoadFile`).
const MODEL_LOADER_POINTER: u32 = 0x011c_3b3c;
/// The object `0042ce10` is called on at the start of `Load3D`.
const LOAD_STATE_POINTER: u32 = 0x011d_df38;
/// The `TESIdleManager` singleton pointer (`ECX` of `GetRootFilenameList`).
const IDLE_MANAGER_POINTER: u32 = 0x011c_b6a0;
/// The calendar singleton.
const CALENDAR: u32 = 0x011d_e7b8;
/// The `ProcessLists` singleton.
const PROCESS_LISTS: u32 = 0x011e_0e80;
/// The object whose float getter (`0084d030`) feeds `Sky::Update`.
const TIMER_SOURCE_011F6394: u32 = 0x011f_6394;
/// The lock object taken by `0040fbf0` and released by `0040fba0`, and the
/// byte that says the processing under it is enabled.
const PROCESS_LOCK: u32 = 0x011f_11a0;
const PROCESS_LOCK_ENABLED: u32 = 0x011f_1220;
/// The byte tested at the start of each wait step (`0094df80`): the step then
/// also updates the magic and the health; the step that ends the countdown sets
/// it to 1 (its value in the exe's data is 1).
const WAIT_STEP_ACTIVE: u32 = 0x011a_3b30;
/// The byte `0094dba0` returns.
const SKIP_WATER_HEIGHT_FLAG: u32 = 0x011c_7a5a;
/// The globals `0094e1d0`, `0094eaf0` and `0094eb40` fill with nodes found by
/// name (slot `0x9c`) in the player's models: the node named by `0094ead0`, the
/// "Camera1st" node, the "Camera3rd" node and the "Bip" node (the names are
/// the ones of the error messages).
const NODE_BY_EAD0_SLOT: u32 = 0x011e_07cc;
const NODE_CAMERA_1ST_SLOT: u32 = 0x011e_07d0;
const NODE_CAMERA_3RD_SLOT: u32 = 0x011e_07d4;
const NODE_BIP_SLOT: u32 = 0x011e_07d8;
/// The words `0094eac0`, `0094ead0`, `0094eae0` and `0094eb30` return (node
/// names the 3D lookups use).
const NODE_NAME_EAC0: u32 = 0x011c_61a8;
const NODE_NAME_EAD0: u32 = 0x011c_6264;
const NODE_NAME_EAE0: u32 = 0x011c_626c;
const NODE_NAME_EB30: u32 = 0x011c_6270;
/// The byte `00950090` returns, and the vanity-mode globals `009500a0` resets.
const VANITY_ACTIVE: u32 = 0x011e_07b8;
const VANITY_SAVED_FLAG: u32 = 0x011e_07b9;
const VANITY_SAVED_VALUE: u32 = 0x011e_07bc;
const VANITY_RESTORED_VALUE: u32 = 0x011e_0b5c;
const VANITY_FLAG_07C1: u32 = 0x011e_07c1;
const VANITY_FLAG_07C3: u32 = 0x011e_07c3;
const VANITY_VALUE_07C4: u32 = 0x011e_07c4;
const VANITY_VALUE_07C8: u32 = 0x011e_07c8;
const VANITY_VALUE_07DC: u32 = 0x011e_07dc;
const VANITY_VALUE_0B58: u32 = 0x011e_0b58;
const VANITY_VALUE_0B60: u32 = 0x011e_0b60;
/// The region that `0094cae0` found to hold sounds, kept for the sound code.
const CURRENT_REGION_WITH_SOUNDS: u32 = 0x011d_d380;
/// The `char *` table indexed by the reason argument of `ExportProgressData`.
const REASON_NAME_TABLE: u32 = 0x011a_3b44;
/// The `char *` table indexed by the value of `0087f4c0` (the sex of the player).
const SEX_NAME_TABLE: u32 = 0x0119_9e8c;
/// The `char *` table indexed by the cause word (`+0x10`) of the death extra data.
const CAUSE_NAME_TABLE: u32 = 0x0118_4a7c;
/// The game version string and the build-name buffer written to the export.
const GAME_VERSION_TEXT: u32 = 0x0107_6e68;
const BUILD_NAME_BUFFER: u32 = 0x011c_5fa0;
const UNKNOWN_TEXT: u32 = 0x0101_5890;
/// The nine words (a 3x3 matrix) `Load3D` copies and hands to `MultipleMatrixByRace`.
const LOAD_3D_MATRIX: u32 = 0x011a_9448;
/// The word `ExportProgressData` passes as the second argument of the file's
/// slot `0x14` (seek).
const EXPORT_SEEK_MODE: u32 = 0x010a_2488;

// Settings: objects whose text (`00403df0`), float (`00403e20`) or integer
// (`0043d4d0`) value the code reads. They are constructed at start-up (zero in
// the exe's data), so their names are the roles the code gives them.
/// Its text is the path of the player's 1st-person model.
const MODEL_PATH_SETTING: u32 = 0x011c_dd78;
/// Its text is the directory prefix of the progress-data file.
const EXPORT_DIRECTORY_SETTING: u32 = 0x011e_09d4;
/// The `float` that limits the change of actor value `0x4b` per wait step.
const ACTOR_VALUE_4B_LIMIT_SETTING: u32 = 0x011d_0c24;
// The karma is kept within the lower and upper limit; a change of the size of
// the threshold or more shows the big message. Each message is an icon and a
// text setting.
const KARMA_LOWER_LIMIT_SETTING: u32 = 0x011c_dd6c;
const KARMA_UPPER_LIMIT_SETTING: u32 = 0x011c_d644;
const KARMA_MESSAGE_THRESHOLD_SETTING: u32 = 0x011d_26ac;
const KARMA_LOSS_BIG_ICON_SETTING: u32 = 0x011d_42c4;
const KARMA_LOSS_BIG_TEXT_SETTING: u32 = 0x011d_4a14;
const KARMA_LOSS_ICON_SETTING: u32 = 0x011d_2cd0;
const KARMA_LOSS_TEXT_SETTING: u32 = 0x011d_3420;
const KARMA_GAIN_ICON_SETTING: u32 = 0x011d_3e38;
const KARMA_GAIN_TEXT_SETTING: u32 = 0x011d_1fb0;
const KARMA_GAIN_BIG_ICON_SETTING: u32 = 0x011d_3b14;
const KARMA_GAIN_BIG_TEXT_SETTING: u32 = 0x011d_3c70;
/// The text of the message shown by `ReturnToLastKnownGoodPosition`.
const RETURN_MESSAGE_SETTING: u32 = 0x011d_50bc;
/// A boolean setting read by `0094cae0` through `00408d60`.
const BORDER_REGION_SETTING: u32 = 0x011e_08e0;

// Float and double constants in the exe's data.
/// `2.0f`: the display time of the on-screen messages.
const MESSAGE_SECONDS: u32 = 0x0101_62c0;
/// `-FLT_MAX`: the water height when the cell has none.
const NO_WATER_HEIGHT: u32 = 0x0101_5f5c;
/// `4096.0` (`double`): the size of an exterior cell in world units.
const CELL_SIZE: u32 = 0x0101_7a10;
/// `2048.0` (`double`): the border distance limit.
const BORDER_DISTANCE_LIMIT: u32 = 0x0101_6968;
/// The sphere radius of the camera body (`float`, `0.001`).
const CAMERA_SPHERE_RADIUS: u32 = 0x0101_7d00;
/// `3600.0` (`double`): seconds per hour.
const SECONDS_PER_HOUR: u32 = 0x0101_2640;
/// The `float` (`3600.0`) the wait step passes to the process-list updates.
const WAIT_UPDATE_TIME: u32 = 0x0108_4838;

// Strings in the exe's data.
const SOURCE_FILE_NAME: u32 = 0x0108_a8e0;
const SAD_ICON_PATH: u32 = 0x0102_08a0;
const EMPTY_QUOTE_TEXT: u32 = 0x0102_1184;
const SOUND_KARMA_UP: u32 = 0x0108_b2d0;
const SOUND_KARMA_DOWN: u32 = 0x0108_b2dc;
const FORMAT_TEXT: u32 = 0x0108_b2b4;
const FORMAT_INT: u32 = 0x0108_b2b0;
const FORMAT_EXPORT_PATH: u32 = 0x0108_b2b8;
const FORMAT_PERK: u32 = 0x0108_b2a0;
const TEXT_CLOSE_QUOTE: u32 = 0x0108_b29c;
const TEXT_EMPTY_QUOTES: u32 = 0x0108_b298;
const FORMAT_TWO_FLOATS: u32 = 0x0108_b28c;
const FORMAT_ONE_FLOAT: u32 = 0x0108_b284;
const FORMAT_QUEST: u32 = 0x0108_b278;
const FORMAT_CELL_WORLD: u32 = 0x0108_b264;
const FORMAT_CELL_UNKNOWN_WORLD: u32 = 0x0108_b248;
const FORMAT_NAME_ID: u32 = 0x0108_b23c;
const TEXT_NONE: u32 = 0x0108_b234;
const FORMAT_CAUSE_OBJECT: u32 = 0x0108_b224;
const TEXT_UNKNOWN_CAUSE: u32 = 0x0108_b214;
const TEXT_UNKNOWN_OBJECT: u32 = 0x0108_b204;
const FORMAT_LEVELLED_BOTH: u32 = 0x0108_b1ec;
const FORMAT_LEVELLED_ORIGINAL_ONLY: u32 = 0x0108_b1d4;
const FORMAT_LEVELLED_TEMPLATE_ONLY: u32 = 0x0108_b1bc;
const TEXT_UNKNOWN_LEVELLED: u32 = 0x0108_b1a4;
const TEXT_NO_WEAPON: u32 = 0x0108_b198;
const TEXT_UNKNOWN_KILLER: u32 = 0x0108_b188;
const TEXT_LINE_END: u32 = 0x0101_f520;
const FORMAT_SET3D_MESSAGE: u32 = 0x0108_b130;
const FORMAT_MISSING_CAMERA_1ST: u32 = 0x0108_b110;
const FORMAT_MISSING_BIP: u32 = 0x0108_b0f8;
const FORMAT_MISSING_CAMERA_3RD: u32 = 0x0108_b0b8;
const TODDLER_SUFFIX: u32 = 0x0108_b0d8;
const FEMALE_SUFFIX: u32 = 0x0101_6f38;
const MALE_SUFFIX: u32 = 0x0101_6f1c;
const INVENTORY_PREFIX_WORN: u32 = 0x0108_b184;
const INVENTORY_PREFIX_CARRIED: u32 = 0x0101_1584;
const FORMAT_INVENTORY_ITEM: u32 = 0x0108_b174;

// ---------------------------------------------------------------------
// Functions of other units
// ---------------------------------------------------------------------

/// Reads the modifier of kind `mode` (0 temporary, 1 script, 2 damage) of an
/// actor value; `ECX` = the player (`0094c3d0`, result in ST0).
const GET_MODIFIER: u32 = 0x0094_c3d0;
/// `ClampAdd(current, delta, mode)` (`00937810`, `cdecl`): the sum, clamped to
/// at most 0 when `mode` is 0 and to at least 0 when `mode` is 1.
const CLAMP_ADD: u32 = 0x0093_7810;
/// `ActorValue::CheckClampDamageModifier` (Xbox PDB), `cdecl`: owner, actor
/// value index, `float`.
const CHECK_CLAMP_DAMAGE_MODIFIER: u32 = 0x0066_eea0;
/// Forwarded to by `0094c640` (`ECX` = the actor, two words, `float` result).
const MODIFIER_ITEM_GETTER: u32 = 0x0088_0660;
/// Forwarded to by `0094c660` (`ECX` = the actor, a word and a `float`).
const MODIFIER_ITEM_SETTER: u32 = 0x0088_0690;
/// `Actor::AddSpell` (Xbox PDB name of `008c32f0`) and `Actor::RemoveSpell`
/// (`008c3400`).
const ACTOR_ADD_SPELL: u32 = 0x008c_32f0;
const ACTOR_REMOVE_SPELL: u32 = 0x008c_3400;
/// `008c45e0`: `ECX` = the actor, two words, the address of an out word and a
/// flag byte; returns a bool.
const SPELL_CHECK: u32 = 0x008c_45e0;
/// `PlayerCharacter::IsGodMode` (Xbox PDB).
const IS_GOD_MODE: u32 = 0x0095_26b0;
/// `MagicCaster::CastAbility` (Xbox PDB name of `00815600`) and
/// `MagicCaster::TransferDisease` (`008157a0`).
const MAGIC_CASTER_CAST_ABILITY: u32 = 0x0081_5600;
const MAGIC_CASTER_TRANSFER_DISEASE: u32 = 0x0081_57a0;
/// `MagicTarget::AddTarget` (Xbox PDB name of `008230f0`).
const MAGIC_TARGET_ADD_TARGET: u32 = 0x0082_30f0;
/// `MagicItem::Unload` and `MagicItem::Preload` (Xbox PDB), one word each.
const MAGIC_ITEM_UNLOAD: u32 = 0x0040_b800;
const MAGIC_ITEM_PRELOAD: u32 = 0x0040_a420;
/// `PlayerCharacter::ResetMagicCastSound` (Xbox PDB).
const RESET_MAGIC_CAST_SOUND: u32 = 0x0096_1f90;
/// The scale of a reference (`00567400`, `TESObjectREFR::GetScale` in the
/// Xbox PDB; ST0).
const REFERENCE_GET_SCALE: u32 = 0x0056_7400;
/// `009a60e0` (`cdecl`): the player and a `float`; returns a pointer or 0.
const PLAYER_LOOKUP_BY_SIZE: u32 = 0x009a_60e0;
/// `00703350`: the fallback used when `009a60e0` finds nothing.
const FALLBACK_LOOKUP: u32 = 0x0070_3350;
/// Base constructor `0056e690` of the object `0094dbb0` builds.
const SPHERE_BASE_CONSTRUCTOR: u32 = 0x0056_e690;
/// `005dc960` and `00678ce0`: boolean tests on the global game-state object.
const GAME_STATE_TEST_1: u32 = 0x005d_c960;
const GAME_STATE_TEST_2: u32 = 0x0067_8ce0;
/// `TESObjectREFR::Set3D` (Xbox PDB name of `005702e0`): the node and a flag.
const REFERENCE_SET_3D: u32 = 0x0057_02e0;
/// `005b5e40` (`cdecl`): logs one message.
const LOG_MESSAGE: u32 = 0x005b_5e40;
/// Destructors with a delete flag: `00418e00` (biped animation) and
/// `00418d20` (animation).
const DELETE_BIPED_ANIM: u32 = 0x0041_8e00;
const DELETE_ANIMATION: u32 = 0x0041_8d20;
/// `NiPointer` assignment (`0066b0d0`): `ECX` = the pointer slot, the new
/// object.
const SMART_POINTER_ASSIGN: u32 = 0x0066_b0d0;
/// `memset` wrapper `00403d30` (`cdecl`): block, value, size.
const MEMSET: u32 = 0x0040_3d30;
/// Reads the word at `+4` of the object in `ECX` (0 for a null `ECX`):
/// `00403df0`, used on settings to get their text.
const SETTING_TEXT: u32 = 0x0040_3df0;
/// Returns its first stack word (`00464f30`).
const IDENTITY: u32 = 0x0046_4f30;
/// `0045a5e0`: `ECX` = the model loader, one word.
const MODEL_LOADER_NOTIFY: u32 = 0x0045_a5e0;
/// `HUDMainMenu::SetMenuMode` (Xbox PDB name of `00771700`), `cdecl`.
const SET_MENU_MODE: u32 = 0x0077_1700;
/// `Interface::GetInventoryMenuVisible` (Xbox PDB name of `00704ad0`).
const INVENTORY_MENU_VISIBLE: u32 = 0x0070_4ad0;
/// `00483710` (11 bytes): called on the player by `00950030` and on sound
/// handles by `RewardKarma`.
const RELEASE_HANDLE: u32 = 0x0048_3710;
/// `ProcessLists::PrintLists` (Xbox PDB name of `008d0600`) and
/// `CharacterProgression::RewardExperience` (`008d5100`).
const PROCESS_LISTS_PRINT_LISTS: u32 = 0x008d_0600;
const CHARACTER_PROGRESSION_REWARD_EXPERIENCE: u32 = 0x008d_5100;
/// `ItemChange::GetWorn` (Xbox PDB name of `004bddd0`), one word.
const ITEM_CHANGE_GET_WORN: u32 = 0x004b_ddd0;
/// The form id of an object (`0084e3a0`: the word at `+0xc`).
const FORM_ID_OF: u32 = 0x0084_e3a0;
/// The word at `+8` of an object (`0044ddc0`): the item's form.
const WORD_AT_8: u32 = 0x0044_ddc0;
/// The full name of an item (`004be2d0`, `ItemChange::GetFullName` in the Xbox
/// PDB).
const ITEM_CHANGE_GET_FULL_NAME: u32 = 0x004b_e2d0;
/// `sprintf` (`00406d00`, `cdecl`) and the `BSString` append `BSStringT<char>::operator+=`
/// (Xbox PDB name of `00404820`).
const SPRINTF: u32 = 0x0040_6d00;
const STRING_APPEND: u32 = 0x0040_4820;
/// List helpers: item slot of a node (`006815c0`), next node (`00726070`),
/// emptiness test (`008256d0`), clear (`00470470`).
const LIST_NODE_ITEM_SLOT: u32 = 0x0068_15c0;
const LIST_NODE_NEXT: u32 = 0x0072_6070;
const LIST_IS_EMPTY: u32 = 0x0082_56d0;
const LIST_CLEAR: u32 = 0x0047_0470;
/// `MobileObject::GetCurrentParentCell`-style getter `008d6f30`: the player's
/// parent cell.
const PARENT_CELL_OF: u32 = 0x008d_6f30;
/// The extra data list of a reference (`005d43c0`).
const EXTRA_LIST_OF: u32 = 0x005d_43c0;
/// `BaseExtraList::GetExtraData` (Xbox PDB name of `00410220`).
const GET_EXTRA_DATA: u32 = 0x0041_0220;
/// The base form of a reference (`004181e0`).
const BASE_FORM_OF: u32 = 0x0041_81e0;
/// The form type byte of a form (`00401170`).
const FORM_TYPE_OF: u32 = 0x0040_1170;
/// `operator new(size)` (`cdecl`).
const OPERATOR_NEW: u32 = 0x0040_1000;
/// `Error(format, ...)` (`cdecl`).
const ERROR: u32 = 0x0040_fbe0;
/// The setting getters: `00403e20` (float pointer) and `0043d4d0` (int
/// pointer); `ECX` = the setting.
const SETTING_FLOAT_POINTER: u32 = 0x0040_3e20;
const SETTING_INT_POINTER: u32 = 0x0043_d4d0;

// ---------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------

layout! {
    /// `PlayerCharacter` (Xbox PDB), `0xe50` bytes on PC (the PDB's `0xe5c` is
    /// larger: the `TESForm` of the PDB has `0x10` more bytes, and a few
    /// accounting words differ; the constructor `00938180` is allocated with
    /// `0xe50`). Offsets are the PC ones, checked against the code of this
    /// part: PDB offset minus `0x10` for everything after `TESForm`. Only the
    /// fields the translations of this part use are listed.
    pub struct PlayerCharacter: 0xe50 {
        /// `pCurrentProcess` (Xbox PDB): `BaseProcess *`.
        0x68 pCurrentProcess: Ptr,
        /// `pMyKiller` (Xbox PDB): `Actor *`.
        0xc0 pMyKiller: Ptr,
        /// `fHealthModifier` (Xbox PDB): the damage modifier of health.
        0x4ac fHealthModifier: f32,
        /// `iSleepTime` (Xbox PDB): hours left to wait or sleep.
        0x654 iSleepTime: i32,
        /// `bIsSleeping` (Xbox PDB).
        0x658 bIsSleeping: bool,
        /// `b3rdPerson` (Xbox PDB).
        0x64a b3rdPerson: bool,
        /// `p1stPersonBipedAnim` (Xbox PDB): `BipedAnim *`.
        0x68c p1stPersonBipedAnim: Ptr,
        /// `p1stPersonAnimation` (Xbox PDB): `Animation *`.
        0x690 p1stPersonAnimation: Ptr,
        /// `sp1stPerson3D` (Xbox PDB): `NiPointer<NiAVObject>`.
        0x694 sp1stPerson3D: Ptr,
        /// `fEyeHeight` (Xbox PDB).
        0x698 fEyeHeight: f32,
        /// `listQuestLog` (Xbox PDB): `BSSimpleList<TESQuestStageItem *>`.
        0x6b0 listQuestLog: Inline<BSSimpleList>,
        /// `pSelectedSpell` (Xbox PDB): the `MagicItem` interface pointer.
        0x6ec pSelectedSpell: Ptr,
        /// `pSelectedScroll` (Xbox PDB): `TESObjectBOOK *`.
        0x6f0 pSelectedScroll: Ptr,
        /// `bChargen` (Xbox PDB).
        0x75c bChargen: u8,
        /// `pOccupiedRegion` (Xbox PDB): `TESRegion *`.
        0x760 pOccupiedRegion: Ptr,
        /// `m_AllOccupiedRegions` (Xbox PDB): `TESRegionList`, whose
        /// `BSSimpleList` starts 4 bytes in (after its vtable pointer).
        0x764 m_AllOccupiedRegions: Ptr,
        /// The `BSSimpleList` inside `m_AllOccupiedRegions` (`+0x768`).
        0x768 AllOccupiedRegionsList: Inline<BSSimpleList>,
        /// `CurrentRegionSoundList` (Xbox PDB): `BSSimpleList<TESRegionSound *>`.
        0x774 CurrentRegionSoundList: Inline<BSSimpleList>,
        /// `bInBorderContainedCell` (Xbox PDB).
        0x79c bInBorderContainedCell: bool,
        /// `bReturnToLastKnownGoodPosition` (Xbox PDB).
        0x79d bReturnToLastKnownGoodPosition: bool,
        /// `LastKnownGoodPosition` (Xbox PDB).
        0x7a0 LastKnownGoodPosition: Inline<NiPoint3>,
        /// `pLastKnownGoodLocation` (Xbox PDB): `TESForm *`.
        0x7ac pLastKnownGoodLocation: Ptr,
        /// `pBorderRegions` (Xbox PDB): `NiTPrimitiveArray<TESRegion *> *`.
        0x7b0 pBorderRegions: Ptr,
        /// `pLastKnownMusicType` (Xbox PDB): `BGSMusicType *`.
        0x7b4 pLastKnownMusicType: Ptr,
        /// `bIsToddler` (Xbox PDB).
        0x7c6 bIsToddler: bool,
        /// `Perks` (Xbox PDB): `BSSimpleList<PerkRankData *>`.
        0x87c Perks: Inline<BSSimpleList>,
        /// `pWobbleNodes` (Xbox PDB): the first of the cleared pointers.
        0xd74 pWobbleNodes: Ptr,
        /// `spCameraRigidBody` (Xbox PDB): `NiPointer<bhkRigidBody>`.
        0xdec spCameraRigidBody: Ptr,
    }
}

/// Offset of the `ActorValueOwner` sub-object in `PlayerCharacter`.
const ACTOR_VALUE_OWNER: u32 = 0xa4;
/// Offset of the `MagicCaster` sub-object.
const MAGIC_CASTER: u32 = 0x88;
/// Offset of the `MagicTarget` sub-object.
const MAGIC_TARGET: u32 = 0x94;
/// Offset of the `CharacterProgression` sub-object.
const CHARACTER_PROGRESSION: u32 = 0x878;
/// Bases of the actor-value modifier arrays (`float` per actor value).
const TEMPORARY_MODIFIERS: u32 = 0x244;
const SCRIPT_MODIFIERS: u32 = 0x378;
const DAMAGE_MODIFIERS: u32 = 0x4b0;
/// The actor value index that has its own modifier word (`fHealthModifier`).
const HEALTH_ACTOR_VALUE: i32 = 0x10;

/// The word at `+4` of a setting object (its text pointer), `00403df0`.
fn setting_text(e: &mut Engine, setting: u32) -> u32 {
    e.call(SETTING_TEXT, &args![setting]).u32()
}

/// An integer setting (`0043d4d0` gives the address of its value).
fn setting_int(e: &mut Engine, setting: u32) -> i32 {
    let at = e.call(SETTING_INT_POINTER, &args![setting]).u32();
    e.mem.i32(at)
}

/// The value slot of the list node (`006815c0`).
fn list_item_slot(e: &mut Engine, node: u32) -> u32 {
    e.call(LIST_NODE_ITEM_SLOT, &args![node]).u32()
}

/// The next node (`00726070`).
fn list_next(e: &mut Engine, node: u32) -> u32 {
    e.call(LIST_NODE_NEXT, &args![node]).u32()
}

/// Whether the list head is empty (`008256d0`).
fn list_is_empty(e: &mut Engine, node: u32) -> bool {
    e.call(LIST_IS_EMPTY, &args![node]).bool()
}

/// The form id (`0084e3a0`).
fn form_id_of(e: &mut Engine, object: u32) -> u32 {
    e.call(FORM_ID_OF, &args![object]).u32()
}

/// The pointer of the actor's process (`this + 0x68`).
fn process_of(e: &Engine, this: Ptr<PlayerCharacter>) -> u32 {
    e.get(this, PlayerCharacter::pCurrentProcess).addr()
}

// Translated from 0094c490 (decompiled, FalloutNV.exe 1.4.0.525)
/// The damage modifier (kind 2, the array at `+0x4b0`) of an actor value.
/// `this` is the `ActorValueOwner` sub-object at `+0xa4` of the player; the
/// result is `0094c3d0(player, 2, actor_value)`.
pub fn fn_0094c490(e: &mut Engine, this: Ptr, actor_value: u32) -> f32 {
    let player = this.addr().wrapping_sub(ACTOR_VALUE_OWNER);
    e.call(GET_MODIFIER, &args![player, 2u32, actor_value])
        .f32()
}

// Translated from 0094c4c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The script modifier (kind 1, the array at `+0x378`) of an actor value.
/// `this` is the `ActorValueOwner` sub-object at `+0xa4` of the player; the
/// result is `0094c3d0(player, 1, actor_value)`.
pub fn fn_0094c4c0(e: &mut Engine, this: Ptr, actor_value: u32) -> f32 {
    let player = this.addr().wrapping_sub(ACTOR_VALUE_OWNER);
    e.call(GET_MODIFIER, &args![player, 1u32, actor_value])
        .f32()
}

// Translated from 0094c4f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Adds `delta` to the modifier of an actor value. `mode` 0 is the
/// temporary modifier array (`+0x244`), 1 the script modifier array
/// (`+0x378`), 2 the damage modifier array (`+0x4b0`, with the actor value
/// `0x10` kept in `fHealthModifier` at `+0x4ac`); the delta of mode 2 first
/// goes through `ActorValue::CheckClampDamageModifier`. The new value is
/// `ClampAdd(old, delta, clamp)`. An actor value of `-1` changes nothing
/// (mode 2 still calls the damage check). Any other mode does nothing.
pub fn fn_0094c4f0(
    e: &mut Engine,
    this: Ptr<PlayerCharacter>,
    mode: i32,
    actor_value: i32,
    delta: f32,
    clamp: u32,
) {
    let base = this.addr();
    let index_bytes = (actor_value as u32).wrapping_mul(4);
    match mode {
        0 | 1 => {
            if actor_value != -1 {
                let array = if mode == 0 {
                    TEMPORARY_MODIFIERS
                } else {
                    SCRIPT_MODIFIERS
                };
                let at = base.wrapping_add(array).wrapping_add(index_bytes);
                let current = e.mem.f32(at);
                let value = e.call(CLAMP_ADD, &args![current, delta, clamp]).f32();
                e.mem.set_f32(at, value);
            }
        }
        2 => {
            let owner = if this.is_null() {
                0
            } else {
                base.wrapping_add(ACTOR_VALUE_OWNER)
            };
            let checked = e
                .call(
                    CHECK_CLAMP_DAMAGE_MODIFIER,
                    &args![owner, actor_value, delta],
                )
                .f32();
            if actor_value == HEALTH_ACTOR_VALUE {
                let current = e.get(this, PlayerCharacter::fHealthModifier);
                let value = e.call(CLAMP_ADD, &args![current, checked, clamp]).f32();
                e.set(this, PlayerCharacter::fHealthModifier, value);
            } else if actor_value != -1 {
                let at = base
                    .wrapping_add(DAMAGE_MODIFIERS)
                    .wrapping_add(index_bytes);
                let current = e.mem.f32(at);
                let value = e.call(CLAMP_ADD, &args![current, checked, clamp]).f32();
                e.mem.set_f32(at, value);
            }
        }
        _ => {}
    }
}

// Translated from 0094c640 (decompiled, FalloutNV.exe 1.4.0.525)
/// Forwards to `00880660` (`ECX` = the actor, two words) and returns its
/// `float`.
pub fn fn_0094c640(e: &mut Engine, this: Ptr, first: u32, second: u32) -> f32 {
    e.call(MODIFIER_ITEM_GETTER, &args![this, first, second])
        .f32()
}

// Translated from 0094c660 (decompiled, FalloutNV.exe 1.4.0.525)
/// Forwards to `00880690` (`ECX` = the actor, a word and a `float`).
pub fn fn_0094c660(e: &mut Engine, this: Ptr, first: u32, second: f32) {
    e.call(MODIFIER_ITEM_SETTER, &args![this, first, second]);
}

// Translated from 0094c680 (decompiled, FalloutNV.exe 1.4.0.525)
/// `PlayerCharacter::AddSpell` (Xbox PDB): adds the spell through
/// `Actor::AddSpell` and, when that succeeded, runs `SpellLearned`. Returns
/// whether the spell was added.
pub fn player_character_add_spell(e: &mut Engine, this: Ptr<PlayerCharacter>, spell: Ptr) -> bool {
    let added = e.call(ACTOR_ADD_SPELL, &args![this, spell]).bool();
    if added {
        player_character_spell_learned(e, this, spell);
    }
    added
}

// Translated from 0094c6c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `PlayerCharacter::SpellLearned` (Xbox PDB): for a spell whose type (slot
/// `0x18` of its interface at `+0x18`) is 0, 4, 2 or 3, walks the effect list
/// that starts at `+0x28` (a node whose list head is tested with `008256d0`)
/// and, for every entry that has an item, calls `00407e00` on the object
/// `00825c00` gives for that item with `0x200000` and 1. `this` is not used.
pub fn player_character_spell_learned(e: &mut Engine, _this: Ptr<PlayerCharacter>, spell: Ptr) {
    let interface = spell.addr().wrapping_add(0x18);
    let kinds = [0i32, 4, 2, 3];
    let mut matches = false;
    for expected in kinds {
        if e.vcall(interface, 0x18, &args![]).i32() == expected {
            matches = true;
            break;
        }
    }
    if !matches {
        return;
    }
    let mut node = spell.addr();
    while node != 0 {
        let head = node.wrapping_add(0x28);
        if list_is_empty(e, head) {
            break;
        }
        let slot = list_item_slot(e, head);
        let item = e.mem.u32(slot);
        if item != 0 {
            let target = e.call(0x0082_5c00, &args![item]).u32();
            e.call(0x0040_7e00, &args![target, 0x0020_0000u32, 1u32]);
        }
        let next = list_next(e, head);
        node = if next == 0 {
            0
        } else {
            next.wrapping_sub(0x28)
        };
    }
}

// Translated from 0094c7a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Removes the spell through `Actor::RemoveSpell` (`008c3400`); when that
/// succeeded and the spell's interface (`spell + 0x18`, or 0 for no spell) is
/// the selected spell, clears the selection. Returns whether the spell was
/// removed.
pub fn fn_0094c7a0(e: &mut Engine, this: Ptr<PlayerCharacter>, spell: Ptr) -> bool {
    let removed = e.call(ACTOR_REMOVE_SPELL, &args![this, spell]).bool();
    if removed {
        let interface = if spell.is_null() {
            0
        } else {
            spell.addr().wrapping_add(0x18)
        };
        if interface == fn_0094c8c0(e, this) {
            player_character_set_selected_spell(e, this, 0);
        }
    }
    removed
}

// Translated from 0094c800 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls `008c45e0(this, first, second, &code, flag)`; stores the code it
/// reports at `out` when `out` is not null. The result is false when that call
/// failed with code 5; otherwise it is true when
/// `PlayerCharacter::IsGodMode` is, and else the call's own result.
pub fn fn_0094c800(e: &mut Engine, this: Ptr, first: u32, second: u32, out: Ptr, flag: u8) -> bool {
    let code = e.mem.alloc(4);
    e.mem.set_u32(code, 0);
    let ok = e
        .call(SPELL_CHECK, &args![this, first, second, code, flag])
        .bool();
    let reported = e.mem.u32(code);
    e.mem.free(code);
    if !out.is_null() {
        e.mem.set_u32(out.addr(), reported);
    }
    if !ok && reported == 5 {
        return false;
    }
    if e.call(IS_GOD_MODE, &args![]).bool() {
        true
    } else {
        ok
    }
}

// Translated from 0094c870 (decompiled, FalloutNV.exe 1.4.0.525)
/// Forwards to `MagicCaster::CastAbility` (Xbox PDB name of `00815600`).
pub fn fn_0094c870(e: &mut Engine, this: Ptr, ability: u32, flag: u8) {
    e.call(MAGIC_CASTER_CAST_ABILITY, &args![this, ability, flag]);
}

// Translated from 0094c890 (decompiled, FalloutNV.exe 1.4.0.525)
/// Forwards to `MagicCaster::TransferDisease` (Xbox PDB name of `008157a0`).
pub fn fn_0094c890(e: &mut Engine, this: Ptr, first: u32, second: u32, flag: u8) {
    e.call(
        MAGIC_CASTER_TRANSFER_DISEASE,
        &args![this, first, second, flag],
    );
}

// Translated from 0094c8c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The selected spell (`pSelectedSpell`, `+0x6ec`).
pub fn fn_0094c8c0(e: &mut Engine, this: Ptr<PlayerCharacter>) -> u32 {
    e.get(this, PlayerCharacter::pSelectedSpell).addr()
}

// Translated from 0094c8e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `PlayerCharacter::SetSelectedSpell` (Xbox PDB): when the spell changes,
/// unloads the old one (`MagicItem::Unload(1)`), stores the new one, preloads
/// it (`MagicItem::Preload(0)`) and resets the cast sound.
pub fn player_character_set_selected_spell(e: &mut Engine, this: Ptr<PlayerCharacter>, spell: u32) {
    let current = e.get(this, PlayerCharacter::pSelectedSpell);
    if current.addr() == spell {
        return;
    }
    if !current.is_null() {
        e.call(MAGIC_ITEM_UNLOAD, &args![current, 1u32]);
    }
    e.set(this, PlayerCharacter::pSelectedSpell, Ptr::new(spell));
    let selected = e.get(this, PlayerCharacter::pSelectedSpell);
    if !selected.is_null() {
        e.call(MAGIC_ITEM_PRELOAD, &args![selected, 0u32]);
    }
    e.call(RESET_MAGIC_CAST_SOUND, &args![this]);
}

// Translated from 0094c950 (decompiled, FalloutNV.exe 1.4.0.525)
/// Selects a scroll (`pSelectedScroll`, `+0x6f0`). A scroll for which
/// `00726070(scroll + 0x74)` gives nothing clears the selection without
/// touching the selected spell. Otherwise a changed selection stores the
/// scroll and selects the spell `00726070(scroll + 0x74) + 0x18` (or none for
/// no scroll); removing the scroll also clears the selected spell first.
pub fn fn_0094c950(e: &mut Engine, this: Ptr<PlayerCharacter>, scroll: Ptr) {
    if !scroll.is_null() && list_next(e, scroll.addr().wrapping_add(0x74)) == 0 {
        e.set(this, PlayerCharacter::pSelectedScroll, Ptr::NULL);
        return;
    }
    let current = e.get(this, PlayerCharacter::pSelectedScroll);
    if !current.is_null() && current == scroll {
        return;
    }
    if !current.is_null() && scroll.is_null() {
        player_character_set_selected_spell(e, this, 0);
    }
    e.set(this, PlayerCharacter::pSelectedScroll, scroll);
    let stored = e.get(this, PlayerCharacter::pSelectedScroll);
    if stored.is_null() {
        player_character_set_selected_spell(e, this, 0);
    } else {
        let spell = list_next(e, stored.addr().wrapping_add(0x74));
        let interface = if spell == 0 {
            0
        } else {
            spell.wrapping_add(0x18)
        };
        player_character_set_selected_spell(e, this, interface);
    }
}

// Translated from 0094ca20 (decompiled, FalloutNV.exe 1.4.0.525)
/// Forwards to `MagicTarget::AddTarget` (Xbox PDB name of `008230f0`).
pub fn fn_0094ca20(e: &mut Engine, this: Ptr, first: u32, second: u32, third: u32, flag: u8) {
    e.call(
        MAGIC_TARGET_ADD_TARGET,
        &args![this, first, second, third, flag],
    );
}

// Translated from 0094ca50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `this` is the sub-object at `player + 0x88`. Multiplies the value of the
/// player's slot `0x380` (stored as a `double`) by the scale `00567400` gives,
/// rounds the product to a `float`, calls `009a60e0(player_global, product)`
/// (falling back to `00703350()` when that gives 0) and returns slot `0x19c`
/// of the result, or 0 when there is none.
pub fn fn_0094ca50(e: &mut Engine, this: Ptr) -> u32 {
    let player = this.addr().wrapping_sub(0x88);
    let slot_value = e.vcall(player, 0x380, &args![]).f64();
    let scale = e.call(REFERENCE_GET_SCALE, &args![player]).f64();
    let size = (scale * slot_value) as f32;
    let global_player = e.global::<u32>(PLAYER_POINTER);
    let mut found = e
        .call(PLAYER_LOOKUP_BY_SIZE, &args![global_player, size])
        .u32();
    if found == 0 {
        found = e.call(FALLBACK_LOOKUP, &args![]).u32();
    }
    if found != 0 {
        e.vcall(found, 0x19c, &args![]).u32()
    } else {
        0
    }
}

// ---------------------------------------------------------------------
// The parent-cell change handler
// ---------------------------------------------------------------------

/// Stack frame of `0094cae0`.
const FRAME_CORNER_A: u32 = 0x00;
const FRAME_CORNER_B: u32 = 0x08;
const FRAME_CORNER_C: u32 = 0x10;
const FRAME_CORNER_D: u32 = 0x18;
const FRAME_REGION_LIST: u32 = 0x20;
const FRAME_PLAYER_POINT: u32 = 0x28;
const FRAME_REGION_VARIABLE: u32 = 0x30;
const FRAME_ITEM_VARIABLE: u32 = 0x34;
const FRAME_POSITION_COPY: u32 = 0x38;
const FRAME_OUT_BUFFER: u32 = 0x48;
const FRAME_CINFO: u32 = 0x68;
const FRAME_SIZE: u32 = FRAME_CINFO + 0xe0;

/// The region data of a region (`009611e0` gives the data list, `004f35b0`
/// finds the entry of the given kind).
fn region_data(e: &mut Engine, region: u32, kind: u32) -> u32 {
    let list = e.call(0x0096_11e0, &args![region]).u32();
    e.call(0x004f_35b0, &args![list, kind]).u32()
}

/// Whether the region data is the kind that `004f1540` answers true for.
fn region_data_flag(e: &mut Engine, data: u32) -> bool {
    e.call(0x004f_1540, &args![data]).bool()
}

/// The priority byte of region data (`005bb4d0`).
fn region_data_priority(e: &mut Engine, data: u32) -> u8 {
    e.call(0x005b_b4d0, &args![data]).u8()
}

// Translated from 0094cae0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Handles a change of the player's parent cell: hands the new cell to
/// `00930700`, then rebuilds the player's region bookkeeping from the cell
/// (`008d6f30` gives the parent cell the rest of the function works on). Does
/// nothing when `cell` is already the parent cell.
///
/// - Unless `0094dba0` says otherwise, hands the water height of the cell (or
///   the "no water" constant `-FLT_MAX`) to `0062f8f0`.
/// - Calls `00421e40(extra list of the player, 0)` and `00930700(player,
///   cell)`. For no cell it then clears the list at `+0x774`
///   (`CurrentRegionSoundList`), calls `FalloutAudio::UpdateRegionSounds` and
///   stops.
/// - For a cell it stores `005474b0(cell, 0)` in `pLastKnownMusicType`
///   (`+0x7b4`), passes the cell's `004543c0` object on (`0045cd60`,
///   `004893c0`), calls virtual slot `0x1fc` of the player, adds decals
///   (`DecalCaster::Add`) for the values of `00622bb0` and `00622ba0`, and, when
///   `00822510(+0xdec, 0)` says so, builds the camera body: a `bhkSphereShape`
///   (`fn_0094dbb0`), a `bhkRigidBodyCinfo` and a `bhkRigidBody` stored in
///   `spCameraRigidBody`.
/// - It then walks the cell's region list. For every region that applies and
///   has an entry containing the player's position it records the region in
///   the occupied region list (`+0x764`), collects the sounds of the best
///   region data of kind 7 into `+0x774` and `0x011dd380`, sets
///   `bInBorderContainedCell` (`+0x79c`) when all four corners of the cell are
///   inside, updates the occupied region (`+0x760`, through `0093a7a0`) by
///   the flag and priority of the region data of kind 3, and adds border
///   regions to `pBorderRegions` (`+0x7b0`).
/// - It sets `bReturnToLastKnownGoodPosition` (`+0x79d`) when the border check
///   setting is on, the world space has a border region and no border region
///   was found, and the last known good location is another cell, another
///   world space, or too far from the player's position (2048 units).
/// - Finally it hands the music of the occupied region (kind 4 data, slot
///   `0x2c`) to `00705420`, calls `00947d80` and `ResetMapMarkerStructList`,
///   ends the regions that were left (kind 8 data, `004f2480`) and adds the
///   references of the regions entered to the cell (`AddReference`,
///   `004f24e0`).
///
/// C++ exception unwinding is not translated.
pub fn fn_0094cae0(e: &mut Engine, this: Ptr<PlayerCharacter>, cell: Ptr) {
    let player = this.addr();
    let cell = cell.addr();
    if cell == e.call(PARENT_CELL_OF, &args![player]).u32() {
        return;
    }
    if fn_0094dba0(e) == 0 {
        let water = if cell != 0 && e.call(0x0045_18e0, &args![cell]).bool() {
            e.call(0x0054_71e0, &args![cell]).f32()
        } else {
            e.global::<f32>(NO_WATER_HEIGHT)
        };
        e.call(0x0062_f8f0, &args![water]);
    }
    let extra = e.call(EXTRA_LIST_OF, &args![player]).u32();
    e.call(0x0042_1e40, &args![extra, 0u32]);
    e.call(0x0093_0700, &args![player, cell]);
    if cell == 0 {
        e.call(LIST_CLEAR, &args![player + 0x774]);
        e.call(0x0082_d7c0, &args![]);
        return;
    }

    let frame = e.mem.alloc(FRAME_SIZE);
    let list_local = frame + FRAME_REGION_LIST;
    let player_point = frame + FRAME_PLAYER_POINT;
    let region_variable = frame + FRAME_REGION_VARIABLE;
    let item_variable = frame + FRAME_ITEM_VARIABLE;

    if !e.call(0x0042_5fd0, &args![cell]).bool() {
        e.call(0x0093_a7a0, &args![player, 0u32]);
    }
    e.call(0x0057_43f0, &args![]);
    let music = e.call(0x0054_74b0, &args![cell, 0u32]).u32();
    e.mem.set_u32(player + 0x7b4, music);
    let interior = e.call(0x0045_43c0, &args![cell]).u32();
    if interior != 0 {
        let lighting = e.call(0x0045_cd60, &args![interior]).u32();
        e.call(0x0048_93c0, &args![lighting, 1u32]);
    }
    e.vcall(player, 0x1fc, &args![0u32]);

    // Decals for the two values `00622bb0` and `00622ba0` give.
    for getter in [0x0062_2bb0u32, 0x0062_2ba0] {
        let current = e.call(PARENT_CELL_OF, &args![player]).u32();
        if current != 0 && {
            let current = e.call(PARENT_CELL_OF, &args![player]).u32();
            e.call(0x0045_43c0, &args![current]).u32() != 0
        } {
            let value = e.call(getter, &args![]).f32();
            if e.call(0x0062_2a70, &args![value, 0u32]).u32() != 0 {
                let current = e.call(PARENT_CELL_OF, &args![player]).u32();
                let lighting = e.call(0x0045_43c0, &args![current]).u32();
                let value = e.call(getter, &args![]).f32();
                let decal = e.call(0x0062_2a70, &args![value, 1u32]).u32();
                e.call(0x0062_31a0, &args![decal, lighting]);
            }
        }
    }

    // The camera body.
    if e.call(0x0082_2510, &args![player + 0xdec, 0u32]).bool() {
        let block = e.call(0x00aa_13e0, &args![0x14u32]).u32();
        let sphere = if block != 0 {
            fn_0094dbb0(e, Ptr::new(block)).addr()
        } else {
            0
        };
        let radius = e.global::<f32>(CAMERA_SPHERE_RADIUS);
        e.call(0x0056_ea00, &args![sphere, radius]);
        let cinfo = frame + FRAME_CINFO;
        e.call(0x00c8_f510, &args![cinfo]);
        e.call(0x0056_f110, &args![cinfo, sphere]);
        let flags: u32 = 0x0003_0000 | 0x21;
        e.call(0x0056_f0f0, &args![cinfo, flags]);
        let body_block = e.call(0x00aa_13e0, &args![0x1cu32]).u32();
        let body = if body_block != 0 {
            e.call(0x0056_d380, &args![body_block, cinfo]).u32()
        } else {
            0
        };
        e.call(SMART_POINTER_ASSIGN, &args![player + 0xdec, body]);
        let body_ptr = e.call(0x0055_9450, &args![player + 0xdec]).u32();
        e.call(0x0056_1580, &args![body_ptr, 1u32]);
        let body_ptr = e.call(0x0055_9450, &args![player + 0xdec]).u32();
        e.vcall(body_ptr, 0xe4, &args![4u32]);
        let body_ptr = e.call(0x0055_9450, &args![player + 0xdec]).u32();
        let motion = e.call(0x004a_e700, &args![body_ptr]).u32();
        fn_0094db80(e, Ptr::new(motion), 9);
        let out_buffer = frame + FRAME_OUT_BUFFER;
        e.call(LIST_NODE_ITEM_SLOT, &args![out_buffer]);
        let position = e.vcall(player, 0x1f4, &args![]).u32();
        let copy = frame + FRAME_POSITION_COPY;
        for word in 0..3 {
            let value = e.mem.u32(position + 4 * word);
            e.mem.set_u32(copy + 4 * word, value);
        }
        e.call(0x004a_3e00, &args![out_buffer, copy]);
        let body_ptr = e.call(0x0055_9450, &args![player + 0xdec]).u32();
        e.call(0x0056_10f0, &args![body_ptr, copy]);
        e.call(0x0056_d730, &args![cinfo]);
    }
    let body_ptr = e.call(0x0055_9450, &args![player + 0xdec]).u32();
    let current = e.call(PARENT_CELL_OF, &args![player]).u32();
    let current_interior = e.call(0x0045_43c0, &args![current]).u32();
    e.vcall(body_ptr, 0x9c, &args![current_interior]);

    // Whether the player is in a state that keeps the sky as it is.
    let mut flag = e.call(0x0043_7bd0, &args![player]).bool();
    if !flag {
        flag = e.vcall(player, 0x22c, &args![0u32]).bool();
    }
    if !flag {
        flag = e.vcall(player, 0x2e8, &args![]).bool();
    }
    if !flag {
        flag = e.vcall(player, 0x230, &args![]).bool();
    }
    let reset = !flag && !e.call(0x005a_1e50, &args![player]).bool();
    if reset {
        e.vcall(player, 0x494, &args![]);
    }
    let loader = e.global::<u32>(MODEL_LOADER_POINTER);
    e.call(0x0044_6b50, &args![loader, 0u32]);
    let climate = e.call(0x0054_75b0, &args![cell]).u32();
    let sky = e.call(0x0046_dd00, &args![]).u32();
    e.call(0x0063_c8f0, &args![sky, climate, 0u32]);
    let weather = e.call(0x0054_7680, &args![cell]).u32();
    let weather = e.call(0x0050_0940, &args![weather]).u32();
    e.call(0x00b4_f430, &args![weather]);
    let world_space = e.call(0x0054_ddd0, &args![cell]).u32();
    let mut found_border = false;
    e.mem.set_u8(player + 0x79c, 0);

    let border_regions = e.mem.u32(player + 0x7b0);
    if border_regions != 0 {
        e.call(0x005e_03d0, &args![border_regions]);
        let border_regions = e.mem.u32(player + 0x7b0);
        if border_regions != 0 {
            e.vcall(border_regions, 0, &args![1u32]);
        }
        e.mem.set_u32(player + 0x7b0, 0);
    }

    // The four corners of the cell.
    let cell_x = e.call(0x0054_4c30, &args![cell]).i32();
    let cell_y = e.call(0x0054_4c60, &args![cell]).i32();
    let cell_size: f64 = e.global(CELL_SIZE);
    let west = (cell_x << 12) as f32;
    let east = (west as f64 + cell_size) as f32;
    let south = (cell_y << 12) as f32;
    let north = (south as f64 + cell_size) as f32;
    for (corner, x, y) in [
        (FRAME_CORNER_A, west, south),
        (FRAME_CORNER_B, west, north),
        (FRAME_CORNER_C, east, south),
        (FRAME_CORNER_D, east, north),
    ] {
        e.call(0x004f_7070, &args![frame + corner, x, y]);
    }

    e.call(LIST_CLEAR, &args![player + 0x774]);
    e.set_global(CURRENT_REGION_WITH_SOUNDS, 0u32);
    e.call(0x0096_a2d0, &args![list_local]);

    // The regions the player was in.
    let mut node = if player.wrapping_add(0x764) == 0 {
        0
    } else {
        player + 0x768
    };
    while node != 0 {
        let slot = list_item_slot(e, node);
        if e.mem.u32(slot) == 0 {
            break;
        }
        let slot = list_item_slot(e, node);
        let region = e.mem.u32(slot);
        e.mem.set_u32(region_variable, region);
        if region != 0 {
            e.call(0x005a_e3d0, &args![list_local, region_variable]);
        }
        node = list_next(e, node);
    }
    e.call(LIST_CLEAR, &args![player + 0x768]);

    // The regions of the new cell.
    let mut best = 0u32;
    let region_list = e.call(0x0054_7110, &args![cell, 1u32]).u32();
    let mut region_node = if region_list != 0 { region_list + 4 } else { 0 };
    while region_node != 0 {
        let slot = list_item_slot(e, region_node);
        let item = e.mem.u32(slot);
        if item == 0 {
            break;
        }
        e.mem.set_u32(item_variable, item);
        let position = e.call(0x0089_1170, &args![player]).u32();
        e.call(0x004f_7030, &args![player_point, position + 0x10]);

        let skip = e.call(0x0044_0d80, &args![item]).bool() || {
            let form = e.call(0x007a_f430, &args![item]).u32();
            let wrong_world = form != 0 && {
                let again = e.call(0x007a_f430, &args![item]).u32();
                let world = e.call(0x0057_5d70, &args![player]).u32();
                again != world
            };
            wrong_world || {
                let regions = e.call(0x0044_1110, &args![item]).u32();
                regions == 0 || {
                    let regions = e.call(0x0044_1110, &args![item]).u32();
                    list_is_empty(e, regions)
                }
            }
        };
        if skip {
            region_node = list_next(e, region_node);
            continue;
        }

        let mut done = false;
        let mut entry_node = e.call(0x0044_1110, &args![item]).u32();
        while !done && entry_node != 0 {
            let slot = list_item_slot(e, entry_node);
            let entry = e.mem.u32(slot);
            if entry == 0 {
                break;
            }
            if e.call(0x004f_8360, &args![entry, player_point]).bool() {
                e.call(0x004f_6600, &args![player + 0x764, item]);
                if e.call(0x005f_65d0, &args![list_local, item_variable])
                    .bool()
                {
                    e.call(0x0090_5330, &args![list_local, item_variable]);
                }
                let data = region_data(e, item, 7);
                if data != 0 {
                    if region_data_flag(e, data) {
                        let replace = if best == 0 {
                            true
                        } else if best == data {
                            false
                        } else {
                            let old = region_data_priority(e, best);
                            let new = region_data_priority(e, data);
                            old < new
                        };
                        if replace {
                            e.call(LIST_CLEAR, &args![player + 0x774]);
                            best = data;
                        }
                    }
                    if best == 0 || best == data {
                        let mut sound_node = e.call(0x0048_d150, &args![data]).u32();
                        while sound_node != 0 {
                            let slot = list_item_slot(e, sound_node);
                            if e.mem.u32(slot) == 0 {
                                break;
                            }
                            let slot = list_item_slot(e, sound_node);
                            e.call(0x005a_e3d0, &args![player + 0x774, slot]);
                            sound_node = list_next(e, sound_node);
                        }
                        if e.vcall(data, 0x30, &args![]).u32() != 0 {
                            e.set_global(CURRENT_REGION_WITH_SOUNDS, data);
                        }
                    }
                }
                if e.call(0x0054_9580, &args![item]).bool() {
                    found_border = true;
                    let corners = [
                        FRAME_CORNER_A,
                        FRAME_CORNER_B,
                        FRAME_CORNER_C,
                        FRAME_CORNER_D,
                    ];
                    let mut all = true;
                    for corner in corners {
                        if !e.call(0x004f_8360, &args![entry, frame + corner]).bool() {
                            all = false;
                            break;
                        }
                    }
                    if all {
                        e.mem.set_u8(player + 0x79c, 1);
                    }
                }
                let occupied = e.mem.u32(player + 0x760);
                if occupied != 0 {
                    let occupied_data = region_data(e, occupied, 3);
                    let new_data = region_data(e, item, 3);
                    if new_data != 0 {
                        let mut change = false;
                        if occupied_data == 0 {
                            change = true;
                        } else {
                            let new_flag = region_data_flag(e, new_data);
                            if new_flag && !region_data_flag(e, occupied_data) {
                                change = true;
                            } else {
                                let new_flag = region_data_flag(e, new_data);
                                let old_flag = region_data_flag(e, occupied_data);
                                if new_flag == old_flag {
                                    let new_priority = region_data_priority(e, new_data);
                                    let old_priority = region_data_priority(e, occupied_data);
                                    if new_priority > old_priority {
                                        change = true;
                                    }
                                }
                            }
                        }
                        if change {
                            e.call(0x0093_a7a0, &args![player, item]);
                        }
                    }
                } else {
                    e.call(0x0093_a7a0, &args![player, item]);
                }
                done = true;
            }
            entry_node = list_next(e, entry_node);
        }
        if e.call(0x0054_9580, &args![item]).bool() {
            if e.mem.u32(player + 0x7b0) == 0 {
                let block = e.call(OPERATOR_NEW, &args![0x10u32]).u32();
                let array = if block != 0 {
                    e.call(0x0096_a640, &args![block, 1u32, 1u32]).u32()
                } else {
                    0
                };
                e.mem.set_u32(player + 0x7b0, array);
            }
            let array = e.mem.u32(player + 0x7b0);
            e.call(0x0096_a610, &args![array, item_variable]);
        }
        region_node = list_next(e, region_node);
    }

    // The border check.
    let border_enabled = {
        let at = e.call(0x0040_8d60, &args![BORDER_REGION_SETTING]).u32();
        e.mem.u8(at) != 0
    };
    if border_enabled
        && world_space != 0
        && e.call(0x0058_6260, &args![world_space]).bool()
        && !{
            let holder = e.global::<u32>(SKY_HOLDER_POINTER);
            e.call(0x0045_1530, &args![holder]).bool()
        }
        && !found_border
    {
        let mut exterior_cell = 0u32;
        let mut world = 0u32;
        let location = e.mem.u32(player + 0x7ac);
        if location != 0 {
            let kind = e.call(FORM_TYPE_OF, &args![location]).i32();
            if kind == 0x39 {
                exterior_cell = location;
            } else if kind == 0x41 {
                world = location;
            }
        }
        let left_exterior = exterior_cell != 0 && exterior_cell != cell;
        let other_world =
            !left_exterior && world != 0 && world != e.call(0x0054_ddd0, &args![cell]).u32();
        if left_exterior || other_world {
            e.mem.set_u8(player + 0x79d, 1);
        } else {
            let position = e.vcall(player, 0x1f4, &args![]).u32();
            let copy = frame + FRAME_POSITION_COPY;
            for word in 0..3 {
                let value = e.mem.u32(position + 4 * word);
                e.mem.set_u32(copy + 4 * word, value);
            }
            e.call(0x0045_78c0, &args![copy, player + 0x7a0]);
            let distance = e.call(0x0045_7990, &args![copy]).f64();
            let limit: f64 = e.global(BORDER_DISTANCE_LIMIT);
            if distance > limit {
                e.mem.set_u8(player + 0x79d, 1);
            }
        }
    }

    // The music type of the occupied region.
    let occupied = e.mem.u32(player + 0x760);
    let music_data = if occupied != 0 {
        region_data(e, occupied, 4)
    } else {
        0
    };
    let music_value = if music_data != 0 {
        e.vcall(music_data, 0x2c, &args![]).u32()
    } else {
        0
    };
    let pair = e.mem.alloc(8);
    e.call(0x0040_c0e0, &args![pair, music_value]);
    let first = e.mem.u32(pair);
    let second = e.mem.u32(pair + 4);
    e.mem.free(pair);
    let occupied = e.mem.u32(player + 0x760);
    e.call(0x0070_5420, &args![occupied, first, second, 1u32]);
    e.call(0x0094_7d80, &args![player]);
    e.call(0x0094_80c0, &args![player]);

    // The regions that were left.
    let mut node = list_local;
    while node != 0 {
        let slot = list_item_slot(e, node);
        if e.mem.u32(slot) == 0 {
            break;
        }
        let slot = list_item_slot(e, node);
        let region = e.mem.u32(slot);
        if region != 0 {
            let data = region_data(e, region, 8);
            if data != 0 {
                e.call(0x004f_2480, &args![data, 0u32, 0u32]);
            }
        }
        node = list_next(e, node);
    }

    // The regions entered: add their references to the cell.
    let mut node = if player.wrapping_add(0x764) == 0 {
        0
    } else {
        player + 0x768
    };
    while node != 0 {
        let slot = list_item_slot(e, node);
        if e.mem.u32(slot) == 0 {
            break;
        }
        let slot = list_item_slot(e, node);
        let region = e.mem.u32(slot);
        if region != 0 {
            let data = region_data(e, region, 8);
            if data != 0 {
                let mut reference_node = e.call(0x0041_3f40, &args![data]).u32();
                while reference_node != 0 {
                    let slot = list_item_slot(e, reference_node);
                    if e.mem.u32(slot) != 0 {
                        let slot = list_item_slot(e, reference_node);
                        let reference = e.mem.u32(slot);
                        let parent = e.call(PARENT_CELL_OF, &args![player]).u32();
                        e.call(0x0054_8230, &args![parent, reference, 0u32]);
                        let slot = list_item_slot(e, reference_node);
                        let reference = e.mem.u32(slot);
                        e.call(0x004f_24e0, &args![reference, 1u32, 0u32]);
                    }
                    reference_node = list_next(e, reference_node);
                }
            }
        }
        node = list_next(e, node);
    }
    e.call(LIST_CLEAR, &args![list_local]);
    e.call(0x0046_ffb0, &args![list_local]);
    e.mem.free(frame);
}

// Translated from 0094db80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores a byte at `+0x1a` of the object (`0094cae0` calls it with 9 on the
/// object `004ae700` gives for the camera body).
pub fn fn_0094db80(e: &mut Engine, this: Ptr, value: u8) {
    e.mem.set_u8(this.addr().wrapping_add(0x1a), value);
}

// Translated from 0094dba0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The byte at `0x011c7a5a`.
pub fn fn_0094dba0(e: &mut Engine) -> u8 {
    e.global::<u8>(SKIP_WATER_HEIGHT_FLAG)
}

// Translated from 0094dbb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructs a `bhkSphereShape` (its RTTI is the one of the vtable it
/// installs, `0x01030cdc`): the base constructor `0056e690`, the vtable, and
/// the instance counter at `0x01268280` incremented. Returns `this`.
pub fn fn_0094dbb0(e: &mut Engine, this: Ptr) -> Ptr {
    e.call(SPHERE_BASE_CONSTRUCTOR, &args![this]);
    e.mem.set_u32(this.addr(), 0x0103_0cdc);
    let count: u32 = e.global(0x0126_8280);
    e.set_global(0x0126_8280, count.wrapping_add(1));
    this
}

// Translated from 0094dbe0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `PlayerCharacter::ReturnToLastKnownGoodPosition` (Xbox PDB): does nothing
/// without a last known good location (`+0x7ac`). It shows the "sad vault
/// boy" message (`007052f0`), then
/// - with `use_position_functions`: moves the player to that location with
///   `PositionPlayer` (`0093c200`, for a cell) or `PositionPlayerExterior`
///   (`0093cce0`, for a world space), using the position `00430830` gives for
///   it and the stored position (`fn_0094df30`);
/// - otherwise: sets the position directly (the water-height check on the
///   character controller, `SetLocationOnReference`, the controller's
///   `SetPosition`), re-parents the player into the cell found from the
///   location (virtual slot `0x228` of the player, twice), resets the
///   collision objects of both biped bodies and zeroes their velocities.
pub fn player_character_return_to_last_known_good_position(
    e: &mut Engine,
    this: Ptr<PlayerCharacter>,
    use_position_functions: bool,
) {
    let player = this.addr();
    if e.get(this, PlayerCharacter::pLastKnownGoodLocation)
        .is_null()
    {
        return;
    }
    let message = setting_text(e, RETURN_MESSAGE_SETTING);
    let seconds = e.global::<f32>(MESSAGE_SECONDS);
    e.call(
        0x0070_52f0,
        &args![message, 0u32, SAD_ICON_PATH, 0u32, seconds, 0u32],
    );

    let mut cell = 0u32;
    let mut world = 0u32;
    let location = e.get(this, PlayerCharacter::pLastKnownGoodLocation).addr();
    if location != 0 {
        let kind = e.call(FORM_TYPE_OF, &args![location]).i32();
        if kind == 0x39 {
            cell = location;
        } else if kind == 0x41 {
            world = location;
        }
    }

    if use_position_functions {
        let (target, function) = if world != 0 {
            (world, 0x0093_cce0u32)
        } else if cell != 0 {
            (cell, 0x0093_c200u32)
        } else {
            return;
        };
        let position = e.call(0x0043_0830, &args![player]).u32();
        let wanted = [
            e.mem.u32(position),
            e.mem.u32(position + 4),
            e.mem.u32(position + 8),
        ];
        let stored = e.mem.alloc(12);
        let known = fn_0094df30(e, this, Ptr::new(stored));
        let known = [
            e.mem.u32(known.addr()),
            e.mem.u32(known.addr() + 4),
            e.mem.u32(known.addr() + 8),
        ];
        e.mem.free(stored);
        e.call(
            function,
            &args![
                player, known[0], known[1], known[2], wanted[0], wanted[1], wanted[2], target, 0u32
            ],
        );
        return;
    }

    let controller = e.call(0x0093_06d0, &args![player]).u32();
    if controller != 0 && !e.call(0x0088_b0c0, &args![controller]).bool() {
        let reference = e.call(0x0043_6aa0, &args![player]).u32();
        let height = e.mem.u32(reference + 8);
        e.mem.set_u32(player + 0x7a8, height);
    }
    e.call(0x0057_5830, &args![player, player + 0x7a0]);
    if controller != 0 {
        e.call(0x0056_20e0, &args![controller, player + 0x7a0]);
    }
    let mut target_cell = 0u32;
    let mut target_world = 0u32;
    let location = e.get(this, PlayerCharacter::pLastKnownGoodLocation).addr();
    if location != 0 {
        let kind = e.call(FORM_TYPE_OF, &args![location]).i32();
        if kind == 0x39 {
            target_cell = location;
        } else if kind == 0x41 {
            target_world = location;
        }
    }
    if target_world != 0 {
        let x = e.mem.f32(player + 0x7a0);
        let x = e.call(0x0040_6d90, &args![x]).i32() >> 12;
        let y = e.mem.f32(player + 0x7a4);
        let y = e.call(0x0040_6d90, &args![y]).i32() >> 12;
        let source = e.global::<u32>(0x011c_3f2c);
        target_cell = e
            .call(0x0046_1c20, &args![source, x, y, target_world, 0u32])
            .u32();
    }
    if target_cell != 0 {
        e.vcall(player, 0x228, &args![target_cell]);
        e.vcall(player, 0x228, &args![target_cell]);
    }
    let bodies = [
        e.call(0x0095_0bb0, &args![player, 0u32]).u32(),
        e.call(0x0095_0bb0, &args![player, 1u32]).u32(),
    ];
    for body in bodies {
        e.call(0x0044_0460, &args![body, player + 0x7a0]);
    }
    for body in bodies {
        e.call(0x00c6_bd00, &args![body, 1u32]);
    }
    for body in bodies {
        let zero = e.mem.alloc(12);
        e.call(0x0043_d410, &args![zero, 0.0f32, 0u32, 0u32]);
        e.call(0x00a5_9c60, &args![body, zero]);
        e.mem.free(zero);
    }
}

// Translated from 0094df30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Copies the three words at `+0x7a0` (`LastKnownGoodPosition`) into `out` and
/// returns `out`.
pub fn fn_0094df30(e: &mut Engine, this: Ptr<PlayerCharacter>, out: Ptr) -> Ptr {
    let source = this.addr().wrapping_add(0x7a0);
    for word in 0..3 {
        let value = e.mem.u32(source + 4 * word);
        e.mem.set_u32(out.addr() + 4 * word, value);
    }
    out
}

// Translated from 0094df60 (decompiled, FalloutNV.exe 1.4.0.525)
/// `PlayerCharacter::IsSleepingorResting` (Xbox PDB): true while `iSleepTime`
/// (`+0x654`) is positive.
pub fn player_character_is_sleepingor_resting(e: &mut Engine, this: Ptr<PlayerCharacter>) -> bool {
    e.get(this, PlayerCharacter::iSleepTime) > 0
}

// Translated from 0094df80 (decompiled, FalloutNV.exe 1.4.0.525)
/// One hour of waiting or sleeping: ends all sounds of type 4, interrupts the
/// magic cast, advances the system clock and the calendar by
/// `3600 / time scale` seconds, and (while `0x011a3b30` is set) updates the
/// magic target or the process lists, restores health and, for a sleeping
/// player, applies the actor value `0x4b` change; then updates the sky and the
/// process lists and the magic, and counts `iSleepTime` down. When it reaches
/// 0 the wait is over: the flag at `0x011a3b30` is set and a sleeping player
/// is fully restored.
pub fn fn_0094df80(e: &mut Engine, this: Ptr<PlayerCharacter>) {
    let player = this.addr();
    let audio = e.call(0x0045_3a70, &args![]).u32();
    e.call(0x00ad_8780, &args![audio, 4u32]);
    e.call(0x0081_5b00, &args![player + MAGIC_CASTER]);
    let time_scale = e.call(0x0086_7950, &args![CALENDAR]).f64();
    let seconds_per_hour: f64 = e.global(SECONDS_PER_HOUR);
    let step = (seconds_per_hour / time_scale) as f32;
    let clock = e.call(0x0096_d490, &args![PROCESS_LISTS]).f64();
    let new_clock = (clock + step as f64) as f32;
    e.call(0x0096_d4b0, &args![PROCESS_LISTS, new_clock]);
    e.call(0x0086_7a40, &args![CALENDAR, step]);

    if e.global::<u8>(WAIT_STEP_ACTIVE) != 0 {
        if !e.call(0x005c_7870, &args![player]).bool() {
            if e.global::<u8>(PROCESS_LOCK_ENABLED) != 0 {
                e.call(0x0040_fbf0, &args![PROCESS_LOCK, 0u32]);
                let time = e.global::<f32>(WAIT_UPDATE_TIME);
                for function in [0x0096_bcd0u32, 0x0096_b810, 0x0096_b470, 0x0096_b050] {
                    e.call(function, &args![PROCESS_LISTS, time, 1u32]);
                }
                e.call(0x0040_fba0, &args![PROCESS_LOCK]);
            }
        } else {
            e.call(0x0082_3c40, &args![player + MAGIC_TARGET, step]);
        }
        let global_player = e.global::<u32>(PLAYER_POINTER);
        e.call(0x0088_b510, &args![global_player, step]);
        if e.call(0x004d_1360, &args![player]).bool() {
            let global_player = e.global::<u32>(PLAYER_POINTER);
            if e.call(0x005a_1e50, &args![global_player]).bool() {
                let value = e
                    .vcall(player + ACTOR_VALUE_OWNER, 0xc, &args![0x4bu32])
                    .f32();
                let at = e
                    .call(SETTING_FLOAT_POINTER, &args![ACTOR_VALUE_4B_LIMIT_SETTING])
                    .u32();
                let limit = e.mem.f32(at);
                let change = if value < limit { value } else { limit };
                e.vcall(player, 0x3a4, &args![0x4bu32, -change, 0u32]);
            }
        }
    }

    let source = e.call(0x0084_d030, &args![TIMER_SOURCE_011F6394]).f32();
    let holder = e.global::<u32>(SKY_HOLDER_POINTER);
    let sky = e.call(0x008d_8520, &args![holder]).u32();
    e.call(0x0063_ac70, &args![sky, source]);
    e.call(0x0097_23f0, &args![PROCESS_LISTS]);
    e.call(0x008c_3c40, &args![player, 1u32, 0u32]);
    let remaining = e.get(this, PlayerCharacter::iSleepTime).wrapping_sub(1);
    e.set(this, PlayerCharacter::iSleepTime, remaining);
    if remaining <= 0 {
        e.set_global(WAIT_STEP_ACTIVE, 1u8);
        if e.get(this, PlayerCharacter::bIsSleeping) && !e.call(0x004d_1360, &args![player]).bool()
        {
            e.call(0x008a_0960, &args![player]);
        }
        e.set(this, PlayerCharacter::bIsSleeping, false);
    }
}

// Translated from 0094e1d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `PlayerCharacter::Load3D` (Xbox PDB): loads the player's 3D.
///
/// - Opens the debug scope (`00404eb0`, size `0x34`, line `0x2458` of
///   `PlayerCharacter.cpp`) and closes it at the end.
/// - Unless `0042ce10(load state)` or the byte at `0x011d8907` say otherwise,
///   looks for a worn item in the 20 slots of the inventory changes; with
///   none it calls `006047c0` on the base form, and with items but none worn
///   in slot 6 it creates the default item for slot 6 (`004c8220`) and
///   equips it (virtual slot `0x184`).
/// - Loads the model named by the model-path setting (`ModelLoader::LoadFile`),
///   stores it in `sp1stPerson3D`, finds the bone root and the two camera
///   nodes by name (logging an error for a missing one), positions the model,
///   builds the `BipedAnim` and `Animation` objects (stored at `+0x68c` and
///   `+0x690`), runs the base `0087e060` with `b3rdPerson` set, clears the
///   animation group, builds the animation file list (the model's `.kf` list
///   plus the locomotion idle animations for toddlers and for the sex of the
///   player), starts the animation, then updates the third person camera
///   node, process, lighting and character-controller proxy and finally
///   updates the biped bodies. Returns the node `0087e060` returned.
///
/// C++ exception unwinding and the stack cookie are not translated.
pub fn player_character_load_3d(e: &mut Engine, this: Ptr<PlayerCharacter>, flag: u8) -> u32 {
    let player = this.addr();
    let scope = e.mem.alloc(0x34);
    e.call(
        0x0040_4eb0,
        &args![scope, 0x34u32, 1u32, SOURCE_FILE_NAME, 0x2458u32],
    );
    let form = e.call(0x007a_f430, &args![player]).u32();
    let load_state = e.global::<u32>(LOAD_STATE_POINTER);
    let skip = e.call(0x0042_ce10, &args![load_state]).bool() || e.global::<u8>(0x011d_8907) != 0;
    if !skip {
        let mut nothing_worn = true;
        let inventory = e.call(0x004b_f220, &args![player]).u32();
        if inventory != 0 {
            for slot in 0..0x14u32 {
                let worn = e.call(0x004c_8c10, &args![inventory, slot, 0u32]).u32();
                if worn != 0 {
                    nothing_worn = false;
                    e.call(0x0044_59e0, &args![worn, 1u32]);
                    break;
                }
            }
        }
        if nothing_worn {
            e.call(0x0060_47c0, &args![form, player, 1u32, 1u32, 0u32, 0u32]);
        } else if inventory != 0 {
            let worn = e.call(0x004c_8c10, &args![inventory, 6u32, 0u32]).u32();
            if worn == 0 {
                let base = e.call(BASE_FORM_OF, &args![player]).u32();
                let item = e
                    .call(0x004c_8220, &args![inventory, base, 6u32, 1u32])
                    .u32();
                if item != 0 {
                    let name = e.call(WORD_AT_8, &args![item]).u32();
                    let mut extra = 0;
                    if e.call(0x0055_9450, &args![item]).u32() != 0 {
                        let pointer = e.call(0x0055_9450, &args![item]).u32();
                        let slot = list_item_slot(e, pointer);
                        extra = e.mem.u32(slot);
                    }
                    e.vcall(player, 0x184, &args![name, 1u32, extra, 0u32]);
                    e.call(0x0044_59e0, &args![item, 1u32]);
                }
            } else {
                e.call(0x0044_59e0, &args![worn, 1u32]);
            }
        }
    }

    let path = setting_text(e, MODEL_PATH_SETTING);
    let path = e.call(IDENTITY, &args![path, 0u32]).u32();
    let loader = e.global::<u32>(MODEL_LOADER_POINTER);
    let model = e
        .call(
            0x0044_7080,
            &args![loader, path, 3u32, 1u32, 0u32, 0u32, 0u32],
        )
        .u32();
    e.call(0x0054_68b0, &args![model, 1u32]);
    e.call(SMART_POINTER_ASSIGN, &args![player + 0x694, model]);
    for (name, slot) in [
        (fn_0094ead0(e), NODE_BY_EAD0_SLOT),
        (fn_0094eae0(e), NODE_CAMERA_1ST_SLOT),
        (fn_0094eac0(e), NODE_BIP_SLOT),
    ] {
        let node = e.call(0x0055_9450, &args![player + 0x694]).u32();
        let found = e.vcall(node, 0x9c, &args![name]).u32();
        e.set_global(slot, found);
    }
    if e.global::<u32>(NODE_CAMERA_1ST_SLOT) == 0 {
        let text = setting_text(e, MODEL_PATH_SETTING);
        e.call(ERROR, &args![FORMAT_MISSING_CAMERA_1ST, text]);
    }
    if e.global::<u32>(NODE_BIP_SLOT) == 0 {
        let text = setting_text(e, MODEL_PATH_SETTING);
        e.call(ERROR, &args![FORMAT_MISSING_BIP, text]);
    }
    let zero = e.mem.alloc(12);
    e.call(0x0043_d410, &args![zero, 0.0f32, 0u32, 0u32]);
    e.call(0x00a5_9c60, &args![model, zero]);
    e.mem.free(zero);
    let camera = e.global::<u32>(NODE_CAMERA_1ST_SLOT);
    let at = e.call(0x0045_bb80, &args![camera]).u32();
    let height = e.mem.f32(at + 8);
    e.set(this, PlayerCharacter::fEyeHeight, height);

    let matrix_copy = e.mem.alloc(0x24);
    let matrix_result = e.mem.alloc(0x24);
    for word in 0..9u32 {
        let value = e.mem.u32(LOAD_3D_MATRIX + 4 * word);
        e.mem.set_u32(matrix_copy + 4 * word, value);
    }
    let rotated = e
        .call(0x0056_fac0, &args![player, matrix_result, matrix_copy])
        .u32();
    e.call(0x0043_fa80, &args![model, rotated]);
    e.mem.free(matrix_result);
    e.mem.free(matrix_copy);

    let block = e.call(OPERATOR_NEW, &args![0x2b4u32]).u32();
    let biped = if block != 0 {
        e.call(0x004a_aca0, &args![block, player, model]).u32()
    } else {
        0
    };
    e.set(this, PlayerCharacter::p1stPersonBipedAnim, Ptr::new(biped));
    let block = e.call(OPERATOR_NEW, &args![0x13cu32]).u32();
    let animation = if block != 0 {
        e.call(0x0048_f810, &args![block]).u32()
    } else {
        0
    };
    e.set(
        this,
        PlayerCharacter::p1stPersonAnimation,
        Ptr::new(animation),
    );

    let third_person = e.get(this, PlayerCharacter::b3rdPerson);
    e.set(this, PlayerCharacter::b3rdPerson, true);
    let node = e.call(0x0087_e060, &args![player, flag]).u32();
    e.set(this, PlayerCharacter::b3rdPerson, third_person);

    let current = e.call(0x0095_0a60, &args![player, 0u32]).u32();
    e.call(0x0049_6080, &args![current, 0u32, 0.0f32]);

    let path = setting_text(e, MODEL_PATH_SETTING);
    let path = e.call(IDENTITY, &args![path, 0u32]).u32();
    let loader = e.global::<u32>(MODEL_LOADER_POINTER);
    let list = e
        .call(0x0044_7330, &args![loader, path, 1u32, 0u32, 0xcu32])
        .u32();
    let path = setting_text(e, MODEL_PATH_SETTING);
    let path = e.call(IDENTITY, &args![path, 0u32]).u32();
    let buffer = e.mem.alloc(0x104);
    e.call(0x0040_6d30, &args![buffer, 0x104u32, path]);
    let last_separator = e.call(0x0040_ab30, &args![buffer, 0x5cu32]).u32();
    let room = 0x103u32.wrapping_sub(last_separator.wrapping_sub(buffer));
    let idle_manager = e.global::<u32>(IDLE_MANAGER_POINTER);
    if e.call(0x0083_97d0, &args![player]).bool() {
        e.call(0x0040_6d30, &args![last_separator, room, TODDLER_SUFFIX]);
        let root = e
            .call(0x0060_0700, &args![idle_manager, buffer, 0u32])
            .u32();
        let loader = e.global::<u32>(MODEL_LOADER_POINTER);
        e.call(0x0044_7850, &args![loader, root, list]);
    }
    let suffix = if e.call(0x0087_f4c0, &args![player]).i32() == 1 {
        FEMALE_SUFFIX
    } else {
        MALE_SUFFIX
    };
    e.call(0x0040_6d30, &args![last_separator, room, suffix]);
    let root = e
        .call(0x0060_0700, &args![idle_manager, buffer, 0u32])
        .u32();
    let loader = e.global::<u32>(MODEL_LOADER_POINTER);
    e.call(0x0044_7850, &args![loader, root, list]);
    e.mem.free(buffer);

    let animation = e.get(this, PlayerCharacter::p1stPersonAnimation);
    e.call(0x0048_ffd0, &args![animation, list, model, player, 1u32]);
    let first = e.call(0x008d_8520, &args![player]).u32();
    let second = e.call(0x008d_8520, &args![player]).u32();
    let result = e.vcall(first, 0x148, &args![0u32, 0u32]).u32();
    e.vcall(second, 0x160, &args![result]);
    fn_0094eaf0(e, this.cast(), Ptr::new(node), true);
    if e.global::<u32>(NODE_CAMERA_3RD_SLOT) == 0 {
        let name = e.call(0x0057_15d0, &args![player]).u32();
        e.call(ERROR, &args![FORMAT_MISSING_CAMERA_3RD, name]);
    }
    e.call(0x008c_2e50, &args![player]);
    let process = process_of(e, this);
    e.vcall(process, 0x470, &args![]);
    e.call(0x008b_0bd0, &args![player, model]);
    e.vcall(player, 0x1c4, &args![]);
    let controller = e.call(0x0093_06d0, &args![player]).u32();
    let proxy = if controller != 0 {
        e.call(0x0062_1ad0, &args![controller]).u32()
    } else {
        0
    };
    if proxy != 0 {
        let out = e.mem.alloc(4);
        let built = e.call(0x0070_c440, &args![controller, out]).u32();
        let value = e.call(0x004a_3a20, &args![built]).u32();
        e.mem.free(out);
        e.vcall(proxy, 0xd0, &args![node, 1u32, 0u32, value, 0u32]);
    }
    e.call(0x0070_59d0, &args![]);
    if e.get(this, PlayerCharacter::bIsToddler) {
        e.set(this, PlayerCharacter::bIsToddler, false);
        e.call(0x0096_97c0, &args![player, 1u32]);
    }
    for which in 0..2u32 {
        let biped = e.call(0x0095_0b00, &args![player, which]).u32();
        let pointer = e.call(0x0055_9450, &args![biped]).u32();
        e.call(0x0093_80e0, &args![pointer]);
    }
    e.call(0x0040_4ee0, &args![scope]);
    e.mem.free(scope);
    node
}

// Translated from 0094eac0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The word at `0x011c61a8`.
pub fn fn_0094eac0(e: &mut Engine) -> u32 {
    e.global::<u32>(NODE_NAME_EAC0)
}

// Translated from 0094ead0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The word at `0x011c6264`.
pub fn fn_0094ead0(e: &mut Engine) -> u32 {
    e.global::<u32>(NODE_NAME_EAD0)
}

// Translated from 0094eae0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The word at `0x011c626c`.
pub fn fn_0094eae0(e: &mut Engine) -> u32 {
    e.global::<u32>(NODE_NAME_EAE0)
}

// Translated from 0094eaf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Looks up the third person camera node: with `flag`, first calls `00450f90`
/// on `object` (with 1); then stores `object`'s slot `0x9c` with the word of
/// `0094eb30` in the global `0x011e07d4`. `this` is not used.
pub fn fn_0094eaf0(e: &mut Engine, _this: Ptr, object: Ptr, flag: bool) {
    if flag {
        e.call(0x0045_0f90, &args![object, 1u32]);
    }
    let name = fn_0094eb30(e);
    let found = e.vcall(object.addr(), 0x9c, &args![name]).u32();
    e.set_global(NODE_CAMERA_3RD_SLOT, found);
}

// Translated from 0094eb30 (decompiled, FalloutNV.exe 1.4.0.525)
/// The word at `0x011c6270`.
pub fn fn_0094eb30(e: &mut Engine) -> u32 {
    e.global::<u32>(NODE_NAME_EB30)
}

// Translated from 0094eb40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls `TESObjectREFR::Set3D` (Xbox PDB name of `005702e0`) with the node and
/// the flag. Setting no node while the game state object (`0x011dea0c`) exists
/// and both tests `005dc960` and `00678ce0` fail is refused with a log line.
/// Setting no node otherwise releases the 1st-person `BipedAnim` and
/// `Animation` (`+0x68c`, `+0x690`), clears `sp1stPerson3D`, the three camera
/// node globals and the `0x60` bytes at `+0xd74`, and tells the model loader.
pub fn fn_0094eb40(e: &mut Engine, this: Ptr<PlayerCharacter>, node: Ptr, flag: u8) {
    let player = this.addr();
    let state = e.global::<u32>(GAME_STATE_POINTER);
    if state != 0
        && !e.call(GAME_STATE_TEST_1, &args![state]).bool()
        && !e.call(GAME_STATE_TEST_2, &args![state]).bool()
        && node.is_null()
    {
        e.call(LOG_MESSAGE, &args![FORMAT_SET3D_MESSAGE]);
        return;
    }
    e.call(REFERENCE_SET_3D, &args![player, node, flag]);
    if !node.is_null() {
        return;
    }
    let biped = e.get(this, PlayerCharacter::p1stPersonBipedAnim);
    if !biped.is_null() {
        e.call(DELETE_BIPED_ANIM, &args![biped, 1u32]);
    }
    e.set(this, PlayerCharacter::p1stPersonBipedAnim, Ptr::NULL);
    let animation = e.get(this, PlayerCharacter::p1stPersonAnimation);
    if !animation.is_null() {
        e.call(DELETE_ANIMATION, &args![animation, 1u32]);
    }
    e.set(this, PlayerCharacter::p1stPersonAnimation, Ptr::NULL);
    e.call(SMART_POINTER_ASSIGN, &args![player + 0x694, 0u32]);
    e.set_global(NODE_CAMERA_1ST_SLOT, 0u32);
    e.set_global(NODE_CAMERA_3RD_SLOT, 0u32);
    e.set_global(NODE_BIP_SLOT, 0u32);
    e.call(MEMSET, &args![player + 0xd74, 0u32, 0x60u32]);
    let path = setting_text(e, MODEL_PATH_SETTING);
    let path = e.call(IDENTITY, &args![path, 0u32]).u32();
    let loader = e.global::<u32>(MODEL_LOADER_POINTER);
    e.call(MODEL_LOADER_NOTIFY, &args![loader, path]);
}

// Translated from 0094ec90 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BuildInvStringCallback` (Xbox PDB): appends `"<prefix><count> <name>
/// (<form id>),"` to the string `output` for an inventory entry; the prefix
/// is `"[E]"` for a worn entry. Does nothing for a null entry or string.
/// Always returns false. The stack-cookie check is not translated.
pub fn build_inv_string_callback(e: &mut Engine, item: Ptr, output: Ptr) -> bool {
    if !output.is_null() && !item.is_null() {
        let worn = e.call(ITEM_CHANGE_GET_WORN, &args![item, 0u32]).bool();
        let prefix = if worn {
            INVENTORY_PREFIX_WORN
        } else {
            INVENTORY_PREFIX_CARRIED
        };
        let form = e.call(WORD_AT_8, &args![item]).u32();
        let id = form_id_of(e, form);
        let name = e.call(ITEM_CHANGE_GET_FULL_NAME, &args![item]).u32();
        let count = list_next(e, item.addr());
        let buffer = e.mem.alloc(0x104);
        e.call(
            SPRINTF,
            &args![
                buffer,
                0x104u32,
                FORMAT_INVENTORY_ITEM,
                prefix,
                count,
                name,
                id
            ],
        );
        e.call(STRING_APPEND, &args![output, buffer]);
        e.mem.free(buffer);
    }
    false
}

// ---------------------------------------------------------------------
// ExportProgressData
// ---------------------------------------------------------------------

/// `BSStringT::operator=`-style formatter `00406f60` (`cdecl`, variadic):
/// formats into the string `text`.
fn format_into(e: &mut Engine, text: u32, rest: &[u32]) {
    let mut words = vec![text];
    words.extend_from_slice(rest);
    e.call(0x0040_6f60, &words);
}

/// Writes the string to the file (virtual slot `0x38` of the file with the
/// string and the flag 1).
fn write_text(e: &mut Engine, file: u32, text: u32) {
    e.vcall(file, 0x38, &args![text, 1u32]);
}

/// Formats `rest` into `text` and writes it to the file.
fn format_and_write(e: &mut Engine, file: u32, text: u32, rest: &[u32]) {
    format_into(e, text, rest);
    write_text(e, file, text);
}

/// The text of a name (`00408da0`, `ECX` = form + `0x18`).
fn name_of_form(e: &mut Engine, form: u32) -> u32 {
    e.call(0x0040_8da0, &args![form.wrapping_add(0x18)]).u32()
}

/// Resets the string to `"` (a quote), `004037f0`.
fn reset_to_quote(e: &mut Engine, text: u32) {
    e.call(0x0040_37f0, &args![text, EMPTY_QUOTE_TEXT, 0u32]);
}

// Translated from 0094ed40 (decompiled, FalloutNV.exe 1.4.0.525)
/// `PlayerCharacter::ExportProgressData` (Xbox PDB): writes a tab-separated
/// progress line to `"<export directory>ProgressData_1.txt"`: build name,
/// version, calendar value, the reason (indexed by `reason`), the player's
/// name and location, sex, level, karma, perks, skills and attributes
/// (base and current), inventory, weight, reputation values, quest log,
/// the 43 counters of `005a3370`, the cell, and, for reason 1 (death), the
/// cause, killer and weapon data. Returns nothing; does nothing when the file
/// cannot be opened. C++ exception unwinding and the stack cookie are not
/// translated.
pub fn player_character_export_progress_data(
    e: &mut Engine,
    this: Ptr<PlayerCharacter>,
    reason: i32,
) {
    let player = this.addr();
    let directory = setting_text(e, EXPORT_DIRECTORY_SETTING);
    let path = e.mem.alloc(0x104);
    e.call(
        SPRINTF,
        &args![path, 0x104u32, FORMAT_EXPORT_PATH, directory],
    );
    let block = e.call(OPERATOR_NEW, &args![0x158u32]).u32();
    let file = if block != 0 {
        e.call(0x00b0_0260, &args![block, path, 1u32, 0x4000u32, 0u32])
            .u32()
    } else {
        0
    };
    e.mem.free(path);
    if file == 0 || !e.call(0x0089_05f0, &args![file]).bool() {
        return;
    }
    let seek = e.global::<u32>(EXPORT_SEEK_MODE);
    e.vcall(file, 0x14, &args![0u32, seek]);
    let text = e.mem.alloc(8);
    e.call(0x0040_37b0, &args![text]);
    let _form = e.call(0x007a_f430, &args![player]).u32();

    let owner = player + ACTOR_VALUE_OWNER;
    if e.mem.i8(BUILD_NAME_BUFFER) != 0 {
        format_and_write(e, file, text, &args![FORMAT_TEXT, BUILD_NAME_BUFFER]);
    } else {
        format_and_write(e, file, text, &args![FORMAT_TEXT, UNKNOWN_TEXT]);
    }
    format_and_write(e, file, text, &args![FORMAT_TEXT, GAME_VERSION_TEXT]);
    let calendar_value = e.call(0x0086_7e30, &args![CALENDAR]).u32();
    format_and_write(e, file, text, &args![FORMAT_INT, calendar_value]);
    let reason_name = e
        .mem
        .u32(REASON_NAME_TABLE.wrapping_add((reason as u32).wrapping_mul(4)));
    format_and_write(e, file, text, &args![FORMAT_TEXT, reason_name]);
    let name = e.call(0x0055_d520, &args![player]).u32();
    format_and_write(e, file, text, &args![FORMAT_TEXT, name]);
    let location = e.vcall(player, 0x37c, &args![]).u32();
    let location_name = name_of_form(e, location);
    format_and_write(e, file, text, &args![FORMAT_TEXT, location_name]);
    let sex = e.call(0x0087_f4c0, &args![player]).u32();
    let sex_name = e.mem.u32(SEX_NAME_TABLE.wrapping_add(sex.wrapping_mul(4)));
    format_and_write(e, file, text, &args![FORMAT_TEXT, sex_name]);
    let level = e.call(0x0087_f9f0, &args![player]).u16() as u32;
    format_and_write(e, file, text, &args![FORMAT_INT, level]);
    let experience = e.vcall(owner, 0x8, &args![0x18u32]).u32();
    format_and_write(e, file, text, &args![FORMAT_INT, experience]);

    // The perks, as `"name (id)-rank,name (id)-rank,"`.
    reset_to_quote(e, text);
    let mut node = player + 0x87c;
    while node != 0 && !list_is_empty(e, node) {
        let slot = list_item_slot(e, node);
        let entry = e.mem.u32(slot);
        node = list_next(e, node);
        if e.mem.u32(entry) != 0 {
            let rank = e.mem.u8(entry + 4) as u32;
            let perk = e.mem.u32(entry);
            let id = form_id_of(e, perk);
            let perk_name = name_of_form(e, perk);
            let buffer = e.mem.alloc(0x104);
            e.call(
                SPRINTF,
                &args![buffer, 0x104u32, FORMAT_PERK, perk_name, id, rank],
            );
            e.call(STRING_APPEND, &args![text, buffer]);
            e.mem.free(buffer);
        }
    }
    e.call(STRING_APPEND, &args![text, TEXT_CLOSE_QUOTE]);
    write_text(e, file, text);

    for index in 0..0xeu32 {
        let value = e.vcall(owner, 0x8, &args![index + 0x20]).u32();
        format_and_write(e, file, text, &args![FORMAT_INT, value]);
    }
    for index in 0..7u32 {
        let value = e.vcall(owner, 0x0, &args![index + 5]).u32();
        format_and_write(e, file, text, &args![FORMAT_INT, value]);
        let value = e.vcall(owner, 0x8, &args![index + 5]).u32();
        format_and_write(e, file, text, &args![FORMAT_INT, value]);
    }
    for index in 0..0x14u32 {
        let value = e.vcall(owner, 0x0, &args![index + 0xc]).u32();
        format_and_write(e, file, text, &args![FORMAT_INT, value]);
        let value = e.vcall(owner, 0x8, &args![index + 0xc]).u32();
        format_and_write(e, file, text, &args![FORMAT_INT, value]);
    }

    // The inventory.
    let inventory = e.call(0x004b_f220, &args![player]).u32();
    if inventory != 0 {
        reset_to_quote(e, text);
        e.call(0x004d_4530, &args![inventory, 0x0094_ec90u32, text, 0u32]);
        e.call(STRING_APPEND, &args![text, TEXT_CLOSE_QUOTE]);
    } else {
        format_into(e, text, &args![TEXT_EMPTY_QUOTES]);
    }
    write_text(e, file, text);

    let pair = e.mem.alloc(8);
    e.call(0x0093_abe0, &args![player, pair, pair + 4]);
    let first = e.mem.f32(pair) as f64;
    let second = e.mem.f32(pair + 4) as f64;
    e.mem.free(pair);
    format_and_write(e, file, text, &args![FORMAT_TWO_FLOATS, first, second]);
    let weight = if inventory != 0 {
        e.call(0x004d_0f40, &args![inventory, 0u32, 0u32]).u32()
    } else {
        0
    };
    format_and_write(e, file, text, &args![FORMAT_INT, weight]);
    let value = e.call(0x0057_7d50, &args![player]).u32();
    format_and_write(e, file, text, &args![FORMAT_INT, value]);
    let speed = e.vcall(player, 0x43c, &args![]).f64();
    format_and_write(e, file, text, &args![FORMAT_ONE_FLOAT, speed]);

    // The quest log.
    reset_to_quote(e, text);
    let mut node = player + 0x6b0;
    while node != 0 && !list_is_empty(e, node) {
        let slot = list_item_slot(e, node);
        let item = e.mem.u32(slot);
        node = list_next(e, node);
        if e.call(0x0060_fd70, &args![item]).bool() && e.call(0x005e_3fa0, &args![item]).u32() != 0
        {
            let quest = e.call(0x005e_3fa0, &args![item]).u32();
            let quest_again = e.call(0x005e_3fa0, &args![item]).u32();
            let id = form_id_of(e, quest_again);
            let quest_name = e.vcall(quest, 0x138, &args![]).u32();
            let buffer = e.mem.alloc(0x104);
            e.call(
                SPRINTF,
                &args![buffer, 0x104u32, FORMAT_QUEST, quest_name, id],
            );
            e.call(STRING_APPEND, &args![text, buffer]);
            e.mem.free(buffer);
        }
    }
    e.call(STRING_APPEND, &args![text, TEXT_CLOSE_QUOTE]);
    write_text(e, file, text);

    for index in 0..0x2bu32 {
        let value = e.call(0x005a_3370, &args![index]).u32();
        format_and_write(e, file, text, &args![FORMAT_INT, value]);
    }

    // The cell.
    let cell = e.call(PARENT_CELL_OF, &args![player]).u32();
    if cell != 0 {
        if e.call(0x0042_5fd0, &args![cell]).bool() {
            let id = form_id_of(e, cell);
            let cell_name = name_of_form(e, cell);
            format_into(e, text, &args![FORMAT_NAME_ID, cell_name, id]);
        } else {
            let world = e.call(0x0054_ddd0, &args![cell]).u32();
            if world != 0 {
                let id = form_id_of(e, cell);
                let y = e.call(0x0054_4c60, &args![cell]).u32();
                let x = e.call(0x0054_4c30, &args![cell]).u32();
                let world_name = name_of_form(e, world);
                format_into(e, text, &args![FORMAT_CELL_WORLD, world_name, x, y, id]);
            } else {
                let id = form_id_of(e, cell);
                let y = e.call(0x0054_4c60, &args![cell]).u32();
                let x = e.call(0x0054_4c30, &args![cell]).u32();
                format_into(e, text, &args![FORMAT_CELL_UNKNOWN_WORLD, x, y, id]);
            }
        }
    } else {
        format_into(e, text, &args![TEXT_NONE]);
    }
    write_text(e, file, text);

    if reason == 1 {
        let extra_list = e.call(EXTRA_LIST_OF, &args![player]).u32();
        let extra = e.call(GET_EXTRA_DATA, &args![extra_list, 0x5fu32]).u32();
        if extra != 0 {
            let cause = e.mem.i32(extra + 0x10);
            if cause != -1 {
                let cause_name = e
                    .mem
                    .u32(CAUSE_NAME_TABLE.wrapping_add((cause as u32).wrapping_mul(4)));
                format_into(e, text, &args![FORMAT_TEXT, cause_name]);
            } else {
                format_into(e, text, &args![TEXT_NONE]);
            }
            write_text(e, file, text);
            let object = e.mem.u32(extra + 0x14);
            if object != 0 {
                let id = form_id_of(e, object);
                let object_name = e.vcall(object, 0x130, &args![]).u32();
                let object_text = e.call(0x0044_0e30, &args![object]).u32();
                format_into(
                    e,
                    text,
                    &args![FORMAT_CAUSE_OBJECT, object_text, object_name, id],
                );
            } else {
                format_into(e, text, &args![TEXT_NONE]);
            }
        } else {
            format_and_write(e, file, text, &args![TEXT_UNKNOWN_CAUSE]);
            format_and_write(e, file, text, &args![TEXT_UNKNOWN_OBJECT]);
        }

        let killer = e.get(this, PlayerCharacter::pMyKiller).addr();
        if killer != 0 {
            let id = form_id_of(e, killer);
            let killer_name = e.call(0x0055_d520, &args![killer]).u32();
            format_and_write(e, file, text, &args![FORMAT_NAME_ID, killer_name, id]);
            if e.call(0x0056_af40, &args![killer]).bool() {
                let extra_list = e.call(EXTRA_LIST_OF, &args![killer]).u32();
                let original = e.call(0x0042_16f0, &args![extra_list]).u32();
                let extra_list = e.call(EXTRA_LIST_OF, &args![killer]).u32();
                let template = e.call(0x0042_1720, &args![extra_list]).u32();
                if original != 0 && template != 0 {
                    let template_id = form_id_of(e, template);
                    let template_name = e.vcall(template, 0x130, &args![]).u32();
                    let original_id = form_id_of(e, original);
                    let original_name = e.vcall(original, 0x130, &args![]).u32();
                    format_into(
                        e,
                        text,
                        &args![
                            FORMAT_LEVELLED_BOTH,
                            original_name,
                            original_id,
                            template_name,
                            template_id
                        ],
                    );
                } else if original != 0 {
                    let original_id = form_id_of(e, original);
                    let original_name = e.vcall(original, 0x130, &args![]).u32();
                    format_into(
                        e,
                        text,
                        &args![FORMAT_LEVELLED_ORIGINAL_ONLY, original_name, original_id],
                    );
                } else if template != 0 {
                    let template_id = form_id_of(e, template);
                    let template_name = e.vcall(template, 0x130, &args![]).u32();
                    format_into(
                        e,
                        text,
                        &args![FORMAT_LEVELLED_TEMPLATE_ONLY, template_name, template_id],
                    );
                } else {
                    format_into(e, text, &args![TEXT_UNKNOWN_LEVELLED]);
                }
            } else {
                let base = e.call(BASE_FORM_OF, &args![killer]).u32();
                let base_again = e.call(BASE_FORM_OF, &args![killer]).u32();
                let id = form_id_of(e, base_again);
                let base_name = e.vcall(base, 0x130, &args![]).u32();
                format_into(e, text, &args![FORMAT_NAME_ID, base_name, id]);
            }
            write_text(e, file, text);
            let damage = e.call(0x0056_8ad0, &args![killer]).f64();
            format_and_write(e, file, text, &args![FORMAT_ONE_FLOAT, damage]);
            let weapon = e.call(0x008a_1710, &args![killer]).u32();
            if weapon != 0 {
                let id = form_id_of(e, weapon);
                let weapon_name = e.call(0x0040_8da0, &args![weapon + 0x30]).u32();
                format_into(e, text, &args![FORMAT_NAME_ID, weapon_name, id]);
            } else {
                format_into(e, text, &args![TEXT_NO_WEAPON]);
            }
            write_text(e, file, text);
        } else {
            format_and_write(e, file, text, &args![TEXT_UNKNOWN_KILLER]);
        }
    }

    format_and_write(e, file, text, &args![TEXT_LINE_END]);
    e.call(0x00af_fd10, &args![file]);
    e.vcall(file, 0, &args![1u32]);
    e.call(0x0040_37d0, &args![text]);
    e.mem.free(text);
}

// Translated from 0094fce0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Forwards to `ProcessLists::PrintLists` (Xbox PDB name of `008d0600`) on the
/// sub-object at `+0x878` (`ECX` = the player's `CharacterProgression`... the
/// object at `+0x878`).
pub fn fn_0094fce0(e: &mut Engine, this: Ptr<PlayerCharacter>, first: u32, second: u32) {
    e.call(
        PROCESS_LISTS_PRINT_LISTS,
        &args![
            this.addr().wrapping_add(CHARACTER_PROGRESSION),
            first,
            second
        ],
    );
}

// Translated from 0094fd10 (decompiled, FalloutNV.exe 1.4.0.525)
/// Forwards to `CharacterProgression::RewardExperience` (Xbox PDB name of
/// `008d5100`) on the `CharacterProgression` at `+0x878`.
pub fn fn_0094fd10(e: &mut Engine, this: Ptr<PlayerCharacter>, amount: u32) {
    e.call(
        CHARACTER_PROGRESSION_REWARD_EXPERIENCE,
        &args![this.addr().wrapping_add(CHARACTER_PROGRESSION), amount],
    );
}

// Translated from 0094fd30 (decompiled, FalloutNV.exe 1.4.0.525)
/// `PlayerCharacter::RewardKarma` (Xbox PDB): changes the karma (actor value
/// `0x17`) by `delta`. The change is first limited so that the new karma stays
/// within the lower and upper limit settings, a message (a big one when the
/// change reaches the message threshold setting) is shown and the sound
/// `UIKarmaUp` or `UIKarmaDown` is played; the actor value is changed (virtual
/// slot `0x3a8` of the player) unless the karma already sits at the limit in
/// the direction of the change.
pub fn player_character_reward_karma(e: &mut Engine, this: Ptr<PlayerCharacter>, delta: i32) {
    let player = this.addr();
    let owner = player + ACTOR_VALUE_OWNER;
    let karma = e.vcall(owner, 0x8, &args![0x17u32]).i32();
    e.call(BASE_FORM_OF, &args![player]);
    let karma_value = e.vcall(owner, 0xc, &args![0x17u32]).f32();
    e.call(0x0047_e040, &args![karma_value]);
    let mut delta = delta;

    let show = |e: &mut Engine, icon: u32, text: u32| {
        let icon = setting_text(e, icon);
        let text = setting_text(e, text);
        let seconds = e.global::<f32>(MESSAGE_SECONDS);
        e.call(0x0070_52f0, &args![text, 0u32, icon, 0u32, seconds, 0u32]);
    };

    let sound = if delta < 0 {
        let lower = setting_int(e, KARMA_LOWER_LIMIT_SETTING);
        if karma.wrapping_add(delta) < lower {
            delta = lower.wrapping_sub(karma);
        }
        let threshold = setting_int(e, KARMA_MESSAGE_THRESHOLD_SETTING);
        if delta < threshold.wrapping_neg() {
            show(e, KARMA_LOSS_BIG_ICON_SETTING, KARMA_LOSS_BIG_TEXT_SETTING);
        } else {
            show(e, KARMA_LOSS_ICON_SETTING, KARMA_LOSS_TEXT_SETTING);
        }
        SOUND_KARMA_DOWN
    } else {
        let upper = setting_int(e, KARMA_UPPER_LIMIT_SETTING);
        if karma.wrapping_add(delta) > upper {
            delta = upper.wrapping_sub(karma);
        }
        let threshold = setting_int(e, KARMA_MESSAGE_THRESHOLD_SETTING);
        if delta < threshold {
            show(e, KARMA_GAIN_ICON_SETTING, KARMA_GAIN_TEXT_SETTING);
        } else {
            show(e, KARMA_GAIN_BIG_ICON_SETTING, KARMA_GAIN_BIG_TEXT_SETTING);
        }
        SOUND_KARMA_UP
    };

    // `BSSoundHandle` locals: the handle and the temporary the lookup fills.
    let handle = e.mem.alloc(12);
    e.call(0x0041_a250, &args![handle]);
    let temporary = e.mem.alloc(12);
    let audio = e.call(0x0045_3a70, &args![]).u32();
    let found = e
        .call(0x00ad_7550, &args![audio, temporary, sound, 0x121u32])
        .u32();
    e.call(0x0041_8900, &args![handle, found]);
    e.call(RELEASE_HANDLE, &args![temporary]);
    e.mem.free(temporary);
    e.call(0x00ad_8830, &args![handle, 0u32]);
    e.call(RELEASE_HANDLE, &args![handle]);
    e.mem.free(handle);

    let upper = setting_int(e, KARMA_UPPER_LIMIT_SETTING);
    let upper_ok = delta <= 0 || karma < upper;
    let lower = setting_int(e, KARMA_LOWER_LIMIT_SETTING);
    let lower_ok = delta >= 0 || karma > lower;
    if upper_ok && lower_ok {
        e.vcall(player, 0x3a8, &args![0x17u32, delta, 0u32]);
    }
}

// Translated from 00950010 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `bChargen` (`+0x75c`).
pub fn fn_00950010(e: &mut Engine, this: Ptr<PlayerCharacter>, value: u8) {
    e.set(this, PlayerCharacter::bChargen, value);
}

// Translated from 00950030 (decompiled, FalloutNV.exe 1.4.0.525)
/// `PlayerCharacter::UpdatePlayer3d` (Xbox PDB): when the process's slot
/// `0x474` (with 1) says false and the inventory menu is visible, calls
/// `00483710` on the player; then calls the process's slot `0x464` with the
/// player.
pub fn player_character_update_player_3d(e: &mut Engine, this: Ptr<PlayerCharacter>) {
    let process = process_of(e, this);
    if !e.vcall(process, 0x474, &args![1u32]).bool()
        && e.call(INVENTORY_MENU_VISIBLE, &args![]).bool()
    {
        e.call(RELEASE_HANDLE, &args![this]);
    }
    let process = process_of(e, this);
    e.vcall(process, 0x464, &args![this]);
}

// Translated from 00950090 (decompiled, FalloutNV.exe 1.4.0.525)
/// The byte at `0x011e07b8` (vanity mode active). `this` is not used.
pub fn fn_00950090(e: &mut Engine, _this: Ptr) -> u8 {
    e.global::<u8>(VANITY_ACTIVE)
}

// Translated from 009500a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `PlayerCharacter::StopVanityMode` (Xbox PDB): ends vanity mode. When the
/// saved flag (`0x011e07b9`) is set, restores the saved value; clears both
/// flags, sets the HUD menu mode to 1 and resets the vanity state.
pub fn player_character_stop_vanity_mode(e: &mut Engine, _this: Ptr) {
    if e.global::<u8>(VANITY_SAVED_FLAG) != 0 {
        let saved: f32 = e.global(VANITY_SAVED_VALUE);
        e.set_global(VANITY_RESTORED_VALUE, saved);
    }
    e.set_global(VANITY_ACTIVE, 0u8);
    e.set_global(VANITY_SAVED_FLAG, 0u8);
    e.call(SET_MENU_MODE, &args![1u32]);
    e.set_global(VANITY_FLAG_07C3, 1u8);
    e.set_global(VANITY_FLAG_07C1, 0u8);
    for address in [
        VANITY_VALUE_07C8,
        VANITY_VALUE_07C4,
        VANITY_VALUE_0B58,
        VANITY_VALUE_0B60,
        VANITY_VALUE_07DC,
    ] {
        e.set_global(address, 0.0f32);
    }
}

/// This part's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x0094c490, fn_0094c490(Ptr, u32) -> f32),
        entry!(0x0094c4c0, fn_0094c4c0(Ptr, u32) -> f32),
        entry!(
            0x0094c4f0,
            fn_0094c4f0(Ptr<PlayerCharacter>, i32, i32, f32, u32)
        ),
        entry!(0x0094c640, fn_0094c640(Ptr, u32, u32) -> f32),
        entry!(0x0094c660, fn_0094c660(Ptr, u32, f32)),
        entry!(
            0x0094c680,
            player_character_add_spell(Ptr<PlayerCharacter>, Ptr) -> bool
        ),
        entry!(
            0x0094c6c0,
            player_character_spell_learned(Ptr<PlayerCharacter>, Ptr)
        ),
        entry!(0x0094c7a0, fn_0094c7a0(Ptr<PlayerCharacter>, Ptr) -> bool),
        entry!(0x0094c800, fn_0094c800(Ptr, u32, u32, Ptr, u8) -> bool),
        entry!(0x0094c870, fn_0094c870(Ptr, u32, u8)),
        entry!(0x0094c890, fn_0094c890(Ptr, u32, u32, u8)),
        entry!(0x0094c8c0, fn_0094c8c0(Ptr<PlayerCharacter>) -> u32),
        entry!(
            0x0094c8e0,
            player_character_set_selected_spell(Ptr<PlayerCharacter>, u32)
        ),
        entry!(0x0094c950, fn_0094c950(Ptr<PlayerCharacter>, Ptr)),
        entry!(0x0094ca20, fn_0094ca20(Ptr, u32, u32, u32, u8)),
        entry!(0x0094ca50, fn_0094ca50(Ptr) -> u32),
        entry!(0x0094cae0, fn_0094cae0(Ptr<PlayerCharacter>, Ptr)),
        entry!(0x0094db80, fn_0094db80(Ptr, u8)),
        entry!(0x0094dba0, fn_0094dba0() -> u8),
        entry!(0x0094dbb0, fn_0094dbb0(Ptr) -> Ptr),
        entry!(
            0x0094dbe0,
            player_character_return_to_last_known_good_position(Ptr<PlayerCharacter>, bool)
        ),
        entry!(0x0094df30, fn_0094df30(Ptr<PlayerCharacter>, Ptr) -> Ptr),
        entry!(
            0x0094df60,
            player_character_is_sleepingor_resting(Ptr<PlayerCharacter>) -> bool
        ),
        entry!(0x0094df80, fn_0094df80(Ptr<PlayerCharacter>)),
        entry!(
            0x0094e1d0,
            player_character_load_3d(Ptr<PlayerCharacter>, u8) -> u32
        ),
        entry!(0x0094eac0, fn_0094eac0() -> u32),
        entry!(0x0094ead0, fn_0094ead0() -> u32),
        entry!(0x0094eae0, fn_0094eae0() -> u32),
        entry!(0x0094eaf0, fn_0094eaf0(Ptr, Ptr, bool)),
        entry!(0x0094eb30, fn_0094eb30() -> u32),
        entry!(0x0094eb40, fn_0094eb40(Ptr<PlayerCharacter>, Ptr, u8)),
        entry!(0x0094ec90, build_inv_string_callback(Ptr, Ptr) -> bool),
        entry!(
            0x0094ed40,
            player_character_export_progress_data(Ptr<PlayerCharacter>, i32)
        ),
        entry!(0x0094fce0, fn_0094fce0(Ptr<PlayerCharacter>, u32, u32)),
        entry!(0x0094fd10, fn_0094fd10(Ptr<PlayerCharacter>, u32)),
        entry!(
            0x0094fd30,
            player_character_reward_karma(Ptr<PlayerCharacter>, i32)
        ),
        entry!(0x00950010, fn_00950010(Ptr<PlayerCharacter>, u8)),
        entry!(
            0x00950030,
            player_character_update_player_3d(Ptr<PlayerCharacter>)
        ),
        entry!(0x00950090, fn_00950090(Ptr) -> u8),
        entry!(0x009500a0, player_character_stop_vanity_mode(Ptr)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// The source of this file: every code address it mentions is a callee
    /// (or one of its own functions), and gets a test double below.
    const SOURCE: &str = include_str!("playercharacter_p2.rs");

    static NEXT_TARGET: AtomicU32 = AtomicU32::new(0x0300_0000);

    fn eax(value: u32) -> Ret {
        Ret {
            eax: value,
            ..Ret::default()
        }
    }

    fn st0(value: f64) -> Ret {
        Ret {
            st0: value,
            ..Ret::default()
        }
    }

    /// Every address in the code range that the source mentions.
    fn mentioned_code_addresses() -> Vec<u32> {
        let bytes = SOURCE.as_bytes();
        let mut found = Vec::new();
        let mut i = 0;
        while i + 2 < bytes.len() {
            if bytes[i] == b'0' && bytes[i + 1] == b'x' {
                let mut j = i + 2;
                let mut digits = String::new();
                while j < bytes.len() && (bytes[j].is_ascii_hexdigit() || bytes[j] == b'_') {
                    if bytes[j] != b'_' {
                        digits.push(bytes[j] as char);
                    }
                    j += 1;
                }
                if let Ok(value) = u32::from_str_radix(&digits, 16) {
                    if (0x0040_1000..0x00fd_f000).contains(&value) && !found.contains(&value) {
                        found.push(value);
                    }
                }
                i = j;
            } else {
                i += 1;
            }
        }
        found
    }

    /// An engine with the data pages the code reads, and a double that answers
    /// zero over every callee outside this file.
    fn engine() -> Engine {
        let mut e = Engine::new();
        e.map(0x0100_0000, 0x0010_0000);
        e.map(0x0118_0000, 0x0010_0000);
        e.map(0x0600_0000, 0x0001_0000);
        let own: Vec<u32> = funcs().iter().map(|(address, _)| *address).collect();
        for address in mentioned_code_addresses() {
            if !own.contains(&address) {
                e.register_double(address, |_, _| Ret::default());
            }
        }
        e
    }

    fn double(e: &mut Engine, address: u32, ret: Ret) {
        e.register_double(address, move |_, _| ret);
    }

    /// The arguments of every logged call to `address`.
    fn calls_to(e: &Engine, address: u32) -> Vec<Vec<u32>> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .filter(|(called, _)| *called == address)
            .map(|(_, words)| words.clone())
            .collect()
    }

    fn start_log(e: &mut Engine) {
        e.call_log = Some(Vec::new());
    }

    fn new_player(e: &mut Engine) -> Ptr<PlayerCharacter> {
        e.new_object::<PlayerCharacter>()
    }

    /// Puts a double answering `ret` into slot `offset` of the object's vtable
    /// (a fresh table when it has none).
    fn slot(e: &mut Engine, object: u32, offset: u32, ret: Ret) -> u32 {
        let mut table = e.mem.u32(object);
        if table == 0 {
            table = e.mem.alloc(0x800);
            e.mem.set_u32(object, table);
        }
        let target = NEXT_TARGET.fetch_add(4, Ordering::Relaxed);
        e.mem.set_u32(table + offset, target);
        double(e, target, ret);
        target
    }

    /// Like [`slot`] with a double that records its argument words.
    fn recording_slot(e: &mut Engine, object: u32, offset: u32, ret: Ret) -> CallRecord {
        let mut table = e.mem.u32(object);
        if table == 0 {
            table = e.mem.alloc(0x800);
            e.mem.set_u32(object, table);
        }
        let target = NEXT_TARGET.fetch_add(4, Ordering::Relaxed);
        e.mem.set_u32(table + offset, target);
        let seen = Rc::new(RefCell::new(Vec::new()));
        let record = seen.clone();
        e.register_double(target, move |_, words| {
            record.borrow_mut().push(words.to_vec());
            ret
        });
        seen
    }

    /// Makes the list helpers behave as the game's: a node is `{item, next}`,
    /// the slot of a node is the node, the head test is "both words zero".
    fn list_helpers(e: &mut Engine) {
        e.register(LIST_NODE_ITEM_SLOT, |_, a| eax(a[0]));
        e.register(LIST_NODE_NEXT, |e, a| eax(e.mem.u32(a[0] + 4)));
        e.register(LIST_IS_EMPTY, |e, a| {
            eax((e.mem.u32(a[0]) == 0 && e.mem.u32(a[0] + 4) == 0) as u32)
        });
    }

    /// A list node `{item, next}`.
    fn node(e: &mut Engine, item: u32, next: u32) -> u32 {
        let block = e.mem.alloc(8);
        e.mem.set_u32(block, item);
        e.mem.set_u32(block + 4, next);
        block
    }

    type CallRecord = Rc<RefCell<Vec<Vec<u32>>>>;

    // ----- actor value modifiers -----

    #[test]
    fn modifier_getters_ask_for_the_player_and_their_kind() {
        let mut e = engine();
        let player = new_player(&mut e);
        double(&mut e, GET_MODIFIER, st0(1.5));
        start_log(&mut e);
        let owner = Ptr::<()>::new(player.addr() + 0xa4);
        assert_eq!(fn_0094c490(&mut e, owner, 7), 1.5);
        assert_eq!(fn_0094c4c0(&mut e, owner, 9), 1.5);
        assert_eq!(
            calls_to(&e, GET_MODIFIER),
            vec![vec![player.addr(), 2, 7], vec![player.addr(), 1, 9]]
        );
    }

    fn clamp_adds(e: &mut Engine) {
        e.register(CLAMP_ADD, |_, a| {
            st0((f32::from_bits(a[0]) + f32::from_bits(a[1])) as f64)
        });
        e.register(CHECK_CLAMP_DAMAGE_MODIFIER, |_, a| {
            st0((f32::from_bits(a[2]) * 2.0) as f64)
        });
    }

    #[test]
    fn modifier_change_updates_the_temporary_and_script_arrays() {
        let mut e = engine();
        clamp_adds(&mut e);
        let player = new_player(&mut e);
        e.mem.set_f32(player.addr() + 0x244 + 3 * 4, 2.0);
        e.mem.set_f32(player.addr() + 0x378 + 5 * 4, 10.0);
        start_log(&mut e);
        fn_0094c4f0(&mut e, player, 0, 3, 1.5, 0);
        fn_0094c4f0(&mut e, player, 1, 5, -4.0, 1);
        assert_eq!(e.mem.f32(player.addr() + 0x244 + 12), 3.5);
        assert_eq!(e.mem.f32(player.addr() + 0x378 + 20), 6.0);
        let adds = calls_to(&e, CLAMP_ADD);
        assert_eq!(adds[0][2], 0);
        assert_eq!(adds[1][2], 1);
    }

    #[test]
    fn modifier_change_for_no_actor_value_or_unknown_mode_does_nothing() {
        let mut e = engine();
        clamp_adds(&mut e);
        let player = new_player(&mut e);
        start_log(&mut e);
        fn_0094c4f0(&mut e, player, 0, -1, 1.0, 0);
        fn_0094c4f0(&mut e, player, 1, -1, 1.0, 0);
        fn_0094c4f0(&mut e, player, 7, 3, 1.0, 0);
        assert!(calls_to(&e, CLAMP_ADD).is_empty());
        assert!(calls_to(&e, CHECK_CLAMP_DAMAGE_MODIFIER).is_empty());
    }

    #[test]
    fn damage_modifier_goes_through_the_clamp_check_first() {
        let mut e = engine();
        clamp_adds(&mut e);
        let player = new_player(&mut e);
        e.mem.set_f32(player.addr() + 0x4b0 + 4 * 4, 1.0);
        e.mem.set_f32(player.addr() + 0x4ac, 10.0);
        start_log(&mut e);
        // Actor value 4: the damage array; the check doubles the delta.
        fn_0094c4f0(&mut e, player, 2, 4, 3.0, 0);
        assert_eq!(e.mem.f32(player.addr() + 0x4b0 + 16), 7.0);
        let checks = calls_to(&e, CHECK_CLAMP_DAMAGE_MODIFIER);
        assert_eq!(checks[0][0], player.addr() + 0xa4);
        assert_eq!(checks[0][1], 4);
        // Actor value 0x10 has its own word.
        fn_0094c4f0(&mut e, player, 2, 0x10, 1.0, 0);
        assert_eq!(e.get(player, PlayerCharacter::fHealthModifier), 12.0);
        // No actor value: the check still runs, nothing is stored.
        fn_0094c4f0(&mut e, player, 2, -1, 1.0, 0);
        assert_eq!(calls_to(&e, CHECK_CLAMP_DAMAGE_MODIFIER).len(), 3);
        assert_eq!(calls_to(&e, CLAMP_ADD).len(), 2);
    }

    #[test]
    fn modifier_item_forwarders_pass_everything_on() {
        let mut e = engine();
        let player = new_player(&mut e);
        double(&mut e, MODIFIER_ITEM_GETTER, st0(2.5));
        start_log(&mut e);
        assert_eq!(fn_0094c640(&mut e, player.cast(), 4, 5), 2.5);
        fn_0094c660(&mut e, player.cast(), 6, 0.25);
        assert_eq!(
            calls_to(&e, MODIFIER_ITEM_GETTER),
            vec![vec![player.addr(), 4, 5]]
        );
        assert_eq!(
            calls_to(&e, MODIFIER_ITEM_SETTER),
            vec![vec![player.addr(), 6, 0.25f32.to_bits()]]
        );
    }

    // ----- spells -----

    /// A spell object whose interface (at `+0x18`) answers `kind` in slot 0x18.
    fn spell_with_kind(e: &mut Engine, kind: u32) -> u32 {
        let spell = e.mem.alloc(0x100);
        slot(e, spell + 0x18, 0x18, eax(kind));
        spell
    }

    #[test]
    fn learning_a_spell_of_a_plain_kind_does_nothing() {
        let mut e = engine();
        list_helpers(&mut e);
        let player = new_player(&mut e);
        let spell = spell_with_kind(&mut e, 1);
        e.mem.set_u32(spell + 0x28, 0x1234);
        start_log(&mut e);
        player_character_spell_learned(&mut e, player, Ptr::new(spell));
        assert!(calls_to(&e, 0x0040_7e00).is_empty());
    }

    #[test]
    fn learning_a_spell_walks_its_effect_list() {
        let mut e = engine();
        list_helpers(&mut e);
        e.register(0x0082_5c00, |_, a| eax(a[0] + 0x1000));
        let player = new_player(&mut e);
        for kind in [0, 4, 2, 3] {
            let spell = spell_with_kind(&mut e, kind);
            // Two entries, the second without an item, then the end.
            let third = node(&mut e, 0x30, 0);
            let second = node(&mut e, 0, third);
            e.mem.set_u32(spell + 0x28, 0x10);
            e.mem.set_u32(spell + 0x2c, second);
            start_log(&mut e);
            player_character_spell_learned(&mut e, player, Ptr::new(spell));
            assert_eq!(
                calls_to(&e, 0x0040_7e00),
                vec![
                    vec![0x10 + 0x1000, 0x0020_0000, 1],
                    vec![0x30 + 0x1000, 0x0020_0000, 1]
                ],
                "kind {kind}"
            );
        }
    }

    #[test]
    fn adding_a_spell_learns_it_only_when_it_was_added() {
        let mut e = engine();
        list_helpers(&mut e);
        let player = new_player(&mut e);
        let spell = spell_with_kind(&mut e, 1);
        double(&mut e, ACTOR_ADD_SPELL, eax(0));
        start_log(&mut e);
        assert!(!player_character_add_spell(&mut e, player, Ptr::new(spell)));
        double(&mut e, ACTOR_ADD_SPELL, eax(1));
        assert!(player_character_add_spell(&mut e, player, Ptr::new(spell)));
        let table = e.mem.u32(spell + 0x18);
        assert!(table != 0);
        assert_eq!(
            calls_to(&e, ACTOR_ADD_SPELL),
            vec![vec![player.addr(), spell], vec![player.addr(), spell]]
        );
    }

    #[test]
    fn selecting_a_spell_unloads_the_old_one_and_preloads_the_new_one() {
        let mut e = engine();
        let player = new_player(&mut e);
        start_log(&mut e);
        player_character_set_selected_spell(&mut e, player, 0x5000);
        assert_eq!(
            e.get(player, PlayerCharacter::pSelectedSpell).addr(),
            0x5000
        );
        assert!(calls_to(&e, MAGIC_ITEM_UNLOAD).is_empty());
        assert_eq!(calls_to(&e, MAGIC_ITEM_PRELOAD), vec![vec![0x5000, 0]]);
        assert_eq!(calls_to(&e, RESET_MAGIC_CAST_SOUND).len(), 1);
        // The same spell again changes nothing.
        player_character_set_selected_spell(&mut e, player, 0x5000);
        assert_eq!(calls_to(&e, RESET_MAGIC_CAST_SOUND).len(), 1);
        // Another spell unloads the first.
        player_character_set_selected_spell(&mut e, player, 0x6000);
        assert_eq!(calls_to(&e, MAGIC_ITEM_UNLOAD), vec![vec![0x5000, 1]]);
        // No spell: unload only.
        player_character_set_selected_spell(&mut e, player, 0);
        assert_eq!(calls_to(&e, MAGIC_ITEM_UNLOAD).len(), 2);
        assert_eq!(calls_to(&e, MAGIC_ITEM_PRELOAD).len(), 2);
        assert_eq!(calls_to(&e, RESET_MAGIC_CAST_SOUND).len(), 3);
        assert_eq!(fn_0094c8c0(&mut e, player), 0);
    }

    #[test]
    fn removing_the_selected_spell_clears_the_selection() {
        let mut e = engine();
        let player = new_player(&mut e);
        let spell = 0x7000;
        e.set(
            player,
            PlayerCharacter::pSelectedSpell,
            Ptr::new(spell + 0x18),
        );
        double(&mut e, ACTOR_REMOVE_SPELL, eax(0));
        assert!(!fn_0094c7a0(&mut e, player, Ptr::new(spell)));
        assert_eq!(fn_0094c8c0(&mut e, player), spell + 0x18);
        // Removed, but another spell is selected.
        double(&mut e, ACTOR_REMOVE_SPELL, eax(1));
        assert!(fn_0094c7a0(&mut e, player, Ptr::new(0x8000)));
        assert_eq!(fn_0094c8c0(&mut e, player), spell + 0x18);
        // Removed and selected.
        assert!(fn_0094c7a0(&mut e, player, Ptr::new(spell)));
        assert_eq!(fn_0094c8c0(&mut e, player), 0);
        // No spell and nothing selected: the null interface matches.
        assert!(fn_0094c7a0(&mut e, player, Ptr::NULL));
    }

    #[test]
    fn selecting_a_scroll_selects_its_spell() {
        let mut e = engine();
        list_helpers(&mut e);
        let player = new_player(&mut e);
        // A scroll whose list entry at +0x74 has a next word of 0x9000.
        let scroll = e.mem.alloc(0x100);
        e.mem.set_u32(scroll + 0x78, 0x9000);
        start_log(&mut e);
        fn_0094c950(&mut e, player, Ptr::new(scroll));
        assert_eq!(
            e.get(player, PlayerCharacter::pSelectedScroll).addr(),
            scroll
        );
        assert_eq!(fn_0094c8c0(&mut e, player), 0x9000 + 0x18);
        // Selecting it again does nothing.
        let before = calls_to(&e, RESET_MAGIC_CAST_SOUND).len();
        fn_0094c950(&mut e, player, Ptr::new(scroll));
        assert_eq!(calls_to(&e, RESET_MAGIC_CAST_SOUND).len(), before);
        // Clearing it clears the spell too.
        fn_0094c950(&mut e, player, Ptr::NULL);
        assert_eq!(e.get(player, PlayerCharacter::pSelectedScroll).addr(), 0);
        assert_eq!(fn_0094c8c0(&mut e, player), 0);
    }

    #[test]
    fn a_scroll_without_a_spell_clears_the_scroll_only() {
        let mut e = engine();
        list_helpers(&mut e);
        let player = new_player(&mut e);
        let scroll = e.mem.alloc(0x100);
        e.set(player, PlayerCharacter::pSelectedScroll, Ptr::new(0x4444));
        e.set(player, PlayerCharacter::pSelectedSpell, Ptr::new(0x5555));
        fn_0094c950(&mut e, player, Ptr::new(scroll));
        assert_eq!(e.get(player, PlayerCharacter::pSelectedScroll).addr(), 0);
        assert_eq!(fn_0094c8c0(&mut e, player), 0x5555);
    }

    #[test]
    fn the_spell_check_reports_its_code_and_honours_god_mode() {
        let mut e = engine();
        let player = new_player(&mut e);
        let out = e.mem.alloc(4);
        e.register_double(SPELL_CHECK, |e, a| {
            e.mem.set_u32(a[3], 5);
            eax(0)
        });
        // Code 5 and failure: false even in god mode.
        double(&mut e, IS_GOD_MODE, eax(1));
        assert!(!fn_0094c800(&mut e, player.cast(), 1, 2, Ptr::new(out), 3));
        assert_eq!(e.mem.u32(out), 5);
        // Another failure code: god mode wins.
        e.register_double(SPELL_CHECK, |e, a| {
            e.mem.set_u32(a[3], 2);
            eax(0)
        });
        assert!(fn_0094c800(&mut e, player.cast(), 1, 2, Ptr::NULL, 3));
        // Without god mode the call's own answer is returned.
        double(&mut e, IS_GOD_MODE, eax(0));
        assert!(!fn_0094c800(&mut e, player.cast(), 1, 2, Ptr::NULL, 3));
        e.register_double(SPELL_CHECK, |_, _| eax(1));
        assert!(fn_0094c800(&mut e, player.cast(), 1, 2, Ptr::NULL, 3));
    }

    #[test]
    fn magic_forwarders_pass_everything_on() {
        let mut e = engine();
        let player = new_player(&mut e);
        start_log(&mut e);
        fn_0094c870(&mut e, player.cast(), 11, 1);
        fn_0094c890(&mut e, player.cast(), 12, 13, 1);
        fn_0094ca20(&mut e, player.cast(), 14, 15, 16, 1);
        let me = player.addr();
        assert_eq!(
            calls_to(&e, MAGIC_CASTER_CAST_ABILITY),
            vec![vec![me, 11, 1]]
        );
        assert_eq!(
            calls_to(&e, MAGIC_CASTER_TRANSFER_DISEASE),
            vec![vec![me, 12, 13, 1]]
        );
        assert_eq!(
            calls_to(&e, MAGIC_TARGET_ADD_TARGET),
            vec![vec![me, 14, 15, 16, 1]]
        );
    }

    #[test]
    fn the_size_lookup_falls_back_when_nothing_is_found() {
        let mut e = engine();
        let player = new_player(&mut e);
        e.set_global(PLAYER_POINTER, 0x1111u32);
        slot(&mut e, player.addr(), 0x380, st0(2.0));
        double(&mut e, REFERENCE_GET_SCALE, st0(1.5));
        let found = e.mem.alloc(0x10);
        let found_slot = slot(&mut e, found, 0x19c, eax(77));
        let _ = found_slot;
        double(&mut e, PLAYER_LOOKUP_BY_SIZE, eax(0));
        double(&mut e, FALLBACK_LOOKUP, eax(0));
        let this = Ptr::<()>::new(player.addr() + 0x88);
        start_log(&mut e);
        assert_eq!(fn_0094ca50(&mut e, this), 0);
        assert_eq!(
            calls_to(&e, PLAYER_LOOKUP_BY_SIZE),
            vec![vec![0x1111, 3.0f32.to_bits()]]
        );
        // The fallback finds the object.
        double(&mut e, FALLBACK_LOOKUP, eax(found));
        assert_eq!(fn_0094ca50(&mut e, this), 77);
        // The first lookup finds it.
        double(&mut e, PLAYER_LOOKUP_BY_SIZE, eax(found));
        double(&mut e, FALLBACK_LOOKUP, eax(0));
        assert_eq!(fn_0094ca50(&mut e, this), 77);
    }

    // ----- the parent-cell change handler -----

    const PARENT_SLOT: u32 = 0x0600_0000;

    fn quiet_slots(e: &mut Engine, object: u32, offsets: &[u32]) {
        for offset in offsets {
            slot(e, object, *offset, Ret::default());
        }
    }

    /// A player in a world where the parent cell is `old`, with the pointer
    /// getter, the camera body and the virtual slots the handler uses.
    fn cell_environment(e: &mut Engine, old: u32) -> (Ptr<PlayerCharacter>, u32) {
        list_helpers(e);
        let player = new_player(e);
        quiet_slots(e, player.addr(), &[0x1fc, 0x22c, 0x2e8, 0x230, 0x494]);
        let body = e.mem.alloc(0x40);
        quiet_slots(e, body, &[0x9c, 0xe4]);
        e.mem.set_u32(player.addr() + 0xdec, body);
        e.mem.set_u32(PARENT_SLOT, old);
        e.register(PARENT_CELL_OF, |e, _| eax(e.mem.u32(PARENT_SLOT)));
        e.register(0x0093_0700, |e, a| {
            e.mem.set_u32(PARENT_SLOT, a[1]);
            Ret::default()
        });
        e.register(0x0055_9450, |e, a| eax(e.mem.u32(a[0])));
        e.set_global(CELL_SIZE, 4096.0f64);
        e.set_global(BORDER_DISTANCE_LIMIT, 2048.0f64);
        e.set_global(NO_WATER_HEIGHT, -3.4e38f32);
        // The border setting reads a zero byte.
        e.register(0x0040_8d60, |_, _| eax(0x0600_0100));
        (player, body)
    }

    #[test]
    fn entering_the_current_cell_does_nothing() {
        let mut e = engine();
        let (player, _) = cell_environment(&mut e, 0x5000);
        start_log(&mut e);
        fn_0094cae0(&mut e, player, Ptr::new(0x5000));
        let log = e.call_log.clone().unwrap();
        let addresses: Vec<u32> = log.iter().map(|(address, _)| *address).collect();
        assert_eq!(addresses, vec![PARENT_CELL_OF]);
    }

    #[test]
    fn leaving_every_cell_clears_the_region_sounds() {
        let mut e = engine();
        let (player, _) = cell_environment(&mut e, 0x5000);
        start_log(&mut e);
        fn_0094cae0(&mut e, player, Ptr::NULL);
        // The water height falls back to the "no water" constant.
        assert_eq!(
            calls_to(&e, 0x0062_f8f0),
            vec![vec![(-3.4e38f32).to_bits()]]
        );
        assert_eq!(calls_to(&e, 0x0042_1e40).len(), 1);
        assert_eq!(calls_to(&e, 0x0093_0700), vec![vec![player.addr(), 0]]);
        assert_eq!(calls_to(&e, LIST_CLEAR), vec![vec![player.addr() + 0x774]]);
        assert_eq!(calls_to(&e, 0x0082_d7c0).len(), 1);
        assert_eq!(e.mem.u32(PARENT_SLOT), 0);
        // Nothing else of the cell handling ran.
        assert!(calls_to(&e, 0x0054_74b0).is_empty());
    }

    #[test]
    fn the_water_height_of_the_new_cell_is_used_when_it_has_water() {
        let mut e = engine();
        let (player, _) = cell_environment(&mut e, 0x1111);
        double(&mut e, 0x0045_18e0, eax(1));
        double(&mut e, 0x0054_71e0, st0(12.5));
        start_log(&mut e);
        fn_0094cae0(&mut e, player, Ptr::new(0x5000));
        assert_eq!(calls_to(&e, 0x0062_f8f0), vec![vec![12.5f32.to_bits()]]);
        // The flag byte of 0094dba0 suppresses it.
        let mut e = engine();
        let (player, _) = cell_environment(&mut e, 0x1111);
        e.set_global(SKIP_WATER_HEIGHT_FLAG, 1u8);
        start_log(&mut e);
        fn_0094cae0(&mut e, player, Ptr::new(0x5000));
        assert!(calls_to(&e, 0x0062_f8f0).is_empty());
    }

    #[test]
    fn a_new_cell_updates_the_music_the_interior_and_the_corners() {
        let mut e = engine();
        let (player, _) = cell_environment(&mut e, 0x1111);
        double(&mut e, 0x0054_74b0, eax(0xaaaa));
        double(&mut e, 0x0045_43c0, eax(0x6000));
        double(&mut e, 0x0045_cd60, eax(0x6100));
        double(&mut e, 0x0054_4c30, eax(2));
        double(&mut e, 0x0054_4c60, eax(3));
        double(&mut e, 0x0054_ddd0, eax(0x7777));
        double(&mut e, 0x0054_75b0, eax(0x6200));
        double(&mut e, 0x0046_dd00, eax(0x6300));
        start_log(&mut e);
        fn_0094cae0(&mut e, player, Ptr::new(0x5000));
        assert_eq!(e.mem.u32(player.addr() + 0x7b4), 0xaaaa);
        assert_eq!(calls_to(&e, 0x0093_a7a0), vec![vec![player.addr(), 0]]);
        assert_eq!(calls_to(&e, 0x0048_93c0), vec![vec![0x6100, 1]]);
        assert_eq!(calls_to(&e, 0x0063_c8f0), vec![vec![0x6300, 0x6200, 0]]);
        let corners: Vec<(u32, u32)> = calls_to(&e, 0x004f_7070)
            .iter()
            .map(|words| (words[1], words[2]))
            .collect();
        let bits = |x: f32, y: f32| (x.to_bits(), y.to_bits());
        assert_eq!(
            corners,
            vec![
                bits(8192.0, 12288.0),
                bits(8192.0, 16384.0),
                bits(12288.0, 12288.0),
                bits(12288.0, 16384.0)
            ]
        );
        assert_eq!(
            calls_to(&e, 0x0070_5420),
            vec![vec![0, 0, 0, 1]],
            "no occupied region: the music pair is built from nothing"
        );
        assert_eq!(calls_to(&e, 0x0094_7d80), vec![vec![player.addr()]]);
        assert_eq!(calls_to(&e, 0x0094_80c0), vec![vec![player.addr()]]);
    }

    #[test]
    fn a_cell_region_that_holds_the_player_becomes_the_current_region() {
        let mut e = engine();
        let (player, _) = cell_environment(&mut e, 0x1111);
        // The cell has one region entry: region REGION with one shape ENTRY.
        let region = e.mem.alloc(0x40);
        let entry = 0x4242;
        let entries = node(&mut e, entry, 0);
        let region_list = e.mem.alloc(0x10);
        e.mem.set_u32(region_list + 4, region);
        e.register_double(0x0054_7110, move |_, _| eax(region_list));
        double(&mut e, 0x0044_1110, eax(entries));
        double(&mut e, 0x004f_8360, eax(1));
        // Region data: kind 7 is the sound data.
        let sound_data = e.mem.alloc(0x40);
        let sound = node(&mut e, 0x9999, 0);
        slot(&mut e, sound_data, 0x30, eax(1));
        e.register(0x0096_11e0, |_, a| eax(a[0]));
        e.register_double(0x004f_35b0, move |_, a| {
            eax(if a[1] == 7 { sound_data } else { 0 })
        });
        double(&mut e, 0x004f_1540, eax(1));
        double(&mut e, 0x0048_d150, eax(sound));
        double(&mut e, 0x0054_9580, eax(1));
        e.register(OPERATOR_NEW, |e, a| eax(e.mem.alloc(a[0])));
        e.register(0x0096_a640, |_, a| eax(a[0]));
        start_log(&mut e);
        fn_0094cae0(&mut e, player, Ptr::new(0x5000));

        // The region sounds were collected and the region remembered.
        assert_eq!(e.global::<u32>(CURRENT_REGION_WITH_SOUNDS), sound_data);
        assert_eq!(
            calls_to(&e, 0x005a_e3d0),
            vec![vec![player.addr() + 0x774, sound]]
        );
        // The best region replaced the (empty) list once, at the start and
        // for the replacement.
        let clears: Vec<u32> = calls_to(&e, LIST_CLEAR).iter().map(|w| w[0]).collect();
        assert_eq!(
            clears
                .iter()
                .filter(|a| **a == player.addr() + 0x774)
                .count(),
            2
        );
        assert!(clears.contains(&(player.addr() + 0x768)));
        // The player is inside all four corners: the cell is a border cell.
        assert!(e.get(player, PlayerCharacter::bInBorderContainedCell));
        // No occupied region yet: the region becomes the occupied one.
        assert!(calls_to(&e, 0x0093_a7a0).contains(&vec![player.addr(), region]));
        // The border region array was created and the region added to it.
        let array = e.mem.u32(player.addr() + 0x7b0);
        assert!(array != 0);
        let added = calls_to(&e, 0x0096_a610);
        assert_eq!(added.len(), 1);
        assert_eq!(added[0][0], array);
        assert_eq!(e.mem.u32(added[0][1]), region);
        // The entries of the occupied region list were gathered with the
        // player point.
        assert_eq!(
            calls_to(&e, 0x004f_6600),
            vec![vec![player.addr() + 0x764, region]]
        );
    }

    #[test]
    fn regions_that_do_not_apply_are_skipped() {
        let mut e = engine();
        let (player, _) = cell_environment(&mut e, 0x1111);
        let region = e.mem.alloc(0x40);
        let region_list = e.mem.alloc(0x10);
        e.mem.set_u32(region_list + 4, region);
        e.register_double(0x0054_7110, move |_, _| eax(region_list));
        // Hidden regions are skipped before anything else.
        double(&mut e, 0x0044_0d80, eax(1));
        start_log(&mut e);
        fn_0094cae0(&mut e, player, Ptr::new(0x5000));
        assert!(calls_to(&e, 0x0044_1110).is_empty());
        // A region of another world space is skipped.
        double(&mut e, 0x0044_0d80, eax(0));
        double(&mut e, 0x007a_f430, eax(0x1000));
        double(&mut e, 0x0057_5d70, eax(0x2000));
        start_log(&mut e);
        fn_0094cae0(&mut e, player, Ptr::new(0x5001));
        assert!(calls_to(&e, 0x0044_1110).is_empty());
        // A region without entries is skipped.
        double(&mut e, 0x0057_5d70, eax(0x1000));
        double(&mut e, 0x0044_1110, eax(0));
        start_log(&mut e);
        fn_0094cae0(&mut e, player, Ptr::new(0x5002));
        assert!(calls_to(&e, 0x004f_8360).is_empty());
        // So is a region whose entry list is empty.
        let empty = e.mem.alloc(8);
        double(&mut e, 0x0044_1110, eax(empty));
        start_log(&mut e);
        fn_0094cae0(&mut e, player, Ptr::new(0x5003));
        assert!(calls_to(&e, 0x004f_8360).is_empty());
        assert_eq!(e.mem.u32(player.addr() + 0x7b0), 0);
    }

    #[test]
    fn an_occupied_region_is_replaced_by_a_better_one() {
        let mut e = engine();
        let (player, _) = cell_environment(&mut e, 0x1111);
        let region = e.mem.alloc(0x40);
        let entries = node(&mut e, 0x4242, 0);
        let region_list = e.mem.alloc(0x10);
        e.mem.set_u32(region_list + 4, region);
        e.register_double(0x0054_7110, move |_, _| eax(region_list));
        double(&mut e, 0x0044_1110, eax(entries));
        double(&mut e, 0x004f_8360, eax(1));
        e.register(0x0096_11e0, |_, a| eax(a[0]));
        // The occupied region and the new one have data of kind 3.
        let occupied = e.mem.alloc(0x40);
        e.mem.set_u32(player.addr() + 0x760, occupied);
        let old_data = 0x7100;
        let new_data = 0x7200;
        e.register_double(0x004f_35b0, move |_, a| {
            eax(match (a[0], a[1]) {
                (list, 3) if list == occupied => old_data,
                (_, 3) => new_data,
                _ => 0,
            })
        });
        // Same flag, higher priority for the new data.
        double(&mut e, 0x004f_1540, eax(0));
        e.register(0x005b_b4d0, |_, a| eax(if a[0] == 0x7200 { 9 } else { 4 }));
        start_log(&mut e);
        fn_0094cae0(&mut e, player, Ptr::new(0x5000));
        assert!(calls_to(&e, 0x0093_a7a0).contains(&vec![player.addr(), region]));
        // A lower priority keeps the occupied region.
        e.register(0x005b_b4d0, |_, a| eax(if a[0] == 0x7200 { 2 } else { 4 }));
        e.mem.set_u32(PARENT_SLOT, 0x1111);
        start_log(&mut e);
        fn_0094cae0(&mut e, player, Ptr::new(0x5000));
        assert!(!calls_to(&e, 0x0093_a7a0).contains(&vec![player.addr(), region]));
    }

    fn border_environment(e: &mut Engine) -> (Ptr<PlayerCharacter>, u32) {
        let (player, _) = cell_environment(e, 0x1111);
        let setting = e.mem.alloc(4);
        e.mem.set_u8(setting, 1);
        e.register_double(0x0040_8d60, move |_, _| eax(setting));
        let world = 0x7777;
        double(e, 0x0054_ddd0, eax(world));
        double(e, 0x0058_6260, eax(1));
        quiet_slots(e, player.addr(), &[0x1f4]);
        let position = e.mem.alloc(12);
        slot(e, player.addr(), 0x1f4, eax(position));
        (player, world)
    }

    #[test]
    fn leaving_the_remembered_cell_marks_a_return_to_the_last_good_position() {
        let mut e = engine();
        let (player, _) = border_environment(&mut e);
        // The last known good location is an exterior cell (type 0x39) other
        // than the new one.
        e.mem.set_u32(player.addr() + 0x7ac, 0x3333);
        double(&mut e, FORM_TYPE_OF, eax(0x39));
        fn_0094cae0(&mut e, player, Ptr::new(0x5000));
        assert!(e.get(player, PlayerCharacter::bReturnToLastKnownGoodPosition));
    }

    #[test]
    fn another_world_space_marks_a_return_to_the_last_good_position() {
        let mut e = engine();
        let (player, _) = border_environment(&mut e);
        e.mem.set_u32(player.addr() + 0x7ac, 0x3333);
        double(&mut e, FORM_TYPE_OF, eax(0x41));
        fn_0094cae0(&mut e, player, Ptr::new(0x5000));
        assert!(e.get(player, PlayerCharacter::bReturnToLastKnownGoodPosition));
    }

    #[test]
    fn far_from_the_border_in_the_same_world_space_marks_it_too() {
        let mut e = engine();
        let (player, world) = border_environment(&mut e);
        e.mem.set_u32(player.addr() + 0x7ac, world);
        double(&mut e, FORM_TYPE_OF, eax(0x41));
        double(&mut e, 0x0045_7990, st0(3000.0));
        fn_0094cae0(&mut e, player, Ptr::new(0x5000));
        assert!(e.get(player, PlayerCharacter::bReturnToLastKnownGoodPosition));
        // Within the limit it stays clear.
        let mut e = engine();
        let (player, world) = border_environment(&mut e);
        e.mem.set_u32(player.addr() + 0x7ac, world);
        double(&mut e, FORM_TYPE_OF, eax(0x41));
        double(&mut e, 0x0045_7990, st0(2048.0));
        fn_0094cae0(&mut e, player, Ptr::new(0x5000));
        assert!(!e.get(player, PlayerCharacter::bReturnToLastKnownGoodPosition));
    }

    #[test]
    fn the_border_check_is_skipped_without_the_setting() {
        let mut e = engine();
        let (player, _) = border_environment(&mut e);
        let off = e.mem.alloc(4);
        e.register_double(0x0040_8d60, move |_, _| eax(off));
        e.mem.set_u32(player.addr() + 0x7ac, 0x3333);
        double(&mut e, FORM_TYPE_OF, eax(0x39));
        fn_0094cae0(&mut e, player, Ptr::new(0x5000));
        assert!(!e.get(player, PlayerCharacter::bReturnToLastKnownGoodPosition));
    }

    #[test]
    fn the_camera_body_is_built_when_it_is_missing() {
        let mut e = engine();
        let (player, body) = cell_environment(&mut e, 0x1111);
        e.register(OPERATOR_NEW, |e, a| eax(e.mem.alloc(a[0])));
        e.register(0x00aa_13e0, |e, a| eax(e.mem.alloc(a[0])));
        e.register(SMART_POINTER_ASSIGN, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            Ret::default()
        });
        double(&mut e, 0x0082_2510, eax(1));
        let prototype = e.mem.alloc(0x10);
        quiet_slots(&mut e, prototype, &[0x9c, 0xe4]);
        e.register_double(0x0056_d380, move |e, a| {
            let table = e.mem.u32(prototype);
            e.mem.set_u32(a[0], table);
            eax(a[0])
        });
        let motion = e.mem.alloc(0x40);
        double(&mut e, 0x004a_e700, eax(motion));
        let position = e.mem.alloc(12);
        e.mem.set_u32(position, 1);
        e.mem.set_u32(position + 4, 2);
        e.mem.set_u32(position + 8, 3);
        slot(&mut e, player.addr(), 0x1f4, eax(position));
        e.set_global(CAMERA_SPHERE_RADIUS, 1.25f32);
        let counter = e.global::<u32>(0x0126_8280);
        start_log(&mut e);
        fn_0094cae0(&mut e, player, Ptr::new(0x5000));
        // The sphere shape: allocated with 0x14, built by the constructor.
        let sizes: Vec<u32> = calls_to(&e, 0x00aa_13e0).iter().map(|w| w[0]).collect();
        assert_eq!(sizes, vec![0x14, 0x1c]);
        let sphere = calls_to(&e, SPHERE_BASE_CONSTRUCTOR)[0][0];
        assert_eq!(e.mem.u32(sphere), 0x0103_0cdc);
        assert_eq!(e.global::<u32>(0x0126_8280), counter + 1);
        assert_eq!(
            calls_to(&e, 0x0056_ea00),
            vec![vec![sphere, 1.25f32.to_bits()]]
        );
        // The rigid body info got the shape and the flags 0x30021.
        let cinfo = calls_to(&e, 0x00c8_f510)[0][0];
        assert_eq!(calls_to(&e, 0x0056_f110), vec![vec![cinfo, sphere]]);
        assert_eq!(calls_to(&e, 0x0056_f0f0), vec![vec![cinfo, 0x30021]]);
        // The body was built from it and stored; its motion byte is 9.
        let built = calls_to(&e, 0x0056_d380)[0][0];
        assert_eq!(e.mem.u32(player.addr() + 0xdec), built);
        assert_ne!(built, body);
        assert_eq!(e.mem.u8(motion + 0x1a), 9);
        // The body is placed at the player's position.
        let placed = calls_to(&e, 0x0056_10f0);
        assert_eq!(placed[0][0], built);
        assert_eq!(
            [
                e.mem.u32(placed[0][1]),
                e.mem.u32(placed[0][1] + 4),
                e.mem.u32(placed[0][1] + 8)
            ],
            [1, 2, 3]
        );
        assert_eq!(calls_to(&e, 0x0056_d730), vec![vec![cinfo]]);
    }

    // ----- small accessors and setters -----

    #[test]
    fn small_setters_and_getters() {
        let mut e = engine();
        let object = e.mem.alloc(0x40);
        fn_0094db80(&mut e, Ptr::new(object), 7);
        assert_eq!(e.mem.u8(object + 0x1a), 7);

        e.set_global(SKIP_WATER_HEIGHT_FLAG, 3u8);
        assert_eq!(fn_0094dba0(&mut e), 3);

        let player = new_player(&mut e);
        fn_00950010(&mut e, player, 1);
        assert_eq!(e.get(player, PlayerCharacter::bChargen), 1);

        e.set_global(VANITY_ACTIVE, 1u8);
        assert_eq!(fn_00950090(&mut e, player.cast()), 1);

        e.set_global(NODE_NAME_EAC0, 11u32);
        e.set_global(NODE_NAME_EAD0, 12u32);
        e.set_global(NODE_NAME_EAE0, 13u32);
        e.set_global(NODE_NAME_EB30, 14u32);
        assert_eq!(fn_0094eac0(&mut e), 11);
        assert_eq!(fn_0094ead0(&mut e), 12);
        assert_eq!(fn_0094eae0(&mut e), 13);
        assert_eq!(fn_0094eb30(&mut e), 14);
    }

    #[test]
    fn the_last_known_good_position_is_copied_out() {
        let mut e = engine();
        let player = new_player(&mut e);
        e.mem.set_f32(player.addr() + 0x7a0, 1.0);
        e.mem.set_f32(player.addr() + 0x7a4, 2.0);
        e.mem.set_f32(player.addr() + 0x7a8, 3.0);
        let out = e.mem.alloc(12);
        let result = fn_0094df30(&mut e, player, Ptr::new(out));
        assert_eq!(result.addr(), out);
        assert_eq!(
            [e.mem.f32(out), e.mem.f32(out + 4), e.mem.f32(out + 8)],
            [1.0, 2.0, 3.0]
        );
    }

    #[test]
    fn sleeping_or_resting_means_a_positive_sleep_time() {
        let mut e = engine();
        let player = new_player(&mut e);
        assert!(!player_character_is_sleepingor_resting(&mut e, player));
        e.set(player, PlayerCharacter::iSleepTime, 3);
        assert!(player_character_is_sleepingor_resting(&mut e, player));
        e.set(player, PlayerCharacter::iSleepTime, -2);
        assert!(!player_character_is_sleepingor_resting(&mut e, player));
    }

    #[test]
    fn the_camera_node_lookup_stores_the_found_node() {
        let mut e = engine();
        let object = e.mem.alloc(0x40);
        let seen = recording_slot(&mut e, object, 0x9c, eax(0x5151));
        e.set_global(NODE_NAME_EB30, 0x66u32);
        start_log(&mut e);
        fn_0094eaf0(&mut e, Ptr::NULL, Ptr::new(object), false);
        assert_eq!(e.global::<u32>(NODE_CAMERA_3RD_SLOT), 0x5151);
        assert_eq!(seen.borrow().clone(), vec![vec![object, 0x66]]);
        assert!(calls_to(&e, 0x0045_0f90).is_empty());
        // With the flag, the object is first told (with 1).
        fn_0094eaf0(&mut e, Ptr::NULL, Ptr::new(object), true);
        assert_eq!(calls_to(&e, 0x0045_0f90), vec![vec![object, 1]]);
    }

    #[test]
    fn forwarders_to_the_progression_object() {
        let mut e = engine();
        let player = new_player(&mut e);
        start_log(&mut e);
        fn_0094fce0(&mut e, player, 5, 6);
        fn_0094fd10(&mut e, player, 9);
        assert_eq!(
            calls_to(&e, PROCESS_LISTS_PRINT_LISTS),
            vec![vec![player.addr() + 0x878, 5, 6]]
        );
        assert_eq!(
            calls_to(&e, CHARACTER_PROGRESSION_REWARD_EXPERIENCE),
            vec![vec![player.addr() + 0x878, 9]]
        );
    }

    #[test]
    fn updating_the_player_3d_notifies_the_menu_only_when_asked() {
        let mut e = engine();
        let player = new_player(&mut e);
        let process = e.mem.alloc(0x40);
        e.set(player, PlayerCharacter::pCurrentProcess, Ptr::new(process));
        let busy = recording_slot(&mut e, process, 0x474, eax(1));
        let update = recording_slot(&mut e, process, 0x464, Ret::default());
        double(&mut e, INVENTORY_MENU_VISIBLE, eax(1));
        start_log(&mut e);
        // The process says yes: nothing to release.
        player_character_update_player_3d(&mut e, player);
        assert!(calls_to(&e, RELEASE_HANDLE).is_empty());
        assert_eq!(busy.borrow().clone(), vec![vec![process, 1]]);
        assert_eq!(update.borrow().clone(), vec![vec![process, player.addr()]]);
        // The process says no and the inventory is visible.
        let table = e.mem.u32(process);
        let no = NEXT_TARGET.fetch_add(4, Ordering::Relaxed);
        e.mem.set_u32(table + 0x474, no);
        double(&mut e, no, eax(0));
        player_character_update_player_3d(&mut e, player);
        assert_eq!(calls_to(&e, RELEASE_HANDLE), vec![vec![player.addr()]]);
        // ... and the inventory is not.
        double(&mut e, INVENTORY_MENU_VISIBLE, eax(0));
        player_character_update_player_3d(&mut e, player);
        assert_eq!(calls_to(&e, RELEASE_HANDLE).len(), 1);
        assert_eq!(update.borrow().len(), 3);
    }

    #[test]
    fn stopping_vanity_mode_restores_and_clears() {
        let mut e = engine();
        e.set_global(VANITY_ACTIVE, 1u8);
        e.set_global(VANITY_SAVED_FLAG, 1u8);
        e.set_global(VANITY_SAVED_VALUE, 42.5f32);
        e.set_global(VANITY_RESTORED_VALUE, 1.0f32);
        for address in [
            VANITY_VALUE_07C4,
            VANITY_VALUE_07C8,
            VANITY_VALUE_07DC,
            VANITY_VALUE_0B58,
            VANITY_VALUE_0B60,
        ] {
            e.set_global(address, 9.0f32);
        }
        e.set_global(VANITY_FLAG_07C1, 1u8);
        start_log(&mut e);
        player_character_stop_vanity_mode(&mut e, Ptr::NULL);
        assert_eq!(e.global::<f32>(VANITY_RESTORED_VALUE), 42.5);
        assert_eq!(e.global::<u8>(VANITY_ACTIVE), 0);
        assert_eq!(e.global::<u8>(VANITY_SAVED_FLAG), 0);
        assert_eq!(e.global::<u8>(VANITY_FLAG_07C3), 1);
        assert_eq!(e.global::<u8>(VANITY_FLAG_07C1), 0);
        for address in [
            VANITY_VALUE_07C4,
            VANITY_VALUE_07C8,
            VANITY_VALUE_07DC,
            VANITY_VALUE_0B58,
            VANITY_VALUE_0B60,
        ] {
            assert_eq!(e.global::<f32>(address), 0.0);
        }
        assert_eq!(calls_to(&e, SET_MENU_MODE), vec![vec![1]]);
        // Without the saved flag the restored value is left alone.
        e.set_global(VANITY_RESTORED_VALUE, 5.0f32);
        e.set_global(VANITY_SAVED_VALUE, 7.0f32);
        player_character_stop_vanity_mode(&mut e, Ptr::NULL);
        assert_eq!(e.global::<f32>(VANITY_RESTORED_VALUE), 5.0);
    }

    #[test]
    fn the_sphere_shape_constructor_installs_its_vtable_and_counts() {
        let mut e = engine();
        let object = e.mem.alloc(0x14);
        e.set_global(0x0126_8280u32, 4u32);
        start_log(&mut e);
        let result = fn_0094dbb0(&mut e, Ptr::new(object));
        assert_eq!(result.addr(), object);
        assert_eq!(e.mem.u32(object), 0x0103_0cdc);
        assert_eq!(e.global::<u32>(0x0126_8280), 5);
        assert_eq!(calls_to(&e, SPHERE_BASE_CONSTRUCTOR), vec![vec![object]]);
    }

    // ----- ReturnToLastKnownGoodPosition -----

    #[test]
    fn nothing_happens_without_a_last_known_good_location() {
        let mut e = engine();
        let player = new_player(&mut e);
        start_log(&mut e);
        player_character_return_to_last_known_good_position(&mut e, player, true);
        player_character_return_to_last_known_good_position(&mut e, player, false);
        assert!(calls_to(&e, 0x0070_52f0).is_empty());
    }

    #[test]
    fn returning_with_the_position_functions_picks_the_world_space_function() {
        let mut e = engine();
        let player = new_player(&mut e);
        e.set_global(MESSAGE_SECONDS, 2.0f32);
        e.mem.set_f32(player.addr() + 0x7a0, 10.0);
        e.mem.set_f32(player.addr() + 0x7a4, 20.0);
        e.mem.set_f32(player.addr() + 0x7a8, 30.0);
        e.mem.set_u32(player.addr() + 0x7ac, 0x6000);
        e.register(0x0040_3df0, |_, a| eax(a[0] + 4));
        let wanted = e.mem.alloc(12);
        e.mem.set_f32(wanted, 1.0);
        e.mem.set_f32(wanted + 4, 2.0);
        e.mem.set_f32(wanted + 8, 3.0);
        double(&mut e, 0x0043_0830, eax(wanted));
        // A cell (type 0x39) uses PositionPlayer.
        double(&mut e, FORM_TYPE_OF, eax(0x39));
        start_log(&mut e);
        player_character_return_to_last_known_good_position(&mut e, player, true);
        let known = [10.0f32.to_bits(), 20.0f32.to_bits(), 30.0f32.to_bits()];
        let wanted_bits = [1.0f32.to_bits(), 2.0f32.to_bits(), 3.0f32.to_bits()];
        let mut expected = vec![player.addr()];
        expected.extend(known);
        expected.extend(wanted_bits);
        expected.extend([0x6000, 0]);
        assert_eq!(calls_to(&e, 0x0093_c200), vec![expected.clone()]);
        assert!(calls_to(&e, 0x0093_cce0).is_empty());
        // The message: text, 0, icon, 0, 2.0 seconds, 0.
        assert_eq!(
            calls_to(&e, 0x0070_52f0),
            vec![vec![
                RETURN_MESSAGE_SETTING + 4,
                0,
                SAD_ICON_PATH,
                0,
                2.0f32.to_bits(),
                0
            ]]
        );
        // A world space (type 0x41) uses PositionPlayerExterior.
        double(&mut e, FORM_TYPE_OF, eax(0x41));
        player_character_return_to_last_known_good_position(&mut e, player, true);
        assert_eq!(calls_to(&e, 0x0093_cce0), vec![expected]);
        // Any other form does not move the player.
        double(&mut e, FORM_TYPE_OF, eax(0x10));
        player_character_return_to_last_known_good_position(&mut e, player, true);
        assert_eq!(calls_to(&e, 0x0093_c200).len(), 1);
        assert_eq!(calls_to(&e, 0x0093_cce0).len(), 1);
    }

    #[test]
    fn returning_directly_sets_the_position_and_resets_the_bodies() {
        let mut e = engine();
        let player = new_player(&mut e);
        let cell = 0x6000;
        e.mem.set_u32(player.addr() + 0x7ac, cell);
        e.mem.set_f32(player.addr() + 0x7a0, 8192.0);
        e.mem.set_f32(player.addr() + 0x7a4, 12288.5);
        e.register(0x0040_3df0, |_, a| eax(a[0] + 4));
        double(&mut e, FORM_TYPE_OF, eax(0x39));
        let controller = 0x9100;
        double(&mut e, 0x0093_06d0, eax(controller));
        let ground = e.mem.alloc(16);
        e.mem.set_f32(ground + 8, 55.0);
        double(&mut e, 0x0043_6aa0, eax(ground));
        let move_calls = recording_slot(&mut e, player.addr(), 0x228, Ret::default());
        let bodies = [0xb100, 0xb200];
        e.register_double(0x0095_0bb0, move |_, a| eax(bodies[a[1] as usize]));
        start_log(&mut e);
        player_character_return_to_last_known_good_position(&mut e, player, false);
        // No water: the height comes from the reference position.
        assert_eq!(e.mem.f32(player.addr() + 0x7a8), 55.0);
        let position = player.addr() + 0x7a0;
        assert_eq!(
            calls_to(&e, 0x0057_5830),
            vec![vec![player.addr(), position]]
        );
        assert_eq!(calls_to(&e, 0x0056_20e0), vec![vec![controller, position]]);
        // The location is a cell: the player is moved into it, twice.
        assert_eq!(
            move_calls.borrow().clone(),
            vec![vec![player.addr(), cell]; 2]
        );
        for body in bodies {
            assert_eq!(
                calls_to(&e, 0x0044_0460)
                    .iter()
                    .filter(|w| w[0] == body)
                    .count(),
                1
            );
            assert!(calls_to(&e, 0x00c6_bd00).contains(&vec![body, 1]));
            assert_eq!(
                calls_to(&e, 0x00a5_9c60)
                    .iter()
                    .filter(|w| w[0] == body)
                    .count(),
                1
            );
        }
    }

    #[test]
    fn returning_directly_to_a_world_space_finds_the_cell_from_the_position() {
        let mut e = engine();
        let player = new_player(&mut e);
        let world = 0x6000;
        e.mem.set_u32(player.addr() + 0x7ac, world);
        e.mem.set_f32(player.addr() + 0x7a0, 8192.0);
        e.mem.set_f32(player.addr() + 0x7a4, 12288.5);
        e.register(0x0040_3df0, |_, a| eax(a[0] + 4));
        double(&mut e, FORM_TYPE_OF, eax(0x41));
        // Water: the height is left alone.
        double(&mut e, 0x0093_06d0, eax(0x9100));
        double(&mut e, 0x0088_b0c0, eax(1));
        e.register(0x0040_6d90, |_, a| eax(f32::from_bits(a[0]) as i32 as u32));
        e.set_global(0x011c_3f2cu32, 0x3000u32);
        double(&mut e, 0x0046_1c20, eax(0x7000));
        let move_calls = recording_slot(&mut e, player.addr(), 0x228, Ret::default());
        e.mem.set_f32(player.addr() + 0x7a8, 5.0);
        start_log(&mut e);
        player_character_return_to_last_known_good_position(&mut e, player, false);
        assert_eq!(e.mem.f32(player.addr() + 0x7a8), 5.0);
        assert_eq!(
            calls_to(&e, 0x0046_1c20),
            vec![vec![0x3000, 2, 3, world, 0]]
        );
        assert_eq!(
            move_calls.borrow().clone(),
            vec![vec![player.addr(), 0x7000]; 2]
        );
    }

    // ----- the one-hour wait step -----

    /// A player whose actor value owner answers `0xc` with `value` and whose
    /// slot `0x3a4` records its calls.
    fn waiting_player(e: &mut Engine, value: f64) -> (Ptr<PlayerCharacter>, CallRecord) {
        let player = new_player(e);
        slot(e, player.addr() + 0xa4, 0xc, st0(value));
        let changes = recording_slot(e, player.addr(), 0x3a4, Ret::default());
        e.set_global(SECONDS_PER_HOUR, 3600.0f64);
        e.set_global(PLAYER_POINTER, 0x1111u32);
        e.set_global(SKY_HOLDER_POINTER, 0x2222u32);
        e.set_global(WAIT_UPDATE_TIME, 1.5f32);
        double(e, 0x0045_3a70, eax(0x1234));
        double(e, 0x0086_7950, st0(2.0));
        double(e, 0x0096_d490, st0(100.0));
        double(e, 0x0084_d030, st0(0.25));
        double(e, 0x008d_8520, eax(0x4000));
        let limit = e.mem.alloc(4);
        e.mem.set_f32(limit, 50.0);
        e.register_double(SETTING_FLOAT_POINTER, move |_, _| eax(limit));
        (player, changes)
    }

    #[test]
    fn an_hour_passes_on_the_clock_the_calendar_and_the_sky() {
        let mut e = engine();
        let (player, _) = waiting_player(&mut e, 80.0);
        e.set(player, PlayerCharacter::iSleepTime, 3);
        start_log(&mut e);
        fn_0094df80(&mut e, player);
        // One hour at time scale 2 is 1800 seconds.
        assert_eq!(calls_to(&e, 0x00ad_8780), vec![vec![0x1234, 4]]);
        assert_eq!(calls_to(&e, 0x0081_5b00), vec![vec![player.addr() + 0x88]]);
        assert_eq!(
            calls_to(&e, 0x0096_d4b0),
            vec![vec![PROCESS_LISTS, 1900.0f32.to_bits()]]
        );
        assert_eq!(
            calls_to(&e, 0x0086_7a40),
            vec![vec![CALENDAR, 1800.0f32.to_bits()]]
        );
        // The wait is not active: no updates of the magic or the lists.
        assert!(calls_to(&e, 0x0088_b510).is_empty());
        assert!(calls_to(&e, 0x0040_fbf0).is_empty());
        // The sky and the lists are updated, the countdown goes on.
        assert_eq!(
            calls_to(&e, 0x0063_ac70),
            vec![vec![0x4000, 0.25f32.to_bits()]]
        );
        assert_eq!(calls_to(&e, 0x0097_23f0), vec![vec![PROCESS_LISTS]]);
        assert_eq!(calls_to(&e, 0x008c_3c40), vec![vec![player.addr(), 1, 0]]);
        assert_eq!(e.get(player, PlayerCharacter::iSleepTime), 2);
        assert_eq!(e.global::<u8>(WAIT_STEP_ACTIVE), 0);
    }

    #[test]
    fn an_active_wait_updates_the_process_lists_under_the_lock() {
        let mut e = engine();
        let (player, changes) = waiting_player(&mut e, 80.0);
        e.set_global(WAIT_STEP_ACTIVE, 1u8);
        e.set_global(PROCESS_LOCK_ENABLED, 1u8);
        e.set(player, PlayerCharacter::iSleepTime, 3);
        start_log(&mut e);
        fn_0094df80(&mut e, player);
        let log: Vec<u32> = e
            .call_log
            .as_ref()
            .unwrap()
            .iter()
            .map(|(address, _)| *address)
            .collect();
        let at = |address: u32| log.iter().position(|a| *a == address).unwrap();
        assert!(at(0x0040_fbf0) < at(0x0096_bcd0));
        assert!(at(0x0096_bcd0) < at(0x0096_b810));
        assert!(at(0x0096_b810) < at(0x0096_b470));
        assert!(at(0x0096_b470) < at(0x0096_b050));
        assert!(at(0x0096_b050) < at(0x0040_fba0));
        assert_eq!(calls_to(&e, 0x0040_fbf0), vec![vec![PROCESS_LOCK, 0]]);
        for address in [0x0096_bcd0, 0x0096_b810, 0x0096_b470, 0x0096_b050] {
            assert_eq!(
                calls_to(&e, address),
                vec![vec![PROCESS_LISTS, 1.5f32.to_bits(), 1]]
            );
        }
        // Health is restored on the player global.
        assert_eq!(
            calls_to(&e, 0x0088_b510),
            vec![vec![0x1111, 1800.0f32.to_bits()]]
        );
        assert!(calls_to(&e, 0x0082_3c40).is_empty());
        assert!(changes.borrow().is_empty());
    }

    #[test]
    fn an_active_wait_without_the_lock_flag_skips_the_list_updates() {
        let mut e = engine();
        let (player, _) = waiting_player(&mut e, 80.0);
        e.set_global(WAIT_STEP_ACTIVE, 1u8);
        e.set(player, PlayerCharacter::iSleepTime, 3);
        start_log(&mut e);
        fn_0094df80(&mut e, player);
        assert!(calls_to(&e, 0x0040_fbf0).is_empty());
        assert!(calls_to(&e, 0x0096_bcd0).is_empty());
        assert_eq!(calls_to(&e, 0x0088_b510).len(), 1);
    }

    #[test]
    fn an_active_wait_with_a_magic_target_updates_it() {
        let mut e = engine();
        let (player, _) = waiting_player(&mut e, 80.0);
        e.set_global(WAIT_STEP_ACTIVE, 1u8);
        double(&mut e, 0x005c_7870, eax(1));
        e.set(player, PlayerCharacter::iSleepTime, 3);
        start_log(&mut e);
        fn_0094df80(&mut e, player);
        assert_eq!(
            calls_to(&e, 0x0082_3c40),
            vec![vec![player.addr() + 0x94, 1800.0f32.to_bits()]]
        );
        assert!(calls_to(&e, 0x0096_bcd0).is_empty());
    }

    #[test]
    fn a_waiting_player_loses_the_smaller_of_the_value_and_the_limit() {
        let mut e = engine();
        let (player, changes) = waiting_player(&mut e, 80.0);
        e.set_global(WAIT_STEP_ACTIVE, 1u8);
        double(&mut e, 0x004d_1360, eax(1));
        double(&mut e, 0x005a_1e50, eax(1));
        e.set(player, PlayerCharacter::iSleepTime, 3);
        fn_0094df80(&mut e, player);
        assert_eq!(
            changes.borrow().clone(),
            vec![vec![player.addr(), 0x4b, (-50.0f32).to_bits(), 0]]
        );
        // A value below the limit is used as it is.
        let (player, changes) = waiting_player(&mut e, 20.0);
        e.set_global(WAIT_STEP_ACTIVE, 1u8);
        e.set(player, PlayerCharacter::iSleepTime, 3);
        fn_0094df80(&mut e, player);
        assert_eq!(
            changes.borrow().clone(),
            vec![vec![player.addr(), 0x4b, (-20.0f32).to_bits(), 0]]
        );
    }

    #[test]
    fn the_last_hour_ends_the_wait_and_restores_a_sleeper() {
        let mut e = engine();
        let (player, _) = waiting_player(&mut e, 80.0);
        e.set(player, PlayerCharacter::iSleepTime, 1);
        e.set(player, PlayerCharacter::bIsSleeping, true);
        start_log(&mut e);
        fn_0094df80(&mut e, player);
        assert_eq!(e.get(player, PlayerCharacter::iSleepTime), 0);
        assert_eq!(e.global::<u8>(WAIT_STEP_ACTIVE), 1);
        assert!(!e.get(player, PlayerCharacter::bIsSleeping));
        assert_eq!(calls_to(&e, 0x008a_0960), vec![vec![player.addr()]]);
        // A player in the state 004d1360 describes is not restored.
        let (player, _) = waiting_player(&mut e, 80.0);
        e.set(player, PlayerCharacter::iSleepTime, 1);
        e.set(player, PlayerCharacter::bIsSleeping, true);
        double(&mut e, 0x004d_1360, eax(1));
        fn_0094df80(&mut e, player);
        assert_eq!(calls_to(&e, 0x008a_0960).len(), 1);
        assert!(!e.get(player, PlayerCharacter::bIsSleeping));
    }

    // ----- karma -----

    fn int_setting_slot(setting: u32) -> u32 {
        0x0600_1000 + (setting & 0xfff) * 4
    }

    fn karma_world(e: &mut Engine, karma: i32) -> (Ptr<PlayerCharacter>, CallRecord) {
        let player = new_player(e);
        slot(e, player.addr() + 0xa4, 0x8, eax(karma as u32));
        slot(e, player.addr() + 0xa4, 0xc, st0(karma as f64));
        let changes = recording_slot(e, player.addr(), 0x3a8, Ret::default());
        e.register(SETTING_INT_POINTER, |_, a| eax(int_setting_slot(a[0])));
        e.register(SETTING_TEXT, |_, a| eax(a[0] + 4));
        e.set_global(MESSAGE_SECONDS, 2.0f32);
        for (setting, value) in [
            (KARMA_LOWER_LIMIT_SETTING, -1000i32),
            (KARMA_UPPER_LIMIT_SETTING, 1000),
            (KARMA_MESSAGE_THRESHOLD_SETTING, 100),
        ] {
            e.mem.set_i32(int_setting_slot(setting), value);
        }
        double(e, 0x0045_3a70, eax(0x1234));
        e.register(0x00ad_7550, |_, a| eax(a[1]));
        (player, changes)
    }

    fn message_of(e: &Engine) -> Vec<u32> {
        let messages = calls_to(e, 0x0070_52f0);
        assert_eq!(messages.len(), 1);
        messages[0].clone()
    }

    #[test]
    fn a_small_karma_loss_shows_the_small_message_and_plays_the_down_sound() {
        let mut e = engine();
        let (player, changes) = karma_world(&mut e, 10);
        start_log(&mut e);
        player_character_reward_karma(&mut e, player, -50);
        assert_eq!(
            message_of(&e),
            vec![
                KARMA_LOSS_TEXT_SETTING + 4,
                0,
                KARMA_LOSS_ICON_SETTING + 4,
                0,
                2.0f32.to_bits(),
                0
            ]
        );
        assert_eq!(
            changes.borrow().clone(),
            vec![vec![player.addr(), 0x17, (-50i32) as u32, 0]]
        );
        let lookups = calls_to(&e, 0x00ad_7550);
        assert_eq!(lookups.len(), 1);
        assert_eq!(lookups[0][0], 0x1234);
        assert_eq!(lookups[0][2], SOUND_KARMA_DOWN);
        assert_eq!(lookups[0][3], 0x121);
        // The sound is played from the handle and both handles are released.
        let handle = calls_to(&e, 0x0041_a250)[0][0];
        assert_eq!(calls_to(&e, 0x00ad_8830), vec![vec![handle, 0]]);
        assert_eq!(calls_to(&e, RELEASE_HANDLE).len(), 2);
        assert_eq!(calls_to(&e, 0x0041_8900)[0][0], handle);
        assert_eq!(calls_to(&e, 0x0047_e040).len(), 1);
    }

    #[test]
    fn a_big_karma_loss_shows_the_big_message() {
        let mut e = engine();
        let (player, changes) = karma_world(&mut e, 10);
        start_log(&mut e);
        player_character_reward_karma(&mut e, player, -200);
        assert_eq!(
            message_of(&e)[..3],
            [
                KARMA_LOSS_BIG_TEXT_SETTING + 4,
                0,
                KARMA_LOSS_BIG_ICON_SETTING + 4
            ]
        );
        assert_eq!(changes.borrow()[0][2], (-200i32) as u32);
    }

    #[test]
    fn a_karma_loss_stops_at_the_lower_limit() {
        let mut e = engine();
        let (player, changes) = karma_world(&mut e, 10);
        start_log(&mut e);
        player_character_reward_karma(&mut e, player, -5000);
        // The change is cut to reach exactly the limit: -1000 - 10.
        assert_eq!(changes.borrow()[0][2], (-1010i32) as u32);
        assert_eq!(
            message_of(&e)[..3],
            [
                KARMA_LOSS_BIG_TEXT_SETTING + 4,
                0,
                KARMA_LOSS_BIG_ICON_SETTING + 4
            ]
        );
    }

    #[test]
    fn karma_gains_use_the_up_sound_and_the_upper_limit() {
        let mut e = engine();
        let (player, changes) = karma_world(&mut e, 10);
        start_log(&mut e);
        player_character_reward_karma(&mut e, player, 30);
        assert_eq!(
            message_of(&e)[..3],
            [KARMA_GAIN_TEXT_SETTING + 4, 0, KARMA_GAIN_ICON_SETTING + 4]
        );
        assert_eq!(calls_to(&e, 0x00ad_7550)[0][2], SOUND_KARMA_UP);
        assert_eq!(changes.borrow()[0][2], 30);

        let (player, changes) = karma_world(&mut e, 10);
        start_log(&mut e);
        player_character_reward_karma(&mut e, player, 100);
        assert_eq!(
            message_of(&e)[..3],
            [
                KARMA_GAIN_BIG_TEXT_SETTING + 4,
                0,
                KARMA_GAIN_BIG_ICON_SETTING + 4
            ]
        );
        assert_eq!(changes.borrow()[0][2], 100);

        let (player, changes) = karma_world(&mut e, 990);
        start_log(&mut e);
        player_character_reward_karma(&mut e, player, 500);
        assert_eq!(changes.borrow()[0][2], 10);
        let _ = player;
    }

    #[test]
    fn a_zero_karma_reward_still_reports_a_gain() {
        let mut e = engine();
        let (player, changes) = karma_world(&mut e, 10);
        start_log(&mut e);
        player_character_reward_karma(&mut e, player, 0);
        assert_eq!(calls_to(&e, 0x00ad_7550)[0][2], SOUND_KARMA_UP);
        assert_eq!(changes.borrow()[0][2], 0);
    }

    // ----- the inventory callback -----

    #[test]
    fn the_inventory_callback_appends_one_entry() {
        let mut e = engine();
        list_helpers(&mut e);
        let item = e.mem.alloc(0x20);
        e.mem.set_u32(item + 4, 3);
        let output = e.mem.alloc(8);
        double(&mut e, ITEM_CHANGE_GET_WORN, eax(1));
        e.register(WORD_AT_8, |_, a| eax(a[0] + 0x100));
        e.register(FORM_ID_OF, |_, a| eax(a[0] + 1));
        double(&mut e, ITEM_CHANGE_GET_FULL_NAME, eax(0x5050));
        start_log(&mut e);
        assert!(!build_inv_string_callback(
            &mut e,
            Ptr::new(item),
            Ptr::new(output)
        ));
        let format = calls_to(&e, SPRINTF);
        assert_eq!(format.len(), 1);
        assert_eq!(
            format[0][1..],
            [
                0x104,
                FORMAT_INVENTORY_ITEM,
                INVENTORY_PREFIX_WORN,
                3,
                0x5050,
                item + 0x100 + 1
            ]
        );
        let buffer = format[0][0];
        assert_eq!(calls_to(&e, STRING_APPEND), vec![vec![output, buffer]]);
        // A carried item gets the other prefix.
        double(&mut e, ITEM_CHANGE_GET_WORN, eax(0));
        assert!(!build_inv_string_callback(
            &mut e,
            Ptr::new(item),
            Ptr::new(output)
        ));
        assert_eq!(calls_to(&e, SPRINTF)[1][3], INVENTORY_PREFIX_CARRIED);
    }

    #[test]
    fn the_inventory_callback_ignores_missing_arguments() {
        let mut e = engine();
        start_log(&mut e);
        assert!(!build_inv_string_callback(
            &mut e,
            Ptr::NULL,
            Ptr::new(0x1000)
        ));
        assert!(!build_inv_string_callback(
            &mut e,
            Ptr::new(0x1000),
            Ptr::NULL
        ));
        assert!(calls_to(&e, SPRINTF).is_empty());
    }

    // ----- Set3D -----

    #[test]
    fn setting_no_3d_before_the_game_is_over_is_refused() {
        let mut e = engine();
        let player = new_player(&mut e);
        e.set_global(GAME_STATE_POINTER, 0x4444u32);
        start_log(&mut e);
        fn_0094eb40(&mut e, player, Ptr::NULL, 1);
        assert_eq!(calls_to(&e, LOG_MESSAGE), vec![vec![FORMAT_SET3D_MESSAGE]]);
        assert!(calls_to(&e, REFERENCE_SET_3D).is_empty());
        // The game being over lets it through.
        double(&mut e, GAME_STATE_TEST_2, eax(1));
        fn_0094eb40(&mut e, player, Ptr::NULL, 1);
        assert_eq!(calls_to(&e, LOG_MESSAGE).len(), 1);
        assert_eq!(calls_to(&e, REFERENCE_SET_3D).len(), 1);
        // So does a missing state object or a real node.
        double(&mut e, GAME_STATE_TEST_2, eax(0));
        double(&mut e, GAME_STATE_TEST_1, eax(0));
        fn_0094eb40(&mut e, player, Ptr::new(0x1234), 0);
        assert_eq!(calls_to(&e, LOG_MESSAGE).len(), 1);
        assert_eq!(
            calls_to(&e, REFERENCE_SET_3D).last().unwrap().clone(),
            vec![player.addr(), 0x1234, 0]
        );
    }

    #[test]
    fn setting_a_real_3d_changes_nothing_else() {
        let mut e = engine();
        let player = new_player(&mut e);
        e.set(
            player,
            PlayerCharacter::p1stPersonBipedAnim,
            Ptr::new(0x1000),
        );
        start_log(&mut e);
        fn_0094eb40(&mut e, player, Ptr::new(0x2000), 1);
        assert_eq!(
            calls_to(&e, REFERENCE_SET_3D),
            vec![vec![player.addr(), 0x2000, 1]]
        );
        assert!(calls_to(&e, DELETE_BIPED_ANIM).is_empty());
        assert_eq!(
            e.get(player, PlayerCharacter::p1stPersonBipedAnim).addr(),
            0x1000
        );
    }

    #[test]
    fn clearing_the_3d_releases_the_first_person_objects() {
        let mut e = engine();
        let player = new_player(&mut e);
        e.set(
            player,
            PlayerCharacter::p1stPersonBipedAnim,
            Ptr::new(0x1000),
        );
        e.set(
            player,
            PlayerCharacter::p1stPersonAnimation,
            Ptr::new(0x2000),
        );
        e.set_global(NODE_CAMERA_1ST_SLOT, 1u32);
        e.set_global(NODE_CAMERA_3RD_SLOT, 2u32);
        e.set_global(NODE_BIP_SLOT, 3u32);
        e.set_global(MODEL_LOADER_POINTER, 0x3000u32);
        e.register(SETTING_TEXT, |_, a| eax(a[0] + 4));
        e.register(IDENTITY, |_, a| eax(a[0]));
        start_log(&mut e);
        fn_0094eb40(&mut e, player, Ptr::NULL, 0);
        assert_eq!(calls_to(&e, DELETE_BIPED_ANIM), vec![vec![0x1000, 1]]);
        assert_eq!(calls_to(&e, DELETE_ANIMATION), vec![vec![0x2000, 1]]);
        assert_eq!(
            e.get(player, PlayerCharacter::p1stPersonBipedAnim).addr(),
            0
        );
        assert_eq!(
            e.get(player, PlayerCharacter::p1stPersonAnimation).addr(),
            0
        );
        assert_eq!(
            calls_to(&e, SMART_POINTER_ASSIGN),
            vec![vec![player.addr() + 0x694, 0]]
        );
        assert_eq!(e.global::<u32>(NODE_CAMERA_1ST_SLOT), 0);
        assert_eq!(e.global::<u32>(NODE_CAMERA_3RD_SLOT), 0);
        assert_eq!(e.global::<u32>(NODE_BIP_SLOT), 0);
        assert_eq!(
            calls_to(&e, MEMSET),
            vec![vec![player.addr() + 0xd74, 0, 0x60]]
        );
        assert_eq!(
            calls_to(&e, MODEL_LOADER_NOTIFY),
            vec![vec![0x3000, MODEL_PATH_SETTING + 4]]
        );
    }

    // ----- Load3D -----

    /// An object whose slot `0x9c` finds a node by name: it answers
    /// `0x1000 + name`, or 0 for the names in `missing`.
    fn finder(e: &mut Engine, missing: Vec<u32>) -> u32 {
        let object = e.mem.alloc(0x40);
        let table = e.mem.alloc(0x800);
        e.mem.set_u32(object, table);
        let target = NEXT_TARGET.fetch_add(4, Ordering::Relaxed);
        e.mem.set_u32(table + 0x9c, target);
        e.register_double(target, move |_, a| {
            if missing.contains(&a[1]) {
                Ret::default()
            } else {
                eax(0x1000 + a[1])
            }
        });
        object
    }

    struct LoadWorld {
        player: Ptr<PlayerCharacter>,
        model: u32,
        node: u32,
        inventory: u32,
        entry_calls: Rc<RefCell<Vec<Vec<u32>>>>,
        proxy_calls: Rc<RefCell<Vec<Vec<u32>>>>,
        process_calls: Rc<RefCell<Vec<Vec<u32>>>>,
        third_person_during_base: Rc<RefCell<Vec<u8>>>,
        file_lists: Rc<RefCell<Vec<String>>>,
    }

    fn load_world(e: &mut Engine, missing_first_person: Vec<u32>) -> LoadWorld {
        let player = new_player(e);
        e.set_global(NODE_NAME_EAD0, 0x11u32);
        e.set_global(NODE_NAME_EAE0, 0x12u32);
        e.set_global(NODE_NAME_EAC0, 0x13u32);
        e.set_global(NODE_NAME_EB30, 0x14u32);
        e.set_global(MODEL_LOADER_POINTER, 0x3000u32);
        e.set_global(IDLE_MANAGER_POINTER, 0x3100u32);
        e.mem.set_cstr(
            MODEL_PATH_SETTING + 4,
            b"Meshes\\Characters\\_1stPerson\\1stPersonMale.nif",
        );
        e.mem
            .set_cstr(TODDLER_SUFFIX, b"\\Locomotion\\Toddler\\IdleAnims");
        e.mem
            .set_cstr(FEMALE_SUFFIX, b"\\Locomotion\\Female\\IdleAnims");
        e.mem
            .set_cstr(MALE_SUFFIX, b"\\Locomotion\\Male\\IdleAnims");
        for word in 0..9u32 {
            e.mem.set_u32(LOAD_3D_MATRIX + 4 * word, 0x100 + word);
        }
        e.register(SETTING_TEXT, |_, a| eax(a[0] + 4));
        e.register(IDENTITY, |_, a| eax(a[0]));
        e.register(OPERATOR_NEW, |e, a| eax(e.mem.alloc(a[0])));
        e.register(SMART_POINTER_ASSIGN, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            Ret::default()
        });
        e.register(0x0055_9450, |e, a| eax(e.mem.u32(a[0])));
        list_helpers(e);
        double(e, 0x007a_f430, eax(0xf0f0));
        let inventory = e.mem.alloc(0x40);
        double(e, 0x004b_f220, eax(inventory));
        let model = finder(e, missing_first_person);
        double(e, 0x0044_7080, eax(model));
        let eye = e.mem.alloc(16);
        e.mem.set_f32(eye + 8, 1.75);
        double(e, 0x0045_bb80, eax(eye));
        e.register(0x0056_fac0, |_, a| eax(a[1]));
        e.register(0x004a_aca0, |_, a| eax(a[0]));
        e.register(0x0048_f810, |_, a| eax(a[0]));
        let node = finder(e, vec![]);
        let third_person_during_base = Rc::new(RefCell::new(Vec::new()));
        let seen = third_person_during_base.clone();
        let player_address = player.addr();
        e.register_double(0x0087_e060, move |e, _| {
            seen.borrow_mut().push(e.mem.u8(player_address + 0x64a));
            eax(node)
        });
        double(e, 0x0095_0a60, eax(0x8800));
        double(e, 0x0044_7330, eax(0x9900));
        e.register(0x0040_6d30, |e, a| {
            let text = e.mem.cstr(a[2]);
            e.mem.set_cstr(a[0], &text);
            eax(a[0])
        });
        e.register(0x0040_ab30, |e, a| {
            let text = e.mem.cstr(a[0]);
            match text.iter().rposition(|byte| *byte as u32 == a[1]) {
                Some(at) => eax(a[0] + at as u32),
                None => eax(0),
            }
        });
        let file_lists = Rc::new(RefCell::new(Vec::new()));
        let lists = file_lists.clone();
        e.register_double(0x0060_0700, move |e, a| {
            lists
                .borrow_mut()
                .push(String::from_utf8(e.mem.cstr(a[1])).unwrap());
            eax(0x5500)
        });
        let process = e.mem.alloc(0x40);
        e.set(player, PlayerCharacter::pCurrentProcess, Ptr::new(process));
        let process_calls = recording_slot(e, process, 0x470, Ret::default());
        slot(e, process, 0x148, eax(0x77));
        let entry_calls = recording_slot(e, process, 0x160, Ret::default());
        double(e, 0x008d_8520, eax(process));
        quiet_slots(e, player.addr(), &[0x1c4]);
        let proxy = e.mem.alloc(0x40);
        let proxy_calls = recording_slot(e, proxy, 0xd0, Ret::default());
        double(e, 0x0093_06d0, eax(0x6600));
        double(e, 0x0062_1ad0, eax(proxy));
        e.register(0x0070_c440, |_, a| eax(a[1]));
        double(e, 0x004a_3a20, eax(0x55));
        let bipeds: Vec<u32> = (0..2u32)
            .map(|which| {
                let holder = e.mem.alloc(8);
                e.mem.set_u32(holder, 0xc0 + which);
                holder
            })
            .collect();
        e.register_double(0x0095_0b00, move |_, a| eax(bipeds[a[1] as usize]));
        LoadWorld {
            player,
            model,
            node,
            inventory,
            entry_calls,
            proxy_calls,
            process_calls,
            third_person_during_base,
            file_lists,
        }
    }

    #[test]
    fn loading_the_3d_builds_the_first_person_model() {
        let mut e = engine();
        let world = load_world(&mut e, vec![]);
        let player = world.player;
        e.set(player, PlayerCharacter::b3rdPerson, false);
        start_log(&mut e);
        let result = player_character_load_3d(&mut e, player, 5);
        assert_eq!(result, world.node);

        // The scope object is opened with the source line and closed again.
        let scope = calls_to(&e, 0x0040_4eb0);
        assert_eq!(scope[0][1..], [0x34, 1, SOURCE_FILE_NAME, 0x2458]);
        assert_eq!(calls_to(&e, 0x0040_4ee0), vec![vec![scope[0][0]]]);

        // The model comes from the model-path setting.
        assert_eq!(
            calls_to(&e, 0x0044_7080),
            vec![vec![0x3000, MODEL_PATH_SETTING + 4, 3, 1, 0, 0, 0]]
        );
        assert_eq!(
            e.get(player, PlayerCharacter::sp1stPerson3D).addr(),
            world.model
        );
        assert_eq!(calls_to(&e, 0x0054_68b0), vec![vec![world.model, 1]]);

        // The named nodes were found in the model.
        assert_eq!(e.global::<u32>(NODE_BY_EAD0_SLOT), 0x1011);
        assert_eq!(e.global::<u32>(NODE_CAMERA_1ST_SLOT), 0x1012);
        assert_eq!(e.global::<u32>(NODE_BIP_SLOT), 0x1013);
        // ... and the third person camera in the 3D the base returned.
        assert_eq!(e.global::<u32>(NODE_CAMERA_3RD_SLOT), 0x1014);
        assert!(calls_to(&e, ERROR).is_empty());

        // The eye height comes from the first person camera node.
        assert_eq!(e.get(player, PlayerCharacter::fEyeHeight), 1.75);

        // The model is rotated by the matrix copied from the exe.
        let rotate = calls_to(&e, 0x0056_fac0);
        assert_eq!(rotate.len(), 1);
        let matrix: Vec<u32> = (0..9)
            .map(|word| e.mem.u32(rotate[0][2] + 4 * word))
            .collect();
        assert_eq!(
            matrix,
            (0..9).map(|word| 0x100 + word).collect::<Vec<u32>>()
        );
        assert_eq!(
            calls_to(&e, 0x0043_fa80),
            vec![vec![world.model, rotate[0][1]]]
        );

        // The objects were built with the right sizes and stored.
        let biped = e.get(player, PlayerCharacter::p1stPersonBipedAnim).addr();
        let animation = e.get(player, PlayerCharacter::p1stPersonAnimation).addr();
        assert_eq!(
            calls_to(&e, 0x004a_aca0),
            vec![vec![biped, player.addr(), world.model]]
        );
        assert_eq!(calls_to(&e, 0x0048_f810), vec![vec![animation]]);
        assert_eq!(e.mem.block_size(biped), Some(0x2b8));
        assert_eq!(e.mem.block_size(animation), Some(0x140));

        // The base load runs with the third person flag on, then restores it.
        assert_eq!(world.third_person_during_base.borrow().clone(), vec![1]);
        assert!(!e.get(player, PlayerCharacter::b3rdPerson));
        assert_eq!(calls_to(&e, 0x0087_e060), vec![vec![player.addr(), 5]]);

        // The animation group is cleared and the file list built.
        assert_eq!(calls_to(&e, 0x0095_0a60), vec![vec![player.addr(), 0]]);
        assert_eq!(calls_to(&e, 0x0049_6080), vec![vec![0x8800, 0, 0]]);
        assert_eq!(
            calls_to(&e, 0x0044_7330),
            vec![vec![0x3000, MODEL_PATH_SETTING + 4, 1, 0, 0xc]]
        );
        // Not a toddler, sex 0: only the male idle animations.
        assert_eq!(
            world.file_lists.borrow().clone(),
            vec!["Meshes\\Characters\\_1stPerson\\Locomotion\\Male\\IdleAnims".to_string()]
        );
        assert_eq!(
            calls_to(&e, 0x0044_7850),
            vec![vec![0x3000, 0x5500, 0x9900]]
        );
        assert_eq!(
            calls_to(&e, 0x0048_ffd0),
            vec![vec![animation, 0x9900, world.model, player.addr(), 1]]
        );

        // The process: slot 0x148 (0, 0) feeds slot 0x160.
        assert_eq!(world.entry_calls.borrow().len(), 1);
        assert_eq!(world.entry_calls.borrow()[0][1], 0x77);
        assert_eq!(world.process_calls.borrow().len(), 1);

        // The lighting and the character controller proxy.
        assert_eq!(
            calls_to(&e, 0x008b_0bd0),
            vec![vec![player.addr(), world.model]]
        );
        assert_eq!(
            world.proxy_calls.borrow()[0][1..],
            [world.node, 1, 0, 0x55, 0]
        );
        // The biped bodies are updated.
        assert_eq!(calls_to(&e, 0x0093_80e0), vec![vec![0xc0], vec![0xc1]]);
    }

    #[test]
    fn toddlers_and_women_get_their_own_idle_animations() {
        let mut e = engine();
        let world = load_world(&mut e, vec![]);
        let player = world.player;
        double(&mut e, 0x0083_97d0, eax(1));
        double(&mut e, 0x0087_f4c0, eax(1));
        e.set(player, PlayerCharacter::bIsToddler, true);
        start_log(&mut e);
        player_character_load_3d(&mut e, player, 0);
        assert_eq!(
            world.file_lists.borrow().clone(),
            vec![
                "Meshes\\Characters\\_1stPerson\\Locomotion\\Toddler\\IdleAnims".to_string(),
                "Meshes\\Characters\\_1stPerson\\Locomotion\\Female\\IdleAnims".to_string()
            ]
        );
        // The toddler flag is cleared and announced.
        assert!(!e.get(player, PlayerCharacter::bIsToddler));
        assert_eq!(calls_to(&e, 0x0096_97c0), vec![vec![player.addr(), 1]]);
    }

    #[test]
    fn a_missing_node_is_reported() {
        let mut e = engine();
        let world = load_world(&mut e, vec![0x12, 0x13]);
        // The third person node is missing too.
        let node = finder(&mut e, vec![0x14]);
        e.register_double(0x0087_e060, move |_, _| eax(node));
        double(&mut e, 0x0057_15d0, eax(0x7a7a));
        start_log(&mut e);
        player_character_load_3d(&mut e, world.player, 0);
        let errors = calls_to(&e, ERROR);
        assert_eq!(
            errors,
            vec![
                vec![FORMAT_MISSING_CAMERA_1ST, MODEL_PATH_SETTING + 4],
                vec![FORMAT_MISSING_BIP, MODEL_PATH_SETTING + 4],
                vec![FORMAT_MISSING_CAMERA_3RD, 0x7a7a],
            ]
        );
        assert_eq!(e.global::<u32>(NODE_CAMERA_3RD_SLOT), 0);
    }

    #[test]
    fn a_player_without_worn_items_gets_the_base_form_default_outfit() {
        let mut e = engine();
        let world = load_world(&mut e, vec![]);
        start_log(&mut e);
        player_character_load_3d(&mut e, world.player, 0);
        assert_eq!(
            calls_to(&e, 0x0060_47c0),
            vec![vec![0xf0f0, world.player.addr(), 1, 1, 0, 0]]
        );
        // All twenty slots were examined.
        assert_eq!(calls_to(&e, 0x004c_8c10).len(), 20);
        assert!(calls_to(&e, 0x004c_8220).is_empty());
    }

    #[test]
    fn a_player_with_worn_items_but_nothing_in_slot_six_gets_a_default_item() {
        let mut e = engine();
        let world = load_world(&mut e, vec![]);
        let inventory = world.inventory;
        e.register_double(0x004c_8c10, move |_, a| {
            assert_eq!(a[0], inventory);
            eax(if a[1] == 2 { 0xabc0 } else { 0 })
        });
        let item = e.mem.alloc(8);
        let holder = e.mem.alloc(8);
        e.mem.set_u32(holder, 0xe5);
        e.mem.set_u32(item, holder);
        double(&mut e, 0x004c_8220, eax(item));
        e.register(WORD_AT_8, |_, a| eax(a[0] + 0x10));
        double(&mut e, BASE_FORM_OF, eax(0xba5e));
        let equip = recording_slot(&mut e, world.player.addr(), 0x184, Ret::default());
        start_log(&mut e);
        player_character_load_3d(&mut e, world.player, 0);
        // The first found worn item is released, the search stops there.
        assert_eq!(calls_to(&e, 0x0044_59e0)[0], vec![0xabc0, 1]);
        assert!(calls_to(&e, 0x0060_47c0).is_empty());
        assert_eq!(
            calls_to(&e, 0x004c_8220),
            vec![vec![inventory, 0xba5e, 6, 1]]
        );
        assert_eq!(
            equip.borrow().clone(),
            vec![vec![world.player.addr(), item + 0x10, 1, 0xe5, 0]]
        );
        // The created item is released.
        assert_eq!(
            calls_to(&e, 0x0044_59e0).last().unwrap().clone(),
            vec![item, 1]
        );
    }

    #[test]
    fn a_player_with_an_item_in_slot_six_keeps_it() {
        let mut e = engine();
        let world = load_world(&mut e, vec![]);
        e.register(0x004c_8c10, |_, a| eax(if a[1] == 6 { 0xabc6 } else { 0 }));
        start_log(&mut e);
        player_character_load_3d(&mut e, world.player, 0);
        assert!(calls_to(&e, 0x004c_8220).is_empty());
        assert_eq!(calls_to(&e, 0x0044_59e0)[0], vec![0xabc6, 1]);
    }

    #[test]
    fn the_outfit_check_is_skipped_while_loading_a_save() {
        let mut e = engine();
        let world = load_world(&mut e, vec![]);
        double(&mut e, 0x0042_ce10, eax(1));
        start_log(&mut e);
        player_character_load_3d(&mut e, world.player, 0);
        assert!(calls_to(&e, 0x004b_f220).is_empty());
        assert!(calls_to(&e, 0x0060_47c0).is_empty());
        // The other byte does the same.
        double(&mut e, 0x0042_ce10, eax(0));
        e.set_global(0x011d_8907u32, 1u8);
        start_log(&mut e);
        player_character_load_3d(&mut e, world.player, 0);
        assert!(calls_to(&e, 0x004b_f220).is_empty());
    }

    // ----- ExportProgressData -----

    type Snapshot = Vec<Vec<u32>>;
    /// Marks "string appended" in the recorded text operations.
    const APPENDED: u32 = 0xffff_0001;

    struct ExportWorld {
        player: Ptr<PlayerCharacter>,
        file: u32,
        lines: Rc<RefCell<Vec<Snapshot>>>,
        seeks: Rc<RefCell<Vec<Vec<u32>>>>,
        deletes: Rc<RefCell<Vec<Vec<u32>>>>,
    }

    /// An object whose slots `0x130` and `0x138` answer `this + 0x30`.
    fn named_object(e: &mut Engine) -> u32 {
        let object = e.mem.alloc(0x40);
        let table = e.mem.alloc(0x800);
        e.mem.set_u32(object, table);
        for offset in [0x130u32, 0x138] {
            let target = NEXT_TARGET.fetch_add(4, Ordering::Relaxed);
            e.mem.set_u32(table + offset, target);
            e.register_double(target, |_, a| eax(a[0] + 0x30));
        }
        object
    }

    fn f64_words(value: f64) -> [u32; 2] {
        let bits = value.to_bits();
        [bits as u32, (bits >> 32) as u32]
    }

    fn export_world(e: &mut Engine) -> ExportWorld {
        let player = new_player(e);
        list_helpers(e);
        e.register(SETTING_TEXT, |_, a| eax(a[0] + 4));
        e.register(OPERATOR_NEW, |e, a| eax(e.mem.alloc(a[0])));

        // The file and the text recorder.
        let file = e.mem.alloc(0x20);
        double(e, 0x00b0_0260, eax(file));
        double(e, 0x0089_05f0, eax(1));
        let seeks = recording_slot(e, file, 0x14, Ret::default());
        let deletes = recording_slot(e, file, 0x0, Ret::default());
        let table = e.mem.u32(file);
        let current: Rc<RefCell<Snapshot>> = Rc::default();
        let lines: Rc<RefCell<Vec<Snapshot>>> = Rc::default();
        let recorded = current.clone();
        e.register_double(0x0040_6f60, move |_, a| {
            *recorded.borrow_mut() = vec![a[1..].to_vec()];
            Ret::default()
        });
        let recorded = current.clone();
        e.register_double(0x0040_37f0, move |_, _| {
            recorded.borrow_mut().clear();
            Ret::default()
        });
        let recorded = current.clone();
        e.register_double(STRING_APPEND, move |_, a| {
            recorded.borrow_mut().push(vec![APPENDED, a[1]]);
            Ret::default()
        });
        let write = NEXT_TARGET.fetch_add(4, Ordering::Relaxed);
        e.mem.set_u32(table + 0x38, write);
        let written = lines.clone();
        let recorded = current.clone();
        e.register_double(write, move |_, _| {
            written.borrow_mut().push(recorded.borrow().clone());
            Ret::default()
        });

        // The player's values.
        let owner = player.addr() + 0xa4;
        let owner_table = e.mem.alloc(0x100);
        e.mem.set_u32(owner, owner_table);
        for (offset, base) in [(0u32, 1000u32), (8, 2000)] {
            let target = NEXT_TARGET.fetch_add(4, Ordering::Relaxed);
            e.mem.set_u32(owner_table + offset, target);
            e.register_double(target, move |_, a| eax(base + a[1]));
        }
        slot(e, player.addr(), 0x37c, eax(0x1000));
        slot(e, player.addr(), 0x43c, st0(1.25));
        e.register(0x0040_8da0, |_, a| eax(a[0] + 1));
        e.register(FORM_ID_OF, |_, a| eax(a[0] + 1));
        e.register(0x0055_d520, |_, a| eax(a[0] + 7));
        double(e, 0x0086_7e30, eax(777));
        e.mem.set_u32(REASON_NAME_TABLE, 0xaaa0);
        e.mem.set_u32(REASON_NAME_TABLE + 4, 0xaaa1);
        e.mem.set_u32(SEX_NAME_TABLE + 4, 0xbbb1);
        e.mem.set_u32(CAUSE_NAME_TABLE + 12, 0xccc3);
        double(e, 0x0087_f4c0, eax(1));
        double(e, 0x0087_f9f0, eax(0x0007_1234));
        e.register(0x005a_3370, |_, a| eax(5000 + a[0]));
        double(e, PARENT_CELL_OF, eax(0x4000));
        double(e, 0x0042_5fd0, eax(1));
        ExportWorld {
            player,
            file,
            lines,
            seeks,
            deletes,
        }
    }

    #[test]
    fn nothing_is_written_when_the_file_cannot_be_opened() {
        let mut e = engine();
        let world = export_world(&mut e);
        double(&mut e, 0x00b0_0260, eax(0));
        start_log(&mut e);
        player_character_export_progress_data(&mut e, world.player, 0);
        assert!(world.lines.borrow().is_empty());
        assert!(calls_to(&e, 0x0040_37b0).is_empty());
        // The path is built from the directory setting.
        let path = calls_to(&e, SPRINTF);
        assert_eq!(
            path[0][1..],
            [0x104, FORMAT_EXPORT_PATH, EXPORT_DIRECTORY_SETTING + 4]
        );
        // The block of the file is 0x158 bytes.
        assert_eq!(calls_to(&e, OPERATOR_NEW), vec![vec![0x158]]);
        // A file that does not open is not written either.
        double(&mut e, 0x00b0_0260, eax(world.file));
        double(&mut e, 0x0089_05f0, eax(0));
        player_character_export_progress_data(&mut e, world.player, 0);
        assert!(world.lines.borrow().is_empty());
        assert!(world.seeks.borrow().is_empty());
    }

    #[test]
    fn the_progress_record_has_all_its_lines() {
        let mut e = engine();
        let world = export_world(&mut e);
        let player = world.player;
        e.set_global(EXPORT_SEEK_MODE, 2u32);

        // Two perks, the second one empty.
        let perk = e.mem.alloc(0x20);
        let entry = e.mem.alloc(8);
        e.mem.set_u32(entry, perk);
        e.mem.set_u8(entry + 4, 2);
        let empty_entry = e.mem.alloc(8);
        let second = node(&mut e, empty_entry, 0);
        e.mem.set_u32(player.addr() + 0x87c, entry);
        e.mem.set_u32(player.addr() + 0x880, second);

        // The inventory and the values taken from it.
        let inventory = e.mem.alloc(0x40);
        double(&mut e, 0x004b_f220, eax(inventory));
        e.register(0x0093_abe0, |e, a| {
            e.mem.set_f32(a[1], 1.5);
            e.mem.set_f32(a[2], 2.5);
            Ret::default()
        });
        double(&mut e, 0x004d_0f40, eax(321));
        double(&mut e, 0x0057_7d50, eax(654));

        // The quest log: one real quest entry, one without a quest.
        let quest_item = e.mem.alloc(8);
        let other_item = e.mem.alloc(8);
        let quest = named_object(&mut e);
        e.register_double(0x0060_fd70, move |_, a| eax((a[0] == quest_item) as u32));
        e.register_double(0x005e_3fa0, move |_, a| {
            eax(if a[0] == quest_item { quest } else { 0 })
        });
        let quest_second = node(&mut e, other_item, 0);
        e.mem.set_u32(player.addr() + 0x6b0, quest_item);
        e.mem.set_u32(player.addr() + 0x6b4, quest_second);

        start_log(&mut e);
        player_character_export_progress_data(&mut e, player, 0);

        let lines = world.lines.borrow().clone();
        assert_eq!(lines.len(), 129);
        let one = |words: &[u32]| vec![words.to_vec()];
        assert_eq!(lines[0], one(&[FORMAT_TEXT, UNKNOWN_TEXT]));
        assert_eq!(lines[1], one(&[FORMAT_TEXT, GAME_VERSION_TEXT]));
        assert_eq!(lines[2], one(&[FORMAT_INT, 777]));
        assert_eq!(lines[3], one(&[FORMAT_TEXT, 0xaaa0]));
        assert_eq!(lines[4], one(&[FORMAT_TEXT, player.addr() + 7]));
        assert_eq!(lines[5], one(&[FORMAT_TEXT, 0x1000 + 0x18 + 1]));
        assert_eq!(lines[6], one(&[FORMAT_TEXT, 0xbbb1]));
        assert_eq!(lines[7], one(&[FORMAT_INT, 0x1234]));
        assert_eq!(lines[8], one(&[FORMAT_INT, 2000 + 0x18]));

        // The perk line: quote, one entry, closing quote.
        let perk_format = calls_to(&e, SPRINTF)
            .into_iter()
            .find(|words| words[2] == FORMAT_PERK)
            .unwrap();
        assert_eq!(perk_format[3..], [perk + 0x18 + 1, perk + 1, 2]);
        assert_eq!(
            lines[9],
            vec![
                vec![APPENDED, perk_format[0]],
                vec![APPENDED, TEXT_CLOSE_QUOTE]
            ]
        );

        // Fourteen skills (actual value slot 8), then 7 and 20 pairs.
        for index in 0..14u32 {
            assert_eq!(
                lines[10 + index as usize],
                one(&[FORMAT_INT, 2000 + 0x20 + index])
            );
        }
        for index in 0..7u32 {
            assert_eq!(
                lines[24 + 2 * index as usize],
                one(&[FORMAT_INT, 1000 + 5 + index])
            );
            assert_eq!(
                lines[25 + 2 * index as usize],
                one(&[FORMAT_INT, 2000 + 5 + index])
            );
        }
        for index in 0..20u32 {
            assert_eq!(
                lines[38 + 2 * index as usize],
                one(&[FORMAT_INT, 1000 + 12 + index])
            );
            assert_eq!(
                lines[39 + 2 * index as usize],
                one(&[FORMAT_INT, 2000 + 12 + index])
            );
        }

        // The inventory is built by the callback of 0094ec90.
        assert_eq!(lines[78], vec![vec![APPENDED, TEXT_CLOSE_QUOTE]]);
        let walk = calls_to(&e, 0x004d_4530);
        assert_eq!(walk.len(), 1);
        assert_eq!(walk[0][0], inventory);
        assert_eq!(walk[0][1], 0x0094_ec90);
        assert_eq!(walk[0][3], 0);
        let [low1, high1] = f64_words(1.5);
        let [low2, high2] = f64_words(2.5);
        assert_eq!(
            lines[79],
            one(&[FORMAT_TWO_FLOATS, low1, high1, low2, high2])
        );
        assert_eq!(lines[80], one(&[FORMAT_INT, 321]));
        assert_eq!(lines[81], one(&[FORMAT_INT, 654]));
        let [low, high] = f64_words(1.25);
        assert_eq!(lines[82], one(&[FORMAT_ONE_FLOAT, low, high]));

        // The quest line.
        let quest_format = calls_to(&e, SPRINTF)
            .into_iter()
            .find(|words| words[2] == FORMAT_QUEST)
            .unwrap();
        assert_eq!(quest_format[3..], [quest + 0x30, quest + 1]);
        assert_eq!(
            lines[83],
            vec![
                vec![APPENDED, quest_format[0]],
                vec![APPENDED, TEXT_CLOSE_QUOTE]
            ]
        );

        // The 43 counters, the cell (an interior), the line end.
        for index in 0..43u32 {
            assert_eq!(lines[84 + index as usize], one(&[FORMAT_INT, 5000 + index]));
        }
        assert_eq!(
            lines[127],
            one(&[FORMAT_NAME_ID, 0x4000 + 0x18 + 1, 0x4001])
        );
        assert_eq!(lines[128], one(&[TEXT_LINE_END]));

        // The file was seeked, closed and deleted; the string was built and
        // destroyed.
        assert_eq!(world.seeks.borrow().clone(), vec![vec![world.file, 0, 2]]);
        assert_eq!(calls_to(&e, 0x00af_fd10), vec![vec![world.file]]);
        assert_eq!(world.deletes.borrow().clone(), vec![vec![world.file, 1]]);
        let text = calls_to(&e, 0x0040_37b0)[0][0];
        assert_eq!(calls_to(&e, 0x0040_37d0), vec![vec![text]]);
        // The file is opened for writing with the buffer size 0x4000.
        let open = calls_to(&e, 0x00b0_0260);
        assert_eq!(open[0][2..], [1, 0x4000, 0]);
    }

    #[test]
    fn the_record_without_an_inventory_has_empty_quotes_and_zero_weight() {
        let mut e = engine();
        let world = export_world(&mut e);
        e.mem.set_u8(BUILD_NAME_BUFFER, b'N');
        e.mem.set_u32(world.player.addr() + 0x87c, 0);
        // The first perk entry list head is empty: no perks at all.
        start_log(&mut e);
        player_character_export_progress_data(&mut e, world.player, 1);
        let lines = world.lines.borrow().clone();
        assert_eq!(lines[0], vec![vec![FORMAT_TEXT, BUILD_NAME_BUFFER]]);
        assert_eq!(lines[9], vec![vec![APPENDED, TEXT_CLOSE_QUOTE]]);
        assert_eq!(lines[78], vec![vec![TEXT_EMPTY_QUOTES]]);
        assert_eq!(lines[79][0][1..], [0, 0, 0, 0]);
        assert_eq!(lines[80], vec![vec![FORMAT_INT, 0]]);
        assert!(calls_to(&e, 0x004d_4530).is_empty());
        assert_eq!(calls_to(&e, 0x004d_0f40).len(), 0);
    }

    #[test]
    fn the_cell_line_covers_exteriors_and_no_cell() {
        // A cell in a world space.
        let mut e = engine();
        let world = export_world(&mut e);
        double(&mut e, 0x0042_5fd0, eax(0));
        double(&mut e, 0x0054_ddd0, eax(0x7000));
        double(&mut e, 0x0054_4c30, eax(11));
        double(&mut e, 0x0054_4c60, eax(22));
        player_character_export_progress_data(&mut e, world.player, 0);
        assert_eq!(
            world.lines.borrow()[127],
            vec![vec![FORMAT_CELL_WORLD, 0x7000 + 0x19, 11, 22, 0x4001]]
        );
        // A cell without a world space.
        let mut e = engine();
        let world = export_world(&mut e);
        double(&mut e, 0x0042_5fd0, eax(0));
        double(&mut e, 0x0054_ddd0, eax(0));
        double(&mut e, 0x0054_4c30, eax(11));
        double(&mut e, 0x0054_4c60, eax(22));
        player_character_export_progress_data(&mut e, world.player, 0);
        assert_eq!(
            world.lines.borrow()[127],
            vec![vec![FORMAT_CELL_UNKNOWN_WORLD, 11, 22, 0x4001]]
        );
        // No cell at all.
        let mut e = engine();
        let world = export_world(&mut e);
        double(&mut e, PARENT_CELL_OF, eax(0));
        player_character_export_progress_data(&mut e, world.player, 0);
        assert_eq!(world.lines.borrow()[127], vec![vec![TEXT_NONE]]);
    }

    /// The lines after the cell line for a death (reason 1).
    fn death_tail(e: &mut Engine, world: &ExportWorld) -> Vec<Snapshot> {
        player_character_export_progress_data(e, world.player, 1);
        world.lines.borrow()[128..].to_vec()
    }

    #[test]
    fn a_death_names_the_cause_the_killer_and_the_weapon() {
        let mut e = engine();
        let world = export_world(&mut e);
        let killer = e.mem.alloc(0x40);
        e.set(world.player, PlayerCharacter::pMyKiller, Ptr::new(killer));
        // The cause: damage type 3 from an object.
        let extra = e.mem.alloc(0x20);
        e.mem.set_i32(extra + 0x10, 3);
        let source = named_object(&mut e);
        e.mem.set_u32(extra + 0x14, source);
        double(&mut e, EXTRA_LIST_OF, eax(0xe0));
        double(&mut e, GET_EXTRA_DATA, eax(extra));
        double(&mut e, 0x0044_0e30, eax(0x6161));
        // A plain killer with a base form, damage and a weapon.
        let base = named_object(&mut e);
        double(&mut e, BASE_FORM_OF, eax(base));
        double(&mut e, 0x0056_8ad0, st0(12.5));
        let weapon = e.mem.alloc(0x40);
        double(&mut e, 0x008a_1710, eax(weapon));
        start_log(&mut e);
        let tail = death_tail(&mut e, &world);
        let [low, high] = f64_words(12.5);
        assert_eq!(
            tail,
            vec![
                vec![vec![FORMAT_TEXT, 0xccc3]],
                vec![vec![FORMAT_NAME_ID, killer + 7, killer + 1]],
                vec![vec![FORMAT_NAME_ID, base + 0x30, base + 1]],
                vec![vec![FORMAT_ONE_FLOAT, low, high]],
                vec![vec![FORMAT_NAME_ID, weapon + 0x31, weapon + 1]],
                vec![vec![TEXT_LINE_END]],
            ]
        );
        // The object of the cause is formatted but not written: its format
        // call is there.
        let object_format: Vec<Vec<u32>> = calls_to(&e, 0x0040_6f60)
            .into_iter()
            .filter(|words| words[1] == FORMAT_CAUSE_OBJECT)
            .collect();
        assert_eq!(object_format.len(), 1);
        assert_eq!(object_format[0][2..], [0x6161, source + 0x30, source + 1]);
    }

    #[test]
    fn a_death_without_extra_data_or_killer_is_unknown() {
        let mut e = engine();
        let world = export_world(&mut e);
        double(&mut e, GET_EXTRA_DATA, eax(0));
        let tail = death_tail(&mut e, &world);
        assert_eq!(
            tail,
            vec![
                vec![vec![TEXT_UNKNOWN_CAUSE]],
                vec![vec![TEXT_UNKNOWN_OBJECT]],
                vec![vec![TEXT_UNKNOWN_KILLER]],
                vec![vec![TEXT_LINE_END]],
            ]
        );
    }

    #[test]
    fn a_death_without_a_cause_type_says_none() {
        let mut e = engine();
        let world = export_world(&mut e);
        let extra = e.mem.alloc(0x20);
        e.mem.set_i32(extra + 0x10, -1);
        double(&mut e, GET_EXTRA_DATA, eax(extra));
        let tail = death_tail(&mut e, &world);
        assert_eq!(tail[0], vec![vec![TEXT_NONE]]);
        // Without a source object, "none" is formatted and not written.
        assert_eq!(tail[1], vec![vec![TEXT_UNKNOWN_KILLER]]);
    }

    fn levelled_killer(
        e: &mut Engine,
        world: &ExportWorld,
        original: u32,
        template: u32,
    ) -> Vec<Snapshot> {
        let killer = e.mem.alloc(0x40);
        e.set(world.player, PlayerCharacter::pMyKiller, Ptr::new(killer));
        double(e, GET_EXTRA_DATA, eax(0));
        double(e, 0x0056_af40, eax(1));
        double(e, 0x0042_16f0, eax(original));
        double(e, 0x0042_1720, eax(template));
        death_tail(e, world)
    }

    #[test]
    fn a_levelled_killer_shows_the_original_and_the_template() {
        let mut e = engine();
        let world = export_world(&mut e);
        let original = named_object(&mut e);
        let template = named_object(&mut e);
        let tail = levelled_killer(&mut e, &world, original, template);
        // The extras line, the killer line, the levelled line.
        assert_eq!(
            tail[3],
            vec![vec![
                FORMAT_LEVELLED_BOTH,
                original + 0x30,
                original + 1,
                template + 0x30,
                template + 1
            ]]
        );
    }

    #[test]
    fn a_levelled_killer_may_lack_one_of_them() {
        let mut e = engine();
        let world = export_world(&mut e);
        let original = named_object(&mut e);
        let tail = levelled_killer(&mut e, &world, original, 0);
        assert_eq!(
            tail[3],
            vec![vec![
                FORMAT_LEVELLED_ORIGINAL_ONLY,
                original + 0x30,
                original + 1
            ]]
        );
        let mut e = engine();
        let world = export_world(&mut e);
        let template = named_object(&mut e);
        let tail = levelled_killer(&mut e, &world, 0, template);
        assert_eq!(
            tail[3],
            vec![vec![
                FORMAT_LEVELLED_TEMPLATE_ONLY,
                template + 0x30,
                template + 1
            ]]
        );
        let mut e = engine();
        let world = export_world(&mut e);
        let tail = levelled_killer(&mut e, &world, 0, 0);
        assert_eq!(tail[3], vec![vec![TEXT_UNKNOWN_LEVELLED]]);
        // The weapon line reports no weapon.
        assert_eq!(tail[5], vec![vec![TEXT_NO_WEAPON]]);
    }
}
