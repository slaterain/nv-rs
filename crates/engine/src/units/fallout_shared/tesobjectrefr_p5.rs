//! `fallout shared/tesobjectrefr.cpp` (Xbox PDB source unit), part 5: its functions from `00576870` up to
//! (not including) `0057cac0` in FalloutNV.exe 1.4.0.525, translated
//! (docs/ENGINE_CRATE.md). The unit's shared layouts and helpers are in
//! [`super::tesobjectrefr`]; anything public there may be used here.
//!
//! # What is in this part
//!
//! This part holds the batch `00576870` to `00578970` (the first 40 open
//! functions of the range). It is mostly the glue between a reference and
//! its scene-graph and Havok objects: the transform-change callback
//! (`TransChangeCallback`, `00577450`), the recursive addon-node walkers
//! (`AddAddonNodes`, `RemoveAddonNodes`, ...), the owner tests
//! (`IsAnOwner`, `IsOwnerEvil`) and the small accessors the linker placed
//! between them. The next session continues with the next open function
//! after `00578970` in address order (`005789b0`).
//!
//! # Conventions of the code read here
//!
//! * The compiler's pushes in front of a call often belong to a later call:
//!   `PUSH x; CALL getter; MOV ECX,EAX; CALL method` hands `x` to `method`.
//!   Every call's argument words below were checked against the callee's
//!   `RET n` (or the caller's stack cleanup).
//! * Callees that take no stack word and ignore `ECX` are called without
//!   arguments (`00453a70`, `00448a80`, `0045a190`, `00458b20`).
//! * Locals the game keeps on its stack and passes by address (vectors,
//!   transforms) are scratch blocks of the engine's heap, freed before the
//!   function returns.
//! * C++ exception-unwinding frames (`00577ca0`) and stack-cookie checks are
//!   not translated.

#[allow(unused_imports)]
use super::tesobjectrefr::*;
#[allow(unused_imports)]
use crate::prelude::*;

/// `005d43c0`: the reference's embedded `ExtraDataList` (`this + 0x44`).
const GET_EXTRA_LIST: u32 = 0x005d_43c0;
/// `BGSSaveFormBuffer::GetForm` (engine map name; the body returns the
/// reference's base object, `pObjectReference`).
const GET_BASE_FORM: u32 = 0x007a_f430;
/// The form type byte (`cFormType`, at +4) of a form.
const FORM_TYPE: u32 = 0x0040_1170;
/// `__RTDynamicCast(object, 0, from, to, 0)`.
const DYNAMIC_CAST: u32 = 0x00ec_43fb;
/// `_finite(double)` and `_isnan(double)` of the C runtime.
const CRT_FINITE: u32 = 0x00ec_7595;
const CRT_ISNAN: u32 = 0x00ec_75b1;
/// `strncmp`.
const STRNCMP: u32 = 0x00ec_8a19;
/// The reference's parent cell (`pParentCell`, or the child-cell fallback).
const GET_PARENT_CELL: u32 = 0x008d_6f30;
/// Prints a message to the game's log (`format, args...`, cdecl).
const MESSAGE: u32 = 0x005b_5e40;

/// `0043b300(type, object)`: true when `object` is an instance of the
/// run-time type described by the descriptor at `type` (cdecl).
const IS_KIND_OF: u32 = 0x0043_b300;
/// Number of children of a scene-graph node.
const CHILD_COUNT: u32 = 0x0043_b480;
/// Child `index` of a scene-graph node.
const CHILD_AT: u32 = 0x0043_b4a0;
/// The run-time type descriptor of the scene-graph node class the addon
/// walkers accept (passed to [`IS_KIND_OF`]).
const NODE_TYPE: u32 = 0x0120_2de8;
/// `AsNode`: the node a scene-graph object is (virtual slot at +0x0c).
const SLOT_AS_NODE: u32 = 0x0c;
/// A reference's 3D (the virtual at +0x1d0, no stack word; the Actor
/// override of the slot is called the same way).
const SLOT_GET_3D: u32 = 0x1d0;
/// `IsActor` (virtual at +0x100).
const SLOT_IS_ACTOR: u32 = 0x100;
/// Writes the reference's position into the buffer it is given and returns a
/// pointer to the position (virtual at +0x170).
const SLOT_GET_POSITION: u32 = 0x170;
/// Returns a pointer to the reference's position (virtual at +0x1f4).
const SLOT_POSITION_PTR: u32 = 0x1f4;

/// `TESDataHandler` singleton (global `011c3f2c`).
const GLOBAL_DATA_HANDLER: u32 = 0x011c_3f2c;
/// `TES` singleton (global `011dea10`).
const GLOBAL_TES: u32 = 0x011d_ea10;
/// The object whose byte flag [`SWAP_FLAG`] and `0042ce10` read (global
/// `011ddf38`; the main file calls it the loading object).
const GLOBAL_FLAG_OBJECT: u32 = 0x011d_df38;
/// The player character (global `011dea3c`).
const GLOBAL_PLAYER: u32 = 0x011d_ea3c;
/// Base-form pointers compared with a reference's base object.
const GLOBAL_BASE_FORM_A: u32 = 0x011c_a240;
const GLOBAL_BASE_FORM_B: u32 = 0x011c_a27c;
const GLOBAL_BASE_FORM_C: u32 = 0x011c_a280;

/// `0x011f426c`: a zero vector (three floats).
const ZERO_VECTOR: u32 = 0x011f_426c;
/// Sets the byte flag of the object it is called on and returns the old one.
const SWAP_FLAG: u32 = 0x0046_23f0;
/// Moves a reference's position to the given point (pointer to three floats).
const SET_POSITION: u32 = 0x0057_5830;
/// Sets a reference's rotation (three euler angles by value).
const SET_ROTATION: u32 = 0x0057_5700;

/// Form type numbers compared with `cFormType` here; only the numbers are
/// known from the code.
const FORM_TYPE_NPC_LIKE: u32 = 0x08;
const FORM_TYPE_FACTION_LIKE: u32 = 0x2a;

/// `double` 128.0: how far below the terrain minimum a reference counts as
/// fallen out of the world.
const GROUND_MARGIN: u32 = 0x0102_e430;
/// `double` constants of `TransChangeCallback`.
const VELOCITY_SCALE: u32 = 0x0102_90b0;
const VELOCITY_LIMIT: u32 = 0x0102_0998;
/// `float` -1.0 used as the velocity of a pushed body.
const PUSH_VELOCITY: u32 = 0x0101_2054;
/// `float` threshold of the stillness test in `005768b0`.
const STILL_THRESHOLD: u32 = 0x0101_7d00;
/// Object whose address is `this` of the Havok vector helper (`0045bb20`).
const HAVOK_VECTOR_HELPER: u32 = 0x011a_9484;
/// Table of `float`s indexed by a `hkUFloat8` byte (`00577bf0`).
const UFLOAT8_TABLE: u32 = 0x010c_7948;
/// The message "HAVOK: Disabling collision on ref '%s' (%08X)."
const DISABLE_COLLISION_MESSAGE: u32 = 0x0103_0ed8;
/// "BASE", the name prefix of the addon-node markers.
const BASE_PREFIX: u32 = 0x0103_0f08;
/// The empty string literal.
const EMPTY_STRING: u32 = 0x0101_1584;
/// Divisor of the addon randomizer's seed (`double`).
const RANDOM_DIVISOR: u32 = 0x0101_7a40;
/// `double` compared with a global's value in `IsAnOwner`.
const OWNERSHIP_VALUE_LIMIT: u32 = 0x0101_2060;

/// `_finite` on a `float`.
fn crt_finite(e: &mut Engine, value: f32) -> bool {
    e.call(CRT_FINITE, &args![value as f64]).i32() != 0
}

/// `_isnan` on a `float`.
fn crt_isnan(e: &mut Engine, value: f32) -> bool {
    e.call(CRT_ISNAN, &args![value as f64]).i32() != 0
}

fn is_node(e: &mut Engine, object: u32) -> bool {
    e.call(IS_KIND_OF, &args![NODE_TYPE, object]).bool()
}

fn child_count(e: &mut Engine, node: u32) -> u32 {
    e.call(CHILD_COUNT, &args![node]).u32()
}

fn child_at(e: &mut Engine, node: u32, index: u32) -> u32 {
    e.call(CHILD_AT, &args![node, index]).u32()
}

/// The child's `AsNode` (virtual at +0x0c), 0 for a null child.
fn child_node(e: &mut Engine, child: u32) -> u32 {
    if child == 0 {
        0
    } else {
        e.vcall(child, SLOT_AS_NODE, &args![]).u32()
    }
}

/// The addon record the data handler holds for `node` (`009ee040(node)` is
/// the key handed to `004617e0`).
fn addon_for_node(e: &mut Engine, node: u32) -> u32 {
    let key = e.call(0x009e_e040, &args![node]).u32();
    let handler = global_word(e, GLOBAL_DATA_HANDLER);
    e.call(0x0046_17e0, &args![handler, key]).u32()
}

fn global_word(e: &Engine, addr: u32) -> u32 {
    e.global::<u32>(addr)
}

/// `body` of a collision object, or 0 (`006fa820` guarded by a null test).
fn body_of(e: &mut Engine, collision: u32) -> u32 {
    if collision == 0 {
        0
    } else {
        e.call(0x006f_a820, &args![collision]).u32()
    }
}

// Translated from 00576870 (decompiled, FalloutNV.exe 1.4.0.525)
/// Looks up the element with form ID `form_id` in the list at `this + 4`
/// (`0057c400`) and returns the address 4 bytes before it, or 0.
pub fn fn_00576870(e: &mut Engine, this: Ptr, form_id: u32) -> u32 {
    let found = e.call(0x0057_c400, &args![this.addr() + 4, form_id]).u32();
    if found == 0 {
        0
    } else {
        found - 4
    }
}

// Translated from 005768b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Called when a reference's 3D has just been loaded: initializes the
/// Havok objects its base form needs (placeable water, primitive trigger,
/// ...), hands the loaded 3D's collision object to the cell's Havok world,
/// sets the motion type of dynamic bodies, and finishes with `0056c880`.
///
/// The world calls (virtual slots `0xd0`, `0xd4` of the cell's world) are
/// the ones the code makes; the names of the callees are not known from the
/// exe.
pub fn fn_005768b0(e: &mut Engine, this: Ptr<TESObjectREFR>) {
    let me = this.addr();
    let cell = e.get(this, TESObjectREFR::pParentCell);
    if !cell.is_null()
        && !e.call(0x0045_0ff0, &args![cell]).bool()
        && !e.call(0x0045_23c0, &args![cell]).bool()
    {
        return;
    }
    let model = e.vcall(me, SLOT_GET_3D, &args![]).u32();
    if model != 0 {
        let tes = global_word(e, GLOBAL_TES);
        e.call(0x0045_3860, &args![tes, 1u32]);
        init_havok_for_base_form(e, this);
        handle_loaded_model(e, this, model);
        let tes = global_word(e, GLOBAL_TES);
        e.call(0x0045_3860, &args![tes, 0u32]);
    }
    e.call(0x0056_c880, &args![this, 1u32, 1u32]);
}

/// The first half of `005768b0`: unless `004523e0(this, 1)`, picks the Havok
/// initializer by the base form.
fn init_havok_for_base_form(e: &mut Engine, this: Ptr<TESObjectREFR>) {
    if e.call(0x0045_23e0, &args![this, 1u32]).bool() {
        return;
    }
    let base = e.call(GET_BASE_FORM, &args![this]).u32();
    if e.call(FORM_TYPE, &args![base]).u32() == 0x23 {
        let base = e.call(GET_BASE_FORM, &args![this]).u32();
        if !e.call(0x0045_2440, &args![base]).bool() {
            // the placeable-water initializer
            e.call(0x0056_c8f0, &args![this]);
        }
        return;
    }
    if e.call(GET_BASE_FORM, &args![this]).u32() == global_word(e, GLOBAL_BASE_FORM_A) {
        e.call(0x0056_eab0, &args![this]);
        return;
    }
    let base = e.call(GET_BASE_FORM, &args![this]).u32();
    if e.call(FORM_TYPE, &args![base]).u32() == 0x0e {
        e.call(0x0056_f140, &args![this]);
        return;
    }
    let extra = e.call(GET_EXTRA_LIST, &args![this]).u32();
    if e.call(0x0041_fbe0, &args![extra]).u32() != 0 {
        let base = e.call(GET_BASE_FORM, &args![this]).u32();
        if e.call(FORM_TYPE, &args![base]).u32() == 0x15 {
            // the primitive-trigger initializer
            e.call(0x0056_d7e0, &args![this]);
        }
    }
}

/// The second half of `005768b0`, for a reference that has a 3D `model`.
fn handle_loaded_model(e: &mut Engine, this: Ptr<TESObjectREFR>, model: u32) {
    let me = this.addr();
    let model_node = e.vcall(model, SLOT_AS_NODE, &args![]).u32();
    let mut target = model;
    if !e.vcall(me, SLOT_IS_ACTOR, &args![]).bool() {
        let found = e.call(0x00c8_12d0, &args![model_node, 1u32]).u32();
        if found != 0 {
            target = e.call(0x0044_ddc0, &args![found]).u32();
        }
    }
    if target == 0 {
        // no target: a model of the `011d5e70` type is offered to the
        // cell's world through its slot 0xd0
        if e.call(IS_KIND_OF, &args![0x011d_5e70u32, model]).bool() {
            let cell = e.call(GET_PARENT_CELL, &args![this]).u32();
            if cell != 0 {
                let world = e.call(0x0045_43c0, &args![cell]).u32();
                if world != 0 {
                    e.vcall(world, 0xd0, &args![model, 0u32, 0u32, 0u32, 0u32]);
                }
            }
        }
        return;
    }
    let cell = e.call(GET_PARENT_CELL, &args![this]).u32();
    if cell == 0 {
        return;
    }
    let world = e.call(0x0045_43c0, &args![cell]).u32();
    let collision = e.call(0x004b_5260, &args![target]).u32();
    let body = body_of(e, collision);
    if world == 0 {
        return;
    }
    if body != 0 {
        let body_world = e.vcall(body, 0x94, &args![]).u32();
        let this_world = e.vcall(world, 0x94, &args![]).u32();
        if body_world == this_world {
            return;
        }
    }
    e.vcall(world, 0xd4, &args![target, model]);

    let base = e.call(GET_BASE_FORM, &args![this]).u32();
    let set_dynamic = base == global_word(e, GLOBAL_BASE_FORM_B)
        || e.call(GET_BASE_FORM, &args![this]).u32() == global_word(e, GLOBAL_BASE_FORM_C);
    if set_dynamic {
        let body = body_of(e, collision);
        let shape = e.call(0x0056_0d80, &args![body]).u32();
        let probe = e.mem.alloc(16);
        e.call(0x004a_3c90, &args![probe, shape]);
        let flag_object = global_word(e, GLOBAL_FLAG_OBJECT);
        let mut motion = false;
        let mut test_moving = true;
        if (e.call(0x0042_ce10, &args![flag_object]).bool() || fn_00576d30(e, this.cast()))
            && body != 0
        {
            let still_object = e.call(0x0045_8b20, &args![]).u32();
            let threshold: f32 = e.global(STILL_THRESHOLD);
            if e.call(0x0045_8900, &args![probe, still_object, threshold])
                .u32()
                != 0
            {
                motion = true;
                test_moving = false;
            }
        }
        if test_moving && !e.call(0x0056_4c10, &args![this]).bool() {
            motion = true;
        }
        if motion {
            e.call(0x00c6_a350, &args![model, 5u32, 1u32, 0u32, 1u32]);
        }
        e.mem.free(probe);
    }
    let base = e.call(GET_BASE_FORM, &args![this]).u32();
    let base_type = e.call(FORM_TYPE, &args![base]).i32();
    if (0x20..=0x21).contains(&base_type) {
        e.call(0x00c6_a350, &args![model, 5u32, 1u32, 0u32, 1u32]);
    } else if e.call(0x0056_4c10, &args![this]).bool() {
        let body = body_of(e, collision);
        let rigid = if body != 0 {
            e.call(0x0062_0b80, &args![body]).u32()
        } else {
            0
        };
        if rigid != 0 {
            let flags = e.call(0x009d_9f40, &args![rigid]).u32();
            if e.call(0x0051_7690, &args![flags]).i32() != 5 {
                let flags = e.call(0x009d_9f40, &args![rigid]).u32();
                if e.call(0x0051_7690, &args![flags]).i32() != 4 {
                    let velocity = e.mem.alloc(16);
                    let down: f32 = e.global(PUSH_VELOCITY);
                    e.call(0x0045_bb20, &args![HAVOK_VECTOR_HELPER, velocity, down]);
                    e.call(0x0062_b8d0, &args![model, velocity, 0u32]);
                    e.mem.free(velocity);
                }
            }
        }
        e.call(0x0056_4c60, &args![this, 0u32]);
    }
    e.call(0x0086_a830, &args![this, model]);
}

// Translated from 00576d30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the form flag `0x200000` (`TESForm::iFormFlags`, +0x08) is set.
pub fn fn_00576d30(e: &mut Engine, this: Ptr) -> bool {
    // TESForm::iFormFlags +0x08
    e.mem.u32(this.addr() + 8) & 0x0020_0000 != 0
}

