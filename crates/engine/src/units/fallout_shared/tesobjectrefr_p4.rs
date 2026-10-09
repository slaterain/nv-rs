//! `fallout shared/tesobjectrefr.cpp` (Xbox PDB source unit), part 4: its functions from `0056ea40` up to
//! (not including) `00576870` in FalloutNV.exe 1.4.0.525, translated
//! (docs/ENGINE_CRATE.md). The unit's shared layouts and helpers are in
//! [`super::tesobjectrefr`]; anything public there may be used here.
//!
//! # Where this file stops
//!
//! The first batch holds the 40 open functions from `0056ea40` to
//! `00572230` (the Havok collision builders of a reference, its orientation
//! and bound getters, `Set3D` and its helpers, the model getters and
//! `RemoveWeapon`, `MarkAsPickedUp`). The next session continues with the
//! next open function after `00572230`, which is `00572270`
//! (`MarkAsDeleted`).
//!
//! # Conventions of this part
//!
//! The private constants of the main file are not visible here, so the few
//! small accessors the functions lean on are declared below and called by
//! address exactly as the game calls them (`005d43c0` is `this + 0x44`, the
//! embedded `ExtraDataList`; `007af430` reads the base object at `+0x20`;
//! `0043fcd0` is the reference's 3D, the node held at `+0x14` of its loaded
//! data, or the thread's override). Several callees take a pointer to an
//! x86 stack local; here such locals live in one zeroed block ([`Frame`])
//! whose `ebp`-relative offsets are the disassembly's, so each address in
//! a comment can be checked against the code.
//!
//! A pushed argument the callee never reads (`PUSH 0` before a callee that
//! takes no argument) is still passed, as the callers do.

#[allow(unused_imports)]
use super::tesobjectrefr::*;
#[allow(unused_imports)]
use crate::prelude::*;

/// `ExtraDataList` of a reference: `this + 0x44` (`005d43c0`).
const GET_EXTRA_LIST: u32 = 0x005d_43c0;
/// Base object of a reference, the word at `+0x20` (`007af430`).
const GET_BASE_FORM: u32 = 0x007a_f430;
/// `cFormType`, the byte at `+4` of a form (`00401170`).
const FORM_TYPE: u32 = 0x0040_1170;
/// Parent cell of a reference, the word at `+0x40` (`008d6f30`).
const GET_PARENT_CELL: u32 = 0x008d_6f30;
/// The reference's 3D (`0043fcd0`).
const GET_3D: u32 = 0x0043_fcd0;
/// `IsDeleted` test on a form (flag `0x20`, `00440d80`).
const FORM_IS_DELETED: u32 = 0x0044_0d80;
/// `IsDisabled` test on a form (flag `0x800`, `00440da0`).
const FORM_IS_DISABLED: u32 = 0x0044_0da0;
/// The first dword of its object (`00559450`, used on `NiPointer` slots and
/// list nodes).
const READ_FIRST_DWORD: u32 = 0x0055_9450;
/// The word at `+4` of a list node (`00726070`), also read from the
/// primitive extra for its shape type.
const LIST_NEXT: u32 = 0x0072_6070;
/// Constructor of a `NiPoint3`/`NiMatrix3` local that does nothing and
/// returns `this` (`006815c0`).
const NOTHING_CONSTRUCTOR: u32 = 0x0068_15c0;
/// The word at `+0x18` of a node, its parent (`009611e0`).
const NODE_PARENT: u32 = 0x0096_11e0;
/// The word at `+0x68` of an actor (its process, `008d8520`).
const GET_PROCESS: u32 = 0x008d_8520;
/// `operator new` of the `Ni` allocator (`00aa13e0`).
const NI_OPERATOR_NEW: u32 = 0x00aa_13e0;
/// `operator delete` of the `Ni` allocator (`00aa1460`, pointer and size).
const NI_OPERATOR_DELETE: u32 = 0x00aa_1460;
/// Aligned allocation of the Havok objects (`0056d280`, cdecl).
const HAVOK_ALLOCATE: u32 = 0x0056_d280;
/// `ExtraDataList::GetPrimitive` (`0041fbe0`).
const GET_PRIMITIVE: u32 = 0x0041_fbe0;
/// `TESObjectREFR::GetScale` (`00567400`), the scale in `ST0`.
const GET_SCALE: u32 = 0x0056_7400;
/// `ModelLoader::CancelReference` (`00445570`), on the model loader.
const CANCEL_REFERENCE: u32 = 0x0044_5570;
/// `NiObjectNET::GetExtraData` (`00a5bdd0`).
const NODE_GET_EXTRA_DATA: u32 = 0x00a5_bdd0;
/// `NiObjectNET::AddExtraData_ov2` (`00a5bca0`).
const NODE_ADD_EXTRA_DATA: u32 = 0x00a5_bca0;
/// Update of a node with an `NiUpdateData` (`00a59c60`): the node's virtual
/// at `+0xa4` with the data and 0, then the virtual at `+0xfc` of its parent
/// (`+0x18`) when it has one.
const NODE_UPDATE: u32 = 0x00a5_9c60;
/// `NiMatrix3::FromEulerAnglesXYZ` (`00a59540`).
const MATRIX_FROM_EULER: u32 = 0x00a5_9540;
/// `NiMatrix3::ToEulerAnglesXYZ` (`00a592c0`).
const MATRIX_TO_EULER: u32 = 0x00a5_92c0;
/// `NiUpdateData` constructor (`0043d410`; 0xC bytes in the Xbox PDB):
/// `fTime`, then `bUpdateControllers` and `bParallelUpdate`; the other flags
/// are cleared.
const UPDATE_DATA_CONSTRUCTOR: u32 = 0x0043_d410;
/// Constructor of the local pair (a `0x24`-byte matrix at +0 and the vector
/// after it) the Havok builders keep at `ebp-0xc0` (`00476a80`; it runs the
/// empty constructor `006815c0` on both parts).
const MATRIX_CONSTRUCTOR: u32 = 0x0047_6a80;
/// Constructor of the 16-byte local `00561500` fills (`006240d0`; it runs
/// the empty constructor `006815c0`).
const QUATERNION_CONSTRUCTOR: u32 = 0x0062_40d0;
/// Converts the `NiPoint3` at the source into the four floats at the
/// destination (`004a3e00`, cdecl: destination, source): each component is
/// scaled by `004a3e90` (the factor at `011c582c`), the fourth is 0.0.
const CONVERT_VECTOR: u32 = 0x004a_3e00;
/// Copies the four components of the source into the destination through
/// their accessors (`00561500`, cdecl: destination, source).
const CONVERT_ROTATION: u32 = 0x0056_1500;
/// The model loader pointer (`ModelLoader::` methods take it as `this`).
const GLOBAL_MODEL_LOADER: u32 = 0x011c_3b3c;
/// `PlayerCharacter *`.
const GLOBAL_PLAYER: u32 = 0x011d_ea3c;
/// The object `TES::GetCellPriority` and the nav-mesh helpers use as `this`.
const GLOBAL_TES: u32 = 0x011d_ea10;
/// The object whose `0042ce10` flag says "loading".
const GLOBAL_LOADING_OBJECT: u32 = 0x011d_df38;
/// `ProcessLists`.
const OBJECT_PROCESS_LISTS: u32 = 0x011e_0e80;
/// `TESDataHandler *`.
const GLOBAL_DATA_HANDLER: u32 = 0x011c_3f2c;
/// `TESSaveLoadGame *`.
const GLOBAL_SAVE_LOAD: u32 = 0x011d_e45c;
/// The zero `NiPoint3` the game keeps for "no bound".
const ZERO_VECTOR: u32 = 0x011f_426c;
/// Scratch `NiPoint3` the min-corner getter fills (`011ca3b8`).
const SCRATCH_BOUND_MIN: u32 = 0x011c_a3b8;
/// Scratch `NiPoint3` the max-corner getters fill (`011ca3d4`).
const SCRATCH_BOUND_MAX: u32 = 0x011c_a3d4;

/// One zeroed block standing for the stack frame of a function that keeps
/// many locals and passes some of them by address. Offsets are `ebp`
/// relative, as the disassembly writes them.
struct Frame(u32);

/// Bytes of the block, and where `ebp` sits in it.
const FRAME_SIZE: u32 = 0x300;
const FRAME_EBP: i32 = 0x2c0;

impl Frame {
    /// The address of the local at `ebp + offset`.
    fn at(&self, offset: i32) -> u32 {
        (self.0 as i32 + FRAME_EBP + offset) as u32
    }
}

/// Runs `f` with a zeroed [`Frame`].
fn with_frame<R>(e: &mut Engine, f: impl FnOnce(&mut Engine, &Frame) -> R) -> R {
    e.with_stack(FRAME_SIZE, |e, block| f(e, &Frame(block.addr())))
}

fn extra_list(e: &mut Engine, refr: u32) -> u32 {
    e.call(GET_EXTRA_LIST, &args![refr]).u32()
}

fn base_form(e: &mut Engine, refr: u32) -> u32 {
    e.call(GET_BASE_FORM, &args![refr]).u32()
}

fn form_type(e: &mut Engine, form: u32) -> u32 {
    e.call(FORM_TYPE, &args![form]).u32()
}

fn parent_cell(e: &mut Engine, refr: u32) -> u32 {
    e.call(GET_PARENT_CELL, &args![refr]).u32()
}

fn get_3d(e: &mut Engine, refr: u32) -> u32 {
    e.call(GET_3D, &args![refr]).u32()
}

/// `IsActor` (virtual at +0x100).
fn is_actor(e: &mut Engine, refr: u32) -> bool {
    e.vcall(refr, 0x100, &args![]).bool()
}

/// The reference's node (virtual at +0x1d0).
fn node_of(e: &mut Engine, refr: u32) -> u32 {
    e.vcall(refr, 0x1d0, &args![]).u32()
}

/// Copies `words` dwords from `from` to `to`.
fn copy_words(e: &mut Engine, to: u32, from: u32, words: u32) {
    let bytes = e.mem.bytes(from, words * 4);
    e.mem.write(to, &bytes);
}

/// The model loader (`this` of the `ModelLoader::` methods).
fn model_loader(e: &Engine) -> u32 {
    e.global::<u32>(GLOBAL_MODEL_LOADER)
}

// Translated from 0056ea40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructor wrapper of a Havok glue object (the linker placed it here):
/// runs the base constructor `0056e8d0` on `this` and returns `this`.
pub fn fn_0056ea40(e: &mut Engine, this: Ptr) -> Ptr {
    e.call(0x0056_e8d0, &args![this]);
    this
}

// Translated from 0056ea60 (decompiled, FalloutNV.exe 1.4.0.525)
/// `bhkSphereShape::_scalar_deleting_destructor_` (Xbox PDB): runs
/// `~bhkSphereShape` (`00ca0660`) and, when bit 0 of `flags` is set, frees
/// the 0x14-byte object with `operator delete` (`00aa1460`). Returns `this`.
pub fn bhk_sphere_shape_scalar_deleting_destructor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(0x00ca_0660, &args![this]);
    if flags & 1 != 0 {
        e.call(NI_OPERATOR_DELETE, &args![this, 0x14u32]);
    }
    this
}

// Translated from 0056ea90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Destructor wrapper of a Havok glue object: runs `0056d730` on `this`.
pub fn fn_0056ea90(e: &mut Engine, this: Ptr) {
    e.call(0x0056_d730, &args![this]);
}

// Translated from 0056eab0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Builds the Havok collision of a reference from its primitive extra
/// (`ExtraPrimitive`): a box (primitive type 1; type 3 is a box whose second
/// half extent is replaced by the constant at `01013ea4`) or a sphere (type 2)
/// shape, a `bhkRigidBody` positioned on the reference node, a `BSXFlags`
/// extra on the node and the `bhkNiCollisionObject` that ties the body to
/// the node, then adds the body (virtual `+0x9c`) to the world of the parent
/// cell: what `004543c0` gives for it (the object held in the extra data of
/// an interior cell, else the shared one at `011ca0d8`). Does
/// nothing without a primitive or a world.
///
/// Before building anything it calls `00567490(this, 1.0)`, sets the scale
/// of its node (`00440490`, the float at `+0x64`) to 1.0 and updates the node (update data at
/// time 0.0, `00a59c60`).
///
/// The `0xE0`-byte `bhkRigidBodyCinfo` lives at `ebp-0x210`. The body gets
/// its position and rotation from the node and the reference (the body's
/// virtuals `+0xdc` and `+0xe0`, fed by `004a3e00` and `00561500`). The
/// pair the function assembles at `ebp-0xc0` (orientation, position) is
/// never read again; the calls that build it are kept.
///
/// The compiler's exception-unwinding frame is not translated.
pub fn fn_0056eab0(e: &mut Engine, this: Ptr<TESObjectREFR>) {
    let me = this.addr();
    let extra = extra_list(e, me);
    let primitive = e.call(GET_PRIMITIVE, &args![extra]).u32();
    let cell = parent_cell(e, me);
    let world = if cell != 0 {
        let cell = parent_cell(e, me);
        e.call(0x0045_43c0, &args![cell]).u32()
    } else {
        0
    };
    if world == 0 || primitive == 0 {
        return;
    }
    with_frame(e, |e, f| {
        let mut shape = 0u32;
        // 0056ded0: constructor of the unused local at ebp-0x70
        e.call(0x0056_ded0, &args![f.at(-0x70)]);
        e.call(NOTHING_CONSTRUCTOR, &args![f.at(-0x80)]);
        // 00567490(this, 1.0f) and node->00440490(1.0f)
        e.call(0x0056_7490, &args![me, 1.0f32]);
        let node = node_of(e, me);
        e.call(0x0044_0490, &args![node, 1.0f32]);
        e.call(
            UPDATE_DATA_CONSTRUCTOR,
            &args![f.at(-0x8c), 0.0f32, 0u32, 0u32],
        );
        let node = node_of(e, me);
        e.call(NODE_UPDATE, &args![node, f.at(-0x8c)]);
        e.call(MATRIX_CONSTRUCTOR, &args![f.at(-0xc0)]);
        let orientation = tes_object_refr_get_orientation(e, this, Ptr::new(f.at(-0xe4))).addr();
        copy_words(e, f.at(-0xc0), orientation, 9);
        let position = e.vcall(me, 0x1f4, &args![]).u32();
        copy_words(e, f.at(-0x9c), position, 3);
        e.mem.set_f32(f.at(-0x90), 1.0);
        match e.call(LIST_NEXT, &args![primitive]).u32() {
            1 => {
                // ExtraPrimitive bounds (00413fc0 copies the three floats at +0x18)
                let half = e.call(0x0041_3fc0, &args![primitive, f.at(-0x10c)]).u32();
                e.call(CONVERT_VECTOR, &args![f.at(-0x80), half]);
                let object = e.call(NI_OPERATOR_NEW, &args![0x14u32]).u32();
                shape = if object != 0 {
                    e.call(0x0056_e610, &args![object, f.at(-0x80)]).u32()
                } else {
                    0
                };
            }
            2 => {
                let object = e.call(NI_OPERATOR_NEW, &args![0x14u32]).u32();
                shape = if object != 0 {
                    let half = e.call(0x0041_3fc0, &args![primitive, f.at(-0x128)]).u32();
                    let x = e.mem.f32(half);
                    // 004a3e90 scales a length into Havok units (cdecl, float in ST0)
                    let radius = e.call(0x004a_3e90, &args![x]).f32();
                    e.call(0x0056_e950, &args![object, radius, 0u32]).u32()
                } else {
                    0
                };
            }
            3 => {
                e.call(0x0041_3fc0, &args![primitive, f.at(-0xf4)]);
                let fourth: f32 = e.global(0x0101_3ea4);
                e.mem.set_f32(f.at(-0xf0), fourth);
                e.call(CONVERT_VECTOR, &args![f.at(-0x80), f.at(-0xf4)]);
                let object = e.call(NI_OPERATOR_NEW, &args![0x14u32]).u32();
                shape = if object != 0 {
                    e.call(0x0056_e610, &args![object, f.at(-0x80)]).u32()
                } else {
                    0
                };
            }
            _ => {}
        }
        // the bhkRigidBodyCinfo
        let cinfo = f.at(-0x210);
        e.call(0x00c8_f510, &args![cinfo]);
        fn_0056f110(e, Ptr::new(cinfo), shape);
        let extra = extra_list(e, me);
        let collision_data = e.call(0x0042_11a0, &args![extra]).u32();
        let layer = if collision_data != 0 {
            e.call(READ_FIRST_DWORD, &args![collision_data]).u32()
        } else {
            3
        };
        fn_0056f0f0(e, Ptr::new(cinfo), 0x3_0000 | layer);
        e.call(0x0056_d360, &args![cinfo, 5u32]);
        // BSXFlags extra on the node
        let object = e.call(NI_OPERATOR_NEW, &args![0x10u32]).u32();
        let flags = if object != 0 {
            e.call(0x00c4_2f70, &args![object]).u32()
        } else {
            0
        };
        e.call(0x0051_9230, &args![flags, 2u32, 1u32]);
        let node = node_of(e, me);
        e.call(NODE_ADD_EXTRA_DATA, &args![node, flags]);
        // the body
        let object = e.call(NI_OPERATOR_NEW, &args![0x1cu32]).u32();
        let body = if object != 0 {
            e.call(0x0056_d380, &args![object, cinfo]).u32()
        } else {
            0
        };
        e.call(NOTHING_CONSTRUCTOR, &args![f.at(-0x250)]);
        e.call(QUATERNION_CONSTRUCTOR, &args![f.at(-0x260)]);
        e.call(NOTHING_CONSTRUCTOR, &args![f.at(-0x270)]);
        let node = node_of(e, me);
        e.call(0x0056_d230, &args![node, f.at(-0x270)]);
        let position = e.vcall(me, 0x1f4, &args![]).u32();
        e.call(CONVERT_VECTOR, &args![f.at(-0x250), position]);
        e.call(CONVERT_ROTATION, &args![f.at(-0x260), f.at(-0x270)]);
        e.vcall(body, 0xdc, &args![f.at(-0x250)]);
        e.vcall(body, 0xe0, &args![f.at(-0x260)]);
        let node = node_of(e, me);
        e.call(0x00c8_5c10, &args![body, node, 0u32]);
        e.vcall(body, 0x9c, &args![world]);
        // the collision object tying the body to the node
        let object = e.call(NI_OPERATOR_NEW, &args![0x14u32]).u32();
        let collision_object = if object != 0 {
            let node = node_of(e, me);
            e.call(0x0056_d580, &args![object, node]).u32()
        } else {
            0
        };
        e.call(0x00c6_b980, &args![collision_object, body]);
        e.call(0x0056_d730, &args![cinfo]);
    });
}

// Translated from 0056f0f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the collision filter info of a `bhkRigidBodyCinfo` (`this`): the
/// word at `+0` and its `hk` copy at `+0x20`.
pub fn fn_0056f0f0(e: &mut Engine, this: Ptr, filter_info: u32) {
    e.mem.set_u32(this.addr(), filter_info);
    e.mem.set_u32(this.addr() + 0x20, filter_info);
}

// Translated from 0056f110 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the shape of a `bhkRigidBodyCinfo` (`this`): the word at `+4`
/// gets what `004ae750` returns for `shape` (0 for a null shape, else what
/// `00620b80` finds in it), and the `hk` copy at `+0x24` the same.
pub fn fn_0056f110(e: &mut Engine, this: Ptr, shape: u32) {
    let havok_shape = e.call(0x004a_e750, &args![shape]).u32();
    e.mem.set_u32(this.addr() + 4, havok_shape);
    let copy = e.mem.u32(this.addr() + 4);
    e.mem.set_u32(this.addr() + 0x24, copy);
}

// Translated from 0056f140 (decompiled, FalloutNV.exe 1.4.0.525)
/// Builds the Havok collision of a reference whose primitive extra describes
/// a box and registers it (the bounding-volume variant of `0056eab0`): the
/// box shape wrapped in a `0x1c`-byte bounding-volume shape (`00c9d990`
/// over the box and a `0x10`-byte companion built by `0056f690`), a body
/// with the fixed filter `0x50015`, `BSXFlags` on the node and the
/// collision object; the body is added to the world of the parent cell
/// through `0056d2c0`. Does nothing without a world. It starts as `0056eab0`
/// does (`00567490(this, 1.0)`, node scale 1.0, node update).
///
/// The `bhkRigidBodyCinfo` lives at `ebp-0x200`; its position (`+0x30`) and
/// rotation (`+0x40`) are set through `0056d300` and `0056d320`.
///
/// The compiler's exception-unwinding frame is not translated.
pub fn fn_0056f140(e: &mut Engine, this: Ptr<TESObjectREFR>) {
    let me = this.addr();
    let extra = extra_list(e, me);
    let primitive = e.call(GET_PRIMITIVE, &args![extra]).u32();
    let cell = parent_cell(e, me);
    let world = if cell != 0 {
        let cell = parent_cell(e, me);
        e.call(0x0045_43c0, &args![cell]).u32()
    } else {
        0
    };
    if world == 0 {
        return;
    }
    with_frame(e, |e, f| {
        e.call(0x0056_ded0, &args![f.at(-0x70)]);
        e.call(NOTHING_CONSTRUCTOR, &args![f.at(-0x80)]);
        e.call(0x0056_7490, &args![me, 1.0f32]);
        let node = node_of(e, me);
        e.call(0x0044_0490, &args![node, 1.0f32]);
        e.call(
            UPDATE_DATA_CONSTRUCTOR,
            &args![f.at(-0x8c), 0.0f32, 0u32, 0u32],
        );
        let node = node_of(e, me);
        e.call(NODE_UPDATE, &args![node, f.at(-0x8c)]);
        e.call(MATRIX_CONSTRUCTOR, &args![f.at(-0xc0)]);
        let orientation = tes_object_refr_get_orientation(e, this, Ptr::new(f.at(-0xe4))).addr();
        copy_words(e, f.at(-0xc0), orientation, 9);
        let position = e.vcall(me, 0x1f4, &args![]).u32();
        copy_words(e, f.at(-0x9c), position, 3);
        e.mem.set_f32(f.at(-0x90), 1.0);
        // the dead store of 0x15 at ebp-0xe8 is not kept
        let half = e.call(0x0041_3fc0, &args![primitive, f.at(-0xf4)]).u32();
        e.call(CONVERT_VECTOR, &args![f.at(-0x80), half]);
        // the box
        let object = e.call(HAVOK_ALLOCATE, &args![0x30u32]).u32();
        let box_shape = if object != 0 {
            let radius: f32 = e.global(0x011b_153c);
            e.call(0x00c9_ddb0, &args![object, f.at(-0x80), radius])
                .u32()
        } else {
            0
        };
        // its 0x10-byte companion
        let object = e.call(HAVOK_ALLOCATE, &args![0x10u32]).u32();
        let companion = if object != 0 {
            fn_0056f690(e, Ptr::new(object)).addr()
        } else {
            0
        };
        // the wrapping shape
        let object = e.call(HAVOK_ALLOCATE, &args![0x1cu32]).u32();
        let wrapped = if object != 0 {
            e.call(0x00c9_d990, &args![object, box_shape, companion])
                .u32()
        } else {
            0
        };
        let cinfo = f.at(-0x200);
        e.call(0x00c8_f510, &args![cinfo]);
        e.mem.set_u32(cinfo + 4, wrapped);
        e.call(0x0056_d360, &args![cinfo, 5u32]);
        e.call(NOTHING_CONSTRUCTOR, &args![f.at(-0x210)]);
        e.call(NOTHING_CONSTRUCTOR, &args![f.at(-0x220)]);
        e.call(QUATERNION_CONSTRUCTOR, &args![f.at(-0x230)]);
        let node = node_of(e, me);
        e.call(0x0056_d230, &args![node, f.at(-0x220)]);
        let position = e.vcall(me, 0x1f4, &args![]).u32();
        e.call(CONVERT_VECTOR, &args![f.at(-0x210), position]);
        e.call(CONVERT_ROTATION, &args![f.at(-0x230), f.at(-0x220)]);
        e.call(0x0056_d300, &args![cinfo, f.at(-0x210)]);
        e.call(0x0056_d320, &args![cinfo, f.at(-0x230)]);
        // the collision filter info 0x50000 | 0x15
        e.mem.set_u32(cinfo, 0x5_0000 | 0x15);
        // the body
        let object = e.call(NI_OPERATOR_NEW, &args![0x1cu32]).u32();
        let body = if object != 0 {
            e.call(0x0056_d380, &args![object, cinfo]).u32()
        } else {
            0
        };
        // the collision object
        let object = e.call(NI_OPERATOR_NEW, &args![0x14u32]).u32();
        let collision_object = if object != 0 {
            let node = node_of(e, me);
            e.call(0x0056_d580, &args![object, node]).u32()
        } else {
            0
        };
        // BSXFlags on the node
        let object = e.call(NI_OPERATOR_NEW, &args![0x10u32]).u32();
        let flags = if object != 0 {
            e.call(0x00c4_2f70, &args![object]).u32()
        } else {
            0
        };
        e.call(0x0051_9230, &args![flags, 2u32, 1u32]);
        let node = node_of(e, me);
        e.call(NODE_ADD_EXTRA_DATA, &args![node, flags]);
        e.call(0x00c6_b980, &args![collision_object, body]);
        let node = node_of(e, me);
        e.call(0x00c8_5c10, &args![body, node, 0u32]);
        e.call(0x0056_d2c0, &args![world, body]);
        e.call(0x0056_d730, &args![cinfo]);
    });
}

// Translated from 0056f690 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructor of the 0x10-byte Havok companion object of `0056f140`: runs
/// the base constructor `0056d610` and sets its own vtable (`01030dc0`).
/// Returns `this`.
pub fn fn_0056f690(e: &mut Engine, this: Ptr) -> Ptr {
    e.call(0x0056_d610, &args![this]);
    e.mem.set_u32(this.addr(), 0x0103_0dc0);
    this
}

// Translated from 0056f6b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Asks the base object of the reference, when it has one, its virtual at
/// `+0xa0` and returns the answer as a bool (false without a base object).
pub fn fn_0056f6b0(e: &mut Engine, this: Ptr<TESObjectREFR>) -> bool {
    if base_form(e, this.addr()) == 0 {
        return false;
    }
    let base = base_form(e, this.addr());
    e.vcall(base, 0xa0, &args![]).bool()
}

