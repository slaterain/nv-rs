//! The cell's extra data and a few of a reference's (`0041b7e0` to `0041c900`): the cell's 3D node,
//! its Havok world and MOPP, region list, music type, acoustic space, climate, image space, impact
//! swap and canopy shadow mask, a terminal's state and lock override with their extra data's
//! constructor, the editor id with its constructor and destructors, and the reference pointer.
//!
//! The smart pointers (`NiPointer`) the cell's extra data hold are read in place (`00559450`) and
//! assigned by address (`0066b0d0`, which counts the references); their compare with a pointer
//! (`00822510`) is done in place, and so is a getter's temporary made from null when the list has
//! no such extra data (`00633c90`, its read and `0045cec0`: 0).

use super::{
    access, cell_refs, extra_init, first_of_kind, free_when, list_add, list_remove, new_object,
    read_word, word_at, EXTRA_PAYLOAD, FAMILY,
};
use engine_core::{args, typed_call, Engine};

/// The extra data types (`cEtype`) of this file.
const KIND_HAVOK: u8 = 0x01;
const KIND_CELL_3D: u8 = 0x02;
const KIND_REGION_LIST: u8 = 0x04;
const KIND_EDITOR_ID: u8 = 0x06;
const KIND_CELL_MUSIC_TYPE: u8 = 0x07;
const KIND_CLIMATE: u8 = 0x08;
const KIND_CANOPY_SHADOW_MASK: u8 = 0x0a;
const KIND_REFERENCE_POINTER: u8 = 0x1c;
const KIND_TERMINAL_STATE: u8 = 0x50;
const KIND_IMAGE_SPACE: u8 = 0x59;
const KIND_ACOUSTIC_SPACE: u8 = 0x81;
const KIND_IMPACT_SWAP: u8 = 0x8c;

/// The constructors (other units') the setters build their extra data with (`thiscall(block, ...)`).
const EXTRA_HAVOK_INIT: u32 = 0x0040_eda0;
const EXTRA_CELL_3D_INIT: u32 = 0x0040_ec00;
const EXTRA_REGION_LIST_INIT: u32 = 0x0040_ef00;
const EXTRA_CELL_MUSIC_TYPE_INIT: u32 = 0x0040_ef30;
const EXTRA_ACOUSTIC_SPACE_INIT: u32 = 0x0040_efd0;
const EXTRA_CLIMATE_INIT: u32 = 0x0040_f070;
const EXTRA_IMAGE_SPACE_INIT: u32 = 0x0040_f110;
const EXTRA_IMPACT_SWAP_INIT: u32 = 0x0040_f1b0;
const EXTRA_CANOPY_SHADOW_MASK_INIT: u32 = 0x0040_f440;
const EXTRA_REFERENCE_POINTER_INIT: u32 = 0x0043_27e0;
/// `NiPointer`'s assignment (`thiscall(at, pointer)`: the new one counted, the old one released).
const NI_POINTER_ASSIGN: u32 = 0x0066_b0d0;
/// `BSStringT<char>`'s constructor, destructor and set from a C string (`thiscall(string)`,
/// `thiscall(string, text, 0)`).
const BS_STRING_INIT: u32 = 0x0040_37b0;
const BS_STRING_DROP: u32 = 0x0040_37d0;
const BS_STRING_SET: u32 = 0x0040_37f0;
/// The vtables of `ExtraTerminalState` and `ExtraEditorID`.
const VTABLE_EXTRA_TERMINAL_STATE: u32 = 0x0101_5190;
const VTABLE_EXTRA_EDITOR_ID: u32 = 0x0101_519c;
/// `TESObjectREFR::GetRefPersists` (Xbox PDB).
const GET_REF_PERSISTS: u32 = 0x0056_53d0;
/// A terminal state's two bytes: the state (+0xc) and the lock override (+0xd).
const TERMINAL_STATE: u32 = 0x0c;
const TERMINAL_LOCK_OVERRIDE: u32 = 0x0d;

