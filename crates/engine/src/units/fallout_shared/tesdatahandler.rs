//! `fallout shared/tesdatahandler.cpp` (Xbox PDB source unit), subsystem `fallout shared`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! `TESDataHandler` is the game's form database: the singleton at
//! `0x011c3f2c` (0x63C bytes on both builds) owns one `BSSimpleList` per kind
//! of form (`listScripts`, `listFactions`, ...), the loaded `TESFile`s and the
//! interior cell and add-on node arrays, and it loads and saves forms for
//! the plugin files.
//!
//! Session 1 of this unit (the first 40 functions in address order,
//! `0040fbe0` to `00461250`) holds the constructor and destructor
//! (`0045d270`, `0045d970`), `ClearData` (`0045dfe0`), `LoadForm`, `SaveForm`,
//! `AddFormToDataHandler` and the many one-line helpers the compiler
//! emitted next to them: the address of one of the form lists
//! (`this + offset`, which the linker folded with identical accessors of
//! other classes, so some of the addresses belong to other units), the
//! flag getters and the scalar deleting destructors of the objects the
//! handler owns. The next session continues at `00461270`.
//!
//! Notes for the next session:
//! - A `BSSimpleList<T>` is 8 bytes: the head node (item at +0, next at +4)
//!   lives in the owner. `006815c0(list)` returns the address of the head
//!   node's item slot, `0063f7b0(list)` removes the head node, `00726070`
//!   steps to the next node and `005ae3d0(list, &item)` adds an item.
//! - Functions of this same unit that come later in the queue and are called
//!   by address from the ones here: `00461820` (`AddAddonNode`),
//!   `00461bc0` (`GetCellFromWorldCoord`), `00464e50` (`CleanUpBadForms`)
//!   and `00464f30` (returns its argument). The engine map assigns no unit
//!   to the other list accessors `AddFormToDataHandler` reaches
//!   (`00460fb0`, `00461070`, `004610d0`, `004610f0`, `00461110`,
//!   `00461130`, `00461150`, `00461190`) nor to `004612b0` (the name of a
//!   form type); they are called by address too.
//! - Type numbers: `AddFormToDataHandler` dispatches on the form type byte
//!   (`TESForm +4`); its table is `LOCAL_LIST_TYPES`/`EXTERNAL_LIST_TYPES`
//!   in the tests (form type to list).
//! - The calling convention of the leak report in `ClearData` pushes a
//!   stray 0 before two of its calls (`TESForm::GetFile` and the virtual at
//!   `0x130`); the callees do not read it (the `ADD ESP, 8` after the
//!   identity call `00464f30` removes it), so it is not passed.
//! - The compiler's exception-unwinding frames (`FS:[0]` chains and the
//!   `__CxxFrameHandler` state words) of the constructor and destructor are
//!   not translated.

#[allow(unused_imports)]
use crate::prelude::*;
use crate::types::{BSSimpleList, NiTArray};

/// `operator new(size)` (cdecl, one stack argument).
const OPERATOR_NEW: u32 = 0x0040_1000;
/// `operator delete(pointer)` (cdecl, one stack argument).
const OPERATOR_DELETE: u32 = 0x0040_1030;
/// `memset(destination, value, size)` (cdecl wrapper).
const MEMSET: u32 = 0x0040_3d30;
/// `__RTDynamicCast(object, vfDelta, sourceType, targetType, isReference)`.
const DYNAMIC_CAST: u32 = 0x00ec_43fb;

// Callees on the form lists, which are `BSSimpleList`s: the head node is in
// the owner, so `this` of these calls is the address of the list.
/// `BSSimpleList::BSSimpleList`.
const LIST_CONSTRUCT: u32 = 0x0096_a2d0;
/// The list destructor body.
const LIST_DESTRUCT: u32 = 0x0046_ffb0;
/// The address of the head node's item slot.
const LIST_HEAD_ITEM: u32 = 0x0068_15c0;
/// Removes the head node.
const LIST_REMOVE_HEAD: u32 = 0x0063_f7b0;
/// The node after `this` (a node is a list address too).
const LIST_NEXT: u32 = 0x0072_6070;
/// Adds an item: the argument is the address of a word holding the item.
const LIST_ADD: u32 = 0x005a_e3d0;

// Callees on the arrays and the bad-form list.
/// The constructor of `arrayInteriorCells` (`this`, 0, 1).
const CELL_ARRAY_CONSTRUCT: u32 = 0x0047_0040;
/// The constructor of `arrayAddonNodes` (`this`, 0, 1).
const ADDON_NODE_ARRAY_CONSTRUCT: u32 = 0x0047_00a0;
/// The constructor of `listBadForms`.
const BAD_FORM_LIST_CONSTRUCT: u32 = 0x0048_f200;
/// The destructor of `listBadForms`.
const BAD_FORM_LIST_DESTRUCT: u32 = 0x0054_da00;
/// The destructor body `0045d930` calls for `arrayInteriorCells`.
const CELL_ARRAY_DESTRUCT: u32 = 0x0046_ffd0;
/// The destructor body `0045d950` calls for `arrayAddonNodes`.
const ADDON_NODE_ARRAY_DESTRUCT: u32 = 0x0047_0070;
/// The element count of `arrayInteriorCells` (`this` is the array).
const ARRAY_SIZE: u32 = 0x0065_8930;
/// The address of element `index` of `arrayInteriorCells` (`this` is the
/// array, the argument the index).
const ARRAY_AT: u32 = 0x0087_7a30;
/// Called with the cell array and 0 once its cells are destroyed.
const ARRAY_RESET: u32 = 0x0096_ad30;
/// Stores an element at an index, growing the array (`this` is the array;
/// arguments: the index, the address of a word holding the element).
const ARRAY_SET_AT_GROW: u32 = 0x0047_0000;
/// Called on `arrayInteriorCells` with 100 by the constructor (stores a
/// 16-bit value at `+0xE`, the array's grow-by).
const CELL_ARRAY_SET_GROW_BY: u32 = 0x0055_9490;
/// Called by `ClearData` on `arrayAddonNodes`.
const ADDON_NODE_ARRAY_CLEAR: u32 = 0x005e_03d0;

// Owned objects.
/// The `TESObjectList` constructor (`this`, 1); the object is 0x10 bytes.
const OBJECT_LIST_CONSTRUCT: u32 = 0x0051_00d0;
/// The `TESObjectList` destructor body.
const OBJECT_LIST_DESTRUCT: u32 = 0x0051_0110;
/// Adds an object to the `TESObjectList` (`this` is the list).
const OBJECT_LIST_ADD: u32 = 0x0051_02b0;
/// Called by `ClearData` on the `TESObjectList`.
const OBJECT_LIST_CLEAR: u32 = 0x0051_02e0;
/// The `TESRegionList` constructor (`this`, 1); 0x10 bytes.
const REGION_LIST_CONSTRUCT: u32 = 0x004f_6320;
/// Called by `ClearData` on the `TESRegionList`.
const REGION_LIST_CLEAR: u32 = 0x004f_6640;
/// The constructor of the `TESRegionDataManager`; 8 bytes.
const REGION_DATA_MANAGER_CONSTRUCT: u32 = 0x004f_3660;
/// The `TESIdleManager` constructor (0x28 bytes) and destructor body.
const IDLE_MANAGER_CONSTRUCT: u32 = 0x005f_faf0;
const IDLE_MANAGER_DESTRUCT: u32 = 0x005f_fb60;
/// The `BGSCameraPathManager` constructor (0x28 bytes) and destructor body.
const CAMERA_PATH_MANAGER_CONSTRUCT: u32 = 0x0058_b770;
const CAMERA_PATH_MANAGER_DESTRUCT: u32 = 0x0058_b7d0;
/// `BGSCameraPathManager::DestroyRootPathArrayForms` (`this`).
const CAMERA_PATH_MANAGER_DESTROY_ROOT_PATHS: u32 = 0x0058_ba40;
/// `InventoryChanges`' scalar deleting destructor (`this`, flags).
const INVENTORY_CHANGES_DELETE: u32 = 0x0043_1920;
/// Called by `~TESDataHandler` with `TESForm::pAllForms` as `this`: empties
/// the map (frees the entries of each bucket).
const ALL_FORMS_CLEAR: u32 = 0x0043_8af0;
/// Called with the address of `listFiles` by `0045dfa0` (cdecl).
const FILES_LIST_DESTROY: u32 = 0x0047_3270;

// `TESFile`.
/// `TESFile`'s destructor body.
const FILE_DESTRUCT: u32 = 0x0047_09f0;
/// `TESFile::CloseTES` (`this` is the file).
const FILE_CLOSE: u32 = 0x0047_1130;
/// `TESFile::GetMaster` and `TESFile::GetActive` (answer in AL).
const FILE_GET_MASTER: u32 = 0x0047_1c20;
const FILE_GET_ACTIVE: u32 = 0x0047_1d60;
/// Called on the active file by `ClearData` (`this`, 0) when the handler is
/// not a master save.
const FILE_CLOSE_ACTIVE: u32 = 0x0047_1d90;
/// `this + 0x20` of a `TESFile` (its name).
const FILE_NAME: u32 = 0x0089_1170;

// `TESForm`.
/// `TESForm::GetFile(index)` (`this` is the form).
const FORM_GET_FILE: u32 = 0x0048_4e60;
/// The form type byte (`TESForm +4`), zero-extended.
const FORM_GET_TYPE: u32 = 0x0040_1170;
/// The form id (`TESForm +0xC`).
const FORM_GET_ID: u32 = 0x0084_e3a0;
/// The name of the form's type, from the table at `0x01187004`.
const FORM_GET_TYPE_NAME: u32 = 0x0044_0e30;
/// Sets or clears bit 0 of the form's flags (`this`, flag).
const FORM_SET_FLAG_BIT_0: u32 = 0x0048_44f0;
/// Bit 5 (`0x20`) of the form's flags.
const FORM_HAS_FLAG_BIT_5: u32 = 0x0044_0d80;
/// Called by the type-3 branch of the leak report on a form.
const FORM_LIST_CLEAR: u32 = 0x0047_0470;
/// Vtable slots (byte offsets) of `TESForm`: `~TESForm` (deleting, flag),
/// `Load(file)`, `Save(file)`, `SaveEdit(file)`, `SetAltered(flag)`, a slot
/// the leak report calls with (0, 1) on forms of type 3, and the one it calls
/// to get a text for the log (the base version returns an empty string).
const FORM_VTABLE_DELETE: u32 = 0x10;
const FORM_VTABLE_LOAD: u32 = 0x20;
const FORM_VTABLE_SAVE: u32 = 0x28;
const FORM_VTABLE_SAVE_EDIT: u32 = 0x34;
const FORM_VTABLE_SET_ALTERED: u32 = 0xc8;
const FORM_VTABLE_SLOT_128: u32 = 0x128;
const FORM_VTABLE_SLOT_130: u32 = 0x130;
/// Returns its argument (cdecl).
const IDENTITY: u32 = 0x0046_4f30;

// Logging and the rest of the game.
/// `MessageHandler::IncDisableWarningCount(count)` (cdecl).
const DISABLE_WARNING_COUNT: u32 = 0x0043_b2b0;
/// The logging `printf` (cdecl: format, then the arguments).
const LOG_MESSAGE: u32 = 0x005b_5e40;
/// `TESDataHandler::CleanUpBadForms` (`this`).
const CLEAN_UP_BAD_FORMS: u32 = 0x0046_4e50;
/// `GarbageCollector::ClearAll(flag)`.
const GARBAGE_COLLECTOR_CLEAR_ALL: u32 = 0x0086_8d70;

// The map of every form (`TESForm::pAllForms`).
/// `GetFirstPos`: the first position, or 0 (`this` is the map).
const MAP_FIRST_POSITION: u32 = 0x004b_9ba0;
/// `GetNext(&position, &key, &value)`.
const MAP_GET_NEXT: u32 = 0x006b_7f20;
/// `SetAt(key, value)`.
const MAP_SET_AT: u32 = 0x0084_4700;

// Singletons and globals (the exe addresses of the variables).
/// `TESForm::pAllForms` (pointer variable).
const ALL_FORMS_MAP: u32 = 0x011c_54c0;
/// An object whose `00863db0` `ClearData` calls first (pointer variable).
const OBJECT_011C54C4: u32 = 0x011c_54c4;
/// The `TES` singleton (pointer variable).
const TES_SINGLETON: u32 = 0x011d_ea10;
/// The `PlayerCharacter` singleton (pointer variable).
const PLAYER_SINGLETON: u32 = 0x011d_ea3c;
/// An object `ClearData` asks (`004f6de0`) which idle-manager reset to run
/// (pointer variable).
const OBJECT_011DEA0C: u32 = 0x011d_ea0c;
/// The `TESIdleManager` singleton (pointer variable).
const IDLE_MANAGER: u32 = 0x011c_b6a0;
/// The `BGSCameraPathManager` singleton (pointer variable).
const CAMERA_PATH_MANAGER: u32 = 0x011c_a700;
/// Words that `00460160`, `00460190` and `ClearData` clear.
const FLAG_011CA830: u32 = 0x011c_a830;
const FLAG_011C9520: u32 = 0x011c_9520;
const FLAG_011CB96C: u32 = 0x011c_b96c;
const FLAG_011CA53C: u32 = 0x011c_a53c;
/// The words `ClearData` clears at its end (`0x011ca220` to `0x011ca26c`,
/// without `0x011ca24c` and `0x011ca264`).
const CLEARED_WORDS: [u32; 18] = [
    0x011c_a220,
    0x011c_a224,
    0x011c_a228,
    0x011c_a22c,
    0x011c_a230,
    0x011c_a234,
    0x011c_a238,
    0x011c_a23c,
    0x011c_a240,
    0x011c_a244,
    0x011c_a248,
    0x011c_a250,
    0x011c_a254,
    0x011c_a258,
    0x011c_a25c,
    0x011c_a260,
    0x011c_a268,
    0x011c_a26c,
];

// RTTI type descriptors for the `dynamic_cast`s of `AddFormToDataHandler`.
/// `TESForm`, the source type of every cast.
const FORM_TYPE_DESCRIPTOR: u32 = 0x0118_3028;
/// `EffectSetting`.
const EFFECT_SETTING_TYPE_DESCRIPTOR: u32 = 0x0118_37c4;
/// `TESObjectREFR`.
const REFERENCE_TYPE_DESCRIPTOR: u32 = 0x0118_41cc;
/// `TESObject`.
const OBJECT_TYPE_DESCRIPTOR: u32 = 0x0118_3128;

// Strings.
/// `"UNKNOWN"`.
const UNKNOWN_FILE_NAME: u32 = 0x0101_5890;
/// `"FORMS: Form '%s' (%08X) of type %s in file '%s' was not freed."`.
const FORM_LEAKED_FORMAT: u32 = 0x0101_8630;
/// `"FORMS: Forms were leaked during ClearData. Check Warnings file for more info."`.
const FORMS_LEAKED_MESSAGE: u32 = 0x0101_85e0;
/// `"FORMS: Unknown form type '%s' encountered in AddFormToDataHandler."`.
const UNKNOWN_FORM_TYPE_FORMAT: u32 = 0x0101_8670;

/// Size in bytes of `pFileIndex` (`TESFile*[1020]`).
const FILE_INDEX_BYTES: u32 = 0x3fc;
/// The first and last form list (`listPackages` to
/// `listMediaLocationControllers`): 58 consecutive `BSSimpleList`s, 8 bytes
/// apart.
const FIRST_LIST: u32 = 0x008;
const LAST_LIST: u32 = 0x1d0;

/// Returns `this + 0x6c` of the `TES` object (an embedded object), which
/// `ClearData` hands to `004ee920`.
const TES_GET_EMBEDDED_OBJECT: u32 = 0x0043_b5d0;
/// Called on that embedded object by `ClearData` (frees the chain of nodes at
/// `+4`).
const EMBEDDED_OBJECT_RESET: u32 = 0x004e_e920;

