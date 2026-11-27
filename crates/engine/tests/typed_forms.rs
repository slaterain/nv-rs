//! The differential test of every typed function of the forms family (`engine_typed::forms`)
//! against its translation: `TESForm`'s own functions (`tesform`) and `TESActorBaseData`'s
//! (`actor_base_data`, whose world adds the settings, the save/load game and the template walk),
//! `TESActorBase`'s and `TESAttributes`' (`actor_base`, `attributes`, whose world adds actor bases
//! with their components), `TESAIForm`'s and `TESAnimation`'s (`ai_form`, `animation`, whose
//! world adds the package list's answers, the copied settings and a chunk of KF file names), and
//! `TESObject`'s unit (`object`, `debug_string`, whose world adds the unit's globals, strings, bound
//! objects and the model loader's and scene graph's answers).
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
use engine_typed::forms::{
    actor_base, actor_base_data, ai_form, animation, attributes, conditions, debug_string, object,
    package_data, package_list, schedule, tesform, FORMS,
};
use engine_typed::pathing::PATHING;

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
const ANSWERED: [u32; 88] = [
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
    0x0046_5110, // CreateFormOfType
    0x0040_37d0, // a string local's destructor
    0x0040_6f60, // sprintf
    0x0048_6a60, // the map base's destruction
    0x0048_6c80, // the string map base's
    0x0048_6dc0, // the char key map base's
    0x0048_6d60, // NiTMap's
    0x0048_6ca0, // the string template map's
    0x0041_8520, // the actor base data's callee
    0x0041_aeb0, // the actor base data's callee
    0x0041_af50, // the actor base data's callee
    0x0041_21b0, // the actor base data's callee
    0x0046_23f0, // the actor base data's callee
    0x0048_1680, // the actor base data's callee
    0x00ec_658f, // the actor base data's callee
    0x0086_2110, // the actor base data's callee
    0x0056_afc0, // the actor base data's callee
    0x0048_8350, // the actor base data's callee
    0x0088_4f80, // the actor base data's callee
    0x0056_b020, // the actor base data's callee
    0x0057_5690, // the actor base data's callee
    0x0056_5730, // the actor base data's callee
    0x004b_f220, // the actor base data's callee
    0x004d_1440, // the actor base data's callee
    0x004d_1610, // the actor base data's callee
    0x004d_1960, // the actor base data's callee
    0x006e_b820, // the actor base data's callee
    0x006e_bd10, // the actor base data's callee
    0x0087_f9f0, // the actor base data's callee
    0x0084_a6d0, // the actor base data's callee
    0x0084_a620, // the save/load game's change test
    0x0084_a740, // and its form-changed test
    0x0048_73d0, // GetFormHealth
    0x0047_ea30, // the actor base data's buffer save
    0x0047_ea70, // and load
    0x0048_e040, // the spell list's buffer save
    0x0048_e140, // and load
    0x0048_d8f0, // the spell list's save size
    0x0048_da20, // the spell list's save
    0x0048_dca0, // the spell list's load
    0x0086_5e70, // SaveString
    0x0048_9100, // the full name's setter
    0x0096_bbc0, // the iterator's walk
    0x00ec_782f, // the vector constructor iterator
    0x00ec_5fce, // and destructor iterator
    0x0048_8f50, // TESModelRDT's base constructor
    0x0048_04d0, // TESBipedModelForm::InitializeDataComponent
    0x0048_0570, // and ClearDataComponent
    0x0051_b950, // TESModelRDT's destructor
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

// ---- TESActorBaseData (`actor_base_data`) ----

/// The buffer the save/load game's position (+0x14) points into: what the save writes and the
/// load reads.
const STREAM: u32 = 0x0300_6000;
/// The save/load game's buffer position (`00825c00`), current save and load entries (`004fd3e0`,
/// `004fd3c0`) and version byte (`008df040`).
const SL_POSITION: u32 = 0x14;
const SL_SAVE_ENTRY: u32 = 0x88;
const SL_LOAD_ENTRY: u32 = 0x84;
const SL_VERSION: u32 = 0x80;

/// The one-line bodies the actor base data's native code does in place (see `actor_base_data`'s
/// module comment): left out of the translation's calls, with no double, as [`IN_PLACE`].
const ABD_IN_PLACE: &[u32] = &[
    0x0046_1580, // the base flags' test
    0x0046_1560, // and its 0x80 one
    0x005d_8a70, // the faction list's head
    0x0059_bb30, // the template form
    0x0044_1110, // the death item
    0x008f_21d0, // the speed multiplier
    0x0050_f9a0, // the death item's setter
    0x0050_f9c0, // the voice type's
    0x0070_37c0, // the template form's
    0x0071_7e50, // the address +4
    0x005d_43c0, // the address +0x44
    0x0040_3e20, // a float setting's value pointer
    0x0043_d4d0, // an integer setting's
    0x0040_8d60, // a byte setting's
    0x0040_3df0, // a string setting's string
    0x0082_5c00, // the save/load game's buffer position
    0x004f_d3e0, // its current save entry
    0x004f_d3c0, // and load entry
    0x008d_f040, // its version byte
    0x0042_cde0, // the inventory changes' empty-list test
];
/// The pages of the settings, the karma tables and the constants the component reads.
const ACTOR_PAGES: [u32; 5] = [
    0x011c_4000,
    0x0118_6000,
    0x0101_7000,
    0x0101_1000,
    0x0101_b000,
];

/// The world plus what the actor base data reaches: the settings' value pointers, the faction
/// list's head and count, the save buffer's stream, the template walk's callees.
fn actor_world(e: &mut Engine) {
    world(e);
    for page in ACTOR_PAGES {
        e.map(page, 0x1000);
    }
    answered!(
        e,
        0x0041_8520,
        0x0041_aeb0,
        0x0041_af50,
        0x0041_21b0,
        0x0046_23f0,
        0x0048_1680,
        0x00ec_658f,
        0x0086_2110,
        0x0056_afc0,
        0x0048_8350,
        0x0088_4f80,
        0x0056_b020,
        0x0057_5690,
        0x0056_5730,
        0x004b_f220,
        0x004d_1440,
        0x004d_1610,
        0x004d_1960,
        0x006e_b820,
        0x006e_bd10,
        0x0087_f9f0,
        0x0084_a6d0,
    );
    e.register(0x0040_eb10, |_, a| {
        let (x, y, tolerance) = (
            f32::from_bits(a[0]),
            f32::from_bits(a[1]),
            f32::from_bits(a[2]),
        );
        ret(u32::from((x - y).abs() <= tolerance))
    });
    e.register(0x005a_e380, |e, a| {
        let (mut node, mut count) = (a[0], 0);
        while node != 0 {
            if e.mem.u32(node) != 0 {
                count += 1;
            }
            node = e.mem.u32(node + 4);
        }
        ret(count)
    });
    e.register(0x0048_1610, |e, a| {
        let entry = e.mem.alloc(8);
        e.mem.set_u32(entry + 4, 0x77);
        e.mem.set_u32(a[0] + 4, entry);
        ret(a[0])
    });
    // The save/load game's byte doubles move its buffer position (+0x14) over the stream.
    fn advance(e: &mut Engine, from: u32, n: u32, to_stream: bool) {
        let save_load = e.mem.u32(SAVE_LOAD_GAME);
        let at = e.mem.u32(save_load + SL_POSITION);
        if to_stream {
            copy_bytes(e, at, from, n);
        } else {
            copy_bytes(e, from, at, n);
        }
        e.mem.set_u32(save_load + SL_POSITION, at + n);
    }
    e.register(0x0085_79b0, |e, a| {
        advance(e, a[1], a[2], true);
        ret(0)
    });
    e.register(0x0085_7a10, |e, a| {
        advance(e, a[1], a[2], true);
        ret(0)
    });
    e.register(0x0085_79e0, |e, a| {
        advance(e, a[1], a[2], false);
        ret(0)
    });
    e.register(0x0085_7aa0, |e, a| {
        advance(e, a[1], a[2], false);
        ret(1)
    });
}

/// A `TESActorBaseData` at +0x30 of a form (both with the test vtable), its fields filled from
/// `seed`, with no factions.
fn actor_data(e: &mut Engine, seed: u32) -> u32 {
    let form = e.mem.alloc(0x80);
    e.mem.set_u32(form, VTABLE);
    let c = form + 0x30;
    e.mem.set_u32(c, VTABLE);
    for i in 1..0x0b {
        e.mem
            .set_u32(c + 4 * i, seed.wrapping_mul(0x0101_0101).wrapping_add(i));
    }
    e.mem.set_f32(c + 0x14, 12.5);
    e.mem.set_u32(c + 0x1c, 0);
    e.mem.set_u32(c + 0x20, 0);
    e.mem.set_u32(c + 0x24, 0);
    c
}

/// A faction form with its flags (+0x34) and id.
fn faction(e: &mut Engine, flags: u32, id: u32) -> u32 {
    let f = form(e, 0x08, 0, id);
    e.mem.set_u32(f + 0x34, flags);
    f
}

/// The component's faction list: `(faction, rank)` entries; a 0 faction is a node without one.
fn set_factions(e: &mut Engine, c: u32, entries: &[(u32, i8)]) {
    let mut node = c + 0x2c;
    for (i, (f, rank)) in entries.iter().enumerate() {
        if i > 0 {
            let next = e.mem.alloc(8);
            e.mem.set_u32(node + 4, next);
            node = next;
        }
        if *f != 0 {
            let entry = e.mem.alloc(8);
            e.mem.set_u32(entry, *f);
            e.mem.set_i8(entry + 4, *rank);
            e.mem.set_u32(node, entry);
        }
    }
}

/// A float setting: its value at +4.
fn set_setting(e: &mut Engine, setting: u32, value: f32) {
    e.mem.set_f32(setting + 4, value);
}

/// The settings and tables the karma functions read: thresholds -750, -250, 250, 750 and the
/// default alignment values 0, -1000, -500, 1000, 500 (or, `consistent` false, a table that
/// breaks the alignment check).
fn karma_settings(e: &mut Engine, consistent: bool) {
    set_setting(e, 0x011c_5144, -250.0);
    set_setting(e, 0x011c_4f64, -750.0);
    set_setting(e, 0x011c_4e54, 250.0);
    set_setting(e, 0x011c_5090, 750.0);
    set_setting(e, 0x011c_5108, 1000.0);
    set_setting(e, 0x011c_5160, -1000.0);
    set_setting(e, 0x011c_4fa4, 52.75);
    set_setting(e, 0x011c_5074, 3.0);
    e.mem.set_u16(0x011c_50d8 + 4, 35);
    let defaults: [f32; 5] = if consistent {
        [0.0, 500.0, -500.0, 1000.0, -1000.0]
    } else {
        [0.0, 600.0, -2.0, 1000.0, -1000.0]
    };
    for (i, v) in defaults.into_iter().enumerate() {
        e.mem.set_f32(0x0118_6d08 + 4 * i as u32, v);
    }
    e.mem.set_i32(0x0118_6f78, -1);
    e.mem.set_f32(0x0101_7d00, 0.001);
    e.mem.set_f64(0x0101_1588, 0.5);
    e.mem.set_f64(0x0101_7a40, 100.0);
    e.mem.set_f64(0x0101_7b70, 1000.0);
}

/// Every actor base data test runs in [`actor_world`] with the karma settings.
fn actor_cases(d: Diff) -> Diff {
    // The script's set-up (`00565730`, the cell references family's) and the template walk's extra
    // data getters (`00418520`, `004121b0`, the extra data family's) are doubles here, so their
    // typed calls go by address.
    d.forms()
        .ignore_calls(ABD_IN_PLACE)
        .goes_by_address(&[0x0056_5730, 0x0041_8520, 0x0041_21b0])
}

/// The actor base data tests' shared cases, as steps of the `Diff` chain.
trait ActorCases {
    fn actor(self) -> Diff;
    fn abd_getter(self) -> Diff;
    fn abd_setter(self) -> Diff;
    fn template_flags(self) -> Diff;
    fn faction_flags(self) -> Diff;
    fn mark_call(self) -> Diff;
}

impl ActorCases for Diff {
    fn actor(self) -> Diff {
        actor_cases(self)
    }
    fn abd_getter(self) -> Diff {
        abd_getter_cases(self)
    }
    fn abd_setter(self) -> Diff {
        abd_setter_cases(self)
    }
    fn template_flags(self) -> Diff {
        template_flags_cases(self)
    }
    fn faction_flags(self) -> Diff {
        faction_flags_cases(self)
    }
    fn mark_call(self) -> Diff {
        mark_call_cases(self)
    }
}

/// A getter's cases: components filled from a few seeds.
fn abd_getter_cases(d: Diff) -> Diff {
    d.actor()
        .inputs(&[Input::Of(&[0, 1, 0x7f, 0xfe, 0xff])])
        .setup(|e, case| {
            actor_world(e);
            vec![actor_data(e, case.word(0))]
        })
}

/// A setter's cases: a value, over components filled from two seeds.
fn abd_setter_cases(d: Diff) -> Diff {
    d.actor()
        .inputs(&[
            Input::Of(&[0, 1]),
            Input::Of(&[0, 1, 0x8000, 0xffff, 0x1_2345]),
        ])
        .setup(|e, case| {
            actor_world(e);
            karma_settings(e, true);
            vec![actor_data(e, case.word(0)), case.word(1)]
        })
}

#[test]
fn abd_construct() {
    Diff::new(entry!(0x0047_c700, actor_base_data::construct(u32) -> u32))
        .actor()
        .inputs(&[Input::Of(&[0, 1])])
        .setup(|e, case| {
            actor_world(e);
            karma_settings(e, case.word(0) != 0);
            vec![actor_data(e, 3)]
        })
        .run()
        .assert_same();
}

#[test]
fn abd_no_vats_melee() {
    Diff::new(entry!(0x0047_c770, actor_base_data::no_vats_melee(u32) -> bool))
        .abd_getter()
        .run()
        .assert_same();
}

#[test]
fn abd_allow_pc_dialogue() {
    Diff::new(entry!(0x0047_c790, actor_base_data::allow_pc_dialogue(u32) -> bool))
        .abd_getter()
        .run()
        .assert_same();
}

#[test]
fn abd_allow_pick_pocket() {
    Diff::new(entry!(0x0047_c7b0, actor_base_data::allow_pick_pocket(u32) -> bool))
        .abd_getter()
        .run()
        .assert_same();
}

#[test]
fn abd_is_ghost() {
    Diff::new(entry!(0x0047_c7d0, actor_base_data::is_ghost(u32) -> bool))
        .abd_getter()
        .run()
        .assert_same();
}

#[test]
fn abd_invulnerable() {
    Diff::new(entry!(0x0047_c7f0, actor_base_data::invulnerable(u32) -> bool))
        .abd_getter()
        .run()
        .assert_same();
}

#[test]
fn abd_cant_open_doors() {
    Diff::new(entry!(0x0047_c810, actor_base_data::cant_open_doors(u32) -> bool))
        .abd_getter()
        .run()
        .assert_same();
}

#[test]
fn abd_auto_calc_service_flags() {
    Diff::new(entry!(
        0x0047_c830,
        actor_base_data::auto_calc_service_flags(u32) -> bool
    ))
    .abd_getter()
    .run()
    .assert_same();
}

#[test]
fn abd_karma() {
    Diff::new(entry!(0x0047_c860, actor_base_data::karma(u32) -> f32))
        .abd_getter()
        .run()
        .assert_same();
}

/// A component with factions (one node without one) for the list functions.
fn with_factions(e: &mut Engine, seed: u32) -> u32 {
    actor_world(e);
    karma_settings(e, true);
    let c = actor_data(e, seed);
    let (a, b) = (faction(e, 2, 0x10), faction(e, 0xa, 0x11));
    match seed {
        0 => {}
        1 => set_factions(e, c, &[(a, 1)]),
        _ => set_factions(e, c, &[(a, -1), (0, 0), (b, 4)]),
    }
    c
}

#[test]
fn abd_destructor() {
    Diff::new(entry!(0x0047_c880, actor_base_data::destructor(u32)))
        .actor()
        .inputs(&[Input::Of(&[0, 1, 2])])
        .setup(|e, case| vec![with_factions(e, case.word(0))])
        .run()
        .assert_same();
}

#[test]
fn abd_initialize_data_component() {
    Diff::new(entry!(
        0x0047_c8e0,
        actor_base_data::initialize_data_component(u32)
    ))
    .actor()
    .inputs(&[Input::Of(&[0, 0x4780_0000, 0x7fc0_0000, 0xc2f0_0000])])
    .setup(|e, case| {
        actor_world(e);
        karma_settings(e, true);
        e.mem.set_u32(0x011c_4fa4 + 4, case.word(0));
        vec![actor_data(e, 2)]
    })
    .run()
    .assert_same();
}

#[test]
fn abd_clear_data_component() {
    Diff::new(entry!(
        0x0047_c9b0,
        actor_base_data::clear_data_component(u32)
    ))
    .actor()
    .inputs(&[Input::Of(&[0, 1, 2])])
    .setup(|e, case| vec![with_factions(e, case.word(0))])
    .run()
    .assert_same();
}

#[test]
fn abd_clear_faction_list() {
    Diff::new(entry!(
        0x0047_c9d0,
        actor_base_data::clear_faction_list(u32)
    ))
    .actor()
    .inputs(&[Input::Of(&[0, 1, 2])])
    .setup(|e, case| vec![with_factions(e, case.word(0))])
    .run()
    .assert_same();
}

#[test]
fn abd_copy_faction_list() {
    Diff::new(entry!(
        0x0047_ca20,
        actor_base_data::copy_faction_list(u32, u32)
    ))
    .actor()
    .inputs(&[Input::Of(&[0, 1, 2]), Input::Of(&[0, 1, 2])])
    .setup(|e, case| {
        let this = with_factions(e, case.word(0));
        let source = actor_data(e, 9);
        let (a, b) = (faction(e, 0, 0x20), faction(e, 4, 0x21));
        match case.word(1) {
            0 => {}
            1 => set_factions(e, source, &[(a, 3)]),
            _ => set_factions(e, source, &[(a, 3), (0, 0), (b, -2)]),
        }
        vec![this, source]
    })
    .run()
    .assert_same();
}

#[test]
fn abd_save() {
    Diff::new(entry!(0x0047_cac0, actor_base_data::save(u32)))
        .actor()
        .inputs(&[Input::Flag, Input::Flag])
        .setup(|e, case| {
            actor_world(e);
            record_buffer(e);
            e.mem.set_u8(BIG_ENDIAN, case.word(0) as u8);
            let c = actor_data(e, 4);
            if case.word(1) != 0 {
                for offset in [0x1c, 0x20, 0x24] {
                    let f = form(e, 0x2a, 0, 0x300 + offset);
                    e.mem.set_u32(c + offset, f);
                }
            }
            vec![c]
        })
        .run()
        .assert_same();
}

#[test]
fn abd_swap_data() {
    Diff::new(entry!(0x0047_cb90, actor_base_data::swap_data(u32)))
        .actor()
        .inputs(&[Input::Of(&[0])])
        .setup(|e, _| {
            actor_world(e);
            vec![actor_data(e, 5) + 4]
        })
        .run()
        .assert_same();
}

/// Template-use flags cases: the flags and a bit index.
fn template_flags_cases(d: Diff) -> Diff {
    d.actor()
        .inputs(&[
            Input::Of(&[0, 0x8001, 0xffff]),
            Input::Of(&[0, 1, 15, 16, 0xffff_ffff]),
            Input::Flag,
        ])
        .setup(|e, case| {
            actor_world(e);
            let c = actor_data(e, 1);
            e.mem.set_u16(c + 0x1a, case.word(0) as u16);
            vec![c, case.word(1), case.word(2)]
        })
}

#[test]
fn abd_template_use_flag_15() {
    Diff::new(entry!(0x0047_cca0, actor_base_data::template_use_flag_15(u32) -> bool))
        .abd_getter()
        .run()
        .assert_same();
}

#[test]
fn abd_set_template_use_flag_15() {
    Diff::new(entry!(
        0x0047_ccc0,
        actor_base_data::set_template_use_flag_15(u32, u8)
    ))
    .abd_setter()
    .run()
    .assert_same();
}

#[test]
fn abd_set_template_use_flag() {
    Diff::new(entry!(
        0x0047_cce0,
        actor_base_data::set_template_use_flag(u32, i32, u8)
    ))
    .template_flags()
    .run()
    .assert_same();
}

#[test]
fn abd_template_use_flag() {
    Diff::new(entry!(
        0x0047_cd40,
        actor_base_data::template_use_flag(u32, i32) -> bool
    ))
    .template_flags()
    .run()
    .assert_same();
}

#[test]
fn abd_set_template_use_flags() {
    Diff::new(entry!(
        0x0047_cd90,
        actor_base_data::set_template_use_flags(u32, u16)
    ))
    .abd_setter()
    .run()
    .assert_same();
}

/// A component whose template is of `kind` (0: none).
fn templated(e: &mut Engine, kind: u32) -> u32 {
    actor_world(e);
    karma_settings(e, true);
    let c = actor_data(e, 6);
    if kind != 0 {
        let template = form(e, kind, 0, 0x400);
        e.mem.set_u32(c + 0x24, template);
    }
    c
}

#[test]
fn abd_has_actor_template_again() {
    Diff::new(entry!(
        0x0047_cdb0,
        actor_base_data::has_actor_template_again(u32) -> bool
    ))
    .actor()
    .inputs(&[Input::Of(&[0, 0x2b, 0x2c, 0x2d, 0x2e])])
    .setup(|e, case| vec![templated(e, case.word(0))])
    .run()
    .assert_same();
}

#[test]
fn abd_has_actor_template() {
    Diff::new(entry!(0x0047_cdd0, actor_base_data::has_actor_template(u32) -> bool))
        .actor()
        .inputs(&[Input::Of(&[0, 0x2b, 0x2c, 0x2d, 0x2e])])
        .setup(|e, case| vec![templated(e, case.word(0))])
        .run()
        .assert_same();
}

#[test]
fn abd_apply_template() {
    // The actor: none, blocked, or let through; the handler's flag; the actor's slots.
    Diff::new(entry!(
        0x0047_ce10,
        actor_base_data::apply_template(u32, u32)
    ))
    .actor()
    .inputs(&[
        Input::Of(&[0, 0x2c, 0x2d, 0x2e]),
        Input::Of(&[0, 1, 2]),
        Input::Flag,
        Input::Flag,
    ])
    .setup(|e, case| {
        let c = templated(e, case.word(0));
        let actor = match case.word(1) {
            0 => 0,
            kind => {
                set_answer(e, 0x0056_afc0, u32::from(kind == 1));
                let a = form(e, 0x2b, 0, 0x500);
                let process = e.mem.alloc(8);
                e.mem.set_u32(a + 0x68, process);
                let base = form(e, 0x2a, 0, 0x501);
                e.mem.set_u32(a + 0x20, base);
                a
            }
        };
        let flag = case.word(2);
        e.mem.set_u32(SLOT_ANSWERS + 0xfc, flag);
        e.mem.set_u32(SLOT_ANSWERS + 0x100, flag);
        // The inventory changes: their list (the first word) empty or not (`0042cde0`).
        let changes = e.mem.alloc(8);
        let list = e.mem.alloc(8);
        e.mem.set_u32(changes, list);
        e.mem.set_u32(list, u32::from(case.word(3) == 0));
        set_answer(e, 0x004b_f220, changes);
        if case.word(3) != 0 {
            let handler = e.mem.alloc(8);
            e.mem.set_u32(BGS_SAVE_LOAD_GAME, handler);
            set_answer(e, 0x0041_21b0, flag);
            e.mem.set_u32(0x011c_5290, 0x55);
        }
        let created = actor_data(e, 8) - 0x30;
        set_answer(e, 0x0046_5110, created);
        vec![c, actor]
    })
    .run()
    .assert_same();
}

#[test]
fn abd_path_manager_instance() {
    Diff::new(entry!(0x0047_d0b0, actor_base_data::path_manager_instance() -> u32))
        .actor()
        .inputs(&[Input::Of(&[0, 1, 2, 3])])
        .setup(|e, case| {
            actor_world(e);
            e.mem.set_u32(0x011c_5298, case.word(0));
            vec![]
        })
        .run()
        .assert_same();
}

#[test]
fn abd_create_template_actor_base() {
    Diff::new(entry!(
        0x0047_d130,
        actor_base_data::create_template_actor_base(u32, u32) -> u32
    ))
    .actor()
    .inputs(&[Input::Of(&[0, 1, 0x101])])
    .setup(|e, case| {
        actor_world(e);
        set_answer(e, 0x0046_23f0, case.word(0));
        let created = actor_data(e, 8) - 0x30;
        set_answer(e, 0x0046_5110, created);
        vec![form(e, 0x2a, 0, 0x600), 0x4321]
    })
    .run()
    .assert_same();
}

#[test]
fn abd_template_chain_mask() {
    // A chain of three forms; the slot 0xf8 says true for the first `depth` of them.
    Diff::new(entry!(
        0x0047_d1a0,
        actor_base_data::template_chain_mask(u32, u32) -> u32
    ))
    .actor()
    .inputs(&[Input::Of(&[0, 1, 2]), Input::Flag])
    .setup(|e, case| {
        actor_world(e);
        e.mem.set_u32(SLOT_ANSWERS + 0xf8, case.word(1));
        let third = actor_data(e, 3) - 0x30;
        let second = actor_data(e, 2) - 0x30;
        let first = actor_data(e, 1) - 0x30;
        e.mem.set_u32(first + 0x30 + 0x24, second);
        e.mem.set_u32(
            second + 0x30 + 0x24,
            if case.word(0) > 0 { third } else { 0 },
        );
        e.mem.set_u16(first + 0x4a, 0xf0f0);
        e.mem.set_u16(second + 0x4a, 0x3c3c);
        let mask = e.mem.alloc(4);
        vec![first, mask]
    })
    .run()
    .assert_same();
}

#[test]
fn abd_copy_component() {
    Diff::new(entry!(
        0x0047_d220,
        actor_base_data::copy_component(u32, u32)
    ))
    .actor()
    .inputs(&[Input::Flag, Input::Of(&[0, 1, 2])])
    .setup(|e, case| {
        let this = with_factions(e, case.word(1));
        let other = actor_data(e, 0x44);
        let rank = faction(e, 0, 0x30);
        set_factions(e, other, &[(rank, 2)]);
        set_cast(e, 0x0118_6cb4, if case.word(0) != 0 { other } else { 0 });
        e.mem.set_u32(SLOT_ANSWERS + 0x60, 0x1_0033);
        e.mem.set_u32(SLOT_ANSWERS + 0x64, 1234.5f32.to_bits());
        e.mem.set_u32(SLOT_ANSWERS + 0x68, 0x0202_0202);
        vec![this, other + 0x10]
    })
    .run()
    .assert_same();
}

#[test]
fn abd_level() {
    Diff::new(entry!(0x0047_d370, actor_base_data::level(u32) -> u16))
        .abd_getter()
        .run()
        .assert_same();
}

#[test]
fn abd_calc_level_min() {
    Diff::new(entry!(0x0047_d390, actor_base_data::calc_level_min(u32) -> u16))
        .abd_getter()
        .run()
        .assert_same();
}

#[test]
fn abd_calc_level_max() {
    Diff::new(entry!(0x0047_d3b0, actor_base_data::calc_level_max(u32) -> u16))
        .abd_getter()
        .run()
        .assert_same();
}

#[test]
fn abd_base_disposition() {
    Diff::new(entry!(0x0047_d3d0, actor_base_data::base_disposition(u32) -> u16))
        .abd_getter()
        .run()
        .assert_same();
}

#[test]
fn abd_bartergold() {
    Diff::new(entry!(0x0047_d3f0, actor_base_data::bartergold(u32) -> u16))
        .abd_getter()
        .run()
        .assert_same();
}

#[test]
fn abd_compare_component() {
    // What differs: 0 nothing; 1 the flags; 2 a faction's rank; 3 the template use flags; 4 the
    // karma (a NaN); 5 the faction count.
    Diff::new(entry!(
        0x0047_d410,
        actor_base_data::compare_component(u32, u32) -> bool
    ))
    .actor()
    .inputs(&[Input::Flag, Input::Of(&[0, 1, 2, 3, 4, 5])])
    .setup(|e, case| {
        actor_world(e);
        karma_settings(e, true);
        let this = actor_data(e, 7);
        let other = actor_data(e, 7);
        let (a, b) = (faction(e, 0, 0x40), faction(e, 0, 0x41));
        set_factions(e, this, &[(a, 1), (b, 2)]);
        let differ = case.word(1);
        let rank = if differ == 2 { 3 } else { 2 };
        if differ == 5 {
            set_factions(e, other, &[(a, 1)]);
        } else {
            set_factions(e, other, &[(b, rank), (a, 1)]);
        }
        match differ {
            1 => e.mem.set_u32(other + 4, 0xdead),
            3 => e.mem.set_u16(other + 0x1a, 0x77),
            _ => {}
        }
        let karma = if differ == 4 { f32::NAN } else { 2.0 };
        e.mem.set_u32(SLOT_ANSWERS + 0x64, karma.to_bits());
        set_cast(e, 0x0118_6cb4, if case.word(0) != 0 { other } else { 0 });
        vec![this, other + 0x10]
    })
    .run()
    .assert_same();
}

#[test]
fn abd_faction_rank() {
    Diff::new(entry!(
        0x0047_d680,
        actor_base_data::faction_rank(u32, u32, u8) -> i32
    ))
    .actor()
    .inputs(&[Input::Of(&[0, 1, 2]), Input::Of(&[0, 1, 2]), Input::Flag])
    .setup(|e, case| {
        let c = with_factions(e, case.word(0));
        // The factions `with_factions` makes are the two forms it allocates first after the
        // component: look them up through the list instead.
        let first = e.mem.u32(c + 0x2c);
        let wanted = match case.word(1) {
            0 => 0x1234,
            1 if first != 0 => e.mem.u32(first),
            _ => {
                let last = e.mem.u32(c + 0x30);
                let last = if last != 0 { e.mem.u32(last + 4) } else { 0 };
                if last != 0 {
                    e.mem.u32(e.mem.u32(last))
                } else {
                    0x99
                }
            }
        };
        vec![c, wanted, case.word(2)]
    })
    .run()
    .assert_same();
}

/// A faction flag test's cases.
fn faction_flags_cases(d: Diff) -> Diff {
    d.actor()
        .inputs(&[Input::Of(&[0, 2, 4, 8, 0xe, 0xffff_ffff])])
        .setup(|e, case| {
            actor_world(e);
            vec![faction(e, case.word(0), 1)]
        })
}

#[test]
fn abd_faction_flag_8() {
    Diff::new(entry!(0x0047_d720, actor_base_data::faction_flag_8(u32) -> bool))
        .faction_flags()
        .run()
        .assert_same();
}

#[test]
fn abd_in_evil_factions_only() {
    Diff::new(entry!(
        0x0047_d740,
        actor_base_data::in_evil_factions_only(u32) -> bool
    ))
    .actor()
    .inputs(&[Input::Of(&[0, 1, 2]), Input::Flag])
    .setup(|e, case| {
        let c = with_factions(e, case.word(0));
        if case.word(1) != 0 {
            // Every listed faction evil.
            let mut node = c + 0x2c;
            while node != 0 {
                let entry = e.mem.u32(node);
                if entry != 0 {
                    let f = e.mem.u32(entry);
                    e.mem.set_u32(f + 0x34, 2);
                }
                node = e.mem.u32(node + 4);
            }
        }
        vec![c]
    })
    .run()
    .assert_same();
}

#[test]
fn abd_faction_flag_2() {
    Diff::new(entry!(0x0047_d7c0, actor_base_data::faction_flag_2(u32) -> bool))
        .faction_flags()
        .run()
        .assert_same();
}

#[test]
fn abd_faction_flag_4() {
    Diff::new(entry!(0x0047_d7e0, actor_base_data::faction_flag_4(u32) -> bool))
        .faction_flags()
        .run()
        .assert_same();
}

#[test]
fn abd_set_faction_rank() {
    // The faction: 0 none; 1 the listed first one; 2 an unlisted one. The rank: -1 or 5.
    Diff::new(entry!(
        0x0047_d800,
        actor_base_data::set_faction_rank(u32, u32, i8)
    ))
    .actor()
    .inputs(&[
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0xff, 5]),
    ])
    .setup(|e, case| {
        let c = with_factions(e, case.word(0));
        let first = e.mem.u32(c + 0x2c);
        let target = match case.word(1) {
            0 => 0,
            1 if first != 0 => e.mem.u32(first),
            _ => faction(e, 0, 0x77),
        };
        vec![c, target, case.word(2)]
    })
    .run()
    .assert_same();
}

