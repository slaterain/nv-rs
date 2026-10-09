//! `fallout shared/teswater.cpp` (Xbox PDB source unit), subsystem `fallout shared`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! The unit is `TESWaterSystem` (the singleton `TES::pWaterSystem` points
//! at), `PlaceableWaterGroup` and the small accessors the water code is
//! built from. It has 166 functions, `004e21b0` to `004edd80`.
//!
//! Translated so far: the first 40 functions of the queue (`004e21b0` to
//! `004e46b0`). The next session continues at `004e4730`
//! (`TESWaterSystem::AddPlaceableWater_ov2`).
//!
//! Notes for the next session:
//! - `00559450` is both `NiPointer<T>::operator T*` and
//!   `NiTPointerListBase::GetHeadPos` (it returns `*this`). A list node is
//!   `{next, prev, item}`: `007b52d0` returns `next`, `006317a0` the address
//!   of `item`. `for_each_list_item` walks a list the way every loop in the
//!   unit does (the next position first, then the item).
//! - `TESWaterSystem` and `PlaceableWaterGroup` keep the Xbox PDB offsets on
//!   the PC. `TESWaterForm` fields from `spNoiseNormalMap` on are 0x10 lower than
//!   the Xbox PDB says, and `WaterShaderProperty` fields from `bFullReflections` on
//!   are 8 lower (checked against the colour, fog and texture fields the
//!   update code moves between them). The layouts below list only the fields
//!   translated code touches.
//! - The engine map names `005f36f0` `ActorMover::GetPreferredMoveMode`; its
//!   body returns `TES::pInteriorCell` (`TES + 0x34`) and the water code
//!   uses it as "the player is in an interior" (`in_interior`).
//! - Water state kept in globals (`0x011c7a58..0x011c7a74`, the
//!   rendered-texture slots `0x011c7b64`, `0x011c7b68`, `0x011c7ad4` and
//!   `0x011c7c2c`) is named for the Xbox PDB static it matches by use; the
//!   INI settings are named by the string their static initializer
//!   registers.
//! - The compiler's exception-unwinding frames (`FS:[0]` chains) and the
//!   stack-protector cookie of `UpdatePlaceableWater` are not translated.
//!
//! x87 note: the game computes in extended precision and stores `float`
//! results. The translations compute in `f64` and round to `f32` where the
//! code stores a `float` (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

// ---------------------------------------------------------------------------
// Callees in other units, by address.

/// `*this`: `NiPointer<T>::operator T*` and `NiTPointerListBase::GetHeadPos`
/// (one folded body).
const NI_POINTER_GET: u32 = 0x0055_9450;
/// `NiTPointerListBase::GetNextPos(this, position)`: the node's `next`.
const LIST_NEXT_POSITION: u32 = 0x007b_52d0;
/// `NiTPointerListBase::GetAt(this, position)`: the address of the node's
/// item (node + 8).
const LIST_ITEM_SLOT: u32 = 0x0063_17a0;
/// Whether a `NiTPointerList` is empty (its count at +8 is zero).
const LIST_IS_EMPTY: u32 = 0x0076_b610;
/// The count of a list or array (`this + 8`).
const LIST_COUNT: u32 = 0x0044_ddc0;
/// `NiTPointerListBase::RemoveAll` (the engine map's name for the body).
const LIST_REMOVE_ALL: u32 = 0x004e_d900;
/// `NiTMapBase::RemoveAll`.
const MAP_REMOVE_ALL: u32 = 0x0043_8af0;
/// `NiPointer<T>::operator=(T*)`: `this` is the slot, the argument the new
/// pointer (releases the old object, references the new one).
const NI_POINTER_ASSIGN: u32 = 0x0066_b0d0;
/// `NiPointer<T>::operator=(const NiPointer&)` (the engine map names the
/// instance `NiPointer<PathingDebugGeometryData>::operator_`): `this` is the
/// slot, the argument the address of the slot to copy from.
const NI_POINTER_ASSIGN_FROM: u32 = 0x006e_5cc0;
/// `SettingT<INISettingCollection>::GetValue`: the address of the setting's
/// value (setting + 4).
const SETTING_VALUE: u32 = 0x0040_8d60;
/// `TES::pInteriorCell` read through the `TES` object (`TES + 0x34`).
const TES_GET_INTERIOR_CELL: u32 = 0x005f_36f0;
/// `TESObjectREFR::pBaseForm` (+0x20).
const REFERENCE_BASE_FORM: u32 = 0x007a_f430;
/// `TESObjectREFR::pParentCell` (+0x40 on the PC).
const REFERENCE_PARENT_CELL: u32 = 0x008d_6f30;
/// `ExtraDataList` of a reference (`TESObjectREFR + 0x44`).
const REFERENCE_EXTRA_DATA_LIST: u32 = 0x005d_43c0;
/// `ExtraDataList::GetReflectedRefs` (Xbox PDB name).
const EXTRA_DATA_LIST_GET_REFLECTED_REFS: u32 = 0x0041_f140;
/// `true` when the cell's byte at +0x26 is 6 (`00450fd0` reads the byte).
const CELL_BYTE_IS_SIX: u32 = 0x0045_0ff0;
/// `TESObjectREFR` virtual at +0x1d0 (the Xbox PDB's `Get3D` is at +0x1cc):
/// the reference's 3D node (`NiAVObject*`), null when it has none.
const REFERENCE_GET_3D: u32 = 0x1d0;
/// `true` when `NiAVObject::m_uFlags & 1` (the node is culled).
const NODE_IS_CULLED: u32 = 0x0045_6610;
/// `NiAVObject::m_kWorld.m_Translate` (`this + 0x8c`).
const NODE_WORLD_TRANSLATE: u32 = 0x0045_bb80;
/// `NiAVObject::m_kWorld.m_Rotate` (`this + 0x68`, a 3x3 matrix).
const NODE_WORLD_ROTATE: u32 = 0x0046_1130;
/// `NiAVObject::GetProperty(this, type)` (Xbox PDB name).
const NODE_GET_PROPERTY: u32 = 0x00a5_9d30;
/// `[this + 0xc0]` of a water reference's 3D object, and the type id
/// (`+0x68`) of that object.
const NODE_OWNER: u32 = 0x0040_30b0;
const OWNER_TYPE: u32 = 0x008d_8520;
/// The owner type id the shader-property updates apply to.
const WATER_OWNER_TYPE: u32 = 0x0d;
/// `NiPoint4::NiPoint4(this, x, y, z, w)` (returns `this`).
const NI_POINT4_CONSTRUCT: u32 = 0x0041_4430;
/// `this[0] == 0 && this[1] == 0`.
const WORDS_ARE_ZERO: u32 = 0x0082_56d0;
/// Whether the form flags of a base object have a bit set: `this`, then the
/// mask.
const FORM_FLAG_TEST: u32 = 0x0045_2460;
/// The same with the mask `0x40000000`.
const FORM_FLAG_TEST_40000000: u32 = 0x0045_2440;

/// `TESWaterSystem` and water-group helpers in this unit that are not
/// translated yet (called by address).
/// `this`, reference: the reference's 3D object used for its water shader
/// property, or null.
const WATER_REFERENCE_3D: u32 = 0x004e_8030;
/// `this`, reference, viewer: the range test `UpdatePlaceableWater` and
/// `UpdateLODWater` apply to a reference and the viewer node.
const REFERENCE_IS_IN_RANGE: u32 = 0x004e_62e0;
/// `this`, group, 0, 0: called for a group that is not rendered (and by the
/// LOD update while water is skipped).
const RELEASE_GROUP: u32 = 0x004e_58a0;
/// `this`, group: called for a rendered group once its passes were
/// scheduled.
const FINISH_GROUP: u32 = 0x004e_56c0;
/// `this`, group: called before `FINISH_GROUP` for a rendered group with
/// reflections when the player is in an interior.
const FINISH_GROUP_INTERIOR: u32 = 0x004e_b220;
/// `this`, viewer, group, flag: called for a group with reflections (the
/// flag is 0 for high detail or in an interior, else 1).
const SETUP_GROUP_REFLECTIONS: u32 = 0x004e_9d40;
/// `this`, viewer, group: called for a group that wants depth.
const SETUP_GROUP_DEPTH: u32 = 0x004e_bef0;
/// `this`, viewer: called when the world / sky reflections are on.
const SETUP_WORLD_REFLECTIONS: u32 = 0x004e_aa00;
const SETUP_SKY_REFLECTIONS: u32 = 0x004e_af80;
/// `this`: called when the world / sky reflections are on, after the groups
/// were scheduled.
const FINISH_WORLD_REFLECTIONS: u32 = 0x004e_b540;
const FINISH_SKY_REFLECTIONS: u32 = 0x004e_bbe0;
/// `this`, camera, group, first, stencil mask: called for every group whose
/// depth is rendered.
const RENDER_GROUP_DEPTH: u32 = 0x004e_cb60;
/// `this`, group, stencil mask: what follows a group's depth render.
const AFTER_GROUP_DEPTH: u32 = 0x004e_c800;
/// `cdecl(index, x, y, z, w)`: stores a four-float shader constant (the
/// depth pass sets index 0x1e to the viewer's position).
const VIEWER_POSITION_UPDATE: u32 = 0x004e_20c0;
/// Returns the byte at `0x011c7a59`; the water code skips its passes while it
/// is set.
const WATER_SKIPPED_FLAG: u32 = 0x004e_2180;

/// Renderer calls the passes make.
/// The `NiRenderer*` (`[0x011f9508]`) and its virtuals the depth pass uses.
const RENDERER: u32 = 0x004a_0e90;
const RENDERER_GET_CLEAR_COLOR: u32 = 0xb4;
const RENDERER_SET_CLEAR_COLOR: u32 = 0xac;
/// The `BSTextureManager*` (`[0x011f91a8]`) and
/// `BSTextureManager::ReturnRenderedTexture(this, texture)` (Xbox PDB name).
const TEXTURE_MANAGER: u32 = 0x004a_0ea0;
const RETURN_RENDERED_TEXTURE: u32 = 0x00b6_da10;
/// `BSRenderedTexture::StopOffscreen` (engine map name): `this` is the
/// texture; its result goes to `00b6b8d0(mode, result)`.
const RENDERED_TEXTURE_STOP: u32 = 0x00b6_b260;
const RENDER_TARGET_SET: u32 = 0x00b6_b8d0;
/// No arguments: runs after a group's depth effect.
const RENDER_TARGET_RESET: u32 = 0x00b6_b790;
/// `BSBatchRenderer::EndPass` (Xbox PDB name; static).
const BATCH_RENDERER_END_PASS: u32 = 0x00b9_88e0;
/// `cdecl(2, 0xff, stencil mask, 0)`.
const SET_STENCIL_STATE: u32 = 0x00b9_8180;
/// `ImageSpaceShaderParam` (0x8c bytes on the stack): constructor,
/// `InitConstantMap(this, 0, 1)`, `SetPixelConstant(this, index, x, y, z, w)`
/// and destructor.
const SHADER_PARAM_SIZE: u32 = 0x8c;
const SHADER_PARAM_CONSTRUCT: u32 = 0x00b8_a9e0;
const SHADER_PARAM_INIT_CONSTANT_MAP: u32 = 0x00b8_ab10;
const SHADER_PARAM_SET_PIXEL_CONSTANT: u32 = 0x00b8_a790;
const SHADER_PARAM_DESTRUCT: u32 = 0x00b8_a8e0;
/// `ImageSpaceManager::RenderEffect(this, effect, renderer, texture, param, 0)`.
const IMAGE_SPACE_RENDER_EFFECT: u32 = 0x00b8_c730;
/// `this`, 0x21, renderer, texture, noise map, 0, 1.
const IMAGE_SPACE_RENDER_NOISE: u32 = 0x00b9_75f0;
/// `this` (texture manager), renderer, 0x13, 0, 0, 0: creates the rendered
/// texture the water noise map is drawn into.
const CREATE_RENDERED_TEXTURE: u32 = 0x00b6_e110;
/// `this` (the flush object): runs when its `+0x2b8` byte is clear.
const FLUSH_OBJECT_FLUSH: u32 = 0x00b9_b7f0;
/// `this`, name, slot, 1, 0 (`TES::CreateTextureImage`, Xbox PDB name).
const CREATE_TEXTURE_IMAGE: u32 = 0x0045_68c0;
/// `TESWaterSystem::AddPlaceableWater_ov2` (Xbox PDB name): `this`,
/// reference, position, rotation matrix.
const ADD_PLACEABLE_WATER_OV2: u32 = 0x004e_4730;

/// `TESWaterForm` helpers.
/// Whether the form's `NoiseTexture` (`TESTexture` at +0x64, Xbox +0x74) has
/// a name (`0048cee0` on its string).
const WATER_FORM_HAS_NOISE_TEXTURE: u32 = 0x0058_0080;
/// The name of the form's noise texture (`00408da0` on `this + 0x64`).
const WATER_FORM_NOISE_NAME: u32 = 0x0058_00a0;
/// `this`, slot: `*slot = this->spNoiseTexture` (the form's `NiPointer` at
/// +0x17c, Xbox +0x18c), through `NI_POINTER_ASSIGN_FROM`.
const WATER_FORM_COPY_NOISE_TEXTURE: u32 = 0x0058_00e0;
/// `NiPointer<T>::NiPointer(this, pointer)` (stores the pointer and
/// references it) and `NiPointer<T>::~NiPointer`, for a temporary of 4 bytes.
const POINTER_TEMP_CONSTRUCT: u32 = 0x0063_3c90;
const POINTER_TEMP_DESTRUCT: u32 = 0x0045_cec0;
/// `NiTMapBase<TESWaterForm *, bool>::GetAt(this, key, bool *result)` and
/// `SetAt(this, key, value)` (the map is `WaterTypeUpdateMap`).
const WATER_TYPE_MAP_FIND: u32 = 0x0057_c850;
const WATER_TYPE_MAP_SET: u32 = 0x0084_d310;

/// Float helpers and the CRT.
/// `fmodf(a, b)` (cdecl): the CRT `fmod` (`00ec9130`) on two floats.
const FLOAT_REMAINDER: u32 = 0x004b_1520;
/// `max(a, b)` of two floats (cdecl): `b < a ? a : b`.
const FLOAT_MAX: u32 = 0x0040_4010;
/// `cdecl(byte)`: the float at `0x011f940c + 4 * (byte != 0)`.
const FLOAT_TABLE_ENTRY: u32 = 0x0045_2e70;
/// The CRT `ceil`, `cos` and `sin` (cdecl, one `double`).
const CRT_CEIL: u32 = 0x00ec_9e10;
const CRT_COS: u32 = 0x00ec_9f30;
const CRT_SIN: u32 = 0x00ec_a060;
/// `PlaneConstant`: the `float` at `this + 0xc` (ST0).
const PLANE_CONSTANT: u32 = 0x0084_d030;
/// `sprintf_s(buffer, size, format, ...)` and the message logger
/// `(format, ...)`.
const FORMATTED_PRINT: u32 = 0x0040_6d00;
const LOG_MESSAGE: u32 = 0x005b_5e40;

// ---------------------------------------------------------------------------
// Globals.

/// The `TES*` (`TES + 0x34` is `pInteriorCell`).
const TES_POINTER: u32 = 0x011d_ea10;
/// The `ImageSpaceManager*` the water passes render effects with.
const IMAGE_SPACE_MANAGER: u32 = 0x011f_91ac;
/// A pointer to an object the water update flushes (`fn_004e4680`).
const FLUSH_OBJECT: u32 = 0x011f_fe44;
/// A byte flag (`fn_004e3c40` returns it; `00575c96` writes it).
const WATER_FLAG_011C7A58: u32 = 0x011c_7a58;
/// An object `UpdatePlaceableWater` asks `WORDS_ARE_ZERO` about
/// (`fn_004e3260` returns its address).
const WATER_OBJECT_011CA13C: u32 = 0x011c_a13c;
/// `PlaceableWaterGroup::bRenderWorldReflections` and
/// `bRenderSkyReflections` (Xbox PDB statics; match by use), and
/// `iWorldReflectionThreadStage` / `iSkyReflectionThreadStage`.
const RENDER_WORLD_REFLECTIONS: u32 = 0x011c_7a66;
const RENDER_SKY_REFLECTIONS: u32 = 0x011c_7a67;
const WORLD_REFLECTION_THREAD_STAGE: u32 = 0x011c_7a6c;
const SKY_REFLECTION_THREAD_STAGE: u32 = 0x011c_7a70;
/// `TESWaterSystem::iActiveWaterGroups` (Xbox PDB static; matches by use).
const ACTIVE_WATER_GROUPS: u32 = 0x011c_7a68;
/// The stencil mask of the group whose depth was rendered last.
const LAST_DEPTH_STENCIL_MASK: u32 = 0x011c_7a74;
/// `PlaceableWaterGroup::spWorldReflectionMap` and `spSkyReflectionMap`
/// (`NiPointer<BSRenderedTexture>` statics; match by use).
const WORLD_REFLECTION_MAP: u32 = 0x011c_7ad4;
const SKY_REFLECTION_MAP: u32 = 0x011c_7c2c;
/// `PlaceableWaterGroup::spDepthMap` and `spWadingWaterHeightMap`
/// (`NiPointer<BSRenderedTexture>` statics; match by use).
const DEPTH_MAP: u32 = 0x011c_7b68;
const WADING_WATER_HEIGHT_MAP: u32 = 0x011c_7b64;
/// Four floats: the clear colour the depth pass sets.
const DEPTH_CLEAR_COLOR: u32 = 0x011a_9bd0;
/// A float `UpdateWaterShaderProperties_ov2` stores (`00885d70` of the form).
const WATER_SHADER_FLOAT: u32 = 0x011f_f108;
/// The 24 words `fn_004e3d60` publishes for the shader: from
/// `0x011ffe48`, the scroll rows and the blend factors (see there).
const SHADER_NOISE_BASE: u32 = 0x011f_fe48;
/// The width and height words of the noise map's rendered texture.
const NOISE_WIDTH: u32 = 0x011a_d81c;
const NOISE_HEIGHT: u32 = 0x011a_d820;

/// INI settings (`SettingT<INISettingCollection>` statics), by the name the
/// game's static initializer registers them under.
/// `bAutoWaterSilhouetteReflections:Water`.
const SETTING_AUTO_SILHOUETTE_REFLECTIONS: u32 = 0x011c_7a90;
/// `bForceLowDetailReflections:Water`.
const SETTING_FORCE_LOW_DETAIL_REFLECTIONS: u32 = 0x011c_7aa8;
/// `bUseWaterReflections:Water`.
const SETTING_USE_WATER_REFLECTIONS: u32 = 0x011c_7b6c;
/// `bUsePerWorldSpaceWaterNoise:Water`.
const SETTING_USE_PER_WORLD_SPACE_NOISE: u32 = 0x011c_7b98;
/// `bUseWaterDepth:Water`.
const SETTING_USE_WATER_DEPTH: u32 = 0x011c_7bbc;
/// `bForceHighDetailReflections:Water`.
const SETTING_FORCE_HIGH_DETAIL_REFLECTIONS: u32 = 0x011c_7c00;
/// `bUseWaterRefractions:Water`.
const SETTING_USE_WATER_REFRACTIONS: u32 = 0x011c_7c60;

/// `0.017453292` (`double`): degrees to radians.
const DEGREES_TO_RADIANS: u32 = 0x0102_3128;
/// `255.0` (`double`).
const BYTE_RANGE: u32 = 0x0101_e568;
/// `100.0` (`double`).
const ONE_HUNDRED: u32 = 0x0101_7a40;
/// `-1.0` (`float`).
const MINUS_ONE: u32 = 0x0101_2054;
/// `"%s"`.
const FORMAT_STRING: u32 = 0x0101_9f08;
/// `"WATER: Can not render water group at height %f using water type %s.
/// You are using too many water groups : %d"`.
const TOO_MANY_GROUPS_MESSAGE: u32 = 0x0102_3060;
/// `"WATER: LOD Water objects still trying to update in an interior when
/// they should not be."`
const LOD_IN_INTERIOR_MESSAGE: u32 = 0x0102_30d0;

/// `TESWaterForm` virtual at +0x130 (the Xbox PDB's `GetObjectTypeName`):
/// the name the log message shows.
const WATER_FORM_GET_NAME: u32 = 0x130;
/// `TESObjectREFR` base form virtual at +0x140: its water form (null when it
/// has none).
const BASE_FORM_GET_WATER_FORM: u32 = 0x140;
/// The 3D object's virtual at +0x18 `UpdateWaterShaderProperties_ov2` tests.
const NODE_PROPERTIES_VIRTUAL: u32 = 0x18;
/// The property's virtual at +0xa4 (a count, taken as a 16-bit value).
const PROPERTY_PASS_COUNT_VIRTUAL: u32 = 0xa4;
/// The rendered texture's virtuals at +0x94 / +0x98 (width, height).
const TEXTURE_WIDTH_VIRTUAL: u32 = 0x94;
const TEXTURE_HEIGHT_VIRTUAL: u32 = 0x98;

/// The bits of the base form's flags the water code tests.
const FORM_FLAG_0001: u32 = 0x0000_0001;
const FORM_FLAG_0200: u32 = 0x0000_0200;
const FORM_FLAG_0800: u32 = 0x0000_0800;
const FORM_FLAG_40000: u32 = 0x0004_0000;
const FORM_FLAG_8000000: u32 = 0x0800_0000;
const FORM_FLAG_10000000: u32 = 0x1000_0000;
const FORM_FLAG_20000000: u32 = 0x2000_0000;
const FORM_FLAG_80000000: u32 = 0x8000_0000;

// ---------------------------------------------------------------------------
// Layouts.

