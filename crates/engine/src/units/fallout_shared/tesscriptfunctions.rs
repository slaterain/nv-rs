//! `fallout shared/tesscriptfunctions.cpp` (Xbox PDB source unit), subsystem `fallout shared`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! The unit holds the bodies of the script commands (script and console
//! functions). Every body is `cdecl` and receives the same eight stack
//! words, [`ScriptArgs`]; most of them hand the words to
//! `Script::ParseParameters` (`005accb0`) to read their arguments into locals
//! passed by address ([`ScriptArgs::parse_into`]). The result is `AL`: false
//! when the arguments do not parse. Bodies that never read a word
//! (`005b5b90`, `005b5cc0`, ...) take no parameters here.
//!
//! Progress: the first 40 queue entries (`005b4b20` to `005b7000`) are
//! translated. The next session continues at `005b7070`
//! (`Script::ToggleScriptsFunction`).
//!
//! Notes on the exe's code that the translations rely on:
//! - Several tiny accessors are called by address and named here after what
//!   they do, not after the engine map (the linker folded identical code, so
//!   the map often carries another class's name: `007af430` is
//!   `*(this + 0x20)`, the base form of a reference, although the map calls
//!   it `BGSSaveFormBuffer::GetForm`).
//! - The console print `00703c00` is `printf`-like: format address first,
//!   `double` arguments take two words.
//! - Virtual slots are named by byte offset, exactly as the code indexes the
//!   vtable; the Xbox PDB name of a slot is only given where the PC
//!   behaviour matches it.
//! - The compiler's exception-unwinding frames are not translated.

#[allow(unused_imports)]
use crate::prelude::*;

// ---- The eight words of a script command body ----------------------------

/// The eight stack words of a script command body (`cdecl`, in the order the
/// caller pushes them: the first is the one at `[ebp+8]`). Field names follow
/// how the bodies use them: `this_obj` is the reference the command runs on,
/// `script_obj` the running script (its name is printed by `005b4be0`) and
/// `result` the `double` a function-style command writes its value to;
/// `param_info`, `script_data`, `containing_obj`, `event_list` and
/// `opcode_offset` are only passed on to `Script::ParseParameters` (and
/// `containing_obj` is a second reference in `005b53d0` and `005b5860`).
#[derive(Clone, Copy, Debug)]
pub struct ScriptArgs {
    /// `[ebp+0x08]`.
    pub param_info: u32,
    /// `[ebp+0x0c]`.
    pub script_data: u32,
    /// `[ebp+0x10]`.
    pub this_obj: Ptr,
    /// `[ebp+0x14]`.
    pub containing_obj: Ptr,
    /// `[ebp+0x18]`.
    pub script_obj: Ptr,
    /// `[ebp+0x1c]`.
    pub event_list: u32,
    /// `[ebp+0x20]`.
    pub result: Ptr,
    /// `[ebp+0x24]`.
    pub opcode_offset: u32,
}

impl Arg for ScriptArgs {
    const WORDS: usize = 8;
    fn take(words: &[u32], i: &mut usize) -> Self {
        ScriptArgs {
            param_info: u32::take(words, i),
            script_data: u32::take(words, i),
            this_obj: Ptr::take(words, i),
            containing_obj: Ptr::take(words, i),
            script_obj: Ptr::take(words, i),
            event_list: u32::take(words, i),
            result: Ptr::take(words, i),
            opcode_offset: u32::take(words, i),
        }
    }
    fn put(self, out: &mut Vec<u32>) {
        self.param_info.put(out);
        self.script_data.put(out);
        self.this_obj.put(out);
        self.containing_obj.put(out);
        self.script_obj.put(out);
        self.event_list.put(out);
        self.result.put(out);
        self.opcode_offset.put(out);
    }
}

impl ScriptArgs {
    /// `Script::ParseParameters` (`005accb0`) with the given output addresses
    /// after the seven fixed words: its `AL`.
    fn parse(self, e: &mut Engine, outs: &[u32]) -> bool {
        let mut words = args![
            self.param_info,
            self.script_data,
            self.opcode_offset,
            self.this_obj,
            self.containing_obj,
            self.script_obj,
            self.event_list
        ];
        words.extend_from_slice(outs);
        e.call(PARSE_PARAMETERS, &words).bool()
    }

    /// [`ScriptArgs::parse`] with `N` word-sized locals (the stack slots the
    /// game passes by address) initialised to `init`. `None` when the
    /// parameters do not parse, otherwise the values left in the locals.
    fn parse_into<const N: usize>(self, e: &mut Engine, init: [u32; N]) -> Option<[u32; N]> {
        let block = e.mem.alloc(4 * N as u32);
        let mut outs = [0u32; N];
        for (i, value) in init.iter().enumerate() {
            outs[i] = block + 4 * i as u32;
            e.mem.set_u32(outs[i], *value);
        }
        let ok = self.parse(e, &outs);
        let mut values = init;
        for (i, value) in values.iter_mut().enumerate() {
            *value = e.mem.u32(outs[i]);
        }
        e.mem.free(block);
        ok.then_some(values)
    }
}

// ---- Callees outside the unit (by exe address) -----------------------------

/// `Script::ParseParameters` (Xbox PDB), `cdecl`, varargs.
const PARSE_PARAMETERS: u32 = 0x005a_ccb0;
/// The console print: format address, then the arguments (`cdecl`).
const CONSOLE_PRINT: u32 = 0x0070_3c00;
/// `Script::GetDistanceConditionFunction` (Xbox PDB), `cdecl`
/// (`thisObj, object, 0, result`).
const GET_DISTANCE_CONDITION: u32 = 0x0059_bfa0;
/// `Script::GetInZoneConditionFunction` (Xbox PDB), `cdecl`, same arguments.
const GET_IN_ZONE_CONDITION: u32 = 0x0059_c010;
/// `Script::GetItemCountConditionFunction` (Xbox PDB), `cdecl`
/// (`actor, item, 0, double* result`).
const GET_ITEM_COUNT_CONDITION: u32 = 0x0059_d8e0;
/// `__RTDynamicCast` (`object, 0, source type, target type, 0`).
const DYNAMIC_CAST: u32 = 0x00ec_43fb;
/// RTTI type descriptor of `TESObjectREFR` (`.?AVTESObjectREFR@@`).
const RTTI_TES_OBJECT_REFR: u32 = 0x0118_41cc;
/// RTTI type descriptor of `Actor` (`.?AVActor@@`).
const RTTI_ACTOR: u32 = 0x0118_46d4;
/// `_ftol2_sse`: the `double` in `ST0` (a leading `f64` argument) truncated
/// into `EAX`.
const FTOL: u32 = 0x00ec_62c0;
/// `_memset` (`dest, value, count`).
const MEMSET: u32 = 0x00ec_61c0;
/// `_strlen`.
const STRLEN: u32 = 0x00ec_6130;
/// `strcpy_s` (`dest, size, source`).
const STRCPY_S: u32 = 0x0040_6d30;
/// `strcat_s` (`dest, size, source`).
const STRCAT_S: u32 = 0x0040_6d50;
/// `sprintf_s` (`dest, size, format, ...`).
const SPRINTF_S: u32 = 0x0040_6d00;
/// `strncpy`-like copy that terminates (`dest, source, count`).
const STRNCPY_S: u32 = 0x004a_dd50;
/// `strstr`-like search (`haystack, needle`): non-zero when found.
const FIND_SUBSTRING: u32 = 0x0048_12f0;
/// `eh_vector_constructor_iterator` (`array, size, count, constructor,
/// destructor`).
const VECTOR_CONSTRUCT: u32 = 0x00ec_782f;
/// `eh_vector_destructor_iterator` (`array, size, count, destructor`).
const VECTOR_DESTRUCT: u32 = 0x00ec_5fce;

/// Form type byte (`this + 4`, `movzx`).
const FORM_TYPE: u32 = 0x0040_1170;
/// `*(this + 0x20)`: the base form of a reference.
const GET_BASE_FORM: u32 = 0x007a_f430;
/// `this + 0x44`: the extra data list of a reference.
const EXTRA_DATA_LIST: u32 = 0x005d_43c0;
/// `ExtraDataList::GetContainerChanges` (Xbox PDB).
const GET_CONTAINER_CHANGES: u32 = 0x0041_8520;
/// `InventoryChanges::GetInventoryChanges` (Xbox PDB), `cdecl` (`reference`).
const GET_INVENTORY_CHANGES: u32 = 0x004b_f220;
/// `InventoryChanges::WearingObject` (Xbox PDB), `thiscall` (`item, 0`).
const WEARING_OBJECT: u32 = 0x004b_fda0;
/// `ExtraDataList::SetCanNotWear` (Xbox PDB), `thiscall` (`0`).
const SET_CAN_NOT_WEAR: u32 = 0x0041_ab70;
/// `InventoryChanges::DuplicateAllItems` (Xbox PDB), `thiscall`
/// (`reference, target`).
const DUPLICATE_ALL_ITEMS: u32 = 0x004c_d9c0;
/// `thiscall` on the inventory changes (`reference, item, 0, flag, 0, flag,
/// slot, extra`); returns a `float` in `ST0` that the caller discards.
const FN_004CE340: u32 = 0x004c_e340;
/// Same shape as [`FN_004CE340`] (`reference, item, 0, flag, 0, flag, -1,
/// 0`).
const FN_004CE380: u32 = 0x004c_e380;
/// `thiscall` on the inventory changes (`form`): the entry for the form.
const FN_004CBA70: u32 = 0x004c_ba70;
/// `thiscall` on an actor (`form`): whether the actor has the form.
const FN_00575400: u32 = 0x0057_5400;
/// `thiscall` on an extra data list (`form`).
const FN_00419700: u32 = 0x0041_9700;
/// `cdecl` (`reference, form, 0, result`).
const FN_005A2E20: u32 = 0x005a_2e20;
/// `PlayerCharacter::CheckForQuestTargetUpdate` (Xbox PDB), `thiscall` on
/// the player (`reference`).
const CHECK_QUEST_TARGET_UPDATE: u32 = 0x0095_2c30;
/// `Interface::IsInMenuMode` (Xbox PDB).
const IS_IN_MENU_MODE: u32 = 0x0070_2360;
/// Interface function the item commands call after the player's inventory
/// changed (`cdecl`, no arguments).
const FN_00704AF0: u32 = 0x0070_4af0;
/// `thiscall` on the base form of a reference (`reference, extra, 1`).
const FN_00606540: u32 = 0x0060_6540;
/// `TESContainer::TESContainer` (Xbox PDB): a 0x10-byte object.
const TES_CONTAINER_CONSTRUCT: u32 = 0x0048_1610;
/// Destructor of the 0x10-byte container object.
const TES_CONTAINER_DESTRUCT: u32 = 0x0048_1680;
/// `thiscall` on `leveled list + 0x30` (`level, count, container, 0`).
const FN_00487F70: u32 = 0x0048_7f70;
/// `thiscall` on the container object (`item, count, 0`).
const FN_004818E0: u32 = 0x0048_18e0;
/// `thiscall` on the container object (`float`).
const FN_00482090: u32 = 0x0048_2090;
/// `thiscall` on the container object (`reference, flag`).
const FN_004821A0: u32 = 0x0048_21a0;
/// `TESObjectREFR::GetCalcLevel` (Xbox PDB), `thiscall` (`0`).
const GET_CALC_LEVEL: u32 = 0x0056_7e10;
/// Signed byte at `this + 0xf4`.
const FN_00446390: u32 = 0x0044_6390;
/// `TESObjectREFR::GetActionRef` (Xbox PDB).
const GET_ACTION_REF: u32 = 0x0057_2e30;
/// `thiscall` (`action`): whether the reference has the action set.
const HAS_ACTION: u32 = 0x0057_2d30;
/// `thiscall` (`action`): sets the action.
const SET_ACTION: u32 = 0x0057_2d50;
/// `TESObjectREFR::ClearAction` (Xbox PDB), `thiscall` (`action`).
const CLEAR_ACTION: u32 = 0x0057_2db0;
/// `TESObjectREFR::Activate` (Xbox PDB), `thiscall` (`activator, 0, 0, 1`).
const ACTIVATE: u32 = 0x0057_3170;
/// Tests form flag `0x800` (`this + 8`).
const FN_00440DA0: u32 = 0x0044_0da0;
/// `thiscall` on the item list element: the actor's `+0x94` member
/// (`item, 1`).
const FN_008248E0: u32 = 0x0082_48e0;
/// `MiddleHighProcess::GetSavedAcquireObject` (Xbox PDB), `thiscall`.
const MIDDLE_HIGH_PROCESS_SAVED_ACQUIRE: u32 = 0x008d_8520;
/// Interface message with icon (`cdecl`: `text, 0, icon path, sound, float,
/// 0`).
const SHOW_MESSAGE: u32 = 0x0070_52f0;
/// `Actor::GetPickUpSoundName` (Xbox PDB), `thiscall` on the player
/// (`item, 0, 0`).
const GET_PICK_UP_SOUND_NAME: u32 = 0x008a_dcf0;
/// `TESFullName::GetFullName` (Xbox PDB), `cdecl` (`form`).
const GET_FULL_NAME: u32 = 0x0048_2720;
/// `this + 0x10` of the data handler (`00460140`): the world space list.
const DATA_HANDLER_WORLD_SPACES: u32 = 0x0046_0140;
/// `this + 0x40` of the data handler (`0087eaa0`): the spell list.
const DATA_HANDLER_SPELLS: u32 = 0x0087_eaa0;
/// `TESWorldSpace::BuildMapMarkerList` (Xbox PDB), `thiscall` (`0`): returns a
/// freshly allocated list.
const BUILD_MAP_MARKER_LIST: u32 = 0x0058_82a0;
/// `TESObjectREFR::GetMapMarkerData` (Xbox PDB).
const GET_MAP_MARKER_DATA: u32 = 0x0056_9060;
/// `MapMarkerData::GetHidden` (Xbox PDB).
const MAP_MARKER_GET_HIDDEN: u32 = 0x0043_8f10;
/// `MapMarkerData::GetTravelLoc` (Xbox PDB).
const MAP_MARKER_GET_TRAVEL_LOC: u32 = 0x0043_8ef0;
/// `MapMarkerData::SetVisible` (Xbox PDB), `thiscall` (`bool`).
const MAP_MARKER_SET_VISIBLE: u32 = 0x0044_de40;
/// `MapMarkerData::SetTravelLoc` (Xbox PDB), `thiscall` (`bool`).
const MAP_MARKER_SET_TRAVEL_LOC: u32 = 0x0044_de80;
/// `ProcessLists::PrintLists` (Xbox PDB), `thiscall` (`detection, -1`).
const PROCESS_LISTS_PRINT_LISTS: u32 = 0x008d_0600;
/// Empty `thiscall` on the process lists singleton.
const FN_00483710: u32 = 0x0048_3710;
/// `PlayerCharacter::SetGodMode` (Xbox PDB), `cdecl` (`bool`).
const SET_GOD_MODE: u32 = 0x0095_26a0;
/// `PlayerCharacter::IsGodMode` (Xbox PDB).
const IS_GOD_MODE: u32 = 0x0095_26b0;
/// `PlayerCharacter::SetDemigodMode` (Xbox PDB), `cdecl` (`bool`).
const SET_DEMIGOD_MODE: u32 = 0x0095_26e0;
/// `PlayerCharacter::IsDemigodMode` (Xbox PDB).
const IS_DEMIGOD_MODE: u32 = 0x0095_26f0;

// Linked list nodes: item at +0, next node at +4 (`BSSimpleList`).
/// `true` when the node has neither an item nor a next node.
const LIST_IS_EMPTY: u32 = 0x0082_56d0;
/// Returns `this`: the address of the node's item.
const LIST_ITEM_PTR: u32 = 0x0068_15c0;
/// `*(this + 4)`: the next node (the same code reads the reference count of
/// an `NiRefObject`, whose field is also at +4).
const LIST_NEXT: u32 = 0x0072_6070;
/// Pulls the next node's content into the head and frees that node.
const LIST_POP_FRONT: u32 = 0x0063_f7b0;
/// Number of nodes with a non-null item (`thiscall`).
const LIST_COUNT: u32 = 0x005a_e380;
/// Initialises an empty list head (a two-word object).
const LIST_CONSTRUCT: u32 = 0x0096_a2d0;
/// `thiscall` on the list head: appends the item whose *cell address* is
/// passed.
const LIST_APPEND: u32 = 0x005a_e3d0;
/// Destructor of the list head.
const LIST_DESTRUCT: u32 = 0x0046_ffb0;
/// Scalar deleting destructor of a heap list (`this, 1`).
const LIST_DELETE: u32 = 0x0047_02f0;
/// `this + 0x18`: the item list embedded in a form list.
const FORM_LIST_ITEMS: u32 = 0x0050_0940;

// Strings.
/// `BSStringT` constructor (an 8-byte object).
const BS_STRING_CONSTRUCT: u32 = 0x0040_37b0;
/// `BSStringT` destructor.
const BS_STRING_DESTRUCT: u32 = 0x0040_37d0;
/// `BSStringT` format (`string, format, ...`).
const BS_STRING_FORMAT: u32 = 0x0040_6f60;
/// `BSStringT<char>::operator=` / append (`string, text`).
const BS_STRING_APPEND: u32 = 0x0040_4820;
/// `*(this + 4)` of a string global: its text.
const BS_STRING_TEXT: u32 = 0x0040_3df0;
/// `NiPointer::operator T*` and `BSStringT::c_str`: `*this`.
const NI_POINTER_GET: u32 = 0x0055_9450;

// Memory statistics.
/// Constructs the 0x18-byte memory heap dump file object (`file name`).
const MEM_STATS_FILE_CONSTRUCT: u32 = 0x0087_83c0;
/// Its destructor.
const MEM_STATS_FILE_DESTRUCT: u32 = 0x0087_84d0;
/// `cdecl` (`dump object, flag`): returns the output object (global `011f6238`).
const MEM_STATS_OUTPUT_GET: u32 = 0x0040_1020;
/// `thiscall` on the output object (its `RET 8` pops the two words the
/// previous call left).
const MEM_STATS_OUTPUT_SET: u32 = 0x00aa_47c0;
/// `BSFile::BSFile` (Xbox PDB), `thiscall` (`path, 1, 0x4000, 0`).
const BS_FILE_CONSTRUCT: u32 = 0x00b0_0260;
/// `BSFile::Open` (Xbox PDB), `thiscall` (`0, 0`).
const BS_FILE_OPEN: u32 = 0x00af_f300;
/// `BSFile::Close` (Xbox PDB).
const BS_FILE_CLOSE: u32 = 0x00af_fd10;
/// `BSFile::~BSFile` (Xbox PDB).
const BS_FILE_DESTRUCT: u32 = 0x00af_f240;
/// `thiscall` on a `BSFile` (`buffer, size`): writes bytes.
const BS_FILE_WRITE: u32 = 0x00af_f950;
/// `ModelLoader::OutputModelMapContents` (Xbox PDB), `thiscall`
/// (`file, 0`).
const MODEL_LOADER_OUTPUT_MODEL_MAP: u32 = 0x0044_3270;
/// `BSFaceGenManager::GetModelCache` (Xbox PDB).
const GET_MODEL_CACHE: u32 = 0x0065_2110;
/// `BSFaceGenModelMap::OutputModelMapContents` (Xbox PDB), `thiscall`
/// (`file`).
const FACE_GEN_OUTPUT_MODEL_MAP: u32 = 0x0064_fe80;
/// `NiPointer` constructor passed to the vector iterator.
const NI_POINTER_CONSTRUCT: u32 = 0x0066_94e0;
/// `NiPointer` destructor passed to the vector iterator.
const NI_POINTER_DESTRUCT: u32 = 0x0045_cec0;
/// Zero-fills (`array, 0, bytes`).
const ZERO_FILL: u32 = 0x0040_3d30;
/// First texture of the global list.
const FIRST_TEXTURE: u32 = 0x0045_6510;
/// `this + 0x2c`: the next texture.
const NEXT_TEXTURE: u32 = 0x0055_b980;
/// `this + 0x24`: the texture's Direct3D texture.
const GET_D3D_TEXTURE: u32 = 0x0059_bb30;
/// `NiPointer::operator=` (`to`, `from`).
const NI_POINTER_ASSIGN: u32 = 0x006e_5cc0;
/// Assigns a raw pointer to an `NiPointer` (`slot, pointer`).
const NI_POINTER_SET: u32 = 0x0066_b0d0;
/// `cdecl` (`class, texture`): whether the texture is of the class.
const TEXTURE_IS_SOURCE: u32 = 0x0043_b300;
/// `thiscall`: the C string of a name handle.
const NAME_TEXT: u32 = 0x0043_b1b0;
/// `this + 8`.
const FN_00413F40: u32 = 0x0041_3f40;
/// `cdecl` (`text, size`): fixes the path in place.
const FIX_PATH: u32 = 0x004a_fb00;
/// `this + 0x14`.
const FN_007D6BB0: u32 = 0x007d_6bb0;
/// `cdecl` (`surface`): the Direct3D format id.
const D3D_FORMAT_ID: u32 = 0x00e7_bd20;
/// `NiXenonRenderer::GetD3DFormatString` (Xbox PDB), `cdecl`
/// (`format id`).
const GET_D3D_FORMAT_STRING: u32 = 0x00e7_a9a0;
/// `thiscall` (`texture`): 3 for the textures flagged "L".
const TEXTURE_KIND: u32 = 0x0043_c430;
/// `BGSTextureUseMap` constructor (a 0x14-byte object).
const TEXTURE_USE_MAP_CONSTRUCT: u32 = 0x004a_8f50;
/// `thiscall` (`node, 1`): collects the textures below a scene node.
const TEXTURE_USE_MAP_ADD_NODE: u32 = 0x004a_a3d0;
/// `thiscall` (`1`): collects the textures of the whole scene.
const TEXTURE_USE_MAP_ADD_SCENE: u32 = 0x004a_abb0;
/// `BGSTextureUseMap::WriteToFile` (Xbox PDB), `thiscall` (`path`).
const TEXTURE_USE_MAP_WRITE_TO_FILE: u32 = 0x004a_a900;
/// Sum of the sizes of the textures in the map.
const TEXTURE_USE_MAP_TOTAL_SIZE: u32 = 0x004a_a890;
/// `BGSTextureUseMap` destructor.
const TEXTURE_USE_MAP_DESTRUCT: u32 = 0x004a_a340;
/// Archive profiling (`BSSystem/archive.cpp`), compiled to `XOR AL,AL` in
/// this build.
const ARCHIVE_PROFILE_START: u32 = 0x00af_4390;

