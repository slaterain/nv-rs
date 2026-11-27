//! The part 5 file's getters and setters (`0042dd90` to `0042f420`, `004300c0`, `00555bc0`,
//! `00569140`): the saved state's drop, the hot key 0x4a, the info general topic 0x4d with its menu
//! topic's two helpers, the talking actor 0x55, the model swap 0x5b, the navmesh portal 0x5a, the
//! weapon mod flags 0x8d and the is-modding flag 0x8e, the faction changes 0x5e, the dismembered
//! limbs 0x5f, the actor cause 0x60, the combat style 0x69, the ammo 0x6e, the say-to topic info
//! 0x75, the topic infos said once a day 0x73, the water zone map 0x7e, the ignored by sandbox flag
//! 0x80, the patrol reference in use 0x88, the follower swim breadcrumbs 0x8b, the animation's
//! constructor 0x10, and the two getters over a list held in an object.
//!
//! Every setter is the compiler's one pattern: the first extra data of its type (`00410220`, the
//! dispatch family's) updated, or a new one built (`operator new`, its class's constructor) and
//! added (`AddExtra`, the cell references family's), a failed allocation's null extra data stored
//! to and added as the exe does it. The file's own classes' constructors (the model swap, the
//! weapon mod flags, the is-modding flag, the combat style, the ammo, the animation) and its
//! one-instruction leaves (the byte store and OR at +0xc, the weapon mod flags' byte, a form's
//! flags word and its bit test, the say-to topic info's topic store, the water zone map's counts,
//! the navmesh portal record's copy, a list at +0x28 or +0x44 of its holder) are done in place; the
//! actor cause's handle assignment (`0042f780`), the removers and getters this file's setters use
//! and the family's other constructors are typed calls. The other units' constructors, the
//! said-once list's add, the water zone map's update, key and lookup, the menu topic's forward and
//! the simple list's count are called by address.

use super::setters::remove_kind;
use super::{
    access, arrays, cell_refs, classes, dispatch, extra_init, first_of_kind, followers, list_add,
    list_remove, new_object, records, word_at, EXTRA_PAYLOAD, FAMILY,
};
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// The extra data types (`cEtype`, Xbox PDB `EXTRA_DATA_TYPE`) of this file.
const KIND_ANIM: u8 = 0x10;
const KIND_USED_MARKERS: u8 = 0x12;
const KIND_HOT_KEY: u8 = 0x4a;
const KIND_INFO_GENERAL_TOPIC: u8 = 0x4d;
const KIND_TALKING_ACTOR: u8 = 0x55;
const KIND_NAVMESH_PORTAL: u8 = 0x5a;
const KIND_MODEL_SWAP: u8 = 0x5b;
const KIND_FACTION_CHANGES: u8 = 0x5e;
const KIND_DISMEMBERED_LIMBS: u8 = 0x5f;
const KIND_ACTOR_CAUSE: u8 = 0x60;
const KIND_COMBAT_STYLE: u8 = 0x69;
const KIND_AMMO: u8 = 0x6e;
const KIND_SAY_TOPIC_INFO_ONCE_A_DAY: u8 = 0x73;
const KIND_SAY_TO_TOPIC_INFO: u8 = 0x75;
const KIND_WATER_ZONE_MAP: u8 = 0x7e;
const KIND_IGNORED_BY_SANDBOX: u8 = 0x80;
const KIND_PATROL_REF_IN_USE: u8 = 0x88;
const KIND_FOLLOWER_SWIM_BREADCRUMBS: u8 = 0x8b;
const KIND_WEAPON_MOD_FLAGS: u8 = 0x8d;
const KIND_WEAPON_IS_MODDING: u8 = 0x8e;

/// The vtables of the file's own classes.
const VTABLE_EXTRA_MODEL_SWAP: u32 = 0x0101_5980;
const VTABLE_EXTRA_AMMO: u32 = 0x0101_5998;
const VTABLE_EXTRA_WEAPON_MOD_FLAGS: u32 = 0x0101_59a4;
const VTABLE_EXTRA_WEAPON_IS_MODDING: u32 = 0x0101_59bc;
const VTABLE_EXTRA_COMBAT_STYLE: u32 = 0x0101_59c8;
const VTABLE_EXTRA_ANIM: u32 = 0x0101_5b28;

/// Other units' constructors (`thiscall(block)`, or with the stated words): the hot key (its byte
/// zero-extended), the info general topic (the topic), the talking actor (two words), the faction
/// changes, the dismembered limbs (0x30 bytes), the say-to topic info (0x1c bytes; with its info,
/// or empty), the topic infos said once a day (with the first), the water zone map (0x20 bytes),
/// the ignored by sandbox flag (0xc bytes), the patrol reference in use (with its user) and the
/// follower swim breadcrumbs (0x28 bytes).
const HOT_KEY_INIT: u32 = 0x0043_2380;
const INFO_GENERAL_TOPIC_INIT: u32 = 0x0043_2470;
const TALKING_ACTOR_INIT: u32 = 0x0043_6b20;
const FACTION_CHANGES_INIT: u32 = 0x0043_6cb0;
const DISMEMBERED_LIMBS_INIT: u32 = 0x0043_0200;
const SAY_TO_TOPIC_INFO_INIT: u32 = 0x0043_76e0;
const SAY_TO_TOPIC_INFO_INIT_EMPTY: u32 = 0x0043_7690;
const SAID_ONCE_INIT: u32 = 0x0043_7510;
const WATER_ZONE_MAP_INIT: u32 = 0x0043_7750;
const IGNORED_BY_SANDBOX_INIT: u32 = 0x0043_7970;
const PATROL_REF_IN_USE_INIT: u32 = 0x0043_79a0;
const FOLLOWER_SWIM_BREADCRUMBS_INIT: u32 = 0x0043_7c40;