// Translated from 0056f700 (decompiled, FalloutNV.exe 1.4.0.525)
/// Brings the 3D of a reference up to date after a load or enable: a
/// reference that has no 3D (or `005651e0` says is excluded), is not
/// already known to the model loader (`00445750`), is not deleted and not
/// held back by its base object (an `0x1e`-type base the loader flag
/// `00444ed0` says is being saved is exempt) is queued with the model loader
/// (`00444850`) at the priority its cell gets from `TES::GetCellPriority`
/// (`00458be0`) when it is enabled; a disabled one gets its script action
/// list initialised and flag `0x1000` set unless the loading object says
/// loading. Then, with a 3D, the node is updated with update data at time 0.0 (`00a59c60`), an actor's
/// dismembered limbs are rebuilt, the 3D virtual at `+0x1c4` runs and the
/// saved Havok data is restored; without a 3D, a primitive extra makes it
/// run the virtual at `+0x1c4`.
pub fn fn_0056f700(e: &mut Engine, this: Ptr<TESObjectREFR>) {
    let me = this.addr();
    let mut saving_other = false;
    if e.call(0x0044_4ed0, &args![me]).bool() && base_form(e, me) != 0 {
        let base = base_form(e, me);
        if form_type(e, base) != 0x1e {
            saving_other = true;
        }
    }
    let node = get_3d(e, me);
    if (node == 0 || e.call(0x0056_51e0, &args![me]).bool())
        && !e.call(0x0044_5750, &args![model_loader(e), me]).bool()
        && !e.call(FORM_IS_DELETED, &args![me]).bool()
        && !saving_other
    {
        if !e.call(FORM_IS_DISABLED, &args![me]).bool() {
            if is_actor(e, me) {
                let actor = me;
                if actor != 0 {
                    e.vcall(actor, 0x240, &args![]);
                    e.call(0x0096_f400, &args![OBJECT_PROCESS_LISTS, actor]);
                }
            }
            let cell = parent_cell(e, me);
            let tes = e.global::<u32>(GLOBAL_TES);
            let priority = e.call(0x0045_8be0, &args![tes, cell, 0u32, 1u32]).u32();
            e.call(0x0044_4850, &args![model_loader(e), me, priority]);
        } else {
            let loading = e.global::<u32>(GLOBAL_LOADING_OBJECT);
            if !e.call(0x0042_ce10, &args![loading]).bool() {
                let extra = extra_list(e, me);
                e.call(0x005a_c190, &args![me, extra]);
                let extra = extra_list(e, me);
                e.call(0x005a_c750, &args![me, extra, 0x1000u32]);
            }
        }
    }
    if get_3d(e, me) != 0 {
        // the NiUpdateData local at ebp-0x18 (time 0.0)
        e.with_stack(0xc, |e, point| {
            e.call(UPDATE_DATA_CONSTRUCTOR, &args![point, 0.0f32, 0u32, 0u32]);
            let node = get_3d(e, me);
            e.call(NODE_UPDATE, &args![node, point]);
            let actor = is_actor(e, me);
            if actor {
                e.call(0x008b_6ae0, &args![me]);
                let extra = extra_list(e, me);
                e.call(0x0042_2bc0, &args![extra, me]);
            }
            e.vcall(me, 0x1c4, &args![]);
            if actor {
                e.call(0x008a_1a70, &args![me]);
                e.call(0x008b_65f0, &args![me]);
            } else {
                let extra = extra_list(e, me);
                e.call(0x0042_2bc0, &args![extra, me]);
            }
        });
    } else {
        let extra = extra_list(e, me);
        if e.call(GET_PRIMITIVE, &args![extra]).u32() != 0 {
            e.vcall(me, 0x1c4, &args![]);
        }
    }
}

// Translated from 0056f930 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::FindReferenceFor3D` (Xbox PDB), cdecl: the reference a
/// node (or one of its ancestors) belongs to. Walks up the parents
/// (`009611e0`): the player's first-person root (`00950bb0(player, 1)`)
/// answers with the player; a node whose owner (virtual at `+0x10`) has a
/// reference (`009ad610`) answers with it, or, when that reference's base
/// object is of form type `0x29`, with what the recursion on the node's
/// parent finds if it finds one. Returns 0 when nothing owns the node.
pub fn tes_object_refr_find_reference_for_3d(e: &mut Engine, node: u32) -> u32 {
    let mut found = 0u32;
    let mut first_person_root = 0u32;
    let player = e.global::<u32>(GLOBAL_PLAYER);
    if player != 0 {
        first_person_root = e.call(0x0095_0bb0, &args![player, 1u32]).u32();
    }
    let mut current = node;
    while current != 0 {
        if current == first_person_root {
            return e.global::<u32>(GLOBAL_PLAYER);
        }
        let owner = e.vcall(current, 0x10, &args![]).u32();
        if owner != 0 {
            found = e.call(0x009a_d610, &args![owner]).u32();
            if found != 0 {
                if base_form(e, found) != 0 {
                    let base = base_form(e, found);
                    if form_type(e, base) == 0x29 {
                        let parent = e.call(NODE_PARENT, &args![current]).u32();
                        let deeper = tes_object_refr_find_reference_for_3d(e, parent);
                        if deeper != 0 {
                            found = deeper;
                        }
                    }
                }
                return found;
            }
        }
        current = e.call(NODE_PARENT, &args![current]).u32();
    }
    found
}

// Translated from 0056fa00 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::GetOrientation` (Xbox PDB): writes the 3x3 rotation
/// matrix (nine floats) of the reference's angles (`+0x24`, `+0x28`,
/// `+0x2c`) to `out` through `NiMatrix3::FromEulerAnglesXYZ`; an actor uses
/// only its heading (`x = y = 0`). Returns `out`.
pub fn tes_object_refr_get_orientation(e: &mut Engine, this: Ptr<TESObjectREFR>, out: Ptr) -> Ptr {
    let me = this.addr();
    e.with_stack(0x24, |e, matrix| {
        e.call(NOTHING_CONSTRUCTOR, &args![matrix]);
        // OBJ_REFR::Angle (Xbox PDB) +0x24 / +0x28 / +0x2c
        let angle_z = e.mem.f32(me + 0x2c);
        if is_actor(e, me) {
            e.call(MATRIX_FROM_EULER, &args![matrix, 0.0f32, 0.0f32, angle_z]);
        } else {
            let angle_x = e.mem.f32(me + 0x24);
            let angle_y = e.mem.f32(me + 0x28);
            e.call(MATRIX_FROM_EULER, &args![matrix, angle_x, angle_y, angle_z]);
        }
        copy_words(e, out.addr(), matrix.addr(), 9);
    });
    out
}

// Translated from 0056fa90 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::SetOrientation` (Xbox PDB): the rotation matrix, passed
/// by value (nine words), is converted back into the reference's three
/// angles (`+0x24`, `+0x28`, `+0x2c`) by `NiMatrix3::ToEulerAnglesXYZ`.
#[allow(clippy::too_many_arguments)]
pub fn tes_object_refr_set_orientation(
    e: &mut Engine,
    this: Ptr<TESObjectREFR>,
    m0: u32,
    m1: u32,
    m2: u32,
    m3: u32,
    m4: u32,
    m5: u32,
    m6: u32,
    m7: u32,
    m8: u32,
) {
    let me = this.addr();
    e.with_stack(0x24, |e, matrix| {
        for (i, word) in [m0, m1, m2, m3, m4, m5, m6, m7, m8].into_iter().enumerate() {
            e.mem.set_u32(matrix.addr() + 4 * i as u32, word);
        }
        e.call(
            MATRIX_TO_EULER,
            &args![matrix, me + 0x24, me + 0x28, me + 0x2c],
        );
    });
}

// Translated from 0056fac0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::MultipleMatrixByRace` (Xbox PDB): copies the matrix
/// `matrix` (nine words) to `out` and returns `out`. The race-dependent
/// part is compiled out; the only thing left of it is the read of the form
/// type of the base object (`00401170`, `+0x20`).
pub fn tes_object_refr_multiple_matrix_by_race(
    e: &mut Engine,
    this: Ptr<TESObjectREFR>,
    out: Ptr,
    matrix: Ptr,
) -> Ptr {
    // OBJ_REFR::pObjectReference (Xbox PDB) +0x20
    let base = e.mem.u32(this.addr() + 0x20);
    form_type(e, base);
    copy_words(e, out.addr(), matrix.addr(), 9);
    out
}

/// The bound corner of a 3D node: if `node` has a bound extra
/// (`NiObjectNET::GetExtraData` with the name at `012031e4`, read through
/// `0050ea90`) the corner `corner` computes into the scratch vector
/// `scratch`, which is copied to `out`; otherwise the zero vector.
fn bound_corner(e: &mut Engine, out: u32, node: u32, corner: u32, scratch: u32) -> u32 {
    if node != 0 {
        let name = e.call(0x0050_ea90, &args![]).u32();
        let bound = e.call(NODE_GET_EXTRA_DATA, &args![node, name]).u32();
        if bound != 0 {
            e.call(corner, &args![bound, scratch]);
            copy_words(e, out, scratch, 3);
            return out;
        }
    }
    copy_words(e, out, ZERO_VECTOR, 3);
    out
}

// Translated from 0056fb10 (decompiled, FalloutNV.exe 1.4.0.525)
/// Writes the minimum corner of the reference's 3D bound to `out` (center
/// minus extents, `0050ea10`), the zero vector when it has no 3D or no
/// bound extra. Returns `out`.
pub fn fn_0056fb10(e: &mut Engine, this: Ptr<TESObjectREFR>, out: Ptr) -> Ptr {
    let node = get_3d(e, this.addr());
    Ptr::new(bound_corner(
        e,
        out.addr(),
        node,
        0x0050_ea10,
        SCRATCH_BOUND_MIN,
    ))
}

// Translated from 0056fba0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Writes the maximum corner of the reference's 3D bound to `out` (center
/// plus extents, `0050ea50`), the zero vector when it has no 3D or no bound
/// extra. Returns `out`.
pub fn fn_0056fba0(e: &mut Engine, this: Ptr<TESObjectREFR>, out: Ptr) -> Ptr {
    let node = get_3d(e, this.addr());
    Ptr::new(bound_corner(
        e,
        out.addr(),
        node,
        0x0050_ea50,
        SCRATCH_BOUND_MAX,
    ))
}

// Translated from 0056fc30 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::GetBoundMax` (Xbox PDB), cdecl: the maximum corner of
/// the bound of `node` written to `out` (as `0056fba0` does for the
/// reference's 3D); the zero vector for a null node or one without a bound
/// extra. Returns `out`.
pub fn tes_object_refr_get_bound_max(e: &mut Engine, out: Ptr, node: u32) -> Ptr {
    Ptr::new(bound_corner(
        e,
        out.addr(),
        node,
        0x0050_ea50,
        SCRATCH_BOUND_MAX,
    ))
}

/// The item of a list node (`006815c0` returns the address of the node's
/// item slot).
fn node_item(e: &mut Engine, node: u32) -> u32 {
    let slot = e.call(NOTHING_CONSTRUCTOR, &args![node]).u32();
    e.mem.u32(slot)
}

/// Base object forms that mark rooms and portals: the base object of a
/// reference is compared with the words at these globals.
const GLOBAL_ROOM_MARKER_BASE: u32 = 0x011c_a238;
const GLOBAL_PORTAL_MARKER_BASE: u32 = 0x011c_a23c;
const GLOBAL_MULTIBOUND_BASE: u32 = 0x011c_a234;

// Translated from 0056fcb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Takes a reference out of everything that tracks its 3D, before the 3D is
/// dropped (`Set3D` with a null node calls it). Does nothing when the
/// reference has no node (virtual at `+0x1d0`). Otherwise:
///
/// * a base object of type `0x23` decrements the counter at `+0xb8` of the
///   object at `011dea10` (`0045ce60`);
/// * `00565670(this)`, `RemoveFromAllWater(this, 0)` (`0057b520`), and for
///   each item of the extra list `00420770` returns, when both this
///   reference and the item have a multibound room (`005699b0`), the room's
///   animation data is added to the item room's list (`0057c5d0`);
/// * a portal marker (base object equal to the word at `011ca23c`) is taken
///   out of the two rooms it joins (`00420bc0`, `00420da0`);
/// * a type `0x1e` base object clears the water-light reference of every lit
///   water reference (`0041f810`, `0041f840`) and, for the ones with a
///   process of type 0xd, updates the property list on its node;
/// * a type `0xe` base object in a cell tells the acoustic space listener
///   (`0061f890`);
/// * with a parent cell the reference leaves the cell's emittance and
///   lighting lists (`00545360`, `005454f0`, `005456e0`) and the room,
///   multibound or portal data of the cell is updated.
///
/// The compiler's exception-unwinding frame is not translated.
pub fn fn_0056fcb0(e: &mut Engine, this: Ptr<TESObjectREFR>) {
    let me = this.addr();
    if node_of(e, me) == 0 {
        return;
    }
    let tes = e.global::<u32>(GLOBAL_TES);
    if base_form(e, me) != 0 {
        let base = base_form(e, me);
        if form_type(e, base) == 0x23 {
            e.call(0x0045_ce60, &args![tes]);
        }
    }
    e.call(0x0056_5670, &args![me]);
    e.call(0x0057_b520, &args![me, 0u32]);
    let extra = extra_list(e, me);
    let mut rooms = e.call(0x0042_0770, &args![extra]).u32();
    while rooms != 0 && node_item(e, rooms) != 0 {
        let other = node_item(e, rooms);
        if e.call(0x0056_99b0, &args![me]).u32() != 0
            && e.call(0x0056_99b0, &args![other]).u32() != 0
        {
            with_frame(e, |e, f| {
                let own_room = e.call(0x0056_99b0, &args![me]).u32();
                let data = e.call(0x0066_29f0, &args![own_room]).u32();
                e.call(0x0063_3c90, &args![f.at(-0x60), data]);
                let other_room = e.call(0x0056_99b0, &args![other]).u32();
                let list = e.call(0x0045_c650, &args![other_room]).u32();
                e.call(0x0057_c5d0, &args![list, f.at(-0x64), f.at(-0x60)]);
                e.call(0x0045_cec0, &args![f.at(-0x64)]);
                e.call(0x0045_cec0, &args![f.at(-0x60)]);
            });
        }
        rooms = e.call(LIST_NEXT, &args![rooms]).u32();
    }
    // the portal marker
    if base_form(e, me) == e.global::<u32>(GLOBAL_PORTAL_MARKER_BASE) {
        let extra = extra_list(e, me);
        let _portal = e.call(0x0042_0dd0, &args![extra]).u32();
        if base_form(e, me) == e.global::<u32>(GLOBAL_PORTAL_MARKER_BASE) {
            for side in 0..2u32 {
                let extra = extra_list(e, me);
                let room = e.call(0x0042_0bc0, &args![extra, side]).u32();
                if room != 0 {
                    let room_extra = extra_list(e, room);
                    e.call(0x0042_0da0, &args![room_extra, me]);
                }
            }
        }
    }
    // a type 0x1e base object: the water lights
    if base_form(e, me) != 0 {
        let base = base_form(e, me);
        if form_type(e, base) == 0x1e {
            let extra = extra_list(e, me);
            let mut lit = e.call(0x0041_f810, &args![extra]).u32();
            let stored = e.call(0x0057_25f0, &args![me]).u32();
            while lit != 0 && node_item(e, lit) != 0 {
                let item = node_item(e, lit);
                let item_extra = extra_list(e, item);
                e.call(0x0041_f840, &args![item_extra, me, 0u32]);
                if stored != 0 && e.call(READ_FIRST_DWORD, &args![stored]).u32() != 0 {
                    let object = node_item(e, lit);
                    let node = node_of(e, object);
                    if node != 0 && e.call(0x0070_ec90, &args![tes]).u32() != 0 {
                        let item = node_item(e, lit);
                        let manager = e.call(0x0070_ec90, &args![tes]).u32();
                        let water_node = e.call(0x004e_8030, &args![manager, item]).u32();
                        let scheduler = e.call(0x0040_30b0, &args![water_node]).u32();
                        if scheduler != 0 && e.call(GET_PROCESS, &args![scheduler]).u32() == 0xd {
                            let property = e.call(0x00a5_9d30, &args![water_node, 3u32]).u32();
                            let value = e.call(READ_FIRST_DWORD, &args![stored]).u32();
                            with_frame(e, |e, f| {
                                e.mem.set_u32(f.at(-0x68), value);
                                e.call(0x0057_c730, &args![property + 0x128, f.at(-0x68)]);
                            });
                        }
                    }
                }
                lit = e.call(LIST_NEXT, &args![lit]).u32();
            }
        }
    }
    // a type 0xe base object in a cell
    if base_form(e, me) != 0 {
        let base = base_form(e, me);
        if form_type(e, base) == 0xe && parent_cell(e, me) != 0 {
            let cell = parent_cell(e, me);
            let world = e.call(0x0045_43c0, &args![cell]).u32();
            let listener = e.call(0x0045_cd60, &args![world]).u32();
            e.call(0x0061_f890, &args![listener, me, 1u32]);
        }
    }
    // TESObjectREFR::pParentCell (Xbox PDB) +0x40
    let cell = e.mem.u32(me + 0x40);
    if cell == 0 {
        return;
    }
    if base_form(e, me) != 0 {
        let base = base_form(e, me);
        if form_type(e, base) == 0x23 {
            let manager = e.call(0x0070_ec90, &args![tes]).u32();
            e.call(0x004e_52f0, &args![manager, me]);
        }
    }
    e.call(0x0054_5360, &args![cell, me]);
    let cell = e.mem.u32(me + 0x40);
    e.call(0x0054_54f0, &args![cell, me]);
    let cell = e.mem.u32(me + 0x40);
    let lighting = if cell != 0 && e.call(0x0045_43c0, &args![cell]).u32() != 0 {
        let world = e.call(0x0045_43c0, &args![cell]).u32();
        e.call(0x0059_bb30, &args![world]).u32()
    } else {
        0
    };
    if lighting != 0 {
        e.call(0x0063_1370, &args![lighting, me, 0u32, 1u32]);
    }
    if base_form(e, me) != 0 {
        let base = base_form(e, me);
        if form_type(e, base) == 0x23 {
            let cell = e.mem.u32(me + 0x40);
            e.call(0x0054_56e0, &args![cell, me]);
            e.call(0x0063_12d0, &args![lighting, me]);
        }
    }
    let cell = e.mem.u32(me + 0x40);
    let registry = e.call(0x009d_9f20, &args![cell]).u32();
    if registry == 0 || base_form(e, me) == 0 {
        return;
    }
    let base = base_form(e, me);
    if base == e.global::<u32>(GLOBAL_ROOM_MARKER_BASE) {
        let extra = extra_list(e, me);
        let room = e.call(0x0042_0ed0, &args![extra]).u32();
        if room != 0 {
            e.call(0x00c5_b060, &args![registry, room]);
        }
    } else if base_form(e, me) == e.global::<u32>(GLOBAL_MULTIBOUND_BASE) {
        let extra = extra_list(e, me);
        let multibound = e.call(0x0042_2120, &args![extra]).u32();
        if multibound != 0 {
            let extra = extra_list(e, me);
            let linked = e.call(0x0042_1e10, &args![extra]).u32();
            if linked != 0 {
                let extra = extra_list(e, me);
                let linked = e.call(0x0042_1e10, &args![extra]).u32();
                let linked_extra = extra_list(e, linked);
                let room = e.call(0x0042_0ed0, &args![linked_extra]).u32();
                if room != 0 {
                    e.call(0x00c3_9ad0, &args![room, multibound]);
                } else {
                    e.call(0x00c5_aec0, &args![registry, multibound]);
                }
            } else {
                e.call(0x00c5_aec0, &args![registry, multibound]);
            }
        }
    } else if base_form(e, me) == e.global::<u32>(GLOBAL_PORTAL_MARKER_BASE) {
        let extra = extra_list(e, me);
        let portal = e.call(0x0042_0dd0, &args![extra]).u32();
        if portal != 0 {
            for side in 0..2u32 {
                let extra = extra_list(e, me);
                let joined = e.call(0x0042_0410, &args![extra]).u32();
                let joined_item = if joined != 0 {
                    e.mem.u32(joined + 4 * side)
                } else {
                    0
                };
                if joined_item != 0 {
                    if e.call(0x0056_99b0, &args![joined_item]).u32() != 0 {
                        let item = e.mem.u32(joined + 4 * side);
                        let room = e.call(0x0056_99b0, &args![item]).u32();
                        let list = e.call(0x006a_b360, &args![room]).u32();
                        with_frame(e, |e, f| {
                            e.mem.set_u32(f.at(-0x54), portal);
                            e.call(0x0057_c730, &args![list, f.at(-0x54)]);
                        });
                        if side == 0 {
                            e.call(0x0097_07f0, &args![portal, 0u32]);
                        } else {
                            e.call(0x0042_0ba0, &args![portal, 0u32]);
                        }
                    }
                } else {
                    e.call(0x00c5_b190, &args![registry, portal]);
                }
            }
        }
    }
}

// Translated from 005702e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::Set3D` (Xbox PDB, virtual at `+0x1cc`): replaces the 3D
/// of the reference with `new_node` (null to unload it). `log_unload` makes
/// the unload of an actor whose process type is at most 1 print the "AI:
/// Unloading art" message.
///
/// * The same node as the current one: nothing, except that "replacing" no
///   3D with none cancels the model-loader request and runs
///   `00579ac0(this, 0)`.
/// * The model-loader request is cancelled when the current 3D has no
///   parent. When unloading an actor's 3D whose process type is at most 1,
///   the process is told to drop its art (`0092b7f0` and the virtuals
///   `+0x79c`, `+0x794`, `+0x7a4`). The loaded data's word at `+4` is
///   cleared; when unloading, the primitive extra's slots are cleared
///   (`004a4c50`) and the reference is detached from everything that tracks
///   its 3D (`fn_0056fcb0`).
/// * A reference with a non-zero object health (`0041b6b0`, when `00452370`
///   holds) asks its base object for the destruction form
///   (`GetDestructionForm`, `00475400`): unloading runs `004778a0` on it,
///   getting a 3D preloads its replacement models (`PreloadReplacementModels`,
///   `00477780`).
/// * The old node (held in a `NiPointer` local) leaves its parent (virtual
///   `+0xe8`, or `0087ac60` on the task queue) and the shadow scene node;
///   its local translation (`+0x58`), rotation (`+0x34`) and scale (`+0x64`)
///   are remembered. Unloading also saves Havok data and animation for the
///   changes the loading object reports (`0084a6d0`), kills the light and
///   the animation, removes the weapon of an actor
///   ([`tes_object_refr_remove_weapon`]), clears the ragdoll data of an
///   actor, stops the moving sounds, queues the node with the garbage
///   collector (`00868660`, or `00c45830` for the player or a non-actor) and
///   releases it.
/// * The new node gets the remembered transform (`00440460`, `0043fa80`,
///   `00440490`), is attached to the old parent (virtual `+0xdc`) and
///   updated (`00a5a040`, `00a59c60` with update data at time 0.0).
/// * The nav-mesh obstacle manager is told of the change (virtual `+0x15c`,
///   `006c0c30`, `006c0c80`).
///
/// A reference whose base object has form type `0x25` is left alone when
/// it already has a 3D and gets one. The compiler's exception-unwinding
/// frame is not translated.
pub fn tes_object_refr_set_3d(
    e: &mut Engine,
    this: Ptr<TESObjectREFR>,
    new_node: u32,
    log_unload: bool,
) {
    let me = this.addr();
    if get_3d(e, me) == new_node {
        if new_node == 0 {
            let loader = model_loader(e);
            e.call(CANCEL_REFERENCE, &args![loader, me]);
            e.call(0x0057_9ac0, &args![me, 0u32]);
        }
        return;
    }
    let current = get_3d(e, me);
    if current != 0 {
        let current = get_3d(e, me);
        if e.call(NODE_PARENT, &args![current]).u32() == 0 {
            let loader = model_loader(e);
            e.call(CANCEL_REFERENCE, &args![loader, me]);
        }
    }
    // TESObjectREFR::pLoadedData (Xbox PDB) +0x64, LOADED_REF_DATA's 3D at +0x14
    let loaded = e.mem.u32(me + 0x64);
    if loaded != 0
        && e.call(READ_FIRST_DWORD, &args![loaded + 0x14]).u32() != 0
        && new_node == 0
        && is_actor(e, me)
        && e.call(GET_PROCESS, &args![me]).u32() != 0
        && (e.call(0x0093_1850, &args![me]).u32() as i32) <= 1
    {
        if log_unload {
            let node = e.call(READ_FIRST_DWORD, &args![loaded + 0x14]).u32();
            let name = e.call(0x0041_3f40, &args![node]).u32();
            let name = e.call(0x0043_b1b0, &args![name]).u32();
            e.call(0x005b_5e40, &args![0x0103_0df8u32, name]);
        }
        let process = e.call(GET_PROCESS, &args![me]).u32();
        e.vcall(process, 0x79c, &args![0u32]);
        e.vcall(process, 0x794, &args![0u32]);
        e.vcall(process, 0x7a4, &args![0u32]);
        e.call(0x0092_b7f0, &args![process]);
    }
    let loaded = e.mem.u32(me + 0x64);
    if loaded != 0 {
        e.mem.set_u32(loaded + 4, 0);
    }
    let extra = extra_list(e, me);
    let primitive = e.call(GET_PRIMITIVE, &args![extra]).u32();
    if new_node == 0 && primitive != 0 {
        e.call(0x004a_4c50, &args![primitive]);
    }
    if new_node == 0 {
        fn_0056fcb0(e, this);
    }
    with_frame(e, |e, f| {
        let data = e.call(0x0089_1170, &args![me]).u32();
        // OBJ_REFR::Location (Xbox PDB) +0x10 of the data
        copy_words(e, f.at(-0x4c), data + 0x10, 3);
        e.call(NOTHING_CONSTRUCTOR, &args![f.at(-0x3c)]);
        let mut scale = e.call(GET_SCALE, &args![me]).f32();
        let mut parent = 0u32;
        let mut actor = 0u32;
        if is_actor(e, me) {
            actor = me;
        }
        if get_3d(e, me) != 0 && new_node != 0 {
            let base = e.mem.u32(me + 0x20);
            if base != 0 && form_type(e, base) == 0x25 {
                return;
            }
        }
        if e.call(0x0045_2370, &args![me]).bool() {
            let extra = extra_list(e, me);
            let health = e.call(0x0041_b6b0, &args![extra]).f64();
            if health != e.global::<f64>(0x0101_2060) {
                if get_3d(e, me) != 0 && new_node == 0 {
                    let base = base_form(e, me);
                    let destruction = e.call(0x0047_5400, &args![base]).u32();
                    e.call(0x0047_78a0, &args![destruction]);
                } else if new_node != 0 && get_3d(e, me) == 0 {
                    let first = base_form(e, me);
                    let second = base_form(e, me);
                    let destruction = e.call(0x0047_5400, &args![second]).u32();
                    e.call(0x0047_7780, &args![destruction, first]);
                }
            }
        }
        let old = f.at(-0x18);
        let current = get_3d(e, me);
        e.call(0x0063_3c90, &args![old, current]);
        let old_node = e.call(READ_FIRST_DWORD, &args![old]).u32();
        if old_node == 0 {
            if new_node == 0 {
                let loader = model_loader(e);
                e.call(CANCEL_REFERENCE, &args![loader, me]);
            }
        } else {
            let old_node = e.call(READ_FIRST_DWORD, &args![old]).u32();
            parent = e.call(NODE_PARENT, &args![old_node]).u32();
            if parent != 0 {
                let base = e.mem.u32(me + 0x20);
                let leave_attached = e.call(0x0040_77c0, &args![me]).bool()
                    && base != 0
                    && form_type(e, base) == 0x25;
                if !leave_attached {
                    if e.call(0x008c_7aa0, &args![]).bool() {
                        let node = e.call(READ_FIRST_DWORD, &args![old]).u32();
                        let manager = e.call(0x0045_37b0, &args![]).u32();
                        e.call(0x0087_ac60, &args![manager, node]);
                    } else {
                        let node = e.call(READ_FIRST_DWORD, &args![old]).u32();
                        e.vcall(parent, 0xe8, &args![node]);
                    }
                }
                // the transform of the node being replaced
                let node = e.call(READ_FIRST_DWORD, &args![old]).u32();
                let translate = e.call(0x0043_c490, &args![node]).u32();
                copy_words(e, f.at(-0x4c), translate, 3);
                let node = e.call(READ_FIRST_DWORD, &args![old]).u32();
                let rotate = e.call(0x006a_9540, &args![node]).u32();
                copy_words(e, f.at(-0x3c), rotate, 9);
                let node = e.call(READ_FIRST_DWORD, &args![old]).u32();
                scale = e.call(0x009c_dae0, &args![node]).f32();
            }
            let node = e.call(READ_FIRST_DWORD, &args![old]).u32();
            let scene = e.call(0x0045_0b80, &args![0u32]).u32();
            e.call(0x00b5_b1c0, &args![scene, node]);
            if e.vcall(me, 0xfc, &args![]).bool() {
                let node = e.call(READ_FIRST_DWORD, &args![old]).u32();
                let scene = e.call(0x0045_0b80, &args![0u32]).u32();
                e.call(0x00b5_edf0, &args![scene, node]);
            }
            if new_node == 0 {
                unload_old_3d(e, f, me, actor, old);
            }
            let cell = e.mem.u32(me + 0x40);
            if cell == 0 || !fn_00570ec0(e, Ptr::new(cell)) {
                e.call(0x0097_49b0, &args![OBJECT_PROCESS_LISTS, me]);
                if (fn_00570f60(e) as i32) > 0 {
                    e.call(0x008c_ddd0, &args![me, 1u32]);
                }
            }
            if actor == 0 || actor == e.global::<u32>(GLOBAL_PLAYER) {
                let node = e.call(READ_FIRST_DWORD, &args![old]).u32();
                e.call(0x00c4_5830, &args![node]);
            } else {
                let process = e.call(GET_PROCESS, &args![actor]).u32();
                if process != 0 && (e.call(0x0045_cd60, &args![process]).u32() as i32) <= 1 {
                    e.vcall(process, 0x4fc, &args![0u32]);
                }
                let node = e.call(READ_FIRST_DWORD, &args![old]).u32();
                e.call(0x0086_8660, &args![node]);
            }
            let held_by_loader = e.call(0x0045_23e0, &args![me, 4u32]).bool();
            e.call(0x0066_b0d0, &args![old, 0u32]);
            fn_00570f70(e, this, 0);
            let base = e.mem.u32(me + 0x20);
            if new_node == 0 && base != 0 {
                if e.call(0x0056_51e0, &args![me]).bool() {
                    let loader = model_loader(e);
                    e.call(CANCEL_REFERENCE, &args![loader, me]);
                } else if !held_by_loader {
                    let base = e.mem.u32(me + 0x20);
                    e.vcall(base, 0x150, &args![me]);
                }
            }
        }
        if new_node == 0 {
            e.call(0x0057_9ac0, &args![me, 0u32]);
        }
        fn_00570f70(e, this, new_node);
        if new_node != 0 && parent != 0 {
            e.call(0x0044_0460, &args![new_node, f.at(-0x4c)]);
            e.call(0x0043_fa80, &args![new_node, f.at(-0x3c)]);
            e.call(0x0044_0490, &args![new_node, scale]);
            e.vcall(parent, 0xdc, &args![new_node, 1u32]);
            e.call(0x00c6_bd00, &args![new_node, 1u32]);
            e.call(0x00a5_a040, &args![new_node]);
            e.call(
                UPDATE_DATA_CONSTRUCTOR,
                &args![f.at(-0x88), 0.0f32, 0u32, 0u32],
            );
            e.call(NODE_UPDATE, &args![new_node, f.at(-0x88)]);
        }
        let base = e.mem.u32(me + 0x20);
        if new_node != 0 && base != 0 && form_type(e, base) != 0x25 {
            e.call(0x0056_c7d0, &args![new_node, me]);
        }
        let save_load = e.global::<u32>(GLOBAL_SAVE_LOAD);
        if !e.call(0x0047_c850, &args![save_load]).bool()
            && new_node == 0
            && e.call(0x0056_8e50, &args![me]).u32() != 0
        {
            // the teleport data of the reference and the cell it leads to
            let teleport = e.call(0x0056_8e50, &args![me]).u32();
            if e.call(0x0043_a2b0, &args![teleport]).u32() != 0 {
                let teleport = e.call(0x0056_8e50, &args![me]).u32();
                let cell = e.call(0x0043_a2b0, &args![teleport]).u32();
                e.call(0x0055_1480, &args![cell, 0u32]);
            }
        }
        if e.vcall(me, 0x15c, &args![]).bool() {
            let manager = e.call(0x006c_0720, &args![]).u32();
            if new_node != 0 {
                e.call(0x006c_0c30, &args![manager, me]);
            } else {
                e.call(0x006c_0c80, &args![manager, me]);
            }
        }
        if new_node == 0 && base_form(e, me) != 0 {
            let base = base_form(e, me);
            if form_type(e, base) == 0x1c {
                let manager = e.call(0x006c_0720, &args![]).u32();
                e.call(0x006c_1060, &args![manager, me]);
            }
        }
        e.call(0x0045_cec0, &args![old]);
    });
}

