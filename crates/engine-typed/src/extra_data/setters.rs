//! The second run of single extra data setters, getters and removers (`0041a6a0` to `0041b7c0`):
//! the activator's loop sound and the sound, the ghost, worn and can-not-wear flags, the seed, the
//! package start location, the one-line removers by type, the starting position's rotation and
//! location, the starting world or cell, the action's flags and reference, the health percentage
//! and the object health with their extra data's constructors and destructor.

use super::{
    access, cell_refs, extra_init, first_of_kind, free_when, has_kind, list_add, list_remove,
    new_object, word_at, EXTRA_PAYLOAD, FAMILY,
};
use engine_core::x87::x87_quiet_bits;
use engine_core::{args, typed_call, Engine};

/// The extra data types (`cEtype`) of this file.
const KIND_SCRIPT: u8 = 0x0d;
const KIND_ACTION: u8 = 0x0e;
const KIND_STARTING_POSITION: u8 = 0x0f;
const KIND_ANIM: u8 = 0x10;
const KIND_CONTAINER_CHANGES: u8 = 0x15;
const KIND_WORN: u8 = 0x16;
const KIND_WORN_LEFT: u8 = 0x17;
const KIND_PACKAGE_START_LOCATION: u8 = 0x18;
const KIND_GHOST: u8 = 0x1f;
const KIND_OWNERSHIP: u8 = 0x21;
const KIND_COUNT: u8 = 0x24;
const KIND_HEALTH: u8 = 0x25;
const KIND_LIGHT: u8 = 0x29;
const KIND_LOCK: u8 = 0x2a;
const KIND_TELEPORT: u8 = 0x2b;
const KIND_ANIM_SAVE: u8 = 0x2d;
const KIND_SCALE: u8 = 0x30;
const KIND_SEED: u8 = 0x31;
const KIND_CAN_NOT_WEAR: u8 = 0x3e;
const KIND_POISON: u8 = 0x3f;
const KIND_SPELL_EFFECT_LIGHT: u8 = 0x40;
const KIND_STARTING_WORLD_OR_CELL: u8 = 0x49;
const KIND_SOUND: u8 = 0x4f;
const KIND_OBJECT_HEALTH: u8 = 0x56;
const KIND_HEALTH_PERC: u8 = 0x7a;
const KIND_ACTIVATE_LOOP_SOUND: u8 = 0x87;

/// The constructors (other units') the setters build their extra data with (`thiscall(block, ...)`).
const EXTRA_GHOST_INIT: u32 = 0x0043_22c0;
const EXTRA_WORN_INIT: u32 = 0x0043_22f0;
const EXTRA_WORN_LEFT_INIT: u32 = 0x0043_2320;
const EXTRA_CAN_NOT_WEAR_INIT: u32 = 0x0043_2350;
const EXTRA_SEED_INIT: u32 = 0x0043_25b0;
const EXTRA_PACKAGE_START_LOCATION_INIT: u32 = 0x0043_26c0;
const EXTRA_STARTING_WORLD_OR_CELL_INIT: u32 = 0x0043_08c0;
const EXTRA_SOUND_INIT: u32 = 0x0043_60c0;
const EXTRA_ACTIVATE_LOOP_SOUND_INIT: u32 = 0x0043_6660;
/// `TESObjectCELL::GetWorldSpace` (Xbox PDB), the log (`overridable.tsv`) and the message it is
/// given for a reference with no cell to save in, and the reference's virtual whose result the
/// message prints for `%s`.
const CELL_GET_WORLD_SPACE: u32 = 0x0054_ddd0;
const LOG_MESSAGE: u32 = 0x005b_5e40;
const MESSAGE_NO_PARENT_SAVE_CELL: u32 = 0x0101_5140;
const REFERENCE_NAME_SLOT: u32 = 0x130;
/// The body of `BSExtraData`'s destructor the health percentage's destructor runs (`fastcall`).
const EXTRA_DATA_BASE_DESTRUCTOR: u32 = 0x0043_2f30;
/// The vtables of `ExtraHealthPerc` and `ExtraObjectHealth`.
const VTABLE_EXTRA_HEALTH_PERC: u32 = 0x0101_5178;
const VTABLE_EXTRA_OBJECT_HEALTH: u32 = 0x0101_5184;