/// Other units' members: the simple list's count (`thiscall(list)`), the menu topic's forward
/// (`thiscall(topic, a, b, c, argument)`), the said-once list's add (`thiscall(list, &value)`), the
/// water zone map's update (`thiscall(extra, zone, flag)`), its key (`thiscall(map)`) and lookup
/// (`thiscall(map, &key, &out, &other)`), and the patrol reference's `GetCanBeUsedBy`
/// (`thiscall(extra, user)`, a byte).
const LIST_COUNT: u32 = 0x005a_e380;
const MENU_TOPIC_FORWARD: u32 = 0x0083_df80;
const SAID_ONCE_ADD: u32 = 0x005a_e3d0;
const WATER_ZONE_MAP_UPDATE: u32 = 0x0043_7850;
const WATER_ZONE_MAP_KEY: u32 = 0x004b_9ba0;
const WATER_ZONE_MAP_LOOKUP: u32 = 0x006b_7f20;
const PATROL_REF_CAN_BE_USED_BY: u32 = 0x0043_7ab0;

// ---- What the file shares --------------------------------------------------------------------

/// The class constructors of this file: `BSExtraData`'s (`0040ec80`, in place) with `kind`, then
/// `vtable`.
#[inline]
fn extra_with_vtable(e: &mut Engine, this: u32, kind: u8, vtable: u32) {
    extra_init(e, this, kind);
    e.mem.set_u32(this, vtable);
}

/// The removers by element: the first extra data of `kind` deleted (`RemoveExtra` with its destroy
/// flag), if the list has one.
#[inline]
fn remove_found(e: &mut Engine, list: u32, kind: u8) {
    let extra = first_of_kind(e, list, kind);
    if extra != 0 {
        list_remove(e, list, extra, true);
    }
}

/// A new extra data of `size` bytes from another unit's constructor at `construct` (`thiscall`,
/// with `words` after the block), added to the list (a null one too when the allocation failed).
/// Answers it.
#[inline]
fn add_built(e: &mut Engine, list: u32, size: u32, construct: u32, words: &[u32]) -> u32 {
    let extra = new_object(e, size, |e, block| {
        let mut all = Vec::with_capacity(words.len() + 1);
        all.push(block);
        all.extend_from_slice(words);
        e.call(construct, &all).u32()
    });
    list_add(e, list, extra);
    extra
}

/// The flag setters by element: with `wanted` and no extra data of `kind`, one built by `build`
/// added; without `wanted` and one in the list, it deleted.
#[inline]
fn flag_by_element(
    e: &mut Engine,
    list: u32,
    kind: u8,
    wanted: bool,
    build: impl FnOnce(&mut Engine),
) {
    let extra = first_of_kind(e, list, kind);
    if extra == 0 {
        if wanted {
            build(e);
        }
    } else if !wanted {
        list_remove(e, list, extra, true);
    }
}

/// The getters of a word: the word at `offset` of the first extra data of `kind`, or 0.
#[inline]
fn word_of(e: &mut Engine, list: u32, kind: u8, offset: u32) -> u32 {
    match first_of_kind(e, list, kind) {
        0 => 0,
        extra => word_at(e, extra, offset),
    }
}

/// The actor cause's handle at +0xc of `extra` assigned `cause` (`0042ea00` in place: the typed
/// `NiPointer<ActorCause>::operator=`, `0042f780`).
#[inline]
fn assign_cause(e: &mut Engine, extra: u32, cause: u32) {
    let handle = extra.wrapping_add(EXTRA_PAYLOAD);
    typed_call!(
        FAMILY,
        0x0042_f780,
        arrays::actor_cause_assign(e, handle, cause)
    );
}

// ---- The functions --------------------------------------------------------------------------

// Typed from 0042dd90
/// Drops a reference's saved state before it is reset from `form` (`thiscall(list, form)`, its only
/// caller `005629a0`): the saved animation, the saved havok data and the last finished sequence
/// removed (the family's `00422aa0`, `00422c20`, `00422920`); then, when bit 31 of the form's flags
/// word (+0x2c; `0042ce30` and `004280f0` in place) is set, the used markers (type 0x12) removed by
/// type.
pub fn drop_saved_state(e: &mut Engine, this: u32, form: u32) {
    typed_call!(
        FAMILY,
        0x0042_2aa0,
        followers::remove_saved_animation(e, this)
    );
    typed_call!(
        FAMILY,
        0x0042_2c20,
        followers::remove_saved_havok_data(e, this)
    );
    typed_call!(
        FAMILY,
        0x0042_2920,
        followers::remove_last_finished_sequence(e, this)
    );
    if word_at(e, form, 0x2c) & 0x8000_0000 != 0 {
        remove_kind(e, this, KIND_USED_MARKERS);
    }
}