#[test]
fn abd_init_item() {
    // The factions resolve (1) or not (0); the karma: a default value, or not; the table
    // consistent or not; the template and its use flag 0.
    Diff::new(entry!(0x0047_d8b0, actor_base_data::init_item(u32, u32)))
        .actor()
        .inputs(&[
            Input::Flag,
            Input::Of(&[0, 1, 2, 3, 4, 5]),
            Input::Flag,
            Input::Of(&[0, 1, 2]),
        ])
        .setup(|e, case| {
            let c = with_factions(e, 2);
            karma_settings(e, case.word(2) != 0);
            let m = form(e, 0x7f, 0, 0);
            e.mem.set_u32(FORM_MAP, m);
            set_found(e, true);
            let found = faction(e, 0, 0x88);
            e.mem.set_u32(LOOKUP_VALUE, found);
            let resolved = if case.word(0) != 0 { found } else { 0 };
            for target in [0x0118_4704, 0x0118_6318, 0x0118_6f7c] {
                set_cast(e, target, resolved);
            }
            e.mem.set_u32(c + 0x20, 0x0100_0020);
            e.mem.set_u32(c + 0x1c, 0x0100_001c);
            match case.word(3) {
                0 => {}
                use_flag => {
                    e.mem.set_u32(c + 0x24, 0x0100_0024);
                    e.mem.set_u16(c + 0x1a, (use_flag - 1) as u16);
                }
            }
            let karma = [0.0005f32, 500.0, -500.0, 1000.0, -1000.0, 42.0][case.word(1) as usize];
            e.mem.set_f32(c + 0x14, karma);
            let owner = form(e, 0x2a, 0, 0x0100_0abc);
            // Two source files with their compile indexes: the ids are remapped by the last one.
            let (a, b) = (file(e, true), file(e, false));
            e.mem.set_u8(a + FILE_COMPILE_INDEX_AT, 3);
            e.mem.set_u8(b + FILE_COMPILE_INDEX_AT, 7);
            set_files(e, owner, &[a, b]);
            vec![c, owner]
        })
        .run()
        .assert_same();
}

#[test]
fn abd_set_base_flags() {
    Diff::new(entry!(
        0x0047_dd30,
        actor_base_data::set_base_flags(u32, u32)
    ))
    .abd_setter()
    .run()
    .assert_same();
}

#[test]
fn abd_set_flag_bit() {
    Diff::new(entry!(
        0x0047_dd50,
        actor_base_data::set_flag_bit(u32, u32, u8, u8)
    ))
    .actor()
    .inputs(&[
        Input::Of(&[1, 2, 0x80]),
        Input::Flag,
        Input::Flag,
        Input::Of(&[0, 1, 2]),
    ])
    .setup(|e, case| {
        actor_world(e);
        let c = actor_data(e, 0);
        e.mem.set_u32(c + 4, 0x0000_0082);
        // The cast to a form: none (0), and the save game's change answer (1, 2).
        let form = if case.word(3) != 0 { c - 0x30 } else { 0 };
        set_cast(e, 0x0118_3028, form);
        set_answer(e, 0x0084_a6d0, u32::from(case.word(3) == 2));
        vec![c, case.word(0), case.word(1), case.word(2)]
    })
    .run()
    .assert_same();
}

#[test]
fn abd_set_calc_level_min() {
    Diff::new(entry!(
        0x0047_de40,
        actor_base_data::set_calc_level_min(u32, u16)
    ))
    .abd_setter()
    .run()
    .assert_same();
}

#[test]
fn abd_set_calc_level_max() {
    Diff::new(entry!(
        0x0047_de70,
        actor_base_data::set_calc_level_max(u32, u16)
    ))
    .abd_setter()
    .run()
    .assert_same();
}

#[test]
fn abd_set_base_disposition() {
    Diff::new(entry!(
        0x0047_dea0,
        actor_base_data::set_base_disposition(u32, u16)
    ))
    .abd_setter()
    .run()
    .assert_same();
}

#[test]
fn abd_calculated_level() {
    // The level, flag 0x80, the player's base form there or not, the calculated range, the player's
    // level (10 with 700 or 900: a product just under an integer, which 24 bits round up).
    Diff::new(entry!(0x0047_ded0, actor_base_data::calculated_level(u32) -> u16))
        .actor()
        .inputs(&[
            Input::Of(&[1, 500, 700, 900, 2000]),
            Input::Flag,
            Input::Flag,
            Input::Of(&[0, 1, 2]),
            Input::Of(&[37, 10]),
        ])
        .setup(|e, case| {
            actor_world(e);
            karma_settings(e, true);
            let c = actor_data(e, 0);
            e.mem
                .set_u32(c + 4, if case.word(1) != 0 { 0x80 } else { 0 });
            e.mem.set_u16(c + 0xc, case.word(0) as u16);
            let (min, max) = [(0, 0), (30, 0), (0, 12)][case.word(3) as usize];
            e.mem.set_u16(c + 0xe, min);
            e.mem.set_u16(c + 0x10, max);
            let player = e.mem.alloc(0x30);
            e.mem.set_u32(0x011d_ea3c, player);
            if case.word(2) != 0 {
                let base = actor_data(e, 0) - 0x30;
                e.mem.set_u16(base + 0x30 + 0xc, case.word(4) as u16);
                e.mem.set_u32(player + 0x20, base);
            }
            vec![c]
        })
        .run()
        .assert_same();
}

#[test]
fn abd_set_level() {
    Diff::new(entry!(0x0047_dfe0, actor_base_data::set_level(u32, u16)))
        .abd_setter()
        .run()
        .assert_same();
}

#[test]
fn abd_set_fatigue() {
    Diff::new(entry!(0x0047_e010, actor_base_data::set_fatigue(u32, u16)))
        .abd_setter()
        .run()
        .assert_same();
}

#[test]
fn abd_alignment_for_karma() {
    Diff::new(entry!(
        0x0047_e040,
        actor_base_data::alignment_for_karma(f32) -> i32
    ))
    .actor()
    .inputs(&[Input::Of(&[
        0xc47a_0000, // -1000
        0xc43b_8000, // -750
        0xc3fa_0000, // -500
        0xc37a_0000, // -250
        0x0000_0000,
        0x437a_0000, // 250
        0x43fa_0000, // 500
        0x443b_8000, // 750
        0x447a_0000, // 1000
        0x7fc0_0000, // NaN
    ])])
    .setup(|e, case| {
        actor_world(e);
        karma_settings(e, true);
        vec![case.word(0)]
    })
    .run()
    .assert_same();
}

#[test]
fn abd_karmic_title() {
    Diff::new(entry!(0x0047_e0e0, actor_base_data::karmic_title(u32) -> u32))
        .actor()
        .inputs(&[
            Input::Flag,
            Input::Of(&[0, 1, 30, 50, 70]),
            Input::Of(&[0xffff_fc18, 0xffff_ff9c, 0, 0x190, 0x3e8]),
        ])
        .setup(|e, case| {
            actor_world(e);
            karma_settings(e, true);
            for row in 0..3u32 {
                for level in 0..50u32 {
                    let setting = if (row + level) % 3 == 0 {
                        0
                    } else {
                        let s = e.mem.alloc(8);
                        e.mem.set_u32(s + 4, 0x0200_0000 + row * 0x100 + level);
                        s
                    };
                    e.mem.set_u32(0x0118_6d20 + row * 0xc8 + level * 4, setting);
                }
            }
            set_answer(e, 0x0087_f9f0, case.word(1));
            let actor = if case.word(0) != 0 {
                let a = form(e, 0x2b, 0, 0x700);
                e.mem.set_u32(a + 0xa4, VTABLE);
                a
            } else {
                0
            };
            e.mem.set_u32(SLOT_ANSWERS + 8, case.word(2));
            vec![actor]
        })
        .run()
        .assert_same();
}

#[test]
fn abd_set_karma() {
    Diff::new(entry!(0x0047_e1c0, actor_base_data::set_karma(u32, f32)))
        .actor()
        .inputs(&[Input::Of(&[
            0,
            0x447a_4000,
            0xc47a_4000,
            0x7fc0_0000,
            0x3f80_0000,
            0x7f80_0001,
        ])])
        .setup(|e, case| {
            actor_world(e);
            karma_settings(e, true);
            vec![actor_data(e, 1), case.word(0)]
        })
        .run()
        .assert_same();
}

#[test]
fn abd_set_bartergold() {
    Diff::new(entry!(
        0x0047_e240,
        actor_base_data::set_bartergold(u32, u16)
    ))
    .abd_setter()
    .run()
    .assert_same();
}

#[test]
fn abd_set_speed_mult() {
    Diff::new(entry!(
        0x0047_e270,
        actor_base_data::set_speed_mult(u32, u16)
    ))
    .abd_setter()
    .run()
    .assert_same();
}

/// A mark call's cases: a component, or none.
fn mark_call_cases(d: Diff) -> Diff {
    d.actor()
        .inputs(&[Input::Flag, Input::Of(&[2, 0x0100_0000])])
        .setup(|e, case| {
            actor_world(e);
            let c = if case.word(0) != 0 {
                actor_data(e, 1)
            } else {
                0
            };
            vec![c, case.word(1)]
        })
}

#[test]
fn abd_mark_changed() {
    Diff::new(entry!(0x0047_e2a0, actor_base_data::mark_changed(u32, u32)))
        .mark_call()
        .run()
        .assert_same();
}

#[test]
fn abd_mark_unchanged() {
    Diff::new(entry!(
        0x0047_e2f0,
        actor_base_data::mark_unchanged(u32, u32)
    ))
    .mark_call()
    .run()
    .assert_same();
}

#[test]
fn abd_set_change_flags() {
    Diff::new(entry!(
        0x0047_e340,
        actor_base_data::set_change_flags(u32, u32)
    ))
    .abd_setter()
    .run()
    .assert_same();
}

#[test]
fn abd_clear_change_flags() {
    Diff::new(entry!(
        0x0047_e360,
        actor_base_data::clear_change_flags(u32, u32)
    ))
    .abd_setter()
    .run()
    .assert_same();
}

/// The save's set-up: blocks in use or not, the debug switch, a current save entry or not.
fn save_setup(e: &mut Engine, case: &engine_core::typed::diff::Case, factions: u32) -> u32 {
    let c = with_factions(e, factions);
    let save_load = e.mem.alloc(0x100);
    e.mem.set_u32(SAVE_LOAD_GAME, save_load);
    set_answer(e, 0x0086_2110, case.word(0));
    e.mem.set_u8(0x011d_e4e8 + 4, case.word(1) as u8);
    if case.word(2) != 0 {
        let entry = e.mem.alloc(0x10);
        e.mem.set_u32(entry, 0x0100_0abc);
        e.mem.set_u32(entry + 5, 0x1234_5678);
        e.mem.set_u8(entry + 9, 0x52);
        e.mem.set_u32(save_load + SL_SAVE_ENTRY, entry);
        e.mem.set_u32(save_load + SL_LOAD_ENTRY, entry);
        let m = form(e, 0x7f, 0, 0);
        e.mem.set_u32(FORM_MAP, m);
        set_found(e, true);
        let named = form(e, 0x2a, 0, 0x0100_0abc);
        e.mem.set_u32(LOOKUP_VALUE, named);
    }
    e.mem.set_u8(save_load + SL_VERSION, 0x53);
    e.mem.set_u32(save_load + SL_POSITION, STREAM);
    c
}

#[test]
fn abd_save_size() {
    Diff::new(entry!(0x0047_e380, actor_base_data::save_size(u32, u32) -> u16))
        .actor()
        .inputs(&[Input::Flag, Input::Flag, Input::Flag, Input::Of(&[0, 2, 3])])
        .setup(|e, case| {
            let c = save_setup(e, case, 2);
            vec![c, case.word(3)]
        })
        .run()
        .assert_same();
}

#[test]
fn abd_save_game() {
    Diff::new(entry!(0x0047_e4a0, actor_base_data::save_game(u32, u32)))
        .actor()
        .inputs(&[
            Input::Flag,
            Input::Flag,
            Input::Flag,
            Input::Of(&[0, 2]),
            Input::Of(&[0, 2]),
        ])
        .setup(|e, case| {
            let c = save_setup(e, case, case.word(4));
            vec![c, case.word(3)]
        })
        .run()
        .assert_same();
}

#[test]
fn abd_load_game() {
    // The stream: its block header right or wrong, the block's size against what is read, the
    // factions that resolve.
    Diff::new(entry!(
        0x0047_e6f0,
        actor_base_data::load_game(u32, u32, u32)
    ))
    .actor()
    .inputs(&[
        Input::Flag,
        Input::Flag,
        Input::Flag,
        Input::Of(&[0, 2]),
        Input::Of(&[0, 1, 2]),
        Input::Flag,
    ])
    .setup(|e, case| {
        let c = save_setup(e, case, 1);
        let flags = case.word(3);
        let mut bytes: Vec<u8> = Vec::new();
        if case.word(0) != 0 {
            let tag: u32 = if case.word(5) != 0 {
                0x424c_4f4b
            } else {
                0x1111_1111
            };
            bytes.extend(tag.to_le_bytes());
            let body = if flags != 0 { 0x18 } else { 0 } + 2 + 5 * 2;
            let size = [body, body - 3, body + 3][case.word(4) as usize] as u16;
            bytes.extend(size.to_le_bytes());
        }
        if flags != 0 {
            bytes.extend((0..0x18u8).map(|i| i * 3 + 1));
        }
        bytes.extend(2u16.to_le_bytes());
        for (id, rank) in [(0x0100_0041u32, 3u8), (0x0100_0042, 0xfe)] {
            bytes.extend(id.to_le_bytes());
            bytes.push(rank);
        }
        for (i, b) in bytes.into_iter().enumerate() {
            e.mem.set_u8(STREAM + i as u32, b);
        }
        if case.word(5) == 0 && case.word(2) == 0 {
            // Without a current load entry, no form resolves either. (With one, a wrong tag logs
            // the header message naming the entry's form, which resolves.)
            set_found(e, false);
            let save_load = e.mem.u32(SAVE_LOAD_GAME);
            e.mem.set_u32(save_load + SL_LOAD_ENTRY, 0);
        } else if case.word(2) == 0 {
            let m = form(e, 0x7f, 0, 0);
            e.mem.set_u32(FORM_MAP, m);
            set_found(e, true);
            let f = faction(e, 0, 0x41);
            e.mem.set_u32(LOOKUP_VALUE, f);
        }
        vec![c, flags, 0x99]
    })
    .run()
    .assert_same();
}

#[test]
fn abd_fatigue() {
    Diff::new(entry!(0x004a_8ae0, actor_base_data::fatigue(u32) -> u16))
        .abd_getter()
        .run()
        .assert_same();
}

// ---- TESActorBase (`actor_base`) and TESAttributes (`attributes`) ----

/// The one-line bodies the actor base's and the attributes' native code does in place (see their
/// module comments): left out of the translation's calls, with no double, as [`IN_PLACE`].
/// This family's own bodies the callers do in place: the attribute byte, the attribute getters by actor
/// value, the creature flag test 0x10 or 1 and the iterator's constructor. Each is left out only in its
/// callers' tests (in its own, it is the function under test).
const AB_ATTRIBUTE_BYTE: &[u32] = &[0x0048_0160];
const AB_ATTRIBUTE_GETTERS: &[u32] = &[
    0x005f_11f0,
    0x005f_1210,
    0x005f_1230,
    0x005f_1250,
    0x005f_1270,
    0x005f_1290,
    0x005f_12b0,
];
const AB_IN_PLACE: &[u32] = &[
    0x0040_1170, // the form type
    0x0046_1580, // the base flags' test
    0x0044_ddc0, // the form flags
    0x0040_3550, // and their setter
    0x0084_e3a0, // the form id
    0x0046_9860, // the created-id test
    0x0047_e340, // the actor base data's change flags, set
    0x0047_e360, // and cleared
    0x008f_21d0, // its speed multiplier
    0x004f_1540, // the AI form's bytes
    0x004f_6de0,
    0x005b_b4d0,
    0x0047_ef30,
    0x0062_f2f0,
    0x0047_ee30,
    0x006e_cd40, // the health's setter
    0x0040_8da0, // the full name's text
    0x0041_81e0, // a reference's base object
    0x007a_f430,
    0x008d_8520, // a reference's process
    0x0072_6070, // the word at +4
    0x0042_ce10, // the save/load game's flag 2
    0x008c_71b0, // the change flags' word
    0x0042_8110, // a save buffer's change flags
    0x0042_80f0, // their bits
    0x0040_8d60, // a setting's value
    0x0082_5c00, // the save/load game's buffer position
    0x004f_d3e0, // its current save entry
    0x004f_d3c0, // and load entry
    0x008d_f040, // its version byte
    0x005d_8b10, // and its base's
    0x009d_2510, // the attributes' size
    0x0066_ec10, // ToActorValue
    0x0040_2d30, // the base component's constructor
];
/// The floats the combat formulas answer (`ST0`), a double each.
const FLOAT_ANSWERS: u32 = 0x0300_5000;
const FLOAT_CALLEES: [u32; 5] = [
    0x0064_4ce0, // CalcWeaponDamage
    0x0064_7cf0, // the clothing's rating
    0x0064_6360, // CalcArmorRating
    0x004b_e060, // the armour's base rating
    0x004b_e180, // and its extra
];
/// A text the full name holds (`Abc`).
const NAME_TEXT: u32 = 0x0300_5100;
/// The player reference.
const PLAYER_REF: u32 = 0x011d_ea3c;

fn float_answer<const I: u32>(e: &mut Engine, _: &[u32]) -> Ret {
    let st0 = e.mem.f64(FLOAT_ANSWERS + 8 * I);
    Ret {
        st0,
        ..Ret::default()
    }
}

/// The actor base data world plus what the actor base reaches: the AI form's and spell list's
/// functions, the save/load game's change tests, the combat formulas, the iterator's walk, the
/// name's setter and the biped model form's members, all doubles; slots 0x22c and 0x468.
fn ab_world(e: &mut Engine) {
    actor_world(e);
    karma_settings(e, true);
    for page in [0x011c_a000, 0x0101_6000] {
        e.map(page, 0x1000);
    }
    answered!(
        e,
        0x0084_a620,
        0x0084_a740,
        0x0048_73d0,
        0x0047_ea30,
        0x0047_ea70,
        0x0048_e040,
        0x0048_e140,
        0x0048_d8f0,
        0x0048_da20,
        0x0048_dca0,
        0x0086_5e70,
        0x0048_9100,
        0x0096_bbc0,
        0x00ec_782f,
        0x00ec_5fce,
        0x0048_8f50,
        0x0048_04d0,
        0x0048_0570,
        0x0051_b950,
    );
    e.register(FLOAT_CALLEES[0], float_answer::<0>);
    e.register(FLOAT_CALLEES[1], float_answer::<1>);
    e.register(FLOAT_CALLEES[2], float_answer::<2>);
    e.register(FLOAT_CALLEES[3], float_answer::<3>);
    e.register(FLOAT_CALLEES[4], float_answer::<4>);
    // The armour's base rating truncates to 65836, whose low word (300) is what the rating takes; its
    // extra rating is a hair above half a float's step at 11 (2^-21 + 2^-56), so 11 plus it rounds
    // up in a 24-bit add and to even (11) in a 53-bit add and the float store: the threads'
    // precisions give different results.
    let extra = f64::from_bits(0x3ea0_0000_0002_0000);
    for (i, v) in [7.25f64, 3.5, 11.0, 65836.75, extra]
        .into_iter()
        .enumerate()
    {
        e.mem.set_f64(FLOAT_ANSWERS + 8 * i as u32, v);
    }
    // LoadString: two characters and a zero into the buffer it is given.
    e.register(0x0086_49a0, |e, a| {
        for (i, b) in [b'N', b'm', 0].into_iter().enumerate() {
            e.mem.set_u8(a[1] + i as u32, b);
        }
        ret(0)
    });
    for slot in [0x22c, 0x468] {
        e.mem.set_u32(VTABLE + slot, VT_FN + slot);
    }
    e.register(VT_FN + 0x22c, slot_answer::<0x22c>);
    e.register(VT_FN + 0x468, slot_answer::<0x468>);
    for (i, b) in b"Abc\0".iter().enumerate() {
        e.mem.set_u8(NAME_TEXT + i as u32, *b);
    }
    let bgs = e.mem.alloc(0x300);
    e.mem.set_u32(BGS_SAVE_LOAD_GAME, bgs);
}

/// An actor base: form type `kind`, form flags `flags`, id `id`, its actor base data filled from `id`
/// (no factions), the base flags `base_flags`, the AI form's and the attributes' bytes, the health
/// form (the test vtable, health 0x37) and the full name holding [`NAME_TEXT`] (or nothing).
fn actor_base_form(
    e: &mut Engine,
    kind: u32,
    flags: u32,
    id: u32,
    base_flags: u32,
    named: bool,
) -> u32 {
    let a = e.mem.alloc(0x120);
    e.mem.set_u32(a, VTABLE);
    e.mem.set_u8(a + 4, kind as u8);
    e.mem.set_u32(a + 8, flags);
    e.mem.set_u32(a + 0xc, id);
    let c = a + 0x30;
    e.mem.set_u32(c, VTABLE);
    for i in 2..0x0b {
        e.mem
            .set_u32(c + 4 * i, 0x2357_1113u32.wrapping_mul(i).wrapping_add(id));
    }
    e.mem.set_u32(c + 4, base_flags);
    e.mem.set_f32(c + 0x14, 12.5);
    for i in [0x1c, 0x20, 0x24, 0x28, 0x2c, 0x30] {
        e.mem.set_u32(c + i, 0);
    }
    e.mem.set_u32(c + 0x28, 0x0f0f_0003);
    for i in 0..0x20 {
        e.mem.set_u8(a + 0x90 + i, (0x21 + 7 * i) as u8);
    }
    e.mem.set_u32(a + 0x90, VTABLE);
    e.mem.set_u32(a + 0xb0, VTABLE);
    e.mem.set_u32(a + 0xb4, 0x37);
    e.mem.set_u32(a + 0xb8, VTABLE);
    for i in 0..7 {
        e.mem.set_u8(a + 0xbc + i, (0x41 + 5 * i) as u8);
    }
    e.mem.set_u32(a + 0xd0, VTABLE);
    e.mem.set_u32(a + 0xd4, if named { NAME_TEXT } else { 0 });
    a
}

/// Every actor base and attributes test runs in [`ab_world`].
fn ab_cases(d: Diff) -> Diff {
    d.forms()
        .ignore_calls(ABD_IN_PLACE)
        .ignore_calls(AB_IN_PLACE)
        .goes_by_address(&[0x0056_5730, 0x0041_8520, 0x0041_21b0])
}

const AB_TYPES: &[u32] = &[0x2a, 0x2b, 0x18];
const BASE_FLAGS: &[u32] = &[
    0,
    1,
    0x10,
    0x20,
    0x40,
    0x0004_0010,
    0x0080_0000,
    // The bits 2, 0x20000 and 0x400000, none a tested mask's own: a widened mask shows.
    0x0042_0002,
    0xffff_ffff,
];

/// A flag getter's cases: a form type and base flags.
fn ab_flag_cases(d: Diff) -> Diff {
    d.ab()
        .inputs(&[Input::Of(AB_TYPES), Input::Of(BASE_FLAGS)])
        .setup(|e, case| {
            ab_world(e);
            vec![actor_base_form(
                e,
                case.word(0),
                0,
                0x0100_0001,
                case.word(1),
                false,
            )]
        })
}

