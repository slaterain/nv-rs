//! `ExtraDataList::InitItem` (`00416be0`, Xbox PDB): the fix-up of a list loaded from the master
//! file for its reference. Each extra data that holds form ids gets the forms they name (the id as
//! the file stores it through `TESForm::AddCompileIndex`, the lookup `004839c0`, the dynamic cast to
//! the type the slot holds); one whose form is missing is logged ("MASTERFILE: Unable to find ...")
//! and removed, and some link their target back to this reference (the linked reference, the enable
//! parent, the activate parents, the portals). The one-instruction readers the exe calls are done in
//! place: a form's id (`0084e3a0`) and type (`00401170`), a reference's extra list (`005d43c0`), a
//! simple list's item, next and emptiness (`006815c0`, `00726070`, `008256d0`) and item store
//! (`00726c60`), the map marker's flags and reputation (`00438ed0`, `00438ef0`, `0044de80`,
//! `0044edb0`), the audio marker's controller (`0059bb30`, `007037c0`). The forms, the casts, the
//! log and the targets' own fix-ups are called by address.

use super::{access, records, FAMILY, OPERATOR_DELETE};
use super::{give_lock, head_of, kind_of, list_remove, next_of, take_lock, word_at, EXTRA_PAYLOAD};
use engine_core::{args, typed_call, Engine};

/// `TESForm::GetFile(index)` (`-1`: the last file that changed the form).
const TES_FORM_GET_FILE: u32 = 0x0048_4e60;
/// `TESForm::AddCompileIndex(&id, file)` (cdecl) and the lookup by id.
const ADD_COMPILE_INDEX: u32 = 0x0048_5d50;
const LOOKUP_FORM: u32 = 0x0048_39c0;
/// `__RTDynamicCast(form, 0, from, to, 0)` and the type descriptors.
const RT_DYNAMIC_CAST: u32 = 0x00ec_43fb;
const RTTI_TES_FORM: u32 = 0x0118_3028;
const RTTI_TES_OBJECT_REFR: u32 = 0x0118_41cc;
const RTTI_TES_KEY: u32 = 0x0118_41b4;
const RTTI_MEDIA_LOCATION_CONTROLLER: u32 = 0x0118_418c;
/// The log (overridable: by address).
const LOG_MESSAGE: u32 = 0x005b_5e40;
/// A form's id (+0xc), type (+4) and a reference's extra list (+0x44).
const FORM_ID: u32 = 0x0c;
const FORM_TYPE: u32 = 0x04;
const REFERENCE_EXTRA: u32 = 0x44;
/// The callees by address.
const ACTOR_BASE_GET_HEALTH: u32 = 0x005f_0b00;
const DOOR_TELEPORT_DATA_INIT_ITEM: u32 = 0x0043_a590;
const DATA_HANDLER: u32 = 0x011c_3f2c;
const DATA_HANDLER_GET_REPUTATION: u32 = 0x0046_1630;
const MAP_MARKER_SET_REPUTATION: u32 = 0x0043_7730;
const CHECK_ENABLE_PARENT_LOOP: u32 = 0x0056_aac0;
const ENABLE_PARENT_ADD_CHILD: u32 = 0x0041_dcd0;
const LINKED_REF_ADD_CHILD: u32 = 0x0041_e530;
const DECAL_REFS_INIT_ITEM: u32 = 0x0043_3db0;
const REFLECTOR_REFS_INIT_ITEM: u32 = 0x0043_3ed0;
const LIT_WATER_REFS_INIT_ITEM: u32 = 0x0043_4050;
const REFERENCE_GET_INTERIOR: u32 = 0x0057_5d10;
const GET_PATROL_REF_DATA: u32 = 0x0041_fe90;
const PATROL_REF_DATA_INIT_ITEM: u32 = 0x0067_c6c0;
const GET_OCCLUSION_PLANE: u32 = 0x0042_2120;
const PORTAL_ADD_REFERENCE: u32 = 0x0042_0ce0;
const IMPACT_SWAP_INIT_ITEM: u32 = 0x0058_f210;
const ADD_ACTIVATE_REF_CHILD: u32 = 0x0041_edd0;
const LIST_REMOVE_AFTER: u32 = 0x0090_5330;
const LIST_REMOVE_HEAD: u32 = 0x0063_f7b0;
const STRING_LENGTH: u32 = 0x0040_48e0;
/// The messages.
const MESSAGE_RAGDOLL_ON_LIVE_ACTOR: u32 = 0x0101_4b88;
const MESSAGE_LOCK_KEY_MISSING: u32 = 0x0101_5060;
const MESSAGE_ENABLE_PARENT_MISSING: u32 = 0x0101_4c90;
const MESSAGE_ENABLE_PARENT_LOOP: u32 = 0x0101_4c50;
const MESSAGE_LINKED_REF_MISSING: u32 = 0x0101_4b28;
const MESSAGE_EMPTY_DECALS: u32 = 0x0101_4890;
const MESSAGE_MULTIBOUND_REF_MISSING: u32 = 0x0101_46b8;
const MESSAGE_EMPTY_REFLECTOR_REFS: u32 = 0x0101_4684;
const MESSAGE_EMITTANCE_SOURCE_MISSING: u32 = 0x0101_4728;
const MESSAGE_RADIO_POSITION_MISSING: u32 = 0x0101_50f8;
const MESSAGE_RADIO_POSITION_INTERIOR: u32 = 0x0101_50b0;
const MESSAGE_PLANE_REF_MISSING: u32 = 0x0101_4ab8;
const MESSAGE_NO_OCCLUSION_PLANE: u32 = 0x0101_4a80;
const MESSAGE_PORTAL_REF_MISSING: u32 = 0x0101_4a18;
const MESSAGE_PORTAL_ROOMS_SAME: u32 = 0x0101_49c0;
const MESSAGE_PORTAL_ROOMS_NULL: u32 = 0x0101_4984;
const MESSAGE_ROOM_REF_MISSING: u32 = 0x0101_4920;
const MESSAGE_EMPTY_LIT_WATER: u32 = 0x0101_4658;
const MESSAGE_CONTROLLER_MISSING: u32 = 0x0101_5000;
const MESSAGE_ACTIVATE_REF_MISSING: u32 = 0x0101_48ec;
const MESSAGE_EMPTY_ACTIVATE_PARENT: u32 = 0x0101_48b8;

