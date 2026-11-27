//! The extra data list in the buffered save format (`BGSSaveGameBuffer`, `BGSLoadGameBuffer`):
//! `ExtraDataList::SaveGame_ov2` (`00426a30`) and `LoadGame_ov2` (`00428150`), the fix-up of a loaded
//! list (`0042ceb0`), `FinishLoadGame` (`0042d9e0`) and the clean-up after a load (`0042dae0`). The
//! buffer's own accessors of this family are done in place: its save kind (`00428110`, the
//! unaligned word at +0x17, tested by `004280f0` and set by `00428130`), its flags (`0042ce30`,
//! +0x2c) and the flag bit 3 of its word at +0x28 (`0042ce90`, `0042ce50`); so are the leaves a map
//! marker's byte (`00424940`), a form's id (`0084e3a0`), a list node's item and next, an array's
//! count (+8) and element (`006a7ad0`), a handle's word (`00559450`), an object's word at +0x14
//! (`00825c00`), a reference's extra list (`005d43c0`), a list node's item store (`00726c60`), the
//! store of a payload word (`0041fd00`) and a created form's id (`00469860`). The buffer's reads and
//! writes, the other units' savers, loaders and constructors are called by address; the list's own
//! setters, getters and constructors are typed calls.
//!
//! **Extenders.** JIP LN NVSE puts its own arm for type 0x27 into both jump tables (`0x427F30` in
//! the save, `0x42C328` in the load) while JIP is installed: each runs as an `e.site` where the
//! translation has it.

use super::{
    access, cell_data, classes, first_of_kind, followers, head_of, kind_of, links, list_add,
    list_remove, new_object, next_of, packages, read_word, setters, word_at, EXTRA_PAYLOAD, FAMILY,
    OPERATOR_DELETE, OPERATOR_NEW,
};
use crate::crt::low_byte;
use crate::{cell_refs, dispatch, forms, save};
use engine_core::extend::jip_patches;
use engine_core::x87::x87_quiet_bits;
use engine_core::{args, typed_call, Engine, Ptr};

/// The mask of save kinds that save each extra data type (0x93 words).
const SAVE_KIND_TABLE: u32 = 0x0118_3d30;
/// A mask's bit for a type saved only for a reference that is a created form.
const CREATED_ONLY: u32 = 0x4000_0000;
/// The save buffer's methods (`thiscall(buffer, ...)`).
const SAVE_BYTES: u32 = 0x0086_5e50;
const SAVE_STRING: u32 = 0x0086_5e70;
const START_SIZED_VALUE: u32 = 0x0086_5f20;
const SAVE_SIZED_VALUE_AT: u32 = 0x0086_5ff0;
const SAVE_SIZED_VALUE: u32 = 0x0086_5f60;
/// The load buffer's methods.
const LOAD_BYTES: u32 = 0x0086_4980;
const LOAD_FORM_ID: u32 = 0x0086_48a0;
const LOAD_FORM_ID_AT: u32 = 0x0086_48e0;
const LOAD_SIZED_VALUE: u32 = 0x0086_4a60;
const LOAD_STRING: u32 = 0x0086_49a0;
const SET_BUFFER_FLAGS: u32 = 0x0086_cf00;
/// The warning both formats give for a type they do not know (`cdecl(format, type)`).
const WARNING: u32 = 0x0084_5cc0;
const SAVE_UNKNOWN_TYPE: u32 = 0x0101_57b0;
const LOAD_UNKNOWN_TYPE: u32 = 0x0101_57f8;
const PAIR_NOT_FOUND: u32 = 0x0101_5838;
const UNKNOWN_REFERENCE: u32 = 0x0101_5890;
/// `__RTDynamicCast` and the type descriptors it is given.
const RT_DYNAMIC_CAST: u32 = 0x00ec_43fb;
const RTTI_BS_EXTRA_DATA: u32 = 0x0118_3b2c;
const RTTI_TES_FORM: u32 = 0x0118_3028;
const RTTI_TES_OBJECT_REFR: u32 = 0x0118_41cc;
const RTTI_TES_GLOBAL: u32 = 0x0118_3ad4;
const RTTI_TES_SCRIPT: u32 = 0x0118_46bc;
const RTTI_TES_ENCOUNTER_ZONE: u32 = 0x0118_40dc;
const RTTI_TES_PACKAGE: u32 = 0x0118_46a0;
const RTTI_TES_KEY: u32 = 0x0118_41b4;
const RTTI_ACTOR: u32 = 0x0118_46d4;
const RTTI_TYPE_73_FORM: u32 = 0x0118_4738;
const RTTI_TYPE_75_FORM: u32 = 0x0118_4720;
const RTTI_TYPE_5E_FORM: u32 = 0x0118_4704;
const RTTI_TYPE_6E_FORM: u32 = 0x0118_40c4;
const RTTI_TYPE_3F_SOURCE: u32 = 0x0118_3140;
const RTTI_TYPE_3F_TARGET: u32 = 0x0118_30cc;
const RTTI_TYPE_2E_FORM: u32 = 0x0118_46e8;
const RTTI_DISMEMBERED_FORM: u32 = 0x0118_3108;
const RTTI_TYPE_92_EXTRA: u32 = 0x0118_4078;
const RTTI_TES_SCRIPTABLE_FORM: u32 = 0x0118_3254;
const RTTI_MOBILE_OBJECT: u32 = 0x0118_4920;
/// A form's name slot.
const NAME_SLOT: u32 = 0x130;
/// Globals: the player, the object manager, the pair lookup's object.
const PLAYER: u32 = 0x011d_ea3c;
const OBJECT_MANAGER: u32 = 0x011d_df38;
const PAIR_LOOKUP_OBJECT: u32 = 0x011e_0e80;
/// Other units: arrays, lists, savers and loaders, constructors.
const ARRAY_RESIZE: u32 = 0x0042_f5f0;
const ARRAY_RESIZE_OTHER: u32 = 0x006f_2290;
const ARRAY_REMOVE_AT: u32 = 0x009a_4320;
const LIST_ADD_HEAD: u32 = 0x005a_e3d0;
const LIST_REMOVE_AFTER: u32 = 0x0090_5330;
const LIST_REMOVE_HEAD: u32 = 0x0063_f7b0;
const TIME_STAMP_SAVE: u32 = 0x009a_5f90;
const TIME_STAMP_LOAD: u32 = 0x009a_5ff0;
const REMOVE_OLD_HITS: u32 = 0x0043_5e40;
const SCRIPT_LOCALS_SAVE: u32 = 0x005a_9db0;
const SCRIPT_LOCALS_LOAD: u32 = 0x005a_9f20;
const SCRIPT_LOCALS_NEW: u32 = 0x005a_8b80;
const SCRIPT_LOCALS_DROP: u32 = 0x005a_8bc0;
const SCRIPT_CREATE_LOCALS: u32 = 0x005a_bf60;
const SCRIPT_LOCALS_FINISH: u32 = 0x005a_a090;
const MODEL_SWAP_INDEX: u32 = 0x0047_5920;
const MODEL_SWAP_OF_INDEX: u32 = 0x0047_59a0;
const MAGIC_ITEM_ID: u32 = 0x0040_a1e0;
const MAGIC_TARGET_ID: u32 = 0x0082_54c0;
const MAGIC_ITEM_OF: u32 = 0x0040_a250;
const MAGIC_TARGET_OF_ID: u32 = 0x0082_5550;
const EFFECTS_SAVE: u32 = 0x0080_6a10;
const EFFECTS_LOAD: u32 = 0x0080_6a90;
const EFFECTS_RESOLVE: u32 = 0x0080_6b00;
const EFFECTS_FINISH: u32 = 0x0080_6b50;
const TOPIC_SAVE: u32 = 0x0083_fb10;
const TOPIC_LOAD: u32 = 0x0083_fbe0;
const TELEPORT_SAVE: u32 = 0x0043_aa40;
const TELEPORT_LOAD: u32 = 0x0043_aaa0;
const TELEPORT_NEW: u32 = 0x0043_a160;
const TELEPORT_FINISH: u32 = 0x0083_5fe0;
const STRING_DROP: u32 = 0x0040_37d0;
const SET_FIRST_STRING: u32 = 0x0043_8260;
const SET_SECOND_STRING: u32 = 0x0043_8280;
const SELF_DAMAGE_STAGE: u32 = 0x0047_7bc0;
const SET_SELF_DAMAGE: u32 = 0x0047_7ce0;
const PAIR_LOOKUP: u32 = 0x0097_24e0;
const OBJECT_OF_KIND: u32 = 0x008c_6ca0;
const MARKER_SET_FLAGS: u32 = 0x0042_e680;
const PACKAGE_NEW: u32 = 0x0067_0b90;
const REGISTER_OBJECT: u32 = 0x0084_aa30;
const HANDLE_OF_ID: u32 = 0x0066_e330;
const ASSIGN_HANDLE: u32 = 0x0042_f780;
const RECORD_8B_NEW: u32 = 0x0043_7c10;
const LIMBS_NEW: u32 = 0x0043_0200;
const LIMBS_COMPARE: u32 = 0x0043_0660;
const OWNER_CHANGE_BEGIN: u32 = 0x008b_6820;
const OWNER_CHANGE_END: u32 = 0x008b_6ae0;
const OWNER_REFRESH: u32 = 0x008a_dc50;
const OWNER_SET_BASE: u32 = 0x0057_5690;
const CREATURE_ACTOR_NEW: u32 = 0x0060_1170;
const CREATURE_NEW: u32 = 0x005f_7230;
const INVENTORY_CHANGES: u32 = 0x004b_f220;
const CHANGES_FIX_A: u32 = 0x004d_1440;
const CHANGES_FIX_B: u32 = 0x004d_1610;
const CHANGES_FIX_C: u32 = 0x004d_1960;
const PROCEDURE_TYPE: u32 = 0x0096_11e0;
const CALCULATE_PROCEDURE_TYPE: u32 = 0x0067_77b0;
const CRIME_ITEM_FIX: u32 = 0x009e_b9c0;
const SAY_TO_CALL_BACK: u32 = 0x0093_6a20;
const MODEL_SWAP: u32 = 0x0042_e250;