// Typed from 0042dde0
/// `ExtraDataList::SetHotKey` (Xbox PDB): the byte at +0xc of the type 0x4a extra data set, or one
/// (0x10 bytes, `00432380` given the byte) added.
pub fn set_hot_key(e: &mut Engine, this: u32, hot_key: u8) {
    match first_of_kind(e, this, KIND_HOT_KEY) {
        0 => {
            add_built(e, this, 0x10, HOT_KEY_INIT, &[u32::from(hot_key)]);
        }
        extra => e.mem.set_u8(extra.wrapping_add(EXTRA_PAYLOAD), hot_key),
    }
}

// Typed from 0042de90
/// `ExtraDataList::GetHotKey` (Xbox PDB): the byte at +0xc of the type 0x4a extra data, or 0xff.
pub fn hot_key(e: &mut Engine, this: u32) -> u8 {
    match first_of_kind(e, this, KIND_HOT_KEY) {
        0 => 0xff,
        extra => e.mem.u8(extra.wrapping_add(EXTRA_PAYLOAD)),
    }
}

// Typed from 0042dec0
/// `ExtraDataList::RemoveHotKey` (Xbox PDB): the type 0x4a extra data removed by type.
pub fn remove_hot_key(e: &mut Engine, this: u32) {
    remove_kind(e, this, KIND_HOT_KEY);
}

// Typed from 0042dee0
/// `ExtraDataList::SetInfoGeneralTopic` (Xbox PDB): the menu topic at +0xc of the type 0x4d extra
/// data set, or one (0x10 bytes, `00432470` given the topic) added.
pub fn set_info_general_topic(e: &mut Engine, this: u32, topic: u32) {
    match first_of_kind(e, this, KIND_INFO_GENERAL_TOPIC) {
        0 => {
            add_built(e, this, 0x10, INFO_GENERAL_TOPIC_INIT, &[topic]);
        }
        extra => e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), topic),
    }
}

// Typed from 0042df90
/// The menu topic of the type 0x4d extra data, or 0 (`thiscall(list, argument)`); a topic whose
/// list is empty ([`topic_is_empty`]) is first handed on with `argument` ([`topic_forward`]).
pub fn info_general_topic(e: &mut Engine, this: u32, argument: u32) -> u32 {
    let extra = first_of_kind(e, this, KIND_INFO_GENERAL_TOPIC);
    if extra == 0 {
        return 0;
    }
    let topic = word_at(e, extra, EXTRA_PAYLOAD);
    if topic != 0 && typed_call!(FAMILY, 0x0042_dff0, topic_is_empty(e, topic)) {
        typed_call!(FAMILY, 0x0042_e010, topic_forward(e, topic, argument));
    }
    topic
}

// Typed from 0042dff0
/// Whether the simple list at +0xc of the menu topic has no item (its count, `005ae380`, 0).
pub fn topic_is_empty(e: &mut Engine, this: u32) -> bool {
    e.call(LIST_COUNT, &args![this.wrapping_add(0x0c)]).u32() == 0
}

// Typed from 0042e010
/// The menu topic's forward (`0083df80`) with its words at +0x14, +0x28 and +0x18, then `argument`.
pub fn topic_forward(e: &mut Engine, this: u32, argument: u32) {
    let first = word_at(e, this, 0x14);
    let second = word_at(e, this, 0x28);
    let third = word_at(e, this, 0x18);
    e.call(
        MENU_TOPIC_FORWARD,
        &args![this, first, second, third, argument],
    );
}

// Typed from 0042e040
/// The type 0x4d extra data removed by type.
pub fn remove_info_general_topic(e: &mut Engine, this: u32) {
    remove_kind(e, this, KIND_INFO_GENERAL_TOPIC);
}

// Typed from 0042e060
/// `ExtraDataList::SetTalkingActorExtra` (Xbox PDB): the type 0x55 extra data deleted if there is
/// one, then a new one (0x10 bytes, `00436b20` given both words) added.
pub fn set_talking_actor(e: &mut Engine, this: u32, first: u32, second: u32) {
    remove_found(e, this, KIND_TALKING_ACTOR);
    add_built(e, this, 0x10, TALKING_ACTOR_INIT, &[first, second]);
}

// Typed from 0042e110
/// `ExtraDataList::GetTalkingActorExtra` (Xbox PDB): the type 0x55 extra data, or null.
pub fn talking_actor(e: &mut Engine, this: u32) -> u32 {
    first_of_kind(e, this, KIND_TALKING_ACTOR)
}

// Typed from 0042e130
/// The type 0x55 extra data removed by type.
pub fn remove_talking_actor(e: &mut Engine, this: u32) {
    remove_kind(e, this, KIND_TALKING_ACTOR);
}

// Typed from 0042e150
/// The model swap and its form (+0xc, +0x10) set in the type 0x5b extra data, or one (0x14 bytes,
/// [`model_swap_new`] in place) built with them and added.
pub fn set_model_swap(e: &mut Engine, this: u32, swap: u32, form: u32) {
    match first_of_kind(e, this, KIND_MODEL_SWAP) {
        0 => {
            let extra = new_object(e, 0x14, |e, block| model_swap_new(e, block, swap, form));
            list_add(e, this, extra);
        }
        extra => {
            e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), swap);
            e.mem.set_u32(extra.wrapping_add(0x10), form);
        }
    }
}

