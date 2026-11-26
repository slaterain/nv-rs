//! `ExtraDataList::AddExtraCopy` (Xbox PDB, `004103d0`) and the copies and compares of whole lists
//! (`00411ec0` to `004127e0`): `CopyList`, `IsCopyableExtra`, `RemoveAllCopyableExtra`,
//! `RemoveNonPersistentCellData` and its thread flag, `CopyListForContainer`,
//! `DuplicateExtraListForContainer`, `CopyListForReference`, `CompareListForContainer` and
//! `CompareList`. `AddExtraCopy` copies an extra data through the setter of its type: the exe's jump
//! table over 82 types, here a `match`, each setter of this family a typed call and the others called
//! by address as the exe calls them.

use super::{
    access, cell_data, cell_refs, copy_words, delete_extra_data, first_of_kind, followers,
    forget_cached_all, give_lock, head_of, kind_of, links, list_remove, new_object, next_node,
    next_of, packages, read_word, rooms, set_next_of, setters, take_lock, word_at, EXTRA_PAYLOAD,
    FAMILY, LIST_HEAD,
};
use crate::crt;
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// `__RTDynamicCast`.
const RT_DYNAMIC_CAST: u32 = 0x00ec_43fb;
/// The log `AddExtraCopy` reports a type with no copy to (`overridable.tsv`), and its message.
const LOG_MESSAGE: u32 = 0x005b_5e40;
const NO_COPY_MESSAGE: u32 = 0x0101_43ec;
/// The names the copies and compares give the extra data lock.
const NAME_COPY_LIST: u32 = 0x0101_4468;
const NAME_REMOVE_ALL_COPYABLE: u32 = 0x0101_4484;
const NAME_REMOVE_NON_PERSISTENT: u32 = 0x0101_44ac;
const NAME_COPY_LIST_FOR_CONTAINER: u32 = 0x0101_44dc;
const NAME_DUPLICATE_FOR_CONTAINER: u32 = 0x0101_4504;
const NAME_COPY_LIST_FOR_REFERENCE: u32 = 0x0101_4534;
const NAME_COMPARE_LIST_FOR_CONTAINER: u32 = 0x0101_455c;
const NAME_COMPARE_LIST: u32 = 0x0101_4588;
/// The thread's word at +0x294 of the TLS block whose bit 2 `RemoveNonPersistentCellData` tests.
const THREAD_FLAGS: u32 = 0x294;
/// `ExtraScript`'s constructor (`thiscall(block, script)`) and `Script`'s compiled result
/// (`thiscall(script)`).
const EXTRA_SCRIPT_INIT: u32 = 0x0043_2000;
const SCRIPT_GET_RESULT: u32 = 0x005a_bf60;
/// JIP LN NVSE's keyed data (`ExtraJIP`), whose arm JIP writes into the jump table.
const JIP_KEYED_DATA_TYPE: u8 = 0x27;

/// The copies of the word at +0xc (`list.Setter(word)`): this family's setters as typed calls, the
/// others by address. False for a type with no such setter.
fn copy_word(e: &mut Engine, list: u32, kind: u8, extra: u32) -> bool {
    if !matches!(
        kind,
        0x03 | 0x07
            | 0x08
            | 0x14
            | 0x1a
            | 0x1c
            | 0x1e
            | 0x20
            | 0x21
            | 0x22
            | 0x23
            | 0x3b
            | 0x3c
            | 0x3f
            | 0x44
            | 0x46
            | 0x51
            | 0x59
            | 0x63
            | 0x67
            | 0x74
            | 0x81
    ) {
        return false;
    }
    let word = word_at(e, extra, EXTRA_PAYLOAD);
    match kind {
        0x20 => {
            typed_call!(
                FAMILY,
                0x0041_8550,
                access::set_original_reference(e, list, word)
            );
        }
        0x21 => {
            typed_call!(FAMILY, 0x0041_9700, access::set_ownership(e, list, word));
        }
        0x22 => {
            typed_call!(FAMILY, 0x0041_97d0, access::set_global(e, list, word));
        }
        0x23 => {
            typed_call!(FAMILY, 0x0041_98a0, access::set_rank(e, list, word as i32));
        }
        0x3f => {
            typed_call!(FAMILY, 0x0041_9d10, access::set_poison(e, list, word));
        }
        0x46 => {
            typed_call!(FAMILY, 0x0041_9dc0, access::set_item_dropper(e, list, word));
        }
        0x07 => {
            typed_call!(
                FAMILY,
                0x0041_bd10,
                cell_data::set_music_type(e, list, word)
            );
        }
        0x08 => {
            typed_call!(FAMILY, 0x0041_c190, cell_data::set_climate(e, list, word));
        }
        0x14 => {
            typed_call!(
                FAMILY,
                0x0041_d5b0,
                packages::set_rag_doll_data_from_copy(e, list, word)
            );
        }
        0x1a => {
            typed_call!(
                FAMILY,
                0x0041_cc90,
                packages::set_trespass_package(e, list, word)
            );
        }
        0x1c => {
            typed_call!(
                FAMILY,
                0x0041_c7f0,
                cell_data::set_reference_pointer(e, list, word)
            );
        }
        0x59 => {
            typed_call!(
                FAMILY,
                0x0041_c290,
                cell_data::set_image_space(e, list, word)
            );
        }
        0x81 => {
            typed_call!(
                FAMILY,
                0x0041_c090,
                cell_data::set_acoustic_space(e, list, word)
            );
        }
        0x03 => {
            typed_call!(FAMILY, 0x0041_e160, links::set_water_type(e, list, word));
        }
        0x1e => {
            typed_call!(
                FAMILY,
                0x0042_1540,
                followers::set_lev_crea_modifier(e, list, word)
            );
        }
        0x3b => {
            typed_call!(
                FAMILY,
                0x0041_e250,
                links::set_teleport_marker(e, list, word)
            );
        }
        0x3c => {
            typed_call!(
                FAMILY,
                0x0042_1430,
                followers::set_merchant_container(e, list, word)
            );
        }
        0x44 => {
            typed_call!(FAMILY, 0x0042_1b70, followers::set_x_target(e, list, word));
        }
        0x51 => {
            typed_call!(FAMILY, 0x0041_e440, links::set_linked_ref(e, list, word));
        }
        0x63 => {
            typed_call!(
                FAMILY,
                0x0042_1e40,
                followers::set_multibound_ref(e, list, word)
            );
        }
        0x67 => {
            typed_call!(
                FAMILY,
                0x0042_1d50,
                followers::set_emittance_source(e, list, word)
            );
        }
        0x74 => {
            typed_call!(
                FAMILY,
                0x0042_1c60,
                followers::set_encounter_zone(e, list, word)
            );
        }
        _ => unreachable!(),
    }
    true
}