/// The load's constructors of the types it builds when the list has none: `(type, size,
/// constructor)`. The ones of this family and the cell references' are typed calls
/// ([`construct`]).
const CONSTRUCTIONS: &[(u8, u32, u32)] = &[
    (0x0d, 0x14, 0x0042_c760),
    (0x18, 0x20, 0x0043_2650),
    (0x19, 0x1c, 0x0043_2810),
    (0x1a, 0x10, 0x0042_c860),
    (0x1b, 0x10, 0x0043_3010),
    (0x1c, 0x10, 0x0042_c830),
    (0x1d, 0x10, 0x0043_0dd0),
    (0x1e, 0x10, 0x0043_5040),
    (0x21, 0x10, 0x0042_c5e0),
    (0x22, 0x10, 0x0042_c610),
    (0x23, 0x10, 0x0042_c640),
    (0x24, 0x10, 0x0042_c670),
    (0x25, 0x10, 0x0042_c6a0),
    (0x26, 0x10, 0x0042_c6d0),
    (0x27, 0x10, 0x0042_c700),
    (0x28, 0x10, 0x0042_c730),
    (0x2a, 0x10, 0x0042_c4b0),
    (0x2b, 0x10, 0x0042_c580),
    (0x2e, 0x14, 0x0043_11f0),
    (0x2f, 0x14, 0x0042_c8c0),
    (0x30, 0x10, 0x0042_c7d0),
    (0x32, 0x24, 0x0082_5a10),
    (0x33, 0x28, 0x0082_5ea0),
    (0x35, 0x10, 0x0043_29d0),
    (0x39, 0x10, 0x0043_5920),
    (0x3c, 0x10, 0x0043_4fc0),
    (0x3f, 0x10, 0x0042_c900),
    (0x45, 0x1c, 0x0043_5c40),
    (0x46, 0x10, 0x0042_c930),
    (0x49, 0x10, 0x0043_08c0),
    (0x4a, 0x10, 0x0042_c800),
    (0x4d, 0x10, 0x0043_24a0),
    (0x4e, 0x10, 0x0042_c990),
    (0x50, 0x10, 0x0041_bef0),
    (0x55, 0x10, 0x0043_6ac0),
    (0x56, 0x10, 0x0042_c9c0),
    (0x5b, 0x14, 0x0042_c9f0),
    (0x5c, 0x10, 0x0042_ca30),
    (0x5d, 0x10, 0x0042_ca60),
    (0x5e, 0x10, 0x0043_6cb0),
    (0x60, 0x10, 0x0042_ca90),
    (0x6c, 0x10, 0x0041_e750),
    (0x6e, 0x14, 0x0042_cba0),
    (0x70, 0x10, 0x0042_cbe0),
    (0x73, 0x10, 0x0043_7440),
    (0x74, 0x10, 0x0043_2e60),
    (0x75, 0x1c, 0x0043_7690),
    (0x7c, 0x1c, 0x0043_0f30),
    (0x89, 0x10, 0x0043_36c0),
    (0x8b, 0x28, 0x0043_7c40),
    (0x8d, 0x10, 0x0042_cc40),
    (0x8f, 0x1c, 0x0042_ccc0),
    (0x92, 0x14, 0x0041_1e40),
];

/// A save kind mask's word, for an extra data type.
#[inline]
fn kind_mask(e: &Engine, kind: u32) -> u32 {
    e.mem
        .u32(SAVE_KIND_TABLE.wrapping_add(kind.wrapping_mul(4)))
}

/// Whether the buffer's save kind has a bit of `mask`, in place (`00428110`, `004280f0`).
#[inline]
fn kind_allows(e: &Engine, buffer: u32, mask: u32) -> bool {
    e.mem.u32(buffer.wrapping_add(0x17)) & mask != 0
}

/// Whether the buffer's save kind lets the extra data of `kind` through.
#[inline]
fn allows(e: &Engine, buffer: u32, kind: u8) -> bool {
    let mask = kind_mask(e, u32::from(kind));
    kind_allows(e, buffer, mask)
}

/// Whether the buffer's flags (+0x2c) have a bit of `mask`, in place (`0042ce30`, `004280f0`).
#[inline]
fn flags_allow(e: &Engine, buffer: u32, mask: u32) -> bool {
    e.mem.u32(buffer.wrapping_add(0x2c)) & mask != 0
}

/// A form's id, in place (`0084e3a0`).
#[inline]
fn id_of(e: &Engine, form: u32) -> u32 {
    word_at(e, form, 0x0c)
}

/// Whether a form id is a created form's, in place (`00469860`).
#[inline]
fn is_created(id: u32) -> bool {
    id >= 0xff00_0000
}

/// An array's count (+8, `0044ddc0`) and element `index`'s address (`006a7ad0`), in place.
#[inline]
fn array_count(e: &Engine, array: u32) -> u32 {
    word_at(e, array, 8)
}

#[inline]
fn element(e: &Engine, array: u32, index: u32) -> u32 {
    word_at(e, array, 4).wrapping_add(index.wrapping_mul(4))
}

/// Whether a list node holds nothing (`008256d0`: both words zero), in place.
#[inline]
fn node_empty(e: &Engine, node: u32) -> bool {
    word_at(e, node, 0) == 0 && word_at(e, node, 4) == 0
}

/// A reference's extra list, in place (`005d43c0`'s `LEA EAX,[ECX+0x44]`).
#[inline]
fn extra_list_of(reference: u32) -> u32 {
    reference.wrapping_add(0x44)
}

/// `size` bytes at `at` saved into the buffer.
fn save(e: &mut Engine, buffer: u32, at: u32, size: u32) {
    e.call(SAVE_BYTES, &args![buffer, at, size, 0u32]);
}

/// The low `size` bytes of `value` saved from a local.
fn save_value(e: &mut Engine, buffer: u32, value: u32, size: u32) {
    e.with_stack(4, |e, local| {
        e.mem.set_u32(local.addr(), value);
        save(e, buffer, local.addr(), size);
    });
}

/// A form saved as its form id (`SaveFormID_ov2`, the save family's).
fn save_form(e: &mut Engine, buffer: u32, form: u32) {
    typed_call!(
        save::FAMILY,
        0x0086_5df0,
        save::refs::save_form_id(e, Ptr::new(buffer), Ptr::new(form), 0)
    );
}

/// An id saved as it is (`SaveFormID`, the save family's).
fn save_id(e: &mut Engine, buffer: u32, id: u32) {
    typed_call!(
        save::FAMILY,
        0x0086_5db0,
        save::refs::save_numeric_form_id(e, Ptr::new(buffer), id, 0)
    );
}

/// A counted value: its start, `each` (which answers how many it saved), its count saved there.
fn save_counted(e: &mut Engine, buffer: u32, each: impl FnOnce(&mut Engine) -> u32) {
    let start = e.call(START_SIZED_VALUE, &args![buffer]).u32();
    let count = each(e);
    e.call(SAVE_SIZED_VALUE_AT, &args![buffer, count, start]);
}

/// The items of a simple list from `node` on (null items too), as the buffered saver walks them.
fn each_node_item(e: &mut Engine, mut node: u32, mut f: impl FnMut(&mut Engine, u32)) {
    while node != 0 {
        let item = word_at(e, node, 0);
        f(e, item);
        node = word_at(e, node, 4);
    }
}

/// `size` bytes of the buffer loaded to `at`.
fn load(e: &mut Engine, buffer: u32, at: u32, size: u32) {
    e.call(LOAD_BYTES, &args![buffer, at, size]);
}

/// `size` bytes loaded into a local that starts as `initial`, answered.
fn load_value(e: &mut Engine, buffer: u32, size: u32, initial: Option<u32>) -> u32 {
    e.with_stack(4, |e, local| {
        if let Some(initial) = initial {
            e.mem.set_u32(local.addr(), initial);
        }
        load(e, buffer, local.addr(), size);
        e.mem.u32(local.addr())
    })
}

/// The next form id of the buffer (`LoadFormID`).
fn load_id(e: &mut Engine, buffer: u32) -> u32 {
    e.call(LOAD_FORM_ID, &args![buffer]).u32()
}

/// A form loaded to `at` (`LoadFormID_ov2`).
fn load_form_to(e: &mut Engine, buffer: u32, at: u32) {
    e.call(LOAD_FORM_ID_AT, &args![buffer, at]);
}

/// A counted value's count.
fn load_count(e: &mut Engine, buffer: u32) -> u32 {
    e.call(LOAD_SIZED_VALUE, &args![buffer]).u32()
}

/// The buffer's version byte (its slot 0).
fn buffer_version(e: &mut Engine, buffer: u32) -> u8 {
    e.vcall(buffer, 0, &args![]).u8()
}

/// The form of an id (`004839c0`, the forms family's).
fn form_by_id(e: &mut Engine, id: u32) -> u32 {
    typed_call!(
        forms::FAMILY,
        0x0048_39c0,
        forms::tesform::form_by_id(e, id)
    )
}

/// The form of an id cast from `TESForm` to `target`.
fn form_as(e: &mut Engine, id: u32, target: u32) -> u32 {
    let form = form_by_id(e, id);
    e.call(
        RT_DYNAMIC_CAST,
        &args![form, 0u32, RTTI_TES_FORM, target, 0u32],
    )
    .u32()
}

/// The form of an id cast to `target`, null for the id 0 (the exe tests the id first).
fn form_or_null(e: &mut Engine, id: u32, target: u32) -> u32 {
    if id == 0 {
        0
    } else {
        form_as(e, id, target)
    }
}

/// The extra data of `kind` removed (`RemoveExtraOfType`).
fn remove(e: &mut Engine, list: u32, kind: u8) {
    setters::remove_kind(e, list, kind);
}

/// `BSSimpleList::AddHead` given a local that holds `item`.
fn add_head(e: &mut Engine, list: u32, item: u32) {
    e.with_stack(4, |e, local| {
        e.mem.set_u32(local.addr(), item);
        e.call(LIST_ADD_HEAD, &args![list, local]);
    });
}

// Typed from 00426a30
/// `ExtraDataList::SaveGame_ov2` (Xbox PDB): the list saved into the save buffer `buffer` as a counted
/// value of each extra data that the buffer's save kind lets through (the table at `01183d30`; a
/// type whose mask has bit 0x40000000 only when the buffer's reference, its slot 4, is a created
/// form; type 0x89 only when the buffer's slot 8 answers non-zero): its type byte and payload.
pub fn save_game_buffered(e: &mut Engine, this: u32, buffer: u32) {
    let start = e.call(START_SIZED_VALUE, &args![buffer]).u32();
    let reference = e.vcall(buffer, 4, &args![]).u32();
    let flag = e.vcall(buffer, 8, &args![]).u32();
    let created = reference != 0 && is_created(id_of(e, reference));
    let mut count = 0u32;
    let mut extra = head_of(e, this);
    while extra != 0 {
        let kind = kind_of(e, extra);
        let mask = kind_mask(e, u32::from(kind));
        let saved = mask != 0
            && kind_allows(e, buffer, mask)
            && (created || mask & CREATED_ONLY == 0)
            && (flag != 0 || kind != 0x89);
        if saved {
            save_value(e, buffer, u32::from(kind), 1);
            save_payload(e, buffer, extra, kind, flag);
            count += 1;
        }
        extra = next_of(e, extra);
    }
    e.call(SAVE_SIZED_VALUE_AT, &args![buffer, count, start]);
}