/// The save/load set-ups: save blocks or not, the reports on or not, a current save and load
/// entry or not, the save version (0x53 or 0x6d), and the buffer position over the stream.
fn ab_save_setup(e: &mut Engine, case: &engine_core::typed::diff::Case, named: bool) -> u32 {
    ab_world(e);
    let a = actor_base_form(e, 0x2a, 0x0040_0000, 0x0100_0abc, 0x11, named);
    let save_load = e.mem.alloc(0x100);
    e.mem.set_u32(SAVE_LOAD_GAME, save_load);
    set_answer(e, 0x0086_2110, case.word(0));
    e.mem.set_u8(0x011d_e4e8 + 4, case.word(1) as u8);
    if case.word(2) != 0 {
        let entry = e.mem.alloc(0x10);
        e.mem.set_u32(entry, 0x0100_0abc);
        e.mem.set_u32(entry + 5, 0x1234_5678);
        e.mem.set_u8(entry + 9, 0x52);
        e.mem.set_u32(save_load + SL_SAVE_ENTRY, entry);
        e.mem.set_u32(save_load + SL_LOAD_ENTRY, entry);
        let m = form(e, 0x7f, 0, 0);
        e.mem.set_u32(FORM_MAP, m);
        set_found(e, true);
        e.mem.set_u32(LOOKUP_VALUE, a);
    }
    let version = if case.word(3) != 0 { 0x6d } else { 0x53 };
    e.mem.set_u8(save_load + SL_VERSION, version);
    e.mem.set_u32(save_load + SL_POSITION, STREAM);
    set_answer(e, 0x0048_d8f0, 0x0011);
    a
}

/// The actor base tests' shared cases, as steps of the `Diff` chain.
trait AbCases {
    fn ab(self) -> Diff;
    fn ab_flags(self) -> Diff;
    fn ab_attribute_getter(self) -> Diff;
    fn ab_attribute_setter(self) -> Diff;
    fn ab_change(self) -> Diff;
    fn ab_buffer(self) -> Diff;
}

impl AbCases for Diff {
    fn ab(self) -> Diff {
        ab_cases(self)
    }
    fn ab_flags(self) -> Diff {
        ab_flag_cases(self)
    }
    fn ab_attribute_getter(self) -> Diff {
        self.ab()
            .ignore_calls(AB_ATTRIBUTE_BYTE)
            .inputs(&[Input::Of(&[0, 1])])
            .setup(|e, case| {
                ab_world(e);
                let a = actor_base_form(e, 0x2a, 0, 0x0100_0002 + case.word(0), 0, false);
                vec![a + 0xb8]
            })
    }
    fn ab_attribute_setter(self) -> Diff {
        self.ab()
            .inputs(&[Input::Of(&[0, 7, 0xff]), Input::Flag])
            .setup(|e, case| {
                ab_world(e);
                let a = actor_base_form(e, 0x2a, 0, 0x0100_0003, 0, false);
                set_cast(e, TYPE_TES_FORM, if case.word(1) != 0 { a } else { 0 });
                vec![a + 0xb8, case.word(0)]
            })
    }
    fn ab_change(self) -> Diff {
        self.ab()
            .inputs(&[
                Input::Of(&[0x0100_0001, 0xff00_0000, 0xfeff_ffff]),
                Input::Flag,
                Input::Of(&[0, 0x10, 0xffff_ffff]),
            ])
            .setup(|e, case| {
                ab_world(e);
                let a = actor_base_form(e, 0x2a, 0, case.word(0), 0, false);
                registries(e, false, 0, false, true, a);
                let bgs = e.mem.alloc(0x300);
                e.mem.set_u32(BGS_SAVE_LOAD_GAME, bgs);
                set_answer(e, 0x0084_a620, case.word(1));
                set_answer(e, 0x0084_a740, case.word(1));
                vec![a, case.word(2)]
            })
    }
    fn ab_buffer(self) -> Diff {
        self.ab()
            .inputs(&[Input::Of(&[0, 0x20, 0xffff_ffff]), Input::Flag])
            .setup(|e, case| {
                ab_world(e);
                let a = actor_base_form(e, 0x2b, 0x1234_5678, 0x14, 0x11, case.word(1) != 0);
                let buffer = e.mem.alloc(0x20);
                e.mem.set_u32(buffer + SAVE_KIND_AT, case.word(0));
                vec![a, buffer]
            })
    }
}

#[test]
fn ab_creature_flag_20() {
    Diff::new(entry!(0x005f_0c40, actor_base::creature_flag_20(u32) -> bool))
        .ab_flags()
        .run()
        .assert_same();
}

#[test]
fn ab_is_immobile() {
    Diff::new(entry!(0x005f_0c80, actor_base::is_immobile(u32) -> bool))
        .ab_flags()
        .run()
        .assert_same();
}

#[test]
fn ab_get_sex() {
    Diff::new(entry!(0x005f_0cc0, actor_base::get_sex(u32) -> i32))
        .ab_flags()
        .run()
        .assert_same();
}

#[test]
fn ab_npc_flag_10() {
    Diff::new(entry!(0x005f_0d00, actor_base::npc_flag_10(u32) -> bool))
        .ab_flags()
        .run()
        .assert_same();
}

#[test]
fn ab_creature_flag_10_or_1() {
    Diff::new(entry!(0x005f_0b50, actor_base::creature_flag_10_or_1(u32) -> bool))
        .ab_flags()
        .run()
        .assert_same();
}

#[test]
fn ab_creature_flag_40_or_1() {
    Diff::new(entry!(0x005f_0bf0, actor_base::creature_flag_40_or_1(u32) -> bool))
        .ab_flags()
        .run()
        .assert_same();
}

#[test]
fn ab_creature_flags_allow() {
    Diff::new(entry!(0x005f_0ba0, actor_base::creature_flags_allow(u32) -> bool))
        .ab_flags()
        .ignore_calls(&[0x005f_0b50])
        .run()
        .assert_same();
}

#[test]
fn ab_starts_dead() {
    Diff::new(entry!(0x005f_0f50, actor_base::starts_dead(u32) -> bool))
        .ab()
        .inputs(&[Input::Of(&[0, 0x0008_0000, 0xfff7_ffff, 0xffff_ffff])])
        .setup(|e, case| {
            ab_world(e);
            vec![actor_base_form(
                e,
                0x2a,
                case.word(0),
                0x0100_0001,
                0,
                false,
            )]
        })
        .run()
        .assert_same();
}

#[test]
fn ab_set_starts_dead() {
    Diff::new(entry!(0x0056_5210, actor_base::set_starts_dead(u32, u8)))
        .ab()
        .inputs(&[
            Input::Of(&[0, 0x0008_0000, 0xffff_ffff]),
            Input::Of(&[0, 1, 0x80]),
        ])
        .setup(|e, case| {
            ab_world(e);
            let a = actor_base_form(e, 0x2a, case.word(0), 0x0100_0001, 0, false);
            vec![a, case.word(1)]
        })
        .run()
        .assert_same();
}

#[test]
fn ab_set_npc_flag_10() {
    Diff::new(entry!(0x005f_0d30, actor_base::set_npc_flag_10(u32, u8)))
        .ab()
        .inputs(&[
            Input::Of(AB_TYPES),
            Input::Of(&[0, 0x10, 0xffff_ffff]),
            Input::Of(&[0, 1]),
        ])
        .setup(|e, case| {
            ab_world(e);
            let a = actor_base_form(e, case.word(0), 0, 0x0100_0001, case.word(1), false);
            vec![a, case.word(2)]
        })
        .run()
        .assert_same();
}

#[test]
fn ab_attribute_5() {
    Diff::new(entry!(0x005f_11f0, actor_base::attribute_5(u32) -> u32))
        .ab_attribute_getter()
        .run()
        .assert_same();
}

#[test]
fn ab_attribute_6() {
    Diff::new(entry!(0x005f_1210, actor_base::attribute_6(u32) -> u32))
        .ab_attribute_getter()
        .run()
        .assert_same();
}

#[test]
fn ab_attribute_7() {
    Diff::new(entry!(0x005f_1230, actor_base::attribute_7(u32) -> u32))
        .ab_attribute_getter()
        .run()
        .assert_same();
}

#[test]
fn ab_attribute_8() {
    Diff::new(entry!(0x005f_1250, actor_base::attribute_8(u32) -> u32))
        .ab_attribute_getter()
        .run()
        .assert_same();
}

#[test]
fn ab_attribute_9() {
    Diff::new(entry!(0x005f_1270, actor_base::attribute_9(u32) -> u32))
        .ab_attribute_getter()
        .run()
        .assert_same();
}

#[test]
fn ab_attribute_a() {
    Diff::new(entry!(0x005f_1290, actor_base::attribute_a(u32) -> u32))
        .ab_attribute_getter()
        .run()
        .assert_same();
}

#[test]
fn ab_attribute_b() {
    Diff::new(entry!(0x005f_12b0, actor_base::attribute_b(u32) -> u32))
        .ab_attribute_getter()
        .run()
        .assert_same();
}

#[test]
fn ab_set_attribute_5() {
    Diff::new(entry!(0x005f_14b0, actor_base::set_attribute_5(u32, u8)))
        .ab_attribute_setter()
        .run()
        .assert_same();
}

#[test]
fn ab_set_attribute_6() {
    Diff::new(entry!(0x005f_14d0, actor_base::set_attribute_6(u32, u8)))
        .ab_attribute_setter()
        .run()
        .assert_same();
}

#[test]
fn ab_set_attribute_7() {
    Diff::new(entry!(0x005f_14f0, actor_base::set_attribute_7(u32, u8)))
        .ab_attribute_setter()
        .run()
        .assert_same();
}

#[test]
fn ab_set_attribute_8() {
    Diff::new(entry!(0x005f_1510, actor_base::set_attribute_8(u32, u8)))
        .ab_attribute_setter()
        .run()
        .assert_same();
}

#[test]
fn ab_set_attribute_9() {
    Diff::new(entry!(0x005f_1530, actor_base::set_attribute_9(u32, u8)))
        .ab_attribute_setter()
        .run()
        .assert_same();
}

#[test]
fn ab_set_attribute_a() {
    Diff::new(entry!(0x005f_1550, actor_base::set_attribute_a(u32, u8)))
        .ab_attribute_setter()
        .run()
        .assert_same();
}

#[test]
fn ab_set_attribute_b() {
    Diff::new(entry!(0x005f_1570, actor_base::set_attribute_b(u32, u8)))
        .ab_attribute_setter()
        .run()
        .assert_same();
}

#[test]
fn ab_get_health() {
    Diff::new(entry!(0x005f_0b00, actor_base::get_health(u32) -> u32))
        .ab()
        .inputs(&[Input::Of(&[0, 0x64, 0xffff_ffff])])
        .setup(|e, case| {
            ab_world(e);
            e.mem.set_u32(SLOT_ANSWERS + 0x10, case.word(0));
            vec![actor_base_form(e, 0x2a, 0, 0x0100_0001, 0, false)]
        })
        .run()
        .assert_same();
}

#[test]
fn ab_set_health() {
    Diff::new(entry!(0x005f_0b30, actor_base::set_health(u32, u32)))
        .ab()
        .inputs(&[Input::Of(WORDS)])
        .setup(|e, case| {
            ab_world(e);
            vec![
                actor_base_form(e, 0x2a, 0, 0x0100_0001, 0, false),
                case.word(0),
            ]
        })
        .run()
        .assert_same();
}

#[test]
fn ab_player_or_default_value() {
    Diff::new(entry!(0x005f_0f80, actor_base::player_or_default_value(u32) -> u32))
        .ab()
        .inputs(&[Input::Of(&[0, 5]), Input::Flag])
        .setup(|e, case| {
            ab_world(e);
            let a = actor_base_form(e, 0x2a, 0, 0x0100_0001, 0, false);
            e.mem.set_u32(0x011c_aff0, case.word(0));
            e.mem.set_u32(0x011c_afec, 9);
            let player = e.mem.alloc(0x80);
            let base = if case.word(1) != 0 { a } else { a + 0x10 };
            e.mem.set_u32(player + 0x20, base);
            e.mem.set_u32(PLAYER_REF, player);
            vec![a]
        })
        .run()
        .assert_same();
}

#[test]
fn ab_add_change() {
    Diff::new(entry!(0x005f_1630, actor_base::add_change(u32, u32)))
        .ab_change()
        .run()
        .assert_same();
}

#[test]
fn ab_remove_change() {
    Diff::new(entry!(0x005f_1690, actor_base::remove_change(u32, u32)))
        .ab_change()
        .run()
        .assert_same();
}

#[test]
fn ab_get_actor_value() {
    Diff::new(entry!(0x005f_0fb0, actor_base::get_actor_value(u32, u32) -> u32))
        .ab()
        .ignore_calls(AB_ATTRIBUTE_GETTERS)
        .ignore_calls(AB_ATTRIBUTE_BYTE)
        .inputs(&[
            Input::Of(&[
                0,
                1,
                2,
                3,
                4,
                5,
                6,
                7,
                8,
                9,
                0xa,
                0xb,
                0xc,
                0x10,
                0x15,
                0x16,
                0x17,
                0x39,
                0xffff_ffff,
            ]),
            // The karma: 12.75 and -7.5 truncate (to 12 and -7) where rounding would not; 12.5.
            Input::Of(&[0x414c_0000, 0xc0f0_0000, 0x4148_0000]),
        ])
        .setup(|e, case| {
            ab_world(e);
            let a = actor_base_form(e, 0x2a, 0, 0x0100_0001, 0, false);
            e.mem.set_u32(SLOT_ANSWERS + 0x10, 0x55);
            e.mem.set_u32(SLOT_ANSWERS + 0x60, 0x0001_0123);
            e.mem.set_u32(SLOT_ANSWERS + 0x64, case.word(1));
            vec![a + 0x100, case.word(0)]
        })
        .run()
        .assert_same();
}

#[test]
fn ab_set_actor_value() {
    Diff::new(entry!(
        0x005f_12d0,
        actor_base::set_actor_value(u32, u32, u32)
    ))
    .ab()
    .inputs(&[
        Input::Of(&[
            0,
            1,
            2,
            3,
            4,
            5,
            6,
            7,
            8,
            9,
            0xa,
            0xb,
            0xc,
            0x10,
            0x15,
            0x16,
            0x17,
            0xffff_ffff,
        ]),
        Input::Of(&[0, 0x1234_5678, 0xffff_fffe, 0x0100_0001]),
    ])
    .setup(|e, case| {
        ab_world(e);
        let a = actor_base_form(e, 0x2a, 0, 0x0100_0001, 0, false);
        set_cast(e, TYPE_TES_FORM, a);
        vec![a, case.word(0), case.word(1)]
    })
    .run()
    .assert_same();
}

#[test]
fn ab_get_desirability() {
    // Each item type, its cast found or not, its health or none, and a null actor base.
    Diff::new(entry!(0x005f_0d60, actor_base::get_desirability(u32, u32) -> f32))
        .ab()
        .inputs(&[
            Input::Of(&[0x29, 0x28, 0x1a, 0x18, 0x1e, 0x2a, 0xff]),
            Input::Flag,
            Input::Flag,
            Input::Of(&[0, 1, 2]),
        ])
        .setup(|e, case| {
            ab_world(e);
            let a = actor_base_form(e, 0x2a, 0, 0x0100_0001, 0, false);
            let item = e.mem.alloc(0x80);
            e.mem.set_u32(item, VTABLE);
            e.mem.set_u8(item + 4, case.word(0) as u8);
            e.mem.set_u32(item + 0xc, 0x0100_0040);
            e.mem.set_u32(item + 0x5c, 0x0abc_0001);
            let found = if case.word(1) != 0 { item } else { 0 };
            for target in [0x0118_40c4, 0x0118_3998, 0x0118_6fcc, 0x0118_3a34] {
                set_cast(e, target, found);
            }
            set_answer(e, 0x0048_73d0, case.word(2));
            e.mem.set_u32(0x0101_6410, 0x7fa0_0001);
            let this = [a, 0, a][case.word(3) as usize];
            let item = if case.word(3) == 2 { 0 } else { item };
            vec![this, item]
        })
        .run()
        .assert_same();
}

#[test]
fn ab_should_run_init_default_worn() {
    Diff::new(entry!(
        0x005f_1590,
        actor_base::should_run_init_default_worn(u32, u32) -> bool
    ))
    .ab()
    .inputs(&[
        Input::Of(&[0, 2]),
        Input::Of(&[0, 0x0020_0000]),
        Input::Flag,
        Input::Flag,
    ])
    .setup(|e, case| {
        ab_world(e);
        let a = actor_base_form(e, 0x2a, 0, 0x0100_0001, 0, false);
        let bgs = e.mem.u32(BGS_SAVE_LOAD_GAME);
        e.mem.set_u32(bgs + 0x244, case.word(0) | 0x10);
        let reference = form(e, 0x3b, case.word(1), 0x0100_0050);
        e.mem.set_u32(SLOT_ANSWERS + 0x22c, case.word(2));
        set_answer(e, 0x0084_a6d0, case.word(3));
        vec![a, reference]
    })
    .run()
    .assert_same();
}

#[test]
fn ab_save_to_buffer() {
    Diff::new(entry!(0x005f_1f30, actor_base::save_to_buffer(u32, u32)))
        .ab_buffer()
        .run()
        .assert_same();
}

#[test]
fn ab_load_from_buffer() {
    Diff::new(entry!(0x005f_1fd0, actor_base::load_from_buffer(u32, u32)))
        .ab_buffer()
        .run()
        .assert_same();
}

#[test]
fn ab_save_size() {
    Diff::new(entry!(0x005f_16f0, actor_base::save_size(u32, u32) -> u16))
        .ab()
        .inputs(&[
            Input::Flag,
            Input::Flag,
            Input::Flag,
            Input::Flag,
            Input::Of(&[0, 0x20, 0x22, 0xffff_ffff]),
            Input::Flag,
        ])
        .setup(|e, case| {
            let a = ab_save_setup(e, case, case.word(5) != 0);
            vec![a, case.word(4)]
        })
        .run()
        .assert_same();
}

#[test]
fn ab_save_game() {
    Diff::new(entry!(0x005f_18c0, actor_base::save_game(u32, u32)))
        .ab()
        .inputs(&[
            Input::Flag,
            Input::Flag,
            Input::Flag,
            Input::Flag,
            Input::Of(&[0, 0x20, 0x22]),
            Input::Flag,
        ])
        .setup(|e, case| {
            let a = ab_save_setup(e, case, case.word(5) != 0);
            vec![a, case.word(4)]
        })
        .run()
        .assert_same();
}

#[test]
fn ab_save_game_block_too_big() {
    // A spell list save that leaves the position 0xffff or 0x10000 bytes past the block's size
    // placeholder (4 bytes into the stream, after the tag): no warning, then the block's warning.
    Diff::new(entry!(0x005f_18c0, actor_base::save_game(u32, u32)))
        .ab()
        .inputs(&[
            Input::Of(&[1]),
            Input::Flag,
            Input::Of(&[0]),
            Input::Flag,
            Input::Of(&[0xffff, 0x1_0000]),
        ])
        .setup(|e, case| {
            let a = ab_save_setup(e, case, false);
            let past = case.word(4);
            e.register_double(0x0048_da20, move |e, _| {
                let save_load = e.mem.u32(SAVE_LOAD_GAME);
                e.mem.set_u32(save_load + SL_POSITION, STREAM + 4 + past);
                ret(0)
            });
            e.map(STREAM + 0x1_0000, 0x1000);
            vec![a, 0]
        })
        .run()
        .assert_same();
}

#[test]
fn ab_load_game() {
    // The stream: the block's tag right or wrong; its size short, exact or long; the name's
    // length byte and characters where the reads reach them (version 0x53).
    Diff::new(entry!(0x005f_1b30, actor_base::load_game(u32, u32, u32)))
        .ab()
        .inputs(&[
            Input::Flag,
            Input::Flag,
            Input::Flag,
            Input::Flag,
            Input::Of(&[0, 0x20]),
            Input::Flag,
            Input::Of(&[0, 1, 2]),
        ])
        .setup(|e, case| {
            let a = ab_save_setup(e, case, false);
            let blocks = case.word(0) != 0;
            let named = case.word(4) != 0;
            if blocks && case.word(5) != 0 {
                e.mem.set_u32(STREAM, 0x424c_4f4b);
            }
            let header = if blocks { 6 } else { 0 };
            let data = if blocks { 8 } else { 2 };
            let name_at = header + data;
            if named {
                e.mem.set_u8(STREAM + name_at, 3);
                for (i, b) in b"Xyz".iter().enumerate() {
                    e.mem.set_u8(STREAM + name_at + 1 + i as u32, *b);
                }
            }
            if blocks {
                let exact = 2 + data + if named { 4 } else { 0 };
                let size = [exact - 1, exact, exact + 5][case.word(6) as usize];
                e.mem.set_u16(STREAM + 4, size as u16);
            }
            vec![a, case.word(4), 0x77]
        })
        .run()
        .assert_same();
}

#[test]
fn ab_reset_3d() {
    Diff::new(entry!(0x005f_20a0, actor_base::reset_3d(u32, u8)))
        .ab()
        .ignore_calls(&[0x005f_2150])
        .inputs(&[Input::Of(&[0, 1, 0x80]), Input::Flag, Input::Flag])
        .setup(|e, case| {
            ab_world(e);
            let a = actor_base_form(e, 0x2a, 0, 0x0100_0001, 0, false);
            let player = e.mem.alloc(0x80);
            let base = if case.word(1) != 0 { a } else { a + 0x10 };
            e.mem.set_u32(player + 0x20, base);
            if case.word(2) != 0 {
                let process = form(e, 0, 0, 0);
                e.mem.set_u32(player + 0x68, process);
            }
            e.mem.set_u32(PLAYER_REF, player);
            vec![a, case.word(0)]
        })
        .run()
        .assert_same();
}

#[test]
fn ab_construct_reset_3d_iterator() {
    Diff::new(entry!(0x005f_2150, actor_base::construct_reset_3d_iterator(u32) -> u32))
        .ab()
        .setup(|e, _| {
            ab_world(e);
            vec![data(e, 0x10)]
        })
        .run()
        .assert_same();
}

#[test]
fn ab_reset_3d_operate() {
    Diff::new(entry!(0x005f_2170, actor_base::reset_3d_operate(u32, u32)))
        .ab()
        .inputs(&[Input::Flag, Input::Flag])
        .setup(|e, case| {
            ab_world(e);
            let iterator = data(e, 0x10);
            let reference = e.mem.alloc(0x80);
            e.mem.set_u32(iterator + 8, 0x0100_7000);
            let base = if case.word(0) != 0 {
                0x0100_7000
            } else {
                0x0100_7001
            };
            e.mem.set_u32(reference + 0x20, base);
            if case.word(1) != 0 {
                let process = form(e, 0, 0, 0);
                e.mem.set_u32(reference + 0x68, process);
            }
            vec![iterator, reference]
        })
        .run()
        .assert_same();
}

/// An attributes component: the test vtable and seven bytes from `seed`.
fn attributes_component(e: &mut Engine, seed: u32) -> u32 {
    let c = e.mem.alloc(0x10);
    e.mem.set_u32(c, VTABLE);
    for i in 0..7 {
        e.mem.set_u8(c + 4 + i, (seed * 0x1d + 3 * i) as u8);
    }
    c
}

#[test]
fn attributes_load_data() {
    Diff::new(entry!(0x0048_0130, attributes::load_data(u32, u32) -> u32))
        .ab()
        .setup(|e, _| {
            ab_world(e);
            vec![attributes_component(e, 1), data(e, 0x10)]
        })
        .run()
        .assert_same();
}

#[test]
fn attributes_save_data() {
    Diff::new(entry!(0x0048_0100, attributes::save_data(u32, u32) -> u32))
        .ab()
        .setup(|e, _| {
            ab_world(e);
            vec![attributes_component(e, 2), data(e, 0x10)]
        })
        .run()
        .assert_same();
}

#[test]
fn attributes_initialize_data_component() {
    Diff::new(entry!(
        0x0047_ffc0,
        attributes::initialize_data_component(u32)
    ))
    .ab()
    .setup(|e, _| {
        ab_world(e);
        vec![attributes_component(e, 3)]
    })
    .run()
    .assert_same();
}

#[test]
fn attributes_copy_component() {
    Diff::new(entry!(0x0048_0000, attributes::copy_component(u32, u32)))
        .ab()
        .ignore_calls(AB_ATTRIBUTE_BYTE)
        .inputs(&[Input::Flag])
        .setup(|e, case| {
            ab_world(e);
            let (this, source) = (attributes_component(e, 4), attributes_component(e, 5));
            let cast = if case.word(0) != 0 { source } else { 0 };
            set_cast(e, 0x0118_6c74, cast);
            vec![this, source]
        })
        .run()
        .assert_same();
}

#[test]
fn attributes_compare_component() {
    // Not an attributes component; the same bytes; one differing byte (the first or the last).
    Diff::new(entry!(0x0048_0070, attributes::compare_component(u32, u32) -> bool))
        .ab()
        .ignore_calls(AB_ATTRIBUTE_BYTE)
        .inputs(&[Input::Of(&[0, 1, 2, 3])])
        .setup(|e, case| {
            ab_world(e);
            let (this, other) = (attributes_component(e, 6), attributes_component(e, 6));
            match case.word(0) {
                2 => e.mem.set_u8(other + 4, 0xee),
                3 => e.mem.set_u8(other + 0xa, 0xee),
                _ => {}
            }
            let cast = if case.word(0) != 0 { other } else { 0 };
            set_cast(e, 0x0118_6c74, cast);
            vec![this, other]
        })
        .run()
        .assert_same();
}

#[test]
fn attributes_attribute() {
    Diff::new(entry!(0x0048_0160, attributes::attribute(u32, u32) -> u32))
        .ab()
        .inputs(&[Input::Of(&[1, 5, 8, 0xb, 0xfc, 0x100])])
        .setup(|e, case| {
            ab_world(e);
            vec![attributes_component(e, 7), case.word(0)]
        })
        .run()
        .assert_same();
}

#[test]
fn attributes_set_attribute_value() {
    Diff::new(entry!(
        0x0048_0180,
        attributes::set_attribute_value(u32, u32, u8, u8)
    ))
    .ab()
    .inputs(&[
        Input::Of(&[1, 5, 0xb]),
        Input::Of(&[0, 0x99]),
        Input::Of(&[0, 1, 0x80]),
        Input::Flag,
        // The form's slot 0x48: a stand-in, `TESActorBase::AddChange` or `TESForm::AddChange`.
        Input::Of(&[0, 1, 2]),
        // A loaded or a created form.
        Input::Of(&[0x0100_0060, 0xff00_0010]),
    ])
    .setup(|e, case| {
        ab_world(e);
        let this = attributes_component(e, 8);
        let form = actor_base_form(e, 0x2a, 0, case.word(5), 0, false);
        if case.word(4) != 0 {
            let vtable = e.mem.alloc(0x50);
            let add_change = if case.word(4) == 1 {
                0x005f_1630
            } else {
                0x0048_4b60
            };
            e.mem.set_u32(vtable + 0x48, add_change);
            e.mem.set_u32(form, vtable);
        }
        set_cast(e, TYPE_TES_FORM, if case.word(3) != 0 { form } else { 0 });
        vec![this, case.word(0), case.word(1), case.word(2)]
    })
    .run()
    .assert_same();
}

#[test]
fn attributes_save_size() {
    Diff::new(entry!(0x0048_01e0, attributes::save_size(u32, u32) -> u16))
        .ab()
        .inputs(&[Input::Of(FLAGS)])
        .setup(|e, case| {
            ab_world(e);
            vec![attributes_component(e, 9), case.word(0)]
        })
        .run()
        .assert_same();
}

#[test]
fn attributes_save_game() {
    Diff::new(entry!(0x0048_0210, attributes::save_game(u32, u32)))
        .ab()
        .inputs(&[Input::Of(FLAGS)])
        .setup(|e, case| {
            ab_world(e);
            vec![attributes_component(e, 9), case.word(0)]
        })
        .run()
        .assert_same();
}

