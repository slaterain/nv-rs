//! What the command modules' tests share: an engine with JIP installed, the game's heap and the
//! two `TESModel` virtual functions as small stand-ins (written from the exe's functions, not from
//! the code under test), forms built at the offsets of the headers and the exe, and compiled
//! arguments.
//!
//! Memory: forms from [`FORMS`], the stand-in heap from [`HEAP`], the vtables at their real
//! addresses (`TESModel`'s `0101CF44`, `TESModelTextureSwap`'s `0101D124`, `Clouds`' `0104EC08`).

pub use crate::test_support::{call, console, RESULT, SCRIPT};
use engine_core::prelude::*;
use std::cell::RefCell;

pub const FORMS: u32 = 0x0200_0000;
pub const HEAP: u32 = 0x0400_0000;
const HEAP_SIZE: u32 = 0x10_0000;
/// The heap's next free byte.
const HEAP_TOP: u32 = HEAP + HEAP_SIZE - 4;

/// Where the stand-in heap hands out its next block.
pub fn heap_next(e: &Engine) -> u32 {
    e.mem.u32(HEAP_TOP)
}
/// `""`, what the game's `GetModelPath` (`00408DA0`) answers for a model with no path (the empty
/// text at `01011584`).
pub const EMPTY_TEXT: u32 = 0x0101_1584;
pub const TES_MODEL_VTBL: u32 = 0x0101_cf44;
pub const TES_MODEL_TEXTURE_SWAP_VTBL: u32 = 0x0101_d124;
pub const CLOUDS_VTBL: u32 = 0x0104_ec08;
const GET_MODEL_PATH_FN: u32 = 0x0040_8da0;
const SET_MODEL_PATH_FN: u32 = 0x0048_9100;
const CLOUDS_UPDATE_FN: u32 = 0x0063_4110;
const SET_CURRENT_CLIMATE_FN: u32 = 0x0063_c8f0;
const HEAP_ALLOCATE: u32 = 0x00aa_3e40;
const HEAP_FREE: u32 = 0x00aa_4060;
const GAME_HEAP: u32 = 0x011f_6238;

thread_local! {
    /// The game functions the commands called, in order, with their words.
    pub static CALLS: RefCell<Vec<(u32, Vec<u32>)>> = const { RefCell::new(Vec::new()) };
}

/// The calls made since the engine was set up.
pub fn calls() -> Vec<(u32, Vec<u32>)> {
    CALLS.with(|c| c.borrow().clone())
}

fn record(addr: u32, words: &[u32]) {
    CALLS.with(|c| c.borrow_mut().push((addr, words.to_vec())));
}