// Translated from 00576d50 (decompiled, FalloutNV.exe 1.4.0.525)
/// Walks a scene-graph object and its children (`this` is only handed on to
/// the recursion): converts each node's world
/// transform into a Havok transform and passes it to the collision shape's
/// two setters (`00ca4120` / `00ca36a0`, by shape type).
///
/// For a node with a collision object the rigid body's shape is looked up;
/// a shape of the first two types (descriptors `012682e8` / `012682dc`) is
/// used directly, one of the third type (`012682d0`) through the shape it
/// wraps (`00577190`).
pub fn fn_00576d50(e: &mut Engine, _unused_0: Ptr, node: Ptr) {
    if node.is_null() {
        return;
    }
    let n = node.addr();
    // a NiPoint3 local, built as (0, 1, 0) and then filled by the virtual
    // at +0xb8 (it writes the vector it is given)
    let direction = e.mem.alloc(16);
    e.call(0x0043_d410, &args![direction, 0.0f32, 1u32, 0u32]);
    e.vcall(n, 0xb8, &args![direction]);
    let transform = e.call(0x0046_1130, &args![node]).u32();
    let copy = e.mem.alloc(52);
    for word in 0..13 {
        let value = e.mem.u32(transform + word * 4);
        e.mem.set_u32(copy + word * 4, value);
    }
    let havok_transform = e.mem.alloc(72);
    e.call(0x0056_ded0, &args![havok_transform]);
    fn_00577070(e, Ptr::new(havok_transform));
    e.call(0x0056_df80, &args![havok_transform, copy]);

    let collision = e.call(0x0043_b610, &args![node]).u32();
    let body = body_of(e, collision);
    let shape = if body != 0 {
        e.call(0x004a_e6a0, &args![body]).u32()
    } else {
        0
    };
    if shape != 0 {
        let mut first_kind = 0u32;
        let mut second_kind = 0u32;
        if e.call(IS_KIND_OF, &args![0x0126_82e8u32, shape]).bool() {
            first_kind = e.call(0x0062_0b80, &args![shape]).u32();
        } else if e.call(IS_KIND_OF, &args![0x0126_82dcu32, shape]).bool() {
            second_kind = e.call(0x0062_0b80, &args![shape]).u32();
        } else if e.call(IS_KIND_OF, &args![0x0126_82d0u32, shape]).bool() {
            let inner = fn_00577190(e, Ptr::new(shape));
            if inner != 0 && e.call(IS_KIND_OF, &args![0x0126_82e8u32, inner]).bool() {
                first_kind = e.call(0x0062_0b80, &args![inner]).u32();
            } else if inner != 0 && e.call(IS_KIND_OF, &args![0x0126_82dcu32, inner]).bool() {
                second_kind = e.call(0x0062_0b80, &args![inner]).u32();
            }
        }
        if first_kind != 0 {
            e.call(0x00ca_4120, &args![first_kind, havok_transform]);
        }
        if second_kind != 0 {
            e.call(0x00ca_36a0, &args![second_kind, havok_transform]);
        }
    }
    e.mem.free(direction);
    e.mem.free(copy);
    e.mem.free(havok_transform);

    let children = e.vcall(n, SLOT_AS_NODE, &args![]).u32();
    if children != 0 {
        let count = child_count(e, children);
        for index in 0..count {
            let child = child_at(e, children, index);
            fn_00576d50(e, _unused_0, Ptr::new(child));
        }
    }
}

// Translated from 00577070 (decompiled, FalloutNV.exe 1.4.0.525)
/// Resets a 72-byte Havok transform: identity rotation (`00577090`) and a
/// zero translation vector at +0x30.
pub fn fn_00577070(e: &mut Engine, this: Ptr) {
    fn_00577090(e, this);
    e.call(0x0053_8f40, &args![this.addr() + 0x30]);
}

// Translated from 00577090 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the three rows of a 3x4 matrix at `this` to the identity: zeroes
/// them (`00577110`) and writes 1.0 on the diagonal.
pub fn fn_00577090(e: &mut Engine, this: Ptr) {
    fn_00577110(e, this);
    for k in 0..3u32 {
        let cell = fn_005770e0(e, this, k, k);
        e.mem.set_f32(cell, 1.0);
    }
}

// Translated from 005770e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Address of element (`column`, `row`) of a matrix of 16-byte rows:
/// `this + row * 16 + column * 4`.
pub fn fn_005770e0(e: &mut Engine, this: Ptr, column: u32, row: u32) -> u32 {
    let row_address = e.call(0x004b_4cf0, &args![this, row]).u32();
    e.call(0x0056_0d30, &args![row_address, column]).u32()
}

// Translated from 00577110 (decompiled, FalloutNV.exe 1.4.0.525)
/// Zeroes the three rows (16 bytes each) of the matrix at `this`.
pub fn fn_00577110(e: &mut Engine, this: Ptr) {
    let zero = e.mem.alloc(16);
    e.call(0x0068_15c0, &args![zero]);
    e.call(0x0053_8f40, &args![zero]);
    for row in 0..3u32 {
        let row_address = e.call(0x004b_4cf0, &args![this, row]).u32();
        e.call(0x004a_3f10, &args![row_address, zero]);
    }
    e.mem.free(zero);
}

// Translated from 00577190 (decompiled, FalloutNV.exe 1.4.0.525)
/// The shape wrapped by a collision shape: `005771f0` of it, converted by
/// `005771d0`, or 0.
pub fn fn_00577190(e: &mut Engine, this: Ptr) -> u32 {
    let inner = fn_005771f0(e, this);
    if inner == 0 {
        0
    } else {
        fn_005771d0(e, inner)
    }
}

// Translated from 005771d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `cdecl` wrapper of `004a3a40(value)`.
pub fn fn_005771d0(e: &mut Engine, value: u32) -> u32 {
    e.call(0x004a_3a40, &args![value]).u32()
}

// Translated from 005771f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The object `00577190` works on: `004ae750(this)`, then its virtual at
/// +0x10, 0x10 bytes before the pointer it returns (0 when either is null).
pub fn fn_005771f0(e: &mut Engine, this: Ptr) -> u32 {
    let first = e.call(0x004a_e750, &args![this]).u32();
    if first == 0 {
        return 0;
    }
    let second = e.vcall(first, 0x10, &args![]).u32();
    if second == 0 {
        0
    } else {
        second - 0x10
    }
}

// Translated from 00577250 (decompiled, FalloutNV.exe 1.4.0.525)
/// A float derived from the reference's container (the extra data's
/// `00418520` list value, else `00481e10` of the container) and limited for
/// actors: a non-player actor is capped at `008a0c20`'s value, the player
/// gets `00577310` added.
pub fn fn_00577250(e: &mut Engine, this: Ptr, flag: u8) -> f32 {
    let mut value = 0.0f32;
    if e.call(0x0055_d310, &args![this]).u32() != 0 {
        let extra = e.call(GET_EXTRA_LIST, &args![this]).u32();
        let list = e.call(0x0041_8520, &args![extra]).u32();
        if list != 0 {
            value = e.call(0x004d_0900, &args![list, flag]).f32();
        } else {
            let container = e.call(0x0055_d310, &args![this]).u32();
            value = e.call(0x0048_1e10, &args![container, flag]).f32();
        }
    }
    if e.vcall(this.addr(), SLOT_IS_ACTOR, &args![]).bool() {
        let player = global_word(e, GLOBAL_PLAYER);
        if this.addr() != player {
            let cap = e.call(0x008a_0c20, &args![this]).f32();
            if value as f64 > cap as f64 {
                value = e.call(0x008a_0c20, &args![this]).f32();
            }
        } else {
            let bonus = fn_00577310(e, Ptr::new(player));
            value = (bonus as f64 + value as f64) as f32;
        }
    }
    value
}

// Translated from 00577310 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `float` at +0xe04 of the object (the player character when called
/// from `00577250`).
pub fn fn_00577310(e: &mut Engine, this: Ptr) -> f32 {
    e.mem.f32(this.addr() + 0xe04)
}

// Translated from 00577330 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::RestoreRagDollData` (Xbox PDB): when `0043fcd0(this)`
/// (the reference's 3D) is non-null, applies the ragdoll data `data` (taken
/// from the reference's extra data list when `data` is null) to the
/// reference (`004d9910`); without a 3D, stores `data` in the extra data list
/// (`0041d5b0`).
pub fn tes_object_refr_restore_rag_doll_data(e: &mut Engine, this: Ptr, data: u32) {
    if e.call(0x0043_fcd0, &args![this]).u32() != 0 {
        let mut data = data;
        if data == 0 {
            let extra = e.call(GET_EXTRA_LIST, &args![this]).u32();
            data = e.call(0x0041_d6d0, &args![extra]).u32();
        }
        if data != 0 {
            e.call(0x004d_9910, &args![data, this]);
        }
    } else if data != 0 {
        let extra = e.call(GET_EXTRA_LIST, &args![this]).u32();
        e.call(0x0041_d5b0, &args![extra, data]);
    }
}

// Translated from 00577390 (decompiled, FalloutNV.exe 1.4.0.525)
/// For a reference whose extra data holds a primitive that `0056c7f0`
/// accepts: reads the primitive's 16-byte value (`00413f90`), sets its last
/// float to the `float` at `0101df44` (`enable`) or to 0.0, hands the value
/// back through the primitive's virtual at +8, and, unless the base form is
/// the one in `011ca240` and when the reference has a 3D, calls
/// `00450f90(3D, enable == 0)` on the 3D.
pub fn fn_00577390(e: &mut Engine, this: Ptr, enable: u8) {
    let extra = e.call(GET_EXTRA_LIST, &args![this]).u32();
    let primitive = e.call(0x0041_fbe0, &args![extra]).u32();
    if primitive == 0 || e.call(0x0056_c7f0, &args![primitive]).u32() == 0 {
        return;
    }
    let class_id = e.mem.alloc(16);
    e.call(0x0041_3f90, &args![primitive, class_id]);
    let value: f32 = if enable != 0 {
        e.global(0x0101_df44)
    } else {
        0.0
    };
    // the last four bytes of the 16-byte ID
    e.mem.set_f32(class_id + 12, value);
    e.vcall(primitive, 8, &args![class_id]);
    e.mem.free(class_id);
    if e.call(GET_BASE_FORM, &args![this]).u32() != global_word(e, GLOBAL_BASE_FORM_A)
        && e.vcall(this.addr(), SLOT_GET_3D, &args![]).u32() != 0
    {
        let model = e.vcall(this.addr(), SLOT_GET_3D, &args![]).u32();
        e.call(0x0045_0f90, &args![model, (enable == 0) as u32]);
    }
}

// Translated from 00577450 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::TransChangeCallback` (Xbox PDB): the scene-graph node
/// `node` of a reference has moved (`flags & 1`) or rotated (`flags & 2`);
/// brings the reference in line.
///
/// With `flags & 1` it checks the new position: not finite, or below the
/// terrain minimum (exterior) or the cell's bound (interior), means the
/// reference fell out of the world, and it is put back at the position
/// the cell's placement info (`0054cfd0`) gives (or its Havok body is frozen
/// when it moves too fast). It then writes the position back to the
/// reference (`00575830`), sinks water-bound references' addon nodes, and,
/// outdoors, moves the reference to the cell of its new coordinates.
/// With `flags & 2` it converts the node's rotation matrix to euler angles
/// and writes them to the reference (`00575700`). It always finishes by
/// refreshing the 3D's lighting.
pub fn tes_object_refr_trans_change_callback(e: &mut Engine, node: Ptr, flags: u32) {
    let node_owner = e.call(0x0044_ddc0, &args![node]).u32();
    let refr = e.call(0x0056_f930, &args![node_owner]).u32();
    if refr == 0 {
        return;
    }
    let model = e.vcall(refr, SLOT_GET_3D, &args![]).u32();
    // the result (a ragdoll test) is not used
    e.call(0x0062_c3c0, &args![refr, model]);
    let scratch = e.mem.alloc(0x100);
    if flags & 1 != 0 {
        position_changed(e, node, node_owner, refr, scratch);
    }
    if flags & 2 != 0 {
        rotation_changed(e, node_owner, refr, scratch);
    }
    let model = e.vcall(refr, SLOT_GET_3D, &args![]).u32();
    let lighting = e.call(0x0045_0b80, &args![0u32]).u32();
    e.call(0x00b5_d9f0, &args![lighting, model, 1u32]);
    e.mem.free(scratch);
}

/// `flags & 1` half of [`tes_object_refr_trans_change_callback`]. `scratch`
/// is a block of 0x100 bytes for the function's stack locals.
fn position_changed(e: &mut Engine, node: Ptr, node_owner: u32, refr: u32, scratch: u32) {
    let cell = e.call(GET_PARENT_CELL, &args![refr]).u32();
    if cell != 0 {
        let (fell_out, below_both) = check_fell_out(e, node_owner, refr, cell, scratch);
        if fell_out {
            if below_both {
                // put the reference back at the cell's placement point
                let point = scratch + 0x30;
                for word in 0..3u32 {
                    let value = e.mem.u32(ZERO_VECTOR + word * 4);
                    e.mem.set_u32(point + word * 4, value);
                }
                let info = scratch + 0x40;
                e.call(0x0068_15c0, &args![info]);
                e.call(0x0054_cfd0, &args![cell, point, info]);
                let [x, y, z] = [e.mem.u32(point), e.mem.u32(point + 4), e.mem.u32(point + 8)];
                e.vcall(refr, 0x174, &args![x, y, z]);
            }
            let model = e.vcall(refr, SLOT_GET_3D, &args![]).u32();
            if model != 0 {
                let position = e
                    .vcall(refr, SLOT_GET_POSITION, &args![scratch + 0x50])
                    .u32();
                e.call(0x0044_0460, &args![model, position]);
                e.call(0x0043_fa80, &args![model, 0x011a_9448u32]);
            }
            e.call(0x00c6_bd00, &args![model, 1u32]);
            let direction = scratch + 0x60;
            e.call(0x0043_d410, &args![direction, 0.0f32, 1u32, 0u32]);
            e.call(0x00a5_9c60, &args![model, direction]);
            let body = e.call(0x006f_a820, &args![node]).u32();
            if body != 0 {
                let velocity = bhk_rigid_body_gethk_max_linear_velocity(e, Ptr::new(body));
                let scaled = (velocity as f64 * e.global::<f64>(VELOCITY_SCALE)) as f32;
                if scaled as f64 > e.global::<f64>(VELOCITY_LIMIT) {
                    fn_00577c40(e, Ptr::new(body), scaled);
                } else {
                    e.vcall(node.addr(), 0xb0, &args![4u32, 0u32, 0u32]);
                    e.call(0x0057_3ed0, &args![node, 0u32]);
                    let form_id = e.call(0x0084_e3a0, &args![refr]).u32();
                    let name = e.vcall(refr, 0x130, &args![]).u32();
                    e.call(MESSAGE, &args![DISABLE_COLLISION_MESSAGE, name, form_id]);
                }
            }
        }
    }

    // the reference follows the node: put the position back through the
    // reference's setter with the flag object's flag cleared
    let position = e.vcall(refr, SLOT_POSITION_PTR, &args![]).u32();
    let saved = scratch + 0x70;
    for word in 0..3u32 {
        let value = e.mem.u32(position + word * 4);
        e.mem.set_u32(saved + word * 4, value);
    }
    let flag_object = global_word(e, GLOBAL_FLAG_OBJECT);
    let old_flag = e.call(SWAP_FLAG, &args![flag_object, 0u32]).u8();
    let node_position = e.call(0x0045_bb80, &args![node_owner]).u32();
    e.call(SET_POSITION, &args![refr, node_position]);
    e.call(SWAP_FLAG, &args![flag_object, old_flag]);
    e.vcall(refr, 0x48, &args![4u32]);

    let base = e.call(GET_BASE_FORM, &args![refr]).u32();
    if base != 0 {
        let base = e.call(GET_BASE_FORM, &args![refr]).u32();
        if e.call(FORM_TYPE, &args![base]).u32() == 0x1e
            && !e.call(0x0044_8a20, &args![refr]).bool()
        {
            let position = e.vcall(refr, SLOT_POSITION_PTR, &args![]).u32();
            let water = e.call(0x0057_b0a0, &args![refr]).f32();
            if e.mem.f32(position + 8) < water {
                e.call(0x0048_4a70, &args![refr, 1u32]);
                let model = e.vcall(refr, SLOT_GET_3D, &args![]).u32();
                tes_object_refr_remove_addon_nodes(e, model);
                e.call(0x0057_29e0, &args![refr, 0u32]);
            }
        }
    }

    let tes = global_word(e, GLOBAL_TES);
    if e.call(0x005f_36f0, &args![tes]).u32() != 0 {
        return;
    }
    // outdoors: the reference may have crossed into another cell
    let world_space = e.call(0x004f_d3e0, &args![tes]).u32();
    let mut actor = 0;
    if refr != 0 && e.vcall(refr, SLOT_IS_ACTOR, &args![]).bool() {
        actor = refr;
    }
    if actor != 0 {
        e.call(0x008a_e640, &args![actor, 0u32]);
    }
    let at = e.vcall(refr, SLOT_POSITION_PTR, &args![]).u32();
    let y = e.mem.f32(at + 4);
    let at = e.vcall(refr, SLOT_POSITION_PTR, &args![]).u32();
    let x = e.mem.f32(at);
    let handler = global_word(e, GLOBAL_DATA_HANDLER);
    let new_cell = e
        .call(0x0046_1bc0, &args![handler, x, y, world_space, 0u32])
        .u32();
    if new_cell == cell {
        return;
    }
    if new_cell == 0 {
        // the new coordinates are in no cell: restore the saved position
        let old_flag = e.call(SWAP_FLAG, &args![flag_object, 0u32]).u8();
        e.call(SET_POSITION, &args![refr, saved]);
        e.call(SWAP_FLAG, &args![flag_object, old_flag]);
        e.call(0x0044_0460, &args![node_owner, saved]);
        e.vcall(node.addr(), 0xb0, &args![4u32, 0u32, 0u32]);
        return;
    }
    e.call(0x0057_3800, &args![refr, 0u32, world_space]);
    if !e.vcall(refr, SLOT_IS_ACTOR, &args![]).bool()
        && e.vcall(refr, SLOT_GET_3D, &args![]).u32() != 0
    {
        let attach = e.call(0x0054_97a0, &args![new_cell]).u32();
        if attach != 0 {
            if !e.call(0x008c_7aa0, &args![]).bool() {
                e.call(0x0054_96b0, &args![new_cell, refr, attach]);
            } else {
                e.call(0x0054_abd0, &args![refr, attach]);
            }
        }
    }
}

/// The position-validity tests of `TransChangeCallback`: returns (the
/// reference left the world, it is also below the world at its saved
/// position). `scratch` holds the stack locals.
fn check_fell_out(
    e: &mut Engine,
    node_owner: u32,
    refr: u32,
    cell: u32,
    scratch: u32,
) -> (bool, bool) {
    let position = e.call(0x0045_bb80, &args![node_owner]).u32();
    let [x, y, z] = [
        e.mem.f32(position),
        e.mem.f32(position + 4),
        e.mem.f32(position + 8),
    ];
    if !crt_finite(e, x)
        || !crt_finite(e, y)
        || !crt_finite(e, z)
        || crt_isnan(e, x)
        || crt_isnan(e, y)
        || crt_isnan(e, z)
    {
        return (true, false);
    }
    let mut fell_out = false;
    let mut below_both = false;
    if e.call(0x0042_5fd0, &args![cell]).bool() {
        // interior cell: compare with the cell's bounding sphere
        let bounds_node = e.call(0x0045_6fc0, &args![cell, 3u32]).u32();
        if bounds_node != 0 {
            let source = e.call(0x0043_d450, &args![bounds_node]).u32();
            let bound = scratch;
            for word in 0..4u32 {
                let value = e.mem.u32(source + word * 4);
                e.mem.set_u32(bound + word * 4, value);
            }
            let me = e.call(0x0068_15c0, &args![bound]).u32();
            if e.call(0x0043_9090, &args![me, ZERO_VECTOR]).bool() {
                let me = e.call(0x0068_15c0, &args![bound]).u32();
                let current = e.call(0x0045_bb80, &args![node_owner]).u32();
                let drop = e.mem.f32(me + 8) as f64 - e.mem.f32(current + 8) as f64;
                let radius = e.call(0x0084_d030, &args![bound]).f32();
                if (radius as f64) < drop {
                    fell_out = true;
                    let me = e.call(0x0068_15c0, &args![bound]).u32();
                    let saved = e
                        .vcall(refr, SLOT_GET_POSITION, &args![scratch + 0x10])
                        .u32();
                    let drop = e.mem.f32(me + 8) as f64 - e.mem.f32(saved + 8) as f64;
                    let radius = e.call(0x0084_d030, &args![bound]).f32();
                    if (radius as f64) < drop {
                        below_both = true;
                    }
                }
            }
        }
    } else if e.call(0x0054_6fb0, &args![cell]).u32() != 0 {
        // exterior: compare with the land's lowest height minus 128
        let land = e.call(0x0054_6fb0, &args![cell]).u32();
        let lowest = e.call(0x0053_f440, &args![land, scratch + 0x20]).u32();
        let limit = (e.mem.f32(lowest) as f64 - e.global::<f64>(GROUND_MARGIN)) as f32;
        let current = e.call(0x0045_bb80, &args![node_owner]).u32();
        if e.mem.f32(current + 8) < limit {
            fell_out = true;
            let saved = e
                .vcall(refr, SLOT_GET_POSITION, &args![scratch + 0x10])
                .u32();
            if e.mem.f32(saved + 8) < limit {
                below_both = true;
            }
        }
    }
    (fell_out, below_both)
}

