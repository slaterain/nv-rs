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
//! Progress: the first 80 queue entries (`005b4b20` to `005b9520`) are
//! translated. The next session continues at `005b9540`
//! (`Script::ToggleSkyFunction`).
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
//! - The decompiler often attaches pushes to the wrong call: a `PUSH` before a
//!   call that is followed by no stack clean-up belongs to the next callee
//!   (`RET n`), and a call written with arguments in C can be a call without
//!   (`004e3270` takes none, `00b8cb00` takes the pushed word). Argument
//!   order here is always read from the disassembly.
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

// ---- Second batch (`005b7070` onward): callees, globals and strings --------------
//
// Offsets named "PC offset" are the ones the PC code reads; the Xbox layouts
// of `TESObjectREFR` and `TESObjectCELL` differ from the PC ones.

/// `Script::SetProcessScripts` (Xbox PDB), `cdecl` (`bool`).
const SET_PROCESS_SCRIPTS: u32 = 0x005a_c730;
/// `Script::GetProcessScripts` (Xbox PDB).
const GET_PROCESS_SCRIPTS: u32 = 0x005a_c740;

// Grass display.
/// `cdecl`, no arguments: loads the global `011ca438`, the object the grass
/// toggle works on (zero when there is none).
const FN_0054F4C0: u32 = 0x0054_f4c0;
/// `thiscall` on that object: the flag the toggle reads (calls
/// `00456630(this, 1)`).
const FN_00456610: u32 = 0x0045_6610;
/// `thiscall` on that object (`flag`): calls `0043b370(this, flag, 1)`.
const FN_00450F90: u32 = 0x0045_0f90;
/// `cdecl`, no arguments: run when the flag is set.
const FN_00B62A20: u32 = 0x00b6_2a20;
/// `cdecl`, no arguments: loads the global `011deb7c`.
const FN_0045C670: u32 = 0x0045_c670;
/// `BSFaceGenNiNode::GetAnimationData` (engine map name; the body reads the
/// pointer at `this + 0xac`).
const FN_006629F0: u32 = 0x0066_29f0;
/// `thiscall` (a vector and a `float`, all ignored): returns `this + 0x8c`,
/// the address of a vector.
const FN_0045BB80: u32 = 0x0045_bb80;
/// `cdecl` (`x, y, z`): takes the three words of that vector.
const FN_0057D0A0: u32 = 0x0057_d0a0;

// Cell tests and the `TES` singleton (`GLOBAL_0011DEA10`).
/// `TES::TestAllCells` (Xbox PDB), `thiscall` (`cell count or -1`).
const TES_TEST_ALL_CELLS: u32 = 0x0045_56d0;
/// `thiscall` on the `TES`: `bRunningCellTests` or `bRunningCellTests2`
/// (`+0x51`, `+0x52`).
const TES_IS_RUNNING_CELL_TESTS: u32 = 0x0045_1530;
/// `thiscall` on a reference: the parent cell (PC offset `+0x40`).
const REFR_GET_PARENT_CELL: u32 = 0x008d_6f30;
/// `TESObjectCELL::RenderTestCell` (Xbox PDB), `cdecl` (`cell, 0, flag`).
const CELL_RENDER_TEST_CELL: u32 = 0x0055_7dd0;
/// `thiscall` on the `TES`, no arguments.
const FN_00456A60: u32 = 0x0045_6a60;
/// `thiscall` on the `TES`, no arguments.
const FN_00456AA0: u32 = 0x0045_6aa0;
/// `thiscall` on the `TES` (`flag`): sets `bShowLANDborders` (`+0x60`) and
/// passes it on.
const TES_SET_SHOW_LAND_BORDERS: u32 = 0x0045_6cb0;
/// `thiscall` on the `TES` (`name`): appends `".hkx"` (`01017cec`) to the
/// name and hands the path to `00c67430`.
const FN_00456B10: u32 = 0x0045_6b10;
/// `*(this + 0x34)` (the engine map names it
/// `ActorMover::GetPreferredMoveMode`; the code is shared).
const FN_005F36F0: u32 = 0x005f_36f0;

// Ini refresh.
/// `cdecl`, no arguments: the address `01202fa0`.
const FN_004DC110: u32 = 0x004d_c110;
/// `thiscall` on an ini collection singleton (`path`): calls
/// `005e0830(this, path, 0)` and, when that succeeds, virtual slots `0x24`
/// and `0x1c`.
const FN_005E0200: u32 = 0x005e_0200;

// Sky and weather.
/// `Sky::GetInstance` (Xbox PDB), `cdecl`.
const SKY_GET_INSTANCE: u32 = 0x0046_dd00;
/// `*(sky + 0x0c)`: `pCurrentClimate` (Xbox PDB, same on PC).
const SKY_GET_CURRENT_CLIMATE: u32 = 0x0084_e3a0;
/// `*(sky + 0x10)`: `pCurrentWeather` (Xbox PDB, same on PC); the engine map
/// names it `BaseProcess::GetCurrentProcedureIndex` (identical code).
const SKY_GET_CURRENT_WEATHER: u32 = 0x0044_edb0;
/// `TESForm::GetFile` (Xbox PDB), `thiscall` (`-1`).
const FORM_GET_FILE: u32 = 0x0048_4e60;
/// `TESFile::GetThreadSafeFile` (Xbox PDB), `thiscall`.
const FILE_GET_THREAD_SAFE_FILE: u32 = 0x0047_39b0;
/// `TESFile::OpenTES` (Xbox PDB), `thiscall` (`0, 0`).
const FILE_OPEN_TES: u32 = 0x0047_0c70;
/// `TESFile::FindForm` (Xbox PDB), `thiscall` (`form`).
const FILE_FIND_FORM: u32 = 0x0047_34d0;
/// `Sky::SetCurrentClimate` (Xbox PDB), `thiscall` (`climate, 1`).
const SKY_SET_CURRENT_CLIMATE: u32 = 0x0063_c8f0;
/// `Sky::ForceWeather` (Xbox PDB), `thiscall` (`weather, flag`).
const SKY_FORCE_WEATHER: u32 = 0x0063_d0e0;
/// `thiscall` on the player (`0`), run after the default weather was set.
const FN_0093A7A0: u32 = 0x0093_a7a0;
/// `thiscall` on the sky (`1`), run after a weather was set.
const FN_0063E860: u32 = 0x0063_e860;
/// Stores 0 at `this + 0x1c` (`pOverrideWeather`); the engine map names it
/// `MagicHitEffect::ClearTarget` (identical code).
const SKY_CLEAR_OVERRIDE_WEATHER: u32 = 0x0081_bc50;
/// `pDefaultWeather` in `Sky` (Xbox PDB, same on PC).
const SKY_DEFAULT_WEATHER: u32 = 0x18;
/// `pOverrideWeather` in `Sky` (Xbox PDB, same on PC).
const SKY_OVERRIDE_WEATHER: u32 = 0x1c;

// Image space modifiers.
/// `ImageSpaceModifierInstanceForm::Trigger` (Xbox PDB), `cdecl` (`form,
/// strength, 0`).
const IMAGE_SPACE_MODIFIER_TRIGGER: u32 = 0x0052_99a0;
/// `ImageSpaceModifierInstanceForm::Stop` (Xbox PDB), `cdecl` (`form`).
const IMAGE_SPACE_MODIFIER_STOP: u32 = 0x0052_9c90;
/// `thiscall` (`argument`): calls `004610d0(this, argument)` and
/// `0041c290` on its result.
const FN_00547750: u32 = 0x0054_7750;
/// `cdecl`, no arguments: loads the global `011f91ac`.
const GET_IMAGE_SPACE_MANAGER: u32 = 0x004e_3270;
/// `thiscall` on that object (`form`): stores the form at `this + 0xb0`.
const IMAGE_SPACE_MANAGER_SET_OVERRIDE: u32 = 0x00b8_cb00;
/// `thiscall` on that object, no arguments.
const FN_00B8B500: u32 = 0x00b8_b500;

// Allocation and construction.
/// `operator new` (`MemoryManager::Allocate`), `cdecl` (`size`).
const OPERATOR_NEW: u32 = 0x0040_1000;
/// `NiMemObject::operator new`, `cdecl` (`size`).
const NI_ALLOC: u32 = 0x00aa_13e0;
/// Allocates an array of `short`s, `cdecl` (`bytes`).
const NI_ALLOC_SHORTS: u32 = 0x00aa_1070;
/// `NiNode::NiNode(capacity)`, `thiscall` (`0`).
const NI_NODE_CONSTRUCT: u32 = 0x00a5_ecb0;
/// The array constructor loop, `cdecl` (`array, element size, count,
/// constructor`): calls the constructor (`thiscall`) on every element.
const VECTOR_CONSTRUCT_SIMPLE: u32 = 0x0040_1050;
/// A trivial constructor: returns `this` (the same code as
/// [`LIST_ITEM_PTR`]).
const TRIVIAL_CONSTRUCT: u32 = 0x0068_15c0;
/// `NiPoint3::NiPoint3` (`x, y, z`), `thiscall`, returns `this`.
const NI_POINT3_CONSTRUCT: u32 = 0x0041_6870;
/// `NiColorA::NiColorA` (`r, g, b, a`), `thiscall`, returns `this`.
const NI_COLOR_A_CONSTRUCT: u32 = 0x0041_4430;
/// The constructor of one `NiColorA` element: `NiColorA(0, 0, 0, 0)`.
const NI_COLOR_A_ZERO_CONSTRUCT: u32 = 0x004a_7800;
/// `NiPoint2::NiPoint2` (`u, v`), `thiscall`, returns `this`.
const NI_POINT2_CONSTRUCT: u32 = 0x0045_2dc0;
/// `thiscall` (`angle`): fills the 36-byte matrix at `this` with the
/// rotation about z by `angle`.
const NI_MATRIX_FROM_Z_ANGLE: u32 = 0x004a_0c90;
/// `thiscall` on a node (`matrix`): copies the 36 bytes to `this + 0x34`.
const NODE_SET_LOCAL_ROTATE: u32 = 0x0043_fa80;
/// `thiscall` on a node (`vector`): copies 12 bytes to `this + 0x58`.
const NODE_SET_LOCAL_TRANSLATE: u32 = 0x0044_0460;
/// `thiscall` on a node (`x, y, z`): stores the vector at `this + 0x58`.
const NODE_SET_LOCAL_TRANSLATE_XYZ: u32 = 0x004b_c1f0;
/// `thiscall` on a node (`update data`): calls its virtual slot `0xa4`
/// (`data, 0`) and, when `this + 0x18` is set, slot `0xfc` of that object.
const NODE_UPDATE_WITH_DATA: u32 = 0x00a5_9c60;
/// `NiAVObject::UpdateProperties` (Xbox PDB), `thiscall`.
const NODE_UPDATE_PROPERTIES: u32 = 0x00a5_a040;
/// Constructor of a 9-byte object, `thiscall` (`float`, `byte`, `byte`).
const UPDATE_DATA_CONSTRUCT: u32 = 0x0043_d410;
/// `TES::AddTempDebugObject` (Xbox PDB), `thiscall` on the `TES` (`node,
/// seconds`).
const TES_ADD_TEMP_DEBUG_OBJECT: u32 = 0x0045_8e20;
/// `NiTriShape::NiTriShape` (Xbox PDB), `thiscall` (`vertex count,
/// positions, normals, colours, uvs, 1, 0, triangle count, indices`).
const NI_TRI_SHAPE_CONSTRUCT: u32 = 0x00a7_4410;
/// `NiTexturingProperty` constructor, `thiscall`.
const NI_TEXTURING_PROPERTY_CONSTRUCT: u32 = 0x00a6_aa40;
/// `thiscall` on the texturing property (`0`): sets a value of its base map
/// (the engine map names it `NiTexturingProperty::SetBaseClampMode`).
const NI_TEXTURING_PROPERTY_SET_BASE_CLAMP_MODE: u32 = 0x004f_3200;
/// `thiscall` on the texturing property (`2`): calls
/// `00439360(this, 2, 0xe, 1)`.
const FN_00533FB0: u32 = 0x0053_3fb0;
/// `thiscall` on a shape (`property`): attaches the property (engine map
/// name `NiAVObject::AttachProperty`).
const NI_AV_OBJECT_ATTACH_PROPERTY: u32 = 0x0043_9410;
/// `NiPointer` constructor taking the pointer, `thiscall` (`pointer`).
const NI_POINTER_INIT: u32 = 0x0063_3c90;
/// `MakeTriangle` (Xbox PDB), `cdecl` (three vectors by value, `colour`,
/// `1`): a coloured triangle node.
const MAKE_TRIANGLE: u32 = 0x004b_3570;
/// Address of element `index` (`base + index * 4`) of the array at `this`,
/// `thiscall` (`index`).
const ARRAY_ELEMENT_ADDRESS: u32 = 0x0087_7a30;
/// `thiscall` on an array (`index, &value`): stores the element.
const ARRAY_SET_ELEMENT: u32 = 0x0096_ae90;
/// Constructor of a 0x10-byte object (the map of a texturing property),
/// `thiscall`.
const TEXTURE_MAP_CONSTRUCT: u32 = 0x00a6_9dd0;
/// `thiscall` on that object (`texture`): stores the texture through
/// `NI_POINTER_SET`.
const TEXTURE_MAP_SET_TEXTURE: u32 = 0x004d_c540;

// Cells, references and the player.
/// `TESObjectREFR::GetInterior` (Xbox PDB), `thiscall`.
const REFR_GET_INTERIOR: u32 = 0x0057_5d10;
/// `TESObjectREFR::GetWorldSpace` (Xbox PDB), `thiscall`.
const REFR_GET_WORLD_SPACE: u32 = 0x0057_5d70;
/// `TESWorldSpace::GetCellFromCellCoord` (Xbox PDB), `thiscall` (`x, y`).
const WORLD_SPACE_GET_CELL_FROM_CELL_COORD: u32 = 0x0058_75a0;
/// The seen data of a cell (`ExtraDataList::GetSeenData` of its extra data
/// list), `thiscall` on the cell.
const CELL_GET_SEEN_DATA: u32 = 0x0055_5bc0;
/// `TESObjectCELL::AdjustCoordForNorthRotation` (Xbox PDB), `thiscall`
/// (`source vector, destination vector, 1`).
const CELL_ADJUST_COORD_FOR_NORTH_ROTATION: u32 = 0x0055_5b10;
/// `TESObjectCELL::GetIntSeenSection` (Xbox PDB), `thiscall` (`x, y, 0`).
const CELL_GET_INT_SEEN_SECTION: u32 = 0x0055_6ef0;
/// `TESObjectCELL::GetInteriorLocalMapTexture` (Xbox PDB), `thiscall` (`x,
/// y, texture slot`).
const CELL_GET_INTERIOR_LOCAL_MAP_TEXTURE: u32 = 0x0054_e750;
/// `thiscall` on a cell (`texture slot`).
const FN_0054E640: u32 = 0x0054_e640;
/// `TESObjectCELL::GetSeenValue` (Xbox PDB), `cdecl` (`vector`): an `int`.
const CELL_GET_SEEN_VALUE: u32 = 0x0055_6870;
/// `TESObjectCELL::GetDataX` (Xbox PDB), `thiscall`.
const CELL_GET_DATA_X: u32 = 0x0054_4c30;
/// `TESObjectCELL::GetDataY` (Xbox PDB), `thiscall`.
const CELL_GET_DATA_Y: u32 = 0x0054_4c60;
/// Bit 0 of the byte at `+0x24` (PC offset): the cell is an interior.
const CELL_IS_INTERIOR: u32 = 0x0042_5fd0;
/// `float` result in `ST0`: the north rotation of an interior cell, 0 for an
/// exterior.
const CELL_GET_NORTH_ROTATION: u32 = 0x0055_5ad0;
/// `thiscall` on the `TES` (`x, y`): the address of the cell slot of a grid
/// position.
const TES_GET_GRID_CELL_SLOT: u32 = 0x0045_7050;
/// Virtual slot of the player that returns the address of its position
/// vector.
const PLAYER_POSITION_SLOT: u32 = 0x1f4;
/// Virtual slot of the seen data that draws it (`node, vector, 1`).
const SEEN_DATA_DRAW_SLOT: u32 = 0x4;
/// Virtual slot of a node that attaches a child (`child, 1`).
const NODE_ATTACH_CHILD_SLOT: u32 = 0xdc;
/// `thiscall` on the player: `this + 0x24` (PC offset).
const FN_00430830: u32 = 0x0043_0830;
/// Float to `int`, `cdecl` (`float`), rounding like `FISTP`.
const FLOAT_TO_INT: u32 = 0x0040_6d90;
/// `float` result in `ST0`: the global `011a31e8`.
const GET_LOCAL_MAP_SCALE: u32 = 0x0087_9d90;
/// `thiscall` on a vector (`other`): adds `other` in place.
const POINT3_ADD_ASSIGN: u32 = 0x0063_c8a0;
/// `thiscall` on a vector (`out, reference`): `out = this - reference`.
const POINT3_SUBTRACT: u32 = 0x0043_9ef0;
/// The `uGridsToLoad` setting (`Setting` object at `011c63cc`).
const SETTING_GRIDS_TO_LOAD: u32 = 0x011c_63cc;
/// Address of the value of an integer setting (`this + 4`), `thiscall`.
const SETTING_INT_VALUE_ADDRESS: u32 = 0x0043_d4d0;
/// `cdecl` (`&a, &b, 1, 0`): fills two `float`s; the second is a size in
/// bytes.
const FN_004A8BB0: u32 = 0x004a_8bb0;