fn eax(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

/// The stand-in heap: blocks handed out upward, 8-byte aligned, filled with 0xCD (so a field
/// the command forgets to set shows).
fn allocate(e: &mut Engine, w: &[u32]) -> Ret {
    assert_eq!(w[0], GAME_HEAP, "the game's MemoryManager");
    record(HEAP_ALLOCATE, w);
    let at = e.mem.u32(HEAP_TOP);
    let size = w[1].next_multiple_of(8);
    for i in 0..size {
        e.mem.set_u8(at + i, 0xcd);
    }
    e.mem.set_u32(HEAP_TOP, at + size);
    eax(at)
}

fn free(_: &mut Engine, w: &[u32]) -> Ret {
    assert_eq!(w[0], GAME_HEAP, "the game's MemoryManager");
    record(HEAP_FREE, w);
    eax(0)
}

/// The sizes asked of the game's heap so far, in order.
pub fn allocations() -> Vec<u32> {
    calls()
        .into_iter()
        .filter(|c| c.0 == HEAP_ALLOCATE)
        .map(|c| c.1[1])
        .collect()
}

/// The blocks freed so far.
pub fn freed() -> Vec<u32> {
    calls()
        .into_iter()
        .filter(|c| c.0 == HEAP_FREE)
        .map(|c| c.1[1])
        .collect()
}

/// `00408DA0`: the model's path text (`+4`), or `""` when it has no buffer.
fn get_model_path(e: &mut Engine, w: &[u32]) -> Ret {
    record(GET_MODEL_PATH_FN, w);
    let data = e.mem.u32(w[0] + 4);
    eax(if data == 0 { EMPTY_TEXT } else { data })
}

/// `00489100`: the text copied into the model's path (`BSStringT::Set` at `+4`); here into a
/// new block, length and size set to the text's.
fn set_model_path(e: &mut Engine, w: &[u32]) -> Ret {
    record(SET_MODEL_PATH_FN, w);
    let text = nvse::table::guest_text(e, w[1]);
    let block = allocate(e, &[GAME_HEAP, text.len() as u32 + 1]).eax;
    e.mem.write(block, &text);
    e.mem.set_u8(block + text.len() as u32, 0);
    e.mem.set_u32(w[0] + 4, block);
    e.mem.set_u16(w[0] + 8, text.len() as u16);
    e.mem.set_u16(w[0] + 10, text.len() as u16);
    eax(0)
}

/// The engine: xNVSE's table, JIP installed answering "installed" (built commands have their
/// handlers only then: until step 3.10 every JIP opcode is as without the DLL), forms and heap
/// memory, the stand-ins registered.
pub fn engine() -> Engine {
    CALLS.with(|c| c.borrow_mut().clear());
    let mut e = crate::test_support::engine();
    crate::install_as(&mut e, true);
    e.map(FORMS, 0x10_0000);
    e.map(HEAP, HEAP_SIZE);
    e.mem.set_u32(HEAP_TOP, HEAP);
    e.map(0x0101_1000, 0x1000);
    e.map(0x0101_c000, 0x2000);
    e.map(0x0104_e000, 0x1000);
    e.map(0x011f_f000, 0x1000);
    for vtbl in [TES_MODEL_VTBL, TES_MODEL_TEXTURE_SWAP_VTBL] {
        e.mem.set_u32(vtbl + 0x14, GET_MODEL_PATH_FN);
        e.mem.set_u32(vtbl + 0x18, SET_MODEL_PATH_FN);
    }
    e.mem.set_u32(CLOUDS_VTBL + 0xc, CLOUDS_UPDATE_FN);
    e.register(HEAP_ALLOCATE, allocate);
    e.register(HEAP_FREE, free);
    e.register(GET_MODEL_PATH_FN, get_model_path);
    e.register(SET_MODEL_PATH_FN, set_model_path);
    e.register(CLOUDS_UPDATE_FN, |_, w| {
        record(CLOUDS_UPDATE_FN, w);
        eax(0)
    });
    e.register(SET_CURRENT_CLIMATE_FN, |_, w| {
        record(SET_CURRENT_CLIMATE_FN, w);
        eax(0)
    });
    e
}

/// A form of `type_id` with id `id` at `at` (its vtable word left 0).
pub fn form(e: &mut Engine, at: u32, type_id: u8, id: u32) -> u32 {
    e.mem.set_u8(at + 4, type_id);
    e.mem.set_u32(at + 0xc, id);
    at
}

/// A game string (`BSStringT`) at `at` holding `text` in a buffer of its own (`None`: no buffer).
pub fn string(e: &mut Engine, at: u32, text: Option<&[u8]>) {
    match text {
        None => {
            e.mem.set_u32(at, 0);
            e.mem.set_u16(at + 4, 0);
            e.mem.set_u16(at + 6, 0);
        }
        Some(t) => {
            let block = allocate(e, &[GAME_HEAP, t.len() as u32 + 1]).eax;
            e.mem.write(block, t);
            e.mem.set_u8(block + t.len() as u32, 0);
            e.mem.set_u32(at, block);
            e.mem.set_u16(at + 4, t.len() as u16);
            e.mem.set_u16(at + 6, t.len() as u16);
        }
    }
}

/// A `TESModel` at `at` (its vtable `0101CF44`) with the path `text`.
pub fn model(e: &mut Engine, at: u32, text: Option<&[u8]>) {
    e.mem.set_u32(at, TES_MODEL_VTBL);
    string(e, at + 4, text);
}

/// The text of the game string at `at`.
pub fn text_at(e: &Engine, at: u32) -> Vec<u8> {
    nvse::table::guest_text(e, e.mem.u32(at))
}

/// Compiled arguments: a reference of the script's list (`r` and its index), an immediate
/// integer (`n`), an immediate double (`z`), a string.
pub fn r(index: u16) -> Vec<u8> {
    let mut b = vec![b'r'];
    b.extend_from_slice(&index.to_le_bytes());
    b
}
pub fn n(v: i32) -> Vec<u8> {
    let mut b = vec![b'n'];
    b.extend_from_slice(&v.to_le_bytes());
    b
}
pub fn z(v: f64) -> Vec<u8> {
    let mut b = vec![b'z'];
    b.extend_from_slice(&v.to_le_bytes());
    b
}
pub fn s(text: &[u8]) -> Vec<u8> {
    crate::test_support::s(text)
}

/// Makes `form` reference `index` (from 1) of the script's reference list (the list at
/// `Script +0x44`, each `RefVariable` with its form at `+8` and no variable index at `+0xC`).
pub fn set_ref(e: &mut Engine, index: u16, form: u32) {
    let i = u32::from(index);
    let ref_var = 0x0300_a000 + i * 0x10;
    e.mem.set_u32(ref_var + 8, form);
    e.mem.set_u32(ref_var + 0xc, 0);
    let mut node = SCRIPT + 0x44;
    for at in 1..i {
        let next = e.mem.u32(node + 4);
        node = if next == 0 {
            let made = 0x0300_b000 + (at + 1) * 8;
            e.mem.set_u32(node + 4, made);
            made
        } else {
            next
        };
    }
    e.mem.set_u32(node, ref_var);
}

/// The string a string command answered (its result is a string variable's id).
pub fn string_result(e: &Engine, result: f64) -> Vec<u8> {
    nvse::vars::string(e, result as u32).expect("a string variable")
}

/// The result's low word, as a form command writes the id there.
pub fn form_result(e: &Engine) -> u32 {
    e.mem.u32(RESULT)
}

/// A call on reference `this` (the handler's `thisObj` word), as a script's `ref.Command` makes.
pub fn call_on(e: &mut Engine, opcode: u32, this: u32, args: &[Vec<u8>]) -> (u32, f64) {
    let mut words = crate::test_support::line(e, opcode, args);
    words[2] = this;
    let execute = crate::test_support::execute_of(e, opcode);
    let ok = e.call(execute, &words).eax & 0xff;
    (ok, e.mem.f64(RESULT))
}

/// `BaseExtraList::GetByType` (`00410220`), `AddExtra` (`0040FF60`), `RemoveByType` (`00410140`)
/// and `BGSSaveLoadGame::AddChange` (`0084A690`) as stand-ins written from the exe's functions:
/// the walk from the list's +4 (each extra's type at +4, next at +8), the append at the chain's
/// end with the presence bit set (+8 bits), the unlink of the first extra of the type with its
/// bit cleared (its delete through the vtable left out: recorded instead), and the change call
/// recorded. `g_BGSSaveLoadGame` points at [`SAVE_LOAD`].
pub const GET_BY_TYPE_FN: u32 = 0x0041_0220;
pub const ADD_EXTRA_FN: u32 = 0x0040_ff60;
pub const REMOVE_BY_TYPE_FN: u32 = 0x0041_0140;
pub const ADD_CHANGE_FN: u32 = 0x0084_a690;
pub const SAVE_LOAD: u32 = FORMS + 0xf_f000;

pub fn extra_stand_ins(e: &mut Engine) {
    e.map(0x011d_d000, 0x1000);
    e.mem.set_u32(0x011d_df38, SAVE_LOAD);
    e.register(GET_BY_TYPE_FN, |e, w| {
        let mut x = e.mem.u32(w[0] + 4);
        while x != 0 && u32::from(e.mem.u8(x + 4)) != w[1] {
            x = e.mem.u32(x + 8);
        }
        eax(x)
    });
    e.register(ADD_EXTRA_FN, |e, w| {
        record(ADD_EXTRA_FN, w);
        let (list, extra) = (w[0], w[1]);
        let mut at = list + 4;
        while e.mem.u32(at) != 0 {
            at = e.mem.u32(at) + 8;
        }
        e.mem.set_u32(at, extra);
        let kind = u32::from(e.mem.u8(extra + 4));
        let bits = list + 8 + (kind >> 3);
        e.mem.set_u8(bits, e.mem.u8(bits) | (1 << (kind & 7)));
        eax(extra)
    });
    e.register(REMOVE_BY_TYPE_FN, |e, w| {
        record(REMOVE_BY_TYPE_FN, w);
        let (list, kind) = (w[0], w[1]);
        let mut at = list + 4;
        loop {
            let x = e.mem.u32(at);
            if x == 0 {
                break;
            }
            if u32::from(e.mem.u8(x + 4)) == kind {
                e.mem.set_u32(at, e.mem.u32(x + 8));
                break;
            }
            at = x + 8;
        }
        let bits = list + 8 + (kind >> 3);
        e.mem.set_u8(bits, e.mem.u8(bits) & !(1 << (kind & 7)));
        eax(0)
    });
    e.register(ADD_CHANGE_FN, |_, w| {
        record(ADD_CHANGE_FN, w);
        eax(0)
    });
}

/// An extra of `kind` at `at` holding `word` at +0xC, put first in the list at `list` (presence
/// bit set).
pub fn put_extra(e: &mut Engine, list: u32, at: u32, kind: u32, word: u32) {
    e.mem.set_u8(at + 4, kind as u8);
    e.mem.set_u32(at + 8, e.mem.u32(list + 4));
    e.mem.set_u32(at + 0xc, word);
    e.mem.set_u32(list + 4, at);
    let bits = list + 8 + (kind >> 3);
    e.mem.set_u8(bits, e.mem.u8(bits) | (1 << (kind & 7)));
}

/// The calls made to `addr`, their words.
pub fn calls_to(addr: u32) -> Vec<Vec<u32>> {
    calls()
        .into_iter()
        .filter(|c| c.0 == addr)
        .map(|c| c.1)
        .collect()
}

/// The game's form map (`*011C54C0`: 16 buckets at +0x100, entries from +0x400) and the data
/// handler (`*011C3F2C`) in test memory; the page of both globals mapped.
pub const FORM_MAP: u32 = FORMS + 0xf_0000;
pub const DATA_HANDLER_AT: u32 = FORMS + 0xf_8000;

pub fn game_globals(e: &mut Engine) {
    e.map(0x011c_3000, 0x3000);
    e.mem.set_u32(0x011c_54c0, FORM_MAP);
    e.mem.set_u32(FORM_MAP + 4, 16);
    e.mem.set_u32(FORM_MAP + 8, FORM_MAP + 0x100);
    e.mem.set_u32(0x011c_3f2c, DATA_HANDLER_AT);
}

/// Puts `form` in the form map under `id` (entry `n`, first in its bucket).
pub fn in_form_map(e: &mut Engine, form: u32, id: u32, n: u32) {
    let item = FORM_MAP + 0x400 + 0x10 * n;
    let bucket = FORM_MAP + 0x100 + 4 * (id % 16);
    e.mem.set_u32(item, e.mem.u32(bucket));
    e.mem.set_u32(item + 4, id);
    e.mem.set_u32(item + 8, form);
    e.mem.set_u32(bucket, item);
}

/// A loaded file `info` at `index` of the data handler's slots (+0x21C), named `name` (+0x20),
/// its own index at +0x40C; the loaded count (+0x218) raised to cover it.
pub fn loaded_mod(e: &mut Engine, index: u32, info: u32, name: &[u8]) {
    e.mem.set_u32(DATA_HANDLER_AT + 0x21c + 4 * index, info);
    e.mem.write(info + 0x20, name);
    e.mem.set_u8(info + 0x20 + name.len() as u32, 0);
    e.mem.set_u8(info + 0x40c, index as u8);
    let count = e.mem.u32(DATA_HANDLER_AT + 0x218).max(index + 1);
    e.mem.set_u32(DATA_HANDLER_AT + 0x218, count);
}

/// A packed xNVSE array of `values`, owned by mod 0; its id.
pub fn array_of(e: &mut Engine, values: Vec<nvse::vars::Value>) -> u32 {
    let id = nvse::vars::create_array(e, nvse::vars::KEY_NUMBER, true, 0);
    for (i, v) in values.into_iter().enumerate() {
        nvse::vars::set_element(e, id, &nvse::vars::Key::Number(i as f64), v);
    }
    id
}

/// The values of array `id` in order.
pub fn array_data(e: &Engine, id: f64) -> Vec<nvse::vars::Data> {
    crate::commands::jip_fn_aux_variable::array_values(e, id as u32).expect("an array")
}