/// One extra data's payload in [`save_game_buffered`] (the jump table at `00427ed8`).
fn save_payload(e: &mut Engine, buffer: u32, extra: u32, kind: u8, flag: u32) {
    // JIP LN NVSE's arm for type 0x27 (`SafeWrite32(0x427F30, ...)`) while JIP is installed.
    if e.jip_patches() & jip_patches::INSTALLED != 0
        && kind == 0x27
        && e.site(0x0042_7f30, &[buffer, extra]).is_some()
    {
        return;
    }
    let at = |offset: u32| extra.wrapping_add(offset);
    match kind {
        0x16 | 0x3e | 0x90 | 0x91 | 0x1f => {}
        0x24 => save(e, buffer, at(0x0c), 2),
        // The friend hits: the old hits go first, then the counted time stamps.
        0x45 => {
            e.call(REMOVE_OLD_HITS, &args![extra]);
            let array = at(0x0c);
            let n = array_count(e, array);
            e.call(SAVE_SIZED_VALUE, &args![buffer, n]);
            for index in 0..n {
                let hit = element(e, array, index);
                e.call(TIME_STAMP_SAVE, &args![hit, buffer]);
            }
        }
        // The dismembered limbs: fields, a form and the counted records, each with its forms.
        0x5f => {
            save(e, buffer, at(0x0c), 2);
            save(e, buffer, at(0x10), 4);
            save(e, buffer, at(0x18), 4);
            save(e, buffer, at(0x1c), 1);
            let form = word_at(e, extra, 0x14);
            save_form(e, buffer, form);
            let array = at(0x20);
            let n = array_count(e, array);
            e.call(SAVE_SIZED_VALUE, &args![buffer, n]);
            for index in 0..n {
                let entry = e.mem.u32(element(e, array, index));
                for offset in 0..4 {
                    save(e, buffer, entry.wrapping_add(offset), 1);
                }
                let inner = entry.wrapping_add(4);
                let inner_n = array_count(e, inner);
                e.call(SAVE_SIZED_VALUE, &args![buffer, inner_n]);
                for inner_index in 0..inner_n {
                    let form = e.mem.u32(element(e, inner, inner_index));
                    save_form(e, buffer, form);
                }
            }
        }
        0x25 | 0x30 | 0x23 | 0x56 | 0x5c | 0x5d | 0x1e | 0x27 | 0x28 => {
            save(e, buffer, at(0x0c), 4);
        }
        0x4a | 0x26 | 0x8d | 0x4e => save(e, buffer, at(0x0c), 1),
        0x21 | 0x22 | 0x1c | 0x49 | 0x6c | 0x74 | 0x55 | 0x3c | 0x39 | 0x46 | 0x89 => {
            let form = word_at(e, extra, 0x0c);
            save_form(e, buffer, form);
        }
        // The script: its form, then its locals.
        0x0d => {
            let form = word_at(e, extra, 0x0c);
            save_form(e, buffer, form);
            let locals = word_at(e, extra, 0x10);
            e.call(SCRIPT_LOCALS_SAVE, &args![locals, buffer]);
        }
        // The model swap: the form, then the swap's index in it.
        0x5b => {
            let form = word_at(e, extra, 0x10);
            save_form(e, buffer, form);
            let swap = word_at(e, extra, 0x0c);
            let index = e.call(MODEL_SWAP_INDEX, &args![form, swap]).u32();
            save_value(e, buffer, index, 4);
        }
        // The package: its form and target, a word and three bytes.
        0x19 => {
            let form = word_at(e, extra, 0x0c);
            save_form(e, buffer, form);
            let target = word_at(e, extra, 0x14);
            save_form(e, buffer, target);
            save(e, buffer, at(0x10), 4);
            save(e, buffer, at(0x18), 1);
            save(e, buffer, at(0x19), 1);
            save(e, buffer, at(0x1a), 1);
        }
        // The package start location: its form, 12 bytes and the rotation.
        0x18 => {
            let form = word_at(e, extra, 0x0c);
            save_form(e, buffer, form);
            save(e, buffer, at(0x10), 0x0c);
            save(e, buffer, at(0x1c), 4);
        }
        // The run once packages and the type 0x5e list: counted (form, byte) records.
        0x1b | 0x5e => {
            let first = word_at(e, extra, 0x0c);
            save_counted(e, buffer, |e| {
                let mut n = 0;
                each_node_item(e, first, |e, item| {
                    if item != 0 {
                        let form = word_at(e, item, 0);
                        save_form(e, buffer, form);
                        save(e, buffer, item.wrapping_add(4), 1);
                        n += 1;
                    }
                });
                n
            });
        }
        // The followers: counted forms.
        0x1d => save_counted(e, buffer, |e| {
            let mut n = 0;
            let first = word_at(e, extra, 0x0c);
            each_node_item(e, first, |e, item| {
                if item != 0 {
                    save_form(e, buffer, item);
                    n += 1;
                }
            });
            n
        }),
        // Counted (form, word, word) records.
        0x73 => save_counted(e, buffer, |e| {
            let mut n = 0;
            let first = word_at(e, extra, 0x0c);
            each_node_item(e, first, |e, item| {
                if item != 0 {
                    let form = word_at(e, item, 0);
                    save_form(e, buffer, form);
                    save(e, buffer, item.wrapping_add(4), 4);
                    save(e, buffer, item.wrapping_add(8), 4);
                    n += 1;
                }
            });
            n
        }),
        // Two forms and a byte.
        0x75 => {
            let first = word_at(e, extra, 0x10);
            save_form(e, buffer, first);
            let second = word_at(e, extra, 0x0c);
            save_form(e, buffer, second);
            save(e, buffer, at(0x18), 1);
        }
        // Counted pairs: each item's words at +0x28 and +4 (`0045cd60`, `00726070`, in place).
        0x35 => save_counted(e, buffer, |e| {
            let mut n = 0;
            let first = word_at(e, extra, 0x0c);
            each_node_item(e, first, |e, item| {
                if item != 0 {
                    let a = word_at(e, item, 0x28);
                    let b = word_at(e, item, 4);
                    save_value(e, buffer, a, 4);
                    save_value(e, buffer, b, 4);
                    n += 1;
                }
            });
            n
        }),
        // The package data: its object's kind byte (slot 8, -1 for none), then its data (slot 0xc).
        0x70 => {
            let object = word_at(e, extra, 0x0c);
            let value = if object != 0 {
                e.vcall(object, 8, &args![]).u32()
            } else {
                u32::MAX
            };
            save_value(e, buffer, value, 1);
            if object != 0 {
                e.vcall(object, 0x0c, &args![buffer]);
            }
        }
        // The map marker: its data's byte at +0xc (`00424940`, in place).
        0x2c => {
            let data = word_at(e, extra, 0x0c);
            let flags = if data != 0 {
                u32::from(e.mem.u8(data.wrapping_add(EXTRA_PAYLOAD)))
            } else {
                0
            };
            save_value(e, buffer, flags, 1);
        }
        // A counted array of ids.
        0x7c => {
            let array = at(0x0c);
            let n = array_count(e, array);
            e.call(SAVE_SIZED_VALUE, &args![buffer, n]);
            for index in 0..n {
                let id = e.mem.u32(element(e, array, index));
                save_id(e, buffer, id);
            }
        }
        0x6e => {
            let form = word_at(e, extra, 0x0c);
            save_form(e, buffer, form);
            save(e, buffer, at(0x10), 4);
        }
        // The trespass package: its form and its object's state (slot 0x54).
        0x1a => {
            let object = word_at(e, extra, 0x0c);
            save_form(e, buffer, object);
            if object != 0 {
                e.vcall(object, 0x54, &args![buffer]);
            }
        }
        // The lock: its level and flags, the key, two words.
        0x2a => {
            let lock = word_at(e, extra, 0x0c);
            save(e, buffer, lock, 1);
            save(e, buffer, lock.wrapping_add(8), 1);
            let key = word_at(e, lock, 4);
            save_form(e, buffer, key);
            save(e, buffer, lock.wrapping_add(0x0c), 4);
            save(e, buffer, lock.wrapping_add(0x10), 4);
        }
        0x2f => {
            save(e, buffer, at(0x0c), 4);
            save(e, buffer, at(0x10), 1);
        }
        // The magic caster: the magic item's and target's ids, the reference.
        0x32 => {
            let caster = word_at(e, extra, 0x18);
            let caster_id = if caster != 0 {
                e.call(MAGIC_ITEM_ID, &args![caster]).u32()
            } else {
                0
            };
            save_id(e, buffer, caster_id);
            let target = word_at(e, extra, 0x1c);
            let target_id = if target != 0 {
                e.call(MAGIC_TARGET_ID, &args![target]).u32()
            } else {
                0
            };
            save_id(e, buffer, target_id);
            let form = word_at(e, extra, 0x20);
            save_form(e, buffer, form);
        }
        // The magic target: the reference and the active effects.
        0x33 => {
            let form = word_at(e, extra, 0x1c);
            save_form(e, buffer, form);
            e.call(EFFECTS_SAVE, &args![buffer, at(0x20)]);
        }
        // The poison: the id of the form's magic item (+0x30).
        0x3f => {
            let form = word_at(e, extra, 0x0c);
            let id = if form != 0 {
                e.call(MAGIC_ITEM_ID, &args![form.wrapping_add(0x30)]).u32()
            } else {
                0
            };
            save_id(e, buffer, id);
        }
        0x4d => {
            let topic = word_at(e, extra, 0x0c);
            e.call(TOPIC_SAVE, &args![topic, buffer]);
        }
        0x2b => {
            let data = word_at(e, extra, 0x0c);
            e.call(TELEPORT_SAVE, &args![data, buffer]);
        }
        // The actor cause: its handle's object's word at +0x14 (`00825c00`), -1 for none.
        0x60 => {
            let object = read_word(e, at(0x0c));
            let value = if object != 0 {
                word_at(e, object, 0x14)
            } else {
                u32::MAX
            };
            save_value(e, buffer, value, 4);
        }
        // The leveled creature: both forms, then the creature's state (slot 0x54) saved under the
        // marker as the buffer's save kind.
        0x2e => {
            let base = word_at(e, extra, 0x0c);
            save_form(e, buffer, base);
            let template = word_at(e, extra, 0x10);
            save_form(e, buffer, template);
            let object = typed_call!(FAMILY, 0x0041_81e0, access::save_form(e, flag));
            let marker = word_at(e, object.wrapping_add(0x30), 0x28);
            save_value(e, buffer, marker, 4);
            let kind_at = buffer.wrapping_add(0x17);
            let saved = e.mem.u32(kind_at);
            e.mem.set_u32(kind_at, marker);
            e.vcall(object, 0x54, &args![buffer]);
            e.mem.set_u32(kind_at, saved);
        }
        0x54 => save(e, buffer, at(0x14), 4),
        0x50 => {
            save(e, buffer, at(0x0c), 1);
            save(e, buffer, at(0x0d), 1);
        }
        // 12 bytes, an id, a word and counted records.
        0x8b => {
            save(e, buffer, at(0x10), 0x0c);
            let id = word_at(e, extra, 0x1c);
            save_id(e, buffer, id);
            save(e, buffer, at(0x0c), 4);
            save_counted(e, buffer, |e| {
                let mut n = 0;
                each_node_item(e, at(0x20), |e, item| {
                    if item != 0 {
                        save(e, buffer, item, 0x0c);
                        let first = word_at(e, item, 0x0c);
                        save_id(e, buffer, first);
                        save(e, buffer, item.wrapping_add(0x10), 0x0c);
                        let second = word_at(e, item, 0x1c);
                        save_id(e, buffer, second);
                        save(e, buffer, item.wrapping_add(0x20), 1);
                        n += 1;
                    }
                });
                n
            });
        }
        // The securitron face: both strings, each copied to a local, saved and dropped.
        0x8f => {
            for second in [false, true] {
                e.with_stack(8, |e, text| {
                    let text = text.addr();
                    let copy = if second {
                        typed_call!(
                            FAMILY,
                            0x0042_80b0,
                            classes::second_string_to(e, extra, text)
                        )
                    } else {
                        typed_call!(
                            FAMILY,
                            0x0042_8070,
                            classes::first_string_to(e, extra, text)
                        )
                    };
                    let characters = read_word(e, copy);
                    e.call(SAVE_STRING, &args![buffer, characters, 0u32]);
                    e.call(STRING_DROP, &args![text]);
                });
            }
        }
        // The special render flags: the word at +0xc (`0084e3a0`, in place) and 4 bytes at +0x10.
        0x92 => {
            let id = id_of(e, extra);
            save_value(e, buffer, id, 4);
            save(e, buffer, at(0x10), 4);
        }
        _ => {
            e.call(WARNING, &args![SAVE_UNKNOWN_TYPE, u32::from(kind)]);
        }
    }
}