// ---- What the setters share ------------------------------------------------------------------

/// The pointer the `NiPointer` at `offset` of the first extra data of `kind` holds, or 0 (the exe's
/// temporary made from null, read and released, in place).
fn held_pointer(e: &mut Engine, list: u32, kind: u8, offset: u32) -> u32 {
    match first_of_kind(e, list, kind) {
        0 => 0,
        extra => read_word(e, extra.wrapping_add(offset)),
    }
}

/// `NiPointer`'s assignment of `pointer` to the one at `at`.
#[inline]
fn assign_pointer(e: &mut Engine, at: u32, pointer: u32) {
    e.call(NI_POINTER_ASSIGN, &args![at, pointer]);
}

/// A terminal state's byte at `offset` set: a nonzero `value` is stored, in a new extra data
/// ([`terminal_state_new`], 0x10 bytes, added) when the list has none; zero is stored in the one
/// there, which is deleted when it then holds nothing ([`terminal_state_empty`]).
fn set_terminal_byte(e: &mut Engine, list: u32, offset: u32, value: u8) {
    let mut extra = first_of_kind(e, list, KIND_TERMINAL_STATE);
    if value == 0 {
        if extra != 0 {
            e.mem.set_u8(extra.wrapping_add(offset), 0);
            if typed_call!(FAMILY, 0x0041_bf20, terminal_state_empty(e, extra)) {
                list_remove(e, list, extra, true);
            }
        }
        return;
    }
    if extra == 0 {
        extra = new_object(e, 0x10, |e, block| {
            typed_call!(FAMILY, 0x0041_bef0, terminal_state_new(e, block))
        });
        list_add(e, list, extra);
    }
    e.mem.set_u8(extra.wrapping_add(offset), value);
}

/// A terminal state's byte at `offset`, or `default` without one.
fn terminal_byte(e: &mut Engine, list: u32, offset: u32, default: u8) -> u8 {
    match first_of_kind(e, list, KIND_TERMINAL_STATE) {
        0 => default,
        extra => e.mem.u8(extra.wrapping_add(offset)),
    }
}

// ---- The functions --------------------------------------------------------------------------

// Typed from 0041b7e0
/// The cell's 3D node (the `NiPointer` at +0xc of the type 0x2 extra data) set. A null `node`
/// deletes the extra data when its pointer is already null, assigns null to it otherwise, and does
/// nothing without one; any other is assigned, to a new one (0x10 bytes, `0040ec00`, added) when
/// the list has none.
pub fn set_cell_3d(e: &mut Engine, this: u32, node: u32) {
    let mut extra = first_of_kind(e, this, KIND_CELL_3D);
    if node == 0 {
        if extra != 0 && word_at(e, extra, EXTRA_PAYLOAD) == node {
            list_remove(e, this, extra, true);
            extra = 0;
        }
        if extra == 0 {
            return;
        }
    }
    if extra == 0 {
        extra = access::add_new(e, this, 0x10, EXTRA_CELL_3D_INIT, None);
    }
    assign_pointer(e, extra.wrapping_add(EXTRA_PAYLOAD), node);
}

// Typed from 0041b8d0
/// The cell's Havok world (the `NiPointer` at +0xc of the type 0x1 extra data) set: a null `world`
/// deletes the extra data; any other is assigned, or one (0x14 bytes, `0040eda0`) built from it and
/// added.
pub fn set_havok_world(e: &mut Engine, this: u32, world: u32) {
    let extra = first_of_kind(e, this, KIND_HAVOK);
    if world == 0 {
        if extra != 0 {
            list_remove(e, this, extra, true);
        }
    } else if extra == 0 {
        access::add_new(e, this, 0x14, EXTRA_HAVOK_INIT, Some(world));
    } else {
        assign_pointer(e, extra.wrapping_add(EXTRA_PAYLOAD), world);
    }
}

