//! `TESObject`'s unit (the Xbox PDB's `tesobject` source unit, `0050e820` to `0050fdf0`): the
//! `TESObject` base, `TESBoundObject` (its `OBND` box and `Clone3D`), `TESBoundAnimObject` and
//! `TESBoundTreeObject`, each as the exe does it, but `TESObject::IsMarker` (`0050f9e0`), which JIP LN
//! NVSE rewrites: it stays translated.
//!
//! `TESObject` (0x24 bytes): the 0x18-byte form, then its list node (the parent list at +0x18, the
//! previous and next objects at +0x1c and +0x20). `TESBoundObject` adds the `OBND` box at +0x24: two
//! `NiNPShortPoint3` (three signed 16-bit components each), the minimum at +0x24 and the maximum at
//! +0x2a.
//!
//! **Native.** The one-line bodies the exe calls are done in place, each the body it holds (checked
//! in the exe): the form type (`00401170`, the byte at +4), the byte order (`00401500`, the byte at
//! `011c54ba`), the half swap (`00407a90`, the runtime's `_byteswap_ushort`), the file's swapped
//! byte (`00401680`, +0x299), the empty constructors of the box's points (`006815c0`, so the box's
//! constructor `0050e9c0` does nothing), the empty `008d0600` (so `0050f780` does nothing) and the
//! float-to-int helper (`00406d90`: `FISTP` under the thread's rounding, to nearest). The read-only
//! doubles the exe multiplies and compares with (0.5, 1.0, 0.0 and 100.0) are constants. `TESForm`'s
//! constructor, destructor and `__AddChunkData` and this unit's own functions are this family's
//! typed calls; the point maths (`NiPoint3`'s difference and sum, the square root) the pathing
//! family's; `memcpy` the C runtime family's; the word getters and the point constructor the
//! leaves'; a node's child, an object's controllers and `NiDynamicCast` (`0043b4a0`, `0043b230`,
//! `00653270`) dispatch's, the node's child count test (`0045bc00`) the multibound family's, a
//! model's reference count (`0092c870`) the cell references', the node's update (`00a59c60`) the
//! scene family's and a result's word at +0x40 (`008d6f30`) the AI queries'. `Clone3D`'s own slot
//! (0x14c) is a typed call when it holds `Clone3D` itself, and is called through otherwise. What
//! stays a call by address: the model loader and its models, the scene graph's clone and fix-ups,
//! the file's chunk reads, the RTTI cast, the model path getters, the reference's scale and model,
//! `operator new` and `delete`, the logger, `sprintf_s`; every other virtual call goes through its
//! slot. JIP LN's seam in `Clone3D` (`0050ef46`, its clone of a model with lights of its own) runs
//! where it does, behind the same test. The model registration (`0050fbf0`) reads the model
//! loader's pointer (`011c3b3c`) once, where the exe reads it again before each of its four uses:
//! the loader is made once at start-up and never changes after, so the word is the same each time.
//!
//! Floats: the box's components become floats exactly (`FILD`); the box's size is the pathing
//! family's root of the integer sum of squares (stored as a float), halved, as a float; the
//! scaled model name's percent is the exe's `FMUL` under the running thread's control word
//! (`x87_mul`), stored as the float the exe stores and rounded to an integer as `FISTP` does. The
//! scales `Clone3D` and the model registration compare are the callees' `ST0` values compared as
//! `FUCOMPP` / `FCOMP` do (`==` or `!=` on `f64`), the reference's scale stored as the float the exe
//! stores where it does.

use super::{tesform, FAMILY};
use crate::culling::layout::NiPoint3;
use crate::{ai_queries, cell_refs, crt, dispatch, multibound, pathing, scene, slot_target};
use engine_core::extend::jip_patches;
use engine_core::typed::leaf;
use engine_core::x87::{fistp, x87_mul, x87_quiet_bits, x87_store32};
use engine_core::{args, typed_call, Engine, Ptr};

