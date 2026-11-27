//! `ExtraDataList::Load` (`004144a0`, Xbox PDB): one chunk of a reference's or a cell's extra data,
//! read from the master file the reader stands at and applied to the list, the counterpart of the
//! save writer ([`super::save`]). The chunk is named by its four letters (`b"XCWT"` is the word
//! `0x54574358`); a chunk the loader does not know is left alone. The chunk's size and the file's
//! byte-swap flag (`00401660`, `00401680`: the words at +0x25c and +0x299 of the `TESFile`) are read
//! in place; the file's other readers, the setters and constructors of other units, `operator new`
//! and `operator delete` are called by address, the family's setters, getters and records, the
//! lookup, `AddExtra`, `RemoveExtra` and `memset` as typed calls.

use super::{
    access, cell_data, first_of_kind, followers, links, list_add, list_remove, new_object,
    packages, records, rooms, setters, word_at, zero_bytes, EXTRA_PAYLOAD, FAMILY, OPERATOR_DELETE,
    OPERATOR_NEW, SIMPLE_LIST_NEW,
};
use crate::crt::{low_byte, low_half, signed};
use crate::{cell_refs, forms, pathing};
use engine_core::x87::x87_quiet_bits;
use engine_core::{args, typed_call, Engine};

/// The file's readers (`TESFile`, Xbox PDB): the current chunk's name, its data as one word, its
/// data into a buffer, the next chunk, the file's name.
const GET_TES_CHUNK: u32 = 0x0047_26b0;
const GET_CHUNK_WORD: u32 = 0x0047_27f0;
const GET_CHUNK_DATA: u32 = 0x0047_2890;
const SKIP_CHUNK: u32 = 0x0047_26f0;
const FILE_NAME: u32 = 0x0089_1170;
/// The `TESFile`'s current chunk size (+0x25c) and byte-swap flag (+0x299).
const FILE_CHUNK_SIZE: u32 = 0x25c;
const FILE_SWAPS: u32 = 0x299;
/// The byte swaps: `Swap32(ptr, 0)`, a word, the word at +4, two words.
const SWAP_DWORD: u32 = 0x0040_1080;
const SWAP_WORD_AT: u32 = 0x0050_3210;
const SWAP_WORD_AT_4: u32 = 0x0060_ce80;
const SWAP_TWO_WORDS: u32 = 0x0046_2230;
/// The log (overridable: by address).
const LOG_MESSAGE: u32 = 0x005b_5e40;
const MESSAGE_BAD_RAGDOLL: u32 = 0x0101_45ac;
const MESSAGE_BAD_REGION_LIST: u32 = 0x0101_45d8;
/// `TESDataHandler` and the distant data's default third float.
const DATA_HANDLER: u32 = 0x011c_3f2c;
const LOD_DEFAULT: u32 = 0x0101_45a8;
/// `BSSimpleList::AddHead(&item)`.
const LIST_ADD_HEAD: u32 = 0x005a_e3d0;
/// Other units' constructors.
const ACTIVATE_REF_INIT: u32 = 0x0043_38b0;
const EXTRA_LOCK_INIT: u32 = 0x0043_0c70;
const EXTRA_RAGDOLL_DATA_INIT: u32 = 0x0043_2cb0;
const RAGDOLL_DATA_INIT: u32 = 0x004d_9330;
const DOOR_TELEPORT_DATA_INIT: u32 = 0x0043_a160;
const MAP_MARKER_DATA_INIT: u32 = 0x0043_8bb0;
const AUDIO_MARKER_DATA_INIT: u32 = 0x0058_9450;
const AUDIO_BUOY_MARKER_DATA_INIT: u32 = 0x0068_0890;
const REGION_LIST_INIT: u32 = 0x004f_6320;
const DECAL_DATA_INIT: u32 = 0x0055_a400;
const MULTIBOUND_MARKER_DATA_INIT: u32 = 0x0043_8f50;
const NAVMESH_PORTAL_INIT: u32 = 0x0069_2870;
const OCCLUSION_PLANE_INIT: u32 = 0x00c3_35d0;
const PATROL_REF_DATA_INIT: u32 = 0x0067_c690;
const ALLOCATE_ALIGNED: u32 = 0x00aa_13e0;
/// Other units' loaders and the objects' own calls.
const DOOR_TELEPORT_DATA_LOAD: u32 = 0x0043_a4e0;
const MAP_MARKER_DATA_LOAD: u32 = 0x0043_8ca0;
const AUDIO_MARKER_DATA_LOAD: u32 = 0x0058_9600;
const AUDIO_BUOY_MARKER_DATA_LOAD: u32 = 0x0058_9890;
const RAGDOLL_DATA_LOAD: u32 = 0x004d_94a0;
const MULTIBOUND_MARKER_DATA_LOAD: u32 = 0x0043_8fc0;
const NAVMESH_PORTAL_LOAD: u32 = 0x0069_dfd0;
const NAVMESH_PORTAL_EXTRA_DESTROY: u32 = 0x0043_2f30;
const PACKAGE_EVENT_ACTION_LOAD: u32 = 0x0067_dd20;
const REFR_LOCK_SET_LOCKED: u32 = 0x0043_0a90;
const REGION_LIST_FIND: u32 = 0x004f_66c0;
const REGION_LIST_ADD: u32 = 0x004f_6600;
const PRIMITIVE_CREATE: u32 = 0x004a_4e90;
/// The list's getters and setters of the other part files (by address until they are typed).
const GET_AMMO: u32 = 0x0042_eb40;
const SET_AMMO: u32 = 0x0042_eb60;
const SET_NAVMESH_PORTAL: u32 = 0x0042_e2c0;
const SET_IGNORED_BY_SANDBOX: u32 = 0x0042_f200;

