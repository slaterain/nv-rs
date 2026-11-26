//! The extra data list's save writer (`00412970`; its Xbox name is not in the map): under the extra
//! data lock, each extra data of a type that is saved adds the chunk or chunks `ExtraDataList::Load`
//! ([`super::load`]) reads back, through the form writer's adders (`TESForm::AddChunk...`, the forms
//! family's, as typed calls). Forms go as their form ids (`0084e3a0`, in place); records the game keeps in memory are
//! copied to a local record, byte-swapped around the add when the target is big-endian (the flag at
//! `011c54ba`, read in place; never set in the PC build).

use super::{
    access, give_lock, head_of, kind_of, links, next_of, read_word, records, take_lock, word_at,
    EXTRA_PAYLOAD, FAMILY, OPERATOR_DELETE, OPERATOR_NEW, SIMPLE_LIST_NEW,
};
use crate::crt::low_word;
use crate::{cell_refs, crt, forms, pathing};
use engine_core::x87::x87_quiet_bits;
use engine_core::{args, typed_call, Engine, Ptr};

/// The save writer's "target is big-endian" byte (`00401500`'s `MOV AL,[0x011c54ba]`).
const BIG_ENDIAN: u32 = 0x011c_54ba;
/// The byte swaps of other units: a word, the word at +4, two words.
const SWAP_WORD_AT: u32 = 0x0050_3210;
const SWAP_WORD_AT_4: u32 = 0x0060_ce80;
const SWAP_TWO_WORDS: u32 = 0x0046_2230;
/// `BSSimpleList::Count`.
const LIST_COUNT: u32 = 0x005a_e380;
/// Other units' savers, constructors and string calls.
const RAGDOLL_DATA_SAVE: u32 = 0x004d_93c0;
const TELEPORT_DATA_SAVE: u32 = 0x0043_a420;
const MAP_MARKER_DATA_SAVE: u32 = 0x0043_8c10;
const MULTIBOUND_MARKER_DATA_SAVE: u32 = 0x0043_8f90;
const AUDIO_MARKER_DATA_SAVE: u32 = 0x0058_9560;
const PACKAGE_EVENT_ACTION_SAVE: u32 = 0x0067_dc80;
const DECAL_DATA_INIT: u32 = 0x0055_a400;
const PLANE_HALF_EXTENTS: u32 = 0x007d_6bb0;
const MATRIX_TO_AXIS_ANGLE: u32 = 0x00a5_8550;
const STRING_LENGTH: u32 = 0x0040_48e0;
const STRING_DESTROY: u32 = 0x0040_37d0;

/// A chunk's four letters as the word the exe pushes.
const fn tag(name: &[u8; 4]) -> u32 {
    u32::from_le_bytes(*name)
}

/// A form's id, in place (`0084e3a0`'s `MOV EAX,[ECX+0xc]`).
#[inline]
fn id_of(e: &Engine, form: u32) -> u32 {
    word_at(e, form, 0x0c)
}

/// Whether the target is big-endian, in place.
#[inline]
fn big_endian(e: &Engine) -> bool {
    e.mem.u8(BIG_ENDIAN) != 0
}

/// A simple list that holds nothing, in place (`008256d0`: both words zero).
#[inline]
fn list_empty(e: &Engine, list: u32) -> bool {
    word_at(e, list, 4) == 0 && word_at(e, list, 0) == 0
}

/// One word for `tag`.
fn put_word(e: &mut Engine, tag: u32, value: u32) {
    typed_call!(
        forms::FAMILY,
        0x0048_5910,
        forms::tesform::add_chunk_word(e, tag, value)
    );
}

/// One byte for `tag`.
fn put_byte(e: &mut Engine, tag: u32, value: u8) {
    typed_call!(
        forms::FAMILY,
        0x0048_58f0,
        forms::tesform::add_chunk_byte(e, tag, u32::from(value))
    );
}