// Typed from 0041b9a0
/// The cell's Havok world (+0xc of the type 0x1 extra data), or 0.
pub fn havok_world(e: &mut Engine, this: u32) -> u32 {
    held_pointer(e, this, KIND_HAVOK, EXTRA_PAYLOAD)
}

// Typed from 0041ba60
/// The cell's MOPP (the `NiPointer` at +0x10 of the type 0x1 extra data) set: a nonzero one builds
/// the extra data from a null world (`0040eda0`) when the list has none; then it is assigned in
/// every case (to null plus 0x10 without an extra data, as the exe does).
pub fn set_cell_mopp(e: &mut Engine, this: u32, mopp: u32) {
    let mut extra = first_of_kind(e, this, KIND_HAVOK);
    if mopp != 0 && extra == 0 {
        extra = access::add_new(e, this, 0x14, EXTRA_HAVOK_INIT, Some(0));
    }
    assign_pointer(e, extra.wrapping_add(0x10), mopp);
}

// Typed from 0041bb10
/// The cell's MOPP (+0x10 of the type 0x1 extra data), or 0.
pub fn cell_mopp(e: &mut Engine, this: u32) -> u32 {
    held_pointer(e, this, KIND_HAVOK, 0x10)
}

// Typed from 0041bbd0
/// `ExtraDataList::SetRegionList` (Xbox PDB): a null `regions` deletes the type 0x4 extra data; any
/// other is stored in it, an old list other than it deleted first (through its slot 0 with 1), or
/// one (0x10 bytes, `0040ef00`) built from it and added.
pub fn set_region_list(e: &mut Engine, this: u32, regions: u32) {
    let extra = first_of_kind(e, this, KIND_REGION_LIST);
    if regions == 0 {
        if extra != 0 {
            list_remove(e, this, extra, true);
        }
    } else if extra == 0 {
        access::add_new(e, this, 0x10, EXTRA_REGION_LIST_INIT, Some(regions));
    } else {
        let old = word_at(e, extra, EXTRA_PAYLOAD);
        if old != 0 && old != regions {
            e.vcall(old, 0, &args![1u32]);
        }
        e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), regions);
    }
}

// Typed from 0041bce0
/// `ExtraDataList::GetRegionList` (Xbox PDB): the word at +0xc of the type 0x4 extra data, or 0.
pub fn region_list(e: &mut Engine, this: u32) -> u32 {
    access::word_or(e, this, KIND_REGION_LIST, 0)
}

// Typed from 0041bd10
/// The cell's music type (+0xc of the type 0x7 extra data) set; a null one deletes it (built by
/// `0040ef30`).
pub fn set_music_type(e: &mut Engine, this: u32, music_type: u32) {
    access::set_or_remove(
        e,
        this,
        KIND_CELL_MUSIC_TYPE,
        EXTRA_CELL_MUSIC_TYPE_INIT,
        music_type,
        0,
    );
}

// Typed from 0041bde0
/// The cell's music type (+0xc of the type 0x7 extra data), or 0.
pub fn music_type(e: &mut Engine, this: u32) -> u32 {
    access::word_or(e, this, KIND_CELL_MUSIC_TYPE, 0)
}

// Typed from 0041be10
/// `ExtraDataList::SetTerminalState` (Xbox PDB): the state byte (+0xc of the type 0x50 extra data)
/// set, as `set_terminal_byte` has it.
pub fn set_terminal_state(e: &mut Engine, this: u32, state: u8) {
    set_terminal_byte(e, this, TERMINAL_STATE, state);
}

// Typed from 0041bef0
/// `ExtraTerminalState`'s constructor (type 0x50): the base with the type (`0040ec80`, in place),
/// its vtable, the state 0 and the lock override 0xff. Answers `this`.
pub fn terminal_state_new(e: &mut Engine, this: u32) -> u32 {
    extra_init(e, this, KIND_TERMINAL_STATE);
    e.mem.set_u32(this, VTABLE_EXTRA_TERMINAL_STATE);
    e.mem.set_u8(this.wrapping_add(TERMINAL_STATE), 0);
    e.mem
        .set_u8(this.wrapping_add(TERMINAL_LOCK_OVERRIDE), 0xff);
    this
}