/// A chunk's four letters as the word the file holds.
const fn tag(name: &[u8; 4]) -> u32 {
    u32::from_le_bytes(*name)
}

/// The chunks the loader knows.
const T_XACT: u32 = tag(b"XACT");
const T_XOWN: u32 = tag(b"XOWN");
const T_XGLB: u32 = tag(b"XGLB");
const T_XRNK: u32 = tag(b"XRNK");
const T_XPSN: u32 = tag(b"XPSN");
const T_XCMO: u32 = tag(b"XCMO");
const T_XCAS: u32 = tag(b"XCAS");
const T_XCCM: u32 = tag(b"XCCM");
const T_XCIM: u32 = tag(b"XCIM");
const T_XCWT: u32 = tag(b"XCWT");
const T_XRTM: u32 = tag(b"XRTM");
const T_XLKR: u32 = tag(b"XLKR");
const T_XMRC: u32 = tag(b"XMRC");
const T_XTRG: u32 = tag(b"XTRG");
const T_XEZN: u32 = tag(b"XEZN");
const T_XEMI: u32 = tag(b"XEMI");
const T_XMBR: u32 = tag(b"XMBR");
const T_XLCM: u32 = tag(b"XLCM");
const T_XHLP: u32 = tag(b"XHLP");
const T_XTIM: u32 = tag(b"XTIM");
const T_XCHG: u32 = tag(b"XCHG");
const T_XRDS: u32 = tag(b"XRDS");
const T_XRAD: u32 = tag(b"XRAD");
const T_XHLT: u32 = tag(b"XHLT");
const T_XUSE: u32 = tag(b"XUSE");
const T_XCNT: u32 = tag(b"XCNT");
const T_XAMC: u32 = tag(b"XAMC");
const T_XAMT: u32 = tag(b"XAMT");
const T_XLOC: u32 = tag(b"XLOC");
const T_XRDO: u32 = tag(b"XRDO");
const T_XTEL: u32 = tag(b"XTEL");
const T_XMRK: u32 = tag(b"XMRK");
const T_MMRK: u32 = tag(b"MMRK");
const T_AMRK: u32 = tag(b"AMRK");
const T_XSED: u32 = tag(b"XSED");
const T_XCLR: u32 = tag(b"XCLR");
const T_XPSL: u32 = tag(b"XPSL");
const T_XRGD: u32 = tag(b"XRGD");
const T_XRGB: u32 = tag(b"XRGB");
const T_XLOD: u32 = tag(b"XLOD");
const T_XESP: u32 = tag(b"XESP");
const T_XAPD: u32 = tag(b"XAPD");
const T_XATO: u32 = tag(b"XATO");
const T_XAPR: u32 = tag(b"XAPR");
const T_XACR: u32 = tag(b"XACR");
const T_XDCR: u32 = tag(b"XDCR");
const T_XMBO: u32 = tag(b"XMBO");
const T_XPCI: u32 = tag(b"XPCI");
const T_XNVP: u32 = tag(b"XNVP");
const T_XNDP: u32 = tag(b"XNDP");
const T_XPWR: u32 = tag(b"XPWR");
const T_XLTW: u32 = tag(b"XLTW");
const T_XPRM: u32 = tag(b"XPRM");
const T_XOCP: u32 = tag(b"XOCP");
const T_XPTL: u32 = tag(b"XPTL");
const T_XPRD: u32 = tag(b"XPRD");
const T_XORD: u32 = tag(b"XORD");
const T_XPOD: u32 = tag(b"XPOD");
const T_XRMR: u32 = tag(b"XRMR");
const T_XTRI: u32 = tag(b"XTRI");
const T_XPPA: u32 = tag(b"XPPA");
const T_XIBS: u32 = tag(b"XIBS");
const T_XSRF: u32 = tag(b"XSRF");
const T_XSRD: u32 = tag(b"XSRD");
const T_XCMT: u32 = tag(b"XCMT");
const T_XCET: u32 = tag(b"XCET");

/// The data of the current chunk as one word (`TESFile::GetChunkData(&word)`, `004727f0`), read
/// into a local the exe zeroes first (a chunk shorter than a word leaves the rest 0).
fn file_word(e: &mut Engine, file: u32) -> u32 {
    e.with_stack(4, |e, local| {
        e.mem.set_u32(local.addr(), 0);
        e.call(GET_CHUNK_WORD, &args![file, local]);
        e.mem.u32(local.addr())
    })
}

/// `size` bytes of the current chunk's data into `buffer` (`TESFile::GetChunkData`, `00472890`).
fn file_bytes(e: &mut Engine, file: u32, buffer: u32, size: u32) {
    e.call(GET_CHUNK_DATA, &args![file, buffer, size]);
}

/// The current chunk's size, in place (`00401660`'s `MOV EAX,[ECX+0x25c]`).
#[inline]
fn file_chunk_size(e: &Engine, file: u32) -> u32 {
    e.mem.u32(file.wrapping_add(FILE_CHUNK_SIZE))
}

/// Whether the file's data is byte-swapped, in place (`00401680`'s `MOV AL,[ECX+0x299]`).
#[inline]
fn file_swaps(e: &Engine, file: u32) -> bool {
    e.mem.u8(file.wrapping_add(FILE_SWAPS)) != 0
}

