//! `fallout shared/modelloader.cpp` (Xbox PDB source unit), part 3: its functions from `004457d0` up to
//! (not including) `00449c00` in FalloutNV.exe 1.4.0.525, translated
//! (docs/ENGINE_CRATE.md). The unit's shared layouts and helpers are in
//! [`super::modelloader`]; anything public there may be used here.
//!
//! State of this file: the first 40 functions, `004457d0` to `00447300`:
//! `QueueCreatureParts`, `QueueAnimations` and the list helpers they use
//! (`004463b0`, `00446500`, `004465f0`), the replacement KF list's
//! constructor and virtual functions, the reference re-prioritising loop
//! (`00446b50`), the loading-status text (`00446cb0`) and the small getters
//! around the background clone thread, then the model and KF map inserts,
//! `LoadFile`, `LoadKF`, `FindModel` and `BuildFileList`. The next
//! function, `00447330` (`BuildKFFileList`), is the first of the next
//! session.
//!
//! The game's `char *` lists passed around here are `BSSimpleList<char *>`
//! heads: the head node holds the first item inline (item at +0, next node
//! at +4); `006815c0` returns the node's item address and `00726070` the
//! next node. `0063f7b0` removes the head (the next node's contents move
//! into it and the next node is deleted); `004702f0(list, 1)` deletes a
//! list.
//!
//! Not translated: the compiler's exception-unwinding frames (`FS:[0]`
//! chains and state variables) and the stack-cookie checks. Locals the game
//! keeps on its stack and passes by address are `with_stack` blocks.
//!
//! Callees in other files of the unit are called by address.

#[allow(unused_imports)]
use super::modelloader::*;
#[allow(unused_imports)]
use crate::prelude::*;

/// `MemoryManager` allocation, `__cdecl(size) -> block` (`00401000`).
const MEMORY_ALLOC: u32 = 0x0040_1000;
/// `MemoryManager` deallocation, `__cdecl(block)` (`00401030`).
const MEMORY_FREE: u32 = 0x0040_1030;
/// `strcpy_s(destination, size, source)` through the game's wrapper
/// (`00406d30`).
const STRING_COPY: u32 = 0x0040_6d30;
/// `strcat_s(destination, size, source)` through the game's wrapper
/// (`00406d50`).
const STRING_CONCAT: u32 = 0x0040_6d50;
/// `sprintf_s(buffer, size, format, ...)` through the game's wrapper
/// (`00406d00`).
const FORMAT_BUFFER: u32 = 0x0040_6d00;
/// `strrchr(text, character)` (`0040ab30`).
const STRRCHR: u32 = 0x0040_ab30;
/// `_strnicmp(first, second, count)` (`00ec7ec0`).
const STRNICMP: u32 = 0x00ec_7ec0;
/// `sprintf(buffer, format, ...)` (`00ec623a`).
const SPRINTF: u32 = 0x00ec_623a;
/// `strlen` through the game's wrapper (`0044a670`).
const STRLEN: u32 = 0x0044_a670;
/// `__RTDynamicCast(object, offset, source type, target type, reference)`
/// (`00ec43fb`).
const RT_DYNAMIC_CAST: u32 = 0x00ec_43fb;
/// The game's `printf`-style log (`005b5e40`), `__cdecl(format, ...)`.
const LOG: u32 = 0x005b_5e40;

/// List node accessors: the item address of a node (`006815c0`, the node
/// itself) and the next node (`00726070`, the dword at +4).
const LIST_NODE_ITEM_ADDRESS: u32 = 0x0068_15c0;
const LIST_NEXT_NODE: u32 = 0x0072_6070;
/// Removes the head of a list (`0063f7b0`).
const LIST_REMOVE_HEAD: u32 = 0x0063_f7b0;
/// Deletes a list, `__thiscall(list, flags)` (`004702f0`).
const LIST_DELETE: u32 = 0x0047_02f0;
/// Constructs an empty list in a block of 8 bytes (`0096a2d0`; the engine
/// map files it under another unit).
const LIST_CONSTRUCT: u32 = 0x0096_a2d0;
/// Appends the item whose address is passed (`005ae3d0`).
const LIST_PUSH_BACK: u32 = 0x005a_e3d0;
/// The dword at +8 of an object (`0044ddc0`).
const FIELD_AT_8: u32 = 0x0044_ddc0;
/// `NiPointer` read (`00559450`): the pointer stored at the address.
const POINTER_GET: u32 = 0x0055_9450;
/// `NiPointer<QueuedFile>::operator_` (Xbox PDB, `006f74f0`),
/// `__thiscall(object)`.
const QUEUED_FILE_POINTER_ASSIGN: u32 = 0x006f_74f0;
/// Smart pointer to a task: constructor with a reference (`00528cb0`) and
/// destructor (`0044cbf0`).
const TASK_POINTER_CONSTRUCT: u32 = 0x0052_8cb0;
const TASK_POINTER_DESTRUCT: u32 = 0x0044_cbf0;
/// `NiPointer<Model>::operator=(Model*)` (`0044aed0`).
const MODEL_POINTER_ASSIGN: u32 = 0x0044_aed0;
/// The scope guard that sets the memory context for the rest of a function:
/// `__thiscall(context, 1, source file, line)` and its destructor.
const MEMORY_CONTEXT_ENTER: u32 = 0x0040_4eb0;
const MEMORY_CONTEXT_LEAVE: u32 = 0x0040_4ee0;
/// `QueuedFile::QueuedFile(context)` (Xbox PDB, `00c3c590`).
const QUEUED_FILE_CONSTRUCT: u32 = 0x00c3_c590;
/// `QueuedFile::CheckFinished` (Xbox PDB, `00c3c930`).
const QUEUED_FILE_CHECK_FINISHED: u32 = 0x00c3_c930;
/// The `QueuedFile` destructor body the queued classes share (`0043dc50`).
const QUEUED_FILE_DESTRUCT: u32 = 0x0043_dc50;
/// `QueuedFile` method (`00443aa0`, no name in the engine map),
/// `__thiscall(parent)`: stores `spParent` and tells the parent.
const QUEUED_FILE_SET_PARENT: u32 = 0x0044_3aa0;
/// Puts a queued task into state 5 (`00449150`: `this + 0xC = 5`).
const TASK_SET_DONE: u32 = 0x0044_9150;
/// `QueuedReplacementKF` constructor (`0043e8b0`), `__thiscall(name,
/// context, animation, owner)`.
const QUEUED_REPLACEMENT_KF_CONSTRUCT: u32 = 0x0043_e8b0;
/// `QueuedModel` constructor taking a file name (`0043c6e0`), `__thiscall(name,
/// context, LOD multiplier, flag, flag)`.
const QUEUED_MODEL_CONSTRUCT: u32 = 0x0043_c6e0;
/// `QueuedModel` method (`00443ff0`, `__thiscall(flag)`): sets or clears
/// bit 0x20 of `cFlags`.
const QUEUED_MODEL_SET_FLAG_20: u32 = 0x0044_3ff0;
/// `QueuedModel::Run` (Xbox PDB, `0043ccf0`), the finishing call `LoadFile`
/// makes (`0043d180`) and the destructor (`0043c830`).
const QUEUED_MODEL_RUN: u32 = 0x0043_ccf0;
const QUEUED_MODEL_FINISH: u32 = 0x0043_d180;
const QUEUED_MODEL_DESTRUCT: u32 = 0x0043_c830;
/// `__cdecl(flag, byte pointer)` (`0044af70`): sets (non-zero `flag`) or
/// clears bit 0x10 of the byte.
const SET_FLAG_BIT_10: u32 = 0x0044_af70;
/// `QueuedKF` constructor (`0043e0f0`, `__thiscall(name, context)`), `Run`
/// (`0043e2a0`), the finishing call (`0043e4b0`) and destructor
/// (`0043e1b0`).
const QUEUED_KF_CONSTRUCT: u32 = 0x0043_e0f0;
const QUEUED_KF_RUN: u32 = 0x0043_e2a0;
const QUEUED_KF_FINISH: u32 = 0x0043_e4b0;
const QUEUED_KF_DESTRUCT: u32 = 0x0043_e1b0;
/// `Animation` method (`00490fa0`), `__thiscall(KFModel)`: takes the loaded
/// `KFModel` into the animation.
const ANIMATION_KF_LOADED: u32 = 0x0049_0fa0;
/// Interlocked increment of the dword at +0x10 (`0043be10`): for a
/// `KFModel` that is `iManualRefCount`.
const KF_MODEL_ADD_MANUAL_REFERENCE: u32 = 0x0043_be10;
/// `NiRefObject::IncRefCount` (`0092c870`).
const ADD_REFERENCE: u32 = 0x0092_c870;
/// The `NiPointer` at +0xC of an object, dereferenced (`0043b230`): for a
/// `Model` that is `spObject3D`.
const MODEL_OBJECT_3D: u32 = 0x0043_b230;
/// `iRefCount + iManualRefCount` of a `Model` (`00443190`) and of a
/// `KFModel` (`004431b0`).
const MODEL_REFERENCE_TOTAL: u32 = 0x0044_3190;
const KF_MODEL_REFERENCE_TOTAL: u32 = 0x0044_31b0;
/// `ModelLoader::QueueModel` taking a name (`00444040`), `__thiscall(name,
/// key, parent, 0, 1, 0, 0)`.
const MODEL_LOADER_QUEUE_MODEL: u32 = 0x0044_4040;
/// `ModelLoader` method (`004442c0`), `__thiscall(name, entry, key, parent,
/// 0, 1, 0, 0)`: queues a model together with an entry of the model's list.
const MODEL_LOADER_QUEUE_MODEL_WITH_ENTRY: u32 = 0x0044_42c0;
/// `ModelLoader` method (`004446a0`), `__thiscall(name, key, parent)`:
/// queues one KF file.
const MODEL_LOADER_QUEUE_KF: u32 = 0x0044_46a0;
/// `ModelLoader` method (`00444d40`), `__thiscall(queued object, key, parent,
/// actor)`.
const MODEL_LOADER_QUEUE_PARTS: u32 = 0x0044_4d40;
/// `ModelLoader::CopyFilenameList` (Xbox PDB, `00447850`), `__thiscall(source
/// list, destination list)`.
const MODEL_LOADER_COPY_FILENAME_LIST: u32 = 0x0044_7850;
/// `ModelLoader::BuildKFFileList` (Xbox PDB, `00447330`), `__thiscall(path,
/// 1, 1, creature)`.
const MODEL_LOADER_BUILD_KF_FILE_LIST: u32 = 0x0044_7330;
/// `afe420(path, name, 1, flag)` (cdecl), which `BuildFileList` forwards to.
const BUILD_FILE_LIST: u32 = 0x00af_e420;
/// `TESIdleManager::GetRootFilenameList` (Xbox PDB, `00600700`),
/// `__thiscall(path, 0)` on the idle manager at [`IDLE_MANAGER`].
const GET_ROOT_FILENAME_LIST: u32 = 0x0060_0700;
/// `Actor::GetCurrentWeapon` (Xbox PDB, `008a1710`).
const ACTOR_GET_CURRENT_WEAPON: u32 = 0x008a_1710;
/// Actor method (`008a1760`, no name in the engine map) the KF list takes as
/// its creature argument.
const ACTOR_CREATURE_FORM: u32 = 0x008a_1760;
/// Actor method (`0087f4c0`, in the actor unit): 1 for a female actor.
const ACTOR_GET_SEX: u32 = 0x0087_f4c0;
/// `TESAnimation::HasKFFiles` (Xbox PDB, `0047fd90`).
const TESANIMATION_HAS_KF_FILES: u32 = 0x0047_fd90;
/// The list of an object cast to its animation or creature class
/// (`00717e50`: `this + 4`).
const LIST_HEAD_OF_OBJECT: u32 = 0x0071_7e50;
/// `TESModel` accessor (`0048d150`: `this + 0xC`), the entry array.
const MODEL_ENTRIES: u32 = 0x0048_d150;
/// `InventoryChanges::GetInventoryChanges` (Xbox PDB, `004bf220`), `__cdecl(actor)`.
const GET_INVENTORY_CHANGES: u32 = 0x004b_f220;
/// `InventoryChanges` methods (unit `inventorychanges.cpp`; `004c7400`,
/// `__thiscall(form, float out, 6, 0)`; `004c6f60`, `__thiscall(form, 0)`),
/// each giving an object that `QueueCreatureParts` hands to `00444d40`
/// and then deletes ([`fn_004459e0`]).
const INVENTORY_FIND_FIRST: u32 = 0x004c_7400;
const INVENTORY_FIND_SECOND: u32 = 0x004c_6f60;
/// Destructor body of the object `004c7400` returns (`004bc5f0`).
const INVENTORY_RESULT_DESTRUCT: u32 = 0x004b_c5f0;

/// Form type ids (`00401170`) that `00446a60` tests: `0x28` (its
/// `00522d80` is filed under `tesobjectweap.cpp` in the engine map) and
/// `0x2a` (the owner whose sex it asks for).
const WEAPON_FORM_TYPE: u32 = 0x28;
const ACTOR_BASE_FORM_TYPE: u32 = 0x2a;
/// A reference's form (`007af430`, the engine map's folded
/// `BGSSaveFormBuffer::GetForm`) and `TESObjectREFR::GetTESModel` (Xbox
/// PDB, `00571630`).
const REFERENCE_GET_FORM: u32 = 0x007a_f430;
const REFERENCE_GET_TES_MODEL: u32 = 0x0057_1630;
/// Form type id getter (`00401170`).
const FORM_TYPE: u32 = 0x0040_1170;
/// `00522d80` (model of a type `0x28` form), `TESObjectREFR::GetOwner` (Xbox
/// PDB, `00567790`), `TESActorBase::GetSex` (Xbox PDB, `005f0cc0`) and
/// `TESBipedModelForm::GetWorldTESModel_ov2` (Xbox PDB, `00481110`).
const WEAPON_GET_MODEL: u32 = 0x0052_2d80;
const REFERENCE_GET_OWNER: u32 = 0x0056_7790;
const ACTOR_BASE_GET_SEX: u32 = 0x005f_0cc0;
const BIPED_GET_WORLD_MODEL: u32 = 0x0048_1110;

/// `LockFreeMapIterator` constructor (`0044cc10`) and destructor
/// (`004499c0`), the "is done" test (`006ebde0`) and the map's
/// next-entry call (`00906570`, `__thiscall(iterator, key out, value out,
/// 1) -> bool`).
const MAP_ITERATOR_CONSTRUCT: u32 = 0x0044_cc10;
const MAP_ITERATOR_DESTRUCT: u32 = 0x0044_99c0;
const MAP_ITERATOR_IS_DONE: u32 = 0x006e_bde0;
const MAP_NEXT_ENTRY: u32 = 0x0090_6570;
/// The cell of a reference (`008d6f30`, no name in the engine map; its
/// result is the cell `TES::GetCellPriority` is asked about).
const REFERENCE_CELL: u32 = 0x008d_6f30;
/// `TESObjectREFR::Is3DCritical` (Xbox PDB, `00572350`).
const REFERENCE_IS_3D_CRITICAL: u32 = 0x0057_2350;
/// `TES::GetCellPriority` (Xbox PDB, `00458be0`), `__thiscall(cell, 0)` on
/// the object at [`TES_GLOBAL`].
const GET_CELL_PRIORITY: u32 = 0x0045_8be0;
/// The priority byte of a task's key (`0043cc60`).
const TASK_PRIORITY: u32 = 0x0043_cc60;

/// `0040fc90`: the current thread id. `00442600`, `00442510` and `00442530`
/// are the calls the loader makes on the clone thread (no names in the
/// engine map).
const CURRENT_THREAD_ID: u32 = 0x0040_fc90;
const CLONE_THREAD_CALL_FIRST: u32 = 0x0044_2600;
const CLONE_THREAD_CALL_SECOND: u32 = 0x0044_2510;
const CLONE_THREAD_CALL_THIRD: u32 = 0x0044_2530;
/// `004491f0(task manager)`: the task count.
const TASK_MANAGER_TASK_COUNT: u32 = 0x0044_91f0;
/// Setting getter (`00408d60`, `__thiscall` on a setting object: `this + 4`,
/// or the address of a zero scratch byte when `this` is 0).
const SETTING_VALUE_POINTER: u32 = 0x0040_8d60;
/// `SetWindowTextA` (import slot `00fdf2dc`), `__stdcall(window, text)`.
const SET_WINDOW_TEXT: u32 = 0x00fd_f2dc;
/// `ModelLoader` methods `00448c10` and `00448cc0` (`00446e40` ANDs them).
const LOADER_CHECK_FIRST: u32 = 0x0044_8c10;
const LOADER_CHECK_SECOND: u32 = 0x0044_8cc0;
/// `00878080(0x11deffc, 0)`, `0082fb70()`, the test `0047c850` and
/// `00861ea0`, the last two called on the object at [`MAIN_LOOP_OBJECT`].
const REFRESH_FIRST: u32 = 0x0087_8080;
const REFRESH_SECOND: u32 = 0x0082_fb70;
const MAIN_LOOP_TEST: u32 = 0x0047_c850;
const MAIN_LOOP_UPDATE: u32 = 0x0086_1ea0;