/// The constructor of `kind` run on `block`: a typed call for this family's and the cell
/// references', else by address.
fn construct(e: &mut Engine, kind: u8, block: u32) -> u32 {
    match kind {
        0x0d => typed_call!(FAMILY, 0x0042_c760, classes::script_extra_new(e, block)),
        0x1a => typed_call!(
            FAMILY,
            0x0042_c860,
            classes::trespass_package_extra_new(e, block)
        ),
        0x1c => typed_call!(
            FAMILY,
            0x0042_c830,
            classes::reference_pointer_extra_new(e, block)
        ),
        0x21 => typed_call!(FAMILY, 0x0042_c5e0, classes::ownership_extra_new(e, block)),
        0x22 => typed_call!(FAMILY, 0x0042_c610, classes::global_extra_new(e, block)),
        0x23 => typed_call!(FAMILY, 0x0042_c640, classes::rank_extra_new(e, block)),
        0x24 => typed_call!(FAMILY, 0x0042_c670, classes::count_extra_new(e, block)),
        0x25 => typed_call!(FAMILY, 0x0042_c6a0, classes::health_extra_new(e, block)),
        0x26 => typed_call!(FAMILY, 0x0042_c6d0, classes::uses_extra_new(e, block)),
        0x27 => typed_call!(FAMILY, 0x0042_c700, classes::time_left_extra_new(e, block)),
        0x28 => typed_call!(FAMILY, 0x0042_c730, classes::charge_extra_new(e, block)),
        0x2a => typed_call!(FAMILY, 0x0042_c4b0, classes::lock_extra_new(e, block)),
        0x2b => typed_call!(FAMILY, 0x0042_c580, classes::teleport_extra_new(e, block)),
        0x2f => typed_call!(
            FAMILY,
            0x0042_c8c0,
            classes::leveled_item_extra_new(e, block)
        ),
        0x30 => typed_call!(FAMILY, 0x0042_c7d0, classes::scale_extra_new(e, block)),
        0x3f => typed_call!(FAMILY, 0x0042_c900, classes::poison_extra_new(e, block)),
        0x46 => typed_call!(
            FAMILY,
            0x0042_c930,
            classes::head_track_target_extra_new(e, block)
        ),
        0x4a => typed_call!(FAMILY, 0x0042_c800, classes::hot_key_extra_new(e, block)),
        0x4e => typed_call!(FAMILY, 0x0042_c990, classes::no_rumors_extra_new(e, block)),
        0x50 => typed_call!(FAMILY, 0x0041_bef0, cell_data::terminal_state_new(e, block)),
        0x56 => typed_call!(
            FAMILY,
            0x0042_c9c0,
            classes::object_health_extra_new(e, block)
        ),
        0x5b => typed_call!(FAMILY, 0x0042_c9f0, classes::model_swap_extra_new(e, block)),
        0x5c => typed_call!(FAMILY, 0x0042_ca30, classes::radius_extra_default(e, block)),
        0x5d => typed_call!(
            FAMILY,
            0x0042_ca60,
            classes::radiation_extra_default(e, block)
        ),
        0x60 => typed_call!(
            FAMILY,
            0x0042_ca90,
            classes::actor_cause_extra_new(e, block)
        ),
        0x6c => typed_call!(
            FAMILY,
            0x0041_e750,
            links::open_close_activate_ref_new(e, block)
        ),
        0x6e => typed_call!(FAMILY, 0x0042_cba0, classes::ammo_extra_new(e, block)),
        0x70 => typed_call!(
            FAMILY,
            0x0042_cbe0,
            classes::package_data_extra_new(e, block)
        ),
        0x8d => typed_call!(
            FAMILY,
            0x0042_cc40,
            classes::weapon_mod_slots_extra_new(e, block)
        ),
        0x8f => typed_call!(
            FAMILY,
            0x0042_ccc0,
            classes::securitron_face_extra_new(e, block)
        ),
        0x92 => typed_call!(
            cell_refs::FAMILY,
            0x0041_1e40,
            cell_refs::extra_list::type_92_new(e, block)
        ),
        _ => {
            let &(_, _, constructor) = CONSTRUCTIONS
                .iter()
                .find(|entry| entry.0 == kind)
                .unwrap_or_else(|| panic!("no constructor for the extra data type {kind:#x}"));
            e.call(constructor, &args![block]).u32()
        }
    }
}

/// The extra data of `kind` for the load: the list's when it was marked, else a new one (its size
/// from `operator new`, its constructor, null when the allocation failed) added to the list.
fn marked_or_new(e: &mut Engine, list: u32, marks: &[u8; 0x93], kind: u8) -> u32 {
    if marks[usize::from(kind)] != 0 {
        return first_of_kind(e, list, kind);
    }
    let &(_, size, _) = CONSTRUCTIONS
        .iter()
        .find(|entry| entry.0 == kind)
        .unwrap_or_else(|| panic!("no constructor for the extra data type {kind:#x}"));
    let extra = new_object(e, size, |e, block| construct(e, kind, block));
    typed_call!(
        cell_refs::FAMILY,
        0x0040_ff60,
        cell_refs::extra_list::add_extra(e, list, extra)
    )
}

/// The list's extra data of `kind` when it was marked, else none.
fn marked(e: &mut Engine, list: u32, marks: &[u8; 0x93], kind: u8) -> u32 {
    if marks[usize::from(kind)] != 0 {
        first_of_kind(e, list, kind)
    } else {
        0
    }
}

// Typed from 00428150
/// `ExtraDataList::LoadGame_ov2` (Xbox PDB): the list loaded from the load buffer `buffer` (its
/// reference from slot 8, its owner from slot 0xc). The types the list has that the buffer's save
/// kind lets through are marked; each loaded extra data goes into the marked one of its type or a
/// new one, and its mark is cleared. At the end the types still marked are removed, but for type
/// 0x2e (kept) and type 0x92 (its render flags set from its word at +0xc, then kept).
pub fn load_game_buffered(e: &mut Engine, this: u32, buffer: u32) {
    let reference = e.vcall(buffer, 8, &args![]).u32();
    let owner = e.vcall(buffer, 0x0c, &args![]).u32();
    let mut marks = [0u8; 0x93];
    let mut extra = head_of(e, this);
    while extra != 0 {
        let kind = kind_of(e, extra);
        let mask = kind_mask(e, u32::from(kind));
        if mask != 0 && kind_allows(e, buffer, mask) {
            marks[usize::from(kind)] = 1;
        }
        extra = next_of(e, extra);
    }
    let n = load_count(e, buffer);
    for _ in 0..n {
        let kind = low_byte(load_value(e, buffer, 1, None));
        load_payload(e, this, buffer, reference, owner, &marks, kind);
        if let Some(mark) = marks.get_mut(usize::from(kind)) {
            *mark = 0;
        }
    }
    for kind in 0..0x93u8 {
        if marks[usize::from(kind)] == 0 || kind == 0x2e {
            continue;
        }
        if kind == 0x92 {
            let extra = first_of_kind(e, this, 0x92);
            let special = e
                .call(
                    RT_DYNAMIC_CAST,
                    &args![extra, 0u32, RTTI_BS_EXTRA_DATA, RTTI_TYPE_92_EXTRA, 0u32],
                )
                .u32();
            if special != 0 {
                // `0041fd00` in place: the word at +0xc keeps its low three bits.
                let id = id_of(e, special);
                e.mem.set_u32(special.wrapping_add(EXTRA_PAYLOAD), id & 7);
            }
            continue;
        }
        remove(e, this, kind);
    }
}

