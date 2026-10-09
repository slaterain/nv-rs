//! `fallout shared/bipedanim.cpp` (Xbox PDB source unit), subsystem `fallout shared`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! `BipedAnim` is the set of body parts an actor wears: a root `NiNode`, five
//! named bones, and 20 biped slots, each with the form that fills it, the
//! model part, the loaded 3D clone and a skinned flag. A second array of 20
//! slots (the "buffered" objects) keeps the previous set while a new one is
//! loaded, so a part that did not change is reused.
//!
//! Session 1 (b0021) covers `004aaca0` to `004ad4a0`, the first 40 functions:
//! construction and `InitRoot`, `RemoveAllParts`, the slot clearing and
//! lookup helpers, the weapon helpers, the form to slot assignment
//! (`004abad0`), the model and texture queueing (`004abd30`, `004abfe0`,
//! `QueueSkinTexture`), `LoadBipedParts`, the small `NiCloningProcess`
//! members this unit emits, `ApplySkinnedObjects` and `AttachSkinnedObject`.
//! The next session continues at `004add50`.
//!
//! Conventions this file uses, so the next session finds them:
//!
//! - The layouts ([`BipedAnim`], [`BipedBone`], [`BipedObject`],
//!   [`NiCloningProcess`]) and the slot accessors ([`bone`], [`object`],
//!   [`buffered`]) are first below and `pub(crate)`. A function of this unit
//!   that is not translated yet (the ones from `004add50` on) is called by
//!   address through a `FN_` constant until a later session translates it; one
//!   that is translated is called directly.
//! - Tiny functions of other units are called by address through the
//!   constants below. `00559450` is the one the linker folded into dozens of
//!   getters: it returns the word at the address in `ECX`, so on an
//!   `NiPointer` it is the pointer itself ([`ni_pointer_get`]).
//! - The compiler's exception-unwinding frames and stack cookies are not
//!   translated. A local the game keeps on its stack and passes by address is
//!   a block from `stack_alloc` (released with `stack_free`).
//! - The decompiler hung pushed words on the wrong calls in this unit
//!   (`008a16d0`, `008b70d0`, `0043f8d0`, `006a9540` take no or other stack
//!   words than it shows); every call below follows the disassembly.

#[allow(unused_imports)]
use crate::prelude::*;

// ---- Layouts -----------------------------------------------------------------

layout! {
    /// `BipedAnim` (Xbox PDB), 0x2b4 bytes: the root node at +0, the five
    /// bones at +4 (8 bytes each, [`bone`]), the 20 slots at +0x2c (16 bytes
    /// each, [`object`]), the 20 buffered slots at +0x16c ([`buffered`]), the
    /// weapon offset and the requester.
    pub struct BipedAnim: 0x2b4 {
        /// `root` (Xbox PDB): the skeleton's `NiNode*`.
        0x000 root: Ptr,
        /// `m_fWeaponOffset` (Xbox PDB).
        0x2ac m_fWeaponOffset: f32,
        /// `m_pRequester` (Xbox PDB): the `TESObjectREFR*` (an actor) the
        /// parts are loaded for.
        0x2b0 m_pRequester: Ptr,
    }

    /// `BIPBONE` (Xbox PDB), 8 bytes: one named bone of the skeleton.
    pub struct BipedBone: 0x08 {
        /// `cFlags` (Xbox PDB): bit 0 is set once the bone is found.
        0x00 cFlags: u8,
        /// `pParent` (Xbox PDB): the bone's node, where parts are attached.
        0x04 pParent: Ptr,
    }

    /// `BIPOBJECT` (Xbox PDB), 16 bytes: one biped slot.
    pub struct BipedObject: 0x10 {
        /// `pParent` (Xbox PDB): the form (`TESForm*`) filling the slot.
        0x00 pParent: Ptr,
        /// `pPart` (Xbox PDB): the model (`TESModel*`) of the part.
        0x04 pPart: Ptr,
        /// `pPartClone` (Xbox PDB): the loaded clone (`NiAVObject*`).
        0x08 pPartClone: Ptr,
        /// `bSkinned` (Xbox PDB).
        0x0C bSkinned: bool,
    }

    /// `NiCloningProcess` (Xbox PDB), 0x10 bytes on the Xbox; the PC build
    /// adds the scale (three floats) at +0x10, set by the constructors here.
    pub struct NiCloningProcess: 0x1c {
        /// `m_pkCloneMap` (Xbox PDB): `NiTPointerMap<NiObject *,NiObject *>*`.
        0x00 m_pkCloneMap: Ptr,
        /// `m_pkProcessMap` (Xbox PDB): `NiTPointerMap<NiObject *,bool>*`.
        0x04 m_pkProcessMap: Ptr,
        /// `m_eCopyType` (Xbox PDB).
        0x08 m_eCopyType: u32,
        /// `m_cAppendChar` (Xbox PDB).
        0x0C m_cAppendChar: u8,
        /// The scale, x (PC only).
        0x10 m_fScale_x: f32,
        /// The scale, y (PC only).
        0x14 m_fScale_y: f32,
        /// The scale, z (PC only).
        0x18 m_fScale_z: f32,
    }
}

/// Number of bones in [`BipedAnim`] (`BipedAnim::bone`).
pub(crate) const BONE_COUNT: u32 = 5;
/// Number of biped slots (and of buffered slots).
pub(crate) const SLOT_COUNT: u32 = 20;
/// Offsets of the three arrays inside [`BipedAnim`].
pub(crate) const BONES_OFFSET: u32 = 0x04;
pub(crate) const OBJECTS_OFFSET: u32 = 0x2c;
pub(crate) const BUFFERED_OFFSET: u32 = 0x16c;
/// The slot of the weapon.
pub(crate) const WEAPON_SLOT: u32 = 5;

/// Bone `index` of `this` (`BipedAnim::bone[index]`).
pub(crate) fn bone(this: Ptr<BipedAnim>, index: u32) -> Ptr<BipedBone> {
    Ptr::new(this.addr().wrapping_add(BONES_OFFSET + 8 * index))
}

/// Slot `slot` of `this` (`BipedAnim::object[slot]`).
pub(crate) fn object(this: Ptr<BipedAnim>, slot: u32) -> Ptr<BipedObject> {
    Ptr::new(
        this.addr()
            .wrapping_add(OBJECTS_OFFSET.wrapping_add(slot.wrapping_mul(0x10))),
    )
}

/// Buffered slot `slot` of `this` (`BipedAnim::bufferedObjects[slot]`).
pub(crate) fn buffered(this: Ptr<BipedAnim>, slot: u32) -> Ptr<BipedObject> {
    Ptr::new(
        this.addr()
            .wrapping_add(BUFFERED_OFFSET.wrapping_add(slot.wrapping_mul(0x10))),
    )
}

// ---- Globals and data of the exe ----------------------------------------------

/// The global holding the `ModelLoader` (`011c3b3c`).
pub(crate) const MODEL_LOADER: u32 = 0x011c_3b3c;
/// The global holding the player character (`011dea3c`).
pub(crate) const PLAYER: u32 = 0x011d_ea3c;
/// The global holding the `TES` object (`011dea10`), `this` of
/// `TES::CreateDeepCopySameTextures`.
const TES_GLOBAL: u32 = 0x011d_ea10;
/// The data handler singleton pointer (`011c3f2c`).
const DATA_HANDLER: u32 = 0x011c_3f2c;
/// The save-load object pointer (`011de45c`).
const SAVE_LOAD: u32 = 0x011d_e45c;
/// The object at `011ddf38` whose flags `0042ce10` reads.
const FLAGS_OBJECT: u32 = 0x011d_df38;
/// The object `004538a0` and `004538c0` (the Xbox name of the latter is
/// `RtlLeaveCriticalSection`) bracket the removal of all parts with.
const PART_LOCK: u32 = 0x011c_5cf0;
/// A byte global tested before the FaceGen body texture palette is cleared
/// and before skin complexion is adjusted (its meaning is not confirmed).
const PALETTE_FLAG: u32 = 0x011c_5cb4;
/// Setting objects whose string value (`00408d60` gives its address) the
/// biped code tests for "not empty" (which settings they are is not
/// confirmed): `SETTING_FACE` alone, and the pair `004abfa0` tests.
const SETTING_FACE: u32 = 0x011c_5ef0;
const SETTING_PAIR_FIRST: u32 = 0x011d_5a10;
const SETTING_PAIR_SECOND: u32 = 0x011d_5a44;
/// Three-word global (zero) copied into a cloned node's translation
/// (`00440460`), and the nine-word global (identity) copied into its
/// rotation (`0043fa80`).
const ZERO_TRANSLATION: u32 = 0x011f_426c;
const IDENTITY_ROTATION: u32 = 0x011a_9448;
/// `-pi / 2` as a `float` (`01016b78`): the angle of the rotation about Y
/// `LoadBipedParts` applies to a FaceGen part.
const QUARTER_TURN_BACK: u32 = 0x0101_6b78;
/// The word `004ad1b0` returns and the byte `004ad1c0` returns: copied into
/// every new `NiCloningProcess`.
const CLONE_COPY_TYPE: u32 = 0x011f_4300;
const CLONE_APPEND_CHAR: u32 = 0x011a_94a8;
/// The global `004ab220` returns: the key of the extra data
/// `AttachSkinnedObject` keeps on the weapon bone (`NiFixedString`).
const EXTRA_DATA_KEY: u32 = 0x011c_61e4;
/// Class argument of `00653270` (the class `ApplySkinnedObjects` asks the
/// object about).
const SKINNED_CLASS: u32 = 0x011f_9140;

/// The bone names: five `char*` (`01188b74`).
const BONE_NAMES: u32 = 0x0118_8b74;
/// The three slots that carry skin (`01188b88`; slots 2, 3 and 4).
const SKIN_SLOTS: u32 = 0x0118_8b88;
/// The 20 slot names, `char*` each (`01188b98`).
const SLOT_NAMES: u32 = 0x0118_8b98;
/// For each slot, the bone index it attaches to, or -1 (`01188be8`).
const SLOT_BONES: u32 = 0x0118_8be8;

/// Type descriptors `__RTDynamicCast` is given: `TESForm`, `TESRace`,
/// `TESBoundObject` and `TESNPC`.
const TYPE_TES_FORM: u32 = 0x0118_3028;
const TYPE_TES_RACE: u32 = 0x0118_6370;
const TYPE_TES_BOUND_OBJECT: u32 = 0x0118_3108;
const TYPE_TES_NPC: u32 = 0x0118_3a1c;

/// The form type (`TESForm::cFormType`) `004ab400` and `004ab750` require
/// (a weapon form) and the one `LoadBipedParts` requires of the requester's
/// base form (an NPC).
const FORM_TYPE_WEAPON: u32 = 0x28;
const FORM_TYPE_NPC: u32 = 0x2a;
/// The form id `004ab400` treats specially when choosing the weapon model.
const FORM_ID_SPECIAL_WEAPON: u32 = 0x0017_35d4;

/// Texts: the root node name (`Bip01`), the log formats, the weapon bone's
/// attachment node name, the backpack node, the normal map suffix (`_n`),
/// `NULL` and the prefixes of the pip-boy node names.
const ROOT_NODE_NAME: u32 = 0x0101_e460;
const MISSING_BONE_FORMAT: u32 = 0x0101_f684;
const BACKPACK_NAME: u32 = 0x0101_f5cc;
const SLOT_NAME_FORMAT: u32 = 0x0101_f6a8;
const SKIN_ATTACHMENT_NAME: u32 = 0x0101_f724;
const NON_GEOMETRY_FORMAT: u32 = 0x0101_f6f4;
const SHOULD_BE_SKINNED_FORMAT: u32 = 0x0101_f6bc;
const NORMAL_MAP_SUFFIX: u32 = 0x0101_f6b8;
const UNNAMED_BONE_FORMAT: u32 = 0x0101_f738;
const NULL_TEXT: u32 = 0x0101_f7a8;
const PIPBOY_OFF_PREFIX: u32 = 0x0101_f7bc;
const PIPBOY_ON_PREFIX: u32 = 0x0101_f7b0;

// ---- Callees outside this unit --------------------------------------------------

/// `memset(destination, value, size)` (`00403d30`, cdecl).
const MEMSET: u32 = 0x0040_3d30;
/// `operator new(size)` (`00401000`).
const OPERATOR_NEW: u32 = 0x0040_1000;
/// `NiMemObject::operator new(size)` (`00aa13e0`).
const NI_OPERATOR_NEW: u32 = 0x00aa_13e0;
/// `__RTDynamicCast(object, 0, source type, target type, 0)`.
const RT_DYNAMIC_CAST: u32 = 0x00ec_43fb;
/// `__ehvec_ctor` and `__ehvec_dtor` (`00ec782f`, `00ec5fce`).
const VECTOR_CONSTRUCT: u32 = 0x00ec_782f;
const VECTOR_DESTRUCT: u32 = 0x00ec_5fce;
/// `_strnicmp`-style `(a, b, count)` compare (`00ec7ec0`), and the string
/// compare `(a, b)` (`00404dc0`).
const COMPARE_PREFIX: u32 = 0x00ec_7ec0;
const COMPARE_TEXT: u32 = 0x0040_4dc0;
/// `strlen` (`0044a670`, cdecl).
const TEXT_LENGTH: u32 = 0x0044_a670;
/// The log function `(format, ...)` (`005b5e40`, cdecl) and `sprintf` into a
/// `BSString` (`00406f60`: `(string, format, ...)`).
const LOG: u32 = 0x005b_5e40;
const SPRINTF: u32 = 0x0040_6f60;
/// `BSString` (8 bytes) constructor and destructor.
const STRING_INIT: u32 = 0x0040_37b0;
const STRING_FREE: u32 = 0x0040_37d0;
/// `BSString::BSString(const char *)` (`0040c0e0`).
const STRING_FROM_TEXT: u32 = 0x0040_c0e0;
/// `NiFixedString` constructor from `const char *` (`00438170`, returns the
/// fixed string) and destructor (`004381b0`).
const FIXED_STRING_INIT: u32 = 0x0043_8170;
const FIXED_STRING_FREE: u32 = 0x0043_81b0;
/// `NiObjectNET::SetName(const NiFixedString &)` (`00a5b950`).
const SET_NAME: u32 = 0x00a5_b950;
/// `NiPointer` operations: init from a pointer, assign, release, and the
/// folded getter `00559450`.
const NI_POINTER_INIT: u32 = 0x0063_3c90;
const NI_POINTER_SET: u32 = 0x0066_b0d0;
const NI_POINTER_RELEASE: u32 = 0x0045_cec0;
const READ_WORD: u32 = 0x0055_9450;
/// `NiNode` child accessors: the count of the child array (`0043b480`) and
/// the child at an index (`0043b4a0`).
const CHILD_COUNT: u32 = 0x0043_b480;
const CHILD_AT: u32 = 0x0043_b4a0;
/// `00413f40(node)` gives the address of the node's name field (`this+8`)
/// and `0043b1b0` turns it into the name text.
const NAME_FIELD: u32 = 0x0041_3f40;
const NAME_TEXT: u32 = 0x0043_b1b0;
/// `0043fad0(node)`: the `NiPointer` at `+0xbc` of a geometry node (its
/// skin or property data); `0043b230(data)` and `005495f0(node)` read
/// further fields of such objects.
const NODE_DATA_POINTER: u32 = 0x0043_fad0;
const NODE_DATA_FIELD: u32 = 0x0043_b230;
const NODE_SKIN_DATA: u32 = 0x0054_95f0;
/// `NiObjectNET::GetExtraData(key)`, `AddExtraData(key, data)`,
/// `RemoveExtraData(key)`.
const GET_EXTRA_DATA: u32 = 0x00a5_bdd0;
const ADD_EXTRA_DATA: u32 = 0x00a5_bc40;
const REMOVE_EXTRA_DATA: u32 = 0x00a5_be90;
/// `NiObject::Clone(cloning process)`, `Clone_ov2` and
/// `CreateDeepCopy(out NiPointer)`.
const NI_OBJECT_CLONE: u32 = 0x00a5_d2c0;
const NI_OBJECT_CLONE_OV2: u32 = 0x00a5_d680;
const NI_OBJECT_DEEP_COPY: u32 = 0x00a5_d510;
/// `TES::CreateDeepCopySameTextures(model, cloning process)`.
const DEEP_COPY_SAME_TEXTURES: u32 = 0x0045_7ba0;
/// `HasMorpherController(model)`.
const HAS_MORPHER_CONTROLLER: u32 = 0x004b_5bf0;
/// `NiAVObject::UpdateProperties` (`00a5a040`), the `NiNode` constructor
/// `(this, 0)` (`00a5ecb0`) and `BSFadeNode::SetLODMultType` (`00b4dec0`).
const UPDATE_PROPERTIES: u32 = 0x00a5_a040;
const NI_NODE_INIT: u32 = 0x00a5_ecb0;
const SET_LOD_MULT_TYPE: u32 = 0x00b4_dec0;
/// The form type getter (`TESForm::cFormType`, `00401170`) and the form id
/// getter (`0084e3a0`, `this+0xc`).
const FORM_TYPE: u32 = 0x0040_1170;
const FORM_ID: u32 = 0x0084_e3a0;
/// The reference's base form getter (`007af430`; the maps name it
/// `BGSSaveFormBuffer::GetForm`, a folded name).
const REFERENCE_FORM: u32 = 0x007a_f430;
/// `TESBipedModelForm::GetFormAsBipedModel` (`00480db0`, cdecl),
/// `FillsBipedSlot(slot, flag, flag)` (`00480af0`) and the model path getter
/// `(this)` `00480ce0`.
const GET_FORM_AS_BIPED_MODEL: u32 = 0x0048_0db0;
const FILLS_BIPED_SLOT: u32 = 0x0048_0af0;
const BIPED_MODEL_PATH: u32 = 0x0048_0ce0;
/// `TESModelTextureSwap::SwapTextures(node)` and
/// `SwapPlatformLanguageTextures(node)`.
const SWAP_TEXTURES: u32 = 0x0048_afe0;
const SWAP_PLATFORM_TEXTURES: u32 = 0x004b_7660;
/// `00522df0(form; flag, 1)`: the model a form wears (`TESModel*`).
const FORM_MODEL: u32 = 0x0052_2df0;
/// `008d8b00(form)` (named `MiddleHighProcess::GetHeadAnims`, a folded
/// name): the word at `+0x250` of a weapon form.
const FORM_ANIMS: u32 = 0x008d_8b00;
/// `0048cee0(model)`: whether a model has a path.
const MODEL_HAS_PATH: u32 = 0x0048_cee0;
/// The model loader methods: `LoadFile(path, 3, 1, 0, flag, 0)`,
/// `QueueTexture(path, a, b)`, the two `QueueModel(path, a, b, 3, 1, 0, 0)`
/// overloads and `0045a5e0(path)` (releases a queued model).
const LOAD_FILE: u32 = 0x0044_7080;
const QUEUE_TEXTURE: u32 = 0x0044_36c0;
const QUEUE_MODEL_FORM: u32 = 0x0044_3d30;
const QUEUE_MODEL: u32 = 0x0044_4040;
const RELEASE_MODEL: u32 = 0x0045_a5e0;
/// `BSShaderManager::GetModifiedTextureFilename(out, in, suffix, 1)`.
const MODIFIED_TEXTURE_FILENAME: u32 = 0x00b4_f5e0;
/// The race body texture helpers: `TESRace::GetBodyModTextureName`
/// (`006147e0`), `GetBodyModTextureFileName` (`006148f0`) and the skin
/// texture path `00614640(npc, slot, out)`.
const BODY_MOD_TEXTURE_NAME: u32 = 0x0061_47e0;
const BODY_MOD_TEXTURE_FILE_NAME: u32 = 0x0061_48f0;
const SKIN_TEXTURE_PATH: u32 = 0x0061_4640;
/// `006151f0(form; arg, slot)`: the item a form gives for a slot.
const FORM_SLOT_ITEM: u32 = 0x0061_51f0;
/// `00726070(object)`: reads the word at `+4`.
const READ_WORD_PLUS_4: u32 = 0x0072_6070;
/// The player: `GetBiped(first)` (`00950b00`), `Is1stPersonBiped(biped)`
/// (`00950b30`), `GetNode(first)` (`00950bb0`) and
/// `GetAnimation(first)` (`00950a60`).
const GET_BIPED: u32 = 0x0095_0b00;
const IS_FIRST_PERSON_BIPED: u32 = 0x0095_0b30;
const PLAYER_NODE: u32 = 0x0095_0bb0;
const PLAYER_ANIMATION: u32 = 0x0095_0a60;
/// Actor: `GetSavedAcquireObject` (`008d8520`, the word at `+0x68`),
/// `GetAnimation` (`008b70d0`), `IsWeaponDrawn` (`008a16d0`) and
/// `ReloadTargets(flag)` (`008b0b00`).
const SAVED_ACQUIRE_OBJECT: u32 = 0x008d_8520;
const ACTOR_ANIMATION: u32 = 0x008b_70d0;
const IS_WEAPON_DRAWN: u32 = 0x008a_16d0;
const RELOAD_TARGETS: u32 = 0x008b_0b00;
/// `Animation::BlendOut(4, 0)` (`004994f0`) and `FindSkinnedNode(node)`
/// (`004910d0`).
const BLEND_OUT: u32 = 0x0049_94f0;
const FIND_SKINNED_NODE: u32 = 0x0049_10d0;
/// Tiny flag getters of other units: the data handler's flag (`004226e0`),
/// the save-load object's (`0047c850`), `0042ce10` of the flags object and
/// `0043faf0`/`00408d60` of the settings.
const DATA_HANDLER_FLAG: u32 = 0x0042_26e0;
const SAVE_LOAD_FLAG: u32 = 0x0047_c850;
const FLAGS_OBJECT_FLAG: u32 = 0x0042_ce10;
const SETTING_BYTE_VALUE: u32 = 0x0043_faf0;
const SETTING_BYTE_POINTER: u32 = 0x0040_8d60;
/// The weapon helper `00450f90(node, flag)`.
const SET_WEAPON_FLAG: u32 = 0x0045_0f90;
/// `ItemChange::GetModSlots` (`004bd820`) and `HasModEffectActive`
/// (`004bd8d0`).
const GET_MOD_SLOTS: u32 = 0x004b_d820;
const HAS_MOD_EFFECT_ACTIVE: u32 = 0x004b_d8d0;
/// `00504e60(form)` and `00709c20(value)` (cdecl).
const WEAPON_MODEL_VALUE: u32 = 0x0050_4e60;
const SHOW_WEAPON: u32 = 0x0070_9c20;
/// `00571760(actor; form)`.
const ACTOR_SET_WEAPON: u32 = 0x0057_1760;
/// The dismemberment: `005d43c0(actor)` gives the extra data list,
/// `GetDismembermentExtra` (`0042e8c0`), `Dismembered(limb)` (`004303e0`)
/// and `BGSBodyPart::HideLimb(limb, node)` (`005e4730`).
const ACTOR_EXTRA_LIST: u32 = 0x005d_43c0;
const GET_DISMEMBERMENT_EXTRA: u32 = 0x0042_e8c0;
const LIMB_DISMEMBERED: u32 = 0x0043_03e0;
const HIDE_LIMB: u32 = 0x005e_4730;
/// `0055d520(actor)`: the text naming the actor in a log line.
const ACTOR_LOG_NAME: u32 = 0x0055_d520;
/// `0043fcd0(actor)`: the actor's 3D root, `00440460(node, translation)`,
/// `0043fa80(node, rotation)`.
const ACTOR_ROOT: u32 = 0x0043_fcd0;
const SET_TRANSLATION: u32 = 0x0044_0460;
const SET_ROTATION: u32 = 0x0043_fa80;
/// `NiMatrix3::MakeYRotation(angle)` (`0043f850`), `NiMatrix3::operator*`
/// (`0043f8d0`: `this` is the left matrix, then the result and the right
/// matrix), `006a9540(node)`, the address of the node's rotation, and the
/// matrix constructor `006815c0`.
const MAKE_Y_ROTATION: u32 = 0x0043_f850;
const MATRIX_MULTIPLY: u32 = 0x0043_f8d0;
const NODE_ROTATION: u32 = 0x006a_9540;
const MATRIX_INIT: u32 = 0x0068_15c0;
/// `TESNPC::GetFaceCoord(out)` (`00603ad0`) and the FaceGen model method
/// `0065a970(coords, node, 0)`.
const GET_FACE_COORD: u32 = 0x0060_3ad0;
const APPLY_FACE_COORDS: u32 = 0x0065_a970;
/// `BSFaceGenManager::ClearBodyTexturesFromPalette(form)`.
const CLEAR_BODY_PALETTE: u32 = 0x0065_70e0;
/// Constructor and destructor of one FaceGen coordinate (0x20 bytes).
const FACE_COORD_INIT: u32 = 0x0044_9610;
const FACE_COORD_FREE: u32 = 0x0044_9680;
/// `00653270(class, node)`.
const CAST_TO_CLASS: u32 = 0x0065_3270;
/// The shadow scene node table getter `00450b80(index)`, `RemoveObject`
/// (`00b5b1c0`), the node's parent (`009611e0`) and
/// `bhkWorld::RemoveObjects(node, 1, 0)` (`00c69ee0`).
const SHADOW_SCENE_NODE: u32 = 0x0045_0b80;
const SHADOW_REMOVE_OBJECT: u32 = 0x00b5_b1c0;
const NODE_PARENT: u32 = 0x0096_11e0;
const WORLD_REMOVE_OBJECTS: u32 = 0x00c6_9ee0;
/// `004b6ce0(node)`.
const PREPARE_DETACH: u32 = 0x004b_6ce0;
/// The deferred detach: `008c7aa0()` (whether detaching must be deferred),
/// `004537b0()` (the task queue) and `0087acb0(queue; node)`.
const MUST_DEFER: u32 = 0x008c_7aa0;
const TASK_QUEUE_GETTER: u32 = 0x0045_37b0;
const TASK_QUEUE_DETACH: u32 = 0x0087_acb0;
/// `004538a0(lock, 0)` and `004538c0(lock)`.
const LOCK_ENTER: u32 = 0x0045_38a0;
const LOCK_LEAVE: u32 = 0x0045_38c0;
/// `0049c680(list, &key, 0)` and `004700d0(list, &item)` (the weapon bone's
/// extra data list).
const FIND_IN_LIST: u32 = 0x0049_c680;
const ADD_TO_LIST: u32 = 0x0047_00d0;
/// `00528cb0(this, 0)` and `0044cbf0(this)` (a four-byte object built and
/// destroyed by `004abd30`), `0092c820(arg; object)`, `00437730(data; arg)`.
const QUEUE_OBJECT_INIT: u32 = 0x0052_8cb0;
const QUEUE_OBJECT_FREE: u32 = 0x0044_cbf0;
const QUEUE_OBJECT_USE: u32 = 0x0092_c820;
const SET_PARENT_OBJECT: u32 = 0x0043_7730;
/// The body part lights of a geometry's data: `005585e0(data)`,
/// `008041a0(owner)` (count) and `00825c00(data)` (array of items).
const BODY_PART_OWNER: u32 = 0x0055_85e0;
const BODY_PART_COUNT: u32 = 0x0080_41a0;
const BODY_PART_ARRAY: u32 = 0x0082_5c00;
/// `00c4b310(node, name, 1)`: the object named `name` under `node`.
const FIND_OBJECT_BY_NAME: u32 = 0x00c4_b310;
/// Functions of this unit after the 40 this session covers.
const FN_004ADDA0: u32 = 0x004a_dda0;
const FN_004ADDC0: u32 = 0x004a_ddc0;
const FN_004ADDE0: u32 = 0x004a_dde0;
const FN_004ADE00: u32 = 0x004a_de00;
const FN_004ADE20: u32 = 0x004a_de20;
const FN_004ADD50: u32 = 0x004a_dd50;
const FN_004ADD70: u32 = 0x004a_dd70;
const FN_004AE250: u32 = 0x004a_e250;
const FN_004AE790: u32 = 0x004a_e790;
const FN_004AE8A0: u32 = 0x004a_e8a0;
const FN_004AEDE0: u32 = 0x004a_ede0;
const FN_004AEE60: u32 = 0x004a_ee60;
const FN_004AEED0: u32 = 0x004a_eed0;
const FN_004AF240: u32 = 0x004a_f240;
const FN_004AF490: u32 = 0x004a_f490;
const FN_004AF950: u32 = 0x004a_f950;
const FN_004AFA20: u32 = 0x004a_fa20;
const FN_004AFA50: u32 = 0x004a_fa50;
const FN_004AFAD0: u32 = 0x004a_fad0;
const FN_004AFB20: u32 = 0x004a_fb20;
const FN_004AFB50: u32 = 0x004a_fb50;
const FN_004AFB80: u32 = 0x004a_fb80;
const FN_004AFBA0: u32 = 0x004a_fba0;
const FN_004AFC50: u32 = 0x004a_fc50;
const FN_004AFF00: u32 = 0x004a_ff00;

