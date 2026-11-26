//! The part 3 file's second run (`0041f080` to `004213c0`): the decal list's adder (type 0x57), the
//! reflected and reflector references (0x65, 0x66), the water light and lit water references (0x84,
//! 0x85), the primitive (0x6b), the patrol reference data (0x6f), the occlusion plane (0x76), the
//! portal (0x77, 0x78) and room (0x7b, 0x79) reference data with the master room search, the
//! collision data (0x72), the actor package data (0x70) and the guards (0x7c); with the classes'
//! constructors, compares and destructors.
//!
//! The constructors of this file's classes are done in place where its setters build one
//! (`BSExtraData`'s constructor, the vtable, the payload), as is the store of a payload word
//! (`0041fd00`); the destructors and the data they own run as the exe has them, each a typed call.

use super::links::{add_unless_held, is_empty, new_by, remove_item, LIST_ADD_HEAD};
use super::{
    access, cell_refs, extra_init, first_of_kind, free_when, list_add, list_remove, new_object,
    read_word, records, setters, word_at, EXTRA_PAYLOAD, FAMILY, OPERATOR_DELETE,
};
use engine_core::{args, typed_call, Engine};

/// The extra data types (`cEtype`, Xbox PDB `EXTRA_DATA_TYPE`) of this file.
const KIND_DECAL_REFS: u8 = 0x57;
const KIND_REFLECTED_REFS: u8 = 0x65;
const KIND_REFLECTOR_REFS: u8 = 0x66;
const KIND_PRIMITIVE: u8 = 0x6b;
const KIND_PATROL_REF_DATA: u8 = 0x6f;
const KIND_PACKAGE_DATA: u8 = 0x70;
const KIND_COLLISION_DATA: u8 = 0x72;
const KIND_OCCLUSION_PLANE_REF_DATA: u8 = 0x76;
const KIND_PORTAL_REF_DATA: u8 = 0x77;
const KIND_PORTAL: u8 = 0x78;
const KIND_ROOM: u8 = 0x79;
const KIND_ROOM_REF_DATA: u8 = 0x7b;
const KIND_GUARDED_REF_DATA: u8 = 0x7c;
const KIND_WATER_LIGHT_REFS: u8 = 0x84;
const KIND_LIT_WATER_REFS: u8 = 0x85;

/// The vtables this file's constructors set.
const VTABLE_EXTRA_PRIMITIVE: u32 = 0x0101_51b4;
const VTABLE_EXTRA_PATROL_REF_DATA: u32 = 0x0101_51c0;
const VTABLE_EXTRA_OCCLUSION_PLANE_REF_DATA: u32 = 0x0101_51cc;
const VTABLE_EXTRA_PORTAL_REF_DATA: u32 = 0x0101_51d8;
const VTABLE_EXTRA_ROOM_REF_DATA: u32 = 0x0101_51e4;
const VTABLE_EXTRA_COLLISION_DATA: u32 = 0x0101_51f0;
const VTABLE_EXTRA_PACKAGE_DATA: u32 = 0x0101_51fc;

/// Other units' constructors (`thiscall(block)`): the decal list, `ExtraPortal`, `ExtraRoom`,
/// `ExtraGuardedRefData`.
const DECAL_REFS_INIT: u32 = 0x0043_3ca0;
const EXTRA_PORTAL_INIT: u32 = 0x0043_5820;
const EXTRA_ROOM_INIT: u32 = 0x0043_58a0;
const GUARDED_REF_DATA_INIT: u32 = 0x0043_0f30;
/// The decal list's add (`thiscall(extra, reference, intersection, normal)`), the reflect lists'
/// remove (`thiscall(extra, reference)`) and flag set (`thiscall(extra, reference, flags)`), the
/// guards' add (`thiscall(extra, guard)`) and their two-word method.
const DECAL_REFS_ADD: u32 = 0x0043_4480;
const REFLECT_REFS_REMOVE: u32 = 0x0043_4800;
const REFLECT_REFS_SET_FLAGS: u32 = 0x0043_48a0;
const GUARDED_REF_DATA_ADD_GUARD: u32 = 0x0043_10d0;
const GUARDED_REF_DATA_TWO_WORDS: u32 = 0x0043_1120;
/// `PatrolRefData::Compare` (`thiscall(data, other)`: true when they differ) and its destructor's
/// body (`thiscall(data)`).
const PATROL_DATA_COMPARE: u32 = 0x0067_c720;
const PATROL_DATA_DROP: u32 = 0x0067_c670;
/// A `BSSimpleList`'s destructor body (`thiscall(list)`).
const LIST_DROP: u32 = 0x0046_ffb0;
/// `NiPointer<T>::operator=(T*)` (`thiscall(at, object)`).
const NI_POINTER_ASSIGN: u32 = 0x0066_b0d0;
/// The master room search's visited set (`NiTMap<TESObjectREFR *, bool>`, 0x10 bytes): its
/// constructor (`thiscall(set, hash size)`), destructor body, `GetAt(key, &value)` and
/// `SetAt(key, value)`.
const VISITED_NEW: u32 = 0x0042_f4e0;
const VISITED_DROP: u32 = 0x0042_fe80;
const VISITED_FIND: u32 = 0x0057_c850;
const VISITED_INSERT: u32 = 0x0084_d310;

/// A reference's extra data list (`005d43c0`, in place), a room data's lists and master flag, a
/// portal's two rooms (`009707f0` and `00420ba0`, in place).
const REFERENCE_EXTRA_LIST: u32 = 0x44;
const ROOM_PORTAL_LIST: u32 = 0;
const ROOM_ROOM_LIST: u32 = 8;
const ROOM_MASTER: u32 = 0x10;
const PORTAL_FIRST_ROOM: u32 = 0xfc;
const PORTAL_SECOND_ROOM: u32 = 0x100;

// ---- What the file shares --------------------------------------------------------------------

/// The body of this file's constructors: `BSExtraData`'s (`0040ec80`, in place) with `kind`, then
/// the class's vtable. Answers `this`.
#[inline]
fn extra_with_vtable(e: &mut Engine, this: u32, kind: u8, vtable: u32) -> u32 {
    extra_init(e, this, kind);
    e.mem.set_u32(this, vtable);
    this
}