/// One extra data's payload in [`load_game_buffered`] (the jump table at `0042c2c8`).
fn load_payload(
    e: &mut Engine,
    this: u32,
    buffer: u32,
    reference: u32,
    owner: u32,
    marks: &[u8; 0x93],
    kind: u8,
) {
    // JIP LN NVSE's arm for type 0x27 (`SafeWrite32(0x42C328, ...)`) while JIP is installed.
    if e.jip_patches() & jip_patches::INSTALLED != 0
        && kind == 0x27
        && e.site(0x0042_c328, &[this, buffer]).is_some()
    {
        return;
    }
    match kind {
        0x16 => typed_call!(FAMILY, 0x0041_aa20, setters::set_worn(e, this, true, false)),
        0x3e => typed_call!(
            FAMILY,
            0x0041_ab70,
            setters::set_can_not_wear(e, this, true)
        ),
        // A word, a half word or a byte at +0xc.
        0x24 | 0x25 | 0x30 | 0x23 | 0x8d | 0x4a | 0x5c | 0x5d | 0x1e | 0x26 | 0x27 | 0x28
        | 0x4e => {
            let extra = marked_or_new(e, this, marks, kind);
            let size = match kind {
                0x24 => 2,
                0x8d | 0x4a | 0x26 | 0x4e => 1,
                _ => 4,
            };
            load(e, buffer, extra.wrapping_add(0x0c), size);
        }
        // The id's form as it is, or cast to the type's class.
        0x21 | 0x49 | 0x22 | 0x74 => {
            let extra = marked_or_new(e, this, marks, kind);
            let id = load_id(e, buffer);
            let form = match kind {
                0x22 => form_as(e, id, RTTI_TES_GLOBAL),
                0x74 => form_as(e, id, RTTI_TES_ENCOUNTER_ZONE),
                _ => form_by_id(e, id),
            };
            e.mem.set_u32(extra.wrapping_add(0x0c), form);
        }
        0x1c | 0x6c | 0x39 | 0x3c | 0x46 | 0x55 | 0x89 => {
            let extra = marked_or_new(e, this, marks, kind);
            load_form_to(e, buffer, extra.wrapping_add(0x0c));
        }
        // The object health: 4 bytes, then the reference's destruction stage for that health.
        0x56 => {
            let extra = marked_or_new(e, this, marks, kind);
            load(e, buffer, extra.wrapping_add(0x0c), 4);
            let health = f32::from_bits(x87_quiet_bits(word_at(e, extra, 0x0c)));
            let stage = e.call(SELF_DAMAGE_STAGE, &args![reference, health]).u32();
            if stage != 0 {
                e.call(SET_SELF_DAMAGE, &args![reference, stage]);
            }
        }
        0x0d => load_script(e, this, buffer, reference, marks),
        // The model swap: the form, then the swap of the index in it; kept when both are there
        // (the reference told, slot 0x1cc), else removed.
        0x5b => {
            let extra = marked_or_new(e, this, marks, kind);
            let id = load_id(e, buffer);
            let form = form_by_id(e, id);
            e.mem.set_u32(extra.wrapping_add(0x10), form);
            let index = load_value(e, buffer, 4, Some(u32::MAX));
            if form != 0 {
                let swap = e.call(MODEL_SWAP_OF_INDEX, &args![form, index]).u32();
                e.mem.set_u32(extra.wrapping_add(0x0c), swap);
            }
            if word_at(e, extra, 0x10) != 0 && word_at(e, extra, 0x0c) != 0 {
                if reference != 0 {
                    e.vcall(reference, 0x1cc, &args![0u32, 1u32]);
                }
            } else {
                remove(e, this, 0x5b);
            }
        }
        0x19 => {
            let extra = marked_or_new(e, this, marks, kind);
            load_form_to(e, buffer, extra.wrapping_add(0x0c));
            load_form_to(e, buffer, extra.wrapping_add(0x14));
            load(e, buffer, extra.wrapping_add(0x10), 4);
            load(e, buffer, extra.wrapping_add(0x18), 1);
            load(e, buffer, extra.wrapping_add(0x19), 1);
            load(e, buffer, extra.wrapping_add(0x1a), 1);
        }
        0x18 => {
            let extra = marked_or_new(e, this, marks, kind);
            let id = load_id(e, buffer);
            let form = form_by_id(e, id);
            e.mem.set_u32(extra.wrapping_add(0x0c), form);
            load(e, buffer, extra.wrapping_add(0x10), 0x0c);
            load(e, buffer, extra.wrapping_add(0x1c), 4);
        }
        // The run once packages: each a new (package, byte) record at the list's head.
        0x1b => {
            let extra = marked_or_new(e, this, marks, kind);
            let n = load_count(e, buffer);
            for _ in 0..n {
                let item = e.call(OPERATOR_NEW, &args![8u32]).u32();
                let id = load_id(e, buffer);
                let package = form_as(e, id, RTTI_TES_PACKAGE);
                e.mem.set_u32(item, package);
                load(e, buffer, item.wrapping_add(4), 1);
                let list = word_at(e, extra, 0x0c);
                add_head(e, list, item);
            }
        }
        // The followers: each form at the list's head, as `LoadFormID_ov2` gives it.
        0x1d => {
            let extra = marked_or_new(e, this, marks, kind);
            let n = load_count(e, buffer);
            for _ in 0..n {
                let form = e.with_stack(4, |e, local| {
                    load_form_to(e, buffer, local.addr());
                    e.mem.u32(local.addr())
                });
                let list = word_at(e, extra, 0x0c);
                add_head(e, list, form);
            }
        }
        // New (form, word, word) records at the list's head.
        0x73 => {
            let extra = marked_or_new(e, this, marks, kind);
            let n = load_count(e, buffer);
            for _ in 0..n {
                let item = new_object(e, 0x0c, |e, block| {
                    typed_call!(FAMILY, 0x0042_cc10, classes::record_new(e, block))
                });
                let id = load_id(e, buffer);
                let form = form_as(e, id, RTTI_TYPE_73_FORM);
                e.mem.set_u32(item, form);
                load(e, buffer, item.wrapping_add(4), 4);
                load(e, buffer, item.wrapping_add(8), 4);
                let list = word_at(e, extra, 0x0c);
                add_head(e, list, item);
            }
        }
        0x75 => {
            let extra = marked_or_new(e, this, marks, kind);
            let id = load_id(e, buffer);
            let first = form_as(e, id, RTTI_TYPE_75_FORM);
            e.mem.set_u32(extra.wrapping_add(0x10), first);
            let id = load_id(e, buffer);
            let second = form_as(e, id, RTTI_TYPE_73_FORM);
            e.mem.set_u32(extra.wrapping_add(0x0c), second);
            load(e, buffer, extra.wrapping_add(0x18), 1);
        }
        0x35 => load_crime_pairs(e, this, buffer, reference, marks),
        // The friend hits: the array resized to the count, each time stamp loaded; none left
        // removes them.
        0x45 => {
            let extra = marked_or_new(e, this, marks, kind);
            let array = extra.wrapping_add(0x0c);
            let n = load_count(e, buffer);
            e.call(ARRAY_RESIZE, &args![array, n, 1u32]);
            for index in 0..n {
                let hit = element(e, array, index);
                e.call(TIME_STAMP_LOAD, &args![hit, buffer]);
            }
            if array_count(e, array) == 0 {
                typed_call!(FAMILY, 0x0042_2670, followers::remove_friend_hits(e, this));
            }
        }
        0x5e => load_form_byte_records(e, this, buffer, marks),
        // The package data: a signed kind byte (-1 for none) whose object is built and loaded
        // (slot 0x10).
        0x70 => {
            let extra = marked_or_new(e, this, marks, kind);
            let object_kind = e.with_stack(4, |e, local| {
                e.mem.set_u8(local.addr(), 0xff);
                load(e, buffer, local.addr(), 1);
                e.mem.u8(local.addr())
            });
            if object_kind != 0xff {
                let signed = object_kind as i8 as i32 as u32;
                let object = e.call(OBJECT_OF_KIND, &args![signed]).u32();
                e.mem.set_u32(extra.wrapping_add(0x0c), object);
                e.vcall(object, 0x10, &args![buffer]);
            }
        }
        // The map marker: a byte for the marker data the list already holds.
        0x2c => {
            let flags = load_value(e, buffer, 1, None) & 0xff;
            let extra = marked(e, this, marks, 0x2c);
            if extra != 0 {
                let data = word_at(e, extra, 0x0c);
                if data != 0 {
                    e.call(MARKER_SET_FLAGS, &args![data, flags]);
                }
            }
        }
        0x90 | 0x91 => {}
        // A counted array of forms.
        0x7c => {
            let extra = marked_or_new(e, this, marks, kind);
            let array = extra.wrapping_add(0x0c);
            let n = load_count(e, buffer);
            e.call(ARRAY_RESIZE_OTHER, &args![array, n, 1u32]);
            for index in 0..n {
                let slot = element(e, array, index);
                load_form_to(e, buffer, slot);
            }
        }
        0x6e => {
            let extra = marked_or_new(e, this, marks, kind);
            let id = load_id(e, buffer);
            let form = form_as(e, id, RTTI_TYPE_6E_FORM);
            e.mem.set_u32(extra.wrapping_add(0x0c), form);
            load(e, buffer, extra.wrapping_add(0x10), 4);
        }
        // The trespass package: for a form, a new package object registered under it and loaded
        // (slot 0x5c).
        0x1a => {
            let extra = marked_or_new(e, this, marks, kind);
            let form = e.with_stack(4, |e, local| {
                e.mem.set_u32(local.addr(), 0);
                load_form_to(e, buffer, local.addr());
                e.mem.u32(local.addr())
            });
            if form != 0 {
                let object = e.call(PACKAGE_NEW, &args![0x17u32]).u32();
                e.mem.set_u32(extra.wrapping_add(0x0c), object);
                let id = id_of(e, object);
                let manager = e.mem.u32(OBJECT_MANAGER);
                e.call(REGISTER_OBJECT, &args![manager, form, id]);
                e.vcall(object, 0x5c, &args![buffer]);
            }
        }
        0x1f => typed_call!(FAMILY, 0x0041_a960, setters::set_ghost(e, this, true)),
        // The lock: level, flags and key into the lock data, two words from buffer version 0x10.
        0x2a => {
            let extra = marked_or_new(e, this, marks, kind);
            let lock = word_at(e, extra, 0x0c);
            load(e, buffer, lock, 1);
            load(e, buffer, lock.wrapping_add(8), 1);
            let id = load_id(e, buffer);
            let key = form_as(e, id, RTTI_TES_KEY);
            e.mem.set_u32(lock.wrapping_add(4), key);
            if buffer_version(e, buffer) >= 0x10 {
                load(e, buffer, lock.wrapping_add(0x0c), 4);
                load(e, buffer, lock.wrapping_add(0x10), 4);
            }
        }
        0x2f => {
            let extra = marked_or_new(e, this, marks, kind);
            load(e, buffer, extra.wrapping_add(0x0c), 4);
            load(e, buffer, extra.wrapping_add(0x10), 1);
        }
        0x32 => {
            let extra = marked_or_new(e, this, marks, kind);
            load_form_to(e, buffer, extra.wrapping_add(0x18));
            load_form_to(e, buffer, extra.wrapping_add(0x1c));
            load_form_to(e, buffer, extra.wrapping_add(0x20));
        }
        0x33 => {
            let extra = marked_or_new(e, this, marks, kind);
            load_form_to(e, buffer, extra.wrapping_add(0x1c));
            e.call(EFFECTS_LOAD, &args![buffer, extra.wrapping_add(0x20)]);
        }
        // The poison: the form, then its magic item cast to the poison's class.
        0x3f => {
            let extra = marked_or_new(e, this, marks, kind);
            load_form_to(e, buffer, extra.wrapping_add(0x0c));
            let form = word_at(e, extra, 0x0c);
            let source = e.call(MAGIC_ITEM_OF, &args![form]).u32();
            let poison = e
                .call(
                    RT_DYNAMIC_CAST,
                    &args![source, 0u32, RTTI_TYPE_3F_SOURCE, RTTI_TYPE_3F_TARGET, 0u32],
                )
                .u32();
            e.mem.set_u32(extra.wrapping_add(0x0c), poison);
        }
        0x4d => {
            let extra = marked_or_new(e, this, marks, kind);
            let topic = word_at(e, extra, 0x0c);
            e.call(TOPIC_LOAD, &args![topic, buffer]);
        }
        // The teleport: its data built when there is none, then loaded.
        0x2b => {
            let extra = marked_or_new(e, this, marks, kind);
            if word_at(e, extra, 0x0c) == 0 {
                let data = new_object(e, 0x20, |e, block| {
                    e.call(TELEPORT_NEW, &args![block]).u32()
                });
                e.mem.set_u32(extra.wrapping_add(0x0c), data);
            }
            let data = word_at(e, extra, 0x0c);
            e.call(TELEPORT_LOAD, &args![data, buffer]);
        }
        // The actor cause: a 4-byte id turned into a handle; removed when it names nothing.
        0x60 => {
            let extra = marked_or_new(e, this, marks, kind);
            let id = load_value(e, buffer, 4, Some(u32::MAX));
            let handle = e.call(HANDLE_OF_ID, &args![id]).u32();
            e.call(ASSIGN_HANDLE, &args![extra.wrapping_add(0x0c), handle]);
            if read_word(e, extra.wrapping_add(0x0c)) == 0 {
                remove(e, this, 0x60);
            }
        }
        // 4 bytes at +0x14 of the one the list holds, or read into a local.
        0x54 => {
            let extra = marked(e, this, marks, 0x54);
            if extra != 0 {
                load(e, buffer, extra.wrapping_add(0x14), 4);
            } else {
                e.with_stack(4, |e, local| load(e, buffer, local.addr(), 4));
            }
        }
        0x50 => {
            let extra = marked_or_new(e, this, marks, kind);
            load(e, buffer, extra.wrapping_add(0x0c), 1);
            load(e, buffer, extra.wrapping_add(0x0d), 1);
        }
        0x8b => load_records_8b(e, this, buffer, marks),
        // The securitron face: both strings through a local of 0x100 bytes.
        0x8f => {
            let extra = marked_or_new(e, this, marks, kind);
            e.with_stack(0x100, |e, text| {
                e.call(LOAD_STRING, &args![buffer, text]);
                e.call(SET_FIRST_STRING, &args![extra, text]);
                e.call(LOAD_STRING, &args![buffer, text]);
                e.call(SET_SECOND_STRING, &args![extra, text]);
            });
        }
        // The special render flags: the word (`0041fd00`, in place) and 4 bytes at +0x10.
        0x92 => {
            let extra = marked_or_new(e, this, marks, kind);
            let flags = load_value(e, buffer, 4, None);
            load(e, buffer, extra.wrapping_add(0x10), 4);
            e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), flags);
        }
        0x5f => load_limbs(e, this, buffer, owner, marks),
        0x2e => load_leveled_creature(e, this, buffer, owner, marks),
        _ => {
            e.call(WARNING, &args![LOAD_UNKNOWN_TYPE, u32::from(kind)]);
        }
    }
}