/// One word through the x87's load and store (a signalling NaN comes out quiet), as a `float`.
#[inline]
fn moved_float(bits: u32) -> f32 {
    f32::from_bits(x87_quiet_bits(bits))
}

/// The extra data of `kind` in the list, made by `build` (`size` bytes) and added when it has none.
fn extra_or_new(
    e: &mut Engine,
    list: u32,
    kind: u8,
    size: u32,
    build: impl FnOnce(&mut Engine, u32) -> u32,
) -> u32 {
    let mut extra = first_of_kind(e, list, kind);
    if extra == 0 {
        extra = new_object(e, size, build);
        list_add(e, list, extra);
    }
    extra
}

/// The activate-parent extra data (type 0x53, the `0x20`-byte constructor `004338b0`), added when
/// the list has none.
fn activate_ref_of(e: &mut Engine, list: u32) -> u32 {
    extra_or_new(e, list, 0x53, 0x20, |e, block| {
        e.call(ACTIVATE_REF_INIT, &args![block]).u32()
    })
}

/// The type 0x92 extra data, made (`00411e40`, `0x14` bytes) and added when the list has none:
/// then `AddExtra`'s answer.
fn special_render_of(e: &mut Engine, list: u32) -> u32 {
    let existing = first_of_kind(e, list, 0x92);
    if existing != 0 {
        return existing;
    }
    let made = new_object(e, 0x14, |e, block| {
        typed_call!(
            cell_refs::FAMILY,
            0x0041_1e40,
            cell_refs::extra_list::type_92_new(e, block)
        )
    });
    typed_call!(
        cell_refs::FAMILY,
        0x0040_ff60,
        cell_refs::extra_list::add_extra(e, list, made)
    )
}

/// Whether the save buffer's form (`+0x20`, `007af430` in place) is a form of type 0x23
/// (`00401170`'s byte at +4, in place).
fn buffer_form_is_0x23(e: &Engine, buffer: u32) -> bool {
    let form = word_at(e, buffer, 0x20);
    form != 0 && e.mem.u8(form.wrapping_add(4)) == 0x23
}

/// `BSSimpleList::AddHead` of one item, through a local.
fn add_head(e: &mut Engine, list: u32, item: u32) {
    e.with_stack(4, |e, local| {
        e.mem.set_u32(local.addr(), item);
        e.call(LIST_ADD_HEAD, &args![list, local]);
    });
}

/// One of the markers kept behind a getter and a setter of the list (the teleport data, the map
/// marker, the audio markers): the getter's answer, or a new object given to the setter.
fn member_or_new(
    e: &mut Engine,
    list: u32,
    get: fn(&mut Engine, u32) -> u32,
    size: u32,
    construct: u32,
    set: fn(&mut Engine, u32, u32),
) -> u32 {
    let mut member = get(e, list);
    if member == 0 {
        member = new_object(e, size, |e, block| e.call(construct, &args![block]).u32());
        set(e, list, member);
    }
    member
}