#[test]
fn attributes_load_game() {
    Diff::new(entry!(0x0048_0240, attributes::load_game(u32, u32, u32)))
        .ab()
        .inputs(&[Input::Of(FLAGS)])
        .setup(|e, case| {
            ab_world(e);
            vec![attributes_component(e, 9), case.word(0), 0x77]
        })
        .run()
        .assert_same();
}

#[test]
fn attributes_save_to_buffer() {
    Diff::new(entry!(0x0048_0270, attributes::save_to_buffer(u32, u32)))
        .ab()
        .inputs(&[Input::Of(&[0, 4, 0xffff_fffb, 0xffff_ffff])])
        .setup(|e, case| {
            ab_world(e);
            let buffer = e.mem.alloc(0x20);
            e.mem.set_u32(buffer + SAVE_KIND_AT, case.word(0));
            vec![attributes_component(e, 10), buffer]
        })
        .run()
        .assert_same();
}

#[test]
fn attributes_load_from_buffer() {
    Diff::new(entry!(0x0048_02b0, attributes::load_from_buffer(u32, u32)))
        .ab()
        .inputs(&[Input::Of(&[0, 4, 0xffff_fffb, 0xffff_ffff])])
        .setup(|e, case| {
            ab_world(e);
            let buffer = e.mem.alloc(0x20);
            e.mem.set_u32(buffer + SAVE_KIND_AT, case.word(0));
            vec![attributes_component(e, 11), buffer]
        })
        .run()
        .assert_same();
}

#[test]
fn attributes_construct_biped_model_form() {
    Diff::new(entry!(0x0048_02f0, attributes::construct_biped_model_form(u32) -> u32))
        .ab()
        .setup(|e, _| {
            ab_world(e);
            vec![data(e, 0xdc)]
        })
        .run()
        .assert_same();
}

#[test]
fn attributes_construct_model_rdt_at() {
    Diff::new(entry!(0x0048_03e0, attributes::construct_model_rdt_at(u32) -> u32))
        .ab()
        .setup(|e, _| {
            ab_world(e);
            vec![data(e, 0x18)]
        })
        .run()
        .assert_same();
}

#[test]
fn attributes_destruct_biped_model_form() {
    Diff::new(entry!(
        0x0048_0400,
        attributes::destruct_biped_model_form(u32)
    ))
    .ab()
    .setup(|e, _| {
        ab_world(e);
        vec![data(e, 0xdc)]
    })
    .run()
    .assert_same();
}

// ---- TESAIForm (`ai_form`) and TESAnimation (`animation`) ----

/// The words the AI form's and the animation's callees by address answer, one each from
/// [`AI_ANSWERS`]: the equip type and a file's chunk type. (The package list's operations are
/// this family's: their own code runs, over the package world.)
const AI_ANSWERS: u32 = 0x0300_7000;
const AI_ANSWERED: [u32; 2] = [
    0x0047_9430, // the equip type
    0x0047_26b0, // a file's chunk type
];
/// The animation's vtable: slot 4 (`ClearDataComponent`) holds the class's own function or a
/// stand-in that answers a word.
const ANIM_VTABLE: u32 = 0x0300_7100;
/// The bytes the file's chunk read gives, as many as [`AI_CHUNK_SIZE`] says.
const AI_CHUNK: u32 = 0x0300_7200;
const AI_CHUNK_SIZE: u32 = 0x0300_72f0;
const TYPE_TES_AI_FORM: u32 = 0x0118_6cd4;
const TYPE_TES_ANIMATION: u32 = 0x0118_584c;
const AI_FORM_VTABLE: u32 = 0x0101_be98;
const KFFZ: u32 = 0x5a46_464b;
/// The five copied settings (setting, copy), their guard byte, the byte setting `InitItem` reads.
const AI_SETTINGS: [(u32, u32); 5] = [
    (0x011c_533c, 0x011c_52a8),
    (0x011c_5320, 0x011c_52ac),
    (0x011c_52fc, 0x011c_52b0),
    (0x011c_530c, 0x011c_52b4),
    (0x011c_52e4, 0x011c_52b8),
];
const AI_SETTINGS_COPIED: u32 = 0x011c_52c0;
const AI_INIT_ITEM_SETTING: u32 = 0x011c_5348;

/// The one-line bodies of other classes the AI form's and the animation's native code does in
/// place (see their module comments): left out of the translations' calls, with no double, as
/// [`IN_PLACE`] (which has the form type, the base component's and the empty list's constructors,
/// the aggression getter, a file's chunk size and a save buffer's change flags).
const AI_IN_PLACE: &[u32] = &[
    0x004f_6de0, // the confidence
    0x005b_b4d0, // the energy
    0x0062_f2f0, // the mood
    0x0070_5cf0, // the aggro radius flag
    0x0082_5c00, // the aggro radius
    0x0084_e3a0, // the service flags
    0x0082_4060, // the training level
    0x0050_0940, // the package list's address
    0x0068_0890, // its constructor
    0x0040_3e20, // a float setting's value pointer
    0x0040_8d60, // a byte setting's
    0x0041_81e0, // a reference's base form
    0x0082_56d0, // a list's empty test
    0x0068_15c0, // a node's item slot
    0x0072_6070, // its next node
];
/// The family's own getters its callers do in place: listed only in their callers' tests.
const AI_OWN_IN_PLACE: &[u32] = &[
    0x0047_ee30, // the assistance
    0x0047_ef30, // the responsibility
    0x0047_f000, // the training skill
    0x0047_efe0, // the service flags' test
    0x0047_f060, // the training skill's range
];

fn ai_answer_at(addr: u32) -> u32 {
    let index = AI_ANSWERED
        .iter()
        .position(|a| *a == addr)
        .expect("an answered callee");
    AI_ANSWERS + 4 * index as u32
}

fn ai_answer<const A: u32>(e: &mut Engine, _: &[u32]) -> Ret {
    ret(e.mem.u32(ai_answer_at(A)))
}

fn set_ai_answer(e: &mut Engine, addr: u32, value: u32) {
    e.mem.set_u32(ai_answer_at(addr), value);
}

macro_rules! ai_answered {
    ($e:expr, $($addr:expr),* $(,)?) => {
        $( $e.register($addr, ai_answer::<{ $addr }>); )*
    };
}

/// The package world plus what the AI form and the animation reach: their callees' answers, the
/// slot stand-in, a chunk read of the case's bytes and the save/load game.
fn ai_world(e: &mut Engine) {
    package_world(e);
    ai_answered!(e, 0x0047_9430, 0x0047_26b0);
    e.register(VT_FN + 4, slot_answer::<4>);
    e.register(0x0047_2890, |e, a| {
        let size = e.mem.u32(AI_CHUNK_SIZE);
        copy_bytes(e, a[1], AI_CHUNK, size);
        ret(1)
    });
    let save_load = e.mem.alloc(0x20);
    e.mem.set_u32(SAVE_LOAD_GAME, save_load);
}

/// An actor base form (the test vtable) and its `TESAIForm` at +0x90, returned: the class's vtable,
/// then the AI data's bytes made from `seed`, and a package list of none, one or two packages.
fn ai_form(e: &mut Engine, seed: u32) -> u32 {
    let owner = e.mem.alloc(0xc0);
    e.mem.set_u32(owner, VTABLE);
    let ai = owner + 0x90;
    e.mem.set_u32(ai, AI_FORM_VTABLE);
    for i in 4..0x20 {
        e.mem
            .set_u8(ai + i, seed.wrapping_mul(0x3b).wrapping_add(i * 0x11) as u8);
    }
    let items = packages(e, seed, seed % 3, false);
    set_list(e, ai + 0x18, &items);
    ai
}

/// The ranged fields a case picks: the mood and confidence below their counts and a training skill
/// that is a skill (`low`), or each past it.
fn set_ai_ranges(e: &mut Engine, ai: u32, low: bool) {
    let (mood, confidence, skill) = if low { (3, 2, 5) } else { (9, 6, 0x30) };
    e.mem.set_u8(ai + 8, mood);
    e.mem.set_u8(ai + 5, confidence);
    e.mem.set_u8(ai + 0x10, skill);
}

/// A C string of the test's own.
fn c_string(e: &mut Engine, text: &[u8]) -> u32 {
    let s = e.mem.alloc(text.len() as u32 + 1);
    for (i, b) in text.iter().enumerate() {
        e.mem.set_u8(s + i as u32, *b);
    }
    s
}

/// An animation component on [`ANIM_VTABLE`], its name list holding `names` (a `None` is a node
/// with no name; further nodes are blocks of their own).
fn animation(e: &mut Engine, names: &[Option<&str>]) -> u32 {
    let a = e.mem.alloc(0xc);
    e.mem.set_u32(a, ANIM_VTABLE);
    let mut node = a + 4;
    for (i, name) in names.iter().enumerate() {
        if i > 0 {
            let next = e.mem.alloc(8);
            e.mem.set_u32(node + 4, next);
            node = next;
        }
        let text = name.map_or(0, |n| c_string(e, n.as_bytes()));
        e.mem.set_u32(node, text);
    }
    a
}

/// The name lists the animation's cases pick from.
const NAME_LISTS: [&[Option<&str>]; 5] = [
    &[],
    &[Some("walk.kf")],
    &[Some("walk.kf"), Some("Run.KF"), Some("idle.kf")],
    &[Some("walk.kf"), None, Some("idle.kf")],
    &[None, Some("jump.kf")],
];

/// The AI form's and the animation's cases: the forms family's, with their own in-place bodies.
fn ai_cases(d: Diff) -> Diff {
    pk_locals(
        d.forms()
            .ignore_calls(AI_IN_PLACE)
            .ignore_calls(PK_IN_PLACE)
            .ignore_calls(PK_OWN_IN_PLACE)
            .goes_by_address(PK_BY_ADDRESS),
    )
}

/// The AI form's and the animation's shared cases, as steps of the `Diff` chain.
trait AiCases {
    fn ai(self) -> Diff;
    fn ai_caller(self) -> Diff;
    fn ai_byte_setter(self) -> Diff;
    fn ai_setter(self, values: &'static [u32]) -> Diff;
    fn ai_getter(self) -> Diff;
    fn ai_saves(self, words: usize) -> Diff;
    fn ai_buffer(self) -> Diff;
    fn ai_reference(self) -> Diff;
    fn names(self) -> Diff;
}

impl AiCases for Diff {
    fn ai(self) -> Diff {
        ai_cases(self)
    }
    fn ai_caller(self) -> Diff {
        ai_cases(self).ignore_calls(AI_OWN_IN_PLACE)
    }
    fn ai_byte_setter(self) -> Diff {
        ai_byte_setter_cases(self)
    }
    fn ai_setter(self, values: &'static [u32]) -> Diff {
        ai_setter_cases(self, values)
    }
    fn ai_getter(self) -> Diff {
        ai_getter_cases(self)
    }
    fn ai_saves(self, words: usize) -> Diff {
        ai_saves_cases(self, words)
    }
    fn ai_buffer(self) -> Diff {
        ai_buffer_cases(self)
    }
    fn ai_reference(self) -> Diff {
        ai_reference_cases(self)
    }
    fn names(self) -> Diff {
        names_cases(self)
    }
}

/// A byte setter with a notify flag: a value (its low byte stored) and the flag (its low byte
/// tested), on components from two seeds.
fn ai_byte_setter_cases(d: Diff) -> Diff {
    d.ai()
        .inputs(&[
            Input::Of(&[0, 1]),
            Input::Of(&[0, 0x7f, 0xff, 0x1_23]),
            Input::Of(&[0, 1, 0x100, 0xffff_ffff]),
        ])
        .setup(|e, case| {
            ai_world(e);
            vec![ai_form(e, case.word(0)), case.word(1), case.word(2)]
        })
}

/// A setter of one value, on components from two seeds.
fn ai_setter_cases(d: Diff, values: &'static [u32]) -> Diff {
    d.ai()
        .inputs(&[Input::Of(&[0, 1]), Input::Of(values)])
        .setup(|e, case| {
            ai_world(e);
            vec![ai_form(e, case.word(0)), case.word(1)]
        })
}

/// A getter: components from a few seeds.
fn ai_getter_cases(d: Diff) -> Diff {
    d.ai()
        .inputs(&[Input::Of(&[0, 1, 0x7f, 0xff])])
        .setup(|e, case| {
            ai_world(e);
            vec![ai_form(e, case.word(0))]
        })
}

/// The save game functions: the change flags (with flag 8 or not) and, for the load (`words` 3),
/// the word it does not read.
fn ai_saves_cases(d: Diff, words: usize) -> Diff {
    d.ai()
        .inputs(&[Input::Of(&[0, 8, 7, 0xffff_ffff]), Input::Of(&[0, 0x55])])
        .setup(move |e, case| {
            ai_world(e);
            e.mem.set_u32(READ_WORD, 0x1234_5678);
            let mut call = vec![ai_form(e, 3), case.word(0), case.word(1)];
            call.truncate(words);
            call
        })
}

/// The save buffer functions: the buffer's change flags (its word at +0x17).
fn ai_buffer_cases(d: Diff) -> Diff {
    d.ai()
        .inputs(&[Input::Of(&[0, 8, 7, 0xffff_ffff])])
        .setup(|e, case| {
            ai_world(e);
            e.mem.set_u32(READ_WORD, 0x1234_5678);
            let buffer = e.mem.alloc(0x20);
            e.mem.set_u32(buffer + 0x17, case.word(0));
            vec![ai_form(e, 5), buffer]
        })
}

/// The package queries' references: none, one whose base form is none, one with a base form; the
/// list's answer.
fn ai_reference_cases(d: Diff) -> Diff {
    d.ai()
        .inputs(&[Input::Of(&[0, 1, 2]), Input::Of(&[0, 0x77])])
}

/// The reference a package query's case picks (0, a reference with no base form, or one with an
/// actor base form at +0x20).
fn ai_reference(e: &mut Engine, pick: u32) -> u32 {
    if pick == 0 {
        return 0;
    }
    let reference = e.mem.alloc(0x80);
    let process = process(e, 0, 0, reference);
    e.mem.set_u32(reference + REF_PROCESS, process);
    if pick == 2 {
        let base = ai_form(e, 7) - 0x90;
        e.mem.set_u32(reference + 0x20, base);
    }
    reference
}

/// The animation's name cases: one of [`NAME_LISTS`] and the case-blind compare's answer (0: the
/// same name). The list's append is handed a local's address, which the typed side may place
/// elsewhere.
fn names_cases(d: Diff) -> Diff {
    d.ai()
        .local_at(0x0090_5820, 1, 4)
        .inputs(&[Input::Of(&[0, 1, 2, 3, 4]), Input::Of(&[0, 1, 0xffff_ffff])])
}

#[test]
fn ai_clear_data_component() {
    Diff::new(entry!(0x0047_ede0, ai_form::clear_data_component(u32)))
        .ai_getter()
        .run()
        .assert_same();
}

#[test]
fn ai_set_aggression() {
    Diff::new(entry!(0x0047_ee00, ai_form::set_aggression(u32, u8, u8)))
        .ai_byte_setter()
        .run()
        .assert_same();
}

#[test]
fn ai_set_confidence() {
    Diff::new(entry!(0x0047_eed0, ai_form::set_confidence(u32, u8, u8)))
        .ai_byte_setter()
        .run()
        .assert_same();
}

#[test]
fn ai_set_energy() {
    Diff::new(entry!(0x0047_ef00, ai_form::set_energy(u32, u8, u8)))
        .ai_byte_setter()
        .run()
        .assert_same();
}

#[test]
fn ai_set_responsibility() {
    Diff::new(entry!(
        0x0047_ef50,
        ai_form::set_responsibility(u32, u8, u8)
    ))
    .ai_byte_setter()
    .run()
    .assert_same();
}

#[test]
fn ai_is_skill() {
    Diff::new(entry!(0x0047_f060, ai_form::is_skill(i32) -> bool))
        .ai()
        .inputs(&[Input::Of(&[
            0,
            0x1f,
            0x20,
            0x27,
            0x2d,
            0x2e,
            0x7fff_ffff,
            0x8000_0000,
            0x8000_0020,
            0xffff_ffff,
        ])])
        .run()
        .assert_same();
}

#[test]
fn ai_assistance() {
    Diff::new(entry!(0x0047_ee30, ai_form::assistance(u32) -> u8))
        .ai_getter()
        .run()
        .assert_same();
}

#[test]
fn ai_responsibility() {
    Diff::new(entry!(0x0047_ef30, ai_form::responsibility(u32) -> u8))
        .ai_getter()
        .run()
        .assert_same();
}

#[test]
fn ai_mark_changed() {
    Diff::new(entry!(0x0047_f790, ai_form::mark_changed(u32, u32)))
        .ai()
        // `TESActorBase::AddChange` and `TESForm::AddChange` run in some cases: their in-place bodies.
        .ignore_calls(AB_IN_PLACE)
        .ignore_calls(ABD_IN_PLACE)
        .inputs(&[Input::Of(&[0, 1, 2, 3, 4, 5]), Input::Of(&[8, 0xffff_ffff])])
        .setup(|e, case| {
            ai_world(e);
            answered!(e, 0x0084_a620);
            set_answer(e, 0x0084_a620, 1);
            let bgs = e.mem.alloc(0x40);
            e.mem.set_u32(BGS_SAVE_LOAD_GAME, bgs);
            let this = match case.word(0) {
                0 => 0,
                // The component's own address with its form at 0: no form to mark.
                1 => 0x90,
                // A form whose slot 0x48 is a stand-in.
                2 => ai_form(e, 1),
                // `TESActorBase::AddChange` in the slot, for a loaded and a created form, and
                // `TESForm::AddChange`.
                pick => {
                    let ai = ai_form(e, 1);
                    let owner = ai - 0x90;
                    let vtable = e.mem.alloc(0x50);
                    let (body, id) = match pick {
                        3 => (0x005f_1630, 0x0100_0042),
                        4 => (0x005f_1630, 0xff00_0042),
                        _ => (0x0048_4b60, 0x0100_0042),
                    };
                    e.mem.set_u32(vtable + 0x48, body);
                    e.mem.set_u32(owner, vtable);
                    e.mem.set_u32(owner + 0xc, id);
                    ai
                }
            };
            vec![this, case.word(1)]
        })
        .run()
        .assert_same();
}

#[test]
fn ai_set_assistance() {
    Diff::new(entry!(0x0047_ee50, ai_form::set_assistance(u32, u8)))
        .ai_setter(&[0, 0x7f, 0xff, 0x1_23])
        .run()
        .assert_same();
}

#[test]
fn ai_set_aggro_radius_flag() {
    Diff::new(entry!(0x0047_ee70, ai_form::set_aggro_radius_flag(u32, u8)))
        .ai_setter(&[0, 1, 0xff, 0x1_00])
        .run()
        .assert_same();
}

#[test]
fn ai_set_aggro_radius() {
    Diff::new(entry!(0x0047_ee90, ai_form::set_aggro_radius(u32, u32)))
        .ai_setter(WORDS)
        .run()
        .assert_same();
}

#[test]
fn ai_set_mood() {
    Diff::new(entry!(0x0047_ef80, ai_form::set_mood(u32, u8)))
        .ai_setter(&[0, 3, 7, 8, 9, 0xff, 0x1_07, 0x1_08])
        .run()
        .assert_same();
}

#[test]
fn ai_set_service_flags() {
    Diff::new(entry!(
        0x0047_efb0,
        ai_form::set_service_flags(u32, u32, u8)
    ))
    .ai()
    .inputs(&[Input::Of(FLAGS), Input::Of(&[0, 1, 0x100])])
    .setup(|e, case| {
        ai_world(e);
        vec![ai_form(e, 2), case.word(0), case.word(1)]
    })
    .run()
    .assert_same();
}

#[test]
fn ai_training_skill() {
    Diff::new(entry!(0x0047_f000, ai_form::training_skill(u32) -> u32))
        .ai_getter()
        .run()
        .assert_same();
}

#[test]
fn ai_set_training_skill() {
    Diff::new(entry!(0x0047_f020, ai_form::set_training_skill(u32, u32)))
        .ai_caller()
        .inputs(&[
            Input::Of(&[0, 1]),
            Input::Of(&[0, 0x1f, 0x20, 0x2a, 0x2d, 0x2e, 0x8000_0020, 0xffff_ffff]),
        ])
        .setup(|e, case| {
            ai_world(e);
            vec![ai_form(e, case.word(0)), case.word(1)]
        })
        .run()
        .assert_same();
}

#[test]
fn ai_set_training_level() {
    Diff::new(entry!(0x0047_f090, ai_form::set_training_level(u32, u8)))
        .ai_setter(&[0, 0x64, 0xff, 0x1_01])
        .run()
        .assert_same();
}

/// The copied settings' cases: copied before or not, and their floats (one set with a signalling
/// NaN, an infinity and a negative zero).
fn set_ai_settings(e: &mut Engine, copied: u32, odd: u32) {
    e.mem.set_u8(AI_SETTINGS_COPIED, copied as u8);
    let floats: [u32; 5] = if odd == 0 {
        [0x3fc0_0000, 0xc000_0000, 0x42c8_0000, 0x3e80_0000, 0]
    } else {
        [
            0x7f80_0001,
            0xff80_0000,
            0x8000_0000,
            0x7fc0_0002,
            0x0000_0001,
        ]
    };
    for ((setting, copy), bits) in AI_SETTINGS.into_iter().zip(floats) {
        e.mem.set_u32(setting + 4, bits);
        e.mem.set_u32(copy, 0x1111_1111);
    }
}

#[test]
fn ai_initialize_data_component() {
    Diff::new(entry!(0x0047_ed10, ai_form::initialize_data_component(u32)))
        .ai()
        .inputs(&[Input::Of(&[0, 1, 0x100]), Input::Of(&[0, 1])])
        .setup(|e, case| {
            ai_world(e);
            set_ai_settings(e, case.word(0), case.word(1));
            vec![ai_form(e, 4)]
        })
        .run()
        .assert_same();
}

#[test]
fn ai_construct() {
    Diff::new(entry!(0x0047_ec40, ai_form::construct(u32) -> u32))
        .ai()
        .inputs(&[Input::Of(&[0, 1]), Input::Of(&[0, 1])])
        .setup(|e, case| {
            ai_world(e);
            set_ai_settings(e, case.word(0), case.word(1));
            vec![ai_form(e, 6)]
        })
        .run()
        .assert_same();
}

#[test]
fn ai_destruct() {
    Diff::new(entry!(0x0047_ecb0, ai_form::destruct(u32)))
        .ai_getter()
        .run()
        .assert_same();
}

#[test]
fn ai_load_data() {
    Diff::new(entry!(0x0047_f0b0, ai_form::load_data(u32, u32)))
        .ai()
        .inputs(&[Input::Of(&[0, 1, 2]), Input::Of(&[0, 1])])
        .setup(|e, case| {
            ai_world(e);
            let this = ai_form(e, case.word(1));
            let record = if case.word(0) == 0 {
                0
            } else {
                // A record: its bytes made up, the confidence and mood in range or not.
                let record = data(e, 0x14);
                let low = case.word(0) == 1;
                e.mem.set_u8(record + 1, if low { 4 } else { 5 });
                e.mem.set_u8(record + 4, if low { 7 } else { 8 });
                record
            };
            vec![this, record]
        })
        .run()
        .assert_same();
}

#[test]
fn ai_copy_ai_data() {
    Diff::new(entry!(0x0047_f1e0, ai_form::copy_ai_data(u32, u32)))
        .ai_caller()
        .inputs(&[Input::Of(&[0, 1, 2])])
        .setup(|e, case| {
            ai_world(e);
            let this = ai_form(e, 1);
            // Another component with its ranged fields in range or not, or this one itself.
            let other = match case.word(0) {
                2 => this,
                pick => {
                    let other = ai_form(e, 9);
                    set_ai_ranges(e, other, pick == 0);
                    other
                }
            };
            vec![this, other]
        })
        .run()
        .assert_same();
}

#[test]
fn ai_copy_packages() {
    Diff::new(entry!(0x0047_f2d0, ai_form::copy_packages(u32, u32)))
        .ai()
        .inputs(&[Input::Of(&[0, 1])])
        .setup(|e, case| {
            ai_world(e);
            vec![ai_form(e, case.word(0)), ai_form(e, 8)]
        })
        .run()
        .assert_same();
}

#[test]
fn ai_copy_component() {
    Diff::new(entry!(0x0047_f190, ai_form::copy_component(u32, u32)))
        .ai_caller()
        .inputs(&[Input::Of(&[0, 1, 2])])
        .setup(|e, case| {
            ai_world(e);
            let this = ai_form(e, 1);
            let other = ai_form(e, 0x22);
            set_ai_ranges(e, other, case.word(0) == 1);
            set_cast(
                e,
                TYPE_TES_AI_FORM,
                if case.word(0) == 0 { 0 } else { other },
            );
            vec![this, other]
        })
        .run()
        .assert_same();
}

#[test]
fn ai_add_package() {
    Diff::new(entry!(0x0047_f500, ai_form::add_package(u32, u32)))
        .ai_setter(WORDS)
        .run()
        .assert_same();
}

#[test]
fn ai_init_item() {
    Diff::new(entry!(0x0047_f4c0, ai_form::init_item(u32, u32)))
        .ai()
        .inputs(&[Input::Of(&[0, 1, 0x80]), Input::Of(&[0, 0x1234])])
        .setup(|e, case| {
            ai_world(e);
            e.mem.set_u8(AI_INIT_ITEM_SETTING + 4, case.word(0) as u8);
            let owner = if case.word(1) != 0 {
                form(e, 0x2a, 0, 0x0100_0abc)
            } else {
                0
            };
            vec![ai_form(e, 4), owner]
        })
        .run()
        .assert_same();
}

#[test]
fn ai_current_package() {
    Diff::new(entry!(0x0047_f520, ai_form::current_package(u32) -> u32))
        .ai_reference()
        .setup(|e, case| {
            ai_world(e);
            set_calendar(e, case.word(1));
            vec![ai_reference(e, case.word(0))]
        })
        .run()
        .assert_same();
}

#[test]
fn ai_missed_packages() {
    Diff::new(entry!(
        0x0047_f590,
        ai_form::missed_packages(u32, u32, f32, f32)
    ))
    .ai_reference()
    .inputs(&[
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0, 0x77]),
        Input::Of(&[0x3fc0_0000, 0x7f80_0001, 0x8000_0000]),
        Input::Of(&[0xc2c8_0000, 0xff80_0001, 0x7f80_0000]),
    ])
    .setup(|e, case| {
        ai_world(e);
        vec![
            ai_reference(e, case.word(0)),
            if case.word(1) != 0 {
                new_list(e, &[])
            } else {
                0
            },
            case.word(2),
            case.word(3),
        ]
    })
    .run()
    .assert_same();
}

#[test]
fn ai_copied_setting() {
    Diff::new(entry!(0x0047_eeb0, ai_form::copied_setting(u8) -> f32))
        .ai()
        .inputs(&[Input::Of(&[0, 1, 2, 4, 5, 0xff, 0x102])])
        .setup(|e, case| {
            ai_world(e);
            for (i, bits) in [0x3fc0_0000u32, 0xc120_0000, 0x7f80_0001, 0, 0x8000_0000]
                .into_iter()
                .enumerate()
            {
                e.mem.set_u32(0x011c_52a8 + 4 * i as u32, bits);
            }
            vec![case.word(0)]
        })
        .run()
        .assert_same();
}

#[test]
fn ai_has_service() {
    Diff::new(entry!(0x0047_efe0, ai_form::has_service(u32, u32) -> bool))
        .ai_setter(&[0, 1, 0x400, 0x8000_0000, 0xffff_ffff])
        .run()
        .assert_same();
}

/// The fields of the AI data the compare reads, by offset, and padding bytes it does not.
const AI_COMPARED: [u32; 21] = [
    4, 5, 6, 7, 8, 0xc, 0xd, 0xe, 0xf, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 9, 0xa, 0xb,
    0x18,
];

