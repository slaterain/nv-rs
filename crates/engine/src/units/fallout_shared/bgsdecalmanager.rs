//! `fallout shared/bgsdecalmanager.cpp` (Xbox PDB source unit), subsystem `fallout shared`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! The decal manager (`BGSDecalManager`, one instance) turns impact decals
//! into `BSTempEffectSimpleDecal`s: `AddDecal` walks the scene graph of the
//! target with `AddGeometryDecalRecurse`, which creates the temp effects and
//! queues them on the manager's pending list; `UpdateSimpleDecals` then
//! runs an occlusion query for each one (`IssueDecalOcclusionQuery`) and
//! places or discards it. The unit also holds the cleanup of the manager's
//! emitter list, a few `NiPoint3` and `NiMatrix3` helpers the compiler
//! emitted here, and the `BSCullingProcess` constructor and destructor the
//! occlusion query uses.
//!
//! Progress: the first 40 of the unit's open functions are translated
//! (`004a0030` to `004a1ff0`); the next session continues at `004a2020`.
//! Callees at later addresses of this unit (`004a2020`, `004a2070`,
//! `004a2cf0`, `004a2d50`, `004a4460`, `004a4500`, `004a47b0`) are called
//! by address until they are translated; replace those `e.call`s with
//! direct calls then.
//!
//! Notes for the next session:
//! - `BSTempEffectSimpleDecal` belongs to `bstempeffectsimpledecal.cpp`;
//!   its fields are read at the offsets named by the `DECAL_*` constants
//!   (Xbox PDB names, offsets checked against the PC code).
//! - `DecalPlacement` is the description `AddDecal` is given; the Xbox PDB
//!   dump has no type for it, so its field names describe what the code
//!   does with them.
//! - x87: the game computes in extended precision and stores `float`s. The
//!   translations compute in `f64` and round to `f32` at each store.
//! - The compiler's exception-unwinding frames (`__CxxFrameHandler`) and
//!   the stack-cookie check are not translated.

#[allow(unused_imports)]
use crate::prelude::*;
use crate::types::{NiPoint3, NiTPointerList};

// ---------------------------------------------------------------------
// Layouts

layout! {
    /// `BGSDecalManager` (Xbox PDB), 0x28 bytes. One instance (`pInstance`).
    pub struct BGSDecalManager: 0x28 {
        /// `spQueryTexture` (Xbox PDB): `NiPointer<BSRenderedTexture>`.
        0x00 spQueryTexture: Ptr,
        /// `bClearQueryTexture` (Xbox PDB).
        0x04 bClearQueryTexture: bool,
        /// `PendingSimpleDecalList` (Xbox PDB):
        /// `NiTPointerList<NiPointer<BSTempEffectSimpleDecal>>`.
        0x08 PendingSimpleDecalList: Inline<NiTPointerList>,
        /// `DecalEmitterList` (Xbox PDB): `NiTPointerList<BGSDecalEmitter*>`.
        0x14 DecalEmitterList: Inline<NiTPointerList>,
        /// `spQueryAccum` (Xbox PDB): `NiPointer<BSShaderAccumulator>`.
        0x20 spQueryAccum: Ptr,
        /// `spQueryCamera` (Xbox PDB): `NiPointer<NiCamera>`.
        0x24 spQueryCamera: Ptr,
    }

    /// The decal description `AddDecal` is given (no type for it in the
    /// Xbox PDB dump; the size is just past the last field read, not the
    /// real size). The names describe what the code does with each field.
    pub struct DecalPlacement: 0x78 {
        /// The point the decal is placed at (a `NiPoint3`).
        0x00 origin: Inline<NiPoint3>,
        /// The direction of the decal (a `NiPoint3`; the projected path
        /// normalizes it).
        0x0C direction: Inline<NiPoint3>,
        /// An object whose virtual at `+0x1d0` returns the node a type 2
        /// decal is projected onto.
        0x24 source_object: Ptr,
        /// The object (scene graph node) the decal is placed on.
        0x28 target: Ptr,
        /// A size; the larger of this and `size_b` is the decal's reach.
        0x38 size_a: f32,
        /// The other size.
        0x3C size_b: f32,
        /// An object passed to the decal caster (texture set lookup);
        /// when non-null the new decal is also queued on the pending list.
        0x48 object_48: Ptr,
        /// A float the reach test divides by; the test only runs when it
        /// is positive.
        0x5C value_5c: f32,
        /// Set by `AddDecal` for type 4 before it adds the geometry decal.
        0x71 flag_71: u8,
        /// When set (and the occlusion setting is on) the new decal gets
        /// an occlusion query.
        0x72 occlusion_query_wanted: u8,
        /// When set, type 1 skips its distance limit.
        0x77 skip_distance_limit: u8,
    }
}

// `BSTempEffectSimpleDecal` fields (Xbox PDB names, PC offsets).
/// `bFinished`.
const DECAL_FINISHED: u32 = 0x18;
/// `bFinalize`.
const DECAL_FINALIZE: u32 = 0x19;
/// `bOcclusionQuery`.
const DECAL_QUERY_PENDING: u32 = 0x1a;
/// `bClearQueryTexture`.
const DECAL_CLEAR_QUERY_TEXTURE: u32 = 0x1b;
/// `pOcclusionQuery`.
const DECAL_QUERY: u32 = 0x1c;
/// `iCurrentOcclusionResult`.
const DECAL_QUERY_RESULT: u32 = 0x24;
/// `bValidDecal`.
const DECAL_VALID: u32 = 0x2d;
/// `pOrigin` (a `NiPoint3`).
const DECAL_ORIGIN: u32 = 0x34;
/// `pVector` (a `NiPoint3`).
const DECAL_VECTOR: u32 = 0x40;
/// `spTargetNode` (a `NiPointer`).
const DECAL_TARGET_NODE: u32 = 0x10c;
/// `fHeight`.
const DECAL_HEIGHT: u32 = 0x11c;
/// `fDepth`.
const DECAL_DEPTH: u32 = 0x120;
/// Size of a `BSTempEffectSimpleDecal`.
const DECAL_SIZE: u32 = 0x144;
/// Slot of the decal's virtual that places it (`Initialize`, Xbox PDB).
const DECAL_VSLOT_INITIALIZE: u32 = 0x8c;

// ---------------------------------------------------------------------
// Callees outside the translated part of this file

/// The first dword of the object at `this` (`NiPointer::operator*`, the
/// head of a list, ...: the linker folded the identical bodies).
const FIRST_WORD: u32 = 0x0055_9450;
/// The dword at `this + 8`: the item count of an `NiTPointerList`.
const LIST_COUNT: u32 = 0x0044_ddc0;
/// Advances an `NiTPointerList` iterator (`this` = the list, then the
/// address of the node pointer) and returns the address of the item slot
/// it left.
const LIST_ADVANCE: u32 = 0x0057_cbe0;
/// Appends an item (the address of the item slot) to the list.
const LIST_ADD_TAIL: u32 = 0x0057_c590;
/// Adds an item (the address of the item slot) at the head of the list.
const LIST_ADD_HEAD: u32 = 0x0076_b660;
/// Removes the node an iterator points at from the emitter list (`this` =
/// the list, then the address of the node pointer).
const EMITTER_LIST_REMOVE: u32 = 0x0049_f590;
/// Removes the node an iterator points at from the pending-decal list
/// (`this` = the list, then the address of a result slot, then the address
/// of the node pointer). Later in this unit.
const PENDING_LIST_REMOVE: u32 = 0x004a_4500;
/// Whether the list holds the item (`this` = the list, then the address
/// of the item, then 0).
const LIST_CONTAINS: u32 = 0x0049_c680;
/// Empties the list.
const LIST_REMOVE_ALL: u32 = 0x004e_d900;
/// Constructs an empty `NiTPointerList`.
const LIST_CONSTRUCTOR: u32 = 0x0048_f200;
/// Destructor body of the local list `AddDecal` builds. Later in this
/// unit.
const LIST_LOCAL_DESTRUCTOR_BODY: u32 = 0x004a_47b0;
/// `NiPointer` release (`this` = the pointer slot).
const NI_POINTER_RELEASE: u32 = 0x0045_cec0;
/// `NiPointer` assign (`this` = the slot, then the new pointer).
const NI_POINTER_ASSIGN: u32 = 0x0066_b0d0;
/// `NiPointer` constructor (`this` = the slot, then the pointer).
const NI_POINTER_INIT: u32 = 0x0063_3c90;
/// `operator delete` (cdecl, one argument).
const FREE: u32 = 0x0040_1030;
/// Allocator (cdecl, size).
const ALLOCATE: u32 = 0x0040_1000;
/// Allocator for the decal temp effect (cdecl, size).
const ALLOCATE_OBJECT: u32 = 0x00aa_13e0;
/// Returns the pointer to a setting's value (`this` = the setting).
const SETTING_VALUE_POINTER: u32 = 0x0040_8d60;
/// The same for an integer setting.
const SETTING_INT_POINTER: u32 = 0x0043_d4d0;
/// A float setting's value, returned in `ST0` (`this` = the setting).
const SETTING_FLOAT_VALUE: u32 = 0x0045_0410;
/// Log line (cdecl `printf`-style: format, arguments).
const LOG: u32 = 0x005b_5e40;
/// `sprintf_s`-style formatter (cdecl: buffer, size, format, arguments).
const FORMAT: u32 = 0x0040_6d00;
/// Error report (cdecl: text, 0).
const REPORT: u32 = 0x0040_fbe0;
/// `strncmp` (cdecl).
const STRNCMP: u32 = 0x00ec_8a19;
/// `QueryPerformanceFrequency` import slot.
const QUERY_PERFORMANCE_FREQUENCY: u32 = 0x00fd_f0a4;
/// `QueryPerformanceCounter` import slot.
const QUERY_PERFORMANCE_COUNTER: u32 = 0x00fd_f0a0;
/// A function returning `this`: the compiler's empty `NiPoint3` and
/// `NiMatrix3` default constructors, folded into one body.
const EMPTY_CONSTRUCTOR: u32 = 0x0068_15c0;
/// `NiPoint3::NiPoint3(x, y, z)` (`this`, three floats).
const POINT3_CONSTRUCTOR: u32 = 0x0041_6870;
/// `NiPoint3::NiPoint3(x, y, z)` as the compiler emitted it a second
/// time, used for the zero point.
const POINT3_CONSTRUCTOR_ZERO: u32 = 0x0043_d410;
/// `sin` and `cos` of an angle (cdecl: angle, address of sin, address of
/// cos).
const SIN_COS: u32 = 0x0041_69a0;
/// Scales the vector in place by a float (`this`, scalar).
const POINT3_SCALE_IN_PLACE: u32 = 0x0043_9180;
/// `NiPoint3 + NiPoint3` (`this`, result, other): returns the result.
const POINT3_ADD: u32 = 0x0043_9e90;
/// `NiPoint3 - NiPoint3` (`this`, result, other): returns the result.
const POINT3_SUBTRACT: u32 = 0x0043_9ef0;
/// `NiPoint3 * float` (`this`, result, scalar): returns the result.
const POINT3_MULTIPLY: u32 = 0x0045_bb20;
/// The length of the vector at `this`, in `ST0`.
const POINT3_LENGTH: u32 = 0x0045_7990;
/// `NiMatrix3::operator*` (Xbox PDB `operatorP`; `this`, result, other):
/// returns the result.
const MATRIX_MULTIPLY: u32 = 0x0043_f8d0;
/// Reads column `index` of a `NiMatrix3` into the vector (`this`, index,
/// vector).
const MATRIX_GET_COLUMN: u32 = 0x0047_6930;
/// Writes column `index` of a `NiMatrix3` from the vector (`this`,
/// index, vector).
const MATRIX_SET_COLUMN: u32 = 0x0047_69c0;
/// `BSUtilities::GetRotationZToVector` (Xbox PDB name; cdecl: matrix, then
/// the vector by value).
const ROTATION_Z_TO_VECTOR: u32 = 0x00c4_b600;
/// The camera's world direction (`this` = camera, result): returns it.
const CAMERA_DIRECTION: u32 = 0x0045_bba0;
/// Sets the camera's translation (`this`, vector).
const SET_TRANSLATE: u32 = 0x0044_0460;
/// Sets the camera's rotation (`this`, matrix).
const SET_ROTATE: u32 = 0x0043_fa80;
/// `NiCamera::SetViewFrustum` (Xbox PDB name; `this`, frustum).
const SET_VIEW_FRUSTUM: u32 = 0x00a6_faf0;
/// `NiFrustum::NiFrustum` (Xbox PDB name; `this`, one argument).
const FRUSTUM_CONSTRUCTOR: u32 = 0x00a7_1b70;
/// A call on the camera taking the address of a zeroed 12-byte object.
const UPDATE_WITH_ZERO: u32 = 0x00a5_9c60;
/// `NiColorA` constructor (`this`, r, g, b, a): returns `this`.
const COLOR_CONSTRUCTOR: u32 = 0x0041_4430;
/// Creates a debug line object (cdecl: start, start colour, end, end
/// colour, 1): returns the object.
const MAKE_DEBUG_LINE: u32 = 0x004b_3890;
/// `TES::AddTempDebugObject` (Xbox PDB name; `this`, object, seconds).
const ADD_TEMP_DEBUG_OBJECT: u32 = 0x0045_8e20;
/// The global `TES` object.
const TES_OBJECT: u32 = 0x011d_ea10;
/// A global object whose virtual at [`CAMERA_POSITION_SLOT`] returns the
/// address of the player's camera position (a `NiPoint3`).
const CAMERA_POSITION_OWNER: u32 = 0x011d_ea3c;
/// Slot of that virtual.
const CAMERA_POSITION_SLOT: u32 = 0x1f4;
/// The name holder of a node (`this` = the node).
const NODE_NAME_HOLDER: u32 = 0x0041_3f40;
/// The text of a name holder.
const NAME_TEXT: u32 = 0x0043_b1b0;
/// `BSTempEffectSimpleDecal::CreateDebugBox` (Xbox PDB name; `this`,
/// colour, solid flag).
const CREATE_DEBUG_BOX: u32 = 0x0068_dea0;
/// `BSTempEffectSimpleDecal::FinalizeGeometry` (Xbox PDB name).
const FINALIZE_GEOMETRY: u32 = 0x0068_be90;
/// Whether the decal's occlusion query has finished.
const QUERY_FINISHED: u32 = 0x0068_de30;
/// `BSTempEffectSimpleDecal` constructor (`this`, geometry node, lifetime,
/// placement).
const DECAL_CONSTRUCTOR: u32 = 0x0068_ad20;
/// Returns the global object `0x011f4748`.
const GET_GLOBAL_OBJECT: u32 = 0x0043_c4b0;
/// The global `fn_004a0e90` reads the first word of.
const QUERY_TEXTURE_FORMAT: u32 = 0x011f_9508;
/// The global `fn_004a0ea0` returns (the object that creates the query
/// texture).
const TEXTURE_MANAGER: u32 = 0x011f_91a8;
/// Creates the query texture (`this` = the texture manager, 5 arguments).
const CREATE_QUERY_TEXTURE: u32 = 0x00b6_e110;
/// `BSRenderedTexture::StopOffscreen` (Xbox PDB name; `this` = texture).
const STOP_OFFSCREEN: u32 = 0x00b6_b260;
/// Starts rendering to a target (cdecl: target index, the value
/// `StopOffscreen` returned).
const START_OFFSCREEN: u32 = 0x00b6_b8d0;
/// `BSOcclusionQuery` constructor.
const QUERY_CONSTRUCTOR: u32 = 0x00c4_ecf0;
/// `BSOcclusionQuery::Begin`.
const QUERY_BEGIN: u32 = 0x00c4_ed70;
/// `BSOcclusionQuery::End` (Xbox PDB name).
const QUERY_END: u32 = 0x00c4_edb0;
/// `BSShaderUtil::AccumulateScene` (Xbox PDB name; cdecl: camera, node,
/// culling process).
const ACCUMULATE_SCENE: u32 = 0x00b6_bee0;
/// Renders the accumulated scene (cdecl: camera, accumulator, 0).
const RENDER_ACCUMULATED: u32 = 0x00b6_c0d0;
/// Ends the offscreen render.
const END_OFFSCREEN: u32 = 0x00b6_b790;
/// The accumulator's mode (cdecl, one argument).
const ACCUMULATOR_MODE: u32 = 0x0045_0b80;
/// The `NiBound` of an object, or a default one (`this` = the object).
const GET_WORLD_BOUND: u32 = 0x0043_d450;
/// The radius of a `NiBound` (`this` = bound), in `ST0`.
const BOUND_RADIUS: u32 = 0x0084_d030;
/// The scale of a node (`this` = the node), in `ST0`.
const TARGET_SCALE: u32 = 0x008d_01e0;
/// The rotation angle of a decal, in `ST0`.
const DECAL_ROTATION: u32 = 0x004e_3d00;
/// The width of a decal (`+0x118`), in `ST0`.
const DECAL_WIDTH_GETTER: u32 = 0x0050_7b20;
/// `max(a, b)` for floats (cdecl; ties and unordered give `b`), in `ST0`.
const FLOAT_MAX: u32 = 0x0040_4010;
/// Creates the decal caster (cdecl: radius, 1): returns the caster.
const DECAL_CASTER_CREATE: u32 = 0x0062_2a70;
/// Casts a probe (`this` = the decal caster, start, end, radius): returns
/// the number of hits.
const DECAL_CASTER_CAST: u32 = 0x0062_2f80;
/// The object hit by hit `index` (`this` = the decal caster, index).
const DECAL_CASTER_HIT_OBJECT: u32 = 0x0062_3290;
/// Copies hit `index`'s position and normal out (`this` = the decal
/// caster, index, position, normal).
const DECAL_CASTER_HIT_DATA: u32 = 0x0062_3370;
/// Sets the decal caster's texture set (`this`, set).
const DECAL_CASTER_SET_TEXTURE_SET: u32 = 0x0062_31a0;
/// The texture set of an object (`this` = the object).
const OBJECT_TEXTURE_SET: u32 = 0x0045_43c0;
/// `TESObjectREFR::FindReferenceFor3D` (cdecl, one argument).
const FIND_REFERENCE_FOR_3D: u32 = 0x0056_f930;
/// `NiNode::GetChildCount` (`this` = the node or list).
const CHILD_COUNT: u32 = 0x0043_b480;
/// `NiNode::GetAt` (`this`, index).
const CHILD_AT: u32 = 0x0043_b4a0;
/// Profiling scope constructor (`this` = 4-byte guard; 7, 1, file name,
/// line).
const SCOPE_BEGIN: u32 = 0x0040_4eb0;
/// Profiling scope destructor.
const SCOPE_END: u32 = 0x0040_4ee0;
/// The emitter destructor body (`BGSDecalEmitter`). Later in this unit.
const EMITTER_DESTRUCTOR_BODY: u32 = 0x004a_2cf0;
/// Per-emitter update. Later in this unit.
const EMITTER_UPDATE: u32 = 0x004a_2d50;
/// Whether the emitter has finished.
const EMITTER_FINISHED: u32 = 0x004f_1540;
/// `AddProjectedDecalRecurse` (Xbox PDB name). Later in this unit.
const ADD_PROJECTED_DECAL_RECURSE: u32 = 0x004a_2070;
/// A test on the model object, taking a tag number (`this`, 0x1a). Later
/// in this unit.
const MODEL_HAS_TAG: u32 = 0x004a_2020;
/// Whether an entry of a bound list is the end marker. Later in this unit.
const BOUND_IS_END: u32 = 0x004a_4460;
/// The next entry of a bound list.
const BOUND_NEXT: u32 = 0x0072_6070;
/// Whether the node is a model node (cdecl: the filter object, the node).
const IS_MODEL_NODE: u32 = 0x0043_b300;
/// Whether the node is hidden (`this` = the node).
const NODE_IS_HIDDEN: u32 = 0x0045_6610;
/// The model data of a node (`this` = the node).
const NODE_MODEL_DATA: u32 = 0x0050_d100;
/// The model object of model data (`this` = the data).
const MODEL_DATA_OBJECT: u32 = 0x0043_b230;
/// The geometry list of a node (`this` = the node).
const NODE_GEOMETRY_LIST: u32 = 0x0096_11e0;
/// Whether the geometry is accepted (cdecl: the filter, the geometry).
const GEOMETRY_ACCEPTED: u32 = 0x0045_bad0;
/// A flag test on an object (`this`, mask): `(flags & mask) != 0`.
const FLAG_TEST: u32 = 0x0045_6630;
/// The base form of a reference (`this` = the reference): `+0x20`.
const REFERENCE_BASE_FORM: u32 = 0x007a_f430;
/// The form type of a form (`this` = the form): the byte at `+4`.
const FORM_TYPE: u32 = 0x0040_1170;
/// Collision-filter call `fn_004a1a10` forwards to (cdecl: 0x27, a, b).
const COLLISION_CALL: u32 = 0x00c8_27f0;
/// Lazily created singleton `fn_004a1a50` asks for (no parameters; the
/// caller still pushes one word).
const SINGLETON_GETTER: u32 = 0x0049_fef0;
/// Called on the singleton's `+0x14`.
const SINGLETON_RELEASE: u32 = 0x004e_d8c0;
/// Lock helpers of the object `0043c4b0` returns.
const LOCK_ENTER: u32 = 0x0082_f1b0;
const LOCK_LEAVE: u32 = 0x0082_f1f0;
const LOCK_STATE: u32 = 0x00ac_bb70;
const LOCK_EXTRA: u32 = 0x0048_3710;
/// `BSCullingProcess` base constructor (`this`, one argument) and base
/// destructor.
const CULLING_BASE_CONSTRUCTOR: u32 = 0x00a6_9400;
const CULLING_BASE_DESTRUCTOR: u32 = 0x00a6_93e0;
/// The object whose virtual at `+0xa8` `fn_004a1ff0` calls (`this` = the
/// node).
const OBJECT_OF_NODE: u32 = 0x0054_95f0;

// ---------------------------------------------------------------------
// Globals and constants of the exe