/// The vtables of `TESBoundObject`, `TESBoundAnimObject`, `TESBoundTreeObject` and `TESObject`.
const BOUND_OBJECT_VTABLE: u32 = 0x0102_960c;
const BOUND_ANIM_OBJECT_VTABLE: u32 = 0x0102_980c;
const BOUND_TREE_OBJECT_VTABLE: u32 = 0x0102_9994;
const OBJECT_VTABLE: u32 = 0x0102_9b1c;
/// `TESObject`'s list node: the parent list, the previous and the next object.
const PARENT_LIST: u32 = 0x18;
const PREVIOUS: u32 = 0x1c;
const NEXT: u32 = 0x20;
/// The `OBND` box: the minimum point at +0x24, the maximum at +0x2a, 12 bytes in all.
const BOUND_MIN: u32 = 0x24;
const BOUND_MAX: u32 = 0x2a;
const BOUND_SIZE: u32 = 12;
/// `'OBND'` as a little-endian chunk type.
const OBND_CHUNK: u32 = 0x444e_424f;
/// The function-local default point's guard word (bit 0: constructed) and the point.
const DEFAULT_BOUND_GUARD: u32 = 0x011c_912c;
const DEFAULT_BOUND: u32 = 0x011c_9124;
/// The target's byte order: not 0 when big-endian (`00401500`'s byte).
const BIG_ENDIAN: u32 = 0x011c_54ba;
/// A file's byte that says its current chunk is byte-swapped (`00401680`).
const FILE_SWAPPED_AT: u32 = 0x299;
/// The file's current chunk type and its chunk read (`thiscall` on the file).
const FILE_CURRENT_CHUNK: u32 = 0x0047_26b0;
const FILE_READ_CHUNK: u32 = 0x0047_2890;
/// `operator new(size)` and `operator delete(ptr)`.
const OPERATOR_NEW: u32 = 0x0040_1000;
const OPERATOR_DELETE: u32 = 0x0040_1030;
/// Removes an object from a list (the list as `ECX`).
const LIST_REMOVE: u32 = 0x0051_0130;
/// A form's model path and the virtual slots the unit calls: `Clone3D`, the model's scene node, the
/// virtual after the model path, the type 0x25 form's result.
const FORM_MODEL_PATH: u32 = 0x0048_95b0;
const SLOT_CLONE_3D: u32 = 0x14c;
const SLOT_MODEL_NODE: u32 = 0x174;
const SLOT_AFTER_MODEL_PATH: u32 = 0x17c;
const SLOT_TAKE_RESULT: u32 = 0x180;
/// The two type 0x20 forms for which `Clone3D` and the model registration drop the reference.
const SPECIAL_FORM_A: u32 = 0x011c_a220;
const SPECIAL_FORM_B: u32 = 0x011c_a250;
/// The three type 0x20 forms `0050fb90` looks for.
const NAMED_FORMS_TYPE_20: [u32; 3] = [0x011c_a238, 0x011c_a234, 0x011c_a23c];
/// The model loader (its pointer), the two counters the clone and the registration move, the
/// clone's helper object, the addresses it passes, and the clone's two tables.
const MODEL_LOADER: u32 = 0x011c_3b3c;
const COUNTER_A: u32 = 0x011c_90a8;
const COUNTER_B: u32 = 0x011c_90ac;
const CLONE_HELPER: u32 = 0x011d_ea10;
const CLONE_ADDRESS_ARGUMENT: u32 = 0x011f_36ac;
const CLONE_NAME_TABLE: u32 = 0x011a_9448;
const CLONE_DATA_TABLE: u32 = 0x011f_426c;
/// The messages for a scale of 0 and for a model loader copy of another scale, and their printer.
const SCALE_ZERO_MESSAGE: u32 = 0x0102_97d0;
const SCALE_INVALID_MESSAGE: u32 = 0x0102_9798;
const PRINT_MESSAGE: u32 = 0x005b_5e40;
/// The reference's scale (`ST0`), the model's scale (`ST0`).
const REFERENCE_SCALE: u32 = 0x0056_7400;
const MODEL_SCALE: u32 = 0x009c_dae0;
/// The model loader's register, find and add (`thiscall` on the loader).
const LOADER_REGISTER: u32 = 0x0045_a5e0;
const LOADER_FIND: u32 = 0x0044_72a0;
const LOADER_ADD: u32 = 0x0044_7010;
/// A model holder's constructor, destructor and set.
const HOLDER_CONSTRUCT: u32 = 0x0063_3c90;
const HOLDER_DESTRUCT: u32 = 0x0040_c110;
const HOLDER_SET: u32 = 0x0044_aed0;
/// The clone's process object: its constructor (with the scale) and destructor, 0x1c bytes.
const PROCESS_CONSTRUCT: u32 = 0x004a_d050;
const PROCESS_DESTRUCT: u32 = 0x004a_d270;
const PROCESS_SIZE: u32 = 0x1c;
/// The scene graph's clone.
const SCENE_CLONE: u32 = 0x00a5_d2c0;
/// The scaled model name: its buffer (0x108 bytes, 0x104 of them written), `"%s%i"` and
/// `sprintf_s`.
const NAME_SIZE: u32 = 0x108;
const NAME_ROOM: u32 = 0x104;
const SCALED_NAME_FORMAT: u32 = 0x0102_9ca8;
const PRINT_TO: u32 = 0x0040_6d00;
/// The read-only doubles: 0.5 (the box's half diagonal), 1.0 and 0.0 (the scales), 100.0 (the
/// percent).
const HALF: f64 = 0.5;
const ONE: f64 = 1.0;
const ZERO: f64 = 0.0;
const ONE_HUNDRED: f64 = 100.0;

fn at(this: u32, offset: u32) -> u32 {
    this.wrapping_add(offset)
}

/// The form's type, the byte at +4 (`00401170`).
fn form_type(e: &Engine, form: u32) -> u32 {
    u32::from(e.mem.u8(at(form, 4)))
}

/// Whether the target stores multi-byte values big-endian (`00401500`).
fn is_big_endian(e: &Engine) -> bool {
    e.mem.u8(BIG_ENDIAN) != 0
}

/// The `u16` at `at` byte-swapped in place (`00407a90`).
fn swap_half(e: &mut Engine, at: u32) {
    let half = e.mem.u16(at);
    e.mem.set_u16(at, half.swap_bytes());
}

/// A point component: the signed 16-bit value at `at`.
fn component(e: &Engine, at: u32) -> i16 {
    e.mem.i16(at)
}

/// The low 16 bits of a word, as a 16-bit store keeps them.
#[allow(clippy::cast_possible_truncation)]
fn low_half(word: u32) -> u16 {
    word as u16
}

/// `FSTP dword`: the `ST0` value stored as a float.
#[allow(clippy::cast_possible_truncation)]
fn as_float(value: f64) -> f32 {
    value as f32
}

/// Three words copied from `from` to `to`, in order.
fn copy_three_words(e: &mut Engine, from: u32, to: u32) {
    for offset in [0u32, 4, 8] {
        let word = e.mem.u32(at(from, offset));
        e.mem.set_u32(at(to, offset), word);
    }
}

/// A node's first child (`NiNode::GetAt(0)`, `0043b4a0`), the dispatch family's.
fn first_child(e: &mut Engine, node: u32) -> u32 {
    typed_call!(
        dispatch::FAMILY,
        0x0043_b4a0,
        dispatch::scene::child(e, Ptr::new(node), 0)
    )
    .addr()
}

/// An object's controller list head (`NiObjectNET::GetControllers`, `0043b230`), the dispatch
/// family's.
fn controllers(e: &mut Engine, object: u32) -> u32 {
    typed_call!(
        dispatch::FAMILY,
        0x0043_b230,
        dispatch::scene::first_controller(e, Ptr::new(object))
    )
    .addr()
}

/// `this` is one of the two forms for which the reference is dropped.
fn drops_reference(e: &Engine, this: u32) -> bool {
    this == e.mem.u32(SPECIAL_FORM_A) || this == e.mem.u32(SPECIAL_FORM_B)
}

