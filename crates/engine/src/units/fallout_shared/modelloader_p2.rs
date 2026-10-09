//! `fallout shared/modelloader.cpp` (Xbox PDB source unit), part 2: its functions from `0043fed0` up to
//! (not including) `004457d0` in FalloutNV.exe 1.4.0.525, translated
//! (docs/ENGINE_CRATE.md). The unit's shared layouts and helpers are in
//! [`super::modelloader`]; anything public there may be used here.
//!
//! State of this file: the first 40 functions of its range, `0043fed0` to
//! `00441440`. They are the reference tasks of the loader: `QueuedReference`
//! (`QueueMe`, `QueueModels`, `UseDistant3D`, `AttachDistant3D`,
//! `CheckFinished`, `Cancel`, `BackgroundClone`, `Attach`, `GetDescription`),
//! the `AttachDistant3DTask` and `IOTask` helpers they use, the counters of
//! the parent cell, and the subclasses `QueuedTree`, `QueuedActor` and
//! `QueuedCharacter` (constructors, destructors, `QueueModels`, `QueueMe`,
//! `UseDistant3D`). The next function of the range is `00441560`
//! (`QueuedCharacter::QueueModels`).
//!
//! Not translated: the compiler's exception-unwinding frames (the `FS:[0]`
//! chains and state variables) and the stack-cookie check of `00440190`.
//! The memory-context guard the game keeps on its stack (`00404eb0` /
//! `00404ee0`) and the other locals it passes by address are stack blocks of
//! the engine here.
//!
//! Stack arguments: as in the main file, several functions push an argument
//! for a `thiscall` helper before the `this` of an earlier, argument-less
//! getter is loaded; the translations follow the disassembly.

#[allow(unused_imports)]
use super::modelloader::*;
#[allow(unused_imports)]
use crate::prelude::*;
use crate::types::TESForm;
use crate::units::fallout_shared::tesobjectcell::TESObjectCELL;

/// `MemoryManager` allocation, `__cdecl(size) -> block` (`00401000`), and
/// deallocation, `__cdecl(block)` (`00401030`).
const MEMORY_ALLOC: u32 = 0x0040_1000;
const MEMORY_FREE: u32 = 0x0040_1030;
/// The memory-context guard of the game's stack frame: constructor
/// `__thiscall(guard, context, 1, source file name, source line)`
/// (`00404eb0`) and destructor (`00404ee0`).
const MEMORY_CONTEXT_ENTER: u32 = 0x0040_4eb0;
const MEMORY_CONTEXT_LEAVE: u32 = 0x0040_4ee0;
/// The source file name the guard is given (`"D:\_Fallout3\Platforms\Common\
/// Code\Fallout Shared\ModelLoader.cpp"`).
const MODEL_LOADER_SOURCE: u32 = 0x0101_6840;
/// `NiPointer<T>` functions: `operator T*` (`00559450`),
/// `operator=(T*)` (`0066b0d0`), `NiPointer(T*)` (`00633c90`) and
/// `~NiPointer` (`0045cec0`).
const NI_POINTER_GET: u32 = 0x0055_9450;
const NI_POINTER_ASSIGN: u32 = 0x0066_b0d0;
const NI_POINTER_CONSTRUCT: u32 = 0x0063_3c90;
const NI_POINTER_DESTRUCT: u32 = 0x0045_cec0;
/// The task pointer functions of this unit: `NiPointer(task)` (`00528cb0`),
/// `~NiPointer` (`0044cbf0`) and `operator=(task)` (`006f74f0`, named
/// `NiPointer<QueuedFile>::operator=` in the engine map).
const TASK_POINTER_CONSTRUCT: u32 = 0x0052_8cb0;
const TASK_POINTER_DESTRUCT: u32 = 0x0044_cbf0;
const TASK_POINTER_ASSIGN: u32 = 0x006f_74f0;
/// `NiPointer<Model>::operator=(Model*)` (`0044aed0`) and the `NiPointer<
/// Model>` destructor (`0040c110`).
const MODEL_POINTER_ASSIGN: u32 = 0x0044_aed0;
const MODEL_POINTER_RELEASE: u32 = 0x0040_c110;
/// `InterlockedDecrement` wrapper, `__cdecl(address)` (`004019a0`).
const INTERLOCKED_DECREMENT: u32 = 0x0040_19a0;
/// `sprintf_s`-style formatter, `__cdecl(buffer, size, format, ...)`
/// (`00406d00`).
const FORMAT_STRING: u32 = 0x0040_6d00;

/// The global holding the `ModelLoader` (`011c3b3c`).
const MODEL_LOADER: u32 = 0x011c_3b3c;
/// The global holding the `TES` object (`011dea10`).
const TES_GLOBAL: u32 = 0x011d_ea10;
/// The global holding the reference the loader treats specially
/// (`011dea3c`; `QueuedReference::CheckFinished` and `QueuedActor::QueueMe`
/// compare the task's reference with it).
const SPECIAL_REFERENCE: u32 = 0x011d_ea3c;
/// The global holding the object whose dword at +0x10 is the id of the
/// thread that may attach 3D directly (`011dea0c`; getter `0044edb0`).
const THREAD_OWNER_OBJECT: u32 = 0x011d_ea0c;
/// The global holding the IO manager (`01202d98`).
const TASK_QUEUE: u32 = 0x0120_2d98;
/// The table of 12-byte entries per `TESForm` type whose first dword is the
/// type's name text (`01187004`, used by `00440e30`).
const FORM_TYPE_TABLE: u32 = 0x0118_7004;
/// The thread-local byte at +0x25C that `00441290` swaps (the same byte
/// `0043c130` reads).
const TLS_QUEUED_FLAG: u32 = 0x25c;

/// `"Queued ref '%s' (%08X) of type %s"`: the description format of
/// `QueuedReference::GetDescription`.
const QUEUED_REF_FORMAT: u32 = 0x0101_6c30;
/// The virtual tables of `AttachDistant3DTask` (`01016bec`), `IOTask`
/// (`01016c10`), `QueuedTree` (`01016c5c`), `QueuedActor` (`01016ca4`) and
/// `QueuedCharacter` (`01016cec`).
const ATTACH_DISTANT_3D_TASK_VTABLE: u32 = 0x0101_6bec;
const IO_TASK_VTABLE: u32 = 0x0101_6c10;
const QUEUED_TREE_VTABLE: u32 = 0x0101_6c5c;
const QUEUED_ACTOR_VTABLE: u32 = 0x0101_6ca4;
const QUEUED_CHARACTER_VTABLE: u32 = 0x0101_6cec;

/// Source lines the memory-context guards of the functions are given.
const QUEUE_ME_SOURCE_LINE: u32 = 0x72a;
const QUEUE_MODELS_SOURCE_LINE: u32 = 0x752;
const BACKGROUND_CLONE_SOURCE_LINE: u32 = 0x7ea;
const ATTACH_SOURCE_LINE: u32 = 0x804;
const TREE_QUEUE_MODELS_SOURCE_LINE: u32 = 0x847;
const ACTOR_QUEUE_MODELS_SOURCE_LINE: u32 = 0x8a2;

/// `TESObjectREFR` methods: `GetTESModel` (Xbox PDB, `00571630`), the
/// address of the embedded extra data list (`005d43c0`, `this + 0x44`),
/// `00570f70(node)` (gives the reference its 3D), the parent cell getter
/// (`008d6f30`), `Set3DVerySimple` (Xbox PDB, `00571080`), `Is3DCritical`
/// (Xbox PDB, `00572350`), `TESActorBase::SetStartsDead` (Xbox PDB,
/// `00565210`), `GetOrientation(out)` (Xbox PDB, `0056fa00`, returns the
/// matrix it wrote) and `GetScale` (Xbox PDB, `00567400`, a `float` in ST0).
const REFERENCE_GET_TES_MODEL: u32 = 0x0057_1630;
const REFERENCE_EXTRA_LIST: u32 = 0x005d_43c0;
const REFERENCE_SET_3D: u32 = 0x0057_0f70;
const REFERENCE_CELL: u32 = 0x008d_6f30;
const REFERENCE_SET_3D_VERY_SIMPLE: u32 = 0x0057_1080;
const REFERENCE_IS_3D_CRITICAL: u32 = 0x0057_2350;
const SET_STARTS_DEAD: u32 = 0x0056_5210;
const REFERENCE_GET_ORIENTATION: u32 = 0x0056_fa00;
const REFERENCE_GET_SCALE: u32 = 0x0056_7400;
/// Two predicates of `tesobjectrefr.cpp`'s range on a reference
/// (`00564e60`, `00564f00`; no names in the engine map). The first is true
/// when bit `0x8000` of the form flags is set, or when the base form passes
/// `00564e40`.
const REFERENCE_TEST_00564E60: u32 = 0x0056_4e60;
const REFERENCE_TEST_00564F00: u32 = 0x0056_4f00;
/// `BGSSaveFormBuffer::GetForm` (the engine map's name for the getter of the
/// dword at +0x20; of a reference it gives the base form).
const GET_BASE_FORM: u32 = 0x007a_f430;
/// The getter of the dword at +0xC of a form, its form ID (`0084e3a0`).
const GET_FORM_ID: u32 = 0x0084_e3a0;
/// The form type byte of a form (`00401170`, `this + 4` zero-extended).
const FORM_TYPE: u32 = 0x0040_1170;
/// `ExtraDataList::GetDecalRefs` (Xbox PDB, `0041f050`) and
/// `ExtraDataList::GetDismembermentExtra` (Xbox PDB, `0042e8c0`).
const EXTRA_GET_DECAL_REFS: u32 = 0x0041_f050;
const EXTRA_GET_DISMEMBERMENT: u32 = 0x0042_e8c0;
/// Singly linked list node helpers: `IsEmpty` (`008256d0`: no item and no
/// next node), the address of the item (`006815c0`, the node itself) and the
/// next node (`00726070`, `node + 4`).
const LIST_NODE_IS_EMPTY: u32 = 0x0082_56d0;
const LIST_NODE_ITEM_ADDRESS: u32 = 0x0068_15c0;
const LIST_NODE_NEXT: u32 = 0x0072_6070;
/// The element count of a `BSSimpleArray` (the dword at +8, `0044ddc0`) and
/// the element address getter (`006a7ad0`, `__thiscall(index)`).
const ARRAY_COUNT_AT_8: u32 = 0x0044_ddc0;
const ARRAY_ELEMENT_ADDRESS: u32 = 0x006a_7ad0;
/// `BGSTextureSet::QueueTextureSet` (Xbox PDB, `005930e0`),
/// `__thiscall(priority, parent task, 0)`.
const QUEUE_TEXTURE_SET: u32 = 0x0059_30e0;
/// `QueuedFile::Cancel` (Xbox PDB, `00c3cb70`), `__thiscall(2 arguments)`.
const QUEUED_FILE_CANCEL: u32 = 0x00c3_cb70;
/// `IOManager::RequeueTask` (Xbox PDB, `00c3df40`), `__thiscall(task, 1
/// argument)`.
const IO_MANAGER_REQUEUE_TASK: u32 = 0x00c3_df40;
/// `BSTask` state helpers: set the state to 5 (`00449150`), state at least 4
/// (`00449110`), state is 6 (`00449130`), state is 0 (`0069b080`) and set the
/// state to 6 (`00449170`).
const TASK_SET_STATE_5: u32 = 0x0044_9150;
const TASK_STATE_AT_LEAST_4: u32 = 0x0044_9110;
const TASK_STATE_IS_6: u32 = 0x0044_9130;
const TASK_STATE_IS_0: u32 = 0x0069_b080;
const TASK_SET_STATE_6: u32 = 0x0044_9170;
/// The `IOTask` base constructor (`00449090`) and destructor (`004490e0`).
const IO_TASK_BASE_CONSTRUCT: u32 = 0x0044_9090;
const IO_TASK_BASE_DESTRUCT: u32 = 0x0044_90e0;
/// The `ModelLoader`'s queue of finished tasks: `__thiscall(task pointer)` on
/// the object at `loader + 0x1c` (`00449240`).
const FINISHED_QUEUE_ADD: u32 = 0x0044_9240;
/// `ModelLoader` methods used by the reference tasks, `__thiscall` on the
/// loader: `FindModel(name, NiPointer<Model> out)` (`004472a0`), the
/// reference-model queueing `00444dc0(form, NiPointer<QueuedModel> slot,
/// priority, parent task, reference)`, `00444540(reference, form, priority,
/// parent task, LOD multiplier)`, `00444d40(value, priority, parent task,
/// reference)` and `LoadAddonNodes(reference, node)` (`004489b0`).
const MODEL_LOADER_FIND_MODEL: u32 = 0x0044_72a0;
const MODEL_LOADER_QUEUE_REFERENCE_MODEL: u32 = 0x0044_4dc0;
const MODEL_LOADER_QUEUE_TREE_MODEL: u32 = 0x0044_4540;
const MODEL_LOADER_QUEUE_DISMEMBER_PART: u32 = 0x0044_4d40;
const MODEL_LOADER_LOAD_ADDON_NODES: u32 = 0x0044_89b0;
/// `ModelLoader` method `00442580(task)`, called on the object at
/// `loader + 0x28`.
const BACKGROUND_CLONE_QUEUE_ADD: u32 = 0x0044_2580;
/// `QueuedModel` getter of the `NiPointer<Model>` at +0x30 (`004a8a90`).
const QUEUED_MODEL_GET_MODEL: u32 = 0x004a_8a90;
/// `TES` methods (`tes.cpp`): `IsCellLoaded(cell, flag)` (`004511e0`),
/// `00451e40(reference)` and the reference loader `00451ef0(reference, cell,
/// task or 0, 0)`.
const TES_IS_CELL_LOADED: u32 = 0x0045_11e0;
const TES_REFERENCE_PREDICATE: u32 = 0x0045_1e40;
const TES_LOAD_REFERENCE: u32 = 0x0045_1ef0;
/// `TESObjectCELL` methods (`tesobjectcell.cpp`): `00541ac0` and `00541ae0`
/// are called on the parent cell before and after the attach.
const CELL_ENTER_ATTACH: u32 = 0x0054_1ac0;
const CELL_LEAVE_ATTACH: u32 = 0x0054_1ae0;
/// `TESObjectTREE::BuildDistant3D` (Xbox PDB, `0051cba0`, `__thiscall(0)`).
const BUILD_DISTANT_3D: u32 = 0x0051_cba0;
/// `TESModel::GetDistantModelName_ov2` (Xbox PDB, `00489190`),
/// `__cdecl(buffer, form)`.
const GET_DISTANT_MODEL_NAME: u32 = 0x0048_9190;
/// `BGSDestructibleObjectForm::GetDestructionForm` (Xbox PDB, `00475400`,
/// `__cdecl(form)`) and `BGSDestructibleObjectForm::QueueFiles` (Xbox PDB,
/// `00477640`, `__thiscall(form, priority, parent task)`).
const GET_DESTRUCTION_FORM: u32 = 0x0047_5400;
const DESTRUCTIBLE_QUEUE_FILES: u32 = 0x0047_7640;
/// `NiObject::Clone_ov2` (Xbox PDB, `00a5d680`) and `BSFadeNode::
/// SetLODMultType` (Xbox PDB, `00b4dec0`).
const NI_OBJECT_CLONE: u32 = 0x00a5_d680;
const FADE_NODE_SET_LOD_MULT_TYPE: u32 = 0x00b4_dec0;
/// `TES::GetLODMult` (Xbox PDB, `0045c6b0`, `__cdecl(form)`).
const GET_LOD_MULT: u32 = 0x0045_c6b0;
/// `NiAVObject::GetProperty` (Xbox PDB, `00a59d30`, `__thiscall(type)`),
/// `NiGeometryData::SetConsistency` (Xbox PDB, `00a67050`) and
/// `BSShaderManager::PrepareObject(node, flag, flag)` (Xbox PDB, `00b57e30`,
/// `__cdecl`).
const GET_PROPERTY: u32 = 0x00a5_9d30;
const SET_CONSISTENCY: u32 = 0x00a6_7050;
const PREPARE_OBJECT: u32 = 0x00b5_7e30;
/// A method of `tesobjectcell.cpp`'s range (`005495f0`): the `NiPointer` at
/// +0xB8 of the object dereferenced.
const OBJECT_POINTER_AT_B8: u32 = 0x0054_95f0;
/// Two setters on the geometry data (`004410d0`, `004410f0`; no names in the
/// engine map, `__thiscall(one argument)`) and the type getter of a
/// property (`00441110`).
const GEOMETRY_DATA_SETTER_FIRST: u32 = 0x0044_10d0;
const GEOMETRY_DATA_SETTER_SECOND: u32 = 0x0044_10f0;
const PROPERTY_GET_TYPE: u32 = 0x0044_1110;
/// The type `00441110` gives for the property whose shader flags
/// `QueuedTree::UseDistant3D` sets (`0xb`).
const SHADER_PROPERTY_TYPE: u32 = 0xb;
/// The property type passed to `GetProperty` by `QueuedTree::UseDistant3D`.
const SHADER_PROPERTY_SLOT: u32 = 3;
/// `Actor` methods of `actor.cpp` / `actor_magic.cpp`: `008b7b70` (a
/// predicate), `008b7360(1, task)` and `008c2e50` (no result used), and
/// `00936ef0` (a getter whose result is only tested against 0).
const ACTOR_TEST_008B7B70: u32 = 0x008b_7b70;
const ACTOR_ATTACH_TASK: u32 = 0x008b_7360;
const ACTOR_PREPARE: u32 = 0x008c_2e50;
const REFERENCE_TEST_00936EF0: u32 = 0x0093_6ef0;
/// `GetCurrentThreadId` through the game's wrapper (`0040fc90`) and the
/// getter of the thread id (`0044edb0`, `this + 0x10`).
const CURRENT_THREAD_ID: u32 = 0x0040_fc90;
const OWNER_THREAD_ID: u32 = 0x0044_edb0;
/// A `float -> float` helper of `effectsetting.cpp`'s range (`00408820`).
const FLOAT_FILTER: u32 = 0x0040_8820;
/// `__thiscall()` on a reference-counted object (`00401970`): decrements the
/// count at +4 and calls the object's virtual function 1 when it reaches 0.
const OBJECT_RELEASE: u32 = 0x0040_1970;