/// `BSCullingProcess` vtable.
const CULLING_PROCESS_VTABLE: u32 = 0x0101_e2ec;
/// `BSCullingProcess` RTTI (`GetRTTI`'s result).
const CULLING_PROCESS_RTTI: u32 = 0x0120_30a8;
/// `iDecalDebugFlags` (Xbox PDB static): bit 0 wireframe box, bit 1 solid
/// alpha box, bit 2 occlusion query camera direction, bit 4 display
/// failed.
const DECAL_DEBUG_FLAGS: u32 = 0x011c_57f4;
/// The counter printed by "Placing Simple Decal #%d" (the Xbox PDB static
/// `iDecalCount`, matched by use).
const DECAL_COUNT: u32 = 0x011c_57ec;
/// `iDecalsThisFrame` (Xbox PDB static, matched by use).
const DECALS_THIS_FRAME: u32 = 0x011c_57e4;
/// `iSkinnedDecalCount` (Xbox PDB static, matched by use).
const SKINNED_DECAL_COUNT: u32 = 0x011c_57f0;
/// `pDecalCaster` (Xbox PDB static).
const DECAL_CASTER: u32 = 0x011c_57e0;
/// A setting whose byte switches the `DECAL:` log lines on (probably
/// `bDebugDecals`).
const SETTING_DEBUG_LOG: u32 = 0x011c_5904;
/// A setting whose byte switches the timing of type 1 on (probably
/// `bProfileDecals`).
const SETTING_PROFILE: u32 = 0x011c_5884;
/// A setting whose byte enables the geometry decals (probably `bDecals`).
const SETTING_DECALS: u32 = 0x011c_5834;
/// A setting whose byte enables the projected decals (probably
/// `bSkinnedDecals`).
const SETTING_SKINNED_DECALS: u32 = 0x011c_58bc;
/// A setting whose byte enables the occlusion query (probably
/// `bDecalOcclusionQuery`).
const SETTING_OCCLUSION_QUERY: u32 = 0x011c_58a8;
/// Integer setting: the maximum of non-skinned decals per frame (probably
/// `iMaxDecalsPerFrame`).
const SETTING_MAX_DECALS_PER_FRAME: u32 = 0x011c_5890;
/// Integer setting: the maximum number of skinned decals (probably
/// `iMaxSkinDecals`).
const SETTING_MAX_SKIN_DECALS: u32 = 0x011c_58e4;
/// Float setting: the decal lifetime (probably `fDecalLifetime`).
const SETTING_DECAL_LIFETIME: u32 = 0x011c_5874;
/// Float setting: the distance limit of type 1.
const SETTING_GEOMETRY_DISTANCE: u32 = 0x011c_77a8;
/// Float setting: the distance limit of type 2.
const SETTING_PROJECTED_DISTANCE: u32 = 0x011c_7588;
/// The filter object `GEOMETRY_ACCEPTED` tests against.
const GEOMETRY_FILTER: u32 = 0x011c_7d34;
/// The object `IS_MODEL_NODE` tests against.
const MODEL_FILTER: u32 = 0x011f_4a20;
/// A global `NiPoint3` the fallback probe of type 2 is aimed along
/// (negated).
const FALLBACK_VECTOR: u32 = 0x011a_9484;
/// The flag bit `fn_004a19d0` tests.
const FLAG_HIDDEN: u32 = 0x400;
/// 0.25 (float): the alpha of the translucent debug box.
const DEBUG_BOX_ALPHA: u32 = 0x0101_622c;
/// 2.0 (double).
const TWO: u32 = 0x0101_1590;
/// 50.0 (double): the margin added to the decal depth for the far plane.
const FAR_MARGIN: u32 = 0x0101_e2c0;
/// 0.1 (float): the near plane.
const NEAR_PLANE: u32 = 0x0101_e2bc;
/// 50.0 (float): the factor the decal direction is scaled by.
const DIRECTION_SCALE: u32 = 0x0101_b268;
/// 30.0 (float): seconds the debug line stays.
const DEBUG_LINE_SECONDS: u32 = 0x0101_8f5c;
/// 0.0 (double).
const ZERO: u32 = 0x0101_2060;
/// 1000.0 (double) and 1e-6 (double): the tick scale defaults.
const THOUSAND: u32 = 0x0101_7b70;
const MICROSECOND: u32 = 0x0101_e3d0;
/// The length (a `double`) up to which `fn_004a0c10` zeroes the vector.
const MINIMUM_LENGTH: u32 = 0x0101_7cf8;
/// 80.0 and 32.0 (floats): the decal caster's probe length and radius.
const PROBE_LENGTH: u32 = 0x0101_e33c;
const PROBE_RADIUS: u32 = 0x0101_e340;
/// 2.0 (float): the length of the fallback probe's shift.
const FALLBACK_SCALE: u32 = 0x0101_62c0;
/// Text constants.
const TEXT_QUERY_FAILED: u32 = 0x0101_e19c;
const TEXT_QUERY_SUCCEEDED: u32 = 0x0101_e1c8;
const TEXT_PLACING: u32 = 0x0101_e174;
const TEXT_ISSUING: u32 = 0x0101_e298;
const TEXT_ORIGIN: u32 = 0x0101_e274;
const TEXT_DIRECTION: u32 = 0x0101_e24c;
const TEXT_FRUSTUM: u32 = 0x0101_e1f8;
const TEXT_INSTANTIATED: u32 = 0x0101_e3a8;
const TEXT_TICKS: u32 = 0x0101_e370;
const TEXT_SKIN_LIMIT: u32 = 0x0101_e344;
const TEXT_FRAME_COUNT: u32 = 0x0101_e420;
const TEXT_FRAME_LIMIT: u32 = 0x0101_e490;
const TEXT_SOURCE_FILE: u32 = 0x0101_e3d8;
const TEXT_SET_SCREEN_SPACE_CAMERA_DATA: u32 = 0x0101_e2c8;
/// Node-name prefixes `AddGeometryDecalRecurse` never puts a decal on:
/// (address of the text, length compared).
const SKIPPED_NAME_PREFIXES: [(u32, u32); 6] = [
    (0x0101_e484, 10), // "Decal Node"
    (0x0101_e47c, 5),  // "Decal"
    (0x0101_e474, 7),  // "FaceGen"
    (0x0101_e468, 9),  // "BSFaceGen"
    (0x0101_e460, 5),  // "Bip01"
    (0x0101_e450, 15), // "Debug Decal Box"
];
/// Source line passed to the profiling scope of `AddDecal`.
const ADD_DECAL_SOURCE_LINE: u32 = 0x1b8;

// ---------------------------------------------------------------------
// Helpers

/// The byte a setting object's value pointer points at.
fn setting_flag(e: &mut Engine, setting: u32) -> bool {
    let value = e.call(SETTING_VALUE_POINTER, &args![setting]).u32();
    e.mem.u8(value) != 0
}

/// The integer a setting object's value pointer points at.
fn setting_int(e: &mut Engine, setting: u32) -> i32 {
    let value = e.call(SETTING_INT_POINTER, &args![setting]).u32();
    e.mem.i32(value)
}

/// Copies `words` dwords.
fn copy_words(e: &mut Engine, to: u32, from: u32, words: u32) {
    for i in 0..words {
        let value = e.mem.u32(from + 4 * i);
        e.mem.set_u32(to + 4 * i, value);
    }
}

/// `printf`-style log call whose arguments are floats passed as `double`s.
fn log_floats(e: &mut Engine, format: u32, values: &[f32]) {
    let mut words = vec![format];
    for value in values {
        (*value as f64).put(&mut words);
    }
    e.call(LOG, &words);
}

/// The name text of a decal's target node, as the log lines print it.
fn target_name(e: &mut Engine, decal: Ptr) -> u32 {
    let node = fn_004a0330(e, decal);
    let holder = e.call(NODE_NAME_HOLDER, &args![node]).u32();
    e.call(NAME_TEXT, &args![holder]).u32()
}

/// Copies the player camera's position (the `NiPoint3` the camera object's
/// virtual at `+0x1f4` returns) to `to`.
fn load_camera_position(e: &mut Engine, to: u32) {
    let owner = e.global::<u32>(CAMERA_POSITION_OWNER);
    let source = e.vcall(owner, CAMERA_POSITION_SLOT, &args![]).u32();
    copy_words(e, to, source, 3);
}

// ---------------------------------------------------------------------
// Translations

// Translated from 004a0030 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BGSDecalManager::UpdateSimpleDecals` (Xbox PDB): for every decal on the
/// pending list, issues its occlusion query if one is wanted, places it
/// (or marks it finished when the query failed) and removes it from the
/// list. Logs when the debug setting is on and draws the debug boxes the
/// `iDecalDebugFlags` ask for.
pub fn bgs_decal_manager_update_simple_decals(e: &mut Engine, this: Ptr<BGSDecalManager>) {
    let pending = this.at(BGSDecalManager::PendingSimpleDecalList);
    if e.call(LIST_COUNT, &args![pending]).u32() == 0 {
        return;
    }
    let object = e.call(GET_GLOBAL_OBJECT, &args![]).ptr::<()>();
    fn_004a0370(e, object);
    e.with_stack(0x40, |e, frame| {
        // The iterator, the copy of it before it advances, the result slot
        // of the list removal, and the colour of the debug boxes.
        let iterator = frame.addr();
        let node = iterator + 4;
        let removed = iterator + 8;
        let colour = iterator + 0x10;
        let first = e.call(FIRST_WORD, &args![pending]).u32();
        e.mem.set_u32(iterator, first);
        while e.mem.u32(iterator) != 0 {
            let current = e.mem.u32(iterator);
            e.mem.set_u32(node, current);
            let slot = e.call(LIST_ADVANCE, &args![pending, iterator]).u32();
            let decal = e.call(FIRST_WORD, &args![slot]).u32();
            if decal == 0 {
                continue;
            }
            let decal_ptr = Ptr::<()>::new(decal);
            if e.mem.u8(decal + DECAL_QUERY_PENDING) != 0 {
                bgs_decal_manager_issue_decal_occlusion_query(e, this, decal_ptr);
                e.mem.set_u8(decal + DECAL_QUERY_PENDING, 0);
            }
            if e.mem.u32(decal + DECAL_QUERY) == 0 {
                e.vcall(decal, DECAL_VSLOT_INITIALIZE, &args![]);
            } else if e.call(QUERY_FINISHED, &args![decal]).bool() {
                if e.mem.u32(decal + DECAL_QUERY_RESULT) > 0 {
                    if setting_flag(e, SETTING_DEBUG_LOG) {
                        let name = target_name(e, decal_ptr);
                        e.call(LOG, &args![TEXT_QUERY_SUCCEEDED, name]);
                    }
                    e.vcall(decal, DECAL_VSLOT_INITIALIZE, &args![]);
                } else {
                    if setting_flag(e, SETTING_DEBUG_LOG) {
                        let name = target_name(e, decal_ptr);
                        e.call(LOG, &args![TEXT_QUERY_FAILED, name]);
                    }
                    e.mem.set_u8(decal + DECAL_FINISHED, 1);
                }
            }
            if e.mem.u8(decal + DECAL_FINALIZE) != 0 && e.mem.u8(decal + DECAL_FINISHED) == 0 {
                e.call(FINALIZE_GEOMETRY, &args![decal]);
            }
            if e.mem.u8(decal + DECAL_FINISHED) != 0 {
                if fn_004a0350(e, decal_ptr) {
                    let count = e.global::<u32>(DECAL_COUNT).wrapping_add(1);
                    e.set_global(DECAL_COUNT, count);
                    if setting_flag(e, SETTING_DEBUG_LOG) {
                        let name = target_name(e, decal_ptr);
                        e.call(LOG, &args![TEXT_PLACING, count, name]);
                    }
                    if e.global::<u32>(DECAL_DEBUG_FLAGS) & 1 != 0 {
                        debug_box(e, decal, colour, [0.0, 1.0, 0.0, 1.0], true);
                    }
                    if e.global::<u32>(DECAL_DEBUG_FLAGS) & 2 != 0 {
                        let alpha = e.global::<f32>(DEBUG_BOX_ALPHA);
                        debug_box(e, decal, colour, [0.0, 1.0, 0.0, alpha], false);
                    }
                } else if e.global::<u32>(DECAL_DEBUG_FLAGS) & 0x10 != 0 {
                    if e.global::<u32>(DECAL_DEBUG_FLAGS) & 1 != 0 {
                        debug_box(e, decal, colour, [1.0, 0.0, 0.0, 1.0], true);
                    }
                    if e.global::<u32>(DECAL_DEBUG_FLAGS) & 2 != 0 {
                        let alpha = e.global::<f32>(DEBUG_BOX_ALPHA);
                        debug_box(e, decal, colour, [1.0, 0.0, 0.0, alpha], false);
                    }
                }
                e.call(PENDING_LIST_REMOVE, &args![pending, removed, node]);
                e.call(NI_POINTER_RELEASE, &args![removed]);
            }
        }
    });
    let object = e.call(GET_GLOBAL_OBJECT, &args![]).ptr::<()>();
    fn_004a03c0(e, object);
}

/// Builds a colour in `slot` and draws the decal's debug box with it.
fn debug_box(e: &mut Engine, decal: u32, slot: u32, rgba: [f32; 4], solid: bool) {
    e.call(
        COLOR_CONSTRUCTOR,
        &args![slot, rgba[0], rgba[1], rgba[2], rgba[3]],
    );
    e.call(CREATE_DEBUG_BOX, &args![decal, slot, solid as u32]);
}

// Translated from 004a0330 (decompiled, FalloutNV.exe 1.4.0.525)
/// The decal's target node: the pointer held by the `NiPointer` at
/// `+0x10c` (`spTargetNode`, Xbox PDB), read through `FIRST_WORD`.
pub fn fn_004a0330(e: &mut Engine, this: Ptr) -> u32 {
    e.call(FIRST_WORD, &args![this.byte_add(DECAL_TARGET_NODE)])
        .u32()
}

// Translated from 004a0350 (decompiled, FalloutNV.exe 1.4.0.525)
/// The decal's `bValidDecal` flag (Xbox PDB, `+0x2d`).
pub fn fn_004a0350(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u8(this.addr() + DECAL_VALID) != 0
}

// Translated from 004a0370 (decompiled, FalloutNV.exe 1.4.0.525)
/// Takes the lock of the object `0043c4b0` returns: enters the lock at
/// `+0x80` and, when its state (`fn_004a03a0`) is 1, calls `00483710`.
pub fn fn_004a0370(e: &mut Engine, this: Ptr) {
    e.call(LOCK_ENTER, &args![this.byte_add(0x80)]);
    if fn_004a03a0(e, this) == 1 {
        e.call(LOCK_EXTRA, &args![this]);
    }
}

// Translated from 004a03a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The state of the lock at `+0x80` (`00acbb70`, which returns zero in
/// this build).
pub fn fn_004a03a0(e: &mut Engine, this: Ptr) -> u32 {
    e.call(LOCK_STATE, &args![this.byte_add(0x80)]).u32()
}

// Translated from 004a03c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Releases what `fn_004a0370` took: calls `00483710` when the lock state
/// is 1, then leaves the lock at `+0x80`.
pub fn fn_004a03c0(e: &mut Engine, this: Ptr) {
    if fn_004a03a0(e, this) == 1 {
        e.call(LOCK_EXTRA, &args![this]);
    }
    e.call(LOCK_LEAVE, &args![this.byte_add(0x80)]);
}

// Translated from 004a03f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Walks the `DecalEmitterList` (`this + 0x14`): updates every emitter and
/// removes and destroys the ones that report they are finished.
pub fn fn_004a03f0(e: &mut Engine, this: Ptr<BGSDecalManager>) {
    let emitters = this.at(BGSDecalManager::DecalEmitterList);
    e.with_stack(8, |e, frame| {
        let iterator = frame.addr();
        let node = iterator + 4;
        let first = e.call(FIRST_WORD, &args![emitters]).u32();
        e.mem.set_u32(iterator, first);
        while e.mem.u32(iterator) != 0 {
            let current = e.mem.u32(iterator);
            e.mem.set_u32(node, current);
            let slot = e.call(LIST_ADVANCE, &args![emitters, iterator]).u32();
            let emitter = e.mem.u32(slot);
            if emitter == 0 {
                continue;
            }
            e.call(EMITTER_UPDATE, &args![emitter]);
            if e.call(EMITTER_FINISHED, &args![emitter]).bool() {
                e.call(EMITTER_LIST_REMOVE, &args![emitters, node]);
                fn_004a0490(e, Ptr::new(emitter), 1);
            }
        }
    });
}

// Translated from 004a0490 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BGSDecalEmitter`'s scalar deleting destructor: runs the destructor
/// body (`004a2cf0`) and frees the emitter when bit 0 of `flags` is set.
/// Returns `this`.
pub fn fn_004a0490(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(EMITTER_DESTRUCTOR_BODY, &args![this]);
    if flags & 1 != 0 {
        e.call(FREE, &args![this]);
    }
    this
}