/// `flags & 2` half of [`tes_object_refr_trans_change_callback`].
fn rotation_changed(e: &mut Engine, node_owner: u32, refr: u32, scratch: u32) {
    let angles = scratch + 0x80;
    e.call(0x0068_15c0, &args![angles]);
    let matrix = e.call(0x0046_1130, &args![node_owner]).u32();
    e.call(0x00a5_92c0, &args![matrix, angles, angles + 4, angles + 8]);
    let flag_object = global_word(e, GLOBAL_FLAG_OBJECT);
    let old_flag = e.call(SWAP_FLAG, &args![flag_object, 0u32]).u8();
    let [x, y, z] = [
        e.mem.u32(angles),
        e.mem.u32(angles + 4),
        e.mem.u32(angles + 8),
    ];
    e.call(SET_ROTATION, &args![refr, x, y, z]);
    e.call(SWAP_FLAG, &args![flag_object, old_flag]);
}

// Translated from 00577b50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `bhkRigidBody::GethkMaxLinearVelocity` (Xbox PDB): the maximum linear
/// velocity of the body's `004ae750` object, a `float` looked up in the
/// table at `010c7948` by the `hkUFloat8` byte stored at +0xac of its
/// rigid-body info; 0.0 when there is no such object.
///
/// When the object has a body-info object (`00ca86c0`), the byte is read
/// in place; otherwise the byte is built from `00577c10`'s velocity with
/// `00ca9360` in a temporary.
pub fn bhk_rigid_body_gethk_max_linear_velocity(e: &mut Engine, this: Ptr) -> f32 {
    let object = e.call(0x004a_e750, &args![this]).u32();
    if object == 0 {
        return 0.0;
    }
    let info = e.call(0x00ca_86c0, &args![object]).u32();
    if info != 0 {
        let rigid = e.call(0x0046_0140, &args![info]).u32();
        fn_00577bf0(e, Ptr::new(rigid + 0xac))
    } else {
        let velocity = fn_00577c10(e, Ptr::new(object));
        let temporary = e.mem.alloc(4);
        let byte_cell = fn_00577bd0(e, Ptr::new(temporary), velocity);
        let result = fn_00577bf0(e, Ptr::new(byte_cell));
        e.mem.free(temporary);
        result
    }
}

// Translated from 00577bd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `value` in the `hkUFloat8` at `this` (`00ca9360`, which takes the
/// address of the float) and returns `this`.
pub fn fn_00577bd0(e: &mut Engine, this: Ptr, value: f32) -> u32 {
    let argument = e.mem.alloc(4);
    e.mem.set_f32(argument, value);
    e.call(0x00ca_9360, &args![this, argument]);
    e.mem.free(argument);
    this.addr()
}

// Translated from 00577bf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `float` of the `hkUFloat8` at `this`: the table entry for its byte.
pub fn fn_00577bf0(e: &mut Engine, this: Ptr) -> f32 {
    let index = e.mem.u8(this.addr()) as u32;
    e.global(UFLOAT8_TABLE + index * 4)
}

// Translated from 00577c10 (decompiled, FalloutNV.exe 1.4.0.525)
/// The velocity table entry of the `hkUFloat8` at +0xac of the info object
/// `00460140(009d9f40(this))` gives.
pub fn fn_00577c10(e: &mut Engine, this: Ptr) -> f32 {
    let body = e.call(0x009d_9f40, &args![this]).u32();
    let info = e.call(0x0046_0140, &args![body]).u32();
    fn_00577bf0(e, Ptr::new(info + 0xac))
}

// Translated from 00577c40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the `hkUFloat8` of `004ae750(this)` from `value` (`00577c70`), if it
/// has such an object.
pub fn fn_00577c40(e: &mut Engine, this: Ptr, value: f32) {
    let object = e.call(0x004a_e750, &args![this]).u32();
    if object != 0 {
        fn_00577c70(e, Ptr::new(object), value);
    }
}

// Translated from 00577c70 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `value` into the `hkUFloat8` at +0xac of the info object
/// `00460140(009d9f40(this))` gives (`00ca9360`, which takes the float's
/// address).
pub fn fn_00577c70(e: &mut Engine, this: Ptr, value: f32) {
    let body = e.call(0x009d_9f40, &args![this]).u32();
    let info = e.call(0x0046_0140, &args![body]).u32();
    let argument = e.mem.alloc(4);
    e.mem.set_f32(argument, value);
    e.call(0x00ca_9360, &args![info + 0xac, argument]);
    e.mem.free(argument);
}

// Translated from 00577ca0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::DebugDisplayCallback` (Xbox PDB): for a non-null node,
/// allocates a 0x80-byte debug-display object (`00aa13e0`, built by
/// `00b6fc90`), sizes it for 0x30 entries (`00441130`), attaches it to the
/// node (`00439410`) and finishes with `00b57e30(node, 0, 0)`. The
/// exception-unwinding frame of the allocation is not translated.
pub fn tes_object_refr_debug_display_callback(e: &mut Engine, node: Ptr) {
    if node.is_null() {
        return;
    }
    let memory = e.call(0x00aa_13e0, &args![0x80u32]).u32();
    let object = if memory != 0 {
        e.call(0x00b6_fc90, &args![memory]).u32()
    } else {
        0
    };
    e.call(0x0044_1130, &args![object, 0x30u32, 1u32]);
    e.call(0x0043_9410, &args![node, object]);
    e.call(0x00b5_7e30, &args![node, 0u32, 0u32]);
}

// Translated from 00577d50 (decompiled, FalloutNV.exe 1.4.0.525)
/// For a reference with a container (`0055d310`): takes the object
/// `004bf220(this)` gives, passes it and the result of `004839c0(0xf)` to
/// `004c8f30` when that result is non-null, and, when `0042cde0` of the
/// object is true, calls `0041aeb0` on the reference's extra data list.
/// Returns what `004c8f30` returned (0 without a container or block).
pub fn fn_00577d50(e: &mut Engine, this: Ptr) -> u32 {
    let mut result = 0;
    if e.call(0x0055_d310, &args![this]).u32() != 0 {
        let owner = e.call(0x004b_f220, &args![this]).u32();
        let block = e.call(0x0048_39c0, &args![0x0fu32]).u32();
        if block != 0 {
            result = e.call(0x004c_8f30, &args![owner, block]).u32();
        }
        if e.call(0x0042_cde0, &args![owner]).bool() {
            let extra = e.call(GET_EXTRA_LIST, &args![this]).u32();
            e.call(0x0041_aeb0, &args![extra]);
        }
    }
    result
}

// Translated from 00577de0 (decompiled, FalloutNV.exe 1.4.0.525)
/// For a reference with a container (`0055d310`): `004d0360` of the
/// container's owner object (`004bf220`); false without a container.
pub fn fn_00577de0(e: &mut Engine, this: Ptr) -> bool {
    if e.call(0x0055_d310, &args![this]).u32() == 0 {
        return false;
    }
    let owner = e.call(0x004b_f220, &args![this]).u32();
    e.call(0x004d_0360, &args![owner]).u8() != 0
}

// Translated from 00577e20 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::AddAddonNodes` (Xbox PDB): walks the scene graph under
/// `node` and attaches the addon node the data handler holds for it.
///
/// For a node: every child whose name starts with `"BASE"` (and whose
/// `0048af70` test passes) is detached (virtual at +0xf0 with its index).
/// Then the node's addon record (`004617e0`) is attached (virtual at +0xdc)
/// when it has a model and `00448bf0` is false, after being randomized with
/// `RandomizeAddons` and a seed taken from `00487f50() % 1000 / 100.0`.
/// The children are then processed recursively; when anything was added,
/// `004902f0(node, 1)` and `00576660(node, 0)` are called. Returns whether
/// anything was added.
pub fn tes_object_refr_add_addon_nodes(e: &mut Engine, node: u32) -> bool {
    if node == 0 {
        return false;
    }
    let mut added = false;
    if is_node(e, node) {
        let mut index = 0;
        while index < child_count(e, node) {
            let child = child_at(e, node, index);
            let child_as_node = child_node(e, child);
            if child_as_node != 0 {
                let name_holder = e.call(0x0041_3f40, &args![child_as_node]).u32();
                if e.call(0x0048_af70, &args![name_holder]).bool() {
                    let name_holder = e.call(0x0041_3f40, &args![child_as_node]).u32();
                    let name = e.call(0x0043_b1b0, &args![name_holder]).u32();
                    if e.call(STRNCMP, &args![name, BASE_PREFIX, 4u32]).i32() == 0 {
                        e.vcall(child_as_node, 0xf0, &args![index]);
                    }
                }
            }
            index += 1;
        }
        let addon = addon_for_node(e, node);
        if addon != 0
            && e.call(0x0048_cee0, &args![addon + 0x30]).u32() != 0
            && !e.call(0x0044_8bf0, &args![addon]).bool()
        {
            let created = e.vcall(addon, 0x178, &args![0u32]).u32();
            if created != 0 {
                let created_node = e.vcall(created, 0x10, &args![]).u32();
                if created_node != 0 {
                    e.call(0x0056_c7d0, &args![created_node, 0u32]);
                }
                let tick = e.call(0x0048_7f50, &args![]).u32();
                let seed = ((tick % 1000) as f64 / e.global::<f64>(RANDOM_DIVISOR)) as f32;
                fn_005784f0(e, created, seed);
                e.vcall(node, 0xdc, &args![created, 1u32]);
                added = true;
            }
        }
    }
    let mut index = 0;
    while index < child_count(e, node) {
        let child = child_at(e, node, index);
        let child_as_node = child_node(e, child);
        if child_as_node != 0 {
            added |= tes_object_refr_add_addon_nodes(e, child_as_node);
        }
        index += 1;
    }
    if added {
        e.call(0x0049_02f0, &args![node, 1u32]);
        e.call(0x0057_6660, &args![node, 0u32]);
    }
    added
}

// Translated from 00578060 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::AddMasterParticleAddonNodes` (Xbox PDB): for a node whose
/// addon record (`004617e0`) satisfies `00448bf0`, calls `00450f90(node, 0)`
/// and `00c505c0` with the record's `+0x5c` value (`0059e300`) and the node,
/// on the object `0045a190` returns. Then, unless `00456610(node)`, it does
/// the same for every child. Returns whether anything was done.
pub fn tes_object_refr_add_master_particle_addon_nodes(e: &mut Engine, node: u32) -> bool {
    if node == 0 {
        return false;
    }
    let mut done = false;
    if is_node(e, node) {
        let addon = addon_for_node(e, node);
        if addon != 0 && e.call(0x0044_8bf0, &args![addon]).bool() {
            e.call(0x0045_0f90, &args![node, 0u32]);
            let value = e.call(0x0059_e300, &args![addon]).u32();
            let target = e.call(0x0045_a190, &args![]).u32();
            e.call(0x00c5_05c0, &args![target, value, node]);
            done = true;
        }
    }
    if e.call(0x0045_6610, &args![node]).bool() {
        return done;
    }
    let mut index = 0;
    while index < child_count(e, node) {
        let child = child_at(e, node, index);
        let child_as_node = child_node(e, child);
        if child_as_node != 0 {
            done |= tes_object_refr_add_master_particle_addon_nodes(e, child_as_node);
        }
        index += 1;
    }
    done
}

// Translated from 00578170 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::RemoveMasterParticleAddonNodes` (Xbox PDB): for a node
/// whose addon record satisfies `00448bf0`, calls `00450f90(node, 1)`, and
/// recurses over the children. Returns whether anything was done.
pub fn tes_object_refr_remove_master_particle_addon_nodes(e: &mut Engine, node: u32) -> bool {
    if node == 0 {
        return false;
    }
    let mut done = false;
    if is_node(e, node) {
        let addon = addon_for_node(e, node);
        if addon != 0 && e.call(0x0044_8bf0, &args![addon]).bool() {
            e.call(0x0045_0f90, &args![node, 1u32]);
            done = true;
        }
    }
    let mut index = 0;
    while index < child_count(e, node) {
        let child = child_at(e, node, index);
        let child_as_node = child_node(e, child);
        if child_as_node != 0 {
            done |= tes_object_refr_remove_master_particle_addon_nodes(e, child_as_node);
        }
        index += 1;
    }
    done
}

// Translated from 00578250 (decompiled, FalloutNV.exe 1.4.0.525)
/// Walks `node` and its children: for each node `00570ea0` accepts, calls
/// `00ad8570(manager, node, 0.0, 0)` with the object `00453a70` returns.
/// Returns whether the root node itself was accepted.
pub fn fn_00578250(e: &mut Engine, node: u32) -> bool {
    if node == 0 {
        return false;
    }
    let mut accepted = false;
    if e.call(0x0057_0ea0, &args![node]).bool() {
        let manager = e.call(0x0045_3a70, &args![]).u32();
        e.call(0x00ad_8570, &args![manager, node, 0.0f32, 0u32]);
        accepted = true;
    }
    let mut index = 0;
    while index < child_count(e, node) {
        let child = child_at(e, node, index);
        let child_as_node = child_node(e, child);
        if child_as_node != 0 {
            fn_00578250(e, child_as_node);
        }
        index += 1;
    }
    accepted
}

// Translated from 00578300 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::RemoveAddonNodes` (Xbox PDB): for a node, calls
/// `00c69ee0(node, 1, 1)`; if its addon record has a model (`0048cee0` of
/// `record + 0x30`) and `00453470(node)` is non-zero, tells the record
/// (virtual at +0x150) and calls `00572160(node)`. Recurses over the
/// children. Returns whether the record was told.
pub fn tes_object_refr_remove_addon_nodes(e: &mut Engine, node: u32) -> bool {
    if node == 0 {
        return false;
    }
    let mut removed = false;
    if is_node(e, node) {
        e.call(0x00c6_9ee0, &args![node, 1u32, 1u32]);
        let addon = addon_for_node(e, node);
        if addon != 0
            && e.call(0x0048_cee0, &args![addon + 0x30]).u32() != 0
            && e.call(0x0045_3470, &args![node]).u32() != 0
        {
            e.vcall(addon, 0x150, &args![0u32]);
            e.call(0x0057_2160, &args![node]);
            removed = true;
        }
    }
    let mut index = 0;
    while index < child_count(e, node) {
        let child = child_at(e, node, index);
        let child_as_node = child_node(e, child);
        if child_as_node != 0 {
            tes_object_refr_remove_addon_nodes(e, child_as_node);
        }
        index += 1;
    }
    removed
}

// Translated from 00578400 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::HasAddonNodes` (Xbox PDB): whether the node, or any node
/// below it, passes `00453470`.
pub fn tes_object_refr_has_addon_nodes(e: &mut Engine, node: u32) -> bool {
    if node == 0 {
        return false;
    }
    if is_node(e, node) && e.call(0x0045_3470, &args![node]).u32() != 0 {
        return true;
    }
    let mut index = 0;
    while index < child_count(e, node) {
        let child = child_at(e, node, index);
        let child_as_node = child_node(e, child);
        if child_as_node != 0 && tes_object_refr_has_addon_nodes(e, child_as_node) {
            return true;
        }
        index += 1;
    }
    false
}

// Translated from 005784b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::HasAddonFlags` (Xbox PDB): whether the object has an
/// extra-data entry (`00a5bdd0`) of the kind `00448a80` returns and
/// `00448a40` accepts it.
pub fn tes_object_refr_has_addon_flags(e: &mut Engine, object: u32) -> bool {
    let kind = e.call(0x0044_8a80, &args![]).u32();
    let entry = e.call(0x00a5_bdd0, &args![object, kind]).u32();
    entry != 0 && e.call(0x0044_8a40, &args![entry]).bool()
}

// Translated from 005784f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::RandomizeAddons` (Xbox PDB): applies `value` to every
/// entry of the chain of `node`'s first list (`0043b230`, stepping with
/// `004a8a90`, applying `00621b20`), then to the chain of the list its
/// `00a59d30(node, 5)` entry has, then recurses over the children.
pub fn fn_005784f0(e: &mut Engine, node: u32, value: f32) {
    if node == 0 {
        return;
    }
    let mut entry = e.call(0x0043_b230, &args![node]).u32();
    while entry != 0 {
        e.call(0x0062_1b20, &args![entry, value]);
        entry = e.call(0x004a_8a90, &args![entry]).u32();
    }
    let second = e.call(0x00a5_9d30, &args![node, 5u32]).u32();
    entry = if second != 0 {
        e.call(0x0043_b230, &args![second]).u32()
    } else {
        0
    };
    while entry != 0 {
        e.call(0x0062_1b20, &args![entry, value]);
        entry = e.call(0x004a_8a90, &args![entry]).u32();
    }
    let children = e.vcall(node, SLOT_AS_NODE, &args![]).u32();
    if children != 0 {
        let mut index = 0;
        while index < child_count(e, children) {
            let child = child_at(e, children, index);
            fn_005784f0(e, child, value);
            index += 1;
        }
    }
}