layout! {
    /// `QueuedCharacter` (Xbox PDB), 0x48 bytes: a `QueuedActor` (the same
    /// 0x40 bytes as `QueuedReference`) and the head and helmet tasks.
    pub struct QueuedCharacter: 0x48 {
        /// `eContext` (`QueuedFile`, Xbox PDB).
        0x18 eContext: u32,
        /// `pRef` (`QueuedReference`, Xbox PDB): `TESObjectREFR*`.
        0x28 pRef: Ptr,
        /// `spQueuedHead` (Xbox PDB): `NiPointer<QueuedHead>`.
        0x40 spQueuedHead: Ptr,
        /// `spQueuedHelmet` (Xbox PDB): `NiPointer<QueuedHelmet>`.
        0x44 spQueuedHelmet: Ptr,
    }
}

/// Runs `body` inside the memory-context guard the game keeps on its stack
/// (`context`, a source line of `ModelLoader.cpp`), leaving it afterwards.
fn in_context<R>(
    e: &mut Engine,
    context: u32,
    line: u32,
    body: impl FnOnce(&mut Engine) -> R,
) -> R {
    e.with_stack(4, |e, guard| {
        e.call(
            MEMORY_CONTEXT_ENTER,
            &args![guard, context, 1u32, MODEL_LOADER_SOURCE, line],
        );
        let result = body(e);
        e.call(MEMORY_CONTEXT_LEAVE, &args![guard]);
        result
    })
}

/// The `NiPointer` stored at `slot`, dereferenced (`00559450`).
fn ni_pointer_value(e: &mut Engine, slot: Ptr) -> u32 {
    e.call(NI_POINTER_GET, &args![slot]).u32()
}

/// The priority byte of a task (`0043cc60`), as the word the game pushes.
fn task_priority(e: &mut Engine, task: Ptr) -> u32 {
    fn_0043cc60(e, task.cast()) as u32
}

// Translated from 0043fed0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedReference::QueueMe` (Xbox PDB): inside the memory context of the
/// task (source line `0x72a`), a reference that passes `00564e60` and whose
/// virtual function `0x1d0` gives 0 makes the task call its virtual function
/// `0x30` (`UseDistant3D`). Then every item of the reference's decal list
/// (`ExtraDataList::GetDecalRefs` of its extra data) whose form has type 4 is
/// queued as a texture set (`BGSTextureSet::QueueTextureSet`) with the task's
/// priority. Last the task calls its virtual function `0x2c` (`QueueModels`);
/// a task still in state 0 is then put into state 5 (`00449150`), and the
/// virtual function `0x28` (`CheckFinished`) runs.
///
/// The exception-unwinding frame is not translated.
pub fn queued_reference_queue_me(e: &mut Engine, this: Ptr<QueuedReference>) {
    let context = e.get(this, QueuedReference::eContext);
    in_context(e, context, QUEUE_ME_SOURCE_LINE, |e| {
        let reference = e.get(this, QueuedReference::pRef);
        if !reference.is_null()
            && e.call(REFERENCE_TEST_00564E60, &args![reference]).bool()
            && e.vcall(reference.addr(), 0x1d0, &args![]).u32() == 0
        {
            e.vcall(this.addr(), 0x30, &args![]);
        }
        let reference = e.get(this, QueuedReference::pRef);
        if !reference.is_null() {
            let extra = e.call(REFERENCE_EXTRA_LIST, &args![reference]).u32();
            let mut node = e.call(EXTRA_GET_DECAL_REFS, &args![extra]).u32();
            while node != 0 && !e.call(LIST_NODE_IS_EMPTY, &args![node]).bool() {
                let item_slot = e.call(LIST_NODE_ITEM_ADDRESS, &args![node]).u32();
                let item = e.mem.u32(item_slot);
                node = e.call(LIST_NODE_NEXT, &args![node]).u32();
                if e.mem.u32(item) != 0 {
                    let holder = e.mem.u32(item);
                    let form = e.call(GET_BASE_FORM, &args![holder]).u32();
                    if e.call(FORM_TYPE, &args![form]).u32() == 4 {
                        let holder = e.mem.u32(item);
                        let texture_set = e.call(GET_BASE_FORM, &args![holder]).u32();
                        let priority = fn_0043cc80(e, this.cast()) as u8 as u32;
                        e.call(QUEUE_TEXTURE_SET, &args![texture_set, priority, this, 0u32]);
                    }
                }
            }
        }
        e.vcall(this.addr(), 0x2c, &args![]);
        if e.call(TASK_STATE_IS_0, &args![this]).bool() {
            e.call(TASK_SET_STATE_5, &args![this]);
        }
        e.vcall(this.addr(), 0x28, &args![]);
    });
}

// Translated from 00440050 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedReference::QueueModels` (Xbox PDB): inside the memory context of the
/// task (source line `0x752`), a reference whose `TESModel`
/// (`TESObjectREFR::GetTESModel`) has a non-empty name (virtual function
/// `0x14`) has its model queued by the loader (`00444dc0` with the base form,
/// the `spQueuedModel` slot, the task's priority, the task and the
/// reference). When that left `spQueuedModel` empty the loader's `FindModel`
/// looks the model up by name into `spModel`. The base form's destruction
/// form (`BGSDestructibleObjectForm::GetDestructionForm`) has its files queued
/// (`QueueFiles`).
///
/// The exception-unwinding frame is not translated.
pub fn queued_reference_queue_models(e: &mut Engine, this: Ptr<QueuedReference>) {
    let context = e.get(this, QueuedReference::eContext);
    in_context(e, context, QUEUE_MODELS_SOURCE_LINE, |e| {
        let reference = e.get(this, QueuedReference::pRef);
        let form = e.call(GET_BASE_FORM, &args![reference]).u32();
        let tes_model = e.call(REFERENCE_GET_TES_MODEL, &args![reference]).u32();
        if tes_model == 0 {
            return;
        }
        let name = e.vcall(tes_model, 0x14, &args![]).u32();
        if e.mem.i8(name) == 0 {
            return;
        }
        let reference = e.get(this, QueuedReference::pRef);
        let priority = task_priority(e, this.cast());
        let loader = e.global::<u32>(MODEL_LOADER);
        e.call(
            MODEL_LOADER_QUEUE_REFERENCE_MODEL,
            &args![
                loader,
                form,
                this.byte_add(QueuedReference::spQueuedModel.off),
                priority,
                this,
                reference
            ],
        );
        if ni_pointer_value(e, this.byte_add(QueuedReference::spQueuedModel.off)) == 0 {
            let name = e.vcall(tes_model, 0x14, &args![]).u32();
            let loader = e.global::<u32>(MODEL_LOADER);
            e.call(
                MODEL_LOADER_FIND_MODEL,
                &args![loader, name, this.byte_add(QueuedReference::spModel.off)],
            );
        }
        let destruction = e.call(GET_DESTRUCTION_FORM, &args![form]).u32();
        if destruction != 0 {
            let priority = task_priority(e, this.cast());
            e.call(
                DESTRUCTIBLE_QUEUE_FILES,
                &args![destruction, form, priority, this],
            );
        }
    });
}

// Translated from 00440190 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedReference::UseDistant3D` (Xbox PDB): looks up the loader's model
/// for the base form's distant model name (`TESModel::GetDistantModelName_ov2`
/// into a 0x108-byte buffer, then `ModelLoader::FindModel`); if there is one
/// with a root node, the node is cloned (`NiObject::Clone_ov2`). When the
/// clone's virtual function `0xc` gives a node (the fade node), and its
/// virtual function `0x10` is non-zero, its LOD multiplier type is set to 6;
/// a clone without such a node is only given a counted reference and let go.
/// A fade node is handed to the task's virtual function `0x34`
/// (`AttachDistant3D`). The model pointer is released at the end.
///
/// The exception-unwinding frame and the stack-cookie check are not
/// translated.
pub fn queued_reference_use_distant_3d(e: &mut Engine, this: Ptr<QueuedReference>) {
    let reference = e.get(this, QueuedReference::pRef);
    let form = e.call(GET_BASE_FORM, &args![reference]).u32();
    e.with_stack(0x108, |e, name| {
        e.call(GET_DISTANT_MODEL_NAME, &args![name, form]);
        // The `NiPointer<Model>` of the game's stack frame.
        e.with_stack(4, |e, model| {
            e.call(NI_POINTER_CONSTRUCT, &args![model, 0u32]);
            let loader = e.global::<u32>(MODEL_LOADER);
            e.call(MODEL_LOADER_FIND_MODEL, &args![loader, name, model]);
            let mut fade_node = 0;
            let found = ni_pointer_value(e, model);
            if found != 0 {
                let root = fn_0043b230(e, Ptr::new(found));
                if !root.is_null() {
                    let clone = e.call(NI_OBJECT_CLONE, &args![root]).u32();
                    if clone != 0 {
                        fade_node = e.vcall(clone, 0xc, &args![]).u32();
                        if fade_node == 0 {
                            // A temporary `NiPointer` takes the clone and
                            // releases it again.
                            e.with_stack(4, |e, temporary| {
                                e.call(NI_POINTER_CONSTRUCT, &args![temporary, clone]);
                                e.call(NI_POINTER_DESTRUCT, &args![temporary]);
                            });
                        } else if e.vcall(fade_node, 0x10, &args![]).u32() != 0 {
                            e.call(FADE_NODE_SET_LOD_MULT_TYPE, &args![fade_node, 6u32]);
                        }
                    }
                }
            }
            if fade_node != 0 {
                e.vcall(this.addr(), 0x34, &args![fade_node]);
            }
            e.call(MODEL_POINTER_RELEASE, &args![model]);
        });
    });
}

// Translated from 00440310 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedReference::AttachDistant3D` (Xbox PDB): gives the node the
/// reference's position (virtual function `0x1f4` of the reference, stored
/// by [`fn_00440460`]), orientation (`TESObjectREFR::GetOrientation` into a
/// 12-byte buffer, copied by `0043fa80`) and scale (`GetScale`, filtered by
/// [`fn_00440490`]). When the current thread is the one recorded at +0x10 of
/// the object at global `011dea0c`, the node is attached at once: the
/// reference gets it as its 3D (`00570f70`), `TES` loads the reference
/// (`00451ef0` with its parent cell) and `TESActorBase::SetStartsDead(1)`
/// is called. Otherwise a new 0x20-byte `AttachDistant3DTask`
/// ([`fn_004404c0`]) for the reference and node is stored in
/// `spAttachDistant3DTask` and handed to the loader ([`fn_00440710`]).
///
/// The exception-unwinding frame is not translated.
pub fn queued_reference_attach_distant_3d(e: &mut Engine, this: Ptr<QueuedReference>, node: Ptr) {
    let reference = e.get(this, QueuedReference::pRef);
    let position = e.vcall(reference.addr(), 0x1f4, &args![]).u32();
    fn_00440460(e, node, Ptr::new(position));
    // The 12-byte orientation buffer of the game's stack frame.
    e.with_stack(12, |e, buffer| {
        let orientation = e
            .call(REFERENCE_GET_ORIENTATION, &args![reference, buffer])
            .u32();
        fn_0043fa80(e, node, Ptr::new(orientation));
    });
    let scale = e.call(REFERENCE_GET_SCALE, &args![reference]).f32();
    fn_00440490(e, node, scale);
    let owner = e.global::<u32>(THREAD_OWNER_OBJECT);
    let owner_thread = e.call(OWNER_THREAD_ID, &args![owner]).u32();
    let current_thread = e.call(CURRENT_THREAD_ID, &args![]).u32();
    if owner_thread == current_thread {
        e.call(REFERENCE_SET_3D, &args![reference, node]);
        let cell = e.call(REFERENCE_CELL, &args![reference]).u32();
        let tes = e.global::<u32>(TES_GLOBAL);
        e.call(TES_LOAD_REFERENCE, &args![tes, reference, cell, 0u32, 0u32]);
        e.call(SET_STARTS_DEAD, &args![reference, 1u32]);
    } else {
        let block = e.call(MEMORY_ALLOC, &args![0x20u32]).u32();
        let task = if block != 0 {
            fn_004404c0(e, Ptr::new(block), reference, node).addr()
        } else {
            0
        };
        let slot = this.byte_add(QueuedReference::spAttachDistant3DTask.off);
        e.call(TASK_POINTER_ASSIGN, &args![slot, task]);
        let task = ni_pointer_value(e, slot);
        let loader = e.global::<u32>(MODEL_LOADER);
        fn_00440710(e, Ptr::new(loader), Ptr::new(task));
    }
}

// Translated from 00440460 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiAVObject` method (no name in the engine map): copies the three words
/// at `source` (a position) into the local translation at `this + 0x58`.
pub fn fn_00440460(e: &mut Engine, this: Ptr, source: Ptr) {
    for i in 0..3 {
        let word = e.mem.u32(source.addr() + 4 * i);
        e.mem.set_u32(this.addr() + 0x58 + 4 * i, word);
    }
}

// Translated from 00440490 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiAVObject` method (no name in the engine map): stores `scale` passed
/// through the `float` helper `00408820` into the local scale at
/// `this + 0x64`.
pub fn fn_00440490(e: &mut Engine, this: Ptr, scale: f32) {
    let filtered = e.call(FLOAT_FILTER, &args![scale]).f32();
    e.mem.set_f32(this.addr() + 0x64, filtered);
}

// Translated from 004404c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `AttachDistant3DTask` constructor (no name in the engine map): the
/// `IOTask` constructor with priority 0 ([`fn_00440540`]), the task's virtual
/// table, `pRef`, and `spNode` set to `node`. Returns `this`.
///
/// The exception-unwinding frame is not translated.
pub fn fn_004404c0(
    e: &mut Engine,
    this: Ptr<AttachDistant3DTask>,
    reference: Ptr,
    node: Ptr,
) -> Ptr<AttachDistant3DTask> {
    fn_00440540(e, this.cast(), 0);
    e.mem.set_u32(this.addr(), ATTACH_DISTANT_3D_TASK_VTABLE);
    e.set(this, AttachDistant3DTask::pRef, reference);
    e.call(
        NI_POINTER_CONSTRUCT,
        &args![this.byte_add(AttachDistant3DTask::spNode.off), node],
    );
    this
}

// Translated from 00440540 (decompiled, FalloutNV.exe 1.4.0.525)
/// `IOTask` constructor taking a priority (no name in the engine map): the
/// base constructor (`00449090`), the `IOTask` virtual table, then
/// [`fn_004405b0`] with `priority`. Returns `this`.
///
/// The exception-unwinding frame is not translated.
pub fn fn_00440540(e: &mut Engine, this: Ptr<IOTask>, priority: u32) -> Ptr<IOTask> {
    e.call(IO_TASK_BASE_CONSTRUCT, &args![this]);
    e.mem.set_u32(this.addr(), IO_TASK_VTABLE);
    fn_004405b0(e, this, priority as u8);
    this
}

// Translated from 004405b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `IOTask` method (no name in the engine map): [`fn_004405d0`] with the
/// priority byte.
pub fn fn_004405b0(e: &mut Engine, this: Ptr<IOTask>, priority: u8) {
    fn_004405d0(e, this, priority);
}

// Translated from 004405d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `IOTask` method (no name in the engine map): puts `priority` into bits 16
/// to 23 of the task's 64-bit `Key`: the key with the bits `0x00ff0000` of
/// its low dword cleared, plus `priority << 16` (a 64-bit add).
pub fn fn_004405d0(e: &mut Engine, this: Ptr<IOTask>, priority: u8) {
    let key = e.get(this, IOTask::Key);
    let cleared = key & 0xffff_ffff_ff00_ffff;
    e.set(
        this,
        IOTask::Key,
        cleared.wrapping_add((priority as u64) << 16),
    );
}

// Translated from 00440610 (decompiled, FalloutNV.exe 1.4.0.525)
/// `IOTask` scalar deleting destructor (no name in the engine map): runs the
/// destructor ([`fn_00440640`]) and, when bit 0 of `flags` is set, frees the
/// object. Returns `this`.
pub fn fn_00440610(e: &mut Engine, this: Ptr<IOTask>, flags: u32) -> Ptr<IOTask> {
    fn_00440640(e, this);
    if flags & 1 != 0 {
        e.call(MEMORY_FREE, &args![this]);
    }
    this
}

// Translated from 00440640 (decompiled, FalloutNV.exe 1.4.0.525)
/// `IOTask` destructor (no name in the engine map): the base destructor
/// (`004490e0`).
pub fn fn_00440640(e: &mut Engine, this: Ptr<IOTask>) {
    e.call(IO_TASK_BASE_DESTRUCT, &args![this]);
}

// Translated from 00440660 (decompiled, FalloutNV.exe 1.4.0.525)
/// `IOTask::Requeue` (Xbox PDB): `IOManager::RequeueTask` on the IO manager
/// (the global at `01202d98`) with the task and `requeue_argument`.
pub fn io_task_requeue(e: &mut Engine, this: Ptr<IOTask>, requeue_argument: u32) {
    let manager = e.global::<u32>(TASK_QUEUE);
    e.call(
        IO_MANAGER_REQUEUE_TASK,
        &args![manager, this, requeue_argument],
    );
}

// Translated from 00440680 (decompiled, FalloutNV.exe 1.4.0.525)
/// `AttachDistant3DTask::_scalar_deleting_destructor_` (Xbox PDB): runs the
/// destructor ([`fn_004406b0`]) and, when bit 0 of `flags` is set, frees the
/// object. Returns `this`.
pub fn attach_distant_3d_task_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr<AttachDistant3DTask>,
    flags: u32,
) -> Ptr<AttachDistant3DTask> {
    fn_004406b0(e, this);
    if flags & 1 != 0 {
        e.call(MEMORY_FREE, &args![this]);
    }
    this
}

// Translated from 004406b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `AttachDistant3DTask` destructor (no name in the engine map): releases
/// `spNode` and runs the `IOTask` destructor ([`fn_00440640`]). It does not
/// restore the virtual table.
///
/// The exception-unwinding frame is not translated.
pub fn fn_004406b0(e: &mut Engine, this: Ptr<AttachDistant3DTask>) {
    e.call(
        NI_POINTER_DESTRUCT,
        &args![this.byte_add(AttachDistant3DTask::spNode.off)],
    );
    fn_00440640(e, this.cast());
}