/// The setters that replace an extra data owning one word: the first of `kind` deleted, then for a
/// non-null `value` a new one (0x10 bytes, built by `init`) given it (`0041fd00`, in place) and
/// added.
fn replace_holding(
    e: &mut Engine,
    list: u32,
    kind: u8,
    value: u32,
    init: impl FnOnce(&mut Engine, u32) -> u32,
) {
    let old = first_of_kind(e, list, kind);
    if old != 0 {
        list_remove(e, list, old, true);
    }
    if value != 0 {
        let built = new_object(e, 0x10, init);
        e.mem.set_u32(built.wrapping_add(EXTRA_PAYLOAD), value);
        list_add(e, list, built);
    }
}

/// The compares of the extra data owning data: equal when neither has data, different when one
/// has, else `differ` on the two.
#[inline]
fn compare_owned(
    e: &mut Engine,
    this: u32,
    other: u32,
    differ: impl FnOnce(&mut Engine, u32, u32) -> bool,
) -> bool {
    let mine = word_at(e, this, EXTRA_PAYLOAD);
    let theirs = word_at(e, other, EXTRA_PAYLOAD);
    match (mine, theirs) {
        (0, 0) => false,
        (0, _) | (_, 0) => true,
        _ => differ(e, mine, theirs),
    }
}

/// Whether any of the first `n` words of `a` and `b` differ.
#[inline]
fn words_differ(e: &Engine, a: u32, b: u32, n: u32) -> bool {
    (0..n).any(|i| e.mem.u32(a.wrapping_add(4 * i)) != e.mem.u32(b.wrapping_add(4 * i)))
}

/// Word `index` of the block at `data` (the index not checked).
#[inline]
fn word_index(data: u32, index: i32) -> u32 {
    data.wrapping_add((index as u32).wrapping_mul(4))
}

/// The reflect lists' flag setters: the entries of the extra data of `kind` walked to the first
/// whose reference is `reference` (stopping at a null entry). Found: `bit` set (`set`), or cleared
/// with the entry removed (`00434800`) once no flag is left and the extra data deleted when its list
/// is then empty. Not found, with `set`: the entry added with `bit` (`004348a0`), the extra data
/// first built by `init` (0x14 bytes) and added when the list has none.
fn set_reflect_flag(
    e: &mut Engine,
    list: u32,
    kind: u8,
    reference: u32,
    set: bool,
    bit: u32,
    init: impl FnOnce(&mut Engine, u32) -> u32,
) {
    let mut extra = first_of_kind(e, list, kind);
    if extra != 0 {
        let mut node = extra.wrapping_add(EXTRA_PAYLOAD);
        while node != 0 {
            let entry = e.mem.u32(node);
            if entry == 0 {
                break;
            }
            if e.mem.u32(entry) == reference {
                let at = entry.wrapping_add(4);
                let flags = e.mem.u32(at);
                if set {
                    e.mem.set_u32(at, flags | bit);
                } else {
                    let flags = flags & !bit;
                    e.mem.set_u32(at, flags);
                    if flags == 0 {
                        e.call(REFLECT_REFS_REMOVE, &args![extra, reference]);
                        if is_empty(e, extra.wrapping_add(EXTRA_PAYLOAD)) {
                            list_remove(e, list, extra, true);
                        }
                    }
                }
                return;
            }
            node = e.mem.u32(node.wrapping_add(4));
        }
    }
    if set {
        if extra == 0 {
            extra = new_object(e, 0x14, init);
            list_add(e, list, extra);
        }
        e.call(REFLECT_REFS_SET_FLAGS, &args![extra, reference, bit]);
    }
}

/// The water light and lit water setters: `reference` (in a one-word local) put at the head of the
/// list of the extra data of `kind` unless it holds it (`add`), or taken out of it, the extra data
/// then deleted when `drop_empty` and its list is empty. Without the extra data an add builds one
/// (0x14 bytes, `init`), adds it and puts the reference in; a removal does nothing.
fn change_reference(
    e: &mut Engine,
    list: u32,
    kind: u8,
    reference: u32,
    add: bool,
    drop_empty: bool,
    init: impl FnOnce(&mut Engine, u32) -> u32,
) {
    let extra = first_of_kind(e, list, kind);
    if extra != 0 {
        let head = extra.wrapping_add(EXTRA_PAYLOAD);
        if add {
            add_unless_held(e, head, reference);
        } else {
            remove_item(e, head, reference);
            if drop_empty && is_empty(e, head) {
                list_remove(e, list, extra, true);
            }
        }
    } else if add {
        let extra = new_object(e, 0x14, init);
        list_add(e, list, extra);
        e.with_stack(4, |e, slot| {
            e.mem.set_u32(slot.addr(), reference);
            e.call(
                LIST_ADD_HEAD,
                &args![extra.wrapping_add(EXTRA_PAYLOAD), slot],
            );
        });
    }
}

/// `SetPortal` and `SetRoom`: a null `value` deletes the extra data of `kind` by type; another is
/// assigned to the `NiPointer` at +0xc of the one there, or of a new one (0x10 bytes, `construct`)
/// added after.
fn set_pointer_extra(e: &mut Engine, list: u32, kind: u8, construct: u32, value: u32) {
    if value == 0 {
        setters::remove_kind(e, list, kind);
        return;
    }
    match first_of_kind(e, list, kind) {
        0 => {
            let built = new_by(e, 0x10, construct);
            e.call(
                NI_POINTER_ASSIGN,
                &args![built.wrapping_add(EXTRA_PAYLOAD), value],
            );
            list_add(e, list, built);
        }
        extra => {
            e.call(
                NI_POINTER_ASSIGN,
                &args![extra.wrapping_add(EXTRA_PAYLOAD), value],
            );
        }
    }
}

/// The object an `ExtraPortal` or `ExtraRoom` points at (`00559450` on +0xc, in place), or 0.
#[inline]
fn pointer_extra(e: &mut Engine, list: u32, kind: u8) -> u32 {
    match first_of_kind(e, list, kind) {
        0 => 0,
        extra => read_word(e, extra.wrapping_add(EXTRA_PAYLOAD)),
    }
}

