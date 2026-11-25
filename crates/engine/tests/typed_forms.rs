//! The differential test of every typed function of the forms family (`engine_typed::forms`)
//! against its translation: `TESForm`'s own functions (`tesform`).
//!
//! One world serves the whole unit: the unit's globals (the registries, the record writer's buffer,
//! the save games, the data handler, the form-type table and the group tag, the big-endian byte);
//! forms with one vtable whose slots are stand-ins, each answering a word the case sets; files,
//! save buffers and components with the real fields the one-line bodies read; handlers at the
//! unit's seams; and doubles for the callees by address that do real work. Most doubles answer a
//! word the case sets; the allocator, the resize and the source file list's operations do their
//! work on the test's own objects, so the loops over the list run as they do in the game. The
//! bodies the typed code does in place have no double ([`IN_PLACE`]): their translations run, and
//! `memcpy` and `memset` run as they are.

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::{Diff, Input};
use engine_typed::crt::CRT;
use engine_typed::forms::{tesform, FORMS};

/// The stand-in slots: `VT_FN + slot`, each answering the word at `SLOT_ANSWERS + slot`.
const VT_FN: u32 = 0x00ff_4000;
const SLOT_ANSWERS: u32 = 0x0300_0000;
/// The answering doubles' words, one per callee in [`ANSWERED`].
const ANSWERS: u32 = 0x0300_1000;
/// The one vtable every test form has.
const VTABLE: u32 = 0x0300_2000;
/// What the form map's lookup finds; whether `operator new` fails; the word a load reads.
const LOOKUP_VALUE: u32 = 0x0300_3000;
const NEW_FAILS: u32 = 0x0300_3004;
const READ_WORD: u32 = 0x0300_300c;
/// Whether the form map's lookup finds [`LOOKUP_VALUE`].
const FOUND: u32 = 0x0300_3010;
/// What `deflate` uses of its output and what it answers.
const DEFLATED: u32 = 0x0300_3014;
const DEFLATE_STATUS: u32 = 0x0300_3018;
/// Whether the string local's constructor leaves it without text.
const STRING_EMPTY: u32 = 0x0300_3020;
/// What the seams' handlers were given: the destructor's, `SetTemporary`'s.
const SEAMS_SEEN: u32 = 0x0300_3024;
/// A float the chunk read puts in the chunk (its bits, 0 for none) and its offset there.
const CHUNK_FLOAT: u32 = 0x0300_302c;
const CHUNK_FLOAT_AT: u32 = 0x0300_3030;
/// What `__RTDynamicCast` answers for each target type: the word at `CASTS + (type & 0xfff)`.
const CASTS: u32 = 0x0300_4000;
const TYPE_TES_FORM: u32 = 0x0118_3028;
const TYPE_TES_OBJECT_REFR: u32 = 0x0118_41cc;
/// The components' type descriptors, in the order the component data casts to them.
const COMPONENT_TYPES: [u32; 8] = [
    0x0118_6b88,
    0x0118_6b6c,
    0x0118_6c3c,
    0x0118_3274,
    0x0118_6bdc,
    0x0118_6c90,
    0x0118_6c74,
    0x0118_68d4,
];
const SLOTS: [u32; 21] = [
    0, 8, 0x10, 0x28, 0x2c, 0x48, 0x4c, 0x60, 0x64, 0x68, 0xc8, 0xdc, 0xf0, 0xf8, 0xfc, 0x100,
    0x108, 0x130, 0x134, 0x1cc, 0x244,
];

// The unit's globals.
const FORM_COUNT: u32 = 0x011c_54bc;
const FORM_MAP: u32 = 0x011c_54c0;
const FORM_LIST: u32 = 0x011c_54c4;
const EDITOR_ID_MAP: u32 = 0x011c_54c8;
const SAVE_BUFFER: u32 = 0x011c_54cc;
const SAVE_BUFFER_SIZE: u32 = 0x011c_54d0;
const BIG_ENDIAN: u32 = 0x011c_54ba;
const DATA_HANDLER: u32 = 0x011c_3f2c;
const SAVE_LOAD_GAME: u32 = 0x011d_e45c;
const BGS_SAVE_LOAD_GAME: u32 = 0x011d_df38;
const FORM_ENUM_TABLE: u32 = 0x0118_7000;
const GROUP_TAG: u32 = 0x0118_7020;
const GROUP_TAG_VALUE: u32 = 0x5055_5247;

// Callees with a double of their own.
const OPERATOR_NEW: u32 = 0x0040_1000;
const OPERATOR_DELETE: u32 = 0x0040_1030;
const BUFFER_REALLOC: u32 = 0x0042_f5d0;
const MAP_LOOKUP: u32 = 0x0085_3130;
const LOAD_BUFFER_READ: u32 = 0x0086_4980;
const SAVE_LOAD_LOAD_DATA: u32 = 0x0085_79e0;
const FORM_NAME_KEY: u32 = 0x0047_4cb0;
const LIST_POP_NODE: u32 = 0x0063_f7b0;
const LIST_REMOVE_ITEM: u32 = 0x0090_5330;
const LIST_APPEND: u32 = 0x0090_5820;
const LIST_PUSH_FRONT: u32 = 0x005a_e3d0;

// Callees answered by a word.
const FILE_ADD_FORM: u32 = 0x0047_2fe0;
const COMPONENTS_COMPARE: u32 = 0x0047_c670;
const RT_DYNAMIC_CAST: u32 = 0x00ec_43fb;
const STRING_COMPARE: u32 = 0x0040_4dc0;
const FILE_GET_INDEX_FILE: u32 = 0x0047_1a10;

/// The one-line bodies the typed code does in place (see `tesform`'s module comment). The
/// translations call them and the typed functions do not, so every test leaves them, with what
/// they call, out of the translation's calls. No double stands at any of them: the translations
/// run, so what they answer is what the exe's bodies answer, from the objects' real fields.
const IN_PLACE: &[u32] = &[
    0x0040_1170, // the form type (and the one-byte component getters)
    0x0040_77c0, // temporary
    0x0046_0340, // flag 2
    0x0046_0140, // the list's first node
    0x0040_1500, // the big-endian byte
    0x0040_1080, // the word swap
    0x0040_7a90, // the half swap
    0x0041_41e0, // the chunk header's swap
    0x0047_0650, // FORM::Endian
    0x0042_8110, // a save buffer's save kind
    0x0042_80f0, // a word's bits
    0x0047_0bb0, // the record version
    0x0047_3070, // the record writer's buffer
    0x0047_3080, // and its size
    0x0047_c850, // the save/load stub
    0x008c_71b0, // the store of a word
    0x0047_1c20, // TESFile::GetMaster
    0x0046_4df0, // the file's master list
    0x0047_3c70, // the end of the file's chain
    0x0047_3250, // the file's compile index
    0x0040_1660, // the file's chunk size
    0x0040_1680, // the file's big-endian byte
    0x0048_2f20, // the data handler's flag byte
    0x0040_1280, // the form's name
    0x0040_2d30, // the base component's constructor
    0x0096_a2d0, // the empty list's
    0x0047_c050, // the component collection's
    0x006e_cd40, // the health's setter
    0x0047_ff50, // the attack damage's setter
    0x004f_1540, // the quality's getter
    0x006b_9130, // the weight's getter
    0x004f_5d90, // and setter
    0x009d_2510, // the attributes' size
    0x0044_0e30, // the form type's name
];

/// A file's fields the in-place bodies read: its flags (bit 0 a master), master list, next file,
/// compile index, current chunk's size and byte-swapped byte.
const FILE_FLAGS: u32 = 0x3e8;
const FILE_MASTERS: u32 = 0x3ec;
const FILE_PARENT: u32 = 4;
const FILE_COMPILE_INDEX_AT: u32 = 0x40c;
const FILE_CHUNK_SIZE_AT: u32 = 0x25c;
const FILE_SWAPPED_AT: u32 = 0x299;
/// A save buffer's save kind (+0x17) and the data handler's flag byte (+0x620).
const SAVE_KIND_AT: u32 = 0x17;
const HANDLER_NO_IDS: u32 = 0x620;

/// Every callee answered by a word of its own.
const ANSWERED: [u32; 49] = [
    0x0047_0470, // the source file list's destructor
    0x0046_ffb0, // and its wrapper
    0x0048_6b80, // the form array's remove
    0x0047_0140, // its add
    0x0047_01d0, // its constructor
    0x0086_3db0, // its clear
    0x0048_6ab0, // the editor id map's
    0x0043_8af0, // a map's clear
    0x0040_5430, // a map's remove
    0x0084_4700, // a map's set
    0x0046_96f0, // the data handler's id removal
    0x0085_6c70, // TESSaveLoadGame's change removal
    0x0085_a410, // and its form removal
    0x0084_a7c0, // BGSSaveLoadGame's change removal
    0x0084_a690, // its add-change
    0x0084_a780, // and that one's sibling
    0x0047_2e60, // TESFile::StartForm
    FILE_ADD_FORM,
    0x0047_3090, // the file's call after a deleted form's start
    0x0047_c070, // its fill
    0x0047_c560, // its visit of slot 0
    0x0047_c5b0, // and of slot 1
    0x0047_c600, // its copy
    COMPONENTS_COMPARE,
    0x0048_ce80, // type 0x39's clear
    0x005b_5e40, // the log
    0x0085_79b0, // SaveGameDataOLD
    0x0085_7a10, // SaveNumericID
    0x0085_7aa0, // LoadNumericID
    0x0086_5e50, // the save buffer's write
    STRING_COMPARE,
    0x0047_2f60, // TESFile::LoadForm
    FILE_GET_INDEX_FILE,
    0x0046_9800, // its next id
    0x0043_b2b0, // the warning count
    0x00b4_6360, // deflateInit_
    0x00b4_7370, // deflateEnd
    0x004f_15a0, // a component byte's setter
    0x0048_e960, // the value's setter
    0x0048_0100, // their save
    0x0048_0130, // their load
    0x0046_5110, // CreateFormOfType
    0x0040_37d0, // a string local's destructor
    0x0040_6f60, // sprintf
    0x0048_6a60, // the map base's destruction
    0x0048_6c80, // the string map base's
    0x0048_6dc0, // the char key map base's
    0x0048_6d60, // NiTMap's
    0x0048_6ca0, // the string template map's
];

