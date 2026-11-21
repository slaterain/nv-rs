//! The form id map a save writes every form reference through (`BGSSaveLoadFormIDMap`, Xbox PDB):
//! a forward word map from a form id to its save index at `+0`, the reverse map at `+0x10` and the
//! next index at `+0x20`; the maps are `NiTMap<unsigned int, unsigned int>` (vtable `0106cf68`),
//! whose slots hold the word map's hash and compare (`006c6a60`, `006b81d0`, the pathing family's),
//! the item's `SetValue` (`00863840`) and the new item (`00844a70`). A quicksave in Goodsprings
//! writes about 230,000 form ids, each through `SetNumericID` (`00853570`) and the get-or-add here.

use super::FAMILY;
use crate::dispatch;
use crate::pathing;
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::{args, typed_call, Engine};

/// The data handler (`011c3f2c`): the `this` of the created-id test.
const DATA_HANDLER: u32 = 0x011c_3f2c;
/// The save/load game (`011ddf38`); its form id map is the pointer at `+8`.
const SAVE_LOAD_GAME: u32 = 0x011d_df38;
/// The game's `operator new` (`00401000`, cdecl: size).
const ALLOCATE: u32 = 0x0040_1000;
/// The save buffer's append (`00865570`, `thiscall`: source, size, a byte flag).
const APPEND: u32 = 0x0086_5570;

/// The word map's vtable slots, by the functions `NiTMap<unsigned int, unsigned int>` holds there.
const WORD_MAP_HASH: u32 = 0x006c_6a60;
const WORD_MAP_EQUAL: u32 = 0x006b_81d0;
const WORD_MAP_SET_VALUE: u32 = 0x0086_3840;
const WORD_MAP_NEW_ITEM: u32 = 0x0084_4a70;

// Typed from 00469860
/// Whether `id` is a created form's (`0xff000000` and up): `TESDataHandler`'s test (`thiscall`;
/// `this` is not read).
pub fn is_created_id(_e: &mut Engine, _this: Ptr, id: u32) -> bool {
    id >= 0xff00_0000
}

/// A map's hash of `key` (vtable `+4`): the word map's own directly, any other by address.
#[inline(always)]
fn hash(e: &mut Engine, this: Ptr, key: u32) -> u32 {
    match crate::slot_target(e, this.addr(), 4) {
        WORD_MAP_HASH => typed_call!(
            pathing::FAMILY,
            0x006c_6a60,
            pathing::search::hash(e, this.cast(), key)
        ),
        _ => e.vcall(this.addr(), 4, &args![key]).u32(),
    }
}

/// A map's compare of `key` with a node's key (vtable `+8`), the target read once by the caller.
#[inline(always)]
fn equal(e: &mut Engine, this: Ptr, slot: u32, key: u32, existing: u32) -> u8 {
    match slot {
        WORD_MAP_EQUAL => typed_call!(
            pathing::FAMILY,
            0x006b_81d0,
            pathing::search::keys_equal(e, this, key, existing)
        ),
        _ => e.vcall(this.addr(), 8, &args![key, existing]).u8(),
    }
}