/// The room reference data (`004207e0`).
#[inline]
fn room_data_of(e: &mut Engine, list: u32) -> u32 {
    typed_call!(FAMILY, 0x0042_07e0, room_ref_data(e, list))
}

/// The room data's adders: the data built first (0x14 bytes, `00416b80`) and set (`00420440`) when
/// the list has none and `reference` is not null; then `reference` put at the head of the list at
/// `offset` of the data unless it holds it.
fn add_to_room_data(e: &mut Engine, list: u32, offset: u32, reference: u32) {
    if room_data_of(e, list) == 0 && reference != 0 {
        let data = new_object(e, 0x14, |e, block| {
            typed_call!(FAMILY, 0x0041_6b80, records::room_links_new(e, block))
        });
        typed_call!(FAMILY, 0x0042_0440, set_room_ref_data(e, list, data));
    }
    let head = room_data_of(e, list).wrapping_add(offset);
    add_unless_held(e, head, reference);
}

// ---- The functions --------------------------------------------------------------------------

// Typed from 0041f080
/// A decal for a non-null `reference` added (`00434480`, given the intersection and the normal) to
/// the type 0x57 extra data, a new one (0x14 bytes, `00433ca0`) added first when the list has none.
pub fn add_decal(e: &mut Engine, this: u32, reference: u32, intersect: u32, normal: u32) {
    if reference == 0 {
        return;
    }
    let mut extra = first_of_kind(e, this, KIND_DECAL_REFS);
    if extra == 0 {
        extra = new_by(e, 0x14, DECAL_REFS_INIT);
        list_add(e, this, extra);
    }
    e.call(DECAL_REFS_ADD, &args![extra, reference, intersect, normal]);
}

// Typed from 0041f140
/// `ExtraDataList::GetReflectedRefs` (Xbox PDB): the head node at +0xc of the type 0x65 extra data,
/// or null.
pub fn reflected_refs(e: &mut Engine, this: u32) -> u32 {
    access::record_or_null(e, this, KIND_REFLECTED_REFS)
}

// Typed from 0041f170
/// The head node at +0xc of the type 0x66 extra data (the reflector references), or null.
pub fn reflector_refs(e: &mut Engine, this: u32) -> u32 {
    access::record_or_null(e, this, KIND_REFLECTOR_REFS)
}

/// The reflector references' constructor (`00411be0`, the cell references family's).
#[inline]
fn reflector_refs_new(e: &mut Engine, block: u32) -> u32 {
    typed_call!(
        cell_refs::FAMILY,
        0x0041_1be0,
        cell_refs::extra_list::reflector_refs_new(e, block)
    )
}

/// The reflected references' constructor (`00411b40`, the cell references family's).
#[inline]
fn reflected_refs_new(e: &mut Engine, block: u32) -> u32 {
    typed_call!(
        cell_refs::FAMILY,
        0x0041_1b40,
        cell_refs::extra_list::reflected_refs_new(e, block)
    )
}

// Typed from 0041f1a0
/// Flag bit 0 of the reflector entry for `reference` set or cleared ([`set_reflect_flag`]).
pub fn set_reflector_flag_1(e: &mut Engine, this: u32, reference: u32, set: bool) {
    set_reflect_flag(
        e,
        this,
        KIND_REFLECTOR_REFS,
        reference,
        set,
        1,
        reflector_refs_new,
    );
}

// Typed from 0041f330
/// Flag bit 1 (2) of the reflector entry for `reference` set or cleared.
pub fn set_reflector_flag_2(e: &mut Engine, this: u32, reference: u32, set: bool) {
    set_reflect_flag(
        e,
        this,
        KIND_REFLECTOR_REFS,
        reference,
        set,
        2,
        reflector_refs_new,
    );
}

// Typed from 0041f4c0
/// Flag bit 0 of the reflected entry for `reference` set or cleared.
pub fn set_reflected_flag_1(e: &mut Engine, this: u32, reference: u32, set: bool) {
    set_reflect_flag(
        e,
        this,
        KIND_REFLECTED_REFS,
        reference,
        set,
        1,
        reflected_refs_new,
    );
}

// Typed from 0041f650
/// Flag bit 1 (2) of the reflected entry for `reference` set or cleared.
pub fn set_reflected_flag_2(e: &mut Engine, this: u32, reference: u32, set: bool) {
    set_reflect_flag(
        e,
        this,
        KIND_REFLECTED_REFS,
        reference,
        set,
        2,
        reflected_refs_new,
    );
}

// Typed from 0041f7e0
/// `ExtraDataList::GetWaterLightRefs` (Xbox PDB): the head node at +0xc of the type 0x84 extra
/// data, or null.
pub fn water_light_refs(e: &mut Engine, this: u32) -> u32 {
    access::record_or_null(e, this, KIND_WATER_LIGHT_REFS)
}

// Typed from 0041f810
/// `ExtraDataList::GetLitWaterRefs` (Xbox PDB): the head node at +0xc of the type 0x85 extra data,
/// or null.
pub fn lit_water_refs(e: &mut Engine, this: u32) -> u32 {
    access::record_or_null(e, this, KIND_LIT_WATER_REFS)
}

// Typed from 0041f840
/// `ExtraDataList::SetWaterLightRef` (Xbox PDB): `reference` added to or removed from the water
/// light references ([`change_reference`], `00411c80`); a removal never deletes the extra data.
pub fn set_water_light_ref(e: &mut Engine, this: u32, reference: u32, add: bool) {
    change_reference(
        e,
        this,
        KIND_WATER_LIGHT_REFS,
        reference,
        add,
        false,
        |e, block| {
            typed_call!(
                cell_refs::FAMILY,
                0x0041_1c80,
                cell_refs::extra_list::water_light_refs_new(e, block)
            )
        },
    );
}

// Typed from 0041f940
/// `reference` added to or removed from the lit water references (`00411d20`); a removal deletes
/// the extra data when its list is then empty.
pub fn set_lit_water_ref(e: &mut Engine, this: u32, reference: u32, add: bool) {
    change_reference(
        e,
        this,
        KIND_LIT_WATER_REFS,
        reference,
        add,
        true,
        |e, block| {
            typed_call!(
                cell_refs::FAMILY,
                0x0041_1d20,
                cell_refs::extra_list::lit_water_refs_new(e, block)
            )
        },
    );
}