// ---- What the setters share ------------------------------------------------------------------

/// The first extra data of `kind` deleted: `RemoveExtra(type)` (`00410140`, the cell references
/// family's).
#[inline]
pub(super) fn remove_kind(e: &mut Engine, list: u32, kind: u8) {
    typed_call!(
        cell_refs::FAMILY,
        0x0041_0140,
        cell_refs::extra_list::remove_extra_of_type(e, list, kind)
    );
}

/// The flag setters: an extra data of `kind` with no payload (0xc bytes, built by `construct`)
/// added when `wanted` and the list has none (its bit, in place), the first one deleted when not
/// `wanted` and the list has one.
fn flag_setter(e: &mut Engine, list: u32, kind: u8, construct: u32, wanted: bool) {
    let present = has_kind(e, list, kind);
    if wanted && !present {
        access::add_new(e, list, 0xc, construct, None);
    } else if !wanted && present {
        remove_kind(e, list, kind);
    }
}

/// The list's starting position (type 0xf): its own, or the one [`access::starting_position`]
/// builds from `position` and adds.
fn starting_position_of(e: &mut Engine, list: u32, position: u32) -> u32 {
    match first_of_kind(e, list, KIND_STARTING_POSITION) {
        0 => typed_call!(
            FAMILY,
            0x0041_8d50,
            access::starting_position(e, list, position)
        ),
        extra => extra,
    }
}

/// The three words at `from` stored at `to`, in order (an `NiPoint3` moved through the integer
/// registers).
fn move_point(e: &mut Engine, from: u32, to: u32) {
    for offset in [0u32, 4, 8] {
        let word = e.mem.u32(from.wrapping_add(offset));
        e.mem.set_u32(to.wrapping_add(offset), word);
    }
}

/// The words `x`, `y`, `z` stored at `to`.
fn store_point(e: &mut Engine, to: u32, x: u32, y: u32, z: u32) {
    e.mem.set_u32(to, x);
    e.mem.set_u32(to.wrapping_add(4), y);
    e.mem.set_u32(to.wrapping_add(8), z);
}

/// The action's flags (`eAction`, the byte at +0xc of the type 0xe extra data), or 1.
#[inline]
fn action_flags_of(e: &mut Engine, list: u32) -> u32 {
    typed_call!(FAMILY, 0x0041_b370, action_flags(e, list))
}

/// The list's action (type 0xe): the one there, or the one [`access::action`] builds and adds.
#[inline]
fn action_or_new(e: &mut Engine, list: u32) -> u32 {
    typed_call!(FAMILY, 0x0041_95b0, access::action(e, list))
}

// ---- The functions --------------------------------------------------------------------------

// Typed from 0041a6a0
/// The type 0x87 extra data's sound handle set (the activator's loop sound, built by `00436660`;
/// [`access::sound_setter`]'s cases).
pub fn set_activate_loop_sound(e: &mut Engine, this: u32, sound: u32) {
    access::sound_setter(
        e,
        this,
        KIND_ACTIVATE_LOOP_SOUND,
        EXTRA_ACTIVATE_LOOP_SOUND_INIT,
        sound,
    );
}

// Typed from 0041a800
/// `ExtraDataList::SetSound` (Xbox PDB): as [`set_activate_loop_sound`] for type 0x4f (built by
/// `004360c0`).
pub fn set_sound(e: &mut Engine, this: u32, sound: u32) {
    access::sound_setter(e, this, KIND_SOUND, EXTRA_SOUND_INIT, sound);
}

// Typed from 0041a960
/// `ExtraDataList::SetGhost` (Xbox PDB): the type 0x1f flag (built by `004322c0`).
pub fn set_ghost(e: &mut Engine, this: u32, ghost: bool) {
    flag_setter(e, this, KIND_GHOST, EXTRA_GHOST_INIT, ghost);
}