// ---- Small helpers --------------------------------------------------------------

/// `memset`.
fn memset(e: &mut Engine, destination: u32, value: u32, size: u32) {
    e.call(MEMSET, &args![destination, value, size]);
}

/// `NiPointer::NiPointer(T*)` on the four-byte `handle`.
fn ni_pointer_new(e: &mut Engine, handle: Ptr, value: Ptr) {
    e.call(NI_POINTER_INIT, &args![handle, value]);
}

/// `NiPointer::~NiPointer` on `handle`.
fn ni_pointer_release(e: &mut Engine, handle: Ptr) {
    e.call(NI_POINTER_RELEASE, &args![handle]);
}

/// `NiPointer::operator T*` (`00559450`): the pointer held in `handle`.
pub(crate) fn ni_pointer_get(e: &mut Engine, handle: Ptr) -> Ptr {
    e.call(READ_WORD, &args![handle]).ptr()
}

/// `NiPointer::operator=(T*)` on `handle`.
fn ni_pointer_assign(e: &mut Engine, handle: Ptr, value: Ptr) {
    e.call(NI_POINTER_SET, &args![handle, value]);
}

/// The number of children of `node` (`0043b480`).
fn child_count(e: &mut Engine, node: Ptr) -> u32 {
    e.call(CHILD_COUNT, &args![node]).u32()
}

/// The child of `node` at `index` (`0043b4a0`).
fn child_at(e: &mut Engine, node: Ptr, index: u32) -> Ptr {
    e.call(CHILD_AT, &args![node, index]).ptr()
}

/// The name text of `node` (`0043b1b0(00413f40(node))`).
fn node_name(e: &mut Engine, node: Ptr) -> Ptr {
    let field = e.call(NAME_FIELD, &args![node]).u32();
    e.call(NAME_TEXT, &args![field]).ptr()
}

/// `__RTDynamicCast(object, 0, source, target, 0)`.
fn dynamic_cast(e: &mut Engine, object: Ptr, source: u32, target: u32) -> Ptr {
    e.call(RT_DYNAMIC_CAST, &args![object, 0u32, source, target, 0u32])
        .ptr()
}

/// The base form of `requester` cast to `TESNPC` (null when it is not one).
fn requester_npc(e: &mut Engine, requester: Ptr) -> Ptr {
    let form = e.call(REFERENCE_FORM, &args![requester]).ptr();
    dynamic_cast(e, form, TYPE_TES_BOUND_OBJECT, TYPE_TES_NPC)
}

/// `PlayerCharacter::GetBiped(first)`.
fn player_biped(e: &mut Engine, first: u8) -> u32 {
    let player = e.global::<u32>(PLAYER);
    e.call(GET_BIPED, &args![player, first]).u32()
}

/// A block for a local the game keeps on its stack; `stack_free` releases it.
fn stack_alloc(e: &mut Engine, size: u32) -> Ptr {
    Ptr::new(e.mem.alloc(size))
}

fn stack_free(e: &mut Engine, block: Ptr) {
    e.mem.free(block.addr());
}

/// Whether the string setting object at `setting` holds a non-empty text.
fn setting_text_set(e: &mut Engine, setting: u32) -> bool {
    let text = e.call(SETTING_BYTE_POINTER, &args![setting]).u32();
    e.mem.u8(text) != 0
}

/// `004ae8a0(this; slot)`: whether the slot holds FaceGen data (a function
/// of this unit that a later session translates).
fn is_face_gen_slot(e: &mut Engine, this: Ptr<BipedAnim>, slot: u32) -> bool {
    e.call(FN_004AE8A0, &args![this, slot]).bool()
}

/// Adds `item` to the array `array` (`NiTArray<NiPointer<NiAVObject>>::
/// Add`, `004afc50`) through a temporary `NiPointer`.
fn array_add(e: &mut Engine, array: Ptr, item: Ptr) {
    let handle = stack_alloc(e, 4);
    ni_pointer_new(e, handle, item);
    e.call(FN_004AFC50, &args![array, handle]);
    ni_pointer_release(e, handle);
    stack_free(e, handle);
}

/// Names `node` `"<slot name> <form name> (<form id>)"` (the text built with
/// `sprintf` into a `BSString`, turned into an `NiFixedString`), as the
/// weapon and the other parts are named.
fn name_part_node(e: &mut Engine, node: Ptr, form: Ptr, slot: u32) {
    let text = stack_alloc(e, 8);
    e.call(STRING_INIT, &args![text]);
    name_part_node_with(e, node, form, slot, text);
    e.call(STRING_FREE, &args![text]);
    stack_free(e, text);
}

/// [`name_part_node`] with the caller's `BSString` `text`.
fn name_part_node_with(e: &mut Engine, node: Ptr, form: Ptr, slot: u32, text: Ptr) {
    let id = e.call(FORM_ID, &args![form]).u32();
    // Virtual slot 0x130 of the form: its name.
    let name = e.vcall(form.addr(), 0x130, &[]).u32();
    let slot_name = e.global::<u32>(SLOT_NAMES + 4 * slot);
    e.call(SPRINTF, &args![text, SLOT_NAME_FORMAT, slot_name, name, id]);
    let chars = ni_pointer_get(e, text);
    let fixed = stack_alloc(e, 4);
    let fixed_name = e.call(FIXED_STRING_INIT, &args![fixed, chars]).u32();
    e.call(SET_NAME, &args![node, fixed_name]);
    e.call(FIXED_STRING_FREE, &args![fixed]);
    stack_free(e, fixed);
}

/// Tells the actor's middle-high process (`008d8520`) whether the weapon is
/// drawn, with the biped, the animation and the actor (virtual slot 0x1cc;
/// the player's animation `00950a60(1)` when `this` is the player's first
/// person biped, `008b70d0` otherwise).
fn fire_weapon_state(e: &mut Engine, this: Ptr<BipedAnim>, requester: Ptr) {
    let player = e.global::<u32>(PLAYER);
    let on_player_biped = requester.addr() == player && player_biped(e, 1) == this.addr();
    let process = e.call(SAVED_ACQUIRE_OBJECT, &args![requester]).u32();
    let animation = if on_player_biped {
        e.call(PLAYER_ANIMATION, &args![player, 1u8]).u32()
    } else {
        e.call(ACTOR_ANIMATION, &args![requester]).u32()
    };
    let drawn = e.call(IS_WEAPON_DRAWN, &args![requester]).bool();
    e.vcall(process, 0x1cc, &args![drawn, this, animation, requester]);
}

/// The part shared by `RemoveBipedWeapon` and `004ab750`: when the actor has
/// a process and neither the data handler nor the save-load object says
/// otherwise, and `this` is not the player's first person biped, the
/// actor's targets are reloaded with `flag`.
fn reload_targets_unless_blocked(
    e: &mut Engine,
    this: Ptr<BipedAnim>,
    requester: Ptr,
    process: u32,
    flag: u8,
) {
    if process == 0 {
        return;
    }
    let handler = e.global::<u32>(DATA_HANDLER);
    if e.call(DATA_HANDLER_FLAG, &args![handler]).bool() {
        return;
    }
    let save_load = e.global::<u32>(SAVE_LOAD);
    if e.call(SAVE_LOAD_FLAG, &args![save_load]).bool() {
        return;
    }
    if this.addr() != player_biped(e, 1) {
        e.call(RELOAD_TARGETS, &args![requester, flag]);
    }
}

// ---- Functions ----------------------------------------------------------------------

// Translated from 004aaca0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BipedAnim::BipedAnim` (Xbox PDB): clears the object and the buffered
/// slots, stores the requester and, given a root node, finds the bones
/// (`InitRoot`). Returns `this`.
pub fn biped_anim_biped_anim(
    e: &mut Engine,
    this: Ptr<BipedAnim>,
    requester: Ptr,
    root: Ptr,
) -> Ptr<BipedAnim> {
    memset(e, this.addr(), 0, 0x2b4);
    memset(e, buffered(this, 0).addr(), 0, 0x140);
    e.set(this, BipedAnim::m_pRequester, requester);
    if !root.is_null() {
        biped_anim_init_root(e, this, root);
    }
    this
}

// Translated from 004aad00 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BipedAnim::InitRoot` (Xbox PDB): stores the node named `Bip01` under
/// `root`, then looks up the five bones by name; a bone found (and a node)
/// gets flag bit 0 and its node stored, one that is missing is logged.
pub fn biped_anim_init_root(e: &mut Engine, this: Ptr<BipedAnim>, root: Ptr) {
    let root_node = fn_004aae30(e, root, Ptr::new(ROOT_NODE_NAME));
    e.set(this, BipedAnim::root, root_node);
    for index in 0..BONE_COUNT {
        let name = e.global::<u32>(BONE_NAMES + 4 * index);
        let found = fn_004aae30(e, root, Ptr::new(name));
        let handle = stack_alloc(e, 4);
        ni_pointer_new(e, handle, found);
        let mut is_node = false;
        if !ni_pointer_get(e, handle).is_null() {
            let object = ni_pointer_get(e, handle);
            // Virtual slot 0xc of the object: non-zero for a node.
            if e.vcall(object.addr(), 0x0c, &[]).u32() != 0 {
                is_node = true;
            }
        }
        if is_node {
            let flags = e.get(bone(this, index), BipedBone::cFlags);
            e.set(bone(this, index), BipedBone::cFlags, flags | 1);
            let object = ni_pointer_get(e, handle);
            e.set(bone(this, index), BipedBone::pParent, object);
        } else {
            let owner = node_name(e, root);
            e.call(LOG, &args![MISSING_BONE_FORMAT, name, owner]);
        }
        ni_pointer_release(e, handle);
        stack_free(e, handle);
    }
}

// Translated from 004aae30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Looks up the object named `name` under `node` (`00c4b310(node, name, 1)`).
pub fn fn_004aae30(e: &mut Engine, node: Ptr, name: Ptr) -> Ptr {
    e.call(FIND_OBJECT_BY_NAME, &args![node, name, 1u32]).ptr()
}

// Translated from 004aae50 (decompiled, FalloutNV.exe 1.4.0.525)
/// A wrapper of [`biped_anim_remove_all_parts`] (`ECX` is passed on).
pub fn fn_004aae50(e: &mut Engine, this: Ptr<BipedAnim>) {
    biped_anim_remove_all_parts(e, this);
}

// Translated from 004aae70 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BipedAnim::RemoveAllParts` (Xbox PDB): with the model loader's queue
/// lock (`004afb80` on the loader's first object) and the part lock held,
/// removes every slot and every buffered slot, clearing their forms.
pub fn biped_anim_remove_all_parts(e: &mut Engine, this: Ptr<BipedAnim>) {
    let loader = e.global::<u32>(MODEL_LOADER);
    fn_004aaef0(e, Ptr::new(loader));
    e.call(LOCK_ENTER, &args![PART_LOCK, 0u32]);
    for slot in 0..SLOT_COUNT {
        fn_004aaff0(e, this, slot, 1, Ptr::NULL);
        fn_004ab020(e, this, buffered(this, slot), 1, Ptr::NULL);
    }
    e.call(LOCK_LEAVE, &args![PART_LOCK]);
    let loader = e.global::<u32>(MODEL_LOADER);
    fn_004aaf10(e, Ptr::new(loader));
}

// Translated from 004aaef0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls `004afb80` on the object the `ModelLoader` holds in its first
/// field.
pub fn fn_004aaef0(e: &mut Engine, this: Ptr) {
    let first = e.mem.u32(this.addr());
    e.call(FN_004AFB80, &args![first]);
}

// Translated from 004aaf10 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls `004afba0` on the object the `ModelLoader` holds in its first
/// field.
pub fn fn_004aaf10(e: &mut Engine, this: Ptr) {
    let first = e.mem.u32(this.addr());
    e.call(FN_004AFBA0, &args![first]);
}

// Translated from 004aaf30 (decompiled, FalloutNV.exe 1.4.0.525)
/// The slot whose form is `form`: its address, or null (also for a null
/// form).
pub fn fn_004aaf30(e: &mut Engine, this: Ptr<BipedAnim>, form: Ptr) -> Ptr<BipedObject> {
    if form.is_null() {
        return Ptr::NULL;
    }
    for slot in 0..SLOT_COUNT {
        let candidate = object(this, slot);
        if e.get(candidate, BipedObject::pParent) == form {
            return candidate;
        }
    }
    Ptr::NULL
}

// Translated from 004aaf90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Clears every slot whose form is `form` (`004aaff0(slot, 1, 0)`).
pub fn fn_004aaf90(e: &mut Engine, this: Ptr<BipedAnim>, form: Ptr) {
    if form.is_null() {
        return;
    }
    for slot in 0..SLOT_COUNT {
        if e.get(object(this, slot), BipedObject::pParent) == form {
            fn_004aaff0(e, this, slot, 1, Ptr::NULL);
        }
    }
}

// Translated from 004aaff0 (decompiled, FalloutNV.exe 1.4.0.525)
/// [`fn_004ab020`] on slot `slot`.
pub fn fn_004aaff0(e: &mut Engine, this: Ptr<BipedAnim>, slot: u32, clear: u8, new_part: Ptr) {
    fn_004ab020(e, this, object(this, slot), clear, new_part);
}

// Translated from 004ab020 (decompiled, FalloutNV.exe 1.4.0.525)
/// Empties the slot `entry`: when it holds a loaded clone, removes the
/// decal data from the weapon bone, detaches the clone (at once, or queued
/// when `008c7aa0` says detaching must be deferred), releases the model with
/// the loader and clears the clone. With `clear` non-zero the form is
/// cleared too and the part replaced by `new_part`.
pub fn fn_004ab020(
    e: &mut Engine,
    this: Ptr<BipedAnim>,
    entry: Ptr<BipedObject>,
    clear: u8,
    new_part: Ptr,
) {
    let clone = e.get(entry, BipedObject::pPartClone);
    if !clone.is_null() {
        biped_anim_recursively_remove_all_decal_placement_vector_data_from_weapon_bone(
            e, this, clone,
        );
        if !e.call(MUST_DEFER, &[]).bool() {
            biped_anim_run_biped_3d_detach(e, clone);
        } else {
            let queue = e.call(TASK_QUEUE_GETTER, &[]).u32();
            e.call(TASK_QUEUE_DETACH, &args![queue, clone]);
        }
        let part = e.get(entry, BipedObject::pPart);
        // Virtual slot 0x14 of the model: its path.
        let path = e.vcall(part.addr(), 0x14, &[]).u32();
        let loader = e.global::<u32>(MODEL_LOADER);
        e.call(RELEASE_MODEL, &args![loader, path]);
        e.set(entry, BipedObject::pPartClone, Ptr::NULL);
    }
    if clear != 0 {
        e.set(entry, BipedObject::pParent, Ptr::NULL);
        e.set(entry, BipedObject::pPart, new_part);
    }
}

// Translated from 004ab0c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BipedAnim::RunBiped3DDetach` (Xbox PDB): detaches `node`: prepares it
/// (`004b6ce0`), removes it from the first shadow scene node and, when it has
/// a parent, removes its physics objects and the node from the parent
/// (virtual slot 0xe8 of the parent).
pub fn biped_anim_run_biped_3d_detach(e: &mut Engine, node: Ptr) {
    if node.is_null() {
        return;
    }
    e.call(PREPARE_DETACH, &args![node]);
    let shadow = e.call(SHADOW_SCENE_NODE, &args![0u32]).u32();
    e.call(SHADOW_REMOVE_OBJECT, &args![shadow, node]);
    if e.call(NODE_PARENT, &args![node]).u32() != 0 {
        e.call(WORLD_REMOVE_OBJECTS, &args![node, 1u32, 0u32]);
        let parent = e.call(NODE_PARENT, &args![node]).u32();
        e.vcall(parent, 0xe8, &args![node]);
    }
}

// Translated from 004ab130 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BipedAnim::RecursivelyRemoveAllDecalPlacementVectorDataFromWeaponBone`
/// (Xbox PDB): looks up the extra data the weapon bone (bone 1) keeps under
/// the key at `011c61e4`; for a geometry node (virtual slot 0x1c) that has
/// data (`0043fad0`) and is found in that extra data's list (at `+0x34`),
/// removes the extra data from the bone; otherwise recurses through the
/// node's children (virtual slot 0xc gives the node).
pub fn biped_anim_recursively_remove_all_decal_placement_vector_data_from_weapon_bone(
    e: &mut Engine,
    this: Ptr<BipedAnim>,
    node: Ptr,
) {
    if node.is_null() {
        return;
    }
    let weapon_bone = fn_004ab230(e, this, 1);
    let key = fn_004ab220(e);
    let extra = e.call(GET_EXTRA_DATA, &args![weapon_bone, key]).ptr::<()>();
    if extra.is_null() {
        return;
    }
    if e.vcall(node.addr(), 0x1c, &[]).u32() != 0 {
        if e.call(NODE_DATA_POINTER, &args![node]).u32() != 0 {
            let key_slot = stack_alloc(e, 4);
            e.mem.set_u32(key_slot.addr(), node.addr());
            let found = e
                .call(FIND_IN_LIST, &args![extra.byte_add(0x34), key_slot, 0u32])
                .u32();
            stack_free(e, key_slot);
            if found != 0 {
                let key = fn_004ab220(e);
                e.call(REMOVE_EXTRA_DATA, &args![weapon_bone, key]);
            }
        }
    } else {
        let children = e.vcall(node.addr(), 0x0c, &[]).ptr::<()>();
        if !children.is_null() {
            let mut index = 0;
            while index < child_count(e, children) {
                let child = child_at(e, children, index);
                biped_anim_recursively_remove_all_decal_placement_vector_data_from_weapon_bone(
                    e, this, child,
                );
                index += 1;
            }
        }
    }
}

// Translated from 004ab220 (decompiled, FalloutNV.exe 1.4.0.525)
/// The key of the extra data kept on the weapon bone (the global at
/// `011c61e4`).
pub fn fn_004ab220(e: &mut Engine) -> Ptr {
    Ptr::new(e.global::<u32>(EXTRA_DATA_KEY))
}

// Translated from 004ab230 (decompiled, FalloutNV.exe 1.4.0.525)
/// The node of bone `index` (`bone[index].pParent`).
pub fn fn_004ab230(e: &mut Engine, this: Ptr<BipedAnim>, index: u32) -> Ptr {
    e.get(bone(this, index), BipedBone::pParent)
}

// Translated from 004ab250 (decompiled, FalloutNV.exe 1.4.0.525)
/// Resets the slots and puts `form` into the three skin slots. When no
/// buffered slot is in use the slots are first copied to the buffered ones
/// (slot by slot for an actor whose virtual slot 0x1d0 is non-zero, the
/// slots are then cleared one by one; otherwise both arrays are cleared at
/// once). For each of the three skin slots that has no bone or whose bone
/// was found, the item `006151f0` gives for `form` is assigned by
/// `004abad0`.
pub fn fn_004ab250(e: &mut Engine, this: Ptr<BipedAnim>, form: Ptr, argument: u32) {
    if form.is_null() {
        return;
    }
    let mut buffer_empty = true;
    for slot in 0..SLOT_COUNT {
        if !e.get(buffered(this, slot), BipedObject::pParent).is_null() {
            buffer_empty = false;
        }
    }
    let requester = e.get(this, BipedAnim::m_pRequester);
    if !requester.is_null() && e.vcall(requester.addr(), 0x1d0, &[]).u32() != 0 {
        for slot in 0..SLOT_COUNT {
            if buffer_empty {
                let from = object(this, slot);
                let to = buffered(this, slot);
                for word in 0..4 {
                    let value = e.mem.u32(from.addr() + 4 * word);
                    e.mem.set_u32(to.addr() + 4 * word, value);
                }
            }
            memset(e, object(this, slot).addr(), 0, 0x10);
        }
    } else {
        if buffer_empty {
            memset(e, buffered(this, 0).addr(), 0, 0x140);
        }
        memset(e, object(this, 0).addr(), 0, 0x140);
    }
    for index in 0..3 {
        let slot = e.global::<u32>(SKIN_SLOTS + 4 * index);
        let bone_index = e.global::<u32>(SLOT_BONES + 4 * slot);
        if bone_index == 0xffff_ffff || e.get(bone(this, bone_index), BipedBone::cFlags) & 1 != 0 {
            let slot = e.global::<u32>(SKIN_SLOTS + 4 * index);
            let item = e.call(FORM_SLOT_ITEM, &args![form, argument, slot]).ptr();
            fn_004abad0(e, this, form, item, slot);
        }
    }
}

// Translated from 004ab400 (decompiled, FalloutNV.exe 1.4.0.525)
/// Chooses the weapon form and model for slot 5: for a weapon form (type
/// 0x28) that is not already loaded, clears slot 5 and stores the form and
/// the model `00522df0` gives; for the player's biped (first or third
/// person) the model is the weapon's animation model (`+0x30` of the item
/// `008d8b00` gives, or of the item `004ab500` gives for the flags when the
/// form id is not `0x1735d4`).
pub fn fn_004ab400(e: &mut Engine, this: Ptr<BipedAnim>, form: Ptr, flags: u8) {
    if form.is_null() {
        return;
    }
    if e.call(FORM_TYPE, &args![form]).u32() != FORM_TYPE_WEAPON {
        return;
    }
    let weapon = object(this, WEAPON_SLOT);
    if form == e.get(weapon, BipedObject::pParent)
        && !e.get(weapon, BipedObject::pPartClone).is_null()
    {
        return;
    }
    fn_004aaff0(e, this, WEAPON_SLOT, 1, Ptr::NULL);
    let mut model = e.call(FORM_MODEL, &args![form, flags, 1u32]).ptr::<()>();
    let on_player_biped = this.addr() == player_biped(e, 1) || this.addr() == player_biped(e, 0);
    if on_player_biped {
        let mut anims = e.call(FORM_ANIMS, &args![form]).ptr::<()>();
        if flags != 0 && e.call(FORM_ID, &args![form]).u32() != FORM_ID_SPECIAL_WEAPON {
            let chosen = fn_004ab500(e, form, flags);
            if chosen != 0 {
                anims = Ptr::new(chosen);
            }
        }
        if !anims.is_null() {
            model = anims.byte_add(0x30);
        }
    }
    e.set(weapon, BipedObject::pParent, form);
    e.set(weapon, BipedObject::pPart, model);
}

// Translated from 004ab500 (decompiled, FalloutNV.exe 1.4.0.525)
/// The word of the weapon form `form` that the low three bits of `flags`
/// select among the eight words from `+0x250` to `+0x26c` (bit 0 and bit 1
/// and bit 2 each pick a variant).
pub fn fn_004ab500(e: &mut Engine, form: Ptr, flags: u8) -> u32 {
    let offset = match (flags & 1 != 0, flags & 2 != 0, flags & 4 != 0) {
        (false, false, false) => 0x250,
        (false, false, true) => 0x25c,
        (false, true, false) => 0x258,
        (false, true, true) => 0x264,
        (true, false, false) => 0x254,
        (true, false, true) => 0x268,
        (true, true, false) => 0x260,
        (true, true, true) => 0x26c,
    };
    e.mem.u32(form.addr() + offset)
}

// Translated from 004ab5b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BipedAnim::RemoveBipedWeapon` (Xbox PDB): lets go of the weapon: clears
/// the weapon flag of the clone of slot 7's part when slot 5's form has the
/// flag, empties the buffered slots holding the buffered weapon form and
/// slot 5, and, for an actor (virtual slot 0x100), reloads its targets and
/// removes the backpack node from its parent.
pub fn biped_anim_remove_biped_weapon(e: &mut Engine, this: Ptr<BipedAnim>) {
    let weapon_form = e.get(object(this, WEAPON_SLOT), BipedObject::pParent);
    if !weapon_form.is_null() && fn_004ab730(e, weapon_form) {
        let clone = e.get(object(this, 7), BipedObject::pPartClone);
        if !clone.is_null() {
            e.call(SET_WEAPON_FLAG, &args![clone, 0u32]);
        }
    }
    let buffered_form = e.get(buffered(this, WEAPON_SLOT), BipedObject::pParent);
    if !buffered_form.is_null() {
        for slot in 0..SLOT_COUNT {
            if e.get(buffered(this, slot), BipedObject::pParent) == buffered_form {
                fn_004ab020(e, this, buffered(this, slot), 1, Ptr::NULL);
            }
        }
    }
    fn_004aaff0(e, this, WEAPON_SLOT, 1, Ptr::NULL);
    let requester = e.get(this, BipedAnim::m_pRequester);
    if e.vcall(requester.addr(), 0x100, &[]).bool() {
        let process = e.call(SAVED_ACQUIRE_OBJECT, &args![requester]).u32();
        reload_targets_unless_blocked(e, this, requester, process, 0);
        let root = e.get(this, BipedAnim::root);
        let backpack = fn_004aae30(e, root, Ptr::new(BACKPACK_NAME));
        if !backpack.is_null() {
            let parent = e.call(NODE_PARENT, &args![backpack]).u32();
            e.vcall(parent, 0xe8, &args![backpack]);
        }
    }
}

// Translated from 004ab730 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether bit 4 of the byte at `+0x100` of `form` is set.
pub fn fn_004ab730(e: &mut Engine, form: Ptr) -> bool {
    e.mem.u8(form.addr() + 0x100) & 0x10 != 0
}