/// The reference's scale (`ST0`).
fn reference_scale(e: &mut Engine, reference: u32) -> f64 {
    e.call(REFERENCE_SCALE, &args![reference]).f64()
}

/// The model's scale (`ST0`).
fn model_scale(e: &mut Engine, model: u32) -> f64 {
    e.call(MODEL_SCALE, &args![model]).f64()
}

/// A float pushed through `FLD` / `FSTP [ESP]`: its bits, a signalling NaN quieted.
fn pushed(value: f32) -> u32 {
    x87_quiet_bits(value.to_bits())
}

/// `Clone3D` through the form's slot 0x14c: a typed call when the slot holds it.
fn clone_through_slot(e: &mut Engine, this: u32, reference: u32, flag: u8) -> u32 {
    if slot_target(e, this, SLOT_CLONE_3D) == 0x0050_ee50 {
        typed_call!(
            FAMILY,
            0x0050_ee50,
            bound_object_clone_3d(e, this, reference, flag)
        )
    } else {
        e.vcall(this, SLOT_CLONE_3D, &args![reference, u32::from(flag)])
            .u32()
    }
}

/// JIP LN's `CreateObjectNodeHook` (`WriteRelJump(0x50EF46, ..)`; group GL's row, H06's companion):
/// at `0050ef46`, where a model only the model cache holds could be shared, JIP sends a model with
/// point lights of its own to the clone path, so every reference gets its own lights. The seam's
/// words are the model, the form and the reference; an answer other than 0 is "clone". One load
/// while JIP is not installed (the exe's path).
fn jip_always_clones(e: &mut Engine, this: u32, reference: u32, source: u32) -> bool {
    e.jip_patches() & jip_patches::INSTALLED != 0
        && e.site(0x0050_ef46, &[source, this, reference])
            .is_some_and(|answer| answer != 0)
}

// Typed from 0050e820
/// `TESBoundObject::TESBoundObject` (Xbox PDB): `TESObject`'s constructor, the vtable, the box's
/// constructor (its points' empty constructors) and both points copied from the default point.
pub fn bound_object_construct(e: &mut Engine, this: u32) -> u32 {
    typed_call!(FAMILY, 0x0050_f8c0, object_construct(e, this));
    e.mem.set_u32(this, BOUND_OBJECT_VTABLE);
    let default = typed_call!(FAMILY, 0x0050_e8f0, default_bound(e));
    typed_call!(
        FAMILY,
        0x0050_e8b0,
        copy_box_point(e, at(this, BOUND_MIN), default)
    );
    let default = typed_call!(FAMILY, 0x0050_e8f0, default_bound(e));
    typed_call!(
        FAMILY,
        0x0050_e8b0,
        copy_box_point(e, at(this, BOUND_MAX), default)
    );
    this
}

// Typed from 0050e8b0
/// A point's copy: each component read and stored, x, y, then z. Answers `this`.
pub fn copy_box_point(e: &mut Engine, this: u32, source: u32) -> u32 {
    for offset in [0u32, 2, 4] {
        let value = e.mem.u16(at(source, offset));
        e.mem.set_u16(at(this, offset), value);
    }
    this
}

// Typed from 0050e8f0
/// The function-local default point: made (0, 0, 0) the first time (bit 0 of its guard word set
/// first), and its address.
pub fn default_bound(e: &mut Engine) -> u32 {
    let guard = e.mem.u32(DEFAULT_BOUND_GUARD);
    if guard & 1 == 0 {
        e.mem.set_u32(DEFAULT_BOUND_GUARD, guard | 1);
        typed_call!(FAMILY, 0x0050_e960, set_point(e, DEFAULT_BOUND, 0, 0, 0));
    }
    DEFAULT_BOUND
}

// Typed from 0050e960
/// A point's constructor from three components: the low 16 bits of each word stored. Answers
/// `this`.
pub fn set_point(e: &mut Engine, this: u32, x: u32, y: u32, z: u32) -> u32 {
    e.mem.set_u16(this, low_half(x));
    e.mem.set_u16(at(this, 2), low_half(y));
    e.mem.set_u16(at(this, 4), low_half(z));
    this
}

// Typed from 0050e990
/// `TESBoundObject`'s scalar deleting destructor: the destructor's body, then `operator delete`
/// with bit 0 of `flags`. Answers `this`.
pub fn bound_object_deleting_destructor(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0050_e9f0, bound_object_destruct(e, this));
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Typed from 0050e9c0
/// The box's constructor: its two points' constructors (`006815c0`) are empty bodies. Answers
/// `this`.
pub fn bound_data_construct(_e: &mut Engine, this: u32) -> u32 {
    this
}

// Typed from 0050e9f0
/// `TESBoundObject`'s destructor body: the vtable, then `TESObject`'s.
pub fn bound_object_destruct(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, BOUND_OBJECT_VTABLE);
    typed_call!(FAMILY, 0x0050_f930, object_destruct(e, this));
}

/// The two points at +0xc and +0x18 combined by `NiPoint3`'s operator (`subtract` or `add`) into a
/// temporary, whose three words are copied to `result`.
fn combine_points(e: &mut Engine, this: u32, result: u32, sum: bool) {
    let (first, second) = (
        Ptr::<NiPoint3>::new(at(this, 0xc)),
        Ptr::new(at(this, 0x18)),
    );
    e.with_stack(12, |e, temporary| {
        let out = Ptr::<NiPoint3>::new(temporary.addr());
        let combined = if sum {
            typed_call!(
                pathing::FAMILY,
                0x0043_9e90,
                pathing::point::add(e, first, out, second)
            )
        } else {
            typed_call!(
                pathing::FAMILY,
                0x0043_9ef0,
                pathing::point::subtract(e, first, out, second)
            )
        };
        copy_three_words(e, combined.addr(), result);
    });
}