/// The loader object (`011c3b3c`).
const LOADER: u32 = 0x011c_3b3c;
/// The task manager (`01202d98`).
const TASK_MANAGER: u32 = 0x0120_2d98;
/// The object of `TES::GetCellPriority` (`011dea10`).
const TES_GLOBAL: u32 = 0x011d_ea10;
/// The idle manager (`011cb6a0`).
const IDLE_MANAGER: u32 = 0x011c_b6a0;
/// The object whose +8 is the window `00446cb0` sets the text of
/// (`011dea0c`) and the text (`011a2fe8`).
const WINDOW_OWNER: u32 = 0x011d_ea0c;
const WINDOW_TEXT: u32 = 0x011a_2fe8;
/// The object `00446ea0` tests (`011de45c`).
const MAIN_LOOP_OBJECT: u32 = 0x011d_e45c;
/// The setting object `00446e10` reads (`011c77b4`).
const LOADING_SETTING: u32 = 0x011c_77b4;
/// The byte flag `00446cb0` keeps (`011c3bfc`): set while it shows a status,
/// cleared when it resets the window text.
const STATUS_SHOWN_FLAG: u32 = 0x011c_3bfc;
/// The address `00446ef0` returns (`011deffc`).
const REFRESH_OBJECT: u32 = 0x011d_effc;
/// Two tables `QueueAnimations` indexes with the weapon type: dwords at
/// `0118a838` give an index into the string pointers at `011977a4` (the
/// holster file prefix of the weapon type).
const HOLSTER_INDEX_TABLE: u32 = 0x0118_a838;
const HOLSTER_PREFIX_TABLE: u32 = 0x0119_77a4;

/// Type descriptors `__RTDynamicCast` is given: the class every cast starts
/// from (`011831e8`; the model class), the creature class (`01183a00`) and
/// the animation class (`0118584c`) of `QueueCreatureParts` and
/// `QueueAnimations`; and `00446a60`'s source (`01183108`) with its targets
/// `011831e8` and `01183978` (the biped class).
const SOURCE_TYPE: u32 = 0x0118_31e8;
const CREATURE_TYPE: u32 = 0x0118_3a00;
const ANIMATION_TYPE: u32 = 0x0118_584c;
const FORM_SOURCE_TYPE: u32 = 0x0118_3108;
const FORM_MODEL_TYPE: u32 = 0x0118_31e8;
const FORM_BIPED_TYPE: u32 = 0x0118_3978;

/// `QueuedReplacementKFList`'s virtual table (`01016fc0`).
const QUEUED_REPLACEMENT_KF_LIST_VTABLE: u32 = 0x0101_6fc0;
/// `"D:\_Fallout3\Platforms\Common\Code\Fallout Shared\ModelLoader.cpp"`.
const MODEL_LOADER_SOURCE: u32 = 0x0101_6840;
/// The memory context (`0x33`) and source line of the scope `004465f0`
/// opens.
const REPLACEMENT_CONTEXT: u32 = 0x33;
const REPLACEMENT_SOURCE_LINE: u32 = 0xe9f;

/// Strings: `"Skeleton"`, `"Data\\"`, `"Meshes\\"`, `"\\MTIdle.KF"`,
/// `"\\%sHolster.KF"`, `"\\PA%sHolster.KF"`, `"\\Death.KF"`,
/// `"\\Locomotion\\Child\\IdleAnims"`, `"\\Locomotion\\Female\\IdleAnims"`,
/// `"\\Locomotion\\Male\\IdleAnims"`, `"\\SpecialAnims\\"`.
const SKELETON_WORD: u32 = 0x0101_3458;
const DATA_PREFIX: u32 = 0x0101_6fb4;
const MESHES_PREFIX: u32 = 0x0101_6fac;
const MTIDLE_FILE: u32 = 0x0101_6fa0;
const HOLSTER_FORMAT: u32 = 0x0101_6f90;
const POWER_ARMOR_HOLSTER_FORMAT: u32 = 0x0101_6f80;
const DEATH_FILE: u32 = 0x0101_6f74;
const CHILD_IDLE_DIRECTORY: u32 = 0x0101_6f58;
const FEMALE_IDLE_DIRECTORY: u32 = 0x0101_6f38;
const MALE_IDLE_DIRECTORY: u32 = 0x0101_6f1c;
const SPECIAL_ANIMS_DIRECTORY: u32 = 0x0101_6f0c;
/// `"Loading %d references( %d background cloning ) and %d tasks( %d post
/// processing )"`.
const LOADING_STATUS_FORMAT: u32 = 0x0101_6ff8;
/// `"MODELS: LoadFile retrieved model %s with a zero ref count"` and the
/// KF version.
const LOAD_FILE_ZERO_REFERENCE_MESSAGE: u32 = 0x0101_704c;
const LOAD_KF_ZERO_REFERENCE_MESSAGE: u32 = 0x0101_7088;

/// Size of the path buffers (`strcpy_s` is told this size).
const PATH_BUFFER_SIZE: u32 = 0x104;
/// The backslash `strrchr` looks for.
const BACKSLASH: u32 = 0x5c;
/// Size of the blocks the lists allocate: a name list, a
/// `QueuedReplacementKFList` and a `QueuedReplacementKF`.
const LIST_SIZE: u32 = 8;
const QUEUED_REPLACEMENT_KF_LIST_SIZE: u32 = 0x38;
const QUEUED_REPLACEMENT_KF_SIZE: u32 = 0x40;
/// Size of a `QueuedKF` the game keeps on the stack in `LoadKF`.
const QUEUED_KF_SIZE: u32 = 0x38;

/// Runs `body` with the file name to queue for the list item `item`: the
/// item itself, or, when `prefix` is not null, the prefix and the item
/// concatenated in a 260-byte buffer.
fn with_prefixed_name<R>(
    e: &mut Engine,
    prefix: u32,
    item: u32,
    body: impl FnOnce(&mut Engine, u32) -> R,
) -> R {
    if prefix == 0 {
        return body(e, item);
    }
    e.with_stack(PATH_BUFFER_SIZE, |e, buffer| {
        e.call(STRING_COPY, &args![buffer, PATH_BUFFER_SIZE, prefix]);
        e.call(STRING_CONCAT, &args![buffer, PATH_BUFFER_SIZE, item]);
        body(e, buffer.addr())
    })
}

/// The item of the list node `node` (the dword at the node's item address).
fn node_item(e: &mut Engine, node: u32) -> u32 {
    let slot = e.call(LIST_NODE_ITEM_ADDRESS, &args![node]).u32();
    e.mem.u32(slot)
}

/// Hands the list's files to `00446500` and deletes the list afterwards (the
/// pattern `QueueAnimations` repeats for each list it builds).
fn queue_and_delete_list(e: &mut Engine, this: Ptr, list: u32, key: u32, parent: Ptr) {
    fn_00446500(e, this, Ptr::new(list), key, parent, 0);
    if list != 0 {
        e.call(LIST_DELETE, &args![list, 1u32]);
    }
}

/// Whether the name after the separator at `separator` is `Skeleton`.
fn is_skeleton_name(e: &mut Engine, separator: u32) -> bool {
    e.call(STRNICMP, &args![separator + 1, SKELETON_WORD, 8u32])
        .u32()
        == 0
}

/// A new empty list in an 8-byte block (0 when the allocation failed).
fn new_list(e: &mut Engine) -> u32 {
    let block = e.call(MEMORY_ALLOC, &args![LIST_SIZE]).u32();
    if block == 0 {
        0
    } else {
        e.call(LIST_CONSTRUCT, &args![block]).u32()
    }
}

// Translated from 004457d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ModelLoader::QueueCreatureParts` (Xbox PDB): for a model that casts to
/// the creature class, queues the models of the creature's list (the list
/// at `+0x118` of the cast, with the entry array `+0x120`, and the directory
/// of the model's own path as prefix) with [`fn_004463b0`]. With an `actor`,
/// the two objects `InventoryChanges` finds for the creature
/// (`004c7400(cast, float out, 6, 0)` and `004c6f60(cast, 0)`) are each
/// handed to `00444d40` (their dword at +8) and deleted
/// ([`fn_004459e0`]).
pub fn model_loader_queue_creature_parts(
    e: &mut Engine,
    this: Ptr,
    model: Ptr,
    key: u32,
    parent: Ptr,
    actor: Ptr,
) {
    let creature = e
        .call(
            RT_DYNAMIC_CAST,
            &args![model, 0u32, SOURCE_TYPE, CREATURE_TYPE, 0u32],
        )
        .ptr::<()>();
    if creature.is_null() {
        return;
    }
    // The creature's sub-object at +0x114 holds the list and the entries.
    let part = creature.addr() + 0x114;
    let list = e.call(LIST_HEAD_OF_OBJECT, &args![part]).u32();
    let model_path = e.vcall(model.addr(), 0x14, &args![]).u32();
    e.with_stack(PATH_BUFFER_SIZE, |e, directory| {
        e.call(STRING_COPY, &args![directory, PATH_BUFFER_SIZE, model_path]);
        let separator = e.call(STRRCHR, &args![directory, BACKSLASH]).u32();
        if separator != 0 {
            e.mem.set_u8(separator + 1, 0);
        }
        let entries = e.call(MODEL_ENTRIES, &args![part]).u32();
        fn_004463b0(
            e,
            this,
            Ptr::new(list),
            Ptr::new(entries),
            key,
            parent,
            directory.addr(),
        );
        if actor.is_null() {
            return;
        }
        let changes = e.call(GET_INVENTORY_CHANGES, &args![actor]).u32();
        let first = e.with_stack(4, |e, float_out| {
            e.call(
                INVENTORY_FIND_FIRST,
                &args![changes, creature, float_out, 6u32, 0u32],
            )
            .u32()
        });
        if first != 0 {
            let queued = e.call(FIELD_AT_8, &args![first]).u32();
            e.call(
                MODEL_LOADER_QUEUE_PARTS,
                &args![this, queued, key, parent, actor],
            );
            fn_004459e0(e, Ptr::new(first), 1);
        }
        let second = e
            .call(INVENTORY_FIND_SECOND, &args![changes, creature, 0u32])
            .u32();
        if second != 0 {
            let queued = e.call(FIELD_AT_8, &args![second]).u32();
            e.call(
                MODEL_LOADER_QUEUE_PARTS,
                &args![this, queued, key, parent, actor],
            );
            fn_004459e0(e, Ptr::new(second), 1);
        }
    });
}

// Translated from 004459e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Scalar deleting destructor of the object `InventoryChanges` hands to
/// `QueueCreatureParts` (no name in the engine map): runs its destructor
/// body (`004bc5f0`) and, when bit 0 of `flags` is set, frees the object.
/// Returns `this`.
pub fn fn_004459e0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(INVENTORY_RESULT_DESTRUCT, &args![this]);
    if flags & 1 != 0 {
        e.call(MEMORY_FREE, &args![this]);
    }
    this
}

// Translated from 00445a10 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ModelLoader::QueueAnimations` (Xbox PDB): queues the animation files of
/// the model `model` (its virtual function `0x14` gives the path).
///
/// With an `actor` whose virtual function `0x22c(1)` is true only an actor
/// skeleton is handled ([`queue_skeleton_animations`]) and only for a path
/// whose last component is `Skeleton...`; the function ends there.
///
/// Otherwise the files come from one of two sources. When the model is a
/// skeleton or `second_flag` is set, and `first_flag` is set, it is the KF
/// file list of the path (`BuildKFFileList`, with the actor's `008a1760` as
/// creature when `actor` passes its `0x100` test). Else the actor's virtual
/// function `0x218` is tested (the game does not check `actor` for null
/// here) and, when true, the idle list ([`queue_idle_animations`]). Last, a
/// model that casts to the animation class and `HasKFFiles` queues the
/// cast's own list of file names ([`queue_special_animations`]).
///
/// Every list that is built is passed to `00446500` and deleted.
#[allow(clippy::too_many_arguments)]
pub fn model_loader_queue_animations(
    e: &mut Engine,
    this: Ptr,
    model: Ptr,
    key: u32,
    parent: Ptr,
    actor: Ptr,
    first_flag: u8,
    second_flag: u8,
) {
    let model_path = e.vcall(model.addr(), 0x14, &args![]).u32();
    e.with_stack(PATH_BUFFER_SIZE, |e, path| {
        e.call(STRING_COPY, &args![path, PATH_BUFFER_SIZE, model_path]);
        let last_separator = e.call(STRRCHR, &args![path, BACKSLASH]).u32();
        let skeleton_actor = !actor.is_null() && e.vcall(actor.addr(), 0x22c, &args![1u32]).bool();
        if skeleton_actor {
            if last_separator != 0 && is_skeleton_name(e, last_separator) {
                queue_skeleton_animations(e, this, key, parent, actor, path);
            }
            return;
        }
        let separator = e.call(STRRCHR, &args![path, BACKSLASH]).u32();
        let is_skeleton = separator != 0 && is_skeleton_name(e, separator);
        if (is_skeleton || second_flag != 0) && first_flag != 0 {
            let mut creature = 0;
            if !actor.is_null() && e.vcall(actor.addr(), 0x100, &args![]).bool() {
                creature = e.call(ACTOR_CREATURE_FORM, &args![actor]).u32();
            }
            let list = e
                .call(
                    MODEL_LOADER_BUILD_KF_FILE_LIST,
                    &args![this, path, 1u32, 1u32, creature],
                )
                .u32();
            queue_and_delete_list(e, this, list, key, parent);
        } else if e.vcall(actor.addr(), 0x218, &args![]).bool() {
            queue_idle_animations(e, this, key, parent, actor, path);
        }
        let animation = e
            .call(
                RT_DYNAMIC_CAST,
                &args![model, 0u32, SOURCE_TYPE, ANIMATION_TYPE, 0u32],
            )
            .u32();
        if animation != 0 && e.call(TESANIMATION_HAS_KF_FILES, &args![animation]).bool() {
            queue_special_animations(e, this, key, parent, model, animation);
        }
    });
}

/// The skeleton part of [`model_loader_queue_animations`]: the path
/// `Data\` (+ `Meshes\` unless the model path has it) + the model path, then
/// its last component replaced in turn by `MTIdle.KF`, when the actor passes
/// its virtual function `0x100` and has a current weapon whose type has a
/// holster prefix (tables at `0118a838` and `011977a4`) `<prefix>Holster.KF`
/// and `PA<prefix>Holster.KF`, and `Death.KF`; each is a `BuildFileList`
/// that is queued and deleted.
fn queue_skeleton_animations(
    e: &mut Engine,
    this: Ptr,
    key: u32,
    parent: Ptr,
    actor: Ptr,
    model_path: Ptr,
) {
    e.with_stack(PATH_BUFFER_SIZE, |e, full| {
        e.call(STRING_COPY, &args![full, PATH_BUFFER_SIZE, DATA_PREFIX]);
        if e.call(STRNICMP, &args![model_path, MESHES_PREFIX, 7u32])
            .u32()
            != 0
        {
            e.call(STRING_CONCAT, &args![full, PATH_BUFFER_SIZE, MESHES_PREFIX]);
        }
        e.call(STRING_CONCAT, &args![full, PATH_BUFFER_SIZE, model_path]);
        let file_part = e.call(STRRCHR, &args![full, BACKSLASH]).u32();
        let room = PATH_BUFFER_SIZE.wrapping_sub(file_part.wrapping_sub(full.addr()));
        e.call(STRING_COPY, &args![file_part, room, MTIDLE_FILE]);
        let list = model_loader_build_file_list(e, this, full.addr(), model_path.addr(), 0);
        queue_and_delete_list(e, this, list, key, parent);
        let armed = e.vcall(actor.addr(), 0x100, &args![]).bool();
        if armed && e.call(ACTOR_GET_CURRENT_WEAPON, &args![actor]).u32() != 0 {
            let weapon = e.call(ACTOR_GET_CURRENT_WEAPON, &args![actor]).ptr::<()>();
            let weapon_type = fn_00446390(e, weapon);
            let index = e
                .mem
                .u32(HOLSTER_INDEX_TABLE.wrapping_add((weapon_type as u32) << 2));
            let prefix = e.mem.u32(HOLSTER_PREFIX_TABLE.wrapping_add(index << 2));
            if prefix != 0 && e.mem.i8(prefix) != 0 {
                for format in [HOLSTER_FORMAT, POWER_ARMOR_HOLSTER_FORMAT] {
                    e.call(SPRINTF, &args![file_part, format, prefix]);
                    let list =
                        model_loader_build_file_list(e, this, full.addr(), model_path.addr(), 0);
                    queue_and_delete_list(e, this, list, key, parent);
                }
            }
        }
        e.call(STRING_COPY, &args![file_part, room, DEATH_FILE]);
        let list = model_loader_build_file_list(e, this, full.addr(), model_path.addr(), 0);
        queue_and_delete_list(e, this, list, key, parent);
    });
}