#[test]
fn ai_compare_component() {
    Diff::new(entry!(0x0047_f300, ai_form::compare_component(u32, u32) -> bool))
        .ai_caller()
        .inputs(&[
            Input::Of(&[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
            ]),
            Input::Of(&[0, 1, 0x100]),
        ])
        .setup(|e, case| {
            ai_world(e);
            let this = ai_form(e, 0x31);
            let other = ai_form(e, 0x31);
            // The package lists: the same nodes, or each its own.
            if case.word(1) == 0 {
                for at in [0x18, 0x1c] {
                    let word = e.mem.u32(this + at);
                    e.mem.set_u32(other + at, word);
                }
            }
            // 0: not a TESAIForm; 1: the same data; then one byte of the data different.
            if let Some(offset) = case.word(0).checked_sub(2) {
                let at = other + AI_COMPARED[offset as usize];
                let byte = e.mem.u8(at);
                e.mem.set_u8(at, byte ^ 0x40);
            }
            set_cast(
                e,
                TYPE_TES_AI_FORM,
                if case.word(0) == 0 { 0 } else { other },
            );
            vec![this, other]
        })
        .run()
        .assert_same();
}

#[test]
fn ai_sells_buys_item() {
    Diff::new(entry!(0x0047_f600, ai_form::sells_buys_item(u32, u32) -> bool))
        .ai_caller()
        .inputs(&[
            Input::Of(&[
                0, 0x17, 0x18, 0x19, 0x1a, 0x1f, 0x28, 0x29, 0x2a, 0x2f, 0x32, 0x67, 0x68, 0xff,
            ]),
            Input::Of(&[9, 0xa, 0xb, 0xc, 0xd, 0xe]),
            Input::Of(&[0, 0xffff_ffff, 0x0000_0555, 0x0000_00aa]),
        ])
        .max_cases(400)
        .setup(|e, case| {
            ai_world(e);
            let this = ai_form(e, 2);
            e.mem.set_u32(this + 0xc, case.word(2));
            let item = form(e, case.word(0), 0, 0x55);
            set_ai_answer(e, 0x0047_9430, case.word(1));
            vec![this, item]
        })
        .run()
        .assert_same();
}

#[test]
fn ai_save_size() {
    Diff::new(entry!(0x0047_f7e0, ai_form::save_size(u32, u32) -> u16))
        .ai_setter(&[0, 8, 7, 0xffff_ffff, 0x8000_0008])
        .run()
        .assert_same();
}

#[test]
fn ai_save_game() {
    Diff::new(entry!(0x0047_f840, ai_form::save_game(u32, u32)))
        .ai_saves(2)
        .run()
        .assert_same();
}

#[test]
fn ai_load_game() {
    Diff::new(entry!(0x0047_f8c0, ai_form::load_game(u32, u32, u32)))
        .ai_saves(3)
        .run()
        .assert_same();
}

#[test]
fn ai_save_to_buffer() {
    Diff::new(entry!(0x0047_f940, ai_form::save_to_buffer(u32, u32)))
        .ai_buffer()
        .run()
        .assert_same();
}

#[test]
fn ai_load_from_buffer() {
    Diff::new(entry!(0x0047_f980, ai_form::load_from_buffer(u32, u32)))
        .ai_buffer()
        .run()
        .assert_same();
}

#[test]
fn animation_construct() {
    Diff::new(entry!(0x0047_f9c0, animation::construct(u32) -> u32))
        .ai()
        .inputs(&[Input::Of(&[0, 1])])
        .setup(|e, case| {
            ai_world(e);
            vec![data(e, 0xc + case.word(0))]
        })
        .run()
        .assert_same();
}

#[test]
fn animation_destruct() {
    Diff::new(entry!(0x0047_f9f0, animation::destruct(u32)))
        .names()
        .setup(|e, case| {
            ai_world(e);
            vec![animation(e, NAME_LISTS[case.word(0) as usize])]
        })
        .run()
        .assert_same();
}

#[test]
fn animation_clear_data_component() {
    Diff::new(entry!(0x0047_fa50, animation::clear_data_component(u32)))
        .names()
        .setup(|e, case| {
            ai_world(e);
            vec![animation(e, NAME_LISTS[case.word(0) as usize])]
        })
        .run()
        .assert_same();
}

#[test]
fn animation_load_names() {
    Diff::new(entry!(0x0047_faa0, animation::load_names(u32, u32, u32)))
        .names()
        .inputs(&[
            Input::Of(&[0, 1, 2, 3, 4, 5]),
            Input::Of(&[0, 1, 0xffff_ffff]),
        ])
        .setup(|e, case| {
            ai_world(e);
            set_answer(e, STRING_COMPARE, case.word(1));
            let this = animation(e, NAME_LISTS[1]);
            // 0: no file; 1: no component; 2: another chunk; then a chunk of names, one with an
            // empty first name, one whose names repeat.
            let chunk: &[u8] = match case.word(0) {
                4 => b"\0walk.kf\0",
                5 => b"a.kf\0A.KF\0b.kf\0\0",
                _ => b"walk.kf\0Run.kf\0idle.kf\0\0",
            };
            for (i, b) in chunk.iter().enumerate() {
                e.mem.set_u8(AI_CHUNK + i as u32, *b);
            }
            e.mem.set_u32(AI_CHUNK_SIZE, chunk.len() as u32);
            let file = file(e, false);
            e.mem.set_u32(file + FILE_CHUNK_SIZE_AT, chunk.len() as u32);
            set_ai_answer(
                e,
                0x0047_26b0,
                if case.word(0) == 2 { 0x4d52_4f46 } else { KFFZ },
            );
            let component = if case.word(0) == 1 { 0 } else { this };
            let file = if case.word(0) == 0 { 0 } else { file };
            vec![this, component, file]
        })
        .run()
        .assert_same();
}

#[test]
fn animation_copy_component() {
    Diff::new(entry!(0x0047_fb50, animation::copy_component(u32, u32)))
        .names()
        .inputs(&[
            Input::Of(&[0, 1, 2, 3, 4]),
            Input::Of(&[0, 1, 0xffff_ffff]),
            Input::Of(&[0, 1, 2]),
        ])
        .setup(|e, case| {
            ai_world(e);
            set_answer(e, STRING_COMPARE, case.word(1));
            let this = animation(e, NAME_LISTS[2]);
            let other = animation(e, NAME_LISTS[case.word(0) as usize]);
            // 0: not a TESAnimation; 1: slot 4 holds `ClearDataComponent`; 2: a stand-in.
            let slot = if case.word(2) == 1 {
                0x0047_fa50
            } else {
                VT_FN + 4
            };
            e.mem.set_u32(ANIM_VTABLE + 4, slot);
            set_cast(
                e,
                TYPE_TES_ANIMATION,
                if case.word(2) == 0 { 0 } else { other },
            );
            vec![this, other]
        })
        .run()
        .assert_same();
}

#[test]
fn animation_compare_component() {
    Diff::new(entry!(0x0047_fbd0, animation::compare_component(u32, u32) -> bool))
        .names()
        .inputs(&[
            Input::Of(&[0, 1, 2, 3, 4]),
            Input::Of(&[0, 1, 0xffff_ffff]),
            Input::Of(&[0, 1, 2, 3, 4, 5]),
        ])
        .setup(|e, case| {
            ai_world(e);
            set_answer(e, STRING_COMPARE, case.word(1));
            let this = animation(e, NAME_LISTS[case.word(0) as usize]);
            // 5: not a TESAnimation; else another list.
            let pick = case.word(2).min(4);
            let other = animation(e, NAME_LISTS[pick as usize]);
            set_cast(
                e,
                TYPE_TES_ANIMATION,
                if case.word(2) == 5 { 0 } else { other },
            );
            vec![this, other]
        })
        .run()
        .assert_same();
}

#[test]
fn animation_has_name() {
    Diff::new(entry!(0x0047_fcb0, animation::has_name(u32, u32) -> bool))
        .names()
        .setup(|e, case| {
            ai_world(e);
            set_answer(e, STRING_COMPARE, case.word(1));
            let this = animation(e, NAME_LISTS[case.word(0) as usize]);
            vec![this, c_string(e, b"WALK.kf")]
        })
        .run()
        .assert_same();
}

#[test]
fn animation_add_name() {
    Diff::new(entry!(0x0047_fd20, animation::add_name(u32, u32)))
        .names()
        .setup(|e, case| {
            ai_world(e);
            set_answer(e, STRING_COMPARE, case.word(1));
            let this = animation(e, NAME_LISTS[case.word(0) as usize]);
            vec![this, c_string(e, b"jump.kf")]
        })
        .run()
        .assert_same();
}

/// `copied_setting` (`0047eeb0`) quiets a signalling NaN, as its `FLD` does. Its Diff can't see
/// that: the harness widens every float result to an `f64`, which quiets it on its own. So the bits
/// the typed function answers are read here.
#[test]
fn ai_copied_setting_quiets_a_signalling_nan() {
    let mut e = Engine::new_untyped_for_test();
    e.map(0x011c_5000, 0x1000);
    e.mem.set_u32(0x011c_52a8 + 8, 0x7f80_0001);
    e.mem.set_u32(0x011c_52a8 + 4, 0x7fc1_2345);
    assert_eq!(ai_form::copied_setting(&mut e, 2).to_bits(), 0x7fc0_0001);
    assert_eq!(ai_form::copied_setting(&mut e, 1).to_bits(), 0x7fc1_2345);
}

/// The slot shortcut of `mark_changed` goes direct: with the slot holding `TESActorBase::AddChange`
/// for a created form or `TESForm::AddChange`, whose bodies make no typed call of their own, each
/// run's one direct call is the shortcut's (a vtable call reaches the same typed function by
/// address, not directly).
#[test]
fn ai_mark_changed_goes_direct() {
    let report = Diff::new(entry!(0x0047_f790, ai_form::mark_changed(u32, u32)))
        .ai()
        .ignore_calls(AB_IN_PLACE)
        .ignore_calls(ABD_IN_PLACE)
        .inputs(&[Input::Of(&[4, 5]), Input::Of(&[8, 0xffff_ffff])])
        .setup(|e, case| {
            ai_world(e);
            answered!(e, 0x0084_a620);
            set_answer(e, 0x0084_a620, 1);
            let bgs = e.mem.alloc(0x40);
            e.mem.set_u32(BGS_SAVE_LOAD_GAME, bgs);
            let ai = ai_form(e, 1);
            let owner = ai - 0x90;
            let vtable = e.mem.alloc(0x50);
            let (body, id) = if case.word(0) == 4 {
                (0x005f_1630, 0xff00_0042)
            } else {
                (0x0048_4b60, 0x0100_0042)
            };
            e.mem.set_u32(vtable + 0x48, body);
            e.mem.set_u32(owner, vtable);
            e.mem.set_u32(owner + 0xc, id);
            vec![ai, case.word(1)]
        })
        .run();
    report.assert_same();
    assert_direct_each_run(&report);
}

/// The slot shortcut of the animation's copy goes direct: slot 4 holding `ClearDataComponent` and
/// a source with no names, so the shortcut is each run's one direct call.
#[test]
fn animation_copy_component_goes_direct() {
    let report = Diff::new(entry!(0x0047_fb50, animation::copy_component(u32, u32)))
        .names()
        .inputs(&[Input::Of(&[1, 2])])
        .setup(|e, case| {
            ai_world(e);
            let this = animation(e, NAME_LISTS[case.word(0) as usize]);
            let other = animation(e, NAME_LISTS[0]);
            e.mem.set_u32(ANIM_VTABLE + 4, 0x0047_fa50);
            set_cast(e, TYPE_TES_ANIMATION, other);
            vec![this, other]
        })
        .run();
    report.assert_same();
    assert_direct_each_run(&report);
}

/// Each direct run made exactly one direct call (the shortcut under test), and there were direct
/// runs (with debug assertions: the harness runs the direct branch only then).
fn assert_direct_each_run(report: &engine_core::typed::diff::Report) {
    if cfg!(debug_assertions) {
        assert!(report.direct_runs > 0, "no direct runs");
        assert_eq!(
            report.direct_calls, report.direct_runs,
            "the shortcut did not go direct on every run"
        );
    }
}

// ---- TESObject's unit: the debug-heap string (`debug_string`) and the object classes (`object`) ----

/// The unit's answer words (one per callee in [`OB_ANSWERED`]); what the JIP LN seam answers and
/// was given; the word the model loader's find puts in the model holder; the object a throw throws;
/// a vtable whose slot 0x14c holds `Clone3D` itself.
const OB_ANSWERS: u32 = 0x0320_0000;
const OB_SEAM: u32 = 0x0320_1f00;
const OB_SEAM_SEEN: u32 = 0x0320_1f04;
const OB_FIND: u32 = 0x0320_1f10;
const OB_THROWN: u32 = 0x0320_1f20;
const OB_CLONE_VTABLE: u32 = 0x0320_1800;
/// Where a test object keeps the scale (an `f64`) the scale getters answer for it.
const OB_SCALE_AT: u32 = 0x40;

/// The callees answered by a word of their own.
const OB_ANSWERED: [u32; 47] = [
    0x0064_df10, // the string's deallocate
    0x0047_26b0, // the file's current chunk type
    0x0051_0130, // the parent list's removal
    0x0048_95b0, // the form's model path
    0x0048_0e60, // the biped model's
    0x0057_15d0, // the reference's
    0x0045_a5e0, // the model loader's register
    0x0040_c110, // the model holder's destructor
    0x0040_6d00, // sprintf_s
    0x00a5_a040, // the node's preparation
    0x00a5_9c60, // and update (the scene family's: by address here)
    0x0066_4840, // the object manager
    0x0066_4f50, // its build
    0x008d_6f30, // the result's word (the AI queries': by address here)
    0x0056_9350, // the reference's call with it
    0x0060_3bb0, // the type 0x2a form's call
    0x0044_4ed0, // the reference's 3D test
    0x0050_d4a0, // the form's test
    0x0069_3ef0, // the reference's call
    0x004b_5bf0, // the deep copy test
    0x0045_bc00, // the node's child count test (multibound's: by address here)
    0x0043_b4a0, // the node's child (dispatch's: by address here)
    0x0043_b230, // an object's controllers (dispatch's: by address here)
    0x0065_3270, // NiDynamicCast (dispatch's: by address here)
    0x0047_a520, // the controller's test
    0x0062_c3c0, // the shared model's test
    0x0044_8a80, // the property name
    0x00a5_bdd0, // the extra data
    0x0044_8a40, // its test
    0x0048_b310, // the texture swap test
    0x0056_9410, // the emittance test
    0x0055_d520, // the reference's name
    0x004a_d050, // the clone process's constructor
    0x004a_d270, // and destructor
    0x0045_7ba0, // the helper's deep copy
    0x00aa_13e0, // the node's allocation
    0x00a5_ecb0, // its constructor
    0x00a5_d2c0, // the scene graph's clone
    0x0044_0490, // the clone's scale
    0x0043_abf0, // the model's constructor
    0x0092_c870, // its reference count (the cell references': by address here)
    0x0044_7010, // the model loader's add
    0x0044_31f0, // the model's release
    0x0045_0f90, // the clone's fix-ups
    0x0043_fa80,
    0x0044_0460,
    0x0096_11e0,
];

/// The bodies the unit's typed code does in place (see `object`'s and `debug_string`'s module
/// comments): left out of the translation's calls, with no double, as [`IN_PLACE`].
const OB_IN_PLACE: &[u32] = &[
    0x0040_1170, // the form type
    0x0040_1500, // the byte order
    0x0040_7a90, // the half swap
    0x0040_1680, // the file's swapped byte
    0x0068_15c0, // the empty constructor
    0x0065_de30, // the proxy's empty constructor
    0x008d_0600, // the empty forward target
    0x0040_6d90, // the float-to-int helper
    0x0044_a970, // _Myptr
    0x0082_5c00, // the string's length
    0x0044_ab90, // _Inside
    0x0044_a880, // _Eos
];
/// The family's own members the unit's callers do in place: only in the callers' tests.
const OB_OWN_IN_PLACE: &[u32] = &[0x0044_a7b0, 0x0044_a7d0, 0x0050_e9c0];
/// The other families' functions the unit calls typed that doubles stand at here: their own tests
/// run them.
const OB_BY_ADDRESS: &[u32] = &[
    0x0043_b230,
    0x0043_b4a0,
    0x0065_3270,
    0x0045_bc00,
    0x0092_c870,
    0x00a5_9c60,
    0x008d_6f30,
];

fn ob_answer_at(addr: u32) -> u32 {
    let index = OB_ANSWERED
        .iter()
        .position(|a| *a == addr)
        .expect("an answered callee");
    OB_ANSWERS + 4 * index as u32
}

fn ob_answer<const A: u32>(e: &mut Engine, _: &[u32]) -> Ret {
    ret(e.mem.u32(ob_answer_at(A)))
}

fn set_ob_answer(e: &mut Engine, addr: u32, value: u32) {
    e.mem.set_u32(ob_answer_at(addr), value);
}

/// A scale getter's answer: the `f64` its object keeps at [`OB_SCALE_AT`].
fn ob_scale(e: &mut Engine, a: &[u32]) -> Ret {
    Ret {
        eax: 0,
        edx: 0,
        st0: e.mem.f64(a[0] + OB_SCALE_AT),
    }
}

macro_rules! ob_answered {
    ($e:expr, $($addr:expr),* $(,)?) => {
        $( $e.register($addr, ob_answer::<{ $addr }>); )*
    };
}

/// The JIP LN seam's handler: notes the three words and answers [`OB_SEAM`].
fn ob_seam(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let seen = a[0] ^ a[1].rotate_left(8) ^ a[2].rotate_left(16);
    e.mem.set_u32(OB_SEAM_SEEN, seen);
    Some(e.mem.u32(OB_SEAM))
}

/// The world of the unit's cases: the shared one, the unit's globals and read-only doubles, the
/// slots it calls, and its callees' doubles. `_Grow`, `copy_s` and `move_s` do their work on the
/// test's strings; `_Xran` throws; the model holder keeps its word.
fn ob_world(e: &mut Engine) {
    world(e);
    for page in [
        0x0101_1000,
        0x0101_2000,
        0x0101_7000,
        0x011c_6000,
        0x011c_9000,
        0x011c_a000,
        0x011f_8000,
        0x0120_3000,
    ] {
        e.map(page, 0x1000);
    }
    e.map(OB_ANSWERS, 0x2000);
    e.mem.set_u32(0x0101_73f0, u32::MAX);
    e.mem.set_f64(0x0101_1588, 0.5);
    e.mem.set_f64(0x0101_2060, 0.0);
    e.mem.set_f64(0x0101_2070, 1.0);
    e.mem.set_f64(0x0101_7a40, 100.0);
    for slot in [0xc, 0x14c, 0x174, 0x17c, 0x180, 0x3d0] {
        e.mem.set_u32(VTABLE + slot, VT_FN + slot);
    }
    e.register(VT_FN + 0xc, slot_answer::<0xc>);
    e.register(VT_FN + 0x14c, slot_answer::<0x14c>);
    e.register(VT_FN + 0x174, slot_answer::<0x174>);
    e.register(VT_FN + 0x17c, slot_answer::<0x17c>);
    e.register(VT_FN + 0x180, slot_answer::<0x180>);
    e.register(VT_FN + 0x3d0, slot_answer::<0x3d0>);
    for slot in (0..0x400).step_by(4) {
        let word = e.mem.u32(VTABLE + slot);
        e.mem.set_u32(OB_CLONE_VTABLE + slot, word);
    }
    e.mem.set_u32(OB_CLONE_VTABLE + 0x14c, 0x0050_ee50);
    ob_answered!(
        e,
        0x0064_df10,
        0x0047_26b0,
        0x0051_0130,
        0x0048_95b0,
        0x0048_0e60,
        0x0057_15d0,
        0x0045_a5e0,
        0x0040_c110,
        0x0040_6d00,
        0x00a5_a040,
        0x00a5_9c60,
        0x0066_4840,
        0x0066_4f50,
        0x008d_6f30,
        0x0056_9350,
        0x0060_3bb0,
        0x0044_4ed0,
        0x0050_d4a0,
        0x0069_3ef0,
        0x004b_5bf0,
        0x0045_bc00,
        0x0043_b4a0,
        0x0043_b230,
        0x0065_3270,
        0x0047_a520,
        0x0062_c3c0,
        0x0044_8a80,
        0x00a5_bdd0,
        0x0044_8a40,
        0x0048_b310,
        0x0056_9410,
        0x0055_d520,
        0x004a_d050,
        0x004a_d270,
        0x0045_7ba0,
        0x00aa_13e0,
        0x00a5_ecb0,
        0x00a5_d2c0,
        0x0044_0490,
        0x0043_abf0,
        0x0092_c870,
        0x0044_7010,
        0x0044_31f0,
        0x0045_0f90,
        0x0043_fa80,
        0x0044_0460,
        0x0096_11e0,
    );
    e.register(0x0056_7400, ob_scale);
    e.register(0x009c_dae0, ob_scale);
    // `_Grow(count, trim)`: a heap buffer of its own when the reserve is too small, the characters
    // copied over; whether `count` is not 0.
    e.register(0x0044_a8d0, |e, a| {
        let (s, count) = (a[0], a[1]);
        let reserve = e.mem.u32(s + 0x18);
        if count > reserve {
            let new = count | 0xf;
            let block = e.mem.alloc(new + 1);
            let size = e.mem.u32(s + 0x14).min(new);
            let old = if reserve >= 0x10 {
                e.mem.u32(s + 4)
            } else {
                s + 4
            };
            copy_bytes(e, block, old, size);
            e.mem.set_u32(s + 4, block);
            e.mem.set_u32(s + 0x18, new);
        }
        ret(u32::from(count > 0))
    });
    // `copy_s(dest, room, src, count)` and `move_s(string, dest, room, src, count)`.
    e.register(0x0044_ccd0, |e, a| {
        copy_bytes(e, a[0], a[2], a[3].min(a[1]));
        ret(a[0])
    });
    e.register(0x0044_d580, |e, a| {
        let n = a[4].min(a[2]);
        let bytes: Vec<u8> = (0..n).map(|i| e.mem.u8(a[3] + i)).collect();
        for (i, b) in bytes.into_iter().enumerate() {
            e.mem.set_u8(a[1] + i as u32, b);
        }
        ret(a[1])
    });
    e.register(0x00ec_30e8, |e, _| {
        engine_core::cxx_exception::throw(e, OB_THROWN, 0x0117_327c)
    });
    // The model holder: its constructor and set keep the word; the loader's find puts
    // [`OB_FIND`] in it and answers whether that is not 0.
    e.register(0x0063_3c90, |e, a| {
        e.mem.set_u32(a[0], a[1]);
        ret(a[0])
    });
    e.register(0x0044_aed0, |e, a| {
        e.mem.set_u32(a[0], a[1]);
        ret(a[0])
    });
    e.register(0x0044_72a0, |e, a| {
        let found = e.mem.u32(OB_FIND);
        if found != 0 {
            e.mem.set_u32(a[2], found);
        }
        ret(u32::from(found != 0))
    });
}

/// A test object with a vtable, its scale 1.0.
fn ob_object(e: &mut Engine, vtable: u32) -> u32 {
    let o = e.mem.alloc(0x50);
    e.mem.set_u32(o, vtable);
    e.mem.set_f64(o + OB_SCALE_AT, 1.0);
    o
}

/// `size` bytes as the stack leaves them before a constructor runs.
fn ob_garbage(e: &mut Engine, size: u32) -> u32 {
    let o = e.mem.alloc(size);
    for i in 0..size {
        e.mem.set_u8(o + i, 0xcd);
    }
    o
}

/// The strings the cases pick: empty, short, the longest the buffer holds, and one on the heap.
const OB_TEXTS: [&[u8]; 4] = [
    b"",
    b"abc",
    b"fifteen bytes!!",
    b"a text of thirty-one characters",
];

/// A debug-heap string holding `text`: in its buffer up to 15 characters, else on the heap.
fn ob_string(e: &mut Engine, text: &[u8]) -> u32 {
    let s = e.mem.alloc(0x1c);
    e.mem.set_u32(s, 0x5151_5151);
    let n = text.len() as u32;
    let (reserve, chars) = if n < 0x10 {
        (0xf, s + 4)
    } else {
        let reserve = n | 0xf;
        let block = e.mem.alloc(reserve + 1);
        e.mem.set_u32(s + 4, block);
        (reserve, block)
    };
    for (i, b) in text.iter().enumerate() {
        e.mem.set_u8(chars + i as u32, *b);
    }
    e.mem.set_u8(chars + n, 0);
    e.mem.set_u32(s + 0x14, n);
    e.mem.set_u32(s + 0x18, reserve);
    s
}

/// The characters of a test string.
fn ob_chars(e: &Engine, s: u32) -> u32 {
    if e.mem.u32(s + 0x18) >= 0x10 {
        e.mem.u32(s + 4)
    } else {
        s + 4
    }
}

/// A form to destroy: the shared world's dying form, without the shared world again.
fn ob_dying(e: &mut Engine, flags: u32, count: u32, save_games: bool) -> u32 {
    let f = form(e, 0x20, flags, 0x0100_0abc);
    registries(e, true, 2, true, save_games, f);
    e.mem.set_u32(FORM_COUNT, count);
    f
}

/// A form to construct over: `TESForm`'s constructor with its table packed and no handler.
fn ob_fresh(e: &mut Engine) -> u32 {
    e.mem.set_u8(0x011c_54b8, 1);
    ob_garbage(e, 0x40)
}

/// The unit's cases: the forms family's, the pathing family (its point maths), the unit's in-place
/// bodies, and the other families' typed callees that doubles stand at.
fn ob_cases(d: Diff) -> Diff {
    d.forms()
        .family(&PATHING)
        .ignore_calls(OB_IN_PLACE)
        .goes_by_address(OB_BY_ADDRESS)
}

trait ObCases {
    fn ob(self) -> Diff;
    fn ob_caller(self) -> Diff;
    fn ob_root(self) -> Diff;
}

impl ObCases for Diff {
    fn ob(self) -> Diff {
        ob_cases(self)
    }
    fn ob_caller(self) -> Diff {
        ob_cases(self).ignore_calls(OB_OWN_IN_PLACE)
    }
    /// The square root's chain: the translation of `004019b0` calls `004019d0` plainly, and the
    /// pathing family's root does the C runtime's `sqrt` (`00ec6054`) in place.
    fn ob_root(self) -> Diff {
        ob_cases(self)
            .translation_calls_plainly(&[0x0040_19d0])
            .ignore_calls(&[0x00ec_6054])
    }
}

const OB_PICK: &[u32] = &[0, 1, 2, 3];