// Typed from 0050ea10
/// The difference of the points at +0xc and +0x18 written to `result` (no result answered).
pub fn points_difference(e: &mut Engine, this: u32, result: u32) {
    combine_points(e, this, result, false);
}

// Typed from 0050ea50
/// The sum of the points at +0xc and +0x18 written to `result` (no result answered).
pub fn points_sum(e: &mut Engine, this: u32, result: u32) {
    combine_points(e, this, result, true);
}

// Typed from 0050ea90
/// `BSBound`'s extra data name (the `NiFixedString` the global at `012031e4` holds).
pub fn bound_extra_data_name(e: &mut Engine) -> u32 {
    e.mem.u32(0x0120_31e4)
}

// Typed from 0050eaa0
/// The box's 12 bytes copied from `source`'s (`memcpy`); answers what `memcpy` answers.
pub fn copy_bound(e: &mut Engine, this: u32, source: u32) -> u32 {
    typed_call!(
        crt::FAMILY,
        0x0040_1460,
        crt::mem::memcpy_wrapper(
            e,
            Ptr::new(at(this, BOUND_MIN)),
            Ptr::new(at(source, BOUND_MIN)),
            BOUND_SIZE
        )
    )
    .addr()
}

/// A point's components as floats (z, y, then x, as the exe reads them), made a `NiPoint3` in a
/// temporary whose three words are copied to `result`. Answers `result`.
fn point_as_floats(e: &mut Engine, point: u32, result: u32) -> u32 {
    let z = typed_call!(FAMILY, 0x0050_eb70, point_z(e, point));
    let y = typed_call!(FAMILY, 0x0050_eb50, point_y(e, point));
    let x = typed_call!(FAMILY, 0x0050_eb30, point_x(e, point));
    e.with_stack(12, |e, temporary| {
        typed_call!(
            leaf::FAMILY,
            0x0041_6870,
            leaf::point3_new(e, temporary, x, y, z)
        );
        copy_three_words(e, temporary.addr(), result);
    });
    result
}

// Typed from 0050ead0
/// The box's minimum as a `NiPoint3` written to `result`. Answers `result`.
pub fn bound_min_as_floats(e: &mut Engine, this: u32, result: u32) -> u32 {
    point_as_floats(e, at(this, BOUND_MIN), result)
}

// Typed from 0050eb30
/// A point's x as a float (exact).
pub fn point_x(e: &mut Engine, this: u32) -> f32 {
    f32::from(component(e, this))
}

// Typed from 0050eb50
/// A point's y as a float (exact).
pub fn point_y(e: &mut Engine, this: u32) -> f32 {
    f32::from(component(e, at(this, 2)))
}

// Typed from 0050eb70
/// A point's z as a float (exact).
pub fn point_z(e: &mut Engine, this: u32) -> f32 {
    f32::from(component(e, at(this, 4)))
}

// Typed from 0050eb90
/// The box's maximum as a `NiPoint3` written to `result`. Answers `result`.
pub fn bound_max_as_floats(e: &mut Engine, this: u32, result: u32) -> u32 {
    point_as_floats(e, at(this, BOUND_MAX), result)
}

// Typed from 0050ebf0
/// `TESBoundObject::GetBoundSize` (Xbox PDB): the length of `minimum - maximum` (a point in a
/// temporary), times 0.5, stored as a float.
pub fn bound_size(e: &mut Engine, this: u32) -> f32 {
    let length = e.with_stack(6, |e, difference| {
        let difference = typed_call!(
            FAMILY,
            0x0050_ec30,
            point_difference(
                e,
                at(this, BOUND_MIN),
                difference.addr(),
                at(this, BOUND_MAX)
            )
        );
        typed_call!(FAMILY, 0x0050_ec80, point_length(e, difference))
    });
    as_float(length * HALF)
}

// Typed from 0050ec30
/// `this - other` stored in `result` by the point's constructor, given each component's whole
/// 32-bit difference as the exe pushes it (z, y and x computed first). Answers `result`.
pub fn point_difference(e: &mut Engine, this: u32, result: u32, other: u32) -> u32 {
    let difference = |e: &Engine, offset: u32| {
        let value =
            i32::from(component(e, at(this, offset))) - i32::from(component(e, at(other, offset)));
        u32::from_ne_bytes(value.to_ne_bytes())
    };
    let z = difference(e, 4);
    let y = difference(e, 2);
    let x = difference(e, 0);
    typed_call!(FAMILY, 0x0050_e960, set_point(e, result, x, y, z));
    result
}

// Typed from 0050ec80
/// A point's length: the 32-bit sum of the components' squares, made a float and given to the
/// square root; its `ST0`.
pub fn point_length(e: &mut Engine, this: u32) -> f64 {
    let square = |e: &Engine, offset: u32| {
        let c = i32::from(component(e, at(this, offset)));
        c.wrapping_mul(c)
    };
    let sum = square(e, 0)
        .wrapping_add(square(e, 2))
        .wrapping_add(square(e, 4));
    // `FILD`, then `FSTP dword` under the thread's rounding: to nearest.
    let value = sum as f32;
    let root = typed_call!(
        pathing::FAMILY,
        0x0045_79e0,
        pathing::point::root_of(e, value)
    );
    f64::from(root)
}

// Typed from 0050ece0
/// Saves the `OBND` chunk: on a big-endian target the box's halves swapped before the 12 bytes are
/// written (`__AddChunkData`) and swapped back after.
pub fn save_bound(e: &mut Engine, this: u32) {
    let bound = at(this, BOUND_MIN);
    if is_big_endian(e) {
        typed_call!(FAMILY, 0x0050_ed30, swap_bound(e, bound));
    }
    typed_call!(
        FAMILY,
        0x0048_5990,
        tesform::add_chunk_data(e, OBND_CHUNK, bound, BOUND_SIZE)
    );
    if is_big_endian(e) {
        typed_call!(FAMILY, 0x0050_ed30, swap_bound(e, bound));
    }
}