/// The idle part of [`model_loader_queue_animations`]: the model's directory
/// with `\Locomotion\Child\IdleAnims` (actor virtual function `0x1a0(1)`),
/// `\Locomotion\Female\IdleAnims` (`0087f4c0 == 1`) or
/// `\Locomotion\Male\IdleAnims` as the last component; the idle manager's
/// root file name list for it is copied into a new list
/// (`CopyFilenameList`, on the loader at `011c3b3c`), which is queued and
/// deleted.
fn queue_idle_animations(
    e: &mut Engine,
    this: Ptr,
    key: u32,
    parent: Ptr,
    actor: Ptr,
    model_path: Ptr,
) {
    e.with_stack(PATH_BUFFER_SIZE, |e, directory| {
        e.call(STRING_COPY, &args![directory, PATH_BUFFER_SIZE, model_path]);
        let separator = e.call(STRRCHR, &args![directory, BACKSLASH]).u32();
        let room = PATH_BUFFER_SIZE
            .wrapping_sub(separator.wrapping_sub(directory.addr()))
            .wrapping_sub(1);
        let name = if e.vcall(actor.addr(), 0x1a0, &args![1u32]).bool() {
            CHILD_IDLE_DIRECTORY
        } else if e.call(ACTOR_GET_SEX, &args![actor]).u32() == 1 {
            FEMALE_IDLE_DIRECTORY
        } else {
            MALE_IDLE_DIRECTORY
        };
        e.call(STRING_COPY, &args![separator, room, name]);
        let list = new_list(e);
        let idle_manager = e.global::<u32>(IDLE_MANAGER);
        let roots = e
            .call(
                GET_ROOT_FILENAME_LIST,
                &args![idle_manager, directory, 0u32],
            )
            .u32();
        let loader = e.global::<u32>(LOADER);
        e.call(MODEL_LOADER_COPY_FILENAME_LIST, &args![loader, roots, list]);
        queue_and_delete_list(e, this, list, key, parent);
    });
}

/// The last part of [`model_loader_queue_animations`]: a copy of every file
/// name of the cast's list, queued with `<model directory>\SpecialAnims\` as
/// prefix, then the list is deleted.
fn queue_special_animations(
    e: &mut Engine,
    this: Ptr,
    key: u32,
    parent: Ptr,
    model: Ptr,
    animation: u32,
) {
    let list = new_list(e);
    let mut node = e.call(LIST_HEAD_OF_OBJECT, &args![animation]).u32();
    while node != 0 {
        let item = node_item(e, node);
        let size = e.call(STRLEN, &args![item]).u32().wrapping_add(1);
        let copy = e.call(MEMORY_ALLOC, &args![size]).u32();
        e.with_stack(4, |e, holder| {
            e.mem.set_u32(holder.addr(), copy);
            e.call(STRING_COPY, &args![copy, size, item]);
            e.call(LIST_PUSH_BACK, &args![list, holder]);
        });
        node = e.call(LIST_NEXT_NODE, &args![node]).u32();
    }
    let model_path = e.vcall(model.addr(), 0x14, &args![]).u32();
    e.with_stack(PATH_BUFFER_SIZE, |e, directory| {
        e.call(STRING_COPY, &args![directory, PATH_BUFFER_SIZE, model_path]);
        let separator = e.call(STRRCHR, &args![directory, BACKSLASH]).u32();
        if separator != 0 {
            let room = PATH_BUFFER_SIZE.wrapping_sub(separator.wrapping_sub(directory.addr()));
            e.call(
                STRING_COPY,
                &args![separator, room, SPECIAL_ANIMS_DIRECTORY],
            );
        }
        fn_00446500(e, this, Ptr::new(list), key, parent, directory.addr());
    });
    if list != 0 {
        e.call(LIST_DELETE, &args![list, 1u32]);
    }
}

// Translated from 00446390 (decompiled, FalloutNV.exe 1.4.0.525)
/// The signed byte at +0xF4 of the object `Actor::GetCurrentWeapon` returns
/// (no name in the engine map): the weapon type `QueueAnimations` indexes
/// its holster table with.
pub fn fn_00446390(e: &mut Engine, this: Ptr) -> i32 {
    e.mem.i8(this.addr() + 0xf4) as i32
}

// Translated from 004463b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ModelLoader` method (no name in the engine map): queues every name of
/// the list `list`, in order. A name, with `prefix` put in front when it is
/// not null, goes to `QueueModel` (`00444040(name, key, parent, 0, 1, 0, 0)`)
/// when `entries` is null, otherwise to `004442c0` together with the
/// `index`th entry of `entries` ([`fn_004464c0`], 0 past the end). Empty
/// names are skipped but still count in `index`.
pub fn fn_004463b0(
    e: &mut Engine,
    this: Ptr,
    list: Ptr,
    entries: Ptr,
    key: u32,
    parent: Ptr,
    prefix: u32,
) {
    let mut node = list.addr();
    let mut index = 0u32;
    while node != 0 {
        let item = node_item(e, node);
        if item != 0 {
            with_prefixed_name(e, prefix, item, |e, name| {
                if entries.is_null() {
                    e.call(
                        MODEL_LOADER_QUEUE_MODEL,
                        &args![this, name, key, parent, 0u32, 1u32, 0u32, 0u32],
                    );
                } else {
                    let entry = fn_004464c0(e, entries, index);
                    e.call(
                        MODEL_LOADER_QUEUE_MODEL_WITH_ENTRY,
                        &args![this, name, entry, key, parent, 0u32, 1u32, 0u32, 0u32],
                    );
                }
            });
        }
        index += 1;
        node = e.call(LIST_NEXT_NODE, &args![node]).u32();
    }
}

// Translated from 004464c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `index`th entry of an array (count at +0, entry pointer at +4), or 0
/// when `index` is not below the count.
pub fn fn_004464c0(e: &mut Engine, this: Ptr, index: u32) -> u32 {
    if index < e.mem.u32(this.addr()) {
        let entries = e.mem.u32(this.addr() + 4);
        e.mem.u32(entries + index * 4)
    } else {
        0
    }
}

// Translated from 00446500 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ModelLoader` method (no name in the engine map): drains the list `list`
/// of file names. Each non-null name (with `prefix` put in front when it is
/// not null) goes to `004446a0(name, key, parent)`; the item is freed and the
/// head removed. An empty head moves on to the next node.
pub fn fn_00446500(e: &mut Engine, this: Ptr, list: Ptr, key: u32, parent: Ptr, prefix: u32) {
    let mut node = list.addr();
    while node != 0 {
        let item = node_item(e, node);
        if item == 0 {
            node = e.call(LIST_NEXT_NODE, &args![node]).u32();
            continue;
        }
        with_prefixed_name(e, prefix, item, |e, name| {
            e.call(MODEL_LOADER_QUEUE_KF, &args![this, name, key, parent]);
        });
        e.call(MEMORY_FREE, &args![item]);
        e.call(LIST_REMOVE_HEAD, &args![node]);
    }
}

// Translated from 004465f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ModelLoader` method (no name in the engine map): queues the replacement
/// KF files of `animation`. Nothing happens for a null list or animation, or
/// an empty first item. In a memory context (`0x33`, source line `0xe9f`)
/// each file name of the list (prefixed as in [`fn_00446500`]) is looked up
/// in the map of loaded KF models (`this + 4`): if found, the animation takes
/// the model (`00490fa0`). If not, the `QueuedReplacementKFList` of the
/// animation is taken from the map at `this + 0x14` or created
/// ([`fn_004469c0`]), once, and a `QueuedReplacementKF` for the name is
/// created, made a child of the list and queued (virtual function `0x20`).
/// The item is freed and the head removed. Afterwards a list without
/// children is dropped; one with children gets `parent` as parent, goes into
/// the map (virtual function `0x10` of `this + 0x14`, with 1), is put in
/// state 5 and has `CheckFinished` (virtual function `0x28`) called.
pub fn fn_004465f0(
    e: &mut Engine,
    this: Ptr,
    list: Ptr,
    animation: Ptr,
    key: u32,
    parent: Ptr,
    prefix: u32,
) {
    if list.is_null() || animation.is_null() {
        return;
    }
    if node_item(e, list.addr()) == 0 {
        return;
    }
    e.with_stack(4, |e, scope| {
        e.call(
            MEMORY_CONTEXT_ENTER,
            &args![
                scope,
                REPLACEMENT_CONTEXT,
                1u32,
                MODEL_LOADER_SOURCE,
                REPLACEMENT_SOURCE_LINE
            ],
        );
        e.with_stack(4, |e, queued_list| {
            e.call(TASK_POINTER_CONSTRUCT, &args![queued_list, 0u32]);
            let kf_map = e.mem.u32(this.addr() + 4);
            let list_map = e.mem.u32(this.addr() + 0x14);
            let mut node = list.addr();
            while node != 0 {
                let item = node_item(e, node);
                if item == 0 {
                    node = e.call(LIST_NEXT_NODE, &args![node]).u32();
                    continue;
                }
                with_prefixed_name(e, prefix, item, |e, name| {
                    let (found, model) = e.with_stack(4, |e, out| {
                        let hit = e.vcall(kf_map, 8, &args![name, out]).bool();
                        (hit, e.mem.u32(out.addr()))
                    });
                    if found {
                        e.call(ANIMATION_KF_LOADED, &args![animation, model]);
                        return;
                    }
                    if e.call(POINTER_GET, &args![queued_list]).u32() == 0
                        && !e.vcall(list_map, 8, &args![animation, queued_list]).bool()
                    {
                        let block = e
                            .call(MEMORY_ALLOC, &args![QUEUED_REPLACEMENT_KF_LIST_SIZE])
                            .u32();
                        let created = if block != 0 {
                            fn_004469c0(e, Ptr::new(block), animation, key).addr()
                        } else {
                            0
                        };
                        e.call(QUEUED_FILE_POINTER_ASSIGN, &args![queued_list, created]);
                    }
                    let block = e
                        .call(MEMORY_ALLOC, &args![QUEUED_REPLACEMENT_KF_SIZE])
                        .u32();
                    let kf = if block != 0 {
                        let owner = e.call(POINTER_GET, &args![queued_list]).u32();
                        e.call(
                            QUEUED_REPLACEMENT_KF_CONSTRUCT,
                            &args![block, name, key, animation, owner],
                        )
                        .u32()
                    } else {
                        0
                    };
                    e.with_stack(4, |e, kf_pointer| {
                        e.call(TASK_POINTER_CONSTRUCT, &args![kf_pointer, kf]);
                        let owner = e.call(POINTER_GET, &args![queued_list]).u32();
                        let child = e.call(POINTER_GET, &args![kf_pointer]).u32();
                        e.call(QUEUED_FILE_SET_PARENT, &args![child, owner]);
                        let child = e.call(POINTER_GET, &args![kf_pointer]).u32();
                        e.vcall(child, 0x20, &args![]);
                        e.call(TASK_POINTER_DESTRUCT, &args![kf_pointer]);
                    });
                });
                e.call(MEMORY_FREE, &args![item]);
                e.call(LIST_REMOVE_HEAD, &args![node]);
            }
            let queued = e.call(POINTER_GET, &args![queued_list]).u32();
            if queued == 0 || fn_00446990(e, Ptr::new(queued)) == 0 {
                e.call(QUEUED_FILE_POINTER_ASSIGN, &args![queued_list, 0u32]);
            } else {
                let queued = e.call(POINTER_GET, &args![queued_list]).u32();
                e.call(QUEUED_FILE_SET_PARENT, &args![queued, parent]);
                e.vcall(list_map, 0x10, &args![animation, queued_list, 1u32]);
                let queued = e.call(POINTER_GET, &args![queued_list]).u32();
                e.call(TASK_SET_DONE, &args![queued]);
                let queued = e.call(POINTER_GET, &args![queued_list]).u32();
                e.vcall(queued, 0x28, &args![]);
            }
            e.call(TASK_POINTER_DESTRUCT, &args![queued_list]);
        });
        e.call(MEMORY_CONTEXT_LEAVE, &args![scope]);
    });
}

// Translated from 00446990 (decompiled, FalloutNV.exe 1.4.0.525)
/// The dword at +8 of the queued file's `pChildren` (the number of
/// children), 0 when there is no `pChildren`.
pub fn fn_00446990(e: &mut Engine, this: Ptr) -> u32 {
    let children = e.mem.u32(this.addr() + QueuedFile::pChildren.off);
    if children == 0 {
        0
    } else {
        e.call(FIELD_AT_8, &args![children]).u32()
    }
}

// Translated from 004469c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedReplacementKFList` constructor (no name in the engine map): the
/// `QueuedFile` constructor with `context`, the list's virtual table, `pAnim`
/// set to `animation` and both child counters 0. Returns `this`.
pub fn fn_004469c0(
    e: &mut Engine,
    this: Ptr<QueuedReplacementKFList>,
    animation: Ptr,
    context: u32,
) -> Ptr<QueuedReplacementKFList> {
    e.call(QUEUED_FILE_CONSTRUCT, &args![this, context]);
    e.mem
        .set_u32(this.addr(), QUEUED_REPLACEMENT_KF_LIST_VTABLE);
    e.set(this, QueuedReplacementKFList::pAnim, animation);
    e.set(this, QueuedReplacementKFList::iPostProcessingChildCount, 0);
    e.set(this, QueuedReplacementKFList::iPostProcessedChildCount, 0);
    this
}

// Translated from 00446a10 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedReplacementKFList::_scalar_deleting_destructor_` (Xbox PDB): runs
/// the destructor body (`0043dc50`) and, when bit 0 of `flags` is set, frees
/// the object. Returns `this`.
pub fn queued_replacement_kf_list_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr<QueuedReplacementKFList>,
    flags: u32,
) -> Ptr<QueuedReplacementKFList> {
    e.call(QUEUED_FILE_DESTRUCT, &args![this]);
    if flags & 1 != 0 {
        e.call(MEMORY_FREE, &args![this]);
    }
    this
}

// Translated from 00446a40 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedReplacementKFList::CheckFinished` (Xbox PDB):
/// `QueuedFile::CheckFinished`.
pub fn queued_replacement_kf_list_check_finished(
    e: &mut Engine,
    this: Ptr<QueuedReplacementKFList>,
) {
    e.call(QUEUED_FILE_CHECK_FINISHED, &args![this]);
}

// Translated from 00446a60 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ModelLoader` method (no name in the engine map; `this` is not used):
/// the model of `form` for `reference`. When the reference's form
/// (`007af430`) is `form`, it is the reference's `GetTESModel`. Otherwise:
/// for a form of type `0x28` it is `00522d80(form)`; else a form that casts
/// to the model class is itself the result; else a form that casts to the
/// biped class gives `GetWorldTESModel_ov2` with the sex of the
/// reference's owner when the owner (`GetOwner`) has type `0x2a` (else 0).
/// Anything else gives 0.
pub fn fn_00446a60(e: &mut Engine, _this: Ptr, form: Ptr, reference: Ptr) -> Ptr {
    if !reference.is_null() && e.call(REFERENCE_GET_FORM, &args![reference]).ptr::<()>() == form {
        return e.call(REFERENCE_GET_TES_MODEL, &args![reference]).ptr();
    }
    if !form.is_null() && e.call(FORM_TYPE, &args![form]).u32() == WEAPON_FORM_TYPE {
        return e.call(WEAPON_GET_MODEL, &args![form]).ptr();
    }
    let as_model = e
        .call(
            RT_DYNAMIC_CAST,
            &args![form, 0u32, FORM_SOURCE_TYPE, FORM_MODEL_TYPE, 0u32],
        )
        .ptr::<()>();
    if !as_model.is_null() {
        return as_model;
    }
    let as_biped = e
        .call(
            RT_DYNAMIC_CAST,
            &args![form, 0u32, FORM_SOURCE_TYPE, FORM_BIPED_TYPE, 0u32],
        )
        .u32();
    if as_biped == 0 {
        return Ptr::NULL;
    }
    let mut sex = 0;
    if !reference.is_null() {
        let owner = e.call(REFERENCE_GET_OWNER, &args![reference]).u32();
        if owner != 0 && e.call(FORM_TYPE, &args![owner]).u32() == ACTOR_BASE_FORM_TYPE {
            sex = e.call(ACTOR_BASE_GET_SEX, &args![owner]).u32();
        }
    }
    e.call(BIPED_GET_WORLD_MODEL, &args![as_biped, sex]).ptr()
}