/// The part of `Set3D` that runs when the 3D is being unloaded and the
/// reference had a 3D (`old` is the `NiPointer` local at `ebp-0x18`).
fn unload_old_3d(e: &mut Engine, f: &Frame, me: u32, actor: u32, old: u32) {
    let mut owner_has_flag = false;
    let node = e.call(READ_FIRST_DWORD, &args![old]).u32();
    let owner = e.vcall(node, 0x10, &args![]).u32();
    if owner != 0 {
        e.call(0x0056_c7d0, &args![owner, 0u32]);
        owner_has_flag = fn_00570ee0(e, Ptr::new(owner));
    }
    let node = e.call(READ_FIRST_DWORD, &args![old]).u32();
    if fn_00570ea0(e, Ptr::new(node)) {
        let node = e.call(READ_FIRST_DWORD, &args![old]).u32();
        let name = e.vcall(node, 0xc, &args![]).u32();
        e.call(0x0057_8250, &args![name]);
    }
    e.call(0x0056_8480, &args![me]);
    e.call(0x009c_9e90, &args![0x011f_2250u32, me]);
    if !e.call(FORM_IS_DELETED, &args![me]).bool() {
        let loading = e.global::<u32>(GLOBAL_LOADING_OBJECT);
        if fn_00570f00(e, Ptr::new(loading)) {
            let change = change_flags(e, 4);
            if e.call(0x0084_a6d0, &args![loading, me, change]).bool() {
                e.call(0x0044_dee0, &args![f.at(-0x64)]);
                e.call(0x0086_5020, &args![f.at(-0x64), me]);
                let buffer = e.call(READ_FIRST_DWORD, &args![f.at(-0x64)]).u32();
                let extra = extra_list(e, me);
                e.call(0x0042_2ac0, &args![extra, buffer]);
            }
            let change = change_flags(e, 0x1000_0000);
            if e.call(0x0084_a6d0, &args![loading, me, change]).bool() {
                let mut has_animation = false;
                if !is_actor(e, me) {
                    has_animation = e.call(0x0056_3530, &args![me]).bool();
                }
                if has_animation {
                    e.call(0x0044_dee0, &args![f.at(-0x6c)]);
                    e.call(0x0086_4f20, &args![f.at(-0x6c), me]);
                    let buffer = e.call(READ_FIRST_DWORD, &args![f.at(-0x6c)]).u32();
                    let extra = extra_list(e, me);
                    e.call(0x0042_2940, &args![extra, buffer]);
                } else {
                    e.vcall(me, 0x4c, &args![0x1000_0000u32]);
                }
            }
        }
    }
    // KillLight and SetAnimation (the map names 00572a50 and 00572e50)
    e.call(0x0057_2a50, &args![me, 0u32]);
    e.call(0x0057_2e50, &args![me, 0u32]);
    if actor != 0 {
        tes_object_refr_remove_weapon(e, Ptr::new(me));
        e.call(0x0045_34f0, &args![me, 0u32]);
        e.call(0x0048_3710, &args![me]);
        e.call(0x0048_3710, &args![me]);
    }
    e.vcall(me, 0x1f0, &args![0u32]);
    let node = e.call(READ_FIRST_DWORD, &args![old]).u32();
    let name = e.vcall(node, 0xc, &args![]).u32();
    let has_addon_flags = e.call(0x0057_84b0, &args![name]).bool();
    e.call(0x0056_c880, &args![me, 2u32, u32::from(has_addon_flags)]);
    if has_addon_flags {
        let node = e.call(READ_FIRST_DWORD, &args![old]).u32();
        e.call(0x0057_8300, &args![node]);
    }
    if actor != 0 {
        let controller = e.call(0x0093_06d0, &args![actor]).u32();
        if controller != 0 {
            e.call(0x00c6_d7a0, &args![controller, 0u32]);
        }
        if e.call(GET_PROCESS, &args![actor]).u32() != 0 {
            let process = e.call(GET_PROCESS, &args![actor]).u32();
            e.vcall(process, 0x5f4, &args![0u32]);
        }
        // Actor +0xac: an owned object deleted with its deleting destructor
        let owned = e.mem.u32(actor + 0xac);
        if owned != 0 {
            e.vcall(owned, 0, &args![1u32]);
            e.mem.set_u32(actor + 0xac, 0);
        }
        // Actor +0xb0: bhkRagdollPenetrationUtil::Clear's owner
        let ragdoll = e.mem.u32(actor + 0xb0);
        if ragdoll != 0 {
            e.call(0x00ca_20e0, &args![ragdoll]);
        }
    }
    if owner_has_flag || e.vcall(me, 0x224, &args![]).bool() || e.vcall(me, 0xfc, &args![]).bool() {
        let node = node_of(e, me);
        let audio = e.call(0x0045_3a70, &args![]).u32();
        e.call(0x00ad_8570, &args![audio, node, 0.0f32, 0u32]);
    }
    e.call(0x0057_a3c0, &args![me, 1u32]);
}

/// A `BGSChangeFlags`-style by-value change mask (`008c71b0` stores the
/// value into the stack slot the game passes as the argument).
fn change_flags(e: &mut Engine, value: u32) -> u32 {
    e.with_stack(4, |e, slot| {
        e.call(0x008c_71b0, &args![slot, value]);
        e.mem.u32(slot.addr())
    })
}

// Translated from 00570ea0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Tests flag `0x20000` of the node `this` (`00456630`, which tests its
/// flags word at `+0x30`).
pub fn fn_00570ea0(e: &mut Engine, this: Ptr) -> bool {
    e.call(0x0045_6630, &args![this, 0x2_0000u32]).bool()
}

// Translated from 00570ec0 (decompiled, FalloutNV.exe 1.4.0.525)
/// True when `00450fd0` (the byte at `+0x26`) of the cell `this` is 1.
pub fn fn_00570ec0(e: &mut Engine, this: Ptr) -> bool {
    e.call(0x0045_0fd0, &args![this]).u32() == 1
}

// Translated from 00570ee0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Wrapper of `00570ea0` (flag `0x20000` of the node `this`).
pub fn fn_00570ee0(e: &mut Engine, this: Ptr) -> bool {
    fn_00570ea0(e, this)
}

// Translated from 00570f00 (decompiled, FalloutNV.exe 1.4.0.525)
/// True when `00462480` holds and `00570f40` holds for `this` (the object
/// at `011ddf38`): the thread's state says it is loading and the object has
/// bit 0 of its word at `+0x244` clear.
pub fn fn_00570f00(e: &mut Engine, this: Ptr) -> bool {
    e.call(0x0046_2480, &args![this]).bool() && fn_00570f40(e, this)
}

// Translated from 00570f40 (decompiled, FalloutNV.exe 1.4.0.525)
/// True when bit 0 of the word at `+0x244` of `this` is clear.
pub fn fn_00570f40(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u32(this.addr() + 0x244) & 1 == 0
}

// Translated from 00570f60 (decompiled, FalloutNV.exe 1.4.0.525)
/// The global at `011dfc98` (a count the callers compare with 0).
pub fn fn_00570f60(e: &mut Engine) -> u32 {
    e.global::<u32>(0x011d_fc98)
}

// Translated from 00570f70 (decompiled, FalloutNV.exe 1.4.0.525)
/// Tail of `Set3D`: tells the nav-mesh obstacle manager (`006c0720`) about
/// the reference when its virtual at `+0x15c` says it matters (added with a
/// 3D, removed without), forgets a base object of form type `0x1c` when the
/// 3D is gone (`006c1060`), clears the room extra of a room marker or the
/// portal extra of a portal marker (`00420f00`, `00420e00`), runs the
/// virtual at `+0x1c0` (or, when `008c7aa0` holds, the task queue's
/// `0087b2b0` on the singleton `004537b0`) and finally `Set3DVerySimple`.
pub fn fn_00570f70(e: &mut Engine, this: Ptr<TESObjectREFR>, node: u32) {
    let me = this.addr();
    if e.vcall(me, 0x15c, &args![]).bool() {
        let manager = e.call(0x006c_0720, &args![]).u32();
        if node != 0 {
            e.call(0x006c_0c30, &args![manager, me]);
        } else {
            e.call(0x006c_0c80, &args![manager, me]);
        }
    }
    if node == 0 && base_form(e, me) != 0 {
        let base = base_form(e, me);
        if form_type(e, base) == 0x1c {
            let manager = e.call(0x006c_0720, &args![]).u32();
            e.call(0x006c_1060, &args![manager, me]);
        }
    }
    if node == 0 && base_form(e, me) == e.global::<u32>(GLOBAL_ROOM_MARKER_BASE) {
        let extra = extra_list(e, me);
        e.call(0x0042_0f00, &args![extra, 0u32]);
    } else if node == 0 && base_form(e, me) == e.global::<u32>(GLOBAL_PORTAL_MARKER_BASE) {
        let extra = extra_list(e, me);
        e.call(0x0042_0e00, &args![extra, 0u32]);
    }
    if e.call(0x008c_7aa0, &args![]).bool() {
        let queue = e.call(0x0045_37b0, &args![]).u32();
        e.call(0x0087_b2b0, &args![queue, me]);
    } else {
        e.vcall(me, 0x1c0, &args![]);
    }
    tes_object_refr_set_3d_very_simple(e, this, node);
}

// Translated from 00571080 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::Set3DVerySimple` (Xbox PDB): with a node, makes sure the
/// loaded data exists (`CreateLoadedData`, `0057c180`) and stores the node in
/// its 3D slot at `+0x14` (`0066b0d0`); without one, clears the loaded data
/// (`0057c300`).
pub fn tes_object_refr_set_3d_very_simple(e: &mut Engine, this: Ptr<TESObjectREFR>, node: u32) {
    if node != 0 {
        e.call(0x0057_c180, &args![this]);
        // TESObjectREFR::pLoadedData (Xbox PDB) +0x64
        let loaded = e.get(this, TESObjectREFR::pLoadedData).addr();
        e.call(0x0066_b0d0, &args![loaded + 0x14, node]);
    } else {
        e.call(0x0057_c300, &args![this]);
    }
}

// Translated from 005710c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Attaches the worn models of the base object of a reference to its 3D
/// (`this` is the reference). Returns what the base object accepted:
///
/// * an NPC base (form type `0x2a`): when the reference has a biped
///   (virtual `+0x1e8`) the base replaces the model (`TESNPC::ReplaceRefModel`,
///   `00605d70`); true;
/// * a creature base (`0x2b`) with a 3D: for each package of the base the
///   process runs (`00717e50`, `008256d0`) the model file named by the
///   reference's model with the file name replaced by the package's is
///   loaded (`ModelLoader::LoadFile`, `00447080`), cloned (deep copy when it
///   has a morpher controller) and attached to the 3D, to the skeleton under
///   a "SkinAttachment" node when it is skinned and the 3D has the named
///   node; true;
/// * any other base: its virtual at `+0x160`.
///
/// The animation (virtual `+0x1e4`) is first reset through `00496280`. The
/// compiler's exception-unwinding frame is not translated.
pub fn fn_005710c0(e: &mut Engine, this: Ptr<TESObjectREFR>) -> bool {
    let me = this.addr();
    let base = e.mem.u32(me + 0x20);
    let biped = e.vcall(me, 0x1e8, &args![]).u32();
    let mut done = false;
    let animation = e.vcall(me, 0x1e4, &args![]).u32();
    if animation != 0 {
        e.call(0x0049_6280, &args![animation]);
    }
    let node = get_3d(e, me);
    match form_type(e, base) {
        0x2a => {
            if biped != 0 {
                e.call(0x0060_5d70, &args![base, me]);
            }
            done = true;
        }
        0x2b => {
            if node != 0 {
                attach_creature_models(e, me, base, node);
            }
            done = true;
        }
        _ => {}
    }
    if !done {
        let base = e.mem.u32(me + 0x20);
        done = e.vcall(base, 0x160, &args![]).bool();
    }
    done
}

/// The creature branch of `fn_005710c0`: attaches the model of each package
/// to `node`.
fn attach_creature_models(e: &mut Engine, me: u32, base: u32, node: u32) {
    if is_actor(e, me) {
        let package = e.call(0x0093_44a0, &args![me]).u32();
        if package != 0 {
            e.call(0x0044_1b00, &args![package]);
        }
    }
    e.call(0x005f_a0a0, &args![base, me]);
    let mut list = e.call(0x0071_7e50, &args![base + 0x114]).u32();
    while list != 0 && !e.call(0x0082_56d0, &args![list]).bool() {
        with_frame(e, |e, f| {
            let path = tes_object_refr_get_model(e, Ptr::new(me));
            let buffer = f.at(-0x14c);
            e.call(0x0040_6d30, &args![buffer, 0x104u32, path]);
            let slash = e.call(0x0040_ab30, &args![buffer, 0x5cu32]).u32();
            let file = node_item(e, list);
            let room = 0x104u32
                .wrapping_sub(slash.wrapping_sub(buffer))
                .wrapping_sub(1);
            e.call(0x0040_6d30, &args![slash + 1, room, file]);
            let loader = model_loader(e);
            let model = e
                .call(
                    0x0044_7080,
                    &args![loader, buffer, 3u32, 1u32, 0u32, 0u32, 0u32],
                )
                .u32();
            if model != 0 {
                let cloning = f.at(-0x180);
                let scale = e.call(GET_SCALE, &args![me]).f32();
                e.call(0x004a_d050, &args![cloning, scale]);
                let holder = f.at(-0x15c);
                e.call(0x0063_3c90, &args![holder, 0u32]);
                let cloned = if e.call(0x004b_5bf0, &args![model]).bool() {
                    let tes = e.global::<u32>(GLOBAL_TES);
                    let copy = e.call(0x0045_7ba0, &args![tes, model, cloning]).u32();
                    e.call(0x0066_b0d0, &args![holder, copy]);
                    e.call(READ_FIRST_DWORD, &args![holder]).u32()
                } else {
                    e.call(0x00a5_d2c0, &args![model, cloning]).u32()
                };
                let skinned = e.call(0x0049_10d0, &args![cloned]).bool();
                let name = e.call(0x0057_1550, &args![]).u32();
                let found = fn_00571530(e, Ptr::new(me), node, name);
                if skinned && found != 0 {
                    let fixed = f.at(-0x188);
                    let key = e.call(0x0043_8170, &args![fixed, 0x0101_f724u32]).u32();
                    let child = e.vcall(found, 0x9c, &args![key]).u32();
                    e.call(0x0043_81b0, &args![fixed]);
                    if child != 0 {
                        e.vcall(child, 0xdc, &args![cloned, 1u32]);
                    } else {
                        let parent = e.call(NODE_PARENT, &args![found]).u32();
                        e.vcall(parent, 0xdc, &args![cloned, 1u32]);
                    }
                    e.call(0x004a_de40, &args![node, cloned, 0u32, 1u32]);
                } else {
                    e.call(
                        0x004a_e250,
                        &args![node, cloned, 0u32, 0u32, 0xffff_ffffu32, 0u32],
                    );
                }
                e.call(0x0045_cec0, &args![holder]);
                e.call(0x004a_d270, &args![cloning]);
            }
        });
        list = e.call(LIST_NEXT, &args![list]).u32();
    }
}

// Translated from 00571530 (decompiled, FalloutNV.exe 1.4.0.525)
/// Finds the child of `root` named `name` (`BSUtilities::GetObjectByName`,
/// `00c4b470`, with its flag 1). `this` is not used (the map names the body
/// `std::allocator<char>::deallocate`, which it is not).
pub fn fn_00571530(e: &mut Engine, _this: Ptr, root: u32, name: u32) -> u32 {
    e.call(0x00c4_b470, &args![root, name, 1u32]).u32()
}

// Translated from 00571550 (decompiled, FalloutNV.exe 1.4.0.525)
/// The global at `011c61a4` (the name of the node the skinned models hang
/// under).
pub fn fn_00571550(e: &mut Engine) -> u32 {
    e.global::<u32>(0x011c_61a4)
}

// Translated from 00571560 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::ClearInterpolators` (Xbox PDB): with an animation
/// (virtual `+0x1e4`), frees the special idle (`00498910`), clears the
/// animation group `0x14` (`00496080`), deactivates all the controllers of
/// its controller manager (`00496940`, `0048fef0`), clears the
/// controllers' interpolators (`00499160`) and the blend interpolators
/// (`00499080`).
pub fn tes_object_refr_clear_interpolators(e: &mut Engine, this: Ptr<TESObjectREFR>) {
    let me = this.addr();
    let animation = e.vcall(me, 0x1e4, &args![]).u32();
    if animation == 0 {
        return;
    }
    e.call(0x0049_8910, &args![animation, 1u32, 0u32]);
    e.call(0x0049_6080, &args![animation, 0x14u32, 0.0f32]);
    let manager = e.call(0x0049_6940, &args![animation]).u32();
    e.call(0x0048_fef0, &args![manager, 0.0f32]);
    let controllers = e.call(0x0055_85e0, &args![animation]).u32();
    e.call(0x0049_9160, &args![controllers]);
    e.call(0x0049_9080, &args![animation]);
}

// Translated from 005715d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::GetModel` (Xbox PDB): the path of the reference's
/// `TESModel` (its virtual at `+0x14`), 0 without a model.
pub fn tes_object_refr_get_model(e: &mut Engine, this: Ptr<TESObjectREFR>) -> u32 {
    let model = tes_object_refr_get_tes_model(e, this);
    if model == 0 {
        return 0;
    }
    e.vcall(model, 0x14, &args![]).u32()
}

// Translated from 00571600 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::GetModelBoundSize` (Xbox PDB): `GetBoundSize` of the
/// base object (`0050ebf0`), 0.0 without one.
pub fn tes_object_refr_get_model_bound_size(e: &mut Engine, this: Ptr<TESObjectREFR>) -> f32 {
    if base_form(e, this.addr()) == 0 {
        return 0.0;
    }
    let base = base_form(e, this.addr());
    e.call(0x0050_ebf0, &args![base]).f32()
}

// Translated from 00571630 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::GetTESModel` (Xbox PDB): the model of the reference.
/// First the model swap of its extra list (`0042e250`). Without one: a
/// weapon base (form type `0x28`) gives its model, the one of the weapon
/// mods when the extra list has weapon-mod flags (`00522df0` with those
/// flags and 0, else `00522d80`); any other base gives the `TESBoundObject`
/// dynamic cast of itself to the type at `011831e8`. Still none: a base
/// with a biped model (`00480db0`) gives the world model for the sex of
/// the reference's owner when that is an NPC (`00481110`).
pub fn tes_object_refr_get_tes_model(e: &mut Engine, this: Ptr<TESObjectREFR>) -> u32 {
    let me = this.addr();
    let extra = extra_list(e, me);
    let mut model = e.call(0x0042_e250, &args![extra]).u32();
    if model == 0 && base_form(e, me) != 0 {
        let base = base_form(e, me);
        if form_type(e, base) == 0x28 {
            let weapon = base_form(e, me);
            let extra = extra_list(e, me);
            if e.call(0x0042_e560, &args![extra]).bool() {
                let extra = extra_list(e, me);
                let flags = e.call(0x0042_e560, &args![extra]).u8();
                model = e
                    .call(0x0052_2df0, &args![weapon, u32::from(flags), 0u32])
                    .u32();
            } else {
                model = e.call(0x0052_2d80, &args![weapon]).u32();
            }
        } else {
            let base = base_form(e, me);
            model = e
                .call(
                    0x00ec_43fb,
                    &args![base, 0u32, 0x0118_3108u32, 0x0118_31e8u32, 0u32],
                )
                .u32();
        }
    }
    if model == 0 {
        let base = base_form(e, me);
        let biped_form = e.call(0x0048_0db0, &args![base]).u32();
        if biped_form != 0 {
            let mut sex = 0u32;
            let owner = e.call(0x0056_7790, &args![me]).u32();
            if owner != 0 && form_type(e, owner) == 0x2a {
                sex = e.call(0x005f_0cc0, &args![owner]).u32();
            }
            model = e.call(0x0048_1110, &args![biped_form, sex]).u32();
        }
    }
    model
}

/// Index into the table of weapon-animation group numbers at `0118a838`
/// for the item `item`: the signed byte at `+0xf4` (`00446390`) selects the
/// entry.
fn animation_type_entry(e: &mut Engine, item: u32) -> u32 {
    let index = e.call(0x0044_6390, &args![item]).u32() as i32;
    e.mem
        .u32(0x0118_a838u32.wrapping_add((index as u32).wrapping_mul(4)))
}

