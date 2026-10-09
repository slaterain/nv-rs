//! `fallout shared/modelloader.cpp` (Xbox PDB source unit), subsystem `fallout shared`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! The unit has 517 functions (`ledger queue "fallout shared/modelloader.cpp"`);
//! it is translated in address order, a session at a time. State of this file:
//! the first 120 functions, `0043aaf0` to `0043d850`. The first 40, to
//! `0043baa0`, are the `Model` and `KFModel` classes, `Model::InitModel` and
//! the small helpers the unit's compiler emitted next to them (`BSStream`,
//! `NiNode` and `NiFixedString` accessors, interlocked-operation wrappers).
//! The next 40, from `0043bac0`, are `LoadedFile`, `QueuedFile`,
//! `QueuedTexture` (constructors, `QueueMe`, `Run`, `Finish`, `Cancel`) and
//! the start of `QueuedModel`, with their flag, TLS and texture-map helpers.
//! The third 40, from `0043c890`, finish `QueuedModel` (constructors,
//! `CheckFinished`, `QueueMe`, `Run`, `Finish`, `GetDescription`), the
//! `BSStream` helpers `Run` needs, and begin the tree classes
//! (`QueuedTreeBillboard`, `QueuedTreeModel`).
//! The next session continues at `0043d8c0`.
//!
//! Layouts and helpers added here live at the top of the file, below.
//!
//! Not translated: the compiler's exception-unwinding frames (the `FS:[0]`
//! chains and state variables of the constructors, destructors and
//! `InitModel`) and the stack-cookie checks (`0043b7e0`, `0043bb80`, `0043bd10`). Locals the game
//! keeps on its stack and passes by address (the temporary `NiFixedString`s
//! and the 268-byte name buffer of `0043b7e0`) are heap blocks here, freed
//! where the game's scope ends.
//!
//! Stack arguments: several functions of the unit call a `thiscall` helper
//! with one stack argument pushed before the `this` is loaded. The
//! decompiler attaches such an argument to the wrong call when a helper
//! returns with a plain `RET` (see `0043b5b0`, `0043b7e0`); the translations
//! follow the disassembly.

#[allow(unused_imports)]
use crate::prelude::*;
use crate::types::NiFixedString;

/// `NiPointer<T>::operator T*` (`00559450`): returns the pointer stored at
/// the `NiPointer`'s address.
const NI_POINTER_GET: u32 = 0x0055_9450;
/// `NiPointer<T>::operator=(T*)` (`0066b0d0`): releases the old object,
/// stores and adds a reference to the new one; returns the `NiPointer`.
const NI_POINTER_ASSIGN: u32 = 0x0066_b0d0;
/// `NiPointer<T>::NiPointer(T*)` (`00633c90`): stores the object and adds a
/// reference.
const NI_POINTER_CONSTRUCT: u32 = 0x0063_3c90;
/// `NiPointer<T>::~NiPointer` (`0045cec0`): releases the object.
const NI_POINTER_DESTRUCT: u32 = 0x0045_cec0;
/// `MemoryManager` allocation, `__cdecl(size) -> block` (`00401000`).
const MEMORY_ALLOC: u32 = 0x0040_1000;
/// `MemoryManager` deallocation, `__cdecl(block)` (`00401030`).
const MEMORY_FREE: u32 = 0x0040_1030;
/// `strlen` through the game's wrapper (`0044a670`).
const STRLEN: u32 = 0x0044_a670;
/// `strcpy_s(destination, size, source)` through the game's wrapper (`00406d30`).
const STRING_COPY: u32 = 0x0040_6d30;
/// `memcpy(destination, source, count)` through the game's wrapper (`00401460`).
const MEMORY_COPY: u32 = 0x0040_1460;
/// Address of the name slot of an object (`this + 8`; `00413f40`).
const NAME_SLOT: u32 = 0x0041_3f40;
/// The array-element address getter of `BSStream` (`00877a30`):
/// `base + index * 4` of the array at `this`.
const ARRAY_ELEMENT_ADDRESS: u32 = 0x0087_7a30;
/// The element-count getter of that array (`0084e3a0`): the dword at +0xC.
const ARRAY_COUNT: u32 = 0x0084_e3a0;
/// `NiNode::NiNode(capacity)` (`00a5ecb0`).
const NI_NODE_CONSTRUCT: u32 = 0x00a5_ecb0;
/// Allocator of `NiMemObject`-derived objects, `__cdecl(size) -> block`
/// (`00aa13e0`).
const NI_ALLOC: u32 = 0x00aa_13e0;
/// `NiAVObject::GetProperty(type)` (Xbox PDB, `00a59d30`).
const GET_PROPERTY: u32 = 0x00a5_9d30;
/// `NiAVObject::RemoveProperty_ov2(type)` (Xbox PDB, `00a5b230`).
const REMOVE_PROPERTY: u32 = 0x00a5_b230;
/// `RemoveEditorMarkers(node)` (Xbox PDB, `004b5d10`).
const REMOVE_EDITOR_MARKERS: u32 = 0x004b_5d10;
/// `HasMorpherController(node)` (Xbox PDB, `004b5bf0`).
const HAS_MORPHER_CONTROLLER: u32 = 0x004b_5bf0;
/// `BSShaderManager::PrepareObject(node, flag, flag)` (Xbox PDB, `00b57e30`).
const PREPARE_OBJECT: u32 = 0x00b5_7e30;
/// `BSStream::FreeAllObjects` (Xbox PDB, `00c3b370`).
const STREAM_FREE_ALL_OBJECTS: u32 = 0x00c3_b370;
/// The game's `printf`-style log (`005b5e40`), `__cdecl(format, ...)`.
const LOG: u32 = 0x005b_5e40;
/// Run-time type check, `__cdecl(type descriptor, object) -> object or 0`
/// (`00653270`).
const DYNAMIC_CAST: u32 = 0x0065_3270;
/// Reads the pointer an object `InitModel` cast holds (`00537bd0`, a
/// `NiPointer` read in the land unit).
const CAST_RESULT_POINTER: u32 = 0x0053_7bd0;
/// Run-time type test, `__thiscall(object, type descriptor) -> bool`
/// (`006532c0`).
const IS_KIND_OF: u32 = 0x0065_32c0;
/// `TESAnimGroup::LoadAnimGroup(sequence, name)` (Xbox PDB, `005f3a20`).
const LOAD_ANIM_GROUP: u32 = 0x005f_3a20;
/// Builds the `NiControllerSequence` of a `KFModel`, `__cdecl(file, 0,
/// NiPointer out)` (`004eeb60`).
const LOAD_KF_SEQUENCE: u32 = 0x004e_eb60;
/// `KFModel` accessor returning its `spSequence` (`007fa950`).
const KF_MODEL_SEQUENCE: u32 = 0x007f_a950;
/// `NiFixedString::NiFixedString(const char*)` (`00438170`): stores the
/// pooled handle, returns the string.
const FIXED_STRING_CONSTRUCT: u32 = 0x0043_8170;
/// `NiFixedString::~NiFixedString` (`004381b0`).
const FIXED_STRING_DESTRUCT: u32 = 0x0043_81b0;
/// Releases a `NiFixedString` handle (`004381d0`), `__cdecl(handle pointer)`.
const FIXED_STRING_RELEASE: u32 = 0x0043_81d0;
/// Address of the reference count of a pooled string, `__cdecl(handle
/// pointer)` (`00438210`).
const FIXED_STRING_REFCOUNT: u32 = 0x0043_8210;
/// `InterlockedIncrement` wrapper, `__cdecl(address)` (`0040b460`).
const INTERLOCKED_INCREMENT: u32 = 0x0040_b460;
/// `InterlockedDecrement` wrapper, `__cdecl(address)` (`004019a0`).
const INTERLOCKED_DECREMENT: u32 = 0x0040_19a0;
/// `Sleep` wrapper, `__cdecl(milliseconds)` (`0040fca0`).
const SLEEP: u32 = 0x0040_fca0;
/// `KERNEL32 InterlockedCompareExchange` (import slot).
const INTERLOCKED_COMPARE_EXCHANGE: u32 = 0x00fd_f1bc;
/// `strchr` (`00ec7690`).
const STRCHR: u32 = 0x00ec_7690;
/// `Bip[0-9][0-9]`-style pattern match, `__cdecl(text, pattern) -> bool`
/// (`00af3bd0`).
const MATCHES_PATTERN: u32 = 0x00af_3bd0;
/// `__thiscall(object, 1)` (`004534f0`): the call `0043b5b0` makes on the
/// object embedded in the block at `this`.
const DELETE_OBJECT: u32 = 0x0045_34f0;
/// Returns the field at +8 of the object it is called on (`00620b80`, the
/// same as `0044ddc0`).
const REF_COUNT_OBJECT_POINTER: u32 = 0x0062_0b80;
/// Returns `this + 0x10` (`00460140`).
const HOLDER_VALUE: u32 = 0x0046_0140;
/// An empty function (`00483710`).
const HOLDER_DESTRUCT: u32 = 0x0048_3710;
/// Reads the field at +8 of an object (`0044ddc0`).
const FIELD_AT_8: u32 = 0x0044_ddc0;
/// Stores a pointer into the object it is called on (`008c71b0`).
const STORE_POINTER: u32 = 0x008c_71b0;
/// `NiAVObject::m_spCollisionObject` getter (`006838b0`).
const COLLISION_OBJECT: u32 = 0x0068_38b0;
/// The array-size getter of `0043b480`'s array (`00658930`): the word at +0xA.
const ARRAY_SIZE: u32 = 0x0065_8930;

/// `"Meshes\\Marker_Error.NIF"`.
const MARKER_ERROR_NIF: u32 = 0x0101_667c;
/// `"Bip[0-9][0-9]"`.
const BIP_PATTERN: u32 = 0x0101_666c;
/// `"MODELS: %s: Reexport '%s' to get rid of the ZBuffer and/or VertextColor
/// property."`.
const REEXPORT_WITH_FILE_MESSAGE: u32 = 0x0101_6618;
/// `"MODELS: Reexport '%s' to get rid of the ZBuffer and/or VertextColor
/// property."`.
const REEXPORT_MESSAGE: u32 = 0x0101_65c8;
/// `"ANIMATION: Could not create ControllerManager Sequence for \"%s\".\r\n"`.
const NO_SEQUENCE_MESSAGE: u32 = 0x0101_6698;

/// `MessageHandler`'s disable-warning counter (a signed dword, never below 0).
const DISABLE_WARNING_COUNT: u32 = 0x0120_2d6c;
/// A byte flag `InitGunWobble` writes and `InitModel` reads (its meaning is
/// not known).
const GUN_WOBBLE_FLAG: u32 = 0x0118_5520;
/// The pooled empty string handle `NiFixedString` leaves unreferenced.
const EMPTY_FIXED_STRING: u32 = 0x0109_b220;
/// The word `0043b3b0` returns when it finds nothing (`0xFFFF`).
const NOT_FOUND_INDEX: u32 = 0x0109_6320;
/// `NiPointer` globals holding the shared properties `InitModel` compares
/// against (the property types are `0043b290` and `00702440`).
const SHARED_PROPERTY_FIRST: u32 = 0x011f_4444;
const SHARED_PROPERTY_SECOND: u32 = 0x011f_4438;
/// Type descriptor `InitModel` casts the root's child to (the class is not
/// identified).
const TYPE_DESCRIPTOR_011F36AC: u32 = 0x011f_36ac;
/// Type descriptor `0043b7e0` tests the loaded sequence against (the class is
/// not identified).
const TYPE_DESCRIPTOR_011C7D74: u32 = 0x011c_7d74;
/// Type descriptor `0043b610` casts to (`bhkCollisionObject`).
const TYPE_DESCRIPTOR_BHK_COLLISION_OBJECT: u32 = 0x0120_43f8;
/// The property type `0043b290` returns.
const PROPERTY_TYPE_FIRST: u32 = 0xb;
/// Returns the second property type `InitModel` strips (`00702440`, in the
/// interface unit).
const PROPERTY_TYPE_SECOND_GETTER: u32 = 0x0070_2440;

// --- Queued files (`0043bac0` onwards): `LoadedFile`, `QueuedTexture`, `QueuedModel`. ---

/// `QueuedFileEntry::QueuedFileEntry(context)` (`00c3ce60`): the shared base
/// constructor of the queued texture and model.
const QUEUED_FILE_ENTRY_CONSTRUCT: u32 = 0x00c3_ce60;
/// `QueuedFileEntry` destructor body (`00c3cea0`).
const QUEUED_FILE_ENTRY_DESTRUCT: u32 = 0x00c3_cea0;
/// Copies a file name into `pFileName` (`00c3cee0`, `__thiscall(name)`).
const QUEUED_FILE_ENTRY_SET_FILE_NAME: u32 = 0x00c3_cee0;
/// Looks the file entry of `pFileName` up (`00c3cf60`, `__thiscall(flag)`).
const QUEUED_FILE_ENTRY_FIND_FILE_ENTRY: u32 = 0x00c3_cf60;
/// Stores `pFileEntry` (`00c3cf40`, `__thiscall(file entry)`).
const QUEUED_FILE_ENTRY_SET_FILE_ENTRY: u32 = 0x00c3_cf40;
/// `QueuedFileEntry::GetFile` (Xbox PDB, `00c3cff0`), `__thiscall(1, 2)`:
/// opens the file of the entry and returns the `BSFile` (0 on failure).
const QUEUED_FILE_ENTRY_GET_FILE: u32 = 0x00c3_cff0;
/// `QueuedFileEntry::GetDescription` (Xbox PDB, `00c3d0e0`),
/// `__thiscall(buffer, size, kind text) -> bool`.
const QUEUED_FILE_ENTRY_GET_DESCRIPTION: u32 = 0x00c3_d0e0;
/// `QueuedFile::Cancel` (Xbox PDB, `00c3cb70`), `__thiscall(2 arguments)`.
const QUEUED_FILE_CANCEL: u32 = 0x00c3_cb70;
/// Getter of `QueuedFileEntry::pFileEntry` (`0055b980`, `this + 0x2c`; the
/// linker folded it with the same getter of the object unit).
const QUEUED_FILE_ENTRY_GET_FILE_ENTRY: u32 = 0x0055_b980;
/// Getter of `QueuedFileEntry::pFileName` (`0045cd60`, `this + 0x28`).
const QUEUED_FILE_ENTRY_GET_FILE_NAME: u32 = 0x0045_cd60;
/// `BSFileEntry::iSize & 0x3fffffff` (`0062a100`).
const FILE_ENTRY_GET_SIZE: u32 = 0x0062_a100;
/// The archive that holds a file entry, `__cdecl(file entry, 2) -> archive`
/// (`00af6910`, in the archive unit; 0 when there is none).
const FILE_ENTRY_FIND_ARCHIVE: u32 = 0x00af_6910;
/// `__thiscall(flag, byte)` helpers of this unit, `__cdecl(flag, byte
/// pointer)`: set (non-zero `flag`) or clear one bit of the byte.
const SET_FLAG_BIT_1: u32 = 0x0044_ae20;
const SET_FLAG_BIT_2: u32 = 0x0044_ae60;
const SET_FLAG_BIT_4: u32 = 0x0044_aea0;
/// `__cdecl(flags byte) -> bool`: whether bit 2 / bit 4 of the byte is set.
const TEST_FLAG_BIT_2: u32 = 0x0044_ae50;
const TEST_FLAG_BIT_4: u32 = 0x0044_ae90;
/// `__thiscall(flag)` on a `QueuedTexture` (`0043e480`, in this unit, not
/// translated yet): sets or clears bit 1 of `cFlags`.
const QUEUED_TEXTURE_SET_FLAG_1: u32 = 0x0043_e480;
/// `__thiscall()` on a `QueuedTexture` (`0043e530`, in this unit, not
/// translated yet): whether bit 1 of `cFlags` is set.
const QUEUED_TEXTURE_TEST_FLAG_1: u32 = 0x0043_e530;
/// The object at `011c3b3c`, whose methods `00448330`, `00448370`,
/// `00448ed0` and `00448f50` (this unit, not translated yet) keep the maps
/// of loaded files and of queued textures.
const FILE_MAP_OWNER: u32 = 0x011c_3b3c;
/// `__thiscall(name, loaded file) -> bool` on [`FILE_MAP_OWNER`]: adds a
/// `LoadedFile` to the map of loaded files.
const FILE_MAP_ADD_LOADED_FILE: u32 = 0x0044_8330;
/// `__thiscall(name)` on [`FILE_MAP_OWNER`]: removes it again.
const FILE_MAP_REMOVE_LOADED_FILE: u32 = 0x0044_8370;
/// `__thiscall(file entry, queued texture) -> bool` on [`FILE_MAP_OWNER`]:
/// adds the texture to the map of queued textures by file entry.
const FILE_MAP_ADD_QUEUED_TEXTURE: u32 = 0x0044_8ed0;
/// `__thiscall(file entry)` on [`FILE_MAP_OWNER`]: removes it.
const FILE_MAP_REMOVE_QUEUED_TEXTURE: u32 = 0x0044_8f50;
/// The object at `01202d98` (a task queue); slot `0x48` takes the queued
/// texture to run.
const TASK_QUEUE: u32 = 0x0120_2d98;
/// The object at `011f6388` that `LoadedFile`'s destructor reports to
/// (slot `0x14`: `(severity, text, 0, 0, 0)`), 0 when there is none.
const MESSAGE_SINK: u32 = 0x011f_6388;
/// `BSTexturePalette::pTexMapA` (Xbox PDB), `011f4468`: the texture map.
const TEXTURE_MAP: u32 = 0x011f_4468;
/// The word `0043c4b0` returns (`011f4748`, a default texture setting).
const DEFAULT_TEXTURE_SETTING: u32 = 0x011f_4748;
/// `BSTexturePalette::SetTexture` (Xbox PDB, `00a61c50`), `__cdecl(texture,
/// file entry)`.
const TEXTURE_PALETTE_SET_TEXTURE: u32 = 0x00a6_1c50;
/// `BSTexturePalette::GetTexture_ov2` (Xbox PDB, `00a61b90`), `__cdecl(file
/// name, NiPointer out)`.
const TEXTURE_PALETTE_GET_TEXTURE_BY_NAME: u32 = 0x00a6_1b90;
/// Hash-bucket lookup of the texture map (`00a61a60`), `__thiscall(bucket,
/// file entry, NiPointer out) -> bool`.
const TEXTURE_MAP_FIND: u32 = 0x00a6_1a60;
/// Creates a texture from an open file, `__cdecl(NiFixedString*, file,
/// setting, format preferences)` (`00a61040`, in the cube map unit).
const CREATE_TEXTURE_FROM_FIXED_NAME: u32 = 0x00a6_1040;
/// The engine map's `NiSourceCubeMap::Create` (`00a5fe30`), `__cdecl(file,
/// name, format preferences, 1)`; `Run` takes it for names without `_e.dd`,
/// so the map's name is doubtful.
const CREATE_TEXTURE_FROM_FILE: u32 = 0x00a5_fe30;
/// `strstr` (`00ec7750`), `__cdecl(text, pattern)`.
const STRSTR: u32 = 0x00ec_7750;
/// `sprintf` (`00ec623a`), `__cdecl(buffer, format, ...)`.
const SPRINTF: u32 = 0x00ec_623a;
/// Path-normalizing copy, `__cdecl(name, buffer, size)` (`00af4200`).
const NORMALIZE_PATH: u32 = 0x00af_4200;
/// The texture's surface getter (`0059bb30`, `this + 0x24`; the engine map
/// calls it `D3DTexture_LockRect`).
const TEXTURE_GET_SURFACE: u32 = 0x0059_bb30;
/// The scope guard that sets the memory context for the rest of a function:
/// `__thiscall(context, 1, source file, line)` and its destructor.
const MEMORY_CONTEXT_ENTER: u32 = 0x0040_4eb0;
const MEMORY_CONTEXT_LEAVE: u32 = 0x0040_4ee0;
/// Releases the `NiPointer<Model>` at `this` (`0040c110`).
const MODEL_POINTER_RELEASE: u32 = 0x0040_c110;

/// `QueuedTexture`'s virtual table (`01016788`) and `QueuedModel`'s
/// (`01016890`).
const QUEUED_TEXTURE_VTABLE: u32 = 0x0101_6788;
const QUEUED_MODEL_VTABLE: u32 = 0x0101_6890;

/// `"texture"`.
const TEXTURE_WORD: u32 = 0x0101_6884;
/// `"_e.dd"`.
const ENVIRONMENT_MAP_SUFFIX: u32 = 0x0101_6838;
/// `"D:\_Fallout3\Platforms\Common\Code\Fallout Shared\ModelLoader.cpp"`.
const MODEL_LOADER_SOURCE: u32 = 0x0101_6840;
/// The source line the memory context scope of `Run` records.
const RUN_SOURCE_LINE: u32 = 0x2b7;
/// `"MODELS: Could not get file for texture %s."`.
const NO_FILE_FOR_TEXTURE_NAME_MESSAGE: u32 = 0x0101_67b8;
/// `"MODELS: Could not get file for texture with file entry offset %i and
/// size %i."`.
const NO_FILE_FOR_TEXTURE_ENTRY_MESSAGE: u32 = 0x0101_67e8;
/// `"LoadedFile %s was loaded at %i and added to the map, but is being thrown
/// away without being used"`.
const LOADED_FILE_UNUSED_MESSAGE: u32 = 0x0101_6720;
/// `"%s ref object at mem %i being destroyed with %i references\r\n"`.
const LOADED_FILE_REFERENCES_MESSAGE: u32 = 0x0101_66dc;
/// The format preferences object `Run` passes to the texture creators.
const TEXTURE_FORMAT_PREFERENCES: u32 = 0x011a_9598;

// --- `QueuedModel` and the tree classes (`0043c890` onwards). ---

/// `NiPointer<Model>::operator=(Model*)` (Xbox PDB `NiPointer<Model>::operator_`,
/// `0044aed0`, in this unit, not translated yet), `__thiscall(model)` on the
/// `NiPointer`.
const MODEL_POINTER_ASSIGN: u32 = 0x0044_aed0;
/// `QueuedFile::CheckFinished` (Xbox PDB, `00c3c930`).
const QUEUED_FILE_CHECK_FINISHED: u32 = 0x00c3_c930;
/// `QueuedFileEntry::GenerateKey` (Xbox PDB, `00c3d440`).
const QUEUED_FILE_ENTRY_GENERATE_KEY: u32 = 0x00c3_d440;
/// `BSStream::Load` (Xbox PDB, `00c3a8a0`), `__thiscall(name, file)`: reads
/// the stream from the open `BSFile`; false on failure.
const BS_STREAM_LOAD: u32 = 0x00c3_a8a0;
/// `NiStream::NiStream` (`00a66150`) and its destructor body (`00a65300`).
const NI_STREAM_CONSTRUCT: u32 = 0x00a6_6150;
const NI_STREAM_DESTRUCT: u32 = 0x00a6_5300;
/// `NiStreamSaveBinary<unsigned int>` (Xbox PDB, `0044adf0`),
/// `__cdecl(stream, value pointer)`.
const NI_STREAM_SAVE_BINARY_U32: u32 = 0x0044_adf0;
/// `ModelLoader::FindModel` (Xbox PDB, `004472a0`) on [`FILE_MAP_OWNER`],
/// `__thiscall(name, NiPointer<Model> out)`: gives the pointer the model of
/// that name when the loader has it.
const MODEL_LOADER_FIND_MODEL: u32 = 0x0044_72a0;
/// Adds a model to the loader's map (`00447010`, in this unit, not
/// translated yet), `__thiscall(name, model) -> bool` on [`FILE_MAP_OWNER`];
/// false when the name is already there.
const MODEL_LOADER_ADD_MODEL: u32 = 0x0044_7010;
/// `ModelLoader::QueueTexture_ov2` (Xbox PDB, `00443af0`) on
/// [`FILE_MAP_OWNER`], `__thiscall(texture, key, parent queued file)`.
const MODEL_LOADER_QUEUE_TEXTURE: u32 = 0x0044_3af0;
/// `BGSTextureSet::QueueTextureSet` (Xbox PDB, `005930e0`),
/// `__thiscall(key, parent queued file, 0)`.
const QUEUE_TEXTURE_SET: u32 = 0x0059_30e0;
/// `__thiscall()` on a `TESModel` (`0048d150`): `this + 0xC`, the texture
/// hash block `QueueMe_ov2` reads.
const TES_MODEL_TEXTURE_BLOCK: u32 = 0x0048_d150;
/// `__thiscall()` getters of the texture hash block (`009373f0`: the byte at
/// +0; `00726070`: the dword at +4, also the "next" link of the list nodes).
const BYTE_AT_0: u32 = 0x0093_73f0;
const FIELD_AT_4: u32 = 0x0072_6070;
/// List helpers of the texture set list: `00500940` returns `this + 0x18`
/// (the first node), `006815c0` returns the node itself (its item is the
/// first dword); the next node is [`FIELD_AT_4`].
const LIST_FIRST_NODE: u32 = 0x0050_0940;
const LIST_NODE_ITEM_ADDRESS: u32 = 0x0068_15c0;
/// Whether the dword at +0xC (`eState`) is 0 (`0069b080`; the engine map
/// files it under the navmesh code).
const STATE_IS_ZERO: u32 = 0x0069_b080;
/// The finished-child count of a `QueuedChildren` (`0044edb0`, named
/// `BaseProcess::GetCurrentProcedureIndex` by the engine map but folded: it
/// returns the dword at +0x10).
const FINISHED_COUNT_GETTER: u32 = 0x0044_edb0;
/// `__cdecl(flags byte) -> bool` tests of the other bits of a queued file's
/// flag byte, and the setter of bit 8 (`__cdecl(flag, byte pointer)`).
const TEST_FLAG_BIT_1: u32 = 0x0044_ae10;
const TEST_FLAG_BIT_8: u32 = 0x0044_af20;
const TEST_FLAG_BIT_10: u32 = 0x0044_af60;
const TEST_FLAG_BIT_20: u32 = 0x0044_afa0;
const SET_FLAG_BIT_8: u32 = 0x0044_af30;
/// `NiRefObject::IncRefCount` (`0092c870`): one more reference.
const ADD_REFERENCE: u32 = 0x0092_c870;
/// `NiAVObject::Update(NiUpdateData*)` as `Finish` calls it (`00a59c60`,
/// `__thiscall(update data)`).
const NI_AV_OBJECT_UPDATE: u32 = 0x00a5_9c60;
/// Radius of a `NiBound` (`0084d030`, the float at +0xC, returned in ST0).
const BOUND_RADIUS: u32 = 0x0084_d030;
/// `TESObjectREFR::GetBoundMax` (Xbox PDB, `0056fc30`), `__cdecl(out, node)
/// -> vector pointer`.
const GET_BOUND_MAX: u32 = 0x0056_fc30;
/// Address of the value of a setting object (`00403e20`: `this + 4`, or a
/// zero scratch float when `this` is 0).
const SETTING_VALUE_POINTER: u32 = 0x0040_3e20;
/// `QueuedModel::mfOverriddenVisualDistance` getter (`00792760`, in ST0).
const OVERRIDDEN_VISUAL_DISTANCE: u32 = 0x0079_2760;
/// `BSFadeNode::SetRange` (Xbox PDB, `00b4dfd0`), `__thiscall(float, float)`.
const FADE_NODE_SET_RANGE: u32 = 0x00b4_dfd0;
/// `BSFadeNode::SetLODMultType` (Xbox PDB, `00b4dec0`).
const FADE_NODE_SET_LOD_MULT_TYPE: u32 = 0x00b4_dec0;
/// The setting objects `Finish` reads: the world size in cells (`011c63cc`,
/// +4 is the value), the scale of the visual distance of a model
/// (`011c3be4`) and the scale of the fade range (`011c3bb8`).
const CELL_COUNT_SETTING: u32 = 0x011c_63cc;
const VISUAL_DISTANCE_SCALE_SETTING: u32 = 0x011c_3be4;
const FADE_RANGE_SCALE_SETTING: u32 = 0x011c_3bb8;
/// The zero scratch dword `0043d4d0` returns the address of when it has no
/// object.
const SCRATCH_ZERO: u32 = 0x0120_2800;
/// The bound `NiAVObject::GetWorldBound` returns when the object has none.
const EMPTY_WORLD_BOUND: u32 = 0x011f_4288;
/// Doubles in `.rdata`: 0.001, 2048.0, 0.5 and 0.0.
const SMALL_RADIUS: u32 = 0x0101_6978;
const LOADED_AREA_MARGIN: u32 = 0x0101_6968;
const HALF: u32 = 0x0101_1588;
const ZERO: u32 = 0x0101_2060;
/// The file name of the model being loaded (`011f94c0`): `QueuedModel::Run`
/// sets it while the stream loads and clears it afterwards.
const LOADING_FILE_NAME: u32 = 0x011f_94c0;
/// `TESObjectTREE::BuildDistant3D` (Xbox PDB, `0051cba0`), `__thiscall(1)`.
const BUILD_DISTANT_3D: u32 = 0x0051_cba0;
/// Zeroes the three words of the object (`00b57e70`).
const ZERO_THREE_WORDS: u32 = 0x00b5_7e70;
/// Builds the instanced billboards (`00b59ae0`), `__cdecl(cell chunk, cell
/// key, instanced node, out, locations, colors, count)`.
const CREATE_BILLBOARD_INSTANCES: u32 = 0x00b5_9ae0;
/// The destructor body of `TREE_BILLBOARD_DATA` (`0051d400`).
const TREE_BILLBOARD_DATA_DESTRUCT: u32 = 0x0051_d400;
/// The smart pointer to a task: `__thiscall(task)` constructor that adds a
/// reference (`00528cb0`), and its destructor (`0044cbf0`).
const TASK_POINTER_CONSTRUCT: u32 = 0x0052_8cb0;
const TASK_POINTER_DESTRUCT: u32 = 0x0044_cbf0;
/// Puts a task into the queue of its priority (`00449210`),
/// `__thiscall(priority, task pointer)` on the table of queues at
/// `IOManager + 0x64`.
const QUEUE_TABLE_ADD: u32 = 0x0044_9210;

/// `BSStream`'s virtual table (`01016904`), `QueuedTreeBillboard`'s
/// (`0101698c`) and `QueuedTreeModel`'s (`010169d4`).
const BS_STREAM_VTABLE: u32 = 0x0101_6904;
const QUEUED_TREE_BILLBOARD_VTABLE: u32 = 0x0101_698c;
const QUEUED_TREE_MODEL_VTABLE: u32 = 0x0101_69d4;

/// `"MODEL ERROR: Could not %s file '%s'."`, and its two verbs.
const MODEL_ERROR_MESSAGE: u32 = 0x0101_68c4;
const FIND_WORD: u32 = 0x0101_68ec;
const LOAD_WORD: u32 = 0x0101_68f4;
/// `"model"` and `"Tree Billboard"`.
const MODEL_WORD: u32 = 0x0101_6980;
const TREE_BILLBOARD_WORD: u32 = 0x0101_69c0;
/// The source lines the memory context scopes of `QueuedModel::Run` and
/// `Finish` record.
const MODEL_RUN_SOURCE_LINE: u32 = 0x3b8;
const MODEL_FINISH_SOURCE_LINE: u32 = 0x3e5;
/// The `eContext` a queued tree model starts with (`0x1e`).
const TREE_MODEL_CONTEXT: u32 = 0x1e;

/// The offsets in a thread's TLS block `QueuedTexture` uses: a byte flag
/// (read by `0043c130`) and a word (`0043c410`/`0043c3f0`) that `Run` sets to
/// 1 while it loads a texture whose flag bit 2 is set.
const TLS_QUEUED_FLAG: u32 = 0x25c;
const TLS_QUEUED_VALUE: u32 = 0x29c;
/// Puts a queued task into state 5, done (`00449150`: `this + 0xC = 5`).
const TASK_SET_DONE: u32 = 0x0044_9150;

layout! {
    /// `Model` (Xbox PDB), 0x18 bytes on the Xbox, 0x10 on PC (the two
    /// leading memory-accounting words `iVBMem` and `iDefaultMem` are gone,
    /// so every field sits 8 bytes earlier).
    pub struct Model: 0x10 {
        /// `pFilename` (Xbox PDB): `char*`, a copy owned by the model.
        0x00 pFilename: u32,
        /// `iRefCount` (Xbox PDB).
        0x04 iRefCount: i32,
        /// `iManualRefCount` (Xbox PDB): changed with interlocked operations.
        0x08 iManualRefCount: i32,
        /// `spObject3D` (Xbox PDB): `NiPointer<NiNode>`.
        0x0C spObject3D: Ptr,
    }

    /// `KFModel` (Xbox PDB), 0x1C bytes on the Xbox, 0x14 on PC (without
    /// `iVBMem` and `iDefaultMem`).
    pub struct KFModel: 0x14 {
        /// `pFilename` (Xbox PDB): `char*`, a copy owned by the model.
        0x00 pFilename: u32,
        /// `spSequence` (Xbox PDB): `NiPointer<NiControllerSequence>`.
        0x04 spSequence: Ptr,
        /// `spAnimGroup` (Xbox PDB): `NiPointer<TESAnimGroup>`.
        0x08 spAnimGroup: Ptr,
        /// `iRefCount` (Xbox PDB).
        0x0C iRefCount: i32,
        /// `iManualRefCount` (Xbox PDB).
        0x10 iManualRefCount: i32,
    }
}

layout! {
    /// `LoadedFile` (Xbox PDB), 0x10 bytes (the same on PC): a file kept
    /// open by the loader, found by name in the map of loaded files.
    pub struct LoadedFile: 0x10 {
        /// `iRefCount` (Xbox PDB).
        0x00 iRefCount: i32,
        /// `pFileName` (Xbox PDB): `char*`, a copy owned by the object.
        0x04 pFileName: u32,
        /// `pFile` (Xbox PDB): `BSFile*`, deleted with the object.
        0x08 pFile: Ptr,
        /// `bInLoadedFileMap` (Xbox PDB).
        0x0C bInLoadedFileMap: bool,
        /// `bFileUsed` (Xbox PDB).
        0x0D bFileUsed: bool,
    }

    /// `QueuedChildren` (Xbox PDB), 0x14 bytes: the Xbox class derives from
    /// `BSSimpleArray<NiPointer<QueuedFile>>`; only the counter at the end
    /// is used here.
    pub struct QueuedChildren: 0x14 {
        /// `iNumChildrenFinished` (Xbox PDB).
        0x10 iNumChildrenFinished: u32,
    }

    /// `QueuedFile` (Xbox PDB), 0x28 bytes on the Xbox (virtual table, the
    /// `IOTask` base, then these); the PC offsets used here are the same.
    pub struct QueuedFile: 0x28 {
        /// `eContext` (Xbox PDB): the memory context the task runs in.
        0x18 eContext: u32,
        /// `pChildren` (Xbox PDB): `QueuedChildren*`.
        0x20 pChildren: Ptr,
    }

    /// `QueuedTexture` (Xbox PDB), 0x38 bytes on the Xbox (a `QueuedFile`,
    /// then `QueuedFileEntry`'s `pFileName` and `pFileEntry`, then these);
    /// the offsets used here are the same on PC.
    pub struct QueuedTexture: 0x38 {
        /// `eContext` (`QueuedFile`, Xbox PDB).
        0x18 eContext: u32,
        /// `pChildren` (`QueuedFile`, Xbox PDB).
        0x20 pChildren: Ptr,
        /// `pFileName` (`QueuedFileEntry`, Xbox PDB).
        0x28 pFileName: u32,
        /// `pFileEntry` (`QueuedFileEntry`, Xbox PDB): `BSFileEntry*`.
        0x2C pFileEntry: Ptr,
        /// `spTexture` (Xbox PDB): `NiPointer<NiTexture>`.
        0x30 spTexture: Ptr,
        /// `cFlags` (Xbox PDB). Bit 4: the texture is in the map of queued
        /// textures (set from the result of the map's add, cleared by
        /// `Finish` and `Cancel` when they remove it). Bit 2: set when the
        /// thread flag `TLS_QUEUED_FLAG` was set at queueing time; `Run`
        /// then sets the TLS word while it loads. Bit 1 is set by
        /// `0043e480` and tested by `0043e530` (not translated yet).
        0x34 cFlags: u8,
    }

    /// `QueuedModel` (Xbox PDB), 0x48 bytes on the Xbox; the offsets used
    /// here are the same on PC.
    pub struct QueuedModel: 0x48 {
        /// `eContext` (`QueuedFile`, Xbox PDB).
        0x18 eContext: u32,
        /// `pChildren` (`QueuedFile`, Xbox PDB): `QueuedChildren*`.
        0x20 pChildren: Ptr,
        /// `spModel` (Xbox PDB): `NiPointer<Model>`.
        0x30 spModel: Ptr,
        /// `pTESModel` (Xbox PDB): `TESModel*`.
        0x34 pTESModel: Ptr,
        /// `eLODFadeMult` (Xbox PDB).
        0x38 eLODFadeMult: u32,
        /// `cFlags` (Xbox PDB): bit 1 and bit 2 are set by the constructor's
        /// last two arguments.
        0x3C cFlags: u8,
        /// `mfOverriddenVisualDistance` (Xbox PDB).
        0x40 mfOverriddenVisualDistance: f32,
    }
}

layout! {
    /// `BSStream` (Xbox PDB), 0x5cc bytes (the `NiStream` base, then these):
    /// `QueuedModel::Run` keeps one on its stack.
    pub struct BSStream: 0x5cc {
        /// `pObjectRefMap` (Xbox PDB): `NiTStringMap<NiObjectNET*>*`, deleted
        /// (virtual destructor, flag 1) by `0043d100`.
        0x5C4 pObjectRefMap: Ptr,
        /// `spNodeReferences` (Xbox PDB): `NiPointer<BSNodeReferences>`.
        0x5C8 spNodeReferences: Ptr,
    }

    /// `IOTask` (Xbox PDB), 0x18 bytes: the base of the queued files.
    pub struct IOTask: 0x18 {
        /// `Key` (Xbox PDB): `i64`; bits 16 to 23 are the priority of the
        /// queue the task goes into.
        0x10 Key: u64,
    }

    /// `NiUpdateData` (Xbox PDB), 0xc bytes.
    pub struct NiUpdateData: 0xc {
        /// `fTime` (Xbox PDB).
        0x00 fTime: f32,
        /// `bUpdateControllers` (Xbox PDB).
        0x04 bUpdateControllers: u8,
        /// `bParallelUpdate` (Xbox PDB).
        0x05 bParallelUpdate: u8,
        /// `bFoundParticles` (Xbox PDB).
        0x06 bFoundParticles: u8,
        /// `bFoundMorphController` (Xbox PDB).
        0x07 bFoundMorphController: u8,
        /// `bSceneGraphChange` (Xbox PDB).
        0x08 bSceneGraphChange: u8,
    }

    /// `TREE_BILLBOARD_DATA` (Xbox PDB), 0x1c bytes (the same on PC).
    pub struct TreeBillboardData: 0x1c {
        /// `pTree` (Xbox PDB): `TESObjectTREE*`.
        0x00 pTree: Ptr,
        /// `iCellChunk` (Xbox PDB).
        0x04 iCellChunk: u32,
        /// `iCellKey` (Xbox PDB).
        0x08 iCellKey: u32,
        /// `pInstancedNode` (Xbox PDB): `NiNode*`.
        0x0C pInstancedNode: Ptr,
        /// `iArraySize` (Xbox PDB); the game reads the low word.
        0x10 iArraySize: u32,
        /// `pLocArray` (Xbox PDB): `NiPoint3*`.
        0x14 pLocArray: Ptr,
        /// `pColorArray` (Xbox PDB): `f32*`.
        0x18 pColorArray: Ptr,
    }

    /// `QueuedTreeBillboard` (Xbox PDB), 0x40 bytes: a `QueuedTexture` and
    /// the data to build the billboard from.
    pub struct QueuedTreeBillboard: 0x40 {
        /// `eContext` (`QueuedFile`, Xbox PDB).
        0x18 eContext: u32,
        /// `pBillboardData` (Xbox PDB): `TREE_BILLBOARD_DATA*`, owned.
        0x38 pBillboardData: Ptr,
    }

    /// `QueuedTreeModel` (Xbox PDB), 0x50 bytes: a `QueuedModel` and the
    /// reference and tree it is for.
    pub struct QueuedTreeModel: 0x50 {
        /// `eContext` (`QueuedFile`, Xbox PDB).
        0x18 eContext: u32,
        /// `pRef` (Xbox PDB): `TESObjectREFR*`.
        0x48 pRef: Ptr,
        /// `pTree` (Xbox PDB): `TESObjectTREE*`.
        0x4C pTree: Ptr,
    }
}

