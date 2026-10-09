//! `fallout shared/animation.cpp` (Xbox PDB source unit), subsystem `fallout shared`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! The unit holds the animation sequence containers (`AnimSequenceBase`
//! and its two subclasses) and the `Animation` object every animated
//! reference owns. Session 1 (b0010) covers `0048ee70` to `00491090`, the
//! first 40 functions: the sequence containers, `Animation`'s constructor
//! and destructor, and `AddAnimation` with the functions that feed it. The
//! next session continues at `004910d0` (`Animation::FindSkinnedNode`).
//!
//! Conventions this file uses, so the next session finds them:
//!
//! - Tiny functions of other units that the game reaches by address are
//!   called by address through the constants below (`READ_WORD`,
//!   `NI_POINTER_*`, the list accessors). `00559450` is the one the linker
//!   folded into dozens of getters (`NiPointer::operator T*`,
//!   `NiTList::GetHeadPos`, `KFModel` and `NiTMapBase` field readers): it
//!   returns the word at the address in `ECX`.
//! - Functions of this unit that are translated here are called directly;
//!   the ones further on (`00498910`, `0049c170`, `0049c250`, `0049c390`,
//!   `004946a0`, ...) by address until a later session translates them.
//! - The compiler's exception-unwinding frames and security cookies are
//!   not translated. A local the game keeps on its stack and passes by
//!   address is [`Engine::with_stack`].
//! - Stack arguments a callee declares but never reads (the second word of
//!   the `AnimSequenceBase` virtual functions) are parameters named
//!   `_unused_*`, so the uniform form accepts what the callers push.

#[allow(unused_imports)]
use crate::prelude::*;
use crate::types::BSSimpleList;
use crate::units::fallout_shared::extradataobjects::NiPoint3;

// ---- Callees outside this unit ---------------------------------------------

/// `operator new` (`00401000`), `operator delete` (`00401030`).
const OPERATOR_NEW: u32 = 0x0040_1000;
const OPERATOR_DELETE: u32 = 0x0040_1030;
/// `NiMemObject::operator new(size)`, the block comes back in EAX.
const NI_OPERATOR_NEW: u32 = 0x00aa_13e0;
/// Returns the word at the address in `ECX` (see the module comment).
const READ_WORD: u32 = 0x0055_9450;
/// `NiPointer::NiPointer(T*)`: stores the pointer and adds a reference.
const NI_POINTER_INIT: u32 = 0x0063_3c90;
/// `NiPointer::operator=(T*)`: releases the old, stores and adds the new.
const NI_POINTER_SET: u32 = 0x0066_b0d0;
/// `NiPointer::~NiPointer`: releases the pointed-to object.
const NI_POINTER_RELEASE: u32 = 0x0045_cec0;
/// `NiRefObject::IncRefCount` and `DecRefCount` (deletes at zero).
const REF_ADD: u32 = 0x0040_f6e0;
const REF_RELEASE: u32 = 0x0040_1970;
/// `NiObjectNET::GetName` (address of the `NiFixedString` at +8), and
/// `NiFixedString::operator const char*`.
const OBJECT_NAME: u32 = 0x0041_3f40;
const FIXED_STRING_CSTR: u32 = 0x0043_b1b0;
/// `NiFixedString::NiFixedString(const char*)` and its destructor.
const FIXED_STRING_INIT: u32 = 0x0043_8170;
const FIXED_STRING_FREE: u32 = 0x0043_81b0;

/// `strcpy_s(dst, size, src)` wrapper, `sprintf_s(dst, size, fmt, ...)`
/// wrapper, `strcpy`, `strcat`, `strchr`, `strrchr`, `_stricmp`.
const STRCPY_S: u32 = 0x0040_6d30;
const SPRINTF_S: u32 = 0x0040_6d00;
const STRCPY: u32 = 0x0040_46f0;
const STRCAT: u32 = 0x00ec_6380;
const STRCHR: u32 = 0x0043_b9d0;
const STRRCHR: u32 = 0x00ec_6e30;
const STRICMP: u32 = 0x0040_4dc0;
/// `memset(dst, value, size)` wrapper.
const MEMSET: u32 = 0x0040_3d30;
/// `_eh_vector_constructor_iterator_(ptr, size, count, ctor, dtor)` and
/// `_eh_vector_destructor_iterator_(ptr, size, count, dtor)`.
const VECTOR_CONSTRUCT: u32 = 0x00ec_782f;
/// The per-element constructor `Animation` hands to it (`NiPointer()` with
/// a null pointer).
const NI_POINTER_DEFAULT_CONSTRUCT: u32 = 0x0066_94e0;
const VECTOR_DESTRUCT: u32 = 0x00ec_5fce;

/// `NiTList<T>` (the sequence list of `AnimSequenceMultiple`): first
/// position (`READ_WORD`), next position, address of a position's item,
/// element count, add at head, remove by item address, is empty, and the
/// scalar deleting destructor `NiTList::~NiTList(flags)`'s inner body.
const LIST_HEAD: u32 = READ_WORD;
const LIST_NEXT: u32 = 0x007b_52d0;
const LIST_ITEM_ADDRESS: u32 = 0x0063_17a0;
const LIST_COUNT: u32 = 0x0044_ddc0;
const LIST_ADD_HEAD: u32 = 0x0055_97d0;
const LIST_REMOVE: u32 = 0x0049_c0b0;
const LIST_IS_EMPTY: u32 = 0x0076_b610;
const LIST_DESTRUCT: u32 = 0x0054_da00;
/// The empty `NiTList` constructor (the linker folded it with an interface
/// manager constructor).
const LIST_CONSTRUCT: u32 = 0x0071_8d90;

/// `NiPoint3::NiPoint3()` (`006815c0`, shared with the `BSSimpleList`
/// item accessor: both return `ECX`).
const NI_POINT3_CONSTRUCT: u32 = 0x0068_15c0;
/// `KFModel` release `~Animation` runs on every model of `kfModelList`.
const KF_MODEL_RELEASE: u32 = 0x0043_bac0;
/// `TESObjectREFR::GetBaseForm`-style getter and the form-type getter
/// `fn_0048ffd0` runs on its reference (the maps name `007af430`
/// `BGSSaveFormBuffer::GetForm`, a folded name).
const REFERENCE_FORM: u32 = 0x007a_f430;
const FORM_TYPE: u32 = 0x0040_1170;
/// The form type `fn_0048ffd0` compares the reference's form with.
const FORM_TYPE_COMPARED: u32 = 0x2a;
/// `NiNode` helper `fn_0048ffd0` calls on the accumulation root with the
/// `NiRTTI` at `011a9448`.
const ACCUM_ROOT_SETUP: u32 = 0x0043_fa80;
const ACCUM_ROOT_SETUP_ARGUMENT: u32 = 0x011a_9448;
/// `ShouldQueueSequenceCloning`'s inputs: the player singleton pointer,
/// `PlayerCharacter::GetNode(first)` (`00950bb0`), the setting-value
/// address getter (`0043d4d0`) and the setting `011c5778` that turns
/// cloning on, `00822510` (the node accepts an object), and the other
/// flag objects.
const PLAYER_SINGLETON: u32 = 0x011d_ea3c;
const PLAYER_NODE: u32 = 0x0095_0bb0;
const SETTING_VALUE_ADDRESS: u32 = 0x0043_d4d0;
const SETTING_CLONING: u32 = 0x011c_5778;
const NODE_ACCEPTS_OBJECT: u32 = 0x0082_2510;
const FLAG_OBJECT_GLOBAL: u32 = 0x011d_df38;
const FLAG_OBJECT_CHECK: u32 = 0x0042_ce10;
const PLAYER_REFERENCE: u32 = 0x0047_b200;
const TES_GLOBAL: u32 = 0x011d_ea10;
const TES_CHECK: u32 = 0x0045_1530;
/// `fn_00491090`: begin and end time getters of the sequence.
const SEQUENCE_BEGIN_TIME: u32 = 0x0075_9450;
const SEQUENCE_END_TIME: u32 = 0x0050_8100;
/// `BSSimpleList<T>`: constructor, address of a node's item (the node
/// itself), next node, is empty, remove the head, remove all, destructor,
/// push back (pointer to the item), and the list's scalar deleting
/// destructor.
const SIMPLE_LIST_CONSTRUCT: u32 = 0x0096_a2d0;
const SIMPLE_LIST_ITEM: u32 = 0x0068_15c0;
const SIMPLE_LIST_NEXT: u32 = 0x0072_6070;
const SIMPLE_LIST_IS_EMPTY: u32 = 0x0082_56d0;
const SIMPLE_LIST_POP_FRONT: u32 = 0x0063_f7b0;
const SIMPLE_LIST_CLEAR: u32 = 0x0047_0470;
const SIMPLE_LIST_DESTRUCT: u32 = 0x0046_ffb0;
const SIMPLE_LIST_PUSH_BACK: u32 = 0x0090_5820;
const SIMPLE_LIST_DELETE: u32 = 0x0047_02f0;

/// `NiTMapBase<unsigned short, AnimSequenceBase*>` (Xbox PDB name of
/// `GetAt`): constructor with the hash size, `GetAt(key, &value)`,
/// `SetAt(key, value)`, `RemoveAt(key)`, first position, next
/// (`GetNext(&pos, &key, &value)`), and the clear-all.
const MAP_CONSTRUCT: u32 = 0x0049_c050;
const MAP_GET_AT: u32 = 0x0049_c390;
const MAP_SET_AT: u32 = 0x0049_c170;
const MAP_REMOVE_AT: u32 = 0x0049_c250;
const MAP_FIRST_POSITION: u32 = 0x004b_9ba0;
const MAP_GET_NEXT: u32 = 0x0049_c410;
const MAP_REMOVE_ALL: u32 = 0x0043_8af0;

/// The 4-byte scope guard the game keeps on its stack (sets the
/// memory-allocation tag to its first argument for its lifetime).
const SCOPE_GUARD_CTOR: u32 = 0x0040_4eb0;
const SCOPE_GUARD_DTOR: u32 = 0x0040_4ee0;
const SCOPE_GUARD_SIZE: u32 = 4;
const SCOPE_GUARD_TAG: u32 = 0x33;
/// `"D:\\_Fallout3\\Platforms\\Common\\Code\\Fallout Shared\\Animation.cpp"`.
const SOURCE_FILE: u32 = 0x0101_d930;

/// The `ModelLoader` singleton pointer (`ECX` of the loader's methods).
const MODEL_LOADER: u32 = 0x011c_3b3c;
/// `ModelLoader::RemoveModel(name, 1)` (body of [`fn_0048eff0`]).
const MODEL_LOADER_REMOVE: u32 = 0x0044_5370;
/// `ModelLoader::LoadKF(name)`, `CancelReplacementKFList(animation)`.
const MODEL_LOADER_LOAD_KF: u32 = 0x0044_71c0;
const MODEL_LOADER_CANCEL_REPLACEMENT_KF_LIST: u32 = 0x0044_5430;
/// `KFModel` field getters: its sequence (+4) and its animation group (+8),
/// the second one adds a reference when the group is kept (`0043be10`).
const KF_MODEL_SEQUENCE: u32 = 0x007f_a950;
const KF_MODEL_ANIM_GROUP: u32 = 0x0055_85e0;
const KF_MODEL_ADD_REF: u32 = 0x0043_be10;

/// `ANIM_GROUP` helpers: the group's sequence type (`TESAnimGroup` +0x10
/// run through the type table), and the group id (`TESAnimGroup` +0x10).
const ANIM_GROUP_SEQUENCE_TYPE: u32 = 0x005f_2420;
const ANIM_GROUP_ID: u32 = 0x0047_d3b0;
/// The per-sequence-type table: 0x24 bytes per type, its first byte says
/// the type keeps several sequences (an `AnimSequenceMultiple`).
const SEQUENCE_TYPE_TABLE: u32 = 0x0119_77dc;
/// The 8 pointers to the names of the sound priority bones.
const SOUND_PRIORITY_BONE_NAMES: u32 = 0x0119_9bcc;

/// `NiControllerSequence` helpers: cycle type (+0x24), number of objects,
/// the name of object `i` (`GetObjectNameAt(i, buffer)`), and
/// `BSAnimGroupSequence::BSAnimGroupSequence(group, sequence)`.
const SEQUENCE_CYCLE_TYPE: u32 = 0x0059_bb30;
const SEQUENCE_OBJECT_COUNT: u32 = 0x0084_e3a0;
const SEQUENCE_OBJECT_NAME_AT: u32 = 0x00a3_07c0;
const ANIM_GROUP_SEQUENCE_CONSTRUCT: u32 = 0x004e_e9f0;
/// `NiRTTI` of `BSAnimGroupSequence` and `NiObject::IsKindOf` on it
/// (`__cdecl(rtti, object)`).
const RTTI_ANIM_GROUP_SEQUENCE: u32 = 0x011c_7d74;
const IS_KIND_OF: u32 = 0x0043_b300;
/// The root name passed to `NiControllerManager::AddSequence`.
const ACCUM_ROOT_NAME: u32 = 0x011f_49e0;

/// `NiControllerManager` methods: `NiControllerManager(root, cumulative)`,
/// `AddSequence(sequence, rootName, flag)`, `RemoveSequence(sequence)`,
/// `Deactivate(sequence, easeOut)`, a "reset" (`00a2e9a0`), the target
/// getter (`0055b980`), and the element accessors of its sequence
/// array/active set.
const MANAGER_CONSTRUCT: u32 = 0x00a2_efa0;
const MANAGER_ADD_SEQUENCE: u32 = 0x00a2_f0c0;
const MANAGER_REMOVE_SEQUENCE: u32 = 0x00a2_ec50;
const MANAGER_DEACTIVATE: u32 = 0x0047_b220;
const MANAGER_RESET: u32 = 0x00a2_e9a0;
const MANAGER_TARGET: u32 = 0x0055_b980;
const ACTIVE_SET_ELEMENT: u32 = 0x0062_9ab0;
const SEQUENCE_ARRAY_COUNT: u32 = 0x0065_8930;
const SEQUENCE_ARRAY_ELEMENT: u32 = 0x0087_7a30;
/// Root-node helper of `GetAccumRoot` (`005f5f80`, on the first sequence's
/// object).
const ACCUM_ROOT_OF_SEQUENCE: u32 = 0x005f_5f80;
/// `NiObjectNET::GetController(rtti)`, `RemoveController(controller)`, and
/// the two controller `NiRTTI`s `~Animation` removes.
const GET_CONTROLLER: u32 = 0x00a5_c570;
const REMOVE_CONTROLLER: u32 = 0x00a5_c480;
const RTTI_CONTROLLER_FIRST: u32 = 0x011f_36bc;
const RTTI_CONTROLLER_SECOND: u32 = 0x011f_36e4;
/// `NiNode` helpers used by `AddAnimation` and the initializer:
/// `SetFlag(flag, 2)` (`0043b370`), `FindObject(root, name, 1)`
/// (`__cdecl`), `NiAVObject::SetTranslate(&point)` (`00a59c60`),
/// the `NiPoint3`-style constructor `(float, byte, byte)` (`0043d410`), and
/// `NiControllerSequence::Activate(priority, flag, weight, easeIn, ...)`.
const NODE_SET_FLAG: u32 = 0x0043_b370;
const NODE_FIND_OBJECT: u32 = 0x00c4_b310;
const NODE_SET_TRANSLATE: u32 = 0x00a5_9c60;
const TRANSLATION_CONSTRUCT: u32 = 0x0043_d410;
const SEQUENCE_ACTIVATE: u32 = 0x00a3_4f20;

/// Functions of this unit past the first 40, called by address:
/// `Animation::SpecialIdleFree(bool, bool)`, `AnimIdleFree(&idle)`,
/// `AddGroup(group)`.
const SPECIAL_IDLE_FREE: u32 = 0x0049_8910;
const ANIM_IDLE_FREE: u32 = 0x0049_8670;
const ADD_GROUP: u32 = 0x0049_46a0;

/// `Interface::IsMenuIDVisible(id, 0)` and `Interface::IsInMenuMode`.
const IS_MENU_ID_VISIBLE: u32 = 0x0070_2680;
const IS_IN_MENU_MODE: u32 = 0x0070_2360;
const MENU_ID_PIPBOY_WAIT: u32 = 0x40c;
/// `printf`-style logger (`__cdecl(format, ...)`).
const LOG: u32 = 0x005b_5e40;
/// `MessageHandler::IncDisableWarningCount(flag)`.
const DISABLE_WARNINGS: u32 = 0x0043_b2b0;
/// Random number source (`MersenneTwister` wrapper) used for random picks.
const RANDOM: u32 = 0x0048_7f50;

// ---- Data used by the functions ----------------------------------------------

/// `AnimSequenceBase`, `AnimSequenceSingle` and `AnimSequenceMultiple`
/// vtables (`.rdata`).
const VTABLE_ANIM_SEQUENCE_SINGLE: u32 = 0x0101_d8c8;
const VTABLE_ANIM_SEQUENCE_BASE: u32 = 0x0101_d8e8;
const VTABLE_ANIM_SEQUENCE_MULTIPLE: u32 = 0x0101_d908;
/// The base destructor body (`0048ef40`, sets the base vtable).
const ANIM_SEQUENCE_BASE_DESTRUCT: u32 = 0x0048_ef40;

/// `"%s\\%s\\%s"` and `"SpecialAnims"`.
const SPECIAL_ANIMS_FORMAT: u32 = 0x0101_7f74;
const SPECIAL_ANIMS_FOLDER: u32 = 0x0101_3448;
/// The two suffix strings `GetCorrespondingSequence` appends for kinds 5
/// and 6.
const SUFFIX_KIND_5: u32 = 0x0101_d92c;
const SUFFIX_KIND_6: u32 = 0x0101_d924;
/// Log formats of `AddAnimation`.
const LOG_SEQUENCE_TYPE_NOT_FOUND: u32 = 0x0101_da38;
const LOG_UNABLE_TO_ADD: u32 = 0x0101_d9a8;
const LOG_OBJECT_NOT_IN_SKELETON: u32 = 0x0101_d970;
/// The three floats a fresh `Animation` copies into `movementDelta` and
/// `AccumRootTranslate`.
const ZERO_VECTOR: u32 = 0x011f_426c;