// Translated from 00440710 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ModelLoader` method (no name in the engine map) on the loader at
/// `this`: puts `task` into a task pointer of the game's stack frame and
/// hands it to the object at `this + 0x1c` (`00449240`).
///
/// The exception-unwinding frame is not translated.
pub fn fn_00440710(e: &mut Engine, this: Ptr, task: Ptr) {
    e.with_stack(4, |e, holder| {
        e.call(TASK_POINTER_CONSTRUCT, &args![holder, task]);
        let queue = e.mem.u32(this.addr() + 0x1c);
        e.call(FINISHED_QUEUE_ADD, &args![queue, holder]);
        e.call(TASK_POINTER_DESTRUCT, &args![holder]);
    });
}

// Translated from 00440780 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedReference::CheckFinished` (Xbox PDB): once the task's state is 4 or
/// more but not 6 and its children are all finished ([`fn_0043caa0`]): a task
/// that has a queued model but no model takes the queued model's
/// (`004a8a90`). It is then attached by the loader (`00442580` through
/// [`fn_004408d0`]) when it has a model, its reference is not the one at
/// global `011dea3c`, the reference's parent cell is loaded (`TES::
/// IsCellLoaded`), `00451e40` accepts the reference and [`fn_0043fcd0`] finds
/// none; except that a reference whose virtual functions `0x224` or `0x220`
/// answer yes is never attached this way. All other tasks go to the
/// post-process queue ([`iomanager_add_post_process_task`]).
pub fn queued_reference_check_finished(e: &mut Engine, this: Ptr<QueuedReference>) {
    if !(e.call(TASK_STATE_AT_LEAST_4, &args![this]).bool()
        && fn_0043caa0(e, this.cast())
        && !e.call(TASK_STATE_IS_6, &args![this]).bool())
    {
        return;
    }
    let queued_slot = this.byte_add(QueuedReference::spQueuedModel.off);
    let model_slot = this.byte_add(QueuedReference::spModel.off);
    if ni_pointer_value(e, queued_slot) != 0 && ni_pointer_value(e, model_slot) == 0 {
        let queued = ni_pointer_value(e, queued_slot);
        let model = e.call(QUEUED_MODEL_GET_MODEL, &args![queued]).u32();
        e.call(MODEL_POINTER_ASSIGN, &args![model_slot, model]);
    }
    let mut attach = false;
    let reference = e.get(this, QueuedReference::pRef);
    if ni_pointer_value(e, model_slot) != 0
        && reference.addr() != e.global::<u32>(SPECIAL_REFERENCE)
    {
        let cell = e.call(REFERENCE_CELL, &args![reference]).u32();
        let tes = e.global::<u32>(TES_GLOBAL);
        if e.call(TES_IS_CELL_LOADED, &args![tes, cell, 0u32]).bool()
            && e.call(TES_REFERENCE_PREDICATE, &args![tes, reference])
                .bool()
            && fn_0043fcd0(e, reference).is_null()
        {
            attach = true;
        }
    }
    if e.vcall(reference.addr(), 0x224, &args![]).bool()
        || e.vcall(reference.addr(), 0x220, &args![]).bool()
    {
        attach = false;
    }
    if attach {
        let loader = e.global::<u32>(MODEL_LOADER);
        fn_004408d0(e, Ptr::new(loader), this.cast());
    } else {
        iomanager_add_post_process_task(e, this.cast());
    }
}

// Translated from 004408d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ModelLoader` method (no name in the engine map) on the loader at `this`:
/// `00442580(task)` on the object at `this + 0x28`.
pub fn fn_004408d0(e: &mut Engine, this: Ptr, task: Ptr) {
    let queue = e.mem.u32(this.addr() + 0x28);
    e.call(BACKGROUND_CLONE_QUEUE_ADD, &args![queue, task]);
}

// Translated from 004408f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedReference::Cancel` (Xbox PDB): a pending attach task is set to
/// state 6 (`00449170`), `QueuedFile::Cancel` runs with the two arguments,
/// and a cloned 3D is taken off the reference (`Set3DVerySimple`, the
/// `NiPointer` cleared, the reference's virtual function `0x1cc` called with
/// two zeros). The loader's object at `loader + 8` is told about the
/// reference ([`fn_004409e0`]); the parent cell's queued-reference count
/// is lowered ([`fn_00440a10`]), and, for a reference whose 3D is critical,
/// its critical count too ([`fn_00440a50`]).
pub fn queued_reference_cancel(
    e: &mut Engine,
    this: Ptr<QueuedReference>,
    first: u32,
    second: u32,
) {
    let attach_slot = this.byte_add(QueuedReference::spAttachDistant3DTask.off);
    let attach_task = ni_pointer_value(e, attach_slot);
    if attach_task != 0 {
        let attach_task = ni_pointer_value(e, attach_slot);
        e.call(TASK_SET_STATE_6, &args![attach_task]);
    }
    e.call(QUEUED_FILE_CANCEL, &args![this, first, second]);
    let cloned_slot = this.byte_add(QueuedReference::spCloned3D.off);
    let reference = e.get(this, QueuedReference::pRef);
    if ni_pointer_value(e, cloned_slot) != 0 {
        let cloned = ni_pointer_value(e, cloned_slot);
        e.call(REFERENCE_SET_3D_VERY_SIMPLE, &args![reference, cloned]);
        e.call(NI_POINTER_ASSIGN, &args![cloned_slot, 0u32]);
        e.vcall(reference.addr(), 0x1cc, &args![0u32, 0u32]);
    }
    let loader = e.global::<u32>(MODEL_LOADER);
    fn_004409e0(e, Ptr::new(loader), reference);
    let cell = e.call(REFERENCE_CELL, &args![reference]).u32();
    if cell != 0 {
        let cell = e.call(REFERENCE_CELL, &args![reference]).u32();
        fn_00440a10(e, Ptr::new(cell));
        if e.call(REFERENCE_IS_3D_CRITICAL, &args![reference]).bool() {
            let cell = e.call(REFERENCE_CELL, &args![reference]).u32();
            fn_00440a50(e, Ptr::new(cell));
        }
    }
}

// Translated from 004409e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ModelLoader` method (no name in the engine map) on the loader at `this`:
/// calls the virtual function `0x14` of the object at `this + 8` with
/// `reference`. What that slot does is not confirmed; the reference tasks
/// call it when they are over.
pub fn fn_004409e0(e: &mut Engine, this: Ptr, reference: Ptr) {
    let object = e.mem.u32(this.addr() + 8);
    e.vcall(object, 0x14, &args![reference]);
}

// Translated from 00440a10 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL` method (no name in the engine map): lowers
/// `iQueuedRefCount` (Xbox PDB, an interlocked decrement) and keeps it from
/// going below 0.
pub fn fn_00440a10(e: &mut Engine, this: Ptr<TESObjectCELL>) {
    e.call(
        INTERLOCKED_DECREMENT,
        &args![this.byte_add(TESObjectCELL::iQueuedRefCount.off)],
    );
    if e.get(this, TESObjectCELL::iQueuedRefCount) < 0 {
        e.set(this, TESObjectCELL::iQueuedRefCount, 0);
    }
}

// Translated from 00440a50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL` method (no name in the engine map): lowers
/// `iCriticalQueuedRefCount` (Xbox PDB, an interlocked decrement) and keeps
/// it from going below 0.
pub fn fn_00440a50(e: &mut Engine, this: Ptr<TESObjectCELL>) {
    e.call(
        INTERLOCKED_DECREMENT,
        &args![this.byte_add(TESObjectCELL::iCriticalQueuedRefCount.off)],
    );
    if e.get(this, TESObjectCELL::iCriticalQueuedRefCount) < 0 {
        e.set(this, TESObjectCELL::iCriticalQueuedRefCount, 0);
    }
}

// Translated from 00440a90 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedReference::BackgroundClone` (Xbox PDB): inside the memory context
/// of the task (source line `0x7ea`), when the reference's parent cell is
/// loaded (`TES::IsCellLoaded`) and `00451e40` accepts the reference and
/// [`fn_0043fcd0`] finds no object for it, the reference's virtual function
/// `0x1c8` (called with 1) gives the clone, which is stored in `spCloned3D`
/// and then released once (`00401970`, balancing the reference the virtual
/// function returned). Returns whether the cell was loaded.
///
/// The exception-unwinding frame is not translated.
pub fn queued_reference_background_clone(e: &mut Engine, this: Ptr<QueuedReference>) -> bool {
    let context = e.get(this, QueuedReference::eContext);
    in_context(e, context, BACKGROUND_CLONE_SOURCE_LINE, |e| {
        let reference = e.get(this, QueuedReference::pRef);
        let cell = e.call(REFERENCE_CELL, &args![reference]).u32();
        let tes = e.global::<u32>(TES_GLOBAL);
        if !e.call(TES_IS_CELL_LOADED, &args![tes, cell, 0u32]).bool() {
            return false;
        }
        let tes = e.global::<u32>(TES_GLOBAL);
        if e.call(TES_REFERENCE_PREDICATE, &args![tes, reference])
            .bool()
            && fn_0043fcd0(e, reference).is_null()
        {
            let clone = e.vcall(reference.addr(), 0x1c8, &args![1u32]).u32();
            let cloned_slot = this.byte_add(QueuedReference::spCloned3D.off);
            e.call(NI_POINTER_ASSIGN, &args![cloned_slot, clone]);
            if ni_pointer_value(e, cloned_slot) != 0 {
                let cloned = ni_pointer_value(e, cloned_slot);
                e.call(OBJECT_RELEASE, &args![cloned]);
            }
        }
        true
    })
}

// Translated from 00440ba0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedReference::Attach` (Xbox PDB): nothing happens for a task in state
/// 6. Otherwise, inside the memory context of the task (source line
/// `0x804`), a reference with form flag `0x800` or `0x20` ([`fn_00440da0`],
/// [`fn_00440d80`]) is left alone; any other reference whose parent cell is
/// loaded (`TES::IsCellLoaded`) gets its addon nodes loaded
/// (`ModelLoader::LoadAddonNodes` with the model's root node, when the task
/// has a model) and is loaded by `TES` (`00451ef0` with the task as source).
/// In both cases the loader's object at `loader + 8` is told about the
/// reference ([`fn_004409e0`]), and the parent cell's queued-reference count
/// is lowered (and its critical count, for a reference whose 3D is critical);
/// `00541ac0` is called on the parent cell at the start and `00541ae0` at the
/// end.
///
/// The exception-unwinding frame is not translated.
pub fn queued_reference_attach(e: &mut Engine, this: Ptr<QueuedReference>) {
    let context = e.get(this, QueuedReference::eContext);
    in_context(e, context, ATTACH_SOURCE_LINE, |e| {
        if e.call(TASK_STATE_IS_6, &args![this]).bool() {
            return;
        }
        let reference = e.get(this, QueuedReference::pRef);
        let skip = fn_00440da0(e, reference.cast()) || fn_00440d80(e, reference.cast());
        let cell = e.call(REFERENCE_CELL, &args![reference]).u32();
        if cell != 0 {
            let cell = e.call(REFERENCE_CELL, &args![reference]).u32();
            e.call(CELL_ENTER_ATTACH, &args![cell]);
        }
        if !skip {
            let cell = e.call(REFERENCE_CELL, &args![reference]).u32();
            let tes = e.global::<u32>(TES_GLOBAL);
            if e.call(TES_IS_CELL_LOADED, &args![tes, cell, 0u32]).bool() {
                let model_slot = this.byte_add(QueuedReference::spModel.off);
                if ni_pointer_value(e, model_slot) != 0 {
                    let model = ni_pointer_value(e, model_slot);
                    let node = fn_0043b230(e, Ptr::new(model)).addr();
                    let loader = e.global::<u32>(MODEL_LOADER);
                    e.call(
                        MODEL_LOADER_LOAD_ADDON_NODES,
                        &args![loader, reference, node],
                    );
                }
                let cell = e.call(REFERENCE_CELL, &args![reference]).u32();
                let tes = e.global::<u32>(TES_GLOBAL);
                e.call(TES_LOAD_REFERENCE, &args![tes, reference, cell, this, 0u32]);
            }
        }
        let loader = e.global::<u32>(MODEL_LOADER);
        fn_004409e0(e, Ptr::new(loader), reference);
        let cell = e.call(REFERENCE_CELL, &args![reference]).u32();
        if cell != 0 {
            let cell = e.call(REFERENCE_CELL, &args![reference]).u32();
            fn_00440a10(e, Ptr::new(cell));
            if e.call(REFERENCE_IS_3D_CRITICAL, &args![reference]).bool() {
                let cell = e.call(REFERENCE_CELL, &args![reference]).u32();
                fn_00440a50(e, Ptr::new(cell));
            }
            let cell = e.call(REFERENCE_CELL, &args![reference]).u32();
            e.call(CELL_LEAVE_ATTACH, &args![cell]);
        }
    });
}

// Translated from 00440d80 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESForm` method (no name in the engine map): whether bit `0x20` of
/// `iFormFlags` is set.
pub fn fn_00440d80(e: &mut Engine, this: Ptr<TESForm>) -> bool {
    e.get(this, TESForm::iFormFlags) & 0x20 != 0
}

// Translated from 00440da0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESForm` method (no name in the engine map): whether bit `0x800` of
/// `iFormFlags` is set.
pub fn fn_00440da0(e: &mut Engine, this: Ptr<TESForm>) -> bool {
    e.get(this, TESForm::iFormFlags) & 0x800 != 0
}

// Translated from 00440dc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedReference::GetDescription` (Xbox PDB): for a task not in state 6
/// writes `"Queued ref '%s' (%08X) of type %s"` into `buffer` (`size` bytes)
/// and returns true: the first text is the result of the reference's virtual
/// function `0x130`, the number is its form ID (`0084e3a0`), and the last is
/// the name of the base form's type ([`fn_00440e30`]). A task in state 6
/// returns false.
pub fn queued_reference_get_description(
    e: &mut Engine,
    this: Ptr<QueuedReference>,
    buffer: u32,
    size: u32,
) -> bool {
    if e.call(TASK_STATE_IS_6, &args![this]).bool() {
        return false;
    }
    let reference = e.get(this, QueuedReference::pRef);
    let form = e.call(GET_BASE_FORM, &args![reference]).u32();
    let type_name = fn_00440e30(e, Ptr::new(form));
    let form_id = e.call(GET_FORM_ID, &args![reference]).u32();
    let text = e.vcall(reference.addr(), 0x130, &args![]).u32();
    e.call(
        FORMAT_STRING,
        &args![buffer, size, QUEUED_REF_FORMAT, text, form_id, type_name],
    );
    true
}

// Translated from 00440e30 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESForm` method (no name in the engine map): the name text of the
/// form's type, the first dword of the 12-byte entry `cFormType` of the table
/// at `01187004`.
pub fn fn_00440e30(e: &mut Engine, this: Ptr<TESForm>) -> u32 {
    let form_type = e.get(this, TESForm::cFormType) as u32;
    e.global::<u32>(FORM_TYPE_TABLE + form_type * 12)
}

// Translated from 00440e50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedTree` constructor (no name in the engine map): the
/// `QueuedReference` constructor ([`fn_0043fd40`]) with `reference` and
/// `context`, then the tree task's virtual table. Returns `this`.
pub fn fn_00440e50(
    e: &mut Engine,
    this: Ptr<QueuedReference>,
    reference: Ptr,
    context: u32,
) -> Ptr<QueuedReference> {
    fn_0043fd40(e, this, reference, context);
    e.mem.set_u32(this.addr(), QUEUED_TREE_VTABLE);
    this
}

// Translated from 00440e80 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedTree` scalar deleting destructor (no name in the engine map): runs
/// the destructor ([`fn_00440eb0`]) and, when bit 0 of `flags` is set, frees
/// the object. Returns `this`.
pub fn fn_00440e80(e: &mut Engine, this: Ptr<QueuedReference>, flags: u32) -> Ptr<QueuedReference> {
    fn_00440eb0(e, this);
    if flags & 1 != 0 {
        e.call(MEMORY_FREE, &args![this]);
    }
    this
}

// Translated from 00440eb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedTree` destructor (no name in the engine map): the
/// `QueuedReference` destructor ([`fn_0043fe40`]).
pub fn fn_00440eb0(e: &mut Engine, this: Ptr<QueuedReference>) {
    fn_0043fe40(e, this);
}

// Translated from 00440ed0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedTree::QueueModels` (Xbox PDB): inside the memory context of the
/// task (source line `0x847`), the loader queues the tree model
/// (`00444540(reference, base form, priority, task, LOD multiplier)`). The LOD
/// multiplier is `TES::GetLODMult(base form)`, replaced by 6 when `00564e60`
/// accepts the reference and then by 10 when `00564f00` does.
///
/// The exception-unwinding frame is not translated.
pub fn queued_tree_queue_models(e: &mut Engine, this: Ptr<QueuedReference>) {
    let context = e.get(this, QueuedReference::eContext);
    in_context(e, context, TREE_QUEUE_MODELS_SOURCE_LINE, |e| {
        let reference = e.get(this, QueuedReference::pRef);
        let form = e.call(GET_BASE_FORM, &args![reference]).u32();
        let mut lod_mult = e.call(GET_LOD_MULT, &args![form]).u32();
        let reference = e.get(this, QueuedReference::pRef);
        if !reference.is_null() && e.call(REFERENCE_TEST_00564E60, &args![reference]).bool() {
            lod_mult = 6;
        }
        let reference = e.get(this, QueuedReference::pRef);
        if !reference.is_null() && e.call(REFERENCE_TEST_00564F00, &args![reference]).bool() {
            lod_mult = 0xa;
        }
        let priority = task_priority(e, this.cast());
        let reference = e.get(this, QueuedReference::pRef);
        let loader = e.global::<u32>(MODEL_LOADER);
        e.call(
            MODEL_LOADER_QUEUE_TREE_MODEL,
            &args![loader, reference, form, priority, this, lod_mult],
        );
    });
}