// Typed from 0042e210
/// `ExtraModelSwap`'s constructor: type 0x5b, its vtable, the model swap and its form. Answers
/// `this`.
pub fn model_swap_new(e: &mut Engine, this: u32, swap: u32, form: u32) -> u32 {
    extra_with_vtable(e, this, KIND_MODEL_SWAP, VTABLE_EXTRA_MODEL_SWAP);
    e.mem.set_u32(this.wrapping_add(EXTRA_PAYLOAD), swap);
    e.mem.set_u32(this.wrapping_add(0x10), form);
    this
}

// Typed from 0042e250
/// `ExtraDataList::GetModelSwap` (Xbox PDB): the model swap at +0xc of the type 0x5b extra data, or
/// 0.
pub fn model_swap(e: &mut Engine, this: u32) -> u32 {
    word_of(e, this, KIND_MODEL_SWAP, EXTRA_PAYLOAD)
}

// Typed from 0042e280
/// The type 0x5b extra data removed by type.
pub fn remove_model_swap(e: &mut Engine, this: u32) {
    remove_kind(e, this, KIND_MODEL_SWAP);
}

// Typed from 0042e2a0
/// `ExtraDataList::GetNavMeshPortal` (Xbox PDB): the type 0x5a extra data, or null.
pub fn navmesh_portal(e: &mut Engine, this: u32) -> u32 {
    first_of_kind(e, this, KIND_NAVMESH_PORTAL)
}

// Typed from 0042e2c0
/// The navmesh portal's setter: with no type 0x5a extra data, one (0x14 bytes, the family's
/// `00416ad0`) built from the record at +0xc of `portal` (the mesh word and the triangle's 16 bits,
/// handed by value as `0069a690` copies them, in place) and added; one there is kept.
pub fn set_navmesh_portal(e: &mut Engine, this: u32, portal: u32) {
    if first_of_kind(e, this, KIND_NAVMESH_PORTAL) != 0 {
        return;
    }
    let extra = new_object(e, 0x14, |e, block| {
        let mesh = word_at(e, portal, EXTRA_PAYLOAD);
        let triangle = u32::from(e.mem.u16(portal.wrapping_add(0x10)));
        typed_call!(
            FAMILY,
            0x0041_6ad0,
            records::portal_extra_new(e, block, mesh, triangle)
        )
    });
    list_add(e, this, extra);
}

// Typed from 0042e380
/// `ExtraDataList::SetWeaponModSlot` (Xbox PDB): the bits `slot` ORed into the byte at +0xc of the
/// type 0x8d extra data; with none and a nonzero `slot`, one (0x10 bytes, [`weapon_mod_flags_new`]
/// in place) built with `slot`, ORed and added. With none and `slot` 0 the exe ORs at +0xc of the
/// null extra data (address 0xc), as here.
pub fn set_weapon_mod_slot(e: &mut Engine, this: u32, slot: u8) {
    let extra = first_of_kind(e, this, KIND_WEAPON_MOD_FLAGS);
    if extra == 0 && slot != 0 {
        let made = new_object(e, 0x10, |e, block| weapon_mod_flags_new(e, block, slot));
        or_byte_c(e, made, slot);
        list_add(e, this, made);
    } else {
        or_byte_c(e, extra, slot);
    }
}

// Typed from 0042e450
/// `ExtraWeaponModFlags`' constructor (type 0x8d): its vtable and the byte `slot`. Answers `this`.
pub fn weapon_mod_flags_new(e: &mut Engine, this: u32, slot: u8) -> u32 {
    extra_with_vtable(
        e,
        this,
        KIND_WEAPON_MOD_FLAGS,
        VTABLE_EXTRA_WEAPON_MOD_FLAGS,
    );
    e.mem.set_u8(this.wrapping_add(EXTRA_PAYLOAD), slot);
    this
}

// Typed from 0042e480
/// The bits `bits` ORed into the byte at +0xc of the extra data.
#[inline]
pub fn or_byte_c(e: &mut Engine, this: u32, bits: u8) {
    let at = this.wrapping_add(EXTRA_PAYLOAD);
    let byte = e.mem.u8(at);
    e.mem.set_u8(at, byte | bits);
}

// Typed from 0042e4a0
/// The byte at +0xc of the type 0x8d extra data set to `value`; with none, one (0x10 bytes, the
/// family's `0042cc40`) built, set and added.
pub fn set_weapon_mod_flags(e: &mut Engine, this: u32, value: u8) {
    match first_of_kind(e, this, KIND_WEAPON_MOD_FLAGS) {
        0 => {
            let made = new_object(e, 0x10, |e, block| {
                typed_call!(
                    FAMILY,
                    0x0042_cc40,
                    classes::weapon_mod_slots_extra_new(e, block)
                )
            });
            set_byte_c(e, made, value);
            list_add(e, this, made);
        }
        extra => set_byte_c(e, extra, value),
    }
}

// Typed from 0042e560
/// `ExtraDataList::GetWeaponModFlags` (Xbox PDB): the byte at +0xc of the type 0x8d extra data
/// (`00424940`, in place), or 0.
pub fn weapon_mod_flags(e: &mut Engine, this: u32) -> u8 {
    match first_of_kind(e, this, KIND_WEAPON_MOD_FLAGS) {
        0 => 0,
        extra => e.mem.u8(extra.wrapping_add(EXTRA_PAYLOAD)),
    }
}