// Translated from 00571760 (decompiled, FalloutNV.exe 1.4.0.525)
/// Equips the biped model of `item` (an armor or weapon form; the biped
/// part is at `item + 0x3c`) on the reference. Does nothing without a 3D.
///
/// * An actor whose process allows it (virtual `+0x454` on the process)
///   is told virtual `+0x458` (`actor, 0`).
/// * The player's first-person biped gets `004ab750(biped, item, slots)`
///   with the mod slots (`004bd820`) of the equipped item and, when the
///   item has mod effect `0xe` active (`004bd8d0`), the user-interface
///   hook `00709c20` runs for the weapon's `00504e60`.
/// * For another reference the weapon animation group of the item is loaded
///   when its animation lacks it (`005f2370`, `00494710`, the KF file list
///   `00447330` and `00490400`).
/// * Without a biped the model is loaded and attached (`004aeed0`) and the
///   process is told virtual `+0x1cc` (drawn state, biped, animation,
///   actor).
/// * An actor then reloads its targets (without a biped), refreshes its
///   lighting property, alpha and weapon condition effects (`008b0bd0`,
///   `008c4640`, `00891190`), and, for an item in a biped model list
///   (`00475020`) that has a list node, tells its process virtual `+0x468`.
///
/// The string "Lily" in the reference's name (`0055d520`) prints a debug
/// message.
pub fn fn_00571760(e: &mut Engine, this: Ptr<TESObjectREFR>, item: u32) {
    let me = this.addr();
    if get_3d(e, me) == 0 {
        return;
    }
    let name = e.call(0x0055_d520, &args![me]).u32();
    if e.call(0x00ec_7750, &args![name, 0x0102_fdc8u32]).u32() != 0 {
        e.call(0x005b_5e40, &args![0x0103_0e38u32]);
    }
    let mut has_effect = false;
    let mut slots = 0u32;
    let mut actor = 0u32;
    if is_actor(e, me) {
        actor = me;
    }
    let loading = e.global::<u32>(GLOBAL_LOADING_OBJECT);
    if !e.call(0x0042_ce10, &args![loading]).bool() {
        let process = e.call(GET_PROCESS, &args![actor]).u32();
        if e.vcall(process, 0x454, &args![]).bool() {
            let process = e.call(GET_PROCESS, &args![actor]).u32();
            e.vcall(process, 0x458, &args![actor, 0u32]);
        }
    }
    let player = e.global::<u32>(GLOBAL_PLAYER);
    let mut biped = if me == player {
        e.call(0x0095_0b00, &args![player, 1u32]).u32()
    } else {
        e.vcall(me, 0x1e8, &args![]).u32()
    };
    if me == e.global::<u32>(GLOBAL_PLAYER) {
        let process = e.call(GET_PROCESS, &args![player]).u32();
        let worn = e.vcall(process, 0x148, &args![]).u32();
        if worn != 0 {
            slots = u32::from(e.call(0x004b_d820, &args![worn]).u8());
            // the float the effect check fills (ebp-0x10), starting at 0.0
            has_effect = e.with_stack(4, |e, value| {
                e.call(0x004b_d8d0, &args![worn, 0xeu32, value]).bool()
            });
        }
        if biped != 0 {
            e.call(0x004a_b750, &args![biped, item, slots]);
        }
        biped = e.call(0x0095_0b00, &args![player, 0u32]).u32();
        if e.call(0x004a_d010, &args![item]).bool()
            && me == e.global::<u32>(GLOBAL_PLAYER)
            && (!e.call(0x004a_d030, &args![item]).bool() || has_effect)
        {
            let hook = e.call(0x0050_4e60, &args![item]).u32();
            e.call(0x0070_9c20, &args![hook]);
        }
    } else {
        let animation = e.vcall(me, 0x1e4, &args![]).u32();
        if animation != 0 {
            let animation = e.vcall(me, 0x1e4, &args![]).u32();
            if e.call(0x0049_6940, &args![animation]).u32() != 0 {
                let entry = animation_type_entry(e, item);
                let group = e
                    .call(0x005f_2370, &args![0u32, entry, 0x18u32, 0u32])
                    .u16();
                let animation = e.vcall(me, 0x1e4, &args![]).u32();
                if !e
                    .call(0x0049_4710, &args![animation, u32::from(group)])
                    .bool()
                    && !e.vcall(me, 0x22c, &args![1u32]).bool()
                {
                    let entry = animation_type_entry(e, item);
                    let path = tes_object_refr_get_model(e, this);
                    let loader = model_loader(e);
                    let list = e
                        .call(0x0044_7330, &args![loader, path, 1u32, 0u32, entry])
                        .u32();
                    let animation = e.vcall(me, 0x1e4, &args![]).u32();
                    e.call(0x0049_0400, &args![animation, list]);
                }
            }
        }
    }
    if biped != 0 {
        e.call(0x004a_b750, &args![biped, item, slots]);
    } else {
        // the biped part of the item is embedded at +0x3c
        let part = if item != 0 { item + 0x3c } else { 0 };
        if part != 0 {
            e.call(0x004a_eed0, &args![item, part, 5u32, me, 0u32]);
            let loading = e.global::<u32>(GLOBAL_LOADING_OBJECT);
            if !e.call(0x0042_ce10, &args![loading]).bool() {
                let mut drawn = e.call(0x008a_16d0, &args![actor]).u8();
                if drawn == 0 && e.call(0x0046_e8c0, &args![item]).bool() {
                    drawn = 1;
                }
                let process = e.call(GET_PROCESS, &args![actor]).u32();
                let animation = e.vcall(actor, 0x1e4, &args![]).u32();
                e.vcall(
                    process,
                    0x1cc,
                    &args![u32::from(drawn), biped, animation, actor],
                );
            }
        }
    }
    if actor != 0 {
        if biped == 0 {
            e.call(0x008b_0b00, &args![actor, 1u32]);
        }
        let node = get_3d(e, me);
        e.call(0x008b_0bd0, &args![actor, node]);
        e.call(0x008c_4640, &args![actor]);
        e.call(0x0089_1190, &args![actor, 1u32]);
        let list = e.call(0x0047_5020, &args![item]).u32();
        if list != 0 && e.call(LIST_NEXT, &args![list]).u32() != 0 {
            let process = e.call(GET_PROCESS, &args![actor]).u32();
            e.vcall(process, 0x468, &args![1u32]);
        }
    }
}

/// Whether the node list `list` holds an entry whose name equals the string
/// at `01030e44` (the loops of `RemoveWeapon`).
fn list_has_named_entry(e: &mut Engine, list: u32) -> bool {
    let mut index = 0u32;
    while index < e.call(0x0043_b480, &args![list]).u32() {
        let item = e.call(0x0043_b4a0, &args![list, index]).u32();
        if item != 0 {
            let text = e.call(0x0041_3f40, &args![item]).u32();
            if e.call(0x0043_b1b0, &args![text]).u32() != 0 {
                let text = e.call(0x0041_3f40, &args![item]).u32();
                let text = e.call(0x0043_b1b0, &args![text]).u32();
                if e.call(0x0040_8b20, &args![text, 0x0103_0e44u32]).u32() == 0 {
                    return true;
                }
            }
        }
        index += 1;
    }
    false
}

/// Takes `object` out of the shadow scene node and clears its attached
/// effects (`00b5b1c0`, `00572160`).
fn remove_from_scene(e: &mut Engine, object: u32) {
    let scene = e.call(0x0045_0b80, &args![0u32]).u32();
    e.call(0x00b5_b1c0, &args![scene, object]);
    fn_00572160(e, Ptr::new(object));
}

// Translated from 00571b50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::RemoveWeapon` (Xbox PDB): takes the weapon of an actor
/// out of its 3D. Does nothing without a 3D.
///
/// * The worn weapon's inventory entry (slot 5 of the container changes,
///   `004c8c10`) is looked up; the player first leaves iron sights
///   (`008bb650`) and VATS (`009c8950`), drops the biped weapon
///   (`004ab5b0`, with `00572220` around it for a non-first-person biped),
///   and hides the gun scope (`00709c40`, `00709ca0`).
/// * Without a biped the "Backpack" node is detached, the weapon node of the
///   process (virtual `+0x1a0`, or its named children `+0x1a4`, or the
///   node's "Weapon" child) is found and, when the worn form has a model,
///   `0045a5e0` is told its path.
/// * The weapon node, the "Weapon" child when the actor virtual `+0x21c`
///   says so, and the named list are taken out of the shadow scene node
///   (`remove_from_scene`).
/// * An actor reloads its targets, refreshes its lighting property and,
///   for a biped model list with a node, tells its process virtual `+0x468`.
pub fn tes_object_refr_remove_weapon(e: &mut Engine, this: Ptr<TESObjectREFR>) {
    let me = this.addr();
    if get_3d(e, me) == 0 {
        return;
    }
    let mut worn_form = 0u32;
    let extra = extra_list(e, me);
    let changes = e.call(0x0041_8520, &args![extra]).u32();
    if changes != 0 {
        let entry = e.call(0x004c_8c10, &args![changes, 5u32, 0u32]).u32();
        if entry != 0 {
            worn_form = e.call(0x0044_ddc0, &args![entry]).u32();
            e.call(0x0044_59e0, &args![entry, 1u32]);
        }
    }
    let player = e.global::<u32>(GLOBAL_PLAYER);
    let mut biped;
    if me == player {
        if e.call(0x008b_bc10, &args![player]).bool() && fn_005721e0(e, Ptr::new(player)) {
            e.call(0x008b_b650, &args![player, 0u32, 0u32, 0u32]);
        }
        biped = e.call(0x0095_0b00, &args![player, 1u32]).u32();
    } else {
        biped = e.vcall(me, 0x1e8, &args![]).u32();
    }
    let mut actor = 0u32;
    if is_actor(e, me) {
        actor = me;
    }
    if me == e.global::<u32>(GLOBAL_PLAYER) {
        biped = e.call(0x0095_0b00, &args![player, 1u32]).u32();
        e.call(0x009c_8950, &args![0x011f_2250u32, 0u32, 1u32]);
        if biped != 0 {
            e.call(0x004a_b5b0, &args![biped]);
        }
        biped = e.call(0x0095_0b00, &args![player, 0u32]).u32();
        if e.call(0x008b_bc10, &args![player]).bool() {
            e.call(0x0070_9c40, &args![0u32]);
        }
        e.call(0x0070_9ca0, &args![]);
    }
    let process = e.call(GET_PROCESS, &args![actor]).u32();
    if biped != 0 {
        fn_00572220(e, 1);
        e.call(0x004a_b5b0, &args![biped]);
        fn_00572220(e, 0);
    } else {
        let mut weapon_found = false;
        let node = node_of(e, me);
        let backpack = e.call(0x004a_ae30, &args![node, 0x0101_f5ccu32]).u32();
        if backpack != 0 {
            let parent = e.call(NODE_PARENT, &args![backpack]).u32();
            e.vcall(parent, 0xe8, &args![backpack]);
        }
        if process != 0 {
            let held = e.vcall(process, 0x1a0, &args![0u32]).u32();
            weapon_found = held != 0 && e.call(0x0045_3470, &args![held]).u32() != 0;
            if !weapon_found {
                let list = e.vcall(process, 0x1a4, &args![0u32]).u32();
                let mut found = false;
                if list != 0 {
                    found = list_has_named_entry(e, list);
                }
                if found {
                    weapon_found = list != 0 && e.call(0x0045_3470, &args![list]).u32() != 0;
                }
                if !weapon_found {
                    let node = node_of(e, me);
                    if node != 0 {
                        let weapon = e.call(0x004a_ae30, &args![node, 0x0101_3be8u32]).u32();
                        if weapon != 0 {
                            weapon_found = true;
                        }
                    }
                }
            }
        }
        if worn_form != 0 {
            // the TESModel part of the worn form is embedded at +0x3c
            let model = worn_form + 0x3c;
            if weapon_found {
                let path = e.vcall(model, 0x14, &args![]).u32();
                let loader = model_loader(e);
                e.call(0x0045_a5e0, &args![loader, path]);
            }
        }
    }
    if process != 0 {
        let held = e.vcall(process, 0x1a0, &args![0u32]).u32();
        if held != 0 {
            remove_from_scene(e, held);
        }
        if e.vcall(me, 0x21c, &args![]).bool() {
            let node = node_of(e, me);
            if node != 0 {
                let weapon = e.call(0x004a_ae30, &args![node, 0x0101_3be8u32]).u32();
                if weapon != 0 {
                    remove_from_scene(e, weapon);
                }
            }
        }
        let list = e.vcall(process, 0x1a4, &args![0u32]).u32();
        if list != 0 && list_has_named_entry(e, list) {
            remove_from_scene(e, list);
        }
    }
    if actor != 0 {
        let process = e.call(GET_PROCESS, &args![actor]).u32();
        let data_handler = e.global::<u32>(GLOBAL_DATA_HANDLER);
        let save_load = e.global::<u32>(GLOBAL_SAVE_LOAD);
        if biped == 0
            && process != 0
            && !e.call(0x0042_26e0, &args![data_handler]).bool()
            && !e.call(0x0047_c850, &args![save_load]).bool()
        {
            e.call(0x008b_0b00, &args![actor, 0u32]);
        }
        let node = get_3d(e, me);
        e.call(0x008b_0bd0, &args![actor, node]);
        if e.call(GET_PROCESS, &args![actor]).u32() != 0 {
            let list = e.call(0x0047_5020, &args![worn_form]).u32();
            if list != 0 && e.call(LIST_NEXT, &args![list]).u32() != 0 {
                let process = e.call(GET_PROCESS, &args![actor]).u32();
                e.vcall(process, 0x468, &args![1u32]);
            }
        }
    }
}

// Translated from 00572160 (decompiled, FalloutNV.exe 1.4.0.525)
/// Releases the attached effects of the node `this`: for each element of
/// the array at `+0x9c` (`00658930` counts them, `00877a30` indexes) whose
/// pointer is set, `0050f5a0` runs on it; then the array is emptied
/// (`NiTArray<NiPointer<BSTempEffect>>::RemoveAll`, `004dffa0`).
pub fn fn_00572160(e: &mut Engine, this: Ptr) {
    let array = this.addr() + 0x9c;
    let count = e.call(0x0065_8930, &args![array]).u32();
    let mut index = 0u32;
    while index < count {
        let slot = e.call(0x0087_7a30, &args![array, index]).u32();
        let effect = e.call(READ_FIRST_DWORD, &args![slot]).u32();
        if effect != 0 {
            e.call(0x0050_f5a0, &args![effect]);
        }
        index += 1;
    }
    e.call(0x004d_ffa0, &args![array]);
}

// Translated from 005721e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// True when either of the bytes at `+0x64f` and `+0x64d` of the player
/// (`this`) is set (the iron-sights state flags).
pub fn fn_005721e0(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u8(this.addr() + 0x64f) != 0 || e.mem.u8(this.addr() + 0x64d) != 0
}

// Translated from 00572220 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `value` in the byte at `011dcfa7` (cdecl). `RemoveWeapon` sets it
/// to 1 around the removal of the biped weapon.
pub fn fn_00572220(e: &mut Engine, value: u8) {
    e.set_global(0x011d_cfa7, value);
}

// Translated from 00572230 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::MarkAsPickedUp` (Xbox PDB): when the reference was
/// dropped by someone (`ExtraDataList::GetItemDropper`, `0041de00`), takes
/// it out of that dropper's list of dropped items (`RemoveDroppedItem`,
/// `0041e0d0`); then `MarkAsDeleted` (`00572270`).
pub fn tes_object_refr_mark_as_picked_up(e: &mut Engine, this: Ptr<TESObjectREFR>) {
    let me = this.addr();
    let extra = extra_list(e, me);
    let dropper = e.call(0x0041_de00, &args![extra]).u32();
    if dropper != 0 {
        let dropper_extra = extra_list(e, dropper);
        e.call(0x0041_e0d0, &args![dropper_extra, me]);
    }
    e.call(0x0057_2270, &args![me]);
}