// Process and memory level singletons.
/// Singleton getter (loads the global `011f96a0`).
const FN_0044F560: u32 = 0x0044_f560;
/// Singleton getter (loads the global `011f35a0`).
const FN_004DE490: u32 = 0x004d_e490;
/// `this + 4` (`BaseProcess::GetActorPackageThatIsRunning` in the engine
/// map).
const PLUS_4: u32 = 0x0071_7e50;
/// `thiscall` (`package`).
const FN_005E01B0: u32 = 0x005e_01b0;
/// `cdecl` (`scope, 1, 1, 1`): initialises a 12-byte level scope.
const FN_00878160: u32 = 0x0087_8160;
/// `cdecl` (`scope`): tears the scope down.
const FN_00878200: u32 = 0x0087_8200;
/// `MemoryLevelManager::FreeReleasedObjects` (Xbox PDB), `cdecl`
/// (`level byte`).
const FREE_RELEASED_OBJECTS: u32 = 0x0087_8250;
/// `thiscall` on the singleton [`GLOBAL_0011DEA10`] (`1, 0`).
const FN_004539A0: u32 = 0x0045_39a0;
/// `thiscall` on the save/load singleton (`0, 0`).
const FN_00848E70: u32 = 0x0084_8e70;

// ---- Globals ---------------------------------------------------------------

/// The `PlayerCharacter` singleton pointer.
const PLAYER: u32 = 0x011d_ea3c;
/// Pointer to the data handler singleton.
const DATA_HANDLER: u32 = 0x011c_3f2c;
/// Singleton passed as `this` to `ProcessLists::PrintLists` and `00483710`.
const PROCESS_LISTS: u32 = 0x011e_0e80;
/// Pointer to the model loader (non-null once created).
const MODEL_LOADER: u32 = 0x011c_3b3c;
/// Texture class object passed to [`TEXTURE_IS_SOURCE`].
const TEXTURE_CLASS: u32 = 0x011f_444c;
/// Pointer to the singleton whose byte at `+1` `005b6cb0` sets.
const GLOBAL_0011DEA0C: u32 = 0x011d_ea0c;
/// Pointer to the singleton `005b6cd0` calls `004539a0` on.
const GLOBAL_0011DEA10: u32 = 0x011d_ea10;
/// Pointer to the save/load singleton.
const SAVE_LOAD_GAME: u32 = 0x011d_df38;
/// Byte set by `005b5d40`.
const FLAG_0011F122D: u32 = 0x011f_122d;
/// Byte set by `005b5da0`.
const FLAG_0011F9FC0: u32 = 0x011f_9fc0;
/// Byte toggled by `Script::ToggleOcclusion`.
const OCCLUSION_QUERY: u32 = 0x011f_9181;
/// Byte of `005b6f60` / `005b6f70` (verbose messages).
const VERBOSE_MESSAGES: u32 = 0x011f_158c;
/// `float` the item commands pass as the pick-up message volume.
const PICKUP_VOLUME: u32 = 0x0101_62c0;
/// `float` `005b4be0` passes to `00482090`.
const ADD_ITEM_FLOAT: u32 = 0x0102_31e0;
/// Byte at `+0x268` of the TLS block: commands echo to the console when set.
const TLS_ECHO: u32 = 0x268;
/// Dword at `+0x28c` of the TLS block: nesting depth of the Activate command.
const TLS_ACTIVATE_DEPTH: u32 = 0x28c;
/// An Activate nested deeper than this does nothing.
const ACTIVATE_DEPTH_LIMIT: u32 = 5;
/// The player's list of message parts: the texts pasted into the "item
/// removed" message (`BSStringT`-like globals whose text is at +4).
const MESSAGE_PART_A: u32 = 0x011d_3db4;
const MESSAGE_PART_B: u32 = 0x011d_3114;

/// Size of a `BSFile` object (`BSFile::BSFile`).
const BS_FILE_SIZE: u32 = 0x160;
/// Number of slots in `OutputMemStats`'s sorted texture table.
const TEXTURE_SLOTS: u32 = 0x800;

// ---- String literals (addresses in the exe's data) ---------------------------

/// `"SCRIPTS: AddItem in script '%s' failed to generate an item."`
const MSG_ADD_ITEM_FAILED: u32 = 0x0103_9358;
/// `"SCRIPTS: Never have the player character activate something in a script very Bad"`
const MSG_PLAYER_ACTIVATE: u32 = 0x0103_93b0;
/// `"IsActionRef >> %0.2f"`
const MSG_IS_ACTION_REF: u32 = 0x0103_9394;
/// `"Command removed. Use TDT and switch to COMBAT INFO page."`
const MSG_COMBAT_STATS_REMOVED: u32 = 0x0103_9404;
/// `"%d spells added to Player Character"`
const MSG_SPELLS_ADDED: u32 = 0x0103_9440;
/// `"Detection list printed"`
const MSG_DETECTION_LIST_PRINTED: u32 = 0x0103_9464;
/// `"Ai Lists Printed"`
const MSG_AI_LISTS_PRINTED: u32 = 0x0103_947c;
/// `"Deprecated, use TFC 2 instead"`
const MSG_FREEZE_RENDERER_DEPRECATED: u32 = 0x0103_9490;
/// `"Occlusion Query : %s"`
const MSG_OCCLUSION_QUERY: u32 = 0x0103_94b0;
/// `"off"`
const TEXT_OFF: u32 = 0x0103_94c8;
/// `"on"`
const TEXT_ON: u32 = 0x0101_22c8;
/// `"OutputMemContexts is no longer supported.  Use Sherlock or OutputMemStats for context info."`
const MSG_MEM_CONTEXTS_UNSUPPORTED: u32 = 0x0103_94d0;
/// `"Bye."`
const MSG_BYE: u32 = 0x0103_97a4;
/// `"shown."`
const TEXT_SHOWN: u32 = 0x0103_97c8;
/// `"hidden."`
const TEXT_HIDDEN: u32 = 0x0103_97c0;
/// `"Verbose messages %s"`
const MSG_VERBOSE: u32 = 0x0103_97d0;
/// `"All map markers %s"`
const MSG_MAP_MARKERS: u32 = 0x0103_97ac;
/// `"God Mode %s"`
const MSG_GOD_MODE: u32 = 0x0103_97e4;
/// `"Demigod Mode %s"`
const MSG_DEMIGOD_MODE: u32 = 0x0103_9820;
/// `"Demigod Mode disabled"`
const MSG_DEMIGOD_DISABLED: u32 = 0x0103_9808;
/// `"enabled."`
const TEXT_ENABLED: u32 = 0x0103_97fc;
/// `"disabled."`
const TEXT_DISABLED: u32 = 0x0103_97f0;
/// `".txt"`
const EXTENSION_TXT: u32 = 0x0103_9788;
/// `"ArchiveProfile.txt"`
const DEFAULT_ARCHIVE_PROFILE: u32 = 0x0103_9790;
/// `"Outputting Archive profile to file %s"`
const MSG_ARCHIVE_PROFILE_OUTPUT: u32 = 0x0103_9760;
/// `"Archive profiling is not enabled"`
const MSG_ARCHIVE_PROFILE_DISABLED: u32 = 0x0103_973c;
/// `"TextureUseMap.txt"`
const DEFAULT_TEXTURE_USE_MAP: u32 = 0x0103_9728;
/// `"FAILED to write to '%s'"`
const MSG_MAP_WRITE_FAILED: u32 = 0x0103_96f4;
/// `"map written to file '%s'"`
const MSG_MAP_WRITTEN: u32 = 0x0103_970c;
/// `"%d MB"`
const FORMAT_MB: u32 = 0x0103_96ec;
/// `"%d KB"`
const FORMAT_KB: u32 = 0x0103_96e4;
/// `"%d bytes"`
const FORMAT_BYTES: u32 = 0x0103_96d8;
/// `" textures in use."`
const TEXT_TEXTURES_IN_USE: u32 = 0x0103_96c4;
/// `"OutputMemStats.mhd"`
const DEFAULT_MEM_STATS: u32 = 0x0103_96b0;
/// `"MemStats%s.mhd"`
const FORMAT_MEM_STATS: u32 = 0x0103_96a0;
/// `"ModelDump%s.txt"`
const FORMAT_MODEL_DUMP: u32 = 0x0103_9690;
/// `"TextureDump%s.txt"`
const FORMAT_TEXTURE_DUMP: u32 = 0x0103_967c;
/// `"UNNAMED SOURCE TEXTURE"`
const TEXT_UNNAMED_SOURCE_TEXTURE: u32 = 0x0103_9664;
/// `"RENDERED_TEXTURE (%s)"`
const FORMAT_RENDERED_TEXTURE: u32 = 0x0103_964c;
/// `"RENDERED_TEXTURE (none)"`
const TEXT_RENDERED_TEXTURE_NONE: u32 = 0x0103_9634;
/// `"TEXTURES: %d:\t%s\t%s\t%dx%d\t%d\t%s\trefcount %d\t%s\t%s\r\n"`
const FORMAT_TEXTURE_LINE: u32 = 0x0103_952c;
/// `"L"`
const TEXT_L: u32 = 0x0103_9564;
/// `"H"`
const TEXT_H: u32 = 0x0103_9560;
/// `"%i %s%s %s"`
const FORMAT_ITEM_COUNT_MESSAGE: u32 = 0x0101_c178;
/// `"%s %s"`
const FORMAT_ITEM_MESSAGE: u32 = 0x0101_2058;
/// `"Interface\Icons\Message Icons\glow_message_vaultboy_neutral.dds"`
const ICON_VAULT_BOY: u32 = 0x0102_08e0;
// The path fragments `OutputMemStats` groups textures by.
/// `"\armor\"`
const PATH_ARMOR: u32 = 0x0103_9624;
/// `"\creatures\"`
const PATH_CREATURES: u32 = 0x0103_9618;
/// `"\characters\"`
const PATH_CHARACTERS: u32 = 0x0103_9608;
/// `"\pipboy3000\"`
const PATH_PIPBOY: u32 = 0x0103_95f8;
/// `"\weapons\"`
const PATH_WEAPONS: u32 = 0x0103_95ec;
/// `"\decals\"`
const PATH_DECALS: u32 = 0x0103_95e0;
/// `"\projectiles\"`
const PATH_PROJECTILES: u32 = 0x0103_95d0;
/// `"\fonts\"`
const PATH_FONTS: u32 = 0x0103_95c0;
/// `"\interface\"`
const PATH_INTERFACE: u32 = 0x0103_95b4;
/// `"\effects\"`
const PATH_EFFECTS: u32 = 0x0103_95a0;
/// `"\gore\"`
const PATH_GORE: u32 = 0x0103_9598;
/// `"\sky\"`
const PATH_SKY: u32 = 0x0103_9588;
/// `"\lod\"`
const PATH_LOD: u32 = 0x0103_957c;
/// `"\water\"`
const PATH_WATER: u32 = 0x0103_9570;
/// `"SYSTEM"`
const CATEGORY_SYSTEM: u32 = 0x0103_962c;
/// `"ACTOR"`
const CATEGORY_ACTOR: u32 = 0x0103_95c8;
/// `"MENUS"`
const CATEGORY_MENUS: u32 = 0x0103_95ac;
/// `"EFFECTS"`
const CATEGORY_EFFECTS: u32 = 0x0103_9590;
/// `"SKY"`
const CATEGORY_SKY: u32 = 0x0103_9584;
/// `"LOD"`
const CATEGORY_LOD: u32 = 0x0103_9578;
/// `"WATER"`
const CATEGORY_WATER: u32 = 0x0102_3014;
/// `"MISCREF"`
const CATEGORY_MISCREF: u32 = 0x0103_9568;

// ---- Small helpers -------------------------------------------------------------

fn console_print(e: &mut Engine, words: &[u32]) {
    e.call(CONSOLE_PRINT, words);
}

/// The item of a list node (`*LIST_ITEM_PTR(node)`).
fn node_item(e: &mut Engine, node: u32) -> u32 {
    let slot = e.call(LIST_ITEM_PTR, &args![node]).u32();
    e.mem.u32(slot)
}

fn list_is_empty(e: &mut Engine, node: u32) -> bool {
    e.call(LIST_IS_EMPTY, &args![node]).bool()
}

fn form_type(e: &mut Engine, form: u32) -> u32 {
    e.call(FORM_TYPE, &args![form]).u32()
}

fn ni_pointer_get(e: &mut Engine, slot: u32) -> u32 {
    e.call(NI_POINTER_GET, &args![slot]).u32()
}

fn player(e: &Engine) -> u32 {
    e.global::<u32>(PLAYER)
}

/// Appends `element` to the list head `list`: `005ae3d0` takes the address of
/// a cell holding the element (the game's stack local).
fn list_append(e: &mut Engine, list: Ptr, element: u32) {
    e.with_stack(4, |e, cell| {
        e.mem.set_u32(cell.addr(), element);
        e.call(LIST_APPEND, &args![list, cell]);
    });
}

// ---- Translated functions --------------------------------------------------------

// Translated from 005b4b20 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetDistanceFunction` (Xbox PDB): parses one object argument and
/// returns `Script::GetDistanceConditionFunction(thisObj, object, 0, result)`.
pub fn script_get_distance_function(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([object]) = a.parse_into(e, [0]) else {
        return false;
    };
    e.call(
        GET_DISTANCE_CONDITION,
        &args![a.this_obj, object, 0u32, a.result],
    )
    .bool()
}

// Translated from 005b4b80 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetInZoneFunction` (Xbox PDB): parses one object argument and
/// returns `Script::GetInZoneConditionFunction(thisObj, object, 0, result)`.
pub fn script_get_in_zone_function(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([object]) = a.parse_into(e, [0]) else {
        return false;
    };
    e.call(
        GET_IN_ZONE_CONDITION,
        &args![a.this_obj, object, 0u32, a.result],
    )
    .bool()
}

// Translated from 005b4be0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `AddItem` body (its failure message names AddItem): arguments are an
/// item form, a count and a flag. The items go into a temporary
/// `TESContainer` (0x10 bytes) that is then handed to the reference
/// (`004821a0`): a leveled list (form type `0x34`) is rolled at the
/// reference's level, a form list (type `0x55`) adds every element that
/// passes virtual slot `0xe4`, and any other form that passes it is added
/// directly. When nothing could be added the failure message is logged
/// (through the stub [`fn_005b5e40`]). A reference that answers true to virtual
/// slots `0x100` and `0x218` and is not the player finally gets `00606540(base
/// form, reference, result of slot 0x1e8, 1)`.
pub fn fn_005b4be0(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([form, count, flag]) = a.parse_into(e, [0, 0, 0]) else {
        return false;
    };
    if a.this_obj.is_null() {
        return true;
    }
    let level = e.call(GET_CALC_LEVEL, &args![a.this_obj, 0u32]).u32();
    e.with_stack(0x10, |e, container| {
        e.call(TES_CONTAINER_CONSTRUCT, &args![container]);
        let mut leveled_list = 0u32;
        let mut form_list = 0u32;
        let mut item = 0u32;
        let mut type_0x28 = false;
        let mut check_item = false;
        match form_type(e, form) {
            0x28 => {
                type_0x28 = true;
                check_item = true;
            }
            0x34 => leveled_list = form,
            0x55 => form_list = form,
            _ => check_item = true,
        }
        if check_item && e.vcall(form, 0xe4, &args![]).bool() {
            item = form;
        }
        if leveled_list != 0 {
            e.call(
                FN_00487F70,
                &args![
                    leveled_list + 0x30,
                    level & 0xffff,
                    count & 0xffff,
                    container,
                    0u32
                ],
            );
        } else if form_list != 0 && count != 0 {
            let mut node = e.call(FORM_LIST_ITEMS, &args![form_list]).u32();
            while node != 0 && !list_is_empty(e, node) {
                let element = node_item(e, node);
                node = e.call(LIST_NEXT, &args![node]).u32();
                if e.vcall(element, 0xe4, &args![]).bool() {
                    e.call(FN_004818E0, &args![container, element, count, 0u32]);
                }
            }
        } else if item != 0 && count != 0 {
            e.call(FN_004818E0, &args![container, item, count, 0u32]);
        } else {
            // `MSG_ADD_ITEM_FAILED` with the script's name (virtual slot
            // `0x130`) goes to the stub, which discards it.
            let _name = e.vcall(a.script_obj.addr(), 0x130, &args![]).u32();
            let _ = MSG_ADD_ITEM_FAILED;
            fn_005b5e40(e);
        }
        if type_0x28 && e.call(FN_00446390, &args![form]).i32() < 10 {
            let value: f32 = e.global(ADD_ITEM_FLOAT);
            e.call(FN_00482090, &args![container, value]);
        }
        e.call(FN_004821A0, &args![container, a.this_obj, flag == 1]);
        if e.vcall(a.this_obj.addr(), 0x100, &args![]).bool()
            && a.this_obj.addr() != player(e)
            && e.vcall(a.this_obj.addr(), 0x218, &args![]).bool()
        {
            let extra = e.vcall(a.this_obj.addr(), 0x1e8, &args![]).u32();
            let base = e.call(GET_BASE_FORM, &args![a.this_obj]).u32();
            e.call(FN_00606540, &args![base, a.this_obj, extra, 1u32]);
        }
        e.call(TES_CONTAINER_DESTRUCT, &args![container]);
    });
    true
}

// Translated from 005b4e90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Removes items from the reference: arguments are an item form, a count
/// and a flag that silences the player's message.
///
/// The forms to remove are collected into a temporary list: the form itself
/// when it passes virtual slot `0xe4` (a form of type `0x34` is looked up in
/// the reference's inventory changes instead), or every passing element of a
/// form list (type `0x55`). For each entry: a reference that is not an actor
/// just gets the entry removed through virtual slot `0x17c`; an actor has
/// the count capped to what it carries (`GetItemCountConditionFunction`), a
/// worn entry marked not wearable, the removal done through slot `0x17c`
/// and, for the player, the on-screen message
/// ([`player_removed_message`]) and the interface refresh `00704af0`.
pub fn fn_005b4e90(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([form, count, silent]) = a.parse_into(e, [0, 0, 0]) else {
        return false;
    };
    if a.this_obj.is_null() || form == 0 {
        return true;
    }
    e.with_stack(8, |e, list| {
        e.call(LIST_CONSTRUCT, &args![list]);
        let mut item = 0u32;
        if e.vcall(form, 0xe4, &args![]).bool() {
            item = form;
        }
        if form_type(e, form) == 0x34 {
            let extra = e.call(EXTRA_DATA_LIST, &args![a.this_obj]).u32();
            let changes = e.call(GET_CONTAINER_CHANGES, &args![extra]).u32();
            if changes != 0 {
                item = e.call(FN_004CBA70, &args![changes, form]).u32();
            }
        }
        if item != 0 {
            list_append(e, list, item);
        } else if form_type(e, form) == 0x55 {
            let mut node = e.call(FORM_LIST_ITEMS, &args![form]).u32();
            while node != 0 && !list_is_empty(e, node) {
                let element = node_item(e, node);
                node = e.call(LIST_NEXT, &args![node]).u32();
                if e.vcall(element, 0xe4, &args![]).bool() {
                    list_append(e, list, element);
                }
            }
        }
        while !list_is_empty(e, list.addr()) {
            let item = node_item(e, list.addr());
            e.call(LIST_POP_FRONT, &args![list]);
            let mut count = count;
            let actor = e
                .call(
                    DYNAMIC_CAST,
                    &args![a.this_obj, 0u32, RTTI_TES_OBJECT_REFR, RTTI_ACTOR, 0u32],
                )
                .u32();
            if actor == 0 {
                e.vcall(
                    a.this_obj.addr(),
                    0x17c,
                    &args![item, 0u32, count, 0u32, 0u32, 0u32, 0u32, 0u32, 1u32, 0u32],
                );
                continue;
            }
            // What the actor carries, written as a `double` by the callee.
            let carried = e.with_stack(8, |e, have| {
                e.mem.set_f64(have.addr(), count as i32 as f64);
                e.call(
                    GET_ITEM_COUNT_CONDITION,
                    &args![a.this_obj, item, 0u32, have],
                );
                e.mem.f64(have.addr())
            });
            if (count as i32 as f64) > carried {
                count = e.call(FTOL, &args![carried]).u32();
            }
            let mut worn = 0u32;
            if e.call(FN_00575400, &args![actor, item]).bool() {
                let changes = e.call(GET_INVENTORY_CHANGES, &args![a.this_obj]).u32();
                if changes != 0 {
                    worn = e.call(WEARING_OBJECT, &args![changes, item, 0u32]).u32();
                    if worn != 0 {
                        e.call(SET_CAN_NOT_WEAR, &args![worn, 0u32]);
                    }
                }
            }
            if (count as i32) <= 0 {
                continue;
            }
            let wearable = e.call(FN_00575400, &args![actor, item]).bool();
            if wearable {
                e.call(FN_008248E0, &args![actor + 0x94, item, 1u32]);
            }
            e.vcall(
                actor,
                0x17c,
                &args![item, worn, count, 0u32, 0u32, 0u32, 0u32, 0u32, 1u32, 0u32],
            );
            if actor == player(e) {
                if silent == 0 {
                    player_removed_message(e, item, count as i32, silent);
                }
                e.call(FN_00704AF0, &args![]);
            } else if wearable {
                let process = e
                    .call(MIDDLE_HIGH_PROCESS_SAVED_ACQUIRE, &args![actor])
                    .u32();
                if process != 0 && e.vcall(actor, 0x1d0, &args![]).u32() != 0 {
                    let process = e
                        .call(MIDDLE_HIGH_PROCESS_SAVED_ACQUIRE, &args![actor])
                        .u32();
                    e.vcall(process, 0xc4, &args![actor, 0u32]);
                }
            }
        }
        e.call(LIST_DESTRUCT, &args![list]);
    });
    true
}