// Typed from 0050ed30
/// The box's six halves byte-swapped in place: +6, +8, +0xa, then +0, +2, +4.
pub fn swap_bound(e: &mut Engine, this: u32) {
    for offset in [6u32, 8, 0xa, 0, 2, 4] {
        swap_half(e, at(this, offset));
    }
}

// Typed from 0050edd0
/// Loads the `OBND` chunk from `file`: when it is there and its current chunk is `'OBND'`, 12 bytes
/// read into the box, swapped when the file's chunk is.
pub fn load_bound(e: &mut Engine, this: u32, file: u32) {
    if file == 0 || e.call(FILE_CURRENT_CHUNK, &args![file]).u32() != OBND_CHUNK {
        return;
    }
    let bound = at(this, BOUND_MIN);
    e.call(FILE_READ_CHUNK, &args![file, bound, BOUND_SIZE]);
    if e.mem.u8(at(file, FILE_SWAPPED_AT)) != 0 {
        typed_call!(FAMILY, 0x0050_ed30, swap_bound(e, bound));
    }
}

// Typed from 0050ee20
/// `Clone3D(reference, 0)` through the form's slot.
pub fn clone_3d_unflagged(e: &mut Engine, this: u32, reference: u32) -> u32 {
    clone_through_slot(e, this, reference, 0)
}

/// Whether `Clone3D` may hand back `source` itself: the per-type checks, the extra data, the
/// texture swap and the emittance; `flag` becomes 1 for a type 0x15 or 0x2b model that asks for a
/// deep copy. A shareable model is shared when the reference's scale is the model's.
fn shares_model(e: &mut Engine, this: u32, reference: u32, source: u32, flag: &mut u8) -> bool {
    let mut shareable = false;
    match form_type(e, this) {
        0x15 | 0x2b => {
            if e.call(0x004b_5bf0, &args![source]).bool() {
                *flag = 1;
            }
        }
        0x1c => {
            shareable = true;
            let node = e.vcall(source, 0xc, &[]).u32();
            if node != 0
                && typed_call!(
                    multibound::FAMILY,
                    0x0045_bc00,
                    multibound::getters::node_child(e, Ptr::new(node), 0)
                ) != 0
            {
                let first = first_child(e, node);
                if controllers(e, first) != 0 {
                    let second = first_child(e, node);
                    let key = controllers(e, second);
                    let found = typed_call!(
                        dispatch::FAMILY,
                        0x0065_3270,
                        dispatch::scene::cast_if(e, CLONE_ADDRESS_ARGUMENT, key)
                    );
                    if found != 0 {
                        let table = typed_call!(FAMILY, 0x0050_f5c0, clone_controller_word(e));
                        if e.call(0x0047_a520, &args![found, table]).u32() != 0 {
                            shareable = false;
                        }
                    }
                }
            }
        }
        0x23 | 0x28 | 0x29 | 0x2a | 0x33 | 0x51 => {}
        _ => {
            shareable = e.call(0x0062_c3c0, &args![reference, source]).u32() & 0xff == 0;
        }
    }
    if shareable {
        let property_name = e.call(0x0044_8a80, &[]).u32();
        let extra = e.call(0x00a5_bdd0, &args![source, property_name]).u32();
        if extra != 0 && e.call(0x0044_8a40, &args![extra]).bool() {
            shareable = false;
        }
    }
    if shareable && e.call(0x0048_b310, &args![this, reference]).bool() {
        shareable = false;
    }
    if shareable && e.call(0x0056_9410, &args![source]).bool() {
        shareable = false;
    }
    if !shareable {
        return false;
    }
    if reference == 0 {
        return true;
    }
    let scale = reference_scale(e, reference);
    scale == model_scale(e, source)
}

/// The model looked up by its scaled file name in the model loader: added and cloned again at 1.0
/// when it is not there; when it is there and its scale is the reference's, the form's own
/// `Clone3D` answers (`Some`), otherwise the mismatch is printed. `clone` is what to finish.
fn clone_scaled(
    e: &mut Engine,
    this: u32,
    reference: u32,
    flag: u8,
    clone: &mut u32,
    process: u32,
) -> Option<u32> {
    let name = e.mem.alloc(NAME_SIZE);
    typed_call!(
        FAMILY,
        0x0050_fdf0,
        scaled_model_file_name(e, this, reference, name)
    );
    let model = e.mem.alloc(4);
    e.call(HOLDER_CONSTRUCT, &args![model, 0u32]);
    let loader = e.mem.u32(MODEL_LOADER);
    e.call(LOADER_FIND, &args![loader, name, model]);
    if typed_call!(leaf::FAMILY, 0x0055_9450, leaf::word_at_0(e, model)) == 0 {
        let block = e.call(OPERATOR_NEW, &args![0x10u32]).u32();
        let created = if block != 0 {
            e.call(0x0043_abf0, &args![block, name, *clone]).u32()
        } else {
            0
        };
        e.call(HOLDER_SET, &args![model, created]);
        let current = typed_call!(leaf::FAMILY, 0x0055_9450, leaf::word_at_0(e, model));
        typed_call!(
            cell_refs::FAMILY,
            0x0092_c870,
            cell_refs::queue::add_ref(e, current)
        );
        let current = typed_call!(leaf::FAMILY, 0x0055_9450, leaf::word_at_0(e, model));
        let loader = e.mem.u32(MODEL_LOADER);
        if e.call(LOADER_ADD, &args![loader, name, current]).bool() {
            let counter = e.mem.u32(COUNTER_A);
            e.mem.set_u32(COUNTER_A, counter.wrapping_add(1));
        } else {
            let current = typed_call!(leaf::FAMILY, 0x0055_9450, leaf::word_at_0(e, model));
            e.call(HOLDER_SET, &args![model, 0u32]);
            if current != 0 {
                e.call(0x0044_31f0, &args![current, 1u32]);
            }
        }
        let second_process = e.mem.alloc(PROCESS_SIZE);
        e.call(PROCESS_CONSTRUCT, &args![second_process, pushed(1.0)]);
        *clone = e.call(SCENE_CLONE, &args![*clone, second_process]).u32();
        e.call(PROCESS_DESTRUCT, &args![second_process]);
        e.mem.free(second_process);
    } else {
        let current = typed_call!(leaf::FAMILY, 0x0055_9450, leaf::word_at_0(e, model));
        if controllers(e, current) != 0 {
            let current = typed_call!(leaf::FAMILY, 0x0055_9450, leaf::word_at_0(e, model));
            let found = controllers(e, current);
            let found_scale = model_scale(e, found);
            if found_scale == reference_scale(e, reference) {
                let result = clone_through_slot(e, this, reference, flag);
                e.call(HOLDER_DESTRUCT, &args![model]);
                e.call(PROCESS_DESTRUCT, &args![process]);
                e.mem.free(model);
                e.mem.free(name);
                e.mem.free(process);
                return Some(result);
            }
        }
        e.call(PRINT_MESSAGE, &args![SCALE_INVALID_MESSAGE, name]);
    }
    e.call(HOLDER_DESTRUCT, &args![model]);
    e.mem.free(model);
    e.mem.free(name);
    None
}

