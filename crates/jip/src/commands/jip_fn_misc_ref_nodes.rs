//! `functions_jip/jip_fn_misc_ref.h` (JIP LN NVSE): the commands that only READ a reference's
//! scene graph, on the node lookup of `nif_nodes.rs`: a block's translation, rotation, scale and
//! flags, its parent nodes, a node's child blocks, the nodes with collision, a geometry block's
//! material values and texture paths, and a reference's dimensions and centre from its bounding
//! box. The rest of
//! the file's reference commands are in `jip_fn_misc_ref.rs`.
//!
//! Still waiting (each also in AUDIT-lane-F.md):
//! - Scene graph writes: `SetNifBlockTranslation`, `SetNifBlockRotation`, `SetNifBlockScale`
//!   (JIP calls `NiNode::ResetCollision` and `NiAVObject::UpdateDownwardPass`),
//!   `SetNifBlockFlag`, `SetMaterialPropertyValue` (`NiMaterialProperty::SetTraitValue` and its
//!   recursive form), `RenameNifBlock`, `RemoveNifBlock`, `SetTextureTransformKey`,
//!   `PlayAnimSequence`, `SetAnimSequenceFrequency`, `SetRefrModelPath`: the write lane.
//! - Models, lights, lines and cameras attached into the graph (`AttachLight`, `RemoveLight`,
//!   `InsertNode`, `AttachModel`, `AttachLine`, `AttachExtraCamera`, `ProjectExtraCamera`) and
//!   `ModelHasBlock`, which also reads the nodes `InsertNode`/`AttachModel` record
//!   (`s_insertNodeMap`) and loads a base form's model (`ModelLoader::LoadModel`): JIP's
//!   insert-node state and its hooks.
//! - `GetAnimSequenceFrequency` reads only, but needs the controller manager's and its
//!   sequences' layouts confirmed and JIP's `StrCompareCI` walk: the next agent of this lane.
//! - Havok: the velocity, collision property and layer commands (`*Alt` velocity getters
//!   included), `ToggleNodeCollision`, ray casts.

use super::arrays::{create_array, Element};
use super::nif_nodes as nif;
use super::Handlers;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x262c, get_object_dimensions),
    (0x276f, get_center_pos),
    (0x278c, get_material_property_value),
    (0x284b, get_nif_block_translation),
    (0x284d, get_nif_block_rotation),
    (0x284f, get_nif_block_scale),
    (0x2851, get_nif_block_flag),
    (0x2871, get_nif_block_parent_nodes),
    (0x28f2, get_collision_nodes),
    (0x28f3, get_child_blocks),
    (0x28f4, get_block_texture_set),
    (0x290d, get_nif_block_translation_alt),
    (0x290e, get_nif_block_rotation_alt),
];

/// JIP's block name buffer (`char blockName[0x40]`).
const NAME_BUFFER: usize = 0x40;
/// `TESObjectREFR::baseForm` (+0x20), `TESBoundObject::bounds` (six `SInt16`, min x y z then max,
/// +0x24: `TESBoundObject`'s constructor `0050E820` sets +0x24 and +0x2A), `IsBoundObject`
/// (vtable +0xE8).
const BASE: u32 = 0x20;
const BOUNDS: u32 = 0x24;
const IS_BOUND_OBJECT: u32 = 0xe8;
/// `BSBound::centre` (+0xC) and `dimensions` (+0x18).
const BOUND_CENTRE: u32 = 0x0c;
const BOUND_DIMENSIONS: u32 = 0x18;
/// `NiGeometry`'s property state (+0x9C, one per property type: `UpdatePropertiesDownward`
/// `00A80820` stores type t at +0x9C + 4t): the material property (type 2) +0xA4.
pub(crate) const MATERIAL_PROPERTY: u32 = 0xa4;
/// `NiMaterialProperty`: specular +0x1C, emissive +0x28 (three floats each), glossiness +0x38,
/// alpha +0x3C, emit multiplier +0x40 (`NiMaterialProperty::LoadBinary` `00A75790`).
pub(crate) const SPECULAR: u32 = 0x1c;
pub(crate) const EMISSIVE: u32 = 0x28;
pub(crate) const GLOSSINESS: u32 = 0x38;
pub(crate) const ALPHA: u32 = 0x3c;
pub(crate) const EMIT_MULT: u32 = 0x40;
/// `bhkNiCollisionObject::worldObj` (+0x10: the collision object's `UpdateWorldData`,
/// `bhkCollisionObject`'s vtable `0103046C` +0x90 = `00C65890`, reads it first).
const WORLD_OBJECT: u32 = 0x10;