// Translated from 004ab750 (decompiled, FalloutNV.exe 1.4.0.525)
/// Loads the weapon `form` into slot 5: for a weapon form not already
/// loaded, chooses the form and model (`004ab400`), loads and attaches it
/// (`004aeed0`; for the player's own actor with the node `00950bb0` gives),
/// names the loaded node `"<slot name> <form name> (<form id>)"`, applies
/// the weapon flag and, for an actor, reloads its targets, blends out the
/// animation and tells the middle-high process the weapon state (virtual
/// slot 0x1cc).
pub fn fn_004ab750(e: &mut Engine, this: Ptr<BipedAnim>, form: Ptr, flags: u8) {
    if form.is_null() {
        return;
    }
    if e.call(FORM_TYPE, &args![form]).u32() != FORM_TYPE_WEAPON {
        return;
    }
    let weapon = object(this, WEAPON_SLOT);
    if form == e.get(weapon, BipedObject::pParent)
        && !e.get(weapon, BipedObject::pPartClone).is_null()
    {
        return;
    }
    fn_004ab400(e, this, form, flags);
    let requester = e.get(this, BipedAnim::m_pRequester);
    let player = e.global::<u32>(PLAYER);
    let weapon_form = e.get(weapon, BipedObject::pParent);
    let weapon_model = e.get(weapon, BipedObject::pPart);
    let node = if requester.addr() == player {
        let first_person = e.call(IS_FIRST_PERSON_BIPED, &args![player, this]).u8();
        e.call(PLAYER_NODE, &args![player, first_person]).u32()
    } else {
        0
    };
    let loaded = e
        .call(
            FN_004AEED0,
            &args![weapon_form, weapon_model, WEAPON_SLOT, requester, node],
        )
        .ptr::<()>();
    e.set(weapon, BipedObject::pPartClone, loaded);
    if loaded.is_null() {
        return;
    }
    let weapon_form = e.get(weapon, BipedObject::pParent);
    name_part_node(e, loaded, weapon_form, WEAPON_SLOT);
    let weapon_clone = e.get(object(this, 7), BipedObject::pPartClone);
    if fn_004ab730(e, form) && !weapon_clone.is_null() {
        e.call(SET_WEAPON_FLAG, &args![weapon_clone, 1u32]);
    }
    if e.vcall(requester.addr(), 0x100, &[]).bool() {
        let process = e.call(SAVED_ACQUIRE_OBJECT, &args![requester]).u32();
        reload_targets_unless_blocked(e, this, requester, process, 1);
        let flags_object = e.global::<u32>(FLAGS_OBJECT);
        if process != 0 && !e.call(FLAGS_OBJECT_FLAG, &args![flags_object]).bool() {
            let on_player_biped = requester.addr() == player && player_biped(e, 1) == this.addr();
            let animation = if on_player_biped {
                e.call(PLAYER_ANIMATION, &args![player, 1u8]).u32()
            } else {
                e.call(ACTOR_ANIMATION, &args![requester]).u32()
            };
            if animation != 0 {
                e.call(BLEND_OUT, &args![animation, 4u32, 0u32]);
            }
            fire_weapon_state(e, this, requester);
        }
    }
}

// Translated from 004abad0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Assigns `form` (with its item `item`) to biped slots. A form that is a
/// biped model (`GetFormAsBipedModel`) is put into the first slot it fills
/// (`FillsBipedSlot(slot, 1, 0)`), after the slots it fills that hold a
/// different form are emptied (by slot for a race, by form otherwise); a
/// race (`TESRace`) goes into slot `slot` alone. For the player's first
/// person biped the item may be replaced by the one `004afa20` gives for the
/// slot (when `004af950` says so).
pub fn fn_004abad0(e: &mut Engine, this: Ptr<BipedAnim>, form: Ptr, item: Ptr, slot: u32) {
    let mut item = item;
    let biped_model = e.call(GET_FORM_AS_BIPED_MODEL, &args![form]).ptr::<()>();
    let race = dynamic_cast(e, form, TYPE_TES_FORM, TYPE_TES_RACE);
    let buffer = stack_alloc(e, 0x104);
    let mut replace_item = false;
    if !item.is_null() {
        let player = e.global::<u32>(PLAYER);
        if e.call(IS_FIRST_PERSON_BIPED, &args![player, this]).bool() {
            replace_item = e.call(FN_004AF950, &args![this, item, buffer]).bool();
        }
    }
    if !biped_model.is_null() {
        for index in 0..SLOT_COUNT {
            let fills = e
                .call(FILLS_BIPED_SLOT, &args![biped_model, index, 0u32, 0u32])
                .bool();
            if fills && e.get(object(this, index), BipedObject::pParent) != form {
                let current = e.get(object(this, index), BipedObject::pParent);
                if !current.is_null()
                    && !dynamic_cast(e, current, TYPE_TES_FORM, TYPE_TES_RACE).is_null()
                {
                    fn_004aaff0(e, this, index, 1, Ptr::NULL);
                } else {
                    fn_004aaf90(e, this, current);
                }
            }
        }
        for index in 0..SLOT_COUNT {
            if e.call(FILLS_BIPED_SLOT, &args![biped_model, index, 1u32, 0u32])
                .bool()
            {
                if replace_item {
                    item = e.call(FN_004AFA20, &args![this, index, buffer]).ptr();
                }
                e.set(object(this, index), BipedObject::pParent, form);
                e.set(object(this, index), BipedObject::pPart, item);
                break;
            }
        }
    } else if !race.is_null() {
        fn_004aaff0(e, this, slot, 1, Ptr::NULL);
        if replace_item {
            item = e.call(FN_004AFA20, &args![this, slot, buffer]).ptr();
        }
        e.set(object(this, slot), BipedObject::pParent, form);
        e.set(object(this, slot), BipedObject::pPart, item);
    }
    stack_free(e, buffer);
}

// Translated from 004abd30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Queues the models and textures of the parts with the model loader. For
/// each slot with a part, adds its part flags (`004abf80`) to a mask; when
/// the slot is a FaceGen slot (`004ae8a0`) and the settings ask for it the
/// FaceGen files are queued (`004aee60`), otherwise the part's model (and
/// the player's alternative `004afa50` gives) and the biped model's own path
/// are queued. Finally the object `0092c820` is told and the skin textures
/// of the mask are queued (`004abfe0`). `first` and `third` are the two
/// arguments passed on to every queueing call, `second` the object told at
/// the end.
pub fn fn_004abd30(e: &mut Engine, this: Ptr<BipedAnim>, first: u32, second: Ptr, third: u32) {
    let object_state = stack_alloc(e, 4);
    e.call(QUEUE_OBJECT_INIT, &args![object_state, 0u32]);
    let mut flags = 0u32;
    let requester = e.get(this, BipedAnim::m_pRequester);
    let npc = requester_npc(e, requester);
    if !npc.is_null() {
        for slot in 0..SLOT_COUNT {
            let part = e.get(object(this, slot), BipedObject::pPart);
            if part.is_null() || part.addr() == 0xffff_ffff {
                continue;
            }
            flags |= fn_004abf80(e, part);
            if is_face_gen_slot(e, this, slot)
                && setting_text_set(e, SETTING_FACE)
                && e.call(SETTING_BYTE_VALUE, &[]).bool()
                && fn_004abfa0(e)
            {
                e.call(FN_004AEE60, &args![this, first, third, object_state, slot]);
                continue;
            }
            let loader = e.global::<u32>(MODEL_LOADER);
            if requester.addr() == e.global::<u32>(PLAYER) {
                let alternative = e.call(FN_004AFA50, &args![this, slot, part]).u32();
                if alternative != part.addr() {
                    e.call(
                        QUEUE_MODEL_FORM,
                        &args![loader, alternative, first, third, 3u32, 1u32, 0u32, 0u32],
                    );
                }
            }
            e.call(
                QUEUE_MODEL_FORM,
                &args![loader, part, first, third, 3u32, 1u32, 0u32, 0u32],
            );
            let form = e.get(object(this, slot), BipedObject::pParent);
            let biped_model = e.call(GET_FORM_AS_BIPED_MODEL, &args![form]).u32();
            if biped_model != 0 {
                let path = e.call(BIPED_MODEL_PATH, &args![biped_model]).u32();
                if e.mem.i8(path) != 0 {
                    e.call(
                        QUEUE_MODEL,
                        &args![loader, path, first, third, 3u32, 1u32, 0u32, 0u32],
                    );
                }
            }
        }
    }
    e.call(QUEUE_OBJECT_USE, &args![second, object_state]);
    fn_004abfe0(e, this, flags, first, third);
    e.call(QUEUE_OBJECT_FREE, &args![object_state]);
    stack_free(e, object_state);
}

// Translated from 004abf80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Bits 1 to 3 of the byte at `+0x14` of `item`.
pub fn fn_004abf80(e: &mut Engine, item: Ptr) -> u32 {
    (e.mem.u8(item.addr() + 0x14) & 0x0e) as u32
}

// Translated from 004abfa0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the two settings at `011d5a10` and `011d5a44` both hold a
/// non-empty text.
pub fn fn_004abfa0(e: &mut Engine) -> bool {
    setting_text_set(e, SETTING_PAIR_FIRST) && setting_text_set(e, SETTING_PAIR_SECOND)
}

// Translated from 004abfe0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Queues the skin textures for the parts in `mask` (bit 1: slot 2, bit 2:
/// slot 4, bit 3: slot 3) and the race's body modification texture, for the
/// requester's NPC.
pub fn fn_004abfe0(e: &mut Engine, this: Ptr<BipedAnim>, mask: u32, first: u32, second: u32) {
    if mask == 0 {
        return;
    }
    let requester = e.get(this, BipedAnim::m_pRequester);
    let npc = requester_npc(e, requester);
    if npc.is_null() {
        return;
    }
    if mask & 2 != 0 {
        biped_anim_queue_skin_texture(e, this, npc, 2, first, second);
    }
    if mask & 4 != 0 {
        biped_anim_queue_skin_texture(e, this, npc, 4, first, second);
    }
    if mask & 8 != 0 {
        biped_anim_queue_skin_texture(e, this, npc, 3, first, second);
    }
    let name = stack_alloc(e, 0x104);
    let file = stack_alloc(e, 0x104);
    let race = fn_004ac110(e, npc);
    e.call(BODY_MOD_TEXTURE_NAME, &args![race, npc, name]);
    let race = fn_004ac110(e, npc);
    if e.call(BODY_MOD_TEXTURE_FILE_NAME, &args![race, npc, name, file])
        .bool()
    {
        let loader = e.global::<u32>(MODEL_LOADER);
        e.call(QUEUE_TEXTURE, &args![loader, file, first, second]);
    }
    stack_free(e, file);
    stack_free(e, name);
}

// Translated from 004ac110 (decompiled, FalloutNV.exe 1.4.0.525)
/// The word at `+0x110` of the NPC `npc` (its race, `this` of the `TESRace`
/// methods), read with `00726070` on `npc + 0x10c`.
pub fn fn_004ac110(e: &mut Engine, npc: Ptr) -> u32 {
    e.call(READ_WORD_PLUS_4, &args![npc.byte_add(0x10c)]).u32()
}

// Translated from 004ac130 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BipedAnim::QueueSkinTexture` (Xbox PDB): when the NPC's race has a skin
/// texture for `slot` (`00614640`), queues it with the model loader and the
/// matching normal map (the path `GetModifiedTextureFilename` gives for the
/// suffix `_n`, when it is not empty).
pub fn biped_anim_queue_skin_texture(
    e: &mut Engine,
    _this: Ptr<BipedAnim>,
    npc: Ptr,
    slot: u32,
    first: u32,
    second: u32,
) {
    let path = stack_alloc(e, 0x104);
    let normal = stack_alloc(e, 0x104);
    let race = fn_004ac110(e, npc);
    if e.call(SKIN_TEXTURE_PATH, &args![race, npc, slot, path])
        .bool()
    {
        let loader = e.global::<u32>(MODEL_LOADER);
        e.call(QUEUE_TEXTURE, &args![loader, path, first, second]);
        e.call(
            MODIFIED_TEXTURE_FILENAME,
            &args![normal, path, NORMAL_MAP_SUFFIX, 1u32],
        );
        if e.mem.i8(normal.addr()) != 0 {
            e.call(QUEUE_TEXTURE, &args![loader, normal, first, second]);
        }
    }
    stack_free(e, normal);
    stack_free(e, path);
}

// Translated from 004ac1e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BipedAnim::LoadBipedParts` (Xbox PDB): for each of the 20 slots, brings
/// the loaded 3D in line with the slot's form and model. A slot whose form
/// and part equal the buffered slot's (and that has a buffered clone) takes
/// the buffered clone over; the other slots are loaded: the file is loaded
/// with the model loader and cloned, textures swapped, skinned or attached
/// to its bone or the root, named, its add-on nodes added and limbs hidden
/// as dismembered. Slot 5 (the weapon) is loaded through `004ab750`
/// instead. With `flag` zero a FaceGen slot may be handled by `004aede0`.
/// The actor's 3D properties are updated at the end.
pub fn biped_anim_load_biped_parts(e: &mut Engine, this: Ptr<BipedAnim>, flag: u8) {
    let requester = e.get(this, BipedAnim::m_pRequester);
    let player = e.global::<u32>(PLAYER);
    let mut first_person = false;
    let has_slot_6 = !e.get(object(this, 6), BipedObject::pParent).is_null();
    if requester.addr() == player && e.call(IS_FIRST_PERSON_BIPED, &args![player, this]).bool() {
        first_person = true;
    }
    if e.global::<u8>(PALETTE_FLAG) != 0 {
        let form = e.call(REFERENCE_FORM, &args![requester]).u32();
        e.call(CLEAR_BODY_PALETTE, &args![form]);
    }
    for slot in 0..SLOT_COUNT {
        load_parts_step(e, this, flag, slot, first_person, has_slot_6);
    }
    let requester = e.get(this, BipedAnim::m_pRequester);
    let root = e.call(ACTOR_ROOT, &args![requester]).u32();
    e.call(UPDATE_PROPERTIES, &args![root]);
}

/// One pass of `LoadBipedParts`' loop (slot `slot`).
fn load_parts_step(
    e: &mut Engine,
    this: Ptr<BipedAnim>,
    flag: u8,
    slot: u32,
    first_person: bool,
    has_slot_6: bool,
) {
    let current = object(this, slot);
    let previous = buffered(this, slot);
    let part = e.get(current, BipedObject::pPart);
    let reuse = !part.is_null()
        && part == e.get(previous, BipedObject::pPart)
        && e.get(current, BipedObject::pParent) == e.get(previous, BipedObject::pParent)
        && !e.get(previous, BipedObject::pPartClone).is_null();
    if reuse {
        // The buffered slot becomes the slot, the buffered slot is cleared.
        for word in 0..4 {
            let value = e.mem.u32(previous.addr() + 4 * word);
            e.mem.set_u32(current.addr() + 4 * word, value);
        }
        memset(e, previous.addr(), 0, 0x10);
        let clone = e.get(current, BipedObject::pPartClone);
        if e.global::<u8>(PALETTE_FLAG) != 0 && !clone.is_null() {
            // Virtual slot 0xc of the clone: its node.
            let node = e.vcall(clone.addr(), 0x0c, &[]).ptr::<()>();
            if !node.is_null() {
                let mut index = 0;
                while index < child_count(e, node) {
                    let child = child_at(e, node, index);
                    e.call(FN_004AF490, &args![this, child, 0u32]);
                    index += 1;
                }
            } else {
                e.call(FN_004AF490, &args![this, clone, 0u32]);
            }
        }
        let weapon_form = e.get(object(this, WEAPON_SLOT), BipedObject::pParent);
        if slot == 7 && !weapon_form.is_null() && fn_004ab730(e, weapon_form) {
            let clone = e.get(current, BipedObject::pPartClone);
            if !clone.is_null() {
                e.call(SET_WEAPON_FLAG, &args![clone, 1u32]);
            }
        }
        return;
    }
    if !e.get(previous, BipedObject::pPartClone).is_null() {
        fn_004ab020(e, this, previous, 1, Ptr::NULL);
    } else {
        memset(e, previous.addr(), 0, 0x10);
    }
    load_part(e, this, flag, slot, first_person, has_slot_6);
}

/// The second half of one pass of `LoadBipedParts`' loop: loads the part of
/// slot `slot` (after the buffered slot was dealt with).
fn load_part(
    e: &mut Engine,
    this: Ptr<BipedAnim>,
    flag: u8,
    slot: u32,
    first_person: bool,
    has_slot_6: bool,
) {
    let current = object(this, slot);
    let requester = e.get(this, BipedAnim::m_pRequester);
    let player = e.global::<u32>(PLAYER);
    let form = e.get(current, BipedObject::pParent);
    if slot == WEAPON_SLOT && !form.is_null() {
        if requester.addr() == player {
            let process = e.call(SAVED_ACQUIRE_OBJECT, &args![player]).u32();
            // Virtual slot 0x148 of the process: the item change of the
            // equipped weapon.
            let item_change = e.vcall(process, 0x148, &[]).u32();
            let mod_slots = e.call(GET_MOD_SLOTS, &args![item_change]).u8();
            let effect = stack_alloc(e, 4);
            e.mem.set_f32(effect.addr(), 0.0);
            let has_effect = e
                .call(HAS_MOD_EFFECT_ACTIVE, &args![item_change, 0x0eu32, effect])
                .bool();
            stack_free(e, effect);
            fn_004ab750(e, this, form, mod_slots);
            if fn_004ad010(e, form)
                && player_biped(e, 0) == this.addr()
                && (!fn_004ad030(e, form) || has_effect)
            {
                let value = e.call(WEAPON_MODEL_VALUE, &args![form]).u32();
                e.call(SHOW_WEAPON, &args![value]);
            }
        } else {
            e.call(ACTOR_SET_WEAPON, &args![requester, form]);
        }
        return;
    }
    let part = e.get(current, BipedObject::pPart);
    if part.is_null() || part.addr() == 0xffff_ffff {
        return;
    }
    if first_person && !(2..=6).contains(&slot) {
        return;
    }
    if flag == 0
        && is_face_gen_slot(e, this, slot)
        && requester.addr() != player
        && setting_text_set(e, SETTING_FACE)
        && e.call(SETTING_BYTE_VALUE, &[]).bool()
        && fn_004abfa0(e)
    {
        e.call(FN_004AEDE0, &args![this, 0u32, 0u32, slot]);
        return;
    }
    if e.call(MODEL_HAS_PATH, &args![part]).u32() == 0 {
        return;
    }
    let bone_index = e.global::<u32>(SLOT_BONES + 4 * slot);
    let face_slot = is_face_gen_slot(e, this, slot);
    // Virtual slot 0x14 of the model: its path.
    let path = e.vcall(part.addr(), 0x14, &[]).u32();
    let loader = e.global::<u32>(MODEL_LOADER);
    let file = e
        .call(
            LOAD_FILE,
            &args![loader, path, 3u32, 1u32, 0u32, face_slot, 0u32],
        )
        .ptr::<()>();
    let cloning = stack_alloc(e, 0x1c);
    fn_004ad050(e, cloning, 1.0);
    let handle = stack_alloc(e, 4);
    ni_pointer_new(e, handle, Ptr::NULL);
    let copy = if e.call(HAS_MORPHER_CONTROLLER, &args![file]).bool() {
        let tes = e.global::<u32>(TES_GLOBAL);
        let deep = e
            .call(DEEP_COPY_SAME_TEXTURES, &args![tes, file, cloning])
            .ptr();
        ni_pointer_assign(e, handle, deep);
        ni_pointer_get(e, handle)
    } else {
        e.call(NI_OBJECT_CLONE, &args![file, cloning]).ptr()
    };
    // Virtual slot 0xc of the copy: its node.
    let node = e.vcall(copy.addr(), 0x0c, &[]).ptr::<()>();
    let mut result = Ptr::NULL;
    if !node.is_null() {
        let part = e.get(current, BipedObject::pPart);
        let swap = if part.is_null() {
            0
        } else {
            e.vcall(part.addr(), 0x1c, &[]).u32()
        };
        if swap != 0 {
            e.call(SWAP_TEXTURES, &args![swap, node]);
        }
        let parent_form = e.get(current, BipedObject::pParent);
        if !parent_form.is_null() && e.vcall(parent_form.addr(), 0xac, &[]).bool() {
            e.call(SWAP_PLATFORM_TEXTURES, &args![node]);
        }
        // Virtual slot 0x10 of the node: the fade node, if it is one.
        let fade = e.vcall(node.addr(), 0x10, &[]).u32();
        if fade != 0 {
            e.call(SET_LOD_MULT_TYPE, &args![fade, 7u32]);
        }
        e.call(SET_TRANSLATION, &args![node, ZERO_TRANSLATION]);
        e.call(SET_ROTATION, &args![node, IDENTITY_ROTATION]);
        result = biped_anim_apply_skinned_objects(
            e,
            this,
            node,
            slot,
            first_person as u8,
            has_slot_6 as u8,
            Ptr::NULL,
        );
        if result.is_null() {
            result = node;
            if !this.is_null() && is_face_gen_slot(e, this, slot) {
                let face = e.call(FN_004AE790, &args![this, slot]).ptr::<()>();
                if !face.is_null() {
                    let mut index = 0;
                    while index < child_count(e, node) {
                        let child = child_at(e, node, index);
                        // Virtual slot 0x1c of the child: its geometry.
                        let geometry = e.vcall(child.addr(), 0x1c, &[]).ptr::<()>();
                        if !child.is_null() && geometry.is_null() {
                            let part = e.get(current, BipedObject::pPart);
                            let name = e.vcall(part.addr(), 0x14, &[]).u32();
                            e.call(LOG, &args![NON_GEOMETRY_FORMAT, name]);
                        }
                        if !geometry.is_null()
                            && e.call(NODE_SKIN_DATA, &args![geometry]).u32() != 0
                            && !requester.is_null()
                            && e.call(REFERENCE_FORM, &args![requester]).u32() != 0
                        {
                            let base = e.call(REFERENCE_FORM, &args![requester]);
                            if e.call(FORM_TYPE, &args![base.u32()]).u32() == FORM_TYPE_NPC {
                                apply_face_gen(e, requester, face, geometry);
                            }
                        }
                        index += 1;
                    }
                }
            }
            let root = e.get(this, BipedAnim::root);
            e.call(FN_004AE250, &args![root, node, 0u32, this, slot, 0u32]);
            let save_load = e.global::<u32>(SAVE_LOAD);
            if !e.call(SAVE_LOAD_FLAG, &args![save_load]).bool() && slot == WEAPON_SLOT {
                let requester = e.get(this, BipedAnim::m_pRequester);
                let process = e.call(SAVED_ACQUIRE_OBJECT, &args![requester]).u32();
                if process != 0 {
                    fire_weapon_state(e, this, requester);
                }
            }
        }
    }
    if !result.is_null() {
        finish_part(e, this, slot, node, file, result, bone_index);
    }
    ni_pointer_release(e, handle);
    stack_free(e, handle);
    ni_cloning_process_destructor(e, cloning.cast());
    stack_free(e, cloning);
}

/// The FaceGen block of `LoadBipedParts`: a copy of the geometry's skin data
/// (`005495f0`) is made and set on the geometry (virtual slot 0xe4), the
/// NPC's face coordinates are fetched and applied to the FaceGen model
/// `face`; when they are (or the setting `0043faf0` is off) the geometry's
/// rotation is turned by a quarter turn about Y.
fn apply_face_gen(e: &mut Engine, requester: Ptr, face: Ptr, geometry: Ptr) {
    let npc = e.call(REFERENCE_FORM, &args![requester]).ptr::<()>();
    let coords = stack_alloc(e, 0x80);
    e.call(
        VECTOR_CONSTRUCT,
        &args![coords, 0x20u32, 4u32, FACE_COORD_INIT, FACE_COORD_FREE],
    );
    let first = stack_alloc(e, 4);
    ni_pointer_new(e, first, Ptr::NULL);
    let second = stack_alloc(e, 4);
    ni_pointer_new(e, second, Ptr::NULL);
    let skin = e.call(NODE_SKIN_DATA, &args![geometry]).ptr::<()>();
    e.call(NI_OBJECT_DEEP_COPY, &args![skin, first]);
    let copy = ni_pointer_get(e, first);
    ni_pointer_assign(e, second, copy);
    let copy = ni_pointer_get(e, second);
    e.vcall(geometry.addr(), 0xe4, &args![copy]);
    e.call(GET_FACE_COORD, &args![npc, coords]);
    let setting_on = e.call(SETTING_BYTE_VALUE, &[]).bool();
    if !setting_on
        || e.call(APPLY_FACE_COORDS, &args![face, coords, geometry, 0u32])
            .bool()
    {
        let turn = stack_alloc(e, 0x24);
        let product = stack_alloc(e, 0x24);
        e.call(MATRIX_INIT, &args![turn]);
        let angle = e.global::<f32>(QUARTER_TURN_BACK);
        e.call(MAKE_Y_ROTATION, &args![turn, angle]);
        let rotation = e.call(NODE_ROTATION, &args![geometry]).u32();
        let result = e
            .call(MATRIX_MULTIPLY, &args![rotation, product, turn])
            .u32();
        e.call(SET_ROTATION, &args![geometry, result]);
        stack_free(e, product);
        stack_free(e, turn);
    }
    ni_pointer_release(e, second);
    stack_free(e, second);
    ni_pointer_release(e, first);
    stack_free(e, first);
    e.call(
        VECTOR_DESTRUCT,
        &args![coords, 0x20u32, 4u32, FACE_COORD_FREE],
    );
    stack_free(e, coords);
}

/// The last part of loading slot `slot`: names the node `result`
/// (`"<slot name> <form name> (<form id>)"`), adds the add-on nodes of the
/// model `file`, attaches an unskinned node to its bone (or the root, or
/// logs when the slot has no bone), hides dismembered limbs, flags the
/// weapon and stores `result` as the slot's clone.
fn finish_part(
    e: &mut Engine,
    this: Ptr<BipedAnim>,
    slot: u32,
    node: Ptr,
    file: Ptr,
    result: Ptr,
    bone_index: u32,
) {
    let current = object(this, slot);
    let requester = e.get(this, BipedAnim::m_pRequester);
    let mut result = result;
    let form = e.get(current, BipedObject::pParent);
    name_part_node(e, result, form, slot);
    e.call(FN_004AF240, &args![file, node, slot, requester]);
    if !e.get(current, BipedObject::bSkinned) && e.vcall(result.addr(), 0x0c, &[]).u32() != 0 {
        let world = e.call(NODE_PARENT, &args![result]).u32();
        if bone_index == 0xffff_ffff {
            let actor_name = e.call(ACTOR_LOG_NAME, &args![requester]).u32();
            let part = e.get(current, BipedObject::pPart);
            let path = e.vcall(part.addr(), 0x14, &[]).u32();
            e.call(LOG, &args![SHOULD_BE_SKINNED_FORMAT, path, actor_name]);
            if world != 0 {
                e.vcall(world, 0xe8, &args![result]);
            } else {
                let handle = stack_alloc(e, 4);
                ni_pointer_new(e, handle, result);
                ni_pointer_release(e, handle);
                stack_free(e, handle);
            }
            result = Ptr::NULL;
        } else if !fn_004ab230(e, this, bone_index).is_null() {
            let target = fn_004ab230(e, this, bone_index);
            e.vcall(target.addr(), 0xdc, &args![result, 1u32]);
        } else if e.call(NODE_PARENT, &args![result]).u32() != 0 {
            let root = e.get(this, BipedAnim::root);
            e.vcall(root.addr(), 0xdc, &args![result, 1u32]);
        }
    }
    let list = e.call(ACTOR_EXTRA_LIST, &args![requester]).u32();
    let extra = e.call(GET_DISMEMBERMENT_EXTRA, &args![list]).u32();
    if extra != 0 {
        for limb in 0..15u32 {
            if e.call(LIMB_DISMEMBERED, &args![extra, limb]).bool() {
                e.call(HIDE_LIMB, &args![limb, result]);
            }
        }
    }
    let weapon_form = e.get(object(this, WEAPON_SLOT), BipedObject::pParent);
    if slot == 7 && !weapon_form.is_null() && fn_004ab730(e, weapon_form) && !result.is_null() {
        e.call(SET_WEAPON_FLAG, &args![result, 1u32]);
    }
    e.set(current, BipedObject::pPartClone, result);
}