/// The player's "item removed" message of `005b4e90`: formats
/// `"<count> <name><b> <a>"` (or `"<name> <a>"` for a single item) into a
/// temporary `BSStringT` from the two string globals; for the form types
/// below it is then shown with the vault boy icon and the item's pick-up
/// sound (none when `silent`), for every other type it is dropped.
fn player_removed_message(e: &mut Engine, item: u32, count: i32, silent: u32) {
    let text = e.mem.alloc(8);
    e.call(BS_STRING_CONSTRUCT, &args![text]);
    if count > 1 {
        let part_a = e.call(BS_STRING_TEXT, &args![MESSAGE_PART_A]).u32();
        let part_b = e.call(BS_STRING_TEXT, &args![MESSAGE_PART_B]).u32();
        let name = e.call(GET_FULL_NAME, &args![item]).u32();
        e.call(
            BS_STRING_FORMAT,
            &args![text, FORMAT_ITEM_COUNT_MESSAGE, count, name, part_b, part_a],
        );
    } else {
        let part_a = e.call(BS_STRING_TEXT, &args![MESSAGE_PART_A]).u32();
        let name = e.call(GET_FULL_NAME, &args![item]).u32();
        e.call(
            BS_STRING_FORMAT,
            &args![text, FORMAT_ITEM_MESSAGE, name, part_a],
        );
    }
    // The compiler's switch on `form type - 0x18` (a jump table over a byte
    // table): these types take the icon branch, all others do nothing.
    if matches!(
        form_type(e, item),
        0x18 | 0x19 | 0x1e | 0x1f | 0x28 | 0x29 | 0x2e | 0x2f | 0x32 | 0x67 | 0x6c | 0x73 | 0x74
    ) {
        let sound = if silent != 0 {
            0
        } else {
            let player = player(e);
            e.call(GET_PICK_UP_SOUND_NAME, &args![player, item, 0u32, 0u32])
                .u32()
        };
        let volume: f32 = e.global(PICKUP_VOLUME);
        let string = ni_pointer_get(e, text);
        e.call(
            SHOW_MESSAGE,
            &args![string, 0u32, ICON_VAULT_BOY, sound, volume, 0u32],
        );
    }
    e.call(BS_STRING_DESTRUCT, &args![text]);
    e.mem.free(text);
}

// Translated from 005b53d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Gives `containingObj` the base form of `thisObj` through virtual slot
/// `0x17c` (one parsed argument is passed along), carrying the worn entry
/// when `containingObj` is an actor that has the form, then refreshes the
/// player's quest targets. False when it did something; true when either
/// reference is missing.
pub fn fn_005b53d0(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([extra]) = a.parse_into(e, [0]) else {
        return false;
    };
    if a.this_obj.is_null() || a.containing_obj.is_null() {
        return true;
    }
    let actor = e
        .call(
            DYNAMIC_CAST,
            &args![
                a.containing_obj,
                0u32,
                RTTI_TES_OBJECT_REFR,
                RTTI_ACTOR,
                0u32
            ],
        )
        .u32();
    let mut worn = 0u32;
    if actor != 0 {
        let base = e.call(GET_BASE_FORM, &args![a.this_obj]).u32();
        if e.call(FN_00575400, &args![actor, base]).bool() {
            let changes = e
                .call(GET_INVENTORY_CHANGES, &args![a.containing_obj])
                .u32();
            if changes != 0 {
                let base = e.call(GET_BASE_FORM, &args![a.this_obj]).u32();
                worn = e.call(WEARING_OBJECT, &args![changes, base, 0u32]).u32();
            }
        }
    }
    let base = e.call(GET_BASE_FORM, &args![a.this_obj]).u32();
    e.vcall(
        a.containing_obj.addr(),
        0x17c,
        &args![base, worn, 1u32, 0u32, 0u32, extra, 0u32, 0u32, 1u32, 0u32],
    );
    let player = player(e);
    e.call(CHECK_QUEST_TARGET_UPDATE, &args![player, a.this_obj]);
    if a.containing_obj.addr() == player {
        e.call(FN_00704AF0, &args![]);
    }
    false
}

// Translated from 005b5500 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::DuplicateAllItemsFunction` (Xbox PDB): duplicates all of
/// `thisObj`'s inventory into the parsed target reference
/// (`InventoryChanges::DuplicateAllItems(thisObj, target)`) when its extra
/// data list has inventory changes.
pub fn script_duplicate_all_items_function(e: &mut Engine, a: ScriptArgs) -> bool {
    // The second argument is parsed into a local flag the function never
    // reads.
    let Some([target, _unused]) = a.parse_into(e, [0, 0]) else {
        return false;
    };
    if !a.this_obj.is_null() {
        let list = e.call(EXTRA_DATA_LIST, &args![a.this_obj]).u32();
        if list != 0 {
            let list = e.call(EXTRA_DATA_LIST, &args![a.this_obj]).u32();
            let changes = e.call(GET_CONTAINER_CHANGES, &args![list]).u32();
            if changes != 0 {
                e.call(DUPLICATE_ALL_ITEMS, &args![changes, a.this_obj, target]);
            }
        }
    }
    true
}

// Translated from 005b55a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls the inventory-changes routine `004ce340` for `thisObj` with the
/// five parsed arguments (item, flag, option, slot -1 by default, extra),
/// then refreshes the player's quest targets.
pub fn fn_005b55a0(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([item, flag, option, slot, extra]) = a.parse_into(e, [0, 0, 0, 0xffff_ffff, 0]) else {
        return false;
    };
    if !a.this_obj.is_null() {
        let list = e.call(EXTRA_DATA_LIST, &args![a.this_obj]).u32();
        if list != 0 {
            let list = e.call(EXTRA_DATA_LIST, &args![a.this_obj]).u32();
            let changes = e.call(GET_CONTAINER_CHANGES, &args![list]).u32();
            if changes != 0 {
                // The routine returns a `float` in `ST0`, which is discarded.
                e.call(
                    FN_004CE340,
                    &args![
                        changes,
                        a.this_obj,
                        item,
                        0u32,
                        flag != 0,
                        0u32,
                        option != 0,
                        slot,
                        extra
                    ],
                );
                let player = player(e);
                e.call(CHECK_QUEST_TARGET_UPDATE, &args![player, a.this_obj]);
            }
        }
    }
    true
}

// Translated from 005b5690 (decompiled, FalloutNV.exe 1.4.0.525)
/// Like [`fn_005b55a0`] with `004ce380`, three parsed arguments (item, flag,
/// option), slot `-1` and extra `0`.
pub fn fn_005b5690(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([item, flag, option]) = a.parse_into(e, [0, 0, 0]) else {
        return false;
    };
    if !a.this_obj.is_null() {
        let list = e.call(EXTRA_DATA_LIST, &args![a.this_obj]).u32();
        if list != 0 {
            let list = e.call(EXTRA_DATA_LIST, &args![a.this_obj]).u32();
            let changes = e.call(GET_CONTAINER_CHANGES, &args![list]).u32();
            if changes != 0 {
                e.call(
                    FN_004CE380,
                    &args![
                        changes,
                        a.this_obj,
                        item,
                        0u32,
                        flag != 0,
                        0u32,
                        option != 0,
                        0xffff_ffffu32,
                        0u32
                    ],
                );
                let player = player(e);
                e.call(CHECK_QUEST_TARGET_UPDATE, &args![player, a.this_obj]);
            }
        }
    }
    true
}

// Translated from 005b5760 (decompiled, FalloutNV.exe 1.4.0.525)
/// Function-style command: clears the result, parses a form (default: the
/// player's base form) and calls `005a2e20(thisObj, form, 0, result)`, which
/// fills `*result`.
pub fn fn_005b5760(e: &mut Engine, a: ScriptArgs) -> bool {
    e.mem.set_f64(a.result.addr(), 0.0);
    let Some([form]) = a.parse_into(e, [0]) else {
        return false;
    };
    let form = if form == 0 {
        let player = player(e);
        e.call(GET_BASE_FORM, &args![player]).u32()
    } else {
        form
    };
    e.call(FN_005A2E20, &args![a.this_obj, form, 0u32, a.result]);
    true
}

// Translated from 005b57e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Parses a form (default: the player's base form); for a reference, calls
/// `00419700` on its extra data list with the form and then virtual slot
/// `0x48` with `0x40`.
pub fn fn_005b57e0(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([form]) = a.parse_into(e, [0]) else {
        return false;
    };
    let form = if form == 0 {
        let player = player(e);
        e.call(GET_BASE_FORM, &args![player]).u32()
    } else {
        form
    };
    if !a.this_obj.is_null() {
        let list = e.call(EXTRA_DATA_LIST, &args![a.this_obj]).u32();
        e.call(FN_00419700, &args![list, form]);
        e.vcall(a.this_obj.addr(), 0x48, &args![0x40u32]);
    }
    true
}

// Translated from 005b5860 (decompiled, FalloutNV.exe 1.4.0.525)
/// Gives `containingObj` the base form of `thisObj` through virtual slot
/// `0x17c`, refreshes the player's quest targets and, in menu mode, calls the
/// interface refresh `00704af0`. False when it did something; true when
/// either reference is missing.
pub fn fn_005b5860(e: &mut Engine, a: ScriptArgs) -> bool {
    if a.this_obj.is_null() || a.containing_obj.is_null() {
        return true;
    }
    let base = e.call(GET_BASE_FORM, &args![a.this_obj]).u32();
    e.vcall(
        a.containing_obj.addr(),
        0x17c,
        &args![base, 0u32, 1u32, 0u32, 1u32, 0u32, 0u32, 0u32, 1u32, 0u32],
    );
    let player = player(e);
    e.call(CHECK_QUEST_TARGET_UPDATE, &args![player, a.this_obj]);
    if e.call(IS_IN_MENU_MODE, &args![]).bool() {
        e.call(FN_00704AF0, &args![]);
    }
    false
}

// Translated from 005b58d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Parses a form and a count and has `thisObj` process them through its
/// virtual slot `0x17c`.
pub fn fn_005b58d0(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([form, count]) = a.parse_into(e, [0, 0]) else {
        return false;
    };
    if !a.this_obj.is_null() {
        e.vcall(
            a.this_obj.addr(),
            0x17c,
            &args![form, 0u32, count, 0u32, 1u32, 0u32, 0u32, 0u32, 1u32, 0u32],
        );
    }
    true
}

// Translated from 005b5950 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::IsActionRefFunction` (Xbox PDB): `*result` is 1.0 when the
/// parsed reference is `thisObj`'s action reference, else 0.0; echoes the
/// value to the console when the TLS echo flag is set.
pub fn script_is_action_ref_function(e: &mut Engine, a: ScriptArgs) -> bool {
    e.mem.set_f64(a.result.addr(), 0.0);
    let Some([reference]) = a.parse_into(e, [0]) else {
        return false;
    };
    if !a.this_obj.is_null() && reference != 0 {
        let action_ref = e.call(GET_ACTION_REF, &args![a.this_obj]).u32();
        if reference == action_ref {
            e.mem.set_f64(a.result.addr(), 1.0);
        }
    }
    let tls = e.tls();
    if e.mem.u8(tls + TLS_ECHO) != 0 {
        let value = e.mem.f64(a.result.addr());
        console_print(e, &args![MSG_IS_ACTION_REF, value]);
    }
    true
}

// Translated from 005b59f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::ActivateFunction` (Xbox PDB): has `thisObj` activate the parsed
/// reference (default: its action reference) unless that reference has form
/// flag `0x800`. Nested more than [`ACTIVATE_DEPTH_LIMIT`] + 1 deep (a
/// counter in the TLS block) it does nothing. With the second argument zero,
/// action 1 is set for the duration if it was not set, and action 2 always.
/// The player may not run it: the command logs a message (through the stub
/// [`fn_005b5e40`]) and fails.
pub fn script_activate_function(e: &mut Engine, a: ScriptArgs) -> bool {
    if player(e) == a.this_obj.addr() {
        let _ = MSG_PLAYER_ACTIVATE;
        fn_005b5e40(e);
        return false;
    }
    let Some([target, direct]) = a.parse_into(e, [0, 0]) else {
        return false;
    };
    if a.this_obj.is_null() {
        return true;
    }
    let tls = e.tls();
    let depth = e.mem.u32(tls + TLS_ACTIVATE_DEPTH);
    if depth > ACTIVATE_DEPTH_LIMIT {
        return true;
    }
    e.mem
        .set_u32(tls + TLS_ACTIVATE_DEPTH, depth.wrapping_add(1));
    let target = if target == 0 {
        e.call(GET_ACTION_REF, &args![a.this_obj]).u32()
    } else {
        target
    };
    if target != 0 && !e.call(FN_00440DA0, &args![target]).bool() {
        if direct == 0 {
            let had_action = e.call(HAS_ACTION, &args![a.this_obj, 1u32]).bool();
            if !had_action {
                e.call(SET_ACTION, &args![a.this_obj, 1u32]);
            }
            e.call(SET_ACTION, &args![a.this_obj, 2u32]);
            e.call(ACTIVATE, &args![a.this_obj, target, 0u32, 0u32, 1u32]);
            e.call(CLEAR_ACTION, &args![a.this_obj, 2u32]);
            if !had_action {
                e.call(CLEAR_ACTION, &args![a.this_obj, 1u32]);
            }
        } else {
            e.call(ACTIVATE, &args![a.this_obj, target, 0u32, 0u32, 1u32]);
        }
    }
    let depth = e.mem.u32(tls + TLS_ACTIVATE_DEPTH);
    e.mem
        .set_u32(tls + TLS_ACTIVATE_DEPTH, depth.wrapping_sub(1));
    true
}

// Translated from 005b5b90 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::ToggleCombatStatsFunction` (Xbox PDB): prints that the command
/// was removed.
pub fn script_toggle_combat_stats_function(e: &mut Engine) -> bool {
    console_print(e, &args![MSG_COMBAT_STATS_REMOVED]);
    true
}

// Translated from 005b5bb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::PlayerSpellBook` (Xbox PDB): walks the data handler's spell list
/// (`+0x40`); for every non-null entry whose member at `+0x18` reports the
/// type 0, 2 or 3 through its virtual slot `0x18`, asks the player (virtual
/// slot `0x3e0`) to take it, counts the successes and prints the count.
pub fn script_player_spell_book(e: &mut Engine) -> bool {
    let handler = e.global::<u32>(DATA_HANDLER);
    let mut node = e.call(DATA_HANDLER_SPELLS, &args![handler]).u32();
    let mut added = 0u32;
    while node != 0 {
        if e.call(LIST_COUNT, &args![node]).u32() == 0 {
            break;
        }
        if node_item(e, node) != 0 {
            // The type is asked afresh at every test, as the game does.
            let kind = |e: &mut Engine| {
                let entry = node_item(e, node);
                e.vcall(entry + 0x18, 0x18, &args![]).u32()
            };
            let take = kind(e) == 0 || kind(e) == 2 || kind(e) == 3;
            if take {
                let spell = node_item(e, node);
                let player = player(e);
                if e.vcall(player, 0x3e0, &args![spell]).bool() {
                    added += 1;
                }
            }
        }
        node = e.call(LIST_NEXT, &args![node]).u32();
    }
    console_print(e, &args![MSG_SPELLS_ADDED, added]);
    true
}

// Translated from 005b5cc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::ShowWhoDetectsPlayer` (Xbox PDB): prints the detection lists.
pub fn script_show_who_detects_player(e: &mut Engine) -> bool {
    e.call(
        PROCESS_LISTS_PRINT_LISTS,
        &args![PROCESS_LISTS, 1u32, -1i32],
    );
    console_print(e, &args![MSG_DETECTION_LIST_PRINTED]);
    true
}

// Translated from 005b5cf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::PrintAILists` (Xbox PDB): prints the AI lists.
pub fn script_print_ai_lists(e: &mut Engine) -> bool {
    e.call(
        PROCESS_LISTS_PRINT_LISTS,
        &args![PROCESS_LISTS, 0u32, -1i32],
    );
    console_print(e, &args![MSG_AI_LISTS_PRINTED]);
    true
}

// Translated from 005b5d20 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls the (empty) `00483710` on the process lists singleton.
pub fn fn_005b5d20(e: &mut Engine) -> bool {
    e.call(FN_00483710, &args![PROCESS_LISTS]);
    true
}

// Translated from 005b5d40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Parses one integer and stores whether it is non-zero in the byte global
/// `011f122d`; false when the arguments do not parse.
pub fn fn_005b5d40(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([value]) = a.parse_into(e, [0]) else {
        return false;
    };
    e.set_global(FLAG_0011F122D, (value != 0) as u8);
    true
}

// Translated from 005b5da0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the byte global `011f9fc0` to 1.
pub fn fn_005b5da0(e: &mut Engine) -> bool {
    e.set_global(FLAG_0011F9FC0, 1u8);
    true
}

// Translated from 005b5db0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::FreezeRendererAccumulation` (Xbox PDB): prints that the command
/// is deprecated.
pub fn script_freeze_renderer_accumulation(e: &mut Engine) -> bool {
    console_print(e, &args![MSG_FREEZE_RENDERER_DEPRECATED]);
    true
}

// Translated from 005b5dd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::ToggleOcclusion` (Xbox PDB): flips the occlusion query flag and
/// prints its new state.
pub fn script_toggle_occlusion(e: &mut Engine) -> bool {
    let enabled = e.global::<u8>(OCCLUSION_QUERY) == 0;
    e.set_global(OCCLUSION_QUERY, enabled as u8);
    let state = if enabled { TEXT_ON } else { TEXT_OFF };
    console_print(e, &args![MSG_OCCLUSION_QUERY, state]);
    true
}

// Translated from 005b5e20 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::OutputMemContexts` (Xbox PDB): prints that the command is no
/// longer supported.
pub fn script_output_mem_contexts(e: &mut Engine) -> bool {
    console_print(e, &args![MSG_MEM_CONTEXTS_UNSUPPORTED]);
    true
}

// Translated from 005b5e40 (decompiled, FalloutNV.exe 1.4.0.525)
/// A logging stub: accepts whatever arguments the caller pushes and returns
/// 0 (the release build compiles the log call down to this). Callers in this
/// unit pass no arguments here.
pub fn fn_005b5e40(_e: &mut Engine) -> u32 {
    0
}