/// `tag` with no data.
fn put_empty(e: &mut Engine, tag: u32) {
    typed_call!(
        forms::FAMILY,
        0x0048_56d0,
        forms::tesform::add_chunk(e, tag)
    );
}

/// The form id of `form` for `tag`.
fn put_form(e: &mut Engine, tag: u32, form: u32) {
    let id = id_of(e, form);
    put_word(e, tag, id);
}

/// The form id of `form` for `tag`, when there is a form.
fn put_form_if_any(e: &mut Engine, tag: u32, form: u32) {
    if form != 0 {
        put_form(e, tag, form);
    }
}

/// `size` bytes at `data` for `tag`, byte-swapped by `swap` around the add on a big-endian target.
fn put_swapped(e: &mut Engine, tag: u32, data: u32, size: u32, swap: fn(&mut Engine, u32)) {
    if big_endian(e) {
        swap(e, data);
    }
    typed_call!(
        forms::FAMILY,
        0x0048_5990,
        forms::tesform::add_chunk_data(e, tag, data, size)
    );
    if big_endian(e) {
        swap(e, data);
    }
}

/// `bytes` bytes (whole words) copied from `source` to `target`.
fn copy_bytes(e: &mut Engine, source: u32, target: u32, bytes: u32) {
    for offset in (0..bytes).step_by(4) {
        let word = e.mem.u32(source.wrapping_add(offset));
        e.mem.set_u32(target.wrapping_add(offset), word);
    }
}

/// The byte swaps the adds take, each a call of its function.
fn swap_word(e: &mut Engine, at: u32) {
    e.call(SWAP_WORD_AT, &args![at]);
}
fn swap_word_4(e: &mut Engine, at: u32) {
    e.call(SWAP_WORD_AT_4, &args![at]);
}
fn swap_pair(e: &mut Engine, at: u32) {
    e.call(SWAP_TWO_WORDS, &args![at]);
}
fn swap_record_16(e: &mut Engine, at: u32) {
    typed_call!(FAMILY, 0x0041_4220, records::swap_four_words(e, at));
}
fn swap_record_decal(e: &mut Engine, at: u32) {
    typed_call!(FAMILY, 0x0041_4030, records::swap_decal(e, at));
}
fn swap_record_portal(e: &mut Engine, at: u32) {
    typed_call!(FAMILY, 0x0041_41e0, records::swap_word_short(e, at));
}
fn swap_record_primitive(e: &mut Engine, at: u32) {
    typed_call!(FAMILY, 0x0041_4290, records::swap_primitive(e, at));
}
fn swap_record_plane_refs(e: &mut Engine, at: u32) {
    typed_call!(FAMILY, 0x0041_4370, records::swap_plane_refs(e, at));
}
fn swap_record_plane(e: &mut Engine, at: u32) {
    typed_call!(FAMILY, 0x0041_40f0, records::swap_plane(e, at));
}

/// `n` bytes copied by the C runtime family's `memcpy` wrapper (`00401460`).
fn copy_record(e: &mut Engine, target: u32, source: u32, n: u32) {
    typed_call!(
        crt::FAMILY,
        0x0040_1460,
        crt::mem::memcpy_wrapper(e, Ptr::new(target), Ptr::new(source), n)
    );
}

// Typed from 00412970
/// The save writer: walks the chain under the lock and saves each extra data. The compiler's
/// exception-unwinding frame is not translated.
pub fn save_list(e: &mut Engine, this: u32) {
    take_lock(e, 0);
    let mut current = head_of(e, this);
    while current != 0 {
        let kind = kind_of(e, current);
        save_one(e, this, current, kind);
        current = next_of(e, current);
    }
    give_lock(e);
}