// Typed from 004144a0
/// `ExtraDataList::Load` (Xbox PDB): the chunk the file stands at (`TESFile::GetTESChunk`,
/// `004726b0`) applied to this list; `buffer` is the `BGSSaveFormBuffer`, whose form (+0x20) keeps
/// the `XPWR` and `XLTW` chunks out for a form of type 0x23. The compiler's exception-unwinding frame
/// is not translated.
pub fn load_chunk(e: &mut Engine, this: u32, file: u32, buffer: u32) {
    let chunk = e.call(GET_TES_CHUNK, &args![file]).u32();
    match chunk {
        // One word for a setter.
        T_XACT => {
            let word = file_word(e, file);
            typed_call!(
                FAMILY,
                0x0041_b3d0,
                setters::set_action_flags(e, this, word)
            );
        }
        T_XOWN => {
            let word = file_word(e, file);
            typed_call!(FAMILY, 0x0041_9700, access::set_ownership(e, this, word));
        }
        T_XGLB => {
            let word = file_word(e, file);
            typed_call!(FAMILY, 0x0041_97d0, access::set_global(e, this, word));
        }
        T_XRNK => {
            let word = file_word(e, file);
            typed_call!(FAMILY, 0x0041_98a0, access::set_rank(e, this, signed(word)));
        }
        T_XPSN => {
            let word = file_word(e, file);
            typed_call!(FAMILY, 0x0041_9d10, access::set_poison(e, this, word));
        }
        T_XCMO => {
            let word = file_word(e, file);
            typed_call!(
                FAMILY,
                0x0041_bd10,
                cell_data::set_music_type(e, this, word)
            );
        }
        T_XCAS => {
            let word = file_word(e, file);
            typed_call!(
                FAMILY,
                0x0041_c090,
                cell_data::set_acoustic_space(e, this, word)
            );
        }
        T_XCCM => {
            let word = file_word(e, file);
            typed_call!(FAMILY, 0x0041_c190, cell_data::set_climate(e, this, word));
        }
        T_XCIM => {
            let word = file_word(e, file);
            typed_call!(
                FAMILY,
                0x0041_c290,
                cell_data::set_image_space(e, this, word)
            );
        }
        T_XCWT => {
            let water = file_word(e, file);
            typed_call!(FAMILY, 0x0041_e160, links::set_water_type(e, this, water));
        }
        T_XRTM => {
            let marker = file_word(e, file);
            typed_call!(
                FAMILY,
                0x0041_e250,
                links::set_teleport_marker(e, this, marker)
            );
        }
        T_XLKR => {
            let reference = file_word(e, file);
            typed_call!(
                FAMILY,
                0x0041_e440,
                links::set_linked_ref(e, this, reference)
            );
        }
        T_XMRC => {
            let word = file_word(e, file);
            typed_call!(
                FAMILY,
                0x0042_1430,
                followers::set_merchant_container(e, this, word)
            );
        }
        T_XTRG => {
            let word = file_word(e, file);
            typed_call!(FAMILY, 0x0042_1b70, followers::set_x_target(e, this, word));
        }
        T_XEZN => {
            let word = file_word(e, file);
            typed_call!(
                FAMILY,
                0x0042_1c60,
                followers::set_encounter_zone(e, this, word)
            );
        }
        T_XEMI => {
            let word = file_word(e, file);
            typed_call!(
                FAMILY,
                0x0042_1d50,
                followers::set_emittance_source(e, this, word)
            );
        }
        T_XMBR => {
            let word = file_word(e, file);
            typed_call!(
                FAMILY,
                0x0042_1e40,
                followers::set_multibound_ref(e, this, word)
            );
        }
        T_XLCM => {
            let word = file_word(e, file);
            typed_call!(
                FAMILY,
                0x0042_1540,
                followers::set_lev_crea_modifier(e, this, word)
            );
        }
        // One float for a setter.
        T_XHLP => {
            let value = moved_float(file_word(e, file));
            typed_call!(
                FAMILY,
                0x0041_b580,
                setters::set_health_perc(e, this, value)
            );
        }
        T_XTIM => {
            let value = moved_float(file_word(e, file));
            typed_call!(FAMILY, 0x0041_9bb0, access::set_time_left(e, this, value));
        }
        T_XCHG => {
            let value = moved_float(file_word(e, file));
            typed_call!(FAMILY, 0x0041_9c60, access::set_charge(e, this, value));
        }
        T_XRDS => {
            let value = moved_float(file_word(e, file));
            typed_call!(FAMILY, 0x0042_2220, followers::set_radius(e, this, value));
        }
        T_XRAD => {
            let value = moved_float(file_word(e, file));
            typed_call!(
                FAMILY,
                0x0042_2350,
                followers::set_radiation(e, this, value)
            );
        }
        // EXTRA_HEALTH: an integer in the file, a float for the setter.
        T_XHLT => {
            let health = signed(file_word(e, file));
            typed_call!(
                FAMILY,
                0x0041_9970,
                access::set_health(e, this, health as f32)
            );
        }
        T_XUSE => {
            let uses = low_byte(file_word(e, file));
            typed_call!(FAMILY, 0x0041_9a20, access::set_uses(e, this, uses));
        }
        T_XCNT => {
            let count = low_half(file_word(e, file));
            typed_call!(FAMILY, 0x0041_9ad0, access::set_count(e, this, count));
        }
        // EXTRA_AMMO's count, for the ammo the list already has.
        T_XAMC => {
            let count = file_word(e, file);
            if e.call(GET_AMMO, &args![this]).u32() != 0 {
                let ammo = e.call(GET_AMMO, &args![this]).u32();
                let form = word_at(e, ammo, EXTRA_PAYLOAD);
                e.call(SET_AMMO, &args![this, form, count]);
            }
        }
        // EXTRA_AMMO's form id, with a count of 999.
        T_XAMT => {
            let ammo_id = file_word(e, file);
            e.call(SET_AMMO, &args![this, ammo_id, 999u32]);
        }
        T_XLOC => load_lock(e, this, file),
        // EXTRA_RADIO_DATA: a RADIO_DATA (0x10 bytes) for the setter.
        T_XRDO => e.with_stack(0x10, |e, radio| {
            let radio = radio.addr();
            typed_call!(FAMILY, 0x0041_6b40, records::radio_data_new(e, radio));
            file_bytes(e, file, radio, 0x10);
            if file_swaps(e, file) {
                typed_call!(FAMILY, 0x0041_4220, records::swap_four_words(e, radio));
            }
            typed_call!(FAMILY, 0x0041_8310, access::set_radio_data(e, this, radio));
        }),
        T_XTEL => {
            let teleport = member_or_new(
                e,
                this,
                |e, list| typed_call!(FAMILY, 0x0041_8460, access::teleport(e, list)),
                0x20,
                DOOR_TELEPORT_DATA_INIT,
                |e, list, member| {
                    typed_call!(FAMILY, 0x0041_9120, access::set_teleport(e, list, member));
                },
            );
            e.call(DOOR_TELEPORT_DATA_LOAD, &args![teleport, file]);
        }
        T_XMRK => {
            let marker = member_or_new(
                e,
                this,
                |e, list| typed_call!(FAMILY, 0x0041_8490, access::map_marker(e, list)),
                0x14,
                MAP_MARKER_DATA_INIT,
                |e, list, member| {
                    typed_call!(FAMILY, 0x0041_9250, access::set_map_marker(e, list, member));
                },
            );
            e.call(MAP_MARKER_DATA_LOAD, &args![marker, file]);
        }
        T_MMRK => {
            let marker = member_or_new(
                e,
                this,
                |e, list| typed_call!(FAMILY, 0x0041_84c0, access::audio_marker(e, list)),
                0x34,
                AUDIO_MARKER_DATA_INIT,
                |e, list, member| {
                    typed_call!(
                        FAMILY,
                        0x0041_9380,
                        access::set_audio_marker(e, list, member)
                    );
                },
            );
            e.call(AUDIO_MARKER_DATA_LOAD, &args![marker, file]);
        }
        T_AMRK => {
            let marker = member_or_new(
                e,
                this,
                |e, list| typed_call!(FAMILY, 0x0041_84f0, access::audio_buoy_marker(e, list)),
                0x08,
                AUDIO_BUOY_MARKER_DATA_INIT,
                |e, list, member| {
                    typed_call!(
                        FAMILY,
                        0x0041_94b0,
                        access::set_audio_buoy_marker(e, list, member)
                    );
                },
            );
            e.call(AUDIO_BUOY_MARKER_DATA_LOAD, &args![marker, file]);
        }
        // EXTRA_SEED: one byte, from a 4-byte chunk or a 1-byte one.
        T_XSED => {
            let seed = if file_chunk_size(e, file) == 4 {
                low_byte(file_word(e, file))
            } else {
                e.with_stack(1, |e, local| {
                    file_bytes(e, file, local.addr(), 1);
                    e.mem.u8(local.addr())
                })
            };
            typed_call!(FAMILY, 0x0041_ac30, setters::set_seed(e, this, seed));
        }
        T_XCLR => load_regions(e, this, file),
        // EXTRA_PACKAGESTARTLOC: a WORLD_LOCATION (0x14 bytes).
        T_XPSL => e.with_stack(0x14, |e, record| {
            typed_call!(
                pathing::FAMILY,
                0x006d_5320,
                pathing::location::world_location_new(e, record)
            );
            let at = record.addr();
            zero_bytes(e, at, 0x14);
            file_bytes(e, file, at, 0x14);
            if file_swaps(e, file) {
                typed_call!(FAMILY, 0x0041_4220, records::swap_four_words(e, at));
            }
            let location = e.mem.u32(at);
            let z_rotation = moved_float(e.mem.u32(at + 0x10));
            typed_call!(
                FAMILY,
                0x0041_ad00,
                setters::set_package_start_location(e, this, location, 0, at + 4, z_rotation)
            );
        }),
        // EXTRA_RAGDOLLDATA (both names).
        T_XRGD | T_XRGB => load_ragdoll(e, this, file),
        // EXTRA_DISTANTDATA: two zeros and the default third float (`00416870` in place), then the
        // chunk's three floats.
        T_XLOD => e.with_stack(12, |e, local| {
            let at = local.addr();
            let third = x87_quiet_bits(e.mem.u32(LOD_DEFAULT));
            e.mem.set_u32(at, 0);
            e.mem.set_u32(at + 4, 0);
            e.mem.set_u32(at + 8, third);
            file_bytes(e, file, at, 12);
            if file_swaps(e, file) {
                for index in 0..3 {
                    e.call(SWAP_DWORD, &args![at + index * 4, 0u32]);
                }
            }
            typed_call!(FAMILY, 0x0041_d950, packages::set_land_normal(e, this, at));
        }),
        // EXTRA_ENABLESTATEPARENT: the parent's form id and its flags byte.
        T_XESP => e.with_stack(8, |e, local| {
            let at = local.addr();
            zero_bytes(e, at, 8);
            file_bytes(e, file, at, 8);
            if file_swaps(e, file) {
                e.call(SWAP_WORD_AT, &args![at]);
            }
            let parent = e.mem.u32(at);
            typed_call!(
                FAMILY,
                0x0041_da40,
                packages::set_enable_state_parent(e, this, parent)
            );
            let flags = e.mem.u8(at + 4);
            typed_call!(FAMILY, 0x0041_dc70, links::set_parent_flags(e, this, flags));
        }),
        // EXTRA_ACTIVATE_REF's flags byte.
        T_XAPD => {
            let extra = activate_ref_of(e, this);
            if file_chunk_size(e, file) == 4 {
                let flags = low_byte(file_word(e, file));
                e.mem.set_u8(extra.wrapping_add(0x14), flags);
            } else {
                file_bytes(e, file, extra.wrapping_add(0x14), 1);
            }
        }
        // EXTRA_ACTIVATE_REF's text override.
        T_XATO => {
            activate_ref_of(e, this);
            let size = file_chunk_size(e, file);
            if signed(size) > 1 {
                let text = e.call(OPERATOR_NEW, &args![size]).u32();
                file_bytes(e, file, text, size);
                typed_call!(FAMILY, 0x0041_ece0, links::set_activate_text(e, this, text));
                e.call(OPERATOR_DELETE, &args![text]);
            }
        }
        // EXTRA_ACTIVATE_REF: one REF_ACTIVATE_DATA of the parent list.
        T_XAPR => {
            let extra = activate_ref_of(e, this);
            let data = new_object(e, 8, |e, block| {
                typed_call!(FAMILY, 0x0041_4010, records::empty_node_new(e, block))
            });
            file_bytes(e, file, data, 8);
            if file_swaps(e, file) {
                e.call(SWAP_TWO_WORDS, &args![data]);
            }
            add_head(e, extra.wrapping_add(EXTRA_PAYLOAD), data);
        }
        T_XACR => load_old_activate_ref(e, this, file),
        // EXTRA_DECAL_REFS: one REF_DECAL_DATA (0x1c bytes).
        T_XDCR => e.with_stack(0x1c, |e, decal| {
            let at = decal.addr();
            e.call(DECAL_DATA_INIT, &args![at]);
            zero_bytes(e, at, 0x1c);
            file_bytes(e, file, at, 0x1c);
            if file_swaps(e, file) {
                typed_call!(FAMILY, 0x0041_4030, records::swap_decal(e, at));
            }
            let reference = e.mem.u32(at);
            typed_call!(
                FAMILY,
                0x0041_f080,
                rooms::add_decal(e, this, reference, at + 4, at + 0x10)
            );
        }),
        // EXTRA_MULTIBOUND_DATA
        T_XMBO => {
            let bound = new_object(e, 0x0c, |e, block| {
                e.call(MULTIBOUND_MARKER_DATA_INIT, &args![block]).u32()
            });
            e.call(MULTIBOUND_MARKER_DATA_LOAD, &args![bound, file]);
            typed_call!(
                FAMILY,
                0x0042_1f30,
                followers::set_multibound_data(e, this, bound)
            );
        }
        T_XPCI => {
            e.call(SKIP_CHUNK, &args![file]);
        }
        // EXTRA_NAVMESH_PORTAL: the chunk read whole for the portal's own loader.
        T_XNVP => e.with_stack(8, |e, portal| {
            e.call(NAVMESH_PORTAL_INIT, &args![portal]);
            let size = file_chunk_size(e, file);
            let data = e.call(OPERATOR_NEW, &args![size]).u32();
            file_bytes(e, file, data, size);
            e.call(NAVMESH_PORTAL_LOAD, &args![portal, buffer, file, data]);
            e.call(OPERATOR_DELETE, &args![data]);
        }),
        T_XNDP => load_old_portal(e, this, file),
        // EXTRA_PORTAL_REF_DATA (`XPWR`) and EXTRA_LIT_WATER_REFS (`XLTW`), not for a form of type 0x23.
        T_XPWR => {
            if !buffer_form_is_0x23(e, buffer) {
                load_reflector(e, this, file);
            }
        }
        T_XLTW => {
            if !buffer_form_is_0x23(e, buffer) {
                let reference = file_word(e, file);
                typed_call!(
                    FAMILY,
                    0x0041_f940,
                    rooms::set_lit_water_ref(e, this, reference, true)
                );
            }
        }
        T_XPRM => load_primitive(e, this, file),
        T_XOCP => load_plane(e, this, file),
        // EXTRA_PORTAL: the record read and dropped.
        T_XPTL => e.with_stack(0x24, |e, record| {
            let at = record.addr();
            typed_call!(FAMILY, 0x0041_4470, records::plane_new(e, at));
            zero_bytes(e, at, 0x24);
            file_bytes(e, file, at, 0x24);
            if file_swaps(e, file) {
                typed_call!(FAMILY, 0x0041_40f0, records::swap_plane(e, at));
            }
        }),
        // EXTRA_PATROL_REF_DATA's time at the reference (`006815c0`, the local's own address, in
        // place).
        T_XPRD => e.with_stack(4, |e, local| {
            let at = local.addr();
            file_bytes(e, file, at, 4);
            if file_swaps(e, file) {
                e.call(SWAP_WORD_AT, &args![at]);
            }
            let patrol = new_object(e, 0x14, |e, block| {
                e.call(PATROL_REF_DATA_INIT, &args![block, at]).u32()
            });
            typed_call!(
                FAMILY,
                0x0041_fc10,
                rooms::set_patrol_ref_data(e, this, patrol)
            );
        }),
        // EXTRA_OCCLUSION_PLANE_REF_DATA: four form ids.
        T_XORD => {
            let data = new_object(e, 0x10, |e, block| {
                typed_call!(
                    cell_refs::FAMILY,
                    0x0041_1dc0,
                    cell_refs::extra_list::four_words_new(e, block)
                )
            });
            file_bytes(e, file, data, 0x10);
            if file_swaps(e, file) {
                typed_call!(FAMILY, 0x0041_4370, records::swap_plane_refs(e, data));
            }
            typed_call!(
                FAMILY,
                0x0041_fec0,
                rooms::set_occlusion_plane_ref_data(e, this, data)
            );
        }
        // EXTRA_PORTAL_REF_DATA: two form ids.
        T_XPOD => {
            let data = new_object(e, 8, |e, block| {
                typed_call!(FAMILY, 0x0041_43c0, records::portal_ref_new(e, block))
            });
            file_bytes(e, file, data, 8);
            if file_swaps(e, file) {
                e.call(SWAP_TWO_WORDS, &args![data]);
            }
            typed_call!(
                FAMILY,
                0x0042_0210,
                rooms::set_portal_ref_data(e, this, data)
            );
        }
        T_XRMR => load_rooms(e, this, file),
        // EXTRA_COLLISION_DATA: one word.
        T_XTRI => {
            let data = new_object(e, 4, |e, block| {
                typed_call!(
                    cell_refs::FAMILY,
                    0x0041_1e00,
                    cell_refs::extra_list::word_16_new(e, block)
                )
            });
            file_bytes(e, file, data, 4);
            if file_swaps(e, file) {
                e.call(SWAP_WORD_AT, &args![data]);
            }
            typed_call!(
                FAMILY,
                0x0042_0fd0,
                rooms::set_collision_data(e, this, data)
            );
        }
        // EXTRA_PATROL_REF_DATA's PackageEventAction.
        T_XPPA => {
            let patrol = typed_call!(FAMILY, 0x0041_fe90, rooms::patrol_ref_data(e, this));
            e.call(SKIP_CHUNK, &args![file]);
            e.call(
                PACKAGE_EVENT_ACTION_LOAD,
                &args![patrol.wrapping_add(4), file],
            );
        }
        T_XIBS => {
            e.call(SET_IGNORED_BY_SANDBOX, &args![this, 1u32]);
        }
        // Type 0x92: the word at +0xc, then the float at +0x10.
        T_XSRF => {
            let extra = special_render_of(e, this);
            let value = file_word(e, file);
            // `0041fd00` in place: the word stored at +0xc.
            e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), value);
        }
        T_XSRD => {
            let extra = special_render_of(e, this);
            let value = x87_quiet_bits(file_word(e, file));
            e.mem.set_u32(extra.wrapping_add(0x10), value);
        }
        // Two chunks read into a byte and dropped.
        T_XCMT | T_XCET => e.with_stack(1, |e, local| {
            file_bytes(e, file, local.addr(), 1);
        }),
        _ => {}
    }
}