// Typed from 0041fa60
/// `ExtraDataList::AddPrimitive` (Xbox PDB): a new primitive extra data (0x10 bytes, its constructor
/// `0041faf0` in place) owning a non-null `primitive`, added; the list is not asked for one first.
pub fn add_primitive(e: &mut Engine, this: u32, primitive: u32) {
    if primitive == 0 {
        return;
    }
    let built = new_object(e, 0x10, |e, block| {
        extra_with_vtable(e, block, KIND_PRIMITIVE, VTABLE_EXTRA_PRIMITIVE);
        e.mem.set_u32(block.wrapping_add(EXTRA_PAYLOAD), primitive);
        block
    });
    list_add(e, this, built);
}

// Typed from 0041faf0
/// `ExtraPrimitive`'s constructor: `BSExtraData`'s with type 0x6b, its vtable, the primitive.
/// Answers `this`.
pub fn primitive_extra_new(e: &mut Engine, this: u32, primitive: u32) -> u32 {
    extra_with_vtable(e, this, KIND_PRIMITIVE, VTABLE_EXTRA_PRIMITIVE);
    e.mem.set_u32(this.wrapping_add(EXTRA_PAYLOAD), primitive);
    this
}

// Typed from 0041fb20
/// `ExtraPrimitive`'s scalar deleting destructor: its destructor (`0041fb50`), then `operator
/// delete` when bit 0 of `flags` is set. Answers `this`.
pub fn primitive_extra_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0041_fb50, primitive_extra_drop(e, this));
    free_when(e, this, flags)
}

// Typed from 0041fb50
/// `ExtraPrimitive`'s destructor: its vtable back, the primitive it owns deleted through its slot 0
/// (with 1) when there is one, then `BSExtraData`'s destructor (`0040ecb0`).
pub fn primitive_extra_drop(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, VTABLE_EXTRA_PRIMITIVE);
    let primitive = word_at(e, this, EXTRA_PAYLOAD);
    if primitive != 0 {
        e.vcall(primitive, 0, &args![1u32]);
    }
    typed_call!(
        cell_refs::FAMILY,
        0x0040_ecb0,
        cell_refs::extra_list::extra_data_drop(e, this)
    );
}

// Typed from 0041fbe0
/// `ExtraDataList::GetPrimitive` (Xbox PDB): the primitive (+0xc of the type 0x6b extra data), or 0.
pub fn primitive(e: &mut Engine, this: u32) -> u32 {
    access::word_or(e, this, KIND_PRIMITIVE, 0)
}

// Typed from 0041fc10
/// The patrol reference data set: the type 0x6f extra data deleted, then a new one (its
/// constructor `0041fcd0` in place) given a non-null `data` and added.
pub fn set_patrol_ref_data(e: &mut Engine, this: u32, data: u32) {
    replace_holding(e, this, KIND_PATROL_REF_DATA, data, |e, block| {
        extra_with_vtable(e, block, KIND_PATROL_REF_DATA, VTABLE_EXTRA_PATROL_REF_DATA)
    });
}

// Typed from 0041fcd0
/// `ExtraPatrolRefData`'s constructor: `BSExtraData`'s with type 0x6f and its vtable (the data word
/// left as it is). Answers `this`.
pub fn patrol_extra_new(e: &mut Engine, this: u32) -> u32 {
    extra_with_vtable(e, this, KIND_PATROL_REF_DATA, VTABLE_EXTRA_PATROL_REF_DATA)
}

// Typed from 0041fd20
/// `ExtraPatrolRefData::Compare` (Xbox PDB): true when they differ ([`compare_owned`];
/// `PatrolRefData::Compare` `0067c720` on two data).
pub fn patrol_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    compare_owned(e, this, other, |e, mine, theirs| {
        e.call(PATROL_DATA_COMPARE, &args![mine, theirs]).bool()
    })
}

// Typed from 0041fd80
/// `ExtraPatrolRefData`'s scalar deleting destructor: its destructor (`0041fdb0`), then `operator
/// delete` for bit 0 of `flags`. Answers `this`.
pub fn patrol_extra_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0041_fdb0, patrol_extra_drop(e, this));
    free_when(e, this, flags)
}

// Typed from 0041fdb0
/// `ExtraPatrolRefData`'s destructor: its vtable back, its data deleted (`0041fe10`), then
/// `BSExtraData`'s destructor.
pub fn patrol_extra_drop(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, VTABLE_EXTRA_PATROL_REF_DATA);
    typed_call!(FAMILY, 0x0041_fe10, patrol_drop_data(e, this));
    typed_call!(
        cell_refs::FAMILY,
        0x0040_ecb0,
        cell_refs::extra_list::extra_data_drop(e, this)
    );
}

// Typed from 0041fe10
/// The patrol data deleted through its scalar deleting destructor (`0041fe60`, with 1) when there is
/// one, and the word cleared.
pub fn patrol_drop_data(e: &mut Engine, this: u32) {
    let data = word_at(e, this, EXTRA_PAYLOAD);
    if data != 0 {
        typed_call!(FAMILY, 0x0041_fe60, patrol_data_delete(e, data, 1));
    }
    e.mem.set_u32(this.wrapping_add(EXTRA_PAYLOAD), 0);
}

// Typed from 0041fe60
/// `PatrolRefData`'s scalar deleting destructor (a copy in this unit): its destructor's body
/// (`0067c670`), then `operator delete` for bit 0 of `flags`. Answers `this`.
pub fn patrol_data_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    e.call(PATROL_DATA_DROP, &args![this]);
    free_when(e, this, flags)
}

// Typed from 0041fe90
/// The patrol data (+0xc of the type 0x6f extra data), or 0.
pub fn patrol_ref_data(e: &mut Engine, this: u32) -> u32 {
    access::word_or(e, this, KIND_PATROL_REF_DATA, 0)
}