/// The copies of the `float` at +0xc. False for a type with no such setter.
fn copy_float(e: &mut Engine, list: u32, kind: u8, extra: u32) -> bool {
    if !matches!(kind, 0x25 | 0x27 | 0x28 | 0x30 | 0x5c | 0x5d | 0x7a) {
        return false;
    }
    let value = f32::from_bits(word_at(e, extra, EXTRA_PAYLOAD));
    match kind {
        0x25 => {
            typed_call!(FAMILY, 0x0041_9970, access::set_health(e, list, value));
        }
        0x27 => {
            typed_call!(FAMILY, 0x0041_9bb0, access::set_time_left(e, list, value));
        }
        0x28 => {
            typed_call!(FAMILY, 0x0041_9c60, access::set_charge(e, list, value));
        }
        0x30 => {
            typed_call!(FAMILY, 0x0041_9fb0, access::set_scale(e, list, value));
        }
        0x5c => {
            typed_call!(FAMILY, 0x0042_2220, followers::set_radius(e, list, value));
        }
        0x7a => {
            typed_call!(
                FAMILY,
                0x0041_b580,
                setters::set_health_perc(e, list, value)
            );
        }
        0x5d => {
            typed_call!(
                FAMILY,
                0x0042_2350,
                followers::set_radiation(e, list, value)
            );
        }
        _ => unreachable!(),
    }
    true
}

/// The copies of the byte at +0xc. False for a type with no such setter.
fn copy_byte(e: &mut Engine, list: u32, kind: u8, extra: u32) -> bool {
    if !matches!(kind, 0x0e | 0x26 | 0x31 | 0x4a) {
        return false;
    }
    let byte = e.mem.u8(extra.wrapping_add(EXTRA_PAYLOAD));
    let setter = match kind {
        0x26 => {
            typed_call!(FAMILY, 0x0041_9a20, access::set_uses(e, list, byte));
            return true;
        }
        0x0e => {
            typed_call!(
                FAMILY,
                0x0041_b3d0,
                setters::set_action_flags(e, list, u32::from(byte))
            );
            return true;
        }
        0x31 => {
            typed_call!(FAMILY, 0x0041_ac30, setters::set_seed(e, list, byte));
            return true;
        }
        0x4a => 0x0042_dde0,
        _ => unreachable!(),
    };
    e.call(setter, &args![list, byte]);
    true
}

/// The dispatch family's empty `thiscall(a, b)` body (`RET 8`).
const NOTHING_2: u32 = 0x0045_34f0;

/// The copies that make a new payload object: (size, constructor, the object's copy from the
/// source's word), by type.
fn new_object_copy(kind: u8) -> Option<(u32, u32, u32)> {
    match kind {
        0x2b => Some((0x20, 0x0043_a160, 0x0043_a810)),
        0x2c => Some((0x14, 0x0043_8bb0, 0x0043_8df0)),
        0x6f => Some((0x14, 0x0067_c650, 0x0067_c6f0)),
        0x8c => Some((0x15c, 0x0058_efd0, 0x0058_f4c0)),
        0x90 => Some((0x34, 0x0058_9450, 0x0058_9770)),
        0x91 => Some((0x08, 0x0068_0890, 0x0045_34f0)),
        _ => None,
    }
}