/// The arms that only swap the id at +0xc for its form, removing the extra data when there is
/// none: (`__RTDynamicCast` target, 0 for none; the message with the id).
fn form_fixup(kind: u8) -> Option<(u32, u32)> {
    Some(match kind {
        // EXTRA_WATERTYPE, EXTRA_CELLMUSICTYPE, EXTRA_CLIMATE
        0x03 => (0x0118_4118, 0x0101_4db0),
        0x07 => (0x0118_40fc, 0x0101_4d60),
        0x08 => (0x0118_4170, 0x0101_4eb8),
        // EXTRA_PACKAGESTARTLOC (the cell) and EXTRA_OWNERSHIP (the owner): not cast.
        0x18 => (0, 0x0101_4cf0),
        0x21 => (0, 0x0101_4fa8),
        // EXTRA_GLOBAL, EXTRA_TELEPORTMARKER, EXTRA_MERCHANTCONTAINER, EXTRA_POISON, EXTRA_XTARGET
        0x22 => (0x0118_3ad4, 0x0101_4f50),
        0x3b => (RTTI_TES_OBJECT_REFR, 0x0101_4be0),
        0x3c => (RTTI_TES_OBJECT_REFR, 0x0101_4830),
        0x3f => (0x0118_30cc, 0x0101_4f08),
        0x44 => (RTTI_TES_OBJECT_REFR, 0x0101_47d8),
        // EXTRA_IMAGESPACE, EXTRA_AMMO, EXTRA_ENCOUNTERZONE, EXTRA_CELL_ACOUSTIC_SPACE
        0x59 => (0x0118_4154, 0x0101_4e60),
        0x6e => (0x0118_40c4, 0x0101_4620),
        0x74 => (0x0118_40dc, 0x0101_4788),
        0x81 => (0x0118_4134, 0x0101_4e00),
        _ => return None,
    })
}

/// A form's id, in place (`0084e3a0`'s `MOV EAX,[ECX+0xc]`).
#[inline]
fn id_of(e: &Engine, form: u32) -> u32 {
    word_at(e, form, FORM_ID)
}

/// A reference's extra list, in place (`005d43c0`'s `LEA EAX,[ECX+0x44]`).
#[inline]
fn extra_list_of(reference: u32) -> u32 {
    reference.wrapping_add(REFERENCE_EXTRA)
}

/// A simple list that holds nothing, in place (`008256d0`: both words zero).
#[inline]
fn list_empty(e: &Engine, list: u32) -> bool {
    word_at(e, list, 4) == 0 && word_at(e, list, 0) == 0
}