// Translated from 004ad010 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether bit 2 of the byte at `+0x100` of `form` is set.
pub fn fn_004ad010(e: &mut Engine, form: Ptr) -> bool {
    e.mem.u8(form.addr() + 0x100) & 4 != 0
}

// Translated from 004ad030 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether bit 13 of the word at `+0x12c` of `form` is set.
pub fn fn_004ad030(e: &mut Engine, form: Ptr) -> bool {
    e.mem.u32(form.addr() + 0x12c) & 0x2000 != 0
}

// Translated from 004ad050 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiCloningProcess::NiCloningProcess(float)`: builds the process with hash
/// size `0x101`, initializes the vector at `+0x10` and sets its three floats
/// to `scale`. Returns `this`.
pub fn fn_004ad050(e: &mut Engine, this: Ptr, scale: f32) -> Ptr {
    fn_004ad0c0(e, this.cast(), 0x101);
    e.call(MATRIX_INIT, &args![this.byte_add(0x10)]);
    fn_004ad240(e, this.cast(), scale);
    this
}

// Translated from 004ad0c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiCloningProcess::NiCloningProcess(hash size)`: allocates the clone map
/// and the process map (16 bytes each, built by `004afb20` and `004afb50`
/// with `hash_size`) and stores the copy type (the word `004ad1b0` gives)
/// and the append character (the byte `004ad1c0` gives). Returns `this`.
pub fn fn_004ad0c0(e: &mut Engine, this: Ptr<NiCloningProcess>, hash_size: u32) -> Ptr {
    let block = e.call(OPERATOR_NEW, &args![0x10u32]).u32();
    let clone_map = if block != 0 {
        e.call(FN_004AFB20, &args![block, hash_size]).ptr()
    } else {
        Ptr::NULL
    };
    e.set(this, NiCloningProcess::m_pkCloneMap, clone_map);
    let block = e.call(OPERATOR_NEW, &args![0x10u32]).u32();
    let process_map = if block != 0 {
        e.call(FN_004AFB50, &args![block, hash_size]).ptr()
    } else {
        Ptr::NULL
    };
    e.set(this, NiCloningProcess::m_pkProcessMap, process_map);
    let copy_type = fn_004ad1b0(e);
    e.set(this, NiCloningProcess::m_eCopyType, copy_type);
    let append = fn_004ad1c0(e);
    e.set(this, NiCloningProcess::m_cAppendChar, append);
    this.cast()
}

// Translated from 004ad1b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The word at `011f4300`.
pub fn fn_004ad1b0(e: &mut Engine) -> u32 {
    e.global::<u32>(CLONE_COPY_TYPE)
}

// Translated from 004ad1c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The byte at `011a94a8`.
pub fn fn_004ad1c0(e: &mut Engine) -> u8 {
    e.global::<u8>(CLONE_APPEND_CHAR)
}

// Translated from 004ad1d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Destroys the two maps of an `NiCloningProcess` (virtual slot 0 with the
/// delete flag 1, when not null).
pub fn fn_004ad1d0(e: &mut Engine, this: Ptr<NiCloningProcess>) {
    let clone_map = e.get(this, NiCloningProcess::m_pkCloneMap);
    if !clone_map.is_null() {
        e.vcall(clone_map.addr(), 0, &args![1u32]);
    }
    let process_map = e.get(this, NiCloningProcess::m_pkProcessMap);
    if !process_map.is_null() {
        e.vcall(process_map.addr(), 0, &args![1u32]);
    }
}

// Translated from 004ad240 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the three scale floats of an `NiCloningProcess` to `scale`.
pub fn fn_004ad240(e: &mut Engine, this: Ptr<NiCloningProcess>, scale: f32) {
    e.set(this, NiCloningProcess::m_fScale_z, scale);
    e.set(this, NiCloningProcess::m_fScale_y, scale);
    e.set(this, NiCloningProcess::m_fScale_x, scale);
}

// Translated from 004ad270 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiCloningProcess::~NiCloningProcess` (Xbox PDB): destroys the two maps
/// ([`fn_004ad1d0`]).
pub fn ni_cloning_process_destructor(e: &mut Engine, this: Ptr<NiCloningProcess>) {
    fn_004ad1d0(e, this);
}

// Translated from 004ad290 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BipedAnim::ApplySkinnedObjects` (Xbox PDB): for the loaded node `node`
/// of slot `slot`, records whether it holds skinned nodes (`FindSkinnedNode`)
/// in the slot's skinned flag and, when it does and the biped has a root,
/// attaches them under the root's `SkinAttachment` node (or the root's
/// parent) with [`biped_anim_attach_skinned_object`]: once for `node`, or for
/// each child when `00653270` says the object is of the skinned class.
/// Returns the attached node; 0 when the node is not skinned, `node` itself
/// when the biped has no root (or `node` is null).
pub fn biped_anim_apply_skinned_objects(
    e: &mut Engine,
    this: Ptr<BipedAnim>,
    node: Ptr,
    slot: u32,
    first_flag: u8,
    second_flag: u8,
    face_model: Ptr,
) -> Ptr {
    let skinned_class = e
        .call(CAST_TO_CLASS, &args![SKINNED_CLASS, node])
        .ptr::<()>();
    let handle = stack_alloc(e, 4);
    ni_pointer_new(e, handle, Ptr::NULL);
    let result;
    if node.is_null() {
        result = node;
    } else {
        let skinned = e.call(FIND_SKINNED_NODE, &args![node]).u8();
        e.mem.set_u8(object(this, slot).addr() + 0xc, skinned);
        if e.get(this, BipedAnim::root).is_null() {
            result = node;
        } else if e.get(object(this, slot), BipedObject::bSkinned) {
            let attachment = stack_alloc(e, 4);
            ni_pointer_new(e, attachment, Ptr::NULL);
            let root = e.get(this, BipedAnim::root);
            let found = fn_004aae30(e, root, Ptr::new(SKIN_ATTACHMENT_NAME));
            if !found.is_null() {
                ni_pointer_assign(e, attachment, found);
            } else {
                let parent = e.call(NODE_PARENT, &args![root]).ptr();
                ni_pointer_assign(e, attachment, parent);
            }
            let mut attached = Ptr::NULL;
            if skinned_class.is_null() {
                let parent_object = ni_pointer_get(e, attachment);
                attached = biped_anim_attach_skinned_object(
                    e,
                    this,
                    node,
                    parent_object,
                    slot,
                    first_flag,
                    second_flag,
                    face_model,
                );
            } else {
                let mut index = 0;
                while index < child_count(e, skinned_class) {
                    let child = child_at(e, skinned_class, index);
                    attached = biped_anim_attach_skinned_object(
                        e,
                        this,
                        child,
                        Ptr::NULL,
                        slot,
                        first_flag,
                        second_flag,
                        face_model,
                    );
                    index += 1;
                }
            }
            result = attached;
            if attached != node {
                // A reference is taken on `node` and given back at once.
                let extra = stack_alloc(e, 4);
                ni_pointer_new(e, extra, node);
                ni_pointer_release(e, extra);
                stack_free(e, extra);
            }
            ni_pointer_release(e, attachment);
            stack_free(e, attachment);
        } else {
            result = Ptr::NULL;
        }
    }
    ni_pointer_release(e, handle);
    stack_free(e, handle);
    result
}

// Translated from 004ad4a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BipedAnim::AttachSkinnedObject` (Xbox PDB): attaches the skinned parts
/// of `node`. A node `parent_object` is given: when `node`'s second child
/// has a geometry (virtual slot 0x18), a new `NiNode` is made, added to
/// `parent_object` (virtual slot 0xdc) and becomes the node the parts go
/// to. The weapon bone keeps the node's extra data (`011c61e4`) and the
/// list of its geometries. Children without a geometry (and, per the flags,
/// those named like the pip-boy or the third slot) are collected and
/// removed from `node`; each child with data has its FaceGen data prepared,
/// its skin copied, its body part lights bound to the skeleton's bones by
/// name and is added to the node. Returns the node made, else the first
/// child with data, else 0.
#[allow(clippy::too_many_arguments)]
pub fn biped_anim_attach_skinned_object(
    e: &mut Engine,
    this: Ptr<BipedAnim>,
    node: Ptr,
    parent_object: Ptr,
    slot: u32,
    flag_a: u8,
    flag_b: u8,
    face_model: Ptr,
) -> Ptr {
    let mut result = Ptr::NULL;
    let mut attach_to = parent_object;
    let handle = stack_alloc(e, 4);
    ni_pointer_new(e, handle, Ptr::NULL);
    e.call(FN_004ADD70, &args![node]);
    let mut face = face_model;
    if face.is_null() && is_face_gen_slot(e, this, slot) {
        face = e.call(FN_004AE790, &args![this, slot]).ptr();
    }
    e.call(SET_TRANSLATION, &args![node, ZERO_TRANSLATION]);
    if !parent_object.is_null() && child_count(e, node) >= 2 {
        let second = child_at(e, node, 1);
        if !second.is_null() {
            let second = child_at(e, node, 1);
            // Virtual slot 0x18: the geometry of a node.
            if e.vcall(second.addr(), 0x18, &[]).u32() != 0 {
                let block = e.call(NI_OPERATOR_NEW, &args![0xacu32]).u32();
                attach_to = if block != 0 {
                    e.call(NI_NODE_INIT, &args![block, 0u32]).ptr()
                } else {
                    Ptr::NULL
                };
                e.vcall(parent_object.addr(), 0xdc, &args![attach_to, 1u32]);
                result = attach_to;
            }
        }
    }
    let key = fn_004ab220(e);
    let extra = e.call(GET_EXTRA_DATA, &args![node, key]).ptr::<()>();
    if !extra.is_null() {
        let weapon_bone = fn_004ab230(e, this, 1);
        if !weapon_bone.is_null() {
            let key = fn_004ab220(e);
            e.call(REMOVE_EXTRA_DATA, &args![weapon_bone, key]);
        }
        let key = fn_004ab220(e);
        e.call(ADD_EXTRA_DATA, &args![weapon_bone, key, extra]);
        let mut index = 0;
        while index < child_count(e, node) {
            let child = child_at(e, node, index);
            if e.vcall(child.addr(), 0x18, &[]).u32() != 0 {
                let geometry = e.vcall(child.addr(), 0x18, &[]).u32();
                if e.call(NODE_DATA_POINTER, &args![geometry]).u32() != 0 {
                    let item = stack_alloc(e, 4);
                    let child = child_at(e, node, index);
                    e.mem.set_u32(item.addr(), child.addr());
                    e.call(ADD_TO_LIST, &args![extra.byte_add(0x34), item]);
                    stack_free(e, item);
                }
            }
            index += 1;
        }
    }
    let collected = stack_alloc(e, 0x10);
    e.call(FN_004AFF00, &args![collected, 0u32, 1u32]);
    let mut index = 0;
    while index < child_count(e, node) {
        let child = child_at(e, node, index);
        if !child.is_null() && e.vcall(child.addr(), 0x18, &[]).u32() == 0 {
            array_add(e, collected, child);
            e.vcall(node.addr(), 0xe8, &args![child]);
            index += 1;
            continue;
        }
        if flag_a != 0 {
            let name = node_name(e, child);
            if !name.is_null() {
                let prefix = e.global::<u32>(SLOT_NAMES + 8);
                let length = e.call(TEXT_LENGTH, &args![prefix]).u32();
                let name = node_name(e, child);
                if e.call(COMPARE_PREFIX, &args![name, prefix, length]).i32() == 0 {
                    array_add(e, collected, child);
                    e.vcall(node.addr(), 0xe8, &args![child]);
                    index += 1;
                    continue;
                }
            }
        }
        let name = node_name(e, child);
        if !name.is_null() {
            let name = node_name(e, child);
            let text = stack_alloc(e, 8);
            e.call(STRING_FROM_TEXT, &args![text, name]);
            e.call(FN_004AFAD0, &args![text]);
            let prefix = stack_alloc(e, 12);
            let chars = ni_pointer_get(e, text);
            let (count, wanted) = if flag_b != 0 {
                (9u32, PIPBOY_OFF_PREFIX)
            } else {
                (8u32, PIPBOY_ON_PREFIX)
            };
            e.call(FN_004ADD50, &args![prefix, chars, count]);
            e.mem.set_u8(prefix.addr() + count, 0);
            let matched = e.call(COMPARE_TEXT, &args![prefix, wanted]).i32() == 0;
            stack_free(e, prefix);
            if matched {
                array_add(e, collected, child);
                e.vcall(node.addr(), 0xe8, &args![child]);
                e.call(STRING_FREE, &args![text]);
                stack_free(e, text);
                index += 1;
                continue;
            }
            e.call(STRING_FREE, &args![text]);
            stack_free(e, text);
        }
        if !child.is_null() {
            let requester = e.get(this, BipedAnim::m_pRequester);
            if e.call(NODE_SKIN_DATA, &args![child]).u32() != 0
                && e.call(NODE_DATA_POINTER, &args![child]).u32() != 0
                && !face.is_null()
                && !requester.is_null()
                && e.call(REFERENCE_FORM, &args![requester]).u32() != 0
            {
                let base = e.call(REFERENCE_FORM, &args![requester]).u32();
                if e.call(FORM_TYPE, &args![base]).u32() == FORM_TYPE_NPC {
                    attach_face_gen(e, requester, face, child, handle);
                }
            }
        }
        e.call(FN_004AF490, &args![this, child, 0u32]);
        let data = e.call(NODE_DATA_POINTER, &args![child]).ptr::<()>();
        if !data.is_null() {
            if result.is_null() {
                result = child;
            }
            let owner = e.call(BODY_PART_OWNER, &args![data]).u32();
            let count = e.call(BODY_PART_COUNT, &args![owner]).u32();
            let items = e.call(BODY_PART_ARRAY, &args![data]).u32();
            let mut light = 0;
            while light < count {
                let item = e.mem.u32(items + 4 * light);
                if node_name(e, Ptr::new(item)).is_null() {
                    let parent_name = if e.call(NODE_PARENT, &args![node]).u32() != 0 {
                        let parent = e.call(NODE_PARENT, &args![node]).ptr();
                        node_name(e, parent)
                    } else {
                        Ptr::new(NULL_TEXT)
                    };
                    let own_name = node_name(e, node);
                    e.call(
                        LOG,
                        &args![UNNAMED_BONE_FORMAT, light, parent_name, own_name],
                    );
                    light += 1;
                    continue;
                }
                let root = e.get(this, BipedAnim::root);
                let field = e.call(NAME_FIELD, &args![item]).u32();
                let bone_node = e.call(FN_004ADE00, &args![root, field]).u32();
                if bone_node != 0 {
                    e.call(FN_004ADDA0, &args![data, light, bone_node]);
                }
                light += 1;
            }
            if !attach_to.is_null() {
                e.call(SET_PARENT_OBJECT, &args![data, parent_object]);
                e.vcall(attach_to.addr(), 0xdc, &args![child, 1u32]);
            }
        }
        index += 1;
    }
    e.call(FN_004ADD70, &args![node]);
    e.call(FN_004ADE20, &args![collected]);
    stack_free(e, collected);
    ni_pointer_release(e, handle);
    stack_free(e, handle);
    result
}