fn ret(eax: u32) -> Ret {
    Ret {
        eax,
        ..Ret::default()
    }
}

fn answer_at(addr: u32) -> u32 {
    let index = ANSWERED
        .iter()
        .position(|a| *a == addr)
        .expect("an answered callee");
    ANSWERS + 4 * index as u32
}

fn answer<const A: u32>(e: &mut Engine, _: &[u32]) -> Ret {
    ret(e.mem.u32(answer_at(A)))
}

fn set_answer(e: &mut Engine, addr: u32, value: u32) {
    e.mem.set_u32(answer_at(addr), value);
}

fn slot_answer<const S: u32>(e: &mut Engine, _: &[u32]) -> Ret {
    ret(e.mem.u32(SLOT_ANSWERS + S))
}

/// A heap block of `size` bytes with its size in the word before it (what the resize reads).
fn heap_block(e: &mut Engine, size: u32) -> u32 {
    let block = e.mem.alloc(size.max(1) + 4);
    e.mem.set_u32(block, size);
    block + 4
}

fn copy_bytes(e: &mut Engine, to: u32, from: u32, n: u32) {
    for i in 0..n {
        let b = e.mem.u8(from + i);
        e.mem.set_u8(to + i, b);
    }
}

/// `BSSimpleList`'s removal of a node's item: the next node's moves up, the last node is cleared.
fn pop_node(e: &mut Engine, node: u32) {
    let next = e.mem.u32(node + 4);
    if next != 0 {
        let item = e.mem.u32(next);
        let after = e.mem.u32(next + 4);
        e.mem.set_u32(node, item);
        e.mem.set_u32(node + 4, after);
    } else {
        e.mem.set_u32(node, 0);
    }
}

macro_rules! answered {
    ($e:expr, $($addr:expr),* $(,)?) => {
        $( $e.register($addr, answer::<{ $addr }>); )*
    };
}

/// The world every case starts from.
fn world(e: &mut Engine) {
    for page in [
        0x011c_5000,
        0x011c_3000,
        0x011d_d000,
        0x011d_e000,
        0x0118_7000,
    ] {
        e.map(page, 0x1000);
    }
    e.map(SLOT_ANSWERS, 0x8000);
    for slot in SLOTS {
        e.mem.set_u32(VTABLE + slot, VT_FN + slot);
    }
    e.register(VT_FN, slot_answer::<0>);
    e.register(VT_FN + 0x10, slot_answer::<0x10>);
    e.register(VT_FN + 0x28, slot_answer::<0x28>);
    e.register(VT_FN + 0x2c, slot_answer::<0x2c>);
    e.register(VT_FN + 0x48, slot_answer::<0x48>);
    e.register(VT_FN + 0x4c, slot_answer::<0x4c>);
    e.register(VT_FN + 0xc8, slot_answer::<0xc8>);
    e.register(VT_FN + 0xdc, slot_answer::<0xdc>);
    e.register(VT_FN + 0xf0, slot_answer::<0xf0>);
    e.register(VT_FN + 0x130, slot_answer::<0x130>);
    e.register(VT_FN + 8, slot_answer::<8>);
    e.register(VT_FN + 0x60, slot_answer::<0x60>);
    e.register(VT_FN + 0x68, slot_answer::<0x68>);
    e.register(VT_FN + 0xf8, slot_answer::<0xf8>);
    e.register(VT_FN + 0xfc, slot_answer::<0xfc>);
    e.register(VT_FN + 0x100, slot_answer::<0x100>);
    e.register(VT_FN + 0x108, slot_answer::<0x108>);
    e.register(VT_FN + 0x134, slot_answer::<0x134>);
    e.register(VT_FN + 0x1cc, slot_answer::<0x1cc>);
    e.register(VT_FN + 0x244, slot_answer::<0x244>);
    // The slot answering a float (`ST0`) as well as its bits.
    e.register(VT_FN + 0x64, |e, _| {
        let bits = e.mem.u32(SLOT_ANSWERS + 0x64);
        Ret {
            eax: bits,
            edx: 0,
            st0: f64::from(f32::from_bits(bits)),
        }
    });
    // The form-type table's packed strings and the group tag.
    for t in 0..0x79u32 {
        e.mem.set_u32(FORM_ENUM_TABLE + 12 * t + 8, 0x4d52_0000 | t);
        // Its name pointer (what `00440e30` reads), a word of its own.
        e.mem.set_u32(FORM_ENUM_TABLE + 12 * t + 4, 0x0101_0000 | t);
    }
    e.mem.set_u32(GROUP_TAG, GROUP_TAG_VALUE);
    // The seams the destructor (the script extender's, `00483669`) and `SetTemporary` (JohnnyGuitar
    // NVSE's, `0048449a`) keep: each handler notes the form it is given, so a seam not reached, or
    // given another word, shows in memory.
    e.on_site_for_test(0x0048_3669, |e, a| {
        e.mem.set_u32(SEAMS_SEEN, a[0]);
        None
    });
    e.on_site_for_test(0x0048_449a, |e, a| {
        e.mem.set_u32(SEAMS_SEEN + 4, a[0]);
        None
    });
    answered!(
        e,
        0x0047_0470,
        0x0046_ffb0,
        0x0048_6b80,
        0x0047_0140,
        0x0047_01d0,
        0x0086_3db0,
        0x0048_6ab0,
        0x0043_8af0,
        0x0040_5430,
        0x0084_4700,
        0x0046_96f0,
        0x0085_6c70,
        0x0085_a410,
        0x0084_a7c0,
        0x0084_a690,
        0x0084_a780,
        0x0047_2e60,
        0x0047_2fe0,
        0x0047_3090,
        0x0047_c070,
        0x0047_c560,
        0x0047_c5b0,
        0x0047_c600,
        0x0047_c670,
        0x0048_ce80,
        0x005b_5e40,
        0x0085_79b0,
        0x0085_7a10,
        0x0085_7aa0,
        0x0086_5e50,
        0x0040_4dc0,
        0x0047_2f60,
        0x0047_1a10,
        0x0046_9800,
        0x0043_b2b0,
        0x00b4_6360,
        0x00b4_7370,
        0x004f_15a0,
        0x0048_e960,
        0x0048_0100,
        0x0048_0130,
        0x0046_5110,
        0x0040_37d0,
        0x0040_6f60,
        0x0048_6a60,
        0x0048_6c80,
        0x0048_6dc0,
        0x0048_6d60,
        0x0048_6ca0,
    );
    e.register(OPERATOR_NEW, |e, a| {
        if e.mem.u32(NEW_FAILS) != 0 {
            ret(0)
        } else {
            ret(heap_block(e, a[0]))
        }
    });
    e.register(OPERATOR_DELETE, |_, _| ret(0));
    e.register(RT_DYNAMIC_CAST, |e, a| {
        ret(e.mem.u32(CASTS + (a[3] & 0xfff)))
    });
    e.register(0x00b4_6b70, |e, a| {
        let left = e.mem.u32(a[0] + 0x10).wrapping_sub(e.mem.u32(DEFLATED));
        e.mem.set_u32(a[0] + 0x10, left);
        ret(e.mem.u32(DEFLATE_STATUS))
    });
    e.register(0x0047_2890, |e, a| {
        for i in 0..a[2] {
            e.mem.set_u8(a[1] + i, (i * 13 + 5) as u8);
        }
        // A float of the case's own at the weight's offset, when it fits.
        let (bits, at) = (e.mem.u32(CHUNK_FLOAT), e.mem.u32(CHUNK_FLOAT_AT));
        if bits != 0 && at + 4 <= a[2] {
            e.mem.set_u32(a[1] + at, bits);
        }
        ret(0)
    });
    e.register(0x0040_c0e0, |e, a| {
        e.mem.set_u32(
            a[0],
            if e.mem.u32(STRING_EMPTY) != 0 {
                0
            } else {
                a[1]
            },
        );
        ret(a[0])
    });
    // The buckets' allocation: a block of its own (the map base's `memset` zeroes it).
    e.register(0x00aa_1070, |e, a| ret(heap_block(e, a[0])));
    e.register(BUFFER_REALLOC, |e, a| {
        let block = heap_block(e, a[1]);
        if a[0] != 0 {
            let old = e.mem.u32(a[0] - 4).min(a[1]);
            copy_bytes(e, block, a[0], old);
        }
        ret(block)
    });
    e.register(MAP_LOOKUP, |e, a| {
        let found = e.mem.u32(FOUND);
        if found != 0 {
            let value = e.mem.u32(LOOKUP_VALUE);
            e.mem.set_u32(a[2], value);
        }
        ret(found)
    });
    e.register(LOAD_BUFFER_READ, |e, a| {
        let word = e.mem.u32(READ_WORD);
        e.mem.set_u32(a[1], word);
        ret(0)
    });
    e.register(SAVE_LOAD_LOAD_DATA, |e, a| {
        let word = e.mem.u32(READ_WORD);
        e.mem.set_u32(a[1], word);
        ret(0)
    });
    e.register(FORM_NAME_KEY, |e, a| ret(e.mem.u32(a[0] + 0x30)));
    e.register(LIST_POP_NODE, |e, a| {
        pop_node(e, a[0]);
        ret(0)
    });
    e.register(LIST_REMOVE_ITEM, |e, a| {
        let item = e.mem.u32(a[1]);
        let mut node = a[0];
        while node != 0 {
            if e.mem.u32(node) == item {
                pop_node(e, node);
                break;
            }
            node = e.mem.u32(node + 4);
        }
        ret(0)
    });
    e.register(LIST_APPEND, |e, a| {
        let item = e.mem.u32(a[1]);
        let mut node = a[0];
        if e.mem.u32(node) == 0 && e.mem.u32(node + 4) == 0 {
            e.mem.set_u32(node, item);
            return ret(0);
        }
        while e.mem.u32(node + 4) != 0 {
            node = e.mem.u32(node + 4);
        }
        let new = e.mem.alloc(8);
        e.mem.set_u32(new, item);
        e.mem.set_u32(node + 4, new);
        ret(0)
    });
    e.register(LIST_PUSH_FRONT, |e, a| {
        let item = e.mem.u32(a[1]);
        let first = a[0];
        if e.mem.u32(first) == 0 && e.mem.u32(first + 4) == 0 {
            e.mem.set_u32(first, item);
            return ret(0);
        }
        let new = e.mem.alloc(8);
        let (old_item, old_next) = (e.mem.u32(first), e.mem.u32(first + 4));
        e.mem.set_u32(new, old_item);
        e.mem.set_u32(new + 4, old_next);
        e.mem.set_u32(first, item);
        e.mem.set_u32(first + 4, new);
        ret(0)
    });
}