// Typed from 0041aa20
/// `ExtraDataList::SetWorn` (Xbox PDB): the type 0x17 flag with `left` (built by `00432320`), else
/// the type 0x16 one (built by `004322f0`).
pub fn set_worn(e: &mut Engine, this: u32, worn: bool, left: bool) {
    if left {
        flag_setter(e, this, KIND_WORN_LEFT, EXTRA_WORN_LEFT_INIT, worn);
    } else {
        flag_setter(e, this, KIND_WORN, EXTRA_WORN_INIT, worn);
    }
}

// Typed from 0041ab70
/// `ExtraDataList::SetCanNotWear` (Xbox PDB): the type 0x3e flag (built by `00432350`).
pub fn set_can_not_wear(e: &mut Engine, this: u32, wanted: bool) {
    flag_setter(e, this, KIND_CAN_NOT_WEAR, EXTRA_CAN_NOT_WEAR_INIT, wanted);
}

// Typed from 0041ac30
/// The seed (a byte at +0xc of the type 0x31 extra data) set: 0xff (no seed) deletes the one there;
/// any other is stored, or one (0x10 bytes, `004325b0`, given the seed zero-extended) added.
pub fn set_seed(e: &mut Engine, this: u32, seed: u8) {
    let extra = first_of_kind(e, this, KIND_SEED);
    if seed == 0xff {
        if extra != 0 {
            list_remove(e, this, extra, true);
        }
    } else if extra == 0 {
        access::add_new(e, this, 0x10, EXTRA_SEED_INIT, Some(u32::from(seed)));
    } else {
        e.mem.set_u8(extra.wrapping_add(EXTRA_PAYLOAD), seed);
    }
}

// Typed from 0041ad00
/// `ExtraDataList::SetPackageStartLocation` (Xbox PDB): without the type 0x18 extra data, one
/// (0x20 bytes) built by `004326c0` from the two forms, the position's address and `z_rot` (through
/// the x87, so a signalling NaN comes out quiet) and added; with one, its location form (+0xc) set
/// to `location_form`, or `cell` when that is null, and the position copied to +0x10 (`z_rot` is
/// not stored then).
pub fn set_package_start_location(
    e: &mut Engine,
    this: u32,
    location_form: u32,
    cell: u32,
    position: u32,
    z_rot: f32,
) {
    let extra = first_of_kind(e, this, KIND_PACKAGE_START_LOCATION);
    if extra == 0 {
        let created = new_object(e, 0x20, |e, block| {
            let z_rot = x87_quiet_bits(z_rot.to_bits());
            e.call(
                EXTRA_PACKAGE_START_LOCATION_INIT,
                &args![block, location_form, cell, position, z_rot],
            )
            .u32()
        });
        list_add(e, this, created);
        return;
    }
    let form = if location_form != 0 {
        location_form
    } else {
        cell
    };
    e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), form);
    move_point(e, position, extra.wrapping_add(0x10));
}

// Typed from 0041adf0
/// `ExtraDataList::RemoveAnimPtr` (Xbox PDB): the type 0x10 extra data deleted.
pub fn remove_anim_ptr(e: &mut Engine, this: u32) {
    remove_kind(e, this, KIND_ANIM);
}

// Typed from 0041ae10
/// `ExtraDataList::RemoveAnimSave` (Xbox PDB): the type 0x2d extra data deleted.
pub fn remove_anim_save(e: &mut Engine, this: u32) {
    remove_kind(e, this, KIND_ANIM_SAVE);
}

// Typed from 0041ae30
/// The type 0x29 extra data (the light) deleted.
pub fn remove_light(e: &mut Engine, this: u32) {
    remove_kind(e, this, KIND_LIGHT);
}

// Typed from 0041ae50
/// The type 0x40 extra data (the spell effect's light) deleted.
pub fn remove_spell_effect_light(e: &mut Engine, this: u32) {
    remove_kind(e, this, KIND_SPELL_EFFECT_LIGHT);
}

