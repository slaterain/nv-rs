//! `TESAttributes`' functions (the Xbox PDB's `tesattributes` source unit) and the unit after it
//! (`run_004801e0`: the attributes' save game functions and `TESBipedModelForm`'s constructor and
//! destructor), each as the exe does it.
//!
//! `TESAttributes` is a form component (at +0xb8 of `TESNPC` and `TESCreature`): its vtable, then
//! the seven attribute bytes at +4 to +0xa. An actor value code names an attribute: codes start at
//! 5 (`ActorValue::ToActorValue(0, index)` is `index + 5`), so code `c` is the byte at
//! `this + c - 1`. `TESBipedModelForm` (0xdc bytes): the component vtable, the biped model data at
//! +4, two `TESModelTextureSwap` arrays at +0xc and +0x4c (two of 0x20 bytes each), the icons at
//! +0x8c (two `TESIcon` of 0xc), the message icons at +0xa4 (two `BGSMessageIcon` of 0x10) and the
//! constraint template at +0xc4 (`TESModelRDT`).
//!
//! **Native.** The one-line bodies the exe calls are done in place, each the body it holds (checked
//! in the exe): the attributes' size (`009d2510`: 7), `ToActorValue` for the attribute group
//! (`0066ec10` with 0: `index + 5`), the attribute byte (`00480160`, with `EAX`'s upper three
//! bytes the sum's, as the exe leaves them), a save buffer's change flags (`00428110`, the word at
//! +0x17, and `004280f0`, its bits), the base component's constructor (`00402d30`, whose vtable
//! the class's own replaces at once) and `TESModelRDT`'s constructor inside the biped model form's.
//! `memcpy` is the C runtime family's typed call; the vtable slot that marks a form changed is this
//! family's typed call when it holds one of its `AddChange`s (`TESActorBase`'s `005f1630`, which
//! NPCs and creatures keep, or `TESForm`'s `00484b60`), and any other target is called through it.
//! What stays a call by address: the RTTI casts, the save buffers' reads and writes, the vector
//! constructor and destructor iterators and the members' own constructors and destructors. The
//! three save functions whose change mask the compiler folded to 0 do nothing, as in the exe.
//!
//! No floats.

use super::{actor_base, tesform, FAMILY};
use crate::{crt, slot_target};
use engine_core::{args, typed_call, Engine, Ptr};

/// `__RTDynamicCast` (cdecl: object, vftable delta, source type, target type, is-reference).
const DYNAMIC_CAST: u32 = 0x00ec_43fb;
const TYPE_BASE_FORM_COMPONENT: u32 = 0x0118_3040;
const TYPE_TES_ATTRIBUTES: u32 = 0x0118_6c74;
const TYPE_TES_FORM: u32 = 0x0118_3028;
/// The attribute bytes: seven from +4.
const ATTRIBUTES: u32 = 4;
const ATTRIBUTE_COUNT: u32 = 7;
/// The value `InitializeDataComponent` gives every attribute.
const ATTRIBUTE_DEFAULT: u8 = 5;
/// A form's vtable slot that marks it changed (with the change flag).
const SLOT_MARK_CHANGED: u32 = 0x48;
/// The change flag of the attributes.
const CHANGE_ATTRIBUTES: u32 = 4;
/// A save/load buffer's change flags: the word at +0x17 (`00428110`).
const BUFFER_CHANGE_FLAGS: u32 = 0x17;
const SAVE_BUFFER_WRITE: u32 = 0x0086_5e50;
const LOAD_BUFFER_READ: u32 = 0x0086_4980;
const BIPED_MODEL_FORM_VTABLE: u32 = 0x0101_bfd8;
const MODEL_RDT_VTABLE: u32 = 0x0101_bfec;
const VECTOR_CONSTRUCT: u32 = 0x00ec_782f;
const VECTOR_DESTRUCT: u32 = 0x00ec_5fce;
const TEXTURE_SWAP_CONSTRUCT: u32 = 0x0048_a3d0;
const TEXTURE_SWAP_DESTRUCT: u32 = 0x0048_a490;
const ICON_CONSTRUCT: u32 = 0x0040_2c20;
const ICON_DESTRUCT: u32 = 0x0040_2c40;
const MESSAGE_ICON_CONSTRUCT: u32 = 0x0047_a130;
const MESSAGE_ICON_DESTRUCT: u32 = 0x0047_a1a0;
const BIPED_INITIALIZE_DATA_COMPONENT: u32 = 0x0048_04d0;
const BIPED_CLEAR_DATA_COMPONENT: u32 = 0x0048_0570;
const MODEL_RDT_BASE_CONSTRUCT: u32 = 0x0048_8f50;
const MODEL_RDT_DESTRUCT: u32 = 0x0051_b950;
/// The biped model form's members: the two model arrays, the icons, the message icons and the
/// constraint template, with each array's element size.
const MODELS: u32 = 0xc;
const WORLD_MODELS: u32 = 0x4c;
const MODEL_SIZE: u32 = 0x20;
const ICONS: u32 = 0x8c;
const ICON_SIZE: u32 = 0xc;
const MESSAGE_ICONS: u32 = 0xa4;
const MESSAGE_ICON_SIZE: u32 = 0x10;
const CONSTRAINT_TEMPLATE: u32 = 0xc4;