/// [`load_payload`]'s script (type 0x0d): the script, its locals made when the extra data has none,
/// loaded (into a temporary when there are none), with the buffer's form packer value set around it
/// for a buffer without a reference (`007037c0`, the dispatch family's).
fn load_script(e: &mut Engine, this: u32, buffer: u32, reference: u32, marks: &[u8; 0x93]) {
    let extra = marked_or_new(e, this, marks, 0x0d);
    let id = load_id(e, buffer);
    let script = form_as(e, id, RTTI_TES_SCRIPT);
    e.mem.set_u32(extra.wrapping_add(0x0c), script);
    if word_at(e, extra, 0x10) == 0 {
        let locals = if script != 0 {
            e.call(SCRIPT_CREATE_LOCALS, &args![script]).u32()
        } else {
            0
        };
        e.mem.set_u32(extra.wrapping_add(0x10), locals);
    }
    if reference == 0 {
        let value = typed_call!(FAMILY, 0x0042_ce00, classes::given_word(e));
        typed_call!(
            dispatch::FAMILY,
            0x0070_37c0,
            dispatch::refs::set_date(e, Ptr::new(buffer), value)
        );
    }
    let locals = word_at(e, extra, 0x10);
    if locals == 0 {
        e.with_stack(0x14, |e, temporary| {
            e.call(SCRIPT_LOCALS_NEW, &args![temporary]);
            e.call(SCRIPT_LOCALS_LOAD, &args![temporary, buffer]);
            e.call(SCRIPT_LOCALS_DROP, &args![temporary]);
        });
    } else {
        e.call(SCRIPT_LOCALS_LOAD, &args![locals, buffer]);
    }
    if reference == 0 {
        typed_call!(
            dispatch::FAMILY,
            0x0070_37c0,
            dispatch::refs::set_date(e, Ptr::new(buffer), 0)
        );
    }
}

/// [`load_payload`]'s crime pairs (type 0x35): each pair of words looked up (`009724e0`) and put at
/// the list's head, or warned about with the reference's name and id; a list whose first node holds
/// nothing then removes the extra data.
fn load_crime_pairs(e: &mut Engine, this: u32, buffer: u32, reference: u32, marks: &[u8; 0x93]) {
    let extra = marked_or_new(e, this, marks, 0x35);
    let n = load_count(e, buffer);
    for _ in 0..n {
        let (first, second) = e.with_stack(8, |e, pair| {
            let pair = pair.addr();
            e.mem.set_u32(pair, 0);
            e.mem.set_u32(pair.wrapping_add(4), u32::MAX);
            load(e, buffer, pair, 4);
            load(e, buffer, pair.wrapping_add(4), 4);
            (e.mem.u32(pair), e.mem.u32(pair.wrapping_add(4)))
        });
        let item = e
            .call(PAIR_LOOKUP, &args![PAIR_LOOKUP_OBJECT, second, first])
            .u32();
        if item != 0 {
            let list = word_at(e, extra, 0x0c);
            add_head(e, list, item);
        } else {
            let (id, name) = if reference != 0 {
                let id = id_of(e, reference);
                (id, e.vcall(reference, NAME_SLOT, &args![]).u32())
            } else {
                (0, UNKNOWN_REFERENCE)
            };
            e.call(WARNING, &args![PAIR_NOT_FOUND, first, second, name, id]);
        }
    }
    let list = word_at(e, extra, 0x0c);
    if word_at(e, list, 0) == 0 {
        remove(e, this, 0x35);
    }
}

/// [`load_payload`]'s type 0x5e: new (form, byte) records at the list's head; then the records
/// without a form are deleted and their nodes removed (after the previous node, or at the head).
fn load_form_byte_records(e: &mut Engine, this: u32, buffer: u32, marks: &[u8; 0x93]) {
    let extra = marked_or_new(e, this, marks, 0x5e);
    let n = load_count(e, buffer);
    for _ in 0..n {
        let item = e.call(OPERATOR_NEW, &args![8u32]).u32();
        let id = load_id(e, buffer);
        let form = form_as(e, id, RTTI_TYPE_5E_FORM);
        e.mem.set_u32(item, form);
        load(e, buffer, item.wrapping_add(4), 1);
        let list = word_at(e, extra, 0x0c);
        add_head(e, list, item);
    }
    let mut node = word_at(e, extra, 0x0c);
    let mut previous = 0u32;
    while node != 0 && !node_empty(e, node) {
        let item = word_at(e, node, 0);
        let drop = if item == 0 {
            true
        } else if word_at(e, item, 0) == 0 {
            e.call(OPERATOR_DELETE, &args![item]);
            true
        } else {
            false
        };
        if !drop {
            previous = node;
            node = word_at(e, node, 4);
        } else if previous != 0 {
            e.call(LIST_REMOVE_AFTER, &args![previous, node]);
            node = word_at(e, previous, 4);
        } else {
            e.call(LIST_REMOVE_HEAD, &args![node]);
        }
    }
}

/// [`load_payload`]'s type 0x8b: 12 bytes, a form (an id before buffer version 0x12), a word, then
/// counted new records at the list at +0x20, each with its two forms read the same way.
fn load_records_8b(e: &mut Engine, this: u32, buffer: u32, marks: &[u8; 0x93]) {
    let extra = marked_or_new(e, this, marks, 0x8b);
    load(e, buffer, extra.wrapping_add(0x10), 0x0c);
    load_form_or_id(e, buffer, extra.wrapping_add(0x1c));
    load(e, buffer, extra.wrapping_add(0x0c), 4);
    let n = load_count(e, buffer);
    for _ in 0..n {
        let item = new_object(e, 0x24, |e, block| {
            e.call(RECORD_8B_NEW, &args![block]).u32()
        });
        load(e, buffer, item, 0x0c);
        load_form_or_id(e, buffer, item.wrapping_add(0x0c));
        load(e, buffer, item.wrapping_add(0x10), 0x0c);
        load_form_or_id(e, buffer, item.wrapping_add(0x1c));
        load(e, buffer, item.wrapping_add(0x20), 1);
        add_head(e, extra.wrapping_add(0x20), item);
    }
}

/// A form to `at` from buffer version 0x12 on, 4 bytes before it.
fn load_form_or_id(e: &mut Engine, buffer: u32, at: u32) {
    if buffer_version(e, buffer) >= 0x12 {
        load_form_to(e, buffer, at);
    } else {
        load(e, buffer, at, 4);
    }
}