layout! {
    /// `NiTPointerList<T>` / `NiTPointerListSingleThread<T>` (Xbox PDB),
    /// 0xC bytes for every `T`: head, tail, count (not yet in
    /// `crate::types`).
    pub struct NiTPointerList: 0x0c {
    }

    /// `NiTPointerMap<K, V>` (Xbox PDB), 0x10 bytes for every `K`, `V` (not
    /// yet in `crate::types`).
    pub struct NiTPointerMap: 0x10 {
    }

    /// `NiPlane` (Xbox PDB): the normal and the constant (at +0xC).
    pub struct NiPlane: 0x10 {
    }

    /// `NiColorA` (Xbox PDB): four floats.
    pub struct NiColorA: 0x10 {
    }

    /// `TESWaterSystem` (Xbox PDB), 0xA0 bytes on the Xbox (the PC size was
    /// not checked); the fields below sit at the same offsets on the PC.
    pub struct TESWaterSystem: 0xa0 {
        /// `iAccumulationCount` (Xbox PDB): the running stage number
        /// `UpdatePlaceableWater` hands to each pass it sets up.
        0x00 iAccumulationCount: i32,
        /// `spWaterNoiseTexture` (Xbox PDB): `NiPointer<NiTexture>`.
        0x1c spWaterNoiseTexture: u32,
        /// `PlaceableWaterGroupList` (Xbox PDB):
        /// `NiTPointerList<PlaceableWaterGroup *>`.
        0x3c PlaceableWaterGroupList: Inline<NiTPointerList>,
        /// `pLODWaterGroup` (Xbox PDB).
        0x48 pLODWaterGroup: Ptr<PlaceableWaterGroup>,
        /// `ReflectionRefMap` (Xbox PDB).
        0x4c ReflectionRefMap: Inline<NiTPointerMap>,
        /// `DepthRefMap` (Xbox PDB).
        0x5c DepthRefMap: Inline<NiTPointerMap>,
        /// `WaterTypeUpdateMap` (Xbox PDB):
        /// `NiTPointerMap<TESWaterForm *, bool>`.
        0x6c WaterTypeUpdateMap: Inline<NiTPointerMap>,
    }

    /// `PlaceableWaterGroup` (Xbox PDB), 0xB0 bytes on the Xbox; the fields
    /// below sit at the same offsets on the PC.
    pub struct PlaceableWaterGroup: 0xb0 {
        /// `pWaterType` (Xbox PDB): `TESWaterForm*`.
        0x00 pWaterType: Ptr,
        /// `ReflectWaterPlane` (Xbox PDB).
        0x04 ReflectWaterPlane: Inline<NiPlane>,
        /// `PlaceableWaterList` (Xbox PDB): the water references of the group.
        0x24 PlaceableWaterList: Inline<NiTPointerList>,
        /// `ObjectInWaterList` (Xbox PDB).
        0x30 ObjectInWaterList: Inline<NiTPointerList>,
        /// `ActorsInWaterList` (Xbox PDB).
        0x3c ActorsInWaterList: Inline<NiTPointerList>,
        /// `spWadingWaterGeometry` (Xbox PDB): `NiPointer<NiAVObject>`.
        0x58 spWadingWaterGeometry: u32,
        /// `bGroupAtWorldSpaceWaterHeight` (Xbox PDB).
        0x5c bGroupAtWorldSpaceWaterHeight: bool,
        /// `bRenderGroup` (Xbox PDB).
        0x5d bRenderGroup: bool,
        /// `bRenderDepth` (Xbox PDB).
        0x5e bRenderDepth: bool,
        /// `bRenderGroupReflections` (Xbox PDB).
        0x5f bRenderGroupReflections: bool,
        /// `bRenderSilhouetteReflections` (Xbox PDB).
        0x60 bRenderSilhouetteReflections: bool,
        /// `StaticDepthObjects` (Xbox PDB).
        0x7c StaticDepthObjects: Inline<NiTPointerList>,
        /// `DynamicDepthObjects` (Xbox PDB).
        0x88 DynamicDepthObjects: Inline<NiTPointerList>,
        /// `iReflectionThreadStage` (Xbox PDB).
        0x9c iReflectionThreadStage: i32,
        /// `iDepthThreadStage` (Xbox PDB).
        0xa0 iDepthThreadStage: i32,
        /// `spDepthCamera` (Xbox PDB): `NiPointer<NiCamera>`.
        0xa8 spDepthCamera: u32,
        /// `iStencilBitMask` (Xbox PDB); the code reads it as a 32-bit word
        /// and as a 16-bit word.
        0xac iStencilBitMask: u32,
    }

    /// `WaterShaderProperty` (Xbox PDB, 0x158 bytes there): the fields the
    /// water update code touches at their PC offsets. Fields from `bFullReflections`
    /// on are 8 lower on the PC than the Xbox PDB says. The members named
    /// `value_...` take a value from a `TESWaterForm` getter; the Xbox PDB
    /// has `NiColorA` members around these offsets and the mapping was not
    /// checked.
    pub struct WaterShaderProperty: 0x150 {
        /// `pRenderPassList` (Xbox PDB, `BSShaderProperty`).
        0x3c pRenderPassList: Ptr,
        /// `bFullReflections` (Xbox PDB; Xbox +0x6a).
        0x62 bFullReflections: bool,
        /// `bDepth` (Xbox PDB; Xbox +0x6b).
        0x63 bDepth: bool,
        /// `fFogFar` (Xbox PDB; Xbox +0x7c).
        0x74 fFogFar: f32,
        /// `fFogRange` (Xbox PDB; Xbox +0x80).
        0x78 fFogRange: f32,
        /// `bUpdateConstants` (Xbox PDB; Xbox +0x87).
        0x7f bUpdateConstants: bool,
        /// `bReflections` (Xbox PDB; Xbox +0x88).
        0x80 bReflections: bool,
        /// `bRefractions` (Xbox PDB; Xbox +0x89).
        0x81 bRefractions: bool,
        /// `bObjectTexCoords` (Xbox PDB; Xbox +0x8a).
        0x82 bObjectTexCoords: bool,
        /// `pShallowColor` (Xbox PDB; Xbox +0x90).
        0x88 pShallowColor: Inline<NiColorA>,
        /// `pDeepColor` (Xbox PDB; Xbox +0xa0).
        0x98 pDeepColor: Inline<NiColorA>,
        /// `pReflectionColor` (Xbox PDB; Xbox +0xb0).
        0xa8 pReflectionColor: Inline<NiColorA>,
        0xb8 value_00b8: f32,
        0xbc value_00bc: f32,
        0xc0 value_00c0: f32,
        0xc4 value_00c4: f32,
        0xd0 value_00d0: f32,
        0xd4 value_00d4: f32,
        0xd8 value_00d8: f32,
        0xdc value_00dc: f32,
        0x100 value_0100: f32,
        0x104 value_0104: f32,
        /// `fFresnelAmount` (Xbox PDB; Xbox +0x120).
        0x118 fFresnelAmount: f32,
        /// `fNoiseScale` (Xbox PDB; Xbox +0x124).
        0x11c fNoiseScale: f32,
        /// `fFogAmount` (Xbox PDB; Xbox +0x128).
        0x120 fFogAmount: f32,
        /// `fUVScale` (Xbox PDB; Xbox +0x12c).
        0x124 fUVScale: f32,
        /// `spNoiseHeightMap` (Xbox PDB; Xbox +0x13c): `NiPointer<NiTexture>`.
        0x134 spNoiseHeightMap: u32,
        /// `spNoiseNormalMap` (Xbox PDB; Xbox +0x140).
        0x138 spNoiseNormalMap: u32,
        /// `spReflectionMap` (Xbox PDB; Xbox +0x144).
        0x13c spReflectionMap: u32,
        /// `spRefractionMap` (Xbox PDB; Xbox +0x148).
        0x140 spRefractionMap: u32,
        /// `spDepthMap` (Xbox PDB; Xbox +0x14c).
        0x144 spDepthMap: u32,
    }

    /// `TESWaterForm` (Xbox PDB): the fields the water update code touches
    /// at their PC offsets (`spNoiseNormalMap` and later fields are 0x10 lower
    /// than the Xbox PDB says; the `WaterShaderData` member starts at +0x84).
    pub struct TESWaterForm: 0x194 {
        /// `spNoiseNormalMap` (Xbox PDB; Xbox +0x40):
        /// `NiPointer<BSRenderedTexture>`.
        0x30 spNoiseNormalMap: u32,
        /// `fTexScroll0` (Xbox PDB; Xbox +0x44), red.
        0x34 fTexScroll0R: f32,
        /// `fTexScroll0` green.
        0x38 fTexScroll0G: f32,
        /// `fTexScroll1` (Xbox PDB; Xbox +0x54), red.
        0x44 fTexScroll1R: f32,
        /// `fTexScroll1` green.
        0x48 fTexScroll1G: f32,
        /// `fTexScroll2` (Xbox PDB; Xbox +0x64), red.
        0x54 fTexScroll2R: f32,
        /// `fTexScroll2` green.
        0x58 fTexScroll2G: f32,
        /// `Data.fFresnelAmount` (`WaterShaderData` +0x18).
        0x9c fFresnelAmount: f32,
        /// `Data.fNoiseWindDirection0` (`WaterShaderData` +0x64).
        0xe8 fNoiseWindDirection0: f32,
        /// `Data.fNoiseWindDirection2` (`WaterShaderData` +0x6c).
        0xf0 fNoiseWindDirection2: f32,
        /// `Data.fNoiseWindSpeed1` (`WaterShaderData` +0x74).
        0xf8 fNoiseWindSpeed1: f32,
        /// `Data.fFogAmount` (`WaterShaderData` +0x84).
        0x108 fFogAmount: f32,
        /// `Data.fUnderwaterFogDistNear` (`WaterShaderData` +0x90).
        0x114 fUnderwaterFogDistNear: f32,
        /// `Data.fLightRadius` (`WaterShaderData` +0xa4).
        0x128 fLightRadius: f32,
        /// `Data.fLightBrightness` (`WaterShaderData` +0xa8).
        0x12c fLightBrightness: f32,
        /// `Data.fHeightUVScale0` (`WaterShaderData` +0xac).
        0x130 fHeightUVScale0: f32,
        /// `bResetNoiseTexture` (Xbox PDB; Xbox +0x1a0).
        0x190 bResetNoiseTexture: u8,
    }
}

// ---------------------------------------------------------------------------
// Helpers.

/// The address of `field` of `object`.
fn address_of<S, T>(object: Ptr<S>, field: Field<S, T>) -> u32 {
    object.addr() + field.off
}

/// The object a `NiPointer` slot (or the head of a list) holds.
fn pointer_in_slot(e: &mut Engine, slot: u32) -> u32 {
    e.call(NI_POINTER_GET, &args![slot]).u32()
}

/// Stores `value` in a `NiPointer` slot.
fn assign_slot(e: &mut Engine, slot: u32, value: u32) {
    e.call(NI_POINTER_ASSIGN, &args![slot, value]);
}

/// An INI setting read as a `bool` (the byte at the setting's value).
fn setting_flag(e: &mut Engine, setting: u32) -> bool {
    let value = e.call(SETTING_VALUE, &args![setting]).u32();
    e.mem.u8(value) != 0
}

/// Whether the current cell is an interior (`TES::pInteriorCell` through
/// `005f36f0`; see the module notes).
fn in_interior(e: &mut Engine) -> bool {
    let tes = e.global::<u32>(TES_POINTER);
    e.call(TES_GET_INTERIOR_CELL, &args![tes]).u32() != 0
}

/// `byte [0x011c7a59]`: the flag `004e2180` returns.
fn water_skipped(e: &mut Engine) -> bool {
    e.call(WATER_SKIPPED_FLAG, &[]).bool()
}

/// Calls `visit` for every item of an `NiTPointerList`, the way every loop
/// of the unit does: the next position is fetched before the item is
/// visited, so the visitor may remove the item.
fn for_each_list_item(e: &mut Engine, list: u32, mut visit: impl FnMut(&mut Engine, u32)) {
    let mut position = pointer_in_slot(e, list);
    while position != 0 {
        let next = e.call(LIST_NEXT_POSITION, &args![list, position]).u32();
        let slot = e.call(LIST_ITEM_SLOT, &args![list, position]).u32();
        let item = e.mem.u32(slot);
        visit(e, item);
        position = next;
    }
}

/// `NiAVObject::GetProperty(node, 3)`: the node's water shader property.
fn water_shader_property(e: &mut Engine, node: u32) -> Ptr<WaterShaderProperty> {
    e.call(NODE_GET_PROPERTY, &args![node, 3u32]).ptr()
}

/// A `float` getter called with `ECX = this` (the result is in ST0).
fn float_getter(e: &mut Engine, getter: u32, this: u32) -> f32 {
    e.call(getter, &args![this]).f32()
}

/// Clears the reflection, refraction and depth texture slots of a water
/// shader property whose feature is off or whose INI setting forbids it,
/// then the normal-map slot.
fn clear_disabled_texture_slots(e: &mut Engine, property: Ptr<WaterShaderProperty>) {
    let reflections = e.get(property, WaterShaderProperty::bReflections);
    if !(reflections && setting_flag(e, SETTING_USE_WATER_REFLECTIONS)) {
        assign_slot(
            e,
            address_of(property, WaterShaderProperty::spReflectionMap),
            0,
        );
    }
    let refractions = e.get(property, WaterShaderProperty::bRefractions);
    if !(refractions && setting_flag(e, SETTING_USE_WATER_REFRACTIONS)) {
        assign_slot(
            e,
            address_of(property, WaterShaderProperty::spRefractionMap),
            0,
        );
    }
    let depth = e.get(property, WaterShaderProperty::bDepth);
    if !(depth && setting_flag(e, SETTING_USE_WATER_DEPTH)) {
        assign_slot(e, address_of(property, WaterShaderProperty::spDepthMap), 0);
    }
    assign_slot(
        e,
        address_of(property, WaterShaderProperty::spNoiseNormalMap),
        0,
    );
}

/// Gives the rendered texture in `slot` back to the texture manager, when
/// there is one (the slot itself is not cleared). Returns whether there was
/// one.
fn return_rendered_texture(e: &mut Engine, slot: u32) -> bool {
    if pointer_in_slot(e, slot) == 0 {
        return false;
    }
    let texture = pointer_in_slot(e, slot);
    let manager = e.call(TEXTURE_MANAGER, &[]).u32();
    e.call(RETURN_RENDERED_TEXTURE, &args![manager, texture]);
    true
}

/// Copies a 16-byte `NiPoint4`/`NiColorA` word for word.
fn copy_four_words(e: &mut Engine, from: u32, to: u32) {
    for i in 0..4 {
        let word = e.mem.u32(from + 4 * i);
        e.mem.set_u32(to + 4 * i, word);
    }
}

/// Stores a packed colour (red in the low byte) as a `NiColorA` with alpha
/// 1.0 at `destination`.
fn store_packed_color(e: &mut Engine, destination: u32, packed: u32) {
    let range: f64 = e.global(BYTE_RANGE);
    let red = ((packed & 0xff) as f64 / range) as f32;
    let green = (((packed >> 8) & 0xff) as f64 / range) as f32;
    let blue = (((packed >> 16) & 0xff) as f64 / range) as f32;
    e.with_stack(0x10, |e, color| {
        e.call(NI_POINT4_CONSTRUCT, &args![color, red, green, blue, 1.0f32]);
        copy_four_words(e, color.addr(), destination);
    });
}

// ---------------------------------------------------------------------------
// Translated functions.

// Translated from 004e21b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESWaterSystem::UpdatePlaceableWater` (Xbox PDB): decides which water
/// groups render this frame (a group renders while one of its water
/// references is near the `viewer` node, or when `force_update` is set),
/// sets up their reflection and depth passes, renders the depth maps and
/// updates the shader properties of the rendered groups. When no group
/// renders it gives the shared textures back.
///
/// `viewer` is an `NiAVObject` (the player's 3D); only its world position is
/// read.
pub fn tes_water_system_update_placeable_water(
    e: &mut Engine,
    this: Ptr<TESWaterSystem>,
    viewer: Ptr,
    force_update: bool,
) {
    let group_list = this.at(TESWaterSystem::PlaceableWaterGroupList).addr();
    let mut any_group_switched_on = false;
    e.set_global(ACTIVE_WATER_GROUPS, 0u32);

    // Which groups render: every group starts off and is switched on by a
    // water reference of its own that is in range.
    for_each_list_item(e, group_list, |e, item| {
        let group: Ptr<PlaceableWaterGroup> = Ptr::new(item);
        e.set(group, PlaceableWaterGroup::bRenderGroup, false);
        e.set(group, PlaceableWaterGroup::bRenderGroupReflections, false);
        e.set(group, PlaceableWaterGroup::bRenderDepth, false);
        e.set(
            group,
            PlaceableWaterGroup::bRenderSilhouetteReflections,
            false,
        );
        let members = group.at(PlaceableWaterGroup::PlaceableWaterList).addr();
        for_each_list_item(e, members, |e, reference| {
            if update_group_member(e, this, viewer, force_update, group, reference) {
                any_group_switched_on = true;
            }
        });
        settle_group_after_members(e, this, group);
    });

    let world_reflections = e.global::<u8>(RENDER_WORLD_REFLECTIONS) != 0;
    let sky_reflections = e.global::<u8>(RENDER_SKY_REFLECTIONS) != 0;
    if !any_group_switched_on && !world_reflections && !sky_reflections {
        // Nothing renders: give the shared textures back.
        return_rendered_texture(e, DEPTH_MAP);
        return_rendered_texture(e, WADING_WATER_HEIGHT_MAP);
        for_each_list_item(e, group_list, |e, item| {
            let group: Ptr<PlaceableWaterGroup> = Ptr::new(item);
            let water_type = e.get(group, PlaceableWaterGroup::pWaterType);
            if !water_type.is_null() {
                let slot = address_of(
                    water_type.cast::<TESWaterForm>(),
                    TESWaterForm::spNoiseNormalMap,
                );
                if return_rendered_texture(e, slot) {
                    assign_slot(e, slot, 0);
                }
            }
        });
        assign_slot(e, DEPTH_MAP, 0);
        assign_slot(e, WADING_WATER_HEIGHT_MAP, 0);
        return;
    }

    let interior = in_interior(e);
    if !interior {
        if e.global::<u8>(RENDER_WORLD_REFLECTIONS) != 0 {
            let stage = e.get(this, TESWaterSystem::iAccumulationCount);
            e.set_global(WORLD_REFLECTION_THREAD_STAGE, stage);
            e.set(
                this,
                TESWaterSystem::iAccumulationCount,
                stage.wrapping_add(1),
            );
            e.call(SETUP_WORLD_REFLECTIONS, &args![this, viewer]);
        }
        if e.global::<u8>(RENDER_SKY_REFLECTIONS) != 0 {
            let stage = e.get(this, TESWaterSystem::iAccumulationCount);
            e.set_global(SKY_REFLECTION_THREAD_STAGE, stage);
            e.set(
                this,
                TESWaterSystem::iAccumulationCount,
                stage.wrapping_add(1),
            );
            e.call(SETUP_SKY_REFLECTIONS, &args![this, viewer]);
        }
    }

    // Set up the passes of every rendered group.
    for_each_list_item(e, group_list, |e, item| {
        let group: Ptr<PlaceableWaterGroup> = Ptr::new(item);
        if !e.get(group, PlaceableWaterGroup::bRenderGroup) || water_skipped(e) {
            return;
        }
        e.call(
            MAP_REMOVE_ALL,
            &args![address_of(this, TESWaterSystem::ReflectionRefMap)],
        );
        e.call(
            MAP_REMOVE_ALL,
            &args![address_of(this, TESWaterSystem::DepthRefMap)],
        );
        e.call(
            LIST_REMOVE_ALL,
            &args![address_of(group, PlaceableWaterGroup::StaticDepthObjects)],
        );
        e.call(
            LIST_REMOVE_ALL,
            &args![address_of(group, PlaceableWaterGroup::DynamicDepthObjects)],
        );
        let stage = e.get(this, TESWaterSystem::iAccumulationCount);
        if stage.wrapping_add(1) < 0x10 {
            if setting_flag(e, SETTING_USE_WATER_REFLECTIONS)
                && e.get(group, PlaceableWaterGroup::bRenderGroupReflections)
            {
                let stage = e.get(this, TESWaterSystem::iAccumulationCount);
                e.set(group, PlaceableWaterGroup::iReflectionThreadStage, stage);
                e.set(
                    this,
                    TESWaterSystem::iAccumulationCount,
                    stage.wrapping_add(1),
                );
                let high_detail =
                    setting_flag(e, SETTING_FORCE_HIGH_DETAIL_REFLECTIONS) || in_interior(e);
                let low_detail: u32 = if high_detail { 0 } else { 1 };
                e.call(
                    SETUP_GROUP_REFLECTIONS,
                    &args![this, viewer, group, low_detail],
                );
            }
            if e.get(group, PlaceableWaterGroup::bRenderDepth)
                && setting_flag(e, SETTING_USE_WATER_DEPTH)
            {
                let stage = e.get(this, TESWaterSystem::iAccumulationCount);
                e.set(group, PlaceableWaterGroup::iDepthThreadStage, stage);
                e.set(
                    this,
                    TESWaterSystem::iAccumulationCount,
                    stage.wrapping_add(1),
                );
                e.call(SETUP_GROUP_DEPTH, &args![this, viewer, group]);
            }
        } else {
            // Too many groups for the 16 stages: log it and switch the group off.
            let count = e.call(LIST_COUNT, &args![group_list]).u32();
            let water_type = e.get(group, PlaceableWaterGroup::pWaterType);
            let name = e.vcall(water_type.addr(), WATER_FORM_GET_NAME, &[]).u32();
            let height = e
                .call(
                    PLANE_CONSTANT,
                    &args![address_of(group, PlaceableWaterGroup::ReflectWaterPlane)],
                )
                .f32();
            e.with_stack(0x100, |e, text| {
                e.call(
                    FORMATTED_PRINT,
                    &args![
                        text,
                        0x100u32,
                        TOO_MANY_GROUPS_MESSAGE,
                        height as f64,
                        name,
                        count
                    ],
                );
                e.call(LOG_MESSAGE, &args![FORMAT_STRING, text]);
            });
            e.set(group, PlaceableWaterGroup::bRenderGroup, false);
        }
    });

    if !in_interior(e) {
        if e.global::<u8>(RENDER_WORLD_REFLECTIONS) != 0 && !water_skipped(e) {
            e.call(FINISH_WORLD_REFLECTIONS, &args![this]);
        }
        if e.global::<u8>(RENDER_SKY_REFLECTIONS) != 0 && !water_skipped(e) {
            e.call(FINISH_SKY_REFLECTIONS, &args![this]);
        }
        for_each_list_item(e, group_list, |e, item| {
            let group: Ptr<PlaceableWaterGroup> = Ptr::new(item);
            if e.get(group, PlaceableWaterGroup::bRenderGroup) && !water_skipped(e) {
                e.call(FINISH_GROUP, &args![this, group]);
            } else {
                e.call(RELEASE_GROUP, &args![this, group, 0u32, 0u32]);
            }
        });
    } else {
        for_each_list_item(e, group_list, |e, item| {
            let group: Ptr<PlaceableWaterGroup> = Ptr::new(item);
            if e.get(group, PlaceableWaterGroup::bRenderGroup) && !water_skipped(e) {
                if setting_flag(e, SETTING_USE_WATER_REFLECTIONS)
                    && e.get(group, PlaceableWaterGroup::bRenderGroupReflections)
                {
                    e.call(FINISH_GROUP_INTERIOR, &args![this, group]);
                }
                e.call(FINISH_GROUP, &args![this, group]);
            } else {
                e.call(RELEASE_GROUP, &args![this, group, 0u32, 0u32]);
            }
        });
    }

    // Render the depth maps: the clear colour is set to the depth colour for
    // the duration and restored afterwards.
    render_depth_maps(e, this, viewer);

    e.call(
        MAP_REMOVE_ALL,
        &args![address_of(this, TESWaterSystem::WaterTypeUpdateMap)],
    );
    for_each_list_item(e, group_list, |e, item| {
        let group: Ptr<PlaceableWaterGroup> = Ptr::new(item);
        if e.get(group, PlaceableWaterGroup::bRenderGroup) {
            tes_water_system_update_water_shader_properties(e, this, group);
            fn_004e3d60(e, this, group);
        }
    });
}