/// One arm of the save writer's switch (the jump table at `00413dc8`).
fn save_one(e: &mut Engine, list: u32, extra: u32, kind: u8) {
    // The word at +0xc most arms use; the arms of the extra data without one (0x61, 0x80) never look
    // at it, so it reads as 0 where the address is not mapped.
    let mut bytes = [0u8; 4];
    let value = match e
        .mem
        .try_read(extra.wrapping_add(EXTRA_PAYLOAD), &mut bytes)
    {
        Ok(()) => u32::from_le_bytes(bytes),
        Err(_) => 0,
    };
    match kind {
        0x03 => put_form(e, tag(b"XCWT"), value),
        0x04 => save_regions(e, value),
        0x07 => put_form(e, tag(b"XCMO"), value),
        0x08 => put_form(e, tag(b"XCCM"), value),
        0x0e => {
            let action = e.mem.u8(extra.wrapping_add(EXTRA_PAYLOAD));
            put_word(e, tag(b"XACT"), u32::from(action));
        }
        // EXTRA_DISTANTDATA: three words.
        0x13 => e.with_stack(12, |e, record| {
            let at = record.addr();
            copy_bytes(e, extra.wrapping_add(EXTRA_PAYLOAD), at, 12);
            typed_call!(
                forms::FAMILY,
                0x0048_5710,
                forms::tesform::add_chunk_array_words(e, tag(b"XLOD"), at, 3u32)
            );
        }),
        0x14 => {
            e.call(RAGDOLL_DATA_SAVE, &args![value]);
        }
        // EXTRA_PACKAGESTARTLOC: the location as a form id, then the three words at +0x10.
        0x18 => e.with_stack(0x14, |e, record| {
            typed_call!(
                pathing::FAMILY,
                0x006d_5320,
                pathing::location::world_location_new(e, record)
            );
            let at = record.addr();
            let id = id_of(e, value);
            e.mem.set_u32(at, id);
            copy_bytes(e, extra.wrapping_add(0x10), at + 4, 12);
            put_swapped(e, tag(b"XPSL"), at, 0x14, swap_record_16);
        }),
        0x1e => put_word(e, tag(b"XLCM"), value),
        0x21 => put_form(e, tag(b"XOWN"), value),
        0x22 => put_form(e, tag(b"XGLB"), value),
        0x23 => put_word(e, tag(b"XRNK"), value),
        // EXTRA_COUNT: a signed 16-bit count.
        0x24 => {
            let count = e.mem.i16(extra.wrapping_add(EXTRA_PAYLOAD));
            put_word(e, tag(b"XCNT"), i32::from(count) as u32);
        }
        // EXTRA_HEALTH: the float as an integer (`_ftol2`).
        0x25 => {
            let health = e.mem.f32(extra.wrapping_add(EXTRA_PAYLOAD));
            let whole = typed_call!(
                crt::FAMILY,
                0x00ec_62c0,
                crt::math::ftol2_sse(e, f64::from(health))
            );
            put_word(e, tag(b"XHLT"), low_word(whole));
        }
        0x26 => {
            let uses = e.mem.u8(extra.wrapping_add(EXTRA_PAYLOAD));
            put_word(e, tag(b"XUSE"), u32::from(uses));
        }
        0x27 => put_word(e, tag(b"XTIM"), x87_quiet_bits(value)),
        0x28 => put_word(e, tag(b"XCHG"), x87_quiet_bits(value)),
        0x2a => save_lock(e, value),
        0x2b => {
            e.call(TELEPORT_DATA_SAVE, &args![value]);
        }
        0x2c => {
            put_empty(e, tag(b"XMRK"));
            e.call(MAP_MARKER_DATA_SAVE, &args![value]);
        }
        0x31 => {
            let seed = typed_call!(FAMILY, 0x0041_8b40, access::seed(e, list));
            put_byte(e, tag(b"XSED"), seed);
        }
        // EXTRA_ENABLESTATEPARENT: the parent as a form id and its flags byte.
        0x37 => {
            if value != 0 {
                e.with_stack(8, |e, record| {
                    let at = record.addr();
                    let id = id_of(e, value);
                    e.mem.set_u32(at, id);
                    let flags = e.mem.u8(extra.wrapping_add(0x10));
                    e.mem.set_u8(at + 4, flags);
                    put_swapped(e, tag(b"XESP"), at, 8, swap_word);
                });
            }
        }
        0x3b => put_form_if_any(e, tag(b"XRTM"), value),
        0x3c => put_form_if_any(e, tag(b"XMRC"), value),
        0x3f => put_form(e, tag(b"XPSN"), value),
        0x44 => put_form_if_any(e, tag(b"XTRG"), value),
        0x51 => put_form_if_any(e, tag(b"XLKR"), value),
        0x53 => save_activate_ref(e, list, extra),
        0x57 => save_decals(e, extra),
        0x59 => put_form(e, tag(b"XCIM"), value),
        // EXTRA_NAVMESH_PORTAL: the member's 8 bytes (`0069a690` in place: the word and the 16 bits
        // at +4) with the form id of its first word.
        0x5a => {
            let member = extra.wrapping_add(EXTRA_PAYLOAD);
            e.with_stack(8, |e, record| {
                let at = record.addr();
                let first = e.mem.u32(member);
                e.mem.set_u32(at, first);
                let triangle = e.mem.u16(member + 4);
                e.mem.set_u16(at + 4, triangle);
                if first != 0 {
                    let id = id_of(e, first);
                    e.mem.set_u32(at, id);
                    put_swapped(e, tag(b"XNDP"), at, 8, swap_record_portal);
                }
            });
        }
        0x5c => put_word(e, tag(b"XRDS"), x87_quiet_bits(value)),
        0x5d => put_word(e, tag(b"XRAD"), x87_quiet_bits(value)),
        0x61 => put_empty(e, tag(b"XMBP")),
        0x62 => {
            if value != 0 {
                e.call(MULTIBOUND_MARKER_DATA_SAVE, &args![value]);
            }
        }
        0x63 => put_form_if_any(e, tag(b"XMBR"), value),
        0x66 => save_reflectors(e, extra),
        0x67 => put_form_if_any(e, tag(b"XEMI"), value),
        // EXTRA_RADIO_DATA: the RADIO_DATA (0x10 bytes) with its position reference (+0xc) as a form
        // id while it is added.
        0x68 => {
            let data = extra.wrapping_add(EXTRA_PAYLOAD);
            let position = word_at(e, extra, 0x18);
            if position != 0 {
                let id = id_of(e, position);
                e.mem.set_u32(data + 0x0c, id);
            }
            put_swapped(e, tag(b"XRDO"), data, 0x10, swap_record_16);
            e.mem.set_u32(data + 0x0c, position);
        }
        0x6b => save_primitive(e, value),
        // EXTRA_AMMO: the ammo as a form id (0 for none), then the count.
        0x6e => {
            let count = word_at(e, extra, 0x10);
            let ammo_id = if value == 0 { 0 } else { id_of(e, value) };
            put_word(e, tag(b"XAMT"), ammo_id);
            put_word(e, tag(b"XAMC"), count);
        }
        // EXTRA_PATROL_REF_DATA: the time at the reference, then the PackageEventAction (`XPPA`).
        0x6f => {
            e.with_stack(4, |e, record| {
                let at = record.addr();
                typed_call!(FAMILY, 0x0041_3ff0, records::float_copy(e, at, value));
                put_swapped(e, tag(b"XPRD"), at, 4, swap_word);
            });
            e.call(
                PACKAGE_EVENT_ACTION_SAVE,
                &args![value.wrapping_add(4), tag(b"XPPA")],
            );
        }
        0x71 => save_plane(e, extra, tag(b"XOCP")),
        0x78 => save_plane(e, extra, tag(b"XPTL")),
        // EXTRA_COLLISION_DATA: the word it points at.
        0x72 => {
            if big_endian(e) {
                swap_word(e, value);
            }
            typed_call!(
                forms::FAMILY,
                0x0048_5990,
                forms::tesform::add_chunk_data(e, tag(b"XTRI"), value, 4u32)
            );
            if big_endian(e) {
                swap_word(e, value);
            }
        }
        0x74 => put_form_if_any(e, tag(b"XEZN"), value),
        // EXTRA_OCCLUSION_PLANE_REF_DATA: four linked references as form ids.
        0x76 => e.with_stack(16, |e, record| {
            let at = record.addr();
            typed_call!(
                cell_refs::FAMILY,
                0x0041_1dc0,
                cell_refs::extra_list::four_words_new(e, at)
            );
            for index in 0..4 {
                let reference = word_at(e, value, index * 4);
                let id = if reference == 0 {
                    0
                } else {
                    id_of(e, reference)
                };
                e.mem.set_u32(at + index * 4, id);
            }
            put_swapped(e, tag(b"XORD"), at, 0x10, swap_record_plane_refs);
        }),
        // EXTRA_PORTAL_REF_DATA: two linked references as form ids.
        0x77 => e.with_stack(8, |e, record| {
            let at = record.addr();
            typed_call!(FAMILY, 0x0041_43c0, records::portal_ref_new(e, at));
            for index in 0..2 {
                let reference = word_at(e, value, index * 4);
                let id = if reference == 0 {
                    0
                } else {
                    id_of(e, reference)
                };
                e.mem.set_u32(at + index * 4, id);
            }
            put_swapped(e, tag(b"XPOD"), at, 8, swap_pair);
        }),
        0x7a => put_word(e, tag(b"XHLP"), x87_quiet_bits(value)),
        0x7b => save_rooms(e, value),
        0x80 => put_empty(e, tag(b"XIBS")),
        0x81 => put_form(e, tag(b"XCAS"), value),
        // EXTRA_LIT_WATER_REFS: each reference as a form id.
        0x85 => {
            let mut node = extra.wrapping_add(EXTRA_PAYLOAD);
            while node != 0 && !list_empty(e, node) {
                let reference = e.mem.u32(node);
                let id = id_of(e, reference);
                put_word(e, tag(b"XLTW"), id);
                node = word_at(e, node, 4);
            }
        }
        // EXTRA_WEAPON_MOD_SLOTS: the active slots byte (under the health chunk's name, as the exe has
        // it).
        0x8d => {
            let slots = e.mem.u8(extra.wrapping_add(EXTRA_PAYLOAD));
            put_byte(e, tag(b"XHLT"), slots);
        }
        0x90 => {
            put_empty(e, tag(b"MMRK"));
            e.call(AUDIO_MARKER_DATA_SAVE, &args![value]);
        }
        // EXTRA_AUDIOBUOYMARKER: its saver is an empty body (`00483710`), so only the name.
        0x91 => put_empty(e, tag(b"AMRK")),
        // EXTRA_SPECIAL_RENDER_FLAGS: the form id reader on the extra data itself (the word at +0xc),
        // then the float at +0x10.
        0x92 => {
            let id = id_of(e, extra);
            put_word(e, tag(b"XSRF"), id);
            let second = word_at(e, extra, 0x10);
            put_word(e, tag(b"XSRD"), x87_quiet_bits(second));
        }
        _ => {}
    }
}