// Typed from 0042e5a0
/// `ExtraDataList::SetIsModding` (Xbox PDB): the byte at +0xc of the type 0x8e extra data set, or
/// one (0x10 bytes, [`is_modding_new`] in place) built with it and added.
pub fn set_is_modding(e: &mut Engine, this: u32, modding: u8) {
    match first_of_kind(e, this, KIND_WEAPON_IS_MODDING) {
        0 => {
            let made = new_object(e, 0x10, |e, block| is_modding_new(e, block, modding));
            list_add(e, this, made);
        }
        extra => set_byte_c(e, extra, modding),
    }
}

// Typed from 0042e650
/// The is-modding extra data's constructor (type 0x8e): its vtable and the byte. Answers `this`.
pub fn is_modding_new(e: &mut Engine, this: u32, modding: u8) -> u32 {
    extra_with_vtable(
        e,
        this,
        KIND_WEAPON_IS_MODDING,
        VTABLE_EXTRA_WEAPON_IS_MODDING,
    );
    e.mem.set_u8(this.wrapping_add(EXTRA_PAYLOAD), modding);
    this
}

// Typed from 0042e680
/// The byte at +0xc of the extra data set to `value`.
#[inline]
pub fn set_byte_c(e: &mut Engine, this: u32, value: u8) {
    e.mem.set_u8(this.wrapping_add(EXTRA_PAYLOAD), value);
}

// Typed from 0042e6a0
/// The is-modding extra data's scalar deleting destructor: [`is_modding_drop`] in place, then
/// `operator delete` for bit 0 of `flags`. Answers `this`.
pub fn is_modding_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    is_modding_drop(e, this);
    super::free_when(e, this, flags)
}

// Typed from 0042e6d0
/// The is-modding extra data's destructor: its vtable, then `BSExtraData`'s (`0040ecb0`, the cell
/// references family's).
pub fn is_modding_drop(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, VTABLE_EXTRA_WEAPON_IS_MODDING);
    typed_call!(
        cell_refs::FAMILY,
        0x0040_ecb0,
        cell_refs::extra_list::extra_data_drop(e, this)
    );
}

// Typed from 0042e6f0
/// `ExtraDataList::RemoveIsModding` (Xbox PDB): the type 0x8e extra data deleted, if there is one.
pub fn remove_is_modding(e: &mut Engine, this: u32) {
    remove_found(e, this, KIND_WEAPON_IS_MODDING);
}

// Typed from 0042e730
/// `ExtraDataList::RemoveNavMeshPortal` (Xbox PDB): the type 0x5a extra data deleted, if there is
/// one.
pub fn remove_navmesh_portal(e: &mut Engine, this: u32) {
    remove_found(e, this, KIND_NAVMESH_PORTAL);
}

// Typed from 0042e760
/// A type 0x5e extra data (the faction changes; 0x10 bytes, `00436cb0`) added when the list has
/// none.
pub fn add_faction_changes(e: &mut Engine, this: u32) {
    if first_of_kind(e, this, KIND_FACTION_CHANGES) == 0 {
        add_built(e, this, 0x10, FACTION_CHANGES_INIT, &[]);
    }
}

// Typed from 0042e800
/// The type 0x5e extra data, or null.
pub fn faction_changes(e: &mut Engine, this: u32) -> u32 {
    first_of_kind(e, this, KIND_FACTION_CHANGES)
}

// Typed from 0042e820
/// `ExtraDataList::AddDismembermentExtra` (Xbox PDB): the type 0x5f extra data, one (0x30 bytes,
/// `00430200`) built and added first when the list has none.
pub fn add_dismemberment(e: &mut Engine, this: u32) -> u32 {
    match first_of_kind(e, this, KIND_DISMEMBERED_LIMBS) {
        0 => add_built(e, this, 0x30, DISMEMBERED_LIMBS_INIT, &[]),
        extra => extra,
    }
}

// Typed from 0042e8c0
/// `ExtraDataList::GetDismembermentExtra` (Xbox PDB): the type 0x5f extra data, or null.
pub fn dismemberment(e: &mut Engine, this: u32) -> u32 {
    first_of_kind(e, this, KIND_DISMEMBERED_LIMBS)
}

// Typed from 0042e8e0
/// `ExtraDataList::RemoveDismembermentExtra` (Xbox PDB): the type 0x5f extra data deleted, if there
/// is one.
pub fn remove_dismemberment(e: &mut Engine, this: u32) {
    remove_found(e, this, KIND_DISMEMBERED_LIMBS);
}

// Typed from 0042e910
/// The type 0x60 extra data (the actor cause), or null.
pub fn actor_cause(e: &mut Engine, this: u32) -> u32 {
    first_of_kind(e, this, KIND_ACTOR_CAUSE)
}