// Typed from 00844700
/// `NiTMapBase::SetAt` (`thiscall(this, key, value)`, one body for every map keyed by a word): the
/// key's bucket (vtable `+4`), then its chain (the table at `+8`; a node is the next node, its key
/// and its value), each node's key compared (vtable `+8`); an equal one gets the value. Else a new
/// item (vtable `+0x14`) is given the key and value (vtable `+0xc`), put at the head of the bucket's
/// chain, and the count at `+0xc` goes up. The table is read again after each call, as in the exe.
/// The word map's slots are called directly, any other by address through the vtable.
pub fn set_at(e: &mut Engine, this: Ptr, key: u32, value: u32) {
    let t = this.addr();
    let bucket = hash(e, this, key).wrapping_mul(4);
    let table = e.mem.u32(t.wrapping_add(8));
    let mut node = e.mem.u32(table.wrapping_add(bucket));
    while node != 0 {
        let existing = e.mem.u32(node.wrapping_add(4));
        let slot = crate::slot_target(e, t, 8);
        if equal(e, this, slot, key, existing) != 0 {
            e.mem.set_u32(node.wrapping_add(8), value);
            return;
        }
        node = e.mem.u32(node);
    }
    let node = match crate::slot_target(e, t, 0x14) {
        WORD_MAP_NEW_ITEM => typed_call!(FAMILY, 0x0084_4a70, new_item(e, this)).addr(),
        _ => e.vcall(t, 0x14, &args![]).u32(),
    };
    match crate::slot_target(e, t, 0xc) {
        WORD_MAP_SET_VALUE => typed_call!(
            FAMILY,
            0x0086_3840,
            set_value(e, this, Ptr::new(node), key, value)
        ),
        _ => {
            e.vcall(t, 0xc, &args![node, key, value]);
        }
    }
    let table = e.mem.u32(t.wrapping_add(8));
    let next = e.mem.u32(table.wrapping_add(bucket));
    e.mem.set_u32(node, next);
    let table = e.mem.u32(t.wrapping_add(8));
    e.mem.set_u32(table.wrapping_add(bucket), node);
    let count = e.mem.u32(t.wrapping_add(0xc));
    e.mem.set_u32(t.wrapping_add(0xc), count.wrapping_add(1));
}

// Typed from 00844a70
/// The word map's new item (`thiscall`): its allocator, the member at `+0xc` (`006f99e0`).
pub fn new_item(e: &mut Engine, this: Ptr) -> Ptr {
    typed_call!(
        FAMILY,
        0x006f_99e0,
        allocate_item(e, this.byte_add(0xc).cast())
    )
}

// Typed from 006f99e0
/// The map items' allocator (`thiscall`, `this` not read): twelve bytes from `operator new`
/// (`00401000`), not initialised.
pub fn allocate_item(e: &mut Engine, _this: Ptr) -> Ptr {
    e.call(ALLOCATE, &args![12u32]).ptr()
}

// Typed from 00863840
/// The word map's `SetValue` (`thiscall(map, item, key, value)`, the map not read): the key at
/// `+4` and the value at `+8` of the item.
pub fn set_value(e: &mut Engine, _this: Ptr, item: Ptr, key: u32, value: u32) {
    e.mem.set_u32(item.addr().wrapping_add(4), key);
    e.mem.set_u32(item.addr().wrapping_add(8), value);
}

// Typed from 00846c90
/// The form id map's get-or-add (`thiscall(map, id)`): a created form's id is its own index
/// (`00469860`); else the forward map's index for it (`00853130`), or a new one, the next index
/// (`+0x20`, counted up first), put in the forward map (id to index) and in the reverse map at
/// `+0x10` (index to id) with `SetAt` (`00844700`).
pub fn get_or_add_index(e: &mut Engine, this: Ptr, id: u32) -> u32 {
    let data_handler = e.mem.u32(DATA_HANDLER);
    if typed_call!(
        FAMILY,
        0x0046_9860,
        is_created_id(e, Ptr::new(data_handler), id)
    ) {
        return id;
    }
    e.with_stack(4, |e, out| {
        e.mem.set_u32(out.addr(), 0);
        let found = typed_call!(
            dispatch::FAMILY,
            0x0085_3130,
            dispatch::refs::map_lookup(e, this, id, out.cast())
        );
        if found != 0 {
            return e.mem.u32(out.addr());
        }
        let t = this.addr();
        let index = e.mem.u32(t.wrapping_add(0x20));
        e.mem.set_u32(t.wrapping_add(0x20), index.wrapping_add(1));
        typed_call!(FAMILY, 0x0084_4700, set_at(e, this, id, index));
        typed_call!(
            FAMILY,
            0x0084_4700,
            set_at(e, this.byte_add(0x10), index, id)
        );
        index
    })
}

// Typed from 00846d20
/// The form id map's resolve (`thiscall(map, index)`): a created form's id stands for itself
/// (`00469860`); else the reverse map's id for the index (`00853130` on `+0x10`), 0 when absent.
pub fn resolve_index(e: &mut Engine, this: Ptr, index: u32) -> u32 {
    let data_handler = e.mem.u32(DATA_HANDLER);
    if typed_call!(
        FAMILY,
        0x0046_9860,
        is_created_id(e, Ptr::new(data_handler), index)
    ) {
        return index;
    }
    e.with_stack(4, |e, out| {
        e.mem.set_u32(out.addr(), 0);
        let found = typed_call!(
            dispatch::FAMILY,
            0x0085_3130,
            dispatch::refs::map_lookup(e, this.byte_add(0x10), index, out.cast())
        );
        if found != 0 {
            e.mem.u32(out.addr())
        } else {
            0
        }
    })
}