/// The list's setter of a new payload object, by type: this family's as typed calls.
fn set_new_object(e: &mut Engine, list: u32, kind: u8, object: u32) {
    match kind {
        0x2b => {
            typed_call!(FAMILY, 0x0041_9120, access::set_teleport(e, list, object));
        }
        0x2c => {
            typed_call!(FAMILY, 0x0041_9250, access::set_map_marker(e, list, object));
        }
        0x90 => {
            typed_call!(
                FAMILY,
                0x0041_9380,
                access::set_audio_marker(e, list, object)
            );
        }
        0x91 => {
            typed_call!(
                FAMILY,
                0x0041_94b0,
                access::set_audio_buoy_marker(e, list, object)
            );
        }
        0x6f => {
            typed_call!(
                FAMILY,
                0x0041_fc10,
                rooms::set_patrol_ref_data(e, list, object)
            );
        }
        _ => {
            typed_call!(
                FAMILY,
                0x0041_c390,
                cell_data::set_impact_swap(e, list, object)
            );
        }
    }
}

/// The first extra data of `kind`, one of `size` bytes built by `construct` and added when the list
/// has none (`AddExtraCopy`'s list and structure arms).
fn get_or_add(
    e: &mut Engine,
    list: u32,
    kind: u8,
    size: u32,
    construct: impl FnOnce(&mut Engine, u32) -> u32,
) -> u32 {
    let mut existing = first_of_kind(e, list, kind);
    if existing == 0 {
        existing = new_object(e, size, construct);
        super::list_add(e, list, existing);
    }
    existing
}

/// `AddExtraCopy`'s body, as called from `caller` (the exe's return address of the call, 0 for the
/// callers JIP does not tell apart) with that caller's flag: the two words JIP LN NVSE's copy hook
/// reads from the frame to choose whether its keyed data moves or is copied.
fn add_extra_copy_from(e: &mut Engine, list: u32, extra: u32, caller: u32, caller_flag: u32) {
    if extra == 0 || !typed_call!(FAMILY, 0x0041_1f20, is_copyable_extra(e, list, extra)) {
        return;
    }
    let kind = kind_of(e, extra);
    // JIP's ExtraJIP framework puts its own arm in the jump table's word of type 0x27
    // (`SafeWrite32(0x411978, ...)`) while it is installed: the site does the arm.
    if e.jip_patches() & engine_core::extend::jip_patches::INSTALLED != 0
        && kind == JIP_KEYED_DATA_TYPE
        && e.site(0x0041_1978, &[list, extra, caller, caller_flag])
            .is_some()
    {
        return;
    }
    if copy_word(e, list, kind, extra)
        || copy_float(e, list, kind, extra)
        || copy_byte(e, list, kind, extra)
    {
        return;
    }
    if kind == 0x02 {
        let node = read_word(e, extra.wrapping_add(EXTRA_PAYLOAD));
        typed_call!(FAMILY, 0x0041_b7e0, cell_data::set_cell_3d(e, list, node));
        return;
    }
    if kind == 0x79 {
        let word = read_word(e, extra.wrapping_add(EXTRA_PAYLOAD));
        typed_call!(FAMILY, 0x0042_0f00, rooms::set_room(e, list, word));
        return;
    }
    if kind == 0x61 {
        let word = read_word(e, extra.wrapping_add(EXTRA_PAYLOAD));
        typed_call!(
            FAMILY,
            0x0042_2050,
            followers::set_multi_bound(e, list, word)
        );
        return;
    }
    if kind == 0x71 {
        let word = read_word(e, extra.wrapping_add(EXTRA_PAYLOAD));
        typed_call!(
            FAMILY,
            0x0042_2150,
            followers::set_occlusion_plane(e, list, word)
        );
        return;
    }
    match kind {
        0x13 => {
            typed_call!(
                FAMILY,
                0x0041_d950,
                packages::set_land_normal(e, list, extra.wrapping_add(EXTRA_PAYLOAD))
            );
            return;
        }
        0x68 => {
            typed_call!(
                FAMILY,
                0x0041_8310,
                access::set_radio_data(e, list, extra.wrapping_add(EXTRA_PAYLOAD))
            );
            return;
        }
        _ => {}
    }
    if let Some((size, construct, copy)) = new_object_copy(kind) {
        let word = word_at(e, extra, EXTRA_PAYLOAD);
        let object = new_object(e, size, |e, block| e.call(construct, &args![block]).u32());
        // The audio buoy marker's copy is the dispatch family's empty body (`004534f0`, `RET 8`),
        // done in place: nothing.
        if copy != NOTHING_2 {
            e.call(copy, &args![object, word]);
        }
        set_new_object(e, list, kind, object);
        return;
    }
    copy_by_arm(e, list, extra, kind);
}

/// A walk of the `BSSimpleList` from `node` (its items in place: the node is its item's slot),
/// handing each item to `each`; with `stop_at_empty` the walk ends at the first empty item.
fn each_item(
    e: &mut Engine,
    mut node: u32,
    stop_at_empty: bool,
    mut each: impl FnMut(&mut Engine, u32),
) {
    while node != 0 {
        let item = e.mem.u32(node);
        if stop_at_empty && item == 0 {
            break;
        }
        each(e, item);
        node = next_node(e, node);
    }
}