/// What `__RTDynamicCast` answers for `target`.
fn set_cast(e: &mut Engine, target: u32, value: u32) {
    e.mem.set_u32(CASTS + (target & 0xfff), value);
}

/// Whether the form map's lookup finds its value (a word beside the answers).
fn set_found(e: &mut Engine, found: bool) {
    e.mem.set_u32(FOUND, u32::from(found));
}

/// A form: the test vtable, a type, flags and an id; no source files; no name key.
fn form(e: &mut Engine, kind: u32, flags: u32, id: u32) -> u32 {
    let f = e.mem.alloc(0x40);
    e.mem.set_u32(f, VTABLE);
    e.mem.set_u8(f + 4, kind as u8);
    e.mem.set_u32(f + 8, flags);
    e.mem.set_u32(f + 0xc, id);
    f
}

/// A file `TESFile::GetMaster` accepts (`master`: bit 0 of its flags) or not, with no next file, an
/// empty master list, compile index 0 and no chunk.
fn file(e: &mut Engine, master: bool) -> u32 {
    let f = e.mem.alloc(0x410);
    e.mem.set_u32(f + FILE_FLAGS, 0x40 | u32::from(master));
    f
}

/// The form's source file list: the first node (embedded at +0x10) holds `files[0]`, each further
/// file a node of its own (a 0 is a node without a file).
fn set_files(e: &mut Engine, form: u32, files: &[u32]) {
    let mut node = form + 0x10;
    for (i, f) in files.iter().enumerate() {
        if i > 0 {
            let next = e.mem.alloc(8);
            e.mem.set_u32(node + 4, next);
            node = next;
        }
        e.mem.set_u32(node, *f);
    }
}

/// The form array: `count` slots of other forms, `form` at `at` when given.
fn form_list(e: &mut Engine, count: u32, holds: Option<(u32, u32)>) -> u32 {
    let list = e.mem.alloc(0x18);
    let items = e.mem.alloc(4 * count.max(1));
    for i in 0..count {
        let other = form(e, 0x20, 2, 0x100 + i);
        e.mem.set_u32(items + 4 * i, other);
    }
    if let Some((form, at)) = holds {
        e.mem.set_u32(items + 4 * at, form);
    }
    e.mem.set_u32(list, VTABLE);
    e.mem.set_u32(list + 4, items);
    e.mem.set_u32(list + 0xc, count);
    list
}

/// The registries, the save games and the data handler, each there or not, the form listed in the
/// array when `listed` (2: there and listed, 1: there, 0: no array).
fn registries(e: &mut Engine, map: bool, listed: u32, handler: bool, save_games: bool, form: u32) {
    if map {
        let m = self::form(e, 0x7f, 0, 0);
        e.mem.set_u32(FORM_MAP, m);
        let names = self::form(e, 0x7f, 0, 0);
        e.mem.set_u32(EDITOR_ID_MAP, names);
    }
    match listed {
        2 => {
            let list = form_list(e, 3, Some((form, 1)));
            e.mem.set_u32(FORM_LIST, list);
        }
        1 => {
            let list = form_list(e, 2, None);
            e.mem.set_u32(FORM_LIST, list);
        }
        _ => {}
    }
    if handler {
        let h = e.mem.alloc(0x630);
        e.mem.set_u32(DATA_HANDLER, h);
    }
    if save_games {
        let s = e.mem.alloc(0x20);
        e.mem.set_u32(SAVE_LOAD_GAME, s);
        let b = e.mem.alloc(0x20);
        e.mem.set_u32(BGS_SAVE_LOAD_GAME, b);
    }
}

/// The record writer's buffer, as `StartForm` leaves it: a 0x18 byte header.
fn record_buffer(e: &mut Engine) {
    let buffer = heap_block(e, 0x18);
    for i in 0..0x18 {
        e.mem.set_u8(buffer + i, (i as u8 + 1).wrapping_mul(0x11));
    }
    e.mem.set_u32(SAVE_BUFFER, buffer);
    e.mem.set_u32(SAVE_BUFFER_SIZE, 0x18);
}

/// A block of `size` bytes of data, each byte its own.
fn data(e: &mut Engine, size: u32) -> u32 {
    let d = heap_block(e, size);
    for i in 0..size {
        e.mem.set_u8(d + i, (i * 7 + 3) as u8);
    }
    d
}

/// Every test runs with this family and the families its typed calls reach (the leaves', the C
/// runtime's: `memcpy` and `memset` run as they are), and leaves the bodies done in place out of
/// the translation's calls. The maps' lookup (dispatch's `00853130`) and set (the save family's
/// `00844700`) are doubles over the test's stand-in maps, so those families are not named: their
/// typed calls go by address to the doubles on both sides (both families, like this one, run on
/// every thread).
fn forms_cases(d: Diff) -> Diff {
    d.family(&FORMS)
        .family(&leaf::LEAVES)
        .family(&CRT)
        .goes_by_address(&[0x0085_3130, 0x0084_4700])
        .ignore_calls(IN_PLACE)
}