// Typed from 0050ee50
/// `TESBoundObject::Clone3D` (Xbox PDB): a scene-graph copy of the form's model for `reference`
/// (null for none); `flag` asks for a deep copy. In the exe's order: the model from slot 0x174 (the
/// reference dropped for the two special forms); a reference whose owner is not of type 0x1e gets
/// the model itself; a model only the cache holds may be shared ([`shares_model`], JIP LN's seam
/// first); otherwise it is cloned (the scene graph's clone, or a deep copy through the helper object
/// with `flag`) at the reference's scale (1.0 when the model has it already; a scale of 0 printed
/// and made 1.0), and a scale other than 1.0 on a model of 1.0 goes through the model loader by
/// the scaled file name ([`clone_scaled`]). The result gets the final fix-ups.
pub fn bound_object_clone_3d(e: &mut Engine, this: u32, reference: u32, flag: u8) -> u32 {
    let reference = if drops_reference(e, this) {
        0
    } else {
        reference
    };
    let mut flag = flag;
    let source = e.vcall(this, SLOT_MODEL_NODE, &args![reference]).u32();
    if reference != 0 && e.call(0x0044_4ed0, &args![reference]).bool() {
        let owner = typed_call!(leaf::FAMILY, 0x007a_f430, leaf::word_at_20(e, reference));
        if form_type(e, owner) != 0x1e {
            return source;
        }
    }
    if source == 0 {
        return 0;
    }
    if e.call(0x0050_d4a0, &args![this]).bool() && reference != 0 {
        e.call(0x0069_3ef0, &args![reference, 1u32]);
    }
    let mut clone = 0u32;
    if flag == 0
        && typed_call!(leaf::FAMILY, 0x0072_6070, leaf::word_at_4(e, source)) == 1
        && !jip_always_clones(e, this, reference, source)
        && shares_model(e, this, reference, source, &mut flag)
    {
        clone = source;
    }
    if clone == 0 {
        let mut scale = 1.0f32;
        if reference != 0 {
            scale = as_float(reference_scale(e, reference));
        }
        if model_scale(e, source) == f64::from(scale) {
            scale = 1.0;
        }
        if f64::from(scale) == ZERO {
            let name = e.call(0x0055_d520, &args![reference]).u32();
            e.call(PRINT_MESSAGE, &args![SCALE_ZERO_MESSAGE, name]);
            scale = 1.0;
        }
        let process = e.mem.alloc(PROCESS_SIZE);
        e.call(PROCESS_CONSTRUCT, &args![process, pushed(scale)]);
        if flag != 0 {
            let helper = e.mem.u32(CLONE_HELPER);
            let copy = e.call(0x0045_7ba0, &args![helper, source, process]).u32();
            if copy != 0 && e.vcall(copy, 0xc, &[]).u32() != 0 {
                clone = e.vcall(copy, 0xc, &[]).u32();
            } else {
                let block = e.call(0x00aa_13e0, &args![0xacu32]).u32();
                let node = if block != 0 {
                    e.call(0x00a5_ecb0, &args![block, 0u32]).u32()
                } else {
                    0
                };
                e.vcall(node, 0xdc, &args![copy, 1u32]);
                clone = node;
            }
        } else {
            clone = e.call(SCENE_CLONE, &args![source, process]).u32();
        }
        if f64::from(scale) != ONE && model_scale(e, source) == ONE {
            e.call(0x0044_0490, &args![clone, pushed(scale)]);
            if let Some(result) = clone_scaled(e, this, reference, flag, &mut clone, process) {
                return result;
            }
        }
        e.call(PROCESS_DESTRUCT, &args![process]);
        e.mem.free(process);
    }
    if clone != 0 {
        e.call(0x0044_0490, &args![clone, pushed(1.0)]);
        e.call(0x0045_0f90, &args![clone, 0u32]);
        e.call(0x0043_fa80, &args![clone, CLONE_NAME_TABLE]);
        e.call(0x0044_0460, &args![clone, CLONE_DATA_TABLE]);
        if e.call(0x0096_11e0, &args![clone]).u32() != 0 {
            typed_call!(FAMILY, 0x0050_f5a0, clear_parent_list(e, clone));
        }
    }
    clone
}

// Typed from 0050f5a0
/// The word at +0x18 cleared (`TESObject`'s parent list; `Clone3D` uses the body on its clone).
pub fn clear_parent_list(e: &mut Engine, this: u32) {
    e.mem.set_u32(at(this, PARENT_LIST), 0);
}

// Typed from 0050f5c0
/// The global word at `011c6218`, which `Clone3D` hands the found controller's test (`0047a520`).
pub fn clone_controller_word(e: &mut Engine) -> u32 {
    e.mem.u32(0x011c_6218)
}