/// `AddExtraCopy`'s arms of their own (the jump table's other entries).
fn copy_by_arm(e: &mut Engine, list: u32, extra: u32, kind: u8) {
    match kind {
        0x0a => {
            // The canopy shadow mask's setter hands the structure it made back through `result`;
            // the source's words at +0x14 and +0x18 then go into its first two words.
            let word = word_at(e, extra, 0x0c);
            let handle = read_word(e, extra.wrapping_add(0x10));
            // `result` is a local of the frame, as in the exe.
            e.with_stack(4, |e, result| {
                typed_call!(
                    FAMILY,
                    0x0041_c490,
                    cell_data::set_canopy_shadow_mask(e, list, word, handle, result.addr())
                );
                let created = e.mem.u32(result.addr());
                let second = word_at(e, extra, 0x18);
                e.mem.set_u32(created.wrapping_add(4), second);
                let first = word_at(e, extra, 0x14);
                e.mem.set_u32(created, first);
            });
        }
        0x0d => {
            let (first, second) = (word_at(e, extra, 0x0c), word_at(e, extra, 0x10));
            typed_call!(FAMILY, 0x0041_9ed0, access::set_script(e, list, first));
            typed_call!(
                FAMILY,
                0x0041_9f80,
                access::set_script_locals(e, list, second)
            );
        }
        0x16 | 0x17 => {
            typed_call!(
                FAMILY,
                0x0041_aa20,
                setters::set_worn(e, list, true, kind == 0x17)
            );
        }
        0x18 => {
            let word = word_at(e, extra, 0x0c);
            let first_cast = e
                .call(
                    RT_DYNAMIC_CAST,
                    &args![word, 0u32, 0x0118_3028u32, 0x0118_3fd0u32, 0u32],
                )
                .u32();
            let second_cast = e
                .call(
                    RT_DYNAMIC_CAST,
                    &args![word, 0u32, 0x0118_3028u32, 0x0118_3fb4u32, 0u32],
                )
                .u32();
            let last = f32::from_bits(word_at(e, extra, 0x1c));
            typed_call!(
                FAMILY,
                0x0041_ad00,
                setters::set_package_start_location(
                    e,
                    list,
                    first_cast,
                    second_cast,
                    extra.wrapping_add(0x10),
                    last
                )
            );
        }
        0x19 => {
            let words = [0x0cu32, 0x10, 0x14].map(|offset| word_at(e, extra, offset));
            let bytes = [0x18u32, 0x19, 0x1a].map(|offset| e.mem.u8(extra.wrapping_add(offset)));
            typed_call!(
                FAMILY,
                0x0041_c930,
                packages::set_package(
                    e,
                    list,
                    words[0],
                    words[1] as i32,
                    words[2],
                    bytes[0],
                    bytes[1],
                    bytes[2]
                )
            );
        }
        0x1b => each_item(e, word_at(e, extra, 0x0c), false, |e, entry| {
            if entry != 0 {
                let (word, flag) = (e.mem.u32(entry), e.mem.u8(entry.wrapping_add(4)));
                typed_call!(
                    FAMILY,
                    0x0041_d700,
                    packages::add_run_once_package(e, list, word, flag)
                );
            }
        }),
        0x1d => each_item(e, word_at(e, extra, 0x0c), true, |e, follower| {
            typed_call!(
                FAMILY,
                0x0042_2480,
                followers::add_follower(e, list, follower)
            );
        }),
        0x2a => {
            // `ExtraLock`: a copy of the 0x14-byte lock record.
            let source = word_at(e, extra, 0x0c);
            let object = new_object(e, 0x14, |e, block| {
                typed_call!(
                    cell_refs::FAMILY,
                    0x0041_1b00,
                    cell_refs::extra_list::lock_data_new(e, block)
                )
            });
            typed_call!(
                crt::FAMILY,
                0x0040_1460,
                crt::mem::memcpy_wrapper(e, Ptr::new(object), Ptr::new(source), 0x14)
            );
            typed_call!(FAMILY, 0x0041_9050, access::set_lock_ptr(e, list, object));
        }
        0x2f | 0x37 => {
            let word = word_at(e, extra, 0x0c);
            let flag = e.mem.u8(extra.wrapping_add(0x10));
            if kind == 0x2f {
                typed_call!(
                    FAMILY,
                    0x0041_d280,
                    packages::set_leveled_item_index(e, list, word as i32)
                );
                typed_call!(
                    FAMILY,
                    0x0041_d330,
                    packages::set_leveled_item_default(e, list, flag)
                );
            } else {
                typed_call!(
                    FAMILY,
                    0x0041_da40,
                    packages::set_enable_state_parent(e, list, word)
                );
                typed_call!(FAMILY, 0x0041_dc70, links::set_parent_flags(e, list, flag));
            }
        }
        0x24 => {
            let count = e.mem.u16(extra.wrapping_add(0x0c));
            typed_call!(FAMILY, 0x0041_9ad0, access::set_count(e, list, count));
        }
        0x3e => {
            typed_call!(
                FAMILY,
                0x0041_ab70,
                setters::set_can_not_wear(e, list, true)
            );
        }
        0x48 => {
            let value = e.mem.f32(extra.wrapping_add(0x0c));
            typed_call!(
                FAMILY,
                0x0042_2750,
                followers::set_refraction_property(e, list, 1, value)
            );
        }
        0x4c => {
            let existing = get_or_add(e, list, 0x4c, 0x30, |e, block| {
                e.call(0x0043_5fa0, &args![block, 0u32]).u32()
            });
            // The payload from +0xc to +0x2f (nine words) copied whole.
            copy_words(e, extra, existing, 0x0c, 0x30);
        }
        0x53 => {
            let existing = get_or_add(e, list, 0x53, 0x20, |e, block| {
                e.call(0x0043_38b0, &args![block]).u32()
            });
            e.call(0x0043_3b70, &args![existing, extra]);
        }
        0x55 => {}
        0x57 => {
            let existing = get_or_add(e, list, 0x57, 0x14, |e, block| {
                e.call(0x0043_3ca0, &args![block]).u32()
            });
            e.call(0x0043_42b0, &args![existing, extra]);
        }
        0x5a => {
            e.call(0x0042_e2c0, &args![list, extra]);
        }
        0x5e => {
            e.call(0x0042_e760, &args![list]);
            let target = e.call(0x0042_e800, &args![list]).u32();
            each_item(e, word_at(e, extra, 0x0c), true, |e, entry| {
                // The entry is a word and a signed byte.
                let word = e.mem.u32(entry);
                let small = i32::from(e.mem.u8(entry.wrapping_add(4)) as i8);
                e.call(0x0043_6f80, &args![target, word, small]);
            });
        }
        0x62 => {
            let source = word_at(e, extra, 0x0c);
            let mut object = 0;
            if source != 0 {
                object = new_object(e, 0xc, |e, block| e.call(0x0043_8f50, &args![block]).u32());
                e.call(0x0043_9120, &args![object, source]);
            }
            typed_call!(
                FAMILY,
                0x0042_1f30,
                followers::set_multibound_data(e, list, object)
            );
        }
        0x65 => {
            let existing = get_or_add(e, list, 0x65, 0x14, |e, block| {
                typed_call!(
                    cell_refs::FAMILY,
                    0x0041_1b40,
                    cell_refs::extra_list::reflected_refs_new(e, block)
                )
            });
            e.call(0x0043_46f0, &args![existing, extra]);
        }
        0x66 => {
            let existing = get_or_add(e, list, 0x66, 0x14, |e, block| {
                typed_call!(
                    cell_refs::FAMILY,
                    0x0041_1be0,
                    cell_refs::extra_list::reflector_refs_new(e, block)
                )
            });
            e.call(0x0043_4c80, &args![existing, extra]);
        }
        0x6b => {
            let word = word_at(e, extra, 0x0c);
            let converted = e.call(0x004a_4d40, &args![word]).u32();
            typed_call!(
                FAMILY,
                0x0041_fa60,
                rooms::add_primitive(e, list, converted)
            );
        }
        0x6e => {
            let (first, second) = (word_at(e, extra, 0x0c), word_at(e, extra, 0x10));
            e.call(0x0042_eb60, &args![list, first, second]);
        }
        0x72 => {
            let word = word_at(e, extra, 0x0c);
            let object = new_object(e, 4, |e, block| {
                typed_call!(
                    cell_refs::FAMILY,
                    0x0041_1e00,
                    cell_refs::extra_list::word_16_new(e, block)
                )
            });
            let read = read_word(e, word);
            e.call(0x0053_7e90, &args![object, read]);
            typed_call!(
                FAMILY,
                0x0042_0fd0,
                rooms::set_collision_data(e, list, object)
            );
        }
        0x73 => each_item(e, word_at(e, extra, 0x0c), true, |e, entry| {
            e.call(0x0042_ef20, &args![list, entry]);
        }),
        0x76 => {
            let source = word_at(e, extra, 0x0c);
            let object = new_object(e, 0x10, |e, block| {
                typed_call!(
                    cell_refs::FAMILY,
                    0x0041_1dc0,
                    cell_refs::extra_list::four_words_new(e, block)
                )
            });
            copy_words(e, source, object, 0, 0x10);
            typed_call!(
                FAMILY,
                0x0041_fec0,
                rooms::set_occlusion_plane_ref_data(e, list, object)
            );
        }
        0x77 => {
            let source = word_at(e, extra, 0x0c);
            for index in 0..2i32 {
                let value = e.mem.u32(source.wrapping_add(4 * index as u32));
                typed_call!(
                    FAMILY,
                    0x0042_0a60,
                    rooms::set_portal_word(e, list, index, value)
                );
            }
        }
        0x78 => {
            // The exe makes a temporary (a local) for a null extra data, which the test at the top
            // rules out: the word read is the payload.s.
            let word = read_word(e, extra.wrapping_add(0x0c));
            typed_call!(FAMILY, 0x0042_0e00, rooms::set_portal(e, list, word));
        }
        0x7b => {
            let room_data = word_at(e, extra, 0x0c);
            each_item(e, room_data.wrapping_add(8), true, |e, entry| {
                typed_call!(FAMILY, 0x0042_0c10, rooms::add_room(e, list, entry));
            });
            let flag = e.mem.u8(room_data.wrapping_add(0x10)) != 0;
            typed_call!(
                FAMILY,
                0x0042_0870,
                rooms::set_room_is_master(e, list, flag)
            );
        }
        0x80 => {
            e.call(0x0042_f200, &args![list, 1u32]);
        }
        0x84 => {
            let existing = get_or_add(e, list, 0x84, 0x14, |e, block| {
                typed_call!(
                    cell_refs::FAMILY,
                    0x0041_1c80,
                    cell_refs::extra_list::water_light_refs_new(e, block)
                )
            });
            e.call(0x0043_4a70, &args![existing, extra]);
        }
        0x85 => {
            let existing = get_or_add(e, list, 0x85, 0x14, |e, block| {
                typed_call!(
                    cell_refs::FAMILY,
                    0x0041_1d20,
                    cell_refs::extra_list::lit_water_refs_new(e, block)
                )
            });
            e.call(0x0043_4ee0, &args![existing, extra]);
        }
        0x8b => {
            // The get-or-create (`0042f420`) adds the new extra data itself.
            let target = e.call(0x0042_f420, &args![list]).u32();
            copy_words(e, extra, target, 0x10, 0x1c);
            let word = word_at(e, extra, 0x1c);
            e.mem.set_u32(target.wrapping_add(0x1c), word);
            let first = word_at(e, extra, 0x0c);
            e.mem.set_u32(target.wrapping_add(0x0c), first);
            let into = target.wrapping_add(0x20);
            e.call(0x0047_0470, &args![into]);
            let mut node = extra.wrapping_add(0x20);
            while node != 0 {
                if e.mem.u32(node) != 0 {
                    e.call(0x0090_5820, &args![into, node]);
                }
                node = next_node(e, node);
            }
        }
        0x8d => {
            // Each weapon mod slot active in the source set here (`00411e20`'s mask test in place,
            // the byte read for each slot as the exe reads it).
            for mask in [1u8, 2, 4] {
                if e.mem.u8(extra.wrapping_add(EXTRA_PAYLOAD)) & mask != 0 {
                    e.call(0x0042_e380, &args![list, u32::from(mask)]);
                }
            }
        }
        0x92 => {
            let existing = get_or_add(e, list, 0x92, 0x14, |e, block| {
                typed_call!(
                    cell_refs::FAMILY,
                    0x0041_1e40,
                    cell_refs::extra_list::type_92_new(e, block)
                )
            });
            e.vcall(existing, 0x0c, &args![extra]);
        }
        _ => {
            e.call(LOG_MESSAGE, &args![NO_COPY_MESSAGE, u32::from(kind)]);
        }
    }
}