// Translated from 005b5e60 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::OutputMemStats` (Xbox PDB): the memory statistics dump.
///
/// Opens the memory heap dump file named by the argument (default
/// `OutputMemStats.mhd`, otherwise `MemStats<name>.mhd`), then writes
/// `ModelDump<name>.txt` (the model loader's and the face-gen manager's
/// model maps) and `TextureDump<name>.txt`: every live texture is inserted
/// into a table of 0x800 `NiPointer`s ordered by size (the size function is
/// the stub [`fn_005b5e40`], so every size is 0), then one tab-separated line
/// is written per table entry (index, category by path, description, width x
/// height, size in KB, format, reference count minus one, "L" or "H", and
/// the owner's name).
///
/// Differences from the game: when the argument does not parse, the game's
/// `<name>` buffer is uninitialised stack (zeroed here); two locals the game
/// fills (total size, texture count) and never reads are left out.
pub fn script_output_mem_stats(e: &mut Engine, a: ScriptArgs) -> bool {
    let path = e.mem.alloc(0x200);
    let name = e.mem.alloc(0x104);
    let description = e.mem.alloc(0x104);
    let line = e.mem.alloc(0x200);
    let heap_dump = e.mem.alloc(0x18);
    let model_file = e.mem.alloc(BS_FILE_SIZE);
    let texture_file = e.mem.alloc(BS_FILE_SIZE);
    let table = e.mem.alloc(4 * TEXTURE_SLOTS);

    e.mem.set_u8(path, 0);
    e.call(MEMSET, &args![path + 1, 0u32, 0x1ffu32]);
    if a.parse(e, &[path]) {
        e.call(STRCPY_S, &args![name, 0x104u32, path]);
        e.call(SPRINTF_S, &args![path, 0x200u32, FORMAT_MEM_STATS, name]);
    } else {
        e.call(STRCPY_S, &args![path, 0x200u32, DEFAULT_MEM_STATS]);
    }
    e.call(MEM_STATS_FILE_CONSTRUCT, &args![heap_dump, path]);
    let output = e.call(MEM_STATS_OUTPUT_GET, &args![heap_dump, 0u32]).u32();
    e.call(MEM_STATS_OUTPUT_SET, &args![output]);

    e.call(SPRINTF_S, &args![path, 0x200u32, FORMAT_MODEL_DUMP, name]);
    e.call(
        BS_FILE_CONSTRUCT,
        &args![model_file, path, 1u32, 0x4000u32, 0u32],
    );
    e.call(BS_FILE_OPEN, &args![model_file, 0u32, 0u32]);
    let model_loader = e.global::<u32>(MODEL_LOADER);
    if model_loader != 0 {
        e.call(
            MODEL_LOADER_OUTPUT_MODEL_MAP,
            &args![model_loader, model_file, 0u32],
        );
    }
    if e.call(GET_MODEL_CACHE, &args![]).u32() != 0 {
        let cache = e.call(GET_MODEL_CACHE, &args![]).u32();
        e.call(FACE_GEN_OUTPUT_MODEL_MAP, &args![cache, model_file]);
    }
    e.call(BS_FILE_CLOSE, &args![model_file]);

    e.call(SPRINTF_S, &args![path, 0x200u32, FORMAT_TEXTURE_DUMP, name]);
    e.call(
        BS_FILE_CONSTRUCT,
        &args![texture_file, path, 1u32, 0x4000u32, 0u32],
    );
    e.call(BS_FILE_OPEN, &args![texture_file, 0u32, 0u32]);
    e.call(
        VECTOR_CONSTRUCT,
        &args![
            table,
            4u32,
            TEXTURE_SLOTS,
            NI_POINTER_CONSTRUCT,
            NI_POINTER_DESTRUCT
        ],
    );
    e.call(ZERO_FILL, &args![table, 0u32, 4 * TEXTURE_SLOTS]);

    // Insert every texture before the first table entry that is empty or
    // smaller than it, shifting the entries above up by one.
    let mut texture = e.call(FIRST_TEXTURE, &args![]).u32();
    while texture != 0 {
        if e.call(GET_D3D_TEXTURE, &args![texture]).u32() != 0 {
            let size = fn_005b5e40(e);
            let mut insert_at = -1i32;
            let mut i = 0i32;
            while i < TEXTURE_SLOTS as i32 && insert_at == -1 {
                let slot = table + 4 * i as u32;
                if ni_pointer_get(e, slot) == 0 {
                    insert_at = i;
                } else {
                    // The game fetches the pointer again to pass it to the
                    // size stub.
                    let _other = ni_pointer_get(e, slot);
                    if fn_005b5e40(e) < size {
                        insert_at = i;
                    }
                }
                i += 1;
            }
            if insert_at != -1 {
                let mut j = TEXTURE_SLOTS as i32 - 1;
                while j > insert_at {
                    let to = table + 4 * j as u32;
                    e.call(NI_POINTER_ASSIGN, &args![to, to - 4]);
                    j -= 1;
                }
                e.call(
                    NI_POINTER_SET,
                    &args![table + 4 * insert_at as u32, texture],
                );
            }
        }
        texture = e.call(NEXT_TEXTURE, &args![texture]).u32();
    }

    for i in 0..TEXTURE_SLOTS {
        let slot = table + 4 * i;
        if ni_pointer_get(e, slot) == 0 {
            continue;
        }
        let mut rendered = false;
        let candidate = ni_pointer_get(e, slot);
        if e.call(TEXTURE_IS_SOURCE, &args![TEXTURE_CLASS, candidate])
            .bool()
        {
            let texture = ni_pointer_get(e, slot);
            let source = e.vcall(texture, 0x9c, &args![]).u32();
            let mut text = e.call(NAME_TEXT, &args![source]).u32();
            if text == 0 || e.mem.i8(text) == 0 {
                let texture = ni_pointer_get(e, slot);
                let source = e.call(FN_00413F40, &args![texture]).u32();
                text = e.call(NAME_TEXT, &args![source]).u32();
            }
            if text == 0 || e.mem.i8(text) == 0 {
                e.call(
                    STRCPY_S,
                    &args![description, 0x104u32, TEXT_UNNAMED_SOURCE_TEXTURE],
                );
            } else {
                e.call(STRCPY_S, &args![description, 0x104u32, text]);
            }
        } else {
            rendered = true;
            let texture = ni_pointer_get(e, slot);
            let source = e.call(FN_00413F40, &args![texture]).u32();
            let text = e.call(NAME_TEXT, &args![source]).u32();
            if text != 0 && e.mem.i8(text) != 0 {
                e.call(
                    SPRINTF_S,
                    &args![description, 0x104u32, FORMAT_RENDERED_TEXTURE, text],
                );
            } else {
                e.call(
                    SPRINTF_S,
                    &args![description, 0x104u32, TEXT_RENDERED_TEXTURE_NONE],
                );
            }
        }
        e.call(FIX_PATH, &args![description, 0x104u32]);
        let mut category = CATEGORY_SYSTEM;
        if !rendered {
            let contains = |e: &mut Engine, needle: u32| {
                e.call(FIND_SUBSTRING, &args![description, needle]).u32() != 0
            };
            category = if [
                PATH_ARMOR,
                PATH_CREATURES,
                PATH_CHARACTERS,
                PATH_PIPBOY,
                PATH_WEAPONS,
                PATH_DECALS,
                PATH_PROJECTILES,
            ]
            .into_iter()
            .any(|needle| contains(e, needle))
            {
                CATEGORY_ACTOR
            } else if contains(e, PATH_FONTS) || contains(e, PATH_INTERFACE) {
                CATEGORY_MENUS
            } else if contains(e, PATH_EFFECTS) || contains(e, PATH_GORE) {
                CATEGORY_EFFECTS
            } else if contains(e, PATH_SKY) {
                CATEGORY_SKY
            } else if contains(e, PATH_LOD) {
                CATEGORY_LOD
            } else if contains(e, PATH_WATER) {
                CATEGORY_WATER
            } else {
                CATEGORY_MISCREF
            };
        }
        // The game passes the pointer to the size stub.
        let _size_argument = ni_pointer_get(e, slot);
        let size = fn_005b5e40(e);
        let texture = ni_pointer_get(e, slot);
        let d3d_texture = e.call(GET_D3D_TEXTURE, &args![texture]).u32();
        let surface = e.call(FN_007D6BB0, &args![d3d_texture]).u32();
        let format_id = e.call(D3D_FORMAT_ID, &args![surface]).u32();
        let format_name = e.call(GET_D3D_FORMAT_STRING, &args![format_id]).u32();
        let texture = ni_pointer_get(e, slot);
        let owner = e.vcall(texture, 8, &args![]).u32();
        let owner_name = ni_pointer_get(e, owner);
        let texture = ni_pointer_get(e, slot);
        let flag_text = if e.call(TEXTURE_KIND, &args![texture]).u32() == 3 {
            TEXT_L
        } else {
            TEXT_H
        };
        let for_height = ni_pointer_get(e, slot);
        let for_width = ni_pointer_get(e, slot);
        let texture = ni_pointer_get(e, slot);
        let references = e.call(LIST_NEXT, &args![texture]).u32().wrapping_sub(1);
        let height = e.vcall(for_height, 0x98, &args![]).u32();
        let width = e.vcall(for_width, 0x94, &args![]).u32();
        e.call(
            SPRINTF_S,
            &args![
                line,
                0x200u32,
                FORMAT_TEXTURE_LINE,
                i + 1,
                category,
                description,
                width,
                height,
                size >> 10,
                format_name,
                references,
                flag_text,
                owner_name
            ],
        );
        fn_005b5e40(e);
        let length = e.call(STRLEN, &args![line]).u32();
        e.call(BS_FILE_WRITE, &args![texture_file, line, length + 1]);
    }

    e.call(BS_FILE_CLOSE, &args![texture_file]);
    e.call(
        VECTOR_DESTRUCT,
        &args![table, 4u32, TEXTURE_SLOTS, NI_POINTER_DESTRUCT],
    );
    e.call(BS_FILE_DESTRUCT, &args![texture_file]);
    e.call(BS_FILE_DESTRUCT, &args![model_file]);
    e.call(MEM_STATS_FILE_DESTRUCT, &args![heap_dump]);

    for block in [
        path,
        name,
        description,
        line,
        heap_dump,
        model_file,
        texture_file,
        table,
    ] {
        e.mem.free(block);
    }
    true
}

// Translated from 005b6800 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::OutputTextureUseMap` (Xbox PDB): writes the texture use map of
/// the reference's 3D (virtual slot `0x1d0`), or of the whole scene when
/// there is none, to the file named by the argument (default
/// `TextureUseMap.txt`), prints whether that worked and the size of the
/// textures in use.
pub fn script_output_texture_use_map(e: &mut Engine, a: ScriptArgs) -> bool {
    let path = e.mem.alloc(0x200);
    let map = e.mem.alloc(0x14);
    e.mem.set_u8(path, 0);
    e.call(MEMSET, &args![path + 1, 0u32, 0x1ffu32]);
    e.call(STRCPY_S, &args![path, 0x200u32, DEFAULT_TEXTURE_USE_MAP]);
    if !a.parse(e, &[path]) {
        e.call(STRCPY_S, &args![path, 0x200u32, DEFAULT_TEXTURE_USE_MAP]);
    }
    e.call(TEXTURE_USE_MAP_CONSTRUCT, &args![map]);
    if !a.this_obj.is_null() && e.vcall(a.this_obj.addr(), 0x1d0, &args![]).u32() != 0 {
        let node = e.vcall(a.this_obj.addr(), 0x1d0, &args![]).u32();
        e.call(TEXTURE_USE_MAP_ADD_NODE, &args![map, node, 1u32]);
    } else {
        e.call(TEXTURE_USE_MAP_ADD_SCENE, &args![map, 1u32]);
    }
    if e.call(TEXTURE_USE_MAP_WRITE_TO_FILE, &args![map, path])
        .bool()
    {
        console_print(e, &args![MSG_MAP_WRITTEN, path]);
    } else {
        console_print(e, &args![MSG_MAP_WRITE_FAILED, path]);
    }
    let text = e.mem.alloc(8);
    e.call(BS_STRING_CONSTRUCT, &args![text]);
    let mut size = e.call(TEXTURE_USE_MAP_TOTAL_SIZE, &args![map]).u32();
    if size > 0x10_0000 {
        size >>= 20;
        e.call(BS_STRING_FORMAT, &args![text, FORMAT_MB, size]);
    } else if size > 0x400 {
        size >>= 10;
        e.call(BS_STRING_FORMAT, &args![text, FORMAT_KB, size]);
    } else {
        e.call(BS_STRING_FORMAT, &args![text, FORMAT_BYTES, size]);
    }
    e.call(BS_STRING_APPEND, &args![text, TEXT_TEXTURES_IN_USE]);
    let message = ni_pointer_get(e, text);
    console_print(e, &args![message]);
    e.call(BS_STRING_DESTRUCT, &args![text]);
    e.call(TEXTURE_USE_MAP_DESTRUCT, &args![map]);
    e.mem.free(text);
    e.mem.free(map);
    e.mem.free(path);
    true
}

// Translated from 005b6a30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Opens the memory heap dump file named by the argument (default
/// `OutputMemStats.mhd`) the way `Script::OutputMemStats` does and
/// configures the output object `011f6238` with flag 1 (where
/// `OutputMemStats` uses flag 0).
pub fn fn_005b6a30(e: &mut Engine, a: ScriptArgs) -> bool {
    let path = e.mem.alloc(0x200);
    let heap_dump = e.mem.alloc(0x18);
    e.mem.set_u8(path, 0);
    e.call(MEMSET, &args![path + 1, 0u32, 0x1ffu32]);
    e.call(STRNCPY_S, &args![path, DEFAULT_MEM_STATS, 0x1ffu32]);
    if !a.parse(e, &[path]) {
        e.call(STRNCPY_S, &args![path, DEFAULT_MEM_STATS, 0x1ffu32]);
    }
    e.call(MEM_STATS_FILE_CONSTRUCT, &args![heap_dump, path]);
    let output = e.call(MEM_STATS_OUTPUT_GET, &args![heap_dump, 1u32]).u32();
    e.call(MEM_STATS_OUTPUT_SET, &args![output]);
    e.call(MEM_STATS_FILE_DESTRUCT, &args![heap_dump]);
    e.mem.free(heap_dump);
    e.mem.free(path);
    true
}

// Translated from 005b6b50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::OutputArchiveProfile` (Xbox PDB): the file name argument
/// (default `ArchiveProfile.txt`) gets `.txt` appended when it has none;
/// archive profiling (`00af4390`) is compiled out of this build, so the
/// command normally prints that it is not enabled.
pub fn script_output_archive_profile(e: &mut Engine, a: ScriptArgs) -> bool {
    let path = e.mem.alloc(0x200);
    e.call(STRCPY_S, &args![path, 0x200u32, DEFAULT_ARCHIVE_PROFILE]);
    if !a.parse(e, &[path]) {
        e.call(STRCPY_S, &args![path, 0x200u32, DEFAULT_ARCHIVE_PROFILE]);
    }
    if e.call(FIND_SUBSTRING, &args![path, EXTENSION_TXT]).u32() == 0 {
        e.call(STRCAT_S, &args![path, 0x200u32, EXTENSION_TXT]);
    }
    if e.call(ARCHIVE_PROFILE_START, &args![path]).bool() {
        console_print(e, &args![MSG_ARCHIVE_PROFILE_OUTPUT, path]);
    } else {
        console_print(e, &args![MSG_ARCHIVE_PROFILE_DISABLED]);
    }
    e.mem.free(path);
    true
}

// Translated from 005b6c50 (decompiled, FalloutNV.exe 1.4.0.525)
/// For each of the two singletons `0044f560` and `004de490` return: calls
/// `005e01b0(singleton, singleton + 4)`.
pub fn fn_005b6c50(e: &mut Engine) -> bool {
    for getter in [FN_0044F560, FN_004DE490] {
        let singleton = e.call(getter, &args![]).u32();
        let plus_4 = e.call(PLUS_4, &args![singleton]).u32();
        let singleton = e.call(getter, &args![]).u32();
        e.call(FN_005E01B0, &args![singleton, plus_4]);
    }
    true
}

// Translated from 005b6c90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Prints "Bye." and sets the byte at `+1` of the singleton `011dea0c`
/// (`005b6cb0`).
pub fn fn_005b6c90(e: &mut Engine) -> bool {
    console_print(e, &args![MSG_BYE]);
    let singleton = Ptr::new(e.global::<u32>(GLOBAL_0011DEA0C));
    fn_005b6cb0(e, singleton);
    true
}

// Translated from 005b6cb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the byte at `this + 1` to 1.
pub fn fn_005b6cb0(e: &mut Engine, this: Ptr) {
    e.mem.set_u8(this.addr() + 1, 1);
}

// Translated from 005b6cd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Frees released objects: sets up a 12-byte level scope `(1, 1, 1)`
/// (`00878160`), calls `004539a0(1, 0)` on the singleton `011dea10`, calls
/// `MemoryLevelManager::FreeReleasedObjects` with the byte at `+5` of the
/// scope and tears the scope down (`00878200`).
pub fn fn_005b6cd0(e: &mut Engine) -> bool {
    e.with_stack(12, |e, scope| {
        e.call(FN_00878160, &args![scope, 1u32, 1u32, 1u32]);
        let singleton = e.global::<u32>(GLOBAL_0011DEA10);
        e.call(FN_004539A0, &args![singleton, 1u32, 0u32]);
        let level = e.mem.u8(scope.addr() + 5);
        e.call(FREE_RELEASED_OBJECTS, &args![level]);
        e.call(FN_00878200, &args![scope]);
    });
    true
}

// Translated from 005b6d20 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls `00848e70(0, 0)` on the save/load singleton `011ddf38`.
pub fn fn_005b6d20(e: &mut Engine) -> bool {
    let singleton = e.global::<u32>(SAVE_LOAD_GAME);
    e.call(FN_00848E70, &args![singleton, 0u32, 0u32]);
    true
}

// Translated from 005b6d40 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::ToggleMapMarkersFunction` (Xbox PDB): shows or hides the map
/// markers of every world space in the data handler's list.
///
/// Arguments (defaults 1, 1, 1): `visible`, `travel`, `skip_hidden`. A marker
/// whose data is hidden is skipped when `skip_hidden` is set; otherwise its
/// visibility is set to `visible` and, when shown and it has no travel
/// location yet, the travel location to `travel` (when hidden both are
/// cleared); then virtual slot `0x48` is called with `0x80000000`. Prints
/// the new state when the TLS echo flag is set.
pub fn script_toggle_map_markers_function(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([visible, travel, skip_hidden]) = a.parse_into(e, [1, 1, 1]) else {
        return false;
    };
    let handler = e.global::<u32>(DATA_HANDLER);
    let mut world_node = e.call(DATA_HANDLER_WORLD_SPACES, &args![handler]).u32();
    while world_node != 0 && !list_is_empty(e, world_node) {
        let world = node_item(e, world_node);
        world_node = e.call(LIST_NEXT, &args![world_node]).u32();
        let markers = e.call(BUILD_MAP_MARKER_LIST, &args![world, 0u32]).u32();
        if markers == 0 {
            continue;
        }
        while !list_is_empty(e, markers) {
            let marker = node_item(e, markers);
            e.call(LIST_POP_FRONT, &args![markers]);
            let data = e.call(GET_MAP_MARKER_DATA, &args![marker]).u32();
            if skip_hidden != 0 && e.call(MAP_MARKER_GET_HIDDEN, &args![data]).bool() {
                continue;
            }
            if visible == 0 {
                e.call(MAP_MARKER_SET_VISIBLE, &args![data, 0u32]);
                e.call(MAP_MARKER_SET_TRAVEL_LOC, &args![data, 0u32]);
            } else {
                e.call(MAP_MARKER_SET_VISIBLE, &args![data, 1u32]);
                if !e.call(MAP_MARKER_GET_TRAVEL_LOC, &args![data]).bool() {
                    e.call(MAP_MARKER_SET_TRAVEL_LOC, &args![data, travel != 0]);
                }
            }
            e.vcall(marker, 0x48, &args![0x8000_0000u32]);
        }
        e.call(LIST_DELETE, &args![markers, 1u32]);
    }
    let tls = e.tls();
    if e.mem.u8(tls + TLS_ECHO) != 0 {
        let state = if (visible as i32) > 0 {
            TEXT_SHOWN
        } else {
            TEXT_HIDDEN
        };
        console_print(e, &args![MSG_MAP_MARKERS, state]);
    }
    true
}

// Translated from 005b6f10 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::ToggleVerboseFunction` (Xbox PDB): flips the verbose messages
/// flag and prints the new state.
pub fn script_toggle_verbose_function(e: &mut Engine) -> bool {
    let verbose = fn_005b6f70(e);
    fn_005b6f60(e, !verbose);
    let state = if fn_005b6f70(e) {
        TEXT_SHOWN
    } else {
        TEXT_HIDDEN
    };
    console_print(e, &args![MSG_VERBOSE, state]);
    true
}

// Translated from 005b6f60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the verbose messages flag (byte global `011f158c`).
pub fn fn_005b6f60(e: &mut Engine, value: bool) {
    e.set_global(VERBOSE_MESSAGES, value as u8);
}

// Translated from 005b6f70 (decompiled, FalloutNV.exe 1.4.0.525)
/// The verbose messages flag (byte global `011f158c`).
pub fn fn_005b6f70(e: &mut Engine) -> bool {
    e.global::<u8>(VERBOSE_MESSAGES) != 0
}

// Translated from 005b6f80 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::ToggleGodModeFunction` (Xbox PDB): flips god mode; turning it
/// off also turns demigod mode off (with a message); prints the new state.
pub fn script_toggle_god_mode_function(e: &mut Engine) -> bool {
    let god = e.call(IS_GOD_MODE, &args![]).bool();
    e.call(SET_GOD_MODE, &args![!god]);
    if !e.call(IS_GOD_MODE, &args![]).bool() && e.call(IS_DEMIGOD_MODE, &args![]).bool() {
        e.call(SET_DEMIGOD_MODE, &args![false]);
        console_print(e, &args![MSG_DEMIGOD_DISABLED]);
    }
    let state = if e.call(IS_GOD_MODE, &args![]).bool() {
        TEXT_ENABLED
    } else {
        TEXT_DISABLED
    };
    console_print(e, &args![MSG_GOD_MODE, state]);
    true
}