// Typed from 0041fec0
/// The occlusion plane data set as the patrol data is (type 0x76, its constructor `0041ff80` in
/// place).
pub fn set_occlusion_plane_ref_data(e: &mut Engine, this: u32, data: u32) {
    replace_holding(e, this, KIND_OCCLUSION_PLANE_REF_DATA, data, |e, block| {
        extra_with_vtable(
            e,
            block,
            KIND_OCCLUSION_PLANE_REF_DATA,
            VTABLE_EXTRA_OCCLUSION_PLANE_REF_DATA,
        )
    });
}

// Typed from 0041ff80
/// `ExtraOcclusionPlaneRefData`'s constructor: `BSExtraData`'s with type 0x76 and its vtable.
/// Answers `this`.
pub fn occlusion_extra_new(e: &mut Engine, this: u32) -> u32 {
    extra_with_vtable(
        e,
        this,
        KIND_OCCLUSION_PLANE_REF_DATA,
        VTABLE_EXTRA_OCCLUSION_PLANE_REF_DATA,
    )
}

// Typed from 0041ffb0
/// `ExtraOcclusionPlaneRefData::Compare` (Xbox PDB): true when they differ; two data differ in any
/// of their four words.
pub fn occlusion_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    compare_owned(e, this, other, |e, mine, theirs| {
        words_differ(e, mine, theirs, 4)
    })
}

// Typed from 00420030
/// `ExtraOcclusionPlaneRefData`'s scalar deleting destructor (`00420060`, then `operator delete`
/// for bit 0 of `flags`). Answers `this`.
pub fn occlusion_extra_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0042_0060, occlusion_extra_drop(e, this));
    free_when(e, this, flags)
}

// Typed from 00420060
/// `ExtraOcclusionPlaneRefData`'s destructor: its vtable back, its data freed (`004200c0`), then
/// `BSExtraData`'s destructor.
pub fn occlusion_extra_drop(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, VTABLE_EXTRA_OCCLUSION_PLANE_REF_DATA);
    typed_call!(FAMILY, 0x0042_00c0, free_data(e, this));
    typed_call!(
        cell_refs::FAMILY,
        0x0040_ecb0,
        cell_refs::extra_list::extra_data_drop(e, this)
    );
}

// Typed from 004200c0
/// The data at +0xc freed with `operator delete` when there is one, and the word cleared (the
/// occlusion plane's and the portal's).
pub fn free_data(e: &mut Engine, this: u32) {
    let data = word_at(e, this, EXTRA_PAYLOAD);
    if data != 0 {
        e.call(OPERATOR_DELETE, &args![data]);
    }
    e.mem.set_u32(this.wrapping_add(EXTRA_PAYLOAD), 0);
}

// Typed from 00420100
/// `ExtraDataList::GetOcclusionPlaneLinkedRefData` (Xbox PDB): the data (+0xc of the type 0x76
/// extra data), or 0.
pub fn occlusion_plane_data(e: &mut Engine, this: u32) -> u32 {
    access::word_or(e, this, KIND_OCCLUSION_PLANE_REF_DATA, 0)
}

// Typed from 00420130
/// Word `index` of the occlusion plane data (`00420100`), or 0 without it.
pub fn occlusion_plane_word(e: &mut Engine, this: u32, index: i32) -> u32 {
    match typed_call!(FAMILY, 0x0042_0100, occlusion_plane_data(e, this)) {
        0 => 0,
        data => e.mem.u32(word_index(data, index)),
    }
}

// Typed from 00420160
/// Word `index` of the occlusion plane data set to `reference`: the data built first (0x10 bytes,
/// `00411dc0`) and set (`0041fec0`) when the list has none and `reference` is not null; the word
/// stored whenever the list then has data.
pub fn set_occlusion_plane_word(e: &mut Engine, this: u32, reference: u32, index: i32) {
    if typed_call!(FAMILY, 0x0042_0100, occlusion_plane_data(e, this)) == 0 && reference != 0 {
        let data = new_object(e, 0x10, |e, block| {
            typed_call!(
                cell_refs::FAMILY,
                0x0041_1dc0,
                cell_refs::extra_list::four_words_new(e, block)
            )
        });
        typed_call!(
            FAMILY,
            0x0041_fec0,
            set_occlusion_plane_ref_data(e, this, data)
        );
    }
    let data = typed_call!(FAMILY, 0x0042_0100, occlusion_plane_data(e, this));
    if data != 0 {
        e.mem.set_u32(word_index(data, index), reference);
    }
}

// Typed from 00420210
/// The portal data set as the patrol data is (type 0x77, its constructor `004202d0` in place).
pub fn set_portal_ref_data(e: &mut Engine, this: u32, data: u32) {
    replace_holding(e, this, KIND_PORTAL_REF_DATA, data, |e, block| {
        extra_with_vtable(e, block, KIND_PORTAL_REF_DATA, VTABLE_EXTRA_PORTAL_REF_DATA)
    });
}

// Typed from 004202d0
/// `ExtraPortalRefData`'s constructor: `BSExtraData`'s with type 0x77 and its vtable. Answers `this`.
pub fn portal_extra_new(e: &mut Engine, this: u32) -> u32 {
    extra_with_vtable(e, this, KIND_PORTAL_REF_DATA, VTABLE_EXTRA_PORTAL_REF_DATA)
}

// Typed from 00420300
/// `ExtraPortalRefData::Compare` (Xbox PDB): true when they differ; two data differ in either of
/// their two words.
pub fn portal_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    compare_owned(e, this, other, |e, mine, theirs| {
        words_differ(e, mine, theirs, 2)
    })
}

// Typed from 00420380
/// `ExtraPortalRefData`'s scalar deleting destructor (`004203b0`, then `operator delete` for bit 0
/// of `flags`). Answers `this`.
pub fn portal_extra_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0042_03b0, portal_extra_drop(e, this));
    free_when(e, this, flags)
}

// Typed from 004203b0
/// `ExtraPortalRefData`'s destructor: its vtable back, its data freed (`004200c0`), then
/// `BSExtraData`'s destructor.
pub fn portal_extra_drop(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, VTABLE_EXTRA_PORTAL_REF_DATA);
    typed_call!(FAMILY, 0x0042_00c0, free_data(e, this));
    typed_call!(
        cell_refs::FAMILY,
        0x0040_ecb0,
        cell_refs::extra_list::extra_data_drop(e, this)
    );
}