// Typed from 0050f5d0
/// The form's slot 0x17c with its model path; its answer.
pub fn model_path_slot(e: &mut Engine, this: u32) -> u32 {
    let model = e.call(FORM_MODEL_PATH, &args![this]).u32();
    e.vcall(this, SLOT_AFTER_MODEL_PATH, &args![model]).u32()
}

// Typed from 0050f600
/// The form's model path registered with the model loader, its scene node (slot 0x174 with 0)
/// prepared (`00a5a040`) and moved to the origin (`00a59c60`, the point `0043d410` makes). Always
/// true; the word is not read.
pub fn register_model(e: &mut Engine, this: u32, _unused: u32) -> bool {
    let model = e.call(FORM_MODEL_PATH, &args![this]).u32();
    let loader = e.mem.u32(MODEL_LOADER);
    e.call(LOADER_REGISTER, &args![loader, model]);
    let node = e.vcall(this, SLOT_MODEL_NODE, &args![0u32]).u32();
    e.call(0x00a5_a040, &args![node]);
    e.with_stack(12, |e, origin| {
        e.call(0x0043_d410, &args![origin, 0.0f32, 0u32, 0u32]);
        typed_call!(
            scene::FAMILY,
            0x00a5_9c60,
            scene::walk::update(e, Ptr::new(node), origin.addr())
        );
    });
    true
}

// Typed from 0050f670
/// `object` cast (RTTI, `011841cc` to `011846d4`) and, when the cast and `target` are not null, its
/// slot 0x3d0 with (`target`, `extra`, `flag`); whether it called. The fourth word is not read.
pub fn cast_and_call(
    e: &mut Engine,
    _this: u32,
    target: u32,
    object: u32,
    flag: u8,
    _unused: u32,
    extra: u32,
) -> bool {
    let cast = e
        .call(
            0x00ec_43fb,
            &args![object, 0u32, 0x0118_41ccu32, 0x0118_46d4u32, 0u32],
        )
        .u32();
    if cast == 0 || target == 0 {
        return false;
    }
    e.vcall(cast, 0x3d0, &args![target, extra, flag]);
    true
}

// Typed from 0050f6e0
/// The file finder (the global at `011f81dc`).
pub fn file_finder(e: &mut Engine) -> u32 {
    e.mem.u32(0x011f_81dc)
}

// Typed from 0050f6f0
/// `TESBoundAnimObject::TESBoundAnimObject` (Xbox PDB): `TESBoundObject`'s constructor, then its
/// vtable.
pub fn bound_anim_object_construct(e: &mut Engine, this: u32) -> u32 {
    typed_call!(FAMILY, 0x0050_e820, bound_object_construct(e, this));
    e.mem.set_u32(this, BOUND_ANIM_OBJECT_VTABLE);
    this
}

// Typed from 0050f710
/// `TESBoundAnimObject`'s scalar deleting destructor.
pub fn bound_anim_object_deleting_destructor(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0050_f740, bound_anim_object_destruct(e, this));
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Typed from 0050f740
/// `TESBoundAnimObject`'s destructor body: its vtable, then `TESBoundObject`'s.
pub fn bound_anim_object_destruct(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, BOUND_ANIM_OBJECT_VTABLE);
    typed_call!(FAMILY, 0x0050_e9f0, bound_object_destruct(e, this));
}

// Typed from 0050f760
/// `0050f600` with the word; its answer.
pub fn register_model_again(e: &mut Engine, this: u32, argument: u32) -> bool {
    typed_call!(FAMILY, 0x0050_f600, register_model(e, this, argument))
}

// Typed from 0050f780
/// Forwards to `008d0600`, an empty body: nothing.
pub fn forward_to_empty(_e: &mut Engine, _this: u32, _argument: u32, _flag: u8) {}

// Typed from 0050f7a0
/// `TESBoundTreeObject`'s constructor: `TESBoundObject`'s, then its vtable.
pub fn bound_tree_object_construct(e: &mut Engine, this: u32) -> u32 {
    typed_call!(FAMILY, 0x0050_e820, bound_object_construct(e, this));
    e.mem.set_u32(this, BOUND_TREE_OBJECT_VTABLE);
    this
}

// Typed from 0050f7c0
/// `TESBoundTreeObject`'s scalar deleting destructor.
pub fn bound_tree_object_deleting_destructor(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0050_f7f0, bound_tree_object_destruct(e, this));
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Typed from 0050f7f0
/// `TESBoundTreeObject`'s destructor body: its vtable, then `TESBoundObject`'s.
pub fn bound_tree_object_destruct(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, BOUND_TREE_OBJECT_VTABLE);
    typed_call!(FAMILY, 0x0050_e9f0, bound_object_destruct(e, this));
}

// Typed from 0050f810
/// For a form of type 0x25: the object manager's (`00664840` with 1) result for (`argument`, the
/// form) given to the form's slot 0x180; when that result's slot 0x100 answers and `argument` is
/// there, its `008d6f30` passed to `00569350` on `argument`. Answers the result (0 for another
/// type); the second word is not read.
pub fn type_25_result(e: &mut Engine, this: u32, argument: u32, _unused: u32) -> u32 {
    if form_type(e, this) != 0x25 {
        return 0;
    }
    let mut result = 0;
    if e.call(0x0066_4840, &args![1u32]).u32() != 0 {
        let manager = e.call(0x0066_4840, &args![1u32]).u32();
        result = e.call(0x0066_4f50, &args![manager, argument, this]).u32();
    }
    e.vcall(this, SLOT_TAKE_RESULT, &args![result]);
    if result != 0 && e.vcall(result, 0x100, &[]).u32() != 0 && argument != 0 {
        let answer = e.vcall(result, 0x100, &[]).u32();
        let value = typed_call!(
            ai_queries::FAMILY,
            0x008d_6f30,
            ai_queries::queries::word_at_40(e, answer)
        );
        e.call(0x0056_9350, &args![argument, value]);
    }
    result
}