layout! {
    /// `TESDataHandler` (Xbox PDB), 0x63C bytes on both builds. The lists
    /// are `BSSimpleList`s of pointers to the form class named in each doc.
    pub struct TESDataHandler: 0x63C {
        /// `cDLCFlags` (Xbox PDB): `char`.
        0x000 cDLCFlags: u8,
        /// `pObjectList` (Xbox PDB): `TESObjectList*`.
        0x004 pObjectList: Ptr,
        /// `listPackages` (Xbox PDB): `BSSimpleList<TESPackage *>`.
        0x008 listPackages: Inline<BSSimpleList>,
        /// `listWorldSpaces` (Xbox PDB): `BSSimpleList<TESWorldSpace *>`.
        0x010 listWorldSpaces: Inline<BSSimpleList>,
        /// `listClimates` (Xbox PDB): `BSSimpleList<TESClimate *>`.
        0x018 listClimates: Inline<BSSimpleList>,
        /// `listImageSpaces` (Xbox PDB): `BSSimpleList<TESImageSpace *>`.
        0x020 listImageSpaces: Inline<BSSimpleList>,
        /// `listImageSpaceModifiers` (Xbox PDB): `BSSimpleList<TESImageSpaceModifier *>`.
        0x028 listImageSpaceModifiers: Inline<BSSimpleList>,
        /// `listWeather` (Xbox PDB): `BSSimpleList<TESWeather *>`.
        0x030 listWeather: Inline<BSSimpleList>,
        /// `listEnchantmentItems` (Xbox PDB): `BSSimpleList<EnchantmentItem *>`.
        0x038 listEnchantmentItems: Inline<BSSimpleList>,
        /// `listSpellItems` (Xbox PDB): `BSSimpleList<SpellItem *>`.
        0x040 listSpellItems: Inline<BSSimpleList>,
        /// `listHeadParts` (Xbox PDB): `BSSimpleList<BGSHeadPart *>`.
        0x048 listHeadParts: Inline<BSSimpleList>,
        /// `listHair` (Xbox PDB): `BSSimpleList<TESHair *>`.
        0x050 listHair: Inline<BSSimpleList>,
        /// `listEyes` (Xbox PDB): `BSSimpleList<TESEyes *>`.
        0x058 listEyes: Inline<BSSimpleList>,
        /// `listRaces` (Xbox PDB): `BSSimpleList<TESRace *>`.
        0x060 listRaces: Inline<BSSimpleList>,
        /// `listZones` (Xbox PDB): `BSSimpleList<BGSEncounterZone *>`.
        0x068 listZones: Inline<BSSimpleList>,
        /// `listLandTexts` (Xbox PDB): `BSSimpleList<TESLandTexture *>`.
        0x070 listLandTexts: Inline<BSSimpleList>,
        /// `listCameraShots` (Xbox PDB): `BSSimpleList<BGSCameraShot *>`.
        0x078 listCameraShots: Inline<BSSimpleList>,
        /// `listClasses` (Xbox PDB): `BSSimpleList<TESClass *>`.
        0x080 listClasses: Inline<BSSimpleList>,
        /// `listFactions` (Xbox PDB): `BSSimpleList<TESFaction *>`.
        0x088 listFactions: Inline<BSSimpleList>,
        /// `listReputations` (Xbox PDB): `BSSimpleList<TESReputation *>`.
        0x090 listReputations: Inline<BSSimpleList>,
        /// `listChallenges` (Xbox PDB): `BSSimpleList<TESChallenge *>`.
        0x098 listChallenges: Inline<BSSimpleList>,
        /// `listRecipes` (Xbox PDB): `BSSimpleList<TESRecipe *>`.
        0x0A0 listRecipes: Inline<BSSimpleList>,
        /// `listRecipeCategories` (Xbox PDB): `BSSimpleList<TESRecipeCategory *>`.
        0x0A8 listRecipeCategories: Inline<BSSimpleList>,
        /// `listAmmoEffects` (Xbox PDB): `BSSimpleList<TESAmmoEffect *>`.
        0x0B0 listAmmoEffects: Inline<BSSimpleList>,
        /// `listCasinos` (Xbox PDB): `BSSimpleList<TESCasino *>`.
        0x0B8 listCasinos: Inline<BSSimpleList>,
        /// `listCaravanDecks` (Xbox PDB): `BSSimpleList<TESCaravanDeck *>`.
        0x0C0 listCaravanDecks: Inline<BSSimpleList>,
        /// `listScripts` (Xbox PDB): `BSSimpleList<Script *>`.
        0x0C8 listScripts: Inline<BSSimpleList>,
        /// `listSounds` (Xbox PDB): `BSSimpleList<TESSound *>`.
        0x0D0 listSounds: Inline<BSSimpleList>,
        /// `listAcousticSpaces` (Xbox PDB): `BSSimpleList<BGSAcousticSpace *>`.
        0x0D8 listAcousticSpaces: Inline<BSSimpleList>,
        /// `listRagdolls` (Xbox PDB): `BSSimpleList<BGSRagdoll *>`.
        0x0E0 listRagdolls: Inline<BSSimpleList>,
        /// `listGlobals` (Xbox PDB): `BSSimpleList<TESGlobal *>`.
        0x0E8 listGlobals: Inline<BSSimpleList>,
        /// `listVoiceTypes` (Xbox PDB): `BSSimpleList<BGSVoiceType *>`.
        0x0F0 listVoiceTypes: Inline<BSSimpleList>,
        /// `listImpactData` (Xbox PDB): `BSSimpleList<BGSImpactData *>`.
        0x0F8 listImpactData: Inline<BSSimpleList>,
        /// `listImpactDataSet` (Xbox PDB): `BSSimpleList<BGSImpactDataSet *>`.
        0x100 listImpactDataSet: Inline<BSSimpleList>,
        /// `listTopics` (Xbox PDB): `BSSimpleList<TESTopic *>`.
        0x108 listTopics: Inline<BSSimpleList>,
        /// `listTopicInfos` (Xbox PDB): `BSSimpleList<TESTopicInfo *>`.
        0x110 listTopicInfos: Inline<BSSimpleList>,
        /// `listQuests` (Xbox PDB): `BSSimpleList<TESQuest *>`.
        0x118 listQuests: Inline<BSSimpleList>,
        /// `listCombatStyles` (Xbox PDB): `BSSimpleList<TESCombatStyle *>`.
        0x120 listCombatStyles: Inline<BSSimpleList>,
        /// `listLoadScreens` (Xbox PDB): `BSSimpleList<TESLoadScreen *>`.
        0x128 listLoadScreens: Inline<BSSimpleList>,
        /// `listWater` (Xbox PDB): `BSSimpleList<TESWaterForm *>`.
        0x130 listWater: Inline<BSSimpleList>,
        /// `listEffectShaders` (Xbox PDB): `BSSimpleList<TESEffectShader *>`.
        0x138 listEffectShaders: Inline<BSSimpleList>,
        /// `listProjectiles` (Xbox PDB): `BSSimpleList<BGSProjectile *>`.
        0x140 listProjectiles: Inline<BSSimpleList>,
        /// `listExplosions` (Xbox PDB): `BSSimpleList<BGSExplosion *>`.
        0x148 listExplosions: Inline<BSSimpleList>,
        /// `listRadiation` (Xbox PDB): `BSSimpleList<BGSRadiationStage *>`.
        0x150 listRadiation: Inline<BSSimpleList>,
        /// `listDehydration` (Xbox PDB): `BSSimpleList<BGSDehydrationStage *>`.
        0x158 listDehydration: Inline<BSSimpleList>,
        /// `listHunger` (Xbox PDB): `BSSimpleList<BGSHungerStage *>`.
        0x160 listHunger: Inline<BSSimpleList>,
        /// `listSleepDeprevation` (Xbox PDB): `BSSimpleList<BGSSleepDeprevationStage *>`.
        0x168 listSleepDeprevation: Inline<BSSimpleList>,
        /// `listDebris` (Xbox PDB): `BSSimpleList<BGSDebris *>`.
        0x170 listDebris: Inline<BSSimpleList>,
        /// `listPerks` (Xbox PDB): `BSSimpleList<BGSPerk *>`.
        0x178 listPerks: Inline<BSSimpleList>,
        /// `listPartData` (Xbox PDB): `BSSimpleList<BGSBodyPartData *>`.
        0x180 listPartData: Inline<BSSimpleList>,
        /// `listNotes` (Xbox PDB): `BSSimpleList<BGSNote *>`.
        0x188 listNotes: Inline<BSSimpleList>,
        /// `listListForms` (Xbox PDB): `BSSimpleList<BGSListForm *>`.
        0x190 listListForms: Inline<BSSimpleList>,
        /// `listMenuIcons` (Xbox PDB): `BSSimpleList<BGSMenuIcon *>`.
        0x198 listMenuIcons: Inline<BSSimpleList>,
        /// `animObjects` (Xbox PDB): `BSSimpleList<TESObjectANIO *>`.
        0x1A0 animObjects: Inline<BSSimpleList>,
        /// `listMessages` (Xbox PDB): `BSSimpleList<BGSMessage *>`.
        0x1A8 listMessages: Inline<BSSimpleList>,
        /// `listLightingTemplates` (Xbox PDB): `BSSimpleList<BGSLightingTemplate *>`.
        0x1B0 listLightingTemplates: Inline<BSSimpleList>,
        /// `listMusicTypes` (Xbox PDB): `BSSimpleList<BGSMusicType *>`.
        0x1B8 listMusicTypes: Inline<BSSimpleList>,
        /// `listLoadScreenTypes` (Xbox PDB): `BSSimpleList<TESLoadScreenType *>`.
        0x1C0 listLoadScreenTypes: Inline<BSSimpleList>,
        /// `listMediaSets` (Xbox PDB): `BSSimpleList<MediaSet *>`.
        0x1C8 listMediaSets: Inline<BSSimpleList>,
        /// `listMediaLocationControllers` (Xbox PDB): `BSSimpleList<MediaLocationController *>`.
        0x1D0 listMediaLocationControllers: Inline<BSSimpleList>,
        /// `pRegionList` (Xbox PDB): `TESRegionList*`.
        0x1D8 pRegionList: Ptr,
        /// `arrayInteriorCells` (Xbox PDB): `NiTPrimitiveArray<TESObjectCELL *>`.
        0x1DC arrayInteriorCells: Inline<NiTArray>,
        /// `arrayAddonNodes` (Xbox PDB): `NiTPrimitiveArray<BGSAddonNode *>`.
        0x1EC arrayAddonNodes: Inline<NiTArray>,
        /// `listBadForms` (Xbox PDB): `NiTList<TESForm *>`.
        0x1FC listBadForms: Inline<()>,
        /// `iNextID` (Xbox PDB): `u32`.
        0x208 iNextID: u32,
        /// `pActiveFile` (Xbox PDB): `TESFile*`.
        0x20C pActiveFile: Ptr,
        /// `listFiles` (Xbox PDB): `BSSimpleList<TESFile *>`.
        0x210 listFiles: Inline<BSSimpleList>,
        /// `iNumCompile` (Xbox PDB): `u32`.
        0x218 iNumCompile: u32,
        /// `pFileIndex` (Xbox PDB): `TESFile*[1020]` (0x3FC bytes), the first slot.
        0x21C pFileIndex: Inline<()>,
        /// `bMasterSave` (Xbox PDB): `bool`.
        0x618 bMasterSave: bool,
        /// `bSaveLoadGame` (Xbox PDB): `bool`.
        0x619 bSaveLoadGame: bool,
        /// `bSaveLoad` (Xbox PDB): `bool`.
        0x61A bSaveLoad: bool,
        /// `bAutoSaving` (Xbox PDB): `bool`.
        0x61B bAutoSaving: bool,
        /// `bExportingPlugin` (Xbox PDB): `bool`.
        0x61C bExportingPlugin: bool,
        /// `bClearingData` (Xbox PDB): `bool`.
        0x61D bClearingData: bool,
        /// `bHasDesiredFiles` (Xbox PDB): `bool`.
        0x61E bHasDesiredFiles: bool,
        /// `bCheckingModels` (Xbox PDB): `bool`.
        0x61F bCheckingModels: bool,
        /// `bLoadingFiles` (Xbox PDB): `bool`.
        0x620 bLoadingFiles: bool,
        /// `bDontRemoveIDs` (Xbox PDB): `bool`.
        0x621 bDontRemoveIDs: bool,
        /// `ucGameSettingsLoadState` (Xbox PDB): `u8`.
        0x622 ucGameSettingsLoadState: u8,
        /// `pRegionDataManager` (Xbox PDB): `TESRegionDataManager*`.
        0x624 pRegionDataManager: Ptr,
        /// `pBarterContainer` (Xbox PDB): `InventoryChanges*`.
        0x628 pBarterContainer: Ptr,
        /// `pRecipeContainer` (Xbox PDB): `InventoryChanges*`.
        0x62C pRecipeContainer: Ptr,
        /// `pSpotterShader` (Xbox PDB): `TESEffectShader*`.
        0x630 pSpotterShader: Ptr,
        /// `pItemDetectedShader` (Xbox PDB): `TESEffectShader*`.
        0x634 pItemDetectedShader: Ptr,
        /// `pCateyeMobileShader` (Xbox PDB): `TESEffectShader*`.
        0x638 pCateyeMobileShader: Ptr,
    }
}

// Translated from 0040fbe0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Error` (Xbox PDB): an empty function (the code only sets up and tears
/// down its stack frame).
pub fn error(_e: &mut Engine) {}

// Translated from 0045d270 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESDataHandler::TESDataHandler` (Xbox PDB): constructs the 58 form lists
/// (`FIRST_LIST` to `LAST_LIST`), the two arrays, `listBadForms` and
/// `listFiles`, sets the flags and the next form id (`0x800`), clears the
/// file index, creates the object list, the region list, the region data
/// manager, the idle manager and the camera path manager, and returns
/// `this`. The cell array is given its size (100) after `pRegionList` is
/// stored.
pub fn tes_data_handler_tes_data_handler(
    e: &mut Engine,
    this: Ptr<TESDataHandler>,
) -> Ptr<TESDataHandler> {
    for offset in (FIRST_LIST..=LAST_LIST).step_by(8) {
        e.call(LIST_CONSTRUCT, &args![this.byte_add(offset)]);
    }
    let cells = this.at(TESDataHandler::arrayInteriorCells);
    e.call(CELL_ARRAY_CONSTRUCT, &args![cells, 0u32, 1u32]);
    let addon_nodes = this.at(TESDataHandler::arrayAddonNodes);
    e.call(ADDON_NODE_ARRAY_CONSTRUCT, &args![addon_nodes, 0u32, 1u32]);
    e.call(
        BAD_FORM_LIST_CONSTRUCT,
        &args![this.at(TESDataHandler::listBadForms)],
    );
    e.call(LIST_CONSTRUCT, &args![this.at(TESDataHandler::listFiles)]);

    e.set(this, TESDataHandler::iNextID, 0x800);
    e.set(this, TESDataHandler::iNumCompile, 0);
    e.set(this, TESDataHandler::pActiveFile, Ptr::NULL);
    e.set(this, TESDataHandler::bMasterSave, false);
    e.set(this, TESDataHandler::bSaveLoadGame, false);
    e.set(this, TESDataHandler::bSaveLoad, false);
    e.set(this, TESDataHandler::bAutoSaving, false);
    e.set(this, TESDataHandler::bExportingPlugin, false);
    e.set(this, TESDataHandler::bClearingData, false);
    e.set(this, TESDataHandler::bHasDesiredFiles, true);
    e.set(this, TESDataHandler::bDontRemoveIDs, false);
    e.set(this, TESDataHandler::cDLCFlags, 0);
    e.set(this, TESDataHandler::ucGameSettingsLoadState, 0);
    e.call(
        MEMSET,
        &args![this.at(TESDataHandler::pFileIndex), 0u32, FILE_INDEX_BYTES],
    );

    let object_list = new_object(e, 0x10, OBJECT_LIST_CONSTRUCT, Some(1));
    e.set(this, TESDataHandler::pObjectList, object_list);
    let region_list = new_object(e, 0x10, REGION_LIST_CONSTRUCT, Some(1));
    e.set(this, TESDataHandler::pRegionList, region_list);
    e.set(this, TESDataHandler::pBarterContainer, Ptr::NULL);
    e.set(this, TESDataHandler::pRecipeContainer, Ptr::NULL);
    e.call(CELL_ARRAY_SET_GROW_BY, &args![cells, 100u32]);
    let region_data_manager = new_object(e, 8, REGION_DATA_MANAGER_CONSTRUCT, None);
    e.set(
        this,
        TESDataHandler::pRegionDataManager,
        region_data_manager,
    );
    let idle_manager = new_object(e, 0x28, IDLE_MANAGER_CONSTRUCT, None);
    e.set_global(IDLE_MANAGER, idle_manager.addr());
    let camera_path_manager = new_object(e, 0x28, CAMERA_PATH_MANAGER_CONSTRUCT, None);
    e.set_global(CAMERA_PATH_MANAGER, camera_path_manager.addr());
    e.set(this, TESDataHandler::bCheckingModels, false);
    e.set(this, TESDataHandler::pCateyeMobileShader, Ptr::NULL);
    e.set(this, TESDataHandler::pSpotterShader, Ptr::NULL);
    e.set(this, TESDataHandler::pItemDetectedShader, Ptr::NULL);
    this
}

/// `new T(args)` as the constructor writes it: `operator new(size)`, then
/// the constructor with `this` set to the block (and the one argument some
/// of them take) only if the allocation succeeded, else a null pointer.
fn new_object(e: &mut Engine, size: u32, construct: u32, argument: Option<u32>) -> Ptr {
    let block = e.call(OPERATOR_NEW, &args![size]).u32();
    if block == 0 {
        return Ptr::NULL;
    }
    let object = match argument {
        Some(argument) => e.call(construct, &args![block, argument]),
        None => e.call(construct, &args![block]),
    };
    object.ptr()
}

// Translated from 0045d930 (decompiled, FalloutNV.exe 1.4.0.525)
/// The destructor body of `arrayInteriorCells` (the `NiTPrimitiveArray` at
/// `this`): calls `0046ffd0`.
pub fn fn_0045d930(e: &mut Engine, this: Ptr) {
    e.call(CELL_ARRAY_DESTRUCT, &args![this]);
}

// Translated from 0045d950 (decompiled, FalloutNV.exe 1.4.0.525)
/// The destructor body of `arrayAddonNodes`: calls `00470070`.
pub fn fn_0045d950(e: &mut Engine, this: Ptr) {
    e.call(ADDON_NODE_ARRAY_DESTRUCT, &args![this]);
}

// Translated from 0045d970 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESDataHandler::~TESDataHandler`: deletes the barter container, the
/// files (`0045dfa0`), the object list, the region list, the region data
/// manager, the idle manager (and clears its global), the camera path
/// manager (ditto) and the `pAllForms` map's `00438af0` object, then runs
/// the member destructors in reverse order of construction.
pub fn fn_0045d970(e: &mut Engine, this: Ptr<TESDataHandler>) {
    let barter_container = e.get(this, TESDataHandler::pBarterContainer);
    if !barter_container.is_null() {
        e.call(INVENTORY_CHANGES_DELETE, &args![barter_container, 1u32]);
    }
    fn_0045dfa0(e, this);
    let object_list = e.get(this, TESDataHandler::pObjectList);
    if !object_list.is_null() {
        fn_0045df10(e, object_list, 1);
    }
    let region_list = e.get(this, TESDataHandler::pRegionList);
    if !region_list.is_null() {
        e.vcall(region_list.addr(), 0, &args![1u32]);
    }
    let region_data_manager = e.get(this, TESDataHandler::pRegionDataManager);
    e.call(OPERATOR_DELETE, &args![region_data_manager]);
    let idle_manager: u32 = e.global(IDLE_MANAGER);
    if idle_manager != 0 {
        fn_0045df40(e, Ptr::new(idle_manager), 1);
    }
    e.set_global(IDLE_MANAGER, 0u32);
    let camera_path_manager: u32 = e.global(CAMERA_PATH_MANAGER);
    if camera_path_manager != 0 {
        fn_0045df70(e, Ptr::new(camera_path_manager), 1);
    }
    e.set_global(CAMERA_PATH_MANAGER, 0u32);
    let all_forms: u32 = e.global(ALL_FORMS_MAP);
    if all_forms != 0 {
        e.call(ALL_FORMS_CLEAR, &args![all_forms]);
    }
    e.call(LIST_DESTRUCT, &args![this.at(TESDataHandler::listFiles)]);
    e.call(
        BAD_FORM_LIST_DESTRUCT,
        &args![this.at(TESDataHandler::listBadForms)],
    );
    fn_0045d950(e, this.at(TESDataHandler::arrayAddonNodes).cast());
    fn_0045d930(e, this.at(TESDataHandler::arrayInteriorCells).cast());
    for offset in (FIRST_LIST..=LAST_LIST).rev().step_by(8) {
        e.call(LIST_DESTRUCT, &args![this.byte_add(offset)]);
    }
}