/// Whether `list` lacks an extra data of `extra`'s type or holds one whose `Compare` (slot 4) says
/// it differs.
fn differs_from_extra_in(e: &mut Engine, list: u32, extra: u32) -> bool {
    let kind = kind_of(e, extra);
    match first_of_kind(e, list, kind) {
        0 => true,
        found => e.vcall(found, 4, &args![extra]).bool(),
    }
}

/// `CopyListForReference`'s general arm: the extra data copied (the call returning to `0x412523`)
/// and, when it is moved, deleted from `source` and the walk started again from its head. Answers
/// whether the walk goes on to the next.
fn copy_for_reference(
    e: &mut Engine,
    this: u32,
    source: u32,
    current: &mut u32,
    moved: bool,
) -> bool {
    add_extra_copy_from(e, this, *current, 0x0041_2523, u32::from(moved));
    if moved {
        list_remove(e, source, *current, true);
        *current = head_of(e, source);
    }
    !moved
}

// Typed from 004103d0
/// `ExtraDataList::AddExtraCopy` (Xbox PDB, `thiscall(list, extra)`): a copy of `extra` put in the
/// list through the setter of its type (the word, float, byte, handle's word or payload address at
/// +0xc), a new payload object, or the arm of its type; nothing for a null extra data or one
/// [`is_copyable_extra`] refuses; a type with no copy reported to the log.
pub fn add_extra_copy(e: &mut Engine, this: u32, extra: u32) {
    add_extra_copy_from(e, this, extra, 0, 0);
}