/// The block name argument, as JIP's 0x40-byte buffer holds it.
pub(crate) fn block_name(e: &Engine, read: &Read, i: usize, command: &str) -> Vec<u8> {
    super::buffer_arg(e, read, i, NAME_BUFFER, command)
}

/// The axis argument (`kParamType_Axis`: the letter) as JIP's `axis - 'X'`; `None` for any other
/// byte (JIP indexes with it unchecked; the compiler writes only X, Y or Z).
fn axis_of(read: &Read, i: usize) -> Option<u32> {
    match read.args.get(i) {
        Some(nvse::args::Arg::Byte(b)) => {
            let axis = b.wrapping_sub(b'X');
            (axis <= 2).then_some(u32::from(axis))
        }
        _ => None,
    }
}

/// `ResultVars::Set`: each value as a double into its script variable (`ScriptVar::data`, +8).
fn set_vars(e: &mut Engine, vars: [u32; 3], values: [f32; 3]) {
    for (var, v) in vars.into_iter().zip(values) {
        e.mem.set_f64(var.wrapping_add(8), f64::from(v));
    }
}

/// The three script variable arguments from `first`; `None` when the reader did not give three.
fn var_args(read: &Read, first: usize) -> Option<[u32; 3]> {
    let vars = [
        read.local(first),
        read.local(first + 1),
        read.local(first + 2),
    ];
    vars.iter().all(|&v| v != 0).then_some(vars)
}

/// Three floats as a three-element array (JIP's `ArrayElementL elements[3]`); its id the result.
fn array_of_3(e: &mut Engine, a: &CommandArgs, v: [f32; 3]) {
    let elements = v.map(|x| Element::Number(f64::from(x)));
    let id = create_array(e, a.script, &elements);
    a.set_result(e, f64::from(id));
}

// ---- bounds ----