// Translated from 004a04c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BGSDecalManager::IssueDecalOcclusionQuery` (Xbox PDB): renders the decal's
/// target from a camera placed near the decal, into the query texture,
/// between `BSOcclusionQuery::Begin` and `End`, and stores the query on the
/// decal.
///
/// The camera sits at `pOrigin + normalized(pVector) * 50`; its rotation is
/// `GetRotationZToVector` of the negated scaled direction, turned by the
/// decal's rotation about Z, with its columns
/// permuted. The frustum is orthographic: near plane 0.1, far plane the
/// decal depth + 50, half width and half height scaled by the target's
/// scale. Debug draw (flag bit 2) and the log lines are included. The
/// stack-unwinding frame is not translated.
pub fn bgs_decal_manager_issue_decal_occlusion_query(
    e: &mut Engine,
    this: Ptr<BGSDecalManager>,
    decal: Ptr,
) {
    // Named offsets inside the frame holding the function's locals.
    const DIRECTION: u32 = 0x00; // NiPoint3
    const POSITION: u32 = 0x0c; // NiPoint3: the camera position
    const ORIGIN: u32 = 0x18; // NiPoint3: copy of the decal's origin
    const NEGATED: u32 = 0x24; // NiPoint3
    const ROTATION: u32 = 0x30; // NiMatrix3
    const SPIN: u32 = 0x54; // NiMatrix3
    const PRODUCT: u32 = 0x78; // NiMatrix3
    const COLUMN_A: u32 = 0x9c; // NiPoint3
    const COLUMN_B: u32 = 0xa8; // NiPoint3
    const CAMERA_ROTATION: u32 = 0xb4; // NiMatrix3
    const LINE_START: u32 = 0xd8; // NiPoint3
    const LINE_END: u32 = 0xe4; // NiPoint3
    const FRUSTUM: u32 = 0xf0; // NiFrustum, 0x1c bytes
    const ZERO_POINT: u32 = 0x10c; // NiPoint3
    const VIEW_DIRECTION: u32 = 0x118; // NiPoint3
    const AIM: u32 = 0x124; // NiPoint3
    const AIM_SCALED: u32 = 0x130; // NiPoint3
    const AIM_END: u32 = 0x13c; // NiPoint3
    const RED: u32 = 0x148; // NiColorA
    const BLUE: u32 = 0x158; // NiColorA
    const CULLING: u32 = 0x180; // BSCullingProcess

    let camera_slot = this.byte_add(0x24); // spQueryCamera
    let accum_slot = this.byte_add(0x20); // spQueryAccum
    let decal_addr = decal.addr();
    e.with_stack(0x300, |e, frame| {
        let f = frame.addr();
        let direction = f + DIRECTION;
        let position = f + POSITION;
        let origin = f + ORIGIN;
        fn_004a0d60(e, decal, Ptr::new(direction));
        fn_004a0c10(e, Ptr::new(direction));
        let scale = e.global::<f32>(DIRECTION_SCALE);
        e.call(POINT3_SCALE_IN_PLACE, &args![direction, scale]);
        let copy = fn_004a0d30(e, decal, Ptr::new(origin)).addr();
        e.call(POINT3_ADD, &args![copy, position, direction]);

        let negated = fn_004a0bd0(e, Ptr::new(direction), Ptr::new(f + NEGATED)).addr();
        let (nx, ny, nz) = (
            e.mem.u32(negated),
            e.mem.u32(negated + 4),
            e.mem.u32(negated + 8),
        );
        let rotation = f + ROTATION;
        e.call(ROTATION_Z_TO_VECTOR, &args![rotation, nx, ny, nz]);

        let spin = f + SPIN;
        e.call(EMPTY_CONSTRUCTOR, &args![spin]);
        let angle = e.call(DECAL_ROTATION, &args![decal]).f32();
        fn_004a0c90(e, Ptr::new(spin), angle);
        let product = e
            .call(MATRIX_MULTIPLY, &args![spin, f + PRODUCT, rotation])
            .u32();
        copy_words(e, rotation, product, 9);

        let column_a = f + COLUMN_A;
        let column_b = f + COLUMN_B;
        e.call(EMPTY_CONSTRUCTOR, &args![column_a]);
        e.call(EMPTY_CONSTRUCTOR, &args![column_b]);
        e.call(MATRIX_GET_COLUMN, &args![rotation, 0u32, column_a]);
        e.call(MATRIX_GET_COLUMN, &args![rotation, 1u32, column_b]);
        e.call(MATRIX_GET_COLUMN, &args![rotation, 2u32, direction]);
        let camera_rotation = f + CAMERA_ROTATION;
        e.call(EMPTY_CONSTRUCTOR, &args![camera_rotation]);
        e.call(MATRIX_SET_COLUMN, &args![camera_rotation, 0u32, direction]);
        e.call(MATRIX_SET_COLUMN, &args![camera_rotation, 1u32, column_a]);
        e.call(MATRIX_SET_COLUMN, &args![camera_rotation, 2u32, column_b]);
        let line_start = f + LINE_START;
        let line_end = f + LINE_END;
        e.call(EMPTY_CONSTRUCTOR, &args![line_start]);
        e.call(EMPTY_CONSTRUCTOR, &args![line_end]);

        // Frustum: half width and half height scaled by the target's scale.
        let two = e.global::<f64>(TWO);
        let width = e.call(DECAL_WIDTH_GETTER, &args![decal]).f32();
        let half_width = width as f64 / two;
        let node = fn_004a0330(e, decal);
        let target_scale = e.call(TARGET_SCALE, &args![node]).f32();
        let right = ((1.0 / target_scale as f64) * half_width) as f32;
        let height = fn_004a0d90(e, decal);
        let half_height = height as f64 / two;
        let node = fn_004a0330(e, decal);
        let target_scale = e.call(TARGET_SCALE, &args![node]).f32();
        let top = ((1.0 / target_scale as f64) * half_height) as f32;
        let depth = fn_004a0db0(e, decal);
        let far = (depth as f64 + e.global::<f64>(FAR_MARGIN)) as f32;
        let frustum = f + FRUSTUM;
        e.call(FRUSTUM_CONSTRUCTOR, &args![frustum, 0u32]);
        e.mem.set_u8(frustum + 0x18, 1);
        let near = e.global::<f32>(NEAR_PLANE);
        e.mem.set_f32(frustum + 0x10, near);
        e.mem.set_f32(frustum, -right);
        e.mem.set_f32(frustum + 4, right);
        e.mem.set_f32(frustum + 8, top);
        e.mem.set_f32(frustum + 0xc, -top);
        e.mem.set_f32(frustum + 0x14, far);

        let camera = e.call(FIRST_WORD, &args![camera_slot]).u32();
        e.call(SET_TRANSLATE, &args![camera, position]);
        let camera = e.call(FIRST_WORD, &args![camera_slot]).u32();
        e.call(SET_ROTATE, &args![camera, camera_rotation]);
        let camera = e.call(FIRST_WORD, &args![camera_slot]).u32();
        e.call(SET_VIEW_FRUSTUM, &args![camera, frustum]);
        let zero_point = f + ZERO_POINT;
        e.call(
            POINT3_CONSTRUCTOR_ZERO,
            &args![zero_point, 0.0f32, 0u32, 0u32],
        );
        let camera = e.call(FIRST_WORD, &args![camera_slot]).u32();
        e.call(UPDATE_WITH_ZERO, &args![camera, zero_point]);

        if e.global::<u32>(DECAL_DEBUG_FLAGS) & 4 != 0 {
            copy_words(e, line_start, position, 3);
            let camera = e.call(FIRST_WORD, &args![camera_slot]).u32();
            let view = e.call(CAMERA_DIRECTION, &args![camera, f + AIM]).u32();
            let scaled = e
                .call(POINT3_MULTIPLY, &args![view, f + AIM_SCALED, far])
                .u32();
            let end = e
                .call(POINT3_ADD, &args![position, f + AIM_END, scaled])
                .u32();
            copy_words(e, line_end, end, 3);
            let first_colour = e
                .call(
                    COLOR_CONSTRUCTOR,
                    &args![f + RED, 1.0f32, 0.0f32, 0.0f32, 1.0f32],
                )
                .u32();
            let second_colour = e
                .call(
                    COLOR_CONSTRUCTOR,
                    &args![f + BLUE, 0.0f32, 0.0f32, 1.0f32, 1.0f32],
                )
                .u32();
            let line = e
                .call(
                    MAKE_DEBUG_LINE,
                    &args![line_start, second_colour, line_end, first_colour, 1u32],
                )
                .u32();
            let seconds = e.global::<f32>(DEBUG_LINE_SECONDS);
            let tes = e.global::<u32>(TES_OBJECT);
            e.call(ADD_TEMP_DEBUG_OBJECT, &args![tes, line, seconds]);
        }

        if setting_flag(e, SETTING_DEBUG_LOG) {
            let view_direction = f + VIEW_DIRECTION;
            let camera = e.call(FIRST_WORD, &args![camera_slot]).u32();
            e.call(CAMERA_DIRECTION, &args![camera, view_direction]);
            let name = target_name(e, decal);
            e.call(LOG, &args![TEXT_ISSUING, name]);
            let origin_values = [
                e.mem.f32(position),
                e.mem.f32(position + 4),
                e.mem.f32(position + 8),
            ];
            log_floats(e, TEXT_ORIGIN, &origin_values);
            let direction_values = [
                e.mem.f32(view_direction),
                e.mem.f32(view_direction + 4),
                e.mem.f32(view_direction + 8),
            ];
            log_floats(e, TEXT_DIRECTION, &direction_values);
            log_floats(e, TEXT_FRUSTUM, &[far, -right, right, top, -top]);
        }

        if e.call(FIRST_WORD, &args![this]).u32() == 0 {
            let format = fn_004a0e90(e);
            let manager = fn_004a0ea0(e);
            let texture = e
                .call(
                    CREATE_QUERY_TEXTURE,
                    &args![manager, format, 0x33u32, 0u32, 0u32, 0u32],
                )
                .u32();
            e.call(NI_POINTER_ASSIGN, &args![this, texture]);
        }
        let texture = e.call(FIRST_WORD, &args![this]).u32();
        let stopped = e.call(STOP_OFFSCREEN, &args![texture]).u32();
        let target = if e.mem.u8(decal_addr + DECAL_CLEAR_QUERY_TEXTURE) != 0 {
            7u32
        } else {
            0u32
        };
        e.call(START_OFFSCREEN, &args![target, stopped]);

        let query_memory = e.call(ALLOCATE, &args![4u32]).u32();
        let query = if query_memory != 0 {
            e.call(QUERY_CONSTRUCTOR, &args![query_memory]).u32()
        } else {
            0
        };
        e.mem.set_u32(decal_addr + DECAL_QUERY, query);
        e.call(QUERY_BEGIN, &args![query]);
        let camera = e.call(FIRST_WORD, &args![camera_slot]).u32();
        let camera_frustum = fn_004a0d10(e, Ptr::new(camera));
        let global_object = e.call(GET_GLOBAL_OBJECT, &args![]).ptr::<()>();
        fn_004a0dd0(e, global_object, camera_frustum.addr());

        let culling = f + CULLING;
        fn_004a0eb0(e, Ptr::new(culling), 0);
        let accumulator = e.call(FIRST_WORD, &args![accum_slot]).u32();
        fn_004a0fd0(e, Ptr::new(culling), accumulator);
        let mode = e.call(ACCUMULATOR_MODE, &args![0u32]).u32();
        let accumulator = e.call(FIRST_WORD, &args![accum_slot]).u32();
        fn_004a1020(e, Ptr::new(accumulator), mode);
        let accumulator = e.call(FIRST_WORD, &args![accum_slot]).u32();
        fn_004a1040(e, Ptr::new(accumulator), 0xf);
        let node = fn_004a0330(e, decal);
        let camera = e.call(FIRST_WORD, &args![camera_slot]).u32();
        e.call(ACCUMULATE_SCENE, &args![camera, node, culling]);
        let accumulator = e.call(FIRST_WORD, &args![accum_slot]).u32();
        let camera = e.call(FIRST_WORD, &args![camera_slot]).u32();
        e.call(RENDER_ACCUMULATED, &args![camera, accumulator, 0u32]);
        fn_004a0fd0(e, Ptr::new(culling), 0);
        let query = e.mem.u32(decal_addr + DECAL_QUERY);
        e.call(QUERY_END, &args![query]);
        e.call(END_OFFSCREEN, &args![]);
        bs_culling_process_dtor(e, Ptr::new(culling));
    });
}

// Translated from 004a0bd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Builds the negation of the vector at `this` in `result` (through the
/// `NiPoint3` constructor) and returns `result`.
pub fn fn_004a0bd0(e: &mut Engine, this: Ptr, result: Ptr) -> Ptr {
    let x = -e.mem.f32(this.addr());
    let y = -e.mem.f32(this.addr() + 4);
    let z = -e.mem.f32(this.addr() + 8);
    e.call(POINT3_CONSTRUCTOR, &args![result, x, y, z]);
    result
}

// Translated from 004a0c10 (decompiled, FalloutNV.exe 1.4.0.525)
/// Normalizes the vector at `this` in place; a vector whose length is not
/// above the minimum length (the double at `0x01017cf8`) becomes zero.
pub fn fn_004a0c10(e: &mut Engine, this: Ptr) {
    let length = e.call(POINT3_LENGTH, &args![this]).f32();
    let at = this.addr();
    if length as f64 > e.global::<f64>(MINIMUM_LENGTH) {
        let inverse = (1.0 / length as f64) as f32;
        for offset in [0, 4, 8] {
            let component = e.mem.f32(at + offset);
            e.mem
                .set_f32(at + offset, (component as f64 * inverse as f64) as f32);
        }
    } else {
        for offset in [0, 4, 8] {
            e.mem.set_f32(at + offset, 0.0);
        }
    }
}

// Translated from 004a0c90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Writes the 3x3 rotation about Z by `angle` into the matrix at `this`:
/// `[cos sin 0; -sin cos 0; 0 0 1]`.
pub fn fn_004a0c90(e: &mut Engine, this: Ptr, angle: f32) {
    let (sine, cosine) = e.with_stack(8, |e, slot| {
        e.call(SIN_COS, &args![angle, slot, slot.byte_add(4)]);
        (e.mem.f32(slot.addr()), e.mem.f32(slot.addr() + 4))
    });
    let at = this.addr();
    e.mem.set_f32(at, cosine);
    e.mem.set_f32(at + 0x04, sine);
    e.mem.set_f32(at + 0x08, 0.0);
    e.mem.set_f32(at + 0x0c, -sine);
    e.mem.set_f32(at + 0x10, cosine);
    e.mem.set_f32(at + 0x14, 0.0);
    e.mem.set_f32(at + 0x18, 0.0);
    e.mem.set_f32(at + 0x1c, 0.0);
    e.mem.set_f32(at + 0x20, 1.0);
}

// Translated from 004a0d10 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of the `NiFrustum` inside a `NiCamera` (`m_kViewFrustum`,
/// Xbox PDB, `+0x100`).
pub fn fn_004a0d10(_e: &mut Engine, this: Ptr) -> Ptr {
    this.byte_add(0x100)
}

// Translated from 004a0d30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Copies the decal's `pOrigin` (`+0x34`) into `result` and returns it.
pub fn fn_004a0d30(e: &mut Engine, this: Ptr, result: Ptr) -> Ptr {
    copy_words(e, result.addr(), this.addr() + DECAL_ORIGIN, 3);
    result
}

// Translated from 004a0d60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Copies the decal's `pVector` (`+0x40`) into `result` and returns it.
pub fn fn_004a0d60(e: &mut Engine, this: Ptr, result: Ptr) -> Ptr {
    copy_words(e, result.addr(), this.addr() + DECAL_VECTOR, 3);
    result
}

// Translated from 004a0d90 (decompiled, FalloutNV.exe 1.4.0.525)
/// The decal's `fHeight` (Xbox PDB, `+0x11c`).
pub fn fn_004a0d90(e: &mut Engine, this: Ptr) -> f32 {
    e.mem.f32(this.addr() + DECAL_HEIGHT)
}

// Translated from 004a0db0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The decal's `fDepth` (Xbox PDB, `+0x120`).
pub fn fn_004a0db0(e: &mut Engine, this: Ptr) -> f32 {
    e.mem.f32(this.addr() + DECAL_DEPTH)
}

// Translated from 004a0dd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `SetScreenSpaceCameraData`: when the object's mode (`fn_004a0e10` with
/// the name `"SetScreenSpaceCameraData"` and 1) matches, calls its virtual
/// at `+0x190` with `camera_data`.
pub fn fn_004a0dd0(e: &mut Engine, this: Ptr, camera_data: u32) {
    if fn_004a0e10(e, this, TEXT_SET_SCREEN_SPACE_CAMERA_DATA, 1) {
        e.vcall(this.addr(), 0x190, &args![camera_data]);
    }
}

// Translated from 004a0e10 (decompiled, FalloutNV.exe 1.4.0.525)
/// True when `fn_004a0e50` holds and the byte at `+0x208` equals
/// `expected`. The first parameter (a name) is not read.
pub fn fn_004a0e10(e: &mut Engine, this: Ptr, _unused_1: u32, expected: u8) -> bool {
    if !fn_004a0e50(e, this) {
        return false;
    }
    e.mem.u8(this.addr() + 0x208) == expected
}

// Translated from 004a0e50 (decompiled, FalloutNV.exe 1.4.0.525)
/// True when the dword at `+0x200` is 1 or 2.
pub fn fn_004a0e50(e: &mut Engine, this: Ptr) -> bool {
    let mode = e.mem.u32(this.addr() + 0x200);
    mode == 1 || mode == 2
}

// Translated from 004a0e90 (decompiled, FalloutNV.exe 1.4.0.525)
/// The first word of the global at `0x011f9508` (the query texture's
/// format argument).
pub fn fn_004a0e90(e: &mut Engine) -> u32 {
    e.call(FIRST_WORD, &args![QUERY_TEXTURE_FORMAT]).u32()
}

// Translated from 004a0ea0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The global at `0x011f91a8` (the object that creates the query texture).
pub fn fn_004a0ea0(e: &mut Engine) -> u32 {
    e.global::<u32>(TEXTURE_MANAGER)
}

// Translated from 004a0eb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSCullingProcess` constructor (no name in the engine map): the base
/// constructor with `base_argument`, the vtable, `+0x90`, `+0xbc` and
/// `+0xc0` cleared, and the `NiPointer` at `+0xc4` initialized with null.
/// Returns `this`. The unwinding frame is not translated.
pub fn fn_004a0eb0(e: &mut Engine, this: Ptr, base_argument: u32) -> Ptr {
    e.call(CULLING_BASE_CONSTRUCTOR, &args![this, base_argument]);
    let at = this.addr();
    e.mem.set_u32(at, CULLING_PROCESS_VTABLE);
    e.mem.set_u32(at + 0x90, 0);
    e.mem.set_u32(at + 0xbc, 0);
    e.mem.set_u32(at + 0xc0, 0);
    e.call(NI_POINTER_INIT, &args![at + 0xc4, 0u32]);
    this
}

// Translated from 004a0f50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSCullingProcess::GetRTTI` (Xbox PDB): the address of the class's
/// `NiRTTI`.
pub fn bs_culling_process_get_rtti(_e: &mut Engine, _this: Ptr) -> u32 {
    CULLING_PROCESS_RTTI
}

// Translated from 004a0f60 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSCullingProcess::~BSCullingProcess` (Xbox PDB): sets the vtable,
/// releases the `NiPointer` at `+0xc4` and runs the base destructor. The
/// unwinding frame is not translated.
pub fn bs_culling_process_dtor(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), CULLING_PROCESS_VTABLE);
    e.call(NI_POINTER_RELEASE, &args![this.byte_add(0xc4)]);
    e.call(CULLING_BASE_DESTRUCTOR, &args![this]);
}

// Translated from 004a0fd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Assigns `accumulator` to the `NiPointer` at `+0xc4` (the culling
/// process's accumulator).
pub fn fn_004a0fd0(e: &mut Engine, this: Ptr, accumulator: u32) {
    e.call(NI_POINTER_ASSIGN, &args![this.byte_add(0xc4), accumulator]);
}

// Translated from 004a0ff0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSCullingProcess::_scalar_deleting_destructor_` (Xbox PDB): the
/// destructor, then frees the object when bit 0 of `flags` is set. Returns
/// `this`.
pub fn bs_culling_process_scalar_deleting_destructor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    bs_culling_process_dtor(e, this);
    if flags & 1 != 0 {
        e.call(FREE, &args![this]);
    }
    this
}

// Translated from 004a1020 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `value` at `+0x194` (the accumulator's mode).
pub fn fn_004a1020(e: &mut Engine, this: Ptr, value: u32) {
    e.mem.set_u32(this.addr() + 0x194, value);
}

// Translated from 004a1040 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `value` at `+0x19c` (the accumulator's pass mask).
pub fn fn_004a1040(e: &mut Engine, this: Ptr, value: u32) {
    e.mem.set_u32(this.addr() + 0x19c, value);
}

// Translated from 004a1060 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether a decal may be placed on the reference: false when its base
/// form's type byte (`+4`) is `0x23`, `0x2a`, `0x2b` or `0x33`, true
/// otherwise. The second parameter is not read.
pub fn fn_004a1060(e: &mut Engine, reference: Ptr, _unused_1: u32) -> bool {
    let form = e.call(REFERENCE_BASE_FORM, &args![reference]).u32();
    let form_type = e.call(FORM_TYPE, &args![form]).u32();
    !matches!(form_type, 0x23 | 0x2a | 0x2b | 0x33)
}

// Translated from 004a10d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BGSDecalManager::AddDecal` (Xbox PDB): marks the query texture to be
/// cleared and, by `decal_type` (the Xbox PDB `DECAL_TYPE`: 1 simple, 2
/// projected, 4 permanent simple), adds the decal described by `placement`:
///
/// - types 1 and 4: with the geometry-decal setting on, a target, and a
///   target that passes `fn_004a19d0` (flag `0x400` clear), adds a
///   geometry decal. Type 1 does it when the distance from the camera is
///   within the limit (`fn_004a19f0`), or `skip_distance_limit` or `force`
///   is set, and times the call when the profile setting is on; type 4 sets
///   `flag_71` first and has no distance test.
/// - type 2: with the projected-decal setting on, projects onto the
///   target's world bound (when it passes `fn_004a19d0`), or onto the node
///   `source_object` returns, in both cases only when the distance from
///   the camera to the bound is within the limit (`fn_004a1a00`) and under
///   the skinned-decal cap, or `force` is set. Otherwise, with the
///   geometry-decal setting on, casts probes with the decal caster and
///   adds a geometry decal at every hit that is a reference accepted by
///   `fn_004a1060`.
///
/// The profiling scope's begin and end calls are made; the
/// stack-unwinding frame is not translated.
pub fn bgs_decal_manager_add_decal(
    e: &mut Engine,
    this: Ptr<BGSDecalManager>,
    placement: Ptr<DecalPlacement>,
    decal_type: u32,
    force: bool,
) {
    let p = placement.addr();
    e.with_stack(4, |e, scope| {
        e.call(
            SCOPE_BEGIN,
            &args![scope, 7u32, 1u32, TEXT_SOURCE_FILE, ADD_DECAL_SOURCE_LINE],
        );
        e.set(this, BGSDecalManager::bClearQueryTexture, true);
        match decal_type {
            1 => add_decal_type_1(e, this, placement, force),
            4 => {
                let target = e.mem.u32(p + 0x28);
                if setting_flag(e, SETTING_DECALS)
                    && target != 0
                    && !fn_004a19d0(e, Ptr::new(target))
                {
                    e.mem.set_u8(p + 0x71, 1);
                    bgs_decal_manager_add_geometry_decal_recurse(e, this, placement);
                }
            }
            2 => add_decal_type_2(e, this, placement, force),
            _ => {}
        }
        e.call(SCOPE_END, &args![scope]);
    });
}

/// Type 1 of `AddDecal`.
fn add_decal_type_1(
    e: &mut Engine,
    this: Ptr<BGSDecalManager>,
    placement: Ptr<DecalPlacement>,
    force: bool,
) {
    let p = placement.addr();
    if !setting_flag(e, SETTING_DECALS) {
        return;
    }
    let target = e.mem.u32(p + 0x28);
    if target == 0 || fn_004a19d0(e, Ptr::new(target)) {
        return;
    }
    // 8 bytes each for the frequency and the two counters, 12 for the
    // camera position copy and 12 for the offset.
    e.with_stack(0x40, |e, frame| {
        let f = frame.addr();
        let (frequency, start, end) = (f, f + 8, f + 0x10);
        let (camera, offset) = (f + 0x18, f + 0x24);
        load_camera_position(e, camera);
        let difference = e.call(POINT3_SUBTRACT, &args![p, offset, camera]).u32();
        let length = e.call(POINT3_LENGTH, &args![difference]).f32();
        let mut ticks_per_microsecond = e.global::<f64>(THOUSAND);
        e.mem.set_u64(start, 0);
        if setting_flag(e, SETTING_PROFILE) {
            e.call(QUERY_PERFORMANCE_FREQUENCY, &args![frequency]);
            ticks_per_microsecond =
                e.mem.u64(frequency) as i64 as f64 * e.global::<f64>(MICROSECOND);
            e.call(QUERY_PERFORMANCE_COUNTER, &args![start]);
        }
        let limit = fn_004a19f0(e);
        let within = (length < limit) != (length == limit);
        if within || e.mem.u8(p + 0x77) != 0 || force {
            if setting_flag(e, SETTING_DEBUG_LOG) {
                e.call(LOG, &args![TEXT_INSTANTIATED]);
            }
            bgs_decal_manager_add_geometry_decal_recurse(e, this, placement);
        }
        if setting_flag(e, SETTING_PROFILE) {
            e.call(QUERY_PERFORMANCE_COUNTER, &args![end]);
            let ticks = e.mem.u64(end).wrapping_sub(e.mem.u64(start));
            let microseconds = ticks as i64 as f64 / ticks_per_microsecond;
            let buffer = e.mem.alloc(0x100);
            let mut words = vec![buffer, 0xff, TEXT_TICKS, ticks as u32, (ticks >> 32) as u32];
            microseconds.put(&mut words);
            e.call(FORMAT, &words);
            e.call(REPORT, &args![buffer, 0u32]);
            e.mem.free(buffer);
        }
    });
}

/// The distance test of the projected paths of type 2: the distance is
/// within the limit (`fn_004a1a00`) and the skinned-decal count is under
/// its cap, or `force` is set.
fn projected_decal_allowed(e: &mut Engine, distance: f32, force: bool) -> bool {
    let limit = fn_004a1a00(e);
    let within = (distance < limit) != (distance == limit);
    if within {
        let cap = setting_int(e, SETTING_MAX_SKIN_DECALS) as u32;
        if e.global::<u32>(SKINNED_DECAL_COUNT) < cap {
            return true;
        }
    }
    force
}

/// The "Reached Max Skin Decal Limit" log of the projected paths.
fn log_skin_limit(e: &mut Engine) {
    let cap = setting_int(e, SETTING_MAX_SKIN_DECALS) as u32;
    if e.global::<u32>(SKINNED_DECAL_COUNT) >= cap && setting_flag(e, SETTING_DEBUG_LOG) {
        let cap = setting_int(e, SETTING_MAX_SKIN_DECALS);
        e.call(LOG, &args![TEXT_SKIN_LIMIT, cap]);
    }
}