// Translated from 00446b50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ModelLoader` method (no name in the engine map): walks the map of
/// queued references (`this + 8`) with an iterator. For each entry whose
/// reference has a cell (`008d6f30`) the wanted priority is 0 when that cell
/// is `cell` and the reference is `Is3DCritical`, otherwise
/// `TES::GetCellPriority(cell, 0)`; when it differs from the priority byte of
/// the queued task's key (`0043cc60`), the task's virtual function `0x1c` is
/// called with the new priority.
pub fn fn_00446b50(e: &mut Engine, this: Ptr, cell: Ptr) {
    let map = e.mem.u32(this.addr() + 8);
    e.with_stack(0x10, |e, iterator| {
        e.call(MAP_ITERATOR_CONSTRUCT, &args![iterator]);
        while !e.call(MAP_ITERATOR_IS_DONE, &args![iterator]).bool() {
            e.with_stack(8, |e, slots| {
                let key = slots;
                let value = slots.byte_add(4);
                e.call(TASK_POINTER_CONSTRUCT, &args![value, 0u32]);
                let more = e
                    .call(MAP_NEXT_ENTRY, &args![map, iterator, key, value, 1u32])
                    .bool();
                if more {
                    let reference = e.mem.u32(key.addr());
                    let entry_cell = e.call(REFERENCE_CELL, &args![reference]).u32();
                    if entry_cell != 0 {
                        let task = e.call(POINTER_GET, &args![value]).u32();
                        let current = e.call(TASK_PRIORITY, &args![task]).u32();
                        let critical = entry_cell == cell.addr()
                            && e.call(REFERENCE_IS_3D_CRITICAL, &args![reference]).bool();
                        let wanted = if critical {
                            0
                        } else {
                            let tes = e.global::<u32>(TES_GLOBAL);
                            e.call(GET_CELL_PRIORITY, &args![tes, entry_cell, 0u32])
                                .u32()
                        };
                        if current != wanted {
                            let task = e.call(POINTER_GET, &args![value]).u32();
                            e.vcall(task, 0x1c, &args![wanted]);
                        }
                    }
                }
                e.call(TASK_POINTER_DESTRUCT, &args![value]);
            });
        }
        e.call(MAP_ITERATOR_DESTRUCT, &args![iterator]);
    });
}

// Translated from 00446c90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Virtual function `0x44` of the map of queued references (`this + 8`),
/// whose result `00446cb0` prints as the number of references.
pub fn fn_00446c90(e: &mut Engine, this: Ptr) -> u32 {
    let map = e.mem.u32(this.addr() + 8);
    e.vcall(map, 0x44, &args![]).u32()
}

// Translated from 00446cb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ModelLoader` method (no name in the engine map): the loading status.
/// Unless the setting `00446e10` is set, it counts the queued references
/// ([`fn_00446c90`]), the background cloning work ([`fn_00446dc0`]), the
/// tasks of the task manager (`004491f0`) and its post-processing tasks
/// ([`fn_00446da0`]). When any is non-zero the text `Loading %d
/// references( %d background cloning ) and %d tasks( %d post processing )`
/// is formatted into a local buffer (the game never uses the text) and the
/// flag `011c3bfc` is set. When all are zero and the flag is set it is
/// cleared and the text `011a2fe8` is set on the window at +8 of the object
/// at `011dea0c` (`SetWindowTextA`).
pub fn fn_00446cb0(e: &mut Engine, this: Ptr) {
    if fn_00446e10(e) != 0 {
        return;
    }
    let references = fn_00446c90(e, this);
    let cloning = fn_00446dc0(e, this);
    let manager = e.global::<u32>(TASK_MANAGER);
    let tasks = e.call(TASK_MANAGER_TASK_COUNT, &args![manager]).u32();
    let post_processing = fn_00446da0(e, Ptr::new(manager));
    if tasks != 0 || cloning != 0 || references != 0 || post_processing != 0 {
        e.with_stack(PATH_BUFFER_SIZE, |e, buffer| {
            e.call(
                FORMAT_BUFFER,
                &args![
                    buffer,
                    PATH_BUFFER_SIZE,
                    LOADING_STATUS_FORMAT,
                    references,
                    cloning,
                    tasks,
                    post_processing
                ],
            );
        });
        e.mem.set_u8(STATUS_SHOWN_FLAG, 1);
    } else if e.mem.u8(STATUS_SHOWN_FLAG) != 0 {
        e.mem.set_u8(STATUS_SHOWN_FLAG, 0);
        let text = e.global::<u32>(WINDOW_TEXT);
        let owner = e.global::<u32>(WINDOW_OWNER);
        let window = e.call(FIELD_AT_8, &args![owner]).u32();
        e.call(SET_WINDOW_TEXT, &args![window, text]);
    }
}

// Translated from 00446da0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Virtual function `0xc` of the object at +0x64 of the task manager: the
/// number of post-processing tasks.
pub fn fn_00446da0(e: &mut Engine, this: Ptr) -> u32 {
    let queues = e.mem.u32(this.addr() + 0x64);
    e.vcall(queues, 0xc, &args![]).u32()
}

// Translated from 00446dc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The background clone thread's work count ([`fn_00446de0`] of
/// `pBackgroundCloneThread`, at `this + 0x28`).
pub fn fn_00446dc0(e: &mut Engine, this: Ptr) -> u32 {
    let thread = e.mem.u32(this.addr() + 0x28);
    fn_00446de0(e, Ptr::new(thread))
}

// Translated from 00446de0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BackgroundCloneThread` method (no name in the engine map): virtual
/// function `0x10` of `pCloneReferencesQueue` (+0x38) plus `iRunningCount`
/// (+0x34).
pub fn fn_00446de0(e: &mut Engine, this: Ptr) -> u32 {
    let queue = e.mem.u32(this.addr() + 0x38);
    let queued = e.vcall(queue, 0x10, &args![]).u32();
    queued.wrapping_add(e.mem.u32(this.addr() + 0x34))
}

// Translated from 00446e10 (decompiled, FalloutNV.exe 1.4.0.525)
/// The byte value of the setting object at `011c77b4` (0 when there is no
/// object, from the scratch byte `00408d60` returns).
pub fn fn_00446e10(e: &mut Engine) -> u8 {
    let setting = e.global::<u32>(LOADING_SETTING);
    let value = e.call(SETTING_VALUE_POINTER, &args![setting]).u32();
    e.mem.u8(value)
}

// Translated from 00446e30 (decompiled, FalloutNV.exe 1.4.0.525)
/// [`fn_00446cb0`] on the loader at `011c3b3c`.
pub fn fn_00446e30(e: &mut Engine) {
    let loader = e.global::<u32>(LOADER);
    fn_00446cb0(e, Ptr::new(loader));
}

// Translated from 00446e40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether both `00448c10` and `00448cc0` (called on the loader at
/// `011c3b3c`, both always) answer true.
pub fn fn_00446e40(e: &mut Engine) -> bool {
    let loader = e.global::<u32>(LOADER);
    let first = e.call(LOADER_CHECK_FIRST, &args![loader]).bool();
    let second = e.call(LOADER_CHECK_SECOND, &args![loader]).bool();
    first && second
}

// Translated from 00446ea0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Refreshes the screen state: `00878080(0x11deffc, 0)`, `0082fb70()`, then,
/// when `0047c850` of the object at `011de45c` is true (the game tests it
/// twice), `00861ea0` on it.
pub fn fn_00446ea0(e: &mut Engine) {
    let object = fn_00446ef0(e);
    e.call(REFRESH_FIRST, &args![object, 0u32]);
    e.call(REFRESH_SECOND, &args![]);
    let main_loop = e.global::<u32>(MAIN_LOOP_OBJECT);
    if e.call(MAIN_LOOP_TEST, &args![main_loop]).bool() {
        let main_loop = e.global::<u32>(MAIN_LOOP_OBJECT);
        if e.call(MAIN_LOOP_TEST, &args![main_loop]).bool() {
            let main_loop = e.global::<u32>(MAIN_LOOP_OBJECT);
            e.call(MAIN_LOOP_UPDATE, &args![main_loop]);
        }
    }
}

// Translated from 00446ef0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address `011deffc`.
pub fn fn_00446ef0(_e: &mut Engine) -> u32 {
    REFRESH_OBJECT
}

// Translated from 00446f00 (decompiled, FalloutNV.exe 1.4.0.525)
/// [`fn_00446dc0`] on the loader at `011c3b3c`.
pub fn fn_00446f00(e: &mut Engine) -> u32 {
    let loader = e.global::<u32>(LOADER);
    fn_00446dc0(e, Ptr::new(loader))
}

// Translated from 00446f10 (decompiled, FalloutNV.exe 1.4.0.525)
/// [`fn_00446f20`] on the loader at `011c3b3c`.
pub fn fn_00446f10(e: &mut Engine) {
    let loader = e.global::<u32>(LOADER);
    fn_00446f20(e, Ptr::new(loader));
}

// Translated from 00446f20 (decompiled, FalloutNV.exe 1.4.0.525)
/// [`fn_00446f40`] on `pBackgroundCloneThread` (`this + 0x28`).
pub fn fn_00446f20(e: &mut Engine, this: Ptr) {
    let thread = e.mem.u32(this.addr() + 0x28);
    fn_00446f40(e, Ptr::new(thread));
}

// Translated from 00446f40 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BackgroundCloneThread` method (no name in the engine map): when the
/// current thread id differs from the thread's `iThreadID` (+8), runs
/// [`fn_00446f70`].
pub fn fn_00446f40(e: &mut Engine, this: Ptr) {
    let thread_id = e.call(FIELD_AT_8, &args![this]).u32();
    if thread_id != e.call(CURRENT_THREAD_ID, &args![]).u32() {
        fn_00446f70(e, this);
    }
}

// Translated from 00446f70 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BackgroundCloneThread` method (no name in the engine map): `00442600`
/// then `00442510` on the thread.
pub fn fn_00446f70(e: &mut Engine, this: Ptr) {
    e.call(CLONE_THREAD_CALL_FIRST, &args![this]);
    e.call(CLONE_THREAD_CALL_SECOND, &args![this]);
}

// Translated from 00446f90 (decompiled, FalloutNV.exe 1.4.0.525)
/// [`fn_00446fa0`] on the loader at `011c3b3c`.
pub fn fn_00446f90(e: &mut Engine) {
    let loader = e.global::<u32>(LOADER);
    fn_00446fa0(e, Ptr::new(loader));
}

// Translated from 00446fa0 (decompiled, FalloutNV.exe 1.4.0.525)
/// [`fn_00446fc0`] on `pBackgroundCloneThread` (`this + 0x28`).
pub fn fn_00446fa0(e: &mut Engine, this: Ptr) {
    let thread = e.mem.u32(this.addr() + 0x28);
    fn_00446fc0(e, Ptr::new(thread));
}

// Translated from 00446fc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BackgroundCloneThread` method (no name in the engine map): when the
/// current thread id differs from the thread's `iThreadID` (+8), runs
/// [`fn_00446ff0`].
pub fn fn_00446fc0(e: &mut Engine, this: Ptr) {
    let thread_id = e.call(FIELD_AT_8, &args![this]).u32();
    if thread_id != e.call(CURRENT_THREAD_ID, &args![]).u32() {
        fn_00446ff0(e, this);
    }
}

// Translated from 00446ff0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BackgroundCloneThread` method (no name in the engine map): `00442600`
/// then `00442530` on the thread.
pub fn fn_00446ff0(e: &mut Engine, this: Ptr) {
    e.call(CLONE_THREAD_CALL_FIRST, &args![this]);
    e.call(CLONE_THREAD_CALL_THIRD, &args![this]);
}

// Translated from 00447010 (decompiled, FalloutNV.exe 1.4.0.525)
/// Adds a model to the loader's map of models (no name in the engine map):
/// virtual function `0x10` of the map (`this + 0`) with the name, the
/// address of the model word and 0. False when the name is already there.
pub fn fn_00447010(e: &mut Engine, this: Ptr, name: u32, model: Ptr) -> bool {
    let map = e.mem.u32(this.addr());
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), model.addr());
        e.vcall(map, 0x10, &args![name, slot, 0u32]).bool()
    })
}

// Translated from 00447040 (decompiled, FalloutNV.exe 1.4.0.525)
/// Adds a `KFModel` to the loader's map of KF models (no name in the engine
/// map): virtual function `0x10` of the map at `this + 4` with the name, the
/// address of the model word and 0. False when the name is already there.
pub fn fn_00447040(e: &mut Engine, this: Ptr, name: u32, kf_model: Ptr) -> bool {
    let map = e.mem.u32(this.addr() + 4);
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), kf_model.addr());
        e.vcall(map, 0x10, &args![name, slot, 0u32]).bool()
    })
}

// Translated from 00447080 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ModelLoader::LoadFile` (Xbox PDB): the 3D object of the model of file
/// `name`. The map of models (`this + 0`) is asked first (virtual function
/// 8); a model found with a zero reference total (`00443190`) is logged.
/// Otherwise a `QueuedModel` on the stack is built for the name
/// (`0043c6e0(name, 0, lod_fade_mult, first_flag, 0)`), `override_flag` is
/// given to `00443ff0`, bit 0x10 of `cFlags` is set, and the task is run and
/// finished in place; its `spModel` is the model. A model gets one more
/// reference (`0092c870`) unless `no_reference` is set, and the result is its
/// 3D object (`0043b230`); 0 when there is no model. The word after
/// `first_flag` is not read.
#[allow(clippy::too_many_arguments)]
pub fn model_loader_load_file(
    e: &mut Engine,
    this: Ptr,
    name: u32,
    lod_fade_mult: u32,
    first_flag: u8,
    _unused_4: u32,
    override_flag: u8,
    no_reference: u8,
) -> Ptr {
    let map = e.mem.u32(this.addr());
    let (found, mut model) = e.with_stack(4, |e, out| {
        let hit = e.vcall(map, 8, &args![name, out]).bool();
        (hit, e.mem.u32(out.addr()))
    });
    if !found {
        model = e.with_stack(<QueuedModel as Layout>::SIZE, |e, task| {
            e.call(
                QUEUED_MODEL_CONSTRUCT,
                &args![task, name, 0u32, lod_fade_mult, first_flag, 0u8],
            );
            e.call(QUEUED_MODEL_SET_FLAG_20, &args![task, override_flag]);
            fn_00447190(e, task.cast(), 1);
            e.call(QUEUED_MODEL_RUN, &args![task]);
            e.call(QUEUED_MODEL_FINISH, &args![task]);
            let loaded = e
                .call(POINTER_GET, &args![task.byte_add(QueuedModel::spModel.off)])
                .u32();
            e.call(QUEUED_MODEL_DESTRUCT, &args![task]);
            loaded
        });
    } else if e.call(MODEL_REFERENCE_TOTAL, &args![model]).u32() == 0 {
        e.call(LOG, &args![LOAD_FILE_ZERO_REFERENCE_MESSAGE, name]);
    }
    if model == 0 {
        return Ptr::NULL;
    }
    if no_reference == 0 {
        e.call(ADD_REFERENCE, &args![model]);
    }
    e.call(MODEL_OBJECT_3D, &args![model]).ptr()
}

// Translated from 00447190 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets (`flag` non-zero) or clears bit 0x10 of the queued model's `cFlags`.
pub fn fn_00447190(e: &mut Engine, this: Ptr<QueuedModel>, flag: u8) {
    e.call(
        SET_FLAG_BIT_10,
        &args![flag, this.byte_add(QueuedModel::cFlags.off)],
    );
}