/// A log line with its words (by address: overridable).
fn note(e: &mut Engine, message: u32, words: &[u32]) {
    match *words {
        [] => e.call(LOG_MESSAGE, &args![message]),
        [a] => e.call(LOG_MESSAGE, &args![message, a]),
        [a, b, ..] => e.call(LOG_MESSAGE, &args![message, a, b]),
    };
}

/// The id as the file stores it made the load order's (`AddCompileIndex` on a local) and the form
/// it names, cast to `target` when that is not 0. Answers the id and the form.
fn resolve(e: &mut Engine, file: u32, stored: u32, target: u32) -> (u32, u32) {
    let id = e.with_stack(4, |e, local| {
        e.mem.set_u32(local.addr(), stored);
        e.call(ADD_COMPILE_INDEX, &args![local, file]);
        e.mem.u32(local.addr())
    });
    let form = e.call(LOOKUP_FORM, &args![id]).u32();
    let form = if target == 0 {
        form
    } else {
        e.call(
            RT_DYNAMIC_CAST,
            &args![form, 0u32, RTTI_TES_FORM, target, 0u32],
        )
        .u32()
    };
    (id, form)
}

/// The extra data taken out of the list and deleted (`RemoveExtra(extra, true)`).
#[inline]
fn drop_extra(e: &mut Engine, list: u32, extra: u32) {
    list_remove(e, list, extra, true);
}

// Typed from 00416be0
/// `ExtraDataList::InitItem` (Xbox PDB): walks the chain under the extra data lock, reading each
/// extra data's next before its arm runs (an arm may remove it). The compiler's exception-unwinding
/// frame is not translated.
pub fn init_item(e: &mut Engine, this: u32, reference: u32) {
    take_lock(e, 0);
    let mut current = head_of(e, this);
    let file = e
        .call(TES_FORM_GET_FILE, &args![reference, 0xffff_ffffu32])
        .u32();
    while current != 0 {
        let next = next_of(e, current);
        let kind = kind_of(e, current);
        fix_up(e, this, reference, file, current, kind);
        current = next;
    }
    give_lock(e);
}