// Typed from 0042e930
/// The actor cause's setter: a null `cause` removes the type 0x60 extra data by type; otherwise the
/// cause is assigned to the handle of the one there, or of a new one (0x10 bytes, the family's
/// `0042ca90`) added after.
pub fn set_actor_cause(e: &mut Engine, this: u32, cause: u32) {
    if cause == 0 {
        remove_kind(e, this, KIND_ACTOR_CAUSE);
        return;
    }
    match first_of_kind(e, this, KIND_ACTOR_CAUSE) {
        0 => {
            let made = new_object(e, 0x10, |e, block| {
                typed_call!(
                    FAMILY,
                    0x0042_ca90,
                    classes::actor_cause_extra_new(e, block)
                )
            });
            assign_cause(e, made, cause);
            list_add(e, this, made);
        }
        extra => assign_cause(e, extra, cause),
    }
}

// Typed from 0042ea00
/// The handle at +0xc of the actor cause extra data assigned `cause` (`0042f780`).
pub fn actor_cause_assign_at(e: &mut Engine, this: u32, cause: u32) {
    assign_cause(e, this, cause);
}

// Typed from 0042ea20
/// The combat style at +0xc of the type 0x69 extra data, or 0.
pub fn combat_style(e: &mut Engine, this: u32) -> u32 {
    word_of(e, this, KIND_COMBAT_STYLE, EXTRA_PAYLOAD)
}

// Typed from 0042ea50
/// The combat style's setter: a null style removes the type 0x69 extra data by type; otherwise it
/// is stored in the one there, or a new one (0x10 bytes, [`combat_style_new`] in place) built with
/// it is added.
pub fn set_combat_style(e: &mut Engine, this: u32, style: u32) {
    if style == 0 {
        remove_kind(e, this, KIND_COMBAT_STYLE);
        return;
    }
    match first_of_kind(e, this, KIND_COMBAT_STYLE) {
        0 => {
            let made = new_object(e, 0x10, |e, block| combat_style_new(e, block, style));
            list_add(e, this, made);
        }
        extra => e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), style),
    }
}

// Typed from 0042eb10
/// `ExtraCombatStyle`'s constructor (type 0x69): its vtable and the style. Answers `this`.
pub fn combat_style_new(e: &mut Engine, this: u32, style: u32) -> u32 {
    extra_with_vtable(e, this, KIND_COMBAT_STYLE, VTABLE_EXTRA_COMBAT_STYLE);
    e.mem.set_u32(this.wrapping_add(EXTRA_PAYLOAD), style);
    this
}

// Typed from 0042eb40
/// `ExtraDataList::GetAmmo`: the type 0x6e extra data, or null.
pub fn ammo(e: &mut Engine, this: u32) -> u32 {
    first_of_kind(e, this, KIND_AMMO)
}

// Typed from 0042eb60
/// `ExtraDataList::SetAmmo(ammo, count)`: a null ammo or a zero count removes the type 0x6e extra
/// data by type; otherwise both are stored (+0xc, +0x10) in the one there, or a new one (0x14
/// bytes, [`ammo_new`] in place) built with them is added.
pub fn set_ammo(e: &mut Engine, this: u32, ammo: u32, count: u32) {
    if ammo == 0 || count == 0 {
        remove_kind(e, this, KIND_AMMO);
        return;
    }
    match first_of_kind(e, this, KIND_AMMO) {
        0 => {
            let made = new_object(e, 0x14, |e, block| ammo_new(e, block, ammo, count));
            list_add(e, this, made);
        }
        extra => {
            e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), ammo);
            e.mem.set_u32(extra.wrapping_add(0x10), count);
        }
    }
}

// Typed from 0042ec30
/// `ExtraAmmo`'s constructor (type 0x6e): its vtable, the ammo and the count. Answers `this`.
pub fn ammo_new(e: &mut Engine, this: u32, ammo: u32, count: u32) -> u32 {
    extra_with_vtable(e, this, KIND_AMMO, VTABLE_EXTRA_AMMO);
    e.mem.set_u32(this.wrapping_add(EXTRA_PAYLOAD), ammo);
    e.mem.set_u32(this.wrapping_add(0x10), count);
    this
}

// Typed from 0042ec70
/// The say-to topic info's info (+0xc): a null info deletes the type 0x75 extra data
/// ([`remove_say_to`]); otherwise it is stored in the one there, or a new one (0x1c bytes,
/// `004376e0` given the info) is added.
pub fn set_say_to_info(e: &mut Engine, this: u32, info: u32) {
    if info == 0 {
        typed_call!(FAMILY, 0x0042_edb0, remove_say_to(e, this));
        return;
    }
    match first_of_kind(e, this, KIND_SAY_TO_TOPIC_INFO) {
        0 => {
            add_built(e, this, 0x1c, SAY_TO_TOPIC_INFO_INIT, &[info]);
        }
        extra => e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), info),
    }
}

// Typed from 0042ed30
/// The say-to topic info's topic (+0x10), or 0.
pub fn say_to_topic(e: &mut Engine, this: u32) -> u32 {
    word_of(e, this, KIND_SAY_TO_TOPIC_INFO, 0x10)
}

// Typed from 0042ed70
/// The say-to topic info's info (+0xc), or 0.
pub fn say_to_info(e: &mut Engine, this: u32) -> u32 {
    word_of(e, this, KIND_SAY_TO_TOPIC_INFO, EXTRA_PAYLOAD)
}

// Typed from 0042edb0
/// `ExtraDataList::RemoveSayToInfoExtra` (Xbox PDB): the type 0x75 extra data deleted, if there is
/// one.
pub fn remove_say_to(e: &mut Engine, this: u32) {
    remove_found(e, this, KIND_SAY_TO_TOPIC_INFO);
}