// Translated from 0045df10 (decompiled, FalloutNV.exe 1.4.0.525)
/// The scalar deleting destructor of the `TESObjectList` (`this`): runs its
/// destructor body (`00510110`) and, when bit 0 of `flags` is set, frees the
/// block. Returns `this`.
pub fn fn_0045df10(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(OBJECT_LIST_DESTRUCT, &args![this]);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Translated from 0045df40 (decompiled, FalloutNV.exe 1.4.0.525)
/// The scalar deleting destructor of the `TESIdleManager`: body `005ffb60`,
/// then free when bit 0 of `flags` is set. Returns `this`.
pub fn fn_0045df40(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(IDLE_MANAGER_DESTRUCT, &args![this]);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Translated from 0045df70 (decompiled, FalloutNV.exe 1.4.0.525)
/// The scalar deleting destructor of the `BGSCameraPathManager`: body
/// `0058b7d0`, then free when bit 0 of `flags` is set. Returns `this`.
pub fn fn_0045df70(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(CAMERA_PATH_MANAGER_DESTRUCT, &args![this]);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Translated from 0045dfa0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls `00473270` (cdecl) with the address of `listFiles`.
pub fn fn_0045dfa0(e: &mut Engine, this: Ptr<TESDataHandler>) {
    let files = fn_0045dfc0(e, this);
    e.call(FILES_LIST_DESTROY, &args![files]);
}

// Translated from 0045dfc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of `listFiles` (`this + 0x210`).
pub fn fn_0045dfc0(_e: &mut Engine, this: Ptr<TESDataHandler>) -> Ptr<BSSimpleList> {
    this.at(TESDataHandler::listFiles)
}

/// Empties a form list the way `ClearData` does: while the list's first
/// item is not null, removes the first node and destroys the item through
/// its vtable (slot `0x10`, the scalar deleting destructor, with the flag 1).
fn drain_list(e: &mut Engine, list: Ptr<BSSimpleList>) {
    loop {
        let head_item = e.call(LIST_HEAD_ITEM, &args![list]).u32();
        let item = e.mem.u32(head_item);
        if item == 0 {
            break;
        }
        e.call(LIST_REMOVE_HEAD, &args![list]);
        e.vcall(item, FORM_VTABLE_DELETE, &args![1u32]);
    }
}

/// `drain_list` without destroying the items: removes every node of the list
/// (`listTopicInfos`, whose items the topics own).
fn pop_list(e: &mut Engine, list: Ptr<BSSimpleList>) {
    loop {
        let head_item = e.call(LIST_HEAD_ITEM, &args![list]).u32();
        if e.mem.u32(head_item) == 0 {
            break;
        }
        e.call(LIST_REMOVE_HEAD, &args![list]);
    }
}

// Translated from 0045dfe0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESDataHandler::ClearData` (Xbox PDB): destroys everything the data
/// handler loaded, in this order:
/// 1. sets `bClearingData`, runs `CleanUpBadForms` and a series of resets of
///    other subsystems (calls by address, in the code's order);
/// 2. empties the 58 form lists, destroying each form through its vtable
///    (slot `0x10`, flag 1), in the fixed order of the code (see the
///    test's `CLEAR_ORDER`); `listTopicInfos` is only emptied, the interior
///    cells of `arrayInteriorCells` are destroyed and the array reset, the
///    player is destroyed and its global cleared, and the singletons
///    (object list, add-on node array, region list, navmesh obstacles, idle
///    manager, camera path manager, ...) are reset in between;
/// 3. when `TESForm::pAllForms` is still there, reports the forms that were
///    not freed (`report_leaked_forms`);
/// 4. clears the file index and, unless a save is being loaded, closes
///    (unless `bMasterSave`) and destroys the active file and restores
///    `iNextID` to `0x800`;
/// 5. clears the words `0x011ca220`..`0x011ca26c`, `bClearingData` and
///    the three effect shader pointers.
///
/// Returns true.
pub fn tes_data_handler_clear_data(e: &mut Engine, this: Ptr<TESDataHandler>) -> bool {
    e.set(this, TESDataHandler::bClearingData, true);
    e.call(CLEAN_UP_BAD_FORMS, &args![this]);
    let object: u32 = e.global(OBJECT_011C54C4);
    e.call(0x0086_3db0, &args![object]);
    let tes: u32 = e.global(TES_SINGLETON);
    e.call(0x0045_39a0, &args![tes, 0u32, 0u32]);
    fn_00460170(e, Ptr::new(tes));
    let tes_object = e.call(TES_GET_EMBEDDED_OBJECT, &args![tes]).u32();
    e.call(EMBEDDED_OBJECT_RESET, &args![tes_object]);
    e.call(GARBAGE_COLLECTOR_CLEAR_ALL, &args![0u32]);
    e.call(0x004f_f190, &args![]);
    let player: u32 = e.global(PLAYER_SINGLETON);
    e.call(0x0095_2f90, &args![player]);
    e.call(0x004c_1c40, &args![]);
    e.call(0x0059_3210, &args![]);
    e.call(0x0058_d710, &args![]);
    fn_00460190(e);
    drain_list(e, this.at(TESDataHandler::listScripts));
    drain_list(e, this.at(TESDataHandler::listListForms));
    drain_list(e, this.at(TESDataHandler::listMenuIcons));
    drain_list(e, this.at(TESDataHandler::listHeadParts));
    drain_list(e, this.at(TESDataHandler::listHair));
    drain_list(e, this.at(TESDataHandler::listEyes));
    drain_list(e, this.at(TESDataHandler::listRaces));
    e.set_global(FLAG_011CB96C, 0u32);
    drain_list(e, this.at(TESDataHandler::listZones));
    drain_list(e, this.at(TESDataHandler::listClimates));
    drain_list(e, this.at(TESDataHandler::listWeather));
    drain_list(e, this.at(TESDataHandler::listClasses));
    drain_list(e, this.at(TESDataHandler::listFactions));
    drain_list(e, this.at(TESDataHandler::listChallenges));
    drain_list(e, this.at(TESDataHandler::listReputations));
    drain_list(e, this.at(TESDataHandler::listRecipes));
    drain_list(e, this.at(TESDataHandler::listCasinos));
    drain_list(e, this.at(TESDataHandler::listRecipeCategories));
    drain_list(e, this.at(TESDataHandler::listAmmoEffects));
    drain_list(e, this.at(TESDataHandler::listGlobals));
    drain_list(e, this.at(TESDataHandler::listVoiceTypes));
    drain_list(e, this.at(TESDataHandler::listImpactData));
    drain_list(e, this.at(TESDataHandler::listImpactDataSet));
    drain_list(e, this.at(TESDataHandler::listQuests));
    drain_list(e, this.at(TESDataHandler::listTopics));
    e.call(0x0061a270, &args![]);
    pop_list(e, this.at(TESDataHandler::listTopicInfos));
    let cell_count = e
        .call(
            ARRAY_SIZE,
            &args![this.at(TESDataHandler::arrayInteriorCells)],
        )
        .i32();
    for index in 0..cell_count {
        let slot = e
            .call(
                ARRAY_AT,
                &args![this.at(TESDataHandler::arrayInteriorCells), index],
            )
            .u32();
        let cell = e.mem.u32(slot);
        if cell != 0 {
            e.vcall(cell, 0x10, &args![1u32]);
        }
    }
    e.call(
        ARRAY_RESET,
        &args![this.at(TESDataHandler::arrayInteriorCells), 0u32],
    );
    drain_list(e, this.at(TESDataHandler::listWorldSpaces));
    e.call(GARBAGE_COLLECTOR_CLEAR_ALL, &args![0u32]);
    drain_list(e, this.at(TESDataHandler::listSounds));
    let list = fn_00460090(e, this);
    drain_list(e, list);
    let list = fn_004600b0(e, this);
    drain_list(e, list);
    let list = fn_004600d0(e, this);
    drain_list(e, list);
    drain_list(e, this.at(TESDataHandler::listAcousticSpaces));
    drain_list(e, this.at(TESDataHandler::listLandTexts));
    drain_list(e, this.at(TESDataHandler::listMessages));
    let player_character: u32 = e.global(PLAYER_SINGLETON);
    if player_character != 0 {
        e.vcall(player_character, 0x10, &args![1u32]);
    }
    e.set_global(PLAYER_SINGLETON, 0u32);
    drain_list(e, this.at(TESDataHandler::listImageSpaces));
    drain_list(e, this.at(TESDataHandler::listImageSpaceModifiers));
    let object_list = e.get(this, TESDataHandler::pObjectList);
    e.call(OBJECT_LIST_CLEAR, &args![object_list]);
    e.call(
        ADDON_NODE_ARRAY_CLEAR,
        &args![this.at(TESDataHandler::arrayAddonNodes)],
    );
    drain_list(e, this.at(TESDataHandler::listSpellItems));
    drain_list(e, this.at(TESDataHandler::listEnchantmentItems));
    drain_list(e, this.at(TESDataHandler::listPackages));
    drain_list(e, this.at(TESDataHandler::listCombatStyles));
    drain_list(e, this.at(TESDataHandler::listLoadScreens));
    drain_list(e, this.at(TESDataHandler::listLoadScreenTypes));
    drain_list(e, this.at(TESDataHandler::listWater));
    e.set_global(FLAG_011CA53C, 0u32);
    drain_list(e, this.at(TESDataHandler::animObjects));
    drain_list(e, this.at(TESDataHandler::listEffectShaders));
    drain_list(e, this.at(TESDataHandler::listProjectiles));
    drain_list(e, this.at(TESDataHandler::listExplosions));
    drain_list(e, this.at(TESDataHandler::listDebris));
    drain_list(e, this.at(TESDataHandler::listPerks));
    drain_list(e, this.at(TESDataHandler::listRadiation));
    drain_list(e, this.at(TESDataHandler::listDehydration));
    drain_list(e, this.at(TESDataHandler::listHunger));
    drain_list(e, this.at(TESDataHandler::listSleepDeprevation));
    drain_list(e, this.at(TESDataHandler::listPartData));
    // `TES` reset calls.
    let region_list = e.get(this, TESDataHandler::pRegionList);
    e.call(REGION_LIST_CLEAR, &args![region_list]);
    let navmesh_obstacles = e.call(0x006c_0720, &args![]).u32();
    e.call(0x006c_09f0, &args![navmesh_obstacles]);
    let tes: u32 = e.global(TES_SINGLETON);
    if e.call(0x0045_af00, &args![tes]).u32() != 0 {
        e.call(0x0045_afb0, &args![tes, 0u32]);
    }
    let object: u32 = e.global(OBJECT_011DEA0C);
    let idle_manager: u32 = e.global(IDLE_MANAGER);
    if object == 0 || !e.call(0x004f_6de0, &args![object]).bool() {
        e.call(0x005f_fd20, &args![idle_manager]);
    } else {
        e.call(0x0060_0030, &args![idle_manager]);
    }
    let camera_path_manager: u32 = e.global(CAMERA_PATH_MANAGER);
    e.call(
        CAMERA_PATH_MANAGER_DESTROY_ROOT_PATHS,
        &args![camera_path_manager],
    );
    drain_list(e, this.at(TESDataHandler::listCameraShots));
    drain_list(e, this.at(TESDataHandler::listLightingTemplates));
    e.call(0x0040_8de0, &args![]);
    e.call(0x0066_f110, &args![]);
    drain_list(e, this.at(TESDataHandler::listNotes));
    drain_list(e, this.at(TESDataHandler::listRagdolls));
    drain_list(e, this.at(TESDataHandler::listCaravanDecks));
    let all_forms: u32 = e.global(ALL_FORMS_MAP);
    if all_forms != 0 {
        report_leaked_forms(e, all_forms);
    }

    // The file index: forget every loaded file.
    let file_count = e.get(this, TESDataHandler::iNumCompile);
    for index in 0..file_count {
        e.mem
            .set_u32(this.at(TESDataHandler::pFileIndex).addr() + 4 * index, 0);
    }
    e.set(this, TESDataHandler::iNumCompile, 0);

    let active_file = e.get(this, TESDataHandler::pActiveFile);
    if !e.get(this, TESDataHandler::bSaveLoadGame) && !active_file.is_null() {
        if !fn_004600f0(e, this) {
            e.call(FILE_CLOSE_ACTIVE, &args![active_file, 0u32]);
        }
        let active_file = e.get(this, TESDataHandler::pActiveFile);
        if !active_file.is_null() {
            fn_004601a0(e, active_file, 1);
        }
        e.set(this, TESDataHandler::pActiveFile, Ptr::NULL);
        e.set(this, TESDataHandler::iNextID, 0x800);
    }
    fn_00460160(e);
    for word in CLEARED_WORDS {
        e.set_global(word, 0u32);
    }
    e.set(this, TESDataHandler::bClearingData, false);
    e.set(this, TESDataHandler::pItemDetectedShader, Ptr::NULL);
    e.set(this, TESDataHandler::pSpotterShader, Ptr::NULL);
    e.set(this, TESDataHandler::pCateyeMobileShader, Ptr::NULL);
    true
}

/// The leak report at the end of `ClearData`: walks `TESForm::pAllForms`
/// with warnings disabled around it. A form of type 3 is
/// released through `00460110` and its virtual `0x128` (arguments 0 and 1);
/// any other is logged (`FORMS: Form '%s' (%08X) of type %s in file '%s' was
/// not freed.`, with the file of the form's last source file or `UNKNOWN`)
/// and removed from the map with `SetAt(key, 0)`. If any was logged, a final
/// message says so.
fn report_leaked_forms(e: &mut Engine, all_forms: u32) {
    e.call(DISABLE_WARNING_COUNT, &args![1u32]);
    let mut leaked = false;
    e.with_stack(12, |e, locals| {
        // position, key, form: the three out parameters of `GetNext`.
        let position = locals.addr();
        let key = locals.addr() + 4;
        let form_slot = locals.addr() + 8;
        let first = e.call(MAP_FIRST_POSITION, &args![all_forms]).u32();
        e.mem.set_u32(position, first);
        while e.mem.u32(position) != 0 {
            e.call(MAP_GET_NEXT, &args![all_forms, position, key, form_slot]);
            let form = e.mem.u32(form_slot);
            if form == 0 {
                continue;
            }
            if e.call(FORM_GET_TYPE, &args![form]).u32() == 3 {
                fn_00460110(e, Ptr::new(form));
                e.vcall(form, FORM_VTABLE_SLOT_128, &args![0u32, 1u32]);
                continue;
            }
            let file = e.call(FORM_GET_FILE, &args![form, -1i32]).u32();
            let file_name = if file != 0 {
                let file = e.call(FORM_GET_FILE, &args![form, -1i32]).u32();
                let name = e.call(FILE_NAME, &args![file]).u32();
                e.call(IDENTITY, &args![name]).u32()
            } else {
                UNKNOWN_FILE_NAME
            };
            let type_name = e.call(FORM_GET_TYPE_NAME, &args![form]).u32();
            let form_id = e.call(FORM_GET_ID, &args![form]).u32();
            // The 0 the code pushes before this call is not an argument (the
            // callee does not pop it; see the module notes).
            let editor_id = e.vcall(form, FORM_VTABLE_SLOT_130, &args![]).u32();
            let editor_id = e.call(IDENTITY, &args![editor_id]).u32();
            e.call(
                LOG_MESSAGE,
                &args![FORM_LEAKED_FORMAT, editor_id, form_id, type_name, file_name],
            );
            let key_value = e.mem.u32(key);
            e.call(MAP_SET_AT, &args![all_forms, key_value, 0u32]);
            leaked = true;
        }
    });
    e.call(DISABLE_WARNING_COUNT, &args![0u32]);
    if leaked {
        e.call(LOG_MESSAGE, &args![FORMS_LEAKED_MESSAGE]);
    }
}

// Translated from 00460090 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of `listMusicTypes` (`this + 0x1b8`).
pub fn fn_00460090(_e: &mut Engine, this: Ptr<TESDataHandler>) -> Ptr<BSSimpleList> {
    this.at(TESDataHandler::listMusicTypes)
}

// Translated from 004600b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of `listMediaSets` (`this + 0x1c8`).
pub fn fn_004600b0(_e: &mut Engine, this: Ptr<TESDataHandler>) -> Ptr<BSSimpleList> {
    this.at(TESDataHandler::listMediaSets)
}

// Translated from 004600d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of `listMediaLocationControllers` (`this + 0x1d0`).
pub fn fn_004600d0(_e: &mut Engine, this: Ptr<TESDataHandler>) -> Ptr<BSSimpleList> {
    this.at(TESDataHandler::listMediaLocationControllers)
}

// Translated from 004600f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `bMasterSave` (Xbox PDB, `this + 0x618`).
pub fn fn_004600f0(e: &mut Engine, this: Ptr<TESDataHandler>) -> bool {
    e.get(this, TESDataHandler::bMasterSave)
}

// Translated from 00460110 (decompiled, FalloutNV.exe 1.4.0.525)
/// On a `TESForm`: when the address of the list at `this + 0x10`
/// (`00460140`) is not null, calls `00470470` with it (as `this`).
pub fn fn_00460110(e: &mut Engine, this: Ptr) {
    if !fn_00460140(e, this.cast()).is_null() {
        let list = fn_00460140(e, this.cast());
        e.call(FORM_LIST_CLEAR, &args![list]);
    }
}

// Translated from 00460140 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address `this + 0x10`: the handler's `listWorldSpaces`, or, called on
/// a `TESForm` (the linker folded the two), the list at `+0x10` of the form
/// that `TESForm::GetFile` walks.
pub fn fn_00460140(_e: &mut Engine, this: Ptr<TESDataHandler>) -> Ptr<BSSimpleList> {
    this.at(TESDataHandler::listWorldSpaces)
}

// Translated from 00460160 (decompiled, FalloutNV.exe 1.4.0.525)
/// Clears the word at `0x011ca830`.
pub fn fn_00460160(e: &mut Engine) {
    e.set_global(FLAG_011CA830, 0u32);
}

// Translated from 00460170 (decompiled, FalloutNV.exe 1.4.0.525)
/// Clears the word at `this + 0x88` (of the `TES` object).
pub fn fn_00460170(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr() + 0x88, 0);
}

// Translated from 00460190 (decompiled, FalloutNV.exe 1.4.0.525)
/// Clears the word at `0x011c9520`.
pub fn fn_00460190(e: &mut Engine) {
    e.set_global(FLAG_011C9520, 0u32);
}

// Translated from 004601a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The scalar deleting destructor of a `TESFile`: body `004709f0`, then free
/// when bit 0 of `flags` is set. Returns `this`.
pub fn fn_004601a0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(FILE_DESTRUCT, &args![this]);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Translated from 004601d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESDataHandler::LoadForm` (Xbox PDB), a plain function of the form and
/// the file: loads the form from the file (virtual `0x20`), then sets the
/// form's master flag (bit 0 of its flags) to 1 if it already had it, or to
/// whether the file is a master (`TESFile::GetMaster`) if it did not, and
/// marks the form altered (virtual `0xc8`, argument 1) when the file is the
/// active one. Returns the load's result.
pub fn tes_data_handler_load_form(e: &mut Engine, form: Ptr, file: Ptr) -> bool {
    let had_master_flag = fn_00460250(e, form);
    let loaded = e.vcall(form.addr(), FORM_VTABLE_LOAD, &args![file]).bool();
    if had_master_flag {
        e.call(FORM_SET_FLAG_BIT_0, &args![form, 1u32]);
    } else {
        let is_master = e.call(FILE_GET_MASTER, &args![file]).bool();
        e.call(FORM_SET_FLAG_BIT_0, &args![form, is_master as u32]);
    }
    if e.call(FILE_GET_ACTIVE, &args![file]).bool() {
        e.vcall(form.addr(), FORM_VTABLE_SET_ALTERED, &args![1u32]);
    }
    loaded
}

// Translated from 00460250 (decompiled, FalloutNV.exe 1.4.0.525)
/// Bit 0 of the form's `iFormFlags` (`this + 8`).
pub fn fn_00460250(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u32(this.addr() + 8) & 1 != 0
}

// Translated from 00460270 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESDataHandler::SaveForm` (Xbox PDB): saves `form` to the active file.
/// False when there is no active file, or when the form's last file is not
/// the active file (or the active file is a master) and the form does not
/// have bit 1 of its flags. A form with bit 0 set is written with virtual
/// `0x34` (`SaveEdit`); otherwise it is skipped if `00440d80` (bit 5 of its
/// flags) says so, else written with virtual `0x28` (`Save`). The second
/// stack argument (the last file's master flag, computed by the one caller)
/// is not used.
pub fn tes_data_handler_save_form(
    e: &mut Engine,
    this: Ptr<TESDataHandler>,
    form: Ptr,
    _is_master: u8,
) -> bool {
    let active_file = e.get(this, TESDataHandler::pActiveFile);
    if active_file.is_null() {
        return false;
    }
    let file = e.call(FORM_GET_FILE, &args![form, -1i32]).ptr::<()>();
    let in_active_file =
        file == active_file && !e.call(FILE_GET_MASTER, &args![active_file]).bool();
    if !in_active_file && !fn_00460340(e, form) {
        return false;
    }
    if fn_00460250(e, form) {
        return e
            .vcall(form.addr(), FORM_VTABLE_SAVE_EDIT, &args![active_file])
            .bool();
    }
    if e.call(FORM_HAS_FLAG_BIT_5, &args![form]).bool() {
        return false;
    }
    e.vcall(form.addr(), FORM_VTABLE_SAVE, &args![active_file])
        .bool()
}

// Translated from 00460340 (decompiled, FalloutNV.exe 1.4.0.525)
/// Bit 1 of the form's `iFormFlags` (`this + 8`).
pub fn fn_00460340(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u32(this.addr() + 8) & 2 != 0
}

// Translated from 00460360 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls `TESFile::CloseTES` on every file in `listFiles`, stopping at the
/// first node whose item is null.
pub fn fn_00460360(e: &mut Engine, this: Ptr<TESDataHandler>) {
    let mut node = fn_0045dfc0(e, this).addr();
    while node != 0 {
        let head_item = e.call(LIST_HEAD_ITEM, &args![node]).u32();
        if e.mem.u32(head_item) == 0 {
            break;
        }
        let head_item = e.call(LIST_HEAD_ITEM, &args![node]).u32();
        let file = e.mem.u32(head_item);
        e.call(FILE_CLOSE, &args![file]);
        node = e.call(LIST_NEXT, &args![node]).u32();
    }
}

/// Adds `form` to `list` through `005ae3d0`, which takes the address of a
/// word holding the item.
fn list_add(e: &mut Engine, list: Ptr, form: Ptr) {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), form.addr());
        e.call(LIST_ADD, &args![list, slot]);
    });
}

