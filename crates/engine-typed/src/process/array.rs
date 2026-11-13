//! The process array (`ProcessArray`, Xbox PDB; 0x40 bytes): an array of actor pointers (the first
//! 0x10 bytes: the vtable, the element block at +4, the capacity at +8, the count at +0xa and the
//! number of non-null elements at +0xc, all three counts words) divided into four consecutive process
//! levels. Level `n` owns the indexes from `ProcessLevelHeads[n]` (+0x10) up to, not including,
//! `ProcessLevelTails[n]` (+0x20); `ProcessLevelNextUpdate[n]` (+0x30) is a cursor inside that range.
//! The process lists hold the array at +4.

use super::FAMILY;
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::{typed_call, Engine};

/// The three tables of four words: the levels' heads, tails and update cursors.
const HEADS: u32 = 0x10;
const TAILS: u32 = 0x20;
const NEXT_UPDATE: u32 = 0x30;
/// The levels' table word for `level`, a signed index whose address wraps like the exe's
/// `[this + level * 4 + table]`.
fn slot(this: u32, table: u32, level: i32) -> u32 {
    this.wrapping_add(table)
        .wrapping_add((level as u32).wrapping_mul(4))
}

fn head(e: &Engine, this: u32, level: i32) -> u32 {
    e.mem.u32(slot(this, HEADS, level))
}

fn tail(e: &Engine, this: u32, level: i32) -> u32 {
    e.mem.u32(slot(this, TAILS, level))
}

// Typed from 005de0f0
/// `thiscall(array, level)`: the first index of process level `level` (the word at +0x10 + 4 level;
/// identical code serves other classes, so it reads that word of any object).
pub fn level_head(e: &mut Engine, this: Ptr, level: u32) -> u32 {
    e.mem.u32(
        this.addr()
            .wrapping_add(HEADS)
            .wrapping_add(level.wrapping_mul(4)),
    )
}

// Typed from 005be5c0
/// `thiscall(array, level)`: one past the last index of process level `level` (the word at +0x20 +
/// 4 level, of any object).
pub fn level_tail(e: &mut Engine, this: Ptr, level: u32) -> u32 {
    e.mem.u32(
        this.addr()
            .wrapping_add(TAILS)
            .wrapping_add(level.wrapping_mul(4)),
    )
}

// Typed from 0096ae90
/// `thiscall(array, index, value)`: stores the pointer `*value` at `index`. An index at or past the
/// count raises the count to `index + 1` (as a word) and counts a non-null pointer in; below the
/// count the number of non-null elements follows the element being replaced.
pub fn set_element(e: &mut Engine, this: Ptr, index: u32, value: Ptr) {
    let base = this.addr();
    let pointer = e.mem.u32(value.addr());
    if index >= u32::from(e.mem.u16(base.wrapping_add(0xa))) {
        // The count is a word: the low half of `index + 1`.
        let [low, high, _, _] = index.wrapping_add(1).to_le_bytes();
        e.mem
            .set_u16(base.wrapping_add(0xa), u16::from_le_bytes([low, high]));
        if pointer != 0 {
            let count = e.mem.u16(base.wrapping_add(0xc));
            e.mem.set_u16(base.wrapping_add(0xc), count.wrapping_add(1));
        }
    } else {
        let elements = e.mem.u32(base.wrapping_add(4));
        let current = e.mem.u32(elements.wrapping_add(index.wrapping_mul(4)));
        if pointer != 0 {
            if current == 0 {
                let count = e.mem.u16(base.wrapping_add(0xc));
                e.mem.set_u16(base.wrapping_add(0xc), count.wrapping_add(1));
            }
        } else if current != 0 {
            let count = e.mem.u16(base.wrapping_add(0xc));
            e.mem.set_u16(base.wrapping_add(0xc), count.wrapping_sub(1));
        }
    }
    let elements = e.mem.u32(base.wrapping_add(4));
    e.mem
        .set_u32(elements.wrapping_add(index.wrapping_mul(4)), pointer);
}

// Typed from 0096ab20
/// `thiscall(array, index)`: the process level that owns `index`, the first `n` in `0..4` with
/// `heads[n] <= index < tails[n]`, else -1.
pub fn level_of(e: &mut Engine, this: Ptr, index: u32) -> i32 {
    let a = this.addr();
    (0..4)
        .find(|&level| index >= head(e, a, level) && index < tail(e, a, level))
        .unwrap_or(-1)
}

