//! What the command modules' tests share: an engine with JIP installed, the game's heap and the
//! two `TESModel` virtual functions as small stand-ins (written from the exe's functions, not from
//! the code under test), forms built at the offsets of the headers and the exe, and compiled
//! arguments.
//!
//! Memory: forms from [`FORMS`], the stand-in heap from [`HEAP`], the vtables at their real
//! addresses (`TESModel`'s `0101CF44`, `TESModelTextureSwap`'s `0101D124`, `Clouds`' `0104EC08`).

pub use crate::test_support::{call, console, DATA, RESULT, SCRIPT};
use engine_core::prelude::*;
use std::cell::RefCell;

pub const FORMS: u32 = 0x0200_0000;
pub const HEAP: u32 = 0x0400_0000;
const HEAP_SIZE: u32 = 0x10_0000;
/// The heap's next free byte.
const HEAP_TOP: u32 = HEAP + HEAP_SIZE - 4;
/// `""`, what the game's `GetModelPath` (`00408DA0`) answers for a model with no path (its
/// `DAT_01011584`).
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

/// The engine: xNVSE's table, JIP installed (answering "not installed", as today), forms and
/// heap memory, the stand-ins registered.
pub fn engine() -> Engine {
    CALLS.with(|c| c.borrow_mut().clear());
    let mut e = crate::test_support::engine();
    crate::install(&mut e);
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