// The interface.
/// `Interface::ToggleFullHelp` (Xbox PDB), `cdecl`.
const INTERFACE_TOGGLE_FULL_HELP: u32 = 0x0070_3100;
/// `Interface::GetFullHelp` (Xbox PDB), `cdecl`.
const INTERFACE_GET_FULL_HELP: u32 = 0x0070_3150;
/// `Interface::ToggleSafeZone` (Xbox PDB), `cdecl` (`2`).
const INTERFACE_TOGGLE_SAFE_ZONE: u32 = 0x0070_38e0;
/// `cdecl` (`flag`).
const FN_00703810: u32 = 0x0070_3810;
/// `MobileObject::GetCharController` (Xbox PDB), `thiscall`.
const MOBILE_OBJECT_GET_CHAR_CONTROLLER: u32 = 0x0093_06d0;
/// `bhkCharacterController::SetShapeType` (Xbox PDB), `thiscall` (`type`).
const CHAR_CONTROLLER_SET_SHAPE_TYPE: u32 = 0x00c7_0830;

// Globals of the second batch.
/// Byte: `1` when the grass display is on.
const GRASS_DISPLAY_FLAG: u32 = 0x0118_c000;
/// Three words (a vector) in the data section.
const START_VECTOR: u32 = 0x011f_426c;
/// Byte: the NPC facial emotions flag.
const EMOTIONS_FLAG: u32 = 0x0119_b4e0;
/// Pointer to the name of `Fallout.ini`.
const FALLOUT_INI_NAME: u32 = 0x011a_2ff0;
/// Pointer to the name of `Custom.ini`.
const CUSTOM_INI_NAME: u32 = 0x011a_2ff4;
/// Pointer to the name of `FalloutPrefs.ini`.
const FALLOUT_PREFS_INI_NAME: u32 = 0x011a_2ff8;
/// Byte: the conversation stats flag.
const CONVERSATION_STATS_FLAG: u32 = 0x011c_beac;
/// Byte: the magic stats flag.
const MAGIC_STATS_FLAG: u32 = 0x011c_3534;
/// Byte set to 1 while `005b9240` runs `00456a60`.
const FLAG_012680FC: u32 = 0x0126_80fc;
/// Byte: the borders flag.
const BORDERS_FLAG: u32 = 0x011c_ae44;
/// Byte: the projectile debug flag.
const PROJECTILE_DEBUG_FLAG: u32 = 0x011f_20a4;
/// Byte: the menus flag.
const MENUS_FLAG: u32 = 0x0118_c6f0;
/// Four words: a colour.
const DEFAULT_TILE_COLOUR: u32 = 0x011a_9be0;
/// Nine words: a 3x3 matrix.
const MATRIX_011A9448: u32 = 0x011a_9448;
/// `double` constants in `.rdata`: 0.0, -1.0, 2.0, 4.0, 10.0, 4096.0,
/// 1048576.0.
const DOUBLE_ZERO: u32 = 0x0101_2060;
const DOUBLE_MINUS_ONE: u32 = 0x0101_a6b0;
const DOUBLE_TWO: u32 = 0x0101_1590;
const DOUBLE_FOUR: u32 = 0x0101_db80;
const DOUBLE_TEN: u32 = 0x0102_0758;
const DOUBLE_4096: u32 = 0x0101_7a10;
const DOUBLE_MEGABYTE: u32 = 0x0101_ece0;
/// `float` constants: the seconds a debug object stays, and the corners of
/// the player marker triangle.
const DEBUG_OBJECT_SECONDS: u32 = 0x0101_8f5c;
const MARKER_TOP_Y: u32 = 0x0101_6088;
const MARKER_RIGHT_X: u32 = 0x0101_6248;
const MARKER_BOTTOM: u32 = 0x0102_295c;

// Strings of the second batch.
/// `"Script processing %s"`
const MSG_SCRIPT_PROCESSING: u32 = 0x0103_9830;
/// `"Grass Display %s"`
const MSG_GRASS_DISPLAY: u32 = 0x0103_9848;
/// `"Disabled."`
const TEXT_DISABLED_CAPITAL: u32 = 0x0103_985c;
/// `"Enabled."`
const TEXT_ENABLED_CAPITAL: u32 = 0x0103_9868;
/// `"TestAllCells %s"`
const MSG_TEST_ALL_CELLS: u32 = 0x0103_9874;
/// `"stopped"`
const TEXT_STOPPED: u32 = 0x0103_9884;
/// `"running"`
const TEXT_RUNNING: u32 = 0x0103_988c;
/// `"RenderTestCell failed: no cell"`
const MSG_RENDER_TEST_CELL_FAILED: u32 = 0x0103_9894;
/// `"RenderTestCell complete. Check warnings file for more info."`
const MSG_RENDER_TEST_CELL_COMPLETE: u32 = 0x0103_98b4;
/// `"The in-game settings have been refreshed from the Fallout.ini file."`
const MSG_INI_REFRESHED: u32 = 0x0103_98f0;
/// `"NPC Facial Emotions %s"`
const MSG_NPC_EMOTIONS: u32 = 0x0103_9934;
/// `"M# for loaded area = %.0f MB"`
const MSG_LOADED_AREA_MEGABYTES: u32 = 0x0103_994c;
/// `"Conversation stats %s"`
const MSG_CONVERSATION_STATS: u32 = 0x0103_996c;
/// `"Toggle Full Help %s"`
const MSG_TOGGLE_FULL_HELP: u32 = 0x0103_9984;
/// `"Magic stats %s"`
const MSG_MAGIC_STATS: u32 = 0x0103_9998;
/// `"Borders -> %s"`
const MSG_BORDERS: u32 = 0x0103_99a8;
/// `"Off"`
const TEXT_OFF_CAPITAL: u32 = 0x0103_99b8;
/// `"On"`
const TEXT_ON_CAPITAL: u32 = 0x0103_99bc;
/// `"Debug lines/shapes now show for projectiles."`
const MSG_PROJECTILE_DEBUG_1: u32 = 0x0103_9a88;
/// `"    -Yellow line for targeting."`
const MSG_PROJECTILE_DEBUG_2: u32 = 0x0103_9a68;
/// `"    -Red diamond for spawn point."`
const MSG_PROJECTILE_DEBUG_3: u32 = 0x0103_9a44;
/// `"    -Blue diamond for non-supersonic/non-hit-scan projectile sound start."`
const MSG_PROJECTILE_DEBUG_4: u32 = 0x0103_99f8;
/// `"    -Teal diamond for near miss sound from supersonic."`
const MSG_PROJECTILE_DEBUG_5: u32 = 0x0103_99c0;
/// `"Menus -> %s"`
const MSG_MENUS: u32 = 0x0103_9ab8;

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

/// Whether the commands echo their result to the console: the byte at
/// `+0x268` of the TLS block.
fn echo_enabled(e: &mut Engine) -> bool {
    let tls = e.tls();
    e.mem.u8(tls + TLS_ECHO) != 0
}

/// An `int` stored in a `float` the way `FILD` then `FSTP` does it.
fn float_from_int(value: i32) -> f32 {
    value as f32
}

/// `NiPoint3::NiPoint3(x, y, z)`: builds the point in a 12-byte block of its
/// own and returns its three words.
fn make_point(e: &mut Engine, x: f32, y: f32, z: f32) -> [u32; 3] {
    e.with_stack(12, |e, block| {
        let point = e.call(NI_POINT3_CONSTRUCT, &args![block, x, y, z]).u32();
        [e.mem.u32(point), e.mem.u32(point + 4), e.mem.u32(point + 8)]
    })
}

/// `new NiNode(0)`: the allocation (0xac bytes) and the constructor, zero
/// when the allocation failed.
fn new_ni_node(e: &mut Engine) -> u32 {
    let block = e.call(NI_ALLOC, &args![0xacu32]).u32();
    if block == 0 {
        0
    } else {
        e.call(NI_NODE_CONSTRUCT, &args![block, 0u32]).u32()
    }
}

/// The end the two debug drawings share: an update-data object (`0.0, 0, 0`)
/// on the stack, the node updated with it and its properties updated, and the
/// node handed to `TES::AddTempDebugObject` for the time in
/// [`DEBUG_OBJECT_SECONDS`].
fn show_debug_node(e: &mut Engine, node: u32) {
    e.with_stack(12, |e, data| {
        e.call(UPDATE_DATA_CONSTRUCT, &args![data, 0.0f32, 0u32, 0u32]);
        e.call(NODE_UPDATE_WITH_DATA, &args![node, data]);
        e.call(NODE_UPDATE_PROPERTIES, &args![node]);
        let seconds = e.global::<f32>(DEBUG_OBJECT_SECONDS);
        let tes = e.global::<u32>(GLOBAL_0011DEA10);
        e.call(TES_ADD_TEMP_DEBUG_OBJECT, &args![tes, node, seconds]);
    });
}

/// The value of the `uGridsToLoad` setting (`SETTING_GRIDS_TO_LOAD`), read
/// through the setting's address getter every time the game does.
fn grids_to_load(e: &mut Engine) -> u32 {
    let value = e
        .call(SETTING_INT_VALUE_ADDRESS, &args![SETTING_GRIDS_TO_LOAD])
        .u32();
    e.mem.u32(value)
}

/// `FISTP` of a `float` through `00406d90`.
fn float_to_int(e: &mut Engine, value: f32) -> i32 {
    e.call(FLOAT_TO_INT, &args![value]).i32()
}

/// An array of `count` elements of `size` bytes: `operator new` of
/// `count * size` (all ones when that overflows) and the element constructor
/// loop (`00401050`) when the block came back.
fn new_element_array(e: &mut Engine, count: u32, size: u32, constructor: u32) -> u32 {
    let bytes = count.saturating_mul(size);
    let block = e.call(OPERATOR_NEW, &args![bytes]).u32();
    if block != 0 {
        e.call(
            VECTOR_CONSTRUCT_SIMPLE,
            &args![block, size, count, constructor],
        );
    }
    block
}