// Translated from 005785e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::IsAnOwner` (Xbox PDB): whether `actor` counts as an owner
/// of this reference.
///
/// The reference's owner (`00567790`) is a form; without one everybody
/// owns it. An `actor` that is the owner itself (compared with its
/// `+0x1a4` form, else its base form) owns it. An owner of form type 8
/// (an NPC) that `00578770` marks (flag `0x200` at +0x34) makes the
/// `check_base` parameter true; with an NPC owner and `check_base`, the
/// actor owns the reference when `008b8e90(actor, owner)` and the actor's
/// base is of form type 0x2a/0x2b. A non-NPC owner (or one without the
/// flag) never matches here (the code only compares its global's value
/// with `01012060`, which cannot change the result).
pub fn tes_object_refr_is_an_owner(
    e: &mut Engine,
    this: Ptr,
    actor: u32,
    mut check_base: bool,
) -> bool {
    let mut result = false;
    let mut owner = 0;
    let mut owner_global = 0;
    if e.call(0x0056_7790, &args![this]).u32() != 0 {
        owner = e.call(0x0056_7790, &args![this]).u32();
        // the rank is read (and not used further)
        e.call(0x0056_79f0, &args![this]);
        owner_global = e.call(0x0056_7960, &args![this]).u32();
    }
    if owner != 0 && actor != 0 {
        let mut actor_form = e.vcall(actor, 0x1a4, &args![]).u32();
        if actor_form == 0 {
            actor_form = e.call(GET_BASE_FORM, &args![actor]).u32();
        }
        let mut as_bound_object = 0;
        let form_type = e.call(FORM_TYPE, &args![actor_form]).i32();
        if (0x2a..=0x2b).contains(&form_type) {
            as_bound_object = e
                .call(
                    DYNAMIC_CAST,
                    &args![actor_form, 0u32, 0x0118_3108u32, 0x0118_46e8u32, 0u32],
                )
                .u32();
        }
        if owner == actor_form {
            result = true;
        } else {
            let mut npc_owner = 0;
            if owner != 0 && e.call(FORM_TYPE, &args![owner]).u32() == FORM_TYPE_NPC_LIKE {
                npc_owner = owner;
            }
            if npc_owner != 0 && fn_00578770(e, Ptr::new(npc_owner)) {
                check_base = true;
            }
            if npc_owner == 0 || !check_base {
                if owner_global != 0 {
                    // the global's value is compared with a constant and 0
                    // is stored into the result when they are equal (it is
                    // already false)
                    let value = e.call(0x0052_6ac0, &args![owner_global]).f64();
                    if value == e.global::<f64>(OWNERSHIP_VALUE_LIMIT) {
                        result = false;
                    }
                } else {
                    result = false;
                }
            } else if as_bound_object != 0 && e.call(0x008b_8e90, &args![actor, npc_owner]).bool() {
                result = true;
            }
        }
    } else if owner == 0 {
        result = true;
    }
    result
}

// Translated from 00578770 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether flag `0x200` of the dword at +0x34 is set.
pub fn fn_00578770(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u32(this.addr() + 0x34) & 0x200 != 0
}

// Translated from 00578790 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::IsOwnerEvil` (Xbox PDB): whether the owner of this
/// reference (`owner`, or `00567790` of the reference when null) is evil:
/// an NPC owner (form type 8) is evil when `0047d7c0` says so; an owner of
/// form type 0x2a is evil when it has no actor in the process lists
/// (`00970a20`) or that actor's value 0x17 (virtual at +0xc of the object at
/// +0x100 of its base) maps to 2 or 4 through `0047e040`. Other owners are
/// not evil.
pub fn tes_object_refr_is_owner_evil(e: &mut Engine, this: Ptr, owner: u32) -> bool {
    let mut evil = false;
    let owner = if owner == 0 {
        e.call(0x0056_7790, &args![this]).u32()
    } else {
        owner
    };
    if owner == 0 {
        return false;
    }
    let owner_type = e.call(FORM_TYPE, &args![owner]).u32();
    if owner_type == FORM_TYPE_NPC_LIKE {
        if e.call(0x0047_d7c0, &args![owner]).bool() {
            evil = true;
        }
    } else if e.call(FORM_TYPE, &args![owner]).u32() == FORM_TYPE_FACTION_LIKE {
        let process_lists = 0x011e_0e80u32;
        let actor_ref = e
            .call(0x0097_0a20, &args![process_lists, owner, 1u32])
            .u32();
        evil = true;
        if actor_ref != 0 {
            let base = e.call(0x0041_81e0, &args![actor_ref]).u32();
            let values = base + 0x100;
            let value = e.vcall(values, 0xc, &args![0x17u32]).f32();
            let class = e.call(0x0047_e040, &args![value]).i32();
            if class != 2 && class != 4 {
                evil = false;
            }
        }
    }
    evil
}

// Translated from 00578870 (decompiled, FalloutNV.exe 1.4.0.525)
/// Writes the name of the reference's location into the string `result`: the
/// parent cell's name (`00408da0(cell + 0x18)`) when the reference is in an
/// interior cell (`00425fd0`), else what the worldspace-side object
/// (`00575d70`) formats for the position (virtual at +0x138), else the empty
/// string. The string is set with `004037f0(result, text, 0)`.
pub fn fn_00578870(e: &mut Engine, this: Ptr, result: Ptr) {
    let mut cell = e.call(GET_PARENT_CELL, &args![this]).u32();
    if cell != 0 && !e.call(0x0042_5fd0, &args![cell]).bool() {
        cell = 0;
    }
    if cell != 0 {
        let name = e.call(0x0040_8da0, &args![cell + 0x18]).u32();
        e.call(0x0040_37f0, &args![result, name, 0u32]);
        return;
    }
    let place = e.call(0x0057_5d70, &args![this]).u32();
    if place != 0 {
        let [x, y, z] = {
            let position = e.vcall(this.addr(), SLOT_POSITION_PTR, &args![]).u32();
            [
                e.mem.u32(position),
                e.mem.u32(position + 4),
                e.mem.u32(position + 8),
            ]
        };
        e.vcall(place, 0x138, &args![result, x, y, z]);
    } else {
        e.call(0x0040_37f0, &args![result, EMPTY_STRING, 0u32]);
    }
}

// Translated from 00578920 (decompiled, FalloutNV.exe 1.4.0.525)
/// `00c6bd00(3D, flag)` on the reference's 3D (virtual at +0x1d0).
pub fn fn_00578920(e: &mut Engine, this: Ptr, flag: u8) {
    let model = e.vcall(this.addr(), SLOT_GET_3D, &args![]).u32();
    e.call(0x00c6_bd00, &args![model, flag]);
}

// Translated from 00578950 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::AddTalkingActivatorMobileObjectREFR` (Xbox PDB):
/// `004ff070` (`BGSTalkingActivator::AddMobileObjectExtra`) on the base form
/// with this reference; returns its result.
pub fn tes_object_refr_add_talking_activator_mobile_object_refr(e: &mut Engine, this: Ptr) -> u32 {
    let base = e.call(GET_BASE_FORM, &args![this]).u32();
    e.call(0x004f_f070, &args![base, this]).u32()
}

// Translated from 00578970 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::GetMobileObjectExtra` (Xbox PDB): the dword at +0xc of
/// the talking-actor extra (`0042e110`) of the reference's extra data list,
/// or 0.
pub fn tes_object_refr_get_mobile_object_extra(e: &mut Engine, this: Ptr) -> u32 {
    let mut result = 0;
    let extra = e.call(GET_EXTRA_LIST, &args![this]).u32();
    if e.call(0x0042_e110, &args![extra]).u32() != 0 {
        let extra = e.call(GET_EXTRA_LIST, &args![this]).u32();
        let talking = e.call(0x0042_e110, &args![extra]).u32();
        result = e.mem.u32(talking + 0xc);
    }
    result
}