#[test]
fn ob_string_construct_from_text() {
    Diff::new(entry!(0x0044_a4c0, debug_string::construct_from_text(u32, u32) -> u32))
        .ob_caller()
        .inputs(&[Input::Of(OB_PICK)])
        .setup(|e, case| {
            ob_world(e);
            let this = ob_garbage(e, 0x1c);
            vec![this, c_string(e, OB_TEXTS[case.word(0) as usize])]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_string_copy_construct() {
    Diff::new(entry!(0x0044_a500, debug_string::copy_construct(u32, u32) -> u32))
        .ob_caller()
        .inputs(&[Input::Of(OB_PICK)])
        .setup(|e, case| {
            ob_world(e);
            let this = ob_garbage(e, 0x1c);
            vec![this, ob_string(e, OB_TEXTS[case.word(0) as usize])]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_string_destruct() {
    Diff::new(entry!(0x0044_a540, debug_string::destruct(u32)))
        .ob()
        .inputs(&[Input::Of(OB_PICK)])
        .setup(|e, case| {
            ob_world(e);
            vec![ob_string(e, OB_TEXTS[case.word(0) as usize])]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_string_c_str() {
    Diff::new(entry!(0x0044_a560, debug_string::c_str(u32) -> u32))
        .ob()
        .inputs(&[Input::Of(OB_PICK)])
        .setup(|e, case| {
            ob_world(e);
            vec![ob_string(e, OB_TEXTS[case.word(0) as usize])]
        })
        .run()
        .assert_same();
}

/// The length of the right string a case picks: itself (0) or `OB_TEXTS[n - 1]`.
fn ob_right_length(this: u32, right: u32) -> u32 {
    let pick = if right == 0 { this } else { right - 1 };
    OB_TEXTS[pick as usize].len() as u32
}

#[test]
fn ob_string_assign_range() {
    Diff::new(entry!(0x0044_a580, debug_string::assign_range(u32, u32, u32, u32) -> u32))
        .ob()
        .inputs(&[
            Input::Of(OB_PICK),
            // The right string: this one, or another.
            Input::Of(&[0, 1, 2, 3, 4]),
            Input::Of(&[0, 2, 3, 15, 31, 40]),
            Input::Of(&[0, 1, 3, 0xffff_ffff]),
        ])
        .expect_stops(|case| case.word(2) > ob_right_length(case.word(0), case.word(1)))
        .setup(|e, case| {
            ob_world(e);
            let this = ob_string(e, OB_TEXTS[case.word(0) as usize]);
            let right = match case.word(1) {
                0 => this,
                n => ob_string(e, OB_TEXTS[n as usize - 1]),
            };
            vec![this, right, case.word(2), case.word(3)]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_string_assign_text() {
    Diff::new(entry!(0x0044_a640, debug_string::assign_text(u32, u32) -> u32))
        .ob()
        .inputs(&[Input::Of(OB_PICK), Input::Of(&[0, 1, 2, 3, 4, 5])])
        .setup(|e, case| {
            ob_world(e);
            let this = ob_string(e, OB_TEXTS[case.word(0) as usize]);
            let text = match case.word(1) {
                // Text inside the string itself: its start and one past it.
                4 => ob_chars(e, this),
                5 => ob_chars(e, this) + 1,
                n => c_string(e, OB_TEXTS[n as usize]),
            };
            vec![this, text]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_string_text_length() {
    Diff::new(entry!(0x0044_a670, debug_string::text_length(u32) -> u32))
        .ob()
        .inputs(&[Input::Of(OB_PICK)])
        .setup(|e, case| {
            ob_world(e);
            vec![c_string(e, OB_TEXTS[case.word(0) as usize])]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_string_erase() {
    Diff::new(entry!(0x0044_a690, debug_string::erase(u32, u32, u32) -> u32))
        .ob()
        .inputs(&[
            Input::Of(OB_PICK),
            Input::Of(&[0, 1, 3, 14, 31, 32]),
            Input::Of(&[0, 1, 2, 20, 0xffff_ffff]),
        ])
        .expect_stops(|case| case.word(1) > OB_TEXTS[case.word(0) as usize].len() as u32)
        .setup(|e, case| {
            ob_world(e);
            let this = ob_string(e, OB_TEXTS[case.word(0) as usize]);
            vec![this, case.word(1), case.word(2)]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_string_tidy() {
    Diff::new(entry!(0x0044_a730, debug_string::tidy(u32, bool, u32)))
        .ob()
        .inputs(&[
            Input::Of(OB_PICK),
            Input::Of(&[0, 1, 0x100]),
            Input::Of(&[0, 1, 2, 3, 15, 16]),
        ])
        .setup(|e, case| {
            ob_world(e);
            let this = ob_string(e, OB_TEXTS[case.word(0) as usize]);
            vec![this, case.word(1), case.word(2)]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_string_construct_proxy() {
    Diff::new(entry!(0x0044_a7b0, debug_string::construct_proxy(u32, u32) -> u32))
        .ob()
        .inputs(&[Input::Of(WORDS)])
        .setup(|e, case| {
            ob_world(e);
            vec![ob_garbage(e, 0x1c), case.word(0)]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_string_construct_proxy_from() {
    Diff::new(entry!(0x0044_a7d0, debug_string::construct_proxy_from(u32, u32) -> u32))
        .ob()
        .inputs(&[Input::Of(WORDS)])
        .setup(|e, case| {
            ob_world(e);
            vec![ob_garbage(e, 0x1c), case.word(0)]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_string_assign_chars() {
    Diff::new(entry!(0x0044_a7f0, debug_string::assign_chars(u32, u32, u32) -> u32))
        .ob()
        .inputs(&[
            Input::Of(OB_PICK),
            Input::Of(&[0, 1, 2, 3]),
            Input::Of(&[0, 1, 2, 5, 20]),
        ])
        .setup(|e, case| {
            ob_world(e);
            let this = ob_string(e, OB_TEXTS[case.word(0) as usize]);
            let text = match case.word(1) {
                0 => c_string(e, b"outside"),
                1 => 0,
                2 => ob_chars(e, this),
                _ => ob_chars(e, this) + 1,
            };
            // A null text with characters to copy is a crash in the exe too.
            let count = if text == 0 { 0 } else { case.word(2) };
            vec![this, text, count]
        })
        .run()
        .assert_same();
}

/// A bound object of the test vtable: its box's points from `seed`.
fn ob_bound(e: &mut Engine, seed: u32) -> u32 {
    let o = ob_object(e, VTABLE);
    for i in 0..6u32 {
        let value = seed.wrapping_mul(0x9e37_79b9).rotate_left(i * 5) as u16;
        e.mem.set_u16(o + 0x24 + 2 * i, value);
    }
    o
}

const OB_SEEDS: &[u32] = &[0, 1, 2, 0x7fff, 0xdead_beef];

#[test]
fn ob_bound_object_construct() {
    Diff::new(entry!(0x0050_e820, object::bound_object_construct(u32) -> u32))
        .ob_caller()
        .inputs(&[Input::Of(&[0, 1, 2, 3])])
        .setup(|e, case| {
            ob_world(e);
            // The default point's guard: not yet made, made (with a point of its own), other bits.
            e.mem.set_u32(0x011c_912c, case.word(0));
            e.mem.set_u32(0x011c_9124, 0x1234_5678);
            e.mem.set_u16(0x011c_9128, 0x9abc);
            vec![ob_fresh(e)]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_copy_point() {
    Diff::new(entry!(0x0050_e8b0, object::copy_box_point(u32, u32) -> u32))
        .ob()
        .inputs(&[Input::Of(OB_SEEDS), Input::Of(&[0, 1, 2])])
        .setup(|e, case| {
            ob_world(e);
            let o = ob_bound(e, case.word(0));
            // Apart, overlapping one way, and the other.
            match case.word(1) {
                0 => vec![o + 0x24, o + 0x2a],
                1 => vec![o + 0x26, o + 0x24],
                _ => vec![o + 0x24, o + 0x26],
            }
        })
        .run()
        .assert_same();
}

#[test]
fn ob_default_bound() {
    Diff::new(entry!(0x0050_e8f0, object::default_bound() -> u32))
        .ob()
        .inputs(&[Input::Of(&[0, 1, 2, 0xffff_fffe, 0xffff_ffff])])
        .setup(|e, case| {
            ob_world(e);
            e.mem.set_u32(0x011c_912c, case.word(0));
            e.mem.set_u32(0x011c_9124, 0x1111_2222);
            vec![]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_set_point() {
    Diff::new(entry!(0x0050_e960, object::set_point(u32, u32, u32, u32) -> u32))
        .ob()
        .inputs(&[Input::Word, Input::Of(&[0, 0x8000, 0x1_7fff]), Input::Word])
        .setup(|e, case| {
            ob_world(e);
            let o = ob_garbage(e, 8);
            vec![o, case.word(0), case.word(1), case.word(2)]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_bound_object_deleting_destructor() {
    Diff::new(entry!(0x0050_e990, object::bound_object_deleting_destructor(u32, u32) -> u32))
        .ob()
        .inputs(&[Input::Of(TEMPORARY), Input::Flag, Input::Flag])
        .setup(|e, case| {
            ob_world(e);
            let f = ob_dying(e, case.word(0), 1, true);
            if case.word(2) != 0 {
                let list = e.mem.alloc(0x10);
                e.mem.set_u32(f + 0x18, list);
            }
            vec![f, case.word(1)]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_bound_data_construct() {
    Diff::new(entry!(0x0050_e9c0, object::bound_data_construct(u32) -> u32))
        .ob()
        .inputs(&[Input::Of(OB_SEEDS)])
        .setup(|e, case| {
            ob_world(e);
            vec![ob_bound(e, case.word(0)) + 0x24]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_bound_object_destruct() {
    Diff::new(entry!(0x0050_e9f0, object::bound_object_destruct(u32)))
        .ob()
        .inputs(&[Input::Of(TEMPORARY), Input::Of(&[1, 2]), Input::Flag])
        .setup(|e, case| {
            ob_world(e);
            let f = ob_dying(e, case.word(0), case.word(1), true);
            if case.word(2) != 0 {
                let list = e.mem.alloc(0x10);
                e.mem.set_u32(f + 0x18, list);
            }
            vec![f]
        })
        .run()
        .assert_same();
}

/// An object with two `NiPoint3`s at +0xc and +0x18 (ordinary floats from `seed`).
fn ob_points(e: &mut Engine, seed: u32) -> u32 {
    let o = e.mem.alloc(0x30);
    let floats = [1.5f32, -2.25, 0.1, 3.0e30, -0.0, 7.0, 1.0e-40, -5.5];
    for i in 0..6u32 {
        let pick = (seed.wrapping_add(i * 3) % floats.len() as u32) as usize;
        e.mem.set_u32(o + 0xc + 4 * i, floats[pick].to_bits());
    }
    o
}

#[test]
fn ob_points_difference() {
    Diff::new(entry!(0x0050_ea10, object::points_difference(u32, u32)))
        .ob()
        .inputs(&[Input::Of(&[0, 1, 2, 3, 4, 5, 6, 7])])
        .setup(|e, case| {
            ob_world(e);
            let o = ob_points(e, case.word(0));
            vec![o, e.mem.alloc(0xc)]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_points_sum() {
    Diff::new(entry!(0x0050_ea50, object::points_sum(u32, u32)))
        .ob()
        .inputs(&[Input::Of(&[0, 1, 2, 3, 4, 5, 6, 7])])
        .setup(|e, case| {
            ob_world(e);
            let o = ob_points(e, case.word(0));
            vec![o, e.mem.alloc(0xc)]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_word_012031e4() {
    Diff::new(entry!(0x0050_ea90, object::bound_extra_data_name() -> u32))
        .ob()
        .inputs(&[Input::Of(WORDS)])
        .setup(|e, case| {
            ob_world(e);
            e.mem.set_u32(0x0120_31e4, case.word(0));
            vec![]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_copy_bound() {
    Diff::new(entry!(0x0050_eaa0, object::copy_bound(u32, u32) -> u32))
        .ob()
        .inputs(&[Input::Of(OB_SEEDS)])
        .setup(|e, case| {
            ob_world(e);
            let this = ob_bound(e, 3);
            vec![this, ob_bound(e, case.word(0))]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_bound_min_as_floats() {
    Diff::new(entry!(0x0050_ead0, object::bound_min_as_floats(u32, u32) -> u32))
        .ob()
        .inputs(&[Input::Of(OB_SEEDS)])
        .setup(|e, case| {
            ob_world(e);
            let this = ob_bound(e, case.word(0));
            vec![this, e.mem.alloc(0xc)]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_point_x() {
    Diff::new(entry!(0x0050_eb30, object::point_x(u32) -> f32))
        .ob()
        .inputs(&[Input::Of(OB_SEEDS)])
        .setup(|e, case| {
            ob_world(e);
            vec![ob_bound(e, case.word(0)) + 0x24]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_point_y() {
    Diff::new(entry!(0x0050_eb50, object::point_y(u32) -> f32))
        .ob()
        .inputs(&[Input::Of(OB_SEEDS)])
        .setup(|e, case| {
            ob_world(e);
            vec![ob_bound(e, case.word(0)) + 0x24]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_point_z() {
    Diff::new(entry!(0x0050_eb70, object::point_z(u32) -> f32))
        .ob()
        .inputs(&[Input::Of(OB_SEEDS)])
        .setup(|e, case| {
            ob_world(e);
            vec![ob_bound(e, case.word(0)) + 0x2a]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_bound_max_as_floats() {
    Diff::new(entry!(0x0050_eb90, object::bound_max_as_floats(u32, u32) -> u32))
        .ob()
        .inputs(&[Input::Of(OB_SEEDS)])
        .setup(|e, case| {
            ob_world(e);
            let this = ob_bound(e, case.word(0));
            vec![this, e.mem.alloc(0xc)]
        })
        .run()
        .assert_same();
}

/// A box from a case's three words: each the minimum's and maximum's component pair, as halves.
fn ob_box(e: &mut Engine, words: [u32; 3]) -> u32 {
    let o = ob_object(e, VTABLE);
    for (i, w) in words.iter().enumerate() {
        e.mem.set_u16(o + 0x24 + 2 * i as u32, *w as u16);
        e.mem.set_u16(o + 0x2a + 2 * i as u32, (*w >> 16) as u16);
    }
    o
}

/// Component pairs: equal, small, opposite extremes (a 16-bit overflow), mixed signs, a large one.
const OB_PAIRS: &[u32] = &[
    0,
    0x0003_0007,
    0x8000_7fff,
    0x7fff_8000,
    0xfff0_0010,
    0x1234_edcb,
    0x0000_6000,
];
/// Points whose sum of squares stays in the signed 32-bit range: past it the square root takes the
/// C runtime's domain error, whose `errno` the pathing family's root leaves out (its module says
/// so). The components are the pairs' low halves.
const OB_COMPONENTS: &[u32] = &[0, 0x0003_0007, 0xfff0_0010, 0x1234_edcb, 0x0000_6000];

#[test]
fn ob_bound_size() {
    Diff::new(entry!(0x0050_ebf0, object::bound_size(u32) -> f32))
        .ob_root()
        .inputs(&[
            Input::Of(OB_PAIRS),
            Input::Of(OB_PAIRS),
            Input::Of(OB_PAIRS),
        ])
        .setup(|e, case| {
            ob_world(e);
            vec![ob_box(e, [case.word(0), case.word(1), case.word(2)])]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_point_difference() {
    Diff::new(entry!(0x0050_ec30, object::point_difference(u32, u32, u32) -> u32))
        .ob()
        .inputs(&[Input::Of(OB_PAIRS), Input::Of(OB_PAIRS), Input::Of(&[0, 1])])
        .setup(|e, case| {
            ob_world(e);
            let o = ob_box(e, [case.word(0), case.word(1), 0x8000_7fff]);
            // The result apart from the points, or over the minimum.
            let result = if case.word(2) == 0 {
                ob_garbage(e, 8)
            } else {
                o + 0x24
            };
            vec![o + 0x24, result, o + 0x2a]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_point_length() {
    Diff::new(entry!(0x0050_ec80, object::point_length(u32) -> f64))
        .ob_root()
        .inputs(&[
            Input::Of(OB_COMPONENTS),
            Input::Of(OB_COMPONENTS),
            Input::Of(OB_COMPONENTS),
        ])
        .setup(|e, case| {
            ob_world(e);
            vec![ob_box(e, [case.word(0), case.word(1), case.word(2)]) + 0x24]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_save_bound() {
    Diff::new(entry!(0x0050_ece0, object::save_bound(u32)))
        .ob()
        .inputs(&[Input::Of(&[0, 1, 0x80]), Input::Of(OB_SEEDS)])
        .setup(|e, case| {
            ob_world(e);
            record_buffer(e);
            e.mem.set_u8(BIG_ENDIAN, case.word(0) as u8);
            vec![ob_bound(e, case.word(1))]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_swap_bound() {
    Diff::new(entry!(0x0050_ed30, object::swap_bound(u32)))
        .ob()
        .inputs(&[Input::Of(OB_SEEDS)])
        .setup(|e, case| {
            ob_world(e);
            vec![ob_bound(e, case.word(0)) + 0x24]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_load_bound() {
    Diff::new(entry!(0x0050_edd0, object::load_bound(u32, u32)))
        .ob()
        .inputs(&[
            Input::Flag,
            Input::Of(&[0x444e_424f, 0x444e_4250, 0]),
            Input::Of(&[0, 1, 0x80]),
        ])
        .setup(|e, case| {
            ob_world(e);
            set_ob_answer(e, 0x0047_26b0, case.word(1));
            let file = if case.word(0) != 0 {
                let f = file(e, true);
                e.mem.set_u8(f + FILE_SWAPPED_AT, case.word(2) as u8);
                f
            } else {
                0
            };
            vec![ob_bound(e, 5), file]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_clone_3d_unflagged() {
    Diff::new(entry!(0x0050_ee20, object::clone_3d_unflagged(u32, u32) -> u32))
        .ob()
        .inputs(&[Input::Flag, Input::Flag, Input::Of(&[0, 0x1234])])
        .setup(|e, case| {
            ob_world(e);
            // The slot: a stand-in (its answer the case's), or `Clone3D` with no model.
            let vtable = if case.word(0) != 0 {
                OB_CLONE_VTABLE
            } else {
                VTABLE
            };
            e.mem.set_u32(SLOT_ANSWERS + 0x14c, case.word(2));
            e.mem.set_u32(SLOT_ANSWERS + 0x174, 0);
            let reference = if case.word(1) != 0 {
                ob_object(e, VTABLE)
            } else {
                0
            };
            vec![ob_object(e, vtable), reference]
        })
        .run()
        .assert_same();
}

/// The slot shortcut of `0050ee20` goes direct: with `Clone3D` in the slot and no model it makes
/// no typed call of its own, so the shortcut is each run's one direct call.
#[test]
fn ob_clone_3d_unflagged_goes_direct() {
    let report = Diff::new(entry!(0x0050_ee20, object::clone_3d_unflagged(u32, u32) -> u32))
        .ob()
        .inputs(&[Input::Flag])
        .setup(|e, case| {
            ob_world(e);
            e.mem.set_u32(SLOT_ANSWERS + 0x174, 0);
            let reference = if case.word(0) != 0 {
                ob_object(e, VTABLE)
            } else {
                0
            };
            vec![ob_object(e, OB_CLONE_VTABLE), reference]
        })
        .run();
    report.assert_same();
    assert_direct_each_run(&report);
}

/// The scales a `Clone3D` case picks: the reference's and the model's.
const OB_SCALES: [(f64, f64); 5] = [(1.0, 1.0), (2.0, 1.0), (0.0, 1.0), (2.0, 2.0), (1.5, 3.0)];

#[test]
fn ob_clone_3d() {
    Diff::new(entry!(0x0050_ee50, object::bound_object_clone_3d(u32, u32, u8) -> u32))
        .ob()
        .inputs(&[
            // The form is one of the two special forms.
            Input::Flag,
            // A reference.
            Input::Flag,
            // The reference's 3D test fails; passes with an owner of type 0x1e; of type 0x20.
            Input::Of(&[0, 1, 2]),
            // No model; a model held once (its word at +4 is 1); one held twice.
            Input::Of(&[0, 1, 2]),
            // The deep copy flag.
            Input::Flag,
            // JIP LN: off; on, its seam answering 0; answering 1.
            Input::Of(&[0, 1, 2]),
            // The form type.
            Input::Of(&[0x15, 0x2b, 0x1c, 0x23, 0x28, 0x29, 0x2a, 0x33, 0x51, 0x20]),
            // The checks' answers, a bit each; the last three words make the emittance alone refuse,
            // the 0x1c path's cast find its controllers with the last test answering 0, and the cast
            // fail after the controllers are found.
            Input::Of(&[
                0,
                0xffff_ffff,
                0x5555_5555,
                0xaaaa_aaaa,
                0x3c3c_3c3c,
                0x0f0f_0f0f,
                0x0000_0080,
                0x0000_0604,
                0x0000_0204,
            ]),
            // The scales.
            Input::Of(&[0, 1, 2, 3, 4]),
            // The helper's copy: one whose node is there, one without, none.
            Input::Of(&[0, 1, 2]),
            // The model loader: no model; one of the reference's scale; of another.
            Input::Of(&[0, 1, 2]),
            // The loader's add.
            Input::Flag,
        ])
        .max_cases(2048)
        .setup(|e, case| {
            ob_world(e);
            let this = ob_object(e, VTABLE);
            e.mem.set_u8(this + 4, case.word(6) as u8);
            if case.word(0) != 0 {
                e.mem.set_u32(0x011c_a250, this);
            }
            let (reference_scale, model_scale) = OB_SCALES[case.word(8) as usize];
            let reference = if case.word(1) != 0 {
                let r = ob_object(e, VTABLE);
                e.mem.set_f64(r + OB_SCALE_AT, reference_scale);
                let owner = ob_object(e, VTABLE);
                e.mem
                    .set_u8(owner + 4, if case.word(2) == 2 { 0x20 } else { 0x1e });
                e.mem.set_u32(r + 0x20, owner);
                r
            } else {
                0
            };
            set_ob_answer(e, 0x0044_4ed0, u32::from(case.word(2) != 0));
            let source = match case.word(3) {
                0 => 0,
                held => {
                    let m = ob_object(e, VTABLE);
                    e.mem.set_u32(m + 4, held);
                    e.mem.set_f64(m + OB_SCALE_AT, model_scale);
                    m
                }
            };
            e.mem.set_u32(SLOT_ANSWERS + 0x174, source);
            if case.word(5) != 0 {
                e.set_jip_patches(engine_core::extend::jip_patches::INSTALLED);
                e.mem.set_u32(OB_SEAM, case.word(5) - 1);
                e.on_site_for_test(0x0050_ef46, ob_seam);
            }
            let bits = case.word(7);
            let bit = |i: u32| (bits >> i) & 1;
            for (i, callee) in [
                0x0050_d4a0,
                0x004b_5bf0,
                0x0045_bc00,
                0x0047_a520,
                0x0062_c3c0,
                0x0044_8a40,
                0x0048_b310,
                0x0056_9410,
                0x0096_11e0,
            ]
            .into_iter()
            .enumerate()
            {
                set_ob_answer(e, callee, bit(i as u32));
            }
            let child = ob_object(e, VTABLE);
            set_ob_answer(e, 0x0043_b4a0, child);
            // The controllers (and, for the loader's model, the model found): an object of the
            // case's scale, or none.
            let found = ob_object(e, VTABLE);
            e.mem.set_f64(
                found + OB_SCALE_AT,
                if case.word(10) == 1 {
                    reference_scale
                } else {
                    7.0
                },
            );
            set_ob_answer(e, 0x0043_b230, if bit(9) != 0 { found } else { 0 });
            set_ob_answer(e, 0x0065_3270, bit(10) * child);
            set_ob_answer(e, 0x00a5_bdd0, bit(11) * child);
            set_ob_answer(e, 0x0044_8a80, 0x0101_4444);
            // The helper's copy and the nodes.
            let node = ob_object(e, VTABLE);
            e.mem
                .set_u32(SLOT_ANSWERS + 0xc, if case.word(9) == 1 { 0 } else { node });
            let copy = if case.word(9) == 2 {
                0
            } else {
                ob_object(e, VTABLE)
            };
            set_ob_answer(e, 0x0045_7ba0, copy);
            let block = e.mem.alloc(0xac);
            set_ob_answer(e, 0x00aa_13e0, block);
            let built = ob_object(e, VTABLE);
            set_ob_answer(e, 0x00a5_ecb0, built);
            let clone = ob_object(e, VTABLE);
            set_ob_answer(e, 0x00a5_d2c0, clone);
            e.mem.set_u32(SLOT_ANSWERS + 0x14c, 0x0bad_c10e);
            // The model loader's find and add, the model's constructor and the names.
            let held = ob_object(e, VTABLE);
            e.mem
                .set_u32(OB_FIND, if case.word(10) != 0 { held } else { 0 });
            set_ob_answer(e, 0x0044_7010, case.word(11));
            let created = ob_object(e, VTABLE);
            set_ob_answer(e, 0x0043_abf0, created);
            set_ob_answer(e, 0x0055_d520, 0x0101_5555);
            let path = c_string(e, b"meshes\\box.nif");
            set_ob_answer(e, 0x0048_95b0, path);
            vec![this, reference, case.word(4)]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_clear_parent_list() {
    Diff::new(entry!(0x0050_f5a0, object::clear_parent_list(u32)))
        .ob()
        .inputs(&[Input::Of(OB_SEEDS)])
        .setup(|e, case| {
            ob_world(e);
            vec![ob_bound(e, case.word(0))]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_word_011c6218() {
    Diff::new(entry!(0x0050_f5c0, object::clone_controller_word() -> u32))
        .ob()
        .inputs(&[Input::Of(WORDS)])
        .setup(|e, case| {
            ob_world(e);
            e.mem.set_u32(0x011c_6218, case.word(0));
            vec![]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_model_path_slot() {
    Diff::new(entry!(0x0050_f5d0, object::model_path_slot(u32) -> u32))
        .ob()
        .inputs(&[Input::Of(WORDS), Input::Of(&[0, 0x0101_2345])])
        .setup(|e, case| {
            ob_world(e);
            e.mem.set_u32(SLOT_ANSWERS + 0x17c, case.word(0));
            set_ob_answer(e, 0x0048_95b0, case.word(1));
            vec![ob_object(e, VTABLE)]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_register_model() {
    Diff::new(entry!(0x0050_f600, object::register_model(u32, u32) -> bool))
        .ob()
        .inputs(&[Input::Of(WORDS), Input::Of(&[0, 0x0101_2345])])
        .setup(|e, case| {
            ob_world(e);
            let node = ob_object(e, VTABLE);
            e.mem.set_u32(SLOT_ANSWERS + 0x174, node);
            set_ob_answer(e, 0x0048_95b0, case.word(1));
            e.mem.set_u32(0x011c_3b3c, 0x0123_4567);
            vec![ob_object(e, VTABLE), case.word(0)]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_cast_and_call() {
    Diff::new(entry!(0x0050_f670, object::cast_and_call(u32, u32, u32, u8, u32, u32) -> bool))
        .ob()
        .inputs(&[
            Input::Flag,
            Input::Flag,
            Input::Of(&[0, 1, 0x1ff]),
            Input::Of(&[0, 0xffff_ffff]),
        ])
        .setup(|e, case| {
            ob_world(e);
            let object = ob_object(e, VTABLE);
            set_cast(e, 0x0118_46d4, if case.word(0) != 0 { object } else { 0 });
            let target = if case.word(1) != 0 {
                ob_object(e, VTABLE)
            } else {
                0
            };
            vec![
                0x0bad_0001,
                target,
                object,
                case.word(2),
                0x0bad_0002,
                case.word(3),
            ]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_word_011f81dc() {
    Diff::new(entry!(0x0050_f6e0, object::file_finder() -> u32))
        .ob()
        .inputs(&[Input::Of(WORDS)])
        .setup(|e, case| {
            ob_world(e);
            e.mem.set_u32(0x011f_81dc, case.word(0));
            vec![]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_bound_anim_object_construct() {
    Diff::new(entry!(0x0050_f6f0, object::bound_anim_object_construct(u32) -> u32))
        .ob_caller()
        .inputs(&[Input::Flag])
        .setup(|e, case| {
            ob_world(e);
            e.mem.set_u32(0x011c_912c, case.word(0));
            vec![ob_fresh(e)]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_bound_anim_object_deleting_destructor() {
    Diff::new(entry!(
        0x0050_f710,
        object::bound_anim_object_deleting_destructor(u32, u32) -> u32
    ))
    .ob()
    .inputs(&[Input::Of(TEMPORARY), Input::Flag, Input::Flag])
    .setup(|e, case| {
        ob_world(e);
        let f = ob_dying(e, case.word(0), 1, true);
        if case.word(2) != 0 {
            let list = e.mem.alloc(0x10);
            e.mem.set_u32(f + 0x18, list);
        }
        vec![f, case.word(1)]
    })
    .run()
    .assert_same();
}

#[test]
fn ob_bound_anim_object_destruct() {
    Diff::new(entry!(0x0050_f740, object::bound_anim_object_destruct(u32)))
        .ob()
        .inputs(&[Input::Of(TEMPORARY), Input::Of(&[1, 2])])
        .setup(|e, case| {
            vec![{
                ob_world(e);
                ob_dying(e, case.word(0), case.word(1), false)
            }]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_register_model_again() {
    Diff::new(entry!(0x0050_f760, object::register_model_again(u32, u32) -> bool))
        .ob()
        .inputs(&[Input::Of(WORDS)])
        .setup(|e, case| {
            ob_world(e);
            let node = ob_object(e, VTABLE);
            e.mem.set_u32(SLOT_ANSWERS + 0x174, node);
            set_ob_answer(e, 0x0048_95b0, 0x0101_2345);
            vec![ob_object(e, VTABLE), case.word(0)]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_forward_to_empty() {
    Diff::new(entry!(0x0050_f780, object::forward_to_empty(u32, u32, u8)))
        .ob()
        .inputs(&[Input::Of(WORDS), Input::Of(&[0, 1, 0x1ff])])
        .setup(|e, case| {
            ob_world(e);
            vec![ob_object(e, VTABLE), case.word(0), case.word(1)]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_bound_tree_object_construct() {
    Diff::new(entry!(0x0050_f7a0, object::bound_tree_object_construct(u32) -> u32))
        .ob_caller()
        .inputs(&[Input::Flag])
        .setup(|e, case| {
            ob_world(e);
            e.mem.set_u32(0x011c_912c, case.word(0));
            vec![ob_fresh(e)]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_bound_tree_object_deleting_destructor() {
    Diff::new(entry!(
        0x0050_f7c0,
        object::bound_tree_object_deleting_destructor(u32, u32) -> u32
    ))
    .ob()
    .inputs(&[Input::Of(TEMPORARY), Input::Flag, Input::Flag])
    .setup(|e, case| {
        ob_world(e);
        let f = ob_dying(e, case.word(0), 2, true);
        if case.word(2) != 0 {
            let list = e.mem.alloc(0x10);
            e.mem.set_u32(f + 0x18, list);
        }
        vec![f, case.word(1)]
    })
    .run()
    .assert_same();
}

#[test]
fn ob_bound_tree_object_destruct() {
    Diff::new(entry!(0x0050_f7f0, object::bound_tree_object_destruct(u32)))
        .ob()
        .inputs(&[Input::Of(TEMPORARY), Input::Of(&[1, 2])])
        .setup(|e, case| {
            vec![{
                ob_world(e);
                ob_dying(e, case.word(0), case.word(1), true)
            }]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_type_25_result() {
    Diff::new(entry!(0x0050_f810, object::type_25_result(u32, u32, u32) -> u32))
        .ob()
        .inputs(&[
            Input::Of(&[0x25, 0x24, 0x125]),
            Input::Flag,
            Input::Flag,
            Input::Of(&[0, 0x77]),
            Input::Flag,
        ])
        .setup(|e, case| {
            ob_world(e);
            let this = ob_object(e, VTABLE);
            e.mem.set_u8(this + 4, case.word(0) as u8);
            set_ob_answer(e, 0x0066_4840, case.word(1) * 0x0123_0000);
            let result = if case.word(2) != 0 {
                ob_object(e, VTABLE)
            } else {
                0
            };
            set_ob_answer(e, 0x0066_4f50, result);
            e.mem.set_u32(SLOT_ANSWERS + 0x100, case.word(3));
            set_ob_answer(e, 0x008d_6f30, 0x0456_0000);
            let argument = if case.word(4) != 0 {
                ob_object(e, VTABLE)
            } else {
                0
            };
            vec![this, argument, 0x0bad_0003]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_object_construct() {
    Diff::new(entry!(0x0050_f8c0, object::object_construct(u32) -> u32))
        .ob()
        .inputs(&[Input::Of(&[0, 5])])
        .setup(|e, case| {
            ob_world(e);
            e.mem.set_u32(FORM_COUNT, case.word(0));
            vec![ob_fresh(e)]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_object_deleting_destructor() {
    Diff::new(entry!(0x0050_f900, object::object_deleting_destructor(u32, u32) -> u32))
        .ob()
        .inputs(&[Input::Of(TEMPORARY), Input::Flag, Input::Flag])
        .setup(|e, case| {
            ob_world(e);
            let f = ob_dying(e, case.word(0), 1, false);
            if case.word(2) != 0 {
                let list = e.mem.alloc(0x10);
                e.mem.set_u32(f + 0x18, list);
            }
            vec![f, case.word(1)]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_object_destruct() {
    Diff::new(entry!(0x0050_f930, object::object_destruct(u32)))
        .ob()
        .inputs(&[
            Input::Of(TEMPORARY),
            Input::Of(&[1, 2]),
            Input::Flag,
            Input::Flag,
        ])
        .setup(|e, case| {
            ob_world(e);
            let f = ob_dying(e, case.word(0), case.word(1), case.word(3) != 0);
            if case.word(2) != 0 {
                let list = e.mem.alloc(0x10);
                e.mem.set_u32(f + 0x18, list);
            }
            vec![f]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_set_previous() {
    Diff::new(entry!(0x0050_f9a0, object::set_previous(u32, u32)))
        .ob()
        .inputs(&[Input::Of(WORDS)])
        .setup(|e, case| {
            ob_world(e);
            vec![ob_bound(e, 1), case.word(0)]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_set_word_20() {
    Diff::new(entry!(0x0050_f9c0, object::set_word_20(u32, u32)))
        .ob()
        .inputs(&[Input::Of(WORDS)])
        .setup(|e, case| {
            ob_world(e);
            vec![ob_bound(e, 2), case.word(0)]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_is_named_type_20() {
    Diff::new(entry!(0x0050_fb90, object::is_named_type_20(u32) -> bool))
        .ob()
        .inputs(&[
            Input::Flag,
            Input::Of(&[0x20, 0x21, 0x120]),
            Input::Of(&[0, 1, 2, 3]),
        ])
        .setup(|e, case| {
            ob_world(e);
            if case.word(0) == 0 {
                return vec![0];
            }
            let this = ob_object(e, VTABLE);
            e.mem.set_u8(this + 4, case.word(1) as u8);
            if let Some(global) =
                [0x011c_a238u32, 0x011c_a234, 0x011c_a23c].get(case.word(2) as usize)
            {
                e.mem.set_u32(*global, this);
            }
            vec![this]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_zero() {
    Diff::new(entry!(0x0050_fbe0, object::zero(u32, u32, u32) -> u32))
        .ob()
        .inputs(&[Input::Of(WORDS), Input::Of(WORDS), Input::Of(WORDS)])
        .setup(|e, case| {
            ob_world(e);
            case.words()
        })
        .run()
        .assert_same();
}

#[test]
fn ob_register_form_model() {
    Diff::new(entry!(0x0050_fbf0, object::register_form_model(u32, u32)))
        .ob()
        .inputs(&[
            // The form: one of the special forms; of type 0x2a; another.
            Input::Of(&[0, 1, 2]),
            // A reference (its scale 1.0, 2.0 or a NaN).
            Input::Of(&[0, 1, 2, 3]),
            // The path: none, empty, a name.
            Input::Of(&[0, 1, 2]),
            // The loader's find: nothing; a model held 0, 1 or 2 times, or below 0.
            Input::Of(&[0, 1, 2, 3, 4]),
        ])
        .setup(|e, case| {
            ob_world(e);
            let this = ob_object(e, VTABLE);
            e.mem
                .set_u8(this + 4, if case.word(0) == 1 { 0x2a } else { 0x15 });
            if case.word(0) == 0 {
                e.mem.set_u32(0x011c_a220, this);
            }
            let reference = match case.word(1) {
                0 => 0,
                n => {
                    let r = ob_object(e, VTABLE);
                    let scale = [1.0, 2.0, f64::NAN][n as usize - 1];
                    e.mem.set_f64(r + OB_SCALE_AT, scale);
                    r
                }
            };
            let path = match case.word(2) {
                0 => 0,
                1 => c_string(e, b""),
                _ => c_string(e, b"meshes\\box.nif"),
            };
            set_ob_answer(e, 0x0057_15d0, path);
            set_ob_answer(e, 0x0048_0e60, path);
            set_ob_answer(e, 0x0048_95b0, path);
            let found = match case.word(3) {
                0 => 0,
                n => {
                    let m = ob_object(e, VTABLE);
                    e.mem.set_u32(m + 8, [0, 1, 2, 0x8000_0000][n as usize - 1]);
                    m
                }
            };
            e.mem.set_u32(OB_FIND, found);
            e.mem.set_u32(0x011c_3b3c, 0x0123_4567);
            e.mem.set_u32(0x011c_90a8, 7);
            e.mem.set_u32(0x011c_90ac, 9);
            vec![this, reference]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_model_path() {
    Diff::new(entry!(0x0050_fd90, object::model_path(u32, u32) -> u32))
        .ob()
        .inputs(&[
            Input::Flag,
            Input::Of(&[0, 0x0101_0001]),
            Input::Of(&[0, 0x0101_0002]),
            Input::Of(&[0, 0x0101_0003]),
        ])
        .setup(|e, case| {
            ob_world(e);
            set_ob_answer(e, 0x0057_15d0, case.word(1));
            set_ob_answer(e, 0x0048_0e60, case.word(2));
            set_ob_answer(e, 0x0048_95b0, case.word(3));
            let reference = if case.word(0) != 0 {
                ob_object(e, VTABLE)
            } else {
                0
            };
            vec![ob_object(e, VTABLE), reference]
        })
        .run()
        .assert_same();
}

#[test]
fn ob_scaled_model_file_name() {
    Diff::new(entry!(
        0x0050_fdf0,
        object::scaled_model_file_name(u32, u32, u32)
    ))
    .ob()
    .inputs(&[Input::F64])
    .float_sweep(64)
    .setup(|e, case| {
        ob_world(e);
        set_ob_answer(e, 0x0057_15d0, 0x0101_0001);
        let reference = ob_object(e, VTABLE);
        e.mem.set_f64(reference + OB_SCALE_AT, case.f64(0));
        vec![ob_object(e, VTABLE), reference, e.mem.alloc(0x108)]
    })
    .run()
    .assert_same();
}

// ---- The package list (`package_list`), the schedule (`schedule`), the package data
// (`package_data`) and the condition lists (`conditions`) ----

/// The package world's calendar and answers: the calendar's month, day, day of the week and hour
/// (`f32` bits), the days in a month, the interface's target and a package target's reference.
const PK_STATE: u32 = 0x0300_7300;
const PK_MONTH: u32 = PK_STATE;
const PK_DAY: u32 = PK_STATE + 4;
const PK_DAY_OF_WEEK: u32 = PK_STATE + 8;
const PK_HOUR: u32 = PK_STATE + 0xc;
const PK_DAYS: u32 = PK_STATE + 0x10;
const PK_TARGET: u32 = PK_STATE + 0x14;
const PK_TARGET_REFERENCE: u32 = PK_STATE + 0x18;
/// The forms the form map's lookup finds, by an id's low three bits (0: none found).
const PK_FORMS: u32 = PK_STATE + 0x40;
/// The packages' and the processes' vtables; each slot's stand-in sits at `PK_FN + slot`.
const PK_VTABLE: u32 = 0x0300_7400;
const PROCESS_VTABLE: u32 = 0x0300_7a00;
const PK_FN: u32 = 0x00ff_8000;
const TYPE_TES_PACKAGE: u32 = 0x0118_46a0;
const TYPE_ACTOR: u32 = 0x0118_46d4;
const PACKAGE_FORM_TYPE: u8 = 0x49;
/// A package's fields (the exe's), then the answers the test keeps in it: whether it may run for a
/// reference (its slot 0x138), its conditions' answer, the per-actor test's, the flag test's bits
/// (bit n answering the test with n), its type and the run-once-a-day list's answer.
const PK_PROCEDURE: u32 = 0x18;
const PK_FLAGS: u32 = 0x1c;
const PK_TARGET_AT: u32 = 0x30;
const PK_SCHEDULE: u32 = 0x38;
const PK_MAY_RUN: u32 = 0x60;
const PK_HOLDS: u32 = 0x61;
const PK_ACTOR_TEST: u32 = 0x62;
const PK_FLAG_TESTS: u32 = 0x63;
const PK_TYPE: u32 = 0x64;
const PK_RUN_ONCE: u32 = 0x68;
/// A reference's process (the exe's), then the test's current package and procedure value.
const REF_PROCESS: u32 = 0x68;
const REF_CURRENT: u32 = 0x70;
const REF_VALUE: u32 = 0x74;

/// The one-line bodies of other classes the package functions do in place (see their module
/// comments): left out of the translations' calls, with no double, as [`IN_PLACE`] (which has the
/// big-endian byte, the word and half swaps, the empty list's constructor and the word store at +4).
const PK_IN_PLACE: &[u32] = &[
    0x0068_15c0, // a node's item slot
    0x0072_6070, // its next node
    0x0082_56d0, // a list's empty test
    0x0072_6c60, // a node's item store
    0x005a_e380, // a list's item count
    0x0041_d8a0, // a package's schedule
    0x0087_eaa0, // its conditions
    0x0067_1d10, // its target
    0x0096_11e0, // its procedure type
    0x0067_4e40, // its never-to-run flag
    0x0067_0f90, // its once-a-day flag
    0x0067_8ca0, // a schedule's month
    0x0086_a460, // and hour
    0x0054_07b0, // the month's setter
    0x005d_43c0, // a reference's extra data
    0x008d_8520, // its process
    0x0060_ce80, // the duration's swap
    0x0067_f030, // the loaded object's constructor
    0x0055_9450, // an event action's idle
    0x0053_7e90, // its setter
    0x0044_ddc0, // its topic
    0x0040_3550, // its setter
    0x0084_e3a0, // its word at +0xc
    0x0041_fd00, // its setter
];
/// The family's own one-line bodies its callers do in place: listed only in the callers' tests.
const PK_OWN_IN_PLACE: &[u32] = &[
    0x0067_8cc0, // the schedule's day of the week
    0x0067_8ce0, // its date
    0x0067_fdb0, // the day of the week's setter
    0x0067_fdd0, // the date's
    0x0067_fdf0, // the hour's
    0x0067_ad20, // TESPackageData's constructor
    0x0067_ad70, // its destructor
    0x0067_c490, // the patrol data's destructor
];
/// The typed calls into families these tests don't name, doubles on both sides: a reference's
/// current package and a package's type (`ai_queries`), the run-once-a-day test (`extra_data`), the
/// calendar's day (`ai`) and the interface's target (`interface`).
const PK_BY_ADDRESS: &[u32] = &[
    0x0093_44a0,
    0x0041_ca90,
    0x0041_d8c0,
    0x0086_7d60,
    0x0070_3180,
];

/// The world plus what the package functions reach: the form map, the packages' and processes'
/// slots, an owner's name slot, and the doubles of their callees by address.
fn package_world(e: &mut Engine) {
    world(e);
    // The pages of the exe's constants the package functions read: the hour step (1.0), the last
    // hour (23.0), the dialogue data's distance (100.0) and the package types' names.
    for page in [0x0101_2000, 0x0101_6000, 0x0103_5000, 0x0119_b000] {
        e.map(page, 0x1000);
    }
    e.mem.set_f64(0x0101_2070, 1.0);
    e.mem.set_f64(0x0103_5838, 23.0);
    e.mem.set_u32(0x0101_6410, 0x42c8_0000);
    for kind in 0..3 {
        e.mem.set_u32(0x0119_bcb0 + 4 * kind, 0x0101_0b00 + kind);
    }
    let map = form(e, 0x7f, 0, 0);
    e.mem.set_u32(FORM_MAP, map);
    for slot in [0x10, 0x130, 0x138] {
        e.mem.set_u32(PK_VTABLE + slot, PK_FN + slot);
    }
    for slot in [0x30, 0x34, 0x38, 0x128, 0x59c, 0x5a8] {
        e.mem.set_u32(PROCESS_VTABLE + slot, PK_FN + slot);
    }
    e.register(PK_FN + 0x10, |_, _| ret(0));
    e.register(PK_FN + 0x130, |_, a| ret(a[0] + 0x50));
    e.register(PK_FN + 0x138, |e, a| {
        ret(u32::from(e.mem.u8(a[0] + PK_MAY_RUN)))
    });
    e.register(PK_FN + 0x30, |e, a| ret(e.mem.u32(a[0] + 0x10)));
    e.register(PK_FN + 0x34, |e, a| ret(e.mem.u32(a[0] + 0x14)));
    e.register(PK_FN + 0x38, |e, a| ret(e.mem.u32(a[0] + 0x18)));
    e.register(PK_FN + 0x128, |e, a| ret(e.mem.u32(a[0] + 0x1c)));
    e.register(PK_FN + 0x59c, |_, _| ret(0));
    e.register(PK_FN + 0x5a8, |_, _| ret(0));
    // The owner's name: a text of its own into the string handed.
    e.mem.set_u32(VTABLE + 0x90, VT_FN + 0x90);
    e.register(VT_FN + 0x90, |e, a| {
        e.mem.set_u32(a[1], 0x0101_0090);
        ret(0)
    });
    e.register(0x0067_8d40, |e, a| {
        ret(u32::from(e.mem.u8(a[0] + PK_FLAG_TESTS) >> (a[1] & 7) & 1))
    });
    e.register(0x0086_7d20, |e, _| ret(e.mem.u32(PK_MONTH)));
    e.register(0x0086_7d60, |e, _| ret(e.mem.u32(PK_DAY)));
    e.register(0x0086_7ef0, |e, _| ret(e.mem.u32(PK_DAY_OF_WEEK)));
    e.register(0x0086_7da0, |e, _| {
        let bits = e.mem.u32(PK_HOUR);
        Ret {
            eax: bits,
            edx: 0,
            st0: f64::from(f32::from_bits(bits)),
        }
    });
    e.register(0x0093_44a0, |e, a| ret(e.mem.u32(a[0] + REF_CURRENT)));
    e.register(0x0041_ca90, |e, a| ret(e.mem.u32(a[0] + PK_TYPE)));
    e.register(0x0041_d8c0, |e, a| {
        ret(u32::from(e.mem.u8(a[1] + PK_RUN_ONCE)))
    });
    e.register(0x0068_0020, |e, _| ret(e.mem.u32(PK_TARGET_REFERENCE)));
    e.register(0x0068_0c30, |e, a| {
        ret(u32::from(e.mem.u8(a[0] - 0x40 + PK_HOLDS)))
    });
    e.register(0x0070_3180, |e, _| ret(e.mem.u32(PK_TARGET)));
    e.register(0x0046_4f30, |_, a| ret(a[0]));
    e.register(0x0055_d520, |_, a| ret(a[0].wrapping_add(0x78)));
    e.register(0x0056_9b80, |e, a| {
        ret(if a[0] == 0 {
            0
        } else {
            e.mem.u32(a[0] + REF_VALUE)
        })
    });
    e.register(0x0067_7760, |e, a| {
        ret(u32::from(e.mem.u8(a[0] + PK_ACTOR_TEST)))
    });
    // The output list's test: whether a node holds the package the word points at.
    e.register(0x005f_65d0, |e, a| {
        let item = e.mem.u32(a[1]);
        let mut node = a[0];
        while node != 0 {
            if e.mem.u32(node) == item {
                return ret(1);
            }
            node = e.mem.u32(node + 4);
        }
        ret(0)
    });
    e.register(0x004b_10d0, |e, _| ret(e.mem.u32(PK_DAYS)));
    // The procedure type's calculation stores the value it is given.
    e.register(0x0067_77b0, |e, a| {
        e.mem.set_u32(a[0] + PK_PROCEDURE, a[1]);
        ret(0)
    });
    for addr in [
        0x005a_c750,
        0x0042_2690,
        0x00ec_623a,
        0x0070_3c00,
        0x0067_fbe0,
        0x005a_a420,
        0x007b_3fa0,
    ] {
        e.register(addr, |_, _| ret(0));
    }
    e.register(0x0040_37b0, |e, a| {
        e.mem.set_u32(a[0], 0);
        e.mem.set_u32(a[0] + 4, 0);
        ret(a[0])
    });
    e.register(0x0040_37f0, |e, a| {
        e.mem.set_u32(a[0], a[1]);
        ret(0)
    });
    e.register(0x005a_a0f0, |_, a| ret(a[0]));
    e.register(MAP_LOOKUP, |e, a| {
        let form = e.mem.u32(PK_FORMS + 4 * (a[1] & 7));
        if form != 0 {
            e.mem.set_u32(a[2], form);
        }
        ret(u32::from(form != 0))
    });
    // The cast to `TESPackage` finds a form of the package type; the others answer as `CASTS` says.
    e.register(RT_DYNAMIC_CAST, |e, a| {
        if a[3] == TYPE_TES_PACKAGE {
            ret(if a[0] != 0 && e.mem.u8(a[0] + 4) == PACKAGE_FORM_TYPE {
                a[0]
            } else {
                0
            })
        } else {
            ret(e.mem.u32(CASTS + (a[3] & 0xfff)))
        }
    });
}

/// A list whose head is at `head` holding `items`, a node each (none: both words zero).
fn set_list(e: &mut Engine, head: u32, items: &[u32]) {
    e.mem.set_u32(head, 0);
    e.mem.set_u32(head + 4, 0);
    let mut node = head;
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            let next = e.mem.alloc(8);
            e.mem.set_u32(node + 4, next);
            node = next;
        }
        e.mem.set_u32(node, *item);
    }
}

fn new_list(e: &mut Engine, items: &[u32]) -> u32 {
    let head = e.mem.alloc(8);
    set_list(e, head, items);
    head
}

/// A word mixed from `seed` and `k`: the builders' picks.
fn mix(seed: u32, k: u32) -> u32 {
    let x = seed.wrapping_mul(0x9e37_79b9) ^ k.wrapping_mul(0x85eb_ca6b);
    (x ^ (x >> 15)).wrapping_mul(0x2c1b_3c6d) >> 7
}

fn pick<T: Copy>(seed: u32, k: u32, from: &[T]) -> T {
    from[(mix(seed, k) % from.len() as u32) as usize]
}

/// A schedule's eight bytes from `seed`: the month, the day of the week, the date and the hour, each
/// "any" or a value, and a duration.
fn set_schedule(e: &mut Engine, schedule: u32, seed: u32) {
    e.mem.set_u8(schedule, pick(seed, 1, &[0xff, 0, 4, 11]));
    e.mem.set_u8(schedule + 1, pick(seed, 2, &[0xff, 0, 3, 6]));
    e.mem
        .set_u8(schedule + 2, pick(seed, 3, &[0, 1, 14, 15, 30, 0xff]));
    e.mem.set_u8(schedule + 3, pick(seed, 4, &[0xff, 0, 7, 22]));
    e.mem.set_u32(schedule + 4, pick(seed, 5, &[0, 2, 6, 30]));
}

fn new_schedule(e: &mut Engine, seed: u32) -> u32 {
    let s = e.mem.alloc(8);
    set_schedule(e, s, seed);
    s
}

/// A package from `seed`: the test's vtable and the package form type, its procedure type (unset
/// or set), flags, schedule and target, and the answers its callees give.
fn package(e: &mut Engine, seed: u32) -> u32 {
    let p = e.mem.alloc(0x80);
    e.mem.set_u32(p, PK_VTABLE);
    e.mem.set_u8(p + 4, PACKAGE_FORM_TYPE);
    e.mem
        .set_u32(p + PK_PROCEDURE, pick(seed, 6, &[0xffff_ffff, 3]));
    let flags = pick(seed, 7, &[0, 4, 0x400, 0x8000, 0x404, 0x8004, 0xffff_ffff]);
    e.mem.set_u32(p + PK_FLAGS, flags);
    set_schedule(e, p + PK_SCHEDULE, seed);
    e.mem
        .set_u32(p + PK_TARGET_AT, pick(seed, 8, &[0, 0x0101_0300]));
    for (k, at) in [
        (9, PK_MAY_RUN),
        (10, PK_HOLDS),
        (11, PK_ACTOR_TEST),
        (12, PK_RUN_ONCE),
    ] {
        e.mem.set_u8(p + at, pick(seed, k, &[0u8, 1, 1]));
    }
    e.mem
        .set_u8(p + PK_FLAG_TESTS, pick(seed, 13, &[0u8, 1, 2, 3]));
    e.mem.set_u32(p + PK_TYPE, pick(seed, 14, &[0u32, 1, 2]));
    p
}

/// A package that may run at any time: any month, day and hour, its conditions holding.
fn package_any_time(e: &mut Engine, seed: u32) -> u32 {
    let p = package(e, seed);
    e.mem.set_u32(p + PK_FLAGS, 4);
    for (at, byte) in [(0, 0xff), (1, 0xff), (2, 0), (3, 0xff)] {
        e.mem.set_u8(p + PK_SCHEDULE + at, byte);
    }
    e.mem.set_u8(p + PK_MAY_RUN, 1);
    e.mem.set_u8(p + PK_HOLDS, 1);
    e.mem.set_u8(p + PK_ACTOR_TEST, 1);
    p
}

/// `count` packages from `seed` (every third one may run at any time), a null item in the middle
/// when `null` is set.
fn packages(e: &mut Engine, seed: u32, count: u32, null: bool) -> Vec<u32> {
    let mut items: Vec<u32> = (0..count)
        .map(|i| {
            let s = seed.wrapping_mul(7).wrapping_add(i);
            if s % 3 == 2 {
                package_any_time(e, s)
            } else {
                package(e, s)
            }
        })
        .collect();
    if null && count > 1 {
        items.insert(1, 0);
    }
    items
}

/// A process answering `day`, `month` and a year, acting on `object`.
fn process(e: &mut Engine, day: u32, month: u32, object: u32) -> u32 {
    let p = e.mem.alloc(0x40);
    e.mem.set_u32(p, PROCESS_VTABLE);
    e.mem.set_u32(p + 0x10, day);
    e.mem.set_u32(p + 0x14, month);
    e.mem.set_u32(p + 0x18, 2281);
    e.mem.set_u32(p + 0x1c, object);
    p
}

/// A reference for the package queries: its process (acting on the reference itself), its current
/// package and the value its procedure type takes.
fn pk_reference(e: &mut Engine, current: u32, day: u32, month: u32) -> u32 {
    let r = e.mem.alloc(0x80);
    let process = process(e, day, month, r);
    e.mem.set_u32(r + REF_PROCESS, process);
    e.mem.set_u32(r + REF_CURRENT, current);
    e.mem.set_u32(r + REF_VALUE, 0x55);
    r
}

/// The calendars a case picks: the month, day, day of the week and hour (`f32` bits).
const PK_CALENDARS: [(u32, u32, u32, u32); 4] = [
    (4, 15, 3, 0x4100_0000),
    (0, 1, 0, 0x41b8_0000),
    (11, 30, 6, 0),
    (4, 14, 2, 0x40e0_0000),
];

fn set_calendar(e: &mut Engine, pick: u32) {
    let (month, day, day_of_week, hour) = PK_CALENDARS[pick as usize % PK_CALENDARS.len()];
    e.mem.set_u32(PK_MONTH, month);
    e.mem.set_u32(PK_DAY, day);
    e.mem.set_u32(PK_DAY_OF_WEEK, day_of_week);
    e.mem.set_u32(PK_HOUR, hour);
    e.mem.set_u32(PK_DAYS, 30);
}

/// A block of `size` bytes of its own (each byte from `seed`).
fn pk_block(e: &mut Engine, size: u32, seed: u32) -> u32 {
    let b = e.mem.alloc(size);
    for i in 0..size {
        e.mem.set_u8(b + i, (mix(seed, i) & 0xff) as u8);
    }
    b
}

/// The stack words the package functions hand their callees: the list's append and removal, the
/// compile index, the string, the owner's name slot, the output list's test, the line and its print,
/// and the load buffer's read.
fn pk_locals(d: Diff) -> Diff {
    d.local_at(0x0090_5820, 1, 4)
        .local_at(0x0090_5330, 1, 4)
        .local_at(0x0048_5d50, 0, 4)
        .local_at(0x0040_37b0, 0, 8)
        .local_at(0x0040_37f0, 0, 8)
        .local_at(0x0040_37d0, 0, 8)
        .local_at(VT_FN + 0x90, 1, 8)
        .local_at(0x005f_65d0, 1, 4)
        .local_at(0x00ec_623a, 0, 0x134)
        .local_at(0x0070_3c00, 0, 0x134)
        .local_at(0x0086_4980, 1, 4)
}

/// The package functions' cases: the forms family's, with their own in-place bodies, the typed
/// calls that go by address and the stack words they hand on.
fn pk_cases(d: Diff) -> Diff {
    pk_locals(
        d.forms()
            .ignore_calls(PK_IN_PLACE)
            .goes_by_address(PK_BY_ADDRESS),
    )
}

/// The package functions' shared cases, as steps of the `Diff` chain.
trait PkCases {
    fn pk(self) -> Diff;
    fn pk_caller(self) -> Diff;
    fn pk_list(self) -> Diff;
    fn pk_object(self, size: u32) -> Diff;
    fn pk_deleting(self) -> Diff;
    fn pk_load(self) -> Diff;
    fn pk_schedule(self) -> Diff;
    fn pk_schedule_pair(self) -> Diff;
    fn pk_schedule_setter(self) -> Diff;
    fn pk_conditions(self) -> Diff;
}

impl PkCases for Diff {
    fn pk(self) -> Diff {
        pk_cases(self)
    }
    fn pk_caller(self) -> Diff {
        pk_cases(self).ignore_calls(PK_OWN_IN_PLACE)
    }
    fn pk_list(self) -> Diff {
        pk_list_cases(self)
    }
    fn pk_object(self, size: u32) -> Diff {
        pk_object_cases(self, size)
    }
    fn pk_deleting(self) -> Diff {
        pk_deleting_cases(self)
    }
    fn pk_load(self) -> Diff {
        pk_load_cases(self)
    }
    fn pk_schedule(self) -> Diff {
        pk_schedule_cases(self)
    }
    fn pk_schedule_pair(self) -> Diff {
        pk_schedule_pair_cases(self)
    }
    fn pk_schedule_setter(self) -> Diff {
        pk_schedule_setter_cases(self)
    }
    fn pk_conditions(self) -> Diff {
        pk_conditions_cases(self)
    }
}

/// The item lists the list tests pick from (words standing for packages, 0 a null item).
const PK_LISTS: [&[u32]; 6] = [
    &[],
    &[0x0101_0a01],
    &[0x0101_0a01, 0x0101_0a02],
    &[0x0101_0a01, 0x0101_0a03, 0x0101_0a02],
    &[0x0101_0a01, 0, 0x0101_0a02],
    &[0, 0x0101_0a02],
];

/// A list from [`PK_LISTS`].
fn pk_list_cases(d: Diff) -> Diff {
    d.pk()
        .inputs(&[Input::Of(&[0, 1, 2, 3, 4, 5])])
        .setup(|e, case| {
            package_world(e);
            vec![new_list(e, PK_LISTS[case.word(0) as usize])]
        })
}

#[test]
fn pk_list_destruct() {
    Diff::new(entry!(0x0067_e3a0, package_list::destruct(u32)))
        .pk_list()
        .run()
        .assert_same();
}

#[test]
fn pk_list_free() {
    Diff::new(entry!(0x0067_e400, package_list::free(u32)))
        .pk_list()
        .run()
        .assert_same();
}

#[test]
fn pk_list_add_tail() {
    Diff::new(entry!(0x0067_e420, package_list::add_tail(u32, u32)))
        .pk()
        .inputs(&[Input::Of(&[0, 1, 2, 4]), Input::Of(WORDS)])
        .setup(|e, case| {
            package_world(e);
            vec![new_list(e, PK_LISTS[case.word(0) as usize]), case.word(1)]
        })
        .run()
        .assert_same();
}

#[test]
fn pk_list_copy() {
    Diff::new(entry!(0x0067_e440, package_list::copy(u32, u32)))
        .pk()
        .inputs(&[Input::Of(&[0, 1, 3]), Input::Of(&[0, 1, 2, 3, 4, 5])])
        .setup(|e, case| {
            package_world(e);
            let this = new_list(e, PK_LISTS[case.word(0) as usize]);
            let other = new_list(e, PK_LISTS[case.word(1) as usize]);
            vec![this, other]
        })
        .run()
        .assert_same();
}

#[test]
fn pk_lists_differ() {
    Diff::new(entry!(0x0067_e4a0, package_list::lists_differ(u32, u32) -> bool))
        .pk()
        .inputs(&[
            Input::Of(&[0, 1, 2, 3, 4, 5]),
            Input::Of(&[0, 1, 2, 3, 4, 5]),
        ])
        .setup(|e, case| {
            package_world(e);
            let this = new_list(e, PK_LISTS[case.word(0) as usize]);
            let other = new_list(e, PK_LISTS[case.word(1) as usize]);
            vec![this, other]
        })
        .run()
        .assert_same();
}

/// The form ids `InitItem`'s cases pick from: the lookup finds a package for low bits 1, another
/// form for 2 and nothing for 3.
const PK_IDS: [&[u32]; 6] = [
    &[],
    &[0x0100_0001],
    &[0x0100_0002],
    &[0x0100_0001, 0x0100_0003, 0x0100_0009],
    &[0x0100_0003, 0x0100_0001],
    &[0x0100_0002, 0x0100_0003, 0x0100_000a],
];

#[test]
fn pk_list_init_item() {
    Diff::new(entry!(0x0067_e560, package_list::init_item(u32, u32)))
        // Not `.pk()`: its stack words compare by their place in frames shaped alike (the id's, the
        // name's and the removal's), since the typed loop makes no call between one node's name and
        // the next node's id that would tell a new local at the same address from the old one.
        .forms()
        .ignore_calls(PK_IN_PLACE)
        .goes_by_address(PK_BY_ADDRESS)
        .inputs(&[Input::Of(&[0, 1, 2, 3, 4, 5]), Input::Of(&[0, 1, 2])])
        .setup(|e, case| {
            package_world(e);
            let p = package(e, 1);
            e.mem.set_u32(PK_FORMS + 4, p);
            let other = form(e, 0x2a, 0, 0x0100_0002);
            e.mem.set_u32(PK_FORMS + 8, other);
            // The owner: none, a form without a file, a form from a file.
            let owner = match case.word(1) {
                0 => 0,
                n => {
                    let f = form(e, 0x2a, 0, 0x0100_0abc);
                    if n == 2 {
                        let source = file(e, true);
                        // Index 5, but 1 for the list whose missing package has a node before
                        // it: the removal looks for the id with the index, which the node
                        // doesn't hold, so the exe would walk that node for ever.
                        let index = if case.word(0) == 3 { 1 } else { 5 };
                        e.mem.set_u8(source + FILE_COMPILE_INDEX_AT, index);
                        set_files(e, f, &[source]);
                    }
                    f
                }
            };
            vec![new_list(e, PK_IDS[case.word(0) as usize]), owner]
        })
        .run()
        .assert_same();
}

/// Hours as `f32` bits: 0.0, 2.0, 5.0, 5.9, 21.5, 23.5, -1.0, a quiet NaN.
const PK_HOURS: &[u32] = &[
    0,
    0x4000_0000,
    0x40a0_0000,
    0x40bc_cccd,
    0x41ac_0000,
    0x41bc_0000,
    0xbf80_0000,
    0x7fc0_0000,
];

#[test]
fn pk_in_schedule_window() {
    Diff::new(entry!(
        0x0067_e700,
        package_list::in_schedule_window(i32, i32, f32) -> bool
    ))
    .pk()
    .inputs(&[
        Input::Of(&[0, 5, 20, 23, 0xffff_ffff]),
        Input::Of(&[0, 1, 3, 8, 30]),
        Input::Of(PK_HOURS),
    ])
    .run()
    .assert_same();
}

#[test]
fn pk_find_current_package() {
    Diff::new(entry!(
        0x0067_e780,
        package_list::find_current_package(u32, u32) -> u32
    ))
    .pk_caller()
    .inputs(&[
        // The list: its seed and size (0: empty), a null item in it.
        Input::Of(&[1, 2, 3, 4, 5, 6, 7, 8]),
        Input::Of(&[0, 1, 2, 3]),
        Input::Flag,
        // The reference: none; its current package none, the list's first, another of type 1.
        Input::Of(&[0, 1, 2, 3]),
        Input::Of(&[0, 1, 2, 3]),
        // The interface's target is the reference; the process's object is an actor.
        Input::Flag,
        Input::Flag,
    ])
    .max_cases(1024)
    .setup(|e, case| {
        package_world(e);
        let items = packages(e, case.word(0), case.word(1), case.word(2) != 0);
        // List 8's first package: a window past midnight (22:00 for 6 hours) on the 30th, which the
        // calendar's 1st does not take as the day before (its day before is day 0).
        if case.word(0) == 8 {
            if let Some(&first) = items.first() {
                for (at, byte) in [(0, 0xff), (1, 0xff), (2, 30), (3, 22)] {
                    e.mem.set_u8(first + PK_SCHEDULE + at, byte);
                }
                e.mem.set_u32(first + PK_SCHEDULE + 4, 6);
                e.mem.set_u32(first + PK_FLAGS, 0);
                e.mem.set_u8(first + PK_MAY_RUN, 1);
                e.mem.set_u8(first + PK_HOLDS, 1);
            }
        }
        let list = new_list(e, &items);
        set_calendar(e, case.word(4));
        let reference = match case.word(3) {
            0 => 0,
            1 => pk_reference(e, 0, 3, 4),
            2 => pk_reference(e, items.first().copied().unwrap_or(0), 3, 4),
            _ => {
                let other = package(e, 99);
                e.mem.set_u32(other + PK_TYPE, 1);
                pk_reference(e, other, 3, 4)
            }
        };
        e.mem.set_u32(
            PK_TARGET,
            if case.word(5) != 0 {
                reference
            } else {
                0x0101_0bad
            },
        );
        set_cast(e, TYPE_ACTOR, if case.word(6) != 0 { reference } else { 0 });
        e.mem.set_u32(PK_TARGET_REFERENCE, 0x0101_0c00);
        vec![list, reference]
    })
    .run()
    .assert_same();
}

/// The missed packages' walks: the actor's process's day and month, and the calendar's month, day
/// and hour: no day; today, ending at 8:00; from the day before; across the year's end to 2:00.
const PK_WALKS: [(u32, u32, u32, u32, u32); 4] = [
    (0, 4, 4, 15, 0x4100_0000),
    (15, 4, 4, 15, 0x4100_0000),
    (14, 4, 4, 15, 0x4100_0000),
    (30, 11, 0, 1, 0x4000_0000),
];

#[test]
fn pk_missed_packages() {
    Diff::new(entry!(
        0x0067_ec40,
        package_list::missed_packages(u32, u32, u32, u32, f32)
    ))
    .pk_caller()
    .inputs(&[
        // The list's seed and size (0: empty).
        Input::Of(&[1, 2, 3, 4, 5]),
        Input::Of(&[0, 1, 2, 3]),
        Input::Of(&[0, 1, 2, 3]),
        // The output list: empty, or holding the list's first package.
        Input::Flag,
        // The actor's current package: none or the list's second.
        Input::Flag,
        // The hour: 5.0, 22.0, 7.5, 0.0 (a start past 23:00 today would walk a whole year).
        Input::Of(&[0x40a0_0000, 0x41b0_0000, 0x40f0_0000, 0]),
    ])
    .max_cases(768)
    .setup(|e, case| {
        package_world(e);
        let items = packages(e, case.word(0), case.word(1), false);
        // List 5's first package: missable at 7:00 on any day, so the walks from 5:00 and 0:00 reach
        // its hour exactly (an equal hour has not passed it yet).
        if case.word(0) == 5 {
            if let Some(&first) = items.first() {
                for (at, byte) in [(0, 0xff), (1, 0xff), (2, 0), (3, 7)] {
                    e.mem.set_u8(first + PK_SCHEDULE + at, byte);
                }
                e.mem.set_u32(first + PK_FLAGS, 4);
                e.mem.set_u8(first + PK_ACTOR_TEST, 1);
            }
        }
        let list = new_list(e, &items);
        let (day, month, calendar_month, calendar_day, hour) = PK_WALKS[case.word(2) as usize];
        e.mem.set_u32(PK_MONTH, calendar_month);
        e.mem.set_u32(PK_DAY, calendar_day);
        e.mem.set_u32(PK_DAY_OF_WEEK, 4);
        e.mem.set_u32(PK_HOUR, hour);
        e.mem.set_u32(PK_DAYS, 30);
        let current = if case.word(4) != 0 {
            items.get(1).copied().unwrap_or(0)
        } else {
            0
        };
        let actor = pk_reference(e, current, day, month);
        let first = items.first().copied().unwrap_or(0);
        let seeded = [first];
        let out = new_list(e, if case.word(3) != 0 { &seeded[..] } else { &[] });
        vec![list, actor, out, 0x3f80_0000, case.word(5)]
    })
    .run()
    .assert_same();
}

#[test]
fn pk_is_missable() {
    Diff::new(entry!(0x0067_efd0, package_list::is_missable(u32) -> bool))
        .pk()
        .inputs(&[Input::Of(&[0, 4, 0xffff_fffb, 0xffff_ffff])])
        .setup(|e, case| {
            package_world(e);
            let p = package(e, 3);
            e.mem.set_u32(p + PK_FLAGS, case.word(0));
            vec![p]
        })
        .run()
        .assert_same();
}

/// The schedule seeds the schedule tests pick from.
const PK_SEEDS: &[u32] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];

#[test]
fn pk_schedule_compare() {
    Diff::new(entry!(0x0067_8a80, schedule::compare(u32, u32) -> i32))
        .pk_caller()
        .inputs(&[Input::Of(PK_SEEDS), Input::Of(PK_SEEDS)])
        .setup(|e, case| {
            package_world(e);
            let p = package(e, case.word(0));
            vec![p, new_schedule(e, case.word(1))]
        })
        .run()
        .assert_same();
}

/// A schedule's getter: schedules from the seeds.
fn pk_schedule_cases(d: Diff) -> Diff {
    d.pk().inputs(&[Input::Of(PK_SEEDS)]).setup(|e, case| {
        package_world(e);
        vec![new_schedule(e, case.word(0))]
    })
}

#[test]
fn pk_schedule_day_of_week() {
    Diff::new(entry!(
        0x0067_8cc0,
        schedule::day_of_week(u32) -> i8
    ))
    .pk_schedule()
    .run()
    .assert_same();
}

#[test]
fn pk_schedule_date() {
    Diff::new(entry!(0x0067_8ce0, schedule::date(u32) -> u8))
        .pk_schedule()
        .run()
        .assert_same();
}

#[test]
fn pk_schedule_construct() {
    Diff::new(entry!(0x0067_fcb0, schedule::construct(u32) -> u32))
        .pk()
        .inputs(&[Input::Of(&[0, 1])])
        .setup(|e, case| {
            package_world(e);
            vec![pk_block(e, 0xc, case.word(0))]
        })
        .run()
        .assert_same();
}

#[test]
fn pk_schedule_swap_three_words() {
    Diff::new(entry!(0x0067_ff10, schedule::swap_three_words(u32)))
        .pk()
        .inputs(&[Input::Of(&[0, 1])])
        .setup(|e, case| {
            package_world(e);
            vec![pk_block(e, 0x14, case.word(0))]
        })
        .run()
        .assert_same();
}

/// Two schedules (the second none for the last seed), from the seeds.
fn pk_schedule_pair_cases(d: Diff) -> Diff {
    d.pk_caller()
        .inputs(&[Input::Of(PK_SEEDS), Input::Of(&[0, 1, 5, 6, 0xff])])
        .setup(|e, case| {
            package_world(e);
            let this = new_schedule(e, case.word(0));
            let other = if case.word(1) == 0xff {
                0
            } else {
                new_schedule(e, case.word(1))
            };
            vec![this, other]
        })
}

#[test]
fn pk_schedule_assign() {
    Diff::new(entry!(0x0067_fcf0, schedule::assign(u32, u32) -> u32))
        .pk()
        .inputs(&[Input::Of(&[0, 1, 5]), Input::Of(&[2, 6])])
        .setup(|e, case| {
            package_world(e);
            vec![new_schedule(e, case.word(0)), new_schedule(e, case.word(1))]
        })
        .run()
        .assert_same();
}

#[test]
fn pk_schedule_set_from() {
    Diff::new(entry!(0x0067_fd40, schedule::set_from(u32, u32)))
        .pk_schedule_pair()
        .run()
        .assert_same();
}

#[test]
fn pk_schedule_differs() {
    Diff::new(entry!(
        0x0067_fe10,
        schedule::differs(u32, u32) -> bool
    ))
    .pk_schedule_pair()
    .run()
    .assert_same();
}

/// A schedule's byte setter: a schedule and a word whose low byte it stores.
fn pk_schedule_setter_cases(d: Diff) -> Diff {
    d.pk()
        .inputs(&[
            Input::Of(&[0, 3]),
            Input::Of(&[0, 0x7f, 0x80, 0xff, 0x1_23]),
        ])
        .setup(|e, case| {
            package_world(e);
            vec![new_schedule(e, case.word(0)), case.word(1)]
        })
}

#[test]
fn pk_schedule_set_day_of_week() {
    Diff::new(entry!(0x0067_fdb0, schedule::set_day_of_week(u32, u8)))
        .pk_schedule_setter()
        .run()
        .assert_same();
}

#[test]
fn pk_schedule_set_date() {
    Diff::new(entry!(0x0067_fdd0, schedule::set_date(u32, u8)))
        .pk_schedule_setter()
        .run()
        .assert_same();
}

#[test]
fn pk_schedule_set_hour() {
    Diff::new(entry!(0x0067_fdf0, schedule::set_hour(u32, u8)))
        .pk_schedule_setter()
        .run()
        .assert_same();
}

#[test]
fn pk_schedule_save() {
    Diff::new(entry!(0x0067_fec0, schedule::save(u32)))
        .pk()
        .inputs(&[Input::Flag, Input::Of(&[0, 3, 7])])
        .setup(|e, case| {
            package_world(e);
            record_buffer(e);
            e.mem.set_u8(BIG_ENDIAN, case.word(0) as u8);
            vec![new_schedule(e, case.word(1))]
        })
        .run()
        .assert_same();
}

/// A package data object of `size` bytes of its own: its constructor's or destructor's cases.
fn pk_object_cases(d: Diff, size: u32) -> Diff {
    d.pk().inputs(&[Input::Of(&[0, 1])]).setup(move |e, case| {
        package_world(e);
        vec![pk_block(e, size, case.word(0))]
    })
}

/// A deleting destructor's cases: an object and its flags.
fn pk_deleting_cases(d: Diff) -> Diff {
    d.pk_caller()
        .inputs(&[Input::Of(&[0, 1, 2, 3])])
        .setup(|e, case| {
            package_world(e);
            vec![pk_block(e, 0x10, 5), case.word(0)]
        })
}

#[test]
fn pk_data_construct() {
    Diff::new(entry!(0x0067_ad20, package_data::construct(u32) -> u32))
        .pk_object(8)
        .run()
        .assert_same();
}

#[test]
fn pk_data_scalar_deleting_destructor() {
    Diff::new(entry!(
        0x0067_ad40,
        package_data::scalar_deleting_destructor(u32, u32) -> u32
    ))
    .pk_deleting()
    .run()
    .assert_same();
}

#[test]
fn pk_data_destruct() {
    Diff::new(entry!(0x0067_ad70, package_data::destruct(u32)))
        .pk_object(8)
        .run()
        .assert_same();
}

/// A save game load's cases: the byte read (0: no object; 1 and 0x101: one; 0x100: none, its low
/// byte clear) and whether the allocation fails.
fn pk_load_cases(d: Diff) -> Diff {
    d.pk()
        .inputs(&[Input::Of(&[0, 1, 0x100, 0x101]), Input::Flag])
        .setup(|e, case| {
            package_world(e);
            e.mem.set_u32(READ_WORD, case.word(0));
            e.mem.set_u32(NEW_FAILS, case.word(1));
            vec![pk_block(e, 0x10, 2), 0x0101_0e00]
        })
}

#[test]
fn pk_dialogue_load_game() {
    Diff::new(entry!(
        0x0067_b050,
        package_data::dialogue_load_game(u32, u32)
    ))
    .pk_load()
    .run()
    .assert_same();
}

#[test]
fn pk_dialogue_swap_endian() {
    Diff::new(entry!(0x0067_b100, package_data::dialogue_swap_endian(u32)))
        .pk_object(0x20)
        .run()
        .assert_same();
}

#[test]
fn pk_dialogue_construct() {
    Diff::new(entry!(0x0067_b170, package_data::dialogue_construct(u32) -> u32))
        .pk_object(0x24)
        .ignore_calls(&[0x0067_ad20])
        .run()
        .assert_same();
}

#[test]
fn pk_eat_construct() {
    Diff::new(entry!(0x0067_b950, package_data::eat_construct(u32) -> u32))
        .pk_object(0xc)
        .ignore_calls(&[0x0067_ad20])
        .run()
        .assert_same();
}

#[test]
fn pk_escort_construct() {
    Diff::new(entry!(0x0067_bbf0, package_data::escort_construct(u32) -> u32))
        .pk_object(0x10)
        .ignore_calls(&[0x0067_ad20])
        .run()
        .assert_same();
}

#[test]
fn pk_follow_construct() {
    Diff::new(entry!(0x0067_bfd0, package_data::follow_construct(u32) -> u32))
        .pk_object(0x10)
        .ignore_calls(&[0x0067_ad20])
        .run()
        .assert_same();
}

#[test]
fn pk_patrol_load_game() {
    Diff::new(entry!(
        0x0067_c330,
        package_data::patrol_load_game(u32, u32)
    ))
    .pk_load()
    .run()
    .assert_same();
}

#[test]
fn pk_patrol_construct() {
    Diff::new(entry!(0x0067_c3f0, package_data::patrol_construct(u32) -> u32))
        .pk_object(0xc)
        .ignore_calls(&[0x0067_ad20])
        .run()
        .assert_same();
}

#[test]
fn pk_patrol_scalar_deleting_destructor() {
    Diff::new(entry!(
        0x0067_c420,
        package_data::patrol_scalar_deleting_destructor(u32, u32) -> u32
    ))
    .pk_deleting()
    .run()
    .assert_same();
}

#[test]
fn pk_patrol_construct_from() {
    Diff::new(entry!(
        0x0067_c450,
        package_data::patrol_construct_from(u32, u32) -> u32
    ))
    .pk_caller()
    .inputs(&[Input::Of(&[0, 1, 0x0201, 0x7766_0201, 0xffff_ffff])])
    .setup(|e, case| {
        package_world(e);
        vec![pk_block(e, 0xc, 4), case.word(0)]
    })
    .run()
    .assert_same();
}

#[test]
fn pk_patrol_destruct() {
    Diff::new(entry!(0x0067_c490, package_data::patrol_destruct(u32)))
        .pk()
        .ignore_calls(&[0x0067_ad70])
        .inputs(&[Input::Of(&[0, 1])])
        .setup(|e, case| {
            package_world(e);
            vec![pk_block(e, 0xc, case.word(0))]
        })
        .run()
        .assert_same();
}

#[test]
fn pk_use_item_construct() {
    Diff::new(entry!(0x0067_c770, package_data::use_item_construct(u32) -> u32))
        .pk_object(0xc)
        .ignore_calls(&[0x0067_ad20])
        .run()
        .assert_same();
}

#[test]
fn pk_use_weapon_swap_endian() {
    Diff::new(entry!(
        0x0067_ca10,
        package_data::use_weapon_swap_endian(u32)
    ))
    .pk_object(0x18)
    .run()
    .assert_same();
}

#[test]
fn pk_clear_word_4() {
    Diff::new(entry!(0x0067_da20, package_data::clear_word_4(u32, u32)))
        .pk()
        .inputs(&[Input::Of(&[0, 1]), Input::Of(&[0, 0x1234])])
        .setup(|e, case| {
            package_world(e);
            vec![pk_block(e, 0x10, case.word(0)), case.word(1)]
        })
        .run()
        .assert_same();
}

#[test]
fn pk_event_action_construct() {
    Diff::new(entry!(
        0x0067_da40,
        package_data::event_action_construct(u32) -> u32
    ))
    .pk_object(0x10)
    .run()
    .assert_same();
}

/// An event action: an idle, a script of the test's (or none), a topic and a word.
fn event_action(e: &mut Engine, seed: u32, script: bool) -> u32 {
    let a = pk_block(e, 0x10, seed);
    let s = if script {
        let s = e.mem.alloc(0x54);
        e.mem.set_u32(s, PK_VTABLE);
        s
    } else {
        0
    };
    e.mem.set_u32(a + 4, s);
    a
}

#[test]
fn pk_event_action_clear() {
    Diff::new(entry!(0x0067_da80, package_data::event_action_clear(u32)))
        .pk()
        .inputs(&[Input::Flag])
        .setup(|e, case| {
            package_world(e);
            vec![event_action(e, 1, case.word(0) != 0)]
        })
        .run()
        .assert_same();
}

#[test]
fn pk_event_action_copy() {
    Diff::new(entry!(
        0x0067_dad0,
        package_data::event_action_copy(u32, u32)
    ))
    .pk()
    .inputs(&[Input::Flag, Input::Flag, Input::Flag])
    .setup(|e, case| {
        package_world(e);
        registries(e, true, 0, true, true, 0);
        e.mem.set_u32(NEW_FAILS, case.word(2));
        let this = event_action(e, 1, case.word(0) != 0);
        let other = event_action(e, 2, case.word(1) != 0);
        vec![this, other]
    })
    // A script to copy into an action without one, when the allocation fails: the exe marks the
    // null script temporary (a read of its flags at 8) and stops there.
    .expect_stops(|case| case.word(0) == 0 && case.word(1) != 0 && case.word(2) != 0)
    .run()
    .assert_same();
}

#[test]
fn pk_location_swap_endian() {
    Diff::new(entry!(0x0067_eff0, package_data::location_swap_endian(u32)))
        .pk_object(0xc)
        .run()
        .assert_same();
}

#[test]
fn pk_conditions_construct() {
    Diff::new(entry!(0x0068_0890, conditions::construct(u32) -> u32))
        .pk_object(8)
        .run()
        .assert_same();
}

/// A condition list from [`PK_LISTS`] (the items standing for condition items), or none (6).
fn pk_conditions_cases(d: Diff) -> Diff {
    d.pk()
        .inputs(&[Input::Of(&[0, 1, 2, 3, 4, 5, 6])])
        .setup(|e, case| {
            package_world(e);
            let pick = case.word(0) as usize;
            vec![if pick == 6 {
                0
            } else {
                new_list(e, PK_LISTS[pick])
            }]
        })
}

#[test]
fn pk_conditions_destruct() {
    Diff::new(entry!(0x0068_08b0, conditions::destruct(u32)))
        .pk()
        .inputs(&[Input::Of(&[0, 1, 2, 3, 4, 5])])
        .setup(|e, case| {
            package_world(e);
            vec![new_list(e, PK_LISTS[case.word(0) as usize])]
        })
        .run()
        .assert_same();
}

#[test]
fn pk_conditions_clear() {
    Diff::new(entry!(0x0068_0fe0, conditions::clear(u32)))
        .pk_conditions()
        .run()
        .assert_same();
}

/// Values as `f32` bits: 0.0, -0.0, 1.0, -1.0, 2.5, infinity, a quiet NaN, a signalling one.
const PK_VALUES: &[u32] = &[
    0,
    0x8000_0000,
    0x3f80_0000,
    0xbf80_0000,
    0x4020_0000,
    0x7f80_0000,
    0x7fc0_0000,
    0x7f80_0001,
];

#[test]
fn pk_conditions_compare_values() {
    Diff::new(entry!(
        0x0068_1050,
        conditions::compare_values(u32, f32, f32) -> bool
    ))
    .pk()
    .inputs(&[
        Input::Of(&[0, 1, 2, 3, 4, 5, 6, 0xffff_ffff]),
        Input::Of(PK_VALUES),
        Input::Of(PK_VALUES),
    ])
    .run()
    .assert_same();
}

#[test]
fn pk_conditions_swap_function_data() {
    Diff::new(entry!(0x0068_1110, conditions::swap_function_data(u32)))
        .pk_object(0xc)
        .run()
        .assert_same();
}