// Typed from 0041bf20
/// Whether a terminal state holds nothing: the state 0 and the lock override -1.
pub fn terminal_state_empty(e: &mut Engine, this: u32) -> bool {
    e.mem.u8(this.wrapping_add(TERMINAL_STATE)) == 0
        && e.mem.u8(this.wrapping_add(TERMINAL_LOCK_OVERRIDE)) == 0xff
}

// Typed from 0041bf50
/// `ExtraDataList::GetTerminalState` (Xbox PDB): the state byte of the type 0x50 extra data, or 0.
pub fn terminal_state(e: &mut Engine, this: u32) -> u8 {
    terminal_byte(e, this, TERMINAL_STATE, 0)
}

// Typed from 0041bf80
/// The terminal's lock override (+0xd of the type 0x50 extra data) set, as `set_terminal_byte` has
/// it.
pub fn set_lock_override(e: &mut Engine, this: u32, lock_override: u8) {
    set_terminal_byte(e, this, TERMINAL_LOCK_OVERRIDE, lock_override);
}

// Typed from 0041c060
/// The terminal's lock override (+0xd of the type 0x50 extra data), or 0xff.
pub fn lock_override(e: &mut Engine, this: u32) -> u8 {
    terminal_byte(e, this, TERMINAL_LOCK_OVERRIDE, 0xff)
}

// Typed from 0041c090
/// The cell's acoustic space (+0xc of the type 0x81 extra data) set; a null one deletes it (built
/// by `0040efd0`).
pub fn set_acoustic_space(e: &mut Engine, this: u32, space: u32) {
    access::set_or_remove(
        e,
        this,
        KIND_ACOUSTIC_SPACE,
        EXTRA_ACOUSTIC_SPACE_INIT,
        space,
        0,
    );
}

// Typed from 0041c160
/// `ExtraDataList::GetAcousticSpace` (Xbox PDB): the word at +0xc of the type 0x81 extra data, or 0.
pub fn acoustic_space(e: &mut Engine, this: u32) -> u32 {
    access::word_or(e, this, KIND_ACOUSTIC_SPACE, 0)
}

// Typed from 0041c190
/// The cell's climate (+0xc of the type 0x8 extra data) set; a null one deletes it (built by
/// `0040f070`).
pub fn set_climate(e: &mut Engine, this: u32, climate: u32) {
    access::set_or_remove(e, this, KIND_CLIMATE, EXTRA_CLIMATE_INIT, climate, 0);
}

// Typed from 0041c260
/// The cell's climate (+0xc of the type 0x8 extra data), or 0.
pub fn climate(e: &mut Engine, this: u32) -> u32 {
    access::word_or(e, this, KIND_CLIMATE, 0)
}

// Typed from 0041c290
/// The cell's image space (+0xc of the type 0x59 extra data) set; a null one deletes it (built by
/// `0040f110`).
pub fn set_image_space(e: &mut Engine, this: u32, image_space: u32) {
    access::set_or_remove(
        e,
        this,
        KIND_IMAGE_SPACE,
        EXTRA_IMAGE_SPACE_INIT,
        image_space,
        0,
    );
}

// Typed from 0041c360
/// The cell's image space (+0xc of the type 0x59 extra data), or 0.
pub fn image_space(e: &mut Engine, this: u32) -> u32 {
    access::word_or(e, this, KIND_IMAGE_SPACE, 0)
}