/// The list `AddFormToDataHandler` appends a form of type `form_type` to,
/// for the 57 types that have a plain list; `None` for the types it treats
/// specially (`0x10`, `0x39`, `0x3a`, `0x3d` to `0x40`, `0x58`, `0x69`) and
/// for the types it does not know. The lists are reached through the
/// accessors (`this + offset`) the compiler emitted for each, so the
/// accessors of other units are called by address.
fn list_for_form_type(e: &mut Engine, this: Ptr<TESDataHandler>, form_type: u32) -> Option<Ptr> {
    let accessor = match form_type {
        0x05 => return Some(fn_00461210(e, this).cast()),
        0x06 => 0x0046_1190,
        0x07 => 0x0061_30e0,
        0x08 => 0x0087_1a30,
        0x09 => 0x0062_4700,
        0x0a => 0x0061_3790,
        0x0b => 0x0043_c490,
        0x0c => 0x004e_a950,
        0x0d => 0x0045_c650,
        0x0e => return Some(fn_00461170(e, this).cast()),
        0x11 => 0x0063_77e0,
        0x12 => 0x0046_10f0,
        0x13 => 0x0041_d8a0,
        0x14 => 0x0087_eaa0,
        0x31 => return Some(fn_004611f0(e, this).cast()),
        0x33 => return Some(fn_00461010(e, this).cast()),
        0x35 => 0x0043_6aa0,
        0x36 => 0x0050_0940,
        0x37 => {
            // The list sits 4 bytes into what `004169d0` returns.
            let base = e.call(0x0041_69d0, &args![this]).u32();
            return Some(Ptr::new(base + 4));
        }
        0x41 => return Some(fn_00460140(e, this).cast()),
        0x47 => 0x0045_5600,
        0x49 => 0x0041_3f40,
        0x4a => return Some(fn_004610b0(e, this).cast()),
        0x4b => 0x0046_1070,
        0x4d => return Some(fn_00461090(e, this).cast()),
        0x4e => 0x0045_a730,
        0x4f => 0x0087_4670,
        0x51 => 0x0046_0ff0,
        0x52 => return Some(fn_00460fd0(e, this).cast()),
        0x53 => 0x0089_1170,
        0x54 => 0x0046_10d0,
        0x55 => return Some(fn_00461230(e, this).cast()),
        0x56 => 0x0046_0fb0,
        0x57 => 0x0045_a330,
        0x5a => return Some(fn_00461030(e, this).cast()),
        0x5b => 0x0046_1110,
        0x5d => return Some(fn_004611b0(e, this).cast()),
        0x5e => return Some(fn_004611d0(e, this).cast()),
        0x5f => 0x004a_0d10,
        0x61 => 0x0046_1130,
        0x62 => return Some(fn_00461250(e, this).cast()),
        0x63 => 0x009d_9f40,
        0x65 => 0x0046_1270,
        0x66 => return Some(fn_00460090(e, this).cast()),
        0x68 => 0x009c_1a50,
        0x6a => 0x0040_77e0,
        0x6b => 0x0050_3650,
        0x6d => 0x0098_4250,
        0x6e => 0x0046_1290,
        0x6f => return Some(fn_004600b0(e, this).cast()),
        0x70 => return Some(fn_004600d0(e, this).cast()),
        0x71 => 0x0046_1150,
        0x72 => 0x0098_4230,
        0x75 => 0x0051_4f30,
        0x76 => 0x0050_6390,
        0x77 => 0x0062_d2f0,
        0x78 => return Some(fn_00461050(e, this).cast()),
        _ => return None,
    };
    Some(e.call(accessor, &args![this]).ptr())
}

// Translated from 004603b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESDataHandler::AddFormToDataHandler` (Xbox PDB): files a newly
/// created or loaded form in the handler. Returns false for a null form and
/// for a form whose type nothing handles. By the form's type byte
/// (`TESForm +4`):
/// - most types are appended to their own `BSSimpleList` (`list_for_form_type`);
/// - `0x39` (a cell) is appended to `arrayInteriorCells` when its flag
///   `00425fd0` says so;
/// - `0x10` is cast to `EffectSetting` and given to `00409060`;
/// - `0x3a`, `0x3d` to `0x40` and `0x69` are cast to `TESObjectREFR` and
///   added to the cell they are in (`TESObjectCELL::AddReference`); a
///   reference with no parent cell gets the one found at its position in the
///   world space; persistence is updated with `TESObjectREFR::SetRefPersists`
///   and `00564eb0`;
/// - `0x58` goes to `AddAddonNode` and then also takes the last case's path;
/// - any other type is cast to `TESObject` and added to the object list, and
///   if it is not one the message `FORMS: Unknown form type ...` is logged and
///   false is returned.
pub fn tes_data_handler_add_form_to_data_handler(
    e: &mut Engine,
    this: Ptr<TESDataHandler>,
    form: Ptr,
) -> bool {
    if form.is_null() {
        return false;
    }
    let form_type = e.call(FORM_GET_TYPE, &args![form]).u32();
    if let Some(list) = list_for_form_type(e, this, form_type) {
        list_add(e, list, form);
        return true;
    }
    match form_type {
        0x10 => {
            let setting = e
                .call(
                    DYNAMIC_CAST,
                    &args![
                        form,
                        0u32,
                        FORM_TYPE_DESCRIPTOR,
                        EFFECT_SETTING_TYPE_DESCRIPTOR,
                        0u32
                    ],
                )
                .ptr::<()>();
            if !setting.is_null() {
                e.call(0x0040_9060, &args![setting]);
            }
            true
        }
        0x39 => {
            if e.call(0x0042_5fd0, &args![form]).bool() {
                let cells = this.at(TESDataHandler::arrayInteriorCells);
                let count = e.call(ARRAY_SIZE, &args![cells]).u32();
                e.with_stack(4, |e, slot| {
                    e.mem.set_u32(slot.addr(), form.addr());
                    e.call(ARRAY_SET_AT_GROW, &args![cells, count, slot]);
                });
            }
            true
        }
        0x3a | 0x3d | 0x3e | 0x3f | 0x40 | 0x69 => {
            add_reference_to_cell(e, this, form);
            true
        }
        0x58 => {
            e.call(0x0046_1820, &args![this, form]);
            add_form_to_object_list(e, this, form)
        }
        _ => add_form_to_object_list(e, this, form),
    }
}

/// The reference cases of `AddFormToDataHandler`: finds the reference's cell
/// (its parent cell, else the preferred one of `TES`'s `005f36f0`, else the
/// cell at its position in the current world space, `GetCellFromWorldCoord`;
/// a parent cell that is exterior also gets that lookup) and, if there is
/// one, adds the reference to it and updates the reference's persistence.
fn add_reference_to_cell(e: &mut Engine, this: Ptr<TESDataHandler>, form: Ptr) {
    let reference = e
        .call(
            DYNAMIC_CAST,
            &args![
                form,
                0u32,
                FORM_TYPE_DESCRIPTOR,
                REFERENCE_TYPE_DESCRIPTOR,
                0u32
            ],
        )
        .ptr::<()>();
    if reference.is_null() {
        return;
    }
    let tes: u32 = e.global(TES_SINGLETON);
    let mut cell = e.call(0x008d_6f30, &args![reference]).u32();
    if cell == 0 {
        cell = e.call(0x005f_36f0, &args![tes]).u32();
        if cell == 0 {
            cell = cell_at_reference_position(e, this, reference);
        }
    } else if !e.call(0x0042_5fd0, &args![cell]).bool() {
        cell = cell_at_reference_position(e, this, reference);
    }
    if cell != 0 {
        e.call(0x0054_8230, &args![cell, reference, 0u32]);
        if e.call(0x0056_5260, &args![reference]).bool() {
            e.call(0x0056_5480, &args![reference, 1u32]);
        }
        if e.call(0x0056_4e00, &args![reference]).bool() {
            e.call(0x0056_4eb0, &args![reference, 1u32]);
        }
    }
}

/// `GetCellFromWorldCoord(x, y, worldSpace)` for the reference's position
/// (virtual `0x1f4` of the reference returns the address of its position;
/// x is the first float, y the second) in the world space of `TES`
/// (`004fd3e0(1)`).
fn cell_at_reference_position(e: &mut Engine, this: Ptr<TESDataHandler>, reference: Ptr) -> u32 {
    let tes: u32 = e.global(TES_SINGLETON);
    let world_space = e.call(0x004f_d3e0, &args![tes, 1u32]).u32();
    let position = e.vcall(reference.addr(), 0x1f4, &args![]).u32();
    let y = e.mem.f32(position + 4);
    let position = e.vcall(reference.addr(), 0x1f4, &args![]).u32();
    let x = e.mem.f32(position);
    e.call(0x0046_1bc0, &args![this, x, y, world_space]).u32()
}

/// The last case of `AddFormToDataHandler`: the form is cast to `TESObject`
/// and added to the handler's object list; when it is not one, the unknown
/// type is logged and false returned.
fn add_form_to_object_list(e: &mut Engine, this: Ptr<TESDataHandler>, form: Ptr) -> bool {
    let object = e
        .call(
            DYNAMIC_CAST,
            &args![
                form,
                0u32,
                FORM_TYPE_DESCRIPTOR,
                OBJECT_TYPE_DESCRIPTOR,
                0u32
            ],
        )
        .ptr::<()>();
    if object.is_null() {
        let form_type = e.call(FORM_GET_TYPE, &args![form]).u32();
        let type_name = e.call(0x0046_12b0, &args![form_type]).u32();
        e.call(LOG_MESSAGE, &args![UNKNOWN_FORM_TYPE_FORMAT, type_name]);
        return false;
    }
    let object_list = e.get(this, TESDataHandler::pObjectList);
    e.call(OBJECT_LIST_ADD, &args![object_list, object]);
    true
}

// Translated from 00460fd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of `listDebris` (`this + 0x170`).
pub fn fn_00460fd0(_e: &mut Engine, this: Ptr<TESDataHandler>) -> Ptr<BSSimpleList> {
    this.at(TESDataHandler::listDebris)
}

// Translated from 00461010 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of `listProjectiles` (`this + 0x140`).
pub fn fn_00461010(_e: &mut Engine, this: Ptr<TESDataHandler>) -> Ptr<BSSimpleList> {
    this.at(TESDataHandler::listProjectiles)
}

// Translated from 00461030 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of `listRadiation` (`this + 0x150`).
pub fn fn_00461030(_e: &mut Engine, this: Ptr<TESDataHandler>) -> Ptr<BSSimpleList> {
    this.at(TESDataHandler::listRadiation)
}

// Translated from 00461050 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of `listSleepDeprevation` (`this + 0x168`).
pub fn fn_00461050(_e: &mut Engine, this: Ptr<TESDataHandler>) -> Ptr<BSSimpleList> {
    this.at(TESDataHandler::listSleepDeprevation)
}

// Translated from 00461090 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of `animObjects` (`this + 0x1a0`).
pub fn fn_00461090(_e: &mut Engine, this: Ptr<TESDataHandler>) -> Ptr<BSSimpleList> {
    this.at(TESDataHandler::animObjects)
}

// Translated from 004610b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of `listCombatStyles` (`this + 0x120`).
pub fn fn_004610b0(_e: &mut Engine, this: Ptr<TESDataHandler>) -> Ptr<BSSimpleList> {
    this.at(TESDataHandler::listCombatStyles)
}

// Translated from 00461170 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of `listAcousticSpaces` (`this + 0xd8`).
pub fn fn_00461170(_e: &mut Engine, this: Ptr<TESDataHandler>) -> Ptr<BSSimpleList> {
    this.at(TESDataHandler::listAcousticSpaces)
}

// Translated from 004611b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of `listVoiceTypes` (`this + 0xf0`).
pub fn fn_004611b0(_e: &mut Engine, this: Ptr<TESDataHandler>) -> Ptr<BSSimpleList> {
    this.at(TESDataHandler::listVoiceTypes)
}