// Typed from 0041ae70
/// `ExtraDataList::RemoveLockPtr` (Xbox PDB): the type 0x2a extra data deleted.
pub fn remove_lock_ptr(e: &mut Engine, this: u32) {
    remove_kind(e, this, KIND_LOCK);
}

// Typed from 0041ae90
/// `ExtraDataList::RemoveTeleportPtr` (Xbox PDB): the type 0x2b extra data deleted.
pub fn remove_teleport_ptr(e: &mut Engine, this: u32) {
    remove_kind(e, this, KIND_TELEPORT);
}

// Typed from 0041aeb0
/// The type 0x15 extra data (the container changes) deleted.
pub fn remove_container_changes(e: &mut Engine, this: u32) {
    remove_kind(e, this, KIND_CONTAINER_CHANGES);
}

// Typed from 0041aed0
/// `ExtraDataList::RemoveOwnership` (Xbox PDB): the type 0x21 extra data deleted.
pub fn remove_ownership(e: &mut Engine, this: u32) {
    remove_kind(e, this, KIND_OWNERSHIP);
}

// Typed from 0041aef0
/// `ExtraDataList::RemoveHealth` (Xbox PDB): the type 0x25 extra data deleted.
pub fn remove_health(e: &mut Engine, this: u32) {
    remove_kind(e, this, KIND_HEALTH);
}

// Typed from 0041af10
/// `ExtraDataList::RemoveCount` (Xbox PDB): the type 0x24 extra data deleted.
pub fn remove_count(e: &mut Engine, this: u32) {
    remove_kind(e, this, KIND_COUNT);
}

// Typed from 0041af30
/// The type 0x3f extra data (the poison) deleted.
pub fn remove_poison(e: &mut Engine, this: u32) {
    remove_kind(e, this, KIND_POISON);
}

// Typed from 0041af50
/// The type 0xd extra data (the script) deleted.
pub fn remove_script(e: &mut Engine, this: u32) {
    remove_kind(e, this, KIND_SCRIPT);
}

// Typed from 0041afa0
/// `ExtraDataList::ClearScriptLocals` (Xbox PDB): the script's locals (+0x10 of the type 0xd extra
/// data) set to null, not freed; nothing without one.
pub fn clear_script_locals(e: &mut Engine, this: u32) {
    let extra = first_of_kind(e, this, KIND_SCRIPT);
    if extra != 0 {
        e.mem.set_u32(extra.wrapping_add(0x10), 0);
    }
}

// Typed from 0041afd0
/// The type 0x30 extra data (the scale) deleted.
pub fn remove_scale(e: &mut Engine, this: u32) {
    remove_kind(e, this, KIND_SCALE);
}

// Typed from 0041aff0
/// The type 0x1f extra data (the ghost) deleted.
pub fn remove_ghost(e: &mut Engine, this: u32) {
    remove_kind(e, this, KIND_GHOST);
}

// Typed from 0041b010
/// The worn flag of one hand deleted: type 0x17 with `left`, else 0x16.
pub fn remove_worn(e: &mut Engine, this: u32, left: bool) {
    remove_kind(e, this, if left { KIND_WORN_LEFT } else { KIND_WORN });
}

// Typed from 0041b040
/// `ExtraDataList::RemoveCannotWearExtra` (Xbox PDB): the type 0x3e extra data deleted.
pub fn remove_can_not_wear(e: &mut Engine, this: u32) {
    remove_kind(e, this, KIND_CAN_NOT_WEAR);
}

// Typed from 0041b060
/// The type 0x18 extra data (the package start location) deleted.
pub fn remove_package_start_location(e: &mut Engine, this: u32) {
    remove_kind(e, this, KIND_PACKAGE_START_LOCATION);
}