// Typed from 0041c390
/// The cell's impact swap (+0xc of the type 0x8c extra data) set: a null one deletes it; any other
/// is stored, or in a new one (0x10 bytes, `0040f1b0`, which takes nothing: the word is stored
/// after it, to null plus 0xc when the allocation failed) that is then added.
pub fn set_impact_swap(e: &mut Engine, this: u32, impact_swap: u32) {
    let extra = first_of_kind(e, this, KIND_IMPACT_SWAP);
    if impact_swap == 0 {
        if extra != 0 {
            list_remove(e, this, extra, true);
        }
    } else if extra == 0 {
        let created = new_object(e, 0x10, |e, block| {
            e.call(EXTRA_IMPACT_SWAP_INIT, &args![block]).u32()
        });
        e.mem
            .set_u32(created.wrapping_add(EXTRA_PAYLOAD), impact_swap);
        list_add(e, this, created);
    } else {
        e.mem
            .set_u32(extra.wrapping_add(EXTRA_PAYLOAD), impact_swap);
    }
}

// Typed from 0041c460
/// The cell's impact swap (+0xc of the type 0x8c extra data), or 0.
pub fn impact_swap(e: &mut Engine, this: u32) -> u32 {
    access::word_or(e, this, KIND_IMPACT_SWAP, 0)
}

// Typed from 0041c490
/// `ExtraDataList::SetCanopyShadowMask` (Xbox PDB, type 0xa, 0x1c bytes: the state +0xc, the mask's
/// `NiPointer` +0x10, the locked rectangle +0x14). A zero `state` deletes it and stores nothing in
/// `rect_out`; otherwise the state is stored and the mask assigned, or one built from both
/// (`0040f440`) and added, and `rect_out` given the address of its rectangle.
pub fn set_canopy_shadow_mask(e: &mut Engine, this: u32, state: u32, mask: u32, rect_out: u32) {
    let mut extra = first_of_kind(e, this, KIND_CANOPY_SHADOW_MASK);
    if state == 0 {
        if extra != 0 {
            list_remove(e, this, extra, true);
        }
        return;
    }
    if extra == 0 {
        extra = new_object(e, 0x1c, |e, block| {
            e.call(EXTRA_CANOPY_SHADOW_MASK_INIT, &args![block, state, mask])
                .u32()
        });
        list_add(e, this, extra);
    } else {
        e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), state);
        assign_pointer(e, extra.wrapping_add(0x10), mask);
    }
    e.mem.set_u32(rect_out, extra.wrapping_add(0x14));
}

// Typed from 0041c580
/// The canopy shadow mask read: 0 stored in `mask_out` and `rect_out`, then, with the type 0xa
/// extra data, its mask (read in place) and its rectangle's address there. Answers its state, or 0.
pub fn canopy_shadow_mask(e: &mut Engine, this: u32, mask_out: u32, rect_out: u32) -> u32 {
    let extra = first_of_kind(e, this, KIND_CANOPY_SHADOW_MASK);
    e.mem.set_u32(mask_out, 0);
    e.mem.set_u32(rect_out, 0);
    if extra == 0 {
        return 0;
    }
    let mask = read_word(e, extra.wrapping_add(0x10));
    e.mem.set_u32(mask_out, mask);
    e.mem.set_u32(rect_out, extra.wrapping_add(0x14));
    word_at(e, extra, EXTRA_PAYLOAD)
}

// Typed from 0041c5e0
/// `ExtraEditorID`'s constructor (type 0x6, 0x14 bytes): the base with the type (in place), its
/// vtable, then the editor id's string (+0xc) made empty (`004037b0`) and set from `editor_id`
/// (`004037f0`). Answers `this`.
pub fn editor_id_new(e: &mut Engine, this: u32, editor_id: u32) -> u32 {
    extra_init(e, this, KIND_EDITOR_ID);
    e.mem.set_u32(this, VTABLE_EXTRA_EDITOR_ID);
    let string = this.wrapping_add(EXTRA_PAYLOAD);
    e.call(BS_STRING_INIT, &args![string]);
    e.call(BS_STRING_SET, &args![string, editor_id, 0u32]);
    this
}