// Translated from 004611d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of `listImpactData` (`this + 0xf8`).
pub fn fn_004611d0(_e: &mut Engine, this: Ptr<TESDataHandler>) -> Ptr<BSSimpleList> {
    this.at(TESDataHandler::listImpactData)
}

// Translated from 004611f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of `listNotes` (`this + 0x188`).
pub fn fn_004611f0(_e: &mut Engine, this: Ptr<TESDataHandler>) -> Ptr<BSSimpleList> {
    this.at(TESDataHandler::listNotes)
}

// Translated from 00461210 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of `listMenuIcons` (`this + 0x198`).
pub fn fn_00461210(_e: &mut Engine, this: Ptr<TESDataHandler>) -> Ptr<BSSimpleList> {
    this.at(TESDataHandler::listMenuIcons)
}

// Translated from 00461230 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of `listListForms` (`this + 0x190`).
pub fn fn_00461230(_e: &mut Engine, this: Ptr<TESDataHandler>) -> Ptr<BSSimpleList> {
    this.at(TESDataHandler::listListForms)
}

// Translated from 00461250 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of `listMessages` (`this + 0x1a8`).
pub fn fn_00461250(_e: &mut Engine, this: Ptr<TESDataHandler>) -> Ptr<BSSimpleList> {
    this.at(TESDataHandler::listMessages)
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x0040fbe0, error()),
        entry!(
            0x0045d270,
            tes_data_handler_tes_data_handler(Ptr<TESDataHandler>) -> Ptr<TESDataHandler>
        ),
        entry!(0x0045d930, fn_0045d930(Ptr)),
        entry!(0x0045d950, fn_0045d950(Ptr)),
        entry!(0x0045d970, fn_0045d970(Ptr<TESDataHandler>)),
        entry!(0x0045df10, fn_0045df10(Ptr, u32) -> Ptr),
        entry!(0x0045df40, fn_0045df40(Ptr, u32) -> Ptr),
        entry!(0x0045df70, fn_0045df70(Ptr, u32) -> Ptr),
        entry!(0x0045dfa0, fn_0045dfa0(Ptr<TESDataHandler>)),
        entry!(
            0x0045dfc0,
            fn_0045dfc0(Ptr<TESDataHandler>) -> Ptr<BSSimpleList>
        ),
        entry!(
            0x0045dfe0,
            tes_data_handler_clear_data(Ptr<TESDataHandler>) -> bool
        ),
        entry!(
            0x00460090,
            fn_00460090(Ptr<TESDataHandler>) -> Ptr<BSSimpleList>
        ),
        entry!(
            0x004600b0,
            fn_004600b0(Ptr<TESDataHandler>) -> Ptr<BSSimpleList>
        ),
        entry!(
            0x004600d0,
            fn_004600d0(Ptr<TESDataHandler>) -> Ptr<BSSimpleList>
        ),
        entry!(0x004600f0, fn_004600f0(Ptr<TESDataHandler>) -> bool),
        entry!(0x00460110, fn_00460110(Ptr)),
        entry!(
            0x00460140,
            fn_00460140(Ptr<TESDataHandler>) -> Ptr<BSSimpleList>
        ),
        entry!(0x00460160, fn_00460160()),
        entry!(0x00460170, fn_00460170(Ptr)),
        entry!(0x00460190, fn_00460190()),
        entry!(0x004601a0, fn_004601a0(Ptr, u32) -> Ptr),
        entry!(0x004601d0, tes_data_handler_load_form(Ptr, Ptr) -> bool),
        entry!(0x00460250, fn_00460250(Ptr) -> bool),
        entry!(
            0x00460270,
            tes_data_handler_save_form(Ptr<TESDataHandler>, Ptr, u8) -> bool
        ),
        entry!(0x00460340, fn_00460340(Ptr) -> bool),
        entry!(0x00460360, fn_00460360(Ptr<TESDataHandler>)),
        entry!(
            0x004603b0,
            tes_data_handler_add_form_to_data_handler(Ptr<TESDataHandler>, Ptr) -> bool
        ),
        entry!(
            0x00460fd0,
            fn_00460fd0(Ptr<TESDataHandler>) -> Ptr<BSSimpleList>
        ),
        entry!(
            0x00461010,
            fn_00461010(Ptr<TESDataHandler>) -> Ptr<BSSimpleList>
        ),
        entry!(
            0x00461030,
            fn_00461030(Ptr<TESDataHandler>) -> Ptr<BSSimpleList>
        ),
        entry!(
            0x00461050,
            fn_00461050(Ptr<TESDataHandler>) -> Ptr<BSSimpleList>
        ),
        entry!(
            0x00461090,
            fn_00461090(Ptr<TESDataHandler>) -> Ptr<BSSimpleList>
        ),
        entry!(
            0x004610b0,
            fn_004610b0(Ptr<TESDataHandler>) -> Ptr<BSSimpleList>
        ),
        entry!(
            0x00461170,
            fn_00461170(Ptr<TESDataHandler>) -> Ptr<BSSimpleList>
        ),
        entry!(
            0x004611b0,
            fn_004611b0(Ptr<TESDataHandler>) -> Ptr<BSSimpleList>
        ),
        entry!(
            0x004611d0,
            fn_004611d0(Ptr<TESDataHandler>) -> Ptr<BSSimpleList>
        ),
        entry!(
            0x004611f0,
            fn_004611f0(Ptr<TESDataHandler>) -> Ptr<BSSimpleList>
        ),
        entry!(
            0x00461210,
            fn_00461210(Ptr<TESDataHandler>) -> Ptr<BSSimpleList>
        ),
        entry!(
            0x00461230,
            fn_00461230(Ptr<TESDataHandler>) -> Ptr<BSSimpleList>
        ),
        entry!(
            0x00461250,
            fn_00461250(Ptr<TESDataHandler>) -> Ptr<BSSimpleList>
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    fn ret(value: u32) -> Ret {
        Ret {
            eax: value,
            ..Ret::default()
        }
    }

    /// An engine with the pages holding the exe globals this unit reads and
    /// writes, and a zeroed `TESDataHandler`.
    fn engine() -> (Engine, Ptr<TESDataHandler>) {
        let mut e = Engine::new();
        for page in [
            0x011c_5000,
            0x011c_9000,
            0x011c_a000,
            0x011c_b000,
            0x011d_e000,
        ] {
            e.map(page, 0x1000);
        }
        let this = e.new_object::<TESDataHandler>();
        (e, this)
    }

    /// Registers a double that does nothing and returns 0 for each address.
    fn do_nothing(e: &mut Engine, addrs: &[u32]) {
        for &addr in addrs {
            e.register(addr, |_, _| Ret::default());
        }
    }

    fn start_log(e: &mut Engine) {
        e.call_log = Some(vec![]);
    }

    /// The argument words of every logged call to `addr`.
    fn calls(e: &Engine, addr: u32) -> Vec<Vec<u32>> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .filter(|(called, _)| *called == addr)
            .map(|(_, words)| words.clone())
            .collect()
    }

    /// The logged calls (address, argument words) after the first (the call
    /// under test), without those to the addresses in `except`.
    fn log_without(e: &Engine, except: &[u32]) -> Vec<(u32, Vec<u32>)> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .skip(1)
            .filter(|(addr, _)| !except.contains(addr))
            .cloned()
            .collect()
    }

    /// A vtable at `vtable` with the given (byte offset, target) slots; every
    /// other slot points at an address nobody registered, so a call through
    /// it stops the test. Returns an object (a zeroed 0x80-byte block)
    /// using it.
    fn object_with(e: &mut Engine, vtable: u32, slots: &[(u32, u32)]) -> u32 {
        let mut table = vec![0x7fff_0000u32; 0x80];
        for &(offset, target) in slots {
            table[offset as usize / 4] = target;
        }
        e.put_vtable(vtable, &table);
        let object = e.mem.alloc(0x80);
        e.mem.set_u32(object, vtable);
        object
    }

    /// Registers a double at `target` that records (this, first argument)
    /// of each call in the returned log.
    fn recording_double(e: &mut Engine, target: u32) -> Rc<RefCell<Vec<(u32, u32)>>> {
        let log = Rc::new(RefCell::new(Vec::new()));
        let shared = log.clone();
        e.register_double(target, move |_, a| {
            shared
                .borrow_mut()
                .push((a[0], a.get(1).copied().unwrap_or(0)));
            Ret::default()
        });
        log
    }

    // ---------------------------------------------------------------
    // The small functions.
    // ---------------------------------------------------------------

    #[test]
    fn error_does_nothing() {
        let mut e = Engine::new();
        start_log(&mut e);
        e.call(0x0040_fbe0, &args![]);
        assert_eq!(e.call_log.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn array_destructor_bodies_call_their_callees() {
        let (mut e, _) = engine();
        do_nothing(&mut e, &[CELL_ARRAY_DESTRUCT, ADDON_NODE_ARRAY_DESTRUCT]);
        start_log(&mut e);
        e.call(0x0045_d930, &args![0x1234u32]);
        e.call(0x0045_d950, &args![0x5678u32]);
        assert_eq!(calls(&e, CELL_ARRAY_DESTRUCT), vec![vec![0x1234]]);
        assert_eq!(calls(&e, ADDON_NODE_ARRAY_DESTRUCT), vec![vec![0x5678]]);
    }

    /// Checks a scalar deleting destructor: the body always runs, the block
    /// is freed only when bit 0 of the flags is set, `this` is returned.
    fn check_deleting_destructor(function: u32, body: u32) {
        let (mut e, _) = engine();
        do_nothing(&mut e, &[body, OPERATOR_DELETE]);
        start_log(&mut e);
        let result = e.call(function, &args![0x4000u32, 1u32]);
        assert_eq!(result.u32(), 0x4000);
        assert_eq!(calls(&e, body), vec![vec![0x4000]]);
        assert_eq!(calls(&e, OPERATOR_DELETE), vec![vec![0x4000]]);
        let result = e.call(function, &args![0x5000u32, 2u32]);
        assert_eq!(result.u32(), 0x5000);
        assert_eq!(calls(&e, body).len(), 2);
        assert_eq!(calls(&e, OPERATOR_DELETE).len(), 1, "bit 0 clear: no free");
    }

    #[test]
    fn object_list_deleting_destructor() {
        check_deleting_destructor(0x0045_df10, OBJECT_LIST_DESTRUCT);
    }

    #[test]
    fn idle_manager_deleting_destructor() {
        check_deleting_destructor(0x0045_df40, IDLE_MANAGER_DESTRUCT);
    }

    #[test]
    fn camera_path_manager_deleting_destructor() {
        check_deleting_destructor(0x0045_df70, CAMERA_PATH_MANAGER_DESTRUCT);
    }

    #[test]
    fn file_deleting_destructor() {
        check_deleting_destructor(0x0046_01a0, FILE_DESTRUCT);
    }

    #[test]
    fn files_list_destroy_gets_the_list_address() {
        let (mut e, this) = engine();
        do_nothing(&mut e, &[FILES_LIST_DESTROY]);
        start_log(&mut e);
        e.call(0x0045_dfa0, &args![this]);
        assert_eq!(
            calls(&e, FILES_LIST_DESTROY),
            vec![vec![this.addr() + 0x210]]
        );
    }

    /// Checks one accessor: it returns `this + offset`.
    fn check_accessor(addr: u32, offset: u32) {
        let (mut e, this) = engine();
        assert_eq!(e.call(addr, &args![this]).u32(), this.addr() + offset);
    }

    #[test]
    fn accessor_0045dfc0() {
        check_accessor(0x0045_dfc0, 0x210);
    }
    #[test]
    fn accessor_00460090() {
        check_accessor(0x0046_0090, 0x1b8);
    }
    #[test]
    fn accessor_004600b0() {
        check_accessor(0x0046_00b0, 0x1c8);
    }
    #[test]
    fn accessor_004600d0() {
        check_accessor(0x0046_00d0, 0x1d0);
    }
    #[test]
    fn accessor_00460140() {
        check_accessor(0x0046_0140, 0x10);
    }
    #[test]
    fn accessor_00460fd0() {
        check_accessor(0x0046_0fd0, 0x170);
    }
    #[test]
    fn accessor_00461010() {
        check_accessor(0x0046_1010, 0x140);
    }
    #[test]
    fn accessor_00461030() {
        check_accessor(0x0046_1030, 0x150);
    }
    #[test]
    fn accessor_00461050() {
        check_accessor(0x0046_1050, 0x168);
    }
    #[test]
    fn accessor_00461090() {
        check_accessor(0x0046_1090, 0x1a0);
    }
    #[test]
    fn accessor_004610b0() {
        check_accessor(0x0046_10b0, 0x120);
    }
    #[test]
    fn accessor_00461170() {
        check_accessor(0x0046_1170, 0xd8);
    }
    #[test]
    fn accessor_004611b0() {
        check_accessor(0x0046_11b0, 0xf0);
    }
    #[test]
    fn accessor_004611d0() {
        check_accessor(0x0046_11d0, 0xf8);
    }
    #[test]
    fn accessor_004611f0() {
        check_accessor(0x0046_11f0, 0x188);
    }
    #[test]
    fn accessor_00461210() {
        check_accessor(0x0046_1210, 0x198);
    }
    #[test]
    fn accessor_00461230() {
        check_accessor(0x0046_1230, 0x190);
    }
    #[test]
    fn accessor_00461250() {
        check_accessor(0x0046_1250, 0x1a8);
    }

    #[test]
    fn master_save_flag_is_read_from_0x618() {
        let (mut e, this) = engine();
        assert!(!e.call(0x0046_00f0, &args![this]).bool());
        e.mem.set_u8(this.addr() + 0x618, 1);
        assert!(e.call(0x0046_00f0, &args![this]).bool());
    }

    #[test]
    fn form_list_clear_gets_the_address_of_the_list_at_0x10() {
        let (mut e, _) = engine();
        do_nothing(&mut e, &[FORM_LIST_CLEAR]);
        start_log(&mut e);
        e.call(0x0046_0110, &args![0x3000u32]);
        assert_eq!(calls(&e, FORM_LIST_CLEAR), vec![vec![0x3010]]);
    }

    #[test]
    fn one_line_functions_clear_their_words() {
        let (mut e, _) = engine();
        e.set_global(FLAG_011CA830, 7u32);
        e.set_global(FLAG_011C9520, 9u32);
        e.call(0x0046_0160, &args![]);
        assert_eq!(e.global::<u32>(FLAG_011CA830), 0);
        assert_eq!(e.global::<u32>(FLAG_011C9520), 9);
        e.call(0x0046_0190, &args![]);
        assert_eq!(e.global::<u32>(FLAG_011C9520), 0);
        let tes = Ptr::<()>::new(e.mem.alloc(0x100));
        e.mem.set_u32(tes.addr() + 0x88, 5);
        e.mem.set_u32(tes.addr() + 0x8c, 6);
        e.call(0x0046_0170, &args![tes]);
        assert_eq!(e.mem.u32(tes.addr() + 0x88), 0);
        assert_eq!(e.mem.u32(tes.addr() + 0x8c), 6);
    }

    #[test]
    fn form_flag_getters_read_bits_0_and_1() {
        let (mut e, _) = engine();
        let form = Ptr::<()>::new(e.mem.alloc(0x100));
        for (flags, bit_0, bit_1) in [
            (0u32, false, false),
            (1, true, false),
            (2, false, true),
            (3, true, true),
            (0x20, false, false),
        ] {
            e.mem.set_u32(form.addr() + 8, flags);
            assert_eq!(e.call(0x0046_0250, &args![form]).bool(), bit_0);
            assert_eq!(e.call(0x0046_0340, &args![form]).bool(), bit_1);
        }
    }

    #[test]
    fn close_tes_runs_for_each_file_until_a_null_item() {
        let (mut e, this) = engine();
        // `listFiles` is the node at `this + 0x210`: file 0xa1, then a node
        // at 0x9100 with file 0xb2, then one at 0x9200 whose item is null.
        e.map(0x9000, 0x1000);
        let head = this.addr() + 0x210;
        let nodes: HashMap<u32, (u32, u32)> = HashMap::from([
            (head, (0xa1, 0x9100)),
            (0x9100, (0xb2, 0x9200)),
            (0x9200, (0, 0)),
        ]);
        let slot = e.mem.alloc(4);
        let table = nodes.clone();
        e.register_double(LIST_HEAD_ITEM, move |e, a| {
            e.mem.set_u32(slot, table[&a[0]].0);
            ret(slot)
        });
        e.register_double(LIST_NEXT, move |_, a| ret(nodes[&a[0]].1));
        do_nothing(&mut e, &[FILE_CLOSE]);
        start_log(&mut e);
        e.call(0x0046_0360, &args![this]);
        assert_eq!(calls(&e, FILE_CLOSE), vec![vec![0xa1], vec![0xb2]]);
    }

    // ---------------------------------------------------------------
    // Constructor and destructor.
    // ---------------------------------------------------------------

    /// The constructors of the owned objects: each returns its `this`.
    fn register_owned_constructors(e: &mut Engine) {
        for addr in [
            OBJECT_LIST_CONSTRUCT,
            REGION_LIST_CONSTRUCT,
            REGION_DATA_MANAGER_CONSTRUCT,
            IDLE_MANAGER_CONSTRUCT,
            CAMERA_PATH_MANAGER_CONSTRUCT,
        ] {
            e.register(addr, |_, a| ret(a[0]));
        }
    }

    #[test]
    fn constructor_builds_the_lists_and_the_owned_objects() {
        let (mut e, this) = engine();
        do_nothing(
            &mut e,
            &[
                LIST_CONSTRUCT,
                CELL_ARRAY_CONSTRUCT,
                ADDON_NODE_ARRAY_CONSTRUCT,
                BAD_FORM_LIST_CONSTRUCT,
                CELL_ARRAY_SET_GROW_BY,
            ],
        );
        register_owned_constructors(&mut e);
        // Garbage that the constructor must overwrite.
        e.mem.write(this.addr() + 0x21c, &[0xaa; 0x3fc]);
        for offset in [0x618, 0x619, 0x61a, 0x61b, 0x61c, 0x61d, 0x621, 0x622, 0] {
            e.mem.set_u8(this.addr() + offset, 0x55);
        }
        e.mem.set_u32(this.addr() + 0x62c, 0x1234);
        start_log(&mut e);

        let result = e.call(0x0045_d270, &args![this]);
        assert_eq!(result.u32(), this.addr());

        // 58 form lists in order, then the files list.
        let mut expected: Vec<Vec<u32>> = (0x08..=0x1d0)
            .step_by(8)
            .map(|offset| vec![this.addr() + offset])
            .collect();
        expected.push(vec![this.addr() + 0x210]);
        assert_eq!(expected.len(), 59);
        assert_eq!(calls(&e, LIST_CONSTRUCT), expected);
        assert_eq!(
            calls(&e, CELL_ARRAY_CONSTRUCT),
            vec![vec![this.addr() + 0x1dc, 0, 1]]
        );
        assert_eq!(
            calls(&e, ADDON_NODE_ARRAY_CONSTRUCT),
            vec![vec![this.addr() + 0x1ec, 0, 1]]
        );
        assert_eq!(
            calls(&e, BAD_FORM_LIST_CONSTRUCT),
            vec![vec![this.addr() + 0x1fc]]
        );
        assert_eq!(
            calls(&e, CELL_ARRAY_SET_GROW_BY),
            vec![vec![this.addr() + 0x1dc, 100]]
        );

        // The fields.
        assert_eq!(e.get(this, TESDataHandler::iNextID), 0x800);
        assert_eq!(e.get(this, TESDataHandler::iNumCompile), 0);
        assert!(e.get(this, TESDataHandler::pActiveFile).is_null());
        assert!(e.get(this, TESDataHandler::bHasDesiredFiles));
        for flag in [
            TESDataHandler::bMasterSave,
            TESDataHandler::bSaveLoadGame,
            TESDataHandler::bSaveLoad,
            TESDataHandler::bAutoSaving,
            TESDataHandler::bExportingPlugin,
            TESDataHandler::bClearingData,
            TESDataHandler::bDontRemoveIDs,
            TESDataHandler::bCheckingModels,
            TESDataHandler::bLoadingFiles,
        ] {
            assert!(!e.get(this, flag));
        }
        assert_eq!(e.get(this, TESDataHandler::cDLCFlags), 0);
        assert_eq!(e.get(this, TESDataHandler::ucGameSettingsLoadState), 0);
        assert!(e
            .mem
            .bytes(this.addr() + 0x21c, 0x3fc)
            .iter()
            .all(|&byte| byte == 0));
        assert!(e.get(this, TESDataHandler::pBarterContainer).is_null());
        assert!(e.get(this, TESDataHandler::pRecipeContainer).is_null());
        assert!(e.get(this, TESDataHandler::pSpotterShader).is_null());
        assert!(e.get(this, TESDataHandler::pItemDetectedShader).is_null());
        assert!(e.get(this, TESDataHandler::pCateyeMobileShader).is_null());

        // The owned objects, constructed in their fresh blocks.
        let object_list = e.get(this, TESDataHandler::pObjectList);
        let region_list = e.get(this, TESDataHandler::pRegionList);
        let region_data = e.get(this, TESDataHandler::pRegionDataManager);
        assert_eq!(
            calls(&e, OBJECT_LIST_CONSTRUCT),
            vec![vec![object_list.addr(), 1]]
        );
        assert_eq!(
            calls(&e, REGION_LIST_CONSTRUCT),
            vec![vec![region_list.addr(), 1]]
        );
        assert_eq!(
            calls(&e, REGION_DATA_MANAGER_CONSTRUCT),
            vec![vec![region_data.addr()]]
        );
        assert_eq!(e.mem.block_size(object_list.addr()), Some(0x10));
        assert_eq!(e.mem.block_size(region_list.addr()), Some(0x10));
        assert_eq!(e.mem.block_size(region_data.addr()), Some(8));
        let idle = e.global::<u32>(IDLE_MANAGER);
        let camera = e.global::<u32>(CAMERA_PATH_MANAGER);
        assert_eq!(calls(&e, IDLE_MANAGER_CONSTRUCT), vec![vec![idle]]);
        assert_eq!(calls(&e, CAMERA_PATH_MANAGER_CONSTRUCT), vec![vec![camera]]);
        assert_eq!(e.mem.block_size(idle), Some(0x28));
        assert_eq!(e.mem.block_size(camera), Some(0x28));
    }

    #[test]
    fn constructor_stores_null_when_an_allocation_fails() {
        let (mut e, this) = engine();
        do_nothing(
            &mut e,
            &[
                LIST_CONSTRUCT,
                CELL_ARRAY_CONSTRUCT,
                ADDON_NODE_ARRAY_CONSTRUCT,
                BAD_FORM_LIST_CONSTRUCT,
                CELL_ARRAY_SET_GROW_BY,
            ],
        );
        register_owned_constructors(&mut e);
        e.register(OPERATOR_NEW, |_, _| Ret::default());
        e.set_global(IDLE_MANAGER, 0x77u32);
        start_log(&mut e);
        e.call(0x0045_d270, &args![this]);
        assert!(e.get(this, TESDataHandler::pObjectList).is_null());
        assert!(e.get(this, TESDataHandler::pRegionList).is_null());
        assert!(e.get(this, TESDataHandler::pRegionDataManager).is_null());
        assert_eq!(e.global::<u32>(IDLE_MANAGER), 0);
        assert_eq!(e.global::<u32>(CAMERA_PATH_MANAGER), 0);
        for constructor in [
            OBJECT_LIST_CONSTRUCT,
            REGION_LIST_CONSTRUCT,
            REGION_DATA_MANAGER_CONSTRUCT,
            IDLE_MANAGER_CONSTRUCT,
            CAMERA_PATH_MANAGER_CONSTRUCT,
        ] {
            assert!(calls(&e, constructor).is_empty());
        }
    }

    /// Everything `~TESDataHandler` calls.
    const DESTRUCTOR_CALLEES: [u32; 11] = [
        INVENTORY_CHANGES_DELETE,
        FILES_LIST_DESTROY,
        OBJECT_LIST_DESTRUCT,
        IDLE_MANAGER_DESTRUCT,
        CAMERA_PATH_MANAGER_DESTRUCT,
        OPERATOR_DELETE,
        ALL_FORMS_CLEAR,
        LIST_DESTRUCT,
        BAD_FORM_LIST_DESTRUCT,
        CELL_ARRAY_DESTRUCT,
        ADDON_NODE_ARRAY_DESTRUCT,
    ];

    #[test]
    fn destructor_deletes_the_owned_objects_then_the_members() {
        let (mut e, this) = engine();
        do_nothing(&mut e, &DESTRUCTOR_CALLEES);
        let region_destroy = 0x7000_0000;
        let region_log = recording_double(&mut e, region_destroy);
        let region_list = object_with(&mut e, 0x7100_0000, &[(0, region_destroy)]);
        e.set(this, TESDataHandler::pBarterContainer, Ptr::new(0xb0b));
        e.set(this, TESDataHandler::pObjectList, Ptr::new(0x0b1));
        e.set(this, TESDataHandler::pRegionList, Ptr::new(region_list));
        e.set(this, TESDataHandler::pRegionDataManager, Ptr::new(0xd0d));
        e.set_global(IDLE_MANAGER, 0x1d1eu32);
        e.set_global(CAMERA_PATH_MANAGER, 0xca4u32);
        e.set_global(ALL_FORMS_MAP, 0xa11u32);
        start_log(&mut e);

        e.call(0x0045_d970, &args![this]);

        let mut expected: Vec<(u32, Vec<u32>)> = vec![
            (INVENTORY_CHANGES_DELETE, vec![0xb0b, 1]),
            (FILES_LIST_DESTROY, vec![this.addr() + 0x210]),
            (OBJECT_LIST_DESTRUCT, vec![0x0b1]),
            (OPERATOR_DELETE, vec![0x0b1]),
            (region_destroy, vec![region_list, 1]),
            (OPERATOR_DELETE, vec![0xd0d]),
            (IDLE_MANAGER_DESTRUCT, vec![0x1d1e]),
            (OPERATOR_DELETE, vec![0x1d1e]),
            (CAMERA_PATH_MANAGER_DESTRUCT, vec![0xca4]),
            (OPERATOR_DELETE, vec![0xca4]),
            (ALL_FORMS_CLEAR, vec![0xa11]),
            (LIST_DESTRUCT, vec![this.addr() + 0x210]),
            (BAD_FORM_LIST_DESTRUCT, vec![this.addr() + 0x1fc]),
            (ADDON_NODE_ARRAY_DESTRUCT, vec![this.addr() + 0x1ec]),
            (CELL_ARRAY_DESTRUCT, vec![this.addr() + 0x1dc]),
        ];
        for offset in (0x08..=0x1d0).rev().step_by(8) {
            expected.push((LIST_DESTRUCT, vec![this.addr() + offset]));
        }
        assert_eq!(e.call_log.clone().unwrap()[1..], expected[..]);
        assert_eq!(region_log.borrow().as_slice(), &[(region_list, 1)]);
        assert_eq!(e.global::<u32>(IDLE_MANAGER), 0);
        assert_eq!(e.global::<u32>(CAMERA_PATH_MANAGER), 0);
    }

    #[test]
    fn destructor_skips_the_objects_that_are_not_there() {
        let (mut e, this) = engine();
        do_nothing(&mut e, &DESTRUCTOR_CALLEES);
        start_log(&mut e);
        e.call(0x0045_d970, &args![this]);
        let log = e.call_log.clone().unwrap()[1..].to_vec();
        // Only the unconditional calls remain: the files list, the delete of
        // the (null) region data manager, and the member destructors.
        assert_eq!(log[0], (FILES_LIST_DESTROY, vec![this.addr() + 0x210]));
        assert_eq!(log[1], (OPERATOR_DELETE, vec![0]));
        assert_eq!(log[2], (LIST_DESTRUCT, vec![this.addr() + 0x210]));
        assert_eq!(log.len(), 64, "6 calls and the 58 list destructors");
        for skipped in [
            INVENTORY_CHANGES_DELETE,
            OBJECT_LIST_DESTRUCT,
            IDLE_MANAGER_DESTRUCT,
            CAMERA_PATH_MANAGER_DESTRUCT,
            ALL_FORMS_CLEAR,
        ] {
            assert!(calls(&e, skipped).is_empty());
        }
    }

    // ---------------------------------------------------------------
    // LoadForm and SaveForm.
    // ---------------------------------------------------------------

    /// A form with the flags `flags` whose vtable slots `Load`, `Save`,
    /// `SaveEdit` and `SetAltered` are doubles at `0x7000_0020`, `0x7000_0028`,
    /// `0x7000_0034` and `0x7000_00c8` that answer true (their calls show
    /// in the call log).
    fn form_with_slots(e: &mut Engine, flags: u32) -> u32 {
        let slots = [
            (FORM_VTABLE_LOAD, 0x7000_0020),
            (FORM_VTABLE_SAVE, 0x7000_0028),
            (FORM_VTABLE_SAVE_EDIT, 0x7000_0034),
            (FORM_VTABLE_SET_ALTERED, 0x7000_00c8),
        ];
        let form = object_with(e, 0x7100_1000, &slots);
        e.mem.set_u32(form + 8, flags);
        for (_, target) in slots {
            e.register(target, |_, _| ret(1));
        }
        form
    }

    #[test]
    fn load_form_sets_the_master_flag_and_marks_the_active_file() {
        let (mut e, _) = engine();
        e.register(FILE_GET_MASTER, |_, a| ret((a[0] == 0xf11e) as u32));
        e.register(FILE_GET_ACTIVE, |_, a| ret((a[0] == 0xac71) as u32));
        e.register(FORM_SET_FLAG_BIT_0, |_, _| Ret::default());

        // Flag bit 0 already set: the load result is returned, the flag is
        // set to 1 whatever the file.
        let form = form_with_slots(&mut e, 1);
        start_log(&mut e);
        assert!(e.call(0x0046_01d0, &args![form, 0x0f11u32]).bool());
        assert_eq!(calls(&e, 0x7000_0020), vec![vec![form, 0x0f11]]);
        assert_eq!(calls(&e, FORM_SET_FLAG_BIT_0), vec![vec![form, 1]]);
        assert!(calls(&e, FILE_GET_MASTER).is_empty());
        assert!(calls(&e, 0x7000_00c8).is_empty());

        // Flag clear and a master file: the flag becomes 1; active file:
        // `SetAltered(1)`.
        let form = form_with_slots(&mut e, 0);
        start_log(&mut e);
        e.call(0x0046_01d0, &args![form, 0xf11eu32]);
        assert_eq!(calls(&e, FORM_SET_FLAG_BIT_0), vec![vec![form, 1]]);
        assert!(calls(&e, 0x7000_00c8).is_empty());

        // Flag clear and a plain file that is the active one: flag 0, altered.
        start_log(&mut e);
        e.call(0x0046_01d0, &args![form, 0xac71u32]);
        assert_eq!(calls(&e, FORM_SET_FLAG_BIT_0), vec![vec![form, 0]]);
        assert_eq!(calls(&e, 0x7000_00c8), vec![vec![form, 1]]);
    }

    #[test]
    fn save_form_chooses_the_virtual_by_the_form_flags() {
        let (mut e, this) = engine();
        let active_file = 0xac71;
        e.register(FILE_GET_MASTER, |_, _| ret(0));
        e.register(
            FORM_HAS_FLAG_BIT_5,
            |e, a| ret(e.mem.u32(a[0] + 8) >> 5 & 1),
        );
        // The form's last file is stored at +0x30 of the test form.
        e.register(FORM_GET_FILE, |e, a| {
            assert_eq!(a[1] as i32, -1);
            ret(e.mem.u32(a[0] + 0x30))
        });

        // No active file: false, nothing called.
        let form = form_with_slots(&mut e, 0);
        e.mem.set_u32(form + 0x30, active_file);
        start_log(&mut e);
        assert!(!e.call(0x0046_0270, &args![this, form, 0u32]).bool());
        assert!(calls(&e, 0x7000_0028).is_empty());

        e.set(this, TESDataHandler::pActiveFile, Ptr::new(active_file));
        // A plain form of the active file: `Save`.
        assert!(e.call(0x0046_0270, &args![this, form, 0u32]).bool());
        assert_eq!(calls(&e, 0x7000_0028), vec![vec![form, active_file]]);
        // Bit 0: `SaveEdit`.
        e.mem.set_u32(form + 8, 1);
        assert!(e.call(0x0046_0270, &args![this, form, 0u32]).bool());
        assert_eq!(calls(&e, 0x7000_0034), vec![vec![form, active_file]]);
        // Bit 5 (deleted) and not bit 0: skipped.
        e.mem.set_u32(form + 8, 0x20);
        assert!(!e.call(0x0046_0270, &args![this, form, 0u32]).bool());
        assert_eq!(calls(&e, 0x7000_0028).len(), 1);
        // A form of another file is refused unless bit 1 is set.
        e.mem.set_u32(form + 8, 0);
        e.mem.set_u32(form + 0x30, 0x0f11);
        assert!(!e.call(0x0046_0270, &args![this, form, 0u32]).bool());
        e.mem.set_u32(form + 8, 2);
        assert!(e.call(0x0046_0270, &args![this, form, 0u32]).bool());
        assert_eq!(calls(&e, 0x7000_0028).len(), 2);
    }

    #[test]
    fn save_form_refuses_forms_of_a_master_active_file_without_bit_1() {
        let (mut e, this) = engine();
        e.register(FILE_GET_MASTER, |_, _| ret(1));
        e.register(FORM_HAS_FLAG_BIT_5, |_, _| ret(0));
        e.register(FORM_GET_FILE, |_, _| ret(0xac71));
        let form = form_with_slots(&mut e, 0);
        e.set(this, TESDataHandler::pActiveFile, Ptr::new(0xac71));
        start_log(&mut e);
        assert!(!e.call(0x0046_0270, &args![this, form, 0u32]).bool());
        e.mem.set_u32(form + 8, 2);
        assert!(e.call(0x0046_0270, &args![this, form, 0u32]).bool());
        assert_eq!(calls(&e, 0x7000_0028), vec![vec![form, 0xac71]]);
    }

    // ---------------------------------------------------------------
    // ClearData.
    // ---------------------------------------------------------------

    /// The offsets of the 58 lists in the order `ClearData` empties them.
    const CLEAR_ORDER: [u32; 58] = [
        0xc8, 0x190, 0x198, 0x48, 0x50, 0x58, 0x60, 0x68, 0x18, 0x30, 0x80, 0x88, 0x98, 0x90, 0xa0,
        0xb8, 0xa8, 0xb0, 0xe8, 0xf0, 0xf8, 0x100, 0x118, 0x108, 0x110, 0x10, 0xd0, 0x1b8, 0x1c8,
        0x1d0, 0xd8, 0x70, 0x1a8, 0x20, 0x28, 0x40, 0x38, 0x8, 0x120, 0x128, 0x1c0, 0x130, 0x1a0,
        0x138, 0x140, 0x148, 0x170, 0x178, 0x150, 0x158, 0x160, 0x168, 0x180, 0x78, 0x1b0, 0x188,
        0xe0, 0xc0,
    ];

    /// The callees of `ClearData` that the tests stand in for with doubles
    /// that do nothing.
    const CLEAR_CALLEES: [u32; 33] = [
        CLEAN_UP_BAD_FORMS,
        0x0086_3db0,
        0x0045_39a0,
        EMBEDDED_OBJECT_RESET,
        GARBAGE_COLLECTOR_CLEAR_ALL,
        0x004f_f190,
        0x0095_2f90,
        0x004c_1c40,
        0x0059_3210,
        0x0058_d710,
        0x0061_a270,
        ARRAY_RESET,
        OBJECT_LIST_CLEAR,
        ADDON_NODE_ARRAY_CLEAR,
        REGION_LIST_CLEAR,
        0x006c_09f0,
        0x0045_af00,
        0x0045_afb0,
        0x004f_6de0,
        0x005f_fd20,
        0x0060_0030,
        CAMERA_PATH_MANAGER_DESTROY_ROOT_PATHS,
        0x0040_8de0,
        0x0066_f110,
        DISABLE_WARNING_COUNT,
        LOG_MESSAGE,
        FILE_CLOSE_ACTIVE,
        FILE_DESTRUCT,
        OPERATOR_DELETE,
        FORM_LIST_CLEAR,
        MAP_SET_AT,
        0x7000_0128,
        0x7000_0130,
    ];

    /// The address the test forms' scalar deleting destructor has.
    const FORM_DESTROY: u32 = 0x7000_0010;

    /// What the list doubles keep: the items of each list (the head first)
    /// and the lists asked for their head item, in order.
    #[derive(Default)]
    struct ListModel {
        items: HashMap<u32, Vec<u32>>,
        visited: Vec<u32>,
    }

    /// Doubles for `GetHead` and `RemoveHead` over a [`ListModel`].
    fn install_list_model(e: &mut Engine) -> Rc<RefCell<ListModel>> {
        let model = Rc::new(RefCell::new(ListModel::default()));
        let slot = e.mem.alloc(4);
        let shared = model.clone();
        e.register_double(LIST_HEAD_ITEM, move |e, a| {
            let mut model = shared.borrow_mut();
            model.visited.push(a[0]);
            let head = model
                .items
                .get(&a[0])
                .and_then(|items| items.first().copied())
                .unwrap_or(0);
            e.mem.set_u32(slot, head);
            ret(slot)
        });
        let shared = model.clone();
        e.register_double(LIST_REMOVE_HEAD, move |_, a| {
            if let Some(items) = shared.borrow_mut().items.get_mut(&a[0]) {
                items.remove(0);
            }
            Ret::default()
        });
        model
    }

    /// An object whose destructor is [`FORM_DESTROY`].
    fn destroyable(e: &mut Engine) -> u32 {
        object_with(e, 0x7100_2000, &[(0x10, FORM_DESTROY)])
    }

    struct ClearSetup {
        e: Engine,
        this: Ptr<TESDataHandler>,
        model: Rc<RefCell<ListModel>>,
        /// (this, flag) of every call to a form's destructor.
        destroyed: Rc<RefCell<Vec<(u32, u32)>>>,
        tes: u32,
        player: u32,
    }

    /// An engine where `ClearData` can run: doubles for what it calls, the
    /// list model, the singletons it uses, and `TES::pAllForms` null.
    fn clear_setup() -> ClearSetup {
        let (mut e, this) = engine();
        do_nothing(&mut e, &CLEAR_CALLEES);
        e.register(TES_GET_EMBEDDED_OBJECT, |_, _| ret(0x1e1e));
        e.register(0x006c_0720, |_, _| ret(0x4e4));
        let model = install_list_model(&mut e);
        let destroyed = recording_double(&mut e, FORM_DESTROY);
        let tes = e.mem.alloc(0x100);
        e.mem.set_u32(tes + 0x88, 5);
        e.set_global(TES_SINGLETON, tes);
        let player = destroyable(&mut e);
        e.set_global(PLAYER_SINGLETON, player);
        e.set_global(OBJECT_011C54C4, 0x0c54c4u32);
        e.set_global(IDLE_MANAGER, 0x1d1eu32);
        e.set_global(CAMERA_PATH_MANAGER, 0xca4u32);
        e.set(this, TESDataHandler::pObjectList, Ptr::new(0x0b1));
        e.set(this, TESDataHandler::pRegionList, Ptr::new(0x0e6));
        ClearSetup {
            e,
            this,
            model,
            destroyed,
            tes,
            player,
        }
    }

    #[test]
    fn clear_data_empties_the_lists_in_the_games_order() {
        let ClearSetup {
            mut e,
            this,
            model,
            destroyed,
            tes,
            player,
        } = clear_setup();
        let cells = this.addr() + 0x1dc;
        let scripts = [destroyable(&mut e), destroyable(&mut e)];
        let cell_objects = [destroyable(&mut e), destroyable(&mut e)];
        let music = destroyable(&mut e);
        let media_set = destroyable(&mut e);
        let media_location = destroyable(&mut e);
        let anim = destroyable(&mut e);
        let topic_info = destroyable(&mut e);
        {
            let mut model = model.borrow_mut();
            let at = |offset: u32| this.addr() + offset;
            model.items.insert(at(0xc8), scripts.to_vec());
            model.items.insert(at(0x1b8), vec![music]);
            model.items.insert(at(0x1c8), vec![media_set]);
            model.items.insert(at(0x1d0), vec![media_location]);
            model.items.insert(at(0x1a0), vec![anim]);
            model.items.insert(at(0x110), vec![topic_info]);
        }
        // The cell array: two cells.
        let slots = e.mem.alloc(8);
        e.mem.set_u32(slots, cell_objects[0]);
        e.mem.set_u32(slots + 4, cell_objects[1]);
        e.register(ARRAY_SIZE, |_, _| ret(2));
        e.register_double(ARRAY_AT, move |_, a| ret(slots + 4 * a[1]));
        for flag in [FLAG_011CB96C, FLAG_011CA53C, 0x011c_a24c, 0x011c_a264] {
            e.set_global(flag, 7u32);
        }
        for word in CLEARED_WORDS {
            e.set_global(word, 7u32);
        }
        e.set_global(FLAG_011CA830, 7u32);
        e.set(this, TESDataHandler::bClearingData, false);
        for shader in [
            TESDataHandler::pSpotterShader,
            TESDataHandler::pItemDetectedShader,
            TESDataHandler::pCateyeMobileShader,
        ] {
            e.set(this, shader, Ptr::new(0x5a));
        }
        start_log(&mut e);

        assert!(e.call(0x0045_dfe0, &args![this]).bool());

        // Forms destroyed, in the order of the lists they were in.
        let expected_destroyed: Vec<(u32, u32)> = [
            scripts[0],
            scripts[1],
            cell_objects[0],
            cell_objects[1],
            music,
            media_set,
            media_location,
            player,
            anim,
        ]
        .iter()
        .map(|&form| (form, 1))
        .collect();
        assert_eq!(destroyed.borrow().as_slice(), expected_destroyed.as_slice());
        assert_eq!(
            e.global::<u32>(PLAYER_SINGLETON),
            0,
            "the player is released"
        );

        // The lists were visited in the game's order, each until empty;
        // `listTopicInfos` was emptied without destroying its item.
        let mut visited = model.borrow().visited.clone();
        visited.dedup();
        let expected_order: Vec<u32> = CLEAR_ORDER.iter().map(|o| this.addr() + o).collect();
        assert_eq!(visited, expected_order);
        assert!(model.borrow().items.values().all(Vec::is_empty));

        // Every other call, in order.
        let idle = 0x1d1e;
        let camera = 0xca4;
        let other_calls = log_without(&e, &[LIST_HEAD_ITEM, LIST_REMOVE_HEAD, FORM_DESTROY]);
        let expected: Vec<(u32, Vec<u32>)> = vec![
            (CLEAN_UP_BAD_FORMS, vec![this.addr()]),
            (0x0086_3db0, vec![0x0c54c4]),
            (0x0045_39a0, vec![tes, 0, 0]),
            (TES_GET_EMBEDDED_OBJECT, vec![tes]),
            (EMBEDDED_OBJECT_RESET, vec![0x1e1e]),
            (GARBAGE_COLLECTOR_CLEAR_ALL, vec![0]),
            (0x004f_f190, vec![]),
            (0x0095_2f90, vec![player]),
            (0x004c_1c40, vec![]),
            (0x0059_3210, vec![]),
            (0x0058_d710, vec![]),
            (0x0061_a270, vec![]),
            (ARRAY_SIZE, vec![cells]),
            (ARRAY_AT, vec![cells, 0]),
            (ARRAY_AT, vec![cells, 1]),
            (ARRAY_RESET, vec![cells, 0]),
            (GARBAGE_COLLECTOR_CLEAR_ALL, vec![0]),
            (OBJECT_LIST_CLEAR, vec![0x0b1]),
            (ADDON_NODE_ARRAY_CLEAR, vec![this.addr() + 0x1ec]),
            (REGION_LIST_CLEAR, vec![0x0e6]),
            (0x006c_0720, vec![]),
            (0x006c_09f0, vec![0x4e4]),
            (0x0045_af00, vec![tes]),
            (0x005f_fd20, vec![idle]),
            (CAMERA_PATH_MANAGER_DESTROY_ROOT_PATHS, vec![camera]),
            (0x0040_8de0, vec![]),
            (0x0066_f110, vec![]),
        ];
        assert_eq!(other_calls, expected);

        // State.
        assert_eq!(e.mem.u32(tes + 0x88), 0);
        assert!(!e.get(this, TESDataHandler::bClearingData));
        for shader in [
            TESDataHandler::pSpotterShader,
            TESDataHandler::pItemDetectedShader,
            TESDataHandler::pCateyeMobileShader,
        ] {
            assert!(e.get(this, shader).is_null());
        }
        for word in CLEARED_WORDS {
            assert_eq!(e.global::<u32>(word), 0, "{word:08x}");
        }
        for flag in [FLAG_011CB96C, FLAG_011CA53C, FLAG_011CA830] {
            assert_eq!(e.global::<u32>(flag), 0, "{flag:08x}");
        }
        // Not touched.
        assert_eq!(e.global::<u32>(0x011c_a24c), 7);
        assert_eq!(e.global::<u32>(0x011c_a264), 7);
    }

    #[test]
    fn clear_data_picks_the_idle_manager_reset_and_the_navmesh_map() {
        let ClearSetup { mut e, this, .. } = clear_setup();
        e.register(ARRAY_SIZE, |_, _| ret(0));
        // A navmesh info map is set: it is cleared with `SetNavMeshInfoMap(0)`.
        e.register(0x0045_af00, |_, _| ret(0x9));
        // An object that answers yes: the other idle-manager reset.
        e.set_global(OBJECT_011DEA0C, 0x0b0bu32);
        e.register(0x004f_6de0, |_, _| ret(1));
        start_log(&mut e);
        e.call(0x0045_dfe0, &args![this]);
        let tes = e.global::<u32>(TES_SINGLETON);
        assert_eq!(calls(&e, 0x0045_afb0), vec![vec![tes, 0]]);
        assert_eq!(calls(&e, 0x004f_6de0), vec![vec![0x0b0b]]);
        assert_eq!(calls(&e, 0x0060_0030), vec![vec![0x1d1e]]);
        assert!(calls(&e, 0x005f_fd20).is_empty());

        // The object answers no: the first reset.
        e.register(0x004f_6de0, |_, _| ret(0));
        start_log(&mut e);
        e.call(0x0045_dfe0, &args![this]);
        assert_eq!(calls(&e, 0x005f_fd20), vec![vec![0x1d1e]]);
        assert!(calls(&e, 0x0060_0030).is_empty());
    }

    /// Doubles for the callees of the leak report, with the forms in
    /// `entries` (key, form) in `TESForm::pAllForms`.
    fn install_all_forms(e: &mut Engine, entries: Vec<(u32, u32)>) {
        e.set_global(ALL_FORMS_MAP, 0xa11u32);
        e.register(MAP_FIRST_POSITION, |_, _| ret(1));
        e.register_double(MAP_GET_NEXT, move |e, a| {
            let position = e.mem.u32(a[1]) as usize;
            let (key, form) = entries[position - 1];
            e.mem.set_u32(a[2], key);
            e.mem.set_u32(a[3], form);
            e.mem.set_u32(
                a[1],
                if position < entries.len() {
                    position as u32 + 1
                } else {
                    0
                },
            );
            Ret::default()
        });
        // The form type is the byte at +4, its id the word at +0xC, its
        // last file the word at +0x24.
        e.register(FORM_GET_TYPE, |e, a| ret(e.mem.u8(a[0] + 4) as u32));
        e.register(FORM_GET_ID, |e, a| ret(e.mem.u32(a[0] + 0xc)));
        e.register(FORM_GET_FILE, |e, a| {
            assert_eq!(a[1] as i32, -1);
            ret(e.mem.u32(a[0] + 0x24))
        });
        e.register(FILE_NAME, |_, a| ret(a[0] + 0x20));
        e.register(IDENTITY, |_, a| ret(a[0]));
        e.register(FORM_GET_TYPE_NAME, |_, _| ret(0x7e57));
        e.register(0x7000_0130, |_, _| ret(0xed17));
    }

    /// A form of `form_type` with `id`, last in `file`.
    fn leak_form(e: &mut Engine, form_type: u8, id: u32, file: u32) -> u32 {
        let form = object_with(
            e,
            0x7100_3000,
            &[
                (FORM_VTABLE_SLOT_128, 0x7000_0128),
                (FORM_VTABLE_SLOT_130, 0x7000_0130),
            ],
        );
        e.mem.set_u8(form + 4, form_type);
        e.mem.set_u32(form + 0xc, id);
        e.mem.set_u32(form + 0x24, file);
        form
    }

    #[test]
    fn clear_data_reports_the_forms_that_were_not_freed() {
        let ClearSetup { mut e, this, .. } = clear_setup();
        e.register(ARRAY_SIZE, |_, _| ret(0));
        let file = e.mem.alloc(0x40);
        let game_setting = leak_form(&mut e, 3, 0x1111, 0);
        let leaked = leak_form(&mut e, 5, 0x2222, file);
        let leaked_without_file = leak_form(&mut e, 6, 0x3333, 0);
        install_all_forms(
            &mut e,
            vec![
                (0x100, game_setting),
                (0x101, leaked),
                (0x102, 0),
                (0x103, leaked_without_file),
            ],
        );
        start_log(&mut e);

        e.call(0x0045_dfe0, &args![this]);

        // Warnings are switched off around the walk.
        assert_eq!(calls(&e, DISABLE_WARNING_COUNT), vec![vec![1], vec![0]]);
        // The type 3 form is released, not reported.
        assert_eq!(calls(&e, FORM_LIST_CLEAR), vec![vec![game_setting + 0x10]]);
        assert_eq!(calls(&e, 0x7000_0128), vec![vec![game_setting, 0, 1]]);
        // The others are reported and dropped from the map.
        assert_eq!(
            calls(&e, LOG_MESSAGE),
            vec![
                vec![FORM_LEAKED_FORMAT, 0xed17, 0x2222, 0x7e57, file + 0x20],
                vec![
                    FORM_LEAKED_FORMAT,
                    0xed17,
                    0x3333,
                    0x7e57,
                    UNKNOWN_FILE_NAME
                ],
                vec![FORMS_LEAKED_MESSAGE],
            ]
        );
        assert_eq!(
            calls(&e, MAP_SET_AT),
            vec![vec![0xa11, 0x101, 0], vec![0xa11, 0x103, 0]]
        );
    }

    #[test]
    fn clear_data_is_quiet_when_only_game_settings_remain() {
        let ClearSetup { mut e, this, .. } = clear_setup();
        e.register(ARRAY_SIZE, |_, _| ret(0));
        let game_setting = leak_form(&mut e, 3, 0x1111, 0);
        install_all_forms(&mut e, vec![(0x100, game_setting)]);
        start_log(&mut e);
        e.call(0x0045_dfe0, &args![this]);
        assert!(calls(&e, LOG_MESSAGE).is_empty());
        assert!(calls(&e, MAP_SET_AT).is_empty());
        assert_eq!(calls(&e, DISABLE_WARNING_COUNT), vec![vec![1], vec![0]]);
    }

    #[test]
    fn clear_data_forgets_the_files_and_destroys_the_active_file() {
        let ClearSetup { mut e, this, .. } = clear_setup();
        e.register(ARRAY_SIZE, |_, _| ret(0));
        let index = this.addr() + 0x21c;
        let prepare = |e: &mut Engine| {
            e.set(this, TESDataHandler::iNumCompile, 3);
            for slot in 0..4 {
                e.mem.set_u32(index + 4 * slot, 9);
            }
            e.set(this, TESDataHandler::pActiveFile, Ptr::new(0xf11e));
            e.set(this, TESDataHandler::iNextID, 0x1234);
        };

        // A normal session: the active file is closed and destroyed.
        prepare(&mut e);
        start_log(&mut e);
        e.call(0x0045_dfe0, &args![this]);
        assert_eq!(e.get(this, TESDataHandler::iNumCompile), 0);
        for slot in 0..3 {
            assert_eq!(e.mem.u32(index + 4 * slot), 0);
        }
        assert_eq!(
            e.mem.u32(index + 12),
            9,
            "only iNumCompile slots are cleared"
        );
        assert_eq!(calls(&e, FILE_CLOSE_ACTIVE), vec![vec![0xf11e, 0]]);
        assert_eq!(calls(&e, FILE_DESTRUCT), vec![vec![0xf11e]]);
        assert_eq!(calls(&e, OPERATOR_DELETE), vec![vec![0xf11e]]);
        assert!(e.get(this, TESDataHandler::pActiveFile).is_null());
        assert_eq!(e.get(this, TESDataHandler::iNextID), 0x800);

        // A master save is not closed first.
        prepare(&mut e);
        e.set(this, TESDataHandler::bMasterSave, true);
        start_log(&mut e);
        e.call(0x0045_dfe0, &args![this]);
        assert!(calls(&e, FILE_CLOSE_ACTIVE).is_empty());
        assert_eq!(calls(&e, FILE_DESTRUCT), vec![vec![0xf11e]]);

        // While loading a save the active file is left alone.
        prepare(&mut e);
        e.set(this, TESDataHandler::bSaveLoadGame, true);
        start_log(&mut e);
        e.call(0x0045_dfe0, &args![this]);
        assert!(calls(&e, FILE_DESTRUCT).is_empty());
        assert_eq!(e.get(this, TESDataHandler::pActiveFile).addr(), 0xf11e);
        assert_eq!(e.get(this, TESDataHandler::iNextID), 0x1234);
    }

    // ---------------------------------------------------------------
    // AddFormToDataHandler.
    // ---------------------------------------------------------------

    /// (form type, offset of its list) for the types whose list accessor is
    /// in this unit.
    const LOCAL_LIST_TYPES: [(u32, u32); 17] = [
        (0x05, 0x198),
        (0x0e, 0x0d8),
        (0x31, 0x188),
        (0x33, 0x140),
        (0x41, 0x010),
        (0x4a, 0x120),
        (0x4d, 0x1a0),
        (0x52, 0x170),
        (0x55, 0x190),
        (0x5a, 0x150),
        (0x5d, 0x0f0),
        (0x5e, 0x0f8),
        (0x62, 0x1a8),
        (0x66, 0x1b8),
        (0x6f, 0x1c8),
        (0x70, 0x1d0),
        (0x78, 0x168),
    ];

    /// (form type, accessor address) for the types whose list accessor is in
    /// another unit.
    const EXTERNAL_LIST_TYPES: [(u32, u32); 39] = [
        (0x06, 0x00461190),
        (0x07, 0x006130e0),
        (0x08, 0x00871a30),
        (0x09, 0x00624700),
        (0x0a, 0x00613790),
        (0x0b, 0x0043c490),
        (0x0c, 0x004ea950),
        (0x0d, 0x0045c650),
        (0x11, 0x006377e0),
        (0x12, 0x004610f0),
        (0x13, 0x0041d8a0),
        (0x14, 0x0087eaa0),
        (0x35, 0x00436aa0),
        (0x36, 0x00500940),
        (0x47, 0x00455600),
        (0x49, 0x00413f40),
        (0x4b, 0x00461070),
        (0x4e, 0x0045a730),
        (0x4f, 0x00874670),
        (0x51, 0x00460ff0),
        (0x53, 0x00891170),
        (0x54, 0x004610d0),
        (0x56, 0x00460fb0),
        (0x57, 0x0045a330),
        (0x5b, 0x00461110),
        (0x5f, 0x004a0d10),
        (0x61, 0x00461130),
        (0x63, 0x009d9f40),
        (0x65, 0x00461270),
        (0x68, 0x009c1a50),
        (0x6a, 0x004077e0),
        (0x6b, 0x00503650),
        (0x6d, 0x00984250),
        (0x6e, 0x00461290),
        (0x71, 0x00461150),
        (0x72, 0x00984230),
        (0x75, 0x00514f30),
        (0x76, 0x00506390),
        (0x77, 0x0062d2f0),
    ];

    /// What the list append recorded: the list and the item at the address
    /// it was given.
    type Appended = Rc<RefCell<Vec<(u32, u32)>>>;

    /// An engine with the form type reader and the list append stand-ins.
    fn add_engine() -> (Engine, Ptr<TESDataHandler>, Appended) {
        let (mut e, this) = engine();
        e.register(FORM_GET_TYPE, |e, a| ret(e.mem.u8(a[0] + 4) as u32));
        let appended: Appended = Rc::default();
        let shared = appended.clone();
        e.register_double(LIST_ADD, move |e, a| {
            shared.borrow_mut().push((a[0], e.mem.u32(a[1])));
            Ret::default()
        });
        (e, this, appended)
    }

    /// A form of `form_type` (a zeroed block with the type byte at +4).
    fn form_of_type(e: &mut Engine, form_type: u32) -> Ptr {
        let form = Ptr::<()>::new(e.mem.alloc(0x100));
        e.mem.set_u8(form.addr() + 4, form_type as u8);
        form
    }

    #[test]
    fn add_form_ignores_a_null_form() {
        let (mut e, this, appended) = add_engine();
        start_log(&mut e);
        assert!(!e.call(0x0046_03b0, &args![this, 0u32]).bool());
        assert!(e.call_log.as_ref().unwrap().len() == 1);
        assert!(appended.borrow().is_empty());
    }

    #[test]
    fn add_form_appends_to_the_list_of_its_type() {
        let (mut e, this, appended) = add_engine();
        for (form_type, offset) in LOCAL_LIST_TYPES {
            let form = form_of_type(&mut e, form_type);
            assert!(e.call(0x0046_03b0, &args![this, form]).bool());
            assert_eq!(
                appended.borrow().last().copied(),
                Some((this.addr() + offset, form.addr())),
                "type {form_type:#x}"
            );
        }
        for (form_type, accessor) in EXTERNAL_LIST_TYPES {
            let list = 0x5000_0000 + form_type * 8;
            e.register_double(accessor, move |_, a| {
                assert_ne!(a[0], 0);
                ret(list)
            });
            let form = form_of_type(&mut e, form_type);
            assert!(e.call(0x0046_03b0, &args![this, form]).bool());
            assert_eq!(
                appended.borrow().last().copied(),
                Some((list, form.addr())),
                "type {form_type:#x}"
            );
        }
        assert_eq!(appended.borrow().len(), 17 + 39);
    }

    #[test]
    fn add_form_of_type_0x37_uses_the_list_4_bytes_into_its_accessor_result() {
        let (mut e, this, appended) = add_engine();
        e.register(0x0041_69d0, |_, a| ret(a[0] + 0x300));
        let form = form_of_type(&mut e, 0x37);
        assert!(e.call(0x0046_03b0, &args![this, form]).bool());
        assert_eq!(
            appended.borrow().as_slice(),
            &[(this.addr() + 0x304, form.addr())]
        );
    }

    #[test]
    fn add_form_of_type_0x39_goes_to_the_interior_cells_only_when_flagged() {
        let (mut e, this, _) = add_engine();
        let cells = this.addr() + 0x1dc;
        e.register(ARRAY_SIZE, |_, _| ret(3));
        e.register(0x0042_5fd0, |e, a| ret(e.mem.u8(a[0] + 0x24) as u32 & 1));
        let added: Appended = Rc::default();
        let shared = added.clone();
        e.register_double(ARRAY_SET_AT_GROW, move |e, a| {
            shared.borrow_mut().push((a[0], a[1]));
            shared.borrow_mut().push((a[0], e.mem.u32(a[2])));
            Ret::default()
        });
        let form = form_of_type(&mut e, 0x39);
        assert!(e.call(0x0046_03b0, &args![this, form]).bool());
        assert!(added.borrow().is_empty());
        e.mem.set_u8(form.addr() + 0x24, 1);
        assert!(e.call(0x0046_03b0, &args![this, form]).bool());
        assert_eq!(
            added.borrow().as_slice(),
            &[(cells, 3), (cells, form.addr())]
        );
    }

    #[test]
    fn add_form_of_type_0x10_hands_effect_settings_to_their_registry() {
        let (mut e, this, appended) = add_engine();
        let cast_result = Rc::new(RefCell::new(0u32));
        let shared = cast_result.clone();
        e.register_double(DYNAMIC_CAST, move |_, _| ret(*shared.borrow()));
        do_nothing(&mut e, &[0x0040_9060]);
        let form = form_of_type(&mut e, 0x10);
        start_log(&mut e);
        // Not an `EffectSetting`: nothing more happens.
        assert!(e.call(0x0046_03b0, &args![this, form]).bool());
        assert!(calls(&e, 0x0040_9060).is_empty());
        *cast_result.borrow_mut() = 0xe5;
        assert!(e.call(0x0046_03b0, &args![this, form]).bool());
        assert_eq!(calls(&e, 0x0040_9060), vec![vec![0xe5]]);
        assert_eq!(
            calls(&e, DYNAMIC_CAST).last().unwrap(),
            &vec![
                form.addr(),
                0,
                FORM_TYPE_DESCRIPTOR,
                EFFECT_SETTING_TYPE_DESCRIPTOR,
                0
            ]
        );
        assert!(appended.borrow().is_empty());
    }

    /// What the reference cases see and do.
    struct ReferenceWorld {
        cast_result: Rc<RefCell<u32>>,
        parent_cell: Rc<RefCell<u32>>,
        preferred_cell: Rc<RefCell<u32>>,
        found_cell: Rc<RefCell<u32>>,
        must_persist: Rc<RefCell<bool>>,
        save_flag: Rc<RefCell<bool>>,
        reference: u32,
    }

    /// Doubles for everything the reference cases call. Cells are blocks
    /// whose byte at +0x24 is the interior flag.
    fn reference_world(e: &mut Engine) -> ReferenceWorld {
        let mut world = ReferenceWorld {
            cast_result: Rc::default(),
            parent_cell: Rc::default(),
            preferred_cell: Rc::default(),
            found_cell: Rc::default(),
            must_persist: Rc::default(),
            save_flag: Rc::default(),
            reference: 0,
        };
        let position = e.mem.alloc(12);
        e.mem.set_f32(position, 1.5);
        e.mem.set_f32(position + 4, -2.5);
        e.register(0x0042_5fd0, |e, a| ret(e.mem.u8(a[0] + 0x24) as u32 & 1));
        let shared = world.cast_result.clone();
        e.register_double(DYNAMIC_CAST, move |_, _| ret(*shared.borrow()));
        let shared = world.parent_cell.clone();
        e.register_double(0x008d_6f30, move |_, _| ret(*shared.borrow()));
        let shared = world.preferred_cell.clone();
        e.register_double(0x005f_36f0, move |_, _| ret(*shared.borrow()));
        e.register(0x004f_d3e0, |_, a| {
            assert_eq!(a[1], 1);
            ret(0x77)
        });
        let shared = world.found_cell.clone();
        e.register_double(0x0046_1bc0, move |_, _| ret(*shared.borrow()));
        let shared = world.must_persist.clone();
        e.register_double(0x0056_5260, move |_, _| ret(*shared.borrow() as u32));
        let shared = world.save_flag.clone();
        e.register_double(0x0056_4e00, move |_, _| ret(*shared.borrow() as u32));
        do_nothing(e, &[0x0054_8230, 0x0056_5480, 0x0056_4eb0]);
        world.reference = object_with(e, 0x7100_4000, &[(0x1f4, 0x7000_01f4)]);
        e.register_double(0x7000_01f4, move |_, _| ret(position));
        world
    }

    fn cell(e: &mut Engine, interior: bool) -> u32 {
        let cell = e.mem.alloc(0x40);
        e.mem.set_u8(cell + 0x24, interior as u8);
        cell
    }

    #[test]
    fn add_reference_to_the_cell_it_is_in() {
        let (mut e, this, _) = add_engine();
        let world = reference_world(&mut e);
        let form = form_of_type(&mut e, 0x3a);
        *world.cast_result.borrow_mut() = world.reference;
        let parent = cell(&mut e, true);
        *world.parent_cell.borrow_mut() = parent;
        start_log(&mut e);
        assert!(e.call(0x0046_03b0, &args![this, form]).bool());
        assert_eq!(
            calls(&e, DYNAMIC_CAST),
            vec![vec![
                form.addr(),
                0,
                FORM_TYPE_DESCRIPTOR,
                REFERENCE_TYPE_DESCRIPTOR,
                0
            ]]
        );
        assert_eq!(
            calls(&e, 0x0054_8230),
            vec![vec![parent, world.reference, 0]]
        );
        // Interior parent cell: no lookup by position.
        assert!(calls(&e, 0x0046_1bc0).is_empty());
        assert!(calls(&e, 0x005f_36f0).is_empty());
        // Neither flag set: persistence untouched.
        assert!(calls(&e, 0x0056_5480).is_empty());
        assert!(calls(&e, 0x0056_4eb0).is_empty());
    }

    #[test]
    fn add_reference_to_an_exterior_cell_looks_the_cell_up_by_position() {
        let (mut e, this, _) = add_engine();
        let world = reference_world(&mut e);
        // Every form type of the group.
        for form_type in [0x3a, 0x3d, 0x3e, 0x3f, 0x40, 0x69] {
            let form = form_of_type(&mut e, form_type);
            *world.cast_result.borrow_mut() = world.reference;
            let parent = cell(&mut e, false);
            *world.parent_cell.borrow_mut() = parent;
            let found = cell(&mut e, false);
            *world.found_cell.borrow_mut() = found;
            start_log(&mut e);
            assert!(e.call(0x0046_03b0, &args![this, form]).bool());
            // `GetCellFromWorldCoord(x, y, world space)`, x and y being the
            // first two floats of the position.
            assert_eq!(
                calls(&e, 0x0046_1bc0),
                vec![vec![
                    this.addr(),
                    1.5f32.to_bits(),
                    (-2.5f32).to_bits(),
                    0x77
                ]]
            );
            assert_eq!(
                calls(&e, 0x0054_8230),
                vec![vec![found, world.reference, 0]]
            );
        }
    }

    #[test]
    fn add_reference_without_a_parent_cell_tries_the_preferred_cell_then_the_position() {
        let (mut e, this, _) = add_engine();
        let world = reference_world(&mut e);
        let form = form_of_type(&mut e, 0x3a);
        *world.cast_result.borrow_mut() = world.reference;
        let preferred = cell(&mut e, true);
        *world.preferred_cell.borrow_mut() = preferred;
        start_log(&mut e);
        assert!(e.call(0x0046_03b0, &args![this, form]).bool());
        assert_eq!(
            calls(&e, 0x0054_8230),
            vec![vec![preferred, world.reference, 0]]
        );
        assert!(calls(&e, 0x0046_1bc0).is_empty());

        // No preferred cell either: the lookup by position decides.
        *world.preferred_cell.borrow_mut() = 0;
        *world.found_cell.borrow_mut() = 0;
        start_log(&mut e);
        assert!(e.call(0x0046_03b0, &args![this, form]).bool());
        assert_eq!(calls(&e, 0x0046_1bc0).len(), 1);
        // No cell at all: the reference is not added and its persistence
        // is left alone.
        assert!(calls(&e, 0x0054_8230).is_empty());
        assert!(calls(&e, 0x0056_5480).is_empty());
        assert!(calls(&e, 0x0056_4eb0).is_empty());
    }

    #[test]
    fn add_reference_updates_the_persistence_flags() {
        let (mut e, this, _) = add_engine();
        let world = reference_world(&mut e);
        let form = form_of_type(&mut e, 0x3a);
        *world.cast_result.borrow_mut() = world.reference;
        let parent = cell(&mut e, true);
        *world.parent_cell.borrow_mut() = parent;
        *world.must_persist.borrow_mut() = true;
        *world.save_flag.borrow_mut() = true;
        start_log(&mut e);
        assert!(e.call(0x0046_03b0, &args![this, form]).bool());
        assert_eq!(calls(&e, 0x0056_5480), vec![vec![world.reference, 1]]);
        assert_eq!(calls(&e, 0x0056_4eb0), vec![vec![world.reference, 1]]);
    }

    #[test]
    fn add_reference_that_is_not_a_reference_does_nothing() {
        let (mut e, this, _) = add_engine();
        reference_world(&mut e);
        let form = form_of_type(&mut e, 0x40);
        start_log(&mut e);
        assert!(e.call(0x0046_03b0, &args![this, form]).bool());
        assert_eq!(calls(&e, DYNAMIC_CAST).len(), 1);
        assert!(calls(&e, 0x008d_6f30).is_empty());
    }

    #[test]
    fn add_form_of_other_types_goes_to_the_object_list() {
        let (mut e, this, _) = add_engine();
        let cast_result = Rc::new(RefCell::new(0xb0u32));
        let shared = cast_result.clone();
        e.register_double(DYNAMIC_CAST, move |_, _| ret(*shared.borrow()));
        e.register(0x0046_12b0, |_, a| ret(0x4a3e_0000 + a[0]));
        do_nothing(&mut e, &[OBJECT_LIST_ADD, LOG_MESSAGE]);
        e.set(this, TESDataHandler::pObjectList, Ptr::new(0x0b1));
        // 0x03 has no case of its own.
        let form = form_of_type(&mut e, 0x03);
        start_log(&mut e);
        assert!(e.call(0x0046_03b0, &args![this, form]).bool());
        assert_eq!(calls(&e, OBJECT_LIST_ADD), vec![vec![0x0b1, 0xb0]]);
        assert_eq!(
            calls(&e, DYNAMIC_CAST),
            vec![vec![
                form.addr(),
                0,
                FORM_TYPE_DESCRIPTOR,
                OBJECT_TYPE_DESCRIPTOR,
                0
            ]]
        );
        // Not an object: logged and refused.
        *cast_result.borrow_mut() = 0;
        start_log(&mut e);
        assert!(!e.call(0x0046_03b0, &args![this, form]).bool());
        assert!(calls(&e, OBJECT_LIST_ADD).is_empty());
        assert_eq!(
            calls(&e, LOG_MESSAGE),
            vec![vec![UNKNOWN_FORM_TYPE_FORMAT, 0x4a3e_0003]]
        );
        // Types above the table end take the same path.
        let form = form_of_type(&mut e, 0xf0);
        assert!(!e.call(0x0046_03b0, &args![this, form]).bool());
    }

    #[test]
    fn add_form_of_type_0x58_adds_the_addon_node_first() {
        let (mut e, this, _) = add_engine();
        e.register(DYNAMIC_CAST, |_, a| ret(a[0] + 0x1000));
        do_nothing(&mut e, &[0x0046_1820, OBJECT_LIST_ADD]);
        e.set(this, TESDataHandler::pObjectList, Ptr::new(0x0b1));
        let form = form_of_type(&mut e, 0x58);
        start_log(&mut e);
        assert!(e.call(0x0046_03b0, &args![this, form]).bool());
        let log = e.call_log.clone().unwrap();
        let addon = log
            .iter()
            .position(|(addr, _)| *addr == 0x0046_1820)
            .unwrap();
        let add = log
            .iter()
            .position(|(addr, _)| *addr == OBJECT_LIST_ADD)
            .unwrap();
        assert!(addon < add, "the add-on node comes first");
        assert_eq!(log[addon].1, vec![this.addr(), form.addr()]);
        assert_eq!(log[add].1, vec![0x0b1, form.addr() + 0x1000]);
    }
}