fn at(this: u32, offset: u32) -> u32 {
    this.wrapping_add(offset)
}

/// `other` as a `TESAttributes` (the RTTI cast from `BaseFormComponent`), or 0.
fn as_attributes(e: &mut Engine, other: u32) -> u32 {
    e.call(
        DYNAMIC_CAST,
        &args![
            other,
            0u32,
            TYPE_BASE_FORM_COMPONENT,
            TYPE_TES_ATTRIBUTES,
            0u32
        ],
    )
    .u32()
}

/// `memcpy(to, from, 7)`, the game's wrapper (`00401460`).
fn copy_attributes(e: &mut Engine, to: u32, from: u32) {
    typed_call!(
        crt::FAMILY,
        0x0040_1460,
        crt::mem::memcpy_wrapper(e, Ptr::new(to), Ptr::new(from), ATTRIBUTE_COUNT)
    );
}

/// Whether a save/load buffer's change flags have the attributes' flag (`00428110`, `004280f0`).
fn buffer_has_attributes(e: &mut Engine, buffer: u32) -> bool {
    e.mem.u32(at(buffer, BUFFER_CHANGE_FLAGS)) & CHANGE_ATTRIBUTES != 0
}

// Typed from 00480130
/// The attributes' record data read: the seven bytes copied from `source` to +4; the size read
/// (7, `009d2510`).
pub fn load_data(e: &mut Engine, this: u32, source: u32) -> u32 {
    copy_attributes(e, at(this, ATTRIBUTES), source);
    ATTRIBUTE_COUNT
}

// Typed from 0047ffc0
/// `TESAttributes::InitializeDataComponent` (Xbox PDB): every attribute 5.
pub fn initialize_data_component(e: &mut Engine, this: u32) {
    for index in 0..ATTRIBUTE_COUNT {
        e.mem
            .set_u8(at(this, ATTRIBUTES + index), ATTRIBUTE_DEFAULT);
    }
}

// Typed from 00480000
/// `TESAttributes::CopyComponent` (Xbox PDB): when `source` is a `TESAttributes`, its seven
/// attributes copied byte by byte, the first first.
pub fn copy_component(e: &mut Engine, this: u32, source: u32) {
    let source = as_attributes(e, source);
    if source == 0 {
        return;
    }
    for index in 0..ATTRIBUTE_COUNT {
        let value = e.mem.u8(at(source, ATTRIBUTES + index));
        e.mem.set_u8(at(this, ATTRIBUTES + index), value);
    }
}

// Typed from 00480160
/// The attribute an actor value `code` names (the byte at `this + code - 1`) in `AL`; `EAX`'s
/// upper three bytes are `this + code`'s, as the exe leaves them.
pub fn attribute(e: &mut Engine, this: u32, code: u32) -> u32 {
    let sum = this.wrapping_add(code);
    (sum & 0xffff_ff00) | u32::from(e.mem.u8(sum.wrapping_sub(1)))
}

// Typed from 00480180
/// `TESAttributes::SetAttributeValue` (Xbox PDB): the attribute `code` names set to `value`; with
/// `mark`, the form this component is part of (an RTTI cast to `TESForm`) marked changed with the
/// attributes' flag (its vtable slot 0x48).
pub fn set_attribute_value(e: &mut Engine, this: u32, code: u32, value: u8, mark: u8) {
    e.mem.set_u8(this.wrapping_add(code).wrapping_sub(1), value);
    if mark == 0 {
        return;
    }
    let form = e
        .call(
            DYNAMIC_CAST,
            &args![this, 0u32, TYPE_TES_ATTRIBUTES, TYPE_TES_FORM, 0u32],
        )
        .u32();
    if form == 0 {
        return;
    }
    // The slot holds `TESActorBase::AddChange` for an NPC or a creature and `TESForm::AddChange`
    // for a form that keeps it: both this family's.
    match slot_target(e, form, SLOT_MARK_CHANGED) {
        0x005f_1630 => typed_call!(
            FAMILY,
            0x005f_1630,
            actor_base::add_change(e, form, CHANGE_ATTRIBUTES)
        ),
        0x0048_4b60 => typed_call!(
            FAMILY,
            0x0048_4b60,
            tesform::add_change(e, form, CHANGE_ATTRIBUTES)
        ),
        _ => {
            e.vcall(form, SLOT_MARK_CHANGED, &args![CHANGE_ATTRIBUTES]);
        }
    }
}

// Typed from 00480070
/// `TESAttributes::CompareComponent` (Xbox PDB): true when `other` is not a `TESAttributes` or one
/// of its attributes differs from this one's.
pub fn compare_component(e: &mut Engine, this: u32, other: u32) -> bool {
    let other = as_attributes(e, other);
    if other == 0 {
        return true;
    }
    for index in 0..ATTRIBUTE_COUNT {
        if e.mem.u8(at(this, ATTRIBUTES + index)) != e.mem.u8(at(other, ATTRIBUTES + index)) {
            return true;
        }
    }
    false
}