/// One arm of `InitItem`'s switch (the jump table at `0041809c`).
fn fix_up(e: &mut Engine, this: u32, reference: u32, file: u32, extra: u32, kind: u8) {
    if let Some((target, message)) = form_fixup(kind) {
        let stored = word_at(e, extra, EXTRA_PAYLOAD);
        let (id, form) = resolve(e, file, stored, target);
        e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), form);
        if form == 0 {
            note(e, message, &[id]);
            drop_extra(e, this, extra);
        }
        return;
    }
    match kind {
        // EXTRA_RAGDOLLDATA: only logged on a living actor.
        0x14 => {
            if e.vcall(reference, 0x100, &args![]).bool()
                && typed_call!(FAMILY, 0x0041_81e0, access::save_form(e, reference)) != 0
            {
                let base = typed_call!(FAMILY, 0x0041_81e0, access::save_form(e, reference));
                let health = e.call(ACTOR_BASE_GET_HEALTH, &args![base]).i32();
                if health > 0 {
                    let id = id_of(e, reference);
                    note(e, MESSAGE_RAGDOLL_ON_LIVE_ACTOR, &[id]);
                }
            }
        }
        // EXTRA_LOCK: the key becomes its TESKey, or the lock goes.
        0x2a => {
            let lock_data = word_at(e, extra, EXTRA_PAYLOAD);
            let key = word_at(e, lock_data, 4);
            if key != 0 {
                let (id, resolved) = resolve(e, file, key, RTTI_TES_KEY);
                e.mem.set_u32(lock_data.wrapping_add(4), resolved);
                if resolved == 0 {
                    note(e, MESSAGE_LOCK_KEY_MISSING, &[id]);
                    drop_extra(e, this, extra);
                }
            }
        }
        // EXTRA_TELEPORT: the door's own fix-up decides.
        0x2b => {
            let teleport = word_at(e, extra, EXTRA_PAYLOAD);
            if !e
                .call(DOOR_TELEPORT_DATA_INIT_ITEM, &args![teleport, reference])
                .bool()
            {
                drop_extra(e, this, extra);
            }
        }
        0x2c => fix_up_map_marker(e, word_at(e, extra, EXTRA_PAYLOAD)),
        0x37 => fix_up_enable_parent(e, this, reference, file, extra),
        // EXTRA_LINKED_REF: the linked reference, told about this one.
        0x51 => {
            let stored = word_at(e, extra, EXTRA_PAYLOAD);
            let (id, linked) = resolve(e, file, stored, RTTI_TES_OBJECT_REFR);
            e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), linked);
            if linked == 0 {
                note(e, MESSAGE_LINKED_REF_MISSING, &[id]);
                drop_extra(e, this, extra);
            } else {
                e.call(
                    LINKED_REF_ADD_CHILD,
                    &args![extra_list_of(linked), reference],
                );
            }
        }
        0x53 => fix_up_activate_ref(e, this, reference, file, extra),
        // EXTRA_DECAL_REFS, EXTRA_REFLECTOR_REFS, EXTRA_LIT_WATER_REFS: their own fix-up, then gone
        // when left empty.
        0x57 => fix_up_ref_list(
            e,
            this,
            reference,
            extra,
            DECAL_REFS_INIT_ITEM,
            MESSAGE_EMPTY_DECALS,
        ),
        0x66 => fix_up_ref_list(
            e,
            this,
            reference,
            extra,
            REFLECTOR_REFS_INIT_ITEM,
            MESSAGE_EMPTY_REFLECTOR_REFS,
        ),
        0x85 => fix_up_ref_list(
            e,
            this,
            reference,
            extra,
            LIT_WATER_REFS_INIT_ITEM,
            MESSAGE_EMPTY_LIT_WATER,
        ),
        // EXTRA_MULTIBOUND_REF: not the reference itself.
        0x63 => {
            let stored = word_at(e, extra, EXTRA_PAYLOAD);
            let (id, bound) = resolve(e, file, stored, RTTI_TES_OBJECT_REFR);
            let bound = if bound == reference { 0 } else { bound };
            e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), bound);
            if bound == 0 {
                let reference_id = id_of(e, reference);
                note(e, MESSAGE_MULTIBOUND_REF_MISSING, &[id, reference_id]);
                drop_extra(e, this, extra);
            }
        }
        // EXTRA_EMITTANCE_SOURCE: a light (form type 0x1e) or an activator (0x37) only.
        0x67 => {
            let stored = word_at(e, extra, EXTRA_PAYLOAD);
            let (id, source) = resolve(e, file, stored, 0);
            let kept =
                source != 0 && matches!(e.mem.u8(source.wrapping_add(FORM_TYPE)), 0x1e | 0x37);
            e.mem.set_u32(
                extra.wrapping_add(EXTRA_PAYLOAD),
                if kept { source } else { 0 },
            );
            if !kept {
                note(e, MESSAGE_EMITTANCE_SOURCE_MISSING, &[id]);
                drop_extra(e, this, extra);
            }
        }
        // EXTRA_RADIO_DATA: the RADIO_DATA's position reference (+0xc of the record at +0xc).
        0x68 => {
            let slot = extra.wrapping_add(EXTRA_PAYLOAD + 0x0c);
            let stored = e.mem.u32(slot);
            if stored != 0 {
                let radio_file = e
                    .call(TES_FORM_GET_FILE, &args![reference, 0xffff_ffffu32])
                    .u32();
                let (id, position) = resolve(e, radio_file, stored, RTTI_TES_OBJECT_REFR);
                e.mem.set_u32(slot, position);
                if position == 0 {
                    note(e, MESSAGE_RADIO_POSITION_MISSING, &[id]);
                } else if e.call(REFERENCE_GET_INTERIOR, &args![position]).bool() {
                    note(e, MESSAGE_RADIO_POSITION_INTERIOR, &[id]);
                }
            }
        }
        // EXTRA_PATROL_REF_DATA
        0x6f => {
            let patrol = e.call(GET_PATROL_REF_DATA, &args![this]).u32();
            if patrol != 0 {
                e.call(PATROL_REF_DATA_INIT_ITEM, &args![patrol, reference]);
            }
        }
        0x76 => fix_up_plane_refs(e, this, file, extra),
        0x77 => fix_up_portal_refs(e, this, reference, file, extra),
        0x7b => fix_up_rooms(e, this, file, extra),
        // EXTRA_IMPACTSWAP
        0x8c => {
            let swap = word_at(e, extra, EXTRA_PAYLOAD);
            if swap != 0 {
                e.call(IMPACT_SWAP_INIT_ITEM, &args![swap, reference]);
            }
        }
        // EXTRA_AUDIOMARKER: the controller's id (`0059bb30`, +0x24) through the lookup; the id is
        // stored back (`007037c0`) when the controller exists.
        0x90 => {
            let marker = word_at(e, extra, EXTRA_PAYLOAD);
            let stored = word_at(e, marker, 0x24);
            let controller_file = e
                .call(TES_FORM_GET_FILE, &args![reference, 0xffff_ffffu32])
                .u32();
            let (id, controller) =
                resolve(e, controller_file, stored, RTTI_MEDIA_LOCATION_CONTROLLER);
            if controller != 0 {
                e.mem.set_u32(marker.wrapping_add(0x24), id);
            } else {
                note(e, MESSAGE_CONTROLLER_MISSING, &[id]);
            }
        }
        _ => {}
    }
}