// Typed from 00411ec0
/// `ExtraDataList::CopyList` (Xbox PDB, `thiscall(list, source)`): the list emptied of its copyable
/// extra data (deleted), then, under the lock, a copy of each of `source`'s added.
pub fn copy_list(e: &mut Engine, this: u32, source: u32) {
    typed_call!(
        FAMILY,
        0x0041_1fd0,
        remove_all_copyable_extra(e, this, true)
    );
    take_lock(e, NAME_COPY_LIST);
    if source != 0 {
        let mut current = head_of(e, source);
        while current != 0 {
            typed_call!(FAMILY, 0x0041_03d0, add_extra_copy(e, this, current));
            current = next_of(e, current);
        }
    }
    give_lock(e);
}

// Typed from 00411f20
/// `ExtraDataList::IsCopyableExtra` (Xbox PDB, `thiscall(list, extra)`): false for the types a copy
/// leaves out (the jump table's), true for the rest and for a null extra data.
pub fn is_copyable_extra(e: &mut Engine, _this: u32, extra: u32) -> bool {
    extra == 0
        || !matches!(
            kind_of(e, extra),
            0x01 | 0x02
                | 0x04
                | 0x0c
                | 0x0f
                | 0x10
                | 0x11
                | 0x15
                | 0x29
                | 0x34
                | 0x40
                | 0x4b
                | 0x52
                | 0x54
                | 0x55
                | 0x58
        )
}