/// EXTRA_REGIONLIST: the regions as an array of form ids.
fn save_regions(e: &mut Engine, regions: u32) {
    let mut node = if regions == 0 { 0 } else { regions + 4 };
    let count = e.call(LIST_COUNT, &args![node]).u32();
    if count == 0 {
        return;
    }
    let buffer = e.call(OPERATOR_NEW, &args![count.saturating_mul(4)]).u32();
    let mut index = 0u32;
    while node != 0 {
        let region = e.mem.u32(node);
        let id = id_of(e, region);
        e.mem.set_u32(buffer.wrapping_add(index * 4), id);
        index += 1;
        node = word_at(e, node, 4);
    }
    typed_call!(
        forms::FAMILY,
        0x0048_5710,
        forms::tesform::add_chunk_array_words(e, tag(b"XCLR"), buffer, count)
    );
    e.call(OPERATOR_DELETE, &args![buffer]);
}

/// EXTRA_LOCK: a REFR_LOCK (0x14 bytes) with the key as a form id.
fn save_lock(e: &mut Engine, lock_data: u32) {
    e.with_stack(0x14, |e, record| {
        let at = record.addr();
        typed_call!(
            cell_refs::FAMILY,
            0x0041_1b00,
            cell_refs::extra_list::lock_data_new(e, at)
        );
        let flags = e.mem.u8(lock_data.wrapping_add(8));
        e.mem.set_u8(at + 8, flags);
        let base_level = e.mem.u8(lock_data);
        e.mem.set_u8(at, base_level);
        let key = word_at(e, lock_data, 4);
        let key_id = if key == 0 { 0 } else { id_of(e, key) };
        e.mem.set_u32(at + 4, key_id);
        put_swapped(e, tag(b"XLOC"), at, 0x14, swap_word_4);
    });
}