// Typed from 0041c660
/// `ExtraEditorID`'s scalar deleting destructor: [`editor_id_drop`], then `operator delete` with bit
/// 0 of `flags`. Answers `this`.
pub fn editor_id_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0041_c690, editor_id_drop(e, this));
    free_when(e, this, flags)
}

// Typed from 0041c690
/// `ExtraEditorID`'s destructor: the string (+0xc, `004037d0`), then the base's (`0040ecb0`, the cell
/// references family's).
pub fn editor_id_drop(e: &mut Engine, this: u32) {
    e.call(BS_STRING_DROP, &args![this.wrapping_add(EXTRA_PAYLOAD)]);
    typed_call!(
        cell_refs::FAMILY,
        0x0040_ecb0,
        cell_refs::extra_list::extra_data_drop(e, this)
    );
}

// Typed from 0041c6f0
/// The editor id (the string at +0xc of the type 0x6 extra data) set: a null `editor_id` deletes the
/// extra data; any other is set in it (`004037f0`), or one ([`editor_id_new`], 0x14 bytes) built
/// from it and added.
pub fn set_editor_id(e: &mut Engine, this: u32, editor_id: u32) {
    let extra = first_of_kind(e, this, KIND_EDITOR_ID);
    if editor_id == 0 {
        if extra != 0 {
            list_remove(e, this, extra, true);
        }
    } else if extra == 0 {
        let created = new_object(e, 0x14, |e, block| {
            typed_call!(FAMILY, 0x0041_c5e0, editor_id_new(e, block, editor_id))
        });
        list_add(e, this, created);
    } else {
        e.call(
            BS_STRING_SET,
            &args![extra.wrapping_add(EXTRA_PAYLOAD), editor_id, 0u32],
        );
    }
}

// Typed from 0041c7c0
/// The editor id's text (the first word of the string at +0xc of the type 0x6 extra data, read in
/// place), or 0.
pub fn editor_id_text(e: &mut Engine, this: u32) -> u32 {
    held_pointer(e, this, KIND_EDITOR_ID, EXTRA_PAYLOAD)
}

// Typed from 0041c7f0
/// The reference pointer (+0xc of the type 0x1c extra data) set, unless `reference` is a reference
/// that does not persist (`TESObjectREFR::GetRefPersists`; the save/load singleton's test before it,
/// `0047c850`, always answers false: in place). Stored, or one (0x10 bytes, `004327e0`) built from
/// it and added; a null one too.
///
/// JIP LN NVSE's fix for temporary forms (`WriteRelCall(0x41C833, IsPersistentHook)`) answers the
/// persistence test while its row is on: the seam answers it.
pub fn set_reference_pointer(e: &mut Engine, this: u32, reference: u32) {
    if reference != 0 {
        let fixes = e.jip_patches() & engine_core::extend::jip_patches::FIXES != 0;
        let persists = match fixes.then(|| e.site(0x0041_c833, &[reference])).flatten() {
            Some(persists) => persists != 0,
            None => e.call(GET_REF_PERSISTS, &args![reference]).bool(),
        };
        if !persists {
            return;
        }
    }
    access::set_word_or_add(
        e,
        this,
        KIND_REFERENCE_POINTER,
        EXTRA_REFERENCE_POINTER_INIT,
        reference,
    );
}

// Typed from 0041c8d0
/// `ExtraDataList::GetReferencePointer` (Xbox PDB): the word at +0xc of the type 0x1c extra data, or
/// 0.
pub fn reference_pointer(e: &mut Engine, this: u32) -> u32 {
    access::word_or(e, this, KIND_REFERENCE_POINTER, 0)
}

// Typed from 0041c900
/// The type 0x1c extra data (the reference pointer) removed and deleted, if there is one.
pub fn remove_reference_pointer(e: &mut Engine, this: u32) {
    let extra = first_of_kind(e, this, KIND_REFERENCE_POINTER);
    if extra != 0 {
        list_remove(e, this, extra, true);
    }
}