/// `XLOC`: a REFR_LOCK (0x14 bytes) read into the list's ExtraLock (made with its lock when the
/// list has none), then locked.
fn load_lock(e: &mut Engine, this: u32, file: u32) {
    let extra = first_of_kind(e, this, 0x2a);
    let lock_data = if extra == 0 {
        let made = new_object(e, 0x14, |e, block| {
            typed_call!(
                cell_refs::FAMILY,
                0x0041_1b00,
                cell_refs::extra_list::lock_data_new(e, block)
            )
        });
        let created = new_object(e, 0x10, |e, block| {
            e.call(EXTRA_LOCK_INIT, &args![block, made]).u32()
        });
        list_add(e, this, created);
        made
    } else {
        word_at(e, extra, EXTRA_PAYLOAD)
    };
    file_bytes(e, file, lock_data, 0x14);
    if file_swaps(e, file) {
        e.call(SWAP_WORD_AT_4, &args![lock_data]);
    }
    e.call(REFR_LOCK_SET_LOCKED, &args![lock_data, 1u32]);
}

/// `XCLR`: the region form ids, each looked up in the data handler's region list (`004169d0`) and
/// gathered in a new TESRegionList; a size that is not whole words is logged and dropped.
fn load_regions(e: &mut Engine, this: u32, file: u32) {
    let size = file_chunk_size(e, file);
    let count = size >> 2;
    if size % 4 != 0 {
        let name = e.call(FILE_NAME, &args![file]).u32();
        e.call(LOG_MESSAGE, &args![MESSAGE_BAD_REGION_LIST, name]);
        return;
    }
    let regions = new_object(e, 0x10, |e, block| {
        e.call(REGION_LIST_INIT, &args![block, 0u32]).u32()
    });
    let ids = e.call(OPERATOR_NEW, &args![count << 2]).u32();
    file_bytes(e, file, ids, size);
    for index in 0..count {
        let slot = ids.wrapping_add(index * 4);
        if file_swaps(e, file) {
            e.call(SWAP_DWORD, &args![slot, 0u32]);
        }
        typed_call!(
            forms::FAMILY,
            0x0048_5d50,
            forms::tesform::add_compile_index(e, slot, file)
        );
        let id = e.mem.u32(slot);
        let handler = e.mem.u32(DATA_HANDLER);
        let known = typed_call!(FAMILY, 0x0041_69d0, records::word_1d8(e, handler));
        let region = e.call(REGION_LIST_FIND, &args![known, id]).u32();
        if region != 0 {
            e.call(REGION_LIST_ADD, &args![regions, region]);
        }
    }
    typed_call!(
        FAMILY,
        0x0041_bbd0,
        cell_data::set_region_list(e, this, regions)
    );
    e.call(OPERATOR_DELETE, &args![ids]);
}