/// EXTRA_DECAL_REFS: each REF_DECAL_DATA (0x1c bytes) with its reference as a form id.
fn save_decals(e: &mut Engine, extra: u32) {
    let mut node = extra.wrapping_add(EXTRA_PAYLOAD);
    while node != 0 && !list_empty(e, node) {
        let decal = e.mem.u32(node);
        e.with_stack(0x1c, |e, record| {
            let at = record.addr();
            e.call(DECAL_DATA_INIT, &args![at]);
            copy_record(e, at, decal, 0x1c);
            let reference = e.mem.u32(decal);
            let id = id_of(e, reference);
            e.mem.set_u32(at, id);
            put_swapped(e, tag(b"XDCR"), at, 0x1c, swap_record_decal);
        });
        node = word_at(e, node, 4);
    }
}

/// EXTRA_REFLECTOR_REFS: each entry (a reference and a word) with the reference as a form id.
fn save_reflectors(e: &mut Engine, extra: u32) {
    let mut node = extra.wrapping_add(EXTRA_PAYLOAD);
    while node != 0 && !list_empty(e, node) {
        let data = e.mem.u32(node);
        e.with_stack(8, |e, record| {
            let at = record.addr();
            e.call(SIMPLE_LIST_NEW, &args![at]);
            copy_record(e, at, data, 8);
            let reference = e.mem.u32(data);
            let id = id_of(e, reference);
            e.mem.set_u32(at, id);
            put_swapped(e, tag(b"XPWR"), at, 8, swap_pair);
        });
        node = word_at(e, node, 4);
    }
}