/// `NiPoint3::NiPoint3(x, y, z)` in a 12-byte block of its own that lives
/// while `f` runs with the block's address (a point the game keeps on its
/// stack and passes by address).
fn with_point<R>(
    e: &mut Engine,
    x: f32,
    y: f32,
    z: f32,
    f: impl FnOnce(&mut Engine, u32) -> R,
) -> R {
    e.with_stack(12, |e, block| {
        e.call(NI_POINT3_CONSTRUCT, &args![block, x, y, z]);
        f(e, block.addr())
    })
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

// Translated from 005b7070 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::ToggleScriptsFunction` (Xbox PDB): flips script processing and
/// prints the new state.
pub fn script_toggle_scripts_function(e: &mut Engine) -> bool {
    let processing = e.call(GET_PROCESS_SCRIPTS, &args![]).bool();
    e.call(SET_PROCESS_SCRIPTS, &args![!processing]);
    let state = if e.call(GET_PROCESS_SCRIPTS, &args![]).bool() {
        TEXT_ENABLED
    } else {
        TEXT_DISABLED
    };
    console_print(e, &args![MSG_SCRIPT_PROCESSING, state]);
    true
}

// Translated from 005b70c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::ToggleGrassFunction` (Xbox PDB): when the grass object
/// (`0054f4c0`) exists, flips its flag (`00450f90`) and keeps the byte
/// [`GRASS_DISPLAY_FLAG`] at the opposite value. Turning the flag on runs
/// `00b62a20`; turning it off takes the animation data of the object behind
/// `0045c670`, and when there is one passes the vector `0045bb80` returns to
/// `0057d0a0`. Prints "Grass Display Enabled." or "Disabled.".
pub fn script_toggle_grass_function(e: &mut Engine) -> bool {
    let object = e.call(FN_0054F4C0, &args![]).u32();
    if object == 0 {
        return true;
    }
    let shown = e.call(FN_00456610, &args![object]).bool();
    let flag = !shown;
    e.call(FN_00450F90, &args![object, flag]);
    e.set_global(GRASS_DISPLAY_FLAG, (!flag) as u8);
    if flag {
        e.call(FN_00B62A20, &args![]);
    } else {
        let holder = e.call(FN_0045C670, &args![]).u32();
        let animation = e.call(FN_006629F0, &args![holder]).u32();
        if animation != 0 {
            let start = [
                e.global::<u32>(START_VECTOR),
                e.global::<u32>(START_VECTOR + 4),
                e.global::<u32>(START_VECTOR + 8),
            ];
            let vector = e
                .call(
                    FN_0045BB80,
                    &args![animation, start[0], start[1], start[2], 0.0f32],
                )
                .u32();
            let words = [
                e.mem.u32(vector),
                e.mem.u32(vector + 4),
                e.mem.u32(vector + 8),
            ];
            e.call(FN_0057D0A0, &args![words[0], words[1], words[2]]);
        }
    }
    let state = if flag {
        TEXT_DISABLED_CAPITAL
    } else {
        TEXT_ENABLED_CAPITAL
    };
    console_print(e, &args![MSG_GRASS_DISPLAY, state]);
    true
}

// Translated from 005b71b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::TestAllCellsFunction` (Xbox PDB): one optional argument, the
/// number of cells (0, the default, becomes -1), passed to
/// `TES::TestAllCells`; then prints whether the cell test is "running" or
/// "stopped".
pub fn script_test_all_cells_function(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([mut count]) = a.parse_into(e, [0]) else {
        return false;
    };
    if count == 0 {
        count = u32::MAX;
    }
    let tes = e.global::<u32>(GLOBAL_0011DEA10);
    e.call(TES_TEST_ALL_CELLS, &args![tes, count]);
    let tes = e.global::<u32>(GLOBAL_0011DEA10);
    let state = if e.call(TES_IS_RUNNING_CELL_TESTS, &args![tes]).bool() {
        TEXT_RUNNING
    } else {
        TEXT_STOPPED
    };
    console_print(e, &args![MSG_TEST_ALL_CELLS, state]);
    true
}

/// The body both `RenderTestCell` commands share: renders the player's
/// parent cell with `flag` and prints the outcome.
fn render_test_cell_command(e: &mut Engine, flag: u32) -> bool {
    let player = player(e);
    let cell = e.call(REFR_GET_PARENT_CELL, &args![player]).u32();
    if cell == 0 {
        console_print(e, &args![MSG_RENDER_TEST_CELL_FAILED]);
    } else {
        let cell = e.call(REFR_GET_PARENT_CELL, &args![player]).u32();
        e.call(CELL_RENDER_TEST_CELL, &args![cell, 0u32, flag]);
        console_print(e, &args![MSG_RENDER_TEST_CELL_COMPLETE]);
    }
    true
}

// Translated from 005b7250 (decompiled, FalloutNV.exe 1.4.0.525)
/// The "RenderTestCell" command with the flag 1: `TESObjectCELL::RenderTestCell`
/// on the player's parent cell, or "RenderTestCell failed: no cell".
pub fn fn_005b7250(e: &mut Engine) -> bool {
    render_test_cell_command(e, 1)
}

// Translated from 005b72a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The "RenderTestCell" command with the flag 0 (see [`fn_005b7250`]).
pub fn fn_005b72a0(e: &mut Engine) -> bool {
    render_test_cell_command(e, 0)
}

// Translated from 005b72f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Reloads the ini files: for each of `Fallout.ini`, `Custom.ini`
/// (`0044f560` collection) and `FalloutPrefs.ini` (`004de490` collection)
/// whose name pointer is set, builds `<game directory><name>` in a 0x104-byte
/// buffer (`strcpy_s`, `strcat_s`) and hands it to `005e0200` on the
/// collection. Prints that the settings were refreshed. (The stack protector
/// cookie check is left out.)
pub fn fn_005b72f0(e: &mut Engine) -> bool {
    for (name_pointer, collection_getter) in [
        (FALLOUT_INI_NAME, FN_0044F560),
        (CUSTOM_INI_NAME, FN_0044F560),
        (FALLOUT_PREFS_INI_NAME, FN_004DE490),
    ] {
        let name = e.global::<u32>(name_pointer);
        if name != 0 {
            e.with_stack(0x104, |e, path| {
                let directory = e.call(FN_004DC110, &args![]).u32();
                e.call(STRCPY_S, &args![path, 0x104u32, directory]);
                e.call(STRCAT_S, &args![path, 0x104u32, name]);
                let collection = e.call(collection_getter, &args![]).u32();
                e.call(FN_005E0200, &args![collection, path]);
            });
        }
    }
    console_print(e, &args![MSG_INI_REFRESHED]);
    true
}

// Translated from 005b7420 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::ToggleEmotionsFunction` (Xbox PDB): flips the NPC facial
/// emotions flag and prints "Enabled." or "Disabled.".
pub fn script_toggle_emotions_function(e: &mut Engine) -> bool {
    let flag = fn_005b7470(e);
    fn_005b7480(e, (flag == 0) as u8);
    let state = if fn_005b7470(e) != 0 {
        TEXT_ENABLED_CAPITAL
    } else {
        TEXT_DISABLED_CAPITAL
    };
    console_print(e, &args![MSG_NPC_EMOTIONS, state]);
    true
}

// Translated from 005b7470 (decompiled, FalloutNV.exe 1.4.0.525)
/// The NPC facial emotions flag (byte global `0119b4e0`).
pub fn fn_005b7470(e: &mut Engine) -> u8 {
    e.global::<u8>(EMOTIONS_FLAG)
}

// Translated from 005b7480 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the NPC facial emotions flag (byte global `0119b4e0`).
pub fn fn_005b7480(e: &mut Engine, value: u8) {
    e.set_global(EMOTIONS_FLAG, value);
}

/// Reloads a form from the file it came from, as `ReloadCurrentClimate` and
/// `005b7520` do: the form's last file (`TESForm::GetFile(-1)`) made thread
/// safe, opened (`OpenTES(0, 0)`), the form found in it, then virtual slots
/// `0x18` (clear) and `0x20` (load, with the file). Returns false when the
/// form has no file.
fn reload_form_from_file(e: &mut Engine, form: u32) -> bool {
    let file = e.call(FORM_GET_FILE, &args![form, u32::MAX]).u32();
    if file == 0 {
        return false;
    }
    let file = e.call(FILE_GET_THREAD_SAFE_FILE, &args![file]).u32();
    e.call(FILE_OPEN_TES, &args![file, 0u32, 0u32]);
    e.call(FILE_FIND_FORM, &args![file, form]);
    e.vcall(form, 0x18, &args![]);
    e.vcall(form, 0x20, &args![file]);
    true
}

// Translated from 005b7490 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::ReloadCurrentClimate` (Xbox PDB): reloads the sky's current
/// climate from its file and makes it the sky's current climate again
/// (`Sky::SetCurrentClimate(climate, 1)`).
pub fn script_reload_current_climate(e: &mut Engine) -> bool {
    let sky = e.call(SKY_GET_INSTANCE, &args![]).u32();
    let climate = e.call(SKY_GET_CURRENT_CLIMATE, &args![sky]).u32();
    if reload_form_from_file(e, climate) {
        let sky = e.call(SKY_GET_INSTANCE, &args![]).u32();
        e.call(SKY_SET_CURRENT_CLIMATE, &args![sky, climate, 1u32]);
    }
    true
}

// Translated from 005b7520 (decompiled, FalloutNV.exe 1.4.0.525)
/// Reloads the sky's current weather from its file.
pub fn fn_005b7520(e: &mut Engine) -> bool {
    let sky = e.call(SKY_GET_INSTANCE, &args![]).u32();
    let weather = e.call(SKY_GET_CURRENT_WEATHER, &args![sky]).u32();
    reload_form_from_file(e, weather);
    true
}

// Translated from 005b7590 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the weather: arguments `weather` and `override`. While the sky has a
/// current weather, the weather becomes the override weather
/// (`Sky + 0x1c`) when `override` is set, otherwise the default weather
/// (`Sky + 0x18`, followed by `0093a7a0(0)` on the player); then, unless the
/// TLS echo flag is set, `0063e860(1)` on the sky. Without a current weather
/// it is `Sky::ForceWeather(weather, override != 0)`.
pub fn fn_005b7590(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([weather, override_flag]) = a.parse_into(e, [0, 0]) else {
        return false;
    };
    let sky = e.call(SKY_GET_INSTANCE, &args![]).u32();
    if e.call(SKY_GET_CURRENT_WEATHER, &args![sky]).u32() != 0 {
        if override_flag != 0 {
            e.mem.set_u32(sky + SKY_OVERRIDE_WEATHER, weather);
        } else {
            e.mem.set_u32(sky + SKY_DEFAULT_WEATHER, weather);
            let player = player(e);
            e.call(FN_0093A7A0, &args![player, 0u32]);
        }
        if !echo_enabled(e) {
            e.call(FN_0063E860, &args![sky, 1u32]);
        }
    } else {
        e.call(SKY_FORCE_WEATHER, &args![sky, weather, override_flag != 0]);
    }
    true
}

// Translated from 005b7660 (decompiled, FalloutNV.exe 1.4.0.525)
/// Forces the weather: arguments `weather` and `flag`, passed to
/// `Sky::ForceWeather` on the sky instance.
pub fn fn_005b7660(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([weather, flag]) = a.parse_into(e, [0, 0]) else {
        return false;
    };
    let sky = e.call(SKY_GET_INSTANCE, &args![]).u32();
    e.call(SKY_FORCE_WEATHER, &args![sky, weather, flag != 0]);
    true
}

// Translated from 005b76d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Clears the sky's override weather (`Sky + 0x1c`).
pub fn fn_005b76d0(e: &mut Engine) -> bool {
    let sky = e.call(SKY_GET_INSTANCE, &args![]).u32();
    e.call(SKY_CLEAR_OVERRIDE_WEATHER, &args![sky]);
    true
}

// Translated from 005b76f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::ApplyImageSpaceModifier` (Xbox PDB): arguments the modifier form
/// and a strength (default 1.0); calls
/// `ImageSpaceModifierInstanceForm::Trigger(form, strength, 0)`.
pub fn script_apply_image_space_modifier(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([form, strength]) = a.parse_into(e, [0, 1.0f32.to_bits()]) else {
        return false;
    };
    e.call(IMAGE_SPACE_MODIFIER_TRIGGER, &args![form, strength, 0u32]);
    true
}

// Translated from 005b7760 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::RemoveImageSpaceModifier` (Xbox PDB): arguments the modifier form
/// and a strength (default 1.0, unused); calls
/// `ImageSpaceModifierInstanceForm::Stop(form)`.
pub fn script_remove_image_space_modifier(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([form, _strength]) = a.parse_into(e, [0, 1.0f32.to_bits()]) else {
        return false;
    };
    e.call(IMAGE_SPACE_MODIFIER_STOP, &args![form]);
    true
}

// Translated from 005b77c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Two arguments: calls `00547750(first, second)` (`thiscall` on the first).
pub fn fn_005b77c0(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([first, second]) = a.parse_into(e, [0, 0]) else {
        return false;
    };
    e.call(FN_00547750, &args![first, second]);
    true
}

// Translated from 005b7820 (decompiled, FalloutNV.exe 1.4.0.525)
/// One argument, an object with a list at `+0x18` (or none): sets the image
/// space manager's `+0xb0` to that list's address (0 for none), then calls
/// `00b8b500` on the manager.
pub fn fn_005b7820(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([object]) = a.parse_into(e, [0]) else {
        return false;
    };
    let value = if object != 0 {
        e.call(FORM_LIST_ITEMS, &args![object]).u32()
    } else {
        0
    };
    let manager = e.call(GET_IMAGE_SPACE_MANAGER, &args![]).u32();
    e.call(IMAGE_SPACE_MANAGER_SET_OVERRIDE, &args![manager, value]);
    let manager = e.call(GET_IMAGE_SPACE_MANAGER, &args![]).u32();
    e.call(FN_00B8B500, &args![manager]);
    true
}

// Translated from 005b78a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Draws the seen data of the cells around the player as a debug object (a
/// `NiNode`). In an interior it asks the parent cell's seen data (`00555bc0`)
/// to draw itself at the point `(0, 0, player z)` (virtual slot `4` of the
/// seen data with the node, the point and 1) and, when the cell has a north
/// rotation (`00555ad0` times -1) other than 0, rotates the node by it
/// (`004a0c90` into the identity matrix `011a9448`, `0043fa80`). In an
/// exterior it walks the `uGridsToLoad` squared grid cells of the `TES`
/// (`00457050`) and has each one's seen data draw itself at the cell's
/// corner `(GetDataX << 12, GetDataY << 12, player z)`. The node is finished
/// by [`show_debug_node`]. (The exception frame around the node allocation
/// is left out.)
pub fn fn_005b78a0(e: &mut Engine) -> bool {
    let node = new_ni_node(e);
    let player = player(e);
    let parent_cell = e.call(REFR_GET_PARENT_CELL, &args![player]).u32();
    if e.call(CELL_IS_INTERIOR, &args![parent_cell]).bool() {
        let seen = e.call(CELL_GET_SEEN_DATA, &args![parent_cell]).u32();
        if seen != 0 {
            let position = e.vcall(player, PLAYER_POSITION_SLOT, &args![]).u32();
            let z = e.mem.f32(position + 8);
            with_point(e, 0.0, 0.0, z, |e, point| {
                e.vcall(seen, SEEN_DATA_DRAW_SLOT, &args![node, point, 1u32]);
            });
            let rotation = e.call(CELL_GET_NORTH_ROTATION, &args![parent_cell]).f64();
            let angle = (rotation * e.global::<f64>(DOUBLE_MINUS_ONE)) as f32;
            if f64::from(angle) != e.global::<f64>(DOUBLE_ZERO) {
                e.with_stack(36, |e, matrix| {
                    let identity = e.mem.bytes(MATRIX_011A9448, 36);
                    e.mem.write(matrix.addr(), &identity);
                    e.call(NI_MATRIX_FROM_Z_ANGLE, &args![matrix, angle]);
                    e.call(NODE_SET_LOCAL_ROTATE, &args![node, matrix]);
                });
            }
        }
    } else {
        let mut outer = 0u32;
        while outer < grids_to_load(e) {
            let mut inner = 0u32;
            while inner < grids_to_load(e) {
                let tes = e.global::<u32>(GLOBAL_0011DEA10);
                let slot = e
                    .call(TES_GET_GRID_CELL_SLOT, &args![tes, outer, inner])
                    .u32();
                let cell = e.mem.u32(slot);
                let seen = if cell != 0 {
                    e.call(CELL_GET_SEEN_DATA, &args![cell]).u32()
                } else {
                    0
                };
                if seen != 0 {
                    let position = e.vcall(player, PLAYER_POSITION_SLOT, &args![]).u32();
                    let z = e.mem.f32(position + 8);
                    let y = e.call(CELL_GET_DATA_Y, &args![cell]).i32();
                    let y = float_from_int(y.wrapping_shl(12));
                    let x = e.call(CELL_GET_DATA_X, &args![cell]).i32();
                    let x = float_from_int(x.wrapping_shl(12));
                    with_point(e, x, y, z, |e, point| {
                        e.vcall(seen, SEEN_DATA_DRAW_SLOT, &args![node, point, 1u32]);
                    });
                }
                inner += 1;
            }
            outer += 1;
        }
    }
    show_debug_node(e, node);
    true
}

// ---- TestLocalMap (005b7b40) -------------------------------------------------

/// The numbers `TestLocalMap` fixes in its frame: one tile is 0x40 units
/// wide and has 0x10 x 0x10 quads (0x11 x 0x11 vertices), so a quad is
/// `0x40 / 0x10 = 4` units wide.
const LOCAL_MAP_TILE_SIZE: i32 = 0x40;
const LOCAL_MAP_QUADS_PER_SIDE: i32 = 0x10;
const LOCAL_MAP_QUAD_COUNT: i32 = LOCAL_MAP_QUADS_PER_SIDE * LOCAL_MAP_QUADS_PER_SIDE;
const LOCAL_MAP_QUAD_SIZE: i32 = LOCAL_MAP_TILE_SIZE / LOCAL_MAP_QUADS_PER_SIDE;
const LOCAL_MAP_VERTICES_PER_SIDE: i32 = 0x11;
const LOCAL_MAP_VERTEX_COUNT: u32 =
    (LOCAL_MAP_VERTICES_PER_SIDE * LOCAL_MAP_VERTICES_PER_SIDE) as u32;

/// What the loop body of `TestLocalMap` needs from the command's frame.
struct LocalMapFrame {
    /// The root node the tiles and the marker are attached to.
    node: u32,
    /// The command's argument: when set, the tile colours are the seen values.
    use_seen_values: bool,
    /// The player.
    player: u32,
    /// Address of the player's position copy (x, y, z + 10.0).
    position: u32,
    /// `uGridsToLoad`: tiles per side.
    grid: i32,
    /// Width of all tiles together (`0x40 * grid`).
    extent: i32,
    /// The float `00879d90` returns: the distance of two colour samples.
    scale: f32,
}

// Translated from 005b7b40 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::TestLocalMap` (Xbox PDB): builds a debug picture of the local map
/// around the player and shows it as a temporary debug object.
///
/// One argument, `use_seen_values`. A root `NiNode` and the player's
/// position (virtual slot `0x1f4`, z raised by 10.0) are taken first. Then
/// for every cell of the `uGridsToLoad` squared grid ([`local_map_tile`])
/// a 17 x 17 vertex tile `NiTriShape` is built, attached to the root and
/// moved to its place, and given the cell's local map texture as the base
/// map of a new `NiTexturingProperty`. Finally a red triangle (the player
/// marker, `MakeTriangle`) is attached at the player's place in the grid,
/// rotated by the cell's north rotation plus `Player + 0x24 + 8`, the root is
/// moved to the player's position and [`show_debug_node`] shows it.
///
/// The exception-unwinding states in the frame are not translated.
pub fn script_test_local_map(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([use_seen_values]) = a.parse_into(e, [0]) else {
        return false;
    };
    let node = new_ni_node(e);
    let player = player(e);
    // The player's position and z + 10.0 (a copy the draw calls take by address).
    let position = e.mem.alloc(12);
    let source = e.vcall(player, PLAYER_POSITION_SLOT, &args![]).u32();
    for i in 0..3 {
        let word = e.mem.u32(source + 4 * i);
        e.mem.set_u32(position + 4 * i, word);
    }
    let raised = (f64::from(e.mem.f32(position + 8)) + e.global::<f64>(DOUBLE_TEN)) as f32;
    e.mem.set_f32(position + 8, raised);

    let grid = grids_to_load(e) as i32;
    let frame = LocalMapFrame {
        node,
        use_seen_values: use_seen_values != 0,
        player,
        position,
        grid,
        extent: LOCAL_MAP_TILE_SIZE.wrapping_mul(grid),
        scale: e.call(GET_LOCAL_MAP_SCALE, &args![]).f32(),
    };
    let mut index = 0i32;
    while index < grid.wrapping_mul(grid) {
        local_map_tile(e, &frame, index);
        index += 1;
    }

    // The player marker.
    let marker_centre = e.mem.alloc(12);
    e.call(TRIVIAL_CONSTRUCT, &args![marker_centre]);
    let source = e.vcall(player, PLAYER_POSITION_SLOT, &args![]).u32();
    let player_position = e.mem.alloc(12);
    for i in 0..3 {
        let word = e.mem.u32(source + 4 * i);
        e.mem.set_u32(player_position + 4 * i, word);
    }
    if e.call(REFR_GET_INTERIOR, &args![player]).bool() {
        let adjusted = e.mem.alloc(12);
        e.call(TRIVIAL_CONSTRUCT, &args![adjusted]);
        let source = e.vcall(player, PLAYER_POSITION_SLOT, &args![]).u32();
        let cell = e.call(REFR_GET_PARENT_CELL, &args![player]).u32();
        e.call(
            CELL_ADJUST_COORD_FOR_NORTH_ROTATION,
            &args![cell, source, adjusted, 1u32],
        );
        let adjusted_x = e.mem.f32(adjusted);
        let cell_x = (float_to_int(e, adjusted_x).wrapping_sub(0x800)) >> 12;
        let adjusted_y = e.mem.f32(adjusted + 4);
        let cell_y = (float_to_int(e, adjusted_y).wrapping_sub(0x800)) >> 12;
        let centre = make_point(
            e,
            float_from_int(cell_x.wrapping_shl(12).wrapping_add(0x1000)),
            float_from_int(cell_y.wrapping_shl(12).wrapping_add(0x1000)),
            0.0,
        );
        copy_words(e, marker_centre, &centre);
        for i in 0..3 {
            let word = e.mem.u32(adjusted + 4 * i);
            e.mem.set_u32(player_position + 4 * i, word);
        }
        e.mem.free(adjusted);
    } else {
        let source = e.vcall(player, PLAYER_POSITION_SLOT, &args![]).u32();
        let source_x = e.mem.f32(source);
        let cell_x = float_to_int(e, source_x) >> 12;
        let source = e.vcall(player, PLAYER_POSITION_SLOT, &args![]).u32();
        let source_y = e.mem.f32(source + 4);
        let cell_y = float_to_int(e, source_y) >> 12;
        let centre = make_point(
            e,
            float_from_int(cell_x.wrapping_shl(12).wrapping_add(0x800)),
            float_from_int(cell_y.wrapping_shl(12).wrapping_add(0x800)),
            0.0,
        );
        copy_words(e, marker_centre, &centre);
    }
    e.mem.set_f32(player_position + 8, 0.0);
    // The marker's offset from the tile centre, in tile units.
    let offset = e.mem.alloc(12);
    e.call(
        POINT3_SUBTRACT,
        &args![player_position, offset, marker_centre],
    );
    let unit = (f64::from(LOCAL_MAP_TILE_SIZE) / e.global::<f64>(DOUBLE_4096)) as f32;
    for i in 0..2 {
        let scaled = (f64::from(e.mem.f32(offset + 4 * i)) * f64::from(unit)) as f32;
        e.mem.set_f32(offset + 4 * i, scaled);
    }
    e.mem.set_f32(offset + 8, 0.0);
    let (marker_top, marker_right, marker_bottom) = (
        e.global::<f32>(MARKER_TOP_Y),
        e.global::<f32>(MARKER_RIGHT_X),
        e.global::<f32>(MARKER_BOTTOM),
    );
    let top = make_point(e, 0.0, marker_top, 1.0);
    let right = make_point(e, marker_right, marker_bottom, 1.0);
    let left = make_point(e, marker_bottom, marker_bottom, 1.0);
    let colour_block = e.mem.alloc(16);
    let red = e
        .call(
            NI_COLOR_A_CONSTRUCT,
            &args![colour_block, 1.0f32, 0.0f32, 0.0f32, 0.0f32],
        )
        .u32();
    let triangle = e
        .call(
            MAKE_TRIANGLE,
            &args![
                top[0], top[1], top[2], right[0], right[1], right[2], left[0], left[1], left[2],
                red, 1u32
            ],
        )
        .u32();
    e.call(NODE_SET_LOCAL_TRANSLATE, &args![triangle, offset]);
    e.with_stack(36, |e, matrix| {
        let identity = e.mem.bytes(MATRIX_011A9448, 36);
        e.mem.write(matrix.addr(), &identity);
        let cell = e.call(REFR_GET_PARENT_CELL, &args![player]).u32();
        let north = e.call(CELL_GET_NORTH_ROTATION, &args![cell]).f64();
        let data = e.call(FN_00430830, &args![player]).u32();
        let angle = (f64::from(e.mem.f32(data + 8)) + north) as f32;
        e.call(NI_MATRIX_FROM_Z_ANGLE, &args![matrix, angle]);
        e.call(NODE_SET_LOCAL_ROTATE, &args![triangle, matrix]);
    });
    e.vcall(node, NODE_ATTACH_CHILD_SLOT, &args![triangle, 1u32]);
    e.call(NODE_SET_LOCAL_TRANSLATE, &args![node, position]);
    show_debug_node(e, node);
    for block in [
        position,
        marker_centre,
        player_position,
        offset,
        colour_block,
    ] {
        e.mem.free(block);
    }
    true
}

/// Copies the three words of a point to `address`.
fn copy_words(e: &mut Engine, address: u32, words: &[u32; 3]) {
    for (i, word) in words.iter().enumerate() {
        e.mem.set_u32(address + 4 * i as u32, *word);
    }
}

/// The loop body of `TestLocalMap` for the tile `index` (row-major in the
/// `uGridsToLoad` squared grid): finds the cell, builds the tile's vertex
/// arrays and index array, the `NiTriShape` and the texturing property.
fn local_map_tile(e: &mut Engine, frame: &LocalMapFrame, index: i32) {
    let grid = frame.grid;
    let player = frame.player;
    let divisor = grids_to_load(e);
    let tile_x = index as u32 % divisor;
    let divisor = grids_to_load(e);
    let tile_y = index as u32 / divisor;

    // The tile's texture, a `NiPointer` the game keeps on its stack.
    let texture_slot = e.mem.alloc(4);
    e.call(NI_POINTER_INIT, &args![texture_slot, 0u32]);
    // Where the tile starts in the world; the start vector unless a cell sets it.
    let origin = e.mem.alloc(12);
    for i in 0..3 {
        let word = e.global::<u32>(START_VECTOR + 4 * i);
        e.mem.set_u32(origin + 4 * i, word);
    }
    let half_grid = grid >> 1;
    if e.call(REFR_GET_INTERIOR, &args![player]).bool() {
        let cell = e.call(REFR_GET_PARENT_CELL, &args![player]).u32();
        if cell != 0 {
            let adjusted = e.mem.alloc(12);
            for i in 0..3 {
                let word = e.mem.u32(frame.position + 4 * i);
                e.mem.set_u32(adjusted + 4 * i, word);
            }
            e.call(
                CELL_ADJUST_COORD_FOR_NORTH_ROTATION,
                &args![cell, frame.position, adjusted, 1u32],
            );
            let adjusted_x = e.mem.f32(adjusted);
            let base_x = (float_to_int(e, adjusted_x).wrapping_sub(0x800)) >> 12;
            let adjusted_y = e.mem.f32(adjusted + 4);
            let base_y = (float_to_int(e, adjusted_y).wrapping_sub(0x800)) >> 12;
            e.mem.free(adjusted);
            let cell_x = base_x.wrapping_add(tile_x as i32).wrapping_sub(half_grid);
            let cell_y = base_y.wrapping_add(tile_y as i32).wrapping_sub(half_grid);
            let start = make_point(
                e,
                float_from_int(cell_x.wrapping_shl(12).wrapping_add(0x800)),
                float_from_int(cell_y.wrapping_shl(12).wrapping_add(0x800)),
                0.0,
            );
            copy_words(e, origin, &start);
            e.call(
                CELL_GET_INT_SEEN_SECTION,
                &args![cell, cell_x, cell_y, 0u32],
            );
            e.call(
                CELL_GET_INTERIOR_LOCAL_MAP_TEXTURE,
                &args![cell, cell_x, cell_y, texture_slot],
            );
        }
    } else {
        let world = e.call(REFR_GET_WORLD_SPACE, &args![player]).u32();
        let position_x = e.mem.f32(frame.position);
        let base_x = float_to_int(e, position_x) >> 12;
        let position_y = e.mem.f32(frame.position + 4);
        let base_y = float_to_int(e, position_y) >> 12;
        let cell_x = base_x.wrapping_add(tile_x as i32).wrapping_sub(half_grid);
        let cell_y = base_y.wrapping_add(tile_y as i32).wrapping_sub(half_grid);
        let start = make_point(
            e,
            float_from_int(cell_x.wrapping_shl(12)),
            float_from_int(cell_y.wrapping_shl(12)),
            0.0,
        );
        copy_words(e, origin, &start);
        let mut cell = 0;
        if world != 0 {
            cell = e
                .call(
                    WORLD_SPACE_GET_CELL_FROM_CELL_COORD,
                    &args![world, cell_x, cell_y],
                )
                .u32();
        }
        if cell != 0 {
            e.call(CELL_GET_SEEN_DATA, &args![cell]);
            e.call(FN_0054E640, &args![cell, texture_slot]);
        }
    }

    // The vertex arrays: positions, normals, uvs and colours.
    let positions = new_element_array(e, LOCAL_MAP_VERTEX_COUNT, 12, TRIVIAL_CONSTRUCT);
    let normals = new_element_array(e, LOCAL_MAP_VERTEX_COUNT, 12, TRIVIAL_CONSTRUCT);
    let uvs = new_element_array(e, LOCAL_MAP_VERTEX_COUNT, 8, TRIVIAL_CONSTRUCT);
    let colours = new_element_array(e, LOCAL_MAP_VERTEX_COUNT, 0x10, NI_COLOR_A_ZERO_CONSTRUCT);
    let indices = e
        .call(
            NI_ALLOC_SHORTS,
            &args![((LOCAL_MAP_QUAD_COUNT << 1) * 3) << 1],
        )
        .u32();

    let half_tile = f64::from(LOCAL_MAP_TILE_SIZE) / e.global::<f64>(DOUBLE_TWO);
    let side = LOCAL_MAP_VERTICES_PER_SIDE;
    let mut vertex = 0u32;
    for row in 0..side {
        for column in 0..side {
            let x = (f64::from(column * LOCAL_MAP_QUAD_SIZE) - half_tile) as f32;
            let y = (f64::from(row * LOCAL_MAP_QUAD_SIZE) - half_tile) as f32;
            let point = make_point(e, x, y, 0.0);
            copy_words(e, positions + 12 * vertex, &point);
            vertex += 1;
        }
    }
    let mut vertex = 0u32;
    for row in 0..side {
        for column in 0..side {
            let up = make_point(e, 0.0, 0.0, 1.0);
            copy_words(e, normals + 12 * vertex, &up);
            let v = (1.0 - f64::from(row) / f64::from(side)) as f32;
            let u = (f64::from(column) / f64::from(side)) as f32;
            let uv = e.with_stack(8, |e, block| {
                let result = e.call(NI_POINT2_CONSTRUCT, &args![block, u, v]).u32();
                [e.mem.u32(result), e.mem.u32(result + 4)]
            });
            e.mem.set_u32(uvs + 8 * vertex, uv[0]);
            e.mem.set_u32(uvs + 8 * vertex + 4, uv[1]);
            if !frame.use_seen_values {
                for i in 0..4 {
                    let word = e.global::<u32>(DEFAULT_TILE_COLOUR + 4 * i);
                    e.mem.set_u32(colours + 16 * vertex + 4 * i, word);
                }
            } else {
                let sample_x = (f64::from(column) * f64::from(frame.scale)) as f32;
                let sample_y = (f64::from(row) * f64::from(frame.scale)) as f32;
                let seen = e.with_stack(12, |e, sample| {
                    e.call(
                        NI_POINT3_CONSTRUCT,
                        &args![sample, sample_x, sample_y, 0.0f32],
                    );
                    e.call(POINT3_ADD_ASSIGN, &args![sample, origin]);
                    e.call(CELL_GET_SEEN_VALUE, &args![sample]).i32()
                });
                let value = float_from_int(seen);
                let shade = (f64::from(value) / e.global::<f64>(DOUBLE_FOUR)) as f32;
                let colour = e.with_stack(16, |e, block| {
                    let result = e
                        .call(
                            NI_COLOR_A_CONSTRUCT,
                            &args![block, shade, shade, shade, 0.0f32],
                        )
                        .u32();
                    [
                        e.mem.u32(result),
                        e.mem.u32(result + 4),
                        e.mem.u32(result + 8),
                        e.mem.u32(result + 12),
                    ]
                });
                for (i, word) in colour.iter().enumerate() {
                    e.mem.set_u32(colours + 16 * vertex + 4 * i as u32, *word);
                }
            }
            vertex += 1;
        }
    }

    // The index array: two triangles per quad, the diagonal alternating.
    let mut at = 0u32;
    for row in 0..LOCAL_MAP_QUADS_PER_SIDE {
        for column in 0..LOCAL_MAP_QUADS_PER_SIDE {
            let below = (row + 1) * side;
            let this_row = row * side;
            let corners: [i32; 6] = if row % 2 == column % 2 {
                [
                    below + column + 1,
                    below + column,
                    this_row + column,
                    this_row + column,
                    this_row + column + 1,
                    below + column + 1,
                ]
            } else {
                [
                    below + column,
                    this_row + column,
                    this_row + column + 1,
                    this_row + column + 1,
                    below + column + 1,
                    below + column,
                ]
            };
            for corner in corners {
                e.mem.set_u16(indices + 2 * at, corner as u16);
                at += 1;
            }
        }
    }

    // The shape, attached to the root and moved to the tile's place.
    let block = e.call(NI_ALLOC, &args![0xc4u32]).u32();
    let shape = if block == 0 {
        0
    } else {
        e.call(
            NI_TRI_SHAPE_CONSTRUCT,
            &args![
                block,
                LOCAL_MAP_VERTEX_COUNT & 0xffff,
                positions,
                normals,
                colours,
                uvs,
                1u32,
                0u32,
                (LOCAL_MAP_QUAD_COUNT << 1) as u32,
                indices
            ],
        )
        .u32()
    };
    e.vcall(frame.node, NODE_ATTACH_CHILD_SLOT, &args![shape, 1u32]);
    let half_extent = f64::from(frame.extent) / e.global::<f64>(DOUBLE_TWO);
    let offset = |tile: u32, e: &Engine| {
        let tile_offset = f64::from(LOCAL_MAP_TILE_SIZE.wrapping_mul(tile as i32));
        ((tile_offset + f64::from(LOCAL_MAP_TILE_SIZE) / e.global::<f64>(DOUBLE_TWO)) - half_extent)
            as f32
    };
    let y = offset(tile_y, e);
    let x = offset(tile_x, e);
    e.call(NODE_SET_LOCAL_TRANSLATE_XYZ, &args![shape, x, y, 0.0f32]);

    // The texture, as the base map of a new texturing property.
    if e.call(NI_POINTER_GET, &args![texture_slot]).u32() != 0 {
        let block = e.call(NI_ALLOC, &args![0x30u32]).u32();
        let property = if block == 0 {
            0
        } else {
            e.call(NI_TEXTURING_PROPERTY_CONSTRUCT, &args![block]).u32()
        };
        let texture = e.call(NI_POINTER_GET, &args![texture_slot]).u32();
        fn_005b8fc0(e, Ptr::new(property), texture);
        e.call(
            NI_TEXTURING_PROPERTY_SET_BASE_CLAMP_MODE,
            &args![property, 0u32],
        );
        e.call(FN_00533FB0, &args![property, 2u32]);
        e.call(NI_AV_OBJECT_ATTACH_PROPERTY, &args![shape, property]);
        e.call(NI_POINTER_SET, &args![texture_slot, 0u32]);
    }
    e.call(NI_POINTER_DESTRUCT, &args![texture_slot]);
    e.mem.free(texture_slot);
    e.mem.free(origin);
}

// Translated from 005b8fc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the texture of the base map of a texturing property: the first
/// element of the array at `this + 0x1c` (created by `00a69dd0` when it is
/// missing and stored through `0096ae90`), then `004dc540(map, texture)`.
/// (The same shape as the engine map's `NiTexturingProperty::SetBaseClampMode`
/// at `004f3200`.)
pub fn fn_005b8fc0(e: &mut Engine, this: Ptr, texture: u32) {
    let list = this.addr() + 0x1c;
    let slot = e.call(ARRAY_ELEMENT_ADDRESS, &args![list, 0u32]).u32();
    let existing = e.mem.u32(slot);
    let cell = e.mem.alloc(4);
    e.mem.set_u32(cell, existing);
    if existing == 0 {
        let block = e.call(OPERATOR_NEW, &args![0x10u32]).u32();
        let map = if block == 0 {
            0
        } else {
            e.call(TEXTURE_MAP_CONSTRUCT, &args![block]).u32()
        };
        e.mem.set_u32(cell, map);
        e.call(ARRAY_SET_ELEMENT, &args![list, 0u32, cell]);
    }
    let map = e.mem.u32(cell);
    e.call(TEXTURE_MAP_SET_TEXTURE, &args![map, texture]);
    e.mem.free(cell);
}

// Translated from 005b9070 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::TestCode` (Xbox PDB): one argument (unused), then asks `004a8bb0`
/// for the memory in use by the loaded area and prints it in megabytes
/// ("M# for loaded area = %.0f MB").
pub fn script_test_code(e: &mut Engine, a: ScriptArgs) -> bool {
    if a.parse_into(e, [0]).is_none() {
        return false;
    }
    e.with_stack(8, |e, outputs| {
        let size = outputs.addr() + 4;
        e.mem.set_f32(outputs.addr(), 0.0);
        e.mem.set_f32(size, 0.0);
        e.call(FN_004A8BB0, &args![outputs, size, 1u32, 0u32]);
        let bytes = e.mem.f32(size);
        let megabytes = (f64::from(bytes) / e.global::<f64>(DOUBLE_MEGABYTE)) as f32;
        e.mem.set_f32(size, megabytes);
        console_print(e, &args![MSG_LOADED_AREA_MEGABYTES, f64::from(megabytes)]);
    });
    true
}

// Translated from 005b9100 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::ToggleConversationStatsFunction` (Xbox PDB): flips the
/// conversation stats flag and prints "shown." or "hidden.".
pub fn script_toggle_conversation_stats_function(e: &mut Engine) -> bool {
    let flag = fn_005b9160(e);
    fn_005b9150(e, (flag == 0) as u8);
    let state = if fn_005b9160(e) != 0 {
        TEXT_SHOWN
    } else {
        TEXT_HIDDEN
    };
    console_print(e, &args![MSG_CONVERSATION_STATS, state]);
    true
}

// Translated from 005b9150 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the conversation stats flag (byte global `011cbeac`).
pub fn fn_005b9150(e: &mut Engine, value: u8) {
    e.set_global(CONVERSATION_STATS_FLAG, value);
}

// Translated from 005b9160 (decompiled, FalloutNV.exe 1.4.0.525)
/// The conversation stats flag (byte global `011cbeac`).
pub fn fn_005b9160(e: &mut Engine) -> u8 {
    e.global::<u8>(CONVERSATION_STATS_FLAG)
}

// Translated from 005b9170 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::ToggleFullHelp` (Xbox PDB): `Interface::ToggleFullHelp`, then
/// prints "shown." or "hidden." after `Interface::GetFullHelp`.
pub fn script_toggle_full_help(e: &mut Engine) -> bool {
    e.call(INTERFACE_TOGGLE_FULL_HELP, &args![]);
    let state = if e.call(INTERFACE_GET_FULL_HELP, &args![]).bool() {
        TEXT_SHOWN
    } else {
        TEXT_HIDDEN
    };
    console_print(e, &args![MSG_TOGGLE_FULL_HELP, state]);
    true
}

// Translated from 005b91b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::ToggleMagicStatsFunction` (Xbox PDB): flips the magic stats flag
/// and prints "shown." or "hidden.".
pub fn script_toggle_magic_stats_function(e: &mut Engine) -> bool {
    let flag = fn_005b9210(e);
    fn_005b9200(e, (flag == 0) as u8);
    let state = if fn_005b9210(e) != 0 {
        TEXT_SHOWN
    } else {
        TEXT_HIDDEN
    };
    console_print(e, &args![MSG_MAGIC_STATS, state]);
    true
}

// Translated from 005b9200 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the magic stats flag (byte global `011c3534`).
pub fn fn_005b9200(e: &mut Engine, value: u8) {
    e.set_global(MAGIC_STATS_FLAG, value);
}

// Translated from 005b9210 (decompiled, FalloutNV.exe 1.4.0.525)
/// The magic stats flag (byte global `011c3534`).
pub fn fn_005b9210(e: &mut Engine) -> u8 {
    e.global::<u8>(MAGIC_STATS_FLAG)
}

// Translated from 005b9220 (decompiled, FalloutNV.exe 1.4.0.525)
/// Runs `00456a60` on the `TES`.
pub fn fn_005b9220(e: &mut Engine) -> bool {
    let tes = e.global::<u32>(GLOBAL_0011DEA10);
    e.call(FN_00456A60, &args![tes]);
    true
}

// Translated from 005b9240 (decompiled, FalloutNV.exe 1.4.0.525)
/// Runs `00456a60` on the `TES` with the byte `012680fc` set to 1 meanwhile.
pub fn fn_005b9240(e: &mut Engine) -> bool {
    e.set_global(FLAG_012680FC, 1u8);
    let tes = e.global::<u32>(GLOBAL_0011DEA10);
    e.call(FN_00456A60, &args![tes]);
    e.set_global(FLAG_012680FC, 0u8);
    true
}

// Translated from 005b9260 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::ToggleCharControllerShapeFunction` (Xbox PDB): when the command
/// runs on a reference that is an `Actor` (`__RTDynamicCast` from
/// `TESObjectREFR`) with a character controller, switches the controller's
/// shape type (`this + 0x59c`, [`fn_005b92e0`]) between 0 and 1: 0 becomes 1,
/// 1 becomes 0, any other value is set again unchanged.
pub fn script_toggle_char_controller_shape_function(e: &mut Engine, a: ScriptArgs) -> bool {
    if a.this_obj.is_null() {
        return true;
    }
    let actor = e
        .call(
            DYNAMIC_CAST,
            &args![a.this_obj, 0u32, RTTI_TES_OBJECT_REFR, RTTI_ACTOR, 0u32],
        )
        .u32();
    if actor == 0 {
        return true;
    }
    let controller = e
        .call(MOBILE_OBJECT_GET_CHAR_CONTROLLER, &args![actor])
        .u32();
    if controller == 0 {
        return true;
    }
    let mut shape = fn_005b92e0(e, Ptr::new(controller));
    if shape == 0 {
        shape = 1;
    } else if shape == 1 {
        shape = 0;
    }
    e.call(CHAR_CONTROLLER_SET_SHAPE_TYPE, &args![controller, shape]);
    true
}

// Translated from 005b92e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The dword at `this + 0x59c` (the shape type of a
/// `bhkCharacterController`, PC offset).
pub fn fn_005b92e0(e: &mut Engine, this: Ptr) -> u32 {
    e.mem.u32(this.addr() + 0x59c)
}

// Translated from 005b9300 (decompiled, FalloutNV.exe 1.4.0.525)
/// One string argument (a 0x204-byte buffer), handed to `00456b10` on the
/// `TES`. (The stack protector cookie check is left out.)
pub fn fn_005b9300(e: &mut Engine, a: ScriptArgs) -> bool {
    e.with_stack(0x204, |e, buffer| {
        if !a.parse(e, &[buffer.addr()]) {
            return false;
        }
        let tes = e.global::<u32>(GLOBAL_0011DEA10);
        e.call(FN_00456B10, &args![tes, buffer]);
        true
    })
}

// Translated from 005b9370 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::ToggleBordersFunction` (Xbox PDB): flips the borders flag; while
/// `005f36f0` of the `TES` is 0, passes the flag to
/// `TES::bShowLANDborders`'s setter (`00456cb0`); echoes "Borders -> On" or
/// "Off" when the TLS echo flag is set.
pub fn script_toggle_borders_function(e: &mut Engine) -> bool {
    let flag = e.global::<u8>(BORDERS_FLAG) ^ 1;
    e.set_global(BORDERS_FLAG, flag);
    let tes = e.global::<u32>(GLOBAL_0011DEA10);
    if e.call(FN_005F36F0, &args![tes]).u32() == 0 {
        let flag = e.global::<u8>(BORDERS_FLAG);
        let tes = e.global::<u32>(GLOBAL_0011DEA10);
        e.call(TES_SET_SHOW_LAND_BORDERS, &args![tes, u32::from(flag)]);
    }
    if echo_enabled(e) {
        let state = if e.global::<u8>(BORDERS_FLAG) != 0 {
            TEXT_ON_CAPITAL
        } else {
            TEXT_OFF_CAPITAL
        };
        console_print(e, &args![MSG_BORDERS, state]);
    }
    true
}

// Translated from 005b9400 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::ToggleProjectileDebugFunction` (Xbox PDB): flips the projectile
/// debug flag ([`fn_005b9470`]) and, with the TLS echo flag set, prints what
/// the debug lines and diamonds mean.
pub fn script_toggle_projectile_debug_function(e: &mut Engine) -> bool {
    fn_005b9470(e);
    if echo_enabled(e) {
        for message in [
            MSG_PROJECTILE_DEBUG_1,
            MSG_PROJECTILE_DEBUG_2,
            MSG_PROJECTILE_DEBUG_3,
            MSG_PROJECTILE_DEBUG_4,
            MSG_PROJECTILE_DEBUG_5,
        ] {
            console_print(e, &args![message]);
        }
    }
    true
}

// Translated from 005b9470 (decompiled, FalloutNV.exe 1.4.0.525)
/// Flips the projectile debug flag (byte global `011f20a4`).
pub fn fn_005b9470(e: &mut Engine) {
    let flag = e.global::<u8>(PROJECTILE_DEBUG_FLAG);
    e.set_global(PROJECTILE_DEBUG_FLAG, (flag == 0) as u8);
}

// Translated from 005b9490 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::ToggleMenusFunction` (Xbox PDB): flips the menus flag, passes it
/// to `00703810` and, with the TLS echo flag set, prints "Menus -> On" or
/// "Off".
pub fn script_toggle_menus_function(e: &mut Engine) -> bool {
    let flag = e.global::<u8>(MENUS_FLAG) ^ 1;
    e.set_global(MENUS_FLAG, flag);
    let flag = e.global::<u8>(MENUS_FLAG);
    e.call(FN_00703810, &args![u32::from(flag)]);
    if echo_enabled(e) {
        let state = if e.global::<u8>(MENUS_FLAG) != 0 {
            TEXT_ON_CAPITAL
        } else {
            TEXT_OFF_CAPITAL
        };
        console_print(e, &args![MSG_MENUS, state]);
    }
    true
}

// Translated from 005b9500 (decompiled, FalloutNV.exe 1.4.0.525)
/// Runs `00456aa0` on the `TES`.
pub fn fn_005b9500(e: &mut Engine) -> bool {
    let tes = e.global::<u32>(GLOBAL_0011DEA10);
    e.call(FN_00456AA0, &args![tes]);
    true
}

// Translated from 005b9520 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::ToggleSafeZone(2)`.
pub fn fn_005b9520(e: &mut Engine) -> bool {
    e.call(INTERFACE_TOGGLE_SAFE_ZONE, &args![2u32]);
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
        entry!(0x005b7070, script_toggle_scripts_function() -> bool),
        entry!(0x005b70c0, script_toggle_grass_function() -> bool),
        entry!(
            0x005b71b0,
            script_test_all_cells_function(ScriptArgs) -> bool
        ),
        entry!(0x005b7250, fn_005b7250() -> bool),
        entry!(0x005b72a0, fn_005b72a0() -> bool),
        entry!(0x005b72f0, fn_005b72f0() -> bool),
        entry!(0x005b7420, script_toggle_emotions_function() -> bool),
        entry!(0x005b7470, fn_005b7470() -> u8),
        entry!(0x005b7480, fn_005b7480(u8)),
        entry!(0x005b7490, script_reload_current_climate() -> bool),
        entry!(0x005b7520, fn_005b7520() -> bool),
        entry!(0x005b7590, fn_005b7590(ScriptArgs) -> bool),
        entry!(0x005b7660, fn_005b7660(ScriptArgs) -> bool),
        entry!(0x005b76d0, fn_005b76d0() -> bool),
        entry!(
            0x005b76f0,
            script_apply_image_space_modifier(ScriptArgs) -> bool
        ),
        entry!(
            0x005b7760,
            script_remove_image_space_modifier(ScriptArgs) -> bool
        ),
        entry!(0x005b77c0, fn_005b77c0(ScriptArgs) -> bool),
        entry!(0x005b7820, fn_005b7820(ScriptArgs) -> bool),
        entry!(0x005b78a0, fn_005b78a0() -> bool),
        entry!(0x005b7b40, script_test_local_map(ScriptArgs) -> bool),
        entry!(0x005b8fc0, fn_005b8fc0(Ptr, u32)),
        entry!(0x005b9070, script_test_code(ScriptArgs) -> bool),
        entry!(
            0x005b9100,
            script_toggle_conversation_stats_function() -> bool
        ),
        entry!(0x005b9150, fn_005b9150(u8)),
        entry!(0x005b9160, fn_005b9160() -> u8),
        entry!(0x005b9170, script_toggle_full_help() -> bool),
        entry!(0x005b91b0, script_toggle_magic_stats_function() -> bool),
        entry!(0x005b9200, fn_005b9200(u8)),
        entry!(0x005b9210, fn_005b9210() -> u8),
        entry!(0x005b9220, fn_005b9220() -> bool),
        entry!(0x005b9240, fn_005b9240() -> bool),
        entry!(
            0x005b9260,
            script_toggle_char_controller_shape_function(ScriptArgs) -> bool
        ),
        entry!(0x005b92e0, fn_005b92e0(Ptr) -> u32),
        entry!(0x005b9300, fn_005b9300(ScriptArgs) -> bool),
        entry!(0x005b9370, script_toggle_borders_function() -> bool),
        entry!(
            0x005b9400,
            script_toggle_projectile_debug_function() -> bool
        ),
        entry!(0x005b9470, fn_005b9470()),
        entry!(0x005b9490, script_toggle_menus_function() -> bool),
        entry!(0x005b9500, fn_005b9500() -> bool),
        entry!(0x005b9520, fn_005b9520() -> bool),
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

    // ---- The second batch (005b7070 onward) -----------------------------------

    /// [`engine`] with the pages of the globals and constants of the second
    /// batch mapped.
    fn engine_b() -> Engine {
        let mut e = engine();
        for page in [
            0x0101_1000,
            0x0101_7000,
            0x0101_8000,
            0x0101_a000,
            0x0101_d000,
            0x0101_e000,
            0x0102_0000,
            0x0102_2000,
            0x0118_c000,
            0x0119_b000,
            0x011a_2000,
            0x011a_9000,
            0x011c_6000,
            0x011c_a000,
            0x011c_b000,
            0x011f_2000,
            0x011f_4000,
            0x0126_8000,
        ] {
            e.map(page, 0x1000);
        }
        e
    }

    /// Sets the TLS echo flag.
    fn set_echo(e: &mut Engine, on: bool) {
        let tls = e.tls();
        e.mem.set_u8(tls + TLS_ECHO, on as u8);
    }

    #[test]
    fn toggle_scripts_flips_script_processing() {
        let mut e = engine_b();
        // Script processing is a byte at 0x011f9000 in the doubles.
        e.register(GET_PROCESS_SCRIPTS, |e, _| {
            (e.mem.u8(0x011f_9000) != 0).into_ret()
        });
        e.register(SET_PROCESS_SCRIPTS, |e, a| {
            e.mem.set_u8(0x011f_9000, a[0] as u8);
            Ret::default()
        });
        assert_eq!(
            run_and_print(&mut e, 0x005b_7070),
            vec![vec![MSG_SCRIPT_PROCESSING, TEXT_ENABLED]]
        );
        assert_eq!(
            run_and_print(&mut e, 0x005b_7070),
            vec![vec![MSG_SCRIPT_PROCESSING, TEXT_DISABLED]]
        );
        assert_eq!(e.mem.u8(0x011f_9000), 0);
    }

    /// The grass object of `ToggleGrass`: a byte at +0 is its flag.
    fn grass_world(e: &mut Engine) -> u32 {
        let object = e.mem.alloc(16);
        e.register_double(FN_0054F4C0, move |_, _| object.into_ret());
        e.register(FN_00456610, |e, a| (e.mem.u8(a[0]) != 0).into_ret());
        e.register(FN_00450F90, |e, a| {
            e.mem.set_u8(a[0], a[1] as u8);
            Ret::default()
        });
        accept(e, &[CONSOLE_PRINT, FN_00B62A20, FN_0057D0A0]);
        e.register(FN_0045C670, |_, _| 0x5000u32.into_ret());
        object
    }

    #[test]
    fn toggle_grass_does_nothing_without_the_grass_object() {
        let mut e = engine_b();
        e.register(FN_0054F4C0, |_, _| 0u32.into_ret());
        assert!(run_and_print(&mut e, 0x005b_70c0).is_empty());
        assert_eq!(e.global::<u8>(GRASS_DISPLAY_FLAG), 0);
    }

    #[test]
    fn toggle_grass_sets_the_flag_and_the_display_byte() {
        let mut e = engine_b();
        let object = grass_world(&mut e);
        // The object's flag is 0: the new flag is 1, the display byte 0 and
        // `00b62a20` runs; "Disabled." is printed.
        start_log(&mut e);
        assert_eq!(
            run_and_print(&mut e, 0x005b_70c0),
            vec![vec![MSG_GRASS_DISPLAY, TEXT_DISABLED_CAPITAL]]
        );
        assert_eq!(calls(&e, FN_00450F90), vec![vec![object, 1]]);
        assert_eq!(e.mem.u8(object), 1);
        assert_eq!(e.global::<u8>(GRASS_DISPLAY_FLAG), 0);
        assert_eq!(calls(&e, FN_00B62A20).len(), 1);
        assert!(calls(&e, FN_0057D0A0).is_empty());
    }

    #[test]
    fn toggle_grass_off_places_the_grass_at_the_animation_data_vector() {
        let mut e = engine_b();
        let object = grass_world(&mut e);
        e.mem.set_u8(object, 1);
        // `0045c670` -> holder 0x5000; its animation data (`006629f0`) is
        // 0x6000; `0045bb80` returns a vector in memory.
        let animation = e.mem.alloc(0x100);
        let vector = e.mem.alloc(12);
        for (i, w) in [1.0f32, 2.0, 3.0].iter().enumerate() {
            e.mem.set_f32(vector + 4 * i as u32, *w);
        }
        for (i, w) in [7.0f32, 8.0, 9.0].iter().enumerate() {
            e.set_global(START_VECTOR + 4 * i as u32, *w);
        }
        e.register_double(FN_006629F0, move |_, a| {
            assert_eq!(a[0], 0x5000);
            animation.into_ret()
        });
        e.register_double(FN_0045BB80, move |_, _| vector.into_ret());
        start_log(&mut e);
        assert_eq!(
            run_and_print(&mut e, 0x005b_70c0),
            vec![vec![MSG_GRASS_DISPLAY, TEXT_ENABLED_CAPITAL]]
        );
        assert_eq!(calls(&e, FN_00450F90), vec![vec![object, 0]]);
        assert_eq!(e.global::<u8>(GRASS_DISPLAY_FLAG), 1);
        assert!(calls(&e, FN_00B62A20).is_empty());
        // The start vector and a 0.0 go to `0045bb80`; its result to `0057d0a0`.
        assert_eq!(
            calls(&e, FN_0045BB80),
            vec![vec![
                animation,
                7.0f32.to_bits(),
                8.0f32.to_bits(),
                9.0f32.to_bits(),
                0
            ]]
        );
        assert_eq!(
            calls(&e, FN_0057D0A0),
            vec![vec![1.0f32.to_bits(), 2.0f32.to_bits(), 3.0f32.to_bits()]]
        );
        // Without animation data nothing is placed.
        e.mem.set_u8(object, 1);
        e.register(FN_006629F0, |_, _| 0u32.into_ret());
        start_log(&mut e);
        run_and_print(&mut e, 0x005b_70c0);
        assert!(calls(&e, FN_0057D0A0).is_empty());
    }

    #[test]
    fn test_all_cells_passes_the_count_and_reports_the_state() {
        let mut e = engine_b();
        let tes = 0x7000u32;
        e.set_global(GLOBAL_0011DEA10, tes);
        accept(&mut e, &[TES_TEST_ALL_CELLS, CONSOLE_PRINT]);
        e.register(TES_IS_RUNNING_CELL_TESTS, |_, _| true.into_ret());
        parse_gives(&mut e, true, &[0]);
        start_log(&mut e);
        assert!(e.call(0x005b_71b0, &args![script(0)]).bool());
        // No count: -1.
        assert_eq!(calls(&e, TES_TEST_ALL_CELLS), vec![vec![tes, u32::MAX]]);
        assert_eq!(printed(&e), vec![vec![MSG_TEST_ALL_CELLS, TEXT_RUNNING]]);
        // A count is passed as it is; a stopped test says so.
        parse_gives(&mut e, true, &[25]);
        e.register(TES_IS_RUNNING_CELL_TESTS, |_, _| false.into_ret());
        start_log(&mut e);
        assert!(e.call(0x005b_71b0, &args![script(0)]).bool());
        assert_eq!(calls(&e, TES_TEST_ALL_CELLS), vec![vec![tes, 25]]);
        assert_eq!(printed(&e), vec![vec![MSG_TEST_ALL_CELLS, TEXT_STOPPED]]);
        // Bad parameters.
        parse_gives(&mut e, false, &[]);
        start_log(&mut e);
        assert!(!e.call(0x005b_71b0, &args![script(0)]).bool());
        assert!(calls(&e, TES_TEST_ALL_CELLS).is_empty());
    }

    #[test]
    fn render_test_cell_commands_render_the_players_parent_cell() {
        let mut e = engine_b();
        e.set_global(PLAYER, 0x4000u32);
        accept(&mut e, &[CELL_RENDER_TEST_CELL, CONSOLE_PRINT]);
        // No parent cell.
        e.register(REFR_GET_PARENT_CELL, |_, _| 0u32.into_ret());
        for addr in [0x005b_7250, 0x005b_72a0] {
            assert_eq!(
                run_and_print(&mut e, addr),
                vec![vec![MSG_RENDER_TEST_CELL_FAILED]]
            );
            assert!(calls(&e, CELL_RENDER_TEST_CELL).is_empty());
        }
        // A cell: flag 1 for the first command, 0 for the second.
        e.register(REFR_GET_PARENT_CELL, |_, a| {
            assert_eq!(a, [0x4000]);
            0x8000u32.into_ret()
        });
        assert_eq!(
            run_and_print(&mut e, 0x005b_7250),
            vec![vec![MSG_RENDER_TEST_CELL_COMPLETE]]
        );
        assert_eq!(calls(&e, CELL_RENDER_TEST_CELL), vec![vec![0x8000, 0, 1]]);
        assert_eq!(
            run_and_print(&mut e, 0x005b_72a0),
            vec![vec![MSG_RENDER_TEST_CELL_COMPLETE]]
        );
        assert_eq!(calls(&e, CELL_RENDER_TEST_CELL), vec![vec![0x8000, 0, 0]]);
    }

    #[test]
    fn ini_refresh_loads_each_ini_file_that_has_a_name() {
        let mut e = engine_b();
        // The names the pointers point to and the game directory.
        let directory = e.mem.alloc(16);
        e.mem.set_cstr(directory, b"C:\\Game\\");
        let names = [
            (FALLOUT_INI_NAME, b"Fallout.ini".as_slice()),
            (CUSTOM_INI_NAME, b"Custom.ini".as_slice()),
            (FALLOUT_PREFS_INI_NAME, b"FalloutPrefs.ini".as_slice()),
        ];
        for (slot, text) in names {
            let at = e.mem.alloc(32);
            e.mem.set_cstr(at, text);
            e.set_global(slot, at);
        }
        e.register_double(FN_004DC110, move |_, _| directory.into_ret());
        e.register(STRCPY_S, |e, a| {
            let text = e.mem.cstr(a[2]);
            e.mem.set_cstr(a[0], &text);
            Ret::default()
        });
        e.register(STRCAT_S, |e, a| {
            let mut text = e.mem.cstr(a[0]);
            text.extend(e.mem.cstr(a[2]));
            e.mem.set_cstr(a[0], &text);
            Ret::default()
        });
        e.register(FN_0044F560, |_, _| 0xa000u32.into_ret());
        e.register(FN_004DE490, |_, _| 0xb000u32.into_ret());
        // The double sees the path while the buffer lives.
        e.register_double(FN_005E0200, |e, a| {
            let path = String::from_utf8(e.mem.cstr(a[1])).unwrap();
            let mut log = e.mem.cstr(0x0126_8100);
            log.extend_from_slice(format!("{}|{}\n", a[0], path).as_bytes());
            e.mem.set_cstr(0x0126_8100, &log);
            Ret::default()
        });
        accept(&mut e, &[CONSOLE_PRINT]);
        start_log(&mut e);
        assert!(e.call(0x005b_72f0, &args![]).bool());
        let log = String::from_utf8(e.mem.cstr(0x0126_8100)).unwrap();
        assert_eq!(
            log,
            format!(
                "{}|C:\\Game\\Fallout.ini\n{}|C:\\Game\\Custom.ini\n{}|C:\\Game\\FalloutPrefs.ini\n",
                0xa000, 0xa000, 0xb000
            )
        );
        assert_eq!(printed(&e), vec![vec![MSG_INI_REFRESHED]]);
        // A missing name is skipped.
        e.set_global(CUSTOM_INI_NAME, 0u32);
        e.mem.set_cstr(0x0126_8100, b"");
        assert!(e.call(0x005b_72f0, &args![]).bool());
        let log = String::from_utf8(e.mem.cstr(0x0126_8100)).unwrap();
        assert_eq!(log.lines().count(), 2);
    }

    #[test]
    fn emotions_flag_accessors_and_toggle() {
        let mut e = engine_b();
        assert_eq!(e.call(0x005b_7470, &args![]).u8(), 0);
        e.call(0x005b_7480, &args![1u32]);
        assert_eq!(e.call(0x005b_7470, &args![]).u8(), 1);
        assert_eq!(e.global::<u8>(EMOTIONS_FLAG), 1);
        assert_eq!(
            run_and_print(&mut e, 0x005b_7420),
            vec![vec![MSG_NPC_EMOTIONS, TEXT_DISABLED_CAPITAL]]
        );
        assert_eq!(
            run_and_print(&mut e, 0x005b_7420),
            vec![vec![MSG_NPC_EMOTIONS, TEXT_ENABLED_CAPITAL]]
        );
    }

    /// A sky with a climate and a weather, a form file for each and doubles
    /// for the file functions. Returns `(sky, climate, weather)`.
    fn sky_world(e: &mut Engine) -> (u32, u32, u32) {
        let sky = e.mem.alloc(0x138);
        let climate = object_with(e, &[(0x18, V_TRUE), (0x20, V_RECORD)]);
        let weather = object_with(e, &[(0x18, V_TRUE), (0x20, V_RECORD)]);
        e.mem.set_u32(sky + 0x0c, climate);
        e.mem.set_u32(sky + 0x10, weather);
        e.register_double(SKY_GET_INSTANCE, move |_, _| sky.into_ret());
        e.register(SKY_GET_CURRENT_CLIMATE, |e, a| {
            e.mem.u32(a[0] + 0x0c).into_ret()
        });
        e.register(SKY_GET_CURRENT_WEATHER, |e, a| {
            e.mem.u32(a[0] + 0x10).into_ret()
        });
        e.register(FORM_GET_FILE, |_, a| (a[0] + 0xe000).into_ret());
        e.register(FILE_GET_THREAD_SAFE_FILE, |_, a| (a[0] + 0x10).into_ret());
        accept(
            e,
            &[
                FILE_OPEN_TES,
                FILE_FIND_FORM,
                SKY_SET_CURRENT_CLIMATE,
                SKY_FORCE_WEATHER,
                FN_0093A7A0,
                FN_0063E860,
                SKY_CLEAR_OVERRIDE_WEATHER,
            ],
        );
        (sky, climate, weather)
    }

    #[test]
    fn reload_current_climate_reloads_the_climate_from_its_file() {
        let mut e = engine_b();
        let (sky, climate, _) = sky_world(&mut e);
        start_log(&mut e);
        assert!(e.call(0x005b_7490, &args![]).bool());
        let file = climate + 0xe000 + 0x10;
        assert_eq!(calls(&e, FORM_GET_FILE), vec![vec![climate, u32::MAX]]);
        assert_eq!(calls(&e, FILE_OPEN_TES), vec![vec![file, 0, 0]]);
        assert_eq!(calls(&e, FILE_FIND_FORM), vec![vec![file, climate]]);
        // Virtual slots 0x18 (no arguments) and 0x20 (the file).
        assert_eq!(calls(&e, V_TRUE), vec![vec![climate]]);
        assert_eq!(calls(&e, V_RECORD), vec![vec![climate, file]]);
        assert_eq!(
            calls(&e, SKY_SET_CURRENT_CLIMATE),
            vec![vec![sky, climate, 1]]
        );
        // A climate without a file: nothing is reloaded.
        e.register(FORM_GET_FILE, |_, _| 0u32.into_ret());
        start_log(&mut e);
        assert!(e.call(0x005b_7490, &args![]).bool());
        assert!(calls(&e, FILE_OPEN_TES).is_empty());
        assert!(calls(&e, SKY_SET_CURRENT_CLIMATE).is_empty());
    }

    #[test]
    fn reload_current_weather_reloads_the_weather_only() {
        let mut e = engine_b();
        let (_, _, weather) = sky_world(&mut e);
        start_log(&mut e);
        assert!(e.call(0x005b_7520, &args![]).bool());
        let file = weather + 0xe000 + 0x10;
        assert_eq!(calls(&e, FILE_FIND_FORM), vec![vec![file, weather]]);
        assert_eq!(calls(&e, V_RECORD), vec![vec![weather, file]]);
        assert!(calls(&e, SKY_SET_CURRENT_CLIMATE).is_empty());
        e.register(FORM_GET_FILE, |_, _| 0u32.into_ret());
        start_log(&mut e);
        assert!(e.call(0x005b_7520, &args![]).bool());
        assert!(calls(&e, FILE_OPEN_TES).is_empty());
    }

    #[test]
    fn set_weather_stores_the_weather_in_the_override_or_default_slot() {
        let mut e = engine_b();
        let (sky, _, weather) = sky_world(&mut e);
        e.set_global(PLAYER, 0x4000u32);
        // Override flag set: the override slot, `0063e860(1)` as the echo is off.
        parse_gives(&mut e, true, &[0x1111, 1]);
        start_log(&mut e);
        assert!(e.call(0x005b_7590, &args![script(0)]).bool());
        assert_eq!(e.mem.u32(sky + SKY_OVERRIDE_WEATHER), 0x1111);
        assert_eq!(e.mem.u32(sky + SKY_DEFAULT_WEATHER), 0);
        assert_eq!(calls(&e, FN_0063E860), vec![vec![sky, 1]]);
        assert!(calls(&e, FN_0093A7A0).is_empty());
        // No override: the default slot and the player is told.
        parse_gives(&mut e, true, &[0x2222, 0]);
        start_log(&mut e);
        assert!(e.call(0x005b_7590, &args![script(0)]).bool());
        assert_eq!(e.mem.u32(sky + SKY_DEFAULT_WEATHER), 0x2222);
        assert_eq!(calls(&e, FN_0093A7A0), vec![vec![0x4000, 0]]);
        // With the echo flag set `0063e860` is not called.
        set_echo(&mut e, true);
        start_log(&mut e);
        assert!(e.call(0x005b_7590, &args![script(0)]).bool());
        assert!(calls(&e, FN_0063E860).is_empty());
        set_echo(&mut e, false);
        // Without a current weather the weather is forced.
        e.mem.set_u32(sky + 0x10, 0);
        parse_gives(&mut e, true, &[0x3333, 5]);
        start_log(&mut e);
        assert!(e.call(0x005b_7590, &args![script(0)]).bool());
        assert_eq!(calls(&e, SKY_FORCE_WEATHER), vec![vec![sky, 0x3333, 1]]);
        assert!(calls(&e, FN_0063E860).is_empty());
        // Bad parameters.
        parse_gives(&mut e, false, &[]);
        start_log(&mut e);
        assert!(!e.call(0x005b_7590, &args![script(0)]).bool());
        assert!(calls(&e, SKY_FORCE_WEATHER).is_empty());
        let _ = weather;
    }

    #[test]
    fn force_weather_command_forces_the_weather() {
        let mut e = engine_b();
        let (sky, _, _) = sky_world(&mut e);
        parse_gives(&mut e, true, &[0x4444, 3]);
        start_log(&mut e);
        assert!(e.call(0x005b_7660, &args![script(0)]).bool());
        assert_eq!(calls(&e, SKY_FORCE_WEATHER), vec![vec![sky, 0x4444, 1]]);
        parse_gives(&mut e, true, &[0x4444, 0]);
        start_log(&mut e);
        assert!(e.call(0x005b_7660, &args![script(0)]).bool());
        assert_eq!(calls(&e, SKY_FORCE_WEATHER), vec![vec![sky, 0x4444, 0]]);
        parse_gives(&mut e, false, &[]);
        start_log(&mut e);
        assert!(!e.call(0x005b_7660, &args![script(0)]).bool());
        assert!(calls(&e, SKY_FORCE_WEATHER).is_empty());
    }

    #[test]
    fn clear_override_weather_command_calls_the_sky() {
        let mut e = engine_b();
        let (sky, _, _) = sky_world(&mut e);
        start_log(&mut e);
        assert!(e.call(0x005b_76d0, &args![]).bool());
        assert_eq!(calls(&e, SKY_CLEAR_OVERRIDE_WEATHER), vec![vec![sky]]);
    }

    #[test]
    fn image_space_modifier_commands() {
        let mut e = engine_b();
        accept(
            &mut e,
            &[IMAGE_SPACE_MODIFIER_TRIGGER, IMAGE_SPACE_MODIFIER_STOP],
        );
        // The strength defaults to 1.0 (the parse double leaves the default
        // when it stores nothing).
        e.register_double(PARSE_PARAMETERS, |e, a| {
            assert_eq!(e.mem.u32(a[8]), 1.0f32.to_bits());
            e.mem.set_u32(a[7], 0x77);
            true.into_ret()
        });
        start_log(&mut e);
        assert!(e.call(0x005b_76f0, &args![script(0)]).bool());
        assert_eq!(
            calls(&e, IMAGE_SPACE_MODIFIER_TRIGGER),
            vec![vec![0x77, 1.0f32.to_bits(), 0]]
        );
        assert!(e.call(0x005b_7760, &args![script(0)]).bool());
        assert_eq!(calls(&e, IMAGE_SPACE_MODIFIER_STOP), vec![vec![0x77]]);
        // A given strength is passed on.
        parse_gives(&mut e, true, &[0x88, 0.5f32.to_bits()]);
        start_log(&mut e);
        assert!(e.call(0x005b_76f0, &args![script(0)]).bool());
        assert_eq!(
            calls(&e, IMAGE_SPACE_MODIFIER_TRIGGER),
            vec![vec![0x88, 0.5f32.to_bits(), 0]]
        );
        // Bad parameters.
        parse_gives(&mut e, false, &[]);
        start_log(&mut e);
        assert!(!e.call(0x005b_76f0, &args![script(0)]).bool());
        assert!(!e.call(0x005b_7760, &args![script(0)]).bool());
        assert!(calls(&e, IMAGE_SPACE_MODIFIER_TRIGGER).is_empty());
        assert!(calls(&e, IMAGE_SPACE_MODIFIER_STOP).is_empty());
    }

    #[test]
    fn two_argument_command_calls_005477_50_with_both() {
        let mut e = engine_b();
        accept(&mut e, &[FN_00547750]);
        parse_gives(&mut e, true, &[0x10, 0x20]);
        start_log(&mut e);
        assert!(e.call(0x005b_77c0, &args![script(0)]).bool());
        assert_eq!(calls(&e, FN_00547750), vec![vec![0x10, 0x20]]);
        parse_gives(&mut e, false, &[]);
        start_log(&mut e);
        assert!(!e.call(0x005b_77c0, &args![script(0)]).bool());
        assert!(calls(&e, FN_00547750).is_empty());
    }

    #[test]
    fn image_space_override_command_sets_the_managers_list() {
        let mut e = engine_b();
        e.register(GET_IMAGE_SPACE_MANAGER, |_, _| 0x9000u32.into_ret());
        accept(&mut e, &[IMAGE_SPACE_MANAGER_SET_OVERRIDE, FN_00B8B500]);
        e.register(FORM_LIST_ITEMS, |_, a| (a[0] + 0x18).into_ret());
        parse_gives(&mut e, true, &[0x100]);
        start_log(&mut e);
        assert!(e.call(0x005b_7820, &args![script(0)]).bool());
        assert_eq!(
            calls(&e, IMAGE_SPACE_MANAGER_SET_OVERRIDE),
            vec![vec![0x9000, 0x118]]
        );
        assert_eq!(calls(&e, FN_00B8B500), vec![vec![0x9000]]);
        // No argument: the list is cleared.
        parse_gives(&mut e, true, &[0]);
        start_log(&mut e);
        assert!(e.call(0x005b_7820, &args![script(0)]).bool());
        assert_eq!(
            calls(&e, IMAGE_SPACE_MANAGER_SET_OVERRIDE),
            vec![vec![0x9000, 0]]
        );
        assert!(calls(&e, FORM_LIST_ITEMS).is_empty());
        parse_gives(&mut e, false, &[]);
        start_log(&mut e);
        assert!(!e.call(0x005b_7820, &args![script(0)]).bool());
        assert!(calls(&e, FN_00B8B500).is_empty());
    }

    /// Shared records the doubles of the debug drawings write to.
    type Record = std::rc::Rc<std::cell::RefCell<Vec<Vec<u32>>>>;

    /// Virtual functions of the debug world: the player's position slot and
    /// the seen data's draw slot.
    const V_POSITION: u32 = 0x0900_0020;
    const V_DRAW: u32 = 0x0900_0021;
    /// Address of the node vtable (slot `0xdc` is [`V_ATTACH`]).
    const NODE_VTABLE: u32 = 0x0950_0000;
    const V_ATTACH: u32 = 0x0900_0022;

    /// The pieces of the world that both debug drawings use: the player with
    /// a position, the `TES` global, node allocation, point construction and
    /// the calls that finish a debug node. Returns `(player, position)`.
    fn debug_world(e: &mut Engine, position: [f32; 3]) -> (u32, u32) {
        let at = e.mem.alloc(12);
        for (i, v) in position.iter().enumerate() {
            e.mem.set_f32(at + 4 * i as u32, *v);
        }
        e.register_double(V_POSITION, move |_, _| at.into_ret());
        let player = object_with(e, &[(PLAYER_POSITION_SLOT, V_POSITION)]);
        e.set_global(PLAYER, player);
        e.set_global(GLOBAL_0011DEA10, 0x7000u32);
        e.set_global(DEBUG_OBJECT_SECONDS, 30.0f32);
        e.set_global(DOUBLE_MINUS_ONE, -1.0f64);
        e.set_global(DOUBLE_ZERO, 0.0f64);
        e.set_global(DOUBLE_TWO, 2.0f64);
        e.set_global(DOUBLE_FOUR, 4.0f64);
        e.set_global(DOUBLE_TEN, 10.0f64);
        e.set_global(DOUBLE_4096, 4096.0f64);
        for i in 0..9u32 {
            e.set_global(
                MATRIX_011A9448 + 4 * i,
                [1.0f32, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0][i as usize],
            );
        }
        e.register(OPERATOR_NEW, |e, a| e.mem.alloc(a[0]).into_ret());
        e.register(NI_ALLOC, |e, a| e.mem.alloc(a[0]).into_ret());
        e.put_vtable(NODE_VTABLE, &[0; 56]);
        e.mem.set_u32(NODE_VTABLE + 0xdc, V_ATTACH);
        e.register(V_ATTACH, |_, _| Ret::default());
        e.register(NI_NODE_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], NODE_VTABLE);
            a[0].into_ret()
        });
        e.register(NI_POINT3_CONSTRUCT, |e, a| {
            for i in 0..3 {
                e.mem.set_u32(a[0] + 4 * i, a[1 + i as usize]);
            }
            a[0].into_ret()
        });
        accept(
            e,
            &[
                UPDATE_DATA_CONSTRUCT,
                NODE_UPDATE_WITH_DATA,
                NODE_UPDATE_PROPERTIES,
                TES_ADD_TEMP_DEBUG_OBJECT,
                NODE_SET_LOCAL_ROTATE,
            ],
        );
        e.register(NI_MATRIX_FROM_Z_ANGLE, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            Ret::default()
        });
        (player, at)
    }

    /// The calls that end a debug drawing were made for `node`: the update
    /// data constructed as `(0.0, 0, 0)`, the node updated and handed to the
    /// `TES` for 30 seconds.
    fn assert_debug_node_shown(e: &Engine, node: u32) {
        assert_eq!(calls(e, UPDATE_DATA_CONSTRUCT).len(), 1);
        assert_eq!(calls(e, UPDATE_DATA_CONSTRUCT)[0][1..], [0, 0, 0]);
        assert_eq!(calls(e, NODE_UPDATE_WITH_DATA)[0][0], node);
        assert_eq!(calls(e, NODE_UPDATE_PROPERTIES), vec![vec![node]]);
        assert_eq!(
            calls(e, TES_ADD_TEMP_DEBUG_OBJECT),
            vec![vec![0x7000, node, 30.0f32.to_bits()]]
        );
    }

    #[test]
    fn seen_data_drawing_in_an_interior_draws_once_and_rotates_by_the_north() {
        let mut e = engine_b();
        let (_, _) = debug_world(&mut e, [10.0, 20.0, 30.0]);
        let draws: Record = Record::default();
        let record = draws.clone();
        e.register_double(V_DRAW, move |e, a| {
            record.borrow_mut().push(vec![
                a[0],
                a[1],
                e.mem.u32(a[2]),
                e.mem.u32(a[2] + 4),
                e.mem.u32(a[2] + 8),
                a[3],
            ]);
            Ret::default()
        });
        let seen = object_with(&mut e, &[(SEEN_DATA_DRAW_SLOT, V_DRAW)]);
        e.register(REFR_GET_PARENT_CELL, |_, _| 0x8000u32.into_ret());
        e.register(CELL_IS_INTERIOR, |_, _| true.into_ret());
        e.register_double(CELL_GET_SEEN_DATA, move |_, a| {
            assert_eq!(a[0], 0x8000);
            seen.into_ret()
        });
        e.register(CELL_GET_NORTH_ROTATION, |_, _| 0.5f32.into_ret());
        start_log(&mut e);
        assert!(e.call(0x005b_78a0, &args![]).bool());
        let node = calls(&e, NI_NODE_CONSTRUCT)[0][0];
        // The seen data draws the node at (0, 0, z + 0 of the player).
        assert_eq!(
            *draws.borrow(),
            vec![vec![
                seen,
                node,
                0.0f32.to_bits(),
                0.0f32.to_bits(),
                30.0f32.to_bits(),
                1
            ]]
        );
        // The north rotation 0.5 times -1 turns the node.
        let matrix_calls = calls(&e, NI_MATRIX_FROM_Z_ANGLE);
        assert_eq!(matrix_calls.len(), 1);
        assert_eq!(matrix_calls[0][1], (-0.5f32).to_bits());
        let rotate = calls(&e, NODE_SET_LOCAL_ROTATE);
        assert_eq!(rotate.len(), 1);
        assert_eq!(rotate[0][0], node);
        assert_debug_node_shown(&e, node);

        // North rotation 0: the node is not rotated.
        e.register(CELL_GET_NORTH_ROTATION, |_, _| 0.0f32.into_ret());
        start_log(&mut e);
        assert!(e.call(0x005b_78a0, &args![]).bool());
        assert!(calls(&e, NODE_SET_LOCAL_ROTATE).is_empty());
        assert_eq!(draws.borrow().len(), 2);

        // A cell without seen data: no drawing, no rotation.
        e.register(CELL_GET_SEEN_DATA, |_, _| 0u32.into_ret());
        start_log(&mut e);
        assert!(e.call(0x005b_78a0, &args![]).bool());
        assert_eq!(draws.borrow().len(), 2);
        assert!(calls(&e, NI_MATRIX_FROM_Z_ANGLE).is_empty());
    }

    #[test]
    fn seen_data_drawing_in_an_exterior_walks_the_grid() {
        let mut e = engine_b();
        debug_world(&mut e, [10.0, 20.0, 30.0]);
        let draws: Record = Record::default();
        let record = draws.clone();
        e.register_double(V_DRAW, move |e, a| {
            record.borrow_mut().push(vec![
                a[0],
                e.mem.u32(a[2]),
                e.mem.u32(a[2] + 4),
                e.mem.u32(a[2] + 8),
            ]);
            Ret::default()
        });
        // uGridsToLoad is 2: four grid positions; (outer 1, inner 0) has no
        // cell, (1, 1) has a cell without seen data.
        let grid = e.mem.alloc(4);
        e.mem.set_u32(grid, 2);
        e.register_double(SETTING_INT_VALUE_ADDRESS, move |_, a| {
            assert_eq!(a[0], SETTING_GRIDS_TO_LOAD);
            grid.into_ret()
        });
        // Cell objects: +0 the seen data, +4 data x, +8 data y.
        let seen_a = object_with(&mut e, &[(SEEN_DATA_DRAW_SLOT, V_DRAW)]);
        let seen_b = object_with(&mut e, &[(SEEN_DATA_DRAW_SLOT, V_DRAW)]);
        let mut cells = vec![];
        for (seen, x, y) in [(seen_a, 3, 0xffff_fffeu32), (seen_b, 5, 7), (0, 0, 0)] {
            let cell = e.mem.alloc(16);
            e.mem.set_u32(cell, seen);
            e.mem.set_u32(cell + 4, x);
            e.mem.set_u32(cell + 8, y);
            cells.push(cell);
        }
        let slots = e.mem.alloc(16);
        e.mem.set_u32(slots, cells[0]);
        e.mem.set_u32(slots + 4, cells[1]);
        e.mem.set_u32(slots + 8, 0);
        e.mem.set_u32(slots + 12, cells[2]);
        e.register_double(TES_GET_GRID_CELL_SLOT, move |_, a| {
            assert_eq!(a[0], 0x7000);
            (slots + 8 * a[1] + 4 * a[2]).into_ret()
        });
        e.register(REFR_GET_PARENT_CELL, |_, _| 0x8000u32.into_ret());
        e.register(CELL_IS_INTERIOR, |_, _| false.into_ret());
        e.register(CELL_GET_SEEN_DATA, |e, a| e.mem.u32(a[0]).into_ret());
        e.register(CELL_GET_DATA_X, |e, a| e.mem.u32(a[0] + 4).into_ret());
        e.register(CELL_GET_DATA_Y, |e, a| e.mem.u32(a[0] + 8).into_ret());
        start_log(&mut e);
        assert!(e.call(0x005b_78a0, &args![]).bool());
        let node = calls(&e, NI_NODE_CONSTRUCT)[0][0];
        // Cell (0, 0): x 3 << 12, y -2 << 12, z of the player; cell (0, 1):
        // 5 << 12, 7 << 12. The setting is read for every loop test.
        assert_eq!(
            *draws.borrow(),
            vec![
                vec![
                    seen_a,
                    (3.0f32 * 4096.0).to_bits(),
                    (-2.0f32 * 4096.0).to_bits(),
                    30.0f32.to_bits()
                ],
                vec![
                    seen_b,
                    (5.0f32 * 4096.0).to_bits(),
                    (7.0f32 * 4096.0).to_bits(),
                    30.0f32.to_bits()
                ],
            ]
        );
        assert!(calls(&e, NODE_SET_LOCAL_ROTATE).is_empty());
        assert_debug_node_shown(&e, node);
    }

    #[test]
    fn texture_map_is_created_once_and_given_the_texture() {
        let mut e = engine_b();
        // The property's array: element 0 is at `list + 0x40`.
        let property = e.mem.alloc(0x100);
        e.register(ARRAY_ELEMENT_ADDRESS, |_, a| {
            assert_eq!(a[1], 0);
            (a[0] + 0x40).into_ret()
        });
        e.register(OPERATOR_NEW, |e, a| {
            assert_eq!(a[0], 0x10);
            e.mem.alloc(0x10).into_ret()
        });
        e.register(TEXTURE_MAP_CONSTRUCT, |_, a| a[0].into_ret());
        e.register(ARRAY_SET_ELEMENT, |e, a| {
            let value = e.mem.u32(a[2]);
            e.mem.set_u32(a[0] + 0x40 + 4 * a[1], value);
            Ret::default()
        });
        accept(&mut e, &[TEXTURE_MAP_SET_TEXTURE]);
        start_log(&mut e);
        e.call(0x005b_8fc0, &args![Ptr::<()>::new(property), 0x1234u32]);
        let map = e.mem.u32(property + 0x1c + 0x40);
        assert_ne!(map, 0);
        assert_eq!(calls(&e, TEXTURE_MAP_CONSTRUCT), vec![vec![map]]);
        assert_eq!(calls(&e, TEXTURE_MAP_SET_TEXTURE), vec![vec![map, 0x1234]]);
        // The map exists now: it is reused.
        start_log(&mut e);
        e.call(0x005b_8fc0, &args![Ptr::<()>::new(property), 0x5678u32]);
        assert!(calls(&e, TEXTURE_MAP_CONSTRUCT).is_empty());
        assert!(calls(&e, ARRAY_SET_ELEMENT).is_empty());
        assert_eq!(calls(&e, TEXTURE_MAP_SET_TEXTURE), vec![vec![map, 0x5678]]);
    }

    #[test]
    fn test_code_prints_the_loaded_area_in_megabytes() {
        let mut e = engine_b();
        e.set_global(DOUBLE_MEGABYTE, 1048576.0f64);
        e.register(FN_004A8BB0, |e, a| {
            assert_eq!(a[2..], [1, 0]);
            e.mem.set_f32(a[1], 3.0 * 1048576.0 + 524288.0);
            Ret::default()
        });
        accept(&mut e, &[CONSOLE_PRINT]);
        parse_gives(&mut e, true, &[0]);
        start_log(&mut e);
        assert!(e.call(0x005b_9070, &args![script(0)]).bool());
        let mut words = vec![MSG_LOADED_AREA_MEGABYTES];
        words.extend(args![3.5f64]);
        assert_eq!(printed(&e), vec![words]);
        parse_gives(&mut e, false, &[]);
        start_log(&mut e);
        assert!(!e.call(0x005b_9070, &args![script(0)]).bool());
        assert!(printed(&e).is_empty());
    }

    #[test]
    fn stats_flags_accessors_and_toggles() {
        let mut e = engine_b();
        for (set, get, toggle, flag, message) in [
            (
                0x005b_9150,
                0x005b_9160,
                0x005b_9100,
                CONVERSATION_STATS_FLAG,
                MSG_CONVERSATION_STATS,
            ),
            (
                0x005b_9200,
                0x005b_9210,
                0x005b_91b0,
                MAGIC_STATS_FLAG,
                MSG_MAGIC_STATS,
            ),
        ] {
            assert_eq!(e.call(get, &args![]).u8(), 0);
            e.call(set, &args![1u32]);
            assert_eq!(e.call(get, &args![]).u8(), 1);
            assert_eq!(e.global::<u8>(flag), 1);
            assert_eq!(
                run_and_print(&mut e, toggle),
                vec![vec![message, TEXT_HIDDEN]]
            );
            assert_eq!(e.global::<u8>(flag), 0);
            assert_eq!(
                run_and_print(&mut e, toggle),
                vec![vec![message, TEXT_SHOWN]]
            );
            assert_eq!(e.global::<u8>(flag), 1);
        }
    }

    #[test]
    fn full_help_toggle_prints_the_new_state() {
        let mut e = engine_b();
        // The help flag is a byte of the double.
        e.register(INTERFACE_TOGGLE_FULL_HELP, |e, _| {
            let flag = e.mem.u8(0x011f_9100);
            e.mem.set_u8(0x011f_9100, (flag == 0) as u8);
            Ret::default()
        });
        e.register(INTERFACE_GET_FULL_HELP, |e, _| {
            (e.mem.u8(0x011f_9100) != 0).into_ret()
        });
        assert_eq!(
            run_and_print(&mut e, 0x005b_9170),
            vec![vec![MSG_TOGGLE_FULL_HELP, TEXT_SHOWN]]
        );
        assert_eq!(
            run_and_print(&mut e, 0x005b_9170),
            vec![vec![MSG_TOGGLE_FULL_HELP, TEXT_HIDDEN]]
        );
    }

    #[test]
    fn tes_commands_call_their_function_on_the_tes() {
        let mut e = engine_b();
        e.set_global(GLOBAL_0011DEA10, 0x7000u32);
        accept(&mut e, &[FN_00456AA0, INTERFACE_TOGGLE_SAFE_ZONE]);
        // `005b9220` plain; `005b9240` with the byte set while it runs.
        e.register(FN_00456A60, |e, a| {
            assert_eq!(a, [0x7000]);
            let during = e.mem.u8(FLAG_012680FC);
            e.mem.set_u8(0x0126_8100, during);
            Ret::default()
        });
        start_log(&mut e);
        assert!(e.call(0x005b_9220, &args![]).bool());
        assert_eq!(e.mem.u8(0x0126_8100), 0);
        assert!(e.call(0x005b_9240, &args![]).bool());
        assert_eq!(e.mem.u8(0x0126_8100), 1);
        assert_eq!(e.global::<u8>(FLAG_012680FC), 0);
        assert_eq!(calls(&e, FN_00456A60).len(), 2);
        assert!(e.call(0x005b_9500, &args![]).bool());
        assert_eq!(calls(&e, FN_00456AA0), vec![vec![0x7000]]);
        assert!(e.call(0x005b_9520, &args![]).bool());
        assert_eq!(calls(&e, INTERFACE_TOGGLE_SAFE_ZONE), vec![vec![2]]);
    }

    #[test]
    fn char_controller_shape_switches_between_zero_and_one() {
        let mut e = engine_b();
        e.register(DYNAMIC_CAST, |_, a| {
            assert_eq!(a[1..], [0, RTTI_TES_OBJECT_REFR, RTTI_ACTOR, 0]);
            (if a[0] == 0x66 { 0u32 } else { a[0] + 1 }).into_ret()
        });
        let controller = e.mem.alloc(0x600);
        e.register_double(MOBILE_OBJECT_GET_CHAR_CONTROLLER, move |_, a| {
            (if a[0] == 0x41 { controller } else { 0 }).into_ret()
        });
        accept(&mut e, &[CHAR_CONTROLLER_SET_SHAPE_TYPE]);
        for (before, after) in [(0u32, 1u32), (1, 0), (2, 2)] {
            e.mem.set_u32(controller + 0x59c, before);
            start_log(&mut e);
            assert!(e.call(0x005b_9260, &args![script(0x40)]).bool());
            assert_eq!(
                calls(&e, CHAR_CONTROLLER_SET_SHAPE_TYPE),
                vec![vec![controller, after]]
            );
        }
        assert_eq!(
            e.call(0x005b_92e0, &args![Ptr::<()>::new(controller)])
                .u32(),
            2
        );
        // No reference, a reference that is no actor, an actor without a
        // controller: nothing changes.
        for this_obj in [0, 0x66, 0x50] {
            start_log(&mut e);
            assert!(e.call(0x005b_9260, &args![script(this_obj)]).bool());
            assert!(calls(&e, CHAR_CONTROLLER_SET_SHAPE_TYPE).is_empty());
        }
    }

    #[test]
    fn string_command_hands_its_string_to_the_tes() {
        let mut e = engine_b();
        e.set_global(GLOBAL_0011DEA10, 0x7000u32);
        e.register(FN_00456B10, |e, a| {
            assert_eq!(a[0], 0x7000);
            assert_eq!(e.mem.cstr(a[1]), b"mapname".to_vec());
            Ret::default()
        });
        e.register_double(PARSE_PARAMETERS, |e, a| {
            e.mem.set_cstr(a[7], b"mapname");
            true.into_ret()
        });
        start_log(&mut e);
        assert!(e.call(0x005b_9300, &args![script(0)]).bool());
        assert_eq!(calls(&e, FN_00456B10).len(), 1);
        parse_gives(&mut e, false, &[]);
        start_log(&mut e);
        assert!(!e.call(0x005b_9300, &args![script(0)]).bool());
        assert!(calls(&e, FN_00456B10).is_empty());
    }

    #[test]
    fn borders_toggle_flips_the_flag_and_tells_the_tes() {
        let mut e = engine_b();
        e.set_global(GLOBAL_0011DEA10, 0x7000u32);
        accept(&mut e, &[TES_SET_SHOW_LAND_BORDERS, CONSOLE_PRINT]);
        e.register(FN_005F36F0, |e, _| e.mem.u32(0x0126_8100).into_ret());
        start_log(&mut e);
        assert!(e.call(0x005b_9370, &args![]).bool());
        assert_eq!(e.global::<u8>(BORDERS_FLAG), 1);
        assert_eq!(calls(&e, TES_SET_SHOW_LAND_BORDERS), vec![vec![0x7000, 1]]);
        // The echo flag is off: no print.
        assert!(printed(&e).is_empty());
        // `005f36f0` non-zero: the TES is left alone; with the echo flag the
        // state is printed.
        e.mem.set_u32(0x0126_8100, 5);
        set_echo(&mut e, true);
        start_log(&mut e);
        assert!(e.call(0x005b_9370, &args![]).bool());
        assert_eq!(e.global::<u8>(BORDERS_FLAG), 0);
        assert!(calls(&e, TES_SET_SHOW_LAND_BORDERS).is_empty());
        assert_eq!(printed(&e), vec![vec![MSG_BORDERS, TEXT_OFF_CAPITAL]]);
        start_log(&mut e);
        assert!(e.call(0x005b_9370, &args![]).bool());
        assert_eq!(printed(&e), vec![vec![MSG_BORDERS, TEXT_ON_CAPITAL]]);
    }

    #[test]
    fn projectile_debug_toggle_explains_the_shapes_when_echoing() {
        let mut e = engine_b();
        accept(&mut e, &[CONSOLE_PRINT]);
        start_log(&mut e);
        assert!(e.call(0x005b_9400, &args![]).bool());
        assert_eq!(e.global::<u8>(PROJECTILE_DEBUG_FLAG), 1);
        assert!(printed(&e).is_empty());
        set_echo(&mut e, true);
        start_log(&mut e);
        assert!(e.call(0x005b_9400, &args![]).bool());
        assert_eq!(e.global::<u8>(PROJECTILE_DEBUG_FLAG), 0);
        assert_eq!(
            printed(&e),
            vec![
                vec![MSG_PROJECTILE_DEBUG_1],
                vec![MSG_PROJECTILE_DEBUG_2],
                vec![MSG_PROJECTILE_DEBUG_3],
                vec![MSG_PROJECTILE_DEBUG_4],
                vec![MSG_PROJECTILE_DEBUG_5],
            ]
        );
        // The flip alone.
        e.mem.set_u8(PROJECTILE_DEBUG_FLAG, 7);
        e.call(0x005b_9470, &args![]);
        assert_eq!(e.global::<u8>(PROJECTILE_DEBUG_FLAG), 0);
        e.call(0x005b_9470, &args![]);
        assert_eq!(e.global::<u8>(PROJECTILE_DEBUG_FLAG), 1);
    }

    #[test]
    fn menus_toggle_passes_the_flag_on() {
        let mut e = engine_b();
        accept(&mut e, &[FN_00703810, CONSOLE_PRINT]);
        start_log(&mut e);
        assert!(e.call(0x005b_9490, &args![]).bool());
        assert_eq!(e.global::<u8>(MENUS_FLAG), 1);
        assert_eq!(calls(&e, FN_00703810), vec![vec![1]]);
        assert!(printed(&e).is_empty());
        set_echo(&mut e, true);
        start_log(&mut e);
        assert!(e.call(0x005b_9490, &args![]).bool());
        assert_eq!(calls(&e, FN_00703810), vec![vec![0]]);
        assert_eq!(printed(&e), vec![vec![MSG_MENUS, TEXT_OFF_CAPITAL]]);
        start_log(&mut e);
        e.call(0x005b_9490, &args![]);
        assert_eq!(printed(&e), vec![vec![MSG_MENUS, TEXT_ON_CAPITAL]]);
    }

    /// The words at `addr`.
    fn words_at(e: &Engine, addr: u32, count: u32) -> Vec<u32> {
        (0..count).map(|i| e.mem.u32(addr + 4 * i)).collect()
    }

    /// The records `TestLocalMap`'s doubles keep.
    struct LocalMapRecords {
        /// `NiTriShape::NiTriShape` arguments.
        shapes: Record,
        /// `(shape, x, y, z)` of the tile moves.
        tile_moves: Record,
        /// `(target, x, y, z)` of the `NODE_SET_LOCAL_TRANSLATE` calls.
        node_moves: Record,
        /// The points `CELL_GET_SEEN_VALUE` was asked about.
        seen_points: Record,
        /// The words `MAKE_TRIANGLE` got.
        triangles: Record,
    }

    /// The world of the `TestLocalMap` tests: an exterior with the player at
    /// `position`, `grid` tiles per side, a world space whose cells have a
    /// local map texture and doubles that record what the command builds.
    fn local_map_world(e: &mut Engine, position: [f32; 3], grid: u32) -> LocalMapRecords {
        let (_, _) = debug_world(e, position);
        let grid_value = e.mem.alloc(4);
        e.mem.set_u32(grid_value, grid);
        e.register_double(SETTING_INT_VALUE_ADDRESS, move |_, _| grid_value.into_ret());
        e.register(GET_LOCAL_MAP_SCALE, |_, _| 2.0f32.into_ret());
        e.register(REFR_GET_INTERIOR, |_, _| false.into_ret());
        e.register(REFR_GET_WORLD_SPACE, |_, _| 0x3000u32.into_ret());
        e.register(REFR_GET_PARENT_CELL, |_, _| 0x8000u32.into_ret());
        e.register(WORLD_SPACE_GET_CELL_FROM_CELL_COORD, |_, _| {
            0x3100u32.into_ret()
        });
        e.register(CELL_GET_SEEN_DATA, |_, _| 0u32.into_ret());
        e.register(FN_0054E640, |e, a| {
            e.mem.set_u32(a[1], 0x5555);
            Ret::default()
        });
        e.register(FLOAT_TO_INT, |_, a| {
            (f32::from_bits(a[0]).round_ties_even() as i32 as u32).into_ret()
        });
        e.register(NI_POINT2_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            e.mem.set_u32(a[0] + 4, a[2]);
            a[0].into_ret()
        });
        e.register(NI_COLOR_A_CONSTRUCT, |e, a| {
            for i in 0..4 {
                e.mem.set_u32(a[0] + 4 * i, a[1 + i as usize]);
            }
            a[0].into_ret()
        });
        accept(e, &[VECTOR_CONSTRUCT_SIMPLE]);
        e.register(NI_ALLOC_SHORTS, |e, a| e.mem.alloc(a[0]).into_ret());
        e.register(TRIVIAL_CONSTRUCT, |_, a| a[0].into_ret());
        e.register(NI_POINTER_INIT, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            Ret::default()
        });
        e.register(NI_POINTER_SET, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            Ret::default()
        });
        accept(
            e,
            &[
                NI_POINTER_DESTRUCT,
                TEXTURE_MAP_SET_TEXTURE,
                NI_TEXTURING_PROPERTY_SET_BASE_CLAMP_MODE,
                FN_00533FB0,
                NI_AV_OBJECT_ATTACH_PROPERTY,
            ],
        );
        e.register(NI_TEXTURING_PROPERTY_CONSTRUCT, |_, a| a[0].into_ret());
        e.register(ARRAY_ELEMENT_ADDRESS, |_, a| (a[0] + 4 * a[1]).into_ret());
        e.register(TEXTURE_MAP_CONSTRUCT, |_, a| a[0].into_ret());
        e.register(ARRAY_SET_ELEMENT, |e, a| {
            let value = e.mem.u32(a[2]);
            e.mem.set_u32(a[0] + 4 * a[1], value);
            Ret::default()
        });
        e.register(POINT3_ADD_ASSIGN, |e, a| {
            for i in 0..3 {
                let sum = e.mem.f32(a[0] + 4 * i) + e.mem.f32(a[1] + 4 * i);
                e.mem.set_f32(a[0] + 4 * i, sum);
            }
            Ret::default()
        });
        e.register(POINT3_SUBTRACT, |e, a| {
            for i in 0..3 {
                let difference = e.mem.f32(a[0] + 4 * i) - e.mem.f32(a[2] + 4 * i);
                e.mem.set_f32(a[1] + 4 * i, difference);
            }
            a[1].into_ret()
        });
        e.register(CELL_ADJUST_COORD_FOR_NORTH_ROTATION, |e, a| {
            assert_eq!(a[3], 1);
            for i in 0..3 {
                let shift = [100.0f32, 200.0, 0.0][i as usize];
                let value = e.mem.f32(a[1] + 4 * i) + shift;
                e.mem.set_f32(a[2] + 4 * i, value);
            }
            Ret::default()
        });
        e.register(CELL_GET_NORTH_ROTATION, |_, _| 0.25f32.into_ret());
        let data = e.mem.alloc(16);
        e.mem.set_f32(data + 8, 1.5);
        e.register_double(FN_00430830, move |_, _| data.into_ret());
        for (addr, value) in [
            (MARKER_TOP_Y, 1.5f32),
            (MARKER_RIGHT_X, 0.5),
            (MARKER_BOTTOM, -0.5),
        ] {
            e.set_global(addr, value);
        }
        for (i, value) in [0.1f32, 0.2, 0.3, 0.4].iter().enumerate() {
            e.set_global(DEFAULT_TILE_COLOUR + 4 * i as u32, *value);
        }
        let records = LocalMapRecords {
            shapes: Record::default(),
            tile_moves: Record::default(),
            node_moves: Record::default(),
            seen_points: Record::default(),
            triangles: Record::default(),
        };
        let record = records.shapes.clone();
        e.register_double(NI_TRI_SHAPE_CONSTRUCT, move |_, a| {
            record.borrow_mut().push(a.to_vec());
            a[0].into_ret()
        });
        let record = records.tile_moves.clone();
        e.register_double(NODE_SET_LOCAL_TRANSLATE_XYZ, move |_, a| {
            record.borrow_mut().push(a.to_vec());
            Ret::default()
        });
        let record = records.node_moves.clone();
        e.register_double(NODE_SET_LOCAL_TRANSLATE, move |e, a| {
            record.borrow_mut().push(vec![
                a[0],
                e.mem.u32(a[1]),
                e.mem.u32(a[1] + 4),
                e.mem.u32(a[1] + 8),
            ]);
            Ret::default()
        });
        let record = records.seen_points.clone();
        e.register_double(CELL_GET_SEEN_VALUE, move |e, a| {
            record.borrow_mut().push(vec![
                e.mem.u32(a[0]),
                e.mem.u32(a[0] + 4),
                e.mem.u32(a[0] + 8),
            ]);
            8u32.into_ret()
        });
        let record = records.triangles.clone();
        e.register_double(MAKE_TRIANGLE, move |_, a| {
            record.borrow_mut().push(a.to_vec());
            0x6000u32.into_ret()
        });
        records
    }

    #[test]
    fn test_local_map_builds_one_tile_and_the_marker_in_an_exterior() {
        let mut e = engine_b();
        let records = local_map_world(&mut e, [5000.0, 9000.0, 50.0], 1);
        parse_gives(&mut e, true, &[0]);
        e.register_double(WORLD_SPACE_GET_CELL_FROM_CELL_COORD, |_, a| {
            // The tile's cell: the player's cell (5000 >> 12, 9000 >> 12).
            assert_eq!(a, [0x3000, 1, 2]);
            0x3100u32.into_ret()
        });
        start_log(&mut e);
        assert!(e.call(0x005b_7b40, &args![script(0)]).bool());
        let root = calls(&e, NI_NODE_CONSTRUCT)[0][0];

        // Four element arrays and the index array were allocated.
        let constructed = calls(&e, VECTOR_CONSTRUCT_SIMPLE);
        assert_eq!(constructed.len(), 4);
        for (call, (size, constructor)) in constructed.iter().zip([
            (12, TRIVIAL_CONSTRUCT),
            (12, TRIVIAL_CONSTRUCT),
            (8, TRIVIAL_CONSTRUCT),
            (16, NI_COLOR_A_ZERO_CONSTRUCT),
        ]) {
            assert_eq!(call[1..], [size, 289, constructor]);
        }
        let shapes = records.shapes.borrow();
        assert_eq!(shapes.len(), 1);
        let shape = &shapes[0];
        let (positions, normals, colours, uvs, indices) = (
            constructed[0][0],
            constructed[1][0],
            constructed[3][0],
            constructed[2][0],
            shape[9],
        );
        assert_eq!(
            shape[1..],
            [289, positions, normals, colours, uvs, 1, 0, 512, indices]
        );

        // Positions: x = column * 4 - 32, y = row * 4 - 32, z = 0.
        let point = |i: u32| {
            words_at(&e, positions + 12 * i, 3)
                .iter()
                .map(|w| f32::from_bits(*w))
                .collect::<Vec<_>>()
        };
        assert_eq!(point(0), [-32.0, -32.0, 0.0]);
        assert_eq!(point(1), [-28.0, -32.0, 0.0]);
        assert_eq!(point(17), [-32.0, -28.0, 0.0]);
        assert_eq!(point(288), [32.0, 32.0, 0.0]);
        // Normals point up, the uvs follow the grid, the colours are the default.
        assert_eq!(
            words_at(&e, normals + 12 * 100, 3),
            [0, 0, 1.0f32.to_bits()]
        );
        assert_eq!(
            words_at(&e, uvs + 8, 2),
            [((1.0f64 / 17.0) as f32).to_bits(), 1.0f32.to_bits()]
        );
        assert_eq!(
            words_at(&e, uvs + 8 * 17, 2),
            [0, ((1.0f64 - 1.0 / 17.0) as f32).to_bits()]
        );
        assert_eq!(
            words_at(&e, colours + 16 * 288, 4),
            [0.1f32, 0.2, 0.3, 0.4].map(f32::to_bits)
        );
        // Indices: the diagonal alternates between neighbouring quads.
        let index = |k: u32| e.mem.u16(indices + 2 * k);
        assert_eq!(
            (0..12).map(index).collect::<Vec<_>>(),
            [18, 17, 0, 0, 1, 18, 18, 1, 2, 2, 19, 18]
        );
        assert_eq!(index(1535), 288);
        // The shape hangs on the root at the tile's place, 0 for one tile.
        let attached = calls(&e, V_ATTACH);
        assert_eq!(attached[0], vec![root, shape[0], 1]);
        assert_eq!(
            *records.tile_moves.borrow(),
            vec![vec![
                shape[0],
                0.0f32.to_bits(),
                0.0f32.to_bits(),
                0.0f32.to_bits()
            ]]
        );
        // The cell's texture became the base map of a new property.
        let property = calls(&e, NI_TEXTURING_PROPERTY_CONSTRUCT)[0][0];
        let map = e.mem.u32(property + 0x1c);
        assert_eq!(calls(&e, TEXTURE_MAP_SET_TEXTURE), vec![vec![map, 0x5555]]);
        assert_eq!(
            calls(&e, NI_AV_OBJECT_ATTACH_PROPERTY),
            vec![vec![shape[0], property]]
        );
        assert_eq!(calls(&e, NI_POINTER_SET).len(), 1);
        assert_eq!(calls(&e, NI_POINTER_DESTRUCT).len(), 1);

        // The marker: a red triangle at the player's offset from the centre of
        // the player's cell, turned by the north rotation + 1.5.
        let triangles = records.triangles.borrow();
        assert_eq!(triangles.len(), 1);
        let float_words = |values: [f32; 9]| values.map(f32::to_bits);
        assert_eq!(
            triangles[0][..9],
            float_words([0.0, 1.5, 1.0, 0.5, -0.5, 1.0, -0.5, -0.5, 1.0])
        );
        assert_eq!(triangles[0][10], 1);
        let colour = triangles[0][9];
        assert_eq!(
            calls(&e, NI_COLOR_A_CONSTRUCT)[0],
            vec![colour, 1.0f32.to_bits(), 0, 0, 0]
        );
        let moves = records.node_moves.borrow();
        assert_eq!(
            moves[0],
            vec![
                0x6000,
                (-17.875f32).to_bits(),
                (-19.375f32).to_bits(),
                0.0f32.to_bits()
            ]
        );
        assert_eq!(attached[1], vec![root, 0x6000, 1]);
        assert_eq!(calls(&e, NI_MATRIX_FROM_Z_ANGLE)[0][1], 1.75f32.to_bits());
        // The root goes to the player's position, z raised by 10.
        assert_eq!(
            moves[1],
            vec![
                root,
                5000.0f32.to_bits(),
                9000.0f32.to_bits(),
                60.0f32.to_bits()
            ]
        );
        assert_debug_node_shown(&e, root);
    }

    #[test]
    fn test_local_map_colours_the_vertices_by_the_seen_value() {
        let mut e = engine_b();
        let records = local_map_world(&mut e, [5000.0, 9000.0, 50.0], 1);
        parse_gives(&mut e, true, &[1]);
        start_log(&mut e);
        assert!(e.call(0x005b_7b40, &args![script(0)]).bool());
        let constructed = calls(&e, VECTOR_CONSTRUCT_SIMPLE);
        let colours = constructed[3][0];
        // 17 x 17 samples, column * 2 (the scale), row * 2, 0 plus the tile
        // origin (1 << 12, 2 << 12, 0).
        let seen = records.seen_points.borrow();
        assert_eq!(seen.len(), 289);
        let sample = |x: f32, y: f32| vec![x.to_bits(), y.to_bits(), 0.0f32.to_bits()];
        assert_eq!(seen[0], sample(4096.0, 8192.0));
        assert_eq!(seen[1], sample(4098.0, 8192.0));
        assert_eq!(seen[17], sample(4096.0, 8194.0));
        assert_eq!(seen[288], sample(4128.0, 8224.0));
        // A seen value of 8 divided by 4 is the colour (2, 2, 2, 0).
        for vertex in [0, 100, 288] {
            assert_eq!(
                words_at(&e, colours + 16 * vertex, 4),
                [2.0f32, 2.0, 2.0, 0.0].map(f32::to_bits)
            );
        }
    }

    #[test]
    fn test_local_map_in_an_interior_uses_the_adjusted_position() {
        let mut e = engine_b();
        let records = local_map_world(&mut e, [10000.0, 20000.0, 5.0], 1);
        e.register(REFR_GET_INTERIOR, |_, _| true.into_ret());
        accept(
            &mut e,
            &[
                CELL_GET_INT_SEEN_SECTION,
                CELL_GET_INTERIOR_LOCAL_MAP_TEXTURE,
            ],
        );
        parse_gives(&mut e, true, &[0]);
        start_log(&mut e);
        assert!(e.call(0x005b_7b40, &args![script(0)]).bool());
        // The adjusted position is (10100, 20200): cell ((x - 0x800) >> 12,
        // ...) = (1, 4); the seen section and the texture are asked for it.
        let slot = calls(&e, CELL_GET_INTERIOR_LOCAL_MAP_TEXTURE)[0][3];
        assert_eq!(
            calls(&e, CELL_GET_INT_SEEN_SECTION),
            vec![vec![0x8000, 1, 4, 0]]
        );
        assert_eq!(
            calls(&e, CELL_GET_INTERIOR_LOCAL_MAP_TEXTURE),
            vec![vec![0x8000, 1, 4, slot]]
        );
        assert!(calls(&e, WORLD_SPACE_GET_CELL_FROM_CELL_COORD).is_empty());
        // The marker's centre is the middle of the cell (1, 4) plus one cell:
        // (8192, 20480); the player's offset from it is in tile units.
        let moves = records.node_moves.borrow();
        assert_eq!(
            moves[0],
            vec![
                0x6000,
                29.8125f32.to_bits(),
                (-4.375f32).to_bits(),
                0.0f32.to_bits()
            ]
        );
        // The root goes to the unadjusted position.
        assert_eq!(
            moves[1][1..],
            [
                10000.0f32.to_bits(),
                20000.0f32.to_bits(),
                15.0f32.to_bits()
            ]
        );
        drop(moves);
        // Without a parent cell the tile asks for nothing.
        e.register(REFR_GET_PARENT_CELL, |_, _| 0u32.into_ret());
        start_log(&mut e);
        assert!(e.call(0x005b_7b40, &args![script(0)]).bool());
        assert!(calls(&e, CELL_GET_INT_SEEN_SECTION).is_empty());
    }

    #[test]
    fn test_local_map_covers_a_grid_of_tiles() {
        let mut e = engine_b();
        let records = local_map_world(&mut e, [5000.0, 9000.0, 50.0], 2);
        parse_gives(&mut e, true, &[0]);
        let cells = std::rc::Rc::new(std::cell::RefCell::new(vec![]));
        let record = cells.clone();
        e.register_double(WORLD_SPACE_GET_CELL_FROM_CELL_COORD, move |_, a| {
            record.borrow_mut().push((a[1], a[2]));
            0x3100u32.into_ret()
        });
        start_log(&mut e);
        assert!(e.call(0x005b_7b40, &args![script(0)]).bool());
        // 2 x 2 tiles around the player's cell (1, 2): cells (0..2, 1..3).
        assert_eq!(*cells.borrow(), vec![(0, 1), (1, 1), (0, 2), (1, 2)]);
        // The tiles are 64 wide and centred on the whole 128 wide map.
        let moves = records.tile_moves.borrow();
        let xy: Vec<(f32, f32)> = moves
            .iter()
            .map(|m| (f32::from_bits(m[1]), f32::from_bits(m[2])))
            .collect();
        assert_eq!(
            xy,
            [(-32.0, -32.0), (32.0, -32.0), (-32.0, 32.0), (32.0, 32.0)]
        );
        assert_eq!(records.shapes.borrow().len(), 4);
        // Bad parameters: nothing is built.
        parse_gives(&mut e, false, &[]);
        start_log(&mut e);
        assert!(!e.call(0x005b_7b40, &args![script(0)]).bool());
        assert!(calls(&e, NI_ALLOC).is_empty());
    }
}
