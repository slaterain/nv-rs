//! lStewieAl's Tweaks built in (`crates/stewie`): which of its rows are on. Each change Tweaks makes
//! to the game's code is a row of its own (docs/FIXES.md, "lStewieAl's Tweaks"), one bit of a table
//! every engine of a game shares ([`Engine::stewie`]). A translation Tweaks patches tests its row's
//! bit at the patched place before anything else, with one load; with the bit clear it runs as the
//! exe's code. `crates/stewie` sets the bits at its install, before the game's threads start: all
//! clear unless the host's row `NV_STEWIE` is on, then each row on unless its own key (or Tweaks'
//! own `[Tweaks]` key for it) turns it off.
//!
//! The rows' numbers are fixed ranges per group ([`CRASH`], ...), so groups built side by side add
//! rows in ranges of their own; `crates/stewie`'s table names each row once and checks that no two
//! rows share a bit.

use crate::Engine;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

/// The table's words: 2,048 rows (Tweaks has 1,082 rows in `measure/stewie/PLAN.md`).
pub const WORDS: usize = 32;

/// One row's bit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Row(u16);

impl Row {
    /// Row `n` of the group starting at `group`.
    pub const fn at(group: u16, n: u16) -> Row {
        let index = group + n;
        assert!((index as usize) < WORDS * 64, "past the table's rows");
        Row(index)
    }

    /// Its bit's number in the table.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// The groups' first rows. M1a: the crash, null-check, use-after-free and leak fixes VNV's INI has
/// on (`measure/stewie/STATUS-1118.md`, 80 rows).
pub const CRASH: u16 = 0;
/// The rest of the engine fixes (M1) and the hook points they need.
pub const FIXES: u16 = 128;
/// VNV's tweaks (M2).
pub const VNV_TWEAKS: u16 = 640;
/// The other switches (M3).
pub const OTHER_TWEAKS: u16 = 1024;

const _: () = assert!(CRASH < FIXES && FIXES < VNV_TWEAKS && VNV_TWEAKS < OTHER_TWEAKS);

/// S0079, Tweaks' `patchDialogSpeechNullCheck`: `0x935E35`, in the shared actor's clear
/// (`00935d90`), asks `NiObjectNET::GetController` (`00a5c570`) of the speaker's 3D, which is null
/// for a speaker whose 3D is not loaded; Tweaks calls `NiObject_FindType_NullChecked` there, which
/// answers null for no node.
pub const DIALOG_SPEECH_NULL_CHECK: Row = Row::at(CRASH, 0);

/// The table: one bit a row.
#[derive(Debug)]
pub(crate) struct Bits([AtomicU64; WORDS]);

impl Default for Bits {
    fn default() -> Self {
        Bits(std::array::from_fn(|_| AtomicU64::new(0)))
    }
}

impl Engine {
    /// Whether Tweaks' row `row` is on. One load; a translation reads it at the patched place
    /// before anything else.
    #[inline]
    pub fn stewie(&self, row: Row) -> bool {
        let word = self.extend.stewie.0[row.index() / 64].load(Relaxed);
        word & (1 << (row.index() % 64)) != 0
    }

    /// Turns `row` on or off, in one atomic step (`crates/stewie`'s install, before the threads
    /// start).
    pub fn set_stewie(&self, row: Row, on: bool) {
        let word = &self.extend.stewie.0[row.index() / 64];
        let bit = 1u64 << (row.index() % 64);
        if on {
            word.fetch_or(bit, Relaxed);
        } else {
            word.fetch_and(!bit, Relaxed);
        }
    }

    /// Every row off.
    pub fn clear_stewie(&self) {
        for word in &self.extend.stewie.0 {
            word.store(0, Relaxed);
        }
    }

    /// How many rows are on.
    pub fn stewie_rows_on(&self) -> u32 {
        self.extend
            .stewie
            .0
            .iter()
            .map(|w| w.load(Relaxed).count_ones())
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_row_is_one_bit_shared_by_the_threads_engines() {
        let e = Engine::new();
        let other = Row::at(OTHER_TWEAKS, 70);
        assert!(!e.stewie(DIALOG_SPEECH_NULL_CHECK));
        e.set_stewie(DIALOG_SPEECH_NULL_CHECK, true);
        assert!(e.stewie(DIALOG_SPEECH_NULL_CHECK));
        assert!(!e.stewie(other), "only its own bit");
        e.set_stewie(other, true);
        assert_eq!(e.stewie_rows_on(), 2);
        // A real thread's engine sees the same table.
        let thread = e.extend.for_thread();
        assert!(thread.stewie.0[0].load(Relaxed) & 1 != 0);
        e.set_stewie(DIALOG_SPEECH_NULL_CHECK, false);
        assert!(!e.stewie(DIALOG_SPEECH_NULL_CHECK));
        assert!(e.stewie(other));
        assert_eq!(thread.stewie.0[0].load(Relaxed) & 1, 0);
        e.clear_stewie();
        assert_eq!(e.stewie_rows_on(), 0);
    }

    #[test]
    fn the_groups_fit_the_table_in_order() {
        assert_eq!(Row::at(OTHER_TWEAKS, 1023).index(), WORDS * 64 - 1);
    }
}