/// The address of `Model::spObject3D`, which the game passes to the
/// `NiPointer` functions.
fn object_3d_pointer(model: Ptr<Model>) -> Ptr {
    model.byte_add(Model::spObject3D.off)
}

/// The address of `KFModel::spSequence`.
fn sequence_pointer(model: Ptr<KFModel>) -> Ptr {
    model.byte_add(KFModel::spSequence.off)
}

/// The address of `KFModel::spAnimGroup`.
fn anim_group_pointer(model: Ptr<KFModel>) -> Ptr {
    model.byte_add(KFModel::spAnimGroup.off)
}

/// `NiPointer::operator T*` on the `NiPointer` at `pointer`.
fn ni_pointer_get(e: &mut Engine, pointer: Ptr) -> Ptr {
    e.call(NI_POINTER_GET, &args![pointer]).ptr()
}

/// Copies the NUL-terminated `name` into a fresh block of the memory manager
/// (what the constructors do for `pFilename`).
fn copy_name(e: &mut Engine, name: u32) -> u32 {
    let size = e.call(STRLEN, &args![name]).u32().wrapping_add(1);
    let copy = e.call(MEMORY_ALLOC, &args![size]).u32();
    e.call(STRING_COPY, &args![copy, size, name]);
    copy
}

/// A fresh `NiNode` for the model to hold (`operator new` then constructor,
/// 0 when the allocation failed).
fn new_ni_node(e: &mut Engine) -> u32 {
    let block = e.call(NI_ALLOC, &args![0xacu32]).u32();
    if block == 0 {
        0
    } else {
        e.call(NI_NODE_CONSTRUCT, &args![block, 0u32]).u32()
    }
}

// Translated from 0043aaf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Model::Model` (Xbox PDB): clears the object pointer and loads the model
/// `name` from `stream` ([`model_init_model`]); returns `this`.
pub fn model_model(
    e: &mut Engine,
    this: Ptr<Model>,
    name: u32,
    stream: Ptr,
    prepare: u8,
    force: u8,
) -> Ptr<Model> {
    let object_3d = object_3d_pointer(this);
    e.call(NI_POINTER_CONSTRUCT, &args![object_3d, 0u32]);
    e.call(NI_POINTER_ASSIGN, &args![object_3d, 0u32]);
    model_init_model(e, this, name, stream, prepare, force);
    this
}

// Translated from 0043ab70 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Model::~Model` (Xbox PDB): releases the 3D object, frees the file name
/// and destroys the `NiPointer`.
pub fn model_destructor(e: &mut Engine, this: Ptr<Model>) {
    let object_3d = object_3d_pointer(this);
    e.call(NI_POINTER_ASSIGN, &args![object_3d, 0u32]);
    let filename = e.get(this, Model::pFilename);
    e.call(MEMORY_FREE, &args![filename]);
    e.call(NI_POINTER_DESTRUCT, &args![object_3d]);
}

// Translated from 0043abf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// A second `Model` constructor (the map has no name for it): copies `name`,
/// holds the already loaded `node` as the 3D object and clears both counts.
/// Returns `this`.
pub fn fn_0043abf0(e: &mut Engine, this: Ptr<Model>, name: u32, node: Ptr) -> Ptr<Model> {
    let object_3d = object_3d_pointer(this);
    e.call(NI_POINTER_CONSTRUCT, &args![object_3d, 0u32]);
    let copy = copy_name(e, name);
    e.set(this, Model::pFilename, copy);
    e.call(NI_POINTER_ASSIGN, &args![object_3d, node]);
    e.set(this, Model::iRefCount, 0);
    e.set(this, Model::iManualRefCount, 0);
    this
}

// Translated from 0043acb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Decrements `iManualRefCount` (interlocked) when it is positive.
pub fn fn_0043acb0(e: &mut Engine, this: Ptr<Model>) {
    if e.get(this, Model::iManualRefCount) > 0 {
        let counter = this.byte_add(Model::iManualRefCount.off);
        e.call(INTERLOCKED_DECREMENT, &args![counter]);
    }
}

// Translated from 0043ace0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Model::InitModel` (Xbox PDB): copies `name` into `pFilename`, takes the
/// `NiNode`s out of the loaded `stream` into `spObject3D` and prepares them.
///
/// A stream without objects first loads `Meshes\Marker_Error.NIF` (and leaves
/// `pFilename` null if even that fails). A first object that is a node
/// becomes the model's object directly, unless its name matches `Bip##`, in
/// which case it is added under a fresh wrapper node. Otherwise every node or
/// geometry of the stream is added, in order, to a wrapper node. When
/// something was taken, the editor markers are removed, the two shared
/// properties (z-buffer and vertex colour) are stripped from the root when
/// it holds exactly those objects (with a "Reexport" log message), and, if
/// `prepare` is set, the object is handed to the shader manager (`force` or a
/// morpher controller in the tree sets its last flag).
pub fn model_init_model(
    e: &mut Engine,
    this: Ptr<Model>,
    name: u32,
    stream: Ptr,
    prepare: u8,
    force: u8,
) {
    e.set(this, Model::iRefCount, 0);
    e.set(this, Model::iManualRefCount, 0);
    let copy = copy_name(e, name);
    e.set(this, Model::pFilename, copy);

    if fn_0043b1e0(e, stream) == 0 {
        e.call(STREAM_FREE_ALL_OBJECTS, &args![stream]);
        // Slot 0x60 of the stream: load a file into it.
        if !e
            .vcall(stream.addr(), 0x60, &args![MARKER_ERROR_NIF, 0u32])
            .bool()
        {
            e.set(this, Model::pFilename, 0);
            return;
        }
    }

    let object_3d = object_3d_pointer(this);
    let first = fn_0043b200(e, stream, 0);
    let mut taken = false;
    // Slot 0xC of an `NiObject`: `IsNode` (non-null for an `NiNode`).
    if !first.is_null() && e.vcall(first.addr(), 0xc, &args![]).u32() != 0 {
        let name_slot = e.call(NAME_SLOT, &args![first]).ptr();
        let node_name = fn_0043b1b0(e, name_slot);
        if node_name != 0
            && e.call(MATCHES_PATTERN, &args![node_name, BIP_PATTERN])
                .bool()
        {
            let wrapper = new_ni_node(e);
            e.call(NI_POINTER_ASSIGN, &args![object_3d, wrapper]);
            let root = ni_pointer_get(e, object_3d);
            // Slot 0xF8 of `NiNode`: `SetAt(index, child)`.
            e.vcall(root.addr(), 0xf8, &args![0u32, first]);
        } else {
            e.call(NI_POINTER_ASSIGN, &args![object_3d, first]);
        }
        taken = true;
    } else {
        let mut slot = 0u32;
        let mut index = 0u32;
        while index < fn_0043b1e0(e, stream) {
            let object = fn_0043b200(e, stream, index);
            if !object.is_null() {
                // Slot 0xC `IsNode`, slot 0x18 `IsGeometry`.
                let takes = e.vcall(object.addr(), 0xc, &args![]).u32() != 0
                    || e.vcall(object.addr(), 0x18, &args![]).u32() != 0;
                if takes {
                    if ni_pointer_get(e, object_3d).is_null() {
                        let wrapper = new_ni_node(e);
                        e.call(NI_POINTER_ASSIGN, &args![object_3d, wrapper]);
                    }
                    let root = ni_pointer_get(e, object_3d);
                    e.vcall(root.addr(), 0xf8, &args![slot, object]);
                    slot += 1;
                    taken = true;
                }
            }
            index += 1;
        }
    }
    if !taken {
        return;
    }

    let root = ni_pointer_get(e, object_3d);
    e.call(REMOVE_EDITOR_MARKERS, &args![root]);

    if e.global::<u8>(GUN_WOBBLE_FLAG) != 0 {
        let root = ni_pointer_get(e, object_3d);
        let child = fn_0043b230(e, root);
        let cast = e.call(DYNAMIC_CAST, &args![TYPE_DESCRIPTOR_011F36AC, child]);
        if cast.u32() != 0 {
            let target: Ptr = e.call(CAST_RESULT_POINTER, &args![cast.u32()]).ptr();
            if !target.is_null() {
                let root = ni_pointer_get(e, object_3d);
                let name_slot = e.call(NAME_SLOT, &args![root]).ptr();
                if fn_0043b1b0(e, name_slot) != 0 {
                    let root = ni_pointer_get(e, object_3d);
                    let name_slot = e.call(NAME_SLOT, &args![root]);
                    e.vcall(target.addr(), 0x90, &args![name_slot.u32(), 0u32]);
                }
            }
        }
    }

    let mut stripped = false;
    message_handler_inc_disable_warning_count(e, 1);
    let property_type = fn_0043b290(e);
    let root = ni_pointer_get(e, object_3d);
    let property: Ptr = e.call(GET_PROPERTY, &args![root, property_type]).ptr();
    if !property.is_null() {
        let shared = fn_0043b2a0(e);
        if fn_0043b260(e, property, shared) {
            let property_type = fn_0043b290(e);
            let root = ni_pointer_get(e, object_3d);
            e.call(REMOVE_PROPERTY, &args![root, property_type]);
            stripped = true;
        }
    }
    let property_type = e.call(PROPERTY_TYPE_SECOND_GETTER, &args![]).u32();
    let root = ni_pointer_get(e, object_3d);
    let property: Ptr = e.call(GET_PROPERTY, &args![root, property_type]).ptr();
    if !property.is_null() {
        let shared = fn_0043b250(e);
        if fn_0043b260(e, property, shared) {
            let property_type = e.call(PROPERTY_TYPE_SECOND_GETTER, &args![]).u32();
            let root = ni_pointer_get(e, object_3d);
            e.call(REMOVE_PROPERTY, &args![root, property_type]);
            stripped = true;
        }
    }
    if stripped {
        // `BSStream` byte at +8 (the file name's first character): non-zero
        // when the stream has a file name.
        if e.mem.i8(stream.addr() + 8) != 0 {
            e.call(
                LOG,
                &args![REEXPORT_WITH_FILE_MESSAGE, stream.addr() + 8, name],
            );
        } else {
            e.call(LOG, &args![REEXPORT_MESSAGE, name]);
        }
    }
    message_handler_inc_disable_warning_count(e, 0);

    if prepare == 0 {
        return;
    }
    let mut has_morpher = 0u32;
    if force != 0 {
        has_morpher = 1;
    } else {
        let root = ni_pointer_get(e, object_3d);
        if e.call(HAS_MORPHER_CONTROLLER, &args![root]).bool() {
            has_morpher = 1;
        }
    }
    let root = ni_pointer_get(e, object_3d);
    e.call(PREPARE_OBJECT, &args![root, force as u32, has_morpher]);
}

// Translated from 0043b1b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The dword at `pointer`, read through the acquire-load helper `0043b1d0`
/// (the name slot of an object is read this way).
pub fn fn_0043b1b0(e: &mut Engine, pointer: Ptr) -> u32 {
    fn_0043b1d0(e, pointer)
}

// Translated from 0043b1d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Concurrency::details::_Subatomic_impl<4>::_LoadWithAquire` (library): the
/// dword at `pointer`.
pub fn fn_0043b1d0(e: &mut Engine, pointer: Ptr) -> u32 {
    e.mem.u32(pointer.addr())
}

// Translated from 0043b1e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSStream`: the number of objects in the stream's array at +0x21C.
pub fn fn_0043b1e0(e: &mut Engine, stream: Ptr) -> u32 {
    e.call(ARRAY_COUNT, &args![stream.byte_add(0x21c)]).u32()
}

// Translated from 0043b200 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSStream`: the object at `index` of the stream's array at +0x21C.
pub fn fn_0043b200(e: &mut Engine, stream: Ptr, index: u32) -> Ptr {
    let element = e
        .call(ARRAY_ELEMENT_ADDRESS, &args![stream.byte_add(0x21c), index])
        .ptr::<()>();
    ni_pointer_get(e, element)
}

// Translated from 0043b230 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `NiPointer` at +0xC of `object` dereferenced (in Gamebryo's
/// `NiObjectNET` layout, the head of the controller list).
pub fn fn_0043b230(e: &mut Engine, object: Ptr) -> Ptr {
    ni_pointer_get(e, object.byte_add(0xc))
}

// Translated from 0043b250 (decompiled, FalloutNV.exe 1.4.0.525)
/// The shared property held by the global `NiPointer` at `011f4438`.
pub fn fn_0043b250(e: &mut Engine) -> Ptr {
    ni_pointer_get(e, Ptr::new(SHARED_PROPERTY_SECOND))
}

// Translated from 0043b260 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether `this` and `other` have the same value in the 16-bit field at
/// +0x18 (the shared-property check of `InitModel`).
pub fn fn_0043b260(e: &mut Engine, this: Ptr, other: Ptr) -> bool {
    e.mem.u16(this.addr() + 0x18) == e.mem.u16(other.addr() + 0x18)
}

// Translated from 0043b290 (decompiled, FalloutNV.exe 1.4.0.525)
/// The property type `InitModel` strips first (11).
pub fn fn_0043b290(_e: &mut Engine) -> u32 {
    PROPERTY_TYPE_FIRST
}

// Translated from 0043b2a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The shared property held by the global `NiPointer` at `011f4444`.
pub fn fn_0043b2a0(e: &mut Engine) -> Ptr {
    ni_pointer_get(e, Ptr::new(SHARED_PROPERTY_FIRST))
}

// Translated from 0043b2b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MessageHandler::IncDisableWarningCount` (Xbox PDB): increments (when
/// `increment` is non-zero) or decrements the disable-warning counter with an
/// interlocked operation and clamps it at 0.
pub fn message_handler_inc_disable_warning_count(e: &mut Engine, increment: u8) {
    if increment != 0 {
        e.call(INTERLOCKED_INCREMENT, &args![DISABLE_WARNING_COUNT]);
    } else {
        e.call(INTERLOCKED_DECREMENT, &args![DISABLE_WARNING_COUNT]);
    }
    if e.global::<i32>(DISABLE_WARNING_COUNT) < 0 {
        e.set_global(DISABLE_WARNING_COUNT, 0i32);
    }
}

// Translated from 0043b300 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether `object` is an instance of the class with type descriptor
/// `type_descriptor`; false for a null `object`.
pub fn fn_0043b300(e: &mut Engine, type_descriptor: u32, object: Ptr) -> bool {
    if object.is_null() {
        return false;
    }
    e.call(IS_KIND_OF, &args![object, type_descriptor]).bool()
}

// Translated from 0043b320 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiNode`: stores `child` at `index` of the child array (base at +0x38) and
/// sets the flag `0x80` of a non-null child ([`fn_0043b350`]).
pub fn fn_0043b320(e: &mut Engine, this: Ptr, index: u16, child: Ptr) {
    let base = e.mem.u32(this.addr() + 0x38);
    e.mem.set_u32(base + index as u32 * 4, child.addr());
    if !child.is_null() {
        fn_0043b350(e, child, 1);
    }
}

// Translated from 0043b350 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets (`set` non-zero) or clears the flag `0x80` of the flag word at +0x30.
pub fn fn_0043b350(e: &mut Engine, this: Ptr, set: u8) {
    fn_0043b370(e, this, set, 0x80);
}

// Translated from 0043b370 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets (`set` non-zero) or clears the bits `mask` of the flag word at +0x30.
pub fn fn_0043b370(e: &mut Engine, this: Ptr, set: u8, mask: u32) {
    let flags = e.mem.u32(this.addr() + 0x30);
    let flags = if set != 0 {
        flags | mask
    } else {
        !mask & flags
    };
    e.mem.set_u32(this.addr() + 0x30, flags);
}

// Translated from 0043b3b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiNode`: the index of `child` in the child array (base at +0x38, 16-bit
/// count at +0x3C), or the word at `01096320` (`0xFFFF`) when it is not there.
pub fn fn_0043b3b0(e: &mut Engine, this: Ptr, child: Ptr) -> u16 {
    let base = e.mem.u32(this.addr() + 0x38);
    let count = e.mem.u16(this.addr() + 0x3c);
    for index in 0..count {
        if e.mem.u32(base + index as u32 * 4) == child.addr() {
            return index;
        }
    }
    e.global::<u16>(NOT_FOUND_INDEX)
}

// Translated from 0043b410 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Model::ModManualRefCount` (Xbox PDB): adds `delta` to `iManualRefCount`
/// with a compare-and-swap loop (sleeping between failed attempts).
pub fn model_mod_manual_ref_count(e: &mut Engine, this: Ptr<Model>, delta: i32) {
    let counter = this.byte_add(Model::iManualRefCount.off);
    let mut current = e.get(this, Model::iManualRefCount);
    while fn_0043b460(e, counter, current, current.wrapping_add(delta)) != current {
        e.call(SLEEP, &args![0u32]);
        current = e.get(this, Model::iManualRefCount);
    }
}

// Translated from 0043b460 (decompiled, FalloutNV.exe 1.4.0.525)
/// `InterlockedCompareExchange(target, new, old)` wrapper: returns the value
/// `target` held.
pub fn fn_0043b460(e: &mut Engine, target: Ptr, old: i32, new: i32) -> i32 {
    e.call(INTERLOCKED_COMPARE_EXCHANGE, &args![target, new, old])
        .i32()
}

// Translated from 0043b480 (decompiled, FalloutNV.exe 1.4.0.525)
/// The number of elements of the array at +0x9C.
pub fn fn_0043b480(e: &mut Engine, this: Ptr) -> u16 {
    e.call(ARRAY_SIZE, &args![this.byte_add(0x9c)]).u16()
}

// Translated from 0043b4a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The object at `index` of the array at +0x9C.
pub fn fn_0043b4a0(e: &mut Engine, this: Ptr, index: u32) -> Ptr {
    let element = e
        .call(ARRAY_ELEMENT_ADDRESS, &args![this.byte_add(0x9c), index])
        .ptr::<()>();
    ni_pointer_get(e, element)
}

// Translated from 0043b4d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The low seven bits of the dword at `this`.
pub fn fn_0043b4d0(e: &mut Engine, this: Ptr) -> u32 {
    e.mem.u32(this.addr()) & 0x7f
}

// Translated from 0043b4f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Compiler-emitted helper (the decompiler names it
/// `std::locale::id::id`): stores into `out` (`008c71b0`) the value
/// [`fn_0043b540`] reads from the block [`fn_0043b560`] finds for `this`
/// (0 when there is none); returns `out`.
pub fn fn_0043b4f0(e: &mut Engine, this: Ptr, out: Ptr) -> Ptr {
    let holder = fn_0043b560(e, this);
    let value = if holder != 0 {
        fn_0043b540(e, Ptr::new(holder))
    } else {
        0
    };
    e.call(STORE_POINTER, &args![out, value]);
    out
}

// Translated from 0043b540 (decompiled, FalloutNV.exe 1.4.0.525)
/// The field at +8 (`0044ddc0`) of the object embedded at +0x14.
pub fn fn_0043b540(e: &mut Engine, this: Ptr) -> u32 {
    e.call(FIELD_AT_8, &args![this.byte_add(0x14)]).u32()
}

// Translated from 0043b560 (decompiled, FalloutNV.exe 1.4.0.525)
/// Compiler-emitted helper (the decompiler names it after
/// `std::_Ref_count_obj<_ExceptionHolder>`): finds the block at the field of
/// `this` that `00620b80` returns; if there is one, runs `0043b5b0` on it,
/// takes `block + 0x10` (`00460140`) as the result and runs `0043b5f0`.
/// Returns 0 when there is no block.
pub fn fn_0043b560(e: &mut Engine, this: Ptr) -> u32 {
    let mut value = 0;
    let held: Ptr = e.call(REF_COUNT_OBJECT_POINTER, &args![this]).ptr();
    if !held.is_null() {
        fn_0043b5b0(e, held);
        value = e.call(HOLDER_VALUE, &args![held]).u32();
        fn_0043b5f0(e, held);
    }
    value
}

// Translated from 0043b5b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `std::_Ref_count_obj<_ExceptionHolder>::_Destroy` (library): calls
/// `004534f0` on the object embedded at +0x6C with the argument 1 (which the
/// game pushes before `0043b5d0` and `004534f0` takes).
pub fn fn_0043b5b0(e: &mut Engine, this: Ptr) {
    let held = fn_0043b5d0(e, this);
    e.call(DELETE_OBJECT, &args![held, 1u32]);
}

// Translated from 0043b5d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `std::_Ref_count_obj<_ExceptionHolder>::_Getptr` (library): the held
/// object, embedded at +0x6C.
pub fn fn_0043b5d0(_e: &mut Engine, this: Ptr) -> Ptr {
    this.byte_add(0x6c)
}

// Translated from 0043b5f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls `00483710` on the object embedded at +0x6C ([`fn_0043b5d0`]).
pub fn fn_0043b5f0(e: &mut Engine, this: Ptr) {
    let held = fn_0043b5d0(e, this);
    e.call(HOLDER_DESTRUCT, &args![held]);
}

// Translated from 0043b610 (decompiled, FalloutNV.exe 1.4.0.525)
/// `bhkCollisionObject::GetbhkCollisionObject` (Xbox PDB): the collision
/// object of `object`, cast to `bhkCollisionObject` (null when it is not
/// one).
pub fn bhk_collision_object_get_bhk_collision_object(e: &mut Engine, object: Ptr) -> Ptr {
    let collision_object: Ptr = e.call(COLLISION_OBJECT, &args![object]).ptr();
    e.call(
        DYNAMIC_CAST,
        &args![TYPE_DESCRIPTOR_BHK_COLLISION_OBJECT, collision_object],
    )
    .ptr()
}

// Translated from 0043b640 (decompiled, FalloutNV.exe 1.4.0.525)
/// `KFModel::KFModel` (Xbox PDB): copies `name`, builds the sequence from the
/// loaded `file` and, when that worked, loads its animation group
/// ([`fn_0043b7e0`]); otherwise logs that no sequence could be created.
/// Returns `this`.
pub fn kf_model_kf_model(e: &mut Engine, this: Ptr<KFModel>, name: u32, file: Ptr) -> Ptr<KFModel> {
    let sequence = sequence_pointer(this);
    let anim_group = anim_group_pointer(this);
    e.call(NI_POINTER_CONSTRUCT, &args![sequence, 0u32]);
    e.call(NI_POINTER_CONSTRUCT, &args![anim_group, 0u32]);
    e.set(this, KFModel::iRefCount, 0);
    e.set(this, KFModel::iManualRefCount, 0);
    e.call(NI_POINTER_ASSIGN, &args![anim_group, 0u32]);
    let copy = copy_name(e, name);
    e.set(this, KFModel::pFilename, copy);
    e.call(LOAD_KF_SEQUENCE, &args![file, 0u32, sequence]);
    if !ni_pointer_get(e, sequence).is_null() {
        fn_0043b7e0(e, this, name);
    } else {
        let filename = e.get(this, KFModel::pFilename);
        e.call(LOG, &args![NO_SEQUENCE_MESSAGE, filename]);
    }
    this
}

// Translated from 0043b750 (decompiled, FalloutNV.exe 1.4.0.525)
/// `KFModel::~KFModel` (the map has no name for it): frees the file name and
/// releases the animation group and the sequence.
pub fn fn_0043b750(e: &mut Engine, this: Ptr<KFModel>) {
    let sequence = sequence_pointer(this);
    let anim_group = anim_group_pointer(this);
    let filename = e.get(this, KFModel::pFilename);
    e.call(MEMORY_FREE, &args![filename]);
    e.call(NI_POINTER_ASSIGN, &args![sequence, 0u32]);
    e.call(NI_POINTER_ASSIGN, &args![anim_group, 0u32]);
    e.call(NI_POINTER_DESTRUCT, &args![anim_group]);
    e.call(NI_POINTER_DESTRUCT, &args![sequence]);
}

// Translated from 0043b7e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Finishes a `KFModel` once its sequence exists: when the sequence's name
/// has an underscore, the sequence is first renamed to the part before it;
/// then the animation group is loaded from `name` (`LoadAnimGroup`), and, if
/// the sequence is of the class `011c7d74`, the group is stored in the
/// sequence (`0043baa0`); finally the sequence is named `name`.
///
/// The stack-cookie check at the end is not translated.
pub fn fn_0043b7e0(e: &mut Engine, this: Ptr<KFModel>, name: u32) {
    let sequence_slot = sequence_pointer(this);
    if ni_pointer_get(e, sequence_slot).is_null() {
        return;
    }
    let sequence = ni_pointer_get(e, sequence_slot);
    let name_slot = e.call(NAME_SLOT, &args![sequence]).ptr();
    let sequence_name = fn_0043b1b0(e, name_slot);
    if sequence_name != 0 {
        let underscore = fn_0043b9d0(e, sequence_name, 0x5f);
        if !underscore.is_null() {
            let prefix_length = underscore.addr().wrapping_sub(sequence_name);
            // `char buffer[268]` of the game's stack frame.
            e.with_stack(0x10c, |e, buffer| {
                e.call(MEMORY_COPY, &args![buffer, sequence_name, prefix_length]);
                e.mem.set_u8(buffer.addr().wrapping_add(prefix_length), 0);
                e.with_stack(4, |e, temporary| {
                    let prefix: Ptr<NiFixedString> = e
                        .call(FIXED_STRING_CONSTRUCT, &args![temporary, buffer])
                        .ptr();
                    let sequence = ni_pointer_get(e, sequence_slot);
                    fn_0043b9f0(e, sequence, prefix);
                    e.call(FIXED_STRING_DESTRUCT, &args![temporary]);
                });
            });
        }
    }

    let sequence = ni_pointer_get(e, sequence_slot);
    let group: Ptr = e.call(LOAD_ANIM_GROUP, &args![sequence, name]).ptr();
    e.call(NI_POINTER_ASSIGN, &args![anim_group_pointer(this), group]);
    let sequence = ni_pointer_get(e, sequence_slot);
    if fn_0043b300(e, TYPE_DESCRIPTOR_011C7D74, sequence) {
        let group = ni_pointer_get(e, anim_group_pointer(this));
        // `007fa950` takes no stack argument: the pushed group is the
        // argument of `0043baa0`.
        let sequence: Ptr = e.call(KF_MODEL_SEQUENCE, &args![this]).ptr();
        fn_0043baa0(e, sequence, group);
    }

    e.with_stack(4, |e, temporary| {
        let full_name: Ptr<NiFixedString> = e
            .call(FIXED_STRING_CONSTRUCT, &args![temporary, name])
            .ptr();
        let sequence = ni_pointer_get(e, sequence_slot);
        fn_0043b9f0(e, sequence, full_name);
        e.call(FIXED_STRING_DESTRUCT, &args![temporary]);
    });
}

// Translated from 0043b9d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `strchr` (library): the first occurrence of `character` in `text`.
pub fn fn_0043b9d0(e: &mut Engine, text: u32, character: i32) -> Ptr {
    e.call(STRCHR, &args![text, character]).ptr()
}

// Translated from 0043b9f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiControllerSequence`: assigns `name` to the `NiFixedString` at +8 (the
/// sequence's name).
pub fn fn_0043b9f0(e: &mut Engine, this: Ptr, name: Ptr<NiFixedString>) -> Ptr<NiFixedString> {
    fn_0043ba10(e, this.byte_add(8).cast(), name)
}

// Translated from 0043ba10 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiFixedString::operator=` (library): when the handles differ, adds a
/// reference to the new string, releases the old one and stores the new
/// handle. Returns `this`.
pub fn fn_0043ba10(
    e: &mut Engine,
    this: Ptr<NiFixedString>,
    other: Ptr<NiFixedString>,
) -> Ptr<NiFixedString> {
    let handle = e.get(this, NiFixedString::m_kHandle);
    let new_handle = e.get(other, NiFixedString::m_kHandle);
    if handle != new_handle {
        e.with_stack(4, |e, local| {
            e.mem.set_u32(local.addr(), new_handle);
            fn_0043ba60(e, local);
            e.call(FIXED_STRING_RELEASE, &args![this]);
            let stored = e.mem.u32(local.addr());
            e.set(this, NiFixedString::m_kHandle, stored);
        });
    }
    this
}

// Translated from 0043ba60 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiFixedString` reference add (library): increments the pooled string's
/// reference count unless the handle is the empty string's.
pub fn fn_0043ba60(e: &mut Engine, handle: Ptr) {
    if e.mem.u32(handle.addr()) != e.global::<u32>(EMPTY_FIXED_STRING) {
        let counter = e.call(FIXED_STRING_REFCOUNT, &args![handle]).u32();
        e.call(INTERLOCKED_INCREMENT, &args![counter]);
    }
}

// Translated from 0043baa0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiControllerSequence`: assigns `group` to the `NiPointer` at +0x74 (the
/// sequence's animation group). Returns that `NiPointer`.
pub fn fn_0043baa0(e: &mut Engine, this: Ptr, group: Ptr) -> Ptr {
    e.call(NI_POINTER_ASSIGN, &args![this.byte_add(0x74), group])
        .ptr()
}

/// The address of `QueuedTexture::spTexture`, which the game passes to the
/// `NiPointer` functions.
fn texture_pointer(texture: Ptr<QueuedTexture>) -> Ptr {
    texture.byte_add(QueuedTexture::spTexture.off)
}

/// The address of `QueuedModel::spModel`.
fn model_pointer(model: Ptr<QueuedModel>) -> Ptr {
    model.byte_add(QueuedModel::spModel.off)
}

/// `QueuedFileEntry::pFileEntry` through the game's getter (`0055b980`).
fn queued_file_entry(e: &mut Engine, this: Ptr<QueuedTexture>) -> u32 {
    e.call(QUEUED_FILE_ENTRY_GET_FILE_ENTRY, &args![this]).u32()
}

/// `QueuedFileEntry::pFileName` through the game's getter (`0045cd60`).
fn queued_file_name(e: &mut Engine, this: Ptr<QueuedTexture>) -> u32 {
    e.call(QUEUED_FILE_ENTRY_GET_FILE_NAME, &args![this]).u32()
}

/// The address of a queued texture's `cFlags` byte, which the game passes to
/// the flag helpers.
fn texture_flags_pointer(texture: Ptr<QueuedTexture>) -> Ptr {
    texture.byte_add(QueuedTexture::cFlags.off)
}

// Translated from 0043bac0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedChildren` method (no name in the engine map): decrements
/// `iNumChildrenFinished` (interlocked) when it is positive.
pub fn fn_0043bac0(e: &mut Engine, this: Ptr<QueuedChildren>) {
    // The comparison is signed although the Xbox PDB declares a `u32`.
    if e.get(this, QueuedChildren::iNumChildrenFinished) as i32 > 0 {
        let counter = this.byte_add(QueuedChildren::iNumChildrenFinished.off);
        e.call(INTERLOCKED_DECREMENT, &args![counter]);
    }
}

// Translated from 0043baf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `LoadedFile::LoadedFile` (no name in the engine map): keeps `file`,
/// copies `name`, adds the object to the map of loaded files (the result
/// becomes `bInLoadedFileMap`) and starts it unused with no references.
/// Returns `this`.
pub fn fn_0043baf0(e: &mut Engine, this: Ptr<LoadedFile>, name: u32, file: Ptr) -> Ptr<LoadedFile> {
    e.set(this, LoadedFile::pFile, file);
    let copy = copy_name(e, name);
    e.set(this, LoadedFile::pFileName, copy);
    let owner = e.global::<u32>(FILE_MAP_OWNER);
    let added = e
        .call(FILE_MAP_ADD_LOADED_FILE, &args![owner, copy, this])
        .bool();
    e.set(this, LoadedFile::bInLoadedFileMap, added);
    e.set(this, LoadedFile::bFileUsed, false);
    e.set(this, LoadedFile::iRefCount, 0);
    this
}

// Translated from 0043bb80 (decompiled, FalloutNV.exe 1.4.0.525)
/// `LoadedFile::~LoadedFile` (Xbox PDB): complains (log) about a file that
/// was added to the map but never used, removes it from the map, reports a
/// non-zero reference count to the message sink, frees the name and deletes
/// the file.
///
/// The stack-cookie check is not translated.
pub fn loaded_file_destructor(e: &mut Engine, this: Ptr<LoadedFile>) {
    let name = e.get(this, LoadedFile::pFileName);
    let in_map = e.get(this, LoadedFile::bInLoadedFileMap);
    if !e.get(this, LoadedFile::bFileUsed) && in_map {
        e.call(LOG, &args![LOADED_FILE_UNUSED_MESSAGE, name, this]);
    }
    if in_map {
        let owner = e.global::<u32>(FILE_MAP_OWNER);
        e.call(FILE_MAP_REMOVE_LOADED_FILE, &args![owner, name]);
    }
    let references = e.get(this, LoadedFile::iRefCount);
    if references != 0 {
        // `char text[268]` of the game's stack frame.
        e.with_stack(0x10c, |e, text| {
            e.call(
                SPRINTF,
                &args![text, LOADED_FILE_REFERENCES_MESSAGE, name, this, references],
            );
            if fn_0043bce0(e) {
                let sink = fn_0043bd00(e);
                // Slot 0x14: report `(severity 4, text, 0, 0, 0)`.
                e.vcall(sink, 0x14, &args![4u32, text, 0u32, 0u32, 0u32]);
            }
        });
    }
    let name = e.get(this, LoadedFile::pFileName);
    e.call(MEMORY_FREE, &args![name]);
    let file = e.get(this, LoadedFile::pFile);
    if !file.is_null() {
        // Slot 0: the scalar deleting destructor, with the delete flag.
        e.vcall(file.addr(), 0, &args![1u32]);
    }
}

// Translated from 0043bce0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the message sink at `011f6388` exists.
pub fn fn_0043bce0(e: &mut Engine) -> bool {
    e.global::<u32>(MESSAGE_SINK) != 0
}

// Translated from 0043bd00 (decompiled, FalloutNV.exe 1.4.0.525)
/// The message sink at `011f6388`.
pub fn fn_0043bd00(e: &mut Engine) -> u32 {
    e.global::<u32>(MESSAGE_SINK)
}

// Translated from 0043bd10 (decompiled, FalloutNV.exe 1.4.0.525)
/// A `QueuedTexture` constructor (no name in the engine map) that loads by
/// file name: the base constructor with `context`, the texture's virtual
/// table, a null texture and no flags, then the path `name` normalized into
/// a 260-character buffer becomes `pFileName` and its file entry is looked
/// up (flag 1). Returns `this`.
///
/// The stack-cookie check and the exception-unwinding frame are not
/// translated.
pub fn fn_0043bd10(
    e: &mut Engine,
    this: Ptr<QueuedTexture>,
    name: u32,
    context: u32,
) -> Ptr<QueuedTexture> {
    e.call(QUEUED_FILE_ENTRY_CONSTRUCT, &args![this, context]);
    e.mem.set_u32(this.addr(), QUEUED_TEXTURE_VTABLE);
    let texture = texture_pointer(this);
    e.call(NI_POINTER_CONSTRUCT, &args![texture, 0u32]);
    e.set(this, QueuedTexture::cFlags, 0);
    // `char path[260]` of the game's stack frame (268 bytes with padding).
    e.with_stack(0x10c, |e, path| {
        e.call(NORMALIZE_PATH, &args![name, path, 0x104u32]);
        e.call(QUEUED_FILE_ENTRY_SET_FILE_NAME, &args![this, path]);
    });
    e.call(QUEUED_FILE_ENTRY_FIND_FILE_ENTRY, &args![this, 1u32]);
    this
}

// Translated from 0043bde0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedFile::NotifyChildFinished` (Xbox PDB): counts one more finished
/// child ([`fn_0043be10`] on `pChildren`), then asks the file whether it is
/// finished (slot `0x28`, `CheckFinished`). The stack argument (`RET 4`,
/// the finished child) is not used.
pub fn queued_file_notify_child_finished(e: &mut Engine, this: Ptr<QueuedFile>, _child: Ptr) {
    let children = e.get(this, QueuedFile::pChildren);
    fn_0043be10(e, children.cast());
    e.vcall(this.addr(), 0x28, &args![]);
}

// Translated from 0043be10 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedChildren` method (no name in the engine map): increments
/// `iNumChildrenFinished` (interlocked).
pub fn fn_0043be10(e: &mut Engine, this: Ptr<QueuedChildren>) {
    let counter = this.byte_add(QueuedChildren::iNumChildrenFinished.off);
    e.call(INTERLOCKED_INCREMENT, &args![counter]);
}

// Translated from 0043be30 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedTexture::_scalar_deleting_destructor_` (Xbox PDB): runs the
/// destructor ([`fn_0043bf80`]) and, when bit 0 of `flags` is set, frees the
/// object. Returns `this`.
pub fn queued_texture_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr<QueuedTexture>,
    flags: u32,
) -> Ptr<QueuedTexture> {
    fn_0043bf80(e, this);
    if flags & 1 != 0 {
        e.call(MEMORY_FREE, &args![this]);
    }
    this
}

// Translated from 0043be60 (decompiled, FalloutNV.exe 1.4.0.525)
/// A `QueuedTexture` constructor (no name in the engine map) that is given
/// the file entry: like [`fn_0043bd10`] but it stores `file_entry` as
/// `pFileEntry` instead of resolving a name. Returns `this`.
pub fn fn_0043be60(
    e: &mut Engine,
    this: Ptr<QueuedTexture>,
    file_entry: Ptr,
    context: u32,
) -> Ptr<QueuedTexture> {
    e.call(QUEUED_FILE_ENTRY_CONSTRUCT, &args![this, context]);
    e.mem.set_u32(this.addr(), QUEUED_TEXTURE_VTABLE);
    let texture = texture_pointer(this);
    e.call(NI_POINTER_CONSTRUCT, &args![texture, 0u32]);
    e.set(this, QueuedTexture::cFlags, 0);
    e.call(QUEUED_FILE_ENTRY_SET_FILE_ENTRY, &args![this, file_entry]);
    this
}