// Typed from 00411fd0
/// `ExtraDataList::RemoveAllCopyableExtra` (Xbox PDB, `thiscall(list, destroy)`): under the lock,
/// the caches forgotten and every copyable extra data unlinked, deleted when `destroy`; the bitmap
/// is left as it was.
pub fn remove_all_copyable_extra(e: &mut Engine, this: u32, destroy: bool) {
    take_lock(e, NAME_REMOVE_ALL_COPYABLE);
    let mut next = head_of(e, this);
    let mut kept = 0;
    forget_cached_all(e, this);
    while next != 0 {
        let current = next;
        next = next_of(e, current);
        if !typed_call!(FAMILY, 0x0041_1f20, is_copyable_extra(e, this, current)) {
            kept = current;
            continue;
        }
        if current == head_of(e, this) {
            e.mem.set_u32(this.wrapping_add(LIST_HEAD), next);
        }
        if kept != 0 {
            set_next_of(e, kept, next);
        }
        if destroy {
            delete_extra_data(e, current);
        }
    }
    give_lock(e);
}

// Typed from 004120b0
/// `ExtraDataList::RemoveNonPersistentCellData` (Xbox PDB): under the lock, the caches forgotten and
/// every extra data deleted but type 4 and, when the thread's flag says so ([`thread_flag_4`], in
/// place), types 1 and 2; the bitmap is left as it was.
pub fn remove_non_persistent_cell_data(e: &mut Engine, this: u32) {
    take_lock(e, NAME_REMOVE_NON_PERSISTENT);
    forget_cached_all(e, this);
    let mut kept = 0;
    let mut next = head_of(e, this);
    while next != 0 {
        let current = next;
        next = next_of(e, current);
        let keep = match kind_of(e, current) {
            1 | 2 => thread_flag_4(e, 0),
            4 => true,
            _ => false,
        };
        if keep {
            kept = current;
            continue;
        }
        if current == head_of(e, this) {
            e.mem.set_u32(this.wrapping_add(LIST_HEAD), next);
        }
        if kept != 0 {
            set_next_of(e, kept, next);
        }
        delete_extra_data(e, current);
    }
    give_lock(e);
}

// Typed from 004121b0
/// Bit 2 of the thread's word at +0x294 of the TLS block (`thiscall`; `this`, the game loader's
/// global the exe hands it, is not read).
pub fn thread_flag_4(e: &mut Engine, _this: u32) -> bool {
    let tls = e.tls();
    e.mem.u32(tls.wrapping_add(THREAD_FLAGS)) & 4 != 0
}

// Typed from 004121e0
/// `ExtraDataList::CopyListForContainer` (Xbox PDB, `thiscall(list, source, copy_only)`): under the
/// lock, `source`'s extra data that go with an inventory item copied (type 0xd and the jump table's
/// others); unless `copy_only` each copied one also leaves `source` (0xd unlinked, the others but
/// 0x30 deleted) and the walk starts again from its head.
pub fn copy_list_for_container(e: &mut Engine, this: u32, source: u32, copy_only: bool) {
    take_lock(e, NAME_COPY_LIST_FOR_CONTAINER);
    let mut current = head_of(e, source);
    while current != 0 {
        let mut advance = true;
        match kind_of(e, current) {
            0x0d => {
                typed_call!(FAMILY, 0x0041_03d0, add_extra_copy(e, this, current));
                if !copy_only {
                    list_remove(e, source, current, false);
                    current = head_of(e, source);
                    advance = false;
                }
            }
            0x16 | 0x17 | 0x1c | 0x21 | 0x22 | 0x23 | 0x25 | 0x26 | 0x27 | 0x28 | 0x2f | 0x30
            | 0x3f | 0x47 | 0x4a | 0x8d => {
                // The call returning to `0x412250`; the type read again after it, as the exe does.
                add_extra_copy_from(e, this, current, 0x0041_2250, u32::from(copy_only));
                if kind_of(e, current) != 0x30 && !copy_only {
                    list_remove(e, source, current, true);
                    current = head_of(e, source);
                    advance = false;
                }
            }
            _ => {}
        }
        if current != 0 && advance {
            current = next_of(e, current);
        }
    }
    give_lock(e);
}

// Typed from 00412380
/// `ExtraDataList::DuplicateExtraListForContainer` (Xbox PDB, `thiscall(list, source)`): under the
/// lock, a copy of each of `source`'s extra data of the inventory types (with 0x24 too); `source`
/// keeps them.
pub fn duplicate_for_container(e: &mut Engine, this: u32, source: u32) {
    take_lock(e, NAME_DUPLICATE_FOR_CONTAINER);
    let mut current = head_of(e, source);
    while current != 0 {
        if matches!(
            kind_of(e, current),
            0x0d | 0x16
                | 0x17
                | 0x1c
                | 0x21
                | 0x22
                | 0x23
                | 0x24
                | 0x25
                | 0x26
                | 0x27
                | 0x28
                | 0x2f
                | 0x30
                | 0x3f
                | 0x47
                | 0x4a
                | 0x8d
        ) {
            typed_call!(FAMILY, 0x0041_03d0, add_extra_copy(e, this, current));
        }
        current = next_of(e, current);
    }
    give_lock(e);
}