/// `XRGD` / `XRGB`: the ragdoll data's own loader on the list's ExtraRagDollData (made with its
/// data when the list has none); a load that fails is logged and the extra data removed.
fn load_ragdoll(e: &mut Engine, this: u32, file: u32) {
    let mut extra = first_of_kind(e, this, 0x14);
    if extra == 0 {
        extra = new_object(e, 0x10, |e, block| {
            e.call(EXTRA_RAGDOLL_DATA_INIT, &args![block]).u32()
        });
        let data = new_object(e, 0x14, |e, block| {
            e.call(RAGDOLL_DATA_INIT, &args![block]).u32()
        });
        e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), data);
        list_add(e, this, extra);
    }
    let ragdoll = word_at(e, extra, EXTRA_PAYLOAD);
    if !e.call(RAGDOLL_DATA_LOAD, &args![ragdoll, file]).bool() {
        e.call(LOG_MESSAGE, &args![MESSAGE_BAD_RAGDOLL]);
        list_remove(e, this, extra, true);
    }
}

/// `XACR`: the old form of a parent list entry (the reference, the delay, the flags byte).
fn load_old_activate_ref(e: &mut Engine, this: u32, file: u32) {
    let extra = activate_ref_of(e, this);
    e.with_stack(0x0c, |e, old| {
        let at = old.addr();
        zero_bytes(e, at, 0x0c);
        file_bytes(e, file, at, 0x0c);
        if file_swaps(e, file) {
            e.call(SWAP_DWORD, &args![at, 0u32]);
            e.call(SWAP_DWORD, &args![at + 4, 0u32]);
        }
        let node = new_object(e, 8, |e, block| {
            typed_call!(FAMILY, 0x0041_4010, records::empty_node_new(e, block))
        });
        let reference = e.mem.u32(at);
        e.mem.set_u32(node, reference);
        let delay = x87_quiet_bits(e.mem.u32(at + 4));
        e.mem.set_u32(node.wrapping_add(4), delay);
        add_head(e, extra.wrapping_add(EXTRA_PAYLOAD), node);
        let flags = e.mem.u8(at + 8);
        e.mem.set_u8(extra.wrapping_add(0x14), flags);
    });
}