/// The bound object's `bounds[i]` (an `SInt16`), `None` when unreadable.
fn bound(e: &Engine, object: u32, i: u32) -> Option<i32> {
    nif::rd16(e, object.wrapping_add(BOUNDS + 2 * i)).map(|v| i32::from(v as i16))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetObjectDimensions_Execute.
/// The size on the axis: of the given bound object (scale 1), or of the calling reference times
/// its scale: its bounding box's `centre + dimensions` when it has one, else its base form's
/// bounds (max less min, made positive).
///
/// JIP takes the base form as a bound object unchecked; the port leaves the result for a
/// reference without one.
fn get_object_dimensions(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let Some(axis) = axis_of(&read, 0) else {
        return handled(true);
    };
    let mut object = read.form(1);
    let mut scale = 1.0f32;
    if object == 0 {
        if a.this_obj == 0 {
            return handled(true);
        }
        scale = nif::scale(e, a.this_obj);
        let bounds = nif::bounding_box(e, a.this_obj);
        if bounds != 0 {
            let centre = nif::rdf32(e, bounds.wrapping_add(BOUND_CENTRE + 4 * axis));
            let size = nif::rdf32(e, bounds.wrapping_add(BOUND_DIMENSIONS + 4 * axis));
            if let (Some(c), Some(d)) = (centre, size) {
                a.set_result(e, f64::from((c + d) * scale));
            }
            return handled(true);
        }
        object = nif::rd32(e, a.this_obj.wrapping_add(BASE)).unwrap_or(0);
        if object == 0 {
            return handled(true);
        }
    }
    if let (Some(min), Some(max)) = (bound(e, object, axis), bound(e, object, axis + 3)) {
        a.set_result(e, f64::from(((max - min) as f32 * scale).abs()));
    }
    handled(true)
}

/// `TESForm::IsBoundObject` (the virtual function at +0xE8) of `form`; false when its vtable is
/// unreadable.
fn is_bound_object(e: &mut Engine, form: u32) -> bool {
    let Some(vtbl) = nif::rd32(e, form) else {
        return false;
    };
    let Some(f) = nif::rd32(e, vtbl.wrapping_add(IS_BOUND_OBJECT)) else {
        return false;
    };
    e.call(f, &[form]).eax & 0xff != 0
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetCenterPos_Execute.
/// The reference's position on the axis plus its centre there times its scale: the bounding
/// box's centre, else the middle of a bound base form's bounds (`(min + max) * 0.5`), else 0.
fn get_center_pos(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let Some(axis) = axis_of(&read, 0) else {
        return handled(true);
    };
    let refr = a.this_obj;
    let bounds = nif::bounding_box(e, refr);
    let mut centre = 0.0f32;
    if bounds != 0 {
        centre = nif::rdf32(e, bounds.wrapping_add(BOUND_CENTRE + 4 * axis)).unwrap_or(0.0);
    } else {
        let object = nif::rd32(e, refr.wrapping_add(BASE)).unwrap_or(0);
        if object != 0 && is_bound_object(e, object) {
            if let (Some(min), Some(max)) = (bound(e, object, axis), bound(e, object, axis + 3)) {
                centre = (min + max) as f32 * 0.5;
            }
        }
    }
    let position = super::ln_fn_math::position(e, refr)[axis as usize];
    let scale = nif::scale(e, refr);
    a.set_result(e, f64::from(position + centre * scale));
    handled(true)
}

// ---- materials ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetMaterialPropertyValue_Execute.
/// A geometry block's material value: trait 0-2 specular, 3-5 emissive (red, green, blue), 6
/// glossiness, 7 alpha, 8 the emit multiplier. Nothing for a higher trait, a block that is not
/// geometry or has no material.
fn get_material_property_value(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let name = block_name(e, &read, 0, "GetMaterialPropertyValue");
    let trait_id = read.int(1, 0);
    if trait_id > 8 {
        return handled(true);
    }
    let block = nif::ni_block(e, a.this_obj, &name);
    if block == 0 || !nif::is_geometry(e, block) {
        return handled(true);
    }
    let Some(material) = nif::ptr(e, block.wrapping_add(MATERIAL_PROPERTY)) else {
        return handled(true);
    };
    let at = match trait_id {
        0..=2 => SPECULAR + 4 * trait_id,
        3..=5 => EMISSIVE + 4 * (trait_id - 3),
        6 => GLOSSINESS,
        7 => ALPHA,
        _ => EMIT_MULT,
    };
    if let Some(v) = nif::rdf32(e, material.wrapping_add(at)) {
        a.set_result(e, f64::from(v));
    }
    handled(true)
}

/// `NiGeometry`'s shader property (type 3, +0xA8); `BSShaderProperty::shaderType` (+0x1C, the
/// word `00441110` reads: `BipedAnim::GetLightingProperty` `004AF290` compares it with 8-0xC),
/// `kType_PPLighting` (8) and `kType_NoLighting` (0x15) (JIP LN NVSE).
const SHADER_PROPERTY: u32 = 0xa8;
const SHADER_TYPE: u32 = 0x1c;
const PP_LIGHTING: u32 = 8;
const NO_LIGHTING: u32 = 0x15;
/// `BSShaderPPLightingProperty::textureSet` (+0xA4: its `LinkObject` `00B66780` links it);
/// `BSShaderTextureSet::textures` (six game strings of 8 bytes from +8: `BSShaderTextureSet::
/// LoadBinary` `00BA88D0`); `BSShaderNoLightingProperty::srcTexture` (+0x60, `00B6F8D0`);
/// `NiSourceTexture` (vtable `0109B9EC`) and its file name (fixed string +0x30, its
/// `LoadBinary` `00A60180`).
const TEXTURE_SET: u32 = 0xa4;
const TEXTURES: u32 = 0x08;
const SRC_TEXTURE: u32 = 0x60;
const NI_SOURCE_TEXTURE_VTBL: u32 = 0x0109_b9ec;
const DDS_PATH: u32 = 0x30;
/// The folder JIP strips from the paths.
const TEXTURES_FOLDER: &[u8] = b"data\\textures\\";

/// The game string's text length (u16, +4: the game's `String::Set` `004037F0` stores it through
/// `004039C0`; the buffer's size +6 through `00403A00`).
const STRING_LENGTH: u32 = 0x04;

/// `String::CStr` (JIP LN NVSE: `(m_data && m_dataLen) ? m_data : ""`) of the game string at
/// `at`: its text, `""` without a buffer or with length 0; an unreadable one counts as `""`.
fn string_text(e: &Engine, at: u32) -> Vec<u8> {
    match (
        nif::rd32(e, at),
        nif::rd16(e, at.wrapping_add(STRING_LENGTH)),
    ) {
        (Some(p), Some(length)) if p != 0 && length != 0 => nif::c_text(e, p).unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// The path with `data\textures\` (any case) taken off its front.
fn strip_textures(path: Vec<u8>) -> Vec<u8> {
    if crate::utility::str_begins_ci(&path, TEXTURES_FOLDER) != 0 {
        path[TEXTURES_FOLDER.len()..].to_vec()
    } else {
        path
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetBlockTextureSet_Execute.
/// A geometry block's six texture paths as an array (without `data\textures\` in front): its
/// lit shader's texture set, or an unlit shader's source texture first and five empty texts. No
/// array for any other shader, a lit one without a texture set, or an unlit one whose texture is
/// not a source texture or has no path.
fn get_block_texture_set(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let name = block_name(e, &read, 0, "GetBlockTextureSet");
    let block = nif::ni_block(e, a.this_obj, &name);
    if block == 0 || !nif::is_geometry(e, block) {
        return handled(true);
    }
    let Some(shader) = nif::ptr(e, block.wrapping_add(SHADER_PROPERTY)) else {
        return handled(true);
    };
    let paths: Vec<Vec<u8>> = match nif::rd32(e, shader.wrapping_add(SHADER_TYPE)) {
        Some(PP_LIGHTING) => {
            let Some(set) = nif::ptr(e, shader.wrapping_add(TEXTURE_SET)) else {
                return handled(true);
            };
            (0..6)
                .map(|i| strip_textures(string_text(e, set.wrapping_add(TEXTURES + 8 * i))))
                .collect()
        }
        Some(NO_LIGHTING) => {
            let Some(texture) = nif::ptr(e, shader.wrapping_add(SRC_TEXTURE)) else {
                return handled(true);
            };
            if nif::rd32(e, texture) != Some(NI_SOURCE_TEXTURE_VTBL) {
                return handled(true);
            }
            let Some(path) = nif::ptr(e, texture.wrapping_add(DDS_PATH)) else {
                return handled(true);
            };
            let mut paths = vec![strip_textures(nif::c_text(e, path).unwrap_or_default())];
            paths.resize(6, Vec::new());
            paths
        }
        _ => return handled(true),
    };
    let elements: Vec<Element> = paths.into_iter().map(Element::Str).collect();
    let id = create_array(e, a.script, &elements);
    a.set_result(e, f64::from(id));
    handled(true)
}

// ---- a block's transform and flags ----

/// The block a `GetNifBlock*` command names: argument 0 the name, `pc` the first/third person
/// argument's index.
fn named_block(e: &Engine, a: &CommandArgs, read: &Read, pc: usize, command: &str) -> u32 {
    let name = block_name(e, read, 0, command);
    nif::nif_block(e, a.this_obj, read.int(pc, 0), &name)
}

/// The block's translation: world (+0x8C) or local (+0x58).
fn translation(e: &Engine, block: u32, world: bool) -> Option<[f32; 3]> {
    let at = if world {
        nif::WORLD_TRANSLATE
    } else {
        nif::LOCAL_TRANSLATE
    };
    nif::floats3(e, block.wrapping_add(at))
}

/// The block's angles in degrees (each float times `Flt180dPI`): `mode` bit 1 the world rotation
/// (+0x68) else the local (+0x34); bit 2 `ToEulerPRYInv` else `ToEulerPRY`.
fn rotation(e: &Engine, block: u32, mode: u32) -> Option<[f32; 3]> {
    let at = if mode & 1 != 0 {
        nif::WORLD_ROTATE
    } else {
        nif::LOCAL_ROTATE
    };
    let m = nif::matrix(e, block.wrapping_add(at))?;
    let angles = if mode & 2 != 0 {
        nif::to_euler_pry_inv(&m)
    } else {
        nif::to_euler_pry(&m)
    };
    Some(angles.map(|v| v * nif::FLT_180_D_PI))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetNifBlockTranslation_Execute.
/// The block's translation (world with the second argument) as an array of three.
fn get_nif_block_translation(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let block = named_block(e, &a, &read, 2, "GetNifBlockTranslation");
    if block != 0 {
        if let Some(v) = translation(e, block, read.int(1, 0) != 0) {
            array_of_3(e, &a, v);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetNifBlockRotation_Execute.
/// The block's angles in degrees as an array of three ([`rotation`] by the second argument).
fn get_nif_block_rotation(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let block = named_block(e, &a, &read, 2, "GetNifBlockRotation");
    if block != 0 {
        if let Some(v) = rotation(e, block, read.int(1, 0)) {
            array_of_3(e, &a, v);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetNifBlockScale_Execute.
/// The block's local scale (+0x64).
fn get_nif_block_scale(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let block = named_block(e, &a, &read, 1, "GetNifBlockScale");
    if block != 0 {
        if let Some(v) = nif::rdf32(e, block.wrapping_add(nif::LOCAL_SCALE)) {
            a.set_result(e, f64::from(v));
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetNifBlockFlag_Execute.
/// 1 when flag bit `flagID` (0-31) of the block's flags (+0x30) is set.
fn get_nif_block_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let flag = read.int(1, 0);
    if flag > 31 {
        return handled(true);
    }
    let block = named_block(e, &a, &read, 2, "GetNifBlockFlag");
    if block != 0
        && nif::rd32(e, block.wrapping_add(nif::FLAGS)).is_some_and(|f| f & (1 << flag) != 0)
    {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetNifBlockTranslationAlt_Execute.
/// [`get_nif_block_translation`] into three script variables; result 1.
fn get_nif_block_translation_alt(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let Some(vars) = var_args(&read, 1) else {
        return handled(true);
    };
    let block = named_block(e, &a, &read, 5, "GetNifBlockTranslationAlt");
    if block != 0 {
        if let Some(v) = translation(e, block, read.int(4, 0) != 0) {
            set_vars(e, vars, v);
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetNifBlockRotationAlt_Execute.
/// [`get_nif_block_rotation`] into three script variables; result 1.
fn get_nif_block_rotation_alt(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let Some(vars) = var_args(&read, 1) else {
        return handled(true);
    };
    let block = named_block(e, &a, &read, 5, "GetNifBlockRotationAlt");
    if block != 0 {
        if let Some(v) = rotation(e, block, read.int(4, 0)) {
            set_vars(e, vars, v);
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

// ---- parents and children ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetNifBlockParentNodes_Execute.
/// The names of the block's parents, nearest first, up to and including the root, as an array
/// (an empty one for the root itself; `NULL` for a parent without a name, as JIP's `GetName`).
/// The root: the loaded one, but for the player in first person (flag clear) without the odd
/// second argument its first person node.
fn get_nif_block_parent_nodes(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let name = block_name(e, &read, 0, "GetNifBlockParentNodes");
    let refr = a.this_obj;
    let mut root = nif::ref_ni_node(e, refr);
    if nif::is_player(e, refr) && read.int(1, 0) & 1 == 0 && !nif::is_3rd_person(e, refr) {
        root = nif::node_1st_person(e, refr);
    }
    if root == 0 {
        return handled(true);
    }
    let mut block = nif::get_block(e, root, &name);
    if block == 0 {
        return handled(true);
    }
    let mut names = Vec::new();
    let mut walk = nif::Walk::new();
    while block != root && walk.step() {
        block = nif::rd32(e, block.wrapping_add(nif::PARENT)).unwrap_or(0);
        if block == 0 {
            break;
        }
        names.push(Element::Str(nif::name_text(e, block)));
    }
    let id = create_array(e, a.script, &names);
    a.set_result(e, f64::from(id));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h: GetChildBlocks.
/// The node's name, then each child in order: a node's own walk, another block's name. `depth`
/// is the level of `node`'s children below the walk's start.
fn child_blocks(e: &Engine, node: u32, out: &mut Vec<Element>, depth: u32, walk: &mut nif::Walk) {
    out.push(Element::Str(nif::name_text(e, node)));
    if nif::too_deep(depth) {
        return;
    }
    nif::each_child(e, node, walk, |walk, child| {
        if nif::is_node(e, child) {
            child_blocks(e, child, out, depth + 1, walk);
        } else {
            out.push(Element::Str(nif::name_text(e, child)));
        }
        true
    });
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetChildBlocks_Execute.
/// The names below a node (the root for no name), as an array: every block below it in walk
/// order (`NULL` for one without a name), or with the third argument only its children's names.
/// No array when there are none, or when the block is not a node.
fn get_child_blocks(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let node = named_block(e, &a, &read, 1, "GetChildBlocks");
    if node == 0 || !nif::is_node(e, node) {
        return handled(true);
    }
    let mut names = Vec::new();
    if read.int(2, 0) != 0 {
        names.push(Element::Number(0.0));
        for child in nif::children(e, node) {
            names.push(Element::Str(nif::name_text(e, child)));
        }
    } else {
        child_blocks(e, node, &mut names, 1, &mut nif::Walk::new());
    }
    if names.len() > 1 {
        let id = create_array(e, a.script, &names[1..]);
        a.set_result(e, f64::from(id));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// GetCollisionNodes.
/// The node's name when it has a collision object with a world object, then each child node's
/// walk in order. `depth` is the level of `node`'s children below the walk's start.
fn collision_nodes(
    e: &Engine,
    node: u32,
    out: &mut Vec<Element>,
    depth: u32,
    walk: &mut nif::Walk,
) {
    if nif::ptr(e, node.wrapping_add(nif::COLLISION_OBJECT))
        .and_then(|c| nif::ptr(e, c.wrapping_add(WORLD_OBJECT)))
        .is_some()
    {
        out.push(Element::Str(nif::name_text(e, node)));
    }
    if nif::too_deep(depth) {
        return;
    }
    nif::each_child(e, node, walk, |walk, child| {
        if nif::is_node(e, child) {
            collision_nodes(e, child, out, depth + 1, walk);
        }
        true
    });
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetCollisionNodes_Execute.
/// The names of the loaded root's nodes that have collision, as an array; none when there are
/// none.
fn get_collision_nodes(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if a.this_obj == 0 {
        return handled(true);
    }
    let root = nif::ref_ni_node(e, a.this_obj);
    if root == 0 {
        return handled(true);
    }
    let mut names = Vec::new();
    collision_nodes(e, root, &mut names, 1, &mut nif::Walk::new());
    if !names.is_empty() {
        let id = create_array(e, a.script, &names);
        a.set_result(e, f64::from(id));
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_misc_ref_nodes_tests.rs"]
mod tests;