// Translated from 0043bef0 (decompiled, FalloutNV.exe 1.4.0.525)
/// A `QueuedTexture` constructor (no name in the engine map) that is given
/// the finished texture: like [`fn_0043be60`] but it stores `texture` in
/// `spTexture` and runs `00449150`, which puts the task in state 5 (done).
/// Returns `this`.
pub fn fn_0043bef0(
    e: &mut Engine,
    this: Ptr<QueuedTexture>,
    texture: Ptr,
    context: u32,
) -> Ptr<QueuedTexture> {
    e.call(QUEUED_FILE_ENTRY_CONSTRUCT, &args![this, context]);
    e.mem.set_u32(this.addr(), QUEUED_TEXTURE_VTABLE);
    let slot = texture_pointer(this);
    e.call(NI_POINTER_CONSTRUCT, &args![slot, 0u32]);
    e.set(this, QueuedTexture::cFlags, 0);
    e.call(NI_POINTER_ASSIGN, &args![slot, texture]);
    e.call(TASK_SET_DONE, &args![this]);
    this
}

// Translated from 0043bf80 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedTexture` destructor (no name in the engine map): restores the
/// texture's virtual table, releases the texture and runs the base
/// destructor.
pub fn fn_0043bf80(e: &mut Engine, this: Ptr<QueuedTexture>) {
    e.mem.set_u32(this.addr(), QUEUED_TEXTURE_VTABLE);
    let texture = texture_pointer(this);
    e.call(NI_POINTER_DESTRUCT, &args![texture]);
    e.call(QUEUED_FILE_ENTRY_DESTRUCT, &args![this]);
}

// Translated from 0043bfe0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedTexture::GetFileIndex` (Xbox PDB): 0 when the texture has no file
/// entry or its archive is not found, otherwise the archive's file index
/// ([`fn_0043c030`]) plus 4.
pub fn queued_texture_get_file_index(e: &mut Engine, this: Ptr<QueuedTexture>) -> u32 {
    if queued_file_entry(e, this) != 0 {
        let file_entry = queued_file_entry(e, this);
        let archive = e
            .call(FILE_ENTRY_FIND_ARCHIVE, &args![file_entry, 2u32])
            .u32();
        if archive != 0 {
            return fn_0043c030(e, Ptr::new(archive)).wrapping_add(4);
        }
    }
    0
}

// Translated from 0043c030 (decompiled, FalloutNV.exe 1.4.0.525)
/// The dword at +0x1C8 of an archive (what `GetFileIndex` adds 4 to).
pub fn fn_0043c030(e: &mut Engine, this: Ptr) -> u32 {
    e.mem.u32(this.addr() + 0x1c8)
}

// Translated from 0043c050 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedTexture::QueueMe` (Xbox PDB): with a file entry, adds the texture
/// to the map of queued textures by it and records the result as flag bit 4;
/// when the thread flag is set, sets flag bit 2; then hands the task to the
/// task queue (slot `0x48`).
pub fn queued_texture_queue_me(e: &mut Engine, this: Ptr<QueuedTexture>) {
    if queued_file_entry(e, this) != 0 {
        let file_entry = queued_file_entry(e, this);
        let owner = e.global::<u32>(FILE_MAP_OWNER);
        let added = e
            .call(FILE_MAP_ADD_QUEUED_TEXTURE, &args![owner, file_entry, this])
            .u8();
        fn_0043c100(e, this, added);
    }
    if fn_0043c130(e) != 0 {
        fn_0043c0d0(e, this, 1);
    }
    let queue = e.global::<u32>(TASK_QUEUE);
    e.vcall(queue, 0x48, &args![this]);
}

// Translated from 0043c0d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets (`flag` non-zero) or clears bit 2 of the texture's `cFlags`.
pub fn fn_0043c0d0(e: &mut Engine, this: Ptr<QueuedTexture>, flag: u8) {
    let flags = texture_flags_pointer(this);
    e.call(SET_FLAG_BIT_2, &args![flag as u32, flags]);
}

// Translated from 0043c100 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets (`flag` non-zero) or clears bit 4 of the texture's `cFlags`.
pub fn fn_0043c100(e: &mut Engine, this: Ptr<QueuedTexture>, flag: u8) {
    let flags = texture_flags_pointer(this);
    e.call(SET_FLAG_BIT_4, &args![flag as u32, flags]);
}

// Translated from 0043c130 (decompiled, FalloutNV.exe 1.4.0.525)
/// The byte at +0x25C of the thread's TLS block. (The code is a method of
/// the object at `011c3b3c` but does not use `this`.)
pub fn fn_0043c130(e: &mut Engine) -> u8 {
    let address = e.tls() + TLS_QUEUED_FLAG;
    e.mem.u8(address)
}

// Translated from 0043c150 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedTexture::Run` (Xbox PDB): finds the texture, or loads it from its
/// file.
///
/// Inside the memory context of the task (`eContext`), the texture comes
/// from the palette by file entry, or by file name when there is none. If it
/// is there already, flag bit 1 is set, and a texture whose surface format
/// byte ([`fn_0043c430`]) is 3 and whose flag bit 2 is clear gets its
/// virtual function `0x8c` called. Otherwise the file is opened
/// (`QueuedFileEntry::GetFile`); with it, and with the TLS word set to 1 for
/// flag bit 2, the texture is created from it (`00a61040` when the file name
/// contains `_e.dd`, else `00a5fe30`) and stored in `spTexture`; without it
/// the failure is logged with the file entry's offset and size, or with the
/// file name.
///
/// The exception-unwinding frame is not translated.
pub fn queued_texture_run(e: &mut Engine, this: Ptr<QueuedTexture>) {
    let context = e.get(this, QueuedTexture::eContext);
    // The scope guard of the game's stack frame (4 bytes).
    e.with_stack(4, |e, guard| {
        e.call(
            MEMORY_CONTEXT_ENTER,
            &args![guard, context, 1u32, MODEL_LOADER_SOURCE, RUN_SOURCE_LINE],
        );
        queued_texture_run_in_context(e, this);
        e.call(MEMORY_CONTEXT_LEAVE, &args![guard]);
    });
}

/// The body of [`queued_texture_run`] inside its memory context.
fn queued_texture_run_in_context(e: &mut Engine, this: Ptr<QueuedTexture>) {
    let slot = texture_pointer(this);
    if queued_file_entry(e, this) != 0 {
        let file_entry = queued_file_entry(e, this);
        bs_texture_palette_get_texture(e, file_entry, slot);
    } else if queued_file_name(e, this) != 0 {
        let name = queued_file_name(e, this);
        e.call(TEXTURE_PALETTE_GET_TEXTURE_BY_NAME, &args![name, slot]);
    }

    if !ni_pointer_get(e, slot).is_null() {
        e.call(QUEUED_TEXTURE_SET_FLAG_1, &args![this, 1u32]);
        let texture = ni_pointer_get(e, slot);
        if fn_0043c430(e, texture) == 3 && !fn_0043c3d0(e, this) {
            let texture = ni_pointer_get(e, slot);
            e.vcall(texture.addr(), 0x8c, &args![]);
        }
        return;
    }

    let file: Ptr = e
        .call(QUEUED_FILE_ENTRY_GET_FILE, &args![this, 1u32, 2u32])
        .ptr();
    if !file.is_null() {
        // Slot 0x18 of the file: its name.
        let name = e.vcall(file.addr(), 0x18, &args![]).u32();
        let saved = fn_0043c410(e);
        if fn_0043c3d0(e, this) {
            fn_0043c3f0(e, 1);
        }
        if e.call(STRSTR, &args![name, ENVIRONMENT_MAP_SUFFIX]).u32() != 0 {
            let setting = fn_0043c4b0(e);
            // The `NiFixedString` temporary of the game's stack frame.
            e.with_stack(4, |e, local| {
                let fixed_name = e.call(FIXED_STRING_CONSTRUCT, &args![local, name]).u32();
                let texture = e
                    .call(
                        CREATE_TEXTURE_FROM_FIXED_NAME,
                        &args![fixed_name, file, setting, TEXTURE_FORMAT_PREFERENCES],
                    )
                    .u32();
                e.call(NI_POINTER_ASSIGN, &args![slot, texture]);
                e.call(FIXED_STRING_DESTRUCT, &args![local]);
            });
        } else {
            let texture = e
                .call(
                    CREATE_TEXTURE_FROM_FILE,
                    &args![file, name, TEXTURE_FORMAT_PREFERENCES, 1u32],
                )
                .u32();
            e.call(NI_POINTER_ASSIGN, &args![slot, texture]);
        }
        fn_0043c3f0(e, saved);
    } else if queued_file_entry(e, this) != 0 {
        let file_entry = queued_file_entry(e, this);
        let size = e.call(FILE_ENTRY_GET_SIZE, &args![file_entry]).u32();
        let file_entry = queued_file_entry(e, this);
        let offset = fn_0043c3b0(e, Ptr::new(file_entry));
        e.call(LOG, &args![NO_FILE_FOR_TEXTURE_ENTRY_MESSAGE, offset, size]);
    } else if queued_file_name(e, this) != 0 {
        let name = queued_file_name(e, this);
        e.call(LOG, &args![NO_FILE_FOR_TEXTURE_NAME_MESSAGE, name]);
    }
}

// Translated from 0043c3b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSFileEntry::iOffset` with the top bit masked off (the dword at +0xC).
pub fn fn_0043c3b0(e: &mut Engine, this: Ptr) -> u32 {
    e.mem.u32(this.addr() + 0xc) & 0x7fff_ffff
}

// Translated from 0043c3d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether bit 2 of the texture's `cFlags` is set.
pub fn fn_0043c3d0(e: &mut Engine, this: Ptr<QueuedTexture>) -> bool {
    let flags = e.get(this, QueuedTexture::cFlags);
    e.call(TEST_FLAG_BIT_2, &args![flags as u32]).bool()
}

// Translated from 0043c3f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `value` in the word at +0x29C of the thread's TLS block.
pub fn fn_0043c3f0(e: &mut Engine, value: u32) {
    let address = e.tls() + TLS_QUEUED_VALUE;
    e.mem.set_u32(address, value);
}

// Translated from 0043c410 (decompiled, FalloutNV.exe 1.4.0.525)
/// The word at +0x29C of the thread's TLS block.
pub fn fn_0043c410(e: &mut Engine) -> u32 {
    let address = e.tls() + TLS_QUEUED_VALUE;
    e.mem.u32(address)
}

// Translated from 0043c430 (decompiled, FalloutNV.exe 1.4.0.525)
/// A texture's surface format byte: 0 for a texture without a surface,
/// otherwise the byte at +3 of the descriptor at +0x58 of its surface
/// (`0059bb30`, `this + 0x24`).
pub fn fn_0043c430(e: &mut Engine, this: Ptr) -> u32 {
    if e.call(TEXTURE_GET_SURFACE, &args![this]).u32() != 0 {
        let surface = e.call(TEXTURE_GET_SURFACE, &args![this]).ptr();
        let descriptor = fn_0043c490(e, surface);
        fn_0043c470(e, descriptor) as u32
    } else {
        0
    }
}

// Translated from 0043c470 (decompiled, FalloutNV.exe 1.4.0.525)
/// The byte at +3 of `this`.
pub fn fn_0043c470(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr() + 3)
}

// Translated from 0043c490 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address `this + 0x58`.
pub fn fn_0043c490(_e: &mut Engine, this: Ptr) -> Ptr {
    this.byte_add(0x58)
}

// Translated from 0043c4b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The word at `011f4748` (a default texture setting).
pub fn fn_0043c4b0(e: &mut Engine) -> u32 {
    e.global::<u32>(DEFAULT_TEXTURE_SETTING)
}

// Translated from 0043c4c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSTexturePalette::GetTexture` (Xbox PDB): clears the `NiPointer` at
/// `out` and looks the texture of `file_entry` up in the texture map into
/// it.
pub fn bs_texture_palette_get_texture(e: &mut Engine, file_entry: u32, out: Ptr) {
    e.call(NI_POINTER_ASSIGN, &args![out, 0u32]);
    let map = e.global::<u32>(TEXTURE_MAP);
    fn_0043c4f0(e, Ptr::new(map), file_entry, out);
}

// Translated from 0043c4f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSTexturePalette` method (no name in the engine map): looks
/// `file_entry` up in the hash bucket [`fn_0043c530`] gives for it in the
/// texture map `this` (`00a61a60`), storing the texture found into the
/// `NiPointer` at `out`. Returns whether it was found.
pub fn fn_0043c4f0(e: &mut Engine, this: Ptr, file_entry: u32, out: Ptr) -> bool {
    let bucket = fn_0043c530(e, file_entry);
    e.call(TEXTURE_MAP_FIND, &args![this, bucket, file_entry, out])
        .bool()
}

// Translated from 0043c530 (decompiled, FalloutNV.exe 1.4.0.525)
/// The hash bucket of a key: `(key >> 4) % 1001`.
pub fn fn_0043c530(_e: &mut Engine, key: u32) -> u32 {
    (key >> 4) % 0x3e9
}

// Translated from 0043c550 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedTexture::Finish` (Xbox PDB): unless flag bit 1 is set, a loaded
/// texture goes into the palette (`SetTexture`, by file entry, or without
/// one when the texture has only a file name). If the texture is in the map
/// of queued textures (flag bit 4), it is taken out and the flag cleared.
/// Finally asks the task whether it is finished ([`fn_0043c610`]).
pub fn queued_texture_finish(e: &mut Engine, this: Ptr<QueuedTexture>) {
    let flag_1 = e.call(QUEUED_TEXTURE_TEST_FLAG_1, &args![this]).bool();
    let slot = texture_pointer(this);
    if !flag_1 && !ni_pointer_get(e, slot).is_null() {
        if queued_file_entry(e, this) != 0 {
            let file_entry = queued_file_entry(e, this);
            let texture = ni_pointer_get(e, slot);
            e.call(TEXTURE_PALETTE_SET_TEXTURE, &args![texture, file_entry]);
        } else if queued_file_name(e, this) != 0 {
            let texture = ni_pointer_get(e, slot);
            e.call(TEXTURE_PALETTE_SET_TEXTURE, &args![texture, 0u32]);
        }
    }
    leave_texture_map(e, this);
    fn_0043c610(e, this);
}

/// The common end of `Finish` and `Cancel`: a texture with a file entry that
/// is in the map of queued textures (flag bit 4) is removed from it.
fn leave_texture_map(e: &mut Engine, this: Ptr<QueuedTexture>) {
    if queued_file_entry(e, this) != 0 && fn_0043c630(e, this) {
        fn_0043c100(e, this, 0);
        let file_entry = queued_file_entry(e, this);
        let owner = e.global::<u32>(FILE_MAP_OWNER);
        e.call(FILE_MAP_REMOVE_QUEUED_TEXTURE, &args![owner, file_entry]);
    }
}

// Translated from 0043c610 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls the task's virtual function `0x28` (`CheckFinished`).
pub fn fn_0043c610(e: &mut Engine, this: Ptr<QueuedTexture>) {
    e.vcall(this.addr(), 0x28, &args![]);
}

// Translated from 0043c630 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether bit 4 of the texture's `cFlags` is set.
pub fn fn_0043c630(e: &mut Engine, this: Ptr<QueuedTexture>) -> bool {
    let flags = e.get(this, QueuedTexture::cFlags);
    e.call(TEST_FLAG_BIT_4, &args![flags as u32]).bool()
}

// Translated from 0043c650 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedTexture::Cancel` (Xbox PDB): `QueuedFile::Cancel` with its two
/// arguments, then takes the texture out of the map of queued textures as
/// `Finish` does.
pub fn queued_texture_cancel(e: &mut Engine, this: Ptr<QueuedTexture>, first: u32, second: u32) {
    e.call(QUEUED_FILE_CANCEL, &args![this, first, second]);
    leave_texture_map(e, this);
}

// Translated from 0043c6b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedTexture::GetDescription` (Xbox PDB):
/// `QueuedFileEntry::GetDescription` with the kind word `"texture"`;
/// returns its result (always true).
pub fn queued_texture_get_description(
    e: &mut Engine,
    this: Ptr<QueuedTexture>,
    buffer: u32,
    size: u32,
) -> bool {
    e.call(
        QUEUED_FILE_ENTRY_GET_DESCRIPTION,
        &args![this, buffer, size, TEXTURE_WORD],
    )
    .bool()
}

// Translated from 0043c6e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedModel` constructor (no name in the engine map): the base
/// constructor with `context`, the model's virtual table, no model, no
/// `TESModel`, `lod_fade_mult`, no flags; then the file name is copied
/// (`name`), the file entry is looked up (flag 0), and the constructor's two
/// flag arguments set bits 1 and 2 of `cFlags`. The overridden visual
/// distance starts at 0. Returns `this`.
pub fn fn_0043c6e0(
    e: &mut Engine,
    this: Ptr<QueuedModel>,
    name: u32,
    context: u32,
    lod_fade_mult: u32,
    first_flag: u8,
    second_flag: u8,
) -> Ptr<QueuedModel> {
    e.call(QUEUED_FILE_ENTRY_CONSTRUCT, &args![this, context]);
    e.mem.set_u32(this.addr(), QUEUED_MODEL_VTABLE);
    let model = model_pointer(this);
    e.call(NI_POINTER_CONSTRUCT, &args![model, 0u32]);
    e.set(this, QueuedModel::pTESModel, Ptr::NULL);
    e.set(this, QueuedModel::eLODFadeMult, lod_fade_mult);
    e.set(this, QueuedModel::cFlags, 0);
    e.call(QUEUED_FILE_ENTRY_SET_FILE_NAME, &args![this, name]);
    e.call(QUEUED_FILE_ENTRY_FIND_FILE_ENTRY, &args![this, 0u32]);
    fn_0043c7a0(e, this, first_flag);
    fn_0043c7d0(e, this, second_flag);
    e.set(this, QueuedModel::mfOverriddenVisualDistance, 0.0);
    this
}

// Translated from 0043c7a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets (`flag` non-zero) or clears bit 1 of the model's `cFlags`.
pub fn fn_0043c7a0(e: &mut Engine, this: Ptr<QueuedModel>, flag: u8) {
    let flags = this.byte_add(QueuedModel::cFlags.off);
    e.call(SET_FLAG_BIT_1, &args![flag as u32, flags]);
}

// Translated from 0043c7d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets (`flag` non-zero) or clears bit 2 of the model's `cFlags`.
pub fn fn_0043c7d0(e: &mut Engine, this: Ptr<QueuedModel>, flag: u8) {
    let flags = this.byte_add(QueuedModel::cFlags.off);
    e.call(SET_FLAG_BIT_2, &args![flag as u32, flags]);
}

// Translated from 0043c800 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedModel::_scalar_deleting_destructor_` (Xbox PDB): runs the
/// destructor ([`fn_0043c830`]) and, when bit 0 of `flags` is set, frees the
/// object. Returns `this`.
pub fn queued_model_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr<QueuedModel>,
    flags: u32,
) -> Ptr<QueuedModel> {
    fn_0043c830(e, this);
    if flags & 1 != 0 {
        e.call(MEMORY_FREE, &args![this]);
    }
    this
}

// Translated from 0043c830 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedModel` destructor (the engine map names the folded body after a
/// cancellation-token callback): releases the model pointer and runs the
/// base destructor. It does not restore the virtual table.
pub fn fn_0043c830(e: &mut Engine, this: Ptr<QueuedModel>) {
    let model = model_pointer(this);
    e.call(MODEL_POINTER_RELEASE, &args![model]);
    e.call(QUEUED_FILE_ENTRY_DESTRUCT, &args![this]);
}

/// `QueuedFileEntry::pFileName` of a queued model (the getter `0045cd60`).
fn model_file_name(e: &mut Engine, this: Ptr<QueuedModel>) -> u32 {
    e.call(QUEUED_FILE_ENTRY_GET_FILE_NAME, &args![this]).u32()
}

/// The address of a queued model's `cFlags` byte, which the game passes to
/// the flag helpers.
fn model_flags_pointer(model: Ptr<QueuedModel>) -> Ptr {
    model.byte_add(QueuedModel::cFlags.off)
}

// Translated from 0043c890 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedModel` constructor taking a `TESModel` (no name in the engine map;
/// Xbox PDB `QueuedModel::QueuedModel(TESModel*, MEM_CONTEXT, ENUM_LOD_MULT,
/// bool, bool)`): the base constructor with `context`, the model's virtual
/// table, no model, `tes_model`, `lod_fade_mult`, no flags; the file name is
/// the model name (virtual function `0x14` of the `TESModel`), the file
/// entry is looked up (flag 0), and the last two arguments set bits 1 and 2
/// of `cFlags`. The overridden visual distance starts at 0. Returns `this`.
///
/// The exception-unwinding frame is not translated.
pub fn fn_0043c890(
    e: &mut Engine,
    this: Ptr<QueuedModel>,
    tes_model: Ptr,
    context: u32,
    lod_fade_mult: u32,
    first_flag: u8,
    second_flag: u8,
) -> Ptr<QueuedModel> {
    e.call(QUEUED_FILE_ENTRY_CONSTRUCT, &args![this, context]);
    e.mem.set_u32(this.addr(), QUEUED_MODEL_VTABLE);
    let model = model_pointer(this);
    e.call(NI_POINTER_CONSTRUCT, &args![model, 0u32]);
    e.set(this, QueuedModel::pTESModel, tes_model);
    e.set(this, QueuedModel::eLODFadeMult, lod_fade_mult);
    e.set(this, QueuedModel::cFlags, 0);
    let name = e.vcall(tes_model.addr(), 0x14, &args![]).u32();
    e.call(QUEUED_FILE_ENTRY_SET_FILE_NAME, &args![this, name]);
    e.call(QUEUED_FILE_ENTRY_FIND_FILE_ENTRY, &args![this, 0u32]);
    fn_0043c7a0(e, this, first_flag);
    fn_0043c7d0(e, this, second_flag);
    e.set(this, QueuedModel::mfOverriddenVisualDistance, 0.0);
    this
}

// Translated from 0043c960 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedModel` constructor taking a loaded model (no name in the engine
/// map): the base constructor with `context`, the model's virtual table, no
/// `TESModel`, no fade multiplier, no flags, then `model` is stored in
/// `spModel` and the task is put into state done. Returns `this`.
///
/// The exception-unwinding frame is not translated.
pub fn fn_0043c960(
    e: &mut Engine,
    this: Ptr<QueuedModel>,
    model: Ptr,
    context: u32,
) -> Ptr<QueuedModel> {
    e.call(QUEUED_FILE_ENTRY_CONSTRUCT, &args![this, context]);
    e.mem.set_u32(this.addr(), QUEUED_MODEL_VTABLE);
    let slot = model_pointer(this);
    e.call(NI_POINTER_CONSTRUCT, &args![slot, 0u32]);
    e.set(this, QueuedModel::pTESModel, Ptr::NULL);
    e.set(this, QueuedModel::eLODFadeMult, 0);
    e.set(this, QueuedModel::cFlags, 0);
    e.call(MODEL_POINTER_ASSIGN, &args![slot, model]);
    e.call(TASK_SET_DONE, &args![this]);
    e.set(this, QueuedModel::mfOverriddenVisualDistance, 0.0);
    this
}

// Translated from 0043ca10 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedModel::GenerateKey` (Xbox PDB): `QueuedFileEntry::GenerateKey`.
pub fn queued_model_generate_key(e: &mut Engine, this: Ptr<QueuedModel>) {
    e.call(QUEUED_FILE_ENTRY_GENERATE_KEY, &args![this]);
}

// Translated from 0043ca30 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedModel::CheckFinished` (Xbox PDB): a model whose bit 4 is not set
/// takes the plain `QueuedFile::CheckFinished`. With bit 4 set it does
/// nothing while the children are not all finished
/// ([`fn_0043caa0`]); when they are, a task still in state 0 is handed to
/// the task queue (slot `0x48`), any other goes through
/// `QueuedFile::CheckFinished`.
pub fn queued_model_check_finished(e: &mut Engine, this: Ptr<QueuedModel>) {
    if !fn_0043caf0(e, this) {
        e.call(QUEUED_FILE_CHECK_FINISHED, &args![this]);
    } else if fn_0043caa0(e, this) {
        if e.call(STATE_IS_ZERO, &args![this]).bool() {
            let queue = e.global::<u32>(TASK_QUEUE);
            e.vcall(queue, 0x48, &args![this]);
        } else {
            e.call(QUEUED_FILE_CHECK_FINISHED, &args![this]);
        }
    }
}

// Translated from 0043caa0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether all children are finished: true without a `pChildren` object,
/// otherwise the child count (dword at +8) equals the finished count (dword
/// at +0x10).
pub fn fn_0043caa0(e: &mut Engine, this: Ptr<QueuedModel>) -> bool {
    let children = e.get(this, QueuedModel::pChildren);
    if children.is_null() {
        return true;
    }
    let total = e.call(FIELD_AT_8, &args![children]).u32();
    let finished = e.call(FINISHED_COUNT_GETTER, &args![children]).u32();
    total == finished
}

// Translated from 0043caf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether bit 4 of the model's `cFlags` is set.
pub fn fn_0043caf0(e: &mut Engine, this: Ptr<QueuedModel>) -> bool {
    let flags = e.get(this, QueuedModel::cFlags);
    e.call(TEST_FLAG_BIT_4, &args![flags as u32]).bool()
}

// Translated from 0043cb10 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedModel::QueueMe` (Xbox PDB): calls the virtual function `0x30`
/// (`QueueMe_ov2`) with the texture block of the `TESModel` (0 without one).
pub fn queued_model_queue_me(e: &mut Engine, this: Ptr<QueuedModel>) {
    let mut block = 0;
    let tes_model = e.get(this, QueuedModel::pTESModel);
    if !tes_model.is_null() {
        block = e.call(TES_MODEL_TEXTURE_BLOCK, &args![tes_model]).u32();
    }
    e.vcall(this.addr(), 0x30, &args![block]);
}

// Translated from 0043cb50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedModel::QueueMe_ov2` (Xbox PDB): queues the textures of the model.
/// With a texture block (`block`: a count byte, then a pointer to that many
/// texture entries) every non-null entry goes to the loader's
/// `QueueTexture_ov2` with this model's key and as the parent. Then every
/// texture set of the `TESModel`'s list (virtual function `0x1c` of the
/// `TESModel`) that has an item is queued with `QueueTextureSet`. Last, bit
/// 4 of `cFlags` is set and the virtual function `0x28` (`CheckFinished`) is
/// called.
pub fn queued_model_queue_me_ov2(e: &mut Engine, this: Ptr<QueuedModel>, block: u32) {
    if block != 0 {
        let count = e.call(BYTE_AT_0, &args![block]).u8() as u32;
        if count != 0 {
            let textures = e.call(FIELD_AT_4, &args![block]).u32();
            for index in 0..count {
                let texture = e.mem.u32(textures + index * 4);
                if texture != 0 {
                    let key = fn_0043cc60(e, this.cast());
                    let owner = e.global::<u32>(FILE_MAP_OWNER);
                    e.call(
                        MODEL_LOADER_QUEUE_TEXTURE,
                        &args![owner, texture, key as u32, this],
                    );
                }
            }
        }
    }
    let tes_model = e.get(this, QueuedModel::pTESModel);
    if !tes_model.is_null() {
        let list = e.vcall(tes_model.addr(), 0x1c, &args![]).u32();
        if list != 0 {
            let mut node = e.call(LIST_FIRST_NODE, &args![list]).u32();
            while node != 0 {
                let item_slot = e.call(LIST_NODE_ITEM_ADDRESS, &args![node]).u32();
                let texture_set = e.mem.u32(item_slot);
                if texture_set != 0 && e.mem.u32(texture_set) != 0 {
                    let key = fn_0043cc60(e, this.cast());
                    let target = e.mem.u32(texture_set);
                    e.call(QUEUE_TEXTURE_SET, &args![target, key as u32, this, 0u32]);
                }
                node = e.call(FIELD_AT_4, &args![node]).u32();
            }
        }
    }
    fn_0043ccc0(e, this, 1);
    e.vcall(this.addr(), 0x28, &args![]);
}

// Translated from 0043cc60 (decompiled, FalloutNV.exe 1.4.0.525)
/// The priority byte of a task's key: the low byte of [`fn_0043cc80`].
pub fn fn_0043cc60(e: &mut Engine, this: Ptr<IOTask>) -> u8 {
    fn_0043cc80(e, this) as u8
}

// Translated from 0043cc80 (decompiled, FalloutNV.exe 1.4.0.525)
/// The task's `Key` shifted right by 16 ([`fn_0043cca0`] on `this + 0x10`).
pub fn fn_0043cc80(e: &mut Engine, this: Ptr<IOTask>) -> u64 {
    fn_0043cca0(e, this.byte_add(IOTask::Key.off))
}

// Translated from 0043cca0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `__cdecl(pointer)`: the 64-bit value at `value` shifted right by 16 with
/// the sign kept (`__allshr`), returned in `EDX:EAX`.
pub fn fn_0043cca0(e: &mut Engine, value: Ptr) -> u64 {
    ((e.mem.u64(value.addr()) as i64) >> 16) as u64
}

// Translated from 0043ccc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets (`flag` non-zero) or clears bit 4 of the model's `cFlags`.
pub fn fn_0043ccc0(e: &mut Engine, this: Ptr<QueuedModel>, flag: u8) {
    let flags = model_flags_pointer(this);
    e.call(SET_FLAG_BIT_4, &args![flag as u32, flags]);
}

// Translated from 0043ccf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedModel::Run` (Xbox PDB): inside the memory context of the task, a
/// model with a file name is loaded: if the loader already has a model of
/// that name it is taken (and bit 8 set); otherwise the file is opened
/// (`QueuedFileEntry::GetFile`) and read into a `BSStream` on the stack, and
/// a `Model` built from it is stored in `spModel` (when the file could be
/// read, or bit 0x10 allows a model without one); a failure is logged. The
/// name is published in `011f94c0` while the stream loads.
///
/// The exception-unwinding frame and the stack-cookie check are not
/// translated; the `BSStream` of the game's stack frame is a heap block here.
pub fn queued_model_run(e: &mut Engine, this: Ptr<QueuedModel>) {
    let context = e.get(this, QueuedModel::eContext);
    // The scope guard of the game's stack frame (4 bytes).
    e.with_stack(4, |e, guard| {
        e.call(
            MEMORY_CONTEXT_ENTER,
            &args![
                guard,
                context,
                1u32,
                MODEL_LOADER_SOURCE,
                MODEL_RUN_SOURCE_LINE
            ],
        );
        queued_model_run_in_context(e, this);
        e.call(MEMORY_CONTEXT_LEAVE, &args![guard]);
    });
}

/// The body of [`queued_model_run`] inside its memory context.
fn queued_model_run_in_context(e: &mut Engine, this: Ptr<QueuedModel>) {
    let name = model_file_name(e, this);
    if e.call(STRLEN, &args![name]).u32() == 0 {
        return;
    }
    let slot = model_pointer(this);
    let name = model_file_name(e, this);
    let owner = e.global::<u32>(FILE_MAP_OWNER);
    e.call(MODEL_LOADER_FIND_MODEL, &args![owner, name, slot]);
    if !ni_pointer_get(e, slot).is_null() {
        fn_0043cf60(e, this, 1);
        return;
    }

    let file: Ptr = e
        .call(QUEUED_FILE_ENTRY_GET_FILE, &args![this, 0u32, 1u32])
        .ptr();
    e.with_stack(BSStream::SIZE, |e, stream| {
        fn_0043cfd0(e, stream.cast());
        let name = model_file_name(e, this);
        e.set_global(LOADING_FILE_NAME, name);
        let mut create = false;
        if !file.is_null() {
            let name = model_file_name(e, this);
            create = e.call(BS_STREAM_LOAD, &args![stream, name, file]).bool();
        }
        if create || fn_0043cf90(e, this) {
            let block = e.call(MEMORY_ALLOC, &args![0x10u32]).u32();
            let mut model = 0;
            if block != 0 {
                let force = fn_0043cfb0(e, this);
                let prepare = fn_0043cf40(e, this);
                let name = model_file_name(e, this);
                model = model_model(e, Ptr::new(block), name, stream, prepare as u8, force as u8)
                    .addr();
            }
            e.call(MODEL_POINTER_ASSIGN, &args![slot, model]);
        } else {
            let verb = if file.is_null() { FIND_WORD } else { LOAD_WORD };
            let name = model_file_name(e, this);
            e.call(LOG, &args![MODEL_ERROR_MESSAGE, verb, name]);
        }
        e.set_global(LOADING_FILE_NAME, 0u32);
        bs_stream_destructor(e, stream.cast());
    });
}

// Translated from 0043cf40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether bit 1 of the model's `cFlags` is set.
pub fn fn_0043cf40(e: &mut Engine, this: Ptr<QueuedModel>) -> bool {
    let flags = e.get(this, QueuedModel::cFlags);
    e.call(TEST_FLAG_BIT_1, &args![flags as u32]).bool()
}

// Translated from 0043cf60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets (`flag` non-zero) or clears bit 8 of the model's `cFlags`.
pub fn fn_0043cf60(e: &mut Engine, this: Ptr<QueuedModel>, flag: u8) {
    let flags = model_flags_pointer(this);
    e.call(SET_FLAG_BIT_8, &args![flag as u32, flags]);
}

// Translated from 0043cf90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether bit 0x10 of the model's `cFlags` is set.
pub fn fn_0043cf90(e: &mut Engine, this: Ptr<QueuedModel>) -> bool {
    let flags = e.get(this, QueuedModel::cFlags);
    e.call(TEST_FLAG_BIT_10, &args![flags as u32]).bool()
}

// Translated from 0043cfb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether bit 0x20 of the model's `cFlags` is set.
pub fn fn_0043cfb0(e: &mut Engine, this: Ptr<QueuedModel>) -> bool {
    let flags = e.get(this, QueuedModel::cFlags);
    e.call(TEST_FLAG_BIT_20, &args![flags as u32]).bool()
}

// Translated from 0043cfd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSStream::BSStream` (the engine map has no name; Ghidra's library match
/// calls it `CMiniDockFrameWnd`'s, which it is not): the `NiStream`
/// constructor, the stream's virtual table, no node references and no
/// object reference map. Returns `this`.
///
/// The exception-unwinding frame is not translated.
pub fn fn_0043cfd0(e: &mut Engine, this: Ptr<BSStream>) -> Ptr<BSStream> {
    e.call(NI_STREAM_CONSTRUCT, &args![this]);
    e.mem.set_u32(this.addr(), BS_STREAM_VTABLE);
    let references = this.byte_add(BSStream::spNodeReferences.off);
    e.call(NI_POINTER_CONSTRUCT, &args![references, 0u32]);
    e.set(this, BSStream::pObjectRefMap, Ptr::NULL);
    this
}

// Translated from 0043d050 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiStream::SaveLinkID` (Xbox PDB): asks the stream (virtual function
/// `0x2c`) for the link id of `object` and writes it with
/// `NiStreamSaveBinary<unsigned int>`.
pub fn ni_stream_save_link_id(e: &mut Engine, this: Ptr, object: Ptr) {
    let link_id = e.vcall(this.addr(), 0x2c, &args![object]).u32();
    // The local `unsigned int` of the game's stack frame.
    e.with_stack(4, |e, value| {
        e.mem.set_u32(value.addr(), link_id);
        e.call(NI_STREAM_SAVE_BINARY_U32, &args![this, value]);
    });
}

// Translated from 0043d090 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSStream::~BSStream` (Xbox PDB): restores the stream's virtual table,
/// deletes the object reference map ([`fn_0043d100`]), destroys the node
/// references pointer and runs the `NiStream` destructor body.
///
/// The exception-unwinding frame is not translated.
pub fn bs_stream_destructor(e: &mut Engine, this: Ptr<BSStream>) {
    e.mem.set_u32(this.addr(), BS_STREAM_VTABLE);
    fn_0043d100(e, this);
    let references = this.byte_add(BSStream::spNodeReferences.off);
    e.call(NI_POINTER_DESTRUCT, &args![references]);
    e.call(NI_STREAM_DESTRUCT, &args![this]);
}

// Translated from 0043d100 (decompiled, FalloutNV.exe 1.4.0.525)
/// Deletes the stream's object reference map (virtual destructor, flag 1)
/// when there is one and clears the pointer.
pub fn fn_0043d100(e: &mut Engine, this: Ptr<BSStream>) {
    let map = e.get(this, BSStream::pObjectRefMap);
    if !map.is_null() {
        e.vcall(map.addr(), 0, &args![1u32]);
    }
    e.set(this, BSStream::pObjectRefMap, Ptr::NULL);
}

// Translated from 0043d150 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSStream::_scalar_deleting_destructor_` (Xbox PDB): runs the destructor
/// and, when bit 0 of `flags` is set, frees the stream. Returns `this`.
pub fn bs_stream_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr<BSStream>,
    flags: u32,
) -> Ptr<BSStream> {
    bs_stream_destructor(e, this);
    if flags & 1 != 0 {
        e.call(MEMORY_FREE, &args![this]);
    }
    this
}

// Translated from 0043d180 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedModel::Finish` (Xbox PDB): inside the memory context of the task,
/// for a loaded model: bit 2 adds a reference to the model; unless bit 8 is
/// set, the model's 3D object is updated and, when it is a fade node, given
/// its fade range and LOD multiplier type ([`fade_range`]); then the model
/// is added to the loader's map, and when the map already has one of that
/// name `spModel` is replaced by it. Last the task's virtual function `0x28`
/// (`CheckFinished`) is called.
///
/// The exception-unwinding frame is not translated.
pub fn queued_model_finish(e: &mut Engine, this: Ptr<QueuedModel>) {
    let context = e.get(this, QueuedModel::eContext);
    // The scope guard of the game's stack frame (4 bytes).
    e.with_stack(4, |e, guard| {
        e.call(
            MEMORY_CONTEXT_ENTER,
            &args![
                guard,
                context,
                1u32,
                MODEL_LOADER_SOURCE,
                MODEL_FINISH_SOURCE_LINE
            ],
        );
        queued_model_finish_in_context(e, this);
        e.call(MEMORY_CONTEXT_LEAVE, &args![guard]);
    });
}