// Typed from 0041b080
/// The starting position's rotation (+0x18) copied to `out` (the position built from `position`
/// and added when the list has none). Answers `out`.
pub fn starting_rotation(e: &mut Engine, this: u32, out: u32, position: u32) -> u32 {
    let extra = starting_position_of(e, this, position);
    move_point(e, extra.wrapping_add(0x18), out);
    out
}

// Typed from 0041b0d0
/// As [`starting_rotation`] for the location (+0xc). Answers `out`.
pub fn starting_location(e: &mut Engine, this: u32, out: u32, position: u32) -> u32 {
    let extra = starting_position_of(e, this, position);
    move_point(e, extra.wrapping_add(EXTRA_PAYLOAD), out);
    out
}

// Typed from 0041b120
/// The starting position's rotation (+0x18) set to the words `x`, `y`, `z` (the position built from
/// `position` when the list has none), and the same words stored at `out`. Answers `out`.
pub fn set_starting_rotation(
    e: &mut Engine,
    this: u32,
    out: u32,
    position: u32,
    x: u32,
    y: u32,
    z: u32,
) -> u32 {
    let extra = starting_position_of(e, this, position);
    store_point(e, extra.wrapping_add(0x18), x, y, z);
    store_point(e, out, x, y, z);
    out
}

// Typed from 0041b180
/// As [`set_starting_rotation`] for the location (+0xc). Answers `out`.
pub fn set_starting_location(
    e: &mut Engine,
    this: u32,
    out: u32,
    position: u32,
    x: u32,
    y: u32,
    z: u32,
) -> u32 {
    let extra = starting_position_of(e, this, position);
    store_point(e, extra.wrapping_add(EXTRA_PAYLOAD), x, y, z);
    store_point(e, out, x, y, z);
    out
}

// Typed from 0041b1e0
/// The starting world or cell (+0xc of the type 0x49 extra data) set: a null `form` deletes the
/// extra data; any other is stored in it, built (0x10 bytes, `004308c0`) and added when the list has
/// none (a failed allocation's null one too, the store then going to null plus 0xc).
pub fn set_starting_world_or_cell(e: &mut Engine, this: u32, form: u32) {
    if form == 0 {
        typed_call!(FAMILY, 0x0041_b350, remove_starting_world_or_cell(e, this));
        return;
    }
    let mut extra = first_of_kind(e, this, KIND_STARTING_WORLD_OR_CELL);
    if extra == 0 {
        extra = access::add_new(e, this, 0x10, EXTRA_STARTING_WORLD_OR_CELL_INIT, None);
    }
    e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), form);
}

// Typed from 0041b2a0
/// `ExtraDataList::SetStartingWorldOrCellForRef` (Xbox PDB): the cell the reference saves in (its
/// `TESChildCell` at +0x18, slot 0, a call through the slot) gives the starting world or cell: the
/// cell's world space when it has one, else the cell ([`set_starting_world_or_cell`]). Without a
/// cell the reference's name (its slot 0x130) and form id are logged with the message at
/// `01015140`.
pub fn set_starting_world_or_cell_for_ref(e: &mut Engine, this: u32, reference: u32) {
    let child_cell = reference.wrapping_add(0x18);
    let cell = e.vcall(child_cell, 0, &args![]).u32();
    if cell != 0 {
        let world = e.call(CELL_GET_WORLD_SPACE, &args![cell]).u32();
        let form = if world != 0 { world } else { cell };
        typed_call!(
            FAMILY,
            0x0041_b1e0,
            set_starting_world_or_cell(e, this, form)
        );
        return;
    }
    // `TESForm::GetFormID` (`0084e3a0`, the word at +0xc), in place.
    let form_id = word_at(e, reference, 0xc);
    let name = e.vcall(reference, REFERENCE_NAME_SLOT, &args![]).u32();
    e.call(
        LOG_MESSAGE,
        &args![MESSAGE_NO_PARENT_SAVE_CELL, name, form_id],
    );
}