/// The cases several tests share, as steps of the `Diff` chain, so that each test is one statement
/// from its `Diff::new` to its `.run().assert_same()`.
trait Cases {
    fn forms(self) -> Diff;
    fn flag_setter(self) -> Diff;
    fn change_call(self) -> Diff;
    fn forwarder(self) -> Diff;
    fn value_chunk(self) -> Diff;
    fn map_object(self) -> Diff;
    fn chunk_writer(self, values: &'static [u32]) -> Diff;
}

impl Cases for Diff {
    fn forms(self) -> Diff {
        forms_cases(self)
    }
    fn flag_setter(self) -> Diff {
        flag_setter_cases(self)
    }
    fn change_call(self) -> Diff {
        change_call_cases(self)
    }
    fn forwarder(self) -> Diff {
        forwarder_cases(self)
    }
    fn value_chunk(self) -> Diff {
        value_chunk_cases(self)
    }
    fn map_object(self) -> Diff {
        map_object_cases(self)
    }
    fn chunk_writer(self, values: &'static [u32]) -> Diff {
        chunk_writer_cases(self, values)
    }
}

const WORDS: &[u32] = &[0, 1, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff];
const FLAGS: &[u32] = &[0, 0xffff_ffff, 0x0f0f_0f0f];
const TEMPORARY: &[u32] = &[0, 0x4000];

#[test]
fn handler_loading_file() {
    Diff::new(entry!(0x0048_35e0, tesform::handler_loading_file(u32) -> u32))
        .forms()
        .inputs(&[Input::Of(WORDS)])
        .setup(|e, case| {
            world(e);
            let h = e.mem.alloc(0x210);
            e.mem.set_u32(h + 0x20c, case.word(0));
            vec![h]
        })
        .run()
        .assert_same();
}

/// A form with the registries, the save games and the handler around it, for the destructor's
/// cases: (temporary or not, the forms left after it, the save games there or not).
fn dying_form(e: &mut Engine, flags: u32, count: u32, save_games: bool) -> u32 {
    world(e);
    let f = form(e, 0x20, flags, 0x0100_0abc);
    registries(e, true, 2, true, save_games, f);
    e.mem.set_u32(FORM_COUNT, count);
    f
}

#[test]
fn scalar_deleting_destructor() {
    Diff::new(entry!(
        0x0048_3600,
        tesform::scalar_deleting_destructor(u32, u32) -> u32
    ))
    .forms()
    .inputs(&[
        Input::Of(&[0, 1, 3]),
        Input::Of(TEMPORARY),
        Input::Of(&[1, 2]),
        Input::Flag,
    ])
    .setup(|e, case| {
        let f = dying_form(e, case.word(1), case.word(2), case.word(3) != 0);
        vec![f, case.word(0)]
    })
    .run()
    .assert_same();
}

#[test]
fn destructor() {
    Diff::new(entry!(0x0048_3630, tesform::destructor(u32)))
        .forms()
        .inputs(&[Input::Of(TEMPORARY), Input::Of(&[1, 2]), Input::Flag])
        .setup(|e, case| vec![dying_form(e, case.word(0), case.word(1), case.word(2) != 0)])
        .run()
        .assert_same();
}

/// The form types the component visits tell apart: none, the 0x39 sub-object, the rest.
const VISIT_TYPES: &[u32] = &[
    0x11, 0x3a, 0x3d, 0x40, 0x41, 0x42, 0x43, 0x49, 0x69, 0x39, 0x05, 0x6a,
];

#[test]
fn each_component_slot_0() {
    Diff::new(entry!(0x0048_3720, tesform::each_component_slot_0(u32)))
        .forms()
        .inputs(&[Input::Of(VISIT_TYPES)])
        .setup(|e, case| {
            world(e);
            vec![form(e, case.word(0), 0, 0x10)]
        })
        .run()
        .assert_same();
}

#[test]
fn each_component_slot_1() {
    Diff::new(entry!(0x0048_3870, tesform::each_component_slot_1(u32)))
        .forms()
        .inputs(&[Input::Of(VISIT_TYPES)])
        .setup(|e, case| {
            world(e);
            vec![form(e, case.word(0), 0, 0x10)]
        })
        .run()
        .assert_same();
}

#[test]
fn form_by_id() {
    Diff::new(entry!(0x0048_39c0, tesform::form_by_id(u32) -> u32))
        .forms()
        .inputs(&[Input::Flag, Input::Flag, Input::Of(&[0, 0x14, 0xff00_0001])])
        .setup(|e, case| {
            world(e);
            if case.word(0) != 0 {
                let m = form(e, 0x7f, 0, 0);
                e.mem.set_u32(FORM_MAP, m);
            }
            set_found(e, case.word(1) != 0);
            e.mem.set_u32(LOOKUP_VALUE, 0x0bad_f00d);
            vec![case.word(2)]
        })
        .run()
        .assert_same();
}

#[test]
fn create_registries() {
    Diff::new(entry!(0x0048_3a50, tesform::create_registries()))
        .forms()
        .inputs(&[Input::Flag, Input::Flag, Input::Flag, Input::Flag])
        .setup(|e, case| {
            world(e);
            for (i, global) in [FORM_MAP, EDITOR_ID_MAP, FORM_LIST].into_iter().enumerate() {
                if case.word(i) != 0 {
                    e.mem.set_u32(global, 0x0123_4560 + i as u32);
                }
            }
            e.mem.set_u32(NEW_FAILS, case.word(3));
            vec![]
        })
        .run()
        .assert_same();
}

#[test]
fn destroy_registries() {
    Diff::new(entry!(0x0048_3b80, tesform::destroy_registries()))
        .forms()
        .inputs(&[Input::Flag, Input::Flag, Input::Flag])
        .setup(|e, case| {
            world(e);
            for (i, global) in [FORM_MAP, EDITOR_ID_MAP, FORM_LIST].into_iter().enumerate() {
                if case.word(i) != 0 {
                    let object = form(e, 0x7f, 0, i as u32);
                    e.mem.set_u32(global, object);
                }
            }
            vec![]
        })
        .run()
        .assert_same();
}

#[test]
fn remove_from_data_structures() {
    Diff::new(entry!(
        0x0048_3c70,
        tesform::remove_from_data_structures(u32)
    ))
    .forms()
    .inputs(&[
        Input::Of(TEMPORARY),
        Input::Flag,
        Input::Of(&[0, 1, 2]),
        Input::Flag,
    ])
    .setup(|e, case| {
        world(e);
        let f = form(e, 0x20, case.word(0), 0x0100_0abc);
        registries(
            e,
            case.word(1) != 0,
            case.word(2),
            case.word(3) != 0,
            false,
            f,
        );
        vec![f]
    })
    .run()
    .assert_same();
}

#[test]
fn save() {
    Diff::new(entry!(0x0048_3d20, tesform::save(u32, u32) -> bool))
        .forms()
        .inputs(&[Input::Of(TEMPORARY), Input::Of(&[0, 1, 0x100])])
        .setup(|e, case| {
            world(e);
            set_answer(e, FILE_ADD_FORM, case.word(1));
            let f = form(e, 0x20, case.word(0), 0x14);
            vec![f, file(e, true)]
        })
        .run()
        .assert_same();
}

#[test]
fn write_to_file() {
    Diff::new(entry!(0x0048_3fc0, tesform::write_to_file(u32, u32) -> u8))
        .forms()
        .inputs(&[
            Input::Of(&[0, 0x4000, 0x20, 0x4020]),
            Input::Of(&[0, 1, 0x1ff]),
        ])
        .setup(|e, case| {
            world(e);
            e.mem.set_u32(SLOT_ANSWERS + 0x28, case.word(1));
            let f = form(e, 0x20, case.word(0), 0x14);
            vec![f, file(e, false)]
        })
        .run()
        .assert_same();
}

/// Form types whose ranks the file order tells apart: the four ranked apart and some others.
const RANKED_TYPES: &[u32] = &[0, 1, 0x29, 0x67, 0x6c, 0x73, 0x74];

#[test]
fn type_ranks_before() {
    Diff::new(entry!(0x0048_4020, tesform::type_ranks_before(u32, u32) -> bool))
        .forms()
        .inputs(&[Input::Of(RANKED_TYPES), Input::Of(RANKED_TYPES)])
        .setup(|e, case| {
            world(e);
            let a = form(e, case.word(0), 0, 1);
            let b = form(e, case.word(1), 0, 2);
            vec![a, b]
        })
        .run()
        .assert_same();
}

#[test]
fn goes_before_group() {
    // 100: no record; 101: a record that is not a group; else the group type.
    Diff::new(entry!(0x0048_4150, tesform::goes_before_group(u32, u32) -> bool))
        .forms()
        .inputs(&[
            Input::Of(&[100, 101, 0, 1, 2, 3, 4, 5, 6, 7, 8]),
            Input::Of(&[0x23, 0x28, 0x3a, 0x41, 0x67]),
        ])
        .setup(|e, case| {
            world(e);
            let f = form(e, case.word(1), 0, 1);
            let record = match case.word(0) {
                100 => 0,
                kind => {
                    let r = e.mem.alloc(0x18);
                    let tag = if kind == 101 {
                        0x4f4e_5055
                    } else {
                        GROUP_TAG_VALUE
                    };
                    e.mem.set_u32(r, tag);
                    e.mem.set_u32(r + 8, 0x5452_4f46);
                    e.mem.set_u32(r + 0xc, kind);
                    r
                }
            };
            vec![f, record]
        })
        .run()
        .assert_same();
}

#[test]
fn set_temporary() {
    Diff::new(entry!(0x0048_4490, tesform::set_temporary(u32)))
        .forms()
        .inputs(&[Input::Flag, Input::Of(TEMPORARY), Input::Of(&[0, 2])])
        .setup(|e, case| {
            world(e);
            let f = form(e, 0x20, case.word(1), 0x0100_0abc);
            registries(e, true, case.word(2), true, case.word(0) != 0, f);
            vec![f]
        })
        .run()
        .assert_same();
}

/// A flag setter's cases: on or off, over forms whose flags are all clear, all set or mixed.
fn flag_setter_cases(d: Diff) -> Diff {
    d.forms()
        .inputs(&[Input::Flag, Input::Of(FLAGS)])
        .setup(|e, case| {
            world(e);
            vec![form(e, 0x20, case.word(1), 0x14), case.word(0)]
        })
}

#[test]
fn set_flag_1() {
    Diff::new(entry!(0x0048_44f0, tesform::set_flag_1(u32, bool)))
        .flag_setter()
        .run()
        .assert_same();
}

#[test]
fn set_delete() {
    Diff::new(entry!(0x0048_4530, tesform::set_delete(u32, bool)))
        .flag_setter()
        .run()
        .assert_same();
}

#[test]
fn set_empty() {
    Diff::new(entry!(0x0048_4580, tesform::set_empty(u32, bool)))
        .forms()
        .inputs(&[Input::Flag, Input::Of(FLAGS)])
        .setup(|e, case| {
            world(e);
            vec![form(e, 0x20, case.word(1), 0x14), case.word(0)]
        })
        .run()
        .assert_same();
}

#[test]
fn set_flag_2000() {
    Diff::new(entry!(0x0048_4610, tesform::set_flag_2000(u32, bool)))
        .flag_setter()
        .run()
        .assert_same();
}

#[test]
fn set_flag_800000() {
    Diff::new(entry!(0x0048_4650, tesform::set_flag_800000(u32, bool)))
        .flag_setter()
        .run()
        .assert_same();
}

#[test]
fn set_destructible() {
    Diff::new(entry!(0x0048_46a0, tesform::set_destructible(u32, bool)))
        .flag_setter()
        .run()
        .assert_same();
}

#[test]
fn set_flag_4000000() {
    Diff::new(entry!(0x0048_46e0, tesform::set_flag_4000000(u32, bool)))
        .flag_setter()
        .run()
        .assert_same();
}

#[test]
fn set_listed_flag() {
    Diff::new(entry!(0x0048_4730, tesform::set_listed_flag(u32, bool)))
        .forms()
        .inputs(&[
            Input::Flag,
            Input::Of(&[0, 2, 0x4000, 0x4002]),
            Input::Of(&[0, 1, 2]),
        ])
        .setup(|e, case| {
            world(e);
            let f = form(e, 0x20, case.word(1), 0x14);
            registries(e, false, case.word(2), false, false, f);
            vec![f, case.word(0)]
        })
        .run()
        .assert_same();
}

#[test]
fn set_flag_400() {
    Diff::new(entry!(0x0048_4860, tesform::set_flag_400(u32, bool)))
        .flag_setter()
        .run()
        .assert_same();
}

#[test]
fn set_flag_40() {
    Diff::new(entry!(0x0048_48b0, tesform::set_flag_40(u32, bool)))
        .flag_setter()
        .run()
        .assert_same();
}

#[test]
fn set_flag_10000() {
    Diff::new(entry!(0x0048_4900, tesform::set_flag_10000(u32, bool)))
        .flag_setter()
        .run()
        .assert_same();
}

#[test]
fn set_flag_20000() {
    Diff::new(entry!(0x0048_4950, tesform::set_flag_20000(u32, bool)))
        .flag_setter()
        .run()
        .assert_same();
}

#[test]
fn set_flag_100000() {
    Diff::new(entry!(0x0048_4990, tesform::set_flag_100000(u32, bool)))
        .flag_setter()
        .run()
        .assert_same();
}

#[test]
fn set_flag_200() {
    Diff::new(entry!(0x0048_49d0, tesform::set_flag_200(u32, bool)))
        .flag_setter()
        .run()
        .assert_same();
}

#[test]
fn set_flag_40_again() {
    Diff::new(entry!(0x0048_4a10, tesform::set_flag_40_again(u32, bool)))
        .flag_setter()
        .run()
        .assert_same();
}

#[test]
fn set_fire_off() {
    Diff::new(entry!(0x0048_4a70, tesform::set_fire_off(u32, bool)))
        .flag_setter()
        .run()
        .assert_same();
}

#[test]
fn set_flag_8() {
    Diff::new(entry!(0x0048_4ab0, tesform::set_flag_8(u32, bool)))
        .flag_setter()
        .run()
        .assert_same();
}

#[test]
fn set_disabled() {
    Diff::new(entry!(0x0048_4af0, tesform::set_disabled(u32, bool)))
        .flag_setter()
        .run()
        .assert_same();
}

/// Ids on both sides of the low range's bounds.
const IDS: &[u32] = &[0, 1, 0x7ff, 0x800, 0x0100_0001, 0xffff_ffff];

#[test]
fn id_in_low_range() {
    Diff::new(entry!(0x0048_4b40, tesform::id_in_low_range(u32) -> bool))
        .forms()
        .inputs(&[Input::Of(IDS)])
        .setup(|e, case| {
            world(e);
            vec![case.word(0)]
        })
        .run()
        .assert_same();
}

/// A change call's cases: the change words, with the save game there.
fn change_call_cases(d: Diff) -> Diff {
    d.forms().inputs(&[Input::Of(WORDS)]).setup(|e, case| {
        world(e);
        let f = form(e, 0x20, 0, 0x14);
        registries(e, false, 0, false, true, f);
        vec![f, case.word(0)]
    })
}

#[test]
fn add_change() {
    Diff::new(entry!(0x0048_4b60, tesform::add_change(u32, u32)))
        .change_call()
        .run()
        .assert_same();
}

#[test]
fn force_change() {
    Diff::new(entry!(0x0048_4b90, tesform::force_change(u32, u32)))
        .change_call()
        .run()
        .assert_same();
}

#[test]
fn change_sibling() {
    Diff::new(entry!(0x0048_4bc0, tesform::change_sibling(u32, u32)))
        .change_call()
        .run()
        .assert_same();
}

#[test]
fn flags_save_size() {
    Diff::new(entry!(0x0048_4bf0, tesform::flags_save_size(u32, u32) -> u16))
        .forms()
        .inputs(&[Input::Of(&[0, 1, 2, 3, 0xffff_ffff])])
        .setup(|e, case| {
            world(e);
            vec![form(e, 0x20, 0, 0x14), case.word(0)]
        })
        .run()
        .assert_same();
}

#[test]
fn save_flags() {
    Diff::new(entry!(0x0048_4c20, tesform::save_flags(u32, u32)))
        .change_call()
        .run()
        .assert_same();
}

#[test]
fn load_game() {
    Diff::new(entry!(0x0048_4c50, tesform::load_game(u32, u32, u32)))
        .forms()
        .inputs(&[
            Input::Of(&[0, 1, 3]),
            Input::Flag,
            Input::Of(&[0, 0xffff_ffff, 0x5a5a_5a5a]),
            Input::Of(&[0, 0xffff_ffff]),
        ])
        .setup(|e, case| {
            world(e);
            let f = form(e, 0x20, case.word(3), 0x14);
            registries(e, false, 0, false, true, f);
            set_cast(
                e,
                TYPE_TES_OBJECT_REFR,
                if case.word(1) != 0 { f } else { 0 },
            );
            e.mem.set_u32(READ_WORD, case.word(2));
            vec![f, case.word(0), 0x77]
        })
        .run()
        .assert_same();
}

/// A forwarder's cases: two words, with the save game there.
fn forwarder_cases(d: Diff) -> Diff {
    d.forms()
        .inputs(&[Input::Of(WORDS), Input::Of(&[0, 4, 0xffff_ffff])])
        .setup(|e, case| {
            world(e);
            let f = form(e, 0x20, 0, 0x14);
            registries(e, false, 0, false, true, f);
            vec![f, case.word(0), case.word(1)]
        })
}

#[test]
fn save_game_data_old() {
    Diff::new(entry!(
        0x0048_4ce0,
        tesform::save_game_data_old(u32, u32, u32)
    ))
    .forwarder()
    .run()
    .assert_same();
}

#[test]
fn load_game_data_old() {
    // The double writes the read word at the buffer: give it a real one.
    Diff::new(entry!(
        0x0048_4d00,
        tesform::load_game_data_old(u32, u32, u32)
    ))
    .forms()
    .inputs(&[Input::Of(&[0, 4, 0xffff_ffff])])
    .setup(|e, case| {
        world(e);
        let f = form(e, 0x20, 0, 0x14);
        registries(e, false, 0, false, true, f);
        e.mem.set_u32(READ_WORD, 0x1357_9bdf);
        let buffer = e.mem.alloc(8);
        vec![f, buffer, case.word(0)]
    })
    .run()
    .assert_same();
}

#[test]
fn save_numeric_id() {
    Diff::new(entry!(0x0048_4d20, tesform::save_numeric_id(u32, u32, u32)))
        .forwarder()
        .run()
        .assert_same();
}

#[test]
fn load_numeric_id() {
    Diff::new(entry!(0x0048_4d40, tesform::load_numeric_id(u32, u32, u32)))
        .forwarder()
        .run()
        .assert_same();
}

#[test]
fn write_flags() {
    Diff::new(entry!(0x0048_4d60, tesform::write_flags(u32, u32)))
        .forms()
        .inputs(&[Input::Of(&[0, 1, 2, 3])])
        .setup(|e, case| {
            world(e);
            let buffer = e.mem.alloc(0x20);
            e.mem.set_u32(buffer + SAVE_KIND_AT, case.word(0));
            vec![form(e, 0x20, 0x1234_5678, 0x14), buffer]
        })
        .run()
        .assert_same();
}

#[test]
fn read_flags() {
    Diff::new(entry!(0x0048_4da0, tesform::read_flags(u32, u32)))
        .forms()
        .inputs(&[
            Input::Of(&[0, 1, 2]),
            Input::Flag,
            Input::Of(&[0, 0xffff_ffff, 0x5a5a_5a5a]),
            Input::Of(&[0, 0xffff_ffff]),
            // JIP LN's G45: off; on with no handler; on, its handler adding 0x400000 to the mask.
            Input::Of(&[0, 1, 2]),
        ])
        .setup(|e, case| {
            world(e);
            e.mem.set_u32(SLOT_ANSWERS + 0xf0, case.word(1));
            e.mem.set_u32(READ_WORD, case.word(2));
            if case.word(4) != 0 {
                e.set_jip_patches(engine_core::extend::jip_patches::FIXES);
            }
            if case.word(4) == 2 {
                e.on_site_for_test(0x0048_4e03, |_, a| Some(a[0] | 0x40_0000));
            }
            let buffer = e.mem.alloc(0x20);
            e.mem.set_u32(buffer + SAVE_KIND_AT, case.word(0));
            vec![form(e, 0x20, case.word(3), 0x14), buffer]
        })
        .run()
        .assert_same();
}

#[test]
fn has_low_id() {
    Diff::new(entry!(0x0048_4e40, tesform::has_low_id(u32) -> bool))
        .forms()
        .inputs(&[Input::Of(IDS)])
        .setup(|e, case| {
            world(e);
            vec![form(e, 0x20, 0, case.word(0))]
        })
        .run()
        .assert_same();
}

/// A form whose source file list has this shape: 0 empty; 1 one file; 2 a file, a node without
/// one, a file; 3 a node without a file, then three files; 4 two nodes without files.
fn form_with_files(e: &mut Engine, shape: u32) -> u32 {
    world(e);
    let f = form(e, 0x20, 0, 0x0100_0abc);
    let (a, b, c) = (file(e, true), file(e, false), file(e, true));
    match shape {
        1 => set_files(e, f, &[a]),
        2 => set_files(e, f, &[a, 0, b]),
        3 => set_files(e, f, &[0, a, b, c]),
        4 => set_files(e, f, &[0, 0]),
        _ => {}
    }
    f
}

#[test]
fn get_file() {
    Diff::new(entry!(0x0048_4e60, tesform::get_file(u32, i32) -> u32))
        .forms()
        .inputs(&[
            Input::Of(&[0, 1, 2, 3, 4]),
            Input::Of(&[0, 1, 2, 5, 0xffff_ffff, 0xffff_fffe]),
        ])
        .setup(|e, case| vec![form_with_files(e, case.word(0)), case.word(1)])
        .run()
        .assert_same();
}

#[test]
fn get_owner_master() {
    // No list shape with a node of no file before the list's end: the exe asks `GetMaster` of every
    // node's file, and of a null one it would read at 0x3e8.
    Diff::new(entry!(0x0048_4ee0, tesform::get_owner_master(u32) -> u32))
        .forms()
        .inputs(&[Input::Of(&[0, 1, 5])])
        .setup(|e, case| {
            if case.word(0) == 5 {
                // Masters and other files mixed: master, file, master, file.
                world(e);
                let f = form(e, 0x20, 0, 1);
                let files = [file(e, true), file(e, false), file(e, true), file(e, false)];
                set_files(e, f, &files);
                return vec![f];
            }
            vec![form_with_files(e, case.word(0))]
        })
        .run()
        .assert_same();
}

#[test]
fn set_file() {
    // The file: 0 none; 1 a master; 2 another file; 3 a file the list holds; 4 a file whose
    // chain ends in a master.
    Diff::new(entry!(0x0048_4f50, tesform::set_file(u32, u32)))
        .forms()
        .inputs(&[Input::Of(&[0, 1, 2, 3, 4]), Input::Of(&[0, 1, 2, 3, 4, 5])])
        .setup(|e, case| {
            world(e);
            let f = form(e, 0x20, 0, 1);
            let (m1, n1, m2, n2) = (file(e, true), file(e, false), file(e, true), file(e, false));
            match case.word(1) {
                1 => set_files(e, f, &[m1]),
                2 => set_files(e, f, &[n1, m1]),
                3 => set_files(e, f, &[m1, n1, m2, n2]),
                4 => set_files(e, f, &[n1, 0]),
                5 => set_files(e, f, &[n1, n2, m1]),
                _ => {}
            }
            let given = match case.word(0) {
                0 => 0,
                1 => file(e, true),
                2 => file(e, false),
                3 => n1,
                _ => {
                    let root = file(e, true);
                    let given = file(e, false);
                    e.mem.set_u32(given + FILE_PARENT, root);
                    given
                }
            };
            vec![f, given]
        })
        .run()
        .assert_same();
}

#[test]
fn load_form() {
    Diff::new(entry!(0x0048_5110, tesform::load_form(u32, u32)))
        .forms()
        .inputs(&[Input::Of(WORDS)])
        .setup(|e, case| {
            world(e);
            vec![form(e, 0x20, 0, 1), case.word(0)]
        })
        .run()
        .assert_same();
}

#[test]
fn copy() {
    Diff::new(entry!(0x0048_5130, tesform::copy(u32, u32)))
        .forms()
        .inputs(&[Input::Of(&[0, 0x0101_1584])])
        .setup(|e, case| {
            world(e);
            e.mem.set_u32(SLOT_ANSWERS + 0x130, case.word(0));
            let other = form(e, 0x30, 0, 2);
            vec![form(e, 0x20, 0, 1), other]
        })
        .run()
        .assert_same();
}

#[test]
fn compare() {
    Diff::new(entry!(0x0048_5170, tesform::compare(u32, u32) -> bool))
        .forms()
        .inputs(&[Input::Of(&[0, 0x0101_1584])])
        .setup(|e, case| {
            world(e);
            e.mem.set_u32(SLOT_ANSWERS + 0x130, case.word(0));
            let other = form(e, 0x30, 0, 2);
            vec![form(e, 0x20, 0, 1), other]
        })
        .run()
        .assert_same();
}

#[test]
fn copy_all_components() {
    Diff::new(entry!(0x0048_51b0, tesform::copy_all_components(u32, u32)))
        .forms()
        .inputs(&[Input::Of(&[0, 1])])
        .setup(|e, case| {
            world(e);
            let other = if case.word(0) != 0 {
                form(e, 0x30, 0, 2)
            } else {
                0
            };
            vec![form(e, 0x20, 0, 1), other]
        })
        .run()
        .assert_same();
}

#[test]
fn compare_all_components() {
    Diff::new(entry!(
        0x0048_5270,
        tesform::compare_all_components(u32, u32) -> u8
    ))
    .forms()
    .inputs(&[Input::Flag, Input::Of(&[0, 1, 0x101])])
    .setup(|e, case| {
        world(e);
        set_answer(e, COMPONENTS_COMPARE, case.word(1));
        let other = if case.word(0) != 0 {
            form(e, 0x30, 0, 2)
        } else {
            0
        };
        vec![form(e, 0x20, 0, 1), other]
    })
    .run()
    .assert_same();
}

#[test]
fn copy_from() {
    Diff::new(entry!(0x0048_5340, tesform::copy_from(u32, u32)))
        .forms()
        .inputs(&[
            Input::Flag,
            Input::Of(&[0, 2, 0x4000, 0x4002]),
            Input::Of(&[0, 2, 0xffff_ffff]),
        ])
        .setup(|e, case| {
            world(e);
            let source = form(e, 0x31, case.word(2), 2);
            set_cast(e, TYPE_TES_FORM, if case.word(0) != 0 { source } else { 0 });
            let component = source + 0x18;
            vec![form(e, 0x20, case.word(1), 1), component]
        })
        .run()
        .assert_same();
}

#[test]
fn differs_from() {
    // The keys: 0 neither has one; 1 both the same; 2 they differ; 3 only the other has one.
    // The flags: 0 the same; 1 apart only in 0x4000 and 0xb; 2 apart elsewhere.
    Diff::new(entry!(0x0048_53e0, tesform::differs_from(u32, u32) -> bool))
        .forms()
        .inputs(&[
            Input::Flag,
            Input::Flag,
            Input::Of(&[0, 1, 2, 3]),
            Input::Flag,
            Input::Of(&[0, 1, 2]),
        ])
        .setup(|e, case| {
            world(e);
            let flags = 0x0000_1230;
            let theirs = match case.word(4) {
                0 => flags,
                1 => flags ^ 0x400b,
                _ => flags ^ 0x0010_0000,
            };
            let this = form(e, 0x20, flags, 1);
            let kind = if case.word(1) != 0 { 0x20 } else { 0x21 };
            let source = form(e, kind, theirs, 2);
            let (mine_key, their_key) = match case.word(2) {
                0 => (0, 0),
                1 => (7, 7),
                2 => (7, 8),
                _ => (0, 9),
            };
            e.mem.set_u32(this + 0x30, mine_key);
            e.mem.set_u32(source + 0x30, their_key);
            set_answer(e, STRING_COMPARE, case.word(3));
            set_cast(e, TYPE_TES_FORM, if case.word(0) != 0 { source } else { 0 });
            vec![this, source + 0x18]
        })
        .run()
        .assert_same();
}

/// A record for the top group tests: 0 none; 1 not a group; 2 a group of type 1; 3 a top group
/// labelled with the form's type; 4 a top group labelled with another.
fn top_record(e: &mut Engine, kind: u32, form_type: u32) -> u32 {
    if kind == 0 {
        return 0;
    }
    let r = e.mem.alloc(0x18);
    e.mem.set_u32(
        r,
        if kind == 1 {
            0x1234_5678
        } else {
            GROUP_TAG_VALUE
        },
    );
    e.mem.set_u32(r + 0xc, u32::from(kind == 2));
    let label = if kind == 4 { form_type + 1 } else { form_type };
    e.mem.set_u32(r + 8, 0x4d52_0000 | label);
    r
}

#[test]
fn is_own_top_group() {
    Diff::new(entry!(
        0x0048_54e0,
        tesform::is_own_top_group(u32, u32, u32, u32) -> bool
    ))
    .forms()
    .inputs(&[Input::Of(&[0, 1, 2, 3, 4]), Input::Of(&[0x20, 0x45])])
    .setup(|e, case| {
        world(e);
        let f = form(e, case.word(1), 0, 1);
        let r = top_record(e, case.word(0), case.word(1));
        vec![f, r, 5, 6]
    })
    .run()
    .assert_same();
}

#[test]
fn fill_top_group() {
    Diff::new(entry!(0x0048_5530, tesform::fill_top_group(u32, u32, u32)))
        .forms()
        .inputs(&[Input::Flag, Input::Of(&[0, 1, 0x100])])
        .setup(|e, case| {
            world(e);
            let f = form(e, 0x2a, 0, 1);
            let r = if case.word(0) != 0 {
                let r = e.mem.alloc(0x18);
                for i in 0..0x18 {
                    e.mem.set_u8(r + i, 0xcc);
                }
                r
            } else {
                0
            };
            vec![f, r, case.word(1)]
        })
        .run()
        .assert_same();
}

#[test]
fn start_form() {
    Diff::new(entry!(0x0048_55a0, tesform::start_form(u32)))
        .forms()
        .inputs(&[
            Input::Of(&[0, 0x4000, 0xffff_bfff]),
            Input::Of(&[1, 2, 0x29]),
        ])
        .setup(|e, case| {
            world(e);
            vec![form(e, case.word(1), case.word(0), 0x0100_0abc)]
        })
        .run()
        .assert_same();
}

#[test]
fn close_form() {
    Diff::new(entry!(0x0048_5680, tesform::close_form(u32)))
        .forms()
        .inputs(&[Input::Of(TEMPORARY), Input::Flag])
        .setup(|e, case| {
            world(e);
            record_buffer(e);
            e.mem.set_u32(SAVE_BUFFER_SIZE, 0x40);
            e.mem.set_u8(BIG_ENDIAN, case.word(1) as u8);
            vec![form(e, 0x20, case.word(0), 1)]
        })
        .run()
        .assert_same();
}

/// A chunk writer's cases: big-endian or not, with the record buffer started.
fn chunk_writer_cases(d: Diff, values: &'static [u32]) -> Diff {
    d.forms()
        .inputs(&[Input::Flag, Input::Of(values)])
        .setup(|e, case| {
            world(e);
            record_buffer(e);
            e.mem.set_u8(BIG_ENDIAN, case.word(0) as u8);
            let n = case.word(1);
            vec![0x4c4c_5546, data(e, 4 * n.max(1)), n]
        })
}

const COUNTS: &[u32] = &[0, 1, 3];

#[test]
fn add_chunk() {
    Diff::new(entry!(0x0048_56d0, tesform::add_chunk(u32)))
        .forms()
        .inputs(&[Input::Flag])
        .setup(|e, case| {
            world(e);
            record_buffer(e);
            e.mem.set_u8(BIG_ENDIAN, case.word(0) as u8);
            vec![0x4c4c_5546]
        })
        .run()
        .assert_same();
}

#[test]
fn add_chunk_array() {
    Diff::new(entry!(0x0048_56f0, tesform::add_chunk_array(u32, u32, u32)))
        .chunk_writer(&[0, 1, 6, 0x100])
        .run()
        .assert_same();
}

#[test]
fn add_chunk_array_words() {
    Diff::new(entry!(
        0x0048_5710,
        tesform::add_chunk_array_words(u32, u32, u32)
    ))
    .chunk_writer(COUNTS)
    .run()
    .assert_same();
}

#[test]
fn add_chunk_array_halves() {
    Diff::new(entry!(
        0x0048_5730,
        tesform::add_chunk_array_halves(u32, u32, u32)
    ))
    .chunk_writer(COUNTS)
    .run()
    .assert_same();
}

#[test]
fn add_chunk_array32() {
    Diff::new(entry!(
        0x0048_5750,
        tesform::add_chunk_array32(u32, u32, u32)
    ))
    .chunk_writer(COUNTS)
    .run()
    .assert_same();
}

#[test]
fn add_chunk_array16() {
    Diff::new(entry!(
        0x0048_5820,
        tesform::add_chunk_array16(u32, u32, u32)
    ))
    .chunk_writer(COUNTS)
    .run()
    .assert_same();
}

/// A one-value chunk's cases: big-endian or not, the value.
fn value_chunk_cases(d: Diff) -> Diff {
    d.forms()
        .inputs(&[Input::Flag, Input::Of(&[0, 0x1234_5678, 0xffff_ffff])])
        .setup(|e, case| {
            world(e);
            record_buffer(e);
            e.mem.set_u8(BIG_ENDIAN, case.word(0) as u8);
            vec![0x4c4c_5546, case.word(1)]
        })
}

#[test]
fn add_chunk_byte() {
    Diff::new(entry!(0x0048_58f0, tesform::add_chunk_byte(u32, u32)))
        .value_chunk()
        .run()
        .assert_same();
}

#[test]
fn add_chunk_word() {
    Diff::new(entry!(0x0048_5910, tesform::add_chunk_word(u32, u32)))
        .value_chunk()
        .run()
        .assert_same();
}

#[test]
fn add_chunk_half() {
    Diff::new(entry!(0x0048_5950, tesform::add_chunk_half(u32, u32)))
        .value_chunk()
        .run()
        .assert_same();
}

#[test]
fn add_chunk_data() {
    Diff::new(entry!(0x0048_5990, tesform::add_chunk_data(u32, u32, u32)))
        .forms()
        .inputs(&[Input::Flag, Input::Of(&[0, 1, 6, 0xffff, 0x1_0000])])
        .setup(|e, case| {
            world(e);
            record_buffer(e);
            e.mem.set_u8(BIG_ENDIAN, case.word(0) as u8);
            let size = case.word(1);
            vec![0x4c4c_5546, data(e, size), size]
        })
        .run()
        .assert_same();
}

#[test]
fn grow_chunk() {
    Diff::new(entry!(0x0048_5a70, tesform::grow_chunk(u32, u32, u16) -> bool))
        .forms()
        .inputs(&[
            Input::Flag,
            Input::Of(&[0, 1, 0xfff0, 0xffff]),
            Input::Of(&[0, 1, 0x10, 0xffff]),
        ])
        .setup(|e, case| {
            world(e);
            record_buffer(e);
            e.mem.set_u8(BIG_ENDIAN, case.word(0) as u8);
            let chunk = e.mem.alloc(8);
            e.mem.set_u32(chunk, 0x4c4c_5546);
            e.mem.set_u16(chunk + 4, case.word(1) as u16);
            vec![form(e, 0x20, 0, 1), chunk, case.word(2)]
        })
        .run()
        .assert_same();
}

#[test]
fn free_form_buffer() {
    Diff::new(entry!(0x0048_5b30, tesform::free_form_buffer(u32)))
        .forms()
        .inputs(&[Input::Flag])
        .setup(|e, case| {
            world(e);
            if case.word(0) != 0 {
                record_buffer(e);
            }
            vec![form(e, 0x20, 0, 1)]
        })
        .run()
        .assert_same();
}

#[test]
fn packed_id() {
    Diff::new(entry!(0x0048_5b60, tesform::packed_id(u32) -> u32))
        .forms()
        // Shapes with a file (the exe reads the first file's master list, at 0x3ec of a null one).
        .inputs(&[Input::Of(&[1, 2, 3]), Input::Flag])
        .setup(|e, case| {
            let f = form_with_files(e, case.word(0));
            // The first file's masters (its list at +0x3ec): empty, or holding a master.
            let first = match e.mem.u32(f + 0x10) {
                0 => e.mem.u32(e.mem.u32(f + 0x14)),
                file => file,
            };
            if case.word(1) != 0 {
                let m = file(e, true);
                e.mem.set_u32(first + FILE_MASTERS, m);
            }
            vec![f]
        })
        .run()
        .assert_same();
}

#[test]
fn low_id() {
    Diff::new(entry!(0x0048_5bc0, tesform::low_id(u32) -> u32))
        .forms()
        .inputs(&[Input::Of(IDS)])
        .setup(|e, case| {
            world(e);
            vec![form(e, 0x20, 0, case.word(0))]
        })
        .run()
        .assert_same();
}

#[test]
fn same_low_id() {
    Diff::new(entry!(0x0048_5be0, tesform::same_low_id(u32, u32) -> bool))
        .forms()
        .inputs(&[Input::Of(IDS), Input::Of(&[1, 0x0200_0001, 0x800])])
        .setup(|e, case| {
            world(e);
            vec![form(e, 0x20, 0, case.word(0)), case.word(1)]
        })
        .run()
        .assert_same();
}

#[test]
fn set_form_id() {
    // The new id: 0 the same as the old; 1 another; 2 none.
    Diff::new(entry!(0x0048_5c10, tesform::set_form_id(u32, u32, bool)))
        .forms()
        .inputs(&[
            Input::Of(&[0, 1, 2]),
            Input::Of(TEMPORARY),
            Input::Of(&[0, 0x0100_0abc]),
            Input::Flag,
        ])
        .setup(|e, case| {
            world(e);
            let old = case.word(2);
            let f = form(e, 0x20, case.word(1), old);
            registries(e, true, 0, true, false, f);
            let new = match case.word(0) {
                0 => old,
                1 => 0x0200_0def,
                _ => 0,
            };
            vec![f, new, case.word(3)]
        })
        .run()
        .assert_same();
}

#[test]
fn add_compile_index() {
    Diff::new(entry!(0x0048_5d50, tesform::add_compile_index(u32, u32)))
        .forms()
        .inputs(&[
            Input::Of(&[0x5, 0x0100_0abc, 0xff00_0001]),
            Input::Flag,
            Input::Flag,
        ])
        .setup(|e, case| {
            world(e);
            registries(e, false, 0, false, true, 0);
            let id = e.mem.alloc(4);
            e.mem.set_u32(id, case.word(0));
            // The given file's compile index 4, the indexed one's 0x84.
            let f = if case.word(1) != 0 { file(e, false) } else { 0 };
            if f != 0 {
                e.mem.set_u8(f + FILE_COMPILE_INDEX_AT, 4);
            }
            let indexed = if case.word(2) != 0 { file(e, true) } else { 0 };
            if indexed != 0 {
                e.mem.set_u8(indexed + FILE_COMPILE_INDEX_AT, 0x84);
            }
            set_answer(e, FILE_GET_INDEX_FILE, indexed);
            vec![id, f]
        })
        .run()
        .assert_same();
}

/// The form-type table as the exe holds it before the constructor packs it: each entry's type
/// byte and its four characters (`table`: 0 good; 1 one entry out of order and two with the same
/// string).
fn fresh_form_table(e: &mut Engine, table: u32) {
    for t in 0..0x79u32 {
        let entry = FORM_ENUM_TABLE + 12 * t;
        let text = e.mem.alloc(8);
        let chars = [b'A' + (t % 26) as u8, b'a' + (t / 26) as u8, 0xc3, b'Z'];
        for (i, c) in chars.into_iter().enumerate() {
            e.mem.set_u8(text + i as u32, c);
        }
        e.mem.set_u8(entry, t as u8);
        e.mem.set_u32(entry + 4, text);
        e.mem.set_u32(entry + 8, 0);
    }
    if table == 1 {
        e.mem.set_u8(FORM_ENUM_TABLE + 12 * 3, 4);
        let first = e.mem.u32(FORM_ENUM_TABLE + 12 * 4 + 4);
        e.mem.set_u32(FORM_ENUM_TABLE + 12 * 5 + 4, first);
    }
}

#[test]
fn construct() {
    // The table: 0 packed already; 1 fresh; 2 fresh with faults. The handler: 0 none; 1 its flag
    // set; 2 a low next id and no file; 3 a high next id and its loading file.
    Diff::new(entry!(0x0048_3370, tesform::construct(u32) -> u32))
        .forms()
        .inputs(&[
            Input::Of(&[0, 1, 2]),
            Input::Of(&[0, 1, 2, 3]),
            Input::Of(&[0, 5]),
        ])
        .setup(|e, case| {
            world(e);
            match case.word(0) {
                0 => e.mem.set_u8(0x011c_54b8, 1),
                table => fresh_form_table(e, table - 1),
            }
            let f = e.mem.alloc(0x40);
            e.mem.set_u32(FORM_COUNT, case.word(2));
            let handler = case.word(1);
            if handler != 0 {
                registries(e, false, 0, true, false, f);
                let h = e.mem.u32(DATA_HANDLER);
                e.mem.set_u8(h + HANDLER_NO_IDS, u8::from(handler == 1));
                set_answer(
                    e,
                    0x0046_9800,
                    if handler == 2 { 0x10 } else { 0x0100_0abc },
                );
                if handler == 3 {
                    let loading = file(e, false);
                    e.mem.set_u32(h + 0x20c, loading);
                }
                let m = form(e, 0x7f, 0, 0);
                e.mem.set_u32(FORM_MAP, m);
            }
            vec![f]
        })
        .run()
        .assert_same();
}

#[test]
fn compress_save_buffer() {
    // The buffer: 0 none; 1 a header only; 2 already compressed; 3 a body to pack.
    Diff::new(entry!(0x0048_3d70, tesform::compress_save_buffer()))
        .forms()
        .inputs(&[
            Input::Of(&[0, 1, 2, 3]),
            Input::Flag,
            Input::Flag,
            Input::Of(&[0, 0xffff_fffe]),
        ])
        .setup(|e, case| {
            world(e);
            match case.word(0) {
                0 => {}
                kind => {
                    let size = if kind == 1 { 0x18 } else { 0x40 };
                    let buffer = data(e, size);
                    let flags = if kind == 2 { 0x0004_0000 } else { 0x0000_0800 };
                    e.mem.set_u32(buffer + 8, flags);
                    e.mem.set_u32(SAVE_BUFFER, buffer);
                    e.mem.set_u32(SAVE_BUFFER_SIZE, size);
                }
            }
            e.mem.set_u8(BIG_ENDIAN, case.word(1) as u8);
            set_answer(e, 0x00b4_6360, case.word(2));
            e.mem.set_u32(DEFLATED, 0x1c);
            e.mem.set_u32(DEFLATE_STATUS, case.word(3));
            vec![]
        })
        .run()
        .assert_same();
}

/// The form's components, by bit of `mask` in [`COMPONENT_TYPES`]' order, each a small object
/// with the test vtable, its byte at +4 and its word at +4 the getters read: the weight's float a
/// signalling NaN (its load and store bring it back quiet), the quality's byte 0xc8.
fn components(e: &mut Engine, mask: u32) {
    for (i, target) in COMPONENT_TYPES.into_iter().enumerate() {
        let part = if mask & (1 << i) != 0 {
            let p = e.mem.alloc(0x10);
            e.mem.set_u32(p, VTABLE);
            let word = match i {
                3 => 0x7f80_0001,
                4 => 0xc8,
                _ => 0x0102_0300 + i as u32,
            };
            e.mem.set_u32(p + 4, word);
            p
        } else {
            0
        };
        set_cast(e, target, part);
    }
    set_answer(e, 0x0048_0100, 3);
    set_answer(e, 0x0048_0130, 2);
    e.mem.set_u32(SLOT_ANSWERS + 0x10, 0x1234);
}

#[test]
fn save_data() {
    Diff::new(entry!(0x0048_5e00, tesform::save_data(u32)))
        .forms()
        .inputs(&[Input::Flag, Input::Of(&[0, 0xff])])
        .setup(|e, case| {
            world(e);
            record_buffer(e);
            e.mem.set_u8(BIG_ENDIAN, case.word(0) as u8);
            components(e, case.word(1));
            vec![form(e, 0x28, 0, 1)]
        })
        .run()
        .assert_same();
}

#[test]
fn save_data_chunk() {
    Diff::new(entry!(0x0048_5e20, tesform::save_data_chunk(u32, u32, u32)))
        .forms()
        .inputs(&[
            Input::Of(&[0, 3, 0x1_0000]),
            Input::Of(&[0, 0xff, 0x41, 0x18, 0x24]),
            Input::Flag,
        ])
        .setup(|e, case| {
            world(e);
            record_buffer(e);
            e.mem.set_u8(BIG_ENDIAN, case.word(2) as u8);
            components(e, case.word(1));
            let size = case.word(0);
            let d = data(e, size.min(8));
            vec![form(e, 0x28, 0, 1), d, size]
        })
        .run()
        .assert_same();
}

#[test]
fn load_data() {
    Diff::new(entry!(0x0048_61f0, tesform::load_data(u32, u32, u32, u16)))
        .forms()
        .inputs(&[
            Input::Of(&[0, 4, 0x20]),
            Input::Of(&[0, 2, 0x30]),
            Input::Of(&[0, 0xff, 0x05, 0x58]),
            Input::Flag,
            // The weight's float in the chunk: the pattern's; a signalling NaN in the file's byte
            // order (quiet before a byte-swapped file's swap); one a swap makes a signalling NaN
            // (made quiet by the weight's store).
            Input::Of(&[0, 0x7f80_0001, 0x0100_807f]),
        ])
        .setup(|e, case| {
            world(e);
            let mask = case.word(2);
            components(e, mask);
            if mask & 8 != 0 {
                let at = case.word(1) + (mask & 1) + 4 * (mask >> 1 & 1) + 4 * (mask >> 2 & 1);
                e.mem.set_u32(CHUNK_FLOAT, case.word(4));
                e.mem.set_u32(CHUNK_FLOAT_AT, at);
            }
            // The file's current chunk: its size and whether it is byte-swapped.
            let file = file(e, false);
            e.mem.set_u32(file + FILE_CHUNK_SIZE_AT, case.word(0));
            e.mem.set_u8(file + FILE_SWAPPED_AT, case.word(3) as u8);
            let destination = e.mem.alloc(0x40);
            vec![form(e, 0x28, 0, 1), file, destination, case.word(1)]
        })
        .run()
        .assert_same();
}

#[test]
fn smaller_half() {
    Diff::new(entry!(0x0048_65f0, tesform::smaller_half(u16, u16) -> u16))
        .forms()
        .inputs(&[
            Input::Of(&[0, 1, 0xffff, 0x1_0002]),
            Input::Of(&[0, 2, 0xfffe, 0x8000_0001]),
        ])
        .setup(|e, case| {
            world(e);
            vec![case.word(0), case.word(1)]
        })
        .run()
        .assert_same();
}

#[test]
fn components_data_size() {
    Diff::new(entry!(0x0048_6620, tesform::components_data_size(u32) -> u32))
        .forms()
        .inputs(&[Input::Of(&[0, 0xff, 0x40, 0x81, 0x3e])])
        .setup(|e, case| {
            world(e);
            components(e, case.word(0));
            vec![form(e, 0x28, 0, 1)]
        })
        .run()
        .assert_same();
}

#[test]
fn copy_form() {
    Diff::new(entry!(0x0048_67a0, tesform::copy_form(u32, u32, u32) -> u32))
        .forms()
        // The name local's text after the copy: the string's (the double keeps the pointer) or none.
        .inputs(&[Input::Flag, Input::Flag, Input::Flag])
        .setup(|e, case| {
            world(e);
            let created = if case.word(0) != 0 {
                form(e, 0x28, 0, 2)
            } else {
                0
            };
            set_answer(e, 0x0046_5110, created);
            e.mem.set_u32(STRING_EMPTY, case.word(1));
            let map = if case.word(2) != 0 {
                form(e, 0x7f, 0, 0)
            } else {
                0
            };
            vec![form(e, 0x28, 0, 1), 9, map]
        })
        .run()
        .assert_same();
}

#[test]
fn form_type_from_string() {
    Diff::new(entry!(0x0048_6890, tesform::form_type_from_string(u32) -> u32))
        .forms()
        .inputs(&[Input::Of(&[0x1234, 0x4d52_0005, 0x4d52_0078, 0x9999, 0])])
        .setup(|e, case| {
            world(e);
            e.mem.set_u32(0x011c_54f4, 0x1234);
            e.mem.set_u32(0x011c_54f0, 7);
            vec![case.word(0)]
        })
        .run()
        .assert_same();
}

#[test]
fn detailed_string() {
    Diff::new(entry!(0x0048_68f0, tesform::detailed_string(u32, u32)))
        .forms()
        // JohnnyGuitar NVSE's seam at the name's call: no handler (the exe's empty name), or one that
        // answers a name.
        .inputs(&[Input::Flag])
        .setup(|e, case| {
            world(e);
            if case.word(0) != 0 {
                e.on_site_for_test(0x0048_6903, |_, _| Some(0x0101_2345));
            }
            let buffer = e.mem.alloc(0x40);
            vec![form(e, 0x28, 0, 0x0100_0abc), buffer]
        })
        .run()
        .assert_same();
}

/// A map destructor's cases: the release bit and others in `flags`.
fn map_object_cases(d: Diff) -> Diff {
    d.forms()
        .inputs(&[Input::Of(&[0, 1, 2, 3])])
        .setup(|e, case| {
            world(e);
            let m = form(e, 0x7f, 0, 0);
            vec![m, case.word(0)]
        })
}

#[test]
fn form_map_construct() {
    Diff::new(entry!(0x0048_6930, tesform::form_map_construct(u32, u32) -> u32))
        .forms()
        .inputs(&[Input::Of(&[0, 1, 0x2008d])])
        .setup(|e, case| {
            world(e);
            vec![e.mem.alloc(0x10), case.word(0)]
        })
        .run()
        .assert_same();
}

#[test]
fn form_map_scalar_deleting_destructor() {
    Diff::new(entry!(
        0x0048_6960,
        tesform::form_map_scalar_deleting_destructor(u32, u32) -> u32
    ))
    .map_object()
    .run()
    .assert_same();
}

#[test]
fn map_base_construct() {
    Diff::new(entry!(0x0048_6990, tesform::map_base_construct(u32, u32) -> u32))
        .forms()
        .inputs(&[Input::Of(&[0, 1, 0x2008d, 0x4000_0001])])
        .setup(|e, case| {
            world(e);
            vec![e.mem.alloc(0x10), case.word(0)]
        })
        .run()
        .assert_same();
}

#[test]
fn form_map_destructor() {
    Diff::new(entry!(0x0048_6a00, tesform::form_map_destructor(u32)))
        .forms()
        .inputs(&[Input::Of(&[0])])
        .setup(|e, _| {
            world(e);
            vec![form(e, 0x7f, 0, 0)]
        })
        .run()
        .assert_same();
}

#[test]
fn map_base_scalar_deleting_destructor() {
    Diff::new(entry!(
        0x0048_6c00,
        tesform::map_base_scalar_deleting_destructor(u32, u32) -> u32
    ))
    .map_object()
    .run()
    .assert_same();
}

#[test]
fn string_map_scalar_deleting_destructor() {
    Diff::new(entry!(
        0x0048_6c30,
        tesform::string_map_scalar_deleting_destructor(u32, u32) -> u32
    ))
    .map_object()
    .run()
    .assert_same();
}

#[test]
fn string_map_destructor() {
    Diff::new(entry!(0x0048_6c60, tesform::string_map_destructor(u32)))
        .forms()
        .inputs(&[Input::Of(&[0])])
        .setup(|e, _| {
            world(e);
            vec![form(e, 0x7f, 0, 0)]
        })
        .run()
        .assert_same();
}

#[test]
fn char_key_map_base_scalar_deleting_destructor() {
    Diff::new(entry!(
        0x0048_6e50,
        tesform::char_key_map_base_scalar_deleting_destructor(u32, u32) -> u32
    ))
    .map_object()
    .run()
    .assert_same();
}

#[test]
fn char_key_map_scalar_deleting_destructor() {
    Diff::new(entry!(
        0x0048_6e80,
        tesform::char_key_map_scalar_deleting_destructor(u32, u32) -> u32
    ))
    .map_object()
    .run()
    .assert_same();
}

#[test]
fn string_template_map_scalar_deleting_destructor() {
    Diff::new(entry!(
        0x0048_6eb0,
        tesform::string_template_map_scalar_deleting_destructor(u32, u32) -> u32
    ))
    .map_object()
    .run()
    .assert_same();
}