/// What `UpdatePlaceableWater` does for one water reference of a group
/// (`reference` can be a null item): switches the group on when the
/// reference is in range, records what the group has to render (reflections,
/// depth, silhouette reflections) and drops the reference's shader textures
/// the settings do not allow. Returns whether this call switched the group
/// on.
fn update_group_member(
    e: &mut Engine,
    this: Ptr<TESWaterSystem>,
    viewer: Ptr,
    force_update: bool,
    group: Ptr<PlaceableWaterGroup>,
    reference: u32,
) -> bool {
    let mut switched_on = false;
    let base_form = e.call(REFERENCE_BASE_FORM, &args![reference]).u32();
    if reference != 0 && e.vcall(reference, REFERENCE_GET_3D, &[]).u32() != 0 {
        let parent_cell_matches = {
            let cell = e.call(REFERENCE_PARENT_CELL, &args![reference]).u32();
            cell != 0 && {
                let cell = e.call(REFERENCE_PARENT_CELL, &args![reference]).u32();
                e.call(CELL_BYTE_IS_SIX, &args![cell]).bool()
            }
        };
        let relevant = parent_cell_matches || (fn_004e32c0(e, base_form) && !in_interior(e));
        if relevant {
            let node = e.vcall(reference, REFERENCE_GET_3D, &[]).u32();
            let culled = e.call(NODE_IS_CULLED, &args![node]).bool();
            let in_range = !culled
                && e.call(REFERENCE_IS_IN_RANGE, &args![this, reference, viewer])
                    .bool();
            if in_range || force_update {
                if !e.get(group, PlaceableWaterGroup::bRenderGroup) {
                    let active = e.global::<u32>(ACTIVE_WATER_GROUPS);
                    e.set_global(ACTIVE_WATER_GROUPS, active.wrapping_add(1));
                    e.set(group, PlaceableWaterGroup::bRenderGroup, true);
                    switched_on = true;
                    if !water_skipped(e) {
                        if in_interior(e) {
                            e.set(group, PlaceableWaterGroup::bRenderGroupReflections, true);
                        } else if e.get(group, PlaceableWaterGroup::bGroupAtWorldSpaceWaterHeight) {
                            e.set_global(RENDER_WORLD_REFLECTIONS, 1u8);
                        } else {
                            e.set_global(RENDER_SKY_REFLECTIONS, 1u8);
                        }
                    }
                }
                if fn_004e32e0(e, base_form) && fn_004e3280(e, base_form) {
                    let actors = address_of(group, PlaceableWaterGroup::ActorsInWaterList);
                    let objects = address_of(group, PlaceableWaterGroup::ObjectInWaterList);
                    let needs_depth = !e.call(LIST_IS_EMPTY, &args![actors]).bool()
                        || !e.call(LIST_IS_EMPTY, &args![objects]).bool()
                        || needs_depth_for_reference(e, reference, base_form);
                    if needs_depth {
                        e.set(group, PlaceableWaterGroup::bRenderDepth, true);
                    }
                }
                if !in_interior(e)
                    && setting_flag(e, SETTING_AUTO_SILHOUETTE_REFLECTIONS)
                    && (e.call(FORM_FLAG_TEST_40000000, &args![base_form]).bool()
                        || setting_flag(e, SETTING_FORCE_LOW_DETAIL_REFLECTIONS)
                        || fn_004e3300(e, base_form))
                {
                    e.set(
                        group,
                        PlaceableWaterGroup::bRenderSilhouetteReflections,
                        true,
                    );
                }
            }
        }
    }

    // The reference's own shader property: a reference whose group is not
    // rendering loses all its textures; one that renders keeps those the
    // settings allow.
    let rendering = e.get(group, PlaceableWaterGroup::bRenderGroup);
    let node = e.call(WATER_REFERENCE_3D, &args![this, reference]).u32();
    if node != 0 {
        let owner = e.call(NODE_OWNER, &args![node]).u32();
        if owner != 0 && e.call(OWNER_TYPE, &args![owner]).u32() == WATER_OWNER_TYPE {
            let property = water_shader_property(e, node);
            if reference != 0 && !rendering {
                assign_slot(
                    e,
                    address_of(property, WaterShaderProperty::spReflectionMap),
                    0,
                );
                assign_slot(
                    e,
                    address_of(property, WaterShaderProperty::spRefractionMap),
                    0,
                );
                assign_slot(
                    e,
                    address_of(property, WaterShaderProperty::spNoiseNormalMap),
                    0,
                );
                assign_slot(e, address_of(property, WaterShaderProperty::spDepthMap), 0);
            } else {
                clear_disabled_texture_slots(e, property);
            }
        }
    }
    switched_on
}

/// The part of the depth decision that looks at the reference itself, when
/// nothing is in the water: the reflected references of its extra data, then
/// the base form flags.
fn needs_depth_for_reference(e: &mut Engine, reference: u32, base_form: u32) -> bool {
    let extra = e.call(REFERENCE_EXTRA_DATA_LIST, &args![reference]).u32();
    let reflected = e
        .call(EXTRA_DATA_LIST_GET_REFLECTED_REFS, &args![extra])
        .u32();
    if reflected != 0 {
        let extra = e.call(REFERENCE_EXTRA_DATA_LIST, &args![reference]).u32();
        let reflected = e
            .call(EXTRA_DATA_LIST_GET_REFLECTED_REFS, &args![extra])
            .u32();
        if !e.call(WORDS_ARE_ZERO, &args![reflected]).bool() {
            return true;
        }
    }
    if fn_004e32a0(e, base_form) {
        return true;
    }
    if e.call(FORM_FLAG_TEST_40000000, &args![base_form]).bool() {
        let object = fn_004e3260(e);
        return !e.call(WORDS_ARE_ZERO, &args![object]).bool();
    }
    false
}

/// What `UpdatePlaceableWater` does for a group once all its references were
/// looked at: a group that does not render hands back its wading-water
/// textures and is released; one that does render keeps only the textures
/// the settings allow, and gives the shared depth map back when depth is off.
fn settle_group_after_members(
    e: &mut Engine,
    this: Ptr<TESWaterSystem>,
    group: Ptr<PlaceableWaterGroup>,
) {
    let wading_geometry = address_of(group, PlaceableWaterGroup::spWadingWaterGeometry);
    if !e.get(group, PlaceableWaterGroup::bRenderGroup) {
        if pointer_in_slot(e, wading_geometry) != 0 {
            let geometry = pointer_in_slot(e, wading_geometry);
            let property = water_shader_property(e, geometry);
            assign_slot(
                e,
                address_of(property, WaterShaderProperty::spReflectionMap),
                0,
            );
            assign_slot(
                e,
                address_of(property, WaterShaderProperty::spRefractionMap),
                0,
            );
            assign_slot(e, address_of(property, WaterShaderProperty::spDepthMap), 0);
            assign_slot(
                e,
                address_of(property, WaterShaderProperty::spNoiseNormalMap),
                0,
            );
        }
        e.call(RELEASE_GROUP, &args![this, group, 0u32, 0u32]);
        return;
    }
    if pointer_in_slot(e, wading_geometry) != 0 {
        let geometry = pointer_in_slot(e, wading_geometry);
        let property = water_shader_property(e, geometry);
        clear_disabled_texture_slots(e, property);
    }
    if !setting_flag(e, SETTING_USE_WATER_REFLECTIONS) {
        e.call(RELEASE_GROUP, &args![this, group, 0u32, 0u32]);
    }
    if !setting_flag(e, SETTING_USE_WATER_DEPTH) {
        return_rendered_texture(e, DEPTH_MAP);
        assign_slot(e, DEPTH_MAP, 0);
    }
}

/// The depth-map part of `UpdatePlaceableWater`: with the clear colour set
/// to the depth colour and the viewer's position sent to `004e20c0`, every
/// rendered group that wants depth is rendered into the shared depth map and
/// put through the image-space depth effect; the clear colour is restored
/// at the end.
fn render_depth_maps(e: &mut Engine, this: Ptr<TESWaterSystem>, viewer: Ptr) {
    let group_list = this.at(TESWaterSystem::PlaceableWaterGroupList).addr();
    let mut first_group = true;
    e.with_stack(0x40, |e, scratch| {
        let saved_clear_color = scratch.addr();
        let depth_color = saved_clear_color + 0x10;
        let position_point = saved_clear_color + 0x20;
        e.call(
            NI_POINT4_CONSTRUCT,
            &args![saved_clear_color, 0.0f32, 0.0f32, 0.0f32, 0.0f32],
        );
        let renderer = e.call(RENDERER, &[]).u32();
        e.vcall(
            renderer,
            RENDERER_GET_CLEAR_COLOR,
            &args![saved_clear_color],
        );
        for i in 0..4 {
            let word = e.mem.u32(DEPTH_CLEAR_COLOR + 4 * i);
            e.mem.set_u32(depth_color + 4 * i, word);
        }
        let renderer = e.call(RENDERER, &[]).u32();
        e.vcall(renderer, RENDERER_SET_CLEAR_COLOR, &args![depth_color]);
        let translate = e.call(NODE_WORLD_TRANSLATE, &args![viewer]).u32();
        let x = e.mem.u32(translate);
        let y = e.mem.u32(translate + 4);
        let z = e.mem.u32(translate + 8);
        e.call(
            NI_POINT4_CONSTRUCT,
            &args![
                position_point,
                f32::from_bits(x),
                f32::from_bits(y),
                f32::from_bits(z),
                0.0f32
            ],
        );
        let words: Vec<u32> = (0..4).map(|i| e.mem.u32(position_point + 4 * i)).collect();
        e.call(
            VIEWER_POSITION_UPDATE,
            &args![0x1eu32, words[0], words[1], words[2], words[3]],
        );

        for_each_list_item(e, group_list, |e, item| {
            let group: Ptr<PlaceableWaterGroup> = Ptr::new(item);
            if !(e.get(group, PlaceableWaterGroup::bRenderGroup)
                && setting_flag(e, SETTING_USE_WATER_DEPTH)
                && e.get(group, PlaceableWaterGroup::bRenderDepth)
                && !water_skipped(e))
            {
                return;
            }
            let texture = pointer_in_slot(e, DEPTH_MAP);
            let stopped = e.call(RENDERED_TEXTURE_STOP, &args![texture]).u32();
            let mode: u32 = if first_group { 7 } else { 0 };
            e.call(RENDER_TARGET_SET, &args![mode, stopped]);
            let camera = pointer_in_slot(e, address_of(group, PlaceableWaterGroup::spDepthCamera));
            let stencil = e.get(group, PlaceableWaterGroup::iStencilBitMask);
            e.call(
                RENDER_GROUP_DEPTH,
                &args![this, camera, group, first_group as u32, stencil & 0xffff],
            );
            e.call(BATCH_RENDERER_END_PASS, &[]);
            e.set_global(LAST_DEPTH_STENCIL_MASK, stencil);
            e.with_stack(SHADER_PARAM_SIZE, |e, param| {
                e.call(SHADER_PARAM_CONSTRUCT, &args![param]);
                e.call(SHADER_PARAM_INIT_CONSTANT_MAP, &args![param, 0u32, 1u32]);
                e.call(
                    SHADER_PARAM_SET_PIXEL_CONSTANT,
                    &args![param, 0u32, 1.0f32, 0.0f32, 0.0f32, 1.0f32],
                );
                e.call(SET_STENCIL_STATE, &args![2u32, 0xffu32, stencil, 0u32]);
                let texture = pointer_in_slot(e, DEPTH_MAP);
                let renderer = e.call(RENDERER, &[]).u32();
                let manager = fn_004e3270(e);
                e.call(
                    IMAGE_SPACE_RENDER_EFFECT,
                    &args![manager, 0x28u32, renderer, texture, param, 0u32],
                );
                e.call(AFTER_GROUP_DEPTH, &args![this, group, stencil & 0xffff]);
                e.call(RENDER_TARGET_RESET, &[]);
                first_group = false;
                e.call(SHADER_PARAM_DESTRUCT, &args![param]);
            });
        });

        let renderer = e.call(RENDERER, &[]).u32();
        e.vcall(
            renderer,
            RENDERER_SET_CLEAR_COLOR,
            &args![saved_clear_color],
        );
    });
}

// Translated from 004e3260 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of the static object at `0x011ca13c`, which
/// `UpdatePlaceableWater` asks `008256d0` (both words zero) about.
pub fn fn_004e3260(_e: &mut Engine) -> u32 {
    WATER_OBJECT_011CA13C
}

// Translated from 004e3270 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `ImageSpaceManager*` global (`0x011f91ac`) the water passes render
/// effects with.
pub fn fn_004e3270(e: &mut Engine) -> u32 {
    e.global(IMAGE_SPACE_MANAGER)
}

/// A base form flag test: whether `mask` is set in the form flags
/// (`FORM_FLAG_TEST`).
fn form_flag(e: &mut Engine, form: u32, mask: u32) -> bool {
    e.call(FORM_FLAG_TEST, &args![form, mask]).bool()
}

// Translated from 004e3280 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether bit `0x200` is set in the form flags.
pub fn fn_004e3280(e: &mut Engine, form: u32) -> bool {
    form_flag(e, form, FORM_FLAG_0200)
}

// Translated from 004e32a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether bit `0x800` is set in the form flags.
pub fn fn_004e32a0(e: &mut Engine, form: u32) -> bool {
    form_flag(e, form, FORM_FLAG_0800)
}

// Translated from 004e32c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether bit `0x08000000` is set in the form flags.
pub fn fn_004e32c0(e: &mut Engine, form: u32) -> bool {
    form_flag(e, form, FORM_FLAG_8000000)
}

// Translated from 004e32e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether bit `0x10000000` is set in the form flags.
pub fn fn_004e32e0(e: &mut Engine, form: u32) -> bool {
    form_flag(e, form, FORM_FLAG_10000000)
}

// Translated from 004e3300 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether bit `0x40000` is set in the form flags.
pub fn fn_004e3300(e: &mut Engine, form: u32) -> bool {
    form_flag(e, form, FORM_FLAG_40000)
}

// Translated from 004e3320 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESWaterSystem::UpdateLODWater` (Xbox PDB): the same decision as
/// `UpdatePlaceableWater` for the single LOD water group. Logs a message
/// when it runs in an interior (LOD water must not). A water reference in
/// range (or any, with `force_update`) switches the group on and points its
/// shader property at the shared world or sky reflection map; one that is
/// not loses its reflection map; when no reference is left the rendered
/// group's shader properties are updated.
pub fn tes_water_system_update_lod_water(
    e: &mut Engine,
    this: Ptr<TESWaterSystem>,
    viewer: Ptr,
    force_update: bool,
) {
    if e.get(this, TESWaterSystem::pLODWaterGroup).is_null() {
        return;
    }
    if in_interior(e) {
        e.call(LOG_MESSAGE, &args![LOD_IN_INTERIOR_MESSAGE]);
        return;
    }
    let lod_group = e.get(this, TESWaterSystem::pLODWaterGroup);
    e.set(lod_group, PlaceableWaterGroup::bRenderGroup, false);
    let lod_group = e.get(this, TESWaterSystem::pLODWaterGroup);
    let members = lod_group.at(PlaceableWaterGroup::PlaceableWaterList).addr();
    for_each_list_item(e, members, |e, reference| {
        let mut active = false;
        if e.vcall(reference, REFERENCE_GET_3D, &[]).u32() != 0 {
            let node = e.vcall(reference, REFERENCE_GET_3D, &[]).u32();
            let culled = e.call(NODE_IS_CULLED, &args![node]).bool();
            active = !culled
                && e.call(REFERENCE_IS_IN_RANGE, &args![this, reference, viewer])
                    .bool();
        }
        if active || force_update {
            let lod_group = e.get(this, TESWaterSystem::pLODWaterGroup);
            e.set(lod_group, PlaceableWaterGroup::bRenderGroup, true);
            let node = e.call(WATER_REFERENCE_3D, &args![this, reference]).u32();
            if node != 0 {
                let property = water_shader_property(e, node);
                if !water_skipped(e) {
                    let reflection_map = address_of(property, WaterShaderProperty::spReflectionMap);
                    if e.get(property, WaterShaderProperty::bFullReflections) {
                        e.set_global(RENDER_WORLD_REFLECTIONS, 1u8);
                        e.call(
                            NI_POINTER_ASSIGN_FROM,
                            &args![reflection_map, WORLD_REFLECTION_MAP],
                        );
                    } else {
                        e.set_global(RENDER_SKY_REFLECTIONS, 1u8);
                        e.call(
                            NI_POINTER_ASSIGN_FROM,
                            &args![reflection_map, SKY_REFLECTION_MAP],
                        );
                    }
                } else {
                    let lod_group = e.get(this, TESWaterSystem::pLODWaterGroup);
                    e.call(RELEASE_GROUP, &args![this, lod_group, 0u32, 0u32]);
                }
            }
        } else if e.vcall(reference, REFERENCE_GET_3D, &[]).u32() != 0 {
            let node = e.call(WATER_REFERENCE_3D, &args![this, reference]).u32();
            if node != 0 {
                let property = water_shader_property(e, node);
                assign_slot(
                    e,
                    address_of(property, WaterShaderProperty::spReflectionMap),
                    0,
                );
            }
        }
    });
    let lod_group = e.get(this, TESWaterSystem::pLODWaterGroup);
    if e.get(lod_group, PlaceableWaterGroup::bRenderGroup) {
        let lod_group = e.get(this, TESWaterSystem::pLODWaterGroup);
        tes_water_system_update_water_shader_properties(e, this, lod_group);
    }
}

// Translated from 004e3520 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESWaterSystem::UpdateWaterShaderProperties` (Xbox PDB): updates the
/// shader property of every water reference of `group`.
pub fn tes_water_system_update_water_shader_properties(
    e: &mut Engine,
    this: Ptr<TESWaterSystem>,
    group: Ptr<PlaceableWaterGroup>,
) {
    let members = group.at(PlaceableWaterGroup::PlaceableWaterList).addr();
    for_each_list_item(e, members, |e, reference| {
        tes_water_system_update_water_shader_properties_ov2(e, this, group, reference);
    });
}