// Translated from 00440fd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedTree::UseDistant3D` (Xbox PDB): builds the tree's distant 3D
/// (`TESObjectTREE::BuildDistant3D` on the base form). Of the node it
/// returns, child 0 has its geometry data (`005495f0`) set to consistency
/// `0x4000` (`NiGeometryData::SetConsistency`), given `0x11` and `1` by the
/// two setters `004410d0` and `004410f0`, and the child is prepared
/// (`BSShaderManager::PrepareObject(child, 0, 0)`). If the child has a
/// property of type 3 whose own type is `0xb`, that property gets shader
/// flags `0xd` and `0x16` set ([`bs_shader_property_set_flag`]). The node is
/// handed to the task's virtual function `0x34` (`AttachDistant3D`).
pub fn queued_tree_use_distant_3d(e: &mut Engine, this: Ptr<QueuedReference>) {
    let reference = e.get(this, QueuedReference::pRef);
    let form = e.call(GET_BASE_FORM, &args![reference]).u32();
    let node = e.call(BUILD_DISTANT_3D, &args![form, 0u32]).u32();
    if node == 0 {
        return;
    }
    let child = fn_0043b4a0(e, Ptr::new(node), 0);
    let data = e.call(OBJECT_POINTER_AT_B8, &args![child]).u32();
    e.call(SET_CONSISTENCY, &args![data, 0x4000u32]);
    e.call(GEOMETRY_DATA_SETTER_FIRST, &args![data, 0x11u32]);
    e.call(GEOMETRY_DATA_SETTER_SECOND, &args![data, 1u32]);
    e.call(PREPARE_OBJECT, &args![child, 0u32, 0u32]);
    let mut property = 0;
    if e.call(GET_PROPERTY, &args![child, SHADER_PROPERTY_SLOT])
        .u32()
        != 0
    {
        let candidate = e
            .call(GET_PROPERTY, &args![child, SHADER_PROPERTY_SLOT])
            .u32();
        if e.call(PROPERTY_GET_TYPE, &args![candidate]).u32() == SHADER_PROPERTY_TYPE {
            property = e
                .call(GET_PROPERTY, &args![child, SHADER_PROPERTY_SLOT])
                .u32();
        }
    }
    if property != 0 {
        bs_shader_property_set_flag(e, Ptr::new(property), 0xd, 1);
        bs_shader_property_set_flag(e, Ptr::new(property), 0x16, 1);
    }
    e.vcall(this.addr(), 0x34, &args![node]);
}

// Translated from 00441130 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSShaderProperty::SetFlag` (Xbox PDB name in the engine map): sets
/// (`set` non-zero) or clears bit number `bit` of the array of flag dwords at
/// `this + 0x20` (dword `bit / 32`, bit `bit % 32`). When that changes the
/// bit, the dword at `this + 0x38` is cleared.
pub fn bs_shader_property_set_flag(e: &mut Engine, this: Ptr, bit: u32, set: u8) {
    let word = this.addr().wrapping_add(0x20).wrapping_add((bit >> 5) * 4);
    let mask = 1u32 << (bit % 32);
    let flags = e.mem.u32(word);
    if set != 0 {
        if flags & mask == 0 {
            e.mem.set_u32(this.addr() + 0x38, 0);
        }
        let flags = e.mem.u32(word);
        e.mem.set_u32(word, flags | mask);
    } else {
        if flags & mask != 0 {
            e.mem.set_u32(this.addr() + 0x38, 0);
        }
        let flags = e.mem.u32(word);
        e.mem.set_u32(word, !mask & flags);
    }
}

// Translated from 004411d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedActor` constructor (no name in the engine map): the
/// `QueuedReference` constructor ([`fn_0043fd40`]) with `reference` and
/// `context`, then the actor task's virtual table. Returns `this`.
pub fn fn_004411d0(
    e: &mut Engine,
    this: Ptr<QueuedReference>,
    reference: Ptr,
    context: u32,
) -> Ptr<QueuedReference> {
    fn_0043fd40(e, this, reference, context);
    e.mem.set_u32(this.addr(), QUEUED_ACTOR_VTABLE);
    this
}

// Translated from 00441200 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedActor::QueueMe` (Xbox PDB): calls the reference's virtual function
/// `0x260`, then takes the thread-local queued flag ([`fn_0043c130`]). A
/// reference other than the one at global `011dea3c` that `00936ef0` does not
/// accept makes the flag 1 meanwhile ([`fn_00441290`]); an actor that
/// `008b7b70` accepts gets `008b7360(1, task)`. Then `QueuedReference::
/// QueueMe` runs and the flag is put back.
pub fn queued_actor_queue_me(e: &mut Engine, this: Ptr<QueuedReference>) {
    let reference = e.get(this, QueuedReference::pRef);
    e.vcall(reference.addr(), 0x260, &args![]);
    let old_flag = fn_0043c130(e);
    let loader = e.global::<u32>(MODEL_LOADER);
    if reference.addr() != e.global::<u32>(SPECIAL_REFERENCE)
        && e.call(REFERENCE_TEST_00936EF0, &args![reference]).u32() == 0
    {
        fn_00441290(e, Ptr::new(loader), 1);
    }
    if e.call(ACTOR_TEST_008B7B70, &args![reference]).bool() {
        e.call(ACTOR_ATTACH_TASK, &args![reference, 1u32, this]);
    }
    queued_reference_queue_me(e, this);
    let loader = e.global::<u32>(MODEL_LOADER);
    fn_00441290(e, Ptr::new(loader), old_flag);
}

// Translated from 00441290 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ModelLoader` method (no name in the engine map; `this` is not used):
/// stores `value` in the thread's queued flag (the byte at +0x25C of the TLS
/// block, the one [`fn_0043c130`] reads) and returns the byte that was there.
pub fn fn_00441290(e: &mut Engine, _this: Ptr, value: u8) -> u8 {
    let address = e.tls() + TLS_QUEUED_FLAG;
    let old = e.mem.u8(address);
    e.mem.set_u8(address, value);
    old
}

// Translated from 004412e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedActor::QueueModels` (Xbox PDB): inside the memory context of the
/// task (source line `0x8a2`), the reference is prepared (`008c2e50`) and
/// its extra data list is asked for the dismemberment extra
/// (`ExtraDataList::GetDismembermentExtra`). For every entry of the extra's
/// array at +0x20, and every element of that entry's array at +4, the loader
/// queues the element (`00444d40(element, priority, task, reference)`). Last
/// `QueuedReference::QueueModels` runs.
///
/// The exception-unwinding frame is not translated.
pub fn queued_actor_queue_models(e: &mut Engine, this: Ptr<QueuedReference>) {
    let context = e.get(this, QueuedReference::eContext);
    in_context(e, context, ACTOR_QUEUE_MODELS_SOURCE_LINE, |e| {
        let reference = e.get(this, QueuedReference::pRef);
        e.call(ACTOR_PREPARE, &args![reference]);
        let extra = e.call(REFERENCE_EXTRA_LIST, &args![reference]).u32();
        let dismember = e.call(EXTRA_GET_DISMEMBERMENT, &args![extra]).u32();
        if dismember != 0 {
            let priority = task_priority(e, this.cast());
            let mut index = 0;
            while index < e.call(ARRAY_COUNT_AT_8, &args![dismember + 0x20]).u32() {
                let entry = fn_00441420(e, Ptr::new(dismember), index);
                let mut inner = 0;
                while inner < e.call(ARRAY_COUNT_AT_8, &args![entry + 4]).u32() {
                    let slot = e
                        .call(ARRAY_ELEMENT_ADDRESS, &args![entry + 4, inner])
                        .u32();
                    let element = e.mem.u32(slot);
                    let reference = e.get(this, QueuedReference::pRef);
                    let loader = e.global::<u32>(MODEL_LOADER);
                    e.call(
                        MODEL_LOADER_QUEUE_DISMEMBER_PART,
                        &args![loader, element, priority, this, reference],
                    );
                    inner += 1;
                }
                index += 1;
            }
        }
        queued_reference_queue_models(e, this);
    });
}

// Translated from 00441420 (decompiled, FalloutNV.exe 1.4.0.525)
/// Dismemberment extra method (no name in the engine map): the dword stored
/// at the element `index` of the array at `this + 0x20`.
pub fn fn_00441420(e: &mut Engine, this: Ptr, index: u32) -> u32 {
    let slot = e
        .call(ARRAY_ELEMENT_ADDRESS, &args![this.addr() + 0x20, index])
        .u32();
    e.mem.u32(slot)
}

// Translated from 00441440 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedCharacter` constructor (no name in the engine map): the
/// `QueuedActor` constructor ([`fn_004411d0`]) with `reference` and
/// `context`, the character task's virtual table, and empty task pointers in
/// `spQueuedHead` and `spQueuedHelmet`. Returns `this`.
///
/// The exception-unwinding frame is not translated.
pub fn fn_00441440(
    e: &mut Engine,
    this: Ptr<QueuedCharacter>,
    reference: Ptr,
    context: u32,
) -> Ptr<QueuedCharacter> {
    fn_004411d0(e, this.cast(), reference, context);
    e.mem.set_u32(this.addr(), QUEUED_CHARACTER_VTABLE);
    e.call(
        TASK_POINTER_CONSTRUCT,
        &args![this.byte_add(QueuedCharacter::spQueuedHead.off), 0u32],
    );
    e.call(
        TASK_POINTER_CONSTRUCT,
        &args![this.byte_add(QueuedCharacter::spQueuedHelmet.off), 0u32],
    );
    this
}