/// `XNDP`: the old navmesh portal, an 8-byte value made into a type 0x5a extra data on the stack
/// for the setter, then destroyed.
fn load_old_portal(e: &mut Engine, this: u32, file: u32) {
    e.with_stack(8, |e, value| {
        let at = value.addr();
        typed_call!(FAMILY, 0x0041_6ab0, records::portal_record_new(e, at));
        file_bytes(e, file, at, 8);
        if file_swaps(e, file) {
            typed_call!(FAMILY, 0x0041_41e0, records::swap_word_short(e, at));
        }
        let first = e.mem.u32(at);
        let second = u32::from(e.mem.u16(at + 4));
        e.with_stack(0x14, |e, extra| {
            let made = extra.addr();
            typed_call!(
                FAMILY,
                0x0041_6ad0,
                records::portal_extra_new(e, made, first, second)
            );
            e.call(SET_NAVMESH_PORTAL, &args![this, made]);
            e.call(NAVMESH_PORTAL_EXTRA_DESTROY, &args![made]);
        });
    });
}

/// `XPWR`: a reference and its flags; bit 0 adds it as a reflector, bit 1 as reflected.
fn load_reflector(e: &mut Engine, this: u32, file: u32) {
    e.with_stack(8, |e, record| {
        let at = record.addr();
        e.call(SIMPLE_LIST_NEW, &args![at]);
        zero_bytes(e, at, 8);
        file_bytes(e, file, at, 8);
        if file_swaps(e, file) {
            e.call(SWAP_DWORD, &args![at, 0u32]);
            e.call(SWAP_DWORD, &args![at + 4, 0u32]);
        }
        let reference = e.mem.u32(at);
        let flags = e.mem.u32(at + 4);
        if flags & 1 != 0 {
            typed_call!(
                FAMILY,
                0x0041_f1a0,
                rooms::set_reflector_flag_1(e, this, reference, true)
            );
        }
        if flags & 2 != 0 {
            typed_call!(
                FAMILY,
                0x0041_f330,
                rooms::set_reflector_flag_2(e, this, reference, true)
            );
        }
    });
}