// Typed from 0042ede0
/// `ExtraDataList::GetSayToExtra` (Xbox PDB): the type 0x75 extra data, or null.
pub fn say_to_extra(e: &mut Engine, this: u32) -> u32 {
    first_of_kind(e, this, KIND_SAY_TO_TOPIC_INFO)
}

// Typed from 0042ee00
/// The say-to topic info's topic: a null topic deletes the type 0x75 extra data
/// ([`remove_say_to`]); otherwise, with none, an empty one (0x1c bytes, `00437690`) is built, its
/// info zeroed and added; then the topic is stored at +0x10 (`00437730`, in place). A failed
/// allocation's null extra data is written at 0xc and 0x10, as the exe does.
pub fn set_say_to_topic(e: &mut Engine, this: u32, topic: u32) {
    if topic == 0 {
        typed_call!(FAMILY, 0x0042_edb0, remove_say_to(e, this));
        return;
    }
    let mut extra = first_of_kind(e, this, KIND_SAY_TO_TOPIC_INFO);
    if extra == 0 {
        extra = new_object(e, 0x1c, |e, block| {
            e.call(SAY_TO_TOPIC_INFO_INIT_EMPTY, &args![block]).u32()
        });
        e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), 0);
        list_add(e, this, extra);
    }
    e.mem.set_u32(extra.wrapping_add(0x10), topic);
}

// Typed from 0042eec0
/// The say-to topic info's response id (+0x14) set, when the list has one.
pub fn set_say_to_response(e: &mut Engine, this: u32, response: u32) {
    let extra = first_of_kind(e, this, KIND_SAY_TO_TOPIC_INFO);
    if extra != 0 {
        e.mem.set_u32(extra.wrapping_add(0x14), response);
    }
}

// Typed from 0042eef0
/// The say-to topic info's response id (+0x14), or 0.
pub fn say_to_response(e: &mut Engine, this: u32) -> u32 {
    word_of(e, this, KIND_SAY_TO_TOPIC_INFO, 0x14)
}

// Typed from 0042ef20
/// The topic infos said once a day: a null `info` deletes the type 0x73 extra data
/// ([`remove_said_once`]); otherwise the info is added to the list at +0xc of the one there
/// (`005ae3d0`, handed the address of a stack word holding it), or a new one (0x10 bytes,
/// `00437510` given the info) is added.
pub fn add_said_once(e: &mut Engine, this: u32, info: u32) {
    if info == 0 {
        typed_call!(FAMILY, 0x0042_f000, remove_said_once(e, this));
        return;
    }
    match first_of_kind(e, this, KIND_SAY_TOPIC_INFO_ONCE_A_DAY) {
        0 => {
            add_built(e, this, 0x10, SAID_ONCE_INIT, &[info]);
        }
        extra => {
            let list = word_at(e, extra, EXTRA_PAYLOAD);
            e.with_stack(4, |e, slot| {
                e.mem.set_u32(slot.addr(), info);
                e.call(SAID_ONCE_ADD, &args![list, slot]);
            });
        }
    }
}

// Typed from 0042efe0
/// The type 0x73 extra data, or null.
pub fn said_once(e: &mut Engine, this: u32) -> u32 {
    first_of_kind(e, this, KIND_SAY_TOPIC_INFO_ONCE_A_DAY)
}

// Typed from 0042f000
/// The type 0x73 extra data deleted, if there is one.
pub fn remove_said_once(e: &mut Engine, this: u32) {
    remove_found(e, this, KIND_SAY_TOPIC_INFO_ONCE_A_DAY);
}

// Typed from 0042f030
/// The water zone map's update with `zone` and `flag` (`thiscall(list, zone, flag)`): the type 0x7e
/// extra data, one (0x20 bytes, `00437750`) built and added when the list has none, updated
/// (`00437850`); when its map (+0xc) is then empty (its count, `0084e3a0` in place) it is deleted
/// and the answer is 0; otherwise the count of the highest zone (+0x1c, `0084e3a0` at its +4 in
/// place), or 0 without one.
pub fn update_water_zone(e: &mut Engine, this: u32, zone: u32, flag: u8) -> u32 {
    let mut extra = first_of_kind(e, this, KIND_WATER_ZONE_MAP);
    if extra == 0 {
        extra = add_built(e, this, 0x20, WATER_ZONE_MAP_INIT, &[]);
    }
    e.call(WATER_ZONE_MAP_UPDATE, &args![extra, zone, u32::from(flag)]);
    if word_at(e, extra, 0x0c + 0x0c) == 0 {
        list_remove(e, this, extra, true);
        return 0;
    }
    match word_at(e, extra, 0x1c) {
        0 => 0,
        highest => word_at(e, highest, 4 + 0x0c),
    }
}

// Typed from 0042f130
/// Whether the water zone map of the type 0x7e extra data holds `value` with a positive count (the
/// dispatch family's map lookup `00853130` into a stack word that starts at 0); false without one.
pub fn water_zone_has(e: &mut Engine, this: u32, value: u32) -> bool {
    let extra = first_of_kind(e, this, KIND_WATER_ZONE_MAP);
    if extra == 0 {
        return false;
    }
    let count = e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), 0);
        typed_call!(
            dispatch::FAMILY,
            0x0085_3130,
            dispatch::refs::map_lookup(e, Ptr::new(extra.wrapping_add(0x0c)), value, slot.cast())
        );
        e.mem.u32(slot.addr())
    });
    count as i32 > 0
}