// Typed from 00420410
/// The portal data (+0xc of the type 0x77 extra data), or 0.
pub fn portal_ref_data(e: &mut Engine, this: u32) -> u32 {
    access::word_or(e, this, KIND_PORTAL_REF_DATA, 0)
}

// Typed from 00420440
/// The room data set as the patrol data is (type 0x7b, its constructor `00420500` in place).
pub fn set_room_ref_data(e: &mut Engine, this: u32, data: u32) {
    replace_holding(e, this, KIND_ROOM_REF_DATA, data, |e, block| {
        extra_with_vtable(e, block, KIND_ROOM_REF_DATA, VTABLE_EXTRA_ROOM_REF_DATA)
    });
}

// Typed from 00420500
/// `ExtraRoomRefData`'s constructor: `BSExtraData`'s with type 0x7b and its vtable. Answers `this`.
pub fn room_extra_new(e: &mut Engine, this: u32) -> u32 {
    extra_with_vtable(e, this, KIND_ROOM_REF_DATA, VTABLE_EXTRA_ROOM_REF_DATA)
}

// Typed from 00420530
/// `ExtraRoomRefData::Compare` (Xbox PDB): true when they differ; two data differ in their master
/// flags or their room lists' items (in order, and in length).
pub fn room_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    compare_owned(e, this, other, |e, mine, theirs| {
        if e.mem.u8(mine.wrapping_add(ROOM_MASTER)) != e.mem.u8(theirs.wrapping_add(ROOM_MASTER)) {
            return true;
        }
        let mut a = mine.wrapping_add(ROOM_ROOM_LIST);
        let mut b = theirs.wrapping_add(ROOM_ROOM_LIST);
        while a != 0 && b != 0 {
            if e.mem.u32(a) != e.mem.u32(b) {
                return true;
            }
            a = e.mem.u32(a.wrapping_add(4));
            b = e.mem.u32(b.wrapping_add(4));
        }
        a != 0 || b != 0
    })
}

// Typed from 00420600
/// `ExtraRoomRefData`'s scalar deleting destructor (`00420630`, then `operator delete` for bit 0 of
/// `flags`). Answers `this`.
pub fn room_extra_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0042_0630, room_extra_drop(e, this));
    free_when(e, this, flags)
}

// Typed from 00420630
/// `ExtraRoomRefData`'s destructor: its vtable back, its data deleted (`00420690`), then
/// `BSExtraData`'s destructor.
pub fn room_extra_drop(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, VTABLE_EXTRA_ROOM_REF_DATA);
    typed_call!(FAMILY, 0x0042_0690, room_drop_data(e, this));
    typed_call!(
        cell_refs::FAMILY,
        0x0040_ecb0,
        cell_refs::extra_list::extra_data_drop(e, this)
    );
}

// Typed from 00420690
/// The room data deleted through its scalar deleting destructor (`004206e0`, with 1) when there is
/// one, and the word cleared.
pub fn room_drop_data(e: &mut Engine, this: u32) {
    let data = word_at(e, this, EXTRA_PAYLOAD);
    if data != 0 {
        typed_call!(FAMILY, 0x0042_06e0, room_data_delete(e, data, 1));
    }
    e.mem.set_u32(this.wrapping_add(EXTRA_PAYLOAD), 0);
}

// Typed from 004206e0
/// `RoomLinkedRefData`'s scalar deleting destructor (a copy in this unit): its destructor
/// (`00420710`), then `operator delete` for bit 0 of `flags`. Answers `this`.
pub fn room_data_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0042_0710, room_data_drop(e, this));
    free_when(e, this, flags)
}

// Typed from 00420710
/// `RoomLinkedRefData`'s destructor: the room list (+8), then the portal list (+0), each destroyed
/// (`0046ffb0`).
pub fn room_data_drop(e: &mut Engine, this: u32) {
    e.call(LIST_DROP, &args![this.wrapping_add(ROOM_ROOM_LIST)]);
    e.call(LIST_DROP, &args![this.wrapping_add(ROOM_PORTAL_LIST)]);
}

// Typed from 00420770
/// The room list (+8) of the room data (`004207e0`), or 0 without one.
pub fn room_list(e: &mut Engine, this: u32) -> u32 {
    match room_data_of(e, this) {
        0 => 0,
        data => data.wrapping_add(ROOM_ROOM_LIST),
    }
}

// Typed from 004207b0
/// The room data (`004207e0`), masked by itself as the exe does (`-(x != 0) & x`).
pub fn room_data_masked(e: &mut Engine, this: u32) -> u32 {
    room_data_of(e, this)
}

// Typed from 004207e0
/// `ExtraDataList::GetRoomLinkedRefData` (Xbox PDB): the data (+0xc of the type 0x7b extra data), or
/// 0.
pub fn room_ref_data(e: &mut Engine, this: u32) -> u32 {
    access::word_or(e, this, KIND_ROOM_REF_DATA, 0)
}

// Typed from 00420810
/// `ExtraDataList::GetRoomIsMaster` (Xbox PDB): true without room data, with its master flag set,
/// or with its room list empty (`008256d0`, in place).
pub fn room_is_master(e: &mut Engine, this: u32) -> bool {
    match first_of_kind(e, this, KIND_ROOM_REF_DATA) {
        0 => true,
        extra => {
            let data = word_at(e, extra, EXTRA_PAYLOAD);
            e.mem.u8(data.wrapping_add(ROOM_MASTER)) != 0
                || is_empty(e, data.wrapping_add(ROOM_ROOM_LIST))
        }
    }
}

// Typed from 00420870
/// `ExtraDataList::SetRoomIsMaster` (Xbox PDB): the room data's master flag set to 0 or 1; nothing
/// without room data (the data itself not asked for).
pub fn set_room_is_master(e: &mut Engine, this: u32, master: bool) {
    let extra = first_of_kind(e, this, KIND_ROOM_REF_DATA);
    if extra != 0 {
        let data = word_at(e, extra, EXTRA_PAYLOAD);
        e.mem
            .set_u8(data.wrapping_add(ROOM_MASTER), u8::from(master));
    }
}