// Translated from 004471c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ModelLoader::LoadKF` (Xbox PDB): the `KFModel` of file `name`. The map of
/// KF models (`this + 4`) is asked first (virtual function 8); one found with
/// a zero reference total (`004431b0`) is logged. Otherwise a `QueuedKF` on
/// the stack (`0043e0f0(name, 0)`) is run and finished in place and its
/// `spKFModel` is the model, and the stack object is destroyed. A model
/// gets one more manual reference (`0043be10`). Returns it (0 when none).
pub fn model_loader_load_kf(e: &mut Engine, this: Ptr, name: u32) -> Ptr {
    let map = e.mem.u32(this.addr() + 4);
    let (found, mut model) = e.with_stack(4, |e, out| {
        let hit = e.vcall(map, 8, &args![name, out]).bool();
        (hit, e.mem.u32(out.addr()))
    });
    if !found {
        model = e.with_stack(QUEUED_KF_SIZE, |e, task| {
            e.call(QUEUED_KF_CONSTRUCT, &args![task, name, 0u32]);
            e.call(QUEUED_KF_RUN, &args![task]);
            e.call(QUEUED_KF_FINISH, &args![task]);
            let loaded = e
                .call(POINTER_GET, &args![task.byte_add(QueuedKF::spKFModel.off)])
                .u32();
            e.call(QUEUED_KF_DESTRUCT, &args![task]);
            loaded
        });
    } else if e.call(KF_MODEL_REFERENCE_TOTAL, &args![model]).u32() == 0 {
        e.call(LOG, &args![LOAD_KF_ZERO_REFERENCE_MESSAGE, name]);
    }
    if model != 0 {
        e.call(KF_MODEL_ADD_MANUAL_REFERENCE, &args![model]);
    }
    Ptr::new(model)
}

// Translated from 004472a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ModelLoader::FindModel` (Xbox PDB): clears the `NiPointer<Model>` `out`,
/// asks the map of models (`this + 0`, virtual function 8) for `name` and,
/// when found, stores the model into `out`. Returns whether it was found.
pub fn model_loader_find_model(e: &mut Engine, this: Ptr, name: u32, out: Ptr) -> bool {
    e.call(MODEL_POINTER_ASSIGN, &args![out, 0u32]);
    let map = e.mem.u32(this.addr());
    let (found, model) = e.with_stack(4, |e, slot| {
        let hit = e.vcall(map, 8, &args![name, slot]).bool();
        (hit, e.mem.u32(slot.addr()))
    });
    if found {
        e.call(MODEL_POINTER_ASSIGN, &args![out, model]);
    }
    found
}

// Translated from 00447300 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ModelLoader::BuildFileList` (Xbox PDB; `this` is not used): the file
/// list `00afe420(path, name, 1, flag)` builds (cdecl). Returns it.
pub fn model_loader_build_file_list(
    e: &mut Engine,
    _this: Ptr,
    path: u32,
    name: u32,
    flag: u32,
) -> u32 {
    e.call(BUILD_FILE_LIST, &args![path, name, 1u32, flag])
        .u32()
}