/// EXTRA_PRIMITIVE: the primitive's radii, colour and type (+4).
fn save_primitive(e: &mut Engine, primitive: u32) {
    e.with_stack(0x20, |e, record| {
        let at = record.addr();
        typed_call!(FAMILY, 0x0041_43f0, records::primitive_new(e, at));
        e.with_stack(12, |e, radii| {
            let radii = typed_call!(
                FAMILY,
                0x0041_3fc0,
                records::primitive_radii(e, primitive, radii.addr())
            );
            copy_bytes(e, radii, at, 12);
        });
        e.with_stack(16, |e, color| {
            let color = typed_call!(
                FAMILY,
                0x0041_3f90,
                records::primitive_color(e, primitive, color.addr())
            );
            copy_bytes(e, color, at + 0x0c, 16);
        });
        let kind = word_at(e, primitive, 4);
        e.mem.set_u32(at + 0x1c, kind);
        put_swapped(e, tag(b"XPRM"), at, 0x20, swap_record_primitive);
    });
}

/// EXTRA_ROOM_REF_DATA: the count of linked portals with the master flag in bit 16, then each linked
/// room as a form id.
fn save_rooms(e: &mut Engine, data: u32) {
    let mut count = e.call(LIST_COUNT, &args![data.wrapping_add(8)]).u32();
    if e.mem.u8(data.wrapping_add(0x10)) != 0 {
        count |= 0x10000;
    }
    put_word(e, tag(b"XRMR"), count);
    let mut node = data.wrapping_add(8);
    while node != 0 {
        let room = e.mem.u32(node);
        if room == 0 {
            break;
        }
        let id = id_of(e, room);
        put_word(e, tag(b"XLRM"), id);
        node = word_at(e, node, 4);
    }
}