/// Distance from the camera position stored at `f` (12 bytes) to the near
/// surface of the `NiBound` at `bound_source`: the length of the vector
/// from the bound's centre to the camera minus the radius. `f + 0x0c`
/// receives a copy of the bound and `f + 0x1c` the difference vector.
fn distance_to_bound(e: &mut Engine, f: u32, bound_source: u32) -> f32 {
    let (camera, bound, offset) = (f, f + 0x0c, f + 0x1c);
    copy_words(e, bound, bound_source, 4);
    let centre = e.call(EMPTY_CONSTRUCTOR, &args![bound]).u32();
    let difference = e
        .call(POINT3_SUBTRACT, &args![centre, offset, camera])
        .u32();
    let length = e.call(POINT3_LENGTH, &args![difference]).f32();
    let radius = e.call(BOUND_RADIUS, &args![bound]).f32();
    (length as f64 - radius as f64) as f32
}

/// Type 2 of `AddDecal`.
fn add_decal_type_2(
    e: &mut Engine,
    this: Ptr<BGSDecalManager>,
    placement: Ptr<DecalPlacement>,
    force: bool,
) {
    let p = placement.addr();
    let target = e.mem.u32(p + 0x28);
    if setting_flag(e, SETTING_SKINNED_DECALS) && target != 0 && !fn_004a19d0(e, Ptr::new(target)) {
        e.with_stack(0x30, |e, frame| {
            let f = frame.addr();
            load_camera_position(e, f);
            let bound_source = e.call(GET_WORLD_BOUND, &args![target]).u32();
            let distance = distance_to_bound(e, f, bound_source);
            if projected_decal_allowed(e, distance, force) {
                e.call(ADD_PROJECTED_DECAL_RECURSE, &args![this, placement]);
            } else {
                log_skin_limit(e);
            }
        });
        return;
    }
    let source_object = e.mem.u32(p + 0x24);
    if setting_flag(e, SETTING_SKINNED_DECALS) && source_object != 0 {
        e.with_stack(0x30, |e, frame| {
            let f = frame.addr();
            load_camera_position(e, f);
            let node = e.vcall(source_object, 0x1d0, &args![]).u32();
            let bound_source = e.call(GET_WORLD_BOUND, &args![node]).u32();
            let distance = distance_to_bound(e, f, bound_source);
            let node = e.vcall(source_object, 0x1d0, &args![]).u32();
            let projected = if node != 0 {
                e.vcall(node, 0xc, &args![]).u32()
            } else {
                0
            };
            if projected != 0 && !fn_004a19d0(e, Ptr::new(node)) {
                e.mem.set_u32(p + 0x28, projected);
                if projected_decal_allowed(e, distance, force) {
                    e.call(ADD_PROJECTED_DECAL_RECURSE, &args![this, placement]);
                } else {
                    log_skin_limit(e);
                }
            }
        });
        return;
    }
    if !setting_flag(e, SETTING_DECALS) {
        return;
    }
    // Neither projection applies: probe with the decal caster.
    fn_004a0c10(e, Ptr::new(p + 0x0c));
    if e.global::<u32>(DECAL_CASTER) == 0 {
        let size_b = e.mem.f32(p + 0x3c);
        let size_a = e.mem.f32(p + 0x38);
        let radius = e.call(FLOAT_MAX, &args![size_b, size_a]).f32();
        let caster = e.call(DECAL_CASTER_CREATE, &args![radius, 1u32]).u32();
        e.set_global(DECAL_CASTER, caster);
    }
    let caster = e.global::<u32>(DECAL_CASTER);
    fn_004a1a10(e, Ptr::new(caster), 8, 0);
    let caster = e.global::<u32>(DECAL_CASTER);
    fn_004a1a10(e, Ptr::new(caster), 0x1d, 0);
    let object_48 = e.mem.u32(p + 0x48);
    let texture_set = e.call(OBJECT_TEXTURE_SET, &args![object_48]).u32();
    let caster = e.global::<u32>(DECAL_CASTER);
    e.call(DECAL_CASTER_SET_TEXTURE_SET, &args![caster, texture_set]);
    e.with_stack(0x80, |e, frame| {
        let f = frame.addr();
        // Vectors of the two probes, then the list of objects already
        // hit, one hit's position and normal, and the item slot.
        let (scaled, offset) = (f, f + 0x0c);
        let (negated, shifted, sum) = (f + 0x18, f + 0x24, f + 0x30);
        let (aim, aim_end) = (f + 0x3c, f + 0x48);
        let (list, position, normal, slot) = (f + 0x54, f + 0x60, f + 0x6c, f + 0x78);
        let probe_length = e.global::<f32>(PROBE_LENGTH);
        let probe_radius = e.global::<f32>(PROBE_RADIUS);
        let direction = e
            .call(POINT3_MULTIPLY, &args![p + 0x0c, scaled, probe_length])
            .u32();
        let end = e.call(POINT3_ADD, &args![p, offset, direction]).u32();
        let caster = e.global::<u32>(DECAL_CASTER);
        let mut hits = e
            .call(DECAL_CASTER_CAST, &args![caster, p, end, probe_radius])
            .u32() as i32;
        if hits == 0 {
            // Probe again from the direction shifted by the negated
            // fallback vector.
            let opposite = fn_004a0bd0(e, Ptr::new(FALLBACK_VECTOR), Ptr::new(negated)).addr();
            let fallback_scale = e.global::<f32>(FALLBACK_SCALE);
            let pushed = e
                .call(POINT3_MULTIPLY, &args![opposite, shifted, fallback_scale])
                .u32();
            e.call(POINT3_ADD, &args![p + 0x0c, sum, pushed]);
            fn_004a0c10(e, Ptr::new(sum));
            copy_words(e, p + 0x0c, sum, 3);
            let direction = e
                .call(POINT3_MULTIPLY, &args![sum, aim, probe_length])
                .u32();
            let end = e.call(POINT3_ADD, &args![p, aim_end, direction]).u32();
            let caster = e.global::<u32>(DECAL_CASTER);
            hits = e
                .call(DECAL_CASTER_CAST, &args![caster, p, end, probe_radius])
                .u32() as i32;
        }
        e.call(LIST_CONSTRUCTOR, &args![list]);
        for index in 0..hits {
            e.call(EMPTY_CONSTRUCTOR, &args![position]);
            e.call(EMPTY_CONSTRUCTOR, &args![normal]);
            let caster = e.global::<u32>(DECAL_CASTER);
            let object = e.call(DECAL_CASTER_HIT_OBJECT, &args![caster, index]).u32();
            if object == 0 {
                continue;
            }
            e.call(
                DECAL_CASTER_HIT_DATA,
                &args![caster, index, position, normal],
            );
            fn_004a0c10(e, Ptr::new(normal));
            let reference = e.call(FIND_REFERENCE_FOR_3D, &args![object]).u32();
            if fn_004a19d0(e, Ptr::new(object))
                || reference == 0
                || !fn_004a1060(e, Ptr::new(reference), 1)
            {
                continue;
            }
            e.mem.set_u32(slot, object);
            if e.call(LIST_CONTAINS, &args![list, slot, 0u32]).u32() != 0 {
                continue;
            }
            copy_words(e, p, position, 3);
            copy_words(e, p + 0x0c, normal, 3);
            e.mem.set_u32(p + 0x28, object);
            bgs_decal_manager_add_geometry_decal_recurse(e, this, placement);
            e.call(LIST_ADD_HEAD, &args![list, slot]);
        }
        e.call(LIST_REMOVE_ALL, &args![list]);
        fn_004a1a30(e, Ptr::new(list));
    });
}

// Translated from 004a19d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the object has flag bit `0x400` set (`00456630` with the mask).
pub fn fn_004a19d0(e: &mut Engine, this: Ptr) -> bool {
    e.call(FLAG_TEST, &args![this, FLAG_HIDDEN]).bool()
}

// Translated from 004a19f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The float setting at `0x011c77a8` (the distance limit of type 1
/// decals), in `ST0`.
pub fn fn_004a19f0(e: &mut Engine) -> f32 {
    e.call(SETTING_FLOAT_VALUE, &args![SETTING_GEOMETRY_DISTANCE])
        .f32()
}

// Translated from 004a1a00 (decompiled, FalloutNV.exe 1.4.0.525)
/// The float setting at `0x011c7588` (the distance limit of type 2
/// decals), in `ST0`.
pub fn fn_004a1a00(e: &mut Engine) -> f32 {
    e.call(SETTING_FLOAT_VALUE, &args![SETTING_PROJECTED_DISTANCE])
        .f32()
}

// Translated from 004a1a10 (decompiled, FalloutNV.exe 1.4.0.525)
/// The decal caster's `AddFilter`-style call: forwards to
/// `00c827f0(0x27, a, b)` (cdecl); `this` is not used.
pub fn fn_004a1a10(e: &mut Engine, _this: Ptr, a: u32, b: u8) {
    e.call(COLLISION_CALL, &args![0x27u32, a, b]);
}

// Translated from 004a1a30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Runs `004a47b0` (a destructor body later in this unit) on the object.
pub fn fn_004a1a30(e: &mut Engine, this: Ptr) {
    e.call(LIST_LOCAL_DESTRUCTOR_BODY, &args![this]);
}

// Translated from 004a1a50 (decompiled, FalloutNV.exe 1.4.0.525)
/// When `object` is non-null: asks the lazily created singleton
/// (`0049fef0`, which takes no argument although the caller pushes the
/// address of the parameter) and calls `004ed8c0` on its `+0x14`.
pub fn fn_004a1a50(e: &mut Engine, object: u32) {
    if object == 0 {
        return;
    }
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), object);
        let singleton = e.call(SINGLETON_GETTER, &args![slot]).u32();
        e.call(SINGLETON_RELEASE, &args![singleton + 0x14]);
    });
}

// Translated from 004a1a70 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BGSDecalManager::AddGeometryDecalRecurse` (Xbox PDB): adds a decal on
/// the placement's target node, or recurses into its children. Returns
/// false when the per-frame limit is reached or a reach test fails, true
/// otherwise.
///
/// A target whose name starts with one of `SKIPPED_NAME_PREFIXES` gets no
/// decal. A target that is a model node and not hidden gets one: the first
/// accepted geometry of its geometry list becomes the decal's geometry;
/// when `value_5c` is positive, every entry of that geometry's bound list
/// is tested, and the decal is refused when `(max(size_a, size_b) +
/// entry width)^2 / value_5c` is not below the squared offset to the
/// entry's origin summed over the axes. The new `BSTempEffectSimpleDecal`
/// is flagged for an occlusion query when the occlusion setting is on,
/// `fn_004a1ff0` of the target is above 1000 and `occlusion_query_wanted`
/// is set, and is queued on the pending list when `object_48` is set.
pub fn bgs_decal_manager_add_geometry_decal_recurse(
    e: &mut Engine,
    this: Ptr<BGSDecalManager>,
    placement: Ptr<DecalPlacement>,
) -> bool {
    let p = placement.addr();
    let limit = setting_int(e, SETTING_MAX_DECALS_PER_FRAME);
    if e.global::<i32>(DECALS_THIS_FRAME) >= limit {
        if setting_flag(e, SETTING_DEBUG_LOG) {
            e.call(LOG, &args![TEXT_FRAME_LIMIT]);
        }
        return false;
    }
    let node = e.mem.u32(p + 0x28);
    if node == 0 {
        return true;
    }
    let holder = e.call(NODE_NAME_HOLDER, &args![node]).u32();
    let name = e.call(NAME_TEXT, &args![holder]).u32();
    if name != 0 {
        for (prefix, length) in SKIPPED_NAME_PREFIXES {
            if e.call(STRNCMP, &args![name, prefix, length]).u32() == 0 {
                return true;
            }
        }
    }
    let is_model = e.call(IS_MODEL_NODE, &args![MODEL_FILTER, node]).bool();
    if !(is_model && !e.call(NODE_IS_HIDDEN, &args![node]).bool()) {
        // Not a geometry leaf: recurse into the children.
        if e.vcall(node, 0xc, &args![]).u32() == 0 {
            return true;
        }
        if e.call(NODE_IS_HIDDEN, &args![node]).bool() {
            return true;
        }
        let mut index = 0u32;
        while index < e.call(CHILD_COUNT, &args![node]).u32() {
            let child = e.call(CHILD_AT, &args![node, index]).u32();
            if child != 0 {
                e.mem.set_u32(p + 0x28, child);
                bgs_decal_manager_add_geometry_decal_recurse(e, this, placement);
            }
            index += 1;
        }
        return true;
    }

    let model_data = e.call(NODE_MODEL_DATA, &args![node]).u32();
    let model_object = e.call(MODEL_DATA_OBJECT, &args![model_data]).u32();
    if model_object != 0 && e.call(MODEL_HAS_TAG, &args![model_object, 0x1au32]).bool() {
        return true;
    }
    let geometry_list = e.call(NODE_GEOMETRY_LIST, &args![node]).u32();
    if geometry_list == 0 {
        return false;
    }
    let mut picked = 0u32;
    let mut index = 0u32;
    while index < e.call(CHILD_COUNT, &args![geometry_list]).u32() {
        let geometry = e.call(CHILD_AT, &args![geometry_list, index]).u32();
        if geometry != 0
            && e.call(GEOMETRY_ACCEPTED, &args![GEOMETRY_FILTER, geometry])
                .bool()
        {
            picked = geometry;
            break;
        }
        index += 1;
    }

    let value_5c = e.mem.f32(p + 0x5c);
    if picked != 0 && value_5c as f64 > e.global::<f64>(ZERO) {
        let mut bound = e.mem.u32(picked + 0xac);
        while bound != 0 && !e.call(BOUND_IS_END, &args![bound]).bool() {
            let entry_holder = e.call(EMPTY_CONSTRUCTOR, &args![bound]).u32();
            let entry = e.call(FIRST_WORD, &args![entry_holder]).u32();
            bound = e.call(BOUND_NEXT, &args![bound]).u32();
            let refused = e.with_stack(0x30, |e, frame| {
                let f = frame.addr();
                let (entry_origin, offset, squared) = (f, f + 0x0c, f + 0x18);
                fn_004a0d30(e, Ptr::new(entry), Ptr::new(entry_origin));
                let extent = e.call(DECAL_WIDTH_GETTER, &args![entry]).f32();
                e.call(POINT3_SUBTRACT, &args![p, offset, entry_origin]);
                let result = fn_004a1f90(e, Ptr::new(squared), Ptr::new(offset), Ptr::new(offset));
                copy_words(e, offset, result.addr(), 3);
                let size_a = e.mem.f32(p + 0x38);
                let size_b = e.mem.f32(p + 0x3c);
                let larger = if size_b < size_a { size_a } else { size_b };
                let reach = larger as f64 + extent as f64;
                let sum = (e.mem.f32(offset) as f64 + e.mem.f32(offset + 4) as f64)
                    + e.mem.f32(offset + 8) as f64;
                let quotient = reach * reach / value_5c as f64;
                let below = quotient < sum;
                !below
            });
            if refused {
                return false;
            }
        }
    }

    let count = e.global::<i32>(DECALS_THIS_FRAME).wrapping_add(1);
    e.set_global(DECALS_THIS_FRAME, count);
    if setting_flag(e, SETTING_DEBUG_LOG) {
        e.call(LOG, &args![TEXT_FRAME_COUNT, count]);
    }
    let memory = e.call(ALLOCATE_OBJECT, &args![DECAL_SIZE]).u32();
    let decal = if memory != 0 {
        let lifetime = e
            .call(SETTING_FLOAT_VALUE, &args![SETTING_DECAL_LIFETIME])
            .f32();
        e.call(
            DECAL_CONSTRUCTOR,
            &args![memory, picked, lifetime, placement],
        )
        .u32()
    } else {
        0
    };
    let target = e.mem.u32(p + 0x28);
    let query_size = fn_004a1ff0(e, Ptr::new(target));
    if query_size > 1000 && setting_flag(e, SETTING_OCCLUSION_QUERY) && e.mem.u8(p + 0x72) != 0 {
        e.mem.set_u8(decal + DECAL_QUERY_PENDING, 1);
        let clear = e.get(this, BGSDecalManager::bClearQueryTexture);
        e.mem.set_u8(decal + DECAL_CLEAR_QUERY_TEXTURE, clear as u8);
        e.set(this, BGSDecalManager::bClearQueryTexture, false);
    }
    if e.mem.u32(p + 0x48) != 0 {
        e.with_stack(4, |e, slot| {
            e.call(NI_POINTER_INIT, &args![slot, decal]);
            e.call(
                LIST_ADD_TAIL,
                &args![this.at(BGSDecalManager::PendingSimpleDecalList), slot],
            );
            e.call(NI_POINTER_RELEASE, &args![slot]);
        });
    }
    true
}

// Translated from 004a1f90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Component-wise product of the vectors `a` and `b` into `result` (through
/// the `NiPoint3` constructor); returns `result`.
pub fn fn_004a1f90(e: &mut Engine, result: Ptr, a: Ptr, b: Ptr) -> Ptr {
    let z = e.mem.f32(a.addr() + 8) * e.mem.f32(b.addr() + 8);
    let y = e.mem.f32(a.addr() + 4) * e.mem.f32(b.addr() + 4);
    let x = e.mem.f32(a.addr()) * e.mem.f32(b.addr());
    e.call(POINT3_CONSTRUCTOR, &args![result, x, y, z]);
    result
}