// Typed from 00412490
/// `ExtraDataList::CopyListForReference` (Xbox PDB, `thiscall(list, source, moved)`): under the
/// lock, the extra data a reference keeps when it is split copied (with `moved` deleted from
/// `source`, the walk starting again from its head); the script (0xd) moved, or without `moved` a
/// new `ExtraScript` made from `source`'s script and given its result as its locals. JIP's hotkey
/// fix (`0x412675`) copies type 0x4a too while its fixes are on.
pub fn copy_list_for_reference(e: &mut Engine, this: u32, source: u32, moved: bool) {
    take_lock(e, NAME_COPY_LIST_FOR_REFERENCE);
    let mut current = head_of(e, source);
    while current != 0 {
        let mut advance = true;
        match kind_of(e, current) {
            0x0d if !moved => {
                let script = typed_call!(FAMILY, 0x0041_8800, access::script(e, source));
                let block = e.call(super::OPERATOR_NEW, &args![0x14u32]).u32();
                let created = if block == 0 {
                    0
                } else {
                    e.call(EXTRA_SCRIPT_INIT, &args![block, script]).u32()
                };
                super::list_add(e, this, created);
                let script = typed_call!(FAMILY, 0x0041_8800, access::script(e, source));
                let compiled = e.call(SCRIPT_GET_RESULT, &args![script]).u32();
                typed_call!(
                    FAMILY,
                    0x0041_9f80,
                    access::set_script_locals(e, this, compiled)
                );
            }
            0x0d => {
                typed_call!(FAMILY, 0x0041_03d0, add_extra_copy(e, this, current));
                list_remove(e, source, current, false);
                current = head_of(e, source);
                advance = false;
            }
            0x16 | 0x1c | 0x21 | 0x22 | 0x23 | 0x25 | 0x26 | 0x27 | 0x28 | 0x2f | 0x30 | 0x3f
            | 0x8d => {
                advance = copy_for_reference(e, this, source, &mut current, moved);
            }
            // JIP LN NVSE's hotkey fix (`SafeWrite8(0x412675, 1)`, the byte table's entry of type
            // 0x4a): a hotkey is copied (and moved) with the list.
            0x4a if e.jip_patches() & engine_core::extend::jip_patches::FIXES != 0
                && e.site(0x0041_2675, &[]).is_some() =>
            {
                advance = copy_for_reference(e, this, source, &mut current, moved);
            }
            _ => {}
        }
        if current != 0 && advance {
            current = next_of(e, current);
        }
    }
    give_lock(e);
}

// Typed from 004126c0
/// `ExtraDataList::CompareListForContainer` (Xbox PDB, `thiscall(list, other, allow, skip_16)`):
/// whether `other` differs, under the lock. Without `allow` a script (0xd) or an owner (0x21; not
/// with JIP's invisible stack fix, `0x41271E`) is a difference at once; type 0x16 is passed over
/// with `skip_16`, 0x24 and 0x4a always; any other needs a `Compare`-equal one of its type here.
/// Each extra data's type is read once (the exe reads it again with nothing in between).
pub fn compare_list_for_container(
    e: &mut Engine,
    this: u32,
    other: u32,
    allow: bool,
    skip_16: bool,
) -> bool {
    take_lock(e, NAME_COMPARE_LIST_FOR_CONTAINER);
    let mut differs = false;
    let mut current = head_of(e, other);
    while current != 0 {
        let kind = kind_of(e, current);
        if !allow
            && (kind == 0x0d
                || kind == 0x21
                    && !(e.jip_patches() & engine_core::extend::jip_patches::FIXES != 0
                        && e.site(0x0041_271e, &[]).is_some()))
        {
            differs = true;
            break;
        }
        if !(kind == 0x16 && skip_16) && kind != 0x24 && kind != 0x4a {
            let found = first_of_kind(e, this, kind);
            if found == 0 || e.vcall(found, 4, &args![current]).bool() {
                differs = true;
                break;
            }
        }
        current = next_of(e, current);
    }
    give_lock(e);
    differs
}

// Typed from 004127e0
/// `ExtraDataList::CompareList` (Xbox PDB, `thiscall(list, other)`): whether the two lists differ in
/// their copyable extra data, under the lock: each of `other`'s needs a `Compare`-equal one here,
/// then each of this list's one there; with different lengths each of this list's needs its type
/// there.
pub fn compare_list(e: &mut Engine, this: u32, other: u32) -> bool {
    take_lock(e, NAME_COMPARE_LIST);
    let mut differs = false;
    for (walked, against) in [(other, this), (this, other)] {
        let mut current = head_of(e, walked);
        while current != 0 {
            if typed_call!(FAMILY, 0x0041_1f20, is_copyable_extra(e, this, current))
                && differs_from_extra_in(e, against, current)
            {
                differs = true;
                break;
            }
            current = next_of(e, current);
        }
        if differs {
            break;
        }
    }
    let own = typed_call!(
        cell_refs::FAMILY,
        0x0040_fe20,
        cell_refs::extra_list::items_in_list(e, this)
    );
    let theirs = typed_call!(
        cell_refs::FAMILY,
        0x0040_fe20,
        cell_refs::extra_list::items_in_list(e, other)
    );
    if own != theirs {
        let mut current = head_of(e, this);
        while current != 0 {
            if typed_call!(FAMILY, 0x0041_1f20, is_copyable_extra(e, this, current))
                && first_of_kind(e, other, kind_of(e, current)) == 0
            {
                differs = true;
                break;
            }
            current = next_of(e, current);
        }
    }
    give_lock(e);
    differs
}