/// EXTRA_MAPMARKER: a travel location that is not visible is cleared (bit 1 and bit 0 of the flags
/// byte +0xc), and the reputation id (+0x10) becomes the data handler's reputation form.
fn fix_up_map_marker(e: &mut Engine, marker: u32) {
    let flags = e.mem.u8(marker.wrapping_add(0x0c));
    if flags & 2 != 0 && flags & 1 == 0 {
        e.mem.set_u8(marker.wrapping_add(0x0c), flags & !2);
    }
    let reputation_id = word_at(e, marker, 0x10);
    if reputation_id != 0 {
        let handler = e.mem.u32(DATA_HANDLER);
        let reputation = e
            .call(DATA_HANDLER_GET_REPUTATION, &args![handler, reputation_id])
            .u32();
        e.call(MAP_MARKER_SET_REPUTATION, &args![marker, reputation]);
    }
}

/// EXTRA_ENABLESTATEPARENT: the parent reference, told about this one when the chain of parents
/// does not loop back to it.
fn fix_up_enable_parent(e: &mut Engine, this: u32, reference: u32, file: u32, extra: u32) {
    let stored = word_at(e, extra, EXTRA_PAYLOAD);
    let (id, parent) = resolve(e, file, stored, RTTI_TES_OBJECT_REFR);
    e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), parent);
    if parent == 0 {
        note(e, MESSAGE_ENABLE_PARENT_MISSING, &[id]);
        drop_extra(e, this, extra);
        return;
    }
    let as_reference = e
        .call(
            RT_DYNAMIC_CAST,
            &args![reference, 0u32, RTTI_TES_FORM, RTTI_TES_OBJECT_REFR, 0u32],
        )
        .u32();
    if as_reference != 0
        && e.call(CHECK_ENABLE_PARENT_LOOP, &args![as_reference])
            .bool()
    {
        e.call(
            ENABLE_PARENT_ADD_CHILD,
            &args![extra_list_of(parent), as_reference],
        );
    } else {
        note(e, MESSAGE_ENABLE_PARENT_LOOP, &[]);
        drop_extra(e, this, extra);
    }
}

/// A list of references with its own fix-up (`init`), removed when it is left empty.
fn fix_up_ref_list(e: &mut Engine, this: u32, reference: u32, extra: u32, init: u32, message: u32) {
    e.call(init, &args![extra, reference]);
    if list_empty(e, extra.wrapping_add(EXTRA_PAYLOAD)) {
        note(e, message, &[]);
        drop_extra(e, this, extra);
    }
}

/// EXTRA_OCCLUSION_PLANE_REF_DATA: four linked references, each linked to the plane of its own list.
/// The id the table holds is not used: each is resolved from 0, as the exe does.
fn fix_up_plane_refs(e: &mut Engine, this: u32, file: u32, extra: u32) {
    let table = word_at(e, extra, EXTRA_PAYLOAD);
    for index in 0..4u32 {
        let slot = table.wrapping_add(index * 4);
        let (id, linked) = resolve(e, file, 0, RTTI_TES_OBJECT_REFR);
        e.mem.set_u32(slot, linked);
        if id != 0 && linked == 0 {
            note(e, MESSAGE_PLANE_REF_MISSING, &[id]);
            drop_extra(e, this, extra);
            break;
        }
        let plane = e.call(GET_OCCLUSION_PLANE, &args![this]).u32();
        if plane == 0 {
            note(e, MESSAGE_NO_OCCLUSION_PLANE, &[]);
        } else {
            let linked = e.mem.u32(slot);
            if linked != 0 {
                let linked_plane = e
                    .call(GET_OCCLUSION_PLANE, &args![extra_list_of(linked)])
                    .u32();
                typed_call!(
                    FAMILY,
                    0x0041_81c0,
                    records::plane_set_linked(e, plane, index, linked_plane)
                );
            }
        }
    }
}