// Typed from 0084eaf0
/// `BGSNumericIDIndex`'s constructor (`thiscall`): its three bytes cleared one by one.
pub fn index_new(e: &mut Engine, this: Ptr) -> Ptr {
    for offset in 0..3 {
        e.mem.set_u8(this.addr().wrapping_add(offset), 0);
    }
    this
}

/// The save/load game's form id map (`011ddf38`, its `+8`).
#[inline(always)]
fn form_id_map(e: &mut Engine) -> u32 {
    let game = e.mem.u32(SAVE_LOAD_GAME);
    typed_call!(leaf::FAMILY, 0x0044_ddc0, leaf::word_at_8(e, game))
}

// Typed from 00853500
/// `BGSNumericIDIndex::GetNumericID` (Xbox PDB): the three bytes, high byte first; bit 23 marks a
/// created form (`0xff000000` and the low 23 bits), any other index is resolved through the save's
/// form id map (`00846d20`).
pub fn get_numeric_id(e: &mut Engine, this: Ptr) -> u32 {
    let t = this.addr();
    let index = (u32::from(e.mem.u8(t)) << 16)
        | (u32::from(e.mem.u8(t.wrapping_add(1))) << 8)
        | u32::from(e.mem.u8(t.wrapping_add(2)));
    if index & 0x80_0000 != 0 {
        (index & 0x7f_ffff) | 0xff00_0000
    } else {
        let map = form_id_map(e);
        typed_call!(FAMILY, 0x0084_6d20, resolve_index(e, Ptr::new(map), index))
    }
}

// Typed from 00853570
/// `BGSNumericIDIndex::SetNumericID` (Xbox PDB): the id's index from the save's form id map
/// (`00846c90`), bit 23 set for a created form's (top byte `0xff`), stored as three bytes, high
/// byte first.
pub fn set_numeric_id(e: &mut Engine, this: Ptr, id: u32) {
    let map = form_id_map(e);
    let mut index = typed_call!(FAMILY, 0x0084_6c90, get_or_add_index(e, Ptr::new(map), id));
    if index >> 24 == 0xff {
        index |= 0x80_0000;
    }
    let [_, high, middle, low] = index.to_be_bytes();
    let t = this.addr();
    e.mem.set_u8(t, high);
    e.mem.set_u8(t.wrapping_add(1), middle);
    e.mem.set_u8(t.wrapping_add(2), low);
}

/// A form id's three-byte index in a local (`0084eaf0`, `00853570`), appended to the buffer with
/// its separator (`00865570`, 3 bytes, flag 0).
#[inline(always)]
fn append_index(e: &mut Engine, this: Ptr, id: u32) {
    e.with_stack(4, |e, index| {
        typed_call!(FAMILY, 0x0084_eaf0, index_new(e, index));
        typed_call!(FAMILY, 0x0085_3570, set_numeric_id(e, index, id));
        e.call(APPEND, &args![this, index, 3u32, 0u8]);
    });
}

// Typed from 00865db0
/// `BGSSaveGameBuffer::SaveFormID` (Xbox PDB; `RET 8`, the second word unused): the id's index,
/// appended.
pub fn save_numeric_form_id(e: &mut Engine, this: Ptr, id: u32, _unused: u32) {
    append_index(e, this, id);
}

// Typed from 00865df0
/// `BGSSaveGameBuffer::SaveFormID_ov2` (Xbox PDB; `RET 8`, the second word unused): the form's id
/// (`+0xc`, `0084e3a0`; 0 for no form), its index appended.
pub fn save_form_id(e: &mut Engine, this: Ptr, form: Ptr, _unused: u32) {
    let id = if form.is_null() {
        0
    } else {
        typed_call!(leaf::FAMILY, 0x0084_e3a0, leaf::word_at_c(e, form))
    };
    append_index(e, this, id);
}