layout! {
    /// `AnimSequenceSingle` (Xbox PDB): vtable at +0, then the one sequence.
    pub struct AnimSequenceSingle: 0x08 {
        /// `pSeq` (Xbox PDB): `BSAnimGroupSequence*`.
        0x04 pSeq: Ptr,
    }

    /// `AnimSequenceMultiple` (Xbox PDB): vtable at +0, then the list.
    pub struct AnimSequenceMultiple: 0x08 {
        /// `pSeqList` (Xbox PDB): `NiTList<BSAnimGroupSequence*>*`.
        0x04 pSeqList: Ptr,
    }

    /// `BSAnimGroupSequence` (Xbox PDB), 0x78 bytes: a
    /// `NiControllerSequence` (0x74 bytes) and the group.
    pub struct BSAnimGroupSequence: 0x78 {
        /// `spAnimGroup` (Xbox PDB): `NiPointer<TESAnimGroup>`.
        0x74 spAnimGroup: Ptr,
    }

    /// `NiControllerManager` (Xbox PDB), 0x7c bytes; only the fields this
    /// unit reads. `m_kSequenceArray` and `m_kActiveSequences` are embedded
    /// containers: only their addresses are used, so they are declared as
    /// their first word.
    pub struct NiControllerManager: 0x7c {
        /// `m_kSequenceArray` (Xbox PDB): `NiTObjectArray<NiPointer<..>>`.
        0x34 m_kSequenceArray: u32,
        /// `m_kActiveSequences` (Xbox PDB): `NiTPrimitiveSet<..>`.
        0x44 m_kActiveSequences: u32,
        /// `m_bCumulative` (Xbox PDB).
        0x68 m_bCumulative: bool,
    }

    /// `Animation` (Xbox PDB), 0x13c bytes on PC as on the Xbox. The
    /// `[[n]]` arrays of the PDB are `n` bytes: the arrays below are 8
    /// entries of 2 or 4 bytes, declared by their first entry.
    pub struct Animation: 0x13c {
        /// `m_uFlags` (Xbox PDB).
        0x00 m_uFlags: u8,
        /// `pActorRef` (Xbox PDB): `TESObjectREFR*`.
        0x04 pActorRef: Ptr,
        /// `pAnimRoot` (Xbox PDB): `NiPointer<NiNode>`.
        0x08 pAnimRoot: Ptr,
        /// `pAccumRoot` (Xbox PDB): `NiNode*`.
        0x0C pAccumRoot: Ptr,
        /// `movementDelta` (Xbox PDB).
        0x10 movementDelta: Inline<NiPoint3>,
        /// `AccumRootTranslate` (Xbox PDB).
        0x1C AccumRootTranslate: Inline<NiPoint3>,
        /// `pSoundPriorityBone` (Xbox PDB): `NiAVObject*[8]`, first entry.
        0x28 pSoundPriorityBone: Ptr,
        /// `m_fLooking` (Xbox PDB).
        0x48 m_fLooking: f32,
        /// `group` (Xbox PDB): `u16[8]`, first entry.
        0x4C group: u16,
        /// `action` (Xbox PDB): `ANIM_GROUP_ACTION[8]`, first entry.
        0x5C action: i32,
        /// `loopCount` (Xbox PDB): `i32[8]`, first entry.
        0x7C loopCount: i32,
        /// `nextGroup` (Xbox PDB): `u16[8]`, first entry.
        0x9C nextGroup: u16,
        /// `nextLoops` (Xbox PDB): `i32[8]`, first entry.
        0xAC nextLoops: i32,
        /// `cSkipUpdate` (Xbox PDB).
        0xCC cSkipUpdate: u8,
        /// `bShutDown` (Xbox PDB).
        0xCD bShutDown: bool,
        /// `time` (Xbox PDB).
        0xD0 time: f32,
        /// `fLipTime` (Xbox PDB).
        0xD4 fLipTime: f32,
        /// `spManager` (Xbox PDB): `NiPointer<NiControllerManager>`.
        0xD8 spManager: Ptr,
        /// `pAnimSequenceMap` (Xbox PDB):
        /// `NiTPointerMap<unsigned short, AnimSequenceBase*>*`.
        0xDC pAnimSequenceMap: Ptr,
        /// `pCurrentSequence` (Xbox PDB): `BSAnimGroupSequence*[8]`, first
        /// entry.
        0xE0 pCurrentSequence: Ptr,
        /// `pLastMovementSequence` (Xbox PDB).
        0x100 pLastMovementSequence: Ptr,
        /// `kfModelList` (Xbox PDB): `BSSimpleList<KFModel*>`.
        0x104 kfModelList: Inline<BSSimpleList>,
        /// `m_fMoveSpeed` (Xbox PDB).
        0x10C m_fMoveSpeed: f32,
        /// `m_fAttackSpeed` (Xbox PDB).
        0x110 m_fAttackSpeed: f32,
        /// `m_fGlobalTimeMultiplier` (Xbox PDB).
        0x114 m_fGlobalTimeMultiplier: f32,
        /// `m_fReloadModifier` (Xbox PDB).
        0x118 m_fReloadModifier: f32,
        /// `m_fEquipModifier` (Xbox PDB).
        0x11C m_fEquipModifier: f32,
        /// `cSkipNextBlend` (Xbox PDB).
        0x120 cSkipNextBlend: u8,
        /// `sQueuedReloadGroup` (Xbox PDB).
        0x122 sQueuedReloadGroup: u16,
        /// `spAnimIdle` (Xbox PDB): `NiPointer<AnimIdle>`.
        0x124 spAnimIdle: Ptr,
        /// `spAnimIdleQueued` (Xbox PDB): `NiPointer<AnimIdle>`.
        0x128 spAnimIdleQueued: Ptr,
        /// `spAnimIdleFreeWhenInactiveA` (Xbox PDB):
        /// `NiPointer<AnimIdle>[2]`, first entry.
        0x12C spAnimIdleFreeWhenInactiveA: Ptr,
        /// `replayDelayList` (Xbox PDB): `BSSimpleList<IDLE_REPLAY_DELAY*>`.
        0x134 replayDelayList: Inline<BSSimpleList>,
    }
}

// ---- Small helpers --------------------------------------------------------------

/// `*(NiPointer*)pointer`: the pointer held by the `NiPointer` at `pointer`.
fn ni_pointer_get(e: &mut Engine, pointer: u32) -> u32 {
    e.call(READ_WORD, &args![pointer]).u32()
}

/// The `ModelLoader` singleton.
fn model_loader(e: &Engine) -> u32 {
    e.global(MODEL_LOADER)
}

/// The file name of an `NiObjectNET`: `GetName()` then its `const char*`.
fn object_name(e: &mut Engine, object: u32) -> u32 {
    let name = e.call(OBJECT_NAME, &args![object]).u32();
    e.call(FIXED_STRING_CSTR, &args![name]).u32()
}

/// Runs `body` between the scope guard's constructor (tag `0x33`, source
/// line `line` of `Animation.cpp`) and its destructor.
fn with_scope_guard<R>(e: &mut Engine, line: u32, body: impl FnOnce(&mut Engine) -> R) -> R {
    e.with_stack(SCOPE_GUARD_SIZE, |e, guard| {
        e.call(
            SCOPE_GUARD_CTOR,
            &args![guard, SCOPE_GUARD_TAG, 1u32, SOURCE_FILE, line],
        );
        let result = body(e);
        e.call(SCOPE_GUARD_DTOR, &args![guard]);
        result
    })
}

/// Address of entry `i` of an array field of `Animation`.
fn animation_entry<T>(this: Ptr<Animation>, field: Field<Animation, T>, i: u32, size: u32) -> u32 {
    this.addr().wrapping_add(field.off).wrapping_add(i * size)
}

// ---- AnimSequenceBase, AnimSequenceSingle, AnimSequenceMultiple ------------------------------

// Translated from 0048ee70 (decompiled, FalloutNV.exe 1.4.0.525)
/// `AnimSequenceSingle` destructor body (the map has no name for it): sets
/// the vtable, and when a sequence is held, unloads its model by file name
/// (`ModelLoader` `00445370`) and releases it; then the base destructor.
/// The compiler's exception-unwinding frame is not translated.
pub fn fn_0048ee70(e: &mut Engine, this: Ptr<AnimSequenceSingle>) {
    e.mem.set_u32(this.addr(), VTABLE_ANIM_SEQUENCE_SINGLE);
    let sequence = e.get(this, AnimSequenceSingle::pSeq);
    if !sequence.is_null() {
        e.with_stack(0x104, |e, buffer| {
            let name = object_name(e, sequence.addr());
            e.call(STRCPY_S, &args![buffer, 0x104u32, name]);
            let sequence = e.get(this, AnimSequenceSingle::pSeq);
            e.call(REF_RELEASE, &args![sequence]);
            let loader = model_loader(e);
            fn_0048eff0(e, Ptr::new(loader), buffer.addr());
        });
    }
    e.call(ANIM_SEQUENCE_BASE_DESTRUCT, &args![this]);
}

// Translated from 0048ef60 (decompiled, FalloutNV.exe 1.4.0.525)
/// `AnimSequenceBase::_scalar_deleting_destructor_` (Xbox PDB): the base
/// destructor body, then `operator delete` when bit 0 of `flags` is set.
/// Returns `this`.
pub fn anim_sequence_base_scalar_deleting_destructor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(ANIM_SEQUENCE_BASE_DESTRUCT, &args![this]);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Translated from 0048ef90 (decompiled, FalloutNV.exe 1.4.0.525)
/// `AnimSequenceSingle` implementation of two virtual functions the linker
/// folded (`GetSequenceToPlay` at +0x10 and `GetSequence` at +0x14): the
/// held sequence, whatever the argument.
pub fn fn_0048ef90(e: &mut Engine, this: Ptr<AnimSequenceSingle>, _unused_argument: u32) -> Ptr {
    e.get(this, AnimSequenceSingle::pSeq)
}

// Translated from 0048efb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `AnimSequenceSingle::GetSequenceIndex` (slot +0x18): always `0xff`
/// ("no index"), whatever the argument.
pub fn fn_0048efb0(_e: &mut Engine, _this: Ptr<AnimSequenceSingle>, _unused_argument: u32) -> u8 {
    0xff
}