/// This part's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x0043fed0, queued_reference_queue_me(Ptr<QueuedReference>)),
        entry!(
            0x00440050,
            queued_reference_queue_models(Ptr<QueuedReference>)
        ),
        entry!(
            0x00440190,
            queued_reference_use_distant_3d(Ptr<QueuedReference>)
        ),
        entry!(
            0x00440310,
            queued_reference_attach_distant_3d(Ptr<QueuedReference>, Ptr)
        ),
        entry!(0x00440460, fn_00440460(Ptr, Ptr)),
        entry!(0x00440490, fn_00440490(Ptr, f32)),
        entry!(
            0x004404c0,
            fn_004404c0(Ptr<AttachDistant3DTask>, Ptr, Ptr) -> Ptr<AttachDistant3DTask>
        ),
        entry!(0x00440540, fn_00440540(Ptr<IOTask>, u32) -> Ptr<IOTask>),
        entry!(0x004405b0, fn_004405b0(Ptr<IOTask>, u8)),
        entry!(0x004405d0, fn_004405d0(Ptr<IOTask>, u8)),
        entry!(0x00440610, fn_00440610(Ptr<IOTask>, u32) -> Ptr<IOTask>),
        entry!(0x00440640, fn_00440640(Ptr<IOTask>)),
        entry!(0x00440660, io_task_requeue(Ptr<IOTask>, u32)),
        entry!(
            0x00440680,
            attach_distant_3d_task_scalar_deleting_destructor(
                Ptr<AttachDistant3DTask>,
                u32,
            )
                -> Ptr<AttachDistant3DTask>
        ),
        entry!(0x004406b0, fn_004406b0(Ptr<AttachDistant3DTask>)),
        entry!(0x00440710, fn_00440710(Ptr, Ptr)),
        entry!(
            0x00440780,
            queued_reference_check_finished(Ptr<QueuedReference>)
        ),
        entry!(0x004408d0, fn_004408d0(Ptr, Ptr)),
        entry!(
            0x004408f0,
            queued_reference_cancel(Ptr<QueuedReference>, u32, u32)
        ),
        entry!(0x004409e0, fn_004409e0(Ptr, Ptr)),
        entry!(0x00440a10, fn_00440a10(Ptr<TESObjectCELL>)),
        entry!(0x00440a50, fn_00440a50(Ptr<TESObjectCELL>)),
        entry!(
            0x00440a90,
            queued_reference_background_clone(Ptr<QueuedReference>) -> bool
        ),
        entry!(0x00440ba0, queued_reference_attach(Ptr<QueuedReference>)),
        entry!(0x00440d80, fn_00440d80(Ptr<TESForm>) -> bool),
        entry!(0x00440da0, fn_00440da0(Ptr<TESForm>) -> bool),
        entry!(
            0x00440dc0,
            queued_reference_get_description(Ptr<QueuedReference>, u32, u32) -> bool
        ),
        entry!(0x00440e30, fn_00440e30(Ptr<TESForm>) -> u32),
        entry!(
            0x00440e50,
            fn_00440e50(Ptr<QueuedReference>, Ptr, u32) -> Ptr<QueuedReference>
        ),
        entry!(
            0x00440e80,
            fn_00440e80(Ptr<QueuedReference>, u32) -> Ptr<QueuedReference>
        ),
        entry!(0x00440eb0, fn_00440eb0(Ptr<QueuedReference>)),
        entry!(0x00440ed0, queued_tree_queue_models(Ptr<QueuedReference>)),
        entry!(0x00440fd0, queued_tree_use_distant_3d(Ptr<QueuedReference>)),
        entry!(0x00441130, bs_shader_property_set_flag(Ptr, u32, u8)),
        entry!(
            0x004411d0,
            fn_004411d0(Ptr<QueuedReference>, Ptr, u32) -> Ptr<QueuedReference>
        ),
        entry!(0x00441200, queued_actor_queue_me(Ptr<QueuedReference>)),
        entry!(0x00441290, fn_00441290(Ptr, u8) -> u8),
        entry!(0x004412e0, queued_actor_queue_models(Ptr<QueuedReference>)),
        entry!(0x00441420, fn_00441420(Ptr, u32) -> u32),
        entry!(
            0x00441440,
            fn_00441440(Ptr<QueuedCharacter>, Ptr, u32) -> Ptr<QueuedCharacter>
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test doubles that answer 1 and 0, and ones that do nothing; they
    /// stand in for virtual functions (any slot) and plain callees.
    const YES: u32 = 0x0ff0_0001;
    const NO: u32 = 0x0ff0_0000;
    const NOTHING: u32 = 0x0ff0_0010;
    /// Virtual functions the tests tell apart in the call log.
    const SLOT_A: u32 = 0x0ff0_0020;
    const SLOT_B: u32 = 0x0ff0_0021;
    const SLOT_C: u32 = 0x0ff0_0022;
    const SLOT_D: u32 = 0x0ff0_0023;
    /// Virtual functions that return a value chosen by the test.
    const ANSWER_A: u32 = 0x0ff0_0100;
    const ANSWER_B: u32 = 0x0ff0_0101;
    const REFERENCE_VTABLE: u32 = 0x0ff1_0000;
    const TASK_VTABLE: u32 = 0x0ff1_1000;
    const OBJECT_VTABLE: u32 = 0x0ff1_2000;
    const OTHER_VTABLE: u32 = 0x0ff1_3000;
    /// The memory context and the priority of the tasks in the tests.
    const CONTEXT: u32 = 0x31;
    const PRIORITY: u8 = 7;
    /// A mapped word where doubles leave what they observed.
    const OBSERVED: u32 = 0x011d_e800;
    /// `QueueTable::Add` (`00449210`) and the array-element getter
    /// `0043b4a0` uses (`00877a30`).
    const QUEUE_TABLE_ADD: u32 = 0x0044_9210;
    const NODE_ARRAY_ELEMENT: u32 = 0x0087_7a30;
    /// The `QueuedFile` constructor and destructor body (`00c3c590`, `00c3c620`).
    const QUEUED_FILE_CONSTRUCT: u32 = 0x00c3_c590;
    const QUEUED_FILE_DESTRUCT: u32 = 0x00c3_c620;

    fn calls_to(e: &Engine, addr: u32) -> Vec<Vec<u32>> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .filter(|(a, _)| *a == addr)
            .map(|(_, args)| args.clone())
            .collect()
    }

    /// The addresses among `addrs` that were called, in order.
    fn order_of(e: &Engine, addrs: &[u32]) -> Vec<u32> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .map(|(a, _)| *a)
            .filter(|a| addrs.contains(a))
            .collect()
    }

    fn answer(e: &mut Engine, addr: u32, value: u32) {
        e.register_double(addr, move |_, _| value.into_ret());
    }

    fn decrement(e: &mut Engine) {
        e.register(INTERLOCKED_DECREMENT, |e, a| {
            let value = e.mem.i32(a[0]) - 1;
            e.mem.set_i32(a[0], value);
            value.into_ret()
        });
    }

    /// An engine with the memory manager, memory context, `NiPointer` and
    /// task pointer functions as doubles, the pages of the globals the code
    /// reads, a loader object in the loader global, and the `YES`, `NO` and
    /// `NOTHING` doubles.
    fn engine() -> Engine {
        let mut e = Engine::new();
        for page in [0x011c_3000, 0x011d_e000, 0x0118_7000, 0x0120_2000] {
            e.map(page, 0x1000);
        }
        e.register(MEMORY_ALLOC, |e, a| e.mem.alloc(a[0]).into_ret());
        e.register(MEMORY_FREE, |e, a| {
            e.mem.free(a[0]);
            Ret::default()
        });
        e.register(MEMORY_CONTEXT_ENTER, |_, _| Ret::default());
        e.register(MEMORY_CONTEXT_LEAVE, |_, _| Ret::default());
        e.register(NI_POINTER_GET, |e, a| e.mem.u32(a[0]).into_ret());
        for addr in [
            NI_POINTER_ASSIGN,
            NI_POINTER_CONSTRUCT,
            TASK_POINTER_CONSTRUCT,
        ] {
            e.register(addr, |e, a| {
                e.mem.set_u32(a[0], a[1]);
                a[0].into_ret()
            });
        }
        e.register(NI_POINTER_DESTRUCT, |_, _| Ret::default());
        e.register(TASK_POINTER_DESTRUCT, |_, _| Ret::default());
        e.register(YES, |_, _| 1u32.into_ret());
        e.register(NO, |_, _| 0u32.into_ret());
        for addr in [NOTHING, SLOT_A, SLOT_B, SLOT_C, SLOT_D] {
            e.register(addr, |_, _| Ret::default());
        }
        let loader = e.mem.alloc(0x40);
        e.set_global(MODEL_LOADER, loader);
        e
    }

    fn loader(e: &Engine) -> u32 {
        e.global::<u32>(MODEL_LOADER)
    }

    /// Writes a virtual table at `at` whose slots all point to `NO` except
    /// the given (byte offset, function) pairs.
    fn vtable(e: &mut Engine, at: u32, slots: &[(u32, u32)]) {
        let mut table = vec![NO; 0x100];
        for (slot, function) in slots {
            table[(slot / 4) as usize] = *function;
        }
        e.put_vtable(at, &table);
    }

    /// An object of `size` bytes whose virtual table is `table`.
    fn object(e: &mut Engine, table: u32, size: u32) -> u32 {
        let address = e.mem.alloc(size);
        e.mem.set_u32(address, table);
        address
    }

    /// A reference object whose base form (`+0x20`) is `form`.
    fn reference(e: &mut Engine, slots: &[(u32, u32)], form: u32) -> Ptr {
        vtable(e, REFERENCE_VTABLE, slots);
        let address = object(e, REFERENCE_VTABLE, 0x100);
        e.mem.set_u32(address + 0x20, form);
        Ptr::new(address)
    }

    /// The getter of the base form (`+0x20`) as a double.
    fn base_form_double(e: &mut Engine) {
        e.register(GET_BASE_FORM, |e, a| e.mem.u32(a[0] + 0x20).into_ret());
    }

    /// A reference task for `reference`, with the test priority and context;
    /// virtual functions `0x28` to `0x34` are `SLOT_A` to `SLOT_D`.
    fn task(e: &mut Engine, reference: Ptr) -> Ptr<QueuedReference> {
        vtable(
            e,
            TASK_VTABLE,
            &[
                (0x28, SLOT_A),
                (0x2c, SLOT_B),
                (0x30, SLOT_C),
                (0x34, SLOT_D),
            ],
        );
        let this = e.new_object::<QueuedReference>();
        e.mem.set_u32(this.addr(), TASK_VTABLE);
        e.set(this, QueuedReference::eContext, CONTEXT);
        e.set(this, QueuedReference::pRef, reference);
        e.set(this.cast::<IOTask>(), IOTask::Key, (PRIORITY as u64) << 16);
        this
    }

    /// The doubles for a singly linked list of forms and the getters of the
    /// base form and type.
    fn list_doubles(e: &mut Engine) {
        e.register(LIST_NODE_IS_EMPTY, |e, a| {
            ((e.mem.u32(a[0]) == 0 && e.mem.u32(a[0] + 4) == 0) as u32).into_ret()
        });
        e.register(LIST_NODE_ITEM_ADDRESS, |_, a| a[0].into_ret());
        e.register(LIST_NODE_NEXT, |e, a| e.mem.u32(a[0] + 4).into_ret());
        base_form_double(e);
        e.register(FORM_TYPE, |e, a| (e.mem.u8(a[0] + 4) as u32).into_ret());
    }

    /// A form of type `kind`.
    fn form(e: &mut Engine, kind: u8) -> u32 {
        let address = e.mem.alloc(0x30);
        e.mem.set_u8(address + 4, kind);
        address
    }

    /// A list node holding an item that points to a holder of `form`.
    fn list_node(e: &mut Engine, form: u32, next: u32) -> u32 {
        let holder = e.mem.alloc(0x30);
        e.mem.set_u32(holder + 0x20, form);
        let item = e.mem.alloc(8);
        e.mem.set_u32(item, holder);
        let node = e.mem.alloc(8);
        e.mem.set_u32(node, item);
        e.mem.set_u32(node + 4, next);
        node
    }

    #[test]
    fn queue_me_queues_decal_texture_sets_of_type_4_and_runs_the_task_slots() {
        let mut e = engine();
        list_doubles(&mut e);
        e.register(REFERENCE_TEST_00564E60, |_, _| 1u32.into_ret());
        e.register(REFERENCE_EXTRA_LIST, |_, a| (a[0] + 0x44).into_ret());
        let texture_set = form(&mut e, 4);
        let other = form(&mut e, 5);
        let second = list_node(&mut e, other, 0);
        let first = list_node(&mut e, texture_set, second);
        answer(&mut e, EXTRA_GET_DECAL_REFS, first);
        e.register(QUEUE_TEXTURE_SET, |_, _| Ret::default());
        e.register(TASK_STATE_IS_0, |_, _| 1u32.into_ret());
        e.register(TASK_SET_STATE_5, |_, _| Ret::default());
        let base = form(&mut e, 1);
        let reference = reference(&mut e, &[(0x1d0, NO)], base);
        let this = task(&mut e, reference);
        e.call_log = Some(vec![]);
        e.call(0x0043_fed0, &args![this]);
        assert_eq!(
            calls_to(&e, MEMORY_CONTEXT_ENTER)
                .iter()
                .map(|a| a[1..].to_vec())
                .collect::<Vec<_>>(),
            vec![vec![CONTEXT, 1, MODEL_LOADER_SOURCE, 0x72a]]
        );
        assert_eq!(
            calls_to(&e, QUEUE_TEXTURE_SET),
            vec![vec![texture_set, PRIORITY as u32, this.addr(), 0]]
        );
        // UseDistant3D (0x30), QueueModels (0x2c), state 5, CheckFinished (0x28).
        assert_eq!(
            order_of(
                &e,
                &[
                    SLOT_A,
                    SLOT_B,
                    SLOT_C,
                    TASK_SET_STATE_5,
                    MEMORY_CONTEXT_LEAVE
                ]
            ),
            vec![
                SLOT_C,
                SLOT_B,
                TASK_SET_STATE_5,
                SLOT_A,
                MEMORY_CONTEXT_LEAVE
            ]
        );
    }

    #[test]
    fn queue_me_without_use_distant_3d_or_decals_only_queues_models() {
        let mut e = engine();
        e.register(REFERENCE_TEST_00564E60, |_, _| 0u32.into_ret());
        e.register(REFERENCE_EXTRA_LIST, |_, a| (a[0] + 0x44).into_ret());
        e.register(EXTRA_GET_DECAL_REFS, |_, _| 0u32.into_ret());
        e.register(TASK_STATE_IS_0, |_, _| 0u32.into_ret());
        e.register(TASK_SET_STATE_5, |_, _| Ret::default());
        let reference = reference(&mut e, &[], 0);
        let this = task(&mut e, reference);
        e.call_log = Some(vec![]);
        e.call(0x0043_fed0, &args![this]);
        assert_eq!(
            order_of(&e, &[SLOT_A, SLOT_B, SLOT_C, TASK_SET_STATE_5]),
            vec![SLOT_B, SLOT_A]
        );
        // A reference that passes the test but whose slot 0x1d0 answers
        // non-zero does not use the distant 3D either.
        e.register(REFERENCE_TEST_00564E60, |_, _| 1u32.into_ret());
        answer(&mut e, ANSWER_A, 5);
        let busy = self::reference(&mut e, &[(0x1d0, ANSWER_A)], 0);
        e.set(this, QueuedReference::pRef, busy);
        e.call_log = Some(vec![]);
        e.call(0x0043_fed0, &args![this]);
        assert!(calls_to(&e, SLOT_C).is_empty());
        // A null reference has no decals.
        e.set(this, QueuedReference::pRef, Ptr::NULL);
        e.call_log = Some(vec![]);
        e.call(0x0043_fed0, &args![this]);
        assert!(calls_to(&e, EXTRA_GET_DECAL_REFS).is_empty());
    }

    /// A `TESModel` object whose virtual function `0x14` gives the name
    /// `text`.
    fn tes_model(e: &mut Engine, text: &[u8]) -> u32 {
        let name = e.mem.alloc(0x20);
        e.mem.set_cstr(name, text);
        answer(e, ANSWER_B, name);
        vtable(e, OBJECT_VTABLE, &[(0x14, ANSWER_B)]);
        object(e, OBJECT_VTABLE, 0x40)
    }

    #[test]
    fn queue_models_queues_the_model_the_lookup_and_the_destruction_files() {
        let mut e = engine();
        let model = tes_model(&mut e, b"meshes\\a.nif");
        answer(&mut e, REFERENCE_GET_TES_MODEL, model);
        base_form_double(&mut e);
        e.register(MODEL_LOADER_QUEUE_REFERENCE_MODEL, |_, _| Ret::default());
        e.register(MODEL_LOADER_FIND_MODEL, |_, _| Ret::default());
        let destruction = form(&mut e, 9);
        answer(&mut e, GET_DESTRUCTION_FORM, destruction);
        e.register(DESTRUCTIBLE_QUEUE_FILES, |_, _| Ret::default());
        let base = form(&mut e, 1);
        let reference = reference(&mut e, &[], base);
        let this = task(&mut e, reference);
        e.call_log = Some(vec![]);
        e.call(0x0044_0050, &args![this]);
        let loader = loader(&e);
        assert_eq!(
            calls_to(&e, MODEL_LOADER_QUEUE_REFERENCE_MODEL),
            vec![vec![
                loader,
                base,
                this.addr() + 0x2c,
                PRIORITY as u32,
                this.addr(),
                reference.addr()
            ]]
        );
        // The queued model slot stayed empty: the model is looked up by name.
        let find = calls_to(&e, MODEL_LOADER_FIND_MODEL);
        assert_eq!(find.len(), 1);
        assert_eq!(find[0][0], loader);
        assert_eq!(find[0][2], this.addr() + 0x30);
        assert_eq!(e.mem.cstr(find[0][1]), b"meshes\\a.nif".to_vec());
        assert_eq!(
            calls_to(&e, DESTRUCTIBLE_QUEUE_FILES),
            vec![vec![destruction, base, PRIORITY as u32, this.addr()]]
        );
    }

    #[test]
    fn queue_models_skips_the_lookup_when_the_loader_queued_a_model() {
        let mut e = engine();
        let model = tes_model(&mut e, b"a");
        answer(&mut e, REFERENCE_GET_TES_MODEL, model);
        base_form_double(&mut e);
        e.register(MODEL_LOADER_QUEUE_REFERENCE_MODEL, |e, a| {
            e.mem.set_u32(a[2], 0x1234);
            Ret::default()
        });
        e.register(MODEL_LOADER_FIND_MODEL, |_, _| Ret::default());
        answer(&mut e, GET_DESTRUCTION_FORM, 0);
        e.register(DESTRUCTIBLE_QUEUE_FILES, |_, _| Ret::default());
        let reference = reference(&mut e, &[], 0x5000);
        let this = task(&mut e, reference);
        e.call_log = Some(vec![]);
        e.call(0x0044_0050, &args![this]);
        assert!(calls_to(&e, MODEL_LOADER_FIND_MODEL).is_empty());
        assert!(calls_to(&e, DESTRUCTIBLE_QUEUE_FILES).is_empty());
    }

    #[test]
    fn queue_models_does_nothing_without_a_model_name() {
        let mut e = engine();
        base_form_double(&mut e);
        e.register(MODEL_LOADER_QUEUE_REFERENCE_MODEL, |_, _| Ret::default());
        let reference = reference(&mut e, &[], 0x5000);
        let this = task(&mut e, reference);
        // No TESModel.
        answer(&mut e, REFERENCE_GET_TES_MODEL, 0);
        e.call_log = Some(vec![]);
        e.call(0x0044_0050, &args![this]);
        assert!(calls_to(&e, MODEL_LOADER_QUEUE_REFERENCE_MODEL).is_empty());
        assert_eq!(calls_to(&e, MEMORY_CONTEXT_LEAVE).len(), 1);
        // An empty name.
        let model = tes_model(&mut e, b"");
        answer(&mut e, REFERENCE_GET_TES_MODEL, model);
        e.call_log = Some(vec![]);
        e.call(0x0044_0050, &args![this]);
        assert!(calls_to(&e, MODEL_LOADER_QUEUE_REFERENCE_MODEL).is_empty());
        assert_eq!(calls_to(&e, MEMORY_CONTEXT_LEAVE).len(), 1);
    }

    /// Doubles for `UseDistant3D`: the distant model name, a loader that
    /// finds a model (or not), and a clone of its root whose virtual
    /// function `0xc` gives a fade node (or 0) whose virtual function `0x10`
    /// answers `fade_answer`. Returns the task and the fade node.
    fn use_distant_3d_setup(
        e: &mut Engine,
        found: bool,
        has_fade_node: bool,
        fade_answer: u32,
    ) -> (Ptr<QueuedReference>, u32) {
        base_form_double(e);
        e.register(GET_DISTANT_MODEL_NAME, |e, a| {
            e.mem.set_cstr(a[0], b"distant.nif");
            Ret::default()
        });
        let model = e.mem.alloc(0x20);
        let root = e.mem.alloc(0x20);
        e.mem.set_u32(model + 0xc, root);
        e.register_double(MODEL_LOADER_FIND_MODEL, move |e, a| {
            if found {
                e.mem.set_u32(a[2], model);
            }
            Ret::default()
        });
        answer(e, ANSWER_A, fade_answer);
        vtable(e, OTHER_VTABLE, &[(0x10, ANSWER_A)]);
        let fade = if has_fade_node {
            object(e, OTHER_VTABLE, 0x20)
        } else {
            0
        };
        answer(e, ANSWER_B, fade);
        vtable(e, OBJECT_VTABLE, &[(0xc, ANSWER_B)]);
        let clone = object(e, OBJECT_VTABLE, 0x20);
        answer(e, NI_OBJECT_CLONE, clone);
        e.register(FADE_NODE_SET_LOD_MULT_TYPE, |_, _| Ret::default());
        e.register(MODEL_POINTER_RELEASE, |_, _| Ret::default());
        let reference = reference(e, &[], 0x5000);
        (task(e, reference), fade)
    }

    #[test]
    fn use_distant_3d_sets_the_lod_type_of_the_fade_node_and_attaches_it() {
        let mut e = engine();
        let (this, fade) = use_distant_3d_setup(&mut e, true, true, 1);
        e.call_log = Some(vec![]);
        e.call(0x0044_0190, &args![this]);
        assert_eq!(
            calls_to(&e, FADE_NODE_SET_LOD_MULT_TYPE),
            vec![vec![fade, 6]]
        );
        // SLOT_D is the task's virtual function 0x34.
        assert_eq!(calls_to(&e, SLOT_D), vec![vec![this.addr(), fade]]);
        assert_eq!(calls_to(&e, MODEL_POINTER_RELEASE).len(), 1);
        let find = calls_to(&e, MODEL_LOADER_FIND_MODEL);
        assert_eq!(e.mem.cstr(find[0][1]), b"distant.nif".to_vec());
    }

    #[test]
    fn use_distant_3d_attaches_a_fade_node_without_the_lod_change_when_slot_10_is_zero() {
        let mut e = engine();
        let (this, fade) = use_distant_3d_setup(&mut e, true, true, 0);
        e.call_log = Some(vec![]);
        e.call(0x0044_0190, &args![this]);
        assert!(calls_to(&e, FADE_NODE_SET_LOD_MULT_TYPE).is_empty());
        assert_eq!(calls_to(&e, SLOT_D), vec![vec![this.addr(), fade]]);
    }

    #[test]
    fn use_distant_3d_lets_a_clone_without_fade_node_go() {
        let mut e = engine();
        let (this, _) = use_distant_3d_setup(&mut e, true, false, 0);
        e.call_log = Some(vec![]);
        e.call(0x0044_0190, &args![this]);
        assert_eq!(calls_to(&e, NI_OBJECT_CLONE).len(), 1);
        // The temporary NiPointer takes the clone and releases it.
        let holding: Vec<_> = calls_to(&e, NI_POINTER_CONSTRUCT)
            .into_iter()
            .filter(|a| a[1] != 0)
            .collect();
        assert_eq!(holding.len(), 1);
        assert_eq!(calls_to(&e, NI_POINTER_DESTRUCT).len(), 1);
        assert!(calls_to(&e, SLOT_D).is_empty());
        assert!(calls_to(&e, FADE_NODE_SET_LOD_MULT_TYPE).is_empty());
    }

    #[test]
    fn use_distant_3d_without_a_model_only_releases_the_pointer() {
        let mut e = engine();
        let (this, _) = use_distant_3d_setup(&mut e, false, true, 1);
        e.call_log = Some(vec![]);
        e.call(0x0044_0190, &args![this]);
        assert!(calls_to(&e, NI_OBJECT_CLONE).is_empty());
        assert!(calls_to(&e, SLOT_D).is_empty());
        assert_eq!(calls_to(&e, MODEL_POINTER_RELEASE).len(), 1);
    }

    /// Doubles for `AttachDistant3D`; returns the task, the reference and the
    /// node.
    fn attach_distant_3d_setup(
        e: &mut Engine,
        owner_thread: u32,
    ) -> (Ptr<QueuedReference>, Ptr, Ptr) {
        let position = e.mem.alloc(12);
        for (i, v) in [1.5f32, 2.5, 3.5].iter().enumerate() {
            e.mem.set_f32(position + 4 * i as u32, *v);
        }
        answer(e, ANSWER_A, position);
        let reference = reference(e, &[(0x1f4, ANSWER_A)], 0x5000);
        e.register(REFERENCE_GET_ORIENTATION, |e, a| {
            for i in 0..9 {
                e.mem.set_u32(a[1] + 4 * i, 100 + i);
            }
            a[1].into_ret()
        });
        e.register(REFERENCE_GET_SCALE, |_, _| 1.25f32.into_ret());
        e.register(FLOAT_FILTER, |_, a| (f32::from_bits(a[0]) * 2.0).into_ret());
        let owner = e.mem.alloc(0x20);
        e.mem.set_u32(owner + 0x10, owner_thread);
        e.set_global(THREAD_OWNER_OBJECT, owner);
        e.register(OWNER_THREAD_ID, |e, a| e.mem.u32(a[0] + 0x10).into_ret());
        answer(e, CURRENT_THREAD_ID, 5);
        answer(e, REFERENCE_CELL, 0x7000);
        let tes = e.mem.alloc(0x10);
        e.set_global(TES_GLOBAL, tes);
        e.register(REFERENCE_SET_3D, |_, _| Ret::default());
        e.register(TES_LOAD_REFERENCE, |_, _| Ret::default());
        e.register(SET_STARTS_DEAD, |_, _| Ret::default());
        e.register(IO_TASK_BASE_CONSTRUCT, |_, _| Ret::default());
        e.register(TASK_POINTER_ASSIGN, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            a[0].into_ret()
        });
        let queue = e.mem.alloc(0x10);
        let loader = loader(e);
        e.mem.set_u32(loader + 0x1c, queue);
        e.register(FINISHED_QUEUE_ADD, |e, a| {
            let observed = e.mem.u32(a[1]);
            e.mem.set_u32(OBSERVED, observed);
            Ret::default()
        });
        let this = task(e, reference);
        let node = Ptr::new(e.mem.alloc(0x70));
        (this, reference, node)
    }

    #[test]
    fn attach_distant_3d_copies_the_transform_and_attaches_on_the_owner_thread() {
        let mut e = engine();
        let (this, reference, node) = attach_distant_3d_setup(&mut e, 5);
        e.call_log = Some(vec![]);
        e.call(0x0044_0310, &args![this, node]);
        let n = node.addr();
        assert_eq!(e.mem.f32(n + 0x58), 1.5);
        assert_eq!(e.mem.f32(n + 0x5c), 2.5);
        assert_eq!(e.mem.f32(n + 0x60), 3.5);
        assert_eq!(e.mem.u32(n + 0x34), 100);
        assert_eq!(e.mem.u32(n + 0x34 + 32), 108);
        assert_eq!(e.mem.f32(n + 0x64), 2.5);
        assert_eq!(
            calls_to(&e, REFERENCE_SET_3D),
            vec![vec![reference.addr(), n]]
        );
        let tes = e.global::<u32>(TES_GLOBAL);
        assert_eq!(
            calls_to(&e, TES_LOAD_REFERENCE),
            vec![vec![tes, reference.addr(), 0x7000, 0, 0]]
        );
        assert_eq!(
            calls_to(&e, SET_STARTS_DEAD),
            vec![vec![reference.addr(), 1]]
        );
        assert!(calls_to(&e, MEMORY_ALLOC).is_empty());
    }

    #[test]
    fn attach_distant_3d_queues_a_task_on_another_thread() {
        let mut e = engine();
        let (this, reference, node) = attach_distant_3d_setup(&mut e, 6);
        e.call_log = Some(vec![]);
        e.call(0x0044_0310, &args![this, node]);
        assert!(calls_to(&e, REFERENCE_SET_3D).is_empty());
        assert_eq!(calls_to(&e, MEMORY_ALLOC), vec![vec![0x20]]);
        let task = e.get(this, QueuedReference::spAttachDistant3DTask);
        assert!(!task.is_null());
        let task = task.cast::<AttachDistant3DTask>();
        assert_eq!(e.mem.u32(task.addr()), ATTACH_DISTANT_3D_TASK_VTABLE);
        assert_eq!(e.get(task, AttachDistant3DTask::pRef), reference);
        assert_eq!(e.get(task, AttachDistant3DTask::spNode), node);
        // The loader's queue was handed a task pointer holding the task.
        assert_eq!(calls_to(&e, FINISHED_QUEUE_ADD).len(), 1);
        assert_eq!(e.mem.u32(OBSERVED), task.addr());
    }

    #[test]
    fn position_is_copied_to_offset_0x58() {
        let mut e = engine();
        let source = e.mem.alloc(12);
        for i in 0..3 {
            e.mem.set_u32(source + 4 * i, 0x10 + i);
        }
        let node = e.mem.alloc(0x70);
        e.call(
            0x0044_0460,
            &args![Ptr::<()>::new(node), Ptr::<()>::new(source)],
        );
        assert_eq!(e.mem.u32(node + 0x58), 0x10);
        assert_eq!(e.mem.u32(node + 0x5c), 0x11);
        assert_eq!(e.mem.u32(node + 0x60), 0x12);
        assert_eq!(e.mem.u32(node + 0x54), 0);
    }

    #[test]
    fn scale_goes_through_the_float_helper() {
        let mut e = engine();
        e.register(FLOAT_FILTER, |_, a| (f32::from_bits(a[0]) + 1.0).into_ret());
        let node = e.mem.alloc(0x70);
        e.call(0x0044_0490, &args![Ptr::<()>::new(node), 1.5f32]);
        assert_eq!(e.mem.f32(node + 0x64), 2.5);
    }

    #[test]
    fn io_task_constructor_sets_the_table_and_the_priority() {
        let mut e = engine();
        e.register(IO_TASK_BASE_CONSTRUCT, |_, _| Ret::default());
        let task = e.new_object::<IOTask>();
        e.set(task, IOTask::Key, 0x1111_2222_0033_4455);
        e.call_log = Some(vec![]);
        let back = e.call(0x0044_0540, &args![task, 0x1c5u32]).ptr::<IOTask>();
        assert_eq!(back, task);
        assert_eq!(e.mem.u32(task.addr()), IO_TASK_VTABLE);
        assert_eq!(
            calls_to(&e, IO_TASK_BASE_CONSTRUCT),
            vec![vec![task.addr()]]
        );
        // The priority is the low byte of the argument.
        assert_eq!(e.get(task, IOTask::Key), 0x1111_2222_00c5_4455);
        assert_eq!(fn_0043cc60(&mut e, task), 0xc5);
    }

    #[test]
    fn priority_replaces_bits_16_to_23_of_the_key() {
        let mut e = engine();
        let task = e.new_object::<IOTask>();
        e.set(task, IOTask::Key, 0xffff_ffff_ffff_ffff);
        e.call(0x0044_05d0, &args![task, 0x12u8]);
        assert_eq!(e.get(task, IOTask::Key), 0xffff_ffff_ff12_ffff);
        e.call(0x0044_05b0, &args![task, 0u8]);
        assert_eq!(e.get(task, IOTask::Key), 0xffff_ffff_ff00_ffff);
        e.call(0x0044_05b0, &args![task, 0xffu8]);
        assert_eq!(e.get(task, IOTask::Key), 0xffff_ffff_ffff_ffff);
    }

    #[test]
    fn attach_distant_3d_task_constructor_builds_the_task() {
        let mut e = engine();
        e.register(IO_TASK_BASE_CONSTRUCT, |_, _| Ret::default());
        let task = e.new_object::<AttachDistant3DTask>();
        let reference = Ptr::<()>::new(0x7100);
        let node = Ptr::<()>::new(0x7200);
        e.call_log = Some(vec![]);
        let back = e
            .call(0x0044_04c0, &args![task, reference, node])
            .ptr::<AttachDistant3DTask>();
        assert_eq!(back, task);
        assert_eq!(
            calls_to(&e, IO_TASK_BASE_CONSTRUCT),
            vec![vec![task.addr()]]
        );
        assert_eq!(e.mem.u32(task.addr()), ATTACH_DISTANT_3D_TASK_VTABLE);
        assert_eq!(e.get(task, AttachDistant3DTask::pRef), reference);
        assert_eq!(e.get(task, AttachDistant3DTask::spNode), node);
        // The priority of the new task is 0.
        assert_eq!(e.get(task.cast::<IOTask>(), IOTask::Key), 0);
    }

    #[test]
    fn io_task_scalar_deleting_destructor_frees_only_with_bit_0() {
        let mut e = engine();
        e.register(IO_TASK_BASE_DESTRUCT, |_, _| Ret::default());
        let task = e.mem.alloc(0x20);
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x0044_0610, &args![task, 0u32]).u32(), task);
        assert!(calls_to(&e, MEMORY_FREE).is_empty());
        assert_eq!(calls_to(&e, IO_TASK_BASE_DESTRUCT), vec![vec![task]]);
        assert_eq!(e.call(0x0044_0610, &args![task, 3u32]).u32(), task);
        assert_eq!(calls_to(&e, MEMORY_FREE), vec![vec![task]]);
    }

    #[test]
    fn io_task_destructor_calls_the_base_destructor() {
        let mut e = engine();
        e.register(IO_TASK_BASE_DESTRUCT, |_, _| Ret::default());
        e.call_log = Some(vec![]);
        e.call(0x0044_0640, &args![Ptr::<()>::new(0x6000)]);
        assert_eq!(calls_to(&e, IO_TASK_BASE_DESTRUCT), vec![vec![0x6000]]);
    }

    #[test]
    fn requeue_goes_to_the_io_manager() {
        let mut e = engine();
        let manager = e.mem.alloc(0x10);
        e.set_global(TASK_QUEUE, manager);
        e.register(IO_MANAGER_REQUEUE_TASK, |_, _| Ret::default());
        e.call_log = Some(vec![]);
        e.call(0x0044_0660, &args![Ptr::<()>::new(0x6000), 9u32]);
        assert_eq!(
            calls_to(&e, IO_MANAGER_REQUEUE_TASK),
            vec![vec![manager, 0x6000, 9]]
        );
    }

    #[test]
    fn attach_distant_3d_task_destructor_releases_the_node_and_the_base() {
        let mut e = engine();
        e.register(IO_TASK_BASE_DESTRUCT, |_, _| Ret::default());
        let task = e.new_object::<AttachDistant3DTask>();
        e.call_log = Some(vec![]);
        e.call(0x0044_06b0, &args![task]);
        assert_eq!(
            order_of(&e, &[NI_POINTER_DESTRUCT, IO_TASK_BASE_DESTRUCT]),
            vec![NI_POINTER_DESTRUCT, IO_TASK_BASE_DESTRUCT]
        );
        assert_eq!(
            calls_to(&e, NI_POINTER_DESTRUCT),
            vec![vec![task.addr() + 0x1c]]
        );
        // The scalar deleting form frees the object with bit 0.
        e.call(0x0044_0680, &args![task, 1u32]);
        assert_eq!(calls_to(&e, MEMORY_FREE), vec![vec![task.addr()]]);
        let back = e.call(0x0044_0680, &args![task, 0u32]).u32();
        assert_eq!(back, task.addr());
        assert_eq!(calls_to(&e, MEMORY_FREE).len(), 1);
    }

    #[test]
    fn finished_queue_entry_wraps_the_task_in_a_task_pointer() {
        let mut e = engine();
        let queue = e.mem.alloc(0x10);
        let loader = loader(&e);
        e.mem.set_u32(loader + 0x1c, queue);
        e.register(FINISHED_QUEUE_ADD, |e, a| {
            let held = e.mem.u32(a[1]);
            e.mem.set_u32(OBSERVED, held);
            Ret::default()
        });
        e.call_log = Some(vec![]);
        e.call(
            0x0044_0710,
            &args![Ptr::<()>::new(loader), Ptr::<()>::new(0x6100)],
        );
        assert_eq!(e.mem.u32(OBSERVED), 0x6100);
        let adds = calls_to(&e, FINISHED_QUEUE_ADD);
        assert_eq!(adds.len(), 1);
        assert_eq!(adds[0][0], queue);
        assert_eq!(
            order_of(
                &e,
                &[
                    TASK_POINTER_CONSTRUCT,
                    FINISHED_QUEUE_ADD,
                    TASK_POINTER_DESTRUCT
                ]
            ),
            vec![
                TASK_POINTER_CONSTRUCT,
                FINISHED_QUEUE_ADD,
                TASK_POINTER_DESTRUCT
            ]
        );
    }

    /// A ready task with a queued model and no model, and a reference that
    /// may be attached.
    struct Finishing {
        this: Ptr<QueuedReference>,
        reference: Ptr,
        queue: u32,
    }

    fn finishing(e: &mut Engine) -> Finishing {
        answer(e, TASK_STATE_AT_LEAST_4, 1);
        answer(e, TASK_STATE_IS_6, 0);
        answer(e, QUEUED_MODEL_GET_MODEL, 0x6400);
        e.register(MODEL_POINTER_ASSIGN, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            a[0].into_ret()
        });
        answer(e, REFERENCE_CELL, 0x7000);
        let tes = e.mem.alloc(0x10);
        e.set_global(TES_GLOBAL, tes);
        answer(e, TES_IS_CELL_LOADED, 1);
        answer(e, TES_REFERENCE_PREDICATE, 1);
        let other = e.mem.alloc(0x10);
        e.set_global(SPECIAL_REFERENCE, other);
        let queue = e.mem.alloc(0x10);
        let loader = loader(e);
        e.mem.set_u32(loader + 0x28, queue);
        e.register(BACKGROUND_CLONE_QUEUE_ADD, |_, _| Ret::default());
        // The post-process queue of the IO manager.
        let manager = e.mem.alloc(0x80);
        let table = e.mem.alloc(0x10);
        e.mem.set_u32(manager + 0x64, table);
        e.set_global(TASK_QUEUE, manager);
        e.register(QUEUE_TABLE_ADD, |_, _| Ret::default());
        let reference = reference(e, &[(0x224, NO), (0x220, NO)], 0x5000);
        let this = task(e, reference);
        e.set(this, QueuedReference::spQueuedModel, Ptr::new(0x6300));
        Finishing {
            this,
            reference,
            queue,
        }
    }

    #[test]
    fn check_finished_attaches_a_ready_task_through_the_loader() {
        let mut e = engine();
        let f = finishing(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x0044_0780, &args![f.this]);
        // The queued model's model was taken.
        assert_eq!(e.get(f.this, QueuedReference::spModel), Ptr::new(0x6400));
        assert_eq!(
            calls_to(&e, BACKGROUND_CLONE_QUEUE_ADD),
            vec![vec![f.queue, f.this.addr()]]
        );
        assert!(calls_to(&e, QUEUE_TABLE_ADD).is_empty());
        let tes = e.global::<u32>(TES_GLOBAL);
        assert_eq!(calls_to(&e, TES_IS_CELL_LOADED), vec![vec![tes, 0x7000, 0]]);
        assert_eq!(
            calls_to(&e, TES_REFERENCE_PREDICATE),
            vec![vec![tes, f.reference.addr()]]
        );
    }

    #[test]
    fn check_finished_post_processes_when_a_reference_slot_refuses() {
        for slot in [0x224u32, 0x220] {
            let mut e = engine();
            let f = finishing(&mut e);
            vtable(&mut e, REFERENCE_VTABLE, &[(slot, YES)]);
            e.call_log = Some(vec![]);
            e.call(0x0044_0780, &args![f.this]);
            assert!(calls_to(&e, BACKGROUND_CLONE_QUEUE_ADD).is_empty());
            // The post-process queue is the one of the task's priority.
            let adds = calls_to(&e, QUEUE_TABLE_ADD);
            assert_eq!(adds.len(), 1);
            assert_eq!(adds[0][1], PRIORITY as u32);
        }
    }

    #[test]
    fn check_finished_post_processes_when_a_condition_fails() {
        // The cell is not loaded.
        let mut e = engine();
        let f = finishing(&mut e);
        answer(&mut e, TES_IS_CELL_LOADED, 0);
        e.call_log = Some(vec![]);
        e.call(0x0044_0780, &args![f.this]);
        assert_eq!(calls_to(&e, QUEUE_TABLE_ADD).len(), 1);
        // The reference is the special one.
        let mut e = engine();
        let f = finishing(&mut e);
        e.set_global(SPECIAL_REFERENCE, f.reference.addr());
        e.call_log = Some(vec![]);
        e.call(0x0044_0780, &args![f.this]);
        assert!(calls_to(&e, TES_IS_CELL_LOADED).is_empty());
        assert_eq!(calls_to(&e, QUEUE_TABLE_ADD).len(), 1);
        // 00451e40 refuses.
        let mut e = engine();
        let f = finishing(&mut e);
        answer(&mut e, TES_REFERENCE_PREDICATE, 0);
        e.call_log = Some(vec![]);
        e.call(0x0044_0780, &args![f.this]);
        assert_eq!(calls_to(&e, QUEUE_TABLE_ADD).len(), 1);
        // The thread has a 3D object for the reference (0043fcd0).
        let mut e = engine();
        let f = finishing(&mut e);
        let tls = e.tls();
        e.mem.set_u32(tls + 0x264, f.reference.addr());
        e.mem.set_u32(tls + 0x260, 0x6500);
        e.call_log = Some(vec![]);
        e.call(0x0044_0780, &args![f.this]);
        assert_eq!(calls_to(&e, QUEUE_TABLE_ADD).len(), 1);
        // No queued model: no model is taken, and nothing is attached.
        let mut e = engine();
        let f = finishing(&mut e);
        e.set(f.this, QueuedReference::spQueuedModel, Ptr::NULL);
        e.call_log = Some(vec![]);
        e.call(0x0044_0780, &args![f.this]);
        assert!(calls_to(&e, MODEL_POINTER_ASSIGN).is_empty());
        assert_eq!(calls_to(&e, QUEUE_TABLE_ADD).len(), 1);
    }

    #[test]
    fn check_finished_does_nothing_for_unready_tasks() {
        // State below 4.
        let mut e = engine();
        let f = finishing(&mut e);
        answer(&mut e, TASK_STATE_AT_LEAST_4, 0);
        e.call_log = Some(vec![]);
        e.call(0x0044_0780, &args![f.this]);
        assert!(calls_to(&e, QUEUE_TABLE_ADD).is_empty());
        assert!(calls_to(&e, BACKGROUND_CLONE_QUEUE_ADD).is_empty());
        // Unfinished children (2 expected, 1 finished).
        let mut e = engine();
        let f = finishing(&mut e);
        let children = e.mem.alloc(0x20);
        e.mem.set_u32(f.this.addr() + 0x20, children);
        answer(&mut e, 0x0044_ddc0, 2);
        answer(&mut e, 0x0044_edb0, 1);
        e.call_log = Some(vec![]);
        e.call(0x0044_0780, &args![f.this]);
        assert!(calls_to(&e, QUEUE_TABLE_ADD).is_empty());
        assert!(calls_to(&e, BACKGROUND_CLONE_QUEUE_ADD).is_empty());
        // State 6.
        let mut e = engine();
        let f = finishing(&mut e);
        answer(&mut e, TASK_STATE_IS_6, 1);
        e.call_log = Some(vec![]);
        e.call(0x0044_0780, &args![f.this]);
        assert!(calls_to(&e, QUEUE_TABLE_ADD).is_empty());
        assert!(calls_to(&e, BACKGROUND_CLONE_QUEUE_ADD).is_empty());
    }

    #[test]
    fn loader_queue_helper_calls_the_object_at_offset_0x28() {
        let mut e = engine();
        let queue = e.mem.alloc(0x10);
        let loader = loader(&e);
        e.mem.set_u32(loader + 0x28, queue);
        e.register(BACKGROUND_CLONE_QUEUE_ADD, |_, _| Ret::default());
        e.call_log = Some(vec![]);
        e.call(
            0x0044_08d0,
            &args![Ptr::<()>::new(loader), Ptr::<()>::new(0x6600)],
        );
        assert_eq!(
            calls_to(&e, BACKGROUND_CLONE_QUEUE_ADD),
            vec![vec![queue, 0x6600]]
        );
    }

    /// A loader whose object at +8 has the virtual function `0x14` = `SLOT_A`.
    fn observer(e: &mut Engine) -> u32 {
        vtable(e, OTHER_VTABLE, &[(0x14, SLOT_A)]);
        let observer = object(e, OTHER_VTABLE, 0x20);
        let loader = loader(e);
        e.mem.set_u32(loader + 8, observer);
        observer
    }

    #[test]
    fn cancel_takes_the_cloned_3d_off_the_reference_and_lowers_the_cell_counts() {
        let mut e = engine();
        e.register(TASK_SET_STATE_6, |_, _| Ret::default());
        e.register(QUEUED_FILE_CANCEL, |_, _| Ret::default());
        e.register(REFERENCE_SET_3D_VERY_SIMPLE, |_, _| Ret::default());
        decrement(&mut e);
        e.register(REFERENCE_IS_3D_CRITICAL, |_, _| 1u32.into_ret());
        let observer = observer(&mut e);
        let cell = e.new_object::<TESObjectCELL>();
        e.set(cell, TESObjectCELL::iQueuedRefCount, 3);
        e.set(cell, TESObjectCELL::iCriticalQueuedRefCount, 1);
        answer(&mut e, REFERENCE_CELL, cell.addr());
        let reference = reference(&mut e, &[(0x1cc, SLOT_B)], 0);
        let this = task(&mut e, reference);
        e.set(this, QueuedReference::spCloned3D, Ptr::new(0x6700));
        e.set(
            this,
            QueuedReference::spAttachDistant3DTask,
            Ptr::new(0x6800),
        );
        e.call_log = Some(vec![]);
        e.call(0x0044_08f0, &args![this, 11u32, 12u32]);
        assert_eq!(calls_to(&e, TASK_SET_STATE_6), vec![vec![0x6800]]);
        assert_eq!(
            calls_to(&e, QUEUED_FILE_CANCEL),
            vec![vec![this.addr(), 11, 12]]
        );
        assert_eq!(
            calls_to(&e, REFERENCE_SET_3D_VERY_SIMPLE),
            vec![vec![reference.addr(), 0x6700]]
        );
        assert!(e.get(this, QueuedReference::spCloned3D).is_null());
        assert_eq!(calls_to(&e, SLOT_B), vec![vec![reference.addr(), 0, 0]]);
        assert_eq!(calls_to(&e, SLOT_A), vec![vec![observer, reference.addr()]]);
        assert_eq!(e.get(cell, TESObjectCELL::iQueuedRefCount), 2);
        assert_eq!(e.get(cell, TESObjectCELL::iCriticalQueuedRefCount), 0);
    }

    #[test]
    fn cancel_without_pending_work_only_cancels_the_file() {
        let mut e = engine();
        e.register(QUEUED_FILE_CANCEL, |_, _| Ret::default());
        observer(&mut e);
        answer(&mut e, REFERENCE_CELL, 0);
        let reference = reference(&mut e, &[], 0);
        let this = task(&mut e, reference);
        e.call_log = Some(vec![]);
        e.call(0x0044_08f0, &args![this, 1u32, 2u32]);
        assert!(calls_to(&e, TASK_SET_STATE_6).is_empty());
        assert!(calls_to(&e, REFERENCE_SET_3D_VERY_SIMPLE).is_empty());
        assert_eq!(calls_to(&e, QUEUED_FILE_CANCEL).len(), 1);
        assert_eq!(calls_to(&e, SLOT_A).len(), 1);
        assert!(calls_to(&e, INTERLOCKED_DECREMENT).is_empty());
    }

    #[test]
    fn loader_observer_slot_0x14_gets_the_reference() {
        let mut e = engine();
        let observer = observer(&mut e);
        let loader = loader(&e);
        e.call_log = Some(vec![]);
        e.call(
            0x0044_09e0,
            &args![Ptr::<()>::new(loader), Ptr::<()>::new(0x6900)],
        );
        assert_eq!(calls_to(&e, SLOT_A), vec![vec![observer, 0x6900]]);
    }

    #[test]
    fn cell_counters_go_down_and_stop_at_zero() {
        let mut e = engine();
        decrement(&mut e);
        let cell = e.new_object::<TESObjectCELL>();
        e.set(cell, TESObjectCELL::iQueuedRefCount, 3);
        e.call(0x0044_0a10, &args![cell]);
        assert_eq!(e.get(cell, TESObjectCELL::iQueuedRefCount), 2);
        e.set(cell, TESObjectCELL::iQueuedRefCount, 0);
        e.call(0x0044_0a10, &args![cell]);
        assert_eq!(e.get(cell, TESObjectCELL::iQueuedRefCount), 0);
        // The other counter is untouched by the first.
        assert_eq!(e.get(cell, TESObjectCELL::iCriticalQueuedRefCount), 0);
        e.set(cell, TESObjectCELL::iCriticalQueuedRefCount, 5);
        e.call(0x0044_0a50, &args![cell]);
        assert_eq!(e.get(cell, TESObjectCELL::iCriticalQueuedRefCount), 4);
        e.set(cell, TESObjectCELL::iCriticalQueuedRefCount, 0);
        e.call(0x0044_0a50, &args![cell]);
        assert_eq!(e.get(cell, TESObjectCELL::iCriticalQueuedRefCount), 0);
        assert_eq!(e.get(cell, TESObjectCELL::iQueuedRefCount), 0);
    }

    /// Doubles for `BackgroundClone`: a reference whose virtual function
    /// `0x1c8` returns a clone.
    fn background_clone_setup(
        e: &mut Engine,
        loaded: bool,
        accepted: bool,
    ) -> (Ptr<QueuedReference>, u32) {
        answer(e, REFERENCE_CELL, 0x7000);
        let tes = e.mem.alloc(0x10);
        e.set_global(TES_GLOBAL, tes);
        answer(e, TES_IS_CELL_LOADED, loaded as u32);
        answer(e, TES_REFERENCE_PREDICATE, accepted as u32);
        e.register(OBJECT_RELEASE, |_, _| Ret::default());
        let clone = e.mem.alloc(0x20);
        answer(e, ANSWER_A, clone);
        let reference = reference(e, &[(0x1c8, ANSWER_A)], 0);
        (task(e, reference), clone)
    }

    #[test]
    fn background_clone_stores_and_releases_the_clone_of_a_loaded_cell() {
        let mut e = engine();
        let (this, clone) = background_clone_setup(&mut e, true, true);
        e.call_log = Some(vec![]);
        assert!(e.call(0x0044_0a90, &args![this]).bool());
        assert_eq!(e.get(this, QueuedReference::spCloned3D), Ptr::new(clone));
        assert_eq!(calls_to(&e, OBJECT_RELEASE), vec![vec![clone]]);
        let reference = e.get(this, QueuedReference::pRef);
        assert_eq!(calls_to(&e, ANSWER_A), vec![vec![reference.addr(), 1]]);
        assert_eq!(calls_to(&e, MEMORY_CONTEXT_LEAVE).len(), 1);
    }

    #[test]
    fn background_clone_answers_true_for_a_loaded_cell_without_cloning() {
        // The predicate refuses.
        let mut e = engine();
        let (this, _) = background_clone_setup(&mut e, true, false);
        assert!(e.call(0x0044_0a90, &args![this]).bool());
        assert!(e.get(this, QueuedReference::spCloned3D).is_null());
        // A 3D object is already known for the reference.
        let mut e = engine();
        let (this, _) = background_clone_setup(&mut e, true, true);
        let reference = e.get(this, QueuedReference::pRef);
        let tls = e.tls();
        e.mem.set_u32(tls + 0x264, reference.addr());
        e.mem.set_u32(tls + 0x260, 0x6500);
        assert!(e.call(0x0044_0a90, &args![this]).bool());
        assert!(e.get(this, QueuedReference::spCloned3D).is_null());
    }

    #[test]
    fn background_clone_answers_false_for_an_unloaded_cell() {
        let mut e = engine();
        let (this, _) = background_clone_setup(&mut e, false, true);
        e.call_log = Some(vec![]);
        assert!(!e.call(0x0044_0a90, &args![this]).bool());
        assert!(calls_to(&e, TES_REFERENCE_PREDICATE).is_empty());
        assert!(e.get(this, QueuedReference::spCloned3D).is_null());
        assert_eq!(calls_to(&e, MEMORY_CONTEXT_LEAVE).len(), 1);
    }

    /// Doubles for `Attach`: a loaded cell with queued-reference counts, a
    /// model with a root node, and the loader's observer.
    fn attach_setup(
        e: &mut Engine,
        form_flags: u32,
    ) -> (Ptr<QueuedReference>, Ptr<TESObjectCELL>, u32) {
        answer(e, TASK_STATE_IS_6, 0);
        decrement(e);
        let cell = e.new_object::<TESObjectCELL>();
        e.set(cell, TESObjectCELL::iQueuedRefCount, 2);
        e.set(cell, TESObjectCELL::iCriticalQueuedRefCount, 2);
        answer(e, REFERENCE_CELL, cell.addr());
        let tes = e.mem.alloc(0x10);
        e.set_global(TES_GLOBAL, tes);
        answer(e, TES_IS_CELL_LOADED, 1);
        e.register(CELL_ENTER_ATTACH, |_, _| Ret::default());
        e.register(CELL_LEAVE_ATTACH, |_, _| Ret::default());
        e.register(MODEL_LOADER_LOAD_ADDON_NODES, |_, _| Ret::default());
        e.register(TES_LOAD_REFERENCE, |_, _| Ret::default());
        answer(e, REFERENCE_IS_3D_CRITICAL, 0);
        observer(e);
        let reference = reference(e, &[], 0);
        e.mem.set_u32(reference.addr() + 8, form_flags);
        let this = task(e, reference);
        let model = e.mem.alloc(0x20);
        let root = e.mem.alloc(0x20);
        e.mem.set_u32(model + 0xc, root);
        e.set(this, QueuedReference::spModel, Ptr::new(model));
        (this, cell, root)
    }

    #[test]
    fn attach_loads_the_reference_and_lowers_the_cell_count() {
        let mut e = engine();
        let (this, cell, root) = attach_setup(&mut e, 0);
        e.call_log = Some(vec![]);
        e.call(0x0044_0ba0, &args![this]);
        let reference = e.get(this, QueuedReference::pRef);
        let loader = loader(&e);
        assert_eq!(
            calls_to(&e, MODEL_LOADER_LOAD_ADDON_NODES),
            vec![vec![loader, reference.addr(), root]]
        );
        let tes = e.global::<u32>(TES_GLOBAL);
        assert_eq!(
            calls_to(&e, TES_LOAD_REFERENCE),
            vec![vec![tes, reference.addr(), cell.addr(), this.addr(), 0]]
        );
        assert_eq!(e.get(cell, TESObjectCELL::iQueuedRefCount), 1);
        // Not a critical reference.
        assert_eq!(e.get(cell, TESObjectCELL::iCriticalQueuedRefCount), 2);
        assert_eq!(
            order_of(
                &e,
                &[
                    CELL_ENTER_ATTACH,
                    TES_LOAD_REFERENCE,
                    SLOT_A,
                    CELL_LEAVE_ATTACH
                ]
            ),
            vec![
                CELL_ENTER_ATTACH,
                TES_LOAD_REFERENCE,
                SLOT_A,
                CELL_LEAVE_ATTACH
            ]
        );
        assert_eq!(calls_to(&e, MEMORY_CONTEXT_LEAVE).len(), 1);
    }

    #[test]
    fn attach_lowers_the_critical_count_of_a_critical_reference() {
        let mut e = engine();
        let (this, cell, _) = attach_setup(&mut e, 0);
        answer(&mut e, REFERENCE_IS_3D_CRITICAL, 1);
        e.call(0x0044_0ba0, &args![this]);
        assert_eq!(e.get(cell, TESObjectCELL::iQueuedRefCount), 1);
        assert_eq!(e.get(cell, TESObjectCELL::iCriticalQueuedRefCount), 1);
    }

    #[test]
    fn attach_leaves_flagged_references_alone_but_still_lowers_the_count() {
        for flags in [0x20u32, 0x800] {
            let mut e = engine();
            let (this, cell, _) = attach_setup(&mut e, flags);
            e.call_log = Some(vec![]);
            e.call(0x0044_0ba0, &args![this]);
            assert!(calls_to(&e, TES_LOAD_REFERENCE).is_empty());
            assert!(calls_to(&e, MODEL_LOADER_LOAD_ADDON_NODES).is_empty());
            assert_eq!(calls_to(&e, SLOT_A).len(), 1);
            assert_eq!(e.get(cell, TESObjectCELL::iQueuedRefCount), 1);
            assert_eq!(calls_to(&e, CELL_ENTER_ATTACH).len(), 1);
            assert_eq!(calls_to(&e, CELL_LEAVE_ATTACH).len(), 1);
        }
    }

    #[test]
    fn attach_without_a_loaded_cell_or_model_skips_the_loading() {
        let mut e = engine();
        let (this, cell, _) = attach_setup(&mut e, 0);
        answer(&mut e, TES_IS_CELL_LOADED, 0);
        e.call_log = Some(vec![]);
        e.call(0x0044_0ba0, &args![this]);
        assert!(calls_to(&e, TES_LOAD_REFERENCE).is_empty());
        assert_eq!(e.get(cell, TESObjectCELL::iQueuedRefCount), 1);
        // Loaded cell, but no model: no addon nodes, still loaded.
        let mut e = engine();
        let (this, _, _) = attach_setup(&mut e, 0);
        e.set(this, QueuedReference::spModel, Ptr::NULL);
        e.call_log = Some(vec![]);
        e.call(0x0044_0ba0, &args![this]);
        assert!(calls_to(&e, MODEL_LOADER_LOAD_ADDON_NODES).is_empty());
        assert_eq!(calls_to(&e, TES_LOAD_REFERENCE).len(), 1);
    }

    #[test]
    fn attach_does_nothing_in_state_6() {
        let mut e = engine();
        let (this, cell, _) = attach_setup(&mut e, 0);
        answer(&mut e, TASK_STATE_IS_6, 1);
        e.call_log = Some(vec![]);
        e.call(0x0044_0ba0, &args![this]);
        assert!(calls_to(&e, CELL_ENTER_ATTACH).is_empty());
        assert!(calls_to(&e, SLOT_A).is_empty());
        assert_eq!(e.get(cell, TESObjectCELL::iQueuedRefCount), 2);
        assert_eq!(calls_to(&e, MEMORY_CONTEXT_LEAVE).len(), 1);
    }

    #[test]
    fn form_flag_tests_look_at_bits_0x20_and_0x800() {
        let mut e = engine();
        let form = e.new_object::<TESForm>();
        for (flags, bit_20, bit_800) in [
            (0u32, false, false),
            (0x20, true, false),
            (0x800, false, true),
            (0xffff_f7df, false, false),
        ] {
            e.set(form, TESForm::iFormFlags, flags);
            assert_eq!(e.call(0x0044_0d80, &args![form]).bool(), bit_20);
            assert_eq!(e.call(0x0044_0da0, &args![form]).bool(), bit_800);
        }
    }

    #[test]
    fn form_type_name_comes_from_the_table() {
        let mut e = engine();
        e.set_global(FORM_TYPE_TABLE + 12 * 4, 0x0101_c600u32);
        e.set_global(FORM_TYPE_TABLE + 12 * 5, 0x0101_c700u32);
        let form = e.new_object::<TESForm>();
        e.set(form, TESForm::cFormType, 4);
        assert_eq!(e.call(0x0044_0e30, &args![form]).u32(), 0x0101_c600);
        e.set(form, TESForm::cFormType, 5);
        assert_eq!(e.call(0x0044_0e30, &args![form]).u32(), 0x0101_c700);
    }

    #[test]
    fn get_description_formats_the_reference_and_its_type() {
        let mut e = engine();
        let form = e.new_object::<TESForm>();
        e.set(form, TESForm::cFormType, 4);
        e.set_global(FORM_TYPE_TABLE + 12 * 4, 0x0101_c600u32);
        answer(&mut e, GET_BASE_FORM, form.addr());
        answer(&mut e, GET_FORM_ID, 0x000f_00aa);
        answer(&mut e, ANSWER_A, 0x6a00);
        e.register(FORMAT_STRING, |_, _| Ret::default());
        answer(&mut e, TASK_STATE_IS_6, 0);
        let reference = reference(&mut e, &[(0x130, ANSWER_A)], 0);
        let this = task(&mut e, reference);
        e.call_log = Some(vec![]);
        assert!(e.call(0x0044_0dc0, &args![this, 0x6b00u32, 0x80u32]).bool());
        assert_eq!(
            calls_to(&e, FORMAT_STRING),
            vec![vec![
                0x6b00,
                0x80,
                QUEUED_REF_FORMAT,
                0x6a00,
                0x000f_00aa,
                0x0101_c600
            ]]
        );
        // A task in state 6 has no description.
        answer(&mut e, TASK_STATE_IS_6, 1);
        e.call_log = Some(vec![]);
        assert!(!e.call(0x0044_0dc0, &args![this, 0x6b00u32, 0x80u32]).bool());
        assert!(calls_to(&e, FORMAT_STRING).is_empty());
    }

    #[test]
    fn tree_and_actor_constructors_set_their_tables() {
        let mut e = engine();
        e.register(QUEUED_FILE_CONSTRUCT, |_, _| Ret::default());
        for (address, table) in [
            (0x0044_0e50u32, QUEUED_TREE_VTABLE),
            (0x0044_11d0, QUEUED_ACTOR_VTABLE),
        ] {
            let this = e.new_object::<QueuedReference>();
            e.call_log = Some(vec![]);
            let back = e
                .call(address, &args![this, 0x7100u32, 0x1eu32])
                .ptr::<QueuedReference>();
            assert_eq!(back, this);
            assert_eq!(e.mem.u32(this.addr()), table);
            // The `QueuedReference` constructor ran with the reference and
            // the context.
            assert_eq!(e.get(this, QueuedReference::pRef), Ptr::new(0x7100));
            assert_eq!(
                calls_to(&e, QUEUED_FILE_CONSTRUCT),
                vec![vec![this.addr(), 0x1e]]
            );
        }
    }

    #[test]
    fn character_constructor_adds_two_empty_task_pointers() {
        let mut e = engine();
        e.register(QUEUED_FILE_CONSTRUCT, |_, _| Ret::default());
        let this = e.new_object::<QueuedCharacter>();
        e.mem.set_u32(this.addr() + 0x40, 0xdead);
        e.mem.set_u32(this.addr() + 0x44, 0xbeef);
        e.call_log = Some(vec![]);
        let back = e
            .call(0x0044_1440, &args![this, 0x7100u32, 0x1eu32])
            .ptr::<QueuedCharacter>();
        assert_eq!(back, this);
        assert_eq!(e.mem.u32(this.addr()), QUEUED_CHARACTER_VTABLE);
        assert_eq!(e.get(this, QueuedCharacter::pRef), Ptr::new(0x7100));
        assert!(e.get(this, QueuedCharacter::spQueuedHead).is_null());
        assert!(e.get(this, QueuedCharacter::spQueuedHelmet).is_null());
        let own: Vec<_> = calls_to(&e, TASK_POINTER_CONSTRUCT)
            .into_iter()
            .filter(|a| a[0] >= this.addr() + 0x40)
            .collect();
        assert_eq!(
            own,
            vec![vec![this.addr() + 0x40, 0], vec![this.addr() + 0x44, 0]]
        );
    }

    #[test]
    fn tree_destructor_and_scalar_deleting_destructor() {
        let mut e = engine();
        e.register(QUEUED_FILE_DESTRUCT, |_, _| Ret::default());
        e.register(MODEL_POINTER_RELEASE, |_, _| Ret::default());
        let this = e.new_object::<QueuedReference>();
        e.call_log = Some(vec![]);
        e.call(0x0044_0eb0, &args![this]);
        // The `QueuedReference` destructor ran.
        assert_eq!(calls_to(&e, QUEUED_FILE_DESTRUCT), vec![vec![this.addr()]]);
        assert_eq!(calls_to(&e, MODEL_POINTER_RELEASE).len(), 1);
        assert!(calls_to(&e, MEMORY_FREE).is_empty());
        e.call(0x0044_0e80, &args![this, 1u32]);
        assert_eq!(calls_to(&e, MEMORY_FREE), vec![vec![this.addr()]]);
        assert_eq!(calls_to(&e, QUEUED_FILE_DESTRUCT).len(), 2);
        assert_eq!(e.call(0x0044_0e80, &args![this, 0u32]).u32(), this.addr());
        assert_eq!(calls_to(&e, MEMORY_FREE).len(), 1);
    }

    /// Doubles for the queueing of trees; `first` and `second` are the
    /// answers of the two reference predicates.
    fn tree_queue_setup(e: &mut Engine, first: u32, second: u32) -> Ptr<QueuedReference> {
        base_form_double(e);
        answer(e, GET_LOD_MULT, 3);
        answer(e, REFERENCE_TEST_00564E60, first);
        answer(e, REFERENCE_TEST_00564F00, second);
        e.register(MODEL_LOADER_QUEUE_TREE_MODEL, |_, _| Ret::default());
        let reference = reference(e, &[], 0x5000);
        task(e, reference)
    }

    #[test]
    fn tree_queue_models_picks_the_lod_multiplier() {
        for (first, second, lod) in [(0u32, 0u32, 3u32), (1, 0, 6), (0, 1, 0xa), (1, 1, 0xa)] {
            let mut e = engine();
            let this = tree_queue_setup(&mut e, first, second);
            let reference = e.get(this, QueuedReference::pRef);
            e.call_log = Some(vec![]);
            e.call(0x0044_0ed0, &args![this]);
            let loader = loader(&e);
            assert_eq!(
                calls_to(&e, MODEL_LOADER_QUEUE_TREE_MODEL),
                vec![vec![
                    loader,
                    reference.addr(),
                    0x5000,
                    PRIORITY as u32,
                    this.addr(),
                    lod
                ]]
            );
            assert_eq!(
                calls_to(&e, MEMORY_CONTEXT_ENTER)[0][1..].to_vec(),
                vec![CONTEXT, 1, MODEL_LOADER_SOURCE, 0x847]
            );
            assert_eq!(calls_to(&e, MEMORY_CONTEXT_LEAVE).len(), 1);
        }
    }

    #[test]
    fn tree_queue_models_without_a_reference_keeps_the_base_multiplier() {
        let mut e = engine();
        let this = tree_queue_setup(&mut e, 1, 1);
        e.set(this, QueuedReference::pRef, Ptr::NULL);
        // The base form getter reads the reference unconditionally.
        answer(&mut e, GET_BASE_FORM, 0);
        e.call_log = Some(vec![]);
        e.call(0x0044_0ed0, &args![this]);
        let queued = calls_to(&e, MODEL_LOADER_QUEUE_TREE_MODEL);
        assert_eq!(queued[0][5], 3);
        assert!(calls_to(&e, REFERENCE_TEST_00564E60).is_empty());
    }

    /// Doubles for `QueuedTree::UseDistant3D`: a distant node with one child,
    /// geometry data, and a property of type `property_type` (0 for none).
    /// Returns the task, the node, the geometry data, the child and the
    /// property.
    fn tree_distant_setup(
        e: &mut Engine,
        property_type: u32,
    ) -> (Ptr<QueuedReference>, u32, u32, u32, u32) {
        base_form_double(e);
        let node = e.mem.alloc(0xb0);
        let child = e.mem.alloc(0x40);
        // The array at +0x9c: element address = array[1] + index * 4.
        let elements = e.mem.alloc(0x10);
        e.mem.set_u32(elements, child);
        e.mem.set_u32(node + 0x9c + 4, elements);
        e.register(NODE_ARRAY_ELEMENT, |e, a| {
            (e.mem.u32(a[0] + 4) + a[1] * 4).into_ret()
        });
        answer(e, BUILD_DISTANT_3D, node);
        let data = e.mem.alloc(0x20);
        answer(e, OBJECT_POINTER_AT_B8, data);
        for address in [
            SET_CONSISTENCY,
            GEOMETRY_DATA_SETTER_FIRST,
            GEOMETRY_DATA_SETTER_SECOND,
            PREPARE_OBJECT,
        ] {
            e.register(address, |_, _| Ret::default());
        }
        let property = e.mem.alloc(0x60);
        answer(
            e,
            GET_PROPERTY,
            if property_type == 0 { 0 } else { property },
        );
        answer(e, PROPERTY_GET_TYPE, property_type);
        let reference = reference(e, &[], 0x5000);
        (task(e, reference), node, data, child, property)
    }

    #[test]
    fn tree_use_distant_3d_prepares_the_child_and_sets_the_shader_flags() {
        let mut e = engine();
        let (this, node, data, child, property) = tree_distant_setup(&mut e, 0xb);
        e.mem.set_u32(property + 0x20, 1);
        e.mem.set_u32(property + 0x38, 0x55);
        e.call_log = Some(vec![]);
        e.call(0x0044_0fd0, &args![this]);
        assert_eq!(calls_to(&e, BUILD_DISTANT_3D), vec![vec![0x5000, 0]]);
        assert_eq!(calls_to(&e, OBJECT_POINTER_AT_B8), vec![vec![child]]);
        assert_eq!(calls_to(&e, SET_CONSISTENCY), vec![vec![data, 0x4000]]);
        assert_eq!(
            calls_to(&e, GEOMETRY_DATA_SETTER_FIRST),
            vec![vec![data, 0x11]]
        );
        assert_eq!(
            calls_to(&e, GEOMETRY_DATA_SETTER_SECOND),
            vec![vec![data, 1]]
        );
        assert_eq!(calls_to(&e, PREPARE_OBJECT), vec![vec![child, 0, 0]]);
        // Bits 0xd and 0x16 of the first flag dword, besides the one set.
        assert_eq!(e.mem.u32(property + 0x20), 1 | (1 << 0xd) | (1 << 0x16));
        assert_eq!(e.mem.u32(property + 0x38), 0);
        assert_eq!(calls_to(&e, SLOT_D), vec![vec![this.addr(), node]]);
    }

    #[test]
    fn tree_use_distant_3d_leaves_other_properties_and_missing_nodes_alone() {
        // A property of another type.
        let mut e = engine();
        let (this, node, _, _, property) = tree_distant_setup(&mut e, 0xc);
        e.call_log = Some(vec![]);
        e.call(0x0044_0fd0, &args![this]);
        assert_eq!(e.mem.u32(property + 0x20), 0);
        assert_eq!(calls_to(&e, SLOT_D), vec![vec![this.addr(), node]]);
        // No property at all.
        let mut e = engine();
        let (this, node, _, _, _) = tree_distant_setup(&mut e, 0);
        e.call_log = Some(vec![]);
        e.call(0x0044_0fd0, &args![this]);
        assert!(calls_to(&e, PROPERTY_GET_TYPE).is_empty());
        assert_eq!(calls_to(&e, SLOT_D), vec![vec![this.addr(), node]]);
        // No distant 3D.
        let mut e = engine();
        let (this, _, _, _, _) = tree_distant_setup(&mut e, 0xb);
        answer(&mut e, BUILD_DISTANT_3D, 0);
        e.call_log = Some(vec![]);
        e.call(0x0044_0fd0, &args![this]);
        assert!(calls_to(&e, SET_CONSISTENCY).is_empty());
        assert!(calls_to(&e, SLOT_D).is_empty());
    }

    #[test]
    fn shader_property_flags_are_set_and_cleared_by_bit_number() {
        let mut e = engine();
        let property = e.mem.alloc(0x60);
        let flags = |e: &Engine, word: u32| e.mem.u32(property + 0x20 + 4 * word);
        let this = Ptr::<()>::new(property);
        e.mem.set_u32(property + 0x38, 9);
        // Setting a clear bit clears the dword at +0x38.
        e.call(0x0044_1130, &args![this, 0x23u32, 1u8]);
        assert_eq!(flags(&e, 1), 1 << 3);
        assert_eq!(flags(&e, 0), 0);
        assert_eq!(e.mem.u32(property + 0x38), 0);
        // Setting an already set bit leaves it.
        e.mem.set_u32(property + 0x38, 9);
        e.call(0x0044_1130, &args![this, 0x23u32, 1u8]);
        assert_eq!(flags(&e, 1), 1 << 3);
        assert_eq!(e.mem.u32(property + 0x38), 9);
        // Clearing a clear bit does nothing.
        e.call(0x0044_1130, &args![this, 0x24u32, 0u8]);
        assert_eq!(e.mem.u32(property + 0x38), 9);
        assert_eq!(flags(&e, 1), 1 << 3);
        // Clearing a set bit clears it and the dword at +0x38.
        e.mem.set_u32(property + 0x20 + 4 * 2, 0xffff_ffff);
        e.call(0x0044_1130, &args![this, 0x5fu32, 0u8]);
        assert_eq!(flags(&e, 2), 0x7fff_ffff);
        assert_eq!(e.mem.u32(property + 0x38), 0);
    }

    /// Doubles for `QueuedActor::QueueMe`; the virtual function `0x28` of the
    /// task (the end of `QueuedReference::QueueMe`) records the thread flag.
    fn actor_queue_me_setup(
        e: &mut Engine,
        special: bool,
        accepted: u32,
        actor: u32,
    ) -> Ptr<QueuedReference> {
        e.register(REFERENCE_TEST_00564E60, |_, _| 0u32.into_ret());
        e.register(REFERENCE_EXTRA_LIST, |_, a| (a[0] + 0x44).into_ret());
        e.register(EXTRA_GET_DECAL_REFS, |_, _| 0u32.into_ret());
        e.register(TASK_STATE_IS_0, |_, _| 0u32.into_ret());
        answer(e, REFERENCE_TEST_00936EF0, accepted);
        answer(e, ACTOR_TEST_008B7B70, actor);
        e.register(ACTOR_ATTACH_TASK, |_, _| Ret::default());
        let reference = reference(e, &[(0x260, NOTHING)], 0);
        if special {
            e.set_global(SPECIAL_REFERENCE, reference.addr());
        }
        let this = task(e, reference);
        e.register(SLOT_A, |e, _| {
            let tls = e.tls();
            let flag = e.mem.u8(tls + TLS_QUEUED_FLAG) as u32;
            e.mem.set_u32(OBSERVED, flag);
            Ret::default()
        });
        let tls = e.tls();
        e.mem.set_u8(tls + TLS_QUEUED_FLAG, 9);
        this
    }

    #[test]
    fn actor_queue_me_raises_the_thread_flag_around_the_reference_task() {
        let mut e = engine();
        let this = actor_queue_me_setup(&mut e, false, 0, 1);
        e.call_log = Some(vec![]);
        e.call(0x0044_1200, &args![this]);
        // The flag was 1 while the task ran, and is put back.
        assert_eq!(e.mem.u32(OBSERVED), 1);
        let tls = e.tls();
        assert_eq!(e.mem.u8(tls + TLS_QUEUED_FLAG), 9);
        let reference = e.get(this, QueuedReference::pRef);
        assert_eq!(calls_to(&e, NOTHING), vec![vec![reference.addr()]]);
        assert_eq!(
            calls_to(&e, ACTOR_ATTACH_TASK),
            vec![vec![reference.addr(), 1, this.addr()]]
        );
        assert_eq!(calls_to(&e, MEMORY_CONTEXT_LEAVE).len(), 1);
    }

    #[test]
    fn actor_queue_me_keeps_the_flag_for_the_special_or_accepted_reference() {
        // The special reference.
        let mut e = engine();
        let this = actor_queue_me_setup(&mut e, true, 0, 0);
        e.call_log = Some(vec![]);
        e.call(0x0044_1200, &args![this]);
        assert_eq!(e.mem.u32(OBSERVED), 9);
        assert!(calls_to(&e, ACTOR_ATTACH_TASK).is_empty());
        // A reference that 00936ef0 accepts.
        let mut e = engine();
        let this = actor_queue_me_setup(&mut e, false, 0x1234, 0);
        e.call(0x0044_1200, &args![this]);
        assert_eq!(e.mem.u32(OBSERVED), 9);
        let tls = e.tls();
        assert_eq!(e.mem.u8(tls + TLS_QUEUED_FLAG), 9);
    }

    #[test]
    fn thread_flag_swap_returns_the_old_value() {
        let mut e = engine();
        let tls = e.tls();
        e.mem.set_u8(tls + 0x25c, 4);
        assert_eq!(e.call(0x0044_1290, &args![Ptr::<()>::new(0), 1u8]).u8(), 4);
        assert_eq!(e.mem.u8(tls + 0x25c), 1);
        assert_eq!(e.call(0x0044_1290, &args![Ptr::<()>::new(0), 0u8]).u8(), 1);
        assert_eq!(e.mem.u8(tls + 0x25c), 0);
    }

    #[test]
    fn dismemberment_entry_is_read_from_the_array_at_0x20() {
        let mut e = engine();
        e.register(ARRAY_ELEMENT_ADDRESS, |e, a| {
            (e.mem.u32(a[0] + 4) + a[1] * 4).into_ret()
        });
        let extra = e.mem.alloc(0x40);
        let elements = e.mem.alloc(0x10);
        e.mem.set_u32(elements + 4, 0x7a7a);
        e.mem.set_u32(extra + 0x20 + 4, elements);
        e.call_log = Some(vec![]);
        assert_eq!(
            e.call(0x0044_1420, &args![Ptr::<()>::new(extra), 1u32])
                .u32(),
            0x7a7a
        );
        assert_eq!(
            calls_to(&e, ARRAY_ELEMENT_ADDRESS),
            vec![vec![extra + 0x20, 1]]
        );
    }

    #[test]
    fn actor_queue_models_queues_every_dismemberment_part_then_the_models() {
        let mut e = engine();
        e.register(ARRAY_ELEMENT_ADDRESS, |e, a| {
            (e.mem.u32(a[0] + 4) + a[1] * 4).into_ret()
        });
        e.register(ARRAY_COUNT_AT_8, |e, a| e.mem.u32(a[0] + 8).into_ret());
        e.register(ACTOR_PREPARE, |_, _| Ret::default());
        e.register(REFERENCE_EXTRA_LIST, |_, a| (a[0] + 0x44).into_ret());
        // The extra: 2 entries at +0x20; entry 0 has 2 parts, entry 1 one.
        let extra = e.mem.alloc(0x40);
        answer(&mut e, EXTRA_GET_DISMEMBERMENT, extra);
        let entries = e.mem.alloc(0x10);
        e.mem.set_u32(extra + 0x20 + 8, 2);
        e.mem.set_u32(extra + 0x20 + 4, entries);
        let entry_a = e.mem.alloc(0x20);
        let entry_b = e.mem.alloc(0x20);
        e.mem.set_u32(entries, entry_a);
        e.mem.set_u32(entries + 4, entry_b);
        let parts_a = e.mem.alloc(0x10);
        let parts_b = e.mem.alloc(0x10);
        e.mem.set_u32(parts_a, 0xa1);
        e.mem.set_u32(parts_a + 4, 0xa2);
        e.mem.set_u32(parts_b, 0xb1);
        e.mem.set_u32(entry_a + 4 + 8, 2);
        e.mem.set_u32(entry_a + 4 + 4, parts_a);
        e.mem.set_u32(entry_b + 4 + 8, 1);
        e.mem.set_u32(entry_b + 4 + 4, parts_b);
        e.register(MODEL_LOADER_QUEUE_DISMEMBER_PART, |_, _| Ret::default());
        // QueuedReference::QueueModels finds no TESModel here.
        base_form_double(&mut e);
        answer(&mut e, REFERENCE_GET_TES_MODEL, 0);
        let reference = reference(&mut e, &[], 0x5000);
        let this = task(&mut e, reference);
        e.call_log = Some(vec![]);
        e.call(0x0044_12e0, &args![this]);
        let loader = loader(&e);
        let queued = calls_to(&e, MODEL_LOADER_QUEUE_DISMEMBER_PART);
        assert_eq!(
            queued.iter().map(|a| a[1]).collect::<Vec<_>>(),
            vec![0xa1, 0xa2, 0xb1]
        );
        assert_eq!(
            queued[0],
            vec![loader, 0xa1, PRIORITY as u32, this.addr(), reference.addr()]
        );
        assert_eq!(calls_to(&e, ACTOR_PREPARE), vec![vec![reference.addr()]]);
        // The memory context is entered twice, with the lines of the two
        // functions, the inner one being `QueuedReference::QueueModels`.
        assert_eq!(
            calls_to(&e, MEMORY_CONTEXT_ENTER)
                .iter()
                .map(|a| a[4])
                .collect::<Vec<_>>(),
            vec![0x8a2, QUEUE_MODELS_SOURCE_LINE]
        );
        assert_eq!(calls_to(&e, REFERENCE_GET_TES_MODEL).len(), 1);
    }

    #[test]
    fn actor_queue_models_without_a_dismemberment_extra_queues_the_models_only() {
        let mut e = engine();
        e.register(ACTOR_PREPARE, |_, _| Ret::default());
        e.register(REFERENCE_EXTRA_LIST, |_, a| (a[0] + 0x44).into_ret());
        answer(&mut e, EXTRA_GET_DISMEMBERMENT, 0);
        e.register(MODEL_LOADER_QUEUE_DISMEMBER_PART, |_, _| Ret::default());
        base_form_double(&mut e);
        answer(&mut e, REFERENCE_GET_TES_MODEL, 0);
        let reference = reference(&mut e, &[], 0x5000);
        let this = task(&mut e, reference);
        e.call_log = Some(vec![]);
        e.call(0x0044_12e0, &args![this]);
        assert!(calls_to(&e, MODEL_LOADER_QUEUE_DISMEMBER_PART).is_empty());
        assert_eq!(calls_to(&e, REFERENCE_GET_TES_MODEL).len(), 1);
    }
}