// Translated from 004e3590 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESWaterSystem::UpdateWaterShaderProperties_ov2` (Xbox PDB): copies the
/// water form of one water `reference` into its shader property. The
/// property is only touched when the reference's 3D object has the water
/// owner type and the property has `bUpdateConstants` set: colours (shallow,
/// deep, reflection), the form's amounts, the fog values (the underwater
/// ones when the player is underwater), the settings-controlled flags
/// (reflections, refractions, depth) and the depth falloff. When a flag
/// changed, the render-pass lists of the property and of the group's
/// wading-water property are reset.
pub fn tes_water_system_update_water_shader_properties_ov2(
    e: &mut Engine,
    this: Ptr<TESWaterSystem>,
    group: Ptr<PlaceableWaterGroup>,
    reference: u32,
) {
    let node = e.call(WATER_REFERENCE_3D, &args![this, reference]).u32();
    if node == 0 || e.vcall(node, NODE_PROPERTIES_VIRTUAL, &[]).u32() == 0 {
        return;
    }
    let owner = e.call(NODE_OWNER, &args![node]).u32();
    if owner == 0 || e.call(OWNER_TYPE, &args![owner]).u32() != WATER_OWNER_TYPE {
        return;
    }
    let property = water_shader_property(e, node);
    let base_form = e.call(REFERENCE_BASE_FORM, &args![reference]).u32();
    let water_form = e.vcall(base_form, BASE_FORM_GET_WATER_FORM, &[]).u32();
    if property.is_null()
        || water_form == 0
        || !e.get(property, WaterShaderProperty::bUpdateConstants)
    {
        return;
    }

    let packed = e.call(0x004f_9bf0, &args![water_form]).u32();
    store_packed_color(
        e,
        address_of(property, WaterShaderProperty::pShallowColor),
        packed,
    );
    let packed = e.call(0x009e_e040, &args![water_form]).u32();
    store_packed_color(
        e,
        address_of(property, WaterShaderProperty::pDeepColor),
        packed,
    );
    let packed = e.call(0x008d_80e0, &args![water_form]).u32();
    store_packed_color(
        e,
        address_of(property, WaterShaderProperty::pReflectionColor),
        packed,
    );

    let value = float_getter(e, 0x0064_4930, water_form);
    e.set(property, WaterShaderProperty::value_00b8, value);
    let value = float_getter(e, 0x008d_01e0, water_form);
    e.set(property, WaterShaderProperty::value_00bc, value);
    let value = float_getter(e, 0x0058_0100, water_form);
    e.set(property, WaterShaderProperty::value_00c0, value);
    let value = float_getter(e, 0x004a_0d90, water_form);
    e.set(property, WaterShaderProperty::value_00c4, value);
    let value = fn_004e3cc0(e, Ptr::new(water_form));
    e.set(property, WaterShaderProperty::fFresnelAmount, value);
    let value = float_getter(e, 0x004a_0db0, water_form);
    e.set(property, WaterShaderProperty::value_0100, value);
    let value = float_getter(e, 0x0064_6f50, water_form);
    e.set(property, WaterShaderProperty::value_0104, value);
    let value = fn_004e3d20(e, Ptr::new(water_form));
    e.set(property, WaterShaderProperty::value_00d0, value);
    let value = fn_004e3d40(e, Ptr::new(water_form));
    e.set(property, WaterShaderProperty::value_00d4, value);
    let value = float_getter(e, 0x009a_5480, water_form);
    e.set(property, WaterShaderProperty::fNoiseScale, value);
    let value = float_getter(e, 0x0088_5d70, water_form);
    e.set_global(WATER_SHADER_FLOAT, value);

    if water_skipped(e) || fn_004e3c40(e) != 0 {
        let fog_far = float_getter(e, 0x0050_7b20, water_form);
        e.set(property, WaterShaderProperty::fFogFar, fog_far);
        let far = float_getter(e, 0x0050_7b20, water_form);
        let near = fn_004e3d00(e, Ptr::new(water_form));
        e.set(
            property,
            WaterShaderProperty::fFogRange,
            (far as f64 - near as f64) as f32,
        );
        if fn_004e3c70(e, base_form) {
            let value = float_getter(e, 0x0050_8070, water_form);
            e.set(property, WaterShaderProperty::fFogAmount, value);
        }
    } else {
        let fog_far = float_getter(e, 0x009b_88a0, water_form);
        e.set(property, WaterShaderProperty::fFogFar, fog_far);
        let far = float_getter(e, 0x009b_88a0, water_form);
        let near = float_getter(e, 0x0081_2870, water_form);
        e.set(
            property,
            WaterShaderProperty::fFogRange,
            (far as f64 - near as f64) as f32,
        );
        if fn_004e3c70(e, base_form) {
            let value = fn_004e3ce0(e, Ptr::new(water_form));
            e.set(property, WaterShaderProperty::fFogAmount, value);
        } else {
            e.set(property, WaterShaderProperty::fFogAmount, 0.0f32);
        }
    }
    let value = float_getter(e, 0x0094_42a0, water_form);
    e.set(property, WaterShaderProperty::fUVScale, value);

    let old_reflections = e.get(property, WaterShaderProperty::bReflections);
    let old_refractions = e.get(property, WaterShaderProperty::bRefractions);
    let old_depth = e.get(property, WaterShaderProperty::bDepth);
    let reflections = fn_004e3c50(e, base_form) && setting_flag(e, SETTING_USE_WATER_REFLECTIONS);
    e.set(property, WaterShaderProperty::bReflections, reflections);
    let refractions = fn_004e3280(e, base_form) && setting_flag(e, SETTING_USE_WATER_REFRACTIONS);
    e.set(property, WaterShaderProperty::bRefractions, refractions);
    let depth = fn_004e32e0(e, base_form) && setting_flag(e, SETTING_USE_WATER_DEPTH);
    e.set(property, WaterShaderProperty::bDepth, depth);
    if reflections != old_reflections || refractions != old_refractions || depth != old_depth {
        fn_004e3c00(e, property);
        let wading = address_of(group, PlaceableWaterGroup::spWadingWaterGeometry);
        if pointer_in_slot(e, wading) != 0 {
            let geometry = pointer_in_slot(e, wading);
            let wading_property = water_shader_property(e, geometry);
            let passes = e.vcall(
                wading_property.addr(),
                PROPERTY_PASS_COUNT_VIRTUAL,
                &args![0u32],
            );
            if passes.u16() != 0 {
                fn_004e3c00(e, wading_property);
            }
        }
    }
    let flag = fn_004e3ca0(e, base_form);
    e.set(property, WaterShaderProperty::bObjectTexCoords, flag);

    let start = float_getter(e, 0x0064_4950, water_form) as f64;
    let end = float_getter(e, 0x0064_4970, water_form);
    if start == end as f64 {
        let minus_one: f32 = e.global(MINUS_ONE);
        e.set(property, WaterShaderProperty::value_00d8, minus_one);
        e.set(property, WaterShaderProperty::value_00dc, 0.0f32);
    } else {
        let value = float_getter(e, 0x0064_4950, water_form);
        e.set(property, WaterShaderProperty::value_00d8, value);
        let value = float_getter(e, 0x0064_4970, water_form);
        e.set(property, WaterShaderProperty::value_00dc, value);
    }
}

// Translated from 004e3c00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Resets the render-pass list of a shader property: when it has one
/// (`pRenderPassList`, +0x3c), `004e3c20` clears the list's +0x10 field.
pub fn fn_004e3c00(e: &mut Engine, property: Ptr<WaterShaderProperty>) {
    let list = e.get(property, WaterShaderProperty::pRenderPassList);
    if !list.is_null() {
        fn_004e3c20(e, list);
    }
}

// Translated from 004e3c20 (decompiled, FalloutNV.exe 1.4.0.525)
/// Clears the word at +0x10 of a render-pass list.
pub fn fn_004e3c20(e: &mut Engine, list: Ptr) {
    e.mem.set_u32(list.addr() + 0x10, 0);
}

// Translated from 004e3c40 (decompiled, FalloutNV.exe 1.4.0.525)
/// The byte flag at `0x011c7a58`.
pub fn fn_004e3c40(e: &mut Engine) -> u8 {
    e.global(WATER_FLAG_011C7A58)
}

// Translated from 004e3c50 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether bit `1` is set in the form flags.
pub fn fn_004e3c50(e: &mut Engine, form: u32) -> bool {
    form_flag(e, form, FORM_FLAG_0001)
}

// Translated from 004e3c70 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether bit `0x80000000` is clear in the form flags.
pub fn fn_004e3c70(e: &mut Engine, form: u32) -> bool {
    !form_flag(e, form, FORM_FLAG_80000000)
}

// Translated from 004e3ca0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether bit `0x20000000` is set in the form flags.
pub fn fn_004e3ca0(e: &mut Engine, form: u32) -> bool {
    form_flag(e, form, FORM_FLAG_20000000)
}

// Translated from 004e3cc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESWaterForm`'s `Data.fFresnelAmount` (the float at +0x9c).
pub fn fn_004e3cc0(e: &mut Engine, form: Ptr<TESWaterForm>) -> f32 {
    e.get(form, TESWaterForm::fFresnelAmount)
}

// Translated from 004e3ce0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESWaterForm`'s `Data.fFogAmount` (the float at +0x108).
pub fn fn_004e3ce0(e: &mut Engine, form: Ptr<TESWaterForm>) -> f32 {
    e.get(form, TESWaterForm::fFogAmount)
}

// Translated from 004e3d00 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESWaterForm`'s `Data.fUnderwaterFogDistNear` (the float at +0x114).
pub fn fn_004e3d00(e: &mut Engine, form: Ptr<TESWaterForm>) -> f32 {
    e.get(form, TESWaterForm::fUnderwaterFogDistNear)
}

// Translated from 004e3d20 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESWaterForm`'s `Data.fLightRadius` (the float at +0x128).
pub fn fn_004e3d20(e: &mut Engine, form: Ptr<TESWaterForm>) -> f32 {
    e.get(form, TESWaterForm::fLightRadius)
}

// Translated from 004e3d40 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESWaterForm`'s `Data.fLightBrightness` (the float at +0x12c).
pub fn fn_004e3d40(e: &mut Engine, form: Ptr<TESWaterForm>) -> f32 {
    e.get(form, TESWaterForm::fLightBrightness)
}

// Translated from 004e3d60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Per-frame water form update for the references of `group`: for every
/// reference whose base form has a valid water form, makes sure the water
/// shader property has its noise map (the system's shared one, or the form's
/// own, loading it when needed), creates the form's rendered noise texture
/// once, shares it with the property, scrolls the form's three texture
/// scroll rows by the wind and wraps them into 0..1, publishes the rows and
/// the blend factors for the shader and renders the noise map when the
/// water type has not been updated yet this frame.
pub fn fn_004e3d60(e: &mut Engine, this: Ptr<TESWaterSystem>, group: Ptr<PlaceableWaterGroup>) {
    let members = group.at(PlaceableWaterGroup::PlaceableWaterList).addr();
    for_each_list_item(e, members, |e, reference| {
        update_water_form_for_reference(e, this, reference);
    });
}

/// One iteration of `fn_004e3d60`.
fn update_water_form_for_reference(e: &mut Engine, this: Ptr<TESWaterSystem>, reference: u32) {
    let base_form = e.call(REFERENCE_BASE_FORM, &args![reference]).u32();
    let water_form_address = e.vcall(base_form, BASE_FORM_GET_WATER_FORM, &[]).u32();
    if water_form_address == 0
        || e.call(WATER_FORM_HAS_NOISE_TEXTURE, &args![water_form_address])
            .u32()
            == 0
    {
        return;
    }
    let water_form: Ptr<TESWaterForm> = Ptr::new(water_form_address);

    let flush_object = fn_004e4680(e);
    if fn_004e4690(e, flush_object) == 0 {
        let flush_object = fn_004e4680(e);
        e.call(FLUSH_OBJECT_FLUSH, &args![flush_object]);
    }

    let node = e.call(WATER_REFERENCE_3D, &args![this, reference]).u32();
    if node == 0 {
        return;
    }
    let owner = e.call(NODE_OWNER, &args![node]).u32();
    if owner == 0 || e.call(OWNER_TYPE, &args![owner]).u32() != WATER_OWNER_TYPE {
        return;
    }
    let property = water_shader_property(e, node);
    let noise_height_map = address_of(property, WaterShaderProperty::spNoiseHeightMap);

    // The property's noise map: the system's shared one, or the form's own.
    if setting_flag(e, SETTING_USE_PER_WORLD_SPACE_NOISE) {
        if pointer_in_slot(e, noise_height_map) == 0 {
            fn_004e45a0(e, this, noise_height_map);
        }
    } else if pointer_in_slot(e, noise_height_map) == 0 || fn_004e4640(e, water_form) != 0 {
        e.with_stack(8, |e, texture| {
            e.call(POINTER_TEMP_CONSTRUCT, &args![texture, 0u32]);
            e.call(WATER_FORM_COPY_NOISE_TEXTURE, &args![water_form, texture]);
            if pointer_in_slot(e, texture.addr()) != 0 {
                e.call(
                    WATER_FORM_COPY_NOISE_TEXTURE,
                    &args![water_form, noise_height_map],
                );
            } else {
                let name = e.call(WATER_FORM_NOISE_NAME, &args![water_form]).u32();
                let tes = e.global::<u32>(TES_POINTER);
                e.call(
                    CREATE_TEXTURE_IMAGE,
                    &args![tes, name, noise_height_map, 1u32, 0u32],
                );
            }
            fn_004e4660(e, water_form, 0);
            e.call(POINTER_TEMP_DESTRUCT, &args![texture]);
        });
    }

    // The form's rendered noise texture, created once from the property's.
    let form_noise_map = address_of(water_form, TESWaterForm::spNoiseNormalMap);
    if pointer_in_slot(e, noise_height_map) != 0 && pointer_in_slot(e, form_noise_map) == 0 {
        let noise = pointer_in_slot(e, noise_height_map);
        let width = e.vcall(noise, TEXTURE_WIDTH_VIRTUAL, &[]).u32();
        e.set_global(NOISE_WIDTH, width);
        let noise = pointer_in_slot(e, noise_height_map);
        let height = e.vcall(noise, TEXTURE_HEIGHT_VIRTUAL, &[]).u32();
        e.set_global(NOISE_HEIGHT, height);
        let renderer = e.call(RENDERER, &[]).u32();
        let manager = e.call(TEXTURE_MANAGER, &[]).u32();
        let created = e.call(
            CREATE_RENDERED_TEXTURE,
            &args![manager, renderer, 0x13u32, 0u32, 0u32, 0u32],
        );
        assign_slot(e, form_noise_map, created.u32());
    }
    e.call(
        NI_POINTER_ASSIGN_FROM,
        &args![
            address_of(property, WaterShaderProperty::spNoiseNormalMap),
            form_noise_map
        ],
    );

    // Only the first reference of a water form in an update scrolls and
    // renders its noise.
    let update_map = address_of(this, TESWaterSystem::WaterTypeUpdateMap);
    let already_updated = e.with_stack(1, |e, flag| {
        e.call(WATER_TYPE_MAP_FIND, &args![update_map, water_form, flag])
            .bool()
    });
    if already_updated {
        return;
    }
    if pointer_in_slot(e, noise_height_map) != 0 {
        scroll_and_render_noise(e, water_form, property);
    }
    e.call(WATER_TYPE_MAP_SET, &args![update_map, water_form, 1u32]);
}

/// A `float` getter of a water form that the texture scroll multiplies by
/// the time step (`this` is the form).
type ScrollCoefficient = fn(&mut Engine, Ptr<TESWaterForm>) -> f32;
/// `sinf` / `cosf` as `fn_004e44b0` / `fn_004e4470`.
type TrigFunction = fn(&mut Engine, f32) -> f32;

/// The scrolling and noise render of `fn_004e3d60`, for the first reference
/// of a water form in an update: scrolls the form's three texture rows by
/// their wind, wraps them into 0..1, publishes them (and the height scales
/// and amplitudes) for the shader and renders the noise map.
fn scroll_and_render_noise(
    e: &mut Engine,
    water_form: Ptr<TESWaterForm>,
    property: Ptr<WaterShaderProperty>,
) {
    let noise_scale = e.get(property, WaterShaderProperty::fNoiseScale);
    e.set_global(SHADER_NOISE_BASE, noise_scale);
    let time_step = e.call(FLOAT_TABLE_ENTRY, &args![1u32]).f32();
    let direction0 = fn_004e45e0(e, water_form);
    let direction1 = float_getter(e, 0x0096_6a20, water_form.addr());
    let direction2 = fn_004e4600(e, water_form);
    let to_radians: f64 = e.global(DEGREES_TO_RADIANS);

    // Each row moves by (speed * time step) times the sine (x) and cosine
    // (y) of its wind direction.
    let speed0: ScrollCoefficient = |e, form| float_getter(e, 0x0064_47d0, form.addr());
    let speed2: ScrollCoefficient = |e, form| float_getter(e, 0x0064_47f0, form.addr());
    let rows: [(
        Field<TESWaterForm, f32>,
        f32,
        TrigFunction,
        ScrollCoefficient,
    ); 6] = [
        (TESWaterForm::fTexScroll0R, direction0, fn_004e44b0, speed0),
        (TESWaterForm::fTexScroll0G, direction0, fn_004e4470, speed0),
        (
            TESWaterForm::fTexScroll1R,
            direction1,
            fn_004e44b0,
            fn_004e4620,
        ),
        (
            TESWaterForm::fTexScroll1G,
            direction1,
            fn_004e4470,
            fn_004e4620,
        ),
        (TESWaterForm::fTexScroll2R, direction2, fn_004e44b0, speed2),
        (TESWaterForm::fTexScroll2G, direction2, fn_004e4470, speed2),
    ];
    for (field, direction, trig, speed) in rows {
        let angle = (direction as f64 * to_radians) as f32;
        let trig_value = trig(e, angle) as f64;
        let speed_value = speed(e, water_form) as f64;
        let old = e.get(water_form, field);
        let new = ((speed_value * time_step as f64) * trig_value + old as f64) as f32;
        e.set(water_form, field, new);
    }
    for field in [
        TESWaterForm::fTexScroll0R,
        TESWaterForm::fTexScroll0G,
        TESWaterForm::fTexScroll1R,
        TESWaterForm::fTexScroll1G,
        TESWaterForm::fTexScroll2R,
        TESWaterForm::fTexScroll2G,
    ] {
        bs_wrap(e, Ptr::new(address_of(water_form, field)), 0.0, 1.0);
    }

    // The three scroll rows (four words each from +0x34, +0x44, +0x54 of the
    // form), published after the noise scale.
    for (row, source) in [0x34u32, 0x44, 0x54].into_iter().enumerate() {
        for i in 0..4 {
            let word = e.mem.u32(water_form.addr() + source + 4 * i);
            e.mem
                .set_u32(SHADER_NOISE_BASE + 4 + 16 * row as u32 + 4 * i, word);
        }
    }

    // Three height scales, then three amplitudes.
    let scale = fn_004e45c0(e, water_form);
    publish_height_scale(e, scale, SHADER_NOISE_BASE + 0x34);
    let scale = float_getter(e, 0x0082_1640, water_form.addr());
    publish_height_scale(e, scale, SHADER_NOISE_BASE + 0x38);
    let scale = float_getter(e, 0x0082_1660, water_form.addr());
    publish_height_scale(e, scale, SHADER_NOISE_BASE + 0x3c);
    let amplitude = float_getter(e, 0x0056_7470, water_form.addr());
    e.set_global(SHADER_NOISE_BASE + 0x44, amplitude);
    let amplitude = float_getter(e, 0x0082_1680, water_form.addr());
    e.set_global(SHADER_NOISE_BASE + 0x48, amplitude);
    let amplitude = float_getter(e, 0x0081_33b0, water_form.addr());
    e.set_global(SHADER_NOISE_BASE + 0x4c, amplitude);

    let form_noise_map = pointer_in_slot(e, address_of(water_form, TESWaterForm::spNoiseNormalMap));
    let height_map = pointer_in_slot(
        e,
        address_of(property, WaterShaderProperty::spNoiseHeightMap),
    );
    let renderer = e.call(RENDERER, &[]).u32();
    let manager = fn_004e3270(e);
    e.call(
        IMAGE_SPACE_RENDER_NOISE,
        &args![
            manager,
            0x21u32,
            renderer,
            height_map,
            form_noise_map,
            0u32,
            1u32
        ],
    );
}

/// A height scale for the shader: the value divided by 100, rounded up
/// (`fn_004e4430`) and combined with 1.0 by `00404010`.
fn publish_height_scale(e: &mut Engine, value: f32, destination: u32) {
    let hundred: f64 = e.global(ONE_HUNDRED);
    let scaled = (value as f64 / hundred) as f32;
    let rounded = fn_004e4430(e, scaled);
    let combined = e.call(FLOAT_MAX, &args![1.0f32, rounded]).f32();
    e.set_global(destination, combined);
}

// Translated from 004e4430 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ceilf`: `fn_004e4450`.
pub fn fn_004e4430(e: &mut Engine, value: f32) -> f32 {
    fn_004e4450(e, value)
}

// Translated from 004e4450 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ceilf`: the CRT `ceil` (`00ec9e10`) on the value as a `double`, stored
/// as a `float`.
pub fn fn_004e4450(e: &mut Engine, value: f32) -> f32 {
    e.call(CRT_CEIL, &args![value as f64]).f64() as f32
}

// Translated from 004e4470 (decompiled, FalloutNV.exe 1.4.0.525)
/// `cosf`: `fn_004e4490`.
pub fn fn_004e4470(e: &mut Engine, value: f32) -> f32 {
    fn_004e4490(e, value)
}

// Translated from 004e4490 (decompiled, FalloutNV.exe 1.4.0.525)
/// `cosf`: the CRT `cos` (`00ec9f30`) on the value as a `double`, stored as
/// a `float`.
pub fn fn_004e4490(e: &mut Engine, value: f32) -> f32 {
    e.call(CRT_COS, &args![value as f64]).f64() as f32
}

// Translated from 004e44b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `sinf`: `fn_004e44d0`.
pub fn fn_004e44b0(e: &mut Engine, value: f32) -> f32 {
    fn_004e44d0(e, value)
}

// Translated from 004e44d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `sinf`: the CRT `sin` (`00eca060`) on the value as a `double`, stored as
/// a `float`.
pub fn fn_004e44d0(e: &mut Engine, value: f32) -> f32 {
    e.call(CRT_SIN, &args![value as f64]).f64() as f32
}

// Translated from 004e44f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSWrap` (Xbox PDB): wraps the float at `value` into `low..=high`. Does
/// nothing when `high < low` or the value is already inside; a value above
/// `high` becomes `fmod(value - low, high - low) + low`, one below `low`
/// becomes `fmod(value - low, high - low) + high` (the remainder is
/// `004b1520`, taking the offset and the range).
pub fn bs_wrap(e: &mut Engine, value: Ptr, low: f32, high: f32) {
    if high < low {
        return;
    }
    let current = e.mem.f32(value.addr());
    if high < current {
        let range = (high as f64 - low as f64) as f32;
        let offset = (current as f64 - low as f64) as f32;
        let remainder = e.call(FLOAT_REMAINDER, &args![offset, range]).f64();
        e.mem.set_f32(value.addr(), (remainder + low as f64) as f32);
    } else if current < low {
        let range = (high as f64 - low as f64) as f32;
        let offset = (current as f64 - low as f64) as f32;
        let remainder = e.call(FLOAT_REMAINDER, &args![offset, range]).f64();
        e.mem
            .set_f32(value.addr(), (remainder + high as f64) as f32);
    }
}

// Translated from 004e45a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Points the `NiPointer` slot `destination` at the system's shared noise
/// texture (`spWaterNoiseTexture`, +0x1c).
pub fn fn_004e45a0(e: &mut Engine, this: Ptr<TESWaterSystem>, destination: u32) {
    let source = address_of(this, TESWaterSystem::spWaterNoiseTexture);
    e.call(NI_POINTER_ASSIGN_FROM, &args![destination, source]);
}

// Translated from 004e45c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESWaterForm`'s `Data.fHeightUVScale0` (the float at +0x130).
pub fn fn_004e45c0(e: &mut Engine, form: Ptr<TESWaterForm>) -> f32 {
    e.get(form, TESWaterForm::fHeightUVScale0)
}