/// The body of [`queued_model_finish`] inside its memory context.
fn queued_model_finish_in_context(e: &mut Engine, this: Ptr<QueuedModel>) {
    let slot = model_pointer(this);
    if !ni_pointer_get(e, slot).is_null() {
        if fn_0043d490(e, this) {
            let model = ni_pointer_get(e, slot);
            e.call(ADD_REFERENCE, &args![model]);
        }
        if !fn_0043d4b0(e, this) {
            let model = ni_pointer_get(e, slot);
            if !fn_0043b230(e, model).is_null() {
                let model = ni_pointer_get(e, slot);
                let object = fn_0043b230(e, model);
                // Virtual function `0x10` gives the object as a fade node
                // (0 for other classes).
                let fade_node: Ptr = e.vcall(object.addr(), 0x10, &args![]).ptr();
                if !fade_node.is_null() {
                    fade_range(e, this, fade_node);
                }
                let model = ni_pointer_get(e, slot);
                let name = model_file_name(e, this);
                let owner = e.global::<u32>(FILE_MAP_OWNER);
                let added = e
                    .call(MODEL_LOADER_ADD_MODEL, &args![owner, name, model])
                    .bool();
                if !added {
                    let name = model_file_name(e, this);
                    let owner = e.global::<u32>(FILE_MAP_OWNER);
                    e.call(MODEL_LOADER_FIND_MODEL, &args![owner, name, slot]);
                }
            }
        }
    }
    fn_0043c610(e, this.cast());
}

/// What `Finish` does with a model whose 3D object is a fade node: updates
/// the node, then sets its fade range and LOD multiplier type.
///
/// The radius is the world bound's radius, or when that is below 0.001 half
/// of the third component of the reference's maximum bound. The range is the
/// radius times the visual distance scale, unless the radius is 0 or the
/// world is too small for it: then the limit (half the world in units of
/// 4096 per cell, minus radius and 2048). A non-zero overridden visual
/// distance of the queued model replaces the range. The node gets the range
/// times the fade range scale and the range itself.
///
/// The floating-point steps follow the x87 code: values are computed in
/// `f64` and rounded to `f32` where the game stores a `float`.
fn fade_range(e: &mut Engine, this: Ptr<QueuedModel>, fade_node: Ptr) {
    // The update record of the game's stack frame (12 bytes).
    e.with_stack(NiUpdateData::SIZE, |e, update| {
        fn_0043d410(e, update.cast(), 0.0, 0, 0);
        e.call(NI_AV_OBJECT_UPDATE, &args![fade_node, update]);
    });
    let bound = ni_av_object_get_world_bound(e, fade_node);
    let mut radius = e.call(BOUND_RADIUS, &args![bound]).f32();
    if (radius as f64) < e.global::<f64>(SMALL_RADIUS) {
        let model = ni_pointer_get(e, model_pointer(this));
        let object = fn_0043b230(e, model);
        let depth = e.with_stack(12, |e, out| {
            let vector = e.call(GET_BOUND_MAX, &args![out, object]).u32();
            e.mem.f32(vector + 8)
        });
        radius = (depth as f64 * e.global::<f64>(HALF)) as f32;
    }
    // The game first stores `FLT_MAX` (`01016970`) into the limit; it is
    // overwritten at once.
    let cells_setting = fn_0043d4d0(e, Ptr::new(CELL_COUNT_SETTING));
    let cells = e.mem.u32(cells_setting.addr());
    let world_size = (cells << 12) as f64;
    let limit = (world_size * e.global::<f64>(HALF)
        - (radius as f64 + e.global::<f64>(LOADED_AREA_MARGIN))) as f32;
    let scale = e
        .call(SETTING_VALUE_POINTER, &args![VISUAL_DISTANCE_SCALE_SETTING])
        .u32();
    let scaled = (radius as f64 * e.mem.f32(scale) as f64) as f32;
    // `radius == 0`, `limit <= scaled` and unordered compares take the limit.
    let mut range = if radius != 0.0 && scaled < limit {
        scaled
    } else {
        limit
    };
    let overridden = e.call(OVERRIDDEN_VISUAL_DISTANCE, &args![this]).f32();
    if overridden as f64 != e.global::<f64>(ZERO) {
        range = e.call(OVERRIDDEN_VISUAL_DISTANCE, &args![this]).f32();
    }
    let scale = e
        .call(SETTING_VALUE_POINTER, &args![FADE_RANGE_SCALE_SETTING])
        .u32();
    let far = (range as f64 * e.mem.f32(scale) as f64) as f32;
    e.call(FADE_NODE_SET_RANGE, &args![fade_node, far, range]);
    let lod_fade_mult = e.get(this, QueuedModel::eLODFadeMult);
    e.call(
        FADE_NODE_SET_LOD_MULT_TYPE,
        &args![fade_node, lod_fade_mult],
    );
}

// Translated from 0043d410 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiUpdateData::NiUpdateData(float, bool, bool)` (no name in the engine
/// map): stores the time and the first two flags, clears the other three.
/// Returns `this`.
pub fn fn_0043d410(
    e: &mut Engine,
    this: Ptr<NiUpdateData>,
    time: f32,
    update_controllers: u8,
    parallel_update: u8,
) -> Ptr<NiUpdateData> {
    e.set(this, NiUpdateData::fTime, time);
    e.set(this, NiUpdateData::bUpdateControllers, update_controllers);
    e.set(this, NiUpdateData::bParallelUpdate, parallel_update);
    e.set(this, NiUpdateData::bFoundParticles, 0);
    e.set(this, NiUpdateData::bFoundMorphController, 0);
    e.set(this, NiUpdateData::bSceneGraphChange, 0);
    this
}

// Translated from 0043d450 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiAVObject::GetWorldBound` (Xbox PDB): the object's bound (the pointer
/// at +0x20), or the empty bound at `011f4288` when it has none.
pub fn ni_av_object_get_world_bound(e: &mut Engine, this: Ptr) -> Ptr {
    // NiAVObject::m_pkWorldBound (Xbox PDB) +0x20
    let bound = e.mem.u32(this.addr() + 0x20);
    if bound == 0 {
        Ptr::new(EMPTY_WORLD_BOUND)
    } else {
        Ptr::new(bound)
    }
}

// Translated from 0043d490 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether bit 2 of the model's `cFlags` is set.
pub fn fn_0043d490(e: &mut Engine, this: Ptr<QueuedModel>) -> bool {
    let flags = e.get(this, QueuedModel::cFlags);
    e.call(TEST_FLAG_BIT_2, &args![flags as u32]).bool()
}

// Translated from 0043d4b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether bit 8 of the model's `cFlags` is set.
pub fn fn_0043d4b0(e: &mut Engine, this: Ptr<QueuedModel>) -> bool {
    let flags = e.get(this, QueuedModel::cFlags);
    e.call(TEST_FLAG_BIT_8, &args![flags as u32]).bool()
}

// Translated from 0043d4d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Address of the value of a setting object (`this + 4`); without an object
/// (0) a scratch dword at `01202800` is zeroed and its address returned.
pub fn fn_0043d4d0(e: &mut Engine, this: Ptr) -> Ptr {
    if this.is_null() {
        e.set_global(SCRATCH_ZERO, 0u32);
        Ptr::new(SCRATCH_ZERO)
    } else {
        this.byte_add(4)
    }
}

// Translated from 0043d510 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedModel::GetDescription` (Xbox PDB): `QueuedFileEntry::GetDescription`
/// with the kind word `"model"`; returns its result.
pub fn queued_model_get_description(
    e: &mut Engine,
    this: Ptr<QueuedModel>,
    buffer: u32,
    size: u32,
) -> bool {
    e.call(
        QUEUED_FILE_ENTRY_GET_DESCRIPTION,
        &args![this, buffer, size, MODEL_WORD],
    )
    .bool()
}

// Translated from 0043d540 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedTreeBillboard` constructor (no name in the engine map): the
/// `QueuedTexture` constructor with `name` and `context`, the billboard's
/// virtual table and `data` as `pBillboardData`. Returns `this`.
pub fn fn_0043d540(
    e: &mut Engine,
    this: Ptr<QueuedTreeBillboard>,
    name: u32,
    context: u32,
    data: Ptr<TreeBillboardData>,
) -> Ptr<QueuedTreeBillboard> {
    fn_0043bd10(e, this.cast(), name, context);
    e.mem.set_u32(this.addr(), QUEUED_TREE_BILLBOARD_VTABLE);
    e.set(this, QueuedTreeBillboard::pBillboardData, data.cast());
    this
}

// Translated from 0043d580 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedTreeBillboard::_scalar_deleting_destructor_` (Xbox PDB): runs the
/// destructor ([`fn_0043d5b0`]) and, when bit 0 of `flags` is set, frees the
/// object. Returns `this`.
pub fn queued_tree_billboard_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr<QueuedTreeBillboard>,
    flags: u32,
) -> Ptr<QueuedTreeBillboard> {
    fn_0043d5b0(e, this);
    if flags & 1 != 0 {
        e.call(MEMORY_FREE, &args![this]);
    }
    this
}

// Translated from 0043d5b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedTreeBillboard` destructor (no name in the engine map): deletes the
/// billboard data (when there is some) and runs the `QueuedTexture`
/// destructor.
///
/// The exception-unwinding frame is not translated.
pub fn fn_0043d5b0(e: &mut Engine, this: Ptr<QueuedTreeBillboard>) {
    e.mem.set_u32(this.addr(), QUEUED_TREE_BILLBOARD_VTABLE);
    let data = e.get(this, QueuedTreeBillboard::pBillboardData);
    if !data.is_null() {
        fn_0043d640(e, data.cast(), 1);
    }
    fn_0043bf80(e, this.cast());
}

// Translated from 0043d640 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TREE_BILLBOARD_DATA` scalar deleting destructor (the decompiler names it
/// after an MFC class; the engine map has no name): runs the destructor body
/// (`0051d400`) and, when bit 0 of `flags` is set, frees the block. Returns
/// `this`.
pub fn fn_0043d640(
    e: &mut Engine,
    this: Ptr<TreeBillboardData>,
    flags: u32,
) -> Ptr<TreeBillboardData> {
    e.call(TREE_BILLBOARD_DATA_DESTRUCT, &args![this]);
    if flags & 1 != 0 {
        e.call(MEMORY_FREE, &args![this]);
    }
    this
}

// Translated from 0043d670 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedTreeBillboard::Finish` (Xbox PDB): `QueuedTexture::Finish`, then
/// the task is added to the post-process queue ([`iomanager_add_post_process_task`]).
pub fn queued_tree_billboard_finish(e: &mut Engine, this: Ptr<QueuedTreeBillboard>) {
    queued_texture_finish(e, this.cast());
    iomanager_add_post_process_task(e, this.cast());
}

// Translated from 0043d690 (decompiled, FalloutNV.exe 1.4.0.525)
/// `IOManager::AddPostProcessTask` (Xbox PDB): [`fn_0043d6b0`] on the IO
/// manager (the global at `01202d98`) with `task`.
pub fn iomanager_add_post_process_task(e: &mut Engine, task: Ptr<IOTask>) {
    let manager = e.global::<u32>(TASK_QUEUE);
    fn_0043d6b0(e, Ptr::new(manager), task);
}

// Translated from 0043d6b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The body of `IOManager::AddPostProcessTask` (no name in the engine map):
/// takes a counted reference to `task` and puts it into the queue its key
/// names (the priority byte, [`fn_0043cc80`]) in the table of queues at
/// `IOManager + 0x64` (a PC field, the Xbox class ends at 0x5c).
///
/// The exception-unwinding frame is not translated.
pub fn fn_0043d6b0(e: &mut Engine, this: Ptr, task: Ptr<IOTask>) {
    // The task pointer of the game's stack frame.
    e.with_stack(4, |e, holder| {
        e.call(TASK_POINTER_CONSTRUCT, &args![holder, task]);
        let priority = fn_0043cc80(e, task) as u8;
        // IOManager + 0x64: the table of queues (not in the Xbox layout).
        let queues = e.mem.u32(this.addr() + 0x64);
        e.call(QUEUE_TABLE_ADD, &args![queues, priority as u32, holder]);
        e.call(TASK_POINTER_DESTRUCT, &args![holder]);
    });
}

// Translated from 0043d730 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedTreeBillboard::PostProcess` (Xbox PDB): calls the virtual function
/// `0x30` (`CreateBillboard`).
pub fn queued_tree_billboard_post_process(e: &mut Engine, this: Ptr<QueuedTreeBillboard>) {
    e.vcall(this.addr(), 0x30, &args![]);
}

// Translated from 0043d750 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedTreeBillboard::CreateBillboard` (Xbox PDB): builds the distant 3D
/// of the tree (`BuildDistant3D(1)`); with it, a three-word record on the
/// stack (0, the node, the count read from the tree) is passed with the
/// billboard data to the instance builder (`00b59ae0`), and the node is
/// deleted again (virtual destructor, flag 1).
pub fn queued_tree_billboard_create_billboard(e: &mut Engine, this: Ptr<QueuedTreeBillboard>) {
    let data: Ptr<TreeBillboardData> = e.get(this, QueuedTreeBillboard::pBillboardData).cast();
    let tree = e.get(data, TreeBillboardData::pTree);
    let node = e.call(BUILD_DISTANT_3D, &args![tree, 1u32]).u32();
    if node == 0 {
        return;
    }
    // The record of the game's stack frame: +0 zero, +4 the node, +8 the count.
    e.with_stack(12, |e, record| {
        e.call(ZERO_THREE_WORDS, &args![record]);
        let tree = e.get(data, TreeBillboardData::pTree);
        let count = e.call(ARRAY_COUNT, &args![tree]).u32();
        e.mem.set_u32(record.addr() + 8, count);
        e.mem.set_u32(record.addr() + 4, node);
        e.mem.set_u32(record.addr(), 0);
        let size = e.get(data, TreeBillboardData::iArraySize) & 0xffff;
        let chunk = e.get(data, TreeBillboardData::iCellChunk);
        let key = e.get(data, TreeBillboardData::iCellKey);
        let instanced = e.get(data, TreeBillboardData::pInstancedNode);
        let locations = e.get(data, TreeBillboardData::pLocArray);
        let colors = e.get(data, TreeBillboardData::pColorArray);
        e.call(
            CREATE_BILLBOARD_INSTANCES,
            &args![chunk, key, instanced, record, locations, colors, size],
        );
        let node = e.mem.u32(record.addr() + 4);
        if node != 0 {
            e.vcall(node, 0, &args![1u32]);
        }
    });
}

// Translated from 0043d820 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedTreeBillboard::GetDescription` (Xbox PDB):
/// `QueuedFileEntry::GetDescription` with the kind word `"Tree Billboard"`;
/// returns its result.
pub fn queued_tree_billboard_get_description(
    e: &mut Engine,
    this: Ptr<QueuedTreeBillboard>,
    buffer: u32,
    size: u32,
) -> bool {
    e.call(
        QUEUED_FILE_ENTRY_GET_DESCRIPTION,
        &args![this, buffer, size, TREE_BILLBOARD_WORD],
    )
    .bool()
}