/// This part's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00576870, fn_00576870(Ptr, u32) -> u32),
        entry!(0x005768b0, fn_005768b0(Ptr<TESObjectREFR>)),
        entry!(0x00576d30, fn_00576d30(Ptr) -> bool),
        entry!(0x00576d50, fn_00576d50(Ptr, Ptr)),
        entry!(0x00577070, fn_00577070(Ptr)),
        entry!(0x00577090, fn_00577090(Ptr)),
        entry!(0x005770e0, fn_005770e0(Ptr, u32, u32) -> u32),
        entry!(0x00577110, fn_00577110(Ptr)),
        entry!(0x00577190, fn_00577190(Ptr) -> u32),
        entry!(0x005771d0, fn_005771d0(u32) -> u32),
        entry!(0x005771f0, fn_005771f0(Ptr) -> u32),
        entry!(0x00577250, fn_00577250(Ptr, u8) -> f32),
        entry!(0x00577310, fn_00577310(Ptr) -> f32),
        entry!(0x00577330, tes_object_refr_restore_rag_doll_data(Ptr, u32)),
        entry!(0x00577390, fn_00577390(Ptr, u8)),
        entry!(0x00577450, tes_object_refr_trans_change_callback(Ptr, u32)),
        entry!(
            0x00577b50,
            bhk_rigid_body_gethk_max_linear_velocity(Ptr) -> f32
        ),
        entry!(0x00577bd0, fn_00577bd0(Ptr, f32) -> u32),
        entry!(0x00577bf0, fn_00577bf0(Ptr) -> f32),
        entry!(0x00577c10, fn_00577c10(Ptr) -> f32),
        entry!(0x00577c40, fn_00577c40(Ptr, f32)),
        entry!(0x00577c70, fn_00577c70(Ptr, f32)),
        entry!(0x00577ca0, tes_object_refr_debug_display_callback(Ptr)),
        entry!(0x00577d50, fn_00577d50(Ptr) -> u32),
        entry!(0x00577de0, fn_00577de0(Ptr) -> bool),
        entry!(0x00577e20, tes_object_refr_add_addon_nodes(u32) -> bool),
        entry!(
            0x00578060,
            tes_object_refr_add_master_particle_addon_nodes(u32) -> bool
        ),
        entry!(
            0x00578170,
            tes_object_refr_remove_master_particle_addon_nodes(u32) -> bool
        ),
        entry!(0x00578250, fn_00578250(u32) -> bool),
        entry!(0x00578300, tes_object_refr_remove_addon_nodes(u32) -> bool),
        entry!(0x00578400, tes_object_refr_has_addon_nodes(u32) -> bool),
        entry!(0x005784b0, tes_object_refr_has_addon_flags(u32) -> bool),
        entry!(0x005784f0, fn_005784f0(u32, f32)),
        entry!(
            0x005785e0,
            tes_object_refr_is_an_owner(Ptr, u32, bool) -> bool
        ),
        entry!(0x00578770, fn_00578770(Ptr) -> bool),
        entry!(0x00578790, tes_object_refr_is_owner_evil(Ptr, u32) -> bool),
        entry!(0x00578870, fn_00578870(Ptr, Ptr)),
        entry!(0x00578920, fn_00578920(Ptr, u8)),
        entry!(
            0x00578950,
            tes_object_refr_add_talking_activator_mobile_object_refr(Ptr) -> u32
        ),
        entry!(
            0x00578970,
            tes_object_refr_get_mobile_object_extra(Ptr) -> u32
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    type Log = Vec<(u32, Vec<u32>)>;

    /// Targets of the shared doubles: `AsNode` returning its `this`, and
    /// the form-type reader.
    const AS_NODE_SELF: u32 = 0x00ff_0001;
    const RETURNS_TRUE: u32 = 0x00ff_0002;
    const RETURNS_FALSE: u32 = 0x00ff_0003;

    /// An engine with the pages of the globals the translations read mapped
    /// and the scene-graph accessors registered with their real bodies.
    fn engine() -> Engine {
        let mut e = Engine::new();
        for page in [
            0x0101_2000,
            0x0101_7000,
            0x0101_d000,
            0x0102_0000,
            0x0102_9000,
            0x0102_e000,
            0x0103_0000,
            0x010c_7000,
            0x011c_3000,
            0x011c_a000,
            0x011d_d000,
            0x011d_e000,
            0x011f_4000,
        ] {
            e.map(page, 0x1000);
        }
        e.register(AS_NODE_SELF, |_, a| a[0].into_ret());
        e.register(RETURNS_TRUE, |_, _| true.into_ret());
        e.register(RETURNS_FALSE, |_, _| false.into_ret());
        // 0043b480 / 0043b4a0: a node's child count at +0x20 and children
        // from +0x24 (the test nodes below are built that way)
        e.register(CHILD_COUNT, |e, a| e.mem.u32(a[0] + 0x20).into_ret());
        e.register(CHILD_AT, |e, a| {
            e.mem.u32(a[0] + 0x24 + 4 * a[1]).into_ret()
        });
        // 0043b300: every object is of the node type, nothing else
        e.register(IS_KIND_OF, |_, a| (a[0] == NODE_TYPE).into_ret());
        // 005d43c0: the extra data list at this + 0x44
        e.register(GET_EXTRA_LIST, |_, a| (a[0] + 0x44).into_ret());
        // 007af430: pObjectReference at +0x20; 00401170: the form type byte
        e.register(GET_BASE_FORM, |e, a| e.mem.u32(a[0] + 0x20).into_ret());
        e.register(FORM_TYPE, |e, a| u32::from(e.mem.u8(a[0] + 4)).into_ret());
        // 008d6f30: pParentCell at +0x40
        e.register(GET_PARENT_CELL, |e, a| e.mem.u32(a[0] + 0x40).into_ret());
        e
    }

    fn returns(e: &mut Engine, addr: u32, value: u32) {
        e.register_double(addr, move |_, _| Ret {
            eax: value,
            ..Ret::default()
        });
    }

    fn returns_float(e: &mut Engine, addr: u32, value: f32) {
        e.register_double(addr, move |_, _| Ret {
            st0: f64::from(value),
            ..Ret::default()
        });
    }

    fn stub(e: &mut Engine, addrs: &[u32]) {
        for addr in addrs {
            e.register(*addr, |_, _| Ret::default());
        }
    }

    fn start_log(e: &mut Engine) {
        e.call_log = Some(vec![]);
    }

    fn take_log(e: &mut Engine) -> Log {
        e.call_log.take().expect("log started")
    }

    fn calls_to(log: &Log, addr: u32) -> Vec<Vec<u32>> {
        log.iter()
            .filter(|(a, _)| *a == addr)
            .map(|(_, args)| args.clone())
            .collect()
    }

    /// An object whose vtable has the given (byte offset, target) slots.
    fn object_with(e: &mut Engine, size: u32, slots: &[(u32, u32)]) -> u32 {
        let object = e.mem.alloc(size);
        let vtable = e.mem.alloc(0x400);
        for (offset, target) in slots {
            e.mem.set_u32(vtable + offset, *target);
        }
        e.mem.set_u32(object, vtable);
        object
    }

    /// A scene-graph node: `AsNode` returns itself, children listed from
    /// +0x24 with their count at +0x20.
    fn node(e: &mut Engine, children: &[u32]) -> u32 {
        let node = object_with(e, 0x80, &[(0x0c, AS_NODE_SELF)]);
        e.mem.set_u32(node + 0x20, children.len() as u32);
        for (i, child) in children.iter().enumerate() {
            e.mem.set_u32(node + 0x24 + 4 * i as u32, *child);
        }
        node
    }

    /// A form object with its type byte at +4 and flags at +8.
    fn form(e: &mut Engine, kind: u8, flags: u32) -> u32 {
        let form = e.mem.alloc(0x100);
        e.mem.set_u8(form + 4, kind);
        e.mem.set_u32(form + 8, flags);
        form
    }

    // ---- 00576870 -------------------------------------------------------

    #[test]
    fn fn_00576870_returns_the_found_element_minus_four() {
        let mut e = engine();
        returns(&mut e, 0x0057_c400, 0x2004);
        start_log(&mut e);
        assert_eq!(
            e.call(0x0057_6870, &args![0x1000u32, 0x77u32]).u32(),
            0x2000
        );
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, 0x0057_c400), vec![vec![0x1004, 0x77]]);
        returns(&mut e, 0x0057_c400, 0);
        assert_eq!(e.call(0x0057_6870, &args![0x1000u32, 0x77u32]).u32(), 0);
    }

    // ---- 005768b0 -------------------------------------------------------

    /// Doubles for everything `005768b0` calls, all returning 0/false.
    fn loaded_model_world(e: &mut Engine) {
        stub(
            e,
            &[
                0x0045_3860,
                0x0056_c880,
                0x0056_c8f0,
                0x0056_eab0,
                0x0056_f140,
                0x0056_d7e0,
                0x0086_a830,
                0x0056_4c60,
                0x00c6_a350,
                0x0062_b8d0,
                0x0045_bb20,
                0x004a_3c90,
            ],
        );
        stub(
            e,
            &[
                0x0045_0ff0,
                0x0045_23c0,
                0x0045_23e0,
                0x0045_2440,
                0x0041_fbe0,
                0x0056_4c10,
                0x0042_ce10,
                0x0045_8900,
                0x004b_5260,
                0x0056_0d80,
                0x00c8_12d0,
                0x0044_ddc0,
                0x0062_0b80,
                0x009d_9f40,
                0x0051_7690,
                0x0045_43c0,
            ],
        );
    }

    /// A reference at `this` with the given base form, a 3D and IsActor.
    fn loaded_reference(e: &mut Engine, base: u32, model: u32, actor: bool) -> u32 {
        let is_actor = if actor { RETURNS_TRUE } else { RETURNS_FALSE };
        let base = if base == 0 { form(e, 0x30, 0) } else { base };
        let get_model = 0x00ff_0010;
        e.register_double(get_model, move |_, _| Ret {
            eax: model,
            ..Ret::default()
        });
        let refr = object_with(e, 0x80, &[(0x100, is_actor), (0x1d0, get_model)]);
        e.mem.set_u32(refr + 0x20, base);
        refr
    }

    #[test]
    fn fn_005768b0_stops_for_a_cell_the_two_tests_reject() {
        let mut e = engine();
        loaded_model_world(&mut e);
        let cell = e.mem.alloc(0x40);
        let model = node(&mut e, &[]);
        let refr = loaded_reference(&mut e, 0, model, false);
        e.mem.set_u32(refr + 0x40, cell);
        start_log(&mut e);
        e.call(0x0057_68b0, &args![refr]);
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, 0x0045_0ff0), vec![vec![cell]]);
        assert_eq!(calls_to(&log, 0x0045_23c0), vec![vec![cell]]);
        assert!(calls_to(&log, 0x0056_c880).is_empty());
        // one of the tests accepting lets it go on
        returns(&mut e, 0x0045_23c0, 1);
        start_log(&mut e);
        e.call(0x0057_68b0, &args![refr]);
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, 0x0056_c880), vec![vec![refr, 1, 1]]);
    }

    #[test]
    fn fn_005768b0_without_a_model_only_finishes() {
        let mut e = engine();
        loaded_model_world(&mut e);
        let refr = loaded_reference(&mut e, 0, 0, false);
        start_log(&mut e);
        e.call(0x0057_68b0, &args![refr]);
        let log = take_log(&mut e);
        let addrs: Vec<u32> = log
            .iter()
            .map(|(a, _)| *a)
            .filter(|a| *a != 0x00ff_0010)
            .collect();
        assert_eq!(addrs, vec![0x0057_68b0, 0x0056_c880]);
    }

    #[test]
    fn fn_005768b0_initializes_placeable_water_and_hands_the_model_to_the_world() {
        let mut e = engine();
        loaded_model_world(&mut e);
        let base = form(&mut e, 0x23, 0);
        let model = node(&mut e, &[]);
        let refr = loaded_reference(&mut e, base, model, true);
        let cell = e.mem.alloc(0x40);
        e.mem.set_u32(refr + 0x40, cell);
        returns(&mut e, 0x0045_0ff0, 1);
        // the world of the cell
        e.register_double(0x00ff_0020, |_, _| Ret::default());
        e.register_double(0x00ff_0021, |_, _| Ret::default());
        let world = object_with(&mut e, 0x20, &[(0x94, 0x00ff_0020), (0xd4, 0x00ff_0021)]);
        returns(&mut e, 0x0045_43c0, world);
        start_log(&mut e);
        e.call(0x0057_68b0, &args![refr]);
        let log = take_log(&mut e);
        let tes = e.global::<u32>(GLOBAL_TES);
        assert_eq!(
            calls_to(&log, 0x0045_3860),
            vec![vec![tes, 1], vec![tes, 0]]
        );
        // the placeable-water initializer, then the world takes the model
        assert_eq!(calls_to(&log, 0x0056_c8f0), vec![vec![refr]]);
        assert_eq!(calls_to(&log, 0x00ff_0021), vec![vec![world, model, model]]);
        assert_eq!(calls_to(&log, 0x0086_a830), vec![vec![refr, model]]);
        // no motion change for this base form
        assert!(calls_to(&log, 0x00c6_a350).is_empty());
        // a water reference that 452440 accepts skips the initializer
        returns(&mut e, 0x0045_2440, 1);
        start_log(&mut e);
        e.call(0x0057_68b0, &args![refr]);
        let log = take_log(&mut e);
        assert!(calls_to(&log, 0x0056_c8f0).is_empty());
    }

    #[test]
    fn fn_005768b0_picks_the_initializer_by_the_base_form() {
        let mut e = engine();
        loaded_model_world(&mut e);
        let model = node(&mut e, &[]);
        // the base form recorded in the global at 011ca240
        let special = form(&mut e, 1, 0);
        e.set_global(GLOBAL_BASE_FORM_A, special);
        let refr = loaded_reference(&mut e, special, model, false);
        start_log(&mut e);
        e.call(0x0057_68b0, &args![refr]);
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, 0x0056_eab0), vec![vec![refr]]);
        // a form of type 0x0e
        let other = form(&mut e, 0x0e, 0);
        e.mem.set_u32(refr + 0x20, other);
        start_log(&mut e);
        e.call(0x0057_68b0, &args![refr]);
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, 0x0056_f140), vec![vec![refr]]);
        // a form of type 0x15 with a primitive in the extra data
        let trigger = form(&mut e, 0x15, 0);
        e.mem.set_u32(refr + 0x20, trigger);
        returns(&mut e, 0x0041_fbe0, 0x5000);
        start_log(&mut e);
        e.call(0x0057_68b0, &args![refr]);
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, 0x0056_d7e0), vec![vec![refr]]);
        // none of them when 004523e0 already says the work is done
        returns(&mut e, 0x0045_23e0, 1);
        start_log(&mut e);
        e.call(0x0057_68b0, &args![refr]);
        let log = take_log(&mut e);
        assert!(calls_to(&log, 0x0056_d7e0).is_empty());
    }

    #[test]
    fn fn_005768b0_sets_the_motion_of_special_bases() {
        let mut e = engine();
        loaded_model_world(&mut e);
        let model = node(&mut e, &[]);
        let special = form(&mut e, 0x30, 0);
        e.set_global(GLOBAL_BASE_FORM_B, special);
        let refr = loaded_reference(&mut e, special, model, true);
        let cell = e.mem.alloc(0x40);
        e.mem.set_u32(refr + 0x40, cell);
        returns(&mut e, 0x0045_0ff0, 1);
        e.register(0x00ff_0021, |_, _| Ret::default());
        let world = object_with(&mut e, 0x20, &[(0xd4, 0x00ff_0021), (0x94, 0x00ff_0021)]);
        returns(&mut e, 0x0045_43c0, world);
        start_log(&mut e);
        e.call(0x0057_68b0, &args![refr]);
        let log = take_log(&mut e);
        // 00564c10 is false and there is no body: the motion is set
        assert_eq!(calls_to(&log, 0x00c6_a350), vec![vec![model, 5, 1, 0, 1]]);
        // the world's body is checked against the body of the target
        // (there is none here), so the work goes on
        assert_eq!(calls_to(&log, 0x0086_a830).len(), 1);
        // 00564c10 true: no motion change
        returns(&mut e, 0x0056_4c10, 1);
        start_log(&mut e);
        e.call(0x0057_68b0, &args![refr]);
        let log = take_log(&mut e);
        assert!(calls_to(&log, 0x00c6_a350).is_empty());
        // ... unless the stillness test 00458900 says so
        let collision = e.mem.alloc(0x40);
        returns(&mut e, 0x004b_5260, collision);
        // a body of another world than the cell's
        returns(&mut e, 0x00ff_0040, 5);
        let body = object_with(&mut e, 0x40, &[(0x94, 0x00ff_0040)]);
        returns(&mut e, 0x006f_a820, body);
        returns(&mut e, 0x0042_ce10, 1);
        returns(&mut e, 0x0045_8900, 1);
        returns(&mut e, 0x0045_8b20, 0x0126_7e30);
        start_log(&mut e);
        e.call(0x0057_68b0, &args![refr]);
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, 0x00c6_a350).len(), 1);
        let threshold = e.global::<u32>(STILL_THRESHOLD);
        assert_eq!(
            calls_to(&log, 0x0045_8900)[0][1..],
            [0x0126_7e30, threshold]
        );
        // the same body as the cell's world: nothing is done
        returns(&mut e, 0x00ff_0040, 0);
        start_log(&mut e);
        e.call(0x0057_68b0, &args![refr]);
        let log = take_log(&mut e);
        assert!(calls_to(&log, 0x0086_a830).is_empty());
    }

    #[test]
    fn fn_005768b0_offers_a_model_without_target_to_the_world() {
        let mut e = engine();
        loaded_model_world(&mut e);
        let model = node(&mut e, &[]);
        let refr = loaded_reference(&mut e, 0, model, false);
        let cell = e.mem.alloc(0x40);
        e.mem.set_u32(refr + 0x40, cell);
        returns(&mut e, 0x0045_0ff0, 1);
        // IsActor is false: a found object (00c812d0) without a target
        // (0044ddc0 gives 0)
        returns(&mut e, 0x00c8_12d0, 0x9000);
        e.register(IS_KIND_OF, |_, a| (a[0] == 0x011d_5e70).into_ret());
        e.register_double(0x00ff_0030, |_, _| Ret::default());
        let world = object_with(&mut e, 0x20, &[(0xd0, 0x00ff_0030)]);
        returns(&mut e, 0x0045_43c0, world);
        start_log(&mut e);
        e.call(0x0057_68b0, &args![refr]);
        let log = take_log(&mut e);
        assert_eq!(
            calls_to(&log, 0x00ff_0030),
            vec![vec![world, model, 0, 0, 0, 0]]
        );
        assert!(calls_to(&log, 0x0086_a830).is_empty());
    }

    // ---- 00576d30 -------------------------------------------------------

    #[test]
    fn fn_00576d30_tests_flag_200000() {
        let mut e = engine();
        let a = form(&mut e, 0, 0x0020_0000);
        let b = form(&mut e, 0, 0x0010_0000);
        assert!(e.call(0x0057_6d30, &args![a]).bool());
        assert!(!e.call(0x0057_6d30, &args![b]).bool());
    }

    // ---- 00576d50 -------------------------------------------------------

    /// The matrix helpers with the bodies the exe gives them.
    fn matrix_helpers(e: &mut Engine) {
        // 004b4cf0: this + index * 16; 00560d30: this + index * 4
        e.register(0x004b_4cf0, |_, a| (a[0] + a[1] * 16).into_ret());
        e.register(0x0056_0d30, |_, a| (a[0] + a[1] * 4).into_ret());
        // 006815c0: returns this; 00538f40: zeroes 16 bytes; 004a3f10: copies
        // 16 bytes from the argument
        e.register(0x0068_15c0, |_, a| a[0].into_ret());
        e.register(0x0053_8f40, |e, a| {
            for i in 0..4 {
                e.mem.set_u32(a[0] + 4 * i, 0);
            }
            Ret::default()
        });
        e.register(0x004a_3f10, |e, a| {
            for i in 0..4 {
                let v = e.mem.u32(a[1] + 4 * i);
                e.mem.set_u32(a[0] + 4 * i, v);
            }
            Ret::default()
        });
    }

    fn filled(e: &mut Engine, words: u32, value: u32) -> u32 {
        let block = e.mem.alloc(words * 4);
        for i in 0..words {
            e.mem.set_u32(block + 4 * i, value);
        }
        block
    }

    #[test]
    fn fn_005770e0_addresses_the_element() {
        let mut e = engine();
        matrix_helpers(&mut e);
        let m = e.mem.alloc(0x40);
        // column 2 of row 1
        assert_eq!(e.call(0x0057_70e0, &args![m, 2u32, 1u32]).u32(), m + 16 + 8);
    }

    #[test]
    fn fn_00577110_zeroes_three_rows() {
        let mut e = engine();
        matrix_helpers(&mut e);
        let m = filled(&mut e, 16, 0xdead_beef);
        e.call(0x0057_7110, &args![m]);
        for i in 0..12 {
            assert_eq!(e.mem.u32(m + 4 * i), 0);
        }
        // the fourth row is not touched
        assert_eq!(e.mem.u32(m + 0x30), 0xdead_beef);
    }

    #[test]
    fn fn_00577090_makes_the_identity() {
        let mut e = engine();
        matrix_helpers(&mut e);
        let m = filled(&mut e, 16, 0xdead_beef);
        e.call(0x0057_7090, &args![m]);
        for row in 0..3u32 {
            for column in 0..4u32 {
                let expected = if row == column { 1.0f32 } else { 0.0 };
                assert_eq!(e.mem.f32(m + row * 16 + column * 4), expected);
            }
        }
    }

    #[test]
    fn fn_00577070_adds_a_zero_translation() {
        let mut e = engine();
        matrix_helpers(&mut e);
        let m = filled(&mut e, 18, 0xdead_beef);
        e.call(0x0057_7070, &args![m]);
        assert_eq!(e.mem.f32(m), 1.0);
        assert_eq!(e.mem.f32(m + 0x14), 1.0);
        assert_eq!(e.mem.f32(m + 0x28), 1.0);
        for i in 0..4 {
            assert_eq!(e.mem.u32(m + 0x30 + 4 * i), 0);
        }
        // beyond the 16-byte translation vector nothing changes
        assert_eq!(e.mem.u32(m + 0x40), 0xdead_beef);
    }

    // ---- 00577190, 005771d0, 005771f0 ----------------------------------

    #[test]
    fn fn_005771f0_steps_back_0x10_from_the_virtual_result() {
        let mut e = engine();
        let get_inner = 0x00ff_0050;
        returns(&mut e, get_inner, 0x4010);
        let object = object_with(&mut e, 0x20, &[(0x10, get_inner)]);
        returns(&mut e, 0x004a_e750, object);
        assert_eq!(e.call(0x0057_71f0, &args![0x77u32]).u32(), 0x4000);
        // null virtual result
        returns(&mut e, get_inner, 0);
        assert_eq!(e.call(0x0057_71f0, &args![0x77u32]).u32(), 0);
        // no object
        returns(&mut e, 0x004a_e750, 0);
        assert_eq!(e.call(0x0057_71f0, &args![0x77u32]).u32(), 0);
    }

    #[test]
    fn fn_005771d0_forwards_to_004a3a40() {
        let mut e = engine();
        e.register(0x004a_3a40, |_, a| (a[0] + 1).into_ret());
        assert_eq!(e.call(0x0057_71d0, &args![0x10u32]).u32(), 0x11);
    }

    #[test]
    fn fn_00577190_converts_the_inner_object() {
        let mut e = engine();
        let get_inner = 0x00ff_0050;
        returns(&mut e, get_inner, 0x4010);
        let object = object_with(&mut e, 0x20, &[(0x10, get_inner)]);
        returns(&mut e, 0x004a_e750, object);
        e.register(0x004a_3a40, |_, a| (a[0] + 1).into_ret());
        assert_eq!(e.call(0x0057_7190, &args![0x77u32]).u32(), 0x4001);
        returns(&mut e, 0x004a_e750, 0);
        assert_eq!(e.call(0x0057_7190, &args![0x77u32]).u32(), 0);
    }

    // ---- 00576d50 -------------------------------------------------------

    /// A scene-graph node for `00576d50`: virtual 0xb8 fills the vector, the
    /// children are its `AsNode` result (a node).
    fn walked_node(e: &mut Engine, children: Option<u32>) -> u32 {
        e.register(0x00ff_0060, |e, a| {
            e.mem.set_f32(a[1], 7.0);
            Ret::default()
        });
        e.register(0x00ff_0061, move |_, a| {
            // AsNode: the children container if given, else 0
            let _ = a;
            Ret::default()
        });
        let node = object_with(e, 0x80, &[(0xb8, 0x00ff_0060), (0x0c, 0x00ff_0061)]);
        if let Some(children) = children {
            let vtable = e.mem.u32(node);
            e.mem.set_u32(vtable + 0x0c, AS_NODE_SELF);
            // the node itself is its own child container
            e.mem.set_u32(node + 0x20, 1);
            e.mem.set_u32(node + 0x24, children);
        }
        node
    }

    fn shape_doubles(e: &mut Engine) {
        stub(
            e,
            &[
                0x0043_d410,
                0x0056_ded0,
                0x0056_df80,
                0x00ca_4120,
                0x00ca_36a0,
            ],
        );
        matrix_helpers(e);
        returns(e, 0x0043_b610, 0);
    }

    #[test]
    fn fn_00576d50_does_nothing_for_a_null_node() {
        let mut e = engine();
        shape_doubles(&mut e);
        start_log(&mut e);
        e.call(0x0057_6d50, &args![0x11u32, 0u32]);
        let log = take_log(&mut e);
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn fn_00576d50_converts_the_transform_and_walks_the_children() {
        let mut e = engine();
        shape_doubles(&mut e);
        let transform = filled(&mut e, 13, 0x3f80_0000);
        returns(&mut e, 0x0046_1130, transform);
        let child = walked_node(&mut e, None);
        let root = walked_node(&mut e, Some(child));
        start_log(&mut e);
        e.call(0x0057_6d50, &args![0x11u32, root]);
        let log = take_log(&mut e);
        // the node, then its child (the root is its own children container
        // here)
        let converts = calls_to(&log, 0x0056_df80);
        assert!(converts.len() >= 2);
        // the NiTransform is copied (52 bytes) before the conversion
        let copy = converts[0][1];
        for i in 0..13 {
            assert_eq!(e.mem.u32(copy + 4 * i), 0x3f80_0000);
        }
        // the direction vector starts as (0, 1, 0)
        let direction = calls_to(&log, 0x0043_d410)[0].clone();
        assert_eq!(direction[2..], [1, 0]);
        assert!(calls_to(&log, 0x00ca_4120).is_empty());
    }

    #[test]
    fn fn_00576d50_passes_the_transform_to_the_shape_setters() {
        let mut e = engine();
        shape_doubles(&mut e);
        let transform = filled(&mut e, 13, 1);
        returns(&mut e, 0x0046_1130, transform);
        let node = walked_node(&mut e, None);
        // the vtable's AsNode answers 0: no children
        returns(&mut e, 0x0043_b610, 0x3000);
        returns(&mut e, 0x006f_a820, 0x3100);
        returns(&mut e, 0x004a_e6a0, 0x3200);
        returns(&mut e, 0x0062_0b80, 0x5555);
        // first kind
        e.register(IS_KIND_OF, |_, a| (a[0] == 0x0126_82e8).into_ret());
        start_log(&mut e);
        e.call(0x0057_6d50, &args![0u32, node]);
        let log = take_log(&mut e);
        let hk = calls_to(&log, 0x0056_df80)[0][0];
        assert_eq!(calls_to(&log, 0x00ca_4120), vec![vec![0x5555, hk]]);
        assert!(calls_to(&log, 0x00ca_36a0).is_empty());
        // second kind
        e.register(IS_KIND_OF, |_, a| (a[0] == 0x0126_82dc).into_ret());
        start_log(&mut e);
        e.call(0x0057_6d50, &args![0u32, node]);
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, 0x00ca_36a0).len(), 1);
        assert!(calls_to(&log, 0x00ca_4120).is_empty());
        // third kind: the wrapped shape decides
        e.register(IS_KIND_OF, |_, a| {
            (a[0] == 0x0126_82d0 || (a[0] == 0x0126_82dc && a[1] == 0x7001)).into_ret()
        });
        let get_inner = 0x00ff_0050;
        returns(&mut e, get_inner, 0x7011);
        let wrapper = object_with(&mut e, 0x20, &[(0x10, get_inner)]);
        e.register_double(0x004a_e750, move |_, _| Ret {
            eax: wrapper,
            ..Ret::default()
        });
        e.register(0x004a_3a40, |_, a| a[0].into_ret());
        start_log(&mut e);
        e.call(0x0057_6d50, &args![0u32, node]);
        let log = take_log(&mut e);
        // 0x7011 - 0x10 = 0x7001 is the inner shape, a second-kind shape
        assert_eq!(calls_to(&log, 0x00ca_36a0).len(), 1);
    }

    // ---- 00577250, 00577310 --------------------------------------------

    /// A reference-like object for `00577250`: IsActor as given.
    fn container_holder(e: &mut Engine, actor: bool) -> u32 {
        let is_actor = if actor { RETURNS_TRUE } else { RETURNS_FALSE };
        object_with(e, 0xe10, &[(0x100, is_actor)])
    }

    #[test]
    fn fn_00577310_reads_the_float_at_0xe04() {
        let mut e = engine();
        let object = e.mem.alloc(0xe10);
        e.mem.set_f32(object + 0xe04, 2.5);
        assert_eq!(e.call(0x0057_7310, &args![object]).f32(), 2.5);
    }

    #[test]
    fn fn_00577250_takes_the_value_from_the_list_or_the_container() {
        let mut e = engine();
        let this = container_holder(&mut e, false);
        // no container: 0.0
        returns(&mut e, 0x0055_d310, 0);
        assert_eq!(e.call(0x0057_7250, &args![this, 1u32]).f32(), 0.0);
        // a container and a list in the extra data
        returns(&mut e, 0x0055_d310, 0x6000);
        returns(&mut e, 0x0041_8520, 0x6100);
        returns_float(&mut e, 0x004d_0900, 3.5);
        start_log(&mut e);
        assert_eq!(e.call(0x0057_7250, &args![this, 1u32]).f32(), 3.5);
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, 0x004d_0900), vec![vec![0x6100, 1]]);
        // no list: the container answers
        returns(&mut e, 0x0041_8520, 0);
        returns_float(&mut e, 0x0048_1e10, 2.0);
        start_log(&mut e);
        assert_eq!(e.call(0x0057_7250, &args![this, 0u32]).f32(), 2.0);
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, 0x0048_1e10), vec![vec![0x6000, 0]]);
    }

    #[test]
    fn fn_00577250_limits_actors() {
        let mut e = engine();
        let actor = container_holder(&mut e, true);
        let player = container_holder(&mut e, true);
        e.set_global(GLOBAL_PLAYER, player);
        returns(&mut e, 0x0055_d310, 0x6000);
        returns(&mut e, 0x0041_8520, 0x6100);
        returns_float(&mut e, 0x004d_0900, 5.0);
        // a cap below the value lowers it
        returns_float(&mut e, 0x008a_0c20, 3.0);
        assert_eq!(e.call(0x0057_7250, &args![actor, 0u32]).f32(), 3.0);
        // a cap above the value leaves it
        returns_float(&mut e, 0x008a_0c20, 9.0);
        assert_eq!(e.call(0x0057_7250, &args![actor, 0u32]).f32(), 5.0);
        // the player: the +0xe04 float is added
        e.mem.set_f32(player + 0xe04, 0.5);
        assert_eq!(e.call(0x0057_7250, &args![player, 0u32]).f32(), 5.5);
    }

    // ---- 00577330 -------------------------------------------------------

    #[test]
    fn restore_rag_doll_data_applies_the_data_to_a_reference_with_3d() {
        let mut e = engine();
        returns(&mut e, 0x0043_fcd0, 0x9000);
        stub(&mut e, &[0x004d_9910, 0x0041_d5b0]);
        returns(&mut e, 0x0041_d6d0, 0x7700);
        let this = e.mem.alloc(0x80);
        start_log(&mut e);
        e.call(0x0057_7330, &args![this, 0x1234u32]);
        // data of the extra list when none is given
        e.call(0x0057_7330, &args![this, 0u32]);
        let log = take_log(&mut e);
        assert_eq!(
            calls_to(&log, 0x004d_9910),
            vec![vec![0x1234, this], vec![0x7700, this]]
        );
        assert!(calls_to(&log, 0x0041_d5b0).is_empty());
        // nothing to apply
        returns(&mut e, 0x0041_d6d0, 0);
        start_log(&mut e);
        e.call(0x0057_7330, &args![this, 0u32]);
        let log = take_log(&mut e);
        assert!(calls_to(&log, 0x004d_9910).is_empty());
    }

    #[test]
    fn restore_rag_doll_data_stores_the_data_without_3d() {
        let mut e = engine();
        returns(&mut e, 0x0043_fcd0, 0);
        stub(&mut e, &[0x004d_9910, 0x0041_d5b0]);
        let this = e.mem.alloc(0x80);
        start_log(&mut e);
        e.call(0x0057_7330, &args![this, 0x1234u32]);
        e.call(0x0057_7330, &args![this, 0u32]);
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, 0x0041_d5b0), vec![vec![this + 0x44, 0x1234]]);
        assert!(calls_to(&log, 0x004d_9910).is_empty());
    }

    // ---- 00577390 -------------------------------------------------------

    #[test]
    fn fn_00577390_updates_the_primitive_and_the_3d() {
        let mut e = engine();
        e.mem.set_f32(0x0101_df44, 0.25);
        returns(&mut e, 0x0041_fbe0, 0);
        let this = container_holder(&mut e, false);
        let base = form(&mut e, 1, 0);
        e.mem.set_u32(this + 0x20, base);
        start_log(&mut e);
        e.call(0x0057_7390, &args![this, 1u32]);
        let log = take_log(&mut e);
        assert!(calls_to(&log, 0x0041_3f90).is_empty());

        // a primitive that 0056c7f0 accepts: its class ID is read, the
        // last float set, and the primitive's virtual at +8 gets it
        let seen = std::rc::Rc::new(std::cell::RefCell::new(vec![]));
        let seen_in_double = seen.clone();
        e.register_double(0x00ff_0070, move |e, a| {
            seen_in_double.borrow_mut().push(e.mem.f32(a[1] + 12));
            Ret::default()
        });
        let primitive = object_with(&mut e, 0x40, &[(8, 0x00ff_0070)]);
        returns(&mut e, 0x0041_fbe0, primitive);
        returns(&mut e, 0x0056_c7f0, 1);
        stub(&mut e, &[0x0041_3f90, 0x0045_0f90]);
        let model = node(&mut e, &[]);
        let get_model = 0x00ff_0071;
        returns(&mut e, get_model, model);
        let this = object_with(&mut e, 0x100, &[(0x1d0, get_model)]);
        e.mem.set_u32(this + 0x20, base);
        start_log(&mut e);
        e.call(0x0057_7390, &args![this, 1u32]);
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, 0x0045_0f90), vec![vec![model, 0]]);
        e.call(0x0057_7390, &args![this, 0u32]);
        assert_eq!(*seen.borrow(), vec![0.25, 0.0]);
    }

    #[test]
    fn fn_00577390_leaves_the_special_base_alone() {
        let mut e = engine();
        let primitive = object_with(&mut e, 0x40, &[(8, RETURNS_TRUE)]);
        returns(&mut e, 0x0041_fbe0, primitive);
        returns(&mut e, 0x0056_c7f0, 1);
        stub(&mut e, &[0x0041_3f90, 0x0045_0f90]);
        let special = form(&mut e, 1, 0);
        e.set_global(GLOBAL_BASE_FORM_A, special);
        let get_model = 0x00ff_0071;
        returns(&mut e, get_model, 0x100);
        let this = object_with(&mut e, 0x100, &[(0x1d0, get_model)]);
        e.mem.set_u32(this + 0x20, special);
        start_log(&mut e);
        e.call(0x0057_7390, &args![this, 0u32]);
        let log = take_log(&mut e);
        assert!(calls_to(&log, 0x0045_0f90).is_empty());
        // 0056c7f0 refusing stops before the class ID is read
        returns(&mut e, 0x0056_c7f0, 0);
        start_log(&mut e);
        e.call(0x0057_7390, &args![this, 0u32]);
        let log = take_log(&mut e);
        assert!(calls_to(&log, 0x0041_3f90).is_empty());
    }

    // ---- hkUFloat8 helpers ----------------------------------------------

    #[test]
    fn fn_00577bf0_reads_the_table_entry_of_the_byte() {
        let mut e = engine();
        e.mem.set_f32(UFLOAT8_TABLE + 4 * 7, 12.5);
        let byte = e.mem.alloc(8);
        e.mem.set_u8(byte, 7);
        assert_eq!(e.call(0x0057_7bf0, &args![byte]).f32(), 12.5);
    }

    #[test]
    fn fn_00577bd0_hands_the_address_of_the_float_to_the_setter() {
        let mut e = engine();
        // 00ca9360 stores the byte 9 when the float is 2.0
        e.register(0x00ca_9360, |e, a| {
            let value = e.mem.f32(a[1]);
            e.mem.set_u8(a[0], if value == 2.0 { 9 } else { 0 });
            Ret::default()
        });
        let byte = e.mem.alloc(8);
        assert_eq!(e.call(0x0057_7bd0, &args![byte, 2.0f32]).u32(), byte);
        assert_eq!(e.mem.u8(byte), 9);
    }

    #[test]
    fn fn_00577c10_reads_the_table_entry_of_the_infos_byte() {
        let mut e = engine();
        e.mem.set_f32(UFLOAT8_TABLE + 4 * 3, 4.5);
        returns(&mut e, 0x009d_9f40, 0x40);
        // the info object 00460140 gives
        e.register(0x0046_0140, |_, a| {
            assert_eq!(a[0], 0x40);
            0x6000_0000u32.into_ret()
        });
        e.map(0x6000_0000, 0x1000);
        e.mem.set_u8(0x6000_0000 + 0xac, 3);
        assert_eq!(e.call(0x0057_7c10, &args![0x11u32]).f32(), 4.5);
    }

    #[test]
    fn fn_00577c70_and_00577c40_store_through_the_info_object() {
        let mut e = engine();
        returns(&mut e, 0x009d_9f40, 0x40);
        returns(&mut e, 0x0046_0140, 0x6000_0000);
        e.map(0x6000_0000, 0x1000);
        let stored = std::rc::Rc::new(std::cell::RefCell::new(vec![]));
        let stored_in_double = stored.clone();
        e.register_double(0x00ca_9360, move |e, a| {
            stored_in_double.borrow_mut().push((a[0], e.mem.f32(a[1])));
            Ret::default()
        });
        e.call(0x0057_7c70, &args![0x11u32, 6.5f32]);
        assert_eq!(*stored.borrow(), vec![(0x6000_00ac, 6.5)]);
        // 00577c40: through the object 004ae750 gives, if any
        returns(&mut e, 0x004a_e750, 0);
        e.call(0x0057_7c40, &args![0x11u32, 1.5f32]);
        assert_eq!(stored.borrow().len(), 1);
        returns(&mut e, 0x004a_e750, 0x22);
        e.call(0x0057_7c40, &args![0x11u32, 1.5f32]);
        assert_eq!(
            *stored.borrow(),
            vec![(0x6000_00ac, 6.5), (0x6000_00ac, 1.5)]
        );
    }

    #[test]
    fn gethk_max_linear_velocity_has_two_sources() {
        let mut e = engine();
        e.mem.set_f32(UFLOAT8_TABLE + 4 * 5, 8.0);
        e.mem.set_f32(UFLOAT8_TABLE + 4 * 6, 9.0);
        e.map(0x6000_0000, 0x1000);
        e.mem.set_u8(0x6000_0000 + 0xac, 5);
        // no object: 0.0
        returns(&mut e, 0x004a_e750, 0);
        assert_eq!(e.call(0x0057_7b50, &args![0x11u32]).f32(), 0.0);
        // an object with a body-info object: its byte is read in place
        returns(&mut e, 0x004a_e750, 0x33);
        returns(&mut e, 0x00ca_86c0, 0x44);
        returns(&mut e, 0x0046_0140, 0x6000_0000);
        assert_eq!(e.call(0x0057_7b50, &args![0x11u32]).f32(), 8.0);
        // without: a byte is built from 00577c10's value by 00ca9360
        returns(&mut e, 0x00ca_86c0, 0);
        returns(&mut e, 0x009d_9f40, 0x40);
        e.register(0x00ca_9360, |e, a| {
            // the float 8.0 becomes the byte 6
            let byte = if e.mem.f32(a[1]) == 8.0 { 6 } else { 0 };
            e.mem.set_u8(a[0], byte);
            Ret::default()
        });
        assert_eq!(e.call(0x0057_7b50, &args![0x11u32]).f32(), 9.0);
    }

    // ---- 00577ca0, 00577d50, 00577de0 -----------------------------------

    #[test]
    fn debug_display_callback_attaches_a_new_object() {
        let mut e = engine();
        returns(&mut e, 0x00aa_13e0, 0x8000);
        returns(&mut e, 0x00b6_fc90, 0x8100);
        stub(&mut e, &[0x0044_1130, 0x0043_9410, 0x00b5_7e30]);
        start_log(&mut e);
        e.call(0x0057_7ca0, &args![0u32]);
        assert_eq!(take_log(&mut e).len(), 1);
        start_log(&mut e);
        e.call(0x0057_7ca0, &args![0x55u32]);
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, 0x00aa_13e0), vec![vec![0x80]]);
        assert_eq!(calls_to(&log, 0x00b6_fc90), vec![vec![0x8000]]);
        assert_eq!(calls_to(&log, 0x0044_1130), vec![vec![0x8100, 0x30, 1]]);
        assert_eq!(calls_to(&log, 0x0043_9410), vec![vec![0x55, 0x8100]]);
        assert_eq!(calls_to(&log, 0x00b5_7e30), vec![vec![0x55, 0, 0]]);
    }

    #[test]
    fn fn_00577d50_builds_the_list_for_a_container() {
        let mut e = engine();
        let this = e.mem.alloc(0x80);
        returns(&mut e, 0x0055_d310, 0);
        assert_eq!(e.call(0x0057_7d50, &args![this]).u32(), 0);
        returns(&mut e, 0x0055_d310, 0x9000);
        returns(&mut e, 0x004b_f220, 0x9100);
        returns(&mut e, 0x0048_39c0, 0x9200);
        returns(&mut e, 0x004c_8f30, 0x9300);
        returns(&mut e, 0x0042_cde0, 1);
        stub(&mut e, &[0x0041_aeb0]);
        start_log(&mut e);
        assert_eq!(e.call(0x0057_7d50, &args![this]).u32(), 0x9300);
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, 0x0048_39c0), vec![vec![0x0f]]);
        assert_eq!(calls_to(&log, 0x004c_8f30), vec![vec![0x9100, 0x9200]]);
        assert_eq!(calls_to(&log, 0x0041_aeb0), vec![vec![this + 0x44]]);
        // no block: the result is 0, the extra call still depends on 0042cde0
        returns(&mut e, 0x0048_39c0, 0);
        returns(&mut e, 0x0042_cde0, 0);
        start_log(&mut e);
        assert_eq!(e.call(0x0057_7d50, &args![this]).u32(), 0);
        let log = take_log(&mut e);
        assert!(calls_to(&log, 0x004c_8f30).is_empty());
        assert!(calls_to(&log, 0x0041_aeb0).is_empty());
    }

    #[test]
    fn fn_00577de0_asks_the_owner_object() {
        let mut e = engine();
        let this = e.mem.alloc(0x80);
        returns(&mut e, 0x0055_d310, 0);
        assert!(!e.call(0x0057_7de0, &args![this]).bool());
        returns(&mut e, 0x0055_d310, 0x9000);
        returns(&mut e, 0x004b_f220, 0x9100);
        returns(&mut e, 0x004d_0360, 1);
        assert!(e.call(0x0057_7de0, &args![this]).bool());
        returns(&mut e, 0x004d_0360, 0);
        assert!(!e.call(0x0057_7de0, &args![this]).bool());
    }

    // ---- 00577450 -------------------------------------------------------

    struct Scene {
        node: u32,
        owner: u32,
        refr: u32,
        model: u32,
        cell: u32,
    }

    /// The `double` an x87 helper receives in two words.
    fn double_of(a: &[u32]) -> f64 {
        f64::from_bits(u64::from(a[0]) | u64::from(a[1]) << 32)
    }

    /// A node, its owner (position at +0x8c), the reference (position at
    /// +0x90; IsActor false, in a cell) and its 3D, with the callee doubles
    /// of `TransChangeCallback` that do nothing.
    fn scene(e: &mut Engine) -> Scene {
        e.register(CRT_FINITE, |_, a| double_of(a).is_finite().into_ret());
        e.register(CRT_ISNAN, |_, a| double_of(a).is_nan().into_ret());
        // 0044ddc0: +8; 00461130: +0x68; 0045bb80: +0x8c; 006815c0: this
        e.register(0x0044_ddc0, |e, a| e.mem.u32(a[0] + 8).into_ret());
        e.register(0x0046_1130, |_, a| (a[0] + 0x68).into_ret());
        e.register(0x0045_bb80, |_, a| (a[0] + 0x8c).into_ret());
        e.register(0x0068_15c0, |_, a| a[0].into_ret());
        stub(
            e,
            &[
                0x0062_c3c0,
                0x0044_0460,
                0x0043_fa80,
                0x00c6_bd00,
                0x0043_d410,
                0x00a5_9c60,
                0x0057_3ed0,
                0x005b_5e40,
                0x0057_5830,
                SWAP_FLAG,
                0x0048_4a70,
                0x0057_29e0,
                0x00b5_d9f0,
                0x0054_cfd0,
                0x0057_3800,
                0x0054_96b0,
                0x0054_abd0,
                0x008a_e640,
                0x00a5_92c0,
                0x0057_5700,
                0x00c6_9ee0,
                0x009e_e040,
                0x0046_17e0,
                0x006f_a820,
                0x008c_7aa0,
                0x0054_97a0,
            ],
        );
        returns(e, 0x0045_0b80, 0x00a0_0000);
        // exterior without a land by default, inside an interior of TES
        returns(e, 0x0042_5fd0, 0);
        returns(e, 0x0054_6fb0, 0);
        returns(e, 0x005f_36f0, 1);
        e.mem.set_f64(GROUND_MARGIN, 128.0);
        e.mem.set_f64(VELOCITY_SCALE, 0.25);
        e.mem.set_f64(VELOCITY_LIMIT, 5.0);

        // the reference: position at +0x90, virtual 0x170 copies it out,
        // 0x1f4 gives its address
        e.register(0x00ff_0080, |e, a| {
            for i in 0..3 {
                let v = e.mem.u32(a[0] + 0x90 + 4 * i);
                e.mem.set_u32(a[1] + 4 * i, v);
            }
            a[1].into_ret()
        });
        e.register(0x00ff_0081, |_, a| (a[0] + 0x90).into_ret());
        e.register(0x00ff_0082, |e, a| e.mem.u32(a[0] + 0x98).into_ret());
        let model = node(e, &[]);
        let get_model = 0x00ff_0083;
        returns(e, get_model, model);
        let refr = object_with(
            e,
            0x100,
            &[
                (0x100, RETURNS_FALSE),
                (0x1d0, get_model),
                (0x170, 0x00ff_0080),
                (0x1f4, 0x00ff_0081),
                (0x48, 0x00ff_0090),
                (0x130, 0x00ff_0091),
                (0x174, 0x00ff_0092),
            ],
        );
        stub(e, &[0x00ff_0090, 0x00ff_0091, 0x00ff_0092]);
        let base = form(e, 0x30, 0);
        e.mem.set_u32(refr + 0x20, base);
        let cell = e.mem.alloc(0x40);
        e.mem.set_u32(refr + 0x40, cell);
        let owner = e.mem.alloc(0x100);
        let node_object = object_with(e, 0x40, &[(0xb0, 0x00ff_0093)]);
        stub(e, &[0x00ff_0093]);
        e.mem.set_u32(node_object + 8, owner);
        returns(e, 0x0056_f930, refr);
        // positions: node at z = 50, reference at z = 60
        e.mem.set_f32(owner + 0x8c + 8, 50.0);
        e.mem.set_f32(refr + 0x90 + 8, 60.0);
        Scene {
            node: node_object,
            owner,
            refr,
            model,
            cell,
        }
    }

    #[test]
    fn trans_change_callback_ignores_a_node_without_reference() {
        let mut e = engine();
        let s = scene(&mut e);
        returns(&mut e, 0x0056_f930, 0);
        start_log(&mut e);
        e.call(0x0057_7450, &args![s.node, 3u32]);
        let log = take_log(&mut e);
        assert_eq!(log.len(), 3);
        assert_eq!(calls_to(&log, 0x0044_ddc0), vec![vec![s.node]]);
        assert_eq!(calls_to(&log, 0x0056_f930), vec![vec![s.owner]]);
    }

    #[test]
    fn trans_change_callback_with_no_flags_only_refreshes_the_lighting() {
        let mut e = engine();
        let s = scene(&mut e);
        start_log(&mut e);
        e.call(0x0057_7450, &args![s.node, 0u32]);
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, 0x0062_c3c0), vec![vec![s.refr, s.model]]);
        assert_eq!(calls_to(&log, 0x0045_0b80), vec![vec![0]]);
        assert_eq!(
            calls_to(&log, 0x00b5_d9f0),
            vec![vec![0x00a0_0000, s.model, 1]]
        );
        assert!(calls_to(&log, SET_POSITION).is_empty());
        assert!(calls_to(&log, SET_ROTATION).is_empty());
    }

    #[test]
    fn trans_change_callback_writes_the_position_back_around_a_flag_swap() {
        let mut e = engine();
        let s = scene(&mut e);
        // the previous flag value is handed back to the swap
        returns(&mut e, SWAP_FLAG, 1);
        // land at height 100: the limit is -28, the node (z = 50) is above
        returns(&mut e, 0x0054_6fb0, 0x5000);
        let height = e.mem.alloc(8);
        e.mem.set_f32(height, 100.0);
        returns(&mut e, 0x0053_f440, height);
        start_log(&mut e);
        e.call(0x0057_7450, &args![s.node, 1u32]);
        let log = take_log(&mut e);
        let flag_object = e.global::<u32>(GLOBAL_FLAG_OBJECT);
        assert_eq!(
            calls_to(&log, SWAP_FLAG),
            vec![vec![flag_object, 0], vec![flag_object, 1]]
        );
        assert_eq!(
            calls_to(&log, SET_POSITION),
            vec![vec![s.refr, s.owner + 0x8c]]
        );
        assert_eq!(calls_to(&log, 0x00ff_0090), vec![vec![s.refr, 4]]);
        // nothing fell out of the world
        assert!(calls_to(&log, 0x0044_0460).is_empty());
        assert!(calls_to(&log, 0x0054_cfd0).is_empty());
        // the interior test of TES says interior: no cell lookup
        assert!(calls_to(&log, 0x0046_1bc0).is_empty());
    }

    #[test]
    fn trans_change_callback_disables_the_collision_of_a_slow_fallen_node() {
        let mut e = engine();
        let s = scene(&mut e);
        // a position that is not finite
        e.mem.set_f32(s.owner + 0x8c, f32::NAN);
        returns(&mut e, 0x006f_a820, 0x7000);
        returns(&mut e, 0x004a_e750, 0);
        returns(&mut e, 0x0084_e3a0, 0x0012_34ab);
        returns(&mut e, 0x00ff_0091, 0x6666);
        start_log(&mut e);
        e.call(0x0057_7450, &args![s.node, 1u32]);
        let log = take_log(&mut e);
        // not finite: no placement, but the 3D is moved to the position
        assert!(calls_to(&log, 0x0054_cfd0).is_empty());
        assert_eq!(calls_to(&log, 0x0044_0460)[0][0], s.model);
        assert_eq!(
            calls_to(&log, 0x0043_fa80),
            vec![vec![s.model, 0x011a_9448]]
        );
        assert_eq!(calls_to(&log, 0x00c6_bd00), vec![vec![s.model, 1]]);
        assert_eq!(calls_to(&log, 0x00a5_9c60).len(), 1);
        // the body is too slow: the node's virtual +0xb0 disables it
        assert_eq!(calls_to(&log, 0x00ff_0093), vec![vec![s.node, 4, 0, 0]]);
        assert_eq!(calls_to(&log, 0x0057_3ed0), vec![vec![s.node, 0]]);
        assert_eq!(
            calls_to(&log, MESSAGE),
            vec![vec![DISABLE_COLLISION_MESSAGE, 0x6666, 0x0012_34ab]]
        );
    }

    #[test]
    fn trans_change_callback_limits_a_fast_fallen_body() {
        let mut e = engine();
        let s = scene(&mut e);
        e.mem.set_f32(s.owner + 0x8c, f32::INFINITY);
        returns(&mut e, 0x006f_a820, 0x7000);
        // the body's object has an info object whose byte 7 maps to 40.0
        returns(&mut e, 0x004a_e750, 0x7100);
        returns(&mut e, 0x00ca_86c0, 0x7200);
        returns(&mut e, 0x0046_0140, 0x6000_0000);
        returns(&mut e, 0x009d_9f40, 0x7300);
        e.map(0x6000_0000, 0x1000);
        e.mem.set_u8(0x6000_0000 + 0xac, 7);
        e.mem.set_f32(UFLOAT8_TABLE + 28, 40.0);
        let stored = std::rc::Rc::new(std::cell::RefCell::new(vec![]));
        let stored_in_double = stored.clone();
        e.register_double(0x00ca_9360, move |e, a| {
            stored_in_double.borrow_mut().push((a[0], e.mem.f32(a[1])));
            Ret::default()
        });
        start_log(&mut e);
        e.call(0x0057_7450, &args![s.node, 1u32]);
        let log = take_log(&mut e);
        // 40.0 * 0.25 = 10.0 is above 5.0: the limit is stored, the
        // collision is not disabled
        assert_eq!(*stored.borrow(), vec![(0x6000_00ac, 10.0)]);
        assert!(calls_to(&log, 0x0057_3ed0).is_empty());
        assert!(calls_to(&log, MESSAGE).is_empty());
    }

    #[test]
    fn trans_change_callback_replaces_a_reference_below_the_land() {
        let mut e = engine();
        let s = scene(&mut e);
        // land height 100: limit -28; node and reference both lower
        e.mem.set_f32(s.owner + 0x8c + 8, -50.0);
        e.mem.set_f32(s.refr + 0x90 + 8, -40.0);
        returns(&mut e, 0x0054_6fb0, 0x5000);
        let height = e.mem.alloc(8);
        e.mem.set_f32(height, 100.0);
        returns(&mut e, 0x0053_f440, height);
        e.mem.set_u32(ZERO_VECTOR, 0x1111);
        e.mem.set_u32(ZERO_VECTOR + 4, 0x2222);
        e.mem.set_u32(ZERO_VECTOR + 8, 0x3333);
        start_log(&mut e);
        e.call(0x0057_7450, &args![s.node, 1u32]);
        let log = take_log(&mut e);
        // the cell's placement info gets a copy of the vector, then the
        // reference is moved to what it wrote (here: unchanged)
        let place = calls_to(&log, 0x0054_cfd0);
        assert_eq!(place.len(), 1);
        assert_eq!(place[0][0], s.cell);
        assert_eq!(
            calls_to(&log, 0x00ff_0092),
            vec![vec![s.refr, 0x1111, 0x2222, 0x3333]]
        );
        // a reference that is only the node below the land is not replaced
        e.mem.set_f32(s.refr + 0x90 + 8, 60.0);
        start_log(&mut e);
        e.call(0x0057_7450, &args![s.node, 1u32]);
        let log = take_log(&mut e);
        assert!(calls_to(&log, 0x0054_cfd0).is_empty());
        assert_eq!(calls_to(&log, 0x0044_0460).len(), 1);
    }

    #[test]
    fn trans_change_callback_compares_with_the_bounds_of_an_interior_cell() {
        let mut e = engine();
        let s = scene(&mut e);
        returns(&mut e, 0x0042_5fd0, 1);
        returns(&mut e, 0x0045_6fc0, 0x5000);
        // the bounding sphere: centre z = 100
        let bounds = e.mem.alloc(16);
        e.mem.set_f32(bounds + 8, 100.0);
        returns(&mut e, 0x0043_d450, bounds);
        returns(&mut e, 0x0043_9090, 1);
        returns_float(&mut e, 0x0084_d030, 5.0);
        start_log(&mut e);
        e.call(0x0057_7450, &args![s.node, 1u32]);
        let log = take_log(&mut e);
        // node z = 50 is 50 below the centre (> 5), reference z = 60 is 40
        // below: both count as fallen
        assert_eq!(calls_to(&log, 0x0054_cfd0).len(), 1);
        // a large radius keeps the node inside
        returns_float(&mut e, 0x0084_d030, 80.0);
        start_log(&mut e);
        e.call(0x0057_7450, &args![s.node, 1u32]);
        let log = take_log(&mut e);
        assert!(calls_to(&log, 0x0054_cfd0).is_empty());
        // a radius between the two drops: only the node is below
        returns_float(&mut e, 0x0084_d030, 45.0);
        start_log(&mut e);
        e.call(0x0057_7450, &args![s.node, 1u32]);
        let log = take_log(&mut e);
        assert!(calls_to(&log, 0x0054_cfd0).is_empty());
        assert_eq!(calls_to(&log, 0x0044_0460).len(), 1);
        // the test of the bounds (0043_9090) refusing: nothing
        returns(&mut e, 0x0043_9090, 0);
        start_log(&mut e);
        e.call(0x0057_7450, &args![s.node, 1u32]);
        let log = take_log(&mut e);
        assert!(calls_to(&log, 0x0044_0460).is_empty());
    }

    #[test]
    fn trans_change_callback_submits_the_water_bound_reference_to_the_sinking() {
        let mut e = engine();
        let s = scene(&mut e);
        let water = form(&mut e, 0x1e, 0);
        e.mem.set_u32(s.refr + 0x20, water);
        returns(&mut e, 0x0044_8a20, 0);
        returns_float(&mut e, 0x0057_b0a0, 70.0);
        start_log(&mut e);
        e.call(0x0057_7450, &args![s.node, 1u32]);
        let log = take_log(&mut e);
        // the reference (z = 60) is below the water height 70
        assert_eq!(calls_to(&log, 0x0048_4a70), vec![vec![s.refr, 1]]);
        assert_eq!(calls_to(&log, 0x0057_29e0), vec![vec![s.refr, 0]]);
        // above the water, or flagged: nothing
        returns_float(&mut e, 0x0057_b0a0, 10.0);
        start_log(&mut e);
        e.call(0x0057_7450, &args![s.node, 1u32]);
        let log = take_log(&mut e);
        assert!(calls_to(&log, 0x0048_4a70).is_empty());
        returns_float(&mut e, 0x0057_b0a0, 70.0);
        returns(&mut e, 0x0044_8a20, 1);
        start_log(&mut e);
        e.call(0x0057_7450, &args![s.node, 1u32]);
        let log = take_log(&mut e);
        assert!(calls_to(&log, 0x0048_4a70).is_empty());
    }

    #[test]
    fn trans_change_callback_writes_the_euler_angles_back() {
        let mut e = engine();
        let s = scene(&mut e);
        e.register(0x00a5_92c0, |e, a| {
            e.mem.set_f32(a[1], 0.5);
            e.mem.set_f32(a[2], 1.5);
            e.mem.set_f32(a[3], 2.5);
            Ret::default()
        });
        start_log(&mut e);
        e.call(0x0057_7450, &args![s.node, 2u32]);
        let log = take_log(&mut e);
        assert_eq!(
            calls_to(&log, SET_ROTATION),
            vec![vec![
                s.refr,
                0.5f32.to_bits(),
                1.5f32.to_bits(),
                2.5f32.to_bits()
            ]]
        );
        // the matrix is the owner's transform
        assert_eq!(calls_to(&log, 0x00a5_92c0)[0][0], s.owner + 0x68);
        assert!(calls_to(&log, SET_POSITION).is_empty());
    }

    #[test]
    fn trans_change_callback_moves_the_reference_to_the_cell_of_its_coordinates() {
        let mut e = engine();
        let s = scene(&mut e);
        // exterior: TES has no interior cell
        returns(&mut e, 0x005f_36f0, 0);
        returns(&mut e, 0x004f_d3e0, 0x77);
        e.mem.set_f32(s.refr + 0x90, 11.0);
        e.mem.set_f32(s.refr + 0x94, 22.0);
        // the same cell: nothing more happens
        let cell = s.cell;
        returns(&mut e, 0x0046_1bc0, cell);
        start_log(&mut e);
        e.call(0x0057_7450, &args![s.node, 1u32]);
        let log = take_log(&mut e);
        let handler = e.global::<u32>(GLOBAL_DATA_HANDLER);
        assert_eq!(
            calls_to(&log, 0x0046_1bc0),
            vec![vec![handler, 11.0f32.to_bits(), 22.0f32.to_bits(), 0x77, 0]]
        );
        assert!(calls_to(&log, 0x0057_3800).is_empty());
        // no cell: the saved position is restored and the node frozen
        returns(&mut e, 0x0046_1bc0, 0);
        start_log(&mut e);
        e.call(0x0057_7450, &args![s.node, 1u32]);
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, SET_POSITION).len(), 2);
        assert_eq!(calls_to(&log, 0x00ff_0093), vec![vec![s.node, 4, 0, 0]]);
        assert!(calls_to(&log, 0x0057_3800).is_empty());
        // another cell: the reference moves and its 3D is attached
        returns(&mut e, 0x0046_1bc0, 0x0abc_0000);
        returns(&mut e, 0x0054_97a0, 0x0def_0000);
        start_log(&mut e);
        e.call(0x0057_7450, &args![s.node, 1u32]);
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, 0x0057_3800), vec![vec![s.refr, 0, 0x77]]);
        assert_eq!(
            calls_to(&log, 0x0054_96b0),
            vec![vec![0x0abc_0000, s.refr, 0x0def_0000]]
        );
        // the other attach path
        returns(&mut e, 0x008c_7aa0, 1);
        start_log(&mut e);
        e.call(0x0057_7450, &args![s.node, 1u32]);
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, 0x0054_abd0), vec![vec![s.refr, 0x0def_0000]]);
        // an actor is not attached, and its AI is updated
        let vtable = e.mem.u32(s.refr);
        e.mem.set_u32(vtable + 0x100, RETURNS_TRUE);
        start_log(&mut e);
        e.call(0x0057_7450, &args![s.node, 1u32]);
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, 0x008a_e640), vec![vec![s.refr, 0]]);
        assert!(calls_to(&log, 0x0054_abd0).is_empty());
    }

    // ---- addon-node walkers ---------------------------------------------

    /// A node with an addon record: the record is what `004617e0` returns
    /// for the key `009ee040(node)` gives (the node's own address).
    fn addon_world(e: &mut Engine, records: &[(u32, u32)]) {
        e.register(0x009e_e040, |_, a| a[0].into_ret());
        let table: Vec<(u32, u32)> = records.to_vec();
        e.register_double(0x0046_17e0, move |_, a| Ret {
            eax: table
                .iter()
                .find(|(key, _)| *key == a[1])
                .map_or(0, |(_, record)| *record),
            ..Ret::default()
        });
    }

    #[test]
    fn has_addon_nodes_searches_the_tree() {
        let mut e = engine();
        let leaf = node(&mut e, &[]);
        let root = node(&mut e, &[leaf]);
        returns(&mut e, 0x0045_3470, 0);
        assert!(!e.call(0x0057_8400, &args![root]).bool());
        assert!(!e.call(0x0057_8400, &args![0u32]).bool());
        // the leaf accepted
        returns(&mut e, 0x0045_3470, 0);
        assert!(!e.call(0x0057_8400, &args![root]).bool());
        e.register_double(0x0045_3470, move |_, a| Ret {
            eax: u32::from(a[0] == leaf),
            ..Ret::default()
        });
        assert!(e.call(0x0057_8400, &args![root]).bool());
        // the root itself
        e.register_double(0x0045_3470, move |_, a| Ret {
            eax: u32::from(a[0] == root),
            ..Ret::default()
        });
        start_log(&mut e);
        assert!(e.call(0x0057_8400, &args![root]).bool());
        let log = take_log(&mut e);
        // the search stops at the root
        assert!(calls_to(&log, CHILD_COUNT).is_empty());
    }

    #[test]
    fn remove_addon_nodes_tells_the_record_and_recurses() {
        let mut e = engine();
        let leaf = node(&mut e, &[]);
        let root = node(&mut e, &[leaf]);
        let tell = 0x00ff_00a0;
        returns(&mut e, tell, 0);
        let record = object_with(&mut e, 0x60, &[(0x150, tell)]);
        addon_world(&mut e, &[(root, record)]);
        stub(&mut e, &[0x00c6_9ee0, 0x0057_2160]);
        returns(&mut e, 0x0048_cee0, 1);
        returns(&mut e, 0x0045_3470, 1);
        start_log(&mut e);
        assert!(e.call(0x0057_8300, &args![root]).bool());
        let log = take_log(&mut e);
        assert_eq!(
            calls_to(&log, 0x00c6_9ee0),
            vec![vec![root, 1, 1], vec![leaf, 1, 1]]
        );
        assert_eq!(calls_to(&log, tell), vec![vec![record, 0]]);
        assert_eq!(calls_to(&log, 0x0057_2160), vec![vec![root]]);
        // the leaf is visited (00c69ee0) even though it has no record
        assert_eq!(calls_to(&log, 0x0046_17e0).len(), 2);
        // a record without model: nothing is told
        returns(&mut e, 0x0048_cee0, 0);
        start_log(&mut e);
        assert!(!e.call(0x0057_8300, &args![root]).bool());
        let log = take_log(&mut e);
        assert!(calls_to(&log, tell).is_empty());
        assert!(!e.call(0x0057_8300, &args![0u32]).bool());
    }

    #[test]
    fn master_particle_addon_nodes_are_switched_on_and_off() {
        let mut e = engine();
        let leaf = node(&mut e, &[]);
        let root = node(&mut e, &[leaf]);
        let record = e.mem.alloc(0x80);
        addon_world(&mut e, &[(leaf, record)]);
        returns(&mut e, 0x0044_8bf0, 1);
        stub(&mut e, &[0x0045_0f90, 0x00c5_05c0]);
        returns(&mut e, 0x0059_e300, 0x6000);
        returns(&mut e, 0x0045_a190, 0x6100);
        returns(&mut e, 0x0045_6610, 0);
        start_log(&mut e);
        assert!(e.call(0x0057_8060, &args![root]).bool());
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, 0x0045_0f90), vec![vec![leaf, 0]]);
        assert_eq!(
            calls_to(&log, 0x00c5_05c0),
            vec![vec![0x6100, 0x6000, leaf]]
        );
        // 00456610 stops the descent below the root
        returns(&mut e, 0x0045_6610, 1);
        start_log(&mut e);
        assert!(!e.call(0x0057_8060, &args![root]).bool());
        let log = take_log(&mut e);
        assert!(calls_to(&log, 0x0045_0f90).is_empty());
        assert!(!e.call(0x0057_8060, &args![0u32]).bool());

        // removing
        start_log(&mut e);
        assert!(e.call(0x0057_8170, &args![root]).bool());
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, 0x0045_0f90), vec![vec![leaf, 1]]);
        returns(&mut e, 0x0044_8bf0, 0);
        assert!(!e.call(0x0057_8170, &args![root]).bool());
        assert!(!e.call(0x0057_8170, &args![0u32]).bool());
    }

    #[test]
    fn fn_00578250_visits_the_nodes_it_accepts() {
        let mut e = engine();
        let leaf = node(&mut e, &[]);
        let root = node(&mut e, &[leaf]);
        e.register_double(0x0057_0ea0, move |_, a| Ret {
            eax: u32::from(a[0] == root),
            ..Ret::default()
        });
        returns(&mut e, 0x0045_3a70, 0x6000);
        stub(&mut e, &[0x00ad_8570]);
        start_log(&mut e);
        assert!(e.call(0x0057_8250, &args![root]).bool());
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, 0x00ad_8570), vec![vec![0x6000, root, 0, 0]]);
        // not accepted at the root, but below it
        e.register_double(0x0057_0ea0, move |_, a| Ret {
            eax: u32::from(a[0] == leaf),
            ..Ret::default()
        });
        start_log(&mut e);
        assert!(!e.call(0x0057_8250, &args![root]).bool());
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, 0x00ad_8570).len(), 1);
        assert!(!e.call(0x0057_8250, &args![0u32]).bool());
    }

    #[test]
    fn add_addon_nodes_detaches_base_children_and_attaches_the_record() {
        let mut e = engine();
        // a child named "BASE..." (strncmp of the name with "BASE", 4)
        let detach = 0x00ff_00b0;
        returns(&mut e, detach, 0);
        let child = object_with(&mut e, 0x80, &[(0x0c, AS_NODE_SELF), (0xf0, detach)]);
        let attach = 0x00ff_00b1;
        returns(&mut e, attach, 0);
        let root = object_with(&mut e, 0x80, &[(0x0c, AS_NODE_SELF), (0xdc, attach)]);
        e.mem.set_u32(root + 0x20, 1);
        e.mem.set_u32(root + 0x24, child);
        // child names via 00413f40 / 0043b1b0
        e.register(0x0041_3f40, |_, a| a[0].into_ret());
        returns(&mut e, 0x0048_af70, 1);
        e.register(0x0043_b1b0, |_, a| a[0].into_ret());
        e.register(STRNCMP, |e, a| {
            // equal when the child (name holder) is flagged at +0x10
            assert_eq!(a[1], BASE_PREFIX);
            assert_eq!(a[2], 4);
            i32::from(e.mem.u32(a[0] + 0x10) != 0).into_ret()
        });
        // no addon record for the root: only the detaching and the
        // recursion happen
        addon_world(&mut e, &[]);
        start_log(&mut e);
        assert!(!e.call(0x0057_7e20, &args![root]).bool());
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, detach), vec![vec![child, 0]]);
        assert!(calls_to(&log, 0x0049_02f0).is_empty());
        // a child whose name does not start with BASE stays
        e.mem.set_u32(child + 0x10, 1);
        start_log(&mut e);
        e.call(0x0057_7e20, &args![root]);
        let log = take_log(&mut e);
        assert!(calls_to(&log, detach).is_empty());
        assert!(!e.call(0x0057_7e20, &args![0u32]).bool());
    }

    #[test]
    fn add_addon_nodes_attaches_a_randomized_record() {
        let mut e = engine();
        let attach = 0x00ff_00b1;
        returns(&mut e, attach, 0);
        let root = object_with(&mut e, 0x80, &[(0x0c, AS_NODE_SELF), (0xdc, attach)]);
        // the record: virtual 0x178 creates the node to attach (whose
        // virtual 0x10 gives a second object)
        let created = node(&mut e, &[]);
        let make = 0x00ff_00b2;
        returns(&mut e, make, created);
        let second_getter = 0x00ff_00b3;
        returns(&mut e, second_getter, 0x6000);
        let vtable = e.mem.u32(created);
        e.mem.set_u32(vtable + 0x10, second_getter);
        let record = object_with(&mut e, 0x80, &[(0x178, make)]);
        addon_world(&mut e, &[(root, record)]);
        returns(&mut e, 0x0048_cee0, 1);
        returns(&mut e, 0x0044_8bf0, 0);
        stub(&mut e, &[0x0056_c7d0, 0x0049_02f0, 0x0057_6660]);
        // tick 1234: seed = (1234 % 1000) / 100.0 = 2.34
        returns(&mut e, 0x0048_7f50, 1234);
        e.mem.set_f64(RANDOM_DIVISOR, 100.0);
        // the randomizer finds no entries
        returns(&mut e, 0x0043_b230, 0);
        returns(&mut e, 0x00a5_9d30, 0);
        start_log(&mut e);
        assert!(e.call(0x0057_7e20, &args![root]).bool());
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, 0x0056_c7d0), vec![vec![0x6000, 0]]);
        assert_eq!(calls_to(&log, attach), vec![vec![root, created, 1]]);
        assert_eq!(calls_to(&log, 0x0049_02f0), vec![vec![root, 1]]);
        assert_eq!(calls_to(&log, 0x0057_6660), vec![vec![root, 0]]);
        // the same seed reaches the randomizer's entries
        let entry = e.mem.alloc(16);
        e.register_double(0x0043_b230, move |_, a| Ret {
            eax: if a[0] == created { entry } else { 0 },
            ..Ret::default()
        });
        returns(&mut e, 0x004a_8a90, 0);
        stub(&mut e, &[0x0062_1b20]);
        start_log(&mut e);
        e.call(0x0057_7e20, &args![root]);
        let log = take_log(&mut e);
        assert_eq!(
            calls_to(&log, 0x0062_1b20),
            vec![vec![entry, (2.34f64 as f32).to_bits()]]
        );
        // a record that is excluded by 00448bf0 attaches nothing
        returns(&mut e, 0x0044_8bf0, 1);
        start_log(&mut e);
        assert!(!e.call(0x0057_7e20, &args![root]).bool());
        let log = take_log(&mut e);
        assert!(calls_to(&log, attach).is_empty());
    }

    #[test]
    fn randomize_addons_walks_both_lists_and_the_children() {
        let mut e = engine();
        let leaf = node(&mut e, &[]);
        let root = node(&mut e, &[leaf]);
        let first = e.mem.alloc(16);
        let second_list = e.mem.alloc(16);
        let second = e.mem.alloc(16);
        e.register_double(0x0043_b230, move |_, a| Ret {
            eax: if a[0] == root {
                first
            } else if a[0] == second_list {
                second
            } else {
                0
            },
            ..Ret::default()
        });
        returns(&mut e, 0x004a_8a90, 0);
        e.register_double(0x00a5_9d30, move |_, a| Ret {
            eax: if a[0] == root { second_list } else { 0 },
            ..Ret::default()
        });
        stub(&mut e, &[0x0062_1b20]);
        start_log(&mut e);
        e.call(0x0057_84f0, &args![root, 1.5f32]);
        let log = take_log(&mut e);
        let bits = 1.5f32.to_bits();
        assert_eq!(
            calls_to(&log, 0x0062_1b20),
            vec![vec![first, bits], vec![second, bits]]
        );
        // the children are visited with the same value
        let visited = calls_to(&log, 0x0043_b230);
        assert_eq!(visited, vec![vec![root], vec![second_list], vec![leaf]]);
        assert_eq!(calls_to(&log, 0x00a5_9d30)[0], vec![root, 5]);
        e.call(0x0057_84f0, &args![0u32, 1.5f32]);
    }

    #[test]
    fn has_addon_flags_asks_the_extra_data() {
        let mut e = engine();
        returns(&mut e, 0x0044_8a80, 0x0120_2ddc);
        returns(&mut e, 0x00a5_bdd0, 0);
        returns(&mut e, 0x0044_8a40, 1);
        start_log(&mut e);
        assert!(!e.call(0x0057_84b0, &args![0x77u32]).bool());
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, 0x00a5_bdd0), vec![vec![0x77, 0x0120_2ddc]]);
        assert!(calls_to(&log, 0x0044_8a40).is_empty());
        returns(&mut e, 0x00a5_bdd0, 0x88);
        assert!(e.call(0x0057_84b0, &args![0x77u32]).bool());
        returns(&mut e, 0x0044_8a40, 0);
        assert!(!e.call(0x0057_84b0, &args![0x77u32]).bool());
    }

    // ---- owners ---------------------------------------------------------

    #[test]
    fn fn_00578770_tests_flag_200() {
        let mut e = engine();
        let object = e.mem.alloc(0x40);
        assert!(!e.call(0x0057_8770, &args![object]).bool());
        e.mem.set_u32(object + 0x34, 0x200);
        assert!(e.call(0x0057_8770, &args![object]).bool());
    }

    /// A reference whose owner (`00567790`) is `owner`.
    fn owned(e: &mut Engine, owner: u32) -> u32 {
        returns(e, 0x0056_7790, owner);
        stub(e, &[0x0056_79f0, 0x0056_7960]);
        e.mem.alloc(0x80)
    }

    #[test]
    fn is_an_owner_without_owner_everybody_owns() {
        let mut e = engine();
        let this = owned(&mut e, 0);
        assert!(e.call(0x0057_85e0, &args![this, 0x55u32, false]).bool());
        assert!(e.call(0x0057_85e0, &args![this, 0u32, false]).bool());
    }

    #[test]
    fn is_an_owner_compares_the_actors_form_with_the_owner() {
        let mut e = engine();
        let owner = form(&mut e, 0x30, 0);
        let this = owned(&mut e, owner);
        // no actor: not an owner
        assert!(!e.call(0x0057_85e0, &args![this, 0u32, false]).bool());
        // the actor's own form (virtual 0x1a4) is the owner
        let get_form = 0x00ff_00c0;
        returns(&mut e, get_form, owner);
        let actor = object_with(&mut e, 0x80, &[(0x1a4, get_form)]);
        assert!(e.call(0x0057_85e0, &args![this, actor, false]).bool());
        // without a form of its own the base form is used
        returns(&mut e, get_form, 0);
        e.mem.set_u32(actor + 0x20, owner);
        assert!(e.call(0x0057_85e0, &args![this, actor, false]).bool());
        // another form: not an owner
        let other = form(&mut e, 0x30, 0);
        e.mem.set_u32(actor + 0x20, other);
        assert!(!e.call(0x0057_85e0, &args![this, actor, false]).bool());
    }

    #[test]
    fn is_an_owner_accepts_an_npc_owners_actors_through_008b8e90() {
        let mut e = engine();
        let npc = form(&mut e, 8, 0);
        let this = owned(&mut e, npc);
        let get_form = 0x00ff_00c0;
        // the actor's form is of type 0x2a: it is cast to the bound object
        let actor_form = form(&mut e, 0x2a, 0);
        returns(&mut e, get_form, actor_form);
        let actor = object_with(&mut e, 0x80, &[(0x1a4, get_form)]);
        e.register(DYNAMIC_CAST, |_, a| {
            assert_eq!(a[1..], [0, 0x0118_3108, 0x0118_46e8, 0]);
            a[0].into_ret()
        });
        returns(&mut e, 0x008b_8e90, 1);
        // the flag of the NPC (+0x34 & 0x200) or the argument turns it on
        assert!(!e.call(0x0057_85e0, &args![this, actor, false]).bool());
        e.mem.set_u32(npc + 0x34, 0x200);
        start_log(&mut e);
        assert!(e.call(0x0057_85e0, &args![this, actor, false]).bool());
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, 0x008b_8e90), vec![vec![actor, npc]]);
        e.mem.set_u32(npc + 0x34, 0);
        assert!(e.call(0x0057_85e0, &args![this, actor, true]).bool());
        // 008b8e90 refusing
        returns(&mut e, 0x008b_8e90, 0);
        assert!(!e.call(0x0057_85e0, &args![this, actor, true]).bool());
    }

    #[test]
    fn is_an_owner_never_matches_a_faction_owner_by_value() {
        let mut e = engine();
        let faction = form(&mut e, 0x2a, 0);
        let this = owned(&mut e, faction);
        returns(&mut e, 0x0056_7960, 0x6000);
        returns_float(&mut e, 0x0052_6ac0, 1.0);
        e.mem.set_f64(OWNERSHIP_VALUE_LIMIT, 1.0);
        let get_form = 0x00ff_00c0;
        let actor_form = form(&mut e, 0x30, 0);
        returns(&mut e, get_form, actor_form);
        let actor = object_with(&mut e, 0x80, &[(0x1a4, get_form)]);
        assert!(!e.call(0x0057_85e0, &args![this, actor, false]).bool());
        assert!(!e.call(0x0057_85e0, &args![this, actor, true]).bool());
    }

    #[test]
    fn is_owner_evil_for_an_npc_owner() {
        let mut e = engine();
        let npc = form(&mut e, 8, 0);
        returns(&mut e, 0x0047_d7c0, 1);
        let this = e.mem.alloc(0x80);
        start_log(&mut e);
        assert!(e.call(0x0057_8790, &args![this, npc]).bool());
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, 0x0047_d7c0), vec![vec![npc]]);
        returns(&mut e, 0x0047_d7c0, 0);
        assert!(!e.call(0x0057_8790, &args![this, npc]).bool());
        // without an owner argument the reference's owner is used
        returns(&mut e, 0x0047_d7c0, 1);
        returns(&mut e, 0x0056_7790, npc);
        assert!(e.call(0x0057_8790, &args![this, 0u32]).bool());
        returns(&mut e, 0x0056_7790, 0);
        assert!(!e.call(0x0057_8790, &args![this, 0u32]).bool());
    }

    #[test]
    fn is_owner_evil_for_a_faction_owner_depends_on_the_actor_value() {
        let mut e = engine();
        let faction = form(&mut e, 0x2a, 0);
        let this = e.mem.alloc(0x80);
        // no actor reference: evil
        returns(&mut e, 0x0097_0a20, 0);
        assert!(e.call(0x0057_8790, &args![this, faction]).bool());
        // an actor whose value 0x17 maps to 2 or 4 stays evil; others are not
        let get_value = 0x00ff_00d0;
        returns_float(&mut e, get_value, 3.0);
        let base = object_with(&mut e, 0x200, &[]);
        let values = base + 0x100;
        let vtable = e.mem.alloc(0x40);
        e.mem.set_u32(values, vtable);
        e.mem.set_u32(vtable + 0xc, get_value);
        returns(&mut e, 0x0097_0a20, 0x6000);
        returns(&mut e, 0x0041_81e0, base);
        e.register(0x0047_e040, |_, a| {
            let class = if f32::from_bits(a[0]) == 3.0 {
                2u32
            } else {
                1u32
            };
            class.into_ret()
        });
        start_log(&mut e);
        assert!(e.call(0x0057_8790, &args![this, faction]).bool());
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, get_value), vec![vec![values, 0x17]]);
        assert_eq!(
            calls_to(&log, 0x0097_0a20),
            vec![vec![0x011e_0e80, faction, 1]]
        );
        returns_float(&mut e, get_value, 5.0);
        assert!(!e.call(0x0057_8790, &args![this, faction]).bool());
        // other form types are not evil
        let other = form(&mut e, 0x30, 0);
        assert!(!e.call(0x0057_8790, &args![this, other]).bool());
    }

    // ---- 00578870 .. 00578970 -------------------------------------------

    #[test]
    fn fn_00578870_names_the_location() {
        let mut e = engine();
        let this = scene(&mut e).refr;
        stub(&mut e, &[0x0040_37f0]);
        returns(&mut e, 0x0040_8da0, 0x6000);
        returns(&mut e, 0x0042_5fd0, 1);
        let result = e.mem.alloc(16);
        // an interior cell: its name
        start_log(&mut e);
        e.call(0x0057_8870, &args![this, result]);
        let log = take_log(&mut e);
        let cell = e.mem.u32(this + 0x40);
        assert_eq!(calls_to(&log, 0x0040_8da0), vec![vec![cell + 0x18]]);
        assert_eq!(calls_to(&log, 0x0040_37f0), vec![vec![result, 0x6000, 0]]);
        // an exterior cell: the place object formats the position
        returns(&mut e, 0x0042_5fd0, 0);
        let format = 0x00ff_00e0;
        returns(&mut e, format, 0);
        let place = object_with(&mut e, 0x80, &[(0x138, format)]);
        returns(&mut e, 0x0057_5d70, place);
        e.mem.set_f32(this + 0x90, 1.0);
        start_log(&mut e);
        e.call(0x0057_8870, &args![this, result]);
        let log = take_log(&mut e);
        assert_eq!(
            calls_to(&log, format),
            vec![vec![place, result, 1.0f32.to_bits(), 0, 60.0f32.to_bits()]]
        );
        // no place: the empty string
        returns(&mut e, 0x0057_5d70, 0);
        start_log(&mut e);
        e.call(0x0057_8870, &args![this, result]);
        let log = take_log(&mut e);
        assert_eq!(
            calls_to(&log, 0x0040_37f0),
            vec![vec![result, EMPTY_STRING, 0]]
        );
    }

    #[test]
    fn fn_00578920_passes_the_3d_and_the_flag() {
        let mut e = engine();
        stub(&mut e, &[0x00c6_bd00]);
        let get_model = 0x00ff_00f0;
        returns(&mut e, get_model, 0x6000);
        let this = object_with(&mut e, 0x80, &[(0x1d0, get_model)]);
        start_log(&mut e);
        e.call(0x0057_8920, &args![this, 1u32]);
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, 0x00c6_bd00), vec![vec![0x6000, 1]]);
    }

    #[test]
    fn add_talking_activator_mobile_object_refr_forwards_to_the_base_form() {
        let mut e = engine();
        let base = form(&mut e, 0x30, 0);
        let this = e.mem.alloc(0x80);
        e.mem.set_u32(this + 0x20, base);
        returns(&mut e, 0x004f_f070, 0x4242);
        start_log(&mut e);
        assert_eq!(e.call(0x0057_8950, &args![this]).u32(), 0x4242);
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, 0x004f_f070), vec![vec![base, this]]);
    }

    #[test]
    fn get_mobile_object_extra_reads_the_extra_at_0xc() {
        let mut e = engine();
        let this = e.mem.alloc(0x80);
        returns(&mut e, 0x0042_e110, 0);
        assert_eq!(e.call(0x0057_8970, &args![this]).u32(), 0);
        let extra = e.mem.alloc(0x20);
        e.mem.set_u32(extra + 0xc, 0xfeed);
        returns(&mut e, 0x0042_e110, extra);
        assert_eq!(e.call(0x0057_8970, &args![this]).u32(), 0xfeed);
    }
}