/// EXTRA_ACTIVATE_REF: the flags byte (`XAPD`), each REF_ACTIVATE_DATA of the parent list (`XAPR`),
/// then the activate text override (`XATO`): `GetActivateTextOverride` (`0041ec80`) copies it twice,
/// the first copy for the length, the second for the characters (the length + 1 as the size).
fn save_activate_ref(e: &mut Engine, list: u32, extra: u32) {
    let flags = e.mem.u8(extra.wrapping_add(0x14));
    put_byte(e, tag(b"XAPD"), flags);
    let mut node = extra.wrapping_add(EXTRA_PAYLOAD);
    while node != 0 && !list_empty(e, node) {
        let data = e.mem.u32(node);
        node = word_at(e, node, 4);
        e.with_stack(8, |e, record| {
            let at = record.addr();
            typed_call!(FAMILY, 0x0041_4010, records::empty_node_new(e, at));
            copy_record(e, at, data, 8);
            let reference = e.mem.u32(at);
            let id = id_of(e, reference);
            e.mem.set_u32(at, id);
            put_swapped(e, tag(b"XAPR"), at, 8, swap_pair);
        });
    }
    let text = extra.wrapping_add(0x18);
    if e.call(STRING_LENGTH, &args![text]).u32() != 0 {
        e.with_stack(8, |e, first| {
            e.with_stack(8, |e, second| {
                let first = typed_call!(
                    FAMILY,
                    0x0041_ec80,
                    links::activate_text(e, list, first.addr())
                );
                let length = e.call(STRING_LENGTH, &args![first]).u32();
                let size = length.wrapping_add(1);
                let second = typed_call!(
                    FAMILY,
                    0x0041_ec80,
                    links::activate_text(e, list, second.addr())
                );
                let characters = read_word(e, second);
                typed_call!(
                    forms::FAMILY,
                    0x0048_56f0,
                    forms::tesform::add_chunk_array(e, tag(b"XATO"), characters, size)
                );
                e.call(STRING_DESTROY, &args![second]);
                e.call(STRING_DESTROY, &args![first]);
            });
        });
    }
}

/// EXTRA_OCCLUSION_PLANE / EXTRA_PORTAL: nine words, the plane's half extents (two), centre (three),
/// then its rotation as an axis (three floats) and an angle (`NiMatrix3`, `00a58550`).
fn save_plane(e: &mut Engine, extra: u32, tag: u32) {
    e.with_stack(0x24, |e, record| {
        let at = record.addr();
        typed_call!(FAMILY, 0x0041_4470, records::plane_new(e, at));
        let plane = read_word(e, extra.wrapping_add(EXTRA_PAYLOAD));
        let half_extents = e.call(PLANE_HALF_EXTENTS, &args![plane]).u32();
        copy_bytes(e, half_extents, at, 8);
        let center = typed_call!(FAMILY, 0x0041_3f40, records::plane_center(e, plane));
        copy_bytes(e, center, at + 8, 12);
        e.with_stack(0x24, |e, matrix| {
            let matrix = typed_call!(
                FAMILY,
                0x0041_3f60,
                records::plane_rotation(e, plane, matrix.addr())
            );
            e.call(
                MATRIX_TO_AXIS_ANGLE,
                &args![matrix, at + 0x20, at + 0x14, at + 0x18, at + 0x1c],
            );
        });
        put_swapped(e, tag, at, 0x24, swap_record_plane);
    });
}