// Translated from 0043d850 (decompiled, FalloutNV.exe 1.4.0.525)
/// `QueuedTreeModel` constructor (no name in the engine map): the
/// `QueuedModel` constructor over the `TESModel` embedded in the tree (at
/// +0x30; none for a null tree) with `context`, `lod_fade_mult`, bit 1 set
/// and bit 2 clear; then the tree model's virtual table, `pRef`, `pTree` and
/// the context `0x1e`. Returns `this`.
pub fn fn_0043d850(
    e: &mut Engine,
    this: Ptr<QueuedTreeModel>,
    reference: Ptr,
    tree: Ptr,
    context: u32,
    lod_fade_mult: u32,
) -> Ptr<QueuedTreeModel> {
    // TESObjectTREE::TESModel base (Xbox PDB) +0x30
    let tes_model = if tree.is_null() {
        Ptr::NULL
    } else {
        tree.byte_add(0x30)
    };
    fn_0043c890(e, this.cast(), tes_model, context, lod_fade_mult, 1, 0);
    e.mem.set_u32(this.addr(), QUEUED_TREE_MODEL_VTABLE);
    e.set(this, QueuedTreeModel::pRef, reference);
    e.set(this, QueuedTreeModel::pTree, tree);
    e.set(this, QueuedTreeModel::eContext, TREE_MODEL_CONTEXT);
    this
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(
            0x0043aaf0,
            model_model(Ptr<Model>, u32, Ptr, u8, u8) -> Ptr<Model>
        ),
        entry!(0x0043ab70, model_destructor(Ptr<Model>)),
        entry!(0x0043abf0, fn_0043abf0(Ptr<Model>, u32, Ptr) -> Ptr<Model>),
        entry!(0x0043acb0, fn_0043acb0(Ptr<Model>)),
        entry!(0x0043ace0, model_init_model(Ptr<Model>, u32, Ptr, u8, u8)),
        entry!(0x0043b1b0, fn_0043b1b0(Ptr) -> u32),
        entry!(0x0043b1d0, fn_0043b1d0(Ptr) -> u32),
        entry!(0x0043b1e0, fn_0043b1e0(Ptr) -> u32),
        entry!(0x0043b200, fn_0043b200(Ptr, u32) -> Ptr),
        entry!(0x0043b230, fn_0043b230(Ptr) -> Ptr),
        entry!(0x0043b250, fn_0043b250() -> Ptr),
        entry!(0x0043b260, fn_0043b260(Ptr, Ptr) -> bool),
        entry!(0x0043b290, fn_0043b290() -> u32),
        entry!(0x0043b2a0, fn_0043b2a0() -> Ptr),
        entry!(0x0043b2b0, message_handler_inc_disable_warning_count(u8)),
        entry!(0x0043b300, fn_0043b300(u32, Ptr) -> bool),
        entry!(0x0043b320, fn_0043b320(Ptr, u16, Ptr)),
        entry!(0x0043b350, fn_0043b350(Ptr, u8)),
        entry!(0x0043b370, fn_0043b370(Ptr, u8, u32)),
        entry!(0x0043b3b0, fn_0043b3b0(Ptr, Ptr) -> u16),
        entry!(0x0043b410, model_mod_manual_ref_count(Ptr<Model>, i32)),
        entry!(0x0043b460, fn_0043b460(Ptr, i32, i32) -> i32),
        entry!(0x0043b480, fn_0043b480(Ptr) -> u16),
        entry!(0x0043b4a0, fn_0043b4a0(Ptr, u32) -> Ptr),
        entry!(0x0043b4d0, fn_0043b4d0(Ptr) -> u32),
        entry!(0x0043b4f0, fn_0043b4f0(Ptr, Ptr) -> Ptr),
        entry!(0x0043b540, fn_0043b540(Ptr) -> u32),
        entry!(0x0043b560, fn_0043b560(Ptr) -> u32),
        entry!(0x0043b5b0, fn_0043b5b0(Ptr)),
        entry!(0x0043b5d0, fn_0043b5d0(Ptr) -> Ptr),
        entry!(0x0043b5f0, fn_0043b5f0(Ptr)),
        entry!(
            0x0043b610,
            bhk_collision_object_get_bhk_collision_object(Ptr) -> Ptr
        ),
        entry!(
            0x0043b640,
            kf_model_kf_model(Ptr<KFModel>, u32, Ptr) -> Ptr<KFModel>
        ),
        entry!(0x0043b750, fn_0043b750(Ptr<KFModel>)),
        entry!(0x0043b7e0, fn_0043b7e0(Ptr<KFModel>, u32)),
        entry!(0x0043b9d0, fn_0043b9d0(u32, i32) -> Ptr),
        entry!(
            0x0043b9f0,
            fn_0043b9f0(Ptr, Ptr<NiFixedString>) -> Ptr<NiFixedString>
        ),
        entry!(
            0x0043ba10,
            fn_0043ba10(Ptr<NiFixedString>, Ptr<NiFixedString>) -> Ptr<NiFixedString>
        ),
        entry!(0x0043ba60, fn_0043ba60(Ptr)),
        entry!(0x0043baa0, fn_0043baa0(Ptr, Ptr) -> Ptr),
        entry!(0x0043bac0, fn_0043bac0(Ptr<QueuedChildren>)),
        entry!(
            0x0043baf0,
            fn_0043baf0(Ptr<LoadedFile>, u32, Ptr) -> Ptr<LoadedFile>
        ),
        entry!(0x0043bb80, loaded_file_destructor(Ptr<LoadedFile>)),
        entry!(0x0043bce0, fn_0043bce0() -> bool),
        entry!(0x0043bd00, fn_0043bd00() -> u32),
        entry!(
            0x0043bd10,
            fn_0043bd10(Ptr<QueuedTexture>, u32, u32) -> Ptr<QueuedTexture>
        ),
        entry!(
            0x0043bde0,
            queued_file_notify_child_finished(Ptr<QueuedFile>, Ptr)
        ),
        entry!(0x0043be10, fn_0043be10(Ptr<QueuedChildren>)),
        entry!(
            0x0043be30,
            queued_texture_scalar_deleting_destructor(
                Ptr<QueuedTexture>,
                u32,
            ) -> Ptr<QueuedTexture>
        ),
        entry!(
            0x0043be60,
            fn_0043be60(Ptr<QueuedTexture>, Ptr, u32) -> Ptr<QueuedTexture>
        ),
        entry!(
            0x0043bef0,
            fn_0043bef0(Ptr<QueuedTexture>, Ptr, u32) -> Ptr<QueuedTexture>
        ),
        entry!(0x0043bf80, fn_0043bf80(Ptr<QueuedTexture>)),
        entry!(
            0x0043bfe0,
            queued_texture_get_file_index(Ptr<QueuedTexture>) -> u32
        ),
        entry!(0x0043c030, fn_0043c030(Ptr) -> u32),
        entry!(0x0043c050, queued_texture_queue_me(Ptr<QueuedTexture>)),
        entry!(0x0043c0d0, fn_0043c0d0(Ptr<QueuedTexture>, u8)),
        entry!(0x0043c100, fn_0043c100(Ptr<QueuedTexture>, u8)),
        entry!(0x0043c130, fn_0043c130() -> u8),
        entry!(0x0043c150, queued_texture_run(Ptr<QueuedTexture>)),
        entry!(0x0043c3b0, fn_0043c3b0(Ptr) -> u32),
        entry!(0x0043c3d0, fn_0043c3d0(Ptr<QueuedTexture>) -> bool),
        entry!(0x0043c3f0, fn_0043c3f0(u32)),
        entry!(0x0043c410, fn_0043c410() -> u32),
        entry!(0x0043c430, fn_0043c430(Ptr) -> u32),
        entry!(0x0043c470, fn_0043c470(Ptr) -> u8),
        entry!(0x0043c490, fn_0043c490(Ptr) -> Ptr),
        entry!(0x0043c4b0, fn_0043c4b0() -> u32),
        entry!(0x0043c4c0, bs_texture_palette_get_texture(u32, Ptr)),
        entry!(0x0043c4f0, fn_0043c4f0(Ptr, u32, Ptr) -> bool),
        entry!(0x0043c530, fn_0043c530(u32) -> u32),
        entry!(0x0043c550, queued_texture_finish(Ptr<QueuedTexture>)),
        entry!(0x0043c610, fn_0043c610(Ptr<QueuedTexture>)),
        entry!(0x0043c630, fn_0043c630(Ptr<QueuedTexture>) -> bool),
        entry!(
            0x0043c650,
            queued_texture_cancel(Ptr<QueuedTexture>, u32, u32)
        ),
        entry!(
            0x0043c6b0,
            queued_texture_get_description(Ptr<QueuedTexture>, u32, u32) -> bool
        ),
        entry!(
            0x0043c6e0,
            fn_0043c6e0(Ptr<QueuedModel>, u32, u32, u32, u8, u8) -> Ptr<QueuedModel>
        ),
        entry!(0x0043c7a0, fn_0043c7a0(Ptr<QueuedModel>, u8)),
        entry!(0x0043c7d0, fn_0043c7d0(Ptr<QueuedModel>, u8)),
        entry!(
            0x0043c800,
            queued_model_scalar_deleting_destructor(Ptr<QueuedModel>, u32) -> Ptr<QueuedModel>
        ),
        entry!(0x0043c830, fn_0043c830(Ptr<QueuedModel>)),
        entry!(
            0x0043c890,
            fn_0043c890(Ptr<QueuedModel>, Ptr, u32, u32, u8, u8) -> Ptr<QueuedModel>
        ),
        entry!(
            0x0043c960,
            fn_0043c960(Ptr<QueuedModel>, Ptr, u32) -> Ptr<QueuedModel>
        ),
        entry!(0x0043ca10, queued_model_generate_key(Ptr<QueuedModel>)),
        entry!(0x0043ca30, queued_model_check_finished(Ptr<QueuedModel>)),
        entry!(0x0043caa0, fn_0043caa0(Ptr<QueuedModel>) -> bool),
        entry!(0x0043caf0, fn_0043caf0(Ptr<QueuedModel>) -> bool),
        entry!(0x0043cb10, queued_model_queue_me(Ptr<QueuedModel>)),
        entry!(0x0043cb50, queued_model_queue_me_ov2(Ptr<QueuedModel>, u32)),
        entry!(0x0043cc60, fn_0043cc60(Ptr<IOTask>) -> u8),
        entry!(0x0043cc80, fn_0043cc80(Ptr<IOTask>) -> u64),
        entry!(0x0043cca0, fn_0043cca0(Ptr) -> u64),
        entry!(0x0043ccc0, fn_0043ccc0(Ptr<QueuedModel>, u8)),
        entry!(0x0043ccf0, queued_model_run(Ptr<QueuedModel>)),
        entry!(0x0043cf40, fn_0043cf40(Ptr<QueuedModel>) -> bool),
        entry!(0x0043cf60, fn_0043cf60(Ptr<QueuedModel>, u8)),
        entry!(0x0043cf90, fn_0043cf90(Ptr<QueuedModel>) -> bool),
        entry!(0x0043cfb0, fn_0043cfb0(Ptr<QueuedModel>) -> bool),
        entry!(0x0043cfd0, fn_0043cfd0(Ptr<BSStream>) -> Ptr<BSStream>),
        entry!(0x0043d050, ni_stream_save_link_id(Ptr, Ptr)),
        entry!(0x0043d090, bs_stream_destructor(Ptr<BSStream>)),
        entry!(0x0043d100, fn_0043d100(Ptr<BSStream>)),
        entry!(
            0x0043d150,
            bs_stream_scalar_deleting_destructor(Ptr<BSStream>, u32) -> Ptr<BSStream>
        ),
        entry!(0x0043d180, queued_model_finish(Ptr<QueuedModel>)),
        entry!(
            0x0043d410,
            fn_0043d410(Ptr<NiUpdateData>, f32, u8, u8) -> Ptr<NiUpdateData>
        ),
        entry!(0x0043d450, ni_av_object_get_world_bound(Ptr) -> Ptr),
        entry!(0x0043d490, fn_0043d490(Ptr<QueuedModel>) -> bool),
        entry!(0x0043d4b0, fn_0043d4b0(Ptr<QueuedModel>) -> bool),
        entry!(0x0043d4d0, fn_0043d4d0(Ptr) -> Ptr),
        entry!(
            0x0043d510,
            queued_model_get_description(Ptr<QueuedModel>, u32, u32) -> bool
        ),
        entry!(
            0x0043d540,
            fn_0043d540(
                Ptr<QueuedTreeBillboard>,
                u32,
                u32,
                Ptr<TreeBillboardData>,
            ) -> Ptr<QueuedTreeBillboard>
        ),
        entry!(
            0x0043d580,
            queued_tree_billboard_scalar_deleting_destructor(
                Ptr<QueuedTreeBillboard>,
                u32,
            )
                -> Ptr<QueuedTreeBillboard>
        ),
        entry!(0x0043d5b0, fn_0043d5b0(Ptr<QueuedTreeBillboard>)),
        entry!(
            0x0043d640,
            fn_0043d640(Ptr<TreeBillboardData>, u32) -> Ptr<TreeBillboardData>
        ),
        entry!(
            0x0043d670,
            queued_tree_billboard_finish(Ptr<QueuedTreeBillboard>)
        ),
        entry!(0x0043d690, iomanager_add_post_process_task(Ptr<IOTask>)),
        entry!(0x0043d6b0, fn_0043d6b0(Ptr, Ptr<IOTask>)),
        entry!(
            0x0043d730,
            queued_tree_billboard_post_process(Ptr<QueuedTreeBillboard>)
        ),
        entry!(
            0x0043d750,
            queued_tree_billboard_create_billboard(Ptr<QueuedTreeBillboard>)
        ),
        entry!(
            0x0043d820,
            queued_tree_billboard_get_description(Ptr<QueuedTreeBillboard>, u32, u32) -> bool
        ),
        entry!(
            0x0043d850,
            fn_0043d850(Ptr<QueuedTreeModel>, Ptr, Ptr, u32, u32) -> Ptr<QueuedTreeModel>
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A virtual function (any slot) that returns 1, and one that returns 0.
    const ANSWERS_YES: u32 = 0x0ff0_0001;
    const ANSWERS_NO: u32 = 0x0ff0_0000;
    /// Slot 0xF8 of the wrapper node (`SetAt`): does nothing but is logged.
    const SET_AT: u32 = 0x0ff0_0010;
    /// Slot 0x90 of the object `InitModel` looks up in the editor branch.
    const EDITOR_SLOT: u32 = 0x0ff0_0020;
    /// A stream's loader slot (0x60) that gives the stream one node.
    const STREAM_LOADS_A_NODE: u32 = 0x0ff0_0030;
    const NODE_VTABLE: u32 = 0x0ff1_0000;
    const GEOMETRY_VTABLE: u32 = 0x0ff1_1000;
    const PLAIN_VTABLE: u32 = 0x0ff1_2000;
    const STREAM_NO_VTABLE: u32 = 0x0ff1_4000;
    const WRAPPER_VTABLE: u32 = 0x0ff1_5000;
    const EDITOR_VTABLE: u32 = 0x0ff1_6000;
    const STREAM_LOADING_VTABLE: u32 = 0x0ff1_7000;
    /// The property type `00702440` returns in the tests.
    const SECOND_TYPE: u32 = 0x20;
    /// The pooled string's reference counter in the tests.
    const STRING_COUNTER: u32 = 0x0118_5700;

    /// An engine with test doubles for the callees every test may use: the
    /// `NiPointer` functions, the memory manager, string helpers, the stream
    /// array getters, interlocked counters, and the pages of the globals the
    /// unit reads.
    fn loader_engine() -> Engine {
        let mut e = Engine::new();
        for page in [
            0x0109_6000,
            0x0109_b000,
            0x0118_5000,
            0x011f_4000,
            0x0120_2000,
        ] {
            e.map(page, 0x1000);
        }
        e.register(NI_POINTER_GET, |e, a| e.mem.u32(a[0]).into_ret());
        e.register(NI_POINTER_ASSIGN, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            a[0].into_ret()
        });
        e.register(NI_POINTER_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            a[0].into_ret()
        });
        e.register(NI_POINTER_DESTRUCT, |_, _| Ret::default());
        e.register(MEMORY_ALLOC, |e, a| e.mem.alloc(a[0]).into_ret());
        e.register(MEMORY_FREE, |e, a| {
            e.mem.free(a[0]);
            Ret::default()
        });
        e.register(STRLEN, |e, a| (e.mem.cstr(a[0]).len() as u32).into_ret());
        e.register(STRING_COPY, |e, a| {
            let text = e.mem.cstr(a[2]);
            e.mem.set_cstr(a[0], &text);
            Ret::default()
        });
        e.register(MEMORY_COPY, |e, a| {
            let bytes = e.mem.bytes(a[1], a[2]);
            e.mem.write(a[0], &bytes);
            Ret::default()
        });
        e.register(STRCHR, |e, a| {
            let text = e.mem.cstr(a[0]);
            match text.iter().position(|c| *c as u32 == a[1]) {
                Some(i) => (a[0] + i as u32).into_ret(),
                None => 0u32.into_ret(),
            }
        });
        e.register(ARRAY_ELEMENT_ADDRESS, |e, a| {
            (e.mem.u32(a[0] + 4) + a[1] * 4).into_ret()
        });
        e.register(ARRAY_COUNT, |e, a| e.mem.u32(a[0] + 0xc).into_ret());
        e.register(ARRAY_SIZE, |e, a| e.mem.u16(a[0] + 0xa).into_ret());
        e.register(NAME_SLOT, |_, a| (a[0] + 8).into_ret());
        e.register(INTERLOCKED_INCREMENT, |e, a| {
            let value = e.mem.i32(a[0]) + 1;
            e.mem.set_i32(a[0], value);
            value.into_ret()
        });
        e.register(INTERLOCKED_DECREMENT, |e, a| {
            let value = e.mem.i32(a[0]) - 1;
            e.mem.set_i32(a[0], value);
            value.into_ret()
        });
        e.register(INTERLOCKED_COMPARE_EXCHANGE, |e, a| {
            let current = e.mem.u32(a[0]);
            if current == a[2] {
                e.mem.set_u32(a[0], a[1]);
            }
            current.into_ret()
        });
        e.register(SLEEP, |_, _| Ret::default());
        e.register(ANSWERS_YES, |_, _| 1u32.into_ret());
        e.register(ANSWERS_NO, |_, _| 0u32.into_ret());
        e.register(SET_AT, |_, _| Ret::default());
        e.register(EDITOR_SLOT, |_, _| Ret::default());
        e.register(LOG, |_, _| Ret::default());
        e.register(FIXED_STRING_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            a[0].into_ret()
        });
        e.register(FIXED_STRING_DESTRUCT, |_, _| Ret::default());
        e.register(FIXED_STRING_RELEASE, |_, _| Ret::default());
        e.register(FIXED_STRING_REFCOUNT, |_, _| STRING_COUNTER.into_ret());
        // Virtual tables: slot n is at byte 4n (0xC IsNode, 0x18 IsGeometry,
        // 0x60 loader, 0x90 editor slot, 0xF8 SetAt).
        let table = |is_node: u32, is_geometry: u32, loader: u32, editor: u32| {
            let mut slots = vec![ANSWERS_NO; 0x40];
            slots[3] = is_node;
            slots[6] = is_geometry;
            slots[0x18] = loader;
            slots[0x24] = editor;
            slots[0x3e] = SET_AT;
            slots
        };
        let tables = [
            (
                NODE_VTABLE,
                table(ANSWERS_YES, ANSWERS_NO, ANSWERS_NO, ANSWERS_NO),
            ),
            (
                GEOMETRY_VTABLE,
                table(ANSWERS_NO, ANSWERS_YES, ANSWERS_NO, ANSWERS_NO),
            ),
            (
                PLAIN_VTABLE,
                table(ANSWERS_NO, ANSWERS_NO, ANSWERS_NO, ANSWERS_NO),
            ),
            (
                STREAM_NO_VTABLE,
                table(ANSWERS_NO, ANSWERS_NO, ANSWERS_NO, ANSWERS_NO),
            ),
            (
                STREAM_LOADING_VTABLE,
                table(ANSWERS_NO, ANSWERS_NO, STREAM_LOADS_A_NODE, ANSWERS_NO),
            ),
            (
                WRAPPER_VTABLE,
                table(ANSWERS_YES, ANSWERS_NO, ANSWERS_NO, ANSWERS_NO),
            ),
            (
                EDITOR_VTABLE,
                table(ANSWERS_NO, ANSWERS_NO, ANSWERS_NO, EDITOR_SLOT),
            ),
        ];
        for (address, slots) in tables {
            e.put_vtable(address, &slots);
        }
        // The wrapper `InitModel` makes: a zeroed block with the wrapper
        // vtable.
        e.register(NI_ALLOC, |e, a| e.mem.alloc(a[0]).into_ret());
        e.register(NI_NODE_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], WRAPPER_VTABLE);
            a[0].into_ret()
        });
        e
    }

    /// An object with the given virtual table and 0x80 bytes.
    fn object(e: &mut Engine, vtable: u32) -> Ptr {
        let block = e.mem.alloc(0x80);
        e.mem.set_u32(block, vtable);
        Ptr::new(block)
    }

    /// A string in game memory.
    fn text(e: &mut Engine, s: &str) -> u32 {
        let block = e.mem.alloc(s.len() as u32 + 1);
        e.mem.set_cstr(block, s.as_bytes());
        block
    }

    /// A `BSStream` holding `objects` (in the array at +0x21C), with the
    /// given virtual table and no file name.
    fn stream_with(e: &mut Engine, vtable: u32, objects: &[Ptr]) -> Ptr {
        let stream = Ptr::new(e.mem.alloc(0x230));
        e.mem.set_u32(stream.addr(), vtable);
        set_stream_objects(e, stream, objects);
        stream
    }

    fn set_stream_objects(e: &mut Engine, stream: Ptr, objects: &[Ptr]) {
        let base = e.mem.alloc(4 * objects.len() as u32 + 4);
        for (i, object) in objects.iter().enumerate() {
            e.mem.set_u32(base + 4 * i as u32, object.addr());
        }
        e.mem.set_u32(stream.addr() + 0x21c + 4, base);
        e.mem
            .set_u32(stream.addr() + 0x21c + 0xc, objects.len() as u32);
    }

    fn model(e: &mut Engine) -> Ptr<Model> {
        e.new_object()
    }

    /// The argument lists of the calls made to `addr` since `call_log` was
    /// switched on.
    fn calls_to(e: &Engine, addr: u32) -> Vec<Vec<u32>> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .filter(|(a, _)| *a == addr)
            .map(|(_, args)| args.clone())
            .collect()
    }

    /// The addresses called, in order.
    fn call_order(e: &Engine) -> Vec<u32> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .map(|(a, _)| *a)
            .collect()
    }

    #[test]
    fn model_constructor_clears_the_pointer_then_initialises() {
        let mut e = loader_engine();
        let this = model(&mut e);
        let name = text(&mut e, "meshes\\a.nif");
        // An empty stream whose loader fails: InitModel gives up.
        let stream = stream_with(&mut e, STREAM_NO_VTABLE, &[]);
        e.register(STREAM_FREE_ALL_OBJECTS, |_, _| Ret::default());
        e.call_log = Some(vec![]);
        let back = e.call(0x0043_aaf0, &args![this, name, stream, 1u8, 0u8]);
        assert_eq!(back.ptr::<Model>(), this);
        let order = call_order(&e);
        assert_eq!(&order[1..3], &[NI_POINTER_CONSTRUCT, NI_POINTER_ASSIGN]);
        assert_eq!(
            calls_to(&e, NI_POINTER_CONSTRUCT)[0],
            vec![this.addr() + 0xc, 0]
        );
        assert!(order.contains(&STREAM_FREE_ALL_OBJECTS));
        assert_eq!(e.get(this, Model::pFilename), 0);
    }

    #[test]
    fn model_destructor_releases_frees_and_destroys() {
        let mut e = loader_engine();
        let this = model(&mut e);
        let filename = e.mem.alloc(16);
        e.set(this, Model::pFilename, filename);
        e.set(this, Model::spObject3D, Ptr::new(0x1234));
        e.call_log = Some(vec![]);
        e.call(0x0043_ab70, &args![this]);
        let pointer = this.addr() + 0xc;
        assert_eq!(
            e.call_log.take().unwrap(),
            vec![
                (0x0043_ab70, vec![this.addr()]),
                (NI_POINTER_ASSIGN, vec![pointer, 0]),
                (MEMORY_FREE, vec![filename]),
                (NI_POINTER_DESTRUCT, vec![pointer]),
            ]
        );
        assert_eq!(e.mem.u32(pointer), 0);
    }

    #[test]
    fn second_constructor_copies_the_name_and_holds_the_node() {
        let mut e = loader_engine();
        let this = model(&mut e);
        e.set(this, Model::iRefCount, 7);
        e.set(this, Model::iManualRefCount, 9);
        let name = text(&mut e, "abc");
        let back = e.call(0x0043_abf0, &args![this, name, Ptr::<()>::new(0x4444)]);
        assert_eq!(back.ptr::<Model>(), this);
        let copy = e.get(this, Model::pFilename);
        assert_ne!(copy, name);
        assert_eq!(e.mem.cstr(copy), b"abc");
        assert_eq!(e.get(this, Model::spObject3D), Ptr::new(0x4444));
        assert_eq!(e.get(this, Model::iRefCount), 0);
        assert_eq!(e.get(this, Model::iManualRefCount), 0);
    }

    #[test]
    fn manual_count_is_decremented_only_when_positive() {
        let mut e = loader_engine();
        let this = model(&mut e);
        for (before, after) in [(3, 2), (1, 0), (0, 0), (-4, -4)] {
            e.set(this, Model::iManualRefCount, before);
            e.call(0x0043_acb0, &args![this]);
            assert_eq!(e.get(this, Model::iManualRefCount), after);
        }
    }

    /// A model and a stream holding `objects`, `InitModel` run with
    /// `prepare` and `force`.
    fn init(e: &mut Engine, objects: &[Ptr], prepare: u8, force: u8) -> (Ptr<Model>, Ptr) {
        let this = model(e);
        let name = text(e, "meshes\\a.nif");
        let stream = stream_with(e, STREAM_NO_VTABLE, objects);
        e.call(0x0043_ace0, &args![this, name, stream, prepare, force]);
        (this, stream)
    }

    fn object_3d(e: &Engine, model: Ptr<Model>) -> Ptr {
        e.get(model, Model::spObject3D)
    }

    /// Doubles for what `InitModel` calls after it has taken objects, with
    /// no properties on the root.
    fn quiet_init_engine() -> Engine {
        let mut e = loader_engine();
        e.register(REMOVE_EDITOR_MARKERS, |_, _| Ret::default());
        e.register(GET_PROPERTY, |_, _| 0u32.into_ret());
        e.register(PROPERTY_TYPE_SECOND_GETTER, |_, _| SECOND_TYPE.into_ret());
        e.register(MATCHES_PATTERN, |_, _| 0u32.into_ret());
        e.register(STREAM_FREE_ALL_OBJECTS, |_, _| Ret::default());
        e
    }

    #[test]
    fn init_model_keeps_a_single_root_node_directly() {
        let mut e = quiet_init_engine();
        let root = object(&mut e, NODE_VTABLE);
        // The node's name is not a `Bip##` one.
        let node_name = text(&mut e, "Scene Root");
        e.mem.set_u32(root.addr() + 8, node_name);
        e.call_log = Some(vec![]);
        let (this, _) = init(&mut e, &[root], 0, 0);
        assert_eq!(object_3d(&e, this), root);
        assert_eq!(calls_to(&e, REMOVE_EDITOR_MARKERS), vec![vec![root.addr()]]);
        assert_eq!(e.get(this, Model::iRefCount), 0);
        assert_eq!(e.get(this, Model::iManualRefCount), 0);
        let copy = e.get(this, Model::pFilename);
        assert_eq!(e.mem.cstr(copy), b"meshes\\a.nif");
        // No wrapper was made.
        assert!(calls_to(&e, NI_ALLOC).is_empty());
        // The matcher was asked about the name.
        assert_eq!(
            calls_to(&e, MATCHES_PATTERN),
            vec![vec![node_name, BIP_PATTERN]]
        );
    }

    #[test]
    fn init_model_wraps_a_bip_root_in_a_new_node() {
        let mut e = quiet_init_engine();
        e.register(MATCHES_PATTERN, |_, _| 1u32.into_ret());
        let root = object(&mut e, NODE_VTABLE);
        let node_name = text(&mut e, "Bip01");
        e.mem.set_u32(root.addr() + 8, node_name);
        e.call_log = Some(vec![]);
        let (this, _) = init(&mut e, &[root], 0, 0);
        let wrapper = object_3d(&e, this);
        assert_ne!(wrapper, root);
        assert_eq!(e.mem.u32(wrapper.addr()), WRAPPER_VTABLE);
        assert_eq!(calls_to(&e, NI_ALLOC), vec![vec![0xac]]);
        assert_eq!(calls_to(&e, NI_NODE_CONSTRUCT).len(), 1);
        // The Bip node is child 0 of the wrapper.
        assert_eq!(
            calls_to(&e, SET_AT),
            vec![vec![wrapper.addr(), 0, root.addr()]]
        );
    }

    #[test]
    fn init_model_without_a_node_name_keeps_the_root_directly() {
        let mut e = quiet_init_engine();
        e.register(MATCHES_PATTERN, |_, _| 1u32.into_ret());
        // A null name is never matched against the pattern.
        let root = object(&mut e, NODE_VTABLE);
        e.call_log = Some(vec![]);
        let (this, _) = init(&mut e, &[root], 0, 0);
        assert_eq!(object_3d(&e, this), root);
        assert!(calls_to(&e, MATCHES_PATTERN).is_empty());
    }

    #[test]
    fn init_model_collects_nodes_and_geometry_and_skips_the_rest() {
        let mut e = quiet_init_engine();
        let plain = object(&mut e, PLAIN_VTABLE);
        let geometry = object(&mut e, GEOMETRY_VTABLE);
        let node = object(&mut e, NODE_VTABLE);
        // Object 0 is not a node, so the single-root shortcut is not taken;
        // a null entry and a plain object are skipped, the others get
        // consecutive slots of one wrapper.
        e.call_log = Some(vec![]);
        let (this, _) = init(&mut e, &[plain, Ptr::NULL, geometry, node], 0, 0);
        let wrapper = object_3d(&e, this);
        assert_eq!(e.mem.u32(wrapper.addr()), WRAPPER_VTABLE);
        assert_eq!(
            calls_to(&e, SET_AT),
            vec![
                vec![wrapper.addr(), 0, geometry.addr()],
                vec![wrapper.addr(), 1, node.addr()],
            ]
        );
        // Only one wrapper is made.
        assert_eq!(calls_to(&e, NI_ALLOC).len(), 1);
        assert_eq!(
            calls_to(&e, REMOVE_EDITOR_MARKERS),
            vec![vec![wrapper.addr()]]
        );
    }

    #[test]
    fn init_model_with_nothing_to_take_does_no_post_processing() {
        let mut e = quiet_init_engine();
        let plain = object(&mut e, PLAIN_VTABLE);
        e.call_log = Some(vec![]);
        let (this, _) = init(&mut e, &[plain], 1, 0);
        assert_eq!(object_3d(&e, this), Ptr::NULL);
        assert!(calls_to(&e, REMOVE_EDITOR_MARKERS).is_empty());
        assert!(calls_to(&e, PREPARE_OBJECT).is_empty());
    }

    #[test]
    fn init_model_loads_the_error_marker_into_an_empty_stream() {
        let mut e = quiet_init_engine();
        // The stream's loader (slot 0x60) gives it one node and succeeds.
        let marker = object(&mut e, NODE_VTABLE);
        e.register_double(STREAM_LOADS_A_NODE, move |e, a| {
            let stream = Ptr::new(a[0]);
            set_stream_objects(e, stream, &[marker]);
            1u32.into_ret()
        });
        let stream = stream_with(&mut e, STREAM_LOADING_VTABLE, &[]);
        let this = model(&mut e);
        let name = text(&mut e, "meshes\\missing.nif");
        e.call_log = Some(vec![]);
        e.call(0x0043_ace0, &args![this, name, stream, 0u8, 0u8]);
        assert_eq!(
            calls_to(&e, STREAM_FREE_ALL_OBJECTS),
            vec![vec![stream.addr()]]
        );
        // The loader was called with the marker's path and 0.
        assert_eq!(
            calls_to(&e, STREAM_LOADS_A_NODE),
            vec![vec![stream.addr(), MARKER_ERROR_NIF, 0]]
        );
        assert_eq!(object_3d(&e, this), marker);
        // The model keeps the requested name.
        let copy = e.get(this, Model::pFilename);
        assert_eq!(e.mem.cstr(copy), b"meshes\\missing.nif");
    }

    #[test]
    fn init_model_gives_up_when_even_the_error_marker_fails() {
        let mut e = quiet_init_engine();
        e.call_log = Some(vec![]);
        let (this, _) = init(&mut e, &[], 1, 1);
        assert_eq!(e.get(this, Model::pFilename), 0);
        assert_eq!(object_3d(&e, this), Ptr::NULL);
        assert!(calls_to(&e, REMOVE_EDITOR_MARKERS).is_empty());
    }

    /// A scene where the root has both shared properties (equal to the
    /// shared ones in the compared word), with the stream's name byte set or
    /// not. Returns the model, the stream, the model's name and the root.
    fn property_scene(e: &mut Engine, with_stream_name: bool) -> (Ptr<Model>, Ptr, u32, Ptr) {
        // The two shared properties and the root's own copies.
        let shared_first = object(e, PLAIN_VTABLE);
        let shared_second = object(e, PLAIN_VTABLE);
        e.mem.set_u16(shared_first.addr() + 0x18, 5);
        e.mem.set_u16(shared_second.addr() + 0x18, 6);
        e.mem.set_u32(SHARED_PROPERTY_FIRST, shared_first.addr());
        e.mem.set_u32(SHARED_PROPERTY_SECOND, shared_second.addr());
        let root = object(e, NODE_VTABLE);
        let own_first = object(e, PLAIN_VTABLE);
        let own_second = object(e, PLAIN_VTABLE);
        e.mem.set_u16(own_first.addr() + 0x18, 5);
        e.mem.set_u16(own_second.addr() + 0x18, 6);
        let (first, second) = (own_first.addr(), own_second.addr());
        e.register_double(GET_PROPERTY, move |_, a| {
            match a[1] {
                PROPERTY_TYPE_FIRST => first,
                SECOND_TYPE => second,
                _ => 0,
            }
            .into_ret()
        });
        e.register(REMOVE_PROPERTY, |_, _| Ret::default());
        let this = model(e);
        let name = text(e, "meshes\\a.nif");
        let stream = stream_with(e, STREAM_NO_VTABLE, &[root]);
        if with_stream_name {
            e.mem.set_cstr(stream.addr() + 8, b"x.nif");
        }
        (this, stream, name, root)
    }

    #[test]
    fn init_model_strips_both_properties_and_logs_with_the_stream_name() {
        let mut e = quiet_init_engine();
        let (this, stream, name, root) = property_scene(&mut e, true);
        e.call_log = Some(vec![]);
        e.call(0x0043_ace0, &args![this, name, stream, 0u8, 0u8]);
        assert_eq!(
            calls_to(&e, REMOVE_PROPERTY),
            vec![
                vec![root.addr(), PROPERTY_TYPE_FIRST],
                vec![root.addr(), SECOND_TYPE]
            ]
        );
        assert_eq!(
            calls_to(&e, LOG),
            vec![vec![REEXPORT_WITH_FILE_MESSAGE, stream.addr() + 8, name]]
        );
        // The warning counter went up and back down around the checks.
        assert_eq!(calls_to(&e, INTERLOCKED_INCREMENT).len(), 1);
        assert_eq!(calls_to(&e, INTERLOCKED_DECREMENT).len(), 1);
        assert_eq!(e.global::<i32>(DISABLE_WARNING_COUNT), 0);
    }

    #[test]
    fn init_model_logs_without_a_stream_name_when_it_has_none() {
        let mut e = quiet_init_engine();
        let (this, stream, name, _) = property_scene(&mut e, false);
        e.call_log = Some(vec![]);
        e.call(0x0043_ace0, &args![this, name, stream, 0u8, 0u8]);
        assert_eq!(calls_to(&e, LOG), vec![vec![REEXPORT_MESSAGE, name]]);
    }

    #[test]
    fn init_model_keeps_properties_that_are_not_the_shared_ones() {
        let mut e = quiet_init_engine();
        let (this, stream, name, _) = property_scene(&mut e, true);
        // Make the stored shared properties differ from the root's.
        let first = e.mem.u32(SHARED_PROPERTY_FIRST);
        e.mem.set_u16(first + 0x18, 99);
        let second = e.mem.u32(SHARED_PROPERTY_SECOND);
        e.mem.set_u16(second + 0x18, 98);
        e.call_log = Some(vec![]);
        e.call(0x0043_ace0, &args![this, name, stream, 0u8, 0u8]);
        assert!(calls_to(&e, REMOVE_PROPERTY).is_empty());
        assert!(calls_to(&e, LOG).is_empty());
    }

    #[test]
    fn init_model_strips_only_the_second_property_when_the_first_is_missing() {
        let mut e = quiet_init_engine();
        let (this, stream, name, root) = property_scene(&mut e, false);
        let own_second = object(&mut e, PLAIN_VTABLE);
        let shared_second = e.mem.u32(SHARED_PROPERTY_SECOND);
        let word = e.mem.u16(shared_second + 0x18);
        e.mem.set_u16(own_second.addr() + 0x18, word);
        let own_second = own_second.addr();
        // The root has no property of the first type.
        e.register_double(GET_PROPERTY, move |_, a| {
            if a[1] == SECOND_TYPE { own_second } else { 0 }.into_ret()
        });
        e.call_log = Some(vec![]);
        e.call(0x0043_ace0, &args![this, name, stream, 0u8, 0u8]);
        assert_eq!(
            calls_to(&e, REMOVE_PROPERTY),
            vec![vec![root.addr(), SECOND_TYPE]]
        );
    }

    #[test]
    fn init_model_prepares_the_object_when_asked() {
        // `prepare` set, `force` clear, no morpher: PrepareObject(root, 0, 0).
        let mut e = quiet_init_engine();
        let (this, stream, name, root) = property_scene(&mut e, false);
        e.register(HAS_MORPHER_CONTROLLER, |_, _| 0u32.into_ret());
        e.register(PREPARE_OBJECT, |_, _| Ret::default());
        e.call_log = Some(vec![]);
        e.call(0x0043_ace0, &args![this, name, stream, 1u8, 0u8]);
        assert_eq!(
            calls_to(&e, HAS_MORPHER_CONTROLLER),
            vec![vec![root.addr()]]
        );
        assert_eq!(calls_to(&e, PREPARE_OBJECT), vec![vec![root.addr(), 0, 0]]);

        // A morpher controller sets the last flag.
        let (this, stream, name, root) = property_scene(&mut e, false);
        e.register(HAS_MORPHER_CONTROLLER, |_, _| 1u32.into_ret());
        e.call_log = Some(vec![]);
        e.call(0x0043_ace0, &args![this, name, stream, 1u8, 0u8]);
        assert_eq!(calls_to(&e, PREPARE_OBJECT), vec![vec![root.addr(), 0, 1]]);

        // `force` skips the morpher test and passes itself and 1.
        let (this, stream, name, root) = property_scene(&mut e, false);
        e.call_log = Some(vec![]);
        e.call(0x0043_ace0, &args![this, name, stream, 1u8, 1u8]);
        assert!(calls_to(&e, HAS_MORPHER_CONTROLLER).is_empty());
        assert_eq!(calls_to(&e, PREPARE_OBJECT), vec![vec![root.addr(), 1, 1]]);

        // Without `prepare` nothing is prepared.
        let (this, stream, name, _) = property_scene(&mut e, false);
        e.call_log = Some(vec![]);
        e.call(0x0043_ace0, &args![this, name, stream, 0u8, 1u8]);
        assert!(calls_to(&e, PREPARE_OBJECT).is_empty());
    }

    #[test]
    fn init_model_runs_the_editor_branch_only_when_the_flag_is_set() {
        let mut e = quiet_init_engine();
        let (this, stream, name, root) = property_scene(&mut e, false);
        // The root's NiPointer at +0xC and the cast result.
        let controller = 0x7000_1000;
        e.mem.set_u32(root.addr() + 0xc, controller);
        let casted = e.mem.alloc(8);
        let target = object(&mut e, EDITOR_VTABLE);
        e.mem.set_u32(casted, target.addr());
        let editor_name = text(&mut e, "Root");
        e.mem.set_u32(root.addr() + 8, editor_name);
        e.register_double(DYNAMIC_CAST, move |_, _| casted.into_ret());
        e.register(CAST_RESULT_POINTER, |e, a| e.mem.u32(a[0]).into_ret());

        // Flag clear: nothing happens.
        e.call_log = Some(vec![]);
        e.call(0x0043_ace0, &args![this, name, stream, 0u8, 0u8]);
        assert!(calls_to(&e, DYNAMIC_CAST).is_empty());
        assert!(calls_to(&e, EDITOR_SLOT).is_empty());

        // Flag set: the root's `NiPointer` at +0xC is cast, and slot 0x90 of
        // the pointer the cast result holds gets the root's name slot and 0.
        let (this, stream, name, root) = property_scene(&mut e, false);
        e.mem.set_u32(root.addr() + 0xc, controller);
        e.mem.set_u32(root.addr() + 8, editor_name);
        e.mem.set_u8(GUN_WOBBLE_FLAG, 1);
        e.call_log = Some(vec![]);
        e.call(0x0043_ace0, &args![this, name, stream, 0u8, 0u8]);
        assert_eq!(
            calls_to(&e, DYNAMIC_CAST),
            vec![vec![TYPE_DESCRIPTOR_011F36AC, controller]]
        );
        assert_eq!(
            calls_to(&e, EDITOR_SLOT),
            vec![vec![target.addr(), root.addr() + 8, 0]]
        );

        // The cast failing (null) stops it.
        let (this, stream, name, root) = property_scene(&mut e, false);
        e.mem.set_u32(root.addr() + 0xc, controller);
        e.register(DYNAMIC_CAST, |_, _| 0u32.into_ret());
        e.call_log = Some(vec![]);
        e.call(0x0043_ace0, &args![this, name, stream, 0u8, 0u8]);
        assert!(calls_to(&e, EDITOR_SLOT).is_empty());
    }

    #[test]
    fn acquire_load_reads_the_dword() {
        let mut e = loader_engine();
        let slot = e.mem.alloc(8);
        e.mem.set_u32(slot, 0xcafe);
        assert_eq!(
            e.call(0x0043_b1d0, &args![Ptr::<()>::new(slot)]).u32(),
            0xcafe
        );
        assert_eq!(
            e.call(0x0043_b1b0, &args![Ptr::<()>::new(slot)]).u32(),
            0xcafe
        );
    }

    #[test]
    fn stream_accessors_read_the_array_at_0x21c() {
        let mut e = loader_engine();
        let a = object(&mut e, PLAIN_VTABLE);
        let b = object(&mut e, PLAIN_VTABLE);
        let stream = stream_with(&mut e, STREAM_NO_VTABLE, &[a, b]);
        assert_eq!(e.call(0x0043_b1e0, &args![stream]).u32(), 2);
        assert_eq!(e.call(0x0043_b200, &args![stream, 0u32]).ptr::<()>(), a);
        assert_eq!(e.call(0x0043_b200, &args![stream, 1u32]).ptr::<()>(), b);
    }

    #[test]
    fn pointer_field_at_0xc_is_dereferenced() {
        let mut e = loader_engine();
        let node = object(&mut e, NODE_VTABLE);
        e.mem.set_u32(node.addr() + 0xc, 0x1357);
        assert_eq!(e.call(0x0043_b230, &args![node]).u32(), 0x1357);
    }

    #[test]
    fn shared_property_getters_read_their_globals() {
        let mut e = loader_engine();
        e.mem.set_u32(SHARED_PROPERTY_SECOND, 0x1111);
        e.mem.set_u32(SHARED_PROPERTY_FIRST, 0x2222);
        assert_eq!(e.call(0x0043_b250, &args![]).u32(), 0x1111);
        assert_eq!(e.call(0x0043_b2a0, &args![]).u32(), 0x2222);
    }

    #[test]
    fn property_comparison_is_on_the_word_at_0x18() {
        let mut e = loader_engine();
        let a = object(&mut e, PLAIN_VTABLE);
        let b = object(&mut e, PLAIN_VTABLE);
        e.mem.set_u16(a.addr() + 0x18, 12);
        e.mem.set_u16(b.addr() + 0x18, 12);
        assert!(e.call(0x0043_b260, &args![a, b]).bool());
        e.mem.set_u16(b.addr() + 0x18, 13);
        assert!(!e.call(0x0043_b260, &args![a, b]).bool());
        // Bits above the word do not matter.
        e.mem.set_u16(b.addr() + 0x1a, 0xffff);
        e.mem.set_u16(b.addr() + 0x18, 12);
        assert!(e.call(0x0043_b260, &args![a, b]).bool());
    }

    #[test]
    fn first_property_type_is_eleven() {
        let mut e = loader_engine();
        assert_eq!(e.call(0x0043_b290, &args![]).u32(), 11);
    }

    #[test]
    fn warning_counter_moves_by_one_and_never_goes_negative() {
        let mut e = loader_engine();
        e.call(0x0043_b2b0, &args![1u8]);
        assert_eq!(e.global::<i32>(DISABLE_WARNING_COUNT), 1);
        e.call(0x0043_b2b0, &args![1u8]);
        assert_eq!(e.global::<i32>(DISABLE_WARNING_COUNT), 2);
        e.call(0x0043_b2b0, &args![0u8]);
        assert_eq!(e.global::<i32>(DISABLE_WARNING_COUNT), 1);
        e.call(0x0043_b2b0, &args![0u8]);
        e.call(0x0043_b2b0, &args![0u8]);
        // The second decrement went to -1 and was clamped.
        assert_eq!(e.global::<i32>(DISABLE_WARNING_COUNT), 0);
        e.call_log = Some(vec![]);
        e.call(0x0043_b2b0, &args![1u8]);
        assert_eq!(
            calls_to(&e, INTERLOCKED_INCREMENT),
            vec![vec![DISABLE_WARNING_COUNT]]
        );
    }

    #[test]
    fn kind_test_is_false_for_null_and_asks_the_object_otherwise() {
        let mut e = loader_engine();
        e.register(IS_KIND_OF, |_, a| (a[0] == 0x4000).into_ret());
        e.call_log = Some(vec![]);
        let null = Ptr::<()>::NULL;
        assert!(!e.call(0x0043_b300, &args![0x1234u32, null]).bool());
        assert!(calls_to(&e, IS_KIND_OF).is_empty());
        let yes = Ptr::<()>::new(0x4000);
        let no = Ptr::<()>::new(0x5000);
        assert!(e.call(0x0043_b300, &args![0x1234u32, yes]).bool());
        assert!(!e.call(0x0043_b300, &args![0x1234u32, no]).bool());
        // Arguments: the object, then the type descriptor.
        assert_eq!(calls_to(&e, IS_KIND_OF)[0], vec![0x4000, 0x1234]);
    }

    /// A node-like block with a child array of four slots at +0x38 (base)
    /// and the 16-bit count `count` at +0x3C.
    fn parent_with_children(e: &mut Engine, children: &[u32], count: u16) -> Ptr {
        let parent = e.mem.alloc(0x80);
        let base = e.mem.alloc(16);
        for (i, child) in children.iter().enumerate() {
            e.mem.set_u32(base + 4 * i as u32, *child);
        }
        e.mem.set_u32(parent + 0x38, base);
        e.mem.set_u16(parent + 0x3c, count);
        Ptr::new(parent)
    }

    #[test]
    fn set_child_stores_it_and_flags_a_non_null_child() {
        let mut e = loader_engine();
        let parent = parent_with_children(&mut e, &[0, 0, 0, 0], 4);
        let child = object(&mut e, NODE_VTABLE);
        e.call(0x0043_b320, &args![parent, 2u16, child]);
        let base = e.mem.u32(parent.addr() + 0x38);
        assert_eq!(e.mem.u32(base + 8), child.addr());
        assert_eq!(e.mem.u32(child.addr() + 0x30), 0x80);
        // Storing null clears the slot and flags nothing.
        e.call(0x0043_b320, &args![parent, 2u16, Ptr::<()>::NULL]);
        assert_eq!(e.mem.u32(base + 8), 0);
    }

    #[test]
    fn flag_setter_sets_and_clears_the_0x80_bit() {
        let mut e = loader_engine();
        let target = object(&mut e, NODE_VTABLE);
        e.mem.set_u32(target.addr() + 0x30, 0x0000_0101);
        e.call(0x0043_b350, &args![target, 1u8]);
        assert_eq!(e.mem.u32(target.addr() + 0x30), 0x181);
        e.call(0x0043_b350, &args![target, 0u8]);
        assert_eq!(e.mem.u32(target.addr() + 0x30), 0x101);
    }

    #[test]
    fn flag_mask_setter_sets_and_clears_arbitrary_bits() {
        let mut e = loader_engine();
        let target = object(&mut e, NODE_VTABLE);
        e.mem.set_u32(target.addr() + 0x30, 0xf0);
        e.call(0x0043_b370, &args![target, 1u8, 0x0fu32]);
        assert_eq!(e.mem.u32(target.addr() + 0x30), 0xff);
        e.call(0x0043_b370, &args![target, 0u8, 0x3cu32]);
        assert_eq!(e.mem.u32(target.addr() + 0x30), 0xc3);
        // Any non-zero `set` byte means set.
        e.call(0x0043_b370, &args![target, 2u8, 0x100u32]);
        assert_eq!(e.mem.u32(target.addr() + 0x30), 0x1c3);
    }

    #[test]
    fn child_index_search_returns_the_slot_or_the_not_found_word() {
        let mut e = loader_engine();
        e.mem.set_u16(NOT_FOUND_INDEX, 0xffff);
        let parent = parent_with_children(&mut e, &[0x10, 0x20, 0x30, 0x40], 3);
        let find = |e: &mut Engine, child: u32| {
            e.call(0x0043_b3b0, &args![parent, Ptr::<()>::new(child)])
                .u16()
        };
        assert_eq!(find(&mut e, 0x10), 0);
        assert_eq!(find(&mut e, 0x30), 2);
        // Slot 3 holds 0x40 but is beyond the count.
        assert_eq!(find(&mut e, 0x40), 0xffff);
        assert_eq!(find(&mut e, 0x99), 0xffff);
    }

    #[test]
    fn manual_count_changes_with_compare_and_swap() {
        let mut e = loader_engine();
        let this = model(&mut e);
        e.set(this, Model::iManualRefCount, 5);
        e.call_log = Some(vec![]);
        e.call(0x0043_b410, &args![this, 3i32]);
        assert_eq!(e.get(this, Model::iManualRefCount), 8);
        assert!(calls_to(&e, SLEEP).is_empty());
        // Import arguments: target, new value, expected value.
        assert_eq!(
            calls_to(&e, INTERLOCKED_COMPARE_EXCHANGE),
            vec![vec![this.addr() + 8, 8, 5]]
        );
        e.call(0x0043_b410, &args![this, -2i32]);
        assert_eq!(e.get(this, Model::iManualRefCount), 6);
    }

    #[test]
    fn manual_count_retries_after_a_lost_race() {
        let mut e = loader_engine();
        let this = model(&mut e);
        e.set(this, Model::iManualRefCount, 5);
        // Another thread changes the count to 7 during the first attempt.
        let mut attempts = 0;
        e.register_double(INTERLOCKED_COMPARE_EXCHANGE, move |e, a| {
            attempts += 1;
            if attempts == 1 {
                e.mem.set_u32(a[0], 7);
                return 7u32.into_ret();
            }
            let current = e.mem.u32(a[0]);
            if current == a[2] {
                e.mem.set_u32(a[0], a[1]);
            }
            current.into_ret()
        });
        e.call_log = Some(vec![]);
        e.call(0x0043_b410, &args![this, 3i32]);
        assert_eq!(e.get(this, Model::iManualRefCount), 10);
        assert_eq!(calls_to(&e, SLEEP), vec![vec![0]]);
        let attempts = calls_to(&e, INTERLOCKED_COMPARE_EXCHANGE);
        assert_eq!(attempts.len(), 2);
        assert_eq!(attempts[1], vec![this.addr() + 8, 10, 7]);
    }

    #[test]
    fn compare_exchange_wrapper_reorders_the_arguments() {
        let mut e = loader_engine();
        let cell = e.mem.alloc(8);
        e.mem.set_i32(cell, 4);
        e.call_log = Some(vec![]);
        // (target, old, new): the import takes (target, new, old).
        let seen = e.call(0x0043_b460, &args![Ptr::<()>::new(cell), 4i32, 9i32]);
        assert_eq!(seen.i32(), 4);
        assert_eq!(e.mem.i32(cell), 9);
        assert_eq!(
            calls_to(&e, INTERLOCKED_COMPARE_EXCHANGE),
            vec![vec![cell, 9, 4]]
        );
        let seen = e.call(0x0043_b460, &args![Ptr::<()>::new(cell), 4i32, 1i32]);
        assert_eq!(seen.i32(), 9);
        assert_eq!(e.mem.i32(cell), 9);
    }

    #[test]
    fn array_at_0x9c_has_a_size_and_elements() {
        let mut e = loader_engine();
        let owner = e.mem.alloc(0xb0);
        let items = e.mem.alloc(16);
        e.mem.set_u32(items, 0xaaa0);
        e.mem.set_u32(items + 4, 0xbbb0);
        e.mem.set_u32(owner + 0x9c + 4, items);
        e.mem.set_u16(owner + 0x9c + 0xa, 2);
        let owner = Ptr::<()>::new(owner);
        assert_eq!(e.call(0x0043_b480, &args![owner]).u16(), 2);
        assert_eq!(e.call(0x0043_b4a0, &args![owner, 1u32]).u32(), 0xbbb0);
    }

    #[test]
    fn low_seven_bits_are_returned() {
        let mut e = loader_engine();
        let cell = Ptr::<()>::new(e.mem.alloc(8));
        e.mem.set_u32(cell.addr(), 0xffff_ffff);
        assert_eq!(e.call(0x0043_b4d0, &args![cell]).u32(), 0x7f);
        e.mem.set_u32(cell.addr(), 0x1234_5680);
        assert_eq!(e.call(0x0043_b4d0, &args![cell]).u32(), 0);
    }

    /// Test doubles for the helpers of `0043b4f0` .. `0043b5f0`, and a
    /// block standing for the object they find.
    fn holder_engine() -> (Engine, u32) {
        let mut e = loader_engine();
        let block = e.mem.alloc(0x100);
        e.register(REF_COUNT_OBJECT_POINTER, |e, a| {
            e.mem.u32(a[0] + 8).into_ret()
        });
        e.register(DELETE_OBJECT, |_, _| Ret::default());
        e.register(HOLDER_VALUE, |_, a| (a[0] + 0x10).into_ret());
        e.register(HOLDER_DESTRUCT, |_, _| Ret::default());
        e.register(FIELD_AT_8, |e, a| e.mem.u32(a[0] + 8).into_ret());
        e.register(STORE_POINTER, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            a[0].into_ret()
        });
        (e, block)
    }

    #[test]
    fn holder_helper_destroys_the_block_and_returns_its_value() {
        let (mut e, block) = holder_engine();
        let owner = e.mem.alloc(0x40);
        e.mem.set_u32(owner + 8, block);
        e.call_log = Some(vec![]);
        let value = e.call(0x0043_b560, &args![Ptr::<()>::new(owner)]).u32();
        assert_eq!(value, block + 0x10);
        assert_eq!(
            e.call_log.take().unwrap(),
            vec![
                (0x0043_b560, vec![owner]),
                (REF_COUNT_OBJECT_POINTER, vec![owner]),
                (DELETE_OBJECT, vec![block + 0x6c, 1]),
                (HOLDER_VALUE, vec![block]),
                (HOLDER_DESTRUCT, vec![block + 0x6c]),
            ]
        );
        // No block: 0, and nothing else is called.
        let empty = e.mem.alloc(0x40);
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x0043_b560, &args![Ptr::<()>::new(empty)]).u32(), 0);
        assert_eq!(call_order(&e), vec![0x0043_b560, REF_COUNT_OBJECT_POINTER]);
    }

    #[test]
    fn holder_helper_pieces_work_on_the_embedded_object() {
        let (mut e, block) = holder_engine();
        let block = Ptr::<()>::new(block);
        assert_eq!(
            e.call(0x0043_b5d0, &args![block]).u32(),
            block.addr() + 0x6c
        );
        e.call_log = Some(vec![]);
        e.call(0x0043_b5b0, &args![block]);
        e.call(0x0043_b5f0, &args![block]);
        assert_eq!(
            calls_to(&e, DELETE_OBJECT),
            vec![vec![block.addr() + 0x6c, 1]]
        );
        assert_eq!(
            calls_to(&e, HOLDER_DESTRUCT),
            vec![vec![block.addr() + 0x6c]]
        );
    }

    #[test]
    fn field_at_8_of_the_object_at_0x14() {
        let (mut e, block) = holder_engine();
        e.mem.set_u32(block + 0x14 + 8, 0x4242);
        assert_eq!(
            e.call(0x0043_b540, &args![Ptr::<()>::new(block)]).u32(),
            0x4242
        );
    }

    #[test]
    fn holder_store_writes_the_value_or_zero() {
        let (mut e, block) = holder_engine();
        let owner = e.mem.alloc(0x40);
        e.mem.set_u32(owner + 8, block);
        let out = e.mem.alloc(8);
        // `0043b560` returns block + 0x10; `0043b540` reads that object's
        // field at 0x14 + 8.
        e.mem.set_u32(block + 0x10 + 0x14 + 8, 0x77);
        let owner = Ptr::<()>::new(owner);
        let out_ptr = Ptr::<()>::new(out);
        let back = e.call(0x0043_b4f0, &args![owner, out_ptr]);
        assert_eq!(back.u32(), out);
        assert_eq!(e.mem.u32(out), 0x77);
        // Nothing found: stores 0.
        let empty = Ptr::<()>::new(e.mem.alloc(0x40));
        e.mem.set_u32(out, 5);
        e.call(0x0043_b4f0, &args![empty, out_ptr]);
        assert_eq!(e.mem.u32(out), 0);
    }

    #[test]
    fn collision_object_is_cast_to_its_class() {
        let mut e = loader_engine();
        e.register(COLLISION_OBJECT, |_, a| (a[0] + 1).into_ret());
        e.register(DYNAMIC_CAST, |_, a| {
            if a[1] == 0x9001 { a[1] } else { 0 }.into_ret()
        });
        e.call_log = Some(vec![]);
        let found = e.call(0x0043_b610, &args![Ptr::<()>::new(0x9000)]);
        assert_eq!(found.u32(), 0x9001);
        assert_eq!(
            calls_to(&e, DYNAMIC_CAST),
            vec![vec![TYPE_DESCRIPTOR_BHK_COLLISION_OBJECT, 0x9001]]
        );
        assert_eq!(e.call(0x0043_b610, &args![Ptr::<()>::new(0x7000)]).u32(), 0);
    }

    /// An engine for the `KFModel` functions, and a sequence object with
    /// doubles for the animation group and its helpers.
    fn kf_engine() -> (Engine, Ptr) {
        let mut e = loader_engine();
        let sequence = object(&mut e, PLAIN_VTABLE);
        e.register(LOAD_ANIM_GROUP, |_, _| 0x5555u32.into_ret());
        e.register(IS_KIND_OF, |_, _| 0u32.into_ret());
        e.register(KF_MODEL_SEQUENCE, |e, a| e.mem.u32(a[0] + 4).into_ret());
        (e, sequence)
    }

    #[test]
    fn kf_model_without_a_sequence_logs_and_stops() {
        let (mut e, _) = kf_engine();
        e.register(LOAD_KF_SEQUENCE, |_, _| Ret::default());
        let this: Ptr<KFModel> = e.new_object();
        e.set(this, KFModel::iRefCount, 3);
        let name = text(&mut e, "idle.kf");
        let file = object(&mut e, PLAIN_VTABLE);
        e.call_log = Some(vec![]);
        let back = e.call(0x0043_b640, &args![this, name, file]);
        assert_eq!(back.ptr::<KFModel>(), this);
        assert_eq!(e.get(this, KFModel::iRefCount), 0);
        assert_eq!(e.get(this, KFModel::iManualRefCount), 0);
        let copy = e.get(this, KFModel::pFilename);
        assert_eq!(e.mem.cstr(copy), b"idle.kf");
        // The sequence was requested from the file into the NiPointer at +4.
        assert_eq!(
            calls_to(&e, LOAD_KF_SEQUENCE),
            vec![vec![file.addr(), 0, this.addr() + 4]]
        );
        assert_eq!(calls_to(&e, LOG), vec![vec![NO_SEQUENCE_MESSAGE, copy]]);
        assert!(calls_to(&e, LOAD_ANIM_GROUP).is_empty());
    }

    #[test]
    fn kf_model_with_a_sequence_loads_its_animation_group() {
        let (mut e, sequence) = kf_engine();
        e.register_double(LOAD_KF_SEQUENCE, move |e, a| {
            e.mem.set_u32(a[2], sequence.addr());
            Ret::default()
        });
        let this: Ptr<KFModel> = e.new_object();
        let name = text(&mut e, "idle.kf");
        let file = object(&mut e, PLAIN_VTABLE);
        e.call_log = Some(vec![]);
        e.call(0x0043_b640, &args![this, name, file]);
        assert!(calls_to(&e, LOG).is_empty());
        assert_eq!(
            calls_to(&e, LOAD_ANIM_GROUP),
            vec![vec![sequence.addr(), name]]
        );
        assert_eq!(e.get(this, KFModel::spSequence), sequence);
        assert_eq!(e.get(this, KFModel::spAnimGroup), Ptr::new(0x5555));
    }

    #[test]
    fn kf_model_destructor_frees_the_name_and_releases_both_pointers() {
        let (mut e, _) = kf_engine();
        let this: Ptr<KFModel> = e.new_object();
        let filename = e.mem.alloc(16);
        e.set(this, KFModel::pFilename, filename);
        e.set(this, KFModel::spSequence, Ptr::new(0x1000));
        e.set(this, KFModel::spAnimGroup, Ptr::new(0x2000));
        e.call_log = Some(vec![]);
        e.call(0x0043_b750, &args![this]);
        assert_eq!(
            e.call_log.take().unwrap(),
            vec![
                (0x0043_b750, vec![this.addr()]),
                (MEMORY_FREE, vec![filename]),
                (NI_POINTER_ASSIGN, vec![this.addr() + 4, 0]),
                (NI_POINTER_ASSIGN, vec![this.addr() + 8, 0]),
                (NI_POINTER_DESTRUCT, vec![this.addr() + 8]),
                (NI_POINTER_DESTRUCT, vec![this.addr() + 4]),
            ]
        );
    }

    /// A `KFModel` holding `sequence`, whose name handle is `sequence_name`.
    fn kf_model_with(e: &mut Engine, sequence: Ptr, sequence_name: &str) -> Ptr<KFModel> {
        let this: Ptr<KFModel> = e.new_object();
        e.set(this, KFModel::spSequence, sequence);
        let name = text(e, sequence_name);
        e.mem.set_u32(sequence.addr() + 8, name);
        this
    }

    #[test]
    fn finishing_a_kf_model_without_a_sequence_does_nothing() {
        let (mut e, _) = kf_engine();
        let this: Ptr<KFModel> = e.new_object();
        let name = text(&mut e, "idle.kf");
        e.call_log = Some(vec![]);
        e.call(0x0043_b7e0, &args![this, name]);
        assert_eq!(
            call_order(&e),
            vec![0x0043_b7e0, NI_POINTER_GET],
            "only the NiPointer read"
        );
    }

    #[test]
    fn finishing_a_kf_model_renames_the_sequence_with_the_part_before_the_underscore() {
        let (mut e, sequence) = kf_engine();
        let this = kf_model_with(&mut e, sequence, "Walk_Fast");
        let original = e.mem.u32(sequence.addr() + 8);
        let name = text(&mut e, "walk.kf");
        e.call_log = Some(vec![]);
        e.call(0x0043_b7e0, &args![this, name]);
        // The first temporary string was built from the text "Walk"; the
        // second from the file name; the sequence ends up named by the latter.
        let built = calls_to(&e, FIXED_STRING_CONSTRUCT);
        assert_eq!(built.len(), 2);
        assert_eq!(e.mem.cstr(built[0][1]), b"Walk");
        assert_eq!(built[1][1], name);
        assert_eq!(e.mem.u32(sequence.addr() + 8), name);
        assert_ne!(e.mem.u32(sequence.addr() + 8), original);
        // The copy took the four characters before the underscore.
        assert_eq!(calls_to(&e, MEMORY_COPY)[0][1..], [original, 4]);
        // The temporaries were destroyed, the new handles referenced and the
        // old ones released.
        assert_eq!(calls_to(&e, FIXED_STRING_DESTRUCT).len(), 2);
        assert_eq!(calls_to(&e, FIXED_STRING_RELEASE).len(), 2);
        assert_eq!(e.mem.i32(STRING_COUNTER), 2);
        // The animation group was loaded from the file name and stored.
        assert_eq!(
            calls_to(&e, LOAD_ANIM_GROUP),
            vec![vec![sequence.addr(), name]]
        );
        assert_eq!(e.get(this, KFModel::spAnimGroup), Ptr::new(0x5555));
    }

    #[test]
    fn finishing_a_kf_model_with_a_plain_name_names_it_once() {
        let (mut e, sequence) = kf_engine();
        let this = kf_model_with(&mut e, sequence, "Walk");
        let name = text(&mut e, "walk.kf");
        e.call_log = Some(vec![]);
        e.call(0x0043_b7e0, &args![this, name]);
        assert_eq!(calls_to(&e, FIXED_STRING_CONSTRUCT).len(), 1);
        assert!(calls_to(&e, MEMORY_COPY).is_empty());
        assert_eq!(e.mem.u32(sequence.addr() + 8), name);
        // A sequence without a name is also named once.
        e.mem.set_u32(sequence.addr() + 8, 0);
        e.call_log = Some(vec![]);
        e.call(0x0043_b7e0, &args![this, name]);
        assert_eq!(calls_to(&e, FIXED_STRING_CONSTRUCT).len(), 1);
        assert!(calls_to(&e, STRCHR).is_empty());
    }

    #[test]
    fn finishing_a_kf_model_stores_the_group_in_sequences_of_the_checked_class() {
        let (mut e, sequence) = kf_engine();
        e.register(IS_KIND_OF, |_, _| 1u32.into_ret());
        let this = kf_model_with(&mut e, sequence, "Walk");
        let name = text(&mut e, "walk.kf");
        e.call_log = Some(vec![]);
        e.call(0x0043_b7e0, &args![this, name]);
        assert_eq!(
            calls_to(&e, IS_KIND_OF),
            vec![vec![sequence.addr(), TYPE_DESCRIPTOR_011C7D74]]
        );
        // The sequence's NiPointer at +0x74 holds the group.
        assert_eq!(e.mem.u32(sequence.addr() + 0x74), 0x5555);
        // The accessor at 007fa950 was asked for the model's sequence.
        assert_eq!(calls_to(&e, KF_MODEL_SEQUENCE), vec![vec![this.addr()]]);

        // Not of the class: the sequence is left alone.
        let (mut e, sequence) = kf_engine();
        let this = kf_model_with(&mut e, sequence, "Walk");
        let name = text(&mut e, "walk.kf");
        e.call(0x0043_b7e0, &args![this, name]);
        assert_eq!(e.mem.u32(sequence.addr() + 0x74), 0);
    }

    #[test]
    fn strchr_wrapper_passes_text_and_character() {
        let mut e = loader_engine();
        let word = text(&mut e, "ab_cd");
        assert_eq!(e.call(0x0043_b9d0, &args![word, 0x5fi32]).u32(), word + 2);
        assert_eq!(e.call(0x0043_b9d0, &args![word, 0x7ai32]).u32(), 0);
    }

    #[test]
    fn sequence_name_is_assigned_at_offset_8() {
        let (mut e, sequence) = kf_engine();
        let source = e.mem.alloc(8);
        e.mem.set_u32(source, 0xabc0);
        e.mem.set_u32(sequence.addr() + 8, 0xdef0);
        let back = e.call(
            0x0043_b9f0,
            &args![sequence, Ptr::<NiFixedString>::new(source)],
        );
        assert_eq!(back.u32(), sequence.addr() + 8);
        assert_eq!(e.mem.u32(sequence.addr() + 8), 0xabc0);
    }

    #[test]
    fn fixed_string_assignment_references_new_releases_old() {
        let mut e = loader_engine();
        let this = e.mem.alloc(8);
        let other = e.mem.alloc(8);
        e.mem.set_u32(this, 0x1000);
        e.mem.set_u32(other, 0x2000);
        e.call_log = Some(vec![]);
        let back = e.call(
            0x0043_ba10,
            &args![
                Ptr::<NiFixedString>::new(this),
                Ptr::<NiFixedString>::new(other)
            ],
        );
        assert_eq!(back.u32(), this);
        assert_eq!(e.mem.u32(this), 0x2000);
        // The reference is added first, then the old handle is released.
        let order = call_order(&e);
        let reference = order
            .iter()
            .position(|a| *a == INTERLOCKED_INCREMENT)
            .unwrap();
        let release = order
            .iter()
            .position(|a| *a == FIXED_STRING_RELEASE)
            .unwrap();
        assert!(reference < release);
        assert_eq!(calls_to(&e, FIXED_STRING_RELEASE), vec![vec![this]]);
        assert_eq!(e.mem.i32(STRING_COUNTER), 1);

        // Equal handles: nothing happens.
        e.call_log = Some(vec![]);
        e.call(
            0x0043_ba10,
            &args![
                Ptr::<NiFixedString>::new(this),
                Ptr::<NiFixedString>::new(this)
            ],
        );
        assert!(calls_to(&e, FIXED_STRING_RELEASE).is_empty());
        assert!(calls_to(&e, INTERLOCKED_INCREMENT).is_empty());
    }

    #[test]
    fn fixed_string_reference_skips_the_empty_string() {
        let mut e = loader_engine();
        e.mem.set_u32(EMPTY_FIXED_STRING, 0x7777);
        let handle = e.mem.alloc(8);
        e.mem.set_u32(handle, 0x7777);
        e.call_log = Some(vec![]);
        e.call(0x0043_ba60, &args![Ptr::<()>::new(handle)]);
        assert!(calls_to(&e, INTERLOCKED_INCREMENT).is_empty());
        e.mem.set_u32(handle, 0x8888);
        e.call(0x0043_ba60, &args![Ptr::<()>::new(handle)]);
        assert_eq!(
            calls_to(&e, INTERLOCKED_INCREMENT),
            vec![vec![STRING_COUNTER]]
        );
        assert_eq!(calls_to(&e, FIXED_STRING_REFCOUNT), vec![vec![handle]]);
    }

    #[test]
    fn sequence_group_is_assigned_at_offset_0x74() {
        let mut e = loader_engine();
        let sequence = e.mem.alloc(0x80);
        let back = e.call(
            0x0043_baa0,
            &args![Ptr::<()>::new(sequence), Ptr::<()>::new(0x6666)],
        );
        assert_eq!(back.u32(), sequence + 0x74);
        assert_eq!(e.mem.u32(sequence + 0x74), 0x6666);
    }

    // --- Queued files: `LoadedFile`, `QueuedTexture`, `QueuedModel`. ---

    /// A `BSFile`-like object's virtual table: slot 0 deletes, slot 0x18 is
    /// the file name.
    const FILE_VTABLE: u32 = 0x0ff2_0000;
    const FILE_DELETE: u32 = 0x0ff0_0100;
    const FILE_NAME_SLOT: u32 = 0x0ff0_0101;
    /// The message sink's table: slot 0x14 reports.
    const SINK_VTABLE: u32 = 0x0ff2_1000;
    const SINK_REPORT: u32 = 0x0ff0_0102;
    /// A task's table: slot 0x28 is `CheckFinished`.
    const TASK_VTABLE: u32 = 0x0ff2_2000;
    const CHECK_FINISHED: u32 = 0x0ff0_0103;
    /// The task queue's table: slot 0x48 adds a task.
    const QUEUE_VTABLE: u32 = 0x0ff2_3000;
    const QUEUE_ADD: u32 = 0x0ff0_0104;
    /// A texture's table: slot 0x8c is the hook `Run` calls.
    const TEXTURE_VTABLE: u32 = 0x0ff2_4000;
    const TEXTURE_HOOK: u32 = 0x0ff0_0105;
    /// Where the doubles for the texture map and object keep their state.
    const OWNER_OBJECT: u32 = 0x0ff3_0000;
    const MAP_OBJECT: u32 = 0x0ff3_1000;

    fn put_slots(e: &mut Engine, vtable: u32, slots: &[(u32, u32)]) {
        let mut table = vec![ANSWERS_NO; 0x80];
        for (offset, target) in slots {
            table[(*offset / 4) as usize] = *target;
        }
        e.put_vtable(vtable, &table);
    }

    /// Sets (`on`) or clears the bit `mask` of the byte at `address`, as the
    /// game's flag helpers do.
    fn write_flag(e: &mut Engine, address: u32, mask: u8, on: bool) {
        let byte = e.mem.u8(address);
        e.mem
            .set_u8(address, if on { byte | mask } else { byte & !mask });
    }

    /// An engine with doubles for what the queued-file functions call.
    fn queue_engine() -> Engine {
        let mut e = loader_engine();
        for page in [0x011c_3000, 0x011f_6000] {
            e.map(page, 0x1000);
        }
        // The exe's string the suffix test searches for.
        e.map(0x0101_6000, 0x1000);
        e.mem.set_cstr(ENVIRONMENT_MAP_SUFFIX, b"_e.dd");
        e.register(SET_FLAG_BIT_1, |e, a| {
            write_flag(e, a[1], 1, a[0] & 0xff != 0);
            Ret::default()
        });
        e.register(SET_FLAG_BIT_2, |e, a| {
            write_flag(e, a[1], 2, a[0] & 0xff != 0);
            Ret::default()
        });
        e.register(SET_FLAG_BIT_4, |e, a| {
            write_flag(e, a[1], 4, a[0] & 0xff != 0);
            Ret::default()
        });
        e.register(TEST_FLAG_BIT_2, |_, a| (a[0] & 2 != 0).into_ret());
        e.register(TEST_FLAG_BIT_4, |_, a| (a[0] & 4 != 0).into_ret());
        e.register(QUEUED_TEXTURE_SET_FLAG_1, |e, a| {
            write_flag(e, a[0] + 0x34, 1, a[1] != 0);
            Ret::default()
        });
        e.register(QUEUED_TEXTURE_TEST_FLAG_1, |e, a| {
            (e.mem.u8(a[0] + 0x34) & 1 != 0).into_ret()
        });
        e.register(QUEUED_FILE_ENTRY_GET_FILE_ENTRY, |e, a| {
            e.mem.u32(a[0] + 0x2c).into_ret()
        });
        e.register(QUEUED_FILE_ENTRY_GET_FILE_NAME, |e, a| {
            e.mem.u32(a[0] + 0x28).into_ret()
        });
        e.register(FILE_ENTRY_GET_SIZE, |e, a| {
            (e.mem.u32(a[0] + 8) & 0x3fff_ffff).into_ret()
        });
        e.register(TEXTURE_GET_SURFACE, |e, a| {
            e.mem.u32(a[0] + 0x24).into_ret()
        });
        e.register(STRSTR, |e, a| {
            let text = e.mem.cstr(a[0]);
            let pattern = e.mem.cstr(a[1]);
            match text.windows(pattern.len()).position(|w| w == &pattern[..]) {
                Some(i) => (a[0] + i as u32).into_ret(),
                None => 0u32.into_ret(),
            }
        });
        e.register(SPRINTF, |e, a| {
            e.mem.set_cstr(a[0], b"formatted");
            Ret::default()
        });
        e.register(NORMALIZE_PATH, |e, a| {
            let text = e.mem.cstr(a[0]);
            e.mem.set_cstr(a[1], &text);
            Ret::default()
        });
        for address in [
            QUEUED_FILE_ENTRY_CONSTRUCT,
            QUEUED_FILE_ENTRY_DESTRUCT,
            QUEUED_FILE_ENTRY_SET_FILE_NAME,
            QUEUED_FILE_ENTRY_FIND_FILE_ENTRY,
            QUEUED_FILE_ENTRY_SET_FILE_ENTRY,
            QUEUED_FILE_CANCEL,
            TASK_SET_DONE,
            MEMORY_CONTEXT_ENTER,
            MEMORY_CONTEXT_LEAVE,
            MODEL_POINTER_RELEASE,
            FILE_MAP_REMOVE_LOADED_FILE,
            FILE_MAP_REMOVE_QUEUED_TEXTURE,
            TEXTURE_PALETTE_SET_TEXTURE,
            TEXTURE_PALETTE_GET_TEXTURE_BY_NAME,
            FILE_DELETE,
            SINK_REPORT,
            CHECK_FINISHED,
            QUEUE_ADD,
            TEXTURE_HOOK,
        ] {
            e.register(address, |_, _| Ret::default());
        }
        e.register(FILE_MAP_ADD_LOADED_FILE, |_, _| 1u32.into_ret());
        e.register(FILE_MAP_ADD_QUEUED_TEXTURE, |_, _| 1u32.into_ret());
        e.register(QUEUED_FILE_ENTRY_GET_DESCRIPTION, |_, _| 1u32.into_ret());
        e.register(FILE_NAME_SLOT, |e, a| e.mem.u32(a[0] + 4).into_ret());
        e.register(TEXTURE_MAP_FIND, |_, _| 0u32.into_ret());
        e.register(CREATE_TEXTURE_FROM_FILE, |_, _| 0x7001u32.into_ret());
        e.register(CREATE_TEXTURE_FROM_FIXED_NAME, |_, _| 0x7002u32.into_ret());
        put_slots(
            &mut e,
            FILE_VTABLE,
            &[(0, FILE_DELETE), (0x18, FILE_NAME_SLOT)],
        );
        put_slots(&mut e, SINK_VTABLE, &[(0x14, SINK_REPORT)]);
        put_slots(&mut e, TASK_VTABLE, &[(0x28, CHECK_FINISHED)]);
        put_slots(&mut e, QUEUE_VTABLE, &[(0x48, QUEUE_ADD)]);
        put_slots(&mut e, TEXTURE_VTABLE, &[(0x8c, TEXTURE_HOOK)]);
        e
    }

    /// A queued texture whose first word is the task table (so slot `0x28`
    /// is `CheckFinished`).
    fn queued_texture(e: &mut Engine) -> Ptr<QueuedTexture> {
        let texture: Ptr<QueuedTexture> = e.new_object();
        e.mem.set_u32(texture.addr(), TASK_VTABLE);
        texture
    }

    /// A file entry with the given offset and size words.
    fn file_entry(e: &mut Engine, offset: u32, size: u32) -> Ptr {
        let entry = Ptr::new(e.mem.alloc(0x10));
        e.mem.set_u32(entry.addr() + 8, size);
        e.mem.set_u32(entry.addr() + 0xc, offset);
        entry
    }

    /// An open file whose name (slot `0x18`) is `name`.
    fn open_file(e: &mut Engine, name: &str) -> Ptr {
        let file = Ptr::new(e.mem.alloc(0x10));
        let text = text(e, name);
        e.mem.set_u32(file.addr(), FILE_VTABLE);
        e.mem.set_u32(file.addr() + 4, text);
        file
    }

    #[test]
    fn children_counter_is_decremented_only_when_positive() {
        let mut e = queue_engine();
        let children: Ptr<QueuedChildren> = e.new_object();
        for (before, after) in [(3, 2), (1, 0), (0, 0), (-5, -5)] {
            e.set(
                children,
                QueuedChildren::iNumChildrenFinished,
                before as u32,
            );
            e.call(0x0043_bac0, &args![children]);
            assert_eq!(
                e.get(children, QueuedChildren::iNumChildrenFinished) as i32,
                after
            );
        }
    }

    #[test]
    fn loaded_file_constructor_copies_the_name_and_joins_the_map() {
        let mut e = queue_engine();
        e.set_global(FILE_MAP_OWNER, OWNER_OBJECT);
        let this: Ptr<LoadedFile> = e.new_object();
        e.set(this, LoadedFile::iRefCount, 9);
        e.set(this, LoadedFile::bFileUsed, true);
        let name = text(&mut e, "meshes\\a.nif");
        e.call_log = Some(vec![]);
        let back = e.call(0x0043_baf0, &args![this, name, Ptr::<()>::new(0x4321)]);
        assert_eq!(back.ptr::<LoadedFile>(), this);
        let copy = e.get(this, LoadedFile::pFileName);
        assert_ne!(copy, name);
        assert_eq!(e.mem.cstr(copy), b"meshes\\a.nif");
        assert_eq!(e.get(this, LoadedFile::pFile), Ptr::new(0x4321));
        assert!(e.get(this, LoadedFile::bInLoadedFileMap));
        assert!(!e.get(this, LoadedFile::bFileUsed));
        assert_eq!(e.get(this, LoadedFile::iRefCount), 0);
        assert_eq!(
            calls_to(&e, FILE_MAP_ADD_LOADED_FILE),
            vec![vec![OWNER_OBJECT, copy, this.addr()]]
        );
        // A refused add leaves the flag clear.
        e.register(FILE_MAP_ADD_LOADED_FILE, |_, _| 0u32.into_ret());
        e.call(0x0043_baf0, &args![this, name, Ptr::<()>::new(0)]);
        assert!(!e.get(this, LoadedFile::bInLoadedFileMap));
    }

    /// A loaded file with a name, a file object and the given state.
    fn loaded_file(
        e: &mut Engine,
        in_map: bool,
        used: bool,
        references: i32,
    ) -> (Ptr<LoadedFile>, u32, Ptr) {
        let this: Ptr<LoadedFile> = e.new_object();
        let name = text(e, "a.nif");
        let file = open_file(e, "a.nif");
        e.set(this, LoadedFile::pFileName, name);
        e.set(this, LoadedFile::pFile, file);
        e.set(this, LoadedFile::bInLoadedFileMap, in_map);
        e.set(this, LoadedFile::bFileUsed, used);
        e.set(this, LoadedFile::iRefCount, references);
        (this, name, file)
    }

    #[test]
    fn loaded_file_destructor_reports_an_unused_mapped_file() {
        let mut e = queue_engine();
        e.set_global(FILE_MAP_OWNER, OWNER_OBJECT);
        let (this, name, file) = loaded_file(&mut e, true, false, 0);
        e.call_log = Some(vec![]);
        e.call(0x0043_bb80, &args![this]);
        assert_eq!(
            calls_to(&e, LOG),
            vec![vec![LOADED_FILE_UNUSED_MESSAGE, name, this.addr()]]
        );
        assert_eq!(
            calls_to(&e, FILE_MAP_REMOVE_LOADED_FILE),
            vec![vec![OWNER_OBJECT, name]]
        );
        // No references: nothing is formatted or reported.
        assert!(calls_to(&e, SPRINTF).is_empty());
        assert!(calls_to(&e, SINK_REPORT).is_empty());
        assert_eq!(calls_to(&e, MEMORY_FREE), vec![vec![name]]);
        // The file is deleted last, through slot 0 with the delete flag.
        assert_eq!(calls_to(&e, FILE_DELETE), vec![vec![file.addr(), 1]]);
        assert_eq!(*call_order(&e).last().unwrap(), FILE_DELETE);
    }

    #[test]
    fn loaded_file_destructor_reports_leftover_references_to_the_sink() {
        let mut e = queue_engine();
        e.set_global(FILE_MAP_OWNER, OWNER_OBJECT);
        let sink = Ptr::<()>::new(e.mem.alloc(8));
        e.mem.set_u32(sink.addr(), SINK_VTABLE);
        e.set_global(MESSAGE_SINK, sink.addr());
        // Used and not in the map: no complaint, no map removal.
        let (this, name, _) = loaded_file(&mut e, false, true, 2);
        e.call_log = Some(vec![]);
        e.call(0x0043_bb80, &args![this]);
        assert!(calls_to(&e, LOG).is_empty());
        assert!(calls_to(&e, FILE_MAP_REMOVE_LOADED_FILE).is_empty());
        let formats = calls_to(&e, SPRINTF);
        assert_eq!(formats.len(), 1);
        assert_eq!(
            formats[0][1..],
            [LOADED_FILE_REFERENCES_MESSAGE, name, this.addr(), 2]
        );
        let reports = calls_to(&e, SINK_REPORT);
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0][0], sink.addr());
        assert_eq!(reports[0][1], 4);
        assert_eq!(reports[0][2], formats[0][0]);
        assert_eq!(reports[0][3..], [0, 0, 0]);
        // Without a sink the references are formatted but not reported.
        e.set_global(MESSAGE_SINK, 0u32);
        let (this, _, _) = loaded_file(&mut e, false, true, 1);
        e.call_log = Some(vec![]);
        e.call(0x0043_bb80, &args![this]);
        assert_eq!(calls_to(&e, SPRINTF).len(), 1);
        assert!(calls_to(&e, SINK_REPORT).is_empty());
    }

    #[test]
    fn loaded_file_destructor_without_a_file_deletes_nothing() {
        let mut e = queue_engine();
        let (this, _, _) = loaded_file(&mut e, false, true, 0);
        e.set(this, LoadedFile::pFile, Ptr::NULL);
        e.call_log = Some(vec![]);
        e.call(0x0043_bb80, &args![this]);
        assert!(calls_to(&e, FILE_DELETE).is_empty());
        assert_eq!(calls_to(&e, MEMORY_FREE).len(), 1);
    }

    #[test]
    fn message_sink_is_the_word_at_011f6388() {
        let mut e = queue_engine();
        assert!(!e.call(0x0043_bce0, &args![]).bool());
        assert_eq!(e.call(0x0043_bd00, &args![]).u32(), 0);
        e.set_global(MESSAGE_SINK, 0x1234u32);
        assert!(e.call(0x0043_bce0, &args![]).bool());
        assert_eq!(e.call(0x0043_bd00, &args![]).u32(), 0x1234);
    }

    #[test]
    fn texture_constructor_by_name_normalizes_the_path() {
        let mut e = queue_engine();
        let this: Ptr<QueuedTexture> = e.new_object();
        e.set(this, QueuedTexture::cFlags, 0xff);
        let name = text(&mut e, "textures\\a.dds");
        e.call_log = Some(vec![]);
        let back = e.call(0x0043_bd10, &args![this, name, 0x77u32]);
        assert_eq!(back.ptr::<QueuedTexture>(), this);
        assert_eq!(e.mem.u32(this.addr()), QUEUED_TEXTURE_VTABLE);
        assert_eq!(e.get(this, QueuedTexture::cFlags), 0);
        assert_eq!(
            calls_to(&e, QUEUED_FILE_ENTRY_CONSTRUCT),
            vec![vec![this.addr(), 0x77]]
        );
        assert_eq!(
            calls_to(&e, NI_POINTER_CONSTRUCT),
            vec![vec![this.addr() + 0x30, 0]]
        );
        let normalized = calls_to(&e, NORMALIZE_PATH);
        assert_eq!(normalized.len(), 1);
        assert_eq!(normalized[0][0], name);
        assert_eq!(normalized[0][2], 0x104);
        // The normalized buffer is what becomes the file name.
        assert_eq!(
            calls_to(&e, QUEUED_FILE_ENTRY_SET_FILE_NAME),
            vec![vec![this.addr(), normalized[0][1]]]
        );
        assert_eq!(
            calls_to(&e, QUEUED_FILE_ENTRY_FIND_FILE_ENTRY),
            vec![vec![this.addr(), 1]]
        );
    }

    #[test]
    fn child_notification_counts_and_asks_whether_finished() {
        let mut e = queue_engine();
        let this = Ptr::<QueuedFile>::new(e.mem.alloc(0x28));
        e.mem.set_u32(this.addr(), TASK_VTABLE);
        let children: Ptr<QueuedChildren> = e.new_object();
        e.set(this, QueuedFile::pChildren, children.cast());
        e.call_log = Some(vec![]);
        e.call(0x0043_bde0, &args![this, Ptr::<()>::new(0x99)]);
        assert_eq!(e.get(children, QueuedChildren::iNumChildrenFinished), 1);
        assert_eq!(calls_to(&e, CHECK_FINISHED), vec![vec![this.addr()]]);
    }

    #[test]
    fn children_counter_is_incremented() {
        let mut e = queue_engine();
        let children: Ptr<QueuedChildren> = e.new_object();
        e.call(0x0043_be10, &args![children]);
        e.call(0x0043_be10, &args![children]);
        assert_eq!(e.get(children, QueuedChildren::iNumChildrenFinished), 2);
    }

    #[test]
    fn texture_scalar_deleting_destructor_frees_only_on_request() {
        let mut e = queue_engine();
        let this = queued_texture(&mut e);
        e.call_log = Some(vec![]);
        let back = e.call(0x0043_be30, &args![this, 0u32]);
        assert_eq!(back.ptr::<QueuedTexture>(), this);
        assert_eq!(
            call_order(&e),
            vec![0x0043_be30, NI_POINTER_DESTRUCT, QUEUED_FILE_ENTRY_DESTRUCT]
        );
        e.call_log = Some(vec![]);
        e.call(0x0043_be30, &args![this, 3u32]);
        assert_eq!(calls_to(&e, MEMORY_FREE), vec![vec![this.addr()]]);
        // Only bit 0 of the flags counts.
        e.call_log = Some(vec![]);
        e.call(0x0043_be30, &args![this, 2u32]);
        assert!(calls_to(&e, MEMORY_FREE).is_empty());
    }

    #[test]
    fn texture_constructor_by_file_entry_stores_the_entry() {
        let mut e = queue_engine();
        let this: Ptr<QueuedTexture> = e.new_object();
        e.set(this, QueuedTexture::cFlags, 0xff);
        e.call_log = Some(vec![]);
        let back = e.call(0x0043_be60, &args![this, Ptr::<()>::new(0x5150), 0x66u32]);
        assert_eq!(back.ptr::<QueuedTexture>(), this);
        assert_eq!(e.mem.u32(this.addr()), QUEUED_TEXTURE_VTABLE);
        assert_eq!(e.get(this, QueuedTexture::cFlags), 0);
        assert_eq!(
            calls_to(&e, QUEUED_FILE_ENTRY_CONSTRUCT),
            vec![vec![this.addr(), 0x66]]
        );
        assert_eq!(
            calls_to(&e, QUEUED_FILE_ENTRY_SET_FILE_ENTRY),
            vec![vec![this.addr(), 0x5150]]
        );
        assert!(calls_to(&e, QUEUED_FILE_ENTRY_SET_FILE_NAME).is_empty());
    }

    #[test]
    fn texture_constructor_by_texture_marks_the_task_done() {
        let mut e = queue_engine();
        let this: Ptr<QueuedTexture> = e.new_object();
        e.call_log = Some(vec![]);
        let back = e.call(0x0043_bef0, &args![this, Ptr::<()>::new(0x4040), 0x66u32]);
        assert_eq!(back.ptr::<QueuedTexture>(), this);
        assert_eq!(e.mem.u32(this.addr()), QUEUED_TEXTURE_VTABLE);
        assert_eq!(e.get(this, QueuedTexture::spTexture), Ptr::new(0x4040));
        assert_eq!(e.get(this, QueuedTexture::cFlags), 0);
        let order = call_order(&e);
        // Constructed empty first, then assigned, then marked done.
        let construct = order
            .iter()
            .position(|a| *a == NI_POINTER_CONSTRUCT)
            .unwrap();
        let assign = order.iter().position(|a| *a == NI_POINTER_ASSIGN).unwrap();
        let done = order.iter().position(|a| *a == TASK_SET_DONE).unwrap();
        assert!(construct < assign && assign < done);
        assert_eq!(calls_to(&e, TASK_SET_DONE), vec![vec![this.addr()]]);
    }

    #[test]
    fn texture_destructor_restores_the_table_and_runs_the_base() {
        let mut e = queue_engine();
        let this: Ptr<QueuedTexture> = e.new_object();
        e.mem.set_u32(this.addr(), 0x1111);
        e.call_log = Some(vec![]);
        e.call(0x0043_bf80, &args![this]);
        assert_eq!(e.mem.u32(this.addr()), QUEUED_TEXTURE_VTABLE);
        assert_eq!(
            call_order(&e)[1..],
            [NI_POINTER_DESTRUCT, QUEUED_FILE_ENTRY_DESTRUCT]
        );
        assert_eq!(
            calls_to(&e, NI_POINTER_DESTRUCT),
            vec![vec![this.addr() + 0x30]]
        );
    }

    #[test]
    fn file_index_is_the_archive_word_plus_four() {
        let mut e = queue_engine();
        let this = queued_texture(&mut e);
        // No file entry.
        assert_eq!(e.call(0x0043_bfe0, &args![this]).u32(), 0);
        let entry = file_entry(&mut e, 0, 0);
        e.set(this, QueuedTexture::pFileEntry, entry);
        // A file entry without an archive.
        e.register(FILE_ENTRY_FIND_ARCHIVE, |_, _| 0u32.into_ret());
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x0043_bfe0, &args![this]).u32(), 0);
        assert_eq!(
            calls_to(&e, FILE_ENTRY_FIND_ARCHIVE),
            vec![vec![entry.addr(), 2]]
        );
        // With an archive.
        let archive = e.mem.alloc(0x200);
        e.mem.set_u32(archive + 0x1c8, 11);
        e.register_double(FILE_ENTRY_FIND_ARCHIVE, move |_, _| archive.into_ret());
        assert_eq!(e.call(0x0043_bfe0, &args![this]).u32(), 15);
    }

    #[test]
    fn archive_word_is_read_at_0x1c8() {
        let mut e = queue_engine();
        let archive = e.mem.alloc(0x200);
        e.mem.set_u32(archive + 0x1c8, 0xabcd);
        assert_eq!(
            e.call(0x0043_c030, &args![Ptr::<()>::new(archive)]).u32(),
            0xabcd
        );
    }

    /// A task queue object for `QueueMe`.
    fn task_queue(e: &mut Engine) -> u32 {
        let queue = e.mem.alloc(8);
        e.mem.set_u32(queue, QUEUE_VTABLE);
        e.set_global(TASK_QUEUE, queue);
        queue
    }

    #[test]
    fn queue_me_registers_the_file_entry_and_sets_the_thread_flag_bit() {
        let mut e = queue_engine();
        e.set_global(FILE_MAP_OWNER, OWNER_OBJECT);
        let queue = task_queue(&mut e);
        let this = queued_texture(&mut e);
        let entry = file_entry(&mut e, 0, 0);
        e.set(this, QueuedTexture::pFileEntry, entry);
        let tls = e.tls();
        e.mem.set_u8(tls + TLS_QUEUED_FLAG, 1);
        e.call_log = Some(vec![]);
        e.call(0x0043_c050, &args![this]);
        assert_eq!(
            calls_to(&e, FILE_MAP_ADD_QUEUED_TEXTURE),
            vec![vec![OWNER_OBJECT, entry.addr(), this.addr()]]
        );
        // Added to the map (bit 4) and queued with the thread flag set (bit 2).
        assert_eq!(e.get(this, QueuedTexture::cFlags), 6);
        assert_eq!(calls_to(&e, QUEUE_ADD), vec![vec![queue, this.addr()]]);
    }

    #[test]
    fn queue_me_without_entry_or_thread_flag_only_queues() {
        let mut e = queue_engine();
        e.set_global(FILE_MAP_OWNER, OWNER_OBJECT);
        task_queue(&mut e);
        let this = queued_texture(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x0043_c050, &args![this]);
        assert!(calls_to(&e, FILE_MAP_ADD_QUEUED_TEXTURE).is_empty());
        assert_eq!(e.get(this, QueuedTexture::cFlags), 0);
        assert_eq!(calls_to(&e, QUEUE_ADD).len(), 1);
        // A refused add clears bit 4.
        let entry = file_entry(&mut e, 0, 0);
        e.set(this, QueuedTexture::pFileEntry, entry);
        e.set(this, QueuedTexture::cFlags, 4);
        e.register(FILE_MAP_ADD_QUEUED_TEXTURE, |_, _| 0u32.into_ret());
        e.call(0x0043_c050, &args![this]);
        assert_eq!(e.get(this, QueuedTexture::cFlags), 0);
    }

    #[test]
    fn texture_flag_bit_2_is_set_and_cleared() {
        let mut e = queue_engine();
        let this = queued_texture(&mut e);
        e.set(this, QueuedTexture::cFlags, 0x01);
        e.call(0x0043_c0d0, &args![this, 1u8]);
        assert_eq!(e.get(this, QueuedTexture::cFlags), 0x03);
        e.call(0x0043_c0d0, &args![this, 0u8]);
        assert_eq!(e.get(this, QueuedTexture::cFlags), 0x01);
    }

    #[test]
    fn texture_flag_bit_4_is_set_and_cleared() {
        let mut e = queue_engine();
        let this = queued_texture(&mut e);
        e.set(this, QueuedTexture::cFlags, 0x02);
        e.call(0x0043_c100, &args![this, 1u8]);
        assert_eq!(e.get(this, QueuedTexture::cFlags), 0x06);
        e.call(0x0043_c100, &args![this, 0u8]);
        assert_eq!(e.get(this, QueuedTexture::cFlags), 0x02);
    }

    #[test]
    fn thread_flag_is_the_tls_byte_at_0x25c() {
        let mut e = queue_engine();
        assert_eq!(e.call(0x0043_c130, &args![]).u8(), 0);
        let tls = e.tls();
        e.mem.set_u8(tls + 0x25c, 7);
        assert_eq!(e.call(0x0043_c130, &args![]).u8(), 7);
    }

    /// Doubles and objects for `Run`: the texture map finds nothing unless
    /// told otherwise, the memory context is a guard of 4 bytes.
    fn run_scene(e: &mut Engine) -> Ptr<QueuedTexture> {
        e.set_global(TEXTURE_MAP, MAP_OBJECT);
        e.set_global(DEFAULT_TEXTURE_SETTING, 0x5a5a_u32);
        let this = queued_texture(e);
        e.set(this, QueuedTexture::eContext, 0x55);
        this
    }

    #[test]
    fn run_keeps_a_texture_the_palette_already_has() {
        let mut e = queue_engine();
        let this = run_scene(&mut e);
        let entry = file_entry(&mut e, 0, 0);
        e.set(this, QueuedTexture::pFileEntry, entry);
        // The texture has a surface whose format byte is 3.
        let texture = e.mem.alloc(0x40);
        e.mem.set_u32(texture, TEXTURE_VTABLE);
        let surface = e.mem.alloc(0x80);
        e.mem.set_u32(texture + 0x24, surface);
        e.mem.set_u8(surface + 0x58 + 3, 3);
        e.register_double(TEXTURE_MAP_FIND, move |e, a| {
            e.mem.set_u32(a[3], texture);
            1u32.into_ret()
        });
        e.call_log = Some(vec![]);
        e.call(0x0043_c150, &args![this]);
        // The guard is entered with the task's context and left again.
        let enter = calls_to(&e, MEMORY_CONTEXT_ENTER);
        assert_eq!(enter.len(), 1);
        assert_eq!(enter[0][1..], [0x55, 1, MODEL_LOADER_SOURCE, 0x2b7]);
        assert_eq!(calls_to(&e, MEMORY_CONTEXT_LEAVE), vec![vec![enter[0][0]]]);
        // Looked up by file entry in the bucket of the entry's address.
        let bucket = (entry.addr() >> 4) % 1001;
        assert_eq!(
            calls_to(&e, TEXTURE_MAP_FIND),
            vec![vec![MAP_OBJECT, bucket, entry.addr(), this.addr() + 0x30]]
        );
        assert_eq!(e.get(this, QueuedTexture::spTexture), Ptr::new(texture));
        // Flag bit 1 is set; the format byte 3 with bit 2 clear calls the hook.
        assert_eq!(
            calls_to(&e, QUEUED_TEXTURE_SET_FLAG_1),
            vec![vec![this.addr(), 1]]
        );
        assert_eq!(calls_to(&e, TEXTURE_HOOK), vec![vec![texture]]);
        // Nothing was loaded from a file.
        assert!(calls_to(&e, QUEUED_FILE_ENTRY_GET_FILE).is_empty());
        // With bit 2 set the hook is not called.
        e.set(this, QueuedTexture::cFlags, 2);
        e.call_log = Some(vec![]);
        e.call(0x0043_c150, &args![this]);
        assert!(calls_to(&e, TEXTURE_HOOK).is_empty());
        // Nor with another format byte.
        e.set(this, QueuedTexture::cFlags, 0);
        e.mem.set_u8(surface + 0x58 + 3, 4);
        e.call(0x0043_c150, &args![this]);
        assert!(calls_to(&e, TEXTURE_HOOK).is_empty());
    }

    #[test]
    fn run_looks_a_texture_up_by_name_without_a_file_entry() {
        let mut e = queue_engine();
        let this = run_scene(&mut e);
        let name = text(&mut e, "a.dds");
        e.set(this, QueuedTexture::pFileName, name);
        let texture = e.mem.alloc(0x40);
        e.mem.set_u32(texture, TEXTURE_VTABLE);
        e.register_double(TEXTURE_PALETTE_GET_TEXTURE_BY_NAME, move |e, a| {
            e.mem.set_u32(a[1], texture);
            Ret::default()
        });
        e.call_log = Some(vec![]);
        e.call(0x0043_c150, &args![this]);
        assert_eq!(
            calls_to(&e, TEXTURE_PALETTE_GET_TEXTURE_BY_NAME),
            vec![vec![name, this.addr() + 0x30]]
        );
        assert!(calls_to(&e, TEXTURE_MAP_FIND).is_empty());
        assert_eq!(e.get(this, QueuedTexture::spTexture), Ptr::new(texture));
        // A texture without a surface: no hook.
        assert!(calls_to(&e, TEXTURE_HOOK).is_empty());
    }

    #[test]
    fn run_loads_a_plain_texture_from_its_file() {
        let mut e = queue_engine();
        let this = run_scene(&mut e);
        let entry = file_entry(&mut e, 0, 0);
        e.set(this, QueuedTexture::pFileEntry, entry);
        let file = open_file(&mut e, "textures\\wall.dds");
        let name = e.mem.u32(file.addr() + 4);
        e.register(QUEUED_FILE_ENTRY_GET_FILE, |e, a| {
            // The task's file: the one stored in the scene.
            e.mem.u32(a[0] + 0x10).into_ret()
        });
        e.mem.set_u32(this.addr() + 0x10, file.addr());
        // The thread value is 9 before and 1 while the file is opened with
        // bit 2 set; the creator sees it.
        let tls = e.tls();
        e.mem.set_u32(tls + TLS_QUEUED_VALUE, 9);
        e.set(this, QueuedTexture::cFlags, 2);
        e.register_double(CREATE_TEXTURE_FROM_FILE, move |e, _| {
            let tls = e.tls();
            assert_eq!(e.mem.u32(tls + TLS_QUEUED_VALUE), 1);
            0x7001u32.into_ret()
        });
        e.call_log = Some(vec![]);
        e.call(0x0043_c150, &args![this]);
        assert_eq!(
            calls_to(&e, QUEUED_FILE_ENTRY_GET_FILE),
            vec![vec![this.addr(), 1, 2]]
        );
        assert_eq!(
            calls_to(&e, CREATE_TEXTURE_FROM_FILE),
            vec![vec![file.addr(), name, TEXTURE_FORMAT_PREFERENCES, 1]]
        );
        assert!(calls_to(&e, CREATE_TEXTURE_FROM_FIXED_NAME).is_empty());
        assert_eq!(e.get(this, QueuedTexture::spTexture), Ptr::new(0x7001));
        // The thread value is back to what it was.
        assert_eq!(e.mem.u32(tls + TLS_QUEUED_VALUE), 9);
    }

    #[test]
    fn run_loads_an_environment_map_through_a_fixed_string() {
        let mut e = queue_engine();
        let this = run_scene(&mut e);
        let entry = file_entry(&mut e, 0, 0);
        e.set(this, QueuedTexture::pFileEntry, entry);
        let file = open_file(&mut e, "textures\\sky_e.dds");
        let name = e.mem.u32(file.addr() + 4);
        e.register(QUEUED_FILE_ENTRY_GET_FILE, |e, a| {
            e.mem.u32(a[0] + 0x10).into_ret()
        });
        e.mem.set_u32(this.addr() + 0x10, file.addr());
        let tls = e.tls();
        e.mem.set_u32(tls + TLS_QUEUED_VALUE, 9);
        e.call_log = Some(vec![]);
        e.call(0x0043_c150, &args![this]);
        // The fixed string is built from the name and destroyed afterwards.
        let constructed = calls_to(&e, FIXED_STRING_CONSTRUCT);
        assert_eq!(constructed.len(), 1);
        assert_eq!(constructed[0][1], name);
        let local = constructed[0][0];
        assert_eq!(
            calls_to(&e, CREATE_TEXTURE_FROM_FIXED_NAME),
            vec![vec![local, file.addr(), 0x5a5a, TEXTURE_FORMAT_PREFERENCES]]
        );
        assert_eq!(calls_to(&e, FIXED_STRING_DESTRUCT), vec![vec![local]]);
        assert!(calls_to(&e, CREATE_TEXTURE_FROM_FILE).is_empty());
        assert_eq!(e.get(this, QueuedTexture::spTexture), Ptr::new(0x7002));
        // Bit 2 was clear, so the thread value was never changed.
        assert_eq!(e.mem.u32(tls + TLS_QUEUED_VALUE), 9);
    }

    #[test]
    fn run_logs_why_no_file_could_be_opened() {
        let mut e = queue_engine();
        let this = run_scene(&mut e);
        e.register(QUEUED_FILE_ENTRY_GET_FILE, |_, _| 0u32.into_ret());
        // With a file entry: its offset (top bit masked) and size (top two
        // bits masked).
        let entry = file_entry(&mut e, 0x8000_1234, 0xc000_0777);
        e.set(this, QueuedTexture::pFileEntry, entry);
        e.call_log = Some(vec![]);
        e.call(0x0043_c150, &args![this]);
        assert_eq!(
            calls_to(&e, LOG),
            vec![vec![NO_FILE_FOR_TEXTURE_ENTRY_MESSAGE, 0x1234, 0x777]]
        );
        // With only a name.
        let this = run_scene(&mut e);
        let name = text(&mut e, "missing.dds");
        e.set(this, QueuedTexture::pFileName, name);
        e.call_log = Some(vec![]);
        e.call(0x0043_c150, &args![this]);
        assert_eq!(
            calls_to(&e, LOG),
            vec![vec![NO_FILE_FOR_TEXTURE_NAME_MESSAGE, name]]
        );
        // With neither: silence.
        let this = run_scene(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x0043_c150, &args![this]);
        assert!(calls_to(&e, LOG).is_empty());
        assert_eq!(calls_to(&e, MEMORY_CONTEXT_LEAVE).len(), 1);
    }

    #[test]
    fn file_entry_offset_drops_the_top_bit() {
        let mut e = queue_engine();
        let entry = file_entry(&mut e, 0xffff_ffff, 0);
        assert_eq!(e.call(0x0043_c3b0, &args![entry]).u32(), 0x7fff_ffff);
    }

    #[test]
    fn texture_flag_bit_2_test() {
        let mut e = queue_engine();
        let this = queued_texture(&mut e);
        for (flags, expected) in [(0, false), (1, false), (2, true), (7, true), (4, false)] {
            e.set(this, QueuedTexture::cFlags, flags);
            assert_eq!(e.call(0x0043_c3d0, &args![this]).bool(), expected);
        }
    }

    #[test]
    fn thread_value_is_stored_and_read_at_0x29c() {
        let mut e = queue_engine();
        e.call(0x0043_c3f0, &args![0x1234u32]);
        let tls = e.tls();
        assert_eq!(e.mem.u32(tls + 0x29c), 0x1234);
        assert_eq!(e.call(0x0043_c410, &args![]).u32(), 0x1234);
    }

    #[test]
    fn surface_format_byte_is_zero_without_a_surface() {
        let mut e = queue_engine();
        let texture = e.mem.alloc(0x40);
        assert_eq!(
            e.call(0x0043_c430, &args![Ptr::<()>::new(texture)]).u32(),
            0
        );
        let surface = e.mem.alloc(0x80);
        e.mem.set_u32(texture + 0x24, surface);
        e.mem.set_u8(surface + 0x58 + 3, 0x2a);
        e.call_log = Some(vec![]);
        assert_eq!(
            e.call(0x0043_c430, &args![Ptr::<()>::new(texture)]).u32(),
            0x2a
        );
        // The surface getter is asked twice, as the game does.
        assert_eq!(calls_to(&e, TEXTURE_GET_SURFACE).len(), 2);
    }

    #[test]
    fn byte_at_offset_3_is_read() {
        let mut e = queue_engine();
        let block = e.mem.alloc(8);
        e.mem.set_u8(block + 3, 0x42);
        assert_eq!(
            e.call(0x0043_c470, &args![Ptr::<()>::new(block)]).u8(),
            0x42
        );
    }

    #[test]
    fn descriptor_address_is_surface_plus_0x58() {
        let mut e = queue_engine();
        assert_eq!(
            e.call(0x0043_c490, &args![Ptr::<()>::new(0x1000)]).u32(),
            0x1058
        );
    }

    #[test]
    fn default_texture_setting_is_the_word_at_011f4748() {
        let mut e = queue_engine();
        e.set_global(DEFAULT_TEXTURE_SETTING, 0xfeedu32);
        assert_eq!(e.call(0x0043_c4b0, &args![]).u32(), 0xfeed);
    }

    #[test]
    fn palette_get_texture_clears_the_output_and_searches_the_map() {
        let mut e = queue_engine();
        e.set_global(TEXTURE_MAP, MAP_OBJECT);
        let out = e.mem.alloc(4);
        e.mem.set_u32(out, 0xdead);
        e.register_double(TEXTURE_MAP_FIND, |_, _| 0u32.into_ret());
        e.call_log = Some(vec![]);
        e.call(0x0043_c4c0, &args![0x0001_2340u32, Ptr::<()>::new(out)]);
        assert_eq!(e.mem.u32(out), 0);
        assert_eq!(call_order(&e)[1..], [NI_POINTER_ASSIGN, TEXTURE_MAP_FIND]);
        assert_eq!(
            calls_to(&e, TEXTURE_MAP_FIND),
            vec![vec![MAP_OBJECT, 0x1234 % 1001, 0x0001_2340, out]]
        );
    }

    #[test]
    fn palette_lookup_returns_what_the_map_says() {
        let mut e = queue_engine();
        let out = e.mem.alloc(4);
        for found in [false, true] {
            e.register_double(TEXTURE_MAP_FIND, move |_, _| found.into_ret());
            let back = e.call(
                0x0043_c4f0,
                &args![Ptr::<()>::new(MAP_OBJECT), 0x100u32, Ptr::<()>::new(out)],
            );
            assert_eq!(back.bool(), found);
        }
    }

    #[test]
    fn palette_bucket_is_the_key_over_16_modulo_1001() {
        let mut e = queue_engine();
        for key in [0u32, 15, 16, 0x1000, 0x0123_4560, 0xffff_ffff] {
            assert_eq!(e.call(0x0043_c530, &args![key]).u32(), (key >> 4) % 1001);
        }
        assert_eq!(e.call(0x0043_c530, &args![1001u32 * 16]).u32(), 0);
    }

    #[test]
    fn finish_stores_the_texture_in_the_palette_by_file_entry() {
        let mut e = queue_engine();
        e.set_global(FILE_MAP_OWNER, OWNER_OBJECT);
        let this = queued_texture(&mut e);
        let entry = file_entry(&mut e, 0, 0);
        e.set(this, QueuedTexture::pFileEntry, entry);
        e.set(this, QueuedTexture::spTexture, Ptr::new(0x6001));
        // Bit 4: the texture is in the map of queued textures.
        e.set(this, QueuedTexture::cFlags, 4);
        e.call_log = Some(vec![]);
        e.call(0x0043_c550, &args![this]);
        assert_eq!(
            calls_to(&e, TEXTURE_PALETTE_SET_TEXTURE),
            vec![vec![0x6001, entry.addr()]]
        );
        // Taken out of the queued map, bit 4 cleared, then CheckFinished.
        assert_eq!(
            calls_to(&e, FILE_MAP_REMOVE_QUEUED_TEXTURE),
            vec![vec![OWNER_OBJECT, entry.addr()]]
        );
        assert_eq!(e.get(this, QueuedTexture::cFlags), 0);
        assert_eq!(*call_order(&e).last().unwrap(), CHECK_FINISHED);
        assert_eq!(calls_to(&e, CHECK_FINISHED), vec![vec![this.addr()]]);
    }

    #[test]
    fn finish_stores_a_name_only_texture_without_an_entry() {
        let mut e = queue_engine();
        let this = queued_texture(&mut e);
        let name = text(&mut e, "a.dds");
        e.set(this, QueuedTexture::pFileName, name);
        e.set(this, QueuedTexture::spTexture, Ptr::new(0x6002));
        e.call_log = Some(vec![]);
        e.call(0x0043_c550, &args![this]);
        assert_eq!(
            calls_to(&e, TEXTURE_PALETTE_SET_TEXTURE),
            vec![vec![0x6002, 0]]
        );
        assert!(calls_to(&e, FILE_MAP_REMOVE_QUEUED_TEXTURE).is_empty());
        assert_eq!(calls_to(&e, CHECK_FINISHED).len(), 1);
    }

    #[test]
    fn finish_skips_the_palette_when_flag_bit_1_is_set_or_no_texture() {
        let mut e = queue_engine();
        let this = queued_texture(&mut e);
        let entry = file_entry(&mut e, 0, 0);
        e.set(this, QueuedTexture::pFileEntry, entry);
        e.set(this, QueuedTexture::spTexture, Ptr::new(0x6003));
        e.set(this, QueuedTexture::cFlags, 1);
        e.call_log = Some(vec![]);
        e.call(0x0043_c550, &args![this]);
        assert!(calls_to(&e, TEXTURE_PALETTE_SET_TEXTURE).is_empty());
        // No texture at all.
        e.set(this, QueuedTexture::cFlags, 0);
        e.set(this, QueuedTexture::spTexture, Ptr::NULL);
        e.call(0x0043_c550, &args![this]);
        assert!(calls_to(&e, TEXTURE_PALETTE_SET_TEXTURE).is_empty());
        // Bit 4 clear: the queued map is left alone, but CheckFinished runs.
        assert!(calls_to(&e, FILE_MAP_REMOVE_QUEUED_TEXTURE).is_empty());
        assert_eq!(calls_to(&e, CHECK_FINISHED).len(), 2);
    }

    #[test]
    fn check_finished_slot_is_called() {
        let mut e = queue_engine();
        let this = queued_texture(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x0043_c610, &args![this]);
        assert_eq!(calls_to(&e, CHECK_FINISHED), vec![vec![this.addr()]]);
    }

    #[test]
    fn texture_flag_bit_4_test() {
        let mut e = queue_engine();
        let this = queued_texture(&mut e);
        for (flags, expected) in [(0, false), (2, false), (4, true), (7, true)] {
            e.set(this, QueuedTexture::cFlags, flags);
            assert_eq!(e.call(0x0043_c630, &args![this]).bool(), expected);
        }
    }

    #[test]
    fn cancel_runs_the_base_and_leaves_the_queued_map() {
        let mut e = queue_engine();
        e.set_global(FILE_MAP_OWNER, OWNER_OBJECT);
        let this = queued_texture(&mut e);
        let entry = file_entry(&mut e, 0, 0);
        e.set(this, QueuedTexture::pFileEntry, entry);
        e.set(this, QueuedTexture::cFlags, 4);
        e.call_log = Some(vec![]);
        e.call(0x0043_c650, &args![this, 0x11u32, 0x22u32]);
        assert_eq!(
            calls_to(&e, QUEUED_FILE_CANCEL),
            vec![vec![this.addr(), 0x11, 0x22]]
        );
        assert_eq!(
            calls_to(&e, FILE_MAP_REMOVE_QUEUED_TEXTURE),
            vec![vec![OWNER_OBJECT, entry.addr()]]
        );
        assert_eq!(e.get(this, QueuedTexture::cFlags), 0);
        // Unlike `Finish`, nothing asks whether the task is finished.
        assert!(calls_to(&e, CHECK_FINISHED).is_empty());
        // Not in the map: left alone.
        e.call_log = Some(vec![]);
        e.call(0x0043_c650, &args![this, 0u32, 0u32]);
        assert!(calls_to(&e, FILE_MAP_REMOVE_QUEUED_TEXTURE).is_empty());
    }

    #[test]
    fn texture_description_passes_the_word_texture() {
        let mut e = queue_engine();
        let this = queued_texture(&mut e);
        e.call_log = Some(vec![]);
        let back = e.call(0x0043_c6b0, &args![this, 0x9000u32, 0x400u32]);
        assert!(back.bool());
        assert_eq!(
            calls_to(&e, QUEUED_FILE_ENTRY_GET_DESCRIPTION),
            vec![vec![this.addr(), 0x9000, 0x400, TEXTURE_WORD]]
        );
    }

    #[test]
    fn model_constructor_sets_up_the_fields_and_flags() {
        let mut e = queue_engine();
        let this: Ptr<QueuedModel> = e.new_object();
        e.set(this, QueuedModel::pTESModel, Ptr::new(0x1111));
        e.set(this, QueuedModel::mfOverriddenVisualDistance, 5.0);
        e.set(this, QueuedModel::cFlags, 0xff);
        let name = text(&mut e, "meshes\\a.nif");
        e.call_log = Some(vec![]);
        let back = e.call(0x0043_c6e0, &args![this, name, 0x77u32, 0x3u32, 1u8, 0u8]);
        assert_eq!(back.ptr::<QueuedModel>(), this);
        assert_eq!(e.mem.u32(this.addr()), QUEUED_MODEL_VTABLE);
        assert_eq!(e.get(this, QueuedModel::spModel), Ptr::NULL);
        assert_eq!(e.get(this, QueuedModel::pTESModel), Ptr::NULL);
        assert_eq!(e.get(this, QueuedModel::eLODFadeMult), 3);
        // Flag argument 1 sets bit 1, argument 2 (zero) clears bit 2.
        assert_eq!(e.get(this, QueuedModel::cFlags), 1);
        assert_eq!(e.get(this, QueuedModel::mfOverriddenVisualDistance), 0.0);
        assert_eq!(
            calls_to(&e, QUEUED_FILE_ENTRY_CONSTRUCT),
            vec![vec![this.addr(), 0x77]]
        );
        assert_eq!(
            calls_to(&e, QUEUED_FILE_ENTRY_SET_FILE_NAME),
            vec![vec![this.addr(), name]]
        );
        assert_eq!(
            calls_to(&e, QUEUED_FILE_ENTRY_FIND_FILE_ENTRY),
            vec![vec![this.addr(), 0]]
        );
        // The other combination.
        e.call(0x0043_c6e0, &args![this, name, 0x77u32, 0u32, 0u8, 1u8]);
        assert_eq!(e.get(this, QueuedModel::cFlags), 2);
    }

    #[test]
    fn model_flag_bits_1_and_2_are_set_and_cleared() {
        let mut e = queue_engine();
        let this: Ptr<QueuedModel> = e.new_object();
        e.set(this, QueuedModel::cFlags, 0x04);
        e.call(0x0043_c7a0, &args![this, 1u8]);
        assert_eq!(e.get(this, QueuedModel::cFlags), 0x05);
        e.call(0x0043_c7d0, &args![this, 1u8]);
        assert_eq!(e.get(this, QueuedModel::cFlags), 0x07);
        e.call(0x0043_c7a0, &args![this, 0u8]);
        e.call(0x0043_c7d0, &args![this, 0u8]);
        assert_eq!(e.get(this, QueuedModel::cFlags), 0x04);
    }

    #[test]
    fn model_scalar_deleting_destructor_frees_only_on_request() {
        let mut e = queue_engine();
        let this: Ptr<QueuedModel> = e.new_object();
        e.call_log = Some(vec![]);
        let back = e.call(0x0043_c800, &args![this, 0u32]);
        assert_eq!(back.ptr::<QueuedModel>(), this);
        assert_eq!(
            call_order(&e),
            vec![
                0x0043_c800,
                MODEL_POINTER_RELEASE,
                QUEUED_FILE_ENTRY_DESTRUCT
            ]
        );
        e.call_log = Some(vec![]);
        e.call(0x0043_c800, &args![this, 1u32]);
        assert_eq!(calls_to(&e, MEMORY_FREE), vec![vec![this.addr()]]);
    }

    #[test]
    fn model_destructor_releases_the_model_pointer_and_keeps_the_table() {
        let mut e = queue_engine();
        let this: Ptr<QueuedModel> = e.new_object();
        e.mem.set_u32(this.addr(), 0x2222);
        e.call_log = Some(vec![]);
        e.call(0x0043_c830, &args![this]);
        assert_eq!(
            calls_to(&e, MODEL_POINTER_RELEASE),
            vec![vec![this.addr() + 0x30]]
        );
        assert_eq!(
            calls_to(&e, QUEUED_FILE_ENTRY_DESTRUCT),
            vec![vec![this.addr()]]
        );
        // The virtual table is left as it was.
        assert_eq!(e.mem.u32(this.addr()), 0x2222);
    }

    // --- `QueuedModel` (third part) and the tree classes. ---

    /// A queued model's table: slot 0x28 is `CheckFinished`, slot 0x30 the
    /// `QueueMe_ov2` hook that records its argument.
    const MODEL_TASK_VTABLE: u32 = 0x0ff2_5000;
    const QUEUE_ME_HOOK: u32 = 0x0ff0_0110;
    /// A `TESModel`'s table: slot 0x14 is the model name, slot 0x1c the list.
    const TES_MODEL_VTABLE: u32 = 0x0ff2_6000;
    const TES_NAME_SLOT: u32 = 0x0ff0_0111;
    const TES_LIST_SLOT: u32 = 0x0ff0_0112;
    /// A 3D object's table: slot 0 deletes, slot 0x10 casts to a fade node.
    const OBJECT_VTABLE: u32 = 0x0ff2_7000;
    const OBJECT_DELETE: u32 = 0x0ff0_0113;
    const FADE_CAST: u32 = 0x0ff0_0114;
    /// A stream's object reference map table: slot 0 deletes.
    const REF_MAP_VTABLE: u32 = 0x0ff2_8000;
    const REF_MAP_DELETE: u32 = 0x0ff0_0115;
    /// A stream's table: slot 0x2c gives the link id.
    const LINK_STREAM_VTABLE: u32 = 0x0ff2_9000;
    const LINK_ID_SLOT: u32 = 0x0ff0_0116;
    /// A fixed answer of `QueuedFileEntry::GetFile`, the file name and the
    /// name of the tests' models.
    const OPEN_FILE: u32 = 0x0ff4_0000;
    /// Scratch dwords on the mapped page at `011f9000`.
    const SCRATCH: u32 = 0x011f_9100;

    /// An engine with doubles for what the `QueuedModel` and tree functions
    /// call.
    fn model_engine() -> Engine {
        let mut e = queue_engine();
        for page in [0x0101_1000, 0x0101_2000, 0x011c_6000, 0x011f_9000] {
            e.map(page, 0x1000);
        }
        e.register(TEST_FLAG_BIT_1, |_, a| (a[0] & 1 != 0).into_ret());
        e.register(TEST_FLAG_BIT_8, |_, a| (a[0] & 8 != 0).into_ret());
        e.register(TEST_FLAG_BIT_10, |_, a| (a[0] & 0x10 != 0).into_ret());
        e.register(TEST_FLAG_BIT_20, |_, a| (a[0] & 0x20 != 0).into_ret());
        e.register(SET_FLAG_BIT_8, |e, a| {
            write_flag(e, a[1], 8, a[0] & 0xff != 0);
            Ret::default()
        });
        e.register(STATE_IS_ZERO, |e, a| {
            (e.mem.u32(a[0] + 0xc) == 0).into_ret()
        });
        e.register(FIELD_AT_8, |e, a| e.mem.u32(a[0] + 8).into_ret());
        e.register(FIELD_AT_4, |e, a| e.mem.u32(a[0] + 4).into_ret());
        e.register(BYTE_AT_0, |e, a| e.mem.u8(a[0]).into_ret());
        e.register(FINISHED_COUNT_GETTER, |e, a| {
            e.mem.u32(a[0] + 0x10).into_ret()
        });
        e.register(LIST_FIRST_NODE, |_, a| (a[0] + 0x18).into_ret());
        e.register(LIST_NODE_ITEM_ADDRESS, |_, a| a[0].into_ret());
        e.register(TES_MODEL_TEXTURE_BLOCK, |_, a| (a[0] + 0xc).into_ret());
        e.register(MODEL_POINTER_ASSIGN, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            a[0].into_ret()
        });
        e.register(TASK_POINTER_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            a[0].into_ret()
        });
        e.register(ZERO_THREE_WORDS, |e, a| {
            for word in 0..3 {
                e.mem.set_u32(a[0] + word * 4, 0);
            }
            Ret::default()
        });
        e.register(MODEL_LOADER_FIND_MODEL, |_, _| 0u32.into_ret());
        e.register(MODEL_LOADER_ADD_MODEL, |_, _| 1u32.into_ret());
        e.register(BS_STREAM_LOAD, |_, _| 1u32.into_ret());
        e.register(QUEUED_FILE_ENTRY_GET_FILE, |_, _| OPEN_FILE.into_ret());
        // What `Model::Model` (`InitModel`) calls once it has its objects.
        e.register(REMOVE_EDITOR_MARKERS, |_, _| Ret::default());
        e.register(GET_PROPERTY, |_, _| 0u32.into_ret());
        e.register(PROPERTY_TYPE_SECOND_GETTER, |_, _| SECOND_TYPE.into_ret());
        e.register(MATCHES_PATTERN, |_, _| 0u32.into_ret());
        e.register(STREAM_FREE_ALL_OBJECTS, |_, _| Ret::default());
        e.register(HAS_MORPHER_CONTROLLER, |_, _| 0u32.into_ret());
        e.register(PREPARE_OBJECT, |_, _| Ret::default());
        e.register(SETTING_VALUE_POINTER, |_, a| (a[0] + 4).into_ret());
        e.register(BOUND_RADIUS, |e, a| e.mem.f32(a[0] + 0xc).into_ret());
        e.register(OVERRIDDEN_VISUAL_DISTANCE, |e, a| {
            e.mem.f32(a[0] + 0x40).into_ret()
        });
        for address in [
            QUEUED_FILE_CHECK_FINISHED,
            QUEUED_FILE_ENTRY_GENERATE_KEY,
            MODEL_LOADER_QUEUE_TEXTURE,
            QUEUE_TEXTURE_SET,
            NI_STREAM_CONSTRUCT,
            NI_STREAM_DESTRUCT,
            NI_STREAM_SAVE_BINARY_U32,
            ADD_REFERENCE,
            NI_AV_OBJECT_UPDATE,
            FADE_NODE_SET_RANGE,
            FADE_NODE_SET_LOD_MULT_TYPE,
            CREATE_BILLBOARD_INSTANCES,
            TREE_BILLBOARD_DATA_DESTRUCT,
            TASK_POINTER_DESTRUCT,
            QUEUE_TABLE_ADD,
            QUEUE_ME_HOOK,
            OBJECT_DELETE,
            REF_MAP_DELETE,
        ] {
            e.register(address, |_, _| Ret::default());
        }
        e.register(FADE_CAST, |_, _| 0u32.into_ret());
        e.register(LINK_ID_SLOT, |_, a| (a[1] + 0x100).into_ret());
        put_slots(
            &mut e,
            MODEL_TASK_VTABLE,
            &[(0x28, CHECK_FINISHED), (0x30, QUEUE_ME_HOOK)],
        );
        put_slots(
            &mut e,
            TES_MODEL_VTABLE,
            &[(0x14, TES_NAME_SLOT), (0x1c, TES_LIST_SLOT)],
        );
        put_slots(
            &mut e,
            OBJECT_VTABLE,
            &[(0, OBJECT_DELETE), (0x10, FADE_CAST)],
        );
        put_slots(&mut e, REF_MAP_VTABLE, &[(0, REF_MAP_DELETE)]);
        // A `BSStream` whose own loader (slot 0x60) fails.
        put_slots(&mut e, BS_STREAM_VTABLE, &[]);
        // After the tables: the table of the stream shares the page.
        e.mem.set_f64(HALF, 0.5);
        e.mem.set_f64(ZERO, 0.0);
        e.mem.set_f64(LOADED_AREA_MARGIN, 2048.0);
        e.mem.set_f64(SMALL_RADIUS, 0.001f32 as f64);
        put_slots(&mut e, LINK_STREAM_VTABLE, &[(0x2c, LINK_ID_SLOT)]);
        e
    }

    /// A queued model whose first word is the model task table.
    fn queued_model(e: &mut Engine) -> Ptr<QueuedModel> {
        let model: Ptr<QueuedModel> = e.new_object();
        e.mem.set_u32(model.addr(), MODEL_TASK_VTABLE);
        model
    }

    /// A `TESModel` object whose name slot is a double returning `name` and
    /// whose list slot returns `list`.
    fn tes_model(e: &mut Engine) -> Ptr {
        let tes_model = Ptr::new(e.mem.alloc(0x40));
        e.mem.set_u32(tes_model.addr(), TES_MODEL_VTABLE);
        tes_model
    }

    #[test]
    fn model_constructor_with_a_tes_model_sets_up_the_fields_and_flags() {
        let mut e = model_engine();
        e.register(TES_NAME_SLOT, |e, _| e.mem.u32(SCRATCH).into_ret());
        let name = text(&mut e, "meshes\\tree.nif");
        e.mem.set_u32(SCRATCH, name);
        let tes = tes_model(&mut e);
        let this: Ptr<QueuedModel> = e.new_object();
        e.set(this, QueuedModel::cFlags, 0xff);
        e.set(this, QueuedModel::mfOverriddenVisualDistance, 5.0);
        e.call_log = Some(vec![]);
        let back = e.call(0x0043_c890, &args![this, tes, 0x66u32, 0x3u32, 0u8, 1u8]);
        assert_eq!(back.ptr::<QueuedModel>(), this);
        assert_eq!(e.mem.u32(this.addr()), QUEUED_MODEL_VTABLE);
        assert_eq!(e.get(this, QueuedModel::spModel), Ptr::NULL);
        assert_eq!(e.get(this, QueuedModel::pTESModel), tes);
        assert_eq!(e.get(this, QueuedModel::eLODFadeMult), 3);
        // The first flag argument (0) clears bit 1, the second sets bit 2.
        assert_eq!(e.get(this, QueuedModel::cFlags), 2);
        assert_eq!(e.get(this, QueuedModel::mfOverriddenVisualDistance), 0.0);
        assert_eq!(
            calls_to(&e, QUEUED_FILE_ENTRY_CONSTRUCT),
            vec![vec![this.addr(), 0x66]]
        );
        // The file name is what the TESModel's slot 0x14 answered.
        assert_eq!(
            calls_to(&e, QUEUED_FILE_ENTRY_SET_FILE_NAME),
            vec![vec![this.addr(), name]]
        );
        assert_eq!(
            calls_to(&e, QUEUED_FILE_ENTRY_FIND_FILE_ENTRY),
            vec![vec![this.addr(), 0]]
        );
    }

    #[test]
    fn model_constructor_with_a_loaded_model_stores_it_and_finishes_the_task() {
        let mut e = model_engine();
        let this: Ptr<QueuedModel> = e.new_object();
        e.set(this, QueuedModel::pTESModel, Ptr::new(0x1111));
        e.set(this, QueuedModel::cFlags, 0xff);
        e.call_log = Some(vec![]);
        let back = e.call(0x0043_c960, &args![this, Ptr::<()>::new(0x7777), 0x55u32]);
        assert_eq!(back.ptr::<QueuedModel>(), this);
        assert_eq!(e.mem.u32(this.addr()), QUEUED_MODEL_VTABLE);
        assert_eq!(e.get(this, QueuedModel::spModel), Ptr::new(0x7777));
        assert_eq!(e.get(this, QueuedModel::pTESModel), Ptr::NULL);
        assert_eq!(e.get(this, QueuedModel::eLODFadeMult), 0);
        assert_eq!(e.get(this, QueuedModel::cFlags), 0);
        assert_eq!(
            calls_to(&e, QUEUED_FILE_ENTRY_CONSTRUCT),
            vec![vec![this.addr(), 0x55]]
        );
        assert_eq!(calls_to(&e, TASK_SET_DONE), vec![vec![this.addr()]]);
        // No file name is set.
        assert!(calls_to(&e, QUEUED_FILE_ENTRY_SET_FILE_NAME).is_empty());
    }

    #[test]
    fn model_generate_key_is_the_file_entry_one() {
        let mut e = model_engine();
        let this = queued_model(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x0043_ca10, &args![this]);
        assert_eq!(
            calls_to(&e, QUEUED_FILE_ENTRY_GENERATE_KEY),
            vec![vec![this.addr()]]
        );
    }

    #[test]
    fn model_check_finished_follows_bit_4_children_and_state() {
        let mut e = model_engine();
        let queue = task_queue(&mut e);
        let children = Ptr::<()>::new(e.mem.alloc(0x20));
        let this = queued_model(&mut e);
        e.set(this, QueuedModel::pChildren, children);
        // Bit 4 clear: the base `CheckFinished`.
        e.call_log = Some(vec![]);
        e.call(0x0043_ca30, &args![this]);
        assert_eq!(
            calls_to(&e, QUEUED_FILE_CHECK_FINISHED),
            vec![vec![this.addr()]]
        );
        assert!(calls_to(&e, QUEUE_ADD).is_empty());
        // Bit 4 set, 1 of 2 children finished: nothing.
        e.set(this, QueuedModel::cFlags, 4);
        e.mem.set_u32(children.addr() + 8, 2);
        e.mem.set_u32(children.addr() + 0x10, 1);
        e.call_log = Some(vec![]);
        e.call(0x0043_ca30, &args![this]);
        assert!(calls_to(&e, QUEUED_FILE_CHECK_FINISHED).is_empty());
        assert!(calls_to(&e, QUEUE_ADD).is_empty());
        // All finished and the task in state 0: given to the task queue.
        e.mem.set_u32(children.addr() + 0x10, 2);
        e.call_log = Some(vec![]);
        e.call(0x0043_ca30, &args![this]);
        assert_eq!(calls_to(&e, QUEUE_ADD), vec![vec![queue, this.addr()]]);
        assert!(calls_to(&e, QUEUED_FILE_CHECK_FINISHED).is_empty());
        // All finished, state not 0: the base `CheckFinished`.
        e.mem.set_u32(this.addr() + 0xc, 3);
        e.call_log = Some(vec![]);
        e.call(0x0043_ca30, &args![this]);
        assert!(calls_to(&e, QUEUE_ADD).is_empty());
        assert_eq!(
            calls_to(&e, QUEUED_FILE_CHECK_FINISHED),
            vec![vec![this.addr()]]
        );
    }

    #[test]
    fn model_children_finished_compares_the_counts() {
        let mut e = model_engine();
        let this = queued_model(&mut e);
        // No children object: finished.
        assert!(e.call(0x0043_caa0, &args![this]).bool());
        let children = Ptr::<()>::new(e.mem.alloc(0x20));
        e.set(this, QueuedModel::pChildren, children);
        e.mem.set_u32(children.addr() + 8, 3);
        e.mem.set_u32(children.addr() + 0x10, 3);
        assert!(e.call(0x0043_caa0, &args![this]).bool());
        e.mem.set_u32(children.addr() + 0x10, 2);
        assert!(!e.call(0x0043_caa0, &args![this]).bool());
    }

    #[test]
    fn model_bit_4_test_reads_the_flags() {
        let mut e = model_engine();
        let this = queued_model(&mut e);
        assert!(!e.call(0x0043_caf0, &args![this]).bool());
        e.set(this, QueuedModel::cFlags, 4);
        assert!(e.call(0x0043_caf0, &args![this]).bool());
    }

    #[test]
    fn model_queue_me_passes_the_tes_model_block_or_zero() {
        let mut e = model_engine();
        let this = queued_model(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x0043_cb10, &args![this]);
        assert_eq!(calls_to(&e, QUEUE_ME_HOOK), vec![vec![this.addr(), 0]]);
        assert!(calls_to(&e, TES_MODEL_TEXTURE_BLOCK).is_empty());
        let tes = tes_model(&mut e);
        e.set(this, QueuedModel::pTESModel, tes);
        e.call_log = Some(vec![]);
        e.call(0x0043_cb10, &args![this]);
        assert_eq!(
            calls_to(&e, QUEUE_ME_HOOK),
            vec![vec![this.addr(), tes.addr() + 0xc]]
        );
    }

    #[test]
    fn model_queue_me_ov2_queues_the_textures_and_texture_sets() {
        let mut e = model_engine();
        e.set_global(FILE_MAP_OWNER, OWNER_OBJECT);
        let this = queued_model(&mut e);
        // The key of the task: priority byte 7 in bits 16 to 23.
        e.mem.set_u64(this.addr() + 0x10, 0x0007_0000);
        // A block of three texture entries, the middle one empty.
        let textures = e.mem.alloc(12);
        e.mem.set_u32(textures, 0xaa);
        e.mem.set_u32(textures + 8, 0xcc);
        let block = e.mem.alloc(8);
        e.mem.set_u8(block, 3);
        e.mem.set_u32(block + 4, textures);
        // A TESModel with a list of three nodes (the first is embedded in
        // the list at +0x18; a node is an item pointer and a next link): a
        // texture set, a set holder whose set is null, no holder at all.
        let tes = tes_model(&mut e);
        e.set(this, QueuedModel::pTESModel, tes);
        let list = e.mem.alloc(0x20);
        e.register(TES_LIST_SLOT, |e, _| e.mem.u32(SCRATCH).into_ret());
        e.mem.set_u32(SCRATCH, list);
        let holder = e.mem.alloc(8);
        e.mem.set_u32(holder, 0x5e7);
        let empty_holder = e.mem.alloc(8);
        e.mem.set_u32(empty_holder, 0);
        let third = e.mem.alloc(8);
        e.mem.set_u32(third, 0);
        e.mem.set_u32(third + 4, 0);
        let second = e.mem.alloc(8);
        e.mem.set_u32(second, empty_holder);
        e.mem.set_u32(second + 4, third);
        let first = list + 0x18;
        e.mem.set_u32(first, holder);
        e.mem.set_u32(first + 4, second);
        e.call_log = Some(vec![]);
        e.call(0x0043_cb50, &args![this, block]);
        assert_eq!(
            calls_to(&e, MODEL_LOADER_QUEUE_TEXTURE),
            vec![
                vec![OWNER_OBJECT, 0xaa, 7, this.addr()],
                vec![OWNER_OBJECT, 0xcc, 7, this.addr()]
            ]
        );
        assert_eq!(
            calls_to(&e, QUEUE_TEXTURE_SET),
            vec![vec![0x5e7, 7, this.addr(), 0]]
        );
        // Bit 4 is set, then `CheckFinished` is called.
        assert_eq!(e.get(this, QueuedModel::cFlags), 4);
        assert_eq!(calls_to(&e, CHECK_FINISHED), vec![vec![this.addr()]]);
    }

    #[test]
    fn model_queue_me_ov2_without_block_or_tes_model_only_flags_and_checks() {
        let mut e = model_engine();
        let this = queued_model(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x0043_cb50, &args![this, 0u32]);
        assert!(calls_to(&e, MODEL_LOADER_QUEUE_TEXTURE).is_empty());
        assert!(calls_to(&e, QUEUE_TEXTURE_SET).is_empty());
        assert_eq!(e.get(this, QueuedModel::cFlags), 4);
        assert_eq!(calls_to(&e, CHECK_FINISHED).len(), 1);
        // A block with a zero count is skipped as well.
        let block = e.mem.alloc(8);
        e.mem.set_u8(block, 0);
        e.call_log = Some(vec![]);
        e.call(0x0043_cb50, &args![this, block]);
        assert!(calls_to(&e, MODEL_LOADER_QUEUE_TEXTURE).is_empty());
    }

    #[test]
    fn task_key_is_shifted_arithmetically_by_16() {
        let mut e = model_engine();
        let task = queued_model(&mut e);
        e.mem.set_u64(task.addr() + 0x10, 0x8000_0000_0012_0000);
        let shifted = e.call(0x0043_cc80, &args![task]).u64();
        assert_eq!(shifted, 0xffff_8000_0000_0012);
        assert_eq!(e.call(0x0043_cc60, &args![task]).u32(), 0x12);
        assert_eq!(
            e.call(0x0043_cca0, &args![Ptr::<()>::new(task.addr() + 0x10)])
                .u64(),
            0xffff_8000_0000_0012
        );
    }

    #[test]
    fn model_bit_4_setter_sets_and_clears() {
        let mut e = model_engine();
        let this = queued_model(&mut e);
        e.set(this, QueuedModel::cFlags, 1);
        e.call(0x0043_ccc0, &args![this, 1u8]);
        assert_eq!(e.get(this, QueuedModel::cFlags), 5);
        e.call(0x0043_ccc0, &args![this, 0u8]);
        assert_eq!(e.get(this, QueuedModel::cFlags), 1);
    }

    /// A queued model ready for `Run`: a file name and the context 0x21.
    fn runnable_model(e: &mut Engine, name: &str) -> Ptr<QueuedModel> {
        let this = queued_model(e);
        let name = text(e, name);
        e.mem.set_u32(this.addr() + 0x28, name);
        e.set(this, QueuedModel::eContext, 0x21);
        this
    }

    #[test]
    fn model_run_without_a_file_name_does_nothing() {
        let mut e = model_engine();
        let this = runnable_model(&mut e, "");
        e.call_log = Some(vec![]);
        e.call(0x0043_ccf0, &args![this]);
        assert!(calls_to(&e, MODEL_LOADER_FIND_MODEL).is_empty());
        assert!(calls_to(&e, QUEUED_FILE_ENTRY_GET_FILE).is_empty());
        // The memory context was entered and left.
        let enter = calls_to(&e, MEMORY_CONTEXT_ENTER);
        assert_eq!(enter.len(), 1);
        assert_eq!(&enter[0][1..], &[0x21, 1, MODEL_LOADER_SOURCE, 0x3b8]);
        assert_eq!(calls_to(&e, MEMORY_CONTEXT_LEAVE).len(), 1);
    }

    #[test]
    fn model_run_takes_a_model_the_loader_already_has() {
        let mut e = model_engine();
        e.set_global(FILE_MAP_OWNER, OWNER_OBJECT);
        e.register(MODEL_LOADER_FIND_MODEL, |e, a| {
            e.mem.set_u32(a[2], 0x5555);
            1u32.into_ret()
        });
        let this = runnable_model(&mut e, "meshes\\a.nif");
        let name = e.mem.u32(this.addr() + 0x28);
        e.call_log = Some(vec![]);
        e.call(0x0043_ccf0, &args![this]);
        assert_eq!(
            calls_to(&e, MODEL_LOADER_FIND_MODEL),
            vec![vec![OWNER_OBJECT, name, this.addr() + 0x30]]
        );
        assert_eq!(e.get(this, QueuedModel::spModel), Ptr::new(0x5555));
        // Bit 8 is set; the file is never opened.
        assert_eq!(e.get(this, QueuedModel::cFlags), 8);
        assert!(calls_to(&e, QUEUED_FILE_ENTRY_GET_FILE).is_empty());
        assert!(calls_to(&e, 0x0043_aaf0).is_empty());
        assert_eq!(calls_to(&e, MEMORY_CONTEXT_LEAVE).len(), 1);
    }

    /// The loader of the stream (`BSStream::Load`) gives the stream a root
    /// node named `Scene Root`; the returned address is that node. The
    /// loader also records the name published in `011f94c0` at `SCRATCH`.
    fn stream_loads_a_root(e: &mut Engine) -> Ptr {
        let root = object(e, NODE_VTABLE);
        let root_name = text(e, "Scene Root");
        e.mem.set_u32(root.addr() + 8, root_name);
        e.register_double(BS_STREAM_LOAD, move |e, a| {
            let published = e.global::<u32>(LOADING_FILE_NAME);
            e.mem.set_u32(SCRATCH, published);
            set_stream_objects(e, Ptr::new(a[0]), &[root]);
            1u32.into_ret()
        });
        root
    }

    #[test]
    fn model_run_loads_the_file_into_a_stream_and_builds_the_model() {
        let mut e = model_engine();
        e.set_global(FILE_MAP_OWNER, OWNER_OBJECT);
        let root = stream_loads_a_root(&mut e);
        let this = runnable_model(&mut e, "meshes\\a.nif");
        let name = e.mem.u32(this.addr() + 0x28);
        // Bit 1 (prepare) and bit 0x20 (force).
        e.set(this, QueuedModel::cFlags, 0x21);
        e.call_log = Some(vec![]);
        e.call(0x0043_ccf0, &args![this]);
        assert_eq!(
            calls_to(&e, QUEUED_FILE_ENTRY_GET_FILE),
            vec![vec![this.addr(), 0, 1]]
        );
        let loads = calls_to(&e, BS_STREAM_LOAD);
        assert_eq!(loads.len(), 1);
        let stream = loads[0][0];
        assert_eq!(&loads[0][1..], &[name, OPEN_FILE]);
        // The load saw the name published in `011f94c0`.
        assert_eq!(e.mem.u32(SCRATCH), name);
        assert_eq!(calls_to(&e, NI_STREAM_CONSTRUCT), vec![vec![stream]]);
        // The model (a 0x10-byte block) is built from the stream and stored:
        // its 3D object is the stream's root, prepared with `force` and
        // `prepare` taken from bits 0x20 and 1.
        assert_eq!(calls_to(&e, MEMORY_ALLOC)[0], vec![0x10]);
        let model = e.get(this, QueuedModel::spModel);
        assert!(!model.is_null());
        assert_eq!(e.mem.u32(model.addr() + 0xc), root.addr());
        assert_eq!(calls_to(&e, PREPARE_OBJECT), vec![vec![root.addr(), 1, 1]]);
        // The name is withdrawn and the stream destroyed.
        assert_eq!(e.global::<u32>(LOADING_FILE_NAME), 0);
        assert_eq!(calls_to(&e, NI_STREAM_DESTRUCT), vec![vec![stream]]);
        assert!(calls_to(&e, LOG).is_empty());
        // Only bit 1: prepare without force.
        e.set(this, QueuedModel::spModel, Ptr::NULL);
        e.set(this, QueuedModel::cFlags, 0x01);
        e.call_log = Some(vec![]);
        e.call(0x0043_ccf0, &args![this]);
        assert_eq!(calls_to(&e, PREPARE_OBJECT), vec![vec![root.addr(), 0, 0]]);
        // Neither bit: not prepared at all.
        e.set(this, QueuedModel::spModel, Ptr::NULL);
        e.set(this, QueuedModel::cFlags, 0x00);
        e.call_log = Some(vec![]);
        e.call(0x0043_ccf0, &args![this]);
        assert!(calls_to(&e, PREPARE_OBJECT).is_empty());
    }

    #[test]
    fn model_run_logs_a_failed_load_unless_bit_10_allows_a_model() {
        let mut e = model_engine();
        e.set_global(FILE_MAP_OWNER, OWNER_OBJECT);
        e.register(BS_STREAM_LOAD, |_, _| 0u32.into_ret());
        let this = runnable_model(&mut e, "meshes\\a.nif");
        let name = e.mem.u32(this.addr() + 0x28);
        e.call_log = Some(vec![]);
        e.call(0x0043_ccf0, &args![this]);
        assert_eq!(
            calls_to(&e, LOG),
            vec![vec![MODEL_ERROR_MESSAGE, LOAD_WORD, name]]
        );
        // No model block is allocated and the pointer stays empty.
        assert!(calls_to(&e, MEMORY_ALLOC).is_empty());
        assert_eq!(e.get(this, QueuedModel::spModel), Ptr::NULL);
        assert_eq!(e.global::<u32>(LOADING_FILE_NAME), 0);
        assert_eq!(calls_to(&e, NI_STREAM_DESTRUCT).len(), 1);
        // Bit 0x10: a model is built anyway (from the empty stream).
        e.set(this, QueuedModel::cFlags, 0x10);
        e.call_log = Some(vec![]);
        e.call(0x0043_ccf0, &args![this]);
        assert!(calls_to(&e, LOG).is_empty());
        assert_eq!(calls_to(&e, MEMORY_ALLOC)[0], vec![0x10]);
        assert_ne!(e.get(this, QueuedModel::spModel), Ptr::NULL);
    }

    #[test]
    fn model_run_without_a_file_logs_find_or_builds_with_bit_10() {
        let mut e = model_engine();
        e.set_global(FILE_MAP_OWNER, OWNER_OBJECT);
        e.register(QUEUED_FILE_ENTRY_GET_FILE, |_, _| 0u32.into_ret());
        let this = runnable_model(&mut e, "meshes\\a.nif");
        let name = e.mem.u32(this.addr() + 0x28);
        e.call_log = Some(vec![]);
        e.call(0x0043_ccf0, &args![this]);
        assert_eq!(
            calls_to(&e, LOG),
            vec![vec![MODEL_ERROR_MESSAGE, FIND_WORD, name]]
        );
        // The stream is never read.
        assert!(calls_to(&e, BS_STREAM_LOAD).is_empty());
        assert_eq!(e.get(this, QueuedModel::spModel), Ptr::NULL);
        e.set(this, QueuedModel::cFlags, 0x10);
        e.call_log = Some(vec![]);
        e.call(0x0043_ccf0, &args![this]);
        assert!(calls_to(&e, LOG).is_empty());
        assert_ne!(e.get(this, QueuedModel::spModel), Ptr::NULL);
    }
    #[test]
    fn model_flag_tests_read_their_bits() {
        let mut e = model_engine();
        let this = queued_model(&mut e);
        for (address, bit) in [
            (0x0043_cf40u32, 1u8),
            (0x0043_cf90, 0x10),
            (0x0043_cfb0, 0x20),
            (0x0043_d490, 2),
            (0x0043_d4b0, 8),
        ] {
            e.set(this, QueuedModel::cFlags, !bit);
            assert!(!e.call(address, &args![this]).bool(), "{address:08x}");
            e.set(this, QueuedModel::cFlags, bit);
            assert!(e.call(address, &args![this]).bool(), "{address:08x}");
        }
    }

    #[test]
    fn model_bit_8_setter_sets_and_clears() {
        let mut e = model_engine();
        let this = queued_model(&mut e);
        e.call(0x0043_cf60, &args![this, 1u8]);
        assert_eq!(e.get(this, QueuedModel::cFlags), 8);
        e.call(0x0043_cf60, &args![this, 0u8]);
        assert_eq!(e.get(this, QueuedModel::cFlags), 0);
    }

    #[test]
    fn stream_constructor_builds_the_base_and_clears_the_pointers() {
        let mut e = model_engine();
        let stream: Ptr<BSStream> = e.new_object();
        e.set(stream, BSStream::pObjectRefMap, Ptr::new(0x1234));
        e.call_log = Some(vec![]);
        let back = e.call(0x0043_cfd0, &args![stream]);
        assert_eq!(back.ptr::<BSStream>(), stream);
        assert_eq!(calls_to(&e, NI_STREAM_CONSTRUCT), vec![vec![stream.addr()]]);
        assert_eq!(e.mem.u32(stream.addr()), BS_STREAM_VTABLE);
        assert_eq!(
            calls_to(&e, NI_POINTER_CONSTRUCT),
            vec![vec![stream.addr() + 0x5c8, 0]]
        );
        assert_eq!(e.get(stream, BSStream::pObjectRefMap), Ptr::NULL);
        // The base constructor comes first, the table is set after it.
        assert_eq!(call_order(&e)[1], NI_STREAM_CONSTRUCT);
    }

    #[test]
    fn save_link_id_writes_the_id_the_stream_gives() {
        let mut e = model_engine();
        let stream = Ptr::<()>::new(e.mem.alloc(8));
        e.mem.set_u32(stream.addr(), LINK_STREAM_VTABLE);
        e.register(NI_STREAM_SAVE_BINARY_U32, |e, a| {
            let value = e.mem.u32(a[1]);
            e.mem.set_u32(SCRATCH, value);
            Ret::default()
        });
        e.call_log = Some(vec![]);
        e.call(0x0043_d050, &args![stream, Ptr::<()>::new(0x40)]);
        // The slot got the object and answered object + 0x100.
        assert_eq!(calls_to(&e, LINK_ID_SLOT), vec![vec![stream.addr(), 0x40]]);
        assert_eq!(e.mem.u32(SCRATCH), 0x140);
        let saves = calls_to(&e, NI_STREAM_SAVE_BINARY_U32);
        assert_eq!(saves.len(), 1);
        assert_eq!(saves[0][0], stream.addr());
    }

    #[test]
    fn stream_destructor_deletes_the_map_and_destroys_the_pointer_and_base() {
        let mut e = model_engine();
        let stream: Ptr<BSStream> = e.new_object();
        let map = Ptr::<()>::new(e.mem.alloc(8));
        e.mem.set_u32(map.addr(), REF_MAP_VTABLE);
        e.set(stream, BSStream::pObjectRefMap, map);
        e.call_log = Some(vec![]);
        e.call(0x0043_d090, &args![stream]);
        assert_eq!(e.mem.u32(stream.addr()), BS_STREAM_VTABLE);
        assert_eq!(
            call_order(&e),
            vec![
                0x0043_d090,
                REF_MAP_DELETE,
                NI_POINTER_DESTRUCT,
                NI_STREAM_DESTRUCT
            ]
        );
        assert_eq!(calls_to(&e, REF_MAP_DELETE), vec![vec![map.addr(), 1]]);
        assert_eq!(
            calls_to(&e, NI_POINTER_DESTRUCT),
            vec![vec![stream.addr() + 0x5c8]]
        );
        assert_eq!(e.get(stream, BSStream::pObjectRefMap), Ptr::NULL);
    }

    #[test]
    fn stream_map_deleter_skips_a_missing_map() {
        let mut e = model_engine();
        let stream: Ptr<BSStream> = e.new_object();
        e.call_log = Some(vec![]);
        e.call(0x0043_d100, &args![stream]);
        assert!(calls_to(&e, REF_MAP_DELETE).is_empty());
        assert_eq!(e.get(stream, BSStream::pObjectRefMap), Ptr::NULL);
    }

    #[test]
    fn stream_scalar_deleting_destructor_frees_only_on_request() {
        let mut e = model_engine();
        let stream: Ptr<BSStream> = e.new_object();
        e.call_log = Some(vec![]);
        let back = e.call(0x0043_d150, &args![stream, 0u32]);
        assert_eq!(back.ptr::<BSStream>(), stream);
        assert_eq!(calls_to(&e, NI_STREAM_DESTRUCT).len(), 1);
        assert!(calls_to(&e, MEMORY_FREE).is_empty());
        e.call_log = Some(vec![]);
        e.call(0x0043_d150, &args![stream, 1u32]);
        assert_eq!(calls_to(&e, MEMORY_FREE), vec![vec![stream.addr()]]);
    }

    /// A model ready for `Finish`: the queued model holds a model whose 3D
    /// object (its table casts to `fade_node`, which has the world bound
    /// radius `radius`).
    struct FinishSetup {
        this: Ptr<QueuedModel>,
        model: u32,
        object: u32,
        fade_node: u32,
    }

    fn finishing_model(e: &mut Engine, radius: f32) -> FinishSetup {
        let this = queued_model(e);
        e.set(this, QueuedModel::eContext, 0x21);
        let name = text(e, "meshes\\a.nif");
        e.mem.set_u32(this.addr() + 0x28, name);
        e.set(this, QueuedModel::eLODFadeMult, 2);
        let model = e.mem.alloc(0x10);
        let object = e.mem.alloc(0x10);
        e.mem.set_u32(object, OBJECT_VTABLE);
        e.mem.set_u32(model + 0xc, object);
        e.set(this, QueuedModel::spModel, Ptr::new(model));
        // The fade node has a world bound with the radius at +0xC.
        let fade_node = e.mem.alloc(0x40);
        let bound = e.mem.alloc(0x10);
        e.mem.set_f32(bound + 0xc, radius);
        e.mem.set_u32(fade_node + 0x20, bound);
        e.register(FADE_CAST, |e, a| e.mem.u32(a[0] + 4).into_ret());
        e.mem.set_u32(object + 4, fade_node);
        // The settings: 64 cells, visual distance scale 10, fade scale 0.5.
        e.mem.set_u32(CELL_COUNT_SETTING + 4, 64);
        e.mem.set_f32(VISUAL_DISTANCE_SCALE_SETTING + 4, 10.0);
        e.mem.set_f32(FADE_RANGE_SCALE_SETTING + 4, 0.5);
        e.set_global(FILE_MAP_OWNER, OWNER_OBJECT);
        FinishSetup {
            this,
            model,
            object,
            fade_node,
        }
    }

    /// The `(near, far)` floats `SetRange` received.
    fn set_range_args(e: &Engine) -> Vec<(f32, f32)> {
        calls_to(e, FADE_NODE_SET_RANGE)
            .iter()
            .map(|a| (f32::from_bits(a[1]), f32::from_bits(a[2])))
            .collect()
    }

    #[test]
    fn model_finish_sets_the_fade_range_from_the_radius() {
        let mut e = model_engine();
        let setup = finishing_model(&mut e, 100.0);
        e.call_log = Some(vec![]);
        e.call(0x0043_d180, &args![setup.this]);
        // 64 cells: limit = 262144 * 0.5 - (100 + 2048) = 128924; the radius
        // times 10 is smaller, and the node gets (range * 0.5, range).
        assert_eq!(set_range_args(&e), vec![(500.0, 1000.0)]);
        assert_eq!(
            calls_to(&e, FADE_NODE_SET_LOD_MULT_TYPE),
            vec![vec![setup.fade_node, 2]]
        );
        // The node was updated with the zeroed update record first.
        assert_eq!(calls_to(&e, NI_AV_OBJECT_UPDATE).len(), 1);
        assert_eq!(calls_to(&e, NI_AV_OBJECT_UPDATE)[0][0], setup.fade_node);
        // The model is added to the loader's map; the add succeeded.
        let name = e.mem.u32(setup.this.addr() + 0x28);
        assert_eq!(
            calls_to(&e, MODEL_LOADER_ADD_MODEL),
            vec![vec![OWNER_OBJECT, name, setup.model]]
        );
        assert!(calls_to(&e, MODEL_LOADER_FIND_MODEL).is_empty());
        // The memory context of `Finish`, then `CheckFinished`.
        let enter = calls_to(&e, MEMORY_CONTEXT_ENTER);
        assert_eq!(&enter[0][1..], &[0x21, 1, MODEL_LOADER_SOURCE, 0x3e5]);
        assert_eq!(calls_to(&e, CHECK_FINISHED), vec![vec![setup.this.addr()]]);
        assert!(calls_to(&e, ADD_REFERENCE).is_empty());
    }

    #[test]
    fn model_finish_limits_the_range_to_the_world() {
        let mut e = model_engine();
        let setup = finishing_model(&mut e, 100.0);
        // One cell: limit = 4096 * 0.5 - (100 + 2048) = -100.
        e.mem.set_u32(CELL_COUNT_SETTING + 4, 1);
        e.call_log = Some(vec![]);
        e.call(0x0043_d180, &args![setup.this]);
        assert_eq!(set_range_args(&e), vec![(-50.0, -100.0)]);
    }

    #[test]
    fn model_finish_takes_a_tiny_radius_from_the_maximum_bound() {
        let mut e = model_engine();
        let setup = finishing_model(&mut e, 0.0005);
        e.register(GET_BOUND_MAX, |e, a| {
            e.mem.set_f32(a[0] + 8, 20.0);
            a[0].into_ret()
        });
        e.call_log = Some(vec![]);
        e.call(0x0043_d180, &args![setup.this]);
        // The radius becomes 20 * 0.5 = 10, the range 10 * 10.
        let bound_calls = calls_to(&e, GET_BOUND_MAX);
        assert_eq!(bound_calls.len(), 1);
        assert_eq!(bound_calls[0][1], setup.object);
        assert_eq!(set_range_args(&e), vec![(50.0, 100.0)]);
        // A maximum bound of zero leaves radius 0: the limit is used.
        e.register(GET_BOUND_MAX, |e, a| {
            e.mem.set_f32(a[0] + 8, 0.0);
            a[0].into_ret()
        });
        e.call_log = Some(vec![]);
        e.call(0x0043_d180, &args![setup.this]);
        assert_eq!(
            set_range_args(&e),
            vec![(65536.0 - 1024.0, 131072.0 - 2048.0)]
        );
    }

    #[test]
    fn model_finish_uses_the_overridden_visual_distance_when_set() {
        let mut e = model_engine();
        let setup = finishing_model(&mut e, 100.0);
        e.set(setup.this, QueuedModel::mfOverriddenVisualDistance, 77.0);
        e.call_log = Some(vec![]);
        e.call(0x0043_d180, &args![setup.this]);
        assert_eq!(set_range_args(&e), vec![(38.5, 77.0)]);
    }

    #[test]
    fn model_finish_branches_on_flags_missing_objects_and_the_map() {
        let mut e = model_engine();
        let setup = finishing_model(&mut e, 100.0);
        // Bit 2 adds a reference to the model.
        e.set(setup.this, QueuedModel::cFlags, 2);
        e.call_log = Some(vec![]);
        e.call(0x0043_d180, &args![setup.this]);
        assert_eq!(calls_to(&e, ADD_REFERENCE), vec![vec![setup.model]]);
        // Bit 8 skips everything but the final check.
        e.set(setup.this, QueuedModel::cFlags, 8);
        e.call_log = Some(vec![]);
        e.call(0x0043_d180, &args![setup.this]);
        assert!(calls_to(&e, NI_AV_OBJECT_UPDATE).is_empty());
        assert!(calls_to(&e, MODEL_LOADER_ADD_MODEL).is_empty());
        assert_eq!(calls_to(&e, CHECK_FINISHED).len(), 1);
        // No fade node: no range, but the model still goes into the map.
        e.set(setup.this, QueuedModel::cFlags, 0);
        e.mem.set_u32(setup.object + 4, 0);
        e.call_log = Some(vec![]);
        e.call(0x0043_d180, &args![setup.this]);
        assert!(calls_to(&e, FADE_NODE_SET_RANGE).is_empty());
        assert_eq!(calls_to(&e, MODEL_LOADER_ADD_MODEL).len(), 1);
        // No 3D object: nothing is added.
        e.mem.set_u32(setup.model + 0xc, 0);
        e.call_log = Some(vec![]);
        e.call(0x0043_d180, &args![setup.this]);
        assert!(calls_to(&e, MODEL_LOADER_ADD_MODEL).is_empty());
        assert_eq!(calls_to(&e, CHECK_FINISHED).len(), 1);
        // No model at all: only the final check.
        e.set(setup.this, QueuedModel::spModel, Ptr::NULL);
        e.call_log = Some(vec![]);
        e.call(0x0043_d180, &args![setup.this]);
        assert!(calls_to(&e, MODEL_LOADER_ADD_MODEL).is_empty());
        assert_eq!(calls_to(&e, CHECK_FINISHED).len(), 1);
    }

    #[test]
    fn model_finish_replaces_the_model_when_the_map_has_one() {
        let mut e = model_engine();
        let setup = finishing_model(&mut e, 100.0);
        e.register(MODEL_LOADER_ADD_MODEL, |_, _| 0u32.into_ret());
        let name = e.mem.u32(setup.this.addr() + 0x28);
        e.call_log = Some(vec![]);
        e.call(0x0043_d180, &args![setup.this]);
        assert_eq!(
            calls_to(&e, MODEL_LOADER_FIND_MODEL),
            vec![vec![OWNER_OBJECT, name, setup.this.addr() + 0x30]]
        );
    }

    #[test]
    fn update_data_constructor_stores_time_and_two_flags() {
        let mut e = model_engine();
        let data: Ptr<NiUpdateData> = e.new_object();
        for offset in 0..12 {
            e.mem.set_u8(data.addr() + offset, 0xee);
        }
        let back = e.call(0x0043_d410, &args![data, 1.5f32, 1u8, 0u8]);
        assert_eq!(back.ptr::<NiUpdateData>(), data);
        assert_eq!(e.get(data, NiUpdateData::fTime), 1.5);
        assert_eq!(e.get(data, NiUpdateData::bUpdateControllers), 1);
        assert_eq!(e.get(data, NiUpdateData::bParallelUpdate), 0);
        assert_eq!(e.get(data, NiUpdateData::bFoundParticles), 0);
        assert_eq!(e.get(data, NiUpdateData::bFoundMorphController), 0);
        assert_eq!(e.get(data, NiUpdateData::bSceneGraphChange), 0);
        // The padding after the five flags is not touched.
        assert_eq!(e.mem.u8(data.addr() + 9), 0xee);
    }

    #[test]
    fn world_bound_falls_back_to_the_empty_bound() {
        let mut e = model_engine();
        let object = Ptr::<()>::new(e.mem.alloc(0x40));
        assert_eq!(e.call(0x0043_d450, &args![object]).u32(), EMPTY_WORLD_BOUND);
        e.mem.set_u32(object.addr() + 0x20, 0x4321);
        assert_eq!(e.call(0x0043_d450, &args![object]).u32(), 0x4321);
    }

    #[test]
    fn setting_value_address_is_plus_4_or_a_zeroed_scratch() {
        let mut e = model_engine();
        e.mem.set_u32(SCRATCH_ZERO, 0x9999);
        assert_eq!(
            e.call(0x0043_d4d0, &args![Ptr::<()>::new(0x1000)]).u32(),
            0x1004
        );
        assert_eq!(e.mem.u32(SCRATCH_ZERO), 0x9999);
        assert_eq!(
            e.call(0x0043_d4d0, &args![Ptr::<()>::NULL]).u32(),
            SCRATCH_ZERO
        );
        assert_eq!(e.mem.u32(SCRATCH_ZERO), 0);
    }

    #[test]
    fn model_description_passes_the_word_model() {
        let mut e = model_engine();
        let this = queued_model(&mut e);
        e.call_log = Some(vec![]);
        let back = e.call(0x0043_d510, &args![this, 0x9000u32, 0x400u32]);
        assert!(back.bool());
        assert_eq!(
            calls_to(&e, QUEUED_FILE_ENTRY_GET_DESCRIPTION),
            vec![vec![this.addr(), 0x9000, 0x400, MODEL_WORD]]
        );
    }

    #[test]
    fn tree_billboard_constructor_builds_a_texture_with_the_data() {
        let mut e = model_engine();
        let this: Ptr<QueuedTreeBillboard> = e.new_object();
        let name = text(&mut e, "textures\\tree.dds");
        e.call_log = Some(vec![]);
        let back = e.call(
            0x0043_d540,
            &args![this, name, 0x44u32, Ptr::<()>::new(0x6000)],
        );
        assert_eq!(back.ptr::<QueuedTreeBillboard>(), this);
        assert_eq!(
            calls_to(&e, QUEUED_FILE_ENTRY_CONSTRUCT),
            vec![vec![this.addr(), 0x44]]
        );
        // The billboard's table replaces the texture's.
        assert_eq!(e.mem.u32(this.addr()), QUEUED_TREE_BILLBOARD_VTABLE);
        assert_eq!(
            e.get(this, QueuedTreeBillboard::pBillboardData),
            Ptr::new(0x6000)
        );
        assert_eq!(calls_to(&e, QUEUED_FILE_ENTRY_FIND_FILE_ENTRY).len(), 1);
    }

    #[test]
    fn tree_billboard_destructor_deletes_the_data_then_the_texture_part() {
        let mut e = model_engine();
        let this: Ptr<QueuedTreeBillboard> = e.new_object();
        let data: Ptr<TreeBillboardData> = e.new_object();
        e.set(this, QueuedTreeBillboard::pBillboardData, data.cast());
        e.call_log = Some(vec![]);
        e.call(0x0043_d5b0, &args![this]);
        assert_eq!(
            call_order(&e),
            vec![
                0x0043_d5b0,
                TREE_BILLBOARD_DATA_DESTRUCT,
                MEMORY_FREE,
                NI_POINTER_DESTRUCT,
                QUEUED_FILE_ENTRY_DESTRUCT
            ]
        );
        // The data is freed (flag 1).
        assert_eq!(calls_to(&e, MEMORY_FREE), vec![vec![data.addr()]]);
        // The texture part ends with the texture's table.
        assert_eq!(e.mem.u32(this.addr()), QUEUED_TEXTURE_VTABLE);
        // Without data only the base runs.
        e.set(this, QueuedTreeBillboard::pBillboardData, Ptr::NULL);
        e.call_log = Some(vec![]);
        e.call(0x0043_d5b0, &args![this]);
        assert!(calls_to(&e, TREE_BILLBOARD_DATA_DESTRUCT).is_empty());
        assert_eq!(calls_to(&e, QUEUED_FILE_ENTRY_DESTRUCT).len(), 1);
    }

    #[test]
    fn tree_billboard_scalar_deleting_destructor_frees_only_on_request() {
        let mut e = model_engine();
        let this: Ptr<QueuedTreeBillboard> = e.new_object();
        e.call_log = Some(vec![]);
        let back = e.call(0x0043_d580, &args![this, 0u32]);
        assert_eq!(back.ptr::<QueuedTreeBillboard>(), this);
        assert_eq!(calls_to(&e, QUEUED_FILE_ENTRY_DESTRUCT).len(), 1);
        assert!(calls_to(&e, MEMORY_FREE).is_empty());
        e.call_log = Some(vec![]);
        e.call(0x0043_d580, &args![this, 1u32]);
        assert_eq!(calls_to(&e, MEMORY_FREE), vec![vec![this.addr()]]);
    }

    #[test]
    fn billboard_data_deleter_runs_the_destructor_and_frees_on_request() {
        let mut e = model_engine();
        let data: Ptr<TreeBillboardData> = e.new_object();
        e.call_log = Some(vec![]);
        let back = e.call(0x0043_d640, &args![data, 0u32]);
        assert_eq!(back.ptr::<TreeBillboardData>(), data);
        assert_eq!(
            calls_to(&e, TREE_BILLBOARD_DATA_DESTRUCT),
            vec![vec![data.addr()]]
        );
        assert!(calls_to(&e, MEMORY_FREE).is_empty());
        e.call_log = Some(vec![]);
        e.call(0x0043_d640, &args![data, 1u32]);
        assert_eq!(calls_to(&e, MEMORY_FREE), vec![vec![data.addr()]]);
    }

    /// The IO manager of the tests: the table of queues at +0x64 is a block
    /// of dwords.
    const IO_MANAGER: u32 = 0x0ff5_0000;

    #[test]
    fn tree_billboard_finish_finishes_the_texture_then_posts_the_task() {
        let mut e = model_engine();
        e.map(IO_MANAGER, 0x1000);
        e.mem.set_u32(IO_MANAGER + 0x64, 0x7100);
        e.set_global(TASK_QUEUE, IO_MANAGER);
        e.set_global(FILE_MAP_OWNER, OWNER_OBJECT);
        let this: Ptr<QueuedTreeBillboard> = e.new_object();
        e.mem.set_u32(this.addr(), TASK_VTABLE);
        e.mem.set_u64(this.addr() + 0x10, 0x0004_0000);
        e.call_log = Some(vec![]);
        e.call(0x0043_d670, &args![this]);
        // The texture's `Finish` asks whether the task is finished first.
        let order = call_order(&e);
        let finish = order.iter().position(|a| *a == CHECK_FINISHED).unwrap();
        let post = order
            .iter()
            .position(|a| *a == TASK_POINTER_CONSTRUCT)
            .unwrap();
        assert!(finish < post);
        // The task is queued under its priority byte.
        let holder = calls_to(&e, TASK_POINTER_CONSTRUCT)[0][0];
        assert_eq!(
            calls_to(&e, TASK_POINTER_CONSTRUCT),
            vec![vec![holder, this.addr()]]
        );
        assert_eq!(calls_to(&e, QUEUE_TABLE_ADD), vec![vec![0x7100, 4, holder]]);
    }

    #[test]
    fn post_process_task_is_added_through_the_io_manager_global() {
        let mut e = model_engine();
        e.map(IO_MANAGER, 0x1000);
        e.mem.set_u32(IO_MANAGER + 0x64, 0x7100);
        e.set_global(TASK_QUEUE, IO_MANAGER);
        let task = queued_model(&mut e);
        e.mem.set_u64(task.addr() + 0x10, 0x0003_0000);
        e.call_log = Some(vec![]);
        e.call(0x0043_d690, &args![task]);
        let holder = calls_to(&e, TASK_POINTER_CONSTRUCT)[0][0];
        assert_eq!(
            calls_to(&e, TASK_POINTER_CONSTRUCT),
            vec![vec![holder, task.addr()]]
        );
        // The queue table of the manager is used.
        assert_eq!(calls_to(&e, QUEUE_TABLE_ADD), vec![vec![0x7100, 3, holder]]);
    }

    #[test]
    fn post_process_body_queues_under_the_priority_byte() {
        let mut e = model_engine();
        e.map(IO_MANAGER, 0x1000);
        e.mem.set_u32(IO_MANAGER + 0x64, 0x7200);
        let task = queued_model(&mut e);
        // Priority 0x12 in bits 16 to 23; higher bits do not matter.
        e.mem.set_u64(task.addr() + 0x10, 0x0000_0001_0012_0000);
        e.call_log = Some(vec![]);
        e.call(0x0043_d6b0, &args![Ptr::<()>::new(IO_MANAGER), task]);
        let holder = calls_to(&e, TASK_POINTER_CONSTRUCT)[0][0];
        assert_eq!(
            call_order(&e),
            vec![
                0x0043_d6b0,
                TASK_POINTER_CONSTRUCT,
                QUEUE_TABLE_ADD,
                TASK_POINTER_DESTRUCT
            ]
        );
        assert_eq!(
            calls_to(&e, QUEUE_TABLE_ADD),
            vec![vec![0x7200, 0x12, holder]]
        );
        assert_eq!(calls_to(&e, TASK_POINTER_DESTRUCT), vec![vec![holder]]);
    }

    #[test]
    fn tree_billboard_post_process_calls_slot_0x30() {
        let mut e = model_engine();
        let this: Ptr<QueuedTreeBillboard> = e.new_object();
        e.mem.set_u32(this.addr(), MODEL_TASK_VTABLE);
        e.call_log = Some(vec![]);
        e.call(0x0043_d730, &args![this]);
        assert_eq!(calls_to(&e, QUEUE_ME_HOOK), vec![vec![this.addr()]]);
    }

    #[test]
    fn tree_billboard_creates_the_distant_node_and_deletes_it_again() {
        let mut e = model_engine();
        let this: Ptr<QueuedTreeBillboard> = e.new_object();
        let data: Ptr<TreeBillboardData> = e.new_object();
        e.set(this, QueuedTreeBillboard::pBillboardData, data.cast());
        let tree = e.mem.alloc(0x20);
        e.mem.set_u32(tree + 0xc, 9);
        e.set(data, TreeBillboardData::pTree, Ptr::new(tree));
        e.set(data, TreeBillboardData::iCellChunk, 11);
        e.set(data, TreeBillboardData::iCellKey, 12);
        e.set(data, TreeBillboardData::pInstancedNode, Ptr::new(13));
        // The game reads the low word of the size.
        e.set(data, TreeBillboardData::iArraySize, 0x1_0005);
        e.set(data, TreeBillboardData::pLocArray, Ptr::new(15));
        e.set(data, TreeBillboardData::pColorArray, Ptr::new(16));
        e.register(BUILD_DISTANT_3D, |_, _| 0x0ff6_0000u32.into_ret());
        e.map(0x0ff6_0000, 0x1000);
        e.mem.set_u32(0x0ff6_0000, OBJECT_VTABLE);
        e.register(CREATE_BILLBOARD_INSTANCES, |e, a| {
            // The record: 0, the node, the count.
            let record: Vec<u32> = (0..3).map(|i| e.mem.u32(a[3] + i * 4)).collect();
            e.mem.set_u32(SCRATCH, record[0]);
            e.mem.set_u32(SCRATCH + 4, record[1]);
            e.mem.set_u32(SCRATCH + 8, record[2]);
            Ret::default()
        });
        e.call_log = Some(vec![]);
        e.call(0x0043_d750, &args![this]);
        assert_eq!(calls_to(&e, BUILD_DISTANT_3D), vec![vec![tree, 1]]);
        let built = calls_to(&e, CREATE_BILLBOARD_INSTANCES);
        assert_eq!(built.len(), 1);
        assert_eq!(&built[0][..3], &[11, 12, 13]);
        assert_eq!(&built[0][4..], &[15, 16, 5]);
        assert_eq!(
            [
                e.mem.u32(SCRATCH),
                e.mem.u32(SCRATCH + 4),
                e.mem.u32(SCRATCH + 8)
            ],
            [0, 0x0ff6_0000, 9]
        );
        // The node is deleted through its virtual destructor with flag 1.
        assert_eq!(calls_to(&e, OBJECT_DELETE), vec![vec![0x0ff6_0000, 1]]);
        // Without a node nothing else happens.
        e.register(BUILD_DISTANT_3D, |_, _| 0u32.into_ret());
        e.call_log = Some(vec![]);
        e.call(0x0043_d750, &args![this]);
        assert!(calls_to(&e, CREATE_BILLBOARD_INSTANCES).is_empty());
        assert!(calls_to(&e, OBJECT_DELETE).is_empty());
    }

    #[test]
    fn tree_billboard_description_passes_the_words() {
        let mut e = model_engine();
        let this: Ptr<QueuedTreeBillboard> = e.new_object();
        e.call_log = Some(vec![]);
        let back = e.call(0x0043_d820, &args![this, 0x9000u32, 0x400u32]);
        assert!(back.bool());
        assert_eq!(
            calls_to(&e, QUEUED_FILE_ENTRY_GET_DESCRIPTION),
            vec![vec![this.addr(), 0x9000, 0x400, TREE_BILLBOARD_WORD]]
        );
    }

    #[test]
    fn tree_model_constructor_uses_the_trees_embedded_tes_model() {
        let mut e = model_engine();
        e.register(TES_NAME_SLOT, |e, a| e.mem.u32(a[0] + 0x38).into_ret());
        // A tree with its TESModel (a table pointer) at +0x30.
        let tree = Ptr::<()>::new(e.mem.alloc(0x80));
        e.mem.set_u32(tree.addr() + 0x30, TES_MODEL_VTABLE);
        let name = text(&mut e, "trees\\oak.spt");
        e.mem.set_u32(tree.addr() + 0x30 + 0x38, name);
        let this: Ptr<QueuedTreeModel> = e.new_object();
        let reference = Ptr::<()>::new(0x8000);
        e.call_log = Some(vec![]);
        let back = e.call(0x0043_d850, &args![this, reference, tree, 0x31u32, 2u32]);
        assert_eq!(back.ptr::<QueuedTreeModel>(), this);
        assert_eq!(e.mem.u32(this.addr()), QUEUED_TREE_MODEL_VTABLE);
        assert_eq!(e.get(this, QueuedTreeModel::pRef), reference);
        assert_eq!(e.get(this, QueuedTreeModel::pTree), tree);
        assert_eq!(e.get(this, QueuedTreeModel::eContext), 0x1e);
        // The `QueuedModel` part: the embedded TESModel, the context and
        // fade multiplier arguments, bit 1 set and bit 2 clear.
        assert_eq!(
            calls_to(&e, QUEUED_FILE_ENTRY_CONSTRUCT),
            vec![vec![this.addr(), 0x31]]
        );
        assert_eq!(
            e.get(this.cast::<QueuedModel>(), QueuedModel::pTESModel),
            Ptr::new(tree.addr() + 0x30)
        );
        assert_eq!(
            e.get(this.cast::<QueuedModel>(), QueuedModel::eLODFadeMult),
            2
        );
        assert_eq!(e.get(this.cast::<QueuedModel>(), QueuedModel::cFlags), 1);
        assert_eq!(
            calls_to(&e, QUEUED_FILE_ENTRY_SET_FILE_NAME),
            vec![vec![this.addr(), name]]
        );
    }
}