/// EXTRA_PORTAL_REF_DATA: two linked references (rooms), which must differ, each told about this
/// portal. The table is read through the extra data each time, also after an arm removed it (as the
/// exe does).
fn fix_up_portal_refs(e: &mut Engine, this: u32, reference: u32, file: u32, extra: u32) {
    for index in 0..2u32 {
        let stored = word_at(e, word_at(e, extra, EXTRA_PAYLOAD), index * 4);
        let (id, linked) = resolve(e, file, stored, RTTI_TES_OBJECT_REFR);
        let table = word_at(e, extra, EXTRA_PAYLOAD);
        e.mem.set_u32(table.wrapping_add(index * 4), linked);
        if id != 0 && linked == 0 {
            note(e, MESSAGE_PORTAL_REF_MISSING, &[id]);
            drop_extra(e, this, extra);
            break;
        }
    }
    let table = word_at(e, extra, EXTRA_PAYLOAD);
    let first = word_at(e, table, 0);
    if first == word_at(e, table, 4) {
        if first != 0 {
            let first_id = id_of(e, first);
            let reference_id = id_of(e, reference);
            note(e, MESSAGE_PORTAL_ROOMS_SAME, &[reference_id, first_id]);
            let table = word_at(e, extra, EXTRA_PAYLOAD);
            e.mem.set_u32(table.wrapping_add(4), 0);
        } else {
            let reference_id = id_of(e, reference);
            note(e, MESSAGE_PORTAL_ROOMS_NULL, &[reference_id]);
        }
    }
    for index in 0..2u32 {
        let linked = word_at(e, word_at(e, extra, EXTRA_PAYLOAD), index * 4);
        if linked != 0 {
            e.call(
                PORTAL_ADD_REFERENCE,
                &args![extra_list_of(linked), reference],
            );
        }
    }
}

/// EXTRA_ROOM_REF_DATA: the linked rooms of the list at +8 of the room data, each item replaced by
/// its room (`00726c60` in place: the item is stored when it is not null).
fn fix_up_rooms(e: &mut Engine, this: u32, file: u32, extra: u32) {
    let mut node = word_at(e, extra, EXTRA_PAYLOAD).wrapping_add(8);
    while node != 0 {
        let stored = e.mem.u32(node);
        if stored == 0 {
            break;
        }
        let (id, room) = resolve(e, file, stored, RTTI_TES_OBJECT_REFR);
        if id != 0 && room == 0 {
            note(e, MESSAGE_ROOM_REF_MISSING, &[id]);
            drop_extra(e, this, extra);
            break;
        }
        if id != 0 {
            e.mem.set_u32(node, room);
        }
        node = word_at(e, node, 4);
    }
}

/// EXTRA_ACTIVATE_REF: the reference of each REF_ACTIVATE_DATA of the parent list; an entry whose
/// reference is gone is unlinked and its data deleted, the others' lists told about this reference;
/// the extra data goes when it is left with no entries, no flags and no text.
fn fix_up_activate_ref(e: &mut Engine, this: u32, reference: u32, file: u32, extra: u32) {
    let mut node = extra.wrapping_add(EXTRA_PAYLOAD);
    let mut previous = 0u32;
    while node != 0 && !list_empty(e, node) {
        let data = e.mem.u32(node);
        let stored = e.mem.u32(data);
        let (id, activator) = resolve(e, file, stored, RTTI_TES_OBJECT_REFR);
        e.mem.set_u32(data, activator);
        if activator == 0 {
            note(e, MESSAGE_ACTIVATE_REF_MISSING, &[id]);
            if previous != 0 {
                e.with_stack(4, |e, slot| {
                    e.mem.set_u32(slot.addr(), data);
                    e.call(LIST_REMOVE_AFTER, &args![previous, slot]);
                });
                node = word_at(e, previous, 4);
            } else {
                e.call(LIST_REMOVE_HEAD, &args![node]);
            }
            e.call(OPERATOR_DELETE, &args![data]);
        } else {
            e.call(
                ADD_ACTIVATE_REF_CHILD,
                &args![extra_list_of(activator), reference],
            );
            previous = node;
            node = word_at(e, node, 4);
        }
    }
    if list_empty(e, extra.wrapping_add(EXTRA_PAYLOAD))
        && e.mem.u8(extra.wrapping_add(0x14)) == 0
        && e.call(STRING_LENGTH, &args![extra.wrapping_add(0x18)])
            .u32()
            == 0
    {
        note(e, MESSAGE_EMPTY_ACTIVATE_PARENT, &[]);
        drop_extra(e, this, extra);
    }
}