/// How many element words [`find`] reads at once: never past the end of a 4 KB page, so that a
/// search reaching an unmapped page stops at the same element the exe's would.
const PAGE: u32 = 0x1000;

// Typed from 0096abc0
/// `thiscall(array, actor, level)`: the index of the first element equal to `actor`, searching from
/// the start of process level `level` to the end of the last level (`tails[3]`, +0x2c); -1 for a
/// null actor or no match. The exe reads one element at a time through `00877a30`; this reads the
/// element block a page at a time and compares in a plain loop (the same first match).
pub fn find(e: &mut Engine, this: Ptr, actor: u32, level: i32) -> u32 {
    if actor == 0 {
        return u32::MAX;
    }
    let a = this.addr();
    let end = e.mem.u32(a.wrapping_add(0x2c));
    let mut index = head(e, a, level);
    let wanted = actor.to_le_bytes();
    let mut bytes = [0u8; PAGE as usize];
    while index < end {
        let first = typed_call!(leaf::FAMILY, 0x0087_7a30, leaf::array_element(e, a, index));
        // The words from `first` to the page's end (at least one: an element straddling the page
        // end is read whole), no more than the indexes left.
        let in_page = (PAGE - (first & (PAGE - 1))) / 4;
        let n = in_page.clamp(1, end - index);
        let chunk = &mut bytes[..4 * n as usize];
        e.mem.read(first, chunk);
        if let Some((k, _)) = (0u32..)
            .zip(chunk.chunks_exact(4))
            .find(|(_, w)| *w == wanted)
        {
            return index.wrapping_add(k);
        }
        index += n;
    }
    u32::MAX
}

// Typed from 0096aa40
/// `thiscall(array, index)`: removes the element at `index`: finds its level ([`level_of`]; nothing
/// for none), shortens the level by one when it is not empty, moves the level's last element into
/// `index` unless `index` is that last slot, clears the last slot and corrects the cursors.
pub fn remove_at(e: &mut Engine, this: Ptr, index: u32) {
    let a = this.addr();
    let level = typed_call!(FAMILY, 0x0096_ab20, level_of(e, this, index));
    if level == -1 {
        return;
    }
    if tail(e, a, level) > head(e, a, level) {
        let shortened = tail(e, a, level).wrapping_sub(1);
        e.mem.set_u32(slot(a, TAILS, level), shortened);
    }
    if index != tail(e, a, level) {
        let last = tail(e, a, level);
        let element = typed_call!(leaf::FAMILY, 0x0087_7a30, leaf::array_element(e, a, last));
        typed_call!(
            FAMILY,
            0x0096_ae90,
            set_element(e, this, index, Ptr::new(element))
        );
    }
    let last = tail(e, a, level);
    e.with_stack(4, |e, zero| {
        typed_call!(FAMILY, 0x0096_ae90, set_element(e, this, last, zero));
    });
    typed_call!(FAMILY, 0x0096_ac30, correct_cursors(e, this));
}

// Typed from 0096ac30
/// `thiscall(array)`: a level's cursor outside `heads[n] <= cursor < tails[n]` goes back to the
/// level's start.
pub fn correct_cursors(e: &mut Engine, this: Ptr) {
    let a = this.addr();
    for level in 0..4 {
        let cursor = e.mem.u32(slot(a, NEXT_UPDATE, level));
        if cursor < head(e, a, level) || cursor >= tail(e, a, level) {
            let start = head(e, a, level);
            e.mem.set_u32(slot(a, NEXT_UPDATE, level), start);
        }
    }
}

// Typed from 0096aaf0
/// `ProcessArray::RemoveActor` (Xbox PDB; `thiscall(array, actor, level)`): finds `actor` from the
/// start of process level `level` ([`find`]) and removes the element found ([`remove_at`]).
pub fn remove_actor(e: &mut Engine, this: Ptr, actor: u32, level: i32) {
    let index = typed_call!(FAMILY, 0x0096_abc0, find(e, this, actor, level));
    typed_call!(FAMILY, 0x0096_aa40, remove_at(e, this, index));
}