// Typed from 0041b320
/// `ExtraDataList::GetStartingWorldOrCell` (Xbox PDB): the word at +0xc of the type 0x49 extra
/// data, or 0.
pub fn starting_world_or_cell(e: &mut Engine, this: u32) -> u32 {
    access::word_or(e, this, KIND_STARTING_WORLD_OR_CELL, 0)
}

// Typed from 0041b350
/// The type 0x49 extra data (the starting world or cell) deleted.
pub fn remove_starting_world_or_cell(e: &mut Engine, this: u32) {
    remove_kind(e, this, KIND_STARTING_WORLD_OR_CELL);
}

// Typed from 0041b370
/// The action's flags (`eAction`, the byte at +0xc of the type 0xe extra data, zero-extended), or 1.
pub fn action_flags(e: &mut Engine, this: u32) -> u32 {
    match first_of_kind(e, this, KIND_ACTION) {
        0 => 1,
        extra => u32::from(e.mem.u8(extra.wrapping_add(EXTRA_PAYLOAD))),
    }
}

// Typed from 0041b3a0
/// Whether the action's flags ([`action_flags`]) have a bit of `mask`.
pub fn has_action_flag(e: &mut Engine, this: u32, mask: u32) -> bool {
    action_flags_of(e, this) & mask != 0
}

// Typed from 0041b3d0
/// The action's flags set (the low byte of `action` at +0xc of the type 0xe extra data). Without
/// one, an `action` other than 1 gets one from [`access::action`]; with one, 1 deletes it when its
/// reference (+0x10) is null.
pub fn set_action_flags(e: &mut Engine, this: u32, action: u32) {
    let mut extra = first_of_kind(e, this, KIND_ACTION);
    if extra == 0 {
        if action != 1 {
            extra = action_or_new(e, this);
        }
    } else if action == 1 && word_at(e, extra, 0x10) == 0 {
        list_remove(e, this, extra, true);
        extra = 0;
    }
    if extra != 0 {
        e.mem
            .set_u8(extra.wrapping_add(EXTRA_PAYLOAD), action.to_le_bytes()[0]);
    }
}

// Typed from 0041b440
/// The bits of `mask` set in the action's flags.
pub fn add_action_flags(e: &mut Engine, this: u32, mask: u32) {
    let action = action_flags_of(e, this) | mask;
    typed_call!(FAMILY, 0x0041_b3d0, set_action_flags(e, this, action));
}

// Typed from 0041b470
/// The bits of `mask` cleared in the action's flags.
pub fn remove_action_flags(e: &mut Engine, this: u32, mask: u32) {
    let action = action_flags_of(e, this) & !mask;
    typed_call!(FAMILY, 0x0041_b3d0, set_action_flags(e, this, action));
}

// Typed from 0041b4b0
/// The action's reference (+0x10 of the type 0xe extra data) set. Without one, a non-null
/// `reference` gets one from [`access::action`]; with one, a null `reference` deletes it when its
/// flags byte is 1.
pub fn set_action_ref(e: &mut Engine, this: u32, reference: u32) {
    let mut extra = first_of_kind(e, this, KIND_ACTION);
    if extra == 0 {
        if reference != 0 {
            extra = action_or_new(e, this);
        }
    } else if e.mem.u8(extra.wrapping_add(EXTRA_PAYLOAD)) == 1 && reference == 0 {
        list_remove(e, this, extra, true);
        extra = 0;
    }
    if extra != 0 {
        e.mem.set_u32(extra.wrapping_add(0x10), reference);
    }
}

// Typed from 0041b520
/// `ExtraDataList::GetActionRef` (Xbox PDB): the action's reference (+0x10 of the type 0xe extra
/// data), or 0.
pub fn action_ref(e: &mut Engine, this: u32) -> u32 {
    match first_of_kind(e, this, KIND_ACTION) {
        0 => 0,
        extra => word_at(e, extra, 0x10),
    }
}

// Typed from 0041b550
/// `ExtraDataList::GetHealthPerc` (Xbox PDB): the `float` at +0xc of the type 0x7a extra data (through
/// the x87's load), or 1.0 (`FLD1`).
pub fn health_perc(e: &mut Engine, this: u32) -> f32 {
    access::float_or(e, this, KIND_HEALTH_PERC, 1.0f32.to_bits())
}