// Translated from 005b7000 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::ToggleDemigodModeFunction` (Xbox PDB): flips demigod mode, sets
/// god mode to the same new value and prints the new demigod state.
pub fn script_toggle_demigod_mode_function(e: &mut Engine) -> bool {
    let demigod = e.call(IS_DEMIGOD_MODE, &args![]).bool();
    e.call(SET_DEMIGOD_MODE, &args![!demigod]);
    e.call(SET_GOD_MODE, &args![!demigod]);
    let state = if e.call(IS_DEMIGOD_MODE, &args![]).bool() {
        TEXT_ENABLED
    } else {
        TEXT_DISABLED
    };
    console_print(e, &args![MSG_DEMIGOD_MODE, state]);
    true
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x005b4b20, script_get_distance_function(ScriptArgs) -> bool),
        entry!(0x005b4b80, script_get_in_zone_function(ScriptArgs) -> bool),
        entry!(0x005b4be0, fn_005b4be0(ScriptArgs) -> bool),
        entry!(0x005b4e90, fn_005b4e90(ScriptArgs) -> bool),
        entry!(0x005b53d0, fn_005b53d0(ScriptArgs) -> bool),
        entry!(
            0x005b5500,
            script_duplicate_all_items_function(ScriptArgs) -> bool
        ),
        entry!(0x005b55a0, fn_005b55a0(ScriptArgs) -> bool),
        entry!(0x005b5690, fn_005b5690(ScriptArgs) -> bool),
        entry!(0x005b5760, fn_005b5760(ScriptArgs) -> bool),
        entry!(0x005b57e0, fn_005b57e0(ScriptArgs) -> bool),
        entry!(0x005b5860, fn_005b5860(ScriptArgs) -> bool),
        entry!(0x005b58d0, fn_005b58d0(ScriptArgs) -> bool),
        entry!(
            0x005b5950,
            script_is_action_ref_function(ScriptArgs) -> bool
        ),
        entry!(0x005b59f0, script_activate_function(ScriptArgs) -> bool),
        entry!(0x005b5b90, script_toggle_combat_stats_function() -> bool),
        entry!(0x005b5bb0, script_player_spell_book() -> bool),
        entry!(0x005b5cc0, script_show_who_detects_player() -> bool),
        entry!(0x005b5cf0, script_print_ai_lists() -> bool),
        entry!(0x005b5d20, fn_005b5d20() -> bool),
        entry!(0x005b5d40, fn_005b5d40(ScriptArgs) -> bool),
        entry!(0x005b5da0, fn_005b5da0() -> bool),
        entry!(0x005b5db0, script_freeze_renderer_accumulation() -> bool),
        entry!(0x005b5dd0, script_toggle_occlusion() -> bool),
        entry!(0x005b5e20, script_output_mem_contexts() -> bool),
        entry!(0x005b5e40, fn_005b5e40() -> u32),
        entry!(0x005b5e60, script_output_mem_stats(ScriptArgs) -> bool),
        entry!(
            0x005b6800,
            script_output_texture_use_map(ScriptArgs) -> bool
        ),
        entry!(0x005b6a30, fn_005b6a30(ScriptArgs) -> bool),
        entry!(
            0x005b6b50,
            script_output_archive_profile(ScriptArgs) -> bool
        ),
        entry!(0x005b6c50, fn_005b6c50() -> bool),
        entry!(0x005b6c90, fn_005b6c90() -> bool),
        entry!(0x005b6cb0, fn_005b6cb0(Ptr)),
        entry!(0x005b6cd0, fn_005b6cd0() -> bool),
        entry!(0x005b6d20, fn_005b6d20() -> bool),
        entry!(
            0x005b6d40,
            script_toggle_map_markers_function(ScriptArgs) -> bool
        ),
        entry!(0x005b6f10, script_toggle_verbose_function() -> bool),
        entry!(0x005b6f60, fn_005b6f60(bool)),
        entry!(0x005b6f70, fn_005b6f70() -> bool),
        entry!(0x005b6f80, script_toggle_god_mode_function() -> bool),
        entry!(0x005b7000, script_toggle_demigod_mode_function() -> bool),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    // Fake virtual functions the tests put into vtables.
    const V_TRUE: u32 = 0x0900_0001;
    const V_FALSE: u32 = 0x0900_0002;
    const V_NINE: u32 = 0x0900_0003;
    const V_RECORD: u32 = 0x0900_0004;
    /// `V_KIND + k` returns `k`.
    const V_KIND: u32 = 0x0900_0010;

    /// An engine with the pages of the globals these commands touch mapped
    /// and the small accessors every command uses replaced by doubles that
    /// behave like the exe's code (see the constants' documentation).
    fn engine() -> Engine {
        let mut e = Engine::new();
        for page in [
            0x0101_2000,
            0x0101_6000,
            0x0102_3000,
            0x0103_9000,
            0x011c_3000,
            0x011d_d000,
            0x011d_e000,
            0x011e_0000,
            0x011f_1000,
            0x011f_6000,
            0x011f_9000,
        ] {
            e.map(page, 0x1000);
        }
        e.register(V_TRUE, |_, _| true.into_ret());
        e.register(V_FALSE, |_, _| false.into_ret());
        e.register(V_NINE, |_, _| 9u32.into_ret());
        e.register(V_RECORD, |_, _| Ret::default());
        for k in 0..4u32 {
            e.register_double(V_KIND + k, move |_, _| k.into_ret());
        }
        e.register(FORM_TYPE, |e, a| (e.mem.u8(a[0] + 4) as u32).into_ret());
        e.register(GET_BASE_FORM, |e, a| e.mem.u32(a[0] + 0x20).into_ret());
        e.register(EXTRA_DATA_LIST, |_, a| (a[0] + 0x44).into_ret());
        e.register(NI_POINTER_GET, |e, a| e.mem.u32(a[0]).into_ret());
        list_doubles(&mut e);
        e
    }

    /// Doubles for the `BSSimpleList` node functions, behaving like the
    /// exe's (item at +0, next at +4).
    fn list_doubles(e: &mut Engine) {
        e.register(LIST_IS_EMPTY, |e, a| {
            (e.mem.u32(a[0]) == 0 && e.mem.u32(a[0] + 4) == 0).into_ret()
        });
        e.register(LIST_ITEM_PTR, |_, a| a[0].into_ret());
        e.register(LIST_NEXT, |e, a| e.mem.u32(a[0] + 4).into_ret());
        e.register(LIST_POP_FRONT, |e, a| {
            let head = a[0];
            let next = e.mem.u32(head + 4);
            if next != 0 {
                let item = e.mem.u32(next);
                let after = e.mem.u32(next + 4);
                e.mem.set_u32(head, item);
                e.mem.set_u32(head + 4, after);
            } else {
                e.mem.set_u32(head, 0);
            }
            Ret::default()
        });
        e.register(LIST_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], 0);
            e.mem.set_u32(a[0] + 4, 0);
            Ret::default()
        });
        e.register(LIST_APPEND, |e, a| {
            let item = e.mem.u32(a[1]);
            if item != 0 {
                let head = a[0];
                if e.mem.u32(head) == 0 {
                    e.mem.set_u32(head, item);
                } else {
                    let mut tail = head;
                    while e.mem.u32(tail + 4) != 0 {
                        tail = e.mem.u32(tail + 4);
                    }
                    let node = e.mem.alloc(8);
                    e.mem.set_u32(node, item);
                    e.mem.set_u32(tail + 4, node);
                }
            }
            Ret::default()
        });
        e.register(LIST_DESTRUCT, |_, _| Ret::default());
        e.register(LIST_DELETE, |_, _| Ret::default());
        e.register(LIST_COUNT, |e, a| {
            let mut n = 0u32;
            let mut node = a[0];
            while node != 0 {
                if e.mem.u32(node) != 0 {
                    n += 1;
                }
                node = e.mem.u32(node + 4);
            }
            n.into_ret()
        });
    }

    /// Nodes holding `items`; returns the first node (0 for none).
    fn list_of(e: &mut Engine, items: &[u32]) -> u32 {
        let mut next = 0;
        for item in items.iter().rev() {
            let node = e.mem.alloc(8);
            e.mem.set_u32(node, *item);
            e.mem.set_u32(node + 4, next);
            next = node;
        }
        next
    }

    /// An object whose vtable (in the heap) has the given `(byte offset,
    /// function)` slots.
    fn object_with(e: &mut Engine, slots: &[(u32, u32)]) -> u32 {
        let vtable = e.mem.alloc(0x400);
        for (offset, function) in slots {
            e.mem.set_u32(vtable + offset, *function);
        }
        let object = e.mem.alloc(0x100);
        e.mem.set_u32(object, vtable);
        object
    }

    /// The standard eight words with `this_obj` set.
    fn script(this_obj: u32) -> ScriptArgs {
        ScriptArgs {
            param_info: 1,
            script_data: 2,
            this_obj: Ptr::new(this_obj),
            containing_obj: Ptr::NULL,
            script_obj: Ptr::new(5),
            event_list: 6,
            result: Ptr::NULL,
            opcode_offset: 8,
        }
    }

    /// `ParseParameters` double: returns `ok` and stores `outs` through its
    /// output pointers (after the seven fixed words).
    fn parse_gives(e: &mut Engine, ok: bool, outs: &[u32]) {
        let outs = outs.to_vec();
        e.register_double(PARSE_PARAMETERS, move |e, a| {
            for (i, value) in outs.iter().enumerate() {
                e.mem.set_u32(a[7 + i], *value);
            }
            ok.into_ret()
        });
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
            .filter(|(a, _)| *a == addr)
            .map(|(_, words)| words.clone())
            .collect()
    }

    /// The console prints, as argument words.
    fn printed(e: &Engine) -> Vec<Vec<u32>> {
        calls(e, CONSOLE_PRINT)
    }

    /// Registers doubles that just accept the call.
    fn accept(e: &mut Engine, addrs: &[u32]) {
        for addr in addrs {
            e.register(*addr, |_, _| Ret::default());
        }
    }

    #[test]
    fn script_args_round_trip_through_the_uniform_form() {
        let a = ScriptArgs {
            result: Ptr::new(0x77),
            containing_obj: Ptr::new(0x66),
            ..script(0x55)
        };
        let words = args![a];
        assert_eq!(words, vec![1, 2, 0x55, 0x66, 5, 6, 0x77, 8]);
        let back = ScriptArgs::take(&words, &mut 0);
        assert_eq!(back.this_obj, Ptr::new(0x55));
        assert_eq!(back.result, Ptr::new(0x77));
        assert_eq!(back.opcode_offset, 8);
    }

    #[test]
    fn get_distance_passes_the_parsed_object_to_the_condition_function() {
        let mut e = engine();
        parse_gives(&mut e, true, &[0x77]);
        e.register(GET_DISTANCE_CONDITION, |_, _| true.into_ret());
        start_log(&mut e);
        let a = ScriptArgs {
            result: Ptr::new(0x1234),
            ..script(0x40)
        };
        assert!(e.call(0x005b_4b20, &args![a]).bool());
        // ParseParameters gets: info, data, opcode offset, thisObj, containing,
        // script, event list, then the address of the local.
        assert_eq!(
            calls(&e, PARSE_PARAMETERS)[0][..7],
            [1, 2, 8, 0x40, 0, 5, 6]
        );
        assert_eq!(
            calls(&e, GET_DISTANCE_CONDITION),
            vec![vec![0x40, 0x77, 0, 0x1234]]
        );
        // Parameters that do not parse: false, nothing called.
        parse_gives(&mut e, false, &[]);
        start_log(&mut e);
        assert!(!e.call(0x005b_4b20, &args![a]).bool());
        assert!(calls(&e, GET_DISTANCE_CONDITION).is_empty());
    }

    #[test]
    fn get_in_zone_passes_the_parsed_object_to_the_condition_function() {
        let mut e = engine();
        parse_gives(&mut e, true, &[0x78]);
        e.register(GET_IN_ZONE_CONDITION, |_, _| false.into_ret());
        start_log(&mut e);
        let a = ScriptArgs {
            result: Ptr::new(0x1234),
            ..script(0x41)
        };
        assert!(!e.call(0x005b_4b80, &args![a]).bool());
        assert_eq!(
            calls(&e, GET_IN_ZONE_CONDITION),
            vec![vec![0x41, 0x78, 0, 0x1234]]
        );
        parse_gives(&mut e, false, &[]);
        start_log(&mut e);
        assert!(!e.call(0x005b_4b80, &args![a]).bool());
        assert!(calls(&e, GET_IN_ZONE_CONDITION).is_empty());
    }

    /// Doubles for the callees of `005b4be0`.
    fn add_item_engine() -> Engine {
        let mut e = engine();
        e.register(GET_CALC_LEVEL, |_, _| 0x0001_0007u32.into_ret());
        accept(
            &mut e,
            &[
                TES_CONTAINER_CONSTRUCT,
                TES_CONTAINER_DESTRUCT,
                FN_00487F70,
                FN_004818E0,
                FN_00482090,
                FN_004821A0,
                FN_00606540,
            ],
        );
        e.register(FN_00446390, |_, _| 5u32.into_ret());
        e.set_global(ADD_ITEM_FLOAT, 3.5f32);
        e
    }

    #[test]
    fn add_item_rolls_a_leveled_list_at_the_references_level() {
        let mut e = add_item_engine();
        let bound = object_with(&mut e, &[(0xe4, V_TRUE)]);
        e.mem.set_u8(bound + 4, 0x34);
        // An actor (slots 0x100 and 0x218) that is not the player, with
        // base form 0xba5e and slot 0x1e8 answering 9.
        let actor = object_with(&mut e, &[(0x100, V_TRUE), (0x218, V_TRUE), (0x1e8, V_NINE)]);
        e.mem.set_u32(actor + 0x20, 0xba5e);
        e.set_global(PLAYER, 0x4444u32);
        parse_gives(&mut e, true, &[bound, 0x0001_0003, 1]);
        start_log(&mut e);
        assert!(e.call(0x005b_4be0, &args![script(actor)]).bool());
        let container = calls(&e, TES_CONTAINER_CONSTRUCT)[0][0];
        // Level and count are the low 16 bits; the roll happens at +0x30.
        assert_eq!(
            calls(&e, FN_00487F70),
            vec![vec![bound + 0x30, 7, 3, container, 0]]
        );
        assert!(calls(&e, FN_004818E0).is_empty());
        assert_eq!(calls(&e, FN_004821A0), vec![vec![container, actor, 1]]);
        assert_eq!(calls(&e, FN_00606540), vec![vec![0xba5e, actor, 9, 1]]);
        assert_eq!(calls(&e, TES_CONTAINER_DESTRUCT), vec![vec![container]]);
    }

    #[test]
    fn add_item_adds_a_single_item_and_the_player_gets_no_equip_call() {
        let mut e = add_item_engine();
        let item = object_with(&mut e, &[(0xe4, V_TRUE)]);
        e.mem.set_u8(item + 4, 0x28);
        let player = object_with(&mut e, &[(0x100, V_TRUE), (0x218, V_TRUE), (0x1e8, V_NINE)]);
        e.set_global(PLAYER, player);
        parse_gives(&mut e, true, &[item, 4, 0]);
        start_log(&mut e);
        assert!(e.call(0x005b_4be0, &args![script(player)]).bool());
        let container = calls(&e, TES_CONTAINER_CONSTRUCT)[0][0];
        assert_eq!(calls(&e, FN_004818E0), vec![vec![container, item, 4, 0]]);
        // Type 0x28 with a small signed byte at +0xf4 calls 00482090.
        assert_eq!(
            calls(&e, FN_00482090),
            vec![vec![container, 3.5f32.to_bits()]]
        );
        // The flag was 0: the container is handed over with `false`.
        assert_eq!(calls(&e, FN_004821A0), vec![vec![container, player, 0]]);
        assert!(calls(&e, FN_00606540).is_empty());
    }

    #[test]
    fn add_item_walks_a_form_list_and_reports_failure() {
        let mut e = add_item_engine();
        let good = object_with(&mut e, &[(0xe4, V_TRUE)]);
        let bad = object_with(&mut e, &[(0xe4, V_FALSE)]);
        let list = object_with(&mut e, &[(0xe4, V_FALSE)]);
        e.mem.set_u8(list + 4, 0x55);
        let nodes = list_of(&mut e, &[bad, good]);
        e.register_double(FORM_LIST_ITEMS, move |_, a| {
            assert_eq!(a[0], list);
            nodes.into_ret()
        });
        let this_obj = object_with(&mut e, &[(0x100, V_FALSE)]);
        parse_gives(&mut e, true, &[list, 2, 0]);
        start_log(&mut e);
        assert!(e.call(0x005b_4be0, &args![script(this_obj)]).bool());
        let container = calls(&e, TES_CONTAINER_CONSTRUCT)[0][0];
        // Only the element that passes slot 0xe4 is added.
        assert_eq!(calls(&e, FN_004818E0), vec![vec![container, good, 2, 0]]);

        // A plain form that does not pass slot 0xe4: nothing is added and the
        // script's name (slot 0x130) is fetched for the failure message.
        let script_obj = object_with(&mut e, &[(0x130, V_RECORD)]);
        e.mem.set_u8(bad + 4, 0x2a);
        parse_gives(&mut e, true, &[bad, 1, 0]);
        start_log(&mut e);
        let a = ScriptArgs {
            script_obj: Ptr::new(script_obj),
            ..script(this_obj)
        };
        assert!(e.call(0x005b_4be0, &args![a]).bool());
        assert!(calls(&e, FN_004818E0).is_empty());
        assert_eq!(calls(&e, V_RECORD), vec![vec![script_obj]]);

        // No reference: nothing happens; bad parameters: false.
        start_log(&mut e);
        assert!(e.call(0x005b_4be0, &args![script(0)]).bool());
        assert!(calls(&e, TES_CONTAINER_CONSTRUCT).is_empty());
        parse_gives(&mut e, false, &[]);
        assert!(!e.call(0x005b_4be0, &args![script(this_obj)]).bool());
    }

    /// Doubles for the callees of `005b4e90`. By convention of these tests,
    /// byte `+0x50` of a reference marks an actor (the RTTI cast succeeds),
    /// `+0x58` is the `double` the actor carries of every item, and, on an
    /// item, byte `+0x51` says the actor can wear it and `+0x52` that a worn
    /// entry exists.
    fn remove_item_engine() -> Engine {
        let mut e = engine();
        e.register(DYNAMIC_CAST, |e, a| {
            let actor = if e.mem.u8(a[0] + 0x50) != 0 { a[0] } else { 0 };
            actor.into_ret()
        });
        e.register(GET_ITEM_COUNT_CONDITION, |e, a| {
            let carried = e.mem.f64(a[0] + 0x58);
            e.mem.set_f64(a[3], carried);
            Ret::default()
        });
        e.register(FTOL, |_, a| (f64::take(a, &mut 0) as i32 as u32).into_ret());
        e.register(FN_00575400, |e, a| (e.mem.u8(a[1] + 0x51) != 0).into_ret());
        e.register(GET_INVENTORY_CHANGES, |_, _| 0x5150u32.into_ret());
        e.register(WEARING_OBJECT, |e, a| {
            let worn = if e.mem.u8(a[1] + 0x52) != 0 {
                0x7777
            } else {
                0
            };
            (worn as u32).into_ret()
        });
        e.register(GET_CONTAINER_CHANGES, |_, _| 0xc0deu32.into_ret());
        e.register(FN_004CBA70, |_, _| 0x77u32.into_ret());
        e.register(BS_STRING_TEXT, |_, a| (a[0] + 1).into_ret());
        e.register(GET_FULL_NAME, |_, _| 0xaaaau32.into_ret());
        e.register(BS_STRING_FORMAT, |e, a| {
            e.mem.set_u32(a[0], 0x5151);
            Ret::default()
        });
        e.register(GET_PICK_UP_SOUND_NAME, |_, _| 0x50du32.into_ret());
        e.set_global(PICKUP_VOLUME, 2.0f32);
        accept(
            &mut e,
            &[
                SET_CAN_NOT_WEAR,
                FN_008248E0,
                FN_00704AF0,
                BS_STRING_CONSTRUCT,
                BS_STRING_DESTRUCT,
                SHOW_MESSAGE,
            ],
        );
        e
    }

    #[test]
    fn remove_item_from_a_plain_reference_removes_the_form_as_is() {
        let mut e = remove_item_engine();
        let item = object_with(&mut e, &[(0xe4, V_TRUE)]);
        let this_obj = object_with(&mut e, &[(0x17c, V_RECORD)]);
        parse_gives(&mut e, true, &[item, 3, 0]);
        start_log(&mut e);
        assert!(e.call(0x005b_4e90, &args![script(this_obj)]).bool());
        assert_eq!(
            calls(&e, V_RECORD),
            vec![vec![this_obj, item, 0, 3, 0, 0, 0, 0, 0, 1, 0]]
        );
        assert_eq!(calls(&e, LIST_DESTRUCT).len(), 1);
        // No reference or no form: nothing; bad parameters: false.
        start_log(&mut e);
        assert!(e.call(0x005b_4e90, &args![script(0)]).bool());
        parse_gives(&mut e, true, &[0, 3, 0]);
        assert!(e.call(0x005b_4e90, &args![script(this_obj)]).bool());
        assert!(calls(&e, LIST_CONSTRUCT).is_empty());
        parse_gives(&mut e, false, &[]);
        assert!(!e.call(0x005b_4e90, &args![script(this_obj)]).bool());
    }

    #[test]
    fn remove_item_from_the_player_caps_the_count_and_tells_the_player() {
        let mut e = remove_item_engine();
        let item = object_with(&mut e, &[(0xe4, V_TRUE)]);
        e.mem.set_u8(item + 4, 0x18);
        e.mem.set_u8(item + 0x51, 1);
        e.mem.set_u8(item + 0x52, 1);
        let player = object_with(&mut e, &[(0x17c, V_RECORD)]);
        e.mem.set_u8(player + 0x50, 1);
        e.mem.set_f64(player + 0x58, 2.0);
        e.set_global(PLAYER, player);
        parse_gives(&mut e, true, &[item, 5, 0]);
        start_log(&mut e);
        assert!(e.call(0x005b_4e90, &args![script(player)]).bool());
        // The player carries 2 of the 5 asked for.
        assert_eq!(calls(&e, FTOL).len(), 1);
        assert_eq!(calls(&e, SET_CAN_NOT_WEAR), vec![vec![0x7777, 0]]);
        assert_eq!(calls(&e, FN_008248E0), vec![vec![player + 0x94, item, 1]]);
        assert_eq!(
            calls(&e, V_RECORD),
            vec![vec![player, item, 0x7777, 2, 0, 0, 0, 0, 0, 1, 0]]
        );
        let text = calls(&e, BS_STRING_CONSTRUCT)[0][0];
        // "<count> <name><b> <a>" for several items.
        assert_eq!(
            calls(&e, BS_STRING_FORMAT),
            vec![vec![
                text,
                FORMAT_ITEM_COUNT_MESSAGE,
                2,
                0xaaaa,
                MESSAGE_PART_B + 1,
                MESSAGE_PART_A + 1
            ]]
        );
        // Form type 0x18 shows the message with the icon and the pick-up
        // sound.
        assert_eq!(
            calls(&e, SHOW_MESSAGE),
            vec![vec![0x5151, 0, ICON_VAULT_BOY, 0x50d, 2.0f32.to_bits(), 0]]
        );
        assert_eq!(calls(&e, FN_00704AF0).len(), 1);

        // Silenced: no message, the interface refresh still happens.
        parse_gives(&mut e, true, &[item, 1, 1]);
        e.mem.set_f64(player + 0x58, 5.0);
        start_log(&mut e);
        assert!(e.call(0x005b_4e90, &args![script(player)]).bool());
        assert!(calls(&e, BS_STRING_FORMAT).is_empty());
        assert_eq!(calls(&e, FN_00704AF0).len(), 1);

        // One item: "<name> <a>".
        parse_gives(&mut e, true, &[item, 1, 0]);
        start_log(&mut e);
        assert!(e.call(0x005b_4e90, &args![script(player)]).bool());
        let text = calls(&e, BS_STRING_CONSTRUCT)[0][0];
        assert_eq!(
            calls(&e, BS_STRING_FORMAT),
            vec![vec![text, FORMAT_ITEM_MESSAGE, 0xaaaa, MESSAGE_PART_A + 1]]
        );

        // A type the switch does not know: the message is built but not shown.
        e.mem.set_u8(item + 4, 0x2a);
        start_log(&mut e);
        assert!(e.call(0x005b_4e90, &args![script(player)]).bool());
        assert!(calls(&e, SHOW_MESSAGE).is_empty());
    }

    #[test]
    fn remove_item_skips_entries_the_actor_does_not_carry() {
        let mut e = remove_item_engine();
        let item = object_with(&mut e, &[(0xe4, V_TRUE)]);
        let other = object_with(&mut e, &[(0x17c, V_RECORD)]);
        e.mem.set_u8(other + 0x50, 1);
        e.mem.set_f64(other + 0x58, 0.0);
        parse_gives(&mut e, true, &[item, 3, 0]);
        start_log(&mut e);
        assert!(e.call(0x005b_4e90, &args![script(other)]).bool());
        // Carrying none: the count becomes 0 and nothing is removed.
        assert!(calls(&e, V_RECORD).is_empty());
    }

    #[test]
    fn remove_item_from_another_actor_lets_a_wearer_drop_the_acquire_object() {
        let mut e = remove_item_engine();
        let item = object_with(&mut e, &[(0xe4, V_TRUE)]);
        e.mem.set_u8(item + 0x51, 1);
        e.set_global(PLAYER, 0x4444u32);
        let process = object_with(&mut e, &[(0xc4, V_RECORD)]);
        e.register_double(MIDDLE_HIGH_PROCESS_SAVED_ACQUIRE, move |_, _| {
            process.into_ret()
        });
        let actor = object_with(&mut e, &[(0x17c, V_FALSE), (0x1d0, V_NINE)]);
        e.mem.set_u8(actor + 0x50, 1);
        e.mem.set_f64(actor + 0x58, 10.0);
        parse_gives(&mut e, true, &[item, 3, 0]);
        start_log(&mut e);
        assert!(e.call(0x005b_4e90, &args![script(actor)]).bool());
        assert_eq!(calls(&e, V_RECORD), vec![vec![process, actor, 0]]);
        assert!(calls(&e, FN_00704AF0).is_empty());
    }

    #[test]
    fn remove_item_resolves_leveled_entries_and_form_lists() {
        let mut e = remove_item_engine();
        let this_obj = object_with(&mut e, &[(0x17c, V_RECORD)]);
        // Type 0x34: the entry comes from the inventory changes (0x77).
        let leveled = object_with(&mut e, &[(0xe4, V_FALSE)]);
        e.mem.set_u8(leveled + 4, 0x34);
        parse_gives(&mut e, true, &[leveled, 2, 0]);
        start_log(&mut e);
        assert!(e.call(0x005b_4e90, &args![script(this_obj)]).bool());
        assert_eq!(calls(&e, FN_004CBA70), vec![vec![0xc0de, leveled]]);
        assert_eq!(
            calls(&e, V_RECORD),
            vec![vec![this_obj, 0x77, 0, 2, 0, 0, 0, 0, 0, 1, 0]]
        );
        // Type 0x55: every element passing slot 0xe4 is removed, in order.
        let a = object_with(&mut e, &[(0xe4, V_TRUE)]);
        let b = object_with(&mut e, &[(0xe4, V_FALSE)]);
        let c = object_with(&mut e, &[(0xe4, V_TRUE)]);
        let nodes = list_of(&mut e, &[a, b, c]);
        e.register_double(FORM_LIST_ITEMS, move |_, _| nodes.into_ret());
        let list = object_with(&mut e, &[(0xe4, V_FALSE)]);
        e.mem.set_u8(list + 4, 0x55);
        parse_gives(&mut e, true, &[list, 1, 0]);
        start_log(&mut e);
        assert!(e.call(0x005b_4e90, &args![script(this_obj)]).bool());
        let removed: Vec<u32> = calls(&e, V_RECORD).iter().map(|w| w[1]).collect();
        assert_eq!(removed, vec![a, c]);
    }

    #[test]
    fn transfer_gives_the_target_the_base_form_and_the_worn_entry() {
        let mut e = engine();
        e.register(DYNAMIC_CAST, |_, a| a[0].into_ret());
        e.register(FN_00575400, |_, _| true.into_ret());
        e.register(GET_INVENTORY_CHANGES, |_, _| 0x5150u32.into_ret());
        e.register(WEARING_OBJECT, |_, _| 0x7777u32.into_ret());
        accept(&mut e, &[CHECK_QUEST_TARGET_UPDATE, FN_00704AF0]);
        let this_obj = object_with(&mut e, &[]);
        e.mem.set_u32(this_obj + 0x20, 0xba5e);
        let target = object_with(&mut e, &[(0x17c, V_RECORD)]);
        e.set_global(PLAYER, target);
        parse_gives(&mut e, true, &[9]);
        start_log(&mut e);
        let a = ScriptArgs {
            containing_obj: Ptr::new(target),
            ..script(this_obj)
        };
        assert!(!e.call(0x005b_53d0, &args![a]).bool());
        assert_eq!(calls(&e, WEARING_OBJECT), vec![vec![0x5150, 0xba5e, 0]]);
        assert_eq!(
            calls(&e, V_RECORD),
            vec![vec![target, 0xba5e, 0x7777, 1, 0, 0, 9, 0, 0, 1, 0]]
        );
        assert_eq!(
            calls(&e, CHECK_QUEST_TARGET_UPDATE),
            vec![vec![target, this_obj]]
        );
        // The target is the player: the interface is refreshed.
        assert_eq!(calls(&e, FN_00704AF0).len(), 1);

        // Not an actor, not the player: no worn entry, no refresh.
        e.register(DYNAMIC_CAST, |_, _| 0u32.into_ret());
        e.set_global(PLAYER, 0x4444u32);
        start_log(&mut e);
        assert!(!e.call(0x005b_53d0, &args![a]).bool());
        assert!(calls(&e, WEARING_OBJECT).is_empty());
        assert_eq!(calls(&e, V_RECORD)[0][2], 0);
        assert!(calls(&e, FN_00704AF0).is_empty());

        // Either reference missing: true and nothing else.
        start_log(&mut e);
        assert!(e.call(0x005b_53d0, &args![script(this_obj)]).bool());
        assert!(calls(&e, V_RECORD).is_empty());
        parse_gives(&mut e, false, &[]);
        assert!(!e.call(0x005b_53d0, &args![a]).bool());
    }

    #[test]
    fn duplicate_all_items_needs_the_inventory_changes() {
        let mut e = engine();
        e.register(GET_CONTAINER_CHANGES, |_, a| (a[0] + 1).into_ret());
        accept(&mut e, &[DUPLICATE_ALL_ITEMS]);
        parse_gives(&mut e, true, &[0x99, 1]);
        start_log(&mut e);
        assert!(e.call(0x005b_5500, &args![script(0x1000)]).bool());
        // The extra data list is at +0x44; the changes come from it.
        assert_eq!(
            calls(&e, DUPLICATE_ALL_ITEMS),
            vec![vec![0x1045, 0x1000, 0x99]]
        );
        // No changes: nothing is duplicated.
        e.register(GET_CONTAINER_CHANGES, |_, _| 0u32.into_ret());
        start_log(&mut e);
        assert!(e.call(0x005b_5500, &args![script(0x1000)]).bool());
        assert!(calls(&e, DUPLICATE_ALL_ITEMS).is_empty());
        // No reference: true; bad parameters: false.
        assert!(e.call(0x005b_5500, &args![script(0)]).bool());
        parse_gives(&mut e, false, &[]);
        assert!(!e.call(0x005b_5500, &args![script(0x1000)]).bool());
    }

    #[test]
    fn inventory_changes_commands_pass_their_arguments_and_defaults() {
        let mut e = engine();
        e.register(GET_CONTAINER_CHANGES, |_, a| (a[0] + 1).into_ret());
        accept(
            &mut e,
            &[FN_004CE340, FN_004CE380, CHECK_QUEST_TARGET_UPDATE],
        );
        e.set_global(PLAYER, 0x4444u32);
        parse_gives(&mut e, true, &[7, 1, 2, 3, 4]);
        start_log(&mut e);
        assert!(e.call(0x005b_55a0, &args![script(0x1000)]).bool());
        assert_eq!(
            calls(&e, FN_004CE340),
            vec![vec![0x1045, 0x1000, 7, 0, 1, 0, 1, 3, 4]]
        );
        assert_eq!(
            calls(&e, CHECK_QUEST_TARGET_UPDATE),
            vec![vec![0x4444, 0x1000]]
        );
        // Nothing parsed into the locals: their initial values (slot -1).
        parse_gives(&mut e, true, &[]);
        start_log(&mut e);
        assert!(e.call(0x005b_55a0, &args![script(0x1000)]).bool());
        assert_eq!(
            calls(&e, FN_004CE340),
            vec![vec![0x1045, 0x1000, 0, 0, 0, 0, 0, 0xffff_ffff, 0]]
        );

        // The second command has a fixed slot of -1.
        parse_gives(&mut e, true, &[7, 1, 2]);
        start_log(&mut e);
        assert!(e.call(0x005b_5690, &args![script(0x1000)]).bool());
        assert_eq!(
            calls(&e, FN_004CE380),
            vec![vec![0x1045, 0x1000, 7, 0, 1, 0, 1, 0xffff_ffff, 0]]
        );
        assert_eq!(calls(&e, CHECK_QUEST_TARGET_UPDATE).len(), 1);
        // No inventory changes: nothing is called.
        e.register(GET_CONTAINER_CHANGES, |_, _| 0u32.into_ret());
        start_log(&mut e);
        assert!(e.call(0x005b_5690, &args![script(0x1000)]).bool());
        assert!(e.call(0x005b_55a0, &args![script(0x1000)]).bool());
        assert!(calls(&e, FN_004CE380).is_empty());
        assert!(calls(&e, FN_004CE340).is_empty());
        parse_gives(&mut e, false, &[]);
        assert!(!e.call(0x005b_5690, &args![script(0x1000)]).bool());
        assert!(!e.call(0x005b_55a0, &args![script(0x1000)]).bool());
    }

    #[test]
    fn form_parameter_commands_default_to_the_players_base_form() {
        let mut e = engine();
        let player = e.mem.alloc(0x100);
        e.mem.set_u32(player + 0x20, 0xba5e);
        e.set_global(PLAYER, player);
        accept(&mut e, &[FN_005A2E20, FN_00419700]);
        // 005b5760: the result is cleared first, even when the parse fails.
        let result = e.mem.alloc(8);
        e.mem.set_f64(result, 5.0);
        parse_gives(&mut e, false, &[]);
        let a = ScriptArgs {
            result: Ptr::new(result),
            ..script(0x1000)
        };
        assert!(!e.call(0x005b_5760, &args![a]).bool());
        assert_eq!(e.mem.f64(result), 0.0);
        parse_gives(&mut e, true, &[0]);
        start_log(&mut e);
        assert!(e.call(0x005b_5760, &args![a]).bool());
        assert_eq!(
            calls(&e, FN_005A2E20),
            vec![vec![0x1000, 0xba5e, 0, result]]
        );
        parse_gives(&mut e, true, &[0x33]);
        start_log(&mut e);
        assert!(e.call(0x005b_5760, &args![a]).bool());
        assert_eq!(calls(&e, FN_005A2E20)[0][1], 0x33);

        // 005b57e0: extra data list (+0x44) gets the form, the reference
        // slot 0x48 gets 0x40.
        let this_obj = object_with(&mut e, &[(0x48, V_RECORD)]);
        parse_gives(&mut e, true, &[0]);
        start_log(&mut e);
        assert!(e.call(0x005b_57e0, &args![script(this_obj)]).bool());
        assert_eq!(calls(&e, FN_00419700), vec![vec![this_obj + 0x44, 0xba5e]]);
        assert_eq!(calls(&e, V_RECORD), vec![vec![this_obj, 0x40]]);
        // No reference: only the default is looked up.
        start_log(&mut e);
        assert!(e.call(0x005b_57e0, &args![script(0)]).bool());
        assert!(calls(&e, FN_00419700).is_empty());
        parse_gives(&mut e, false, &[]);
        assert!(!e.call(0x005b_57e0, &args![script(this_obj)]).bool());
    }

    #[test]
    fn give_base_form_to_the_other_reference_refreshes_quest_targets() {
        let mut e = engine();
        accept(&mut e, &[CHECK_QUEST_TARGET_UPDATE, FN_00704AF0]);
        e.register(IS_IN_MENU_MODE, |_, _| true.into_ret());
        e.set_global(PLAYER, 0x4444u32);
        let this_obj = object_with(&mut e, &[]);
        e.mem.set_u32(this_obj + 0x20, 0xba5e);
        let target = object_with(&mut e, &[(0x17c, V_RECORD)]);
        let a = ScriptArgs {
            containing_obj: Ptr::new(target),
            ..script(this_obj)
        };
        start_log(&mut e);
        assert!(!e.call(0x005b_5860, &args![a]).bool());
        assert_eq!(
            calls(&e, V_RECORD),
            vec![vec![target, 0xba5e, 0, 1, 0, 1, 0, 0, 0, 1, 0]]
        );
        assert_eq!(
            calls(&e, CHECK_QUEST_TARGET_UPDATE),
            vec![vec![0x4444, this_obj]]
        );
        assert_eq!(calls(&e, FN_00704AF0).len(), 1);
        // Outside menu mode there is no refresh.
        e.register(IS_IN_MENU_MODE, |_, _| false.into_ret());
        start_log(&mut e);
        assert!(!e.call(0x005b_5860, &args![a]).bool());
        assert!(calls(&e, FN_00704AF0).is_empty());
        // A missing reference: true, nothing done.
        start_log(&mut e);
        assert!(e.call(0x005b_5860, &args![script(this_obj)]).bool());
        assert!(calls(&e, V_RECORD).is_empty());
    }

    #[test]
    fn form_and_count_command_forwards_to_slot_0x17c() {
        let mut e = engine();
        let this_obj = object_with(&mut e, &[(0x17c, V_RECORD)]);
        parse_gives(&mut e, true, &[3, 4]);
        start_log(&mut e);
        assert!(e.call(0x005b_58d0, &args![script(this_obj)]).bool());
        assert_eq!(
            calls(&e, V_RECORD),
            vec![vec![this_obj, 3, 0, 4, 0, 1, 0, 0, 0, 1, 0]]
        );
        start_log(&mut e);
        assert!(e.call(0x005b_58d0, &args![script(0)]).bool());
        assert!(calls(&e, V_RECORD).is_empty());
        parse_gives(&mut e, false, &[]);
        assert!(!e.call(0x005b_58d0, &args![script(this_obj)]).bool());
    }

    #[test]
    fn is_action_ref_compares_with_the_action_reference_and_echoes() {
        let mut e = engine();
        e.register(GET_ACTION_REF, |_, _| 0x55u32.into_ret());
        let result = e.mem.alloc(8);
        let a = ScriptArgs {
            result: Ptr::new(result),
            ..script(0x1000)
        };
        parse_gives(&mut e, true, &[0x55]);
        start_log(&mut e);
        assert!(e.call(0x005b_5950, &args![a]).bool());
        assert_eq!(e.mem.f64(result), 1.0);
        // The echo flag (TLS +0x268) is off: no print.
        assert!(printed(&e).is_empty());
        parse_gives(&mut e, true, &[0x56]);
        assert!(e.call(0x005b_5950, &args![a]).bool());
        assert_eq!(e.mem.f64(result), 0.0);
        // With the flag set the value is printed as a `double`.
        let tls = e.tls();
        e.mem.set_u8(tls + TLS_ECHO, 1);
        accept(&mut e, &[CONSOLE_PRINT]);
        parse_gives(&mut e, true, &[0x55]);
        start_log(&mut e);
        assert!(e.call(0x005b_5950, &args![a]).bool());
        assert_eq!(printed(&e), vec![args![MSG_IS_ACTION_REF, 1.0f64]]);
        // No reference: the result stays 0.
        assert!(e
            .call(
                0x005b_5950,
                &args![ScriptArgs {
                    this_obj: Ptr::NULL,
                    ..a
                }]
            )
            .bool());
        assert_eq!(e.mem.f64(result), 0.0);
        parse_gives(&mut e, false, &[]);
        assert!(!e.call(0x005b_5950, &args![a]).bool());
    }

    #[test]
    fn activate_runs_with_the_action_flags_and_guards_against_nesting() {
        let mut e = engine();
        accept(&mut e, &[SET_ACTION, CLEAR_ACTION, ACTIVATE]);
        e.register(GET_ACTION_REF, |_, _| 0x55u32.into_ret());
        e.register(FN_00440DA0, |_, _| false.into_ret());
        e.register(HAS_ACTION, |_, _| false.into_ret());
        e.set_global(PLAYER, 0x4444u32);
        let tls = e.tls();
        // No explicit target, second argument 0: the default target, with
        // actions 1 and 2 around the activation.
        parse_gives(&mut e, true, &[0, 0]);
        start_log(&mut e);
        assert!(e.call(0x005b_59f0, &args![script(0x1000)]).bool());
        let order: Vec<(u32, u32)> = e
            .call_log
            .as_ref()
            .unwrap()
            .iter()
            .filter(|(a, _)| [SET_ACTION, ACTIVATE, CLEAR_ACTION].contains(a))
            .map(|(a, w)| (*a, w[1]))
            .collect();
        assert_eq!(
            order,
            vec![
                (SET_ACTION, 1),
                (SET_ACTION, 2),
                (ACTIVATE, 0x55),
                (CLEAR_ACTION, 2),
                (CLEAR_ACTION, 1)
            ]
        );
        assert_eq!(calls(&e, ACTIVATE), vec![vec![0x1000, 0x55, 0, 0, 1]]);
        // The nesting counter is back to 0.
        assert_eq!(e.mem.u32(tls + TLS_ACTIVATE_DEPTH), 0);

        // With the second argument non-zero: just the activation.
        parse_gives(&mut e, true, &[0x66, 1]);
        start_log(&mut e);
        assert!(e.call(0x005b_59f0, &args![script(0x1000)]).bool());
        assert_eq!(calls(&e, ACTIVATE), vec![vec![0x1000, 0x66, 0, 0, 1]]);
        assert!(calls(&e, SET_ACTION).is_empty());

        // A flagged target is left alone.
        e.register(FN_00440DA0, |_, _| true.into_ret());
        start_log(&mut e);
        assert!(e.call(0x005b_59f0, &args![script(0x1000)]).bool());
        assert!(calls(&e, ACTIVATE).is_empty());
        assert_eq!(e.mem.u32(tls + TLS_ACTIVATE_DEPTH), 0);

        // Nested deeper than the limit: succeeds without doing anything and
        // without touching the counter.
        e.register(FN_00440DA0, |_, _| false.into_ret());
        e.mem.set_u32(tls + TLS_ACTIVATE_DEPTH, 6);
        start_log(&mut e);
        assert!(e.call(0x005b_59f0, &args![script(0x1000)]).bool());
        assert!(calls(&e, ACTIVATE).is_empty());
        assert_eq!(e.mem.u32(tls + TLS_ACTIVATE_DEPTH), 6);
        e.mem.set_u32(tls + TLS_ACTIVATE_DEPTH, 5);
        start_log(&mut e);
        assert!(e.call(0x005b_59f0, &args![script(0x1000)]).bool());
        assert_eq!(calls(&e, ACTIVATE).len(), 1);
        assert_eq!(e.mem.u32(tls + TLS_ACTIVATE_DEPTH), 5);

        // The player may not run it; a missing reference is fine.
        e.set_global(PLAYER, 0x1000u32);
        start_log(&mut e);
        assert!(!e.call(0x005b_59f0, &args![script(0x1000)]).bool());
        assert!(calls(&e, PARSE_PARAMETERS).is_empty());
        e.set_global(PLAYER, 0x4444u32);
        assert!(e.call(0x005b_59f0, &args![script(0)]).bool());
        parse_gives(&mut e, false, &[]);
        assert!(!e.call(0x005b_59f0, &args![script(0x1000)]).bool());
    }

    /// Runs the no-argument command at `addr` and returns its console prints.
    fn run_and_print(e: &mut Engine, addr: u32) -> Vec<Vec<u32>> {
        accept(e, &[CONSOLE_PRINT]);
        start_log(e);
        assert!(e.call(addr, &args![]).bool());
        printed(e)
    }

    #[test]
    fn message_only_commands_print_their_message() {
        let mut e = engine();
        for (addr, message) in [
            (0x005b_5b90, MSG_COMBAT_STATS_REMOVED),
            (0x005b_5db0, MSG_FREEZE_RENDERER_DEPRECATED),
            (0x005b_5e20, MSG_MEM_CONTEXTS_UNSUPPORTED),
        ] {
            assert_eq!(run_and_print(&mut e, addr), vec![vec![message]]);
        }
    }

    #[test]
    fn spell_book_counts_the_spells_the_player_takes() {
        let mut e = engine();
        // Entries with the type their +0x18 member reports through slot 0x18
        // (0, 1, 2 and 3), and a null entry.
        let mut entries = vec![];
        for kind in [0, 1, 2, 3, 3] {
            let entry = e.mem.alloc(0x40);
            let vtable = e.mem.alloc(0x40);
            e.mem.set_u32(vtable + 0x18, V_KIND + kind);
            e.mem.set_u32(entry + 0x18, vtable);
            entries.push(entry);
        }
        entries.insert(2, 0);
        let nodes = list_of(&mut e, &entries);
        e.set_global(DATA_HANDLER, 0x5000u32);
        e.register_double(DATA_HANDLER_SPELLS, move |_, a| {
            assert_eq!(a[0], 0x5000);
            nodes.into_ret()
        });
        // The player (slot 0x3e0) takes every spell but the last one.
        let add_spell = 0x0900_0020;
        let refused = entries[5];
        e.register_double(add_spell, move |_, a| (a[1] != refused).into_ret());
        let player = object_with(&mut e, &[(0x3e0, add_spell)]);
        e.set_global(PLAYER, player);
        accept(&mut e, &[CONSOLE_PRINT]);
        start_log(&mut e);
        assert!(e.call(0x005b_5bb0, &args![]).bool());
        // Asked: the kinds 0, 2, 3 and 3 entries; not the kind 1 or the null
        // entry.
        let asked: Vec<u32> = calls(&e, add_spell).iter().map(|w| w[1]).collect();
        assert_eq!(asked, vec![entries[0], entries[3], entries[4], refused]);
        // Three accepted of four asked.
        assert_eq!(printed(&e), vec![vec![MSG_SPELLS_ADDED, 3]]);
    }

    #[test]
    fn process_list_commands() {
        let mut e = engine();
        accept(&mut e, &[PROCESS_LISTS_PRINT_LISTS, FN_00483710]);
        assert_eq!(
            run_and_print(&mut e, 0x005b_5cc0),
            vec![vec![MSG_DETECTION_LIST_PRINTED]]
        );
        assert_eq!(
            calls(&e, PROCESS_LISTS_PRINT_LISTS),
            vec![vec![PROCESS_LISTS, 1, 0xffff_ffff]]
        );
        assert_eq!(
            run_and_print(&mut e, 0x005b_5cf0),
            vec![vec![MSG_AI_LISTS_PRINTED]]
        );
        assert_eq!(
            calls(&e, PROCESS_LISTS_PRINT_LISTS),
            vec![vec![PROCESS_LISTS, 0, 0xffff_ffff]]
        );
        start_log(&mut e);
        assert!(e.call(0x005b_5d20, &args![]).bool());
        assert_eq!(calls(&e, FN_00483710), vec![vec![PROCESS_LISTS]]);
    }

    #[test]
    fn flag_setting_commands() {
        let mut e = engine();
        parse_gives(&mut e, true, &[5]);
        assert!(e.call(0x005b_5d40, &args![script(0)]).bool());
        assert_eq!(e.global::<u8>(FLAG_0011F122D), 1);
        parse_gives(&mut e, true, &[0]);
        assert!(e.call(0x005b_5d40, &args![script(0)]).bool());
        assert_eq!(e.global::<u8>(FLAG_0011F122D), 0);
        // Bad parameters leave the flag alone and fail.
        e.set_global(FLAG_0011F122D, 1u8);
        parse_gives(&mut e, false, &[0]);
        assert!(!e.call(0x005b_5d40, &args![script(0)]).bool());
        assert_eq!(e.global::<u8>(FLAG_0011F122D), 1);

        assert!(e.call(0x005b_5da0, &args![]).bool());
        assert_eq!(e.global::<u8>(FLAG_0011F9FC0), 1);
    }

    #[test]
    fn toggle_occlusion_flips_and_prints_the_state() {
        let mut e = engine();
        assert_eq!(
            run_and_print(&mut e, 0x005b_5dd0),
            vec![vec![MSG_OCCLUSION_QUERY, TEXT_ON]]
        );
        assert_eq!(e.global::<u8>(OCCLUSION_QUERY), 1);
        assert_eq!(
            run_and_print(&mut e, 0x005b_5dd0),
            vec![vec![MSG_OCCLUSION_QUERY, TEXT_OFF]]
        );
        assert_eq!(e.global::<u8>(OCCLUSION_QUERY), 0);
    }

    #[test]
    fn the_logging_stub_returns_zero() {
        let mut e = engine();
        assert_eq!(e.call(0x005b_5e40, &args![1u32, 2u32]).u32(), 0);
    }

    /// Writes the C string `text` at `addr` (a page the engine mapped).
    fn put_str(e: &mut Engine, addr: u32, text: &str) {
        e.mem.set_cstr(addr, text.as_bytes());
    }

    /// Doubles for the string and file functions `OutputMemStats` and its
    /// relatives call: the C library ones behave like the real ones, the
    /// formatting ones only record their arguments.
    fn output_engine() -> Engine {
        let mut e = engine();
        e.register(MEMSET, |e, a| {
            for i in 0..a[2] {
                e.mem.set_u8(a[0] + i, a[1] as u8);
            }
            Ret::default()
        });
        e.register(STRCPY_S, |e, a| {
            let text = e.mem.cstr(a[2]);
            e.mem.set_cstr(a[0], &text);
            Ret::default()
        });
        e.register(STRNCPY_S, |e, a| {
            let text = e.mem.cstr(a[1]);
            e.mem.set_cstr(a[0], &text);
            Ret::default()
        });
        e.register(STRCAT_S, |e, a| {
            let mut text = e.mem.cstr(a[0]);
            text.extend(e.mem.cstr(a[2]));
            e.mem.set_cstr(a[0], &text);
            Ret::default()
        });
        e.register(SPRINTF_S, |_, _| Ret::default());
        e.register(STRLEN, |e, a| (e.mem.cstr(a[0]).len() as u32).into_ret());
        e.register(FIND_SUBSTRING, |e, a| {
            let haystack = e.mem.cstr(a[0]);
            let needle = e.mem.cstr(a[1]);
            let found = !needle.is_empty()
                && haystack
                    .windows(needle.len())
                    .any(|window| window == needle.as_slice());
            (found as u32).into_ret()
        });
        accept(
            &mut e,
            &[
                MEM_STATS_FILE_CONSTRUCT,
                MEM_STATS_FILE_DESTRUCT,
                MEM_STATS_OUTPUT_SET,
                BS_FILE_CONSTRUCT,
                BS_FILE_OPEN,
                BS_FILE_CLOSE,
                BS_FILE_DESTRUCT,
                BS_FILE_WRITE,
                MODEL_LOADER_OUTPUT_MODEL_MAP,
                FACE_GEN_OUTPUT_MODEL_MAP,
                VECTOR_CONSTRUCT,
                VECTOR_DESTRUCT,
                ZERO_FILL,
                FIX_PATH,
                CONSOLE_PRINT,
            ],
        );
        e.register(MEM_STATS_OUTPUT_GET, |_, _| 0x11f_6238u32.into_ret());
        e.register(GET_MODEL_CACHE, |_, _| 0u32.into_ret());
        e.register(FIRST_TEXTURE, |_, _| 0u32.into_ret());
        e
    }

    #[test]
    fn mem_stats_names_its_files_after_the_argument_and_dumps_the_model_maps() {
        let mut e = output_engine();
        // The argument is parsed into the path buffer (first output).
        e.register_double(PARSE_PARAMETERS, |e, a| {
            e.mem.set_cstr(a[7], b"Run1");
            true.into_ret()
        });
        e.set_global(MODEL_LOADER, 0x6000u32);
        e.register(GET_MODEL_CACHE, |_, _| 0x7000u32.into_ret());
        start_log(&mut e);
        assert!(e.call(0x005b_5e60, &args![script(0)]).bool());
        let formats: Vec<u32> = calls(&e, SPRINTF_S).iter().map(|w| w[2]).collect();
        assert_eq!(
            formats,
            vec![FORMAT_MEM_STATS, FORMAT_MODEL_DUMP, FORMAT_TEXTURE_DUMP]
        );
        // The name argument ("Run1") is copied aside and used in each
        // format; the heap dump object is configured with flag 0.
        let name = calls(&e, STRCPY_S)[0][0];
        assert!(calls(&e, SPRINTF_S).iter().all(|w| w[3] == name));
        let dump = calls(&e, MEM_STATS_FILE_CONSTRUCT)[0][0];
        assert_eq!(calls(&e, MEM_STATS_OUTPUT_GET), vec![vec![dump, 0]]);
        assert_eq!(calls(&e, MEM_STATS_OUTPUT_SET), vec![vec![0x11f_6238]]);
        let model_file = calls(&e, BS_FILE_CONSTRUCT)[0][0];
        assert_eq!(
            calls(&e, MODEL_LOADER_OUTPUT_MODEL_MAP),
            vec![vec![0x6000, model_file, 0]]
        );
        assert_eq!(
            calls(&e, FACE_GEN_OUTPUT_MODEL_MAP),
            vec![vec![0x7000, model_file]]
        );
        assert_eq!(calls(&e, BS_FILE_CLOSE).len(), 2);
        assert_eq!(calls(&e, BS_FILE_DESTRUCT).len(), 2);
        assert_eq!(calls(&e, MEM_STATS_FILE_DESTRUCT), vec![vec![dump]]);
        // The table of 0x800 NiPointers is built and destroyed.
        assert_eq!(calls(&e, VECTOR_CONSTRUCT)[0][1..4], [4, 0x800, 0x6694e0]);
        assert_eq!(calls(&e, VECTOR_DESTRUCT)[0][1..], [4, 0x800, 0x45cec0]);

        // Without an argument the default name is used and the model
        // loader's map is skipped.
        e.register(PARSE_PARAMETERS, |_, _| false.into_ret());
        e.set_global(MODEL_LOADER, 0u32);
        start_log(&mut e);
        assert!(e.call(0x005b_5e60, &args![script(0)]).bool());
        assert_eq!(calls(&e, STRCPY_S)[0][2], DEFAULT_MEM_STATS);
        assert!(calls(&e, MODEL_LOADER_OUTPUT_MODEL_MAP).is_empty());
    }

    #[test]
    fn mem_stats_lists_the_textures_with_their_category() {
        let mut e = output_engine();
        // The path fragments the categories are found by.
        for (addr, text) in [
            (PATH_ARMOR, "\\armor\\"),
            (PATH_CREATURES, "\\creatures\\"),
            (PATH_CHARACTERS, "\\characters\\"),
            (PATH_PIPBOY, "\\pipboy3000\\"),
            (PATH_WEAPONS, "\\weapons\\"),
            (PATH_DECALS, "\\decals\\"),
            (PATH_PROJECTILES, "\\projectiles\\"),
            (PATH_FONTS, "\\fonts\\"),
            (PATH_INTERFACE, "\\interface\\"),
            (PATH_EFFECTS, "\\effects\\"),
            (PATH_GORE, "\\gore\\"),
            (PATH_SKY, "\\sky\\"),
            (PATH_LOD, "\\lod\\"),
            (PATH_WATER, "\\water\\"),
        ] {
            put_str(&mut e, addr, text);
        }
        parse_gives(&mut e, false, &[]);
        // Three textures: a source texture under \armor\, a source texture
        // without a name, and a rendered one. Layout of the test objects:
        // +0x24 Direct3D texture, +0x2c next texture, +4 reference count,
        // +0x30 "is a source texture", +0x34 name text, +0x38 "L" kind.
        let vtable = e.mem.alloc(0x100);
        let owner_name = e.mem.alloc(8);
        let owner = e.mem.alloc(8);
        e.mem.set_u32(owner, owner_name);
        e.register_double(V_KIND + 8, move |_, _| owner.into_ret());
        e.register(V_KIND + 9, |_, _| 128u32.into_ret());
        e.register(V_KIND + 10, |_, _| 64u32.into_ret());
        e.register(V_KIND + 11, |_, a| a[0].into_ret());
        e.mem.set_u32(vtable + 0x08, V_KIND + 8);
        e.mem.set_u32(vtable + 0x94, V_KIND + 9);
        e.mem.set_u32(vtable + 0x98, V_KIND + 10);
        e.mem.set_u32(vtable + 0x9c, V_KIND + 11);
        let armor_name = e.mem.alloc(0x40);
        put_str(&mut e, armor_name, "textures\\armor\\vault.dds");
        let empty = e.mem.alloc(8);
        let mut textures = vec![];
        for (is_source, name, kind) in [(1, armor_name, 3), (1, 0, 1), (0, empty, 1)] {
            let t = e.mem.alloc(0x60);
            e.mem.set_u32(t, vtable);
            e.mem.set_u32(t + 4, 3);
            e.mem.set_u32(t + 0x24, 0xd3d0 + textures.len() as u32);
            e.mem.set_u32(t + 0x30, is_source);
            e.mem.set_u32(t + 0x34, name);
            e.mem.set_u32(t + 0x38, kind);
            textures.push(t);
        }
        for i in 0..2 {
            e.mem.set_u32(textures[i] + 0x2c, textures[i + 1]);
        }
        let first = textures[0];
        e.register_double(FIRST_TEXTURE, move |_, _| first.into_ret());
        e.register(NEXT_TEXTURE, |e, a| e.mem.u32(a[0] + 0x2c).into_ret());
        e.register(GET_D3D_TEXTURE, |e, a| e.mem.u32(a[0] + 0x24).into_ret());
        e.register(NI_POINTER_ASSIGN, |e, a| {
            let value = e.mem.u32(a[1]);
            e.mem.set_u32(a[0], value);
            Ret::default()
        });
        e.register(NI_POINTER_SET, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            Ret::default()
        });
        e.register(TEXTURE_IS_SOURCE, |e, a| {
            assert_eq!(a[0], TEXTURE_CLASS);
            (e.mem.u32(a[1] + 0x30) != 0).into_ret()
        });
        // A source texture's name comes from slot 0x9c (a handle that is the
        // texture itself here); a nameless one falls back to `+8`'s handle.
        e.register(NAME_TEXT, |e, a| e.mem.u32(a[0] + 0x34).into_ret());
        e.register(FN_00413F40, |_, a| a[0].into_ret());
        e.register(FN_007D6BB0, |_, a| a[0].into_ret());
        e.register(D3D_FORMAT_ID, |_, a| (a[0] + 1).into_ret());
        e.register(GET_D3D_FORMAT_STRING, |_, a| (a[0] + 2).into_ret());
        e.register(TEXTURE_KIND, |e, a| e.mem.u32(a[0] + 0x38).into_ret());
        start_log(&mut e);
        assert!(e.call(0x005b_5e60, &args![script(0)]).bool());

        // The textures were inserted in order (all sizes are 0).
        let table = calls(&e, NI_POINTER_SET);
        assert_eq!(table.iter().map(|w| w[1]).collect::<Vec<_>>(), textures);
        assert_eq!(table[1][0], table[0][0] + 4);
        // Descriptions: the armor path, the unnamed one and the rendered one.
        let describing: Vec<u32> = calls(&e, STRCPY_S).iter().skip(1).map(|w| w[2]).collect();
        assert_eq!(describing, vec![armor_name, TEXT_UNNAMED_SOURCE_TEXTURE]);
        let rendered = calls(&e, SPRINTF_S)
            .into_iter()
            .filter(|w| w[2] == TEXT_RENDERED_TEXTURE_NONE)
            .count();
        assert_eq!(rendered, 1);
        // One line per texture: index, category, description, width, height,
        // size in KB, format, reference count minus one, "L"/"H", owner name.
        let lines: Vec<Vec<u32>> = calls(&e, SPRINTF_S)
            .into_iter()
            .filter(|w| w[2] == FORMAT_TEXTURE_LINE)
            .collect();
        assert_eq!(lines.len(), 3);
        let description = lines[0][5];
        assert_eq!(
            lines[0][3..],
            [
                1,
                CATEGORY_ACTOR,
                description,
                128,
                64,
                0,
                0xd3d0 + 3,
                2,
                TEXT_L,
                owner_name
            ]
        );
        // No path fragment matches the unnamed texture: MISCREF.
        assert_eq!(lines[1][3..5], [2, CATEGORY_MISCREF]);
        assert_eq!(lines[1][11], TEXT_H);
        // The rendered one is the "SYSTEM" category, whatever its text says.
        assert_eq!(lines[2][3..5], [3, CATEGORY_SYSTEM]);
        assert_eq!(calls(&e, BS_FILE_WRITE).len(), 3);
    }

    #[test]
    fn mem_stats_categories_follow_the_path() {
        // Each fragment, in the game's order, selects its category.
        for (path, category) in [
            ("a\\creatures\\x", CATEGORY_ACTOR),
            ("a\\projectiles\\x", CATEGORY_ACTOR),
            ("a\\interface\\x", CATEGORY_MENUS),
            ("a\\gore\\x", CATEGORY_EFFECTS),
            ("a\\sky\\x", CATEGORY_SKY),
            ("a\\lod\\x", CATEGORY_LOD),
            ("a\\water\\x", CATEGORY_WATER),
            ("a\\other\\x", CATEGORY_MISCREF),
        ] {
            let mut e = output_engine();
            for (addr, text) in [
                (PATH_ARMOR, "\\armor\\"),
                (PATH_CREATURES, "\\creatures\\"),
                (PATH_CHARACTERS, "\\characters\\"),
                (PATH_PIPBOY, "\\pipboy3000\\"),
                (PATH_WEAPONS, "\\weapons\\"),
                (PATH_DECALS, "\\decals\\"),
                (PATH_PROJECTILES, "\\projectiles\\"),
                (PATH_FONTS, "\\fonts\\"),
                (PATH_INTERFACE, "\\interface\\"),
                (PATH_EFFECTS, "\\effects\\"),
                (PATH_GORE, "\\gore\\"),
                (PATH_SKY, "\\sky\\"),
                (PATH_LOD, "\\lod\\"),
                (PATH_WATER, "\\water\\"),
            ] {
                put_str(&mut e, addr, text);
            }
            parse_gives(&mut e, false, &[]);
            let name = e.mem.alloc(0x40);
            put_str(&mut e, name, path);
            let vtable = e.mem.alloc(0x100);
            let owner = e.mem.alloc(8);
            e.register_double(V_KIND + 8, move |_, _| owner.into_ret());
            e.mem.set_u32(vtable + 0x08, V_KIND + 8);
            e.register(V_KIND + 9, |_, a| a[0].into_ret());
            e.mem.set_u32(vtable + 0x9c, V_KIND + 9);
            e.mem.set_u32(vtable + 0x94, V_KIND);
            e.mem.set_u32(vtable + 0x98, V_KIND);
            let texture = e.mem.alloc(0x60);
            e.mem.set_u32(texture, vtable);
            e.mem.set_u32(texture + 0x34, name);
            e.register_double(FIRST_TEXTURE, move |_, _| texture.into_ret());
            e.register(NEXT_TEXTURE, |_, _| 0u32.into_ret());
            e.register(GET_D3D_TEXTURE, |_, _| 1u32.into_ret());
            e.register(NI_POINTER_ASSIGN, |_, _| Ret::default());
            e.register(NI_POINTER_SET, |e, a| {
                e.mem.set_u32(a[0], a[1]);
                Ret::default()
            });
            e.register(TEXTURE_IS_SOURCE, |_, _| true.into_ret());
            e.register(NAME_TEXT, |e, a| e.mem.u32(a[0] + 0x34).into_ret());
            accept(&mut e, &[FN_00413F40, FN_007D6BB0, D3D_FORMAT_ID]);
            e.register(GET_D3D_FORMAT_STRING, |_, _| 0u32.into_ret());
            e.register(TEXTURE_KIND, |_, _| 0u32.into_ret());
            start_log(&mut e);
            assert!(e.call(0x005b_5e60, &args![script(0)]).bool());
            let line = calls(&e, SPRINTF_S)
                .into_iter()
                .find(|w| w[2] == FORMAT_TEXTURE_LINE)
                .unwrap();
            assert_eq!(line[4], category, "{path}");
        }
    }

    #[test]
    fn texture_use_map_writes_the_file_and_reports_the_size() {
        let mut e = output_engine();
        accept(
            &mut e,
            &[
                TEXTURE_USE_MAP_CONSTRUCT,
                TEXTURE_USE_MAP_ADD_NODE,
                TEXTURE_USE_MAP_ADD_SCENE,
                TEXTURE_USE_MAP_DESTRUCT,
                BS_STRING_CONSTRUCT,
                BS_STRING_DESTRUCT,
                BS_STRING_APPEND,
            ],
        );
        e.register(BS_STRING_FORMAT, |e, a| {
            e.mem.set_u32(a[0], 0x5151);
            Ret::default()
        });
        put_str(&mut e, DEFAULT_TEXTURE_USE_MAP, "TextureUseMap.txt");
        e.register_double(PARSE_PARAMETERS, |e, a| {
            e.mem.set_cstr(a[7], b"my.txt");
            true.into_ret()
        });
        let this_obj = object_with(&mut e, &[(0x1d0, V_NINE)]);

        // A reference with a 3D (slot 0x1d0): its node; written; 5 MB.
        e.register(TEXTURE_USE_MAP_WRITE_TO_FILE, |_, _| true.into_ret());
        e.register(TEXTURE_USE_MAP_TOTAL_SIZE, |_, _| 0x50_0000u32.into_ret());
        start_log(&mut e);
        assert!(e.call(0x005b_6800, &args![script(this_obj)]).bool());
        let map = calls(&e, TEXTURE_USE_MAP_CONSTRUCT)[0][0];
        assert_eq!(calls(&e, TEXTURE_USE_MAP_ADD_NODE), vec![vec![map, 9, 1]]);
        assert!(calls(&e, TEXTURE_USE_MAP_ADD_SCENE).is_empty());
        let path = calls(&e, TEXTURE_USE_MAP_WRITE_TO_FILE)[0][1];
        assert_eq!(e.mem.cstr(path), b"my.txt");
        let text = calls(&e, BS_STRING_CONSTRUCT)[0][0];
        assert_eq!(calls(&e, BS_STRING_FORMAT), vec![vec![text, FORMAT_MB, 5]]);
        assert_eq!(
            calls(&e, BS_STRING_APPEND),
            vec![vec![text, TEXT_TEXTURES_IN_USE]]
        );
        assert_eq!(printed(&e), vec![vec![MSG_MAP_WRITTEN, path], vec![0x5151]]);
        assert_eq!(calls(&e, TEXTURE_USE_MAP_DESTRUCT), vec![vec![map]]);

        // No reference: the whole scene. A failed write is reported; sizes
        // pick KB above 0x400 bytes and bytes at or below.
        e.register(TEXTURE_USE_MAP_WRITE_TO_FILE, |_, _| false.into_ret());
        for (size, format, shown) in [
            (0x2000u32, FORMAT_KB, 8),
            (0x401, FORMAT_KB, 1),
            (0x400, FORMAT_BYTES, 0x400),
            (0x10_0000, FORMAT_KB, 0x400),
            (0x10_0001, FORMAT_MB, 1),
        ] {
            e.register_double(TEXTURE_USE_MAP_TOTAL_SIZE, move |_, _| size.into_ret());
            start_log(&mut e);
            assert!(e.call(0x005b_6800, &args![script(0)]).bool());
            let map = calls(&e, TEXTURE_USE_MAP_CONSTRUCT)[0][0];
            assert_eq!(calls(&e, TEXTURE_USE_MAP_ADD_SCENE), vec![vec![map, 1]]);
            let text = calls(&e, BS_STRING_CONSTRUCT)[0][0];
            assert_eq!(calls(&e, BS_STRING_FORMAT), vec![vec![text, format, shown]]);
            let path = calls(&e, TEXTURE_USE_MAP_WRITE_TO_FILE)[0][1];
            assert_eq!(printed(&e)[0], vec![MSG_MAP_WRITE_FAILED, path]);
        }

        // Bad parameters: the default file name is used.
        e.register(PARSE_PARAMETERS, |_, _| false.into_ret());
        start_log(&mut e);
        assert!(e.call(0x005b_6800, &args![script(0)]).bool());
        assert_eq!(calls(&e, STRCPY_S).len(), 2);
        let path = calls(&e, TEXTURE_USE_MAP_WRITE_TO_FILE)[0][1];
        assert_eq!(e.mem.cstr(path), b"TextureUseMap.txt");
    }

    #[test]
    fn the_other_memory_dump_command_configures_the_output_with_flag_one() {
        let mut e = output_engine();
        put_str(&mut e, DEFAULT_MEM_STATS, "OutputMemStats.mhd");
        e.register_double(PARSE_PARAMETERS, |e, a| {
            e.mem.set_cstr(a[7], b"Other.mhd");
            true.into_ret()
        });
        start_log(&mut e);
        assert!(e.call(0x005b_6a30, &args![script(0)]).bool());
        let dump = calls(&e, MEM_STATS_FILE_CONSTRUCT)[0][0];
        let path = calls(&e, MEM_STATS_FILE_CONSTRUCT)[0][1];
        assert_eq!(e.mem.cstr(path), b"Other.mhd");
        assert_eq!(calls(&e, MEM_STATS_OUTPUT_GET), vec![vec![dump, 1]]);
        assert_eq!(calls(&e, MEM_STATS_OUTPUT_SET), vec![vec![0x11f_6238]]);
        assert_eq!(calls(&e, MEM_STATS_FILE_DESTRUCT), vec![vec![dump]]);
        // Bad parameters: the default name is used.
        e.register(PARSE_PARAMETERS, |_, _| false.into_ret());
        start_log(&mut e);
        assert!(e.call(0x005b_6a30, &args![script(0)]).bool());
        let path = calls(&e, MEM_STATS_FILE_CONSTRUCT)[0][1];
        assert_eq!(e.mem.cstr(path), b"OutputMemStats.mhd");
        assert_eq!(calls(&e, STRNCPY_S).len(), 2);
    }

    #[test]
    fn archive_profile_appends_the_extension_and_reports_it_is_disabled() {
        let mut e = output_engine();
        put_str(&mut e, EXTENSION_TXT, ".txt");
        put_str(&mut e, DEFAULT_ARCHIVE_PROFILE, "ArchiveProfile.txt");
        e.register(ARCHIVE_PROFILE_START, |_, _| false.into_ret());
        e.register_double(PARSE_PARAMETERS, |e, a| {
            e.mem.set_cstr(a[7], b"prof");
            true.into_ret()
        });
        start_log(&mut e);
        assert!(e.call(0x005b_6b50, &args![script(0)]).bool());
        let path = calls(&e, STRCAT_S)[0][0];
        assert_eq!(calls(&e, STRCAT_S), vec![vec![path, 0x200, EXTENSION_TXT]]);
        assert_eq!(e.mem.cstr(path), b"prof.txt");
        assert_eq!(printed(&e), vec![vec![MSG_ARCHIVE_PROFILE_DISABLED]]);

        // A name that has the extension is kept; when profiling starts the
        // file name is printed.
        e.register_double(PARSE_PARAMETERS, |e, a| {
            e.mem.set_cstr(a[7], b"prof.txt");
            true.into_ret()
        });
        e.register(ARCHIVE_PROFILE_START, |_, _| true.into_ret());
        start_log(&mut e);
        assert!(e.call(0x005b_6b50, &args![script(0)]).bool());
        assert!(calls(&e, STRCAT_S).is_empty());
        let path = calls(&e, ARCHIVE_PROFILE_START)[0][0];
        assert_eq!(printed(&e), vec![vec![MSG_ARCHIVE_PROFILE_OUTPUT, path]]);

        // Bad parameters: the default name, which has the extension.
        e.register(PARSE_PARAMETERS, |_, _| false.into_ret());
        start_log(&mut e);
        assert!(e.call(0x005b_6b50, &args![script(0)]).bool());
        let path = calls(&e, ARCHIVE_PROFILE_START)[0][0];
        assert_eq!(e.mem.cstr(path), b"ArchiveProfile.txt");
    }

    #[test]
    fn two_singletons_get_their_plus_4_member_handed_back() {
        let mut e = engine();
        e.register(FN_0044F560, |_, _| 0x1000u32.into_ret());
        e.register(FN_004DE490, |_, _| 0x2000u32.into_ret());
        e.register(PLUS_4, |_, a| (a[0] + 4).into_ret());
        accept(&mut e, &[FN_005E01B0]);
        start_log(&mut e);
        assert!(e.call(0x005b_6c50, &args![]).bool());
        assert_eq!(
            calls(&e, FN_005E01B0),
            vec![vec![0x1000, 0x1004], vec![0x2000, 0x2004]]
        );
    }

    #[test]
    fn bye_sets_the_flag_byte_of_the_singleton() {
        let mut e = engine();
        let singleton = e.mem.alloc(8);
        e.set_global(GLOBAL_0011DEA0C, singleton);
        assert_eq!(run_and_print(&mut e, 0x005b_6c90), vec![vec![MSG_BYE]]);
        assert_eq!(e.mem.u8(singleton + 1), 1);
        // The setter on its own.
        let other = e.mem.alloc(8);
        e.call(0x005b_6cb0, &args![other]);
        assert_eq!(e.mem.u8(other + 1), 1);
        assert_eq!(e.mem.u8(other), 0);
    }

    #[test]
    fn free_released_objects_wraps_the_call_in_a_level_scope() {
        let mut e = engine();
        e.register(FN_00878160, |e, a| {
            e.mem.set_u8(a[0] + 5, 0x42);
            Ret::default()
        });
        accept(&mut e, &[FN_00878200, FN_004539A0, FREE_RELEASED_OBJECTS]);
        e.set_global(GLOBAL_0011DEA10, 0x1234u32);
        start_log(&mut e);
        assert!(e.call(0x005b_6cd0, &args![]).bool());
        // Skip the logged call of the command itself.
        let log: Vec<(u32, Vec<u32>)> = e.call_log.clone().unwrap().into_iter().skip(1).collect();
        let scope = log[0].1[0];
        assert_eq!(
            log,
            vec![
                (FN_00878160, vec![scope, 1, 1, 1]),
                (FN_004539A0, vec![0x1234, 1, 0]),
                (FREE_RELEASED_OBJECTS, vec![0x42]),
                (FN_00878200, vec![scope]),
            ]
        );
    }

    #[test]
    fn the_save_load_singleton_is_called_with_zeros() {
        let mut e = engine();
        accept(&mut e, &[FN_00848E70]);
        e.set_global(SAVE_LOAD_GAME, 0x7777u32);
        start_log(&mut e);
        assert!(e.call(0x005b_6d20, &args![]).bool());
        assert_eq!(calls(&e, FN_00848E70), vec![vec![0x7777, 0, 0]]);
    }

    /// Doubles and objects for `ToggleMapMarkers`: two world spaces (the
    /// second has no markers) and, for the first, two markers; the first
    /// marker's data (at +0x10) is hidden. Data bytes: +0 hidden, +1 has a
    /// travel location.
    fn map_marker_world() -> (Engine, [u32; 2]) {
        let mut e = engine();
        let m1 = object_with(&mut e, &[(0x48, V_RECORD)]);
        let m2 = object_with(&mut e, &[(0x48, V_RECORD)]);
        e.mem.set_u8(m1 + 0x10, 1);
        let w1 = e.mem.alloc(8);
        let w2 = e.mem.alloc(8);
        let worlds = list_of(&mut e, &[w1, w2]);
        e.set_global(DATA_HANDLER, 0x5000u32);
        e.register_double(DATA_HANDLER_WORLD_SPACES, move |_, a| {
            assert_eq!(a[0], 0x5000);
            worlds.into_ret()
        });
        e.register_double(BUILD_MAP_MARKER_LIST, move |e, a| {
            assert_eq!(a[1], 0);
            if a[0] == w1 {
                list_of(e, &[m1, m2]).into_ret()
            } else {
                0u32.into_ret()
            }
        });
        e.register(GET_MAP_MARKER_DATA, |_, a| (a[0] + 0x10).into_ret());
        e.register(MAP_MARKER_GET_HIDDEN, |e, a| {
            (e.mem.u8(a[0]) != 0).into_ret()
        });
        e.register(MAP_MARKER_GET_TRAVEL_LOC, |e, a| {
            (e.mem.u8(a[0] + 1) != 0).into_ret()
        });
        accept(&mut e, &[MAP_MARKER_SET_VISIBLE, MAP_MARKER_SET_TRAVEL_LOC]);
        (e, [m1, m2])
    }

    #[test]
    fn toggle_map_markers_shows_the_markers_that_are_not_hidden() {
        let (mut e, [m1, m2]) = map_marker_world();
        parse_gives(&mut e, true, &[1, 1, 1]);
        start_log(&mut e);
        assert!(e.call(0x005b_6d40, &args![script(0)]).bool());
        // The hidden first marker is skipped; the second is made visible, its
        // travel location set (it had none), and slot 0x48 called on it.
        assert_eq!(calls(&e, MAP_MARKER_SET_VISIBLE), vec![vec![m2 + 0x10, 1]]);
        assert_eq!(
            calls(&e, MAP_MARKER_SET_TRAVEL_LOC),
            vec![vec![m2 + 0x10, 1]]
        );
        assert_eq!(calls(&e, V_RECORD), vec![vec![m2, 0x8000_0000]]);
        // The marker list is deleted; the second world has none.
        assert_eq!(calls(&e, LIST_DELETE).len(), 1);
        assert_eq!(calls(&e, LIST_DELETE)[0][1], 1);
        // The echo flag is off: no print.
        assert!(printed(&e).is_empty());

        // A marker that already has a travel location keeps it.
        e.mem.set_u8(m2 + 0x11, 1);
        start_log(&mut e);
        assert!(e.call(0x005b_6d40, &args![script(0)]).bool());
        assert!(calls(&e, MAP_MARKER_SET_TRAVEL_LOC).is_empty());
        let _ = m1;
    }

    #[test]
    fn toggle_map_markers_hides_everything_and_echoes() {
        let (mut e, [m1, m2]) = map_marker_world();
        // Hide all, hidden markers included (third argument 0).
        parse_gives(&mut e, true, &[0, 1, 0]);
        let tls = e.tls();
        e.mem.set_u8(tls + TLS_ECHO, 1);
        accept(&mut e, &[CONSOLE_PRINT]);
        start_log(&mut e);
        assert!(e.call(0x005b_6d40, &args![script(0)]).bool());
        assert_eq!(
            calls(&e, MAP_MARKER_SET_VISIBLE),
            vec![vec![m1 + 0x10, 0], vec![m2 + 0x10, 0]]
        );
        assert_eq!(
            calls(&e, MAP_MARKER_SET_TRAVEL_LOC),
            vec![vec![m1 + 0x10, 0], vec![m2 + 0x10, 0]]
        );
        assert_eq!(
            calls(&e, V_RECORD),
            vec![vec![m1, 0x8000_0000], vec![m2, 0x8000_0000]]
        );
        assert_eq!(printed(&e), vec![vec![MSG_MAP_MARKERS, TEXT_HIDDEN]]);
        // Shown: the message says so.
        parse_gives(&mut e, true, &[1, 1, 1]);
        start_log(&mut e);
        assert!(e.call(0x005b_6d40, &args![script(0)]).bool());
        assert_eq!(printed(&e), vec![vec![MSG_MAP_MARKERS, TEXT_SHOWN]]);
        // Bad parameters: false and no walk.
        parse_gives(&mut e, false, &[]);
        start_log(&mut e);
        assert!(!e.call(0x005b_6d40, &args![script(0)]).bool());
        assert!(printed(&e).is_empty());
    }

    #[test]
    fn verbose_flag_accessors_and_toggle() {
        let mut e = engine();
        assert!(!e.call(0x005b_6f70, &args![]).bool());
        e.call(0x005b_6f60, &args![true]);
        assert!(e.call(0x005b_6f70, &args![]).bool());
        assert_eq!(e.global::<u8>(VERBOSE_MESSAGES), 1);
        e.call(0x005b_6f60, &args![false]);
        assert_eq!(e.global::<u8>(VERBOSE_MESSAGES), 0);
        // The toggle flips and prints the new state.
        assert_eq!(
            run_and_print(&mut e, 0x005b_6f10),
            vec![vec![MSG_VERBOSE, TEXT_SHOWN]]
        );
        assert_eq!(
            run_and_print(&mut e, 0x005b_6f10),
            vec![vec![MSG_VERBOSE, TEXT_HIDDEN]]
        );
    }

    /// God and demigod mode doubles over the exe's two flag bytes.
    fn mode_engine() -> Engine {
        let mut e = engine();
        e.register(IS_GOD_MODE, |e, _| (e.mem.u8(0x011e_07ba) != 0).into_ret());
        e.register(IS_DEMIGOD_MODE, |e, _| {
            (e.mem.u8(0x011e_07bb) != 0).into_ret()
        });
        e.register(SET_GOD_MODE, |e, a| {
            e.mem.set_u8(0x011e_07ba, a[0] as u8);
            Ret::default()
        });
        e.register(SET_DEMIGOD_MODE, |e, a| {
            e.mem.set_u8(0x011e_07bb, a[0] as u8);
            Ret::default()
        });
        e
    }

    #[test]
    fn god_mode_toggle_also_ends_demigod_mode_when_turned_off() {
        let mut e = mode_engine();
        assert_eq!(
            run_and_print(&mut e, 0x005b_6f80),
            vec![vec![MSG_GOD_MODE, TEXT_ENABLED]]
        );
        assert_eq!(e.mem.u8(0x011e_07ba), 1);
        // Demigod mode on as well: turning god mode off clears it with a
        // message of its own.
        e.mem.set_u8(0x011e_07bb, 1);
        assert_eq!(
            run_and_print(&mut e, 0x005b_6f80),
            vec![
                vec![MSG_DEMIGOD_DISABLED],
                vec![MSG_GOD_MODE, TEXT_DISABLED]
            ]
        );
        assert_eq!(e.mem.u8(0x011e_07ba), 0);
        assert_eq!(e.mem.u8(0x011e_07bb), 0);
        // Off with demigod mode off: just the one message.
        assert_eq!(
            run_and_print(&mut e, 0x005b_6f80),
            vec![vec![MSG_GOD_MODE, TEXT_ENABLED]]
        );
    }

    #[test]
    fn demigod_mode_toggle_sets_both_flags_alike() {
        let mut e = mode_engine();
        assert_eq!(
            run_and_print(&mut e, 0x005b_7000),
            vec![vec![MSG_DEMIGOD_MODE, TEXT_ENABLED]]
        );
        assert_eq!(e.mem.u8(0x011e_07ba), 1);
        assert_eq!(e.mem.u8(0x011e_07bb), 1);
        assert_eq!(
            run_and_print(&mut e, 0x005b_7000),
            vec![vec![MSG_DEMIGOD_MODE, TEXT_DISABLED]]
        );
        assert_eq!(e.mem.u8(0x011e_07ba), 0);
        assert_eq!(e.mem.u8(0x011e_07bb), 0);
    }
}