/// The FaceGen block of `AttachSkinnedObject` (see [`apply_face_gen`] for the
/// part it shares): the geometry's skin copy is set, then its property data
/// is cloned and attached, the NPC's face coordinates are fetched and, when
/// the setting `0043faf0` is on, applied to `face`. `handle` is the
/// function's `NiPointer` the deep copies are written to.
fn attach_face_gen(e: &mut Engine, requester: Ptr, face: Ptr, child: Ptr, handle: Ptr) {
    let npc = e.call(REFERENCE_FORM, &args![requester]).ptr::<()>();
    let coords = stack_alloc(e, 0x80);
    e.call(
        VECTOR_CONSTRUCT,
        &args![coords, 0x20u32, 4u32, FACE_COORD_INIT, FACE_COORD_FREE],
    );
    let copy_handle = stack_alloc(e, 4);
    ni_pointer_new(e, copy_handle, Ptr::NULL);
    let skin = e.call(NODE_SKIN_DATA, &args![child]).ptr::<()>();
    e.call(NI_OBJECT_DEEP_COPY, &args![skin, handle]);
    let copy = ni_pointer_get(e, handle);
    ni_pointer_assign(e, copy_handle, copy);
    let copy = ni_pointer_get(e, copy_handle);
    e.vcall(child.addr(), 0xe4, &args![copy]);
    if e.call(NODE_DATA_POINTER, &args![child]).u32() != 0 {
        let data = e.call(NODE_DATA_POINTER, &args![child]).u32();
        if e.call(NODE_DATA_FIELD, &args![data]).u32() != 0 {
            let data = e.call(NODE_DATA_POINTER, &args![child]).u32();
            let field = e.call(NODE_DATA_FIELD, &args![data]).u32();
            e.call(NI_OBJECT_DEEP_COPY, &args![field, handle]);
            let copied = ni_pointer_get(e, handle);
            let property_handle = stack_alloc(e, 4);
            ni_pointer_new(e, property_handle, copied);
            let data = e.call(NODE_DATA_POINTER, &args![child]).u32();
            let cloned = e.call(NI_OBJECT_CLONE_OV2, &args![data]).u32();
            e.call(FN_004ADDE0, &args![child, cloned]);
            let kept = ni_pointer_get(e, property_handle);
            let data = e.call(NODE_DATA_POINTER, &args![child]).u32();
            e.call(FN_004ADDC0, &args![data, kept]);
            ni_pointer_release(e, property_handle);
            stack_free(e, property_handle);
        }
    }
    e.call(GET_FACE_COORD, &args![npc, coords]);
    if e.call(SETTING_BYTE_VALUE, &[]).bool() {
        e.call(APPLY_FACE_COORDS, &args![face, coords, child, 0u32]);
    }
    ni_pointer_release(e, copy_handle);
    stack_free(e, copy_handle);
    e.call(
        VECTOR_DESTRUCT,
        &args![coords, 0x20u32, 4u32, FACE_COORD_FREE],
    );
    stack_free(e, coords);
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(
            0x004aaca0,
            biped_anim_biped_anim(Ptr<BipedAnim>, Ptr, Ptr) -> Ptr<BipedAnim>
        ),
        entry!(0x004aad00, biped_anim_init_root(Ptr<BipedAnim>, Ptr)),
        entry!(0x004aae30, fn_004aae30(Ptr, Ptr) -> Ptr),
        entry!(0x004aae50, fn_004aae50(Ptr<BipedAnim>)),
        entry!(0x004aae70, biped_anim_remove_all_parts(Ptr<BipedAnim>)),
        entry!(0x004aaef0, fn_004aaef0(Ptr)),
        entry!(0x004aaf10, fn_004aaf10(Ptr)),
        entry!(
            0x004aaf30,
            fn_004aaf30(Ptr<BipedAnim>, Ptr) -> Ptr<BipedObject>
        ),
        entry!(0x004aaf90, fn_004aaf90(Ptr<BipedAnim>, Ptr)),
        entry!(0x004aaff0, fn_004aaff0(Ptr<BipedAnim>, u32, u8, Ptr)),
        entry!(
            0x004ab020,
            fn_004ab020(Ptr<BipedAnim>, Ptr<BipedObject>, u8, Ptr)
        ),
        entry!(0x004ab0c0, biped_anim_run_biped_3d_detach(Ptr)),
        entry!(
            0x004ab130,
            biped_anim_recursively_remove_all_decal_placement_vector_data_from_weapon_bone(
                Ptr<BipedAnim>,
                Ptr,
            )
        ),
        entry!(0x004ab220, fn_004ab220() -> Ptr),
        entry!(0x004ab230, fn_004ab230(Ptr<BipedAnim>, u32) -> Ptr),
        entry!(0x004ab250, fn_004ab250(Ptr<BipedAnim>, Ptr, u32)),
        entry!(0x004ab400, fn_004ab400(Ptr<BipedAnim>, Ptr, u8)),
        entry!(0x004ab500, fn_004ab500(Ptr, u8) -> u32),
        entry!(0x004ab5b0, biped_anim_remove_biped_weapon(Ptr<BipedAnim>)),
        entry!(0x004ab730, fn_004ab730(Ptr) -> bool),
        entry!(0x004ab750, fn_004ab750(Ptr<BipedAnim>, Ptr, u8)),
        entry!(0x004abad0, fn_004abad0(Ptr<BipedAnim>, Ptr, Ptr, u32)),
        entry!(0x004abd30, fn_004abd30(Ptr<BipedAnim>, u32, Ptr, u32)),
        entry!(0x004abf80, fn_004abf80(Ptr) -> u32),
        entry!(0x004abfa0, fn_004abfa0() -> bool),
        entry!(0x004abfe0, fn_004abfe0(Ptr<BipedAnim>, u32, u32, u32)),
        entry!(0x004ac110, fn_004ac110(Ptr) -> u32),
        entry!(
            0x004ac130,
            biped_anim_queue_skin_texture(Ptr<BipedAnim>, Ptr, u32, u32, u32)
        ),
        entry!(0x004ac1e0, biped_anim_load_biped_parts(Ptr<BipedAnim>, u8)),
        entry!(0x004ad010, fn_004ad010(Ptr) -> bool),
        entry!(0x004ad030, fn_004ad030(Ptr) -> bool),
        entry!(0x004ad050, fn_004ad050(Ptr, f32) -> Ptr),
        entry!(0x004ad0c0, fn_004ad0c0(Ptr<NiCloningProcess>, u32) -> Ptr),
        entry!(0x004ad1b0, fn_004ad1b0() -> u32),
        entry!(0x004ad1c0, fn_004ad1c0() -> u8),
        entry!(0x004ad1d0, fn_004ad1d0(Ptr<NiCloningProcess>)),
        entry!(0x004ad240, fn_004ad240(Ptr<NiCloningProcess>, f32)),
        entry!(
            0x004ad270,
            ni_cloning_process_destructor(Ptr<NiCloningProcess>)
        ),
        entry!(
            0x004ad290,
            biped_anim_apply_skinned_objects(Ptr<BipedAnim>, Ptr, u32, u8, u8, Ptr) -> Ptr
        ),
        entry!(
            0x004ad4a0,
            biped_anim_attach_skinned_object(Ptr<BipedAnim>, Ptr, Ptr, u32, u8, u8, Ptr) -> Ptr
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Every function outside this unit (and every later function of it) that
    /// the translations call; `engine` stubs them all so a test only runs the
    /// code it is about.
    const CALLEES: &[u32] = &[
        MEMSET,
        OPERATOR_NEW,
        NI_OPERATOR_NEW,
        RT_DYNAMIC_CAST,
        VECTOR_CONSTRUCT,
        VECTOR_DESTRUCT,
        COMPARE_PREFIX,
        COMPARE_TEXT,
        TEXT_LENGTH,
        LOG,
        SPRINTF,
        STRING_INIT,
        STRING_FREE,
        STRING_FROM_TEXT,
        FIXED_STRING_INIT,
        FIXED_STRING_FREE,
        SET_NAME,
        NI_POINTER_INIT,
        NI_POINTER_SET,
        NI_POINTER_RELEASE,
        READ_WORD,
        CHILD_COUNT,
        CHILD_AT,
        NAME_FIELD,
        NAME_TEXT,
        NODE_DATA_POINTER,
        NODE_DATA_FIELD,
        NODE_SKIN_DATA,
        GET_EXTRA_DATA,
        ADD_EXTRA_DATA,
        REMOVE_EXTRA_DATA,
        NI_OBJECT_CLONE,
        NI_OBJECT_CLONE_OV2,
        NI_OBJECT_DEEP_COPY,
        DEEP_COPY_SAME_TEXTURES,
        HAS_MORPHER_CONTROLLER,
        UPDATE_PROPERTIES,
        NI_NODE_INIT,
        SET_LOD_MULT_TYPE,
        FORM_TYPE,
        FORM_ID,
        REFERENCE_FORM,
        GET_FORM_AS_BIPED_MODEL,
        FILLS_BIPED_SLOT,
        BIPED_MODEL_PATH,
        SWAP_TEXTURES,
        SWAP_PLATFORM_TEXTURES,
        FORM_MODEL,
        FORM_ANIMS,
        MODEL_HAS_PATH,
        LOAD_FILE,
        QUEUE_TEXTURE,
        QUEUE_MODEL_FORM,
        QUEUE_MODEL,
        RELEASE_MODEL,
        MODIFIED_TEXTURE_FILENAME,
        BODY_MOD_TEXTURE_NAME,
        BODY_MOD_TEXTURE_FILE_NAME,
        SKIN_TEXTURE_PATH,
        FORM_SLOT_ITEM,
        READ_WORD_PLUS_4,
        GET_BIPED,
        IS_FIRST_PERSON_BIPED,
        PLAYER_NODE,
        PLAYER_ANIMATION,
        SAVED_ACQUIRE_OBJECT,
        ACTOR_ANIMATION,
        IS_WEAPON_DRAWN,
        RELOAD_TARGETS,
        BLEND_OUT,
        FIND_SKINNED_NODE,
        DATA_HANDLER_FLAG,
        SAVE_LOAD_FLAG,
        FLAGS_OBJECT_FLAG,
        SETTING_BYTE_VALUE,
        SETTING_BYTE_POINTER,
        SET_WEAPON_FLAG,
        GET_MOD_SLOTS,
        HAS_MOD_EFFECT_ACTIVE,
        WEAPON_MODEL_VALUE,
        SHOW_WEAPON,
        ACTOR_SET_WEAPON,
        ACTOR_EXTRA_LIST,
        GET_DISMEMBERMENT_EXTRA,
        LIMB_DISMEMBERED,
        HIDE_LIMB,
        ACTOR_LOG_NAME,
        ACTOR_ROOT,
        SET_TRANSLATION,
        SET_ROTATION,
        MAKE_Y_ROTATION,
        MATRIX_MULTIPLY,
        NODE_ROTATION,
        MATRIX_INIT,
        GET_FACE_COORD,
        APPLY_FACE_COORDS,
        CLEAR_BODY_PALETTE,
        FACE_COORD_INIT,
        FACE_COORD_FREE,
        CAST_TO_CLASS,
        SHADOW_SCENE_NODE,
        SHADOW_REMOVE_OBJECT,
        NODE_PARENT,
        WORLD_REMOVE_OBJECTS,
        PREPARE_DETACH,
        MUST_DEFER,
        TASK_QUEUE_GETTER,
        TASK_QUEUE_DETACH,
        LOCK_ENTER,
        LOCK_LEAVE,
        FIND_IN_LIST,
        ADD_TO_LIST,
        QUEUE_OBJECT_INIT,
        QUEUE_OBJECT_FREE,
        QUEUE_OBJECT_USE,
        SET_PARENT_OBJECT,
        BODY_PART_OWNER,
        BODY_PART_COUNT,
        BODY_PART_ARRAY,
        FIND_OBJECT_BY_NAME,
        FN_004ADDA0,
        FN_004ADDC0,
        FN_004ADDE0,
        FN_004ADE00,
        FN_004ADE20,
        FN_004ADD50,
        FN_004ADD70,
        FN_004AE250,
        FN_004AE790,
        FN_004AE8A0,
        FN_004AEDE0,
        FN_004AEE60,
        FN_004AEED0,
        FN_004AF240,
        FN_004AF490,
        FN_004AF950,
        FN_004AFA20,
        FN_004AFA50,
        FN_004AFAD0,
        FN_004AFB20,
        FN_004AFB50,
        FN_004AFB80,
        FN_004AFBA0,
        FN_004AFC50,
        FN_004AFF00,
    ];

    fn ret(value: u32) -> Ret {
        Ret {
            eax: value,
            ..Ret::default()
        }
    }

    /// An engine whose callees are stubs returning 0 (`memset` really fills
    /// and the `NiPointer` functions keep the pointer in their four bytes),
    /// with the exe pages the translations read mapped.
    fn engine() -> Engine {
        let mut e = Engine::new();
        for (start, length) in [
            (0x0101_6000u32, 0x1000u32),
            (0x0118_8000, 0x1000),
            (0x011a_9000, 0x1000),
            (0x011c_3000, 0x4000),
            (0x011d_5000, 0x1000),
            (0x011d_d000, 0x3000),
            (0x011f_4000, 0x6000),
        ] {
            e.map(start, length);
        }
        for &address in CALLEES {
            e.register(address, |_, _| Ret::default());
        }
        e.register(MEMSET, |e, a| {
            e.mem.write(a[0], &vec![a[1] as u8; a[2] as usize]);
            Ret::default()
        });
        e.register(NI_POINTER_INIT, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            ret(a[0])
        });
        e.register(NI_POINTER_SET, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            ret(a[0])
        });
        e.register(READ_WORD, |e, a| ret(e.mem.u32(a[0])));
        // The player is not any object a test builds.
        e.set_global(PLAYER, 0x0f00_0000);
        e.call_log = Some(vec![]);
        e
    }

    /// Makes `address` return `value`.
    fn returns(e: &mut Engine, address: u32, value: u32) {
        e.register_double(address, move |_, _| ret(value));
    }

    /// The calls made since the last call of `log` (the log starts again).
    fn log(e: &mut Engine) -> Vec<(u32, Vec<u32>)> {
        e.call_log.replace(vec![]).unwrap()
    }

    /// The argument words of the calls to `address` in `calls`.
    fn calls_to(calls: &[(u32, Vec<u32>)], address: u32) -> Vec<Vec<u32>> {
        calls
            .iter()
            .filter(|(a, _)| *a == address)
            .map(|(_, words)| words.clone())
            .collect()
    }

    /// The addresses of `calls`, in order.
    fn addresses(calls: &[(u32, Vec<u32>)]) -> Vec<u32> {
        calls.iter().map(|(a, _)| *a).collect()
    }

    fn biped(e: &mut Engine) -> Ptr<BipedAnim> {
        e.new_object::<BipedAnim>()
    }

    /// A zeroed block of game memory.
    fn block(e: &mut Engine, size: u32) -> u32 {
        e.mem.alloc(size)
    }

    /// A NUL-terminated text in game memory.
    fn text(e: &mut Engine, s: &str) -> u32 {
        let address = e.mem.alloc(s.len() as u32 + 1);
        e.mem.set_cstr(address, s.as_bytes());
        address
    }

    /// An object whose vtable gives, for each `(offset, value)`, a virtual
    /// function (a double at `0x7000_0000 + vtable + offset`) returning
    /// `value`.
    fn object_with_slots(e: &mut Engine, slots: &[(u32, u32)]) -> u32 {
        let vtable = e.mem.alloc(0x400);
        for &(offset, value) in slots {
            let target = 0x7000_0000 + vtable + offset;
            e.register_double(target, move |_, _| ret(value));
            e.mem.set_u32(vtable + offset, target);
        }
        let object = e.mem.alloc(0x300);
        e.mem.set_u32(object, vtable);
        object
    }

    /// The address a virtual function of `object` at `offset` has.
    fn slot_target(e: &Engine, object: u32, offset: u32) -> u32 {
        e.mem.u32(e.mem.u32(object) + offset)
    }

    /// Sets slot `slot` of `this`.
    fn set_slot(e: &mut Engine, this: Ptr<BipedAnim>, slot: u32, form: u32, part: u32, clone: u32) {
        e.set(object(this, slot), BipedObject::pParent, Ptr::new(form));
        e.set(object(this, slot), BipedObject::pPart, Ptr::new(part));
        e.set(object(this, slot), BipedObject::pPartClone, Ptr::new(clone));
    }

    fn slot_words(e: &Engine, address: Ptr<BipedObject>) -> [u32; 4] {
        let a = address.addr();
        [
            e.mem.u32(a),
            e.mem.u32(a + 4),
            e.mem.u32(a + 8),
            e.mem.u32(a + 12),
        ]
    }

    /// Writes the bone names, the skin slots and the slot-to-bone table.
    fn tables(e: &mut Engine) {
        for i in 0..5 {
            e.mem.set_u32(BONE_NAMES + 4 * i, 0x0118_8c00 + 0x10 * i);
        }
        for (i, slot) in [2u32, 3, 4].into_iter().enumerate() {
            e.mem.set_u32(SKIN_SLOTS + 4 * i as u32, slot);
        }
        for slot in 0..20 {
            e.mem
                .set_u32(SLOT_NAMES + 4 * slot, 0x0118_8d00 + 0x10 * slot);
            e.mem.set_u32(SLOT_BONES + 4 * slot, 0xffff_ffff);
        }
    }

    #[test]
    fn the_unit_registers_forty_functions() {
        let table = funcs();
        assert_eq!(table.len(), 40);
        let mut addresses: Vec<u32> = table.iter().map(|(a, _)| *a).collect();
        addresses.sort_unstable();
        addresses.dedup();
        assert_eq!(addresses.len(), 40);
        let mut e = engine();
        // Through the uniform form: the weapon flag word table and a flag bit.
        let form = block(&mut e, 0x300);
        e.mem.set_u32(form + 0x258, 77);
        assert_eq!(e.call(0x004a_b500, &args![form, 2u8]).u32(), 77);
        e.mem.set_u8(form + 0x100, 0x10);
        assert!(e.call(0x004a_b730, &args![form]).bool());
    }

    #[test]
    fn the_constructor_clears_the_object_and_stores_the_requester() {
        let mut e = engine();
        let this = biped(&mut e);
        e.mem.set_u32(this.addr() + 0x100, 0xdead_beef);
        e.mem.set_u32(this.addr() + 0x2a0, 0xdead_beef);
        e.mem.set_u32(this.addr() + 0x170, 0xdead_beef);
        let result = biped_anim_biped_anim(&mut e, this, Ptr::new(0x2222_0000), Ptr::NULL);
        assert_eq!(result, this);
        assert_eq!(e.get(this, BipedAnim::m_pRequester).addr(), 0x2222_0000);
        assert_eq!(e.mem.u32(this.addr() + 0x100), 0);
        assert_eq!(e.mem.u32(this.addr() + 0x170), 0);
        let calls = log(&mut e);
        assert_eq!(
            calls,
            vec![
                (MEMSET, vec![this.addr(), 0, 0x2b4]),
                (MEMSET, vec![this.addr() + 0x16c, 0, 0x140]),
            ]
        );
    }

    #[test]
    fn the_constructor_finds_the_bones_when_given_a_root() {
        let mut e = engine();
        tables(&mut e);
        let this = biped(&mut e);
        biped_anim_biped_anim(&mut e, this, Ptr::new(0x2222_0000), Ptr::new(0x3333_0000));
        let calls = log(&mut e);
        // The root node name and the five bones are looked up under the root.
        let finds = calls_to(&calls, FIND_OBJECT_BY_NAME);
        assert_eq!(finds.len(), 6);
        assert_eq!(finds[0], vec![0x3333_0000, ROOT_NODE_NAME, 1]);
        assert_eq!(finds[1], vec![0x3333_0000, 0x0118_8c00, 1]);
    }

    #[test]
    fn init_root_flags_and_stores_the_bones_that_are_nodes_and_logs_the_others() {
        let mut e = engine();
        tables(&mut e);
        let this = biped(&mut e);
        let root_node = 0x4444_0000;
        let node = object_with_slots(&mut e, &[(0x0c, 1)]);
        let not_a_node = object_with_slots(&mut e, &[(0x0c, 0)]);
        let table: Vec<(u32, u32)> = vec![
            (ROOT_NODE_NAME, root_node),
            (0x0118_8c00, node),
            (0x0118_8c10, not_a_node),
            (0x0118_8c20, 0),
            (0x0118_8c30, node),
            (0x0118_8c40, 0),
        ];
        e.register_double(FIND_OBJECT_BY_NAME, move |_, a| {
            ret(table.iter().find(|(name, _)| *name == a[1]).unwrap().1)
        });
        returns(&mut e, NAME_FIELD, 0x1234);
        returns(&mut e, NAME_TEXT, 0x5678);
        biped_anim_init_root(&mut e, this, Ptr::new(0x3333_0000));
        assert_eq!(e.get(this, BipedAnim::root).addr(), root_node);
        let flags: Vec<u8> = (0..5)
            .map(|i| e.get(bone(this, i), BipedBone::cFlags))
            .collect();
        assert_eq!(flags, vec![1, 0, 0, 1, 0]);
        assert_eq!(e.get(bone(this, 0), BipedBone::pParent).addr(), node);
        assert_eq!(e.get(bone(this, 3), BipedBone::pParent).addr(), node);
        assert!(e.get(bone(this, 1), BipedBone::pParent).is_null());
        let calls = log(&mut e);
        // The three bones that are missing or not nodes are logged with the
        // name of the root's owner.
        assert_eq!(
            calls_to(&calls, LOG),
            vec![
                vec![MISSING_BONE_FORMAT, 0x0118_8c10, 0x5678],
                vec![MISSING_BONE_FORMAT, 0x0118_8c20, 0x5678],
                vec![MISSING_BONE_FORMAT, 0x0118_8c40, 0x5678],
            ]
        );
        assert_eq!(calls_to(&calls, NAME_TEXT)[0], vec![0x1234]);
        // Each NiPointer temporary is released.
        assert_eq!(calls_to(&calls, NI_POINTER_RELEASE).len(), 5);
    }

    #[test]
    fn find_object_by_name_asks_for_the_recursive_search() {
        let mut e = engine();
        returns(&mut e, FIND_OBJECT_BY_NAME, 0x77);
        let found = fn_004aae30(&mut e, Ptr::new(0x10), Ptr::new(0x20));
        assert_eq!(found.addr(), 0x77);
        assert_eq!(
            calls_to(&log(&mut e), FIND_OBJECT_BY_NAME),
            vec![vec![0x10, 0x20, 1]]
        );
    }

    #[test]
    fn remove_all_parts_clears_every_slot_between_the_locks() {
        let mut e = engine();
        let this = biped(&mut e);
        let loader = block(&mut e, 0x10);
        e.mem.set_u32(loader, 0x4444);
        e.set_global(MODEL_LOADER, loader);
        set_slot(&mut e, this, 3, 0x1111, 0x2222, 0);
        e.set(buffered(this, 4), BipedObject::pParent, Ptr::new(0x3333));
        e.set(buffered(this, 4), BipedObject::pPart, Ptr::new(0x3334));
        biped_anim_remove_all_parts(&mut e, this);
        for slot in 0..20 {
            assert_eq!(slot_words(&e, object(this, slot)), [0; 4]);
            assert_eq!(slot_words(&e, buffered(this, slot)), [0; 4]);
        }
        let calls = log(&mut e);
        let order = addresses(&calls);
        assert_eq!(order[0], FN_004AFB80);
        assert_eq!(order[1], LOCK_ENTER);
        assert_eq!(order[order.len() - 2], LOCK_LEAVE);
        assert_eq!(order[order.len() - 1], FN_004AFBA0);
        assert_eq!(calls[0].1, vec![0x4444]);
        assert_eq!(calls[1].1, vec![PART_LOCK, 0]);
        assert_eq!(calls[calls.len() - 1].1, vec![0x4444]);
    }

    #[test]
    fn the_wrapper_removes_all_parts() {
        let mut e = engine();
        let this = biped(&mut e);
        let loader = block(&mut e, 0x10);
        e.set_global(MODEL_LOADER, loader);
        set_slot(&mut e, this, 2, 0x1111, 0x2222, 0);
        fn_004aae50(&mut e, this);
        assert_eq!(slot_words(&e, object(this, 2)), [0; 4]);
    }

    #[test]
    fn the_loader_lock_helpers_call_the_object_in_the_first_field() {
        let mut e = engine();
        let loader = block(&mut e, 0x10);
        e.mem.set_u32(loader, 0x4444);
        fn_004aaef0(&mut e, Ptr::new(loader));
        fn_004aaf10(&mut e, Ptr::new(loader));
        let calls = log(&mut e);
        assert_eq!(
            calls,
            vec![(FN_004AFB80, vec![0x4444]), (FN_004AFBA0, vec![0x4444])]
        );
    }

    #[test]
    fn the_slot_of_a_form_is_found_or_null() {
        let mut e = engine();
        let this = biped(&mut e);
        set_slot(&mut e, this, 9, 0xaaaa, 0, 0);
        set_slot(&mut e, this, 4, 0xbbbb, 0, 0);
        set_slot(&mut e, this, 12, 0xaaaa, 0, 0);
        assert_eq!(fn_004aaf30(&mut e, this, Ptr::new(0xaaaa)), object(this, 9));
        assert_eq!(fn_004aaf30(&mut e, this, Ptr::new(0xbbbb)), object(this, 4));
        assert!(fn_004aaf30(&mut e, this, Ptr::new(0xcccc)).is_null());
        // A null form finds nothing even though empty slots exist.
        assert!(fn_004aaf30(&mut e, this, Ptr::NULL).is_null());
    }

    #[test]
    fn clearing_the_slots_of_a_form_leaves_the_others() {
        let mut e = engine();
        let this = biped(&mut e);
        set_slot(&mut e, this, 9, 0xaaaa, 0x10, 0);
        set_slot(&mut e, this, 4, 0xbbbb, 0x20, 0);
        set_slot(&mut e, this, 12, 0xaaaa, 0x30, 0);
        fn_004aaf90(&mut e, this, Ptr::new(0xaaaa));
        assert_eq!(slot_words(&e, object(this, 9)), [0; 4]);
        assert_eq!(slot_words(&e, object(this, 12)), [0; 4]);
        assert_eq!(slot_words(&e, object(this, 4)), [0xbbbb, 0x20, 0, 0]);
        // A null form clears nothing.
        log(&mut e);
        fn_004aaf90(&mut e, this, Ptr::NULL);
        assert!(log(&mut e).is_empty());
        assert_eq!(slot_words(&e, object(this, 4)), [0xbbbb, 0x20, 0, 0]);
    }

    #[test]
    fn clearing_a_slot_by_number_replaces_the_part_when_asked() {
        let mut e = engine();
        let this = biped(&mut e);
        set_slot(&mut e, this, 4, 0xbbbb, 0x20, 0);
        fn_004aaff0(&mut e, this, 4, 0, Ptr::new(0x99));
        assert_eq!(slot_words(&e, object(this, 4)), [0xbbbb, 0x20, 0, 0]);
        fn_004aaff0(&mut e, this, 4, 1, Ptr::new(0x99));
        assert_eq!(slot_words(&e, object(this, 4)), [0, 0x99, 0, 0]);
    }

    /// A slot with a loaded clone, whose model has a path.
    fn slot_with_clone(e: &mut Engine, this: Ptr<BipedAnim>, slot: u32, clone: u32) {
        let model = object_with_slots(e, &[(0x14, 0xabc0)]);
        set_slot(e, this, slot, 0x1111, model, clone);
    }

    #[test]
    fn emptying_a_slot_with_a_clone_detaches_it_and_releases_the_model() {
        let mut e = engine();
        let this = biped(&mut e);
        e.set_global(MODEL_LOADER, 0x5555);
        slot_with_clone(&mut e, this, 6, 0x6666);
        fn_004ab020(&mut e, this, object(this, 6), 1, Ptr::NULL);
        let calls = log(&mut e);
        // Not deferred: the clone is detached at once.
        assert_eq!(calls_to(&calls, MUST_DEFER), vec![Vec::<u32>::new()]);
        assert_eq!(calls_to(&calls, PREPARE_DETACH), vec![vec![0x6666]]);
        assert!(calls_to(&calls, TASK_QUEUE_DETACH).is_empty());
        assert_eq!(calls_to(&calls, RELEASE_MODEL), vec![vec![0x5555, 0xabc0]]);
        assert_eq!(slot_words(&e, object(this, 6)), [0; 4]);
    }

    #[test]
    fn emptying_a_slot_queues_the_detach_when_it_must_be_deferred() {
        let mut e = engine();
        let this = biped(&mut e);
        e.set_global(MODEL_LOADER, 0x5555);
        returns(&mut e, MUST_DEFER, 1);
        returns(&mut e, TASK_QUEUE_GETTER, 0x7700);
        slot_with_clone(&mut e, this, 6, 0x6666);
        fn_004ab020(&mut e, this, object(this, 6), 0, Ptr::NULL);
        let calls = log(&mut e);
        assert!(calls_to(&calls, PREPARE_DETACH).is_empty());
        assert_eq!(
            calls_to(&calls, TASK_QUEUE_DETACH),
            vec![vec![0x7700, 0x6666]]
        );
        // The clone is gone, the form stays (clear == 0).
        assert!(e.get(object(this, 6), BipedObject::pPartClone).is_null());
        assert_eq!(e.get(object(this, 6), BipedObject::pParent).addr(), 0x1111);
    }

    #[test]
    fn emptying_a_slot_without_a_clone_only_clears_the_form() {
        let mut e = engine();
        let this = biped(&mut e);
        set_slot(&mut e, this, 6, 0x1111, 0x2222, 0);
        fn_004ab020(&mut e, this, object(this, 6), 1, Ptr::new(0x77));
        assert!(log(&mut e).is_empty());
        assert_eq!(slot_words(&e, object(this, 6)), [0, 0x77, 0, 0]);
    }

    #[test]
    fn detaching_a_node_removes_it_from_the_shadow_scene_and_its_parent() {
        let mut e = engine();
        biped_anim_run_biped_3d_detach(&mut e, Ptr::NULL);
        assert!(log(&mut e).is_empty());
        // No parent: only the preparation and the shadow scene node.
        returns(&mut e, SHADOW_SCENE_NODE, 0x8800);
        biped_anim_run_biped_3d_detach(&mut e, Ptr::new(0x6666));
        let calls = log(&mut e);
        assert_eq!(
            addresses(&calls),
            vec![
                PREPARE_DETACH,
                SHADOW_SCENE_NODE,
                SHADOW_REMOVE_OBJECT,
                NODE_PARENT
            ]
        );
        assert_eq!(calls[2].1, vec![0x8800, 0x6666]);
        // With a parent: the physics objects and the parent's slot 0xe8.
        let parent = object_with_slots(&mut e, &[(0xe8, 0)]);
        returns(&mut e, NODE_PARENT, parent);
        biped_anim_run_biped_3d_detach(&mut e, Ptr::new(0x6666));
        let calls = log(&mut e);
        assert_eq!(
            calls_to(&calls, WORLD_REMOVE_OBJECTS),
            vec![vec![0x6666, 1, 0]]
        );
        let target = slot_target(&e, parent, 0xe8);
        assert_eq!(calls_to(&calls, target), vec![vec![parent, 0x6666]]);
    }

    #[test]
    fn decal_data_removal_does_nothing_without_the_extra_data() {
        let mut e = engine();
        let this = biped(&mut e);
        e.set(bone(this, 1), BipedBone::pParent, Ptr::new(0x7000));
        e.set_global(EXTRA_DATA_KEY, 0x9900);
        biped_anim_recursively_remove_all_decal_placement_vector_data_from_weapon_bone(
            &mut e,
            this,
            Ptr::NULL,
        );
        assert!(log(&mut e).is_empty());
        biped_anim_recursively_remove_all_decal_placement_vector_data_from_weapon_bone(
            &mut e,
            this,
            Ptr::new(0x6666),
        );
        assert_eq!(log(&mut e), vec![(GET_EXTRA_DATA, vec![0x7000, 0x9900])]);
    }

    #[test]
    fn decal_data_of_a_geometry_found_in_the_list_is_removed_from_the_bone() {
        let mut e = engine();
        let this = biped(&mut e);
        e.set(bone(this, 1), BipedBone::pParent, Ptr::new(0x7000));
        e.set_global(EXTRA_DATA_KEY, 0x9900);
        returns(&mut e, GET_EXTRA_DATA, 0x8000);
        returns(&mut e, NODE_DATA_POINTER, 0x1);
        let geometry = object_with_slots(&mut e, &[(0x1c, 1)]);
        // The key the list is searched with is the node, in a stack word.
        let searched = Rc::new(RefCell::new(0));
        let seen = searched.clone();
        e.register_double(FIND_IN_LIST, move |e, a| {
            *seen.borrow_mut() = e.mem.u32(a[1]);
            ret(1)
        });
        biped_anim_recursively_remove_all_decal_placement_vector_data_from_weapon_bone(
            &mut e,
            this,
            Ptr::new(geometry),
        );
        assert_eq!(*searched.borrow(), geometry);
        let calls = log(&mut e);
        assert_eq!(calls_to(&calls, FIND_IN_LIST)[0][0], 0x8034);
        assert_eq!(calls_to(&calls, FIND_IN_LIST)[0][2], 0);
        assert_eq!(
            calls_to(&calls, REMOVE_EXTRA_DATA),
            vec![vec![0x7000, 0x9900]]
        );
    }

    #[test]
    fn decal_data_removal_recurses_through_the_children_of_a_node() {
        let mut e = engine();
        let this = biped(&mut e);
        e.set(bone(this, 1), BipedBone::pParent, Ptr::new(0x7000));
        e.set_global(EXTRA_DATA_KEY, 0x9900);
        returns(&mut e, GET_EXTRA_DATA, 0x8000);
        let leaf = object_with_slots(&mut e, &[(0x1c, 1)]);
        let list = 0x6600_0000;
        let node = object_with_slots(&mut e, &[(0x1c, 0), (0x0c, list)]);
        returns(&mut e, CHILD_COUNT, 2);
        e.register_double(CHILD_AT, move |_, _| ret(leaf));
        biped_anim_recursively_remove_all_decal_placement_vector_data_from_weapon_bone(
            &mut e,
            this,
            Ptr::new(node),
        );
        let calls = log(&mut e);
        // The node and its two children each look the extra data up; the
        // leaves have no data so nothing is removed.
        assert_eq!(calls_to(&calls, GET_EXTRA_DATA).len(), 3);
        assert!(calls_to(&calls, REMOVE_EXTRA_DATA).is_empty());
        assert_eq!(
            calls_to(&calls, CHILD_AT),
            vec![vec![list, 0], vec![list, 1]]
        );
    }

    #[test]
    fn the_key_and_bone_getters_read_the_global_and_the_bone() {
        let mut e = engine();
        let this = biped(&mut e);
        e.set_global(EXTRA_DATA_KEY, 0x9900);
        assert_eq!(fn_004ab220(&mut e).addr(), 0x9900);
        e.set(bone(this, 3), BipedBone::pParent, Ptr::new(0x7003));
        assert_eq!(fn_004ab230(&mut e, this, 3).addr(), 0x7003);
        assert!(fn_004ab230(&mut e, this, 2).is_null());
    }

    #[test]
    fn resetting_the_slots_for_an_actor_moves_them_to_the_buffer() {
        let mut e = engine();
        tables(&mut e);
        let this = biped(&mut e);
        let requester = object_with_slots(&mut e, &[(0x1d0, 1)]);
        e.set(this, BipedAnim::m_pRequester, Ptr::new(requester));
        set_slot(&mut e, this, 3, 0x1111, 0x2222, 0x3333);
        returns(&mut e, FORM_SLOT_ITEM, 0);
        log(&mut e);
        fn_004ab250(&mut e, this, Ptr::new(0xf0f0), 0x55);
        // The buffer was empty, so it took the slots over; the slots are clear.
        assert_eq!(
            slot_words(&e, buffered(this, 3)),
            [0x1111, 0x2222, 0x3333, 0]
        );
        assert_eq!(slot_words(&e, object(this, 3)), [0; 4]);
        // The three skin slots (no bone: any) ask the form for their item.
        let calls = log(&mut e);
        assert_eq!(
            calls_to(&calls, FORM_SLOT_ITEM),
            vec![
                vec![0xf0f0, 0x55, 2],
                vec![0xf0f0, 0x55, 3],
                vec![0xf0f0, 0x55, 4]
            ]
        );
    }

    #[test]
    fn resetting_the_slots_keeps_a_used_buffer() {
        let mut e = engine();
        tables(&mut e);
        let this = biped(&mut e);
        let requester = object_with_slots(&mut e, &[(0x1d0, 1)]);
        e.set(this, BipedAnim::m_pRequester, Ptr::new(requester));
        set_slot(&mut e, this, 3, 0x1111, 0x2222, 0x3333);
        e.set(buffered(this, 9), BipedObject::pParent, Ptr::new(0x4444));
        fn_004ab250(&mut e, this, Ptr::new(0xf0f0), 0);
        assert_eq!(slot_words(&e, buffered(this, 3)), [0; 4]);
        assert_eq!(slot_words(&e, buffered(this, 9))[0], 0x4444);
        assert_eq!(slot_words(&e, object(this, 3)), [0; 4]);
    }

    #[test]
    fn resetting_the_slots_without_an_actor_clears_both_arrays() {
        let mut e = engine();
        tables(&mut e);
        let this = biped(&mut e);
        set_slot(&mut e, this, 3, 0x1111, 0x2222, 0x3333);
        // The requester's virtual slot 0x1d0 says no.
        let requester = object_with_slots(&mut e, &[(0x1d0, 0)]);
        e.set(this, BipedAnim::m_pRequester, Ptr::new(requester));
        fn_004ab250(&mut e, this, Ptr::new(0xf0f0), 0);
        assert_eq!(slot_words(&e, object(this, 3)), [0; 4]);
        assert_eq!(slot_words(&e, buffered(this, 3)), [0; 4]);
    }

    #[test]
    fn resetting_the_slots_skips_a_skin_slot_whose_bone_was_not_found() {
        let mut e = engine();
        tables(&mut e);
        // Slot 3 attaches to bone 1, which was not found; slot 4 to bone 2.
        e.mem.set_u32(SLOT_BONES + 4 * 3, 1);
        e.mem.set_u32(SLOT_BONES + 4 * 4, 2);
        let this = biped(&mut e);
        e.set(bone(this, 2), BipedBone::cFlags, 1);
        returns(&mut e, FORM_SLOT_ITEM, 0);
        log(&mut e);
        fn_004ab250(&mut e, this, Ptr::new(0xf0f0), 0);
        let slots: Vec<u32> = calls_to(&log(&mut e), FORM_SLOT_ITEM)
            .into_iter()
            .map(|a| a[2])
            .collect();
        assert_eq!(slots, vec![2, 4]);
        // Without a form nothing happens at all.
        fn_004ab250(&mut e, this, Ptr::NULL, 0);
        assert!(log(&mut e).is_empty());
    }

    /// A weapon form: a block with room for the +0x250.. words and the flags.
    fn weapon_form(e: &mut Engine) -> u32 {
        let form = block(e, 0x300);
        returns(e, FORM_TYPE, 0x28);
        form
    }

    #[test]
    fn choosing_the_weapon_stores_the_form_and_its_model() {
        let mut e = engine();
        let this = biped(&mut e);
        let form = weapon_form(&mut e);
        returns(&mut e, FORM_MODEL, 0xaaaa);
        set_slot(&mut e, this, 5, 0x7777, 0x8888, 0);
        log(&mut e);
        fn_004ab400(&mut e, this, Ptr::new(form), 3);
        assert_eq!(
            slot_words(&e, object(this, 5)),
            [form, 0xaaaa, 0, 0],
            "the old weapon was cleared first"
        );
        let calls = log(&mut e);
        assert_eq!(calls_to(&calls, FORM_MODEL), vec![vec![form, 3, 1]]);
    }

    #[test]
    fn choosing_the_weapon_ignores_other_forms_and_a_loaded_weapon() {
        let mut e = engine();
        let this = biped(&mut e);
        let form = block(&mut e, 0x300);
        returns(&mut e, FORM_TYPE, 0x2a);
        fn_004ab400(&mut e, this, Ptr::new(form), 0);
        fn_004ab400(&mut e, this, Ptr::NULL, 0);
        assert_eq!(slot_words(&e, object(this, 5)), [0; 4]);
        // The same weapon with a clone is not chosen again.
        returns(&mut e, FORM_TYPE, 0x28);
        set_slot(&mut e, this, 5, form, 0x8888, 0x9999);
        log(&mut e);
        fn_004ab400(&mut e, this, Ptr::new(form), 0);
        assert!(calls_to(&log(&mut e), FORM_MODEL).is_empty());
        assert_eq!(slot_words(&e, object(this, 5)), [form, 0x8888, 0x9999, 0]);
    }

    #[test]
    fn the_players_biped_gets_the_weapons_animation_model() {
        let mut e = engine();
        let this = biped(&mut e);
        let form = weapon_form(&mut e);
        returns(&mut e, FORM_MODEL, 0xaaaa);
        returns(&mut e, FORM_ANIMS, 0xb000);
        e.set_global(PLAYER, 0x1000);
        // GetBiped(1) is this biped.
        returns(&mut e, GET_BIPED, this.addr());
        fn_004ab400(&mut e, this, Ptr::new(form), 0);
        assert_eq!(e.get(object(this, 5), BipedObject::pPart).addr(), 0xb030);
        // With flags, the word the flags select is used, except for the
        // special form id.
        e.mem.set_u32(form + 0x254, 0xc000);
        returns(&mut e, FORM_ID, 0x1234);
        fn_004ab400(&mut e, this, Ptr::new(form), 1);
        assert_eq!(e.get(object(this, 5), BipedObject::pPart).addr(), 0xc030);
        returns(&mut e, FORM_ID, 0x0017_35d4);
        fn_004ab400(&mut e, this, Ptr::new(form), 1);
        assert_eq!(e.get(object(this, 5), BipedObject::pPart).addr(), 0xb030);
        // No animation model: the form's own model stays.
        returns(&mut e, FORM_ANIMS, 0);
        fn_004ab400(&mut e, this, Ptr::new(form), 0);
        assert_eq!(e.get(object(this, 5), BipedObject::pPart).addr(), 0xaaaa);
    }

    #[test]
    fn the_third_person_biped_of_the_player_also_gets_it() {
        let mut e = engine();
        let this = biped(&mut e);
        let form = weapon_form(&mut e);
        returns(&mut e, FORM_MODEL, 0xaaaa);
        returns(&mut e, FORM_ANIMS, 0xb000);
        e.set_global(PLAYER, 0x1000);
        let third = this.addr();
        e.register_double(GET_BIPED, move |_, a| {
            ret(if a[1] == 0 { third } else { 0 })
        });
        fn_004ab400(&mut e, this, Ptr::new(form), 0);
        assert_eq!(e.get(object(this, 5), BipedObject::pPart).addr(), 0xb030);
    }

    #[test]
    fn the_weapon_word_is_chosen_by_the_low_three_flag_bits() {
        let mut e = engine();
        let form = block(&mut e, 0x300);
        for (i, offset) in (0x250..=0x26c).step_by(4).enumerate() {
            e.mem.set_u32(form + offset, 100 + i as u32);
        }
        // flags 0..7: bit 0 adds 4, bit 1 adds 8, bit 2 adds 0xc, except that
        // bits 0 and 1 together give +0x10.
        let expected_offsets = [0x250, 0x254, 0x258, 0x260, 0x25c, 0x268, 0x264, 0x26c];
        for (flags, offset) in expected_offsets.into_iter().enumerate() {
            assert_eq!(
                fn_004ab500(&mut e, Ptr::new(form), flags as u8),
                e.mem.u32(form + offset),
                "flags {flags}"
            );
        }
        // Higher bits are ignored.
        assert_eq!(
            fn_004ab500(&mut e, Ptr::new(form), 0x18),
            e.mem.u32(form + 0x250)
        );
    }

    #[test]
    fn the_weapon_flag_is_bit_four_of_the_byte_at_0x100() {
        let mut e = engine();
        let form = block(&mut e, 0x300);
        assert!(!fn_004ab730(&mut e, Ptr::new(form)));
        e.mem.set_u8(form + 0x100, 0xef);
        assert!(!fn_004ab730(&mut e, Ptr::new(form)));
        e.mem.set_u8(form + 0x100, 0x10);
        assert!(fn_004ab730(&mut e, Ptr::new(form)));
    }

    /// An actor-like requester for the weapon removal: virtual slot 0x100 gives
    /// `is_actor`.
    fn requester_with(e: &mut Engine, this: Ptr<BipedAnim>, is_actor: u32) -> u32 {
        let requester = object_with_slots(e, &[(0x100, is_actor)]);
        e.set(this, BipedAnim::m_pRequester, Ptr::new(requester));
        requester
    }

    #[test]
    fn removing_the_weapon_clears_its_slots_and_reloads_the_targets() {
        let mut e = engine();
        let this = biped(&mut e);
        let form = block(&mut e, 0x300);
        e.mem.set_u8(form + 0x100, 0x10);
        set_slot(&mut e, this, 5, form, 0x2222, 0);
        // Slot 7's clone carries the weapon flag.
        set_slot(&mut e, this, 7, 0x1212, 0x3434, 0x5656);
        e.set(buffered(this, 5), BipedObject::pParent, Ptr::new(0x6161));
        e.set(buffered(this, 2), BipedObject::pParent, Ptr::new(0x6161));
        e.set(buffered(this, 3), BipedObject::pParent, Ptr::new(0x6262));
        let requester = requester_with(&mut e, this, 1);
        returns(&mut e, SAVED_ACQUIRE_OBJECT, 0x4141);
        returns(&mut e, GET_BIPED, 0x1);
        let backpack = 0x7171;
        returns(&mut e, FIND_OBJECT_BY_NAME, backpack);
        let parent = object_with_slots(&mut e, &[(0xe8, 0)]);
        returns(&mut e, NODE_PARENT, parent);
        // The model of slot 7 is released through its (stubbed) loader.
        e.set(this, BipedAnim::root, Ptr::new(0x1818));
        log(&mut e);
        biped_anim_remove_biped_weapon(&mut e, this);
        assert_eq!(slot_words(&e, object(this, 5)), [0; 4]);
        assert_eq!(slot_words(&e, buffered(this, 5)), [0; 4]);
        assert_eq!(slot_words(&e, buffered(this, 2)), [0; 4]);
        assert_eq!(slot_words(&e, buffered(this, 3))[0], 0x6262);
        let calls = log(&mut e);
        assert_eq!(calls_to(&calls, SET_WEAPON_FLAG), vec![vec![0x5656, 0]]);
        assert_eq!(calls_to(&calls, RELOAD_TARGETS), vec![vec![requester, 0]]);
        assert_eq!(
            calls_to(&calls, FIND_OBJECT_BY_NAME),
            vec![vec![0x1818, BACKPACK_NAME, 1]]
        );
        let target = slot_target(&e, parent, 0xe8);
        assert_eq!(calls_to(&calls, target), vec![vec![parent, backpack]]);
    }

    #[test]
    fn removing_the_weapon_keeps_the_targets_for_the_first_person_biped_and_non_actors() {
        let mut e = engine();
        let this = biped(&mut e);
        requester_with(&mut e, this, 1);
        returns(&mut e, SAVED_ACQUIRE_OBJECT, 0x4141);
        // GetBiped(1) is this biped: no reload.
        returns(&mut e, GET_BIPED, this.addr());
        log(&mut e);
        biped_anim_remove_biped_weapon(&mut e, this);
        let calls = log(&mut e);
        assert!(calls_to(&calls, RELOAD_TARGETS).is_empty());
        // A data handler flag blocks it too.
        returns(&mut e, GET_BIPED, 0x1);
        returns(&mut e, DATA_HANDLER_FLAG, 1);
        biped_anim_remove_biped_weapon(&mut e, this);
        assert!(calls_to(&log(&mut e), RELOAD_TARGETS).is_empty());
        // Not an actor: neither the reload nor the backpack lookup.
        returns(&mut e, DATA_HANDLER_FLAG, 0);
        requester_with(&mut e, this, 0);
        biped_anim_remove_biped_weapon(&mut e, this);
        let calls = log(&mut e);
        assert!(calls_to(&calls, RELOAD_TARGETS).is_empty());
        assert!(calls_to(&calls, FIND_OBJECT_BY_NAME).is_empty());
    }

    #[test]
    fn loading_a_weapon_attaches_names_and_flags_it() {
        let mut e = engine();
        tables(&mut e);
        let this = biped(&mut e);
        // The weapon form has a name (virtual slot 0x130) and the weapon flag.
        let named = object_with_slots(&mut e, &[(0x130, 0x7e7e)]);
        e.mem.set_u8(named + 0x100, 0x10);
        let requester = requester_with(&mut e, this, 0);
        returns(&mut e, FORM_MODEL, 0xaaaa);
        returns(&mut e, FORM_ID, 0x4321);
        returns(&mut e, FN_004AEED0, 0x9090);
        returns(&mut e, FORM_TYPE, 0x28);
        // Slot 7 has a clone that gets the weapon flag.
        set_slot(&mut e, this, 7, 0x1212, 0x3434, 0x5656);
        returns(&mut e, FIXED_STRING_INIT, 0x1f1f);
        log(&mut e);
        fn_004ab750(&mut e, this, Ptr::new(named), 0);
        assert_eq!(slot_words(&e, object(this, 5)), [named, 0xaaaa, 0x9090, 0]);
        let calls = log(&mut e);
        // Not the player: no first person node.
        assert_eq!(
            calls_to(&calls, FN_004AEED0),
            vec![vec![named, 0xaaaa, 5, requester, 0]]
        );
        let slot_name = e.mem.u32(SLOT_NAMES + 4 * 5);
        let sprintf = calls_to(&calls, SPRINTF);
        assert_eq!(sprintf.len(), 1);
        assert_eq!(
            sprintf[0][1..],
            [SLOT_NAME_FORMAT, slot_name, 0x7e7e, 0x4321]
        );
        assert_eq!(calls_to(&calls, SET_NAME), vec![vec![0x9090, 0x1f1f]]);
        assert_eq!(calls_to(&calls, SET_WEAPON_FLAG), vec![vec![0x5656, 1]]);
    }

    #[test]
    fn a_weapon_that_fails_to_load_is_not_named() {
        let mut e = engine();
        tables(&mut e);
        let this = biped(&mut e);
        let form = weapon_form(&mut e);
        fn_004ab750(&mut e, this, Ptr::new(form), 0);
        let calls = log(&mut e);
        assert!(calls_to(&calls, SPRINTF).is_empty());
        assert!(calls_to(&calls, STRING_INIT).is_empty());
        // Other forms and a loaded weapon are ignored.
        returns(&mut e, FORM_TYPE, 0x2a);
        fn_004ab750(&mut e, this, Ptr::new(form), 0);
        fn_004ab750(&mut e, this, Ptr::NULL, 0);
        assert!(calls_to(&log(&mut e), FN_004AEED0).is_empty());
        returns(&mut e, FORM_TYPE, 0x28);
        set_slot(&mut e, this, 5, form, 1, 2);
        fn_004ab750(&mut e, this, Ptr::new(form), 0);
        assert!(calls_to(&log(&mut e), FN_004AEED0).is_empty());
    }

    #[test]
    fn the_player_loads_the_weapon_with_the_first_person_node() {
        let mut e = engine();
        tables(&mut e);
        let this = biped(&mut e);
        let form = weapon_form(&mut e);
        let requester = 0x1000;
        e.set_global(PLAYER, requester);
        e.set(this, BipedAnim::m_pRequester, Ptr::new(requester));
        returns(&mut e, FORM_MODEL, 0xaaaa);
        returns(&mut e, IS_FIRST_PERSON_BIPED, 1);
        returns(&mut e, PLAYER_NODE, 0x2468);
        returns(&mut e, FN_004AEED0, 0);
        log(&mut e);
        fn_004ab750(&mut e, this, Ptr::new(form), 0);
        let calls = log(&mut e);
        assert_eq!(
            calls_to(&calls, IS_FIRST_PERSON_BIPED),
            vec![vec![requester, this.addr()]]
        );
        assert_eq!(calls_to(&calls, PLAYER_NODE), vec![vec![requester, 1]]);
        assert_eq!(
            calls_to(&calls, FN_004AEED0),
            vec![vec![form, 0xaaaa, 5, requester, 0x2468]]
        );
    }

    #[test]
    fn an_actors_weapon_is_announced_to_its_process() {
        let mut e = engine();
        tables(&mut e);
        let this = biped(&mut e);
        let form = object_with_slots(&mut e, &[(0x130, 0x7e7e)]);
        returns(&mut e, FORM_TYPE, 0x28);
        let process = object_with_slots(&mut e, &[(0x1cc, 0)]);
        let requester = requester_with(&mut e, this, 1);
        returns(&mut e, SAVED_ACQUIRE_OBJECT, process);
        returns(&mut e, FN_004AEED0, 0x9090);
        returns(&mut e, GET_BIPED, 0x1);
        returns(&mut e, ACTOR_ANIMATION, 0x3131);
        returns(&mut e, IS_WEAPON_DRAWN, 1);
        log(&mut e);
        fn_004ab750(&mut e, this, Ptr::new(form), 0);
        let calls = log(&mut e);
        assert_eq!(calls_to(&calls, RELOAD_TARGETS), vec![vec![requester, 1]]);
        assert_eq!(calls_to(&calls, BLEND_OUT), vec![vec![0x3131, 4, 0]]);
        let target = slot_target(&e, process, 0x1cc);
        assert_eq!(
            calls_to(&calls, target),
            vec![vec![process, 1, this.addr(), 0x3131, requester]]
        );
        // The flags object blocks the announcement.
        returns(&mut e, FLAGS_OBJECT_FLAG, 1);
        set_slot(&mut e, this, 5, 0, 0, 0);
        fn_004ab750(&mut e, this, Ptr::new(form), 0);
        let calls = log(&mut e);
        assert!(calls_to(&calls, target).is_empty());
        assert!(calls_to(&calls, BLEND_OUT).is_empty());
    }

    /// Makes the dynamic cast answer `object` for the given objects only
    /// (`races`), and null otherwise.
    fn races_are(e: &mut Engine, races: Vec<u32>) {
        e.register_double(RT_DYNAMIC_CAST, move |_, a| {
            ret(if races.contains(&a[0]) { a[0] } else { 0 })
        });
    }

    #[test]
    fn a_biped_model_form_takes_the_first_slot_it_fills_after_emptying_the_others() {
        let mut e = engine();
        let this = biped(&mut e);
        let form = 0x1111;
        let race = 0x2222;
        let other = 0x3333;
        set_slot(&mut e, this, 3, race, 0xa1, 0);
        set_slot(&mut e, this, 7, other, 0xa2, 0);
        set_slot(&mut e, this, 9, other, 0xa3, 0);
        set_slot(&mut e, this, 11, form, 0xa4, 0);
        returns(&mut e, GET_FORM_AS_BIPED_MODEL, 0x4800);
        races_are(&mut e, vec![race]);
        // The model fills slots 3, 7, 9 and 11 for the first question (any
        // form) and 7 and 9 for the second (the first of those is taken).
        e.register_double(FILLS_BIPED_SLOT, |_, a| {
            let slot = a[1];
            ret(match a[2] {
                0 => [3, 7, 9, 11].contains(&slot) as u32,
                _ => [7, 9].contains(&slot) as u32,
            })
        });
        let item = Ptr::new(0x9999);
        fn_004abad0(&mut e, this, Ptr::new(form), item, 0);
        // Slot 3 held a race: emptied by slot. Slots 7 and 9 held another
        // form: emptied by form. Slot 11 already held `form`: untouched
        // until it is overwritten below (it is not the first slot filled).
        assert_eq!(slot_words(&e, object(this, 3)), [0; 4]);
        assert_eq!(slot_words(&e, object(this, 9)), [0; 4]);
        assert_eq!(slot_words(&e, object(this, 7)), [form, 0x9999, 0, 0]);
        assert_eq!(slot_words(&e, object(this, 11)), [form, 0xa4, 0, 0]);
    }

    #[test]
    fn a_race_goes_into_the_given_slot() {
        let mut e = engine();
        let this = biped(&mut e);
        let race = 0x2222;
        set_slot(&mut e, this, 6, 0x5151, 0xa1, 0);
        races_are(&mut e, vec![race]);
        fn_004abad0(&mut e, this, Ptr::new(race), Ptr::new(0x9999), 6);
        assert_eq!(slot_words(&e, object(this, 6)), [race, 0x9999, 0, 0]);
        // Neither a biped model nor a race: nothing changes.
        fn_004abad0(&mut e, this, Ptr::new(0x4242), Ptr::new(0x7777), 8);
        assert_eq!(slot_words(&e, object(this, 8)), [0; 4]);
    }

    #[test]
    fn the_first_person_player_gets_the_slots_own_item() {
        let mut e = engine();
        let this = biped(&mut e);
        let race = 0x2222;
        races_are(&mut e, vec![race]);
        e.set_global(PLAYER, 0x1000);
        returns(&mut e, IS_FIRST_PERSON_BIPED, 1);
        returns(&mut e, FN_004AF950, 1);
        returns(&mut e, FN_004AFA20, 0x5a5a);
        log(&mut e);
        fn_004abad0(&mut e, this, Ptr::new(race), Ptr::new(0x9999), 6);
        assert_eq!(slot_words(&e, object(this, 6)), [race, 0x5a5a, 0, 0]);
        let calls = log(&mut e);
        assert_eq!(
            calls_to(&calls, IS_FIRST_PERSON_BIPED),
            vec![vec![0x1000, this.addr()]]
        );
        let asked = calls_to(&calls, FN_004AF950);
        assert_eq!((asked[0][0], asked[0][1]), (this.addr(), 0x9999));
        // The item `004af950` answered no for is kept, and a null item skips the question.
        returns(&mut e, FN_004AF950, 0);
        fn_004abad0(&mut e, this, Ptr::new(race), Ptr::new(0x9999), 6);
        assert_eq!(slot_words(&e, object(this, 6)), [race, 0x9999, 0, 0]);
        log(&mut e);
        fn_004abad0(&mut e, this, Ptr::new(race), Ptr::NULL, 6);
        assert!(calls_to(&log(&mut e), FN_004AF950).is_empty());
    }

    /// The setting object answers: a text pointer whose first byte is `byte`
    /// for `setting`.
    fn setting_is(e: &mut Engine, settings: Vec<(u32, u8)>) {
        let byte = |e: &mut Engine, value: u8| {
            let address = e.mem.alloc(8);
            e.mem.set_u8(address, value);
            address
        };
        let table: Vec<(u32, u32)> = settings.into_iter().map(|(s, v)| (s, byte(e, v))).collect();
        e.register_double(SETTING_BYTE_POINTER, move |_, a| {
            ret(table.iter().find(|(s, _)| *s == a[0]).unwrap().1)
        });
    }

    /// A requester whose base form is an NPC (the casts succeed).
    fn npc_requester(e: &mut Engine, this: Ptr<BipedAnim>) -> u32 {
        let requester = 0x0abc_0000;
        e.set(this, BipedAnim::m_pRequester, Ptr::new(requester));
        returns(e, REFERENCE_FORM, 0x0ccc_0000);
        races_are(e, vec![0x0ccc_0000]);
        requester
    }

    #[test]
    fn queueing_models_queues_each_part_and_the_biped_model_path() {
        let mut e = engine();
        e.set_global(MODEL_LOADER, 0x5555);
        let this = biped(&mut e);
        npc_requester(&mut e, this);
        setting_is(&mut e, vec![(SETTING_FACE, 0)]);
        let part = block(&mut e, 0x40);
        e.mem.set_u8(part + 0x14, 0x0e);
        set_slot(&mut e, this, 3, 0x1111, part, 0);
        returns(&mut e, GET_FORM_AS_BIPED_MODEL, 0x4800);
        let path = text(&mut e, "meshes\\x.nif");
        returns(&mut e, BIPED_MODEL_PATH, path);
        log(&mut e);
        fn_004abd30(&mut e, this, 11, Ptr::new(0x7a7a), 22);
        let calls = log(&mut e);
        let order = addresses(&calls);
        assert_eq!(order[0], QUEUE_OBJECT_INIT);
        assert_eq!(
            calls_to(&calls, QUEUE_MODEL_FORM),
            vec![vec![0x5555, part, 11, 22, 3, 1, 0, 0]]
        );
        assert_eq!(
            calls_to(&calls, QUEUE_MODEL),
            vec![vec![0x5555, path, 11, 22, 3, 1, 0, 0]]
        );
        // The mask of the part flags drives the skin textures: bits 2, 4 and 8.
        let slots: Vec<u32> = calls_to(&calls, SKIN_TEXTURE_PATH)
            .into_iter()
            .map(|a| a[2])
            .collect();
        assert_eq!(slots, vec![2, 4, 3]);
        let used = calls_to(&calls, QUEUE_OBJECT_USE);
        assert_eq!(used.len(), 1);
        assert_eq!(used[0][0], 0x7a7a);
        assert_eq!(*order.last().unwrap(), QUEUE_OBJECT_FREE);
    }

    #[test]
    fn queueing_models_hands_a_face_gen_slot_to_the_face_gen_queue() {
        let mut e = engine();
        e.set_global(MODEL_LOADER, 0x5555);
        let this = biped(&mut e);
        npc_requester(&mut e, this);
        setting_is(
            &mut e,
            vec![
                (SETTING_FACE, 1),
                (SETTING_PAIR_FIRST, 1),
                (SETTING_PAIR_SECOND, 1),
            ],
        );
        returns(&mut e, FN_004AE8A0, 1);
        returns(&mut e, SETTING_BYTE_VALUE, 1);
        let part = block(&mut e, 0x40);
        set_slot(&mut e, this, 3, 0x1111, part, 0);
        log(&mut e);
        fn_004abd30(&mut e, this, 11, Ptr::new(0x7a7a), 22);
        let calls = log(&mut e);
        let faces = calls_to(&calls, FN_004AEE60);
        assert_eq!(faces.len(), 1);
        assert_eq!(
            (faces[0][0], faces[0][1], faces[0][2], faces[0][4]),
            (this.addr(), 11, 22, 3)
        );
        assert!(calls_to(&calls, QUEUE_MODEL_FORM).is_empty());
        // One of the pair of settings empty: the part is queued normally.
        setting_is(
            &mut e,
            vec![
                (SETTING_FACE, 1),
                (SETTING_PAIR_FIRST, 1),
                (SETTING_PAIR_SECOND, 0),
            ],
        );
        fn_004abd30(&mut e, this, 11, Ptr::new(0x7a7a), 22);
        let calls = log(&mut e);
        assert!(calls_to(&calls, FN_004AEE60).is_empty());
        assert_eq!(calls_to(&calls, QUEUE_MODEL_FORM).len(), 1);
    }

    #[test]
    fn queueing_models_for_the_player_also_queues_the_alternative_part() {
        let mut e = engine();
        e.set_global(MODEL_LOADER, 0x5555);
        let this = biped(&mut e);
        let requester = npc_requester(&mut e, this);
        e.set_global(PLAYER, requester);
        setting_is(&mut e, vec![(SETTING_FACE, 0)]);
        let part = block(&mut e, 0x40);
        set_slot(&mut e, this, 3, 0x1111, part, 0);
        set_slot(&mut e, this, 4, 0x1112, 0xffff_ffff, 0);
        returns(&mut e, FN_004AFA50, 0x6b6b);
        log(&mut e);
        fn_004abd30(&mut e, this, 1, Ptr::new(0x7a7a), 2);
        let calls = log(&mut e);
        // The alternative first, then the part; the slot with part -1 is skipped.
        assert_eq!(
            calls_to(&calls, QUEUE_MODEL_FORM),
            vec![
                vec![0x5555, 0x6b6b, 1, 2, 3, 1, 0, 0],
                vec![0x5555, part, 1, 2, 3, 1, 0, 0]
            ]
        );
        assert_eq!(
            calls_to(&calls, FN_004AFA50),
            vec![vec![this.addr(), 3, part]]
        );
    }

    #[test]
    fn queueing_models_for_something_that_is_not_an_npc_only_tells_the_object() {
        let mut e = engine();
        let this = biped(&mut e);
        e.set(this, BipedAnim::m_pRequester, Ptr::new(0x0abc_0000));
        let part = block(&mut e, 0x40);
        set_slot(&mut e, this, 3, 0x1111, part, 0);
        log(&mut e);
        fn_004abd30(&mut e, this, 1, Ptr::new(0x7a7a), 2);
        let order = addresses(&log(&mut e));
        assert_eq!(
            order,
            vec![
                QUEUE_OBJECT_INIT,
                REFERENCE_FORM,
                RT_DYNAMIC_CAST,
                QUEUE_OBJECT_USE,
                QUEUE_OBJECT_FREE
            ]
        );
    }

    #[test]
    fn the_part_flags_are_bits_one_to_three() {
        let mut e = engine();
        let item = block(&mut e, 0x40);
        e.mem.set_u8(item + 0x14, 0xff);
        assert_eq!(fn_004abf80(&mut e, Ptr::new(item)), 0x0e);
        e.mem.set_u8(item + 0x14, 0x01);
        assert_eq!(fn_004abf80(&mut e, Ptr::new(item)), 0);
    }

    #[test]
    fn the_pair_of_settings_must_both_be_set() {
        let mut e = engine();
        setting_is(
            &mut e,
            vec![(SETTING_PAIR_FIRST, 1), (SETTING_PAIR_SECOND, 1)],
        );
        assert!(fn_004abfa0(&mut e));
        setting_is(
            &mut e,
            vec![(SETTING_PAIR_FIRST, 1), (SETTING_PAIR_SECOND, 0)],
        );
        assert!(!fn_004abfa0(&mut e));
        setting_is(
            &mut e,
            vec![(SETTING_PAIR_FIRST, 0), (SETTING_PAIR_SECOND, 1)],
        );
        assert!(!fn_004abfa0(&mut e));
    }

    #[test]
    fn skin_textures_are_queued_for_the_masked_slots_and_the_race_texture() {
        let mut e = engine();
        e.set_global(MODEL_LOADER, 0x5555);
        let this = biped(&mut e);
        npc_requester(&mut e, this);
        returns(&mut e, READ_WORD_PLUS_4, 0x7ace);
        returns(&mut e, BODY_MOD_TEXTURE_FILE_NAME, 1);
        log(&mut e);
        fn_004abfe0(&mut e, this, 0, 5, 6);
        assert!(log(&mut e).is_empty());
        fn_004abfe0(&mut e, this, 8, 5, 6);
        let calls = log(&mut e);
        let skins = calls_to(&calls, SKIN_TEXTURE_PATH);
        assert_eq!(skins.len(), 1);
        assert_eq!(
            (skins[0][0], skins[0][1], skins[0][2]),
            (0x7ace, 0x0ccc_0000, 3)
        );
        let names = calls_to(&calls, BODY_MOD_TEXTURE_NAME);
        let files = calls_to(&calls, BODY_MOD_TEXTURE_FILE_NAME);
        assert_eq!((names[0][0], names[0][1]), (0x7ace, 0x0ccc_0000));
        assert_eq!(files[0][..3], [0x7ace, 0x0ccc_0000, names[0][2]]);
        // The race texture file is queued with the loader and the two arguments.
        let queued = calls_to(&calls, QUEUE_TEXTURE);
        assert_eq!(queued.len(), 1);
        assert_eq!((queued[0][0], queued[0][2], queued[0][3]), (0x5555, 5, 6));
        assert_eq!(queued[0][1], files[0][3]);
    }

    #[test]
    fn skin_textures_need_an_npc() {
        let mut e = engine();
        let this = biped(&mut e);
        e.set(this, BipedAnim::m_pRequester, Ptr::new(0x0abc_0000));
        log(&mut e);
        fn_004abfe0(&mut e, this, 2, 5, 6);
        assert!(calls_to(&log(&mut e), SKIN_TEXTURE_PATH).is_empty());
    }

    #[test]
    fn the_race_of_an_npc_is_read_through_its_inner_object() {
        let mut e = engine();
        returns(&mut e, READ_WORD_PLUS_4, 0x7ace);
        assert_eq!(fn_004ac110(&mut e, Ptr::new(0x1000)), 0x7ace);
        assert_eq!(log(&mut e), vec![(READ_WORD_PLUS_4, vec![0x110c])]);
    }

    #[test]
    fn a_skin_texture_is_queued_with_its_normal_map() {
        let mut e = engine();
        e.set_global(MODEL_LOADER, 0x5555);
        let this = biped(&mut e);
        returns(&mut e, READ_WORD_PLUS_4, 0x7ace);
        // No texture for the race: nothing is queued.
        biped_anim_queue_skin_texture(&mut e, this, Ptr::new(0x1000), 4, 8, 9);
        assert!(calls_to(&log(&mut e), QUEUE_TEXTURE).is_empty());
        // A texture and a normal map: both are queued.
        returns(&mut e, SKIN_TEXTURE_PATH, 1);
        e.register_double(MODIFIED_TEXTURE_FILENAME, |e, a| {
            e.mem.set_u8(a[0], b'n');
            ret(a[0])
        });
        biped_anim_queue_skin_texture(&mut e, this, Ptr::new(0x1000), 4, 8, 9);
        let calls = log(&mut e);
        let skin = calls_to(&calls, SKIN_TEXTURE_PATH);
        assert_eq!(skin[0][..3], [0x7ace, 0x1000, 4]);
        let queued = calls_to(&calls, QUEUE_TEXTURE);
        assert_eq!(queued.len(), 2);
        assert_eq!(queued[0], vec![0x5555, skin[0][3], 8, 9]);
        let modified = calls_to(&calls, MODIFIED_TEXTURE_FILENAME);
        assert_eq!(modified[0][1..], [skin[0][3], NORMAL_MAP_SUFFIX, 1]);
        assert_eq!(queued[1], vec![0x5555, modified[0][0], 8, 9]);
        // The normal map is left out when its path is empty.
        returns(&mut e, MODIFIED_TEXTURE_FILENAME, 0);
        biped_anim_queue_skin_texture(&mut e, this, Ptr::new(0x1000), 4, 8, 9);
        assert_eq!(calls_to(&log(&mut e), QUEUE_TEXTURE).len(), 1);
    }

    /// A biped for `load_biped_parts` with one loadable part in `slot` (its
    /// bone index in the table is `bone_index`): the model, the form with a
    /// name, the loaded file and its clone's node.
    struct Scene {
        this: Ptr<BipedAnim>,
        node: u32,
        form: u32,
        model: u32,
    }

    fn scene(e: &mut Engine, slot: u32, bone_index: u32) -> Scene {
        tables(e);
        e.mem.set_u32(SLOT_BONES + 4 * slot, bone_index);
        e.set_global(MODEL_LOADER, 0x5555);
        let this = biped(e);
        e.set(this, BipedAnim::m_pRequester, Ptr::new(0x0abc_0000));
        e.set(this, BipedAnim::root, Ptr::new(0x1818));
        let model = object_with_slots(e, &[(0x14, 0xabc0), (0x1c, 0)]);
        let form = object_with_slots(e, &[(0xac, 0), (0x130, 0x7e7e)]);
        set_slot(e, this, slot, form, model, 0);
        let node = object_with_slots(e, &[(0x0c, 1), (0x10, 0)]);
        let copy = object_with_slots(e, &[(0x0c, node)]);
        returns(e, MODEL_HAS_PATH, 1);
        returns(e, LOAD_FILE, 0xf11e);
        returns(e, NI_OBJECT_CLONE, copy);
        returns(e, FORM_ID, 0x4321);
        Scene {
            this,
            node,
            form,
            model,
        }
    }

    #[test]
    fn a_part_that_did_not_change_takes_the_buffered_clone_over() {
        let mut e = engine();
        let this = biped(&mut e);
        e.set(this, BipedAnim::m_pRequester, Ptr::new(0x0abc_0000));
        set_slot(&mut e, this, 2, 0x1111, 0x2222, 0);
        e.set(buffered(this, 2), BipedObject::pParent, Ptr::new(0x1111));
        e.set(buffered(this, 2), BipedObject::pPart, Ptr::new(0x2222));
        e.set(buffered(this, 2), BipedObject::pPartClone, Ptr::new(0x3333));
        returns(&mut e, ACTOR_ROOT, 0x1234);
        log(&mut e);
        biped_anim_load_biped_parts(&mut e, this, 0);
        assert_eq!(slot_words(&e, object(this, 2)), [0x1111, 0x2222, 0x3333, 0]);
        assert_eq!(slot_words(&e, buffered(this, 2)), [0; 4]);
        let calls = log(&mut e);
        assert!(calls_to(&calls, LOAD_FILE).is_empty());
        assert_eq!(calls_to(&calls, ACTOR_ROOT), vec![vec![0x0abc_0000]]);
        let last = calls.last().unwrap();
        assert_eq!(last, &(UPDATE_PROPERTIES, vec![0x1234]));
    }

    #[test]
    fn a_reused_part_adjusts_its_skin_complexion_when_the_flag_is_set() {
        let mut e = engine();
        let this = biped(&mut e);
        e.set(this, BipedAnim::m_pRequester, Ptr::new(0x0abc_0000));
        e.set_global(PALETTE_FLAG, 1u8);
        returns(&mut e, REFERENCE_FORM, 0x77);
        let list = 0x6600_0000;
        let node = object_with_slots(&mut e, &[(0x0c, list)]);
        set_slot(&mut e, this, 2, 0x1111, 0x2222, 0);
        e.set(buffered(this, 2), BipedObject::pParent, Ptr::new(0x1111));
        e.set(buffered(this, 2), BipedObject::pPart, Ptr::new(0x2222));
        e.set(buffered(this, 2), BipedObject::pPartClone, Ptr::new(node));
        returns(&mut e, CHILD_COUNT, 2);
        e.register_double(CHILD_AT, |_, a| ret(0x8000 + a[1]));
        log(&mut e);
        biped_anim_load_biped_parts(&mut e, this, 0);
        let calls = log(&mut e);
        assert_eq!(calls_to(&calls, CLEAR_BODY_PALETTE), vec![vec![0x77]]);
        assert_eq!(
            calls_to(&calls, FN_004AF490),
            vec![vec![this.addr(), 0x8000, 0], vec![this.addr(), 0x8001, 0]]
        );
        // A clone that is not a node is adjusted itself.
        let leaf = object_with_slots(&mut e, &[(0x0c, 0)]);
        set_slot(&mut e, this, 2, 0x1111, 0x2222, 0);
        set_slot(&mut e, this, 0, 0, 0, 0);
        e.set(buffered(this, 2), BipedObject::pParent, Ptr::new(0x1111));
        e.set(buffered(this, 2), BipedObject::pPart, Ptr::new(0x2222));
        e.set(buffered(this, 2), BipedObject::pPartClone, Ptr::new(leaf));
        biped_anim_load_biped_parts(&mut e, this, 0);
        assert_eq!(
            calls_to(&log(&mut e), FN_004AF490),
            vec![vec![this.addr(), leaf, 0]]
        );
    }

    #[test]
    fn a_reused_back_slot_keeps_the_weapon_flag_of_the_weapon() {
        let mut e = engine();
        let this = biped(&mut e);
        e.set(this, BipedAnim::m_pRequester, Ptr::new(0x0abc_0000));
        let weapon = block(&mut e, 0x300);
        e.mem.set_u8(weapon + 0x100, 0x10);
        // Slot 5 holds the weapon form; slot 7 is reused with its clone.
        e.set(object(this, 5), BipedObject::pParent, Ptr::new(weapon));
        set_slot(&mut e, this, 7, 0x1111, 0x2222, 0);
        e.set(buffered(this, 7), BipedObject::pParent, Ptr::new(0x1111));
        e.set(buffered(this, 7), BipedObject::pPart, Ptr::new(0x2222));
        e.set(buffered(this, 7), BipedObject::pPartClone, Ptr::new(0x3333));
        // (Slot 5 itself would be loaded through 004ab750: make it a non-weapon.)
        returns(&mut e, FORM_TYPE, 0x2a);
        log(&mut e);
        biped_anim_load_biped_parts(&mut e, this, 0);
        let calls = log(&mut e);
        assert_eq!(calls_to(&calls, SET_WEAPON_FLAG), vec![vec![0x3333, 1]]);
    }

    #[test]
    fn a_changed_slot_lets_go_of_its_buffered_part() {
        let mut e = engine();
        let this = biped(&mut e);
        e.set(this, BipedAnim::m_pRequester, Ptr::new(0x0abc_0000));
        let model = object_with_slots(&mut e, &[(0x14, 0xabc0)]);
        e.set(buffered(this, 4), BipedObject::pParent, Ptr::new(0x1111));
        e.set(buffered(this, 4), BipedObject::pPart, Ptr::new(model));
        e.set(buffered(this, 4), BipedObject::pPartClone, Ptr::new(0x3333));
        e.set_global(MODEL_LOADER, 0x5555);
        log(&mut e);
        biped_anim_load_biped_parts(&mut e, this, 0);
        let calls = log(&mut e);
        assert_eq!(slot_words(&e, buffered(this, 4)), [0; 4]);
        assert_eq!(calls_to(&calls, PREPARE_DETACH), vec![vec![0x3333]]);
        assert_eq!(calls_to(&calls, RELEASE_MODEL), vec![vec![0x5555, 0xabc0]]);
    }

    #[test]
    fn a_new_part_is_loaded_cloned_named_and_attached_to_its_bone() {
        let mut e = engine();
        let s = scene(&mut e, 8, 4);
        let bone_node = object_with_slots(&mut e, &[(0xdc, 0)]);
        e.set(bone(s.this, 4), BipedBone::pParent, Ptr::new(bone_node));
        returns(&mut e, FIXED_STRING_INIT, 0x1f1f);
        log(&mut e);
        biped_anim_load_biped_parts(&mut e, s.this, 0);
        assert_eq!(
            e.get(object(s.this, 8), BipedObject::pPartClone).addr(),
            s.node
        );
        let calls = log(&mut e);
        assert_eq!(
            calls_to(&calls, LOAD_FILE),
            vec![vec![0x5555, 0xabc0, 3, 1, 0, 0, 0]]
        );
        // Cloned with a fresh cloning process (no morpher controller).
        let cloned = calls_to(&calls, NI_OBJECT_CLONE);
        assert_eq!(cloned[0][0], 0xf11e);
        // The clone's node is reset to the origin and identity, then attached
        // to the root by 004ae250 (not skinned) and to its bone.
        assert_eq!(
            calls_to(&calls, SET_TRANSLATION),
            vec![vec![s.node, ZERO_TRANSLATION]]
        );
        assert_eq!(
            calls_to(&calls, SET_ROTATION),
            vec![vec![s.node, IDENTITY_ROTATION]]
        );
        assert_eq!(
            calls_to(&calls, FN_004AE250),
            vec![vec![0x1818, s.node, 0, s.this.addr(), 8, 0]]
        );
        let slot_name = e.mem.u32(SLOT_NAMES + 4 * 8);
        let sprintf = calls_to(&calls, SPRINTF);
        assert_eq!(
            sprintf[0][1..],
            [SLOT_NAME_FORMAT, slot_name, 0x7e7e, 0x4321]
        );
        assert_eq!(calls_to(&calls, SET_NAME), vec![vec![s.node, 0x1f1f]]);
        assert_eq!(
            calls_to(&calls, FN_004AF240),
            vec![vec![0xf11e, s.node, 8, 0x0abc_0000]]
        );
        let attach = slot_target(&e, bone_node, 0xdc);
        assert_eq!(calls_to(&calls, attach), vec![vec![bone_node, s.node, 1]]);
        // The cloning process is destroyed (its maps were never made).
        assert!(calls_to(&calls, SWAP_TEXTURES).is_empty());
        let _ = (s.form, s.model);
    }

    #[test]
    fn a_morphing_model_is_deep_copied_with_the_same_textures() {
        let mut e = engine();
        let s = scene(&mut e, 8, 4);
        returns(&mut e, HAS_MORPHER_CONTROLLER, 1);
        let copy = object_with_slots(&mut e, &[(0x0c, s.node)]);
        returns(&mut e, DEEP_COPY_SAME_TEXTURES, copy);
        e.set_global(TES_GLOBAL, 0x7e57);
        log(&mut e);
        biped_anim_load_biped_parts(&mut e, s.this, 0);
        let calls = log(&mut e);
        let deep = calls_to(&calls, DEEP_COPY_SAME_TEXTURES);
        assert_eq!((deep[0][0], deep[0][1]), (0x7e57, 0xf11e));
        assert!(calls_to(&calls, NI_OBJECT_CLONE).is_empty());
        assert_eq!(
            e.get(object(s.this, 8), BipedObject::pPartClone).addr(),
            s.node
        );
    }

    #[test]
    fn textures_are_swapped_when_the_model_or_the_form_ask() {
        let mut e = engine();
        let s = scene(&mut e, 8, 4);
        // The model has a texture swap (virtual slot 0x1c) and the form is a
        // platform-language form (virtual slot 0xac).
        let swap_model = object_with_slots(&mut e, &[(0x14, 0xabc0), (0x1c, 0x5757)]);
        let swap_form = object_with_slots(&mut e, &[(0xac, 1), (0x130, 0x7e7e)]);
        e.set(object(s.this, 8), BipedObject::pPart, Ptr::new(swap_model));
        e.set(object(s.this, 8), BipedObject::pParent, Ptr::new(swap_form));
        log(&mut e);
        biped_anim_load_biped_parts(&mut e, s.this, 0);
        let calls = log(&mut e);
        assert_eq!(calls_to(&calls, SWAP_TEXTURES), vec![vec![0x5757, s.node]]);
        assert_eq!(calls_to(&calls, SWAP_PLATFORM_TEXTURES), vec![vec![s.node]]);
    }

    #[test]
    fn a_part_without_a_bone_that_is_not_skinned_is_logged_and_dropped() {
        let mut e = engine();
        let s = scene(&mut e, 3, 0xffff_ffff);
        returns(&mut e, ACTOR_LOG_NAME, 0x1919);
        let parent = object_with_slots(&mut e, &[(0xe8, 0)]);
        returns(&mut e, NODE_PARENT, parent);
        log(&mut e);
        biped_anim_load_biped_parts(&mut e, s.this, 0);
        let calls = log(&mut e);
        assert_eq!(
            calls_to(&calls, LOG),
            vec![vec![SHOULD_BE_SKINNED_FORMAT, 0xabc0, 0x1919]]
        );
        let target = slot_target(&e, parent, 0xe8);
        assert_eq!(calls_to(&calls, target), vec![vec![parent, s.node]]);
        assert!(e.get(object(s.this, 3), BipedObject::pPartClone).is_null());
    }

    #[test]
    fn a_part_whose_bone_is_missing_goes_under_the_root() {
        let mut e = engine();
        let s = scene(&mut e, 8, 4);
        let root = object_with_slots(&mut e, &[(0xdc, 0)]);
        e.set(s.this, BipedAnim::root, Ptr::new(root));
        returns(&mut e, NODE_PARENT, 0x6767);
        log(&mut e);
        biped_anim_load_biped_parts(&mut e, s.this, 0);
        let calls = log(&mut e);
        let target = slot_target(&e, root, 0xdc);
        assert_eq!(calls_to(&calls, target), vec![vec![root, s.node, 1]]);
        assert_eq!(
            e.get(object(s.this, 8), BipedObject::pPartClone).addr(),
            s.node
        );
    }

    #[test]
    fn dismembered_limbs_are_hidden_on_the_loaded_part() {
        let mut e = engine();
        let s = scene(&mut e, 8, 4);
        returns(&mut e, GET_DISMEMBERMENT_EXTRA, 0x4545);
        e.register_double(
            LIMB_DISMEMBERED,
            |_, a| ret((a[1] == 2 || a[1] == 9) as u32),
        );
        log(&mut e);
        biped_anim_load_biped_parts(&mut e, s.this, 0);
        let calls = log(&mut e);
        assert_eq!(
            calls_to(&calls, HIDE_LIMB),
            vec![vec![2, s.node], vec![9, s.node]]
        );
        assert_eq!(calls_to(&calls, LIMB_DISMEMBERED).len(), 15);
    }

    #[test]
    fn the_back_slot_part_gets_the_weapon_flag_when_the_weapon_has_it() {
        let mut e = engine();
        let s = scene(&mut e, 7, 3);
        let weapon = block(&mut e, 0x300);
        e.mem.set_u8(weapon + 0x100, 0x10);
        // Slot 5 holds a weapon that is reused (non-weapon type, no clone).
        e.set(object(s.this, 5), BipedObject::pParent, Ptr::new(weapon));
        returns(&mut e, FORM_TYPE, 0x2a);
        log(&mut e);
        biped_anim_load_biped_parts(&mut e, s.this, 0);
        let calls = log(&mut e);
        assert_eq!(calls_to(&calls, SET_WEAPON_FLAG), vec![vec![s.node, 1]]);
    }

    #[test]
    fn a_face_gen_slot_may_be_deferred_unless_the_flag_is_set() {
        let mut e = engine();
        let s = scene(&mut e, 3, 0xffff_ffff);
        setting_is(
            &mut e,
            vec![
                (SETTING_FACE, 1),
                (SETTING_PAIR_FIRST, 1),
                (SETTING_PAIR_SECOND, 1),
            ],
        );
        returns(&mut e, FN_004AE8A0, 1);
        returns(&mut e, SETTING_BYTE_VALUE, 1);
        log(&mut e);
        biped_anim_load_biped_parts(&mut e, s.this, 0);
        let calls = log(&mut e);
        assert_eq!(
            calls_to(&calls, FN_004AEDE0),
            vec![vec![s.this.addr(), 0, 0, 3]]
        );
        assert!(calls_to(&calls, LOAD_FILE).is_empty());
        // With the flag set the part is loaded (as a FaceGen slot).
        biped_anim_load_biped_parts(&mut e, s.this, 1);
        let calls = log(&mut e);
        assert!(calls_to(&calls, FN_004AEDE0).is_empty());
        assert_eq!(calls_to(&calls, LOAD_FILE)[0][5], 1);
    }

    #[test]
    fn the_first_person_player_loads_only_the_slots_two_to_six() {
        let mut e = engine();
        let s = scene(&mut e, 3, 0xffff_ffff);
        let requester = 0x1000;
        e.set_global(PLAYER, requester);
        e.set(s.this, BipedAnim::m_pRequester, Ptr::new(requester));
        returns(&mut e, IS_FIRST_PERSON_BIPED, 1);
        // Slots 1 and 8 also hold loadable parts; only slot 3 is loaded.
        let model = object_with_slots(&mut e, &[(0x14, 0xabc0), (0x1c, 0)]);
        let form = object_with_slots(&mut e, &[(0xac, 0), (0x130, 0x7e7e)]);
        set_slot(&mut e, s.this, 1, form, model, 0);
        set_slot(&mut e, s.this, 8, form, model, 0);
        log(&mut e);
        biped_anim_load_biped_parts(&mut e, s.this, 0);
        let calls = log(&mut e);
        assert_eq!(calls_to(&calls, LOAD_FILE).len(), 1);
    }

    #[test]
    fn the_players_weapon_slot_goes_through_the_weapon_loader() {
        let mut e = engine();
        tables(&mut e);
        let this = biped(&mut e);
        let requester = 0x1000;
        e.set_global(PLAYER, requester);
        e.set(this, BipedAnim::m_pRequester, Ptr::new(requester));
        let form = block(&mut e, 0x300);
        e.mem.set_u8(form + 0x100, 0x04);
        e.mem.set_u32(form + 0x12c, 0x2000);
        e.set(object(this, 5), BipedObject::pParent, Ptr::new(form));
        let item_change = 0x1c1c;
        let process = object_with_slots(&mut e, &[(0x148, item_change)]);
        returns(&mut e, SAVED_ACQUIRE_OBJECT, process);
        returns(&mut e, GET_MOD_SLOTS, 3);
        returns(&mut e, HAS_MOD_EFFECT_ACTIVE, 1);
        returns(&mut e, GET_BIPED, this.addr());
        returns(&mut e, WEAPON_MODEL_VALUE, 0x6d6d);
        // 004ab750 ignores a form that is not a weapon.
        returns(&mut e, FORM_TYPE, 0x2a);
        log(&mut e);
        biped_anim_load_biped_parts(&mut e, this, 0);
        let calls = log(&mut e);
        let effect = calls_to(&calls, HAS_MOD_EFFECT_ACTIVE);
        assert_eq!((effect[0][0], effect[0][1]), (item_change, 0x0e));
        assert_eq!(calls_to(&calls, GET_MOD_SLOTS), vec![vec![item_change]]);
        assert_eq!(calls_to(&calls, FORM_TYPE), vec![vec![form]]);
        // The modded weapon with the flag shows the weapon model value.
        assert_eq!(calls_to(&calls, SHOW_WEAPON), vec![vec![0x6d6d]]);
        // Without the mod effect and with the flag word bit set it does not.
        returns(&mut e, HAS_MOD_EFFECT_ACTIVE, 0);
        biped_anim_load_biped_parts(&mut e, this, 0);
        assert!(calls_to(&log(&mut e), SHOW_WEAPON).is_empty());
        // Without the flag word bit it does.
        e.mem.set_u32(form + 0x12c, 0);
        biped_anim_load_biped_parts(&mut e, this, 0);
        assert_eq!(calls_to(&log(&mut e), SHOW_WEAPON).len(), 1);
    }

    #[test]
    fn another_actors_weapon_slot_sets_the_weapon_directly() {
        let mut e = engine();
        tables(&mut e);
        let this = biped(&mut e);
        e.set(this, BipedAnim::m_pRequester, Ptr::new(0x0abc_0000));
        e.set(object(this, 5), BipedObject::pParent, Ptr::new(0x4141));
        log(&mut e);
        biped_anim_load_biped_parts(&mut e, this, 0);
        assert_eq!(
            calls_to(&log(&mut e), ACTOR_SET_WEAPON),
            vec![vec![0x0abc_0000, 0x4141]]
        );
    }

    #[test]
    fn a_face_gen_part_gets_its_coordinates_and_a_quarter_turn() {
        let mut e = engine();
        let s = scene(&mut e, 3, 0xffff_ffff);
        let npc = 0x0ccc_0000;
        returns(&mut e, REFERENCE_FORM, npc);
        returns(&mut e, FORM_TYPE, 0x2a);
        returns(&mut e, FN_004AE8A0, 1);
        returns(&mut e, FN_004AE790, 0x4141);
        e.mem.set_f32(QUARTER_TURN_BACK, -1.5707964);
        setting_is(&mut e, vec![(SETTING_FACE, 0)]);
        // The clone's node has one child, whose geometry has skin data.
        let geometry = object_with_slots(&mut e, &[(0xe4, 0)]);
        let child = object_with_slots(&mut e, &[(0x1c, geometry)]);
        returns(&mut e, CHILD_COUNT, 1);
        returns(&mut e, CHILD_AT, child);
        returns(&mut e, NODE_SKIN_DATA, 0x5a5a);
        returns(&mut e, NODE_ROTATION, 0x3000);
        returns(&mut e, MATRIX_MULTIPLY, 0x3100);
        log(&mut e);
        biped_anim_load_biped_parts(&mut e, s.this, 0);
        let calls = log(&mut e);
        let coords = calls_to(&calls, GET_FACE_COORD);
        assert_eq!(coords[0][0], npc);
        let construct = calls_to(&calls, VECTOR_CONSTRUCT);
        assert_eq!(
            construct[0][..],
            [coords[0][1], 0x20, 4, FACE_COORD_INIT, FACE_COORD_FREE]
        );
        assert_eq!(
            calls_to(&calls, VECTOR_DESTRUCT),
            vec![vec![coords[0][1], 0x20, 4, FACE_COORD_FREE]]
        );
        // The setting is off, so the rotation is turned by -pi/2 about Y.
        let turn = calls_to(&calls, MAKE_Y_ROTATION);
        assert_eq!(f32::from_bits(turn[0][1]), -1.5707964);
        let multiply = calls_to(&calls, MATRIX_MULTIPLY);
        assert_eq!(multiply[0][0], 0x3000);
        assert_eq!(multiply[0][2], turn[0][0]);
        let set = calls_to(&calls, SET_ROTATION);
        assert!(set.contains(&vec![geometry, 0x3100]));
        // The skin data was copied and set on the geometry.
        assert_eq!(calls_to(&calls, NI_OBJECT_DEEP_COPY)[0][0], 0x5a5a);
        let target = slot_target(&e, geometry, 0xe4);
        assert_eq!(calls_to(&calls, target).len(), 1);
    }

    #[test]
    fn a_child_that_is_not_a_geometry_is_logged_when_loading_face_gen() {
        let mut e = engine();
        let s = scene(&mut e, 3, 0xffff_ffff);
        returns(&mut e, FN_004AE8A0, 1);
        returns(&mut e, FN_004AE790, 0x4141);
        setting_is(&mut e, vec![(SETTING_FACE, 0)]);
        let child = object_with_slots(&mut e, &[(0x1c, 0)]);
        returns(&mut e, CHILD_COUNT, 1);
        returns(&mut e, CHILD_AT, child);
        log(&mut e);
        biped_anim_load_biped_parts(&mut e, s.this, 0);
        let calls = log(&mut e);
        assert_eq!(calls_to(&calls, LOG)[0], vec![NON_GEOMETRY_FORMAT, 0xabc0]);
    }

    #[test]
    fn the_cloning_process_flag_getters_read_their_bits() {
        let mut e = engine();
        let form = block(&mut e, 0x300);
        assert!(!fn_004ad010(&mut e, Ptr::new(form)));
        assert!(!fn_004ad030(&mut e, Ptr::new(form)));
        e.mem.set_u8(form + 0x100, 0x04);
        e.mem.set_u32(form + 0x12c, 0x2000);
        assert!(fn_004ad010(&mut e, Ptr::new(form)));
        assert!(fn_004ad030(&mut e, Ptr::new(form)));
        e.mem.set_u8(form + 0x100, 0xfb);
        e.mem.set_u32(form + 0x12c, 0xffff_dfff);
        assert!(!fn_004ad010(&mut e, Ptr::new(form)));
        assert!(!fn_004ad030(&mut e, Ptr::new(form)));
        e.set_global(CLONE_COPY_TYPE, 0x0102);
        e.set_global(CLONE_APPEND_CHAR, 0x2au8);
        assert_eq!(fn_004ad1b0(&mut e), 0x0102);
        assert_eq!(fn_004ad1c0(&mut e), 0x2a);
    }

    #[test]
    fn a_cloning_process_builds_its_two_maps() {
        let mut e = engine();
        e.set_global(CLONE_COPY_TYPE, 7);
        e.set_global(CLONE_APPEND_CHAR, 0x2au8);
        let next = Rc::new(RefCell::new(0x6000u32));
        let counter = next.clone();
        e.register_double(OPERATOR_NEW, move |_, _| {
            let mut value = counter.borrow_mut();
            let result = *value;
            *value += 0x100;
            ret(result)
        });
        e.register(FN_004AFB20, |_, a| ret(a[0]));
        e.register(FN_004AFB50, |_, a| ret(a[0]));
        let this = block(&mut e, 0x1c);
        log(&mut e);
        let result = fn_004ad0c0(&mut e, Ptr::new(this), 0x101);
        assert_eq!(result.addr(), this);
        let process = Ptr::<NiCloningProcess>::new(this);
        assert_eq!(
            e.get(process, NiCloningProcess::m_pkCloneMap).addr(),
            0x6000
        );
        assert_eq!(
            e.get(process, NiCloningProcess::m_pkProcessMap).addr(),
            0x6100
        );
        assert_eq!(e.get(process, NiCloningProcess::m_eCopyType), 7);
        assert_eq!(e.get(process, NiCloningProcess::m_cAppendChar), 0x2a);
        let calls = log(&mut e);
        assert_eq!(calls_to(&calls, OPERATOR_NEW), vec![vec![0x10], vec![0x10]]);
        assert_eq!(calls_to(&calls, FN_004AFB20), vec![vec![0x6000, 0x101]]);
        assert_eq!(calls_to(&calls, FN_004AFB50), vec![vec![0x6100, 0x101]]);
        // When the allocation fails the maps are null and nothing is built.
        returns(&mut e, OPERATOR_NEW, 0);
        fn_004ad0c0(&mut e, Ptr::new(this), 0x101);
        assert!(e.get(process, NiCloningProcess::m_pkCloneMap).is_null());
        assert!(e.get(process, NiCloningProcess::m_pkProcessMap).is_null());
        assert!(calls_to(&log(&mut e), FN_004AFB20).is_empty());
    }

    #[test]
    fn the_scaled_cloning_process_sets_the_scale_vector() {
        let mut e = engine();
        let this = block(&mut e, 0x1c);
        log(&mut e);
        let result = fn_004ad050(&mut e, Ptr::new(this), 2.5);
        assert_eq!(result.addr(), this);
        let process = Ptr::<NiCloningProcess>::new(this);
        assert_eq!(
            [
                e.get(process, NiCloningProcess::m_fScale_x),
                e.get(process, NiCloningProcess::m_fScale_y),
                e.get(process, NiCloningProcess::m_fScale_z)
            ],
            [2.5; 3]
        );
        let calls = log(&mut e);
        // Hash size 0x101 for the maps, the vector at +0x10 initialized.
        assert_eq!(calls_to(&calls, MATRIX_INIT), vec![vec![this + 0x10]]);
        assert_eq!(calls_to(&calls, OPERATOR_NEW).len(), 2);
        // The same through the uniform form (the float is one word).
        e.call(0x004a_d240, &args![this, 1.5f32]);
        assert_eq!(e.get(process, NiCloningProcess::m_fScale_y), 1.5);
    }

    #[test]
    fn destroying_a_cloning_process_deletes_its_maps() {
        let mut e = engine();
        let clone_map = object_with_slots(&mut e, &[(0, 0)]);
        let process_map = object_with_slots(&mut e, &[(0, 0)]);
        let this = block(&mut e, 0x1c);
        let process = Ptr::<NiCloningProcess>::new(this);
        e.set(process, NiCloningProcess::m_pkCloneMap, Ptr::new(clone_map));
        e.set(
            process,
            NiCloningProcess::m_pkProcessMap,
            Ptr::new(process_map),
        );
        log(&mut e);
        ni_cloning_process_destructor(&mut e, process);
        let calls = log(&mut e);
        let first = slot_target(&e, clone_map, 0);
        let second = slot_target(&e, process_map, 0);
        assert_eq!(
            calls,
            vec![(first, vec![clone_map, 1]), (second, vec![process_map, 1])]
        );
        // Null maps are left alone.
        e.set(process, NiCloningProcess::m_pkCloneMap, Ptr::NULL);
        e.set(process, NiCloningProcess::m_pkProcessMap, Ptr::NULL);
        fn_004ad1d0(&mut e, process);
        assert!(log(&mut e).is_empty());
    }

    #[test]
    fn nothing_is_attached_without_a_node() {
        let mut e = engine();
        let s = scene(&mut e, 3, 0xffff_ffff);
        returns(&mut e, CAST_TO_CLASS, 0);
        log(&mut e);
        let result =
            biped_anim_apply_skinned_objects(&mut e, s.this, Ptr::NULL, 3, 0, 0, Ptr::NULL);
        assert!(result.is_null());
        let calls = log(&mut e);
        assert_eq!(
            calls_to(&calls, CAST_TO_CLASS),
            vec![vec![SKINNED_CLASS, 0]]
        );
        assert!(calls_to(&calls, FIND_SKINNED_NODE).is_empty());
    }

    #[test]
    fn a_biped_without_a_root_keeps_the_node_and_records_the_skin_flag() {
        let mut e = engine();
        let s = scene(&mut e, 3, 0xffff_ffff);
        e.set(s.this, BipedAnim::root, Ptr::NULL);
        returns(&mut e, FIND_SKINNED_NODE, 1);
        let result =
            biped_anim_apply_skinned_objects(&mut e, s.this, Ptr::new(s.node), 3, 0, 0, Ptr::NULL);
        assert_eq!(result.addr(), s.node);
        assert!(e.get(object(s.this, 3), BipedObject::bSkinned));
    }

    #[test]
    fn a_node_without_skinned_objects_attaches_nothing() {
        let mut e = engine();
        let s = scene(&mut e, 3, 0xffff_ffff);
        e.mem.set_u8(object(s.this, 3).addr() + 0xc, 1);
        log(&mut e);
        let result =
            biped_anim_apply_skinned_objects(&mut e, s.this, Ptr::new(s.node), 3, 0, 0, Ptr::NULL);
        assert!(result.is_null());
        assert!(!e.get(object(s.this, 3), BipedObject::bSkinned));
        let calls = log(&mut e);
        assert_eq!(calls_to(&calls, FIND_SKINNED_NODE), vec![vec![s.node]]);
        assert!(calls_to(&calls, FN_004ADD70).is_empty());
    }

    #[test]
    fn skinned_objects_are_attached_under_the_skin_attachment_node() {
        let mut e = engine();
        let s = scene(&mut e, 3, 0xffff_ffff);
        returns(&mut e, FIND_SKINNED_NODE, 1);
        let attachment = 0x4a4a;
        returns(&mut e, FIND_OBJECT_BY_NAME, attachment);
        log(&mut e);
        // The node is not of the skinned class: it is attached as a whole.
        // The attached node has too few children to make a node, so nothing
        // comes back, and the node is referenced and released once.
        let result = biped_anim_apply_skinned_objects(
            &mut e,
            s.this,
            Ptr::new(s.node),
            3,
            1,
            0,
            Ptr::new(0x66),
        );
        assert!(result.is_null());
        assert!(e.get(object(s.this, 3), BipedObject::bSkinned));
        let calls = log(&mut e);
        assert_eq!(
            calls_to(&calls, FIND_OBJECT_BY_NAME),
            vec![vec![0x1818, SKIN_ATTACHMENT_NAME, 1]]
        );
        assert_eq!(
            calls_to(&calls, FN_004ADD70),
            vec![vec![s.node], vec![s.node]]
        );
        assert_eq!(
            calls_to(&calls, SET_TRANSLATION),
            vec![vec![s.node, ZERO_TRANSLATION]]
        );
        assert_eq!(calls_to(&calls, GET_EXTRA_DATA).len(), 1);
        let refs = calls_to(&calls, NI_POINTER_INIT);
        assert_eq!(refs.last().unwrap()[1], s.node);
    }

    #[test]
    fn without_a_skin_attachment_node_the_roots_parent_is_used() {
        let mut e = engine();
        let s = scene(&mut e, 3, 0xffff_ffff);
        returns(&mut e, FIND_SKINNED_NODE, 1);
        returns(&mut e, FIND_OBJECT_BY_NAME, 0);
        returns(&mut e, NODE_PARENT, 0x5b5b);
        log(&mut e);
        biped_anim_apply_skinned_objects(&mut e, s.this, Ptr::new(s.node), 3, 0, 0, Ptr::NULL);
        let calls = log(&mut e);
        assert_eq!(calls_to(&calls, NODE_PARENT), vec![vec![0x1818]]);
        assert_eq!(calls_to(&calls, NI_POINTER_SET).len(), 1);
        assert_eq!(calls_to(&calls, NI_POINTER_SET)[0][1], 0x5b5b);
    }

    /// Makes the node children come from `children`.
    fn children_are(e: &mut Engine, children: Vec<u32>) {
        returns(e, CHILD_COUNT, children.len() as u32);
        e.register_double(CHILD_AT, move |_, a| {
            ret(children.get(a[1] as usize).copied().unwrap_or(0))
        });
    }

    #[test]
    fn a_node_of_the_skinned_class_attaches_each_child() {
        let mut e = engine();
        let s = scene(&mut e, 3, 0xffff_ffff);
        returns(&mut e, FIND_SKINNED_NODE, 1);
        returns(&mut e, FIND_OBJECT_BY_NAME, 0x4a4a);
        returns(&mut e, CAST_TO_CLASS, 0x6600_0000);
        let first = object_with_slots(&mut e, &[]);
        let second = object_with_slots(&mut e, &[]);
        // The class's children are attached one after the other, with no
        // parent object; their own children are none.
        e.register_double(CHILD_COUNT, |_, a| {
            ret(if a[0] == 0x6600_0000 { 2 } else { 0 })
        });
        e.register_double(CHILD_AT, move |_, a| ret([first, second][a[1] as usize]));
        log(&mut e);
        biped_anim_apply_skinned_objects(&mut e, s.this, Ptr::new(s.node), 3, 0, 0, Ptr::NULL);
        let calls = log(&mut e);
        assert_eq!(
            calls_to(&calls, SET_TRANSLATION),
            vec![
                vec![first, ZERO_TRANSLATION],
                vec![second, ZERO_TRANSLATION]
            ]
        );
    }

    /// A node for `attach`: it has a virtual slot 0xe8 (remove a child), the
    /// biped has a root.
    fn attach_scene(e: &mut Engine) -> (Ptr<BipedAnim>, u32) {
        let s = scene(e, 3, 0xffff_ffff);
        let node = object_with_slots(e, &[(0xe8, 0)]);
        (s.this, node)
    }

    #[test]
    fn attaching_a_node_without_children_returns_nothing() {
        let mut e = engine();
        let (this, node) = attach_scene(&mut e);
        e.set_global(EXTRA_DATA_KEY, 0x9900);
        log(&mut e);
        let result = biped_anim_attach_skinned_object(
            &mut e,
            this,
            Ptr::new(node),
            Ptr::NULL,
            3,
            0,
            0,
            Ptr::NULL,
        );
        assert!(result.is_null());
        let calls = log(&mut e);
        assert_eq!(calls_to(&calls, FN_004ADD70), vec![vec![node], vec![node]]);
        assert_eq!(
            calls_to(&calls, SET_TRANSLATION),
            vec![vec![node, ZERO_TRANSLATION]]
        );
        assert_eq!(calls_to(&calls, GET_EXTRA_DATA), vec![vec![node, 0x9900]]);
        let collected = calls_to(&calls, FN_004AFF00);
        assert_eq!(collected[0][1..], [0, 1]);
        assert_eq!(calls_to(&calls, FN_004ADE20), vec![vec![collected[0][0]]]);
    }

    #[test]
    fn a_geometry_in_the_second_child_makes_a_new_node_under_the_parent_object() {
        let mut e = engine();
        let (this, node) = attach_scene(&mut e);
        let parent = object_with_slots(&mut e, &[(0xdc, 0)]);
        let first = object_with_slots(&mut e, &[(0x18, 0x1)]);
        let second = object_with_slots(&mut e, &[(0x18, 0x1)]);
        children_are(&mut e, vec![first, second]);
        returns(&mut e, NI_OPERATOR_NEW, 0x9000);
        returns(&mut e, NI_NODE_INIT, 0x9100);
        log(&mut e);
        let result = biped_anim_attach_skinned_object(
            &mut e,
            this,
            Ptr::new(node),
            Ptr::new(parent),
            3,
            0,
            0,
            Ptr::NULL,
        );
        // The new node is what comes back when no child has data.
        assert_eq!(result.addr(), 0x9100);
        let calls = log(&mut e);
        assert_eq!(calls_to(&calls, NI_OPERATOR_NEW), vec![vec![0xac]]);
        assert_eq!(calls_to(&calls, NI_NODE_INIT), vec![vec![0x9000, 0]]);
        let add = slot_target(&e, parent, 0xdc);
        assert_eq!(calls_to(&calls, add), vec![vec![parent, 0x9100, 1]]);
    }

    #[test]
    fn an_allocation_failure_for_the_new_node_is_passed_on_as_null() {
        let mut e = engine();
        let (this, node) = attach_scene(&mut e);
        let parent = object_with_slots(&mut e, &[(0xdc, 0)]);
        let first = object_with_slots(&mut e, &[(0x18, 0x1)]);
        let second = object_with_slots(&mut e, &[(0x18, 0x1)]);
        children_are(&mut e, vec![first, second]);
        log(&mut e);
        let result = biped_anim_attach_skinned_object(
            &mut e,
            this,
            Ptr::new(node),
            Ptr::new(parent),
            3,
            0,
            0,
            Ptr::NULL,
        );
        assert!(result.is_null());
        let calls = log(&mut e);
        assert!(calls_to(&calls, NI_NODE_INIT).is_empty());
        let add = slot_target(&e, parent, 0xdc);
        assert_eq!(calls_to(&calls, add), vec![vec![parent, 0, 1]]);
    }

    #[test]
    fn children_without_a_geometry_are_collected_and_removed() {
        let mut e = engine();
        let (this, node) = attach_scene(&mut e);
        let plain = object_with_slots(&mut e, &[(0x18, 0)]);
        let geometry = object_with_slots(&mut e, &[(0x18, 0x1)]);
        children_are(&mut e, vec![plain, geometry]);
        log(&mut e);
        biped_anim_attach_skinned_object(
            &mut e,
            this,
            Ptr::new(node),
            Ptr::NULL,
            3,
            0,
            0,
            Ptr::NULL,
        );
        let calls = log(&mut e);
        // The plain child went into the collection and out of the node.
        let adds = calls_to(&calls, FN_004AFC50);
        assert_eq!(adds.len(), 1);
        let collected = calls_to(&calls, FN_004AFF00)[0][0];
        assert_eq!(adds[0][0], collected);
        let references = calls_to(&calls, NI_POINTER_INIT);
        assert!(references
            .iter()
            .any(|r| r[0] == adds[0][1] && r[1] == plain));
        let remove = slot_target(&e, node, 0xe8);
        assert_eq!(calls_to(&calls, remove), vec![vec![node, plain]]);
        // The geometry child is adjusted (004af490) but not collected.
        assert_eq!(
            calls_to(&calls, FN_004AF490),
            vec![vec![this.addr(), geometry, 0]]
        );
    }

    #[test]
    fn children_named_like_the_third_slot_are_collected_when_asked() {
        let mut e = engine();
        let (this, node) = attach_scene(&mut e);
        let named = object_with_slots(&mut e, &[(0x18, 0x1)]);
        children_are(&mut e, vec![named]);
        let name = text(&mut e, "UpperBody1");
        returns(&mut e, NAME_TEXT, name);
        let prefix = e.mem.u32(SLOT_NAMES + 8);
        returns(&mut e, TEXT_LENGTH, 9);
        returns(&mut e, COMPARE_PREFIX, 0);
        // The pip-boy prefix test never matches in this test.
        returns(&mut e, COMPARE_TEXT, 1);
        log(&mut e);
        biped_anim_attach_skinned_object(
            &mut e,
            this,
            Ptr::new(node),
            Ptr::NULL,
            3,
            1,
            0,
            Ptr::NULL,
        );
        let calls = log(&mut e);
        assert_eq!(calls_to(&calls, TEXT_LENGTH), vec![vec![prefix]]);
        assert_eq!(
            calls_to(&calls, COMPARE_PREFIX),
            vec![vec![name, prefix, 9]]
        );
        let remove = slot_target(&e, node, 0xe8);
        assert_eq!(calls_to(&calls, remove), vec![vec![node, named]]);
        // Another name stays; and the flag off never compares.
        returns(&mut e, COMPARE_PREFIX, 1);
        biped_anim_attach_skinned_object(
            &mut e,
            this,
            Ptr::new(node),
            Ptr::NULL,
            3,
            1,
            0,
            Ptr::NULL,
        );
        assert!(calls_to(&log(&mut e), remove).is_empty());
        biped_anim_attach_skinned_object(
            &mut e,
            this,
            Ptr::new(node),
            Ptr::NULL,
            3,
            0,
            0,
            Ptr::NULL,
        );
        assert!(calls_to(&log(&mut e), COMPARE_PREFIX).is_empty());
    }

    #[test]
    fn the_pip_boy_children_are_collected_by_a_name_prefix() {
        let mut e = engine();
        let (this, node) = attach_scene(&mut e);
        let named = object_with_slots(&mut e, &[(0x18, 0x1)]);
        children_are(&mut e, vec![named]);
        let name = text(&mut e, "PipBoyOff");
        returns(&mut e, NAME_TEXT, name);
        returns(&mut e, COMPARE_TEXT, 0);
        log(&mut e);
        // With the second flag: nine characters are compared with "pipboyoff".
        biped_anim_attach_skinned_object(
            &mut e,
            this,
            Ptr::new(node),
            Ptr::NULL,
            3,
            0,
            1,
            Ptr::NULL,
        );
        let calls = log(&mut e);
        let from_text = calls_to(&calls, STRING_FROM_TEXT);
        assert_eq!(from_text[0][1], name);
        assert_eq!(calls_to(&calls, FN_004AFAD0), vec![vec![from_text[0][0]]]);
        let copied = calls_to(&calls, FN_004ADD50);
        assert_eq!(copied[0][2], 9);
        let compared = calls_to(&calls, COMPARE_TEXT);
        assert_eq!(compared[0], vec![copied[0][0], PIPBOY_OFF_PREFIX]);
        let remove = slot_target(&e, node, 0xe8);
        assert_eq!(calls_to(&calls, remove), vec![vec![node, named]]);
        assert_eq!(calls_to(&calls, STRING_FREE), vec![vec![from_text[0][0]]]);
        // Without it, eight characters are compared with "pipboyon".
        biped_anim_attach_skinned_object(
            &mut e,
            this,
            Ptr::new(node),
            Ptr::NULL,
            3,
            0,
            0,
            Ptr::NULL,
        );
        let calls = log(&mut e);
        assert_eq!(calls_to(&calls, FN_004ADD50)[0][2], 8);
        assert_eq!(calls_to(&calls, COMPARE_TEXT)[0][1], PIPBOY_ON_PREFIX);
        // A name that does not match keeps the child and frees the string.
        returns(&mut e, COMPARE_TEXT, 1);
        biped_anim_attach_skinned_object(
            &mut e,
            this,
            Ptr::new(node),
            Ptr::NULL,
            3,
            0,
            0,
            Ptr::NULL,
        );
        let calls = log(&mut e);
        assert!(calls_to(&calls, remove).is_empty());
        assert_eq!(calls_to(&calls, STRING_FREE).len(), 1);
    }

    #[test]
    fn a_child_with_data_binds_its_lights_to_the_bones_and_joins_the_attached_node() {
        let mut e = engine();
        let (this, node) = attach_scene(&mut e);
        let child = object_with_slots(&mut e, &[(0x18, 0x1)]);
        children_are(&mut e, vec![child]);
        let parent = object_with_slots(&mut e, &[(0xdc, 0)]);
        let data = 0x0d0d_0000;
        returns(&mut e, NODE_DATA_POINTER, data);
        returns(&mut e, BODY_PART_OWNER, 0x77);
        returns(&mut e, BODY_PART_COUNT, 2);
        let items = block(&mut e, 8);
        e.mem.set_u32(items, 0x1010);
        e.mem.set_u32(items + 4, 0x2020);
        returns(&mut e, BODY_PART_ARRAY, items);
        // Only the first light has a name; names come from the field address.
        e.register(NAME_FIELD, |_, a| ret(a[0]));
        e.register_double(NAME_TEXT, |_, a| ret((a[0] == 0x1010) as u32));
        returns(&mut e, FN_004ADE00, 0x5151);
        e.set(this, BipedAnim::root, Ptr::new(0x1818));
        log(&mut e);
        let result = biped_anim_attach_skinned_object(
            &mut e,
            this,
            Ptr::new(node),
            Ptr::new(parent),
            3,
            0,
            0,
            Ptr::NULL,
        );
        assert_eq!(result.addr(), child);
        let calls = log(&mut e);
        assert_eq!(calls_to(&calls, BODY_PART_OWNER), vec![vec![data]]);
        assert_eq!(calls_to(&calls, BODY_PART_COUNT), vec![vec![0x77]]);
        assert_eq!(calls_to(&calls, FN_004ADE00), vec![vec![0x1818, 0x1010]]);
        assert_eq!(calls_to(&calls, FN_004ADDA0), vec![vec![data, 0, 0x5151]]);
        // The unnamed light is logged with the node's parent name (here none).
        assert_eq!(
            calls_to(&calls, LOG),
            vec![vec![UNNAMED_BONE_FORMAT, 1, NULL_TEXT, 0]]
        );
        assert_eq!(
            calls_to(&calls, SET_PARENT_OBJECT),
            vec![vec![data, parent]]
        );
        let add = slot_target(&e, parent, 0xdc);
        assert_eq!(calls_to(&calls, add), vec![vec![parent, child, 1]]);
    }

    #[test]
    fn the_extra_data_of_the_node_moves_to_the_weapon_bone_with_its_geometries() {
        let mut e = engine();
        let (this, node) = attach_scene(&mut e);
        e.set(bone(this, 1), BipedBone::pParent, Ptr::new(0x7000));
        e.set_global(EXTRA_DATA_KEY, 0x9900);
        returns(&mut e, GET_EXTRA_DATA, 0x8000);
        let geometry = object_with_slots(&mut e, &[(0x18, 0x1)]);
        children_are(&mut e, vec![geometry]);
        returns(&mut e, NODE_DATA_POINTER, 0x0d0d_0000);
        log(&mut e);
        biped_anim_attach_skinned_object(
            &mut e,
            this,
            Ptr::new(node),
            Ptr::NULL,
            3,
            0,
            0,
            Ptr::NULL,
        );
        let calls = log(&mut e);
        assert_eq!(
            calls_to(&calls, REMOVE_EXTRA_DATA),
            vec![vec![0x7000, 0x9900]]
        );
        assert_eq!(
            calls_to(&calls, ADD_EXTRA_DATA),
            vec![vec![0x7000, 0x9900, 0x8000]]
        );
        // The geometry (with data) is added to the extra data's list at +0x34.
        let added = calls_to(&calls, ADD_TO_LIST);
        assert_eq!(added[0][0], 0x8034);
    }

    #[test]
    fn a_skinned_child_of_an_npc_gets_its_face_gen_data() {
        let mut e = engine();
        let (this, node) = attach_scene(&mut e);
        let requester = 0x0abc_0000;
        e.set(this, BipedAnim::m_pRequester, Ptr::new(requester));
        let npc = 0x0ccc_0000;
        returns(&mut e, REFERENCE_FORM, npc);
        returns(&mut e, FORM_TYPE, 0x2a);
        let child = object_with_slots(&mut e, &[(0x18, 0x1), (0xe4, 0)]);
        children_are(&mut e, vec![child]);
        returns(&mut e, NODE_SKIN_DATA, 0x5a5a);
        returns(&mut e, NODE_DATA_POINTER, 0x0d0d_0000);
        returns(&mut e, NODE_DATA_FIELD, 0x0e0e);
        returns(&mut e, NI_OBJECT_CLONE_OV2, 0x0f0f);
        returns(&mut e, SETTING_BYTE_VALUE, 1);
        // The face model comes from the caller.
        log(&mut e);
        biped_anim_attach_skinned_object(
            &mut e,
            this,
            Ptr::new(node),
            Ptr::NULL,
            3,
            0,
            0,
            Ptr::new(0x4141),
        );
        let calls = log(&mut e);
        let coords = calls_to(&calls, GET_FACE_COORD);
        assert_eq!(coords[0][0], npc);
        assert_eq!(
            calls_to(&calls, APPLY_FACE_COORDS),
            vec![vec![0x4141, coords[0][1], child, 0]]
        );
        // The skin and the property data were copied; the property clone and
        // the copy were handed to 004adde0 and 004addc0.
        let copies = calls_to(&calls, NI_OBJECT_DEEP_COPY);
        assert_eq!((copies[0][0], copies[1][0]), (0x5a5a, 0x0e0e));
        assert_eq!(calls_to(&calls, FN_004ADDE0), vec![vec![child, 0x0f0f]]);
        let kept = calls_to(&calls, FN_004ADDC0);
        assert_eq!(kept[0][0], 0x0d0d_0000);
        let set_skin = slot_target(&e, child, 0xe4);
        assert_eq!(calls_to(&calls, set_skin).len(), 1);
        assert_eq!(
            calls_to(&calls, VECTOR_DESTRUCT),
            vec![vec![coords[0][1], 0x20, 4, FACE_COORD_FREE]]
        );
    }

    #[test]
    fn without_a_face_model_the_slots_own_face_gen_model_is_used_and_no_npc_skips_it() {
        let mut e = engine();
        let (this, node) = attach_scene(&mut e);
        let requester = 0x0abc_0000;
        e.set(this, BipedAnim::m_pRequester, Ptr::new(requester));
        returns(&mut e, FN_004AE8A0, 1);
        returns(&mut e, FN_004AE790, 0x4a4a);
        returns(&mut e, REFERENCE_FORM, 0x0ccc_0000);
        // The base form is not an NPC (type 0x2b): no face data is made.
        returns(&mut e, FORM_TYPE, 0x2b);
        let child = object_with_slots(&mut e, &[(0x18, 0x1)]);
        children_are(&mut e, vec![child]);
        returns(&mut e, NODE_SKIN_DATA, 0x5a5a);
        returns(&mut e, NODE_DATA_POINTER, 0x0d0d_0000);
        log(&mut e);
        biped_anim_attach_skinned_object(
            &mut e,
            this,
            Ptr::new(node),
            Ptr::NULL,
            3,
            0,
            0,
            Ptr::NULL,
        );
        let calls = log(&mut e);
        assert_eq!(calls_to(&calls, FN_004AE790), vec![vec![this.addr(), 3]]);
        assert!(calls_to(&calls, GET_FACE_COORD).is_empty());
    }
}