// Translated from 004e45e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESWaterForm`'s `Data.fNoiseWindDirection0` (the float at +0xe8).
pub fn fn_004e45e0(e: &mut Engine, form: Ptr<TESWaterForm>) -> f32 {
    e.get(form, TESWaterForm::fNoiseWindDirection0)
}

// Translated from 004e4600 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESWaterForm`'s `Data.fNoiseWindDirection2` (the float at +0xf0).
pub fn fn_004e4600(e: &mut Engine, form: Ptr<TESWaterForm>) -> f32 {
    e.get(form, TESWaterForm::fNoiseWindDirection2)
}

// Translated from 004e4620 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESWaterForm`'s `Data.fNoiseWindSpeed1` (the float at +0xf8).
pub fn fn_004e4620(e: &mut Engine, form: Ptr<TESWaterForm>) -> f32 {
    e.get(form, TESWaterForm::fNoiseWindSpeed1)
}

// Translated from 004e4640 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESWaterForm::bResetNoiseTexture` (the byte at +0x190).
pub fn fn_004e4640(e: &mut Engine, form: Ptr<TESWaterForm>) -> u8 {
    e.get(form, TESWaterForm::bResetNoiseTexture)
}

// Translated from 004e4660 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets `TESWaterForm::bResetNoiseTexture` (the byte at +0x190).
pub fn fn_004e4660(e: &mut Engine, form: Ptr<TESWaterForm>, value: u8) {
    e.set(form, TESWaterForm::bResetNoiseTexture, value);
}

// Translated from 004e4680 (decompiled, FalloutNV.exe 1.4.0.525)
/// The pointer global at `0x011ffe44` (the object whose byte at +0x2b8
/// `fn_004e4690` tests).
pub fn fn_004e4680(e: &mut Engine) -> u32 {
    e.global(FLUSH_OBJECT)
}

// Translated from 004e4690 (decompiled, FalloutNV.exe 1.4.0.525)
/// The byte at +0x2b8 of `object`.
pub fn fn_004e4690(e: &mut Engine, object: u32) -> u8 {
    e.mem.u8(object + 0x2b8)
}