/// [`load_payload`]'s dismembered limbs (type 0x5f): the list's one is taken out (not deleted);
/// without one, the owner's change begins (`008b6820`) when the buffer's flags have 0x20000 and the
/// owner's slot 0x1d0 answers. A new one is loaded (fields, a form, counted records of flag bytes,
/// the fourth from buffer version 0x10, and their forms, those that name nothing dropped). The old
/// one, compared with it (`00430660`), is deleted; the new one is added and the owner's change ends
/// (`008b6ae0`) unless the compare answered true.
fn load_limbs(e: &mut Engine, this: u32, buffer: u32, owner: u32, marks: &[u8; 0x93]) {
    let existing = marked(e, this, marks, 0x5f);
    if existing != 0 {
        list_remove(e, this, existing, false);
    } else if flags_allow(e, buffer, 0x2_0000) && e.vcall(owner, 0x1d0, &args![]).u32() != 0 {
        e.call(OWNER_CHANGE_BEGIN, &args![owner]);
    }
    let extra = new_object(e, 0x30, |e, block| e.call(LIMBS_NEW, &args![block]).u32());
    load(e, buffer, extra.wrapping_add(0x0c), 2);
    load(e, buffer, extra.wrapping_add(0x10), 4);
    load(e, buffer, extra.wrapping_add(0x18), 4);
    load(e, buffer, extra.wrapping_add(0x1c), 1);
    load_form_to(e, buffer, extra.wrapping_add(0x14));
    let records = extra.wrapping_add(0x20);
    let n = load_count(e, buffer);
    e.call(ARRAY_RESIZE_OTHER, &args![records, n, 1u32]);
    for index in 0..n {
        let slot = element(e, records, index);
        let entry = new_object(e, 0x14, |e, block| {
            typed_call!(FAMILY, 0x0042_c470, classes::flags_record_new(e, block))
        });
        e.mem.set_u32(slot, entry);
        load(e, buffer, entry, 1);
        load(e, buffer, entry.wrapping_add(1), 1);
        load(e, buffer, entry.wrapping_add(2), 1);
        if buffer_version(e, buffer) >= 0x10 {
            load(e, buffer, entry.wrapping_add(3), 1);
        }
        let inner = entry.wrapping_add(4);
        let inner_n = load_count(e, buffer);
        e.call(ARRAY_RESIZE_OTHER, &args![inner, inner_n, 1u32]);
        for inner_index in 0..inner_n {
            let inner_slot = element(e, inner, inner_index);
            load_form_to(e, buffer, inner_slot);
        }
        let mut remaining = array_count(e, inner);
        let mut index = 0u32;
        while index < remaining {
            let inner_slot = element(e, inner, index);
            let id = e.mem.u32(inner_slot);
            let form = form_or_null(e, id, RTTI_DISMEMBERED_FORM);
            e.mem.set_u32(inner_slot, form);
            if form == 0 {
                e.call(ARRAY_REMOVE_AT, &args![inner, index, 1u32]);
                remaining -= 1;
            } else {
                index += 1;
            }
        }
    }
    let mut same = false;
    if existing != 0 {
        if e.call(LIMBS_COMPARE, &args![existing, extra]).bool() {
            same = true;
        } else {
            e.call(OWNER_CHANGE_BEGIN, &args![owner]);
        }
        e.vcall(existing, 0, &args![1u32]);
    }
    list_add(e, this, extra);
    if !same {
        e.call(OWNER_CHANGE_END, &args![owner]);
    }
}

/// [`load_payload`]'s leveled creature (type 0x2e: the base form at +0xc, the template at +0x10):
/// both forms and the marker loaded. The owner is told (slot 0x1cc) when its template changed, or
/// when the object manager's flag (`0042ce10`) is clear. A template change the buffer's save kind
/// does not allow (0x08000020) drops the owner's container changes and refreshes it. With an owner,
/// both forms and a base form that takes a template, the creature is made for the pair and set as
/// the owner's base; else the old forms come back, or the extra data goes. A creature that does
/// not exist yet is built (0x20c bytes for an actor kind, slot 0x218, else 0x160) and loaded (slot
/// 0x5c) with the buffer's save kind, flags and flag bit set for it and put back after.
fn load_leveled_creature(e: &mut Engine, this: u32, buffer: u32, owner: u32, marks: &[u8; 0x93]) {
    let extra = marked_or_new(e, this, marks, 0x2e);
    let old_base = word_at(e, extra, 0x0c);
    let old_template = word_at(e, extra, 0x10);
    let id = load_id(e, buffer);
    let base = form_as(e, id, RTTI_TYPE_2E_FORM);
    e.mem.set_u32(extra.wrapping_add(0x0c), base);
    let id = load_id(e, buffer);
    let template = form_as(e, id, RTTI_TYPE_2E_FORM);
    e.mem.set_u32(extra.wrapping_add(0x10), template);
    let marker = load_value(e, buffer, 4, Some(0));
    let template_changed = owner != 0
        && e.vcall(owner, 0x1d0, &args![]).u32() != 0
        && old_template != 0
        && template != old_template;
    if template_changed {
        e.vcall(owner, 0x1cc, &args![0u32, 0u32]);
    } else {
        let manager = e.mem.u32(OBJECT_MANAGER);
        if !typed_call!(FAMILY, 0x0042_ce10, classes::flag_2_of_244(e, manager)) {
            e.vcall(owner, 0x1cc, &args![0u32, 0u32]);
        }
    }
    let not_allowed =
        old_template != word_at(e, extra, 0x10) && !kind_allows(e, buffer, 0x0800_0020);
    if not_allowed {
        typed_call!(
            FAMILY,
            0x0041_aeb0,
            setters::remove_container_changes(e, extra_list_of(owner))
        );
        e.call(OWNER_REFRESH, &args![owner]);
    }
    let base = word_at(e, extra, 0x0c);
    let template = word_at(e, extra, 0x10);
    let mut creature = 0u32;
    if owner != 0
        && base != 0
        && template != 0
        && typed_call!(
            forms::FAMILY,
            0x0047_cdb0,
            forms::actor_base_data::has_actor_template_again(e, base.wrapping_add(0x30))
        )
    {
        creature = typed_call!(
            forms::FAMILY,
            0x0047_d130,
            forms::actor_base_data::create_template_actor_base(e, base, template)
        );
        let actor_base = typed_call!(FAMILY, 0x0041_81e0, access::save_form(e, owner));
        if is_created(id_of(e, actor_base)) && actor_base != 0 {
            e.vcall(actor_base, 0x10, &args![1u32]);
        }
        e.call(OWNER_SET_BASE, &args![owner, creature]);
    } else if old_base != 0 && old_template != 0 {
        e.mem.set_u32(extra.wrapping_add(0x0c), old_base);
        e.mem.set_u32(extra.wrapping_add(0x10), old_template);
    } else {
        remove(e, this, 0x2e);
    }
    if not_allowed {
        let changes = e.call(INVENTORY_CHANGES, &args![owner]).u32();
        e.call(CHANGES_FIX_A, &args![changes]);
        e.call(CHANGES_FIX_B, &args![changes]);
        e.call(CHANGES_FIX_C, &args![changes]);
        if typed_call!(FAMILY, 0x0042_cde0, classes::list_at_is_empty(e, changes)) {
            typed_call!(
                FAMILY,
                0x0041_aeb0,
                setters::remove_container_changes(e, extra_list_of(owner))
            );
        }
    }
    let mut built = false;
    if creature == 0 {
        let actor_kind = e.vcall(owner, 0x218, &args![]).bool();
        let (size, constructor) = if actor_kind {
            (0x20c, CREATURE_ACTOR_NEW)
        } else {
            (0x160, CREATURE_NEW)
        };
        creature = new_object(e, size, |e, block| e.call(constructor, &args![block]).u32());
        built = true;
    }
    // The buffer's save kind (+0x17), flags (+0x2c) and flag bit 3 of +0x28, in place but for the
    // flags' setter (`0086cf00`), replaced while the creature loads.
    let kind_at = buffer.wrapping_add(0x17);
    let saved_kind = e.mem.u32(kind_at);
    let saved_flags = e.mem.u32(buffer.wrapping_add(0x2c));
    let flag_at = buffer.wrapping_add(0x28);
    let saved_bit = e.mem.u32(flag_at) & 8;
    e.mem.set_u32(kind_at, marker);
    e.call(SET_BUFFER_FLAGS, &args![buffer, 0u32]);
    let word = e.mem.u32(flag_at);
    e.mem.set_u32(flag_at, word & !8);
    e.vcall(creature, 0x5c, &args![buffer]);
    typed_call!(
        forms::FAMILY,
        0x0047_e340,
        forms::actor_base_data::set_change_flags(e, creature.wrapping_add(0x30), marker)
    );
    e.mem.set_u32(kind_at, saved_kind);
    e.call(SET_BUFFER_FLAGS, &args![buffer, saved_flags]);
    let word = e.mem.u32(flag_at);
    e.mem.set_u32(flag_at, (word & !8) | saved_bit);
    if built && creature != 0 {
        e.vcall(creature, 0x10, &args![1u32]);
    }
}

/// The form id at `offset` of an extra data replaced by its form cast to `target` (null for none).
/// Answers the form.
fn resolve(e: &mut Engine, extra: u32, offset: u32, target: u32) -> u32 {
    let at = extra.wrapping_add(offset);
    let id = e.mem.u32(at);
    let form = form_or_null(e, id, target);
    e.mem.set_u32(at, form);
    form
}

/// The list's extra data of `kind` when the buffer's save kind lets it through, else none.
fn allowed_extra(e: &mut Engine, list: u32, buffer: u32, kind: u8) -> u32 {
    if allows(e, buffer, kind) {
        first_of_kind(e, list, kind)
    } else {
        0
    }
}