// Typed from 0041b580
/// The health percentage set: a value equal to the `double` 1.0 at `01012070` (`FCOMP`, so not a
/// NaN) deletes the type 0x7a extra data (`RemoveExtra` is called with a null one too); any other is
/// stored (through the x87), in a new one ([`health_perc_new`], 0x10 bytes, added) when the list
/// has none.
pub fn set_health_perc(e: &mut Engine, this: u32, health_perc: f32) {
    let mut extra = first_of_kind(e, this, KIND_HEALTH_PERC);
    if f64::from(health_perc) == e.mem.f64(access::ONE_DOUBLE) {
        list_remove(e, this, extra, true);
        return;
    }
    if extra == 0 {
        extra = new_object(e, 0x10, |e, block| {
            typed_call!(FAMILY, 0x0041_b650, health_perc_new(e, block))
        });
        list_add(e, this, extra);
    }
    let bits = x87_quiet_bits(health_perc.to_bits());
    e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), bits);
}

// Typed from 0041b650
/// `ExtraHealthPerc`'s constructor (type 0x7a): the base with the type (`0040ec80`, in place), its
/// vtable, the percentage 0.0. Answers `this`.
pub fn health_perc_new(e: &mut Engine, this: u32) -> u32 {
    extra_init(e, this, KIND_HEALTH_PERC);
    e.mem.set_u32(this, VTABLE_EXTRA_HEALTH_PERC);
    e.mem.set_u32(this.wrapping_add(EXTRA_PAYLOAD), 0);
    this
}

// Typed from 0041b680
/// `ExtraHealthPerc`'s scalar deleting destructor: the base's destructor (`00432f30`), then
/// `operator delete` with bit 0 of `flags`. Answers `this`.
pub fn health_perc_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    e.call(EXTRA_DATA_BASE_DESTRUCTOR, &args![this]);
    free_when(e, this, flags)
}

// Typed from 0041b6b0
/// `ExtraDataList::GetObjectHealth` (Xbox PDB): the `float` at +0xc of the type 0x56 extra data, or
/// the -1.0 at `01012054` (each through the x87's load).
pub fn object_health(e: &mut Engine, this: u32) -> f32 {
    let default = e.mem.u32(access::MINUS_ONE);
    access::float_or(e, this, KIND_OBJECT_HEALTH, default)
}

// Typed from 0041b6e0
/// The object health stored (through the x87) in the type 0x56 extra data, or a new one built from
/// it ([`object_health_new`], 0x10 bytes) and added.
pub fn set_object_health(e: &mut Engine, this: u32, health: f32) {
    let extra = first_of_kind(e, this, KIND_OBJECT_HEALTH);
    let bits = x87_quiet_bits(health.to_bits());
    if extra == 0 {
        let created = new_object(e, 0x10, |e, block| {
            typed_call!(
                FAMILY,
                0x0041_b790,
                object_health_new(e, block, f32::from_bits(bits))
            )
        });
        list_add(e, this, created);
    } else {
        e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), bits);
    }
}

// Typed from 0041b790
/// `ExtraObjectHealth`'s constructor (type 0x56): the base with the type (in place), its vtable, the
/// health (through the x87). Answers `this`.
pub fn object_health_new(e: &mut Engine, this: u32, health: f32) -> u32 {
    extra_init(e, this, KIND_OBJECT_HEALTH);
    e.mem.set_u32(this, VTABLE_EXTRA_OBJECT_HEALTH);
    let bits = x87_quiet_bits(health.to_bits());
    e.mem.set_u32(this.wrapping_add(EXTRA_PAYLOAD), bits);
    this
}

// Typed from 0041b7c0
/// The type 0x56 extra data (the object health) deleted.
pub fn remove_object_health(e: &mut Engine, this: u32) {
    remove_kind(e, this, KIND_OBJECT_HEALTH);
}