// Typed from 004208b0
/// `ExtraDataList::GetMasterRoom` (Xbox PDB): 0 without room data or for a master room
/// (`00420810`); else the search (`00420950`) with a fresh visited set (a 0x10-byte local, hash size
/// 0x25, `0042f4e0`, its destructor `0042fe80` after).
pub fn master_room(e: &mut Engine, this: u32) -> u32 {
    if first_of_kind(e, this, KIND_ROOM_REF_DATA) == 0 {
        return 0;
    }
    if typed_call!(FAMILY, 0x0042_0810, room_is_master(e, this)) {
        return 0;
    }
    e.with_stack(0x10, |e, visited| {
        e.call(VISITED_NEW, &args![visited, 0x25u32]);
        let master = typed_call!(
            FAMILY,
            0x0042_0950,
            find_master_room(e, this, visited.addr())
        );
        e.call(VISITED_DROP, &args![visited]);
        master
    })
}

// Typed from 00420950
/// The master room search (the visited set at `visited`): 0 without room data. First the room
/// list's first reference (up to a null one) whose own list (`005d43c0`, in place) is a master room
/// (`00420810`); then each reference not yet visited (`0057c850`) recorded (`0084d310`, with the
/// lookup's flag local, which the exe leaves as it was) and searched in turn; the first found.
pub fn find_master_room(e: &mut Engine, this: u32, visited: u32) -> u32 {
    let extra = first_of_kind(e, this, KIND_ROOM_REF_DATA);
    if extra == 0 {
        return 0;
    }
    let rooms = word_at(e, extra, EXTRA_PAYLOAD).wrapping_add(ROOM_ROOM_LIST);
    let mut node = rooms;
    while node != 0 {
        let room = e.mem.u32(node);
        if room == 0 {
            break;
        }
        let list = room.wrapping_add(REFERENCE_EXTRA_LIST);
        if typed_call!(FAMILY, 0x0042_0810, room_is_master(e, list)) {
            return room;
        }
        node = e.mem.u32(node.wrapping_add(4));
    }
    let mut node = rooms;
    e.with_stack(4, |e, seen| {
        while node != 0 {
            let room = e.mem.u32(node);
            if room == 0 {
                break;
            }
            if !e.call(VISITED_FIND, &args![visited, room, seen]).bool() {
                let seen_value = u32::from(e.mem.u8(seen.addr()));
                e.call(VISITED_INSERT, &args![visited, room, seen_value]);
                let list = room.wrapping_add(REFERENCE_EXTRA_LIST);
                let found = typed_call!(FAMILY, 0x0042_0950, find_master_room(e, list, visited));
                if found != 0 {
                    return found;
                }
            }
            node = e.mem.u32(node.wrapping_add(4));
        }
        0
    })
}

// Typed from 00420a60
/// Word `index` of the portal data set to `reference` (the data built first, 8 bytes `004143c0`,
/// and set with `00420210` when the list has none and `reference` is not null; the word stored
/// whenever the list then has data), and the portal (`00420dd0`) given the room of `reference`'s
/// master room (`004208b0`), or of `reference` itself without one (`00420ed0`), or 0: index 0 its
/// first room, index 1 its second (both in place); another index only the word.
pub fn set_portal_word(e: &mut Engine, this: u32, index: i32, reference: u32) {
    if typed_call!(FAMILY, 0x0042_0410, portal_ref_data(e, this)) == 0 && reference != 0 {
        let data = new_object(e, 8, |e, block| {
            typed_call!(FAMILY, 0x0041_43c0, records::portal_ref_new(e, block))
        });
        typed_call!(FAMILY, 0x0042_0210, set_portal_ref_data(e, this, data));
    }
    let data = typed_call!(FAMILY, 0x0042_0410, portal_ref_data(e, this));
    if data != 0 {
        e.mem.set_u32(word_index(data, index), reference);
    }
    let portal = typed_call!(FAMILY, 0x0042_0dd0, portal(e, this));
    let master = if reference == 0 {
        0
    } else {
        let list = reference.wrapping_add(REFERENCE_EXTRA_LIST);
        typed_call!(FAMILY, 0x0042_08b0, master_room(e, list))
    };
    let owner = if master == 0 { reference } else { master };
    let room = if owner == 0 {
        0
    } else {
        let list = owner.wrapping_add(REFERENCE_EXTRA_LIST);
        typed_call!(FAMILY, 0x0042_0ed0, room(e, list))
    };
    if portal != 0 {
        match index {
            0 => e.mem.set_u32(portal.wrapping_add(PORTAL_FIRST_ROOM), room),
            1 => e.mem.set_u32(portal.wrapping_add(PORTAL_SECOND_ROOM), room),
            _ => {}
        }
    }
}

// Typed from 00420ba0
/// A portal's second room (+0x100) set.
pub fn portal_set_second_room(e: &mut Engine, this: u32, room: u32) {
    e.mem.set_u32(this.wrapping_add(PORTAL_SECOND_ROOM), room);
}

// Typed from 00420bc0
/// Word `index` of the portal data (`00420410`), or 0 without it.
pub fn portal_word(e: &mut Engine, this: u32, index: i32) -> u32 {
    match typed_call!(FAMILY, 0x0042_0410, portal_ref_data(e, this)) {
        0 => 0,
        data => e.mem.u32(word_index(data, index)),
    }
}

// Typed from 00420c10
/// `reference` added to the room data's room list ([`add_to_room_data`]).
pub fn add_room(e: &mut Engine, this: u32, reference: u32) {
    add_to_room_data(e, this, ROOM_ROOM_LIST, reference);
}

// Typed from 00420ce0
/// `reference` added to the room data's portal list ([`add_to_room_data`]).
pub fn add_portal(e: &mut Engine, this: u32, reference: u32) {
    add_to_room_data(e, this, ROOM_PORTAL_LIST, reference);
}

// Typed from 00420da0
/// `reference` taken out of the room data's portal list (`00905330`); nothing without room data.
pub fn remove_portal(e: &mut Engine, this: u32, reference: u32) {
    let data = room_data_of(e, this);
    if data != 0 {
        remove_item(e, data.wrapping_add(ROOM_PORTAL_LIST), reference);
    }
}