// Translated from 0048efc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `AnimSequenceSingle::_scalar_deleting_destructor_` (Xbox PDB):
/// destroys, and frees when bit 0 of `flags` is set. Returns `this`.
pub fn anim_sequence_single_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr<AnimSequenceSingle>,
    flags: u32,
) -> Ptr<AnimSequenceSingle> {
    fn_0048ee70(e, this);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Translated from 0048eff0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unloads the model named `name` from the `ModelLoader` `loader`
/// (`00445370(name, 1)`); the map has no name for it.
pub fn fn_0048eff0(e: &mut Engine, loader: Ptr, name: u32) {
    e.call(MODEL_LOADER_REMOVE, &args![loader, name, 1u32]);
}

// Translated from 0048f010 (decompiled, FalloutNV.exe 1.4.0.525)
/// `AnimSequenceSingle::SetSequence` (Xbox PDB): replaces the held
/// sequence with `sequence`. The old one is released, and when
/// `free_model` is set its model is first unloaded from the `ModelLoader`
/// by file name; the new one gets a reference.
pub fn anim_sequence_single_set_sequence(
    e: &mut Engine,
    this: Ptr<AnimSequenceSingle>,
    sequence: Ptr,
    free_model: u8,
) {
    let old = e.get(this, AnimSequenceSingle::pSeq);
    if !old.is_null() {
        if free_model != 0 {
            let name = object_name(e, old.addr());
            let loader = model_loader(e);
            fn_0048eff0(e, Ptr::new(loader), name);
        }
        let old = e.get(this, AnimSequenceSingle::pSeq);
        e.call(REF_RELEASE, &args![old]);
    }
    e.set(this, AnimSequenceSingle::pSeq, sequence);
    let held = e.get(this, AnimSequenceSingle::pSeq);
    if !held.is_null() {
        e.call(REF_ADD, &args![held]);
    }
}

// Translated from 0048f080 (decompiled, FalloutNV.exe 1.4.0.525)
/// `AnimSequenceSingle::RemoveSequence` (Xbox PDB): when `sequence` is the
/// held one, clears it through `SetSequence(0, free_model)` (virtual call,
/// slot +4) and returns true.
pub fn anim_sequence_single_remove_sequence(
    e: &mut Engine,
    this: Ptr<AnimSequenceSingle>,
    sequence: Ptr,
    free_model: u8,
) -> bool {
    if e.get(this, AnimSequenceSingle::pSeq) == sequence {
        e.vcall(this.addr(), 4, &args![0u32, free_model]);
        true
    } else {
        false
    }
}

// Translated from 0048f0c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `AnimSequenceMultiple` constructor from another sequence container
/// (the map has no name for it): builds an empty list, moves `other`'s
/// current sequence (`GetSequenceToPlay(-1)`) into it, clears `other`
/// (`SetSequence(0, 0)`) and deletes it. Returns `this`.
///
/// `SetSequence` of the multiple declares two stack arguments and reads
/// only the first; the game pushes one, so the second word passed here is
/// not meaningful.
pub fn fn_0048f0c0(
    e: &mut Engine,
    this: Ptr<AnimSequenceMultiple>,
    other: Ptr,
) -> Ptr<AnimSequenceMultiple> {
    fn_0048f1b0(e, this.cast());
    e.mem.set_u32(this.addr(), VTABLE_ANIM_SEQUENCE_MULTIPLE);
    let block = e.call(OPERATOR_NEW, &args![0xcu32]).u32();
    let list = if block == 0 {
        0
    } else {
        fn_0048f200(e, Ptr::new(block)).addr()
    };
    e.set(this, AnimSequenceMultiple::pSeqList, Ptr::new(list));
    let sequence = e
        .vcall(other.addr(), 0x10, &args![0xffff_ffffu32, 1u32])
        .ptr::<()>();
    anim_sequence_multiple_set_sequence(e, this, sequence, 0);
    e.vcall(other.addr(), 4, &args![0u32, 0u32]);
    if !other.is_null() {
        e.vcall(other.addr(), 0, &args![1u32]);
    }
    this
}

// Translated from 0048f1b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `AnimSequenceBase` constructor (the map has no name for it): sets the
/// base vtable. Returns `this`.
pub fn fn_0048f1b0(e: &mut Engine, this: Ptr) -> Ptr {
    e.mem.set_u32(this.addr(), VTABLE_ANIM_SEQUENCE_BASE);
    this
}

// Translated from 0048f1d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `AnimSequenceMultiple::_scalar_deleting_destructor_` (Xbox PDB):
/// destroys, and frees when bit 0 of `flags` is set. Returns `this`.
pub fn anim_sequence_multiple_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr<AnimSequenceMultiple>,
    flags: u32,
) -> Ptr<AnimSequenceMultiple> {
    fn_0048f220(e, this);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Translated from 0048f200 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructor of the empty `NiTList` an `AnimSequenceMultiple` holds
/// (`00718d90`, shared with other lists); returns `this`.
pub fn fn_0048f200(e: &mut Engine, this: Ptr) -> Ptr {
    e.call(LIST_CONSTRUCT, &args![this]);
    this
}

// Translated from 0048f220 (decompiled, FalloutNV.exe 1.4.0.525)
/// `AnimSequenceMultiple` destructor body (the map has no name for it):
/// for every sequence of the list, unloads its model by file name and
/// releases it; then deletes the list and runs the base destructor.
pub fn fn_0048f220(e: &mut Engine, this: Ptr<AnimSequenceMultiple>) {
    e.mem.set_u32(this.addr(), VTABLE_ANIM_SEQUENCE_MULTIPLE);
    let list = e.get(this, AnimSequenceMultiple::pSeqList);
    if !list.is_null() {
        e.with_stack(0x104, |e, buffer| {
            let mut position = e.call(LIST_HEAD, &args![list]).u32();
            while position != 0 {
                let item = e.call(LIST_ITEM_ADDRESS, &args![list, position]).u32();
                let sequence = e.mem.u32(item);
                let name = object_name(e, sequence);
                e.call(STRCPY_S, &args![buffer, 0x104u32, name]);
                e.call(REF_RELEASE, &args![sequence]);
                let loader = model_loader(e);
                fn_0048eff0(e, Ptr::new(loader), buffer.addr());
                position = e.call(LIST_NEXT, &args![list, position]).u32();
            }
        });
        let list = e.get(this, AnimSequenceMultiple::pSeqList);
        if !list.is_null() {
            fn_0048f370(e, list, 1);
        }
    }
    e.call(ANIM_SEQUENCE_BASE_DESTRUCT, &args![this]);
}

// Translated from 0048f370 (decompiled, FalloutNV.exe 1.4.0.525)
/// Scalar deleting destructor of the `NiTList` (the map has no name for
/// it): destroys the list, frees it when bit 0 of `flags` is set. Returns
/// `this`.
pub fn fn_0048f370(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(LIST_DESTRUCT, &args![this]);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Translated from 0048f3a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `AnimSequenceMultiple::SetSequence` (Xbox PDB): adds a reference to
/// `sequence` and adds it at the head of the list. The second argument the
/// virtual interface passes is not read.
pub fn anim_sequence_multiple_set_sequence(
    e: &mut Engine,
    this: Ptr<AnimSequenceMultiple>,
    sequence: Ptr,
    _unused_free_model: u32,
) {
    e.call(REF_ADD, &args![sequence]);
    let list = e.get(this, AnimSequenceMultiple::pSeqList);
    e.with_stack(4, |e, cell| {
        e.mem.set_u32(cell.addr(), sequence.addr());
        e.call(LIST_ADD_HEAD, &args![list, cell]);
    });
}

// Translated from 0048f3d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `AnimSequenceMultiple::RemoveSequence` (Xbox PDB): releases `sequence`
/// and removes it from the list; when that leaves the list empty, deletes
/// the list and returns true, otherwise false. The second argument the
/// virtual interface passes is not read.
pub fn anim_sequence_multiple_remove_sequence(
    e: &mut Engine,
    this: Ptr<AnimSequenceMultiple>,
    sequence: Ptr,
    _unused_free_model: u32,
) -> bool {
    e.call(REF_RELEASE, &args![sequence]);
    let list = e.get(this, AnimSequenceMultiple::pSeqList);
    e.with_stack(4, |e, cell| {
        e.mem.set_u32(cell.addr(), sequence.addr());
        e.call(LIST_REMOVE, &args![list, cell]);
    });
    let list = e.get(this, AnimSequenceMultiple::pSeqList);
    if e.call(LIST_IS_EMPTY, &args![list]).bool() {
        let list = e.get(this, AnimSequenceMultiple::pSeqList);
        if !list.is_null() {
            fn_0048f370(e, list, 1);
        }
        e.set(this, AnimSequenceMultiple::pSeqList, Ptr::NULL);
        true
    } else {
        false
    }
}

// Translated from 0048f450 (decompiled, FalloutNV.exe 1.4.0.525)
/// `AnimSequenceMultiple::GetSequenceToPlay` (Xbox PDB): the sequence at
/// `index` (a signed byte), or a random one (`random % count`) when
/// `index` is -1 or past the end; 0 when the list runs out first. A random
/// pick from an empty list divides by zero, as in the game.
pub fn anim_sequence_multiple_get_sequence_to_play(
    e: &mut Engine,
    this: Ptr<AnimSequenceMultiple>,
    index: i8,
) -> Ptr {
    let list = e.get(this, AnimSequenceMultiple::pSeqList);
    let use_random = if index == -1 {
        true
    } else {
        let count = e.call(LIST_COUNT, &args![list]).u32();
        (index as i32 as u32) >= count
    };
    let mut remaining = if use_random {
        let random = e.call(RANDOM, &args![]).u32();
        let list = e.get(this, AnimSequenceMultiple::pSeqList);
        let count = e.call(LIST_COUNT, &args![list]).u32();
        random % count
    } else {
        index as i32 as u32
    };
    let list = e.get(this, AnimSequenceMultiple::pSeqList);
    let mut position = e.call(LIST_HEAD, &args![list]).u32();
    while position != 0 {
        let item = e.call(LIST_ITEM_ADDRESS, &args![list, position]).u32();
        let sequence = e.mem.u32(item);
        let before = remaining;
        remaining = remaining.wrapping_sub(1);
        if before == 0 {
            return Ptr::new(sequence);
        }
        position = e.call(LIST_NEXT, &args![list, position]).u32();
    }
    Ptr::NULL
}

// Translated from 0048f500 (decompiled, FalloutNV.exe 1.4.0.525)
/// `AnimSequenceMultiple::GetCorrespondingSequence` (Xbox PDB): the
/// sequence in the list whose file name (from the last `\`) equals that of
/// `sequence`, case-insensitively. For `kind` 5 and 6 the name is first
/// rewritten: the text from the first `_` of the file name is replaced by
/// a fixed suffix and the rest of the original name after its first `_`
/// is appended. 0 when there is no match (or no `\` in the name).
pub fn anim_sequence_multiple_get_corresponding_sequence(
    e: &mut Engine,
    this: Ptr<AnimSequenceMultiple>,
    sequence: Ptr,
    kind: i32,
) -> Ptr {
    let list = e.get(this, AnimSequenceMultiple::pSeqList);
    let mut position = e.call(LIST_HEAD, &args![list]).u32();
    let full_name = object_name(e, sequence.addr());
    let file_name = e.call(STRRCHR, &args![full_name, 0x5cu32]).u32();
    if file_name == 0 {
        return Ptr::NULL;
    }
    e.with_stack(0x104, |e, buffer| {
        let mut wanted = file_name;
        let suffix = match kind {
            5 => Some(SUFFIX_KIND_5),
            6 => Some(SUFFIX_KIND_6),
            _ => None,
        };
        if let Some(suffix) = suffix {
            e.call(STRCPY, &args![buffer, file_name]);
            let underscore = e.call(STRCHR, &args![buffer, 0x5fu32]).u32();
            if underscore != 0 {
                e.call(STRCPY, &args![underscore, suffix]);
                let tail = e.call(STRCHR, &args![file_name, 0x5fu32]).u32();
                e.call(STRCAT, &args![buffer, tail]);
            } else {
                e.call(STRCAT, &args![buffer, suffix]);
            }
            wanted = buffer.addr();
        }
        while position != 0 {
            let item = e.call(LIST_ITEM_ADDRESS, &args![list, position]).u32();
            let candidate = e.mem.u32(item);
            let candidate_path = object_name(e, candidate);
            let candidate_name = e.call(STRRCHR, &args![candidate_path, 0x5cu32]).u32();
            if candidate_name != 0 && e.call(STRICMP, &args![candidate_name, wanted]).i32() == 0 {
                return Ptr::new(candidate);
            }
            position = e.call(LIST_NEXT, &args![list, position]).u32();
        }
        Ptr::NULL
    })
}

// Translated from 0048f720 (decompiled, FalloutNV.exe 1.4.0.525)
/// `AnimSequenceMultiple::GetSequenceIndex` (Xbox PDB): the position of
/// `sequence` in the list as a byte, `0xff` when it is not there.
pub fn anim_sequence_multiple_get_sequence_index(
    e: &mut Engine,
    this: Ptr<AnimSequenceMultiple>,
    sequence: Ptr,
) -> u8 {
    let list = e.get(this, AnimSequenceMultiple::pSeqList);
    let mut position = e.call(LIST_HEAD, &args![list]).u32();
    let mut index = 0u8;
    while position != 0 {
        let item = e.call(LIST_ITEM_ADDRESS, &args![list, position]).u32();
        if e.mem.u32(item) == sequence.addr() {
            return index;
        }
        index = index.wrapping_add(1);
        position = e.call(LIST_NEXT, &args![list, position]).u32();
    }
    0xff
}

// Translated from 0048f790 (decompiled, FalloutNV.exe 1.4.0.525)
/// `AnimSequenceMultiple::GetSequence` (Xbox PDB): the sequence of the
/// list whose animation group is `group`, or 0.
pub fn anim_sequence_multiple_get_sequence(
    e: &mut Engine,
    this: Ptr<AnimSequenceMultiple>,
    group: Ptr,
) -> Ptr {
    let list = e.get(this, AnimSequenceMultiple::pSeqList);
    let mut position = e.call(LIST_HEAD, &args![list]).u32();
    while position != 0 {
        let item = e.call(LIST_ITEM_ADDRESS, &args![list, position]).u32();
        let sequence = e.mem.u32(item);
        if fn_0048f7f0(e, Ptr::new(sequence)) == group {
            return Ptr::new(sequence);
        }
        position = e.call(LIST_NEXT, &args![list, position]).u32();
    }
    Ptr::NULL
}

// Translated from 0048f7f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Reads `BSAnimGroupSequence::spAnimGroup` (+0x74). The map names this
/// body `Animation::ZeroGlobalTransform`, a name the linker's folding put
/// on it; the body is the group getter.
pub fn fn_0048f7f0(e: &mut Engine, this: Ptr<BSAnimGroupSequence>) -> Ptr {
    let group = this.byte_add(BSAnimGroupSequence::spAnimGroup.off);
    e.call(READ_WORD, &args![group]).ptr()
}

// ---- Animation ---------------------------------------------------------------------

// Translated from 0048f810 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Animation::Animation` (Xbox PDB): constructs the members (the
/// `NiPointer`s, the lists, the `NiTMap` with 0x65 buckets), copies the
/// zero vector into `movementDelta` and `AccumRootTranslate`, fills the
/// group/action arrays with their "none" values (0xff / -1), sets the
/// speed multipliers to 1.0 and `sQueuedReloadGroup` to 0xff. Returns
/// `this`.
pub fn animation_animation(e: &mut Engine, this: Ptr<Animation>) -> Ptr<Animation> {
    let a = this.addr();
    e.call(NI_POINTER_INIT, &args![a + 0x8, 0u32]);
    e.call(NI_POINT3_CONSTRUCT, &args![a + 0x10]);
    e.call(NI_POINT3_CONSTRUCT, &args![a + 0x1c]);
    e.call(NI_POINTER_INIT, &args![a + 0xd8, 0u32]);
    e.call(SIMPLE_LIST_CONSTRUCT, &args![a + 0x104]);
    e.call(NI_POINTER_INIT, &args![a + 0x124, 0u32]);
    e.call(NI_POINTER_INIT, &args![a + 0x128, 0u32]);
    e.call(
        VECTOR_CONSTRUCT,
        &args![
            a + 0x12c,
            4u32,
            2u32,
            NI_POINTER_DEFAULT_CONSTRUCT,
            NI_POINTER_RELEASE
        ],
    );
    e.call(SIMPLE_LIST_CONSTRUCT, &args![a + 0x134]);
    e.call(NI_POINTER_SET, &args![a + 0x8, 0u32]);
    e.set(this, Animation::pAccumRoot, Ptr::NULL);
    for field in [
        Animation::movementDelta.off,
        Animation::AccumRootTranslate.off,
    ] {
        for word in 0..3 {
            let value = e.mem.u32(ZERO_VECTOR + 4 * word);
            e.mem.set_u32(a + field + 4 * word, value);
        }
    }
    e.set(this, Animation::m_fLooking, 0.0);
    e.set(this, Animation::cSkipUpdate, 0xff);
    e.set(this, Animation::cSkipNextBlend, 0);
    fn_0048fb20(e, this, 0, 0xff, 0);
    e.set(this, Animation::pActorRef, Ptr::NULL);
    e.set(this, Animation::pLastMovementSequence, Ptr::NULL);
    e.set(this, Animation::time, 0.0);
    e.call(NI_POINTER_SET, &args![a + 0xd8, 0u32]);
    e.call(NI_POINTER_SET, &args![a + 0x124, 0u32]);
    e.call(NI_POINTER_SET, &args![a + 0x128, 0u32]);
    e.call(NI_POINTER_SET, &args![a + 0x12c, 0u32]);
    e.call(NI_POINTER_SET, &args![a + 0x130, 0u32]);
    let block = e.call(OPERATOR_NEW, &args![0x10u32]).u32();
    let map = if block == 0 {
        0
    } else {
        e.call(MAP_CONSTRUCT, &args![block, 0x65u32]).u32()
    };
    e.set(this, Animation::pAnimSequenceMap, Ptr::new(map));
    e.call(MEMSET, &args![a + 0x28, 0u32, 0x20u32]);
    e.call(MEMSET, &args![a + 0xe0, 0u32, 0x20u32]);
    e.call(MEMSET, &args![a + 0x4c, 0xffu32, 0x10u32]);
    e.call(MEMSET, &args![a + 0x9c, 0xffu32, 0x10u32]);
    e.call(MEMSET, &args![a + 0x5c, 0xffff_ffffu32, 0x20u32]);
    e.call(MEMSET, &args![a + 0x7c, 0xffff_ffffu32, 0x20u32]);
    e.call(MEMSET, &args![a + 0xac, 0xffff_ffffu32, 0x20u32]);
    e.set(this, Animation::m_fMoveSpeed, 1.0);
    e.set(this, Animation::m_fAttackSpeed, 1.0);
    e.set(this, Animation::m_fGlobalTimeMultiplier, 1.0);
    e.set(this, Animation::m_fReloadModifier, 1.0);
    e.set(this, Animation::m_fEquipModifier, 1.0);
    e.set(this, Animation::sQueuedReloadGroup, 0xff);
    e.set(this, Animation::bShutDown, false);
    this
}

// Translated from 0048fb20 (decompiled, FalloutNV.exe 1.4.0.525)
/// Replaces the bits of `m_uFlags` selected by `mask` with `value` shifted
/// left by `shift` (the shift count is taken modulo 32, as `SHL` does):
/// `flags = (flags & ~mask) | (value << shift)`, kept to a byte.
pub fn fn_0048fb20(e: &mut Engine, this: Ptr<Animation>, value: u8, mask: u8, shift: u8) {
    let flags = e.get(this, Animation::m_uFlags) as u32;
    let flags = (flags & !(mask as u32)) | ((value as u32) << (shift & 0x1f));
    e.set(this, Animation::m_uFlags, flags as u8);
}

// Translated from 0048fb50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Animation::~Animation` body (the map has no name for it): shuts down
/// the idles, cancels the replacement KF list, and when there is a
/// controller manager removes the two controllers it put on the animation
/// root, deactivates all its sequences and detaches it from its target.
/// Then deletes every `AnimSequenceBase` of the map, releases the KF
/// models of the list, frees the map and the replay delay entries, and
/// destroys the members in reverse order of construction. The compiler's
/// exception-unwinding frame is not translated.
pub fn fn_0048fb50(e: &mut Engine, this: Ptr<Animation>) {
    let a = this.addr();
    animation_shutdown_all_anim_idles(e, this);
    let loader = model_loader(e);
    e.call(
        MODEL_LOADER_CANCEL_REPLACEMENT_KF_LIST,
        &args![loader, this],
    );
    if ni_pointer_get(e, a + 0xd8) != 0 {
        let first = if ni_pointer_get(e, a + 8) != 0 {
            let root = ni_pointer_get(e, a + 8);
            e.call(GET_CONTROLLER, &args![root, RTTI_CONTROLLER_FIRST])
                .u32()
        } else {
            0
        };
        let second = if ni_pointer_get(e, a + 8) != 0 {
            let root = ni_pointer_get(e, a + 8);
            e.call(GET_CONTROLLER, &args![root, RTTI_CONTROLLER_SECOND])
                .u32()
        } else {
            0
        };
        if first != 0 {
            let root = ni_pointer_get(e, a + 8);
            e.call(REMOVE_CONTROLLER, &args![root, first]);
        }
        if second != 0 {
            let root = ni_pointer_get(e, a + 8);
            e.call(REMOVE_CONTROLLER, &args![root, second]);
        }
        let manager = ni_pointer_get(e, a + 0xd8);
        ni_controller_manager_deactivate_all(e, Ptr::new(manager), 0.0);
        let manager = ni_pointer_get(e, a + 0xd8);
        e.call(MANAGER_RESET, &args![manager]);
        let manager = ni_pointer_get(e, a + 0xd8);
        let target = e.call(MANAGER_TARGET, &args![manager]).u32();
        if target != 0 && e.vcall(target, 0xc, &args![]).u32() != 0 {
            let manager = ni_pointer_get(e, a + 0xd8);
            e.call(REMOVE_CONTROLLER, &args![target, manager]);
        }
        e.call(NI_POINTER_SET, &args![a + 0xd8, 0u32]);
    }

    // Delete the sequence containers of the map.
    let map = e.get(this, Animation::pAnimSequenceMap).addr();
    let mut position = e.call(MAP_FIRST_POSITION, &args![map]).u32();
    while position != 0 {
        e.with_stack(12, |e, cells| {
            let (position_cell, key_cell, value_cell) =
                (cells.addr(), cells.addr() + 4, cells.addr() + 8);
            e.mem.set_u32(position_cell, position);
            e.mem.set_u32(value_cell, 0);
            e.call(
                MAP_GET_NEXT,
                &args![map, position_cell, key_cell, value_cell],
            );
            position = e.mem.u32(position_cell);
            let value = e.mem.u32(value_cell);
            if value != 0 {
                e.vcall(value, 0, &args![1u32]);
            }
        });
    }

    // Release the KF models.
    let mut node = a + Animation::kfModelList.off;
    while node != 0 && !e.call(SIMPLE_LIST_IS_EMPTY, &args![node]).bool() {
        let item = e.call(SIMPLE_LIST_ITEM, &args![node]).u32();
        let model = e.mem.u32(item);
        e.call(KF_MODEL_RELEASE, &args![model]);
        node = e.call(SIMPLE_LIST_NEXT, &args![node]).u32();
    }
    e.call(SIMPLE_LIST_CLEAR, &args![a + 0x104]);
    let map = e.get(this, Animation::pAnimSequenceMap).addr();
    e.call(MAP_REMOVE_ALL, &args![map]);
    let map = e.get(this, Animation::pAnimSequenceMap).addr();
    if map != 0 {
        e.vcall(map, 0, &args![1u32]);
    }
    e.set(this, Animation::pAnimSequenceMap, Ptr::NULL);
    e.set(this, Animation::pAccumRoot, Ptr::NULL);

    // Free the replay delay entries.
    if !e.call(SIMPLE_LIST_IS_EMPTY, &args![a + 0x134]).bool() {
        let mut node = a + Animation::replayDelayList.off;
        while node != 0 {
            let item = e.call(SIMPLE_LIST_ITEM, &args![node]).u32();
            let entry = e.mem.u32(item);
            e.call(OPERATOR_DELETE, &args![entry]);
            node = e.call(SIMPLE_LIST_NEXT, &args![node]).u32();
        }
        e.call(SIMPLE_LIST_CLEAR, &args![a + 0x134]);
    }

    // Destroy the members in reverse order.
    e.call(SIMPLE_LIST_DESTRUCT, &args![a + 0x134]);
    e.call(
        VECTOR_DESTRUCT,
        &args![a + 0x12c, 4u32, 2u32, NI_POINTER_RELEASE],
    );
    e.call(NI_POINTER_RELEASE, &args![a + 0x128]);
    e.call(NI_POINTER_RELEASE, &args![a + 0x124]);
    e.call(SIMPLE_LIST_DESTRUCT, &args![a + 0x104]);
    e.call(NI_POINTER_RELEASE, &args![a + 0xd8]);
    e.call(NI_POINTER_RELEASE, &args![a + 0x8]);
}

// Translated from 0048fef0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiControllerManager::DeactivateAll` (Xbox PDB): deactivates every
/// active sequence with the ease-out time `ease_out`.
pub fn ni_controller_manager_deactivate_all(
    e: &mut Engine,
    this: Ptr<NiControllerManager>,
    ease_out: f32,
) {
    let active = this.addr() + NiControllerManager::m_kActiveSequences.off;
    let mut i = 0u32;
    while i < e.call(LIST_COUNT, &args![active]).u32() {
        let slot = e.call(ACTIVE_SET_ELEMENT, &args![active, i]).u32();
        let sequence = e.mem.u32(slot);
        e.call(MANAGER_DEACTIVATE, &args![this, sequence, ease_out]);
        i += 1;
    }
}

// Translated from 0048ff50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Animation::ShutdownAllAnimIdles` (Xbox PDB): flags the animation as
/// shut down, frees the special idles (`SpecialIdleFree(1, 1)`), and frees
/// and clears each of the two "free when inactive" idles that are set.
pub fn animation_shutdown_all_anim_idles(e: &mut Engine, this: Ptr<Animation>) {
    e.set(this, Animation::bShutDown, true);
    e.call(SPECIAL_IDLE_FREE, &args![this, 1u32, 1u32]);
    for i in 0..2 {
        let slot = animation_entry(this, Animation::spAnimIdleFreeWhenInactiveA, i, 4);
        if ni_pointer_get(e, slot) != 0 {
            e.call(ANIM_IDLE_FREE, &args![this, slot]);
            e.call(NI_POINTER_SET, &args![slot, 0u32]);
        }
    }
}

// Translated from 0048ffd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets up an animation from its keyframe list (the map has no name for
/// it; the arguments are the KF file `list`, the animation root node
/// `root`, the owning reference `actor` and `play_idle`). Returns false
/// without doing anything when an argument is null or the sequence map
/// already has the entry for group 0; otherwise stores the reference and
/// root, finds the sound priority bones, creates the `NiControllerManager`
/// for the root, loads and adds every KF file (deleting the list),
/// and when the map then has an entry for group 0 and `play_idle` is set,
/// activates its sequence and applies a zero translation to the root.
/// Returns whether the map has that entry. The scope guard of source line
/// 568 is kept; the compiler's exception frame is not translated.
pub fn fn_0048ffd0(
    e: &mut Engine,
    this: Ptr<Animation>,
    list: Ptr,
    root: Ptr,
    actor: Ptr,
    play_idle: bool,
) -> bool {
    with_scope_guard(e, 0x238, |e| {
        if list.is_null() || root.is_null() || actor.is_null() {
            return false;
        }
        let a = this.addr();
        e.set(this, Animation::pActorRef, actor);
        e.with_stack(4, |e, out| {
            if e.get(this, Animation::pAnimSequenceMap).addr() != 0 {
                let map = e.get(this, Animation::pAnimSequenceMap);
                if e.call(MAP_GET_AT, &args![map, 0u32, out]).bool() {
                    return false;
                }
            }
            // The actor's base form type, compared with 0x2a (the value is
            // not used afterwards).
            let form = e.call(REFERENCE_FORM, &args![actor]).u32();
            let _is_form_type_2a = e.call(FORM_TYPE, &args![form]).u32() == FORM_TYPE_COMPARED;
            e.call(NI_POINTER_SET, &args![a + 8, root]);
            for i in 0..8u32 {
                let bone_name = e.mem.u32(SOUND_PRIORITY_BONE_NAMES + 4 * i);
                if bone_name != 0 {
                    let bone = fn_00490310(e, actor, root, bone_name);
                    e.mem
                        .set_u32(a + Animation::pSoundPriorityBone.off + 4 * i, bone);
                }
            }
            let anim_root = ni_pointer_get(e, a + 8);
            fn_004902f0(e, Ptr::new(anim_root), 0);
            let block = e.call(NI_OPERATOR_NEW, &args![0x7cu32]).u32();
            let manager = if block == 0 {
                0
            } else {
                let anim_root = ni_pointer_get(e, a + 8);
                e.call(MANAGER_CONSTRUCT, &args![block, anim_root, 1u32])
                    .u32()
            };
            e.call(NI_POINTER_SET, &args![a + 0xd8, manager]);
            let queue_cloning = animation_should_queue_sequence_cloning(e, this);
            while !e.call(SIMPLE_LIST_IS_EMPTY, &args![list]).bool() {
                let item = e.call(SIMPLE_LIST_ITEM, &args![list]).u32();
                let name = e.mem.u32(item);
                let loader = model_loader(e);
                let kf = e.call(MODEL_LOADER_LOAD_KF, &args![loader, name]).u32();
                if kf != 0 && e.call(KF_MODEL_ANIM_GROUP, &args![kf]).u32() != 0 {
                    animation_add_animation(e, this, kf, queue_cloning);
                }
                e.call(OPERATOR_DELETE, &args![name]);
                e.call(SIMPLE_LIST_POP_FRONT, &args![list]);
            }
            if e.get(this, Animation::pAccumRoot).addr() != 0 {
                let accum_root = e.get(this, Animation::pAccumRoot);
                e.call(
                    ACCUM_ROOT_SETUP,
                    &args![accum_root, ACCUM_ROOT_SETUP_ARGUMENT],
                );
            }
            if !list.is_null() {
                e.call(SIMPLE_LIST_DELETE, &args![list, 1u32]);
            }
            let map = e.get(this, Animation::pAnimSequenceMap);
            let found = e.call(MAP_GET_AT, &args![map, 0u32, out]).bool();
            if found && play_idle {
                let entry = e.mem.u32(out.addr());
                let sequence = e.vcall(entry, 0x10, &args![0xffff_ffffu32]).u32();
                if sequence != 0 {
                    e.call(
                        SEQUENCE_ACTIVATE,
                        &args![sequence, 0x64u32, 1u32, 1.0f32, 0.0f32, 0u32, 0u32],
                    );
                    e.with_stack(12, |e, translation| {
                        e.call(
                            TRANSLATION_CONSTRUCT,
                            &args![translation, 0.0f32, 1u32, 0u32],
                        );
                        let anim_root = ni_pointer_get(e, a + 8);
                        e.call(NODE_SET_TRANSLATE, &args![anim_root, translation]);
                    });
                    e.vcall(sequence, 0x8c, &args![0.0f32, 0u32]);
                }
            }
            found
        })
    })
}

// Translated from 004902f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls `0043b370(flag, 2)` on `this` (the animation root node); the map
/// has no name for it.
pub fn fn_004902f0(e: &mut Engine, this: Ptr, flag: u8) {
    e.call(NODE_SET_FLAG, &args![this, flag, 2u32]);
}

// Translated from 00490310 (decompiled, FalloutNV.exe 1.4.0.525)
/// Finds the object called `name` under `root` (`__cdecl(root, name, 1)`
/// at `00c4b310`). `this` (the reference) is not used.
pub fn fn_00490310(e: &mut Engine, _this: Ptr, root: Ptr, name: u32) -> u32 {
    e.call(NODE_FIND_OBJECT, &args![root, name, 1u32]).u32()
}

// Translated from 00490330 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Animation::AddSpecialAnimations` (Xbox PDB): for every file name in
/// the `BSSimpleList` `list`, loads `"<directory>\\SpecialAnims\\<name>"`
/// as a KF and adds it. Does nothing for a null list. The list and its
/// strings are not freed.
pub fn animation_add_special_animations(
    e: &mut Engine,
    this: Ptr<Animation>,
    list: Ptr,
    directory: u32,
) {
    if list.is_null() {
        return;
    }
    let mut node = list.addr();
    while node != 0 {
        let item = e.call(SIMPLE_LIST_ITEM, &args![node]).u32();
        let name = e.mem.u32(item);
        e.with_stack(0x104, |e, path| {
            e.call(
                SPRINTF_S,
                &args![
                    path,
                    0x104u32,
                    SPECIAL_ANIMS_FORMAT,
                    directory,
                    SPECIAL_ANIMS_FOLDER,
                    name
                ],
            );
            let loader = model_loader(e);
            let kf = e.call(MODEL_LOADER_LOAD_KF, &args![loader, path]).u32();
            let queue_cloning = animation_should_queue_sequence_cloning(e, this);
            if kf != 0 {
                animation_add_animation(e, this, kf, queue_cloning);
            }
        });
        node = e.call(SIMPLE_LIST_NEXT, &args![node]).u32();
    }
}

// Translated from 00490400 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Animation::AddAnimationsFromList` (Xbox PDB): loads and adds every KF
/// file named in the `BSSimpleList` `list` (freeing each name and
/// removing it from the list), then deletes the list. Does nothing for a
/// null list. The scope guard of source line 750 is kept.
pub fn animation_add_animations_from_list(e: &mut Engine, this: Ptr<Animation>, list: Ptr) {
    if list.is_null() {
        return;
    }
    with_scope_guard(e, 0x2ee, |e| {
        let queue_cloning = animation_should_queue_sequence_cloning(e, this);
        while !e.call(SIMPLE_LIST_IS_EMPTY, &args![list]).bool() {
            let item = e.call(SIMPLE_LIST_ITEM, &args![list]).u32();
            let name = e.mem.u32(item);
            let loader = model_loader(e);
            let kf = e.call(MODEL_LOADER_LOAD_KF, &args![loader, name]).u32();
            animation_add_animation(e, this, kf, queue_cloning);
            e.call(OPERATOR_DELETE, &args![name]);
            e.call(SIMPLE_LIST_POP_FRONT, &args![list]);
        }
        if !list.is_null() {
            e.call(SIMPLE_LIST_DELETE, &args![list, 1u32]);
        }
    });
}

// Translated from 00490500 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Animation::AddAnimation` (Xbox PDB): adds the keyframe model `kf` to
/// the animation. Returns true when it was added (or was already there),
/// false when its group has no sequence type, the sequence cannot be added
/// to the controller manager, or a clamped looping sequence is refused.
///
/// - The group's sequence type indexes the per-type table; for types that
///   keep several sequences the existing entry of the group's id in
///   `pAnimSequenceMap` becomes an `AnimSequenceMultiple`, otherwise an
///   existing single sequence is replaced (unless it is the same group, or
///   one of the idles in use), after dropping it from the manager, from
///   `pCurrentSequence`/`group` and `pLastMovementSequence`.
/// - With `queue_cloning` (see `ShouldQueueSequenceCloning`), for types
///   other than 0 and 0x17 and while the Pip-Boy wait menu is not showing,
///   the model is only queued on `kfModelList` and true is returned.
/// - Otherwise the sequence is cloned (unless it is a
///   `BSAnimGroupSequence` with a single reference), stored in the map
///   entry, added to the controller manager, and the accumulation root and
///   group are registered (`AddGroup`).
///
/// The failure report loops over the sequence's objects with a null name
/// buffer, exactly as the game does, so it logs `(null)` for each object
/// the skeleton lacks. The scope guard of source line 794 is kept; the
/// compiler's exception frame is not translated.
pub fn animation_add_animation(
    e: &mut Engine,
    this: Ptr<Animation>,
    kf: u32,
    queue_cloning: bool,
) -> bool {
    e.with_stack(4, |e, group_ref| {
        let group = e.call(KF_MODEL_ANIM_GROUP, &args![kf]).u32();
        e.call(NI_POINTER_INIT, &args![group_ref, group]);
        let result = add_animation_with_group(e, this, kf, queue_cloning, group_ref.addr());
        e.call(NI_POINTER_RELEASE, &args![group_ref]);
        result
    })
}

/// `AddAnimation` after the animation group is held in `group_ref`.
fn add_animation_with_group(
    e: &mut Engine,
    this: Ptr<Animation>,
    kf: u32,
    queue_cloning: bool,
    group_ref: u32,
) -> bool {
    if ni_pointer_get(e, group_ref) == 0 {
        return false;
    }
    let group = ni_pointer_get(e, group_ref);
    let sequence_type = e.call(ANIM_GROUP_SEQUENCE_TYPE, &args![group]).i32();
    let group = ni_pointer_get(e, group_ref);
    let key = e.call(ANIM_GROUP_ID, &args![group]).u16();
    if sequence_type == 0xff {
        if e.call(KF_MODEL_SEQUENCE, &args![kf]).u32() != 0 {
            let file = ni_pointer_get(e, kf);
            let sequence = e.call(KF_MODEL_SEQUENCE, &args![kf]).u32();
            let sequence_name = object_name(e, sequence);
            e.call(
                LOG,
                &args![LOG_SEQUENCE_TYPE_NOT_FOUND, sequence_name, file],
            );
        }
        return false;
    }
    with_scope_guard(e, 0x31a, |e| {
        add_animation_body(e, this, kf, queue_cloning, sequence_type, key)
    })
}

/// The part of `AddAnimation` inside its scope guard.
fn add_animation_body(
    e: &mut Engine,
    this: Ptr<Animation>,
    kf: u32,
    queue_cloning: bool,
    sequence_type: i32,
    key: u16,
) -> bool {
    let a = this.addr();
    let map = e.get(this, Animation::pAnimSequenceMap).addr();
    let (found, mut entry) = e.with_stack(4, |e, cell| {
        e.mem.set_u32(cell.addr(), 0);
        let found = e.call(MAP_GET_AT, &args![map, key, cell]).bool();
        (found, e.mem.u32(cell.addr()))
    });
    if !found {
        let block = e.call(OPERATOR_NEW, &args![8u32]).u32();
        entry = if block == 0 {
            0
        } else {
            fn_00490e10(e, Ptr::new(block)).addr()
        };
        e.call(MAP_SET_AT, &args![map, key, entry]);
    }

    if e.vcall(entry, 0xc, &args![]).bool()
        && e.vcall(entry, 0x10, &args![0xffff_ffffu32]).u32() != 0
    {
        let keeps_several = e
            .mem
            .u8(SEQUENCE_TYPE_TABLE.wrapping_add((sequence_type as u32).wrapping_mul(0x24)))
            != 0;
        if keeps_several {
            e.call(MAP_REMOVE_AT, &args![map, key]);
            let old = entry;
            let block = e.call(OPERATOR_NEW, &args![8u32]).u32();
            entry = if block == 0 {
                0
            } else {
                fn_0048f0c0(e, Ptr::new(block), Ptr::new(old)).addr()
            };
            e.call(MAP_SET_AT, &args![map, key, entry]);
        } else {
            let current = e.vcall(entry, 0x10, &args![0xffff_ffffu32]).u32();
            let current_group = fn_0048f7f0(e, Ptr::new(current));
            let kf_group = e.call(KF_MODEL_ANIM_GROUP, &args![kf]).ptr::<()>();
            if current_group == kf_group {
                return true;
            }
            let mut replace = true;
            for slot in [a + 0x12c, a + 0x130] {
                if ni_pointer_get(e, slot) != 0 {
                    let idle = ni_pointer_get(e, slot);
                    let idle_sequence = fn_00490e40(e, Ptr::new(idle)).addr();
                    let current = e.vcall(entry, 0x10, &args![0xffff_ffffu32]).u32();
                    if idle_sequence == current {
                        replace = false;
                        break;
                    }
                }
            }
            if replace {
                let current = e.vcall(entry, 0x10, &args![0xffff_ffffu32]).u32();
                if current == e.get(this, Animation::pLastMovementSequence).addr() {
                    e.set(this, Animation::pLastMovementSequence, Ptr::NULL);
                }
                let current = e.vcall(entry, 0x10, &args![0xffff_ffffu32]).u32();
                let manager = ni_pointer_get(e, a + 0xd8);
                e.call(MANAGER_REMOVE_SEQUENCE, &args![manager, current]);
            }
            let current = e.vcall(entry, 0x10, &args![0xffff_ffffu32]).u32();
            if current != 0 {
                for i in 0..8u32 {
                    let slot = animation_entry(this, Animation::pCurrentSequence, i, 4);
                    if e.mem.u32(slot) == current {
                        e.mem.set_u32(slot, 0);
                        let group_slot = animation_entry(this, Animation::group, i, 2);
                        e.mem.set_u16(group_slot, 0);
                    }
                }
            }
            if e.get(this, Animation::pLastMovementSequence).addr() != 0 {
                let current = e.vcall(entry, 0x10, &args![0xffff_ffffu32]).u32();
                if current == e.get(this, Animation::pLastMovementSequence).addr() {
                    e.set(this, Animation::pLastMovementSequence, Ptr::NULL);
                }
            }
            e.vcall(entry, 4, &args![0u32, 1u32]);
        }
    }

    // Queue the model instead of adding it now.
    if queue_cloning
        && !e
            .call(IS_MENU_ID_VISIBLE, &args![MENU_ID_PIPBOY_WAIT, 0u32])
            .bool()
        && sequence_type != 0x17
        && sequence_type != 0
        && sequence_type != 0xe0
    {
        e.call(KF_MODEL_ADD_REF, &args![kf]);
        e.with_stack(4, |e, cell| {
            e.mem.set_u32(cell.addr(), kf);
            e.call(SIMPLE_LIST_PUSH_BACK, &args![a + 0x104, cell]);
        });
        return true;
    }

    let new_sequence = e.call(KF_MODEL_SEQUENCE, &args![kf]).u32();
    if (3..=0x10).contains(&sequence_type)
        && e.call(SEQUENCE_CYCLE_TYPE, &args![new_sequence]).u32() == 2
    {
        e.call(MAP_REMOVE_AT, &args![map, key]);
        if entry != 0 {
            e.vcall(entry, 0, &args![1u32]);
        }
        return false;
    }

    let use_sequence = if e
        .call(IS_KIND_OF, &args![RTTI_ANIM_GROUP_SEQUENCE, new_sequence])
        .bool()
        && e.call(SIMPLE_LIST_NEXT, &args![new_sequence]).u32() == 1
    {
        // The sequence has a single reference (`+4` is its count): reuse it.
        new_sequence
    } else {
        let block = e.call(NI_OPERATOR_NEW, &args![0x78u32]).u32();
        if block == 0 {
            0
        } else {
            let group = e.call(KF_MODEL_ANIM_GROUP, &args![kf]).u32();
            e.call(
                ANIM_GROUP_SEQUENCE_CONSTRUCT,
                &args![block, group, new_sequence],
            )
            .u32()
        }
    };
    e.vcall(entry, 4, &args![use_sequence, 1u32]);
    let manager = ni_pointer_get(e, a + 0xd8);
    let added = e
        .call(
            MANAGER_ADD_SEQUENCE,
            &args![manager, use_sequence, ACCUM_ROOT_NAME, 1u32],
        )
        .bool();
    if added {
        if e.get(this, Animation::pAccumRoot).addr() == 0 {
            let manager = ni_pointer_get(e, a + 0xd8);
            let accum_root = ni_controller_manager_get_accum_root(e, Ptr::new(manager));
            e.set(this, Animation::pAccumRoot, accum_root);
        }
        let group = e.call(KF_MODEL_ANIM_GROUP, &args![kf]).u32();
        e.call(ADD_GROUP, &args![this, group]);
        return true;
    }

    // The manager refused the sequence: report which objects the skeleton
    // lacks, undo, and give up.
    let mut removed = false;
    let root = ni_pointer_get(e, a + 8);
    let root_name = object_name(e, root);
    let sequence_name = object_name(e, use_sequence);
    e.call(LOG, &args![LOG_UNABLE_TO_ADD, sequence_name, root_name]);
    e.call(DISABLE_WARNINGS, &args![1u32]);
    let mut i = 0u32;
    while i < e.call(SEQUENCE_OBJECT_COUNT, &args![use_sequence]).u32() {
        // The name buffer the game passes here is null, so `GetObjectNameAt`
        // writes nothing and the name stays null.
        let name = 0u32;
        e.call(SEQUENCE_OBJECT_NAME_AT, &args![use_sequence, i, name]);
        let root = ni_pointer_get(e, a + 8);
        let missing = e.with_stack(4, |e, fixed| {
            e.call(FIXED_STRING_INIT, &args![fixed, name]);
            let found = e.vcall(root, 0x9c, &args![fixed]).u32();
            e.call(FIXED_STRING_FREE, &args![fixed]);
            found == 0
        });
        if missing {
            e.call(LOG, &args![LOG_OBJECT_NOT_IN_SKELETON, name]);
        }
        e.call(OPERATOR_DELETE, &args![name]);
        i += 1;
    }
    e.call(DISABLE_WARNINGS, &args![0u32]);
    if use_sequence != 0 {
        if use_sequence == e.get(this, Animation::pLastMovementSequence).addr() {
            e.set(this, Animation::pLastMovementSequence, Ptr::NULL);
        }
        let manager = ni_pointer_get(e, a + 0xd8);
        e.call(MANAGER_REMOVE_SEQUENCE, &args![manager, use_sequence]);
        removed = e.vcall(entry, 8, &args![use_sequence, 1u32]).bool();
    }
    if entry != 0 && removed {
        e.call(MAP_REMOVE_AT, &args![map, key]);
        e.vcall(entry, 0, &args![1u32]);
    }
    false
}

// Translated from 00490d90 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiControllerManager::GetAccumRoot` (Xbox PDB): for a cumulative
/// manager, the accumulation root of the first sequence that has an
/// object; 0 for a non-cumulative manager or none.
pub fn ni_controller_manager_get_accum_root(e: &mut Engine, this: Ptr<NiControllerManager>) -> Ptr {
    if e.get(this, NiControllerManager::m_bCumulative) {
        let array = this.addr() + NiControllerManager::m_kSequenceArray.off;
        let mut i = 0u32;
        while i < e.call(SEQUENCE_ARRAY_COUNT, &args![array]).u32() {
            let slot = e.call(SEQUENCE_ARRAY_ELEMENT, &args![array, i]).u32();
            if e.call(READ_WORD, &args![slot]).u32() != 0 {
                let slot = e.call(SEQUENCE_ARRAY_ELEMENT, &args![array, i]).u32();
                let sequence = e.call(READ_WORD, &args![slot]).u32();
                return e.call(ACCUM_ROOT_OF_SEQUENCE, &args![sequence]).ptr();
            }
            i += 1;
        }
    }
    Ptr::NULL
}

// Translated from 00490e10 (decompiled, FalloutNV.exe 1.4.0.525)
/// `AnimSequenceSingle` constructor (the map has no name for it): base
/// constructor, the single vtable, no sequence. Returns `this`.
pub fn fn_00490e10(e: &mut Engine, this: Ptr<AnimSequenceSingle>) -> Ptr<AnimSequenceSingle> {
    fn_0048f1b0(e, this.cast());
    e.mem.set_u32(this.addr(), VTABLE_ANIM_SEQUENCE_SINGLE);
    e.set(this, AnimSequenceSingle::pSeq, Ptr::NULL);
    this
}

// Translated from 00490e40 (decompiled, FalloutNV.exe 1.4.0.525)
/// `AnimIdle::pSeq` (+0x18): the sequence an idle plays (the map has no
/// name for it).
pub fn fn_00490e40(e: &mut Engine, this: Ptr) -> Ptr {
    let field = this.byte_add(0x18);
    e.call(READ_WORD, &args![field]).ptr()
}

// Translated from 00490e60 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Animation::ShouldQueueSequenceCloning` (Xbox PDB): true when a KF
/// should be queued instead of cloned right away. False without the
/// player object, its (loaded) 3D, or when the cloning setting at
/// `011c5778` is 0. When the animation root is the player's first or
/// second 3D root, or the player's `+0x69c` object accepts it
/// (`00822510`), the answer is whether the Pip-Boy wait menu is visible.
/// Otherwise false when any of the other checks hold (`0042ce10`, the
/// reference is the one at `0047b200`, menu mode, `00451530`).
pub fn animation_should_queue_sequence_cloning(e: &mut Engine, this: Ptr<Animation>) -> bool {
    let a = this.addr();
    let player: u32 = e.global(PLAYER_SINGLETON);
    if player == 0 {
        return false;
    }
    if e.call(PLAYER_NODE, &args![player, 0u32]).u32() == 0 {
        return false;
    }
    let setting = e.call(SETTING_VALUE_ADDRESS, &args![SETTING_CLONING]).u32();
    if e.mem.u32(setting) == 0 {
        return false;
    }
    let root = ni_pointer_get(e, a + 8);
    let first = e.call(PLAYER_NODE, &args![player, 0u32]).u32();
    let mut menu_check = root == first;
    if !menu_check {
        let root = ni_pointer_get(e, a + 8);
        let second = e.call(PLAYER_NODE, &args![player, 1u32]).u32();
        menu_check = root == second;
    }
    if !menu_check {
        let object = fn_00490f80(e, Ptr::new(player));
        menu_check = e.call(NODE_ACCEPTS_OBJECT, &args![a + 8, object]).bool();
    }
    if menu_check {
        return e
            .call(IS_MENU_ID_VISIBLE, &args![MENU_ID_PIPBOY_WAIT, 0u32])
            .bool();
    }
    let flag_object: u32 = e.global(FLAG_OBJECT_GLOBAL);
    if e.call(FLAG_OBJECT_CHECK, &args![flag_object]).bool() {
        return false;
    }
    let player_reference = e.call(PLAYER_REFERENCE, &args![]).u32();
    if e.get(this, Animation::pActorRef).addr() == player_reference {
        return false;
    }
    if e.call(IS_IN_MENU_MODE, &args![]).bool() {
        return false;
    }
    let tes: u32 = e.global(TES_GLOBAL);
    !e.call(TES_CHECK, &args![tes]).bool()
}

// Translated from 00490f80 (decompiled, FalloutNV.exe 1.4.0.525)
/// The pointer at `+0x69c` of the player object (`READ_WORD` on its
/// address); the map has no name for it.
pub fn fn_00490f80(e: &mut Engine, this: Ptr) -> Ptr {
    let field = this.byte_add(0x69c);
    e.call(READ_WORD, &args![field]).ptr()
}

// Translated from 00490fa0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Queues the keyframe model `kf` on `kfModelList` (two references added)
/// when it has an animation group; the map has no name for it.
pub fn fn_00490fa0(e: &mut Engine, this: Ptr<Animation>, kf: u32) {
    e.with_stack(4, |e, group_ref| {
        let group = e.call(KF_MODEL_ANIM_GROUP, &args![kf]).u32();
        e.call(NI_POINTER_INIT, &args![group_ref, group]);
        if ni_pointer_get(e, group_ref.addr()) != 0 {
            e.call(KF_MODEL_ADD_REF, &args![kf]);
            e.call(KF_MODEL_ADD_REF, &args![kf]);
            e.with_stack(4, |e, cell| {
                e.mem.set_u32(cell.addr(), kf);
                e.call(
                    SIMPLE_LIST_PUSH_BACK,
                    &args![this.addr() + Animation::kfModelList.off, cell],
                );
            });
        }
        e.call(NI_POINTER_RELEASE, &args![group_ref]);
    });
}

// Translated from 00491040 (decompiled, FalloutNV.exe 1.4.0.525)
/// The current sequence of animation group `group`: `pCurrentSequence`
/// slot 1 for group 0x14, slot 4 for group 0x15, otherwise slot `group`
/// (not range checked); the map has no name for it.
pub fn fn_00491040(e: &mut Engine, this: Ptr<Animation>, group: u32) -> Ptr {
    let slot = match group {
        0x14 => 1,
        0x15 => 4,
        other => other,
    };
    let address = animation_entry(this, Animation::pCurrentSequence, slot, 4);
    Ptr::new(e.mem.u32(address))
}

// Translated from 00491090 (decompiled, FalloutNV.exe 1.4.0.525)
/// The length of a sequence in seconds: 0.0 for a null sequence, otherwise
/// `end (00508100) - begin (00759450)`, both read as `float` from the
/// sequence (`__cdecl`, the sequence in the argument, result in ST0); the
/// map has no name for it.
pub fn fn_00491090(e: &mut Engine, sequence: Ptr) -> f32 {
    if sequence.is_null() {
        return 0.0;
    }
    let begin = e.call(SEQUENCE_BEGIN_TIME, &args![sequence]).f32();
    let end = e.call(SEQUENCE_END_TIME, &args![sequence]).f32();
    (end as f64 - begin as f64) as f32
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x0048ee70, fn_0048ee70(Ptr<AnimSequenceSingle>)),
        entry!(
            0x0048ef60,
            anim_sequence_base_scalar_deleting_destructor(Ptr, u32) -> Ptr
        ),
        entry!(0x0048ef90, fn_0048ef90(Ptr<AnimSequenceSingle>, u32) -> Ptr),
        entry!(0x0048efb0, fn_0048efb0(Ptr<AnimSequenceSingle>, u32) -> u8),
        entry!(
            0x0048efc0,
            anim_sequence_single_scalar_deleting_destructor(
                Ptr<AnimSequenceSingle>,
                u32,
            ) -> Ptr<AnimSequenceSingle>
        ),
        entry!(0x0048eff0, fn_0048eff0(Ptr, u32)),
        entry!(
            0x0048f010,
            anim_sequence_single_set_sequence(Ptr<AnimSequenceSingle>, Ptr, u8)
        ),
        entry!(
            0x0048f080,
            anim_sequence_single_remove_sequence(Ptr<AnimSequenceSingle>, Ptr, u8) -> bool
        ),
        entry!(
            0x0048f0c0,
            fn_0048f0c0(Ptr<AnimSequenceMultiple>, Ptr) -> Ptr<AnimSequenceMultiple>
        ),
        entry!(0x0048f1b0, fn_0048f1b0(Ptr) -> Ptr),
        entry!(
            0x0048f1d0,
            anim_sequence_multiple_scalar_deleting_destructor(
                Ptr<AnimSequenceMultiple>,
                u32,
            )
                -> Ptr<AnimSequenceMultiple>
        ),
        entry!(0x0048f200, fn_0048f200(Ptr) -> Ptr),
        entry!(0x0048f220, fn_0048f220(Ptr<AnimSequenceMultiple>)),
        entry!(0x0048f370, fn_0048f370(Ptr, u32) -> Ptr),
        entry!(
            0x0048f3a0,
            anim_sequence_multiple_set_sequence(Ptr<AnimSequenceMultiple>, Ptr, u32)
        ),
        entry!(
            0x0048f3d0,
            anim_sequence_multiple_remove_sequence(Ptr<AnimSequenceMultiple>, Ptr, u32) -> bool
        ),
        entry!(
            0x0048f450,
            anim_sequence_multiple_get_sequence_to_play(Ptr<AnimSequenceMultiple>, i8) -> Ptr
        ),
        entry!(
            0x0048f500,
            anim_sequence_multiple_get_corresponding_sequence(
                Ptr<AnimSequenceMultiple>,
                Ptr,
                i32,
            ) -> Ptr
        ),
        entry!(
            0x0048f720,
            anim_sequence_multiple_get_sequence_index(Ptr<AnimSequenceMultiple>, Ptr) -> u8
        ),
        entry!(
            0x0048f790,
            anim_sequence_multiple_get_sequence(Ptr<AnimSequenceMultiple>, Ptr) -> Ptr
        ),
        entry!(0x0048f7f0, fn_0048f7f0(Ptr<BSAnimGroupSequence>) -> Ptr),
        entry!(
            0x0048f810,
            animation_animation(Ptr<Animation>) -> Ptr<Animation>
        ),
        entry!(0x0048fb20, fn_0048fb20(Ptr<Animation>, u8, u8, u8)),
        entry!(0x0048fb50, fn_0048fb50(Ptr<Animation>)),
        entry!(
            0x0048fef0,
            ni_controller_manager_deactivate_all(Ptr<NiControllerManager>, f32)
        ),
        entry!(
            0x0048ff50,
            animation_shutdown_all_anim_idles(Ptr<Animation>)
        ),
        entry!(
            0x0048ffd0,
            fn_0048ffd0(Ptr<Animation>, Ptr, Ptr, Ptr, bool) -> bool
        ),
        entry!(0x004902f0, fn_004902f0(Ptr, u8)),
        entry!(0x00490310, fn_00490310(Ptr, Ptr, u32) -> u32),
        entry!(
            0x00490330,
            animation_add_special_animations(Ptr<Animation>, Ptr, u32)
        ),
        entry!(
            0x00490400,
            animation_add_animations_from_list(Ptr<Animation>, Ptr)
        ),
        entry!(
            0x00490500,
            animation_add_animation(Ptr<Animation>, u32, bool) -> bool
        ),
        entry!(
            0x00490d90,
            ni_controller_manager_get_accum_root(Ptr<NiControllerManager>) -> Ptr
        ),
        entry!(
            0x00490e10,
            fn_00490e10(Ptr<AnimSequenceSingle>) -> Ptr<AnimSequenceSingle>
        ),
        entry!(0x00490e40, fn_00490e40(Ptr) -> Ptr),
        entry!(
            0x00490e60,
            animation_should_queue_sequence_cloning(Ptr<Animation>) -> bool
        ),
        entry!(0x00490f80, fn_00490f80(Ptr) -> Ptr),
        entry!(0x00490fa0, fn_00490fa0(Ptr<Animation>, u32)),
        entry!(0x00491040, fn_00491040(Ptr<Animation>, u32) -> Ptr),
        entry!(0x00491090, fn_00491090(Ptr) -> f32),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Test support ----------------------------------------------------------

    fn ret(value: u32) -> Ret {
        Ret {
            eax: value,
            ..Ret::default()
        }
    }

    /// Doubles for the tiny getters nearly every function goes through, and
    /// the pages that hold the globals the functions read.
    fn engine() -> Engine {
        let mut e = Engine::new();
        for page in [
            0x0101_d000,
            0x0101_7000,
            0x0101_3000,
            0x0119_7000,
            0x0119_9000,
            0x011c_3000,
            0x011c_5000,
            0x011d_d000,
            0x011d_e000,
            0x011f_4000,
        ] {
            e.map(page, 0x1000);
        }
        e.register(READ_WORD, |e, a| ret(e.mem.u32(a[0])));
        e.register(NI_POINTER_INIT, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            ret(a[0])
        });
        e.register(NI_POINTER_SET, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            ret(a[0])
        });
        e.register(NI_POINTER_RELEASE, |e, a| {
            e.mem.set_u32(a[0], 0);
            Ret::default()
        });
        e.register(OPERATOR_NEW, |e, a| ret(e.mem.alloc(a[0])));
        e.register(NI_OPERATOR_NEW, |e, a| ret(e.mem.alloc(a[0])));
        e.register(OPERATOR_DELETE, |_, _| Ret::default());
        // `GetName` is the address of the name field at +8, whose word is
        // the C string.
        e.register(OBJECT_NAME, |_, a| ret(a[0] + 8));
        e.register(FIXED_STRING_CSTR, |e, a| ret(e.mem.u32(a[0])));
        e.register(REF_ADD, |_, _| Ret::default());
        e.register(REF_RELEASE, |_, _| Ret::default());
        e.register(SCOPE_GUARD_CTOR, |_, _| Ret::default());
        e.register(SCOPE_GUARD_DTOR, |_, _| Ret::default());
        e.set_global(MODEL_LOADER, 0x2222_0000u32);
        e
    }
    fn start_log(e: &mut Engine) {
        e.call_log = Some(vec![]);
    }

    /// The calls recorded since `start_log`, without the first one (the
    /// call of the function under test).
    fn peek_log(e: &Engine) -> Vec<(u32, Vec<u32>)> {
        e.call_log.clone().unwrap().into_iter().skip(1).collect()
    }

    /// Takes the recorded calls like [`peek_log`] and stops recording.
    fn end_log(e: &mut Engine) -> Vec<(u32, Vec<u32>)> {
        let log = peek_log(e);
        e.call_log = None;
        log
    }

    /// The addresses called since `start_log`, in order, without the call of
    /// the function under test.
    fn called(e: &mut Engine) -> Vec<u32> {
        end_log(e).into_iter().map(|(address, _)| address).collect()
    }

    /// The argument words of the calls to `address` in `log`.
    fn arguments_of(log: &[(u32, Vec<u32>)], address: u32) -> Vec<Vec<u32>> {
        log.iter()
            .filter(|(a, _)| *a == address)
            .map(|(_, words)| words.clone())
            .collect()
    }

    /// An object with a vtable whose `(offset, target)` slots are given.
    fn object_with_vtable(e: &mut Engine, size: u32, slots: &[(u32, u32)]) -> u32 {
        let vtable = e.mem.alloc(0x100);
        for (offset, target) in slots {
            e.mem.set_u32(vtable + offset, *target);
        }
        let object = e.mem.alloc(size);
        e.mem.set_u32(object, vtable);
        object
    }

    /// A NUL-terminated string in game memory.
    fn cstring(e: &mut Engine, text: &str) -> u32 {
        let address = e.mem.alloc(text.len() as u32 + 1);
        e.mem.write(address, text.as_bytes());
        address
    }

    fn read_cstring(e: &Engine, address: u32) -> String {
        let mut bytes = vec![];
        while e.mem.u8(address + bytes.len() as u32) != 0 {
            bytes.push(e.mem.u8(address + bytes.len() as u32));
        }
        String::from_utf8(bytes).unwrap()
    }

    /// An object that only has a name (`+8` holds the C string).
    fn named_object(e: &mut Engine, name: &str) -> u32 {
        let object = e.mem.alloc(0x80);
        let text = cstring(e, name);
        e.mem.set_u32(object + 8, text);
        object
    }

    /// An `NiTList`: header `{head, tail, count}` and nodes
    /// `{next, prev, item}`; doubles for its accessors.
    fn make_list(e: &mut Engine, items: &[u32]) -> u32 {
        e.register(LIST_NEXT, |e, a| ret(e.mem.u32(a[1])));
        e.register(LIST_ITEM_ADDRESS, |_, a| ret(a[1] + 8));
        e.register(LIST_COUNT, |e, a| ret(e.mem.u32(a[0] + 8)));
        let list = e.mem.alloc(12);
        let mut next = 0;
        for item in items.iter().rev() {
            let node = e.mem.alloc(12);
            e.mem.set_u32(node, next);
            e.mem.set_u32(node + 8, *item);
            next = node;
        }
        e.mem.set_u32(list, next);
        e.mem.set_u32(list + 8, items.len() as u32);
        list
    }

    /// A `BSSimpleList`: nodes `{item, next}`, the first one is the list;
    /// doubles for its accessors.
    fn make_simple_list(e: &mut Engine, items: &[u32]) -> u32 {
        e.register(SIMPLE_LIST_ITEM, |_, a| ret(a[0]));
        e.register(SIMPLE_LIST_NEXT, |e, a| ret(e.mem.u32(a[0] + 4)));
        e.register(SIMPLE_LIST_IS_EMPTY, |e, a| {
            ret((e.mem.u32(a[0]) == 0 && e.mem.u32(a[0] + 4) == 0) as u32)
        });
        e.register(SIMPLE_LIST_POP_FRONT, |e, a| {
            let next = e.mem.u32(a[0] + 4);
            if next == 0 {
                e.mem.set_u32(a[0], 0);
            } else {
                let item = e.mem.u32(next);
                let after = e.mem.u32(next + 4);
                e.mem.set_u32(a[0], item);
                e.mem.set_u32(a[0] + 4, after);
            }
            Ret::default()
        });
        let mut head = 0;
        let mut next = 0;
        for item in items.iter().rev() {
            let node = e.mem.alloc(8);
            e.mem.set_u32(node, *item);
            e.mem.set_u32(node + 4, next);
            next = node;
            head = node;
        }
        if head == 0 {
            head = e.mem.alloc(8);
        }
        head
    }

    /// Plain C string functions as doubles (`strrchr`, `strchr`, `strcpy`,
    /// `strcat`, case-insensitive compare).
    fn register_string_functions(e: &mut Engine) {
        e.register(STRRCHR, |e, a| {
            let text = read_cstring(e, a[0]);
            match text.rfind(a[1] as u8 as char) {
                Some(i) => ret(a[0] + i as u32),
                None => ret(0),
            }
        });
        e.register(STRCHR, |e, a| {
            let text = read_cstring(e, a[0]);
            match text.find(a[1] as u8 as char) {
                Some(i) => ret(a[0] + i as u32),
                None => ret(0),
            }
        });
        e.register(STRCPY, |e, a| {
            let text = read_cstring(e, a[1]);
            e.mem.write(a[0], text.as_bytes());
            e.mem.set_u8(a[0] + text.len() as u32, 0);
            ret(a[0])
        });
        e.register(STRCAT, |e, a| {
            let (head, tail) = (read_cstring(e, a[0]), read_cstring(e, a[1]));
            e.mem.write(a[0] + head.len() as u32, tail.as_bytes());
            e.mem.set_u8(a[0] + (head.len() + tail.len()) as u32, 0);
            ret(a[0])
        });
        e.register(STRICMP, |e, a| {
            let (x, y) = (read_cstring(e, a[0]), read_cstring(e, a[1]));
            ret((x.to_lowercase() != y.to_lowercase()) as u32)
        });
    }

    /// A single-sequence container holding `sequence`.
    fn single(e: &mut Engine, sequence: u32) -> Ptr<AnimSequenceSingle> {
        let this: Ptr<AnimSequenceSingle> = e.new_object();
        e.set(this, AnimSequenceSingle::pSeq, Ptr::new(sequence));
        this
    }

    /// A multiple-sequence container holding `items`.
    fn multiple(e: &mut Engine, items: &[u32]) -> (Ptr<AnimSequenceMultiple>, u32) {
        let list = make_list(e, items);
        let this: Ptr<AnimSequenceMultiple> = e.new_object();
        e.set(this, AnimSequenceMultiple::pSeqList, Ptr::new(list));
        (this, list)
    }

    // ---- AnimSequenceSingle ------------------------------------------------------------

    #[test]
    fn single_destructor_unloads_the_model_and_runs_the_base_destructor() {
        let mut e = engine();
        let sequence = named_object(&mut e, "Meshes\\a.kf");
        let this = single(&mut e, sequence);
        e.register(STRCPY_S, |_, _| Ret::default());
        e.register(MODEL_LOADER_REMOVE, |_, _| Ret::default());
        e.register(ANIM_SEQUENCE_BASE_DESTRUCT, |_, _| Ret::default());
        start_log(&mut e);
        e.call(0x0048_ee70, &args![this]);
        let log = peek_log(&e);
        assert_eq!(
            called(&mut e),
            [
                OBJECT_NAME,
                FIXED_STRING_CSTR,
                STRCPY_S,
                REF_RELEASE,
                MODEL_LOADER_REMOVE,
                ANIM_SEQUENCE_BASE_DESTRUCT
            ]
        );
        assert_eq!(e.mem.u32(this.addr()), VTABLE_ANIM_SEQUENCE_SINGLE);
        // The copy buffer is 0x104 bytes and the loader's name is that copy.
        let copy = arguments_of(&log, STRCPY_S)[0].clone();
        assert_eq!(copy[1], 0x104);
        assert_eq!(copy[2], e.mem.u32(sequence + 8));
        let remove = arguments_of(&log, MODEL_LOADER_REMOVE)[0].clone();
        assert_eq!(remove, [0x2222_0000, copy[0], 1]);
    }

    #[test]
    fn single_destructor_without_a_sequence_only_runs_the_base_destructor() {
        let mut e = engine();
        let this = single(&mut e, 0);
        e.register(ANIM_SEQUENCE_BASE_DESTRUCT, |_, _| Ret::default());
        start_log(&mut e);
        e.call(0x0048_ee70, &args![this]);
        assert_eq!(called(&mut e), [ANIM_SEQUENCE_BASE_DESTRUCT]);
        assert_eq!(e.mem.u32(this.addr()), VTABLE_ANIM_SEQUENCE_SINGLE);
    }

    #[test]
    fn base_scalar_deleting_destructor_frees_only_when_asked() {
        let mut e = engine();
        let this = Ptr::<()>::new(e.mem.alloc(8));
        e.register(ANIM_SEQUENCE_BASE_DESTRUCT, |_, _| Ret::default());
        start_log(&mut e);
        assert_eq!(e.call(0x0048_ef60, &args![this, 0u32]).ptr::<()>(), this);
        assert_eq!(called(&mut e), [ANIM_SEQUENCE_BASE_DESTRUCT]);
        start_log(&mut e);
        e.call(0x0048_ef60, &args![this, 1u32]);
        assert_eq!(
            called(&mut e),
            [ANIM_SEQUENCE_BASE_DESTRUCT, OPERATOR_DELETE]
        );
    }

    #[test]
    fn single_get_sequence_returns_the_held_sequence() {
        let mut e = engine();
        let this = single(&mut e, 0x1234);
        assert_eq!(e.call(0x0048_ef90, &args![this, 7u32]).u32(), 0x1234);
    }

    #[test]
    fn single_get_sequence_index_is_always_ff() {
        let mut e = engine();
        let this = single(&mut e, 0x1234);
        assert_eq!(e.call(0x0048_efb0, &args![this, 0x1234u32]).u8(), 0xff);
    }

    #[test]
    fn single_scalar_deleting_destructor_frees_only_when_asked() {
        let mut e = engine();
        let this = single(&mut e, 0);
        e.register(ANIM_SEQUENCE_BASE_DESTRUCT, |_, _| Ret::default());
        start_log(&mut e);
        e.call(0x0048_efc0, &args![this, 0u32]);
        assert_eq!(called(&mut e), [ANIM_SEQUENCE_BASE_DESTRUCT]);
        start_log(&mut e);
        let back = e.call(0x0048_efc0, &args![this, 1u32]).ptr::<()>();
        assert_eq!(back.addr(), this.addr());
        assert_eq!(
            called(&mut e),
            [ANIM_SEQUENCE_BASE_DESTRUCT, OPERATOR_DELETE]
        );
    }

    #[test]
    fn unload_model_calls_the_loader_with_the_name_and_one() {
        let mut e = engine();
        e.register(MODEL_LOADER_REMOVE, |_, _| Ret::default());
        start_log(&mut e);
        e.call(0x0048_eff0, &args![0x2222_0000u32, 0x5555u32]);
        let log = end_log(&mut e);
        assert_eq!(
            arguments_of(&log, MODEL_LOADER_REMOVE),
            [[0x2222_0000, 0x5555, 1]]
        );
    }

    #[test]
    fn single_set_sequence_replaces_unloads_and_references() {
        let mut e = engine();
        e.register(MODEL_LOADER_REMOVE, |_, _| Ret::default());
        let old = named_object(&mut e, "old.kf");
        let new = named_object(&mut e, "new.kf");
        let this = single(&mut e, old);

        // Free the old model: unload by name, release old, reference new.
        start_log(&mut e);
        e.call(0x0048_f010, &args![this, new, 1u8]);
        let log = peek_log(&e);
        assert_eq!(
            called(&mut e),
            [
                OBJECT_NAME,
                FIXED_STRING_CSTR,
                MODEL_LOADER_REMOVE,
                REF_RELEASE,
                REF_ADD
            ]
        );
        assert_eq!(arguments_of(&log, REF_RELEASE), [[old]]);
        assert_eq!(arguments_of(&log, REF_ADD), [[new]]);
        assert_eq!(e.get(this, AnimSequenceSingle::pSeq).addr(), new);

        // Keep the model loaded: no unload.
        start_log(&mut e);
        e.call(0x0048_f010, &args![this, 0u32, 0u8]);
        assert_eq!(called(&mut e), [REF_RELEASE]);
        assert!(e.get(this, AnimSequenceSingle::pSeq).is_null());

        // Nothing held: no release.
        start_log(&mut e);
        e.call(0x0048_f010, &args![this, new, 1u8]);
        assert_eq!(called(&mut e), [REF_ADD]);
    }

    #[test]
    fn single_remove_sequence_clears_only_the_held_sequence() {
        let mut e = engine();
        let this = object_with_vtable(&mut e, 8, &[(4, 0x00f0_0001)]);
        e.mem.set_u32(this + 4, 0x4000);
        e.register(0x00f0_0001, |_, _| Ret::default());
        start_log(&mut e);
        assert!(!e.call(0x0048_f080, &args![this, 0x9999u32, 1u8]).bool());
        assert!(called(&mut e).is_empty());
        start_log(&mut e);
        assert!(e.call(0x0048_f080, &args![this, 0x4000u32, 1u8]).bool());
        let log = end_log(&mut e);
        // `SetSequence(0, free_model)` through slot +4.
        assert_eq!(arguments_of(&log, 0x00f0_0001), [[this, 0, 1]]);
    }

    // ---- AnimSequenceMultiple ------------------------------------------------------------

    #[test]
    fn multiple_constructor_moves_the_sequence_and_deletes_the_other() {
        let mut e = engine();
        let other = object_with_vtable(
            &mut e,
            8,
            &[(0, 0x00f0_0000), (4, 0x00f0_0001), (0x10, 0x00f0_0010)],
        );
        e.register(0x00f0_0000, |_, _| Ret::default());
        e.register(0x00f0_0001, |_, _| Ret::default());
        e.register(0x00f0_0010, |_, _| ret(0x7777));
        e.register(LIST_CONSTRUCT, |_, _| Ret::default());
        e.register(LIST_ADD_HEAD, |_, _| Ret::default());
        let this: Ptr<AnimSequenceMultiple> = e.new_object();
        start_log(&mut e);
        let back = e.call(0x0048_f0c0, &args![this, other]).ptr::<()>();
        let log = end_log(&mut e);
        assert_eq!(back.addr(), this.addr());
        assert_eq!(e.mem.u32(this.addr()), VTABLE_ANIM_SEQUENCE_MULTIPLE);
        let list = e.get(this, AnimSequenceMultiple::pSeqList);
        assert!(!list.is_null());
        // GetSequenceToPlay(-1, 1), SetSequence(0, 0), delete(1) on `other`.
        assert_eq!(arguments_of(&log, 0x00f0_0010), [[other, 0xffff_ffff, 1]]);
        assert_eq!(arguments_of(&log, 0x00f0_0001), [[other, 0, 0]]);
        assert_eq!(arguments_of(&log, 0x00f0_0000), [[other, 1]]);
        assert_eq!(arguments_of(&log, REF_ADD), [[0x7777]]);
        assert_eq!(arguments_of(&log, LIST_ADD_HEAD)[0][0], list.addr());
    }

    #[test]
    fn base_constructor_sets_the_base_vtable() {
        let mut e = engine();
        let this = Ptr::<()>::new(e.mem.alloc(8));
        assert_eq!(e.call(0x0048_f1b0, &args![this]).ptr::<()>(), this);
        assert_eq!(e.mem.u32(this.addr()), VTABLE_ANIM_SEQUENCE_BASE);
    }

    #[test]
    fn multiple_scalar_deleting_destructor_frees_only_when_asked() {
        let mut e = engine();
        let (this, _) = multiple(&mut e, &[]);
        e.register(LIST_HEAD, |_, _| ret(0));
        e.register(LIST_DESTRUCT, |_, _| Ret::default());
        e.register(ANIM_SEQUENCE_BASE_DESTRUCT, |_, _| Ret::default());
        start_log(&mut e);
        e.call(0x0048_f1d0, &args![this, 0u32]);
        // The destructor deletes the (empty) list with flags 1, never the
        // object itself.
        assert!(!arguments_of(&end_log(&mut e), OPERATOR_DELETE)
            .iter()
            .any(|words| words[0] == this.addr()));
        let (this, _) = multiple(&mut e, &[]);
        start_log(&mut e);
        e.call(0x0048_f1d0, &args![this, 1u32]);
        let log = end_log(&mut e);
        assert_eq!(log.last().unwrap(), &(OPERATOR_DELETE, vec![this.addr()]));
        assert_eq!(e.mem.u32(this.addr()), VTABLE_ANIM_SEQUENCE_MULTIPLE);
    }

    #[test]
    fn list_constructor_forwards_and_returns_this() {
        let mut e = engine();
        e.register(LIST_CONSTRUCT, |_, _| Ret::default());
        start_log(&mut e);
        let this = Ptr::<()>::new(0x3000_0000);
        assert_eq!(e.call(0x0048_f200, &args![this]).ptr::<()>(), this);
        assert_eq!(called(&mut e), [LIST_CONSTRUCT]);
    }

    #[test]
    fn multiple_destructor_unloads_every_sequence_then_deletes_the_list() {
        let mut e = engine();
        e.register(STRCPY_S, |_, _| Ret::default());
        e.register(MODEL_LOADER_REMOVE, |_, _| Ret::default());
        e.register(LIST_DESTRUCT, |_, _| Ret::default());
        e.register(ANIM_SEQUENCE_BASE_DESTRUCT, |_, _| Ret::default());
        let first = named_object(&mut e, "a.kf");
        let second = named_object(&mut e, "b.kf");
        let (this, list) = multiple(&mut e, &[first, second]);
        start_log(&mut e);
        e.call(0x0048_f220, &args![this]);
        let log = end_log(&mut e);
        assert_eq!(arguments_of(&log, REF_RELEASE), [[first], [second]]);
        assert_eq!(arguments_of(&log, MODEL_LOADER_REMOVE).len(), 2);
        // The list is destroyed and freed (flags 1), then the base runs.
        assert_eq!(arguments_of(&log, LIST_DESTRUCT), [[list]]);
        assert_eq!(arguments_of(&log, OPERATOR_DELETE), [[list]]);
        assert_eq!(log.last().unwrap().0, ANIM_SEQUENCE_BASE_DESTRUCT);
        assert_eq!(e.mem.u32(this.addr()), VTABLE_ANIM_SEQUENCE_MULTIPLE);
    }

    #[test]
    fn multiple_destructor_without_a_list_only_runs_the_base_destructor() {
        let mut e = engine();
        e.register(ANIM_SEQUENCE_BASE_DESTRUCT, |_, _| Ret::default());
        let this: Ptr<AnimSequenceMultiple> = e.new_object();
        start_log(&mut e);
        e.call(0x0048_f220, &args![this]);
        assert_eq!(called(&mut e), [ANIM_SEQUENCE_BASE_DESTRUCT]);
    }

    #[test]
    fn list_deleting_destructor_frees_only_when_asked() {
        let mut e = engine();
        e.register(LIST_DESTRUCT, |_, _| Ret::default());
        let this = Ptr::<()>::new(0x3000_0000);
        start_log(&mut e);
        assert_eq!(e.call(0x0048_f370, &args![this, 0u32]).ptr::<()>(), this);
        assert_eq!(called(&mut e), [LIST_DESTRUCT]);
        start_log(&mut e);
        e.call(0x0048_f370, &args![this, 1u32]);
        assert_eq!(called(&mut e), [LIST_DESTRUCT, OPERATOR_DELETE]);
    }

    #[test]
    fn multiple_set_sequence_references_and_adds_at_the_head() {
        let mut e = engine();
        let (this, list) = multiple(&mut e, &[]);
        let seen = std::rc::Rc::new(std::cell::Cell::new(0u32));
        let seen_in_double = seen.clone();
        e.register_double(LIST_ADD_HEAD, move |e, a| {
            // The item is passed by address of a cell holding the sequence.
            seen_in_double.set(e.mem.u32(a[1]));
            Ret::default()
        });
        start_log(&mut e);
        e.call(0x0048_f3a0, &args![this, 0x4242u32, 0u32]);
        let log = end_log(&mut e);
        assert_eq!(arguments_of(&log, REF_ADD), [[0x4242]]);
        assert_eq!(arguments_of(&log, LIST_ADD_HEAD)[0][0], list);
        assert_eq!(seen.get(), 0x4242);
    }

    #[test]
    fn multiple_remove_sequence_deletes_the_list_when_it_empties() {
        let mut e = engine();
        e.register(LIST_REMOVE, |_, _| Ret::default());
        e.register(LIST_DESTRUCT, |_, _| Ret::default());
        e.register(LIST_IS_EMPTY, |_, _| ret(0));
        let (this, list) = multiple(&mut e, &[1]);
        start_log(&mut e);
        assert!(!e.call(0x0048_f3d0, &args![this, 0x4242u32, 0u32]).bool());
        let log = end_log(&mut e);
        assert_eq!(arguments_of(&log, REF_RELEASE), [[0x4242]]);
        assert_eq!(e.get(this, AnimSequenceMultiple::pSeqList).addr(), list);
        assert!(arguments_of(&log, LIST_DESTRUCT).is_empty());

        e.register(LIST_IS_EMPTY, |_, _| ret(1));
        start_log(&mut e);
        assert!(e.call(0x0048_f3d0, &args![this, 0x4242u32, 0u32]).bool());
        let log = end_log(&mut e);
        assert_eq!(arguments_of(&log, LIST_DESTRUCT), [[list]]);
        assert_eq!(arguments_of(&log, OPERATOR_DELETE), [[list]]);
        assert!(e.get(this, AnimSequenceMultiple::pSeqList).is_null());
    }

    #[test]
    fn multiple_get_sequence_to_play_by_index_and_random() {
        let mut e = engine();
        e.register(RANDOM, |_, _| ret(7));
        let (this, _) = multiple(&mut e, &[0xa, 0xb, 0xc]);
        // An index inside the list is used as is.
        assert_eq!(e.call(0x0048_f450, &args![this, 2i8]).u32(), 0xc);
        // -1 picks `random % count` = 7 % 3 = 1.
        assert_eq!(e.call(0x0048_f450, &args![this, -1i8]).u32(), 0xb);
        // An index past the end also picks at random.
        assert_eq!(e.call(0x0048_f450, &args![this, 5i8]).u32(), 0xb);
    }

    #[test]
    fn multiple_get_corresponding_sequence_matches_file_names() {
        let mut e = engine();
        register_string_functions(&mut e);
        let wanted = named_object(&mut e, "Meshes\\Chars\\Idle.kf");
        let other = named_object(&mut e, "Meshes\\Chars\\Walk.kf");
        let matching = named_object(&mut e, "meshes\\other\\IDLE.KF");
        let (this, _) = multiple(&mut e, &[other, matching]);
        // Same file name, case-insensitively.
        assert_eq!(
            e.call(0x0048_f500, &args![this, wanted, 0u32]).u32(),
            matching
        );
        // A name without a backslash matches nothing.
        let bare = named_object(&mut e, "Idle.kf");
        assert_eq!(e.call(0x0048_f500, &args![this, bare, 0u32]).u32(), 0);
        // No sequence with that file name.
        let missing = named_object(&mut e, "x\\Run.kf");
        assert_eq!(e.call(0x0048_f500, &args![this, missing, 0u32]).u32(), 0);
    }

    #[test]
    fn multiple_get_corresponding_sequence_rewrites_the_name_for_kinds_5_and_6() {
        let mut e = engine();
        register_string_functions(&mut e);
        // The suffix strings live at the exe's addresses.
        e.mem.write(SUFFIX_KIND_5, b"XX\0");
        e.mem.write(SUFFIX_KIND_6, b"YY\0");
        let query = named_object(&mut e, "m\\Foo_Bar.kf");
        // Kind 5 rewrites "\\Foo_Bar.kf" to "\\FooXX_Bar.kf".
        let for_five = named_object(&mut e, "m\\fooxx_bar.kf");
        let for_six = named_object(&mut e, "m\\fooyy_bar.kf");
        let (this, _) = multiple(&mut e, &[for_six, for_five]);
        assert_eq!(
            e.call(0x0048_f500, &args![this, query, 5u32]).u32(),
            for_five
        );
        assert_eq!(
            e.call(0x0048_f500, &args![this, query, 6u32]).u32(),
            for_six
        );
        // Without an underscore the suffix is appended.
        let plain = named_object(&mut e, "m\\Foo.kf");
        let appended = named_object(&mut e, "m\\Foo.kfXX");
        let (this, _) = multiple(&mut e, &[appended]);
        assert_eq!(
            e.call(0x0048_f500, &args![this, plain, 5u32]).u32(),
            appended
        );
    }

    #[test]
    fn multiple_get_sequence_index_counts_positions() {
        let mut e = engine();
        let (this, _) = multiple(&mut e, &[0xa, 0xb, 0xc]);
        assert_eq!(e.call(0x0048_f720, &args![this, 0xcu32]).u8(), 2);
        assert_eq!(e.call(0x0048_f720, &args![this, 0xau32]).u8(), 0);
        assert_eq!(e.call(0x0048_f720, &args![this, 0x99u32]).u8(), 0xff);
    }

    #[test]
    fn multiple_get_sequence_finds_the_sequence_by_group() {
        let mut e = engine();
        let first = e.mem.alloc(0x78);
        let second = e.mem.alloc(0x78);
        e.mem.set_u32(first + 0x74, 0x100);
        e.mem.set_u32(second + 0x74, 0x200);
        let (this, _) = multiple(&mut e, &[first, second]);
        assert_eq!(e.call(0x0048_f790, &args![this, 0x200u32]).u32(), second);
        assert_eq!(e.call(0x0048_f790, &args![this, 0x300u32]).u32(), 0);
    }

    #[test]
    fn sequence_group_getter_reads_offset_74() {
        let mut e = engine();
        let sequence = e.mem.alloc(0x78);
        e.mem.set_u32(sequence + 0x74, 0xabcd);
        assert_eq!(e.call(0x0048_f7f0, &args![sequence]).u32(), 0xabcd);
    }

    // ---- Animation ---------------------------------------------------------------------

    /// An `Animation` whose bytes are all `0xaa`, so the constructor's
    /// writes show.
    fn dirty_animation(e: &mut Engine) -> Ptr<Animation> {
        let this: Ptr<Animation> = e.new_object();
        e.mem.write(this.addr(), &[0xaa; 0x13c]);
        this
    }

    #[test]
    fn animation_constructor_sets_every_member() {
        let mut e = engine();
        for address in [NI_POINT3_CONSTRUCT, SIMPLE_LIST_CONSTRUCT, VECTOR_CONSTRUCT] {
            e.register(address, |_, _| Ret::default());
        }
        e.register(MAP_CONSTRUCT, |_, a| ret(a[0]));
        e.register(MEMSET, |e, a| {
            e.mem.write(a[0], &vec![a[1] as u8; a[2] as usize]);
            ret(a[0])
        });
        for (i, word) in [0x1111u32, 0x2222, 0x3333].iter().enumerate() {
            e.mem.set_u32(ZERO_VECTOR + 4 * i as u32, *word);
        }
        let this = dirty_animation(&mut e);
        start_log(&mut e);
        let back = e.call(0x0048_f810, &args![this]).ptr::<Animation>();
        let log = end_log(&mut e);
        assert_eq!(back, this);
        assert_eq!(e.get(this, Animation::m_uFlags), 0);
        assert_eq!(e.get(this, Animation::pAnimRoot), Ptr::NULL);
        assert!(e.get(this, Animation::pAccumRoot).is_null());
        assert!(e.get(this, Animation::pActorRef).is_null());
        assert!(e.get(this, Animation::pLastMovementSequence).is_null());
        for field in [
            Animation::movementDelta.off,
            Animation::AccumRootTranslate.off,
        ] {
            let words: Vec<u32> = (0..3)
                .map(|i| e.mem.u32(this.addr() + field + 4 * i))
                .collect();
            assert_eq!(words, [0x1111, 0x2222, 0x3333]);
        }
        assert_eq!(e.get(this, Animation::m_fLooking), 0.0);
        assert_eq!(e.get(this, Animation::cSkipUpdate), 0xff);
        assert_eq!(e.get(this, Animation::cSkipNextBlend), 0);
        assert_eq!(e.get(this, Animation::time), 0.0);
        assert!(!e.get(this, Animation::bShutDown));
        // 0x10 bytes of 0xff in `group` and `nextGroup`, 0x20 of -1 in the
        // action arrays, zero in the sound bones and current sequences.
        assert_eq!(e.mem.bytes(this.addr() + 0x4c, 0x10), [0xff; 0x10]);
        assert_eq!(e.mem.bytes(this.addr() + 0x9c, 0x10), [0xff; 0x10]);
        for offset in [0x5c, 0x7c, 0xac] {
            assert_eq!(e.mem.bytes(this.addr() + offset, 0x20), [0xff; 0x20]);
        }
        for offset in [0x28, 0xe0] {
            assert_eq!(e.mem.bytes(this.addr() + offset, 0x20), [0; 0x20]);
        }
        for field in [
            Animation::m_fMoveSpeed,
            Animation::m_fAttackSpeed,
            Animation::m_fGlobalTimeMultiplier,
            Animation::m_fReloadModifier,
            Animation::m_fEquipModifier,
        ] {
            assert_eq!(e.get(this, field), 1.0);
        }
        assert_eq!(e.get(this, Animation::sQueuedReloadGroup), 0xff);
        // The sequence map has 0x65 buckets and is built from a 0x10 byte
        // block.
        let map = e.get(this, Animation::pAnimSequenceMap);
        assert!(!map.is_null());
        assert_eq!(arguments_of(&log, MAP_CONSTRUCT), [[map.addr(), 0x65]]);
        assert_eq!(
            arguments_of(&log, VECTOR_CONSTRUCT),
            [[
                this.addr() + 0x12c,
                4,
                2,
                NI_POINTER_DEFAULT_CONSTRUCT,
                NI_POINTER_RELEASE
            ]]
        );
        // The first member constructed is the `pAnimRoot` NiPointer.
        assert_eq!(log[0], (NI_POINTER_INIT, vec![this.addr() + 8, 0]));
    }

    #[test]
    fn flag_bits_are_replaced_under_the_mask() {
        let mut e = engine();
        let this = dirty_animation(&mut e);
        e.set(this, Animation::m_uFlags, 0x0f);
        e.call(0x0048_fb20, &args![this, 3u8, 0xf0u8, 4u8]);
        assert_eq!(e.get(this, Animation::m_uFlags), 0x3f);
        // Value 0 under mask 0xff clears (what the constructor does).
        e.call(0x0048_fb20, &args![this, 0u8, 0xffu8, 0u8]);
        assert_eq!(e.get(this, Animation::m_uFlags), 0);
        // The shift count is taken modulo 32, as `SHL` does.
        e.call(0x0048_fb20, &args![this, 1u8, 0xffu8, 33u8]);
        assert_eq!(e.get(this, Animation::m_uFlags), 2);
    }

    /// Doubles `~Animation` needs besides the engine's own.
    fn register_destructor_callees(e: &mut Engine) {
        for address in [
            SPECIAL_IDLE_FREE,
            MODEL_LOADER_CANCEL_REPLACEMENT_KF_LIST,
            REMOVE_CONTROLLER,
            MANAGER_RESET,
            MAP_REMOVE_ALL,
            SIMPLE_LIST_CLEAR,
            SIMPLE_LIST_DESTRUCT,
            VECTOR_DESTRUCT,
            KF_MODEL_RELEASE,
        ] {
            e.register(address, |_, _| Ret::default());
        }
    }

    #[test]
    fn animation_destructor_tears_everything_down_in_order() {
        let mut e = engine();
        register_destructor_callees(&mut e);
        e.register(LIST_COUNT, |_, _| ret(0));
        e.register(GET_CONTROLLER, |_, a| {
            ret(if a[1] == RTTI_CONTROLLER_FIRST {
                0xc1
            } else {
                0xc2
            })
        });
        let target = object_with_vtable(&mut e, 0x20, &[(0xc, 0x00f0_000c)]);
        e.register(0x00f0_000c, |_, _| ret(1));
        // The manager's target is the object made above.
        e.register_double(MANAGER_TARGET, move |_, _| ret(target));
        // The map has one entry, an object that is deleted through its
        // vtable.
        let entry = object_with_vtable(&mut e, 8, &[(0, 0x00f0_0000)]);
        e.register(0x00f0_0000, |_, _| Ret::default());
        e.register(MAP_FIRST_POSITION, |_, _| ret(0x55));
        e.register_double(MAP_GET_NEXT, move |e, a| {
            // `GetNext(&pos, &key, &value)`: ends the iteration.
            e.mem.set_u32(a[1], 0);
            e.mem.set_u32(a[3], entry);
            Ret::default()
        });
        let map = object_with_vtable(&mut e, 0x10, &[(0, 0x00f0_0001)]);
        e.register(0x00f0_0001, |_, _| Ret::default());

        let this = dirty_animation(&mut e);
        let manager = 0xaaa0u32;
        let root = 0xbbb0u32;
        e.set(this, Animation::spManager, Ptr::new(manager));
        e.set(this, Animation::pAnimRoot, Ptr::new(root));
        e.set(this, Animation::pAnimSequenceMap, Ptr::new(map));
        e.set(this, Animation::pAccumRoot, Ptr::new(0x77));
        // Both "free when inactive" idles are empty.
        e.mem.set_u32(this.addr() + 0x12c, 0);
        e.mem.set_u32(this.addr() + 0x130, 0);
        // One KF model and one replay delay entry.
        let models = make_simple_list(&mut e, &[0x1357]);
        let (item, next) = (e.mem.u32(models), e.mem.u32(models + 4));
        e.mem.set_u32(this.addr() + 0x104, item);
        e.mem.set_u32(this.addr() + 0x108, next);
        let delays = make_simple_list(&mut e, &[0x2468]);
        let (item, next) = (e.mem.u32(delays), e.mem.u32(delays + 4));
        e.mem.set_u32(this.addr() + 0x134, item);
        e.mem.set_u32(this.addr() + 0x138, next);

        start_log(&mut e);
        e.call(0x0048_fb50, &args![this]);
        let log = end_log(&mut e);
        let addresses: Vec<u32> = log.iter().map(|(a, _)| *a).collect();
        let position = |address: u32| addresses.iter().position(|a| *a == address).unwrap();

        assert_eq!(addresses[0], SPECIAL_IDLE_FREE);
        assert!(position(SPECIAL_IDLE_FREE) < position(MODEL_LOADER_CANCEL_REPLACEMENT_KF_LIST));
        assert_eq!(
            arguments_of(&log, MODEL_LOADER_CANCEL_REPLACEMENT_KF_LIST),
            [[0x2222_0000, this.addr()]]
        );
        // Both controllers are removed from the root, then the manager
        // detaches from its target.
        assert_eq!(
            arguments_of(&log, GET_CONTROLLER),
            [
                [root, RTTI_CONTROLLER_FIRST],
                [root, RTTI_CONTROLLER_SECOND]
            ]
        );
        assert_eq!(
            arguments_of(&log, REMOVE_CONTROLLER),
            [[root, 0xc1], [root, 0xc2], [target, manager]]
        );
        assert_eq!(arguments_of(&log, MANAGER_RESET), [[manager]]);
        // The map entry, the KF model, the map and the replay entry go.
        assert_eq!(arguments_of(&log, 0x00f0_0000), [[entry, 1]]);
        assert_eq!(arguments_of(&log, KF_MODEL_RELEASE), [[0x1357]]);
        assert_eq!(arguments_of(&log, 0x00f0_0001), [[map, 1]]);
        assert_eq!(arguments_of(&log, OPERATOR_DELETE), [[0x2468]]);
        assert!(e.get(this, Animation::pAnimSequenceMap).is_null());
        assert!(e.get(this, Animation::pAccumRoot).is_null());
        assert!(e.get(this, Animation::spManager).is_null());
        // Members are destroyed last, in reverse order.
        let tail: Vec<(u32, Vec<u32>)> = log[log.len() - 7..].to_vec();
        let a = this.addr();
        assert_eq!(
            tail,
            [
                (SIMPLE_LIST_DESTRUCT, vec![a + 0x134]),
                (VECTOR_DESTRUCT, vec![a + 0x12c, 4, 2, NI_POINTER_RELEASE]),
                (NI_POINTER_RELEASE, vec![a + 0x128]),
                (NI_POINTER_RELEASE, vec![a + 0x124]),
                (SIMPLE_LIST_DESTRUCT, vec![a + 0x104]),
                (NI_POINTER_RELEASE, vec![a + 0xd8]),
                (NI_POINTER_RELEASE, vec![a + 8]),
            ]
        );
    }

    #[test]
    fn animation_destructor_without_a_manager_skips_the_controllers() {
        let mut e = engine();
        register_destructor_callees(&mut e);
        e.register(MAP_FIRST_POSITION, |_, _| ret(0));
        let this = dirty_animation(&mut e);
        e.set(this, Animation::spManager, Ptr::NULL);
        e.set(this, Animation::pAnimSequenceMap, Ptr::NULL);
        e.mem.write(this.addr() + 0x104, &[0; 8]);
        e.mem.write(this.addr() + 0x134, &[0; 8]);
        e.mem.write(this.addr() + 0x12c, &[0; 8]);
        e.register(SIMPLE_LIST_IS_EMPTY, |_, _| ret(1));
        start_log(&mut e);
        e.call(0x0048_fb50, &args![this]);
        let addresses = called(&mut e);
        assert!(!addresses.contains(&GET_CONTROLLER));
        assert!(!addresses.contains(&REMOVE_CONTROLLER));
        assert!(!addresses.contains(&MANAGER_RESET));
        // The empty replay list is neither walked nor cleared a second time.
        assert_eq!(
            addresses
                .iter()
                .filter(|a| **a == SIMPLE_LIST_CLEAR)
                .count(),
            1
        );
    }

    #[test]
    fn deactivate_all_deactivates_each_active_sequence() {
        let mut e = engine();
        let manager = e.mem.alloc(0x7c);
        // The active set at +0x44: elements pointer, allocated, used.
        let elements = e.mem.alloc(8);
        e.mem.set_u32(elements, 0xa1);
        e.mem.set_u32(elements + 4, 0xa2);
        e.mem.set_u32(manager + 0x44, elements);
        e.mem.set_u32(manager + 0x4c, 2);
        e.register(LIST_COUNT, |e, a| ret(e.mem.u32(a[0] + 8)));
        e.register(ACTIVE_SET_ELEMENT, |e, a| ret(e.mem.u32(a[0]) + 4 * a[1]));
        e.register(MANAGER_DEACTIVATE, |_, _| Ret::default());
        start_log(&mut e);
        e.call(0x0048_fef0, &args![manager, 0.25f32]);
        let log = end_log(&mut e);
        assert_eq!(
            arguments_of(&log, MANAGER_DEACTIVATE),
            [
                [manager, 0xa1, 0.25f32.to_bits()],
                [manager, 0xa2, 0.25f32.to_bits()]
            ]
        );
    }

    #[test]
    fn shutdown_all_anim_idles_frees_the_idles_that_are_set() {
        let mut e = engine();
        e.register(SPECIAL_IDLE_FREE, |_, _| Ret::default());
        e.register(ANIM_IDLE_FREE, |_, _| Ret::default());
        let this = dirty_animation(&mut e);
        e.mem.set_u32(this.addr() + 0x12c, 0x1234);
        e.mem.set_u32(this.addr() + 0x130, 0);
        e.set(this, Animation::bShutDown, false);
        start_log(&mut e);
        e.call(0x0048_ff50, &args![this]);
        let log = end_log(&mut e);
        assert!(e.get(this, Animation::bShutDown));
        assert_eq!(arguments_of(&log, SPECIAL_IDLE_FREE), [[this.addr(), 1, 1]]);
        // Only the first idle is freed and cleared.
        assert_eq!(
            arguments_of(&log, ANIM_IDLE_FREE),
            [[this.addr(), this.addr() + 0x12c]]
        );
        assert_eq!(
            arguments_of(&log, NI_POINTER_SET),
            [[this.addr() + 0x12c, 0]]
        );
    }

    /// A `NiTMapBase` double holding one entry for any key at `map + 4`.
    fn register_single_entry_map(e: &mut Engine) {
        e.register(MAP_GET_AT, |e, a| {
            let entry = e.mem.u32(a[0] + 4);
            if entry == 0 {
                return ret(0);
            }
            e.mem.set_u32(a[2], entry);
            ret(1)
        });
        e.register(MAP_SET_AT, |e, a| {
            e.mem.set_u32(a[0] + 4, a[2]);
            Ret::default()
        });
        e.register(MAP_REMOVE_AT, |e, a| {
            e.mem.set_u32(a[0] + 4, 0);
            Ret::default()
        });
    }

    #[test]
    fn setup_refuses_null_arguments_and_an_existing_group_entry() {
        let mut e = engine();
        register_single_entry_map(&mut e);
        let this: Ptr<Animation> = e.new_object();
        let list = make_simple_list(&mut e, &[]);
        let object = e.mem.alloc(0x40);
        start_log(&mut e);
        assert!(!e
            .call(0x0048_ffd0, &args![this, list, 0u32, object, true])
            .bool());
        let log = end_log(&mut e);
        // The scope guard is built and torn down around the refusal.
        assert_eq!(
            arguments_of(&log, SCOPE_GUARD_CTOR)[0][1..],
            [SCOPE_GUARD_TAG, 1, SOURCE_FILE, 0x238]
        );
        assert_eq!(log.last().unwrap().0, SCOPE_GUARD_DTOR);
        assert!(e.get(this, Animation::pActorRef).is_null());

        // An entry for group 0 already in the map refuses too.
        let map = e.mem.alloc(8);
        e.mem.set_u32(map + 4, 0x9999);
        e.set(this, Animation::pAnimSequenceMap, Ptr::new(map));
        assert!(!e
            .call(0x0048_ffd0, &args![this, list, object, object, true])
            .bool());
        assert_eq!(e.get(this, Animation::pActorRef).addr(), object);
        assert!(e.get(this, Animation::spManager).is_null());
    }

    #[test]
    fn setup_creates_the_manager_loads_the_list_and_plays_the_idle() {
        let mut e = engine();
        // The map has no entry until the (stubbed) loading adds one.
        register_single_entry_map(&mut e);
        let map = e.mem.alloc(8);
        let this: Ptr<Animation> = e.new_object();
        e.set(this, Animation::pAnimSequenceMap, Ptr::new(map));
        let root = e.mem.alloc(0x40);
        let actor = e.mem.alloc(0x40);
        let name = cstring(&mut e, "Meshes\\a.kf");
        let list = make_simple_list(&mut e, &[name]);

        // The reference's form type is not used; the bone table has one
        // name.
        e.register(REFERENCE_FORM, |_, _| ret(1));
        e.register(FORM_TYPE, |_, _| ret(FORM_TYPE_COMPARED));
        e.mem.set_u32(SOUND_PRIORITY_BONE_NAMES + 4, 0x4040);
        e.register(NODE_FIND_OBJECT, |_, a| ret(a[1] + 1));
        e.register(NODE_SET_FLAG, |_, _| Ret::default());
        e.register(MANAGER_CONSTRUCT, |_, a| ret(a[0] + 0x100));
        // The loader returns a KF without a group, so nothing is added; the
        // entry for the idle is put in the map by hand once loading is done.
        e.register(MODEL_LOADER_LOAD_KF, |_, _| ret(0x6000));
        e.register(KF_MODEL_ANIM_GROUP, |_, _| ret(0));
        e.register(SIMPLE_LIST_DELETE, |_, _| Ret::default());
        e.register(ACCUM_ROOT_SETUP, |_, _| Ret::default());
        let sequence = e.mem.alloc(0x80);
        let entry = object_with_vtable(&mut e, 8, &[(0x10, 0x00f0_0010)]);
        e.register_double(0x00f0_0010, move |_, _| ret(sequence));
        e.register(SEQUENCE_ACTIVATE, |_, _| Ret::default());
        e.register(TRANSLATION_CONSTRUCT, |_, _| Ret::default());
        e.register(NODE_SET_TRANSLATE, |_, _| Ret::default());
        e.register(0x00f0_008c, |_, _| Ret::default());
        // Loading removes nothing from the map; the pop-front double has
        // emptied the list by then and we install the entry now.
        e.register_double(SIMPLE_LIST_POP_FRONT, move |e, a| {
            e.mem.set_u32(a[0], 0);
            e.mem.set_u32(a[0] + 4, 0);
            e.mem.set_u32(map + 4, entry);
            Ret::default()
        });
        // `sequence` needs a vtable for the final virtual call.
        let sequence_vtable = e.mem.alloc(0x100);
        e.mem.set_u32(sequence, sequence_vtable);
        e.mem.set_u32(sequence_vtable + 0x8c, 0x00f0_008c);
        e.set(this, Animation::pAccumRoot, Ptr::new(0x5151));

        start_log(&mut e);
        let result = e
            .call(0x0048_ffd0, &args![this, list, root, actor, true])
            .bool();
        let log = end_log(&mut e);
        assert!(result);
        assert_eq!(e.get(this, Animation::pActorRef).addr(), actor);
        assert_eq!(e.get(this, Animation::pAnimRoot).addr(), root);
        // The bone found for the one named entry is stored at its index.
        assert_eq!(e.mem.u32(this.addr() + 0x28 + 4), 0x4041);
        assert_eq!(arguments_of(&log, NODE_FIND_OBJECT), [[root, 0x4040, 1]]);
        // The root's flag is set to 0, 2, and the manager is made for the
        // root and kept.
        assert_eq!(arguments_of(&log, NODE_SET_FLAG), [[root, 0, 2]]);
        let manager = e.get(this, Animation::spManager).addr();
        assert_ne!(manager, 0);
        assert_eq!(arguments_of(&log, MANAGER_CONSTRUCT)[0][1..], [root, 1]);
        // The listed file was loaded, passed to `AddAnimation` (its group
        // is null so nothing more happens), freed and removed from the list.
        assert_eq!(
            arguments_of(&log, MODEL_LOADER_LOAD_KF),
            [[0x2222_0000, name]]
        );
        assert_eq!(arguments_of(&log, OPERATOR_DELETE), [[name]]);
        assert_eq!(arguments_of(&log, SIMPLE_LIST_DELETE), [[list, 1]]);
        // The accumulation root is set up, then the idle sequence is
        // activated and the root's translation zeroed.
        assert_eq!(
            arguments_of(&log, ACCUM_ROOT_SETUP),
            [[0x5151, ACCUM_ROOT_SETUP_ARGUMENT]]
        );
        assert_eq!(
            arguments_of(&log, SEQUENCE_ACTIVATE),
            [[sequence, 0x64, 1, 1.0f32.to_bits(), 0.0f32.to_bits(), 0, 0]]
        );
        assert_eq!(
            arguments_of(&log, TRANSLATION_CONSTRUCT)[0][1..],
            [0.0f32.to_bits(), 1, 0]
        );
        assert_eq!(arguments_of(&log, NODE_SET_TRANSLATE)[0][0], root);
        assert_eq!(
            arguments_of(&log, 0x00f0_008c),
            [[sequence, 0.0f32.to_bits(), 0]]
        );
    }

    #[test]
    fn root_flag_setter_passes_the_flag_and_two() {
        let mut e = engine();
        e.register(NODE_SET_FLAG, |_, _| Ret::default());
        start_log(&mut e);
        e.call(0x0049_02f0, &args![0x3000u32, 1u8]);
        let log = end_log(&mut e);
        assert_eq!(arguments_of(&log, NODE_SET_FLAG), [[0x3000, 1, 2]]);
    }

    #[test]
    fn find_object_searches_the_root_by_name_with_a_one() {
        let mut e = engine();
        e.register(NODE_FIND_OBJECT, |_, a| ret(a[0] + a[1] + a[2]));
        start_log(&mut e);
        assert_eq!(
            e.call(0x0049_0310, &args![0x1u32, 0x100u32, 0x20u32]).u32(),
            0x121
        );
        let log = end_log(&mut e);
        assert_eq!(arguments_of(&log, NODE_FIND_OBJECT), [[0x100, 0x20, 1]]);
    }

    /// Doubles that make `AddAnimation` do nothing but report that the
    /// keyframe has no group.
    fn register_kf_without_group(e: &mut Engine) {
        e.register(MODEL_LOADER_LOAD_KF, |_, a| ret(a[1] + 0x1000));
        e.register(KF_MODEL_ANIM_GROUP, |_, _| ret(0));
    }

    #[test]
    fn special_animations_are_loaded_from_the_special_folder() {
        let mut e = engine();
        register_kf_without_group(&mut e);
        e.register(SPRINTF_S, |_, _| Ret::default());
        let this: Ptr<Animation> = e.new_object();
        let (first, second) = (cstring(&mut e, "a.kf"), cstring(&mut e, "b.kf"));
        let list = make_simple_list(&mut e, &[first, second]);
        let directory = cstring(&mut e, "Meshes");
        start_log(&mut e);
        e.call(0x0049_0330, &args![this, list, directory]);
        let log = end_log(&mut e);
        let formats = arguments_of(&log, SPRINTF_S);
        assert_eq!(formats.len(), 2);
        for (words, name) in formats.iter().zip([first, second]) {
            assert_eq!(
                words[1..],
                [
                    0x104,
                    SPECIAL_ANIMS_FORMAT,
                    directory,
                    SPECIAL_ANIMS_FOLDER,
                    name
                ]
            );
        }
        // The loader gets the formatted path, and every KF is handed on.
        assert_eq!(arguments_of(&log, MODEL_LOADER_LOAD_KF).len(), 2);
        assert_eq!(arguments_of(&log, KF_MODEL_ANIM_GROUP).len(), 2);
        // A null list does nothing.
        start_log(&mut e);
        e.call(0x0049_0330, &args![this, 0u32, directory]);
        assert!(called(&mut e).is_empty());
    }

    #[test]
    fn animations_from_a_list_are_loaded_freed_and_the_list_deleted() {
        let mut e = engine();
        register_kf_without_group(&mut e);
        e.register(SIMPLE_LIST_DELETE, |_, _| Ret::default());
        let this: Ptr<Animation> = e.new_object();
        let (first, second) = (cstring(&mut e, "a.kf"), cstring(&mut e, "b.kf"));
        let list = make_simple_list(&mut e, &[first, second]);
        start_log(&mut e);
        e.call(0x0049_0400, &args![this, list]);
        let log = end_log(&mut e);
        assert_eq!(
            arguments_of(&log, MODEL_LOADER_LOAD_KF),
            [[0x2222_0000, first], [0x2222_0000, second]]
        );
        assert_eq!(arguments_of(&log, OPERATOR_DELETE), [[first], [second]]);
        assert_eq!(arguments_of(&log, SIMPLE_LIST_DELETE), [[list, 1]]);
        assert_eq!(
            arguments_of(&log, SCOPE_GUARD_CTOR)[0][1..],
            [SCOPE_GUARD_TAG, 1, SOURCE_FILE, 0x2ee]
        );
        assert_eq!(log.last().unwrap().0, SCOPE_GUARD_DTOR);
        // A null list does nothing.
        start_log(&mut e);
        e.call(0x0049_0400, &args![this, 0u32]);
        assert!(called(&mut e).is_empty());
    }

    // ---- AddAnimation ------------------------------------------------------------------

    /// The `AnimSequenceSingle` and `AnimSequenceMultiple` vtables of the
    /// exe, written into memory, and doubles for the folded getters in
    /// them and for the destructor/list pieces they use.
    fn install_sequence_vtables(e: &mut Engine) {
        let single = [
            0x0048_efc0,
            0x0048_f010,
            0x0048_f080,
            0x008d_0360,
            0x0048_ef90,
            0x0048_ef90,
            0x0048_efb0,
        ];
        let multiple = [
            0x0048_f1d0,
            0x0048_f3a0,
            0x0048_f3d0,
            0x0047_c850,
            0x0048_f450,
            0x0048_f790,
            0x0048_f720,
        ];
        for (i, target) in single.iter().enumerate() {
            e.mem
                .set_u32(VTABLE_ANIM_SEQUENCE_SINGLE + 4 * i as u32, *target);
        }
        for (i, target) in multiple.iter().enumerate() {
            e.mem
                .set_u32(VTABLE_ANIM_SEQUENCE_MULTIPLE + 4 * i as u32, *target);
        }
        // `IsSingleSeq`: true for the single, false for the multiple.
        e.register(0x008d_0360, |_, _| ret(1));
        e.register(0x0047_c850, |_, _| ret(0));
        e.register(ANIM_SEQUENCE_BASE_DESTRUCT, |_, _| Ret::default());
        e.register(LIST_CONSTRUCT, |_, _| Ret::default());
        e.register(LIST_ADD_HEAD, |_, _| Ret::default());
    }

    /// Everything `AddAnimation` touches, with doubles that behave like the
    /// game for a one-entry sequence map.
    struct Fixture {
        e: Engine,
        this: Ptr<Animation>,
        kf: u32,
        sequence: u32,
        group: u32,
        map: u32,
        manager: u32,
        root: u32,
    }

    /// A keyframe model `kf` (`+4` the sequence, `+8` the group) of the
    /// given sequence type, and an animation with a manager and a root.
    fn add_fixture(sequence_type: u32) -> Fixture {
        let mut e = engine();
        install_sequence_vtables(&mut e);
        register_single_entry_map(&mut e);
        // The group: id at +0x10 (7), sequence type at +0x20 (the doubles
        // read it there).
        let group = e.mem.alloc(0x80);
        e.mem.set_u16(group + 0x10, 7);
        e.mem.set_u32(group + 0x20, sequence_type);
        // The sequence has one reference, a name and the group.
        let sequence = named_object(&mut e, "seq.kf");
        e.mem.set_u32(sequence + 4, 1);
        e.mem.set_u32(sequence + 0x74, group);
        let kf = e.mem.alloc(0x20);
        let file = cstring(&mut e, "Meshes\\file.kf");
        e.mem.set_u32(kf, file);
        e.mem.set_u32(kf + 4, sequence);
        e.mem.set_u32(kf + 8, group);
        e.register(KF_MODEL_ANIM_GROUP, |e, a| ret(e.mem.u32(a[0] + 8)));
        e.register(KF_MODEL_SEQUENCE, |e, a| ret(e.mem.u32(a[0] + 4)));
        e.register(ANIM_GROUP_SEQUENCE_TYPE, |e, a| ret(e.mem.u32(a[0] + 0x20)));
        e.register(ANIM_GROUP_ID, |e, a| ret(e.mem.u32(a[0] + 0x10) & 0xffff));
        e.register(IS_MENU_ID_VISIBLE, |_, _| ret(0));
        e.register(IS_KIND_OF, |_, _| ret(1));
        e.register(MODEL_LOADER_REMOVE, |_, _| Ret::default());
        e.register(SEQUENCE_CYCLE_TYPE, |_, _| ret(0));
        e.register(MANAGER_ADD_SEQUENCE, |_, _| ret(1));
        e.register(MANAGER_REMOVE_SEQUENCE, |_, _| Ret::default());
        e.register(ADD_GROUP, |_, _| Ret::default());
        e.register(KF_MODEL_ADD_REF, |_, _| Ret::default());
        e.register(SIMPLE_LIST_PUSH_BACK, |_, _| Ret::default());
        e.register(LOG, |_, _| Ret::default());
        e.register(DISABLE_WARNINGS, |_, _| Ret::default());
        e.register(SEQUENCE_OBJECT_COUNT, |_, _| ret(0));
        // `NiRefObject` count read (`+4`) through the node accessor.
        e.register(SIMPLE_LIST_NEXT, |e, a| ret(e.mem.u32(a[0] + 4)));

        let this: Ptr<Animation> = e.new_object();
        let map = e.mem.alloc(8);
        let manager = e.mem.alloc(0x7c);
        let root = object_with_vtable(&mut e, 0x40, &[(0x9c, 0x00f0_009c)]);
        e.register(0x00f0_009c, |_, _| ret(0));
        e.set(this, Animation::pAnimSequenceMap, Ptr::new(map));
        e.mem.set_u32(this.addr() + 0xd8, manager);
        e.mem.set_u32(this.addr() + 8, root);
        // A cumulative-less manager has no accumulation root, so set one.
        e.set(this, Animation::pAccumRoot, Ptr::new(0x5151));
        Fixture {
            e,
            this,
            kf,
            sequence,
            group,
            map,
            manager,
            root,
        }
    }

    impl Fixture {
        fn add(&mut self, queue_cloning: bool) -> bool {
            let (this, kf) = (self.this, self.kf);
            self.e
                .call(0x0049_0500, &args![this, kf, queue_cloning])
                .bool()
        }

        fn entry(&self) -> u32 {
            self.e.mem.u32(self.map + 4)
        }
    }

    #[test]
    fn add_animation_without_a_group_does_nothing() {
        let mut f = add_fixture(5);
        f.e.mem.set_u32(f.kf + 8, 0);
        start_log(&mut f.e);
        assert!(!f.add(false));
        let addresses = called(&mut f.e);
        // The group is held and released, and nothing else happens (no
        // scope guard either).
        assert_eq!(
            addresses,
            [
                KF_MODEL_ANIM_GROUP,
                NI_POINTER_INIT,
                READ_WORD,
                NI_POINTER_RELEASE
            ]
        );
    }

    #[test]
    fn add_animation_reports_a_group_without_a_sequence_type() {
        let mut f = add_fixture(0xff);
        start_log(&mut f.e);
        assert!(!f.add(false));
        let log = end_log(&mut f.e);
        let file = f.e.mem.u32(f.kf);
        let name = f.e.mem.u32(f.sequence + 8);
        assert_eq!(
            arguments_of(&log, LOG),
            [[LOG_SEQUENCE_TYPE_NOT_FOUND, name, file]]
        );
        assert!(arguments_of(&log, SCOPE_GUARD_CTOR).is_empty());
        assert_eq!(f.entry(), 0);
    }

    #[test]
    fn add_animation_queues_the_model_while_the_menu_allows_it() {
        let mut f = add_fixture(5);
        start_log(&mut f.e);
        assert!(f.add(true));
        let log = end_log(&mut f.e);
        assert_eq!(arguments_of(&log, KF_MODEL_ADD_REF), [[f.kf]]);
        let push = arguments_of(&log, SIMPLE_LIST_PUSH_BACK);
        assert_eq!(push[0][0], f.this.addr() + 0x104);
        // A fresh single-sequence entry was made for the group (key 7).
        assert_ne!(f.entry(), 0);
        assert_eq!(f.e.mem.u32(f.entry()), VTABLE_ANIM_SEQUENCE_SINGLE);
        assert_eq!(arguments_of(&log, MAP_SET_AT)[0][1], 7);
        // Nothing was added to the manager.
        assert!(arguments_of(&log, MANAGER_ADD_SEQUENCE).is_empty());
        assert_eq!(
            arguments_of(&log, SCOPE_GUARD_CTOR)[0][1..],
            [SCOPE_GUARD_TAG, 1, SOURCE_FILE, 0x31a]
        );
        assert_eq!(log.last().unwrap().0, NI_POINTER_RELEASE);

        // Types 0, 0x17 and 0xe0, or a visible menu, are added right away.
        for sequence_type in [0u32, 0x17, 0xe0] {
            let mut f = add_fixture(sequence_type);
            start_log(&mut f.e);
            assert!(f.add(true));
            let log = end_log(&mut f.e);
            assert!(arguments_of(&log, KF_MODEL_ADD_REF).is_empty());
            assert_eq!(arguments_of(&log, MANAGER_ADD_SEQUENCE).len(), 1);
        }
        let mut f = add_fixture(5);
        f.e.register(IS_MENU_ID_VISIBLE, |_, a| {
            ret((a[0] == MENU_ID_PIPBOY_WAIT) as u32)
        });
        start_log(&mut f.e);
        assert!(f.add(true));
        let log = end_log(&mut f.e);
        assert!(arguments_of(&log, KF_MODEL_ADD_REF).is_empty());
        assert_eq!(arguments_of(&log, MANAGER_ADD_SEQUENCE).len(), 1);
    }

    #[test]
    fn add_animation_adds_the_sequence_to_the_manager() {
        let mut f = add_fixture(5);
        start_log(&mut f.e);
        assert!(f.add(false));
        let log = end_log(&mut f.e);
        // The sequence has one reference and is a `BSAnimGroupSequence`, so
        // it is used as is, stored in the entry and added to the manager.
        assert!(arguments_of(&log, ANIM_GROUP_SEQUENCE_CONSTRUCT).is_empty());
        assert_eq!(
            f.e.get(
                Ptr::<AnimSequenceSingle>::new(f.entry()),
                AnimSequenceSingle::pSeq
            )
            .addr(),
            f.sequence
        );
        assert_eq!(
            arguments_of(&log, MANAGER_ADD_SEQUENCE),
            [[f.manager, f.sequence, ACCUM_ROOT_NAME, 1]]
        );
        assert_eq!(arguments_of(&log, ADD_GROUP), [[f.this.addr(), f.group]]);
        assert_eq!(f.e.get(f.this, Animation::pAccumRoot).addr(), 0x5151);

        // A sequence with other references is wrapped in a new
        // `BSAnimGroupSequence` (0x78 bytes).
        let mut f = add_fixture(5);
        f.e.mem.set_u32(f.sequence + 4, 2);
        f.e.register(ANIM_GROUP_SEQUENCE_CONSTRUCT, |_, a| ret(a[0]));
        start_log(&mut f.e);
        assert!(f.add(false));
        let log = end_log(&mut f.e);
        let wrapped = arguments_of(&log, ANIM_GROUP_SEQUENCE_CONSTRUCT);
        assert_eq!(wrapped[0][1..], [f.group, f.sequence]);
        assert_eq!(
            arguments_of(&log, MANAGER_ADD_SEQUENCE)[0][1],
            wrapped[0][0]
        );
    }

    #[test]
    fn add_animation_registers_the_accumulation_root_the_first_time() {
        let mut f = add_fixture(5);
        f.e.set(f.this, Animation::pAccumRoot, Ptr::NULL);
        // A manager that is not cumulative has no accumulation root.
        assert!(f.add(false));
        assert!(f.e.get(f.this, Animation::pAccumRoot).is_null());

        // A cumulative one reports the root of its first sequence.
        let mut f = add_fixture(5);
        f.e.set(f.this, Animation::pAccumRoot, Ptr::NULL);
        let slots = f.e.mem.alloc(4);
        f.e.mem.set_u32(slots, 0xd00d);
        f.e.mem.set_u32(f.manager + 0x34, slots);
        f.e.mem.set_u32(f.manager + 0x38, 1);
        f.e.mem.set_u8(f.manager + 0x68, 1);
        f.e.register(SEQUENCE_ARRAY_COUNT, |e, a| ret(e.mem.u32(a[0] + 4)));
        f.e.register(SEQUENCE_ARRAY_ELEMENT, |e, a| {
            ret(e.mem.u32(a[0]) + 4 * a[1])
        });
        f.e.register(ACCUM_ROOT_OF_SEQUENCE, |_, a| ret(a[0] + 1));
        assert!(f.add(false));
        assert_eq!(f.e.get(f.this, Animation::pAccumRoot).addr(), 0xd00e);
    }

    #[test]
    fn add_animation_refuses_a_clamped_sequence_for_the_cycling_types() {
        let mut f = add_fixture(5);
        f.e.register(SEQUENCE_CYCLE_TYPE, |_, _| ret(2));
        f.e.register(ANIM_SEQUENCE_BASE_DESTRUCT, |_, _| Ret::default());
        start_log(&mut f.e);
        assert!(!f.add(false));
        let log = end_log(&mut f.e);
        // The entry that was made for the group is removed again and
        // deleted; the manager never sees the sequence.
        assert_eq!(f.entry(), 0);
        assert_eq!(arguments_of(&log, MAP_REMOVE_AT), [[f.map, 7]]);
        assert!(arguments_of(&log, MANAGER_ADD_SEQUENCE).is_empty());
        assert_eq!(log.last().unwrap().0, NI_POINTER_RELEASE);

        // Type 2 is outside the range 3..=0x10: the cycle type does not
        // matter.
        let mut f = add_fixture(2);
        f.e.register(SEQUENCE_CYCLE_TYPE, |_, _| ret(2));
        assert!(f.add(false));
    }

    #[test]
    fn add_animation_returns_true_when_the_group_is_already_there() {
        let mut f = add_fixture(5);
        // An entry already holds a sequence of the same group.
        let held = f.e.mem.alloc(0x80);
        f.e.mem.set_u32(held + 0x74, f.group);
        let entry = single(&mut f.e, held);
        f.e.mem.set_u32(entry.addr(), VTABLE_ANIM_SEQUENCE_SINGLE);
        f.e.mem.set_u32(f.map + 4, entry.addr());
        start_log(&mut f.e);
        assert!(f.add(false));
        let addresses = called(&mut f.e);
        assert!(!addresses.contains(&MANAGER_ADD_SEQUENCE));
        assert!(!addresses.contains(&MANAGER_REMOVE_SEQUENCE));
        assert_eq!(addresses[addresses.len() - 2], SCOPE_GUARD_DTOR);
    }

    #[test]
    fn add_animation_replaces_an_existing_sequence_of_another_group() {
        let mut f = add_fixture(5);
        let old = f.e.mem.alloc(0x80);
        f.e.mem.set_u32(old + 0x74, 0xbeef);
        let entry = single(&mut f.e, old);
        f.e.mem.set_u32(entry.addr(), VTABLE_ANIM_SEQUENCE_SINGLE);
        f.e.mem.set_u32(f.map + 4, entry.addr());
        // The old sequence is current in slot 3 and the last movement
        // sequence.
        let this = f.this;
        f.e.mem.set_u32(this.addr() + 0xe0 + 12, old);
        f.e.mem.set_u16(this.addr() + 0x4c + 6, 9);
        f.e.set(this, Animation::pLastMovementSequence, Ptr::new(old));
        start_log(&mut f.e);
        assert!(f.add(false));
        let log = end_log(&mut f.e);
        assert_eq!(
            arguments_of(&log, MANAGER_REMOVE_SEQUENCE),
            [[f.manager, old]]
        );
        assert_eq!(f.e.mem.u32(this.addr() + 0xe0 + 12), 0);
        assert_eq!(f.e.mem.u16(this.addr() + 0x4c + 6), 0);
        assert!(f.e.get(this, Animation::pLastMovementSequence).is_null());
        // The entry now holds the new sequence.
        assert_eq!(f.e.get(entry, AnimSequenceSingle::pSeq).addr(), f.sequence);
        assert_eq!(arguments_of(&log, MANAGER_ADD_SEQUENCE).len(), 1);

        // An idle that plays the old sequence keeps it in the manager.
        let mut f = add_fixture(5);
        let old = f.e.mem.alloc(0x80);
        f.e.mem.set_u32(old + 0x74, 0xbeef);
        let entry = single(&mut f.e, old);
        f.e.mem.set_u32(entry.addr(), VTABLE_ANIM_SEQUENCE_SINGLE);
        f.e.mem.set_u32(f.map + 4, entry.addr());
        let idle = f.e.mem.alloc(0x38);
        f.e.mem.set_u32(idle + 0x18, old);
        f.e.mem.set_u32(f.this.addr() + 0x130, idle);
        start_log(&mut f.e);
        assert!(f.add(false));
        let log = end_log(&mut f.e);
        assert!(arguments_of(&log, MANAGER_REMOVE_SEQUENCE).is_empty());
        assert_eq!(f.e.get(entry, AnimSequenceSingle::pSeq).addr(), f.sequence);
    }

    #[test]
    fn add_animation_turns_the_entry_into_a_multiple_for_listed_types() {
        let mut f = add_fixture(5);
        // Type 5 keeps several sequences.
        f.e.mem.set_u8(SEQUENCE_TYPE_TABLE + 5 * 0x24, 1);
        let old = f.e.mem.alloc(0x80);
        f.e.mem.set_u32(old + 0x74, 0xbeef);
        let entry = single(&mut f.e, old);
        f.e.mem.set_u32(entry.addr(), VTABLE_ANIM_SEQUENCE_SINGLE);
        f.e.mem.set_u32(f.map + 4, entry.addr());
        start_log(&mut f.e);
        assert!(f.add(false));
        let log = end_log(&mut f.e);
        // The old entry was removed from the map, a multiple was built from
        // it and stored, and the multiple received the new sequence.
        assert_eq!(arguments_of(&log, MAP_REMOVE_AT), [[f.map, 7]]);
        assert_eq!(arguments_of(&log, MAP_SET_AT).len(), 1);
        assert_ne!(f.entry(), entry.addr());
        assert_eq!(f.e.mem.u32(f.entry()), VTABLE_ANIM_SEQUENCE_MULTIPLE);
        assert!(arguments_of(&log, REF_ADD).contains(&vec![old]));
        assert!(arguments_of(&log, REF_ADD).contains(&vec![f.sequence]));
        assert_eq!(arguments_of(&log, MANAGER_ADD_SEQUENCE).len(), 1);
    }

    #[test]
    fn add_animation_undoes_everything_when_the_manager_refuses() {
        let mut f = add_fixture(5);
        f.e.register(MANAGER_ADD_SEQUENCE, |_, _| ret(0));
        f.e.register(SEQUENCE_OBJECT_COUNT, |_, _| ret(2));
        f.e.register(SEQUENCE_OBJECT_NAME_AT, |_, _| Ret::default());
        f.e.register(FIXED_STRING_INIT, |_, _| Ret::default());
        f.e.register(FIXED_STRING_FREE, |_, _| Ret::default());
        f.e.register(ANIM_SEQUENCE_BASE_DESTRUCT, |_, _| Ret::default());
        f.e.register(MODEL_LOADER_REMOVE, |_, _| Ret::default());
        start_log(&mut f.e);
        assert!(!f.add(false));
        let log = end_log(&mut f.e);
        let root_name = f.e.mem.u32(f.root + 8);
        let _ = root_name;
        let sequence_name = f.e.mem.u32(f.sequence + 8);
        // The failure is logged with the sequence's name, warnings are
        // switched off around the object loop, and each of the two objects
        // (looked up with a null name) is reported as missing.
        let logged = arguments_of(&log, LOG);
        assert_eq!(logged[0][..2], [LOG_UNABLE_TO_ADD, sequence_name]);
        assert_eq!(arguments_of(&log, DISABLE_WARNINGS), [[1], [0]]);
        assert_eq!(
            arguments_of(&log, SEQUENCE_OBJECT_NAME_AT)
                .iter()
                .map(|w| (w[1], w[2]))
                .collect::<Vec<_>>(),
            [(0, 0), (1, 0)]
        );
        assert_eq!(
            logged[1..],
            [
                [LOG_OBJECT_NOT_IN_SKELETON, 0],
                [LOG_OBJECT_NOT_IN_SKELETON, 0]
            ]
        );
        assert_eq!(arguments_of(&log, OPERATOR_DELETE).len(), 2 + 1);
        // The sequence leaves the manager and the entry is removed from the
        // map and deleted.
        assert_eq!(
            arguments_of(&log, MANAGER_REMOVE_SEQUENCE),
            [[f.manager, f.sequence]]
        );
        assert_eq!(f.entry(), 0);
        assert_eq!(log.last().unwrap().0, NI_POINTER_RELEASE);
    }

    // ---- GetAccumRoot and the small getters ----------------------------------------------

    #[test]
    fn get_accum_root_takes_the_first_sequence_that_has_an_object() {
        let mut e = engine();
        let manager = e.mem.alloc(0x7c);
        let slots = e.mem.alloc(12);
        e.mem.set_u32(slots, 0);
        e.mem.set_u32(slots + 4, 0xaaaa);
        e.mem.set_u32(slots + 8, 0xbbbb);
        e.mem.set_u32(manager + 0x34, slots);
        e.mem.set_u32(manager + 0x38, 3);
        e.register(SEQUENCE_ARRAY_COUNT, |e, a| ret(e.mem.u32(a[0] + 4)));
        e.register(SEQUENCE_ARRAY_ELEMENT, |e, a| {
            ret(e.mem.u32(a[0]) + 4 * a[1])
        });
        e.register(ACCUM_ROOT_OF_SEQUENCE, |_, a| ret(a[0] + 1));
        // Not cumulative: null.
        assert_eq!(e.call(0x0049_0d90, &args![manager]).u32(), 0);
        e.mem.set_u8(manager + 0x68, 1);
        assert_eq!(e.call(0x0049_0d90, &args![manager]).u32(), 0xaaab);
        // Cumulative, every slot empty: null.
        e.mem.set_u32(slots + 4, 0);
        e.mem.set_u32(slots + 8, 0);
        assert_eq!(e.call(0x0049_0d90, &args![manager]).u32(), 0);
    }

    #[test]
    fn single_constructor_sets_the_vtable_and_clears_the_sequence() {
        let mut e = engine();
        let this = Ptr::<AnimSequenceSingle>::new(e.mem.alloc(8));
        e.mem.set_u32(this.addr() + 4, 0xdead);
        assert_eq!(
            e.call(0x0049_0e10, &args![this]).ptr::<()>().addr(),
            this.addr()
        );
        assert_eq!(e.mem.u32(this.addr()), VTABLE_ANIM_SEQUENCE_SINGLE);
        assert_eq!(e.mem.u32(this.addr() + 4), 0);
    }

    #[test]
    fn idle_sequence_getter_reads_offset_18() {
        let mut e = engine();
        let idle = e.mem.alloc(0x38);
        e.mem.set_u32(idle + 0x18, 0x7171);
        assert_eq!(e.call(0x0049_0e40, &args![idle]).u32(), 0x7171);
    }

    /// Doubles for `ShouldQueueSequenceCloning`: the player's nodes are
    /// `0xa000 + which`, the cloning setting is 1, and the other checks say
    /// "no".
    fn queue_engine() -> (Engine, Ptr<Animation>) {
        let mut e = engine();
        e.set_global(PLAYER_SINGLETON, 0x6000u32);
        e.register(PLAYER_NODE, |_, a| ret(0xa000 + a[1]));
        e.register(SETTING_VALUE_ADDRESS, |_, a| {
            ret(a[0] - SETTING_CLONING + 0x6100)
        });
        e.mem.map(0x6000, 0x1000);
        e.mem.set_u32(0x6100, 1);
        e.register(NODE_ACCEPTS_OBJECT, |_, _| ret(0));
        e.register(FLAG_OBJECT_CHECK, |_, _| ret(0));
        e.register(PLAYER_REFERENCE, |_, _| ret(0xcafe));
        e.register(IS_IN_MENU_MODE, |_, _| ret(0));
        e.register(TES_CHECK, |_, _| ret(0));
        e.register(IS_MENU_ID_VISIBLE, |_, _| ret(0));
        let this: Ptr<Animation> = e.new_object();
        e.mem.set_u32(this.addr() + 8, 0x9000);
        e.set(this, Animation::pActorRef, Ptr::new(0x1234));
        (e, this)
    }

    #[test]
    fn should_queue_cloning_follows_the_player_and_menu_state() {
        let (mut e, this) = queue_engine();
        // The ordinary case: nothing says no, so queue.
        assert!(e.call(0x0049_0e60, &args![this]).bool());

        // No player, no 3D, or the setting off: false.
        e.set_global(PLAYER_SINGLETON, 0u32);
        assert!(!e.call(0x0049_0e60, &args![this]).bool());
        let (mut e, this) = queue_engine();
        e.register(PLAYER_NODE, |_, _| ret(0));
        assert!(!e.call(0x0049_0e60, &args![this]).bool());
        let (mut e, this) = queue_engine();
        e.mem.set_u32(0x6100, 0);
        assert!(!e.call(0x0049_0e60, &args![this]).bool());

        // The player's own nodes (or an object the player's object
        // accepts) follow the Pip-Boy wait menu instead.
        for root in [0xa000u32, 0xa001] {
            let (mut e, this) = queue_engine();
            e.mem.set_u32(this.addr() + 8, root);
            assert!(!e.call(0x0049_0e60, &args![this]).bool());
            e.register(IS_MENU_ID_VISIBLE, |_, a| {
                ret((a[0] == MENU_ID_PIPBOY_WAIT) as u32)
            });
            assert!(e.call(0x0049_0e60, &args![this]).bool());
        }
        let (mut e, this) = queue_engine();
        e.register(NODE_ACCEPTS_OBJECT, |_, _| ret(1));
        assert!(!e.call(0x0049_0e60, &args![this]).bool());
        e.register(IS_MENU_ID_VISIBLE, |_, _| ret(1));
        assert!(e.call(0x0049_0e60, &args![this]).bool());

        // Each of the other checks says no.
        let (mut e, this) = queue_engine();
        e.register(FLAG_OBJECT_CHECK, |_, _| ret(1));
        assert!(!e.call(0x0049_0e60, &args![this]).bool());
        let (mut e, this) = queue_engine();
        e.register(PLAYER_REFERENCE, |_, _| ret(0x1234));
        assert!(!e.call(0x0049_0e60, &args![this]).bool());
        let (mut e, this) = queue_engine();
        e.register(IS_IN_MENU_MODE, |_, _| ret(1));
        assert!(!e.call(0x0049_0e60, &args![this]).bool());
        let (mut e, this) = queue_engine();
        e.register(TES_CHECK, |_, _| ret(1));
        assert!(!e.call(0x0049_0e60, &args![this]).bool());
    }

    #[test]
    fn player_object_getter_reads_offset_69c() {
        let mut e = engine();
        let player = e.mem.alloc(0x6a0);
        e.mem.set_u32(player + 0x69c, 0x8181);
        assert_eq!(e.call(0x0049_0f80, &args![player]).u32(), 0x8181);
    }

    #[test]
    fn queueing_a_model_needs_an_animation_group() {
        let mut e = engine();
        e.register(KF_MODEL_ADD_REF, |_, _| Ret::default());
        e.register(SIMPLE_LIST_PUSH_BACK, |_, _| Ret::default());
        e.register(KF_MODEL_ANIM_GROUP, |e, a| ret(e.mem.u32(a[0] + 8)));
        let this: Ptr<Animation> = e.new_object();
        let kf = e.mem.alloc(0x20);
        start_log(&mut e);
        e.call(0x0049_0fa0, &args![this, kf]);
        let log = end_log(&mut e);
        // No group: nothing is queued.
        assert!(arguments_of(&log, SIMPLE_LIST_PUSH_BACK).is_empty());
        assert_eq!(log.last().unwrap().0, NI_POINTER_RELEASE);
        e.mem.set_u32(kf + 8, 0x4444);
        start_log(&mut e);
        e.call(0x0049_0fa0, &args![this, kf]);
        let log = end_log(&mut e);
        assert_eq!(arguments_of(&log, KF_MODEL_ADD_REF), [[kf], [kf]]);
        assert_eq!(
            arguments_of(&log, SIMPLE_LIST_PUSH_BACK)[0][0],
            this.addr() + 0x104
        );
    }

    #[test]
    fn current_sequence_of_a_group_maps_the_special_groups() {
        let mut e = engine();
        let this: Ptr<Animation> = e.new_object();
        for slot in 0..8u32 {
            e.mem.set_u32(this.addr() + 0xe0 + 4 * slot, 0x100 + slot);
        }
        assert_eq!(e.call(0x0049_1040, &args![this, 0x14u32]).u32(), 0x101);
        assert_eq!(e.call(0x0049_1040, &args![this, 0x15u32]).u32(), 0x104);
        assert_eq!(e.call(0x0049_1040, &args![this, 2u32]).u32(), 0x102);
    }

    #[test]
    fn sequence_length_is_end_minus_begin() {
        let mut e = engine();
        e.register(SEQUENCE_BEGIN_TIME, |_, _| 1.5f32.into_ret());
        e.register(SEQUENCE_END_TIME, |_, _| 4.0f32.into_ret());
        assert_eq!(e.call(0x0049_1090, &args![0x1000u32]).f32(), 2.5);
        assert_eq!(e.call(0x0049_1090, &args![0u32]).f32(), 0.0);
    }
}