// Translated from 004e46b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESWaterSystem::AddPlaceableWater` (Xbox PDB): reads the reference's 3D
/// node's world position and rotation matrix and passes them (copied to
/// locals) to `AddPlaceableWater_ov2` (`004e4730`) with the reference.
pub fn tes_water_system_add_placeable_water(
    e: &mut Engine,
    this: Ptr<TESWaterSystem>,
    reference: u32,
) {
    let node = e.vcall(reference, REFERENCE_GET_3D, &[]).u32();
    let translate = e.call(NODE_WORLD_TRANSLATE, &args![node]).u32();
    let position: Vec<u32> = (0..3).map(|i| e.mem.u32(translate + 4 * i)).collect();
    let node = e.vcall(reference, REFERENCE_GET_3D, &[]).u32();
    let rotate = e.call(NODE_WORLD_ROTATE, &args![node]).u32();
    let matrix: Vec<u32> = (0..9).map(|i| e.mem.u32(rotate + 4 * i)).collect();
    e.with_stack(0x30, |e, locals| {
        for (i, word) in position.iter().chain(matrix.iter()).enumerate() {
            e.mem.set_u32(locals.addr() + 4 * i as u32, *word);
        }
        let matrix_copy = locals.addr() + 0x0c;
        e.call(
            ADD_PLACEABLE_WATER_OV2,
            &args![this, reference, locals, matrix_copy],
        );
    });
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(
            0x004e21b0,
            tes_water_system_update_placeable_water(Ptr<TESWaterSystem>, Ptr, bool)
        ),
        entry!(0x004e3260, fn_004e3260() -> u32),
        entry!(0x004e3270, fn_004e3270() -> u32),
        entry!(0x004e3280, fn_004e3280(u32) -> bool),
        entry!(0x004e32a0, fn_004e32a0(u32) -> bool),
        entry!(0x004e32c0, fn_004e32c0(u32) -> bool),
        entry!(0x004e32e0, fn_004e32e0(u32) -> bool),
        entry!(0x004e3300, fn_004e3300(u32) -> bool),
        entry!(
            0x004e3320,
            tes_water_system_update_lod_water(Ptr<TESWaterSystem>, Ptr, bool)
        ),
        entry!(
            0x004e3520,
            tes_water_system_update_water_shader_properties(
                Ptr<TESWaterSystem>,
                Ptr<PlaceableWaterGroup>,
            )
        ),
        entry!(
            0x004e3590,
            tes_water_system_update_water_shader_properties_ov2(
                Ptr<TESWaterSystem>,
                Ptr<PlaceableWaterGroup>,
                u32,
            )
        ),
        entry!(0x004e3c00, fn_004e3c00(Ptr<WaterShaderProperty>)),
        entry!(0x004e3c20, fn_004e3c20(Ptr)),
        entry!(0x004e3c40, fn_004e3c40() -> u8),
        entry!(0x004e3c50, fn_004e3c50(u32) -> bool),
        entry!(0x004e3c70, fn_004e3c70(u32) -> bool),
        entry!(0x004e3ca0, fn_004e3ca0(u32) -> bool),
        entry!(0x004e3cc0, fn_004e3cc0(Ptr<TESWaterForm>) -> f32),
        entry!(0x004e3ce0, fn_004e3ce0(Ptr<TESWaterForm>) -> f32),
        entry!(0x004e3d00, fn_004e3d00(Ptr<TESWaterForm>) -> f32),
        entry!(0x004e3d20, fn_004e3d20(Ptr<TESWaterForm>) -> f32),
        entry!(0x004e3d40, fn_004e3d40(Ptr<TESWaterForm>) -> f32),
        entry!(
            0x004e3d60,
            fn_004e3d60(Ptr<TESWaterSystem>, Ptr<PlaceableWaterGroup>)
        ),
        entry!(0x004e4430, fn_004e4430(f32) -> f32),
        entry!(0x004e4450, fn_004e4450(f32) -> f32),
        entry!(0x004e4470, fn_004e4470(f32) -> f32),
        entry!(0x004e4490, fn_004e4490(f32) -> f32),
        entry!(0x004e44b0, fn_004e44b0(f32) -> f32),
        entry!(0x004e44d0, fn_004e44d0(f32) -> f32),
        entry!(0x004e44f0, bs_wrap(Ptr, f32, f32)),
        entry!(0x004e45a0, fn_004e45a0(Ptr<TESWaterSystem>, u32)),
        entry!(0x004e45c0, fn_004e45c0(Ptr<TESWaterForm>) -> f32),
        entry!(0x004e45e0, fn_004e45e0(Ptr<TESWaterForm>) -> f32),
        entry!(0x004e4600, fn_004e4600(Ptr<TESWaterForm>) -> f32),
        entry!(0x004e4620, fn_004e4620(Ptr<TESWaterForm>) -> f32),
        entry!(0x004e4640, fn_004e4640(Ptr<TESWaterForm>) -> u8),
        entry!(0x004e4660, fn_004e4660(Ptr<TESWaterForm>, u8)),
        entry!(0x004e4680, fn_004e4680() -> u32),
        entry!(0x004e4690, fn_004e4690(u32) -> u8),
        entry!(
            0x004e46b0,
            tes_water_system_add_placeable_water(Ptr<TESWaterSystem>, u32)
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ret(value: u32) -> Ret {
        Ret {
            eax: value,
            ..Ret::default()
        }
    }

    /// Vtables of the test objects (each is mapped at its own address).
    const REFERENCE_VTABLE: u32 = 0x0200_0000;
    const BASE_FORM_VTABLE: u32 = 0x0200_1000;
    const NODE_VTABLE: u32 = 0x0200_2000;
    const WATER_FORM_VTABLE: u32 = 0x0200_3000;
    const TEXTURE_VTABLE: u32 = 0x0200_4000;
    const RENDERER_VTABLE: u32 = 0x0200_5000;
    const PROPERTY_VTABLE: u32 = 0x0200_6000;
    /// Addresses of the doubles the vtables point at.
    const REFERENCE_GET_3D_DOUBLE: u32 = 0x0300_0001;
    const BASE_FORM_WATER_FORM_DOUBLE: u32 = 0x0300_0002;
    const NODE_VIRTUAL_DOUBLE: u32 = 0x0300_0003;
    const WATER_FORM_NAME_DOUBLE: u32 = 0x0300_0004;
    const TEXTURE_WIDTH_DOUBLE: u32 = 0x0300_0005;
    const TEXTURE_HEIGHT_DOUBLE: u32 = 0x0300_0006;
    const RENDERER_GET_COLOR_DOUBLE: u32 = 0x0300_0007;
    const RENDERER_SET_COLOR_DOUBLE: u32 = 0x0300_0008;
    const PROPERTY_PASS_COUNT_DOUBLE: u32 = 0x0300_0009;

    // Field offsets of the test objects (the doubles read them).
    /// Reference: the 3D node `Get3D` returns, the water geometry
    /// `004e8030` returns, and the answer of the range test.
    const REFERENCE_NODE: u32 = 0x90;
    const REFERENCE_WATER_NODE: u32 = 0x94;
    const REFERENCE_IN_RANGE: u32 = 0x98;
    /// Base form: the flags `FORM_FLAG_TEST` reads, the water form.
    const BASE_FORM_FLAGS: u32 = 0x10;
    const BASE_FORM_WATER_FORM: u32 = 0x14;
    /// Water geometry node: the property `GetProperty` returns, and the
    /// answer of its virtual at +0x18.
    const NODE_PROPERTY: u32 = 0xa0;
    const NODE_VIRTUAL_RESULT: u32 = 0xb0;
    /// Property: the answer of its virtual at +0xa4.
    const PROPERTY_PASS_COUNT: u32 = 0x1f0;

    /// An object of `size` bytes with `vtable` at +0.
    fn object(e: &mut Engine, vtable: u32, size: u32) -> u32 {
        let object = e.mem.alloc(size);
        e.mem.set_u32(object, vtable);
        object
    }

    /// A vtable at `address` with the given (byte offset, target) slots.
    fn vtable(e: &mut Engine, address: u32, slots: &[(u32, u32)]) {
        let length = slots
            .iter()
            .map(|(offset, _)| offset / 4 + 1)
            .max()
            .unwrap_or(1);
        let mut words = vec![0u32; length as usize];
        for (offset, target) in slots {
            words[(offset / 4) as usize] = *target;
        }
        e.put_vtable(address, &words);
    }

    /// An `NiTPointerList`: head, tail, count, and nodes `{next, prev, item}`.
    fn list(e: &mut Engine, items: &[u32]) -> u32 {
        let header = e.mem.alloc(0x0c);
        let mut previous = 0;
        for item in items {
            let node = e.mem.alloc(12);
            e.mem.set_u32(node + 4, previous);
            e.mem.set_u32(node + 8, *item);
            if previous == 0 {
                e.mem.set_u32(header, node);
            } else {
                e.mem.set_u32(previous, node);
            }
            e.mem.set_u32(header + 4, node);
            previous = node;
        }
        e.mem.set_u32(header + 8, items.len() as u32);
        header
    }

    /// Fills the list embedded at `address` (a `NiTPointerList` field).
    fn fill_list(e: &mut Engine, address: u32, items: &[u32]) {
        let built = list(e, items);
        for i in 0..3 {
            let word = e.mem.u32(built + 4 * i);
            e.mem.set_u32(address + 4 * i, word);
        }
    }

    /// Test doubles for the callees every function here shares: the
    /// `NiPointer`/list helpers, the INI settings (the byte at setting + 4),
    /// the `NiPointer` assignments, the form flag tests and the object
    /// virtuals, plus the pages holding the exe globals.
    fn water_engine() -> Engine {
        let mut e = Engine::new();
        for page in [
            0x0101_2000,
            0x0101_7000,
            0x0101_e000,
            0x0102_3000,
            0x011a_9000,
            0x011a_d000,
            0x011c_7000,
            0x011d_e000,
            0x011f_9000,
            0x011f_f000,
        ] {
            e.map(page, 0x1000);
        }
        e.set_global(MINUS_ONE, -1.0f32);
        e.set_global(BYTE_RANGE, 255.0f64);
        e.set_global(ONE_HUNDRED, 100.0f64);
        e.set_global(DEGREES_TO_RADIANS, 0.017453292f32 as f64);

        // `*this` (NiPointer / list head), list next / item, emptiness, count.
        e.register(NI_POINTER_GET, |e, a| ret(e.mem.u32(a[0])));
        e.register(LIST_NEXT_POSITION, |e, a| ret(e.mem.u32(a[1])));
        e.register(LIST_ITEM_SLOT, |_, a| ret(a[1] + 8));
        e.register(LIST_IS_EMPTY, |e, a| ret((e.mem.u32(a[0] + 8) == 0) as u32));
        e.register(LIST_COUNT, |e, a| ret(e.mem.u32(a[0] + 8)));
        // Settings: the value byte at setting + 4.
        e.register(SETTING_VALUE, |_, a| ret(a[0] + 4));
        // NiPointer assignments.
        e.register(NI_POINTER_ASSIGN, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            ret(a[0])
        });
        e.register(NI_POINTER_ASSIGN_FROM, |e, a| {
            let value = e.mem.u32(a[1]);
            e.mem.set_u32(a[0], value);
            ret(a[0])
        });
        // The TES object: `pInteriorCell` at +0x34.
        let tes = e.mem.alloc(0xc4);
        e.set_global(TES_POINTER, tes);
        e.register(TES_GET_INTERIOR_CELL, |e, a| ret(e.mem.u32(a[0] + 0x34)));
        // The reference: base form at +0x20, parent cell at +0x40; the cell's
        // byte at +0x26; the 3D node's culled flag at +0x30.
        e.register(REFERENCE_BASE_FORM, |e, a| ret(e.mem.u32(a[0] + 0x20)));
        e.register(REFERENCE_PARENT_CELL, |e, a| ret(e.mem.u32(a[0] + 0x40)));
        e.register(CELL_BYTE_IS_SIX, |e, a| {
            ret((e.mem.u8(a[0] + 0x26) == 6) as u32)
        });
        e.register(NODE_IS_CULLED, |e, a| ret(e.mem.u32(a[0] + 0x30) & 1));
        e.register(REFERENCE_EXTRA_DATA_LIST, |_, a| ret(a[0] + 0x44));
        e.register(EXTRA_DATA_LIST_GET_REFLECTED_REFS, |e, a| {
            ret(e.mem.u32(a[0]))
        });
        e.register(WORDS_ARE_ZERO, |e, a| {
            ret((e.mem.u32(a[0]) == 0 && e.mem.u32(a[0] + 4) == 0) as u32)
        });
        // Form flags: the word at base form + 0x10.
        e.register(FORM_FLAG_TEST, |e, a| {
            ret((e.mem.u32(a[0] + BASE_FORM_FLAGS) & a[1] != 0) as u32)
        });
        e.register(FORM_FLAG_TEST_40000000, |e, a| {
            ret((e.mem.u32(a[0] + BASE_FORM_FLAGS) & 0x4000_0000 != 0) as u32)
        });
        // The water reference's 3D object and its owner / property.
        e.register(WATER_REFERENCE_3D, |e, a| {
            ret(e.mem.u32(a[1] + REFERENCE_WATER_NODE))
        });
        e.register(NODE_OWNER, |e, a| ret(e.mem.u32(a[0] + 0xc0)));
        e.register(OWNER_TYPE, |e, a| ret(e.mem.u32(a[0] + 0x68)));
        e.register(NODE_GET_PROPERTY, |e, a| {
            assert_eq!(a[1], 3, "the water property is type 3");
            ret(e.mem.u32(a[0] + NODE_PROPERTY))
        });
        e.register(NODE_WORLD_TRANSLATE, |_, a| ret(a[0] + 0x8c));
        e.register(NODE_WORLD_ROTATE, |_, a| ret(a[0] + 0x68));
        e.register(REFERENCE_IS_IN_RANGE, |e, a| {
            ret(e.mem.u8(a[1] + REFERENCE_IN_RANGE) as u32)
        });
        e.register(WATER_SKIPPED_FLAG, |e, _| ret(e.mem.u8(0x011c_7a59) as u32));
        e.register(NI_POINT4_CONSTRUCT, |e, a| {
            for i in 0..4 {
                e.mem.set_u32(a[0] + 4 * i, a[1 + i as usize]);
            }
            ret(a[0])
        });
        // The vtable slots.
        e.register(REFERENCE_GET_3D_DOUBLE, |e, a| {
            ret(e.mem.u32(a[0] + REFERENCE_NODE))
        });
        e.register(BASE_FORM_WATER_FORM_DOUBLE, |e, a| {
            ret(e.mem.u32(a[0] + BASE_FORM_WATER_FORM))
        });
        e.register(NODE_VIRTUAL_DOUBLE, |e, a| {
            ret(e.mem.u32(a[0] + NODE_VIRTUAL_RESULT))
        });
        e.register(PROPERTY_PASS_COUNT_DOUBLE, |e, a| {
            ret(e.mem.u32(a[0] + PROPERTY_PASS_COUNT))
        });
        e.register(WATER_FORM_NAME_DOUBLE, |_, _| ret(0x0555_0000));
        e.register(TEXTURE_WIDTH_DOUBLE, |_, _| ret(512));
        e.register(TEXTURE_HEIGHT_DOUBLE, |_, _| ret(256));
        e.register(RENDERER_GET_COLOR_DOUBLE, |e, a| {
            for (i, value) in [0.1f32, 0.2, 0.3, 0.4].into_iter().enumerate() {
                e.mem.set_f32(a[1] + 4 * i as u32, value);
            }
            Ret::default()
        });
        e.register(RENDERER_SET_COLOR_DOUBLE, |_, _| Ret::default());
        vtable(
            &mut e,
            REFERENCE_VTABLE,
            &[(REFERENCE_GET_3D, REFERENCE_GET_3D_DOUBLE)],
        );
        vtable(
            &mut e,
            BASE_FORM_VTABLE,
            &[(BASE_FORM_GET_WATER_FORM, BASE_FORM_WATER_FORM_DOUBLE)],
        );
        vtable(
            &mut e,
            NODE_VTABLE,
            &[(NODE_PROPERTIES_VIRTUAL, NODE_VIRTUAL_DOUBLE)],
        );
        vtable(
            &mut e,
            PROPERTY_VTABLE,
            &[(PROPERTY_PASS_COUNT_VIRTUAL, PROPERTY_PASS_COUNT_DOUBLE)],
        );
        vtable(
            &mut e,
            WATER_FORM_VTABLE,
            &[(WATER_FORM_GET_NAME, WATER_FORM_NAME_DOUBLE)],
        );
        vtable(
            &mut e,
            TEXTURE_VTABLE,
            &[
                (TEXTURE_WIDTH_VIRTUAL, TEXTURE_WIDTH_DOUBLE),
                (TEXTURE_HEIGHT_VIRTUAL, TEXTURE_HEIGHT_DOUBLE),
            ],
        );
        vtable(
            &mut e,
            RENDERER_VTABLE,
            &[
                (RENDERER_GET_CLEAR_COLOR, RENDERER_GET_COLOR_DOUBLE),
                (RENDERER_SET_CLEAR_COLOR, RENDERER_SET_COLOR_DOUBLE),
            ],
        );
        e
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

    fn start_log(e: &mut Engine) {
        e.call_log = Some(vec![]);
    }

    /// Makes `setting` read as `value`.
    fn set_setting(e: &mut Engine, setting: u32, value: bool) {
        e.set_global(setting + 4, value as u8);
    }

    // --- the small accessors ------------------------------------------------

    #[test]
    fn address_returning_accessors() {
        let mut e = water_engine();
        assert_eq!(e.call(0x004e_3260, &[]).u32(), 0x011c_a13c);
        e.set_global(IMAGE_SPACE_MANAGER, 0x1234_5678u32);
        assert_eq!(e.call(0x004e_3270, &[]).u32(), 0x1234_5678);
        e.set_global(WATER_FLAG_011C7A58, 7u8);
        assert_eq!(e.call(0x004e_3c40, &[]).u8(), 7);
        e.set_global(FLUSH_OBJECT, 0x0abc_0000u32);
        assert_eq!(e.call(0x004e_4680, &[]).u32(), 0x0abc_0000);
    }

    /// A form flag test: the function must pass `mask` to `00452460`, and
    /// return its result (or the negation, for `fn_004e3c70`).
    fn check_flag_function(address: u32, mask: u32, negated: bool) {
        let mut e = water_engine();
        let form = e.mem.alloc(0x40);
        e.mem.set_u32(form + BASE_FORM_FLAGS, mask);
        start_log(&mut e);
        let with_bit = e.call(address, &args![form]).bool();
        assert_eq!(with_bit, !negated, "{address:08x} with the bit set");
        assert_eq!(calls(&e, FORM_FLAG_TEST), vec![vec![form, mask]]);
        e.mem.set_u32(form + BASE_FORM_FLAGS, !mask);
        let without_bit = e.call(address, &args![form]).bool();
        assert_eq!(without_bit, negated, "{address:08x} without the bit");
    }

    #[test]
    fn form_flag_test_200() {
        check_flag_function(0x004e_3280, 0x200, false);
    }

    #[test]
    fn form_flag_test_800() {
        check_flag_function(0x004e_32a0, 0x800, false);
    }

    #[test]
    fn form_flag_test_8000000() {
        check_flag_function(0x004e_32c0, 0x0800_0000, false);
    }

    #[test]
    fn form_flag_test_10000000() {
        check_flag_function(0x004e_32e0, 0x1000_0000, false);
    }

    #[test]
    fn form_flag_test_40000() {
        check_flag_function(0x004e_3300, 0x4_0000, false);
    }

    #[test]
    fn form_flag_test_1() {
        check_flag_function(0x004e_3c50, 1, false);
    }

    #[test]
    fn form_flag_test_80000000_is_negated() {
        check_flag_function(0x004e_3c70, 0x8000_0000, true);
    }

    #[test]
    fn form_flag_test_20000000() {
        check_flag_function(0x004e_3ca0, 0x2000_0000, false);
    }

    #[test]
    fn render_pass_list_field_is_cleared() {
        let mut e = water_engine();
        let list = e.mem.alloc(0x20);
        e.mem.set_u32(list + 0x10, 99);
        e.call(0x004e_3c20, &args![list]);
        assert_eq!(e.mem.u32(list + 0x10), 0);
    }

    #[test]
    fn render_pass_list_reset_only_when_the_property_has_one() {
        let mut e = water_engine();
        let property = e.mem.alloc(0x150);
        let list = e.mem.alloc(0x20);
        e.mem.set_u32(list + 0x10, 99);
        e.call(0x004e_3c00, &args![property]);
        assert_eq!(e.mem.u32(list + 0x10), 99);
        e.mem.set_u32(property + 0x3c, list);
        e.call(0x004e_3c00, &args![property]);
        assert_eq!(e.mem.u32(list + 0x10), 0);
    }

    /// A `TESWaterForm` float getter reads its offset.
    fn check_form_getter(address: u32, offset: u32) {
        let mut e = water_engine();
        let form = e.mem.alloc(0x1a4);
        e.mem.set_f32(form + offset, 2.5);
        assert_eq!(e.call(address, &args![form]).f32(), 2.5, "{address:08x}");
    }

    #[test]
    fn water_form_fresnel_amount_getter() {
        check_form_getter(0x004e_3cc0, 0x9c);
    }

    #[test]
    fn water_form_fog_amount_getter() {
        check_form_getter(0x004e_3ce0, 0x108);
    }

    #[test]
    fn water_form_underwater_fog_near_getter() {
        check_form_getter(0x004e_3d00, 0x114);
    }

    #[test]
    fn water_form_light_radius_getter() {
        check_form_getter(0x004e_3d20, 0x128);
    }

    #[test]
    fn water_form_light_brightness_getter() {
        check_form_getter(0x004e_3d40, 0x12c);
    }

    #[test]
    fn water_form_height_uv_scale_getter() {
        check_form_getter(0x004e_45c0, 0x130);
    }

    #[test]
    fn water_form_wind_direction_0_getter() {
        check_form_getter(0x004e_45e0, 0xe8);
    }

    #[test]
    fn water_form_wind_direction_2_getter() {
        check_form_getter(0x004e_4600, 0xf0);
    }

    #[test]
    fn water_form_wind_speed_1_getter() {
        check_form_getter(0x004e_4620, 0xf8);
    }

    #[test]
    fn noise_texture_reset_flag_round_trips_its_byte() {
        let mut e = water_engine();
        let form = e.mem.alloc(0x1a4);
        assert_eq!(e.call(0x004e_4640, &args![form]).u8(), 0);
        e.call(0x004e_4660, &args![form, 1u8]);
        assert_eq!(e.mem.u8(form + 0x190), 1);
        assert_eq!(e.call(0x004e_4640, &args![form]).u8(), 1);
        e.call(0x004e_4660, &args![form, 0u8]);
        assert_eq!(e.mem.u8(form + 0x190), 0);
    }

    #[test]
    fn flush_object_flag_is_the_byte_at_2b8() {
        let mut e = water_engine();
        let object = e.mem.alloc(0x2c0);
        e.mem.set_u8(object + 0x2b8, 9);
        assert_eq!(e.call(0x004e_4690, &args![object]).u8(), 9);
    }

    #[test]
    fn shared_noise_texture_is_copied_into_the_slot() {
        let mut e = water_engine();
        let system = e.mem.alloc(0xa0);
        e.mem.set_u32(system + 0x1c, 0x5555);
        let slot = e.mem.alloc(4);
        start_log(&mut e);
        e.call(0x004e_45a0, &args![system, slot]);
        assert_eq!(
            calls(&e, NI_POINTER_ASSIGN_FROM),
            vec![vec![slot, system + 0x1c]]
        );
        assert_eq!(e.mem.u32(slot), 0x5555);
    }

    // --- the CRT wrappers ---------------------------------------------------

    #[test]
    fn ceil_wrappers_round_up_through_a_double() {
        let mut e = water_engine();
        e.register(CRT_CEIL, |_, a| f64::take(a, &mut 0).ceil().into_ret());
        assert_eq!(e.call(0x004e_4450, &args![2.25f32]).f32(), 3.0);
        assert_eq!(e.call(0x004e_4430, &args![-0.5f32]).f32(), 0.0);
        assert_eq!(e.call(0x004e_4430, &args![7.0f32]).f32(), 7.0);
    }

    #[test]
    fn cos_wrappers_pass_a_double_and_return_a_float() {
        let mut e = water_engine();
        e.register(CRT_COS, |_, a| f64::take(a, &mut 0).cos().into_ret());
        start_log(&mut e);
        let direct = e.call(0x004e_4490, &args![1.5f32]).f32();
        let wrapped = e.call(0x004e_4470, &args![1.5f32]).f32();
        assert_eq!(direct, (1.5f64.cos()) as f32);
        assert_eq!(wrapped, direct);
        let logged = calls(&e, CRT_COS);
        assert_eq!(logged.len(), 2);
        assert_eq!(f64::take(&logged[0], &mut 0), 1.5);
    }

    #[test]
    fn sin_wrappers_pass_a_double_and_return_a_float() {
        let mut e = water_engine();
        e.register(CRT_SIN, |_, a| f64::take(a, &mut 0).sin().into_ret());
        start_log(&mut e);
        let direct = e.call(0x004e_44d0, &args![0.25f32]).f32();
        let wrapped = e.call(0x004e_44b0, &args![0.25f32]).f32();
        assert_eq!(direct, (0.25f64.sin()) as f32);
        assert_eq!(wrapped, direct);
        assert_eq!(calls(&e, CRT_SIN).len(), 2);
    }

    // --- BSWrap -------------------------------------------------------------

    /// `BSWrap(&value, low, high)` with `004b1520` standing in as `fmod`.
    fn wrap(value: f32, low: f32, high: f32) -> f32 {
        let mut e = water_engine();
        e.register(FLOAT_REMAINDER, |_, a| {
            ((f32::from_bits(a[0]) as f64) % (f32::from_bits(a[1]) as f64)).into_ret()
        });
        let slot = e.mem.alloc(4);
        e.mem.set_f32(slot, value);
        e.call(0x004e_44f0, &args![slot, low, high]);
        e.mem.f32(slot)
    }

    #[test]
    fn wrap_leaves_values_inside_the_range() {
        assert_eq!(wrap(0.25, 0.0, 1.0), 0.25);
        assert_eq!(wrap(1.0, 0.0, 1.0), 1.0);
        assert_eq!(wrap(0.0, 0.0, 1.0), 0.0);
    }

    #[test]
    fn wrap_brings_values_above_the_range_down() {
        assert_eq!(wrap(1.5, 0.0, 1.0), 0.5);
        assert_eq!(wrap(7.25, 2.0, 4.0), 3.25);
    }

    #[test]
    fn wrap_brings_values_below_the_range_up_from_the_top() {
        // fmod(-0.25, 1) = -0.25, plus the high end 1.0.
        assert_eq!(wrap(-0.25, 0.0, 1.0), 0.75);
    }

    #[test]
    fn wrap_does_nothing_when_the_range_is_reversed() {
        assert_eq!(wrap(5.0, 2.0, 1.0), 5.0);
    }

    // --- AddPlaceableWater --------------------------------------------------

    #[test]
    fn add_placeable_water_passes_the_node_position_and_rotation() {
        let mut e = water_engine();
        let system = e.mem.alloc(0xa0);
        let node = e.mem.alloc(0x100);
        let reference = object(&mut e, REFERENCE_VTABLE, 0x100);
        e.mem.set_u32(reference + REFERENCE_NODE, node);
        for i in 0..3 {
            e.mem.set_f32(node + 0x8c + 4 * i, 10.0 + i as f32);
        }
        for i in 0..9 {
            e.mem.set_f32(node + 0x68 + 4 * i, i as f32);
        }
        e.register_double(ADD_PLACEABLE_WATER_OV2, |e, a| {
            // The position and rotation are passed by address, in locals.
            assert_eq!(a[3], a[2] + 0x0c);
            let mut words = Vec::new();
            for i in 0..12 {
                words.push(e.mem.f32(a[2] + 4 * i));
            }
            e.mem.set_f32(a[1] + 0x40, words[0]);
            e.mem.set_f32(a[1] + 0x44, words[2]);
            e.mem.set_f32(a[1] + 0x48, words[3]);
            e.mem.set_f32(a[1] + 0x4c, words[11]);
            Ret::default()
        });
        start_log(&mut e);
        e.call(0x004e_46b0, &args![system, reference]);
        let ov2 = calls(&e, ADD_PLACEABLE_WATER_OV2);
        assert_eq!(ov2.len(), 1);
        assert_eq!(&ov2[0][..2], &[system, reference]);
        assert_eq!(e.mem.f32(reference + 0x40), 10.0);
        assert_eq!(e.mem.f32(reference + 0x44), 12.0);
        assert_eq!(e.mem.f32(reference + 0x48), 0.0);
        assert_eq!(e.mem.f32(reference + 0x4c), 8.0);
    }

    // --- a scene for the group updates --------------------------------------

    /// Pass functions `UpdatePlaceableWater` schedules and the renderer calls
    /// of the depth pass: doubles that do nothing (the tests read the log).
    const QUIET_CALLS: [u32; 26] = [
        SETUP_WORLD_REFLECTIONS,
        SETUP_SKY_REFLECTIONS,
        FINISH_WORLD_REFLECTIONS,
        FINISH_SKY_REFLECTIONS,
        FINISH_GROUP,
        FINISH_GROUP_INTERIOR,
        RELEASE_GROUP,
        SETUP_GROUP_REFLECTIONS,
        SETUP_GROUP_DEPTH,
        MAP_REMOVE_ALL,
        LIST_REMOVE_ALL,
        RENDER_GROUP_DEPTH,
        AFTER_GROUP_DEPTH,
        VIEWER_POSITION_UPDATE,
        RENDER_TARGET_SET,
        RENDER_TARGET_RESET,
        BATCH_RENDERER_END_PASS,
        SET_STENCIL_STATE,
        SHADER_PARAM_CONSTRUCT,
        SHADER_PARAM_INIT_CONSTANT_MAP,
        SHADER_PARAM_SET_PIXEL_CONSTANT,
        SHADER_PARAM_DESTRUCT,
        IMAGE_SPACE_RENDER_EFFECT,
        RETURN_RENDERED_TEXTURE,
        LOG_MESSAGE,
        FORMATTED_PRINT,
    ];

    /// The depth map and the wading-water height map the tests start with.
    const DEPTH_TEXTURE: u32 = 0x0d00_0001;
    const WADING_TEXTURE: u32 = 0x0d00_0002;
    const FORM_NOISE_TEXTURE: u32 = 0x0d00_0003;

    struct Scene {
        e: Engine,
        this: Ptr<TESWaterSystem>,
        group: Ptr<PlaceableWaterGroup>,
        reference: u32,
        base_form: u32,
        water_node: u32,
        property: Ptr<WaterShaderProperty>,
        viewer: u32,
        water_form: u32,
        renderer: u32,
        texture_manager: u32,
    }

    /// An exterior scene with one group of one reference that is in range,
    /// whose base form allows reflections, refractions and depth, with the
    /// settings for reflections, depth and refractions on.
    fn scene() -> Scene {
        let mut e = water_engine();
        for address in QUIET_CALLS {
            e.register(address, |_, _| Ret::default());
        }
        let renderer = object(&mut e, RENDERER_VTABLE, 0x40);
        e.set_global(0x011f_9508u32, renderer);
        let texture_manager = e.mem.alloc(0x40);
        e.set_global(0x011f_91a8u32, texture_manager);
        e.register(RENDERER, |e, _| ret(e.mem.u32(0x011f_9508)));
        e.register(TEXTURE_MANAGER, |e, _| ret(e.mem.u32(0x011f_91a8)));
        e.register(RENDERED_TEXTURE_STOP, |_, _| ret(0x7777));
        e.register(PLANE_CONSTANT, |e, a| e.mem.f32(a[0] + 0xc).into_ret());
        e.set_global(IMAGE_SPACE_MANAGER, 0x0ee0_0000u32);
        e.set_global(DEPTH_MAP, DEPTH_TEXTURE);
        e.set_global(WADING_WATER_HEIGHT_MAP, WADING_TEXTURE);
        for (i, value) in [0.5f32, 0.25, 0.125, 1.0].into_iter().enumerate() {
            e.set_global(DEPTH_CLEAR_COLOR + 4 * i as u32, value);
        }
        for setting in [
            SETTING_USE_WATER_REFLECTIONS,
            SETTING_USE_WATER_DEPTH,
            SETTING_USE_WATER_REFRACTIONS,
        ] {
            set_setting(&mut e, setting, true);
        }

        let this = e.mem.alloc(0xa0);
        let water_form = object(&mut e, WATER_FORM_VTABLE, 0x1a4);
        e.mem.set_u32(water_form + 0x30, FORM_NOISE_TEXTURE);
        let group = e.mem.alloc(0xb0);
        e.mem.set_u32(group, water_form);
        e.mem.set_f32(group + 0x04 + 0x0c, 5.5);
        e.mem.set_u32(group + 0xa8, 0x0ca0_0001);
        e.mem.set_u32(group + 0xac, 0x0007_0003);
        e.mem.set_u8(group + 0x5c, 1);

        let base_form = object(&mut e, BASE_FORM_VTABLE, 0x40);
        e.mem.set_u32(
            base_form + BASE_FORM_FLAGS,
            0x0800_0000 | 0x1000_0000 | 0x200,
        );
        let node = e.mem.alloc(0x100);
        let owner = e.mem.alloc(0x80);
        e.mem.set_u32(owner + 0x68, WATER_OWNER_TYPE);
        let property = object(&mut e, PROPERTY_VTABLE, 0x200);
        let water_node = object(&mut e, NODE_VTABLE, 0x200);
        e.mem.set_u32(water_node + 0xc0, owner);
        e.mem.set_u32(water_node + NODE_PROPERTY, property);
        let reference = object(&mut e, REFERENCE_VTABLE, 0x100);
        e.mem.set_u32(reference + 0x20, base_form);
        e.mem.set_u32(reference + REFERENCE_NODE, node);
        e.mem.set_u32(reference + REFERENCE_WATER_NODE, water_node);
        e.mem.set_u8(reference + REFERENCE_IN_RANGE, 1);
        // The property has all its texture slots filled.
        for offset in [0x13c, 0x140, 0x144, 0x138] {
            e.mem.set_u32(property + offset, 0x0b00_0000 + offset);
        }

        e.map(0x011c_a000, 0x1000);
        let viewer = e.mem.alloc(0x100);
        for (i, value) in [1.0f32, 2.0, 3.0].into_iter().enumerate() {
            e.mem.set_f32(viewer + 0x8c + 4 * i as u32, value);
        }
        fill_list(&mut e, this + 0x3c, &[group]);
        fill_list(&mut e, group + 0x24, &[reference]);
        Scene {
            e,
            this: Ptr::new(this),
            group: Ptr::new(group),
            reference,
            base_form,
            water_node,
            property: Ptr::new(property),
            viewer,
            water_form,
            renderer,
            texture_manager,
        }
    }

    impl Scene {
        fn update(&mut self, force_update: bool) {
            start_log(&mut self.e);
            let (this, viewer) = (self.this, self.viewer);
            self.e.call(0x004e_21b0, &args![this, viewer, force_update]);
        }

        fn group_flag(&self, offset: u32) -> bool {
            self.e.mem.u8(self.group.addr() + offset) != 0
        }

        fn calls(&self, address: u32) -> Vec<Vec<u32>> {
            calls(&self.e, address)
        }
    }

    // --- UpdatePlaceableWater -----------------------------------------------

    #[test]
    fn a_reference_in_range_switches_its_group_on_and_sets_up_world_reflections() {
        let mut s = scene();
        s.update(false);
        let (this, group, viewer) = (s.this.addr(), s.group.addr(), s.viewer);
        assert!(s.group_flag(0x5d), "bRenderGroup");
        assert_eq!(s.e.global::<u32>(ACTIVE_WATER_GROUPS), 1);
        // The group sits at the world-space water height: world reflections.
        assert_eq!(s.e.global::<u8>(RENDER_WORLD_REFLECTIONS), 1);
        assert_eq!(s.e.global::<u8>(RENDER_SKY_REFLECTIONS), 0);
        assert_eq!(s.calls(SETUP_WORLD_REFLECTIONS), vec![vec![this, viewer]]);
        assert!(s.calls(SETUP_SKY_REFLECTIONS).is_empty());
        assert_eq!(s.e.global::<u32>(WORLD_REFLECTION_THREAD_STAGE), 0);
        assert_eq!(s.e.mem.u32(this), 1, "one stage was used");
        assert_eq!(s.calls(FINISH_WORLD_REFLECTIONS), vec![vec![this]]);
        assert_eq!(s.calls(FINISH_GROUP), vec![vec![this, group]]);
        assert!(s.calls(RELEASE_GROUP).is_empty());
        assert_eq!(
            s.calls(MAP_REMOVE_ALL),
            vec![vec![this + 0x4c], vec![this + 0x5c], vec![this + 0x6c]]
        );
        assert_eq!(
            s.calls(LIST_REMOVE_ALL),
            vec![vec![group + 0x7c], vec![group + 0x88]]
        );
        // No depth was asked for; the viewer's position went to 004e20c0.
        assert!(s.calls(RENDER_GROUP_DEPTH).is_empty());
        assert_eq!(
            s.calls(VIEWER_POSITION_UPDATE),
            vec![vec![
                0x1e,
                1.0f32.to_bits(),
                2.0f32.to_bits(),
                3.0f32.to_bits(),
                0
            ]]
        );
        // The reference's property: bReflections etc. are off, so every
        // texture slot was cleared.
        for offset in [0x13c, 0x140, 0x144, 0x138] {
            assert_eq!(
                s.e.mem.u32(s.property.addr() + offset),
                0,
                "slot {offset:x}"
            );
        }
    }

    #[test]
    fn a_group_not_at_the_world_height_asks_for_sky_reflections() {
        let mut s = scene();
        s.e.mem.set_u8(s.group.addr() + 0x5c, 0);
        s.update(false);
        let (this, viewer) = (s.this.addr(), s.viewer);
        assert_eq!(s.e.global::<u8>(RENDER_SKY_REFLECTIONS), 1);
        assert_eq!(s.e.global::<u8>(RENDER_WORLD_REFLECTIONS), 0);
        assert_eq!(s.calls(SETUP_SKY_REFLECTIONS), vec![vec![this, viewer]]);
        assert_eq!(s.e.global::<u32>(SKY_REFLECTION_THREAD_STAGE), 0);
        assert_eq!(s.calls(FINISH_SKY_REFLECTIONS), vec![vec![this]]);
    }

    #[test]
    fn something_in_the_water_adds_a_depth_pass_per_group() {
        let mut s = scene();
        // A second group with the same reference.
        let second = s.e.mem.alloc(0xb0);
        s.e.mem.set_u32(second, s.water_form);
        s.e.mem.set_u8(second + 0x5c, 1);
        s.e.mem.set_u32(second + 0xa8, 0x0ca0_0002);
        s.e.mem.set_u32(second + 0xac, 0x0001_0005);
        fill_list(&mut s.e, second + 0x24, &[s.reference]);
        fill_list(&mut s.e, s.this.addr() + 0x3c, &[s.group.addr(), second]);
        // An actor is in the water of both groups.
        fill_list(&mut s.e, s.group.addr() + 0x3c, &[1]);
        fill_list(&mut s.e, second + 0x3c, &[1]);
        s.update(false);
        let (this, group, viewer) = (s.this.addr(), s.group.addr(), s.viewer);
        assert!(s.group_flag(0x5e), "bRenderDepth");
        assert_eq!(
            s.calls(SETUP_GROUP_DEPTH),
            vec![vec![this, viewer, group], vec![this, viewer, second]]
        );
        // Stage 0 went to the world reflections.
        assert_eq!(s.e.mem.u32(group + 0xa0), 1);
        assert_eq!(s.e.mem.u32(second + 0xa0), 2);
        // The depth pass: the first group puts the target in mode 7.
        assert_eq!(
            s.calls(RENDER_TARGET_SET),
            vec![vec![7, 0x7777], vec![0, 0x7777]]
        );
        assert_eq!(s.calls(RENDERED_TEXTURE_STOP), vec![vec![DEPTH_TEXTURE]; 2]);
        assert_eq!(
            s.calls(RENDER_GROUP_DEPTH),
            vec![
                vec![this, 0x0ca0_0001, group, 1, 3],
                vec![this, 0x0ca0_0002, second, 0, 5]
            ]
        );
        assert_eq!(s.calls(BATCH_RENDERER_END_PASS).len(), 2);
        assert_eq!(s.e.global::<u32>(LAST_DEPTH_STENCIL_MASK), 0x0001_0005);
        let constructs = s.calls(SHADER_PARAM_CONSTRUCT);
        assert_eq!(constructs.len(), 2);
        let param = constructs[0][0];
        assert_eq!(
            s.calls(SHADER_PARAM_INIT_CONSTANT_MAP)[0],
            vec![param, 0, 1]
        );
        assert_eq!(
            s.calls(SHADER_PARAM_SET_PIXEL_CONSTANT)[0],
            vec![param, 0, 1.0f32.to_bits(), 0, 0, 1.0f32.to_bits()]
        );
        assert_eq!(
            s.calls(SET_STENCIL_STATE),
            vec![vec![2, 0xff, 0x0007_0003, 0], vec![2, 0xff, 0x0001_0005, 0]]
        );
        assert_eq!(
            s.calls(IMAGE_SPACE_RENDER_EFFECT)[0],
            vec![0x0ee0_0000, 0x28, s.renderer, DEPTH_TEXTURE, param, 0]
        );
        assert_eq!(
            s.calls(AFTER_GROUP_DEPTH),
            vec![vec![this, group, 3], vec![this, second, 5]]
        );
        assert_eq!(s.calls(RENDER_TARGET_RESET).len(), 2);
        assert_eq!(s.calls(SHADER_PARAM_DESTRUCT).len(), 2);
        // The clear colour is saved, set to the depth colour and restored.
        let saved = s.calls(RENDERER_GET_COLOR_DOUBLE);
        assert_eq!(saved.len(), 1);
        let set = s.calls(RENDERER_SET_COLOR_DOUBLE);
        assert_eq!(set.len(), 2);
        assert_eq!(set[1], saved[0], "the saved colour is restored");
        assert_eq!(set[0][0], s.renderer);
    }

    #[test]
    fn no_depth_pass_without_the_depth_setting() {
        let mut s = scene();
        fill_list(&mut s.e, s.group.addr() + 0x3c, &[1]);
        set_setting(&mut s.e, SETTING_USE_WATER_DEPTH, false);
        s.update(false);
        assert!(s.group_flag(0x5e), "the group still wants depth");
        assert!(s.calls(SETUP_GROUP_DEPTH).is_empty());
        assert!(s.calls(RENDER_GROUP_DEPTH).is_empty());
        // The shared depth map is given back instead.
        assert_eq!(
            s.calls(RETURN_RENDERED_TEXTURE),
            vec![vec![s.texture_manager, DEPTH_TEXTURE]]
        );
        assert_eq!(s.e.global::<u32>(DEPTH_MAP), 0);
    }

    #[test]
    fn a_reference_out_of_range_leaves_its_group_off_and_gives_the_textures_back() {
        let mut s = scene();
        s.e.mem.set_u8(s.reference + REFERENCE_IN_RANGE, 0);
        s.update(false);
        let (this, group) = (s.this.addr(), s.group.addr());
        assert!(!s.group_flag(0x5d));
        assert_eq!(s.e.global::<u32>(ACTIVE_WATER_GROUPS), 0);
        // The group is released, no pass is set up.
        assert_eq!(s.calls(RELEASE_GROUP), vec![vec![this, group, 0, 0]]);
        assert!(s.calls(SETUP_WORLD_REFLECTIONS).is_empty());
        assert!(s.calls(FINISH_GROUP).is_empty());
        // The shared depth and wading maps and the water form's noise map
        // are returned and cleared.
        let tm = s.texture_manager;
        assert_eq!(
            s.calls(RETURN_RENDERED_TEXTURE),
            vec![
                vec![tm, DEPTH_TEXTURE],
                vec![tm, WADING_TEXTURE],
                vec![tm, FORM_NOISE_TEXTURE]
            ]
        );
        assert_eq!(s.e.global::<u32>(DEPTH_MAP), 0);
        assert_eq!(s.e.global::<u32>(WADING_WATER_HEIGHT_MAP), 0);
        assert_eq!(s.e.mem.u32(s.water_form + 0x30), 0);
        // The reference's property lost all its textures.
        for offset in [0x13c, 0x140, 0x144, 0x138] {
            assert_eq!(
                s.e.mem.u32(s.property.addr() + offset),
                0,
                "slot {offset:x}"
            );
        }
    }

    #[test]
    fn a_forced_update_switches_the_group_on_out_of_range() {
        let mut s = scene();
        s.e.mem.set_u8(s.reference + REFERENCE_IN_RANGE, 0);
        s.update(true);
        assert!(s.group_flag(0x5d));
        assert_eq!(s.calls(FINISH_GROUP).len(), 1);
    }

    #[test]
    fn a_culled_node_is_never_in_range() {
        let mut s = scene();
        let node = s.e.mem.u32(s.reference + REFERENCE_NODE);
        s.e.mem.set_u32(node + 0x30, 1);
        s.update(false);
        assert!(!s.group_flag(0x5d));
        // The range test is not even asked.
        assert!(s.calls(REFERENCE_IS_IN_RANGE).is_empty());
    }

    #[test]
    fn a_reference_without_the_flag_is_ignored_unless_its_cell_matches() {
        let mut s = scene();
        s.e.mem.set_u32(s.base_form + BASE_FORM_FLAGS, 0x1000_0000);
        s.update(false);
        assert!(!s.group_flag(0x5d), "the 0x08000000 flag is missing");
        let cell = s.e.mem.alloc(0x40);
        s.e.mem.set_u8(cell + 0x26, 6);
        s.e.mem.set_u32(s.reference + 0x40, cell);
        s.update(false);
        assert!(s.group_flag(0x5d), "a parent cell of kind 6 is enough");
    }

    #[test]
    fn in_an_interior_the_group_renders_its_own_reflections() {
        let mut s = scene();
        let tes = s.e.global::<u32>(TES_POINTER);
        s.e.mem.set_u32(tes + 0x34, 0x00ce_0000);
        s.e.mem.set_u32(s.this.addr(), 3);
        let cell = s.e.mem.alloc(0x40);
        s.e.mem.set_u8(cell + 0x26, 6);
        s.e.mem.set_u32(s.reference + 0x40, cell);
        s.update(false);
        let (this, group, viewer) = (s.this.addr(), s.group.addr(), s.viewer);
        assert!(s.group_flag(0x5d));
        assert!(s.group_flag(0x5f), "bRenderGroupReflections");
        assert_eq!(s.e.global::<u8>(RENDER_WORLD_REFLECTIONS), 0);
        assert_eq!(s.e.global::<u8>(RENDER_SKY_REFLECTIONS), 0);
        assert!(s.calls(SETUP_WORLD_REFLECTIONS).is_empty());
        assert!(s.calls(FINISH_WORLD_REFLECTIONS).is_empty());
        // The high-detail flag is 0 in an interior.
        assert_eq!(
            s.calls(SETUP_GROUP_REFLECTIONS),
            vec![vec![this, viewer, group, 0]]
        );
        assert_eq!(s.e.mem.u32(group + 0x9c), 3);
        assert_eq!(s.e.mem.u32(this), 4);
        assert_eq!(s.calls(FINISH_GROUP_INTERIOR), vec![vec![this, group]]);
        assert_eq!(s.calls(FINISH_GROUP), vec![vec![this, group]]);
    }

    #[test]
    fn too_many_groups_log_a_message_and_switch_the_group_off() {
        let mut s = scene();
        s.e.mem.set_u32(s.this.addr(), 15);
        s.update(false);
        let (this, group) = (s.this.addr(), s.group.addr());
        // The world reflections took stage 15, so the group has none left.
        assert_eq!(s.e.global::<u32>(WORLD_REFLECTION_THREAD_STAGE), 15);
        assert!(!s.group_flag(0x5d));
        let print = s.calls(FORMATTED_PRINT);
        assert_eq!(print.len(), 1);
        let height = 5.5f64.to_bits();
        assert_eq!(
            print[0][1..],
            [
                0x100,
                TOO_MANY_GROUPS_MESSAGE,
                height as u32,
                (height >> 32) as u32,
                0x0555_0000,
                1
            ]
        );
        let text = print[0][0];
        assert_eq!(s.calls(LOG_MESSAGE), vec![vec![FORMAT_STRING, text]]);
        assert!(s.calls(SETUP_GROUP_REFLECTIONS).is_empty());
        assert_eq!(s.calls(RELEASE_GROUP), vec![vec![this, group, 0, 0]]);
    }

    /// Runs the scene with `flags` as the base form flags and returns the
    /// group's `bRenderDepth`.
    fn depth_wanted(mut s: Scene, flags: u32) -> bool {
        s.e.mem.set_u32(s.base_form + BASE_FORM_FLAGS, flags);
        s.update(false);
        s.group_flag(0x5e)
    }

    #[test]
    fn depth_needs_both_the_depth_and_the_refraction_flags() {
        let base = 0x0800_0000;
        assert!(!depth_wanted(scene(), base | 0x1000_0000));
        assert!(!depth_wanted(scene(), base | 0x200));
        // With both flags but nothing in the water and no reason: no depth.
        assert!(!depth_wanted(scene(), base | 0x1000_0000 | 0x200));
    }

    #[test]
    fn depth_is_wanted_for_objects_in_the_water() {
        let mut s = scene();
        fill_list(&mut s.e, s.group.addr() + 0x30, &[1]);
        assert!(depth_wanted(s, 0x0800_0000 | 0x1000_0000 | 0x200));
    }

    #[test]
    fn depth_is_wanted_for_the_800_flag() {
        let s = scene();
        assert!(depth_wanted(s, 0x0800_0000 | 0x1000_0000 | 0x200 | 0x800));
    }

    #[test]
    fn depth_is_wanted_when_the_reflected_refs_are_not_empty() {
        let mut s = scene();
        // `ExtraDataList::GetReflectedRefs` returns the word at +0x44 of the
        // reference; the object it names is not empty.
        let reflected = s.e.mem.alloc(8);
        s.e.mem.set_u32(reflected, 0x1234);
        s.e.mem.set_u32(s.reference + 0x44, reflected);
        assert!(depth_wanted(s, 0x0800_0000 | 0x1000_0000 | 0x200));
        // An empty one is not a reason.
        let mut s = scene();
        let reflected = s.e.mem.alloc(8);
        s.e.mem.set_u32(s.reference + 0x44, reflected);
        assert!(!depth_wanted(s, 0x0800_0000 | 0x1000_0000 | 0x200));
    }

    #[test]
    fn depth_is_wanted_for_the_40000000_flag_with_the_static_object_filled() {
        let flags = 0x0800_0000 | 0x1000_0000 | 0x200 | 0x4000_0000;
        let mut s = scene();
        s.e.mem.set_u32(WATER_OBJECT_011CA13C, 1);
        assert!(depth_wanted(s, flags));
        let s = scene();
        assert!(!depth_wanted(s, flags));
    }

    #[test]
    fn silhouette_reflections_follow_the_auto_setting_and_the_form_flags() {
        let mut s = scene();
        set_setting(&mut s.e, SETTING_AUTO_SILHOUETTE_REFLECTIONS, true);
        s.e.mem.set_u32(
            s.base_form + BASE_FORM_FLAGS,
            0x0800_0000 | 0x1000_0000 | 0x200 | 0x4_0000,
        );
        s.update(false);
        assert!(s.group_flag(0x60));
        // Without the auto setting they stay off.
        let mut s = scene();
        s.e.mem.set_u32(
            s.base_form + BASE_FORM_FLAGS,
            0x0800_0000 | 0x1000_0000 | 0x200 | 0x4_0000,
        );
        s.update(false);
        assert!(!s.group_flag(0x60));
        // The low-detail setting alone is enough.
        let mut s = scene();
        set_setting(&mut s.e, SETTING_AUTO_SILHOUETTE_REFLECTIONS, true);
        set_setting(&mut s.e, SETTING_FORCE_LOW_DETAIL_REFLECTIONS, true);
        s.update(false);
        assert!(s.group_flag(0x60));
    }

    #[test]
    fn a_rendered_group_keeps_only_the_textures_the_settings_allow() {
        let mut s = scene();
        // The property wants reflections and depth; refractions are off.
        s.e.mem.set_u8(s.property.addr() + 0x80, 1);
        s.e.mem.set_u8(s.property.addr() + 0x63, 1);
        s.update(false);
        let property = s.property.addr();
        assert_ne!(s.e.mem.u32(property + 0x13c), 0, "reflection map kept");
        assert_eq!(s.e.mem.u32(property + 0x140), 0, "refraction map cleared");
        assert_ne!(s.e.mem.u32(property + 0x144), 0, "depth map kept");
        assert_eq!(s.e.mem.u32(property + 0x138), 0, "normal map cleared");
    }

    #[test]
    fn a_rendered_group_with_reflections_off_is_released() {
        let mut s = scene();
        set_setting(&mut s.e, SETTING_USE_WATER_REFLECTIONS, false);
        s.update(false);
        let (this, group) = (s.this.addr(), s.group.addr());
        assert_eq!(s.calls(RELEASE_GROUP), vec![vec![this, group, 0, 0]]);
    }

    #[test]
    fn the_wading_geometry_of_a_group_loses_its_textures_when_the_group_is_off() {
        let mut s = scene();
        s.e.mem.set_u8(s.reference + REFERENCE_IN_RANGE, 0);
        let wading = s.e.mem.alloc(0x200);
        let wading_property = s.e.mem.alloc(0x200);
        s.e.mem.set_u32(wading + NODE_PROPERTY, wading_property);
        for offset in [0x13c, 0x140, 0x144, 0x138] {
            s.e.mem.set_u32(wading_property + offset, 0xaaaa);
        }
        s.e.mem.set_u32(s.group.addr() + 0x58, wading);
        s.update(false);
        for offset in [0x13c, 0x140, 0x144, 0x138] {
            assert_eq!(s.e.mem.u32(wading_property + offset), 0, "slot {offset:x}");
        }
    }

    // --- UpdateLODWater -----------------------------------------------------

    /// A scene whose single group is the system's LOD water group.
    fn lod_scene() -> Scene {
        let mut s = scene();
        let (this, group) = (s.this.addr(), s.group.addr());
        s.e.mem.set_u32(this + 0x48, group);
        s.e.mem.set_u8(group + 0x5d, 0);
        // The shared reflection maps the LOD group points the property at.
        s.e.set_global(WORLD_REFLECTION_MAP, 0x1111u32);
        s.e.set_global(SKY_REFLECTION_MAP, 0x2222u32);
        s
    }

    impl Scene {
        fn update_lod(&mut self, force_update: bool) {
            start_log(&mut self.e);
            let (this, viewer) = (self.this, self.viewer);
            self.e.call(0x004e_3320, &args![this, viewer, force_update]);
        }
    }

    #[test]
    fn lod_water_without_a_group_does_nothing() {
        let mut s = scene();
        s.update_lod(false);
        // Only the call itself is in the log.
        assert_eq!(s.e.call_log.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn lod_water_in_an_interior_only_logs_a_message() {
        let mut s = lod_scene();
        let tes = s.e.global::<u32>(TES_POINTER);
        s.e.mem.set_u32(tes + 0x34, 1);
        s.e.mem.set_u8(s.group.addr() + 0x5d, 1);
        s.update_lod(false);
        assert!(s.calls(WATER_REFERENCE_3D).is_empty());
        assert_eq!(s.calls(LOG_MESSAGE), vec![vec![LOD_IN_INTERIOR_MESSAGE]]);
        assert!(s.group_flag(0x5d), "the group is left as it was");
    }

    #[test]
    fn lod_water_in_range_points_the_property_at_the_sky_map() {
        let mut s = lod_scene();
        s.update_lod(false);
        assert!(s.group_flag(0x5d));
        assert_eq!(s.e.global::<u8>(RENDER_SKY_REFLECTIONS), 1);
        assert_eq!(s.e.global::<u8>(RENDER_WORLD_REFLECTIONS), 0);
        assert_eq!(
            s.calls(NI_POINTER_ASSIGN_FROM),
            vec![vec![s.property.addr() + 0x13c, SKY_REFLECTION_MAP]]
        );
        assert_eq!(s.e.mem.u32(s.property.addr() + 0x13c), 0x2222);
        // The rendered group's shader properties are updated afterwards.
        assert_eq!(
            s.calls(WATER_REFERENCE_3D).last().unwrap(),
            &vec![s.this.addr(), s.reference]
        );
    }

    #[test]
    fn lod_water_with_full_reflections_uses_the_world_map() {
        let mut s = lod_scene();
        s.e.mem.set_u8(s.property.addr() + 0x62, 1);
        s.update_lod(false);
        assert_eq!(s.e.global::<u8>(RENDER_WORLD_REFLECTIONS), 1);
        assert_eq!(s.e.global::<u8>(RENDER_SKY_REFLECTIONS), 0);
        assert_eq!(s.e.mem.u32(s.property.addr() + 0x13c), 0x1111);
    }

    #[test]
    fn lod_water_out_of_range_drops_the_reflection_map() {
        let mut s = lod_scene();
        s.e.mem.set_u8(s.reference + REFERENCE_IN_RANGE, 0);
        s.update_lod(false);
        assert!(!s.group_flag(0x5d));
        assert_eq!(s.e.mem.u32(s.property.addr() + 0x13c), 0);
        // The other slots stay.
        assert_ne!(s.e.mem.u32(s.property.addr() + 0x140), 0);
        // Nothing is updated for a group that is not rendered.
        assert_eq!(s.calls(WATER_REFERENCE_3D).len(), 1);
    }

    #[test]
    fn lod_water_forced_update_ignores_the_range() {
        let mut s = lod_scene();
        s.e.mem.set_u8(s.reference + REFERENCE_IN_RANGE, 0);
        s.update_lod(true);
        assert!(s.group_flag(0x5d));
        assert_eq!(s.e.global::<u8>(RENDER_SKY_REFLECTIONS), 1);
    }

    #[test]
    fn lod_water_while_water_is_skipped_releases_the_group() {
        let mut s = lod_scene();
        s.e.mem.set_u8(0x011c_7a59, 1);
        s.update_lod(false);
        let (this, group) = (s.this.addr(), s.group.addr());
        assert_eq!(s.calls(RELEASE_GROUP), vec![vec![this, group, 0, 0]]);
        assert_eq!(s.e.global::<u8>(RENDER_SKY_REFLECTIONS), 0);
        assert!(s.calls(NI_POINTER_ASSIGN_FROM).is_empty());
    }

    // --- UpdateWaterShaderProperties ----------------------------------------

    #[test]
    fn shader_properties_are_updated_for_every_reference_of_the_group() {
        let mut s = scene();
        let second = object(&mut s.e, REFERENCE_VTABLE, 0x100);
        let (this, group) = (s.this.addr(), s.group.addr());
        fill_list(&mut s.e, group + 0x24, &[s.reference, second]);
        start_log(&mut s.e);
        s.e.call(0x004e_3520, &args![this, group]);
        assert_eq!(
            s.calls(WATER_REFERENCE_3D),
            vec![vec![this, s.reference], vec![this, second]]
        );
    }

    // --- UpdateWaterShaderProperties_ov2 ------------------------------------

    /// The packed colours the water form returns: red 0, green 0x80, blue 0xff.
    const SHALLOW_COLOR: u32 = 0x00ff_8000;
    const DEEP_COLOR: u32 = 0x0000_ff00;
    const REFLECTION_COLOR: u32 = 0x0033_6699;

    /// A scene ready for `ov2`: the reference's water form exists, the
    /// property wants its constants updated and all the water form getters
    /// answer with distinct values.
    fn shader_scene() -> Scene {
        let mut s = scene();
        s.e.mem
            .set_u32(s.base_form + BASE_FORM_WATER_FORM, s.water_form);
        s.e.mem.set_u32(s.water_node + NODE_VIRTUAL_RESULT, 1);
        s.e.mem.set_u8(s.property.addr() + 0x7f, 1);
        let form = s.water_form;
        s.e.mem.set_f32(form + 0x9c, 0.75);
        s.e.mem.set_f32(form + 0x108, 0.3);
        s.e.mem.set_f32(form + 0x114, 20.0);
        s.e.mem.set_f32(form + 0x128, 30.0);
        s.e.mem.set_f32(form + 0x12c, 40.0);
        for (address, color) in [
            (0x004f_9bf0u32, SHALLOW_COLOR),
            (0x009e_e040, DEEP_COLOR),
            (0x008d_80e0, REFLECTION_COLOR),
        ] {
            s.e.register_double(address, move |_, _| ret(color));
        }
        for (address, value) in [
            (0x0064_4930u32, 1.5f32),
            (0x008d_01e0, 2.5),
            (0x0058_0100, 3.5),
            (0x004a_0d90, 4.5),
            (0x004a_0db0, 5.5),
            (0x0064_6f50, 6.5),
            (0x009a_5480, 7.5),
            (0x0088_5d70, 8.5),
            (0x0050_7b20, 100.0),
            (0x0050_8070, 9.5),
            (0x009b_88a0, 50.0),
            (0x0081_2870, 10.0),
            (0x0094_42a0, 11.5),
            (0x0064_4950, 12.5),
            (0x0064_4970, 13.5),
        ] {
            s.e.register_double(address, move |_, _| value.into_ret());
        }
        s
    }

    impl Scene {
        fn update_shader(&mut self) {
            start_log(&mut self.e);
            let (this, group, reference) = (self.this, self.group, self.reference);
            self.e.call(0x004e_3590, &args![this, group, reference]);
        }

        fn property_f32(&self, offset: u32) -> f32 {
            self.e.mem.f32(self.property.addr() + offset)
        }
    }

    #[test]
    fn the_water_form_is_copied_into_the_shader_property() {
        let mut s = shader_scene();
        s.update_shader();
        // Colours: red, green, blue, alpha 1.0.
        let color = |s: &Scene, offset: u32| -> [f32; 4] {
            [0, 4, 8, 12].map(|i| s.property_f32(offset + i))
        };
        assert_eq!(color(&s, 0x88), [0.0, 128.0 / 255.0, 1.0, 1.0]);
        assert_eq!(color(&s, 0x98), [0.0, 1.0, 0.0, 1.0]);
        assert_eq!(
            color(&s, 0xa8),
            [
                (0x99 as f64 / 255.0) as f32,
                (0x66 as f64 / 255.0) as f32,
                (0x33 as f64 / 255.0) as f32,
                1.0
            ]
        );
        // Amounts from the water form.
        assert_eq!(s.property_f32(0xb8), 1.5);
        assert_eq!(s.property_f32(0xbc), 2.5);
        assert_eq!(s.property_f32(0xc0), 3.5);
        assert_eq!(s.property_f32(0xc4), 4.5);
        assert_eq!(s.property_f32(0x118), 0.75);
        assert_eq!(s.property_f32(0x100), 5.5);
        assert_eq!(s.property_f32(0x104), 6.5);
        assert_eq!(s.property_f32(0xd0), 30.0);
        assert_eq!(s.property_f32(0xd4), 40.0);
        assert_eq!(s.property_f32(0x11c), 7.5);
        assert_eq!(s.e.global::<f32>(WATER_SHADER_FLOAT), 8.5);
        assert_eq!(s.property_f32(0x124), 11.5);
        // Not underwater: the fog is 50 (far) and 50 - 10 (range), and the
        // form's own fog amount while its 0x80000000 flag is clear.
        assert_eq!(s.property_f32(0x74), 50.0);
        assert_eq!(s.property_f32(0x78), 40.0);
        assert_eq!(s.property_f32(0x120), 0.3);
        // The depth falloff.
        assert_eq!(s.property_f32(0xd8), 12.5);
        assert_eq!(s.property_f32(0xdc), 13.5);
    }

    #[test]
    fn underwater_the_fog_comes_from_the_underwater_values() {
        let mut s = shader_scene();
        s.e.mem.set_u8(0x011c_7a59, 1);
        s.e.mem.set_f32(s.property.addr() + 0x120, 77.0);
        s.update_shader();
        assert_eq!(s.property_f32(0x74), 100.0);
        // 100 minus the form's fUnderwaterFogDistNear (20).
        assert_eq!(s.property_f32(0x78), 80.0);
        assert_eq!(s.property_f32(0x120), 9.5);
        // With the 0x80000000 flag set the fog amount is left alone.
        let mut s = shader_scene();
        s.e.mem.set_u8(0x011c_7a59, 1);
        s.e.mem.set_u32(s.base_form + BASE_FORM_FLAGS, 0x8000_0000);
        s.e.mem.set_f32(s.property.addr() + 0x120, 77.0);
        s.update_shader();
        assert_eq!(s.property_f32(0x120), 77.0);
    }

    #[test]
    fn the_flag_byte_at_011c7a58_selects_the_underwater_values_too() {
        let mut s = shader_scene();
        s.e.set_global(WATER_FLAG_011C7A58, 1u8);
        s.update_shader();
        assert_eq!(s.property_f32(0x74), 100.0);
    }

    #[test]
    fn a_form_with_the_80000000_flag_has_no_fog_amount_when_not_underwater() {
        let mut s = shader_scene();
        s.e.mem.set_u32(s.base_form + BASE_FORM_FLAGS, 0x8000_0000);
        s.e.mem.set_f32(s.property.addr() + 0x120, 77.0);
        s.update_shader();
        assert_eq!(s.property_f32(0x120), 0.0);
    }

    #[test]
    fn the_property_flags_follow_the_form_and_the_settings() {
        let mut s = shader_scene();
        // Base form: reflections flag (1) and the depth flag, no refraction flag.
        s.e.mem
            .set_u32(s.base_form + BASE_FORM_FLAGS, 1 | 0x1000_0000 | 0x2000_0000);
        s.update_shader();
        let property = s.property.addr();
        assert_eq!(s.e.mem.u8(property + 0x80), 1, "bReflections");
        assert_eq!(s.e.mem.u8(property + 0x81), 0, "bRefractions");
        assert_eq!(s.e.mem.u8(property + 0x63), 1, "bDepth");
        assert_eq!(s.e.mem.u8(property + 0x82), 1, "bObjectTexCoords");
        // A setting that is off keeps the flag off.
        let mut s = shader_scene();
        set_setting(&mut s.e, SETTING_USE_WATER_REFLECTIONS, false);
        s.e.mem.set_u32(s.base_form + BASE_FORM_FLAGS, 1);
        s.update_shader();
        assert_eq!(s.e.mem.u8(s.property.addr() + 0x80), 0);
    }

    #[test]
    fn a_changed_flag_resets_the_render_pass_lists() {
        let mut s = shader_scene();
        let property = s.property.addr();
        let passes = s.e.mem.alloc(0x20);
        s.e.mem.set_u32(passes + 0x10, 5);
        s.e.mem.set_u32(property + 0x3c, passes);
        // The group's wading geometry has a property with passes.
        let wading = s.e.mem.alloc(0x200);
        let wading_property = object(&mut s.e, PROPERTY_VTABLE, 0x200);
        let wading_passes = s.e.mem.alloc(0x20);
        s.e.mem.set_u32(wading_passes + 0x10, 6);
        s.e.mem.set_u32(wading_property + 0x3c, wading_passes);
        s.e.mem.set_u32(wading_property + PROPERTY_PASS_COUNT, 2);
        s.e.mem.set_u32(wading + NODE_PROPERTY, wading_property);
        s.e.mem.set_u32(s.group.addr() + 0x58, wading);
        s.update_shader();
        assert_eq!(s.e.mem.u32(passes + 0x10), 0);
        assert_eq!(s.e.mem.u32(wading_passes + 0x10), 0);
        // The property's virtual at +0xa4 was asked with 0.
        assert_eq!(
            s.calls(PROPERTY_PASS_COUNT_DOUBLE),
            vec![vec![wading_property, 0]]
        );
    }

    #[test]
    fn unchanged_flags_leave_the_render_pass_lists_alone() {
        let mut s = shader_scene();
        let property = s.property.addr();
        // The flags the default base form and settings give: refractions, depth.
        s.e.mem.set_u8(property + 0x81, 1);
        s.e.mem.set_u8(property + 0x63, 1);
        let passes = s.e.mem.alloc(0x20);
        s.e.mem.set_u32(passes + 0x10, 5);
        s.e.mem.set_u32(property + 0x3c, passes);
        s.update_shader();
        assert_eq!(s.e.mem.u32(passes + 0x10), 5);
        assert!(s.calls(PROPERTY_PASS_COUNT_DOUBLE).is_empty());
    }

    #[test]
    fn a_wading_property_without_passes_keeps_its_list() {
        let mut s = shader_scene();
        let wading = s.e.mem.alloc(0x200);
        let wading_property = object(&mut s.e, PROPERTY_VTABLE, 0x200);
        let wading_passes = s.e.mem.alloc(0x20);
        s.e.mem.set_u32(wading_passes + 0x10, 6);
        s.e.mem.set_u32(wading_property + 0x3c, wading_passes);
        s.e.mem.set_u32(wading + NODE_PROPERTY, wading_property);
        s.e.mem.set_u32(s.group.addr() + 0x58, wading);
        s.update_shader();
        assert_eq!(s.e.mem.u32(wading_passes + 0x10), 6);
    }

    #[test]
    fn an_equal_depth_falloff_pair_becomes_minus_one_and_zero() {
        let mut s = shader_scene();
        s.e.register_double(0x0064_4970, |_, _| 12.5f32.into_ret());
        s.e.mem.set_f32(s.property.addr() + 0xdc, 9.0);
        s.update_shader();
        assert_eq!(s.property_f32(0xd8), -1.0);
        assert_eq!(s.property_f32(0xdc), 0.0);
    }

    #[test]
    fn the_shader_property_is_left_alone_without_its_conditions() {
        // No water form for the base object.
        let mut s = shader_scene();
        s.e.mem.set_u32(s.base_form + BASE_FORM_WATER_FORM, 0);
        s.update_shader();
        assert_eq!(s.property_f32(0xb8), 0.0);
        // The property does not want its constants updated.
        let mut s = shader_scene();
        s.e.mem.set_u8(s.property.addr() + 0x7f, 0);
        s.update_shader();
        assert_eq!(s.property_f32(0xb8), 0.0);
        // The owner is not of the water type.
        let mut s = shader_scene();
        let owner = s.e.mem.u32(s.water_node + 0xc0);
        s.e.mem.set_u32(owner + 0x68, 3);
        s.update_shader();
        assert_eq!(s.property_f32(0xb8), 0.0);
        // The node's virtual at +0x18 says no.
        let mut s = shader_scene();
        s.e.mem.set_u32(s.water_node + NODE_VIRTUAL_RESULT, 0);
        s.update_shader();
        assert_eq!(s.property_f32(0xb8), 0.0);
        // The reference has no water node at all.
        let mut s = shader_scene();
        s.e.mem.set_u32(s.reference + REFERENCE_WATER_NODE, 0);
        s.update_shader();
        assert_eq!(s.property_f32(0xb8), 0.0);
    }

    // --- the water form update (004e3d60) -----------------------------------

    /// A flag the `WaterTypeUpdateMap` lookup double answers with.
    const MAP_FIND_ANSWER: u32 = 0x011c_7a5a;

    /// A scene ready for `fn_004e3d60`: the reference's base form has a water
    /// form with a noise texture, the property already has the system's
    /// noise map and the form its rendered noise texture, and every callee
    /// the scroll uses answers with a fixed value.
    fn form_scene() -> Scene {
        let mut s = shader_scene();
        let property = s.property.addr();
        let form = s.water_form;
        s.e.register(WATER_FORM_HAS_NOISE_TEXTURE, |_, _| ret(1));
        for address in [
            FLUSH_OBJECT_FLUSH,
            IMAGE_SPACE_RENDER_NOISE,
            WATER_TYPE_MAP_SET,
            POINTER_TEMP_DESTRUCT,
            CREATE_TEXTURE_IMAGE,
        ] {
            s.e.register(address, |_, _| Ret::default());
        }
        s.e.register(WATER_TYPE_MAP_FIND, |e, _| {
            ret(e.mem.u8(MAP_FIND_ANSWER) as u32)
        });
        s.e.register(FLOAT_TABLE_ENTRY, |_, _| 0.1f32.into_ret());
        s.e.register(CRT_SIN, |_, _| 0.5f64.into_ret());
        s.e.register(CRT_COS, |_, _| 0.25f64.into_ret());
        s.e.register(FLOAT_REMAINDER, |_, a| {
            ((f32::from_bits(a[0]) as f64) % (f32::from_bits(a[1]) as f64)).into_ret()
        });
        s.e.register(FLOAT_MAX, |_, a| {
            f32::from_bits(a[0]).max(f32::from_bits(a[1])).into_ret()
        });
        s.e.register(CRT_CEIL, |_, a| f64::take(a, &mut 0).ceil().into_ret());
        for (address, value) in [
            (0x0096_6a20u32, 10.0f32),
            (0x0064_47d0, 2.0),
            (0x0064_47f0, 10.0),
            (0x0082_1640, 50.0),
            (0x0082_1660, 20.0),
            (0x0056_7470, 1.25),
            (0x0082_1680, 2.25),
            (0x0081_33b0, 3.25),
        ] {
            s.e.register_double(address, move |_, _| value.into_ret());
        }
        let flush_object = s.e.mem.alloc(0x300);
        s.e.set_global(FLUSH_OBJECT, flush_object);
        // The water form: texture scroll rows, wind and the height scale.
        for (offset, value) in [
            (0x34u32, 0.2f32),
            (0x38, 0.1),
            (0x44, 0.3),
            (0x48, 0.4),
            (0x54, 0.9),
            (0x58, 0.5),
            (0xe8, 90.0),
            (0xf0, 45.0),
            (0xf8, 4.0),
            (0x130, 250.0),
        ] {
            s.e.mem.set_f32(form + offset, value);
        }
        s.e.mem.set_f32(property + 0x11c, 0.5);
        s.e.mem.set_u32(property + 0x134, 0xbeef);
        set_setting(&mut s.e, SETTING_USE_PER_WORLD_SPACE_NOISE, true);
        s.e.mem.set_u8(MAP_FIND_ANSWER, 0);
        s
    }

    impl Scene {
        fn update_forms(&mut self) {
            start_log(&mut self.e);
            let (this, group) = (self.this, self.group);
            self.e.call(0x004e_3d60, &args![this, group]);
        }
    }

    /// The scroll arithmetic: `(speed * step) * trig + old`, rounded to float.
    fn scrolled(old: f32, speed: f32, step: f32, trig: f64) -> f32 {
        (((speed as f64) * (step as f64)) * trig + old as f64) as f32
    }

    #[test]
    fn the_first_reference_of_a_form_scrolls_wraps_and_publishes_its_rows() {
        let mut s = form_scene();
        let (this, form) = (s.this.addr(), s.water_form);
        s.update_forms();
        // The reference's property shares the form's rendered noise texture.
        assert_eq!(s.e.mem.u32(s.property.addr() + 0x138), FORM_NOISE_TEXTURE);
        // Sine (0.5) moves the first channel of a row, cosine (0.25) the second.
        let step = 0.1f32;
        let row0 = (
            scrolled(0.2, 2.0, step, 0.5),
            scrolled(0.1, 2.0, step, 0.25),
        );
        let row1 = (
            scrolled(0.3, 4.0, step, 0.5),
            scrolled(0.4, 4.0, step, 0.25),
        );
        // 0.9 + 10 * 0.1 * 0.5 = 1.4 wraps to 0.4.
        let row2_x = scrolled(0.9, 10.0, step, 0.5);
        assert!(row2_x > 1.0);
        let wrapped = ((row2_x as f64) % 1.0) as f32;
        let row2 = (wrapped, scrolled(0.5, 10.0, step, 0.25));
        assert_eq!(s.e.mem.f32(form + 0x34), row0.0);
        assert_eq!(s.e.mem.f32(form + 0x38), row0.1);
        assert_eq!(s.e.mem.f32(form + 0x44), row1.0);
        assert_eq!(s.e.mem.f32(form + 0x48), row1.1);
        assert_eq!(s.e.mem.f32(form + 0x54), row2.0);
        assert_eq!(s.e.mem.f32(form + 0x58), row2.1);
        assert!(row2.0 < 0.5 && row2.0 > 0.39);
        // Published: the noise scale, the three rows and the scales.
        assert_eq!(s.e.global::<f32>(SHADER_NOISE_BASE), 0.5);
        assert_eq!(s.e.global::<f32>(SHADER_NOISE_BASE + 4), row0.0);
        assert_eq!(s.e.global::<f32>(SHADER_NOISE_BASE + 8), row0.1);
        assert_eq!(s.e.global::<f32>(SHADER_NOISE_BASE + 0x14), row1.0);
        assert_eq!(s.e.global::<f32>(SHADER_NOISE_BASE + 0x24), row2.0);
        // 250 / 100 rounded up is 3; 50 / 100 and 20 / 100 round up to 1.
        assert_eq!(s.e.global::<f32>(SHADER_NOISE_BASE + 0x34), 3.0);
        assert_eq!(s.e.global::<f32>(SHADER_NOISE_BASE + 0x38), 1.0);
        assert_eq!(s.e.global::<f32>(SHADER_NOISE_BASE + 0x3c), 1.0);
        assert_eq!(s.e.global::<f32>(SHADER_NOISE_BASE + 0x44), 1.25);
        assert_eq!(s.e.global::<f32>(SHADER_NOISE_BASE + 0x48), 2.25);
        assert_eq!(s.e.global::<f32>(SHADER_NOISE_BASE + 0x4c), 3.25);
        // The noise map is rendered and the form is marked as done.
        assert_eq!(
            s.calls(IMAGE_SPACE_RENDER_NOISE),
            vec![vec![
                0x0ee0_0000,
                0x21,
                s.renderer,
                0xbeef,
                FORM_NOISE_TEXTURE,
                0,
                1
            ]]
        );
        assert_eq!(
            s.calls(WATER_TYPE_MAP_SET),
            vec![vec![this + 0x6c, form, 1]]
        );
        assert_eq!(s.calls(WATER_TYPE_MAP_FIND)[0][..2], [this + 0x6c, form]);
    }

    #[test]
    fn a_form_already_updated_this_frame_is_not_scrolled_again() {
        let mut s = form_scene();
        s.e.mem.set_u8(MAP_FIND_ANSWER, 1);
        s.update_forms();
        assert_eq!(s.e.mem.f32(s.water_form + 0x34), 0.2);
        assert!(s.calls(IMAGE_SPACE_RENDER_NOISE).is_empty());
        assert!(s.calls(WATER_TYPE_MAP_SET).is_empty());
        // The property still shares the rendered noise texture.
        assert_eq!(s.e.mem.u32(s.property.addr() + 0x138), FORM_NOISE_TEXTURE);
    }

    #[test]
    fn the_flush_object_is_flushed_when_its_byte_is_clear() {
        let mut s = form_scene();
        s.update_forms();
        let flush_object = s.e.global::<u32>(FLUSH_OBJECT);
        assert_eq!(s.calls(FLUSH_OBJECT_FLUSH), vec![vec![flush_object]]);
        let mut s = form_scene();
        let flush_object = s.e.global::<u32>(FLUSH_OBJECT);
        s.e.mem.set_u8(flush_object + 0x2b8, 1);
        s.update_forms();
        assert!(s.calls(FLUSH_OBJECT_FLUSH).is_empty());
    }

    #[test]
    fn references_without_a_usable_water_form_are_skipped() {
        let mut s = form_scene();
        s.e.mem.set_u32(s.base_form + BASE_FORM_WATER_FORM, 0);
        s.update_forms();
        assert!(s.calls(FLUSH_OBJECT_FLUSH).is_empty());
        assert!(s.calls(WATER_REFERENCE_3D).is_empty());

        let mut s = form_scene();
        s.e.register(WATER_FORM_HAS_NOISE_TEXTURE, |_, _| ret(0));
        s.update_forms();
        assert!(s.calls(FLUSH_OBJECT_FLUSH).is_empty());

        // A reference without a water node, or with an owner of another type.
        let mut s = form_scene();
        s.e.mem.set_u32(s.reference + REFERENCE_WATER_NODE, 0);
        s.update_forms();
        assert!(s.calls(WATER_TYPE_MAP_FIND).is_empty());
        let mut s = form_scene();
        let owner = s.e.mem.u32(s.water_node + 0xc0);
        s.e.mem.set_u32(owner + 0x68, 4);
        s.update_forms();
        assert!(s.calls(WATER_TYPE_MAP_FIND).is_empty());
    }

    #[test]
    fn the_shared_noise_map_fills_an_empty_property_slot() {
        let mut s = form_scene();
        let (this, property) = (s.this.addr(), s.property.addr());
        s.e.mem.set_u32(this + 0x1c, 0x3131);
        s.e.mem.set_u32(property + 0x134, 0);
        // The slot is filled from the system's texture; the noise map is not
        // a real texture, so the form keeps the noise map it has.
        s.e.mem.set_u8(MAP_FIND_ANSWER, 1);
        s.update_forms();
        assert_eq!(
            s.calls(NI_POINTER_ASSIGN_FROM)[0],
            vec![property + 0x134, this + 0x1c]
        );
        assert_eq!(s.e.mem.u32(property + 0x134), 0x3131);
    }

    /// The doubles the form's own noise texture loading uses: a temporary
    /// `NiPointer` stores its initial pointer, and the form's `+0x17c`
    /// texture is copied into a slot.
    fn own_noise_scene(form_texture: u32) -> Scene {
        let mut s = form_scene();
        set_setting(&mut s.e, SETTING_USE_PER_WORLD_SPACE_NOISE, false);
        s.e.register(POINTER_TEMP_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            ret(a[0])
        });
        s.e.register(WATER_FORM_COPY_NOISE_TEXTURE, |e, a| {
            let texture = e.mem.u32(a[0] + 0x17c);
            e.mem.set_u32(a[1], texture);
            ret(a[1])
        });
        s.e.register(WATER_FORM_NOISE_NAME, |_, _| ret(0x7a7a));
        let form = s.water_form;
        s.e.mem.set_u32(form + 0x17c, form_texture);
        s.e.mem.set_u8(form + 0x190, 1);
        s.e.mem.set_u32(s.property.addr() + 0x134, 0);
        s.e.mem.set_u8(MAP_FIND_ANSWER, 1);
        s
    }

    #[test]
    fn a_form_without_a_loaded_noise_texture_creates_it_from_its_file() {
        let mut s = own_noise_scene(0);
        let (property, form) = (s.property.addr(), s.water_form);
        let tes = s.e.global::<u32>(TES_POINTER);
        s.update_forms();
        assert_eq!(
            s.calls(CREATE_TEXTURE_IMAGE),
            vec![vec![tes, 0x7a7a, property + 0x134, 1, 0]]
        );
        // The reset byte is cleared and the temporary is destroyed.
        assert_eq!(s.e.mem.u8(form + 0x190), 0);
        assert_eq!(s.calls(POINTER_TEMP_DESTRUCT).len(), 1);
        assert_eq!(s.calls(POINTER_TEMP_CONSTRUCT)[0][1], 0);
    }

    #[test]
    fn a_form_with_a_loaded_noise_texture_shares_it_with_the_property() {
        let mut s = own_noise_scene(0x4444);
        let (property, form) = (s.property.addr(), s.water_form);
        s.update_forms();
        assert!(s.calls(CREATE_TEXTURE_IMAGE).is_empty());
        assert_eq!(s.e.mem.u32(property + 0x134), 0x4444);
        assert_eq!(s.e.mem.u8(form + 0x190), 0);
    }

    #[test]
    fn a_property_with_a_noise_map_is_left_alone_unless_the_form_asks_for_a_reset() {
        let mut s = own_noise_scene(0x4444);
        let (property, form) = (s.property.addr(), s.water_form);
        s.e.mem.set_u32(property + 0x134, 0xbeef);
        s.e.mem.set_u8(form + 0x190, 0);
        s.update_forms();
        assert!(s.calls(POINTER_TEMP_CONSTRUCT).is_empty());
        assert_eq!(s.e.mem.u32(property + 0x134), 0xbeef);
        // With the reset byte set the texture is loaded again.
        s.e.mem.set_u8(form + 0x190, 1);
        s.update_forms();
        assert_eq!(s.calls(POINTER_TEMP_CONSTRUCT).len(), 1);
        assert_eq!(s.e.mem.u32(property + 0x134), 0x4444);
    }

    #[test]
    fn a_form_without_a_rendered_noise_texture_gets_one_the_size_of_the_noise_map() {
        let mut s = form_scene();
        let (property, form) = (s.property.addr(), s.water_form);
        let texture = object(&mut s.e, TEXTURE_VTABLE, 0x40);
        s.e.mem.set_u32(property + 0x134, texture);
        s.e.mem.set_u32(form + 0x30, 0);
        s.e.register(CREATE_RENDERED_TEXTURE, |_, _| ret(0x9999));
        s.e.mem.set_u8(MAP_FIND_ANSWER, 1);
        s.update_forms();
        assert_eq!(s.e.global::<u32>(NOISE_WIDTH), 512);
        assert_eq!(s.e.global::<u32>(NOISE_HEIGHT), 256);
        assert_eq!(
            s.calls(CREATE_RENDERED_TEXTURE),
            vec![vec![s.texture_manager, s.renderer, 0x13, 0, 0, 0]]
        );
        assert_eq!(s.e.mem.u32(form + 0x30), 0x9999);
        assert_eq!(s.e.mem.u32(property + 0x138), 0x9999);
    }
}