/// `XPRM`: radii, colour and type made into a primitive for the list.
fn load_primitive(e: &mut Engine, this: u32, file: u32) {
    e.with_stack(0x20, |e, record| {
        let at = record.addr();
        typed_call!(FAMILY, 0x0041_43f0, records::primitive_new(e, at));
        zero_bytes(e, at, 0x20);
        file_bytes(e, file, at, 0x20);
        if file_swaps(e, file) {
            typed_call!(FAMILY, 0x0041_4290, records::swap_primitive(e, at));
        }
        let kind = e.mem.u32(at + 0x1c);
        let primitive = e.call(PRIMITIVE_CREATE, &args![kind, at, at + 0x0c]).u32();
        if primitive != 0 {
            typed_call!(
                FAMILY,
                0x0041_fa60,
                rooms::add_primitive(e, this, primitive)
            );
        }
    });
}

/// `XOCP`: half extents, centre and an axis-angle rotation built into a BSOcclusionPlane (`0xfc`
/// bytes) for the setter.
fn load_plane(e: &mut Engine, this: u32, file: u32) {
    e.with_stack(0x24, |e, record| {
        let at = record.addr();
        typed_call!(FAMILY, 0x0041_4470, records::plane_new(e, at));
        zero_bytes(e, at, 0x24);
        file_bytes(e, file, at, 0x24);
        if file_swaps(e, file) {
            typed_call!(FAMILY, 0x0041_40f0, records::swap_plane(e, at));
        }
        let block = e.call(ALLOCATE_ALIGNED, &args![0xfcu32]).u32();
        let plane = if block == 0 {
            0
        } else {
            e.call(OCCLUSION_PLANE_INIT, &args![block]).u32()
        };
        typed_call!(FAMILY, 0x0041_6a30, records::plane_set_size(e, plane, at));
        typed_call!(
            FAMILY,
            0x0041_69f0,
            records::plane_set_center(e, plane, at + 8)
        );
        e.with_stack(0x24, |e, matrix| {
            let matrix = matrix.addr();
            let angle = moved_float(e.mem.u32(at + 0x20));
            let x = moved_float(e.mem.u32(at + 0x14));
            let y = moved_float(e.mem.u32(at + 0x18));
            let z = moved_float(e.mem.u32(at + 0x1c));
            typed_call!(
                FAMILY,
                0x0041_68a0,
                records::rotation_matrix(e, matrix, angle, x, y, z)
            );
            typed_call!(
                FAMILY,
                0x0041_6a70,
                records::plane_set_rotation(e, plane, matrix)
            );
        });
        typed_call!(
            FAMILY,
            0x0042_2150,
            followers::set_occlusion_plane(e, this, plane)
        );
    });
}

/// `XRMR`: the count of linked portals with the master flag in bit 16, then that many form ids, each
/// in its own chunk, at the head of the room data's list (+8).
fn load_rooms(e: &mut Engine, this: u32, file: u32) {
    let data = new_object(e, 0x14, |e, block| {
        typed_call!(FAMILY, 0x0041_6b80, records::room_links_new(e, block))
    });
    let word = file_word(e, file);
    e.mem.set_u8(data.wrapping_add(0x10), low_byte(word >> 16));
    let count = word & 0xffff;
    for _ in 0..count {
        e.call(SKIP_CHUNK, &args![file]);
        let room = file_word(e, file);
        add_head(e, data.wrapping_add(8), room);
    }
    typed_call!(FAMILY, 0x0042_0440, rooms::set_room_ref_data(e, this, data));
}