/// This part's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(
            0x004457d0,
            model_loader_queue_creature_parts(Ptr, Ptr, u32, Ptr, Ptr)
        ),
        entry!(0x004459e0, fn_004459e0(Ptr, u32) -> Ptr),
        entry!(
            0x00445a10,
            model_loader_queue_animations(Ptr, Ptr, u32, Ptr, Ptr, u8, u8)
        ),
        entry!(0x00446390, fn_00446390(Ptr) -> i32),
        entry!(0x004463b0, fn_004463b0(Ptr, Ptr, Ptr, u32, Ptr, u32)),
        entry!(0x004464c0, fn_004464c0(Ptr, u32) -> u32),
        entry!(0x00446500, fn_00446500(Ptr, Ptr, u32, Ptr, u32)),
        entry!(0x004465f0, fn_004465f0(Ptr, Ptr, Ptr, u32, Ptr, u32)),
        entry!(0x00446990, fn_00446990(Ptr) -> u32),
        entry!(
            0x004469c0,
            fn_004469c0(Ptr<QueuedReplacementKFList>, Ptr, u32) -> Ptr<QueuedReplacementKFList>
        ),
        entry!(
            0x00446a10,
            queued_replacement_kf_list_scalar_deleting_destructor(
                Ptr<QueuedReplacementKFList>,
                u32,
            )
                -> Ptr<QueuedReplacementKFList>
        ),
        entry!(
            0x00446a40,
            queued_replacement_kf_list_check_finished(Ptr<QueuedReplacementKFList>)
        ),
        entry!(0x00446a60, fn_00446a60(Ptr, Ptr, Ptr) -> Ptr),
        entry!(0x00446b50, fn_00446b50(Ptr, Ptr)),
        entry!(0x00446c90, fn_00446c90(Ptr) -> u32),
        entry!(0x00446cb0, fn_00446cb0(Ptr)),
        entry!(0x00446da0, fn_00446da0(Ptr) -> u32),
        entry!(0x00446dc0, fn_00446dc0(Ptr) -> u32),
        entry!(0x00446de0, fn_00446de0(Ptr) -> u32),
        entry!(0x00446e10, fn_00446e10() -> u8),
        entry!(0x00446e30, fn_00446e30()),
        entry!(0x00446e40, fn_00446e40() -> bool),
        entry!(0x00446ea0, fn_00446ea0()),
        entry!(0x00446ef0, fn_00446ef0() -> u32),
        entry!(0x00446f00, fn_00446f00() -> u32),
        entry!(0x00446f10, fn_00446f10()),
        entry!(0x00446f20, fn_00446f20(Ptr)),
        entry!(0x00446f40, fn_00446f40(Ptr)),
        entry!(0x00446f70, fn_00446f70(Ptr)),
        entry!(0x00446f90, fn_00446f90()),
        entry!(0x00446fa0, fn_00446fa0(Ptr)),
        entry!(0x00446fc0, fn_00446fc0(Ptr)),
        entry!(0x00446ff0, fn_00446ff0(Ptr)),
        entry!(0x00447010, fn_00447010(Ptr, u32, Ptr) -> bool),
        entry!(0x00447040, fn_00447040(Ptr, u32, Ptr) -> bool),
        entry!(
            0x00447080,
            model_loader_load_file(Ptr, u32, u32, u8, u32, u8, u8) -> Ptr
        ),
        entry!(0x00447190, fn_00447190(Ptr<QueuedModel>, u8)),
        entry!(0x004471c0, model_loader_load_kf(Ptr, u32) -> Ptr),
        entry!(0x004472a0, model_loader_find_model(Ptr, u32, Ptr) -> bool),
        entry!(
            0x00447300,
            model_loader_build_file_list(Ptr, u32, u32, u32) -> u32
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    /// Addresses of test doubles that stand in for virtual functions.
    const SLOT_A: u32 = 0x7000_0001;
    const SLOT_B: u32 = 0x7000_0002;
    const SLOT_C: u32 = 0x7000_0003;

    /// An engine with the pages the functions read, and doubles for the
    /// allocator, the string functions, the list helpers and the smart
    /// pointer helpers.
    fn engine() -> Engine {
        let mut e = Engine::new();
        for page in [
            0x0101_3000,
            0x0101_6000,
            0x0101_7000,
            0x0118_a000,
            0x0119_7000,
            0x011a_2000,
            0x011c_3000,
            0x011c_7000,
            0x011c_b000,
            0x011d_e000,
            0x0120_2000,
        ] {
            e.map(page, 0x1000);
        }
        e.register(MEMORY_ALLOC, |e, a| e.mem.alloc(a[0]).into_ret());
        e.register(MEMORY_FREE, |e, a| {
            e.mem.free(a[0]);
            Ret::default()
        });
        e.register(STRING_COPY, |e, a| {
            let text = e.mem.cstr(a[2]);
            e.mem.set_cstr(a[0], &text);
            Ret::default()
        });
        e.register(STRING_CONCAT, |e, a| {
            let mut text = e.mem.cstr(a[0]);
            text.extend(e.mem.cstr(a[2]));
            e.mem.set_cstr(a[0], &text);
            Ret::default()
        });
        e.register(STRRCHR, |e, a| {
            let text = e.mem.cstr(a[0]);
            match text.iter().rposition(|c| *c as u32 == a[1]) {
                Some(i) => (a[0] + i as u32).into_ret(),
                None => 0u32.into_ret(),
            }
        });
        e.register(STRNICMP, |e, a| {
            let lower = |text: Vec<u8>| -> Vec<u8> {
                text.iter()
                    .take(a[2] as usize)
                    .map(|c| c.to_ascii_lowercase())
                    .collect()
            };
            u32::from(lower(e.mem.cstr(a[0])) != lower(e.mem.cstr(a[1]))).into_ret()
        });
        e.register(STRLEN, |e, a| (e.mem.cstr(a[0]).len() as u32).into_ret());
        e.register(SPRINTF, |e, a| {
            let format = String::from_utf8(e.mem.cstr(a[1])).unwrap();
            let argument = String::from_utf8(e.mem.cstr(a[2])).unwrap();
            let text = format.replacen("%s", &argument, 1);
            e.mem.set_cstr(a[0], text.as_bytes());
            Ret::default()
        });
        e.register(LIST_NODE_ITEM_ADDRESS, |_, a| a[0].into_ret());
        e.register(LIST_NEXT_NODE, |e, a| e.mem.u32(a[0] + 4).into_ret());
        e.register(LIST_REMOVE_HEAD, |e, a| {
            let next = e.mem.u32(a[0] + 4);
            if next != 0 {
                let (item, after) = (e.mem.u32(next), e.mem.u32(next + 4));
                e.mem.set_u32(a[0], item);
                e.mem.set_u32(a[0] + 4, after);
            } else {
                e.mem.set_u32(a[0], 0);
            }
            Ret::default()
        });
        e.register(LIST_DELETE, |_, _| Ret::default());
        e.register(FIELD_AT_8, |e, a| e.mem.u32(a[0] + 8).into_ret());
        e.register(POINTER_GET, |e, a| e.mem.u32(a[0]).into_ret());
        e.register(TASK_POINTER_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            a[0].into_ret()
        });
        e.register(TASK_POINTER_DESTRUCT, |_, _| Ret::default());
        e.register(QUEUED_FILE_POINTER_ASSIGN, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            a[0].into_ret()
        });
        e.register(MODEL_POINTER_ASSIGN, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            a[0].into_ret()
        });
        e.register(MEMORY_CONTEXT_ENTER, |_, _| Ret::default());
        e.register(MEMORY_CONTEXT_LEAVE, |_, _| Ret::default());
        e.register(RT_DYNAMIC_CAST, |_, _| 0u32.into_ret());
        e.register(LIST_CONSTRUCT, |_, a| a[0].into_ret());
        for (address, value) in [
            (SKELETON_WORD, "Skeleton"),
            (DATA_PREFIX, "Data\\"),
            (MESHES_PREFIX, "Meshes\\"),
            (MTIDLE_FILE, "\\MTIdle.KF"),
            (HOLSTER_FORMAT, "\\%sHolster.KF"),
            (POWER_ARMOR_HOLSTER_FORMAT, "\\PA%sHolster.KF"),
            (DEATH_FILE, "\\Death.KF"),
            (CHILD_IDLE_DIRECTORY, "\\Locomotion\\Child\\IdleAnims"),
            (FEMALE_IDLE_DIRECTORY, "\\Locomotion\\Female\\IdleAnims"),
            (MALE_IDLE_DIRECTORY, "\\Locomotion\\Male\\IdleAnims"),
            (SPECIAL_ANIMS_DIRECTORY, "\\SpecialAnims\\"),
        ] {
            e.mem.set_cstr(address, value.as_bytes());
        }
        e
    }

    /// A NUL-terminated copy of `value` in the heap.
    fn text(e: &mut Engine, value: &str) -> u32 {
        let block = e.mem.alloc(value.len() as u32 + 1);
        e.mem.set_cstr(block, value.as_bytes());
        block
    }

    /// Writes a list into `head` (an 8-byte node): the first item inline,
    /// the others in fresh nodes. The empty string is a null item.
    fn build_list(e: &mut Engine, head: u32, items: &[&str]) {
        let mut node = head;
        for (index, item) in items.iter().enumerate() {
            let value = if item.is_empty() { 0 } else { text(e, item) };
            e.mem.set_u32(node, value);
            if index + 1 < items.len() {
                let next = e.mem.alloc(8);
                e.mem.set_u32(node + 4, next);
                node = next;
            }
        }
    }

    fn make_list(e: &mut Engine, items: &[&str]) -> u32 {
        let head = e.mem.alloc(8);
        build_list(e, head, items);
        head
    }

    /// An object whose virtual table at `table` has the given (byte offset,
    /// function) slots.
    fn object(e: &mut Engine, table: u32, slots: &[(u32, u32)], size: u32) -> u32 {
        let mut words = vec![0u32; 0x90];
        for (offset, target) in slots {
            words[(*offset / 4) as usize] = *target;
        }
        e.put_vtable(table, &words);
        let block = e.mem.alloc(size);
        e.mem.set_u32(block, table);
        block
    }

    /// A double that answers `value` to every call.
    fn constant(e: &mut Engine, address: u32, value: u32) {
        e.register_double(address, move |_, _| value.into_ret());
    }

    /// An actor whose virtual functions at the given offsets answer the
    /// given values.
    fn actor(e: &mut Engine, table: u32, answers: &[(u32, u32)]) -> u32 {
        let mut slots = Vec::new();
        for (offset, value) in answers {
            let function = 0x7100_0000 + offset;
            constant(e, function, *value);
            slots.push((*offset, function));
        }
        object(e, table, &slots, 0x20)
    }

    /// Logs the calls the next function makes.
    fn log(e: &mut Engine) {
        e.call_log = Some(vec![]);
    }

    fn calls_to(e: &Engine, address: u32) -> Vec<Vec<u32>> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .filter(|(a, _)| *a == address)
            .map(|(_, args)| args.clone())
            .collect()
    }

    /// A double that records the string at argument `arg` of each call and
    /// returns `result`.
    fn recorder(e: &mut Engine, address: u32, arg: usize, result: u32) -> Rc<RefCell<Vec<String>>> {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let sink = seen.clone();
        e.register_double(address, move |e, a| {
            sink.borrow_mut()
                .push(String::from_utf8_lossy(&e.mem.cstr(a[arg])).into_owned());
            result.into_ret()
        });
        seen
    }

    // --- 004457d0 -------------------------------------------------------

    #[test]
    fn queue_creature_parts_queues_the_list_and_the_inventory_objects() {
        let mut e = engine();
        let path = text(&mut e, "Creatures\\Rat\\rat.nif");
        let model = object(&mut e, 0x0300_0000, &[(0x14, SLOT_A)], 0x20);
        constant(&mut e, SLOT_A, path);
        let creature = e.mem.alloc(0x200);
        e.register_double(RT_DYNAMIC_CAST, move |_, a| {
            assert_eq!((a[2], a[3]), (SOURCE_TYPE, CREATURE_TYPE));
            creature.into_ret()
        });
        e.register(LIST_HEAD_OF_OBJECT, |_, a| (a[0] + 4).into_ret());
        build_list(&mut e, creature + 0x118, &["Rat.nif"]);
        constant(&mut e, MODEL_ENTRIES, 0);
        let names = recorder(&mut e, MODEL_LOADER_QUEUE_MODEL, 1, 0);
        constant(&mut e, GET_INVENTORY_CHANGES, 0x5000);
        let first = e.mem.alloc(0x20);
        e.mem.set_u32(first + 8, 0x1234);
        constant(&mut e, INVENTORY_FIND_FIRST, first);
        constant(&mut e, INVENTORY_FIND_SECOND, 0);
        e.register(MODEL_LOADER_QUEUE_PARTS, |_, _| Ret::default());
        e.register(INVENTORY_RESULT_DESTRUCT, |_, _| Ret::default());
        let (this, parent, actor) = (0x100, 0x200, 0x300);
        log(&mut e);
        e.call(0x0044_57d0, &args![this, model, 7u32, parent, actor]);
        assert_eq!(*names.borrow(), vec!["Creatures\\Rat\\Rat.nif".to_string()]);
        let find = &calls_to(&e, INVENTORY_FIND_FIRST)[0];
        assert_eq!(
            (find[0], find[1], find[3], find[4]),
            (0x5000, creature, 6, 0)
        );
        assert_eq!(
            calls_to(&e, MODEL_LOADER_QUEUE_PARTS),
            vec![vec![this, 0x1234, 7, parent, actor]]
        );
        assert_eq!(calls_to(&e, INVENTORY_RESULT_DESTRUCT), vec![vec![first]]);
        assert_eq!(calls_to(&e, MEMORY_FREE), vec![vec![first]]);
    }

    #[test]
    fn queue_creature_parts_ignores_models_that_are_not_creatures() {
        let mut e = engine();
        let model = object(&mut e, 0x0300_0000, &[], 0x20);
        log(&mut e);
        e.call(0x0044_57d0, &args![0x100u32, model, 7u32, 0u32, 0u32]);
        assert_eq!(e.call_log.as_ref().unwrap().len(), 2);
    }

    // --- 004459e0 -------------------------------------------------------

    #[test]
    fn fn_004459e0_frees_only_with_the_flag() {
        let mut e = engine();
        e.register(INVENTORY_RESULT_DESTRUCT, |_, _| Ret::default());
        let block = e.mem.alloc(8);
        log(&mut e);
        assert_eq!(e.call(0x0044_59e0, &args![block, 0u32]).u32(), block);
        assert!(calls_to(&e, MEMORY_FREE).is_empty());
        e.call(0x0044_59e0, &args![block, 1u32]);
        assert_eq!(calls_to(&e, MEMORY_FREE), vec![vec![block]]);
        assert_eq!(calls_to(&e, INVENTORY_RESULT_DESTRUCT).len(), 2);
    }

    // --- 00445a10 -------------------------------------------------------

    /// A model object whose virtual function 0x14 gives `path`.
    fn model_with_path(e: &mut Engine, table: u32, path: &str) -> u32 {
        let path = text(e, path);
        constant(e, SLOT_A, path);
        object(e, table, &[(0x14, SLOT_A)], 0x20)
    }

    #[test]
    fn queue_animations_queues_the_skeleton_files_of_an_actor() {
        let mut e = engine();
        let model = model_with_path(&mut e, 0x0300_0000, "Characters\\Skeleton.nif");
        let actor = actor(&mut e, 0x0301_0000, &[(0x22c, 1), (0x100, 1)]);
        let built = recorder(&mut e, BUILD_FILE_LIST, 0, 0);
        let weapon = e.mem.alloc(0x100);
        e.mem.set_u8(weapon + 0xf4, 2);
        constant(&mut e, ACTOR_GET_CURRENT_WEAPON, weapon);
        e.mem.set_u32(HOLSTER_INDEX_TABLE + 8, 1);
        let prefix = text(&mut e, "1HM");
        e.mem.set_u32(HOLSTER_PREFIX_TABLE + 4, prefix);
        e.call(
            0x0044_5a10,
            &args![0x100u32, model, 3u32, 0x200u32, actor, 1u8, 1u8],
        );
        assert_eq!(
            *built.borrow(),
            vec![
                "Data\\Meshes\\Characters\\MTIdle.KF",
                "Data\\Meshes\\Characters\\1HMHolster.KF",
                "Data\\Meshes\\Characters\\PA1HMHolster.KF",
                "Data\\Meshes\\Characters\\Death.KF",
            ]
        );
    }

    #[test]
    fn queue_animations_skeleton_actor_with_another_model_does_nothing() {
        let mut e = engine();
        let model = model_with_path(&mut e, 0x0300_0000, "Characters\\Body.nif");
        let actor = actor(&mut e, 0x0301_0000, &[(0x22c, 1)]);
        log(&mut e);
        e.call(
            0x0044_5a10,
            &args![0x100u32, model, 3u32, 0x200u32, actor, 1u8, 1u8],
        );
        assert!(calls_to(&e, BUILD_FILE_LIST).is_empty());
        assert!(calls_to(&e, RT_DYNAMIC_CAST).is_empty());
    }

    #[test]
    fn queue_animations_queues_the_kf_file_list_of_a_skeleton() {
        let mut e = engine();
        let model = model_with_path(&mut e, 0x0300_0000, "Characters\\Skeleton.nif");
        let list = make_list(&mut e, &["walk.kf"]);
        constant(&mut e, MODEL_LOADER_BUILD_KF_FILE_LIST, list);
        let queued = recorder(&mut e, MODEL_LOADER_QUEUE_KF, 1, 0);
        log(&mut e);
        e.call(
            0x0044_5a10,
            &args![0x100u32, model, 3u32, 0x200u32, 0u32, 1u8, 0u8],
        );
        assert_eq!(*queued.borrow(), vec!["walk.kf".to_string()]);
        let calls = calls_to(&e, MODEL_LOADER_BUILD_KF_FILE_LIST);
        assert_eq!(calls.len(), 1);
        assert_eq!(
            (calls[0][0], calls[0][2..].to_vec()),
            (0x100, vec![1, 1, 0])
        );
        assert_eq!(calls_to(&e, LIST_DELETE), vec![vec![list, 1]]);
        assert_eq!(calls_to(&e, MODEL_LOADER_QUEUE_KF)[0][2..], [3, 0x200]);
    }

    #[test]
    fn queue_animations_queues_the_idle_list_of_a_female_actor() {
        let mut e = engine();
        let model = model_with_path(&mut e, 0x0300_0000, "Characters\\Body.nif");
        let actor = actor(&mut e, 0x0301_0000, &[(0x22c, 0), (0x218, 1), (0x1a0, 0)]);
        constant(&mut e, ACTOR_GET_SEX, 1);
        let roots_path = recorder(&mut e, GET_ROOT_FILENAME_LIST, 1, 0xabc0);
        e.mem.set_u32(LOADER, 0x1111);
        e.mem.set_u32(IDLE_MANAGER, 0x2222);
        e.register_double(MODEL_LOADER_COPY_FILENAME_LIST, |e, a| {
            let item = text(e, "idle.kf");
            e.mem.set_u32(a[2], item);
            Ret::default()
        });
        let queued = recorder(&mut e, MODEL_LOADER_QUEUE_KF, 1, 0);
        log(&mut e);
        e.call(
            0x0044_5a10,
            &args![0x100u32, model, 3u32, 0x200u32, actor, 0u8, 0u8],
        );
        assert_eq!(
            *roots_path.borrow(),
            vec!["Characters\\Locomotion\\Female\\IdleAnims".to_string()]
        );
        let copies = calls_to(&e, MODEL_LOADER_COPY_FILENAME_LIST);
        assert_eq!((copies[0][0], copies[0][1]), (0x1111, 0xabc0));
        assert_eq!(calls_to(&e, GET_ROOT_FILENAME_LIST)[0][0], 0x2222);
        assert_eq!(*queued.borrow(), vec!["idle.kf".to_string()]);
    }

    #[test]
    fn queue_animations_picks_the_child_and_male_idle_directories() {
        for (child, directory) in [
            (1u32, "Characters\\Locomotion\\Child\\IdleAnims"),
            (0, "Characters\\Locomotion\\Male\\IdleAnims"),
        ] {
            let mut e = engine();
            let model = model_with_path(&mut e, 0x0300_0000, "Characters\\Body.nif");
            let actor = actor(
                &mut e,
                0x0301_0000,
                &[(0x22c, 0), (0x218, 1), (0x1a0, child)],
            );
            constant(&mut e, ACTOR_GET_SEX, 0);
            let roots_path = recorder(&mut e, GET_ROOT_FILENAME_LIST, 1, 0);
            e.register(MODEL_LOADER_COPY_FILENAME_LIST, |_, _| Ret::default());
            e.call(
                0x0044_5a10,
                &args![0x100u32, model, 3u32, 0x200u32, actor, 0u8, 0u8],
            );
            assert_eq!(*roots_path.borrow(), vec![directory.to_string()]);
        }
    }

    #[test]
    fn queue_animations_queues_the_special_animations_of_the_cast() {
        let mut e = engine();
        let model = model_with_path(&mut e, 0x0300_0000, "Characters\\Body.nif");
        let actor = actor(&mut e, 0x0301_0000, &[(0x22c, 0), (0x218, 0)]);
        let animation = e.mem.alloc(0x20);
        e.register_double(RT_DYNAMIC_CAST, move |_, a| {
            assert_eq!((a[2], a[3]), (SOURCE_TYPE, ANIMATION_TYPE));
            animation.into_ret()
        });
        constant(&mut e, TESANIMATION_HAS_KF_FILES, 1);
        e.register(LIST_HEAD_OF_OBJECT, |_, a| (a[0] + 4).into_ret());
        build_list(&mut e, animation + 4, &["a.kf", "b.kf"]);
        e.register(LIST_PUSH_BACK, |e, a| {
            // Appends the item whose address is a[1] to the list at a[0].
            let item = e.mem.u32(a[1]);
            let mut node = a[0];
            if e.mem.u32(node) != 0 {
                while e.mem.u32(node + 4) != 0 {
                    node = e.mem.u32(node + 4);
                }
                let next = e.mem.alloc(8);
                e.mem.set_u32(node + 4, next);
                node = next;
            }
            e.mem.set_u32(node, item);
            Ret::default()
        });
        let queued = recorder(&mut e, MODEL_LOADER_QUEUE_KF, 1, 0);
        e.call(
            0x0044_5a10,
            &args![0x100u32, model, 3u32, 0x200u32, actor, 0u8, 0u8],
        );
        assert_eq!(
            *queued.borrow(),
            vec![
                "Characters\\SpecialAnims\\a.kf".to_string(),
                "Characters\\SpecialAnims\\b.kf".to_string()
            ]
        );
    }

    // --- 00446390 .. 00446500 ---------------------------------------------

    #[test]
    fn fn_00446390_reads_the_signed_weapon_type() {
        let mut e = engine();
        let weapon = e.mem.alloc(0x100);
        e.mem.set_u8(weapon + 0xf4, 0xfe);
        assert_eq!(e.call(0x0044_6390, &args![weapon]).i32(), -2);
    }

    #[test]
    fn fn_004463b0_queues_models_with_and_without_entries() {
        let mut e = engine();
        let list = make_list(&mut e, &["a", "", "c"]);
        let plain = recorder(&mut e, MODEL_LOADER_QUEUE_MODEL, 1, 0);
        let with_entry = recorder(&mut e, MODEL_LOADER_QUEUE_MODEL_WITH_ENTRY, 1, 0);
        log(&mut e);
        e.call(
            0x0044_63b0,
            &args![0x100u32, list, 0u32, 5u32, 0x300u32, 0u32],
        );
        assert_eq!(*plain.borrow(), vec!["a".to_string(), "c".to_string()]);
        assert_eq!(
            calls_to(&e, MODEL_LOADER_QUEUE_MODEL)[0][2..],
            [5, 0x300, 0, 1, 0, 0]
        );
        // With entries: the name after the null item takes entry index 2,
        // which is past the end of the two entries.
        let entries = e.mem.alloc(8);
        let array = e.mem.alloc(8);
        e.mem.set_u32(array, 2);
        e.mem.set_u32(array + 4, entries);
        e.mem.set_u32(entries, 0xe0);
        e.mem.set_u32(entries + 4, 0xe1);
        let prefix = text(&mut e, "P\\");
        log(&mut e);
        e.call(
            0x0044_63b0,
            &args![0x100u32, list, array, 5u32, 0x300u32, prefix],
        );
        assert_eq!(
            *with_entry.borrow(),
            vec!["P\\a".to_string(), "P\\c".to_string()]
        );
        let calls = calls_to(&e, MODEL_LOADER_QUEUE_MODEL_WITH_ENTRY);
        assert_eq!((calls[0][2], calls[1][2]), (0xe0, 0));
        assert_eq!(calls[0][3..], [5, 0x300, 0, 1, 0, 0]);
    }

    #[test]
    fn fn_004464c0_returns_zero_past_the_count() {
        let mut e = engine();
        let entries = e.mem.alloc(8);
        let array = e.mem.alloc(8);
        e.mem.set_u32(array, 2);
        e.mem.set_u32(array + 4, entries);
        e.mem.set_u32(entries + 4, 0xe1);
        assert_eq!(e.call(0x0044_64c0, &args![array, 1u32]).u32(), 0xe1);
        assert_eq!(e.call(0x0044_64c0, &args![array, 2u32]).u32(), 0);
    }

    #[test]
    fn fn_00446500_drains_the_list() {
        let mut e = engine();
        let list = make_list(&mut e, &["x", "y"]);
        let queued = recorder(&mut e, MODEL_LOADER_QUEUE_KF, 1, 0);
        log(&mut e);
        e.call(0x0044_6500, &args![0x100u32, list, 4u32, 0x300u32, 0u32]);
        assert_eq!(*queued.borrow(), vec!["x".to_string(), "y".to_string()]);
        assert_eq!(calls_to(&e, MODEL_LOADER_QUEUE_KF)[0][2..], [4, 0x300]);
        assert_eq!(calls_to(&e, LIST_REMOVE_HEAD).len(), 2);
        assert_eq!(calls_to(&e, MEMORY_FREE).len(), 2);
        assert_eq!(e.mem.u32(list), 0);
    }

    #[test]
    fn fn_00446500_skips_empty_heads_and_applies_the_prefix() {
        let mut e = engine();
        let list = make_list(&mut e, &["", "z"]);
        let prefix = text(&mut e, "Dir\\");
        let queued = recorder(&mut e, MODEL_LOADER_QUEUE_KF, 1, 0);
        e.call(0x0044_6500, &args![0x100u32, list, 4u32, 0x300u32, prefix]);
        assert_eq!(*queued.borrow(), vec!["Dir\\z".to_string()]);
    }

    // --- 004465f0 -------------------------------------------------------

    /// The pieces `004465f0` works with: the loader (maps at +4 and +0x14),
    /// the queued objects' virtual tables and the callee doubles. The
    /// calls that matter are noted in `calls`.
    struct Replacement {
        this: u32,
        animation: u32,
        calls: Rc<RefCell<Vec<&'static str>>>,
    }

    fn replacement(e: &mut Engine, children: u32, known_model: u32) -> Replacement {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let kf_map = object(e, 0x0300_0000, &[(8, SLOT_A)], 0x10);
        let list_map = object(e, 0x0301_0000, &[(8, SLOT_B), (0x10, SLOT_C)], 0x10);
        e.register_double(SLOT_A, move |e, a| {
            if known_model != 0 {
                e.mem.set_u32(a[2], known_model);
            }
            u32::from(known_model != 0).into_ret()
        });
        constant(e, SLOT_B, 0);
        let log = calls.clone();
        e.register_double(SLOT_C, move |_, a| {
            assert_eq!(a[3], 1);
            log.borrow_mut().push("map add");
            Ret::default()
        });
        let this = e.mem.alloc(0x30);
        e.mem.set_u32(this + 4, kf_map);
        e.mem.set_u32(this + 0x14, list_map);
        let animation = e.mem.alloc(0x10);
        let log = calls.clone();
        e.register_double(ANIMATION_KF_LOADED, move |_, _| {
            log.borrow_mut().push("animation takes model");
            Ret::default()
        });
        e.register_double(QUEUED_FILE_CONSTRUCT, move |e, a| {
            let counter = e.mem.alloc(0x10);
            e.mem.set_u32(counter + 8, children);
            e.mem.set_u32(a[0] + 0x20, counter);
            a[0].into_ret()
        });
        let mut words = vec![0u32; 0x40];
        words[8] = SLOT_A + 0x10;
        e.put_vtable(0x0302_0000, &words);
        let log = calls.clone();
        e.register_double(SLOT_A + 0x10, move |_, _| {
            log.borrow_mut().push("kf queued");
            Ret::default()
        });
        e.register_double(QUEUED_REPLACEMENT_KF_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], 0x0302_0000);
            a[0].into_ret()
        });
        let log = calls.clone();
        e.register_double(QUEUED_FILE_SET_PARENT, move |_, _| {
            log.borrow_mut().push("set parent");
            Ret::default()
        });
        let log = calls.clone();
        e.register_double(TASK_SET_DONE, move |_, _| {
            log.borrow_mut().push("state done");
            Ret::default()
        });
        e.mem
            .set_u32(QUEUED_REPLACEMENT_KF_LIST_VTABLE + 0x28, SLOT_B + 0x10);
        let log = calls.clone();
        e.register_double(SLOT_B + 0x10, move |_, _| {
            log.borrow_mut().push("check finished");
            Ret::default()
        });
        Replacement {
            this,
            animation,
            calls,
        }
    }

    #[test]
    fn fn_004465f0_hands_a_loaded_model_to_the_animation() {
        let mut e = engine();
        let r = replacement(&mut e, 1, 0xabc);
        let list = make_list(&mut e, &["a.kf"]);
        log(&mut e);
        e.call(
            0x0044_65f0,
            &args![r.this, list, r.animation, 2u32, 0x300u32, 0u32],
        );
        assert_eq!(*r.calls.borrow(), vec!["animation takes model"]);
        assert_eq!(
            calls_to(&e, ANIMATION_KF_LOADED),
            vec![vec![r.animation, 0xabc]]
        );
        assert_eq!(
            calls_to(&e, MEMORY_CONTEXT_ENTER)[0][1..],
            [0x33, 1, MODEL_LOADER_SOURCE, 0xe9f]
        );
        assert_eq!(calls_to(&e, MEMORY_CONTEXT_LEAVE).len(), 1);
        assert_eq!(e.mem.u32(list), 0);
    }

    #[test]
    fn fn_004465f0_queues_a_replacement_list_with_children() {
        let mut e = engine();
        let r = replacement(&mut e, 1, 0);
        let list = make_list(&mut e, &["a.kf", "b.kf"]);
        let prefix = text(&mut e, "Dir\\");
        log(&mut e);
        e.call(
            0x0044_65f0,
            &args![r.this, list, r.animation, 2u32, 0x300u32, prefix],
        );
        assert_eq!(
            *r.calls.borrow(),
            vec![
                "set parent",
                "kf queued",
                "set parent",
                "kf queued",
                "set parent",
                "map add",
                "state done",
                "check finished"
            ]
        );
        // One list was created, with the key as its context.
        let created = calls_to(&e, QUEUED_FILE_CONSTRUCT);
        assert_eq!(created.len(), 1);
        assert_eq!(created[0][1], 2);
        let kfs = calls_to(&e, QUEUED_REPLACEMENT_KF_CONSTRUCT);
        assert_eq!(kfs.len(), 2);
        assert_eq!(kfs[0][2..4], [2, r.animation]);
        assert_eq!(kfs[0][4], created[0][0]);
        assert_eq!(calls_to(&e, MEMORY_FREE).len(), 2);
    }

    #[test]
    fn fn_004465f0_drops_a_list_without_children() {
        let mut e = engine();
        let r = replacement(&mut e, 0, 0);
        let list = make_list(&mut e, &["a.kf"]);
        e.call(
            0x0044_65f0,
            &args![r.this, list, r.animation, 2u32, 0x300u32, 0u32],
        );
        assert_eq!(*r.calls.borrow(), vec!["set parent", "kf queued"]);
    }

    #[test]
    fn fn_004465f0_does_nothing_without_a_list_animation_or_item() {
        let mut e = engine();
        let r = replacement(&mut e, 1, 0);
        let empty = make_list(&mut e, &[""]);
        log(&mut e);
        e.call(
            0x0044_65f0,
            &args![r.this, 0u32, r.animation, 2u32, 0u32, 0u32],
        );
        e.call(0x0044_65f0, &args![r.this, empty, 0u32, 2u32, 0u32, 0u32]);
        e.call(
            0x0044_65f0,
            &args![r.this, empty, r.animation, 2u32, 0u32, 0u32],
        );
        assert!(calls_to(&e, MEMORY_CONTEXT_ENTER).is_empty());
    }

    // --- 00446990 .. 00446a60 -----------------------------------------------

    #[test]
    fn fn_00446990_counts_the_children() {
        let mut e = engine();
        let file = e.mem.alloc(0x30);
        assert_eq!(e.call(0x0044_6990, &args![file]).u32(), 0);
        let children = e.mem.alloc(0x10);
        e.mem.set_u32(children + 8, 3);
        e.mem.set_u32(file + 0x20, children);
        assert_eq!(e.call(0x0044_6990, &args![file]).u32(), 3);
    }

    #[test]
    fn fn_004469c0_builds_the_replacement_list() {
        let mut e = engine();
        e.register(QUEUED_FILE_CONSTRUCT, |_, a| a[0].into_ret());
        let list = e.new_object::<QueuedReplacementKFList>();
        e.mem.set_u32(list.addr() + 0x2c, 9);
        e.mem.set_u32(list.addr() + 0x30, 9);
        log(&mut e);
        e.call(0x0044_69c0, &args![list, 0xaa0u32, 0x33u32]);
        assert_eq!(
            calls_to(&e, QUEUED_FILE_CONSTRUCT),
            vec![vec![list.addr(), 0x33]]
        );
        assert_eq!(e.mem.u32(list.addr()), QUEUED_REPLACEMENT_KF_LIST_VTABLE);
        assert_eq!(e.get(list, QueuedReplacementKFList::pAnim), Ptr::new(0xaa0));
        assert_eq!(
            e.get(list, QueuedReplacementKFList::iPostProcessingChildCount),
            0
        );
        assert_eq!(
            e.get(list, QueuedReplacementKFList::iPostProcessedChildCount),
            0
        );
    }

    #[test]
    fn queued_replacement_kf_list_scalar_deleting_destructor_frees_with_the_flag() {
        let mut e = engine();
        e.register(QUEUED_FILE_DESTRUCT, |_, _| Ret::default());
        let list = e.new_object::<QueuedReplacementKFList>();
        log(&mut e);
        assert_eq!(e.call(0x0044_6a10, &args![list, 0u32]).u32(), list.addr());
        assert!(calls_to(&e, MEMORY_FREE).is_empty());
        e.call(0x0044_6a10, &args![list, 1u32]);
        assert_eq!(calls_to(&e, MEMORY_FREE), vec![vec![list.addr()]]);
        assert_eq!(calls_to(&e, QUEUED_FILE_DESTRUCT).len(), 2);
    }

    #[test]
    fn queued_replacement_kf_list_check_finished_calls_the_base() {
        let mut e = engine();
        e.register(QUEUED_FILE_CHECK_FINISHED, |_, _| Ret::default());
        log(&mut e);
        e.call(0x0044_6a40, &args![0x1234u32]);
        assert_eq!(calls_to(&e, QUEUED_FILE_CHECK_FINISHED), vec![vec![0x1234]]);
    }

    #[test]
    fn fn_00446a60_chooses_the_model_of_the_form() {
        let mut e = engine();
        let (form, reference, owner) = (0x1000, 0x2000, 0x3000);
        e.register_double(REFERENCE_GET_FORM, move |_, a| {
            assert_eq!(a[0], reference);
            form.into_ret()
        });
        constant(&mut e, REFERENCE_GET_TES_MODEL, 0xaa);
        // The reference's own form: its model.
        assert_eq!(
            e.call(0x0044_6a60, &args![0u32, form, reference]).u32(),
            0xaa
        );
        // A form of type 0x28 (the reference has another form).
        constant(&mut e, REFERENCE_GET_FORM, 0xdead);
        e.register(FORM_TYPE, |_, a| {
            (if a[0] == 0x3000 { 0x2a } else { 0x28 }).into_ret()
        });
        constant(&mut e, WEAPON_GET_MODEL, 0xbb);
        assert_eq!(
            e.call(0x0044_6a60, &args![0u32, form, reference]).u32(),
            0xbb
        );
        // Another form that casts to the model class is itself the model.
        e.register(FORM_TYPE, |_, a| {
            (if a[0] == 0x3000 { 0x2a } else { 0x30 }).into_ret()
        });
        e.register(RT_DYNAMIC_CAST, |_, a| {
            (if a[3] == FORM_MODEL_TYPE { a[0] } else { 0 }).into_ret()
        });
        assert_eq!(
            e.call(0x0044_6a60, &args![0u32, form, reference]).u32(),
            form
        );
        // A biped form takes the sex of an owner of type 0x2a.
        e.register(RT_DYNAMIC_CAST, |_, a| {
            (if a[3] == FORM_BIPED_TYPE { a[0] } else { 0 }).into_ret()
        });
        constant(&mut e, REFERENCE_GET_OWNER, owner);
        constant(&mut e, ACTOR_BASE_GET_SEX, 1);
        e.register(BIPED_GET_WORLD_MODEL, |_, a| (0xcc00 + a[1]).into_ret());
        assert_eq!(
            e.call(0x0044_6a60, &args![0u32, form, reference]).u32(),
            0xcc01
        );
        // Without a reference the sex stays 0; a form of no class gives 0.
        assert_eq!(e.call(0x0044_6a60, &args![0u32, form, 0u32]).u32(), 0xcc00);
        e.register(RT_DYNAMIC_CAST, |_, _| 0u32.into_ret());
        assert_eq!(e.call(0x0044_6a60, &args![0u32, form, 0u32]).u32(), 0);
    }

    // --- 00446b50 -------------------------------------------------------

    #[test]
    fn fn_00446b50_requeues_references_with_a_new_priority() {
        let mut e = engine();
        let map = e.mem.alloc(0x10);
        let this = e.mem.alloc(0x10);
        e.mem.set_u32(this + 8, map);
        e.register(MAP_ITERATOR_CONSTRUCT, |_, _| Ret::default());
        e.register(MAP_ITERATOR_DESTRUCT, |_, _| Ret::default());
        let step = Rc::new(Cell::new(0u32));
        let counter = step.clone();
        e.register_double(MAP_ITERATOR_IS_DONE, move |_, _| {
            u32::from(counter.get() >= 2).into_ret()
        });
        // Entry 1: reference 0x10 with a priority 5 task; entry 2:
        // reference 0x20 with a priority 3 task.
        let task_a = object(&mut e, 0x0300_0000, &[(0x1c, SLOT_A)], 0x20);
        let task_b = object(&mut e, 0x0301_0000, &[(0x1c, SLOT_B)], 0x20);
        let counter = step.clone();
        e.register_double(MAP_NEXT_ENTRY, move |e, a| {
            let (reference, task) = if counter.get() == 0 {
                (0x10, task_a)
            } else {
                (0x20, task_b)
            };
            counter.set(counter.get() + 1);
            e.mem.set_u32(a[2], reference);
            e.mem.set_u32(a[3], task);
            1u32.into_ret()
        });
        constant(&mut e, REFERENCE_CELL, 0x7000);
        e.register_double(TASK_PRIORITY, move |_, a| {
            (if a[0] == task_a { 5 } else { 3 }).into_ret()
        });
        e.register(REFERENCE_IS_3D_CRITICAL, |_, a| {
            u32::from(a[0] == 0x10).into_ret()
        });
        e.register_double(GET_CELL_PRIORITY, |_, a| {
            assert_eq!(a[1..], [0x7000, 0]);
            3u32.into_ret()
        });
        e.mem.set_u32(TES_GLOBAL, 0x9999);
        let seen = Rc::new(RefCell::new(Vec::new()));
        for slot in [SLOT_A, SLOT_B] {
            let sink = seen.clone();
            e.register_double(slot, move |_, a| {
                sink.borrow_mut().push((slot, a[1]));
                Ret::default()
            });
        }
        // Task A (priority 5) is in the given cell and critical: wanted 0.
        // Task B is not critical: the cell priority 3 equals its own.
        e.call(0x0044_6b50, &args![this, 0x7000u32]);
        assert_eq!(*seen.borrow(), vec![(SLOT_A, 0)]);
    }

    // --- 00446c90 .. 00446ff0 ---------------------------------------------

    #[test]
    fn fn_00446c90_asks_the_reference_map() {
        let mut e = engine();
        let map = object(&mut e, 0x0300_0000, &[(0x44, SLOT_A)], 0x10);
        constant(&mut e, SLOT_A, 17);
        let this = e.mem.alloc(0x10);
        e.mem.set_u32(this + 8, map);
        assert_eq!(e.call(0x0044_6c90, &args![this]).u32(), 17);
    }

    /// The loader's clone thread: id, running count and a queue whose
    /// virtual function 0x10 gives `queued`.
    fn clone_thread(e: &mut Engine, id: u32, running: u32, queued: u32) -> u32 {
        let queue = object(e, 0x0302_0000, &[(0x10, SLOT_C)], 0x10);
        constant(e, SLOT_C, queued);
        let thread = e.mem.alloc(0x40);
        e.mem.set_u32(thread + 8, id);
        e.mem.set_u32(thread + 0x34, running);
        e.mem.set_u32(thread + 0x38, queue);
        thread
    }

    fn status_engine(setting: u8, references: u32, cloning: u32, tasks: u32, post: u32) -> Engine {
        let mut e = engine();
        let scratch = e.mem.alloc(8);
        e.register_double(SETTING_VALUE_POINTER, move |e, a| {
            assert_eq!(a[0], 0x6666);
            e.mem.set_u8(scratch, setting);
            scratch.into_ret()
        });
        let map = object(&mut e, 0x0300_0000, &[(0x44, SLOT_A)], 0x10);
        constant(&mut e, SLOT_A, references);
        let thread = clone_thread(&mut e, 1, cloning, 0);
        let loader = e.mem.alloc(0x40);
        e.mem.set_u32(loader + 8, map);
        e.mem.set_u32(loader + 0x28, thread);
        constant(&mut e, TASK_MANAGER_TASK_COUNT, tasks);
        let queues = object(&mut e, 0x0303_0000, &[(0xc, SLOT_B)], 0x10);
        constant(&mut e, SLOT_B, post);
        let manager = e.mem.alloc(0x80);
        e.mem.set_u32(manager + 0x64, queues);
        e.mem.set_u32(TASK_MANAGER, manager);
        e.mem.set_u32(LOADER, loader);
        e.register(FORMAT_BUFFER, |_, _| Ret::default());
        e.register(SET_WINDOW_TEXT, |_, _| Ret::default());
        let owner = e.mem.alloc(0x10);
        e.mem.set_u32(owner + 8, 0x4444);
        e.mem.set_u32(WINDOW_OWNER, owner);
        e.mem.set_u32(WINDOW_TEXT, 0x5555);
        e.mem.set_u32(LOADING_SETTING, 0x6666);
        e
    }

    #[test]
    fn fn_00446cb0_formats_the_status_while_work_remains() {
        let mut e = status_engine(0, 4, 3, 2, 1);
        let loader = e.mem.u32(LOADER);
        log(&mut e);
        e.call(0x0044_6cb0, &args![loader]);
        let calls = calls_to(&e, FORMAT_BUFFER);
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0][1..],
            [PATH_BUFFER_SIZE, LOADING_STATUS_FORMAT, 4, 3, 2, 1]
        );
        assert_eq!(e.mem.u8(STATUS_SHOWN_FLAG), 1);
        assert!(calls_to(&e, SET_WINDOW_TEXT).is_empty());
    }

    #[test]
    fn fn_00446cb0_resets_the_window_text_once() {
        let mut e = status_engine(0, 0, 0, 0, 0);
        let loader = e.mem.u32(LOADER);
        e.mem.set_u8(STATUS_SHOWN_FLAG, 1);
        log(&mut e);
        e.call(0x0044_6cb0, &args![loader]);
        assert_eq!(calls_to(&e, SET_WINDOW_TEXT), vec![vec![0x4444, 0x5555]]);
        assert_eq!(e.mem.u8(STATUS_SHOWN_FLAG), 0);
        e.call(0x0044_6cb0, &args![loader]);
        assert_eq!(calls_to(&e, SET_WINDOW_TEXT).len(), 1);
    }

    #[test]
    fn fn_00446cb0_does_nothing_when_the_setting_is_set() {
        let mut e = status_engine(1, 4, 3, 2, 1);
        let loader = e.mem.u32(LOADER);
        log(&mut e);
        e.call(0x0044_6cb0, &args![loader]);
        assert!(calls_to(&e, FORMAT_BUFFER).is_empty());
        assert_eq!(e.mem.u8(STATUS_SHOWN_FLAG), 0);
    }

    #[test]
    fn fn_00446da0_asks_the_queue_table() {
        let mut e = engine();
        let queues = object(&mut e, 0x0300_0000, &[(0xc, SLOT_A)], 0x10);
        constant(&mut e, SLOT_A, 6);
        let manager = e.mem.alloc(0x80);
        e.mem.set_u32(manager + 0x64, queues);
        assert_eq!(e.call(0x0044_6da0, &args![manager]).u32(), 6);
    }

    #[test]
    fn fn_00446dc0_reads_the_clone_thread_of_the_loader() {
        let mut e = engine();
        let thread = clone_thread(&mut e, 1, 2, 5);
        let loader = e.mem.alloc(0x40);
        e.mem.set_u32(loader + 0x28, thread);
        assert_eq!(e.call(0x0044_6dc0, &args![loader]).u32(), 7);
    }

    #[test]
    fn fn_00446de0_adds_the_running_count_to_the_queue() {
        let mut e = engine();
        let thread = clone_thread(&mut e, 1, 2, 5);
        assert_eq!(e.call(0x0044_6de0, &args![thread]).u32(), 7);
    }

    #[test]
    fn fn_00446e10_reads_the_setting_byte() {
        let mut e = engine();
        e.mem.set_u32(LOADING_SETTING, 0x6666);
        let scratch = e.mem.alloc(8);
        e.mem.set_u8(scratch, 3);
        e.register_double(SETTING_VALUE_POINTER, move |_, a| {
            assert_eq!(a[0], 0x6666);
            scratch.into_ret()
        });
        assert_eq!(e.call(0x0044_6e10, &args![]).u8(), 3);
    }

    #[test]
    fn fn_00446e30_runs_the_status_on_the_loader() {
        let mut e = status_engine(1, 0, 0, 0, 0);
        log(&mut e);
        e.call(0x0044_6e30, &args![]);
        assert_eq!(calls_to(&e, SETTING_VALUE_POINTER).len(), 1);
    }

    #[test]
    fn fn_00446e40_needs_both_answers_and_asks_both() {
        for (first, second) in [(1u32, 1u32), (1, 0), (0, 1), (0, 0)] {
            let mut e = engine();
            e.mem.set_u32(LOADER, 0x1234);
            constant(&mut e, LOADER_CHECK_FIRST, first);
            constant(&mut e, LOADER_CHECK_SECOND, second);
            log(&mut e);
            assert_eq!(
                e.call(0x0044_6e40, &args![]).bool(),
                first == 1 && second == 1
            );
            assert_eq!(calls_to(&e, LOADER_CHECK_FIRST), vec![vec![0x1234]]);
            assert_eq!(calls_to(&e, LOADER_CHECK_SECOND), vec![vec![0x1234]]);
        }
    }

    #[test]
    fn fn_00446ea0_updates_only_when_the_test_holds() {
        for (answer, updates) in [(1u32, 1usize), (0, 0)] {
            let mut e = engine();
            e.mem.set_u32(MAIN_LOOP_OBJECT, 0x777);
            e.register(REFRESH_FIRST, |_, _| Ret::default());
            e.register(REFRESH_SECOND, |_, _| Ret::default());
            constant(&mut e, MAIN_LOOP_TEST, answer);
            e.register(MAIN_LOOP_UPDATE, |_, _| Ret::default());
            log(&mut e);
            e.call(0x0044_6ea0, &args![]);
            assert_eq!(calls_to(&e, REFRESH_FIRST), vec![vec![REFRESH_OBJECT, 0]]);
            assert_eq!(calls_to(&e, REFRESH_SECOND).len(), 1);
            assert_eq!(
                calls_to(&e, MAIN_LOOP_TEST).len(),
                if answer == 1 { 2 } else { 1 }
            );
            assert_eq!(calls_to(&e, MAIN_LOOP_UPDATE).len(), updates);
        }
    }

    #[test]
    fn fn_00446ef0_returns_the_object_address() {
        let mut e = engine();
        assert_eq!(e.call(0x0044_6ef0, &args![]).u32(), 0x011d_effc);
    }

    #[test]
    fn fn_00446f00_reads_the_clone_thread_of_the_global_loader() {
        let mut e = engine();
        let thread = clone_thread(&mut e, 1, 1, 1);
        let loader = e.mem.alloc(0x40);
        e.mem.set_u32(loader + 0x28, thread);
        e.mem.set_u32(LOADER, loader);
        assert_eq!(e.call(0x0044_6f00, &args![]).u32(), 2);
    }

    /// The thread-id calls: the current thread is 9; the clone thread has
    /// `id`.
    fn thread_engine(id: u32) -> (Engine, u32) {
        let mut e = engine();
        let thread = clone_thread(&mut e, id, 0, 0);
        let loader = e.mem.alloc(0x40);
        e.mem.set_u32(loader + 0x28, thread);
        e.mem.set_u32(LOADER, loader);
        constant(&mut e, CURRENT_THREAD_ID, 9);
        for address in [
            CLONE_THREAD_CALL_FIRST,
            CLONE_THREAD_CALL_SECOND,
            CLONE_THREAD_CALL_THIRD,
        ] {
            e.register(address, |_, _| Ret::default());
        }
        (e, thread)
    }

    #[test]
    fn fn_00446f10_and_fn_00446f20_reach_the_thread_through_the_loader() {
        let (mut e, thread) = thread_engine(4);
        let loader = e.mem.u32(LOADER);
        log(&mut e);
        e.call(0x0044_6f10, &args![]);
        e.call(0x0044_6f20, &args![loader]);
        assert_eq!(
            calls_to(&e, CLONE_THREAD_CALL_SECOND),
            vec![vec![thread], vec![thread]]
        );
    }

    #[test]
    fn fn_00446f40_runs_only_from_another_thread() {
        let (mut e, thread) = thread_engine(9);
        log(&mut e);
        e.call(0x0044_6f40, &args![thread]);
        assert!(calls_to(&e, CLONE_THREAD_CALL_FIRST).is_empty());
        e.mem.set_u32(thread + 8, 4);
        e.call(0x0044_6f40, &args![thread]);
        assert_eq!(calls_to(&e, CLONE_THREAD_CALL_FIRST), vec![vec![thread]]);
        assert_eq!(calls_to(&e, CLONE_THREAD_CALL_SECOND), vec![vec![thread]]);
    }

    #[test]
    fn fn_00446f70_calls_both_thread_functions_in_order() {
        let (mut e, thread) = thread_engine(9);
        log(&mut e);
        e.call(0x0044_6f70, &args![thread]);
        let order: Vec<u32> = e
            .call_log
            .as_ref()
            .unwrap()
            .iter()
            .map(|(a, _)| *a)
            .collect();
        assert_eq!(
            order,
            vec![
                0x0044_6f70,
                CLONE_THREAD_CALL_FIRST,
                CLONE_THREAD_CALL_SECOND
            ]
        );
    }

    #[test]
    fn fn_00446f90_and_fn_00446fa0_reach_the_thread_through_the_loader() {
        let (mut e, thread) = thread_engine(4);
        let loader = e.mem.u32(LOADER);
        log(&mut e);
        e.call(0x0044_6f90, &args![]);
        e.call(0x0044_6fa0, &args![loader]);
        assert_eq!(
            calls_to(&e, CLONE_THREAD_CALL_THIRD),
            vec![vec![thread], vec![thread]]
        );
    }

    #[test]
    fn fn_00446fc0_runs_only_from_another_thread() {
        let (mut e, thread) = thread_engine(9);
        log(&mut e);
        e.call(0x0044_6fc0, &args![thread]);
        assert!(calls_to(&e, CLONE_THREAD_CALL_THIRD).is_empty());
        e.mem.set_u32(thread + 8, 4);
        e.call(0x0044_6fc0, &args![thread]);
        assert_eq!(calls_to(&e, CLONE_THREAD_CALL_THIRD), vec![vec![thread]]);
    }

    #[test]
    fn fn_00446ff0_calls_both_thread_functions_in_order() {
        let (mut e, thread) = thread_engine(9);
        log(&mut e);
        e.call(0x0044_6ff0, &args![thread]);
        let order: Vec<u32> = e
            .call_log
            .as_ref()
            .unwrap()
            .iter()
            .map(|(a, _)| *a)
            .collect();
        assert_eq!(
            order,
            vec![
                0x0044_6ff0,
                CLONE_THREAD_CALL_FIRST,
                CLONE_THREAD_CALL_THIRD
            ]
        );
    }

    // --- 00447010 .. 00447300 ---------------------------------------------

    #[test]
    fn fn_00447010_and_fn_00447040_add_to_their_maps() {
        let mut e = engine();
        let models = object(&mut e, 0x0300_0000, &[(0x10, SLOT_A)], 0x10);
        let kf_models = object(&mut e, 0x0301_0000, &[(0x10, SLOT_B)], 0x10);
        e.register_double(SLOT_A, |e, a| {
            assert_eq!(e.mem.u32(a[2]), 0xbeef);
            assert_eq!(a[3], 0);
            1u32.into_ret()
        });
        e.register_double(SLOT_B, |e, a| {
            assert_eq!(e.mem.u32(a[2]), 0xfeed);
            0u32.into_ret()
        });
        let this = e.mem.alloc(0x30);
        e.mem.set_u32(this, models);
        e.mem.set_u32(this + 4, kf_models);
        assert!(e
            .call(0x0044_7010, &args![this, 0x111u32, 0xbeefu32])
            .bool());
        assert!(!e
            .call(0x0044_7040, &args![this, 0x111u32, 0xfeedu32])
            .bool());
    }

    /// A map whose lookup (virtual function 8) answers `answer` (a model,
    /// or 0 for not found).
    fn lookup_map(e: &mut Engine, table: u32, answer: u32) -> u32 {
        let map = object(e, table, &[(8, SLOT_A)], 0x10);
        e.register_double(SLOT_A, move |e, a| {
            if answer != 0 {
                e.mem.set_u32(a[2], answer);
            }
            u32::from(answer != 0).into_ret()
        });
        map
    }

    #[test]
    fn load_file_builds_a_queued_model_when_the_map_has_none() {
        let mut e = engine();
        let map = lookup_map(&mut e, 0x0300_0000, 0);
        let this = e.mem.alloc(0x30);
        e.mem.set_u32(this, map);
        let model = 0x5000;
        e.register(QUEUED_MODEL_CONSTRUCT, |_, _| Ret::default());
        e.register(QUEUED_MODEL_SET_FLAG_20, |_, _| Ret::default());
        e.register(SET_FLAG_BIT_10, |_, _| Ret::default());
        e.register(QUEUED_MODEL_RUN, |_, _| Ret::default());
        e.register_double(QUEUED_MODEL_FINISH, move |e, a| {
            e.mem.set_u32(a[0] + 0x30, model);
            Ret::default()
        });
        e.register(QUEUED_MODEL_DESTRUCT, |_, _| Ret::default());
        e.register(ADD_REFERENCE, |_, _| Ret::default());
        constant(&mut e, MODEL_OBJECT_3D, 0x6000);
        log(&mut e);
        let name = 0x400;
        let result = e.call(0x0044_7080, &args![this, name, 3u32, 1u8, 0u32, 1u8, 0u8]);
        assert_eq!(result.u32(), 0x6000);
        let construct = calls_to(&e, QUEUED_MODEL_CONSTRUCT);
        assert_eq!(construct[0][1..], [name, 0, 3, 1, 0]);
        let task = construct[0][0];
        assert_eq!(calls_to(&e, QUEUED_MODEL_SET_FLAG_20), vec![vec![task, 1]]);
        assert_eq!(calls_to(&e, SET_FLAG_BIT_10), vec![vec![1, task + 0x3c]]);
        assert_eq!(calls_to(&e, QUEUED_MODEL_DESTRUCT), vec![vec![task]]);
        assert_eq!(calls_to(&e, ADD_REFERENCE), vec![vec![model]]);
        assert_eq!(calls_to(&e, MODEL_OBJECT_3D), vec![vec![model]]);
    }

    #[test]
    fn load_file_uses_a_known_model_and_logs_a_zero_reference_total() {
        let mut e = engine();
        let map = lookup_map(&mut e, 0x0300_0000, 0x5000);
        let this = e.mem.alloc(0x30);
        e.mem.set_u32(this, map);
        constant(&mut e, MODEL_REFERENCE_TOTAL, 0);
        e.register(LOG, |_, _| Ret::default());
        e.register(ADD_REFERENCE, |_, _| Ret::default());
        constant(&mut e, MODEL_OBJECT_3D, 0x6000);
        log(&mut e);
        // `no_reference` set: no extra reference.
        let result = e.call(
            0x0044_7080,
            &args![this, 0x400u32, 3u32, 1u8, 0u32, 1u8, 1u8],
        );
        assert_eq!(result.u32(), 0x6000);
        assert_eq!(
            calls_to(&e, LOG),
            vec![vec![LOAD_FILE_ZERO_REFERENCE_MESSAGE, 0x400]]
        );
        assert!(calls_to(&e, ADD_REFERENCE).is_empty());
        assert!(calls_to(&e, QUEUED_MODEL_CONSTRUCT).is_empty());
    }

    #[test]
    fn load_file_returns_zero_without_a_model() {
        let mut e = engine();
        let map = lookup_map(&mut e, 0x0300_0000, 0);
        let this = e.mem.alloc(0x30);
        e.mem.set_u32(this, map);
        for address in [
            QUEUED_MODEL_CONSTRUCT,
            QUEUED_MODEL_SET_FLAG_20,
            SET_FLAG_BIT_10,
            QUEUED_MODEL_RUN,
            QUEUED_MODEL_FINISH,
            QUEUED_MODEL_DESTRUCT,
        ] {
            e.register(address, |_, _| Ret::default());
        }
        assert_eq!(
            e.call(
                0x0044_7080,
                &args![this, 0x400u32, 3u32, 1u8, 0u32, 1u8, 0u8]
            )
            .u32(),
            0
        );
    }

    #[test]
    fn fn_00447190_sets_bit_10_of_the_flags() {
        let mut e = engine();
        e.register(SET_FLAG_BIT_10, |_, _| Ret::default());
        log(&mut e);
        e.call(0x0044_7190, &args![0x1000u32, 1u8]);
        assert_eq!(calls_to(&e, SET_FLAG_BIT_10), vec![vec![1, 0x1000 + 0x3c]]);
    }

    #[test]
    fn load_kf_builds_a_queued_kf_when_the_map_has_none() {
        let mut e = engine();
        let map = lookup_map(&mut e, 0x0300_0000, 0);
        let this = e.mem.alloc(0x30);
        e.mem.set_u32(this + 4, map);
        e.register(QUEUED_KF_CONSTRUCT, |_, _| Ret::default());
        e.register(QUEUED_KF_RUN, |_, _| Ret::default());
        e.register_double(QUEUED_KF_FINISH, |e, a| {
            e.mem.set_u32(a[0] + 0x30, 0x7000);
            Ret::default()
        });
        e.register(QUEUED_KF_DESTRUCT, |_, _| Ret::default());
        e.register(KF_MODEL_ADD_MANUAL_REFERENCE, |_, _| Ret::default());
        log(&mut e);
        assert_eq!(e.call(0x0044_71c0, &args![this, 0x400u32]).u32(), 0x7000);
        let construct = calls_to(&e, QUEUED_KF_CONSTRUCT);
        assert_eq!(construct[0][1..], [0x400, 0]);
        assert_eq!(
            calls_to(&e, QUEUED_KF_DESTRUCT),
            vec![vec![construct[0][0]]]
        );
        assert_eq!(
            calls_to(&e, KF_MODEL_ADD_MANUAL_REFERENCE),
            vec![vec![0x7000]]
        );
    }

    #[test]
    fn load_kf_logs_a_known_model_with_a_zero_reference_total() {
        let mut e = engine();
        let map = lookup_map(&mut e, 0x0300_0000, 0x7000);
        let this = e.mem.alloc(0x30);
        e.mem.set_u32(this + 4, map);
        constant(&mut e, KF_MODEL_REFERENCE_TOTAL, 0);
        e.register(LOG, |_, _| Ret::default());
        e.register(KF_MODEL_ADD_MANUAL_REFERENCE, |_, _| Ret::default());
        log(&mut e);
        assert_eq!(e.call(0x0044_71c0, &args![this, 0x400u32]).u32(), 0x7000);
        assert_eq!(
            calls_to(&e, LOG),
            vec![vec![LOAD_KF_ZERO_REFERENCE_MESSAGE, 0x400]]
        );
        assert!(calls_to(&e, QUEUED_KF_CONSTRUCT).is_empty());
    }

    #[test]
    fn load_kf_returns_zero_without_a_model() {
        let mut e = engine();
        let map = lookup_map(&mut e, 0x0300_0000, 0);
        let this = e.mem.alloc(0x30);
        e.mem.set_u32(this + 4, map);
        for address in [
            QUEUED_KF_CONSTRUCT,
            QUEUED_KF_RUN,
            QUEUED_KF_FINISH,
            QUEUED_KF_DESTRUCT,
        ] {
            e.register(address, |_, _| Ret::default());
        }
        log(&mut e);
        assert_eq!(e.call(0x0044_71c0, &args![this, 0x400u32]).u32(), 0);
        assert!(calls_to(&e, KF_MODEL_ADD_MANUAL_REFERENCE).is_empty());
    }

    #[test]
    fn find_model_stores_the_model_when_found() {
        let mut e = engine();
        let this = e.mem.alloc(0x30);
        let out = e.mem.alloc(8);
        e.mem.set_u32(out, 0x1111);
        let map = lookup_map(&mut e, 0x0300_0000, 0x5000);
        e.mem.set_u32(this, map);
        assert!(e.call(0x0044_72a0, &args![this, 0x400u32, out]).bool());
        assert_eq!(e.mem.u32(out), 0x5000);
        let map = lookup_map(&mut e, 0x0301_0000, 0);
        e.mem.set_u32(this, map);
        assert!(!e.call(0x0044_72a0, &args![this, 0x400u32, out]).bool());
        assert_eq!(e.mem.u32(out), 0);
    }

    #[test]
    fn build_file_list_forwards_to_the_builder() {
        let mut e = engine();
        e.register(BUILD_FILE_LIST, |_, _| 0x8000u32.into_ret());
        log(&mut e);
        assert_eq!(
            e.call(0x0044_7300, &args![0x100u32, 0x200u32, 0x300u32, 0u32])
                .u32(),
            0x8000
        );
        assert_eq!(
            calls_to(&e, BUILD_FILE_LIST),
            vec![vec![0x200, 0x300, 1, 0]]
        );
    }
}