// Translated from 004a1ff0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Gets the object `005495f0` returns for `this` and calls its virtual at
/// `+0xa8`; the result is its low 16 bits (the caller reads `AX`).
pub fn fn_004a1ff0(e: &mut Engine, this: Ptr) -> u16 {
    let object = e.call(OBJECT_OF_NODE, &args![this]).u32();
    e.vcall(object, 0xa8, &args![]).u16()
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(
            0x004a0030,
            bgs_decal_manager_update_simple_decals(Ptr<BGSDecalManager>)
        ),
        entry!(0x004a0330, fn_004a0330(Ptr) -> u32),
        entry!(0x004a0350, fn_004a0350(Ptr) -> bool),
        entry!(0x004a0370, fn_004a0370(Ptr)),
        entry!(0x004a03a0, fn_004a03a0(Ptr) -> u32),
        entry!(0x004a03c0, fn_004a03c0(Ptr)),
        entry!(0x004a03f0, fn_004a03f0(Ptr<BGSDecalManager>)),
        entry!(0x004a0490, fn_004a0490(Ptr, u32) -> Ptr),
        entry!(
            0x004a04c0,
            bgs_decal_manager_issue_decal_occlusion_query(Ptr<BGSDecalManager>, Ptr)
        ),
        entry!(0x004a0bd0, fn_004a0bd0(Ptr, Ptr) -> Ptr),
        entry!(0x004a0c10, fn_004a0c10(Ptr)),
        entry!(0x004a0c90, fn_004a0c90(Ptr, f32)),
        entry!(0x004a0d10, fn_004a0d10(Ptr) -> Ptr),
        entry!(0x004a0d30, fn_004a0d30(Ptr, Ptr) -> Ptr),
        entry!(0x004a0d60, fn_004a0d60(Ptr, Ptr) -> Ptr),
        entry!(0x004a0d90, fn_004a0d90(Ptr) -> f32),
        entry!(0x004a0db0, fn_004a0db0(Ptr) -> f32),
        entry!(0x004a0dd0, fn_004a0dd0(Ptr, u32)),
        entry!(0x004a0e10, fn_004a0e10(Ptr, u32, u8) -> bool),
        entry!(0x004a0e50, fn_004a0e50(Ptr) -> bool),
        entry!(0x004a0e90, fn_004a0e90() -> u32),
        entry!(0x004a0ea0, fn_004a0ea0() -> u32),
        entry!(0x004a0eb0, fn_004a0eb0(Ptr, u32) -> Ptr),
        entry!(0x004a0f50, bs_culling_process_get_rtti(Ptr) -> u32),
        entry!(0x004a0f60, bs_culling_process_dtor(Ptr)),
        entry!(0x004a0fd0, fn_004a0fd0(Ptr, u32)),
        entry!(0x004a0ff0, bs_culling_process_scalar_deleting_destructor(Ptr, u32) -> Ptr),
        entry!(0x004a1020, fn_004a1020(Ptr, u32)),
        entry!(0x004a1040, fn_004a1040(Ptr, u32)),
        entry!(0x004a1060, fn_004a1060(Ptr, u32) -> bool),
        entry!(
            0x004a10d0,
            bgs_decal_manager_add_decal(Ptr<BGSDecalManager>, Ptr<DecalPlacement>, u32, bool)
        ),
        entry!(0x004a19d0, fn_004a19d0(Ptr) -> bool),
        entry!(0x004a19f0, fn_004a19f0() -> f32),
        entry!(0x004a1a00, fn_004a1a00() -> f32),
        entry!(0x004a1a10, fn_004a1a10(Ptr, u32, u8)),
        entry!(0x004a1a30, fn_004a1a30(Ptr)),
        entry!(0x004a1a50, fn_004a1a50(u32)),
        entry!(
            0x004a1a70,
            bgs_decal_manager_add_geometry_decal_recurse(
                Ptr<BGSDecalManager>,
                Ptr<DecalPlacement>,
            ) -> bool
        ),
        entry!(0x004a1f90, fn_004a1f90(Ptr, Ptr, Ptr) -> Ptr),
        entry!(0x004a1ff0, fn_004a1ff0(Ptr) -> u16),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    /// Where the settings' values live: `slot(setting)` for each.
    const SETTING_SLOTS: u32 = 0x0300_0000;
    /// The object `GET_GLOBAL_OBJECT` returns.
    const GLOBAL_OBJECT: u32 = 0x0400_0000;
    /// A vtable whose slot `0x8c` is [`INITIALIZE`].
    const DECAL_VTABLE: u32 = 0x0410_0000;
    /// The decals' `Initialize`: finishes the decal when its `+0x2e` is set.
    const INITIALIZE: u32 = 0x0500_0001;

    fn ret(eax: u32) -> Ret {
        Ret {
            eax,
            ..Ret::default()
        }
    }

    fn ret_float(value: f32) -> Ret {
        Ret {
            st0: value as f64,
            ..Ret::default()
        }
    }

    fn slot(setting: u32) -> u32 {
        SETTING_SLOTS + (setting & 0xffff)
    }

    fn set_flag(e: &mut Engine, setting: u32, on: bool) {
        e.mem.set_u8(slot(setting), on as u8);
    }

    fn set_int(e: &mut Engine, setting: u32, value: i32) {
        e.mem.set_i32(slot(setting), value);
    }

    fn set_float(e: &mut Engine, setting: u32, value: f32) {
        e.mem.set_f32(slot(setting), value);
    }

    fn put_vec(e: &mut Engine, at: u32, v: [f32; 3]) {
        for (i, value) in v.iter().enumerate() {
            e.mem.set_f32(at + 4 * i as u32, *value);
        }
    }

    fn get_vec(e: &Engine, at: u32) -> [f32; 3] {
        [e.mem.f32(at), e.mem.f32(at + 4), e.mem.f32(at + 8)]
    }

    /// `values` as the words of `double` arguments.
    fn doubles(values: &[f32]) -> Vec<u32> {
        let mut words = vec![];
        for value in values {
            (*value as f64).put(&mut words);
        }
        words
    }

    /// The recorded calls to `address`, in order.
    fn calls(e: &Engine, address: u32) -> Vec<Vec<u32>> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .filter(|(a, _)| *a == address)
            .map(|(_, words)| words.clone())
            .collect()
    }

    /// The first word of every recorded call to `address`.
    fn first_words(e: &Engine, address: u32) -> Vec<u32> {
        calls(e, address).iter().map(|w| w[0]).collect()
    }

    /// A vtable at `at` whose slot at byte offset `offset` is `target`.
    fn vtable(e: &mut Engine, at: u32, offset: u32, target: u32) {
        let mut slots = vec![0u32; (offset / 4 + 1) as usize];
        slots[(offset / 4) as usize] = target;
        e.put_vtable(at, &slots);
    }

    /// An object of `size` bytes whose first word is `vtable`.
    fn object_with_vtable(e: &mut Engine, size: u32, vtable: u32) -> u32 {
        let object = e.mem.alloc(size);
        e.mem.set_u32(object, vtable);
        object
    }

    /// An `NiTListItem` chain for `elements` under the list header at
    /// `list` (head, tail, count); returns the node addresses.
    fn build_list(e: &mut Engine, list: u32, elements: &[u32]) -> Vec<u32> {
        let nodes: Vec<u32> = elements.iter().map(|_| e.mem.alloc(0x0c)).collect();
        for (i, node) in nodes.iter().enumerate() {
            let next = nodes.get(i + 1).copied().unwrap_or(0);
            let previous = if i == 0 { 0 } else { nodes[i - 1] };
            e.mem.set_u32(*node, next);
            e.mem.set_u32(*node + 4, previous);
            e.mem.set_u32(*node + 8, elements[i]);
        }
        e.mem.set_u32(list, nodes.first().copied().unwrap_or(0));
        e.mem.set_u32(list + 4, nodes.last().copied().unwrap_or(0));
        e.mem.set_u32(list + 8, elements.len() as u32);
        nodes
    }

    /// Every callee outside the file; each is a double that returns zero
    /// until [`rig`] gives the ones the tests rely on the game's behaviour.
    const CALLEES: &[u32] = &[
        FIRST_WORD,
        LIST_COUNT,
        LIST_ADVANCE,
        LIST_ADD_TAIL,
        LIST_ADD_HEAD,
        EMITTER_LIST_REMOVE,
        PENDING_LIST_REMOVE,
        LIST_CONTAINS,
        LIST_REMOVE_ALL,
        LIST_CONSTRUCTOR,
        LIST_LOCAL_DESTRUCTOR_BODY,
        NI_POINTER_RELEASE,
        NI_POINTER_ASSIGN,
        NI_POINTER_INIT,
        FREE,
        ALLOCATE,
        ALLOCATE_OBJECT,
        SETTING_VALUE_POINTER,
        SETTING_INT_POINTER,
        SETTING_FLOAT_VALUE,
        LOG,
        FORMAT,
        REPORT,
        STRNCMP,
        QUERY_PERFORMANCE_FREQUENCY,
        QUERY_PERFORMANCE_COUNTER,
        EMPTY_CONSTRUCTOR,
        POINT3_CONSTRUCTOR,
        POINT3_CONSTRUCTOR_ZERO,
        SIN_COS,
        POINT3_SCALE_IN_PLACE,
        POINT3_ADD,
        POINT3_SUBTRACT,
        POINT3_MULTIPLY,
        POINT3_LENGTH,
        MATRIX_MULTIPLY,
        MATRIX_GET_COLUMN,
        MATRIX_SET_COLUMN,
        ROTATION_Z_TO_VECTOR,
        CAMERA_DIRECTION,
        SET_TRANSLATE,
        SET_ROTATE,
        SET_VIEW_FRUSTUM,
        FRUSTUM_CONSTRUCTOR,
        UPDATE_WITH_ZERO,
        COLOR_CONSTRUCTOR,
        MAKE_DEBUG_LINE,
        ADD_TEMP_DEBUG_OBJECT,
        NODE_NAME_HOLDER,
        NAME_TEXT,
        CREATE_DEBUG_BOX,
        FINALIZE_GEOMETRY,
        QUERY_FINISHED,
        DECAL_CONSTRUCTOR,
        GET_GLOBAL_OBJECT,
        CREATE_QUERY_TEXTURE,
        STOP_OFFSCREEN,
        START_OFFSCREEN,
        QUERY_CONSTRUCTOR,
        QUERY_BEGIN,
        QUERY_END,
        ACCUMULATE_SCENE,
        RENDER_ACCUMULATED,
        END_OFFSCREEN,
        ACCUMULATOR_MODE,
        GET_WORLD_BOUND,
        BOUND_RADIUS,
        TARGET_SCALE,
        DECAL_ROTATION,
        DECAL_WIDTH_GETTER,
        FLOAT_MAX,
        DECAL_CASTER_CREATE,
        DECAL_CASTER_CAST,
        DECAL_CASTER_HIT_OBJECT,
        DECAL_CASTER_HIT_DATA,
        DECAL_CASTER_SET_TEXTURE_SET,
        OBJECT_TEXTURE_SET,
        FIND_REFERENCE_FOR_3D,
        CHILD_COUNT,
        CHILD_AT,
        SCOPE_BEGIN,
        SCOPE_END,
        EMITTER_DESTRUCTOR_BODY,
        EMITTER_UPDATE,
        EMITTER_FINISHED,
        ADD_PROJECTED_DECAL_RECURSE,
        MODEL_HAS_TAG,
        BOUND_IS_END,
        BOUND_NEXT,
        IS_MODEL_NODE,
        NODE_IS_HIDDEN,
        NODE_MODEL_DATA,
        MODEL_DATA_OBJECT,
        NODE_GEOMETRY_LIST,
        GEOMETRY_ACCEPTED,
        FLAG_TEST,
        REFERENCE_BASE_FORM,
        FORM_TYPE,
        COLLISION_CALL,
        SINGLETON_GETTER,
        SINGLETON_RELEASE,
        LOCK_ENTER,
        LOCK_LEAVE,
        LOCK_STATE,
        LOCK_EXTRA,
        CULLING_BASE_CONSTRUCTOR,
        CULLING_BASE_DESTRUCTOR,
        OBJECT_OF_NODE,
    ];

    /// An engine with the exe's constants and the pages the code reads
    /// mapped, every callee a double that returns zero, and the small
    /// helpers (list access, settings, vector arithmetic) behaving as the
    /// game's.
    fn rig() -> Engine {
        let mut e = Engine::new();
        for page in [
            0x011c_5000u32,
            0x011d_e000,
            0x011f_9000,
            0x011a_9000,
            0x0101_1000,
            0x0101_2000,
            0x0101_6000,
            0x0101_7000,
            0x0101_8000,
            0x0101_b000,
            0x0101_e000,
            GLOBAL_OBJECT,
        ] {
            e.map(page, 0x1000);
        }
        e.map(SETTING_SLOTS, 0x1_0000);
        e.set_global(TWO, 2.0f64);
        e.set_global(FAR_MARGIN, 50.0f64);
        e.set_global(NEAR_PLANE, 0.1f32);
        e.set_global(DIRECTION_SCALE, 50.0f32);
        e.set_global(DEBUG_LINE_SECONDS, 30.0f32);
        e.set_global(ZERO, 0.0f64);
        e.set_global(THOUSAND, 1000.0f64);
        e.set_global(MICROSECOND, 1e-6f64);
        e.set_global(MINIMUM_LENGTH, 1e-6f64);
        e.set_global(PROBE_LENGTH, 80.0f32);
        e.set_global(PROBE_RADIUS, 32.0f32);
        e.set_global(FALLBACK_SCALE, 2.0f32);
        e.set_global(DEBUG_BOX_ALPHA, 0.25f32);
        for (address, text) in [
            (0x0101_e484u32, &b"Decal Node"[..]),
            (0x0101_e47c, b"Decal"),
            (0x0101_e474, b"FaceGen"),
            (0x0101_e468, b"BSFaceGen"),
            (0x0101_e460, b"Bip01"),
            (0x0101_e450, b"Debug Decal Box"),
        ] {
            e.mem.set_cstr(address, text);
        }
        for address in CALLEES {
            e.register(*address, |_, _| Ret::default());
        }
        e.register(FIRST_WORD, |e, a| ret(e.mem.u32(a[0])));
        e.register(LIST_COUNT, |e, a| ret(e.mem.u32(a[0] + 8)));
        e.register(LIST_ADVANCE, |e, a| {
            let node = e.mem.u32(a[1]);
            let next = e.mem.u32(node);
            e.mem.set_u32(a[1], next);
            ret(node + 8)
        });
        e.register(EMPTY_CONSTRUCTOR, |_, a| ret(a[0]));
        e.register(SETTING_VALUE_POINTER, |_, a| ret(slot(a[0])));
        e.register(SETTING_INT_POINTER, |_, a| ret(slot(a[0])));
        e.register(SETTING_FLOAT_VALUE, |e, a| ret_float(e.mem.f32(slot(a[0]))));
        e.register(GET_GLOBAL_OBJECT, |_, _| ret(GLOBAL_OBJECT));
        for address in [NI_POINTER_ASSIGN, NI_POINTER_INIT] {
            e.register(address, |e, a| {
                e.mem.set_u32(a[0], a[1]);
                ret(a[0])
            });
        }
        e.register(POINT3_CONSTRUCTOR, |e, a| {
            for i in 0..3 {
                e.mem.set_u32(a[0] + 4 * i, a[1 + i as usize]);
            }
            ret(a[0])
        });
        e.register(POINT3_ADD, |e, a| {
            for i in 0..3 {
                let sum = e.mem.f32(a[0] + 4 * i) + e.mem.f32(a[2] + 4 * i);
                e.mem.set_f32(a[1] + 4 * i, sum);
            }
            ret(a[1])
        });
        e.register(POINT3_SUBTRACT, |e, a| {
            for i in 0..3 {
                let difference = e.mem.f32(a[0] + 4 * i) - e.mem.f32(a[2] + 4 * i);
                e.mem.set_f32(a[1] + 4 * i, difference);
            }
            ret(a[1])
        });
        e.register(POINT3_MULTIPLY, |e, a| {
            for i in 0..3 {
                let product = e.mem.f32(a[0] + 4 * i) * f32::from_bits(a[2]);
                e.mem.set_f32(a[1] + 4 * i, product);
            }
            ret(a[1])
        });
        e.register(POINT3_SCALE_IN_PLACE, |e, a| {
            for i in 0..3 {
                let product = e.mem.f32(a[0] + 4 * i) * f32::from_bits(a[1]);
                e.mem.set_f32(a[0] + 4 * i, product);
            }
            ret(a[0])
        });
        e.register(POINT3_LENGTH, |e, a| {
            let v = [e.mem.f32(a[0]), e.mem.f32(a[0] + 4), e.mem.f32(a[0] + 8)];
            ret_float((v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt())
        });
        e.register(COLOR_CONSTRUCTOR, |_, a| ret(a[0]));
        e.register(MATRIX_MULTIPLY, |_, a| ret(a[1]));
        e.register(CAMERA_DIRECTION, |_, a| ret(a[1]));
        e.register(TARGET_SCALE, |_, _| ret_float(1.0));
        e.register(DECAL_WIDTH_GETTER, |e, a| {
            ret_float(e.mem.f32(a[0] + 0x118))
        });
        e.register(STRNCMP, |e, a| {
            let first = e.mem.cstr(a[0]);
            let second = e.mem.cstr(a[1]);
            let n = a[2] as usize;
            let first = &first[..first.len().min(n)];
            let second = &second[..second.len().min(n)];
            ret(match first.cmp(second) {
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Less => u32::MAX,
                std::cmp::Ordering::Greater => 1,
            })
        });
        e.register(GET_WORLD_BOUND, |_, _| ret(GLOBAL_OBJECT + 0x800));
        vtable(&mut e, DECAL_VTABLE, DECAL_VSLOT_INITIALIZE, INITIALIZE);
        e.register(INITIALIZE, |e, a| {
            if e.mem.u8(a[0] + 0x2e) != 0 {
                e.mem.set_u8(a[0] + DECAL_FINISHED, 1);
            }
            Ret::default()
        });
        e.call_log = Some(vec![]);
        e
    }

    /// A manager with an accumulator and a camera.
    fn new_manager(e: &mut Engine) -> Ptr<BGSDecalManager> {
        let manager: Ptr<BGSDecalManager> = e.new_object();
        let accumulator = e.mem.alloc(0x300);
        let camera = e.mem.alloc(0x300);
        e.set(
            manager,
            BGSDecalManager::spQueryAccum,
            Ptr::new(accumulator),
        );
        e.set(manager, BGSDecalManager::spQueryCamera, Ptr::new(camera));
        manager
    }

    /// A `BSTempEffectSimpleDecal` sized block with the decal vtable.
    fn new_decal(e: &mut Engine) -> u32 {
        object_with_vtable(e, DECAL_SIZE, DECAL_VTABLE)
    }

    /// A placement with `target` as its target node.
    fn new_placement(e: &mut Engine, target: u32) -> Ptr<DecalPlacement> {
        let placement: Ptr<DecalPlacement> = e.new_object();
        e.set(placement, DecalPlacement::target, Ptr::new(target));
        placement
    }

    // -----------------------------------------------------------------
    // Small functions

    #[test]
    fn target_node_is_read_through_the_ni_pointer() {
        let mut e = rig();
        let decal = new_decal(&mut e);
        e.mem.set_u32(decal + 0x10c, 0x1234);
        assert_eq!(e.call(0x004a0330, &args![decal]).u32(), 0x1234);
        assert_eq!(calls(&e, FIRST_WORD), vec![vec![decal + 0x10c]]);
    }

    #[test]
    fn valid_flag_is_the_byte_at_0x2d() {
        let mut e = rig();
        let decal = new_decal(&mut e);
        assert!(!e.call(0x004a0350, &args![decal]).bool());
        e.mem.set_u8(decal + 0x2d, 1);
        assert!(e.call(0x004a0350, &args![decal]).bool());
    }

    #[test]
    fn lock_extra_runs_only_when_the_state_is_one() {
        let mut e = rig();
        let object = e.mem.alloc(0x100);
        e.register(LOCK_STATE, |_, _| ret(1));
        e.call(0x004a0370, &args![object]);
        assert_eq!(calls(&e, LOCK_ENTER), vec![vec![object + 0x80]]);
        assert_eq!(calls(&e, LOCK_EXTRA), vec![vec![object]]);

        let mut e = rig();
        e.call(0x004a0370, &args![object]);
        assert_eq!(calls(&e, LOCK_ENTER).len(), 1);
        assert!(calls(&e, LOCK_EXTRA).is_empty());
    }

    #[test]
    fn lock_state_is_read_at_0x80() {
        let mut e = rig();
        let object = e.mem.alloc(0x100);
        e.register(LOCK_STATE, |_, _| ret(7));
        assert_eq!(e.call(0x004a03a0, &args![object]).u32(), 7);
        assert_eq!(calls(&e, LOCK_STATE), vec![vec![object + 0x80]]);
    }

    #[test]
    fn unlock_leaves_after_the_extra_call() {
        let mut e = rig();
        let object = e.mem.alloc(0x100);
        e.register(LOCK_STATE, |_, _| ret(1));
        e.call(0x004a03c0, &args![object]);
        let order: Vec<u32> = e
            .call_log
            .as_ref()
            .unwrap()
            .iter()
            .map(|(a, _)| *a)
            .filter(|a| [LOCK_EXTRA, LOCK_LEAVE].contains(a))
            .collect();
        assert_eq!(order, vec![LOCK_EXTRA, LOCK_LEAVE]);
        assert_eq!(calls(&e, LOCK_LEAVE), vec![vec![object + 0x80]]);

        let mut e = rig();
        e.call(0x004a03c0, &args![object]);
        assert!(calls(&e, LOCK_EXTRA).is_empty());
        assert_eq!(calls(&e, LOCK_LEAVE).len(), 1);
    }

    #[test]
    fn finished_emitters_are_removed_and_destroyed() {
        let mut e = rig();
        let manager = new_manager(&mut e);
        let list = manager.addr() + 0x14;
        let (first, second) = (0x1000u32, 0x2000u32);
        let nodes = build_list(&mut e, list, &[first, 0, second]);
        e.register_double(EMITTER_FINISHED, move |_, a| ret((a[0] == first) as u32));
        let removed: Rc<RefCell<Vec<(u32, u32)>>> = Rc::default();
        let seen = removed.clone();
        e.register_double(EMITTER_LIST_REMOVE, move |e, a| {
            seen.borrow_mut().push((a[0], e.mem.u32(a[1])));
            Ret::default()
        });
        e.call(0x004a03f0, &args![manager]);
        assert_eq!(first_words(&e, EMITTER_UPDATE), vec![first, second]);
        assert_eq!(*removed.borrow(), vec![(list, nodes[0])]);
        assert_eq!(first_words(&e, EMITTER_DESTRUCTOR_BODY), vec![first]);
        assert_eq!(first_words(&e, FREE), vec![first]);
    }

    #[test]
    fn emitter_destructor_frees_only_when_asked() {
        let mut e = rig();
        let emitter = e.mem.alloc(0x10);
        assert_eq!(e.call(0x004a0490, &args![emitter, 1u32]).u32(), emitter);
        assert_eq!(first_words(&e, EMITTER_DESTRUCTOR_BODY), vec![emitter]);
        assert_eq!(first_words(&e, FREE), vec![emitter]);
        e.call(0x004a0490, &args![emitter, 0u32]);
        assert_eq!(calls(&e, FREE).len(), 1);
    }

    #[test]
    fn negation_goes_through_the_point_constructor() {
        let mut e = rig();
        let source = e.mem.alloc(12);
        let result = e.mem.alloc(12);
        put_vec(&mut e, source, [1.0, -2.0, 3.0]);
        assert_eq!(e.call(0x004a0bd0, &args![source, result]).u32(), result);
        assert_eq!(get_vec(&e, result), [-1.0, 2.0, -3.0]);
        assert_eq!(calls(&e, POINT3_CONSTRUCTOR)[0][0], result);
    }

    #[test]
    fn normalizing_scales_or_zeroes() {
        let mut e = rig();
        let v = e.mem.alloc(12);
        put_vec(&mut e, v, [3.0, 0.0, 4.0]);
        e.call(0x004a0c10, &args![v]);
        let n = get_vec(&e, v);
        assert!((n[0] - 0.6).abs() < 1e-6 && n[1] == 0.0 && (n[2] - 0.8).abs() < 1e-6);
        put_vec(&mut e, v, [1e-8, 0.0, 0.0]);
        e.call(0x004a0c10, &args![v]);
        assert_eq!(get_vec(&e, v), [0.0, 0.0, 0.0]);
        // A length exactly at the minimum is not above it.
        e.set_global(MINIMUM_LENGTH, 5.0f64);
        put_vec(&mut e, v, [3.0, 0.0, 4.0]);
        e.call(0x004a0c10, &args![v]);
        assert_eq!(get_vec(&e, v), [0.0, 0.0, 0.0]);
    }

    #[test]
    fn rotation_about_z_uses_sin_and_cos() {
        let mut e = rig();
        e.register(SIN_COS, |e, a| {
            e.mem.set_f32(a[1], 0.5);
            e.mem.set_f32(a[2], 0.75);
            Ret::default()
        });
        let matrix = e.mem.alloc(36);
        e.call(0x004a0c90, &args![matrix, 1.25f32]);
        assert_eq!(calls(&e, SIN_COS)[0][0], 1.25f32.to_bits());
        let words: Vec<f32> = (0..9).map(|i| e.mem.f32(matrix + 4 * i)).collect();
        assert_eq!(words, vec![0.75, 0.5, 0.0, -0.5, 0.75, 0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn camera_frustum_is_at_0x100() {
        let mut e = rig();
        assert_eq!(e.call(0x004a0d10, &args![0x5000u32]).u32(), 0x5100);
    }

    #[test]
    fn decal_vectors_are_copied_out() {
        let mut e = rig();
        let decal = new_decal(&mut e);
        let out = e.mem.alloc(12);
        put_vec(&mut e, decal + 0x34, [1.0, 2.0, 3.0]);
        put_vec(&mut e, decal + 0x40, [4.0, 5.0, 6.0]);
        assert_eq!(e.call(0x004a0d30, &args![decal, out]).u32(), out);
        assert_eq!(get_vec(&e, out), [1.0, 2.0, 3.0]);
        assert_eq!(e.call(0x004a0d60, &args![decal, out]).u32(), out);
        assert_eq!(get_vec(&e, out), [4.0, 5.0, 6.0]);
    }

    #[test]
    fn decal_height_and_depth_are_floats_in_st0() {
        let mut e = rig();
        let decal = new_decal(&mut e);
        e.mem.set_f32(decal + 0x11c, 2.5);
        e.mem.set_f32(decal + 0x120, 7.0);
        assert_eq!(e.call(0x004a0d90, &args![decal]).f32(), 2.5);
        assert_eq!(e.call(0x004a0db0, &args![decal]).f32(), 7.0);
    }

    #[test]
    fn screen_space_camera_data_needs_the_matching_mode() {
        let target = 0x0600_0001u32;
        let mut e = rig();
        vtable(&mut e, 0x0420_0000, 0x190, target);
        let object = object_with_vtable(&mut e, 0x300, 0x0420_0000);
        e.mem.set_u32(object + 0x200, 2);
        e.mem.set_u8(object + 0x208, 1);
        e.register(target, |_, _| ret(0));
        e.call(0x004a0dd0, &args![object, 0x7700u32]);
        assert_eq!(calls(&e, target), vec![vec![object, 0x7700]]);

        // Wrong byte: no call.
        let mut e = rig();
        vtable(&mut e, 0x0420_0000, 0x190, target);
        let object = object_with_vtable(&mut e, 0x300, 0x0420_0000);
        e.mem.set_u32(object + 0x200, 1);
        e.mem.set_u8(object + 0x208, 0);
        e.register(target, |_, _| ret(0));
        e.call(0x004a0dd0, &args![object, 0x7700u32]);
        assert!(calls(&e, target).is_empty());
    }

    #[test]
    fn mode_check_compares_the_byte_when_the_mode_is_1_or_2() {
        let mut e = rig();
        let object = e.mem.alloc(0x300);
        for (mode, byte, expected) in [
            (0u32, 1u8, false),
            (1, 1, true),
            (2, 1, true),
            (3, 1, false),
            (1, 0, false),
        ] {
            e.mem.set_u32(object + 0x200, mode);
            e.mem.set_u8(object + 0x208, byte);
            assert_eq!(
                e.call(0x004a0e10, &args![object, 0x1234u32, 1u8]).bool(),
                expected,
                "mode {mode} byte {byte}"
            );
        }
    }

    #[test]
    fn mode_is_one_or_two() {
        let mut e = rig();
        let object = e.mem.alloc(0x300);
        for (mode, expected) in [(0u32, false), (1, true), (2, true), (3, false)] {
            e.mem.set_u32(object + 0x200, mode);
            assert_eq!(e.call(0x004a0e50, &args![object]).bool(), expected);
        }
    }

    #[test]
    fn query_texture_globals() {
        let mut e = rig();
        e.mem.set_u32(QUERY_TEXTURE_FORMAT, 0x42);
        e.set_global(TEXTURE_MANAGER, 0x9999u32);
        assert_eq!(e.call(0x004a0e90, &args![]).u32(), 0x42);
        assert_eq!(calls(&e, FIRST_WORD), vec![vec![QUERY_TEXTURE_FORMAT]]);
        assert_eq!(e.call(0x004a0ea0, &args![]).u32(), 0x9999);
    }

    #[test]
    fn culling_process_constructor() {
        let mut e = rig();
        let process = e.mem.alloc(0x100);
        for offset in [0x90, 0xbc, 0xc0] {
            e.mem.set_u32(process + offset, 0xffff);
        }
        assert_eq!(e.call(0x004a0eb0, &args![process, 5u32]).u32(), process);
        assert_eq!(calls(&e, CULLING_BASE_CONSTRUCTOR), vec![vec![process, 5]]);
        assert_eq!(e.mem.u32(process), CULLING_PROCESS_VTABLE);
        for offset in [0x90, 0xbc, 0xc0] {
            assert_eq!(e.mem.u32(process + offset), 0);
        }
        assert_eq!(calls(&e, NI_POINTER_INIT), vec![vec![process + 0xc4, 0]]);
    }

    #[test]
    fn culling_process_rtti() {
        let mut e = rig();
        assert_eq!(e.call(0x004a0f50, &args![0x1000u32]).u32(), 0x0120_30a8);
    }

    #[test]
    fn culling_process_destructor() {
        let mut e = rig();
        let process = e.mem.alloc(0x100);
        e.call(0x004a0f60, &args![process]);
        assert_eq!(e.mem.u32(process), CULLING_PROCESS_VTABLE);
        assert_eq!(calls(&e, NI_POINTER_RELEASE), vec![vec![process + 0xc4]]);
        assert_eq!(calls(&e, CULLING_BASE_DESTRUCTOR), vec![vec![process]]);
    }

    #[test]
    fn culling_process_accumulator_is_assigned_at_0xc4() {
        let mut e = rig();
        let process = e.mem.alloc(0x100);
        e.call(0x004a0fd0, &args![process, 0x7777u32]);
        assert_eq!(
            calls(&e, NI_POINTER_ASSIGN),
            vec![vec![process + 0xc4, 0x7777]]
        );
        assert_eq!(e.mem.u32(process + 0xc4), 0x7777);
    }

    #[test]
    fn culling_process_scalar_deleting_destructor() {
        let mut e = rig();
        let process = e.mem.alloc(0x100);
        assert_eq!(e.call(0x004a0ff0, &args![process, 1u32]).u32(), process);
        assert_eq!(first_words(&e, FREE), vec![process]);
        assert_eq!(calls(&e, CULLING_BASE_DESTRUCTOR).len(), 1);
        e.call(0x004a0ff0, &args![process, 0u32]);
        assert_eq!(calls(&e, FREE).len(), 1);
        assert_eq!(calls(&e, CULLING_BASE_DESTRUCTOR).len(), 2);
    }

    #[test]
    fn accumulator_fields() {
        let mut e = rig();
        let accumulator = e.mem.alloc(0x300);
        e.call(0x004a1020, &args![accumulator, 0x11u32]);
        e.call(0x004a1040, &args![accumulator, 0x22u32]);
        assert_eq!(e.mem.u32(accumulator + 0x194), 0x11);
        assert_eq!(e.mem.u32(accumulator + 0x19c), 0x22);
    }

    #[test]
    fn some_form_types_refuse_decals() {
        let mut e = rig();
        e.register(REFERENCE_BASE_FORM, |_, _| ret(0x9999));
        let types = Rc::new(RefCell::new(0u32));
        let current = types.clone();
        e.register_double(FORM_TYPE, move |_, _| ret(*current.borrow()));
        for (form_type, expected) in [
            (0x23u32, false),
            (0x2a, false),
            (0x2b, false),
            (0x33, false),
            (0x24, true),
            (0x00, true),
        ] {
            *types.borrow_mut() = form_type;
            assert_eq!(
                e.call(0x004a1060, &args![0x5000u32, 1u32]).bool(),
                expected,
                "form type {form_type:#x}"
            );
        }
        assert_eq!(calls(&e, FORM_TYPE)[0], vec![0x9999]);
    }

    #[test]
    fn flag_test_uses_the_mask_0x400() {
        let mut e = rig();
        e.register(FLAG_TEST, |_, a| ret((a[1] == 0x400) as u32));
        assert!(e.call(0x004a19d0, &args![0x1000u32]).bool());
        assert_eq!(calls(&e, FLAG_TEST), vec![vec![0x1000, 0x400]]);
    }

    #[test]
    fn distance_limits_are_float_settings() {
        let mut e = rig();
        set_float(&mut e, SETTING_GEOMETRY_DISTANCE, 12.5);
        set_float(&mut e, SETTING_PROJECTED_DISTANCE, 3.5);
        assert_eq!(e.call(0x004a19f0, &args![]).f32(), 12.5);
        assert_eq!(e.call(0x004a1a00, &args![]).f32(), 3.5);
        assert_eq!(
            first_words(&e, SETTING_FLOAT_VALUE),
            vec![SETTING_GEOMETRY_DISTANCE, SETTING_PROJECTED_DISTANCE]
        );
    }

    #[test]
    fn collision_call_forwards_with_0x27() {
        let mut e = rig();
        e.call(0x004a1a10, &args![0x1000u32, 8u32, 1u8]);
        assert_eq!(calls(&e, COLLISION_CALL), vec![vec![0x27, 8, 1]]);
    }

    #[test]
    fn local_list_destructor_body_is_called() {
        let mut e = rig();
        e.call(0x004a1a30, &args![0x1000u32]);
        assert_eq!(calls(&e, LIST_LOCAL_DESTRUCTOR_BODY), vec![vec![0x1000]]);
    }

    #[test]
    fn singleton_is_asked_with_the_address_of_the_parameter() {
        let mut e = rig();
        let seen: Rc<RefCell<Vec<u32>>> = Rc::default();
        let log = seen.clone();
        e.register_double(SINGLETON_GETTER, move |e, a| {
            log.borrow_mut().push(e.mem.u32(a[0]));
            ret(0x5000)
        });
        e.call(0x004a1a50, &args![0x1234u32]);
        assert_eq!(*seen.borrow(), vec![0x1234]);
        assert_eq!(calls(&e, SINGLETON_RELEASE), vec![vec![0x5014]]);

        let mut e = rig();
        e.call(0x004a1a50, &args![0u32]);
        assert!(calls(&e, SINGLETON_GETTER).is_empty());
    }

    #[test]
    fn component_product_goes_through_the_point_constructor() {
        let mut e = rig();
        let (a, b, out) = (e.mem.alloc(12), e.mem.alloc(12), e.mem.alloc(12));
        put_vec(&mut e, a, [1.0, 2.0, 3.0]);
        put_vec(&mut e, b, [4.0, 5.0, 6.0]);
        assert_eq!(e.call(0x004a1f90, &args![out, a, b]).u32(), out);
        assert_eq!(get_vec(&e, out), [4.0, 10.0, 18.0]);
    }

    #[test]
    fn query_size_is_the_low_word_of_the_virtual_result() {
        let mut e = rig();
        let target = 0x0600_0002u32;
        vtable(&mut e, 0x0430_0000, 0xa8, target);
        let object = object_with_vtable(&mut e, 0x40, 0x0430_0000);
        e.register_double(OBJECT_OF_NODE, move |_, _| ret(object));
        e.register(target, |_, _| ret(0x0001_2345));
        assert_eq!(e.call(0x004a1ff0, &args![0x1000u32]).u32(), 0x2345);
        assert_eq!(calls(&e, OBJECT_OF_NODE), vec![vec![0x1000]]);
        assert_eq!(calls(&e, target), vec![vec![object]]);
    }

    // -----------------------------------------------------------------
    // UpdateSimpleDecals and IssueDecalOcclusionQuery

    #[test]
    fn an_empty_pending_list_does_nothing() {
        let mut e = rig();
        let manager = new_manager(&mut e);
        e.call(0x004a0030, &args![manager]);
        assert!(calls(&e, LOCK_ENTER).is_empty());
        assert!(calls(&e, GET_GLOBAL_OBJECT).is_empty());
    }

    #[test]
    fn pending_decals_are_queried_placed_or_discarded() {
        let mut e = rig();
        let manager = new_manager(&mut e);
        let pending = manager.addr() + 8;
        // D1 wants an occlusion query (the query is not created by the
        // doubles, so it is initialized at once but stays unfinished).
        let d1 = new_decal(&mut e);
        e.mem.set_u8(d1 + 0x1a, 1);
        // D2 has a finished query with no samples: discarded as invalid.
        let d2 = new_decal(&mut e);
        e.mem.set_u32(d2 + 0x1c, 0x777);
        // D3 has no query: initialized, finishes and is valid.
        let d3 = new_decal(&mut e);
        e.mem.set_u8(d3 + 0x2e, 1);
        e.mem.set_u8(d3 + 0x2d, 1);
        let nodes = build_list(&mut e, pending, &[d1, d2, d3]);
        e.register(QUERY_FINISHED, |_, _| ret(1));
        set_flag(&mut e, SETTING_DEBUG_LOG, true);
        e.set_global(DECAL_DEBUG_FLAGS, 0x13u32);
        e.set_global(DECAL_COUNT, 4u32);
        let removed: Rc<RefCell<Vec<(u32, u32)>>> = Rc::default();
        let seen = removed.clone();
        e.register_double(PENDING_LIST_REMOVE, move |e, a| {
            seen.borrow_mut().push((a[0], e.mem.u32(a[2])));
            Ret::default()
        });

        e.call(0x004a0030, &args![manager]);

        assert_eq!(e.mem.u8(d1 + 0x1a), 0, "the query request is cleared");
        assert_eq!(calls(&e, QUERY_BEGIN).len(), 1, "one query was issued");
        assert_eq!(first_words(&e, INITIALIZE), vec![d1, d3]);
        assert_eq!(e.mem.u8(d2 + DECAL_FINISHED), 1);
        assert_eq!(
            *removed.borrow(),
            vec![(pending, nodes[1]), (pending, nodes[2])]
        );
        // Two removals and the culling process of the issued query.
        assert_eq!(calls(&e, NI_POINTER_RELEASE).len(), 3);
        assert_eq!(e.global::<u32>(DECAL_COUNT), 5);

        let log: Vec<u32> = first_words(&e, LOG);
        assert_eq!(
            log,
            vec![
                TEXT_ISSUING,
                TEXT_ORIGIN,
                TEXT_DIRECTION,
                TEXT_FRUSTUM,
                TEXT_QUERY_FAILED,
                TEXT_PLACING,
            ]
        );
        assert_eq!(calls(&e, LOG)[5], vec![TEXT_PLACING, 5, 0]);

        // The invalid decal gets red boxes, the valid one green boxes.
        let colours = calls(&e, COLOR_CONSTRUCTOR);
        let rgba = |r: f32, g: f32, b: f32, a: f32| {
            vec![r.to_bits(), g.to_bits(), b.to_bits(), a.to_bits()]
        };
        let colour_words: Vec<Vec<u32>> = colours.iter().map(|w| w[1..].to_vec()).collect();
        assert_eq!(
            colour_words,
            vec![
                rgba(1.0, 0.0, 0.0, 1.0),
                rgba(1.0, 0.0, 0.0, 0.25),
                rgba(0.0, 1.0, 0.0, 1.0),
                rgba(0.0, 1.0, 0.0, 0.25),
            ]
        );
        let boxes = calls(&e, CREATE_DEBUG_BOX);
        let shapes: Vec<(u32, u32)> = boxes.iter().map(|w| (w[0], w[2])).collect();
        assert_eq!(shapes, vec![(d2, 1), (d2, 0), (d3, 1), (d3, 0)]);

        // The lock is taken first and left last.
        let order: Vec<u32> = e
            .call_log
            .as_ref()
            .unwrap()
            .iter()
            .map(|(a, _)| *a)
            .filter(|a| [LOCK_ENTER, LOCK_LEAVE].contains(a))
            .collect();
        assert_eq!(order, vec![LOCK_ENTER, LOCK_LEAVE]);
    }

    #[test]
    fn failed_boxes_need_the_display_failed_flag_and_valid_ones_do_not() {
        let mut e = rig();
        let manager = new_manager(&mut e);
        let pending = manager.addr() + 8;
        let invalid = new_decal(&mut e);
        e.mem.set_u8(invalid + DECAL_FINISHED, 1);
        let valid = new_decal(&mut e);
        e.mem.set_u8(valid + DECAL_FINISHED, 1);
        e.mem.set_u8(valid + DECAL_VALID, 1);
        build_list(&mut e, pending, &[invalid, valid]);
        e.set_global(DECAL_DEBUG_FLAGS, 0x01u32);
        e.call(0x004a0030, &args![manager]);
        let boxes = calls(&e, CREATE_DEBUG_BOX);
        assert_eq!(boxes.len(), 1, "only the valid decal gets its box");
        assert_eq!(boxes[0][0], valid);
        assert_eq!(calls(&e, PENDING_LIST_REMOVE).len(), 2);
    }

    #[test]
    fn a_decal_waiting_to_be_finalized_is_finalized_unless_finished() {
        let mut e = rig();
        let manager = new_manager(&mut e);
        let pending = manager.addr() + 8;
        let waiting = new_decal(&mut e);
        e.mem.set_u8(waiting + DECAL_FINALIZE, 1);
        let finished = new_decal(&mut e);
        e.mem.set_u8(finished + DECAL_FINALIZE, 1);
        e.mem.set_u8(finished + DECAL_FINISHED, 1);
        build_list(&mut e, pending, &[waiting, finished]);
        e.call(0x004a0030, &args![manager]);
        assert_eq!(first_words(&e, FINALIZE_GEOMETRY), vec![waiting]);
    }

    #[test]
    fn a_finished_query_with_samples_places_the_decal() {
        let mut e = rig();
        let manager = new_manager(&mut e);
        let pending = manager.addr() + 8;
        let decal = new_decal(&mut e);
        e.mem.set_u32(decal + 0x1c, 0x777);
        e.mem.set_u32(decal + 0x24, 12);
        build_list(&mut e, pending, &[decal]);
        e.register(QUERY_FINISHED, |_, _| ret(1));
        set_flag(&mut e, SETTING_DEBUG_LOG, true);
        e.call(0x004a0030, &args![manager]);
        assert_eq!(first_words(&e, INITIALIZE), vec![decal]);
        assert_eq!(first_words(&e, LOG), vec![TEXT_QUERY_SUCCEEDED]);

        // An unfinished query leaves the decal alone.
        let mut e = rig();
        let manager = new_manager(&mut e);
        let decal = new_decal(&mut e);
        e.mem.set_u32(decal + 0x1c, 0x777);
        build_list(&mut e, manager.addr() + 8, &[decal]);
        e.call(0x004a0030, &args![manager]);
        assert!(calls(&e, INITIALIZE).is_empty());
        assert!(calls(&e, PENDING_LIST_REMOVE).is_empty());
    }

    /// A manager and a decal set up for the occlusion query tests: origin
    /// (1, 2, 3), direction (0, 0, 2), width 4, height 2, depth 10, the
    /// target scale 2 and the target node `0x2222`.
    fn query_scene(e: &mut Engine) -> (Ptr<BGSDecalManager>, u32) {
        let manager = new_manager(e);
        let decal = new_decal(e);
        put_vec(e, decal + 0x34, [1.0, 2.0, 3.0]);
        put_vec(e, decal + 0x40, [0.0, 0.0, 2.0]);
        e.mem.set_f32(decal + 0x118, 4.0);
        e.mem.set_f32(decal + 0x11c, 2.0);
        e.mem.set_f32(decal + 0x120, 10.0);
        e.mem.set_u32(decal + 0x10c, 0x2222);
        e.register(TARGET_SCALE, |_, _| ret_float(2.0));
        (manager, decal)
    }

    #[test]
    fn the_query_camera_is_placed_and_the_scene_rendered() {
        let mut e = rig();
        let (manager, decal) = query_scene(&mut e);
        e.mem.set_u8(decal + 0x1b, 1);
        e.mem.set_u32(QUERY_TEXTURE_FORMAT, 0x42);
        e.set_global(TEXTURE_MANAGER, 0x6000u32);
        e.register(CREATE_QUERY_TEXTURE, |_, _| ret(0xabc0));
        e.register(STOP_OFFSCREEN, |_, a| ret(a[0] + 1));
        e.register(ALLOCATE, |_, _| ret(0x5000));
        e.register(QUERY_CONSTRUCTOR, |_, a| ret(a[0] + 0x10));
        e.register(ACCUMULATOR_MODE, |_, _| ret(9));
        e.register(DECAL_ROTATION, |_, _| ret_float(0.5));
        let frustum: Rc<RefCell<Vec<f32>>> = Rc::default();
        let seen = frustum.clone();
        e.register_double(SET_VIEW_FRUSTUM, move |e, a| {
            for i in 0..6 {
                seen.borrow_mut().push(e.mem.f32(a[1] + 4 * i));
            }
            seen.borrow_mut().push(e.mem.u8(a[1] + 0x18) as f32);
            Ret::default()
        });
        let position: Rc<RefCell<[f32; 3]>> = Rc::default();
        let seen = position.clone();
        e.register_double(SET_TRANSLATE, move |e, a| {
            *seen.borrow_mut() = get_vec(e, a[1]);
            Ret::default()
        });
        let rotation_vector: Rc<RefCell<Vec<f32>>> = Rc::default();
        let seen = rotation_vector.clone();
        e.register_double(ROTATION_Z_TO_VECTOR, move |_, a| {
            *seen.borrow_mut() = a[1..].iter().map(|w| f32::from_bits(*w)).collect();
            Ret::default()
        });

        e.call(0x004a04c0, &args![manager, decal]);

        let camera = e.get(manager, BGSDecalManager::spQueryCamera).addr();
        let accumulator = e.get(manager, BGSDecalManager::spQueryAccum).addr();
        // Direction (0, 0, 2) is normalized, scaled by 50 and added to the
        // origin; the rotation is of the negated scaled direction.
        assert_eq!(*position.borrow(), [1.0, 2.0, 53.0]);
        assert_eq!(*rotation_vector.borrow(), vec![0.0, 0.0, -50.0]);
        // Half width 4/2 and half height 2/2 over the target scale 2, near
        // 0.1, far 10 + 50, orthographic.
        assert_eq!(
            *frustum.borrow(),
            vec![-1.0, 1.0, 0.5, -0.5, 0.1, 60.0, 1.0]
        );
        assert_eq!(calls(&e, FRUSTUM_CONSTRUCTOR)[0][1], 0);
        assert_eq!(calls(&e, DECAL_ROTATION), vec![vec![decal]]);
        assert_eq!(calls(&e, SIN_COS)[0][0], 0.5f32.to_bits());

        // The camera's rotation columns: read 0, 1, 2 of the product and
        // written back as 2 -> 0, 0 -> 1, 1 -> 2.
        let reads = calls(&e, MATRIX_GET_COLUMN);
        let writes = calls(&e, MATRIX_SET_COLUMN);
        assert_eq!(
            reads.iter().map(|w| w[1]).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert_eq!(
            writes.iter().map(|w| w[1]).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert_eq!(writes[0][2], reads[2][2]);
        assert_eq!(writes[1][2], reads[0][2]);
        assert_eq!(writes[2][2], reads[1][2]);

        // The texture is created, the render started to target 7 (clear),
        // and the query begun and ended.
        assert_eq!(
            calls(&e, CREATE_QUERY_TEXTURE),
            vec![vec![0x6000, 0x42, 0x33, 0, 0, 0]]
        );
        assert_eq!(
            calls(&e, NI_POINTER_ASSIGN)[0],
            vec![manager.addr(), 0xabc0]
        );
        assert_eq!(calls(&e, START_OFFSCREEN), vec![vec![7, 0xabc1]]);
        assert_eq!(calls(&e, ALLOCATE), vec![vec![4]]);
        assert_eq!(e.mem.u32(decal + 0x1c), 0x5010);
        assert_eq!(calls(&e, QUERY_BEGIN), vec![vec![0x5010]]);
        assert_eq!(calls(&e, QUERY_END), vec![vec![0x5010]]);
        assert_eq!(calls(&e, END_OFFSCREEN).len(), 1);

        // The accumulator is set up and the scene accumulated and rendered.
        assert_eq!(e.mem.u32(accumulator + 0x194), 9);
        assert_eq!(e.mem.u32(accumulator + 0x19c), 0xf);
        let scene = calls(&e, ACCUMULATE_SCENE);
        assert_eq!(scene.len(), 1);
        assert_eq!(&scene[0][..2], &[camera, 0x2222]);
        let culling = scene[0][2];
        assert_eq!(calls(&e, CULLING_BASE_CONSTRUCTOR), vec![vec![culling, 0]]);
        assert_eq!(
            calls(&e, NI_POINTER_ASSIGN)[1..].to_vec(),
            vec![vec![culling + 0xc4, accumulator], vec![culling + 0xc4, 0]]
        );
        assert_eq!(
            calls(&e, RENDER_ACCUMULATED),
            vec![vec![camera, accumulator, 0]]
        );
        assert_eq!(calls(&e, CULLING_BASE_DESTRUCTOR), vec![vec![culling]]);
        assert!(calls(&e, MAKE_DEBUG_LINE).is_empty());
        assert!(calls(&e, LOG).is_empty());
    }

    #[test]
    fn an_existing_query_texture_is_kept_and_not_cleared() {
        let mut e = rig();
        let (manager, decal) = query_scene(&mut e);
        e.set(manager, BGSDecalManager::spQueryTexture, Ptr::new(0x9000));
        e.register(STOP_OFFSCREEN, |_, a| ret(a[0] + 1));
        e.call(0x004a04c0, &args![manager, decal]);
        assert!(calls(&e, CREATE_QUERY_TEXTURE).is_empty());
        assert_eq!(calls(&e, STOP_OFFSCREEN), vec![vec![0x9000]]);
        assert_eq!(calls(&e, START_OFFSCREEN), vec![vec![0, 0x9001]]);
        // Without memory the query is null.
        assert_eq!(e.mem.u32(decal + 0x1c), 0);
        assert_eq!(calls(&e, QUERY_BEGIN), vec![vec![0]]);
        assert!(calls(&e, QUERY_CONSTRUCTOR).is_empty());
    }

    #[test]
    fn the_debug_flag_draws_the_camera_direction_and_the_log_prints_it() {
        let mut e = rig();
        let (manager, decal) = query_scene(&mut e);
        e.set(manager, BGSDecalManager::spQueryTexture, Ptr::new(0x9000));
        e.set_global(DECAL_DEBUG_FLAGS, 4u32);
        e.set_global(TES_OBJECT, 0x7000u32);
        set_flag(&mut e, SETTING_DEBUG_LOG, true);
        e.register(CAMERA_DIRECTION, |e, a| {
            put_vec(e, a[1], [0.0, 1.0, 0.0]);
            ret(a[1])
        });
        let line: Rc<RefCell<Vec<[f32; 3]>>> = Rc::default();
        let seen = line.clone();
        e.register_double(MAKE_DEBUG_LINE, move |e, a| {
            seen.borrow_mut().push(get_vec(e, a[0]));
            seen.borrow_mut().push(get_vec(e, a[2]));
            ret(0x8888)
        });
        e.call(0x004a04c0, &args![manager, decal]);

        // Start at the camera position, end 60 units along the camera
        // direction (the far plane).
        assert_eq!(*line.borrow(), vec![[1.0, 2.0, 53.0], [1.0, 62.0, 53.0]]);
        let colours = calls(&e, COLOR_CONSTRUCTOR);
        assert_eq!(colours.len(), 2);
        let line_call = &calls(&e, MAKE_DEBUG_LINE)[0];
        assert_eq!(line_call[1], colours[1][0], "start colour is the blue one");
        assert_eq!(line_call[3], colours[0][0], "end colour is the red one");
        assert_eq!(line_call[4], 1);
        assert_eq!(
            colours[0][1..].to_vec(),
            vec![1.0f32.to_bits(), 0, 0, 1.0f32.to_bits()]
        );
        assert_eq!(
            colours[1][1..].to_vec(),
            vec![0, 0, 1.0f32.to_bits(), 1.0f32.to_bits()]
        );
        assert_eq!(
            calls(&e, ADD_TEMP_DEBUG_OBJECT),
            vec![vec![0x7000, 0x8888, 30.0f32.to_bits()]]
        );

        // The log prints the target name, the origin, the direction of the
        // camera and the frustum.
        let log = calls(&e, LOG);
        assert_eq!(log[0], vec![TEXT_ISSUING, 0]);
        let mut origin = vec![TEXT_ORIGIN];
        origin.extend(doubles(&[1.0, 2.0, 53.0]));
        assert_eq!(log[1], origin);
        let mut direction = vec![TEXT_DIRECTION];
        direction.extend(doubles(&[0.0, 1.0, 0.0]));
        assert_eq!(log[2], direction);
        let mut frustum = vec![TEXT_FRUSTUM];
        frustum.extend(doubles(&[60.0, -1.0, 1.0, 0.5, -0.5]));
        assert_eq!(log[3], frustum);
    }

    // -----------------------------------------------------------------
    // AddDecal

    /// The player camera object reports `position`.
    fn set_camera(e: &mut Engine, position: [f32; 3]) {
        let point = e.mem.alloc(12);
        put_vec(e, point, position);
        let target = 0x0600_0010u32;
        vtable(e, 0x0440_0000, CAMERA_POSITION_SLOT, target);
        let owner = object_with_vtable(e, 0x40, 0x0440_0000);
        e.set_global(CAMERA_POSITION_OWNER, owner);
        e.register_double(target, move |_, _| ret(point));
    }

    /// Makes `AddGeometryDecalRecurse` return at once with its "reached
    /// the per-frame limit" log line, which the tests count as one visit.
    fn limit_the_recursion(e: &mut Engine) {
        set_int(e, SETTING_MAX_DECALS_PER_FRAME, 0);
        set_flag(e, SETTING_DEBUG_LOG, true);
    }

    fn visits(e: &Engine) -> usize {
        first_words(e, LOG)
            .iter()
            .filter(|w| **w == TEXT_FRAME_LIMIT)
            .count()
    }

    fn add_decal(
        e: &mut Engine,
        manager: Ptr<BGSDecalManager>,
        placement: Ptr<DecalPlacement>,
        decal_type: u32,
        force: bool,
    ) {
        e.call(0x004a10d0, &args![manager, placement, decal_type, force]);
    }

    #[test]
    fn add_decal_marks_the_texture_for_clearing_inside_a_profiling_scope() {
        let mut e = rig();
        let manager = new_manager(&mut e);
        let placement = new_placement(&mut e, 0x1000);
        add_decal(&mut e, manager, placement, 3, false);
        assert!(e.get(manager, BGSDecalManager::bClearQueryTexture));
        let begin = &calls(&e, SCOPE_BEGIN)[0];
        assert_eq!(begin[1..], [7, 1, TEXT_SOURCE_FILE, 0x1b8]);
        assert_eq!(calls(&e, SCOPE_END), vec![vec![begin[0]]]);
        // Type 3 does nothing else.
        assert!(calls(&e, LOG).is_empty());
        assert!(calls(&e, FLAG_TEST).is_empty());
    }

    #[test]
    fn type_1_adds_a_geometry_decal_when_the_target_is_within_the_limit() {
        let mut e = rig();
        let manager = new_manager(&mut e);
        let target = e.mem.alloc(0x40);
        let placement = new_placement(&mut e, target);
        put_vec(&mut e, placement.addr(), [3.0, 4.0, 0.0]);
        set_camera(&mut e, [0.0, 0.0, 0.0]);
        set_flag(&mut e, SETTING_DECALS, true);
        limit_the_recursion(&mut e);
        // Distance 5, limit 5: equal counts as within.
        set_float(&mut e, SETTING_GEOMETRY_DISTANCE, 5.0);
        add_decal(&mut e, manager, placement, 1, false);
        assert_eq!(
            first_words(&e, LOG),
            vec![TEXT_INSTANTIATED, TEXT_FRAME_LIMIT]
        );
        assert_eq!(calls(&e, FLAG_TEST), vec![vec![target, 0x400]]);
        assert!(e.get(manager, BGSDecalManager::bClearQueryTexture));
    }

    #[test]
    fn type_1_beyond_the_limit_needs_a_flag_or_force() {
        for (skip, force, expected) in [(0u8, false, 0usize), (1, false, 1), (0, true, 1)] {
            let mut e = rig();
            let manager = new_manager(&mut e);
            let target = e.mem.alloc(0x40);
            let placement = new_placement(&mut e, target);
            put_vec(&mut e, placement.addr(), [3.0, 4.0, 0.0]);
            e.set(placement, DecalPlacement::skip_distance_limit, skip);
            set_camera(&mut e, [0.0, 0.0, 0.0]);
            set_flag(&mut e, SETTING_DECALS, true);
            limit_the_recursion(&mut e);
            set_float(&mut e, SETTING_GEOMETRY_DISTANCE, 4.9);
            add_decal(&mut e, manager, placement, 1, force);
            assert_eq!(visits(&e), expected, "skip {skip} force {force}");
        }
    }

    #[test]
    fn type_1_needs_the_setting_a_target_and_an_unflagged_target() {
        // Setting off: the camera is not even asked (no camera object).
        let mut e = rig();
        let manager = new_manager(&mut e);
        let placement = new_placement(&mut e, 0x1000);
        add_decal(&mut e, manager, placement, 1, true);
        assert!(calls(&e, FLAG_TEST).is_empty());

        // No target.
        let mut e = rig();
        let manager = new_manager(&mut e);
        let placement = new_placement(&mut e, 0);
        set_flag(&mut e, SETTING_DECALS, true);
        add_decal(&mut e, manager, placement, 1, true);
        assert!(calls(&e, FLAG_TEST).is_empty());

        // Flag 0x400 set on the target.
        let mut e = rig();
        let manager = new_manager(&mut e);
        let placement = new_placement(&mut e, 0x1000);
        set_flag(&mut e, SETTING_DECALS, true);
        limit_the_recursion(&mut e);
        e.register(FLAG_TEST, |_, _| ret(1));
        add_decal(&mut e, manager, placement, 1, true);
        assert_eq!(visits(&e), 0);
    }

    #[test]
    fn type_1_times_the_call_when_profiling() {
        let mut e = rig();
        let manager = new_manager(&mut e);
        let target = e.mem.alloc(0x40);
        let placement = new_placement(&mut e, target);
        set_camera(&mut e, [0.0, 0.0, 0.0]);
        set_flag(&mut e, SETTING_DECALS, true);
        set_flag(&mut e, SETTING_PROFILE, true);
        set_float(&mut e, SETTING_GEOMETRY_DISTANCE, 5.0);
        set_int(&mut e, SETTING_MAX_DECALS_PER_FRAME, 0);
        e.register(QUERY_PERFORMANCE_FREQUENCY, |e, a| {
            e.mem.set_u64(a[0], 2_000_000);
            Ret::default()
        });
        let counter = Rc::new(RefCell::new(0u64));
        let state = counter.clone();
        e.register_double(QUERY_PERFORMANCE_COUNTER, move |e, a| {
            let mut ticks = state.borrow_mut();
            *ticks += 5000;
            e.mem.set_u64(a[0], *ticks);
            Ret::default()
        });
        add_decal(&mut e, manager, placement, 1, false);
        let format = &calls(&e, FORMAT)[0];
        let microseconds = 5000.0f64 / (2_000_000.0f64 * 1e-6);
        let mut expected = vec![format[0], 0xff, TEXT_TICKS, 5000, 0];
        microseconds.put(&mut expected);
        assert_eq!(*format, expected);
        assert_eq!(calls(&e, REPORT), vec![vec![format[0], 0]]);
        assert_eq!(calls(&e, QUERY_PERFORMANCE_FREQUENCY).len(), 1);
        assert_eq!(calls(&e, QUERY_PERFORMANCE_COUNTER).len(), 2);
    }

    #[test]
    fn type_4_sets_a_flag_and_adds_a_geometry_decal() {
        let mut e = rig();
        let manager = new_manager(&mut e);
        let target = e.mem.alloc(0x40);
        let placement = new_placement(&mut e, target);
        set_flag(&mut e, SETTING_DECALS, true);
        limit_the_recursion(&mut e);
        add_decal(&mut e, manager, placement, 4, false);
        assert_eq!(e.get(placement, DecalPlacement::flag_71), 1);
        assert_eq!(visits(&e), 1);

        // Setting off.
        let mut e = rig();
        let manager = new_manager(&mut e);
        let placement = new_placement(&mut e, target);
        limit_the_recursion(&mut e);
        add_decal(&mut e, manager, placement, 4, false);
        assert_eq!(e.get(placement, DecalPlacement::flag_71), 0);
        assert_eq!(visits(&e), 0);
    }

    /// A bound at `centre` with `radius`, as `GET_WORLD_BOUND` returns it.
    fn set_bound(e: &mut Engine, centre: [f32; 3], radius: f32) -> u32 {
        let bound = e.mem.alloc(16);
        put_vec(e, bound, centre);
        e.mem.set_f32(bound + 12, radius);
        e.register_double(GET_WORLD_BOUND, move |_, _| ret(bound));
        e.register(BOUND_RADIUS, |e, a| ret_float(e.mem.f32(a[0] + 12)));
        bound
    }

    #[test]
    fn type_2_projects_onto_the_target_within_the_limits() {
        // Bound distance 10 - 2 = 8 from the camera.
        for (limit, count, force, projected, logged) in [
            (8.0f32, 1u32, false, true, false),
            (7.9, 1, false, false, false),
            (7.9, 1, true, true, false),
            (8.0, 5, false, false, true),
            (8.0, 5, true, true, false),
        ] {
            let mut e = rig();
            let manager = new_manager(&mut e);
            let target = e.mem.alloc(0x40);
            let placement = new_placement(&mut e, target);
            set_camera(&mut e, [0.0, 0.0, 0.0]);
            set_bound(&mut e, [0.0, 0.0, 10.0], 2.0);
            set_flag(&mut e, SETTING_SKINNED_DECALS, true);
            set_flag(&mut e, SETTING_DEBUG_LOG, true);
            set_float(&mut e, SETTING_PROJECTED_DISTANCE, limit);
            set_int(&mut e, SETTING_MAX_SKIN_DECALS, 5);
            e.set_global(SKINNED_DECAL_COUNT, count);
            add_decal(&mut e, manager, placement, 2, force);
            let case = format!("limit {limit} count {count} force {force}");
            assert_eq!(
                calls(&e, ADD_PROJECTED_DECAL_RECURSE),
                if projected {
                    vec![vec![manager.addr(), placement.addr()]]
                } else {
                    vec![]
                },
                "{case}"
            );
            assert_eq!(
                calls(&e, LOG),
                if logged {
                    vec![vec![TEXT_SKIN_LIMIT, 5]]
                } else {
                    vec![]
                },
                "{case}"
            );
            assert_eq!(calls(&e, FLAG_TEST), vec![vec![target, 0x400]], "{case}");
        }
    }

    #[test]
    fn type_2_projects_onto_the_node_the_source_object_returns() {
        let mut e = rig();
        let manager = new_manager(&mut e);
        let projected = 0x3333u32;
        // The source object's virtual at +0x1d0 returns the node, whose
        // virtual at +0xc returns the node to project onto.
        let (first, second) = (0x0600_0020u32, 0x0600_0021u32);
        vtable(&mut e, 0x0450_0000, 0x1d0, first);
        vtable(&mut e, 0x0451_0000, 0xc, second);
        let source = object_with_vtable(&mut e, 0x40, 0x0450_0000);
        let node_object = object_with_vtable(&mut e, 0x40, 0x0451_0000);
        e.register_double(first, move |_, _| ret(node_object));
        e.register_double(second, move |_, _| ret(projected));
        let placement = new_placement(&mut e, 0);
        e.set(placement, DecalPlacement::source_object, Ptr::new(source));
        set_camera(&mut e, [0.0, 0.0, 0.0]);
        set_bound(&mut e, [0.0, 0.0, 3.0], 1.0);
        set_flag(&mut e, SETTING_SKINNED_DECALS, true);
        set_float(&mut e, SETTING_PROJECTED_DISTANCE, 100.0);
        set_int(&mut e, SETTING_MAX_SKIN_DECALS, 5);
        add_decal(&mut e, manager, placement, 2, false);
        assert_eq!(e.get(placement, DecalPlacement::target).addr(), projected);
        assert_eq!(
            calls(&e, ADD_PROJECTED_DECAL_RECURSE),
            vec![vec![manager.addr(), placement.addr()]]
        );
        assert_eq!(calls(&e, GET_WORLD_BOUND), vec![vec![node_object]]);
        assert_eq!(calls(&e, FLAG_TEST), vec![vec![node_object, 0x400]]);

        // Nothing to project onto: the target stays unset.
        let mut e = rig();
        let manager = new_manager(&mut e);
        vtable(&mut e, 0x0450_0000, 0x1d0, first);
        vtable(&mut e, 0x0451_0000, 0xc, second);
        let source = object_with_vtable(&mut e, 0x40, 0x0450_0000);
        let node_object = object_with_vtable(&mut e, 0x40, 0x0451_0000);
        e.register_double(first, move |_, _| ret(node_object));
        e.register_double(second, move |_, _| ret(0));
        let placement = new_placement(&mut e, 0);
        e.set(placement, DecalPlacement::source_object, Ptr::new(source));
        set_camera(&mut e, [0.0, 0.0, 0.0]);
        set_bound(&mut e, [0.0, 0.0, 3.0], 1.0);
        set_flag(&mut e, SETTING_SKINNED_DECALS, true);
        add_decal(&mut e, manager, placement, 2, true);
        assert_eq!(e.get(placement, DecalPlacement::target).addr(), 0);
        assert!(calls(&e, ADD_PROJECTED_DECAL_RECURSE).is_empty());
    }

    /// The decal caster doubles for the probing path of type 2: `hits`
    /// objects, each at position (7, 8, 9) with normal (0, 0, 3); records
    /// every cast's start, end and radius.
    type Casts = Rc<RefCell<Vec<(Vec<f32>, f32)>>>;

    fn caster_with_hits(e: &mut Engine, hits: Vec<u32>) -> Casts {
        let casts: Casts = Rc::default();
        let seen = casts.clone();
        let count = hits.len() as u32;
        e.register_double(DECAL_CASTER_CAST, move |e, a| {
            let mut points = get_vec(e, a[1]).to_vec();
            points.extend(get_vec(e, a[2]));
            seen.borrow_mut().push((points, f32::from_bits(a[3])));
            ret(count)
        });
        e.register_double(
            DECAL_CASTER_HIT_OBJECT,
            move |_, a| ret(hits[a[1] as usize]),
        );
        e.register(DECAL_CASTER_HIT_DATA, |e, a| {
            put_vec(e, a[2], [7.0, 8.0, 9.0]);
            put_vec(e, a[3], [0.0, 0.0, 3.0]);
            Ret::default()
        });
        casts
    }

    #[test]
    fn type_2_probes_with_the_decal_caster() {
        let mut e = rig();
        let manager = new_manager(&mut e);
        let (hit_a, hit_b) = (0x1010u32, 0x2020u32);
        let placement = new_placement(&mut e, 0x1111);
        put_vec(&mut e, placement.addr(), [1.0, 2.0, 3.0]);
        put_vec(&mut e, placement.addr() + 0x0c, [0.0, 0.0, 2.0]);
        e.set(placement, DecalPlacement::size_a, 3.0);
        e.set(placement, DecalPlacement::size_b, 5.0);
        e.set(placement, DecalPlacement::object_48, Ptr::new(0x4444));
        set_flag(&mut e, SETTING_DECALS, true);
        limit_the_recursion(&mut e);
        e.register(FLOAT_MAX, |_, _| ret_float(5.0));
        e.register(DECAL_CASTER_CREATE, |_, _| ret(0xca57));
        e.register(OBJECT_TEXTURE_SET, |_, _| ret(0x7e57));
        e.register(FIND_REFERENCE_FOR_3D, |_, a| {
            ret(if a[0] == 0x1010 { 0x5001 } else { 0 })
        });
        e.register(FORM_TYPE, |_, _| ret(0x24));
        let casts = caster_with_hits(&mut e, vec![hit_a, hit_b]);
        let added: Rc<RefCell<Vec<(u32, u32)>>> = Rc::default();
        let seen = added.clone();
        e.register_double(LIST_ADD_HEAD, move |e, a| {
            seen.borrow_mut().push((a[0], e.mem.u32(a[1])));
            Ret::default()
        });

        add_decal(&mut e, manager, placement, 2, false);

        // The caster is created with the larger size, configured, and
        // given the texture set.
        assert_eq!(
            calls(&e, FLOAT_MAX),
            vec![vec![5.0f32.to_bits(), 3.0f32.to_bits()]]
        );
        assert_eq!(
            calls(&e, DECAL_CASTER_CREATE),
            vec![vec![5.0f32.to_bits(), 1]]
        );
        assert_eq!(e.global::<u32>(DECAL_CASTER), 0xca57);
        assert_eq!(
            calls(&e, COLLISION_CALL),
            vec![vec![0x27, 8, 0], vec![0x27, 0x1d, 0]]
        );
        assert_eq!(calls(&e, OBJECT_TEXTURE_SET), vec![vec![0x4444]]);
        assert_eq!(
            calls(&e, DECAL_CASTER_SET_TEXTURE_SET),
            vec![vec![0xca57, 0x7e57]]
        );
        // One probe: from the origin 80 units along the normalized direction,
        // radius 32.
        assert_eq!(
            *casts.borrow(),
            vec![(vec![1.0, 2.0, 3.0, 1.0, 2.0, 83.0], 32.0)]
        );
        // The first hit is a reference that may take a decal; the second has
        // no reference.
        assert_eq!(
            calls(&e, DECAL_CASTER_HIT_DATA)
                .iter()
                .map(|w| w[1])
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
        assert_eq!(visits(&e), 1);
        assert_eq!(get_vec(&e, placement.addr()), [7.0, 8.0, 9.0]);
        assert_eq!(get_vec(&e, placement.addr() + 0x0c), [0.0, 0.0, 1.0]);
        assert_eq!(e.get(placement, DecalPlacement::target).addr(), hit_a);
        let list = calls(&e, LIST_CONSTRUCTOR)[0][0];
        assert_eq!(*added.borrow(), vec![(list, hit_a)]);
        assert_eq!(calls(&e, LIST_REMOVE_ALL), vec![vec![list]]);
        assert_eq!(calls(&e, LIST_LOCAL_DESTRUCTOR_BODY), vec![vec![list]]);
        assert_eq!(calls(&e, EMPTY_CONSTRUCTOR).len(), 4);
    }

    #[test]
    fn type_2_probes_again_along_the_fallback_direction_when_nothing_is_hit() {
        let mut e = rig();
        let manager = new_manager(&mut e);
        let placement = new_placement(&mut e, 0x1111);
        put_vec(&mut e, placement.addr(), [1.0, 2.0, 3.0]);
        put_vec(&mut e, placement.addr() + 0x0c, [0.0, 0.0, 2.0]);
        set_flag(&mut e, SETTING_DECALS, true);
        e.set_global(DECAL_CASTER, 0xca57u32);
        put_vec(&mut e, FALLBACK_VECTOR, [0.0, 0.0, 1.0]);
        let casts = caster_with_hits(&mut e, vec![]);
        add_decal(&mut e, manager, placement, 2, false);
        assert!(calls(&e, DECAL_CASTER_CREATE).is_empty());
        // The second probe goes from the origin 80 units along the
        // direction shifted by the negated fallback vector times 2.
        assert_eq!(
            *casts.borrow(),
            vec![
                (vec![1.0, 2.0, 3.0, 1.0, 2.0, 83.0], 32.0),
                (vec![1.0, 2.0, 3.0, 1.0, 2.0, -77.0], 32.0),
            ]
        );
        assert_eq!(get_vec(&e, placement.addr() + 0x0c), [0.0, 0.0, -1.0]);
        assert!(calls(&e, LIST_ADD_HEAD).is_empty());
        assert_eq!(calls(&e, LIST_REMOVE_ALL).len(), 1);
    }

    #[test]
    fn type_2_probing_skips_flagged_objects_and_forbidden_forms() {
        let mut e = rig();
        let manager = new_manager(&mut e);
        let placement = new_placement(&mut e, 0x1111);
        put_vec(&mut e, placement.addr() + 0x0c, [0.0, 0.0, 1.0]);
        set_flag(&mut e, SETTING_DECALS, true);
        limit_the_recursion(&mut e);
        e.set_global(DECAL_CASTER, 0xca57u32);
        e.register(FIND_REFERENCE_FOR_3D, |_, _| ret(0x5001));
        // Object 0x1010 carries flag 0x400; object 0x2020's form is a
        // forbidden type; 0x3030 is already in the list.
        e.register(FLAG_TEST, |_, a| ret((a[0] == 0x1010) as u32));
        e.register(FORM_TYPE, |_, _| ret(0x2a));
        let casts = caster_with_hits(&mut e, vec![0x1010, 0x2020]);
        add_decal(&mut e, manager, placement, 2, false);
        assert_eq!(casts.borrow().len(), 1);
        assert_eq!(visits(&e), 0);
        assert!(calls(&e, LIST_CONTAINS).is_empty());

        let mut e = rig();
        let manager = new_manager(&mut e);
        let placement = new_placement(&mut e, 0x1111);
        put_vec(&mut e, placement.addr() + 0x0c, [0.0, 0.0, 1.0]);
        set_flag(&mut e, SETTING_DECALS, true);
        limit_the_recursion(&mut e);
        e.set_global(DECAL_CASTER, 0xca57u32);
        e.register(FIND_REFERENCE_FOR_3D, |_, _| ret(0x5001));
        e.register(FORM_TYPE, |_, _| ret(0x24));
        e.register(LIST_CONTAINS, |_, _| ret(1));
        caster_with_hits(&mut e, vec![0x3030]);
        add_decal(&mut e, manager, placement, 2, false);
        assert_eq!(calls(&e, LIST_CONTAINS).len(), 1);
        assert_eq!(visits(&e), 0);
        assert!(calls(&e, LIST_ADD_HEAD).is_empty());
    }

    #[test]
    fn type_2_without_either_setting_does_nothing() {
        let mut e = rig();
        let manager = new_manager(&mut e);
        let placement = new_placement(&mut e, 0x1111);
        add_decal(&mut e, manager, placement, 2, true);
        assert!(calls(&e, DECAL_CASTER_CAST).is_empty());
        assert!(calls(&e, ADD_PROJECTED_DECAL_RECURSE).is_empty());
    }

    // -----------------------------------------------------------------
    // AddGeometryDecalRecurse

    /// The child lists `CHILD_COUNT` and `CHILD_AT` serve.
    fn set_children(e: &mut Engine, tree: &[(u32, Vec<u32>)]) {
        let map: Rc<HashMap<u32, Vec<u32>>> = Rc::new(tree.iter().cloned().collect());
        let counts = map.clone();
        e.register_double(CHILD_COUNT, move |_, a| {
            ret(counts.get(&a[0]).map_or(0, |c| c.len() as u32))
        });
        e.register_double(CHILD_AT, move |_, a| ret(map[&a[0]][a[1] as usize]));
    }

    /// Node names: the holder of a node is the node itself.
    fn set_names(e: &mut Engine, names: &[(u32, &[u8])]) {
        let mut map = HashMap::new();
        for (node, text) in names {
            let at = e.mem.alloc(text.len() as u32 + 1);
            e.mem.set_cstr(at, text);
            map.insert(*node, at);
        }
        e.register(NODE_NAME_HOLDER, |_, a| ret(a[0]));
        e.register_double(NAME_TEXT, move |_, a| {
            ret(map.get(&a[0]).copied().unwrap_or(0))
        });
    }

    /// An object whose virtual at `+0xc` (`IsNode`) returns `is_node`.
    fn node_object(e: &mut Engine, is_node: u32, vtable_at: u32, target: u32) -> u32 {
        vtable(e, vtable_at, 0xc, target);
        e.register_double(target, move |_, _| ret(is_node));
        object_with_vtable(e, 0x40, vtable_at)
    }

    fn recurse(
        e: &mut Engine,
        manager: Ptr<BGSDecalManager>,
        placement: Ptr<DecalPlacement>,
    ) -> bool {
        e.call(0x004a1a70, &args![manager, placement]).bool()
    }

    #[test]
    fn the_per_frame_limit_stops_the_recursion() {
        let mut e = rig();
        let manager = new_manager(&mut e);
        let placement = new_placement(&mut e, 0x1000);
        set_int(&mut e, SETTING_MAX_DECALS_PER_FRAME, 3);
        e.set_global(DECALS_THIS_FRAME, 3i32);
        assert!(!recurse(&mut e, manager, placement));
        assert!(calls(&e, LOG).is_empty());
        assert!(calls(&e, NODE_NAME_HOLDER).is_empty());

        set_flag(&mut e, SETTING_DEBUG_LOG, true);
        assert!(!recurse(&mut e, manager, placement));
        assert_eq!(calls(&e, LOG), vec![vec![TEXT_FRAME_LIMIT]]);

        // Below the limit it goes on (here: nothing to decorate).
        e.set_global(DECALS_THIS_FRAME, 2i32);
        e.set(placement, DecalPlacement::target, Ptr::new(0));
        assert!(recurse(&mut e, manager, placement));
        assert_eq!(calls(&e, LOG).len(), 1);
    }

    #[test]
    fn names_with_the_skipped_prefixes_get_no_decal() {
        for name in [
            &b"Decal Node 01"[..],
            b"Decal",
            b"FaceGen Head",
            b"BSFaceGen",
            b"Bip01 Spine",
            b"Debug Decal Box",
        ] {
            let mut e = rig();
            let manager = new_manager(&mut e);
            let placement = new_placement(&mut e, 0x1000);
            set_int(&mut e, SETTING_MAX_DECALS_PER_FRAME, 10);
            set_names(&mut e, &[(0x1000, name)]);
            assert!(recurse(&mut e, manager, placement));
            assert!(
                calls(&e, IS_MODEL_NODE).is_empty(),
                "{}",
                String::from_utf8_lossy(name)
            );
        }
        // Other names (and no name) go on to the model test.
        for name in [Some(&b"Weapon"[..]), Some(&b"Dec"[..]), None] {
            let mut e = rig();
            let manager = new_manager(&mut e);
            let node = node_object(&mut e, 0, 0x0460_0000, 0x0600_0030);
            let placement = new_placement(&mut e, node);
            set_int(&mut e, SETTING_MAX_DECALS_PER_FRAME, 10);
            if let Some(name) = name {
                set_names(&mut e, &[(node, name)]);
            }
            recurse(&mut e, manager, placement);
            assert_eq!(calls(&e, IS_MODEL_NODE), vec![vec![MODEL_FILTER, node]]);
        }
    }

    #[test]
    fn interior_nodes_recurse_into_their_children() {
        let mut e = rig();
        let manager = new_manager(&mut e);
        let node = node_object(&mut e, 1, 0x0460_0000, 0x0600_0030);
        let placement = new_placement(&mut e, node);
        set_int(&mut e, SETTING_MAX_DECALS_PER_FRAME, 10);
        let (child_a, child_b) = (0x1111u32, 0x2222u32);
        set_children(&mut e, &[(node, vec![child_a, 0, child_b])]);
        set_names(
            &mut e,
            &[
                (node, b"Root"),
                (child_a, b"Bip01 L"),
                (child_b, b"Debug Decal Box"),
            ],
        );
        assert!(recurse(&mut e, manager, placement));
        assert_eq!(
            calls(&e, CHILD_AT),
            vec![vec![node, 0], vec![node, 1], vec![node, 2]]
        );
        // The last non-null child is left as the placement's target.
        assert_eq!(e.get(placement, DecalPlacement::target).addr(), child_b);
        assert_eq!(calls(&e, IS_MODEL_NODE).len(), 1);
    }

    #[test]
    fn nodes_without_children_or_hidden_nodes_stop_the_recursion() {
        // Not an interior node.
        let mut e = rig();
        let manager = new_manager(&mut e);
        let node = node_object(&mut e, 0, 0x0460_0000, 0x0600_0030);
        let placement = new_placement(&mut e, node);
        set_int(&mut e, SETTING_MAX_DECALS_PER_FRAME, 10);
        assert!(recurse(&mut e, manager, placement));
        assert!(calls(&e, NODE_IS_HIDDEN).is_empty());
        assert!(calls(&e, CHILD_AT).is_empty());

        // Hidden: a model node that is hidden is treated like an interior
        // node and refused.
        let mut e = rig();
        let manager = new_manager(&mut e);
        let node = node_object(&mut e, 1, 0x0460_0000, 0x0600_0030);
        let placement = new_placement(&mut e, node);
        set_int(&mut e, SETTING_MAX_DECALS_PER_FRAME, 10);
        e.register(IS_MODEL_NODE, |_, _| ret(1));
        e.register(NODE_IS_HIDDEN, |_, _| ret(1));
        set_children(&mut e, &[(node, vec![0x1111])]);
        assert!(recurse(&mut e, manager, placement));
        assert!(calls(&e, CHILD_AT).is_empty());
        assert!(calls(&e, NODE_MODEL_DATA).is_empty());
    }

    /// A model node `0x1000` with a geometry list of two geometries, the
    /// second accepted, and everything a new decal needs.
    struct Leaf {
        manager: Ptr<BGSDecalManager>,
        placement: Ptr<DecalPlacement>,
        node: u32,
        rejected: u32,
        accepted: u32,
        decal: u32,
    }

    fn new_leaf(e: &mut Engine) -> Leaf {
        let manager = new_manager(e);
        let node = 0x1000u32;
        let placement = new_placement(e, node);
        let (list, rejected, accepted) =
            (e.mem.alloc(0x40), e.mem.alloc(0x200), e.mem.alloc(0x200));
        let decal = new_decal(e);
        set_int(e, SETTING_MAX_DECALS_PER_FRAME, 10);
        e.set_global(DECALS_THIS_FRAME, 2i32);
        set_float(e, SETTING_DECAL_LIFETIME, 12.5);
        e.register(IS_MODEL_NODE, |_, _| ret(1));
        e.register(NODE_MODEL_DATA, |_, _| ret(0xd1));
        e.register(MODEL_DATA_OBJECT, |_, _| ret(0xd2));
        e.register_double(NODE_GEOMETRY_LIST, move |_, _| ret(list));
        e.register_double(
            GEOMETRY_ACCEPTED,
            move |_, a| ret((a[1] == accepted) as u32),
        );
        set_children(e, &[(list, vec![rejected, accepted])]);
        e.register(ALLOCATE_OBJECT, |_, _| ret(0x6000));
        e.register_double(DECAL_CONSTRUCTOR, move |_, _| ret(decal));
        // The node reports a query size of 1000, which is not enough.
        let target = 0x0600_0031u32;
        vtable(e, 0x0470_0000, 0xa8, target);
        let owner = object_with_vtable(e, 0x40, 0x0470_0000);
        e.register_double(OBJECT_OF_NODE, move |_, _| ret(owner));
        e.register(target, |_, _| ret(1000));
        Leaf {
            manager,
            placement,
            node,
            rejected,
            accepted,
            decal,
        }
    }

    #[test]
    fn a_leaf_gets_a_new_decal_on_its_first_accepted_geometry() {
        let mut e = rig();
        let leaf = new_leaf(&mut e);
        set_flag(&mut e, SETTING_DEBUG_LOG, true);
        assert!(recurse(&mut e, leaf.manager, leaf.placement));
        assert_eq!(e.global::<i32>(DECALS_THIS_FRAME), 3);
        assert_eq!(calls(&e, LOG), vec![vec![TEXT_FRAME_COUNT, 3]]);
        assert_eq!(
            calls(&e, IS_MODEL_NODE),
            vec![vec![MODEL_FILTER, leaf.node]]
        );
        assert_eq!(calls(&e, MODEL_HAS_TAG), vec![vec![0xd2, 0x1a]]);
        assert_eq!(
            calls(&e, GEOMETRY_ACCEPTED),
            vec![
                vec![GEOMETRY_FILTER, leaf.rejected],
                vec![GEOMETRY_FILTER, leaf.accepted]
            ]
        );
        assert_eq!(calls(&e, ALLOCATE_OBJECT), vec![vec![0x144]]);
        assert_eq!(
            calls(&e, DECAL_CONSTRUCTOR),
            vec![vec![
                0x6000,
                leaf.accepted,
                12.5f32.to_bits(),
                leaf.placement.addr()
            ]]
        );
        // A query size of 1000 and no object: no query, nothing queued.
        assert_eq!(e.mem.u8(leaf.decal + 0x1a), 0);
        assert!(calls(&e, LIST_ADD_TAIL).is_empty());
    }

    #[test]
    fn a_large_query_size_flags_the_decal_for_an_occlusion_query() {
        let mut e = rig();
        let leaf = new_leaf(&mut e);
        let target = 0x0600_0032u32;
        vtable(&mut e, 0x0471_0000, 0xa8, target);
        let owner = object_with_vtable(&mut e, 0x40, 0x0471_0000);
        e.register_double(OBJECT_OF_NODE, move |_, _| ret(owner));
        e.register(target, |_, _| ret(1001));
        set_flag(&mut e, SETTING_OCCLUSION_QUERY, true);
        e.set(leaf.placement, DecalPlacement::occlusion_query_wanted, 1);
        e.set(leaf.manager, BGSDecalManager::bClearQueryTexture, true);
        assert!(recurse(&mut e, leaf.manager, leaf.placement));
        assert_eq!(e.mem.u8(leaf.decal + 0x1a), 1);
        assert_eq!(
            e.mem.u8(leaf.decal + 0x1b),
            1,
            "the clear request moves over"
        );
        assert!(!e.get(leaf.manager, BGSDecalManager::bClearQueryTexture));

        // Each of the three conditions is needed.
        for (setting, wanted, size) in [(false, 1u8, 1001u32), (true, 0, 1001), (true, 1, 1000)] {
            let mut e = rig();
            let leaf = new_leaf(&mut e);
            let target = 0x0600_0032u32;
            vtable(&mut e, 0x0471_0000, 0xa8, target);
            let owner = object_with_vtable(&mut e, 0x40, 0x0471_0000);
            e.register_double(OBJECT_OF_NODE, move |_, _| ret(owner));
            e.register_double(target, move |_, _| ret(size));
            set_flag(&mut e, SETTING_OCCLUSION_QUERY, setting);
            e.set(
                leaf.placement,
                DecalPlacement::occlusion_query_wanted,
                wanted,
            );
            e.set(leaf.manager, BGSDecalManager::bClearQueryTexture, true);
            assert!(recurse(&mut e, leaf.manager, leaf.placement));
            assert_eq!(e.mem.u8(leaf.decal + 0x1a), 0, "{setting} {wanted} {size}");
            assert!(e.get(leaf.manager, BGSDecalManager::bClearQueryTexture));
        }
    }

    #[test]
    fn a_decal_with_an_object_is_queued_on_the_pending_list() {
        let mut e = rig();
        let leaf = new_leaf(&mut e);
        e.set(leaf.placement, DecalPlacement::object_48, Ptr::new(0x4444));
        assert!(recurse(&mut e, leaf.manager, leaf.placement));
        let pending = leaf.manager.addr() + 8;
        let init = &calls(&e, NI_POINTER_INIT)[0];
        assert_eq!(init[1], leaf.decal);
        assert_eq!(calls(&e, LIST_ADD_TAIL), vec![vec![pending, init[0]]]);
        assert_eq!(calls(&e, NI_POINTER_RELEASE), vec![vec![init[0]]]);
    }

    #[test]
    fn a_failed_allocation_creates_no_decal() {
        let mut e = rig();
        let leaf = new_leaf(&mut e);
        e.register(ALLOCATE_OBJECT, |_, _| ret(0));
        assert!(recurse(&mut e, leaf.manager, leaf.placement));
        assert!(calls(&e, DECAL_CONSTRUCTOR).is_empty());
        assert_eq!(e.global::<i32>(DECALS_THIS_FRAME), 3, "still counted");
    }

    #[test]
    fn a_tagged_model_and_a_missing_geometry_list_end_early() {
        let mut e = rig();
        let leaf = new_leaf(&mut e);
        e.register(MODEL_HAS_TAG, |_, _| ret(1));
        assert!(recurse(&mut e, leaf.manager, leaf.placement));
        assert!(calls(&e, NODE_GEOMETRY_LIST).is_empty());

        let mut e = rig();
        let leaf = new_leaf(&mut e);
        e.register(NODE_GEOMETRY_LIST, |_, _| ret(0));
        assert!(!recurse(&mut e, leaf.manager, leaf.placement));
        assert!(calls(&e, ALLOCATE_OBJECT).is_empty());

        // A missing model object skips the tag test.
        let mut e = rig();
        let leaf = new_leaf(&mut e);
        e.register(MODEL_DATA_OBJECT, |_, _| ret(0));
        assert!(recurse(&mut e, leaf.manager, leaf.placement));
        assert!(calls(&e, MODEL_HAS_TAG).is_empty());
    }

    #[test]
    fn without_an_accepted_geometry_the_decal_has_none() {
        let mut e = rig();
        let leaf = new_leaf(&mut e);
        e.register(GEOMETRY_ACCEPTED, |_, _| ret(0));
        assert!(recurse(&mut e, leaf.manager, leaf.placement));
        let constructor = &calls(&e, DECAL_CONSTRUCTOR)[0];
        assert_eq!(constructor[1], 0);
    }

    /// A bound list with one entry whose origin is `origin` and whose width
    /// is 1; the placement's origin is zero, its sizes 1 and 2, and its
    /// `value_5c` 1: the entry's reach is 9.
    fn bound_scene(e: &mut Engine, origin: [f32; 3]) -> Leaf {
        let leaf = new_leaf(e);
        let entry = e.mem.alloc(0x144);
        put_vec(e, entry + 0x34, origin);
        e.mem.set_f32(entry + 0x118, 1.0);
        let bound = e.mem.alloc(8);
        e.mem.set_u32(bound, entry);
        e.mem.set_u32(leaf.accepted + 0xac, bound);
        e.set(leaf.placement, DecalPlacement::size_a, 1.0);
        e.set(leaf.placement, DecalPlacement::size_b, 2.0);
        e.set(leaf.placement, DecalPlacement::value_5c, 1.0);
        leaf
    }

    #[test]
    fn a_decal_close_to_a_bound_entry_is_refused() {
        // The squared distance sum is 1 (and 9 at the boundary): the reach
        // 9 / 1 = 9 is not below it.
        for origin in [[1.0, 0.0, 0.0], [3.0, 0.0, 0.0]] {
            let mut e = rig();
            let leaf = bound_scene(&mut e, origin);
            assert!(!recurse(&mut e, leaf.manager, leaf.placement), "{origin:?}");
            assert!(calls(&e, ALLOCATE_OBJECT).is_empty());
            assert_eq!(e.global::<i32>(DECALS_THIS_FRAME), 2);
        }
    }

    #[test]
    fn a_decal_far_from_every_bound_entry_is_created() {
        let mut e = rig();
        let leaf = bound_scene(&mut e, [10.0, 0.0, 0.0]);
        assert!(recurse(&mut e, leaf.manager, leaf.placement));
        assert_eq!(calls(&e, DECAL_CONSTRUCTOR).len(), 1);
        assert_eq!(calls(&e, BOUND_NEXT).len(), 1);
        // The sizes are only tested while the value is positive.
        let mut e = rig();
        let leaf = bound_scene(&mut e, [1.0, 0.0, 0.0]);
        e.set(leaf.placement, DecalPlacement::value_5c, 0.0);
        assert!(recurse(&mut e, leaf.manager, leaf.placement));
        assert!(calls(&e, BOUND_NEXT).is_empty());
        // A bound list that ends at once is not tested either.
        let mut e = rig();
        let leaf = bound_scene(&mut e, [1.0, 0.0, 0.0]);
        e.register(BOUND_IS_END, |_, _| ret(1));
        assert!(recurse(&mut e, leaf.manager, leaf.placement));
        assert!(calls(&e, BOUND_NEXT).is_empty());
        assert_eq!(calls(&e, DECAL_CONSTRUCTOR).len(), 1);
    }
}