// Typed from 00420dd0
/// The portal an `ExtraPortal` (type 0x78) points at, or 0.
pub fn portal(e: &mut Engine, this: u32) -> u32 {
    pointer_extra(e, this, KIND_PORTAL)
}

// Typed from 00420e00
/// `ExtraDataList::SetPortal` (Xbox PDB): [`set_pointer_extra`] for type 0x78 (`00435820`).
pub fn set_portal(e: &mut Engine, this: u32, portal: u32) {
    set_pointer_extra(e, this, KIND_PORTAL, EXTRA_PORTAL_INIT, portal);
}

// Typed from 00420ed0
/// `ExtraDataList::GetRoom` (Xbox PDB): the room an `ExtraRoom` (type 0x79) points at, or 0.
pub fn room(e: &mut Engine, this: u32) -> u32 {
    pointer_extra(e, this, KIND_ROOM)
}

// Typed from 00420f00
/// `ExtraDataList::SetRoom` (Xbox PDB): [`set_pointer_extra`] for type 0x79 (`004358a0`).
pub fn set_room(e: &mut Engine, this: u32, room: u32) {
    set_pointer_extra(e, this, KIND_ROOM, EXTRA_ROOM_INIT, room);
}

// Typed from 00420fd0
/// The collision data set as the patrol data is (type 0x72, its constructor `00421090` in place).
pub fn set_collision_data(e: &mut Engine, this: u32, data: u32) {
    replace_holding(e, this, KIND_COLLISION_DATA, data, |e, block| {
        extra_with_vtable(e, block, KIND_COLLISION_DATA, VTABLE_EXTRA_COLLISION_DATA)
    });
}

// Typed from 00421090
/// `ExtraCollisionData`'s constructor: `BSExtraData`'s with type 0x72 and its vtable. Answers `this`.
pub fn collision_extra_new(e: &mut Engine, this: u32) -> u32 {
    extra_with_vtable(e, this, KIND_COLLISION_DATA, VTABLE_EXTRA_COLLISION_DATA)
}

// Typed from 004210c0
/// `ExtraCollisionData::Compare` (Xbox PDB): true when they differ; two data differ in the word each
/// starts with (`00559450` in place).
pub fn collision_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    compare_owned(e, this, other, |e, mine, theirs| {
        e.mem.u32(mine) != e.mem.u32(theirs)
    })
}

// Typed from 00421130
/// `ExtraCollisionData`'s scalar deleting destructor (`00421160`, then `operator delete` for bit 0
/// of `flags`). Answers `this`.
pub fn collision_extra_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0042_1160, collision_extra_drop(e, this));
    free_when(e, this, flags)
}

// Typed from 00421160
/// `ExtraCollisionData`'s destructor: its vtable back, its data freed with `operator delete` (null
/// or not), then `BSExtraData`'s destructor.
pub fn collision_extra_drop(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, VTABLE_EXTRA_COLLISION_DATA);
    let data = word_at(e, this, EXTRA_PAYLOAD);
    e.call(OPERATOR_DELETE, &args![data]);
    typed_call!(
        cell_refs::FAMILY,
        0x0040_ecb0,
        cell_refs::extra_list::extra_data_drop(e, this)
    );
}

// Typed from 004211a0
/// `ExtraDataList::GetCollisionData` (Xbox PDB): the data (+0xc of the type 0x72 extra data), or 0.
pub fn collision_data(e: &mut Engine, this: u32) -> u32 {
    access::word_or(e, this, KIND_COLLISION_DATA, 0)
}

// Typed from 004211d0
/// `ExtraDataList::SetActorPackageData` (Xbox PDB): `data` (null too) stored in the type 0x70 extra
/// data, or a new one holding it (0x10 bytes, its constructor `00421280` in place) added.
pub fn set_actor_package_data(e: &mut Engine, this: u32, data: u32) {
    match first_of_kind(e, this, KIND_PACKAGE_DATA) {
        0 => {
            let built = new_object(e, 0x10, |e, block| {
                extra_with_vtable(e, block, KIND_PACKAGE_DATA, VTABLE_EXTRA_PACKAGE_DATA);
                e.mem.set_u32(block.wrapping_add(EXTRA_PAYLOAD), data);
                block
            });
            list_add(e, this, built);
        }
        extra => e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), data),
    }
}

// Typed from 00421280
/// `ExtraPackageData`'s constructor: `BSExtraData`'s with type 0x70, its vtable, the data. Answers
/// `this`.
pub fn package_extra_new(e: &mut Engine, this: u32, data: u32) -> u32 {
    extra_with_vtable(e, this, KIND_PACKAGE_DATA, VTABLE_EXTRA_PACKAGE_DATA);
    e.mem.set_u32(this.wrapping_add(EXTRA_PAYLOAD), data);
    this
}

// Typed from 004212e0
/// `ExtraDataList::RemoveActorPackageData` (Xbox PDB): the type 0x70 extra data deleted when the
/// list has one.
pub fn remove_actor_package_data(e: &mut Engine, this: u32) {
    let extra = first_of_kind(e, this, KIND_PACKAGE_DATA);
    if extra != 0 {
        list_remove(e, this, extra, true);
    }
}

// Typed from 00421310
/// `ExtraDataList::AddGuard` (Xbox PDB): `guard` given (`004310d0`) to the type 0x7c extra data, a
/// new one (0x1c bytes, `00430f30`) added first when the list has none.
pub fn add_guard(e: &mut Engine, this: u32, guard: u32) {
    let mut extra = first_of_kind(e, this, KIND_GUARDED_REF_DATA);
    if extra == 0 {
        extra = new_by(e, 0x1c, GUARDED_REF_DATA_INIT);
        list_add(e, this, extra);
    }
    e.call(GUARDED_REF_DATA_ADD_GUARD, &args![extra, guard]);
}

// Typed from 004213c0
/// The two words given to the guards' method `00431120` when the list has a type 0x7c extra data.
pub fn guard_two_words(e: &mut Engine, this: u32, first: u32, second: u32) {
    let extra = first_of_kind(e, this, KIND_GUARDED_REF_DATA);
    if extra != 0 {
        e.call(GUARDED_REF_DATA_TWO_WORDS, &args![extra, first, second]);
    }
}