// Typed from 0042f180
/// The water zone map's lookup: with a map ([`water_zone_map`]) whose key (`004b9ba0`) is nonzero,
/// the lookup `006b7f20` handed the addresses of the key and of two stack words; its first output
/// word, else 0.
pub fn water_zone_lookup(e: &mut Engine, this: u32) -> u32 {
    let map = typed_call!(FAMILY, 0x0042_f1d0, water_zone_map(e, this));
    if map == 0 {
        return 0;
    }
    let key = e.call(WATER_ZONE_MAP_KEY, &args![map]).u32();
    if key == 0 {
        return 0;
    }
    e.with_stack(12, |e, slots| {
        let at = slots.addr();
        e.mem.set_u32(at, key);
        e.call(
            WATER_ZONE_MAP_LOOKUP,
            &args![map, at, at.wrapping_add(4), at.wrapping_add(8)],
        );
        e.mem.u32(at.wrapping_add(4))
    })
}

// Typed from 0042f1d0
/// `ExtraDataList::QWaterZoneMap` (Xbox PDB): the address of the map at +0xc of the type 0x7e extra
/// data, or 0.
pub fn water_zone_map(e: &mut Engine, this: u32) -> u32 {
    access::record_or_null(e, this, KIND_WATER_ZONE_MAP)
}

// Typed from 0042f200
/// `ExtraDataList::SetIgnoredBySandbox` (Xbox PDB): a nonzero `ignored` with no type 0x80 extra
/// data adds one (0xc bytes, `00437970`); a zero one deletes the one there.
pub fn set_ignored_by_sandbox(e: &mut Engine, this: u32, ignored: u8) {
    flag_by_element(e, this, KIND_IGNORED_BY_SANDBOX, ignored != 0, |e| {
        add_built(e, this, 0xc, IGNORED_BY_SANDBOX_INIT, &[]);
    });
}

// Typed from 0042f2d0
/// Whether the list has a type 0x80 extra data.
pub fn ignored_by_sandbox(e: &mut Engine, this: u32) -> bool {
    first_of_kind(e, this, KIND_IGNORED_BY_SANDBOX) != 0
}

// Typed from 0042f300
/// The patrol reference in use: a nonzero `user` with no type 0x88 extra data adds one (0x10 bytes,
/// `004379a0` given the user); a zero one deletes the one there.
pub fn set_patrol_ref_in_use(e: &mut Engine, this: u32, user: u32) {
    flag_by_element(e, this, KIND_PATROL_REF_IN_USE, user != 0, |e| {
        add_built(e, this, 0x10, PATROL_REF_IN_USE_INIT, &[user]);
    });
}

// Typed from 0042f3d0
/// `ExtraDataList::GetPatrolRefCanBeUsedBy` (Xbox PDB): true without a type 0x88 extra data, else
/// whether its `GetCanBeUsedBy` (`00437ab0`) gives a nonzero byte for `user`.
pub fn patrol_ref_can_be_used_by(e: &mut Engine, this: u32, user: u32) -> bool {
    match first_of_kind(e, this, KIND_PATROL_REF_IN_USE) {
        0 => true,
        extra => e.call(PATROL_REF_CAN_BE_USED_BY, &args![extra, user]).u8() != 0,
    }
}

// Typed from 0042f420
/// `ExtraDataList::GetOrCreateExtraFollowerSwimBreadcrumbs` (Xbox PDB): the type 0x8b extra data,
/// one (0x28 bytes, `00437c40`) built and added first when the list has none.
pub fn follower_swim_breadcrumbs(e: &mut Engine, this: u32) -> u32 {
    match first_of_kind(e, this, KIND_FOLLOWER_SWIM_BREADCRUMBS) {
        0 => add_built(e, this, 0x28, FOLLOWER_SWIM_BREADCRUMBS_INIT, &[]),
        extra => extra,
    }
}

// Typed from 004300c0
/// `ExtraAnim`'s constructor (type 0x10): its vtable and the animation at +0xc. Answers `this`.
pub fn anim_new(e: &mut Engine, this: u32, animation: u32) -> u32 {
    extra_with_vtable(e, this, KIND_ANIM, VTABLE_EXTRA_ANIM);
    e.mem.set_u32(this.wrapping_add(EXTRA_PAYLOAD), animation);
    this
}

// Typed from 00555bc0
/// The seen data's word of the list at +0x28 of the object it is called on (`004610d0` in place;
/// the family's `00421910`).
pub fn seen_data_at_28(e: &mut Engine, this: u32) -> u32 {
    let list = this.wrapping_add(0x28);
    typed_call!(FAMILY, 0x0042_1910, followers::seen_data(e, list))
}

// Typed from 00569140
/// The lock data of the list at +0x44 of the reference it is called on (`005d43c0` in place; the
/// family's `004182b0`).
pub fn lock_data_at_44(e: &mut Engine, this: u32) -> u32 {
    let list = this.wrapping_add(0x44);
    typed_call!(FAMILY, 0x0041_82b0, access::lock_data(e, list))
}