// Typed from 0050f8c0
/// `TESObject::TESObject`: `TESForm`'s constructor, the vtable and the list node cleared.
pub fn object_construct(e: &mut Engine, this: u32) -> u32 {
    typed_call!(FAMILY, 0x0048_3370, tesform::construct(e, this));
    e.mem.set_u32(this, OBJECT_VTABLE);
    e.mem.set_u32(at(this, PREVIOUS), 0);
    e.mem.set_u32(at(this, NEXT), 0);
    e.mem.set_u32(at(this, PARENT_LIST), 0);
    this
}

// Typed from 0050f900
/// `TESObject`'s scalar deleting destructor.
pub fn object_deleting_destructor(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0050_f930, object_destruct(e, this));
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Typed from 0050f930
/// `TESObject`'s destructor body: its vtable, the object out of its parent list when it has one,
/// then `TESForm`'s.
pub fn object_destruct(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, OBJECT_VTABLE);
    let list = e.mem.u32(at(this, PARENT_LIST));
    if list != 0 {
        e.call(LIST_REMOVE, &args![list, this]);
    }
    typed_call!(FAMILY, 0x0048_3630, tesform::destructor(e, this));
}

// Typed from 0050f9a0
/// The previous object (+0x1c) set.
pub fn set_previous(e: &mut Engine, this: u32, previous: u32) {
    e.mem.set_u32(at(this, PREVIOUS), previous);
}

// Typed from 0050f9c0
/// A word stored at +0x20 (the next object; other callers use the body for a word of their own).
pub fn set_word_20(e: &mut Engine, this: u32, value: u32) {
    e.mem.set_u32(at(this, NEXT), value);
}

// Typed from 0050fb90
/// Whether the form is there, of type 0x20 and one of the three globals.
pub fn is_named_type_20(e: &mut Engine, this: u32) -> bool {
    this != 0
        && form_type(e, this) == 0x20
        && NAMED_FORMS_TYPE_20
            .iter()
            .any(|global| e.mem.u32(*global) == this)
}

// Typed from 0050fbe0
/// Two words not read; 0.
pub fn zero(_e: &mut Engine, _this: u32, _unused_1: u32, _unused_2: u32) -> u32 {
    0
}

// Typed from 0050fbf0
/// The form's model registered with the model loader (the reference dropped for the two special
/// forms): a path with a reference of a scale other than 1.0 registers the scaled file name, and,
/// unless the loader holds that model with a reference count above 0, the plain path too (counter A
/// down one; counter B down one otherwise); any other path is registered as it is. A form of type
/// 0x2a then runs `00603bb0`.
pub fn register_form_model(e: &mut Engine, this: u32, reference: u32) {
    let reference = if drops_reference(e, this) {
        0
    } else {
        reference
    };
    let path = typed_call!(FAMILY, 0x0050_fd90, model_path(e, this, reference));
    let loader = e.mem.u32(MODEL_LOADER);
    if path != 0 && e.mem.u8(path) != 0 {
        if reference != 0 && reference_scale(e, reference) != ONE {
            let name = e.mem.alloc(NAME_SIZE);
            typed_call!(
                FAMILY,
                0x0050_fdf0,
                scaled_model_file_name(e, this, reference, name)
            );
            e.call(LOADER_REGISTER, &args![loader, name]);
            let model = e.mem.alloc(4);
            e.call(HOLDER_CONSTRUCT, &args![model, 0u32]);
            let mut held = false;
            if e.call(LOADER_FIND, &args![loader, name, model]).bool() {
                let found = typed_call!(leaf::FAMILY, 0x0055_9450, leaf::word_at_0(e, model));
                let count = typed_call!(leaf::FAMILY, 0x0044_ddc0, leaf::word_at_8(e, found));
                held = i32::from_ne_bytes(count.to_ne_bytes()) > 0;
            }
            if held {
                let counter = e.mem.u32(COUNTER_B);
                e.mem.set_u32(COUNTER_B, counter.wrapping_sub(1));
            } else {
                e.call(LOADER_REGISTER, &args![loader, path]);
                let counter = e.mem.u32(COUNTER_A);
                e.mem.set_u32(COUNTER_A, counter.wrapping_sub(1));
            }
            e.call(HOLDER_DESTRUCT, &args![model]);
            e.mem.free(model);
            e.mem.free(name);
        } else {
            e.call(LOADER_REGISTER, &args![loader, path]);
        }
    }
    if form_type(e, this) == 0x2a {
        e.call(0x0060_3bb0, &args![this]);
    }
}

// Typed from 0050fd90
/// The model path for the form and an optional reference: the reference's own (`005715d0`), else
/// the form's biped model one (`00480e60`), else the form's model path.
pub fn model_path(e: &mut Engine, this: u32, reference: u32) -> u32 {
    let mut path = 0;
    if reference != 0 {
        path = e.call(0x0057_15d0, &args![reference]).u32();
    }
    if path == 0 {
        path = e.call(0x0048_0e60, &args![this, reference]).u32();
    }
    if path == 0 {
        path = e.call(FORM_MODEL_PATH, &args![this]).u32();
    }
    path
}

// Typed from 0050fdf0
/// `TESObject::GetScaledModelFileName` (Xbox PDB): the model path followed by the reference's
/// scale in percent (the scale times 100.0, stored as a float, rounded to an integer by `FISTP`),
/// written into the 0x104 bytes at `name` with `"%s%i"` (`sprintf_s`).
pub fn scaled_model_file_name(e: &mut Engine, this: u32, reference: u32, name: u32) {
    let path = typed_call!(FAMILY, 0x0050_fd90, model_path(e, this, reference));
    let product = x87_store32(x87_mul(reference_scale(e, reference), ONE_HUNDRED));
    // `FLD` / `FSTP [ESP]`, then `00406d90`'s `FLD` and `FISTP`.
    let percent = fistp(f64::from(f32::from_bits(x87_quiet_bits(product))));
    let percent = u32::from_ne_bytes(percent.to_ne_bytes());
    e.call(
        PRINT_TO,
        &args![name, NAME_ROOM, SCALED_NAME_FORMAT, path, percent],
    );
}