// Typed from 0042ceb0
/// The fix-up of a list after [`load_game_buffered`] read it, `form` the form it belongs to: for
/// each type below that the buffer's save kind lets through and the list has, the form ids become
/// their forms (null for none): the reference pointer; the script, removed unless it is the
/// owner's base form's (+0xf4) or, without an owner, `form`'s as a scriptable form; then with a
/// reference the open/close activate reference (removed when gone), the magic caster, the magic
/// target (and its active effects), the item dropper (removed when gone, else this reference added
/// to its dropped items) and the talking actor; with no owner the teleport data finishes loading;
/// with an owner the package (removed with none, its procedure type computed when it has none), the
/// followers (those that are no actor removed; the player removed for a buffer older than 0x0e),
/// the package data, the merchant container, the trespass package, the head track target, the
/// dismembered limbs' form, the ash pile and the crime pairs.
pub fn fix_up_loaded(e: &mut Engine, this: u32, buffer: u32, form: u32) {
    let reference = e.vcall(buffer, 8, &args![]).u32();
    let owner = e.vcall(buffer, 0x0c, &args![]).u32();
    let extra = allowed_extra(e, this, buffer, 0x1c);
    if extra != 0 {
        resolve(e, extra, 0x0c, RTTI_TES_OBJECT_REFR);
    }
    let extra = allowed_extra(e, this, buffer, 0x0d);
    if extra != 0 && form != 0 {
        let list = if owner == 0 {
            e.call(
                RT_DYNAMIC_CAST,
                &args![form, 0u32, RTTI_TES_FORM, RTTI_TES_SCRIPTABLE_FORM, 0u32],
            )
            .u32()
        } else {
            let base = typed_call!(FAMILY, 0x0041_81e0, access::save_form(e, owner));
            if base != 0 {
                base.wrapping_add(0xf4)
            } else {
                0
            }
        };
        let held = word_at(e, extra, 0x0c);
        if list == 0 || held == 0 || word_at(e, list, 4) != held {
            remove(e, this, 0x0d);
        }
    }
    if reference == 0 {
        return;
    }
    let extra = allowed_extra(e, this, buffer, 0x6c);
    if extra != 0 && resolve(e, extra, 0x0c, RTTI_TES_OBJECT_REFR) == 0 {
        remove(e, this, 0x6c);
    }
    let extra = allowed_extra(e, this, buffer, 0x32);
    if extra != 0 {
        let id = word_at(e, extra, 0x18);
        let item = if id == 0 {
            0
        } else {
            e.call(MAGIC_ITEM_OF, &args![id]).u32()
        };
        e.mem.set_u32(extra.wrapping_add(0x18), item);
        let id = word_at(e, extra, 0x1c);
        let target = if id == 0 {
            0
        } else {
            e.call(MAGIC_TARGET_OF_ID, &args![id]).u32()
        };
        e.mem.set_u32(extra.wrapping_add(0x1c), target);
        resolve(e, extra, 0x20, RTTI_TES_OBJECT_REFR);
    }
    let extra = allowed_extra(e, this, buffer, 0x33);
    if extra != 0 {
        resolve(e, extra, 0x1c, RTTI_TES_OBJECT_REFR);
        e.call(EFFECTS_RESOLVE, &args![buffer, extra.wrapping_add(0x20)]);
    }
    let extra = allowed_extra(e, this, buffer, 0x39);
    if extra != 0 {
        let dropper = resolve(e, extra, 0x0c, RTTI_TES_OBJECT_REFR);
        if dropper == 0 {
            remove(e, this, 0x39);
        } else {
            typed_call!(
                FAMILY,
                0x0041_e000,
                links::add_to_dropped_items(e, extra_list_of(dropper), reference)
            );
        }
    }
    let extra = allowed_extra(e, this, buffer, 0x55);
    if extra != 0 {
        resolve(e, extra, 0x0c, RTTI_MOBILE_OBJECT);
    }
    if owner == 0 {
        let extra = allowed_extra(e, this, buffer, 0x2b);
        if extra != 0 {
            let data = word_at(e, extra, 0x0c);
            e.call(TELEPORT_FINISH, &args![data, buffer]);
        }
        return;
    }
    let extra = allowed_extra(e, this, buffer, 0x19);
    if extra != 0 {
        let package = resolve(e, extra, 0x0c, RTTI_TES_PACKAGE);
        let target = resolve(e, extra, 0x14, RTTI_TES_OBJECT_REFR);
        if package == 0 {
            remove(e, this, 0x19);
        } else if e.call(PROCEDURE_TYPE, &args![package]).u32() == u32::MAX {
            e.call(CALCULATE_PROCEDURE_TYPE, &args![package, target]);
        }
    }
    let extra = allowed_extra(e, this, buffer, 0x1d);
    if extra != 0 {
        fix_followers(e, extra);
        if buffer_version(e, buffer) < 0x0e {
            let player = e.mem.u32(PLAYER);
            typed_call!(
                FAMILY,
                0x0042_2690,
                followers::remove_follower(e, this, player)
            );
        }
    }
    let extra = allowed_extra(e, this, buffer, 0x70);
    if extra != 0 {
        let object = word_at(e, extra, 0x0c);
        if object != 0 {
            // The package extra is asked for and not used.
            typed_call!(FAMILY, 0x0041_cb10, packages::package(e, this));
            e.vcall(object, 0x14, &args![buffer]);
        }
    }
    let extra = allowed_extra(e, this, buffer, 0x3c);
    if extra != 0 {
        resolve(e, extra, 0x0c, RTTI_TES_OBJECT_REFR);
    }
    let extra = allowed_extra(e, this, buffer, 0x1a);
    if extra != 0 {
        let object = word_at(e, extra, 0x0c);
        if object != 0 {
            e.vcall(object, 0x64, &args![buffer]);
        }
    }
    let extra = allowed_extra(e, this, buffer, 0x46);
    if extra != 0 {
        resolve(e, extra, 0x0c, RTTI_TES_OBJECT_REFR);
    }
    let extra = allowed_extra(e, this, buffer, 0x5f);
    if extra != 0 {
        let id = word_at(e, extra, 0x14);
        let found = if id == 0 { 0 } else { form_by_id(e, id) };
        e.mem.set_u32(extra.wrapping_add(0x14), found);
    }
    let extra = allowed_extra(e, this, buffer, 0x89);
    if extra != 0 {
        let target = resolve(e, extra, 0x0c, RTTI_TES_OBJECT_REFR);
        if target != 0 {
            typed_call!(
                FAMILY,
                0x0041_e340,
                links::set_ash_pile_ref(e, extra_list_of(target), owner)
            );
        }
    }
    let extra = allowed_extra(e, this, buffer, 0x35);
    if extra != 0 {
        let first = word_at(e, extra, 0x0c);
        each_node_item(e, first, |e, item| {
            if item != 0 {
                e.call(CRIME_ITEM_FIX, &args![item, owner]);
            }
        });
    }
}

/// The followers (the list at +0xc of the type 0x1d extra data) become the actors their ids name,
/// stored in the node (`00726c60`, in place: the actor is never 0 there); a node whose id names no
/// actor is removed (after the previous node, `00905330`, or at the head, `0063f7b0`).
fn fix_followers(e: &mut Engine, extra: u32) {
    let mut node = word_at(e, extra, 0x0c);
    let mut previous = 0u32;
    while node != 0 && !node_empty(e, node) {
        let id = word_at(e, node, 0);
        let actor = form_or_null(e, id, RTTI_ACTOR);
        if actor != 0 {
            e.mem.set_u32(node, actor);
            previous = node;
            node = word_at(e, node, 4);
        } else if previous == 0 {
            e.call(LIST_REMOVE_HEAD, &args![node]);
        } else {
            e.call(LIST_REMOVE_AFTER, &args![previous, node]);
            node = word_at(e, previous, 4);
        }
    }
}

// Typed from 0042d9e0
/// `ExtraDataList::FinishLoadGame` (Xbox PDB): with the buffer's reference (slot 8), the say-to
/// topic info (type 0x75) the save kind lets through sets its byte at +0x18 when the reference's
/// slot 0xfc answers false, else calls the reference's say-to call back (`00936a20`) with its word at
/// +0xc and whether both of the extra data's words are set; then the magic target's (type 0x33)
/// active effects finish loading.
pub fn finish_load_game(e: &mut Engine, this: u32, buffer: u32) {
    let reference = e.vcall(buffer, 8, &args![]).u32();
    let extra = allowed_extra(e, this, buffer, 0x75);
    if extra != 0 && reference != 0 {
        if !e.vcall(reference, 0xfc, &args![]).bool() {
            e.mem.set_u8(extra.wrapping_add(0x18), 1);
        } else {
            let handle = id_of(e, reference);
            let both = word_at(e, extra, 0x10) != 0 && word_at(e, extra, 0x0c) != 0;
            e.call(SAY_TO_CALL_BACK, &args![handle, u32::from(both)]);
        }
    }
    if reference != 0 {
        let extra = allowed_extra(e, this, buffer, 0x33);
        if extra != 0 {
            e.call(EFFECTS_FINISH, &args![buffer, extra.wrapping_add(0x20)]);
        }
    }
}

// Typed from 0042dae0
/// The clean-up of a list once the buffer is loaded: with the buffer's reference (slot 8; its owner,
/// slot 0xc, asked and not used), a model swap tells the reference (slot 0x1cc), an object health
/// resets its self damage, an item dropper's dropped items lose it, the map marker's byte at +0xd
/// is copied to +0xc (`0042dd70`), the activate children timer is reset when the buffer's flags have
/// 0x4000000, and the script locals finish loading. Then every type whose mask has a bit of the
/// buffer's flags is removed, but for the types 0x0d, 0x2c, 0x2e, 0x49, 0x54, 0x5f, 0x90 and 0x91,
/// and those whose mask has 0x40000000 when the reference is not a created form.
pub fn clean_up_loaded(e: &mut Engine, this: u32, buffer: u32) {
    let reference = e.vcall(buffer, 8, &args![]).u32();
    if reference != 0 {
        e.vcall(buffer, 0x0c, &args![]);
        if e.call(MODEL_SWAP, &args![this]).u32() != 0 {
            e.vcall(reference, 0x1cc, &args![0u32, 1u32]);
        }
        if first_of_kind(e, this, 0x56) != 0 {
            e.call(SET_SELF_DAMAGE, &args![reference, 0u32]);
        }
        let dropper = typed_call!(FAMILY, 0x0041_de00, links::item_dropper(e, this));
        if dropper != 0 {
            typed_call!(
                FAMILY,
                0x0041_e0d0,
                links::remove_dropped_item(e, extra_list_of(dropper), reference)
            );
        }
        let marker = typed_call!(FAMILY, 0x0041_8490, access::map_marker(e, this));
        if marker != 0 {
            typed_call!(FAMILY, 0x0042_dd70, classes::copy_byte_d_to_c(e, marker));
        }
        if flags_allow(e, buffer, 0x0400_0000) {
            typed_call!(
                FAMILY,
                0x0041_eff0,
                links::set_activate_children_timer(e, this, 0.0)
            );
        }
        let locals = typed_call!(FAMILY, 0x0041_8830, access::script_locals(e, this));
        if locals != 0 {
            e.call(SCRIPT_LOCALS_FINISH, &args![locals, buffer]);
        }
    }
    let created = reference != 0 && is_created(id_of(e, reference));
    for kind in 0..0x93u8 {
        let mask = kind_mask(e, u32::from(kind));
        let wanted = mask != 0
            && flags_allow(e, buffer, mask)
            && (created || mask & CREATED_ONLY == 0)
            && !matches!(kind, 0x0d | 0x2c | 0x2e | 0x49 | 0x54 | 0x5f | 0x90 | 0x91);
        if wanted {
            remove(e, this, kind);
        }
    }
}