/// This part's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x0056ea40, fn_0056ea40(Ptr) -> Ptr),
        entry!(
            0x0056ea60,
            bhk_sphere_shape_scalar_deleting_destructor(Ptr, u32) -> Ptr
        ),
        entry!(0x0056ea90, fn_0056ea90(Ptr)),
        entry!(0x0056eab0, fn_0056eab0(Ptr<TESObjectREFR>)),
        entry!(0x0056f0f0, fn_0056f0f0(Ptr, u32)),
        entry!(0x0056f110, fn_0056f110(Ptr, u32)),
        entry!(0x0056f140, fn_0056f140(Ptr<TESObjectREFR>)),
        entry!(0x0056f690, fn_0056f690(Ptr) -> Ptr),
        entry!(0x0056f6b0, fn_0056f6b0(Ptr<TESObjectREFR>) -> bool),
        entry!(0x0056f700, fn_0056f700(Ptr<TESObjectREFR>)),
        entry!(
            0x0056f930,
            tes_object_refr_find_reference_for_3d(u32) -> u32
        ),
        entry!(
            0x0056fa00,
            tes_object_refr_get_orientation(Ptr<TESObjectREFR>, Ptr) -> Ptr
        ),
        entry!(
            0x0056fa90,
            tes_object_refr_set_orientation(
                Ptr<TESObjectREFR>,
                u32,
                u32,
                u32,
                u32,
                u32,
                u32,
                u32,
                u32,
                u32,
            )
        ),
        entry!(
            0x0056fac0,
            tes_object_refr_multiple_matrix_by_race(Ptr<TESObjectREFR>, Ptr, Ptr) -> Ptr
        ),
        entry!(0x0056fb10, fn_0056fb10(Ptr<TESObjectREFR>, Ptr) -> Ptr),
        entry!(0x0056fba0, fn_0056fba0(Ptr<TESObjectREFR>, Ptr) -> Ptr),
        entry!(
            0x0056fc30,
            tes_object_refr_get_bound_max(Ptr, u32) -> Ptr
        ),
        entry!(0x0056fcb0, fn_0056fcb0(Ptr<TESObjectREFR>)),
        entry!(
            0x005702e0,
            tes_object_refr_set_3d(Ptr<TESObjectREFR>, u32, bool)
        ),
        entry!(0x00570ea0, fn_00570ea0(Ptr) -> bool),
        entry!(0x00570ec0, fn_00570ec0(Ptr) -> bool),
        entry!(0x00570ee0, fn_00570ee0(Ptr) -> bool),
        entry!(0x00570f00, fn_00570f00(Ptr) -> bool),
        entry!(0x00570f40, fn_00570f40(Ptr) -> bool),
        entry!(0x00570f60, fn_00570f60() -> u32),
        entry!(0x00570f70, fn_00570f70(Ptr<TESObjectREFR>, u32)),
        entry!(
            0x00571080,
            tes_object_refr_set_3d_very_simple(Ptr<TESObjectREFR>, u32)
        ),
        entry!(0x005710c0, fn_005710c0(Ptr<TESObjectREFR>) -> bool),
        entry!(0x00571530, fn_00571530(Ptr, u32, u32) -> u32),
        entry!(0x00571550, fn_00571550() -> u32),
        entry!(
            0x00571560,
            tes_object_refr_clear_interpolators(Ptr<TESObjectREFR>)
        ),
        entry!(
            0x005715d0,
            tes_object_refr_get_model(Ptr<TESObjectREFR>) -> u32
        ),
        entry!(
            0x00571600,
            tes_object_refr_get_model_bound_size(Ptr<TESObjectREFR>) -> f32
        ),
        entry!(
            0x00571630,
            tes_object_refr_get_tes_model(Ptr<TESObjectREFR>) -> u32
        ),
        entry!(0x00571760, fn_00571760(Ptr<TESObjectREFR>, u32)),
        entry!(
            0x00571b50,
            tes_object_refr_remove_weapon(Ptr<TESObjectREFR>)
        ),
        entry!(0x00572160, fn_00572160(Ptr)),
        entry!(0x005721e0, fn_005721e0(Ptr) -> bool),
        entry!(0x00572220, fn_00572220(u8)),
        entry!(
            0x00572230,
            tes_object_refr_mark_as_picked_up(Ptr<TESObjectREFR>)
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn ret(value: u32) -> Ret {
        Ret {
            eax: value,
            ..Ret::default()
        }
    }

    /// A double that returns `value` in `eax`.
    fn returns(e: &mut Engine, addr: u32, value: u32) {
        e.register_double(addr, move |_, _| ret(value));
    }

    fn calls_to(log: &[(u32, Vec<u32>)], addr: u32) -> Vec<Vec<u32>> {
        log.iter()
            .filter(|(a, _)| *a == addr)
            .map(|(_, args)| args.clone())
            .collect()
    }

    fn addresses(log: &[(u32, Vec<u32>)]) -> Vec<u32> {
        log.iter().map(|(a, _)| *a).collect()
    }

    /// Every function address this file calls (the `0x0123_4567` literals of
    /// its own source), so a test only has to say what matters.
    fn called_addresses() -> Vec<u32> {
        let source = include_str!("tesobjectrefr_p4.rs");
        let bytes = source.as_bytes();
        let mut found = Vec::new();
        let mut i = 0;
        while i + 11 <= bytes.len() {
            if &bytes[i..i + 2] == b"0x" && bytes[i + 6] == b'_' {
                let (Some(high), Some(low)) = (source.get(i + 2..i + 6), source.get(i + 7..i + 11))
                else {
                    i += 1;
                    continue;
                };
                let text = [high, low].concat();
                if let Ok(value) = u32::from_str_radix(&text, 16) {
                    if (0x0040_0000..0x0100_0000).contains(&value) {
                        found.push(value);
                    }
                }
            }
            i += 1;
        }
        found.sort_unstable();
        found.dedup();
        found
    }

    /// An engine with the small accessors registered with the bodies the exe
    /// gives them, every other callee of this file stubbed out (returns 0)
    /// and the pages of the globals mapped.
    fn engine() -> Engine {
        let mut e = Engine::new();
        // every other callee of this file does nothing and returns 0 (the real
        // translations of other units must not run), except the functions of
        // this file itself
        let mine: Vec<u32> = funcs().into_iter().map(|(addr, _)| addr).collect();
        for addr in called_addresses() {
            if !mine.contains(&addr) {
                e.register(addr, |_, _| Ret::default());
            }
        }
        e.register(GET_EXTRA_LIST, |_, a| ret(a[0] + 0x44));
        e.register(GET_BASE_FORM, |e, a| ret(e.mem.u32(a[0] + 0x20)));
        e.register(FORM_TYPE, |e, a| ret(u32::from(e.mem.u8(a[0] + 4))));
        e.register(GET_PARENT_CELL, |e, a| ret(e.mem.u32(a[0] + 0x40)));
        e.register(GET_3D, |e, a| {
            let loaded = e.mem.u32(a[0] + 0x64);
            ret(if loaded != 0 {
                e.mem.u32(loaded + 0x14)
            } else {
                0
            })
        });
        e.register(READ_FIRST_DWORD, |e, a| ret(e.mem.u32(a[0])));
        e.register(LIST_NEXT, |e, a| ret(e.mem.u32(a[0] + 4)));
        e.register(NOTHING_CONSTRUCTOR, |_, a| ret(a[0]));
        e.register(NODE_PARENT, |e, a| ret(e.mem.u32(a[0] + 0x18)));
        e.register(GET_PROCESS, |e, a| ret(e.mem.u32(a[0] + 0x68)));
        e.register(FORM_IS_DELETED, |e, a| {
            ret(u32::from(e.mem.u32(a[0] + 8) & 0x20 != 0))
        });
        e.register(FORM_IS_DISABLED, |e, a| {
            ret(u32::from(e.mem.u32(a[0] + 8) & 0x800 != 0))
        });
        // NiPointer constructor from a pointer and assignment: both store it
        e.register(0x0063_3c90, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            ret(a[0])
        });
        e.register(0x0066_b0d0, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            ret(a[0])
        });
        e.register(NI_OPERATOR_NEW, |e, a| ret(e.mem.alloc(a[0])));
        e.register(HAVOK_ALLOCATE, |e, a| ret(e.mem.alloc(a[0])));
        for page in [
            0x0101_2000,
            0x0101_3000,
            0x011b_1000,
            0x0118_a000,
            0x011c_3000,
            0x011c_6000,
            0x011c_a000,
            0x011d_c000,
            0x011d_d000,
            0x011d_e000,
            0x011d_f000,
            0x011f_4000,
        ] {
            e.map(page, 0x1000);
        }
        e
    }

    /// A zeroed object of `size` bytes whose vtable has the given
    /// `(byte offset, value)` slots: each slot is a double returning
    /// `value`, registered at the address of the slot itself (see
    /// [`slot`]).
    fn object(e: &mut Engine, size: u32, slots: &[(u32, u32)]) -> u32 {
        let object = e.mem.alloc(size);
        let vtable = e.mem.alloc(0x800);
        for (offset, value) in slots {
            let value = *value;
            e.register_double(vtable + offset, move |_, _| ret(value));
            e.mem.set_u32(vtable + offset, vtable + offset);
        }
        e.mem.set_u32(object, vtable);
        object
    }

    /// The address a virtual call of `slot` on `object` lands on (what the
    /// call log names).
    fn slot(e: &Engine, object: u32, slot: u32) -> u32 {
        e.mem.u32(object) + slot
    }

    /// A reference (zeroed `0x700` bytes, so actor fields exist) whose
    /// `IsActor` answer is `actor` and whose other virtual slots are the
    /// given ones.
    fn reference(e: &mut Engine, actor: bool, slots: &[(u32, u32)]) -> u32 {
        let mut all = vec![(0x100, u32::from(actor))];
        all.extend_from_slice(slots);
        object(e, 0x700, &all)
    }

    /// A form object with a type byte and flags.
    fn form(e: &mut Engine, kind: u8, flags: u32) -> u32 {
        let form = e.mem.alloc(0x100);
        e.mem.set_u8(form + 4, kind);
        e.mem.set_u32(form + 8, flags);
        form
    }

    /// Gives the reference a loaded-data block whose 3D slot holds `node`.
    fn set_node(e: &mut Engine, refr: u32, node: u32) {
        let loaded = e.mem.alloc(0x40);
        e.mem.set_u32(loaded + 0x14, node);
        e.mem.set_u32(refr + 0x64, loaded);
    }

    /// Runs `body` with the call log on and returns the log.
    fn logged(e: &mut Engine, body: impl FnOnce(&mut Engine)) -> Vec<(u32, Vec<u32>)> {
        e.call_log = Some(vec![]);
        body(e);
        e.call_log.take().unwrap()
    }

    fn float_bits(value: f32) -> u32 {
        value.to_bits()
    }

    // ---- 0056ea40, 0056ea60, 0056ea90 ------------------------------------

    #[test]
    fn constructor_wrapper_0056ea40_runs_the_base_constructor_and_returns_this() {
        let mut e = engine();
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0056_ea40, &args![0x1234u32]).u32(), 0x1234);
        });
        assert_eq!(calls_to(&log, 0x0056_e8d0), vec![vec![0x1234]]);
    }

    #[test]
    fn sphere_shape_scalar_deleting_destructor_frees_only_when_asked() {
        let mut e = engine();
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0056_ea60, &args![0x4000u32, 1u32]).u32(), 0x4000);
        });
        assert_eq!(calls_to(&log, 0x00ca_0660), vec![vec![0x4000]]);
        assert_eq!(calls_to(&log, NI_OPERATOR_DELETE), vec![vec![0x4000, 0x14]]);

        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0056_ea60, &args![0x4000u32, 2u32]).u32(), 0x4000);
        });
        assert_eq!(calls_to(&log, 0x00ca_0660).len(), 1);
        assert!(calls_to(&log, NI_OPERATOR_DELETE).is_empty());
    }

    #[test]
    fn destructor_wrapper_0056ea90_runs_the_base_destructor() {
        let mut e = engine();
        let log = logged(&mut e, |e| {
            e.call(0x0056_ea90, &args![0x1234u32]);
        });
        assert_eq!(calls_to(&log, 0x0056_d730), vec![vec![0x1234]]);
    }

    // ---- 0056eab0 ----------------------------------------------------------

    /// A reference in a cell with a world and a primitive extra of type
    /// `kind` (half extents 1.5, 2.5, 3.5), with the Havok callees standing in.
    /// Returns the engine, the reference, the world, the body, the node, and
    /// the cinfo words (filter, shape) seen when the body is built.
    #[allow(clippy::type_complexity)]
    fn collision_setup(kind: u32) -> (Engine, u32, u32, u32, u32, Rc<RefCell<Vec<u32>>>) {
        let mut e = engine();
        let node = object(&mut e, 0x100, &[]);
        let position = e.mem.alloc(0x10);
        let refr = reference(&mut e, false, &[(0x1d0, node), (0x1f4, position)]);
        let cell = e.mem.alloc(0x40);
        e.mem.set_u32(refr + 0x40, cell);
        let world = e.mem.alloc(0x40);
        returns(&mut e, 0x0045_43c0, world);
        let primitive = e.mem.alloc(0x40);
        e.mem.set_u32(primitive + 4, kind);
        e.mem.set_f32(primitive + 0x18, 1.5);
        e.mem.set_f32(primitive + 0x1c, 2.5);
        e.mem.set_f32(primitive + 0x20, 3.5);
        returns(&mut e, GET_PRIMITIVE, primitive);
        // 00413fc0: copies the three floats at +0x18 to its argument
        e.register(0x0041_3fc0, |e, a| {
            copy_words(e, a[1], a[0] + 0x18, 3);
            ret(a[1])
        });
        // 004a3e90: a length scaled by 2
        e.register(0x004a_3e90, |_, a| Ret {
            st0: f64::from(f32::from_bits(a[0])) * 2.0,
            ..Ret::default()
        });
        e.register(0x0056_e610, |_, a| ret(a[0]));
        e.register(0x0056_e950, |_, a| ret(a[0]));
        e.register(0x004a_e750, |_, a| ret(a[0] + 8));
        e.register(0x00c4_2f70, |_, a| ret(a[0]));
        e.register(0x0056_d580, |_, a| ret(a[0]));
        let body = object(&mut e, 0x40, &[(0xdc, 0), (0xe0, 0), (0x9c, 0)]);
        // 0056d380: the body; the cinfo words at that moment are recorded
        let seen = Rc::new(RefCell::new(Vec::new()));
        let record = seen.clone();
        e.register_double(0x0056_d380, move |e, a| {
            let mut seen = record.borrow_mut();
            for offset in [0u32, 4, 0x20, 0x24] {
                seen.push(e.mem.u32(a[1] + offset));
            }
            ret(body)
        });
        (e, refr, world, body, node, seen)
    }

    #[test]
    fn collision_of_a_sphere_primitive_builds_a_sphere_body_in_the_world() {
        let (mut e, refr, world, body, node, seen) = collision_setup(2);
        let log = logged(&mut e, |e| {
            e.call(0x0056_eab0, &args![refr]);
        });
        let spheres = calls_to(&log, 0x0056_e950);
        assert_eq!(spheres.len(), 1);
        // the radius is the scaled first half extent, the second argument 0
        assert_eq!(spheres[0][1], float_bits(3.0));
        assert_eq!(spheres[0][2], 0);
        assert!(calls_to(&log, 0x0056_e610).is_empty());
        let sphere = spheres[0][0];
        assert_eq!(calls_to(&log, NI_OPERATOR_NEW)[0], vec![0x14]);
        // the cinfo holds the shape (through 004ae750) and the default layer 3
        assert_eq!(
            *seen.borrow(),
            vec![0x3_0003, sphere + 8, 0x3_0003, sphere + 8]
        );
        let flags = calls_to(&log, 0x0051_9230);
        assert_eq!(flags.len(), 1);
        assert_eq!(flags[0][1..], [2, 1]);
        assert_eq!(
            calls_to(&log, NODE_ADD_EXTRA_DATA),
            vec![vec![node, flags[0][0]]]
        );
        // the body is told the world (virtual +0x9c) and tied to the node
        assert_eq!(
            calls_to(&log, slot(&e, body, 0x9c)),
            vec![vec![body, world]]
        );
        assert_eq!(calls_to(&log, 0x00c8_5c10), vec![vec![body, node, 0]]);
        assert_eq!(calls_to(&log, 0x00c6_b980).len(), 1);
        assert_eq!(calls_to(&log, 0x00c6_b980)[0][1], body);
        // the cinfo is destroyed last
        assert_eq!(log.last().unwrap().0, 0x0056_d730);
    }

    #[test]
    fn collision_of_box_primitives_converts_the_half_extents() {
        // type 3 (a plane) replaces the second half extent with the global at 01013ea4
        for (kind, expected) in [(1u32, [1.5f32, 2.5, 3.5]), (3u32, [1.5, 0.04, 3.5])] {
            let (mut e, refr, ..) = collision_setup(kind);
            e.set_global(0x0101_3ea4, 0.04f32);
            let converted = Rc::new(RefCell::new(Vec::new()));
            let record = converted.clone();
            e.register_double(CONVERT_VECTOR, move |e, a| {
                // the first conversion is the half extents (source = a[1])
                if record.borrow().is_empty() {
                    for offset in [0u32, 4, 8, 12] {
                        record.borrow_mut().push(e.mem.f32(a[1] + offset));
                    }
                }
                Ret::default()
            });
            let log = logged(&mut e, |e| {
                e.call(0x0056_eab0, &args![refr]);
            });
            assert_eq!(calls_to(&log, 0x0056_e610).len(), 1, "kind {kind}");
            assert!(calls_to(&log, 0x0056_e950).is_empty());
            let seen = converted.borrow();
            assert_eq!(seen[..3], expected, "kind {kind}");
        }
    }

    #[test]
    fn collision_uses_the_collision_data_layer_when_the_extra_has_one() {
        let (mut e, refr, _, _, _, seen) = collision_setup(2);
        let data = e.mem.alloc(8);
        e.mem.set_u32(data, 7);
        returns(&mut e, 0x0042_11a0, data);
        e.call(0x0056_eab0, &args![refr]);
        assert_eq!(seen.borrow()[0], 0x3_0007);
        assert_eq!(seen.borrow()[2], 0x3_0007);
    }

    #[test]
    fn collision_needs_a_primitive_and_a_world() {
        let (mut e, refr, ..) = collision_setup(2);
        returns(&mut e, GET_PRIMITIVE, 0);
        let log = logged(&mut e, |e| {
            e.call(0x0056_eab0, &args![refr]);
        });
        assert!(calls_to(&log, NI_OPERATOR_NEW).is_empty());

        let (mut e, refr, ..) = collision_setup(2);
        returns(&mut e, 0x0045_43c0, 0);
        let log = logged(&mut e, |e| {
            e.call(0x0056_eab0, &args![refr]);
        });
        assert!(calls_to(&log, NI_OPERATOR_NEW).is_empty());

        // no parent cell at all
        let (mut e, refr, ..) = collision_setup(2);
        e.mem.set_u32(refr + 0x40, 0);
        let log = logged(&mut e, |e| {
            e.call(0x0056_eab0, &args![refr]);
        });
        assert!(calls_to(&log, 0x0045_43c0).is_empty());
        assert!(calls_to(&log, NI_OPERATOR_NEW).is_empty());
    }

    // ---- 0056f0f0, 0056f110 ----------------------------------------------

    #[test]
    fn cinfo_filter_info_is_stored_twice() {
        let mut e = engine();
        let cinfo = e.mem.alloc(0xe0);
        e.call(0x0056_f0f0, &args![cinfo, 0x5_0015u32]);
        assert_eq!(e.mem.u32(cinfo), 0x5_0015);
        assert_eq!(e.mem.u32(cinfo + 0x20), 0x5_0015);
    }

    #[test]
    fn cinfo_shape_is_stored_twice_from_the_accessor() {
        let mut e = engine();
        e.register(0x004a_e750, |_, a| ret(a[0] + 0x10));
        let cinfo = e.mem.alloc(0xe0);
        e.call(0x0056_f110, &args![cinfo, 0x2000u32]);
        assert_eq!(e.mem.u32(cinfo + 4), 0x2010);
        assert_eq!(e.mem.u32(cinfo + 0x24), 0x2010);
        // a null shape
        e.register(0x004a_e750, |_, _| Ret::default());
        e.call(0x0056_f110, &args![cinfo, 0u32]);
        assert_eq!(e.mem.u32(cinfo + 4), 0);
        assert_eq!(e.mem.u32(cinfo + 0x24), 0);
    }

    // ---- 0056f140 ----------------------------------------------------------

    #[test]
    fn collision_wrapper_builds_the_bounding_volume_shape_and_adds_the_body() {
        let (mut e, refr, world, body, node, seen) = collision_setup(1);
        // allocation sizes of the Havok objects are logged by the engine's double
        e.register(0x00c9_ddb0, |_, a| ret(a[0]));
        e.register(0x00c9_d990, |_, a| ret(a[0]));
        e.set_global(0x011b_153c, 0.5f32);
        let converted = Rc::new(RefCell::new(Vec::new()));
        let record = converted.clone();
        e.register_double(CONVERT_VECTOR, move |e, a| {
            if record.borrow().is_empty() {
                for offset in [0u32, 4, 8] {
                    record.borrow_mut().push(e.mem.f32(a[1] + offset));
                }
            }
            Ret::default()
        });
        let log = logged(&mut e, |e| {
            e.call(0x0056_f140, &args![refr]);
        });
        assert_eq!(*converted.borrow(), vec![1.5, 2.5, 3.5]);
        let sizes: Vec<u32> = calls_to(&log, HAVOK_ALLOCATE)
            .iter()
            .map(|a| a[0])
            .collect();
        assert_eq!(sizes, vec![0x30, 0x10, 0x1c]);
        let boxes = calls_to(&log, 0x00c9_ddb0);
        assert_eq!(boxes.len(), 1);
        assert_eq!(boxes[0][2], float_bits(0.5));
        // the companion got its own vtable
        let companions = calls_to(&log, 0x0056_d610);
        assert_eq!(companions.len(), 1);
        assert_eq!(e.mem.u32(companions[0][0]), 0x0103_0dc0);
        // the wrapping shape is built from the box and the companion
        let wrappers = calls_to(&log, 0x00c9_d990);
        assert_eq!(wrappers.len(), 1);
        assert_eq!(wrappers[0][1], boxes[0][0]);
        assert_eq!(wrappers[0][2], companions[0][0]);
        // the cinfo: wrapping shape at +4, fixed filter 0x50015 (words 0 and 0x20 are
        // only set by 0056f0f0, so the hk copy stays 0)
        assert_eq!(seen.borrow()[0], 0x5_0015);
        assert_eq!(seen.borrow()[1], wrappers[0][0]);
        // position and rotation are set on the cinfo, and the body added to the world
        assert_eq!(calls_to(&log, 0x0056_d300).len(), 1);
        assert_eq!(calls_to(&log, 0x0056_d320).len(), 1);
        assert_eq!(calls_to(&log, 0x0056_d2c0), vec![vec![world, body]]);
        assert_eq!(calls_to(&log, 0x00c8_5c10), vec![vec![body, node, 0]]);
        assert_eq!(log.last().unwrap().0, 0x0056_d730);
    }

    #[test]
    fn collision_wrapper_needs_a_world() {
        let (mut e, refr, ..) = collision_setup(1);
        returns(&mut e, 0x0045_43c0, 0);
        let log = logged(&mut e, |e| {
            e.call(0x0056_f140, &args![refr]);
        });
        assert!(calls_to(&log, HAVOK_ALLOCATE).is_empty());
    }

    // ---- 0056f690, 0056f6b0 ----------------------------------------------

    #[test]
    fn companion_constructor_sets_its_vtable_after_the_base_constructor() {
        let mut e = engine();
        let object = e.mem.alloc(0x10);
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0056_f690, &args![object]).u32(), object);
        });
        assert_eq!(calls_to(&log, 0x0056_d610), vec![vec![object]]);
        assert_eq!(e.mem.u32(object), 0x0103_0dc0);
    }

    #[test]
    fn base_form_query_0056f6b0_asks_the_base_object() {
        let mut e = engine();
        let refr = e.mem.alloc(0x100);
        assert!(!e.call(0x0056_f6b0, &args![refr]).bool());

        let yes = object(&mut e, 0x40, &[(0xa0, 1)]);
        e.mem.set_u32(refr + 0x20, yes);
        assert!(e.call(0x0056_f6b0, &args![refr]).bool());

        let no = object(&mut e, 0x40, &[(0xa0, 0)]);
        e.mem.set_u32(refr + 0x20, no);
        assert!(!e.call(0x0056_f6b0, &args![refr]).bool());
    }

    // ---- 0056f700 ----------------------------------------------------------

    /// A reference in a cell for the 3D refresh: model loader at the global,
    /// base object of type `base_kind`.
    fn refresh_setup(actor: bool, base_kind: u8, flags: u32) -> (Engine, u32, u32) {
        let mut e = engine();
        let loader = e.mem.alloc(0x20);
        e.set_global(GLOBAL_MODEL_LOADER, loader);
        let tes = e.mem.alloc(0x20);
        e.set_global(GLOBAL_TES, tes);
        let loading = e.mem.alloc(0x20);
        e.set_global(GLOBAL_LOADING_OBJECT, loading);
        let refr = reference(&mut e, actor, &[(0x240, 0), (0x1c4, 0)]);
        let base = form(&mut e, base_kind, 0);
        e.mem.set_u32(refr + 0x20, base);
        e.mem.set_u32(refr + 8, flags);
        let cell = e.mem.alloc(0x40);
        e.mem.set_u32(refr + 0x40, cell);
        returns(&mut e, 0x0045_8be0, 0x77);
        (e, refr, loader)
    }

    #[test]
    fn refresh_queues_an_enabled_reference_without_3d_with_the_model_loader() {
        let (mut e, refr, loader) = refresh_setup(false, 0x1, 0);
        let log = logged(&mut e, |e| {
            e.call(0x0056_f700, &args![refr]);
        });
        let cell = e.mem.u32(refr + 0x40);
        let tes = e.global::<u32>(GLOBAL_TES);
        assert_eq!(calls_to(&log, 0x0045_8be0), vec![vec![tes, cell, 0, 1]]);
        assert_eq!(calls_to(&log, 0x0044_4850), vec![vec![loader, refr, 0x77]]);
        assert!(calls_to(&log, 0x0096_f400).is_empty());
        // no 3D: no node work
        assert!(calls_to(&log, NODE_UPDATE).is_empty());
    }

    #[test]
    fn refresh_removes_an_actor_from_the_temp_change_list_before_queueing() {
        let (mut e, refr, loader) = refresh_setup(true, 0x2a, 0);
        let log = logged(&mut e, |e| {
            e.call(0x0056_f700, &args![refr]);
        });
        let order = addresses(&log);
        let change = order.iter().position(|a| *a == 0x0096_f400).unwrap();
        let queue = order.iter().position(|a| *a == 0x0044_4850).unwrap();
        assert!(change < queue);
        assert_eq!(
            calls_to(&log, 0x0096_f400),
            vec![vec![OBJECT_PROCESS_LISTS, refr]]
        );
        assert_eq!(calls_to(&log, slot(&e, refr, 0x240)), vec![vec![refr]]);
        assert_eq!(calls_to(&log, 0x0044_4850)[0][0], loader);
    }

    #[test]
    fn refresh_skips_references_the_loader_knows_or_that_are_deleted_or_saved() {
        // already known to the model loader
        let (mut e, refr, _) = refresh_setup(false, 0x1, 0);
        returns(&mut e, 0x0044_5750, 1);
        let log = logged(&mut e, |e| {
            e.call(0x0056_f700, &args![refr]);
        });
        assert!(calls_to(&log, 0x0044_4850).is_empty());
        // deleted
        let (mut e, refr, _) = refresh_setup(false, 0x1, 0x20);
        let log = logged(&mut e, |e| {
            e.call(0x0056_f700, &args![refr]);
        });
        assert!(calls_to(&log, 0x0044_4850).is_empty());
        // being saved with a base object that is not of type 0x1e
        let (mut e, refr, _) = refresh_setup(false, 0x1, 0);
        returns(&mut e, 0x0044_4ed0, 1);
        let log = logged(&mut e, |e| {
            e.call(0x0056_f700, &args![refr]);
        });
        assert!(calls_to(&log, 0x0044_4850).is_empty());
        // ... but a base object of type 0x1e is exempt
        let (mut e, refr, _) = refresh_setup(false, 0x1e, 0);
        returns(&mut e, 0x0044_4ed0, 1);
        let log = logged(&mut e, |e| {
            e.call(0x0056_f700, &args![refr]);
        });
        assert_eq!(calls_to(&log, 0x0044_4850).len(), 1);
    }

    #[test]
    fn refresh_of_a_disabled_reference_sets_the_script_action_flag_unless_loading() {
        let (mut e, refr, _) = refresh_setup(false, 0x1, 0x800);
        let log = logged(&mut e, |e| {
            e.call(0x0056_f700, &args![refr]);
        });
        assert!(calls_to(&log, 0x0044_4850).is_empty());
        let extra = refr + 0x44;
        assert_eq!(calls_to(&log, 0x005a_c190), vec![vec![refr, extra]]);
        assert_eq!(calls_to(&log, 0x005a_c750), vec![vec![refr, extra, 0x1000]]);

        let (mut e, refr, _) = refresh_setup(false, 0x1, 0x800);
        returns(&mut e, 0x0042_ce10, 1);
        let log = logged(&mut e, |e| {
            e.call(0x0056_f700, &args![refr]);
        });
        assert!(calls_to(&log, 0x005a_c190).is_empty());
        assert!(calls_to(&log, 0x005a_c750).is_empty());
    }

    #[test]
    fn refresh_with_a_3d_resets_the_node_and_restores_havok_data() {
        // a non-actor: the saved Havok data is restored after the virtual
        let (mut e, refr, _) = refresh_setup(false, 0x1, 0);
        let node = object(&mut e, 0x100, &[]);
        set_node(&mut e, refr, node);
        returns(&mut e, 0x0044_5750, 1); // not queued again
        let log = logged(&mut e, |e| {
            e.call(0x0056_f700, &args![refr]);
        });
        let translate = calls_to(&log, NODE_UPDATE);
        assert_eq!(translate.len(), 1);
        assert_eq!(translate[0][0], node);
        let order = addresses(&log);
        let virtual_call = order
            .iter()
            .position(|a| *a == slot(&e, refr, 0x1c4))
            .unwrap();
        let restore = order.iter().position(|a| *a == 0x0042_2bc0).unwrap();
        assert!(virtual_call < restore);
        assert_eq!(calls_to(&log, 0x0042_2bc0), vec![vec![refr + 0x44, refr]]);
        assert!(calls_to(&log, 0x008b_6ae0).is_empty());

        // an actor: limbs are rebuilt first, then the critical stage and velocity
        let (mut e, refr, _) = refresh_setup(true, 0x2a, 0);
        let node = object(&mut e, 0x100, &[]);
        set_node(&mut e, refr, node);
        returns(&mut e, 0x0044_5750, 1);
        let log = logged(&mut e, |e| {
            e.call(0x0056_f700, &args![refr]);
        });
        let order = addresses(&log);
        let limbs = order.iter().position(|a| *a == 0x008b_6ae0).unwrap();
        let virtual_call = order
            .iter()
            .position(|a| *a == slot(&e, refr, 0x1c4))
            .unwrap();
        let critical = order.iter().position(|a| *a == 0x008a_1a70).unwrap();
        let velocity = order.iter().position(|a| *a == 0x008b_65f0).unwrap();
        assert!(limbs < virtual_call && virtual_call < critical && critical < velocity);
        assert_eq!(calls_to(&log, 0x0042_2bc0), vec![vec![refr + 0x44, refr]]);
    }

    #[test]
    fn refresh_without_3d_runs_the_virtual_only_for_a_primitive() {
        let (mut e, refr, _) = refresh_setup(false, 0x1, 0x20);
        let log = logged(&mut e, |e| {
            e.call(0x0056_f700, &args![refr]);
        });
        assert!(calls_to(&log, slot(&e, refr, 0x1c4)).is_empty());

        returns(&mut e, GET_PRIMITIVE, 0x5000);
        let log = logged(&mut e, |e| {
            e.call(0x0056_f700, &args![refr]);
        });
        assert_eq!(calls_to(&log, slot(&e, refr, 0x1c4)).len(), 1);
    }

    // ---- 0056f930 ----------------------------------------------------------

    /// A node with an owner (virtual +0x10) and a parent (+0x18).
    fn node_with_owner(e: &mut Engine, owner: u32, parent: u32) -> u32 {
        let node = object(e, 0x40, &[(0x10, owner)]);
        e.mem.set_u32(node + 0x18, parent);
        node
    }

    #[test]
    fn find_reference_for_3d_answers_the_player_for_the_first_person_root() {
        let mut e = engine();
        let player = e.mem.alloc(0x40);
        e.set_global(GLOBAL_PLAYER, player);
        let root = node_with_owner(&mut e, 0, 0);
        let leaf = node_with_owner(&mut e, 0, root);
        returns(&mut e, 0x0095_0bb0, root);
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0056_f930, &args![leaf]).u32(), player);
        });
        assert_eq!(calls_to(&log, 0x0095_0bb0), vec![vec![player, 1]]);
    }

    #[test]
    fn find_reference_for_3d_answers_the_reference_of_the_first_owned_ancestor() {
        let mut e = engine();
        let found = e.mem.alloc(0x40);
        let owner = e.mem.alloc(0x40);
        returns(&mut e, 0x009a_d610, found);
        let top = node_with_owner(&mut e, owner, 0);
        let leaf = node_with_owner(&mut e, 0, top);
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0056_f930, &args![leaf]).u32(), found);
        });
        // no player: the first-person root is not asked for
        assert!(calls_to(&log, 0x0095_0bb0).is_empty());
        assert_eq!(calls_to(&log, 0x009a_d610), vec![vec![owner]]);
        // nothing owns anything: 0
        let mut e = engine();
        let top = node_with_owner(&mut e, 0, 0);
        let leaf = node_with_owner(&mut e, 0, top);
        assert_eq!(e.call(0x0056_f930, &args![leaf]).u32(), 0);
        assert_eq!(e.call(0x0056_f930, &args![0u32]).u32(), 0);
    }

    #[test]
    fn find_reference_for_3d_looks_further_up_for_a_base_object_of_type_0x29() {
        let mut e = engine();
        let owner = e.mem.alloc(0x40);
        let outer_owner = e.mem.alloc(0x40);
        let inner = e.mem.alloc(0x40);
        let outer = e.mem.alloc(0x40);
        let base = form(&mut e, 0x29, 0);
        e.mem.set_u32(inner + 0x20, base);
        let plain_base = form(&mut e, 0x1, 0);
        e.mem.set_u32(outer + 0x20, plain_base);
        e.register_double(0x009a_d610, move |_, a| {
            ret(if a[0] == owner {
                inner
            } else if a[0] == outer_owner {
                outer
            } else {
                0
            })
        });
        let outer_node = node_with_owner(&mut e, outer_owner, 0);
        let leaf = node_with_owner(&mut e, owner, outer_node);
        assert_eq!(e.call(0x0056_f930, &args![leaf]).u32(), outer);
        // the parent has no owner: the inner reference stays
        let bare = node_with_owner(&mut e, 0, 0);
        let leaf = node_with_owner(&mut e, owner, bare);
        assert_eq!(e.call(0x0056_f930, &args![leaf]).u32(), inner);
    }

    // ---- 0056fa00 .. 0056fc30 ---------------------------------------------

    #[test]
    fn get_orientation_uses_the_three_angles_of_a_non_actor() {
        let mut e = engine();
        // FromEulerAnglesXYZ writes a recognisable matrix
        e.register(MATRIX_FROM_EULER, |e, a| {
            for i in 0..9 {
                e.mem.set_u32(a[0] + 4 * i, 0x100 + i);
            }
            ret(a[0])
        });
        let refr = reference(&mut e, false, &[]);
        e.mem.set_f32(refr + 0x24, 0.5);
        e.mem.set_f32(refr + 0x28, 1.5);
        e.mem.set_f32(refr + 0x2c, 2.5);
        let out = e.mem.alloc(0x30);
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0056_fa00, &args![refr, out]).u32(), out);
        });
        let call = &calls_to(&log, MATRIX_FROM_EULER)[0];
        assert_eq!(
            call[1..],
            [float_bits(0.5), float_bits(1.5), float_bits(2.5)]
        );
        for i in 0..9 {
            assert_eq!(e.mem.u32(out + 4 * i), 0x100 + i);
        }
    }

    #[test]
    fn get_orientation_of_an_actor_keeps_only_the_heading() {
        let mut e = engine();
        let refr = reference(&mut e, true, &[]);
        e.mem.set_f32(refr + 0x24, 0.5);
        e.mem.set_f32(refr + 0x28, 1.5);
        e.mem.set_f32(refr + 0x2c, 2.5);
        let out = e.mem.alloc(0x30);
        let log = logged(&mut e, |e| {
            e.call(0x0056_fa00, &args![refr, out]);
        });
        let call = &calls_to(&log, MATRIX_FROM_EULER)[0];
        assert_eq!(call[1..], [0, 0, float_bits(2.5)]);
    }

    #[test]
    fn set_orientation_converts_the_matrix_into_the_three_angles() {
        let mut e = engine();
        let refr = e.mem.alloc(0x80);
        let seen = Rc::new(RefCell::new(Vec::new()));
        let record = seen.clone();
        e.register_double(MATRIX_TO_EULER, move |e, a| {
            for i in 0..9 {
                record.borrow_mut().push(e.mem.u32(a[0] + 4 * i));
            }
            Ret::default()
        });
        let log = logged(&mut e, |e| {
            e.call(
                0x0056_fa90,
                &args![refr, 1u32, 2u32, 3u32, 4u32, 5u32, 6u32, 7u32, 8u32, 9u32],
            );
        });
        assert_eq!(*seen.borrow(), (1..=9).collect::<Vec<u32>>());
        let call = &calls_to(&log, MATRIX_TO_EULER)[0];
        assert_eq!(call[1..], [refr + 0x24, refr + 0x28, refr + 0x2c]);
    }

    #[test]
    fn multiple_matrix_by_race_copies_the_matrix_and_reads_the_base_form_type() {
        let mut e = engine();
        let refr = e.mem.alloc(0x80);
        let base = form(&mut e, 0x2a, 0);
        e.mem.set_u32(refr + 0x20, base);
        let input = e.mem.alloc(0x30);
        for i in 0..9 {
            e.mem.set_u32(input + 4 * i, 10 + i);
        }
        let out = e.mem.alloc(0x30);
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0056_fac0, &args![refr, out, input]).u32(), out);
        });
        assert_eq!(calls_to(&log, FORM_TYPE), vec![vec![base]]);
        for i in 0..9 {
            assert_eq!(e.mem.u32(out + 4 * i), 10 + i);
        }
    }

    /// A reference whose 3D node has a bound extra; the corner function
    /// writes (1, 2, 3) to its scratch argument.
    fn bound_setup(corner: u32) -> (Engine, u32, u32, u32) {
        let mut e = engine();
        returns(&mut e, 0x0050_ea90, 0x6000);
        let refr = e.mem.alloc(0x80);
        let node = e.mem.alloc(0x40);
        set_node(&mut e, refr, node);
        let bound = e.mem.alloc(0x40);
        e.register_double(NODE_GET_EXTRA_DATA, move |_, a| {
            ret(if a[1] == 0x6000 { bound } else { 0 })
        });
        e.register(corner, |e, a| {
            e.mem.set_f32(a[1], 1.0);
            e.mem.set_f32(a[1] + 4, 2.0);
            e.mem.set_f32(a[1] + 8, 3.0);
            Ret::default()
        });
        for i in 0..3 {
            e.mem.set_u32(ZERO_VECTOR + 4 * i, 0);
        }
        (e, refr, node, bound)
    }

    #[test]
    fn bound_corners_come_from_the_bound_extra_or_are_zero() {
        for (address, corner, scratch) in [
            (0x0056_fb10u32, 0x0050_ea10u32, SCRATCH_BOUND_MIN),
            (0x0056_fba0, 0x0050_ea50, SCRATCH_BOUND_MAX),
        ] {
            let (mut e, refr, _, bound) = bound_setup(corner);
            let out = e.mem.alloc(0x10);
            let log = logged(&mut e, |e| {
                assert_eq!(e.call(address, &args![refr, out]).u32(), out);
            });
            assert_eq!(calls_to(&log, corner), vec![vec![bound, scratch]]);
            assert_eq!(e.mem.f32(out), 1.0);
            assert_eq!(e.mem.f32(out + 4), 2.0);
            assert_eq!(e.mem.f32(out + 8), 3.0);

            // no bound extra: the zero vector
            e.mem.set_f32(ZERO_VECTOR, 9.0);
            e.register(NODE_GET_EXTRA_DATA, |_, _| Ret::default());
            let out = e.mem.alloc(0x10);
            e.call(address, &args![refr, out]);
            assert_eq!(e.mem.f32(out), 9.0);
            assert_eq!(e.mem.f32(out + 4), 0.0);

            // no 3D: the zero vector as well, without asking for the extra
            let bare = e.mem.alloc(0x80);
            let log = logged(&mut e, |e| {
                e.call(address, &args![bare, out]);
            });
            assert!(calls_to(&log, NODE_GET_EXTRA_DATA).is_empty());
        }
    }

    #[test]
    fn get_bound_max_takes_the_node_directly() {
        let (mut e, _, node, bound) = bound_setup(0x0050_ea50);
        let out = e.mem.alloc(0x10);
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0056_fc30, &args![out, node]).u32(), out);
        });
        assert_eq!(
            calls_to(&log, 0x0050_ea50),
            vec![vec![bound, SCRATCH_BOUND_MAX]]
        );
        assert_eq!(e.mem.f32(out + 8), 3.0);
        // a null node gives the zero vector
        e.mem.set_f32(ZERO_VECTOR, 7.0);
        let log = logged(&mut e, |e| {
            e.call(0x0056_fc30, &args![out, 0u32]);
        });
        assert_eq!(e.mem.f32(out), 7.0);
        assert!(calls_to(&log, NODE_GET_EXTRA_DATA).is_empty());
    }

    // ---- 0056fcb0 ----------------------------------------------------------

    /// A reference with a node and a base object of type `base_kind`; the
    /// global at 011dea10 is an object too.
    fn unload_setup(base_kind: u8) -> (Engine, u32, u32) {
        let mut e = engine();
        let tes = e.mem.alloc(0x100);
        e.set_global(GLOBAL_TES, tes);
        let node = object(&mut e, 0x40, &[]);
        let refr = reference(&mut e, false, &[(0x1d0, node)]);
        let base = form(&mut e, base_kind, 0);
        e.mem.set_u32(refr + 0x20, base);
        (e, refr, tes)
    }

    #[test]
    fn unload_cleanup_does_nothing_without_a_node() {
        let mut e = engine();
        let refr = reference(&mut e, false, &[(0x1d0, 0)]);
        let log = logged(&mut e, |e| {
            e.call(0x0056_fcb0, &args![refr]);
        });
        assert_eq!(log.len(), 2); // the call and the virtual
    }

    #[test]
    fn unload_cleanup_leaves_water_and_counts_type_0x23_down() {
        let (mut e, refr, tes) = unload_setup(0x23);
        let log = logged(&mut e, |e| {
            e.call(0x0056_fcb0, &args![refr]);
        });
        assert_eq!(calls_to(&log, 0x0045_ce60), vec![vec![tes]]);
        assert_eq!(calls_to(&log, 0x0056_5670), vec![vec![refr]]);
        assert_eq!(calls_to(&log, 0x0057_b520), vec![vec![refr, 0]]);
        // no parent cell: nothing about cells
        assert!(calls_to(&log, 0x0054_5360).is_empty());

        // another type: the counter is left alone
        let (mut e, refr, _) = unload_setup(0x1);
        let log = logged(&mut e, |e| {
            e.call(0x0056_fcb0, &args![refr]);
        });
        assert!(calls_to(&log, 0x0045_ce60).is_empty());
        assert_eq!(calls_to(&log, 0x0056_5670).len(), 1);
    }

    #[test]
    fn unload_cleanup_adds_the_room_data_to_each_other_room_item() {
        let (mut e, refr, _) = unload_setup(0x1);
        let other = e.mem.alloc(0x100);
        let lonely = e.mem.alloc(0x100);
        // the list: node 1 holds `other`, node 2 holds `lonely`
        let second = e.mem.alloc(0x10);
        e.mem.set_u32(second, lonely);
        let first = e.mem.alloc(0x10);
        e.mem.set_u32(first, other);
        e.mem.set_u32(first + 4, second);
        returns(&mut e, 0x0042_0770, first);
        let own_room = e.mem.alloc(0x20);
        let other_room = e.mem.alloc(0x20);
        e.register_double(0x0056_99b0, move |_, a| {
            ret(if a[0] == refr {
                own_room
            } else if a[0] == other {
                other_room
            } else {
                0
            })
        });
        returns(&mut e, 0x0066_29f0, 0x7777);
        returns(&mut e, 0x0045_c650, 0x8888);
        let log = logged(&mut e, |e| {
            e.call(0x0056_fcb0, &args![refr]);
        });
        let holders = calls_to(&log, 0x0063_3c90);
        assert_eq!(holders.len(), 1);
        assert_eq!(holders[0][1], 0x7777);
        let adds = calls_to(&log, 0x0057_c5d0);
        assert_eq!(adds.len(), 1);
        assert_eq!(adds[0][0], 0x8888);
        // the second local is the NiPointer built from the room data
        assert_eq!(adds[0][2], holders[0][0]);
        assert_eq!(
            calls_to(&log, 0x0045_cec0),
            vec![vec![adds[0][1]], vec![holders[0][0]]]
        );
        assert_eq!(calls_to(&log, 0x0066_29f0), vec![vec![own_room]]);
        assert_eq!(calls_to(&log, 0x0045_c650), vec![vec![other_room]]);
    }

    #[test]
    fn unload_cleanup_takes_a_portal_marker_out_of_its_two_rooms() {
        let (mut e, refr, _) = unload_setup(0x1);
        let portal_base = e.mem.u32(refr + 0x20);
        e.set_global(GLOBAL_PORTAL_MARKER_BASE, portal_base);
        let room = e.mem.alloc(0x100);
        e.register_double(0x0042_0bc0, move |_, a| {
            ret(if a[1] == 0 { room } else { 0 })
        });
        let log = logged(&mut e, |e| {
            e.call(0x0056_fcb0, &args![refr]);
        });
        assert_eq!(
            calls_to(&log, 0x0042_0bc0),
            vec![vec![refr + 0x44, 0], vec![refr + 0x44, 1]]
        );
        assert_eq!(calls_to(&log, 0x0042_0da0), vec![vec![room + 0x44, refr]]);
    }

    #[test]
    fn unload_cleanup_clears_the_water_lights_of_a_type_0x1e_base() {
        // the water node's scheduler runs a process of type 0xd (or not)
        for (process_type, updates_property) in [(0xdu32, true), (0x2, false)] {
            let (mut e, refr, tes) = unload_setup(0x1e);
            let water_node = object(&mut e, 0x40, &[]);
            let water = reference(&mut e, false, &[(0x1d0, water_node)]);
            let item = e.mem.alloc(0x10);
            e.mem.set_u32(item, water);
            returns(&mut e, 0x0041_f810, item);
            let stored = e.mem.alloc(0x10);
            e.mem.set_u32(stored, 0x5151);
            returns(&mut e, 0x0057_25f0, stored);
            let manager = e.mem.alloc(0x20);
            returns(&mut e, 0x0070_ec90, manager);
            let found = e.mem.alloc(0x100);
            returns(&mut e, 0x004e_8030, found);
            let scheduler = e.mem.alloc(0x100);
            e.mem.set_u32(scheduler + 0x68, process_type);
            returns(&mut e, 0x0040_30b0, scheduler);
            let property = e.mem.alloc(0x200);
            returns(&mut e, 0x00a5_9d30, property);
            let seen = Rc::new(RefCell::new(Vec::new()));
            let record = seen.clone();
            e.register_double(0x0057_c730, move |e, a| {
                record.borrow_mut().push((a[0], e.mem.u32(a[1])));
                Ret::default()
            });
            let log = logged(&mut e, |e| {
                e.call(0x0056_fcb0, &args![refr]);
            });
            assert_eq!(
                calls_to(&log, 0x0041_f840),
                vec![vec![water + 0x44, refr, 0]]
            );
            assert_eq!(calls_to(&log, 0x004e_8030), vec![vec![manager, water]]);
            assert_eq!(calls_to(&log, 0x0070_ec90)[0], vec![tes]);
            if updates_property {
                assert_eq!(calls_to(&log, 0x00a5_9d30), vec![vec![found, 3]]);
                assert_eq!(*seen.borrow(), vec![(property + 0x128, 0x5151)]);
            } else {
                assert!(calls_to(&log, 0x00a5_9d30).is_empty());
                assert!(seen.borrow().is_empty());
            }
        }
    }

    #[test]
    fn unload_cleanup_tells_the_acoustic_listener_about_a_type_0xe_base_in_a_cell() {
        let (mut e, refr, _) = unload_setup(0xe);
        let cell = e.mem.alloc(0x40);
        e.mem.set_u32(refr + 0x40, cell);
        returns(&mut e, 0x0045_43c0, 0x1111);
        returns(&mut e, 0x0045_cd60, 0x2222);
        let log = logged(&mut e, |e| {
            e.call(0x0056_fcb0, &args![refr]);
        });
        assert_eq!(calls_to(&log, 0x0045_43c0)[0], vec![cell]);
        assert_eq!(calls_to(&log, 0x0045_cd60)[0], vec![0x1111]);
        assert_eq!(calls_to(&log, 0x0061_f890), vec![vec![0x2222, refr, 1]]);
    }

    #[test]
    fn unload_cleanup_leaves_the_cell_lists_of_the_reference() {
        let (mut e, refr, tes) = unload_setup(0x23);
        let cell = e.mem.alloc(0x40);
        e.mem.set_u32(refr + 0x40, cell);
        returns(&mut e, 0x0045_43c0, 0x1111);
        returns(&mut e, 0x0059_bb30, 0x3333);
        returns(&mut e, 0x0070_ec90, 0x4444);
        let log = logged(&mut e, |e| {
            e.call(0x0056_fcb0, &args![refr]);
        });
        assert_eq!(calls_to(&log, 0x004e_52f0), vec![vec![0x4444, refr]]);
        assert_eq!(calls_to(&log, 0x0054_5360), vec![vec![cell, refr]]);
        assert_eq!(calls_to(&log, 0x0054_54f0), vec![vec![cell, refr]]);
        assert_eq!(calls_to(&log, 0x0059_bb30), vec![vec![0x1111]]);
        assert_eq!(calls_to(&log, 0x0063_1370), vec![vec![0x3333, refr, 0, 1]]);
        assert_eq!(calls_to(&log, 0x0054_56e0), vec![vec![cell, refr]]);
        assert_eq!(calls_to(&log, 0x0063_12d0), vec![vec![0x3333, refr]]);
        assert_eq!(calls_to(&log, 0x0045_ce60), vec![vec![tes]]);

        // no lighting object: 00631370 is skipped
        returns(&mut e, 0x0059_bb30, 0);
        let log = logged(&mut e, |e| {
            e.call(0x0056_fcb0, &args![refr]);
        });
        assert!(calls_to(&log, 0x0063_1370).is_empty());
    }

    #[test]
    fn unload_cleanup_updates_the_registry_for_room_multibound_and_portal_markers() {
        // a room marker: the room extra goes to the registry
        let (mut e, refr, _) = unload_setup(0x1);
        let cell = e.mem.alloc(0x40);
        e.mem.set_u32(refr + 0x40, cell);
        let base = e.mem.u32(refr + 0x20);
        e.set_global(GLOBAL_ROOM_MARKER_BASE, base);
        returns(&mut e, 0x009d_9f20, 0x5555);
        returns(&mut e, 0x0042_0ed0, 0x6666);
        let log = logged(&mut e, |e| {
            e.call(0x0056_fcb0, &args![refr]);
        });
        assert_eq!(calls_to(&log, 0x009d_9f20), vec![vec![cell]]);
        assert_eq!(calls_to(&log, 0x00c5_b060), vec![vec![0x5555, 0x6666]]);

        // a multibound marker without a linked reference
        let (mut e, refr, _) = unload_setup(0x1);
        let cell = e.mem.alloc(0x40);
        e.mem.set_u32(refr + 0x40, cell);
        let base = e.mem.u32(refr + 0x20);
        e.set_global(GLOBAL_MULTIBOUND_BASE, base);
        returns(&mut e, 0x009d_9f20, 0x5555);
        returns(&mut e, 0x0042_2120, 0x7070);
        let log = logged(&mut e, |e| {
            e.call(0x0056_fcb0, &args![refr]);
        });
        assert_eq!(calls_to(&log, 0x00c5_aec0), vec![vec![0x5555, 0x7070]]);
        assert!(calls_to(&log, 0x00c3_9ad0).is_empty());

        // ... with a linked reference that has a room
        let linked = e.mem.alloc(0x100);
        returns(&mut e, 0x0042_1e10, linked);
        returns(&mut e, 0x0042_0ed0, 0x6666);
        let log = logged(&mut e, |e| {
            e.call(0x0056_fcb0, &args![refr]);
        });
        assert_eq!(calls_to(&log, 0x00c3_9ad0), vec![vec![0x6666, 0x7070]]);
        assert_eq!(calls_to(&log, 0x0042_0ed0), vec![vec![linked + 0x44]]);
        assert!(calls_to(&log, 0x00c5_aec0).is_empty());

        // ... linked but without a room: the registry again
        returns(&mut e, 0x0042_0ed0, 0);
        let log = logged(&mut e, |e| {
            e.call(0x0056_fcb0, &args![refr]);
        });
        assert_eq!(calls_to(&log, 0x00c5_aec0), vec![vec![0x5555, 0x7070]]);
    }

    #[test]
    fn unload_cleanup_updates_both_rooms_of_a_portal_marker_in_a_cell() {
        let (mut e, refr, _) = unload_setup(0x1);
        let cell = e.mem.alloc(0x40);
        e.mem.set_u32(refr + 0x40, cell);
        let base = e.mem.u32(refr + 0x20);
        e.set_global(GLOBAL_PORTAL_MARKER_BASE, base);
        returns(&mut e, 0x009d_9f20, 0x5555);
        returns(&mut e, 0x0042_0dd0, 0x9999);
        // the portal joins one room (slot 0) and nothing (slot 1)
        let rooms = e.mem.alloc(0x10);
        let room_ref = e.mem.alloc(0x100);
        e.mem.set_u32(rooms, room_ref);
        returns(&mut e, 0x0042_0410, rooms);
        returns(&mut e, 0x0056_99b0, 0x1212);
        returns(&mut e, 0x006a_b360, 0x3434);
        let seen = Rc::new(RefCell::new(Vec::new()));
        let record = seen.clone();
        e.register_double(0x0057_c730, move |e, a| {
            record.borrow_mut().push((a[0], e.mem.u32(a[1])));
            Ret::default()
        });
        let log = logged(&mut e, |e| {
            e.call(0x0056_fcb0, &args![refr]);
        });
        assert_eq!(*seen.borrow(), vec![(0x3434, 0x9999)]);
        assert_eq!(calls_to(&log, 0x0097_07f0), vec![vec![0x9999, 0]]);
        // slot 1 is empty: the registry hears about the portal
        assert_eq!(calls_to(&log, 0x00c5_b190), vec![vec![0x5555, 0x9999]]);
        assert!(calls_to(&log, 0x0042_0ba0).is_empty());
    }

    // ---- 005702e0 and its tail 00570f70 -----------------------------------

    /// Everything `Set3D` reads: a reference (an actor when asked) with the
    /// 3D `old` under `parent`, a base object, the process of an actor.
    struct Scene {
        e: Engine,
        refr: u32,
        old: u32,
        parent: u32,
        base: u32,
        process: u32,
    }

    fn scene(actor: bool) -> Scene {
        let mut e = engine();
        for global in [
            GLOBAL_MODEL_LOADER,
            GLOBAL_TES,
            GLOBAL_LOADING_OBJECT,
            GLOBAL_SAVE_LOAD,
            GLOBAL_DATA_HANDLER,
            GLOBAL_PLAYER,
        ] {
            let object = e.mem.alloc(0x300);
            e.set_global(global, object);
        }
        let parent = object(&mut e, 0x100, &[(0xe8, 0), (0xdc, 0)]);
        let old = object(&mut e, 0x100, &[(0x10, 0), (0xc, 0x4242)]);
        e.mem.set_u32(old + 0x18, parent);
        let base = object(&mut e, 0x100, &[(0x150, 0)]);
        e.mem.set_u8(base + 4, 1);
        let process = object(
            &mut e,
            0x700,
            &[
                (0x79c, 0),
                (0x794, 0),
                (0x7a4, 0),
                (0x1a0, 0),
                (0x1a4, 0),
                (0x5f4, 0),
                (0x4fc, 0),
                (0x468, 0),
                (0x454, 0),
                (0x458, 0),
                (0x148, 0),
            ],
        );
        let refr = reference(
            &mut e,
            actor,
            &[
                (0x1d0, old),
                (0x15c, 1),
                (0x1f0, 0),
                (0x224, 0),
                (0xfc, 0),
                (0x4c, 0),
                (0x1c0, 0),
                (0x21c, 0),
                (0x1e8, 0),
            ],
        );
        e.mem.set_u32(refr + 0x20, base);
        if actor {
            e.mem.set_u32(refr + 0x68, process);
        }
        set_node(&mut e, refr, old);
        // the node accessors
        // 00891170: the OBJ_REFR data embedded at +0x20
        e.register(0x0089_1170, |_, a| ret(a[0] + 0x20));
        e.register(0x0043_c490, |_, a| ret(a[0] + 0x58));
        e.register(0x006a_9540, |_, a| ret(a[0] + 0x34));
        e.register(0x009c_dae0, |_, _| Ret {
            st0: 2.0,
            ..Ret::default()
        });
        e.register(0x0045_0b80, |_, _| ret(0x00d0_0d00));
        e.register(0x008c_71b0, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            ret(a[0])
        });
        Scene {
            e,
            refr,
            old,
            parent,
            base,
            process,
        }
    }

    #[test]
    fn set_3d_with_the_current_node_changes_nothing() {
        let mut s = scene(false);
        let (refr, old) = (s.refr, s.old);
        let log = logged(&mut s.e, |e| {
            e.call(0x0057_02e0, &args![refr, old, 0u32]);
        });
        assert!(calls_to(&log, CANCEL_REFERENCE).is_empty());
        assert!(calls_to(&log, 0x0057_9ac0).is_empty());
        assert_eq!(log.len(), 2); // the call and the 3D accessor
    }

    #[test]
    fn set_3d_of_a_reference_without_3d_to_null_cancels_the_loader_request() {
        let mut s = scene(false);
        let refr = s.refr;
        set_node(&mut s.e, refr, 0);
        let loader = model_loader(&s.e);
        let log = logged(&mut s.e, |e| {
            e.call(0x0057_02e0, &args![refr, 0u32, 0u32]);
        });
        assert_eq!(calls_to(&log, CANCEL_REFERENCE), vec![vec![loader, refr]]);
        assert_eq!(calls_to(&log, 0x0057_9ac0), vec![vec![refr, 0]]);
    }

    #[test]
    fn set_3d_on_a_bare_reference_stores_the_node_and_registers_the_obstacle() {
        let mut s = scene(false);
        let refr = s.refr;
        set_node(&mut s.e, refr, 0);
        let loaded = s.e.mem.alloc(0x40);
        s.e.register_double(0x0057_c180, move |e, a| {
            e.mem.set_u32(a[0] + 0x64, loaded);
            Ret::default()
        });
        let node = object(&mut s.e, 0x100, &[]);
        let log = logged(&mut s.e, |e| {
            e.call(0x0057_02e0, &args![refr, node, 0u32]);
        });
        // Set3DVerySimple stores the node in the loaded data
        assert_eq!(calls_to(&log, 0x0066_b0d0), vec![vec![loaded + 0x14, node]]);
        // the obstacle is added (by 00570f70 and by the tail of Set3D)
        assert_eq!(calls_to(&log, 0x006c_0c30).len(), 2);
        assert!(calls_to(&log, 0x006c_0c80).is_empty());
        // no old node: nothing is detached or transformed
        assert!(calls_to(&log, 0x0044_0460).is_empty());
        assert!(calls_to(&log, 0x00b5_b1c0).is_empty());
        assert!(calls_to(&log, CANCEL_REFERENCE).is_empty());
    }

    #[test]
    fn set_3d_replaces_the_node_under_the_same_parent_with_its_transform() {
        let mut s = scene(false);
        let (refr, old, parent) = (s.refr, s.old, s.parent);
        s.e.mem.set_f32(old + 0x58, 1.0);
        s.e.mem.set_f32(old + 0x5c, 2.0);
        s.e.mem.set_f32(old + 0x60, 3.0);
        for i in 0..9 {
            s.e.mem.set_u32(old + 0x34 + 4 * i, 101 + i);
        }
        let node = object(&mut s.e, 0x100, &[]);
        let translation = Rc::new(RefCell::new(Vec::new()));
        let rotation = Rc::new(RefCell::new(Vec::new()));
        let (t, r) = (translation.clone(), rotation.clone());
        s.e.register_double(0x0044_0460, move |e, a| {
            for i in 0..3 {
                t.borrow_mut().push(e.mem.f32(a[1] + 4 * i));
            }
            Ret::default()
        });
        s.e.register_double(0x0043_fa80, move |e, a| {
            for i in 0..9 {
                r.borrow_mut().push(e.mem.u32(a[1] + 4 * i));
            }
            Ret::default()
        });
        let log = logged(&mut s.e, |e| {
            e.call(0x0057_02e0, &args![refr, node, 0u32]);
        });
        // the old node leaves its parent (virtual +0xe8) and the shadow scene node
        assert_eq!(
            calls_to(&log, slot(&s.e, parent, 0xe8)),
            vec![vec![parent, old]]
        );
        assert_eq!(calls_to(&log, 0x00b5_b1c0), vec![vec![0x00d0_0d00, old]]);
        // the new node gets the remembered transform and the old parent
        assert_eq!(*translation.borrow(), vec![1.0, 2.0, 3.0]);
        assert_eq!(*rotation.borrow(), (101..110).collect::<Vec<u32>>());
        assert_eq!(
            calls_to(&log, 0x0044_0490),
            vec![vec![node, float_bits(2.0)]]
        );
        assert_eq!(
            calls_to(&log, slot(&s.e, parent, 0xdc)),
            vec![vec![parent, node, 1]]
        );
        assert_eq!(calls_to(&log, 0x00c6_bd00), vec![vec![node, 1]]);
        assert_eq!(calls_to(&log, 0x00a5_a040), vec![vec![node]]);
        assert_eq!(calls_to(&log, NODE_UPDATE)[0][0], node);
        // the old node is released, the obstacle removed then added
        assert_eq!(calls_to(&log, 0x0066_b0d0)[0][1], 0);
        assert_eq!(calls_to(&log, 0x006c_0c80).len(), 1);
        assert_eq!(calls_to(&log, 0x006c_0c30).len(), 2);
    }

    #[test]
    fn set_3d_leaves_a_3d_of_a_type_0x25_base_alone() {
        let mut s = scene(false);
        let (refr, base) = (s.refr, s.base);
        s.e.mem.set_u8(base + 4, 0x25);
        let node = object(&mut s.e, 0x100, &[]);
        let log = logged(&mut s.e, |e| {
            e.call(0x0057_02e0, &args![refr, node, 0u32]);
        });
        assert!(calls_to(&log, 0x0063_3c90).is_empty());
        assert!(calls_to(&log, 0x006c_0c30).is_empty());
    }

    #[test]
    fn set_3d_to_null_cancels_the_loader_request_of_a_parentless_3d() {
        let mut s = scene(false);
        let (refr, old) = (s.refr, s.old);
        s.e.mem.set_u32(old + 0x18, 0);
        let loader = model_loader(&s.e);
        let log = logged(&mut s.e, |e| {
            e.call(0x0057_02e0, &args![refr, 0u32, 0u32]);
        });
        assert_eq!(calls_to(&log, CANCEL_REFERENCE)[0], vec![loader, refr]);
        // no parent: nothing is detached
        assert!(calls_to(&log, slot(&s.e, s.parent, 0xe8)).is_empty());
    }

    /// Unloads the 3D of an actor reference; returns the log.
    fn unload_actor(
        configure: impl FnOnce(&mut Scene),
        log_unload: bool,
    ) -> (Scene, Vec<(u32, Vec<u32>)>) {
        let mut s = scene(true);
        configure(&mut s);
        let refr = s.refr;
        let log = logged(&mut s.e, |e| {
            e.call(0x0057_02e0, &args![refr, 0u32, u32::from(log_unload)]);
        });
        (s, log)
    }

    #[test]
    fn set_3d_to_null_on_an_actor_drops_the_art_of_its_process() {
        let (s, log) = unload_actor(
            |s| {
                s.e.register(0x0093_1850, |_, _| ret(1));
                let loaded = s.e.mem.u32(s.refr + 0x64);
                s.e.mem.set_u32(loaded + 4, 0x1234);
                s.e.register(0x0041_3f40, |_, _| ret(0x1111));
                s.e.register(0x0043_b1b0, |_, _| ret(0x2222));
                returns(&mut s.e, GET_PRIMITIVE, 0x5000);
            },
            true,
        );
        let process = s.process;
        assert_eq!(
            calls_to(&log, slot(&s.e, process, 0x79c)),
            vec![vec![process, 0]]
        );
        assert_eq!(
            calls_to(&log, slot(&s.e, process, 0x794)),
            vec![vec![process, 0]]
        );
        assert_eq!(
            calls_to(&log, slot(&s.e, process, 0x7a4)),
            vec![vec![process, 0]]
        );
        assert_eq!(calls_to(&log, 0x0092_b7f0), vec![vec![process]]);
        assert_eq!(calls_to(&log, 0x005b_5e40), vec![vec![0x0103_0df8, 0x2222]]);
        // the loaded data forgets the 3D's owner word, the primitive is reset
        let loaded = s.e.mem.u32(s.refr + 0x64);
        assert_eq!(s.e.mem.u32(loaded + 4), 0);
        assert_eq!(calls_to(&log, 0x004a_4c50), vec![vec![0x5000]]);
        // the cleanup of everything that tracks the 3D ran
        assert_eq!(calls_to(&log, 0x0056_5670), vec![vec![s.refr]]);

        // without the log flag no message is printed, and a process type above 1 is left alone
        let (s, log) = unload_actor(
            |s| {
                s.e.register(0x0093_1850, |_, _| ret(2));
            },
            false,
        );
        assert!(calls_to(&log, 0x005b_5e40).is_empty());
        assert!(calls_to(&log, slot(&s.e, s.process, 0x79c)).is_empty());
    }

    #[test]
    fn set_3d_to_null_on_an_actor_saves_havok_data_and_animation_for_a_saved_change() {
        let (s, log) = unload_actor(
            |s| {
                s.e.register(0x0046_2480, |_, _| ret(1));
                s.e.register(0x0084_a6d0, |_, _| ret(1));
                s.e.register(0x0086_5020, |e, a| {
                    e.mem.set_u32(a[0], 0x6161);
                    ret(a[0])
                });
            },
            false,
        );
        // the Havok data goes to the extra list, the animation to the virtual +0x4c
        assert_eq!(
            calls_to(&log, 0x0042_2ac0),
            vec![vec![s.refr + 0x44, 0x6161]]
        );
        assert_eq!(
            calls_to(&log, slot(&s.e, s.refr, 0x4c)),
            vec![vec![s.refr, 0x1000_0000]]
        );
        // the two change masks were built with 008c71b0
        let masks = calls_to(&log, 0x008c_71b0);
        assert_eq!(masks.len(), 2);
        assert_eq!(masks[0][1], 4);
        assert_eq!(masks[1][1], 0x1000_0000);
        let queries = calls_to(&log, 0x0084_a6d0);
        assert_eq!(queries[0][1..], [s.refr, 4]);
        assert_eq!(queries[1][1..], [s.refr, 0x1000_0000]);
    }

    #[test]
    fn set_3d_to_null_does_not_save_when_the_form_is_deleted_or_nothing_changed() {
        let (_, log) = unload_actor(
            |s| {
                s.e.register(0x0046_2480, |_, _| ret(1));
                s.e.register(0x0084_a6d0, |_, _| ret(1));
                s.e.mem.set_u32(s.refr + 8, 0x20);
            },
            false,
        );
        assert!(calls_to(&log, 0x0084_a6d0).is_empty());
        let (_, log) = unload_actor(
            |s| {
                s.e.register(0x0046_2480, |_, _| ret(1));
                s.e.register(0x0084_a6d0, |_, _| ret(0));
            },
            false,
        );
        assert!(calls_to(&log, 0x0042_2ac0).is_empty());
        assert!(calls_to(&log, 0x0042_2940).is_empty());
        // both queries were made, and neither answered yes
        assert_eq!(calls_to(&log, 0x0084_a6d0).len(), 2);
    }

    #[test]
    fn set_3d_to_null_on_an_actor_removes_the_weapon_and_clears_its_ragdoll_data() {
        let (s, log) = unload_actor(
            |s| {
                let owned = object(&mut s.e, 0x20, &[(0, 0)]);
                s.e.mem.set_u32(s.refr + 0xac, owned);
                s.e.mem.set_u32(s.refr + 0xb0, 0x7a7a);
                returns(&mut s.e, 0x0093_06d0, 0x8b8b);
            },
            false,
        );
        let refr = s.refr;
        let owned = s.e.mem.u32(refr + 0xac);
        assert_eq!(owned, 0);
        assert_eq!(calls_to(&log, 0x0045_34f0), vec![vec![refr, 0]]);
        assert_eq!(calls_to(&log, 0x0048_3710), vec![vec![refr], vec![refr]]);
        assert_eq!(calls_to(&log, 0x00c6_d7a0), vec![vec![0x8b8b, 0]]);
        assert_eq!(calls_to(&log, 0x00ca_20e0), vec![vec![0x7a7a]]);
        assert_eq!(calls_to(&log, slot(&s.e, refr, 0x1f0)), vec![vec![refr, 0]]);
        assert_eq!(calls_to(&log, 0x0057_2a50), vec![vec![refr, 0]]);
        assert_eq!(calls_to(&log, 0x0057_2e50), vec![vec![refr, 0]]);
        // the process leaves the simulation (virtual +0x5f4), its ragdoll holder is gone
        assert_eq!(
            calls_to(&log, slot(&s.e, s.process, 0x5f4)),
            vec![vec![s.process, 0]]
        );
        // RemoveWeapon ran: it asked the process for the held weapon node
        assert!(!calls_to(&log, slot(&s.e, s.process, 0x1a0)).is_empty());
    }

    #[test]
    fn set_3d_to_null_stops_the_moving_sounds_unless_the_reference_keeps_them() {
        let (s, log) = unload_actor(
            |s| {
                returns(&mut s.e, 0x0045_3a70, 0xa0d1);
                // the virtual at +0x224 asks for the sounds to be stopped
                let refr = s.refr;
                let vtable = s.e.mem.u32(refr);
                s.e.register(vtable + 0x224, |_, _| ret(1));
            },
            false,
        );
        assert_eq!(calls_to(&log, 0x00ad_8570), vec![vec![0xa0d1, s.old, 0, 0]]);
        assert_eq!(calls_to(&log, 0x0057_a3c0), vec![vec![s.refr, 1]]);

        let (s, log) = unload_actor(|_| {}, false);
        assert!(calls_to(&log, 0x00ad_8570).is_empty());
        assert_eq!(calls_to(&log, 0x0057_a3c0), vec![vec![s.refr, 1]]);
    }

    #[test]
    fn set_3d_to_null_releases_the_old_node_through_the_garbage_collector() {
        let (s, log) = unload_actor(
            |s| {
                s.e.register(0x0045_cd60, |_, _| ret(1));
            },
            false,
        );
        // an actor that is not the player: its process stops using the art, the node is queued
        assert_eq!(
            calls_to(&log, slot(&s.e, s.process, 0x4fc)),
            vec![vec![s.process, 0]]
        );
        assert_eq!(calls_to(&log, 0x0086_8660), vec![vec![s.old]]);
        assert!(calls_to(&log, 0x00c4_5830).is_empty());
        // the base object hears about it (virtual +0x150) once the loader is done
        assert_eq!(
            calls_to(&log, slot(&s.e, s.base, 0x150)),
            vec![vec![s.base, s.refr]]
        );
        // the old node was released through the NiPointer assignment
        assert!(calls_to(&log, 0x0066_b0d0).iter().any(|a| a[1] == 0));
        // a loader that holds the reference: the base object is not told
        let (s, log) = unload_actor(
            |s| {
                s.e.register(0x0045_23e0, |_, _| ret(1));
            },
            false,
        );
        assert!(calls_to(&log, slot(&s.e, s.base, 0x150)).is_empty());
        // a reference the loader wants cancelled
        let (s, log) = unload_actor(
            |s| {
                s.e.register(0x0056_51e0, |_, _| ret(1));
            },
            false,
        );
        assert!(calls_to(&log, slot(&s.e, s.base, 0x150)).is_empty());
        assert!(!calls_to(&log, CANCEL_REFERENCE).is_empty());
    }

    #[test]
    fn set_3d_to_null_hands_the_players_node_to_its_own_release() {
        let mut s = scene(true);
        let (refr, old) = (s.refr, s.old);
        s.e.set_global(GLOBAL_PLAYER, refr);
        let log = logged(&mut s.e, |e| {
            e.call(0x0057_02e0, &args![refr, 0u32, 0u32]);
        });
        assert_eq!(calls_to(&log, 0x00c4_5830), vec![vec![old]]);
        assert!(calls_to(&log, 0x0086_8660).is_empty());
    }

    #[test]
    fn set_3d_to_null_with_a_destructible_updates_the_replacement_models() {
        let mut s = scene(false);
        let (refr, base) = (s.refr, s.base);
        // the object health is set, the reference has a 3D: the destruction form is released
        s.e.register(0x0045_2370, |_, _| ret(1));
        s.e.register(0x0041_b6b0, |_, _| Ret {
            st0: 50.0,
            ..Ret::default()
        });
        returns(&mut s.e, 0x0047_5400, 0x3030);
        let log = logged(&mut s.e, |e| {
            e.call(0x0057_02e0, &args![refr, 0u32, 0u32]);
        });
        assert_eq!(calls_to(&log, 0x0047_5400), vec![vec![base]]);
        assert_eq!(calls_to(&log, 0x0047_78a0), vec![vec![0x3030]]);

        // a node being attached to a reference without 3D preloads the replacements
        let mut s = scene(false);
        let (refr, base) = (s.refr, s.base);
        set_node(&mut s.e, refr, 0);
        s.e.register(0x0045_2370, |_, _| ret(1));
        s.e.register(0x0041_b6b0, |_, _| Ret {
            st0: 50.0,
            ..Ret::default()
        });
        returns(&mut s.e, 0x0047_5400, 0x3030);
        let node = object(&mut s.e, 0x100, &[]);
        let log = logged(&mut s.e, |e| {
            e.call(0x0057_02e0, &args![refr, node, 0u32]);
        });
        assert_eq!(calls_to(&log, 0x0047_7780), vec![vec![0x3030, base]]);

        // a zero health leaves the destruction form alone
        let mut s = scene(false);
        let refr = s.refr;
        s.e.register(0x0045_2370, |_, _| ret(1));
        s.e.register(0x0041_b6b0, |_, _| Ret::default());
        let log = logged(&mut s.e, |e| {
            e.call(0x0057_02e0, &args![refr, 0u32, 0u32]);
        });
        assert!(calls_to(&log, 0x0047_5400).is_empty());
    }

    #[test]
    fn set_3d_tail_0x00570f70_updates_markers_and_runs_the_virtual_without_the_task_queue() {
        let mut s = scene(false);
        let (refr, base) = (s.refr, s.base);
        s.e.mem.set_u8(base + 4, 0x1c);
        returns(&mut s.e, 0x006c_0720, 0x9090);
        let log = logged(&mut s.e, |e| {
            e.call(0x0057_0f70, &args![refr, 0u32]);
        });
        assert_eq!(calls_to(&log, 0x006c_0c80), vec![vec![0x9090, refr]]);
        assert_eq!(calls_to(&log, 0x006c_1060), vec![vec![0x9090, refr]]);
        assert_eq!(calls_to(&log, slot(&s.e, refr, 0x1c0)), vec![vec![refr]]);
        assert!(calls_to(&log, 0x0087_b2b0).is_empty());
        assert_eq!(calls_to(&log, 0x0057_c300), vec![vec![refr]]);

        // the task queue replaces the virtual when 008c7aa0 holds
        s.e.register(0x008c_7aa0, |_, _| ret(1));
        returns(&mut s.e, 0x0045_37b0, 0xabab);
        let log = logged(&mut s.e, |e| {
            e.call(0x0057_0f70, &args![refr, 0u32]);
        });
        assert_eq!(calls_to(&log, 0x0087_b2b0), vec![vec![0xabab, refr]]);
        assert!(calls_to(&log, slot(&s.e, refr, 0x1c0)).is_empty());
    }

    #[test]
    fn set_3d_tail_0x00570f70_clears_the_room_or_portal_extra_of_a_marker() {
        let mut s = scene(false);
        let (refr, base) = (s.refr, s.base);
        s.e.set_global(GLOBAL_ROOM_MARKER_BASE, base);
        let log = logged(&mut s.e, |e| {
            e.call(0x0057_0f70, &args![refr, 0u32]);
        });
        assert_eq!(calls_to(&log, 0x0042_0f00), vec![vec![refr + 0x44, 0]]);
        assert!(calls_to(&log, 0x0042_0e00).is_empty());

        let mut s = scene(false);
        let (refr, base) = (s.refr, s.base);
        s.e.set_global(GLOBAL_PORTAL_MARKER_BASE, base);
        let log = logged(&mut s.e, |e| {
            e.call(0x0057_0f70, &args![refr, 0u32]);
        });
        assert_eq!(calls_to(&log, 0x0042_0e00), vec![vec![refr + 0x44, 0]]);
        assert!(calls_to(&log, 0x0042_0f00).is_empty());

        // with a node nothing is cleared and the obstacle is added
        let mut s = scene(false);
        let (refr, base) = (s.refr, s.base);
        s.e.set_global(GLOBAL_ROOM_MARKER_BASE, base);
        let log = logged(&mut s.e, |e| {
            e.call(0x0057_0f70, &args![refr, 0x4444u32]);
        });
        assert!(calls_to(&log, 0x0042_0f00).is_empty());
        assert_eq!(calls_to(&log, 0x006c_0c30).len(), 1);
    }

    // ---- 00570ea0 .. 00570f60 ---------------------------------------------

    #[test]
    fn node_flag_tests_ask_for_flag_0x20000() {
        let mut e = engine();
        returns(&mut e, 0x0045_6630, 1);
        let log = logged(&mut e, |e| {
            assert!(e.call(0x0057_0ea0, &args![0x4000u32]).bool());
            assert!(e.call(0x0057_0ee0, &args![0x4000u32]).bool());
        });
        assert_eq!(
            calls_to(&log, 0x0045_6630),
            vec![vec![0x4000, 0x2_0000], vec![0x4000, 0x2_0000]]
        );
        returns(&mut e, 0x0045_6630, 0);
        assert!(!e.call(0x0057_0ea0, &args![0x4000u32]).bool());
        assert!(!e.call(0x0057_0ee0, &args![0x4000u32]).bool());
    }

    #[test]
    fn cell_test_00570ec0_is_true_only_for_exactly_one() {
        let mut e = engine();
        for (answer, expected) in [(1u32, true), (0, false), (2, false)] {
            returns(&mut e, 0x0045_0fd0, answer);
            assert_eq!(e.call(0x0057_0ec0, &args![0x4000u32]).bool(), expected);
        }
    }

    #[test]
    fn loading_tests_00570f00_and_00570f40() {
        let mut e = engine();
        let object = e.mem.alloc(0x300);
        assert!(e.call(0x0057_0f40, &args![object]).bool());
        e.mem.set_u32(object + 0x244, 1);
        assert!(!e.call(0x0057_0f40, &args![object]).bool());
        e.mem.set_u32(object + 0x244, 2);
        assert!(e.call(0x0057_0f40, &args![object]).bool());

        returns(&mut e, 0x0046_2480, 1);
        assert!(e.call(0x0057_0f00, &args![object]).bool());
        e.mem.set_u32(object + 0x244, 3);
        assert!(!e.call(0x0057_0f00, &args![object]).bool());
        e.mem.set_u32(object + 0x244, 0);
        returns(&mut e, 0x0046_2480, 0);
        assert!(!e.call(0x0057_0f00, &args![object]).bool());
    }

    #[test]
    fn globals_00570f60_and_00571550_are_read_from_memory() {
        let mut e = engine();
        e.set_global(0x011d_fc98, 7u32);
        e.set_global(0x011c_61a4, 0x1357u32);
        assert_eq!(e.call(0x0057_0f60, &args![]).u32(), 7);
        assert_eq!(e.call(0x0057_1550, &args![]).u32(), 0x1357);
    }

    #[test]
    fn name_lookup_00571530_passes_root_and_name_with_the_flag() {
        let mut e = engine();
        returns(&mut e, 0x00c4_b470, 0x6677);
        let log = logged(&mut e, |e| {
            assert_eq!(
                e.call(0x0057_1530, &args![0x1u32, 0x2000u32, 0x3000u32])
                    .u32(),
                0x6677
            );
        });
        assert_eq!(calls_to(&log, 0x00c4_b470), vec![vec![0x2000, 0x3000, 1]]);
    }

    // ---- 005710c0 ----------------------------------------------------------

    #[test]
    fn worn_models_of_an_npc_base_replace_the_model_when_there_is_a_biped() {
        let mut e = engine();
        let refr = reference(&mut e, false, &[(0x1e8, 0x9000), (0x1e4, 0)]);
        let base = form(&mut e, 0x2a, 0);
        e.mem.set_u32(refr + 0x20, base);
        let log = logged(&mut e, |e| {
            assert!(e.call(0x0057_10c0, &args![refr]).bool());
        });
        assert_eq!(calls_to(&log, 0x0060_5d70), vec![vec![base, refr]]);
        assert!(calls_to(&log, 0x0049_6280).is_empty());

        // without a biped the base is left alone; an animation is reset first
        let refr = reference(&mut e, false, &[(0x1e8, 0), (0x1e4, 0x9100)]);
        e.mem.set_u32(refr + 0x20, base);
        let log = logged(&mut e, |e| {
            assert!(e.call(0x0057_10c0, &args![refr]).bool());
        });
        assert!(calls_to(&log, 0x0060_5d70).is_empty());
        assert_eq!(calls_to(&log, 0x0049_6280), vec![vec![0x9100]]);
    }

    #[test]
    fn worn_models_of_another_base_come_from_its_virtual() {
        let mut e = engine();
        let base = object(&mut e, 0x100, &[(0x160, 1)]);
        e.mem.set_u8(base + 4, 0x28);
        let refr = reference(&mut e, false, &[(0x1e8, 0), (0x1e4, 0)]);
        e.mem.set_u32(refr + 0x20, base);
        assert!(e.call(0x0057_10c0, &args![refr]).bool());
        let base = object(&mut e, 0x100, &[(0x160, 0)]);
        e.mem.set_u8(base + 4, 0x28);
        e.mem.set_u32(refr + 0x20, base);
        assert!(!e.call(0x0057_10c0, &args![refr]).bool());
    }

    /// A creature reference with a 3D and one package model to attach.
    /// Returns the engine, the reference, the node, the clone and the file
    /// name the model loader was asked for.
    fn creature_setup() -> (Engine, u32, u32, u32, Rc<RefCell<Vec<u8>>>) {
        let mut e = engine();
        let loader = e.mem.alloc(0x20);
        e.set_global(GLOBAL_MODEL_LOADER, loader);
        let tes = e.mem.alloc(0x20);
        e.set_global(GLOBAL_TES, tes);
        let refr = reference(&mut e, false, &[(0x1e8, 0), (0x1e4, 0)]);
        let base = form(&mut e, 0x2b, 0);
        e.mem.set_u32(refr + 0x20, base);
        let node = object(&mut e, 0x100, &[]);
        set_node(&mut e, refr, node);
        // the model path of the reference
        let path = e.mem.alloc(0x200);
        e.mem.set_cstr(path, b"meshes\\creatures\\body.nif");
        let model = object(&mut e, 0x40, &[(0x14, path)]);
        returns(&mut e, 0x0042_e250, model);
        // the package list: one entry naming another file
        let file = e.mem.alloc(0x40);
        e.mem.set_cstr(file, b"skeleton.nif");
        let entry = e.mem.alloc(0x10);
        e.mem.set_u32(entry, file);
        returns(&mut e, 0x0071_7e50, entry);
        // string helpers: copy, last backslash
        e.register(0x0040_6d30, |e, a| {
            let text = e.mem.cstr(a[2]);
            e.mem.set_cstr(a[0], &text);
            ret(a[0])
        });
        e.register(0x0040_ab30, |e, a| {
            let text = e.mem.cstr(a[0]);
            match text.iter().rposition(|c| *c == a[1] as u8) {
                Some(index) => ret(a[0] + index as u32),
                None => ret(0),
            }
        });
        let loaded_file = Rc::new(RefCell::new(Vec::new()));
        let record = loaded_file.clone();
        let loaded_model = e.mem.alloc(0x40);
        e.register_double(0x0044_7080, move |e, a| {
            *record.borrow_mut() = e.mem.cstr(a[1]);
            ret(loaded_model)
        });
        e.register(GET_SCALE, |_, _| Ret {
            st0: 1.5,
            ..Ret::default()
        });
        let clone = e.mem.alloc(0x40);
        e.register_double(0x00a5_d2c0, move |_, _| ret(clone));
        (e, refr, node, clone, loaded_file)
    }

    #[test]
    fn creature_package_models_are_cloned_and_attached_to_the_3d() {
        let (mut e, refr, node, clone, loaded_file) = creature_setup();
        let log = logged(&mut e, |e| {
            assert!(e.call(0x0057_10c0, &args![refr]).bool());
        });
        // the file name of the model is replaced by the package's
        assert_eq!(
            loaded_file.borrow().as_slice(),
            b"meshes\\creatures\\skeleton.nif"
        );
        let loads = calls_to(&log, 0x0044_7080);
        assert_eq!(loads[0][2..], [3, 1, 0, 0, 0]);
        // the clone's scale is the reference's scale
        assert_eq!(calls_to(&log, 0x004a_d050)[0][1], float_bits(1.5));
        // not skinned: attached to the node as a plain child
        assert_eq!(
            calls_to(&log, 0x004a_e250),
            vec![vec![node, clone, 0, 0, 0xffff_ffff, 0]]
        );
        assert!(calls_to(&log, 0x004a_de40).is_empty());
        // the NiPointer and the cloning process are destroyed
        assert_eq!(calls_to(&log, 0x0045_cec0).len(), 1);
        assert_eq!(calls_to(&log, 0x004a_d270).len(), 1);
        // the package list was walked from the base object
        let base = e.mem.u32(refr + 0x20);
        assert_eq!(calls_to(&log, 0x0071_7e50), vec![vec![base + 0x114]]);
        assert_eq!(calls_to(&log, 0x005f_a0a0), vec![vec![base, refr]]);
    }

    #[test]
    fn creature_models_with_a_morpher_use_the_deep_copy() {
        let (mut e, refr, node, _, _) = creature_setup();
        returns(&mut e, 0x004b_5bf0, 1);
        let copy = e.mem.alloc(0x40);
        returns(&mut e, 0x0045_7ba0, copy);
        let log = logged(&mut e, |e| {
            e.call(0x0057_10c0, &args![refr]);
        });
        let tes = e.global::<u32>(GLOBAL_TES);
        let copies = calls_to(&log, 0x0045_7ba0);
        assert_eq!(copies.len(), 1);
        assert_eq!(copies[0][0], tes);
        assert!(calls_to(&log, 0x00a5_d2c0).is_empty());
        assert_eq!(calls_to(&log, 0x004a_e250)[0][..2], [node, copy]);
    }

    #[test]
    fn skinned_creature_models_hang_under_the_skin_attachment_child() {
        let (mut e, refr, node, clone, _) = creature_setup();
        returns(&mut e, 0x0049_10d0, 1);
        e.set_global(0x011c_61a4, 0x1234u32);
        let child = object(&mut e, 0x40, &[(0xdc, 0)]);
        let found = object(&mut e, 0x40, &[(0x9c, child)]);
        returns(&mut e, 0x00c4_b470, found);
        returns(&mut e, 0x0043_8170, 0x4040);
        let log = logged(&mut e, |e| {
            e.call(0x0057_10c0, &args![refr]);
        });
        assert_eq!(calls_to(&log, 0x00c4_b470), vec![vec![node, 0x1234, 1]]);
        assert_eq!(
            calls_to(&log, slot(&e, found, 0x9c)),
            vec![vec![found, 0x4040]]
        );
        assert_eq!(
            calls_to(&log, slot(&e, child, 0xdc)),
            vec![vec![child, clone, 1]]
        );
        assert_eq!(calls_to(&log, 0x004a_de40), vec![vec![node, clone, 0, 1]]);
        assert!(calls_to(&log, 0x004a_e250).is_empty());
        // the fixed string "SkinAttachment" (01 01f724) is built and destroyed
        assert_eq!(calls_to(&log, 0x0043_8170)[0][1], 0x0101_f724);
        assert_eq!(calls_to(&log, 0x0043_81b0).len(), 1);

        // without that child the found node's parent takes the clone
        let (mut e, refr, node, clone, _) = creature_setup();
        returns(&mut e, 0x0049_10d0, 1);
        let parent = object(&mut e, 0x40, &[(0xdc, 0)]);
        let found = object(&mut e, 0x40, &[(0x9c, 0)]);
        e.mem.set_u32(found + 0x18, parent);
        returns(&mut e, 0x00c4_b470, found);
        let log = logged(&mut e, |e| {
            e.call(0x0057_10c0, &args![refr]);
        });
        assert_eq!(
            calls_to(&log, slot(&e, parent, 0xdc)),
            vec![vec![parent, clone, 1]]
        );
        assert_eq!(calls_to(&log, 0x004a_de40), vec![vec![node, clone, 0, 1]]);
    }

    #[test]
    fn creature_models_are_skipped_without_a_3d_or_for_a_running_package() {
        let (mut e, refr, ..) = creature_setup();
        set_node(&mut e, refr, 0);
        let log = logged(&mut e, |e| {
            assert!(e.call(0x0057_10c0, &args![refr]).bool());
        });
        assert!(calls_to(&log, 0x0071_7e50).is_empty());

        let (mut e, refr, ..) = creature_setup();
        returns(&mut e, 0x0082_56d0, 1);
        let log = logged(&mut e, |e| {
            e.call(0x0057_10c0, &args![refr]);
        });
        assert!(calls_to(&log, 0x0044_7080).is_empty());
    }

    // ---- 00571560 .. 00571630 ---------------------------------------------

    #[test]
    fn clear_interpolators_resets_the_animation_in_order() {
        let mut e = engine();
        let refr = reference(&mut e, false, &[(0x1e4, 0xa11a)]);
        returns(&mut e, 0x0049_6940, 0xc7c7);
        returns(&mut e, 0x0055_85e0, 0xd8d8);
        let log = logged(&mut e, |e| {
            e.call(0x0057_1560, &args![refr]);
        });
        let order: Vec<(u32, Vec<u32>)> = log.into_iter().skip(2).collect();
        assert_eq!(
            order,
            vec![
                (0x0049_8910, vec![0xa11a, 1, 0]),
                (0x0049_6080, vec![0xa11a, 0x14, 0]),
                (0x0049_6940, vec![0xa11a]),
                (0x0048_fef0, vec![0xc7c7, 0]),
                (0x0055_85e0, vec![0xa11a]),
                (0x0049_9160, vec![0xd8d8]),
                (0x0049_9080, vec![0xa11a]),
            ]
        );

        // no animation: nothing
        let refr = reference(&mut e, false, &[(0x1e4, 0)]);
        let log = logged(&mut e, |e| {
            e.call(0x0057_1560, &args![refr]);
        });
        assert_eq!(log.len(), 2);
    }

    fn model_with_path(e: &mut Engine, path: u32) -> u32 {
        object(e, 0x40, &[(0x14, path)])
    }

    #[test]
    fn model_of_a_reference_prefers_the_model_swap() {
        let mut e = engine();
        let refr = e.mem.alloc(0x100);
        let swap = e.mem.alloc(0x40);
        returns(&mut e, 0x0042_e250, swap);
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0057_1630, &args![refr]).u32(), swap);
        });
        assert_eq!(calls_to(&log, 0x0042_e250), vec![vec![refr + 0x44]]);
        assert!(calls_to(&log, 0x00ec_43fb).is_empty());
    }

    #[test]
    fn model_of_a_weapon_depends_on_its_mod_flags() {
        let mut e = engine();
        let refr = e.mem.alloc(0x100);
        let weapon = form(&mut e, 0x28, 0);
        e.mem.set_u32(refr + 0x20, weapon);
        returns(&mut e, 0x0052_2d80, 0x1010);
        returns(&mut e, 0x0052_2df0, 0x2020);
        // no mod flags: the plain model
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0057_1630, &args![refr]).u32(), 0x1010);
        });
        assert_eq!(calls_to(&log, 0x0052_2d80), vec![vec![weapon]]);
        // mod flags: the modded model for those flags (second word 0)
        returns(&mut e, 0x0042_e560, 5);
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0057_1630, &args![refr]).u32(), 0x2020);
        });
        assert_eq!(calls_to(&log, 0x0052_2df0), vec![vec![weapon, 5, 0]]);
    }

    #[test]
    fn model_of_another_base_is_its_bound_object_cast() {
        let mut e = engine();
        let refr = e.mem.alloc(0x100);
        let base = form(&mut e, 0x1, 0);
        e.mem.set_u32(refr + 0x20, base);
        returns(&mut e, 0x00ec_43fb, 0x3030);
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0057_1630, &args![refr]).u32(), 0x3030);
        });
        assert_eq!(
            calls_to(&log, 0x00ec_43fb),
            vec![vec![base, 0, 0x0118_3108, 0x0118_31e8, 0]]
        );
    }

    #[test]
    fn model_falls_back_to_the_world_model_of_a_biped_form_for_the_owners_sex() {
        let mut e = engine();
        let refr = e.mem.alloc(0x100);
        let base = form(&mut e, 0x1, 0);
        e.mem.set_u32(refr + 0x20, base);
        returns(&mut e, 0x0048_0db0, 0x5050);
        returns(&mut e, 0x0048_1110, 0x6060);
        // no owner: sex 0
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0057_1630, &args![refr]).u32(), 0x6060);
        });
        assert_eq!(calls_to(&log, 0x0048_0db0), vec![vec![base]]);
        assert_eq!(calls_to(&log, 0x0048_1110), vec![vec![0x5050, 0]]);
        // an NPC owner decides
        let owner = form(&mut e, 0x2a, 0);
        returns(&mut e, 0x0056_7790, owner);
        returns(&mut e, 0x005f_0cc0, 1);
        let log = logged(&mut e, |e| {
            e.call(0x0057_1630, &args![refr]);
        });
        assert_eq!(calls_to(&log, 0x0048_1110), vec![vec![0x5050, 1]]);
        // an owner that is not an NPC does not
        let faction = form(&mut e, 0x8, 0);
        returns(&mut e, 0x0056_7790, faction);
        let log = logged(&mut e, |e| {
            e.call(0x0057_1630, &args![refr]);
        });
        assert_eq!(calls_to(&log, 0x0048_1110), vec![vec![0x5050, 0]]);
        // no biped form: no model
        returns(&mut e, 0x0048_0db0, 0);
        assert_eq!(e.call(0x0057_1630, &args![refr]).u32(), 0);
    }

    #[test]
    fn model_path_is_the_virtual_of_the_model_or_zero() {
        let mut e = engine();
        let refr = e.mem.alloc(0x100);
        let base = form(&mut e, 0x1, 0);
        e.mem.set_u32(refr + 0x20, base);
        returns(&mut e, 0x0048_0db0, 0);
        assert_eq!(e.call(0x0057_15d0, &args![refr]).u32(), 0);
        let model = model_with_path(&mut e, 0x7777);
        returns(&mut e, 0x0042_e250, model);
        assert_eq!(e.call(0x0057_15d0, &args![refr]).u32(), 0x7777);
    }

    #[test]
    fn model_bound_size_comes_from_the_base_object() {
        let mut e = engine();
        let refr = e.mem.alloc(0x100);
        assert_eq!(e.call(0x0057_1600, &args![refr]).f32(), 0.0);
        let base = form(&mut e, 0x1, 0);
        e.mem.set_u32(refr + 0x20, base);
        e.register(0x0050_ebf0, |_, _| Ret {
            st0: 3.5,
            ..Ret::default()
        });
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0057_1600, &args![refr]).f32(), 3.5);
        });
        assert_eq!(calls_to(&log, 0x0050_ebf0), vec![vec![base]]);
    }

    // ---- 00571760 ----------------------------------------------------------

    struct Equip {
        e: Engine,
        actor: u32,
        process: u32,
        item: u32,
        animation: u32,
        node: u32,
    }

    fn equip_setup() -> Equip {
        let mut e = engine();
        for global in [GLOBAL_MODEL_LOADER, GLOBAL_LOADING_OBJECT, GLOBAL_PLAYER] {
            let object = e.mem.alloc(0x300);
            e.set_global(global, object);
        }
        let animation = object(&mut e, 0x100, &[]);
        let process = object(
            &mut e,
            0x700,
            &[(0x454, 1), (0x458, 0), (0x1cc, 0), (0x468, 0), (0x148, 0)],
        );
        let actor = reference(&mut e, true, &[(0x1e8, 0), (0x1e4, animation), (0x22c, 0)]);
        e.mem.set_u32(actor + 0x68, process);
        let node = object(&mut e, 0x100, &[]);
        set_node(&mut e, actor, node);
        let item = e.mem.alloc(0x100);
        Equip {
            e,
            actor,
            process,
            item,
            animation,
            node,
        }
    }

    #[test]
    fn equip_does_nothing_without_a_3d() {
        let mut s = equip_setup();
        let (actor, item) = (s.actor, s.item);
        set_node(&mut s.e, actor, 0);
        let log = logged(&mut s.e, |e| {
            e.call(0x0057_1760, &args![actor, item]);
        });
        assert_eq!(log.len(), 2);
    }

    #[test]
    fn equip_prints_the_debug_message_for_names_containing_lily() {
        let mut s = equip_setup();
        let (actor, item) = (s.actor, s.item);
        returns(&mut s.e, 0x0055_d520, 0x4a4a);
        returns(&mut s.e, 0x00ec_7750, 0x4a50);
        let log = logged(&mut s.e, |e| {
            e.call(0x0057_1760, &args![actor, item]);
        });
        assert_eq!(calls_to(&log, 0x00ec_7750), vec![vec![0x4a4a, 0x0102_fdc8]]);
        assert_eq!(calls_to(&log, 0x005b_5e40), vec![vec![0x0103_0e38]]);
        returns(&mut s.e, 0x00ec_7750, 0);
        let log = logged(&mut s.e, |e| {
            e.call(0x0057_1760, &args![actor, item]);
        });
        assert!(calls_to(&log, 0x005b_5e40).is_empty());
    }

    #[test]
    fn equip_on_a_non_player_actor_loads_the_animation_and_attaches_the_item() {
        let mut s = equip_setup();
        let (actor, item, process, animation, node) =
            (s.actor, s.item, s.process, s.animation, s.node);
        let loader = model_loader(&s.e);
        s.e.mem.set_u32(0x0118_a838 + 2 * 4, 0x66);
        returns(&mut s.e, 0x0044_6390, 2);
        returns(&mut s.e, 0x0049_6940, 1);
        returns(&mut s.e, 0x005f_2370, 0x1_1234);
        let path = s.e.mem.alloc(0x40);
        let model = object(&mut s.e, 0x40, &[(0x14, path)]);
        returns(&mut s.e, 0x0042_e250, model);
        returns(&mut s.e, 0x0044_7330, 0x7a7a);
        returns(&mut s.e, 0x0046_e8c0, 1);
        let list = s.e.mem.alloc(0x10);
        s.e.mem.set_u32(list + 4, 1);
        returns(&mut s.e, 0x0047_5020, list);
        let log = logged(&mut s.e, |e| {
            e.call(0x0057_1760, &args![actor, item]);
        });
        // a non-player process gets virtual +0x458 (actor, 0)
        assert_eq!(
            calls_to(&log, slot(&s.e, process, 0x458)),
            vec![vec![process, actor, 0]]
        );
        // the weapon animation group is looked up and its animations loaded
        assert_eq!(calls_to(&log, 0x005f_2370), vec![vec![0, 0x66, 0x18, 0]]);
        assert_eq!(calls_to(&log, 0x0049_4710), vec![vec![animation, 0x1234]]);
        assert_eq!(
            calls_to(&log, slot(&s.e, actor, 0x22c)),
            vec![vec![actor, 1]]
        );
        assert_eq!(
            calls_to(&log, 0x0044_7330),
            vec![vec![loader, path, 1, 0, 0x66]]
        );
        assert_eq!(calls_to(&log, 0x0049_0400), vec![vec![animation, 0x7a7a]]);
        // the item is attached with the drawn state decided by the item
        assert_eq!(
            calls_to(&log, 0x004a_eed0),
            vec![vec![item, item + 0x3c, 5, actor, 0]]
        );
        assert_eq!(
            calls_to(&log, slot(&s.e, process, 0x1cc)),
            vec![vec![process, 1, 0, animation, actor]]
        );
        // the actor refreshes itself
        assert_eq!(calls_to(&log, 0x008b_0b00), vec![vec![actor, 1]]);
        assert_eq!(calls_to(&log, 0x008b_0bd0), vec![vec![actor, node]]);
        assert_eq!(calls_to(&log, 0x008c_4640), vec![vec![actor]]);
        assert_eq!(calls_to(&log, 0x0089_1190), vec![vec![actor, 1]]);
        assert_eq!(
            calls_to(&log, slot(&s.e, process, 0x468)),
            vec![vec![process, 1]]
        );
        // the player hooks are not touched
        assert!(calls_to(&log, 0x004a_b750).is_empty());
    }

    #[test]
    fn equip_skips_the_animation_load_when_the_group_is_already_loaded() {
        let mut s = equip_setup();
        let (actor, item) = (s.actor, s.item);
        returns(&mut s.e, 0x0049_6940, 1);
        returns(&mut s.e, 0x0049_4710, 1);
        let log = logged(&mut s.e, |e| {
            e.call(0x0057_1760, &args![actor, item]);
        });
        assert!(calls_to(&log, 0x0044_7330).is_empty());
        // the drawn state is the actor's own when the item does not force it
        returns(&mut s.e, 0x008a_16d0, 1);
        let process = s.process;
        let log = logged(&mut s.e, |e| {
            e.call(0x0057_1760, &args![actor, item]);
        });
        assert_eq!(calls_to(&log, slot(&s.e, process, 0x1cc))[0][1], 1);
    }

    #[test]
    fn equip_on_the_player_uses_the_first_person_biped_and_the_mod_effect() {
        let mut s = equip_setup();
        let (actor, item, process) = (s.actor, s.item, s.process);
        s.e.set_global(GLOBAL_PLAYER, actor);
        let vtable = s.e.mem.u32(process);
        s.e.register(vtable + 0x148, |_, _| ret(0x6600));
        s.e.register_double(0x0095_0b00, |_, a| {
            ret(if a[1] == 1 { 0xb1b1 } else { 0xb2b2 })
        });
        returns(&mut s.e, 0x004b_d820, 3);
        returns(&mut s.e, 0x004b_d8d0, 1);
        returns(&mut s.e, 0x004a_d010, 1);
        returns(&mut s.e, 0x0050_4e60, 0x4e4e);
        let log = logged(&mut s.e, |e| {
            e.call(0x0057_1760, &args![actor, item]);
        });
        // the mod slots and the mod effect 0xe of the worn item
        assert_eq!(calls_to(&log, 0x004b_d820), vec![vec![0x6600]]);
        let effects = calls_to(&log, 0x004b_d8d0);
        assert_eq!(effects.len(), 1);
        assert_eq!(effects[0][..2], [0x6600, 0xe]);
        // both bipeds get the item with the slots
        assert_eq!(
            calls_to(&log, 0x004a_b750),
            vec![vec![0xb1b1, item, 3], vec![0xb2b2, item, 3]]
        );
        // the user-interface hook runs for the weapon when the mod effect is active
        assert_eq!(calls_to(&log, 0x0070_9c20), vec![vec![0x4e4e]]);
        // no animation loading and no attachment: the biped exists
        assert!(calls_to(&log, 0x0044_7330).is_empty());
        assert!(calls_to(&log, 0x004a_eed0).is_empty());

        // without the mod effect, a weapon that is not exempt gets no hook
        returns(&mut s.e, 0x004b_d8d0, 0);
        returns(&mut s.e, 0x004a_d030, 1);
        let log = logged(&mut s.e, |e| {
            e.call(0x0057_1760, &args![actor, item]);
        });
        assert!(calls_to(&log, 0x0070_9c20).is_empty());
    }

    // ---- 00571b50 ----------------------------------------------------------

    /// An actor with a process whose held-weapon virtual returns `held`,
    /// whose named list is `list`, and a worn weapon form with a model.
    struct Weapon {
        e: Engine,
        actor: u32,
        process: u32,
        node: u32,
        path: u32,
    }

    fn weapon_setup(held: u32, list: u32) -> Weapon {
        let mut e = engine();
        for global in [
            GLOBAL_MODEL_LOADER,
            GLOBAL_PLAYER,
            GLOBAL_DATA_HANDLER,
            GLOBAL_SAVE_LOAD,
        ] {
            let object = e.mem.alloc(0x300);
            e.set_global(global, object);
        }
        let process = object(&mut e, 0x700, &[(0x1a0, held), (0x1a4, list), (0x468, 0)]);
        let node = object(&mut e, 0x100, &[]);
        let actor = reference(&mut e, true, &[(0x1e8, 0), (0x1d0, node), (0x21c, 0)]);
        e.mem.set_u32(actor + 0x68, process);
        set_node(&mut e, actor, node);
        // the worn weapon form, with a TESModel at +0x3c
        let worn = e.mem.alloc(0x100);
        let path = e.mem.alloc(0x40);
        let model_vtable = e.mem.alloc(0x40);
        e.register_double(model_vtable + 0x14, move |_, _| ret(path));
        e.mem.set_u32(model_vtable + 0x14, model_vtable + 0x14);
        e.mem.set_u32(worn + 0x3c, model_vtable);
        let changes = e.mem.alloc(0x40);
        returns(&mut e, 0x0041_8520, changes);
        let entry = e.mem.alloc(0x40);
        returns(&mut e, 0x004c_8c10, entry);
        returns(&mut e, 0x0044_ddc0, worn);
        returns(&mut e, 0x0045_0b80, 0x5c5c);
        Weapon {
            e,
            actor,
            process,
            node,
            path,
        }
    }

    #[test]
    fn remove_weapon_does_nothing_without_a_3d() {
        let mut s = weapon_setup(0, 0);
        let actor = s.actor;
        set_node(&mut s.e, actor, 0);
        let log = logged(&mut s.e, |e| {
            e.call(0x0057_1b50, &args![actor]);
        });
        assert_eq!(log.len(), 2);
    }

    #[test]
    fn remove_weapon_detaches_the_held_weapon_node_and_tells_the_loader_its_model() {
        let mut s = weapon_setup(0, 0);
        let (actor, node, path) = (s.actor, s.node, s.path);
        let held = s.e.mem.alloc(0x100);
        let vtable = s.e.mem.u32(s.process);
        s.e.register_double(vtable + 0x1a0, move |_, _| ret(held));
        returns(&mut s.e, 0x0045_3470, 1);
        let backpack = s.e.mem.alloc(0x40);
        let parent = object(&mut s.e, 0x40, &[(0xe8, 0)]);
        s.e.mem.set_u32(backpack + 0x18, parent);
        s.e.register_double(0x004a_ae30, move |_, a| {
            ret(if a[1] == 0x0101_f5cc { backpack } else { 0 })
        });
        let loader = model_loader(&s.e);
        let log = logged(&mut s.e, |e| {
            e.call(0x0057_1b50, &args![actor]);
        });
        // the worn weapon is looked up in slot 5 and marked
        assert_eq!(calls_to(&log, 0x004c_8c10)[0][1..], [5, 0]);
        // the backpack leaves its parent
        assert_eq!(
            calls_to(&log, slot(&s.e, parent, 0xe8)),
            vec![vec![parent, backpack]]
        );
        // the loader hears the model of the worn weapon
        assert_eq!(calls_to(&log, 0x0045_a5e0), vec![vec![loader, path]]);
        // the held node leaves the shadow scene node and its effects are cleared
        assert_eq!(calls_to(&log, 0x00b5_b1c0), vec![vec![0x5c5c, held]]);
        assert_eq!(calls_to(&log, 0x004d_ffa0), vec![vec![held + 0x9c]]);
        // the actor refreshes its lighting
        assert_eq!(calls_to(&log, 0x008b_0bd0), vec![vec![actor, node]]);
    }

    #[test]
    fn remove_weapon_finds_the_weapon_child_of_the_3d_when_the_process_has_none() {
        let mut s = weapon_setup(0, 0);
        let (actor, node, path) = (s.actor, s.node, s.path);
        let child = s.e.mem.alloc(0x100);
        s.e.register_double(0x004a_ae30, move |_, a| {
            ret(if a[0] == node && a[1] == 0x0101_3be8 {
                child
            } else {
                0
            })
        });
        let loader = model_loader(&s.e);
        let log = logged(&mut s.e, |e| {
            e.call(0x0057_1b50, &args![actor]);
        });
        assert_eq!(calls_to(&log, 0x0045_a5e0), vec![vec![loader, path]]);
        // the child is only removed from the scene when the actor's virtual asks
        assert!(calls_to(&log, 0x00b5_b1c0).is_empty());

        let vtable = s.e.mem.u32(actor);
        s.e.register(vtable + 0x21c, |_, _| ret(1));
        let log = logged(&mut s.e, |e| {
            e.call(0x0057_1b50, &args![actor]);
        });
        assert_eq!(calls_to(&log, 0x00b5_b1c0), vec![vec![0x5c5c, child]]);
        assert_eq!(calls_to(&log, 0x004d_ffa0), vec![vec![child + 0x9c]]);
    }

    #[test]
    fn remove_weapon_recognises_the_named_entry_of_the_process_list() {
        let mut s = weapon_setup(0, 0);
        let (actor, process, path) = (s.actor, s.process, s.path);
        let list = s.e.mem.alloc(0x100);
        let vtable = s.e.mem.u32(process);
        s.e.register_double(vtable + 0x1a4, move |_, _| ret(list));
        returns(&mut s.e, 0x0043_b480, 2);
        let first = s.e.mem.alloc(0x10);
        let second = s.e.mem.alloc(0x10);
        s.e.register_double(0x0043_b4a0, move |_, a| {
            ret(if a[1] == 0 { first } else { second })
        });
        // only the second entry has the wanted name
        s.e.register_double(0x0041_3f40, |_, a| ret(a[0]));
        s.e.register_double(0x0043_b1b0, |_, a| ret(a[0] + 1));
        s.e.register_double(0x0040_8b20, move |_, a| {
            ret(if a[0] == second + 1 { 0 } else { 1 })
        });
        returns(&mut s.e, 0x0045_3470, 1);
        let loader = model_loader(&s.e);
        let log = logged(&mut s.e, |e| {
            e.call(0x0057_1b50, &args![actor]);
        });
        let compared = calls_to(&log, 0x0040_8b20);
        assert_eq!(compared[0], vec![first + 1, 0x0103_0e44]);
        assert_eq!(compared[1], vec![second + 1, 0x0103_0e44]);
        assert_eq!(calls_to(&log, 0x0045_a5e0), vec![vec![loader, path]]);
        // the list is cleared at the end
        assert_eq!(calls_to(&log, 0x00b5_b1c0), vec![vec![0x5c5c, list]]);

        // no entry matches: the list is left alone
        returns(&mut s.e, 0x0040_8b20, 1);
        let log = logged(&mut s.e, |e| {
            e.call(0x0057_1b50, &args![actor]);
        });
        assert!(calls_to(&log, 0x00b5_b1c0).is_empty());
        assert!(calls_to(&log, 0x0045_a5e0).is_empty());
    }

    #[test]
    fn remove_weapon_of_the_player_leaves_iron_sights_and_vats() {
        let mut s = weapon_setup(0, 0);
        let actor = s.actor;
        s.e.set_global(GLOBAL_PLAYER, actor);
        // iron sights: the player flag bytes
        s.e.mem.set_u8(actor + 0x64f, 1);
        returns(&mut s.e, 0x008b_bc10, 1);
        s.e.register_double(0x0095_0b00, |_, a| {
            ret(if a[1] == 1 { 0xb1b1 } else { 0xb2b2 })
        });
        let log = logged(&mut s.e, |e| {
            e.call(0x0057_1b50, &args![actor]);
        });
        assert_eq!(calls_to(&log, 0x008b_b650), vec![vec![actor, 0, 0, 0]]);
        assert_eq!(calls_to(&log, 0x009c_8950), vec![vec![0x011f_2250, 0, 1]]);
        // the first-person and the third-person bipeds both drop the weapon
        assert_eq!(
            calls_to(&log, 0x004a_b5b0),
            vec![vec![0xb1b1], vec![0xb2b2]]
        );
        assert_eq!(calls_to(&log, 0x0070_9c40), vec![vec![0]]);
        assert_eq!(calls_to(&log, 0x0070_9ca0).len(), 1);
        // the flag byte that the biped removal needs is back to 0
        assert_eq!(s.e.global::<u8>(0x011d_cfa7), 0);

        // not in iron sights: no iron-sights exit
        s.e.mem.set_u8(actor + 0x64f, 0);
        let log = logged(&mut s.e, |e| {
            e.call(0x0057_1b50, &args![actor]);
        });
        assert!(calls_to(&log, 0x008b_b650).is_empty());
    }

    // ---- 00572160, 005721e0, 00572220, 00572230 ----------------------------

    #[test]
    fn effects_of_a_node_are_released_one_by_one_and_the_array_emptied() {
        let mut e = engine();
        returns(&mut e, 0x0065_8930, 2);
        let effect = e.mem.alloc(0x10);
        let first = e.mem.alloc(0x10);
        e.mem.set_u32(first, effect);
        let second = e.mem.alloc(0x10);
        e.register_double(0x0087_7a30, move |_, a| {
            ret(if a[1] == 0 { first } else { second })
        });
        let node = e.mem.alloc(0x100);
        let log = logged(&mut e, |e| {
            e.call(0x0057_2160, &args![node]);
        });
        assert_eq!(calls_to(&log, 0x0065_8930), vec![vec![node + 0x9c]]);
        assert_eq!(
            calls_to(&log, 0x0087_7a30),
            vec![vec![node + 0x9c, 0], vec![node + 0x9c, 1]]
        );
        // only the element with a pointer is released
        assert_eq!(calls_to(&log, 0x0050_f5a0), vec![vec![effect]]);
        assert_eq!(log.last().unwrap(), &(0x004d_ffa0, vec![node + 0x9c]));
    }

    #[test]
    fn iron_sights_test_reads_the_two_flag_bytes_of_the_player() {
        let mut e = engine();
        let player = e.mem.alloc(0x700);
        assert!(!e.call(0x0057_21e0, &args![player]).bool());
        e.mem.set_u8(player + 0x64d, 1);
        assert!(e.call(0x0057_21e0, &args![player]).bool());
        e.mem.set_u8(player + 0x64d, 0);
        e.mem.set_u8(player + 0x64f, 5);
        assert!(e.call(0x0057_21e0, &args![player]).bool());
    }

    #[test]
    fn flag_setter_00572220_stores_a_byte_in_the_global() {
        let mut e = engine();
        e.call(0x0057_2220, &args![1u8]);
        assert_eq!(e.global::<u8>(0x011d_cfa7), 1);
        e.call(0x0057_2220, &args![0u8]);
        assert_eq!(e.global::<u8>(0x011d_cfa7), 0);
    }

    #[test]
    fn picking_up_removes_the_dropped_item_from_its_dropper_and_marks_it_deleted() {
        let mut e = engine();
        let refr = e.mem.alloc(0x100);
        let dropper = e.mem.alloc(0x100);
        returns(&mut e, 0x0041_de00, dropper);
        let log = logged(&mut e, |e| {
            e.call(0x0057_2230, &args![refr]);
        });
        assert_eq!(calls_to(&log, 0x0041_de00), vec![vec![refr + 0x44]]);
        assert_eq!(
            calls_to(&log, 0x0041_e0d0),
            vec![vec![dropper + 0x44, refr]]
        );
        assert_eq!(log.last().unwrap(), &(0x0057_2270, vec![refr]));

        // nobody dropped it: only the deletion mark
        returns(&mut e, 0x0041_de00, 0);
        let log = logged(&mut e, |e| {
            e.call(0x0057_2230, &args![refr]);
        });
        assert!(calls_to(&log, 0x0041_e0d0).is_empty());
        assert_eq!(calls_to(&log, 0x0057_2270), vec![vec![refr]]);
    }
}