// Typed from 00480100
/// The attributes' record data written: the seven bytes at +4 copied to `buffer`; the size
/// written (7, `009d2510`).
pub fn save_data(e: &mut Engine, this: u32, buffer: u32) -> u32 {
    copy_attributes(e, buffer, at(this, ATTRIBUTES));
    ATTRIBUTE_COUNT
}

// Typed from 004801e0
/// The attributes' size in a save: 0 (the exe's 7 is under a change mask the compiler folded to 0).
pub fn save_size(_e: &mut Engine, _this: u32, _flags: u32) -> u16 {
    0
}

// Typed from 00480210
/// The attributes' save: nothing (under a change mask folded to 0).
pub fn save_game(_e: &mut Engine, _this: u32, _flags: u32) {}

// Typed from 00480240
/// The attributes' load: nothing (under a change mask folded to 0).
pub fn load_game(_e: &mut Engine, _this: u32, _flags: u32, _unused: u32) {}

// Typed from 00480270
/// The attributes' save into a save buffer: with the buffer's attributes change flag, the seven
/// bytes at +4 written.
pub fn save_to_buffer(e: &mut Engine, this: u32, buffer: u32) {
    if buffer_has_attributes(e, buffer) {
        e.call(
            SAVE_BUFFER_WRITE,
            &args![buffer, at(this, ATTRIBUTES), ATTRIBUTE_COUNT, 0u32],
        );
    }
}

// Typed from 004802b0
/// The attributes' load from a load buffer: with the buffer's attributes change flag, the seven
/// bytes at +4 read.
pub fn load_from_buffer(e: &mut Engine, this: u32, buffer: u32) {
    if buffer_has_attributes(e, buffer) {
        e.call(
            LOAD_BUFFER_READ,
            &args![buffer, at(this, ATTRIBUTES), ATTRIBUTE_COUNT],
        );
    }
}

/// `TESModelRDT`'s construction: its base, then its vtable.
fn construct_model_rdt(e: &mut Engine, this: u32) {
    e.call(MODEL_RDT_BASE_CONSTRUCT, &args![this]);
    e.mem.set_u32(this, MODEL_RDT_VTABLE);
}

// Typed from 004802f0
/// `TESBipedModelForm`'s constructor: its vtable (over the base component's), the two model
/// arrays, the icons and message icons (each array by the vector constructor iterator, two
/// elements), the constraint template, then `InitializeDataComponent`. Returns `this`.
pub fn construct_biped_model_form(e: &mut Engine, this: u32) -> u32 {
    e.mem.set_u32(this, BIPED_MODEL_FORM_VTABLE);
    for (offset, size, construct, destruct) in [
        (
            MODELS,
            MODEL_SIZE,
            TEXTURE_SWAP_CONSTRUCT,
            TEXTURE_SWAP_DESTRUCT,
        ),
        (
            WORLD_MODELS,
            MODEL_SIZE,
            TEXTURE_SWAP_CONSTRUCT,
            TEXTURE_SWAP_DESTRUCT,
        ),
        (ICONS, ICON_SIZE, ICON_CONSTRUCT, ICON_DESTRUCT),
        (
            MESSAGE_ICONS,
            MESSAGE_ICON_SIZE,
            MESSAGE_ICON_CONSTRUCT,
            MESSAGE_ICON_DESTRUCT,
        ),
    ] {
        e.call(
            VECTOR_CONSTRUCT,
            &args![at(this, offset), size, 2u32, construct, destruct],
        );
    }
    typed_call!(
        FAMILY,
        0x0048_03e0,
        construct_model_rdt_at(e, at(this, CONSTRAINT_TEMPLATE))
    );
    e.call(BIPED_INITIALIZE_DATA_COMPONENT, &args![this]);
    this
}

// Typed from 004803e0
/// `TESModelRDT`'s constructor: its base (`00488f50`), then its vtable. Returns `this`.
pub fn construct_model_rdt_at(e: &mut Engine, this: u32) -> u32 {
    construct_model_rdt(e, this);
    this
}

// Typed from 00480400
/// `TESBipedModelForm`'s destructor: its vtable, `ClearDataComponent`, then the members in reverse
/// order (the constraint template, the message icons, the icons, both model arrays).
pub fn destruct_biped_model_form(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, BIPED_MODEL_FORM_VTABLE);
    e.call(BIPED_CLEAR_DATA_COMPONENT, &args![this]);
    e.call(MODEL_RDT_DESTRUCT, &args![at(this, CONSTRAINT_TEMPLATE)]);
    for (offset, size, destruct) in [
        (MESSAGE_ICONS, MESSAGE_ICON_SIZE, MESSAGE_ICON_DESTRUCT),
        (ICONS, ICON_SIZE, ICON_DESTRUCT),
        (WORLD_MODELS, MODEL_SIZE, TEXTURE_SWAP_DESTRUCT),
        (MODELS, MODEL_SIZE, TEXTURE_SWAP_DESTRUCT),
    ] {
        e.call(
            VECTOR_DESTRUCT,
            &args![at(this, offset), size, 2u32, destruct],
        );
    }
}
